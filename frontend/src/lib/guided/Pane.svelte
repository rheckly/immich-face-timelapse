<script>
  import { t } from './i18n.svelte.js';
  // One panel column: title with close button, scrollable body, sticky row
  // with "previous / next step" and Done at the bottom (Mosaik pattern).
  let {
    title,
    level = 1, // 1 = pane, 2 = pane2, 3 = right
    offLeft = false,
    wide = false,
    nav = [], // [{ label, onclick }]
    onclose,
    children,
  } = $props();
</script>

<aside class="pane l{level}" class:offLeft class:galleryPane={wide} aria-label={title}>
  <div class="paneHead">
    <h2>{title}</h2>
    <button type="button" class="x" title={t('common.close')} aria-label={t('pane.close', { title })} onclick={onclose}>×</button>
  </div>
  <div class="panel">
    {@render children()}
    <div class="btnRow doneRow">
      {#each nav as n (n.label)}
        <button type="button" title={t('pane.goTo', { label: n.label })} onclick={n.onclick}>{n.label}</button>
      {/each}
      <button type="button" onclick={onclose}>{t('common.done')}</button>
    </div>
  </div>
</aside>
