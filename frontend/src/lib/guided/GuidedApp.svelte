<script>
  // Guided view: the process as boxes on the left, each step opens a panel on
  // the right (Mosaik pattern). Uses the same API, settings and localStorage
  // keys as the classic view, so both views stay in sync.
  import { onDestroy, tick } from 'svelte';
  import './guided.css';
  import ConnectionStatus from '../components/ConnectionStatus.svelte';
  import VersionBadge from '../components/VersionBadge.svelte';
  import GalleryView from '../components/GalleryView.svelte';
  import Box from './Box.svelte';
  import Pane from './Pane.svelte';
  import PeoplePane from './PeoplePane.svelte';
  import AlbumsPane from './AlbumsPane.svelte';
  import QualityPane from './QualityPane.svelte';
  import FilterDetail from './FilterDetail.svelte';
  import AdvancedPane from './AdvancedPane.svelte';
  import VideoPane from './VideoPane.svelte';
  import MusicPane from './MusicPane.svelte';
  import CreatePane from './CreatePane.svelte';
  import ResultPane from './ResultPane.svelte';
  import HistoryPane from './HistoryPane.svelte';
  import { toPayload, FILTERS, CODECS, POSITIONS } from './config.js';
  import { sanitizeFolderName } from '../utils.js';
  import { handleError } from '../errorHandler.js';
  import { STORAGE_KEYS, WS, JOB_STATUS, API } from '../constants.js';

  let { onswitch } = $props();

  // ---------- Process ----------
  const STEPS = [
    { box: 'person', pane: 'person', label: '1. Choose person' },
    { box: 'albums', pane: 'albums', label: '2. Filter albums' },
    { box: 'period', pane: null, label: '3. Time period' },
    { box: 'quality', pane: 'quality', label: '4. Photo selection and quality' },
    { box: 'video', pane: 'video', label: '5. Video and music' },
    { box: 'create', pane: 'create', label: '6. Create timelapse' },
    { box: 'result', pane: 'result', label: '7. Result' },
  ];
  const PANES = {
    history: 'Previous timelapses',
    ...Object.fromEntries(STEPS.filter((s) => s.pane).map((s) => [s.pane, s.label])),
  };
  const pane2Title = (k) =>
    k === 'adv' ? 'Advanced' : k === 'music' ? 'Music' : (FILTERS.find((f) => 'f:' + f.key === k)?.title ?? '');

  // ---------- Persisted selection (same keys as the classic view) ----------
  function readJson(key, fallback) {
    try {
      const raw = localStorage.getItem(key);
      return raw ? JSON.parse(raw) : fallback;
    } catch {
      return fallback;
    }
  }
  function writeJson(key, value) {
    try {
      if (value == null) localStorage.removeItem(key);
      else localStorage.setItem(key, JSON.stringify(value));
    } catch {
      // Ignore storage errors, the choice just isn't remembered
    }
  }

  const storedPerson = readJson(STORAGE_KEYS.selectedPerson, null);
  const storedAlbums = readJson(STORAGE_KEYS.selectedAlbums, []);

  let person = $state(storedPerson?.id ? { id: storedPerson.id, name: storedPerson.name } : null);
  let albums = $state(Array.isArray(storedAlbums) ? storedAlbums.filter((a) => a?.id) : []);
  let dateFrom = $state('');
  let dateTo = $state('');

  function selectPerson(p) {
    person = { id: p.id, name: p.name };
    writeJson(STORAGE_KEYS.selectedPerson, person);
  }
  function setAlbums(list) {
    albums = list.map((a) => ({ id: a.id, name: a.name }));
    writeJson(STORAGE_KEYS.selectedAlbums, albums.length ? albums : null);
  }

  // ---------- Panels ----------
  let pane = $state(null); // key of PANES
  let pane2 = $state(null); // 'f:<filter>' | 'adv' | 'music'
  let right = $state(false); // gallery of the used photos
  let closed = $state(new Set(readJson(STORAGE_KEYS.guidedClosedBoxes, [])));

  function toggleBox(key) {
    const next = new Set(closed);
    next.has(key) ? next.delete(key) : next.add(key);
    closed = next;
    writeJson(STORAGE_KEYS.guidedClosedBoxes, [...next]);
  }

  // Whoever closes a panel also closes everything to the right of it
  function closeRightOf(level) {
    const keep = ['left', 'pane', 'pane2', 'right'].indexOf(level);
    if (keep < 1) pane = null;
    if (keep < 2) pane2 = null;
    if (keep < 3 && right) {
      right = false;
      loadOutputFolders(); // the gallery may have removed photos
    }
  }
  function openPane(name) {
    if (pane === name) return closeRightOf('left');
    closeRightOf('left');
    pane = name;
  }
  function openPane2(key) {
    closeRightOf('pane');
    pane2 = pane2 === key ? null : key;
  }
  function togglePhotos() {
    if (right) closeRightOf('pane');
    else {
      pane2 = null;
      right = true;
    }
  }
  async function goStep(i) {
    const step = STEPS[i];
    if (step.pane) return openPane(step.pane);
    closeRightOf('left'); // step without a panel: open its box on the left
    if (closed.has(step.box)) toggleBox(step.box);
    await tick();
    document.getElementById(`g-box-${step.box}`)?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }
  function stepNav(paneKey) {
    const i = STEPS.findIndex((s) => s.pane === paneKey);
    if (i < 0) return [];
    return [i - 1, i + 1]
      .filter((j) => STEPS[j] && !stepLocked(STEPS[j].box))
      .map((j) => ({ label: STEPS[j].label, onclick: () => goStep(j) }));
  }
  function stepLocked(box) {
    if (box === 'person') return false;
    if (box === 'result') return !resultFolder;
    return !person;
  }

  // Columns: show as many columns from the right as fit (fitColumns from Mosaik)
  let pageW = $state(1200);
  // ===== Theme (like Mosaik: index.html sets it before render, the button switches and saves it) =====
  let dark = $state(document.documentElement.dataset.theme !== 'light');
  function toggleTheme() {
    dark = !dark;
    localStorage.setItem(STORAGE_KEYS.theme, dark ? 'dark' : 'light');
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  }

  let innerWidth = $state(1200);
  let narrow = $derived(innerWidth <= 860);
  let stack = $derived.by(() => {
    const s = [{ id: 'left', label: 'Process' }];
    if (pane) s.push({ id: 'pane', label: PANES[pane] });
    if (pane2) s.push({ id: 'pane2', label: pane2Title(pane2) });
    if (right) s.push({ id: 'right', label: 'Used photos' });
    return s;
  });
  let fit = $derived.by(() => {
    const ids = stack.map((p) => p.id);
    const leftW = Math.round(Math.min(380, Math.max(300, pageW * 0.32)));
    const width = { left: leftW, pane: 520, pane2: 480, right: 640 };
    const track = { left: `${leftW}px`, pane: 'minmax(0,520px)', pane2: 'minmax(0,480px)', right: 'minmax(0,1fr)' };
    let used = 40 + width[ids.at(-1)];
    let first = ids.length - 1;
    while (first > 0 && used + 16 + width[ids[first - 1]] <= pageW) {
      first--;
      used += 16 + width[ids[first]];
    }
    const cols = ids.slice(first).map((id) => track[id]);
    if (cols.length === 1) cols.push('minmax(0,1fr)');
    return { ids, first, cols: cols.join(' ') };
  });
  const off = (id) => !narrow && fit.ids.indexOf(id) > -1 && fit.ids.indexOf(id) < fit.first;

  function onKeydown(e) {
    if (e.key !== 'Escape' || stack.length < 2) return;
    if (document.querySelector('.lightbox-overlay')) return; // the gallery closes its own lightbox
    closeRightOf(stack.at(-2).id);
  }

  // ---------- Data from the server ----------
  let connected = $state(false);
  let people = $state([]);
  let peopleState = $state({ loading: false, error: null });
  let albumList = $state([]);
  let albumState = $state({ loading: false, error: null });
  let outputFolders = $state([]);
  let music = $state({ music_dir: '', available: false, tracks: [] });
  let musicState = $state({ loading: false, error: null });

  async function getJson(url) {
    const res = await fetch(url);
    if (!res.ok) throw res;
    return res.json();
  }

  async function loadPeople() {
    peopleState = { loading: true, error: null };
    try {
      people = await getJson(API.people);
      // Keep the stored name in sync with Immich
      const found = person && people.find((p) => p.id === person.id);
      if (found && found.name !== person.name) selectPerson(found);
      peopleState = { loading: false, error: null };
    } catch (e) {
      peopleState = { loading: false, error: await handleError('Failed to load people', e) };
    }
  }
  async function loadAlbums() {
    albumState = { loading: true, error: null };
    try {
      albumList = await getJson(API.albums);
      albumState = { loading: false, error: null };
    } catch (e) {
      albumState = { loading: false, error: await handleError('Failed to load albums', e) };
    }
  }
  async function loadOutputFolders() {
    try {
      outputFolders = await getJson(API.output);
    } catch (e) {
      console.error('Failed to load output folders:', e);
    }
  }
  async function loadMusic() {
    musicState = { loading: true, error: null };
    try {
      music = await getJson(API.music);
      musicState = { loading: false, error: null };
    } catch (e) {
      musicState = { loading: false, error: await handleError('Failed to load music', e) };
    }
  }

  // Number of photos of the person (in the chosen albums)
  let assetCount = $state(null);
  let loadingCount = $state(false);
  let countSeq = 0;
  $effect(() => {
    const id = person?.id;
    const ids = albums.map((a) => a.id);
    if (!connected || !id) {
      assetCount = null;
      return;
    }
    fetchCount(id, ids);
  });
  async function fetchCount(id, ids) {
    const seq = ++countSeq;
    loadingCount = true;
    assetCount = null;
    try {
      let url = `${API.people}/${encodeURIComponent(id)}/asset-count`;
      if (ids.length) url += `?album_ids=${ids.map(encodeURIComponent).join(',')}`;
      const data = await getJson(url);
      if (seq === countSeq) assetCount = data;
    } catch (e) {
      if (seq === countSeq) await handleError('Failed to fetch asset count', e);
    } finally {
      if (seq === countSeq) loadingCount = false;
    }
  }

  // ---------- Settings: loaded once, saved automatically ----------
  let config = $state(null);
  let configError = $state(null);
  let saveState = $state(''); // '' | 'pending' | 'saving' | 'saved' | error text
  let lastSaved = '';

  async function loadConfig() {
    configError = null;
    try {
      const data = await getJson(API.config);
      lastSaved = JSON.stringify(toPayload(data));
      config = data;
    } catch (e) {
      configError = await handleError('Failed to load settings', e);
    }
  }
  $effect(() => {
    if (!config) return;
    const json = JSON.stringify(toPayload(config));
    if (json === lastSaved) return;
    saveState = 'pending';
    const t = setTimeout(() => saveConfig(json), 600);
    return () => clearTimeout(t);
  });
  async function saveConfig(json) {
    saveState = 'saving';
    try {
      const res = await fetch(API.config, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: json });
      if (!res.ok) throw res;
      lastSaved = json;
      saveState = 'saved';
    } catch (e) {
      saveState = await handleError('Failed to save settings', e);
    }
  }
  async function resetToDefaults() {
    if (!confirm('Reset all settings to the defaults?')) return;
    try {
      config = await getJson(API.configDefaults);
    } catch (e) {
      saveState = await handleError('Failed to load defaults', e);
    }
  }
  let saveText = $derived(
    { '': '', pending: 'Settings changed...', saving: 'Saving settings...', saved: 'Settings saved' }[saveState] ?? saveState
  );
  let saveFailed = $derived(!['', 'pending', 'saving', 'saved'].includes(saveState));

  // ---------- Job, WebSocket (same logic as the classic view) ----------
  let jobStatus = $state(JOB_STATUS.idle);
  let progress = $state({ completed: 0, total: 0, message: '' });
  let starting = $state(false);
  let historyFolder = $state(null); // folder opened from the history
  let resultStamp = $state(Date.now());
  let ws = null;
  let wsReconnectTimeout = null;

  let busy = $derived(
    jobStatus === JOB_STATUS.running || jobStatus === JOB_STATUS.compiling || jobStatus === JOB_STATUS.cancelling
  );
  let completedFolderName = $derived(
    progress.output_folder ??
      (progress.person_name || progress.person_id ? sanitizeFolderName(progress.person_name, progress.person_id) : null)
  );
  let resultFolder = $derived(historyFolder ?? (jobStatus === JOB_STATUS.completed ? completedFolderName : null));
  let resultInfo = $derived(outputFolders.find((f) => f.name === resultFolder) ?? null);

  function connectWebSocket() {
    if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) return;
    ws = new WebSocket(WS.getUrl());
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const previous = jobStatus;
        const active =
          data.status === JOB_STATUS.running || data.status === JOB_STATUS.compiling || data.status === JOB_STATUS.cancelling;
        if (active || jobStatus !== JOB_STATUS.idle) {
          jobStatus = data.status;
          progress = data;
        }
        const wasActive =
          previous === JOB_STATUS.running || previous === JOB_STATUS.compiling || previous === JOB_STATUS.cancelling;
        const finished =
          data.status === JOB_STATUS.completed || data.status === JOB_STATUS.cancelled || data.status === JOB_STATUS.error;
        if (finished && wasActive) {
          loadOutputFolders();
          resultStamp = Date.now();
        }
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    };
    ws.onclose = () => {
      ws = null;
      if (connected) scheduleReconnect();
    };
    ws.onerror = (error) => console.error('WebSocket error:', error);
  }
  function scheduleReconnect() {
    if (wsReconnectTimeout) return;
    wsReconnectTimeout = setTimeout(() => {
      wsReconnectTimeout = null;
      if (connected) connectWebSocket();
    }, WS.reconnectDelay);
  }
  onDestroy(() => {
    if (wsReconnectTimeout) clearTimeout(wsReconnectTimeout);
    wsReconnectTimeout = null;
    connected = false;
    ws?.close();
    ws = null;
  });

  function handleConnectionChange(data) {
    const was = connected;
    connected = data.connected;
    if (connected) {
      connectWebSocket();
      if (!was) {
        loadPeople();
        loadAlbums();
        loadOutputFolders();
        loadMusic();
        if (!config) loadConfig();
      }
    }
  }

  async function startProcessing() {
    if (!person) return;
    // Each run creates a new versioned output folder; earlier runs are kept.
    starting = true;
    try {
      const res = await fetch(API.start, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          person_id: person.id,
          person_name: person.name || null,
          date_from: dateFrom || null,
          date_to: dateTo || null,
          album_ids: albums.map((a) => a.id),
          album_names: albums.map((a) => a.name),
        }),
      });
      const data = await res.json();
      historyFolder = null;
      if (res.ok && data.success) {
        jobStatus = JOB_STATUS.running;
        progress = { status: JOB_STATUS.running, completed: 0, total: 0, message: 'Starting...' };
      } else {
        jobStatus = JOB_STATUS.error;
        progress = { status: JOB_STATUS.error, completed: 0, total: 0, message: data.message || 'Failed to start processing' };
      }
    } catch (e) {
      const message = await handleError('Failed to start processing', e);
      jobStatus = JOB_STATUS.error;
      progress = { status: JOB_STATUS.error, completed: 0, total: 0, message };
    } finally {
      starting = false;
    }
  }

  function openHistory(name) {
    historyFolder = name;
    resultStamp = Date.now();
    closeRightOf('left');
    pane = 'result';
  }
  function handleFolderDeleted(name) {
    if (jobStatus === JOB_STATUS.completed && (name === null || name === completedFolderName)) {
      jobStatus = JOB_STATUS.idle;
      progress = { completed: 0, total: 0, message: '' };
    }
    if (historyFolder && (name === null || name === historyFolder)) historyFolder = null;
    loadOutputFolders();
  }
  // The result panel has nothing to show any more
  $effect(() => {
    if (!resultFolder && pane === 'result') closeRightOf('left');
  });

  function newTimelapse() {
    closeRightOf('left');
    historyFolder = null;
    if (!busy && jobStatus !== JOB_STATUS.idle) {
      jobStatus = JOB_STATUS.idle;
      progress = { completed: 0, total: 0, message: '' };
    }
    pane = 'person';
  }

  // ---------- Texts for the boxes on the left ----------
  let lock = $derived(!person);
  let activeFilters = $derived(config ? FILTERS.filter((f) => config.processing[f.key].enabled) : []);
  let periodSummary = $derived.by(() => {
    let s = `${dateFrom || 'From the start'} to ${dateTo || 'today'}`;
    const ti = config?.processing.time_interval;
    if (ti?.enabled) s += `, at most ${ti.max_photos} per ${ti.time_range}`;
    return s;
  });
  let photoLine = $derived(
    loadingCount ? 'Counting photos...' : assetCount ? `${assetCount.assets_with_faces} photos of ${person?.name || 'Unnamed'}` : ''
  );
