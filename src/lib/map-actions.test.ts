import { describe, expect, it } from 'vitest';
import { copyMapNodes, mapShortcut, retainMapSelection } from './map-actions';
import { exampleWorld } from './model';

describe('map actions', () => {
  it('copies only selected nodes and internal links, retaining frozen rich values', () => {
    const world = exampleWorld();
    const person = world.nodes.find(node => node.name === 'Mara Vale')!;
    const coat = world.nodes.find(node => node.name === 'Waxed field coat')!;
    const original = structuredClone(world);
    const copy = copyMapNodes(world, [person.id, coat.id])!;
    expect(copy.nodes).toHaveLength(2);
    expect(copy.edges).toHaveLength(1);
    expect(copy.edges[0]).toMatchObject({sourceId:coat.id,targetId:person.id,typeId:'core.worn-by'});
    expect(world).toEqual(original);
    person.name = 'Changed';
    expect(copy.nodes.find(node => node.id === person.id)?.name).toBe('Mara Vale');
    expect(copyMapNodes(world, [])).toBeNull();
  });

  it('does not recursively copy a collapsed holder or copy its external links', () => {
    const world = exampleWorld();
    const copy = copyMapNodes(world, [world.nodes[0].id])!;
    expect(copy.nodes).toHaveLength(1);
    expect(copy.edges).toEqual([]);
    expect('sourceAnnotations' in copy).toBe(false);
    expect('aiContext' in copy).toBe(false);
  });

  it('preserves selected groups and edges through graph rebuilds and permits external focus', () => {
    const next = [{id:'a',selected:false,position:{x:1,y:2}}, {id:'b',selected:false,position:{x:3,y:4}}];
    const previous = next.map(node => ({...node,selected:true}));
    expect(retainMapSelection(next,previous,true).map(node => node.selected)).toEqual([true,true]);
    expect(retainMapSelection(next,previous,false)).toEqual(next);
    expect(retainMapSelection(next,[],true)).toEqual(next);
  });

  it('uses bounded familiar shortcuts without stealing alternate modified commands', () => {
    const event = {key:'c',ctrlKey:true,metaKey:false,shiftKey:false,altKey:false};
    expect(mapShortcut(event)).toBe('copy');
    expect(mapShortcut({...event,key:'V'})).toBe('paste');
    expect(mapShortcut({...event,key:'d'})).toBe('duplicate');
    expect(mapShortcut({...event,key:'a'})).toBe('all');
    expect(mapShortcut({...event,key:'c',shiftKey:true})).toBeNull();
    expect(mapShortcut({...event,altKey:true})).toBeNull();
    expect(mapShortcut({...event,ctrlKey:false,key:'Delete'})).toBe('delete');
    expect(mapShortcut({...event,ctrlKey:false,key:'Escape'})).toBe('clear');
    expect(mapShortcut({...event,ctrlKey:false,key:'v'})).toBeNull();
  });
});
