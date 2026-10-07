// The overview page: how many people there are, where they are kept, and who
// has been added lately.
"use strict";

import {
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

const statsEl = document.getElementById("stats");
const activityEl = document.getElementById("activity");
const worldPanel = document.getElementById("world-panel");
const worldNote = document.getElementById("world-note");
const worldStats = document.getElementById("world-stats");
const worldPeople = document.getElementById("world-people");
const introHeading = document.getElementById("intro-heading");
const introLead = document.getElementById("intro-lead");
const timeClaim = document.getElementById("time-claim");

function stat(label, value, note) {
  const card = el("div", null, { class: "stat" });
  card.append(
    el("div", label, { class: "label" }),
    el("div", value, { class: "value" }),
    el("div", note || " ", { class: "note" }),
  );
  return card;
}

function medianWords(status) {
  if (typeof status.median_age_years !== "number") return ["—", "Nobody here yet"];
  const youngest = ageWords(status.youngest_years);
  const oldest = ageWords(status.oldest_years);
  return [
    `${status.median_age_years.toFixed(0)}`,
    status.population > 1 ? `Youngest ${youngest}, oldest ${oldest}` : `The only person is ${oldest}`,
  ];
}

function writesWords(status) {
  switch (status.writes_mode) {
    case "token":
      return ["Token", "A control token is required"];
    case "loopback":
      return ["Open here", "Anyone on this machine can add people"];
    default:
      return ["Off", "Set a control token to turn it on"];
  }
}

function showStats(status) {
  const [median, ageNote] = medianWords(status);
  const [writes, writesNote] = writesWords(status);
  const sexes = [];
  if (status.female) sexes.push(count(status.female, "woman", "women"));
  if (status.male) sexes.push(count(status.male, "man", "men"));
  statsEl.replaceChildren(
    stat("People", status.population.toLocaleString(), sexes.join(" · ") || "The island is empty"),
    stat(
      "Alive",
      status.alive.toLocaleString(),
      status.dead ? `${count(status.dead, "person", "people")} dead` : "Nobody has died",
    ),
    stat("Median age", median, ageNote),
    stat("Creating people", writes, writesNote),
  );

  const dataDir = document.getElementById("data-dir");
  if (dataDir) dataDir.textContent = status.data_dir || "nothing on disk (in-memory population)";
  const seed = document.getElementById("seed");
  if (seed) {
    seed.textContent = status.seed_hex ? `${status.seed_hex.slice(0, 12)}…` : "unknown";
    seed.setAttribute("title", status.seed_hex || "");
  }
  fillChrome(status);
}

function showActivity(feed) {
  if (!feed.stored) {
    activityEl.replaceChildren(
      emptyState("No creation log", feed.note || "This population is held in memory."),
    );
    return;
  }
  if (feed.error) {
    message(activityEl, feed.error, "errors");
    return;
  }
  const creations = feed.creations || [];
  if (creations.length === 0) {
    activityEl.replaceChildren(
      emptyState(
        "Only the founders so far",
        "Gem-D and Gem-K were seeded with the data directory. Anyone you create from here shows up in this list.",
      ),
    );
    return;
  }
  const list = el("ul", null, { class: "feed" });
  for (const creation of creations) {
    const item = el("li");
    const who = el("a", creation.name || creation.agent_id, {
      class: "who",
      href: "/people?human=" + encodeURIComponent(creation.agent_id),
    });
    const what = el(
      "span",
      `${creation.biological_sex || "unknown"} · ${ageWords(creation.age_years)} · added from the ${creation.by}`,
      { class: "what" },
    );
    item.append(who, what, el("span", `#${creation.counter + 1}`, { class: "when" }));
    list.append(item);
  }
  activityEl.replaceChildren(list);
  if (feed.total > creations.length) {
    activityEl.append(
      el("p", `Showing the ${creations.length} most recent of ${feed.total} creations.`, {
        class: "hint",
      }),
    );
  }
}


/// A number with thousands separators, or a dash when there is nothing to
/// show. Raw floats off the wire are unreadable in a column of figures.
function figure(value, digits) {
  if (typeof value !== "number" || !isFinite(value)) return "—";
  return value.toLocaleString(undefined, {
    minimumFractionDigits: digits || 0,
    maximumFractionDigits: digits || 0,
  });
}

/// The island's own clock, which runs on a 36-hour day.
function clockWords(clock) {
  const hour = Math.floor(clock.hour_of_day);
  const minute = Math.floor((clock.hour_of_day - hour) * 60);
  const time = `${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;
  return `Day ${Math.floor(clock.days) + 1}, ${time} of a ${figure(clock.day_length_hours)}-hour day`;
}

/// How the island is keeping up, when it has been running long enough to
/// say. Shown plainly rather than hidden: an island that cannot keep up is
/// something the person watching should know.
function speedWords(clock) {
  if (typeof clock.achieved_speed !== "number") return `Running at ${clock.requested_speed}`;
  const achieved = clock.achieved_speed;
  const words = achieved >= 2 ? `${figure(achieved)}x real time` : `${figure(achieved, 2)}x real time`;
  return `Asked for ${clock.requested_speed}, managing ${words}`;
}

/// The digest line: what it was, and at which tick, so a reader is never
/// led to think a carried value is live.
function digestWords(world) {
  const d = world.digest;
  if (!d || !d.value) return "";
  const when = d.current ? "now" : `as of tick ${figure(d.at_tick)}`;
  return `State digest ${d.value.slice(0, 16)}… ${when}.`;
}

/// Say which dashboard this is.
///
/// The page was written when there was no world to run, and said so: a
/// heading about the clock not having started, and a claim that nobody ages
/// or sleeps. With --scenario both are false — time is running two panels
/// down — and a page that contradicts itself is worse than one that admits
/// a gap. So the standing copy is the no-world case and this replaces it
/// when there is a world.
function tellTheTruthAboutTime(running) {
  if (!running) return;
  introHeading.textContent = "The island, running";
  introLead.textContent =
    "Everyone here has a full canonical record — body, temperament, needs, " +
    "attention, the lot — kept in their own encrypted folder on disk. The " +
    "island is stepping: they age, sleep, breathe and eat, and what you see " +
    "below is where they have got to.";
  timeClaim.replaceChildren(
    el("strong", "Time is running."),
    document.createTextNode(
      " The island steps on its own thread and these readings come from it. " +
        "People created here are still stored rather than placed in the world: " +
        "that join is Phase 4 Task 6, and it is not built yet.",
    ),
  );
}

function showWorld(world) {
  if (!world || world.running !== true) {
    // No world is the ordinary case for a dashboard started without
    // --scenario, so the panel stays out of the way rather than showing an
    // error for something nobody asked for.
    worldPanel.hidden = true;
    return;
  }
  tellTheTruthAboutTime(true);
  worldPanel.hidden = false;
  worldNote.textContent =
    `${clockWords(world.clock)}. ${speedWords(world.clock)}. ${digestWords(world)}`.trim();

  const alive = world.people.filter((person) => person.alive).length;
  const asleep = world.people.filter((person) => person.alive && person.asleep).length;
  worldStats.replaceChildren(
    stat("Islanders", String(alive), asleep === 1 ? "1 asleep" : `${asleep} asleep`),
    stat("Trees on the estate", figure(world.land.trees), `${figure(world.land.patch_carbon_kgc)} kg of carbon`),
    stat(
      "Battery",
      `${figure(world.estate.battery_charge_kwh, 1)} kWh`,
      `of ${figure(world.estate.battery_capacity_kwh, 1)} kWh, with ${figure(world.estate.fuel_litres)} L of fuel`,
    ),
    stat(
      "Books",
      figure(world.stocks.audits_closed),
      world.stocks.food_shortfalls + world.stocks.water_shortfalls === 0
        ? "every step closed, nothing went short"
        : `${world.stocks.food_shortfalls} food and ${world.stocks.water_shortfalls} water shortfalls`,
    ),
  );

  const list = el("ul", null, { class: "feed" });
  for (const person of world.people) {
    const item = el("li", null, { class: "feed-item" });
    const where = person.space || "off the estate";
    const doing = !person.alive ? "died" : person.asleep ? "asleep" : "awake";
    item.append(
      el("span", person.agent_id, { class: "who" }),
      el(
        "span",
        `${doing} · ${where} · ${ageWords(person.age_years)}` +
          (typeof person.body_carbon_kg === "number"
            ? ` · ${figure(person.body_carbon_kg, 1)} kg of body carbon`
            : ""),
        { class: "what" },
      ),
    );
    list.append(item);
  }
  worldPeople.replaceChildren(list);
}

async function load() {
  // An old bookmark to a person used to point here; send it to the roster.
  const wanted = new URLSearchParams(location.search).get("human");
  if (wanted) {
    location.replace("/people?human=" + encodeURIComponent(wanted));
    return;
  }
  try {
    const [status, feed, world] = await Promise.all([
      getJson("/api/status"),
      getJson("/api/activity?limit=8"),
      getJson("/api/world"),
    ]);
    clearOffline();
    showStats(status);
    showActivity(feed);
    showWorld(world);
  } catch (error) {
    reportOffline(error);
    message(activityEl, "The creation log could not be read while the server is unreachable.", "hint");
  }
}

load();

// A running island changes while the page is open, so the world panel
// refreshes on its own. Only the world: the stored population and the
// creation log change when somebody acts, not on their own.
setInterval(async () => {
  try {
    showWorld(await getJson("/api/world"));
  } catch (error) {
    // A refresh that fails is not worth a banner — the next one will say
    // so, and `load()` already reports a server that has gone away.
  }
}, 2000);
