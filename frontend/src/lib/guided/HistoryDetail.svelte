<script>
  // Detail of a timelapse from the history: what it was made from, watch it,
  // or take its person, sources, period and settings over to edit it.
  import { t, fmt } from './i18n.svelte.js';
  import { API } from '../constants.js';
  import { formatSize, formatFolderVersion } from '../utils.js';
  import { formatDuration, frameSize, FILTERS } from './config.js';

  let { folder, disabled = false, onwatch, onedit, onsettings } = $props();

  let run = $state(null);
  let loadState = $state('loading'); // 'loading' | 'ok' | 'none' | 'error'
  $effect(() => {
    const name = folder.name;
    run = null;
    loadState = 'loading';
    fetch(`${API.output}/${encodeURIComponent(name)}/run`)
      .then(async (res) => {
        if (name !== folder.name) return;
        if (res.status === 404) loadState = 'none';
        else if (!res.ok) loadState = 'error';
        else {
          run = await res.json();
          loadState = 'ok';
        }
      })
      .catch(() => (loadState = 'error'));
  });

  const date = (iso) =>
    iso ? new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : '';
  let sources = $derived.by(() => {
    if (!run) return '';
    const s = run.source;
    const list = s ? [...(s.albums ?? []).map((a) => a.name), ...(s.others ?? []).map((o) => o.name)] : (run.album_names ?? []);
    return list.length ? list.join(', ') : t('albums.all');
  });
  let filters = $derived(run?.processing ? FILTERS.filter((f) => run.processing[f.key]?.enabled).map((f) => f.title) : []);
</script>

<p class="bigCount">{run?.person_name ?? folder.person ?? folder.name}</p>
<p class="muted mt">
  {#if folder.version}{formatFolderVersion(folder.version)} ·{/if}
  {t('common.nPhotos', { n: folder.image_count })} · {formatSize(folder.size_bytes)}
</p>

<div class="btnRow mt">
  <button type="button" class="primary" disabled={!folder.has_video} onclick={onwatch}>{t('hist.watch')}</button>
  <button type="button" disabled={disabled || loadState !== 'ok'} onclick={() => onedit(run)}>{t('hist.edit')}</button>
  <button type="button" disabled={disabled || !run?.processing || !run?.video} onclick={() => onsettings(run)}>{t('hist.settings')}</button>
</div>
{#if !folder.has_video}<p class="muted small">{t('hist.noVideo')}</p>{/if}
{#if loadState === 'ok'}
  <p class="muted small">{t('hist.editHint')} {t('hist.settingsHint')}</p>
{:else if loadState === 'none'}
  <p class="muted small">{t('hist.noRun')}</p>
{:else if loadState === 'error'}
  <p class="err small">{t('hist.errRun')}</p>
{/if}

{#if run}
  <h3 class="paneSection">{t('hist.details')}</h3>
  <dl class="facts">
    <dt>{t('hist.created')}</dt>
    <dd>{date(run.created)}</dd>
    <dt>{t('hist.sources')}</dt>
    <dd>{sources}</dd>
    <dt>{t('hist.period')}</dt>
    <dd>{t('period.fromTo', { from: run.date_from || t('period.fromStart'), to: run.date_to || t('period.today') })}</dd>
    {#if run.result}
      <dt>{t('hist.photos')}</dt>
      <dd>
        {t('create.used', { n: fmt(run.result.kept ?? 0) })}{#if run.result.skipped}
          · {t('create.skipped', { n: fmt(run.result.skipped) })}{/if}{#if run.result.excluded}
          · {t('hist.excluded', { n: fmt(run.result.excluded) })}{/if}
      </dd>
      {#if run.result.video_secs}
        <dt>{t('hist.length')}</dt>
        <dd>{formatDuration(run.result.video_secs)}</dd>
      {/if}
    {/if}
    {#if run.processing?.output}
      <dt>{t('hist.size')}</dt>
      <dd>{frameSize(run.processing.output).join(' × ')} px</dd>
    {/if}
    <dt>{t('hist.filters')}</dt>
    <dd>{filters.length ? filters.join(', ') : '–'}</dd>
    {#if run.video}
      <dt>{t('hist.music')}</dt>
      <dd>{(run.video.music_tracks ?? []).length ? t('music.nTracks', { n: run.video.music_tracks.length }) : t('music.none')}</dd>
    {/if}
  </dl>
{/if}

<style>
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 4px 12px;
    margin: 8px 0 0;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>
