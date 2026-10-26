<script lang="ts">
  import {commands,unwrap} from './native';
  let {storyId,id,accepted=true,alt='Story image',compact=false}:{storyId:string;id:string;accepted?:boolean;alt?:string;compact?:boolean}=$props();
  let url=$state(''),error=$state('');
  $effect(()=>{const story=storyId,key=id,saved=accepted,small=compact;let disposed=false,local='';url='';error='';void commands.imagePreview(story,key,saved,small?64:768).then(unwrap).then(bytes=>{if(disposed)return;local=URL.createObjectURL(new Blob([new Uint8Array(bytes)],{type:'image/png'}));url=local;}).catch(caught=>{if(!disposed)error=caught instanceof Error?caught.message:String(caught);});return()=>{disposed=true;if(local)URL.revokeObjectURL(local);};});
</script>
{#if url}<img class="saved-image" src={url} {alt}/>{:else if compact}<span title={error||'Loading image'} aria-label={error||'Loading image'}>◉</span>{:else if error}<p class="inline-error" role="status">{error}</p>{:else}<p class="field-help">Loading image…</p>{/if}
