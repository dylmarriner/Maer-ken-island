// Real Gmail access via the OAuth2 refresh-token flow. Credentials come only
// from the environment (src/config.js). This service never touches an
// account password: a refresh token is independently revocable in the
// Google account's "Third-party access" settings.

import { google } from 'googleapis';
import { gmailCredentials } from './config.js';

export class GmailConfigError extends Error {}
export class MessageFormatError extends Error {}

function clientFor(agentId) {
  const creds = gmailCredentials(agentId);
  if (!creds) {
    throw new GmailConfigError(`no Gmail credentials configured for agent '${agentId}'`);
  }
  const oauth2Client = new google.auth.OAuth2(creds.clientId, creds.clientSecret);
  oauth2Client.setCredentials({ refresh_token: creds.refreshToken });
  return { gmail: google.gmail({ version: 'v1', auth: oauth2Client }), email: creds.email };
}

function assertHeaderSafe(name, value) {
  if (/[\r\n]/.test(value)) {
    throw new MessageFormatError(`${name} must not contain line breaks`);
  }
}

/** RFC 2047 encoded-word for non-ASCII header text; plain ASCII passes through. */
export function encodeHeaderText(text) {
  // eslint-disable-next-line no-control-regex
  return /^[\x20-\x7e]*$/.test(text) ? text : `=?UTF-8?B?${Buffer.from(text, 'utf8').toString('base64')}?=`;
}

/**
 * RFC 5322 message, base64url-encoded as the Gmail API expects.
 * Header values are checked for CR/LF so a subject or address cannot inject
 * extra headers; the body is sent as UTF-8 text in base64 transfer encoding.
 */
export function buildRawMessage(from, to, subject, body) {
  assertHeaderSafe('From', from);
  assertHeaderSafe('To', to);
  assertHeaderSafe('Subject', subject);
  const encodedBody = Buffer.from(body, 'utf8').toString('base64').replace(/.{76}/g, '$&\r\n');
  const message = [
    `From: ${from}`,
    `To: ${to}`,
    `Subject: ${encodeHeaderText(subject)}`,
    'MIME-Version: 1.0',
    'Content-Type: text/plain; charset="UTF-8"',
    'Content-Transfer-Encoding: base64',
    '',
    encodedBody,
  ].join('\r\n');
  return Buffer.from(message).toString('base64url');
}

/**
 * @returns {Promise<{id: string}>}
 */
export async function sendEmail(agentId, to, subject, body) {
  const { gmail, email } = clientFor(agentId);
  const raw = buildRawMessage(email, to, subject, body);
  const result = await gmail.users.messages.send({ userId: 'me', requestBody: { raw } });
  return { id: result.data.id };
}

/**
 * Unread messages in the agent's inbox, from the INBOX label counters.
 * Needs a refresh token granted the gmail.labels (or gmail.readonly) scope
 * in addition to gmail.send; without it Google answers 403.
 * @returns {Promise<number>}
 */
export async function unreadInboxCount(agentId) {
  const { gmail } = clientFor(agentId);
  const label = await gmail.users.labels.get({ userId: 'me', id: 'INBOX' });
  return label.data.messagesUnread ?? 0;
}

export function isConfigured(agentId) {
  return gmailCredentials(agentId) !== null;
}
