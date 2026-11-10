<script lang="ts">
  import {tick, untrack} from 'svelte';
  import {copyMapNodes, mapShortcut, retainMapSelection, type MapClipboard} from './map-actions';
  import {commands,unwrap} from './native';
  import type {MediaEntry, WorldEdit} from './bindings';
  import { SvelteFlow, Background, Controls, BackgroundVariant, useSvelteFlow, type Edge } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import type { World, Workspace } from './model';
  import { buildGraph, type WorldNode as WorldNodeType } from './graph';
  import WorldNode from './WorldNode.svelte';
  import NodeLibrary from './NodeLibrary.svelte';
  import SaveLibraryDialog from './SaveLibraryDialog.svelte';
  import { libraryDragType, readLibraryDrag, libraryPositions } from './node-library';
  let { world, workspace, search, projectId, storyId,revision, active = true, colorMode, editable, aiEditable, onAiSelection, onSelect, onToggle, onPositions, onLink, onConnect, onEdit, onExpand }: {
    world: World; workspace: Workspace; search: string;
    projectId: string | null;
    storyId:string|null;revision:number;
    active?: boolean;
    colorMode: 'light' | 'dark';
    editable: boolean; onLink: (id: string) => void; onConnect: (source: string, target: string) => void;
    onEdit: (edit: WorldEdit, positions?: Workspace['positions']) => void;
    onExpand: (ids: string[], expanded: boolean) => void;
    aiEditable:boolean;onAiSelection:(ids:string[])=>void;
    onSelect: (id: string) => void; onToggle: (id: string) => void;
    onPositions: (nodes: WorldNodeType[]) => void;
  } = $props();
  let nodes = $state<WorldNodeType[]>([]);
  let edges = $state<Edge[]>([]);
  let portraits=$state<Record<string,MediaEntry>>({});
  $effect(()=>{const story=storyId,current=world;void revision;let disposed=false;if(story)untrack(()=>{void commands.mapImages(story,$state.snapshot(current)).then(unwrap).then(result=>{if(!disposed)portraits=result;}).catch(()=>{if(!disposed)portraits={};});});else portraits={};return()=>{disposed=true;};});
  const nodeTypes = { world: WorldNode };
  const { fitView, screenToFlowPosition } = useSvelteFlow();
  let canvas: HTMLDivElement;
  let flowContainer: HTMLDivElement;
  let showLibrary=$state(false),libraryLoading=$state(false),libraryError=$state(''),libraryDragging=$state(false);
  let library=$state<{refresh:()=>Promise<void>}>();
  let exportSelection=$state<{world:World;ids:string[];positions:Workspace['positions']}|null>(null);
  let clipboard = $state<{projectId: string | null; data: MapClipboard; positions: Workspace['positions']} | null>(null);
  let context = $state<{x:number;y:number} | null>(null);
  let menu = $state<HTMLDivElement>();
  let selectionMenu: HTMLDetailsElement;
  let notice = $state('');
  let lastProject: string | null = null, lastAnchor: string | null = null;
  let lastWorldIds = new Set<string>();
  const selectedNodes = $derived(nodes.filter(node => node.selected));
  const selectedEdges = $derived(edges.filter(edge => edge.selected));
  const canPaste = $derived(editable && !!clipboard && clipboard.projectId === projectId);
  function focusCanvas() { if(selectionMenu)selectionMenu.open=false; canvas?.focus({preventScroll:true}); }
  function reviseSelection(){if(aiEditable&&selectedNodes.length){context=null;onAiSelection(selectedNodes.map(node=>node.id));}}
  function saveSelection() {
    if(!editable||!selectedNodes.length)return;
    exportSelection={world:structuredClone($state.snapshot(world)),ids:selectedNodes.map(node=>node.id),positions:Object.fromEntries(selectedNodes.map(node=>[node.id,{...node.position}]))};
    context=null;
  }
  async function insertLibrary(file:{fileName:string;sha256:string},point?:{x:number;y:number}) {
    if(!editable||libraryLoading)return;
    const boundary=projectId,source=JSON.stringify(world);
    libraryLoading=true;libraryError='';
    try {
      const entry=unwrap(await commands.loadNodeLibrary(file.fileName,file.sha256));
      if(boundary!==projectId||source!==JSON.stringify(world)||!editable)throw Error('The world changed. Add this library entry again from the current world.');
      const bounds=flowContainer.getBoundingClientRect();
      const at=point??screenToFlowPosition({x:bounds.left+bounds.width/3,y:bounds.top+bounds.height/3});
      onEdit({kind:'insertLibrary',entry},libraryPositions(entry,at));
    }catch(caught){libraryError=String(caught);}finally{libraryLoading=false;}
  }
  function dropLibrary(event:DragEvent) {
    if(!event.dataTransfer?.types.includes(libraryDragType))return;
    event.preventDefault();event.stopPropagation();libraryDragging=false;
    const file=readLibraryDrag(event.dataTransfer.getData(libraryDragType));
    if(file)void insertLibrary(file,screenToFlowPosition({x:event.clientX,y:event.clientY}));
  }
  function copy() {
    const data = copyMapNodes($state.snapshot(world), selectedNodes.map(node => node.id));
    if (!data) return;
    clipboard = {projectId, data, positions:Object.fromEntries(selectedNodes.map(node => [node.id, {...node.position}]))};
    notice = `Copied ${data.nodes.length} ${data.nodes.length === 1 ? 'node' : 'nodes'} and ${data.edges.length} internal ${data.edges.length === 1 ? 'connection' : 'connections'}.`;
    context = null; focusCanvas();
  }
  function paste(duplicate = false) {
    if (!editable) return;
    if (duplicate) copy();
    if (!canPaste || !clipboard) return;
    onEdit(structuredClone($state.snapshot(clipboard.data)), structuredClone($state.snapshot(clipboard.positions)));
    clipboard = {...clipboard,positions:Object.fromEntries(Object.entries(clipboard.positions).map(([id,p])=>[id,{x:(p.x??0)+48,y:(p.y??0)+48}]))};
    context = null;
  }
  function remove(connectionsOnly = false) {
    if (!editable) return;
    const nodeIds = connectionsOnly ? [] : selectedNodes.map(node => node.id);
    const edgeIds = selectedEdges.map(edge => edge.id);
    if (nodeIds.length || edgeIds.length) onEdit({kind:'removeSelection', nodeIds, edgeIds});
    context = null;
  }
  function selectAll() { nodes = nodes.map(node => ({...node, selected:true})); edges = edges.map(edge => ({...edge, selected:false})); context = null; focusCanvas(); }
  function clearSelection() { nodes = nodes.map(node => ({...node, selected:false})); edges = edges.map(edge => ({...edge, selected:false})); context = null; focusCanvas(); }
  function keydown(event: KeyboardEvent) {
    if (!active || (event.target instanceof Element && event.target.closest('input,textarea,select,[contenteditable="true"],button'))) return;
    const action = mapShortcut(event);
    if (!action || (!editable && ['paste','duplicate','delete'].includes(action))) return;
    event.preventDefault(); event.stopPropagation();
    if (action === 'copy') copy();
    else if (action === 'paste') paste();
    else if (action === 'duplicate' && selectedNodes.length) paste(true);
    else if (action === 'delete') remove();
    else if (action === 'all') selectAll();
    else clearSelection();
  }
  async function openMenu(event: MouseEvent, kind: 'node'|'edge'|'pane'|'selection', id?: string) {
    event.preventDefault(); event.stopPropagation();
    if (kind === 'node' && !nodes.some(node => node.id === id && node.selected)) {
      nodes = nodes.map(node => ({...node,selected:node.id===id})); edges = edges.map(edge => ({...edge,selected:false}));
      if(id) {lastAnchor=id;onSelect(id);}
    } else if (kind === 'edge' && !edges.some(edge => edge.id === id && edge.selected)) {
      nodes=nodes.map(node=>({...node,selected:false}));edges=edges.map(edge=>({...edge,selected:edge.id===id}));
    } else if (kind === 'pane') clearSelection();
    context = {x:Math.max(4,Math.min(event.clientX,window.innerWidth-240)),y:Math.max(4,Math.min(event.clientY,window.innerHeight-410))};
    await tick();
    if(menu) {
      const bounds=menu.getBoundingClientRect();
      context={x:Math.max(4,Math.min(event.clientX,window.innerWidth-bounds.width-6)),y:Math.max(4,Math.min(event.clientY,window.innerHeight-bounds.height-6))};
      menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    }
  }
  function expand(expanded:boolean) { onExpand(selectedNodes.map(node=>node.id),expanded);context=null;focusCanvas(); }
  function fitSelection() {void fitView({nodes:selectedNodes.length?selectedNodes:undefined,padding:0.2});context=null;focusCanvas();}
  function fitSettledPanes() { requestAnimationFrame(() => requestAnimationFrame(() => { void fitView({ padding: 0.12 }); })); }
  $effect(() => {
    const graph = buildGraph(world, workspace, search, onToggle);
    const currentProject=projectId, anchor=workspace.selectedId;
    const previous=untrack(()=>({nodes:nodes.map(node=>({id:node.id,selected:node.selected})),edges:edges.map(edge=>({id:edge.id,selected:edge.selected}))}));
    const preserve=currentProject===lastProject && anchor===lastAnchor;
    const added=currentProject===lastProject?new Set(world.nodes.filter(node=>!lastWorldIds.has(node.id)).map(node=>node.id)):new Set<string>();
    nodes = retainMapSelection(graph.nodes,previous.nodes,preserve).map(node=>{const image=portraits[node.id];return {...node,...(added.size?{selected:added.has(node.id)}:{}),data:{...node.data,imageId:image?.applicable&&!image.error?image.id:undefined,imageStoryId:storyId??undefined}};});
    const selected=new Set(preserve?previous.edges.filter(edge=>edge.selected).map(edge=>edge.id):[]);
    edges=graph.edges.map(edge=>({...edge,selected:selected.has(edge.id)}));
    if(currentProject!==lastProject){clipboard=null;notice='';context=null;}
    lastProject=currentProject;lastAnchor=anchor;lastWorldIds=new Set(world.nodes.map(node=>node.id));
  });
  $effect(() => { if (active) { void projectId; fitSettledPanes(); } });
