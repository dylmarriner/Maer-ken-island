// Repeated dashboard requests can finish in any order. Only the newest
// response (including errors) may update the visible state.
export function createLatestRequest() {
  let generation = 0;
  let activeController = null;
  return {
    invalidate() {
      generation += 1;
      activeController?.abort();
      activeController = null;
    },
    async load(fetchPerson, showPerson, showError) {
      // Discarding stale responses alone leaves slow polls and rapid inspector
      // selections in flight. Cancel their HTTP requests as well.
      activeController?.abort();
      const controller = new AbortController();
      activeController = controller;
      const request = ++generation;
      try {
        const person = await fetchPerson(controller.signal);
        if (request === generation) showPerson(person);
      } catch (error) {
        if (request === generation) showError(error);
      } finally {
        if (activeController === controller) activeController = null;
      }
    },
  };
}

// Keep the existing People inspector API; conversation polling uses the same
// latest-wins contract so slow responses cannot rewind the visible timeline.
export const createPersonSelection = createLatestRequest;

// Live inspection must keep moving with the simulated clock. Polls are
// non-overlapping, never replace a deliberate selection, and leave the last
// good snapshot visible if the backend briefly fails.
export function createLivePersonInspector(fetchRecord, showRecord, showSelectionError, showRefreshError) {
  const latest = createLatestRequest();
  let active = null;
  let pending = false;
  let revision = 0;

  async function request(refresh) {
    if (!active || (refresh && (!active.live || pending))) return false;
    const { live, id } = active;
    const current = revision;
    pending = true;
    try {
      await latest.load(
        (signal) => fetchRecord(live, id, signal),
        (record) => { if (revision === current) showRecord(record); },
        (error) => {
          if (revision !== current) return;
          if (refresh) showRefreshError(error);
          else showSelectionError(error);
        },
      );
    } finally {
      if (revision === current) pending = false;
    }
    return true;
  }

  return {
    select(id, live) {
      revision += 1;
      active = { id, live };
      return request(false);
    },
    refresh() { return request(true); },
    invalidate() {
      revision += 1;
      active = null;
      pending = false;
      latest.invalidate();
    },
  };
}

// The live island record and the stored human record are different sources.
// Choose the correct live or stored inspection source. Downloads use the
// already inspected JSON snapshot rather than fetching the endpoint again.
export function personRecordPath(onTheIsland, agentId) {
  return (onTheIsland ? '/api/world/humans/' : '/api/humans/') +
    encodeURIComponent(agentId);
}

// Export the exact inspected snapshot, not a second unauthenticated GET that
// can return a stale/different record (or 401 when the API needs a bearer token).
export function personRecordExport(person) {
  if (!person || !person.human || typeof person.human !== "object" || Array.isArray(person.human)) {
    throw new TypeError("Person export requires an inspected human record");
  }
  const agentId = String(person.summary?.agent_id ?? "");
  const safeName = agentId.replace(/[^A-Za-z0-9._-]/g, "_").replace(/^\.+$/, "_").slice(0, 100) || "person";
  return { filename: safeName + ".json", json: JSON.stringify(person.human, null, 2) };
}
