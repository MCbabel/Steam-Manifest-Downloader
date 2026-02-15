/**
 * Steam Manifest Downloader - Electron Preload Script
 * Exposes safe IPC methods to the renderer process.
 */

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  // Window controls
  minimize: () => ipcRenderer.send('window-minimize'),
  maximize: () => ipcRenderer.send('window-maximize'),
  close: () => ipcRenderer.send('window-close'),
  forceClose: () => ipcRenderer.send('force-close'),

  // Download state tracking
  setDownloading: (val) => ipcRenderer.send('set-downloading', val),

  // Close confirmation (main -> renderer)
  onConfirmClose: (callback) => ipcRenderer.on('confirm-close', callback),

  // Maximize state changes (main -> renderer)
  onMaximizeChange: (callback) => ipcRenderer.on('maximize-change', callback),

  // Query current maximize state
  isMaximized: () => ipcRenderer.invoke('is-maximized')
});
