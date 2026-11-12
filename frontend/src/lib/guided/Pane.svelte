<script>
  // One panel column: title with close button, scrollable body, sticky row
  // with "previous / next step" and Done at the bottom (Mosaik pattern).
  let {
    title,
    level = 1, // 1 = pane, 2 = pane2, 3 = right
    offLeft = false,
    wide = false,
    nav = [], // [{ label, onclick }]
    onclose,
    children,
  } = $props();
</script>

<aside class="pane l{level}" class:offLeft class:galleryPane={wide} aria-label={title}>
  <div class="paneHead">
    <h2>{title}</h2>
    <button type="button" class="x" title="Close" aria-label="Close {title}" onclick={onclose}>×</button>
  </div>
  <div class="panel">
    {@render children()}
    <div class="btnRow doneRow">
      {#each nav as n (n.label)}
        <button type="button" title="Go to {n.label}" onclick={n.onclick}>{n.label}</button>
      {/each}
      <button type="button" onclick={onclose}>Done</button>
    </div>
  </div>
</aside>
