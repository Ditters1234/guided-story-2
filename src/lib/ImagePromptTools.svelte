<script lang="ts">
  import {onMount,untrack} from 'svelte';
  import {commands,unwrap} from './native';
  import {ImagePromptSession} from './image-prompt-session.svelte';
  import type {MediaView,ImagePromptSummary} from './bindings';
  let {storyId,media,disabled,otherTextBusy,onBeforeAction,onApply,onRun,onWorking,onSettings}:{storyId:string;media:MediaView;disabled:boolean;otherTextBusy:boolean;onBeforeAction:()=>Promise<MediaView>;onApply:(media:MediaView)=>void;onRun:(id:string,mediaId:string,running:boolean,retained:boolean)=>void;onWorking:(working:boolean)=>void;onSettings:()=>void}=$props();
  const session=new ImagePromptSession(untrack(()=>storyId));
  let saved=$state<ImagePromptSummary[]>([]),working=$state(false),loaded=$state(false),error=$state(''),notice=$state(''),expanded=$state(false);
  const applied=$derived(session.view?.stage==='applied'),locked=$derived(working||session.running||!!session.view?.retained||disabled);
  $effect(()=>{onWorking(working);});
  $effect(()=>{if(session.running||session.view?.retained)expanded=true;});
  $effect(()=>{if(session.view)onRun(session.view.id,media.draft.id,session.running,session.view.retained);});
  $effect(()=>{const version=media.draft.version;if(loaded)untrack(()=>{void session.refresh().catch(caught=>{error=describe(caught);});});void version;});
  function describe(caught:unknown){return caught instanceof Error?caught.message:String(caught);}
  export async function flushDraft(){await session.flush();if(session.view?.retained){await session.refresh();if(session.view?.retained)throw Error(session.view.error);}}
  async function list(){saved=unwrap(await commands.listImagePromptDrafts(storyId,media.draft.id));}
  async function open(id:string){working=true;try{await session.open(id);error='';notice='';}catch(caught){error=describe(caught);}finally{working=false;}}
  async function fresh(){working=true;error='';notice='';try{await session.flush();const direction=session.value?.direction??'';const current=await onBeforeAction();await session.create(current.draft.id,current.draft.version);session.value!.direction=direction;session.touch();await session.flush();await list();}catch(caught){error=describe(caught);}finally{working=false;}}
  async function generate(){working=true;error='';notice='';try{await session.flush();const current=await onBeforeAction();const direction=session.value?.direction??'';if(!session.view||session.view.stage!=='draft'||session.view.stale){await session.create(current.draft.id,current.draft.version);session.value!.direction=direction;session.touch();await session.flush();}const view=session.view!;session.adopt(unwrap(await commands.startImagePrompt(storyId,view.id,view.version)));await list();}catch(caught){error=describe(caught);try{await session.refresh();}catch{}}finally{working=false;}}
  async function apply(){working=true;error='';notice='';try{await session.flush();await onBeforeAction();const view=session.view!;const result=unwrap(await commands.applyImagePrompt(storyId,view.id,view.version));onApply(result.media);await session.refresh();await list();notice='Prompt applied to this image draft. Generate image when ready.';}catch(caught){error=describe(caught);try{await session.refresh();}catch{}}finally{working=false;}}
  async function parse(){working=true;error='';try{await session.flush();const view=session.view!;session.adopt(unwrap(await commands.parseImagePromptJson(storyId,view.id,view.version)));}catch(caught){error=describe(caught);}finally{working=false;}}
  async function retry(){working=true;try{await session.refresh();await list();error='';}catch(caught){error=describe(caught);}finally{working=false;}}
  onMount(()=>{let disposed=false;void(async()=>{try{await list();const existing=saved.find(s=>s.stage==='running')??saved.find(s=>s.stage==='review')??saved.find(s=>s.stage==='draft')??saved[0];if(existing)await open(existing.id);if(!disposed)loaded=true;}catch(caught){error=describe(caught);}})();const poll=setInterval(()=>{if(session.running)void session.refresh().catch(caught=>{error=describe(caught);});},800);return()=>{disposed=true;clearInterval(poll);session.dispose();onWorking(false);};});
