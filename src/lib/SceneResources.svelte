<script lang="ts">
  import type {PassagePlan,SceneContextPreview,SceneResourceRole} from './bindings';
  import type {World} from './model';
  import {sceneConditionChoices,sceneReferenceSuggestions} from './scene-plan';

  let {world,plan,context,updating,prose='',onToggle}:{
    world:World;plan:PassagePlan;context?:SceneContextPreview;updating:boolean;
    prose?:string;
    onToggle:(id:string,role:'participant'|'reference')=>void;
  }=$props();
  let search=$state('');
  let showAutomatic=$state(false);
  const names=$derived(new Map(world.nodes.map(node=>[node.id,node.name])));
  const roles=$derived(new Set([plan.locationId,plan.viewpointId,...plan.participantIds,...plan.referenceIds]));
  const automatic=$derived(context?.resources.filter(resource=>!roles.has(resource.nodeId))??[]);
  const added=$derived([
    ...plan.participantIds.map(id=>({id,role:'participant' as const})),
    ...plan.referenceIds.map(id=>({id,role:'reference' as const})),
  ]);
  const query=$derived(search.trim().toLowerCase());
  const conditions=$derived(sceneConditionChoices(world,plan));
  const suggested=$derived(sceneReferenceSuggestions(world,plan,context?.resources.map(r=>r.nodeId)??[],prose));
  const choices=$derived(world.nodes.filter(node=>!roles.has(node.id)&&node.name.toLowerCase().includes(query)).slice(0,50));
  const labels:Record<SceneResourceRole,string>={focus:'Scene focus',viewpoint:'Viewpoint',participant:'Participant',reference:'Reference',inventory:'Cast inventory',containingPlace:'Placement location',placement:'Placement dependency',claim:'Claim context'};
</script>

<section class="scene-resources" aria-label="Scene resources">
  <h3>Scene resources</h3>
  {#if suggested.suggestions.length}<details class="scene-reference-suggestions">
    <summary>Suggested references ({suggested.suggestions.length})</summary>
    <p class="field-help">Add relevant records before checking changes. Suggestions are not sent until added.</p>
    <div class="scene-resource-list">{#each suggested.suggestions as resource (resource.id)}
      <div><span>{resource.name}<small>{resource.reason}</small></span><button type="button" class="text-action" disabled={updating||plan.referenceIds.length>=12} onclick={()=>onToggle(resource.id,'reference')}>Add reference</button></div>
    {/each}</div>
    {#if suggested.omitted}<p class="field-help">{suggested.omitted} more nearby references; use Find a world node below.</p>{/if}
    {#if plan.referenceIds.length>=12}<p class="field-help">Remove a reference before adding another (limit 12).</p>{/if}
  </details>{/if}
  {#if conditions.length}<div class="scene-resource-list" aria-label="Conditions linked to scene focus">
    {#each conditions.slice(0,12) as condition (condition.id)}
      <div><span>{condition.name}</span><button type="button" class="text-action" disabled={updating||plan.referenceIds.length>=12} onclick={()=>onToggle(condition.id,'reference')}>Add conditions to scene context</button></div>
      {#if condition.scope}<p class="field-help">{condition.scope}</p>{/if}
    {/each}
    {#if conditions.length>12}<p class="field-help">Showing 12 of {conditions.length} linked conditions; use Add references to find others.</p>{/if}
    {#if plan.referenceIds.length>=12}<p class="field-help">Remove a reference before adding conditions (limit 12).</p>{/if}
  </div>{/if}
  {#if added.length}
    <ul class="scene-included">
      {#each added as {id,role} (`${role}:${id}`)}
        <li><label><input type="checkbox" checked onchange={()=>onToggle(id,role)} />{names.get(id)??'Missing node'}<small>{labels[role]}</small></label>
          {#if !updating&&context&&!context.error&&!context.resources.some(resource=>resource.nodeId===id)}<small>Outside the current prose context</small>{/if}
        </li>
      {/each}
    </ul>
  {/if}
  <details>
    <summary>Add participants or references</summary>
    <input aria-label="Find scene resources" placeholder="Find a world node…" bind:value={search} />
    <div class="scene-resource-list">
      {#each choices as node (node.id)}
        <div><span>{node.name}</span>
          {#if node.components['core.character']}<button class="text-action" disabled={plan.participantIds.length>=12} onclick={()=>onToggle(node.id,'participant')}>Add participant</button>{/if}
          <button class="text-action" disabled={plan.referenceIds.length>=12} onclick={()=>onToggle(node.id,'reference')}>Add reference</button>
        </div>
      {:else}<p class="field-help">No matching nodes to add.</p>{/each}
    </div>
  </details>
  {#if updating}
    <p class="field-help" role="status">Updating prose context…</p>
  {:else if context}
    {#if context.error}<p class="story-warning" role="status">{context.error}</p>{/if}
    {#if automatic.length||context.guidance.length}
      <details class="scene-automatic" bind:open={showAutomatic}>
        <summary>Included automatically for prose ({automatic.length+context.guidance.length})</summary>
        <ul class="scene-included">
          {#each automatic as resource (resource.nodeId)}<li>{resource.name}<small>{labels[resource.role]}</small></li>{/each}
          {#each context.guidance as section (section.nodeId)}
            <li><details><summary>{section.name}<small>{section.role==='instructions'?'Story instructions':'Story-wide reference'}</small></summary><p class="story-plan-text">{section.content}</p></details></li>
          {/each}
        </ul>
      </details>
    {/if}
  {/if}
</section>
