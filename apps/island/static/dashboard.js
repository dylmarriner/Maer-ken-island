// People page. Every value from the server is inserted with textContent,
// never as HTML: names and attributes are user-supplied.
"use strict";

const rosterEl = document.getElementById("roster");
const detailEl = document.getElementById("detail");
const statusEl = document.getElementById("status");

function el(tag, text, attrs) {
  const node = document.createElement(tag);
  if (text !== undefined && text !== null) node.textContent = String(text);
  if (attrs) for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  return node;
}

function years(value) {
  return typeof value === "number" ? value.toFixed(1) + " y" : "?";
}

async function getJson(path) {
  const response = await fetch(path, { headers: { accept: "application/json" } });
  if (!response.ok) throw new Error(path + " → " + response.status);
  return response.json();
}

function definitionList(pairs) {
  const dl = el("dl");
  for (const [key, value] of pairs) {
    if (value === undefined || value === null || value === "") continue;
    dl.append(el("dt", key), el("dd", typeof value === "object" ? JSON.stringify(value) : value));
  }
  return dl;
}

async function showDetail(agentId, button) {
  for (const b of rosterEl.querySelectorAll("button")) b.setAttribute("aria-selected", "false");
  if (button) button.setAttribute("aria-selected", "true");
  detailEl.replaceChildren(el("p", "Loading…", { class: "hint" }));
  try {
    const data = await getJson("/api/humans/" + encodeURIComponent(agentId));
    const s = data.summary || {};
    const human = data.human || {};
    const body = human.body || {};
    const profile = human.profile || {};
    detailEl.replaceChildren(
      el("h2", s.name || s.agent_id),
      definitionList([
        ["Agent id", s.agent_id],
        ["Human id", s.human_id],
        ["Sex", s.biological_sex],
        ["Age", years(s.age_years)],
        ["Status", s.status],
        ["Height", typeof body.height_cm === "number" ? body.height_cm + " cm" : undefined],
        ["Build", body.build],
        ["Hair", body.hair_color],
        ["Eyes", body.eye_color],
        ["Skin", body.skin_tone],
        ["Born", (profile.core_identity || {}).birth_timestamp],
        ["Folder", data.folder],
      ]),
    );
    const details = el("details");
    details.append(el("summary", "Everything stored about " + (s.name || s.agent_id)), el("pre", JSON.stringify(human, null, 2)));
    detailEl.append(details);
  } catch (err) {
    detailEl.replaceChildren(el("p", "Could not load: " + err.message, { class: "errors" }));
  }
}

async function load() {
  try {
    const [status, humans] = await Promise.all([getJson("/api/status"), getJson("/api/humans")]);
    statusEl.textContent = status.population + " people · writes: " + status.writes +
      (status.data_dir ? " · stored in " + status.data_dir : " · not stored on disk");
    if (status.time_running) document.getElementById("banner").hidden = true;
    humans.sort((a, b) => (a.name || a.agent_id).localeCompare(b.name || b.agent_id));
    rosterEl.replaceChildren(...humans.map((h) => {
      const button = el("button", h.name || h.agent_id, { type: "button", "aria-selected": "false", "data-agent": h.agent_id });
      button.append(el("span", h.biological_sex + " · " + years(h.age_years) + " · " + h.status, { class: "meta" }));
      button.addEventListener("click", () => showDetail(h.agent_id, button));
      const li = el("li");
      li.append(button);
      return li;
    }));
    const wanted = new URLSearchParams(location.search).get("human");
    const first = humans.find((h) => h.agent_id === wanted) || humans[0];
    if (first) {
      const button = [...rosterEl.querySelectorAll("button")].find((b) => b.dataset.agent === first.agent_id);
      showDetail(first.agent_id, button);
    }
  } catch (err) {
    statusEl.textContent = "Could not reach the island server: " + err.message;
  }
}

load();
