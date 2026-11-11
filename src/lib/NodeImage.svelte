<script lang="ts">
  import {untrack} from 'svelte';
  import {commands,unwrap} from './native';
  import ImagePreview from './ImagePreview.svelte';
  import type {World} from './model';
  import type {MediaEntry} from './bindings';
  let {storyId,nodeId,world,revision,historical=false}:{storyId:string;nodeId:string;world:World;revision:number;historical?:boolean}=$props();
  let selected=$state<MediaEntry|null>(null),error=$state('');
  $effect(()=>{const story=storyId,id=nodeId,current=world;void revision;let disposed=false;untrack(()=>{void commands.nodeImage(story,id,$state.snapshot(current)).then(unwrap).then(result=>{if(!disposed){selected=result;error='';}}).catch(caught=>{if(!disposed)error=String(caught);});});return()=>{disposed=true;};});
</script>
{#if selected&&!selected.error&&(!historical||selected.applicable)}<ImagePreview {storyId} id={selected.id} alt="Selected node image"/>{#if !selected.applicable}<p class="field-help">Appearance or outfit has changed since this image.</p>{/if}
{:else if error||selected?.error}<p class="inline-error" role="status">{error||selected?.error}</p>
{:else}<div class="portrait-placeholder"><small>{historical?'No matching historical image':'No saved image'}</small></div>{/if}
