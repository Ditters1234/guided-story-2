import type {AiContextBinding,TextPromptTask,WorldNode} from './bindings';
import {makeNode,withBuiltinNodeType,type World} from './model';
export const aiContextTasks:{id:TextPromptTask;label:string}[]=[{id:'world',label:'World creation / rerolls'},{id:'passage',label:'Passage writing'},{id:'wording',label:'Wording edits'},{id:'effects',label:'World changes'},{id:'memory',label:'Chapter memory'},{id:'imagePrompt',label:'Image-prompt assistance'}];
export function defaultContextBinding(nodeId:string,order=0,world?:World):AiContextBinding{
  const node=world?.nodes.find(n=>n.id===nodeId),narration=!!node?.components['core.narration-style'],visual=!!node?.components['detail.visual-style'];
  const tasks:TextPromptTask[]=narration?['passage','wording']:visual?[]:aiContextTasks.map(t=>t.id);
  if(visual)tasks.push('imagePrompt');
  return {nodeId,enabled:true,role:narration||visual?'instructions':'reference',tasks,fields:['description','components'],order,...(visual?{imagePreparation:true}:{})};
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

export function addVisualStyle(world:World,id=crypto.randomUUID()):{world:World;node:WorldNode}{
  const schema=withBuiltinNodeType(world,'core.visual-style'),node=makeNode(schema,'core.visual-style','Visual style',id),next={...schema,nodes:[...schema.nodes,node]};
  return {world:setContextBinding(next,defaultContextBinding(id,(world.aiContext?.length??0)*10,next)),node};
}
