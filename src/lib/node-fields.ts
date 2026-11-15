import type {ComponentDefinition, FieldDefinition, PropertyValue, WorldNode} from './bindings';
import type {World} from './model';

export type FieldTarget = {id:string;name:string;components?:WorldNode['components'];capabilities?:string[]};
export const hasCapability = (node:FieldTarget|undefined,id:string):boolean => !!node && (
  !!node.components?.[id] || !!node.capabilities?.includes(id) || (id==='core.holder' && hasCapability(node,'core.container')));

export function defaultProperties(definition:Pick<ComponentDefinition,'fields'>):Record<string,PropertyValue> {
  return Object.fromEntries(definition.fields.filter(f=>f.required||f.defaultValue!==null).map(f=>[f.key,Array.isArray(f.defaultValue)?[...f.defaultValue]:f.defaultValue]));
}
export function imageOnlyField(field:FieldDefinition):boolean {
  const uses=field.metadata?.uses;
  return !!uses?.length && uses.every(use=>use==='visual'||use==='imageGuidance');
}

export function imageOnlyExtension(definition:ComponentDefinition):boolean {
  return definition.fields.length>0&&definition.fields.every(imageOnlyField);
}

export type FieldAudience='all'|'story'|'image';
export function retiredWritingField(capability: string, key: string): boolean {
  return capability === 'core.narration-style' && key === 'viewpointId';
}

export function capabilitySections(world:World,node:WorldNode,audience:FieldAudience='all') {
  const order=[...(world.nodeTypes.find(t=>t.id===node.typeId)?.components??[]),...Object.keys(node.components)];
  return [...new Set(order)].flatMap(id=>{
    const definition=world.componentTypes.find(d=>d.id===id),values=node.components[id];
    if(!definition||!values)return [];
    const fields=definition.fields.filter(field=>!retiredWritingField(id,field.key)&&(audience==='all'||imageOnlyField(field)===(audience==='image')));
    return fields.length?[{definition:{...definition,fields},values}]:[];
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
  return [node.name,node.description,...Object.values(node.components).flatMap(values=>Object.values(values).map(v=>Array.isArray(v)?v.join(' '):v===null?'':String(v)))].join(' ');
}
