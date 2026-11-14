/**
 * Shared state of the quality preview: the sample photos of the selection and the
 * last preview from POST /api/preview. Shared so the quality pane and a filter pane
 * show the same photo and don't ask the server twice.
 */
import { API } from '../constants.js';
import { toPayload } from './config.js';

export const preview = $state({
  selKey: '',
  samples: [], // [{ id, timestamp }]
  total: 0,
  index: 0,
  loadingSamples: false,
  samplesError: '',
  result: null,
  resultKey: '',
  loading: false,
  error: '',
});

async function readError(res) {
  const text = await res.text().catch(() => '');
  return text || `HTTP ${res.status}`;
}

/** Load sample photos of the selection, unless they are already there. */
export async function loadSamples(sel) {
  const key = JSON.stringify(sel);
  if (preview.selKey === key) return;
  preview.selKey = key;
  preview.samples = [];
  preview.total = 0;
  preview.index = 0;
  preview.result = null;
  preview.resultKey = '';
  preview.error = '';
  preview.samplesError = '';
  if (!sel.personId) return;

  const params = new URLSearchParams({ person_id: sel.personId, count: '24' });
  for (const [name, list] of [
    ['album_ids', sel.albumIds],
    ['people_ids', sel.peopleIds],
    ['recipient_ids', sel.recipientIds],
  ]) {
    if (list.length) params.set(name, list.join(','));
  }
  if (sel.dateFrom) params.set('date_from', sel.dateFrom);
  if (sel.dateTo) params.set('date_to', sel.dateTo);

  preview.loadingSamples = true;
  try {
    const res = await fetch(`${API.previewSamples}?${params}`);
    if (!res.ok) throw new Error(await readError(res));
    const data = await res.json();
    if (preview.selKey !== key) return;
    preview.samples = data.samples;
    preview.total = data.total;
  } catch (e) {
    if (preview.selKey === key) preview.samplesError = e.message;
  } finally {
    if (preview.selKey === key) preview.loadingSamples = false;
  }
}

/** The settings that change the resulting frame. Threshold changes are judged in the browser. */
export function frameKey(config) {
  const p = config.processing;
  return JSON.stringify([
    p.use_preview,
    Number(p.output.size),
    p.output.aspect_ratio ?? '1:1',
    Number(p.alignment.eye_distance),
    p.timestamp,
  ]);
}

let timer;
let seq = 0;

/** Ask for the preview of the current photo; settings changes wait ~300 ms. */
export function requestPreview(config, personId) {
  const sample = preview.samples[preview.index];
  if (!sample || !personId) return;
  const key = `${sample.id}|${frameKey(config)}`;
  if (key === preview.resultKey) return;
  const samePhoto = preview.resultKey.startsWith(`${sample.id}|`);
  preview.resultKey = key;
  if (!samePhoto) preview.result = null;
  const body = JSON.stringify({
    asset_id: sample.id,
    person_id: personId,
    timestamp: sample.timestamp,
    processing: toPayload(config).processing,
  });
  clearTimeout(timer);
  timer = setTimeout(
    async () => {
      const my = ++seq;
      preview.loading = true;
      preview.error = '';
      const started = performance.now();
      try {
        const res = await fetch(API.preview, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body });
        if (!res.ok) throw new Error(await readError(res));
        const data = await res.json();
        if (my !== seq) return;
        preview.result = { ...data, roundtrip: Math.round(performance.now() - started) };
      } catch (e) {
        if (my !== seq) return;
        preview.result = null;
        preview.error = e.message;
      } finally {
        if (my === seq) preview.loading = false;
      }
    },
    samePhoto ? 300 : 0
  );
}

/** Show another photo of the samples. */
export function pick(index) {
  if (!preview.samples.length) return;
  preview.index = (index + preview.samples.length) % preview.samples.length;
}
