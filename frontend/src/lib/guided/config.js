/**
 * Config helpers for the guided view.
 * toPayload mirrors the normalization in SettingsPanel.saveConfig so both views
 * send the same shape to PUT /api/config.
 */
import { t } from './i18n.svelte.js';

export function toPayload(config) {
  const p = config.processing;
  const v = config.video;
  return {
    processing: {
      max_workers: Number(p.max_workers),
      use_preview: p.use_preview,
      face_resolution: {
        enabled: p.face_resolution.enabled,
        min_size: Number(p.face_resolution.min_size),
      },
      crop: {
        enabled: p.crop.enabled,
        max_padding_percent: Number(p.crop.max_padding_percent),
      },
      brightness: {
        enabled: p.brightness.enabled,
        min_brightness: Number(p.brightness.min_brightness),
        max_brightness: Number(p.brightness.max_brightness),
      },
      blur: {
        enabled: p.blur.enabled,
        min_sharpness: Number(p.blur.min_sharpness),
      },
      head_pose: {
        enabled: p.head_pose.enabled,
        max_yaw: Number(p.head_pose.max_yaw),
        max_pitch: Number(p.head_pose.max_pitch),
      },
      eye_filter: {
        enabled: p.eye_filter.enabled,
        min_ear: Number(p.eye_filter.min_ear),
      },
      output: {
        ...p.output,
        size: Math.max(64, Math.min(4096, Number(p.output.size) || 512)),
        aspect_ratio: p.output.aspect_ratio ?? '1:1',
        keep_intermediates: p.output.keep_intermediates,
        keep_versions: Math.max(0, Math.floor(Number(p.output.keep_versions) || 0)),
      },
      alignment: {
        eye_distance: Number(p.alignment.eye_distance),
      },
      timestamp: {
        ...p.timestamp,
        enabled: p.timestamp.enabled,
        position: p.timestamp.position,
        year: p.timestamp.year,
        month: p.timestamp.month,
        day: p.timestamp.day,
        font_size: p.timestamp.font_size == null ? null : Number(p.timestamp.font_size),
      },
      time_interval: {
        enabled: p.time_interval.enabled,
        max_photos: Number(p.time_interval.max_photos),
        time_range: p.time_interval.time_range,
      },
    },
    video: {
      enabled: v.enabled,
      framerate: Number(v.framerate),
      codec: v.codec,
      crf: Number(v.crf),
      music_tracks: v.music_tracks ?? [],
      music_fade_out_secs: Number(v.music_fade_out_secs),
      fit_to_music: v.fit_to_music,
      fit_to_duration: !!v.fit_to_duration,
      target_duration_secs: Math.min(3600, Math.max(1, Number(v.target_duration_secs) || 30)),
      min_framerate: Number(v.min_framerate),
      max_framerate: Math.max(Number(v.min_framerate), Number(v.max_framerate)),
      intro: cardPayload(v.intro),
      outro: cardPayload(v.outro),
    },
  };
}

/** Defaults of an intro or outro card, like TitleCardConfig::default on the server. */
export const CARD_DEFAULTS = {
  enabled: false,
  asset_id: null,
  upload: null,
  duration_secs: 5,
  title: '',
  subtitle: '',
  text: '',
  font: 'DejaVuSans-Bold.ttf',
  color: '#ffffff',
  subtitle_color: '', // '' = like the title
  text_color: '',
  background: '#000000',
  size: 8,
  dim: 0.15,
  zoom: true, // background moves; off = still image
  motion: 'auto', // 'auto' | 'zoom' | 'pan_right' | 'pan_left' | 'pan_down' | 'pan_up' | 'random'
  dates: false, // line with the date of the oldest and newest photo
};

/** Movements of the title background; 'still' stands for zoom off. */
export const MOTIONS = ['auto', 'zoom', 'pan_right', 'pan_left', 'pan_down', 'pan_up', 'random'];

const clamp = (v, min, max, fallback) => {
  const n = Number(v);
  return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : fallback;
};

/** An intro or outro card as the server takes it; empty image fields become null. */
export function cardPayload(card) {
  const c = { ...CARD_DEFAULTS, ...(card ?? {}) };
  return {
    enabled: !!c.enabled,
    asset_id: c.asset_id ? String(c.asset_id) : null,
    upload: c.asset_id ? null : c.upload || null,
    duration_secs: clamp(c.duration_secs, 1, 30, 5),
    title: c.title ?? '',
    subtitle: c.subtitle ?? '',
    text: c.text ?? '',
    font: c.font ?? '',
    color: c.color || '#ffffff',
    subtitle_color: c.subtitle_color || '',
    text_color: c.text_color || '',
    background: c.background || '#000000',
    size: clamp(c.size, 2, 25, 8),
    dim: clamp(c.dim, 0, 0.9, 0.15),
    zoom: !!c.zoom,
    motion: MOTIONS.includes(c.motion) ? c.motion : 'auto',
    dates: !!c.dates,
  };
}

/** The quality filters, in the order shown in step 4. Texts follow the language. */
const filter = (key, summary) => ({
  key,
  get title() {
    return t(`filters.${key}.title`);
  },
  get hint() {
    return t(`filters.${key}.hint`);
  },
  summary,
});

