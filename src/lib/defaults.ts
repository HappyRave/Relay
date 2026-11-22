// Defaults for before the backend answers, shared by the store and the
// browser preview. They match the Rust defaults (settings.rs, model.rs).
import type { PlaybackOptions, Settings } from "./types";
import type { SystemLook } from "./ipc/bindings/SystemLook";

export const DEFAULT_SETTINGS: Settings = {
  capture_moves: true,
  capture_keys: true,
  countdown: true,
  esc_stops_recording: true,
  ignore_injected: true,
  capture_screen: true,
  path_mode: "full",
  show_click_labels: true,
  preview_background: "screen",
  close_to_tray: true,
  keep_on_top: "always",
  hide_while_recording: true,
  hide_while_playing: false,
  theme: "system",
  glass: null,
  spacing: "comfortable",
  text_size: 100,
  player_size: "compact",
  reduce_motion: "system",
  sounds: false,
  show_moves: false,
  keys: { record: "Ctrl + Shift + F9", play: "Ctrl + Shift + F10", player: "Ctrl + Shift + M", kill: "Ctrl + Alt + Backspace" },
  intro: "welcome",
};

/** Until Rust says: Windows 11, transparency on. */
export const DEFAULT_LOOK: SystemLook = { glass_default: true, transparency: true };

export const DEFAULT_PLAYBACK: PlaybackOptions = {
  speed: 1,
  repeat: { count: 1 },
  humanize: true,
  jitter_ms: 40,
  stop_on_key: true,
  coord_mode: "screen",
};
