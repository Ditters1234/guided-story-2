import {describe,it,expect} from 'vitest';
import type {PassagePlan} from './bindings';
import {emptyWorld,makeNode} from './model';
import {scenePlan,sceneStartingPositions,nearbySceneCast,withNearbySceneCast,sceneConditionChoices} from './scene-plan';

const base=():PassagePlan=>({chapterTitle:'Chapter 1',chapterPlan:'',title:'Passage 1',direction:'',viewpointId:null,locationId:null,participantIds:[],referenceIds:[],targetWords:450});
function fixture(){
  const world=emptyWorld();
  world.nodeTypes.push({id:'custom.settler',name:'Settler',description:'',components:['core.character','core.holder']});
  const location=makeNode(world,'core.place','Homestead'),otherPlace=makeNode(world,'core.place','Distant village');
  const cast=[makeNode(world,'custom.settler','Nora'),makeNode(world,'core.character','Elias')],distant=makeNode(world,'core.character','Tomas');
  world.nodes=[location,otherPlace,...cast,distant];
  world.edges=[...cast.map(n=>({id:crypto.randomUUID(),typeId:'core.present-at',sourceId:n.id,targetId:location.id,description:''})),{id:crypto.randomUUID(),typeId:'core.present-at',sourceId:distant.id,targetId:otherPlace.id,description:''}];
  return {world,location,otherPlace,cast,distant};
}
describe('editable scene defaults',()=>{
  it('offers only direct conditions for explicit reference selection, without automatic inclusion or inheritance',()=>{
    const {world,location,otherPlace}=fixture(),room=makeNode(world,'core.place','Indoor room'),conditions=makeNode(world,'core.environment','Outdoor conditions'),other=makeNode(world,'core.environment','Distant conditions');
    conditions.typeId='custom.outdoors';
    world.nodes.push(room,conditions,other);
    world.edges.push(
      {id:'inside',typeId:'core.inside',sourceId:room.id,targetId:location.id,description:''},
      {id:'conditions',typeId:'story.conditions-at',sourceId:conditions.id,targetId:location.id,description:'Coverage fallback',properties:{scope:'Outdoor areas only.'}},
      {id:'distant',typeId:'story.conditions-at',sourceId:other.id,targetId:otherPlace.id,description:''},
      {id:'reverse',typeId:'story.conditions-at',sourceId:location.id,targetId:conditions.id,description:''},
    );
    const original=structuredClone(world),plan={...base(),locationId:location.id};
    expect(sceneConditionChoices(world,plan)).toEqual([{id:conditions.id,name:conditions.name,scope:'Outdoor areas only.'}]);
    expect(scenePlan(world,base(),location.id).referenceIds).toEqual([]);
    expect(sceneConditionChoices(world,{...plan,locationId:room.id})).toEqual([]);
    expect(sceneConditionChoices(world,{...plan,locationId:conditions.id})).toEqual([]);
    expect(sceneConditionChoices(world,{...plan,referenceIds:[conditions.id]})).toEqual([]);
    expect(sceneConditionChoices(world,{...plan,participantIds:[conditions.id]})).toEqual([]);
    expect(world).toEqual(original);
    expect(plan.referenceIds).toEqual([]);
    delete world.edges.find(e=>e.id==='conditions')!.properties;
    expect(sceneConditionChoices(world,plan)[0].scope).toBe('Coverage fallback');
  });
  it('stages only the selected place and its present cast, including custom character presets',()=>{
    const {world,location,cast,distant}=fixture(),before=structuredClone(world),plan=base();
    const scene=scenePlan(world,plan,location.id);
    expect(scene.locationId).toBe(location.id);expect(scene.viewpointId).toBe(cast[0].id);expect(scene.participantIds).toEqual([cast[1].id]);expect(scene.participantIds).not.toContain(distant.id);expect(scene.direction).toBe('');expect(scene.referenceIds).toEqual([]);expect(world).toEqual(before);expect(plan).toEqual(base());
  });
  it('follows a selected character location and preserves deliberate previous scene choices',()=>{
    const {world,location,cast,distant}=fixture();
    const scene=scenePlan(world,base(),cast[1].id);expect(scene.viewpointId).toBe(cast[1].id);expect(scene.locationId).toBe(location.id);expect(scene.participantIds).toEqual([cast[0].id]);
    const previous={...base(),viewpointId:cast[1].id,locationId:location.id,participantIds:[distant.id,'removed']};
    expect(scenePlan(world,base(),null,previous).participantIds).toEqual([distant.id]);
    expect(scenePlan(world,base(),location.id,{...base(),participantIds:[]}).viewpointId).toBeNull();
  });
  it('bounds default cast selection and does not pull in the remaining world',()=>{
    const {world,location}=fixture();
    for(let i=0;i<20;i++){const n=makeNode(world,'core.character',`Visitor ${i}`);world.nodes.push(n);world.edges.push({id:crypto.randomUUID(),typeId:'core.present-at',sourceId:n.id,targetId:location.id,description:''});}
    const scene=scenePlan(world,base(),location.id);expect(scene.participantIds).toHaveLength(12);expect(new Set(scene.participantIds).size).toBe(12);expect(scene.participantIds).not.toContain(scene.viewpointId);expect(scene.referenceIds).toEqual([]);
  });
});

