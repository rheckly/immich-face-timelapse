<script>
  // Steps 7 and 8: the animated intro or outro card before or after the film.
  // "Find image" opens the picker in the panel to the right (ImagePickerPane).
  import { onMount } from 'svelte';
  import { t } from './i18n.svelte.js';
  import { CARD_DEFAULTS, MOTIONS, formatDuration } from './config.js';
  import { API } from '../constants.js';
  import TitleCardSummary from './TitleCardSummary.svelte';

  let { config = $bindable(), which = 'intro', pickOpen = false, onpick } = $props();

  // Older settings have no cards yet
  for (const w of ['intro', 'outro']) {
    if (!config.video[w]) config.video[w] = { ...CARD_DEFAULTS };
  }

  let card = $derived(config.video[which]);
  let hasImage = $derived(!!(card.asset_id || card.upload));
  let imageSrc = $derived(
    card.asset_id
      ? `${API.assets}/${encodeURIComponent(card.asset_id)}/thumbnail`
      : card.upload
        ? `${API.titleImages}/${encodeURIComponent(card.upload)}`
        : null
  );

  // Fonts on the server; the chosen one stays in the list even if it is missing
  let fonts = $state([]);
  let fontError = $state('');
  onMount(async () => {
    try {
      const res = await fetch(API.fonts);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      fonts = await res.json();
    } catch (e) {
      fontError = e.message;
    }
  });
  let fontList = $derived(
    card.font && !fonts.some((f) => f.file === card.font) ? [{ file: card.font, name: card.font }, ...fonts] : fonts
  );

  // Asset id or Immich link, loaded with "Load"
  let idText = $state('');
  let idError = $state('');
  $effect(() => {
    idText = card.asset_id ?? '';
    idError = '';
  });
  function loadId(e) {
    e.preventDefault();
    const m = idText.match(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i);
    if (!m) {
      idError = t('pick.idInvalid');
      return;
    }
    idError = '';
    config.video[which] = { ...card, asset_id: m[0].toLowerCase(), upload: null };
  }

  let extraSecs = $derived(
    ['intro', 'outro'].reduce((s, w) => s + (config.video[w]?.enabled ? Number(config.video[w].duration_secs) || 0 : 0), 0)
  );
</script>

<p class="muted">{t(which === 'intro' ? 'titles.leadIntro' : 'titles.leadOutro')}</p>

<label class="check mt">
  <input type="checkbox" bind:checked={config.video[which].enabled} />
  {t(which === 'intro' ? 'titles.useIntro' : 'titles.useOutro')}
</label>

