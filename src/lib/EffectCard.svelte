<script lang="ts">
  import type {PassageEffects,PassageEffect,ContinuityNote,EffectReviewEntry,PropertyValue,WorldNode} from './bindings';
  import type {World} from './model';
  import LinkFields from './LinkFields.svelte';
  import {changeLinkType,setLinkProperty} from './link-fields';
  import NodeFields from './NodeFields.svelte';
  import {defaultProperties} from './node-fields';
  import {evidenceIssue,evidenceQuotes,passageQuotes,proposedNodes,reviewNodes,nodeDetails,effectTitle,linkLabel,nodeDiff,removalPlan,removeProposals,connectionIdentity,linkDiff} from './effects-ui';
  let {world,effects,prose,scopeIds=world.nodes.map(n=>n.id),kind,index,entry,approvedKeys,disabled=false,canApprove=false,onChange,onApprove}:{world:World;effects:PassageEffects;prose:string;scopeIds?:string[];kind:'change'|'note';index:number;entry?:EffectReviewEntry;approvedKeys:string[];disabled?:boolean;canApprove?:boolean;onChange:(value:PassageEffects,invalidated:string[])=>void;onApprove:(keys:string[])=>void}=$props();
  let editing=$state(false),confirmRemove=$state(false),choosingQuote=$state(false);
  const effect=$derived(kind==='change'?effects.changes[index]:null);
  const note=$derived(kind==='note'?effects.notes[index]:null);
  const nodes=$derived(proposedNodes(world,effects));
  const choiceNodes=$derived(reviewNodes(world,effects,scopeIds));
  const scopedLinks=$derived(world.edges.filter(e=>scopeIds.includes(e.sourceId)&&scopeIds.includes(e.targetId)));
  const quote=$derived(effect?.evidence??note?.evidence??'');
  const excerpts=$derived(evidenceQuotes(quote));
  const quoteError=$derived(evidenceIssue(prose,quote));
  const quotes=$derived(choosingQuote?passageQuotes(prose):[]);
  const title=$derived(effect?effectTitle(world,effects,effect):'Continuity note');
  const approved=$derived(!!entry&&approvedKeys.includes(entry.key));
  const issues=$derived(entry?.issues??[]);
  const proposalNode=$derived(effect?.kind==='putNode'?effect.node:null);
  const proposalLink=$derived(effect?.kind==='putLink'?effect.link:null);
  const connection=$derived(proposalLink?connectionIdentity({...world,edges:scopedLinks},proposalLink):'missing');
  const differences=$derived(proposalNode?nodeDiff(world,nodes,proposalNode):[]);
  const isNew=$derived(!!proposalNode&&!world.nodes.some(n=>n.id===proposalNode!.id));
  const displacedLinks=$derived(entry?.displacedLinks??[]);
  const priorLink=$derived(effect?.kind==='removeLink'?world.edges.find(e=>e.id===effect.id):proposalLink?world.edges.find(e=>e.id===proposalLink.id)??displacedLinks[0]:null);
  const removal=$derived(removalPlan(world,effects,kind,index));
  const dependentCount=$derived(removal.changes.length+removal.notes.length-1);
  function change(transform:(value:PassageEffect)=>PassageEffect){onChange({...effects,changes:effects.changes.map((e,i)=>i===index?transform(e):e)},entry?[entry.key]:[]);}
  function changeNote(transform:(value:ContinuityNote)=>ContinuityNote){onChange({...effects,notes:effects.notes.map((n,i)=>i===index?transform(n):n)},entry?[entry.key]:[]);}
  function editNode(transform:(value:WorldNode)=>WorldNode){change(e=>e.kind==='putNode'?{...e,node:transform(e.node)}:e);}
  function property(component:string,key:string,value:PropertyValue){editNode(n=>({...n,components:{...n.components,[component]:{...n.components[component],[key]:value}}}));}
  function setQuote(index:number,value:string){if(kind==='change')change(e=>({...e,evidence:Array.isArray(e.evidence)?e.evidence.map((q,i)=>i===index?value:q):value}));else changeNote(n=>({...n,evidence:value}));}
  function addQuote(){change(e=>({...e,evidence:[...evidenceQuotes(e.evidence),'']}));}
  function removeQuote(index:number){change(e=>({...e,evidence:evidenceQuotes(e.evidence).filter((_,i)=>i!==index)}));}
  function remove(){onChange(removeProposals(effects,removal),[]);confirmRemove=false;}
  const short=(s:string)=>s.length>220?s.slice(0,220)+'…':s;
  function editableField(component:string,key:string){const uses=world.componentTypes.find(d=>d.id===component)?.fields.find(f=>f.key===key)?.metadata?.uses;return !uses?.length||uses.some(u=>u==='story'||u==='visual');}
  function editableLinkField(key:string){const uses=world.relationTypes.find(d=>d.id===proposalLink?.typeId)?.fields?.find(f=>f.key===key)?.metadata?.uses;return !uses?.length||uses.some(u=>u==='story'||u==='visual');}
  function selectConnection(selection:string){
    if(selection==='new')change(e=>e.kind==='putLink'?{...e,link:{...e.link,id:`new:link-${crypto.randomUUID()}`}}:e);
    else{const saved=scopedLinks.find(e=>e.id===selection);if(saved)change(e=>e.kind==='putLink'?{...e,link:{...saved,properties:{...saved.properties}}}:e);}
  }
