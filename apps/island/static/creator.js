// The Human Creator: the form, what the server will accept, and what it says
// back. Every message from the server is shown as text, never as markup.
"use strict";

import {
  clearOffline,
  el,
  fillChrome,
  getJson,
  postJson,
  rememberToken,
  reportOffline,
  storedToken,
} from "/static/app.js";

const form = document.getElementById("creator");
const resultEl = document.getElementById("result");
const submitEl = document.getElementById("submit");
const submitHintEl = document.getElementById("submit-hint");
const writesEl = document.getElementById("writes");
const tokenFieldset = document.getElementById("token-fieldset");

function fill(select, values, chosen) {
  select.replaceChildren(...values.map((value) => el("option", value, { value })));
  if (chosen && values.includes(chosen)) select.value = chosen;
}

function showErrors(heading, messages) {
  const wrap = el("div");
  wrap.append(el("p", heading, { class: "errors" }));
  const list = el("ul", null, { class: "errors" });
  list.append(...messages.map((text) => el("li", text)));
  wrap.append(list);
  resultEl.replaceChildren(wrap);
}

// Age follows the birth date until someone types an age themselves.
let ageEdited = false;
form.age_years.addEventListener("input", () => {
  ageEdited = true;
});
form.birth_local.addEventListener("change", () => {
  if (ageEdited || !form.birth_local.value) return;
  const born = new Date(form.birth_local.value);
  const age = (Date.now() - born.getTime()) / (365.25 * 86400000);
  if (Number.isFinite(age) && age >= 0) form.age_years.value = age.toFixed(1);
});

function describeWrites(status) {
  switch (status.writes_mode) {
    case "token":
      return "This server was started with a control token. Enter it below to create people.";
    case "loopback":
      return "This server is bound to this machine, so anyone sitting at it can create people. No token needed.";
    default:
      return "Creating people is switched off: this server is reachable from the network but has no control token. Restart it with ISLAND_CONTROL_TOKEN set, or bind it to 127.0.0.1.";
  }
}

function lockForm(reason) {
  for (const field of form.elements) field.disabled = true;
  submitHintEl.textContent = reason;
}

const placeFieldset = document.getElementById("place-fieldset");
const placeEl = document.getElementById("place");
const placeHint = document.getElementById("place-hint");
const whereEl = document.getElementById("where");
const placeField = document.getElementById("place-field");
const rowField = document.getElementById("row-field");
const colField = document.getElementById("col-field");
const rowEl = document.getElementById("row");
const colEl = document.getElementById("col");
const cellHint = document.getElementById("cell-hint");

/// Whether a world is running, and which rooms it offers. Decided once at
/// load: a dashboard does not gain or lose its island while a form is open.
let world = null;

/// Offer the estate's own spaces, by the ids the island will accept, so the
/// page can never ask for a room that does not exist.
function offerPlaces(w) {
  world = w && w.running === true ? w : null;
  placeFieldset.hidden = world === null;
  if (!world) return;
  showPlaceMode();
  const spaces = (world.estate && world.estate.spaces) || [];
  placeEl.replaceChildren(
    ...spaces.map((space) => {
      const option = el("option", `${space.label} (${space.kind.toLowerCase()})`);
      option.value = String(space.id);
      return option;
    }),
  );
  const home = spaces.find((s) => /bedroom/i.test(s.label));
  if (home) placeEl.value = String(home.id);
  placeHint.textContent =
    "The island is running, so this person is put into it and starts living " +
    "from the next step. They are placed in this space on the founders' estate.";

  // A cell of the island, for somebody who does not start at the estate.
  // The bounds come from the world rather than being assumed, and the
  // estate's own cell is offered as a starting point because it is the one
  // cell anybody can name without a map.
  const land = world.land || {};
  rowEl.max = Math.max(0, (land.rows || 1) - 1);
  colEl.max = Math.max(0, (land.cols || 1) - 1);
  const estate = world.estate && world.estate.cell;
  if (estate) {
    rowEl.value = String(estate[0]);
    colEl.value = String(estate[1]);
  }
  cellHint.textContent =
    `Rows 0 to ${rowEl.max}, columns 0 to ${colEl.max}. ` +
    `${figure(land.land_cells || 0)} of those cells are land; the rest are sea, and the island ` +
    `will refuse a creation there. The estate is at ${estate ? estate.join(", ") : "?"}, ` +
    "which is a good place to start from.";
}

