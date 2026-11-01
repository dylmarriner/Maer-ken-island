import test from 'node:test';
import assert from 'node:assert/strict';
import { createPersonSelection, personRecordPath, personRecordExport } from '../static/people-selection.mjs';

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
