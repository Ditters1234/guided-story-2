import type { MarkerType, Edge, Node } from '@xyflow/svelte';
import type { World, Workspace } from './model';

export type RecordKind = 'place' | 'character' | 'item' | 'thread' | 'custom';
export type WorldRecord = {
  id: string; name: string; description: string; visualDescription: string;
  kind: RecordKind; parentId: string | null; relation: string; detail: string;
};
export type WorldNodeData = WorldRecord & { count: number; expanded: boolean; onToggle: (id: string) => void };
export type WorldNode = Node<WorldNodeData, 'world'>;

export function worldRecords(world: World): WorldRecord[] {
  const nodes = new Map(world.nodes.map((n) => [n.id, n]));
  const relations = new Map(world.relationTypes.map((d) => [d.id, d]));
  const parents = new Map(world.edges.filter((e) => relations.get(e.typeId)?.physicalPlacement).map((e) => [e.sourceId, e]));
  const wornIds = new Set(world.edges.filter((e) => e.typeId === 'core.worn-by').map((e) => e.sourceId));
  // A nonphysical thread grouping is only a view. It must never close a parent cycle.
  for (const edge of world.edges) {
    if (edge.typeId !== 'core.related' || !nodes.get(edge.sourceId)?.components['core.thread'] || nodes.get(edge.targetId)?.components['core.thread'] || parents.has(edge.sourceId)) continue;
    let ancestor: string | undefined = edge.targetId;
    const seen = new Set([edge.sourceId]);
    while (ancestor && !seen.has(ancestor)) { seen.add(ancestor); ancestor = parents.get(ancestor)?.targetId; }
    if (!ancestor) parents.set(edge.sourceId, edge);
  }
  return world.nodes.map((node) => {
    const parent = parents.get(node.id);
    const worn = wornIds.has(node.id);
    const kind: RecordKind = node.components['core.character'] ? 'character' : node.components['core.place'] ? 'place' : node.components['core.item'] ? 'item' : node.components['core.thread'] ? 'thread' : 'custom';
    const preset=world.nodeTypes.find((d)=>d.id===node.typeId);
    const item=node.components['core.item'];
    return {id:node.id,name:node.name,description:node.description,visualDescription:node.visualDescription,kind,parentId:parent?.targetId??null,
      relation:worn?'worn by':parent?.typeId==='core.stored-in'&&nodes.get(parent.targetId)?.components['core.character']?'carried by':relations.get(parent?.typeId ?? '')?.label??'related to',
      detail:node.components['core.container']?'Container':item ? String(item.quantity)+' '+String(item.unit)+(item.quantity!==1?'s':'') :preset?.name??'World node'};
  });
}

export function recordDepth(record: WorldRecord, records: WorldRecord[]): number {
  const seen = new Set([record.id]);
  let depth = 0, parent = record.parentId;
  while (parent && !seen.has(parent)) {
    seen.add(parent); depth++;
    parent = records.find((r) => r.id === parent)?.parentId ?? null;
  }
  return depth;
}

export function orderedRecords(records: WorldRecord[]): WorldRecord[] {
  const ids = new Set(records.map((r) => r.id));
  const children = new Map<string, WorldRecord[]>();
  for (const record of records) if (record.parentId) children.set(record.parentId, [...(children.get(record.parentId) ?? []), record]);
  const stack = records.filter((r) => !r.parentId || !ids.has(r.parentId)).reverse();
  const rows: WorldRecord[] = [], seen = new Set<string>();
  while (stack.length) {
    const record = stack.pop()!;
    if (seen.has(record.id)) continue;
    seen.add(record.id); rows.push(record);
    stack.push(...[...(children.get(record.id) ?? [])].reverse());
  }
  // An invalid recoverable draft can contain cycles. Keep its nodes reachable for repair.
  return [...rows, ...records.filter((r) => !seen.has(r.id))];
}

export function revealRecord(records: WorldRecord[], expandedIds: string[], id: string | null): string[] {
  const byId = new Map(records.map((r) => [r.id, r]));
  const expanded = new Set(expandedIds), seen = new Set<string>();
  let parent = id ? byId.get(id)?.parentId : null;
  while (parent && !seen.has(parent)) { seen.add(parent); expanded.add(parent); parent = byId.get(parent)?.parentId; }
  return [...expanded];
}

