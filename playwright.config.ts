import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  timeout: 60_000,
  testDir: 'tests/e2e',
  retries: 0,
  use: {
    baseURL: process.env.NEARX_BASE_URL ?? 'http://127.0.0.1:8083',
    permissions: ['clipboard-read', 'clipboard-write'],
    trace: 'on-first-retry',
    video: 'off',
    screenshot: 'only-on-failure',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
});
