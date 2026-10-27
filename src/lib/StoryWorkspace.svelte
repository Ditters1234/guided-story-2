<script lang="ts">
  import {onMount,untrack,type Snippet} from 'svelte';
  import {commands,unwrap} from './native';
  import {PassageSession} from './passage-session.svelte';
  import {diffProse} from './prose-diff';
  import {scenePlan} from './scene-plan';
  import {parseEffects} from './effects-ui';
  import EffectsReviewEditor from './EffectsReviewEditor.svelte';
  import ChapterMemoryEditor from './ChapterMemoryEditor.svelte';
  import PassageImages from './PassageImages.svelte';
  import type {ProjectView,World} from './model';
  import type {PassageAcceptance_Serialize,PassageReading_Serialize,PassageSummary,StoryIndex,PassagePlan,PassageRevisionKind,PassageVersions,CommitResult_Serialize,PassageEffects} from './bindings';
  let {project,world,selectedId,otherBusy,onBeforeAction,onAccept,onMemoryApply,onImage,onHistory,onRun,onFork,onSettings,onPanels}:{
    project:ProjectView;world:World;selectedId:string|null;otherBusy:boolean;
    onBeforeAction:()=>Promise<ProjectView>;onAccept:(result:PassageAcceptance_Serialize)=>Promise<void>;
    onMemoryApply:(result:CommitResult_Serialize)=>Promise<void>;
    onImage:(passage:PassageReading_Serialize,moment:'before'|'after')=>void;
    onHistory:(snapshot:World|null,label:string)=>void;onRun:(running:boolean)=>void;onFork:(id:string)=>Promise<void>;onSettings:()=>void;
    onPanels:(contents?:Snippet,details?:Snippet)=>void;
  }=$props();
  const session=new PassageSession(untrack(()=>project.storyId));
  let index=$state<StoryIndex>({schemaVersion:1,chapters:[],passages:[],memories:{},media:{},portraits:{}}),drafts=$state<PassageSummary[]>([]);
  let reading=$state<PassageReading_Serialize|null>(null),mode=$state<'draft'|'reader'|'memory'>('draft'),moment=$state<'before'|'after'>('after');
  let memoryOpened=$state(false),memoryRunning=$state(false),memoryDetails=$state<Snippet>();
  let memoryPanel=$state<{flushDraft:()=>Promise<void>;refresh:()=>Promise<void>}>();
  function registerMemoryDetails(panel?:Snippet){memoryDetails=panel;}
  async function openMemory(){try{await session.flush();mode='memory';memoryOpened=true;onHistory(null,'');}catch(caught){error=describe(caught);}}
  let loaded=$state(false),working=$state(false),error=$state(''),referenceSearch=$state('');
  let versions=$state<PassageVersions|null>(null),versionsError=$state('');
  const selection=$derived(index.passages.findIndex((p)=>p.id===reading?.passage.id));
  const blocked=$derived(working||session.running);
  const sceneWorld=$derived(session.view?.revision?.world??world);
  const wording=$derived(session.view?.revision?.kind==='wording');
  const replacing=$derived(session.view?.revision?.kind==='events');
  const actors=$derived(sceneWorld.nodes.filter((n)=>!!n.components['core.character']));
  const places=$derived(sceneWorld.nodes.filter((n)=>!!n.components['core.place']));
  const textChanges=$derived(diffProse(session.view?.revision?.prose??'',session.value?.prose??''));
  const wordCount=$derived(session.value?.prose.trim().split(/\s+/).filter(Boolean).length??0);
  const hasProse=$derived(!!session.value?.prose.trim());
  const proposedEffects=$derived(parseEffects(session.value?.effectsText??''));
  const reviewEntries=$derived(session.view?.effectsReview.entries??[]);
  const reviewPending=$derived(reviewEntries.filter(e=>!(session.value?.approvedEffects??[]).includes(e.key)).length);
  const proposalsLoaded=$derived(!!session.view?.originalEffects||!!proposedEffects?.changes.length||!!proposedEffects?.notes.length);
  const effectsChecked=$derived(!!session.view?.effectsReview.checked);
  const generationHelp=$derived(otherBusy?'Finish the other AI request before generating a passage.':memoryRunning?'Finish chapter memory generation first.':wording&&!session.value?.plan.direction.trim()?'Add wording direction before generating a polish candidate.':'');
  const acceptanceHint=$derived.by(()=>{
    if(session.view?.stage==='accepted')return 'Passage accepted.';
    if(session.running)return session.view?.stage==='checking'?'Draft not accepted · checking world changes…':'Draft not accepted · generating candidate…';
    if(session.error)return 'Draft not saved · retry before accepting.';
    if(session.saving||session.dirty)return 'Saving draft · not accepted.';
    if(session.readyToAccept)return wording?'Wording validated · review, then accept.':'World changes checked · review, then Accept passage.';
    if(session.view?.error)return 'Draft not accepted · review the error, then recheck.';
    if(session.view?.stale)return 'Draft not accepted · source changed; check again.';
    if(wording)return 'Draft not accepted · validate wording first.';
    if(proposalsLoaded&&!effectsChecked)return 'Draft not accepted · review and validate the proposals.';
    if(reviewPending)return `Draft not accepted · ${reviewPending} proposal${reviewPending===1?' needs':'s need'} approval.`;
    return hasProse?'Draft not accepted · check world changes first.':'Draft · not accepted.';
  });
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  function reviewMessage(message:string){if(proposedEffects&&message.startsWith('Evidence quotes need correction:'))return 'Correct the source quotes on the marked proposal cards, then validate the changes.';return proposedEffects&&message.startsWith('Proposed changes need correction:')?'Correct the marked proposal cards, then validate the changes. Approvals alone do not resolve invalid connections.':message;}
  function editEffects(next:PassageEffects,invalidated:string[]){if(!session.value)return;session.value.effectsText=JSON.stringify(next,null,2);session.value.approvedEffects=(session.value.approvedEffects??[]).filter(k=>!invalidated.includes(k));session.touch();}
  function approveEffects(keys:string[]){if(!session.value||session.dirty||session.saving)return;const valid=new Set(reviewEntries.map(e=>e.key));session.value.approvedEffects=[...new Set(keys)].filter(k=>valid.has(k));session.touch();}
  function showReview(){document.querySelector('.story-details .effects-review')?.scrollIntoView({block:'start'});}
  $effect(()=>{onRun(session.running||memoryRunning);});
  $effect(()=>{const revision=project.revision;if(loaded)untrack(()=>{void refreshIndex().then(()=>session.refresh()).catch((caught)=>{error=describe(caught);});});void revision;});
  export async function flushDraft(){await session.flush();await memoryPanel?.flushDraft();}
  async function refreshIndex(){index=unwrap(await commands.loadStory(project.storyId));drafts=unwrap(await commands.listPassageDrafts(project.storyId));}
  async function openDraft(id:string){working=true;try{await session.open(id);reading=null;mode='draft';onHistory(null,'');error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function fresh(usePlan?:PassagePlan){working=true;try{
    await session.flush();await onBeforeAction();const plan=usePlan?structuredClone(usePlan):scenePlan(world,unwrap(await commands.nextPassagePlan(project.storyId)),selectedId,session.value?.plan);
    await session.create(plan);reading=null;mode='draft';onHistory(null,'');await refreshIndex();error='';
  }catch(caught){error=describe(caught);}finally{working=false;}}
  async function read(id:string){working=true;try{await session.flush();reading=unwrap(await commands.readPassage(project.storyId,id));versions=null;versionsError='';mode='reader';moment='after';showMoment();error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function revise(kind:PassageRevisionKind){if(!reading)return;working=true;error='';try{
    await session.flush();await onBeforeAction();const target=$state.snapshot(reading);
    session.adopt(unwrap(await commands.beginPassageRevision(project.storyId,crypto.randomUUID(),target.passage.id,target.record,kind)));
    reading=null;mode='draft';onHistory(null,'');await refreshIndex();
  }catch(caught){error=describe(caught);}finally{working=false;}}
  async function earlierVersions(){if(!reading||versions)return;const target=reading;versionsError='';try{const result=unwrap(await commands.passageVersions(project.storyId,target.passage.id));if(reading?.record.id===target.record.id)versions=result;}catch(caught){versionsError=describe(caught);}}
  function showMoment(){if(reading)onHistory(reading[moment],`${moment==='before'?'Before':'After'} ${reading.passage.plan.title}`);}
  async function start(checking:boolean){working=true;error='';try{
    await session.flush();if(!checking&&session.value?.prose.trim()){await session.reroll();await refreshIndex();}
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
    {#if index.passages.length}<button class="secondary-button chapter-memory-link" aria-pressed={mode==='memory'} disabled={working} onclick={()=>{void openMemory();}}>Chapter memory{memoryRunning?' · working…':''}</button>{/if}
    {#if drafts.length}<section class="story-drafts"><h2>Drafts</h2>{#each drafts as draft}{@const current=session.id===draft.id}<button class:active={mode==='draft'&&current} disabled={blocked} onclick={()=>{void openDraft(draft.id);}}><span>{(current?session.value?.plan.title:draft.title)||'Untitled passage'}</span><small>{draft.revisionKind==='wording'?'Wording · ':draft.revisionKind==='events'?'Replace events · ':''}{(current?session.view?.stage??draft.stage:draft.stage).replace('needsCheck','needs review').replace('ready','checked')}</small></button>{/each}</section>{/if}
  </aside>
{/snippet}

{#snippet detailsPanel()}
  <aside class="story-details" aria-label={mode==='memory'?'Chapter memory sources':mode==='reader'?'Passage details':'Scene planning and review'}>
  {#if mode==='memory'}{#if memoryDetails}{@render memoryDetails()}{/if}
  {:else if mode==='reader'&&reading}
    <h2>Passage details</h2>
    <p class="field-help">{reading.passage.plan.chapterTitle} · {reading.passage.plan.title}</p>
    <button class="secondary-button" disabled={working} onclick={()=>onImage(reading!,moment)}>Illustrate passage…</button>
    <dl class="passage-facts"><dt>Viewpoint</dt><dd>{reading.after.nodes.find((n)=>n.id===reading?.passage.plan.viewpointId)?.name??'Ensemble / unspecified'}</dd><dt>Location</dt><dd>{reading.after.nodes.find((n)=>n.id===reading?.passage.plan.locationId)?.name??'Unspecified'}</dd></dl>
    {#if reading.passage.plan.chapterPlan}<details><summary>Chapter direction</summary><p class="story-plan-text">{reading.passage.plan.chapterPlan}</p></details>{/if}
    <div class="passage-revision-actions"><button class="secondary-button" disabled={blocked||otherBusy} onclick={()=>{void revise('wording');}}>Edit wording</button>
      {#if selection===index.passages.length-1}<button class="secondary-button" disabled={blocked||otherBusy||!!reading.replacementError} title={reading.replacementError||'Rewrite from this passage’s saved before-state'} onclick={()=>{void revise('events');}}>Replace latest events</button>{#if reading.replacementError}<p class="field-help">{reading.replacementError}</p>{/if}{/if}
    </div>
    {#if reading.passage.previousVersion}<details class="earlier-versions" ontoggle={(event)=>{if(event.currentTarget.open)void earlierVersions();}}><summary>Earlier text versions</summary>{#if versionsError}<p role="alert">{versionsError}</p>{:else if versions}{#each versions.versions as version,i}<details><summary>{i+1} version{i?'s':''} earlier · {version.revisionKind==='wording'?'wording':version.revisionKind==='events'?'event revision':'original'}</summary><div class="prior-prose">{version.prose}</div></details>{/each}{#if versions.more}<p class="field-help">Showing the latest 20 earlier versions. Older records remain in this project.</p>{/if}{:else}<p class="field-help">Loading versions…</p>{/if}</details>{/if}
    <div class="passage-world-controls"><span>World snapshot</span><button class="secondary-button" aria-pressed={moment==='before'} onclick={()=>{moment='before';showMoment();}}>Before</button><button class="secondary-button" aria-pressed={moment==='after'} onclick={()=>{moment='after';showMoment();}}>After</button><p class="field-help">Switch to World to inspect this passage's saved state.</p><button class="text-action" disabled={otherBusy||session.running||working} onclick={()=>{void session.flush().then(()=>onFork(reading!.passage.id)).catch((caught)=>{error=describe(caught);});}}>Fork after this passage…</button></div>
    {#if reading.evidenceProse}<details class="earlier-evidence"><summary>Evidence from earlier wording</summary><p class="field-help">The events are retained. Their quotes refer to this earlier text, not the current wording.</p><div class="prior-prose">{reading.evidenceProse}</div></details>{/if}
    {#if reading.passage.effects.notes.length}<section class="continuity-notes"><h3>Continuity notes</h3>{#each reading.passage.effects.notes as note}<p>{note.text}</p>{#if reading.evidenceProse}<small>Quote from earlier wording</small>{/if}<blockquote>{note.evidence}</blockquote>{/each}</section>{/if}
  {:else if session.value}
    <h2>{wording?'Wording review':replacing?'Replace latest events':hasProse?'Review world changes':'Scene plan'}</h2>
    <fieldset class="passage-fields" disabled={blocked||session.view?.stage==='accepted'}>
      {#if wording}
        <p class="field-help">Keeps accepted events and world snapshots. Review the diff; Accept wording only confirms that events are unchanged. AI polish can still change meaning.</p>
        {#if session.view?.error}<p class="story-warning" role="alert">{session.view.error}</p>{/if}
        {#if session.view?.stale}<p class="story-warning">The source or connection settings changed. Validate again if the accepted source is unchanged; otherwise start a revision from its current version.</p>{/if}
        <button class="secondary-button" onclick={()=>{void validate();}}>Validate wording</button>
      {:else}
      {#if replacing}<p class="field-help">Uses this passage's saved before-world. Acceptance replaces its events and keeps the original version.</p>{/if}
      <details class="scene-settings" open={!hasProse}><summary>Scene settings</summary>
      <details class="chapter-plan" open={!session.value.prose}><summary>Chapter plan</summary><label class="field">Chapter title<input bind:value={session.value.plan.chapterTitle} oninput={()=>session.touch()} maxlength={200} /></label><label class="field">Chapter direction<textarea bind:value={session.value.plan.chapterPlan} oninput={()=>session.touch()} rows={3} maxlength={4000}></textarea></label></details>
      <label class="field">Viewpoint<select bind:value={session.value.plan.viewpointId} onchange={()=>session.touch()}><option value={null}>Ensemble / unspecified</option>{#each actors as node}<option value={node.id}>{node.name}</option>{/each}</select></label>
      <label class="field">Location<select bind:value={session.value.plan.locationId} onchange={()=>session.touch()}><option value={null}>Unspecified</option>{#each places as node}<option value={node.id}>{node.name}</option>{/each}</select></label>
      <details class="scene-resources"><summary>Participants & references</summary><input aria-label="Find scene resources" placeholder="Find a world node…" bind:value={referenceSearch} />
        <div class="scene-resource-list">{#each sceneWorld.nodes.filter((n)=>n.name.toLowerCase().includes(referenceSearch.toLowerCase())).slice(0,100) as node}<div><span>{node.name}</span>{#if node.components['core.character']&&node.id!==session.value.plan.viewpointId}<label><input type="checkbox" checked={session.value.plan.participantIds.includes(node.id)} disabled={session.value.plan.participantIds.length>=12&&!session.value.plan.participantIds.includes(node.id)} onchange={()=>references(node.id,true)} />Participant</label>{/if}<label><input type="checkbox" checked={session.value.plan.referenceIds.includes(node.id)} disabled={session.value.plan.referenceIds.length>=12&&!session.value.plan.referenceIds.includes(node.id)} onchange={()=>references(node.id)} />Reference</label></div>{/each}</div>
      </details>
      </details>
      {#if session.value.prose.trim()}
        <section class="effects-review" aria-label="Proposed world changes">
          {#if session.view?.stale}<p class="story-warning">Source world/history or LLM settings changed. Recheck before accepting.</p>{/if}
          {#if session.view?.error}<p class="story-warning" role="alert">{reviewMessage(session.view.error)}</p>{/if}
          {#if proposedEffects}<EffectsReviewEditor world={sceneWorld} effects={proposedEffects} prose={session.value.prose} review={session.view?.effectsReview} approvedKeys={session.value.approvedEffects??[]} disabled={blocked} canApprove={!session.dirty&&!session.saving} onChange={editEffects} onApprove={approveEffects}/>{:else}<p class="field-help">The response needs repair before cards can be displayed. Use Advanced JSON or generate new proposals.</p>{/if}
          <div class="effects-actions">{#if proposalsLoaded}<button class="text-action" disabled={otherBusy||memoryRunning} onclick={()=>{void start(true);}}>Generate new proposals with AI</button>{/if}<button class="text-action" onclick={()=>{editEffects({changes:[],notes:[],suggestChapterEnd:false},reviewEntries.map(e=>e.key));void validate();}}>Use no world changes</button></div>
          <details class="passage-effects" open={!proposedEffects}><summary>Advanced JSON</summary><p class="field-help">Changes made here update the same proposal cards and require review. Each evidence field must copy one continuous 8–600 character excerpt.</p>{#if error||session.view?.error}<pre class="effect-raw-error">{error||session.view?.error}</pre>{/if}<textarea aria-label="Passage effects JSON" bind:value={session.value.effectsText} oninput={()=>session.touch()} rows={10} maxlength={500000} spellcheck={false}></textarea><button class="secondary-button" onclick={()=>{void validate();}}>Validate changes</button></details>
          {#if session.view?.suggestedChapterEnd}<p class="field-help">The AI suggests ending this chapter.</p>{/if}
          <label class="check-field"><input type="checkbox" bind:checked={session.value.closeChapter} onchange={()=>session.touch()} /><span>Close chapter after this passage</span></label>
        </section>
      {/if}
      {/if}
    </fieldset>
    {#if session.view}<details class="context-coverage"><summary>Generation context</summary><p>Prose: {session.view.narrationProvider||'Manual'} · Effects: {session.view.effectsProvider||'Manual'}</p><p>{session.view.contextNodes.join(', ')||'No existing world nodes selected.'}</p><p>Recent text: up to two accepted passages. Participants do not move until prose establishes a move.</p>
      {#if session.view.memory.coverage.olderPassages>0}{@const coverage=session.view.memory.coverage}<p>{coverage.selectedPoints} earlier memory points · {coverage.currentSummaries}/{coverage.closedChapters} completed chapters have current summaries.</p><p>{coverage.unrepresentedPassages} older passages have no current summary or accepted notes. {coverage.omittedPoints} available points omitted by scope/size limits.</p>{#if coverage.staleSummaries||coverage.unavailableSummaries}<p>{coverage.staleSummaries} stale and {coverage.unavailableSummaries} unavailable summaries excluded.</p>{/if}{#each session.view.memory.points as point}<button class="text-action memory-source-link" disabled={working} onclick={()=>{void read(point.passageId);}}>{point.text}<small>{point.reason}</small></button>{/each}{/if}
    </details>{/if}
    {#if session.view?.originalProse||session.view?.originalEffects}<details class="original-passage-output"><summary>Original model output</summary>{#if session.view.originalProse}<pre>{session.view.originalProse}</pre>{/if}{#if session.view.originalEffects}<pre>{session.view.originalEffects}</pre>{/if}</details>{/if}
  {:else}<p class="story-loading">Loading story…</p>{/if}
  </aside>
{/snippet}

<section class="story-workspace" aria-label="Story composer and reader">
  {#if memoryOpened}<div class="workspace-surface" hidden={mode!=='memory'}><ChapterMemoryEditor bind:this={memoryPanel} {project} otherBusy={otherBusy||session.running} {onBeforeAction} onApply={onMemoryApply} onRun={running=>{memoryRunning=running;}} onDetails={registerMemoryDetails}/></div>{/if}
  <div class="workspace-surface" hidden={mode==='memory'}>
  {#if error||session.error}<div class="story-error" role="alert"><span>{reviewMessage(error||session.error)}</span><button class="text-action" onclick={()=>{void session.flush().then(()=>session.refresh()).then(()=>{error='';}).catch((caught)=>{error=describe(caught);});}}>Retry / refresh</button></div>{/if}
  {#if session.running}<div class="story-job" role="status"><span>{session.view?.stage==='narrating'?'Writing passage…':'Checking world changes…'} You can browse the world while this runs.</span><button class="secondary-button" onclick={()=>{if(session.id)void commands.cancelAuthoring(session.id).then(unwrap).then(()=>session.refresh()).catch((caught)=>{error=describe(caught);});}}>Cancel</button></div>{/if}
  {#if mode==='draft'&&session.value}
    <fieldset class="passage-fields passage-toolbar" aria-label="Passage generation" disabled={blocked||session.view?.stage==='accepted'}>
      {#if !wording}<label>Target words<input type="number" aria-label="Target words" value={session.value.plan.targetWords} oninput={(event)=>{const n=event.currentTarget.valueAsNumber;if(Number.isFinite(n)&&n>=100&&n<=1500){session.value!.plan.targetWords=Math.round(n);session.touch();}}} onblur={(event)=>{event.currentTarget.value=String(session.value!.plan.targetWords);}} min={100} max={1500} step={50} /></label>{/if}
      <button class="primary-button" disabled={!!generationHelp} title={generationHelp||undefined} onclick={()=>{void start(false);}}>{wording?'Generate polish candidate':session.value.prose?'Generate new candidate':'Generate passage'}</button>
      <button class="text-action" onclick={onSettings}>LLM settings</button>
      {#if generationHelp}<p class="field-help generation-help" role="status">{generationHelp}</p>{/if}
    </fieldset>
  {/if}
  <div class="story-scroll">
  {#if mode==='reader'&&reading}
    <article class="passage-reader">
      <div class="reader-navigation"><button class="secondary-button" disabled={working||selection<=0} onclick={()=>{void read(index.passages[selection-1].id);}}>← Previous</button><span>{selection+1} / {index.passages.length}</span><button class="secondary-button" disabled={working||selection>=index.passages.length-1} onclick={()=>{void read(index.passages[selection+1].id);}}>Next →</button></div>
      <p class="chapter-name">{reading.passage.plan.chapterTitle}</p><h2>{reading.passage.plan.title}</h2>{#if reading.passage.narrationProvider.startsWith('Offline demo')}<p class="field-help">{reading.passage.narrationProvider}</p>{/if}<div class="accepted-prose">{reading.passage.prose}</div>
      <PassageImages storyId={project.storyId} passageId={reading.passage.id} revision={project.revision}/>
    </article>
  {:else if session.value}
    <div class="passage-composer">
      <fieldset class="passage-fields" disabled={blocked||session.view?.stage==='accepted'}>
        <label class="field">Passage title<input bind:value={session.value.plan.title} disabled={wording} oninput={()=>session.touch()} maxlength={200} /></label>
        <label class="field">{wording?'Wording direction (for AI polish)':'Passage direction (optional)'}<textarea bind:value={session.value.plan.direction} oninput={()=>session.touch()} rows={4} maxlength={8000} placeholder={wording?'How should the wording change while keeping events intact?':'Optional: what happens next, and where should it stop?'}></textarea>{#if !wording}<span class="field-help">Leave blank to let AI develop the next passage from the selected scene and chapter direction.</span>{/if}</label>
        {#if session.view?.narrationProvider.startsWith('Offline demo')}<p class="field-help">{session.view.narrationProvider}</p>{/if}
        <label class="field prose-field">Passage<textarea bind:value={session.value.prose} oninput={()=>session.touch()} rows={16} maxlength={100000} placeholder="Generate a passage or write it here."></textarea><span class="field-help">{wordCount} words</span></label>
      </fieldset>
      {#if session.view?.revision}<details class="prose-review" open={session.readyToAccept}><summary>Review text changes</summary><p class="field-help">Removed text is struck through; additions are underlined.</p><div class="prose-diff">{#each textChanges as change}{#if change.kind==='removed'}<del>{change.text}</del>{:else if change.kind==='added'}<ins>{change.text}</ins>{:else}{change.text}{/if}{/each}</div></details>{/if}
    </div>
  {:else}<p class="story-loading">Loading story…</p>{/if}
  </div>
  {#if mode==='draft'}<footer class="composer-footer">
    <span id="passage-acceptance-status" aria-live="polite">{acceptanceHint}</span>
    <div class="composer-actions">
      {#if !wording&&hasProse&&!session.readyToAccept&&session.view?.stage!=='accepted'}<button class="secondary-button" disabled={blocked||otherBusy||memoryRunning} onclick={()=>{if(!proposalsLoaded)void start(true);else if(!effectsChecked)void validate();else showReview();}}>{!proposalsLoaded?'Check world changes with AI':!effectsChecked?'Validate reviewed changes':'Review changes'}</button>{/if}
      <button class="primary-button" aria-describedby="passage-acceptance-status" disabled={blocked||!session.readyToAccept} onclick={()=>{void accept();}}>{wording?'Accept wording only':replacing?'Accept replacement':'Accept passage'}</button>
    </div>
  </footer>{/if}
  </div>
</section>
