<script lang="ts">
  import BackupDialog from './lib/BackupDialog.svelte';
  import { onMount, type Snippet } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { PaneGroup, Pane, PaneResizer } from 'paneforge';
  import { SvelteFlowProvider } from '@xyflow/svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import WorldMap from './lib/WorldMap.svelte';
  import Inspector from './lib/Inspector.svelte';
  import WorldEditDialog from './lib/WorldEditDialog.svelte';
  import RelationshipTypeDialog from './lib/RelationshipTypeDialog.svelte';
  import SettingsDialog from './lib/SettingsDialog.svelte';
  import AiAuthoringDialog from './lib/AiAuthoringDialog.svelte';
  import StoryWorkspace from './lib/StoryWorkspace.svelte';
  import ImageDialog from './lib/ImageDialog.svelte';
  import type {MediaTarget,PassageReading_Serialize} from './lib/bindings';
  import { newAuthoringRequest } from './lib/authoring-ui';
  import { AppearanceSettings } from './lib/appearance.svelte';
  import { applyAppearance, resolveTheme } from './lib/appearance';
  import { exampleWorld, emptyWorld, makeNode, type ProjectView, type Workspace, type World } from './lib/model';
  import { worldRecords, visibleRecords, orderedRecords, revealRecord, recordDepth, editRecord, type WorldNode } from './lib/graph';
  import { commands, desktop, unwrap, RejectedChange } from './lib/native';
  import icon from './assets/app-icon.svg';
  import type { ApplyAuthoringResult_Serialize, AuthoringRequest, CommitResult_Serialize, WorldEdit,PassageAcceptance_Serialize } from './lib/bindings';

  const initialWorld = exampleWorld();
  const initialWorkspace = (): Workspace => ({ version: 0, expandedIds: [...initialWorld.nodes.filter((n) => n.components["core.place"] || n.components["core.character"]).map((n) => n.id)], selectedId: initialWorld.nodes.find((n) => n.components["core.character"])!.id, positions: {} });
  let project = $state<ProjectView | null>(null);
  let imageTarget=$state<MediaTarget|null>(null),imageWorlds=$state<{before:World;after:World}|null>(null);
  let imageDialog=$state<{flushDraft:()=>Promise<void>}>();
  let imagePromptRunId=$state<string|null>(null),imagePromptMediaId=$state<string|null>(null),imagePromptRunning=$state(false),imagePromptRetained=$state(false);
  let imagePromptTarget=$state<MediaTarget|null>(null),imagePromptWorlds=$state<{before:World;after:World}|null>(null),preferredImageId=$state<string|null>(null);
  function promptRun(id:string,mediaId:string,running:boolean,retained:boolean){if(running||retained){imagePromptRunId=id;imagePromptMediaId=mediaId;imagePromptRunning=running;imagePromptRetained=retained;if(imageTarget&&imageWorlds){imagePromptTarget=structuredClone($state.snapshot(imageTarget));imagePromptWorlds=structuredClone($state.snapshot(imageWorlds));}}else if(imagePromptRunId===id){imagePromptRunning=false;imagePromptRetained=false;}}
  function reopenPrompt(){if(!imagePromptTarget||!imagePromptWorlds)return;imageTarget=imagePromptTarget;imageWorlds=imagePromptWorlds;preferredImageId=imagePromptMediaId;}
  function nodeImage(id:string){if(!project||historyWorld)return;preferredImageId=null;imageWorlds={before:structuredClone($state.snapshot(world)),after:structuredClone($state.snapshot(world))};imageTarget={nodeId:id,passageId:null,moment:'after',subjectIds:[]};}
  function passageImage(reading:PassageReading_Serialize,moment:'before'|'after'){preferredImageId=null;imageWorlds={before:reading.before,after:reading.after};const source=moment==='before'?reading.before:reading.after;const plan=reading.passage.plan;const subjects=[...new Set([plan.viewpointId,...plan.participantIds,plan.locationId].filter((id):id is string=>!!id&&source.nodes.some(n=>n.id===id)))].slice(0,4);imageTarget={nodeId:null,passageId:reading.passage.id,moment,subjectIds:subjects};}
  let world = $state<World>(initialWorld);
  let workspace = $state<Workspace>(initialWorkspace());
  let search = $state('');
  let error = $state('');
  let layoutError = $state('');
  let status = $state('Example world');
  let busy = $state(false);
  let backupMode=$state<'export'|'restore'|null>(null),backupWorking=$state(false);
  const appearance = new AppearanceSettings();
  let showSettings = $state(false);
  let settingsSection=$state<'appearance'|'llm'|'images'>('appearance');
  let settingsDialog=$state<{flushSettings:()=>Promise<void>}>();
  let showAi=$state(false),aiRunning=$state(false),aiRunId=$state<string|null>(null),aiRequest=$state<AuthoringRequest|null>(null);
  let aiDialog=$state<{flushDraft:()=>Promise<void>}>();
  let storyPanel=$state<{flushDraft:()=>Promise<void>}>();
  let storyContents=$state<Snippet>(),storyDetails=$state<Snippet>();
  function registerStoryPanels(contents?:Snippet,details?:Snippet){storyContents=contents;storyDetails=details;}
  let surface=$state<'world'|'story'>('world'),storyOpened=$state(false),passageRunning=$state(false);
  let historyWorld=$state<World|null>(null),historyLabel=$state('');
  let historyWorkspace=$state<Workspace>({version:0,expandedIds:[],selectedId:null,positions:{}});
  let systemDark = $state(window.matchMedia('(prefers-color-scheme: dark)').matches);
  const colorMode = $derived(resolveTheme(appearance.value, systemDark).mode);
  $effect(() => { applyAppearance(appearance.value, systemDark); });
  let structuralEdit = $state<WorldEdit | null>(null);
  let showRelationshipType = $state(false);
  let showCreate = $state(false);
  let newTitle = $state('Untitled story');
  let useExample = $state(false);
  let recoveryPath = $state<string | null>(null);
  function showNewProject(){newTitle='Untitled story';useExample=false;showCreate=true;}
  let editVersion = 0, savedVersion = 0, persistedDraftVersion = 0;
  let timer: ReturnType<typeof setTimeout>;
  let layoutTimer: ReturnType<typeof setTimeout>;
  let inFlight: Promise<void> | null = null;
  let layoutFlight: Promise<void> = Promise.resolve();
  type SaveOperation = { requestId: string; expectedRevision: number; title: string; world: World; version: number };
  let pending: SaveOperation | null = null;
  const displayWorld=$derived(historyWorld??world),displayWorkspace=$derived(historyWorld?historyWorkspace:workspace);
  const records = $derived(worldRecords(displayWorld));
  const selected = $derived(records.find((r) => r.id === displayWorkspace.selectedId));
  const treeRows = $derived(orderedRecords(visibleRecords(records, displayWorkspace.expandedIds, search)));

  function describe(caught: unknown) { return caught instanceof Error ? caught.message : String(caught); }
  async function closeSettings() {
    try { await settingsDialog?.flushSettings();await appearance.flush(); showSettings=false; } catch { /* Keep the settings error and edits visible. */ }
  }
  function openAi(targetId?:string){
    if(!project||busy||historyWorld||passageRunning||imagePromptRunning||imagePromptRetained)return;
    if(aiRunning){showAi=true;return;}
    if(targetId||!aiRequest){aiRequest=newAuthoringRequest(world,targetId);aiRunId=null;}
    showAi=true;
  }
  async function beforeAiAction(){await flush();if(!project)throw Error('Open a project first.');return structuredClone($state.snapshot(project));}
  async function applyAi(result:ApplyAuthoringResult_Serialize){await adopt(result.commit.project);showAi=false;aiRunId=result.proposal.id;aiRequest=result.proposal.request;const id=result.proposal.createdNodeIds[0]??result.proposal.request.targetIds[0];if(id)select(id);}
  async function switchSurface(next:'world'|'story'){
    try{await storyPanel?.flushDraft();if(next==='story')await flush();surface=next;if(next==='story')storyOpened=true;}catch(caught){error=describe(caught);}
  }
  function showHistory(snapshot:World|null,label:string){
    if(snapshot){const ids=new Set(snapshot.nodes.map((n)=>n.id));const selectedId=historyWorkspace.selectedId&&ids.has(historyWorkspace.selectedId)?historyWorkspace.selectedId:workspace.selectedId&&ids.has(workspace.selectedId)?workspace.selectedId:snapshot.nodes[0]?.id??null;
      historyWorkspace={version:0,selectedId,expandedIds:snapshot.nodes.filter((n)=>n.components['core.place']).map((n)=>n.id),positions:{}};
    }historyWorld=snapshot;historyLabel=label;search='';
  }
  async function acceptedPassage(result:PassageAcceptance_Serialize){await adopt(result.commit.project);surface='story';storyOpened=true;}
  async function appliedMemory(result:CommitResult_Serialize){await adopt(result.project);surface='story';storyOpened=true;}
  async function forkPassage(id:string){
    if(!project||busy)return;busy=true;
    try{if(!await prepareSwitch())return;const parent=await open({directory:true,multiple:false,title:'Choose where to save the independent story fork'});if(typeof parent!=='string')return;
      const result=unwrap(await commands.forkAfterPassage(project.storyId,id,parent,project.title.slice(0,180)+' (fork)'));await adopt(result);surface='story';storyOpened=true;
    }catch(caught){error=describe(caught);}finally{busy=false;}
  }
  function scheduleSave() {
    if (!project) return;
    editVersion++;
    status = 'Unsaved edits';
    clearTimeout(timer);
    timer = setTimeout(() => { void flush().catch(() => {}); }, 450);
  }

  async function saveLoop() {
    if (!project) return;
    while (pending || savedVersion < editVersion) {
      const session = project;
      if (!session) throw new Error('The project session was closed.');
      const operation: SaveOperation = pending ?? {
        requestId: crypto.randomUUID(), expectedRevision: session.revision, title: session.title,
        world: structuredClone($state.snapshot(world)), version: editVersion,
      };
      status = 'Saving…';
      try {
        if (!pending) {
          unwrap(await commands.saveDraft({ schemaVersion: 1, sourceRevision: operation.expectedRevision, title: operation.title, world: operation.world }));
          persistedDraftVersion = operation.version;
          pending = operation;
        }
        const result: CommitResult_Serialize = unwrap(await commands.saveProject(operation.expectedRevision, operation.requestId, operation.title, operation.world));
        project = result.project;
        savedVersion = operation.version;
        pending = null;
        error = '';
        // Cleanup failure does not turn a committed save into a failed save.
        try { unwrap(await commands.clearMatchingDraft(operation.title, operation.world)); } catch (cleanupError) { layoutError = `Saved. Draft cleanup: ${describe(cleanupError)}`; }
      } catch (caught) {
        if (caught instanceof RejectedChange) pending = null;
        status = persistedDraftVersion >= editVersion ? 'Draft saved · needs attention' : 'Save failed';
        error = describe(caught);
        throw caught;
      }
    }
    status = 'All changes saved';
  }

  async function flush() {
    clearTimeout(timer);
    if (inFlight) return inFlight;
    inFlight = saveLoop();
    try { await inFlight; } finally { inFlight = null; }
  }

  function persistLayout() {
    if (!project) return;
    clearTimeout(layoutTimer);
    layoutTimer = setTimeout(() => { void flushLayout(); }, 350);
  }
  async function flushLayout() {
    clearTimeout(layoutTimer);
    if (!project) return;
    const storyId = project.storyId;
      layoutFlight = layoutFlight.then(async () => {
        if (!project || project.storyId !== storyId) return;
        try {
          const snapshot = structuredClone($state.snapshot(workspace));
          const saved = unwrap(await commands.saveWorkspace(snapshot.version, snapshot));
          // Keep changes made while the save was in flight, only advance its version.
          if (project?.storyId === storyId) workspace = { ...workspace, version: saved.version };
          layoutError = '';
        } catch (caught) { layoutError = `Layout not saved: ${describe(caught)}`; }
      });
    await layoutFlight;
  }
  function select(id: string) {
    if(historyWorld){if(!visibleRecords(records,historyWorkspace.expandedIds,search).some((r)=>r.id===id))search='';historyWorkspace={...historyWorkspace,selectedId:id,expandedIds:revealRecord(records,historyWorkspace.expandedIds,id)};return;}
    if (!visibleRecords(records, workspace.expandedIds, search).some((r) => r.id === id)) search = '';
    workspace = { ...workspace, selectedId: id, expandedIds: revealRecord(records, workspace.expandedIds, id) }; persistLayout();
  }
  function toggle(id: string) {
    if(historyWorld){historyWorkspace={...historyWorkspace,expandedIds:historyWorkspace.expandedIds.includes(id)?historyWorkspace.expandedIds.filter((n)=>n!==id):[...historyWorkspace.expandedIds,id]};return;}
    workspace = { ...workspace, expandedIds: workspace.expandedIds.includes(id) ? workspace.expandedIds.filter((x) => x !== id) : [...workspace.expandedIds, id] };
    persistLayout();
  }
  function moved(nodes: WorldNode[]) {
    if(historyWorld){historyWorkspace={...historyWorkspace,positions:{...historyWorkspace.positions,...Object.fromEntries(nodes.map((n)=>[n.id,n.position]))}};return;}
    workspace = { ...workspace, positions: { ...workspace.positions, ...Object.fromEntries(nodes.map((n) => [n.id, n.position])) } };
    persistLayout();
  }
  function edit(field: 'name' | 'description' | 'visualDescription', value: string) {
    if (!project || !selected || historyWorld) return;
    world = editRecord(world, selected.id, field, value);
    scheduleSave();
  }

  async function adopt(view: ProjectView) {
    clearTimeout(timer); clearTimeout(layoutTimer);
    const sameStory=project?.storyId===view.storyId&&project?.folderPath===view.folderPath;
    project = view; world = view.world;
    historyWorld=null;historyLabel='';
    if(!sameStory){aiRunId=null;aiRunning=false;aiRequest=null;imagePromptRunId=null;imagePromptMediaId=null;imagePromptRunning=false;imagePromptRetained=false;imagePromptTarget=null;imagePromptWorlds=null;passageRunning=false;storyOpened=false;surface='world';}
    editVersion = savedVersion = persistedDraftVersion = 0; pending = null;
    workspace = { version: 0, expandedIds: view.world.nodes.filter((n) => n.components["core.place"]).map((n) => n.id), selectedId: view.world.nodes[0]?.id ?? null, positions: {} };
    search = ''; error = view.warnings.join(' '); layoutError = ''; recoveryPath = null;
    status = 'All changes saved';
    try { const savedWorkspace = unwrap(await commands.loadWorkspace()); if (savedWorkspace.version) workspace = savedWorkspace; } catch (caught) { layoutError = `Map layout could not be loaded: ${describe(caught)}`; }
    try {
      const draft = unwrap(await commands.loadDraft());
      if (draft) {
        if (draft.title === view.title && JSON.stringify(draft.world) === JSON.stringify(view.world)) {
          unwrap(await commands.clearMatchingDraft(draft.title, draft.world));
        } else if (draft.sourceRevision === view.revision) {
          world = draft.world; editVersion = persistedDraftVersion = 1; status = 'Recovered unsaved draft';
        } else {
          error = 'An older editor draft is preserved in editor-draft.json. It has not been applied to this newer save.';
        }
      }
    } catch (caught) { error = `Editor draft could not be loaded: ${describe(caught)}`; }
    const ids = new Set(world.nodes.map((n) => n.id));
    workspace = {...workspace,selectedId:workspace.selectedId && ids.has(workspace.selectedId) ? workspace.selectedId : world.nodes[0]?.id ?? null,
      expandedIds:workspace.expandedIds.filter((id) => ids.has(id)),positions:Object.fromEntries(Object.entries(workspace.positions).filter(([id]) => ids.has(id)))};
    if(!sameStory)try{
      const proposals=unwrap(await commands.listAuthoring(view.storyId));
      const unfinished=proposals.find((proposal)=>['draft','review','interrupted','failed'].includes(proposal.status));
      if(unfinished){const proposal=unwrap(await commands.getAuthoring(view.storyId,unfinished.id));aiRunId=proposal.id;aiRequest=proposal.request;}
    }catch(caught){error=[error,`AI proposals need attention: ${describe(caught)}`].filter(Boolean).join(' ');}
  }

  async function prepareSwitch() {
    try { await aiDialog?.flushDraft();await storyPanel?.flushDraft();await imageDialog?.flushDraft();if(aiRunning||passageRunning||imagePromptRunning||imagePromptRetained)throw Error('Cancel or finish generation before changing projects.');await flush(); await flushLayout(); return true; } catch(caught) { error = 'Resolve unsaved work before changing projects. ' + describe(caught); return false; }
  }
  async function prepareBackup(){
    await aiDialog?.flushDraft();await storyPanel?.flushDraft();await imageDialog?.flushDraft();
    if(aiRunning||passageRunning||imagePromptRunning||imagePromptRetained)throw Error('Finish or cancel generation before making a backup or restoring.');
    try{await flush();}catch(caught){if(persistedDraftVersion<editVersion)throw caught;}
    await flushLayout();if(layoutError)throw Error(layoutError);
    return project?structuredClone($state.snapshot(project)):null;
  }
  async function openRestored(path:string){if(!await prepareSwitch())throw Error(error);await adopt(unwrap(await commands.openProject(path)));backupMode=null;}
  async function openProject() {
    if (!desktop || busy) return;
    busy = true;
    let path: string | null = null;
    try {
      if (!await prepareSwitch()) return;
      const picked = await open({ directory: true, multiple: false, title: 'Open a Guided Story project folder' });
      if (typeof picked !== 'string') return;
      path = picked; await adopt(unwrap(await commands.openProject(path)));
    }
    catch (caught) { error = describe(caught); recoveryPath = path; }
    finally { busy = false; }
  }
  async function createProject(event: SubmitEvent) {
    event.preventDefault();
    if (!desktop || busy || !newTitle.trim()) return;
    busy = true;
    try {
      if (!await prepareSwitch()) return;
      const parent = await open({ directory: true, multiple: false, title: 'Choose where to create the new story folder' });
      if (typeof parent !== 'string') return;
      await adopt(unwrap(await commands.createProject(parent, newTitle.trim(), useExample ? exampleWorld() : emptyWorld(), crypto.randomUUID())));
      showCreate = false;
    } catch (caught) { error = describe(caught); }
    finally { busy = false; }
  }
  async function recoverProject() {
    if (!recoveryPath || busy) return;
    busy = true;
    try { if (await prepareSwitch()) await adopt(unwrap(await commands.recoverProject(recoveryPath))); }
    catch (caught) { error = describe(caught); }
    finally { busy = false; }
  }
  function addRecord(typeId: string) {
    if (!project || busy || historyWorld) return;
    const preset = world.nodeTypes.find((d) => d.id === typeId);
    if (!preset) return;
    const node = makeNode(world, typeId, 'New ' + preset.name.toLowerCase());
    const selectedNode = world.nodes.find((n) => n.id === selected?.id);
    const place = selectedNode?.components['core.place'] ? selectedNode : world.nodes.find((n) => n.components['core.place']);
    let target = place?.id, relation: string | undefined;
    if (node.components['core.place'] && selectedNode?.components['core.place']) relation = 'core.inside';
    else if (node.components['core.character'] && place) relation = 'core.present-at';
    else if (node.components['core.item']) {
      const holder = selectedNode && (selectedNode.components['core.holder'] || selectedNode.components['core.container']) ? selectedNode : place;
      target = holder?.id; if (target) relation = 'core.stored-in';
    } else if (selectedNode) { target = selectedNode.id; relation = 'core.related'; }
    const edge = target && relation ? [{id:crypto.randomUUID(),typeId:relation,sourceId:node.id,targetId:target,description:''}] : [];
    world = {...world,nodes:[...world.nodes,node],edges:[...world.edges,...edge]};
    workspace = {...workspace,selectedId:node.id,expandedIds:[...new Set([...workspace.expandedIds,...(target?[target]:[])])]};
    scheduleSave(); persistLayout();
  }
  function property(componentId: string, key: string, value: string | number | boolean | string[] | null) {
    if (!project || !selected || historyWorld) return;
    world = {...world,nodes:world.nodes.map((node)=>node.id===selected!.id?{...node,components:{...node.components,[componentId]:{...node.components[componentId],[key]:value}}}:node)};
    scheduleSave();
  }
  function addCapability(componentId: string) {
    const definition = world.componentTypes.find((d)=>d.id===componentId);
    if (!definition || !project || !selected || historyWorld) return;
    world = {...world,nodes:world.nodes.map((node)=>node.id===selected!.id?{...node,components:{...node.components,[componentId]:Object.fromEntries(definition.fields.map((f)=>[f.key,structuredClone(f.defaultValue)]))}}:node)};
    scheduleSave();
  }
  function editLink(id: string) {
    if (!project || busy || historyWorld) return;
    const link = world.edges.find((e) => e.id === id);
    if (link) structuralEdit = {kind:'putLink', link:structuredClone($state.snapshot(link))};
  }
  function connect(source: string, target = '') {
    if (!project || busy || historyWorld) return;
    const node = world.nodes.find((n) => n.id === source);
    const destination = world.nodes.find((n) => n.id === target);
    const typeId = node?.components['core.item'] && (destination?.components['core.holder'] || destination?.components['core.container']) ? 'core.stored-in'
      : node?.components['core.character'] && destination?.components['core.place'] ? 'core.present-at'
      : node?.components['core.place'] && destination?.components['core.place'] ? 'core.inside' : 'core.related';
    structuralEdit = {kind:'putLink',link:{id:crypto.randomUUID(),typeId,sourceId:source,targetId:target,description:''}};
  }
  function reviewEdit(edit: WorldEdit) { if (project && !busy && !historyWorld) structuralEdit = edit; }
  function applyStructure(next: World, source: string) {
    if (!project || historyWorld || JSON.stringify(world) !== source) { error = 'The world changed while reviewing. Review this operation again.'; structuralEdit = null; return; }
    const changed = next.edges.filter((edge) => !world.edges.some((old) => old.id === edge.id && old.sourceId === edge.sourceId && old.targetId === edge.targetId));
    world = next;
    const ids = new Set(next.nodes.map((n) => n.id));
    // Layout cleanup is separate from canonical world publication.
    workspace = {...workspace,selectedId:workspace.selectedId && ids.has(workspace.selectedId) ? workspace.selectedId : next.nodes[0]?.id ?? null,
      expandedIds:[...new Set([...workspace.expandedIds.filter((id) => ids.has(id)), ...changed.map((e) => e.targetId)])],
      positions:Object.fromEntries(Object.entries(workspace.positions).filter(([id]) => ids.has(id)))};
    workspace = {...workspace,expandedIds:revealRecord(worldRecords(next), workspace.expandedIds, workspace.selectedId)};
    for (const edge of changed) workspace = {...workspace,expandedIds:revealRecord(worldRecords(next), workspace.expandedIds, edge.sourceId)};
    structuralEdit = null; scheduleSave(); persistLayout();
  }
  let showType = $state(false);
  let typeName = $state('');
  let typeBase = $state('');
  let typeFields = $state('');
  function createType(event: SubmitEvent) {
    event.preventDefault();
    if (!project || !typeName.trim()) return;
    const typeId='custom.'+crypto.randomUUID(), componentId='custom.'+crypto.randomUUID();
    const fields=[...new Set(typeFields.split(',').map((s)=>s.trim()).filter(Boolean))].map((label,index)=>({key:'field'+index,label,kind:'text' as const,required:false,defaultValue:''}));
    const components=[...(world.nodeTypes.find((d)=>d.id===typeBase)?.components??[]),...(fields.length?[componentId]:[])];
    world={...world,nodeTypes:[...world.nodeTypes,{id:typeId,name:typeName.trim(),description:'Custom world node preset',components}],
      componentTypes:[...world.componentTypes,...(fields.length?[{id:componentId,name:typeName.trim()+' details',description:'Custom fields',fields}]:[])]};
    showType=false;addRecord(typeId);typeName='';typeFields='';typeBase='';
  }

  onMount(() => {
    let unlisten: (() => void) | undefined;
    const media=window.matchMedia('(prefers-color-scheme: dark)');
    const changeMode=()=>{systemDark=media.matches;};
    media.addEventListener('change', changeMode);
    void appearance.load();
    const aiPoll=setInterval(()=>{if(imagePromptRunId&&project&&(imagePromptRunning||imagePromptRetained)){const id=imagePromptRunId;void commands.getImagePrompt(project.storyId,id).then(unwrap).then(view=>{if(imagePromptRunId===id){imagePromptRunning=view.stage==='running';imagePromptRetained=view.retained;}}).catch(caught=>{error=describe(caught);});}if(aiRunning&&aiRunId&&project&&!showAi){void commands.getAuthoring(project.storyId,aiRunId).then(unwrap).then((view)=>{aiRunning=view.status==='running';}).catch((caught)=>{error=describe(caught);});}},1000);
    if (desktop) void commands.startupProject().then(unwrap).then(async (saved) => { if (saved) await adopt(saved); }).catch((caught) => { error = describe(caught); });
    if (desktop) void getCurrentWindow().onCloseRequested(async (event) => {
      if(backupWorking){event.preventDefault();return;}
      try { await aiDialog?.flushDraft();await storyPanel?.flushDraft();await imageDialog?.flushDraft();await settingsDialog?.flushSettings();unwrap(await commands.closeAuthoring()); } catch(caught) { event.preventDefault();error=describe(caught);return; }
      try { await appearance.flush(); } catch { event.preventDefault(); showSettings=true; return; }
      if (!project) return;
      busy = true;
      try { await flush(); } catch { /* A persisted invalid form remains recoverable. */ }
      await flushLayout();
      if (savedVersion < editVersion && persistedDraftVersion < editVersion) {
        event.preventDefault();
        busy = false;
        error = 'The latest draft could not be saved. Keep this window open and retry.';
      }
    }).then((off) => { unlisten = off; });
    return () => { clearTimeout(timer); clearTimeout(layoutTimer);clearInterval(aiPoll); media.removeEventListener('change',changeMode); unlisten?.(); };
  });
