import {describe,it,expect} from 'vitest';
import type {PassagePlan} from './bindings';
import {emptyWorld,makeNode} from './model';
import {scenePlan} from './scene-plan';

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
