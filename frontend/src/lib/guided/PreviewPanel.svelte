<script>
  import { t, fmt } from './i18n.svelte.js';
  // Preview of the quality filters on one photo of the selection: the photo with the
  // face, the resulting frame and each filter's verdict. The server measures, the
  // verdicts are judged here so they follow the sliders at once.
  import { untrack } from 'svelte';
  import { FILTERS } from './config.js';
  import { preview, loadSamples, requestPreview, frameKey, pick } from './preview.svelte.js';

  let { config, selection, focus = null } = $props();

  let p = $derived(config.processing);
  let r = $derived(preview.result);
  let sample = $derived(preview.samples[preview.index]);

  $effect(() => {
    const sel = $state.snapshot(selection);
    untrack(() => loadSamples(sel));
  });

  $effect(() => {
    frameKey(config); // the settings that change the frame
    preview.samples;
    preview.index;
    untrack(() => requestPreview(config, selection.personId));
  });

  const num = (x) => Number(x);
  const pct = (x) => Math.round(num(x) * 100);
  const sign = (pass, ok, bad) => (pass ? ok : bad);

  // Verdict of one filter with the current thresholds
  function judge(key, v) {
    switch (key) {
      case 'face_resolution': {
        if (v.face_size == null) return null;
        const min = num(p.face_resolution.min_size);
        const pass = v.face_size >= min;
        return { pass, text: t('pv.face', { v: v.face_size, op: sign(pass, '≥', '<'), min }) };
      }
      case 'crop': {
        if (v.padding_percent == null) return null;
        const max = num(p.crop.max_padding_percent);
        const pass = v.padding_percent <= max;
        return { pass, text: t('pv.padding', { v: v.padding_percent.toFixed(1), op: sign(pass, '≤', '>'), max }) };
      }
      case 'brightness': {
        if (v.brightness == null) return null;
        const min = num(p.brightness.min_brightness);
        const max = num(p.brightness.max_brightness);
        const pass = v.brightness >= min && v.brightness <= max;
        return { pass, text: t(pass ? 'pv.brightIn' : 'pv.brightOut', { v: pct(v.brightness), min: pct(min), max: pct(max) }) };
      }
      case 'blur': {
        if (v.sharpness == null) return null;
        const min = num(p.blur.min_sharpness);
        const pass = v.sharpness >= min;
        return { pass, text: t('pv.sharp', { v: v.sharpness.toFixed(0), op: sign(pass, '≥', '<'), min: min.toFixed(0) }) };
      }
      case 'head_pose': {
        if (v.yaw == null || v.pitch == null) return null;
        const yaw = Math.abs(v.yaw);
        const pitch = Math.abs(v.pitch);
        const maxYaw = num(p.head_pose.max_yaw);
        const maxPitch = num(p.head_pose.max_pitch);
        const pass = yaw <= maxYaw && pitch <= maxPitch;
        return {
          pass,
          text: t('pv.pose', {
            yaw: yaw.toFixed(0),
            yawOp: sign(yaw <= maxYaw, '≤', '>'),
            maxYaw: maxYaw.toFixed(0),
            pitch: pitch.toFixed(0),
            pitchOp: sign(pitch <= maxPitch, '≤', '>'),
            maxPitch: maxPitch.toFixed(0),
          }),
        };
      }
      case 'eye_filter': {
        if (v.ear == null) return null;
        const min = num(p.eye_filter.min_ear);
        const pass = v.ear >= min;
        return { pass, text: t('pv.eyes', { v: v.ear.toFixed(2), op: sign(pass, '≥', '<'), min: min.toFixed(2) }) };
      }
    }
    return null;
  }

  let rows = $derived(
    r
      ? FILTERS.map((f) => {
          const verdict = judge(f.key, r.values);
          return { key: f.key, title: f.title, on: p[f.key].enabled, pass: verdict?.pass ?? null, text: verdict?.text ?? t('pv.notMeasured', { title: f.title }) };
        })
      : []
  );
  let failed = $derived(rows.filter((x) => x.on && x.pass === false));
  let busy = $derived(preview.loading || preview.loadingSamples);
</script>

<section class="pv" aria-label={t('pv.title')}>
  <div class="pvHead">
    <h3>{t('pv.title')}</h3>
    <button type="button" disabled={preview.samples.length < 2} onclick={() => pick(preview.index + 1 + Math.floor(Math.random() * (preview.samples.length - 1)))}>{t('pv.another')}</button>
  </div>

  {#if !selection.personId}
    <p class="muted">{t('pv.noPerson')}</p>
  {:else if preview.samplesError}
    <p class="err">{preview.samplesError}</p>
  {:else if preview.loadingSamples}
    <p class="muted">{t('pv.loadingPhotos')}</p>
  {:else if !preview.samples.length}
    <p class="muted">{t('pv.noPhotos')}</p>
  {:else}
    <div class="pvStrip" role="listbox" aria-label={t('pv.samples')}>
      {#each preview.samples as s, i (s.id)}
        <button
          type="button"
          role="option"
          aria-selected={i === preview.index}
          class:on={i === preview.index}
          title={s.timestamp ? s.timestamp.slice(0, 10) : ''}
          onclick={() => pick(i)}
        >
          <img loading="lazy" alt="" src="/api/albums/{encodeURIComponent(s.id)}/thumbnail" />
        </button>
      {/each}
    </div>
    <p class="muted pvCount">
      {t('pv.count', { n: preview.samples.length, total: fmt(preview.total) })}{sample?.timestamp ? t('pv.thisFrom', { date: sample.timestamp.slice(0, 10) }) : ''}
    </p>

    {#if preview.error}
      <p class="err mt">{preview.error}</p>
    {:else if !r}
      <p class="muted mt">{busy ? t('pv.measuring') : ''}</p>
    {:else}
      <div class="pvPair" class:pvBusy={preview.loading}>
        <figure>
          <div class="pvOrig">
            <img alt={t('pv.origAlt')} src={r.original} />
            <span
              class="pvBox"
              style="left:{r.face_box.x * 100}%;top:{r.face_box.y * 100}%;width:{r.face_box.w * 100}%;height:{r.face_box.h * 100}%"
            ></span>
          </div>
          <figcaption>{t('pv.photo')}</figcaption>
        </figure>
        <figure>
          {#if r.frame}
            <img class="pvFrame" alt={t('pv.frameAlt')} src={r.frame} />
          {:else}
            <div class="pvNoFrame">{t('pv.noFrame')}</div>
          {/if}
          <figcaption>{t('pv.frame')}</figcaption>
        </figure>
      </div>

      <ul class="pvList">
        {#each rows as f (f.key)}
          <li class:focus={f.key === focus} class:off={!f.on}>
            <span class="pvMark" class:ok={f.on && f.pass === true} class:bad={f.on && f.pass === false}>
              {f.pass === true ? '✓' : f.pass === false ? '✗' : '–'}
            </span>
            <span>{f.text}{f.on ? '' : t('pv.filterOff')}</span>
          </li>
        {/each}
      </ul>

      {#if r.problem}
        <p class="pvVerdict bad">{t('pv.skippedProblem')} {r.problem}</p>
      {:else if failed.length}
        <p class="pvVerdict bad">{t('pv.skippedBecause')} {failed.map((f) => f.title).join(', ')}</p>
      {:else}
        <p class="pvVerdict ok">{t('pv.used')}</p>
      {/if}
      <p class="muted pvTime">
        {t(r.cached ? 'pv.cached' : 'pv.downloaded', { ms: r.millis, all: r.roundtrip })}
      </p>
    {/if}
  {/if}
</section>
