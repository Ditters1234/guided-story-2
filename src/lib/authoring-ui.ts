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
    {mode:'create',direction:'',typeId:world.nodeTypes[0]?.id??null,bundle:false,targetIds:[],contextIds:[],fields:[]};
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
