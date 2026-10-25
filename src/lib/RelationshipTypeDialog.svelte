<script lang="ts">
  import type { World } from './model';
  import type { RelationDefinition_Serialize } from './bindings';
  let {world,onCreate,onClose}: {world:World;onCreate:(definition:RelationDefinition_Serialize)=>void;onClose:()=>void}=$props();
  let dialog: HTMLDialogElement;
  let name=$state(''), label=$state(''), source=$state(''), target=$state('');
  let physical=$state(false), single=$state(false);
  $effect(()=>{dialog?.showModal();});
  function create(event:SubmitEvent) {
    event.preventDefault();
    onCreate({id:'custom.'+crypto.randomUUID(),name:name.trim(),label:label.trim(),sourceCapability:source||null,targetCapability:target||null,physicalPlacement:physical,singleTarget:physical||single});
  }
</script>
<dialog bind:this={dialog} class="project-dialog edit-dialog" onclose={onClose} oncancel={onClose} aria-labelledby="relationship-type-title">
  <form onsubmit={create}>
    <span class="eyebrow">YOUR WORLD, YOUR TOOLKIT</span><h2 id="relationship-type-title">Define a relationship.</h2>
    <p>Use capabilities to constrain connections. A physical relationship participates in the same one-holder and containment-loop checks as built-in placements.</p>
    <label class="field">Relationship name<input bind:value={name} required maxlength={200} placeholder="Member of, mounted on…" /></label>
    <label class="field">Label on map<input bind:value={label} required maxlength={200} placeholder="member of" /></label>
    <label class="field">Source needs<select bind:value={source}><option value="">Any node</option>{#each world.componentTypes as capability}<option value={capability.id}>{capability.name}</option>{/each}</select></label>
    <label class="field">Target needs<select bind:value={target}><option value="">Any node</option>{#each world.componentTypes as capability}<option value={capability.id}>{capability.name}</option>{/each}</select></label>
    <label class="check-field"><input type="checkbox" bind:checked={physical} /><span>Physical placement<small>Moving this link replaces the source’s previous location or holder.</small></span></label>
    <label class="check-field"><input type="checkbox" checked={physical||single} disabled={physical} onchange={(e)=>{single=e.currentTarget.checked;}} /><span>One target per source<small>A new connection of this type replaces its previous target.</small></span></label>
    <div class="dialog-actions"><button type="button" class="quiet-button" onclick={onClose}>Cancel</button><button type="submit" class="primary-button" disabled={!name.trim()||!label.trim()}>Create relationship</button></div>
  </form>
</dialog>
