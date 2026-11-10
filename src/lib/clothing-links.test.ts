import {describe,it,expect} from 'vitest';
import {emptyWorld,makeNode} from './model';
import {garmentPlacementTargets,placeCandidateGarment,unplacedCandidateGarments} from './authoring-ui';
describe('proposal clothing placement',()=>{
  it('links an existing proposed garment to a selected wearer without adding another garment or modifying the source',()=>{
    const world=emptyWorld(),person=makeNode(world,'core.character','Rowan','new:rowan'),shirt=makeNode(world,'core.wearable-item','Work shirt','new:shirt'),boots=makeNode(world,'core.wearable-item','Boots','new:boots');
    const candidate={nodes:[person,shirt,boots],edges:[]},before=structuredClone(candidate);
    const linked=placeCandidateGarment(world,candidate,shirt.id,'core.worn-by',person.id,candidate.nodes);
    expect(linked.nodes).toEqual(candidate.nodes);expect(linked.edges).toHaveLength(1);expect(linked.edges[0]).toMatchObject({typeId:'core.worn-by',sourceId:shirt.id,targetId:person.id});
    expect(unplacedCandidateGarments(world,linked).map(n=>n.id)).toEqual([boots.id]);expect(candidate).toEqual(before);
    expect(()=>placeCandidateGarment(world,linked,shirt.id,'core.stored-in',person.id,candidate.nodes)).toThrow();
  });
  it('offers only supplied compatible targets, including indexed people and custom containers',()=>{
    const world=emptyWorld(),shirt=makeNode(world,'core.wearable-item','Shirt','new:shirt');
    const refs=[{id:'indexed-person',name:'Rowan',capabilities:['core.character','core.holder']},{id:'box',name:'Clothing chest',capabilities:['core.item','core.container']},{id:'note',name:'Unrelated note',capabilities:['detail.note']}];
    const choices=garmentPlacementTargets(world,shirt,refs);
    expect(choices.filter(c=>c.typeId==='core.worn-by').map(c=>c.targetId)).toEqual(['indexed-person']);
    expect(choices.filter(c=>c.typeId==='core.stored-in').map(c=>c.targetId)).toEqual(['indexed-person','box']);
    const candidate={nodes:[shirt],edges:[]};
    expect(placeCandidateGarment(world,candidate,shirt.id,'core.stored-in','box',refs).edges[0].targetId).toBe('box');
    expect(()=>placeCandidateGarment(world,candidate,shirt.id,'core.worn-by','hidden-person',refs)).toThrow();
    expect(()=>placeCandidateGarment(world,candidate,shirt.id,'core.worn-by','box',refs)).toThrow();
    const established={...shirt,id:'existing-garment'};expect(()=>placeCandidateGarment(world,{nodes:[established],edges:[]},established.id,'core.worn-by','indexed-person',refs)).toThrow();
  });
});
