<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { World } from './model';
  import type { WorldEdit, WorldEditPreview_Serialize, WorldNode } from './bindings';
  import { commands, unwrap } from './native';
  let {world, edit, onApply, onClose}: {
    world: World; edit: WorldEdit;
    onApply: (world: World, source: string) => void; onClose: () => void;
  } = $props();
  const source = untrack(() => JSON.stringify(world));
  let candidate = $state<WorldEdit>(untrack(() => structuredClone($state.snapshot(edit))));
  let preview = $state<WorldEditPreview_Serialize | null>(null);
  let previewInput = $state('');
  let error = $state('');
  let working = $state(false);
  let dialog: HTMLDialogElement;
  let review = $state<HTMLDivElement>();
  const link = $derived(candidate.kind === 'putLink' ? candidate.link : null);
  const definition = $derived(world.relationTypes.find((d) => d.id === link?.typeId));
  const title = $derived(candidate.kind === 'putLink' ? (world.edges.some((e) => e.id === candidate.link.id) ? 'Edit connection' : 'Connect world nodes') : candidate.kind === 'removeNode' ? 'Remove node' : candidate.kind === 'removeCapability' ? 'Remove extension' : candidate.kind==='addCapability'?'Add extension':candidate.kind==='retypeNode'?'Change preset':'Remove connection');
  function supports(node: WorldNode, capability: string | null | undefined) {
    return !capability || !!node.components[capability] || (capability === 'core.holder' && !!node.components['core.container']);
  }
  $effect(() => { dialog?.showModal(); });
  $effect(() => { const input = JSON.stringify(candidate); if (input !== previewInput) preview = null; });
  async function inspect(event: SubmitEvent) {
    event.preventDefault(); working = true; error = ''; preview = null;
    const input = JSON.stringify(candidate);
    try {
      const result = unwrap(await commands.previewWorldEdit(JSON.parse(source), JSON.parse(input)));
      if (JSON.stringify(candidate) !== input) return;
      previewInput = input; preview = result;
      await tick(); review?.focus();
    } catch (caught) { error = caught instanceof Error ? caught.message : String(caught); }
    finally { working = false; }
  }
  function apply() {
    if (preview && previewInput === JSON.stringify(candidate)) onApply(preview.world, source);
  }
</script>

<dialog bind:this={dialog} class="project-dialog edit-dialog" onclose={onClose} oncancel={onClose} aria-labelledby="edit-world-title">
  <form onsubmit={inspect}>
    <span class="eyebrow">WORLD STRUCTURE</span><h2 id="edit-world-title">{title}</h2>
    {#if link}
      <p>Read this as: source → relationship → target. Moving a placement keeps the node’s own contents with it.</p>
      <fieldset disabled={working}>
        <label class="field">Source node<select bind:value={link.sourceId} required><option value="">Choose source…</option>{#each world.nodes.filter((n) => supports(n, definition?.sourceCapability)) as node}<option value={node.id}>{node.name}</option>{/each}</select></label>
        <label class="field">Relationship<select bind:value={link.typeId}>{#each world.relationTypes as relation}<option value={relation.id}>{relation.name}</option>{/each}</select><span class="field-help">{definition?.physicalPlacement ? 'Physical placement: replaces any previous location or holder.' : definition?.singleTarget ? 'One target per source for this relationship.' : 'Adds a connection without changing physical placement.'}</span></label>
        <label class="field">Target node<select bind:value={link.targetId} required><option value="">Choose target…</option>{#each world.nodes.filter((n) => n.id !== link.sourceId && supports(n, definition?.targetCapability)) as node}<option value={node.id}>{node.name}</option>{/each}</select></label>
        <label class="field">Connection notes<textarea bind:value={link.description} rows={2} maxlength={16000}></textarea></label>
      </fieldset>
    {:else if candidate.kind==='addCapability'}
      <p>Add <strong>{world.componentTypes.find(d=>candidate.kind==='addCapability'&&d.id===candidate.capability)?.name}</strong> to <strong>{world.nodes.find(n=>n.id===candidate.id)?.name}</strong>.</p>
      <p class="field-help">{world.componentTypes.find(d=>candidate.kind==='addCapability'&&d.id===candidate.capability)?.description}</p>
    {:else if candidate.kind==='retypeNode'}
      <label class="field">Preset<select bind:value={candidate.typeId}>{#each world.nodeTypes as type}<option value={type.id}>{type.name}</option>{/each}</select></label><p>Existing extensions, values and connections are retained. Missing extensions from this preset are added with their declared defaults.</p>
    {:else if candidate.kind === 'removeNode'}
      <p>Remove <strong>{world.nodes.find((n) => n.id === candidate.id)?.name}</strong> from the current world. Contents remain as unplaced nodes. Connections and typed references to this node are cleared. Previous saved worlds stay intact.</p>
    {:else if candidate.kind === 'removeCapability'}
      <p>Remove this capability and its field values. If existing connections need it, move or remove those connections first.</p>
    {:else}
      <p>Detach this connection. Removing physical placement also ends wearing; removing only a wear assignment keeps possession.</p>
    {/if}
    {#if error}<p role="alert" class="inline-error">{error}</p>{/if}
    {#if preview}
      <div class="change-review" bind:this={review} tabindex="-1"><h3>Changes to apply</h3><ul>{#each preview.changes as change}<li>{change}</li>{:else}<li>No changes.</li>{/each}</ul></div>
    {/if}
    <div class="dialog-actions"><button type="button" class="quiet-button" onclick={onClose} disabled={working}>Cancel</button>
      {#if preview}<button type="button" class="primary-button" onclick={apply} disabled={!preview.changes.length}>Apply changes</button>
      {:else}<button type="submit" class="primary-button" disabled={working || (!!link && (!link.sourceId || !link.targetId || link.sourceId === link.targetId))}>{working ? 'Checking…' : 'Review changes'}</button>{/if}
    </div>
  </form>
</dialog>
