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

test('rejects a poisoned remembered backend before reading or sending the owner token', async () => {
  const app = await setup();
  app.rememberToken('owner-secret');
  localStorage.setItem('island-backend', 'https://attacker.example/redirect');
  let calls = 0;
  globalThis.fetch = async () => { calls += 1; throw new Error('must not fetch'); };
  await assert.rejects(app.getJson('/api/status'), /bare HTTP/);
  assert.equal(calls, 0);
});

test('API path validation blocks cross-origin and traversal requests before fetch', async () => {
  const app = await setup();
  app.rememberToken('owner-secret');
  let calls = 0;
  globalThis.fetch = async () => { calls += 1; throw new Error('must not fetch'); };
  for (const path of [
    '//attacker.example/api/world', 'https://attacker.example/api/world',
    '/api/../admin', '/api/../../other', '/api/world#fragment',
    '/api/\\\\attacker.example/world', '/not-api/world',
  ]) {
    assert.throws(() => app.apiUrl(path), /Island API path must stay on this backend/, path);
    await assert.rejects(app.getJson(path), /Island API path must stay on this backend/);
    await assert.rejects(app.postJson(path, { action: 'pause' }, app.storedToken()), /Island API path must stay on this backend/);
    await assert.rejects(app.getImage(path), /Island API path must stay on this backend/);
  }
  assert.equal(calls, 0);
});

test('valid API query paths remain usable with and without a configured backend', async () => {
  const app = await setup();
  assert.equal(app.apiUrl('/api/activity?limit=8'), '/api/activity?limit=8');
  app.setBackend('http://127.0.0.1:8080');
  assert.equal(app.apiUrl('/api/activity?limit=8'), 'http://127.0.0.1:8080/api/activity?limit=8');
});

test('server switch does not silently rebind the prefilled control token to another island', async () => {
  const app = await setup();
  app.rememberToken('old-island-secret');
  const oldOrigin = location.origin;
  const newOrigin = 'https://another-island.example';
  const untouched = app.tokenForBackendSwitch(oldOrigin, newOrigin, app.storedToken(), false);
  assert.equal(untouched, '');
  app.setBackend(newOrigin);
  app.rememberToken(untouched);
  assert.equal(app.storedToken(), '');
  assert.equal(sessionStorage.getItem(app.TOKEN_KEY), null);
});

test('same-island reconnect preserves token and deliberate new-island entry is allowed', async () => {
  const app = await setup();
  const oldOrigin = location.origin;
  assert.equal(app.tokenForBackendSwitch(oldOrigin, oldOrigin, '  existing  ', false), 'existing');
  assert.equal(app.tokenForBackendSwitch(oldOrigin, 'https://new.example', '  new-secret  ', true), 'new-secret');
});

test('authenticated JSON requests forward a caller abort signal to fetch', async () => {
  const app = await setup();
  app.rememberToken('owner-secret');
  const controller = new AbortController();
  let observed;
  globalThis.fetch = async (_url, options) => {
    observed = options;
    return { ok: true, json: async () => ({ ok: true }) };
  };
  await app.getJson('/api/conversations', { signal: controller.signal });
  assert.equal(observed.signal, controller.signal);
  assert.equal(observed.redirect, 'error');
  assert.equal(observed.headers.authorization, 'Bearer owner-secret');
});
