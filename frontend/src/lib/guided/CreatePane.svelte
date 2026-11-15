<script>
  // Step 7: start the job and follow its two phases, preparing the photos and
  // creating the video, each with its own progress bar.
  import { onDestroy, untrack } from 'svelte';
  import { FILTERS, frameSize, formatDuration } from './config.js';
  import { JOB_STATUS, API } from '../constants.js';
  import { t, has, fmt } from './i18n.svelte.js';

  let {
    person,
    albums = [],
    dateFrom = '',
    dateTo = '',
    config = $bindable(),
    assetCount = null,
    loadingCount = false,
    jobStatus,
    progress,
    busy = false,
    starting = false,
    onstart,
    onresult,
    openedResult = null, // folder of the timelapse opened from the history, its video already exists
    stale = false, // settings changed since that video
    reviewOpen = false,
    onreview,
    onfinish,
    finishing = false,
    pickSel = null,
  } = $props();

  let active = $derived(config ? FILTERS.filter((f) => config.processing[f.key].enabled).length : 0);

  // ---------- Phases ----------
  const S = JOB_STATUS;
  // The photos are ready and wait for the user's pick before the video is made
  let review = $derived(jobStatus === S.completed && !!progress.awaiting_review);
  let inVideo = $derived(jobStatus === S.compiling || (jobStatus === S.completed && !review));
  // Phase 1 counts, kept while the video is made (the job then reports frames)
  let prep = $state({ completed: 0, total: 0, stats: null });
  // Whether this run reached the video phase; then counts are frames, not photos,
  // also while cancelling.
  let reachedVideo = $state(false);
  $effect(() => {
    const status = jobStatus;
    const { completed, total, skip_stats: st } = progress;
    untrack(() => {
      if (status === S.running || (status === S.cancelling && !reachedVideo)) {
        reachedVideo = false;
        prep = { completed, total, stats: st ?? prep.stats };
      } else if (status === S.compiling || status === S.completed) {
        reachedVideo = true;
        if (st) {
          const done = (st.kept ?? 0) + (st.total ?? 0);
          prep = { completed: Math.max(prep.completed, done), total: Math.max(prep.total, done), stats: st };
        }
      } else if (status === S.idle) {
        reachedVideo = false;
        prep = { completed: 0, total: 0, stats: null };
      }
    });
  });

  const phase1State = $derived(
    inVideo || reachedVideo || (prep.total > 0 && prep.completed >= prep.total)
      ? 'done'
      : jobStatus === S.running || jobStatus === S.cancelling
        ? 'active'
        : 'stopped'
  );
  const phase2State = $derived(
    review
      ? 'waiting'
      : jobStatus === S.completed
      ? 'done'
      : jobStatus === S.compiling
        ? 'active'
        : reachedVideo
          ? 'stopped'
          : 'waiting'
  );

  // Skipped photos by reason, largest first
  let skipped = $derived.by(() => {
    const st = prep.stats;
    if (!st) return null;
    const reasons = Object.entries(st)
      .filter(([k, n]) => k !== 'total' && k !== 'kept' && n > 0)
      .sort((a, b) => b[1] - a[1])
      .map(([k, n]) => `${has('skip.' + k) ? t('skip.' + k) : k.replaceAll('_', ' ')} ${fmt(n)}`);
    return { kept: st.kept ?? 0, total: st.total ?? 0, reasons };
  });

  // Elapsed time of the video phase, ticking each second
  let now = $state(Date.now());
  let timer = null;
  $effect(() => {
    if (jobStatus === S.compiling && !timer) {
      now = Date.now();
      timer = setInterval(() => (now = Date.now()), 1000);
    } else if (jobStatus !== S.compiling && timer) {
      clearInterval(timer);
      timer = null;
    }
  });
  onDestroy(() => timer && clearInterval(timer));
  let elapsed = $derived(progress.video_started_at ? Math.max(0, (now - progress.video_started_at) / 1000) : 0);
  let videoFrames = $derived(jobStatus === S.compiling && progress.completed > 0 && progress.total > 0);

  let cancelling = $state(false);
  async function cancel() {
    cancelling = true;
    try {
      await fetch(API.cancel, { method: 'POST' });
    } catch (e) {
      console.error('Failed to cancel:', e);
    } finally {
      cancelling = false;
    }
  }
  let canCancel = $derived(jobStatus === S.running || jobStatus === S.compiling);

  // How many prepared photos go into the video: from the pick panel, else
  // asked once from the server (a pick may exist from before a reload)
  let loadedSel = $state(null);
  $effect(() => {
    const folder = review ? progress.output_folder : null;
    if (!folder) return;
    untrack(() => {
      if (loadedSel?.folder === folder) return;
      fetch(`${API.output}/${encodeURIComponent(folder)}/selection`)
        .then((res) => (res.ok ? res.json() : null))
        .then((sel) => {
          if (sel) loadedSel = { folder, used: sel.images.length, all: sel.images.length + sel.excluded.length };
        })
        .catch(() => {});
    });
  });
  let pick = $derived.by(() => {
    const folder = progress.output_folder;
    if (pickSel?.folder === folder) return pickSel;
    if (loadedSel?.folder === folder) return loadedSel;
    const n = prep.stats?.kept ?? prep.completed;
    return { folder, used: n, all: n };
  });
