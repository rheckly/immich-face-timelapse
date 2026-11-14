<script>
  import { t } from './i18n.svelte.js';
  let { config = $bindable(), onreset } = $props();
</script>

<label class="check" style="margin-top:0">
  <input type="checkbox" bind:checked={config.processing.use_preview} /> {t('adv.preview')}
</label>
<p class="muted">{t('adv.previewHint')}</p>

<label class="check mt">
  <input type="checkbox" bind:checked={config.processing.output.keep_intermediates} /> {t('adv.debug')}
</label>
<p class="muted">{t('adv.debugHint')}</p>

<h3 class="paneSection">{t('adv.reset')}</h3>
<p class="muted" style="margin-bottom:8px">{t('adv.resetHint')}</p>
<button type="button" onclick={onreset}>{t('adv.resetBtn')}</button>
