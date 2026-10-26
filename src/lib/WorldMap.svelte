<script lang="ts">
  import {untrack} from 'svelte';
  import {commands,unwrap} from './native';
  import type {MediaEntry} from './bindings';
  import { SvelteFlow, Background, Controls, BackgroundVariant, useSvelteFlow, type Edge } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import type { World, Workspace } from './model';
  import { buildGraph, type WorldNode as WorldNodeType } from './graph';
  import WorldNode from './WorldNode.svelte';
  let { world, workspace, search, projectId, storyId,revision, active = true, colorMode, editable, onSelect, onToggle, onPositions, onLink, onConnect }: {
    world: World; workspace: Workspace; search: string;
    projectId: string | null;
    storyId:string|null;revision:number;
    active?: boolean;
    colorMode: 'light' | 'dark';
    editable: boolean; onLink: (id: string) => void; onConnect: (source: string, target: string) => void;
    onSelect: (id: string) => void; onToggle: (id: string) => void;
    onPositions: (nodes: WorldNodeType[]) => void;
  } = $props();
  let nodes = $state<WorldNodeType[]>([]);
  let edges = $state<Edge[]>([]);
  let portraits=$state<Record<string,MediaEntry>>({});
  $effect(()=>{const story=storyId,current=world;void revision;let disposed=false;if(story)untrack(()=>{void commands.mapImages(story,$state.snapshot(current)).then(unwrap).then(result=>{if(!disposed)portraits=result;}).catch(()=>{if(!disposed)portraits={};});});else portraits={};return()=>{disposed=true;};});
  const nodeTypes = { world: WorldNode };
  const { fitView } = useSvelteFlow();
  function fitSettledPanes() { requestAnimationFrame(() => requestAnimationFrame(() => { void fitView({ padding: 0.12 }); })); }
  $effect(() => { const graph = buildGraph(world, workspace, search, onToggle); nodes = graph.nodes.map(node=>{const image=portraits[node.id];return {...node,data:{...node.data,imageId:image?.applicable&&!image.error?image.id:undefined,imageStoryId:storyId??undefined}};}); edges = graph.edges; });
  $effect(() => { if (active) { void projectId; fitSettledPanes(); } });
</script>

<div class="world-canvas" aria-label="World map">
  <SvelteFlow bind:nodes bind:edges {nodeTypes} {colorMode} fitView minZoom={0.25} maxZoom={1.6} oninit={fitSettledPanes}
    nodesConnectable={editable} deleteKey={null} onnodeclick={({ node }) => onSelect(node.id)}
    onedgeclick={({edge}) => { if (editable) onLink(edge.id); }}
    onbeforeconnect={(connection) => { if (editable) onConnect(connection.source, connection.target); return null; }}
    onnodedragstop={({ nodes: moved }) => onPositions(moved)} attributionPosition="bottom-right">
    <Background variant={BackgroundVariant.Dots} patternColor="var(--canvas-dots)" gap={24} />
    <Controls showLock={false} />
  </SvelteFlow>
</div>
