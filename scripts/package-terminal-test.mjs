import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

const root = resolve(import.meta.dirname, '..')
const dist = resolve(root, 'dist-terminal-test')
const source = resolve(root, 'terminal-test')
const release = resolve(root, 'release', 'edus-library-terminal-test')
const portable = resolve(source, 'portable')
if (!existsSync(dist)) throw new Error('dist-terminal-test/ is missing. Run npm run build:terminal-test before packaging.')
if (!existsSync(source) || !existsSync(portable)) throw new Error('terminal-test support files are missing.')
await rm(release, { recursive: true, force: true })
await mkdir(release, { recursive: true })
await cp(dist, resolve(release, 'app'), { recursive: true })
await cp(source, release, { recursive: true })
const version = (await readFile(resolve(root, 'package.json'), 'utf8')).match(/"version":\s*"([^"]+)"/)?.[1]
if (!version) throw new Error('Could not determine package version.')
await writeFile(resolve(release, 'VERSION'), `${version}-terminal-test\n`, 'utf8')
const archive = resolve(root, 'release', `edus-library-terminal-test-${version}.zip`)
const packed = spawnSync('tar', ['-a', '-c', '-f', archive, '-C', resolve(root, 'release'), 'edus-library-terminal-test'], { stdio: 'inherit' })
if (packed.status !== 0) throw new Error('Could not create terminal-test ZIP archive.')
const portableArchive = resolve(root, 'release', `edus-library-terminal-test-portable-${version}.zip`)
const python = process.env.PYTHON ?? 'python'
const portablePacked = spawnSync(python, [resolve(root, 'scripts', 'package-portable-zip.py'), '--app', dist, '--portable', portable, '--server', resolve(source, 'scripts', 'terminal_test_server.py'), '--hardware', resolve(source, 'HARDWARE_TEST.md'), '--version', version, '--output', portableArchive], { stdio: 'inherit' })
if (portablePacked.status !== 0) throw new Error('Could not create portable terminal-test ZIP archive. Python 3 is required for packaging.')
console.log(`Terminal test packages prepared: ${release} and ${portableArchive}`)
