// The People page: the roster on the left, one person's record on the right.
"use strict";

import {
  explain,
  mountServerChrome,
  ageWords,
  clearOffline,
  count,
  el,
  emptyState,
  fillChrome,
  getJson,
  message,
  reportOffline,
} from "/static/app.js";

const rosterEl = document.getElementById("roster");
const rosterCountEl = document.getElementById("roster-count");
const detailEl = document.getElementById("detail");
const statusEl = document.getElementById("status");
const searchEl = document.getElementById("search");
const sortEl = document.getElementById("sort");

/// Whose roster this is. When an island is running the page shows *its*
/// people: a dashboard with a world should not have two different lists of
/// humans, one stored and one alive. Without a world it shows the stored
/// population exactly as before.
let onTheIsland = false;

const leadEl = document.getElementById("people-lead");

let everyone = [];
let selected = null;

function displayName(human) {
  return human.name || human.agent_id;
}

function matches(human, needle) {
  if (!needle) return true;
  const hay = `${human.name || ""} ${human.agent_id} ${human.human_id}`.toLowerCase();
  return hay.includes(needle);
}

function sorted(people, order) {
  const list = [...people];
  switch (order) {
    case "age-desc":
      return list.sort((a, b) => b.age_years - a.age_years);
    case "age-asc":
      return list.sort((a, b) => a.age_years - b.age_years);
    case "id":
      return list.sort((a, b) => a.human_id.localeCompare(b.human_id));
    default:
      return list.sort((a, b) => displayName(a).localeCompare(displayName(b)));
  }
}

function rosterButton(human) {
  const button = el("button", displayName(human), {
    type: "button",
    role: "option",
    "aria-selected": String(human.agent_id === selected),
    "data-agent": human.agent_id,
  });
  button.append(
    el("span", `${human.biological_sex} · ${ageWords(human.age_years)} · ${human.status}`, {
      class: "meta",
    }),
  );
  button.addEventListener("click", () => select(human.agent_id));
  return button;
}

/** Arrow keys walk the roster the way a listbox should. */
function onRosterKey(event) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  const buttons = [...rosterEl.querySelectorAll("button")];
  const here = buttons.indexOf(document.activeElement);
  if (here === -1) return;
  const next = buttons[here + (event.key === "ArrowDown" ? 1 : -1)];
  if (!next) return;
  event.preventDefault();
  next.focus();
}

function drawRoster() {
  const needle = (searchEl.value || "").trim().toLowerCase();
  const shown = sorted(everyone.filter((human) => matches(human, needle)), sortEl.value);
  if (shown.length === 0) {
    rosterEl.replaceChildren();
    rosterCountEl.textContent = `Nobody matches “${searchEl.value.trim()}”.`;
    return;
  }
  rosterEl.replaceChildren(
    ...shown.map((human) => {
      const item = el("li", null, { role: "presentation" });
      item.append(rosterButton(human));
      return item;
    }),
  );
  rosterCountEl.textContent =
    shown.length === everyone.length
      ? `${count(everyone.length, "person", "people")} on the island.`
      : `Showing ${shown.length} of ${count(everyone.length, "person", "people")}.`;
}

function markSelected(agentId) {
  for (const button of rosterEl.querySelectorAll("button")) {
    button.setAttribute("aria-selected", String(button.dataset.agent === agentId));
  }
}

function chip(text, kind) {
  return el("li", text, { class: kind ? `chip ${kind}` : "chip" });
}

const LONG_VALUE = 26;

function reading(field) {
  const value = String(field.value);
  // A long value reads better under its label than squeezed beside it, and
  // one with no spaces in it — a path, an id, a timestamp — reads better
  // still in a monospaced face. A long sentence does not.
  const classes = ["reading"];
  if (value.length > LONG_VALUE) classes.push("is-long");
  if (value.length > LONG_VALUE && !/\s/.test(value)) classes.push("is-code");
  const wrap = el("div", null, { class: classes.join(" ") });
  const line = el("div", null, { class: "line" });
  line.append(el("span", field.label, { class: "name" }), el("span", field.value, { class: "amount" }));
  wrap.append(line);
  if (typeof field.meter === "number") {
    const track = el("span", null, { class: "meter" });
    const fill = el("span");
    fill.style.width = `${Math.round(field.meter * 100)}%`;
    track.append(fill);
    wrap.append(track);
  }
  if (field.detail) wrap.append(el("p", field.detail, { class: "detail" }));
  return wrap;
}

function readingGroup(section) {
  const group = el("section", null, { class: "reading-group", id: `section-${section.id}` });
  group.append(el("h3", section.title), el("p", section.blurb, { class: "blurb" }));
  for (const field of section.fields) group.append(reading(field));
  return group;
}

