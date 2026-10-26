<script lang="ts">
  import { untrack } from 'svelte';
  import type { AppearanceSettings } from './appearance.svelte';
  import type { ThemeDefinition } from './bindings';
  import { builtInThemes, colorKeys, copyTheme, downloadTheme, parseThemeJson, resolveTheme } from './appearance';
  import { desktop } from './native';
  import LlmSettings, {type ConnectionStatus} from './LlmSettings.svelte';
  let {settings,systemDark,onClose}:{settings:AppearanceSettings;systemDark:boolean;onClose:()=>void}=$props();
  const sections=[{id:'appearance',label:'Appearance'},{id:'themes',label:'Custom themes'},{id:'llm',label:'LLM'}] as const;
  let section=$state<'appearance'|'themes'|'llm'>('appearance');
  let llmPanel=$state<{flushConnection:()=>Promise<void>;testConnection:()=>Promise<void>}>();
  let connectionStatus=$state<ConnectionStatus>({ready:false,dirty:false,working:false,error:'',notice:''});
  export async function flushSettings(){try{await llmPanel?.flushConnection();}catch(error){section='llm';throw error;}}
  async function close(){try{await flushSettings();onClose();}catch{section='llm';}}
  let dialog: HTMLDialogElement;
  let draft=$state<ThemeDefinition>(untrack(()=>copyTheme(resolveTheme($state.snapshot(settings.value),systemDark))));
  let themeError=$state('');
  let themeNotice=$state('');
  let json=$state('');
  let showJson=$state(false);
  const activeTheme=$derived(resolveTheme(settings.value,systemDark));
  const allThemes=$derived([...builtInThemes,...settings.value.customThemes]);
  const colorLabels:Record<string,string>={background:'Window',surface:'Panels & nodes',surfaceAlt:'Fields & sections',text:'Text',muted:'Secondary text',border:'Borders',accent:'Accent',accentText:'Accent text',selection:'Selection',canvas:'Map background',canvasDots:'Map dots',connection:'Connections'};
  $effect(()=>{dialog?.showModal();});
  function editTheme(theme:ThemeDefinition) { draft=structuredClone($state.snapshot(theme)); themeError=''; themeNotice=''; showJson=false; }
  function newTheme() { editTheme(copyTheme($state.snapshot(activeTheme))); }
  function saveTheme() {
    try {
      const theme=parseThemeJson(JSON.stringify(draft));
      const themes=settings.value.customThemes.filter((t)=>t.id!==theme.id);
      if(themes.length>=12)throw Error('Remove a custom theme before adding another.');
      settings.update({customThemes:[...themes,theme],themeId:theme.id});
      draft=theme; themeError=''; themeNotice='Theme saved and selected.';
    } catch(error) {themeError=error instanceof Error?error.message:String(error);}
  }
  function removeTheme(id:string) {
    settings.update({customThemes:settings.value.customThemes.filter((t)=>t.id!==id),themeId:settings.value.themeId===id?'system':settings.value.themeId});
    if(draft.id===id)newTheme();
  }
  function loadJson() {try {editTheme(parseThemeJson(json));themeNotice='Theme loaded for editing. Save theme to add it.';}catch(error){themeError=error instanceof Error?error.message:String(error);}}
  async function importTheme(event:Event) {
    const input=event.target as HTMLInputElement,file=input.files?.[0];
    if(!file)return;
    try {if(file.size>64*1024)throw Error('Theme files must be smaller than 64 KB.');editTheme(parseThemeJson(await file.text()));themeNotice='Theme imported for editing. Save theme to add it.';}
    catch(error){themeError=error instanceof Error?error.message:String(error);}
    finally{input.value='';}
  }
  function tabKey(event:KeyboardEvent,index:number) {
    const delta=event.key==='ArrowRight'?1:event.key==='ArrowLeft'?-1:0;
    if(!delta&&event.key!=='Home'&&event.key!=='End')return;
    event.preventDefault();
    const next=event.key==='Home'?0:event.key==='End'?sections.length-1:(index+delta+sections.length)%sections.length;
    section=sections[next].id; document.getElementById('settings-tab-'+section)?.focus();
  }
</script>

