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

// The stored population and the running island are distinct sources.
// A live island must remain inspectable even when the stored-population
// endpoint is unavailable; never silently substitute stale stored records.
export async function loadPeopleRoster(fetchJson) {
  const [status, world] = await Promise.all([
    fetchJson('/api/status'),
    fetchJson('/api/world'),
  ]);
  // An absent or malformed running flag is not proof that the world stopped.
  // Never replace a live population with unrelated stored records on bad data.
  if (!world || typeof world.running !== 'boolean') {
    throw new TypeError('Invalid island world status');
  }
  const onTheIsland = world.running;
  const roster = await fetchJson(onTheIsland ? '/api/world/humans' : '/api/humans');
  return { status, world, roster, onTheIsland };
}


// The running world's roster changes as people arrive, age and die. Refresh
// it without reselecting the same person or replacing a valid inspector
// snapshot. Stored populations remain a separate, non-polled source.
export function reconcileLiveRoster(roster, currentSelection) {
  // The server publishes unsigned simulation ticks. Reject malformed clocks
  // and records from the future before the UI can describe them as current.
  // Safe integers also avoid silently rounding large JSON tick counters.
  if (!roster || !Array.isArray(roster.people) ||
      !Number.isSafeInteger(roster.tick) || roster.tick < 0 ||
      !Number.isSafeInteger(roster.records_at_tick) ||
      roster.records_at_tick < 0 || roster.records_at_tick > roster.tick) {
    throw new TypeError('Invalid live island roster');
  }
  // Do not turn malformed simulation data into a plausible-looking dashboard.
  // In particular, a truthy string "false" must never be shown as alive, and
  // duplicate IDs would make selection/inspection ambiguous.
  const ids = new Set();
  for (const person of roster.people) {
    if (!person || typeof person.agent_id !== 'string' ||
        !person.agent_id.trim() || person.agent_id !== person.agent_id.trim() ||
        ids.has(person.agent_id) || typeof person.alive !== 'boolean' ||
        typeof person.age_years !== 'number' ||
        !Number.isFinite(person.age_years) || person.age_years < 0) {
      throw new TypeError('Invalid live island roster');
    }
    ids.add(person.agent_id);
  }
  const people = roster.people.map((person) => ({
    agent_id: person.agent_id,
    name: person.agent_id,
    human_id: person.agent_id,
    biological_sex: '',
    status: person.alive ? 'Alive' : 'Dead',
    age_years: person.age_years,
  }));
  const selected = people.some((person) => person.agent_id === currentSelection)
    ? currentSelection
    : (people[0]?.agent_id ?? null);
  return { people, selected, selectionChanged: selected !== currentSelection };
}

// Validate either source before the page adopts it. The stored population
// comes from HumanSummary in population.rs; unlike live snapshots, it has no
// simulation tick, but still must not silently display corrupt identities.
export function preparePeopleRoster(roster, onTheIsland, currentSelection) {
  if (onTheIsland) return reconcileLiveRoster(roster, currentSelection).people;
  if (!Array.isArray(roster)) throw new TypeError('Invalid stored people roster');
  const ids = new Set();
  for (const person of roster) {
    if (!person || typeof person !== 'object' ||
        typeof person.agent_id !== 'string' || !person.agent_id.trim() ||
        person.agent_id !== person.agent_id.trim() || ids.has(person.agent_id) ||
        typeof person.name !== 'string' ||
        typeof person.human_id !== 'string' || !person.human_id.trim() ||
        typeof person.biological_sex !== 'string' ||
        !['male', 'female', 'neutral'].includes(person.biological_sex) ||
        !['alive', 'dead', 'dormant'].includes(person.status) ||
        typeof person.age_years !== 'number' ||
        !Number.isFinite(person.age_years) || person.age_years < 0) {
      throw new TypeError('Invalid stored people roster');
    }
    ids.add(person.agent_id);
  }
  return roster;
}

export function createLiveRosterPoller(fetchRoster, showRoster, showError) {
  let pending = false;
  let revision = 0;
  let controller = null;
  return {
    invalidate() {
      // A hidden or unloading tab must not apply an obsolete roster. Abort
      // the fetch and allow a fresh one when the tab becomes visible again.
      revision += 1;
      controller?.abort();
      controller = null;
      pending = false;
    },
    async refresh() {
      if (pending) return false;
      const request = ++revision;
      const currentController = new AbortController();
      controller = currentController;
      pending = true;
      try {
        const roster = await fetchRoster(currentController.signal);
        if (request === revision) showRoster(roster);
      } catch (error) {
        if (request === revision) showError(error);
      } finally {
        if (request === revision) {
          pending = false;
          controller = null;
        }
      }
      return true;
    },
  };
}
