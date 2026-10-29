<script lang="ts">
  import {untrack} from 'svelte';
  import LinkFields from './LinkFields.svelte';
  import {changeLinkType,setLinkProperty} from './link-fields';
  import type { AuthoringRequest,NodeCandidate,PropertyValue,ExistingNodeSummary } from './bindings';
  import type { World } from './model';
  import NodeFields from './NodeFields.svelte';
  import { addCandidateNode,editCandidateNode,addCandidateCapability,addCandidateWornItem,wornItemType,candidateNodeTypes } from './authoring-ui';
  let {candidate,schema,request,existingNodes=[],onChange}:{candidate:NodeCandidate;schema:World;request:AuthoringRequest;existingNodes?:ExistingNodeSummary[];onChange:(value:NodeCandidate)=>void}=$props();
  let selected=$state('');
  let linksOpen=$state(untrack(()=>request.mode==='revise'));
  const linkAllowed=(id:string,property:string|null)=>request.mode==='create'||(request.linkFields??[]).some(f=>f.linkId===id&&f.property===property);
  const node=$derived(candidate.nodes.find((n)=>n.id===selected)??candidate.nodes[0]);
  const references=$derived([...new Map([...existingNodes,...schema.nodes,...candidate.nodes].map(n=>[n.id,n])).values()]);
  const allowedTypes=$derived(candidateNodeTypes(schema,request));
  const canAddGarment=$derived(allowedTypes.some(t=>t.id===wornItemType(schema)));
  const allowed=(id:string,componentId:string|null,key:string)=>request.mode==='create'||request.fields.some((f)=>f.nodeId===id&&f.componentId===componentId&&f.key===key);
  function edit(key:'name'|'description'|'visualDescription',value:string){if(node)onChange(editCandidateNode(candidate,node.id,(n)=>({...n,[key]:value})));}
  function property(componentId:string,key:string,value:PropertyValue){if(node)onChange(editCandidateNode(candidate,node.id,(n)=>({...n,components:{...n.components,[componentId]:{...n.components[componentId],[key]:value}}})));}
  function edge(index:number,key:'typeId'|'sourceId'|'targetId'|'description',value:string){onChange({...candidate,edges:candidate.edges.map((e,i)=>i===index?(key==='typeId'?changeLinkType(schema,e,value):{...e,[key]:value}):e)});}