/// A number with thousands separators.
function figure(value) {
  return typeof value === "number" && isFinite(value) ? value.toLocaleString() : "—";
}

/// Show the fields for the chosen kind of place.
function showPlaceMode() {
  const onEstate = whereEl.value === "estate";
  placeField.hidden = !onEstate;
  rowField.hidden = onEstate;
  colField.hidden = onEstate;
  cellHint.hidden = onEstate;
}

whereEl.addEventListener("change", showPlaceMode);

async function setup() {
  form.token.value = storedToken();
  try {
    const [options, status, w] = await Promise.all([
      getJson("/api/creator/options"),
      getJson("/api/status"),
      getJson("/api/world"),
    ]);
    offerPlaces(w);
    clearOffline();
    fill(form.biological_sex, options.biological_sex);
    fill(form.build, options.build, "Average");
    fill(form.hair_color, options.hair_color, "Brown");
    fill(form.eye_color, options.eye_color, "Brown");
    fill(form.skin_tone, options.skin_tone, "Medium");
    form.age_years.max = options.max_age_years;
    form.height_cm.min = options.height_cm[0];
    form.height_cm.max = options.height_cm[1];
    document.getElementById("height-range").textContent =
      `Anywhere from ${options.height_cm[0]} to ${options.height_cm[1]} cm.`;

    writesEl.textContent = describeWrites(status);
    writesEl.classList.toggle("is-error", status.writes_mode === "disabled");
    tokenFieldset.hidden = status.writes_mode !== "token";
    if (status.writes_mode === "disabled") {
      lockForm("Creating people is off on this server, so the form is read-only.");
    } else if (status.data_dir) {
      submitHintEl.textContent = `They are written to ${status.data_dir}/humans the moment you do.`;
    }
    fillChrome(status);
  } catch (error) {
    reportOffline(error);
    writesEl.textContent = "The server did not answer, so the form cannot be filled in yet.";
    writesEl.classList.add("is-error");
    lockForm("Reload once the island server is running again.");
  }
}

/** The request body, built from the form exactly as the API expects it. */
function request() {
  const offset = form.birth_offset.value.trim();
  const local = form.birth_local.value;
  // datetime-local gives "YYYY-MM-DDTHH:MM"; RFC 3339 needs seconds and an offset.
  const birth = local ? (local.length === 16 ? local + ":00" : local) + offset : "";
  return {
    name: form.name.value.trim(),
    biological_sex: form.biological_sex.value,
    birth_timestamp: birth,
    birth_latitude: Number(form.birth_latitude.value),
    birth_longitude: Number(form.birth_longitude.value),
    age_years: Number(form.age_years.value),
    height_cm: Number(form.height_cm.value),
    build: form.build.value,
    hair_color: form.hair_color.value,
    eye_color: form.eye_color.value,
    skin_tone: form.skin_tone.value,
  };
}

/** What the page can catch before troubling the server. */
function localProblems(body) {
  const problems = [];
  if (!body.name) problems.push("They need a name.");
  if (!body.birth_timestamp) problems.push("They need a date and time of birth.");
  if (!/^[+-]\d\d:\d\d$/.test(form.birth_offset.value.trim())) {
    problems.push("The UTC offset should look like +13:00 or -05:30.");
  }
  if (!Number.isFinite(body.age_years)) problems.push("Their age should be a number of years.");
  if (!Number.isFinite(body.height_cm)) problems.push("Their height should be a number in centimetres.");
  return problems;
}

function showCreated(created) {
  const box = el("div", null, { class: "created" });
  const name = created.summary.name || created.summary.agent_id;
  box.append(el("h3", `${name} exists.`));
  box.append(
    el(
      "p",
      created.folder
        ? `Their record is written to ${created.folder}, and they are on the roster now.`
        : "They are on the roster now. This population is held in memory, so nothing was written to disk.",
    ),
  );
  const links = el("div", null, { class: "quick-links" });
  links.append(
    el("a", `Open ${name}`, {
      class: "button primary",
      href: "/people?human=" + encodeURIComponent(created.summary.agent_id),
    }),
  );
  box.append(links);
  resultEl.replaceChildren(box);
  if (created.storage_error) {
    resultEl.append(
      el(
        "p",
        `They exist, but their folder could not be written: ${created.storage_error}`,
        { class: "errors" },
      ),
    );
  }
}

