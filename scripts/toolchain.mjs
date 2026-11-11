import { existsSync } from 'node:fs';
import { delimiter, join, resolve } from 'node:path';

export function toolchainEnv() {
  const env = { ...process.env };
  const localCargo = resolve('.tools/cargo');
  if (existsSync(join(localCargo, 'bin', 'cargo.exe'))) {
    env.CARGO_HOME = localCargo;
    env.RUSTUP_HOME = resolve('.tools/rustup');
    const pathKey = Object.keys(env).find((key) => key.toLowerCase() === 'path') ?? 'PATH';
    env[pathKey] = `${join(localCargo, 'bin')}${delimiter}${env[pathKey] ?? ''}`;
  }
  return env;
}
