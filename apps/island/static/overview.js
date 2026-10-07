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

async function load() {
  // An old bookmark to a person used to point here; send it to the roster.
  const wanted = new URLSearchParams(location.search).get("human");
  if (wanted) {
    location.replace("/people?human=" + encodeURIComponent(wanted));
    return;
  }
  try {
    const [status, feed] = await Promise.all([
      getJson("/api/status"),
      getJson("/api/activity?limit=8"),
    ]);
    clearOffline();
    showStats(status);
    showActivity(feed);
  } catch (error) {
    reportOffline(error);
    message(activityEl, "The creation log could not be read while the server is unreachable.", "hint");
  }
}

load();
