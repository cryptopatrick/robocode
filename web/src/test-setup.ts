import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/react';

// Unmount rendered components between tests (automatic only with vitest globals).
afterEach(cleanup);
