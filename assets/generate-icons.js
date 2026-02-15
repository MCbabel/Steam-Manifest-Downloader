/**
 * Icon Generator - Creates placeholder PNG icons for the Electron app.
 * Run: node assets/generate-icons.js
 * 
 * This creates simple download-themed icons using Canvas.
 * For production, replace these with proper designed icons.
 */

const { createCanvas } = require('canvas');
const fs = require('fs');
const path = require('path');

function createIcon(size, filename) {
  const canvas = createCanvas(size, size);
  const ctx = canvas.getContext('2d');

  // Background - rounded square with gradient
  const gradient = ctx.createLinearGradient(0, 0, size, size);
  gradient.addColorStop(0, '#4f6ef7');
  gradient.addColorStop(1, '#3d5ce0');

  const radius = size * 0.18;
  ctx.beginPath();
  ctx.moveTo(radius, 0);
  ctx.lineTo(size - radius, 0);
  ctx.quadraticCurveTo(size, 0, size, radius);
  ctx.lineTo(size, size - radius);
  ctx.quadraticCurveTo(size, size, size - radius, size);
  ctx.lineTo(radius, size);
  ctx.quadraticCurveTo(0, size, 0, size - radius);
  ctx.lineTo(0, radius);
  ctx.quadraticCurveTo(0, 0, radius, 0);
  ctx.fillStyle = gradient;
  ctx.fill();

  // Download arrow
  ctx.strokeStyle = 'white';
  ctx.fillStyle = 'white';
  ctx.lineWidth = size * 0.08;
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';

  const cx = size / 2;
  const arrowTop = size * 0.2;
  const arrowBottom = size * 0.6;
  const arrowWidth = size * 0.2;

  // Vertical line
  ctx.beginPath();
  ctx.moveTo(cx, arrowTop);
  ctx.lineTo(cx, arrowBottom);
  ctx.stroke();

  // Arrow head
  ctx.beginPath();
  ctx.moveTo(cx - arrowWidth, arrowBottom - arrowWidth * 0.8);
  ctx.lineTo(cx, arrowBottom);
  ctx.lineTo(cx + arrowWidth, arrowBottom - arrowWidth * 0.8);
  ctx.stroke();

  // Base line
  const baseY = size * 0.75;
  ctx.beginPath();
  ctx.moveTo(size * 0.25, baseY);
  ctx.lineTo(size * 0.75, baseY);
  ctx.stroke();

  const buffer = canvas.toBuffer('image/png');
  fs.writeFileSync(path.join(__dirname, filename), buffer);
  console.log(`Created ${filename} (${size}x${size})`);
}

// Only run if canvas is available
try {
  require('canvas');
  createIcon(256, 'icon.png');
  createIcon(16, 'icon-small.png');
  createIcon(16, 'tray-icon.png');
  console.log('Icons generated! Note: You still need to create icon.ico manually.');
} catch (e) {
  console.log('canvas module not available. Using SVG fallback icons.');
}