{#if card.enabled}
  <div class="mt"><TitleCardSummary {card} {which} /></div>
  <p class="muted small">{t('titles.previewHint')}</p>

  <div class="paneSection sectionHead">
    <h3>{t('titles.image')}</h3>
    <button type="button" class="settingOpen" class:on={pickOpen} title={t('common.openRight')} aria-label={t('titles.findImage')} onclick={() => onpick?.()}>&gt;</button>
  </div>
  <div class="imageRow">
    {#if imageSrc}
      <img class="thumb" src={imageSrc} alt="" onerror={(e) => (e.currentTarget.style.visibility = 'hidden')} />
    {:else}
      <span class="thumb swatch" style:background={card.background}></span>
    {/if}
    <form class="idForm" onsubmit={loadId}>
      <input type="text" placeholder={t('pick.idHint')} aria-label={t('titles.assetId')} bind:value={idText} />
      <button type="submit" disabled={!idText.trim() || idText.trim() === card.asset_id}>{t('titles.load')}</button>
    </form>
    <span class="dash">–</span>
    <button type="button" class:on={pickOpen} onclick={() => onpick?.()}>{t('titles.findImage')}</button>
  </div>
  {#if idError}<p class="err">{idError}</p>{/if}
  <p class="muted small imageInfo">
    {#if card.asset_id}{t('titles.fromImmich')}{:else if card.upload}{card.upload}{:else}{t('titles.noImage')}{/if}
    {#if hasImage}
      <button type="button" onclick={() => (config.video[which] = { ...card, asset_id: null, upload: null })}>{t('titles.removeImage')}</button>
    {/if}
  </p>
  {#if hasImage}
    <label for="g-t-dim">{t('titles.dim')}</label>
    <div class="valueRow">
      <input id="g-t-dim" type="range" min="0" max="0.9" step="0.05" bind:value={config.video[which].dim} />
      <span class="val">{Math.round(Number(card.dim) * 100)} %</span>
    </div>
    <label for="g-t-motion">{t('titles.motion')}</label>
    <select
      id="g-t-motion"
      value={card.zoom ? card.motion || 'auto' : 'still'}
      onchange={(e) => {
        const v = e.currentTarget.value;
        config.video[which] = { ...card, zoom: v !== 'still', motion: v === 'still' ? card.motion || 'auto' : v };
      }}
    >
      {#each ['still', ...MOTIONS] as m (m)}<option value={m}>{t(`titles.motion.${m}`)}</option>{/each}
    </select>
  {:else}
    <label for="g-t-bg">{t('titles.background')}</label>
    <input id="g-t-bg" type="color" bind:value={config.video[which].background} />
  {/if}

  <h3 class="paneSection">{t('titles.textSection')}</h3>
  {#snippet textField(key, colorKey, label, id)}
    <label for={id}>{label}</label>
    <div class="textRow">
      {#if key === 'text'}
        <textarea {id} rows="4" maxlength="3000" bind:value={config.video[which].text}></textarea>
      {:else}
        <input {id} type="text" maxlength="300" bind:value={config.video[which][key]} />
      {/if}
      <div class="colorCol">
        <input
          type="color"
          aria-label={t('titles.colorOf', { what: label })}
          title={t('titles.colorOf', { what: label })}
          value={card[colorKey] || card.color}
          oninput={(e) => (config.video[which][colorKey] = e.currentTarget.value)}
        />
        {#if colorKey !== 'color' && card[colorKey]}
          <button type="button" class="linkBtn" title={t('titles.likeTitleHint')} onclick={() => (config.video[which][colorKey] = '')}>{t('titles.likeTitle')}</button>
        {/if}
      </div>
    </div>
  {/snippet}
  {@render textField('title', 'color', t('titles.title'), 'g-t-title')}
  {@render textField('subtitle', 'subtitle_color', t('titles.subtitle'), 'g-t-sub')}
  {@render textField('text', 'text_color', t('titles.text'), 'g-t-text')}
  <label class="check"><input type="checkbox" bind:checked={config.video[which].dates} /> {t('titles.dates')}</label>
  {#if card.dates}<p class="muted small">{t('titles.datesHint')}</p>{/if}

  <label for="g-t-font">{t('titles.font')}</label>
  <select id="g-t-font" bind:value={config.video[which].font}>
    {#each fontList as f (f.file)}
      <option value={f.file}>{f.name}</option>
    {/each}
  </select>
  {#if fontError}<p class="err">{t('titles.fontsFailed')} {fontError}</p>{/if}
  <label for="g-t-size">{t('titles.size')}</label>
  <div class="valueRow">
    <input id="g-t-size" type="range" min="2" max="25" step="0.5" bind:value={config.video[which].size} />
    <span class="val">{Number(card.size)} %</span>
  </div>

  <h3 class="paneSection">{t('titles.lengthSection')}</h3>
  <label for="g-t-dur">{t('titles.duration')}</label>
  <div class="valueRow">
    <input id="g-t-dur" type="range" min="1" max="30" step="0.5" bind:value={config.video[which].duration_secs} />
    <span class="val">{Number(card.duration_secs)} s</span>
  </div>
  <p class="muted">{t('titles.animation')}</p>
{/if}

{#if extraSecs > 0 && (config.video.fit_to_duration || config.video.fit_to_music)}
  <p class="muted mt">{t('titles.countsToLength', { len: formatDuration(extraSecs) })}</p>
{/if}

<style>
  .imageRow {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .thumb {
    width: 48px;
    height: 48px;
    object-fit: cover;
    border-radius: 6px;
    border: 1px solid var(--line);
    flex: none;
    display: block;
  }
  .idForm {
    display: flex;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }
  .idForm input {
    flex: 1;
    min-width: 0;
  }
  .dash {
    flex: none;
  }
  .imageRow > button {
    flex: none;
  }
  .imageInfo {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    overflow-wrap: anywhere;
    margin-top: 4px;
  }
  .textRow {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .textRow > input,
  .textRow > textarea {
    flex: 1;
    min-width: 0;
  }
  .colorCol {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .linkBtn {
    padding: 0 4px;
    font-size: 0.8em;
    border: none;
    background: none;
    color: var(--accent);
    cursor: pointer;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    padding: 6px 8px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--bg);
    color: inherit;
    font: inherit;
    resize: vertical;
  }
  input[type='color'] {
    width: 48px;
    height: 32px;
    padding: 2px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--bg);
  }
  input[type='text'] {
    width: 100%;
    box-sizing: border-box;
  }
</style>
