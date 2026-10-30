<script lang="ts">
  import type {PropertyValue,WorldEdge} from './bindings';
  import type {World} from './model';
  import type {FieldTarget} from './node-fields';
  import {linkFields} from './link-fields';
  import PropertyInput from './PropertyInput.svelte';
  let {world,link,nodes=world.nodes,disabled=false,canEdit=()=>true,onChange}:{world:World;link:WorldEdge;nodes?:FieldTarget[];disabled?:boolean;canEdit?:(key:string)=>boolean;onChange:(key:string,value:PropertyValue|undefined)=>void}=$props();
  const fields=$derived(linkFields(world,link));
</script>
{#each fields as field (field.key)}
  <PropertyInput {field} value={link.properties?.[field.key]??null} {nodes} disabled={disabled||!canEdit(field.key)} onChange={value=>onChange(field.key,value)}/>
{/each}
{#each Object.keys(link.properties??{}).filter(key=>!fields.some(f=>f.key===key)) as key}
  <p class="inline-error">Unknown field: {key}. <button type="button" class="text-action danger-text" disabled={disabled||!canEdit(key)} onclick={()=>onChange(key,undefined)}>Remove field</button></p>
{/each}
