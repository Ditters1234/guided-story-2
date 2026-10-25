<script lang="ts">
  import { SvelteFlow, Background, Controls, BackgroundVariant, useSvelteFlow, type Edge } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import type { World, Workspace } from './model';
  import { buildGraph, type WorldNode as WorldNodeType } from './graph';
  import WorldNode from './WorldNode.svelte';
  let { world, workspace, search, projectId, editable, onSelect, onToggle, onPositions, onLink, onConnect }: {
    world: World; workspace: Workspace; search: string;
    projectId: string | null;
    editable: boolean; onLink: (id: string) => void; onConnect: (source: string, target: string) => void;
    onSelect: (id: string) => void; onToggle: (id: string) => void;
    onPositions: (nodes: WorldNodeType[]) => void;
  } = $props();
  let nodes = $state<WorldNodeType[]>([]);
  let edges = $state<Edge[]>([]);
  const nodeTypes = { world: WorldNode };
  const { fitView } = useSvelteFlow();
  function fitSettledPanes() { requestAnimationFrame(() => requestAnimationFrame(() => { void fitView({ padding: 0.12 }); })); }
  $effect(() => { const graph = buildGraph(world, workspace, search, onToggle); nodes = graph.nodes; edges = graph.edges; });
  $effect(() => { void projectId; fitSettledPanes(); });
</script>

<div class="world-canvas" aria-label="World map">
  <SvelteFlow bind:nodes bind:edges {nodeTypes} fitView minZoom={0.25} maxZoom={1.6} oninit={fitSettledPanes}
    nodesConnectable={editable} deleteKey={null} onnodeclick={({ node }) => onSelect(node.id)}
    onedgeclick={({edge}) => { if (editable) onLink(edge.id); }}
    onbeforeconnect={(connection) => { if (editable) onConnect(connection.source, connection.target); return null; }}
    onnodedragstop={({ nodes: moved }) => onPositions(moved)} attributionPosition="bottom-right">
    <Background variant={BackgroundVariant.Dots} patternColor="#c7d1c9" gap={24} />
    <Controls showLock={false} />
  </SvelteFlow>
</div>
