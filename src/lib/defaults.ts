// Defaults for before the backend answers, shared by the store and the
// browser preview. They match the Rust defaults (settings.rs, model.rs).
import type { PlaybackOptions, Settings } from "./types";

export const DEFAULT_SETTINGS: Settings = {
  capture_moves: true,
  capture_keys: true,
  countdown: true,
  esc_stops_recording: true,
  ignore_injected: true,
  path_mode: "full",
  show_click_labels: true,
  close_to_tray: true,
  keep_on_top: "always",
};

export const DEFAULT_PLAYBACK: PlaybackOptions = {
  speed: 1,
  repeat: { count: 1 },
  humanize: true,
  jitter_ms: 40,
  stop_on_key: true,
  coord_mode: "screen",
};
