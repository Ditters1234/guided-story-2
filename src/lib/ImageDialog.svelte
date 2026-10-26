<script lang="ts">
  import {onMount,untrack} from 'svelte';
  import {open} from '@tauri-apps/plugin-dialog';
  import {commands,unwrap} from './native';
  import {MediaSession} from './media-session.svelte';
  import ImagePreview from './ImagePreview.svelte';
  import type {ProjectView,World} from './model';
  import type {MediaTarget,MediaEntry,MediaAsset,ImageConnection,CommitResult_Serialize} from './bindings';
  let {project,target,sourceWorlds,onBeforeAction,onApply,onClose,onSettings}:{project:ProjectView;target:MediaTarget;sourceWorlds:{before:World;after:World};onBeforeAction:()=>Promise<ProjectView>;onApply:(result:CommitResult_Serialize)=>Promise<void>;onClose:()=>void;onSettings:()=>void}=$props();
  const session=new MediaSession(untrack(()=>project.storyId));
  let entries=$state<MediaEntry[]>([]),settings=$state<ImageConnection|null>(null),asset=$state<MediaAsset|null>(null),working=$state(false),error=$state(''),promptId=$state(''),graph=$state('');
  let shot=$state<MediaTarget>(untrack(()=>structuredClone($state.snapshot(target))));
  let subjectSearch=$state('');
  let dialog:HTMLDialogElement;
  const stage=$derived(asset?'accepted':session.view?.draft.stage),request=$derived(asset?.request??session.value),source=$derived(asset?.source??session.view?.draft.source);
  const changedShot=$derived(JSON.stringify(shot)!==JSON.stringify(session.view?.draft.target??asset?.target));
  const world=$derived(shot.moment==='before'?sourceWorlds.before:sourceWorlds.after);
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  $effect(()=>{dialog?.showModal();});
  export async function flushDraft(){await session.flush();}
  async function refresh(){entries=unwrap(await commands.listImages(project.storyId,target));}
  async function close(){if(working)return;try{await session.flush();onClose();}catch(caught){error=describe(caught);}}
  async function choose(entry:MediaEntry){working=true;try{await session.flush();if(entry.stage==='accepted'){asset=unwrap(await commands.savedImage(project.storyId,entry.id));}else{await session.open(entry.id);asset=null;}graph='';error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function fresh(reroll=false){working=true;try{await session.flush();await onBeforeAction();const previous=reroll&&request?structuredClone($state.snapshot(request)):null;await session.create($state.snapshot(shot));asset=null;if(previous){if(settings?.profile.bindings.seed)previous.seed=crypto.getRandomValues(new Uint32Array(1))[0];session.value=previous;session.touch();await session.flush();}graph='';await refresh();error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function run(action:()=>Promise<void>){working=true;error='';try{await session.flush();await action();await refresh();}catch(caught){error=describe(caught);}finally{working=false;}}
  async function submit(){await run(async()=>{await onBeforeAction();const d=session.view!.draft;session.adopt(unwrap(await commands.submitImage(project.storyId,d.id,d.version)));});}
  async function check(){await run(async()=>{session.adopt(unwrap(await commands.checkImage(project.storyId,session.view!.draft.id)));});}
  async function recover(){await run(async()=>{const d=session.view!.draft;session.adopt(unwrap(await commands.recoverImagePrompt(project.storyId,d.id,d.version,promptId.trim())));});}
  async function importFile(){await run(async()=>{const path=await open({multiple:false,filters:[{name:'Images',extensions:['png','jpg','jpeg','webp']}]});if(typeof path!=='string')return;const d=session.view!.draft;session.adopt(unwrap(await commands.importImage(project.storyId,d.id,d.version,path)));});}
  async function accept(){await run(async()=>{await onBeforeAction();const d=session.view!.draft;await onApply(unwrap(await commands.acceptImage(project.storyId,d.id,d.version)));await session.refresh();});}
  async function select(){if(!asset||!asset.target.nodeId)return;await run(async()=>{await onBeforeAction();await onApply(unwrap(await commands.chooseNodeImage(project.storyId,asset!.target.nodeId!,asset!.id,crypto.randomUUID())));});}
  async function clear(){if(!target.nodeId)return;await run(async()=>{await onBeforeAction();await onApply(unwrap(await commands.clearNodeImage(project.storyId,target.nodeId!,crypto.randomUUID())));});}
  function subject(id:string){shot.subjectIds=shot.subjectIds.includes(id)?shot.subjectIds.filter(n=>n!==id):[...shot.subjectIds,id];}
  function controlInput(control:'width'|'height'|'seed'|'steps'|'cfg',input:HTMLInputElement){if(!request)return;const value=input.value===''?null:input.valueAsNumber;if(value!==null&&(!Number.isFinite(value)||control!=='cfg'&&(!Number.isInteger(value)||value<0||value>4294967295)))return;request[control]=value;session.touch();}
  onMount(()=>{void(async()=>{try{settings=unwrap(await commands.loadImageSettings());await refresh();const selected=target.nodeId?unwrap(await commands.nodeImage(project.storyId,target.nodeId,$state.snapshot(sourceWorlds.after))):null;const existing=entries.find(e=>e.stage!=='accepted')??entries.find(e=>e.id===selected?.id)??entries[0];if(existing)await choose(existing);else await fresh();}catch(caught){error=describe(caught);}})();return()=>session.dispose();});
</script>
<dialog bind:this={dialog} class="project-dialog image-dialog" aria-labelledby="image-title" oncancel={event=>{event.preventDefault();void close();}}>
  <header class="settings-heading"><h2 id="image-title">{target.nodeId?'Node image':'Passage illustration'}</h2><button class="icon-button" aria-label="Close image editor" disabled={working} onclick={()=>{void close();}}>×</button></header>
  <div class="image-editor-body">
    {#if error||session.error}<div class="story-error" role="alert">{error||session.error}<button class="text-action" disabled={working} onclick={()=>{void run(async()=>{await session.refresh();});}}>Retry / refresh</button></div>{/if}
    <div class="image-tools"><button class="secondary-button" disabled={working} onclick={()=>{void fresh();}}>New shot</button><button class="secondary-button" disabled={working||!request} onclick={()=>{void fresh(true);}}>Reroll image</button><button class="text-action" disabled={working} onclick={()=>{void session.flush().then(onSettings).catch(caught=>{error=describe(caught);});}}>Image settings</button></div>
    {#if target.passageId}<details><summary>Shot source & subjects</summary><label class="field">World state<select bind:value={shot.moment} disabled={working}><option value="before">Before passage</option><option value="after">After passage</option></select></label><input aria-label="Find image subjects" placeholder="Find a subject…" bind:value={subjectSearch}/><div class="image-subjects">{#each world.nodes.filter(node=>node.name.toLowerCase().includes(subjectSearch.toLowerCase())).slice(0,100) as node}<label><input type="checkbox" checked={shot.subjectIds.includes(node.id)} disabled={working||shot.subjectIds.length>=4&&!shot.subjectIds.includes(node.id)} onchange={()=>subject(node.id)}/>{node.name}</label>{/each}</div><p class="field-help">Up to four subjects. New shot captures this selection; manual prompt edits describe the chosen moment.</p></details>{/if}
    {#if changedShot}<p class="story-warning">Choose New shot to use the selected source and subjects.</p>{/if}
    {#if entries.length}<details><summary>Saved images & candidates ({entries.length})</summary><div class="image-history">{#each entries as entry}<button class="secondary-button" disabled={working} onclick={()=>{void choose(entry);}}>{entry.label} · {entry.stage}{entry.applicable?'':' · source changed'}<small>{entry.id.slice(0,8)}</small></button>{/each}</div></details>{/if}
    {#if asset?.target.nodeId}<button class="text-action" disabled={working} onclick={()=>{void clear();}}>Clear displayed image</button>{/if}
    {#if source}<details><summary>Captured appearance & outfit</summary>{#if (asset?.target??session.view?.draft.target)?.passageId}<p class="field-help">{(asset?.target??session.view?.draft.target)?.moment} the accepted passage version. Current outfits are not substituted.</p>{/if}{#each source.facts as fact}<h3>{fact.name}</h3><p class="image-facts">{fact.appearance||'No visible description supplied.'}</p>{#if fact.outfit.length}<ul>{#each fact.outfit as garment}<li>{garment}</li>{/each}</ul>{/if}{/each}</details>{/if}
    {#if session.view?.stale&&!asset}<p class="story-warning">Source appearance or passage changed. This image keeps its captured source; create a new shot before submitting.</p>{/if}
    {#if request}<fieldset class="image-fields" disabled={working||stage!=='draft'}><label class="field">Image prompt<textarea bind:value={request.prompt} rows={7} maxlength={16000} oninput={()=>session.touch()}></textarea></label>
      {#if settings?.profile.bindings.negative||request.negative}<label class="field">Negative prompt<textarea value={request.negative??''} rows={2} maxlength={16000} oninput={event=>{request!.negative=event.currentTarget.value;session.touch();}}></textarea></label>{/if}
      <details><summary>Workflow controls</summary><p class="field-help">{asset?.submission?.profile.name??session.view?.draft.submission?.profile.name??settings?.profile.name}. Blank controls keep workflow defaults.</p><div class="image-controls">{#each ['width','height','seed','steps','cfg'] as key}{@const control=key as 'width'|'height'|'seed'|'steps'|'cfg'}{#if settings?.profile.bindings[control]||request[control]!==null}<label class="field">{control}<input type="number" value={request[control]??''} min={0} step={control==='cfg'?0.1:1} oninput={event=>controlInput(control,event.currentTarget)} onblur={event=>{event.currentTarget.value=String(request![control]??'');}}/></label>{/if}{/each}</div></details>
    </fieldset>{/if}
    {#if session.view?.draft.error}<p class="story-warning">{session.view.draft.error}</p>{/if}
    {#if stage==='unknown'}<label class="field">Recovered ComfyUI prompt ID<input bind:value={promptId} maxlength={100}/></label><button class="secondary-button" disabled={working||!promptId.trim()} onclick={()=>{void recover();}}>Attach prompt ID</button>{/if}
    {#if asset}<ImagePreview storyId={project.storyId} id={asset.id} alt="Accepted image"/>{:else if session.view?.draft.image}<ImagePreview storyId={project.storyId} id={session.view.draft.id} accepted={false} alt="Image candidate for review"/>{/if}
    {#if asset?.submission||session.view?.draft.submission}<details><summary>Submitted workflow & source</summary><p class="field-help">{(asset?.submission??session.view?.draft.submission)?.endpoint} · Prompt {(asset?.submission??session.view?.draft.submission)?.promptId??'unknown'}</p><pre class="image-graph">{JSON.stringify((asset?.submission??session.view?.draft.submission)?.graph,null,2)}</pre></details>{:else if stage==='draft'}<details><summary>Prepared workflow</summary><button class="text-action" disabled={working} onclick={()=>{void run(async()=>{const d=session.view!.draft;graph=JSON.stringify(unwrap(await commands.previewImageWorkflow(project.storyId,d.id,d.version)).graph,null,2);});}}>Inspect exact graph</button>{#if graph}<pre class="image-graph">{graph}</pre>{/if}</details>{/if}
  </div>
  <footer class="settings-footer"><span aria-live="polite">{working?'Working…':session.error?'Draft not saved':session.dirty?'Unsaved draft':session.saving?'Saving draft…':stage==='queued'?'Submitted · check when ready':stage==='unknown'?'Submission uncertain':stage??'Loading…'}</span><div class="settings-footer-actions">
    {#if stage==='draft'}<button class="secondary-button" disabled={working||changedShot} onclick={()=>{void importFile();}}>Import image</button><button class="primary-button" disabled={working||changedShot||!request?.prompt.trim()||session.view?.stale} onclick={()=>{void submit();}}>Generate image</button>
    {:else if stage==='queued'}<button class="primary-button" disabled={working} onclick={()=>{void check();}}>Check result</button>
    {:else if stage==='ready'}<button class="primary-button" disabled={working} onclick={()=>{void accept();}}>{target.nodeId?'Use as node image':'Accept illustration'}</button>
    {:else if asset?.target.nodeId}<button class="primary-button" disabled={working} onclick={()=>{void select();}}>Use as node image</button>{/if}
    <button class="secondary-button" disabled={working} onclick={()=>{void close();}}>Done</button></div></footer>
</dialog>
