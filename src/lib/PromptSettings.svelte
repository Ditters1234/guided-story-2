<script lang="ts">
  import {onMount,untrack} from 'svelte';
  import {commands,desktop,unwrap} from './native';
  import type {PromptSettingsView,TextPromptTask,TextPromptTemplate} from './bindings';
  import type {ConnectionStatus} from './LlmSettings.svelte';
  let {onStatus=()=>{},initialTask='passage'}:{onStatus?:(value:ConnectionStatus)=>void;initialTask?:TextPromptTask}=$props();
  let saved=$state<PromptSettingsView|null>(null),templates=$state<TextPromptTemplate[]>([]),selectedId=$state<TextPromptTask>(untrack(()=>initialTask));
  let working=$state(false),error=$state(''),notice=$state('');
  const selected=$derived(templates.find(t=>t.id===selectedId));
  const dirty=$derived(templates.some(t=>t.text!==saved?.templates.find(s=>s.id===t.id)?.text));
  $effect(()=>onStatus({ready:!!saved,dirty,working,error,notice}));
  async function load(){if(!desktop)return;working=true;try{const result=unwrap(await commands.loadPromptTemplates());saved=result;templates=structuredClone(result.templates);error='';notice='';}catch(caught){error=caught instanceof Error?caught.message:String(caught);}finally{working=false;}}
  onMount(()=>{void load();});
  function changed(){notice='';}
  export async function flushPrompts(){
    if(!dirty||!saved)return;
    if(working)throw Error('Wait for prompt settings to finish saving.');
    working=true;error='';notice='';
    try{const overrides=Object.fromEntries(templates.filter(t=>t.text!==t.defaultText).map(t=>[t.id,t.text]));const result=unwrap(await commands.savePromptTemplates(saved.version,overrides));saved=result;templates=structuredClone(result.templates);notice='Prompts saved. Future requests use these instructions.';}
    catch(caught){error=caught instanceof Error?caught.message:String(caught);throw caught;}finally{working=false;}
  }
</script>
<form id="prompt-settings-form" class="prompt-settings" onsubmit={event=>{event.preventDefault();void flushPrompts().catch(()=>{});}}>
  <p class="field-help">Edit each AI tool's base instructions for this device. Story direction, included nodes and output schemas are supplied separately. Story AI context previews the assembled instructions and reference facts.</p>
  {#if error}<div class="settings-error" role="alert"><span>{error}</span><button type="button" class="text-action" disabled={working} onclick={()=>{void load();}}>Reload saved prompts</button></div>{/if}
  {#if !desktop}<p class="field-help">Prompt settings are available in the desktop app.</p>{:else if !saved&&!error}<p class="field-help">Loading prompts…</p>{/if}
  <fieldset class="prompt-settings-fields" disabled={!saved||working}>
    <label class="field">AI tool<select aria-label="AI tool prompt" bind:value={selectedId}>{#each templates as template}<option value={template.id}>{template.label}</option>{/each}</select></label>
    {#if selected}
      <p class="field-help">{selected.description}</p>
      <div class="prompt-editor-heading"><span>{selected.text===selected.defaultText?'Built-in default':'Custom instructions'}</span><button class="text-action" type="button" disabled={selected.text===selected.defaultText} onclick={()=>{selected!.text=selected!.defaultText;changed();}}>Restore default</button></div>
      <label class="field">System instructions<textarea aria-label={`${selected.label} system instructions`} bind:value={selected.text} oninput={changed} rows={16} maxlength={32768} spellcheck={false}></textarea></label>
      <details class="prompt-default"><summary>Show built-in default</summary><pre>{selected.defaultText}</pre></details>
    {/if}
  </fieldset>
</form>
