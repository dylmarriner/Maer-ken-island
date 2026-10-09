import 'dotenv/config';
import { createApp } from './app.js';

const port = Number(process.env.PORT ?? 8002);
// Loopback by default: the only client is the local simulation engine. Set
// HOST (and COMPUTER_SERVICE_TOKEN) to expose it deliberately.
const host = process.env.HOST ?? '127.0.0.1';
createApp().listen(port, host, () => {
  console.log(`computer-service listening on ${host}:${port}`);
});
