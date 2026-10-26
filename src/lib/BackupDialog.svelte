<script lang="ts">
  import {onMount} from 'svelte';
  import {open} from '@tauri-apps/plugin-dialog';
  import {commands,unwrap} from './native';
  import type {BackupReport,ProjectView_Serialize} from './bindings';
  let {mode,project,onBeforeAction,onOpen,onClose,onWorking}:{mode:'export'|'restore';project:ProjectView_Serialize|null;onBeforeAction:()=>Promise<ProjectView_Serialize|null>;onOpen:(path:string)=>Promise<void>;onClose:()=>void;onWorking:(working:boolean)=>void}=$props();
  let dialog:HTMLDialogElement;
  let source=$state(''),parent=$state(''),preview=$state<BackupReport|null>(null),result=$state<BackupReport|null>(null),working=$state(false),error=$state('');
  let operation:{id:string;revision:number;storyId:string}|null=null;
  $effect(()=>{dialog?.showModal();});
  $effect(()=>{onWorking(working);});
  onMount(()=>()=>onWorking(false));
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  function size(bytes:number){return bytes<1024*1024?`${(bytes/1024).toFixed(1)} KiB`:`${(bytes/1024/1024).toFixed(1)} MiB`;}
  async function chooseSource(){working=true;error='';try{const path=await open({directory:true,multiple:false,title:'Choose a Guided Story backup folder'});if(typeof path!=='string')return;source=path;preview=null;result=null;operation=null;preview=unwrap(await commands.inspectProjectBackup(path));}catch(caught){error=describe(caught);}finally{working=false;}}
  async function chooseParent(){working=true;error='';try{const path=await open({directory:true,multiple:false,title:mode==='export'?'Choose where to save the backup folder':'Choose where to create the restored story folder'});if(typeof path==='string'){parent=path;operation=null;result=null;}}catch(caught){error=describe(caught);}finally{working=false;}}
  async function run(){working=true;error='';try{const current=await onBeforeAction();if(!operation)operation={id:crypto.randomUUID(),revision:current?.revision??0,storyId:current?.storyId??''};result=mode==='export'?unwrap(await commands.exportProjectBackup(operation.storyId,operation.revision,parent,operation.id)):unwrap(await commands.restoreProjectBackup(source,parent,operation.id,preview!.fingerprint));}catch(caught){error=describe(caught);}finally{working=false;}}
  async function openRestored(){if(!result)return;working=true;error='';try{await onOpen(result.folderPath);}catch(caught){error=describe(caught);}finally{working=false;}}
</script>
<dialog bind:this={dialog} class="project-dialog backup-dialog" aria-labelledby="backup-title" oncancel={event=>{event.preventDefault();if(!working)onClose();}}>
  <header class="settings-heading"><h2 id="backup-title">{mode==='export'?'Export backup':'Restore backup'}</h2><button class="icon-button" aria-label="Close backup tools" disabled={working} onclick={onClose}>×</button></header>
  <div class="backup-body">
    <p class="field-help">{mode==='export'?'Save the story, history, images and saved drafts in a self-contained folder. App connection settings stay on this device.':'Validate a backup and create a new story folder. The original story and backup are preserved.'}</p>
    {#if error}<p class="story-error" role="alert">{error}</p>{/if}
    {#if mode==='restore'}<button class="secondary-button" disabled={working||!!result} onclick={()=>{void chooseSource();}}>Choose backup folder</button>{#if source}<p class="backup-path">{source}</p>{/if}{/if}
    {#if preview||mode==='export'&&project}{@const info=preview??project!}<p><strong>{info.title}</strong> · revision {info.revision}</p>{#if preview}<p class="field-help">Validated · {preview.files} files · {size(preview.bytes)}</p>{/if}{/if}
    {#if !result}<button class="secondary-button" disabled={working||mode==='restore'&&!preview} onclick={()=>{void chooseParent();}}>Choose destination</button>{#if parent}<p class="backup-path">{parent}</p>{/if}{:else}<p role="status">{mode==='export'?'Backup saved':'Story restored'} · {result.files} files · {size(result.bytes)}</p><p class="backup-path">{result.folderPath}</p>{#if result.omittedFiles}<p class="field-help">{result.omittedFiles} unrelated entries or temporary writes were excluded. All recognized story data was retained.</p>{/if}{/if}
  </div>
  <footer class="settings-footer"><span aria-live="polite">{working?'Working… keep this window open.':result?'Complete':'Creates a new folder'}</span><div class="settings-footer-actions">{#if !result}<button class="primary-button" disabled={working||!parent||mode==='restore'&&!preview||mode==='export'&&!project} onclick={()=>{void run();}}>{mode==='export'?'Export backup':'Restore to new folder'}</button>{:else if mode==='restore'}<button class="primary-button" disabled={working} onclick={()=>{void openRestored();}}>Open restored story</button>{/if}<button class="secondary-button" disabled={working} onclick={onClose}>{result?'Done':'Cancel'}</button></div></footer>
</dialog>
