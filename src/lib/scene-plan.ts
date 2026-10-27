import type {PassagePlan} from './bindings';
import type {World} from './model';

/** Visible, editable defaults for the current scene; never expands the whole map. */
export function scenePlan(world:World,base:PassagePlan,selectedId:string|null,previous?:PassagePlan):PassagePlan {
  const plan=structuredClone(base);
  const selected=world.nodes.find(n=>n.id===selectedId);
  const character=(id:string|null|undefined)=>world.nodes.find(n=>n.id===id&&n.components['core.character']);
  const place=(id:string|null|undefined)=>world.nodes.find(n=>n.id===id&&n.components['core.place']);
  plan.viewpointId=(selected?.components['core.character']?selected:character(previous?.viewpointId))?.id??null;
  plan.locationId=(selected?.components['core.place']?selected:place(previous?.locationId)??place(world.edges.find(e=>e.typeId==='core.present-at'&&e.sourceId===plan.viewpointId)?.targetId)??world.nodes.find(n=>n.components['core.place']))?.id??null;
  const present=new Set(world.edges.filter(e=>e.typeId==='core.present-at'&&e.targetId===plan.locationId).map(e=>e.sourceId));
  const cast=world.nodes.filter(n=>n.components['core.character']&&present.has(n.id));
  if(!previous&&!plan.viewpointId)plan.viewpointId=cast[0]?.id??null;
  plan.participantIds=(previous?previous.participantIds.filter(id=>character(id)):cast.map(n=>n.id)).filter(id=>id!==plan.viewpointId).slice(0,12);
  return plan;
}
