<script lang="ts">
  import { onMount,untrack } from 'svelte';
  import type { ApplyAuthoringResult_Serialize,AuthoringRequest,AuthoringSummary,AuthoringView_Serialize } from './bindings';
  import type { ProjectView,World } from './model';
  import { commands,unwrap } from './native';
  import { editableFields,fieldId,newAuthoringRequest,parseCandidateForEditor,changeCreationType } from './authoring-ui';
  import CandidateEditor from './CandidateEditor.svelte';
  import DeleteDraftButton from './DeleteDraftButton.svelte';
  let {project,world,initialId,initialRequest,onBeforeAction,onApply,onClose,onSettings,onRun}:{project:ProjectView;world:World;initialId:string|null;initialRequest:AuthoringRequest;onBeforeAction:()=>Promise<ProjectView>;onApply:(result:ApplyAuthoringResult_Serialize)=>Promise<void>;onClose:()=>void;onSettings:(section?:'llm'|'prompts')=>void;onRun:(id:string,running:boolean)=>void}=$props();
  let dialog:HTMLDialogElement;
  let currentId=$state(untrack(()=>initialId??crypto.randomUUID()));
  let request=$state<AuthoringRequest>(untrack(()=>structuredClone($state.snapshot(initialRequest))));
  let view=$state<AuthoringView_Serialize|null>(null),candidateText=$state(''),saved=$state<AuthoringSummary[]>([]);
  let loaded=$state(false),working=$state(false),error=$state(''),saveStatus=$state(''),contextSearch=$state('');
  let version=0,edits=0,savedEdits=0;
  let timer:ReturnType<typeof setTimeout>|undefined,poll:ReturnType<typeof setTimeout>|undefined;
  let flight:Promise<void>|null=null;
  let pending:{request:AuthoringRequest;text:string;edit:number;version:number;review:boolean}|null=null;
  const status=$derived(view?.status??'draft');
  const fields=$derived(editableFields(world,request.targetIds));
  const candidate=$derived(parseCandidateForEditor(candidateText));
  const validShape=$derived(!!candidate);
  $effect(()=>{dialog?.showModal();});
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  async function list(){saved=unwrap(await commands.listAuthoring(project.storyId));}
  async function deleteDraft(id:string,expected:number){
    working=true;error='';
    try{
      await flushDraft();
      unwrap(await commands.deleteAuthoringDraft(project.storyId,id,id===currentId?version:expected));
      if(id===currentId){clearTimeout(timer);clearTimeout(poll);pending=null;savedEdits=edits;loaded=false;onRun(id,false);onClose();}
      else await list();
    }catch(caught){error=describe(caught);try{await list();}catch{}}finally{working=false;}
  }
  function report(){onRun(currentId,view?.status==='running');}
  async function refresh(){view=unwrap(await commands.getAuthoring(project.storyId,currentId));version=view.version;report();if(view.status==='running'){poll=setTimeout(()=>{void refresh().catch((caught)=>{error=describe(caught);});},800);}else{request=structuredClone($state.snapshot(view.request));candidateText=view.candidateText;await list();}}
  function touch(){if(!loaded||!['draft','review'].includes(status))return;edits++;saveStatus='Unsaved proposal';clearTimeout(timer);timer=setTimeout(()=>{void flushDraft().catch(()=>{});},350);}
  export async function flushDraft(){
    clearTimeout(timer);if(flight)return flight;if(!loaded)return;
    flight=(async()=>{while(pending||savedEdits<edits){const operation=pending??{request:structuredClone($state.snapshot(request)),text:candidateText,edit:edits,version,review:status==='review'};pending=operation;
      try{const result=operation.review?unwrap(await commands.saveAuthoringCandidate(project.storyId,currentId,operation.version,operation.text)):unwrap(await commands.saveAuthoringDraft(project.storyId,currentId,operation.version,operation.request));view=result;version=result.version;savedEdits=operation.edit;pending=null;error='';saveStatus='Proposal saved';report();}catch(caught){error=describe(caught);saveStatus='Proposal not saved';throw caught;}}
    })();try{await flight;}finally{flight=null;}
  }
  async function openSaved(id:string){try{await flushDraft();clearTimeout(poll);loaded=false;currentId=id;edits=savedEdits=0;pending=null;await refresh();loaded=true;}catch(caught){error=describe(caught);}}
  async function newDraft(nextRequest?:AuthoringRequest){
    working=true;error='';
    try{
      await flushDraft();clearTimeout(poll);
      if(nextRequest)request=structuredClone(nextRequest);
      currentId=crypto.randomUUID();view=null;candidateText='';version=0;edits=savedEdits=0;pending=null;loaded=true;
      touch();await flushDraft();await list();onRun(currentId,false);
    }catch(caught){error=describe(caught);}finally{working=false;}
  }
  async function generate(){working=true;error='';try{await flushDraft();const current=await onBeforeAction();view=unwrap(await commands.startAuthoring(project.storyId,currentId,version,current.revision));version=view.version;report();await refresh();}catch(caught){error=describe(caught);try{await refresh();}catch{}}finally{working=false;}}
  async function apply(){working=true;error='';try{await flushDraft();await onBeforeAction();const result=unwrap(await commands.applyAuthoring(project.storyId,currentId,version));view=result.proposal;version=view.version;await onApply(result);onClose();}catch(caught){error=describe(caught);try{await refresh();}catch{}}finally{working=false;}}
  async function close(settings:'llm'|'prompts'|null=null){try{await flushDraft();if(settings)onSettings(settings);else onClose();}catch{}}
  function toggleContext(id:string){request.contextIds=request.contextIds.includes(id)?request.contextIds.filter((v)=>v!==id):[...request.contextIds,id];touch();}
  function toggleField(field:typeof fields[number]){const key=fieldId(field);request.fields=request.fields.some((f)=>fieldId(f)===key)?request.fields.filter((f)=>fieldId(f)!==key):[...request.fields,{nodeId:field.nodeId,componentId:field.componentId,key:field.key}];touch();}
  onMount(()=>{void(async()=>{try{if(initialId)await refresh();else{loaded=true;touch();await flushDraft();}loaded=true;await list();}catch(caught){error=describe(caught);}})();return()=>{clearTimeout(timer);clearTimeout(poll);};});
