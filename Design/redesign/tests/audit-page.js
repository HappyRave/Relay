// Runs inside the prototype: checks every visible text and control in Relay's
// window and player (not the prototype's own controls).
(() => {
  const parse = (c) => { const f = c.match(/color\(srgb ([\d.]+) ([\d.]+) ([\d.]+)(?: \/ ([\d.]+))?\)/); if (f) return { r: f[1] * 255, g: f[2] * 255, b: f[3] * 255, a: f[4] != null ? +f[4] : 1 }; const m = c.match(/rgba?\(([^)]+)\)/); if (!m) return null; const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 }; };
  const over = (top, bot) => { const a = top.a + bot.a * (1 - top.a); if (!a) return { r: 0, g: 0, b: 0, a: 0 }; return { r: (top.r * top.a + bot.r * bot.a * (1 - top.a)) / a, g: (top.g * top.a + bot.g * bot.a * (1 - top.a)) / a, b: (top.b * top.a + bot.b * bot.a * (1 - top.a)) / a, a }; };
  const lum = ({ r, g, b }) => { const f = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b); };
  const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
  const desc = (el) => `${el.tagName.toLowerCase()}${el.className && typeof el.className === "string" ? "." + el.className.trim().split(/\s+/).slice(0, 3).join(".") : ""}${el.id ? "#" + el.id : ""}`;
  const text = (el) => (el.innerText ?? el.textContent ?? "").trim().replace(/\s+/g, " ").slice(0, 50);
  const scope = [...document.querySelectorAll(".win-pos, .player-pos, .menu, .exported, .picker, .icon-sheet")];
  const inScope = (el) => scope.some((s) => s.contains(el));
  const visible = (el) => {
    const r = el.getBoundingClientRect(); if (r.width < 1 || r.height < 1) return false;
    for (let e = el; e && e !== document.body; e = e.parentElement) { const cs = getComputedStyle(e); if (cs.visibility === "hidden" || cs.display === "none" || +cs.opacity < 0.05) return false; }
    return true;
  };
  const zoomOf = (el) => { let z = 1; for (let e = el; e; e = e.parentElement) z *= parseFloat(getComputedStyle(e).zoom) || 1; return z; };
  const opacityOf = (el) => { let o = 1; for (let e = el; e; e = e.parentElement) o *= +getComputedStyle(e).opacity; return o; };

  // The background under an element: translucent layers down to an opaque one,
  // or down to the wallpaper when it's glass (worst cases: black and white).
  function backgrounds(el) {
    const layers = [];
    let glass = false;
    for (let e = el; e; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (e.matches?.(".step.played")) { const w = getComputedStyle(e, "::before").backgroundColor; const p = parse(w); if (p && p.a) layers.push(p); }
      const bg = parse(cs.backgroundColor);
      if (bg && bg.a > 0) { layers.push(bg); if (bg.a >= 0.99) break; }
      if (cs.backdropFilter && cs.backdropFilter !== "none") { glass = true; break; }
      if (e.classList?.contains("desktop")) { glass = true; break; }
    }
    const bases = glass ? [{ r: 0, g: 0, b: 0, a: 1 }, { r: 255, g: 255, b: 255, a: 1 }] : [layers.pop() ?? { r: 255, g: 255, b: 255, a: 1 }];
    return bases.map((base) => layers.reduceRight((acc, l) => over(l, acc), base));
  }

  const issues = { contrast: [], small: [], names: [], targets: [], clipped: [], overflow: [] };
  const truncated = [];
  const all = [...document.querySelectorAll("*")].filter((el) => inScope(el) && !el.closest("svg"));
  for (const el of all) {
    if (!visible(el)) continue;
    const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
    const cs = getComputedStyle(el);
    if (own) {
      const size = parseFloat(cs.fontSize) * zoomOf(el);
      if (size < 11.95) issues.small.push({ el: desc(el), text: text(el), size: +size.toFixed(1) });
      const disabled = el.closest(":disabled, [aria-disabled=true]");
      const fg = parse(cs.color); const op = opacityOf(el);
      if (fg && !disabled) {
        const bgs = backgrounds(el);
        const worst = Math.min(...bgs.map((bg) => ratio(over({ ...fg, a: fg.a * op }, bg), bg)));
        const large = size >= 24 || (size >= 18.66 && +cs.fontWeight >= 700);
        const need = large ? 3 : 4.5;
        if (worst < need) issues.contrast.push({ el: desc(el), text: text(el), ratio: +worst.toFixed(2), need, size: +size.toFixed(1) });
      }
      if (!el.classList.contains("sr-only") && el.scrollWidth > el.clientWidth + 1 && cs.overflow !== "visible" && el.clientWidth > 0) {
        (cs.textOverflow === "ellipsis" ? truncated : issues.clipped).push({ el: desc(el), text: text(el) });
      }
    }
  }
  const controls = [...document.querySelectorAll('button, input, textarea, select, [role=button], [role=tab], [role=radio], [role=switch], [role=checkbox], [role=slider], [role=menuitem], [role=listbox]')].filter((el) => inScope(el) && visible(el) && !el.closest("[aria-hidden=true]"));
  for (const el of controls) {
    const lb = el.getAttribute("aria-labelledby");
    const name = el.getAttribute("aria-label") || (lb && lb.split(" ").map((id) => document.getElementById(id)?.textContent).join(" ").trim()) || (el.matches("input, textarea") ? "" : text(el)) || el.title;
    if (!name) issues.names.push({ el: desc(el), html: el.outerHTML.slice(0, 120) });
    const r = el.getBoundingClientRect();
    if ((r.width < 23.5 || r.height < 23.5) && !el.matches("[role=listbox]")) issues.targets.push({ el: desc(el), name: name?.slice(0, 40), w: Math.round(r.width), h: Math.round(r.height) });
  }
  for (const win of document.querySelectorAll(".window, .player")) {
    const wr = win.getBoundingClientRect();
    for (const el of win.querySelectorAll("*")) {
      if (!visible(el) || el.closest(".menu, .toasts, .dlg-wrap")) continue;
      const r = el.getBoundingClientRect();
      let clipped = false;
      for (let e = el.parentElement; e && e !== win; e = e.parentElement) { const o = getComputedStyle(e).overflowY; if (o !== 'visible') { const cr = e.getBoundingClientRect(); if (r.top >= cr.bottom - 1 || r.bottom > cr.bottom + 1 || r.left < cr.left - 1 || r.right > cr.right + 1) { clipped = true; break; } } }
      if (clipped) continue;
      if (r.right > wr.right + 1 || r.bottom > wr.bottom + 1 || r.left < wr.left - 1) { issues.overflow.push({ el: desc(el), text: text(el), dx: Math.round(Math.max(r.right - wr.right, wr.left - r.left)), dy: Math.round(r.bottom - wr.bottom) }); }
    }
  }
  const running = document.getAnimations().filter((a) => a.playState === "running").map((a) => a.animationName ?? a.transitionProperty ?? "anim");
  const dedupe = (arr) => { const seen = new Set(); return arr.filter((x) => { const k = JSON.stringify(x); if (seen.has(k)) return false; seen.add(k); return true; }); };
  for (const k of Object.keys(issues)) issues[k] = dedupe(issues[k]);
  return { issues, truncated: dedupe(truncated).slice(0, 20), running, counts: Object.fromEntries(Object.entries(issues).map(([k, v]) => [k, v.length])) };
})()
