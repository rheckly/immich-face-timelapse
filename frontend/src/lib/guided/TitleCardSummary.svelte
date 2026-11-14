<script>
  // Preview still at half width with the details of the card next to it,
  // used in the box on the left and at the top of the card's panel.
  import { t } from './i18n.svelte.js';
  import { previews } from './titlePreview.svelte.js';

  let { card, which } = $props();
  let p = $derived(previews[which]);
</script>

{#if !card?.enabled}
  <p class="muted">{t(which === 'intro' ? 'titles.offIntro' : 'titles.offOutro')}</p>
{:else}
  <div class="cardSum">
    <div class="shot">
      {#if p.url}
        <img src={p.url} alt={t('titles.preview')} class:stale={p.state === 'loading'} />
      {:else}
        <span class="blank" style:background={card.background}>{p.state === 'loading' ? t('titles.previewLoading') : ''}</span>
      {/if}
    </div>
    <div class="details">
      <strong>{t('titles.boxSecs', { secs: Number(card.duration_secs) })}</strong>
      {#if card.title}<span class="line">{card.title}</span>{/if}
      {#if card.subtitle}<span class="line muted">{card.subtitle}</span>{/if}
      <span class="muted small">
        {#if card.asset_id}{t('titles.fromImmich')}{:else if card.upload}{card.upload}{:else}{t('titles.noImage')}{/if}
      </span>
      {#if p.state && p.state !== 'loading'}<span class="err small">{p.state}</span>{/if}
    </div>
  </div>
{/if}

<style>
  .cardSum {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .shot {
    flex: 0 0 50%;
    min-width: 0;
  }
  .shot img,
  .blank {
    display: block;
    width: 100%;
    border-radius: 6px;
    border: 1px solid var(--line);
    background: #000;
    transition: opacity 0.15s;
  }
  .blank {
    aspect-ratio: 16 / 9;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.8em;
    color: #fff;
  }
  img.stale {
    opacity: 0.6;
  }
  .details {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .line {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
