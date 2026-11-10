import {describe,it,expect} from 'vitest';
import {exampleWorld,makeNode} from './model';
import {newSelectionAuthoringRequest,allowRevisionAdditions,editableFields} from './authoring-ui';

describe('AI revision from a map selection',()=>{
  it('captures the entire selection and internal link details while protecting names',()=>{
    const world=exampleWorld(),before=structuredClone(world),person=world.nodes.find(n=>n.name==='Mara Vale')!,coat=world.nodes.find(n=>n.name==='Waxed field coat')!;
    const request=newSelectionAuthoringRequest(world,[person.id,coat.id]);
    expect(request.mode).toBe('revise');expect(request.targetIds).toEqual([person.id,coat.id]);expect(request.bundle).toBe(false);
    expect(request.linkTargetIds).toEqual(world.edges.filter(e=>e.sourceId===coat.id&&e.targetId===person.id).map(e=>e.id));
    expect(request.fields.some(field=>field.nodeId===coat.id&&field.componentId!==null)).toBe(true);
    expect(request.fields.some(field=>field.key==='name')).toBe(false);expect(world).toEqual(before);
  });
  it('seeds only bounded immediate physical neighbors as read-only references',()=>{
    const world=exampleWorld(),bag=world.nodes.find(n=>n.name==='Canvas satchel')!,person=world.nodes.find(n=>n.name==='Mara Vale')!,seeds=world.nodes.find(n=>n.name==='Seed packets')!;
    const request=newSelectionAuthoringRequest(world,[bag.id]);
    expect(request.targetIds).toEqual([bag.id]);expect(new Set(request.contextIds)).toEqual(new Set([person.id,seeds.id]));
    expect(request.contextIds).not.toContain(world.nodes[0].id);
  });
  it('captures explicit addition types and clears them when additions are disabled',()=>{
    const world=exampleWorld(),request=newSelectionAuthoringRequest(world,[world.nodes[0].id]);
    const enabled=allowRevisionAdditions(world,request,true);expect(enabled.bundle).toBe(true);
    expect(enabled.creationTypeIds).toEqual(world.nodeTypes.filter(type=>type.components.includes('core.item')).map(type=>type.id));
    expect(enabled.targetIds).toEqual(request.targetIds);expect(enabled.contextIds).toEqual(request.contextIds);
    expect(allowRevisionAdditions(world,enabled,false).creationTypeIds).toBeUndefined();
  });
  it('rejects oversized or stale selections and keeps unchecked fields explicitly protected',()=>{
    const world=exampleWorld();expect(()=>newSelectionAuthoringRequest(world,[])).toThrow();expect(()=>newSelectionAuthoringRequest(world,['missing'])).toThrow();
    world.nodes=Array.from({length:13},(_,i)=>makeNode(world,'core.character',`Person ${i}`));world.edges=[];
    expect(()=>newSelectionAuthoringRequest(world,world.nodes.map(n=>n.id))).toThrow(/12/);
    const ids=world.nodes.slice(0,12).map(n=>n.id),request=newSelectionAuthoringRequest(world,ids);
    expect(request.targetIds).toHaveLength(12);expect(request.fields).toHaveLength(200);expect(editableFields(world,ids).length).toBeGreaterThan(200);
  });
});
