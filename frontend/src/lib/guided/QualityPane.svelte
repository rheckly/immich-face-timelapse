<script>
  import { FILTERS } from './config.js';

  let { config = $bindable(), open2 = null, onopen } = $props();
  let p = $derived(config.processing);
</script>

<p class="muted" style="margin-bottom:10px">
  Filters skip photos that would disturb the video. Use &gt; to adjust a filter on the right.
</p>
<div class="settingRows">
  {#each FILTERS as f (f.key)}
    {@const on = open2 === 'f:' + f.key}
    <div class="settingRow" class:on>
      <input type="checkbox" bind:checked={config.processing[f.key].enabled} title="Filter on or off" aria-label="{f.title} on" />
      <div class="settingText"><strong>{f.title}</strong>{p[f.key].enabled ? f.summary(p) : 'Off'}</div>
      <button type="button" class="settingOpen" class:on title="Open on the right" aria-label="Adjust {f.title}" onclick={() => onopen('f:' + f.key)}>&gt;</button>
    </div>
  {/each}
  <div class="settingRow" class:on={open2 === 'adv'}>
    <div class="settingText">
      <strong>Advanced</strong>
      {p.use_preview ? 'Preview images' : 'Original images'}, {p.max_workers} workers{p.output.keep_intermediates ? ', debug images' : ''}
    </div>
    <button type="button" class="settingOpen" class:on={open2 === 'adv'} title="Open on the right" aria-label="Advanced settings" onclick={() => onopen('adv')}>&gt;</button>
  </div>
</div>
<p class="muted mt">Changes are saved automatically and also apply to the classic view.</p>
