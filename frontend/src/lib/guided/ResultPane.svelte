<script>
  import { t } from './i18n.svelte.js';
  import { formatSize } from '../utils.js';
  import { shares, sharesOf, formatDate, copyText } from './shares.svelte.js';

  // stamp changes whenever the result may have changed, so the video is not served from the cache
  let { folderName, folder = null, stamp = 0, photosOpen = false, onphotos, shareOpen = false, onshare, stale = false, onrecreate } = $props();

  let links = $derived(sharesOf(folderName));
  let copied = $state(false);
  async function copyLink() {
    if (links[0]?.url && (await copyText(links[0].url))) {
      copied = true;
      setTimeout(() => (copied = false), 2000);
    }
  }
  const until = (s) => (s.expires_at ? t('share.until', { date: formatDate(s.expires_at) }) : t('share.noExpiry'));

  let videoUrl = $derived(
    `/output/${encodeURIComponent(folderName)}/${encodeURIComponent(folderName)}.mp4?v=${stamp}`
  );
  let failed = $state(false);
  $effect(() => {
    videoUrl;
    failed = false;
  });
</script>

{#if stale}
  <p class="muted" style="margin:0 0 6px">{t('result.stale')}</p>
  <div class="btnRow" style="margin-bottom:10px"><button type="button" onclick={onrecreate}>{t('result.recreate')}</button></div>
{/if}
{#if failed}
  <div class="videoMissing">
    <p>{t('result.none')}</p>
    <p class="muted">{t('result.noneHint')}</p>
  </div>
{:else}
  {#key videoUrl}
    <video controls autoplay muted loop playsinline src={videoUrl} onerror={() => (failed = true)}>
      <track kind="captions" />
    </video>
  {/key}
{/if}
<p class="muted mt">
  {folderName}
  {#if folder}· {t('common.nPhotos', { n: folder.image_count })} · {formatSize(folder.size_bytes)}{/if}
</p>

{#if !failed}
  <h3 class="paneSection">{t('result.download')}</h3>
  <div class="btnRow">
    <a class="btn primary" href={videoUrl} download="{folderName}.mp4">{t('result.downloadVideo')}</a>
    <a class="btn" href={videoUrl} target="_blank" rel="noopener">{t('result.newTab')}</a>
  </div>

  <div class="paneSection sectionHead">
    <h3>{t('share.title')}</h3>
    <button type="button" class="settingOpen" class:on={shareOpen} title={t('common.openRight')} aria-label={t('share.title')} onclick={onshare}>&gt;</button>
  </div>
  {#if !links.length}
    <p class="muted">{t('share.notShared')}</p>
  {:else}
    <p class="shared">
      <strong>{links.length === 1 ? t('share.sharedOne') : t('share.sharedMany', { n: links.length })}</strong>
      {#if links.length === 1}· {until(links[0])}{/if}
    </p>
    {#if links.length > 1}
      <ul class="muted small">
        {#each links as s (s.token)}<li>{s.title || t('share.untitled')} · {until(s)}</li>{/each}
      </ul>
    {/if}
  {/if}
  <div class="btnRow">
    <button type="button" class="btn" class:primary={!links.length} onclick={onshare}>{links.length ? t('share.manage') : t('share.button')}</button>
    {#if links.length === 1 && links[0].url}
      <button type="button" class="btn primary" onclick={copyLink}>{copied ? t('share.copied') : t('share.copy')}</button>
    {/if}
  </div>
{/if}

<div class="paneSection sectionHead">
  <h3>{t('common.usedPhotos')}</h3>
  <button type="button" class="settingOpen" class:on={photosOpen} title={t('common.openRight')} aria-label={t('common.usedPhotos')} onclick={onphotos}>&gt;</button>
</div>
<p class="muted">{t('result.photosHint')}</p>

<style>
  .shared {
    margin: 0 0 6px;
  }
  ul {
    margin: 0 0 8px;
    padding-left: 18px;
  }
</style>
