<script lang="ts">
  import type {FieldUse} from './bindings';
  import type {World} from './model';
  import type {FieldDraft} from './schema-fields';
  let {world,fields=$bindable([])}:{world:World;fields:FieldDraft[]}=$props();
  let fieldNumber=0;
  function addField(){fields=[...fields,{key:'field'+fieldNumber++,label:'',kind:'longText',help:'',uses:['story'],stability:'stable',choices:'',referenceCapability:'',coveredBy:''}];}
  function setUse(index:number,use:FieldUse,checked:boolean){fields=fields.map((f,i)=>i!==index?f:{...f,uses:checked?[...f.uses,use]:f.uses.filter(v=>v!==use)});}
</script>
      {#each fields as field,index (field.key)}
        <section class="definition-field"><div class="section-actions"><strong>Field {index+1}</strong><button type="button" class="text-action danger-text" onclick={()=>{fields=fields.filter(f=>f.key!==field.key);}}>Remove</button></div>
          <div class="definition-field-row"><label class="field">Label<input bind:value={field.label} required maxlength={200}/></label><label class="field">Input<select bind:value={field.kind}><option value="text">Short text</option><option value="longText">Long text</option><option value="number">Number</option><option value="boolean">Yes / no</option><option value="choice">Choice</option><option value="nodeRef">Node reference</option><option value="nodeRefs">Node references</option></select></label></div>
          <label class="field">Help<input bind:value={field.help} maxlength={1000}/></label>
          {#if field.kind==='choice'}<label class="field">Choices<input bind:value={field.choices} placeholder="open, active, resolved" maxlength={4000} required/></label>{/if}
          {#if ['nodeRef','nodeRefs'].includes(field.kind)}<label class="field">Reference needs<select bind:value={field.referenceCapability}><option value="">Any node</option>{#each world.componentTypes as c}<option value={c.id}>{c.name}</option>{/each}</select></label>{/if}
          <div class="definition-task-uses"><span>Use for</span>{#each [{id:'story',label:'Story'},{id:'visual',label:'Images'},{id:'planning',label:'Author planning'}] as use}<label><input type="checkbox" checked={field.uses.includes(use.id as FieldUse)} disabled={field.uses.length===1&&field.uses.includes(use.id as FieldUse)} onchange={e=>setUse(index,use.id as FieldUse,e.currentTarget.checked)}/>{use.label}</label>{/each}</div>
          <label class="field">Information<select bind:value={field.stability}><option value="stable">Lasting trait</option><option value="state">Changing state</option></select></label>
          {#if field.uses.includes('visual')}<label class="field">Hide when covered by garment regions<input bind:value={field.coveredBy} placeholder="Optional: torso, arms" maxlength={1000}/></label>{/if}
        </section>
      {/each}
      <button type="button" class="secondary-button" disabled={fields.length>=40} onclick={addField}>＋ Add field</button>