</script>

{#if (busy || progress?.awaiting_review) && progress?.person_id && progress.person_id !== person?.id}
  <!-- The run on the server belongs to another person (e.g. started in another browser) -->
  <p><strong>{t('job.otherPerson', { name: progress.person_name || progress.person_id })}</strong></p>
{/if}
<p class="bigCount">
  {#if loadingCount}
    {t('count.counting')}
  {:else if assetCount}
    {t('count.photosOf', { n: fmt(assetCount.assets_with_faces), name: person?.name || t('common.unnamed') })}
  {:else}
    {person?.name || t('common.unnamed')}
  {/if}
</p>
<p class="muted mt">
  {albums.length === 0 ? t('albums.all') : albums.map((a) => a.name).join(', ')}
  · {t('period.fromTo', { from: dateFrom || t('period.fromStart'), to: dateTo || t('period.today') })}
  {#if config}
    · {t('create.nFilters', { n: active })} · {frameSize(config.processing.output).join(' × ')} px
    · {(config.video.music_tracks ?? []).length > 0 ? t('create.withMusic') : t('music.none')}
  {/if}
</p>
<p class="muted mt">{t('create.from')}</p>

{#if config}
  <h3 class="paneSection">{t('create.folder')}</h3>
  <label class="check" class:dim={!config.video.enabled}>
    <input
      type="checkbox"
      disabled={busy || !config.video.enabled}
      checked={!config.video.enabled || config.processing.output.keep_frames !== false}
      onchange={(e) => (config.processing.output.keep_frames = e.currentTarget.checked)}
    />
    {t('create.keepFrames')}
  </label>
  <p class="muted small">{t(config.video.enabled ? 'create.keepFramesHint' : 'create.keepFramesNoVideo')}</p>
  <label class="check">
    <input type="checkbox" disabled={busy} bind:checked={config.processing.output.keep_originals} />
    {t('create.keepOriginals')}
  </label>
  <p class="muted small">{t(config.processing.use_preview ? 'create.keepOriginalsPreview' : 'create.keepOriginalsHint')}</p>
{/if}

{#if openedResult}
  <p class="muted mt">{stale ? t('create.openedStale') : t('create.opened')}</p>
  <p class="mt"><button type="button" class:primary={!stale} onclick={onresult}>{t('create.viewExisting')} &gt;</button></p>
{/if}
{#if !busy}
  <p class="mt">
    <button type="button" class="primary" onclick={onstart} disabled={starting || !person}>
      {starting ? t('create.starting') : jobStatus === S.idle ? t('create.start') : t('create.again')}
    </button>
  </p>
{/if}

{#if jobStatus !== S.idle}
  <ol class="phases mt">
    <li class={phase1State}>
      <div class="phaseHead">
        <span class="phaseMark">{phase1State === 'done' ? '✓' : '1'}</span>
        <strong>{t('create.phase1')}</strong>
      </div>
      {#if prep.total > 0}
        <progress max={prep.total} value={prep.completed}></progress>
        <p class="muted">{t('create.prepared', { x: fmt(prep.completed), y: fmt(prep.total) })}</p>
      {:else if phase1State === 'active'}
        <progress></progress>
        <p class="muted">{progress.message || t('create.starting')}</p>
      {/if}
      {#if skipped && (skipped.kept || skipped.total)}
        <p class="muted small">
          {t('create.used', { n: fmt(skipped.kept) })} · {t('create.skipped', { n: fmt(skipped.total) })}{skipped.reasons.length
            ? ` (${skipped.reasons.join(', ')})`
            : ''}
        </p>
      {/if}
    </li>
    <li class={review ? 'active pickBox' : inVideo || reachedVideo ? 'done' : 'waiting'} class:open={review && reviewOpen}>
      {#if review}
        <button type="button" class="pickBtn" aria-expanded={reviewOpen} onclick={onreview}>
          <span class="phaseHead">
            <span class="phaseMark">2</span>
            <strong>{t('create.phasePick')}</strong>
            <span class="chev">&gt;</span>
          </span>
          <span class="muted pickText">
            {pick.used < pick.all
              ? t('create.pickCount', { n: fmt(pick.used), all: fmt(pick.all) })
              : t('create.pickHint', { n: fmt(pick.all) })}
          </span>
        </button>
      {:else}
        <div class="phaseHead">
          <span class="phaseMark">{inVideo || reachedVideo ? '✓' : '2'}</span>
          <strong>{t('create.phasePick')}</strong>
        </div>
      {/if}
    </li>
    <li class={phase2State}>
      <div class="phaseHead">
        <span class="phaseMark">{phase2State === 'done' ? '✓' : '3'}</span>
        <strong>{t('create.phase2')}</strong>
      </div>
      {#if review}
        <p class="muted">{t('create.makeHint')}</p>
        <p class="mt">
          <button type="button" class="primary" disabled={finishing || pick.used === 0} onclick={onfinish}>
            {finishing ? t('create.starting') : t('create.makeVideo')}
          </button>
        </p>
      {:else if phase2State === 'active'}
        {#if videoFrames}
          <progress max={progress.total} value={progress.completed}></progress>
          <p class="muted">
            {t('create.frames', { x: fmt(progress.completed), y: fmt(progress.total) })} · {formatDuration(elapsed)}
          </p>
        {:else}
          <progress></progress>
          <p class="muted">{t('create.elapsed', { time: formatDuration(elapsed) })}</p>
        {/if}
      {:else if phase2State === 'done'}
        <progress max="1" value="1"></progress>
        <p class="muted">
          {progress.video_millis != null
            ? t('create.doneSum', { took: formatDuration(progress.video_millis / 1000), len: formatDuration(progress.video_secs) })
            : t('create.done')}
        </p>
      {/if}
    </li>
  </ol>

  {#if jobStatus === S.cancelling}
    <p class="muted mt">{t('create.cancelling')}</p>
  {:else if jobStatus === S.cancelled}
    <p class="err mt">{t('create.cancelled')}</p>
  {:else if jobStatus === S.error}
    <p class="err mt">{progress.message || t('create.failed')}</p>
  {/if}

  {#if canCancel}
    <p class="mt"><button type="button" class="danger" disabled={cancelling} onclick={cancel}>{t('common.cancel')}</button></p>
  {/if}
  {#if busy}
    <p class="muted mt">{t('create.serverHint')}</p>
  {/if}
  {#if jobStatus === S.completed && !review}
    <p class="mt"><button type="button" class="primary" onclick={onresult}>{t('create.view')} &gt;</button></p>
  {/if}
{/if}

<style>
  .dim {
    opacity: 0.5;
  }
  .phases li.pickBox {
    padding: 0;
  }
  .phases li.pickBox.open {
    background: var(--accent-soft, var(--card));
  }
  .pickBtn {
    display: block;
    width: 100%;
    padding: 10px 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .pickBtn:hover {
    background: var(--hover, rgba(127, 127, 127, 0.08));
  }
  .pickText {
    display: block;
    margin-top: 4px;
  }
  .chev {
    margin-left: auto;
    color: var(--muted, inherit);
  }
</style>