</script>
<dialog bind:this={dialog} class="project-dialog ai-dialog" aria-labelledby="ai-title" oncancel={(event)=>{event.preventDefault();void close();}}>
  <header class="settings-heading"><h2 id="ai-title">AI world tools</h2><button class="icon-button" aria-label="Close AI world tools" onclick={()=>{void close();}}>×</button></header>
  <div class="ai-body">
    <div class="proposal-toolbar">
      <details class="saved-proposals"><summary>Saved proposals ({saved.length})</summary>{#each saved as entry (entry.id)}{@const title=(entry.id===currentId?request.direction:entry.direction)||'Saved AI draft'}<div class="saved-draft-row"><button class:active={entry.id===currentId} onclick={()=>{void openSaved(entry.id);}} disabled={working||status==='running'}><span>{title}</span><small>{entry.id===currentId?status:entry.status}</small></button>{#if entry.id!==currentId&&!['running','applied'].includes(entry.status)}<DeleteDraftButton {title} disabled={working||!loaded||status==='running'} onDelete={()=>deleteDraft(entry.id,entry.version)}/>{/if}</div>{/each}</details>
      <button class="text-action" disabled={!loaded||working||status==='running'} onclick={()=>{void newDraft(newAuthoringRequest(world));}}>New creation</button>
    </div>
    {#if error}<div class="settings-error" role="alert">{error}<button class="quiet-button" onclick={()=>{void flushDraft().then(refresh).catch((caught)=>{error=describe(caught);});}}>Retry / refresh</button></div>{/if}
    {#if status==='draft'}
      <fieldset class="connection-form" disabled={!loaded||working}>
        {#if request.mode==='create'}<label class="field">Create<select value={request.typeId??''} onchange={(event)=>{request=changeCreationType(world,request,event.currentTarget.value||null);touch();}}><option value="">World / mixed group from an idea</option>{#each world.nodeTypes as type}<option value={type.id}>{type.name}</option>{/each}</select></label><label class="check-field"><input type="checkbox" checked={request.bundle} disabled={!request.typeId} onchange={(event)=>{request.bundle=event.currentTarget.checked;touch();}} /><span>Include outfit items, belongings and related nodes</span></label>{#if !request.typeId}<p class="field-help">Develop the premise into named characters, places and linked outfit items. Review everything before Apply.</p>{/if}
        {:else}<div class="field"><strong>Generate / revise {world.nodes.find((n)=>n.id===request.targetIds[0])?.name}</strong><span class="field-help">Fill blank fields or revise existing details. Only checked fields may change; unchecked fields and links stay protected.</span></div><div class="editable-field-list">{#each fields as field}<label><input type="checkbox" checked={request.fields.some((f)=>fieldId(f)===fieldId(field))} onchange={()=>toggleField(field)} /><span>{field.label}</span></label>{/each}</div>{#if world.aiContext?.some(b=>request.targetIds.includes(b.nodeId))}<p class="field-help">This node's included context is editable source for this request. Other required context still applies.</p>{/if}{/if}
        <label class="field">{request.mode==='revise'?'Direction (optional)':'Direction'}<textarea bind:value={request.direction} oninput={touch} rows={4} maxlength={12000} placeholder={request.mode==='revise'?"Optional: specify what to fill in or change. Leave blank to develop checked fields from this node's name, type and context.":"Describe the node, connected group or world idea you want…"}></textarea></label>
        <details class="reference-picker"><summary>Context references ({request.contextIds.length}/12)</summary><input aria-label="Find context references" placeholder="Find a node…" bind:value={contextSearch} /><div>{#each world.nodes.filter((n)=>!request.targetIds.includes(n.id)&&n.name.toLowerCase().includes(contextSearch.toLowerCase())).slice(0,100) as node}<label><input type="checkbox" checked={request.contextIds.includes(node.id)} disabled={request.contextIds.length>=12&&!request.contextIds.includes(node.id)} onchange={()=>toggleContext(node.id)} /><span>{node.name}</span></label>{/each}</div></details>
      </fieldset>
    {:else}
      <div class="proposal-status"><span>{status}</span><strong>{view?.providerLabel}</strong></div>
      <p class="field-help">{request.direction}</p>
      {#if status==='running'}<p class="ai-running" role="status">Generating a proposal. You can close this panel and keep editing the world.</p><button class="secondary-button" onclick={()=>{void commands.cancelAuthoring(currentId).then(unwrap).then(refresh).catch((caught)=>{error=describe(caught);});}}>Cancel generation</button>{/if}
      {#if view?.stale&&status==='review'}<p class="inline-error" role="alert">The source world or LLM settings changed. Start a new candidate from the current source.</p>{/if}
      {#if view?.error}<p class="inline-error" role="alert">{view.error}</p>{/if}
      {#if status==='review'}
        {#if view?.warnings.length}<div class="story-warning" role="status">{#each view.warnings as warning}<p>{warning}</p>{/each}</div>{/if}
        {#if validShape&&candidate&&view}<CandidateEditor {candidate} schema={view.context} {request} onChange={(next)=>{candidateText=JSON.stringify(next,null,2);touch();}} />{/if}
        <details class="candidate-json" open={!validShape}><summary>Edit candidate JSON</summary><textarea aria-label="Candidate JSON" bind:value={candidateText} oninput={touch} rows={12} maxlength={500000} spellcheck={false}></textarea></details>
        {#if view?.changes.length}<div class="change-review"><h3>Changes to apply</h3><ul>{#each view.changes as change}<li>{change}</li>{/each}</ul></div>{/if}
      {/if}
      {#if view?.rawResponse}<details class="candidate-json"><summary>Original provider output</summary><textarea aria-label="Original provider output" readonly value={view.rawResponse} rows={7}></textarea></details>{/if}
      {#if status==='applied'}<p class="field-help">Applied at revision {view?.appliedRevision}.</p>{/if}
    {/if}
  </div>
  <footer class="settings-footer"><div><span class="field-help" aria-live="polite">{saveStatus}</span><button class="text-action" disabled={status==='running'||working} onclick={()=>{void close('llm');}}>LLM settings</button><button class="text-action" disabled={status==='running'||working} onclick={()=>{void close('prompts');}}>Prompts</button></div><div class="ai-footer-actions">
    {#if !['running','applied'].includes(status)}<DeleteDraftButton title={request.direction||'Saved AI draft'} disabled={working||!loaded} onDelete={()=>deleteDraft(currentId,version)}/>{/if}
    {#if status==='draft'}<button class="primary-button" disabled={working||!loaded||(request.mode==='create'&&!request.direction.trim())||(request.mode==='revise'&&!request.fields.length)} onclick={()=>{void generate();}}>{working?'Starting…':request.mode==='revise'?'Generate details':'Generate proposal'}</button>
    {:else if status!=='running'}<button class="secondary-button" disabled={working} onclick={()=>{void newDraft();}}>New candidate / reroll</button>{#if status==='review'}<button class="primary-button" disabled={working||!!view?.error||!!view?.stale||!view?.changes.length} onclick={()=>{void apply();}}>{working?'Applying…':'Apply proposal'}</button>{/if}{/if}
  </div></footer>
</dialog>
