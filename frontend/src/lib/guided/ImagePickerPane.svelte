<script>
  // Background image of an intro or outro card: from an Immich album (its
  // photos open in the panel to the right), a smart search over the whole
  // library, or an uploaded file.
  import { onMount } from 'svelte';
  import { t, fmt } from './i18n.svelte.js';
  import { API } from '../constants.js';
  import PhotoGrid from './PhotoGrid.svelte';

  let { config = $bindable(), which = 'intro', albums = [], openAlbumId = null, onalbum } = $props();

  const ACCEPT = '.jpg,.jpeg,.png,.webp,image/jpeg,image/png,image/webp';

  let card = $derived(config.video[which]);
  let tab = $state('albums'); // 'albums' | 'search' | 'upload'

  function pickAsset(id) {
    config.video[which] = { ...card, asset_id: id, upload: null };
  }
  function pickUpload(name) {
    config.video[which] = { ...card, asset_id: null, upload: name };
  }
  function pickNone() {
    config.video[which] = { ...card, asset_id: null, upload: null };
  }
  const thumb = (id) => `${API.assets}/${encodeURIComponent(id)}/thumbnail`;

  async function getJson(url) {
    const res = await fetch(url);
    if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
    return res.json();
  }

  // ---------- Albums ----------
  let albumQuery = $state('');
  const plain = (s) => (s || '').toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '');
  let albumRows = $derived.by(() => {
    const words = plain(albumQuery).split(/\s+/).filter(Boolean);
    return albums
      .filter((a) => (a.asset_count ?? 0) > 0 && words.every((w) => plain(a.name).includes(w)))
      .sort((a, b) => a.name.localeCompare(b.name));
  });

  // ---------- Smart search ----------
  let searchText = $state('');
  let searched = $state('');
  let photos = $state([]);
  let photoState = $state({ loading: false, error: '' });
  let seq = 0;
  async function runSearch(e) {
    e?.preventDefault();
    const q = searchText.trim();
    if (!q) return;
    searched = q;
    const n = ++seq;
    photos = [];
    photoState = { loading: true, error: '' };
    try {
      const list = await getJson(`${API.assetSearch}?q=${encodeURIComponent(q)}`);
      if (n === seq) photos = list;
      if (n === seq) photoState = { loading: false, error: '' };
    } catch (err) {
      if (n === seq) photoState = { loading: false, error: err.message };
    }
  }
  function clearSearch() {
    seq++; // drop a running search
    searched = '';
    photos = [];
    photoState = { loading: false, error: '' };
  }
  // An emptied search field also removes the results
  $effect(() => {
    if (!searchText.trim()) clearSearch();
  });

  function setTab(next) {
    if (tab === next) return;
    tab = next;
    if (next === 'upload') loadUploads();
  }

  // ---------- Upload ----------
  let uploads = $state([]);
  let uploadState = $state({ busy: '', error: '' });
  let fileInput = $state();
  let dragOver = $state(false);

  async function loadUploads() {
    try {
      uploads = await getJson(API.titleImages);
    } catch (e) {
      uploadState = { busy: '', error: e.message };
    }
  }
  async function upload(files) {
    const file = [...(files ?? [])][0];
    if (!file) return;
    uploadState = { busy: file.name, error: '' };
    try {
      const res = await fetch(`${API.titleImages}/${encodeURIComponent(file.name)}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/octet-stream' },
        body: file,
      });
      if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
      const { name } = await res.json();
      pickUpload(name);
      uploadState = { busy: '', error: '' };
      await loadUploads();
    } catch (e) {
      uploadState = { busy: '', error: `${file.name}: ${e.message}` };
    }
    if (fileInput) fileInput.value = '';
  }
  function onDrop(e) {
    e.preventDefault();
    dragOver = false;
    upload(e.dataTransfer?.files);
  }

  onMount(() => {
    // An uploaded image is chosen: start on the upload tab
    if (card.upload && !card.asset_id) {
      tab = 'upload';
      loadUploads();
    }
  });
</script>

<p class="muted">{t(which === 'intro' ? 'pick.forIntro' : 'pick.forOutro')}</p>

<div class="current mt">
  {#if card.asset_id}
    <img src={thumb(card.asset_id)} alt="" />
    <span>{t('pick.chosenImmich')}</span>
  {:else if card.upload}
    <img src="{API.titleImages}/{encodeURIComponent(card.upload)}" alt="" />
    <span>{card.upload}</span>
  {:else}
    <span class="swatch" style:background={card.background}></span>
    <span>{t('titles.noImage')}</span>
  {/if}
  {#if card.asset_id || card.upload}
    <button type="button" onclick={pickNone}>{t('pick.none')}</button>
  {/if}
</div>

<div class="seg mt" role="tablist">
  <button type="button" role="tab" aria-selected={tab === 'albums'} class:on={tab === 'albums'} onclick={() => setTab('albums')}>{t('pick.albums')}</button>
  <button type="button" role="tab" aria-selected={tab === 'search'} class:on={tab === 'search'} onclick={() => setTab('search')}>{t('pick.search')}</button>
  <button type="button" role="tab" aria-selected={tab === 'upload'} class:on={tab === 'upload'} onclick={() => setTab('upload')}>{t('pick.upload')}</button>
</div>

{#if tab === 'albums'}
  <input class="mt" type="search" placeholder={t('pick.findAlbum')} aria-label={t('pick.findAlbum')} bind:value={albumQuery} />
  <ul class="albums">
    {#each albumRows as a (a.id)}
      <li>
        <button type="button" class:on={openAlbumId === a.id} aria-pressed={openAlbumId === a.id} onclick={() => onalbum?.(a)}>
          {#if a.thumbnail_asset_id}
            <img loading="lazy" alt="" src="/api/albums/{encodeURIComponent(a.thumbnail_asset_id)}/thumbnail" onerror={(e) => (e.currentTarget.style.visibility = 'hidden')} />
          {:else}
            <span class="noThumb"></span>
          {/if}
          <span class="name">{a.name}</span>
          <span class="muted">{fmt(a.asset_count ?? 0)}</span>
          <span class="arrow">&gt;</span>
        </button>
      </li>
    {/each}
  </ul>
  {#if albumRows.length === 0}<p class="muted mt">{t('pick.noAlbums')}</p>{/if}
{:else if tab === 'search'}
  <form class="searchRow mt" onsubmit={runSearch}>
    <input type="search" placeholder={t('pick.searchHint')} aria-label={t('pick.search')} bind:value={searchText} />
    <button type="submit" disabled={!searchText.trim() || photoState.loading}>{t('pick.find')}</button>
  </form>
  <p class="muted small">{t('pick.searchInfo')}</p>
{:else}
  <div
    class="drop mt"
    class:dragOver
    role="group"
    aria-label={t('pick.upload')}
    ondragover={(e) => {
      e.preventDefault();
      dragOver = true;
    }}
    ondragleave={() => (dragOver = false)}
    ondrop={onDrop}
  >
    <p class="muted">{t('pick.dropHint')}</p>
    <button type="button" onclick={() => fileInput?.click()} disabled={!!uploadState.busy}>
      {uploadState.busy ? t('music.uploading') : t('pick.chooseFile')}
    </button>
    <input bind:this={fileInput} type="file" accept={ACCEPT} hidden onchange={(e) => upload(e.currentTarget.files)} />
  </div>
  {#if uploadState.error}<p class="err">{uploadState.error}</p>{/if}
  {#if uploads.length > 0}
    <p class="lbl">{t('pick.earlier')}</p>
    <div class="grid">
      {#each uploads as name (name)}
        <button type="button" class="cell" class:on={card.upload === name && !card.asset_id} title={name} onclick={() => pickUpload(name)}>
          <img loading="lazy" alt={name} src="{API.titleImages}/{encodeURIComponent(name)}" />
        </button>
      {/each}
    </div>
  {/if}
{/if}

{#if tab === 'search' && searched}
  <PhotoGrid {photos} loading={photoState.loading} error={photoState.error} selected={card.asset_id} onpick={pickAsset} />
{/if}

<style>
  .current {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .current img,
  .current .swatch {
    width: 56px;
    height: 56px;
    object-fit: cover;
    border-radius: 6px;
    border: 1px solid var(--line);
    flex: none;
  }
  .current span {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .albums {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }
  .albums button {
    width: 100%;
    display: flex;
    gap: 10px;
    align-items: center;
    text-align: left;
    padding: 4px 8px;
  }
  .albums img,
  .albums .noThumb {
    width: 40px;
    height: 40px;
    object-fit: cover;
    border-radius: 4px;
    flex: none;
    background: var(--line);
  }
  .albums button.on {
    border-color: var(--accent);
  }
  .albums .arrow {
    font-weight: 600;
  }
  .albums .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .searchRow {
    display: flex;
    gap: 8px;
  }
  .searchRow input {
    flex: 1;
    min-width: 0;
  }
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
  .drop {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
    padding: 12px;
    border: 1px dashed var(--line);
    border-radius: 8px;
  }
  .drop.dragOver {
    border-color: var(--accent);
  }
</style>
