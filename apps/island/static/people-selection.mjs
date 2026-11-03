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
  let lastTick = null;
  let lastRecordsAtTick = null;

  async function request(refresh) {
    if (!active || (refresh && (!active.live || pending))) return false;
    const { live, id } = active;
    const current = revision;
    pending = true;
    try {
      await latest.load(
        (signal) => fetchRecord(live, id, signal),
        (record) => {
          if (revision !== current) return;
          if (live) {
            // A lagging or misrouted API response must never display another
            // person's private record under the currently selected identity.
            // The Rust inspector always publishes summary.agent_id.
            if (record?.summary?.agent_id !== id) {
              throw new Error('Mismatched live person identity');
            }
            // Live human records carry the published world clock and the
            // last tick at which full human records were copied. A delayed
            // projection must not rewind an inspected person's state.
            const tick = record?.tick;
            const recordsAtTick = record?.records_at_tick;
            if (!Number.isSafeInteger(tick) || tick < 0 ||
                !Number.isSafeInteger(recordsAtTick) || recordsAtTick < 0 ||
                recordsAtTick > tick) {
              throw new TypeError('Invalid live person record clock');
            }
            // Both clocks are monotonic independently. A newer world tick
            // can still carry an older full-record projection from a lagging
            // reader, which must not replace the inspected human.
            if (lastTick !== null &&
                (tick < lastTick || recordsAtTick < lastRecordsAtTick)) {
              throw new Error('Stale live person record');
            }
            showRecord(record);
            lastTick = tick;
            lastRecordsAtTick = recordsAtTick;
          } else {
            showRecord(record);
          }
        },
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
      // Reselecting the same person must not reset its accepted clock.
      // Only a genuinely different person/source starts a new stream.
      if (!active || active.id !== id || active.live !== live) {
        lastTick = null;
        lastRecordsAtTick = null;
      }
      active = { id, live };
      return request(false);
    },
    refresh() { return request(true); },
    invalidate() {
      revision += 1;
      active = null;
      pending = false;
      lastTick = null;
      lastRecordsAtTick = null;
      latest.invalidate();
    },
  };
}

// The live island record and the stored human record are different sources.
// Choose the correct live or stored inspection source. Downloads use the
// already inspected JSON snapshot rather than fetching the endpoint again.
function isInspectablePersonId(agentId) {
  // Keep both roster sources consistent with the authenticated inspector.
  // Dot segments navigate away from the intended endpoint after URL resolution.
  return typeof agentId === 'string' && agentId.length <= 256 && agentId.trim().length > 0 &&
    agentId === agentId.trim() && agentId !== '.' && agentId !== '..' &&
    // Unicode format/bidi controls and C1 controls can spoof visible identities.
    !/[\u0000-\u001f\u007f-\u009f\p{Cf}]/u.test(agentId);
}

