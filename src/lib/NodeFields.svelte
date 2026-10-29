<script lang="ts">
  import type {PropertyValue,WorldNode} from './bindings';
  import type {World} from './model';
  import NodeFieldSection from './NodeFieldSection.svelte';
  import {capabilitySections,type FieldTarget} from './node-fields';
  let {world,node,nodes=world.nodes,disabled=false,canEdit=()=>true,onChange}:{
    world:World;node:WorldNode;nodes?:FieldTarget[];disabled?:boolean;
    canEdit?:(componentId:string,key:string)=>boolean;
    onChange:(componentId:string,key:string,value:PropertyValue)=>void;
  }=$props();
  const sections=$derived(capabilitySections(world,node));
</script>
{#key node.id}
  <div class="node-fields">
    {#each sections as {definition,values},index (definition.id)}
      <NodeFieldSection {definition} {values} {nodes} {disabled} initiallyOpen={index===0} canEdit={key=>canEdit(definition.id,key)} onChange={(key,value)=>onChange(definition.id,key,value)}/>
    {/each}
  </div>
{/key}
