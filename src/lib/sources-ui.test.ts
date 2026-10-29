import {describe,it,expect} from 'vitest';
import {emptyWorld,makeNode,withBuiltinNodeType,availableNodeTypes} from './model';
import {mergeNativeSources} from './sources-ui';
import type {SourceAnnotation_Serialize as SourceAnnotation} from './bindings';
function fixture(){const world=emptyWorld(),node=makeNode(world,'core.event','Pump failure');world.nodes=[node];const annotation:SourceAnnotation={id:node.id,nodeId:node.id,subjectFingerprint:'0'.repeat(64),origin:{kind:'manual',note:'Author-established backstory'}};return {world,node,annotation};}
describe('source metadata and optional Event presets',()=>{
  it('merges native-added provenance without overwriting typing made during an autosave',()=>{
    const {world,node,annotation}=fixture(),sent=structuredClone(world),local={...world,nodes:[{...node,description:'Newer unsaved wording'}]},saved={...world,sourceAnnotations:[annotation]};
    const merged=mergeNativeSources(local,sent,saved);expect(merged.nodes[0].description).toBe('Newer unsaved wording');expect(merged.sourceAnnotations).toEqual([annotation]);expect(local.sourceAnnotations).toBeUndefined();
  });
  it('does not resurrect removed nodes or citations and retains later source edits',()=>{
    const {world,node,annotation}=fixture(),sent={...world,sourceAnnotations:[annotation]},saved=structuredClone(sent),removed={...world,nodes:[],sourceAnnotations:[]};
    expect(mergeNativeSources(removed,sent,saved)).toBe(removed);
    const local={...world,sourceAnnotations:[{...annotation,origin:{kind:'manual' as const,note:'A newer author note'}}]};expect(mergeNativeSources(local,world,saved).sourceAnnotations?.[0].origin).toEqual({kind:'manual',note:'A newer author note'});
    expect(mergeNativeSources({...world,nodes:[node],sourceAnnotations:[]},sent,saved).sourceAnnotations).toEqual([]);
  });
  it('installs the optional Event bundle explicitly and leaves earlier worlds and unrelated definitions intact',()=>{
    const before=emptyWorld();before.nodeTypes=before.nodeTypes.filter(t=>t.id!=='core.event');before.componentTypes=before.componentTypes.filter(c=>c.id!=='detail.event');before.relationTypes=before.relationTypes.filter(r=>r.sourceCapability!=='detail.event');
    const snapshot=structuredClone(before);expect(availableNodeTypes(before).some(t=>t.id==='core.event')).toBe(true);const after=withBuiltinNodeType(before,'core.event');const event=makeNode(after,'core.event','Old mill fire');
    expect(event.components['detail.event']).toEqual({});expect(before).toEqual(snapshot);expect(after.relationTypes).toEqual(before.relationTypes);expect(after.nodes).toEqual([]);
  });
});
