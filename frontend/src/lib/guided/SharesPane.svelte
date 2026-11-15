<script>
  // Overview of all shared links ("Geteilte Timelapse"), built like the panel
  // "Geteilte Mosaike" in Immich Mosaik (index.html: shareRow/drawShares, same
  // CSS): filter + "delete expired" on top, one row per link with thumbnail,
  // facts as muted lines and small action buttons, the counting rule below.
  import { t, locale } from './i18n.svelte.js';
  import { formatFolderVersion } from '../utils.js';
  import { shares, loadShares, updateShare, revokeShare, deleteExpiredShares, DURATIONS, formatDate, copyText } from './shares.svelte.js';

  // folders: the output folders (for the timelapse's name); onopen(folder): open it in the history
  let { folders = [], onopen } = $props();

  const FILTERS = ['active', 'expired', 'all'];
  let filter = $state('all'); // as in Mosaik: default "All"
  let busy = $state(false);
  let error = $state('');
  let copied = $state('');
  let now = $state(Date.now());

  $effect(() => {
    loadShares();
    const timer = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(timer);
  });

  const expired = (s) => !!s.expires_at && new Date(s.expires_at).getTime() <= now;
  let all = $derived([...shares.list].sort((a, b) => b.created_at.localeCompare(a.created_at)));
  let nExpired = $derived(all.filter(expired).length);
  let counts = $derived({ active: all.length - nExpired, expired: nExpired, all: all.length });
  let list = $derived(filter === 'all' ? all : all.filter((s) => expired(s) === (filter === 'expired')));

  function folderLabel(name) {
    const f = folders.find((x) => x.name === name);
    if (!f) return name;
    const who = f.display_name ?? f.person ?? f.name;
    return f.version ? `${who} · ${formatFolderVersion(f.version)}` : who;
  }
  const videoUrl = (name) => `/output/${encodeURIComponent(name)}/${encodeURIComponent(name)}.mp4#t=1`;
  const dateTime = (iso) => `${formatDate(iso)}, ${new Date(iso).toLocaleTimeString(locale(), { hour: '2-digit', minute: '2-digit' })}`;
  function validity(s) {
    if (!s.expires_at) return t('shares.unlimited');
    const date = formatDate(s.expires_at);
    if (expired(s)) return t('shares.expiredOn', { date });
    return t('shares.left', { n: Math.max(1, Math.ceil((new Date(s.expires_at).getTime() - now) / 86_400_000)), date });
  }
  const viewsText = (s) => (s.views ? t('shares.views', { n: s.views, date: dateTime(s.last_viewed_at) }) : t('shares.noViews'));
  const durLabel = (d) => (d === null ? t('share.forever') : d === 1 ? t('share.day') : t('share.days', { n: d }));

  async function run(fn) {
    busy = true;
    error = '';
    try {
      await fn();
    } catch (e) {
      error = e.message;
    }
    now = Date.now(); // a new validity counts from now, not from the last minute tick
    busy = false;
  }
  const remove = (s) => {
    if (confirm(t('shares.confirmOne', { name: s.title || folderLabel(s.folder) }))) run(() => revokeShare(s.token));
  };
  const removeExpired = () => {
    if (confirm(t('shares.confirmExpired', { n: nExpired }))) run(deleteExpiredShares);
  };
  // As in Mosaik: picking a validity changes it at once (from today)
  function changeValidity(s, e) {
    const v = e.currentTarget.value;
    e.currentTarget.value = '';
    if (v !== '') run(() => updateShare(s.token, s.title, v === 'null' ? null : Number(v)));
  }
  async function copy(s) {
    if (await copyText(s.url)) {
      copied = s.token;
      setTimeout(() => copied === s.token && (copied = ''), 2500);
    } else prompt(t('shares.copyKeys'), s.url);
  }
</script>

