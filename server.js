/**
 * Steam Manifest Downloader - Server Entry Point
 */

const express = require('express');
const path = require('path');
const http = require('http');
const config = require('./config');
const websocket = require('./src/websocket');
const uploadRoute = require('./src/routes/upload');
const downloadRoute = require('./src/routes/download');
const steamRoute = require('./src/routes/steam');
const historyRoute = require('./src/routes/history');

const app = express();
const server = http.createServer(app);

// Middleware
app.use(express.json());
app.use(express.static(path.join(__dirname, 'public')));

// Serve assets directory for title bar icon etc.
app.use('/assets', express.static(path.join(__dirname, 'assets')));

// API Routes
app.use('/api/upload', uploadRoute);
app.use('/api/download', downloadRoute);
app.use('/api/steam', steamRoute);
app.use('/api/history', historyRoute);

// Search routes (mounted from upload module for code organization, but at /api/search)
const searchRoute = require('./src/routes/search');
app.use('/api/search', searchRoute);

// Initialize WebSocket
websocket.init(server);

// Error handler for multer
app.use((err, req, res, next) => {
  if (err.message === 'Only .lua files are accepted') {
    return res.status(400).json({ success: false, error: err.message });
  }
  if (err.code === 'LIMIT_FILE_SIZE') {
    return res.status(400).json({ success: false, error: 'File too large. Max size: 1MB' });
  }
  console.error('Server error:', err);
  res.status(500).json({ success: false, error: 'Internal server error' });
});

/**
 * Start the server and return a promise that resolves with the port.
 * @param {number} [port] - Port to listen on. Defaults to config.port (3000). Pass 0 for a random free port.
 * @returns {Promise<{server: http.Server, port: number}>}
 */
function startServer(port) {
  const listenPort = port !== undefined ? port : config.port;
  return new Promise((resolve, reject) => {
    server.listen(listenPort, () => {
      const actualPort = server.address().port;
      console.log(`\n  Steam Manifest Downloader`);
      console.log(`  ========================`);
      console.log(`  Server running at http://localhost:${actualPort}`);
      console.log(`  Downloads folder: ${config.downloadsDir}`);
      console.log(`  DepotDownloader: ${config.depotDownloaderPath}\n`);
      resolve({ server, port: actualPort });
    });
    server.on('error', reject);
  });
}

// If run directly (not required by Electron), start the server immediately
if (require.main === module) {
  startServer().catch((err) => {
    console.error('Failed to start server:', err);
    process.exit(1);
  });
}

module.exports = { startServer, app, server };
