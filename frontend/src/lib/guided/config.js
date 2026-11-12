/**
 * Config helpers for the guided view.
 * toPayload mirrors the normalization in SettingsPanel.saveConfig so both views
 * send the same shape to PUT /api/config.
 */

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
        size: Math.max(64, Math.min(4096, Number(p.output.size) || 512)),
        keep_intermediates: p.output.keep_intermediates,
      },
      alignment: {
        eye_distance: Number(p.alignment.eye_distance),
      },
      timestamp: {
        enabled: p.timestamp.enabled,
        position: p.timestamp.position,
        year: p.timestamp.year,
        month: p.timestamp.month,
        day: p.timestamp.day,
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
      min_framerate: Number(v.min_framerate),
      max_framerate: Math.max(Number(v.min_framerate), Number(v.max_framerate)),
    },
  };
}

/** The quality filters, in the order shown in step 4. */
export const FILTERS = [
  {
    key: 'face_resolution',
    title: 'Face resolution',
    hint: 'Skip photos where the face is too small. Small faces look blurry when scaled up.',
    summary: (p) => `At least ${p.face_resolution.min_size} px`,
  },
  {
    key: 'crop',
    title: 'Face too close to edge',
    hint: 'Padding is added when a face is at the border of a photo, to move it to the center of the frame.',
    summary: (p) => `At most ${p.crop.max_padding_percent} % padding`,
  },
  {
    key: 'brightness',
    title: 'Brightness',
    hint: 'Discard photos where the face is under or over exposed.',
    summary: (p) =>
      `${Math.round(Number(p.brightness.min_brightness) * 100)} % to ${Math.round(Number(p.brightness.max_brightness) * 100)} %`,
  },
  {
    key: 'blur',
    title: 'Blur',
    hint: 'Discard blurry faces.',
    summary: (p) => `Sharpness at least ${Number(p.blur.min_sharpness).toFixed(0)}`,
  },
  {
    key: 'head_pose',
    title: 'Head pose',
    hint: 'Keep only photos where the person looks roughly into the camera.',
    summary: (p) => `Turn ${Number(p.head_pose.max_yaw).toFixed(0)}°, tilt ${Number(p.head_pose.max_pitch).toFixed(0)}°`,
  },
  {
    key: 'eye_filter',
    title: 'Blink detection',
    hint: 'Discard photos where the eyes are closed.',
    summary: (p) => `Eye opening at least ${Number(p.eye_filter.min_ear).toFixed(2)}`,
  },
];

export function formatDuration(secs) {
  if (secs == null) return '?';
  const s = Math.round(secs);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export const POSITIONS = {
  top_left: 'Top left',
  top_right: 'Top right',
  bottom_left: 'Bottom left',
  bottom_right: 'Bottom right',
};

export const CODECS = {
  libx264: 'H.264',
  libx265: 'H.265',
  'libvpx-vp9': 'VP9',
};
