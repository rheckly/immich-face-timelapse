<script>
  import { t } from './i18n.svelte.js';
  import { FILTERS, frameSize } from './config.js';
  import PaddingFrame from './PaddingFrame.svelte';
  import BrightnessIndicator from '../components/visual/BrightnessIndicator.svelte';
  import EyeIndicator from '../components/visual/EyeIndicator.svelte';
  import PreviewPanel from './PreviewPanel.svelte';

  let { config = $bindable(), key, selection } = $props();
  let f = $derived(FILTERS.find((x) => x.key === key));
  let p = $derived(config.processing);
</script>

<label class="check" style="margin-top:0">
  <input type="checkbox" bind:checked={config.processing[key].enabled} /> {t('filter.active')}
</label>
<p class="muted mt">{f.hint}</p>

<fieldset disabled={!p[key].enabled}>
  {#if key === 'face_resolution'}
    <label for="g-min-size">{t('filter.minFace')}</label>
    <div class="valueRow">
      <input id="g-min-size" type="range" min="20" max="200" step="10" bind:value={config.processing.face_resolution.min_size} />
      <span class="val">{p.face_resolution.min_size} px</span>
    </div>
  {:else if key === 'crop'}
    <label for="g-padding">{t('filter.maxPadding')}</label>
    <div class="valueRow">
      <input id="g-padding" type="range" min="5" max="80" step="5" bind:value={config.processing.crop.max_padding_percent} />
      <span class="val">{p.crop.max_padding_percent} %</span>
    </div>
    {@const [fw, fh] = frameSize(p.output)}
    <div class="indicator"><PaddingFrame maxPaddingPercent={p.crop.max_padding_percent} width={fw} height={fh} /></div>
  {:else if key === 'brightness'}
    <p class="lbl">{t('filter.brightness')}</p>
    <div class="indicator">
      <BrightnessIndicator
        bind:minBrightness={config.processing.brightness.min_brightness}
        bind:maxBrightness={config.processing.brightness.max_brightness}
      />
    </div>
  {:else if key === 'blur'}
    <label for="g-sharp">{t('filter.minSharp')}</label>
    <div class="valueRow">
      <input id="g-sharp" type="range" min="10" max="150" step="1" bind:value={config.processing.blur.min_sharpness} />
      <span class="val">{Number(p.blur.min_sharpness).toFixed(0)}</span>
    </div>
  {:else if key === 'head_pose'}
    <label for="g-yaw">{t('filter.maxYaw')}</label>
    <div class="valueRow">
      <input id="g-yaw" type="range" min="5" max="90" step="5" bind:value={config.processing.head_pose.max_yaw} />
      <span class="val">{Number(p.head_pose.max_yaw).toFixed(0)}°</span>
    </div>
    <label for="g-pitch">{t('filter.maxPitch')}</label>
    <div class="valueRow">
      <input id="g-pitch" type="range" min="5" max="90" step="5" bind:value={config.processing.head_pose.max_pitch} />
      <span class="val">{Number(p.head_pose.max_pitch).toFixed(0)}°</span>
    </div>
  {:else if key === 'eye_filter'}
    <label for="g-ear">{t('filter.minEar')}</label>
    <div class="valueRow">
      <input id="g-ear" type="range" min="0.1" max="0.4" step="0.02" bind:value={config.processing.eye_filter.min_ear} />
      <span class="val">{Number(p.eye_filter.min_ear).toFixed(2)}</span>
    </div>
    <div class="indicator"><EyeIndicator ear={Number(p.eye_filter.min_ear)} /></div>
  {/if}
</fieldset>

<PreviewPanel {config} {selection} focus={key} />
