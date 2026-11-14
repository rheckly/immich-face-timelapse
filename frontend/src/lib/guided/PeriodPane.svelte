<script>
  import { t } from './i18n.svelte.js';
  // Step 3: which photos by date, and how many per day / week / month.
  let { dateFrom = $bindable(''), dateTo = $bindable(''), config = $bindable(), disabled = false } = $props();
  let ti = $derived(config?.processing.time_interval);
</script>

<fieldset {disabled}>
  <h3 class="paneSection first">{t('period.range')}</h3>
  <p class="muted">{t('period.rangeHint')}</p>
  <div class="row mt">
    <div><label for="g-from" style="margin-top:0">{t('period.from')}</label><input id="g-from" type="date" bind:value={dateFrom} max={dateTo || undefined} /></div>
    <div><label for="g-to" style="margin-top:0">{t('period.to')}</label><input id="g-to" type="date" bind:value={dateTo} min={dateFrom || undefined} /></div>
  </div>
  {#if dateFrom || dateTo}
    <div class="btnRow mt">
      <button type="button" onclick={() => ((dateFrom = ''), (dateTo = ''))}>{t('period.allDates')}</button>
    </div>
  {/if}

  {#if config}
    <h3 class="paneSection">{t('period.count')}</h3>
    <label class="check" style="margin-top:0">
      <input type="checkbox" bind:checked={config.processing.time_interval.enabled} /> {t('period.limit')}
    </label>
    <p class="muted mt">{t('period.limitHint')}</p>
    {#if ti.enabled}
      <div class="inline mt">
        <span>{t('period.atMost')}</span>
        <input type="number" min="1" max="10" aria-label={t('common.photos')} bind:value={config.processing.time_interval.max_photos} />
        <span>{t('period.per')}</span>
        <div class="toggles">
          {#each ['day', 'week', 'month'] as r (r)}
            <button
              type="button"
              class:on={ti.time_range === r}
              aria-pressed={ti.time_range === r}
              onclick={() => (config.processing.time_interval.time_range = r)}>{t('period.' + r)}</button
            >
          {/each}
        </div>
      </div>
    {/if}
    <p class="muted mt">{t('common.autosave')}</p>
  {/if}
</fieldset>
