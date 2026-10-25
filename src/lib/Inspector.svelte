<script lang="ts">
  import type { WorldRecord } from './graph';
  import type { World } from './model';
  let { record, records, world, editable, onEdit, onSelect, onProperty, onAddCapability }: {
    record: WorldRecord | undefined; records: WorldRecord[]; world: World; editable: boolean;
    onEdit: (field: 'name' | 'description' | 'visualDescription', value: string) => void;
    onSelect: (id: string) => void;
    onProperty: (componentId: string, key: string, value: string | number | boolean | string[] | null) => void;
    onAddCapability: (componentId: string) => void;
  } = $props();
  const node = $derived(world.nodes.find((n) => n.id === record?.id));
  let portrait = $state<string | null>(null);
  let previewError = $state('');
  let previousId = $state('');
  $effect(() => { if (record?.id !== previousId) { previousId = record?.id ?? ''; portrait = null; previewError = ''; } });
  function preview(event: Event) {
    const file = (event.target as HTMLInputElement).files?.[0];
    if (!file) return;
    if (file.size > 8 * 1024 * 1024 || !['image/png', 'image/jpeg', 'image/webp'].includes(file.type)) { previewError = 'Choose a PNG, JPEG or WebP up to 8 MB.'; return; }
    const reader = new FileReader();
    reader.onload = () => { portrait = String(reader.result); previewError = ''; };
    reader.onerror = () => { previewError = 'The image could not be opened.'; };
    reader.readAsDataURL(file);
  }
</script>

<section class="inspector">
  <div class="panel-heading"><span>INSPECTOR</span><span class="tag">{record?.kind ?? 'World'}</span></div>
  {#if record}
    <div class="record-heading"><span class="eyebrow">{record.detail}</span><h2>{record.name || 'Unnamed record'}</h2>
      {#if record.parentId}<button class="parent-link" onclick={() => onSelect(record!.parentId!)}>{record.relation} {records.find((r) => r.id === record!.parentId)?.name ?? 'Unknown record'} ↗</button>{/if}
    </div>
    {#if record.kind === 'character'}
      <div class="portrait-preview">
        {#if portrait}<img src={portrait} alt={`Preview for ${record.name}`} />{:else}<div class="portrait-placeholder"><span>{record.name.split(' ').map((s) => s[0]).slice(0, 2).join('')}</span><small>No portrait yet</small></div>{/if}
        <label class="preview-button">Preview an image<input type="file" accept="image/png,image/jpeg,image/webp" onchange={preview} /></label>
      </div>
      <p class="field-help">Image preview only. Saved portraits and generation arrive in the media milestone.</p>
      {#if previewError}<p class="inline-error">{previewError}</p>{/if}
    {/if}
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
      <div class="linked-section"><h3>Capabilities</h3><p class="field-help">Combine reusable behaviors on any world node.</p>
        {#each Object.entries(node.components) as [componentId, properties]}
          {@const definition = world.componentTypes.find((d) => d.id === componentId)}
          <section class="capability"><strong>{definition?.name ?? componentId}</strong>
            {#if !definition?.fields.length}<span class="field-help">{definition?.description}</span>{/if}
            {#each definition?.fields ?? [] as field}
              <label class="field compact">{field.label}
                {#if field.kind === 'boolean'}<input type="checkbox" checked={Boolean(properties[field.key])} disabled={!editable} onchange={(e) => onProperty(componentId, field.key, e.currentTarget.checked)} />
                {:else if field.kind === 'number'}<input type="number" value={Number(properties[field.key])} disabled={!editable} oninput={(e) => onProperty(componentId, field.key, e.currentTarget.value === '' ? null : Number(e.currentTarget.value))} />
                {:else if field.kind === 'nodeRef'}<select value={String(properties[field.key] ?? '')} disabled={!editable} onchange={(e) => onProperty(componentId, field.key, e.currentTarget.value || null)}><option value="">Unspecified</option>{#each world.nodes as target}<option value={target.id}>{target.name}</option>{/each}</select>
                {:else if field.kind === 'nodeRefs'}<select multiple value={Array.isArray(properties[field.key]) ? properties[field.key] as string[] : []} disabled={!editable} onchange={(e) => onProperty(componentId, field.key, Array.from(e.currentTarget.selectedOptions).map((o) => o.value))}>{#each world.nodes as target}<option value={target.id}>{target.name}</option>{/each}</select>
                {:else}<input value={String(properties[field.key] ?? '')} disabled={!editable} oninput={(e) => onProperty(componentId, field.key, e.currentTarget.value)} />{/if}
              </label>
            {/each}
          </section>
        {/each}
        {#if editable}<select aria-label="Add a capability" value="" onchange={(e) => { if (e.currentTarget.value) { onAddCapability(e.currentTarget.value); e.currentTarget.value = ''; } }}><option value="">＋ Add capability</option>{#each world.componentTypes.filter((d) => !node!.components[d.id]) as capability}<option value={capability.id}>{capability.name}</option>{/each}</select>{/if}
      </div>
    {/if}
    <div class="scope-note">{editable ? 'Valid edits save automatically. Historical snapshots are preserved.' : 'This is a fictional example. Create a project to save and edit it.'}</div>
  {:else}
    <div class="inspector-empty"><span class="empty-symbol">◇</span><h2>A closer look</h2><p>Select a place, character, item or thread to inspect its details.</p></div>
  {/if}
</section>
