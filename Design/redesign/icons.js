// Relay's own icons: a 16 px grid, 1.6 px strokes with round ends, filled only
// for the transport (record, play, pause, stop). Drawn for the visual redesign.
const S = 'fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"';
const F = 'fill="currentColor"';

const ICONS = {
  record: `<circle cx="8" cy="8" r="5" ${F}/>`,
  play: `<path d="M5 3.2v9.6L12.8 8z" ${F}/>`,
  pause: `<rect x="4" y="3.2" width="2.8" height="9.6" rx=".8" ${F}/><rect x="9.2" y="3.2" width="2.8" height="9.6" rx=".8" ${F}/>`,
  stop: `<rect x="3.8" y="3.8" width="8.4" height="8.4" rx="1.6" ${F}/>`,
  prev: `<path d="M11.5 3.5 6 8l5.5 4.5M4.5 3.5v9" ${S}/>`,
  next: `<path d="M4.5 3.5 10 8l-5.5 4.5M11.5 3.5v9" ${S}/>`,
  back: `<path d="M10 3 5 8l5 5" ${S}/>`,
  down: `<path d="m4 6 4 4 4-4" ${S}/>`,
  undo: `<path d="M5.5 4 2.5 7l3 3M2.5 7H10a3.5 3.5 0 0 1 0 7H8" ${S}/>`,
  redo: `<path d="m10.5 4 3 3-3 3M13.5 7H6a3.5 3.5 0 0 0 0 7h2" ${S}/>`,
  more: `<circle cx="3.5" cy="8" r="1.3" ${F}/><circle cx="8" cy="8" r="1.3" ${F}/><circle cx="12.5" cy="8" r="1.3" ${F}/>`,
  close: `<path d="m4 4 8 8M12 4l-8 8" ${S}/>`,
  minimize: `<path d="M3.5 8h9" ${S}/>`,
  expand: `<path d="M9.5 2.5h4v4M13.5 2.5 9 7M6.5 13.5h-4v-4M2.5 13.5 7 9" ${S}/>`,
  collapse: `<path d="M13.5 6.5h-4v-4M9.5 6.5 14 2M2.5 9.5h4v4M6.5 9.5 2 14" ${S}/>`,
  dial: `<circle cx="8" cy="8" r="5.6" ${S}/><path d="M8 8 10.6 4.6" ${S}/><path d="M8 .9v1.2M15.1 8h-1.2M8 15.1v-1.2M.9 8h1.2" ${S}/>`,
  search: `<circle cx="7" cy="7" r="4.2" ${S}/><path d="m10.2 10.2 3.3 3.3" ${S}/>`,
  plus: `<path d="M8 3v10M3 8h10" ${S}/>`,
  trash: `<path d="M2.8 4.5h10.4M6.3 4.5V2.8h3.4v1.7M4.3 4.5l.7 8.7h6l.7-8.7" ${S}/>`,
  duplicate: `<rect x="5.5" y="5.5" width="8" height="8" rx="1.6" ${S}/><path d="M10.5 3.2v-.2a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h.3" ${S}/>`,
  export: `<path d="M8 10V2.5M5 5.5l3-3 3 3M2.8 9.5v3.7h10.4V9.5" ${S}/>`,
  import: `<path d="M8 2.5V10M5 7l3 3 3-3M2.8 9.5v3.7h10.4V9.5" ${S}/>`,
  mouse: `<rect x="4.4" y="1.8" width="7.2" height="12.4" rx="3.6" ${S}/><path d="M8 1.8v4" ${S}/>`,
  keyboard: `<rect x="1.5" y="4" width="13" height="8.4" rx="1.6" ${S}/><path d="M4.3 7h.01M7 7h.01M9.7 7h.01M12 7h.01M4.8 9.8h6.4" ${S}/>`,
  keycap: `<rect x="1.8" y="2.2" width="12.4" height="11.6" rx="2.6" ${S}/><rect x="4.4" y="4.4" width="7.2" height="5.4" rx="1.4" ${S}/>`,
  sliders: `<path d="M2.2 4.5h6.3M11.9 4.5h1.9M2.2 11.5h1.9M7.5 11.5h6.3" ${S}/><circle cx="10.2" cy="4.5" r="1.7" ${S}/><circle cx="5.8" cy="11.5" r="1.7" ${S}/>`,
  clock: `<circle cx="8" cy="8" r="6" ${S}/><path d="M8 4.6V8l2.4 1.6" ${S}/>`,
  app: `<rect x="1.8" y="2.8" width="12.4" height="10.4" rx="1.6" ${S}/><path d="M1.8 5.8h12.4M4 4.3h.01M5.8 4.3h.01" ${S}/>`,
  pixel: `<rect x="5" y="5" width="6" height="6" rx="1" ${S}/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2" ${S}/>`,
  image: `<rect x="1.8" y="2.8" width="9.4" height="8" rx="1.2" ${S}/><path d="m3.3 9.2 2.4-2.4 1.8 1.8" ${S}/><circle cx="11.4" cy="11" r="2.4" ${S}/><path d="m13.1 12.7 1.4 1.4" ${S}/>`,
  pick: `<circle cx="8" cy="8" r="4.4" ${S}/><path d="M8 .9v3M8 12.1v3M.9 8h3M12.1 8h3" ${S}/><circle cx="8" cy="8" r=".9" ${F}/>`,
  freeze: `<path d="M1.8 5V1.8H5M11 1.8h3.2V5M14.2 11v3.2H11M5 14.2H1.8V11" ${S}/><rect x="5.6" y="5.6" width="4.8" height="4.8" rx=".8" ${F}/>`,
  check: `<path d="m3.3 8.4 3 3 6.4-6.8" ${S}/>`,
  error: `<circle cx="8" cy="8" r="6" ${S}/><path d="m5.8 5.8 4.4 4.4M10.2 5.8l-4.4 4.4" ${S}/>`,
  warning: `<path d="M8 2.2 14.2 13H1.8z" ${S}/><path d="M8 6.4v3" ${S}/><circle cx="8" cy="11.2" r=".9" ${F}/>`,
  info: `<circle cx="8" cy="8" r="6" ${S}/><path d="M8 7.2v4" ${S}/><circle cx="8" cy="4.9" r=".9" ${F}/>`,
  folder: `<path d="M1.8 4.3a1 1 0 0 1 1-1h3.4l1.6 1.6h5.4a1 1 0 0 1 1 1v6.8a1 1 0 0 1-1 1H2.8a1 1 0 0 1-1-1z" ${S}/>`,
  file: `<path d="M3.8 1.8h5.6l2.8 2.8v9.6H3.8z" ${S}/><path d="M9.4 1.8v2.8h2.8M5.8 8h4.4M5.8 10.6h4.4" ${S}/>`,
  sound: `<path d="M2.5 6.2h2.4L8.4 3.3v9.4L4.9 9.8H2.5z" ${S}/><path d="M11 5.6a3.4 3.4 0 0 1 0 4.8M12.8 3.8a6 6 0 0 1 0 8.4" ${S}/>`,
  mute: `<path d="M2.5 6.2h2.4L8.4 3.3v9.4L4.9 9.8H2.5z" ${S}/><path d="m11 6 3 4M14 6l-3 4" ${S}/>`,
  history: `<path d="M2.8 8a5.2 5.2 0 1 0 1.5-3.7" ${S}/><path d="M2.5 2.5v2.4h2.4M8 5v3.2l2.1 1.4" ${S}/>`,
  library: `<rect x="2.2" y="2.2" width="4.8" height="4.8" rx="1.2" ${S}/><rect x="9" y="2.2" width="4.8" height="4.8" rx="1.2" ${S}/><rect x="2.2" y="9" width="4.8" height="4.8" rx="1.2" ${S}/><rect x="9" y="9" width="4.8" height="4.8" rx="1.2" ${S}/>`,
  smooth: `<path d="M1.8 10.5c2-4.5 4.2-4.5 6.2-1.5s4.2 3 6.2-2" ${S}/>`,
  straighten: `<path d="M2 12 14 4" ${S}/><circle cx="2" cy="12" r="1" ${F}/><circle cx="14" cy="4" r="1" ${F}/>`,
  snip: `<path d="M2 5.2V2h3.2M10.8 2H14v3.2M14 10.8V14h-3.2M5.2 14H2v-3.2" ${S}/><path d="M5.5 8h5" ${S}/>`,
  paste: `<rect x="3.3" y="2.9" width="9.4" height="11.3" rx="1.6" ${S}/><rect x="5.9" y="1.6" width="4.2" height="2.6" rx=".8" ${S}/>`,
  target: `<circle cx="8" cy="8" r="5.8" ${S}/><circle cx="8" cy="8" r="2.6" ${S}/><circle cx="8" cy="8" r=".9" ${F}/>`,
  eye: `<path d="M1.5 8S4 3.6 8 3.6 14.5 8 14.5 8 12 12.4 8 12.4 1.5 8 1.5 8z" ${S}/><circle cx="8" cy="8" r="2" ${S}/>`,
  text: `<path d="M3.5 3.5h9M8 3.5v9" ${S}/><path d="M2 9.5c-.8 0-.8.6-.8 1.2s0 1.3-.7 1.3c.7 0 .7.7.7 1.3s0 1.2.8 1.2M14 9.5c.8 0 .8.6.8 1.2s0 1.3.7 1.3c-.7 0-.7.7-.7 1.3s0 1.2-.8 1.2" ${S} stroke-width="1.2"/>`,
  wait: `<path d="M4.5 1.8h7M4.5 14.2h7M5.2 1.8c0 3.6 5.6 3.9 5.6 6.2s-5.6 2.6-5.6 6.2M10.8 1.8c0 3.6-5.6 3.9-5.6 6.2s5.6 2.6 5.6 6.2" ${S}/>`,
  move: `<path d="M2 13.5c2.8 0 3.6-3.8 5.6-5.6s4.6-3.4 6.4-3.4" ${S} stroke-dasharray="1.6 2.4"/><path d="M11 2.6l3 1.9-1.9 3" ${S}/>`,
  click: `<path d="M4.2 2.2v10.2l2.7-2.5 1.9 4.1 1.7-.8-1.9-4 3.8-.2z" ${S}/>`,
  scroll: `<path d="M8 2.5v11M5.5 5 8 2.5 10.5 5M5.5 11 8 13.5l2.5-2.5" ${S}/>`,
  drag: `<path d="M3 13 13 3M8.5 3H13v4.5" ${S}/><circle cx="3" cy="13" r="1.2" ${F}/>`,
  data: `<rect x="1.8" y="2.6" width="12.4" height="10.8" rx="1.4" ${S}/><path d="M1.8 6.2h12.4M1.8 9.8h12.4M6.2 2.6v10.8" ${S}/>`,
  grip: `<circle cx="5.5" cy="4" r="1" ${F}/><circle cx="10.5" cy="4" r="1" ${F}/><circle cx="5.5" cy="8" r="1" ${F}/><circle cx="10.5" cy="8" r="1" ${F}/><circle cx="5.5" cy="12" r="1" ${F}/><circle cx="10.5" cy="12" r="1" ${F}/>`,
  lock: `<rect x="3.2" y="7" width="9.6" height="7" rx="1.6" ${S}/><path d="M5.4 7V5a2.6 2.6 0 0 1 5.2 0v2" ${S}/>`,
  sun: `<circle cx="8" cy="8" r="2.8" ${S}/><path d="M8 1.5v1.4M8 13.1v1.4M1.5 8h1.4M13.1 8h1.4M3.4 3.4l1 1M11.6 11.6l1 1M3.4 12.6l1-1M11.6 4.4l1-1" ${S}/>`,
  moon: `<path d="M13.2 9.6A5.6 5.6 0 0 1 6.4 2.8a5.6 5.6 0 1 0 6.8 6.8z" ${S}/>`,
  glass: `<rect x="2" y="2.5" width="12" height="11" rx="2.4" ${S}/><path d="M4.6 10.8 11 4.4M7.4 11l4-4" ${S} stroke-width="1.2"/>`,
};

/** An icon as inline SVG, hidden from assistive technology (its button carries the name). */
function icon(name, size = 16, extra = "") {
  return `<svg class="ic" width="${size}" height="${size}" viewBox="0 0 16 16" aria-hidden="true" focusable="false"${extra}>${ICONS[name] ?? ""}</svg>`;
}

window.RelayIcons = { ICONS, icon };
