// Text steps: inserting one with + Text, turning typing into one with
// Make editable, and editing its text (placeholders, the preview of what it
// types, mistakes) and duration; how the list, the timeline and the preview show it.
import { beforeEach, describe, expect, test } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import StepsTab from "./StepsTab.svelte";
import Timeline from "../Timeline.svelte";
import Preview from "../Preview.svelte";
import { core, freshStore, settle } from "../../../test/app";
import type { RelayStore } from "../../../lib/state/relay.svelte";

let relay: RelayStore;
beforeEach(async () => {
  relay = await freshStore();
});

const ops = () => core.argsOf("edit_macro").map((a) => a.op);
const stepRows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("row"));
const editor = () => screen.getByRole("group", { name: "Edit step" });
const textField = () => within(editor()).getByRole("textbox", { name: "Text" }) as HTMLTextAreaElement;

/** Renders the steps with a Text step typing `text` as step 2 (after the first move and click), its editor open. */
async function withText(text: string) {
  render(StepsTab);
  await relay.edit({ op: "insert_text", at: 0, text });
  await settle();
  await userEvent.click(stepRows()[2]);
  await settle();
  core.calls = [];
}

/** Replaces what the Text field holds and leaves it, as typing then tabbing away would. */
async function typeText(text: string) {
  await fireEvent.input(textField(), { target: { value: text } });
  await fireEvent.change(textField());
  await settle();
}

describe("+ Text", () => {
  test("inserts an empty Text step at the playhead and opens its editor", async () => {
    render(StepsTab);
    relay.seek(1000);
    await userEvent.click(screen.getByRole("button", { name: "+ Text" }));
    await settle();
    expect(ops()).toEqual([{ op: "insert_text", at: 1000, text: "" }]);
    const row = stepRows()[2]; // in the pause after the first click
    expect(row).toHaveTextContent("TEXT");
    expect(row).toHaveTextContent("Filled in when it plays");
    expect(row).toHaveAttribute("aria-expanded", "true");
    expect(textField().value).toBe("");
    expect(within(editor()).getByRole("status")).toHaveTextContent("Types nothing yet");
  });

  test("is off during a session", async () => {
    render(StepsTab);
    core.emit({ type: "session", mode: "playing", macro_id: relay.view!.id });
    await settle();
    expect(screen.getByRole("button", { name: "+ Text" })).toBeDisabled();
  });
});

