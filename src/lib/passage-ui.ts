import type {PassageEffects,PassageStage,PassageSummary,PassageView_Serialize} from './bindings';

// Presentation of existing native review state; acceptance remains native-owned.
export function passageReviewState(view:PassageView_Serialize|null,effects:PassageEffects|null,hasProse:boolean,unsaved:boolean,approvedKeys:string[]=[]){
  const hasReview=!!view?.originalEffects||(hasProse&&effects===null)||!!effects?.changes.length||!!effects?.notes.length||view?.stage==='ready';
  const checked=!!view?.effectsReview.checked&&!unsaved;
  const pending=view?.effectsReview.entries.filter(e=>!approvedKeys.includes(e.key)).length??0;
  const wording=view?.revision?.kind==='wording';
  const action=!hasProse?'generate':view?.canAccept&&!unsaved?'accept':wording?'validate':!hasReview?'extract':!checked?'validate':'review';
  return {hasReview,checked,pending,action} as const;
}

export function passageStageLabel(stage:PassageStage){
  const labels:Record<PassageStage,string>={draft:'Draft',narrating:'Writing…',needsCheck:'Needs checking',checking:'Checking changes…',ready:'Proposals validated',failed:'Needs attention',interrupted:'Interrupted',cancelled:'Cancelled',accepted:'Accepted',discarded:'Discarded'};
  return labels[stage];
}

export function groupPassageDrafts(drafts:PassageSummary[]){
  const byId=new Map(drafts.map(d=>[d.id,d]));
  function root(draft:PassageSummary){
    const visited=new Set<string>();
    let current=draft;
    while(current.sourceDraftId&&!visited.has(current.id)){
      visited.add(current.id);
      const source=byId.get(current.sourceDraftId);
      if(!source)return current.sourceDraftId;
      current=source;
    }
    return current.id;
  }
  const groups=new Map<string,{id:string;title:string;drafts:PassageSummary[]}>();
  for(const draft of drafts){
    const id=draft.targetPassageId?`passage:${draft.targetPassageId}`:`draft:${root(draft)}`;
    const group=groups.get(id)??{id,title:byId.get(root(draft))?.title||draft.title||'Untitled passage',drafts:[]};
    group.drafts.push(draft);groups.set(id,group);
  }
  return [...groups.values()].map(group=>({...group,drafts:[...group.drafts].sort((a,b)=>a.createdAt.localeCompare(b.createdAt,undefined,{numeric:true})||a.id.localeCompare(b.id)).map((draft,index)=>({
    ...draft,label:`${draft.revisionKind==='wording'?'Wording':draft.revisionKind==='events'?'Event replacement':draft.revisionDirection?'Revision':draft.sourceDraftId?'Alternative':'Draft'} ${index+1}`,
  }))}));
}