function homestead(){
  const world=emptyWorld();
  const estate=makeNode(world,'core.place','Thorne Homestead'),house=makeNode(world,'core.place','Main House'),yard=makeNode(world,'core.place','Yard'),barn=makeNode(world,'core.place','Barn');
  const silas=makeNode(world,'core.character','Silas Thorne'),elara=makeNode(world,'core.character','Elara Vane');
  world.nodes=[estate,house,yard,barn,silas,elara];
  world.edges=[...([house,yard,barn].map(n=>({id:crypto.randomUUID(),typeId:'core.inside',sourceId:n.id,targetId:estate.id,description:''}))),
    {id:crypto.randomUUID(),typeId:'core.present-at',sourceId:silas.id,targetId:house.id,description:''},
    {id:crypto.randomUUID(),typeId:'core.present-at',sourceId:elara.id,targetId:yard.id,description:''}];
  return {world,estate,house,yard,barn,silas,elara};
}

describe('canonical starting positions',()=>{
  it('visibly stages nearby people for an empty barn without moving them from the house and yard',()=>{
    const {world,barn,house,yard,silas,elara}=homestead(),before=structuredClone(world);
    const emptyBarn=scenePlan(world,base(),barn.id);
    expect(emptyBarn).toMatchObject({locationId:barn.id,viewpointId:silas.id,participantIds:[elara.id]});
    const plan={...base(),locationId:barn.id,viewpointId:silas.id,participantIds:[elara.id,silas.id],referenceIds:[house.id]};
    expect(sceneStartingPositions(world,plan)).toEqual([
      {nodeId:silas.id,name:silas.name,placeId:house.id,placeName:house.name,status:'elsewhere'},
      {nodeId:elara.id,name:elara.name,placeId:yard.id,placeName:yard.name,status:'elsewhere'}
    ]);
    expect(world).toEqual(before);
    expect(plan.participantIds).toEqual([elara.id,silas.id]);
  });
  it('continues where the viewpoint is now placed and allows an explicit new focus',()=>{
    const {world,house,yard,barn,silas,elara}=homestead();
    const previous={...base(),locationId:barn.id,viewpointId:elara.id,participantIds:[silas.id]};
    const selected=scenePlan(world,base(),silas.id,previous);
    expect(selected.viewpointId).toBe(silas.id);
    expect(selected.locationId).toBe(house.id);
    expect(scenePlan(world,base(),null,{...previous,viewpointId:silas.id,participantIds:[elara.id]})).toMatchObject({locationId:house.id,viewpointId:silas.id,participantIds:[elara.id]});
    const continuation={...base(),locationId:house.id,viewpointId:silas.id,participantIds:[elara.id]};
    world.edges.find(e=>e.sourceId===silas.id)!.targetId=yard.id;
    const continued=scenePlan(world,base(),null,continuation);
    expect(continued).toMatchObject({locationId:yard.id,viewpointId:silas.id,participantIds:[elara.id]});
    expect(sceneStartingPositions(world,continued).find(p=>p.nodeId===silas.id)).toMatchObject({placeId:yard.id,placeName:yard.name,status:'at'});
    expect(scenePlan(world,base(),barn.id,continuation)).toMatchObject({locationId:barn.id,viewpointId:silas.id,participantIds:[elara.id]});
    expect(world.edges.find(e=>e.sourceId===silas.id)?.targetId).toBe(yard.id);
  });
  it('stages a selected item as an editable reference and carries valid prior references forward',()=>{
    const {world,house,barn}=homestead(),key=makeNode(world,'core.item','Key');world.nodes.push(key);
    world.edges.push({id:crypto.randomUUID(),typeId:'core.stored-in',sourceId:key.id,targetId:barn.id,description:''});
    expect(scenePlan(world,base(),key.id)).toMatchObject({locationId:barn.id,referenceIds:[key.id]});
    expect(scenePlan(world,base(),null,{...base(),locationId:house.id,referenceIds:['removed',key.id]})).toMatchObject({locationId:house.id,referenceIds:[key.id]});
  });
  it('uses custom physical links and nested holders while keeping sibling places distinct',()=>{
    const {world,estate,house,yard,barn,silas,elara}=homestead();
    const room=makeNode(world,'core.place','House study'),holder=makeNode(world,'core.item','Travel carrier');
    holder.components['core.container']={};
    world.nodes.push(room,holder);
    world.relationTypes.push({id:'custom.attached',name:'Physical attachment',label:'attached to',sourceCapability:null,targetCapability:null,physicalPlacement:true,singleTarget:true});
    world.edges=world.edges.filter(e=>e.sourceId!==silas.id);
    world.edges.push(
      {id:crypto.randomUUID(),typeId:'custom.attached',sourceId:silas.id,targetId:holder.id,description:''},
      {id:crypto.randomUUID(),typeId:'custom.attached',sourceId:holder.id,targetId:room.id,description:''},
      {id:crypto.randomUUID(),typeId:'custom.attached',sourceId:room.id,targetId:house.id,description:''},
      {id:crypto.randomUUID(),typeId:'story.feels-toward',sourceId:silas.id,targetId:barn.id,description:'Wants to work there.'}
    );
    expect(scenePlan(world,base(),silas.id).locationId).toBe(room.id);
    expect(scenePlan(world,base(),room.id).viewpointId).toBe(silas.id);
    const plan={...base(),locationId:house.id,viewpointId:silas.id,participantIds:[elara.id]};
    expect(sceneStartingPositions(world,plan).map(p=>[p.placeName,p.status])).toEqual([[room.name,'within'],[yard.name,'elsewhere']]);
    expect(sceneStartingPositions(world,{...plan,locationId:estate.id}).map(p=>p.status)).toEqual(['within','within']);
    expect(sceneStartingPositions(world,{...plan,locationId:barn.id}).map(p=>p.status)).toEqual(['elsewhere','elsewhere']);
    expect(sceneStartingPositions(world,{...plan,locationId:room.id})[0].status).toBe('at');
  });
  it('reports an unestablished location without following associations or looping on malformed drafts',()=>{
    const {world,barn,silas,elara}=homestead();
    world.edges=world.edges.filter(e=>e.sourceId!==silas.id&&e.sourceId!==elara.id);
    world.edges.push(
      {id:crypto.randomUUID(),typeId:'story.feels-toward',sourceId:silas.id,targetId:barn.id,description:'Thinks about the barn.'},
      {id:crypto.randomUUID(),typeId:'core.present-at',sourceId:elara.id,targetId:'unavailable-place',description:''}
    );
    const plan={...base(),locationId:barn.id,viewpointId:silas.id,participantIds:[elara.id,'deleted-actor']};
    expect(sceneStartingPositions(world,plan).map(p=>[p.placeId,p.status])).toEqual([[null,'unplaced'],[null,'unplaced']]);
    world.edges.push({id:crypto.randomUUID(),typeId:'core.present-at',sourceId:silas.id,targetId:silas.id,description:''});
    expect(sceneStartingPositions(world,plan)[0].status).toBe('unplaced');
  });
});

