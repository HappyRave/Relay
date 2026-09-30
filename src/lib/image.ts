// Find image steps carry their image as a base64 PNG.

/** The width and height of a base64 PNG (from its header), or [0, 0] if it isn't one. */
export function pngSize(png: string): [number, number] {
  let bytes: string;
  try {
    bytes = atob(png.slice(0, 32));
  } catch {
    return [0, 0];
  }
  if (!bytes.startsWith("\x89PNG") || bytes.length < 24) return [0, 0];
  const u32 = (i: number) => ((bytes.charCodeAt(i) << 24) | (bytes.charCodeAt(i + 1) << 16) | (bytes.charCodeAt(i + 2) << 8) | bytes.charCodeAt(i + 3)) >>> 0;
  return [u32(16), u32(20)];
}

/** A base64 PNG as an image URL. */
export const pngUrl = (png: string) => `data:image/png;base64,${png}`;
