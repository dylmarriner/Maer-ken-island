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
