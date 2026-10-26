import type { AuthoringRequest,EditableField,NodeCandidate,WorldNode } from './bindings';
import type { World } from './model';
import { makeNode } from './model';
export const fieldId=(field:EditableField)=>JSON.stringify([field.nodeId,field.componentId,field.key]);
export function editableFields(world:World,ids:string[]):(EditableField&{label:string})[]{
  return world.nodes.filter((n)=>ids.includes(n.id)).flatMap((node)=>[
    ...[['name','Name'],['description','Story notes'],['visualDescription','Visible description']].map(([key,label])=>({nodeId:node.id,componentId:null,key,label})),
    ...Object.keys(node.components).flatMap((id)=>{const definition=world.componentTypes.find((d)=>d.id===id);return (definition?.fields??[]).map((field)=>({nodeId:node.id,componentId:id,key:field.key,label:(definition?.name??id)+' / '+field.label}));}),
  ]);
}
export function newAuthoringRequest(world:World,targetId?:string):AuthoringRequest{
  return targetId?{mode:'revise',direction:'',typeId:null,bundle:false,targetIds:[targetId],contextIds:[],fields:editableFields(world,[targetId]).filter((f)=>f.componentId!==null||f.key!=='name').map(({label:_,...field})=>field)}:
    {mode:'create',direction:'',typeId:world.nodes.length?world.nodeTypes[0]?.id??null:null,bundle:!world.nodes.length,targetIds:[],contextIds:[],fields:[]};
}
export function changeCreationType(world:World,request:AuthoringRequest,typeId:string|null):AuthoringRequest{return {...request,typeId,bundle:!typeId||!!world.nodeTypes.find(d=>d.id===typeId)?.components.includes('core.character')||request.bundle};}
export function addCandidateCapability(world:World,candidate:NodeCandidate,nodeId:string,componentId:string):NodeCandidate{
  const definition=world.componentTypes.find(d=>d.id===componentId);
  if(!definition)throw Error('Choose a capability from this project.');
  return editCandidateNode(candidate,nodeId,node=>node.components[componentId]?node:{...node,components:{...node.components,[componentId]:Object.fromEntries(definition.fields.map(f=>[f.key,structuredClone(f.defaultValue)]))}});
}
export function wornItemType(world:World):string|null{
  return world.componentTypes.some(d=>d.id==='core.wearable')&&['core.stored-in','core.worn-by'].every(id=>world.relationTypes.some(r=>r.id===id))?world.nodeTypes.find(d=>d.components.includes('core.item'))?.id??null:null;
}
export function addCandidateWornItem(world:World,candidate:NodeCandidate,wearerId:string):NodeCandidate{
  const typeId=wornItemType(world),wearer=candidate.nodes.find(n=>n.id===wearerId);
  if(!typeId||!wearer?.components['core.character'])throw Error('Choose a proposed character and a schema supporting wearable items.');
  const item=makeNode(world,typeId,'New worn item','new:garment-'+crypto.randomUUID().replaceAll('-',''));
  let next=addCandidateCapability(world,{...candidate,nodes:[...candidate.nodes,item]},item.id,'core.wearable');
  next={...next,edges:[...next.edges,...['core.stored-in','core.worn-by'].map(typeId=>({id:'new:link-'+crypto.randomUUID().replaceAll('-',''),typeId,sourceId:item.id,targetId:wearerId,description:''}))]};
  return next;
}
export function addCandidateNode(world:World,candidate:NodeCandidate,typeId:string):NodeCandidate{
  const name=world.nodeTypes.find((d)=>d.id===typeId)?.name??'node';
  const node=makeNode(world,typeId,'New '+name.toLowerCase(),'new:'+crypto.randomUUID().replaceAll('-',''));
  return {...candidate,nodes:[...candidate.nodes,node]};
}
export function editCandidateNode(candidate:NodeCandidate,id:string,change:(node:WorldNode)=>WorldNode):NodeCandidate{return {...candidate,nodes:candidate.nodes.map((node)=>node.id===id?change(node):node)};}
export function parseCandidateForEditor(text:string):NodeCandidate|null{
  const record=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==='object'&&!Array.isArray(value);
  const property=(value:unknown)=>value===null||['string','number','boolean'].includes(typeof value)||(Array.isArray(value)&&value.every((v)=>typeof v==='string'));
  try{const value:unknown=JSON.parse(text);if(!record(value)||!Array.isArray(value.nodes)||!Array.isArray(value.edges)||value.nodes.length>20||value.edges.length>80)return null;
    if(!value.nodes.every((n)=>record(n)&&['id','typeId','name','description','visualDescription'].every((key)=>typeof n[key]==='string')&&record(n.components)&&Object.values(n.components).every((values)=>record(values)&&Object.values(values).every(property))))return null;
    if(!value.edges.every((e)=>record(e)&&['id','typeId','sourceId','targetId','description'].every((key)=>typeof e[key]==='string')))return null;
    return value as unknown as NodeCandidate;
  }catch{return null;}
}
