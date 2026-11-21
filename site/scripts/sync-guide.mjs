// Turns docs/user-guide (written for GitHub) into Starlight pages under src/content/docs/guide.
// The guide stays the only source; this output is generated and git-ignored.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const BASE = "/Relay";
export const REPO = "https://github.com/HappyRave/Relay/blob/main/docs";

const ALERTS = { NOTE: "note", TIP: "tip", IMPORTANT: "note", WARNING: "caution", CAUTION: "danger" };

/** The page a guide link points to, as a site URL (or a GitHub URL outside the guide). */
export function rewriteLink(href) {
  if (/^([a-z]+:|#|\/)/i.test(href)) {
    return href;
  }
  const [file, hash] = href.split("#");
  const anchor = hash ? `#${hash}` : "";
  if (file.startsWith("../images/")) {
    return href;
  }
  if (file.startsWith("../")) {
    return `${REPO}/${file.slice(3)}${anchor}`;
  }
  if (!file.endsWith(".md")) {
    throw new Error(`Guide link to something that isn't a page: ${href}`);
  }
  const slug = file === "README.md" ? "" : `${file.slice(0, -3)}/`;
  return `${BASE}/guide/${slug}${anchor}`;
}

/** Converts one guide page: its first heading becomes the title, links and callouts become Starlight's. */
export function convert(md, order) {
  const lines = md.replace(/\r\n/g, "\n").split("\n");
  const h1 = lines.findIndex(l => l.startsWith("# "));
  if (h1 < 0) {
    throw new Error("A guide page has no # heading");
  }
  const title = lines[h1].slice(2).trim();
  let body = lines.slice(h1 + 1);

  // The GitHub footer (Previous · Next · Guide home): Starlight draws its own.
  const footer = body.findLastIndex(l => /^Previous: |^Next: |^\[Guide home\]/.test(l));
  if (footer >= 0) {
    const rule = body.lastIndexOf("---", footer);
    body = body.slice(0, rule >= 0 ? rule : footer);
  }

  // The page's own contents list (links to its sections): Starlight shows "On this page".
  const toc = body.findIndex(l => /^- \[[^\]]+\]\(#[^)]+\)$/.test(l));
  if (toc >= 0 && !body.slice(0, toc).some(l => l.startsWith("## "))) {
    let end = toc;
    while (end < body.length && /^- \[[^\]]+\]\(#[^)]+\)$/.test(body[end])) {
      end++;
    }
    body.splice(toc, end - toc);
  }

  const out = [];
  for (let i = 0; i < body.length; i++) {
    const worked = body[i].match(/^If it worked, (.+)$/);
    if (worked) {
      out.push(":::tip[If it worked]", worked[1][0].toUpperCase() + worked[1].slice(1), ":::");
      continue;
    }
    const alert = body[i].match(/^> \[!([A-Z]+)\]\s*$/);
    if (!alert) {
      out.push(body[i]);
      continue;
    }
    const kind = ALERTS[alert[1]];
    if (!kind) {
      throw new Error(`Unknown callout: ${alert[1]}`);
    }
    out.push(`:::${kind}`);
    while (i + 1 < body.length && body[i + 1].startsWith(">")) {
      out.push(body[++i].replace(/^> ?/, ""));
    }
    out.push(":::");
  }

  const text = out.join("\n").trim().replace(/\]\(([^)\s]+)\)/g, (_, href) => `](${rewriteLink(href)})`);
  const lede = out.find(l => l.trim() && !/^[-#>:|<*\d!]/.test(l.trim()));
  const front = [
    "---",
    `title: ${JSON.stringify(title)}`,
    ...(lede ? [`description: ${JSON.stringify(lede.trim())}`] : []),
    `sidebar:`,
    `  order: ${order}`,
    "---",
  ];
  return `${front.join("\n")}\n\n${text}\n`;
}

/** The guide's reading order, following each page's "Next:" link from the first page. */
export function readingOrder(pages) {
  const next = name => pages[name]?.match(/^(?:Previous: .*· )?Next: \[[^\]]*\]\(([^)#]+)(?:#[^)]*)?\)/m)?.[1];
  const order = [];
  for (let name = "getting-started.md"; name && !order.includes(name); name = next(name)) {
    if (!pages[name]) {
      throw new Error(`The guide's chain of Next links points to a missing page: ${name}`);
    }
    order.push(name);
  }
  return order;
}

function sync(root) {
  const src = path.join(root, "docs", "user-guide");
  const dest = path.join(root, "site", "src", "content", "docs", "guide");
  const pages = Object.fromEntries(fs.readdirSync(src).filter(f => f.endsWith(".md")).map(f => [f, fs.readFileSync(path.join(src, f), "utf8")]));
  const order = readingOrder(pages);
  const unchained = Object.keys(pages).filter(f => f !== "README.md" && !order.includes(f));
  if (unchained.length) {
    throw new Error(`Guide pages outside the chain of Next links: ${unchained.join(", ")}`);
  }

  fs.rmSync(dest, { recursive: true, force: true });
  fs.mkdirSync(dest, { recursive: true });
  fs.writeFileSync(path.join(dest, "index.md"), convert(pages["README.md"], 0));
  order.forEach((name, i) => fs.writeFileSync(path.join(dest, name), convert(pages[name], i + 1)));

  const images = path.join(root, "site", "src", "content", "docs", "images");
  fs.rmSync(images, { recursive: true, force: true });
  fs.cpSync(path.join(root, "docs", "images"), images, { recursive: true });
  console.log(`Synced ${order.length + 1} guide pages and the images.`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  sync(path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", ".."));
}
