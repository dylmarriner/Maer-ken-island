// The Island page: what stands here, what has been built, and everything
// that has been done to the island from outside it.
//
// Three endpoints that were served with nothing reading them. Serving data
// nobody can see is half a feature, and the half that cannot be checked by
// looking.
"use strict";

import {
  clearOffline,
  count,
  el,
  emptyState,
  fillChrome,
  getJson,
  message,
  postJson,
  reportOffline,
} from "/static/app.js";

const panels = {
  properties: document.getElementById("properties-panel"),
  economy: document.getElementById("economy-panel"),
  timeline: document.getElementById("timeline-panel"),
};
const bodies = {
  properties: document.getElementById("properties"),
  economy: document.getElementById("economy"),
  timeline: document.getElementById("timeline"),
};
const notes = {
  properties: document.getElementById("properties-note"),
  economy: document.getElementById("economy-note"),
  timeline: document.getElementById("timeline-note"),
};
const noWorldEl = document.getElementById("no-world");

/** A definition row: a label and a value. */
function pair(label, value) {
  const row = el("div", null, { class: "pair" });
  row.append(el("span", label, { class: "pair-label" }), el("span", value, { class: "pair-value" }));
  return row;
}

/** A table from a header row and rows of cells, all as text. */
function table(headings, rows) {
  const t = el("table", null, { class: "data-table" });
  const head = el("tr");
  for (const h of headings) head.append(el("th", h, { scope: "col" }));
  const thead = el("thead");
  thead.append(head);
  t.append(thead);
  const body = el("tbody");
  for (const cells of rows) {
    const tr = el("tr");
    for (const cell of cells) tr.append(el("td", cell));
    body.append(tr);
  }
  t.append(body);
  return t;
}

/** Group a property's items by kind, because eighty of them in a row is a
 *  list nobody reads. */
