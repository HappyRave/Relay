import { expect, test } from "vitest";
import { core, freshStore } from "./app";

test("the store starts against the fake core", async () => {
  const relay = await freshStore();
  expect(core.subscribed).toBe(true);
  expect(relay.library).toHaveLength(4);
  expect(relay.view?.id).toBe(core.ids[0]);
  expect(relay.triggerStatus?.triggers.hotkey.combo).toBe("Ctrl + Alt + 1");
});