</script>

{#snippet fullValue(text:string)}{#if text.length>220}<details class="effect-full-value"><summary>{short(text)}</summary><span>{text}</span></details>{:else}{text}{/if}{/snippet}

<article class="effect-card" class:approved aria-label={`${kind==='change'?'Change':'Note'} ${index+1}: ${title}`}>
  <div class="effect-card-heading"><strong>{title}</strong>{#if issues.length||quoteError}<span class="effect-needs-correction">Needs correction</span>{:else if approved}<span class="effect-approved">Approved</span>{/if}</div>
  {#if issues.length}<div class="effect-card-issues" role="alert">{#each issues as issue}<p>{issue}</p>{/each}</div>{/if}
  {#if proposalNode}
    {#if proposalNode.components['detail.event']}<p class="field-help">On acceptance, this quote becomes an exact passage-version citation. Recording the event does not replay other effects.</p>{/if}
    {#if differences.length}<dl class="effect-diff">{#each differences as row}<dt>{row.label}</dt><dd><del>{@render fullValue(row.before)}</del><span aria-hidden="true"> → </span><ins>{@render fullValue(row.after)}</ins></dd>{/each}</dl>
    {:else}<div>{@render fullValue(proposalNode.description||'No story notes.')}</div>{#if proposalNode.visualDescription}<div class="field-help">{@render fullValue(proposalNode.visualDescription)}</div>{/if}
      {#if isNew}<dl class="effect-diff">{#each nodeDetails(world,nodes,proposalNode) as row}<dt>{row.label}</dt><dd>{@render fullValue(row.value)}</dd>{/each}</dl>{/if}
    {/if}
  {:else if effect?.kind==='putLink'}<div class="effect-diff">{#if priorLink}<p><strong>Before:</strong> {linkLabel(world,world.nodes,priorLink)}</p>{/if}<p><strong>After:</strong> {linkLabel(world,nodes,effect.link)}</p>{#each displacedLinks as link}<p><strong>Replaces:</strong> {linkLabel(world,world.nodes,link)} will be removed.</p>{/each}</div><dl class="effect-diff">{#each linkDiff(world,nodes,effect.link,priorLink) as row}<dt>{row.label}</dt><dd><del>{@render fullValue(row.before)}</del> → <ins>{@render fullValue(row.after)}</ins></dd>{/each}</dl>
  {:else if effect?.kind==='removeLink'}<p>{priorLink?linkLabel(world,world.nodes,priorLink):'Missing connection'} will be removed.</p>
  {:else if note}<p>{note.text||'Empty note'}</p>{/if}

  <div class="effect-card-actions">
    <button class={approved?'text-action':'secondary-button'} aria-pressed={approved} disabled={disabled||!canApprove||!entry||(!approved&&(!!quoteError||issues.length>0))} onclick={()=>{if(entry)onApprove(approved?approvedKeys.filter(k=>k!==entry.key):[...approvedKeys,entry.key]);}}>{approved?'Undo approval':'Approve'}</button>
    <button class="text-action" disabled={disabled} aria-expanded={editing} onclick={()=>{editing=!editing;}}>{editing?'Close editor':'Edit'}</button>
    <button class="text-action danger-text" disabled={disabled} onclick={()=>{if(dependentCount)confirmRemove=true;else remove();}}>Remove</button>
  </div>
  {#if confirmRemove}<div class="effect-removal" role="alert"><p>Also remove {dependentCount} dependent proposal{dependentCount===1?'':'s'}?</p><ul>{#each removal.changes.filter(i=>kind!=='change'||i!==index) as i}<li>{effectTitle(world,effects,effects.changes[i])}</li>{/each}{#each removal.notes.filter(i=>kind!=='note'||i!==index) as i}<li>{effects.notes[i].text}</li>{/each}</ul><button class="secondary-button" disabled={disabled} onclick={remove}>Remove these proposals</button><button class="text-action" onclick={()=>{confirmRemove=false;}}>Cancel</button></div>{/if}

  {#if editing}
    <fieldset class="passage-fields effect-fields" {disabled}>
      {#if proposalNode}
        <label class="field">Name<input value={proposalNode.name} maxlength={200} oninput={e=>editNode(n=>({...n,name:e.currentTarget.value}))}/></label>
        <label class="field">Story notes<textarea value={proposalNode.description} rows={3} maxlength={16000} oninput={e=>editNode(n=>({...n,description:e.currentTarget.value}))}></textarea></label>
        <label class="field">Visible description<textarea value={proposalNode.visualDescription} rows={3} maxlength={16000} oninput={e=>editNode(n=>({...n,visualDescription:e.currentTarget.value}))}></textarea></label>
        <NodeFields {world} node={proposalNode} nodes={choiceNodes} {disabled} canEdit={editableField} onChange={property}/>
        {#if isNew}<select aria-label="Add capability to proposed record" value="" onchange={e=>{const id=e.currentTarget.value,definition=world.componentTypes.find(d=>d.id===id);if(definition)editNode(n=>({...n,components:{...n.components,[id]:defaultProperties(definition)}}));e.currentTarget.value='';}}><option value="">Add capability…</option>{#each world.componentTypes.filter(d=>!proposalNode!.components[d.id]) as d}<option value={d.id}>{d.name}</option>{/each}</select>{/if}
      {:else if proposalLink}
        <label class="field">Connection to update<select aria-label="Connection to update" value={connection} onchange={e=>selectConnection(e.currentTarget.value)}>{#if connection==='missing'}<option value="missing" disabled>Outside scene or missing — choose a connection</option>{/if}<option value="new">Create new connection</option>{#each scopedLinks as edge}<option value={edge.id}>{linkLabel(world,nodes,edge)}</option>{/each}</select></label>
        <label class="field">Source<select aria-label="Source" value={proposalLink.sourceId} onchange={e=>change(v=>v.kind==='putLink'?{...v,link:{...v.link,sourceId:e.currentTarget.value}}:v)}>{#if !choiceNodes.some(n=>n.id===proposalLink!.sourceId)}<option value={proposalLink.sourceId}>Missing source</option>{/if}{#each choiceNodes as n}<option value={n.id}>{n.name}</option>{/each}</select></label>
        <label class="field">Relationship<select aria-label="Relationship" value={proposalLink.typeId} onchange={e=>change(v=>v.kind==='putLink'?{...v,link:changeLinkType(world,v.link,e.currentTarget.value)}:v)}>{#if !world.relationTypes.some(r=>r.id===proposalLink!.typeId)}<option value={proposalLink.typeId}>Unknown relationship</option>{/if}{#each world.relationTypes as r}<option value={r.id}>{r.name}</option>{/each}</select></label>
        <label class="field">Target<select aria-label="Target" value={proposalLink.targetId} onchange={e=>change(v=>v.kind==='putLink'?{...v,link:{...v.link,targetId:e.currentTarget.value}}:v)}>{#if !choiceNodes.some(n=>n.id===proposalLink!.targetId)}<option value={proposalLink.targetId}>Missing target</option>{/if}{#each choiceNodes.filter(n=>n.id!==proposalLink!.sourceId) as n}<option value={n.id}>{n.name}</option>{/each}</select></label>
        <label class="field">Connection notes<textarea value={proposalLink.description} rows={2} maxlength={16000} oninput={e=>change(v=>v.kind==='putLink'?{...v,link:{...v.link,description:e.currentTarget.value}}:v)}></textarea></label>
        <LinkFields {world} link={proposalLink} nodes={choiceNodes} {disabled} canEdit={editableLinkField} onChange={(key,value)=>change(v=>v.kind==='putLink'?{...v,link:setLinkProperty(v.link,key,value)}:v)}/>
      {:else if effect?.kind==='removeLink'}
        <label class="field">Connection to remove<select aria-label="Connection to remove" value={effect.id} onchange={e=>change(v=>v.kind==='removeLink'?{...v,id:e.currentTarget.value}:v)}>{#if !world.edges.some(e=>effect?.kind==='removeLink'&&e.id===effect.id)}<option value={effect.id}>Missing connection</option>{/if}{#each scopedLinks as edge}<option value={edge.id}>{linkLabel(world,nodes,edge)}</option>{/each}</select></label>
      {:else if note}
        <label class="field">Note<textarea value={note.text} rows={3} maxlength={2000} oninput={e=>changeNote(n=>({...n,text:e.currentTarget.value}))}></textarea></label>
        <label class="field">About<select multiple value={note.nodeIds} onchange={e=>changeNote(n=>({...n,nodeIds:Array.from(e.currentTarget.selectedOptions,o=>o.value)}))}>{#each choiceNodes as n}<option value={n.id}>{n.name}</option>{/each}</select></label>
      {/if}
    </fieldset>
  {/if}
  <details class="effect-evidence" open={!!quoteError}><summary>Source quote{quoteError?' · needs correction':''}</summary>
    {#if quoteError}<p class="inline-error" role="alert">{quoteError}</p>{/if}
    {#each excerpts as excerpt,i}<div class="effect-quote">
      <label class="field">Evidence quote{excerpts.length>1?` ${i+1}`:''}<textarea value={excerpt} rows={3} {disabled} oninput={e=>setQuote(i,e.currentTarget.value)}></textarea></label>
      <details ontoggle={e=>{if(e.currentTarget.open)choosingQuote=true;}}><summary>Choose quote from passage</summary>{#if choosingQuote}<select aria-label={`Passage quote ${i+1}`} size={4} {disabled} onchange={e=>{if(e.currentTarget.value)setQuote(i,e.currentTarget.value);}}><option value="">Choose a continuous excerpt…</option>{#each quotes as q}<option value={q} title={q}>{q.length>180?q.slice(0,180)+'…':q}</option>{/each}</select>{/if}</details>
      {#if effect&&excerpts.length>1}<button type="button" class="text-action" {disabled} onclick={()=>removeQuote(i)}>Remove quote {i+1}</button>{/if}
    </div>{/each}
    {#if effect&&excerpts.length<4}<button type="button" class="text-action" {disabled} onclick={addQuote}>Add source quote</button>{/if}
  </details>
</article>
