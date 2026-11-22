import { test } from "node:test";
import assert from "node:assert/strict";
import { convert, readingOrder, rewriteLink } from "./sync-guide.mjs";

test("guide links become site URLs, keeping their anchor", () => {
  assert.equal(rewriteLink("record.md"), "/Relay/guide/record/");
  assert.equal(rewriteLink("record.md#record-the-esc-key"), "/Relay/guide/record/#record-the-esc-key");
  assert.equal(rewriteLink("README.md"), "/Relay/guide/");
});

test("links outside the guide go to GitHub; images, anchors and URLs stay", () => {
  assert.equal(rewriteLink("../spec/recording.md#privacy"), "https://github.com/HappyRave/Relay/blob/main/docs/spec/recording.md#privacy");
  assert.equal(rewriteLink("../engineering/file-formats.md"), "https://github.com/HappyRave/Relay/blob/main/docs/engineering/file-formats.md");
  assert.equal(rewriteLink("../images/editor.png"), "../images/editor.png");
  assert.equal(rewriteLink("#good-to-know"), "#good-to-know");
  assert.equal(rewriteLink("https://example.com/a.md"), "https://example.com/a.md");
});

test("a link to a file that isn't a page is refused", () => {
  assert.throws(() => rewriteLink("notes.txt"), /isn't a page/);
});

const page = `# Record a macro

Record a task so that it plays back right the first time.

> [!WARNING]
> Relay doesn't record clicks on its own windows.
> Move the player away.

Read [Stop a macro](stop-a-macro.md#every-way) and [the rules](../spec/recording.md).

---

Previous: [Get started](getting-started.md) · Next: [Stop a macro](stop-a-macro.md) · [Guide home](README.md)
`;

test("the first heading becomes the title, the next paragraph the description", () => {
  const out = convert(page, 2);
  assert.match(out, /^---\ntitle: "Record a macro"\ndescription: "Record a task so that it plays back right the first time."\nsidebar:\n {2}order: 2\n---\n/);
  assert.doesNotMatch(out, /^# Record a macro/m);
});

test("GitHub callouts become Starlight asides", () => {
  assert.match(convert(page, 2), /:::caution\nRelay doesn't record clicks on its own windows.\nMove the player away.\n:::/);
});

test("links are rewritten and the GitHub footer is dropped", () => {
  const out = convert(page, 2);
  assert.match(out, /\[Stop a macro\]\(\/Relay\/guide\/stop-a-macro\/#every-way\)/);
  assert.match(out, /\[the rules\]\(https:\/\/github.com\/HappyRave\/Relay\/blob\/main\/docs\/spec\/recording.md\)/);
  assert.doesNotMatch(out, /Previous:|Guide home/);
  assert.match(out, /recording\.md\)\.\n$/);
});

test("the page's own contents list is dropped, and other link lists kept", () => {
  const md = "# T\n\nIntro.\n\n- [Start](#start)\n- [End](#end)\n\n## Start\n\n- [Elsewhere](#start)\n- [Record](record.md)\n";
  const out = convert(md, 1);
  assert.doesNotMatch(out, /\[End\]/);
  assert.match(out, /- \[Elsewhere\]\(#start\)/);
});

test("an \"If it worked\" line becomes a green aside", () => {
  assert.match(convert("# T\n\nIf it worked, the player shows REC.\n", 1), /:::tip\[If it worked\]\nThe player shows REC.\n:::/);
});

test("a page without a heading or with an unknown callout is refused", () => {
  assert.throws(() => convert("No heading here.", 1), /no # heading/);
  assert.throws(() => convert("# T\n\n> [!SHOUT]\n> hi", 1), /Unknown callout/);
});

test("the reading order follows the Next links from the first page", () => {
  const pages = {
    "getting-started.md": "# A\n\nNext: [B](record.md) · [Guide home](README.md)",
    "record.md": "# B\n\nPrevious: [A](getting-started.md) · Next: [C](faq.md#top) · [Guide home](README.md)",
    "faq.md": "# C\n\nPrevious: [B](record.md) · [Guide home](README.md)",
  };
  assert.deepEqual(readingOrder(pages), ["getting-started.md", "record.md", "faq.md"]);
});

test("a Next link to a missing page is refused", () => {
  assert.throws(() => readingOrder({ "getting-started.md": "Next: [X](gone.md)" }), /missing page: gone.md/);
});
