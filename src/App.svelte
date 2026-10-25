<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { PaneGroup, Pane, PaneResizer } from 'paneforge';
  import { SvelteFlowProvider } from '@xyflow/svelte';
  import Sidebar from './lib/Sidebar.svelte';
  import WorldMap from './lib/WorldMap.svelte';
  import Inspector from './lib/Inspector.svelte';
  import { exampleWorld, emptyWorld, makeNode, type ProjectView, type Workspace, type World } from './lib/model';
  import { worldRecords, visibleRecords, recordDepth, editRecord, type WorldNode, type RecordKind } from './lib/graph';
  import { commands, desktop, unwrap, RejectedChange } from './lib/native';
  import icon from './assets/app-icon.svg';
  import type { CommitResult_Serialize } from './lib/bindings';

  const initialWorld = exampleWorld();
  const initialWorkspace = (): Workspace => ({ version: 0, expandedIds: [...initialWorld.nodes.filter((n) => n.components["core.place"] || n.components["core.character"]).map((n) => n.id)], selectedId: initialWorld.nodes.find((n) => n.components["core.character"])!.id, positions: {} });
  let project = $state<ProjectView | null>(null);
  let world = $state<World>(initialWorld);
  let workspace = $state<Workspace>(initialWorkspace());
  let search = $state('');
  let error = $state('');
  let layoutError = $state('');
  let status = $state('Example world');
  let busy = $state(false);
  let showCreate = $state(false);
  let newTitle = $state('Untitled story');
  let useExample = $state(true);
  let recoveryPath = $state<string | null>(null);
  let editVersion = 0, savedVersion = 0, persistedDraftVersion = 0;
  let timer: ReturnType<typeof setTimeout>;
  let layoutTimer: ReturnType<typeof setTimeout>;
  let inFlight: Promise<void> | null = null;
  let layoutFlight: Promise<void> = Promise.resolve();
  type SaveOperation = { requestId: string; expectedRevision: number; title: string; world: World; version: number };
  let pending: SaveOperation | null = null;
  const records = $derived(worldRecords(world));
  const selected = $derived(records.find((r) => r.id === workspace.selectedId));
  const treeRows = $derived.by(() => {
    const visible = visibleRecords(records, workspace.expandedIds, search);
    const ids = new Set(visible.map((r) => r.id));
    const rows: typeof visible = [];
    function visit(id: string) { const record = visible.find((r) => r.id === id); if (!record || rows.includes(record)) return; rows.push(record); visible.filter((r) => r.parentId === id).forEach((r) => visit(r.id)); }
    visible.filter((r) => !r.parentId || !ids.has(r.parentId)).forEach((r) => visit(r.id));
    return rows;
  });

  function describe(caught: unknown) { return caught instanceof Error ? caught.message : String(caught); }
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
  function select(id: string) { workspace = { ...workspace, selectedId: id }; persistLayout(); }
  function toggle(id: string) {
    workspace = { ...workspace, expandedIds: workspace.expandedIds.includes(id) ? workspace.expandedIds.filter((x) => x !== id) : [...workspace.expandedIds, id] };
    persistLayout();
  }
  function moved(nodes: WorldNode[]) {
    workspace = { ...workspace, positions: { ...workspace.positions, ...Object.fromEntries(nodes.map((n) => [n.id, n.position])) } };
    persistLayout();
  }
  function edit(field: 'name' | 'description' | 'visualDescription', value: string) {
    if (!project || !selected) return;
    world = editRecord(world, selected.id, field, value);
    scheduleSave();
  }

  async function adopt(view: ProjectView) {
    clearTimeout(timer); clearTimeout(layoutTimer);
    project = view; world = view.world;
    editVersion = savedVersion = persistedDraftVersion = 0; pending = null;
    workspace = { version: 0, expandedIds: view.world.nodes.filter((n) => n.components["core.place"]).map((n) => n.id), selectedId: view.world.nodes[0]?.id ?? null, positions: {} };
    search = ''; error = ''; layoutError = ''; recoveryPath = null;
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
  }

  async function prepareSwitch() {
    try { await flush(); await flushLayout(); return true; } catch { error = 'Resolve the unsaved world edits before changing projects. ' + error; return false; }
  }
  async function openProject() {
    if (!desktop || busy || !await prepareSwitch()) return;
    const path = await open({ directory: true, multiple: false, title: 'Open a Guided Story project folder' });
    if (typeof path !== 'string') return;
    busy = true;
    try { await adopt(unwrap(await commands.openProject(path))); }
    catch (caught) { error = describe(caught); recoveryPath = path; }
    finally { busy = false; }
  }
  async function createProject(event: SubmitEvent) {
    event.preventDefault();
    if (!desktop || busy || !newTitle.trim() || !await prepareSwitch()) return;
    const parent = await open({ directory: true, multiple: false, title: 'Choose where to create the new story folder' });
    if (typeof parent !== 'string') return;
    busy = true;
    try {
      await adopt(unwrap(await commands.createProject(parent, newTitle.trim(), useExample ? exampleWorld() : emptyWorld(), crypto.randomUUID())));
      showCreate = false;
    } catch (caught) { error = describe(caught); }
    finally { busy = false; }
  }
  async function recoverProject() {
    if (!recoveryPath || busy || !await prepareSwitch()) return;
    busy = true;
    try { await adopt(unwrap(await commands.recoverProject(recoveryPath))); }
    catch (caught) { error = describe(caught); }
    finally { busy = false; }
  }
  function addRecord(typeId: string) {
    if (!project || busy) return;
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
    if (!project || !selected) return;
    world = {...world,nodes:world.nodes.map((node)=>node.id===selected!.id?{...node,components:{...node.components,[componentId]:{...node.components[componentId],[key]:value}}}:node)};
    scheduleSave();
  }
  function addCapability(componentId: string) {
    const definition = world.componentTypes.find((d)=>d.id===componentId);
    if (!definition || !project || !selected) return;
    world = {...world,nodes:world.nodes.map((node)=>node.id===selected!.id?{...node,components:{...node.components,[componentId]:Object.fromEntries(definition.fields.map((f)=>[f.key,structuredClone(f.defaultValue)]))}}:node)};
    scheduleSave();
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
    if (desktop) void commands.startupProject().then(unwrap).then(async (saved) => { if (saved) await adopt(saved); }).catch((caught) => { error = describe(caught); });
    if (desktop) void getCurrentWindow().onCloseRequested(async (event) => {
      if (!project) return;
      try { await flush(); } catch { /* A persisted invalid form remains recoverable. */ }
      await flushLayout();
      if (savedVersion < editVersion && persistedDraftVersion < editVersion) {
        event.preventDefault();
        error = 'The latest draft could not be saved. Keep this window open and retry.';
      }
    }).then((off) => { unlisten = off; });
    return () => { clearTimeout(timer); clearTimeout(layoutTimer); unlisten?.(); };
  });
