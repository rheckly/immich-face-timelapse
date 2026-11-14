<script>
  // Shows the running server version (from GET /api/version) bottom left,
  // like Immich Mosaik: "Version 0.4.12 · 13.11.2026". Hover for commit and
  // exact build time.
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

  function day(iso) {
    const d = iso ? new Date(iso) : null;
    if (!d || isNaN(d)) return '';
    const p = (n) => String(n).padStart(2, '0');
    return `${p(d.getDate())}.${p(d.getMonth() + 1)}.${d.getFullYear()}`;
  }

  let label = $derived(
    info ? [`Version ${info.version.replace(/^v/, '')}`, day(info.build_date)].filter(Boolean).join(' · ') : ''
  );
  let details = $derived(
    info
      ? [info.commit && `Commit ${info.commit}`, info.build_date && `Build ${new Date(info.build_date).toLocaleString('de-CH')}`]
          .filter(Boolean)
          .join('\n')
      : ''
  );
</script>

{#if info}
  <div class="version" title={details}>{label}</div>
{/if}

<style>
  /* Same as #version in Immich Mosaik (app/index.html) */
  .version {
    position: fixed;
    left: 20px;
    bottom: 6px;
    z-index: 5;
    padding: 1px 7px;
    /* Classic view has no --bg/--muted: blend into its dark page */
    background: var(--bg, transparent);
    border-radius: 4px;
    font-size: 12px;
    color: var(--muted, #a0a4a8);
  }
</style>
