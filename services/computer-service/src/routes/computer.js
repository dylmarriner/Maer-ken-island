import { Router } from 'express';
import { AGENT_IDS, contacts } from '../config.js';
import * as defaultGmail from '../GmailService.js';
import { MessageFormatError } from '../GmailService.js';
import * as defaultSearch from '../WebSearchService.js';

/**
 * HTTP status for a Google API or search failure. 401/403/429 keep their
 * meaning (the Rust bridge maps them to auth / rate-limit errors); a timeout
 * becomes 504 and anything else 503.
 */
export function statusFor(err) {
  // Client libraries put the HTTP status in different places, and `code` is
  // often a string such as 'ECONNRESET' or 'ERR_BAD_REQUEST'; take the first
  // candidate that is actually a number.
  const code = [err.status, err.response?.status, err.code]
    .map(Number)
    .find((candidate) => Number.isInteger(candidate) && candidate > 0);
  if ([401, 403, 429].includes(code)) return code;
  if (
    err.name === 'TimeoutError' ||
    err.name === 'AbortError' ||
    ['ETIMEDOUT', 'ESOCKETTIMEDOUT', 'ECONNABORTED'].includes(err.code)
  ) {
    return 504;
  }
  return 503;
}

/**
 * Router for /api/computer. Services are injectable so the routes can be
 * exercised without network access.
 */
export function createComputerRouter({ gmail = defaultGmail, search = defaultSearch, env = process.env } = {}) {
  const router = Router();

  router.param('agentId', (req, res, next, agentId) => {
    if (!AGENT_IDS.includes(agentId)) {
      return res.status(404).json({ error: `unknown agent '${agentId}'` });
    }
    next();
  });

  router.post('/:agentId/search', async (req, res) => {
    const { agentId } = req.params;
    const { query } = req.body ?? {};
    if (typeof query !== 'string' || query.trim().length === 0) {
      return res.status(400).json({ error: 'missing "query"' });
    }
    try {
      const { results, satisfaction } = await search.webSearch(query);
      res.json({ query, results, satisfaction });
    } catch (err) {
      console.error(`[${agentId}] web_search error:`, err);
      res.status(statusFor(err)).json({ error: err.message });
    }
  });

  router.post('/:agentId/email', async (req, res) => {
    const { agentId } = req.params;
    const { to, subject, body } = req.body ?? {};
    if (typeof to !== 'string' || typeof subject !== 'string' || typeof body !== 'string') {
      return res.status(400).json({ error: 'missing "to"/"subject"/"body"' });
    }
    // Only configured contacts can be reached; an agent can never address an
    // arbitrary mailbox.
    const address = contacts(env)[to.toLowerCase()];
    if (!address) {
      return res.status(400).json({ error: `unknown contact '${to}'` });
    }
    if (!gmail.isConfigured(agentId)) {
      return res.status(401).json({ error: `no Gmail credentials configured for '${agentId}'` });
    }
    try {
      const result = await gmail.sendEmail(agentId, address, subject, body);
      res.json({ message_id: result.id, to, subject });
    } catch (err) {
      if (err instanceof MessageFormatError) {
        return res.status(400).json({ error: err.message });
      }
      console.error(`[${agentId}] send_email error:`, err);
      res.status(statusFor(err)).json({ error: err.message });
    }
  });

  // powered_on: the agent has working Gmail credentials configured.
  // unread_count: the live INBOX unread counter from Gmail.
  router.get('/:agentId/state', async (req, res) => {
    const { agentId } = req.params;
    if (!gmail.isConfigured(agentId)) {
      return res.json({ agent_id: agentId, powered_on: false, unread_count: 0 });
    }
    try {
      const unread = await gmail.unreadInboxCount(agentId);
      res.json({ agent_id: agentId, powered_on: true, unread_count: unread });
    } catch (err) {
      console.error(`[${agentId}] state error:`, err);
      res.status(statusFor(err)).json({ error: err.message });
    }
  });

  return router;
}
