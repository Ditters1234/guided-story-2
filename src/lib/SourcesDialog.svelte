<script lang="ts">
  import {onMount,untrack} from 'svelte';
  import type {World} from './model';
  import type {NodeSourceView_Serialize,SourceReading,SourceEdit,WorldEditPreview_Serialize,StoryIndex_Serialize,PassageReading_Serialize} from './bindings';
  import {commands,unwrap} from './native';
  import {passageQuotes} from './effects-ui';
  let {storyId,world,nodeId,editable,onApply,onClose}:{storyId:string;world:World;nodeId:string;editable:boolean;onApply:(world:World,source:string)=>void;onClose:()=>void}=$props();
  const source=untrack(()=>JSON.stringify(world)),snapshot=JSON.parse(source) as World;
  const name=snapshot.nodes.find(n=>n.id===nodeId)?.name??'Node';
  const isEvent=!!snapshot.nodes.find(n=>n.id===nodeId)?.components['detail.event'];
  let dialog:HTMLDialogElement,entries=$state<NodeSourceView_Serialize[]>([]),index=$state<StoryIndex_Serialize|null>(null),working=$state(false),error=$state('');
  let mode=$state<'none'|'manual'|'passage'>('none'),note=$state(''),search=$state(''),passage=$state<PassageReading_Serialize|null>(null),quote=$state(''),effectIndex=$state('');
  let reading=$state<SourceReading|null>(null),preview=$state<WorldEditPreview_Serialize|null>(null),signature=$state('');
  const input=$derived(JSON.stringify({mode,note,passage:passage?.record,quote,effectIndex}));
  const choices=$derived((index?.passages??[]).filter(p=>p.title.toLowerCase().includes(search.toLowerCase())).slice(-100));
  $effect(()=>{dialog?.showModal();});
  $effect(()=>{if(input!==signature)preview=null;});
  onMount(()=>{void(async()=>{working=true;try{[entries,index]=await Promise.all([commands.nodeSources(storyId,snapshot,nodeId).then(unwrap),commands.loadStory(storyId).then(unwrap)]);}catch(caught){error=String(caught);}finally{working=false;}})();});
  async function selectPassage(id:string){working=true;error='';preview=null;quote='';effectIndex='';try{passage=id?unwrap(await commands.readPassage(storyId,id)):null;}catch(caught){error=String(caught);}finally{working=false;}}
  async function review(edit?:SourceEdit){
    working=true;error='';preview=null;const captured=input;
    try{
      if(!edit){
        if(mode==='manual')edit={kind:'addManual',nodeId,note};
        else if(mode==='passage'&&passage)edit={kind:'addPassage',nodeId,passageId:passage.passage.id,version:passage.record,quote,effectIndex:effectIndex===''?null:Number(effectIndex)};
        else throw Error('Choose an author source or an accepted passage.');
      }
      const result=unwrap(await commands.previewSourceEdit(storyId,snapshot,$state.snapshot(edit)));if(captured===input){signature=captured;preview=result;}
    }catch(caught){error=String(caught);}finally{working=false;}
  }
  async function openSource(id:string){working=true;error='';try{reading=unwrap(await commands.readNodeSource(storyId,snapshot,id));}catch(caught){error=String(caught);}finally{working=false;}}
  function apply(){if(preview&&signature===input){try{onApply($state.snapshot(preview.world),source);}catch(caught){error=String(caught);}}}
