<script lang="ts">
  import type {PassageEffects,EffectsReview} from './bindings';
  import type {World} from './model';
  import EffectCard from './EffectCard.svelte';
  import {evidenceIssue} from './effects-ui';
  let {world,effects,prose,review,approvedKeys,disabled=false,canApprove=false,onChange,onApprove}:{world:World;effects:PassageEffects;prose:string;review:EffectsReview|undefined;approvedKeys:string[];disabled?:boolean;canApprove?:boolean;onChange:(value:PassageEffects,invalidated:string[])=>void;onApprove:(keys:string[])=>void}=$props();
  const entries=$derived(review?.entries??[]);
  const pending=$derived(entries.filter(e=>!approvedKeys.includes(e.key)).length);
  const corrections=$derived(entries.filter(e=>e.issues?.length||evidenceIssue(prose,(e.kind==='change'?effects.changes[e.index]:effects.notes[e.index])?.evidence??'')).length);
</script>
<div class="effect-review-list">
  {#if entries.length}<div class="effect-review-heading"><span>{pending?`${pending} of ${entries.length} need approval`:`All ${entries.length} proposals approved`}</span>{#if pending>1}<button class="text-action" disabled={disabled||!canApprove||!review?.checked} onclick={()=>onApprove(entries.map(e=>e.key))}>Approve all</button>{/if}</div>{#if corrections}<p class="field-help">{corrections} proposal{corrections===1?' needs':'s need'} correction before validation. Edit the marked cards.</p>{:else if !pending&&!review?.checked}<p class="field-help">Approval complete. Validate reviewed changes before accepting.</p>{:else}<p class="field-help">Approvals are saved with this draft. Accept passage applies the reviewed set together.</p>{/if}{/if}
  {#each effects.changes as _,index}<EffectCard {world} {effects} {prose} kind="change" {index} entry={entries.find(e=>e.kind==='change'&&e.index===index)} {approvedKeys} {disabled} {canApprove} {onChange} {onApprove}/>{/each}
  {#each effects.notes as _,index}<EffectCard {world} {effects} {prose} kind="note" {index} entry={entries.find(e=>e.kind==='note'&&e.index===index)} {approvedKeys} {disabled} {canApprove} {onChange} {onApprove}/>{/each}
  {#if !effects.changes.length&&!effects.notes.length}<p class="field-help">No world changes or continuity notes proposed.</p>{/if}
</div>
