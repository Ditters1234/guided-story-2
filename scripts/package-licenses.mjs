import { readFile, readdir, realpath, stat } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { dirname, isAbsolute, join, relative } from 'node:path';
import { toolchainEnv } from './toolchain.mjs';

async function licenseTexts(directory, declared) {
  const root = await realpath(directory);
  const files = (await readdir(root, { withFileTypes: true }))
    .filter(entry => entry.isFile() && /^(licen[cs]e|copying|copyright|notice)([.\-_]|$)/i.test(entry.name))
    .map(entry => join(root, entry.name));
  if (declared) files.push(isAbsolute(declared) ? declared : join(root, declared));
  const texts = [];
  for (const file of [...new Set(files)].sort()) {
    const resolved = await realpath(file);
    const name = relative(root, resolved);
    if (name === '..' || name.startsWith('..\\') || name.startsWith('../') || isAbsolute(name)) {
      throw new Error('A dependency license path leaves its package directory.');
    }
    const info = await stat(resolved);
    if (!info.isFile() || info.size > 1024 * 1024) throw new Error('Unsupported dependency license file.');
    texts.push(`${name.replaceAll('\\', '/')}\n${await readFile(resolved, 'utf8')}`);
  }
  return texts;
}

export async function packageLicenses(root) {
  const sections = [await readFile(join(root, 'THIRD_PARTY_NOTICES.md'), 'utf8')];
  const missing = [];
  async function add(label, expression, directory, declared, repository) {
    const texts = await licenseTexts(directory, declared);
    const source = typeof repository === 'string' ? repository : repository?.url;
    sections.push(`\n\n--- ${label} ---\nLicense metadata: ${expression || 'See package license files'}\n${source ? `Source: ${source}\n` : ''}${texts.length ? texts.join('\n\n') : 'No separate license-text file was present in the installed package; its published license metadata is retained above.'}`);
    if (!texts.length) missing.push(label);
  }
  const lock = JSON.parse(await readFile(join(root, 'package-lock.json'), 'utf8'));
  for (const [path, metadata] of Object.entries(lock.packages).sort()) {
    if (!path || metadata.dev) continue;
    if (!path.startsWith('node_modules/') || path.split('/').includes('..')) throw new Error('Unexpected dependency path.');
    const directory = join(root, path);
    const pkg = JSON.parse(await readFile(join(directory, 'package.json'), 'utf8'));
    await add(`npm: ${pkg.name} ${pkg.version}`, pkg.license, directory, null, pkg.repository);
  }
  const host = execFileSync('rustc', ['-vV'], { cwd: root, env: toolchainEnv(), encoding: 'utf8' }).match(/^host: (.+)$/m)?.[1].trim();
  if (!host?.includes('windows')) throw new Error('Build the Windows target before collecting its notices.');
  const metadata = JSON.parse(execFileSync(process.execPath,
    [join(root, 'scripts/cargo.mjs'), 'metadata', '--offline', '--locked', '--format-version', '1', '--filter-platform', host],
    { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
  sections.push('\nRust lockfile dependencies follow, including build/test/other-platform packages that may not be linked into this executable.');
  for (const pkg of metadata.packages.filter(pkg => pkg.source).sort((a, b) => a.id.localeCompare(b.id))) {
    await add(`Rust: ${pkg.name} ${pkg.version}`, pkg.license, dirname(pkg.manifest_path), pkg.license_file, pkg.repository);
  }
  return { text: sections.join('\n') + '\n', missing };
}
