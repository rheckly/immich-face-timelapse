// Release notes for one version, from the changelog shown in About → Changelog.
//
//   node scripts/release-notes.mjs 1.0.2 > notes.md
//
// English first, German in a collapsible block. Exits with an error when the
// version is missing or has no changes.

import { CHANGELOG } from '../frontend/src/lib/guided/changelog.js';

const IMAGE = 'ghcr.io/rheckly/immich-face-timelapse';
const TYPES = {
  NEW: ['New', 'Neu'],
  CHG: ['Changed', 'Geändert'],
  FIX: ['Fixed', 'Behoben'],
  LNG: ['Language', 'Sprache'],
};

const version = (process.argv[2] ?? '').replace(/^ft-v/, '');
const rel = CHANGELOG.find((r) => r.version === version);
if (!rel || !rel.changes?.length) {
  console.error(`No changelog entry with changes for version "${version}"`);
  process.exit(1);
}

// Grouped by type in a fixed order, the order inside a type as in the changelog
function section(lang) {
  const i = lang === 'en' ? 0 : 1;
  const out = [];
  for (const [type, names] of Object.entries(TYPES)) {
    const items = rel.changes.filter((c) => c.type === type);
    if (!items.length) continue;
    out.push(`### ${names[i]}`, '');
    for (const c of items) out.push(`- ${c[lang] ?? c.de}`);
    out.push('');
  }
  return out.join('\n');
}

const lines = [
  `Docker image: \`${IMAGE}:${version}\``,
  '',
  rel.note?.en ? rel.note.en + '\n' : '',
  section('en'),
  '<details>',
  '<summary>Deutsch</summary>',
  '',
  rel.note?.de ? rel.note.de + '\n' : '',
  section('de'),
  '</details>',
  '',
];
process.stdout.write(lines.filter((l, i, a) => !(l === '' && a[i - 1] === '')).join('\n'));
