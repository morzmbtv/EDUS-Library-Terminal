import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const hash = (b) => crypto.createHash('sha256').update(b).digest('hex');
const root = path.resolve('dist');
const walk = (d) =>
  fs
    .readdirSync(d, { withFileTypes: true })
    .flatMap((e) => (e.isDirectory() ? walk(path.join(d, e.name)) : [path.join(d, e.name)]));
const files = walk(root)
  .filter((p) => path.basename(p) !== 'frontend-manifest.json')
  .map((p) => ({ path: path.relative(root, p).replaceAll('\\', '/'), sha256: hash(fs.readFileSync(p)) }))
  .sort((a, b) => a.path.localeCompare(b.path));
const frontend_version = JSON.parse(fs.readFileSync('package.json')).version;
const manifest = {
  frontend_version,
  required_local_api_version: '1',
  build_hash: hash(JSON.stringify(files)),
  build_time: new Date().toISOString(),
  files,
};
fs.writeFileSync(path.join(root, 'frontend-manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
console.log('Frontend manifest ' + manifest.build_hash + ' (' + files.length + ' files)');
