# Changelog

## v0.1.0-m0: Scaffold and skin

- Cargo workspace (`relay-core`, `relay-platform`, `src-tauri`) on Tauri 2.11.
- Svelte 5 + TypeScript frontend on Vite 8, with the Modernist design tokens and Archivo bundled locally (no network requests).
- Frameless, always-on-top widget matching the design at 944×612 (expanded) and 604×68 (compact). Switching modes keeps the bottom-center fixed.
- The full UI from the prototype: path preview, Steps / Library / Triggers / Settings tabs, transport, four-lane timeline and export dialog.
- An in-browser mock engine (`npm run dev`) that runs the prototype's logic on its demo desktop. Recording, playback, loops, seek and step edits all work there.
- Frontend unit tests for the timeline and preview math (`npm test`).
