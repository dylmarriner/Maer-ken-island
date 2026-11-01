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

/* ------------------------------------------------------------------ *\
 * Which island this page is looking at.
 *
 * These pages used to be served by the island itself, so every path was
 * relative and that was the whole story. They can now be hosted anywhere
 * -- a static host, another machine, a file somebody opened -- and talk to
 * a backend somewhere else, so every request has to be told where to go.
 *
 * In order, most specific first:
 *
 *   1. `?backend=` in the address, which also remembers itself, so a link
 *      can point somebody at an island.
 *   2. What was remembered in this browser.
 *   3. `window.ISLAND_BACKEND`, written by `/static/config.js`, which the
 *      server generates. This is how a frontend is *deployed* against a
 *      backend rather than pointed at one by hand.
 *   4. Nothing, meaning the server that sent this page -- which is exactly
 *      what these pages did before, and still what happens when the
 *      island serves its own dashboard.
\* ------------------------------------------------------------------ */

const BACKEND_KEY = "island-backend";

function remembered(key) {
  try {
    return localStorage.getItem(key) || "";
  } catch (_) {
    // A browser with storage switched off still works; it just forgets
    // between loads.
    return "";
  }
}

function remember(key, value) {
  try {
    if (value) localStorage.setItem(key, value);
    else localStorage.removeItem(key);
  } catch (_) {
    /* as above */
  }
}

/** Accept only an HTTP(S) origin, never a path, credentials or a query.
 * A ?backend= link is untrusted input, and may not redirect an existing
 * control token to a different server. */
export function tidyBackend(value) {
  const raw = String(value || "").trim();
  if (!raw) return "";
  if (raw.includes("\\") || raw.includes("?") || raw.includes("#")) {
    throw new Error("Backend address must be a bare HTTP(S) origin.");
  }
  let parsed;
  try {
    parsed = new URL(raw);
  } catch (_) {
    throw new Error("Backend address must be a bare HTTP(S) origin.");
  }
  if (!["http:", "https:"].includes(parsed.protocol) ||
      !parsed.hostname || parsed.username || parsed.password ||
      parsed.pathname !== "/") {
    throw new Error("Backend address must be a bare HTTP(S) origin.");
  }
  return parsed.origin;
}

let backendOverride = null;

export function backend() {
  if (backendOverride !== null) return backendOverride;
  const asked = new URLSearchParams(location.search).get("backend");
  if (asked !== null) {
    const tidy = tidyBackend(asked);
    remember(BACKEND_KEY, tidy);
    backendOverride = tidy;
    return tidy;
  }
  const stored = remembered(BACKEND_KEY);
  // Local storage is mutable by other scripts; never trust a cached origin.
  if (stored) return tidyBackend(stored);
  return tidyBackend(window.ISLAND_BACKEND || "");
}

/** A token shown for island A must never silently be saved for island B.
 * Only a token explicitly edited for the destination may cross the switch. */
export function tokenForBackendSwitch(previousOrigin, nextOrigin, visibleToken, edited) {
  return previousOrigin === nextOrigin || edited ? visibleToken.trim() : "";
}

/** Point this browser at another island. `""` means the page's own host. */
export function setBackend(value) {
  const tidy = tidyBackend(value);
  remember(BACKEND_KEY, tidy);
  backendOverride = tidy;
}

/** Whether this page is talking to a backend other than its own host. */
export function backendIsElsewhere() {
  const base = backend();
  return base !== "" && base !== location.origin;
}

/** The full address of an island API path.
 * Reject absolute, scheme-relative and traversal paths before attaching a
 * bearer token. Some paths are assembled from server-provided identifiers. */
export function apiUrl(path) {
  if (typeof path !== "string" || !path.startsWith("/api/") ||
      path.includes("\\") || path.includes("#")) {
    throw new Error("Island API path must stay on this backend.");
  }
  const base = backend();
  const expectedOrigin = base || location.origin;
  const resolved = new URL(path, expectedOrigin);
  if (resolved.origin !== expectedOrigin || !resolved.pathname.startsWith("/api/")) {
    throw new Error("Island API path must stay on this backend.");
  }
  // Preserve relative URLs when the dashboard and API share a host.
  return base ? resolved.href : path;
}

/** The headers every request carries, token included when there is one. */
function headersFor(extra) {
  const headers = { accept: "application/json", ...(extra || {}) };
  // One token on every request, read and write. The backend takes the
  // control token for reads as well, so somebody holding it needs only
  // the one; somebody holding a read token sends it and is refused the
  // writes, which is the right answer and says so.
  const token = storedToken();
  if (token) headers.authorization = "Bearer " + token;
  return headers;
}

