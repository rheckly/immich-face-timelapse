//! Optional login with one user (`LOGIN_USER` / `LOGIN_PASSWORD`, display
//! name `LOGIN_NAME` for the profile button).
//!
//! Without `LOGIN_PASSWORD` the login is off and everything stays open as
//! before. With it, every request on the main port needs a session cookie,
//! except the health check, the login page with its icon and the login API.
//! The share listener (port 5001, `web::shares::public_router`) never asks
//! for a login.
//!
//! The session is a signed cookie: `<expiry>.<hmac>`. The HMAC covers the
//! expiry and a fingerprint of user name and password, so changing either
//! makes all old cookies invalid. The signing key is created at random on the
//! first start and kept in the config folder (`.session-key`, mode 600).

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Redirect, Response},
    Json,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Name of the session cookie.
pub const COOKIE_NAME: &str = "ft_session";
/// Name of the key file in the config folder.
const KEY_FILE: &str = ".session-key";
/// Validity with "Stay signed in": 30 days (also the cookie's Max-Age).
const REMEMBER_SECS: u64 = 30 * 24 * 3600;
/// Validity of a browser-session cookie on the server side.
const SESSION_SECS: u64 = 7 * 24 * 3600;
/// Failed attempts per IP within `WINDOW` before the login is blocked.
const MAX_FAILS: usize = 5;
const WINDOW: Duration = Duration::from_secs(10 * 60);

/// User name and password from the environment, with the signing key.
pub struct Login {
    user: String,
    /// Display name (`LOGIN_NAME`), only shown in the profile menu.
    name: Option<String>,
    password: String,
    key: [u8; 32],
    /// Hash of user and password, part of every signature.
    fingerprint: [u8; 32],
}

impl Login {
    pub fn new(user: impl Into<String>, password: impl Into<String>, key: [u8; 32]) -> Self {
        let user = user.into();
        let password = password.into();
        let mut h = Sha256::new();
        h.update(user.as_bytes());
        h.update([0u8]);
        h.update(password.as_bytes());
        Self {
            fingerprint: h.finalize().into(),
            user,
            name: None,
            password,
            key,
        }
    }

    /// Set the display name (empty means none).
    pub fn with_name(mut self, name: Option<String>) -> Self {
        self.name = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());
        self
    }

    fn mac(&self, expiry: u64) -> HmacSha256 {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC takes any key length");
        mac.update(b"ft-session-v1|");
        mac.update(expiry.to_string().as_bytes());
        mac.update(b"|");
        mac.update(&self.fingerprint);
        mac
    }

    /// Cookie value valid until `expiry` (Unix seconds).
    pub fn token(&self, expiry: u64) -> String {
        let sig = self.mac(expiry).finalize().into_bytes();
        format!("{}.{}", expiry, hex(&sig))
    }

    /// Whether `token` was signed by this login and has not expired at `now`.
    pub fn verify(&self, token: &str, now: u64) -> bool {
        let Some((expiry, sig)) = token.split_once('.') else {
            return false;
        };
        let Ok(expiry) = expiry.parse::<u64>() else {
            return false;
        };
        let Some(sig) = unhex(sig) else {
            return false;
        };
        expiry > now && self.mac(expiry).verify_slice(&sig).is_ok()
    }

    /// Compare in constant time (hashes, so the length leaks nothing either).
    pub fn check(&self, user: &str, password: &str) -> bool {
        let eq = |a: &str, b: &str| -> bool {
            let a: [u8; 32] = Sha256::digest(a.as_bytes()).into();
            let b: [u8; 32] = Sha256::digest(b.as_bytes()).into();
            a.ct_eq(&b).into()
        };
        // Both comparisons always run
        let u = eq(user, &self.user);
        let p = eq(password, &self.password);
        u & p
    }
}

