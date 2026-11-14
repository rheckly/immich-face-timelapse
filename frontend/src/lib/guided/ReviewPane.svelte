<script>
  // Pick the prepared photos that go into the video: a click takes a photo out
  // or puts it back. Taken out photos move to the folder's `excluded/`.
  import { t, fmt } from './i18n.svelte.js';
  import { onDestroy } from 'svelte';
  import { API } from '../constants.js';

  // onsaving gets each save's promise, so the video waits for the last one
  let { folderName, onchange, onsaving } = $props();

  const PAGE = 240;
  let all = $state([]); // all photo names, in time order
  let out = $state(new Set()); // names taken out
  let saved = $state(new Set()); // what the server has
  let loading = $state(true);
  let error = $state('');
  let saving = $state(false);
  let shown = $state(PAGE);

  const base = (name) => `${API.output}/${encodeURIComponent(name)}/selection`;
  $effect(() => {
    const name = folderName;
    loading = true;
    error = '';
    shown = PAGE;
    fetch(base(name))
      .then(async (res) => {
        if (!res.ok) throw new Error(await res.text());
        const sel = await res.json();
        if (name !== folderName) return;
        all = [...sel.images, ...sel.excluded].sort();
        out = new Set(sel.excluded);
        saved = new Set(sel.excluded);
        onchange?.(sel);
      })
      .catch((e) => (error = e.message || t('review.errLoad')))
      .finally(() => (loading = false));
  });

  const src = (f) =>
    `/output/${encodeURIComponent(folderName)}/${saved.has(f) ? 'excluded' : 'images'}/${encodeURIComponent(f)}`;
  const dateText = (f) => (/^\d{4}-\d{2}-\d{2}/.test(f) ? new Date(f.slice(0, 10)).toLocaleDateString() : '');

  // Changes are saved shortly after the last click
  let timer = null;
  function setOut(next) {
    out = next;
    clearTimeout(timer);
    if (all.length - next.size > 0) timer = setTimeout(flush, 500);
  }
  function flush() {
    clearTimeout(timer);
    timer = null;
    const p = save();
    onsaving?.(p);
    return p;
  }
  onDestroy(() => timer && flush());
  function toggle(f) {
    const next = new Set(out);
    next.has(f) ? next.delete(f) : next.add(f);
    setOut(next);
  }
  let dirty = $derived(out.size !== saved.size || [...out].some((f) => !saved.has(f)));
  let used = $derived(all.length - out.size);

  async function save() {
    saving = true;
    error = '';
    try {
      const res = await fetch(base(folderName), {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ excluded: [...out] }),
      });
      if (!res.ok) throw new Error(await res.text());
      const sel = await res.json();
      saved = new Set(sel.excluded);
      onchange?.(sel);
    } catch (e) {
      error = e.message || t('review.errSave');
    } finally {
      saving = false;
    }
  }
</script>

{#if loading}
  <p class="muted">{t('common.loading')}</p>
{:else}
  <p class="muted">{t('review.hint')}</p>
  <p class="mt"><strong>{t('review.count', { n: fmt(used), all: fmt(all.length) })}</strong></p>
  <div class="btnRow mt">
    <button type="button" disabled={out.size === 0} onclick={() => setOut(new Set())}>{t('review.all')}</button>
    <span class="muted small">{saving || dirty ? t('review.saving') : t('review.saved')}</span>
  </div>
  {#if used === 0}<p class="err small">{t('review.noneLeft')}</p>{/if}
  {#if error}<p class="err mt">{error}</p>{/if}
  <div class="grid">
    {#each all.slice(0, shown) as f (f)}
      <button type="button" class="cell" class:off={out.has(f)} title="{dateText(f)} {f}" aria-pressed={!out.has(f)} onclick={() => toggle(f)}>
        <img loading="lazy" alt={f} src={src(f)} />
        {#if out.has(f)}<span class="mark">×</span>{/if}
      </button>
    {/each}
  </div>
  {#if all.length > shown}
    <button type="button" class="mt" onclick={() => (shown += PAGE)}>{t('pick.more', { n: fmt(all.length - shown) })}</button>
  {/if}
{/if}

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 6px;
    margin-top: 12px;
  }
  .cell {
    position: relative;
    padding: 0;
    aspect-ratio: 1;
    overflow: hidden;
    border: 2px solid transparent;
    border-radius: 6px;
    background: var(--line);
  }
  .cell img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .cell.off {
    border-color: var(--danger, #c0392b);
  }
  .cell.off img {
    opacity: 0.3;
    filter: grayscale(1);
  }
  .mark {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 2.4rem;
    font-weight: 700;
    color: var(--danger, #c0392b);
  }
</style>
