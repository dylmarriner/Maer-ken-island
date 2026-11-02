import test from 'node:test';
import assert from 'node:assert/strict';
import { createPersonSelection, createLatestRequest, createLivePersonInspector, personRecordPath, personRecordExport, validateConversationFeed, createConversationClockGuard } from '../static/people-selection.mjs';

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

test('late success and failure never replace a newer person or add stale retry controls', async () => {
  const selection = createPersonSelection();
  const first = deferred();
  const second = deferred();
  const visible = [];
  const a = selection.load(() => first.promise, (person) => visible.push(person), () => visible.push('old retry'));
  const b = selection.load(() => second.promise, (person) => visible.push(person), () => visible.push('new retry'));
  second.resolve('person B');
  await b;
  first.reject(new Error('old failed request'));
  await a;
  assert.deepEqual(visible, ['person B']);
});

test('A to B to A never lets the first A overwrite the latest A', async () => {
  const selection = createPersonSelection();
  const oldA = deferred();
  const b = deferred();
  const newA = deferred();
  const visible = [];
  const render = (value) => visible.push(value);
  const first = selection.load(() => oldA.promise, render, render);
  const middle = selection.load(() => b.promise, render, render);
  const last = selection.load(() => newA.promise, render, render);
  newA.resolve('A latest');
  await last;
  oldA.resolve('A stale');
  b.resolve('B stale');
  await Promise.all([first, middle]);
  assert.deepEqual(visible, ['A latest']);
});

test('latest failure remains visible and invalidation suppresses an obsolete response', async () => {
  const selection = createPersonSelection();
  const pending = deferred();
  const visible = [];
  const first = selection.load(() => pending.promise, (value) => visible.push(value), () => visible.push('retry'));
  selection.invalidate();
  pending.resolve('old person');
  await first;
  assert.deepEqual(visible, []);
  await selection.load(async () => { throw new Error('current failure'); },
    (value) => visible.push(value), (error) => visible.push(error.message));
  assert.deepEqual(visible, ['current failure']);
});

test('record download and inspector share the correct live or stored source', () => {
  assert.equal(personRecordPath(true, 'Gem-D'), '/api/world/humans/Gem-D');
  assert.equal(personRecordPath(false, 'Gem-D'), '/api/humans/Gem-D');
  assert.equal(personRecordPath(true, 'person/name?x=1'), '/api/world/humans/person%2Fname%3Fx%3D1');
});

test('person inspection rejects dot-segment and malformed identifiers before building authenticated URLs', () => {
  for (const id of ['.', '..', '', '  ', ' Gem-D', 'Gem-D ', 'Gem\u0000D', null, 7]) {
    assert.throws(() => personRecordPath(true, id), /Invalid person identifier/, String(id));
    assert.throws(() => personRecordPath(false, id), /Invalid person identifier/, String(id));
  }
  for (const live of [true, false]) {
    const path = personRecordPath(live, 'Gem-D/other?mode=1#fragment');
    const url = new URL(path, 'https://island.example');
    assert.equal(url.pathname, (live ? '/api/world/humans/' : '/api/humans/') + 'Gem-D%2Fother%3Fmode%3D1%23fragment');
    assert.equal(url.search, '');
    assert.equal(url.hash, '');
  }
});

test('download exports exactly the inspected snapshot without a second HTTP request', () => {
  const person = { summary: { agent_id: 'Gem-D' }, human: { tick: 42, alive: true, nested: { mood: 'resting' } } };
  const result = personRecordExport(person);
  assert.equal(result.filename, 'Gem-D.json');
  assert.equal(result.json, JSON.stringify(person.human, null, 2));
  person.human = { tick: 43 };
  assert.equal(JSON.parse(result.json).tick, 42);
});

test('download filenames cannot contain path separators or reserved characters', () => {
  const person = { summary: { agent_id: '../weird/name?x=1' }, human: { id: 1 } };
  assert.equal(personRecordExport(person).filename, '.._weird_name_x_1.json');
  assert.equal(personRecordExport({ summary: { agent_id: '...' }, human: {} }).filename, '_.json');
  assert.throws(() => personRecordExport({ summary: { agent_id: 'a' } }), /inspected human record/);
});

