import { test } from 'node:test';
import assert from 'node:assert/strict';

import { createApp } from '../src/app.js';
import { contacts, gmailCredentials } from '../src/config.js';
import { buildRawMessage, encodeHeaderText, MessageFormatError } from '../src/GmailService.js';
import { parseInstantAnswer } from '../src/WebSearchService.js';
import { statusFor } from '../src/routes/computer.js';

function decode(raw) {
  return Buffer.from(raw, 'base64url').toString('utf8');
}

test('raw message is MIME UTF-8 and rejects header injection', () => {
  const raw = decode(buildRawMessage('a@x.test', 'b@x.test', 'Héllo', 'Body ✓'));
  assert.match(raw, /Subject: =\?UTF-8\?B\?/);
  assert.match(raw, /Content-Type: text\/plain; charset="UTF-8"/);
  const body = raw.split('\r\n\r\n')[1];
  assert.equal(Buffer.from(body.replace(/\r\n/g, ''), 'base64').toString('utf8'), 'Body ✓');
  assert.throws(() => buildRawMessage('a@x.test', 'b@x.test', 'Hi\r\nBcc: evil@x.test', 'x'), MessageFormatError);
  assert.equal(encodeHeaderText('plain'), 'plain');
});

test('contacts and credentials come from the environment only', () => {
  const env = { CONTACT_DYLAN: ' d@x.test ', CONTACT_KIRSTY: 'k@x.test', OTHER: 'y' };
  assert.deepEqual(contacts(env), { dylan: 'd@x.test', kirsty: 'k@x.test' });
  assert.equal(gmailCredentials('Gem-D', {}), null);
  const full = { GEM_D_CLIENT_ID: 'i', GEM_D_CLIENT_SECRET: 's', GEM_D_REFRESH_TOKEN: 't', GEM_D_EMAIL: 'g@x.test' };
  assert.equal(gmailCredentials('Gem-D', full).email, 'g@x.test');
});

test('instant answer parsing flattens category groups and splits titles', () => {
  const parsed = parseInstantAnswer('plants', {
    AbstractText: 'Plants are eukaryotes.',
    Heading: 'Plant',
    AbstractURL: 'https://duckduckgo.com/Plant',
    RelatedTopics: [
      { Text: 'Photosynthesis - process used by plants', FirstURL: 'https://duckduckgo.com/Photosynthesis' },
      { Name: 'Botany', Topics: [{ Text: 'Root - plant organ', FirstURL: 'https://duckduckgo.com/Root' }] },
    ],
  });
  assert.equal(parsed.results.length, 3);
  assert.deepEqual(parsed.results[1], {
    title: 'Photosynthesis',
    snippet: 'process used by plants',
    url: 'https://duckduckgo.com/Photosynthesis',
  });
  assert.equal(parsed.results[2].title, 'Root');
  assert.equal(parsed.satisfaction, 1.0);
  assert.equal(parseInstantAnswer('x', {}).satisfaction, 0.0);
});

async function withServer(services, run, { token } = {}) {
  // Pass the token explicitly so the tests never depend on the caller's env.
  const server = createApp(services, { token }).listen(0);
  await new Promise((resolve) => server.once('listening', resolve));
  const base = `http://127.0.0.1:${server.address().port}`;
  try {
    await run(base);
  } finally {
    server.close();
  }
}

function post(url, body) {
  return fetch(url, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
}

test('routes validate agents and contacts and report real state', async () => {
  const sent = [];
  const gmail = {
    isConfigured: (agent) => agent === 'Gem-D',
    sendEmail: async (agent, to, subject) => {
      sent.push({ agent, to, subject });
      return { id: 'msg-1' };
    },
    unreadInboxCount: async () => 7,
  };
  const search = {
    webSearch: async () => {
      const err = new Error('rate limited');
      err.status = 429;
      throw err;
    },
  };
  const env = { CONTACT_DYLAN: 'd@x.test' };

  await withServer({ gmail, search, env }, async (base) => {
    assert.equal((await post(`${base}/api/computer/Nobody/search`, { query: 'x' })).status, 404);
    assert.equal((await post(`${base}/api/computer/Gem-D/search`, { query: 'x' })).status, 429);

    const unknown = await post(`${base}/api/computer/Gem-D/email`, { to: 'stranger@x.test', subject: 's', body: 'b' });
    assert.equal(unknown.status, 400);
    assert.equal(sent.length, 0);

    const ok = await post(`${base}/api/computer/Gem-D/email`, { to: 'dylan', subject: 'Checking in', body: 'Hi' });
    assert.equal(ok.status, 200);
    assert.deepEqual(await ok.json(), { message_id: 'msg-1', to: 'dylan', subject: 'Checking in' });
    assert.deepEqual(sent, [{ agent: 'Gem-D', to: 'd@x.test', subject: 'Checking in' }]);

    const unconfigured = await post(`${base}/api/computer/Gem-K/email`, { to: 'dylan', subject: 's', body: 'b' });
    assert.equal(unconfigured.status, 401);

    assert.deepEqual(await (await fetch(`${base}/api/computer/Gem-D/state`)).json(), {
      agent_id: 'Gem-D',
      powered_on: true,
      unread_count: 7,
    });
    assert.deepEqual(await (await fetch(`${base}/api/computer/Gem-K/state`)).json(), {
      agent_id: 'Gem-K',
      powered_on: false,
      unread_count: 0,
    });
  });
});

test('a configured token is required on every computer route', async () => {
  const gmail = { isConfigured: () => false, unreadInboxCount: async () => 0 };
  await withServer({ gmail, env: {} }, async (base) => {
    assert.equal((await fetch(`${base}/api/computer/Gem-D/state`)).status, 401);
    const wrong = await fetch(`${base}/api/computer/Gem-D/state`, {
      headers: { authorization: 'Bearer nope' },
    });
    assert.equal(wrong.status, 401);
    const right = await fetch(`${base}/api/computer/Gem-D/state`, {
      headers: { authorization: 'Bearer s3cret' },
    });
    assert.equal(right.status, 200);
    assert.equal((await fetch(`${base}/api/health`)).status, 200);
  }, { token: 's3cret' });
});

test('upstream failures map to the status the bridge understands', () => {
  // A string `code` must not hide the HTTP status carried elsewhere.
  assert.equal(statusFor({ code: 'ERR_BAD_REQUEST', response: { status: 403 } }), 403);
  assert.equal(statusFor({ code: 429 }), 429);
  assert.equal(statusFor({ status: null, response: { status: 401 } }), 401);
  assert.equal(statusFor({ code: 'ETIMEDOUT' }), 504);
  assert.equal(statusFor({ name: 'AbortError' }), 504);
  assert.equal(statusFor({ code: 'ECONNRESET' }), 503);
  assert.equal(statusFor({ response: { status: 500 } }), 503);
});
