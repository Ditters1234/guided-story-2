import {linkReferences,linkFields} from './link-fields';
import type {PassageEffect,PassageEffects,PropertyValue,WorldNode,WorldEdge} from './bindings';
import type {World} from './model';
import {hasCapability,defaultProperties} from './node-fields';

export function proseEditableNode(world:World,node:WorldNode):boolean {
  const operational=world.relationTypes.some(r=>r.physicalPlacement&&[r.sourceCapability,r.targetCapability].some(id=>!!id&&!!node.components[id]));
  const fields=world.componentTypes.filter(d=>node.components[d.id]).flatMap(d=>d.fields);
  return operational||!fields.length||fields.some(f=>!f.metadata?.uses?.length||f.metadata.uses.some(u=>u==='story'||u==='visual'));
}
export function reviewNodes(world:World,effects:PassageEffects,scopeIds:readonly string[]):WorldNode[] {
  const allowed=new Set(scopeIds),existing=new Set(world.nodes.map(n=>n.id));
  return proposedNodes(world,effects).filter(n=>allowed.has(n.id)||(!existing.has(n.id)&&/^new:[A-Za-z0-9_-]{1,60}$/.test(n.id)));
}
export function addReviewProposal(world:World,effects:PassageEffects,scopeIds:readonly string[],kind:'node'|'link'|'removeLink'|'note',id?:string):PassageEffects {
  if(kind==='note'){
    if(effects.notes.length>=12)throw Error('A passage supports at most 12 continuity notes.');
    return {...effects,notes:[...effects.notes,{text:'',evidence:'',nodeIds:[]}]};
  }
  if(effects.changes.length>=80)throw Error('A passage supports at most 80 changes.');
  const scope=new Set(scopeIds);
  let change:PassageEffect;
  if(kind==='node'){
    const saved=world.nodes.find(n=>n.id===id&&scope.has(n.id));
    if(!saved||!proseEditableNode(world,saved))throw Error('Choose an editable node supplied to this scene.');
    if(effects.changes.some(e=>e.kind==='putNode'&&e.node.id===id))throw Error('Edit the existing proposal for this node.');
    const copy={...saved,components:Object.fromEntries(Object.entries(saved.components).map(([key,values])=>[key,Object.fromEntries(Object.entries(values).map(([key,value])=>[key,Array.isArray(value)?[...value]:value]))]))};
    change={kind:'putNode',node:copy,evidence:''};
  }else if(id){
    const saved=world.edges.find(e=>e.id===id&&scope.has(e.sourceId)&&scope.has(e.targetId));
    if(!saved)throw Error('Choose a connection supplied to this scene.');
    if(effects.changes.some(e=>e.kind==='putLink'?e.link.id===id:e.kind==='removeLink'&&e.id===id))throw Error('Edit the existing proposal for this connection.');
    change=kind==='removeLink'?{kind:'removeLink',id,evidence:''}:{kind:'putLink',link:{...saved,properties:Object.fromEntries(Object.entries(saved.properties??{}).map(([key,value])=>[key,Array.isArray(value)?[...value]:value]))},evidence:''};
  }else{
    if(kind==='removeLink')throw Error('Choose a supplied connection to remove.');
    const nodes=reviewNodes(world,effects,scopeIds);
    const relations=[...world.relationTypes].sort((a,b)=>Number(b.id==='core.related')-Number(a.id==='core.related'));
    const relation=relations.find(r=>nodes.some(a=>(!r.sourceCapability||hasCapability(a,r.sourceCapability))&&nodes.some(b=>b.id!==a.id&&(!r.targetCapability||hasCapability(b,r.targetCapability)))));
    if(!relation)throw Error('Add compatible scene references before creating a connection.');
    const source=nodes.find(a=>(!relation.sourceCapability||hasCapability(a,relation.sourceCapability))&&nodes.some(b=>b.id!==a.id&&(!relation.targetCapability||hasCapability(b,relation.targetCapability))))!;
    const target=nodes.find(b=>b.id!==source.id&&(!relation.targetCapability||hasCapability(b,relation.targetCapability)))!;
    change={kind:'putLink',link:{id:`new:link-${crypto.randomUUID()}`,typeId:relation.id,sourceId:source.id,targetId:target.id,description:'',properties:defaultProperties({fields:relation.fields??[]})},evidence:''};
  }
  return {...effects,changes:[...effects.changes,change]};
}

