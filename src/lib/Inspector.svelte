<script lang="ts">
  import type { WorldRecord } from './graph';
  import type { World } from './model';
  import PropertyInput from './PropertyInput.svelte';
  import NodeImage from './NodeImage.svelte';
  let { record, records, world, editable, historical = false, imageStoryId, revision, onImage, onEdit, onSelect, onProperty, onAddCapability, onRemoveNode, onRemoveCapability, onLink, onNewLink, onRemoveLink, onAi }: {
    record: WorldRecord | undefined; records: WorldRecord[]; world: World; editable: boolean; historical?: boolean;
    imageStoryId:string|null;revision:number;onImage:(id:string)=>void;
    onEdit: (field: 'name' | 'description' | 'visualDescription', value: string) => void;
    onSelect: (id: string) => void;
    onProperty: (componentId: string, key: string, value: string | number | boolean | string[] | null) => void;
    onAddCapability: (componentId: string) => void;
    onRemoveNode: (id: string) => void; onRemoveCapability: (id: string, capability: string) => void;
    onLink: (id: string) => void; onNewLink: (id: string) => void; onRemoveLink: (id: string) => void;
    onAi: (id:string)=>void;
  } = $props();
  const node = $derived(world.nodes.find((n) => n.id === record?.id));
</script>

<section class="inspector">
  <div class="panel-heading"><span>INSPECTOR</span><span class="tag">{record?.kind ?? 'World'}</span></div>
  {#if record}
    <div class="record-heading"><span class="eyebrow">{record.detail}</span><h2>{record.name || 'Unnamed record'}</h2>
      {#if record.parentId}<button class="parent-link" onclick={() => onSelect(record!.parentId!)}>{record.relation} {records.find((r) => r.id === record!.parentId)?.name ?? 'Unknown record'} ↗</button>{/if}
    </div>
    {#if editable}<button class="secondary-button record-ai-action" onclick={()=>onAi(record!.id)}>Revise / reroll with AI…</button>{/if}
    {#if imageStoryId}<details class="node-image" open={record.kind==='character'}><summary>Image</summary><div class="portrait-preview"><NodeImage storyId={imageStoryId} nodeId={record.id} {world} {revision} {historical}/></div>{#if editable}<button class="secondary-button" onclick={()=>onImage(record!.id)}>Generate / choose image…</button>{/if}</details>{/if}
    <label class="field">Name<input value={record.name} disabled={!editable} oninput={(e) => onEdit('name', e.currentTarget.value)} maxlength={200} /></label>
    <label class="field">Story notes<textarea value={record.description} disabled={!editable} oninput={(e) => onEdit('description', e.currentTarget.value)} rows={5} maxlength={16000}></textarea></label>
    {#if record.kind !== 'thread'}
      <label class="field">Visible description<textarea value={record.visualDescription} disabled={!editable} oninput={(e) => onEdit('visualDescription', e.currentTarget.value)} rows={4} maxlength={16000}></textarea><span class="field-help">Appearance and design details for future illustrations.</span></label>
    {/if}
    {#if record.kind === 'character'}
      <div class="linked-section"><h3>Worn & carried</h3>
        {#each records.filter((r) => r.kind === 'item' && r.parentId === record!.id) as item}
          <button class="linked-record" onclick={() => onSelect(item.id)}><span>{item.name}</span><small>{item.relation === 'worn by' ? 'Worn' : 'Carried'} ↗</small></button>
        {:else}<p class="field-help">No belongings yet.</p>{/each}
      </div>
    {/if}
    {#if node}
      <div class="linked-section"><div class="section-actions"><h3>Connections</h3>{#if editable}<button class="text-action" onclick={() => onNewLink(node!.id)}>＋ Attach / link</button>{/if}</div>
        <p class="field-help">All incoming and outgoing links, including connections outside the visible map.</p>
        {#each world.edges.filter((e) => e.sourceId === node!.id || e.targetId === node!.id) as edge (edge.id)}
          {@const outgoing = edge.sourceId === node.id}
          {@const other = world.nodes.find((n) => n.id === (outgoing ? edge.targetId : edge.sourceId))}
          {@const relation = world.relationTypes.find((d) => d.id === edge.typeId)}
          <div class="connection-row"><button class="linked-record" onclick={() => onSelect(other!.id)}><span>{outgoing ? '→' : '←'} {other?.name}</span><small>{outgoing ? '' : 'incoming · '}{relation?.label}</small></button>
            {#if editable}<div class="connection-actions"><button class="text-action" onclick={() => onLink(edge.id)} aria-label={`Edit ${relation?.label} connection with ${other?.name}`}>Edit / move</button><button class="text-action danger-text" onclick={() => onRemoveLink(edge.id)} aria-label={`Remove ${relation?.label} connection with ${other?.name}`}>Detach</button></div>{/if}
          </div>
        {:else}<p class="field-help">No connections. Attach this node to another node, or leave it unplaced.</p>{/each}
      </div>
      <div class="linked-section"><h3>Capabilities</h3><p class="field-help">Combine reusable behaviors on any world node.</p>
        {#each Object.entries(node.components) as [componentId, properties]}
          {@const definition = world.componentTypes.find((d) => d.id === componentId)}
          <section class="capability"><div class="section-actions"><strong>{definition?.name ?? componentId}</strong>{#if editable}<button class="text-action danger-text" onclick={() => onRemoveCapability(node!.id, componentId)} aria-label={`Remove ${definition?.name ?? componentId} capability`}>Remove</button>{/if}</div>
            {#if !definition?.fields.length}<span class="field-help">{definition?.description}</span>{/if}
            {#each definition?.fields ?? [] as field}
              <PropertyInput {field} value={properties[field.key]??null} nodes={world.nodes} disabled={!editable} onChange={(value)=>onProperty(componentId,field.key,value)} />
            {/each}
          </section>
        {/each}
        {#if editable}<select aria-label="Add a capability" value="" onchange={(e) => { if (e.currentTarget.value) { onAddCapability(e.currentTarget.value); e.currentTarget.value = ''; } }}><option value="">＋ Add capability</option>{#each world.componentTypes.filter((d) => !node!.components[d.id]) as capability}<option value={capability.id}>{capability.name}</option>{/each}</select>{/if}
      </div>
      {#if editable}<button class="remove-node-button" onclick={() => onRemoveNode(node!.id)}>Remove this node…</button>{/if}
    {/if}
    <div class="scope-note">{historical ? 'Historical world snapshot · read only.' : editable ? 'Valid edits save automatically. Historical snapshots are preserved.' : 'This is a fictional example. Create a project to save and edit it.'}</div>
  {:else}
    <div class="inspector-empty"><span class="empty-symbol">◇</span><h2>A closer look</h2><p>Select a place, character, item or thread to inspect its details.</p></div>
  {/if}
</section>
