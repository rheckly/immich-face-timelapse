<script>
  import { t } from './i18n.svelte.js';
  // The AlignmentIndicator of the classic view for any aspect ratio: the frame with
  // the silhouette, eyes at the same share of the width and height as on the server.
  import silhouetteUrl from '../assets/silhouette.png';

  let { eyeDistance = 0.3, width = 1, height = 1 } = $props();

  const LONG = 120;
  let w = $derived(width >= height ? LONG : (LONG * width) / height);
  let h = $derived(height >= width ? LONG : (LONG * height) / width);
  // Eye distance is a share of the frame width, ~12 % of the silhouette width
  let imgSize = $derived((eyeDistance * w) / 0.12);
  let imgX = $derived((w - imgSize) / 2);
  // Same offset to the eye line as the square indicator, eye line at a share of the height
  let eyeShare = $derived(0.5 - eyeDistance * 0.5);
  let imgY = $derived(eyeShare * h - (eyeShare * w - (w * 1.4 - imgSize) / 2));
</script>

<svg
  viewBox="0 0 {w} {h}"
  style="width:{(w / LONG) * 80}px;height:{(h / LONG) * 80}px"
  role="img"
  aria-label={t('format.frameLabel')}
>
  <rect x="0" y="0" width={w} height={h} fill="rgba(0, 0, 0, 0.8)" />
  <image href={silhouetteUrl} x={imgX} y={imgY} width={imgSize} height={imgSize} class="silhouette" />
</svg>

<style>
  svg {
    overflow: hidden;
  }
  .silhouette {
    filter: invert(1);
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }
</style>
