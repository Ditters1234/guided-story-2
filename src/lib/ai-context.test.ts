import {describe,it,expect} from 'vitest';
import {exampleWorld,emptyWorld,makeNode} from './model';
import {defaultContextBinding,setContextBinding,removeContextBinding,addContextNote} from './ai-context';
describe('story-wide node context',()=>{
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
