<script>
  import { t } from './i18n.svelte.js';
  import { FILTERS } from './config.js';
  import PreviewPanel from './PreviewPanel.svelte';

  let { config = $bindable(), open2 = null, onopen, selection } = $props();
  let p = $derived(config.processing);
</script>

<p class="muted" style="margin-bottom:10px">
  {t('quality.intro')}
</p>
<div class="settingRows">
  {#each FILTERS as f (f.key)}
    {@const on = open2 === 'f:' + f.key}
    <div class="settingRow" class:on>
      <input type="checkbox" bind:checked={config.processing[f.key].enabled} title={t('quality.toggle')} aria-label={t('quality.on', { title: f.title })} />
      <div class="settingText"><strong>{f.title}</strong>{p[f.key].enabled ? f.summary(p) : t('common.off')}</div>
      <button type="button" class="settingOpen" class:on title={t('common.openRight')} aria-label={t('quality.adjust', { title: f.title })} onclick={() => onopen('f:' + f.key)}>&gt;</button>
    </div>
  {/each}
  <div class="settingRow" class:on={open2 === 'adv'}>
    <div class="settingText">
      <strong>{t('adv.title')}</strong>
      {t('quality.advSummary', { preview: p.use_preview, workers: p.max_workers, debug: p.output.keep_intermediates })}
    </div>
    <button type="button" class="settingOpen" class:on={open2 === 'adv'} title={t('common.openRight')} aria-label={t('quality.advLabel')} onclick={() => onopen('adv')}>&gt;</button>
  </div>
</div>
<p class="muted mt">{t('common.autosave')}</p>
<PreviewPanel {config} {selection} />