describe("the Text step's editor", () => {
  test("shows what the text types now, and saves it when the field is left", async () => {
    await withText("No. {n}");
    expect(textField().value).toBe("No. {n}");
    expect(within(editor()).getByRole("status")).toHaveTextContent("Types now: “No. 1”");
    core.clipboardText = "ACME";
    await typeText("{clipboard} on {date}");
    expect(ops()).toEqual([{ op: "update_text", index: 2, text: "{clipboard} on {date}" }]);
    expect(stepRows()[2]).toHaveTextContent("“{clipboard} on {date}”");
    expect(within(editor()).getByRole("status")).toHaveTextContent("Types now: “ACME on 2026-10-01”");
  });

  test("updates the preview while typing, without saving", async () => {
    await withText("");
    await fireEvent.input(textField(), { target: { value: "at {time}" } });
    await settle();
    expect(within(editor()).getByRole("status")).toHaveTextContent("Types now: “at 09:05:07”");
    expect(ops()).toEqual([]);
  });

  test("the data file's columns are placeholders, filled from its first row", async () => {
    await withText("Dear ");
    expect(within(editor()).queryByRole("button", { name: "{col:Customer}" })).toBeNull();
    core.csv.set("C:\\customers.csv", { columns: ["Customer", "", "a{b}"], rows: [["ACME"]] });
    core.dialog.open = "C:\\customers.csv";
    await relay.chooseDataFile();
    await settle();
    expect(within(editor()).queryByRole("button", { name: "{col:}" })).toBeNull();
    textField().setSelectionRange(5, 5);
    await userEvent.click(within(editor()).getByRole("button", { name: "{col:Customer}" }));
    await settle();
    expect(ops()).toEqual([{ op: "update_text", index: 2, text: "Dear {col:Customer}" }]);
    expect(within(editor()).getByRole("status")).toHaveTextContent("Types now: “Dear ACME”");
  });

  test("a column without a data file says where to choose one", async () => {
    await withText("");
    await typeText("{col:Customer}");
    const status = within(editor()).getByRole("status");
    expect(status).toHaveTextContent("A Text step types {col:Customer}: choose a data file in Settings → Playback.");
    expect(status).toHaveClass("wrong");
    expect(ops()).toEqual([]);
  });

  test("a text with a mistake says what's wrong and isn't saved", async () => {
    await withText("No. {n}");
    await typeText("Hello {name}");
    const status = within(editor()).getByRole("status");
    expect(status).toHaveTextContent("{name} isn't a placeholder: use {date}, {time}, {clipboard}, {n} or {col:Name}.");
    expect(status).toHaveClass("wrong");
    expect(ops()).toEqual([]);
    expect(textField().value).toBe("Hello {name}");
    expect(relay.toast).toBeNull();
  });

  test("a placeholder button goes where the caret is, and saves", async () => {
    await withText("Invoice  done");
    textField().setSelectionRange(8, 8);
    await userEvent.click(within(editor()).getByRole("button", { name: "{date}" }));
    await settle();
    expect(ops()).toEqual([{ op: "update_text", index: 2, text: "Invoice {date} done" }]);
    expect(textField().selectionStart).toBe(14);
    for (const name of ["{time}", "{clipboard}", "{n}"]) expect(within(editor()).getByRole("button", { name })).toBeInTheDocument();
  });

  test("leaving the field unchanged saves nothing", async () => {
    await withText("No. {n}");
    await typeText("No. {n}");
    expect(ops()).toEqual([]);
  });

  test("its duration is set like a wait's, and shows what Rust kept", async () => {
    await withText("No. {n}"); // 50 ms of typing
    const duration = within(editor()).getByRole("spinbutton", { name: "Duration s" }) as HTMLInputElement;
    expect(duration.value).toBe("0.05");
    await userEvent.clear(duration);
    await userEvent.type(duration, "2");
    await fireEvent.change(duration);
    await settle();
    expect(ops()).toEqual([{ op: "set_wait_duration", index: 2, dur: 2000 }]);
    await userEvent.clear(duration);
    await userEvent.type(duration, "0");
    await fireEvent.change(duration);
    await settle();
    expect(ops().at(-1)).toEqual({ op: "set_wait_duration", index: 2, dur: 0 });
    expect(duration.value).toBe("0.05"); // not shorter than the typing
  });

  test("a Text step has no label", async () => {
    await withText("x");
    expect(within(editor()).queryByRole("textbox", { name: "Label" })).toBeNull();
  });
});

describe("Make editable", () => {
  test("turns a TYPE step into a Text step typing the same, and keeps its editor open", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[8]); // “invoice_0924”
    await userEvent.click(within(editor()).getByRole("button", { name: "Make editable" }));
    await settle();
    expect(ops()).toEqual([{ op: "make_editable", index: 8 }]);
    expect(stepRows()[8]).toHaveTextContent("TEXT");
    expect(stepRows()[8]).toHaveAttribute("aria-expanded", "true");
    expect(textField().value).toBe("invoice_0924");
  });

  test("only TYPE steps have it", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[7]); // Ctrl + A
    expect(within(editor()).queryByRole("button", { name: "Make editable" })).toBeNull();
  });
});

describe("how a Text step shows", () => {
  test("a chip on the Keys lane, as long as the step", async () => {
    await relay.edit({ op: "make_editable", index: 8 });
    const { container } = render(Timeline);
    const chip = [...container.querySelectorAll<HTMLElement>(".chip")].find((c) => c.title === "invoice_0924")!;
    expect(parseFloat(chip.style.left)).toBeCloseTo((4050 / 10150) * 100, 3);
    expect(parseFloat(chip.style.width)).toBeCloseTo((975 / 10150) * 100, 3);
  });

  test("its text in the preview bar while it plays", async () => {
    await relay.edit({ op: "make_editable", index: 8 });
    const { container } = render(Preview);
    relay.seek(4100);
    await settle();
    expect(container.querySelector(".keys .kind")).toHaveTextContent("Typing");
    expect(container.querySelector(".keys .key")).toHaveTextContent("invoice_0924");
  });
});
