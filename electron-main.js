/**
 * Steam Manifest Downloader - Electron Main Process
 * This replaces server.js as the entry point when running as a desktop app.
 */

const { app, BrowserWindow, ipcMain, nativeImage, dialog, shell } = require('electron');
const { execSync } = require('child_process');
const path = require('path');

// Import the Express server starter
const { startServer } = require('./server');

// Set app name early (affects Task Manager display)
app.setName('Steam Manifest Downloader');

// ============ .NET Runtime Check ============

function checkDotNetRuntime() {
  try {
    const output = execSync('dotnet --list-runtimes', { encoding: 'utf8', timeout: 5000 });
    if (!output.includes('Microsoft.NETCore.App 9.')) {
      return false;
    }
    return true;
  } catch (e) {
    return false;
  }
}

let mainWindow;
let isDownloading = false;
let forceQuit = false;
let serverPort = 3000;

// ============ Icon Helpers ============

function getIconPath(name) {
  // In packaged app, __dirname points to app.asar, assets are alongside
  return path.join(__dirname, 'assets', name);
}

function createAppIcon() {
  const pngPath = getIconPath('icon.png');
  const svgPath = getIconPath('icon.svg');
  
  try {
    const fs = require('fs');
    if (fs.existsSync(pngPath)) {
      return nativeImage.createFromPath(pngPath);
    }
    if (fs.existsSync(svgPath)) {
      const svgBuffer = fs.readFileSync(svgPath);
      return nativeImage.createFromBuffer(svgBuffer, { width: 256, height: 256 });
    }
  } catch (e) {
    console.warn('Could not load app icon:', e.message);
  }
  
  return createFallbackIcon(256);
}

function createFallbackIcon(size) {
  // Create a simple blue square as fallback
  const canvas = `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}">
    <rect width="${size}" height="${size}" rx="${Math.floor(size * 0.18)}" fill="#4f6ef7"/>
  </svg>`;
  return nativeImage.createFromBuffer(Buffer.from(canvas), { width: size, height: size });
}

// ============ Window Creation ============

function createWindow() {
  const appIcon = createAppIcon();

  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 900,
    minHeight: 600,
    title: 'Steam Manifest Downloader',
    frame: false,
    backgroundColor: '#0f1117',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false
    },
    icon: appIcon,
    show: false
  });

  // Load the Express-served app
  mainWindow.loadURL(`http://localhost:${serverPort}`);

  // Show when ready
  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
  });

  // Track maximize state for rounded corners
  mainWindow.on('maximize', () => {
    mainWindow.webContents.send('maximize-change', true);
  });

  mainWindow.on('unmaximize', () => {
    mainWindow.webContents.send('maximize-change', false);
  });

  // Close protection — prevent close during downloads
  mainWindow.on('close', (e) => {
    if (forceQuit) return; // Allow force quit

    if (isDownloading) {
      e.preventDefault();
      mainWindow.webContents.send('confirm-close');
      return;
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// ============ IPC Handlers ============

function setupIPC() {
  ipcMain.on('window-minimize', () => {
    if (mainWindow) mainWindow.minimize();
  });

  ipcMain.on('window-maximize', () => {
    if (mainWindow) {
      if (mainWindow.isMaximized()) {
        mainWindow.unmaximize();
      } else {
        mainWindow.maximize();
      }
    }
  });

  ipcMain.on('window-close', () => {
    if (mainWindow) mainWindow.close();
  });

  ipcMain.on('set-downloading', (event, val) => {
    isDownloading = !!val;
  });

  ipcMain.on('force-close', () => {
    forceQuit = true;
    if (mainWindow) mainWindow.close();
  });

  ipcMain.handle('is-maximized', () => {
    return mainWindow ? mainWindow.isMaximized() : false;
  });
}

// ============ App Lifecycle ============

app.whenReady().then(async () => {
  // Check for .NET 9.0 Runtime before anything else
  if (!checkDotNetRuntime()) {
    const result = dialog.showMessageBoxSync({
      type: 'error',
      title: '.NET Runtime Required',
      message: '.NET 9.0 Runtime is required but not installed.',
      detail: 'DepotDownloaderMod requires .NET 9.0 Runtime to function. Click "Download .NET 9.0" to open the download page.',
      buttons: ['Download .NET 9.0', 'Continue Anyway']
    });
    if (result === 0) {
      shell.openExternal('https://aka.ms/dotnet-core-applaunch?framework=Microsoft.NETCore.App&framework_version=9.0.0&arch=x64&rid=win-x64&os=win10');
    }
  }

  try {
    // Start the Express server on a fixed port
    const result = await startServer(3000);
    serverPort = result.port;
    console.log(`Electron: Express server started on port ${serverPort}`);
  } catch (err) {
    // If port 3000 is taken, try a random port
    console.warn('Port 3000 in use, trying random port...');
    const result = await startServer(0);
    serverPort = result.port;
    console.log(`Electron: Express server started on port ${serverPort}`);
  }

  setupIPC();
  createWindow();
});

app.on('window-all-closed', () => {
  // On Windows, quit when all windows are closed
  app.quit();
});

app.on('activate', () => {
  // macOS: re-create window when dock icon is clicked
  if (!mainWindow) createWindow();
});

// Ensure clean exit
app.on('before-quit', () => {
  forceQuit = true;
});