</script>

<svelte:window bind:innerWidth onkeydown={onKeydown} />

<div class="guided">
  <header>
    <h1>Immich Selfie Timelapse</h1>
    <VersionBadge />
    {#if stack.length > 1}
      <nav class="crumbs" aria-label="Path">
        {#each stack as p, i (p.id)}
          {#if i === stack.length - 1}
            <span class="now">{p.label}</span>
          {:else}
            <button type="button" onclick={() => closeRightOf(p.id)}>{p.label}</button><span class="sep">&gt;</span>
          {/if}
        {/each}
      </nav>
    {/if}
    <ConnectionStatus onchange={handleConnectionChange} />
    <button
      type="button"
      class="themeBtn"
      title={dark ? 'Light mode' : 'Dark mode'}
      aria-label={dark ? 'Light mode' : 'Dark mode'}
      onclick={toggleTheme}>{dark ? '☀' : '☾'}</button
    >
  </header>

  {#if !connected}
    <div class="card notConnected">
      <p>Connect to your Immich server to get started.</p>
      <p class="muted">Make sure the backend is running and configured with your Immich API credentials.</p>
      <p class="muted">
        You need an Immich API key with the following permissions:
        <code>asset.download</code>, <code>asset.read</code>, <code>asset.view</code>, <code>person.read</code>,
        <code>album.read</code>, <code>server.about</code>.
        See the
        <a href="https://immich.app/docs/features/command-line-interface#obtain-the-api-key" target="_blank" rel="noopener noreferrer">Immich docs</a>
        to create one.
      </p>
      <p class="mt"><button type="button" onclick={() => onswitch('classic')}>Back to the classic view</button></p>
    </div>
  {:else}
    <main bind:clientWidth={pageW} style:grid-template-columns={narrow ? null : fit.cols}>
      <!-- Left: the process -->
      <div class="col" class:offLeft={off('left')}>
        <section class="card kind">
          <h2>View</h2>
          <div class="seg">
            <button type="button" onclick={() => onswitch('classic')} aria-pressed="false">Classic</button>
            <button type="button" class="on" aria-pressed="true">Guided</button>
          </div>
          <div class="btnRow mt"><button type="button" onclick={newTimelapse}>New timelapse</button></div>
          {#if configError}<p class="err mt">{configError}</p>{/if}
          {#if saveText}<p class="mt" class:muted={!saveFailed} class:err={saveFailed}>{saveText}</p>{/if}
        </section>

        <div id="g-box-history">
          <Box title="Previous timelapses" paneOpen={pane === 'history'} onopen={() => openPane('history')}>
            <p class="muted">
              {outputFolders.length === 0 ? 'None yet' : `${outputFolders.length} timelapse${outputFolders.length === 1 ? '' : 's'}`}
            </p>
          </Box>
        </div>

        <div id="g-box-person">
          <Box title={STEPS[0].label} paneOpen={pane === 'person'} onopen={() => openPane('person')}>
            {#if person}
              <div class="sumPerson">
                <span class="avatar sm">
                  {(person.name || '?').charAt(0).toUpperCase()}
                  <img src="/api/people/{encodeURIComponent(person.id)}/thumbnail" alt="" onerror={(e) => e.currentTarget.remove()} />
                </span>
                <span><strong>{person.name || 'Unnamed'}</strong><br /><span class="muted">{photoLine}</span></span>
              </div>
            {:else}
              <p class="muted">Nobody chosen yet. Open the list with &gt;</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-albums">
          <Box
            title={STEPS[1].label}
            toggle
            closed={closed.has('albums')}
            ontoggle={() => toggleBox('albums')}
            locked={lock}
            paneOpen={pane === 'albums'}
            onopen={() => openPane('albums')}
            summary={albums.length === 0 ? 'All photos' : `${albums.length} album${albums.length === 1 ? '' : 's'}`}
          >
            <fieldset disabled={lock || busy}>
              <label class="check" style="margin-top:0">
                <input
                  type="checkbox"
                  checked={albums.length === 0}
                  onchange={(e) => {
                    if (e.currentTarget.checked) setAlbums([]);
                    else {
                      e.currentTarget.checked = true; // choosing albums happens in the panel
                      if (pane !== 'albums') openPane('albums');
                    }
                  }}
                />
                All photos
              </label>
              {#if albums.length > 0}
                <p class="lbl">Chosen albums</p>
                {#each albums as a (a.id)}
                  <div class="pickedRow">
                    <span>{a.name}</span>
                    <button type="button" title="Remove" aria-label="Remove {a.name}" onclick={() => setAlbums(albums.filter((x) => x.id !== a.id))}>×</button>
                  </div>
                {/each}
              {/if}
              {#if photoLine}<p class="mt">{photoLine}</p>{/if}
            </fieldset>
          </Box>
        </div>

        <div id="g-box-period">
          <Box
            title={STEPS[2].label}
            toggle
            closed={closed.has('period')}
            ontoggle={() => toggleBox('period')}
            locked={lock}
            summary={periodSummary}
          >
            <fieldset disabled={lock || busy}>
              <div class="row">
                <div><label for="g-from" style="margin-top:0">From</label><input id="g-from" type="date" bind:value={dateFrom} /></div>
                <div><label for="g-to" style="margin-top:0">To</label><input id="g-to" type="date" bind:value={dateTo} /></div>
              </div>
              {#if config}
                <label class="check mt">
                  <input type="checkbox" bind:checked={config.processing.time_interval.enabled} /> Limit the number of photos
                </label>
                {#if config.processing.time_interval.enabled}
                  <div class="inline mt">
                    <span>At most</span>
                    <input type="number" min="1" max="10" aria-label="Photos" bind:value={config.processing.time_interval.max_photos} />
                    <span>per</span>
                    <div class="toggles">
                      {#each ['day', 'week', 'month'] as r (r)}
                        <button
                          type="button"
                          class:on={config.processing.time_interval.time_range === r}
                          aria-pressed={config.processing.time_interval.time_range === r}
                          onclick={() => (config.processing.time_interval.time_range = r)}>{r}</button
                        >
                      {/each}
                    </div>
                  </div>
                {/if}
              {/if}
            </fieldset>
          </Box>
        </div>

        <div id="g-box-quality">
          <Box title={STEPS[3].label} locked={lock || !config} paneOpen={pane === 'quality'} onopen={() => openPane('quality')}>
            {#if config}
              <p>{activeFilters.length} of {FILTERS.length} filters active</p>
              <p class="muted">{activeFilters.map((f) => f.title).join(', ') || 'Every photo is used'}</p>
            {:else}
              <p class="muted">Loading settings...</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-video">
          <Box title={STEPS[4].label} locked={lock || !config} paneOpen={pane === 'video'} onopen={() => openPane('video')}>
            {#if config}
              {@const p = config.processing}
              {@const v = config.video}
              <p class="muted">
                {p.output.size} × {p.output.size} px,
                {v.fit_to_music && (v.music_tracks ?? []).length ? 'fps fit to music' : `${v.framerate} fps`},
                {CODECS[v.codec] ?? v.codec}<br />
                {p.timestamp.enabled ? `Date ${(POSITIONS[p.timestamp.position] ?? '').toLowerCase()}` : 'No date'}
                · {(v.music_tracks ?? []).length ? `Music: ${v.music_tracks.length} track${v.music_tracks.length === 1 ? '' : 's'}` : 'No music'}
              </p>
            {:else}
              <p class="muted">Loading settings...</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-create">
          <Box title={STEPS[5].label} locked={lock} paneOpen={pane === 'create'} onopen={() => openPane('create')}>
            {#if busy}
              <p class="muted">{progress.message || 'Running'}{progress.total ? `: ${progress.completed} of ${progress.total}` : ''}</p>
              <progress max={progress.total || 1} value={progress.total ? progress.completed : 0}></progress>
            {:else if jobStatus === JOB_STATUS.error}
              <p class="err">{progress.message || 'The last run failed'}</p>
            {:else if !person}
              <p class="muted">Choose a person first</p>
            {:else}
              <p class="muted">{photoLine || person.name} · ready</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-result">
          <Box title={STEPS[6].label} locked={!resultFolder} paneOpen={pane === 'result'} onopen={() => openPane('result')}>
            <p class="muted">
              {#if !resultFolder}
                No timelapse yet
              {:else if historyFolder}
                Opened: {resultFolder}
              {:else}
                Finished: {resultFolder}
              {/if}
            </p>
          </Box>
        </div>
      </div>

      <!-- Level 1: the panel of a step -->
      {#if pane}
        <Pane title={PANES[pane]} level={1} offLeft={off('pane')} nav={stepNav(pane)} onclose={() => closeRightOf('left')}>
          {#if pane === 'history'}
            <HistoryPane folders={outputFolders} current={resultFolder} disabled={busy} onopen={openHistory} ondeleted={handleFolderDeleted} />
          {:else if pane === 'person'}
            <PeoplePane
              {people}
              loading={peopleState.loading}
              error={peopleState.error}
              selectedId={person?.id}
              disabled={busy}
              onselect={selectPerson}
            />
          {:else if pane === 'albums'}
            <AlbumsPane
              albums={albumList}
              loading={albumState.loading}
              error={albumState.error}
              selected={albums}
              personName={person?.name}
              disabled={busy}
              onchange={setAlbums}
            />
          {:else if pane === 'quality' && config}
            <fieldset disabled={busy}><QualityPane bind:config open2={pane2} onopen={openPane2} /></fieldset>
          {:else if pane === 'video' && config}
            <fieldset disabled={busy}>
              <VideoPane bind:config musicOpen={pane2 === 'music'} onmusic={() => openPane2('music')} />
            </fieldset>
          {:else if pane === 'create'}
            <CreatePane
              {person}
              {albums}
              {dateFrom}
              {dateTo}
              {config}
              {assetCount}
              {loadingCount}
              {jobStatus}
              {progress}
              {busy}
              {starting}
              onstart={startProcessing}
              onresult={() => goStep(6)}
            />
          {:else if pane === 'result' && resultFolder}
            <ResultPane folderName={resultFolder} folder={resultInfo} stamp={resultStamp} photosOpen={right} onphotos={togglePhotos} />
          {:else}
            <p class="muted">Loading...</p>
          {/if}
        </Pane>
      {/if}

      <!-- Level 2: details of a setting -->
      {#if pane2 && config}
        <Pane title={pane2Title(pane2)} level={2} offLeft={off('pane2')} onclose={() => closeRightOf('pane')}>
          <fieldset disabled={busy}>
            {#if pane2 === 'adv'}
              <AdvancedPane bind:config onreset={resetToDefaults} />
            {:else if pane2 === 'music'}
              <MusicPane bind:config {music} loading={musicState.loading} error={musicState.error} onrefresh={loadMusic} />
            {:else}
              <FilterDetail bind:config key={pane2.slice(2)} />
            {/if}
          </fieldset>
        </Pane>
      {/if}

      <!-- Level 3: the used photos -->
      {#if right && resultFolder}
        <Pane title="Used photos" level={3} wide offLeft={off('right')} onclose={() => closeRightOf('pane')}>
          {#key resultFolder}
            <GalleryView folderName={resultFolder} onBack={() => closeRightOf('pane')} disabled={busy} />
          {/key}
        </Pane>
      {/if}
    </main>
  {/if}
</div>