/// Login state shared by the middleware and the login API.
pub struct Auth {
    /// `None`: login off.
    login: Option<Login>,
    /// Failed attempts per client IP.
    fails: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl Auth {
    pub fn new(login: Option<Login>) -> Arc<Self> {
        Arc::new(Self {
            login,
            fails: Mutex::new(HashMap::new()),
        })
    }

    /// Login from `LOGIN_USER` (default `admin`) and `LOGIN_PASSWORD`; the key
    /// file is read from or written to `config_dir`.
    pub fn from_env(config_dir: &Path) -> Arc<Self> {
        let password = std::env::var("LOGIN_PASSWORD").unwrap_or_default();
        if password.is_empty() {
            tracing::info!("Login off (LOGIN_PASSWORD not set)");
            return Self::new(None);
        }
        let user = std::env::var("LOGIN_USER")
            .ok()
            .map(|u| u.trim().to_string())
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| "admin".to_string());
        tracing::info!("Login on for user '{}'", user);
        let name = std::env::var("LOGIN_NAME").ok();
        Self::new(Some(
            Login::new(user, password, load_key(config_dir)).with_name(name),
        ))
    }

    pub fn enabled(&self) -> bool {
        self.login.is_some()
    }

    /// Whether the request carries a valid session (always true when off).
    fn signed_in(&self, headers: &HeaderMap) -> bool {
        match &self.login {
            None => true,
            Some(login) => cookie(headers, COOKIE_NAME).is_some_and(|t| login.verify(&t, now())),
        }
    }

    /// Minutes until `ip` may try again, if it is blocked.
    fn blocked(&self, ip: IpAddr, at: Instant) -> Option<u64> {
        let mut fails = self.fails.lock().unwrap();
        let list = fails.entry(ip).or_default();
        list.retain(|t| at.duration_since(*t) < WINDOW);
        if list.len() < MAX_FAILS {
            return None;
        }
        let free_at = list[list.len() - MAX_FAILS] + WINDOW;
        let secs = free_at.saturating_duration_since(at).as_secs();
        Some(secs.div_ceil(60).max(1))
    }

    fn record_fail(&self, ip: IpAddr, at: Instant) {
        let mut fails = self.fails.lock().unwrap();
        fails.entry(ip).or_default().push(at);
        // Keep the map small: drop IPs without recent failures
        fails.retain(|_, list| list.iter().any(|t| at.duration_since(*t) < WINDOW));
    }

    fn clear_fails(&self, ip: IpAddr) {
        self.fails.lock().unwrap().remove(&ip);
    }
}

/// Read the signing key, or create it (random, mode 600). If it cannot be
/// written, a key for this run only is used: sessions end with a restart.
fn load_key(dir: &Path) -> [u8; 32] {
    let path = dir.join(KEY_FILE);
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Some(bytes) = unhex(text.trim()) {
            if let Ok(key) = <[u8; 32]>::try_from(bytes.as_slice()) {
                return key;
            }
        }
        tracing::warn!("Ignoring invalid session key in {}", path.display());
    }
    let key: [u8; 32] = rand::random();
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut file = opts.open(&path)?;
        std::io::Write::write_all(&mut file, hex(&key).as_bytes())
    };
    match write() {
        Ok(()) => tracing::info!("Session key created in {}", path.display()),
        Err(e) => tracing::warn!(
            "Cannot write session key to {}: {} - logins end with a restart",
            path.display(),
            e
        ),
    }
    key
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Value of a cookie from the `Cookie` headers.
fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v.to_string())
}

/// Whether the visitor came over HTTPS (behind the reverse proxy).
fn is_https(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("https"))
}

/// Whether a peer may be the reverse proxy: on this machine or in a private
/// network (Docker network, LAN). Only then is `X-Forwarded-For` believed.
fn is_proxy(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_proxy(IpAddr::V4(v4));
            }
            // ::1 and unique local fc00::/7
            v6.is_loopback() || (v6.segments()[0] & 0xfe00) == 0xfc00
        }
    }
}

/// Client IP: the last `X-Forwarded-For` entry (the one our proxy added) if
/// the socket peer is a proxy, else the socket IP.
pub fn client_ip(peer: Option<IpAddr>, headers: &HeaderMap) -> IpAddr {
    let fallback = IpAddr::from([0, 0, 0, 0]);
    let Some(peer) = peer else {
        return fallback;
    };
    if is_proxy(peer) {
        let forwarded = headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(','))
            .filter_map(|s| s.trim().parse::<IpAddr>().ok())
            .last();
        if let Some(ip) = forwarded {
            return ip;
        }
    }
    peer
}

/// `next` as a path on this server, `/` for anything else (no open redirect).
pub fn safe_next(next: Option<&str>) -> String {
    match next {
        Some(n)
            if n.starts_with('/')
                && !n.starts_with("//")
                && !n.starts_with("/\\")
                && !n.chars().any(|c| c.is_control()) =>
        {
            n.to_string()
        }
        _ => "/".to_string(),
    }
}

