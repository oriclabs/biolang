// @ts-check
const { defineConfig } = require('@playwright/test');
const path = require('path');

const viewerPath = path.resolve(__dirname, '..', '..', 'viewer.html');
const viewerURL = 'file:///' + viewerPath.replace(/\\/g, '/');

module.exports = defineConfig({
  testDir: '.',
  testMatch: ['e2e/**/*.spec.js', 'visual/**/*.spec.js', 'screenshots/**/*.spec.js', 'video/**/*.spec.js'],
  timeout: 30000,
  expect: {
    timeout: 10000,
  },
  fullyParallel: false,
  retries: 0,
  reporter: [['list'], ['html', { open: 'never' }]],
  use: {
    baseURL: viewerURL,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    headless: true,
  },
  projects: [
    {
      name: 'chromium',
      use: {
        browserName: 'chromium',
      },
    },
    {
      name: 'video',
      testMatch: 'video/**/*.spec.js',
      use: {
        browserName: 'chromium',
        video: {
          mode: 'on',
          size: { width: 1440, height: 900 },
        },
      },
      timeout: 180000,
    },
  ],
});
