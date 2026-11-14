<script>
  import { t } from './i18n.svelte.js';
  // Step 6: how the frames become a video, and the music.
  import { CODECS, formatDuration } from './config.js';

  let { config = $bindable(), music = { tracks: [] }, musicOpen = false, onmusic } = $props();
  let v = $derived(config.video);
  let tracks = $derived(v.music_tracks ?? []);
  let totalSecs = $derived(
    tracks.reduce((sum, n) => sum + (music.tracks?.find((t) => t.name === n)?.duration_secs ?? 0), 0)
  );

  // Video length: fixed fps, a fixed length, or the length of the music.
  // A fixed length wins over the music in the backend, so only one is ever on.
  let mode = $derived(v.fit_to_duration ? 'duration' : v.fit_to_music ? 'music' : 'fps');
  function setMode(next) {
    config.video.fit_to_duration = next === 'duration';
    config.video.fit_to_music = next === 'music';
  }
</script>

<h3 class="paneSection first">{t('video.title')}</h3>
<p class="lblText" style="margin-top:0">{t('video.length')}</p>
<div class="modeList" role="radiogroup" aria-label={t('video.length')}>
  <label class="check">
    <input type="radio" name="g-vmode" checked={mode === 'fps'} onchange={() => setMode('fps')} />
    {t('video.modeFps')}
  </label>
  <label class="check">
    <input type="radio" name="g-vmode" checked={mode === 'duration'} onchange={() => setMode('duration')} />
    {t('video.modeDuration')}
  </label>
  <label class="check">
    <input type="radio" name="g-vmode" checked={mode === 'music'} onchange={() => setMode('music')} />
    {t('video.modeMusic')}
    {#if totalSecs > 0}<small class="muted">({formatDuration(totalSecs)})</small>{/if}
  </label>
</div>

{#if mode === 'fps'}
  <label for="g-fps">{t('video.fps')}</label>
  <div class="valueRow">
    <input id="g-fps" type="range" min="1" max="60" step="1" bind:value={config.video.framerate} />
    <span class="val">{v.framerate} fps</span>
  </div>
{:else}
  {#if mode === 'duration'}
    <label for="g-dur">{t('video.duration')}</label>
    <div class="valueRow">
      <input id="g-dur" type="number" min="1" max="3600" step="1" bind:value={config.video.target_duration_secs} />
      <span class="val">{formatDuration(Number(v.target_duration_secs))}</span>
    </div>
    <p class="muted">{t('video.durationHint')}</p>
  {:else}
    <p class="muted mt">{t('music.fitHint')}</p>
    {#if tracks.length === 0}<p class="err">{t('video.musicNone')}</p>{/if}
  {/if}
  <div class="row">
    <div>
      <label for="g-minfps">{t('music.minFps')}</label>
      <input id="g-minfps" type="number" min="1" max="60" step="0.5" bind:value={config.video.min_framerate} />
    </div>
    <div>
      <label for="g-maxfps">{t('music.maxFps')}</label>
      <input id="g-maxfps" type="number" min="1" max="120" step="0.5" bind:value={config.video.max_framerate} />
    </div>
  </div>
  {#if Number(v.min_framerate) > Number(v.max_framerate)}
    <p class="err mt">{t('music.minAboveMax')}</p>
  {/if}
{/if}

<label for="g-codec">{t('video.codec')}</label>
<select id="g-codec" bind:value={config.video.codec}>
  {#each Object.entries(CODECS) as [value, label] (value)}
    <option {value}>{label}</option>
  {/each}
</select>
<label for="g-crf">{t('video.crf')}</label>
<div class="valueRow">
  <input id="g-crf" type="range" min="15" max="40" step="1" bind:value={config.video.crf} />
  <span class="val">{v.crf}</span>
</div>

<div class="paneSection sectionHead">
  <h3>{t('music.title')}</h3>
  <button type="button" class="settingOpen" class:on={musicOpen} title={t('common.openRight')} aria-label={t('music.title')} onclick={onmusic}>&gt;</button>
</div>
<p class="muted">
  {#if tracks.length === 0}
    {t('music.none')}
  {:else}
    {tracks.join(', ')}{mode === 'music' ? ' · ' + t('video.fitsMusic') : ''}
  {/if}
</p>
{#if tracks.length > 0}
  {#if totalSecs > 0}<p class="muted">{t('music.length', { len: formatDuration(totalSecs) })}</p>{/if}
  <label for="g-fade">{t('music.fade')}</label>
  <div class="valueRow">
    <input id="g-fade" type="range" min="0" max="15" step="0.5" bind:value={config.video.music_fade_out_secs} />
    <span class="val">{Number(config.video.music_fade_out_secs).toFixed(1)} s</span>
  </div>
{/if}

<style>
  .lblText {
    margin: 0 0 4px;
    font-size: 13px;
    color: var(--muted);
  }
  .modeList {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
