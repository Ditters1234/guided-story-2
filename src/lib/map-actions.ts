import type { World } from './model';
import type { WorldEdit } from './bindings';

export type MapClipboard = Extract<WorldEdit, { kind: 'pasteNodes' }>;

// A frozen, app-local copy. Selecting a collapsed parent does not select its contents.
export function copyMapNodes(world: World, ids: string[]): MapClipboard | null {
  const selected = new Set(ids);
  const nodes = world.nodes.filter(node => selected.has(node.id));
  if (!nodes.length) return null;
  return structuredClone({ kind: 'pasteNodes', nodes,
    edges: world.edges.filter(edge => selected.has(edge.sourceId) && selected.has(edge.targetId)) });
}

export function mapShortcut(event: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>): 'copy' | 'paste' | 'duplicate' | 'delete' | 'all' | 'clear' | null {
  if (event.altKey) return null;
  if (event.ctrlKey || event.metaKey) {
    if (event.shiftKey) return null;
    return ({ c: 'copy', v: 'paste', d: 'duplicate', a: 'all' } as const)[event.key.toLowerCase() as 'c' | 'v' | 'd' | 'a'] ?? null;
  }
  if (event.key === 'Delete' || event.key === 'Backspace') return 'delete';
  return event.key === 'Escape' ? 'clear' : null;
}

// Rebuilds retain multi-selection across world, portrait and layout updates.
export function retainMapSelection<T extends { id: string; selected?: boolean; position: { x: number; y: number } }>(next: T[], previous: {id: string; selected?: boolean}[], preserve: boolean): T[] {
  const byId = new Map(previous.map(node => [node.id, node]));
  return next.map(node => ({ ...node, ...(preserve && byId.has(node.id) ? { selected: !!byId.get(node.id)!.selected } : {}) }));
}
