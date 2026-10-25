import { describe,it,expect } from 'vitest';
import { exampleWorld,makeNode } from './model';
import { addCandidateNode,editableFields,newAuthoringRequest,parseCandidateForEditor } from './authoring-ui';
describe('shared AI node editor',()=>{
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
