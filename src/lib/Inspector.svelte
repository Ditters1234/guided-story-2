<script lang="ts">
  import type { WorldRecord } from './graph';
  import type { World } from './model';
  import {valueLabel} from './effects-ui';
  import NodeFields from './NodeFields.svelte';
  import NodeImage from './NodeImage.svelte';
  import AiContextBindingEditor from './AiContextBindingEditor.svelte';
  import {hasCapability} from './node-fields';
  import type {AiContextBinding} from './bindings';
  let { record, records, world, editable, historical = false, imageStoryId, revision, onImage, onEdit, onSelect, onProperty, onAddCapability, onDefineCapability, onRetype, onRemoveNode, onRemoveCapability, onLink, onNewLink, onRemoveLink, onAi,onAiLink,onContext,onContextOpen }: {
    record: WorldRecord | undefined; records: WorldRecord[]; world: World; editable: boolean; historical?: boolean;
    imageStoryId:string|null;revision:number;onImage:(id:string)=>void;
    onEdit: (field: 'name' | 'description' | 'visualDescription', value: string) => void;
    onSelect: (id: string) => void;
    onProperty: (componentId: string, key: string, value: string | number | boolean | string[] | null) => void;
    onAddCapability: (componentId: string) => void;onDefineCapability:()=>void;onRetype:(id:string)=>void;
    onRemoveNode: (id: string) => void; onRemoveCapability: (id: string, capability: string) => void;
    onLink: (id: string) => void; onNewLink: (id: string) => void; onRemoveLink: (id: string) => void;
    onAi: (id:string)=>void;onAiLink:(id:string)=>void;
    onContext:(binding:AiContextBinding)=>void;onContextOpen:()=>void;
  } = $props();
  const node = $derived(world.nodes.find((n) => n.id === record?.id));
</script>

