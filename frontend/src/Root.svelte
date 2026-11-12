<script>
  // Chooses between the classic single-page view (App.svelte, unchanged)
  // and the guided process view with Mosaik-style panels.
  import App from './App.svelte';
  import GuidedApp from './lib/guided/GuidedApp.svelte';
  import VersionBadge from './lib/components/VersionBadge.svelte';
  import { STORAGE_KEYS } from './lib/constants.js';

  function loadView() {
    try {
      return localStorage.getItem(STORAGE_KEYS.view) === 'guided' ? 'guided' : 'classic';
    } catch {
      return 'classic';
    }
  }

  let view = $state(loadView());

  function setView(next) {
    view = next;
    try {
      localStorage.setItem(STORAGE_KEYS.view, next);
    } catch {
      // Ignore storage errors, the choice just isn't remembered
    }
    window.scrollTo(0, 0);
  }
</script>

{#if view === 'guided'}
  <GuidedApp onswitch={setView} />
{:else}
  <nav class="view-switch" aria-label="View">
    <VersionBadge />
    <span class="label">View</span>
    <div class="seg">
      <button type="button" class="on" aria-pressed="true">Classic</button>
      <button type="button" aria-pressed="false" onclick={() => setView('guided')}>Guided</button>
    </div>
  </nav>
  <App />
{/if}

<style>
  .view-switch {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 0.5rem;
    max-width: 800px;
    margin: 0 auto;
    padding: 0.75rem 2rem 0;
    font-size: 0.8rem;
    color: #888;
  }

  .view-switch .label {
    margin-left: auto;
  }

  .seg {
    display: flex;
    border: 1px solid #333;
    border-radius: 6px;
    overflow: hidden;
  }

  .seg button {
    padding: 0.3rem 0.8rem;
    border: 0;
    background: #1a1a1a;
    color: #e0e0e0;
    font: inherit;
    cursor: pointer;
  }

  .seg button.on {
    background: #4f46e5;
    color: #fff;
    font-weight: 600;
    cursor: default;
  }
</style>
