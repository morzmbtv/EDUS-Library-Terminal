import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { dirname, resolve } from 'node:path'

const root = resolve(import.meta.dirname, '..')
const dist = resolve(root, 'dist')
const release = resolve(root, 'release', 'edus-library-terminal')
const app = resolve(release, 'app')
if (!existsSync(dist)) throw new Error('dist/ is missing. Run npm run build before packaging the release.')
if (existsSync(app)) await rm(app, { recursive: true, force: true })
await mkdir(release, { recursive: true })
await cp(dist, app, { recursive: true })
for (const file of ['README.md', 'BLOCKERS.md', 'PROJECT_STATE.md']) {
  await cp(resolve(root, file), resolve(release, file))
}
const docs = ['DEPLOYMENT.md', 'INSTALL_RU.md', 'ACCEPTANCE_REPORT.md', 'CANTEEN_INTEGRATION_MAP.md', 'LIBRARY_API_CONTRACT.md']
await mkdir(resolve(release, 'docs'), { recursive: true })
for (const file of docs) await cp(resolve(root, 'docs', file), resolve(release, 'docs', file))
const version = (await readFile(resolve(root, 'package.json'), 'utf8')).match(/"version":\s*"([^"]+)"/)?.[1]
if (!version) throw new Error('Could not determine package version.')
await writeFile(resolve(release, 'VERSION'), `${version}\n`, 'utf8')
console.log(`Release prepared: ${release}`)
