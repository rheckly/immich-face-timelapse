<script>
  // Changelog of the fork, grouped by version (data in changelog.js).
  import { CHANGELOG } from './changelog.js';
  import { t, i18n, locale } from './i18n.svelte.js';

  function day(iso) {
    const d = iso ? new Date(iso + 'T12:00:00') : null;
    return d && !isNaN(d) ? d.toLocaleDateString(locale()) : '';
  }
</script>

<p class="muted">{t('changelog.intro')}</p>

{#each CHANGELOG as rel (rel.version)}
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
  .tag.new { background: #2e7d32; }
  .tag.chg { background: #2f6f8f; }
  .tag.fix { background: #b45309; }
</style>
