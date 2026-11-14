<script>
  // Guided view: the process as boxes on the left, each step opens a panel on
  // the right (Mosaik pattern). Uses the same API, settings and localStorage
  // keys as the classic view, so both views stay in sync.
  import { onDestroy, onMount, tick } from 'svelte';
  import './guided.css';
  import AboutPane from './AboutPane.svelte';
  import ChangelogPane from './ChangelogPane.svelte';
  import SettingsPane from './SettingsPane.svelte';
  import PeriodPane from './PeriodPane.svelte';
  import GalleryView from '../components/GalleryView.svelte';
  import Box from './Box.svelte';
  import Pane from './Pane.svelte';
  import PeoplePane from './PeoplePane.svelte';
  import AlbumsPane from './AlbumsPane.svelte';
  import QualityPane from './QualityPane.svelte';
  import FilterDetail from './FilterDetail.svelte';
  import AdvancedPane from './AdvancedPane.svelte';
  import FormatPane from './FormatPane.svelte';
  import DatePane from './DatePane.svelte';
  import VideoPane from './VideoPane.svelte';
  import MusicPane from './MusicPane.svelte';
  import TitlesPane from './TitlesPane.svelte';
  import TitleCardSummary from './TitleCardSummary.svelte';
  import AlbumPhotosPane from './AlbumPhotosPane.svelte';
  import { requestPreview, previewBody } from './titlePreview.svelte.js';
  import ImagePickerPane from './ImagePickerPane.svelte';
  import CreatePane from './CreatePane.svelte';
  import ResultPane from './ResultPane.svelte';
  import SharePane from './SharePane.svelte';
  import { loadShares } from './shares.svelte.js';
  import HistoryPane from './HistoryPane.svelte';
  import HistoryDetail from './HistoryDetail.svelte';
  import ReviewPane from './ReviewPane.svelte';
  import { toPayload, FILTERS, CODECS, POSITIONS, frameSize, formatDuration } from './config.js';
  import { sanitizeFolderName, formatFolderVersion } from '../utils.js';
  import { handleError, showErrorAlert } from '../errorHandler.js';
  import { STORAGE_KEYS, WS, JOB_STATUS, API } from '../constants.js';
  import { t, i18n, setLang, LANGS, locale, fmt } from './i18n.svelte.js';

  let { onswitch } = $props();

  // ---------- Process ----------
  // Labels are getters so they follow the language
  const STEPS = ['person', 'albums', 'period', 'quality', 'format', 'video', 'intro', 'outro', 'create', 'result'].map((key, i) => ({
    box: key,
    pane: key,
    get label() {
      return `${i + 1}. ${t('step.' + key)}`;
    },
  }));
  const paneTitle = (k) =>
    k === 'history' ? t('history.title') : k === 'about' ? t('about.title') : k === 'settings' ? t('settings.title') : (STEPS.find((s) => s.pane === k)?.label ?? '');
  const pane2Title = (k) =>
    ({ adv: t('adv.title'), changelog: t('changelog.title'), music: t('music.title'), date: t('date.title'), pick: t('pick.title.' + titleWhich) })[k] ??
    FILTERS.find((f) => 'f:' + f.key === k)?.title ??
    '';

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
  const storedSources = readJson(STORAGE_KEYS.selectedSources, []);

  let person = $state(storedPerson?.id ? { id: storedPerson.id, name: storedPerson.name } : null);
  let albums = $state(Array.isArray(storedAlbums) ? storedAlbums.filter((a) => a?.id) : []);
  // Other sources besides albums: people in the photo and people albums are shared with
  let others = $state(
    Array.isArray(storedSources) ? storedSources.filter((o) => o?.id && (o.kind === 'person' || o.kind === 'recipient')) : []
  );
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
  let picked = $derived([...albums.map((a) => ({ ...a, kind: 'album' })), ...others]);
  // The selection the quality preview takes its sample photos from
  let selection = $derived({
    personId: connected ? person?.id || '' : '',
    albumIds: albums.map((a) => a.id),
    peopleIds: others.filter((o) => o.kind === 'person').map((o) => o.id),
    recipientIds: others.filter((o) => o.kind === 'recipient').map((o) => o.id),
    dateFrom,
    dateTo,
  });
  function setPicked(list) {
    setAlbums(list.filter((p) => p.kind === 'album'));
    others = list.filter((p) => p.kind !== 'album').map((p) => ({ id: p.id, kind: p.kind, name: p.name }));
    writeJson(STORAGE_KEYS.selectedSources, others.length ? others : null);
  }
  const pickedText = (p) => `${p.name} (${t('kind.' + p.kind)})`;

  // ---------- Panels ----------
  let pane = $state(null); // 'history' or the pane of a step
  let pane2 = $state(null); // 'f:<filter>' | 'adv' | 'music' | 'date' | 'pick'
  let titleWhich = $derived(pane === 'outro' ? 'outro' : 'intro'); // card whose image is picked in 'pick'
  let pickAlbum = $state(null); // level 3: album opened in the image picker
  function openPickAlbum(a) {
    pickAlbum = pickAlbum?.id === a.id ? null : a;
  }
  let right = $state(false); // gallery of the used photos
  let shareOpen = $state(false); // share panel right of the result
  let histDetail = $state(null); // level 2: folder whose details are open from the history
  let reviewOpen = $state(false); // level 2: pick the prepared photos before the video
  let closed = $state(new Set(readJson(STORAGE_KEYS.guidedClosedBoxes, [])));

  function toggleBox(key) {
    const next = new Set(closed);
    next.has(key) ? next.delete(key) : next.add(key);
    closed = next;
    writeJson(STORAGE_KEYS.guidedClosedBoxes, [...next]);
  }

  // Whoever closes a panel also closes everything to the right of it
  function closeRightOf(level) {
    const level2 = level === 'hist' || level === 'review';
    const keep = ['left', 'pane', 'pane2', 'right'].indexOf(level === 'album' ? 'right' : level2 ? 'pane2' : level);
    if (keep < 1) pane = null;
    if (keep < 2) pane2 = null;
    if (keep < 2) histDetail = null;
    if (keep < 2) reviewOpen = false;
    if (keep < 3) pickAlbum = null;
    if (keep < 3) shareOpen = false;
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
  function openHistDetail(name) {
    const same = histDetail === name;
    closeRightOf('pane');
    if (!same) histDetail = name;
  }
  function toggleReview() {
    const open = reviewOpen;
    closeRightOf('pane');
    reviewOpen = !open;
  }
  function togglePhotos() {
    if (right) closeRightOf('pane');
    else {
      closeRightOf('pane');
      right = true;
    }
  }
  function toggleShare() {
    if (shareOpen) closeRightOf('pane');
    else {
      closeRightOf('pane');
      shareOpen = true;
      loadShares();
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
    const s = [{ id: 'left', label: t('crumb.process') }];
    if (pane) s.push({ id: 'pane', label: paneTitle(pane) });
    if (pane2) s.push({ id: 'pane2', label: pane2Title(pane2) });
    if (histDetail) s.push({ id: 'hist', label: histTitle });
    if (reviewOpen) s.push({ id: 'review', label: t('review.title') });
    if (right) s.push({ id: 'right', label: t('common.usedPhotos') });
    if (shareOpen) s.push({ id: 'share', label: t('share.title') });
    if (pane2 === 'pick' && pickAlbum) s.push({ id: 'album', label: pickAlbum.name });
    return s;
  });
  let fit = $derived.by(() => {
    const ids = stack.map((p) => p.id);
    const leftW = Math.round(Math.min(380, Math.max(300, pageW * 0.32)));
    const width = { left: leftW, pane: 520, pane2: 480, right: 640, album: 560, share: 480, hist: 480, review: 640 };
    const track = { left: `${leftW}px`, pane: 'minmax(0,520px)', pane2: 'minmax(0,480px)', right: 'minmax(0,1fr)', album: 'minmax(0,1fr)', share: 'minmax(0,480px)', hist: 'minmax(0,480px)', review: 'minmax(0,1fr)' };
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
  let recipients = $state(null); // null = Smart Albums not configured
  let recipientsError = $state('');
  let coCounts = $state(null); // photos showing the person and each other person
  let coCountsFor = null;
  // Named people besides the chosen person, for "People" in the source picker
  let otherPeople = $derived(people.filter((p) => p.name && p.id !== person?.id));
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
      peopleState = { loading: false, error: await handleError(t('err.people'), e) };
    }
  }
  async function loadAlbums() {
    albumState = { loading: true, error: null };
    try {
      albumList = await getJson(API.albums);
      albumState = { loading: false, error: null };
    } catch (e) {
      albumState = { loading: false, error: await handleError(t('err.albums'), e) };
    }
  }
  async function loadRecipients() {
    try {
      const data = await getJson(API.recipients);
      recipients = data.recipients;
      recipientsError = data.recipients_error || '';
    } catch (e) {
      console.error('Failed to load Smart Albums people:', e);
      recipients = null;
    }
  }
  async function loadCoCounts() {
    const id = person?.id;
    if (!id || coCountsFor === id) return;
    coCountsFor = id;
    coCounts = null;
    try {
      const data = await getJson(`${API.people}/${encodeURIComponent(id)}/co-counts`);
      if (coCountsFor === id) coCounts = data;
    } catch {
      if (coCountsFor === id) coCounts = {};
    }
  }
  // Counts belong to the chosen person
  $effect(() => {
    if (person?.id !== coCountsFor) {
      coCountsFor = null;
      coCounts = null;
    }
  });
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
      musicState = { loading: false, error: await handleError(t('err.music'), e) };
    }
  }

  // Number of photos of the person (in the chosen albums)
  let assetCount = $state(null);
  let loadingCount = $state(false);
  let countSeq = 0;
  $effect(() => {
    const id = person?.id;
    const ids = albums.map((a) => a.id);
    const peopleIds = others.filter((o) => o.kind === 'person').map((o) => o.id);
    const recipientIds = others.filter((o) => o.kind === 'recipient').map((o) => o.id);
    if (!connected || !id) {
      assetCount = null;
      return;
    }
    fetchCount(id, ids, peopleIds, recipientIds);
  });
  async function fetchCount(id, ids, peopleIds, recipientIds) {
    const seq = ++countSeq;
    loadingCount = true;
    assetCount = null;
    try {
      let url = `${API.people}/${encodeURIComponent(id)}/asset-count`;
      const params = [
        ['album_ids', ids],
        ['people_ids', peopleIds],
        ['recipient_ids', recipientIds],
      ].filter(([, list]) => list.length);
      if (params.length) url += '?' + params.map(([k, list]) => `${k}=${list.map(encodeURIComponent).join(',')}`).join('&');
      const data = await getJson(url);
      if (seq === countSeq) assetCount = data;
    } catch (e) {
      if (seq === countSeq) await handleError(t('err.count'), e);
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
      configError = await handleError(t('err.loadSettings'), e);
    }
  }
  $effect(() => {
    if (!config) return;
    const json = JSON.stringify(toPayload(config));
    if (json === lastSaved) return;
    saveState = 'pending';
    const timer = setTimeout(() => saveConfig(json), 600);
    return () => clearTimeout(timer);
  });
  async function saveConfig(json) {
    saveState = 'saving';
    try {
      const res = await fetch(API.config, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body: json });
      if (!res.ok) throw res;
      lastSaved = json;
      saveState = 'saved';
    } catch (e) {
      saveState = await handleError(t('err.saveSettings'), e);
    }
  }
  // Global settings (Settings panel) survive a reset
  function keepGlobal(defaults) {
    if (config) {
      defaults.processing.output.keep_versions = config.processing.output.keep_versions;
      defaults.processing.max_workers = config.processing.max_workers;
    }
    return defaults;
  }
  async function resetToDefaults() {
    if (!confirm(t('confirm.reset'))) return;
    try {
      config = keepGlobal(await getJson(API.configDefaults));
    } catch (e) {
      saveState = await handleError(t('err.defaults'), e);
    }
  }
  let saveText = $derived(
    { '': '', pending: t('save.pending'), saving: t('save.saving'), saved: t('save.saved') }[saveState] ?? saveState
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
  // The run on the server, also one started from another browser: shown on
  // top and in step 9 even when no person is picked here
  let jobActive = $derived(busy || !!progress.awaiting_review);
  let jobName = $derived(progress.person_name || progress.person_id || t('common.unnamed'));
  let stopping = $state(false);
  async function cancelJob() {
    if (!confirm(t('job.confirmCancel', { name: jobName }))) return;
    stopping = true;
    try {
      const res = await fetch(API.cancel, { method: 'POST' });
      if (!res.ok) throw res;
    } catch (e) {
      await showErrorAlert(t('job.errCancel'), e);
    } finally {
      stopping = false;
    }
  }
  function showJob() {
    if (pane !== 'create') openPane('create');
  }
  let completedFolderName = $derived(
    progress.output_folder ??
      (progress.person_name || progress.person_id ? sanitizeFolderName(progress.person_name, progress.person_id) : null)
  );
  // A run that waits for the photo pick has no video yet
  let resultFolder = $derived(
    historyFolder ?? (jobStatus === JOB_STATUS.completed && !progress.awaiting_review ? completedFolderName : null)
  );
  let histInfo = $derived(outputFolders.find((f) => f.name === histDetail) ?? null);
  let histTitle = $derived(
    histInfo ? `${histInfo.display_name ?? histInfo.person ?? histInfo.name}${histInfo.version ? ' · ' + formatFolderVersion(histInfo.version) : ''}` : (histDetail ?? '')
  );
  let resultInfo = $derived(outputFolders.find((f) => f.name === resultFolder) ?? null);
  // Title suggested for a new link: the person's name when the result belongs to them
  let shareTitle = $derived.by(() => {
    const p = resultInfo?.person ?? '';
    const name = resultInfo?.display_name ?? (person?.name && sanitizeFolderName(person.name) === p ? person.name : p);
    return name ? t('share.defaultTitle', { name }) : t('share.untitled');
  });

  function connectWebSocket() {
    if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) return;
    ws = new WebSocket(WS.getUrl());
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        const previous = jobStatus;
        const active =
          data.status === JOB_STATUS.running || data.status === JOB_STATUS.compiling || data.status === JOB_STATUS.cancelling;
        if (active || data.awaiting_review || jobStatus !== JOB_STATUS.idle) {
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

  // Immich connection: checked here (no header pill), shown in the About pane
  let immich = $state({ status: 'checking', version: null, error: null });

  async function checkConnection() {
    immich = { status: 'checking', version: immich.version, error: null };
    try {
      const res = await fetch(API.connection);
      const data = await res.json();
      if (data.connected) {
        immich = { status: 'connected', version: data.version, error: null };
        handleConnectionChange({ connected: true });
      } else {
        immich = { status: 'error', version: null, error: data.error || t('immich.failed') };
        handleConnectionChange({ connected: false });
      }
    } catch (e) {
      immich = { status: 'error', version: null, error: await handleError(t('immich.noBackend'), e) };
      handleConnectionChange({ connected: false });
    }
  }
  onMount(checkConnection);
  onMount(loadShares);

  function handleConnectionChange(data) {
    const was = connected;
    connected = data.connected;
    if (connected) {
      connectWebSocket();
      if (!was) {
        loadPeople();
        loadAlbums();
        loadRecipients();
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
      // The guided view always creates the video: switch it on (classic view may have turned it off)
      if (config && !config.video.enabled) {
        config.video.enabled = true;
        const json = JSON.stringify(toPayload(config));
        await saveConfig(json);
        if (lastSaved !== json) throw new Error(t('err.saveSettings'));
      }
      const res = await fetch(API.start, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          person_id: person.id,
          person_name: person.name || null,
          date_from: dateFrom || null,
          date_to: dateTo || null,
          album_ids: albums.map((a) => a.id),
          album_names: picked.map(pickedText),
          people_ids: others.filter((o) => o.kind === 'person').map((o) => o.id),
          recipient_ids: others.filter((o) => o.kind === 'recipient').map((o) => o.id),
          // Stop after preparing the photos so they can be picked before the video
          review: true,
          // Stored in run.json, so the history can take this selection over
          source: {
            person: { id: person.id, name: person.name || null },
            albums: albums.map((a) => ({ id: a.id, name: a.name })),
            others: others.map((o) => ({ id: o.id, kind: o.kind, name: o.name })),
            date_from: dateFrom || null,
            date_to: dateTo || null,
          },
        }),
      });
      const data = await res.json();
      historyFolder = null;
      if (res.ok && data.success) {
        jobStatus = JOB_STATUS.running;
        progress = { status: JOB_STATUS.running, completed: 0, total: 0, message: t('create.starting') };
      } else {
        jobStatus = JOB_STATUS.error;
        progress = { status: JOB_STATUS.error, completed: 0, total: 0, message: data.message || t('err.start') };
      }
    } catch (e) {
      const message = await handleError(t('err.start'), e);
      jobStatus = JOB_STATUS.error;
      progress = { status: JOB_STATUS.error, completed: 0, total: 0, message };
    } finally {
      starting = false;
    }
  }

  // Make the video from the picked photos of the run that waits for it
  let finishing = $state(false);
  let reviewSave = null; // last save of the photo pick
  let pickSel = $state(null); // { folder, used, all } of the photo pick, for step 2
  async function finishVideo() {
    const folder = progress.output_folder;
    if (!folder) return;
    finishing = true;
    try {
      if (reviewOpen) closeRightOf('pane');
      await tick();
      await reviewSave;
      const res = await fetch(`${API.output}/${encodeURIComponent(folder)}/compile?finish=true`, { method: 'POST' });
      if (!res.ok) throw res;
      jobStatus = JOB_STATUS.compiling;
      progress = { ...progress, status: JOB_STATUS.compiling, awaiting_review: false, completed: 0, video_started_at: Date.now() };
    } catch (e) {
      await showErrorAlert(t('err.compile'), e);
    } finally {
      finishing = false;
    }
  }

  // Take a timelapse from the history over: person, sources, period and settings.
  // Creating it again makes a new version; the old one stays.
  function editFromRun(run) {
    const src = run.source ?? {};
    const p = src.person?.id ? src.person : { id: run.person_id, name: run.person_name };
    if (p?.id) selectPerson(p);
    setPicked([
      ...(src.albums ?? []).map((a) => ({ id: a.id, name: a.name, kind: 'album' })),
      ...(src.others ?? []).filter((o) => o?.id && (o.kind === 'person' || o.kind === 'recipient')),
    ]);
    dateFrom = src.date_from ?? run.date_from ?? '';
    dateTo = src.date_to ?? run.date_to ?? '';
    if (config && run.processing && run.video) {
      const next = $state.snapshot(config);
      next.processing = $state.snapshot(run.processing);
      next.video = $state.snapshot(run.video);
      config = keepGlobal(next);
    }
    historyFolder = null;
    if (!busy && jobStatus !== JOB_STATUS.idle) {
      jobStatus = JOB_STATUS.idle;
      progress = { completed: 0, total: 0, message: '' };
    }
    closeRightOf('left');
    pane = 'create';
  }

  // Only the settings of an earlier run (filters, format, video, music, intro
  // and outro); person, sources and period stay as they are.
  function settingsFromRun(run) {
    if (!config || !run.processing || !run.video) return;
    const next = $state.snapshot(config);
    next.processing = $state.snapshot(run.processing);
    next.video = $state.snapshot(run.video);
    config = keepGlobal(next);
    closeRightOf('left');
    pane = 'create';
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
    if (histDetail && (name === null || name === histDetail)) closeRightOf('pane');
    loadOutputFolders();
  }
  // The result panel has nothing to show any more
  $effect(() => {
    if (!resultFolder && pane === 'result') closeRightOf('left');
  });

  // Start the next timelapse: no person, all photos, no period, back to step 1.
  // The settings stay; Advanced resets them to the defaults.
  async function newTimelapse() {
    if (!confirm(t('confirm.new'))) return;
    closeRightOf('left');
    historyFolder = null;
    if (!busy && jobStatus !== JOB_STATUS.idle) {
      jobStatus = JOB_STATUS.idle;
      progress = { completed: 0, total: 0, message: '' };
    }
    person = null;
    writeJson(STORAGE_KEYS.selectedPerson, null);
    setPicked([]);
    dateFrom = '';
    dateTo = '';
    closed = new Set();
    writeJson(STORAGE_KEYS.guidedClosedBoxes, []);
    pane = 'person';
    await tick();
    document.querySelector('.guided .col')?.scrollTo({ top: 0, behavior: 'smooth' });
  }

  // ---------- Texts for the boxes on the left ----------
  let lock = $derived(!person);
  let activeFilters = $derived(config ? FILTERS.filter((f) => config.processing[f.key].enabled) : []);
  let periodSummary = $derived.by(() => {
    const day = (d) => new Date(d + 'T00:00').toLocaleDateString(locale());
    let s =
      dateFrom || dateTo
        ? t('period.fromTo', { from: dateFrom ? day(dateFrom) : t('period.fromStart'), to: dateTo ? day(dateTo) : t('period.today') })
        : t('period.allDates');
    const ti = config?.processing.time_interval;
    if (ti?.enabled) s += t('period.atMostSum', { n: ti.max_photos, range: t('period.' + ti.time_range) });
    return s;
  });
  // Preview stills of the intro and outro, shown in their boxes and panels
  let introBody = $derived(previewBody(config?.video.intro));
  let outroBody = $derived(previewBody(config?.video.outro));
  $effect(() => {
    const body = introBody;
    const timer = setTimeout(() => requestPreview('intro', body), 500);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    const body = outroBody;
    const timer = setTimeout(() => requestPreview('outro', body), 500);
    return () => clearTimeout(timer);
  });
  let photoLine = $derived(
    loadingCount
      ? t('count.counting')
      : assetCount
        ? t('count.photosOf', { n: assetCount.assets_with_faces, name: person?.name || t('common.unnamed') })
        : ''
  );
