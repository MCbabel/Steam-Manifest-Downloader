const path = require('path');
const os = require('os');
const fs = require('fs');

// Detect if running inside Electron
const isElectron = !!(process.versions && process.versions.electron);

// In packaged Electron app, resources are in process.resourcesPath
// In development (electron . or node server.js), use __dirname
// Detect packaged mode: resourcesPath won't contain 'node_modules' when packaged
const isPackaged = isElectron && process.resourcesPath && !process.resourcesPath.includes('node_modules');
const basePath = isPackaged
  ? process.resourcesPath
  : __dirname;

// For writable data (uploads, history), use Electron's userData path when packaged,
// otherwise use the project directory. In packaged apps, __dirname is inside the
// read-only asar archive, so we must use a writable location.
function getUserDataPath() {
  if (isElectron) {
    try {
      const { app } = require('electron');
      return app.getPath('userData');
    } catch (e) {
      // Fallback if app not ready yet
    }
  }
  return __dirname;
}

const userDataPath = getUserDataPath();
const settingsFilePath = path.join(userDataPath, 'data', 'settings.json');

/**
 * Load settings from settings.json.
 * @returns {Object} Settings object
 */
function loadSettings() {
  try {
    if (fs.existsSync(settingsFilePath)) {
      const content = fs.readFileSync(settingsFilePath, 'utf-8');
      return JSON.parse(content);
    }
  } catch (e) {
    console.error('[Config] Failed to load settings:', e.message);
  }
  return {};
}

/**
 * Save settings to settings.json.
 * @param {Object} settings
 */
function saveSettings(settings) {
  try {
    const dir = path.dirname(settingsFilePath);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(settingsFilePath, JSON.stringify(settings, null, 2), 'utf-8');
  } catch (e) {
    console.error('[Config] Failed to save settings:', e.message);
  }
}

// Load persisted settings
const savedSettings = loadSettings();

module.exports = {
  port: 3000,
  isElectron,
  isPackaged,
  userDataPath,
  uploadsDir: path.join(userDataPath, 'uploads'),
  dataDir: path.join(userDataPath, 'data'),
  downloadsDir: path.join(os.homedir(), 'Documents', 'SteamDownloads'),
  depotDownloaderPath: path.join(basePath, 'DepotDownloaderMod', 'DepotDownloaderMod.exe'),
  githubRepo: 'SteamAutoCracks/ManifestHub',
  githubApiBase: 'https://api.github.com/repos/SteamAutoCracks/ManifestHub',
  githubRawBase: 'https://raw.githubusercontent.com/SteamAutoCracks/ManifestHub',
  githubToken: savedSettings.githubToken || '',
  ddExtraArgs: savedSettings.ddExtraArgs || ['-max-downloads', '256', '-verify-all'],
  settingsFilePath,
  loadSettings,
  saveSettings
};
