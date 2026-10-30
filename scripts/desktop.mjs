import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { toolchainEnv } from './toolchain.mjs';

const require = createRequire(import.meta.url);
const cli = require.resolve('@tauri-apps/cli/tauri.js');
const child = spawn(process.execPath, [cli, ...process.argv.slice(2)], { env: toolchainEnv(), stdio: 'inherit' });
child.on('error', (error) => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code) => { process.exitCode = code ?? 1; });
