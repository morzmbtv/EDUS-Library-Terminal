// Render the vector deliverable with the existing Codex workspace Sharp runtime.
// Set CODEX_NODE_MODULES if your local bundled runtime is installed elsewhere.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { writeFileSync } from 'node:fs';

const dependencyRoot = process.env.CODEX_NODE_MODULES
  ?? 'C:/Users/User/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules';
const require = createRequire(resolve(dependencyRoot, 'package.json'));
const sharp = require('sharp');
const svg = resolve('public/assets/edus-card.svg');
const png = resolve('public/assets/edus-card.png');
await sharp(svg, { density: 216 }).png().toFile(png);
const metadata = await sharp(png).metadata();
if (metadata.width !== 1800 || metadata.height !== 2880 || !metadata.hasAlpha) {
  throw new Error('Card export must be 1800×2880 with transparent corners.');
}
const pixel = await sharp(png).extract({ left: 0, top: 0, width: 1, height: 1 }).raw().toBuffer();
if (pixel[3] !== 0) throw new Error('Exterior must be transparent.');
writeFileSync(resolve('public/assets/edus-card.validation.json'), JSON.stringify({
  width: metadata.width, height: metadata.height, hasAlpha: metadata.hasAlpha,
  exteriorCornerAlpha: pixel[3], embeddedRasterInSVG: false,
  ornament: 'Reconstructed from photograph; not original print artwork',
}, null, 2) + '\n');
console.log(`Validated ${png}: ${metadata.width}×${metadata.height}, transparent rounded corners.`);
