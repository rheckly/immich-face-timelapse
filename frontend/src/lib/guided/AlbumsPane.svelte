<script>
  let { albums = [], loading = false, error = null, selected = [], personName = '', disabled = false, onchange } = $props();

  let query = $state('');
  let ids = $derived(new Set(selected.map((a) => a.id)));
  let shown = $derived(
    query.trim() ? albums.filter((a) => a.name.toLowerCase().includes(query.trim().toLowerCase())) : albums
  );

  function toggle(album) {
    onchange(ids.has(album.id) ? selected.filter((a) => a.id !== album.id) : [...selected, album]);
  }
</script>

<fieldset {disabled}>
  <label class="check" style="margin-top:0">
    <input type="checkbox" checked={selected.length === 0} onchange={() => onchange([])} disabled={selected.length === 0} />
    Use all photos
  </label>
  <p class="muted">Or choose albums below. Every photo of {personName || 'the person'} that is in one of them counts.</p>
  <input class="mt" type="search" placeholder="Search albums" bind:value={query} aria-label="Search albums" />
  {#if loading}
    <p class="muted mt">Loading albums...</p>
  {:else if error}
    <p class="err mt">{error}</p>
  {:else}
    <div class="tableWrap">
      <table>
        <thead><tr><th>Name</th><th class="num">Photos</th></tr></thead>
        <tbody>
          {#each shown as a (a.id)}
            <tr>
              <td>
                <label class="check">
                  <input type="checkbox" checked={ids.has(a.id)} onchange={() => toggle(a)} />
                  {a.name}
                </label>
              </td>
              <td class="num">{a.asset_count ?? ''}</td>
            </tr>
          {:else}
            <tr><td colspan="2" class="muted">{albums.length === 0 ? 'No albums in Immich.' : 'No album matches the search.'}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</fieldset>
