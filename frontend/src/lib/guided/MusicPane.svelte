<script>
  import { formatDuration } from './config.js';

  let { config = $bindable(), music, loading = false, error = null, onrefresh } = $props();

  let tracks = $derived(config.video.music_tracks ?? []);
  let missing = $derived(music.available ? tracks.filter((n) => !music.tracks.some((t) => t.name === n)) : []);
  let totalSecs = $derived(
    tracks.reduce((sum, n) => sum + (music.tracks.find((t) => t.name === n)?.duration_secs ?? 0), 0)
  );

  function toggleTrack(name) {
    config.video.music_tracks = tracks.includes(name) ? tracks.filter((t) => t !== name) : [...tracks, name];
  }
</script>

<div class="sectionHead">
  <p class="muted">Selected tracks play in the numbered order and fade out at the end.</p>
  <button type="button" onclick={onrefresh} disabled={loading}>{loading ? 'Loading...' : 'Refresh'}</button>
</div>

{#if error}
  <p class="err mt">{error}</p>
{:else if !music.available}
  <p class="muted mt">Music folder not found. Mount a folder at <code>/app/music</code>.</p>
{:else if music.tracks.length === 0}
  <p class="muted mt">No audio files (mp3, m4a, aac, wav, flac, ogg, opus) in the music folder.</p>
{:else}
  <ul class="music">
    {#each music.tracks as t (t.name)}
      {@const order = tracks.indexOf(t.name)}
      <li>
        <label class="check">
          <input type="checkbox" checked={order >= 0} onchange={() => toggleTrack(t.name)} />
          <span class="order">{order >= 0 ? order + 1 : ''}</span>
          <span class="name">{t.name}</span>
          <span class="muted">{formatDuration(t.duration_secs)}</span>
        </label>
      </li>
    {/each}
  </ul>
{/if}

{#if missing.length > 0}
  <p class="err mt">
    Selected but missing: {missing.join(', ')}
    <button type="button" onclick={() => (config.video.music_tracks = tracks.filter((t) => !missing.includes(t)))}>Remove</button>
  </p>
{/if}
{#if tracks.length > 0}
  <p class="muted">Length of the selected music: {formatDuration(totalSecs)}</p>
{/if}

<label for="g-fade">Fade-out at the end</label>
<div class="valueRow">
  <input id="g-fade" type="range" min="0" max="15" step="0.5" bind:value={config.video.music_fade_out_secs} />
  <span class="val">{Number(config.video.music_fade_out_secs).toFixed(1)} s</span>
</div>

<label class="check mt">
  <input type="checkbox" bind:checked={config.video.fit_to_music} /> Fit the video to the music
</label>
<p class="muted">Frames per second = number of photos ÷ length of the music, within the range below.</p>
{#if config.video.fit_to_music}
  <div class="row">
    <div>
      <label for="g-minfps">At least (fps)</label>
      <input id="g-minfps" type="number" min="1" max="60" step="0.5" bind:value={config.video.min_framerate} />
    </div>
    <div>
      <label for="g-maxfps">At most (fps)</label>
      <input id="g-maxfps" type="number" min="1" max="120" step="0.5" bind:value={config.video.max_framerate} />
    </div>
  </div>
  {#if Number(config.video.min_framerate) > Number(config.video.max_framerate)}
    <p class="err mt">The minimum is above the maximum; the maximum is raised to match.</p>
  {/if}
{/if}
