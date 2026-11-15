<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { commands, desktop, unwrap } from './native';
  import { libraryDragType, libraryDragData } from './node-library';
  import type { LibraryListing_Serialize, LibraryFile } from './bindings';
  let {editable,onInsert,onClose,onDrag}:{editable:boolean;onInsert:(file:{fileName:string;sha256:string})=>void;onClose:()=>void;onDrag:(active:boolean)=>void}=$props();
  let listing=$state<LibraryListing_Serialize|null>(null),error=$state(''),loading=$state(false),search=$state('');
  const id = $props.id();
  let selectedFolder = $state<string | null>(null);
  let entries: HTMLDivElement;
  let token=0;
  function topFolder(path: string) {
    const separator = path.indexOf('/');
    return separator === -1 ? '' : path.slice(0, separator);
  }
  const folders = $derived([...new Set(listing?.files.map(file => topFolder(file.fileName)))].sort((a, b) => a.localeCompare(b)));
  const tabs = $derived(folders.some(Boolean) ? [null, ...folders] : []);
  const activeFolder = $derived(tabs.includes(selectedFolder) ? selectedFolder : null);
  const activeTab = $derived(tabs.indexOf(activeFolder));
  const query = $derived(search.toLowerCase());
  const filtered = $derived(listing?.files.filter(file =>
    (activeFolder === null || topFolder(file.fileName) === activeFolder) &&
    `${file.name} ${file.description} ${file.fileName}`.toLowerCase().includes(query)
  ) ?? []);

  function selectFolder(folder: string | null) {
    if (folder === selectedFolder) return;
    selectedFolder = folder;
    entries.scrollTop = 0;
  }

  function selectTab(event: KeyboardEvent, index: number) {
    const next = ['ArrowLeft', 'ArrowRight', 'Home', 'End'].indexOf(event.key);
    if (next === -1) return;
    event.preventDefault();
    const destination = [(index + tabs.length - 1) % tabs.length, (index + 1) % tabs.length, 0, tabs.length - 1][next];
    selectFolder(tabs[destination]);
    document.getElementById(`${id}-tab-${destination}`)?.focus();
  }

  function showListing(result: LibraryListing_Serialize) {
    if (listing?.folderPath !== result.folderPath ||
        !result.files.some(file => topFolder(file.fileName) === selectedFolder)) selectedFolder = null;
    listing = result;
  }
  async function updateListing(load: () => Promise<LibraryListing_Serialize>) {
    const current=++token;loading=true;error='';
    try {const result=await load();if(current===token)showListing(result);}
    catch(caught){if(current===token)error=String(caught);}
    finally{if(current===token)loading=false;}
  }
  export function refresh() {
    return updateListing(async () => unwrap(await commands.listNodeLibrary()));
  }
  async function choose() {
    try {
      const path=await open({directory:true,multiple:false,title:'Choose the node library folder'});
      if(typeof path==='string')await updateListing(async () => unwrap(await commands.chooseNodeLibrary(path)));
    }catch(caught){error=String(caught);}
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
  {#if tabs.length}
    <div class="library-folder-tabs" role="tablist" aria-label="Library folders">
      {#each tabs as folder, index (folder)}
        <button id={`${id}-tab-${index}`} role="tab" aria-selected={index === activeTab} aria-controls={`${id}-entries`}
          tabindex={index === activeTab ? 0 : -1} onclick={() => selectFolder(folder)} onkeydown={event => selectTab(event, index)}
          title={folder || undefined}>{folder === null ? 'All' : folder || 'Root'}</button>
      {/each}
    </div>
  {/if}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex (The active tab panel needs a keyboard stop even when it contains no enabled Add buttons.) -->
  <div class="library-entries" bind:this={entries} id={`${id}-entries`} role={tabs.length ? 'tabpanel' : undefined}
    aria-labelledby={tabs.length ? `${id}-tab-${activeTab}` : undefined} tabindex={tabs.length ? 0 : undefined} aria-busy={loading}>
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
      {#if !loading && listing}<p class="field-help">{search?'No matching entries.':'Save a map selection, or add library JSON files and choose Refresh.'}</p>{/if}
    {/each}
  </div>
</aside>
