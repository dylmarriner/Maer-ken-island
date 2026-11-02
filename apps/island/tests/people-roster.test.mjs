import test from 'node:test';
import assert from 'node:assert/strict';
import { loadPeopleRoster, reconcileLiveRoster, createLiveRosterPoller, preparePeopleRoster } from '../static/people-selection.mjs';

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

test('malformed world status cannot silently substitute unrelated stored people', async () => {
  for (const world of [null, {}, { running: 'false' }, { running: 0 }, { running: null }]) {
    const requested = [];
    const fetch = async (path) => {
      requested.push(path);
      if (path === '/api/status') return { population: 3 };
      if (path === '/api/world') return world;
      if (path === '/api/humans') return [{ agent_id: 'stale-person' }];
      throw new Error('unexpected live request');
    };
    await assert.rejects(loadPeopleRoster(fetch), /Invalid island world status/);
    assert.deepEqual(requested.sort(), ['/api/status', '/api/world']);
  }
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
  const first = reconcileLiveRoster({ tick: 15, records_at_tick: 14, people: [
    { agent_id: 'Gem-D', alive: true, age_years: 28 },
    { agent_id: 'Gem-K', alive: false, age_years: 34 },
  ] }, 'Gem-D');
  assert.equal(first.selected, 'Gem-D');
  assert.equal(first.selectionChanged, false);
  assert.deepEqual(first.people.map((p) => [p.agent_id, p.status]), [['Gem-D', 'Alive'], ['Gem-K', 'Dead']]);
  const next = reconcileLiveRoster({ tick: 16, records_at_tick: 16, people: [{ agent_id: 'Gem-K', alive: true, age_years: 34 }] }, 'Gem-D');
  assert.equal(next.selected, 'Gem-K');
  assert.equal(next.selectionChanged, true);
  const empty = reconcileLiveRoster({ tick: 17, records_at_tick: 17, people: [] }, 'Gem-K');
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


test('live roster polling rejects an older simulation tick rather than rewinding the dashboard', async () => {
  const ticks = [12, 11, 13];
  const shown = [];
  const errors = [];
  const poller = createLiveRosterPoller(
    async () => ({ tick: ticks.shift(), people: [] }),
    (roster) => shown.push(roster.tick),
    (error) => errors.push(error.message),
  );
  await poller.refresh();
  await poller.refresh();
  await poller.refresh();
  assert.deepEqual(shown, [12, 13]);
  assert.match(errors[0], /stale.*tick/i);
});

test('invalidating a live roster poller permits a fresh simulation epoch', async () => {
  const ticks = [12, 1];
  const shown = [];
  const poller = createLiveRosterPoller(
    async () => ({ tick: ticks.shift(), people: [] }),
    (roster) => shown.push(roster.tick),
    (error) => assert.fail(error.message),
  );
  await poller.refresh();
  poller.invalidate();
  await poller.refresh();
  assert.deepEqual(shown, [12, 1]);
});

test('hidden-tab invalidation aborts the old roster and cannot overwrite a fresh roster', async () => {
  let oldResolve;
  const old = new Promise((resolve) => { oldResolve = resolve; });
  const seen = [];
  const signals = [];
  let calls = 0;
  const poller = createLiveRosterPoller((signal) => {
    signals.push(signal);
    return ++calls === 1 ? old : Promise.resolve({ tick: 20, people: [] });
  }, (value) => seen.push(value.tick), (error) => seen.push(error.message));
  const first = poller.refresh();
  poller.invalidate();
  assert.equal(signals[0].aborted, true);
  assert.equal(await poller.refresh(), true);
  oldResolve({ tick: 19, people: [] });
  await first;
  assert.deepEqual(seen, [20]);
});

test('invalidated roster errors are suppressed and new requests may start', async () => {
  let oldReject;
  const old = new Promise((_resolve, reject) => { oldReject = reject; });
  const errors = [];
  const poller = createLiveRosterPoller(() => old, () => {}, (error) => errors.push(error.message));
  const first = poller.refresh();
  poller.invalidate();
  oldReject(new Error('stale error'));
  await first;
  assert.deepEqual(errors, []);
});

test('malformed live roster is rejected before replacing the visible population', () => {
  assert.throws(() => reconcileLiveRoster({ tick: 5, records_at_tick: 5, people: null }, 'Gem-D'), /Invalid live island roster/);
  assert.throws(() => reconcileLiveRoster({ tick: 5, records_at_tick: 5, people: [{ agent_id: null }] }, 'Gem-D'), /Invalid live island roster/);
});


test('live roster rejects duplicate or malformed identities and impossible life fields', () => {
  const valid = { agent_id: 'Gem-D', alive: true, age_years: 28 };
  for (const people of [
    [valid, { ...valid }],
    [{ ...valid, agent_id: '' }],
    [{ ...valid, agent_id: '  ' }],
    [{ ...valid, alive: 'false' }],
    [{ ...valid, alive: null }],
    [{ ...valid, age_years: -1 }],
    [{ ...valid, age_years: Infinity }],
    [{ ...valid, age_years: '28' }],
  ]) {
    assert.throws(() => reconcileLiveRoster({ tick: 5, records_at_tick: 5, people }, 'Gem-D'), /Invalid live island roster/);
  }
});

test('live and stored rosters reject uninspectable dot-segment and control-byte identities', () => {
  const live = { tick: 5, records_at_tick: 5, people: [{ agent_id: 'Gem-D', alive: true, age_years: 28 }] };
  const stored = {
    agent_id: 'HUM-000001', name: 'Kauri', human_id: 'human-1',
    biological_sex: 'male', status: 'alive', age_years: 30,
  };
  for (const id of ['.', '..', 'Gem\u0000D', 'Gem\u007fD']) {
    assert.throws(
      () => reconcileLiveRoster({ ...live, people: [{ ...live.people[0], agent_id: id }] }, null),
      /Invalid live island roster/,
      'live: ' + JSON.stringify(id),
    );
    assert.throws(
      () => preparePeopleRoster([{ ...stored, agent_id: id }], false, null),
      /Invalid stored people roster/,
      'stored: ' + JSON.stringify(id),
    );
  }
});

test('roster and inspector reject invisible, bidi and oversized person identifiers', () => {
  const live = { tick: 5, records_at_tick: 5, people: [{ agent_id: 'Gem-D', alive: true, age_years: 28 }] };
  const stored = {
    agent_id: 'HUM-000001', name: 'Kauri', human_id: 'human-1',
    biological_sex: 'male', status: 'alive', age_years: 30,
  };
  for (const id of ['Gem\u202eD', 'Gem\u2066D', 'Gem\u0085D', 'x'.repeat(257)]) {
    assert.throws(() => reconcileLiveRoster({ ...live, people: [{ ...live.people[0], agent_id: id }] }, null), /Invalid live island roster/);
    assert.throws(() => preparePeopleRoster([{ ...stored, agent_id: id }], false, null), /Invalid stored people roster/);
    assert.throws(() => preparePeopleRoster([{ ...stored, human_id: id }], false, null), /Invalid stored people roster/);
  }
});

test('live roster rejects impossible simulation clock and record freshness before display', () => {
  const people = [{ agent_id: 'Gem-D', alive: true, age_years: 28 }];
  const valid = { tick: 102, records_at_tick: 100, people };
  assert.equal(reconcileLiveRoster(valid, 'Gem-D').selected, 'Gem-D');
  for (const [tick, records_at_tick] of [
    [undefined, 100], [102, undefined], ['102', 100],
    [-1, 0], [1.5, 1], [Infinity, 0], [Number.MAX_SAFE_INTEGER + 1, 0],
    [102, -1], [102, 103], [102, 99.5], [102, NaN],
  ]) {
    assert.throws(
      () => reconcileLiveRoster({ tick, records_at_tick, people }, 'Gem-D'),
      /Invalid live island roster/,
      `invalid tick=${tick} records_at_tick=${records_at_tick}`,
    );
  }
});


test('stored summaries are validated before the dashboard replaces its current roster', () => {
  const valid = {
    agent_id: 'HUM-000001', name: 'Kauri', human_id: 'human-1',
    biological_sex: 'male', status: 'alive', age_years: 30,
  };
  assert.deepEqual(preparePeopleRoster([valid], false, null), [valid]);
  for (const roster of [
    null, {}, [valid, { ...valid }], [{ ...valid, age_years: '30' }],
    [{ ...valid, age_years: Infinity }], [{ ...valid, status: 'missing' }],
    [{ ...valid, agent_id: ' ' }], [{ ...valid, human_id: '' }],
    [valid, { ...valid, agent_id: 'HUM-000002' }],
    [{ ...valid, human_id: ' human-1' }],
    [{ ...valid, agent_id: 'HUM-000001\u0000' }],
  ]) {
    assert.throws(() => preparePeopleRoster(roster, false, null), /Invalid stored people roster/);
  }
});

test('live roster is validated before the dashboard adopts its population', () => {
  const valid = { tick: 7, records_at_tick: 7, people: [{ agent_id: 'Gem-D', alive: true, age_years: 28 }] };
  assert.equal(preparePeopleRoster(valid, true, 'Gem-D')[0].agent_id, 'Gem-D');
  assert.throws(() => preparePeopleRoster({ ...valid, tick: '7' }, true, 'Gem-D'), /Invalid live island roster/);
});
