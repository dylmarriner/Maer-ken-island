import test from 'node:test';
import assert from 'node:assert/strict';

let serial = 0;
function storage() {
  const values = new Map();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, String(value)),
    removeItem: (key) => values.delete(key),
  };
}
async function setup(search = '') {
  globalThis.location = { origin: 'https://island.example', search };
  globalThis.window = { ISLAND_BACKEND: '' };
  globalThis.localStorage = storage();
  globalThis.sessionStorage = storage();
  return import(`../static/app.js?isolation-test=${++serial}`);
}

test('a token is bound to the backend origin and never follows a changed server', async () => {
  const app = await setup();
  app.setBackend('http://127.0.0.1:8080/');
  app.rememberToken('owner-secret');
  assert.equal(app.storedToken(), 'owner-secret');
  assert.equal(sessionStorage.getItem(app.TOKEN_ORIGIN_KEY), 'http://127.0.0.1:8080');
  app.setBackend('https://attacker.example');
  assert.equal(app.storedToken(), '');
  app.setBackend('http://127.0.0.1:8080');
  assert.equal(app.storedToken(), 'owner-secret');
  app.rememberToken('');
  assert.equal(app.storedToken(), '');
  assert.equal(sessionStorage.getItem(app.TOKEN_ORIGIN_KEY), null);
});

test('untrusted backend query cannot receive a token saved for another island', async () => {
  const app = await setup('?backend=https%3A%2F%2Fattacker.example');
  sessionStorage.setItem(app.TOKEN_KEY, 'owner-secret');
  sessionStorage.setItem(app.TOKEN_ORIGIN_KEY, 'https://island.example');
  const calls = [];
  globalThis.fetch = async (url, options) => {
    calls.push({ url, options });
    return { ok: true, json: async () => ({ ok: true }) };
  };
  assert.equal(app.backend(), 'https://attacker.example');
  assert.equal(app.storedToken(), '');
  await app.getJson('/api/status');
  assert.equal(calls[0].url, 'https://attacker.example/api/status');
  assert.equal(calls[0].options.headers.authorization, undefined);
  assert.equal(calls[0].options.redirect, 'error');
});

test('legacy unscoped tokens are not forwarded until explicitly re-entered', async () => {
  const app = await setup();
  sessionStorage.setItem(app.TOKEN_KEY, 'legacy-secret');
  assert.equal(app.storedToken(), '');
  app.rememberToken('new-secret');
  assert.equal(app.storedToken(), 'new-secret');
  assert.equal(sessionStorage.getItem(app.TOKEN_ORIGIN_KEY), location.origin);
});

test('rejects ambiguous backend URLs before any request can be sent', async () => {
  const app = await setup();
  for (const value of [
    'javascript:alert(1)', 'https://user:password@island.example',
    'https://island.example/evil', 'https://island.example/?target=evil',
    'https://island.example#fragment', 'https://island.example\\@evil.example',
    'https://island.example////',
  ]) {
    assert.throws(() => app.tidyBackend(value), /bare HTTP\(S\) origin/, value);
  }
  assert.equal(app.tidyBackend('HTTPS://ISLAND.EXAMPLE:443/'), 'https://island.example');
});

test('all JSON, command and image requests reject redirects and keep tokens on their bound origin', async () => {
  const app = await setup();
  app.rememberToken('owner-secret');
  const calls = [];
  globalThis.fetch = async (url, options) => {
    calls.push({ url, options });
    return {
      ok: true, status: 200,
      json: async () => ({ accepted: true }),
      blob: async () => new Blob(['png']),
    };
  };
  await app.getJson('/api/world');
  await app.postJson('/api/control', { action: 'pause' }, app.storedToken());
  const objectUrl = await app.getImage('/api/map.png');
  URL.revokeObjectURL(objectUrl);
  assert.equal(calls.length, 3);
  for (const call of calls) {
    assert.equal(call.options.redirect, 'error');
    assert.equal(call.options.headers.authorization, 'Bearer owner-secret');
    assert.ok(call.url.startsWith('/api/'));
  }
});
