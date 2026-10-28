import type {AiContextBinding,TextPromptTask,WorldNode} from './bindings';
import {makeNode,withBuiltinNodeType,type World} from './model';
export const aiContextTasks:{id:TextPromptTask;label:string}[]=[{id:'world',label:'World creation / rerolls'},{id:'passage',label:'Passage writing'},{id:'wording',label:'Wording edits'},{id:'effects',label:'World changes'},{id:'memory',label:'Chapter memory'},{id:'imagePrompt',label:'Image-prompt assistance'}];
export function defaultContextBinding(nodeId:string,order=0,world?:World):AiContextBinding{
  const narration=!!world?.nodes.find(n=>n.id===nodeId)?.components['core.narration-style'];
  return {nodeId,enabled:true,role:narration?'instructions':'reference',tasks:narration?['passage','wording']:aiContextTasks.map(t=>t.id),fields:['description','components'],order};
}
export function setContextBinding(world:World,binding:AiContextBinding):World {
  return {...world,aiContext:[...(world.aiContext??[]).filter(b=>b.nodeId!==binding.nodeId),binding]};
}
export function removeContextBinding(world:World,nodeId:string):World{return {...world,aiContext:(world.aiContext??[]).filter(b=>b.nodeId!==nodeId)};}
export function addContextNote(world:World,id=crypto.randomUUID()):{world:World;node:WorldNode}{
  const next=withBuiltinNodeType(world,'core.note');
  const node=makeNode(next,'core.note','New context note',id);
  return {world:setContextBinding({...next,nodes:[...next.nodes,node]},defaultContextBinding(id,(world.aiContext?.length??0)*10)),node};
}
export function addNarrationStyle(world:World,id=crypto.randomUUID()):{world:World;node:WorldNode}{
  const schema=withBuiltinNodeType(world,'core.narration-style');
  const node=makeNode(schema,'core.narration-style','Narration style',id);
  const next={...schema,nodes:[...schema.nodes,node]};
  return {world:setContextBinding(next,defaultContextBinding(id,(world.aiContext?.length??0)*10,next)),node};
}