function byKind(items) {
  const groups = new Map();
  for (const item of items) {
    if (!groups.has(item.kind)) groups.set(item.kind, []);
    groups.get(item.kind).push(item.name);
  }
  return [...groups.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

function propertyCard(property) {
  const card = el("article", null, { class: "card" });
  card.append(el("h3", property.name));

  const where = property.on_this_island
    ? `On this island, at cell ${property.cell[0]}, ${property.cell[1]}.`
    : property.cell
      ? `At cell ${property.cell[0]}, ${property.cell[1]} — not this island's estate.`
      : "No fixed place: this is the unowned template the estate was built from.";
  card.append(el("p", where, { class: "panel-note" }));

  const owners = property.owners.length
    ? property.owners.join(" and ")
    : "Nobody — it is unowned.";
  card.append(pair("Owned by", owners));
  card.append(pair("Buildings", count(property.buildings.length, "building", "buildings")));
  card.append(pair("Things", count(property.items.length, "item", "items")));

  if (property.buildings.length) {
    const list = el("ul", null, { class: "plain-list" });
    for (const b of property.buildings) list.append(el("li", `${b.name} (${b.kind})`));
    const details = el("details");
    details.append(el("summary", "Its buildings"), list);
    card.append(details);
  }

  if (property.items.length) {
    const groups = el("div");
    for (const [kind, names] of byKind(property.items)) {
      const list = el("ul", null, { class: "plain-list" });
      for (const name of names) list.append(el("li", name));
      const details = el("details");
      details.append(el("summary", `${kind} — ${count(names.length, "item", "items")}`), list);
      groups.append(details);
    }
    const outer = el("details");
    outer.append(el("summary", "Everything on it"), groups);
    card.append(outer);
  }
  return card;
}

async function showProperties() {
  const data = await getJson("/api/properties");
  notes.properties.textContent =
    `The estate is at cell ${data.estate_cell[0]}, ${data.estate_cell[1]}, laid out in ` +
    `${count(data.spaces.length, "space", "spaces")}. ` +
    `These are read once when the island starts: nothing in the simulation moves a building yet.`;
  if (!data.properties.length) {
    bodies.properties.replaceChildren(
      emptyState("No properties", "This island has no property inventory."),
    );
  } else {
    const wrap = el("div", null, { class: "card-grid" });
    for (const property of data.properties) wrap.append(propertyCard(property));
    bodies.properties.replaceChildren(wrap);
  }
  panels.properties.hidden = false;
}

async function showEconomy() {
  const data = await getJson("/api/economy");
  // The server's own note says what is and is not here. It is shown rather
  // than paraphrased: an empty economy that does not explain itself reads
  // as a broken page.
  notes.economy.textContent = data.note;

  const parts = el("div");
  if (data.structures.length) {
    parts.append(el("h3", "Structures"));
    parts.append(
      table(
        ["#", "What", "Where", "Materials"],
        data.structures.map((s) => [
          String(s.id),
          s.recipe,
          `${s.cell[0]}, ${s.cell[1]}`,
          String(s.material_cost),
        ]),
      ),
    );
  } else {
    parts.append(
      emptyState(
        "Nothing has been built",
        "No structure stands here yet. One put up through an intervention appears in this list.",
      ),
    );
  }

  if (data.events.length) {
    parts.append(el("h3", "What the economy recorded"));
    parts.append(
      table(
        ["Tick", "Who", "What", "Thing", "How many"],
        data.events.map((e) => [
          e.tick.toLocaleString(),
          e.by,
          e.kind,
          e.subject,
          e.quantity.toLocaleString(),
        ]),
      ),
    );
  }

  const freshness = el(
    "p",
    data.at_tick === data.tick
      ? `Current as of tick ${data.tick.toLocaleString()}.`
      : `Read at tick ${data.at_tick.toLocaleString()}; the island is now at ` +
        `${data.tick.toLocaleString()}. It refreshes hourly, and at once when something is built.`,
    { class: "panel-note" },
  );
  parts.append(freshness);

  bodies.economy.replaceChildren(parts);
  panels.economy.hidden = false;
}

async function showTimeline() {
  const data = await getJson("/api/timeline");
  notes.timeline.textContent = data.note;
  if (!data.entries.length) {
    bodies.timeline.replaceChildren(
      emptyState(
        "Nothing yet",
        "Nothing has reached this island from outside. Creating somebody, intervening, " +
          "pausing or changing the speed all appear here, with the tick they landed on.",
      ),
    );
  } else {
    // Newest first: a person looking at this wants what just happened.
    const rows = [...data.entries]
      .sort((a, b) => b.tick - a.tick || b.sequence - a.sequence)
      .map((e) => [e.tick.toLocaleString(), String(e.sequence), e.what]);
    bodies.timeline.replaceChildren(table(["Tick", "Order", "What"], rows));
  }
  panels.timeline.hidden = false;
}

async function load() {
  try {
    const status = await getJson("/api/status");
    fillChrome(status);
  } catch (error) {
    // The footer is decoration; a failure here must not stop the page.
  }
  try {
    const world = await getJson("/api/world");
    clearOffline();
    if (!world.running) {
      noWorldEl.hidden = false;
      return;
    }
    noWorldEl.hidden = true;
    document.getElementById("intervene-panel").hidden = false;
    await Promise.all([
      showProperties(),
      showEconomy(),
      showTimeline(),
      fillPlacesAndPeople(),
    ]);
  } catch (error) {
    if (error && error.status === 404) {
      noWorldEl.hidden = false;
      return;
    }
    reportOffline(error);
    for (const key of Object.keys(bodies)) {
      if (bodies[key]) message(bodies[key], "Could not read this from the island.", "hint");
    }
  }
}

load();
// The economy and the timeline change when somebody does something, so the
// page follows along rather than needing a reload.
setInterval(() => {
  load().catch(() => {});
}, 5000);

// --- Reaching into the world -------------------------------------------
//
// The intervention endpoint had no UI at all: it was reachable only by
// curl, which is not a control surface. This offers the actions the island
// can honour, plus one it cannot, because an operator should be able to see
// a refusal as easily as a success — the refusals are the part of this
// feature that is easy to claim and hard to believe.

const form = document.getElementById("intervene-form");
const actionEl = document.getElementById("action");
const resultEl = document.getElementById("intervene-result");
const hintEl = document.getElementById("intervene-hint");
const whereEl = document.getElementById("where");
const personEl = document.getElementById("person");

/// Which fields each action needs, and what the amount means for it.
const SHAPES = {
  InjectResource: {
    fields: ["amount", "where"],
    amount: ["How much water (kg)", "Water is the one material the island and upstream both name."],
  },
  InjectBiomass: {
    fields: ["amount", "radius", "where"],
    amount: [
      "How much plant carbon (kgC)",
      "Spread evenly over the land in the region. The sea gets none.",
    ],
  },
  ModifyClimate: {
    fields: ["amount", "radius", "where"],
    amount: [
      "Surface temperature (K)",
      "An absolute target, not an offset. 288 K is about 15 °C. The heat is booked as coming " +
        "from outside the island, and relaxes on the climate's own timescale.",
    ],
  },
  ConstructStructure: {
    fields: ["structure", "where"],
    amount: null,
  },
  RemoveHuman: {
    fields: ["person"],
    amount: null,
  },
  SculptTerrain: {
    fields: ["amount", "radius", "where"],
    amount: [
      "Metres of relief",
      "The island will refuse this: its terrain comes from the canon-locked scenario and is " +
        "hashed into the state digest, so editing it would put the island out of agreement " +
        "with its own canon. It is here so a refusal can be seen rather than taken on trust.",
    ],
  },
};

const FIELD_IDS = ["structure", "amount", "person", "radius", "where"];

function shapeForm() {
  const shape = SHAPES[actionEl.value];
  for (const id of FIELD_IDS) {
    const field = document.getElementById(id);
    const label = document.getElementById(`${id}-label`);
    const wanted = shape.fields.includes(id);
    // Both the control and its label, because a label with nothing under it
    // is a control that does nothing — which this dashboard has shipped
    // once already.
    if (field) field.hidden = !wanted;
    if (label) label.hidden = !wanted;
  }
  if (shape.amount) {
    document.getElementById("amount-label").textContent = shape.amount[0];
    hintEl.textContent = shape.amount[1];
  } else {
    hintEl.textContent =
      actionEl.value === "RemoveHuman"
        ? "They leave the registry, the estate, the material ledger and the running tallies. " +
          "A removal is not a death: their body leaves the island rather than becoming soil."
        : "";
  }
}

/// The request body, in the engine's own vocabulary.
function requestFor(place) {
  const amount = Number(document.getElementById("amount").value);
  const radius = Number(document.getElementById("radius").value);
  const location = { latitude: place.latitude, longitude: place.longitude, altitude: null };
  const region = { center: location, radius_km: radius };
  switch (actionEl.value) {
    case "InjectResource":
      return { InjectResource: { resource_type: "Water", amount, location } };
    case "InjectBiomass":
      return { InjectBiomass: { biomass_type: "Producers", amount, region } };
    case "ModifyClimate":
      return { ModifyClimate: { parameter: "Temperature", value: amount, region } };
    case "ConstructStructure":
      return {
        ConstructStructure: { structure: document.getElementById("structure").value, location },
      };
    case "RemoveHuman":
      return { RemoveHuman: { human_id: personEl.value } };
    case "SculptTerrain":
      return { SculptTerrain: { elevation_delta_m: amount, region } };
    default:
      return null;
  }
}

let places = [];

async function fillPlacesAndPeople() {
  const world = await getJson("/api/world");
  places = [
    {
      label: `The estate — cell ${world.estate.cell[0]}, ${world.estate.cell[1]}`,
      latitude: world.estate.latitude,
      longitude: world.estate.longitude,
    },
  ];
  whereEl.replaceChildren(
    ...places.map((p, i) => el("option", p.label, { value: String(i) })),
  );
  personEl.replaceChildren(
    ...world.people.map((p) => el("option", p.agent_id, { value: p.agent_id })),
  );
}

/// Poll a queued command until the island has answered it.
async function settle(id) {
  const deadline = Date.now() + 60_000;
  for (;;) {
    const answer = await getJson(`/api/world/commands/${id}`);
    if (answer.state !== "queued") return answer;
    if (Date.now() > deadline) return { state: "unknown" };
    await new Promise((resume) => setTimeout(resume, 400));
  }
}

function show(text, kind) {
  resultEl.hidden = false;
  resultEl.className = `banner${kind ? " " + kind : ""}`;
  resultEl.textContent = text;
}

if (form) {
  actionEl.addEventListener("change", shapeForm);
  shapeForm();
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const place = places[Number(whereEl.value) || 0];
    if (!place && SHAPES[actionEl.value].fields.includes("where")) {
      show("There is nowhere to aim this: the island has not been read yet.", "is-error");
      return;
    }
    const body = requestFor(place || {});
    show("Asking the island…");
    const { status, body: answer } = await postJson("/api/world/interventions", body, "");
    if (status !== 202) {
      show(
        (answer.errors && answer.errors.join(" ")) || `The island answered ${status}.`,
        "is-error",
      );
      return;
    }
    const outcome = await settle(answer.command);
    if (outcome.state === "intervened") {
      show(outcome.summary, "is-ok");
      await load();
    } else if (outcome.state === "refused") {
      // A refusal is the island saying what it is, not an error on the way
      // in, so it reads as a statement rather than a failure.
      show(outcome.problems.join(" "), "is-warn");
    } else {
      show("The island did not answer in time.", "is-error");
    }
  });
}
