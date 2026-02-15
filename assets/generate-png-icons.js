/**
 * Generate PNG icons from SVG using sharp
 * Run: node assets/generate-png-icons.js
 */
const sharp = require('sharp');
const path = require('path');

const svgPath = path.join(__dirname, 'icon.svg');

async function generate() {
  await sharp(svgPath)
    .resize(256, 256)
    .png()
    .toFile(path.join(__dirname, 'icon.png'));
  console.log('Generated icon.png (256x256)');

  await sharp(svgPath)
    .resize(32, 32)
    .png()
    .toFile(path.join(__dirname, 'icon-small.png'));
  console.log('Generated icon-small.png (32x32)');

  await sharp(svgPath)
    .resize(64, 64)
    .png()
    .toFile(path.join(__dirname, 'tray-icon.png'));
  console.log('Generated tray-icon.png (64x64)');
}

generate().catch(err => {
  console.error('Icon generation failed:', err);
  process.exit(1);
});
