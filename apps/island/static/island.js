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
  storedToken,
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

/// The structures the economy last reported, kept so the map can draw them
/// without asking for them a second time.
let lastStructures = [];

async function showEconomy() {
  const data = await getJson("/api/economy");
  lastStructures = data.structures;
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

// The island, drawn, and zoomable from the whole 2,400 km of it down to a
// single trunk. Everything else on this page describes the world in words
// and tables; `docs/ISLAND_SYSTEM_STATUS.md` has called the missing
// picture what Task 6 still lacks.
//
// The thing to understand before reading any of it: what is drawn is not
// drawn at one resolution. A ground layer under `layers` below, the estate
// and the people at points, and -- only in a wood a kilometre across --
// about 200,000 individual stems. Zooming in improves the picture in
// place, from 2 km cells to 5 m cells to single trunks, and the key says
// at every zoom which of those you are looking at. That is the honest part
// of this viewer: empty ground here is very often ground the island models
// statistically rather than ground with nothing on it.
//
// Everything is held in domain metres, which is the frame the engine
// itself uses and the frame `/api/trees` answers in. Cells are converted
// into metres on the way in rather than the other way round, so the one
// conversion that can be wrong is written once.

// Where the viewer is looking: the centre in domain metres, and how many
// metres one CSS pixel covers. `mpp` is the whole of the zoom.
const view = { cx: 0, cy: 0, mpp: 0, ready: false };

// The island's own dimensions, read from `/api/world` the first time it
// arrives. Nothing can be drawn before these are known.
const ground = { rows: 0, cols: 0, cellM: 0, widthM: 0, heightM: 0 };

// The ground layers. Three pictures of the same island at three
// resolutions, and which of them is drawn is the whole of the zoom story
// below the stems:
//
//   * elevation, one pixel per 2 km cell, drawn once because terrain
//     cannot change -- the island refuses `SculptTerrain` and
//     `SmoothTerrain`;
//   * the island's standing vegetation, also 2 km cells, but redrawn as
//     biomass grows;
//   * the estate's 4 km patch at its own 5 m cells, four hundred times
//     finer over the ground the founders live on.
//
// The patch is drawn over the island layer rather than instead of it, so
// zooming in improves the resolution in place instead of swapping one
// picture for another.
const layers = {
  elevation: { src: "/api/map.png", image: null, ready: false },
  vegetation: { src: "/api/vegetation.png", image: null, ready: false },
  patch: { src: "/api/patch.png", image: null, ready: false },
};
let layerShown = "elevation";

// The last answer from `/api/trees`, kept so a redraw mid-pan has stems to
// draw instead of blinking empty, and so the patch geometry it carries --
// the wood's centre and radius, the estate's yard -- is to hand.
let treesShown = null;
let treeFetch = null;
let treeTimer = null;
let treeAgain = false;

// What the map draws on top of the ground, refreshed by the poll loop.
let mapWorld = null;

// How close you must be before asking for trees at all. Below this the
// whole individually-modelled wood is a couple of pixels across, and four
// thousand dots would land on top of each other for no gain.
const TREES_VISIBLE_PX = 60;

// The hard floor on zoom: two centimetres to the pixel shows a trunk as a
// trunk. Past that there is nothing further to resolve, because a stem is
// a position, a height and a diameter and not a shape.
const CLOSEST_MPP = 0.02;

function mapCanvas() {
  return document.getElementById("map");
}

// Screen pixels per CSS pixel, so the picture is sharp on a retina display
// and the trunk-sized dots are not a blurry smear.
function pixelRatio() {
  return Math.min(window.devicePixelRatio || 1, 2);
}

// Metres to CSS pixels and back. y is flipped: the engine's y grows north
// and a screen's grows down. This is the same flip `island_preview`'s
// `grid_image` applies when it writes grid row `r` to image row
// `rows - 1 - r`, and getting it wrong would mirror the island about the
// equator with nothing on screen looking wrong.
function toScreen(canvas, x, y) {
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  return [w / 2 + (x - view.cx) / view.mpp, h / 2 - (y - view.cy) / view.mpp];
}

function toMetres(canvas, sx, sy) {
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  return [view.cx + (sx - w / 2) * view.mpp, view.cy - (sy - h / 2) * view.mpp];
}

// The zoom at which the whole island just fits.
function fitMpp(canvas) {
  const w = canvas.clientWidth || 1;
  const h = canvas.clientHeight || 1;
  return Math.max(ground.widthM / w, ground.heightM / h);
}

function clampView(canvas) {
  const fit = fitMpp(canvas);
  view.mpp = Math.min(Math.max(view.mpp, CLOSEST_MPP), fit);
  // Pan is bounded by the island itself: you can put any part of it in the
  // middle of the frame but you cannot lose it off the edge entirely.
  view.cx = Math.min(Math.max(view.cx, 0), ground.widthM);
  view.cy = Math.min(Math.max(view.cy, 0), ground.heightM);
}

// The box now in frame, in domain metres.
function viewBox(canvas) {
  const [x0, y1] = toMetres(canvas, 0, 0);
  const [x1, y0] = toMetres(canvas, canvas.clientWidth, canvas.clientHeight);
  return { x0, y0, x1, y1 };
}

// Ask for the stems in frame once the view has settled.
//
// Debounced rather than throttled, and that distinction is the whole
// reason this is five lines longer than it looks like it should be. A
// throttle *drops* the calls that arrive too soon, so the last one -- the
// one that knows where the viewer actually ended up -- is the one thrown
// away: three quick taps on "zoom out" left the map drawing the stems from
// a frame three zooms ago and a key confidently describing them. A
// debounce delays instead, so the last call is the one that runs.
//
// `treeAgain` covers the other half of it: a request already in flight
// cannot be cancelled, so a view that moves while one is out sets this and
// the fetch asks again on its way out.
function wantTrees(canvas) {
  if (!view.ready) return;
  if (treeTimer) clearTimeout(treeTimer);
  treeTimer = setTimeout(() => {
    treeTimer = null;
    fetchTrees(canvas);
  }, 180);
}

async function fetchTrees(canvas) {
  if (treeFetch) {
    treeAgain = true;
    return;
  }
  const box = viewBox(canvas);
  // A cap that follows the zoom. Four thousand stems is the right answer
  // when the wood fills the frame and a waste of 400 kB when it is six
  // pixels wide and every dot lands on the same six.
  const across = (2 * (treesShown ? treesShown.individual_radius_m : 1000)) / view.mpp;
  const cap = across < TREES_VISIBLE_PX ? 200 : 4000;
  treeFetch = (async () => {
    try {
      const data = await getJson(
        `/api/trees?x0=${box.x0.toFixed(1)}&y0=${box.y0.toFixed(1)}` +
          `&x1=${box.x1.toFixed(1)}&y1=${box.y1.toFixed(1)}&cap=${cap}`,
      );
      treesShown = data.trees;
      draw();
    } catch (error) {
      // A map without its trees is still a map, and the key says so.
    } finally {
      treeFetch = null;
      if (treeAgain) {
        treeAgain = false;
        wantTrees(canvas);
      }
    }
  })();
}

// What a stem is drawn as.
//
// The radius is the stem's own diameter, not a crown: the island models a
// trunk's diameter and a height and has no crown spread at all, so drawing
// a canopy would be drawing something the simulation does not have. A one
// and a bit pixel floor keeps a 30 cm trunk visible at a zoom where it is
// a fifth of a pixel wide; the key says that is what you are seeing.
function stemRadiusPx(tree) {
  return Math.max(1.3, tree.stem_diameter_m / 2 / view.mpp);
}

// Colour by what the engine calls it, which is `PlantKind` -- Tree, Shrub
// or Grass -- and is not a species. The island's vegetation carries no
// species at all; the engine's species system is not wired into it.
const KIND_COLOUR = {
  Tree: "#3f9f54",
  Shrub: "#7fbe4a",
  Grass: "#b9d169",
};

function draw() {
  const canvas = mapCanvas();
  if (!canvas || !view.ready) return;
  const ratio = pixelRatio();
  const w = canvas.clientWidth;
  const h = canvas.clientHeight;
  if (!w || !h) return;
  if (canvas.width !== Math.round(w * ratio) || canvas.height !== Math.round(h * ratio)) {
    canvas.width = Math.round(w * ratio);
    canvas.height = Math.round(h * ratio);
  }
  const ctx = canvas.getContext("2d");
  ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
  ctx.clearRect(0, 0, w, h);
  ctx.fillStyle = "#0b1016";
  ctx.fillRect(0, 0, w, h);

  // The ground. Drawn by placing the whole render at the corners of the
  // domain, so the browser does the scaling and this code never has to
  // work out a source rectangle. Smoothing off once a cell is bigger than
  // a pixel: an interpolated blur would quietly imply a resolution the
  // elevation grid has not got.
  const island = layers[layerShown];
  if (island.ready) {
    const [sx0, sy0] = toScreen(canvas, 0, ground.heightM);
    const [sx1, sy1] = toScreen(canvas, ground.widthM, 0);
    ctx.imageSmoothingEnabled = ground.cellM / view.mpp < 1;
    ctx.drawImage(island.image, sx0, sy0, sx1 - sx0, sy1 - sy0);
  }

  // The patch, over the top, at four hundred times the resolution. Only
  // on the vegetation layer: there is no 5 m elevation to go with it, and
  // drawing vegetation over an elevation map would be two things at once.
  if (layerShown === "vegetation" && layers.patch.ready && treesShown && treesShown.patch) {
    const [px, py, pw, ph] = treesShown.patch;
    const [sx0, sy0] = toScreen(canvas, px, py + ph);
    const [sx1, sy1] = toScreen(canvas, px + pw, py);
    ctx.imageSmoothingEnabled = (pw / layers.patch.image.naturalWidth) / view.mpp < 1;
    ctx.drawImage(layers.patch.image, sx0, sy0, sx1 - sx0, sy1 - sy0);
  }

  // The individually-modelled wood, outlined, so somebody looking at an
  // empty island can see where the trees are before they find them; and
  // the estate's yard, outlined too, because it is a hole in that wood and
  // an unexplained empty square in the middle of a forest reads as a bug.
  if (treesShown) {
    if (treesShown.yard) {
      const [w, so, e, no] = treesShown.yard;
      const [yx0, yy0] = toScreen(canvas, w, no);
      const [yx1, yy1] = toScreen(canvas, e, so);
      if (yx1 - yx0 > 3) {
        ctx.strokeStyle = "rgba(255, 255, 255, 0.5)";
        ctx.lineWidth = 1;
        ctx.setLineDash([3, 3]);
        ctx.strokeRect(yx0, yy0, yx1 - yx0, yy1 - yy0);
        ctx.setLineDash([]);
      }
    }
    const [cxs, cys] = toScreen(canvas, treesShown.individual_centre_m[0], treesShown.individual_centre_m[1]);
    ctx.strokeStyle = "rgba(90, 220, 130, 0.55)";
    ctx.lineWidth = 1.5;
    ctx.setLineDash([5, 4]);
    ctx.beginPath();
    ctx.arc(cxs, cys, treesShown.individual_radius_m / view.mpp, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);

    for (const tree of treesShown.trees) {
      const [sx, sy] = toScreen(canvas, tree.x_m, tree.y_m);
      if (sx < -8 || sy < -8 || sx > w + 8 || sy > h + 8) continue;
      ctx.fillStyle = KIND_COLOUR[tree.kind] || "#6aa84f";
      ctx.beginPath();
      ctx.arc(sx, sy, stemRadiusPx(tree), 0, Math.PI * 2);
      ctx.fill();
    }
  }

  if (!mapWorld) {
    sayScale(canvas);
    return;
  }

  // The estate: a box round its cell rather than a dot, because it is a
  // place and not a thing standing at a point. One cell is 2 km, and at
  // island zoom that is a pixel, so the box never shrinks below something
  // you can see.
  if (mapWorld.estate && mapWorld.estate.cell) {
    const [r, c] = mapWorld.estate.cell;
    const [sx, sy] = toScreen(canvas, (c + 0.5) * ground.cellM, (r + 0.5) * ground.cellM);
    const side = Math.max(12, ground.cellM / view.mpp);
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 2;
    ctx.strokeRect(sx - side / 2, sy - side / 2, side, side);
  }

  for (const s of lastStructures) {
    const [r, c] = s.cell;
    const [sx, sy] = toScreen(canvas, (c + 0.5) * ground.cellM, (r + 0.5) * ground.cellM);
    ctx.fillStyle = "#e8663c";
    ctx.strokeStyle = "#4a1a08";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(sx, sy, 5, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }

  // People last, so somebody standing on a structure is still visible.
  //
  // Drawn at their own metres where the island has them, and only at the
  // centre of their cell where it does not. At estate zoom that is the
  // difference between four people in a house and four people stacked on
  // one 2 km dot.
  for (const p of mapWorld.people) {
    if (!p.alive) continue;
    let at = null;
    if (p.position_m) at = p.position_m;
    else if (p.cell) at = [(p.cell[1] + 0.5) * ground.cellM, (p.cell[0] + 0.5) * ground.cellM];
    if (!at) continue;
    const [sx, sy] = toScreen(canvas, at[0], at[1]);
    ctx.fillStyle = "#ffd34d";
    ctx.strokeStyle = "#3a2c00";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.arc(sx, sy, 4.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }

  sayScale(canvas);
}

// How wide the frame is on the ground, and what it is therefore showing.
// Said in words because three coloured dots and a dashed circle mean
// nothing on their own, and because a screen reader gets none of the
// picture at all.
// Fetch one ground layer. The elevation never changes and the other two
// are served `no-cache`, so the browser revalidates them rather than
// holding the island's first forest for ever; this loads each once a
// session and leaves freshness to that header.
function loadLayer(layer) {
  layer.image = new Image();
  layer.image.addEventListener("load", () => {
    layer.ready = true;
    draw();
  });
  layer.image.src = layer.src;
}

// What the map is, in a sentence, following whichever layer is on. The
// two layers are two different pictures of the island and say so: one is
// terrain that cannot change, the other is a quantity that grows.
function sayNote() {
  document.getElementById("map-note").textContent =
    layerShown === "vegetation"
      ? "The island's standing vegetation, redrawn as it grows. Scroll or use the buttons to zoom, drag to pan, click to read a cell. Zooming in improves the picture in place: 2 km cells across the island, 5 m cells over the estate's patch, and individual trees in the wood around the estate."
      : "Elevation, with the coastline and the domain's edge buffer, at one pixel per 2 km cell. Drawn once from the terrain this island was generated with, which nothing can change: the island refuses requests to sculpt or smooth it. Scroll or use the buttons to zoom, drag to pan, click to read a cell and fill the intervention form's coordinates.";
}

function metres(m) {
  return m >= 2000 ? `${(m / 1000).toFixed(1)} km` : `${m.toFixed(0)} m`;
}

function sayScale(canvas) {
  const acrossM = canvas.clientWidth * view.mpp;
  const across = metres(acrossM);
  document.getElementById("map-scale").textContent = `${across} across`;

  const key = document.getElementById("map-key");
  const alive = mapWorld ? mapWorld.people.filter((p) => p.alive && (p.position_m || p.cell)).length : 0;
  // The patch's own resolution, read off the render rather than assumed:
  // the engine sizes that grid from the scenario, and a page that said
  // "5 m" because the default scenario says so would be lying the moment
  // somebody changed it.
  const patchM = treesShown && treesShown.patch ? treesShown.patch[2] : 0;
  const patchCells = layers.patch.ready ? layers.patch.image.naturalWidth : 0;
  const bits = [
    el(
      "span",
      layerShown === "vegetation"
        ? `Standing producer carbon, pale sand for bare ground through to dark green at 20 kgC per square metre, on a square-root scale: the island's median cell carries 0.8 and its densest 18.6, so a straight scale would draw almost all of it one colour. One pixel per ${metres(ground.cellM)} cell across the island` +
          (patchCells
            ? `, and per ${metres(patchM / patchCells)} cell over the estate's ${metres(patchM)} patch. `
            : ". ")
        : `Elevation, at one pixel per ${metres(ground.cellM)} cell. `,
    ),
    el("span", `${count(alive, "person", "people")}`, { class: "map-key-person" }),
    document.createTextNode(" in yellow, "),
    el("span", `${count(lastStructures.length, "structure", "structures")}`, { class: "map-key-structure" }),
    document.createTextNode(" in orange, the estate boxed in white."),
  ];
  const where =
    treesShown &&
    ` Individual stems exist only inside the dashed circle, ${metres(treesShown.individual_radius_m)} around the estate, where the island holds ${treesShown.total.toLocaleString()} of them; the dashed rectangle inside it is the estate's yard, which is cleared, so the engine puts no stem there at all. Beyond the circle vegetation is stand cover per cell, so bare ground on this map is not bare ground.` +
    (layerShown === "vegetation"
      ? // Said because the circle draws paler than the ground around it,
        // and a paler patch in a forest reads as a clearing. It is the
        // opposite: the carbon is there, it is just in the stems.
        ` The ground inside the circle is drawn paler than the forest around it because the stems' own carbon has been taken out of the stand cover and put into the trees you can see — the same carbon, counted once.`
      : "");
  if (treesShown && treesShown.shown) {
    const thinned = treesShown.in_box > treesShown.shown;
    bits.push(
      document.createTextNode(
        ` ${treesShown.shown.toLocaleString()} ` +
          (thinned
            ? `of the ${treesShown.in_box.toLocaleString()} stems in frame, thinned to about one in ${Math.round(treesShown.in_box / treesShown.shown)} — zoom in until the two numbers agree and every stem in view is real.`
            : `stems in frame, which is every one of them.`) +
          ` Each is drawn at its own trunk diameter, with a floor so a narrow one stays visible; the island models a height and a diameter and no crown, so there is no canopy to draw. Green is a tree, lighter green a shrub, lightest a grass — that is the engine's whole vocabulary for a plant, and it is not a species.` +
          where,
      ),
    );
  } else if (treesShown) {
    // Said rather than left blank: an empty frame inside a wood of two
    // hundred thousand stems is either the yard or the ground beyond the
    // radius, and both of those are the island rather than a broken page.
    bits.push(document.createTextNode(` No individual stems stand in this frame.` + where));
  } else {
    bits.push(
      document.createTextNode(
        " Zoom in on the estate to see the island's individual trees: they are modelled only in a wood around it, and everywhere else vegetation is stand cover per cell rather than stems.",
      ),
    );
  }
  key.replaceChildren(...bits);
}

async function onMapClick(event) {
  const canvas = mapCanvas();
  const out = document.getElementById("map-cell");
  if (!view.ready) return;
  const box = canvas.getBoundingClientRect();
  const [x, y] = toMetres(canvas, event.clientX - box.left, event.clientY - box.top);
  const col = Math.floor(x / ground.cellM);
  const row = Math.floor(y / ground.cellM);
  if (col < 0 || row < 0 || col >= ground.cols || row >= ground.rows) return;
  try {
    const data = await getJson(`/api/cell/${row}/${col}`);
    const c = data.cell;
    out.replaceChildren(
      el("p", `Row ${c.row}, column ${c.col} — ${c.latitude.toFixed(4)}°, ${c.longitude.toFixed(4)}°`, { class: "cell-where" }),
      el(
        "p",
        c.land
          ? `Land, ${c.elevation_m.toFixed(0)} m above sea level. ${c.buildable ? "A structure could stand here." : "Too steep to build on."}`
          : `Sea, ${(-c.elevation_m).toFixed(0)} m deep. Nobody can be created here and nothing can be built.`,
        { class: "cell-what" },
      ),
    );
    // The point of clicking. Before this the intervention form offered
    // exactly one place -- the estate -- so the whole rest of the island
    // was unreachable from the page. A clicked cell becomes a place, and
    // is selected.
    clickedPlace = {
      label: `Clicked — cell ${c.row}, ${c.col}${c.land ? "" : " (sea)"}`,
      latitude: c.latitude,
      longitude: c.longitude,
    };
    await fillPlacesAndPeople();
    whereEl.value = String(places.findIndex((p) => p === clickedPlace));
  } catch (error) {
    out.replaceChildren(el("p", explain(error), { class: "cell-what" }));
  }
}

// Zoom about the pointer rather than the centre, so the thing under the
// cursor stays under it. Zooming about the centre makes finding a stand of
// trees a matter of zoom, overshoot, pan back, zoom again.
function zoomAt(canvas, factor, sx, sy) {
  const [bx, by] = toMetres(canvas, sx, sy);
  view.mpp *= factor;
  clampView(canvas);
  const [ax, ay] = toMetres(canvas, sx, sy);
  view.cx += bx - ax;
  view.cy += by - ay;
  clampView(canvas);
  draw();
  wantTrees(canvas);
}

function goTo(x, y, mpp) {
  const canvas = mapCanvas();
  view.cx = x;
  view.cy = y;
  view.mpp = mpp;
  clampView(canvas);
  draw();
  wantTrees(canvas);
}

let mapWired = false;

function wireMap() {
  if (mapWired) return;
  const canvas = mapCanvas();

  canvas.addEventListener("wheel", (event) => {
    if (!view.ready) return;
    event.preventDefault();
    const box = canvas.getBoundingClientRect();
    zoomAt(canvas, Math.exp(event.deltaY * 0.0015), event.clientX - box.left, event.clientY - box.top);
  }, { passive: false });

  // Drag to pan, and a drag is not a click: without the four-pixel test
  // every pan would also select whatever cell it finished on.
  let dragging = null;
  canvas.addEventListener("pointerdown", (event) => {
    if (!view.ready) return;
    canvas.setPointerCapture(event.pointerId);
    dragging = { x: event.clientX, y: event.clientY, moved: 0 };
    canvas.classList.add("map-dragging");
  });
  canvas.addEventListener("pointermove", (event) => {
    if (!dragging) return;
    const dx = event.clientX - dragging.x;
    const dy = event.clientY - dragging.y;
    dragging.moved += Math.abs(dx) + Math.abs(dy);
    dragging.x = event.clientX;
    dragging.y = event.clientY;
    view.cx -= dx * view.mpp;
    view.cy += dy * view.mpp;
    clampView(canvas);
    draw();
  });
  const letGo = (event) => {
    if (!dragging) return;
    const moved = dragging.moved;
    dragging = null;
    canvas.classList.remove("map-dragging");
    if (moved > 4) wantTrees(canvas);
    else onMapClick(event);
  };
  canvas.addEventListener("pointerup", letGo);
  canvas.addEventListener("pointercancel", () => {
    dragging = null;
    canvas.classList.remove("map-dragging");
  });

  // Buttons as well as the wheel, because a wheel is not reachable from a
  // keyboard and "zoom all the way in" should not need a mouse.
  const mid = () => [canvas.clientWidth / 2, canvas.clientHeight / 2];
  document.getElementById("map-in").addEventListener("click", () => zoomAt(canvas, 0.5, ...mid()));
  document.getElementById("map-out").addEventListener("click", () => zoomAt(canvas, 2, ...mid()));
  document.getElementById("map-whole").addEventListener("click", () =>
    goTo(ground.widthM / 2, ground.heightM / 2, fitMpp(canvas)),
  );
  document.getElementById("map-estate").addEventListener("click", () => {
    if (!mapWorld || !mapWorld.estate || !mapWorld.estate.cell) return;
    const [r, c] = mapWorld.estate.cell;
    // Four kilometres across: the whole high-detail patch in frame.
    goTo((c + 0.5) * ground.cellM, (r + 0.5) * ground.cellM, 4000 / (canvas.clientWidth || 1));
  });
  document.getElementById("map-trees").addEventListener("click", () => {
    // Aimed at a stem that is actually there, not at the middle of the
    // wood. The individual radius is measured from the centre of the
    // estate's yard, and the yard is cleared, so "the middle of the wood"
    // is a clearing: this button used to drop you into forty metres of
    // empty grass and look broken.
    if (!treesShown || !treesShown.trees.length) return;
    const [cx, cy] = treesShown.individual_centre_m;
    let best = treesShown.trees[0];
    let near = Infinity;
    for (const t of treesShown.trees) {
      const d = (t.x_m - cx) ** 2 + (t.y_m - cy) ** 2;
      if (d < near) {
        near = d;
        best = t;
      }
    }
    // Forty metres across: close enough that the stems stop being thinned
    // and each one is drawn at its own trunk diameter.
    goTo(best.x_m, best.y_m, 40 / (canvas.clientWidth || 1));
  });

  document.getElementById("map-layer").addEventListener("change", (event) => {
    layerShown = event.target.value;
    sayNote();
    draw();
  });

  window.addEventListener("resize", () => {
    if (!view.ready) return;
    clampView(canvas);
    draw();
  });
  mapWired = true;
}

// Show the map, and start it looking at the whole island.
//
// `world` is the projection the poll loop just read; the map needs its
// grid shape and its people, and takes both from there rather than asking
// for them again.
function showMap(world) {
  const canvas = mapCanvas();
  mapWorld = world;
  sayNote();
  // Unhidden before anything is measured: a hidden canvas has no width,
  // and the opening zoom is "the whole island fits", which is a division
  // by that width. Working it out first put the viewer at a zoom of the
  // entire domain per pixel and showed a blank frame.
  document.getElementById("map-panel").hidden = false;
  if (!view.ready && world && world.land && world.land.cols) {
    ground.rows = world.land.rows;
    ground.cols = world.land.cols;
    ground.cellM = world.land.cell_size_m;
    ground.widthM = ground.cols * ground.cellM;
    ground.heightM = ground.rows * ground.cellM;
    view.cx = ground.widthM / 2;
    view.cy = ground.heightM / 2;
    view.mpp = fitMpp(canvas);
    view.ready = true;
    wireMap();
    for (const layer of Object.values(layers)) loadLayer(layer);
    wantTrees(canvas);
  }
  if (view.ready) draw();
}

async function showTimeline() {
  const data = await getJson("/api/timeline");
  notes.timeline.textContent = data.note;
  // The timeline is served from the island's memory, so it looks healthy
  // whether or not the replay log is reaching disk. If it is not, say so
  // here: a record nobody can replay is the one thing this page must not
  // show silently.
  if (data.log_error) {
    notes.timeline.append(
      el("strong", " The replay log is not being written: ", { class: "warn" }),
      document.createTextNode(data.log_error),
    );
  }
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

/// Take every view off the page. Used when the island is not running: what
/// they hold was true once and is not now, and leaving a write form up for
/// a stopped island invites an operator to act on it.
function hidePanels() {
  for (const id of ["intervene-panel", "map-panel", "properties-panel", "economy-panel", "timeline-panel"]) {
    const panel = document.getElementById(id);
    if (panel) panel.hidden = true;
  }
}

/// Say which kind of "not running" this is, rather than assuming the one
/// that is easier to explain.
function sayNoWorld(stopped) {
  noWorldEl.replaceChildren(
    el("strong", stopped ? "The island has stopped." : "No island is running."),
    el(
      "span",
      stopped
        ? "It was running and is not now — an audit that did not close, or a step that failed. " +
          "The server says why on the console it was started from. What this page last showed is " +
          "gone rather than left up, because it is no longer true."
        : "This dashboard was started without --scenario, so there is no world to describe. " +
          "The People page still works: it shows the stored population.",
    ),
  );
  noWorldEl.hidden = false;
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
      // Everything shown was true of an island that is no longer stepping,
      // and the form would still accept writes for it. Before this, a page
      // that had been showing a running world simply kept showing it, with
      // the banner added underneath.
      hidePanels();
      // Two different things read as `running: false`: a dashboard started
      // without `--scenario`, which has no world at all, and one whose
      // island stopped — an audit that did not close, a step that failed.
      // The data endpoints tell them apart, because the first has nothing
      // to serve and answers 404 while the second still answers.
      let stopped = true;
      try {
        await getJson("/api/properties");
      } catch (_) {
        stopped = false;
      }
      sayNoWorld(stopped);
      return;
    }
    noWorldEl.hidden = true;
    document.getElementById("intervene-panel").hidden = false;
    showMap(world);
    await Promise.all([
      showProperties(),
      showEconomy(),
      showTimeline(),
      fillPlacesAndPeople(),
    ]);
    // Again after the panels, so the structures the economy just read are
    // drawn without fetching them a second time.
    try {
      showMap(world);
    } catch (error) {
      // A map without its markers is still a map; the picture, the zoom
      // and the click handling do not depend on this.
    }
  } catch (error) {
    if (error && error.status === 404) {
      hidePanels();
      sayNoWorld(false);
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

/// Fill a `<select>` without disturbing a choice somebody has already made.
///
/// `replaceChildren` resets the selection to the first option. This page
/// reloads every five seconds, so rebuilding unconditionally meant an
/// operator could pick the second islander, have a refresh land before they
/// pressed the button, and remove the first one instead. A destructive,
/// irreversible action aimed at the wrong person because a timer fired.
///
/// So the options are rebuilt only when the set of them has actually
/// changed, and the chosen value is put back afterwards when it still
/// exists.
function fillSelect(select, options) {
    // Compared by value *and* label. Values alone is not enough: the
    // clicked-cell place keeps index 1 while its label changes with every
    // click, so a value-only comparison skipped the rebuild and left the
    // dropdown naming the cell before last. It took two clicks to see --
    // one click looked perfect.
    const key = (value, label) => `${value}\u0001${label}`;
    const wanted = options.map((o) => key(o.value, o.label)).join("\u0000");
    const have = [...select.options].map((o) => key(o.value, o.text)).join("\u0000");
    if (wanted === have) return;
    const chosen = select.value;
    select.replaceChildren(...options.map((o) => el("option", o.label, { value: o.value })));
    if (options.some((o) => o.value === chosen)) select.value = chosen;
}

/// The cell the operator last clicked on the map, if any.
///
/// Held outside `places` because `fillPlacesAndPeople` runs every five
/// seconds and rebuilds that list from the island. Without this, a click
/// would be forgotten within five seconds -- the same shape of bug as the
/// select that reset its own selection on refresh.
let clickedPlace = null;

async function fillPlacesAndPeople() {
  const world = await getJson("/api/world");
  places = [
    {
      label: `The estate — cell ${world.estate.cell[0]}, ${world.estate.cell[1]}`,
      latitude: world.estate.latitude,
      longitude: world.estate.longitude,
    },
  ];
  if (clickedPlace) places.push(clickedPlace);
  fillSelect(
    whereEl,
    places.map((p, i) => ({ value: String(i), label: p.label })),
  );
  fillSelect(
    personEl,
    world.people.map((p) => ({ value: p.agent_id, label: p.agent_id })),
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
    const { status, body: answer } = await postJson(
      "/api/world/interventions",
      body,
      storedToken(),
    );
    if (status === 401) {
      // Sending "" here made every intervention 401 on a server started
      // with ISLAND_CONTROL_TOKEN, with nothing saying why.
      show(
        `${(answer.errors && answer.errors.join(" ")) || "This dashboard may not intervene."} ` +
          "Enter the control token on the Create a human page and it will be used here too.",
        "is-error",
      );
      return;
    }
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
