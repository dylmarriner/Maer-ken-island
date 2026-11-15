import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

// The dashboard ships an ES module without a Node package.json.
// Import its actual source as an ES module to exercise the shipped helper.
const script = await readFile(new URL("../../apps/island/static/app.js", import.meta.url), "utf8");
const { readIfAvailable } = await import("data:text/javascript," + encodeURIComponent(script));

test("an older backend's missing optional endpoint does not fail the island", async () => {
  const result = await readIfAvailable(async (path) => {
    assert.equal(path, "/api/town");
    throw Object.assign(new Error("not found"), { status: 404 });
  }, "/api/town");
  assert.equal(result, null);
});

test("a working optional endpoint returns the real payload", async () => {
  const expected = { town: { founded: true, residents: 8 }, tick: 12 };
  assert.deepEqual(await readIfAvailable(async () => expected, "/api/town"), expected);
});

test("authentication, backend faults and network failures are not hidden as old-server compatibility", async () => {
  for (const failure of [Object.assign(new Error("unauthorised"), { status: 401 }),
                         Object.assign(new Error("failed"), { status: 500 }),
                         new TypeError("network offline")]) {
    await assert.rejects(readIfAvailable(async () => { throw failure; }, "/api/town"),
                         (error) => error === failure);
  }
});
