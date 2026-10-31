import {describe,it,expect} from 'vitest';
import type {PassagePlan} from './bindings';
import {emptyWorld,makeNode} from './model';
import {scenePlan,sceneStartingPositions} from './scene-plan';

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
  it('does not pull people from the house and yard into an empty selected barn',()=>{
    const {world,barn,house,yard,silas,elara}=homestead(),before=structuredClone(world);
    const emptyBarn=scenePlan(world,base(),barn.id);
    expect(emptyBarn).toMatchObject({locationId:barn.id,viewpointId:null,participantIds:[]});
    const plan={...base(),locationId:barn.id,viewpointId:silas.id,participantIds:[elara.id,silas.id],referenceIds:[house.id]};
    expect(sceneStartingPositions(world,plan)).toEqual([
      {nodeId:silas.id,name:silas.name,placeId:house.id,placeName:house.name,status:'elsewhere'},
      {nodeId:elara.id,name:elara.name,placeId:yard.id,placeName:yard.name,status:'elsewhere'}
    ]);
    expect(world).toEqual(before);
    expect(plan.participantIds).toEqual([elara.id,silas.id]);
  });
  it('follows an explicitly selected character while retaining prior scene focus as intent',()=>{
    const {world,house,yard,barn,silas,elara}=homestead();
    const previous={...base(),locationId:barn.id,viewpointId:elara.id,participantIds:[silas.id]};
    const selected=scenePlan(world,base(),silas.id,previous);
    expect(selected.viewpointId).toBe(silas.id);
    expect(selected.locationId).toBe(house.id);
    expect(scenePlan(world,base(),null,{...previous,viewpointId:silas.id,participantIds:[elara.id]})).toMatchObject({locationId:barn.id,viewpointId:silas.id,participantIds:[elara.id]});
    const continuation={...base(),locationId:house.id,viewpointId:silas.id,participantIds:[elara.id]};
    world.edges.find(e=>e.sourceId===silas.id)!.targetId=yard.id;
    const continued=scenePlan(world,base(),null,continuation);
    expect(continued).toMatchObject({locationId:house.id,viewpointId:silas.id,participantIds:[elara.id]});
    expect(sceneStartingPositions(world,continued).find(p=>p.nodeId===silas.id)).toMatchObject({placeId:yard.id,placeName:yard.name,status:'elsewhere'});
    expect(scenePlan(world,base(),barn.id,continuation)).toMatchObject({locationId:barn.id,viewpointId:silas.id,participantIds:[elara.id]});
    expect(world.edges.find(e=>e.sourceId===silas.id)?.targetId).toBe(yard.id);
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
