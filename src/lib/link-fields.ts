import type {FieldDefinition,PropertyValue,WorldEdge} from './bindings';
import type {World} from './model';
import {defaultProperties} from './node-fields';

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
