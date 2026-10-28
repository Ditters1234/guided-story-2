<script lang="ts">
  import type { FieldDefinition,PropertyValue } from './bindings';
  let {field,value,nodes,disabled=false,onChange}:{field:FieldDefinition;value:PropertyValue;nodes:{id:string;name:string}[];disabled?:boolean;onChange:(value:PropertyValue)=>void}=$props();
</script>
<label class="field compact">{field.label}
  {#if field.kind==='boolean'}<input type="checkbox" checked={Boolean(value)} {disabled} onchange={(event)=>onChange(event.currentTarget.checked)} />
  {:else if field.kind==='number'}<input type="number" value={typeof value==='number'?value:undefined} {disabled} oninput={(event)=>onChange(event.currentTarget.value===''?null:Number(event.currentTarget.value))} />
  {:else if field.kind==='nodeRef'}<select value={String(value??'')} {disabled} onchange={(event)=>onChange(event.currentTarget.value||null)}><option value="">Unspecified</option>{#each nodes as target}<option value={target.id}>{target.name}</option>{/each}</select>
  {:else if field.kind==='nodeRefs'}<select multiple value={Array.isArray(value)?value:[]} {disabled} onchange={(event)=>onChange(Array.from(event.currentTarget.selectedOptions).map((option)=>option.value))}>{#each nodes as target}<option value={target.id}>{target.name}</option>{/each}</select>
  {:else if field.kind==='longText'}<textarea value={String(value??'')} {disabled} rows={5} maxlength={16000} oninput={event=>onChange(event.currentTarget.value)}></textarea>
  {:else}<input value={String(value??'')} {disabled} maxlength={16000} oninput={(event)=>onChange(event.currentTarget.value)} />{/if}
</label>
