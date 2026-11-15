/**
 * Optional login (LOGIN_USER / LOGIN_PASSWORD on the server, see
 * src/web/auth.rs). Every API answer 401 means the session is gone: the
 * browser goes to the login page and comes back here afterwards.
 */

const LOGIN = '/login';

/** Send the browser to the login page, back to the current page afterwards. */
export function toLogin() {
  const here = location.pathname + location.search + location.hash;
  location.assign(here === '/' ? LOGIN : `${LOGIN}?next=${encodeURIComponent(here)}`);
}

/** Wrap window.fetch: a 401 from /api (except the login API) leads to the login page. */
export function installAuthRedirect() {
  const original = window.fetch.bind(window);
  let leaving = false;
  window.fetch = async (input, init) => {
    const res = await original(input, init);
    if (res.status === 401 && !leaving) {
      const url = new URL(input instanceof Request ? input.url : String(input), location.href);
      if (url.origin === location.origin && url.pathname.startsWith('/api/') && !url.pathname.startsWith('/api/auth/')) {
        leaving = true;
        toLogin();
      }
    }
    return res;
  };
}

/** `{enabled, user, version}`; `enabled: false` when the server has no login. */
export async function getSession() {
  try {
    const res = await fetch('/api/auth/session');
    if (res.ok) return await res.json();
  } catch {
    // No answer: treat as no login
  }
  return { enabled: false, user: null };
}

/** Sign out and show the login page. */
export async function logout() {
  try {
    await fetch('/api/auth/logout', { method: 'POST' });
  } finally {
    location.assign(LOGIN);
  }
}