export const FILTERS = [
  filter('face_resolution', (p) => t('filters.face_resolution.sum', { n: p.face_resolution.min_size })),
  filter('crop', (p) => t('filters.crop.sum', { n: p.crop.max_padding_percent })),
  filter('brightness', (p) =>
    t('filters.brightness.sum', {
      min: Math.round(Number(p.brightness.min_brightness) * 100),
      max: Math.round(Number(p.brightness.max_brightness) * 100),
    })
  ),
  filter('blur', (p) => t('filters.blur.sum', { n: Number(p.blur.min_sharpness).toFixed(0) })),
  filter('head_pose', (p) =>
    t('filters.head_pose.sum', {
      yaw: Number(p.head_pose.max_yaw).toFixed(0),
      pitch: Number(p.head_pose.max_pitch).toFixed(0),
    })
  ),
  filter('eye_filter', (p) => t('filters.eye_filter.sum', { n: Number(p.eye_filter.min_ear).toFixed(2) })),
];

export function formatDuration(secs) {
  if (secs == null) return '?';
  const s = Math.round(secs);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export const POSITIONS = {
  get top_left() {
    return t('pos.top_left');
  },
  get top_center() {
    return t('pos.top_center');
  },
  get top_right() {
    return t('pos.top_right');
  },
  get bottom_left() {
    return t('pos.bottom_left');
  },
  get bottom_center() {
    return t('pos.bottom_center');
  },
  get bottom_right() {
    return t('pos.bottom_right');
  },
};

/** Aspect ratios of the frames, as width:height. */
export const ASPECTS = {
  get '1:1'() {
    return t('aspect.1:1');
  },
  get '4:5'() {
    return t('aspect.4:5');
  },
  get '5:4'() {
    return t('aspect.5:4');
  },
  get '16:9'() {
    return t('aspect.16:9');
  },
  get '9:16'() {
    return t('aspect.9:16');
  },
};

/** Frame width and height in pixels, like OutputConfig::dimensions on the server. */
export function frameSize(output) {
  const size = Math.max(64, Math.min(4096, Number(output.size) || 512));
  const [a, b] = String(output.aspect_ratio ?? '1:1').split(':').map(Number);
  const even = (v) => Math.max(2, v - (v % 2));
  const short = (x, y) => Math.floor((size * x + Math.floor(y / 2)) / y);
  return a >= b ? [even(size), even(short(b, a))] : [even(short(a, b)), even(size)];
}

// Date formats in the frames (same names as DateFormat on the server)
export const DATE_FORMATS = [
  'iso', 'dmy_dot', 'dmy_dot_short', 'dmy_dot_yy', 'dmy_dash', 'dmy_slash', 'mdy_dash', 'mdy_slash',
  'long', 'long_short', 'month_year', 'weekday',
];
const NAMED = new Set(['long', 'long_short', 'month_year', 'weekday']);
/** Whether the format writes month (or weekday) names. */
export const namedDateFormat = (f) => NAMED.has(f);

const MONTHS = {
  de: ['Januar', 'Februar', 'März', 'April', 'Mai', 'Juni', 'Juli', 'August', 'September', 'Oktober', 'November', 'Dezember'],
  en: ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'],
};
const MONTHS_SHORT = {
  de: ['Jan.', 'Feb.', 'März', 'Apr.', 'Mai', 'Juni', 'Juli', 'Aug.', 'Sept.', 'Okt.', 'Nov.', 'Dez.'],
  en: ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'],
};
const WEEKDAYS = {
  de: ['Sonntag', 'Montag', 'Dienstag', 'Mittwoch', 'Donnerstag', 'Freitag', 'Samstag'],
  en: ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'],
};

/** The date as the server writes it into the frames (mirror of TimestampStep::format_date). */
export function formatDate(ts, y, m, d) {
  const lang = ts.lang === 'en' ? 'en' : 'de';
  const pad = (n) => String(n).padStart(2, '0');
  const numeric = (order, sep, padded = true, shortYear = false) =>
    [...order]
      .map((c) =>
        c === 'd' && ts.day ? (padded ? pad(d) : String(d))
        : c === 'm' && ts.month ? (padded ? pad(m) : String(m))
        : c === 'y' && ts.year ? (shortYear ? pad(y % 100) : String(y))
        : null
      )
      .filter((v) => v != null)
      .join(sep);
  const named = (short, withDay) =>
    [
      withDay && ts.day ? (lang === 'en' ? String(d) : `${d}.`) : null,
      ts.month ? (short ? MONTHS_SHORT : MONTHS)[lang][m - 1] : null,
      ts.year ? String(y) : null,
    ]
      .filter(Boolean)
      .join(' ');
  switch (ts.format ?? 'iso') {
    case 'dmy_dot': return numeric('dmy', '.');
    case 'dmy_dot_short': return numeric('dmy', '.', false);
    case 'dmy_dot_yy': return numeric('dmy', '.', true, true);
    case 'dmy_dash': return numeric('dmy', '-');
    case 'dmy_slash': return numeric('dmy', '/');
    case 'mdy_dash': return numeric('mdy', '-');
    case 'mdy_slash': return numeric('mdy', '/');
    case 'long': return named(false, true);
    case 'long_short': return named(true, true);
    case 'month_year': return named(false, false);
    case 'weekday': {
      const text = named(false, true);
      if (!(ts.day && ts.month && ts.year)) return text;
      return `${WEEKDAYS[lang][new Date(y, m - 1, d).getDay()]}, ${text}`;
    }
    default: return numeric('ymd', '-');
  }
}

/** Height of the 5x7 date font in pixels: today's 7 px, or a share of the frame height. */
export function datePixels(timestamp, frameHeight) {
  if (timestamp.font_size == null) return 7;
  return Math.max(1, Math.round((Number(timestamp.font_size) / 100) * frameHeight / 7)) * 7;
}

export const CODECS = {
  libx264: 'H.264',
  libx265: 'H.265',
  'libvpx-vp9': 'VP9',
};
