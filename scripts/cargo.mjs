import { spawn } from 'node:child_process';
import { toolchainEnv } from './toolchain.mjs';

const child = spawn('cargo', process.argv.slice(2), { env: toolchainEnv(), stdio: 'inherit' });
child.on('error', (error) => { console.error(`Rust toolchain unavailable: ${error.message}`); process.exitCode = 1; });
child.on('exit', (code) => { process.exitCode = code ?? 1; });
