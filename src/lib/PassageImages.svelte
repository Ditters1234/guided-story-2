<script lang="ts">
  import {commands,unwrap} from './native';
  import ImagePreview from './ImagePreview.svelte';
  import type {MediaEntry} from './bindings';
  let {storyId,passageId,revision}:{storyId:string;passageId:string;revision:number}=$props();
  let entries=$state<MediaEntry[]>([]),selected=$state(''),error=$state('');
  $effect(()=>{const story=storyId,id=passageId;void revision;let disposed=false;void commands.listImages(story,{nodeId:null,passageId:id,moment:'after',subjectIds:[]}).then(unwrap).then(result=>{if(disposed)return;entries=result.filter(e=>e.stage==='accepted');if(!entries.some(e=>e.id===selected))selected=entries.find(e=>e.applicable)?.id??'';error='';}).catch(caught=>{if(!disposed)error=String(caught);});return()=>{disposed=true;};});
</script>
{#if error}<p class="inline-error" role="status">{error}</p>{/if}
{#if entries.length}<section class="passage-images" aria-label="Passage illustrations"><label class="field">Illustration<select bind:value={selected}><option value="">None</option>{#each entries as entry}<option value={entry.id}>{entry.label} · {entry.id.slice(0,8)}{entry.applicable?'':' · earlier passage version'}</option>{/each}</select></label>{#if selected}<ImagePreview {storyId} id={selected} alt="Accepted passage illustration"/>{#if entries.find(e=>e.id===selected)?.applicable===false}<p class="field-help">Captured from an earlier passage version.</p>{/if}{/if}</section>{/if}
