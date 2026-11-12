<script>
  let { people = [], loading = false, error = null, selectedId = null, disabled = false, onselect } = $props();

  let query = $state('');
  let broken = $state(new Set()); // ids whose thumbnail failed to load

  let shown = $derived(
    query.trim()
      ? people.filter((p) => (p.name || '').toLowerCase().includes(query.trim().toLowerCase()))
      : people
  );

  function initial(p) {
    return (p.name || '?').trim().charAt(0).toUpperCase() || '?';
  }
</script>

<fieldset {disabled}>
  <input type="search" placeholder="Search by name" bind:value={query} aria-label="Search people" />
  {#if loading}
    <p class="muted mt">Loading people...</p>
  {:else if error}
    <p class="err mt">{error}</p>
  {:else if shown.length === 0}
    <p class="muted mt">{people.length === 0 ? 'No named people found in Immich.' : 'Nobody matches the search.'}</p>
  {:else}
    <div class="people">
      {#each shown as p (p.id)}
        <button type="button" class="personCard" aria-pressed={p.id === selectedId} onclick={() => onselect(p)}>
          <span class="avatar">
            {initial(p)}
            {#if !broken.has(p.id)}
              <img
                src="/api/people/{encodeURIComponent(p.id)}/thumbnail"
                alt=""
                loading="lazy"
                onerror={() => (broken = new Set(broken).add(p.id))}
              />
            {/if}
          </span>
          <span class="name">{p.name || 'Unnamed'}</span>
        </button>
      {/each}
    </div>
  {/if}
</fieldset>
