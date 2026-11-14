//! Optional page-view statistics with [Umami](https://umami.is).
//!
//! Off unless both `UMAMI_URL` (address of the Umami server, or the full URL
//! of its tracker script) and `UMAMI_WEBSITE_ID` are set. The app loads the
//! tracker through [`get_analytics`]; the public share pages embed it with
//! [`share_tag`], which reports the page without its secret token.

use axum::Json;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Umami {
    /// URL of the tracker script
    pub script: String,
    pub website_id: String,
}

/// Umami settings from the environment, `None` when not configured.
pub fn umami() -> Option<Umami> {
    from_values(
        std::env::var("UMAMI_URL").ok().as_deref(),
        std::env::var("UMAMI_WEBSITE_ID").ok().as_deref(),
    )
}

fn from_values(url: Option<&str>, id: Option<&str>) -> Option<Umami> {
    let url = url?.trim().trim_end_matches('/');
    let id = id?.trim();
    let url_ok = (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() > "https://".len()
        && !url
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | '\\'));
    let id_ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !url_ok || !id_ok {
        if !url.is_empty() || !id.is_empty() {
            tracing::warn!("UMAMI_URL or UMAMI_WEBSITE_ID is invalid, statistics are off");
        }
        return None;
    }
    let script = if url.ends_with(".js") {
        url.to_string()
    } else {
        format!("{url}/script.js")
    };
    Some(Umami {
        script,
        website_id: id.to_string(),
    })
}

/// `GET /api/analytics`: tracker for the app, `null` when off.
pub async fn get_analytics() -> Json<Option<Umami>> {
    Json(umami())
}

/// Script tag for a public share page. The share link holds a secret token,
/// so the page is reported as `/s/` instead of its real address.
pub fn share_tag() -> String {
    match umami() {
        Some(u) => tag(&u),
        None => String::new(),
    }
}

fn tag(u: &Umami) -> String {
    format!(
        "<script defer src=\"{}\" data-website-id=\"{}\" data-auto-track=\"false\" \
         onload=\"umami.track(function (p) {{ p.url = '/s/'; return p; }})\"></script>\n",
        u.script, u.website_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_both_values() {
        assert_eq!(from_values(None, Some("abc")), None);
        assert_eq!(from_values(Some("https://umami.example.com"), None), None);
        assert_eq!(from_values(Some(""), Some("")), None);
    }

    #[test]
    fn builds_script_url() {
        let u = from_values(Some(" https://umami.example.com/ "), Some("1234-abcd")).unwrap();
        assert_eq!(u.script, "https://umami.example.com/script.js");
        assert_eq!(u.website_id, "1234-abcd");
        let u = from_values(Some("https://stats.example.com/x.js"), Some("1")).unwrap();
        assert_eq!(u.script, "https://stats.example.com/x.js");
    }

    #[test]
    fn rejects_unsafe_values() {
        assert_eq!(from_values(Some("javascript:alert(1)"), Some("1")), None);
        assert_eq!(
            from_values(Some("https://a.com/\"><script>"), Some("1")),
            None
        );
        assert_eq!(
            from_values(Some("https://a.com"), Some("1\" onload=\"x")),
            None
        );
    }

    #[test]
    fn share_tag_hides_token() {
        let u = from_values(Some("https://umami.example.com"), Some("abc")).unwrap();
        let t = tag(&u);
        assert!(t.contains("data-auto-track=\"false\""));
        assert!(t.contains("p.url = '/s/'"));
    }
}
