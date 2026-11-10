<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { commands, desktop, unwrap } from './native';
  import { libraryDragType, libraryDragData } from './node-library';
  import type { LibraryListing_Serialize, LibraryFile } from './bindings';
  let {editable,onInsert,onClose,onDrag}:{editable:boolean;onInsert:(file:{fileName:string;sha256:string})=>void;onClose:()=>void;onDrag:(active:boolean)=>void}=$props();
  let listing=$state<LibraryListing_Serialize|null>(null),error=$state(''),loading=$state(false),search=$state('');
  let token=0;
  const filtered=$derived(listing?.files.filter(file=>`${file.name} ${file.description} ${file.fileName}`.toLowerCase().includes(search.toLowerCase()))??[]);
  export async function refresh() {
    const current=++token;loading=true;error='';
    try {const result=unwrap(await commands.listNodeLibrary());if(current===token)listing=result;}
    catch(caught){if(current===token)error=String(caught);}
    finally{if(current===token)loading=false;}
  }
  async function choose() {
    try {
      const path=await open({directory:true,multiple:false,title:'Choose the node library folder'});
      if(typeof path!=='string')return;
      token++;loading=true;error='';listing=unwrap(await commands.chooseNodeLibrary(path));
    }catch(caught){error=String(caught);}finally{loading=false;}
  }
  function drag(event:DragEvent,file:LibraryFile) {
    if(!editable||file.error||!event.dataTransfer){event.preventDefault();return;}
    event.dataTransfer.setData(libraryDragType,libraryDragData(file));event.dataTransfer.setData('text/plain',file.name);event.dataTransfer.effectAllowed='copy';onDrag(true);
  }
  onMount(()=>{void refresh();});
</script>

<aside class="node-library" aria-label="Node library">
  <header><strong>Node library</strong><button class="quiet-button" onclick={onClose} aria-label="Close node library">×</button></header>
  <div class="library-folder-actions"><button class="text-action" onclick={choose} disabled={loading||!desktop}>Choose folder…</button><button class="text-action" onclick={()=>{void refresh();}} disabled={loading||!desktop}>Refresh</button></div>
  {#if listing}<p class="library-folder" title={listing.folderPath}>{listing.folderPath}</p>{/if}
  <p class="field-help">Drag a card onto the map, or choose Add.</p>
  <input aria-label="Search node library" placeholder="Find a node or group…" bind:value={search}/>
  {#if error}<p role="alert" class="inline-error">{error}</p>{/if}
  {#each listing?.warnings??[] as warning}<p class="field-help">{warning}</p>{/each}
  <div class="library-entries" aria-busy={loading}>
    {#each filtered as file (file.fileName)}
      <article class="library-entry" draggable={editable&&!file.error&&!loading} ondragstart={event=>drag(event,file)} ondragend={()=>onDrag(false)}>
        <strong>{file.name}</strong>
        {#if file.error}<p class="inline-error">{file.error}</p>
        {:else}
          <small>{file.nodeCount} {file.nodeCount===1?'node':'nodes'} · {file.connectionCount} {file.connectionCount===1?'connection':'connections'}</small>
          {#if file.description}<p>{file.description}</p>{/if}
          <button class="secondary-button" disabled={!editable||loading} onclick={()=>onInsert(file)} aria-label={`Add ${file.name} to the story`}>Add…</button>
        {/if}
        <small class="library-file-name">{file.fileName}</small>
      </article>
    {:else}
      {#if !loading && listing}<p class="field-help">{search?'No matching entries.':'Save a map selection here, or put library JSON files in this folder and Refresh.'}</p>{/if}
    {/each}
  </div>
</aside>
