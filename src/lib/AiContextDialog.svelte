<script lang="ts">
  import {onMount} from 'svelte';
  import type {AiContextPreview,TextPromptTask} from './bindings';
  import type {World} from './model';
  import {commands,unwrap} from './native';
  import {aiContextTasks,defaultContextBinding,setContextBinding,removeContextBinding,addContextNote,addNarrationStyle,addVisualStyle} from './ai-context';
  import AiContextBindingEditor from './AiContextBindingEditor.svelte';
  import NodeFields from './NodeFields.svelte';
  import type {PropertyValue} from './bindings';
  let {world,onChange,onClose}:{world:World;onChange:(value:World)=>void;onClose:()=>Promise<void>}=$props();
  let closeError=$state(''),closing=$state(false);
  async function close(){closing=true;try{await onClose();}catch(caught){closeError=caught instanceof Error?caught.message:String(caught);}finally{closing=false;}}
  let dialog:HTMLDialogElement,selectedId=$state(''),adding=$state(''),task=$state<TextPromptTask|'imagePreparation'>('passage'),preview=$state<AiContextPreview|null>(null),error=$state(''),waiting=$state(false);
  let alive=true,revision=0;
  const bindings=$derived([...(world.aiContext??[])].sort((a,b)=>a.order-b.order||a.nodeId.localeCompare(b.nodeId)));
  const node=$derived(world.nodes.find(n=>n.id===selectedId));
  const binding=$derived((world.aiContext??[]).find(b=>b.nodeId===selectedId));
  $effect(()=>{dialog?.showModal();});
  $effect(()=>{const source=$state.snapshot(world),tool=task;const token=++revision;waiting=true;const timer=setTimeout(()=>{void (tool==='imagePreparation'?commands.previewImageContext(source):commands.previewAiContext(source,tool)).then(unwrap).then(value=>{if(alive&&token===revision){preview=value;error='';}}).catch(caught=>{if(alive&&token===revision){error=caught instanceof Error?caught.message:String(caught);preview=null;}}).finally(()=>{if(alive&&token===revision)waiting=false;});},150);return()=>clearTimeout(timer);});
  onMount(()=>()=>{alive=false;revision++;});
  function edit(field:'name'|'description',value:string){if(node)onChange({...world,nodes:world.nodes.map(n=>n.id===selectedId?{...n,[field]:value}:n)});}
  function property(componentId:string,key:string,value:PropertyValue){if(node)onChange({...world,nodes:world.nodes.map(n=>n.id===selectedId?{...n,components:{...n.components,[componentId]:{...n.components[componentId],[key]:value}}}:n)});}
</script>
<dialog bind:this={dialog} class="project-dialog ai-context-dialog" aria-labelledby="ai-context-title" oncancel={e=>{e.preventDefault();void close();}}>
  <header class="settings-heading"><h2 id="ai-context-title">Story AI context</h2><button class="icon-button" aria-label="Close AI context" disabled={closing} onclick={()=>{void close();}}>×</button></header>
  <div class="ai-context-body">
    <p class="field-help">These nodes are always included for their selected tools. Connected nodes are included only when separately selected.</p>
    {#if closeError}<p class="inline-error" role="alert">{closeError}</p>{/if}
    <div class="ai-context-add"><select aria-label="Add node to AI context" bind:value={adding}><option value="">Choose an existing node…</option>{#each world.nodes.filter(n=>!world.aiContext?.some(b=>b.nodeId===n.id)) as n}<option value={n.id}>{n.name}</option>{/each}</select><button class="secondary-button" disabled={!adding||bindings.length>=32} onclick={()=>{onChange(setContextBinding(world,defaultContextBinding(adding,bindings.length*10,world)));selectedId=adding;adding='';}}>Include node</button><button class="secondary-button" disabled={bindings.length>=32} onclick={()=>{const added=addNarrationStyle(world);onChange(added.world);selectedId=added.node.id;}}>New narration style</button><button class="secondary-button" disabled={bindings.length>=32} onclick={()=>{const added=addVisualStyle(world);onChange(added.world);selectedId=added.node.id;task='imagePreparation';}}>New visual style</button><button class="secondary-button" disabled={bindings.length>=32} onclick={()=>{const added=addContextNote(world);onChange(added.world);selectedId=added.node.id;}}>New context note</button></div>
    <div class="ai-context-list" aria-label="Included nodes">{#each bindings as b}<button class:active={selectedId===b.nodeId} class="secondary-button" onclick={()=>{selectedId=b.nodeId;}}>{world.nodes.find(n=>n.id===b.nodeId)?.name??'Missing node'}<small>{b.enabled?b.role==='instructions'?'Instructions':'Reference facts':'Disabled'} · {b.order}</small></button>{:else}<p class="field-help">No story-wide context nodes selected.</p>{/each}</div>
    {#if node&&binding}<section class="ai-context-editor" aria-label="Edit included node"><div class="section-actions"><h3>{node.name}</h3><button class="text-action danger-text" onclick={()=>{onChange(removeContextBinding(world,selectedId));selectedId='';}}>Remove inclusion</button></div><label class="field">Node name<input value={node.name} maxlength={200} oninput={e=>edit('name',e.currentTarget.value)}/></label><NodeFields {world} {node} onChange={property}/><label class="field">Story notes<textarea value={node.description} rows={5} maxlength={16000} oninput={e=>edit('description',e.currentTarget.value)}></textarea></label><AiContextBindingEditor nodeId={node.id} {world} {binding} onChange={value=>onChange(setContextBinding(world,value))}/></section>{/if}
    <section class="ai-context-preview" aria-label="Included context preview"><h3>Preview</h3><label class="field">AI tool<select aria-label="Preview AI tool" bind:value={task}>{#each aiContextTasks as t}<option value={t.id}>{t.label}</option>{/each}<option value="imagePreparation">Image generation</option></select></label><p class="field-help">This previews the current world. Historical wording and illustrations use their saved passage boundary.</p>
      {#if waiting}<p class="field-help" role="status">Updating preview…</p>{:else if error}<p class="inline-error" role="alert">{error}</p>{:else if preview}
        {#each preview.sections as section}<article><strong>{section.name}</strong><small>{section.role==='instructions'?'Instructions':'Reference facts'} · {section.order}</small><pre>{section.content}</pre></article>{:else}<p class="field-help">No required context nodes for this tool.</p>{/each}
        <details><summary>{task==='imagePreparation'?'Assembled image guidance':'Assembled system instructions'}</summary><pre>{preview.system}</pre></details><details><summary>Reference facts supplied with the task</summary><pre>{JSON.stringify(preview.references,null,2)}</pre></details>
      {/if}
    </section>
  </div>
  <footer class="settings-footer"><span class="field-help">Edits autosave with this story. Preview makes no model request.</span><button class="primary-button" disabled={closing} onclick={()=>{void close();}}>Done</button></footer>
</dialog>