const record=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==='object'&&!Array.isArray(v);
const property=(v:unknown)=>v===null||['string','number','boolean'].includes(typeof v)||(Array.isArray(v)&&v.every(x=>typeof x==='string'));
const evidenceShape=(v:unknown)=>typeof v==='string'||(Array.isArray(v)&&v.length<=8&&v.every(q=>typeof q==='string'));
const node=(v:unknown):v is WorldNode=>record(v)&&['id','typeId','name','description','visualDescription'].every(k=>typeof v[k]==='string')&&record(v.components)&&Object.values(v.components).every(c=>record(c)&&Object.values(c).every(property));
const link=(v:unknown):v is WorldEdge=>record(v)&&['id','typeId','sourceId','targetId','description'].every(k=>typeof v[k]==='string')&&(v.properties===undefined||(record(v.properties)&&Object.values(v.properties).every(property)));
export function parseEffects(text:string):PassageEffects|null {
  try {
    const v:unknown=JSON.parse(text);
    if(!record(v)||!Array.isArray(v.changes)||v.changes.length>80||!Array.isArray(v.notes)||v.notes.length>12||typeof v.suggestChapterEnd!=='boolean')return null;
    if(!v.changes.every(e=>record(e)&&evidenceShape(e.evidence)&&(e.kind==='putNode'?node(e.node):e.kind==='putLink'?link(e.link):e.kind==='removeLink'&&typeof e.id==='string')))return null;
    if(!v.notes.every(n=>record(n)&&typeof n.text==='string'&&typeof n.evidence==='string'&&Array.isArray(n.nodeIds)&&n.nodeIds.every(id=>typeof id==='string')))return null;
    return v as PassageEffects;
  }catch{return null;}
}
export const normalizeQuote=(text:string)=>text.split(/\p{White_Space}+/u).filter(Boolean).join(' ');
export const evidenceQuotes=(value:string|string[]):string[]=>Array.isArray(value)?value:[value];
export function evidenceIssue(prose:string,evidence:string|string[]):string {
  if(Array.isArray(evidence)){
    if(evidence.length<1||evidence.length>4)return 'Use one to four exact source quotes per change.';
    const seen=new Set<string>();
    for(const [index,quote]of evidence.entries()){
      const issue=evidenceIssue(prose,quote);if(issue)return `Quote ${index+1}: ${issue}`;
      const normalized=normalizeQuote(quote);if(seen.has(normalized))return `Quote ${index+1}: duplicate source quote.`;seen.add(normalized);
    }
    return '';
  }
  const quote=evidence;
  const normalized=normalizeQuote(quote),length=[...normalized].length;
  return length<8||length>600?'Choose a source quote between 8 and 600 characters.':!normalizeQuote(prose).includes(normalized)?'This quote does not match a continuous excerpt from the passage.':'';
}
export function passageQuotes(prose:string):string[] {
  const sentences=Array.from(new Intl.Segmenter(undefined,{granularity:'sentence'}).segment(prose),s=>s.segment);
  const result:string[]=[];
  for(const sentence of sentences){
    let remaining=sentence.trim();
    while(remaining&&result.length<200){
      const chars=[...remaining];let end=Math.min(chars.length,600);
      if(end<chars.length){const space=chars.slice(0,end).lastIndexOf(' ');if(space>=8)end=space;}
      const quote=chars.slice(0,end).join('').trim();
      if([...normalizeQuote(quote)].length>=8&&!result.includes(quote))result.push(quote);
      remaining=chars.slice(end).join('').trim();
    }
    if(result.length>=200)break;
  }
  return result;
}
export function proposedNodes(world:World,effects:PassageEffects):WorldNode[]{
  const nodes=new Map(world.nodes.map(n=>[n.id,n]));
  for(const effect of effects.changes)if(effect.kind==='putNode')nodes.set(effect.node.id,effect.node);
  return [...nodes.values()];
}
export const nodeName=(nodes:WorldNode[],id:string)=>nodes.find(n=>n.id===id)?.name||'Missing node';
export function connectionIdentity(world:World,link:WorldEdge):string {
  return /^new:[A-Za-z0-9_-]{1,60}$/.test(link.id)?'new':world.edges.some(e=>e.id===link.id)?link.id:'missing';
}
export function linkLabel(world:World,nodes:WorldNode[],edge:WorldEdge):string {
  return `${nodeName(nodes,edge.sourceId)} → ${world.relationTypes.find(r=>r.id===edge.typeId)?.label??'Unknown relationship'} → ${nodeName(nodes,edge.targetId)}`;
}
export function effectTitle(world:World,effects:PassageEffects,effect:PassageEffect):string {
  const nodes=proposedNodes(world,effects);
  if(effect.kind==='putNode')return `${world.nodes.some(n=>n.id===effect.node.id)?'Update':'Create'} ${world.nodeTypes.find(t=>t.id===effect.node.typeId)?.name??'node'}: ${effect.node.name||'Unnamed'}`;
  if(effect.kind==='putLink')return linkLabel(world,nodes,effect.link);
  const old=world.edges.find(e=>e.id===effect.id);return old?`Remove: ${linkLabel(world,nodes,old)}`:'Remove connection';
}
function typedReferences(world:World,n:WorldNode):string[]{
  return Object.entries(n.components).flatMap(([id,values])=>(world.componentTypes.find(c=>c.id===id)?.fields??[]).flatMap(f=>{
    const value=values[f.key];return f.kind==='nodeRef'&&typeof value==='string'?[value]:f.kind==='nodeRefs'&&Array.isArray(value)?value:[];
  }));
}
function references(world:World,e:PassageEffect):string[]{return e.kind==='putNode'?typedReferences(world,e.node):e.kind==='putLink'?[e.link.sourceId,e.link.targetId,...linkReferences(world,e.link)]:[];}
export type ProposalRemoval={changes:number[];notes:number[]};
export function removalPlan(world:World,effects:PassageEffects,kind:'change'|'note',index:number):ProposalRemoval {
  if(kind==='note')return {changes:[],notes:[index]};
  const changes=new Set([index]),removedNodes=new Set<string>();let added=true;
  while(added){added=false;
    for(const i of changes){const e=effects.changes[i];if(e?.kind==='putNode'&&!world.nodes.some(n=>n.id===e.node.id))removedNodes.add(e.node.id);}
    const replacements=[...changes].flatMap(i=>{const e=effects.changes[i];return e?.kind==='putLink'?[e.link]:[];});
    effects.changes.forEach((e,i)=>{
      const old=e.kind==='removeLink'?world.edges.find(link=>link.id===e.id):null;
      const paired=!!old&&replacements.some(link=>{
        const definition=world.relationTypes.find(r=>r.id===link.typeId);
        return old.sourceId===link.sourceId&&((definition?.singleTarget&&old.typeId===link.typeId)||(definition?.physicalPlacement&&world.relationTypes.some(r=>r.id===old.typeId&&r.physicalPlacement)));
      });
      if(!changes.has(i)&&(paired||references(world,e).some(id=>removedNodes.has(id)))){changes.add(i);added=true;}
    });
  }
  return {changes:[...changes].sort((a,b)=>a-b),notes:effects.notes.flatMap((n,i)=>n.nodeIds.some(id=>removedNodes.has(id))?[i]:[])};
}
export function removeProposals(effects:PassageEffects,plan:ProposalRemoval):PassageEffects {
  return {...effects,changes:effects.changes.filter((_,i)=>!plan.changes.includes(i)),notes:effects.notes.filter((_,i)=>!plan.notes.includes(i))};
}
export function valueLabel(value:PropertyValue|undefined,nodes:WorldNode[],kind?:string):string {
  if(value===undefined||value===null||value==='')return 'Unspecified';
  if(Array.isArray(value))return value.map(id=>kind==='nodeRefs'?nodeName(nodes,id):id).join(', ')||'None';
  return typeof value==='boolean'?value?'Yes':'No':kind==='nodeRef'?nodes.find(n=>n.id===value)?.name??String(value):String(value);
}
export function nodeDiff(world:World,nodes:WorldNode[],next:WorldNode):{label:string;before:string;after:string}[]{
  const old=world.nodes.find(n=>n.id===next.id);if(!old)return [];
  const rows:{label:string;before:string;after:string}[]=[];
  for(const [key,label] of [['name','Name'],['description','Story notes'],['visualDescription','Visible description']] as const)if(old[key]!==next[key])rows.push({label,before:old[key]||'Unspecified',after:next[key]||'Unspecified'});
  for(const id of new Set([...Object.keys(old.components),...Object.keys(next.components)])){
    const definition=world.componentTypes.find(c=>c.id===id);
    for(const key of new Set([...Object.keys(old.components[id]??{}),...Object.keys(next.components[id]??{})])){
      const before=old.components[id]?.[key],after=next.components[id]?.[key];
      const field=definition?.fields.find(f=>f.key===key);
      if(JSON.stringify(before??null)!==JSON.stringify(after??null))rows.push({label:`${definition?.name??id} · ${field?.label??key}`,before:valueLabel(before,nodes,field?.kind),after:valueLabel(after,nodes,field?.kind)});
    }
  }
  return rows;
}

