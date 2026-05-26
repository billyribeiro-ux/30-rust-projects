import { defineConfig, devices } from '@playwright/test';

const PORT = 4185;

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false, // auth tests share the backend DB; run sequentially to avoid cross-test bleed
  workers: 1,
  reporter: 'list',
  use: { baseURL: `http://localhost:${PORT}`, trace: 'on-first-retry' },
  webServer: {
    command: `pnpm build && pnpm preview --port ${PORT}`,
    port: PORT,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000
  },
  projects: [
    {
      name: 'mobile-portrait-390',
      use: {
        ...devices['Desktop Chrome'],
        viewport: { width: 390, height: 844 },
        isMobile: false,
        hasTouch: true
      }
    },
    {
      name: 'tablet-768',
      use: { ...devices['Desktop Chrome'], viewport: { width: 768, height: 1024 } }
    },
    {
      name: 'laptop-1024',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1024, height: 768 } }
    },
    {
      name: 'desktop-1440',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 900 } }
    }
  ]
});
