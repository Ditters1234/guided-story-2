<script lang="ts">
  import {untrack} from 'svelte';
  import type {World} from './model';
  import type {RelationDefinition_Serialize} from './bindings';
  import {commands,unwrap} from './native';
  import {fieldDefinitions,type FieldDraft} from './schema-fields';
  import FieldDefinitionsEditor from './FieldDefinitionsEditor.svelte';
  let {world,onCreate,onClose}:{world:World;onCreate:(definition:RelationDefinition_Serialize,source:string)=>void;onClose:()=>void}=$props();
  const source=untrack(()=>JSON.stringify(world)),base=JSON.parse(source) as World,id='custom.'+crypto.randomUUID();
  let dialog:HTMLDialogElement,name=$state(''),label=$state(''),from=$state(''),target=$state(''),physical=$state(false),single=$state(false),fields=$state<FieldDraft[]>([]),working=$state(false),error=$state('');
  let reviewed=$state<RelationDefinition_Serialize|null>(null),reviewInput=$state('');
  const input=$derived(JSON.stringify({name,label,from,target,physical,single,fields}));
  $effect(()=>{dialog?.showModal();});
  $effect(()=>{if(input!==reviewInput)reviewed=null;});
  async function review(event:SubmitEvent){
    event.preventDefault();working=true;error='';const signature=input;
    try{
      const definition:RelationDefinition_Serialize={id,name:name.trim(),label:label.trim(),sourceCapability:from||null,targetCapability:target||null,physicalPlacement:physical,singleTarget:physical||single,fields:fieldDefinitions(fields)};
      unwrap(await commands.validateWorld({...base,relationTypes:[...base.relationTypes,definition]}));
      if(input===signature){reviewed=definition;reviewInput=signature;}
    }catch(caught){error=caught instanceof Error?caught.message:String(caught);}finally{working=false;}
  }
  function apply(){if(reviewed&&input===reviewInput){try{onCreate($state.snapshot(reviewed),source);}catch(caught){error=caught instanceof Error?caught.message:String(caught);}}}
</script>
<dialog bind:this={dialog} class="project-dialog node-definition-dialog" onclose={onClose} oncancel={e=>{if(working)e.preventDefault();}} aria-labelledby="relationship-type-title">
  <form onsubmit={review}>
    <header class="settings-heading"><h2 id="relationship-type-title">Define relationship</h2><button type="button" class="icon-button" aria-label="Close relationship definition" disabled={working} onclick={onClose}>×</button></header>
    <div class="node-definition-body"><fieldset disabled={working}>
      <label class="field">Relationship name<input bind:value={name} required maxlength={200}/></label>
      <label class="field">Label on map<input bind:value={label} required maxlength={200}/></label>
      <label class="field">Source needs<select bind:value={from}><option value="">Any node</option>{#each base.componentTypes as c}<option value={c.id}>{c.name}</option>{/each}</select></label>
      <label class="field">Target needs<select bind:value={target}><option value="">Any node</option>{#each base.componentTypes as c}<option value={c.id}>{c.name}</option>{/each}</select></label>
      <label class="check-field"><input type="checkbox" bind:checked={physical}/><span>Physical placement<small>Replaces the source’s previous location or holder, with the usual containment checks.</small></span></label>
      <label class="check-field"><input type="checkbox" checked={physical||single} disabled={physical} onchange={e=>{single=e.currentTarget.checked;}}/><span>One target per source</span></label>
      <FieldDefinitionsEditor world={base} bind:fields/>
      {#if error}<p role="alert" class="inline-error">{error}</p>{/if}
      {#if reviewed}<div class="change-review">Create this relationship type with {fields.length} optional detail fields.</div>{/if}
    </fieldset></div>
    <footer class="settings-footer"><button type="button" class="quiet-button" disabled={working} onclick={onClose}>Cancel</button>{#if reviewed}<button type="button" class="primary-button" onclick={apply}>Apply definition</button>{:else}<button type="submit" class="primary-button" disabled={working||!name.trim()||!label.trim()}>{working?'Checking…':'Review definition'}</button>{/if}</footer>
  </form>
</dialog>
