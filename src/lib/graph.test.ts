import { describe, expect, it } from 'vitest';
import { buildGraph, editRecord, orderedRecords, revealRecord, visibleRecords, worldRecords } from './graph';
import { exampleWorld, makeNode } from './model';

describe('world projection', () => {
  it('collapses physical contents without changing the canonical graph', () => {
    const world = exampleWorld();
    const original = structuredClone(world);
    const visible = visibleRecords(worldRecords(world), []);
    expect(visible.map((r) => r.name)).toEqual(['Brasswater House']);
    expect(world).toEqual(original);
  });
  it('finds nested items with their ancestor path even when collapsed', () => {
    const records = worldRecords(exampleWorld());
    const found = visibleRecords(records, [], 'Seed packets');
    expect(found.map((r) => r.name)).toEqual(['Brasswater House', 'The glasshouse', 'Mara Vale', 'Canvas satchel', 'Seed packets']);
  });
  it('projects wear from explicit links rather than assuming all carried items are worn', () => {
    const records = worldRecords(exampleWorld());
    expect(records.find((r) => r.name === 'Waxed field coat')?.relation).toBe('worn by');
    expect(records.find((r) => r.name === 'Canvas satchel')?.relation).toBe('carried by');
  });
  it('retains custom nodes, capability fields and typed nonphysical edges', () => {
    const world = exampleWorld();
    world.componentTypes.push({ id: 'custom.weather', name: 'Weather', description: '', fields: [{ key: 'forecast', label: 'Forecast', kind: 'text', required: false, defaultValue: 'Rain' }] });
    world.nodeTypes.push({ id: 'custom.storm', name: 'Storm', description: '', components: ['custom.weather'] });
    world.relationTypes.push({ id: 'custom.affects', name: 'Affects', label: 'affects', sourceCapability: null, targetCapability: null, physicalPlacement: false, singleTarget: false });
    const storm = makeNode(world, 'custom.storm', 'Gathering clouds');
    world.nodes.push(storm);
    world.edges.push({ id: crypto.randomUUID(), typeId: 'custom.affects', sourceId: storm.id, targetId: world.nodes[0].id, description: '' });
    const graph = buildGraph(world, { version: 0, positions: {}, selectedId: storm.id, expandedIds: [] }, '', () => {});
    expect(graph.nodes.find((n) => n.id === storm.id)?.data.kind).toBe('custom');
    expect(graph.edges.some((e) => e.label === 'affects')).toBe(true);
    const edited = editRecord(world, storm.id, 'description', 'Rain before evening.');
    expect(edited.nodes.find((n) => n.id === storm.id)?.components['custom.weather'].forecast).toBe('Rain');
  });
  it('projects every visible canonical link with its original ID and direction, including worn and held links', () => {
    const world = exampleWorld();
    const graph = buildGraph(world, {version:0,positions:{},selectedId:null,expandedIds:world.nodes.map((n)=>n.id)}, '', () => {});
    expect(graph.edges.map((e)=>({id:e.id,source:e.source,target:e.target}))).toEqual(world.edges.map((e)=>({id:e.id,source:e.sourceId,target:e.targetId})));
    const coat = world.nodes.find((n)=>n.name==='Waxed field coat')!;
    expect(graph.edges.filter((e)=>e.source===coat.id)).toHaveLength(2);
  });
  it('never turns thread grouping into a containment cycle', () => {
    const world = exampleWorld();
    const thread = world.nodes.find((n)=>n.components['core.thread'])!;
    thread.components['core.place'] = {};
    const house = world.nodes[0];
    world.edges = [{id:crypto.randomUUID(),typeId:'core.inside',sourceId:house.id,targetId:thread.id,description:''},
      {id:crypto.randomUUID(),typeId:'core.related',sourceId:thread.id,targetId:house.id,description:''}];
    const records = worldRecords(world);
    expect(records.find((r)=>r.id===thread.id)?.parentId).toBeNull();
    expect(orderedRecords(records).map((r)=>r.id)).toContain(house.id);
  });
  it('keeps a moved container and nested contents discoverable through their new path', () => {
    const world = exampleWorld();
    const bag = world.nodes.find((n)=>n.name==='Canvas satchel')!;
    const room = world.nodes.find((n)=>n.name==='The map room')!;
    world.edges.find((e)=>e.sourceId===bag.id)!.targetId = room.id;
    const found = visibleRecords(worldRecords(world), [], 'Seed packets');
    expect(found.map((r)=>r.name)).toEqual(['Brasswater House','The map room','Canvas satchel','Seed packets']);
  });
  it('reveals the complete ancestor path when following an inspector connection', () => {
    const records = worldRecords(exampleWorld());
    const seeds = records.find((r) => r.name === 'Seed packets')!;
    const expanded = revealRecord(records, [], seeds.id);
    expect(visibleRecords(records, expanded).some((r) => r.id === seeds.id)).toBe(true);
    expect(expanded).toHaveLength(4);
  });
  it('lays out deeply nested worlds without recursive stack overflow', () => {
    const world = exampleWorld();
    world.nodes = []; world.edges = [];
    for (let i=0;i<1500;i++) {
      const node = makeNode(world, 'core.place', `Place ${i}`);
      if (i) world.edges.push({id:crypto.randomUUID(),typeId:'core.inside',sourceId:node.id,targetId:world.nodes[i-1].id,description:''});
      world.nodes.push(node);
    }
    const graph = buildGraph(world, {version:0,positions:{},selectedId:null,expandedIds:world.nodes.map((n)=>n.id)}, '', () => {});
    expect(graph.nodes).toHaveLength(1500);
    expect(orderedRecords(worldRecords(world))).toHaveLength(1500);
    expect(graph.nodes.at(-1)?.position.x).toBe(1499*280);
  });
});
