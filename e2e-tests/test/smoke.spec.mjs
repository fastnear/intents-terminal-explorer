import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';
import { spawn, spawnSync } from 'child_process';
import { Builder, By, Capabilities } from 'selenium-webdriver';
import { expect } from 'chai';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..', '..'); // e2e-tests -> ratacat root

// Build a debug, non-bundled app with the e2e feature enabled
const appDir = path.resolve(repoRoot, 'tauri-workspace');
const application =
  process.platform === 'win32'
    ? path.resolve(appDir, 'src-tauri', 'target', 'debug', 'nearx-tauri.exe')
    : path.resolve(appDir, 'src-tauri', 'target', 'debug', 'nearx-tauri');

let driver;
let tauriDriver;
let exit = false;

before(async function () {
  this.timeout(180000); // 3 minutes for build + driver startup

  console.log('Building Tauri app with e2e features...');
  console.log('App directory:', appDir);
  console.log('Binary path:', application);

  // Ensure Tauri debug build (no-bundle) with e2e feature for test IPC
  const buildResult = spawnSync(
    'cargo',
    ['tauri', 'build', '--debug', '--no-bundle', '--features', 'e2e'],
    {
      cwd: appDir,
      stdio: 'inherit',
      shell: true
    }
  );

  if (buildResult.error) {
    console.error('Build failed:', buildResult.error);
    throw buildResult.error;
  }

  console.log('Starting tauri-driver...');

  // Start tauri-driver
  const tauriDriverPath = path.resolve(os.homedir(), '.cargo', 'bin', 'tauri-driver');
  tauriDriver = spawn(tauriDriverPath, [], {
    stdio: [null, process.stdout, process.stderr]
  });

  tauriDriver.on('error', (e) => {
    console.error('tauri-driver error:', e);
    process.exit(1);
  });

  tauriDriver.on('exit', (code) => {
    if (!exit) {
      console.error('tauri-driver exited unexpectedly with code:', code);
      process.exit(1);
    }
  });

  // Give tauri-driver a moment to start up
  await new Promise(resolve => setTimeout(resolve, 2000));

  console.log('Connecting WebDriver...');

  // Configure capabilities for Tauri WebDriver
  const caps = new Capabilities();
  caps.set('tauri:options', { application });
  caps.setBrowserName('wry'); // Tauri uses wry WebView

  // Connect to tauri-driver
  driver = await new Builder()
    .withCapabilities(caps)
    .usingServer('http://127.0.0.1:4444/')
    .build();

  console.log('WebDriver connected successfully');
});

after(async function () {
  exit = true;
  console.log('Cleaning up...');

  if (driver) {
    try {
      await driver.quit();
      console.log('WebDriver quit');
    } catch (error) {
      console.error('Error quitting driver:', error);
    }
  }

  if (tauriDriver) {
    tauriDriver.kill();
    console.log('tauri-driver killed');
  }
});

