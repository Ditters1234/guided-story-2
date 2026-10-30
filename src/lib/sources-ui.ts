import type {World} from './model';

// The native save may add author provenance to a new Event. Preserve later typing,
// explicit source edits and deletions while merging only those native additions.
export function mergeNativeSources(local:World,sent:World,saved:World):World {
  const prior=new Set((sent.sourceAnnotations??[]).map(a=>a.id));
  const present=new Set((local.sourceAnnotations??[]).map(a=>a.id));
  const additions=(saved.sourceAnnotations??[]).filter(a=>!prior.has(a.id)&&!present.has(a.id)&&local.nodes.some(n=>n.id===a.nodeId));
  return additions.length?{...local,sourceAnnotations:[...(local.sourceAnnotations??[]),...additions]}:local;
}
