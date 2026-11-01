import test from 'node:test';
import assert from 'node:assert/strict';
import { loadPeopleRoster } from '../static/people-selection.mjs';

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
