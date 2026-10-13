// Runtime configuration, read only from process.env (see .env.example).
// Nothing personal — addresses, OAuth clients, tokens — is kept in source.

/** Agents that may use the service, keyed by the id the Rust bridge sends. */
export const AGENT_IDS = ['Gem-D', 'Gem-K'];

function envPrefix(agentId) {
  return agentId.replace(/[^A-Za-z0-9]/g, '_').toUpperCase(); // 'Gem-D' -> 'GEM_D'
}

/**
 * Gmail OAuth2 credentials for an agent, or null when any part is missing.
 * @param {string} agentId
 * @param {NodeJS.ProcessEnv} env
 */
export function gmailCredentials(agentId, env = process.env) {
  const prefix = envPrefix(agentId);
  const creds = {
    clientId: env[`${prefix}_CLIENT_ID`],
    clientSecret: env[`${prefix}_CLIENT_SECRET`],
    refreshToken: env[`${prefix}_REFRESH_TOKEN`],
    email: env[`${prefix}_EMAIL`],
  };
  return creds.clientId && creds.clientSecret && creds.refreshToken && creds.email ? creds : null;
}

/**
 * Contact directory. The Rust bridge only ever sends short identifiers from
 * its KNOWN_CONTACTS list (it never sees a real address); each identifier is
 * mapped to an address by a CONTACT_<ID> variable, e.g. CONTACT_DYLAN.
 * @param {NodeJS.ProcessEnv} env
 * @returns {Record<string, string>}
 */
export function contacts(env = process.env) {
  const directory = {};
  for (const [key, value] of Object.entries(env)) {
    const match = /^CONTACT_([A-Z0-9_]+)$/.exec(key);
    if (match && value) {
      directory[match[1].toLowerCase()] = value.trim();
    }
  }
  return directory;
}
