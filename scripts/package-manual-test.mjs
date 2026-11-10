import { copyFile, mkdir, readFile, writeFile, cp } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { packageLicenses } from './package-licenses.mjs';
import { toolchainEnv } from './toolchain.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
execFileSync(process.execPath, [join(root, 'scripts/cargo.mjs'), 'run', '-p', 'story-project', '--bin', 'check_bundled_schemas', '--locked'], { cwd: root, env: toolchainEnv(), stdio: 'inherit' });
if (process.platform !== 'win32') throw new Error('This manual testing package targets Windows.');
const executable = join(root, 'target/release/guided-story-2.exe');
const binary = await readFile(executable);
if (binary[0] !== 0x4d || binary[1] !== 0x5a) throw new Error('Build the Windows release executable first.');
const pkg = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));
const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
const commit = git('rev-parse', 'HEAD');
const createdAt = new Date().toISOString();
const notices = await packageLicenses(root);
const folder = join(root, 'build', `manual-test-${pkg.version}-${createdAt.replace(/[:.]/g, '-')}`);
// Each handoff has a new folder. Never delete or replace an earlier build or story data.
await mkdir(dirname(folder), { recursive: true });
await mkdir(folder);
await copyFile(executable, join(folder, 'Guided Story 2.exe'));
await copyFile(join(root, 'MANUAL_TESTING.md'), join(folder, 'START HERE.md'));
await copyFile(join(root, 'NODE_LIBRARY.md'), join(folder, 'NODE LIBRARY.md'));
await copyFile(join(root, 'AI_AUTHORING.md'), join(folder, 'AI WORLD AUTHORING.md'));
await cp(join(root, 'examples/node-library'), join(folder, 'Library examples'), { recursive: true, errorOnExist: true, force: false });
await writeFile(join(folder, 'THIRD_PARTY_NOTICES.txt'), notices.text, { flag: 'wx' });
await writeFile(join(folder, 'BUILD.json'), JSON.stringify({
  application: 'Guided Story 2', version: pkg.version, platform: process.platform, arch: process.arch,
  createdAt, sourceCommit: commit, workingTreeModified: git('status', '--porcelain').length > 0,
  executableSha256: createHash('sha256').update(binary).digest('hex'),
  contents: ['Guided Story 2.exe', 'START HERE.md', 'NODE LIBRARY.md', 'AI WORLD AUTHORING.md', 'Library examples', 'THIRD_PARTY_NOTICES.txt', 'BUILD.json'],
  dependenciesWithoutPackagedLicenseText: notices.missing,
}, null, 2) + '\n', { flag: 'wx' });
console.log(`Manual testing build: ${folder}`);
if (process.argv.includes('--open')) {
  const explorer = spawn('explorer.exe', [folder], { detached: true, stdio: 'ignore' });
  explorer.on('error', (error) => console.warn(`Build completed, but its folder could not open: ${error.message}`));
  explorer.unref();
}
