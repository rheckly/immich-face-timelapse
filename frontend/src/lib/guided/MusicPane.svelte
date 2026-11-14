<script>
  import { t, fmt } from './i18n.svelte.js';
  import { formatDuration } from './config.js';
  import { API } from '../constants.js';

  let { config = $bindable(), music, loading = false, error = null, onrefresh } = $props();

  const ACCEPT = '.mp3,.m4a,.aac,.wav,.flac,.ogg,.opus,audio/*';

  let tracks = $derived(config.video.music_tracks ?? []);
  let missing = $derived(music.available ? tracks.filter((n) => !music.tracks.some((t) => t.name === n)) : []);
  let totalSecs = $derived(
    tracks.reduce((sum, n) => sum + (music.tracks.find((t) => t.name === n)?.duration_secs ?? 0), 0)
  );

  // Search: every word must appear in the file name; chosen tracks always stay visible
  let query = $state('');
  let words = $derived(query.toLowerCase().split(/\s+/).filter(Boolean));
  let shown = $derived(
    words.length === 0
      ? music.tracks
      : music.tracks.filter((tr) => tracks.includes(tr.name) || words.every((w) => tr.name.toLowerCase().includes(w)))
  );

  function toggleTrack(name) {
    config.video.music_tracks = tracks.includes(name) ? tracks.filter((t) => t !== name) : [...tracks, name];
  }

  // Upload: each file is sent as the request body; uploaded tracks are chosen right away
  let fileInput = $state();
  let uploading = $state('');
  let uploadMsg = $state({ ok: '', err: '' });

  async function upload(files) {
    const list = [...(files ?? [])];
    if (list.length === 0) return;
    const done = [];
    const errors = [];
    for (const file of list) {
      uploading = file.name;
      try {
        const res = await fetch(`${API.music}/${encodeURIComponent(file.name)}`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: file,
        });
        if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
        done.push(file.name);
      } catch (e) {
        errors.push(`${file.name}: ${e.message}`);
      }
    }
    uploading = '';
    if (fileInput) fileInput.value = '';
    if (done.length) config.video.music_tracks = [...tracks, ...done.filter((n) => !tracks.includes(n))];
    uploadMsg = { ok: done.length ? t('music.uploaded', { n: done.length }) : '', err: errors.join(' · ') };
    if (done.length) onrefresh?.();
  }

  let dragOver = $state(false);
  function onDrop(e) {
    e.preventDefault();
    dragOver = false;
    upload(e.dataTransfer?.files);
  }
</script>

<div class="sectionHead">
  <p class="muted">{t('music.intro')}</p>
  <button type="button" onclick={onrefresh} disabled={loading}>{loading ? t('common.loading') : t('music.refresh')}</button>
</div>

{#if music.available}
  <div
    class="musicTools"
    class:dragOver
    role="group"
    aria-label={t('music.upload')}
    ondragover={(e) => {
      e.preventDefault();
      dragOver = true;
    }}
    ondragleave={() => (dragOver = false)}
    ondrop={onDrop}
  >
    <input type="search" placeholder={t('music.search')} aria-label={t('music.search')} bind:value={query} />
    <button type="button" onclick={() => fileInput?.click()} disabled={!!uploading}>
      {uploading ? t('music.uploading') : t('music.upload')}
    </button>
    <input bind:this={fileInput} type="file" accept={ACCEPT} multiple hidden onchange={(e) => upload(e.currentTarget.files)} />
  </div>
  {#if uploading}<p class="muted">{t('music.uploadingFile', { name: uploading })}</p>{/if}
  {#if uploadMsg.ok}<p class="muted">{uploadMsg.ok}</p>{/if}
  {#if uploadMsg.err}<p class="err">{uploadMsg.err}</p>{/if}
{/if}

{#if tracks.length > 0}
  <p class="muted">{t('music.length', { len: formatDuration(totalSecs) })}</p>
{/if}

{#if error}
  <p class="err mt">{error}</p>
{:else if !music.available}
  <p class="muted mt">{t('music.noFolder')} <code>/app/music</code>.</p>
{:else if music.tracks.length === 0}
  <p class="muted mt">{t('music.noFiles')}</p>
{:else}
  {#if words.length > 0}
    <p class="muted">{t('music.found', { n: fmt(shown.length), total: fmt(music.tracks.length) })}</p>
  {/if}
  <ul class="music">
    {#each shown as track (track.name)}
      {@const order = tracks.indexOf(track.name)}
      <li>
        <label class="check">
          <input type="checkbox" checked={order >= 0} onchange={() => toggleTrack(track.name)} />
          <span class="order">{order >= 0 ? order + 1 : ''}</span>
          <span class="name">{track.name}</span>
          <span class="muted">{formatDuration(track.duration_secs)}</span>
        </label>
      </li>
    {/each}
  </ul>
{/if}

{#if missing.length > 0}
  <p class="err mt">
    {t('music.missing')} {missing.join(', ')}
    <button type="button" onclick={() => (config.video.music_tracks = tracks.filter((n) => !missing.includes(n)))}>{t('common.remove')}</button>
  </p>
{/if}

<p class="muted mt">{t('music.lengthWhere')}</p>

<style>
  .musicTools {
    display: flex;
    gap: 8px;
    margin: 8px 0;
    padding: 4px;
    border: 1px dashed transparent;
    border-radius: 8px;
  }
  .musicTools.dragOver {
    border-color: var(--accent, #4f46e5);
  }
  .musicTools input[type='search'] {
    flex: 1;
    min-width: 0;
  }
</style>