export function nodeDetails(world:World,nodes:WorldNode[],node:WorldNode):{label:string;value:string}[] {
  return Object.entries(node.components).flatMap(([id,values])=>{
    const definition=world.componentTypes.find(d=>d.id===id);
    return Object.entries(values).filter(([,value])=>value!==null&&value!==''&&(!Array.isArray(value)||value.length)).map(([key,value])=>{
      const field=definition?.fields.find(f=>f.key===key);
      return {label:`${definition?.name??id} · ${field?.label??key}`,value:valueLabel(value,nodes,field?.kind)};
    });
  });
}

export function linkDiff(world:World,nodes:WorldNode[],next:WorldEdge,old?:WorldEdge|null):{label:string;before:string;after:string}[]{
  const rows:{label:string;before:string;after:string}[]=[];
  if((old?.description??'')!==next.description)rows.push({label:'Connection notes',before:old?.description||'Unspecified',after:next.description||'Unspecified'});
  for(const key of new Set([...Object.keys(old?.properties??{}),...Object.keys(next.properties??{})])){
    const before=old?.properties?.[key],after=next.properties?.[key];
    if(JSON.stringify(before??null)===JSON.stringify(after??null))continue;
    const beforeField=old?linkFields(world,old).find(f=>f.key===key):undefined,afterField=linkFields(world,next).find(f=>f.key===key);
    rows.push({label:afterField?.label??beforeField?.label??key,before:valueLabel(before,nodes,beforeField?.kind),after:valueLabel(after,nodes,afterField?.kind)});
  }
  return rows;
}
