// People inspector requests can finish in any order. A monotonically
// increasing generation also handles A -> B -> A (the earlier A is stale).
export function createPersonSelection() {
  let generation = 0;
  return {
    invalidate() { generation += 1; },
    async load(fetchPerson, showPerson, showError) {
      const request = ++generation;
      try {
        const person = await fetchPerson();
        if (request === generation) showPerson(person);
      } catch (error) {
        if (request === generation) showError(error);
      }
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