{#if shares.error}
  <p class="err" style="margin-top:0">{shares.error}</p>
{:else if !shares.loaded}
  <p class="muted" style="margin-top:0">{t('common.loading')}</p>
{:else}
  <div class="shareTools">
    <span class="shareFilter" role="group" aria-label={t('shares.filter')}>
      {#each FILTERS as key (key)}
        <button type="button" class:on={filter === key} aria-pressed={filter === key} onclick={() => (filter = key)}
          >{t('shares.f.' + key)} ({counts[key]})</button
        >
      {/each}
    </span>
    {#if nExpired}<button type="button" disabled={busy} onclick={removeExpired}>{t('shares.deleteExpired')}</button>{/if}
  </div>
  {#if error}<p class="err">{error}</p>{/if}
  {#if !shares.public_url && all.length}<p class="muted" style="margin:0 0 8px">{t('shares.noUrl')}</p>{/if}

  <div class="shareList">
    {#if list.length === 0}
      <p class="muted" style="margin:0">
        {all.length === 0 ? t('shares.none') : t(filter === 'expired' ? 'shares.noneExpired' : 'shares.noneActive')}
      </p>
    {/if}
    {#each list as s (s.token)}
      {@const gone = expired(s)}
      <div class="shareRow" class:expired={gone}>
        <video src={videoUrl(s.folder)} preload="metadata" muted playsinline aria-hidden="true" tabindex="-1"></video>
        <div class="shareMain">
          <strong>{s.title || t('share.untitled')}</strong>
          <span class="shareFacts"
            ><button type="button" class="ftLink" title={t('shares.openFt')} onclick={() => onopen(s.folder)}>{folderLabel(s.folder)}</button>
            · {t('shares.created', { date: formatDate(s.created_at) })}</span
          >
          <span class="shareFacts">{validity(s)}</span>
          <span class="shareFacts">{viewsText(s)}</span>
          <div class="shareActions">
            <button type="button" disabled={!s.url} onclick={() => copy(s)}>{copied === s.token ? t('share.copied') : t('share.copy')}</button>
            <button type="button" disabled={gone || !s.url} title={s.url ? '' : t('shares.noUrl')} onclick={() => window.open(s.url, '_blank', 'noopener')}
              >{t('shares.open')}</button
            >
            <select aria-label={t('shares.changeValidity')} disabled={busy} onchange={(e) => changeValidity(s, e)}>
              <option value="">{t('shares.changeValidity')}</option>
              {#each DURATIONS as d (d)}<option value={String(d)}>{durLabel(d)}</option>{/each}
            </select>
            <button type="button" disabled={busy} onclick={() => remove(s)}>{t('common.delete')}</button>
          </div>
        </div>
      </div>
    {/each}
  </div>
  <p class="muted" style="margin:8px 0 0">{t('shares.counted')}</p>
{/if}

<style>
  /* Copied from Immich Mosaik (index.html, #panelShares … .shareRow.expired) */
  .shareTools { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; justify-content: space-between; flex: none; margin-bottom: 8px; }
  .shareFilter { display: inline-flex; gap: 4px; }
  .shareFilter button.on { border-color: var(--accent); color: var(--accent); font-weight: 600; }
  .shareList { display: flex; flex-direction: column; gap: 6px; flex: 1; min-height: 0; overflow: auto; }
  .shareRow { display: flex; gap: 10px; align-items: flex-start; padding: 8px; border: 1px solid var(--line); border-radius: 5px; }
  .shareRow > video { width: 64px; height: 48px; object-fit: cover; border-radius: 3px; background: var(--bg); flex: none; pointer-events: none; }
  .shareMain { flex: 1; min-width: 0; }
  .shareRow strong { display: block; font-weight: 600; overflow-wrap: anywhere; }
  .shareFacts { display: block; color: var(--muted); font-size: 13px; }
  .shareActions { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
  .shareActions button, .shareActions select { width: auto; margin: 0; padding: 3px 8px; font-size: 13px; }
  .shareRow.expired > video, .shareRow.expired strong, .shareRow.expired .shareFacts { opacity: 0.55; }
  /* FT only: the timelapse's name opens it in the history */
  .ftLink { padding: 0; border: 0; background: none; color: var(--accent); font-size: inherit; cursor: pointer; overflow-wrap: anywhere; }
  .ftLink:hover { text-decoration: underline; }
</style>