export function personRecordPath(onTheIsland, agentId) {
  // encodeURIComponent leaves "." untouched. URL resolution treats "." and
  // ".." as navigation, so a malformed roster ID could redirect an
  // authenticated inspection away from the intended human endpoint.
  if (!isInspectablePersonId(agentId)) {
    throw new TypeError('Invalid person identifier');
  }
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
    if (!person || !isInspectablePersonId(person.agent_id) ||
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
  const humanIds = new Set();
  for (const person of roster) {
    if (!person || typeof person !== 'object' ||
        !isInspectablePersonId(person.agent_id) || ids.has(person.agent_id) ||
        // Stored names are operator-visible attribution, just like live IDs.
        // Refuse invisible/bidi controls, ambiguous padding and empty labels.
        !isInspectablePersonId(person.name) ||
        !isInspectablePersonId(person.human_id) || humanIds.has(person.human_id) ||
        typeof person.biological_sex !== 'string' ||
        !['male', 'female', 'neutral'].includes(person.biological_sex) ||
        !['alive', 'dead', 'dormant'].includes(person.status) ||
        typeof person.age_years !== 'number' ||
        !Number.isFinite(person.age_years) || person.age_years < 0) {
      throw new TypeError('Invalid stored people roster');
    }
    ids.add(person.agent_id);
    humanIds.add(person.human_id);
  }
  return roster;
}

export function createLiveRosterPoller(fetchRoster, showRoster, showError) {
  let pending = false;
  let revision = 0;
  let lastRenderedTick = null;
  let lastRenderedRecordsAtTick = null;
  let controller = null;
  function cancelPending() {
    revision += 1;
    controller?.abort();
    controller = null;
    pending = false;
  }
  return {
    seed(roster) {
      // The initial page load already rendered a live snapshot before polling
      // begins. Without its clock, the first cached poll can rewind that UI.
      // Validate here too: callers must not be able to seed an invalid clock.
      reconcileLiveRoster(roster, null);
      if (lastRenderedTick !== null &&
          (roster.tick < lastRenderedTick ||
           roster.records_at_tick < lastRenderedRecordsAtTick)) {
        throw new Error('Stale live island roster seed');
      }
      lastRenderedTick = roster.tick;
      lastRenderedRecordsAtTick = roster.records_at_tick;
    },
    suspend() {
      // Hiding the page aborts stale HTTP work, but does not prove the
      // simulation restarted. Keep the last accepted clock across visibility
      // changes so a delayed older roster cannot rewind the dashboard.
      cancelPending();
    },
    invalidate() {
      // Explicit new-epoch reset: only use when the simulation is known to
      // have restarted, not for visibility/pagehide.
      cancelPending();
      lastRenderedTick = null;
      lastRenderedRecordsAtTick = null;
    },
    async refresh() {
      if (pending) return false;
      const request = ++revision;
      const currentController = new AbortController();
      controller = currentController;
      pending = true;
      try {
        const roster = await fetchRoster(currentController.signal);
        if (request === revision) {
          // A proxy or delayed server snapshot must not make the visible
          // simulation clock run backwards between completed polls.
          // The world clock and the last full-record projection clock must
          // advance independently. A later world tick may still come from a
          // lagging reader whose human records are older than those shown.
          if (Number.isSafeInteger(roster?.tick) && lastRenderedTick !== null &&
              (roster.tick < lastRenderedTick ||
               (Number.isSafeInteger(roster.records_at_tick) &&
                lastRenderedRecordsAtTick !== null &&
                roster.records_at_tick < lastRenderedRecordsAtTick))) {
            throw new Error('Stale live island roster tick or projection');
          }
          showRoster(roster);
          if (Number.isSafeInteger(roster?.tick)) {
            lastRenderedTick = roster.tick;
            lastRenderedRecordsAtTick = Number.isSafeInteger(roster.records_at_tick)
              ? roster.records_at_tick : null;
          }
        }
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


// Responses can arrive in request order yet contain older published simulation
// projections (e.g. a proxy cache or lagging read). Preserve the last accepted
// clock across hidden-tab suspension. Reset only after an observed no-island
// response, which marks a new simulation epoch.
export function createConversationClockGuard() {
  let lastTick = null;
  return {
    accept(feed) {
      // Legacy stored snapshots may not expose the clock; only the live Rust
      // endpoint guarantees one. Never let an absent tick reset the guard.
      if (feed?.tick === undefined) return true;
      if (!Number.isSafeInteger(feed?.tick) || feed.tick < 0) {
        throw new TypeError('Invalid conversation clock');
      }
      if (lastTick !== null && feed.tick < lastTick) return false;
      lastTick = feed.tick;
      return true;
    },
    reset() { lastTick = null; },
  };
}

// Validate the complete dialogue snapshot before updating any visible card.
// A malformed clock or missing speaker must not masquerade as simulated speech.
export function validateConversationFeed(feed) {
  const invalid = () => { throw new TypeError('Invalid conversation feed'); };
  // Bound both the number of DOM nodes and their text size; a signed-in
  // endpoint returning enormous strings must not exhaust the browser.
  // The Rust projection deliberately publishes at most 40 recent exchanges
  // and 12 lines per exchange (CONVERSATIONS_SHOWN / LINES_SHOWN). Enforce
  // that same UI budget before creating DOM nodes, even for a malformed or
  // unexpectedly oversized authenticated response.
  if (!feed || typeof feed.note !== 'string' || feed.note.length > 2048 ||
      !Array.isArray(feed.conversations) ||
      feed.conversations.length > 40 ||
      (feed.tick !== undefined && (!Number.isSafeInteger(feed.tick) || feed.tick < 0))) invalid();
  for (const conversation of feed.conversations) {
    if (!conversation || !Number.isSafeInteger(conversation.tick) || conversation.tick < 0 ||
        (feed.tick !== undefined && conversation.tick > feed.tick) ||
        (feed.tick !== undefined && conversation.last_tick != null && conversation.last_tick !== 0 &&
          conversation.last_tick > feed.tick) ||
        (conversation.last_tick != null && conversation.last_tick !== 0 &&
          (!Number.isSafeInteger(conversation.last_tick) || conversation.last_tick < conversation.tick)) ||
        // Relationship labels and speaker identities are operator-visible
        // attribution. Reject bidi/format controls and malformed IDs instead
        // of allowing an authenticated but corrupt feed to impersonate people.
        !isInspectablePersonId(conversation.relationship) ||
        conversation.relationship.length > 128 ||
        !Array.isArray(conversation.lines) || conversation.lines.length > 12 ||
        (conversation.lines_said != null &&
          (!Number.isSafeInteger(conversation.lines_said) || conversation.lines_said < 0 ||
            (conversation.lines_said > 0 && conversation.lines_said < conversation.lines.length)))) invalid();
    for (const line of conversation.lines) {
      if (!line || !isInspectablePersonId(line.speaker_name) ||
          (line.speaker_id !== undefined && !isInspectablePersonId(line.speaker_id)) ||
          typeof line.text !== 'string' || !line.text.trim() || line.text.length > 4096) invalid();
    }
  }
  return feed;
}
