<script>
  // About: the running version (GET /api/version, as VersionBadge shows it) and
  // where this app comes from.
  import { onMount } from 'svelte';
  import { API } from '../constants.js';
  import { t, locale } from './i18n.svelte.js';
  import { CHANGELOG } from './changelog.js';

  const UPSTREAM = 'https://github.com/ArnaudCrl/immich-automated-selfie-timelapse';
  const FORK = 'https://github.com/rheckly/immich-automated-selfie-timelapse';

  // immich: { status: 'checking'|'connected'|'error', version, error } from GuidedApp
  let { immich, onretry, changelogOpen = false, onchangelog } = $props();

  const changeCount = CHANGELOG.reduce((n, r) => n + r.changes.length, 0);

  let info = $state(null);
  let failed = $state(false);

  onMount(async () => {
    try {
      const res = await fetch(API.version);
      if (!res.ok) throw new Error(String(res.status));
      info = await res.json();
    } catch {
      failed = true;
    }
  });

  function when(iso) {
    const d = iso ? new Date(iso) : null;
    return d && !isNaN(d) ? d.toLocaleString(locale()) : '';
  }
</script>

<p class="aboutName">{t('app.name')}</p>

<h3 class="paneSection">{t('about.version')}</h3>
{#if info}
  <table class="aboutTable">
    <tbody>
      <tr><th>{t('about.version')}</th><td>{info.version.replace(/^v/, '')}</td></tr>
      {#if info.build_date}<tr><th>{t('about.built')}</th><td>{when(info.build_date)}</td></tr>{/if}
      {#if info.commit}<tr><th>{t('about.commit')}</th><td>
        <!-- The version maps to GitHub through the commit; "-dirty" test builds have no exact commit -->
        <a href={`${FORK}/commit/${info.commit.replace(/-dirty$/, '')}`} target="_blank" rel="noopener noreferrer"><code>{info.commit}</code></a>
      </td></tr>{/if}
    </tbody>
  </table>
{:else if failed}
  <p class="muted">{t('about.noVersion')}</p>
{:else}
  <p class="muted">{t('common.loading')}</p>
{/if}

<h3 class="paneSection">{t('immich.title')}</h3>
<table class="aboutTable">
  <tbody>
    <tr><th>{t('immich.status')}</th><td>
      <span class="immichDot {immich.status}"></span>
      {immich.status === 'connected' ? t('immich.connected') : immich.status === 'checking' ? t('immich.checking') : t('immich.error')}
    </td></tr>
    {#if immich.version}<tr><th>{t('about.version')}</th><td>{immich.version.replace(/^v/, '')}</td></tr>{/if}
    {#if immich.error}<tr><th></th><td class="err">{immich.error}</td></tr>{/if}
  </tbody>
</table>
{#if immich.status === 'error'}
  <p class="mt"><button type="button" onclick={onretry}>{t('immich.retry')}</button></p>
{/if}

<h3 class="paneSection">{t('about.origin')}</h3>
<p>
  {t('about.upstream')}
  <a href={UPSTREAM} target="_blank" rel="noopener noreferrer">immich-automated-selfie-timelapse</a>
  {t('about.by')}
</p>
<p class="mt">
  {t('about.fork')}
  <a href={FORK} target="_blank" rel="noopener noreferrer">rheckly/immich-automated-selfie-timelapse</a>,
  {t('about.branch')} <code>happy-pc</code>.
</p>

<div class="paneSection sectionHead">
  <h3>{t('changelog.button')}</h3>
  <button type="button" class="settingOpen" class:on={changelogOpen} title={t('common.openRight')} aria-label={t('changelog.title')} onclick={onchangelog}>&gt;</button>
</div>
<p class="muted">{t('changelog.sum', { changes: changeCount, versions: CHANGELOG.length })}</p>

<style>
  .immichDot {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-right: 6px;
    border-radius: 50%;
    vertical-align: middle;
    background: #dc2626;
  }
  .immichDot.connected { background: #22c55e; }
  .immichDot.checking { background: #f59e0b; }
</style>
