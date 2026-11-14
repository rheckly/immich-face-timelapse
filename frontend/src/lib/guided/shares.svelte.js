/**
 * Shared links (see src/web/shares.rs): one list for the result panel,
 * the share panel and the settings.
 */
import { API } from '../constants.js';
import { i18n } from './i18n.svelte.js';

export const shares = $state({ loaded: false, error: '', public_url: '', share_port: 5001, list: [] });

async function call(url, method = 'GET', body) {
  const res = await fetch(url, {
    method,
    headers: body ? { 'Content-Type': 'application/json' } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error((await res.text()) || res.statusText);
  return res.status === 204 ? null : res.json();
}

export async function loadShares() {
  try {
    const data = await call(API.shares);
    shares.public_url = data.public_url;
    shares.share_port = data.share_port;
    shares.list = data.shares;
    shares.error = '';
  } catch (e) {
    shares.error = e.message;
  }
  shares.loaded = true;
}

export const sharesOf = (folder) =>
  shares.list.filter((s) => s.folder === folder).sort((a, b) => b.created_at.localeCompare(a.created_at));

export async function createShare(folder, title, days, download = false, photos = false) {
  const s = await call(API.shares, 'POST', { folder, title, days, download, photos });
  shares.list = [...shares.list, s];
  return s;
}

export async function updateShare(token, title, days) {
  const s = await call(`${API.shares}/${token}`, 'PATCH', { title, days });
  shares.list = shares.list.map((x) => (x.token === token ? s : x));
}

/** Allow or stop a download on a link (`{ download }` or `{ photos }`); validity stays. */
export async function setShareDownload(token, change) {
  const s = await call(`${API.shares}/${token}`, 'PATCH', change);
  shares.list = shares.list.map((x) => (x.token === token ? s : x));
}

export async function revokeShare(token) {
  await call(`${API.shares}/${token}`, 'DELETE');
  shares.list = shares.list.filter((x) => x.token !== token);
}

export async function setPublicUrl(url) {
  const data = await call(`${API.shares}/settings`, 'PUT', { public_url: url });
  shares.public_url = data.public_url;
  await loadShares(); // links get their new address
}

/** Validity choices in days; null = until revoked. */
export const DURATIONS = [1, 7, 30, 90, 365, null];

export function formatDate(iso) {
  return new Date(iso).toLocaleDateString(i18n.lang === 'de' ? 'de-CH' : 'en-GB', { day: '2-digit', month: '2-digit', year: 'numeric' });
}

export async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const el = document.createElement('textarea');
    el.value = text;
    document.body.appendChild(el);
    el.select();
    const ok = document.execCommand('copy');
    el.remove();
    return ok;
  }
}
