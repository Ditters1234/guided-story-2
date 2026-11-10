<script lang="ts">
  import { untrack } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { commands, unwrap } from './native';
  import type { World, Workspace } from './model';
  import type { LibraryExport_Serialize, LibraryListing_Serialize } from './bindings';
  let {world,nodeIds,positions,onClose,onSaved}:{world:World;nodeIds:string[];positions:Workspace['positions'];onClose:()=>void;onSaved:(name:string)=>void}=$props();
  const source=untrack(()=>structuredClone($state.snapshot(world)));
  const ids=untrack(()=>[...nodeIds]);
  const layout=untrack(()=>structuredClone($state.snapshot(positions)));
  let name=$state(untrack(()=>ids.length===1?source.nodes.find(node=>node.id===ids[0])?.name??'Library node':'Selected group'));
  let description=$state(''),error=$state(''),working=$state(false),preview=$state<LibraryExport_Serialize|null>(null);
  let listing=$state<LibraryListing_Serialize|null>(null);
  let signature=$state(''),requestId=crypto.randomUUID();
  let dialog:HTMLDialogElement;
  $effect(()=>{dialog?.showModal();});
  $effect(()=>{if(signature!==JSON.stringify([name,description]))preview=null;});
  async function choose() {
    try {const path=await open({directory:true,multiple:false,title:'Choose the node library folder'});if(typeof path==='string')listing=unwrap(await commands.chooseNodeLibrary(path));}
    catch(caught){error=String(caught);}
  }
  async function review(event:SubmitEvent) {
    event.preventDefault();working=true;error='';const input=JSON.stringify([name,description]);
    try {
      listing=unwrap(await commands.listNodeLibrary());
      const result=unwrap(await commands.prepareLibraryExport(source,ids,layout,name,description));
      if(input!==JSON.stringify([name,description]))return;
      signature=input;preview=result;requestId=crypto.randomUUID();
    } catch(caught){error=String(caught);}finally{working=false;}
  }
  async function save() {
    if(!preview||signature!==JSON.stringify([name,description]))return;
    working=true;error='';
    try {const file=unwrap(await commands.saveNodeLibrary(requestId,$state.snapshot(preview.entry)));onSaved(file);}
    catch(caught){error=String(caught);}finally{working=false;}
  }
</script>

<dialog bind:this={dialog} class="project-dialog node-definition-dialog" onclose={onClose} oncancel={event=>{if(working)event.preventDefault();}} aria-labelledby="library-save-title">
  <form onsubmit={review}>
    <header class="settings-heading"><h2 id="library-save-title">Save selection to library</h2></header>
    <div class="node-definition-body">
      <fieldset disabled={working}>
        <label class="field">Library name<input bind:value={name} required maxlength={200}/></label>
        <label class="field">Description<textarea bind:value={description} maxlength={4000} rows={3}></textarea></label>
      </fieldset>
      <p>{ids.length} selected {ids.length===1?'node':'nodes'}. Internal connections and required custom definitions travel with the group.</p>
      {#if listing}<p class="library-folder">{listing.folderPath}</p>{/if}
      <button type="button" class="text-action" onclick={choose} disabled={working}>Choose library folder…</button>
      {#if preview}<div class="change-review"><h3>Library contents</h3><ul>{#each preview.entry.world.nodes as node}<li>{node.name}</li>{/each}</ul>{#each preview.notices as notice}<p>{notice}</p>{/each}</div>{/if}
      {#if error}<p role="alert" class="inline-error">{error}</p>{/if}
    </div>
    <footer class="settings-footer"><button type="button" class="quiet-button" onclick={onClose} disabled={working}>Cancel</button>
      {#if preview}<button type="button" class="primary-button" onclick={save} disabled={working}>{working?'Saving…':'Save library entry'}</button>
      {:else}<button type="submit" class="primary-button" disabled={working}>{working?'Checking…':'Review entry'}</button>{/if}
    </footer>
  </form>
</dialog>