<dialog bind:this={dialog} class="project-dialog settings-dialog" aria-labelledby="settings-title" oncancel={(event)=>{event.preventDefault();void close();}}>
  <header class="settings-heading"><h2 id="settings-title">Settings</h2><button class="icon-button" aria-label="Close settings" onclick={()=>{void close();}}>×</button></header>
  <div class="settings-tabs" role="tablist" aria-label="Settings categories">
    {#each sections as tab,index}<button id={'settings-tab-'+tab.id} role="tab" aria-selected={section===tab.id} aria-controls={'settings-panel-'+tab.id} tabindex={section===tab.id?0:-1} onclick={()=>{section=tab.id;}} onkeydown={(event)=>tabKey(event,index)}>{tab.label}</button>{/each}
  </div>
  {#if settings.error}<div class="settings-error" role="alert"><span>{settings.error}</span><div><button class="quiet-button" onclick={()=>{if(settings.ready)void settings.flush().catch(()=>{});else void settings.load();}}>Retry</button>{#if settings.ready}<button class="quiet-button" onclick={()=>{void settings.load();}}>Reload settings</button>{/if}</div></div>{/if}
  <div class="settings-body">
    <fieldset disabled={!settings.ready} class="settings-fields">
    <div id="settings-panel-appearance" role="tabpanel" tabindex="0" aria-labelledby="settings-tab-appearance" hidden={section!=='appearance'}>
      <h3>Theme</h3>
      <div class="theme-options">
        <button class="theme-option" class:chosen={settings.value.themeId==='system'} aria-pressed={settings.value.themeId==='system'} onclick={()=>settings.update({themeId:'system'})}><span class="theme-system-swatch">◐</span><strong>System</strong></button>
        {#each allThemes as theme}<button class="theme-option" class:chosen={settings.value.themeId===theme.id} aria-pressed={settings.value.themeId===theme.id} onclick={()=>settings.update({themeId:theme.id})}><span class="theme-swatch" style={`background:${theme.colors.surface};color:${theme.colors.text};border-color:${theme.colors.border}`}><span style={`background:${theme.colors.accent}`}></span>Aa</span><strong>{theme.name}</strong></button>{/each}
      </div>
      <div class="settings-row"><label for="settings-text-size">Text size <output>{settings.value.textScale}%</output></label><div class="text-size-control"><span aria-hidden="true">A</span><input id="settings-text-size" type="range" min={85} max={140} step={5} value={settings.value.textScale} oninput={(event)=>settings.update({textScale:Number(event.currentTarget.value)})} /><span aria-hidden="true">A</span><button class="text-action" onclick={()=>settings.update({textScale:100})}>Reset</button></div></div>
      <label class="check-field"><input type="checkbox" checked={settings.value.reducedMotion} onchange={(event)=>settings.update({reducedMotion:event.currentTarget.checked})} /><span>Reduce motion</span></label>
    </div>
    <div id="settings-panel-themes" role="tabpanel" tabindex="0" aria-labelledby="settings-tab-themes" hidden={section!=='themes'}>
      <div class="section-actions"><h3>Custom themes</h3><div class="theme-file-actions"><label class="secondary-button file-button">Import JSON<input type="file" accept=".json,application/json" onchange={importTheme} /></label><button class="secondary-button" onclick={newTheme}>New from current</button></div></div>
      {#if settings.value.customThemes.length}<div class="custom-theme-list">{#each settings.value.customThemes as theme}<div class="custom-theme-row"><span>{theme.name}</span><div><button class="text-action" onclick={()=>editTheme(theme)} aria-label={`Edit ${theme.name} theme`}>Edit</button><button class="text-action danger-text" onclick={()=>removeTheme(theme.id)} aria-label={`Remove ${theme.name} theme`}>Remove</button></div></div>{/each}</div>{/if}
      <label class="field">Theme name<input bind:value={draft.name} maxlength={60} /></label>
      <label class="field">Control style<select bind:value={draft.mode}><option value="light">Light</option><option value="dark">Dark</option></select></label>
      <div class="theme-colors">{#each colorKeys as key}<label><input type="color" bind:value={draft.colors[key]} /><span>{colorLabels[key]}</span></label>{/each}</div>
      <div class="theme-editor-actions"><button class="text-action" onclick={()=>{showJson=!showJson;if(showJson)json=JSON.stringify(draft,null,2);}}>{showJson?'Hide JSON':'Edit JSON'}</button><button class="secondary-button" onclick={()=>downloadTheme($state.snapshot(draft))}>Export JSON</button><button class="primary-button" onclick={saveTheme} disabled={!draft.name.trim()}>Save theme</button></div>
      {#if showJson}<label class="field">Theme JSON<textarea class="theme-json" bind:value={json} rows={9} maxlength={65536} spellcheck={false}></textarea></label><button class="secondary-button" onclick={loadJson}>Load JSON into editor</button>{/if}
      {#if themeError}<p class="inline-error" role="alert">{themeError}</p>{/if}{#if themeNotice}<p class="field-help" role="status">{themeNotice}</p>{/if}
    </div>
    <div id="settings-panel-llm" role="tabpanel" tabindex="0" aria-labelledby="settings-tab-llm" hidden={section!=='llm'}><LlmSettings bind:this={llmPanel} onStatus={(next)=>{connectionStatus=next;}} /></div>
    </fieldset>
  </div>
  <footer class="settings-footer">
    <span aria-live="polite" class:danger-text={section==='llm'&&!!connectionStatus.error}>
      {#if section==='llm'}{connectionStatus.working?'Working…':connectionStatus.error||(connectionStatus.dirty?'Unsaved connection changes':connectionStatus.notice||(desktop?'Connection settings':'Desktop connection settings unavailable'))}
      {:else}{settings.error?'Settings not saved':!settings.ready?'Loading…':settings.saving?'Saving…':settings.dirty?'Unsaved settings':desktop?'Appearance saved on this device':'Preview settings · this tab only'}{/if}
    </span>
    <div class="settings-footer-actions">
      {#if section==='llm'}
        <button type="button" class="secondary-button" disabled={!connectionStatus.ready||connectionStatus.working||!settings.ready} onclick={()=>{void llmPanel?.testConnection();}}>Test connection</button>
        <button class="primary-button" type="submit" form="llm-connection-form" disabled={!connectionStatus.ready||connectionStatus.working||!connectionStatus.dirty||!settings.ready}>Save connection</button>
      {/if}
      <button class="primary-button" disabled={connectionStatus.working} onclick={()=>{void close();}}>Done</button>
    </div>
  </footer>
</dialog>
