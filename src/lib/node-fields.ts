import type {ComponentDefinition, FieldDefinition, PropertyValue, WorldNode} from './bindings';
import type {World} from './model';

export type FieldTarget = {id:string;name:string;components?:WorldNode['components'];capabilities?:string[]};
export const hasCapability = (node:FieldTarget|undefined,id:string):boolean => !!node && (
  !!node.components?.[id] || !!node.capabilities?.includes(id) || (id==='core.holder' && hasCapability(node,'core.container')));

export function defaultProperties(definition:ComponentDefinition):Record<string,PropertyValue> {
  return Object.fromEntries(definition.fields.filter(f=>f.required||f.defaultValue!==null).map(f=>[f.key,Array.isArray(f.defaultValue)?[...f.defaultValue]:f.defaultValue]));
}
export function capabilitySections(world:World,node:WorldNode) {
  const order=[...(world.nodeTypes.find(t=>t.id===node.typeId)?.components??[]),...Object.keys(node.components)];
  return [...new Set(order)].flatMap(id=>{
    const definition=world.componentTypes.find(d=>d.id===id),values=node.components[id];
    return definition?.fields.length&&values?[{definition,values}]:[];
  });
}
export const hasFieldValue=(value:PropertyValue|undefined):boolean=>value!==null&&value!==undefined&&(
  typeof value==='string'?!!value.trim():Array.isArray(value)?!!value.length:true);

export function sectionSummary(definition:ComponentDefinition,values:Record<string,PropertyValue>):string {
  const parts=definition.fields.filter(f=>hasFieldValue(values[f.key])&&f.kind!=='nodeRef'&&f.kind!=='nodeRefs')
    .slice(0,2).map(f=>`${f.label}: ${String(values[f.key]).replaceAll(/\s+/g,' ')}`);
  const text=parts.join(' · ');
  return text.length>115?text.slice(0,112)+'…':text;
}
export function referenceTargets(field:FieldDefinition,nodes:FieldTarget[]):FieldTarget[] {
  const capability=field.metadata?.referenceCapability;
  return capability?nodes.filter(n=>hasCapability(n,capability)):nodes;
}
export function nodeSearchText(node:WorldNode):string {
  return [node.name,node.description,node.visualDescription,...Object.values(node.components).flatMap(values=>Object.values(values).map(v=>Array.isArray(v)?v.join(' '):v===null?'':String(v)))].join(' ');
}
