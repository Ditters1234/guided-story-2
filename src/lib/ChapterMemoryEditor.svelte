<script lang="ts">
  import {onMount,untrack,type Snippet} from 'svelte';
  import {commands,unwrap} from './native';
  import {MemorySession} from './memory-session.svelte';
  import type {ProjectView} from './model';
  import type {ChapterMemoryStatus,MemoryDraftSummary,CommitResult_Serialize} from './bindings';
  let {project,otherBusy,onBeforeAction,onApply,onRun,onDetails}:{project:ProjectView;otherBusy:boolean;onBeforeAction:()=>Promise<ProjectView>;onApply:(result:CommitResult_Serialize)=>Promise<void>;onRun:(running:boolean)=>void;onDetails:(panel?:Snippet)=>void}=$props();
  const session=new MemorySession(untrack(()=>project.storyId));
  let chapters=$state<ChapterMemoryStatus[]>([]),drafts=$state<MemoryDraftSummary[]>([]),chapterId=$state('');
  let working=$state(false),loaded=$state(false),error=$state(''),sourceId=$state(''),sourceText=$state(''),sourceError=$state('');
  const blocked=$derived(working||session.running),applied=$derived(session.view?.stage==='applied');
  const selectedChapter=$derived(chapters.find(c=>c.chapterId===chapterId));
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  $effect(()=>{onRun(session.running);});
  $effect(()=>{const revision=project.revision;if(loaded)untrack(()=>{void refresh().catch(caught=>{error=describe(caught);});});void revision;});
  export async function flushDraft(){await session.flush();}
  export async function refresh(){chapters=unwrap(await commands.chapterMemoryStatus(project.storyId));drafts=unwrap(await commands.listMemoryDrafts(project.storyId));await session.refresh();}
  async function showSource(id:string){sourceId=id;sourceText='';sourceError='';if(!session.id||!id)return;const draftId=session.id;try{const text=unwrap(await commands.memorySourcePassage(project.storyId,draftId,id));if(session.id===draftId&&sourceId===id)sourceText=text;}catch(caught){sourceError=describe(caught);}}
  async function openDraft(id:string){working=true;try{await session.open(id);chapterId=session.view!.chapterId;await showSource(session.view!.sources[0]?.passageId??'');error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function fresh(){if(!chapterId)return;working=true;try{await session.flush();await onBeforeAction();await session.create(chapterId);await refresh();await showSource(session.view!.sources[0]?.passageId??'');error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function chooseChapter(id:string){working=true;try{await session.clear();chapterId=id;sourceId='';sourceText='';sourceError='';if(chapters.find(c=>c.chapterId===id)?.closed){await session.create(id);await refresh();await showSource(session.view!.sources[0]?.passageId??'');}error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function generate(){working=true;error='';try{
    await session.flush();const direction=session.value?.direction??'';
    if(!session.id||session.value?.output.points.length||session.view?.originalOutput||applied){await session.create(chapterId);session.value!.direction=direction;session.touch();await session.flush();}
    const current=await onBeforeAction();if(!session.id||!session.view)throw Error('Create a memory candidate first.');
    session.adopt(unwrap(await commands.startMemoryJob(project.storyId,session.id,session.view.version,current.revision)));await refresh();await showSource(session.view!.sources[0]?.passageId??'');
  }catch(caught){error=describe(caught);try{await session.refresh();}catch{}}finally{working=false;}}
  async function apply(){working=true;error='';try{await session.flush();await onBeforeAction();if(!session.id||!session.view)return;const result=unwrap(await commands.applyMemory(project.storyId,session.id,session.view.version));await onApply(result);await refresh();}catch(caught){error=describe(caught);try{await refresh();}catch{}}finally{working=false;}}
  async function parse(){working=true;error='';try{await session.flush();if(session.id&&session.view)session.adopt(unwrap(await commands.parseMemoryJson(project.storyId,session.id,session.view.version)));}catch(caught){error=describe(caught);}finally{working=false;}}
  function addPoint(){if(!session.value||!session.view)return;session.value.output.noLastingFacts=false;session.value.output.points.push({text:'',passageId:sourceId||session.view.sources[0]?.passageId||'',evidence:'',nodeIds:[]});session.touch();}
  function related(pointIndex:number,id:string){const point=session.value?.output.points[pointIndex];if(!point)return;point.nodeIds=point.nodeIds.includes(id)?point.nodeIds.filter(n=>n!==id):[...point.nodeIds,id];session.touch();}
  onMount(()=>{onDetails(sourcePanel);let disposed=false;void(async()=>{try{await refresh();chapterId=chapters[0]?.chapterId??'';const existing=drafts.find(d=>d.chapterId===chapterId);if(existing)await openDraft(existing.id);if(!disposed)loaded=true;}catch(caught){error=describe(caught);}})();
    const poll=setInterval(()=>{if(session.running)void session.refresh().catch(caught=>{error=describe(caught);});},900);
    return()=>{disposed=true;clearInterval(poll);session.dispose();onDetails();};
  });
</script>

{#snippet sourcePanel()}
  <h2>Memory sources</h2>
  <p class="field-help">Historical context supports continuity. Current world records remain authoritative; these facts are not automatically known to every character.</p>
  {#if session.view}
    <label class="field">Source passage<select value={sourceId} onchange={event=>{void showSource(event.currentTarget.value);}}>{#each session.view.sources as source}<option value={source.passageId}>{source.title}</option>{/each}</select></label>
    {#if sourceError}<p class="story-warning" role="alert">{sourceError}</p>{:else}<div class="memory-source-text">{sourceText||'Loading source…'}</div>{/if}
  {:else}<p class="field-help">Open or create a memory candidate to inspect its accepted source text.</p>{/if}
{/snippet}

<section class="memory-editor" aria-label="Chapter memory editor">
  <header class="memory-toolbar"><h2>Chapter memory</h2><label>Chapter<select bind:value={chapterId} disabled={blocked} onchange={event=>{void chooseChapter(event.currentTarget.value);}}>{#each chapters as chapter}<option value={chapter.chapterId}>{chapter.title} · {chapter.status}</option>{/each}</select></label><button class="secondary-button" disabled={blocked||!selectedChapter?.closed} onclick={()=>{void fresh();}}>New candidate</button></header>
  {#if error||session.error}<div class="story-error" role="alert">{error||session.error}<button class="text-action" onclick={()=>{void session.flush().then(refresh).then(()=>{error='';}).catch(caught=>{error=describe(caught);});}}>Retry / refresh</button></div>{/if}
  {#if session.running}<div class="story-job" role="status"><span>Preparing chapter memory… You can browse while this runs.</span><button class="secondary-button" onclick={()=>{if(session.id)void commands.cancelAuthoring(session.id).then(unwrap).then(()=>session.refresh()).catch(caught=>{error=describe(caught);});}}>Cancel</button></div>{/if}
  <div class="story-scroll">
    {#if !chapters.length}<p class="field-help">Close a chapter to prepare its summary. Accepted passage notes can still provide older context.</p>{/if}
    {#if selectedChapter&&!selectedChapter.closed}<p class="field-help">This chapter has reopened. Close it before creating replacement memory; its older summary is excluded.</p>{/if}
    {#if drafts.length}<details class="memory-saved"><summary>Saved candidates ({drafts.length})</summary>{#each drafts as draft}<button class="secondary-button" disabled={blocked} onclick={()=>{void openDraft(draft.id);}}>{draft.title} · {draft.stage}</button>{/each}</details>{/if}
    {#if session.value&&session.view}
      <p class="field-help">{session.view.title} · {session.view.sources.length} accepted source passages. Applying memory leaves story events unchanged.</p>
      {#if session.view.stale}<p class="story-warning">The chapter changed. Keep this draft and create a candidate from the current chapter before applying.</p>{/if}
      {#if session.view.seededFromStale}<p class="field-help">Started from an older summary. Review the facts and update their source quotes.</p>{/if}
      {#if applied}<p class="field-help">Applied chapter memory. Create a new candidate to revise it.</p>{/if}
      <fieldset class="passage-fields" disabled={blocked||applied}>
        <label class="field">Summary direction (optional)<textarea bind:value={session.value.direction} oninput={()=>session.touch()} rows={2} maxlength={4000} placeholder="Which events or threads matter for later chapters?"></textarea></label>
        <div class="effects-actions"><button class="secondary-button" disabled={otherBusy||!selectedChapter?.closed} onclick={()=>{void generate();}}>Generate summary</button><button class="text-action" disabled={session.value.output.points.length>=12} onclick={addPoint}>＋ Add memory point</button></div>
        {#if session.view.provider}<p class="field-help">{session.view.provider}</p>{/if}
        <label class="check-field"><input type="checkbox" bind:checked={session.value.output.noLastingFacts} disabled={!!session.value.output.points.length} onchange={()=>session.touch()} /><span>No lasting facts to summarize</span></label>
        {#each session.value.output.points as point,i}<section class="memory-point"><div class="section-actions"><h3>Point {i+1}</h3><button class="text-action" onclick={()=>{session.value!.output.points.splice(i,1);session.touch();}}>Remove point</button></div>
          <label class="field">Historical fact<textarea bind:value={point.text} rows={3} maxlength={1000} oninput={()=>session.touch()}></textarea></label>
          <label class="field">Source passage<select bind:value={point.passageId} onchange={()=>{session.touch();void showSource(point.passageId);}}>{#each session.view.sources as source}<option value={source.passageId}>{source.title}</option>{/each}</select></label>
          <label class="field">Exact quote<textarea bind:value={point.evidence} rows={2} maxlength={600} oninput={()=>session.touch()}></textarea></label>
          <details><summary>Related nodes ({point.nodeIds.length})</summary><div class="memory-node-list">{#each session.view.nodes as node}<label><input type="checkbox" checked={point.nodeIds.includes(node.id)} disabled={point.nodeIds.length>=12&&!point.nodeIds.includes(node.id)} onchange={()=>related(i,node.id)} />{node.name}</label>{/each}</div></details>
        </section>{/each}
        <details class="memory-json"><summary>Response JSON / manual repair</summary><textarea aria-label="Memory response JSON" bind:value={session.value.rawJson} oninput={()=>session.touch()} rows={8} maxlength={100000} spellcheck={false}></textarea><button class="secondary-button" onclick={()=>{void parse();}}>Parse response JSON</button></details>
      </fieldset>
      {#if session.view.error}<p class="field-help" role="status">{session.view.error}</p>{/if}
      {#if session.view.originalOutput}<details class="original-passage-output"><summary>Original model output</summary><pre>{session.view.originalOutput}</pre></details>{/if}
    {:else if chapters.length}<p class="field-help">Choose a completed chapter and create a candidate. Write memory points manually or generate a proposal, then review and apply.</p>{/if}
  </div>
  <footer class="composer-footer"><span aria-live="polite">{session.error?'Draft not saved':session.saving?'Saving draft…':session.dirty?'Unsaved draft':session.running?'Request running':applied?'Memory applied':session.readyToApply?'Ready for review':'Draft saved'}</span><button class="primary-button" disabled={blocked||!session.readyToApply} onclick={()=>{void apply();}}>Apply memory</button></footer>
</section>
