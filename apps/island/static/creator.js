// Human Creator page. Server messages and echoed values are inserted with
// textContent, never as HTML.
"use strict";

const TOKEN_KEY = "island-control-token";
const form = document.getElementById("creator");
const resultEl = document.getElementById("result");
const submitEl = document.getElementById("submit");

function el(tag, text, attrs) {
  const node = document.createElement(tag);
  if (text !== undefined && text !== null) node.textContent = String(text);
  if (attrs) for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  return node;
}

function fill(select, values) {
  select.replaceChildren(...values.map((v) => el("option", v, { value: v })));
}

function showErrors(messages) {
  const list = el("ul", null, { class: "errors" });
  list.append(...messages.map((m) => el("li", m)));
  resultEl.replaceChildren(list);
}

function storedToken() {
  try { return sessionStorage.getItem(TOKEN_KEY) || ""; } catch (_) { return ""; }
}

function rememberToken(token) {
  try { token ? sessionStorage.setItem(TOKEN_KEY, token) : sessionStorage.removeItem(TOKEN_KEY); } catch (_) {}
}

// Age follows the birth date unless the person edits it themselves.
let ageEdited = false;
form.age_years.addEventListener("input", () => { ageEdited = true; });
form.birth_local.addEventListener("change", () => {
  if (ageEdited || !form.birth_local.value) return;
  const born = new Date(form.birth_local.value);
  const age = (Date.now() - born.getTime()) / (365.25 * 86400000);
  if (Number.isFinite(age) && age >= 0) form.age_years.value = age.toFixed(1);
});

async function setup() {
  form.token.value = storedToken();
  try {
    const [options, status] = await Promise.all([
      fetch("/api/creator/options").then((r) => r.json()),
      fetch("/api/status").then((r) => r.json()),
    ]);
    fill(form.biological_sex, options.biological_sex);
    fill(form.build, options.build);
    fill(form.hair_color, options.hair_color);
    fill(form.eye_color, options.eye_color);
    fill(form.skin_tone, options.skin_tone);
    form.build.value = "Average";
    form.age_years.max = options.max_age_years;
    form.height_cm.min = options.height_cm[0];
    form.height_cm.max = options.height_cm[1];
    document.getElementById("writes").textContent = "Writes: " + status.writes +
      ". New people get their own folder" + (status.data_dir ? " under " + status.data_dir + "/humans." : ".");
  } catch (err) {
    showErrors(["Could not reach the island server: " + err.message]);
  }
}

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

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  const token = form.token.value.trim();
  rememberToken(token);
  const headers = { "content-type": "application/json" };
  if (token) headers.authorization = "Bearer " + token;
  submitEl.disabled = true;
  resultEl.replaceChildren(el("p", "Creating…", { class: "hint" }));
  try {
    const response = await fetch("/api/humans", { method: "POST", headers, body: JSON.stringify(request()) });
    const body = await response.json().catch(() => ({}));
    if (response.status === 201) {
      const id = body.summary.agent_id;
      const done = el("p", null, { class: "ok" });
      const link = el("a", body.summary.name || id, { href: "/?human=" + encodeURIComponent(id) });
      done.append("Created ", link, body.folder ? " — folder " + body.folder : "");
      resultEl.replaceChildren(done);
      if (body.storage_error) {
        resultEl.append(el("p", "Warning: they exist, but their folder could not be written: " + body.storage_error, { class: "errors" }));
      }
      form.name.value = "";
    } else if (response.status === 401) {
      showErrors(["Not allowed to create (" + (body.writes || "unauthorised") + "). Enter the control token."]);
    } else {
      showErrors(body.errors && body.errors.length ? body.errors : ["Server said " + response.status]);
    }
  } catch (err) {
    showErrors(["Request failed: " + err.message]);
  } finally {
    submitEl.disabled = false;
  }
});

setup();
