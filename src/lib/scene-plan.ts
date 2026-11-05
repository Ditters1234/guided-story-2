import type {PassagePlan} from './bindings';
import type {World} from './model';

/** Closest place first, using only declared physical attachments. */
function placementResolver(world:World) {
  const physical=new Set(world.relationTypes.filter(r=>r.physicalPlacement).map(r=>r.id));
  const nodes=new Map(world.nodes.map(n=>[n.id,n]));
  const parents=new Map(world.edges.filter(e=>physical.has(e.typeId)).map(e=>[e.sourceId,e.targetId]));
  return (id:string|null|undefined)=>{
    const places:World['nodes']=[],seen=new Set<string>();
    let current=id;
    while(current&&!seen.has(current)){
      seen.add(current);
      const target=parents.get(current),node=target?nodes.get(target):undefined;
      if(!node||seen.has(node.id))break;
      if(node.components['core.place'])places.push(node);
      current=node.id;
    }
    return places;
  };
}

export type SceneStartingPosition={nodeId:string;name:string;placeId:string|null;placeName:string|null;status:'at'|'within'|'elsewhere'|'unplaced'};

/** Selection describes intended use; these positions come only from the world. */
export function sceneStartingPositions(world:World,plan:PassagePlan):SceneStartingPosition[] {
  const places=placementResolver(world),selected=new Set([plan.viewpointId,...plan.participantIds]);
  return world.nodes.filter(n=>selected.has(n.id)&&n.components['core.character']).map(node=>{
    const trail=places(node.id),place=trail[0];
    const status=!place?'unplaced':!plan.locationId||place.id===plan.locationId?'at':trail.some(p=>p.id===plan.locationId)?'within':'elsewhere';
    return {nodeId:node.id,name:node.name,placeId:place?.id??null,placeName:place?.name??null,status};
  });
}

/** A bounded cast suggestion, not a claim that these people share the focus. */
export function nearbySceneCast(world:World,focusId:string|null|undefined):World['nodes'] {
  const focus=world.nodes.find(n=>n.id===focusId&&n.components['core.place']);
  if(!focus)return [];
  const places=placementResolver(world),characters=world.nodes.filter(n=>n.components['core.character']);
  const direct=characters.filter(n=>places(n.id)[0]?.id===focus.id).slice(0,13);
  if(direct.length)return direct;
  const boundary=places(focus.id)[0]?.id??focus.id;
  const nodes=new Map(world.nodes.map(n=>[n.id,n]));
  const physical=new Set(world.relationTypes.filter(r=>r.physicalPlacement).map(r=>r.id));
  const neighbors=new Map<string,string[]>();
  for(const edge of world.edges){
    if(!physical.has(edge.typeId)||!nodes.has(edge.sourceId)||!nodes.has(edge.targetId))continue;
    for(const [from,to]of [[edge.sourceId,edge.targetId],[edge.targetId,edge.sourceId]]){
      const adjacent=neighbors.get(from)??[];adjacent.push(to);neighbors.set(from,adjacent);
    }
  }
  const within=(id:string)=>id===boundary||places(id).some(p=>p.id===boundary);
  const distance=new Map([[focus.id,0]]),queue=[focus.id];
  for(let i=0;i<queue.length;i++){
    const current=queue[i],depth=distance.get(current)!;
    if(depth>=3)continue;
    for(const next of neighbors.get(current)??[]){
      if(distance.has(next)||!within(next))continue;
      distance.set(next,depth+1);queue.push(next);
    }
  }
  return characters.filter(n=>distance.has(n.id)&&places(n.id).length).sort((a,b)=>distance.get(a.id)!-distance.get(b.id)!).slice(0,13);
}

/** Explicitly selects suggested identities without changing scene intent or facts. */
export function withNearbySceneCast(world:World,plan:PassagePlan):PassagePlan {
  const next=structuredClone(plan),cast=nearbySceneCast(world,plan.locationId);
  if(cast.length){next.viewpointId=cast[0].id;next.participantIds=cast.slice(1).map(n=>n.id);}
  return next;
}

/** Visible, editable defaults for the current scene; never expands the whole map. */
export function scenePlan(world:World,base:PassagePlan,selectedId:string|null,previous?:PassagePlan):PassagePlan {
  const plan=structuredClone(base);
  const selected=world.nodes.find(n=>n.id===selectedId);
  const character=(id:string|null|undefined)=>world.nodes.find(n=>n.id===id&&n.components['core.character']);
  const place=(id:string|null|undefined)=>world.nodes.find(n=>n.id===id&&n.components['core.place']);
  const physicalPlaces=placementResolver(world);
  const selectedCharacter=selected?.components['core.character']?selected:undefined;
  plan.viewpointId=(selectedCharacter??character(previous?.viewpointId))?.id??null;
  plan.locationId=(selected?.components['core.place']?selected:(selectedCharacter?physicalPlaces(selectedCharacter.id)[0]:undefined)??place(previous?.locationId)??physicalPlaces(plan.viewpointId)[0]??world.nodes.find(n=>n.components['core.place']))?.id??null;
  const directCast=world.nodes.filter(n=>n.components['core.character']&&physicalPlaces(n.id)[0]?.id===plan.locationId);
  const cast=!previous&&!selectedCharacter?nearbySceneCast(world,plan.locationId):directCast;
  if(!previous&&!plan.viewpointId)plan.viewpointId=cast[0]?.id??null;
  plan.participantIds=(previous?previous.participantIds.filter(id=>character(id)):cast.map(n=>n.id)).filter(id=>id!==plan.viewpointId).slice(0,12);
  return plan;
}
