// DuckDuckGo Instant Answer API — no key required. Per
// docs/superpowers/specs/2026-09-11-computer-access-integration-design.md
// "Web Search Provider Specification". Failures surface as errors so the
// caller records an unsuccessful search; nothing is substituted.

const ENDPOINT = 'https://api.duckduckgo.com/';
const MAX_RESULTS = 5;
const TIMEOUT_MS = 5000;

/** Split DuckDuckGo's "Title - description" topic text into its parts. */
function splitTopicText(text) {
  const separator = text.indexOf(' - ');
  return separator > 0
    ? { title: text.slice(0, separator), snippet: text.slice(separator + 3) }
    : { title: text, snippet: text };
}

/** Related topics, flattening DuckDuckGo's category groups ({Name, Topics}). */
function* relatedTopics(topics) {
  for (const topic of topics ?? []) {
    if (Array.isArray(topic.Topics)) {
      yield* relatedTopics(topic.Topics);
    } else if (topic.Text && topic.FirstURL) {
      yield topic;
    }
  }
}

/**
 * Convert an Instant Answer response into ranked results.
 * satisfaction is a result-count heuristic (0 results = 0.0, 1 = 0.5,
 * 2+ = 1.0): the API exposes no relevance score, only presence and count.
 */
export function parseInstantAnswer(query, data) {
  const results = [];
  if (data.AbstractText) {
    results.push({ title: data.Heading || query, snippet: data.AbstractText, url: data.AbstractURL || '' });
  }
  if (data.Answer && results.length < MAX_RESULTS) {
    results.push({ title: data.Heading || query, snippet: String(data.Answer), url: data.AbstractURL || '' });
  }
  for (const topic of relatedTopics(data.RelatedTopics)) {
    if (results.length >= MAX_RESULTS) break;
    results.push({ ...splitTopicText(topic.Text), url: topic.FirstURL });
  }
  const satisfaction = results.length === 0 ? 0.0 : results.length === 1 ? 0.5 : 1.0;
  return { results, satisfaction };
}

/**
 * @param {string} query
 * @returns {Promise<{results: Array<{title: string, snippet: string, url: string}>, satisfaction: number}>}
 */
export async function webSearch(query) {
  const url = new URL(ENDPOINT);
  url.searchParams.set('q', query);
  url.searchParams.set('format', 'json');
  url.searchParams.set('no_html', '1');
  url.searchParams.set('skip_disambig', '1');

  const response = await fetch(url, { signal: AbortSignal.timeout(TIMEOUT_MS) });
  if (!response.ok) {
    const error = new Error(`DuckDuckGo returned HTTP ${response.status}`);
    error.status = response.status === 429 ? 429 : 503;
    throw error;
  }
  return parseInstantAnswer(query, await response.json());
}
