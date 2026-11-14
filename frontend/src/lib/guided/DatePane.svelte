<script>
  import { t } from './i18n.svelte.js';
  // Details of the date in the frames: where, how large and which parts.
  import { POSITIONS, frameSize, datePixels, DATE_FORMATS, formatDate, namedDateFormat } from './config.js';
  import { API } from '../constants.js';
  import { onMount } from 'svelte';

  let { config = $bindable() } = $props();
  let p = $derived(config.processing);
  let ts = $derived(p.timestamp);
  let dims = $derived(frameSize(p.output));
  let px = $derived(datePixels(ts, dims[1]));
  // Without a font size the date keeps today's fixed 7 px: show that as share of the height
  let shown = $derived(ts.font_size == null ? Math.round((7 / dims[1]) * 1000) / 10 : Number(ts.font_size));
  let sample = $derived(formatDate(ts, 2024, 5, 17) || '–');
  let named = $derived(namedDateFormat(ts.format ?? 'iso'));

  // Fonts on the server (same list as for the titles); empty = built-in pixel font
  let fonts = $state([]);
  onMount(async () => {
    try {
      const res = await fetch(API.fonts);
      if (res.ok) fonts = await res.json();
    } catch {
      // keep only the pixel font
    }
  });
  let fontName = $derived(fonts.find((f) => f.file === ts.font)?.name ?? '');
  let color = $derived('#' + String(ts.color ?? 'ffffff').replace('#', ''));
  const ORDER = ['top_left', 'top_center', 'top_right', 'bottom_left', 'bottom_center', 'bottom_right'];

  function setSize(e) {
    config.processing.timestamp.font_size = Number(e.currentTarget.value);
  }

  // Example frame: margin and text grow with the font scale, as on the server
  let style = $derived.by(() => {
    const scale = px / 7;
    const [v, hz] = (ts.position ?? 'bottom_left').split('_');
    const mh = ((8 * scale) / dims[1]) * 100;
    const mw = ((8 * scale) / dims[0]) * 100;
    const y = v === 'top' ? `top:${mh}%` : `bottom:${mh}%`;
    const x = hz === 'left' ? `left:${mw}%` : hz === 'right' ? `right:${mw}%` : 'left:50%;transform:translateX(-50%)';
    // The 5x7 font is 7 px high: a CSS font of ~1.3 times that has the same cap height
    const family = fontName ? `font-family:'${fontName.replace(/'/g, '')}', sans-serif;` : named && !ts.font ? 'font-family:sans-serif;font-weight:700;' : '';
    const shadow = ts.shadow === false ? 'text-shadow:none;' : '';
    return `${y};${x};font-size:${((px * 1.3) / dims[1]) * 100}cqh;color:${color};${family}${shadow}`;
  });
</script>

<label class="check" style="margin-top:0">
  <input type="checkbox" bind:checked={config.processing.timestamp.enabled} /> {t('date.show')}
</label>

<fieldset disabled={!ts.enabled}>
  <p class="lbl">{t('date.position')}</p>
  <div class="posGrid" role="radiogroup" aria-label={t('date.position')}>
    {#each ORDER as value (value)}
      <button
        type="button"
        role="radio"
        aria-checked={ts.position === value}
        class:on={ts.position === value}
        onclick={() => (config.processing.timestamp.position = value)}
      >
        {POSITIONS[value]}
      </button>
    {/each}
  </div>

  <label for="g-dformat">{t('date.format')}</label>
  <select id="g-dformat" bind:value={config.processing.timestamp.format}>
    {#each DATE_FORMATS as f (f)}
      <option value={f}>{formatDate({ ...ts, format: f, day: true, month: true, year: true }, 2024, 5, 17)}</option>
    {/each}
  </select>
  {#if named}
    <label for="g-dlang">{t('date.lang')}</label>
    <select id="g-dlang" bind:value={config.processing.timestamp.lang}>
      <option value="de">Deutsch</option>
      <option value="en">English</option>
    </select>
  {/if}

  <label for="g-dfont">{t('date.fontFace')}</label>
  <select id="g-dfont" bind:value={config.processing.timestamp.font}>
    <option value="">{t('date.pixelFont')}</option>
    {#if ts.font && !fonts.some((f) => f.file === ts.font)}<option value={ts.font}>{ts.font}</option>{/if}
    {#each fonts as f (f.file)}
      <option value={f.file}>{f.name}</option>
    {/each}
  </select>
  {#if named && !ts.font}<p class="muted">{t('date.pixelHint')}</p>{/if}

  <div class="colorRow">
    <label for="g-dcolor">{t('date.color')}</label>
    <input
      id="g-dcolor"
      type="color"
      value={color}
      oninput={(e) => (config.processing.timestamp.color = e.currentTarget.value.slice(1))}
    />
    <label class="check">
      <input type="checkbox" checked={ts.shadow !== false} onchange={(e) => (config.processing.timestamp.shadow = e.currentTarget.checked)} />
      {t('date.shadow')}
    </label>
  </div>

  <label for="g-font">{t('date.font')}</label>
  <div class="valueRow">
    <input id="g-font" type="range" min="0.5" max="10" step="0.5" value={shown} oninput={setSize} />
    <span class="val wide">{ts.font_size == null ? t('date.original') : `${shown} %`}</span>
  </div>
  <p class="muted">
    {t('date.fontHint', { px, w: dims[0], h: dims[1] })}
    {#if ts.font_size != null}
      <button type="button" class="linkBtn" onclick={() => (config.processing.timestamp.font_size = null)}>{t('date.backOriginal')}</button>
    {/if}
  </p>

  <p class="lbl">{t('date.parts')}</p>
  <div class="toggles">
    <button type="button" class:on={ts.day} aria-pressed={ts.day} onclick={() => (config.processing.timestamp.day = !ts.day)}>{t('date.Day')}</button>
    <button type="button" class:on={ts.month} aria-pressed={ts.month} onclick={() => (config.processing.timestamp.month = !ts.month)}>{t('date.Month')}</button>
    <button type="button" class:on={ts.year} aria-pressed={ts.year} onclick={() => (config.processing.timestamp.year = !ts.year)}>{t('date.Year')}</button>
  </div>

  <p class="lbl mt">{t('date.example')}</p>
  <div class="dateFrame" style="aspect-ratio:{dims[0]} / {dims[1]};width:{Math.min(1, dims[0] / dims[1]) * 260}px">
    <span class="dateText" {style}>{sample}</span>
  </div>
  {#if ts.font}<p class="muted small">{t('date.exampleHint')}</p>{/if}
</fieldset>

<style>
  .colorRow {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 10px;
  }
  .colorRow label {
    margin: 0;
  }
  .colorRow input[type='color'] {
    width: 44px;
    height: 28px;
    padding: 0 2px;
  }
</style>
