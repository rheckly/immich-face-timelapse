<script>
  import { t } from './i18n.svelte.js';
  // Step 5: shape and size of the frames, how close the face is, and the date.
  import FrameIndicator from './FrameIndicator.svelte';
  import { ASPECTS, POSITIONS, frameSize, datePixels } from './config.js';

  let { config = $bindable(), dateOpen = false, ondate } = $props();
  let p = $derived(config.processing);
  let dims = $derived(frameSize(p.output));
  let ts = $derived(p.timestamp);
  let parts = $derived([ts.day && t('date.day'), ts.month && t('date.month'), ts.year && t('date.year')].filter(Boolean));
  let current = $derived(p.output.aspect_ratio ?? '1:1');
</script>

<h3 class="paneSection first">{t('format.aspect')}</h3>
<div class="aspects" role="radiogroup" aria-label={t('format.aspect')}>
  {#each Object.entries(ASPECTS) as [value, label] (value)}
    {@const [a, b] = value.split(':').map(Number)}
    <button
      type="button"
      role="radio"
      aria-checked={current === value}
      class:on={current === value}
      onclick={() => (config.processing.output.aspect_ratio = value)}
    >
      <span class="shape" style="width:{(a >= b ? 1 : a / b) * 26}px;height:{(b >= a ? 1 : b / a) * 26}px"></span>
      <strong>{value}</strong>
      <small>{label}</small>
    </button>
  {/each}
</div>

<label for="g-size">{t('format.size')}</label>
<div class="valueRow">
  <input id="g-size" type="number" min="64" max="4096" bind:value={config.processing.output.size} />
  <span class="val wide">{dims[0]} × {dims[1]} px</span>
</div>

<h3 class="paneSection">{t('format.face')}</h3>
<label for="g-eyes" style="margin-top:0">{t('format.eyes')}</label>
<div class="valueRow">
  <input id="g-eyes" type="range" min="0.05" max="0.4" step="0.01" bind:value={config.processing.alignment.eye_distance} />
  <span class="val">{(Number(p.alignment.eye_distance) * 100).toFixed(0)} %</span>
</div>
<p class="muted">{t('format.eyesHint')}</p>
<div class="indicator">
  <FrameIndicator eyeDistance={Number(p.alignment.eye_distance)} width={dims[0]} height={dims[1]} />
</div>

<div class="paneSection sectionHead">
  <h3>{t('date.title')}</h3>
  <button type="button" class="settingOpen" class:on={dateOpen} title={t('common.openRight')} aria-label={t('date.settings')} onclick={ondate}>&gt;</button>
</div>
<label class="check" style="margin-top:6px">
  <input type="checkbox" bind:checked={config.processing.timestamp.enabled} /> {t('date.show')}
</label>
{#if ts.enabled}
  <p class="muted">
    {POSITIONS[ts.position] ?? ts.position} · {parts.length ? parts.join(', ') : t('date.nothing')} · {t('date.pxHigh', { px: datePixels(ts, dims[1]) })}
  </p>
{/if}
