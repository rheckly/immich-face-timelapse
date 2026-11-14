<script>
  import { t } from './i18n.svelte.js';
  // One step of the process in the left column. Optionally collapsible,
  // optionally with a ">" button that opens its panel on the right.
  let {
    title,
    toggle = false,
    closed = false,
    locked = false,
    paneOpen = false,
    summary = '',
    ontoggle,
    onopen,
    children,
  } = $props();
</script>

<section class="box" class:closed={toggle && closed} class:locked>
  <div class="boxHead">
    {#if toggle}
      <h2 class="boxTitle">
        <button type="button" class="boxToggle" aria-expanded={!closed} onclick={ontoggle}>
          <span>{title}</span><span class="boxArrow">▾</span>
        </button>
      </h2>
    {:else}
      <h2 class="boxTitle fixed">{title}</h2>
    {/if}
    {#if onopen}
      <button
        type="button"
        class="paneBtn"
        class:on={paneOpen}
        title={t('common.openRight')}
        aria-label={t('box.open', { title })}
        aria-pressed={paneOpen}
        disabled={locked}
        onclick={onopen}>&gt;</button
      >
    {/if}
    {#if toggle}<span class="boxSum muted">{summary}</span>{/if}
  </div>
  <div class="boxBody">{@render children?.()}</div>
</section>
