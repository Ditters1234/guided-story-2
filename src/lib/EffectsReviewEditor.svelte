<script lang="ts">
  import type {PassageEffects,EffectsReview} from './bindings';
  import type {World} from './model';
  import EffectCard from './EffectCard.svelte';
  import {evidenceIssue,addReviewProposal,proseEditableNode,reviewNodes} from './effects-ui';
  let {world,effects,prose,scopeIds=world.nodes.map(n=>n.id),review,approvedKeys,disabled=false,canApprove=false,onChange,onApprove}:{world:World;effects:PassageEffects;prose:string;scopeIds?:string[];review:EffectsReview|undefined;approvedKeys:string[];disabled?:boolean;canApprove?:boolean;onChange:(value:PassageEffects,invalidated:string[])=>void;onApprove:(keys:string[])=>void}=$props();
  let addError=$state('');
  const entries=$derived(review?.entries??[]);
  const pending=$derived(entries.filter(e=>!approvedKeys.includes(e.key)).length);
  const corrections=$derived(entries.filter(e=>e.issues?.length||evidenceIssue(prose,(e.kind==='change'?effects.changes[e.index]:effects.notes[e.index])?.evidence??'')).length);
  const scoped=$derived(new Set(scopeIds));
  const availableNodes=$derived(world.nodes.filter(n=>scoped.has(n.id)&&proseEditableNode(world,n)&&!effects.changes.some(e=>e.kind==='putNode'&&e.node.id===n.id)));
  const availableLinks=$derived(world.edges.filter(e=>scoped.has(e.sourceId)&&scoped.has(e.targetId)&&!effects.changes.some(c=>c.kind==='putLink'?c.link.id===e.id:c.kind==='removeLink'&&c.id===e.id)));
  const supplied=$derived(world.nodes.filter(n=>scoped.has(n.id)));
  const updated=$derived(new Set(effects.changes.flatMap(e=>e.kind==='putNode'?[e.node.id]:e.kind==='putLink'?[e.link.sourceId,e.link.targetId]:world.edges.filter(link=>link.id===e.id).flatMap(link=>[link.sourceId,link.targetId]))));
  const noted=$derived(new Set(effects.notes.flatMap(n=>n.nodeIds)));
  function add(kind:'node'|'link'|'removeLink'|'note',id?:string){try{onChange(addReviewProposal(world,effects,scopeIds,kind,id),[]);addError='';}catch(caught){addError=caught instanceof Error?caught.message:String(caught);}}
</script>
<div class="effect-review-list">
  <p class="field-help">{effects.changes.length} proposed change{effects.changes.length===1?'':'s'} · {effects.notes.length} continuity note{effects.notes.length===1?'':'s'}</p>
  {#if !effects.changes.length}<p class="story-warning" role="status">No structured changes proposed. Review the prose for missed updates; continuity notes do not update world records.</p>{/if}
  <details class="effect-coverage"><summary>Scene records ({supplied.length})</summary><ul>{#each supplied as node (node.id)}<li>{node.name}<small>{updated.has(node.id)?'Proposed record or connection change':noted.has(node.id)?'Continuity note only':'No proposed update'}</small></li>{/each}</ul><p class="field-help">Add missing records through Scene resources. Validation checks proposals and quotes; review their meaning and coverage.</p></details>
  <details class="effect-add"><summary>Add missing change or note</summary>
    <label class="field">Update scene record<select aria-label="Add node update" value="" disabled={disabled||!canApprove||effects.changes.length>=80} onchange={e=>{if(e.currentTarget.value)add('node',e.currentTarget.value);e.currentTarget.value='';}}><option value="">Choose record…</option>{#each availableNodes as node}<option value={node.id}>{node.name}</option>{/each}</select></label>
    <label class="field">Update scene connection<select aria-label="Add connection update" value="" disabled={disabled||!canApprove||effects.changes.length>=80} onchange={e=>{if(e.currentTarget.value)add('link',e.currentTarget.value);e.currentTarget.value='';}}><option value="">Choose connection…</option>{#each availableLinks as link}<option value={link.id}>{world.nodes.find(n=>n.id===link.sourceId)?.name} → {world.relationTypes.find(r=>r.id===link.typeId)?.label} → {world.nodes.find(n=>n.id===link.targetId)?.name}</option>{/each}</select></label>
    <label class="field">Remove scene connection<select aria-label="Add connection removal" value="" disabled={disabled||!canApprove||effects.changes.length>=80} onchange={e=>{if(e.currentTarget.value)add('removeLink',e.currentTarget.value);e.currentTarget.value='';}}><option value="">Choose connection…</option>{#each availableLinks as link}<option value={link.id}>{world.nodes.find(n=>n.id===link.sourceId)?.name} → {world.relationTypes.find(r=>r.id===link.typeId)?.label} → {world.nodes.find(n=>n.id===link.targetId)?.name}</option>{/each}</select></label>
    <div class="effects-actions"><button type="button" class="text-action" disabled={disabled||!canApprove||effects.changes.length>=80||reviewNodes(world,effects,scopeIds).length<2} onclick={()=>add('link')}>Create connection</button><button type="button" class="text-action" disabled={disabled||!canApprove||effects.notes.length>=12} onclick={()=>add('note')}>Add continuity note</button></div>
    <p class="field-help">Edit the new card and add an exact source quote, then validate and approve.</p>
    {#if addError}<p class="inline-error" role="alert">{addError}</p>{/if}
  </details>
  {#if entries.length}<div class="effect-review-heading"><span>{pending?`${pending} of ${entries.length} need approval`:`All ${entries.length} proposals approved`}</span>{#if pending>1}<button class="text-action" disabled={disabled||!canApprove||!review?.checked} onclick={()=>onApprove(entries.map(e=>e.key))}>Approve all</button>{/if}</div>{#if corrections}<p class="field-help">{corrections} proposal{corrections===1?' needs':'s need'} correction before validation. Edit the marked cards.</p>{:else if !pending&&!review?.checked}<p class="field-help">Approval complete. Validate reviewed changes before accepting.</p>{:else}<p class="field-help">Approvals are saved with this draft. Accept passage applies the reviewed set together.</p>{/if}{/if}
  {#each effects.changes as _,index}<EffectCard {world} {effects} {prose} {scopeIds} kind="change" {index} entry={entries.find(e=>e.kind==='change'&&e.index===index)} {approvedKeys} {disabled} {canApprove} {onChange} {onApprove}/>{/each}
  {#each effects.notes as _,index}<EffectCard {world} {effects} {prose} {scopeIds} kind="note" {index} entry={entries.find(e=>e.kind==='note'&&e.index===index)} {approvedKeys} {disabled} {canApprove} {onChange} {onApprove}/>{/each}
  {#if !effects.changes.length&&!effects.notes.length}<p class="field-help">No world changes or continuity notes proposed.</p>{/if}
</div>
