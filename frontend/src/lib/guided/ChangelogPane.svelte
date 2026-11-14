<script>
  // Changelog of the fork, grouped by version (data in changelog.js).
  import { CHANGELOG } from './changelog.js';
  import { t, i18n, locale } from './i18n.svelte.js';

  function day(iso) {
    const d = iso ? new Date(iso + 'T12:00:00') : null;
    return d && !isNaN(d) ? d.toLocaleDateString(locale()) : '';
  }

  // Filters: one release (or all) and the change types shown
  const TYPES = ['NEW', 'CHG', 'FIX', 'LNG'];
  let release = $state('');
  let types = $state(new Set(TYPES));
  function toggleType(ty) {
    const next = new Set(types);
    next.has(ty) ? next.delete(ty) : next.add(ty);
    types = next;
  }
  let shown = $derived(
    CHANGELOG.filter((rel) => !release || rel.version === release)
      .map((rel) => ({ ...rel, changes: rel.changes.filter((c) => types.has(c.type)) }))
      .filter((rel) => rel.changes.length)
  );
  let counts = $derived(
    Object.fromEntries(
      TYPES.map((ty) => [
        ty,
        CHANGELOG.filter((rel) => !release || rel.version === release).reduce(
          (n, rel) => n + rel.changes.filter((c) => c.type === ty).length,
          0
        ),
      ])
    )
  );
</script>

<p class="muted">{t('changelog.intro')}</p>

<div class="filters">
  <label for="g-cl-release">{t('changelog.release')}</label>
  <select id="g-cl-release" bind:value={release}>
    <option value="">{t('changelog.allReleases')}</option>
    {#each CHANGELOG as rel (rel.version)}
      <option value={rel.version}>{rel.version}{rel.released ? '' : ' (' + t('changelog.unreleased') + ')'}</option>
    {/each}
  </select>
  <div class="chips" role="group" aria-label={t('changelog.types')}>
    {#each TYPES as ty (ty)}
      <button
        type="button"
        class="chip {ty.toLowerCase()}"
        class:on={types.has(ty)}
        aria-pressed={types.has(ty)}
        title={t('changelog.type.' + ty)}
        onclick={() => toggleType(ty)}>{ty} <span class="n">{counts[ty]}</span></button
      >
    {/each}
  </div>
</div>

{#if !shown.length}<p class="muted">{t('changelog.none')}</p>{/if}

{#each shown as rel (rel.version)}
  <h3 class="paneSection relHead">
    <span>{rel.version}</span>
    <span class="muted relDate">{rel.released ? day(rel.date) : t('changelog.unreleased')}</span>
  </h3>
  {#if rel.note}<p class="muted">{rel.note[i18n.lang] ?? rel.note.de}</p>{/if}
  <ul class="changes">
    {#each rel.changes as c, i (i)}
      <li>
        <span class="tag {c.type.toLowerCase()}" title={t('changelog.type.' + c.type)}>{c.type}</span>
        <span>{c[i18n.lang] ?? c.de}</span>
      </li>
    {/each}
  </ul>
{/each}

<style>
  .relHead { display: flex; justify-content: space-between; align-items: baseline; gap: 8px; }
  .relDate { font-weight: normal; }
  .changes { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; font-size: 14px; }
  .changes li { display: flex; gap: 8px; align-items: baseline; }
  .tag {
    flex: none;
    width: 34px;
    padding: 1px 0;
    border-radius: 4px;
    text-align: center;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: #fff;
  }
  .tag.new, .chip.new { --c: #2e7d32; }
  .tag.chg, .chip.chg { --c: #2f6f8f; }
  .tag.fix, .chip.fix { --c: #b45309; }
  .tag.lng, .chip.lng { --c: #7b4ea3; }
  .tag { background: var(--c); }
  .filters { display: flex; flex-direction: column; gap: 6px; margin-bottom: 4px; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 4px; }
  .chip {
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid var(--c);
    background: transparent;
    color: var(--c);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.04em;
    cursor: pointer;
  }
  .chip.on { background: var(--c); color: #fff; }
  .chip .n { font-weight: normal; opacity: 0.85; }
</style>
