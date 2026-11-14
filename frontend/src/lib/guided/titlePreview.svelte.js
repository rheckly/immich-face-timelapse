// Preview stills of the intro and outro cards, shared by the boxes on the left
// and the panels. GuidedApp calls `requestPreview` when a card changes.
import { API } from '../constants.js';
import { cardPayload } from './config.js';

export const previews = $state({
  intro: { url: '', state: '' }, // state: '' | 'loading' | error text
  outro: { url: '', state: '' },
});

const seq = { intro: 0, outro: 0 };
const last = { intro: '', outro: '' };

/** JSON body for the preview of a card, '' when the card is off. */
export function previewBody(card) {
  return card?.enabled ? JSON.stringify({ card: { ...cardPayload(card), enabled: true } }) : '';
}

export async function requestPreview(which, body) {
  if (!body || body === last[which]) return;
  last[which] = body;
  const n = ++seq[which];
  const p = previews[which];
  p.state = 'loading';
  try {
    const res = await fetch(API.titlePreview, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body });
    if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
    const blob = await res.blob();
    if (n !== seq[which]) return;
    if (p.url) URL.revokeObjectURL(p.url);
    p.url = URL.createObjectURL(blob);
    p.state = '';
  } catch (e) {
    if (n !== seq[which]) return;
    last[which] = ''; // try again on the next change
    p.state = e.message || 'error';
  }
}
