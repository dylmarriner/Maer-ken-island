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
    await Promise.all([showProperties(), showEconomy(), showTimeline()]);
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
