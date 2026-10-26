import { describe,it,expect } from 'vitest';
import { exampleWorld,emptyWorld,makeNode } from './model';
import { addCandidateNode,editableFields,newAuthoringRequest,parseCandidateForEditor,changeCreationType,addCandidateWornItem,addCandidateCapability } from './authoring-ui';
describe('shared AI node editor',()=>{
  it('starts a new world as a mixed story idea and enables possessions for character presets',()=>{
    const world=emptyWorld(),request=newAuthoringRequest(world);expect(request.typeId).toBeNull();expect(request.bundle).toBe(true);
    const populated=exampleWorld(),single=newAuthoringRequest(populated);expect(single.bundle).toBe(false);
    world.nodeTypes.push({id:'custom.person',name:'Settler',description:'',components:['core.character','core.holder']});
    for(const type of ['core.character','custom.person'])expect(changeCreationType(world,single,type).bundle).toBe(true);
    expect(changeCreationType(world,single,null).bundle).toBe(true);expect(request.direction).toBe('');
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