export async function getJson(path) {
  const url = apiUrl(path);
  const response = await fetch(url, { headers: headersFor(), redirect: "error" });
  if (!response.ok) {
    // The code travels with the error. Without it a caller could only match
    // on the message, and `island.js` had a `error.status === 404` branch
    // that could never run because this threw a bare `Error`.
    const failed = new Error(`The server answered ${response.status} for ${path}.`);
    failed.status = response.status;
    // The island refuses in words; carry them so a page can show what it
    // actually said instead of a status code.
    try {
      const said = await response.json();
      if (said && Array.isArray(said.errors) && said.errors.length) {
        failed.message = said.errors.join(" ");
        failed.errors = said.errors;
      }
    } catch (_) {
      /* not JSON: the status is all there is */
    }
    throw failed;
  }
  return response.json();
}

export async function postJson(path, body, token) {
  const headers = headersFor({ "content-type": "application/json" });
  // An explicitly-passed token wins, so a page can send one somebody has
  // just typed without storing it first.
  if (token) headers.authorization = "Bearer " + token;
  const response = await fetch(apiUrl(path), {
    method: "POST",
    redirect: "error",
    headers,
    body: JSON.stringify(body),
  });
  const payload = await response.json().catch(() => ({}));
  return { status: response.status, body: payload };
}

/** An image from the island, as something an `<img>` or a canvas can draw.
 *
 * Not `img.src = url`, and that is the whole reason this exists: a browser
 * sends no `Authorization` header on an image it loads that way, so on a
 * backend with a read token every map layer would come back 401 and the
 * island would be drawn as nothing at all. Fetching the bytes and handing
 * over a blob is the only way an image can carry a token.
 *
 * The caller revokes the URL when it is done with it. */
export async function getImage(path) {
  const response = await fetch(apiUrl(path), { headers: headersFor({ accept: "image/png" }), redirect: "error" });
  if (!response.ok) {
    const failed = new Error(`The server answered ${response.status} for ${path}.`);
    failed.status = response.status;
    throw failed;
  }
  return URL.createObjectURL(await response.blob());
}

/** Where the control token lives for this browser tab.
 *
 * Shared because three pages need it and only one had it. The creator page
 * read and wrote this key while the overview's controls and the island
 * page's interventions both sent an empty token, so on a server started
 * with `ISLAND_CONTROL_TOKEN` every pause, every speed change and every
 * intervention came back 401 with nothing on screen explaining why. */
export const TOKEN_KEY = "island-control-token";
export const TOKEN_ORIGIN_KEY = "island-control-token-origin";

// A token belongs to one backend origin. Never forward a token saved for
// island A to island B, including when B came from an untrusted ?backend= link.
// Legacy unscoped tokens are intentionally not sent until re-entered.
function currentBackendOrigin() {
  return backend() || location.origin;
}

export function storedToken() {
  try {
    const token = sessionStorage.getItem(TOKEN_KEY) || "";
    return sessionStorage.getItem(TOKEN_ORIGIN_KEY) === currentBackendOrigin() ? token : "";
  } catch (_) {
    return "";
  }
}

