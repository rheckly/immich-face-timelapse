<script>
  import { formatSize } from '../utils.js';
  import { showErrorAlert } from '../errorHandler.js';
  import { API } from '../constants.js';

  let { folders = [], current = null, disabled = false, onopen, ondeleted } = $props();

  let deleting = $state(null);

  async function remove(name) {
    if (!confirm(`Delete output folder "${name}"? This cannot be undone.`)) return;
    deleting = name;
    try {
      const res = await fetch(`${API.output}/${encodeURIComponent(name)}`, { method: 'DELETE' });
      if (!res.ok) throw res;
      ondeleted(name);
    } catch (e) {
      await showErrorAlert('Failed to delete folder', e);
    } finally {
      deleting = null;
    }
  }

  async function removeAll() {
    if (!confirm('Delete ALL output folders? This cannot be undone.')) return;
    deleting = '__all__';
    try {
      const res = await fetch(API.output, { method: 'DELETE' });
      if (!res.ok) throw res;
      ondeleted(null);
    } catch (e) {
      await showErrorAlert('Failed to delete folders', e);
    } finally {
      deleting = null;
    }
  }
</script>

{#if folders.length === 0}
  <p class="muted">No timelapses yet.</p>
{:else}
  {#each folders as f (f.name)}
    <div class="hist" class:on={current === f.name}>
      <button type="button" class="open" onclick={() => onopen(f.name)} title="Open result">
        <strong>{f.name}</strong>
        <span class="muted">
          {f.image_count} photos · {formatSize(f.size_bytes)}
          {#if f.has_video}<span class="badge">Video</span>{/if}
        </span>
      </button>
      <button
        type="button"
        class="x danger"
        aria-label="Delete {f.name}"
        title="Delete"
        disabled={disabled || deleting !== null}
        onclick={() => remove(f.name)}>{deleting === f.name ? '...' : '×'}</button
      >
    </div>
  {/each}
  <p class="mt">
    <button type="button" class="danger" disabled={disabled || deleting !== null} onclick={removeAll}>
      {deleting === '__all__' ? 'Deleting...' : 'Delete all'}
    </button>
  </p>
{/if}
