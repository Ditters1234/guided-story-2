<script lang="ts">
  import { onMount } from 'svelte';
  import { commands,desktop,unwrap } from './native';
  import type { ApiKeyUpdate,ConnectionView,LlmConnection } from './bindings';
  let view=$state<ConnectionView|null>(null),draft=$state<LlmConnection|null>(null);
  let apiKey=$state(''),clearKey=$state(false),dirty=$state(false),working=$state(false),error=$state(''),notice=$state('');
  async function load(){if(!desktop)return;try{view=unwrap(await commands.loadLlmConnection());draft=structuredClone($state.snapshot(view.settings));apiKey='';clearKey=false;dirty=false;error='';}catch(caught){error=String(caught instanceof Error?caught.message:caught);}}
  onMount(()=>{void load();});
  function changed(){dirty=true;notice='';}
  export async function flushConnection(){
    if(!dirty||!draft)return;
    if(working)throw Error('Wait for the current connection operation.');
    working=true;error='';
    try{
      const key:ApiKeyUpdate=clearKey?{kind:'clear'}:apiKey?{kind:'replace',value:apiKey}:{kind:'keep'};
      view=unwrap(await commands.saveLlmConnection(draft.version,$state.snapshot(draft),key));draft=structuredClone($state.snapshot(view.settings));apiKey='';clearKey=false;dirty=false;notice='Connection saved.';
    }catch(caught){error=String(caught instanceof Error?caught.message:caught);throw caught;}finally{working=false;}
  }
  async function test(){try{await flushConnection();working=true;error='';notice=unwrap(await commands.testLlmConnection());}catch(caught){error=String(caught instanceof Error?caught.message:caught);}finally{working=false;}}
</script>
{#if !desktop}<p class="field-help">Connection settings are available in the desktop app.</p>{/if}
{#if draft}
  <form onsubmit={(event)=>{event.preventDefault();void flushConnection().catch(()=>{});}} oninput={changed} onchange={changed}>
    <fieldset class="connection-form" disabled={working}>
      <label class="field">Provider<select bind:value={draft.provider}><option value="compatible">OpenAI-compatible API</option><option value="demo">Offline demo · no API calls</option></select></label>
      {#if draft.provider==='demo'}<p class="field-help connection-note">Deterministic sample candidates for trying the review flow. No language model is used.</p>{/if}
      <label class="field">API base URL<input bind:value={draft.baseUrl} type="url" required placeholder="http://127.0.0.1:8888/v1" /></label>
      <label class="field">Model<input bind:value={draft.model} maxlength={200} placeholder="Model ID from your server" /></label>
      <label class="field">API key<input type="password" bind:value={apiKey} autocomplete="off" spellcheck={false} placeholder={view?.hasApiKey?'Saved · leave blank to keep':'Optional for local servers'} maxlength={8192} /><span class="field-help">Stored with Windows user encryption, outside story folders.</span></label>
      {#if view?.hasApiKey}<label class="check-field"><input type="checkbox" bind:checked={clearKey} /><span>Remove the saved key</span></label>{/if}
      <div class="connection-grid"><label class="field">Temperature<input type="number" bind:value={draft.temperature} min={0} max={2} step={0.1} /></label><label class="field">Output tokens<input type="number" bind:value={draft.maxTokens} min={64} max={32768} /></label><label class="field">Context tokens<input type="number" bind:value={draft.contextTokens} min={2048} max={262144} /></label><label class="field">Timeout (seconds)<input type="number" bind:value={draft.timeoutSeconds} min={10} max={600} /></label></div>
      <details class="connection-advanced"><summary>API compatibility</summary><label class="field">Structured output<select bind:value={draft.structuredOutput}><option value="json">JSON object</option><option value="schema">JSON Schema</option><option value="prompt">Prompt only</option></select></label><label class="field">Output token parameter<select bind:value={draft.tokenParameter}><option value="maxTokens">max_tokens · classic/local servers</option><option value="maxCompletionTokens">max_completion_tokens</option></select></label></details>
      <div class="dialog-actions"><button type="button" class="secondary-button" onclick={test} disabled={working}>Test connection</button><button class="primary-button" type="submit" disabled={working||!dirty}>Save connection</button></div>
    </fieldset>
  </form>
{:else if desktop}<button class="secondary-button" onclick={load}>Load connection settings</button>{/if}
{#if error}<p role="alert" class="inline-error">{error}</p><button class="secondary-button" onclick={load}>Reload saved connection</button>{/if}{#if notice}<p class="field-help" role="status">{notice}</p>{/if}