// The conversation panel polls every five seconds; a slow old poll or late
// error must not rewind or hide a more recent conversation snapshot.
test('conversation polling renders only the latest completed snapshot', async () => {
  const latest = createLatestRequest();
  const slow = deferred();
  const fast = deferred();
  const shown = [];
  const first = latest.load(() => slow.promise, (data) => shown.push(data.tick), () => shown.push('stale error'));
  const second = latest.load(() => fast.promise, (data) => shown.push(data.tick), () => shown.push('current error'));
  fast.resolve({ tick: 102 });
  await second;
  slow.resolve({ tick: 101 });
  await first;
  assert.deepEqual(shown, [102]);
});

test('a late conversation poll failure cannot hide a newer successful panel', async () => {
  const latest = createLatestRequest();
  const slow = deferred();
  const visible = [];
  const first = latest.load(() => slow.promise, () => visible.push('old'), () => visible.push('hide'));
  await latest.load(async () => ({ tick: 104 }), (data) => visible.push(data.tick), () => visible.push('hide'));
  slow.reject(new Error('temporary timeout'));
  await first;
  assert.deepEqual(visible, [104]);
});

test('new selection cancels the previous network request without cancelling the newest', async () => {
  const latest = createLatestRequest();
  const old = deferred();
  const signals = [];
  const shown = [];
  const first = latest.load((signal) => { signals.push(signal); return old.promise; },
    (value) => shown.push(value), () => shown.push('old error'));
  const second = latest.load((signal) => { signals.push(signal); return Promise.resolve('current'); },
    (value) => shown.push(value), () => shown.push('new error'));
  await second;
  assert.equal(signals.length, 2);
  assert.ok(signals[0] instanceof AbortSignal);
  assert.equal(signals[0].aborted, true);
  assert.equal(signals[1].aborted, false);
  old.reject(new DOMException('aborted', 'AbortError'));
  await first;
  assert.deepEqual(shown, ['current']);
});

test('invalidating the inspector cancels a pending request', async () => {
  const latest = createLatestRequest();
  const pending = deferred();
  let signal;
  const first = latest.load((s) => { signal = s; return pending.promise; },
    () => assert.fail('stale result'), () => assert.fail('stale error'));
  latest.invalidate();
  assert.equal(signal.aborted, true);
  pending.reject(new DOMException('aborted', 'AbortError'));
  await first;
});

test('live inspector refreshes the same person without overlapping requests', async () => {
  const pending = deferred();
  const fetched = [];
  const shown = [];
  const inspector = createLivePersonInspector(
    (live, id, signal) => {
      fetched.push({ live, id, signal });
      return fetched.length === 2 ? pending.promise : Promise.resolve({ tick: fetched.length });
    },
    (record) => shown.push(record.tick),
    () => assert.fail('unexpected selection failure'),
    () => assert.fail('unexpected refresh failure'),
  );
  await inspector.select('Gem-D', true);
  const refresh = inspector.refresh();
  assert.equal(await inspector.refresh(), false);
  pending.resolve({ tick: 2 });
  assert.equal(await refresh, true);
  assert.deepEqual(shown, [1, 2]);
  assert.deepEqual(fetched.map(({ live, id }) => [live, id]), [[true, 'Gem-D'], [true, 'Gem-D']]);
});

test('changing selection cancels a pending live refresh without stale rendering', async () => {
  const pending = deferred();
  const shown = [];
  let signal;
  const inspector = createLivePersonInspector(
    (_live, id, currentSignal) => {
      if (id === 'Gem-D' && shown.length) {
        signal = currentSignal;
        return pending.promise;
      }
      return Promise.resolve({ id });
    },
    (record) => shown.push(record.id),
    () => assert.fail('unexpected selection failure'),
    () => assert.fail('unexpected refresh failure'),
  );
  await inspector.select('Gem-D', true);
  const refresh = inspector.refresh();
  await inspector.select('Gem-K', true);
  assert.equal(signal.aborted, true);
  pending.resolve({ id: 'obsolete Gem-D' });
  await refresh;
  assert.deepEqual(shown, ['Gem-D', 'Gem-K']);
});

