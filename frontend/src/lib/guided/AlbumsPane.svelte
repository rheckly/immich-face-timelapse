<script>
  import { t, fmt, locale } from './i18n.svelte.js';
  // Source picker as in Mosaik: albums, other people in the photo and people
  // albums are shared with (Smart Albums). A photo counts if it is in one of
  // the chosen albums or also shows one of the chosen people.
  import { STORAGE_KEYS } from '../constants.js';

  let {
    albums = [],
    people = [],
    recipients = null, // null = Smart Albums not configured, tab hidden
    recipientsError = '',
    counts = null, // { personId: n } photos showing both, null while not loaded
    loading = false,
    error = null,
    picked = [], // [{ id, kind: 'album' | 'person' | 'recipient', name }]
    disabled = false,
    onchange,
    onpeopletab, // asks for the people counts
  } = $props();

  const DEFAULT_SORT = { albums: { key: 'name', dir: -1 }, people: { key: 'name', dir: -1 }, recipients: { key: 'name', dir: -1 } };

  function readSort() {
    try {
      const s = JSON.parse(localStorage.getItem(STORAGE_KEYS.sourceSort));
      const ok = (o) => o && ['name', 'count', 'date'].includes(o.key) && (o.dir === 1 || o.dir === -1);
      return Object.fromEntries(Object.entries(DEFAULT_SORT).map(([tab, d]) => [tab, ok(s?.[tab]) ? s[tab] : d]));
    } catch {
      return { ...DEFAULT_SORT };
    }
  }

  let tab = $state('albums');
  let query = $state('');
  let sort = $state(readSort());
  let search;

  let order = $derived(sort[tab]);
  let pickedIds = $derived(new Set(picked.map((p) => p.id)));
  let all = $derived(tab === 'albums' ? albums : tab === 'people' ? people : recipients || []);

  const plain = (s) => (s || '').toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '');

  function monthText(value) {
    const d = value ? new Date(value) : null;
    if (!d || isNaN(d)) return '';
    return `${String(d.getMonth() + 1).padStart(2, '0')}.${d.getFullYear()}`;
  }
  function albumDate(a) {
    const from = monthText(a.start_date);
    const to = monthText(a.end_date);
    return from && to && from !== to ? `${from}–${to}` : from || to;
  }
  function countOf(item) {
    if (tab === 'albums') return item.asset_count ?? 0;
    if (tab === 'recipients') return item.albums?.length ?? 0;
    return counts && counts[item.id] !== undefined ? counts[item.id] : -1;
  }
  function sortValue(item, key) {
    if (key === 'name') return plain(item.name);
    if (key === 'date') return item.end_date || item.start_date || '';
    return countOf(item);
  }

  let rows = $derived.by(() => {
    const q = plain(query.trim());
    const { key, dir } = order;
    return all
      .filter((i) => plain(i.name).includes(q))
      .sort((x, y) => {
        const a = sortValue(x, key);
        const b = sortValue(y, key);
        if (key === 'date' && !a !== !b) return a ? -1 : 1; // albums without date last
        return (a < b ? -1 : a > b ? 1 : 0) * dir || plain(x.name).localeCompare(plain(y.name), locale());
      });
  });

  let empty = $derived(
    rows.length
      ? ''
      : loading
        ? t('common.loading')
        : query.trim()
          ? t('sources.noMatch')
          : tab === 'albums'
            ? t('sources.noAlbums')
            : tab === 'recipients'
              ? recipientsError || t('sources.noRecipients')
              : t('sources.noPeople')
  );

  function setTab(name) {
    tab = name;
    query = '';
    if (name === 'people') onpeopletab?.();
    search?.focus();
  }
  function setSort(key) {
    const o = sort[tab];
    sort[tab] = o.key === key ? { key, dir: -o.dir } : { key, dir: key === 'name' ? 1 : -1 };
    try {
      localStorage.setItem(STORAGE_KEYS.sourceSort, JSON.stringify(sort));
    } catch {
      // The sort just isn't remembered
    }
  }
  function toggle(item) {
    if (disabled) return;
    const kind = tab === 'albums' ? 'album' : tab === 'people' ? 'person' : 'recipient';
    onchange(pickedIds.has(item.id) ? picked.filter((p) => p.id !== item.id) : [...picked, { id: item.id, kind, name: item.name }]);
  }
  function countText(item) {
    if (tab === 'albums') return fmt(item.asset_count ?? 0);
    if (tab === 'recipients') {
      const n = item.albums?.length ?? 0;
      return t('sources.nAlbums', { n });
    }
    if (!counts) return '…';
    return counts[item.id] !== undefined ? fmt(counts[item.id]) : '–';
  }
</script>

<div class="srcList">
  <div class="seg srcTabs" role="tablist">
    <button type="button" role="tab" aria-selected={tab === 'albums'} class:on={tab === 'albums'} onclick={() => setTab('albums')}>
      {t('kind.albums')} ({fmt(albums.length)})
    </button>
    <button type="button" role="tab" aria-selected={tab === 'people'} class:on={tab === 'people'} onclick={() => setTab('people')}>
      {t('kind.people')} ({fmt(people.length)})
    </button>
    {#if recipients !== null}
      <button type="button" role="tab" aria-selected={tab === 'recipients'} class:on={tab === 'recipients'} onclick={() => setTab('recipients')}>
        {t('kind.recipients')} ({fmt(recipients.length)})
      </button>
    {/if}
  </div>
  <input class="mt" type="search" placeholder={t('common.searchName')} aria-label={t('common.searchName')} bind:value={query} bind:this={search} />
  {#if error}
    <p class="err mt">{error}</p>
  {/if}
  <div class="tableWrap srcTableWrap">
    <table class="srcTable">
      <thead>
        <tr>
          {#each tab === 'albums' ? ['name', 'count', 'date'] : ['name', 'count'] as key (key)}
            <th class:num={key === 'count'}>
              <button type="button" class="sort" class:on={order.key === key} onclick={() => setSort(key)}>
                {t('sources.col.' + key)}{order.key === key ? (order.dir > 0 ? ' ▲' : ' ▼') : ''}
              </button>
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#each rows as item (item.id)}
          <tr
            data-id={item.id}
            onclick={(e) => {
              if (!e.target.closest('label')) toggle(item);
            }}
          >
            <td>
              <label class="check">
                <input type="checkbox" checked={pickedIds.has(item.id)} {disabled} onchange={() => toggle(item)} />
                {#if tab === 'people'}
                  <img
                    class="face"
                    loading="lazy"
                    alt=""
                    src="/api/people/{encodeURIComponent(item.id)}/thumbnail"
                    onerror={(e) => (e.currentTarget.style.visibility = 'hidden')}
                  />
                {/if}
                <span>{item.name}</span>
              </label>
            </td>
            <td class="num">{countText(item)}</td>
            {#if tab === 'albums'}<td>{albumDate(item)}</td>{/if}
          </tr>
        {/each}
      </tbody>
    </table>
    {#if empty}<p class="muted" style="margin:10px">{empty}</p>{/if}
  </div>
  <p class="muted" style="margin:8px 0 0">{t('sources.hint')}</p>
</div>
