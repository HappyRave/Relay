<script lang="ts" module>
  // The design's inline icons (Lucide-style), kept as SVG markup.
  const ICONS = {
    grip: { fill: true, body: '<circle cx="9" cy="5" r="1.5"/><circle cx="9" cy="12" r="1.5"/><circle cx="9" cy="19" r="1.5"/><circle cx="15" cy="5" r="1.5"/><circle cx="15" cy="12" r="1.5"/><circle cx="15" cy="19" r="1.5"/>' },
    record: { fill: true, body: '<circle cx="12" cy="12" r="7"/>' },
    square: { fill: true, body: '<rect x="5" y="5" width="14" height="14"/>' },
    stop: { fill: true, body: '<rect x="4" y="4" width="16" height="16"/>' },
    play: { fill: true, body: '<polygon points="6 3 20 12 6 21"/>' },
    pause: { fill: true, body: '<rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/>' },
    prev: { fill: true, body: '<polygon points="19 20 9 12 19 4"/><rect x="4" y="4" width="3" height="16"/>' },
    next: { fill: true, body: '<polygon points="5 4 15 12 5 20"/><rect x="17" y="4" width="3" height="16"/>' },
    expand: { fill: false, sw: 2, body: '<polyline points="15 3 21 3 21 9"/><polyline points="9 21 3 21 3 15"/><line x1="21" y1="3" x2="14" y2="10"/><line x1="3" y1="21" x2="10" y2="14"/>' },
    collapse: { fill: false, sw: 2, body: '<polyline points="4 14 10 14 10 20"/><polyline points="20 10 14 10 14 4"/><line x1="14" y1="10" x2="21" y2="3"/><line x1="3" y1="21" x2="10" y2="14"/>' },
    export: { fill: false, sw: 2.2, body: '<path d="M12 15V3"/><path d="m7 8 5-5 5 5"/><path d="M20 15v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-4"/>' },
    x: { fill: false, sw: 2.5, body: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>' },
    loop: { fill: false, sw: 2.2, body: '<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>' },
    minus: { fill: false, sw: 2.5, body: '<path d="M5 12h14"/>' },
    copy: { fill: false, sw: 2, body: '<rect x="9" y="9" width="13" height="13"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>' },
    trash: { fill: false, sw: 2, body: '<path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>' },
    import: { fill: false, sw: 2.2, body: '<path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M20 15v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-4"/>' },
  } as const;
  export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
  let { name, size = 16 }: { name: IconName; size?: number } = $props();
  const icon = $derived(ICONS[name] as { fill: boolean; sw?: number; body: string });
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={icon.fill ? "currentColor" : "none"}
  stroke={icon.fill ? "none" : "currentColor"}
  stroke-width={icon.sw}
  aria-hidden="true"
>
  {@html icon.body}
</svg>

<style>
  svg {
    display: block;
    pointer-events: none;
    flex: none;
  }
</style>