export function visibleRecords(records: WorldRecord[], expandedIds: string[], search = ''): WorldRecord[] {
  const byId = new Map(records.map((r) => [r.id, r]));
  const expanded = new Set(expandedIds);
  const wanted = new Set<string>();
  if (search.trim()) {
    for (const record of records.filter((r) => `${r.name} ${r.description}`.toLowerCase().includes(search.trim().toLowerCase()))) {
      let next: WorldRecord | undefined = record;
      while (next && !wanted.has(next.id)) { wanted.add(next.id); next = next.parentId ? byId.get(next.parentId) : undefined; }
    }
    return records.filter((r) => wanted.has(r.id));
  }
  return records.filter((record) => {
    const seen = new Set([record.id]);
    let parent = record.parentId;
    while (parent && byId.has(parent)) {
      if (!expanded.has(parent) || seen.has(parent)) return false;
      seen.add(parent); parent = byId.get(parent)!.parentId;
    }
    return true;
  });
}

export function buildGraph(world: World, workspace: Workspace, search: string, onToggle: (id: string) => void): { nodes: WorldNode[]; edges: Edge[] } {
  const records = worldRecords(world);
  const visible = visibleRecords(records, workspace.expandedIds, search);
  const ids = new Set(visible.map((r) => r.id));
  const layout = new Map<string, { x: number; y: number }>();
  let row = 0;
  const children = new Map<string, WorldRecord[]>();
  for (const record of visible) if (record.parentId) children.set(record.parentId, [...(children.get(record.parentId) ?? []), record]);
  const stack = visible.filter((r) => !r.parentId || !ids.has(r.parentId)).reverse().map((record) => ({ record, depth: 0, done: false }));
  const visited = new Set<string>();
  while (stack.length) {
    const {record, depth, done} = stack.pop()!;
    const nested = children.get(record.id) ?? [];
    if (!done) {
      if (visited.has(record.id)) continue;
      visited.add(record.id); stack.push({record, depth, done: true});
      for (const child of [...nested].reverse()) stack.push({record: child, depth: depth + 1, done: false});
    } else {
      const ys = nested.map((r) => layout.get(r.id)?.y).filter((y): y is number => y !== undefined);
      layout.set(record.id, {x: depth * 280, y: ys.length ? (ys[0] + ys.at(-1)!) / 2 : row++ * 120});
    }
  }
  const nodes: WorldNode[] = visible.map((record) => ({
    id: record.id, type: 'world', position: { x: workspace.positions[record.id]?.x ?? layout.get(record.id)?.x ?? 0, y: workspace.positions[record.id]?.y ?? layout.get(record.id)?.y ?? 0 },
    selected: workspace.selectedId === record.id,
    data: { ...record, count: records.filter((r) => r.parentId === record.id).length, expanded: workspace.expandedIds.includes(record.id), onToggle },
  }));
  const edges: Edge[] = [];
  for (const edge of world.edges) {
    if (!ids.has(edge.sourceId) || !ids.has(edge.targetId)) continue;
    const definition = world.relationTypes.find((r) => r.id === edge.typeId);
    edges.push({ id: edge.id, source: edge.sourceId, target: edge.targetId, label: definition?.label ?? edge.typeId,
      type: definition?.physicalPlacement ? 'smoothstep' : 'default',
      style: definition?.physicalPlacement ? 'stroke: #8ca692;' : 'stroke: #a88662; stroke-dasharray: 5 5;',
      ariaLabel: `${records.find((r) => r.id === edge.sourceId)?.name} ${definition?.label} ${records.find((r) => r.id === edge.targetId)?.name}`,
      interactionWidth: 24, markerEnd: { type: 'arrowclosed' as MarkerType } });
  }
  return { nodes, edges };
}

export function editRecord(world: World, id: string, field: 'name' | 'description' | 'visualDescription', value: string): World {
  return {...world,nodes:world.nodes.map((node)=>node.id===id?{...node,[field]:value}:node)};
}