<section class="inspector">
  <div class="panel-heading"><span>INSPECTOR</span><span class="tag">{node?world.nodeTypes.find(t=>t.id===node.typeId)?.name:'World'}</span></div>
  {#if record&&node}
    <div class="record-heading"><h2>{record.name || 'Unnamed record'}</h2>
      {#if record.parentId}<button class="parent-link" onclick={() => onSelect(record!.parentId!)}>{record.relation} {records.find((r) => r.id === record!.parentId)?.name ?? 'Unknown record'} ↗</button>{/if}
    </div>
    {#if editable}<button class="secondary-button record-ai-action" onclick={()=>onAi(node!.id)}>Generate / revise with AI…</button>{/if}
    {#if imageStoryId}<details class="node-image"><summary>Image</summary><div class="portrait-preview"><NodeImage storyId={imageStoryId} nodeId={node.id} {world} {revision} {historical}/></div>{#if editable}<button class="secondary-button" onclick={()=>onImage(node!.id)}>Generate / choose image…</button>{/if}</details>{/if}
    <label class="field">Name<input value={node.name} disabled={!editable} oninput={(e) => onEdit('name', e.currentTarget.value)} maxlength={200} /></label>
    <NodeFields {world} {node} disabled={!editable} onChange={onProperty}/>
    <details class="linked-section supplementary-notes"><summary>Additional notes</summary>
      <label class="field">Story notes<textarea value={node.description} disabled={!editable} oninput={(e) => onEdit('description', e.currentTarget.value)} rows={3} maxlength={16000}></textarea></label>
      <label class="field">Extra visual notes<textarea value={node.visualDescription} disabled={!editable} oninput={(e) => onEdit('visualDescription', e.currentTarget.value)} rows={3} maxlength={16000}></textarea><span class="field-help">Additional visible details; structured appearance and design fields also reach image preparation.</span></label>
    </details>
    {#if hasCapability(node,'core.holder')}
      <div class="linked-section"><h3>{hasCapability(node,'core.character')?'Worn & carried':'Contents'}</h3>
        {#each records.filter((r) => r.parentId === node!.id&&hasCapability(world.nodes.find(n=>n.id===r.id),'core.item')) as item}
          <button class="linked-record" onclick={() => onSelect(item.id)}><span>{item.name}</span><small>{item.relation === 'worn by' ? 'Worn' : item.relation} ↗</small></button>
        {:else}<p class="field-help">No items attached.</p>{/each}
      </div>
    {/if}
    <div class="linked-section"><div class="section-actions"><h3>Connections</h3>{#if editable}<button class="text-action" onclick={() => onNewLink(node!.id)}>＋ Attach / link</button>{/if}</div>
      {#each world.edges.filter((e) => e.sourceId === node!.id || e.targetId === node!.id) as edge (edge.id)}
        {@const outgoing = edge.sourceId === node.id}
        {@const other = world.nodes.find((n) => n.id === (outgoing ? edge.targetId : edge.sourceId))}
        {@const relation = world.relationTypes.find((d) => d.id === edge.typeId)}
        <div class="connection-row"><button class="linked-record" disabled={!other} onclick={() => {if(other)onSelect(other.id);}}><span>{outgoing ? '→' : '←'} {other?.name??'Missing node'}</span><small>{outgoing ? '' : 'incoming · '}{relation?.label}</small></button>
          {#if editable}<div class="connection-actions"><button class="text-action" onclick={() => onLink(edge.id)} aria-label={'Edit '+relation?.label+' connection with '+other?.name}>Edit / move</button><button class="text-action" onclick={()=>onAiLink(edge.id)} aria-label={'Revise '+relation?.label+' connection with AI'}>AI…</button><button class="text-action danger-text" onclick={() => onRemoveLink(edge.id)} aria-label={'Remove '+relation?.label+' connection with '+other?.name}>Detach</button></div>{/if}
        </div>
        {#if edge.description}<p class="field-help">{edge.description}</p>{/if}{#each relation?.fields??[] as field}{#if edge.properties?.[field.key]!==undefined&&edge.properties?.[field.key]!==null&&edge.properties?.[field.key]!==''}<p class="field-help">{field.label}: {valueLabel(edge.properties?.[field.key],world.nodes,field.kind)}</p>{/if}{/each}
      {:else}<p class="field-help">No connections.</p>{/each}
    </div>
    <details class="linked-section"><summary>AI context</summary><div class="section-actions">{#if editable}<button class="text-action" onclick={onContextOpen}>Manage / preview</button>{/if}</div><AiContextBindingEditor nodeId={node.id} {world} binding={world.aiContext?.find(b=>b.nodeId===node!.id)} disabled={!editable} onChange={onContext}/></details>
    <details class="linked-section"><summary>Manage extensions</summary>
      {#each Object.keys(node.components) as id}
        <div class="section-actions"><span>{world.componentTypes.find(d=>d.id===id)?.name??id}</span>{#if editable}<button class="text-action danger-text" onclick={()=>onRemoveCapability(node!.id,id)} aria-label={'Remove '+(world.componentTypes.find(d=>d.id===id)?.name??id)+' extension'}>Remove</button>{/if}</div>
      {/each}
      {#if editable}<label class="field">Add extension<select aria-label="Add an extension" value="" onchange={(e) => { if (e.currentTarget.value) { onAddCapability(e.currentTarget.value); e.currentTarget.value = ''; } }}><option value="">Choose extension…</option>{#each world.componentTypes.filter((d) => !node!.components[d.id]) as capability}<option value={capability.id}>{capability.name}</option>{/each}</select></label><div class="section-actions"><button class="text-action" onclick={onDefineCapability}>Define extension…</button><button class="text-action" onclick={()=>onRetype(node!.id)}>Change preset…</button></div>{/if}
    </details>
    {#if editable}<button class="remove-node-button" onclick={() => onRemoveNode(node!.id)}>Remove this node…</button>{/if}
    {#if historical}<p class="field-help">Historical snapshot · read only.</p>{/if}
  {:else}
    <div class="inspector-empty"><p>Select a node to inspect its details.</p></div>
  {/if}
</section>
