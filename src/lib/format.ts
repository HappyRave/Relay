/** mm:ss.cc, as in the design (`00:07.35`). */
export function fmtTime(ms: number): string {
  // Round once, in centiseconds, so 59.996 s is 01:00.00 and not 00:60.00.
  const cs = Math.round(Math.max(0, ms) / 10);
  const m = Math.floor(cs / 6000);
  const s = (cs % 6000) / 100;
  return String(m).padStart(2, "0") + ":" + s.toFixed(2).padStart(5, "0");
}

/** "Déplacer fenêtre 3" → "déplacer-fenêtre-3", for export file names ("" when nothing is left). */
export function slug(name: string): string {
  return name
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
}

/** A coordinate as 4 digits, with its sign when negative (monitors left of the primary). */
export const pad4 = (n: number) => (n < 0 ? "-" : "") + String(Math.abs(Math.round(n))).padStart(4, "0");

const DAY_NAMES = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/**
 * "Next run: Today 09:00", "Tomorrow 07:30" or "Mon 09:00", from the time
 * relay-core computed (RFC 3339, local). `null` means nothing is scheduled.
 */
export function nextRunLabel(iso: string | null, now = new Date()): string {
  if (!iso) return "No schedule";
  const d = new Date(iso);
  const startOfDay = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOfDay(d) - startOfDay(now)) / 86_400_000);
  const hm = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  const when = days <= 0 ? "Today" : days === 1 ? "Tomorrow" : DAY_NAMES[(d.getDay() + 6) % 7];
  return `Next run: ${when} ${hm}`;
}

/** "Today, 09:12", "Fri, 17:40", "Sep 12" or "Never", as in the Library tab. */
export function fmtLastRun(iso: string | null, now = new Date()): string {
  if (!iso) return "Never";
  const d = new Date(iso);
  const hm = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const startOfDay = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOfDay(now) - startOfDay(d)) / 86_400_000);
  if (days <= 0) return `Today, ${hm}`;
  if (days < 7) return `${d.toLocaleDateString([], { weekday: "short" })}, ${hm}`;
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
}

/** "1 step", "3 steps". */
export const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
