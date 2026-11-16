import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.mjs',
  testIgnore: '**/._*',
  timeout: 120_000,
  workers: 1,
  reporter: 'list',
  use: { browserName: 'chromium', headless: true },
});
