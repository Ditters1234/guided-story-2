<script lang="ts">
  import {onMount,untrack,type Snippet} from 'svelte';
  import {commands,unwrap} from './native';
  import {PassageSession} from './passage-session.svelte';
  import type {ProjectView,World} from './model';
  import type {PassageAcceptance_Serialize,PassageReading_Serialize,PassageSummary,StoryIndex,PassagePlan} from './bindings';
  let {project,world,selectedId,otherBusy,onBeforeAction,onAccept,onHistory,onRun,onFork,onSettings,onPanels}:{
    project:ProjectView;world:World;selectedId:string|null;otherBusy:boolean;
    onBeforeAction:()=>Promise<ProjectView>;onAccept:(result:PassageAcceptance_Serialize)=>Promise<void>;
    onHistory:(snapshot:World|null,label:string)=>void;onRun:(running:boolean)=>void;onFork:(id:string)=>Promise<void>;onSettings:()=>void;
    onPanels:(contents?:Snippet,details?:Snippet)=>void;
  }=$props();
  const session=new PassageSession(untrack(()=>project.storyId));
  let index=$state<StoryIndex>({schemaVersion:1,chapters:[],passages:[]}),drafts=$state<PassageSummary[]>([]);
  let reading=$state<PassageReading_Serialize|null>(null),mode=$state<'draft'|'reader'>('draft'),moment=$state<'before'|'after'>('after');
  let loaded=$state(false),working=$state(false),error=$state(''),referenceSearch=$state('');
  const selection=$derived(index.passages.findIndex((p)=>p.id===reading?.passage.id));
  const blocked=$derived(working||session.running);
  const actors=$derived(world.nodes.filter((n)=>!!n.components['core.character']));
  const places=$derived(world.nodes.filter((n)=>!!n.components['core.place']));
  const wordCount=$derived(session.value?.prose.trim().split(/\s+/).filter(Boolean).length??0);
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  $effect(()=>{onRun(session.running);});
  $effect(()=>{const revision=project.revision;if(loaded)untrack(()=>{void refreshIndex().then(()=>session.refresh()).catch((caught)=>{error=describe(caught);});});void revision;});
  export async function flushDraft(){await session.flush();}
  async function refreshIndex(){index=unwrap(await commands.loadStory(project.storyId));drafts=unwrap(await commands.listPassageDrafts(project.storyId));}
  async function openDraft(id:string){working=true;try{await session.open(id);reading=null;mode='draft';onHistory(null,'');error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function fresh(usePlan?:PassagePlan){working=true;try{
    await session.flush();await onBeforeAction();const plan=usePlan?structuredClone(usePlan):unwrap(await commands.nextPassagePlan(project.storyId));
    if(!usePlan){const previous=session.value?.plan;const selected=world.nodes.find((n)=>n.id===selectedId);
      plan.viewpointId=previous?.viewpointId&&world.nodes.some((n)=>n.id===previous.viewpointId)?previous.viewpointId:selected?.components['core.character']?selected.id:null;
      plan.locationId=previous?.locationId&&world.nodes.some((n)=>n.id===previous.locationId)?previous.locationId:selected?.components['core.place']?selected.id:world.edges.find((e)=>e.typeId==='core.present-at'&&e.sourceId===plan.viewpointId)?.targetId??places[0]?.id??null;
      plan.participantIds=(previous?.participantIds??[]).filter((id)=>world.nodes.some((n)=>n.id===id));
    }
    await session.create(plan);reading=null;mode='draft';onHistory(null,'');await refreshIndex();error='';
  }catch(caught){error=describe(caught);}finally{working=false;}}
  async function read(id:string){working=true;try{await session.flush();reading=unwrap(await commands.readPassage(project.storyId,id));mode='reader';moment='after';showMoment();error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  function showMoment(){if(reading)onHistory(reading[moment],`${moment==='before'?'Before':'After'} ${reading.passage.plan.title}`);}
  async function start(checking:boolean){working=true;error='';try{
    await session.flush();if(!checking&&session.value?.prose.trim()){const plan=structuredClone($state.snapshot(session.value.plan));await session.create(plan);await refreshIndex();}
    const current=await onBeforeAction();if(!session.id||!session.view)throw Error('Create a passage draft first.');
    const result=unwrap(await commands.startPassageJob(project.storyId,session.id,session.view.version,current.revision,checking));session.adopt(result);
  }catch(caught){error=describe(caught);try{await session.refresh();}catch{}}finally{working=false;}}
  async function validate(){working=true;error='';try{await session.flush();await onBeforeAction();if(!session.id||!session.view)return;session.adopt(unwrap(await commands.validatePassage(project.storyId,session.id,session.view.version)));}catch(caught){error=describe(caught);}finally{working=false;}}
  async function accept(){working=true;error='';try{
    await session.flush();await onBeforeAction();if(!session.id||!session.view)return;
    const result=unwrap(await commands.acceptPassage(project.storyId,session.id,session.view.version));await onAccept(result);await session.refresh();await refreshIndex();await read(result.passageId);
  }catch(caught){error=describe(caught);try{await session.refresh();await refreshIndex();}catch{}}finally{working=false;}}
  function references(id:string,participant=false){if(!session.value)return;const key=participant?'participantIds':'referenceIds';const list=session.value.plan[key];session.value.plan[key]=list.includes(id)?list.filter((v)=>v!==id):[...list,id];session.touch();}
  onMount(()=>{onPanels(contentsPanel,detailsPanel);let disposed=false;void(async()=>{try{await refreshIndex();const unfinished=drafts.find((d)=>d.stage!=='accepted');if(unfinished)await openDraft(unfinished.id);else if(index.passages.length)await read(index.passages.at(-1)!.id);else await fresh();if(!disposed)loaded=true;}catch(caught){error=describe(caught);}})();
    const poll=setInterval(()=>{if(session.running)void session.refresh().catch((caught)=>{error=describe(caught);});},900);
    return()=>{disposed=true;clearInterval(poll);session.dispose();onPanels();};
  });
</script>

{#snippet contentsPanel()}
  <aside class="story-sidebar" aria-label="Chapters and passages">
    <div class="world-heading"><h1>Story</h1><span>{index.passages.length}</span></div>
    <button class="primary-button next-passage" disabled={blocked} onclick={()=>{void fresh();}}>Write next passage</button>
    <nav class="story-chapters" aria-label="Story contents">
      {#each index.chapters as chapter}<details open><summary>{chapter.title}{#if chapter.closed}<small>Closed</small>{/if}</summary>
        {#each chapter.passageIds as id}{@const entry=index.passages.find((p)=>p.id===id)}<button class:active={mode==='reader'&&reading?.passage.id===id} aria-current={mode==='reader'&&reading?.passage.id===id?'page':undefined} disabled={working} onclick={()=>{void read(id);}}>{entry?.title??'Passage'}</button>{/each}
      </details>{:else}<p class="field-help">No accepted passages yet.</p>{/each}
    </nav>
    {#if drafts.length}<section class="story-drafts"><h2>Drafts</h2>{#each drafts as draft}{@const current=session.id===draft.id}<button class:active={mode==='draft'&&current} disabled={blocked} onclick={()=>{void openDraft(draft.id);}}><span>{(current?session.value?.plan.title:draft.title)||'Untitled passage'}</span><small>{current?session.view?.stage??draft.stage:draft.stage}</small></button>{/each}</section>{/if}
  </aside>
{/snippet}

{#snippet detailsPanel()}
  <aside class="story-details" aria-label={mode==='reader'?'Passage details':'Scene planning and review'}>
  {#if mode==='reader'&&reading}
    <h2>Passage details</h2>
    <p class="field-help">{reading.passage.plan.chapterTitle} · {reading.passage.plan.title}</p>
    <dl class="passage-facts"><dt>Viewpoint</dt><dd>{reading.after.nodes.find((n)=>n.id===reading?.passage.plan.viewpointId)?.name??'Ensemble / unspecified'}</dd><dt>Location</dt><dd>{reading.after.nodes.find((n)=>n.id===reading?.passage.plan.locationId)?.name??'Unspecified'}</dd></dl>
    {#if reading.passage.plan.chapterPlan}<details><summary>Chapter direction</summary><p class="story-plan-text">{reading.passage.plan.chapterPlan}</p></details>{/if}
    <div class="passage-world-controls"><span>World snapshot</span><button class="secondary-button" aria-pressed={moment==='before'} onclick={()=>{moment='before';showMoment();}}>Before</button><button class="secondary-button" aria-pressed={moment==='after'} onclick={()=>{moment='after';showMoment();}}>After</button><p class="field-help">Switch to World to inspect this passage's saved state.</p><button class="text-action" disabled={otherBusy||session.running||working} onclick={()=>{void session.flush().then(()=>onFork(reading!.passage.id)).catch((caught)=>{error=describe(caught);});}}>Fork after this passage…</button></div>
    {#if reading.passage.effects.notes.length}<section class="continuity-notes"><h3>Continuity notes</h3>{#each reading.passage.effects.notes as note}<p>{note.text}</p><blockquote>{note.evidence}</blockquote>{/each}</section>{/if}
  {:else if session.value}
    <h2>Scene plan</h2>
    <fieldset class="passage-fields" disabled={blocked||session.view?.stage==='accepted'}>
      <details class="chapter-plan" open={!session.value.prose}><summary>Chapter plan</summary><label class="field">Chapter title<input bind:value={session.value.plan.chapterTitle} oninput={()=>session.touch()} maxlength={200} /></label><label class="field">Chapter direction<textarea bind:value={session.value.plan.chapterPlan} oninput={()=>session.touch()} rows={3} maxlength={4000}></textarea></label></details>
      <label class="field">Viewpoint<select bind:value={session.value.plan.viewpointId} onchange={()=>session.touch()}><option value={null}>Ensemble / unspecified</option>{#each actors as node}<option value={node.id}>{node.name}</option>{/each}</select></label>
      <label class="field">Location<select bind:value={session.value.plan.locationId} onchange={()=>session.touch()}><option value={null}>Unspecified</option>{#each places as node}<option value={node.id}>{node.name}</option>{/each}</select></label>
      <details class="scene-resources"><summary>Participants & references</summary><input aria-label="Find scene resources" placeholder="Find a world node…" bind:value={referenceSearch} />
        <div class="scene-resource-list">{#each world.nodes.filter((n)=>n.name.toLowerCase().includes(referenceSearch.toLowerCase())).slice(0,100) as node}<div><span>{node.name}</span>{#if node.components['core.character']&&node.id!==session.value.plan.viewpointId}<label><input type="checkbox" checked={session.value.plan.participantIds.includes(node.id)} disabled={session.value.plan.participantIds.length>=12&&!session.value.plan.participantIds.includes(node.id)} onchange={()=>references(node.id,true)} />Participant</label>{/if}<label><input type="checkbox" checked={session.value.plan.referenceIds.includes(node.id)} disabled={session.value.plan.referenceIds.length>=12&&!session.value.plan.referenceIds.includes(node.id)} onchange={()=>references(node.id)} />Reference</label></div>{/each}</div>
      </details>
      {#if session.value.prose.trim()}
        <section class="effects-review"><h3>World changes</h3>
          {#if session.view?.stale}<p class="story-warning">Source world/history or LLM settings changed. Recheck before accepting.</p>{/if}
          {#if session.view?.error}<p class="story-warning" role="alert">{session.view.error}</p>{/if}
          <div class="effects-actions"><button class="secondary-button" disabled={otherBusy} onclick={()=>{void start(true);}}>Check world changes with AI</button><button class="text-action" onclick={()=>{session.value!.effectsText='{"changes":[],"notes":[],"suggestChapterEnd":false}';session.touch();}}>Use no world changes</button></div>
          <details class="passage-effects" open={!!session.view?.error}><summary>Review / edit effects JSON</summary><textarea aria-label="Passage effects JSON" bind:value={session.value.effectsText} oninput={()=>session.touch()} rows={10} maxlength={500000} spellcheck={false}></textarea></details>
          <button class="secondary-button" onclick={()=>{void validate();}}>Validate my effects</button>
          {#if session.view?.changes.length}<div class="passage-changes"><h3>Changes on acceptance</h3><ul>{#each session.view.changes as change}<li>{change}</li>{/each}</ul></div>{:else}<p class="field-help">No world changes in the current effects draft.</p>{/if}
          {#if session.view?.suggestedChapterEnd}<p class="field-help">The AI suggests ending this chapter.</p>{/if}
          <label class="check-field"><input type="checkbox" bind:checked={session.value.closeChapter} onchange={()=>session.touch()} /><span>Close chapter after this passage</span></label>
        </section>
      {/if}
    </fieldset>
    {#if session.view}<details class="context-coverage"><summary>Generation context</summary><p>Prose: {session.view.narrationProvider||'Manual'} · Effects: {session.view.effectsProvider||'Manual'}</p><p>{session.view.contextNodes.join(', ')||'No existing world nodes selected.'}</p><p>Recent text: up to two accepted passages. Older chapters are not yet summarized automatically. Participants do not move until prose establishes a move.</p></details>{/if}
    {#if session.view?.originalProse||session.view?.originalEffects}<details class="original-passage-output"><summary>Original model output</summary>{#if session.view.originalProse}<pre>{session.view.originalProse}</pre>{/if}{#if session.view.originalEffects}<pre>{session.view.originalEffects}</pre>{/if}</details>{/if}
  {:else}<p class="story-loading">Loading story…</p>{/if}
  </aside>
{/snippet}

<section class="story-workspace" aria-label="Story composer and reader">
  {#if error||session.error}<div class="story-error" role="alert"><span>{error||session.error}</span><button class="text-action" onclick={()=>{void session.flush().then(()=>session.refresh()).then(()=>{error='';}).catch((caught)=>{error=describe(caught);});}}>Retry / refresh</button></div>{/if}
  {#if session.running}<div class="story-job" role="status"><span>{session.view?.stage==='narrating'?'Writing passage…':'Checking world changes…'} You can browse the world while this runs.</span><button class="secondary-button" onclick={()=>{if(session.id)void commands.cancelAuthoring(session.id).then(unwrap).then(()=>session.refresh()).catch((caught)=>{error=describe(caught);});}}>Cancel</button></div>{/if}
  <div class="story-scroll">
  {#if mode==='reader'&&reading}
    <article class="passage-reader">
      <div class="reader-navigation"><button class="secondary-button" disabled={working||selection<=0} onclick={()=>{void read(index.passages[selection-1].id);}}>← Previous</button><span>{selection+1} / {index.passages.length}</span><button class="secondary-button" disabled={working||selection>=index.passages.length-1} onclick={()=>{void read(index.passages[selection+1].id);}}>Next →</button></div>
      <p class="chapter-name">{reading.passage.plan.chapterTitle}</p><h2>{reading.passage.plan.title}</h2>{#if reading.passage.narrationProvider.startsWith('Offline demo')}<p class="field-help">{reading.passage.narrationProvider}</p>{/if}<div class="accepted-prose">{reading.passage.prose}</div>
    </article>
  {:else if session.value}
    <div class="passage-composer">
      <fieldset class="passage-fields" disabled={blocked||session.view?.stage==='accepted'}>
        <label class="field">Passage title<input bind:value={session.value.plan.title} oninput={()=>session.touch()} maxlength={200} /></label>
        <label class="field">Passage direction<textarea bind:value={session.value.plan.direction} oninput={()=>session.touch()} rows={4} maxlength={8000} placeholder="What happens in this passage, and where should it stop?"></textarea></label>
        <div class="passage-generate"><label>Target words<input type="number" value={session.value.plan.targetWords} oninput={(event)=>{const n=event.currentTarget.valueAsNumber;if(Number.isFinite(n)&&n>=100&&n<=1500){session.value!.plan.targetWords=Math.round(n);session.touch();}}} onblur={(event)=>{event.currentTarget.value=String(session.value!.plan.targetWords);}} min={100} max={1500} step={50} /></label><button class="secondary-button" disabled={otherBusy||!session.value.plan.direction.trim()} onclick={()=>{void start(false);}}>{session.value.prose?'Generate new candidate':'Generate passage'}</button><button class="text-action" onclick={onSettings}>LLM settings</button></div>
        {#if session.view?.narrationProvider.startsWith('Offline demo')}<p class="field-help">{session.view.narrationProvider}</p>{/if}
        <label class="field prose-field">Passage<textarea bind:value={session.value.prose} oninput={()=>session.touch()} rows={16} maxlength={100000} placeholder="Generate a passage or write it here."></textarea><span class="field-help">{wordCount} words</span></label>
      </fieldset>
    </div>
  {:else}<p class="story-loading">Loading story…</p>{/if}
  </div>
  {#if mode==='draft'}<footer class="composer-footer"><span aria-live="polite">{session.error?'Draft not saved':session.saving?'Saving draft…':session.dirty?'Unsaved draft':session.running?'Request running':session.readyToAccept?'Checked · ready for review':'Draft saved'}</span><button class="primary-button" disabled={blocked||!session.readyToAccept} onclick={()=>{void accept();}}>Accept passage</button></footer>{/if}
</section>
