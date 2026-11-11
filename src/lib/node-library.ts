import type { LibraryEntry_Serialize, LibraryFile } from './bindings';
import type { Workspace } from './model';
import { buildGraph } from './graph';

export const libraryDragType = 'application/x-guided-story-library';
export function libraryDragData(file: LibraryFile): string {
  return JSON.stringify({fileName:file.fileName,sha256:file.sha256});
}
export function readLibraryDrag(value: string): {fileName:string;sha256:string} | null {
  try {
    const data:unknown=JSON.parse(value);
    if (!data || typeof data!=='object' || !('fileName' in data) || !('sha256' in data) ||
      typeof data.fileName!=='string' || typeof data.sha256!=='string' || !/^[a-f0-9]{64}$/.test(data.sha256)) return null;
    return {fileName:data.fileName,sha256:data.sha256};
  } catch { return null; }
}
export function libraryPositions(entry: LibraryEntry_Serialize, at:{x:number;y:number}): Workspace['positions'] {
  const workspace:Workspace={version:0,selectedId:null,expandedIds:entry.world.nodes.map(node=>node.id),positions:entry.positions??{}};
  const nodes=buildGraph(entry.world,workspace,'',()=>{}).nodes;
  const left=Math.min(...nodes.map(node=>node.position.x)),top=Math.min(...nodes.map(node=>node.position.y));
  return Object.fromEntries(nodes.map(node=>[node.id,{x:node.position.x-left+at.x,y:node.position.y-top+at.y}]));
}
