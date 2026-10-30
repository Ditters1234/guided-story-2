import { isTauri } from '@tauri-apps/api/core';
export { commands } from './bindings';
export const desktop = isTauri();

export class RejectedChange extends Error {}
export function unwrap<T>(result: { status: 'ok'; data: T } | { status: 'error'; error: string }): T {
  if (result.status === 'error') throw new RejectedChange(result.error);
  return result.data;
}
