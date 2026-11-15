import {describe,it,expect} from 'vitest';
import {exampleWorld} from '../test-fixtures/world';
import {emptyWorld,makeNode,availableNodeTypes} from './model';
import {defaultContextBinding,setContextBinding,removeContextBinding,addContextNote,addNarrationStyle} from './ai-context';
import {editableFields} from './authoring-ui';
describe('story-wide node context',()=>{
  it('keeps image fields in optional extensions',()=>{
    const world=exampleWorld();
    expect(availableNodeTypes(world).some(p=>p.id==='core.visual-style')).toBe(false);
    expect(world.nodeTypes.some(p=>p.components.some(c=>['detail.image','detail.anatomy','detail.visual-style'].includes(c)))).toBe(false);
    const fields=world.componentTypes.find(d=>d.id==='detail.visual-style')!.fields;
    expect(fields.map(f=>f.key)).toEqual(['medium','rendering','palette','lighting','composition','instructions']);
    expect(fields.every(f=>f.metadata?.uses?.length===1&&f.metadata.uses[0]==='imageGuidance')).toBe(true);
  });
  it('supports image guidance on custom presets while preserving explicitly selected tools',()=>{
    const world=emptyWorld(),node=makeNode(world,'core.note','Presentation');
    world.nodes.push(node);node.components['detail.visual-style']={palette:'Ultramarine'};
    node.typeId='custom.presentation';node.components['core.narration-style']={person:'Third person',tense:'Past tense'};
    expect(defaultContextBinding(node.id,7,world)).toMatchObject({role:'instructions',tasks:['passage','wording','imagePrompt'],imagePreparation:true,order:7});
    const custom=setContextBinding(world,{...defaultContextBinding(node.id,7,world),tasks:['world'],imagePreparation:false});
    expect(custom.aiContext?.[0]).toMatchObject({tasks:['world'],imagePreparation:false});
    expect(defaultContextBinding('ordinary-node')).not.toHaveProperty('imagePreparation');
  });
  it('offers narration in older stories and explicitly adds its reusable fields and writing-only inclusion',()=>{
    const world=exampleWorld();world.nodeTypes=world.nodeTypes.filter(t=>t.id!=='core.narration-style');world.componentTypes=world.componentTypes.filter(t=>t.id!=='core.narration-style');
    const before=structuredClone(world);expect(availableNodeTypes(world).some(t=>t.id==='core.narration-style')).toBe(true);expect(world).toEqual(before);
    const first=addNarrationStyle(world,'10000000-0000-4000-8000-000000000031');
    expect(first.node.components['core.narration-style']).toEqual({person:'Third person',tense:'Past tense'});
    expect(first.world.aiContext?.[0]).toMatchObject({nodeId:first.node.id,role:'instructions',tasks:['passage','wording'],fields:['description','components'],enabled:true});
    expect(first.world.edges).toEqual(before.edges);expect(first.world.nodes.slice(0,-1)).toEqual(before.nodes);expect(world).toEqual(before);
    expect(editableFields(first.world,[first.node.id]).some(f=>f.componentId==='core.narration-style'&&f.key==='instructions')).toBe(true);
    expect(first.world.componentTypes.find(d=>d.id==='core.narration-style')?.fields.find(f=>f.key==='instructions')?.kind).toBe('longText');
    const second=addNarrationStyle(first.world,'10000000-0000-4000-8000-000000000032');
    expect(second.world.nodeTypes.filter(t=>t.id==='core.narration-style')).toHaveLength(1);expect(second.world.componentTypes.filter(t=>t.id==='core.narration-style')).toHaveLength(1);expect(second.world.aiContext).toHaveLength(2);
  });
  it('uses narration inclusion defaults for any node with the capability while preserving explicit inclusion changes',()=>{
    const initial=addNarrationStyle(emptyWorld()),node=initial.node;
    const custom={...node,typeId:'custom.guidance'};const world={...initial.world,nodes:[custom],nodeTypes:[...initial.world.nodeTypes,{id:'custom.guidance',name:'Guidance',description:'',components:['core.narration-style']}]};
    const removed=removeContextBinding(world,node.id);expect(removed.aiContext).toEqual([]);expect(removed.nodes[0]).toEqual(custom);
    expect(defaultContextBinding(node.id,20,removed)).toMatchObject({role:'instructions',tasks:['passage','wording'],order:20});
    const configured=setContextBinding(removed,{...defaultContextBinding(node.id,20,removed),enabled:false,role:'reference',tasks:['world'],fields:['components']});
    expect(configured.aiContext?.[0]).toMatchObject({enabled:false,role:'reference',tasks:['world']});expect(configured.nodes).toEqual(removed.nodes);
  });
  it('includes arbitrary custom nodes without changing physical links or expanding connected records',()=>{
    const world=exampleWorld(),node=makeNode(world,'core.note','Rules'),before=structuredClone(world);world.nodes.push(node);
    const binding={...defaultContextBinding(node.id),role:'instructions' as const,tasks:['passage' as const],order:-10};
    const next=setContextBinding(world,binding);expect(next.aiContext).toEqual([binding]);expect(next.nodes).toEqual(world.nodes);expect(next.edges).toEqual(before.edges);expect(world.aiContext).toBeUndefined();
    const disabled=setContextBinding(next,{...binding,enabled:false});expect(disabled.aiContext).toHaveLength(1);expect(disabled.aiContext?.[0].tasks).toEqual(['passage']);
  });
  it('removes only the inclusion and retains its source node and other bindings',()=>{
    const world=exampleWorld(),a=world.nodes[0].id,b=world.nodes[1].id;const next=setContextBinding(setContextBinding(world,defaultContextBinding(a)),defaultContextBinding(b));
    const removed=removeContextBinding(next,a);expect(removed.aiContext?.map(b=>b.nodeId)).toEqual([b]);expect(removed.nodes).toEqual(world.nodes);expect(next.aiContext).toHaveLength(2);
  });
  it('adds a plain context note to older project schemas without mutating them or duplicating the preset',()=>{
    const world=emptyWorld();world.nodeTypes=world.nodeTypes.filter(t=>t.id!=='core.note');const before=structuredClone(world);
    const first=addContextNote(world,'10000000-0000-4000-8000-000000000001');expect(first.node.components).toEqual({});expect(first.node.description).toBe('');expect(first.world.aiContext?.[0].nodeId).toBe(first.node.id);
    const second=addContextNote(first.world,'10000000-0000-4000-8000-000000000002');expect(second.world.nodeTypes.filter(t=>t.id==='core.note')).toHaveLength(1);expect(second.world.aiContext).toHaveLength(2);expect(world).toEqual(before);
  });
});
