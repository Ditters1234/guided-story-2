<script lang="ts">
  import {untrack} from 'svelte';
  import type {ComponentDefinition_Serialize} from './bindings';
  import type {World} from './model';
  import {makeNode} from './model';
  import {defaultProperties} from './node-fields';
  import {commands,unwrap} from './native';
  import {fieldDefinitions,type FieldDraft} from './schema-fields';
  import FieldDefinitionsEditor from './FieldDefinitionsEditor.svelte';
  let {world,attachToId,onApply,onClose}:{world:World;attachToId?:string;onApply:(next:World,source:string,selectedId:string)=>void;onClose:()=>void}=$props();
  const source=untrack(()=>JSON.stringify(world)),base=JSON.parse(source) as World;
  const extensionId='custom.'+crypto.randomUUID(),typeId='custom.'+crypto.randomUUID(),newId=crypto.randomUUID();
  let dialog:HTMLDialogElement,name=$state(''),description=$state(''),components=$state<string[]>([]),fields=$state<FieldDraft[]>([]),error=$state(''),working=$state(false);
  let reviewed=$state<World|null>(null),reviewInput=$state('');
  const input=$derived(JSON.stringify({name,description,components,fields}));
  $effect(()=>{dialog?.showModal();});
  $effect(()=>{if(input!==reviewInput)reviewed=null;});
  async function review(event:SubmitEvent){
    event.preventDefault();error='';working=true;
    const signature=input;
    try{
      if(!name.trim()||fields.some(f=>!f.label.trim()))throw Error('Give the definition and each field a name.');
      const definitions=fieldDefinitions(fields);
      const extension:ComponentDefinition_Serialize={id:extensionId,name:name.trim()+(attachToId?'':' details'),description:description.trim(),fields:definitions};
      let next=structuredClone(base);
      if(attachToId||fields.length)next.componentTypes.push(extension);
      if(attachToId){
        const node=next.nodes.find(n=>n.id===attachToId);if(!node)throw Error('The selected node is no longer available.');
        node.components[extensionId]=defaultProperties(extension);
      }else{
        next.nodeTypes.push({id:typeId,name:name.trim(),description:description.trim(),components:[...components,...(fields.length?[extensionId]:[])]});
        next.nodes.push(makeNode(next,typeId,'New '+name.trim().toLowerCase(),newId));
      }
      next=unwrap(await commands.validateWorld(next));
      if(input===signature){reviewInput=signature;reviewed=next;}
    }catch(caught){error=caught instanceof Error?caught.message:String(caught);}finally{working=false;}
  }
  function apply(){if(reviewed&&reviewInput===input){try{onApply($state.snapshot(reviewed),source,attachToId??newId);}catch(caught){error=caught instanceof Error?caught.message:String(caught);}}}
</script>
<dialog bind:this={dialog} class="project-dialog node-definition-dialog" onclose={onClose} oncancel={e=>{if(working)e.preventDefault();}} aria-labelledby="node-definition-title">
  <form onsubmit={review}>
    <header class="settings-heading"><h2 id="node-definition-title">{attachToId?'Define extension':'Define node preset'}</h2><button type="button" class="icon-button" aria-label="Close definition" disabled={working} onclick={onClose}>×</button></header>
    <div class="node-definition-body"><fieldset disabled={working}>
      <label class="field">Name<input bind:value={name} required maxlength={120}/></label>
      <label class="field">Purpose / help<textarea bind:value={description} rows={2} maxlength={1000}></textarea></label>
      {#if !attachToId}
        <label class="field">Start from a preset<select value="" onchange={e=>{components=[...(base.nodeTypes.find(t=>t.id===e.currentTarget.value)?.components??[])];}}><option value="">Choose starting extensions…</option>{#each base.nodeTypes as type}<option value={type.id}>{type.name}</option>{/each}</select></label>
        <details><summary>Included extensions ({components.length})</summary><div class="definition-capabilities">{#each base.componentTypes as capability}<label><input type="checkbox" bind:group={components} value={capability.id}/>{capability.name}</label>{/each}</div></details>
      {/if}
      <FieldDefinitionsEditor world={base} bind:fields/>
      {#if error}<p class="inline-error" role="alert">{error}</p>{/if}
      {#if reviewed}<div class="change-review"><p>{attachToId?'Attach this extension to the selected node.':'Create this preset and one unplaced node.'} {fields.length} optional fields; unspecified values stay empty. Existing definitions and nodes are retained.</p></div>{/if}
    </fieldset></div>
    <footer class="settings-footer"><button type="button" class="quiet-button" disabled={working} onclick={onClose}>Cancel</button>{#if reviewed}<button type="button" class="primary-button" onclick={apply}>Apply definition</button>{:else}<button type="submit" class="primary-button" disabled={working||!name.trim()}>{working?'Checking…':'Review definition'}</button>{/if}</footer>
  </form>
</dialog>
