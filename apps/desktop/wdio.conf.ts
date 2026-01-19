/**
 * WebDriverIO Configuration for Tauri App E2E Tests
 *
 * NOTE: tauri-driver is NOT SUPPORTED on macOS!
 * This config is kept for potential Linux/Windows use.
 *
 * On macOS, use the shell-based test instead:
 *   pnpm test:e2e:app
 *
 * Which runs: tests/e2e-app/run-app-tests.sh
 */
import type { Options } from '@wdio/types';
import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Path to the built Tauri app
const APP_PATH = path.join(
  __dirname,
  'src-tauri/target/release/bundle/macos/Portfolio Now.app/Contents/MacOS/Portfolio Now'
);

let tauriDriver: ChildProcess | null = null;

export const config: Options.Testrunner = {
  runner: 'local',
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tsconfig.json',
      transpileOnly: true,
    },
  },

  specs: ['./tests/e2e-app/**/*.spec.ts'],
  exclude: [],

  maxInstances: 1,

  capabilities: [
    {
      // Use tauri-driver for WebDriver
      browserName: 'wry',
      'tauri:options': {
        application: APP_PATH,
      },
    },
  ],

  logLevel: 'info',
  bail: 0,
  baseUrl: '',
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,

  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },

  // Start tauri-driver before tests
  onPrepare: async function () {
    console.log('Starting tauri-driver...');
    tauriDriver = spawn('tauri-driver', [], {
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    tauriDriver.stdout?.on('data', (data) => {
      console.log(`[tauri-driver] ${data}`);
    });

    tauriDriver.stderr?.on('data', (data) => {
      console.error(`[tauri-driver] ${data}`);
    });

    // Wait for tauri-driver to start
    await new Promise<void>((resolve) => {
      const checkReady = () => {
        // tauri-driver starts on port 4444 by default
        setTimeout(resolve, 2000);
      };
      checkReady();
    });

    console.log('tauri-driver started');
  },

  // Stop tauri-driver after tests
  onComplete: async function () {
    if (tauriDriver) {
      console.log('Stopping tauri-driver...');
      tauriDriver.kill();
      tauriDriver = null;
    }
  },

  // Called before each test file
  before: async function () {
    // Add custom commands if needed
  },
};
