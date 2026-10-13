import { timingSafeEqual } from 'node:crypto';
import express from 'express';
import { createComputerRouter } from './routes/computer.js';
import { router as healthRouter } from './routes/health.js';

/** Constant-time comparison of two strings. */
function sameSecret(given, expected) {
  const a = Buffer.from(given);
  const b = Buffer.from(expected);
  return a.length === b.length && timingSafeEqual(a, b);
}

/**
 * Require `Authorization: Bearer <token>` when a token is configured. The
 * computer routes can read an agent's inbox state and send mail, so any
 * deployment reachable beyond localhost must set `COMPUTER_SERVICE_TOKEN`.
 */
function requireToken(token) {
  return (req, res, next) => {
    if (!token) return next();
    const header = req.get('authorization') ?? '';
    const given = header.startsWith('Bearer ') ? header.slice('Bearer '.length) : '';
    if (!sameSecret(given, token)) {
      return res.status(401).json({ error: 'missing or invalid bearer token' });
    }
    next();
  };
}

export function createApp(services = {}, { token = process.env.COMPUTER_SERVICE_TOKEN } = {}) {
  const app = express();
  app.use(express.json({ limit: '64kb' }));
  app.use('/api/computer', requireToken(token), createComputerRouter(services));
  app.use('/api/health', healthRouter);
  return app;
}