</script>

<svelte:head><title>{project?.title ?? 'World workspace'} · Guided Story 2</title></svelte:head>

<div class="app-shell" inert={showCreate || showType || showRelationshipType || showSettings || showAi || !!structuralEdit || !!backupMode}>
  <header class="app-header">
    <div class="brand" aria-label="Guided Story 2"><img src={icon} alt="" /><span>guided story</span></div>
    <div class="header-actions"><details class="add-menu project-menu"><summary>Project</summary><div><button disabled={!desktop||busy} onclick={event=>{event.currentTarget.closest('details')!.open=false;void openProject();}}>Open project…</button><button disabled={!desktop||busy} onclick={event=>{event.currentTarget.closest('details')!.open=false;showNewProject();}}>New project…</button><button disabled={!desktop||busy||!project} onclick={event=>{event.currentTarget.closest('details')!.open=false;backupMode='export';}}>Export backup…</button><button disabled={!desktop||busy} onclick={event=>{event.currentTarget.closest('details')!.open=false;backupMode='restore';}}>Restore backup…</button></div></details><button class="settings-button quiet-button" onclick={()=>{settingsSection='appearance';showSettings=true;}} aria-label="Settings" title="Settings"><svg viewBox="0 0 24 24" width="19" height="19" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="m9 3-1 3-3 1-2 3 2 2v3l3 1 1 3h6l1-3 3-1v-3l2-2-2-3-3-1-1-3Z"/><circle cx="12" cy="11" r="3"/></svg></button></div>
  </header>
  {#if !desktop}<div class="browser-banner">Browser preview · Project saving is available in the desktop app. Run <code>npm run desktop</code>.</div>{/if}
  {#if appearance.error && !showSettings}<div class="error-banner" role="alert"><span>Settings: {appearance.error}</span><button onclick={()=>{showSettings=true;}}>Open settings</button></div>{/if}
  {#if error}<div class="error-banner" role="alert"><span>{error}</span><div>{#if recoveryPath}<button onclick={recoverProject} disabled={busy}>Recover previous valid save</button>{:else if project}<button onclick={() => { void flush().catch(() => {}); }}>Retry save</button>{/if}<button class="icon-button" aria-label="Dismiss message" onclick={() => { error = ''; }}>×</button></div></div>{/if}
  {#if historyWorld&&surface==='world'}<div class="historical-world-banner"><span>{historyLabel} · world snapshot · read only</span><button class="secondary-button" onclick={()=>showHistory(null,'')}>Current world</button></div>{/if}
  <main class="main-workspace">
    <PaneGroup direction="horizontal">
      <Pane defaultSize={20} minSize={15} maxSize={30}>
        {#if surface==='story'}{#if storyContents}{@render storyContents()}{/if}{:else}
        <Sidebar label="World structure">
          <div class="world-heading"><h1>World</h1><span aria-label={`${records.length} nodes`}>{records.length}</span></div>
          <div class="search-box"><span aria-hidden="true">⌕</span><input aria-label="Search world records" placeholder="Find in your world…" bind:value={search} /></div>
          <nav class="world-tree" aria-label="World records">
            {#each treeRows as record (record.id)}
              {@const count = records.filter((r) => r.parentId === record.id).length}
              <div class="tree-row" class:active={displayWorkspace.selectedId === record.id} style={`--depth: ${recordDepth(record, records)}`}>
                <button class="tree-expander" disabled={!count} aria-label={`${displayWorkspace.expandedIds.includes(record.id) ? 'Collapse' : 'Expand'} ${record.name}`} aria-expanded={count ? displayWorkspace.expandedIds.includes(record.id) : undefined} onclick={() => toggle(record.id)}>{count ? displayWorkspace.expandedIds.includes(record.id) ? '⌄' : '›' : '·'}</button>
                <button class="tree-select" aria-current={displayWorkspace.selectedId === record.id ? 'true' : undefined} onclick={() => select(record.id)}><span class="tree-dot {record.kind}"></span><span>{record.name || 'Unnamed record'}</span></button>
              </div>
            {:else}<p class="empty-tree">{search ? 'No matching records.' : 'Your world is ready for its first place.'}</p>{/each}
          </nav>
        </Sidebar>
        {/if}
      </Pane>
      <PaneResizer class="pane-resizer" />
      <Pane minSize={35}>
        <section class="map-panel" aria-label={surface==='story'?'Story authoring workspace':'World authoring workspace'}>
          <div class="map-toolbar">{#if project}<div class="workspace-tabs" aria-label="Workspace"><button aria-pressed={surface==='world'} onclick={()=>{void switchSurface('world');}}>World</button><button aria-pressed={surface==='story'} onclick={()=>{void switchSurface('story');}}>Story{passageRunning?' · working…':''}</button></div>{/if}<div class="map-tools">
            {#if project&&surface==='world'}<details class="add-menu"><summary>＋ Add node</summary><div>
              {#each world.nodeTypes as type}<button disabled={busy||!!historyWorld} onclick={(event) => { addRecord(type.id); event.currentTarget.closest('details')!.open = false; }}>{type.name}</button>{/each}
              <button disabled={busy||!!historyWorld} class="custom-type-action" onclick={(event) => { showType = true; event.currentTarget.closest('details')!.open = false; }}>Define a custom node type…</button>
              <button disabled={busy||!!historyWorld} onclick={(event) => { showRelationshipType = true; event.currentTarget.closest('details')!.open = false; }}>Define a relationship…</button>
            </div></details>{:else if !project}<button class="secondary-button" disabled={!desktop} onclick={() => { newTitle = 'Brasswater House'; useExample = true; showCreate = true; }}>Save example as a project ↗</button>{/if}
            {#if project&&surface==='world'}<button class="secondary-button" disabled={busy||!!historyWorld||passageRunning||imagePromptRunning||imagePromptRetained} onclick={()=>openAi()}>{aiRunning?'AI · generating…':'AI world tools'}</button>{/if}
          </div></div>
          <div class="workspace-surface" hidden={surface!=='world'}><SvelteFlowProvider><WorldMap storyId={project?.storyId??null} revision={project?.revision??0} world={displayWorld} workspace={displayWorkspace} {search} {colorMode} active={surface==='world'} projectId={project?`${project.storyId}:${historyWorld?historyLabel:'current'}`:null} editable={!!project&&!busy&&!historyWorld} onSelect={select} onToggle={toggle} onPositions={moved} onLink={editLink} onConnect={connect} /></SvelteFlowProvider></div>
          {#if project&&storyOpened}{#key project.storyId}<div class="workspace-surface" hidden={surface!=='story'}><StoryWorkspace bind:this={storyPanel} {project} {world} selectedId={workspace.selectedId} otherBusy={aiRunning||imagePromptRunning||imagePromptRetained||busy} onBeforeAction={beforeAiAction} onAccept={acceptedPassage} onMemoryApply={appliedMemory} onImage={passageImage} onHistory={showHistory} onRun={(running)=>{passageRunning=running;}} onFork={forkPassage} onPanels={registerStoryPanels} onSettings={()=>{void storyPanel?.flushDraft().then(()=>{settingsSection='llm';showSettings=true;}).catch((caught)=>{error=describe(caught);});}} /></div>{/key}{/if}
        </section>
      </Pane>
      <PaneResizer class="pane-resizer" />
      <Pane defaultSize={25} minSize={20} maxSize={36}>
        {#if surface==='story'}{#if storyDetails}{@render storyDetails()}{/if}{:else}
        <Inspector record={selected} {records} world={displayWorld} editable={!!project && !busy&&!historyWorld} historical={!!historyWorld} onEdit={edit} onSelect={select} onProperty={property} onAddCapability={addCapability}
          onAi={openAi} imageStoryId={project?.storyId??null} revision={project?.revision??0} onImage={nodeImage}
          onLink={editLink} onNewLink={connect} onRemoveNode={(id) => reviewEdit({kind:'removeNode',id})}
          onRemoveLink={(id) => reviewEdit({kind:'removeLink',id})} onRemoveCapability={(id,capability) => reviewEdit({kind:'removeCapability',id,capability})} />
        {/if}
      </Pane>
    </PaneGroup>
  </main>
  <footer class="status-bar">{#if !imageTarget&&(imagePromptRunning||imagePromptRetained)}<button class="prompt-activity secondary-button" onclick={reopenPrompt}>{imagePromptRetained?'Prompt reply · save pending':'Image prompt · preparing…'}</button>{/if}<span class="status-text" aria-live="polite"><span class:unsaved={status.includes('failed') || status.includes('attention')} class="live-dot"></span>{status}</span><span class="project-path" title={project?.folderPath}>{layoutError || project?.folderPath || 'Explore freely. Nothing is saved until you create a project.'}</span><span>{project ? `Revision ${project.revision}` : 'JSON project format · v1'}</span></footer>
</div>

{#if backupMode}<BackupDialog mode={backupMode} {project} onBeforeAction={prepareBackup} onOpen={openRestored} onClose={()=>{backupMode=null;}} onWorking={value=>{backupWorking=value;}}/>{/if}

{#if showSettings}<SettingsDialog initialSection={settingsSection} bind:this={settingsDialog} settings={appearance} {systemDark} onClose={()=>{void closeSettings();}} />{/if}
{#if project&&imageTarget&&imageWorlds}<ImageDialog bind:this={imageDialog} {project} target={imageTarget} sourceWorlds={imageWorlds} preferredMediaId={preferredImageId} otherTextBusy={aiRunning||passageRunning||imagePromptRunning||imagePromptRetained} onPromptRun={promptRun} onLlmSettings={()=>{imageTarget=null;imageWorlds=null;settingsSection='llm';showSettings=true;}} onBeforeAction={beforeAiAction} onApply={async result=>{await adopt(result.project);}} onClose={()=>{imageTarget=null;imageWorlds=null;}} onSettings={()=>{imageTarget=null;imageWorlds=null;settingsSection='images';showSettings=true;}}/>{/if}
{#if showAi&&project&&aiRequest}<AiAuthoringDialog bind:this={aiDialog} {project} {world} initialId={aiRunId} initialRequest={aiRequest} onBeforeAction={beforeAiAction} onApply={applyAi} onClose={()=>{showAi=false;}} onSettings={()=>{showAi=false;settingsSection='llm';showSettings=true;}} onRun={(id,running)=>{aiRunId=id;aiRunning=running;}} />{/if}
{#if structuralEdit}<WorldEditDialog {world} edit={structuralEdit} onApply={applyStructure} onClose={() => {structuralEdit = null;}} />{/if}
{#if showRelationshipType}<RelationshipTypeDialog {world} onClose={() => {showRelationshipType=false;}} onCreate={(definition)=>{if(project&&!busy){world={...world,relationTypes:[...world.relationTypes,definition]};scheduleSave();}showRelationshipType=false;}} />{/if}

{#if showCreate}
  <div class="dialog-scrim"></div>
  <dialog open class="project-dialog" aria-labelledby="new-project-title">
    <form onsubmit={createProject}>
      <h2 id="new-project-title">New story</h2>
      <p>A self-contained folder for your world and its history. Choose where to keep it next.</p>
      <label class="field">Story title<input bind:value={newTitle} required maxlength={200} /></label>
      <label class="check-field"><input type="checkbox" bind:checked={useExample} /><span>Use the fictional Brasswater example<small>Leave unchecked to create your own world.</small></span></label>
      <div class="dialog-actions"><button type="button" class="quiet-button" onclick={() => { showCreate = false; }}>Cancel</button><button type="submit" class="primary-button" disabled={busy || !newTitle.trim()}>{busy ? 'Creating…' : 'Choose folder & create'}</button></div>
    </form>
  </dialog>
{/if}

{#if showType}
  <div class="dialog-scrim"></div>
  <dialog open class="project-dialog" aria-labelledby="custom-type-title">
    <form onsubmit={createType}>
      <span class="eyebrow">YOUR WORLD, YOUR TOOLKIT</span><h2 id="custom-type-title">Define a node type.</h2>
      <p>Start from any existing preset or a plain node. Add reusable text fields for the details your world needs.</p>
      <label class="field">Type name<input bind:value={typeName} required maxlength={120} placeholder="Faction, vehicle, weather system…" /></label>
      <label class="field">Starting capabilities<select bind:value={typeBase}><option value="">Plain world node</option>{#each world.nodeTypes as type}<option value={type.id}>{type.name}</option>{/each}</select></label>
      <label class="field">Custom text fields<span class="field-help">Separate labels with commas.</span><input bind:value={typeFields} maxlength={1000} placeholder="Reputation, traditions, current goal" /></label>
      <div class="dialog-actions"><button type="button" class="quiet-button" onclick={()=>{showType=false;}}>Cancel</button><button type="submit" class="primary-button" disabled={!typeName.trim()}>Create type & node</button></div>
    </form>
  </dialog>
{/if}
