<script module lang="ts">
  export type ConnectionStatus={ready:boolean;dirty:boolean;working:boolean;error:string;notice:string};
</script>
<script lang="ts">
  import { onMount } from 'svelte';
  import { commands,desktop,unwrap } from './native';
  import type { ApiKeyUpdate,ConnectionView,LlmConnection } from './bindings';
  import {applyLlmPreset,type LlmPreset} from './llm-presets';
  let {onStatus=()=>{}}:{onStatus?:(status:ConnectionStatus)=>void}=$props();
  let view=$state<ConnectionView|null>(null),draft=$state<LlmConnection|null>(null);
  let models=$state<string[]>([]);
  let apiKey=$state(''),clearKey=$state(false),dirty=$state(false),working=$state(false),error=$state(''),notice=$state('');
  $effect(()=>{onStatus({ready:!!draft,dirty,working,error,notice});});
  async function load(){if(!desktop)return;try{view=unwrap(await commands.loadLlmConnection());draft=structuredClone($state.snapshot(view.settings));apiKey='';clearKey=false;dirty=false;error='';models=[];notice='';}catch(caught){error=String(caught instanceof Error?caught.message:caught);}}
  onMount(()=>{void load();});
  function changed(){dirty=true;notice='';}
  function forgetModels(){models=[];}
  function samplingPreset(preset:LlmPreset){
    if(!draft)return;
    draft=applyLlmPreset(draft,preset);
    changed();
  }
  export async function flushConnection(){
    if(!dirty||!draft)return;
    if(working)throw Error('Wait for the current connection operation.');
    working=true;error='';notice='';
    try{
      const key:ApiKeyUpdate=clearKey?{kind:'clear'}:apiKey?{kind:'replace',value:apiKey}:{kind:'keep'};
      view=unwrap(await commands.saveLlmConnection(draft.version,$state.snapshot(draft),key));draft=structuredClone($state.snapshot(view.settings));apiKey='';clearKey=false;dirty=false;notice='Connection saved.';
    }catch(caught){error=String(caught instanceof Error?caught.message:caught);throw caught;}finally{working=false;}
  }
  export async function testConnection(){
    if(working||!draft)return;
    try{
      await flushConnection();working=true;error='';notice='';models=[];
      const result=unwrap(await commands.testLlmConnection());
      models=result.models;
      if(draft.provider==='compatible'){
        const selected=models.includes(draft.model)?draft.model:models.length===1?models[0]:'';
        if(selected!==draft.model){draft.model=selected;dirty=true;}
      }
      notice=result.message;
    }catch(caught){error=String(caught instanceof Error?caught.message:caught);}finally{working=false;}
  }
</script>
{#if !desktop}<p class="field-help">Connection settings are available in the desktop app.</p>{/if}
{#if draft}
  <form id="llm-connection-form" onsubmit={(event)=>{event.preventDefault();void flushConnection().catch(()=>{});}} oninput={changed} onchange={changed}>
    <fieldset class="connection-form" disabled={working}>
      <label class="field">Provider<select bind:value={draft.provider} onchange={forgetModels}><option value="compatible">OpenAI-compatible API</option><option value="demo">Offline demo · no API calls</option></select></label>
      {#if draft.provider==='demo'}<p class="field-help connection-note">Deterministic sample candidates for trying the review flow. No language model is used.</p>{/if}
      <label class="field">API base URL<input bind:value={draft.baseUrl} type="url" required placeholder="http://127.0.0.1:8888/v1" oninput={forgetModels} /></label>
      <label class="field">Model<select bind:value={draft.model} disabled={draft.provider==='demo'||(!models.length&&!draft.model)}><option value="">{models.length?'Choose a model…':'Test connection to load models'}</option>{#if draft.model&&!models.includes(draft.model)}<option value={draft.model}>{draft.model} · saved</option>{/if}{#each models as model}<option value={model}>{model}</option>{/each}</select><span class="field-help">{draft.provider==='demo'?'No model is needed for the offline demo.':models.length?`${models.length} model${models.length===1?'':'s'} from this connection.`:'Test connection to fetch the server’s available models.'}</span></label>
      <label class="field">API key<input type="password" bind:value={apiKey} autocomplete="off" spellcheck={false} placeholder={view?.hasApiKey?'Enter a replacement key':'Optional for local servers'} maxlength={8192} oninput={forgetModels} /><span class="field-help">{view?.hasApiKey?'A key is saved on this device. Leave blank to keep it.':'Stored with Windows user encryption, outside story folders.'}</span></label>
      {#if view?.hasApiKey}<label class="check-field"><input type="checkbox" bind:checked={clearKey} onchange={forgetModels} /><span>Remove the saved key</span></label>{/if}
      <div class="connection-grid"><label class="field">Temperature<input type="number" bind:value={draft.temperature} min={0} max={2} step={0.1} /></label><label class="field">Output tokens<input type="number" bind:value={draft.maxTokens} min={64} max={32768} /></label><label class="field">Context tokens<input type="number" bind:value={draft.contextTokens} min={2048} max={262144} /></label><label class="field">Timeout (seconds)<input type="number" bind:value={draft.timeoutSeconds} min={10} max={600} /></label></div>
      <details class="connection-advanced"><summary>Sampling & API compatibility</summary>
        <div class="sampling-presets"><button type="button" class="secondary-button" onclick={()=>samplingPreset('qwen')}>Use Qwen 30B defaults</button><button type="button" class="secondary-button" onclick={()=>samplingPreset('generic')}>Use generic API defaults</button><button type="button" class="secondary-button" onclick={()=>samplingPreset('unsloth-gguf')}>Use Unsloth GGUF compatibility</button></div><p class="field-help">Unsloth GGUF selects JSON Schema, direct thinking-off and max_tokens. Model, key, sampling and token budgets stay as configured. Save to apply.</p>
        <div class="connection-grid">
          <label class="field">Top P<input type="number" bind:value={draft.topP} min={0.01} max={1} step={0.01} /></label>
          <label class="field">Top K<input type="number" value={draft.topK??''} min={1} max={1000} placeholder="Server default" oninput={(event)=>{draft!.topK=event.currentTarget.value===''?null:Number(event.currentTarget.value);}} /></label>
          <label class="field">Min P<input type="number" value={draft.minP??''} min={0} max={1} step={0.01} placeholder="Server default" oninput={(event)=>{draft!.minP=event.currentTarget.value===''?null:Number(event.currentTarget.value);}} /></label>
        </div>
        <label class="field">Thinking control<select bind:value={draft.thinking}><option value="provider">Server default</option><option value="chatTemplateOff">Off · chat_template_kwargs</option><option value="directOff">Off · enable_thinking (Guided Story style)</option></select><span class="field-help">Use the option supported by your server. Blank Top K/Min P omits those optional parameters.</span></label>
        <label class="field">Structured output<select bind:value={draft.structuredOutput}><option value="json">JSON object</option><option value="schema">JSON Schema</option><option value="prompt">Prompt only</option></select></label><label class="field">Output token parameter<select bind:value={draft.tokenParameter}><option value="maxTokens">max_tokens · classic/local servers</option><option value="maxCompletionTokens">max_completion_tokens</option></select></label>
      </details>
    </fieldset>
  </form>
{:else if desktop}<button class="secondary-button" onclick={load}>Load connection settings</button>{/if}
{#if error}<p role="alert" class="inline-error">{error}</p><button class="secondary-button" onclick={load}>Reload saved connection</button>{/if}{#if notice}<p class="field-help" role="status">{notice}</p>{/if}
