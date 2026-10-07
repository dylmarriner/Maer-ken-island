// Shared helpers for the three dashboard pages.
//
// Everything that comes back from the server — names, folders, validation
// messages — is put into the page with textContent. None of it is ever
// treated as markup, because a person's name is whatever someone typed.
"use strict";

/** Build an element with text and attributes in one call. */
export function el(tag, text, attrs) {
  const node = document.createElement(tag);
  if (text !== undefined && text !== null) node.textContent = String(text);
  if (attrs) {
    for (const [key, value] of Object.entries(attrs)) {
      if (value === false || value === null || value === undefined) continue;
      node.setAttribute(key, value === true ? "" : String(value));
    }
  }
  return node;
}

/** "1 person" / "4 people", so the page never says "1 people". */
export function count(n, one, many) {
  return `${n.toLocaleString()} ${n === 1 ? one : many}`;
}

/** An age the way someone says it out loud. */
export function ageWords(years) {
  if (typeof years !== "number" || !Number.isFinite(years) || years < 0) return "age unknown";
  const whole = Math.floor(years);
  let months = Math.round((years - whole) * 12);
  let full = whole;
  if (months === 12) {
    full += 1;
    months = 0;
  }
  if (full === 0 && months === 0) return "newborn";
  if (full === 0) return months === 1 ? "1 month old" : `${months} months old`;
  return full === 1 ? "1 year old" : `${full} years old`;
}

export function roundTo(value, places) {
  return typeof value === "number" && Number.isFinite(value)
    ? value.toFixed(places)
    : "—";
}

/** A sentence a person can act on, whatever the fetch failure was. */
export function explain(error) {
  const message = error && error.message ? error.message : String(error);
  if (message.includes("Failed to fetch") || message.includes("NetworkError")) {
    return "The island server did not answer. It may have stopped.";
  }
  return message;
}

export async function getJson(path) {
  const response = await fetch(path, { headers: { accept: "application/json" } });
  if (!response.ok) {
    throw new Error(`The server answered ${response.status} for ${path}.`);
  }
  return response.json();
}

export async function postJson(path, body, token) {
  const headers = { "content-type": "application/json", accept: "application/json" };
  if (token) headers.authorization = "Bearer " + token;
  const response = await fetch(path, { method: "POST", headers, body: JSON.stringify(body) });
  const payload = await response.json().catch(() => ({}));
  return { status: response.status, body: payload };
}

/** Show the "cannot reach the server" banner, with the reason. */
export function reportOffline(error) {
  const banner = document.getElementById("offline");
  if (!banner) return;
  const detail = document.getElementById("offline-detail");
  if (detail) {
    detail.textContent = `${explain(error)} Check that the island server is still running, then reload.`;
  }
  banner.hidden = false;
}

export function clearOffline() {
  const banner = document.getElementById("offline");
  if (banner) banner.hidden = true;
}

/** Fill in the footer from /api/status: build, storage and who may write. */
export function fillChrome(status) {
  const version = document.getElementById("version");
  if (version && status.version) version.textContent = `v${status.version}`;
  const writes = document.getElementById("footer-writes");
  if (writes && status.writes) {
    writes.textContent = `Reading is open to anyone who can reach this page · Creating people: ${status.writes}`;
  }
}

/** Replace a node's children with one message. */
export function message(node, text, className) {
  node.replaceChildren(el("p", text, className ? { class: className } : undefined));
}

/** A placeholder with a heading and a sentence, for an empty list. */
export function emptyState(heading, body) {
  const wrap = el("div", null, { class: "empty" });
  wrap.append(el("h3", heading), el("p", body));
  return wrap;
}
