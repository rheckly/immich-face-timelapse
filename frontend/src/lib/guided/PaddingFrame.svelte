<script>
  import { t } from './i18n.svelte.js';
  // The PaddingIndicator of the classic view for any aspect ratio: the frame (outer)
  // and the smallest share of real photo pixels (inner). The border between them is
  // the padding, filled with duplicated edge pixels.
  let { maxPaddingPercent = 30, width = 1, height = 1 } = $props();

  const LONG = 100;
  const MARGIN = 8;
  let fw = $derived(width >= height ? LONG : (LONG * width) / height);
  let fh = $derived(height >= width ? LONG : (LONG * height) / width);
  // P % of the area may be padding: a uniform border leaves sqrt(1 - P) per side
  let side = $derived(Math.sqrt(Math.max(0, 1 - maxPaddingPercent / 100)));
  let iw = $derived(fw * side);
  let ih = $derived(fh * side);
</script>

<div class="wrap">
  <svg
    viewBox="0 0 {fw + 2 * MARGIN} {fh + 2 * MARGIN}"
    style="width:{fw + 2 * MARGIN}px;height:{fh + 2 * MARGIN}px"
    role="img"
    aria-label={t('filter.paddingLabel')}
  >
    <rect x={MARGIN} y={MARGIN} width={fw} height={fh} fill="#2a2a2a" stroke="#666" stroke-width="1.5" rx="2" />
    <rect x={MARGIN} y={MARGIN} width={fw} height={fh} fill="rgba(239, 68, 68, 0.30)" rx="2" />
    {#if iw > 2 && ih > 2}
      <rect x={MARGIN + (fw - iw) / 2} y={MARGIN + (fh - ih) / 2} width={iw} height={ih} fill="#3a3a3a" />
      <rect
        x={MARGIN + (fw - iw) / 2}
        y={MARGIN + (fh - ih) / 2}
        width={iw}
        height={ih}
        fill="none"
        stroke="rgba(255,255,255,0.35)"
        stroke-width="1"
        stroke-dasharray="3 2"
      />
    {/if}
  </svg>
  <div class="legend">
    <div><span class="swatch real"></span>{t('filter.realPixels')}</div>
    <div><span class="swatch padding"></span>{t('filter.paddingPixels')}</div>
  </div>
</div>

<style>
  .wrap {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  svg {
    flex-shrink: 0;
  }
  .legend {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .legend div {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .swatch {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 1px solid #666;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .swatch.real {
    background: #3a3a3a;
  }
  .swatch.padding {
    background: color-mix(in srgb, rgb(239, 68, 68) 30%, #2a2a2a);
  }
</style>
