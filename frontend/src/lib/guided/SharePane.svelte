<script>
  // Share panel (right of the result): new link with validity, active links
  // of this video with copy, change validity and revoke.
  import { t } from './i18n.svelte.js';
  import { shares, sharesOf, createShare, updateShare, setShareDownload, revokeShare, setPublicUrl, DURATIONS, formatDate, copyText } from './shares.svelte.js';

  let { folderName, defaultTitle = '' } = $props();

  let list = $derived(sharesOf(folderName));
  let title = $state('');
  let days = $state(7);
  let download = $state(false);
  let photos = $state(false);
  let busy = $state(false);
  let error = $state('');
  let copied = $state('');
  let urlText = $state('');
  // Per link: chosen new validity
  let change = $state({});

  $effect(() => {
    title = defaultTitle;
  });
  $effect(() => {
    urlText = shares.public_url;
  });

  const durLabel = (d) => (d === null ? t('share.forever') : d === 1 ? t('share.day') : t('share.days', { n: d }));
  const untilText = (s) => (s.expires_at ? t('share.until', { date: formatDate(s.expires_at) }) : t('share.noExpiry'));

  async function run(fn) {
    busy = true;
    error = '';
    try {
      await fn();
    } catch (e) {
      error = e.message;
    }
    busy = false;
  }
  const create = () => run(async () => {
    const s = await createShare(folderName, title, days, download, photos);
    if (s.url) copy(s);
  });
  const extend = (s) => run(() => updateShare(s.token, s.title, change[s.token] === undefined ? 7 : change[s.token]));
  const toggleDownload = (s, change) => run(() => setShareDownload(s.token, change));
  const revoke = (s) => {
    if (confirm(t('share.revokeConfirm'))) run(() => revokeShare(s.token));
  };
  const saveUrl = () => run(() => setPublicUrl(urlText));
  async function copy(s) {
    if (await copyText(s.url)) {
      copied = s.token;
      setTimeout(() => copied === s.token && (copied = ''), 2000);
    }
  }
</script>

<p class="muted" style="margin-top:0">{t('share.intro')}</p>

{#if shares.loaded && !shares.public_url}
  <div class="notice">
    <p>{t('share.noUrl')}</p>
    <label for="g-share-url">{t('settings.shareUrl')}</label>
    <div class="row">
      <input id="g-share-url" type="text" placeholder="https://timelapse-share.example.ch" bind:value={urlText} />
      <button type="button" class="btn" disabled={busy || !urlText.trim()} onclick={saveUrl}>{t('share.saveUrl')}</button>
    </div>
  </div>
{/if}

<h3 class="paneSection">{t('share.new')}</h3>
<label for="g-share-title">{t('share.titleLabel')}</label>
<input id="g-share-title" type="text" maxlength="200" bind:value={title} />
<p class="muted small">{t('share.titleHint')}</p>
<label for="g-share-days">{t('share.validity')}</label>
<select id="g-share-days" bind:value={days}>
  {#each DURATIONS as d (d)}<option value={d}>{durLabel(d)}</option>{/each}
</select>
<label class="check">
  <input type="checkbox" bind:checked={download} /> {t('share.download')}
</label>
<p class="muted small">{t('share.downloadHint')}</p>
<label class="check">
  <input type="checkbox" bind:checked={photos} /> {t('share.photos')}
</label>
<p class="muted small">{t('share.photosHint')}</p>
<div class="btnRow mt">
  <button type="button" class="btn primary" disabled={busy} onclick={create}>{t('share.create')}</button>
</div>

{#if error}<p class="err">{error}</p>{/if}

<h3 class="paneSection">{t('share.active')}</h3>
{#if !list.length}
  <p class="muted">{t('share.none')}</p>
{:else}
  {#each list as s (s.token)}
    <div class="link">
      <strong>{s.title || t('share.untitled')}</strong>
      <span class="muted small">{t('share.created', { date: formatDate(s.created_at) })} · {untilText(s)}</span>
      {#if s.url}
        <input type="text" readonly value={s.url} onfocus={(e) => e.currentTarget.select()} aria-label={t('share.link')} />
        <div class="btnRow">
          <button type="button" class="btn primary" onclick={() => copy(s)}>{copied === s.token ? t('share.copied') : t('share.copy')}</button>
          <a class="btn" href={s.url} target="_blank" rel="noopener">{t('share.open')}</a>
        </div>
      {:else}
        <p class="muted small">{t('share.noUrlLink')}</p>
      {/if}
      <label class="check">
        <input type="checkbox" checked={!!s.download} disabled={busy} onchange={(e) => toggleDownload(s, { download: e.currentTarget.checked })} />
        {t('share.download')}
      </label>
      <label class="check">
        <input type="checkbox" checked={!!s.photos} disabled={busy} onchange={(e) => toggleDownload(s, { photos: e.currentTarget.checked })} />
        {t('share.photos')}
      </label>
      <div class="row">
        <select aria-label={t('share.validity')} value={change[s.token] === undefined ? 7 : change[s.token]} onchange={(e) => (change[s.token] = DURATIONS[e.currentTarget.selectedIndex])}>
          {#each DURATIONS as d (d)}<option value={d}>{durLabel(d)}</option>{/each}
        </select>
        <button type="button" class="btn" disabled={busy} onclick={() => extend(s)}>{t('share.setValidity')}</button>
        <button type="button" class="btn danger" disabled={busy} onclick={() => revoke(s)}>{t('share.revoke')}</button>
      </div>
    </div>
  {/each}
{/if}

<p class="muted small mt">{t('share.privacy')}</p>

<style>
  .notice {
    border: 1px solid var(--border, #4446);
    border-radius: 8px;
    padding: 10px 12px;
    margin: 8px 0 4px;
  }
  .notice p {
    margin: 0 0 6px;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .row input,
  .row select {
    flex: 1 1 140px;
    min-width: 0;
  }
  .link {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 0;
    border-top: 1px solid var(--border, #4446);
  }
  .link:first-of-type {
    border-top: 0;
  }
  .link input {
    width: 100%;
    box-sizing: border-box;
  }
  .err {
    color: var(--err);
  }
</style>