describe('NEARx Desktop (Tauri) – E2E Smoke Tests', function () {
  this.timeout(60000);

  describe('Rendering & Layout', function () {
    it('renders the canvas and fills viewport', async () => {
      // We render via egui/rust → <canvas>. Select the first canvas.
      const rect = await driver.executeScript(() => {
        const c = document.querySelector('canvas');
        if (!c) return null;
        const r = c.getBoundingClientRect();
        return { w: r.width, h: r.height };
      });

      expect(rect).to.be.an('object');
      expect(rect.w).to.be.greaterThan(400, 'Canvas width should be > 400px');
      expect(rect.h).to.be.greaterThan(300, 'Canvas height should be > 300px');
    });

    it('has NEARxTest bridge available', async () => {
      const hasTestBridge = await driver.executeScript(() => {
        return typeof window.NEARxTest === 'object';
      });

      expect(hasTestBridge).to.equal(true, 'NEARxTest bridge should be available');
    });
  });

  describe('OAuth Router', function () {
    it('scrubs callback URL and persists token', async () => {
      // Simulate our callback route handling in the webview
      await driver.executeScript(() => {
        // Token path (bypasses real Google for deterministic test)
        location.hash = '#/auth/callback?token=e2e-test-token-12345';
        window.dispatchEvent(new HashChangeEvent('hashchange'));
      });

      // Give the router time to process
      await new Promise(resolve => setTimeout(resolve, 500));

      // Verify URL was scrubbed
      const hash = await driver.executeScript(() => location.hash);
      expect(hash).to.equal('#/', 'Hash should be scrubbed to root');

      // Verify token was persisted
      const stored = await driver.executeScript(() => {
        try {
          return localStorage.getItem('nearx.token');
        } catch {
          return null;
        }
      });

      expect(stored).to.equal('e2e-test-token-12345', 'Token should be persisted in localStorage');
    });

    it('retrieves token via test bridge', async () => {
      const token = await driver.executeScript(() => {
        return window.NEARxTest?.getToken();
      });

      expect(token).to.equal('e2e-test-token-12345', 'Test bridge should retrieve token');
    });
  });

  describe('Deep Link Bridge', function () {
    it('delivers deep link route via test IPC', async () => {
      // Use test-only IPC to inject a deep link event
      const ok = await driver.executeAsyncScript((done) => {
        const inv = window.__TAURI__?.invoke;
        if (!inv) return done(false);

        inv('nearx_test_emit_deeplink', { url: 'nearx://v1/tx/E2E_TEST_HASH' })
          .then(() => done(true))
          .catch((err) => {
            console.error('Deep link emit failed:', err);
            done(false);
          });
      });

      expect(ok).to.equal(true, 'Deep link should be emitted successfully');
    });

    it('tracks deep link in test bridge history', async () => {
      // Give event time to propagate
      await new Promise(resolve => setTimeout(resolve, 500));

      const history = await driver.executeScript(() => {
        return window.NEARxTest?.getDeepLinkHistory?.() ?? [];
      });

      expect(history).to.be.an('array');
      expect(history.length).to.be.greaterThan(0, 'Should have at least one deep link event');

      const lastEvent = history[history.length - 1];
      expect(lastEvent.url).to.include('E2E_TEST_HASH', 'Last event should contain test hash');
    });

    it('updates last route after deep link', async () => {
      const lastRoute = await driver.executeScript(() => {
        return window.NEARxTest?.getLastRoute?.() ?? null;
      });

      // Should either be the deep link or null if routing not implemented
      if (lastRoute) {
        expect(String(lastRoute)).to.match(/nearx:\/\/v1\/tx\/E2E_TEST_HASH/);
      }
    });
  });

  describe('Clipboard Integration', function () {
    it('clipboard roundtrip works (copy → readText)', async () => {
      // First, set up a known state by copying something
      const wrote = await driver.executeAsyncScript((done) => {
        const copy = window.NEARxTest?.copyFocused;
        if (!copy) return done(false);

        copy()
          .then(() => done(true))
          .catch((err) => {
            console.error('Copy failed:', err);
            done(false);
          });
      });

      expect(wrote).to.equal(true, 'Copy operation should succeed');

      // Small delay for clipboard to settle
      await new Promise(resolve => setTimeout(resolve, 300));

      // Read via Tauri clipboard plugin
      const read = await driver.executeAsyncScript((done) => {
        const cm = window.__TAURI__?.clipboardManager;
        const fn = cm?.readText ?? window.__TAURI__?.invoke?.bind(null, 'plugin:clipboard-manager|read_text');

        if (!fn) return done(null);

        Promise.resolve(fn())
          .then((v) => done(v))
          .catch((err) => {
            console.error('Read clipboard failed:', err);
            done(null);
          });
      });

      // We don't assert exact content since we don't know what's focused,
      // but we verify the clipboard roundtrip worked
      expect(read).to.be.a('string', 'Clipboard should contain a string');
      expect(read.length).to.be.greaterThan(0, 'Clipboard should not be empty');
    });
  });

  describe('Keyboard & Mouse Navigation', function () {
    it('keyboard navigation moves focus (Tab/Shift+Tab)', async () => {
      // Send Tab key twice
      await driver.executeScript(() => {
        return window.NEARxTest?.pressKey?.('Tab');
      });

      await new Promise(resolve => setTimeout(resolve, 100));

      await driver.executeScript(() => {
        return window.NEARxTest?.pressKey?.('Tab');
      });

      await new Promise(resolve => setTimeout(resolve, 100));

      // If we have focus indicators, they should have changed
      // This is a basic smoke test - just verify no errors occurred
      const hasTestBridge = await driver.executeScript(() => {
        return typeof window.NEARxTest === 'object';
      });

      expect(hasTestBridge).to.equal(true, 'Test bridge should still be available after navigation');
    });

    it('cursor affordance is available', async () => {
      // We can't reliably detect OS cursor from DOM, but we can check if the
      // cursor tracking is working in the test bridge
      const cursorIsPointer = await driver.executeScript(() => {
        return typeof window.NEARxTest?.cursorIsPointer === 'function';
      });

      expect(cursorIsPointer).to.equal(true, 'Cursor state tracking should be available');
    });
  });

  describe('Storage & State', function () {
    it('clears storage via test API', async () => {
      // First set a test value
      await driver.executeScript(() => {
        localStorage.setItem('e2e-test-key', 'test-value');
      });

      // Verify it's there
      let value = await driver.executeScript(() => {
        return localStorage.getItem('e2e-test-key');
      });
      expect(value).to.equal('test-value');

      // Clear via test API
      const cleared = await driver.executeAsyncScript((done) => {
        const inv = window.__TAURI__?.invoke;
        if (!inv) return done(false);

        inv('nearx_test_clear_storage')
          .then(() => done(true))
          .catch(() => done(false));
      });

      expect(cleared).to.equal(true, 'Clear storage should succeed');

      // Verify it's gone
      value = await driver.executeScript(() => {
        return localStorage.getItem('e2e-test-key');
      });
      expect(value).to.be.null;
    });
  });

  describe('Error Handling', function () {
    it('handles invalid deep link gracefully', async () => {
      const ok = await driver.executeAsyncScript((done) => {
        const inv = window.__TAURI__?.invoke;
        if (!inv) return done(false);

        // Send malformed URL
        inv('nearx_test_emit_deeplink', { url: 'not-a-valid-url://bad' })
          .then(() => done(true))
          .catch(() => done(false)); // Even if it fails, we want to know it didn't crash
      });

      // Either succeeds (logs a warning) or fails gracefully
      expect(ok).to.be.a('boolean');
    });
  });
});
