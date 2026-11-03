import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../../apps/island/static/app.js", import.meta.url), "utf8");
const { townWorkerRows } = await import("data:text/javascript," + encodeURIComponent(source));

test("town's employee roster comes from actual per-enterprise staff, not payroll totals", () => {
  const employers = [
    { name: "Gem School", staff: 1, workers: [{ person: "town-3-0", hourly_cents: 2800, hours_unpaid: 6, wages_owed_cents: 200 }] },
    { name: "Gem Health", staff: 2, workers: [
      { person: "town-1-0", hourly_cents: 4200, hours_unpaid: 1.5, wages_owed_cents: 0 },
      { person: "town-0-0", hourly_cents: 3400, hours_unpaid: 2, wages_owed_cents: 10 },
    ] },
  ];
  const before = JSON.stringify(employers);
  assert.deepEqual(townWorkerRows(employers), [
    { enterprise: "Gem Health", person: "town-0-0", hourly_cents: 3400, hours_unpaid: 2, wages_owed_cents: 10 },
    { enterprise: "Gem Health", person: "town-1-0", hourly_cents: 4200, hours_unpaid: 1.5, wages_owed_cents: 0 },
    { enterprise: "Gem School", person: "town-3-0", hourly_cents: 2800, hours_unpaid: 6, wages_owed_cents: 200 },
  ]);
  assert.equal(JSON.stringify(employers), before, "sorting must not mutate the simulation projection");
});

test("older town endpoints without per-person detail show no invented workers", () => {
  assert.deepEqual(townWorkerRows([{ name: "Gem Health", staff: 3 }]), []);
  assert.deepEqual(townWorkerRows([]), []);
});
