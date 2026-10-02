import { beforeEach } from 'vitest';

// Import lazily: a module a setup file loads up front escapes the test file's vi.mock.
beforeEach(async () => (await import('./lib/session')).useSession());
