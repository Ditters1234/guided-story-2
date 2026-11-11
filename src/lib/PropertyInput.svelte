<script lang="ts">
  import type { FieldDefinition,PropertyValue } from './bindings';
  import {referenceTargets,type FieldTarget} from './node-fields';
  let {field,value,nodes,disabled=false,onChange}:{field:FieldDefinition;value:PropertyValue;nodes:FieldTarget[];disabled?:boolean;onChange:(value:PropertyValue)=>void}=$props();
  const targets=$derived(referenceTargets(field,nodes));
  const helpId='field-help-'+crypto.randomUUID();
  const accessibility=$derived({'aria-label':field.label,'aria-describedby':field.metadata?.help?helpId:undefined});
</script>
<label class="field compact">{field.label}
  {#if field.kind==='boolean'&&!field.required}<select {...accessibility} value={value===true?'true':value===false?'false':''} {disabled} onchange={event=>onChange(event.currentTarget.value===''?null:event.currentTarget.value==='true')}><option value="">Unspecified</option><option value="true">Yes</option><option value="false">No</option></select>
  {:else if field.kind==='boolean'}<input {...accessibility} type="checkbox" checked={value===true} {disabled} onchange={(event)=>onChange(event.currentTarget.checked)} />
  {:else if field.kind==='number'}<input {...accessibility} type="number" value={typeof value==='number'?value:undefined} {disabled} oninput={(event)=>onChange(event.currentTarget.value===''?null:Number(event.currentTarget.value))} />
  {:else if field.kind==='choice'}<select {...accessibility} value={String(value??'')} {disabled} onchange={event=>onChange(event.currentTarget.value||null)}><option value="">Unspecified</option>{#if typeof value==='string'&&value&&!field.metadata?.choices?.includes(value)}<option value={value} disabled>{value} (invalid choice)</option>{/if}{#each field.metadata?.choices??[] as choice}<option value={choice}>{choice}</option>{/each}</select>
  {:else if field.kind==='nodeRef'}<select {...accessibility} value={String(value??'')} {disabled} onchange={(event)=>onChange(event.currentTarget.value||null)}><option value="">Unspecified</option>{#if typeof value==='string'&&value&&!targets.some(n=>n.id===value)}<option value={value} disabled>{nodes.find(n=>n.id===value)?.name??'Missing reference'} (not eligible)</option>{/if}{#each targets as target}<option value={target.id}>{target.name}</option>{/each}</select>
  {:else if field.kind==='nodeRefs'}<select {...accessibility} multiple value={Array.isArray(value)?value:[]} {disabled} onchange={(event)=>onChange(Array.from(event.currentTarget.selectedOptions).map((option)=>option.value))}>{#if Array.isArray(value)}{#each value.filter(id=>!targets.some(n=>n.id===id)) as id}<option value={id} disabled>{nodes.find(n=>n.id===id)?.name??'Missing reference'} (not eligible)</option>{/each}{/if}{#each targets as target}<option value={target.id}>{target.name}</option>{/each}</select>
  {:else if field.kind==='longText'}<textarea {...accessibility} value={String(value??'')} {disabled} rows={3} maxlength={16000} oninput={event=>onChange(event.currentTarget.value)}></textarea>
  {:else}<input {...accessibility} value={String(value??'')} {disabled} maxlength={16000} oninput={(event)=>onChange(event.currentTarget.value)} />{/if}
  {#if field.metadata?.help}<span class="field-help" id={helpId}>{field.metadata.help}</span>{/if}
</label>
