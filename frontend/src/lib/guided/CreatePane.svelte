<script>
  import ProgressDisplay from '../components/ProgressDisplay.svelte';
  import { FILTERS } from './config.js';
  import { JOB_STATUS } from '../constants.js';

  let {
    person,
    albums = [],
    dateFrom = '',
    dateTo = '',
    config,
    assetCount = null,
    loadingCount = false,
    jobStatus,
    progress,
    busy = false,
    starting = false,
    onstart,
    onresult,
  } = $props();

  let active = $derived(config ? FILTERS.filter((f) => config.processing[f.key].enabled).length : 0);
</script>

<p class="bigCount">
  {#if loadingCount}
    Counting photos...
  {:else if assetCount}
    {assetCount.assets_with_faces} photos of {person?.name || 'Unnamed'}
  {:else}
    {person?.name || 'Unnamed'}
  {/if}
</p>
<p class="muted mt">
  {albums.length === 0 ? 'All photos' : albums.map((a) => a.name).join(', ')}
  · {dateFrom || 'start'} to {dateTo || 'today'}
  {#if config}
    · {active} filters · {config.processing.output.size} px
    · {(config.video.music_tracks ?? []).length > 0 ? 'with music' : 'no music'}
  {/if}
</p>
<p class="muted mt">Person from step 1, photos from steps 2 and 3, settings from steps 4 and 5.</p>

{#if !busy}
  <p class="mt">
    <button type="button" class="primary" onclick={onstart} disabled={starting || !person}>
      {starting ? 'Starting...' : 'Create timelapse'}
    </button>
  </p>
{/if}

{#if jobStatus !== JOB_STATUS.idle}
  <div class="mt"><ProgressDisplay {jobStatus} {progress} /></div>
  {#if busy}
    <p class="muted mt">The work runs on the server. You can close this page and come back later.</p>
  {/if}
  {#if jobStatus === JOB_STATUS.completed}
    <p class="mt"><button type="button" onclick={onresult}>View result &gt;</button></p>
  {/if}
{/if}
