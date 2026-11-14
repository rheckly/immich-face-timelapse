<script>
  import { t } from './i18n.svelte.js';
  import { formatSize, formatFolderVersion } from '../utils.js';
  import { showErrorAlert } from '../errorHandler.js';
  import { API } from '../constants.js';
  import { formatDuration } from './config.js';

  // running: the folder the server is working on right now (shown as "running")
  let { folders = [], current = null, running = null, disabled = false, onopen, ondeleted } = $props();

  // One group per person (the server sorts by person, newest version first);
  // the heading is the name from run.json, the folder name only for old runs.
  let groups = $derived.by(() => {
    const list = [];
    for (const f of folders) {
      const last = list[list.length - 1];
      if (last && last.key === f.person) last.items.push(f);
      else list.push({ key: f.person, items: [f] });
    }
    for (const g of list) g.title = g.items.find((f) => f.display_name)?.display_name ?? g.key ?? g.items[0].name;
    return list;
  });
  const when = (f) =>
    f.created
      ? new Date(f.created).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })
      : formatFolderVersion(f.version);
  // A run with run.json that never finished and is not running: cancelled, failed or interrupted
  const incomplete = (f) => f.has_run && !f.finished && f.name !== running;

  let deleting = $state(null);

  async function remove(name) {
    if (!confirm(t('history.confirmOne', { name }))) return;
    deleting = name;
    try {
      const res = await fetch(`${API.output}/${encodeURIComponent(name)}`, { method: 'DELETE' });
      if (!res.ok) throw res;
      ondeleted(name);
    } catch (e) {
      await showErrorAlert(t('history.errOne'), e);
    } finally {
      deleting = null;
    }
  }

  async function removeAll() {
    if (!confirm(t('history.confirmAll'))) return;
    deleting = '__all__';
    try {
      const res = await fetch(API.output, { method: 'DELETE' });
      if (!res.ok) throw res;
      ondeleted(null);
    } catch (e) {
      await showErrorAlert(t('history.errAll'), e);
    } finally {
      deleting = null;
    }
  }
</script>

{#if folders.length === 0}
  <p class="muted">{t('history.none')}</p>
{:else}
  {#each groups as g (g.key)}
    <h3 class="paneSection histGroup">{g.title} <span class="muted">({g.items.length})</span></h3>
    {#each g.items as f (f.name)}
      <div class="hist" class:on={current === f.name}>
        <button type="button" class="open" onclick={() => onopen(f.name)} title={t('history.open', { name: f.name })}>
          <strong>{when(f) || f.name}</strong>
          <span class="muted">
            {t('common.nPhotos', { n: f.image_count })} · {formatSize(f.size_bytes)}{#if f.video_secs}
              · {formatDuration(f.video_secs)}{/if}
            {#if f.name === running}
              <span class="badge run">{t('history.running')}</span>
            {:else if incomplete(f)}
              <span class="badge warn" title={t('history.incompleteHint')}>{t('history.incomplete')}</span>
            {:else if f.has_video}
              <span class="badge">{t('common.video')}</span>
            {/if}
          </span>
        </button>
        <button
          type="button"
          class="x danger"
          aria-label={t('history.deleteOne', { name: f.name })}
          title={t('common.delete')}
          disabled={disabled || deleting !== null || f.name === running}
          onclick={() => remove(f.name)}>{deleting === f.name ? '...' : '×'}</button
        >
      </div>
    {/each}
  {/each}
  <p class="mt">
    <button type="button" class="danger" disabled={disabled || deleting !== null} onclick={removeAll}>
      {deleting === '__all__' ? t('history.deleting') : t('history.deleteAll')}
    </button>
  </p>
{/if}
