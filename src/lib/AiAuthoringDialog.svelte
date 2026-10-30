<script lang="ts">
  import { onMount,untrack } from 'svelte';
  import type { ApplyAuthoringResult_Serialize,AuthoringRequest,AuthoringSummary,AuthoringView_Serialize } from './bindings';
  import type { ProjectView,World } from './model';
  import { commands,unwrap } from './native';
  import { editableFields,fieldId,editableLinkFields,linkFieldId,newAuthoringRequest,parseCandidateForEditor,changeCreationType,creationChoice,toggleCreationType,foundationCreationTypes } from './authoring-ui';
  import CandidateEditor from './CandidateEditor.svelte';
  import DeleteDraftButton from './DeleteDraftButton.svelte';
  let {project,world,focusId=null,initialId,initialRequest,onBeforeAction,onApply,onClose,onSettings,onRun}:{project:ProjectView;world:World;focusId?:string|null;initialId:string|null;initialRequest:AuthoringRequest;onBeforeAction:()=>Promise<ProjectView>;onApply:(result:ApplyAuthoringResult_Serialize)=>Promise<void>;onClose:()=>void;onSettings:(section?:'llm'|'prompts')=>void;onRun:(id:string,running:boolean)=>void}=$props();
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
  const linkFields=$derived(editableLinkFields(world,request.linkTargetIds??[]));
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
  function narration(enabled:boolean){request.creationScope='world';request.includeNarration=enabled;if(enabled&&request.creationTypeIds?.length&&!world.nodeTypes.some(t=>request.creationTypeIds!.includes(t.id)&&t.components.includes('core.narration-style'))){const type=world.nodeTypes.find(t=>t.components.includes('core.narration-style'));if(type)request.creationTypeIds.push(type.id);}touch();}
  function toggleLinkField(field:typeof linkFields[number]){const key=linkFieldId(field);request.linkFields=(request.linkFields??[]).some(f=>linkFieldId(f)===key)?request.linkFields?.filter(f=>linkFieldId(f)!==key):[...(request.linkFields??[]),{linkId:field.linkId,property:field.property}];touch();}
  function toggleField(field:typeof fields[number]){const key=fieldId(field);request.fields=request.fields.some((f)=>fieldId(f)===key)?request.fields.filter((f)=>fieldId(f)!==key):[...request.fields,{nodeId:field.nodeId,componentId:field.componentId,key:field.key}];touch();}
  onMount(()=>{void(async()=>{try{if(initialId)await refresh();else{loaded=true;touch();await flushDraft();}loaded=true;await list();}catch(caught){error=describe(caught);}})();return()=>{clearTimeout(timer);clearTimeout(poll);};});
