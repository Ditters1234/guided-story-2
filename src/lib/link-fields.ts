import type {FieldDefinition,PropertyValue,WorldEdge} from './bindings';
import type {World} from './model';
import {defaultProperties,hasCapability,type FieldTarget} from './node-fields';

export function connectionDirection(edge:WorldEdge):'two-way'|'one-way'|'unspecified'|'directed' {
  if(edge.typeId!=='story.route-to')return 'directed';
  const direction=edge.properties?.direction;
  return direction==='two-way'||direction==='one-way'?direction:'unspecified';
}

/** Advice only: ownership never substitutes for a physical inventory link. */
export function ownershipWarnings(world:World,edge:WorldEdge,nodes:FieldTarget[]=world.nodes):string[] {
  if(edge.typeId!=='story.owned-by')return [];
  const source=nodes.find(n=>n.id===edge.sourceId),owner=nodes.find(n=>n.id===edge.targetId),warnings:string[]=[];
  if(source&&!hasCapability(source,'core.item')&&!hasCapability(source,'core.place'))warnings.push(`${source.name}: ownership normally describes an item or place as its source.`);
  if(owner&&!hasCapability(owner,'core.character')&&!hasCapability(owner,'detail.organization'))warnings.push(`${owner.name}: an owner is normally a character or organization.`);
  if(world.relationTypes.find(r=>r.id===edge.typeId)?.physicalPlacement)warnings.push('This ownership definition uses physical placement. Review it: ownership should stay separate from carrying, wearing and storage.');
  return warnings;
}

export const linkFields=(world:World,edge:WorldEdge):FieldDefinition[]=>world.relationTypes.find(d=>d.id===edge.typeId)?.fields??[];
export function changeLinkType(world:World,edge:WorldEdge,typeId:string):WorldEdge {
  if(edge.typeId===typeId)return edge;
  return {...edge,typeId,properties:defaultProperties({fields:world.relationTypes.find(d=>d.id===typeId)?.fields??[]})};
}
export function setLinkProperty(edge:WorldEdge,key:string,value:PropertyValue|undefined):WorldEdge {
  const properties:Record<string,PropertyValue>={...edge.properties};
  if(value===undefined)delete properties[key];else properties[key]=value;
  return {...edge,properties};
}
export function linkReferences(world:World,edge:WorldEdge):string[] {
  return linkFields(world,edge).flatMap(f=>{
    const v=edge.properties?.[f.key];return f.kind==='nodeRef'&&typeof v==='string'?[v]:f.kind==='nodeRefs'&&Array.isArray(v)?v:[];
  });
}