/// Paths reachable without a login.
fn is_public(path: &str) -> bool {
    matches!(
        path,
        "/api/health"
            | "/login"
            | "/favicon.svg"
            | "/api/auth/login"
            | "/api/auth/logout"
            | "/api/auth/session"
    ) || path.starts_with("/brand/") // logos for the login page and link previews
}

/// Middleware: lets signed-in requests (and everything when the login is
/// off) through. Otherwise the API answers 401 JSON and the browser is sent
/// to `/login?next=<path>`.
pub async fn require_login(State(auth): State<Arc<Auth>>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    if !auth.enabled() || is_public(path) || auth.signed_in(req.headers()) {
        return next.run(req).await;
    }
    if path.starts_with("/api/") || path == "/api" {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "unauthorized", "message": "Login required" })),
        )
            .into_response();
    }
    let target = req
        .uri()
        .path_and_query()
        .map_or("/", |pq| pq.as_str())
        .to_string();
    let location = if target == "/" {
        "/login".to_string()
    } else {
        format!("/login?next={}", urlencoding::encode(&target))
    };
    Redirect::to(&location).into_response()
}

#[derive(Deserialize)]
pub struct NextQuery {
    next: Option<String>,
}

/// `GET /login`: the login page (frontend/dist/login.html). Signed-in
/// visitors, and everyone when the login is off, go straight on.
pub async fn login_page(
    State(auth): State<Arc<Auth>>,
    axum::extract::Query(q): axum::extract::Query<NextQuery>,
    headers: HeaderMap,
) -> Response {
    if !auth.enabled() || auth.signed_in(&headers) {
        return Redirect::to(&safe_next(q.next.as_deref())).into_response();
    }
    match tokio::fs::read_to_string("frontend/dist/login.html").await {
        Ok(html) => (
            [(header::CACHE_CONTROL, HeaderValue::from_static("no-store"))],
            Html(html),
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Login page not built").into_response(),
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    remember: bool,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub enabled: bool,
    pub user: Option<String>,
    /// Display name for the profile button; the user name without `LOGIN_NAME`.
    pub name: Option<String>,
    pub version: &'static str,
}

/// `POST /api/auth/login` with `{username, password, remember}`.
pub async fn login(State(auth): State<Arc<Auth>>, req: Request) -> Response {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip());
    let ip = client_ip(peer, req.headers());
    let https = is_https(req.headers());
    let body = match axum::body::to_bytes(req.into_body(), 16 * 1024).await {
        Ok(b) => b,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };
    let Ok(body) = serde_json::from_slice::<LoginRequest>(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "bad_request" })),
        )
            .into_response();
    };
    let Some(login) = &auth.login else {
        return Json(serde_json::json!({ "ok": true, "enabled": false })).into_response();
    };

    let at = Instant::now();
    if let Some(minutes) = auth.blocked(ip, at) {
        tracing::warn!("Login blocked for {} ({} min)", ip, minutes);
        return too_many(minutes);
    }
    if !login.check(body.username.trim(), &body.password) {
        auth.record_fail(ip, at);
        tracing::warn!("Failed login for '{}' from {}", body.username.trim(), ip);
        if let Some(minutes) = auth.blocked(ip, at) {
            return too_many(minutes);
        }
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "wrong" })),
        )
            .into_response();
    }
    auth.clear_fails(ip);
    tracing::info!("Login of '{}' from {}", login.user, ip);

    let lifetime = if body.remember {
        REMEMBER_SECS
    } else {
        SESSION_SECS
    };
    let token = login.token(now() + lifetime);
    let mut cookie = format!("{}={}; Path=/; HttpOnly; SameSite=Lax", COOKIE_NAME, token);
    if body.remember {
        cookie.push_str(&format!("; Max-Age={}", REMEMBER_SECS));
    }
    if https {
        cookie.push_str("; Secure");
    }
    set_cookie(
        Json(serde_json::json!({ "ok": true })).into_response(),
        &cookie,
    )
}

fn too_many(minutes: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, (minutes * 60).to_string())],
        Json(serde_json::json!({ "error": "too_many", "minutes": minutes })),
    )
        .into_response()
}