export function rememberToken(token) {
  try {
    if (token) {
      sessionStorage.setItem(TOKEN_ORIGIN_KEY, currentBackendOrigin());
      sessionStorage.setItem(TOKEN_KEY, token);
    } else {
      sessionStorage.removeItem(TOKEN_KEY);
      sessionStorage.removeItem(TOKEN_ORIGIN_KEY);
    }
  } catch (_) {
    // A browser with storage switched off still works; the token just has
    // to be typed again next time.
  }
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

/* ------------------------------------------------------------------ *\
 * The server control.
 *
 * On every page, because a frontend hosted away from its island has to be
 * told which island, and because the alternative is a page that silently
 * fails against the wrong one. When these pages are served by the island
 * itself -- still the ordinary case -- it says so in one line and there is
 * nothing to do.
\* ------------------------------------------------------------------ */

/** Where the control is written, if the page has a place for it. */
const SERVER_CHROME_ID = "server-chrome";

function describeBackend() {
  const base = backend();
  if (!base) return "this server";
  return base;
}

/** Draw the server line, and the form behind it when it is opened. */
export function mountServerChrome() {
  const host = document.getElementById(SERVER_CHROME_ID);
  if (!host) return;
  host.replaceChildren();

  const line = el("p", null, { class: "server-line" });
  line.append(
    el("span", `Island: ${describeBackend()}`, { id: "server-where" }),
    document.createTextNode(" · "),
    el("span", storedToken() ? "token set" : "no token", {
      id: "server-token-state",
      class: storedToken() ? "" : "muted",
    }),
    document.createTextNode(" "),
  );
  const change = el("button", "Change", { type: "button", class: "linkish" });
  line.append(change);
  host.append(line);

  const form = el("form", null, { class: "server-form", hidden: true });
  const url = el("input", null, {
    type: "url",
    id: "server-url",
    placeholder: "http://island.local:8080",
    value: backend(),
    autocomplete: "off",
    spellcheck: "false",
  });
  const token = el("input", null, {
    type: "password",
    id: "server-token",
    placeholder: "token, if this island needs one",
    value: storedToken(),
    autocomplete: "off",
  });
  const urlLabel = el("label", "Backend address", { for: "server-url" });
  const tokenLabel = el("label", "Token", { for: "server-token" });
  form.append(
    urlLabel,
    url,
    el(
      "p",
      "Scheme, host and port, and nothing else. Leave it empty to use the server that sent this page.",
      { class: "hint" },
    ),
    tokenLabel,
    token,
    el(
      "p",
      "Sent with every request. The control token reads and writes; a read token only reads. When changing islands, enter the new island's token again; the old token is not copied.",
      { class: "hint" },
    ),
  );
  const originalOrigin = currentBackendOrigin();
  let tokenEdited = false;
  token.addEventListener("input", () => { tokenEdited = true; });
  const save = el("button", "Connect", { type: "submit", class: "primary" });
  form.append(save);
  host.append(form);

  change.addEventListener("click", () => {
    form.hidden = !form.hidden;
    if (!form.hidden) url.focus();
  });

  url.addEventListener("input", () => url.setCustomValidity(""));
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    let target;
    try {
      target = tidyBackend(url.value);
      url.setCustomValidity("");
    } catch (error) {
      url.setCustomValidity(error.message);
      url.reportValidity();
      return;
    }
    // The password field initially contains the previous island's token.
    // Changing only the backend URL must not rebind that secret to a new host.
    const nextToken = tokenForBackendSwitch(
      originalOrigin, target || location.origin, token.value, tokenEdited,
    );
    setBackend(target);
    rememberToken(nextToken);
    // A reload rather than a re-fetch: every page holds state built from
    // one island -- a selected person, a map view, a roster -- and
    // carrying that across to a different island would show one island's
    // numbers under another's name.
    location.reload();
  });
}

/** Show the "cannot reach the server" banner, with the reason.
 *
 * Three different failures, three different things to do, so they are not
 * collapsed into one sentence: the island refused for want of a token,
 * the island is not there, or the island answered something unexpected.
 */
export function reportOffline(error) {
  const banner = document.getElementById("offline");
  if (!banner) return;
  const detail = document.getElementById("offline-detail");
  if (detail) {
    detail.textContent = `${explain(error)} ${nextStep(error)}`;
  }
  // The heading is written in the page as "cannot reach the island
  // server", which is right for a server that is not there and plainly
  // wrong for one that answered 401. A browser showed exactly that: the
  // island said what it wanted in the sentence below while the line above
  // it said the island could not be reached.
  const heading = banner.querySelector("strong");
  if (heading) heading.textContent = headline(error);
  banner.hidden = false;
  // Open the server control when a token is what is missing, and scroll
  // to it. The control lives in the footer, so "enter a token below" was
  // true and useless: a screenshot of the first load showed the banner
  // saying it with the field a page and a half further down.
  if (error && error.status === 401) {
    const form = document.querySelector(".server-form");
    if (form && form.hidden) {
      form.hidden = false;
      form.scrollIntoView({ behavior: "smooth", block: "center" });
      const field = document.getElementById("server-token");
      if (field) field.focus({ preventScroll: true });
    }
  }
}

function headline(error) {
  if (error && error.status === 401) return "This island needs a token.";
  if (error && error.status === 403) return "This island refused this page.";
  if (error && error.status >= 500) return "The island server is in trouble.";
  if (error && error.status === 404) return "The island server has no such endpoint.";
  return "The dashboard cannot reach the island server.";
}

function nextStep(error) {
  const where = describeBackend();
  if (error && error.status === 401) {
    return `Enter a token for ${where} below.`;
  }
  if (error && error.status >= 500) {
    return `That is a fault in the island at ${where} rather than in this page.`;
  }
  const message = error && error.message ? error.message : String(error);
  if (message.includes("Failed to fetch") || message.includes("NetworkError")) {
    return backendIsElsewhere()
      ? `Check that ${where} is running and that it was started with --allow-origin ${location.origin}.`
      : "Check that the island server is still running, then reload.";
  }
  return "";
}