describe('bounded nearby cast suggestions',()=>{
  it('prefers direct occupants instead of adding people from adjacent places',()=>{
    const {world,barn,silas,elara}=homestead(),occupant=makeNode(world,'core.character','Barn keeper');
    world.nodes.push(occupant);world.edges.push({id:crypto.randomUUID(),typeId:'core.present-at',sourceId:occupant.id,targetId:barn.id,description:''});
    expect(nearbySceneCast(world,barn.id).map(n=>n.id)).toEqual([occupant.id]);
    expect(scenePlan(world,base(),barn.id)).toMatchObject({viewpointId:occupant.id,participantIds:[]});
    expect(nearbySceneCast(world,barn.id).map(n=>n.id)).not.toContain(silas.id);
    expect(nearbySceneCast(world,barn.id).map(n=>n.id)).not.toContain(elara.id);
  });
  it('keeps the search inside the immediate containing place and three physical hops',()=>{
    const {world,estate,house,barn,silas,elara}=homestead();
    const upper=makeNode(world,'core.place','Larger world'),remote=makeNode(world,'core.place','Unrelated town'),study=makeNode(world,'core.place','House study');
    const upperActor=makeNode(world,'core.character','World traveler'),remoteActor=makeNode(world,'core.character','Town resident'),deepActor=makeNode(world,'core.character','Study resident'),unplaced=makeNode(world,'core.character','Unplaced friend');
    world.nodes.push(upper,remote,study,upperActor,remoteActor,deepActor,unplaced);
    for(const [typeId,sourceId,targetId]of [
      ['core.inside',estate.id,upper.id],['core.inside',study.id,house.id],
      ['core.present-at',upperActor.id,upper.id],['core.present-at',remoteActor.id,remote.id],['core.present-at',deepActor.id,study.id],
      ['story.feels-toward',unplaced.id,barn.id]
    ])world.edges.push({id:crypto.randomUUID(),typeId,sourceId,targetId,description:''});
    expect(nearbySceneCast(world,barn.id).map(n=>n.id)).toEqual([silas.id,elara.id]);
  });
  it('supports custom physical definitions and subrooms without following social associations',()=>{
    const {world,house,barn,silas,elara}=homestead(),study=makeNode(world,'core.place','House study');
    world.nodes.push(study);world.edges.find(e=>e.sourceId===silas.id)!.targetId=study.id;
    world.edges.push({id:crypto.randomUUID(),typeId:'core.inside',sourceId:study.id,targetId:house.id,description:''});
    world.relationTypes.push({id:'custom.physical',name:'Attached',label:'attached',physicalPlacement:true,singleTarget:true,sourceCapability:null,targetCapability:null});
    world.edges=world.edges.map(e=>({...e,typeId:'custom.physical'}));
    const friend=makeNode(world,'core.character','Remote friend');world.nodes.push(friend);world.edges.push({id:crypto.randomUUID(),typeId:'story.feels-toward',sourceId:friend.id,targetId:barn.id,description:''});
    expect(nearbySceneCast(world,house.id).map(n=>n.id)).toEqual([silas.id,elara.id]);
    expect(sceneStartingPositions(world,{...base(),locationId:house.id,viewpointId:silas.id})[0]).toMatchObject({placeId:study.id,status:'within'});
  });
  it('applies suggested roles explicitly while preserving focus, references, direction and world facts',()=>{
    const {world,house,barn,silas,elara}=homestead(),before=structuredClone(world);
    const empty={...base(),locationId:barn.id,direction:'Begin quietly.',referenceIds:[house.id],chapterPlan:'Preserve the premise.'},original=structuredClone(empty);
    expect(scenePlan(world,base(),barn.id,empty)).toMatchObject({viewpointId:null,participantIds:[]});
    const selected=withNearbySceneCast(world,empty);
    expect(selected).toEqual({...empty,viewpointId:silas.id,participantIds:[elara.id]});
    expect(empty).toEqual(original);expect(world).toEqual(before);
    expect(sceneStartingPositions(world,selected).map(p=>p.status)).toEqual(['elsewhere','elsewhere']);
  });
  it('bounds the nearby fallback to thirteen people and twelve participants',()=>{
    const {world,house,barn}=homestead();
    for(let i=0;i<20;i++){const n=makeNode(world,'core.character',`Nearby visitor ${i}`);world.nodes.push(n);world.edges.push({id:crypto.randomUUID(),typeId:'core.present-at',sourceId:n.id,targetId:house.id,description:''});}
    const people=nearbySceneCast(world,barn.id),plan=scenePlan(world,base(),barn.id);
    expect(people).toHaveLength(13);expect(plan.participantIds).toHaveLength(12);expect(new Set([plan.viewpointId,...plan.participantIds]).size).toBe(13);
  });
  it('does not substitute nearby roles for an explicitly selected or deliberately empty cast',()=>{
    const {world,estate,barn,silas}=homestead();
    world.edges=world.edges.filter(e=>e.sourceId!==silas.id);
    expect(scenePlan(world,base(),silas.id)).toMatchObject({locationId:estate.id,viewpointId:silas.id,participantIds:[]});
    expect(scenePlan(world,base(),barn.id,{...base(),locationId:barn.id})).toMatchObject({locationId:barn.id,viewpointId:null,participantIds:[]});
    expect(nearbySceneCast(world,null)).toEqual([]);expect(nearbySceneCast(world,'missing')).toEqual([]);
    expect(withNearbySceneCast(world,{...base(),locationId:'missing'})).toEqual({...base(),locationId:'missing'});
  });
});
