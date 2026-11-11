<script lang="ts">
  import {onMount} from 'svelte';
  import {commands,desktop,unwrap} from './native';
  import type {ImageConnection} from './bindings';
  import type {ConnectionStatus} from './LlmSettings.svelte';
  let {onStatus}:{onStatus:(status:ConnectionStatus)=>void}=$props();
  let value=$state<ImageConnection|null>(null),saved=$state(''),working=$state(false),error=$state(''),notice=$state(''),profileJson=$state('');
  const dirty=$derived(!!value&&JSON.stringify(value)!==saved);
  $effect(()=>onStatus({ready:!!value,dirty,working,error,notice}));
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  async function load(){try{value=unwrap(await commands.loadImageSettings());saved=JSON.stringify(value);profileJson=JSON.stringify(value.profile,null,2);error='';}catch(caught){error=describe(caught);}}
  export async function flushConnection(){if(working)throw Error('Wait for the image connection operation to finish.');if(!value||!dirty)return;working=true;try{value=unwrap(await commands.saveImageSettings(value.version,JSON.parse(JSON.stringify(value)) as ImageConnection));saved=JSON.stringify(value);notice='Image connection saved.';error='';}catch(caught){error=describe(caught);throw caught;}finally{working=false;}}
  export async function testConnection(){if(!value||working)return;working=true;error='';notice='';try{const result=unwrap(await commands.testImageConnection(JSON.parse(JSON.stringify(value)) as ImageConnection));if(result.ready)notice='Connected; workflow classes and installed choices are available.';else error=result.issues.join('\n');}catch(caught){error=describe(caught);}finally{working=false;}}
  async function preset(desktopLayout:boolean){try{if(!value)return;value.profile=unwrap(await commands.bundledImageProfile(desktopLayout));profileJson=JSON.stringify(value.profile,null,2);notice='Workflow loaded for review. Save connection to use it.';error='';}catch(caught){error=describe(caught);}}
  function applyJson(){try{if(!value)return;if(profileJson.length>1_100_000)throw Error('Workflow profiles must be at most 1 MB.');value.profile=JSON.parse(profileJson);notice='Profile loaded for validation on save.';error='';}catch(caught){error=describe(caught);}}
  async function importProfile(event:Event){const input=event.target as HTMLInputElement,file=input.files?.[0];if(!file)return;try{if(file.size>1_100_000)throw Error('Workflow profiles must be at most 1 MB.');profileJson=await file.text();applyJson();}catch(caught){error=describe(caught);}finally{input.value='';}}
  onMount(()=>{if(desktop)void load();else error='Image connections are available in the desktop app.';});
</script>
{#if error}<p class="inline-error image-settings-error" role="alert">{error}</p>{/if}
{#if !value}<button class="secondary-button" onclick={()=>{void load();}}>Reload image settings</button>{:else}<form id="image-connection-form" onsubmit={event=>{event.preventDefault();void flushConnection().catch(()=>{});}}><fieldset class="connection-fields" disabled={working}>
  <label class="field">ComfyUI base URL<input bind:value={value.baseUrl} maxlength={2000}/></label>
  <label class="field">HTTP timeout (seconds)<input type="number" bind:value={value.timeoutSeconds} min={5} max={300}/></label>
  <p class="field-help">Direct ComfyUI connection. Test checks server and workflow availability without generating an image.</p>
  <label class="field">Workflow<input value={value.profile?.name??'Custom profile'} readonly/></label>
  <div class="image-tools"><button type="button" class="secondary-button" onclick={()=>{void preset(true);}}>Guided Story Krea 2 · desktop</button><button type="button" class="secondary-button" onclick={()=>{void preset(false);}}>Original paths</button></div>
  <details><summary>Custom workflow profile</summary><p class="field-help">API graph plus prompt/control/output bindings. Use a separate profile for Z-Image, MOSS or other models.</p><label class="secondary-button file-button">Import profile JSON<input type="file" accept=".json,application/json" onchange={importProfile}/></label><label class="field">Profile JSON<textarea bind:value={profileJson} rows={14} maxlength={1100000} spellcheck={false}></textarea></label><button type="button" class="secondary-button" onclick={applyJson}>Load profile JSON</button></details>
  {#if notice}<p class="field-help" role="status">{notice}</p>{/if}
</fieldset></form>{/if}