</script>

<svelte:head><title>{project?.title ?? 'World workspace'} · Guided Story 2</title></svelte:head>

<div class="app-shell" inert={showCreate || showType}>
  <header class="app-header">
    <div class="brand" aria-label="Guided Story 2"><img src={icon} alt="" /><span>guided story<span class="brand-edition">WORKSPACE</span></span></div>
    <div class="header-divider"></div>
    <div class="project-identity"><strong>{project?.title ?? 'Brasswater House'}</strong><span>{project ? 'Story foundation' : 'A fictional example world'}</span></div>
    <div class="header-actions"><span class="build-tag">FOUNDATION PREVIEW</span><button class="quiet-button" onclick={openProject} disabled={!desktop || busy}>Open project</button><button class="primary-button" onclick={() => { showCreate = true; }} disabled={!desktop || busy}>＋ New project</button></div>
  </header>
  {#if !desktop}<div class="browser-banner">Browser preview · Project saving is available in the desktop app. Run <code>npm run desktop</code>.</div>{/if}
  {#if error}<div class="error-banner" role="alert"><span>{error}</span><div>{#if recoveryPath}<button onclick={recoverProject} disabled={busy}>Recover previous valid save</button>{:else if project}<button onclick={() => { void flush().catch(() => {}); }}>Retry save</button>{/if}<button class="icon-button" aria-label="Dismiss message" onclick={() => { error = ''; }}>×</button></div></div>{/if}
  <main class="main-workspace">
    <PaneGroup direction="horizontal">
      <Pane defaultSize={20} minSize={15} maxSize={30}>
        <Sidebar label="World structure">
          <div class="panel-heading"><span>YOUR STORY</span><span class="tiny-mark">✧</span></div>
          <div class="workspace-tabs"><button class="active">World</button><button disabled title="Passage writing comes in the next milestone">Contents <small>0</small></button></div>
          <div class="search-box"><span aria-hidden="true">⌕</span><input aria-label="Search world records" placeholder="Find in your world…" bind:value={search} /><kbd>⌕</kbd></div>
          <div class="structure-label"><span>PLACES & CONNECTIONS</span><span>{records.length}</span></div>
          <nav class="world-tree" aria-label="World records">
            {#each treeRows as record (record.id)}
              {@const count = records.filter((r) => r.parentId === record.id).length}
              <div class="tree-row" class:active={workspace.selectedId === record.id} style={`--depth: ${recordDepth(record, records)}`}>
                <button class="tree-expander" disabled={!count} aria-label={`${workspace.expandedIds.includes(record.id) ? 'Collapse' : 'Expand'} ${record.name}`} aria-expanded={count ? workspace.expandedIds.includes(record.id) : undefined} onclick={() => toggle(record.id)}>{count ? workspace.expandedIds.includes(record.id) ? '⌄' : '›' : '·'}</button>
                <button class="tree-select" aria-current={workspace.selectedId === record.id ? 'true' : undefined} onclick={() => select(record.id)}><span class="tree-dot {record.kind}"></span><span>{record.name || 'Unnamed record'}</span></button>
              </div>
            {:else}<p class="empty-tree">{search ? 'No matching records.' : 'Your world is ready for its first place.'}</p>{/each}
          </nav>
          <div class="sidebar-bottom"><span class="eyebrow">A WORLD THAT CAN GROW</span><p>Places hold people.<br />People carry stories.</p><span class="field-help">Select a connection to explore it.</span></div>
        </Sidebar>
      </Pane>
      <PaneResizer class="pane-resizer" />
      <Pane minSize={35}>
        <section class="map-panel" aria-label="World authoring workspace">
          <div class="map-toolbar"><div><span class="eyebrow">STORY FOUNDATION</span><h1>Your world, connected.</h1></div><div class="map-tools">{#if project}<details class="add-menu"><summary>＋ Add manually</summary><div>{#each world.nodeTypes as type}<button onclick={(event) => { addRecord(type.id); event.currentTarget.closest('details')!.open = false; }}>{type.name}</button>{/each}<button class="custom-type-action" onclick={(event) => { showType = true; event.currentTarget.closest('details')!.open = false; }}>Define a custom node type…</button></div></details>{:else}<button class="secondary-button" disabled={!desktop} onclick={() => { newTitle = 'Brasswater House'; useExample = true; showCreate = true; }}>Save example as a project ↗</button>{/if}</div></div>
          <div class="map-caption"><span class="live-dot"></span><span>{project ? 'Current world' : 'Explore the example'}</span><span class="caption-divider">/</span><span>Expand a node to discover its connections</span></div>
          <SvelteFlowProvider><WorldMap {world} {workspace} {search} onSelect={select} onToggle={toggle} onPositions={moved} /></SvelteFlowProvider>
          <div class="map-legend"><span><i class="tree-dot place"></i>Places</span><span><i class="tree-dot character"></i>Characters</span><span><i class="tree-dot item"></i>Items</span><span><i class="tree-dot thread"></i>Threads</span></div>
          <div class="writing-intro"><span class="writing-icon">✎</span><div><h3>First, give the story somewhere to happen.</h3><p>Build and edit your world here. Scene planning and passage generation are the next milestone.</p></div></div>
        </section>
      </Pane>
      <PaneResizer class="pane-resizer" />
      <Pane defaultSize={25} minSize={20} maxSize={36}>
        <Inspector record={selected} {records} {world} editable={!!project && !busy} onEdit={edit} onSelect={select} onProperty={property} onAddCapability={addCapability} />
      </Pane>
    </PaneGroup>
  </main>
  <footer class="status-bar"><span class="status-text" aria-live="polite"><span class:unsaved={status.includes('failed') || status.includes('attention')} class="live-dot"></span>{status}</span><span class="project-path" title={project?.folderPath}>{layoutError || project?.folderPath || 'Explore freely. Nothing is saved until you create a project.'}</span><span>{project ? `Revision ${project.revision}` : 'JSON project format · v1'}</span></footer>
</div>

{#if showCreate}
  <div class="dialog-scrim"></div>
  <dialog open class="project-dialog" aria-labelledby="new-project-title">
    <form onsubmit={createProject}>
      <span class="eyebrow">A NEW BEGINNING</span><h2 id="new-project-title">Create your story project.</h2>
      <p>A self-contained folder for your world and its history. Choose where to keep it next.</p>
      <label class="field">Story title<input bind:value={newTitle} required maxlength={200} /></label>
      <label class="check-field"><input type="checkbox" bind:checked={useExample} /><span>Start with the Brasswater example<small>Uncheck for an empty world.</small></span></label>
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