test('failed live refresh preserves inspected record and retries; stored records do not poll', async () => {
  const shown = [];
  const errors = [];
  let calls = 0;
  const inspector = createLivePersonInspector(
    async () => {
      if (++calls === 3) throw new Error('temporary offline');
      return { tick: calls };
    },
    (record) => shown.push(record.tick),
    () => assert.fail('unexpected selection failure'),
    (error) => errors.push(error.message),
  );
  await inspector.select('Gem-D', false);
  assert.equal(await inspector.refresh(), false);
  await inspector.select('Gem-D', true);
  assert.equal(await inspector.refresh(), true);
  assert.equal(await inspector.refresh(), true);
  assert.deepEqual(shown, [1, 2, 4]);
  assert.deepEqual(errors, ['temporary offline']);
});


test('conversation feed accepts a real engine exchange and older optional counters', () => {
  const valid = { note: 'No language model', conversations: [{
    tick: 41, last_tick: 43, lines_said: 2, relationship: 'founders',
    lines: [{ speaker_name: 'Gem-D', text: 'Kia ora' }, { speaker_name: 'Gem-K', text: 'Hello' }],
  }] };
  assert.equal(validateConversationFeed(valid), valid);
  const legacy = { note: 'Older engine', conversations: [{
    tick: 41, last_tick: 0, lines_said: 0, relationship: 'founders', lines: [{ speaker_name: 'Gem-D', text: 'Hello' }],
  }] };
  assert.equal(validateConversationFeed(legacy), legacy);
});

test('conversation feed enforces the engine projection budget before rendering', () => {
  // Must match CONVERSATIONS_SHOWN=40 and LINES_SHOWN=12 in projection.rs.
  const line = { speaker_name: 'Gem-D', text: 'Kia ora' };
  const conversation = { tick: 41, last_tick: 43, lines_said: 13,
    relationship: 'founders', lines: Array.from({ length: 12 }, () => ({ ...line })) };
  const feed = { note: 'No language model', conversations: Array.from({ length: 40 }, () => ({ ...conversation })) };
  assert.equal(validateConversationFeed(feed), feed);
  assert.throws(() => validateConversationFeed({ ...feed, conversations: [...feed.conversations, conversation] }), /Invalid conversation feed/);
  assert.throws(() => validateConversationFeed({ ...feed, conversations: [{ ...conversation, lines: [...conversation.lines, line] }] }), /Invalid conversation feed/);
});

test('conversation feed rejects invalid clocks, missing speakers and dishonest line counts', () => {
  const base = { note: 'No language model', conversations: [{
    tick: 41, last_tick: 43, lines_said: 2, relationship: 'founders',
    lines: [{ speaker_name: 'Gem-D', text: 'Kia ora' }],
  }] };
  const invalid = [
    null, { conversations: [] }, { note: 'n', conversations: null },
    { ...base, conversations: [{ ...base.conversations[0], tick: -1 }] },
    { ...base, conversations: [{ ...base.conversations[0], last_tick: 40 }] },
    { ...base, conversations: [{ ...base.conversations[0], lines_said: 0.5 }] },
    { ...base, conversations: [{ ...base.conversations[0], lines_said: -1 }] },
    { ...base, conversations: [{ ...base.conversations[0], lines: [{ speaker_name: '', text: 'hello' }] }] },
    { ...base, conversations: [{ ...base.conversations[0], lines: [{ speaker_name: 'Gem-D', text: '   ' }] }] },
  ];
  for (const feed of invalid) {
    assert.throws(() => validateConversationFeed(feed), /Invalid conversation feed/);
  }
});


test('conversation projection clock never rewinds on delayed polls or tab restoration', () => {
  const clock = createConversationClockGuard();
  assert.equal(clock.accept({ tick: 102 }), true);
  assert.equal(clock.accept({ tick: 101 }), false);
  assert.equal(clock.accept({ tick: 102 }), true);
  assert.equal(clock.accept({ tick: 103 }), true);
  // Hiding the tab does not reset the accepted clock.
  assert.equal(clock.accept({ tick: 99 }), false);
  // An observed 404 (no running island) permits the next island epoch.
  clock.reset();
  assert.equal(clock.accept({ tick: 0 }), true);
  for (const tick of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1, NaN]) {
    assert.throws(() => clock.accept({ tick }), /Invalid conversation clock/);
  }
});
