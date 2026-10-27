import type {AiContextBinding,TextPromptTask,WorldNode} from './bindings';
import {makeNode,type World} from './model';
export const aiContextTasks:{id:TextPromptTask;label:string}[]=[{id:'world',label:'World creation / rerolls'},{id:'passage',label:'Passage writing'},{id:'wording',label:'Wording edits'},{id:'effects',label:'World changes'},{id:'memory',label:'Chapter memory'},{id:'imagePrompt',label:'Image-prompt assistance'}];
export function defaultContextBinding(nodeId:string,order=0):AiContextBinding{return {nodeId,enabled:true,role:'reference',tasks:aiContextTasks.map(t=>t.id),fields:['description','components'],order};}
export function setContextBinding(world:World,binding:AiContextBinding):World {
  return {...world,aiContext:[...(world.aiContext??[]).filter(b=>b.nodeId!==binding.nodeId),binding]};
}
export function removeContextBinding(world:World,nodeId:string):World{return {...world,aiContext:(world.aiContext??[]).filter(b=>b.nodeId!==nodeId)};}
export function addContextNote(world:World,id=crypto.randomUUID()):{world:World;node:WorldNode}{
  const type={id:'core.note',name:'Note',description:'General notes, instructions, rules or background information.',components:[]};
  const next={...world,nodeTypes:world.nodeTypes.some(t=>t.id===type.id)?world.nodeTypes:[...world.nodeTypes,type]};
  const node=makeNode(next,type.id,'New context note',id);
  return {world:setContextBinding({...next,nodes:[...next.nodes,node]},defaultContextBinding(id,(world.aiContext?.length??0)*10)),node};
}