function recordDetails(person) {
  const text = JSON.stringify(person.human, null, 2);
  const details = el("details", null, { class: "record" });
  details.append(
    el("summary", `Everything stored about ${displayName(person.summary)}`),
    el(
      "p",
      "The complete record, exactly as it is written to disk. Most of it is derived; nothing here is typed by hand.",
      { class: "hint" },
    ),
  );

  const tools = el("div", null, { class: "record-tools" });
  const copy = el("button", "Copy as JSON", { type: "button", class: "secondary" });
  copy.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(text);
      copy.textContent = "Copied";
    } catch (error) {
      copy.textContent = "Could not copy — select the text instead";
    }
    setTimeout(() => {
      copy.textContent = "Copy as JSON";
    }, 2500);
  });
  const download = el("a", "Download", {
    class: "button secondary",
    href: `/api/humans/${encodeURIComponent(person.summary.agent_id)}`,
    download: `${person.summary.agent_id}.json`,
  });
  tools.append(copy, download);
  details.append(tools, el("pre", text));
  return details;
}

function drawPerson(person) {
  const summary = person.summary || {};
  const head = el("div", null, { class: "person-head" });
  head.append(el("h2", displayName(summary)));
  const chips = el("ul", null, { class: "chips" });
  chips.append(
    chip(summary.status === "alive" ? "Alive" : summary.status, summary.status === "alive" ? "is-alive" : "is-dead"),
    chip(summary.biological_sex),
    chip(ageWords(summary.age_years)),
    chip(summary.human_id),
  );

  detailEl.replaceChildren(
    el("h2", "Selected person", { class: "visually-hidden", id: "detail-heading" }),
    head,
    chips,
  );

  const grid = el("div", null, { class: "section-grid" });
  for (const section of person.sections || []) grid.append(readingGroup(section));
  detailEl.append(grid, recordDetails(person));
}

function loadingPerson() {
  const wrap = el("div", null, { class: "loading" });
  wrap.append(
    el("span", null, { class: "skeleton half" }),
    el("span", null, { class: "skeleton wide" }),
    el("span", null, { class: "skeleton" }),
    el("span", null, { class: "skeleton wide" }),
  );
  detailEl.replaceChildren(wrap);
}

async function select(agentId) {
  selected = agentId;
  markSelected(agentId);
  const url = new URL(location.href);
  url.searchParams.set("human", agentId);
  history.replaceState(null, "", url);
  loadingPerson();
  try {
    const path = onTheIsland ? "/api/world/humans/" : "/api/humans/";
    const person = await getJson(path + encodeURIComponent(agentId));
    if (selected !== agentId) return; // a faster click won
    drawPerson(person);
  } catch (error) {
    detailEl.replaceChildren(
      emptyState("That record could not be read", error.message),
    );
    const retry = el("button", "Try again", { type: "button", class: "secondary" });
    retry.addEventListener("click", () => select(agentId));
    detailEl.append(retry);
  }
}

function describeStatus(status) {
  const where = status.data_dir ? `stored in ${status.data_dir}` : "held in memory only";
  const clock = status.time_running ? "the island clock is running" : "the island clock has not started";
  return `${count(status.population, "person", "people")}, ${where} · ${clock}.`;
}

/// What this roster is of, said plainly. A reader should never have to work
/// out whether they are looking at people who live somewhere or records on
/// a disk.
function describeWorld(world) {
  const behind = world.tick - world.records_at_tick;
  const freshness =
    behind <= 0
      ? "Their records are current."
      : `Their full records were read ${behind} step${behind === 1 ? "" : "s"} ago; where they are and whether they are asleep is current.`;
  return `${count(world.people.length, "person", "people")} living on the island. ${freshness}`;
}

async function load() {
  try {
    const [status, stored, world] = await Promise.all([
      getJson("/api/status"),
      getJson("/api/humans"),
      getJson("/api/world"),
    ]);
    clearOffline();
    onTheIsland = world && world.running === true;
    if (onTheIsland) {
      // The standing copy says nothing has stepped these people, which is
      // true of a stored population and false of a living one.
      leadEl.textContent =
        "Pick someone to see how they are: who they are, what their body is " +
        "doing, how they feel and how they think. These people are living on " +
        "the island — the readings move as they breathe, sleep and eat.";
      const roster = await getJson("/api/world/humans");
      // The world's people in the shape this roster already draws.
      everyone = roster.people.map((person) => ({
        agent_id: person.agent_id,
        name: person.agent_id,
        human_id: person.agent_id,
        biological_sex: "",
        status: person.alive ? "Alive" : "Dead",
        age_years: person.age_years,
      }));
      statusEl.textContent = describeWorld(roster);
    } else {
      everyone = stored;
      statusEl.textContent = describeStatus(status);
    }
    fillChrome(status);
    if (everyone.length === 0) {
      rosterEl.replaceChildren();
      detailEl.replaceChildren(
        emptyState(
          "Nobody lives here yet",
          onTheIsland
            ? "The island is running but has nobody on it."
            : "Create the first person and they will appear in this roster.",
        ),
      );
      rosterCountEl.textContent = "";
      return;
    }
    drawRoster();
    const wanted = new URLSearchParams(location.search).get("human");
    const first = everyone.find((human) => human.agent_id === wanted) || sorted(everyone, sortEl.value)[0];
    if (first) select(first.agent_id);
  } catch (error) {
    reportOffline(error);
    statusEl.textContent = "The roster could not be loaded.";
    message(detailEl, `Nothing can be read: ${explain(error)}`, "hint");
  }
}

