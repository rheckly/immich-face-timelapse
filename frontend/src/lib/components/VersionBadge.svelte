<script>
  // Shows the running server version (from GET /api/version) so it's always
  // visible which build is live. Hover for commit and build date.
  import { onMount } from 'svelte';
  import { API } from '../constants.js';

  let info = $state(null);

  onMount(async () => {
    try {
      const res = await fetch(API.version);
      if (res.ok) info = await res.json();
    } catch {
      // Version is informational only, ignore errors
    }
  });

  let label = $derived(
    info ? (info.version.startsWith('v') ? info.version : `v${info.version}`) : ''
  );
  let details = $derived(
    info
      ? [
          `Version ${info.version}`,
          info.commit && `Commit ${info.commit}`,
          info.build_date && `Built ${info.build_date}`,
        ]
          .filter(Boolean)
          .join('\n')
      : ''
  );
</script>

{#if info}
  <span class="version" title={details}>{label}</span>
{/if}

<style>
  .version {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.75rem;
    color: inherit;
    opacity: 0.7;
    white-space: nowrap;
    cursor: default;
  }
</style>
