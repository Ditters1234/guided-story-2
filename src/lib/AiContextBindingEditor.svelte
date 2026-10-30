<script lang="ts">
  import type {AiContextBinding,AiContextField,TextPromptTask} from './bindings';
  import {aiContextTasks,defaultContextBinding} from './ai-context';
  import type {World} from './model';
  let {nodeId,binding,world,disabled=false,onChange}:{nodeId:string;binding?:AiContextBinding;world?:World;disabled?:boolean;onChange:(value:AiContextBinding)=>void}=$props();
  function edit(patch:Partial<AiContextBinding>){onChange({...binding??defaultContextBinding(nodeId,0,world),...patch});}
  function task(id:TextPromptTask){if(binding)edit({tasks:binding.tasks.includes(id)?binding.tasks.filter(t=>t!==id):[...binding.tasks,id]});}
  function field(id:AiContextField){if(binding)edit({fields:binding.fields.includes(id)?binding.fields.filter(f=>f!==id):[...binding.fields,id]});}
</script>
<fieldset class="ai-context-fields" {disabled}>
  <label class="check-field"><input type="checkbox" checked={binding?.enabled??false} onchange={e=>edit({enabled:e.currentTarget.checked})}/><span>Include this node in AI context</span></label>
  {#if binding}
    <label class="field">Include as<select aria-label="AI context role" value={binding.role} onchange={e=>edit({role:e.currentTarget.value as AiContextBinding['role']})}><option value="reference">Reference facts</option><option value="instructions">Instructions</option></select></label>
    <label class="field">Order<input aria-label="AI context order" type="number" min={-10000} max={10000} step={1} value={binding.order} onchange={e=>edit({order:Number(e.currentTarget.value)})}/><span class="field-help">Lower numbers appear first.</span></label>
    <div class="section-actions"><strong>Tools</strong><button class="text-action" type="button" onclick={()=>edit({tasks:aiContextTasks.map(t=>t.id),imagePreparation:true})}>Select all</button></div>
    <div class="ai-context-checks"><label><input type="checkbox" checked={binding.imagePreparation??false} onchange={e=>edit({imagePreparation:e.currentTarget.checked})}/>Image generation (shared with prompt assistance)</label>{#each aiContextTasks as t}<label><input type="checkbox" checked={binding.tasks.includes(t.id)} onchange={()=>task(t.id)}/>{t.label}</label>{/each}</div>
    {#if !binding.tasks.length&&!binding.imagePreparation}<p class="field-help">No tools selected; this node will not be sent.</p>{/if}
    <strong>Content to include</strong><div class="ai-context-checks">{#each [{id:'description' as const,label:'Story notes'},{id:'visualDescription' as const,label:'Visible description'},{id:'components' as const,label:'Capability fields'}] as f}<label><input type="checkbox" checked={binding.fields.includes(f.id)} onchange={()=>field(f.id)}/>{f.label}</label>{/each}</div>
  {/if}
</fieldset>
