import test from 'node:test';
import assert from 'node:assert/strict';
import { createPersonSelection } from '../static/people-selection.mjs';

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