</script>
<nav class="candidate-node-tabs" aria-label="Proposed nodes">{#each candidate.nodes as entry}<button class:active={node?.id===entry.id} onclick={()=>{selected=entry.id;}}>{entry.name||'Unnamed node'}</button>{/each}</nav>
{#if node}
  <div class="candidate-type">{schema.nodeTypes.find((d)=>d.id===node.typeId)?.name??node.typeId}</div>
  <label class="field">Name<input value={node.name} maxlength={200} disabled={!allowed(node.id,null,'name')} oninput={(event)=>edit('name',event.currentTarget.value)} /></label>
  <label class="field">Story notes<textarea value={node.description} rows={3} maxlength={16000} disabled={!allowed(node.id,null,'description')} oninput={(event)=>edit('description',event.currentTarget.value)}></textarea></label>
  <label class="field">Visible description<textarea value={node.visualDescription} rows={3} maxlength={16000} disabled={!allowed(node.id,null,'visualDescription')} oninput={(event)=>edit('visualDescription',event.currentTarget.value)}></textarea></label>
  {#if request.mode==='create'&&node.components['core.character']}<section class="capability"><strong>Worn items</strong>{#each candidate.edges.filter(e=>e.typeId==='core.worn-by'&&e.targetId===node!.id) as link}{@const garment=candidate.nodes.find(n=>n.id===link.sourceId)}{#if garment}<button class="text-action" onclick={()=>{selected=garment.id;}}>{garment.name} ↗</button>{/if}{:else}<p class="field-help">No worn items proposed.</p>{/each}{#if request.mode==='create'&&request.bundle}<button class="secondary-button" disabled={!canAddGarment||candidate.nodes.length>=20} onclick={()=>{const next=addCandidateWornItem(schema,candidate,node!.id);selected=next.nodes.at(-1)!.id;onChange(next);}}>＋ Add worn item</button>{/if}</section>{/if}
  <NodeFields world={schema} {node} nodes={references} canEdit={(componentId,key)=>allowed(node!.id,componentId,key)} onChange={property}/>
  {#if request.mode==='create'}<select aria-label="Add capability to proposed node" value="" onchange={event=>{if(event.currentTarget.value){onChange(addCandidateCapability(schema,candidate,node!.id,event.currentTarget.value));event.currentTarget.value='';}}}><option value="">＋ Add capability…</option>{#each schema.componentTypes.filter(d=>!node!.components[d.id]) as capability}<option value={capability.id}>{capability.name}</option>{/each}</select>{/if}
{/if}
{#if request.mode==='create'&&request.bundle}
  <div class="candidate-group-actions"><select aria-label="Add a proposed node" value="" onchange={(event)=>{if(event.currentTarget.value){onChange(addCandidateNode(schema,candidate,event.currentTarget.value));event.currentTarget.value='';}}}><option value="">＋ Add proposed node…</option>{#each allowedTypes as type}<option value={type.id}>{type.name}</option>{/each}</select>{#if node&&node.id!=='new:root'}<button class="text-action danger-text" onclick={()=>{onChange({nodes:candidate.nodes.filter((n)=>n.id!==node!.id),edges:candidate.edges.filter((e)=>e.sourceId!==node!.id&&e.targetId!==node!.id)});}}>Remove proposed node</button>{/if}</div>
{/if}
{#if candidate.edges.length||request.mode==='create'}
  <details class="candidate-links" bind:open={linksOpen}><summary>Proposed connections ({candidate.edges.length})</summary>{#each candidate.edges as link,index}<div class="candidate-link">
    <label class="field">Source<select value={link.sourceId} disabled={request.mode==='revise'} onchange={event=>edge(index,'sourceId',event.currentTarget.value)}>{#each (request.mode==='create'?candidate.nodes:references) as n}<option value={n.id}>{n.name}</option>{/each}</select></label>
    <label class="field">Relationship<select value={link.typeId} disabled={request.mode==='revise'} onchange={event=>edge(index,'typeId',event.currentTarget.value)}>{#each schema.relationTypes as type}<option value={type.id}>{type.name}</option>{/each}</select></label>
    <label class="field">Target<select value={link.targetId} disabled={request.mode==='revise'} onchange={event=>edge(index,'targetId',event.currentTarget.value)}><option value="">Choose target…</option>{#each references.filter(n=>n.id!==link.sourceId) as n}<option value={n.id}>{n.name}</option>{/each}</select></label>
    <div class="candidate-link-details"><label class="field">Connection notes<textarea value={link.description} rows={2} maxlength={16000} disabled={!linkAllowed(link.id,null)} oninput={event=>edge(index,'description',event.currentTarget.value)}></textarea></label>
    <LinkFields world={schema} {link} nodes={references} canEdit={key=>linkAllowed(link.id,key)} onChange={(key,value)=>onChange({...candidate,edges:candidate.edges.map((e,i)=>i===index?setLinkProperty(e,key,value):e)})}/></div>
    {#if request.mode==='create'}<button class="text-action danger-text" onclick={()=>onChange({...candidate,edges:candidate.edges.filter((_,i)=>i!==index)})}>Remove proposed connection</button>{/if}
  </div>{/each}{#if request.mode==='create'}<button class="secondary-button" onclick={()=>onChange({...candidate,edges:[...candidate.edges,{id:'new:link-'+crypto.randomUUID().replaceAll('-',''),typeId:'core.related',sourceId:candidate.nodes[0]?.id??'',targetId:references.find(n=>n.id!==candidate.nodes[0]?.id)?.id??'',description:''}]})}>＋ Add proposed connection</button>{/if}</details>
{/if}
