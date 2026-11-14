/**
 * Application constants and magic values.
 */

// ===== Timing (milliseconds) =====
export const TIMING = {
  messageDisplayDuration: 2000,
};

// ===== Local Storage Keys =====
export const STORAGE_KEYS = {
  selectedPerson: 'immich-timelapse-selected-person',
  selectedAlbums: 'immich-timelapse-selected-albums',
  selectedSources: 'immich-timelapse-selected-sources', // guided view: people and Smart Albums recipients
  sourceSort: 'immich-timelapse-source-sort',
  view: 'immich-timelapse-view', // 'classic' | 'guided'
  guidedClosedBoxes: 'immich-timelapse-guided-closed-boxes',
  lang: 'lang', // guided view language, per app (localStorage is per host, not shared)
  theme: 'immich-timelapse-theme', // 'light' | 'dark', read again in index.html before render
};

// ===== API Endpoints =====
export const API = {
  health: '/api/health',
  version: '/api/version',
  connection: '/api/connection',
  people: '/api/people',
  albums: '/api/albums',
  recipients: '/api/recipients',
  progress: '/api/progress',
  start: '/api/start',
  cancel: '/api/cancel',
  preview: '/api/preview',
  previewSamples: '/api/preview/samples',
  output: '/api/output',
  config: '/api/config',
  configDefaults: '/api/config/defaults',
  music: '/api/music',
  fonts: '/api/fonts',
  titleImages: '/api/titles/images',
  titlePreview: '/api/titles/preview',
  albumAssets: '/api/album-assets',
  assetSearch: '/api/asset-search',
  assets: '/api/assets', // + /{id}/thumbnail
  shares: '/api/shares',
  ws: '/api/ws',
};

// ===== WebSocket =====
export const WS = {
  reconnectDelay: 1000,
  getUrl: () => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${protocol}//${window.location.host}${API.ws}`;
  },
};

// ===== Job Status =====
export const JOB_STATUS = {
  idle: 'idle',
  running: 'running',
  compiling: 'compiling_video',
  cancelling: 'cancelling',
  completed: 'completed',
  cancelled: 'cancelled',
  error: 'error',
};

