<script>
  // Level 3: the photos of the album opened in the image picker; a click
  // makes one the image of the intro or outro card.
  import { API } from '../constants.js';
  import PhotoGrid from './PhotoGrid.svelte';

  let { config = $bindable(), which = 'intro', album } = $props();

  let photos = $state([]);
  let loading = $state(false);
  let error = $state('');
  let seq = 0;

  $effect(() => {
    const id = album?.id;
    if (id) load(id);
  });
  async function load(id) {
    const n = ++seq;
    photos = [];
    loading = true;
    error = '';
    try {
      const res = await fetch(`${API.albumAssets}/${encodeURIComponent(id)}`);
      if (!res.ok) throw new Error((await res.text()) || `HTTP ${res.status}`);
      const list = await res.json();
      if (n === seq) photos = list;
    } catch (e) {
      if (n === seq) error = e.message;
    }
    if (n === seq) loading = false;
  }

  let card = $derived(config.video[which]);
</script>

<PhotoGrid
  {photos}
  {loading}
  {error}
  selected={card.asset_id}
  onpick={(id) => (config.video[which] = { ...card, asset_id: id, upload: null })}
/>