</script>
<dialog bind:this={dialog} class="project-dialog ai-dialog" aria-labelledby="ai-title" oncancel={(event)=>{event.preventDefault();void close();}}>
  <header class="settings-heading"><h2 id="ai-title">AI world tools</h2><button class="icon-button" aria-label="Close AI world tools" onclick={()=>{void close();}}>×</button></header>
  <div class="ai-body">
    <div class="proposal-toolbar">
      <details class="saved-proposals"><summary>Saved proposals ({saved.length})</summary>{#each saved as entry (entry.id)}{@const title=(entry.id===currentId?request.direction:entry.direction)||'Saved AI draft'}<div class="saved-draft-row"><button class:active={entry.id===currentId} onclick={()=>{void openSaved(entry.id);}} disabled={working||status==='running'}><span>{title}</span><small>{entry.id===currentId?status:entry.status}</small></button>{#if entry.id!==currentId&&!['running','applied'].includes(entry.status)}<DeleteDraftButton {title} disabled={working||!loaded||status==='running'} onDelete={()=>deleteDraft(entry.id,entry.version)}/>{/if}</div>{/each}</details>
      <button class="text-action" disabled={!loaded||working||status==='running'} onclick={()=>{void newDraft(newAuthoringRequest(world,undefined,focusId));}}>New creation</button>
    </div>
    {#if error}<div class="settings-error" role="alert">{error}<button class="quiet-button" onclick={()=>{void flushDraft().then(refresh).catch((caught)=>{error=describe(caught);});}}>Retry / refresh</button></div>{/if}
    {#if request.mode==='create'&&request.creationScope==='group'}
      {#if view?.existingContext}{@const included=view.existingContext}{@const contextWorld=view.context}<details class="existing-world-context"><summary>Existing world context · {included.catalog.length}/{included.totalNodes} indexed · {included.detailedNodeIds.length} full records</summary><p class="field-help">Read-only context for reusing existing people, places, items and containers. Existing records cannot be rewritten by this proposal.</p>{#if included.omittedNodes||included.omittedDetails||included.omittedLinks}<p class="field-help">{included.omittedNodes} nodes outside the index · {included.omittedDetails} relevant records without full details · {included.omittedLinks} additional connections omitted. Select Context references to require full details.</p>{/if}<ul>{#each included.catalog as entry}<li>{entry.name} <small>{contextWorld.nodeTypes.find(t=>t.id===entry.typeId)?.name??entry.typeId} · {included.detailedNodeIds.includes(entry.id)?'Full details':'Index only'}</small>{#if entry.placement}<small>{contextWorld.relationTypes.find(r=>r.id===entry.placement?.typeId)?.label??entry.placement.typeId} → {included.catalog.find(n=>n.id===entry.placement?.targetId)?.name??contextWorld.nodes.find(n=>n.id===entry.placement?.targetId)?.name??'Existing node'}</small>{/if}</li>{/each}</ul></details>
      {:else}<p class="field-help">Group creation checks a compact existing-world index and relevant details automatically. Context references require full details; the current selection starts checked.</p>{/if}
    {/if}
    {#if status==='draft'}
      <fieldset class="connection-form" disabled={!loaded||working}>
        {#if request.mode==='create'}
          <label class="field">Create<select value={creationChoice(request)} onchange={(event)=>{request=changeCreationType(world,request,event.currentTarget.value);touch();}}><option value="@world">Build a world from an idea</option><option value="@group">Create a connected group</option><optgroup label="Node with optional related records">{#each world.nodeTypes as type}<option value={type.id}>{type.name}</option>{/each}</optgroup></select></label>
          {#if request.creationScope==='group'}<label class="field">Group node types<select value={request.groupTypeId??''} onchange={event=>{request.groupTypeId=event.currentTarget.value||null;request.creationTypeIds=request.groupTypeId?undefined:foundationCreationTypes(world);touch();}}><option value="">Mixed types, as requested</option>{#each world.nodeTypes as type}<option value={type.id}>{type.name} only</option>{/each}</select></label><p class="field-help">Create the requested set using existing-world context. Add Context references for people, places or belongings whose full details must be included.</p>
          {:else if !request.typeId}<p class="field-help">Create connected places, developed characters and their belongings. Distinct buildings and scene areas become separate places. Review additions before Apply.</p>{#if world.nodeTypes.some(t=>t.components.includes('core.narration-style'))}<label class="check-field"><input type="checkbox" checked={request.includeNarration??false} onchange={e=>narration(e.currentTarget.checked)}/><span>Create narration style and include it for passage writing and wording edits on Apply</span></label>{/if}
          {:else}<label class="check-field"><input type="checkbox" checked={request.bundle} onchange={(event)=>{request.bundle=event.currentTarget.checked;touch();}} /><span>Include outfit items, belongings and related nodes</span></label>{/if}
          {#if !request.typeId&&!request.groupTypeId}<details class="reference-picker"><summary>Creation types ({request.creationTypeIds?.length||world.nodeTypes.length})</summary><p class="field-help">Only selected types are offered to the model. Add other core or custom types here when needed.</p><div>{#each world.nodeTypes as type}<label><input type="checkbox" checked={!request.creationTypeIds?.length||request.creationTypeIds.includes(type.id)} disabled={request.creationTypeIds?.length===1&&request.creationTypeIds.includes(type.id)} onchange={()=>{request=toggleCreationType(world,request,type.id);touch();}}/><span>{type.name}</span></label>{/each}</div></details>{/if}
        {:else}<div class="field"><strong>Generate / revise {world.nodes.find((n)=>n.id===request.targetIds[0])?.name??'relationship'}</strong><span class="field-help">Fill blank fields or revise existing details. Only checked fields may change; unchecked fields stay protected.</span></div><div class="editable-field-list">{#each fields as field}<label><input type="checkbox" checked={request.fields.some((f)=>fieldId(f)===fieldId(field))} onchange={()=>toggleField(field)} /><span>{field.label}</span></label>{/each}</div>{#if linkFields.length}<p class="field-help">Relationship identity, type and endpoints are protected.</p><div class="editable-fields">{#each linkFields as field}<label><input type="checkbox" checked={(request.linkFields??[]).some(f=>linkFieldId(f)===linkFieldId(field))} onchange={()=>toggleLinkField(field)}/><span>{field.label}</span></label>{/each}</div>{/if}{#if world.aiContext?.some(b=>request.targetIds.includes(b.nodeId))}<p class="field-help">This node's included context is editable source for this request. Other required context still applies.</p>{/if}{/if}
        <label class="field">{request.mode==='revise'?'Direction (optional)':'Direction'}<textarea bind:value={request.direction} oninput={touch} rows={4} maxlength={12000} placeholder={request.mode==='revise'?"Optional: specify what to fill in or change. Leave blank to develop checked fields from the selected records and context.":request.creationScope==='group'?"For example: a field kit with a lantern, blanket and tin of supplies for the selected character.":"Describe the node or story idea you want to develop…"}></textarea></label>
        <details class="reference-picker"><summary>Context references ({request.contextIds.length}/12)</summary><input aria-label="Find context references" placeholder="Find a node…" bind:value={contextSearch} /><div>{#each world.nodes.filter((n)=>!request.targetIds.includes(n.id)&&n.name.toLowerCase().includes(contextSearch.toLowerCase())).slice(0,100) as node}<label><input type="checkbox" checked={request.contextIds.includes(node.id)} disabled={request.contextIds.length>=12&&!request.contextIds.includes(node.id)} onchange={()=>toggleContext(node.id)} /><span>{node.name}</span></label>{/each}</div></details>
      </fieldset>
    {:else}
      <div class="proposal-status"><span>{status}</span><strong>{view?.providerLabel}</strong></div>
      <p class="field-help">{request.direction}</p>
      {#if candidate&&request.mode==='create'}<p class="field-help">{candidate.nodes.filter(n=>n.components['core.place']).length} places · {candidate.nodes.filter(n=>n.components['core.character']).length} characters · {candidate.nodes.filter(n=>n.components['core.item']).length} items · {candidate.nodes.filter(n=>n.components['core.narration-style']).length} narration styles</p>{#if request.includeNarration}<p class="field-help">New narration styles are included as Instructions for passage writing and wording edits only when you Apply.</p>{/if}{/if}
      {#if status==='running'}<p class="ai-running" role="status">Generating a proposal. You can close this panel and keep editing the world.</p><button class="secondary-button" onclick={()=>{void commands.cancelAuthoring(currentId).then(unwrap).then(refresh).catch((caught)=>{error=describe(caught);});}}>Cancel generation</button>{/if}
      {#if view?.stale&&status==='review'}<p class="inline-error" role="alert">The source world or LLM settings changed. Start a new candidate from the current source.</p>{/if}
      {#if view?.error}<p class="inline-error" role="alert">{view.error}</p>{/if}
      {#if status==='review'}
        {#if view?.warnings.length}<div class="story-warning" role="status">{#each view.warnings as warning}<p>{warning}</p>{/each}</div>{/if}
        {#if validShape&&candidate&&view}<CandidateEditor {candidate} schema={view.context} {request} existingNodes={view.existingContext?.catalog??[]} onChange={(next)=>{candidateText=JSON.stringify(next,null,2);touch();}} />{/if}
        <details class="candidate-json" open={!validShape}><summary>Edit candidate JSON</summary><textarea aria-label="Candidate JSON" bind:value={candidateText} oninput={touch} rows={12} maxlength={500000} spellcheck={false}></textarea></details>
        {#if view?.changes.length}<div class="change-review"><h3>Changes to apply</h3><ul>{#each view.changes as change}<li>{change}</li>{/each}</ul></div>{/if}
      {/if}
      {#if view?.rawResponse}<details class="candidate-json"><summary>Original provider output</summary><textarea aria-label="Original provider output" readonly value={view.rawResponse} rows={7}></textarea></details>{/if}
      {#if status==='applied'}<p class="field-help">Applied at revision {view?.appliedRevision}.</p>{/if}
    {/if}
  </div>
  <footer class="settings-footer"><div><span class="field-help" aria-live="polite">{saveStatus}</span><button class="text-action" disabled={status==='running'||working} onclick={()=>{void close('llm');}}>LLM settings</button><button class="text-action" disabled={status==='running'||working} onclick={()=>{void close('prompts');}}>Prompts</button></div><div class="ai-footer-actions">
    {#if !['running','applied'].includes(status)}<DeleteDraftButton title={request.direction||'Saved AI draft'} disabled={working||!loaded} onDelete={()=>deleteDraft(currentId,version)}/>{/if}
    {#if status==='draft'}<button class="primary-button" disabled={working||!loaded||(request.mode==='create'&&!request.direction.trim())||(request.mode==='revise'&&!request.fields.length&&!(request.linkFields?.length))} onclick={()=>{void generate();}}>{working?'Starting…':request.mode==='revise'?'Generate details':'Generate proposal'}</button>
    {:else if status!=='running'}<button class="secondary-button" disabled={working} onclick={()=>{void newDraft();}}>New candidate / reroll</button>{#if status==='review'}<button class="primary-button" disabled={working||!!view?.error||!!view?.stale||!view?.changes.length} onclick={()=>{void apply();}}>{working?'Applying…':'Apply proposal'}</button>{/if}{/if}
  </div></footer>
</dialog>