/// Follow a queued world creation until the island has answered.
///
/// The request is accepted, not done: the island applies it between steps.
/// Polling the command rather than guessing means the page reports what
/// actually happened, including a refusal the island alone could make.
async function followCommand(queued, name) {
  const path = queued.poll || `/api/world/commands/${queued.command}`;
  resultEl.replaceChildren(el("p", `${name} is queued for the island…`, { class: "hint" }));
  const deadline = Date.now() + 30000;
  for (;;) {
    let outcome;
    try {
      outcome = await getJson(path);
    } catch (error) {
      showErrors("The island was asked, but the answer did not arrive:", [error.message]);
      return;
    }
    if (outcome.state === "created") {
      const box = el("div", null, { class: "created" });
      box.append(el("h3", `${name} is on the island.`));
      const where = outcome.space
        ? `They are in ${outcome.space}, as ${outcome.agent_id}, and from the next step they live there with everyone else.`
        : `They are on the island at row ${outcome.cell[0]}, column ${outcome.cell[1]}, as ${outcome.agent_id}, and from the next step they live there.`;
      box.append(el("p", where));
      if (outcome.storage_error) {
        // Never swallowed: a person whose folder would not write still
        // exists, and only saying so lets anybody notice the gap.
        box.append(
          el(
            "p",
            `Their folder could not be written: ${outcome.storage_error}. They are on the island regardless.`,
            { class: "banner is-error" },
          ),
        );
      }
      const links = el("div", null, { class: "quick-links" });
      links.append(el("a", "See the island", { class: "button secondary", href: "/" }));
      box.append(links);
      resultEl.replaceChildren(box);
      form.name.value = "";
      form.name.focus();
      return;
    }
    if (outcome.state === "refused") {
      showErrors("The island would not take that:", outcome.problems || []);
      return;
    }
    if (Date.now() > deadline) {
      showErrors("The island has not answered:", [
        `Command ${queued.command} is still queued after 30 seconds.`,
      ]);
      return;
    }
    await new Promise((wake) => setTimeout(wake, 250));
  }
}

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const body = request();
  const problems = localProblems(body);
  if (problems.length) {
    showErrors("This person cannot be created yet:", problems);
    form.name.focus();
    return;
  }
  const token = tokenFieldset.hidden ? "" : form.token.value.trim();
  rememberToken(token);
  submitEl.disabled = true;
  submitEl.textContent = "Creating…";
  resultEl.replaceChildren(el("p", `Building ${body.name}…`, { class: "hint" }));
  try {
    // With a world, the person goes into it: the island applies the request
    // on its own thread before its next step, so the answer is a command id
    // to follow rather than a person. Without one, they are stored as
    // before.
    const path = world ? "/api/world/humans" : "/api/humans";
    if (world) {
      // Exactly one kind of place, because "somewhere" is not a place and
      // the island refuses a mixture.
      if (whereEl.value === "estate") {
        body.space = Number(placeEl.value);
      } else {
        body.row = Number(rowEl.value);
        body.col = Number(colEl.value);
      }
    }
    const { status, body: answer } = await postJson(path, body, token);
    if (status === 202) {
      await followCommand(answer, body.name);
    } else if (status === 201) {
      showCreated(answer);
      form.name.value = "";
      form.name.focus();
    } else if (status === 401) {
      const reasons =
        answer.errors && answer.errors.length
          ? [...answer.errors]
          : ["This dashboard is not allowed to create people."];
      if (answer.writes) reasons.push(`On this server, ${answer.writes}.`);
      showErrors("Nobody was created:", reasons);
    } else {
      showErrors(
        "The server would not accept that:",
        answer.errors && answer.errors.length
          ? answer.errors
          : [`It answered ${status} without saying why.`],
      );
    }
  } catch (error) {
    showErrors("Nothing was created:", [
      `The request did not get through: ${error.message}`,
    ]);
  } finally {
    submitEl.disabled = false;
    submitEl.textContent = "Create this person";
  }
});

setup();
