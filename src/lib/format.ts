/** mm:ss.cc, as in the design (`00:07.35`). */
export function fmtTime(ms: number): string {
  ms = Math.max(0, ms);
  const s = ms / 1000;
  const m = Math.floor(s / 60);
  return String(m).padStart(2, "0") + ":" + (s % 60).toFixed(2).padStart(5, "0");
}

/** "Recording 3" → "recording-3", used for export file names. */
export function slug(name: string): string {
  return name.replace(/[^\w]+/g, "-").toLowerCase();
}

export const pad4 = (n: number) => String(Math.round(n)).padStart(4, "0");

const DAY_NAMES = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/**
 * The label for the next scheduled run after `now`, e.g. "Next run: Thu 09:00".
 * `days` is Monday-first. Replaced by relay-core's `next_run` in M7.
 */
export function nextRunLabel(enabled: boolean, days: boolean[], time: string, now = new Date()): string {
  if (!enabled || !days.some(Boolean)) return "No schedule";
  const [hh, mm] = time.split(":").map(Number);
  for (let add = 0; add <= 7; add++) {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + add, hh, mm);
    const monFirst = (d.getDay() + 6) % 7;
    if (days[monFirst] && d > now) {
      const when = add === 0 ? "Today" : add === 1 ? "Tomorrow" : DAY_NAMES[monFirst];
      return `Next run: ${when} ${time}`;
    }
  }
  return "No schedule";
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
