<script>
  // Settings: global values that apply to every run. "Reset to defaults"
  // (step panels) leaves them alone.
  import { t } from './i18n.svelte.js';
  import { shares, loadShares, setPublicUrl } from './shares.svelte.js';
  import { onMount } from 'svelte';

  let { config = $bindable() } = $props();

  // 0 = keep all versions; the slider covers 1 to 10
  let keepAll = $derived(!(Number(config.processing.output.keep_versions) > 0));
  let lastCount = $state(Math.min(10, Math.max(1, Number(config.processing.output.keep_versions) || 3)));
  function setKeepAll(on) {
    config.processing.output.keep_versions = on ? 0 : lastCount;
  }
  function setCount(n) {
    lastCount = n;
    config.processing.output.keep_versions = n;
  }

  // Public address for shared links (stored with the shares, not in the config)
  let shareUrl = $state('');
  let shareMsg = $state('');
  onMount(async () => {
    await loadShares();
    shareUrl = shares.public_url;
  });
  async function saveShareUrl() {
    if (shareUrl.trim() === shares.public_url) return;
    try {
      await setPublicUrl(shareUrl);
      shareUrl = shares.public_url;
      shareMsg = t('settings.shareSaved');
    } catch (e) {
      shareMsg = e.message;
    }
  }
</script>

<h3 class="paneSection" style="margin-top:0">{t('settings.versions')}</h3>
<p class="muted">{t('settings.versionsHint')}</p>
<label class="check mt">
  <input type="checkbox" checked={keepAll} onchange={(e) => setKeepAll(e.currentTarget.checked)} />
  {t('settings.keepAll')}
</label>
<label for="g-keep-versions" class:dim={keepAll}>{t('settings.count')}</label>
<div class="valueRow" class:dim={keepAll}>
  <input
    id="g-keep-versions"
    type="range"
    min="1"
    max="10"
    step="1"
    disabled={keepAll}
    value={keepAll ? lastCount : Math.min(10, Number(config.processing.output.keep_versions))}
    oninput={(e) => setCount(Number(e.currentTarget.value))}
  />
  <span class="val">{keepAll ? t('settings.all') : config.processing.output.keep_versions}</span>
</div>

<h3 class="paneSection">{t('settings.workers')}</h3>
<p class="muted">{t('settings.workersHint')}</p>
<label for="g-workers">{t('settings.workersCount')}</label>
<div class="valueRow">
  <input id="g-workers" type="range" min="1" max="16" step="1" bind:value={config.processing.max_workers} />
  <span class="val">{config.processing.max_workers}</span>
</div>

<h3 class="paneSection">{t('settings.share')}</h3>
<p class="muted">{t('settings.shareHint', { port: shares.share_port })}</p>
<label for="g-share-url">{t('settings.shareUrl')}</label>
<input
  id="g-share-url"
  type="text"
  placeholder="https://timelapse-share.example.ch"
  bind:value={shareUrl}
  oninput={() => (shareMsg = '')}
  onchange={saveShareUrl}
  onkeydown={(e) => e.key === 'Enter' && saveShareUrl()}
/>
{#if shareMsg}<p class="muted small">{shareMsg}</p>{/if}

<p class="muted small mt">{t('settings.global')}</p>

<style>
  .dim {
    opacity: 0.5;
  }
</style>