</script>
<details class="image-prompt-tools" bind:open={expanded}><summary>AI prompt tools{session.running?' · working…':''}</summary>
  {#if error||session.error}<p class="inline-error" role="alert">{error||session.error}</p><button class="text-action" disabled={working} onclick={()=>{void retry();}}>Retry / refresh</button>{/if}
  {#if saved.length}<details><summary>Saved suggestions ({saved.length})</summary><div class="image-prompt-history">{#each saved as entry}<button class="secondary-button" disabled={working} onclick={()=>{void open(entry.id);}}>{entry.label} · {entry.stage}</button>{/each}</div></details>{/if}
  {#if session.view?.stale}<p class="story-warning">The prompt, framing, source facts or settings changed. This suggestion is kept; request a new one before applying.</p>{/if}
  {#if session.view?.retained}<p class="story-warning">{session.view.error}</p><button class="secondary-button" disabled={working} onclick={()=>{void retry();}}>Save retained reply</button>{/if}
  {#if !session.view}<button class="secondary-button" disabled={!loaded||working||disabled||media.draft.stage!=='draft'} onclick={()=>{void fresh();}}>Prepare prompt suggestion</button>{:else}
    <fieldset class="image-prompt-fields" disabled={locked||applied}>
      <label class="field">Shot direction<textarea bind:value={session.value!.direction} oninput={()=>session.touch()} maxlength={4000} rows={2} placeholder="Framing, pose, lighting or style…"></textarea></label>
      {#if session.view.stage!=='draft'}<label class="field">Suggested shot<textarea bind:value={session.value!.output.shot} oninput={()=>session.touch()} maxlength={1500} rows={2}></textarea></label><label class="field">Proposed image prompt<textarea bind:value={session.value!.output.prompt} oninput={()=>session.touch()} maxlength={16000} rows={5}></textarea></label>
        {#if session.view.negativeSupported||session.value!.output.negative}<label class="field">Proposed negative prompt<textarea value={session.value!.output.negative??''} oninput={event=>{session.value!.output.negative=event.currentTarget.value||null;session.touch();}} maxlength={4000} rows={2}></textarea></label>{/if}
      {/if}
      {#if session.view.originalOutput||session.value!.rawJson}<details><summary>Response JSON / repair</summary><textarea aria-label="Prompt suggestion JSON" bind:value={session.value!.rawJson} oninput={()=>session.touch()} rows={8} maxlength={600000} spellcheck={false}></textarea><button class="secondary-button" onclick={()=>{void parse();}}>Parse repaired JSON</button></details>{/if}
    </fieldset>
    {#if session.view.provider}<p class="field-help">{session.view.provider}</p>{/if}
    {#if session.view.error&&!session.view.retained}<p class="story-warning">{session.view.error}</p>{/if}
    {#if session.view.originalOutput}<details><summary>Original response</summary><pre class="image-graph">{session.view.originalOutput}</pre></details>{/if}
    {#if session.view.requestJson}<details><summary>Context sent</summary><pre class="image-graph">{session.view.requestJson}</pre></details>{/if}
  {/if}
  <div class="image-tools">
    {#if session.running}<span class="field-help" role="status">Preparing suggestion…</span><button class="secondary-button" disabled={working} onclick={()=>{void commands.cancelAuthoring(session.view!.id).then(unwrap).then(()=>session.refresh()).catch(caught=>{error=describe(caught);});}}>Cancel suggestion</button>
    {:else if session.view}<button class="secondary-button" disabled={working||disabled||otherTextBusy||!!session.view.retained||media.draft.stage!=='draft'} onclick={()=>{void generate();}}>{session.view.stage==='draft'?'Suggest prompt':'Suggest another'}</button><button class="text-action" disabled={working||disabled||!!session.view.retained||media.draft.stage!=='draft'} onclick={()=>{void fresh();}}>New suggestion</button>{/if}
    {#if session.view&&!applied}<button class="primary-button" disabled={working||disabled||!session.ready} onclick={()=>{void apply();}}>Apply prompt</button>{/if}
    <button class="text-action" disabled={working||session.running||disabled} onclick={()=>{void flushDraft().then(onSettings).catch(caught=>{error=describe(caught);});}}>LLM settings</button>
  </div>
  {#if notice}<p class="field-help" role="status">{notice}</p>{/if}
</details>
