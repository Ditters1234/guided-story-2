import { describe,it,expect } from 'vitest';
import { exampleWorld,emptyWorld,makeNode } from './model';
import { addCandidateNode,editableFields,newAuthoringRequest,parseCandidateForEditor,changeCreationType,addCandidateWornItem,addCandidateCapability,creationChoice,candidateNodeTypes } from './authoring-ui';
describe('shared AI node editor',()=>{
  it('starts a new group with the current node as a full-detail reference without changing revision scope',()=>{
    const world=exampleWorld(),focus=world.nodes[3].id,before=structuredClone(world);
    expect(newAuthoringRequest(world,undefined,focus)).toMatchObject({creationScope:'group',contextIds:[focus]});
    expect(newAuthoringRequest(world,undefined,'missing').contextIds).toEqual([]);
    expect(newAuthoringRequest(world,focus,world.nodes[0].id)).toMatchObject({mode:'revise',targetIds:[focus],contextIds:[]});expect(world).toEqual(before);
  });
  it('offers empty descriptions and missing optional custom text fields without requiring placeholder content',()=>{
    const world=emptyWorld();world.componentTypes.push({id:'custom.voice',name:'Voice',description:'',fields:[{key:'voice',label:'Voice',kind:'text',required:false,defaultValue:''}]});
    const node=makeNode(world,'core.note','Narration style');node.components['custom.voice']={};world.nodes.push(node);
    const request=newAuthoringRequest(world,node.id);expect(request.direction).toBe('');expect(request.fields).toEqual(expect.arrayContaining([{nodeId:node.id,componentId:null,key:'description'},{nodeId:node.id,componentId:null,key:'visualDescription'},{nodeId:node.id,componentId:'custom.voice',key:'voice'}]));expect(request.fields.some(f=>f.componentId===null&&f.key==='name')).toBe(false);expect(node.description).toBe('');expect(node.components['custom.voice']).toEqual({});
  });
  it('defaults empty stories to world building and populated stories to groups, with distinct typed-node controls',()=>{
    const world=emptyWorld(),request=newAuthoringRequest(world);expect(request.typeId).toBeNull();expect(request.bundle).toBe(true);expect(creationChoice(request)).toBe('@world');
    const populated=exampleWorld(),group=newAuthoringRequest(populated);expect(group.bundle).toBe(true);expect(group.typeId).toBeNull();expect(creationChoice(group)).toBe('@group');
    world.nodeTypes.push({id:'custom.person',name:'Settler',description:'',components:['core.character','core.holder']});
    for(const type of ['core.character','custom.person'])expect(changeCreationType(world,group,type)).toMatchObject({bundle:true,creationScope:null,groupTypeId:null});
    expect(changeCreationType(world,group,'core.item')).toMatchObject({bundle:false,creationScope:null,typeId:'core.item'});
    expect(changeCreationType(world,group,null).creationScope).toBe('world');expect(request.direction).toBe('');
  });
  it('preserves direction and explicit references while resetting scope-specific choices on mode changes',()=>{
    const world=exampleWorld(),group={...newAuthoringRequest(world),groupTypeId:'core.item',direction:'A travel kit.',contextIds:[world.nodes[0].id]},before=structuredClone(group);
    const building=changeCreationType(world,group,'@world');expect(building).toMatchObject({creationScope:'world',groupTypeId:null,typeId:null,bundle:true,direction:group.direction,contextIds:group.contextIds});
    const typed=changeCreationType(world,building,'core.place');expect(typed).toMatchObject({creationScope:null,groupTypeId:null,bundle:false});
    expect(changeCreationType(world,typed,'@group')).toMatchObject({creationScope:'group',groupTypeId:null,typeId:null,bundle:true});expect(group).toEqual(before);
    const legacy={...group,creationScope:undefined,groupTypeId:undefined};expect(creationChoice(legacy)).toBe('@world');expect(legacy.creationScope).toBeUndefined();
  });
  it('limits candidate additions by any saved group preset without hiding existing references',()=>{
    const world=exampleWorld();world.nodeTypes.push({id:'custom.supply',name:'Supply',description:'',components:['core.item']});
    const request={...newAuthoringRequest(world),groupTypeId:'custom.supply'};expect(candidateNodeTypes(world,request).map(t=>t.id)).toEqual(['custom.supply']);
    expect(candidateNodeTypes(world,{...request,groupTypeId:null})).toEqual(world.nodeTypes);expect(world.nodes).toHaveLength(8);
  });
  it('adds a separate wearable item with one wear placement without changing the world',()=>{
    const world=emptyWorld(),before=structuredClone(world),person=makeNode(world,'core.character','Nora Hale','new:person');const input={nodes:[person],edges:[]};
    const next=addCandidateWornItem(world,input,person.id),item=next.nodes[1];expect(item.components['core.item'].quantity).toBe(1);expect(item.components['core.wearable']).toEqual({regions:'torso',layer:1});expect(next.edges.map(e=>[e.typeId,e.sourceId,e.targetId])).toEqual([['core.worn-by',item.id,person.id]]);expect(item.id.slice(4).length).toBeLessThanOrEqual(40);expect(new Set(next.edges.map(e=>e.id)).size).toBe(1);expect(input.nodes).toHaveLength(1);expect(world).toEqual(before);
  });
  it('uses custom item presets and schema-defined capability defaults when repairing a candidate',()=>{
    const world=emptyWorld();world.nodeTypes=world.nodeTypes.map(t=>t.id==='core.item'?{...t,id:'custom.possession'}:t);world.componentTypes.push({id:'custom.condition',name:'Condition',description:'',fields:[{key:'condition',label:'Condition',kind:'text',required:true,defaultValue:'sound'}]});
    const person=makeNode(world,'core.character','Nora Hale','new:person');const candidate=addCandidateWornItem(world,{nodes:[person],edges:[]},person.id);expect(candidate.nodes[1].typeId).toBe('custom.possession');const repaired=addCandidateCapability(world,candidate,candidate.nodes[1].id,'custom.condition');expect(repaired.nodes[1].components['custom.condition']).toEqual({condition:'sound'});expect(candidate.nodes[1].components['custom.condition']).toBeUndefined();
  });
  it('discovers fields from attached custom capabilities and protects the base name by default',()=>{
    const world=exampleWorld();world.componentTypes.push({id:'custom.details',name:'Extra details',description:'',fields:[{key:'name',label:'Alias',kind:'text',required:false,defaultValue:''}]});
    world.nodes[0].components['custom.details']={name:'Alias'};
    const fields=editableFields(world,[world.nodes[0].id]);expect(fields.some((f)=>f.componentId==='custom.details'&&f.key==='name')).toBe(true);
    const request=newAuthoringRequest(world,world.nodes[0].id);expect(request.fields.some((f)=>f.componentId===null&&f.key==='name')).toBe(false);expect(request.fields.some((f)=>f.componentId==='custom.details'&&f.key==='name')).toBe(true);
  });
  it('adds proposal nodes with temporary aliases and preserves canonical data',()=>{
    const world=exampleWorld(),before=structuredClone(world);const candidate=addCandidateNode(world,{nodes:[],edges:[]},'core.item');expect(candidate.nodes[0].id).toMatch(/^new:/);expect(candidate.nodes[0].components['core.item'].quantity).toBe(1);expect(world).toEqual(before);
  });
  it('keeps malformed candidate JSON out of structured controls without throwing',()=>{
    const world=exampleWorld();const node=makeNode(world,'core.item','Coat','new:root');expect(parseCandidateForEditor(JSON.stringify({nodes:[node],edges:[]}))?.nodes[0].name).toBe('Coat');
    for(const value of ['{unfinished',JSON.stringify({nodes:[{...node,components:{'core.item':null}}],edges:[]}),JSON.stringify({nodes:[{...node,name:{bad:'value'}}],edges:[]}),JSON.stringify({nodes:[],edges:[{}]})])expect(parseCandidateForEditor(value)).toBeNull();
  });
});
