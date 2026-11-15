<script>
  // Profile button at the right end of the header: initials of the signed-in
  // user, a menu with the name and "Sign out". Only there when the server has
  // a login (LOGIN_PASSWORD); the look follows the login mockup shared by
  // Face-Timelapse, Mosaik and Reels.
  import { onMount } from 'svelte';
  import { getSession, logout } from './auth.js';
  import { t } from './guided/i18n.svelte.js';

  let session = $state(null);
  let open = $state(false);
  let root = $state();
  let firstItem = $state();
  let button = $state();

  onMount(() => {
    getSession().then((s) => (session = s));
  });

  /** "Roger Heckly" -> "RH", "admin" -> "A". */
  function initials(name) {
    const words = String(name || '').trim().split(/\s+/).filter(Boolean);
    if (words.length === 0) return '?';
    const first = [...words[0]][0];
    const last = words.length > 1 ? [...words[words.length - 1]][0] : '';
    return (first + last).toUpperCase();
  }

  let name = $derived(session?.name || session?.user || '');
  let letters = $derived(initials(name));

  async function toggle() {
    open = !open;
    if (open) {
      // Keyboard users land on the first entry
      await Promise.resolve();
      firstItem?.focus();
    }
  }

  function close(focusButton = false) {
    open = false;
    if (focusButton) button?.focus();
  }

  function onWindowClick(e) {
    if (open && root && !root.contains(e.target)) close();
  }

  function onWindowKeydown(e) {
    if (open && e.key === 'Escape') {
      e.stopPropagation();
      close(true);
    }
  }
</script>

<svelte:window onclick={onWindowClick} onkeydowncapture={onWindowKeydown} />

{#if session?.enabled && session.user}
  <div class="profile" bind:this={root}>
    <button
      type="button"
      class="avatar"
      class:open
      bind:this={button}
      aria-haspopup="menu"
      aria-expanded={open}
      title={name}
      aria-label={t('profile.menu', { name })}
      onclick={toggle}>{letters}</button
    >
    {#if open}
      <div class="menu" role="menu" aria-label={t('profile.menu', { name })}>
        <div class="who">
          <span class="avatar big" aria-hidden="true">{letters}</span>
          <div>
            <div class="name">{name}</div>
            <div class="sub">{t('profile.signedInAs', { user: session.user })}<br />Face-Timelapse</div>
          </div>
        </div>
        <button type="button" class="item" role="menuitem" bind:this={firstItem} onclick={logout}>
          <svg viewBox="0 0 24 24" aria-hidden="true"
            ><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" /><path d="M16 17l5-5-5-5" /><path d="M21 12H9" /></svg
          >{t('profile.logout')}
        </button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .profile {
    position: relative;
    flex: none;
  }
  .avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    padding: 0;
    margin: 0;
    font: 600 14px system-ui, sans-serif;
    color: var(--on-accent, #111);
    background: var(--accent, #6db3d6);
    border: 0;
    cursor: pointer;
  }
  .avatar.open,
  .avatar:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--bg, #17191b),
      0 0 0 4px var(--accent, #6db3d6);
  }
  .avatar.big {
    width: 44px;
    height: 44px;
    font-size: 15px;
    cursor: default;
    flex: none;
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 10px);
    width: 280px;
    background: var(--card, #222528);
    color: var(--text, #e8e8e6);
    border: 1px solid var(--line, #3a3e42);
    border-radius: 12px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.45);
    padding: 8px;
    z-index: 50;
    text-align: left;
  }
  .who {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 10px 10px 14px;
    border-bottom: 1px solid var(--line, #3a3e42);
    margin-bottom: 8px;
  }
  .name {
    font-weight: 700;
    font-size: 16px;
    overflow-wrap: anywhere;
  }
  .sub {
    font-size: 13px;
    color: var(--muted, #999);
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    margin: 0;
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 15px;
    color: inherit;
    background: transparent;
    border: 2px solid transparent;
    cursor: pointer;
    text-align: left;
  }
  .item:hover,
  .item:focus-visible {
    outline: none;
    background: color-mix(in srgb, var(--accent, #6db3d6) 12%, transparent);
    border-color: var(--accent, #6db3d6);
  }
  .item svg {
    width: 18px;
    height: 18px;
    flex: none;
    stroke: currentColor;
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
