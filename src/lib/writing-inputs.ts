import type {FieldDefinition_Serialize,PassagePlan,SceneContextPreview} from './bindings';
import type {World} from './model';
import writingFields from '../../schema/writing-input-fields.json';
import writingRelations from '../../schema/writing-input-relations.json';
export const narrationRoles=writingRelations;

export type DraftWorldContext={id:string;title:string;plan:PassagePlan;context?:SceneContextPreview;disabled:boolean;updating?:boolean};
const fields=writingFields as Record<string,FieldDefinition_Serialize[]>;
export function needsWritingFields(world:World,id:string){
  const node=world.nodes.find(n=>n.id===id);
  return !!node?.components['core.narration-style']&&writingRelations.some(r=>!world.relationTypes.some(d=>d.id===r.id))||Object.entries(fields).some(([capability,definitions])=>node?.components[capability]&&definitions.some(field=>!world.componentTypes.find(d=>d.id===capability)?.fields.some(f=>f.key===field.key)));
}

function withWritingDefinitions(world:World,capabilities:readonly string[]):World{
  const componentTypes=world.componentTypes.map(definition=>{
    if(!capabilities.includes(definition.id))return definition;
    const additions=(fields[definition.id]??[]).filter(f=>!definition.fields.some(existing=>existing.key===f.key));
    return additions.length?{...definition,fields:[...definition.fields,...structuredClone(additions)]}:definition;
  });
  return componentTypes.some((definition,index)=>definition!==world.componentTypes[index])?{...world,componentTypes}:world;
}

export function withBriefFields(world:World):World{
  return withWritingDefinitions(world,['detail.brief']);
}

// Explicitly adds optional definitions to this world's current schema only.
export function withWritingFields(world:World,id:string):World{
  const node=world.nodes.find(n=>n.id===id);if(!node)return world;
  const next=withWritingDefinitions(world,Object.keys(node.components));
  if(!node.components['core.narration-style'])return next;
  const additions=writingRelations.filter(r=>!world.relationTypes.some(d=>d.id===r.id));
  return additions.length?{...next,relationTypes:[...world.relationTypes,...structuredClone(additions)]}:next;
}
export function briefDraftFields(world: World, id: string | null | undefined): Set<string> {
  const values = world.nodes.find(node => node.id === id)?.components['detail.brief'];
  return new Set(fields['detail.brief'].filter(field => {
    const value = values?.[field.key];
    return typeof value !== 'string' || !value.trim();
  }).map(field => field.key));
}

export function selectWritingSource(plan:PassagePlan,kind:'narration'|'brief',id:string|null):PassagePlan{
  if(kind==='narration')return {...plan,narrationNodeId:id,viewpointId:null,knowledgePerspective:undefined};
  const {savedDraftDirection, ...draft} = plan;
  return {...draft,...savedDraftDirection,briefNodeId:id,consumedBriefPassage:undefined};
}
export type NodeContextChoice={checked:boolean;included:boolean;locked:boolean;disabled:boolean;label:string;reason:string;warning?:boolean};
export function passageNodeChoice(draft:DraftWorldContext|null,id:string,world?:World):NodeContextChoice{
  const binding=world?.aiContext?.find(b=>b.nodeId===id&&b.enabled&&b.tasks.includes('passage'));
  if(!draft)return {checked:!!binding,included:false,locked:!!binding,disabled:true,label:binding?'Story-wide':'No passage draft',reason:binding?'Required passage binding. Open a draft for its exact context; edit this binding in Story-wide AI context.':'Open a passage draft to choose and preview context.'};
  const plan=draft.plan,resource=draft.context?.resources.find(r=>r.nodeId===id),guidance=draft.context?.guidance.find(g=>g.nodeId===id);
  const source=id===plan.narrationNodeId?'Narration source':id===plan.briefNodeId?'Brief source':'';
  const role=id===plan.viewpointId?'POV':id===plan.locationId?'Scene focus':plan.participantIds.includes(id)?'Participant':'';
  const explicit=plan.referenceIds.includes(id),included=!!resource||!!guidance||!!source;
  const automatic=resource?({inventory:'Cast inventory',containingPlace:'Placement location',placement:'Placement dependency',claim:'Claim context',focus:'Scene focus',viewpoint:'POV',participant:'Participant',reference:'Reference'})[resource.role]:'';
  // An explicit reference stays removable even when another dependency also includes it.
  const locked=!!source||!!role||(!explicit&&(included||!!binding));
  const checked=explicit||!!source||!!role||included||!!binding;
  const reason=source||role||(explicit?'Selected reference':guidance?guidance.role==='instructions'?'Story-wide instructions':'Story-wide reference':binding?'Story-wide binding':automatic)||'Not included';
  const limit=!checked&&plan.referenceIds.length>=12;
  const updating=draft.updating||!draft.context;
  const label=updating?'Updating…':checked&&!included?'Not sent':locked?source||role||(guidance?'Story-wide':automatic):included?'In context':'Not included';
  return {checked,included,locked,disabled:draft.disabled||!!updating||locked||limit,label,warning:!updating&&checked&&!included,
    reason:updating?'Updating passage context; wait for the saved preview.':limit?'Remove a reference before adding another (limit 12).':checked&&!included?`${reason} is outside the native prose scope and will not be sent. ${locked?binding?'Review the passage context error or edit its Story-wide AI context binding.':'Review the passage context error or change its narration link / scene role.':'Uncheck to remove its selection.'}`:locked?`${reason} is required. ${source?'Choose a different writing source in Passage setup.':guidance?'Edit its Story-wide AI context binding to change inclusion.':'Change its narration link or scene role to remove this dependency.'}${binding&&(source||role)?' This node also has a Story-wide AI context binding; edit that binding to stop its persistent inclusion.':''}`:explicit?`Selected reference${guidance||automatic&&automatic!=='Reference'?'; also included automatically':''}. Uncheck to remove the explicit reference.`:'Check to include this node as a passage reference.'};
}
