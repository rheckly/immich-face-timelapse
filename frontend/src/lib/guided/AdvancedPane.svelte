<script>
  let { config = $bindable(), onreset } = $props();
</script>

<label class="check" style="margin-top:0">
  <input type="checkbox" bind:checked={config.processing.use_preview} /> Use preview images
</label>
<p class="muted">Download the Immich 1440p previews instead of the originals. Much faster.</p>

<label for="g-workers">Parallel workers</label>
<div class="valueRow">
  <input id="g-workers" type="range" min="1" max="16" step="1" bind:value={config.processing.max_workers} />
  <span class="val">{config.processing.max_workers}</span>
</div>

<label class="check mt">
  <input type="checkbox" bind:checked={config.processing.output.keep_intermediates} /> Keep debug images
</label>
<p class="muted">Save the intermediate processing steps next to the result.</p>

<h3 class="paneSection">Reset</h3>
<p class="muted" style="margin-bottom:8px">Set all settings of steps 3 to 5 back to the defaults.</p>
<button type="button" onclick={onreset}>Reset to defaults</button>
