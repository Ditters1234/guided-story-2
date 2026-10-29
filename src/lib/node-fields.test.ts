import {describe,it,expect} from 'vitest';
import {emptyWorld,makeNode} from './model';
import {capabilitySections,defaultProperties,hasCapability,hasFieldValue,referenceTargets,sectionSummary} from './node-fields';
import {worldRecords,visibleRecords} from './graph';
import type {FieldDefinition} from './bindings';

describe('composable node editing',()=>{
  it('creates useful character bundles without inventing optional profile facts',()=>{
    const world=emptyWorld(),node=makeNode(world,'core.character','Nora');
    for(const id of ['detail.profile','detail.appearance','detail.personality','detail.motivation','detail.voice','detail.intent'])expect(node.components[id]).toEqual({});
    expect(capabilitySections(world,node).map(s=>s.definition.name)).toContain('Goals');
    const item=makeNode(world,'core.item','Pump handle');expect(item.components['core.item']).toEqual({quantity:1,unit:'item'});
  });
  it('uses custom presets and independent capabilities rather than a single display kind',()=>{
    const world=emptyWorld();world.nodeTypes.push({id:'custom.traveller',name:'Traveller',description:'',components:['core.character','core.item','core.container','detail.voice']});
    const node=makeNode(world,'custom.traveller','Clockwork traveller');
    expect(hasCapability(node,'core.character')).toBe(true);expect(hasCapability(node,'core.item')).toBe(true);expect(hasCapability(node,'core.holder')).toBe(true);
    expect(capabilitySections(world,node).map(s=>s.definition.name)).toEqual(['Inventory item','Voice']);
    node.components['detail.voice'].style='Replies in clipped questions';
    world.nodes.push(node);expect(visibleRecords(worldRecords(world),[],'clipped questions').map(n=>n.id)).toEqual([node.id]);
  });
  it('keeps zero and false distinct from unspecified values in defaults and summaries',()=>{
    const definition={id:'custom.state',name:'State',description:'',fields:[
      {key:'count',label:'Count',kind:'number' as const,required:false,defaultValue:0},
      {key:'ready',label:'Ready',kind:'boolean' as const,required:false,defaultValue:false},
      {key:'unknown',label:'Unknown',kind:'boolean' as const,required:false,defaultValue:null}]};
    expect(defaultProperties(definition)).toEqual({count:0,ready:false});
    expect(hasFieldValue(false)).toBe(true);expect(hasFieldValue(0)).toBe(true);expect(hasFieldValue(null)).toBe(false);
    expect(sectionSummary(definition,{count:0,ready:false})).toBe('Count: 0 · Ready: false');
  });
  it('filters typed references with the same container-holder implication and supports index entries',()=>{
    const world=emptyWorld(),person=makeNode(world,'core.character','Nora'),item=makeNode(world,'core.item','Handle');
    const field:FieldDefinition={key:'holder',label:'Holder',kind:'nodeRef',required:false,defaultValue:null,metadata:{referenceCapability:'core.holder'}};
    const index={id:'index-bag',name:'Bag',capabilities:['core.container']};
    expect(referenceTargets(field,[person,item,index]).map(n=>n.name)).toEqual(['Nora','Bag']);
  });
});
