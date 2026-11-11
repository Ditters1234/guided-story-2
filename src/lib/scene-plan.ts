import type {PassagePlan} from './bindings';
import type {World} from './model';

/** Local suggestions only. Nothing enters model context until explicitly added. */
export function sceneReferenceSuggestions(world:World,plan:PassagePlan,scopeIds:readonly string[]=[],prose='') {
  const nodes=new Map(world.nodes.map(n=>[n.id,n]));
  const selected=new Set([plan.locationId,plan.viewpointId,...plan.participantIds,...plan.referenceIds,...scopeIds].filter((id):id is string=>!!id));
  const actors=new Set([...selected].filter(id=>nodes.get(id)?.components['core.character']));
  for(const edge of world.edges)if((edge.typeId==='core.worn-by'||edge.typeId==='core.stored-in')&&actors.has(edge.targetId))selected.add(edge.sourceId);
  const physical=new Set(world.relationTypes.filter(r=>r.physicalPlacement).map(r=>r.id));
  const children=new Map<string,string[]>();
  for(const edge of world.edges)if(physical.has(edge.typeId)&&nodes.get(edge.sourceId)?.components['core.item']){
    const list=children.get(edge.targetId)??[];list.push(edge.sourceId);children.set(edge.targetId,list);
  }
  const candidates=new Map<string,{id:string;name:string;reason:string;rank:number}>();
  const offer=(id:string,reason:string,rank:number)=>{const node=nodes.get(id);if(node&&!selected.has(id)&&!candidates.has(id))candidates.set(id,{id,name:node.name,reason,rank});};
  let frontier=[...selected].filter(id=>id===plan.locationId||nodes.get(id)?.components['core.container']);
  const seen=new Set(frontier);
  for(let depth=0;depth<2;depth++){
    const next:string[]=[];
    for(const holder of frontier)for(const id of children.get(holder)??[]){
      if(seen.has(id))continue;seen.add(id);
      offer(id,`In ${nodes.get(holder)?.name??'selected container'}`,holder===plan.locationId?1:0);
      if(nodes.get(id)?.components['core.container'])next.push(id);
    }
    frontier=next;
  }
  for(const edge of world.edges){
    if(physical.has(edge.typeId))continue;
    for(const [from,to]of [[edge.sourceId,edge.targetId],[edge.targetId,edge.sourceId]]){
      const node=nodes.get(to);
      if(selected.has(from)&&(node?.components['core.thread']||node?.components['detail.thread']))offer(to,`Thread linked to ${nodes.get(from)?.name??'scene resource'}`,2);
    }
  }
  const text=prose.toLocaleLowerCase();
  const all=[...candidates.values()].sort((a,b)=>Number(text.includes(b.name.toLocaleLowerCase()))-Number(text.includes(a.name.toLocaleLowerCase()))||a.rank-b.rank||a.name.localeCompare(b.name)||a.id.localeCompare(b.id));
  return {suggestions:all.slice(0,12),omitted:Math.max(0,all.length-12)};
}

/** Only direct, explicitly authored coverage of the selected focus. No inheritance. */
export function sceneConditionChoices(world:World,plan:PassagePlan):{id:string;name:string;scope:string}[] {
  if(!world.nodes.some(n=>n.id===plan.locationId&&n.components['core.place'])||!world.relationTypes.some(r=>r.id==='story.conditions-at'&&!r.physicalPlacement))return [];
  const selected=new Set([plan.locationId,plan.viewpointId,...plan.participantIds,...plan.referenceIds]);
  return world.edges.filter(e=>e.typeId==='story.conditions-at'&&e.targetId===plan.locationId&&!selected.has(e.sourceId)).flatMap(edge=>{
    const node=world.nodes.find(n=>n.id===edge.sourceId&&n.components['detail.environment']);
    return node?[{id:node.id,name:node.name,scope:typeof edge.properties?.scope==='string'?edge.properties.scope:edge.description}]:[];
  }).filter((choice,index,all)=>all.findIndex(c=>c.id===choice.id)===index);
}

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
  const selectedPlace=selected?.components['core.place']?selected:physicalPlaces(selected?.id)[0];
  plan.locationId=(selectedPlace??physicalPlaces(plan.viewpointId)[0]??place(previous?.locationId)??(!previous?world.nodes.find(n=>n.components['core.place']):undefined))?.id??null;
  const directCast=world.nodes.filter(n=>n.components['core.character']&&physicalPlaces(n.id)[0]?.id===plan.locationId);
  const cast=!previous&&!selectedCharacter?nearbySceneCast(world,plan.locationId):directCast;
  if(!previous&&!plan.viewpointId)plan.viewpointId=cast[0]?.id??null;
  plan.participantIds=(previous?previous.participantIds.filter(id=>character(id)):cast.map(n=>n.id)).filter(id=>id!==plan.viewpointId).slice(0,12);
  const references=new Set(previous?.referenceIds.filter(id=>world.nodes.some(n=>n.id===id)));
  if(selected&&!selectedCharacter&&!selected.components['core.place']&&!selected.components['core.claim'])references.add(selected.id);
  plan.referenceIds=[...references].filter(id=>id!==plan.locationId&&id!==plan.viewpointId&&!plan.participantIds.includes(id)).slice(0,12);
  return plan;
}