</script>

<svelte:window bind:innerWidth onkeydown={onKeydown} />

<div class="guided">
  <header>
    <img class="logo" src="/icon.png" alt="" />
    <h1>{t('app.name')}</h1>
    <!-- Always there: it also pushes the controls to the right -->
    <nav class="crumbs" aria-label={t('crumb.path')}>
      {#if stack.length > 1}
        {#each stack as p, i (p.id)}
          {#if i === stack.length - 1}
            <span class="now">{p.label}</span>
          {:else}
            <button type="button" onclick={() => closeRightOf(p.id)}>{p.label}</button><span class="sep">&gt;</span>
          {/if}
        {/each}
      {/if}
    </nav>
    <nav class="langSwitch" aria-label={t('lang.label')}>
      {#each LANGS as l (l)}
        <button type="button" aria-pressed={i18n.lang === l} title={t('lang.' + l)} onclick={() => setLang(l)}>{l.toUpperCase()}</button>
      {/each}
    </nav>
    <button
      type="button"
      class="themeBtn"
      title={dark ? t('theme.light') : t('theme.dark')}
      aria-label={dark ? t('theme.light') : t('theme.dark')}
      onclick={toggleTheme}>{dark ? '☀️' : '🌙'}</button
    >
  </header>

  {#if !connected}
    <div class="card notConnected">
      <p>{t('conn.title')}</p>
      {#if immich.status === 'checking'}
        <p class="muted">{t('immich.checking')}</p>
      {:else if immich.error}
        <p class="err">{immich.error}</p>
        <p><button type="button" onclick={checkConnection}>{t('immich.retry')}</button></p>
      {/if}
      <p class="muted">{t('conn.backend')}</p>
      <p class="muted">
        {t('conn.key')}
        <code>asset.download</code>, <code>asset.read</code>, <code>asset.view</code>, <code>person.read</code>,
        <code>album.read</code>, <code>server.about</code>.
        <a href="https://immich.app/docs/features/command-line-interface#obtain-the-api-key" target="_blank" rel="noopener noreferrer">{t('conn.docs')}</a>
      </p>
      <p class="mt"><button type="button" onclick={() => onswitch('classic')}>{t('conn.back')}</button></p>
    </div>
  {:else}
    <main bind:clientWidth={pageW} style:grid-template-columns={narrow ? null : fit.cols}>
      <!-- Left: the process -->
      <div class="col" class:offLeft={off('left')}>
        {#if jobActive}
          {@const video = jobStatus === JOB_STATUS.compiling}
          <section class="card jobBar" role="status">
            <p>
              <strong>{busy ? t('job.running', { name: jobName }) : t('job.waiting', { name: jobName })}</strong>
              {#if jobStatus === JOB_STATUS.cancelling}
                <span class="muted">· {t('create.cancelling')}</span>
              {:else if busy}
                <span class="muted"
                  >· {video ? t('create.phase2') : t('create.phase1')}{progress.total && (!video || progress.completed)
                    ? `: ${t('create.xOfY', { x: fmt(progress.completed), y: fmt(progress.total) })}`
                    : ''}</span
                >
              {/if}
            </p>
            <div class="btnRow">
              <button type="button" onclick={showJob}>{t('job.show')}</button>
              {#if busy}
                <button type="button" class="danger" disabled={stopping || jobStatus === JOB_STATUS.cancelling} onclick={cancelJob}
                  >{t('common.cancel')}</button
                >
              {/if}
            </div>
          </section>
        {/if}
        <section class="card kind">
          <h2>{t('view.title')}</h2>
          <div class="seg">
            <button type="button" onclick={() => onswitch('classic')} aria-pressed="false">{t('view.classic')}</button>
            <button type="button" class="on" aria-pressed="true">{t('view.guided')}</button>
          </div>
          <div class="btnRow mt"><button type="button" disabled={busy} title={busy ? t('job.newBusy') : null} onclick={newTimelapse}>{t('view.new')}</button></div>
          {#if configError}<p class="err mt">{configError}</p>{/if}
          {#if saveText}<p class="mt" class:muted={!saveFailed} class:err={saveFailed}>{saveText}</p>{/if}
        </section>

        <div id="g-box-history">
          <Box title={t('history.title')} paneOpen={pane === 'history'} onopen={() => openPane('history')}>
            <p class="muted">
              {outputFolders.length === 0 ? t('history.noneYet') : t('history.n', { n: outputFolders.length })}
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
                <span><strong>{person.name || t('common.unnamed')}</strong><br /><span class="muted">{photoLine}</span></span>
              </div>
            {:else}
              <p class="muted">{t('person.none')}</p>
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
            summary={picked.length === 0 ? t('albums.all') : picked.map(pickedText).join(', ')}
          >
            <fieldset disabled={lock || busy}>
              <label class="check" style="margin-top:0">
                <input
                  type="checkbox"
                  checked={picked.length === 0}
                  onchange={(e) => {
                    if (e.currentTarget.checked) setPicked([]);
                    else {
                      e.currentTarget.checked = true; // choosing albums happens in the panel
                      if (pane !== 'albums') openPane('albums');
                    }
                  }}
                />
                {t('albums.all')}
              </label>
              {#if picked.length > 0}
                <p class="lbl">{t('albums.chosen')}</p>
                {#each picked as a (a.id)}
                  <div class="pickedRow">
                    <span>{pickedText(a)}</span>
                    <button type="button" title={t('common.remove')} aria-label={t('albums.remove', { name: a.name })} onclick={() => setPicked(picked.filter((x) => x.id !== a.id))}>×</button>
                  </div>
                {/each}
              {/if}
              {#if photoLine}<p class="mt">{photoLine}</p>{/if}
            </fieldset>
          </Box>
        </div>

        <div id="g-box-period">
          <Box title={STEPS[2].label} locked={lock} paneOpen={pane === 'period'} onopen={() => openPane('period')}>
            <p class:muted={lock}>{periodSummary}</p>
          </Box>
        </div>

        <div id="g-box-quality">
          <Box title={STEPS[3].label} locked={lock || !config} paneOpen={pane === 'quality'} onopen={() => openPane('quality')}>
            {#if config}
              <p>{t('quality.active', { n: activeFilters.length, total: FILTERS.length })}</p>
              <p class="muted">{activeFilters.map((f) => f.title).join(', ') || t('quality.allUsed')}</p>
            {:else}
              <p class="muted">{t('common.loadingSettings')}</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-format">
          <Box title={STEPS[4].label} locked={lock || !config} paneOpen={pane === 'format'} onopen={() => openPane('format')}>
            {#if config}
              {@const p = config.processing}
              {@const [fw, fh] = frameSize(p.output)}
              <p class="muted">
                {p.output.aspect_ratio ?? '1:1'}, {fw} × {fh} px, {t('format.eyesSum', { n: (Number(p.alignment.eye_distance) * 100).toFixed(0) })}<br />
                {p.timestamp.enabled ? t('format.dateSum', { pos: POSITIONS[p.timestamp.position] ?? '' }) : t('format.noDate')}
              </p>
            {:else}
              <p class="muted">{t('common.loadingSettings')}</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-video">
          <Box title={STEPS[5].label} locked={lock || !config} paneOpen={pane === 'video'} onopen={() => openPane('video')}>
            {#if config}
              {@const v = config.video}
              <p class="muted">
                {v.fit_to_duration
                  ? t('video.fpsDuration', { len: formatDuration(Number(v.target_duration_secs)) })
                  : v.fit_to_music && (v.music_tracks ?? []).length
                    ? t('video.fpsMusic')
                    : `${v.framerate} fps`},
                {CODECS[v.codec] ?? v.codec}, CRF {v.crf}<br />
                {(v.music_tracks ?? []).length ? t('music.nTracks', { n: v.music_tracks.length }) : t('music.none')}
              </p>
            {:else}
              <p class="muted">{t('common.loadingSettings')}</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-intro">
          <Box title={STEPS[6].label} locked={lock || !config} paneOpen={pane === 'intro'} onopen={() => openPane('intro')}>
            {#if config}
              <TitleCardSummary card={config.video.intro} which="intro" />
            {:else}
              <p class="muted">{t('common.loadingSettings')}</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-outro">
          <Box title={STEPS[7].label} locked={lock || !config} paneOpen={pane === 'outro'} onopen={() => openPane('outro')}>
            {#if config}
              <TitleCardSummary card={config.video.outro} which="outro" />
            {:else}
              <p class="muted">{t('common.loadingSettings')}</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-create">
          <Box title={STEPS[8].label} locked={lock && !jobActive} paneOpen={pane === 'create'} onopen={() => openPane('create')}>
            {#if jobStatus === JOB_STATUS.cancelling}
              <p class="muted">{t('create.cancelling')}</p>
            {:else if busy}
              {@const video = jobStatus === JOB_STATUS.compiling}
              <p class="muted">
                {video ? t('create.phase2') : t('create.phase1')}{progress.total && (!video || progress.completed)
                  ? `: ${t('create.xOfY', { x: fmt(progress.completed), y: fmt(progress.total) })}`
                  : ''}
              </p>
              {#if progress.total && (!video || progress.completed)}
                <progress max={progress.total} value={progress.completed}></progress>
              {:else}
                <progress></progress>
              {/if}
            {:else if jobStatus === JOB_STATUS.error}
              <p class="err">{progress.message || t('create.failed')}</p>
            {:else if jobStatus === JOB_STATUS.completed && progress.video_millis != null}
              <p class="muted">{t('create.doneSum', { took: formatDuration(progress.video_millis / 1000), len: formatDuration(progress.video_secs) })}</p>
            {:else if !person}
              <p class="muted">{t('create.choosePerson')}</p>
            {:else}
              <p class="muted">{photoLine || person.name} · {t('create.ready')}</p>
            {/if}
          </Box>
        </div>

        <div id="g-box-result">
          <Box title={STEPS[9].label} locked={!resultFolder} paneOpen={pane === 'result'} onopen={() => openPane('result')}>
            <p class="muted">
              {#if !resultFolder}
                {t('result.noneYet')}
              {:else if historyFolder}
                {t('result.opened')} {resultFolder}
              {:else}
                {t('result.finished')} {resultFolder}
              {/if}
            </p>
          </Box>
        </div>

        <div id="g-box-settings">
          <Box title={t('settings.title')} paneOpen={pane === 'settings'} onopen={() => openPane('settings')}>
            <p class="muted">{t('settings.sum')}</p>
          </Box>
        </div>

        <div id="g-box-about">
          <Box title={t('about.title')} paneOpen={pane === 'about'} onopen={() => openPane('about')}>
            <p class="muted">{t('about.sum')}</p>
          </Box>
        </div>
      </div>

      <!-- Level 1: the panel of a step -->
      {#if pane}
        <Pane title={paneTitle(pane)} level={1} offLeft={off('pane')} nav={stepNav(pane)} onclose={() => closeRightOf('left')}>
          {#if pane === 'about'}
            <AboutPane {immich} onretry={checkConnection} changelogOpen={pane2 === 'changelog'} onchangelog={() => openPane2('changelog')} />
          {:else if pane === 'settings'}
            {#if config}<SettingsPane bind:config />{:else}<p class="muted">{t('common.loading')}</p>{/if}
          {:else if pane === 'history'}
            <HistoryPane
              folders={outputFolders}
              current={histDetail ?? resultFolder}
              running={jobActive ? (progress.output_folder ?? null) : null}
              disabled={busy} onopen={openHistDetail} ondeleted={handleFolderDeleted} />
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
              people={otherPeople}
              {recipients}
              {recipientsError}
              counts={coCounts}
              loading={albumState.loading || peopleState.loading}
              error={albumState.error}
              {picked}
              disabled={busy}
              onchange={setPicked}
              onpeopletab={loadCoCounts}
            />
          {:else if pane === 'period'}
            <PeriodPane bind:dateFrom bind:dateTo bind:config disabled={busy} />
          {:else if pane === 'quality' && config}
            <fieldset disabled={busy}><QualityPane bind:config open2={pane2} onopen={openPane2} {selection} /></fieldset>
          {:else if pane === 'format' && config}
            <fieldset disabled={busy}>
              <FormatPane bind:config dateOpen={pane2 === 'date'} ondate={() => openPane2('date')} />
            </fieldset>
          {:else if pane === 'video' && config}
            <fieldset disabled={busy}>
              <VideoPane bind:config {music} musicOpen={pane2 === 'music'} onmusic={() => openPane2('music')} />
            </fieldset>
          {:else if (pane === 'intro' || pane === 'outro') && config}
            <fieldset disabled={busy}>
              {#key pane}
                <TitlesPane bind:config which={pane} pickOpen={pane2 === 'pick'} onpick={() => openPane2('pick')} />
              {/key}
            </fieldset>
          {:else if pane === 'create'}
            <CreatePane
              {person}
              albums={picked.map((p) => ({ id: p.id, name: pickedText(p) }))}
              {dateFrom}
              {dateTo}
              bind:config
              {assetCount}
              {loadingCount}
              {jobStatus}
              {progress}
              {busy}
              {starting}
              onstart={startProcessing}
              onresult={() => goStep(9)}
              {reviewOpen}
              onreview={toggleReview}
              onfinish={finishVideo}
              {finishing}
              {pickSel}
            />
          {:else if pane === 'result' && resultFolder}
            <ResultPane folderName={resultFolder} folder={resultInfo} stamp={resultStamp} photosOpen={right} onphotos={togglePhotos} {shareOpen} onshare={toggleShare} />
          {:else}
            <p class="muted">{t('common.loading')}</p>
          {/if}
        </Pane>
      {/if}

      <!-- Level 2: details of a setting -->
      {#if pane2 === 'changelog'}
        <Pane title={pane2Title(pane2)} level={2} offLeft={off('pane2')} onclose={() => closeRightOf('pane')}>
          <ChangelogPane />
        </Pane>
      {:else if pane2 && config}
        <Pane title={pane2Title(pane2)} level={2} offLeft={off('pane2')} onclose={() => closeRightOf('pane')}>
          <fieldset disabled={busy}>
            {#if pane2 === 'adv'}
              <AdvancedPane bind:config onreset={resetToDefaults} />
            {:else if pane2 === 'date'}
              <DatePane bind:config />
            {:else if pane2 === 'pick'}
              <ImagePickerPane bind:config which={titleWhich} albums={albumList} openAlbumId={pickAlbum?.id} onalbum={openPickAlbum} />
            {:else if pane2 === 'music'}
              <MusicPane bind:config {music} loading={musicState.loading} error={musicState.error} onrefresh={loadMusic} />
            {:else}
              <FilterDetail bind:config key={pane2.slice(2)} {selection} />
            {/if}
          </fieldset>
        </Pane>
      {/if}

      <!-- Level 2: details of a timelapse from the history -->
      {#if histDetail && histInfo}
        <Pane title={histTitle} level={2} offLeft={off('hist')} onclose={() => closeRightOf('pane')}>
          <HistoryDetail folder={histInfo} disabled={busy} onwatch={() => openHistory(histDetail)} onedit={editFromRun} onsettings={settingsFromRun} />
        </Pane>
      {/if}

      <!-- Level 2: pick the prepared photos before the video is made -->
      {#if reviewOpen && progress.awaiting_review && progress.output_folder}
        <Pane title={t('review.title')} level={2} offLeft={off('review')} onclose={() => closeRightOf('pane')}>
          {#key progress.output_folder}
            <ReviewPane
              folderName={progress.output_folder}
              onsaving={(p) => (reviewSave = p)}
              onchange={(sel) =>
                (pickSel = { folder: progress.output_folder, used: sel.images.length, all: sel.images.length + sel.excluded.length })}
            />
          {/key}
        </Pane>
      {/if}

      <!-- Level 3: photos of the album opened in the image picker -->
      {#if pane2 === 'pick' && pickAlbum && config}
        <Pane title={pickAlbum.name} level={3} offLeft={off('album')} onclose={() => closeRightOf('pane2')}>
          <fieldset disabled={busy}>
            <AlbumPhotosPane bind:config which={titleWhich} album={pickAlbum} />
          </fieldset>
        </Pane>
      {/if}

      <!-- Level 3: share the result -->
      {#if shareOpen && resultFolder}
        <Pane title={t('share.title')} level={3} offLeft={off('share')} onclose={() => closeRightOf('pane')}>
          {#key resultFolder}
            <SharePane folderName={resultFolder} defaultTitle={shareTitle} />
          {/key}
        </Pane>
      {/if}

      <!-- Level 3: the used photos -->
      {#if right && resultFolder}
        <Pane title={t('common.usedPhotos')} level={3} wide offLeft={off('right')} onclose={() => closeRightOf('pane')}>
          {#key resultFolder}
            <GalleryView folderName={resultFolder} onBack={() => closeRightOf('pane')} disabled={busy} />
          {/key}
        </Pane>
      {/if}
    </main>
  {/if}
</div>
