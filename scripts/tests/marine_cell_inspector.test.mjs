import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../../apps/island/static/app.js", import.meta.url), "utf8");
const { handleMapCellClick } = await import("data:text/javascript," + encodeURIComponent(source));

class FakeElement {
  constructor(tag) {
    this.tagName = tag.toUpperCase();
    this.attributes = {};
    this.textContent = "";
  }

  setAttribute(name, value) {
    this.attributes[name] = String(value);
  }
}

async function withDocument(run) {
  const previous = globalThis.document;
  globalThis.document = { createElement: (tag) => new FakeElement(tag) };
  try {
    await run();
  } finally {
    if (previous === undefined) delete globalThis.document;
    else globalThis.document = previous;
  }
}

function clickFixture(cell) {
  const output = {
    children: [],
    replaceChildren(...children) {
      this.children = children;
    },
  };
  const whereEl = { value: "" };
  const places = [];
  let clickedPlace = null;
  const requested = [];
  const dependencies = {
    canvas: { getBoundingClientRect: () => ({ left: 5, top: 7 }) },
    output,
    viewReady: true,
    ground: { cellM: 10, cols: 10, rows: 10 },
    toMetres: (_canvas, x, y) => [x, y],
    getJson: async (path) => {
      requested.push(path);
      return { cell };
    },
    setClickedPlace: (place) => { clickedPlace = place; },
    fillPlacesAndPeople: async () => { places.push(clickedPlace); },
    getPlaces: () => places,
    whereEl,
  };
  return { output, whereEl, places, requested, dependencies, get clickedPlace() { return clickedPlace; } };
}

test("clicking a marine map cell renders the estimate and disclaimer in the inspector DOM", async () => {
  await withDocument(async () => {
    const cell = {
      row: 2,
      col: 3,
      latitude: -41.2,
      longitude: 174.8,
      land: false,
      elevation_m: -25,
      buildable: false,
      marine_aquatic_species: 4,
      marine_aquatic_individuals: 12345,
      marine_aquatic_biomass_t: 67.89,
    };
    const fixture = clickFixture(cell);
    await handleMapCellClick({ clientX: 40, clientY: 32 }, fixture.dependencies);

    assert.deepEqual(fixture.requested, ["/api/cell/2/3"]);
    assert.equal(fixture.output.children.length, 3);
    assert.equal(fixture.output.children[0].attributes.class, "cell-where");
    assert.equal(fixture.output.children[1].attributes.class, "cell-what");
    const marine = fixture.output.children[2];
    assert.equal(marine.attributes.class, "cell-marine");
    assert.ok(marine.textContent.includes("4 marine aquatic species"));
    assert.ok(marine.textContent.includes(
      cell.marine_aquatic_individuals.toLocaleString(undefined, { maximumSignificantDigits: 4 }),
    ));
    assert.ok(marine.textContent.includes(
      cell.marine_aquatic_biomass_t.toLocaleString(undefined, { maximumSignificantDigits: 4 }),
    ));
    assert.ok(marine.textContent.includes(
      "Uncalibrated upstream estimate; not harvestable stock or catch.",
    ));
    assert.deepEqual(fixture.clickedPlace, {
      label: "Clicked — cell 2, 3 (sea)",
      latitude: -41.2,
      longitude: 174.8,
    });
    assert.equal(fixture.whereEl.value, "0");
  });
});

test("negative, non-finite or unsafe marine projections are omitted without hiding ordinary cell details", async () => {
  await withDocument(async () => {
    const invalidProjections = [
      { marine_aquatic_species: Number.MAX_SAFE_INTEGER + 1, marine_aquatic_individuals: 1, marine_aquatic_biomass_t: 1 },
      { marine_aquatic_species: -1, marine_aquatic_individuals: 1, marine_aquatic_biomass_t: 1 },
      { marine_aquatic_species: 1, marine_aquatic_individuals: -1, marine_aquatic_biomass_t: 1 },
      { marine_aquatic_species: 1, marine_aquatic_individuals: 1, marine_aquatic_biomass_t: -1 },
      { marine_aquatic_species: 1, marine_aquatic_individuals: Infinity, marine_aquatic_biomass_t: 1 },
      { marine_aquatic_species: 1, marine_aquatic_individuals: 1, marine_aquatic_biomass_t: NaN },
    ];

    for (const projection of invalidProjections) {
      const fixture = clickFixture({
        row: 1,
        col: 1,
        latitude: -40,
        longitude: 175,
        land: true,
        elevation_m: 12,
        buildable: true,
        ...projection,
      });
      await handleMapCellClick({ clientX: 40, clientY: 32 }, fixture.dependencies);
      assert.equal(fixture.output.children.length, 2);
      assert.equal(fixture.output.children[1].attributes.class, "cell-what");
      assert.equal(fixture.whereEl.value, "0");
    }
  });
});

test("map clicks before the view is ready do not request or render a cell", async () => {
  await withDocument(async () => {
    const fixture = clickFixture({});
    fixture.dependencies.viewReady = false;
    await handleMapCellClick({ clientX: 40, clientY: 32 }, fixture.dependencies);
    assert.deepEqual(fixture.requested, []);
    assert.deepEqual(fixture.output.children, []);
  });
});
