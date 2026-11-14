<script>
  // Grid of Immich photos to click one; shows PAGE at a time.
  import { t, fmt } from './i18n.svelte.js';
  import { API } from '../constants.js';

  let { photos = [], loading = false, error = '', selected = null, onpick } = $props();

  const PAGE = 120;
  let shown = $state(PAGE);
  $effect(() => {
    photos;
    shown = PAGE;
  });
  const thumb = (id) => `${API.assets}/${encodeURIComponent(id)}/thumbnail`;
  const dateText = (d) => {
    const x = d ? new Date(d) : null;
    return x && !isNaN(x) ? x.toLocaleDateString() : '';
  };
</script>

{#if loading}
  <p class="muted mt">{t('common.loading')}</p>
{:else if error}
  <p class="err mt">{error}</p>
{:else if photos.length === 0}
  <p class="muted mt">{t('pick.noPhotos')}</p>
{:else}
  <p class="muted mt">{t('pick.nPhotos', { n: fmt(photos.length) })}</p>
  <div class="grid">
    {#each photos.slice(0, shown) as p (p.id)}
      <button type="button" class="cell" class:on={selected === p.id} title="{p.file_name ?? ''} {dateText(p.date)}" onclick={() => onpick?.(p.id)}>
        <img loading="lazy" alt={p.file_name ?? ''} src={thumb(p.id)} />
      </button>
    {/each}
  </div>
  {#if photos.length > shown}
    <button type="button" class="mt" onclick={() => (shown += PAGE)}>{t('pick.more', { n: fmt(photos.length - shown) })}</button>
  {/if}
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    gap: 6px;
    margin-top: 8px;
  }
  .cell {
    padding: 0;
    aspect-ratio: 1;
    overflow: hidden;
    border: 2px solid transparent;
    border-radius: 6px;
    background: var(--line);
  }
  .cell.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent);
  }
  .cell img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
</style>
