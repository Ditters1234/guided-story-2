import { describe, expect, it } from 'vitest';
import { buildGraph, editRecord, visibleRecords, worldRecords } from './graph';
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
});
