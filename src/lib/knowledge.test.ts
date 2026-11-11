import {describe,it,expect} from 'vitest';
import {emptyWorld,makeNode,withBuiltinNodeType,availableNodeTypes} from './model';
import {scenePlan} from './scene-plan';
import type {PassagePlan} from './bindings';
describe('claim toolkit',()=>{
  it('installs the optional claim contract and relationship only on explicit creation',()=>{
    const world=emptyWorld();world.nodeTypes=world.nodeTypes.filter(d=>d.id!=='core.claim');world.componentTypes=world.componentTypes.filter(d=>d.id!=='core.claim');world.relationTypes=world.relationTypes.filter(d=>d.id!=='core.holds-claim');
    const before=structuredClone(world);expect(availableNodeTypes(world).some(d=>d.id==='core.claim')).toBe(true);expect(world).toEqual(before);
    const next=withBuiltinNodeType(world,'core.claim'),claim=makeNode(next,'core.claim','A rumour');
    expect(claim.components['core.claim']).toEqual({status:'unconfirmed',disclosure:'withheld',subjects:[]});
    expect(next.relationTypes.find(d=>d.id==='core.holds-claim')).toMatchObject({sourceCapability:'core.claim',targetCapability:'core.character',physicalPlacement:false,singleTarget:false});
    expect(withBuiltinNodeType(next,'core.claim')).toEqual(next);expect(next.nodes).toEqual([]);expect(next.aiContext).toBeUndefined();expect(world).toEqual(before);
  });
  it('does not mistake a claim for a character or place when staging a scene',()=>{
    const world=emptyWorld(),claim=makeNode(world,'core.claim','Hidden valve'),person=makeNode(world,'core.character','Mara'),place=makeNode(world,'core.place','Workshop');world.nodes=[claim,person,place];
    const base:PassagePlan={chapterTitle:'Chapter 1',chapterPlan:'',title:'Passage 1',direction:'',viewpointId:null,locationId:null,participantIds:[],referenceIds:[],targetWords:450,knowledgePerspective:'author'};
    const plan=scenePlan(world,base,claim.id);expect(plan.viewpointId).toBeNull();expect(plan.locationId).toBe(place.id);expect(plan.knowledgePerspective).toBe('author');expect(plan.referenceIds).toEqual([]);
  });
});
