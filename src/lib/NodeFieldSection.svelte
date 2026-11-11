<script lang="ts">
  import {untrack} from 'svelte';
  import type {ComponentDefinition,PropertyValue} from './bindings';
  import PropertyInput from './PropertyInput.svelte';
  import {sectionSummary,type FieldTarget} from './node-fields';
  let {definition,values,nodes,disabled,initiallyOpen=false,canEdit,onChange}:{
    definition:ComponentDefinition;values:Record<string,PropertyValue>;nodes:FieldTarget[];
    disabled:boolean;initiallyOpen?:boolean;canEdit:(key:string)=>boolean;
    onChange:(key:string,value:PropertyValue)=>void;
  }=$props();
  // Opening is local UI state. Replacing a node's field values must not reset it.
  let expanded=$state(untrack(()=>initiallyOpen));
</script>
<details class="capability node-field-section" bind:open={expanded}>
  <summary><strong>{definition.name}</strong><span>{sectionSummary(definition,values)}</span></summary>
  <div class="node-field-body">
    {#each definition.fields as field (field.key)}
      <PropertyInput {field} value={values[field.key]??null} {nodes} disabled={disabled||!canEdit(field.key)} onChange={value=>onChange(field.key,value)}/>
    {/each}
  </div>
</details>