fn set_cookie(mut res: Response, cookie: &str) -> Response {
    if let Ok(v) = HeaderValue::from_str(cookie) {
        res.headers_mut().append(header::SET_COOKIE, v);
    }
    res
}

/// `POST /api/auth/logout`: delete the cookie.
pub async fn logout(headers: HeaderMap) -> Response {
    let mut cookie = format!(
        "{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        COOKIE_NAME
    );
    if is_https(&headers) {
        cookie.push_str("; Secure");
    }
    set_cookie(
        Json(serde_json::json!({ "ok": true })).into_response(),
        &cookie,
    )
}

/// `GET /api/auth/session`: `{enabled, user, name, version}`; `user` and
/// `name` only when signed in. The version is shown on the login card.
pub async fn session(State(auth): State<Arc<Auth>>, headers: HeaderMap) -> Json<SessionInfo> {
    let (user, name) = match &auth.login {
        Some(login) if auth.signed_in(&headers) => (
            Some(login.user.clone()),
            Some(login.name.clone().unwrap_or_else(|| login.user.clone())),
        ),
        _ => (None, None),
    };
    Json(SessionInfo {
        enabled: auth.enabled(),
        user,
        name,
        version: crate::web::VersionInfo::current().version,
    })
}

/// Tab icon of the login and share pages (same logo as on the login card).
pub const FAVICON_SVG: &str = include_str!("favicon.svg");

/// `GET /favicon.svg`, on the main and the share port.
pub async fn favicon() -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        FAVICON_SVG,
    )
        .into_response()
}

