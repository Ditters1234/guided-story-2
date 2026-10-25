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
  return world.nodes.map((node) => {
    const parent = world.edges.find((e) => e.sourceId === node.id && world.relationTypes.find((r) => r.id === e.typeId)?.physicalPlacement)
      ?? (node.components['core.thread'] ? world.edges.find((e) => e.sourceId === node.id && e.typeId === 'core.related' && !world.nodes.find((n) => n.id === e.targetId)?.components['core.thread']) : undefined);
    const worn = world.edges.find((e) => e.sourceId === node.id && e.typeId === 'core.worn-by');
    const kind: RecordKind = node.components['core.character'] ? 'character' : node.components['core.place'] ? 'place' : node.components['core.item'] ? 'item' : node.components['core.thread'] ? 'thread' : 'custom';
    const preset=world.nodeTypes.find((d)=>d.id===node.typeId);
    const item=node.components['core.item'];
    return {id:node.id,name:node.name,description:node.description,visualDescription:node.visualDescription,kind,parentId:parent?.targetId??null,
      relation:worn?'worn by':parent?.typeId==='core.stored-in'&&world.nodes.find((n)=>n.id===parent.targetId)?.components['core.character']?'carried by':world.relationTypes.find((r)=>r.id===parent?.typeId)?.label??'related to',
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
  function visit(record: WorldRecord, depth: number, path = new Set<string>()): number {
    if (path.has(record.id)) return row * 120;
    const children = visible.filter((r) => r.parentId === record.id);
    const nextPath = new Set([...path, record.id]);
    const positions = children.map((child) => visit(child, depth + 1, nextPath));
    const y = positions.length ? (positions[0] + positions.at(-1)!) / 2 : row++ * 120;
    layout.set(record.id, { x: depth * 280, y });
    return y;
  }
  visible.filter((r) => !r.parentId || !ids.has(r.parentId)).forEach((r) => visit(r, 0));
  const nodes: WorldNode[] = visible.map((record) => ({
    id: record.id, type: 'world', position: { x: workspace.positions[record.id]?.x ?? layout.get(record.id)?.x ?? 0, y: workspace.positions[record.id]?.y ?? layout.get(record.id)?.y ?? 0 },
    selected: workspace.selectedId === record.id,
    data: { ...record, count: records.filter((r) => r.parentId === record.id).length, expanded: workspace.expandedIds.includes(record.id), onToggle },
  }));
  const edges: Edge[] = visible.filter((r) => r.parentId && ids.has(r.parentId)).map((r) => ({
    id: `${r.parentId}:${r.id}`, source: r.parentId!, target: r.id, label: r.relation,
    style: r.kind === 'thread' ? 'stroke: #a88662; stroke-dasharray: 5 5;' : 'stroke: #bdc9c2;',
    type: 'smoothstep',
  }));
  for (const edge of world.edges) {
    if (!ids.has(edge.sourceId) || !ids.has(edge.targetId)) continue;
    if (visible.find((r) => r.id === edge.sourceId)?.parentId === edge.targetId) continue;
    const definition = world.relationTypes.find((r) => r.id === edge.typeId);
    edges.push({ id: edge.id, source: edge.sourceId, target: edge.targetId, label: definition?.label ?? edge.typeId,
      type: 'smoothstep', style: 'stroke: #a88662; stroke-dasharray: 5 5;', markerEnd: { type: 'arrowclosed' as MarkerType } });
  }
  return { nodes, edges };
}

export function editRecord(world: World, id: string, field: 'name' | 'description' | 'visualDescription', value: string): World {
  return {...world,nodes:world.nodes.map((node)=>node.id===id?{...node,[field]:value}:node)};
}