searchEl.addEventListener("input", drawRoster);
sortEl.addEventListener("change", drawRoster);
rosterEl.addEventListener("keydown", onRosterKey);
// A back or forward step should land on the person in the address bar.
addEventListener("popstate", () => {
  const wanted = new URLSearchParams(location.search).get("human");
  if (wanted && wanted !== selected) select(wanted);
});

// Before anything is asked for, so the control is there to type a
// token into when the first request comes back 401.
mountServerChrome();
load();

// ---------------------------------------------------------------------------
// What the islanders have said to each other.
//
// The engine has generated these all along and nothing has ever shown them:
// `HumanSystem` keeps a bounded feed, `mk_view` projects it, and no surface
// read it. A simulation whose people talk and whose dashboard cannot show a
// word of it is only half observable, and it is the half that cannot be
// checked by looking.
// ---------------------------------------------------------------------------

const talkPanelEl = document.getElementById("talk-panel");
const talkEl = document.getElementById("talk");
const talkNoteEl = document.getElementById("talk-note");

/// One conversation, as the end of an exchange with the speakers named.
///
/// A conversation is the whole exchange between a pair, not the minute of
/// it that happened on one tick, so it spans `tick` to `last_tick` and can
/// hold hundreds of lines. The backend sends the most recent of them and
/// `lines_said` says how many there really are; this card shows that span
/// and that count, so nobody reads a tail as the whole thing.
function conversationCard(conversation) {
  const card = el("article", null, { class: "talk-card" });
  const head = el("p", null, { class: "talk-head" });
  const last = conversation.last_tick ?? conversation.tick;
  const when =
    last > conversation.tick
      ? `ticks ${conversation.tick}-${last}`
      : `tick ${conversation.tick}`;
  head.append(
    el("span", when, { class: "talk-tick" }),
    el("span", conversation.relationship, { class: "talk-rel" }),
  );
  card.append(head);
  // `lines_said` is 0 from a backend too old to send it, which means "no
  // count given" rather than "nothing was said" -- so only say the lines
  // are a tail when the count is there and is actually larger.
  const linesSaid = conversation.lines_said ?? 0;
  if (linesSaid > conversation.lines.length) {
    card.append(
      el("p", `the last ${conversation.lines.length} of ${linesSaid} lines`, {
        class: "talk-more",
      }),
    );
  }
  for (const line of conversation.lines) {
    const said = el("p", null, { class: "talk-line" });
    // The name the engine used. When a person has no typed-in name it is
    // their id, which is what `Person` does everywhere else on this
    // dashboard rather than inventing something friendlier.
    said.append(el("span", `${line.speaker_name}: `, { class: "talk-who" }));
    said.append(document.createTextNode(line.text));
    card.append(said);
  }
  return card;
}

async function loadConversations() {
  try {
    const data = await getJson("/api/conversations");
    // Shown rather than paraphrased, like the island page's notes: a reader
    // needs to know these lines are composed from real state and that no
    // language model wrote them.
    talkNoteEl.textContent = data.note;
    if (!data.conversations.length) {
      talkEl.replaceChildren(
        emptyState(
          "Nobody has spoken yet",
          "Conversations happen when people are near each other or are kin. None has been generated on this island so far.",
        ),
      );
    } else {
      talkEl.replaceChildren(...data.conversations.map(conversationCard));
    }
    talkPanelEl.hidden = false;
  } catch (error) {
    // A missing island is not an error worth shouting about here: the page
    // already says there is no world. Any other failure hides the panel
    // rather than showing an empty one that looks like silence.
    if (error.status !== 404) reportOffline(error);
    talkPanelEl.hidden = true;
  }
}

loadConversations();
// People keep talking while the page is open, so this refreshes. Five
// seconds matches the island page's own cadence.
setInterval(loadConversations, 5000);