/// Routes of the login (page and API), with their own state.
pub fn routes(auth: Arc<Auth>) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/login", get(login_page))
        .route("/favicon.svg", get(favicon))
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/session", get(session))
        .with_state(auth)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, middleware, routing::get, Router};
    use tower::ServiceExt;

    fn login() -> Login {
        Login::new("roger", "secret", [7u8; 32])
    }

    #[test]
    fn token_roundtrip_and_expiry() {
        let l = login();
        let t = l.token(1000);
        assert!(l.verify(&t, 999));
        assert!(!l.verify(&t, 1000));
        assert!(!l.verify("1000.00", 1));
        assert!(!l.verify("garbage", 1));
        // A changed expiry breaks the signature
        let forged = t.replacen("1000", "9999", 1);
        assert!(!l.verify(&forged, 999));
    }

    #[test]
    fn changed_credentials_invalidate_tokens() {
        let t = login().token(1000);
        assert!(!Login::new("roger", "other", [7u8; 32]).verify(&t, 1));
        assert!(!Login::new("anna", "secret", [7u8; 32]).verify(&t, 1));
        assert!(!Login::new("roger", "secret", [8u8; 32]).verify(&t, 1));
    }

    #[test]
    fn checks_user_and_password() {
        let l = login();
        assert!(l.check("roger", "secret"));
        assert!(!l.check("roger", "Secret"));
        assert!(!l.check("anna", "secret"));
        assert!(!l.check("", ""));
    }

    #[test]
    fn next_is_only_a_local_path() {
        assert_eq!(safe_next(Some("/output/a.mp4?x=1")), "/output/a.mp4?x=1");
        assert_eq!(safe_next(Some("//evil.example")), "/");
        assert_eq!(safe_next(Some("/\\evil.example")), "/");
        assert_eq!(safe_next(Some("https://evil.example")), "/");
        assert_eq!(safe_next(Some("/a\nb")), "/");
        assert_eq!(safe_next(None), "/");
    }

    #[test]
    fn forwarded_for_only_from_proxy() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        let proxy: IpAddr = "172.18.0.5".parse().unwrap();
        let outside: IpAddr = "8.8.8.8".parse().unwrap();
        assert_eq!(
            client_ip(Some(proxy), &h),
            "5.6.7.8".parse::<IpAddr>().unwrap()
        );
        assert_eq!(client_ip(Some(outside), &h), outside);
        assert_eq!(client_ip(Some(proxy), &HeaderMap::new()), proxy);
    }

    #[test]
    fn blocks_after_five_failures() {
        let auth = Auth::new(Some(login()));
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        let t0 = Instant::now();
        for _ in 0..4 {
            auth.record_fail(ip, t0);
        }
        assert_eq!(auth.blocked(ip, t0), None);
        auth.record_fail(ip, t0);
        assert_eq!(auth.blocked(ip, t0), Some(10));
        assert_eq!(
            auth.blocked(ip, t0 + Duration::from_secs(9 * 60 + 1)),
            Some(1)
        );
        assert_eq!(auth.blocked(ip, t0 + WINDOW), None);
        // Other IPs are not affected
        assert_eq!(auth.blocked("10.0.0.2".parse().unwrap(), t0), None);
    }

    fn app(auth: Arc<Auth>) -> Router {
        Router::new()
            .route("/api/health", get(|| async { "OK" }))
            .route("/api/people", get(|| async { "people" }))
            .route("/output/x.mp4", get(|| async { "video" }))
            .merge(routes(auth.clone()))
            .layer(middleware::from_fn_with_state(auth, require_login))
    }

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: &str,
    ) -> Response {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        if !body.is_empty() {
            req = req.header(header::CONTENT_TYPE, "application/json");
        }
        app.clone()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn without_password_everything_is_open() {
        let app = app(Auth::new(None));
        assert_eq!(
            call(&app, "GET", "/api/people", None, "").await.status(),
            StatusCode::OK
        );
        assert_eq!(
            call(&app, "GET", "/output/x.mp4", None, "").await.status(),
            StatusCode::OK
        );
        let res = call(&app, "GET", "/api/auth/session", None, "").await;
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["enabled"], false);
    }

    #[tokio::test]
    async fn protected_until_login() {
        let app = app(Auth::new(Some(login())));
        assert_eq!(
            call(&app, "GET", "/api/health", None, "").await.status(),
            StatusCode::OK
        );
        assert_eq!(
            call(&app, "GET", "/api/people", None, "").await.status(),
            StatusCode::UNAUTHORIZED
        );
        let res = call(&app, "GET", "/output/x.mp4", None, "").await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            res.headers()[header::LOCATION],
            "/login?next=%2Foutput%2Fx.mp4"
        );

        let wrong = call(
            &app,
            "POST",
            "/api/auth/login",
            None,
            r#"{"username":"roger","password":"no"}"#,
        )
        .await;
        assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);

        let ok = call(
            &app,
            "POST",
            "/api/auth/login",
            None,
            r#"{"username":"roger","password":"secret","remember":true}"#,
        )
        .await;
        assert_eq!(ok.status(), StatusCode::OK);
        let set = ok.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .to_string();
        assert!(
            set.contains("HttpOnly") && set.contains("SameSite=Lax") && set.contains("Max-Age=")
        );
        let cookie = set.split(';').next().unwrap();
        assert_eq!(
            call(&app, "GET", "/api/people", Some(cookie), "")
                .await
                .status(),
            StatusCode::OK
        );

        let res = call(&app, "GET", "/api/auth/session", Some(cookie), "").await;
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["enabled"], true);
        assert_eq!(v["user"], "roger");
        // Without LOGIN_NAME the user name is the display name
        assert_eq!(v["name"], "roger");
    }

    #[tokio::test]
    async fn session_has_display_name() {
        let named = Login::new("roger", "secret", [7u8; 32]).with_name(Some(" Roger Heckly ".into()));
        let token = named.token(now() + 60);
        let app = app(Auth::new(Some(named)));
        let cookie = format!("{}={}", COOKIE_NAME, token);
        let res = call(&app, "GET", "/api/auth/session", Some(&cookie), "").await;
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["user"], "roger");
        assert_eq!(v["name"], "Roger Heckly");
        // Not signed in: neither user nor name
        let res = call(&app, "GET", "/api/auth/session", None, "").await;
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v["user"].is_null() && v["name"].is_null());
    }

    #[tokio::test]
    async fn too_many_attempts_get_429() {
        let app = app(Auth::new(Some(login())));
        for _ in 0..4 {
            let r = call(
                &app,
                "POST",
                "/api/auth/login",
                None,
                r#"{"username":"roger","password":"no"}"#,
            )
            .await;
            assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        }
        let r = call(
            &app,
            "POST",
            "/api/auth/login",
            None,
            r#"{"username":"roger","password":"no"}"#,
        )
        .await;
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        // Even the right password waits now
        let r = call(
            &app,
            "POST",
            "/api/auth/login",
            None,
            r#"{"username":"roger","password":"secret"}"#,
        )
        .await;
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
