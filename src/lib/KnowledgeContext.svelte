<script lang="ts">
  import {commands,unwrap} from './native';
  import type {World} from './model';
  import type {PassagePlan,KnowledgePreview} from './bindings';
  let {world,plan}:{world:World;plan:PassagePlan}=$props();
  let preview=$state<KnowledgePreview|null>(null),error=$state(''),version=0;
  $effect(()=>{const w=$state.snapshot(world),p=$state.snapshot(plan),token=++version;const timer=setTimeout(()=>{void commands.previewKnowledge(w,p).then(unwrap).then(value=>{if(token===version){preview=value;error='';}}).catch(caught=>{if(token===version){preview=null;error=String(caught);}});},180);return()=>{clearTimeout(timer);version++;};});
</script>
<details class="context-coverage claim-context"><summary>Claim context</summary>
  <p class="field-help">Claim access applies to prose and wording. Extraction and world tools are author review. Direction and ordinary notes remain verbatim; keep withheld information in separate claim nodes.</p>
  {#if error}<p class="inline-error" role="alert">{error}</p>{:else if preview}
    {#each preview.decisions as decision}<div class="claim-decision"><p><strong>{decision.name}</strong> · {decision.included?'Included':'Omitted'}</p><p class="field-help">{decision.reason}</p></div>{:else}<p class="field-help">No relevant claim nodes.</p>{/each}
    {#each preview.included as claim}<article><h3>{claim.name}</h3><p>{claim.proposition||'Proposition not filled in.'}</p>{#if claim.notes}<p>{claim.notes}</p>{/if}{#each claim.details as field}<p class="field-help">{field.capability} / {field.label}: {JSON.stringify(field.value)}</p>{/each}<p class="field-help">Author: {claim.authorAssessment} · Reader: {claim.readerDisclosed?'disclosed':'undisclosed'} · Viewpoint: {claim.viewpointStance??'no recorded stance'}</p>{#each claim.holders as holder}<p class="field-help">{holder.name}: {holder.stance}{holder.acquisition?' · '+holder.acquisition:''}</p>{/each}{#if claim.editedSinceSource}<p class="story-warning">Claim edited since citation; review its Sources.</p>{/if}</article>{/each}
    {#if preview.omittedByLimit}<p class="field-help">{preview.omittedByLimit} relevant claims omitted by the 20-claim limit. Select needed claims explicitly as scene references.</p>{/if}
  {/if}
</details>
<style>
  .claim-context article,.claim-decision{margin-top:12px}
  .claim-context article p{margin-top:5px}
</style>
