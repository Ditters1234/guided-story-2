<script lang="ts">
  import {untrack} from 'svelte';
  import type {ComponentDefinition_Serialize,FieldDefinition_Serialize,FieldKind,FieldUse} from './bindings';
  import type {World} from './model';
  import {makeNode} from './model';
  import {defaultProperties} from './node-fields';
  import {commands,unwrap} from './native';
  type FieldDraft={key:string;label:string;kind:FieldKind;help:string;uses:FieldUse[];stability:'stable'|'state';choices:string;referenceCapability:string;coveredBy:string};
  let {world,attachToId,onApply,onClose}:{world:World;attachToId?:string;onApply:(next:World,source:string,selectedId:string)=>void;onClose:()=>void}=$props();
  const source=untrack(()=>JSON.stringify(world)),base=JSON.parse(source) as World;
  const extensionId='custom.'+crypto.randomUUID(),typeId='custom.'+crypto.randomUUID(),newId=crypto.randomUUID();
  let dialog:HTMLDialogElement,name=$state(''),description=$state(''),components=$state<string[]>([]),fields=$state<FieldDraft[]>([]),error=$state(''),working=$state(false);
  let reviewed=$state<World|null>(null),reviewInput=$state('');
  const input=$derived(JSON.stringify({name,description,components,fields}));
  $effect(()=>{dialog?.showModal();});
  $effect(()=>{if(input!==reviewInput)reviewed=null;});
  let fieldNumber=0;
  function addField(){fields=[...fields,{key:'field'+fieldNumber++,label:'',kind:'longText',help:'',uses:['story'],stability:'stable',choices:'',referenceCapability:'',coveredBy:''}];}
  function setUse(index:number,use:FieldUse,checked:boolean){fields=fields.map((f,i)=>i!==index?f:{...f,uses:checked?[...f.uses,use]:f.uses.filter(v=>v!==use)});}
  async function review(event:SubmitEvent){
    event.preventDefault();error='';working=true;
    const signature=input;
    try{
      if(!name.trim()||fields.some(f=>!f.label.trim()))throw Error('Give the definition and each field a name.');
      const definitions:FieldDefinition_Serialize[]=fields.map(f=>({key:f.key,label:f.label.trim(),kind:f.kind,required:false,defaultValue:null,metadata:{help:f.help.trim()||undefined,uses:[...f.uses],stability:f.stability,choices:f.kind==='choice'?f.choices.split(',').map(v=>v.trim()).filter(Boolean):[],referenceCapability:['nodeRef','nodeRefs'].includes(f.kind)?f.referenceCapability||undefined:undefined,coveredBy:f.uses.includes('visual')?f.coveredBy.split(',').map(v=>v.trim()).filter(Boolean):[]}}));
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
      {#each fields as field,index (field.key)}
        <section class="definition-field"><div class="section-actions"><strong>Field {index+1}</strong><button type="button" class="text-action danger-text" onclick={()=>{fields=fields.filter(f=>f.key!==field.key);}}>Remove</button></div>
          <div class="definition-field-row"><label class="field">Label<input bind:value={field.label} required maxlength={200}/></label><label class="field">Input<select bind:value={field.kind}><option value="text">Short text</option><option value="longText">Long text</option><option value="number">Number</option><option value="boolean">Yes / no</option><option value="choice">Choice</option><option value="nodeRef">Node reference</option><option value="nodeRefs">Node references</option></select></label></div>
          <label class="field">Help<input bind:value={field.help} maxlength={1000}/></label>
          {#if field.kind==='choice'}<label class="field">Choices<input bind:value={field.choices} placeholder="open, active, resolved" maxlength={4000} required/></label>{/if}
          {#if ['nodeRef','nodeRefs'].includes(field.kind)}<label class="field">Reference needs<select bind:value={field.referenceCapability}><option value="">Any node</option>{#each base.componentTypes as c}<option value={c.id}>{c.name}</option>{/each}</select></label>{/if}
          <div class="definition-task-uses"><span>Use for</span>{#each [{id:'story',label:'Story'},{id:'visual',label:'Images'},{id:'planning',label:'Author planning'}] as use}<label><input type="checkbox" checked={field.uses.includes(use.id as FieldUse)} disabled={field.uses.length===1&&field.uses.includes(use.id as FieldUse)} onchange={e=>setUse(index,use.id as FieldUse,e.currentTarget.checked)}/>{use.label}</label>{/each}</div>
          <label class="field">Information<select bind:value={field.stability}><option value="stable">Lasting trait</option><option value="state">Changing state</option></select></label>
          {#if field.uses.includes('visual')}<label class="field">Hide when covered by garment regions<input bind:value={field.coveredBy} placeholder="Optional: torso, arms" maxlength={1000}/></label>{/if}
        </section>
      {/each}
      <button type="button" class="secondary-button" disabled={fields.length>=40} onclick={addField}>＋ Add field</button>
      {#if error}<p class="inline-error" role="alert">{error}</p>{/if}
      {#if reviewed}<div class="change-review"><p>{attachToId?'Attach this extension to the selected node.':'Create this preset and one unplaced node.'} {fields.length} optional fields; unspecified values stay empty. Existing definitions and nodes are retained.</p></div>{/if}
    </fieldset></div>
    <footer class="settings-footer"><button type="button" class="quiet-button" disabled={working} onclick={onClose}>Cancel</button>{#if reviewed}<button type="button" class="primary-button" onclick={apply}>Apply definition</button>{:else}<button type="submit" class="primary-button" disabled={working||!name.trim()}>{working?'Checking…':'Review definition'}</button>{/if}</footer>
  </form>
</dialog>
