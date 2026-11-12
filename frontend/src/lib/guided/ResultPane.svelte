<script>
  import { formatSize } from '../utils.js';

  // stamp changes whenever the result may have changed, so the video is not served from the cache
  let { folderName, folder = null, stamp = 0, photosOpen = false, onphotos } = $props();

  let videoUrl = $derived(
    `/output/${encodeURIComponent(folderName)}/${encodeURIComponent(folderName)}.mp4?v=${stamp}`
  );
  let failed = $state(false);
  $effect(() => {
    videoUrl;
    failed = false;
  });
</script>

{#if failed}
  <div class="videoMissing">
    <p>No video yet.</p>
    <p class="muted">The video may still be in the making, or it was not created. Open the used photos to create it.</p>
  </div>
{:else}
  {#key videoUrl}
    <video controls autoplay muted loop playsinline src={videoUrl} onerror={() => (failed = true)}>
      <track kind="captions" />
    </video>
  {/key}
{/if}
<p class="muted mt">
  {folderName}
  {#if folder}· {folder.image_count} photos · {formatSize(folder.size_bytes)}{/if}
</p>

{#if !failed}
  <h3 class="paneSection">Download</h3>
  <div class="btnRow">
    <a class="btn primary" href={videoUrl} download="{folderName}.mp4">Download video</a>
    <a class="btn" href={videoUrl} target="_blank" rel="noopener">Open in new tab</a>
  </div>
{/if}

<div class="paneSection sectionHead">
  <h3>Used photos</h3>
  <button type="button" class="settingOpen" class:on={photosOpen} title="Open on the right" aria-label="Used photos" onclick={onphotos}>&gt;</button>
</div>
<p class="muted">Look at single photos, remove them and create the video again.</p>
