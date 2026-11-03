import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../../apps/island/static/app.js", import.meta.url), "utf8");
const { mediumCellCentre } = await import("data:text/javascript," + encodeURIComponent(source));

test("town marker centre respects row north and column east in medium-cell metres", () => {
  assert.deepEqual(mediumCellCentre([4, 12], 2000), [25000, 9000]);
  assert.deepEqual(mediumCellCentre([0, 0], 5), [2.5, 2.5]);
});

test("town marker rejects missing and invalid cells instead of drawing a false location", () => {
  for (const value of [null, undefined, [], [1], [-1, 1], [2.5, 3], [0, Infinity]]) {
    assert.equal(mediumCellCentre(value, 2000), null);
  }
  assert.equal(mediumCellCentre([1, 1], 0), null);
  assert.equal(mediumCellCentre([1, 1], -1), null);
});
