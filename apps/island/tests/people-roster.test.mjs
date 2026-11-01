import test from 'node:test';
import assert from 'node:assert/strict';
import { loadPeopleRoster, reconcileLiveRoster, createLiveRosterPoller } from '../static/people-selection.mjs';

test('live island roster does not depend on stale or inaccessible stored population', async () => {
  const requested = [];
  const fetch = async (path) => {
    requested.push(path);
    if (path === '/api/status') return { population: 8 };
    if (path === '/api/world') return { running: true, clock: { tick: 15 } };
    if (path === '/api/world/humans') return { people: [{ agent_id: 'Gem-D' }], tick: 15 };
    if (path === '/api/humans') throw new Error('stored population unavailable');
    throw new Error('unexpected endpoint');
  };
  const result = await loadPeopleRoster(fetch);
  assert.equal(result.onTheIsland, true);
  assert.deepEqual(result.roster.people, [{ agent_id: 'Gem-D' }]);
  assert.equal(requested.includes('/api/humans'), false);
});

test('stored-only population is loaded when there is no running island', async () => {
  const requested = [];
  const fetch = async (path) => {
    requested.push(path);
    if (path === '/api/status') return { population: 1 };
    if (path === '/api/world') return { running: false };
    if (path === '/api/humans') return [{ agent_id: 'HUM-000001' }];
    throw new Error('unexpected live endpoint');
  };
  const result = await loadPeopleRoster(fetch);
  assert.equal(result.onTheIsland, false);
  assert.deepEqual(result.roster, [{ agent_id: 'HUM-000001' }]);
  assert.equal(requested.includes('/api/world/humans'), false);
});

test('live roster failure is not silently replaced by unrelated stored people', async () => {
  const fetch = async (path) => {
    if (path === '/api/status') return { population: 3 };
    if (path === '/api/world') return { running: true };
    if (path === '/api/world/humans') throw new Error('live roster unavailable');
    throw new Error('wrong source');
  };
  await assert.rejects(loadPeopleRoster(fetch), /live roster unavailable/);
});


test('live roster reconciliation retains selected person and detects arrivals and departures', () => {
  const first = reconcileLiveRoster({ people: [
    { agent_id: 'Gem-D', alive: true, age_years: 28 },
    { agent_id: 'Gem-K', alive: false, age_years: 34 },
  ] }, 'Gem-D');
  assert.equal(first.selected, 'Gem-D');
  assert.equal(first.selectionChanged, false);
  assert.deepEqual(first.people.map((p) => [p.agent_id, p.status]), [['Gem-D', 'Alive'], ['Gem-K', 'Dead']]);
  const next = reconcileLiveRoster({ people: [{ agent_id: 'Gem-K', alive: true, age_years: 34 }] }, 'Gem-D');
  assert.equal(next.selected, 'Gem-K');
  assert.equal(next.selectionChanged, true);
  const empty = reconcileLiveRoster({ people: [] }, 'Gem-K');
  assert.equal(empty.selected, null);
  assert.equal(empty.selectionChanged, true);
});

test('live roster refresh never overlaps and preserves last good roster after errors', async () => {
  let finish;
  const gate = new Promise((resolve) => { finish = resolve; });
  const shown = [];
  const errors = [];
  let calls = 0;
  const poller = createLiveRosterPoller(async () => {
    calls++;
    if (calls === 1) return gate;
    throw new Error('temporary unavailable');
  }, (value) => shown.push(value.tick), (error) => errors.push(error.message));
  const pending = poller.refresh();
  assert.equal(await poller.refresh(), false);
  assert.equal(calls, 1);
  finish({ tick: 9, people: [] });
  assert.equal(await pending, true);
  assert.equal(await poller.refresh(), true);
  assert.deepEqual(shown, [9]);
  assert.deepEqual(errors, ['temporary unavailable']);
});


test('malformed live roster is rejected before replacing the visible population', () => {
  assert.throws(() => reconcileLiveRoster({ people: null }, 'Gem-D'), /Invalid live island roster/);
  assert.throws(() => reconcileLiveRoster({ people: [{ agent_id: null }] }, 'Gem-D'), /Invalid live island roster/);
});
