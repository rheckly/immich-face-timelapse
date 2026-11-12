<script>
  import AlignmentIndicator from '../components/visual/AlignmentIndicator.svelte';
  import { CODECS, POSITIONS } from './config.js';

  let { config = $bindable(), musicOpen = false, onmusic } = $props();
  let p = $derived(config.processing);
  let v = $derived(config.video);
  let tracks = $derived(v.music_tracks ?? []);
  let fpsByMusic = $derived(v.fit_to_music && tracks.length > 0);
</script>

<h3 class="paneSection first">Format</h3>
<div class="row">
  <div>
    <label for="g-size">Output size (square, px)</label>
    <input id="g-size" type="number" min="64" max="4096" bind:value={config.processing.output.size} />
  </div>
  <div>
    <label for="g-codec">Codec</label>
    <select id="g-codec" bind:value={config.video.codec}>
      {#each Object.entries(CODECS) as [value, label] (value)}
        <option {value}>{label}</option>
      {/each}
    </select>
  </div>
</div>
<label for="g-eyes">Eye distance (larger = zoom in)</label>
<div class="valueRow">
  <input id="g-eyes" type="range" min="0.05" max="0.4" step="0.01" bind:value={config.processing.alignment.eye_distance} />
  <span class="val">{(Number(p.alignment.eye_distance) * 100).toFixed(0)} %</span>
</div>
<div class="indicator"><AlignmentIndicator eyeDistance={Number(p.alignment.eye_distance)} /></div>

<h3 class="paneSection">Show the date</h3>
<label class="check" style="margin-top:0">
  <input type="checkbox" bind:checked={config.processing.timestamp.enabled} /> Show the date in the video
</label>
{#if p.timestamp.enabled}
  <div class="row">
    <div>
      <label for="g-pos">Position</label>
      <select id="g-pos" bind:value={config.processing.timestamp.position}>
        {#each Object.entries(POSITIONS) as [value, label] (value)}
          <option {value}>{label}</option>
        {/each}
      </select>
    </div>
    <div>
      <p class="lbl">Show</p>
      <div class="toggles">
        <button type="button" class:on={p.timestamp.day} aria-pressed={p.timestamp.day} onclick={() => (config.processing.timestamp.day = !p.timestamp.day)}>Day</button>
        <button type="button" class:on={p.timestamp.month} aria-pressed={p.timestamp.month} onclick={() => (config.processing.timestamp.month = !p.timestamp.month)}>Month</button>
        <button type="button" class:on={p.timestamp.year} aria-pressed={p.timestamp.year} onclick={() => (config.processing.timestamp.year = !p.timestamp.year)}>Year</button>
      </div>
    </div>
  </div>
{/if}

<h3 class="paneSection">Video</h3>
<label for="g-fps" style="margin-top:0">Frames per second</label>
<div class="valueRow">
  <input id="g-fps" type="range" min="1" max="60" step="1" bind:value={config.video.framerate} disabled={fpsByMusic} />
  <span class="val">{v.framerate} fps</span>
</div>
{#if fpsByMusic}<p class="muted">Set by the length of the music, see Music.</p>{/if}
<label for="g-crf">Quality (CRF, lower = better and larger)</label>
<div class="valueRow">
  <input id="g-crf" type="range" min="15" max="40" step="1" bind:value={config.video.crf} />
  <span class="val">{v.crf}</span>
</div>
<label class="check mt">
  <input type="checkbox" bind:checked={config.video.enabled} /> Create the video right after the photos
</label>

<div class="paneSection sectionHead">
  <h3>Music</h3>
  <button type="button" class="settingOpen" class:on={musicOpen} title="Open on the right" aria-label="Music" onclick={onmusic}>&gt;</button>
</div>
<p class="muted">
  {#if tracks.length === 0}
    No music
  {:else}
    {tracks.join(', ')}{v.fit_to_music ? ' · video fits the length of the music' : ''}
  {/if}
</p>