</script>

<svelte:window onpointerdown={(event)=>{if(context && event.target instanceof Node && !menu?.contains(event.target))context=null;}} onkeydown={(event)=>{if(context&&event.key==='Escape'){event.preventDefault();context=null;focusCanvas();}}} />
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (The interactive graph canvas is focusable for its selection and editing shortcuts.) -->
<div class="world-canvas" aria-label="World map" role="application" tabindex="0" bind:this={canvas} onkeydown={keydown}>
  <div class="map-actions" role="group" aria-label="Map actions">
    <span>{selectedNodes.length} {selectedNodes.length===1?'node':'nodes'}{selectedEdges.length?` · ${selectedEdges.length} ${selectedEdges.length===1?'connection':'connections'}`:''} selected</span>
    <button class="quiet-button" onclick={copy} disabled={!selectedNodes.length} title="Copy selected nodes (Ctrl+C)">Copy</button>
    <button class="quiet-button" onclick={()=>paste()} disabled={!canPaste} title="Paste copied nodes (Ctrl+V)">Paste…</button>
    <button class="quiet-button" onclick={()=>paste(true)} disabled={!editable||!selectedNodes.length} title="Duplicate selected nodes (Ctrl+D)">Duplicate…</button>
    <button class="quiet-button" onclick={()=>remove()} disabled={!editable||(!selectedNodes.length&&!selectedEdges.length)} title="Review deletion (Delete)">Delete…</button>
    <button class="quiet-button" onclick={()=>remove(true)} disabled={!editable||!selectedEdges.length}>Unlink…</button>
    <button class="quiet-button" onclick={reviseSelection} disabled={!aiEditable||!selectedNodes.length}>Revise with AI…</button>
    <button class="quiet-button" onclick={()=>{showLibrary=!showLibrary;}} aria-expanded={showLibrary}>Library</button>
    {#if selectedEdges.length===1}<button class="quiet-button" onclick={()=>onLink(selectedEdges[0].id)} disabled={!editable}>Edit connection…</button>{/if}
    <details bind:this={selectionMenu}><summary>Selection</summary><div class="map-selection-actions">
      <button class="quiet-button" onclick={selectAll}>Select all visible (Ctrl+A)</button>
      <button class="quiet-button" onclick={clearSelection}>Clear selection (Esc)</button>
      <button class="quiet-button" onclick={saveSelection} disabled={!editable||!selectedNodes.length}>Save selection to library…</button>
      <button class="quiet-button" onclick={()=>expand(true)} disabled={!selectedNodes.length}>Expand selected</button>
      <button class="quiet-button" onclick={()=>expand(false)} disabled={!selectedNodes.length}>Collapse selected</button>
      <button class="quiet-button" onclick={fitSelection}>Fit {selectedNodes.length?'selection':'map'}</button>
    </div></details>
    <span class="map-selection-help">Ctrl+click to select · Shift+drag to box select</span>
  </div>
  {#if libraryError}<p class="inline-error map-library-error" role="alert">{libraryError}</p>{/if}
  <div class="map-body">
  {#if showLibrary}<NodeLibrary bind:this={library} editable={editable&&!libraryLoading} onDrag={active=>{libraryDragging=active;}} onInsert={file=>{void insertLibrary(file);}} onClose={()=>{showLibrary=false;libraryDragging=false;}}/>{/if}
  <!-- svelte-ignore a11y_no_static_element_interactions (This is the graph's drop target; every library entry also has a keyboard-accessible Add button.) -->
  <div class="map-flow" class:library-drop-target={libraryDragging&&editable} bind:this={flowContainer} ondrop={dropLibrary} ondragover={event=>{if(editable&&event.dataTransfer?.types.includes(libraryDragType)){event.preventDefault();event.dataTransfer.dropEffect='copy';}}}>
  <SvelteFlow bind:nodes bind:edges {nodeTypes} {colorMode} fitView minZoom={0.25} maxZoom={1.6} oninit={fitSettledPanes}
    nodesConnectable={editable} deleteKey={null} onnodeclick={({ node }) => {lastAnchor=node.id;onSelect(node.id);}}
    onedgeclick={({event,edge}) => { if (editable && event.detail===2) onLink(edge.id); }}
    onnodecontextmenu={({event,node})=>{void openMenu(event,'node',node.id);}}
    onedgecontextmenu={({event,edge})=>{void openMenu(event,'edge',edge.id);}}
    onpanecontextmenu={({event})=>{void openMenu(event,'pane');}}
    onselectioncontextmenu={({event})=>{void openMenu(event,'selection');}}
    onbeforeconnect={(connection) => { if (editable) onConnect(connection.source, connection.target); return null; }}
    onnodedragstop={({ nodes: moved }) => onPositions(moved)} attributionPosition="bottom-right">
    <Background variant={BackgroundVariant.Dots} patternColor="var(--canvas-dots)" gap={24} />
    <Controls showLock={false} />
  </SvelteFlow>
  </div>
  </div>
  {#if notice}<span class="map-copy-notice" role="status">{notice}</span>{/if}
</div>
{#if context}
  <div class="map-context-actions" bind:this={menu} role="group" aria-label="Selection actions" style:left={context.x+'px'} style:top={context.y+'px'}>
    <button class="quiet-button" onclick={copy} disabled={!selectedNodes.length}>Copy (Ctrl+C)</button>
    <button class="quiet-button" onclick={()=>paste()} disabled={!canPaste}>Paste… (Ctrl+V)</button>
    <button class="quiet-button" onclick={()=>paste(true)} disabled={!editable||!selectedNodes.length}>Duplicate… (Ctrl+D)</button>
    <button class="quiet-button" onclick={saveSelection} disabled={!editable||!selectedNodes.length}>Save to library…</button>
    <button class="quiet-button" onclick={reviseSelection} disabled={!aiEditable||!selectedNodes.length}>Revise selection with AI…</button>
    <button class="quiet-button" onclick={()=>remove()} disabled={!editable||(!selectedNodes.length&&!selectedEdges.length)}>Delete selection…</button>
    <button class="quiet-button" onclick={()=>remove(true)} disabled={!editable||!selectedEdges.length}>Unlink connections…</button>
    {#if selectedEdges.length===1}<button class="quiet-button" onclick={()=>{onLink(selectedEdges[0].id);context=null;}} disabled={!editable}>Edit connection…</button>{/if}
    {#if selectedNodes.length===1}<button class="quiet-button" onclick={()=>{onConnect(selectedNodes[0].id,'');context=null;}} disabled={!editable}>Connect to…</button>{/if}
    <button class="quiet-button" onclick={()=>expand(true)} disabled={!selectedNodes.length}>Expand selected</button>
    <button class="quiet-button" onclick={()=>expand(false)} disabled={!selectedNodes.length}>Collapse selected</button>
    <button class="quiet-button" onclick={fitSelection}>Fit {selectedNodes.length?'selection':'map'}</button>
    <button class="quiet-button" onclick={selectAll}>Select all visible</button>
    <button class="quiet-button" onclick={clearSelection}>Clear selection</button>
  </div>
{/if}
{#if exportSelection}
  <SaveLibraryDialog world={exportSelection.world} nodeIds={exportSelection.ids} positions={exportSelection.positions} onClose={()=>{exportSelection=null;}} onSaved={file=>{exportSelection=null;notice=`Saved library entry: ${file}`;showLibrary=true;void library?.refresh();}}/>
{/if}