</script>
<dialog bind:this={dialog} class="project-dialog node-definition-dialog sources-dialog" aria-labelledby="sources-title" onclose={onClose} oncancel={e=>{if(working)e.preventDefault();}}>
  <form onsubmit={e=>{e.preventDefault();void review();}}>
    <header class="settings-heading"><h2 id="sources-title">Sources · {name}</h2><button type="button" class="icon-button" aria-label="Close sources" disabled={working} onclick={onClose}>×</button></header>
    <div class="node-definition-body">
      {#if reading}
        <div class="section-actions"><strong>{reading.title}{reading.earlierWording?' · earlier wording':''}</strong><button type="button" class="text-action" onclick={()=>{reading=null;}}>Back to citations</button></div>
        <blockquote>{reading.quote}</blockquote><div class="source-prose">{reading.prose}</div>
      {:else}
        {#each entries as entry (entry.annotation.id)}<article class="source-entry">
          <div class="section-actions"><strong>{entry.title}</strong>{#if editable&&!(isEvent&&entries.length===1&&entry.annotation.origin.kind==='manual')}<button type="button" class="text-action danger-text" disabled={working} onclick={()=>{mode='none';void review({kind:'remove',id:entry.annotation.id});}}>Remove citation</button>{/if}</div>
          {#if entry.annotation.origin.kind==='manual'}<p>{entry.annotation.origin.note}</p>
          {:else}<blockquote>{entry.annotation.origin.quote}</blockquote><p class="field-help">{entry.earlierWording?'Evidence from earlier wording':'Accepted text version'}{entry.annotation.origin.effectIndex!==null?' · Change '+((entry.annotation.origin.effectIndex??0)+1):''}{entry.editedSinceSource?' · Node edited since citation':''}</p><button type="button" class="text-action" disabled={working} onclick={()=>{void openSource(entry.annotation.id);}}>Read exact source</button>{/if}
        </article>{:else}<p class="field-help">No citations recorded.</p>{/each}
        {#if editable}
          <fieldset disabled={working}>
            <label class="field">Add source<select bind:value={mode}><option value="none">Choose…</option><option value="manual">Author / backstory</option><option value="passage">Accepted passage</option></select></label>
            {#if mode==='manual'}<label class="field">Author source<textarea bind:value={note} rows={3} maxlength={2000} placeholder="For example: established backstory, before the first chapter."></textarea></label>
            {:else if mode==='passage'}
              <label class="field">Find accepted passage<input bind:value={search}/></label>
              <label class="field">Passage<select value={passage?.passage.id??''} onchange={e=>{void selectPassage(e.currentTarget.value);}}><option value="">Choose passage…</option>{#each choices as entry}<option value={entry.id}>{entry.title}</option>{/each}</select></label>
              {#if passage}
                <label class="field">Source excerpt<select value="" onchange={e=>{if(e.currentTarget.value)quote=e.currentTarget.value;}}><option value="">Choose a continuous quote…</option>{#each passageQuotes(passage.passage.prose) as excerpt}<option value={excerpt}>{excerpt}</option>{/each}</select></label>
                <label class="field">Quote<textarea bind:value={quote} rows={3} maxlength={1000}></textarea></label>
                <label class="field">Specific world change (optional)<select bind:value={effectIndex}><option value="">Passage text</option>{#each passage.passage.effects.changes as change,i}<option value={String(i)}>Change {i+1} · {change.evidence.slice(0,100)}</option>{/each}</select></label>
                <details><summary>Read selected passage</summary><div class="source-prose">{passage.passage.prose}</div></details>
              {/if}
            {/if}
          </fieldset>
        {/if}
      {/if}
      {#if error}<p class="inline-error" role="alert">{error}</p>{/if}
      {#if preview}<div class="change-review"><ul>{#each preview.changes as change}<li>{change}</li>{/each}</ul></div>{/if}
    </div>
    <footer class="settings-footer"><button type="button" class="quiet-button" disabled={working} onclick={onClose}>Close</button>{#if editable&&!reading}{#if preview}<button type="button" class="primary-button" disabled={working} onclick={apply}>Apply source change</button>{:else}<button type="submit" class="primary-button" disabled={working||mode==='none'||(mode==='manual'&&!note.trim())||(mode==='passage'&&(!passage||!quote.trim()))}>Review citation</button>{/if}{/if}</footer>
  </form>
</dialog>
