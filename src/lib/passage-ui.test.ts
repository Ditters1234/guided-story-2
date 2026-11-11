import {describe,expect,it} from 'vitest';
import {groupPassageDrafts,passageReviewState} from './passage-ui';
import type {PassageEffects,PassageSummary,PassageView_Serialize} from './bindings';

const empty:PassageEffects={changes:[],notes:[],suggestChapterEnd:false};
function view(overrides:Partial<PassageView_Serialize>={}){
  return {stage:'needsCheck',originalEffects:'',canAccept:false,effectsReview:{checked:false,entries:[]},...overrides} as PassageView_Serialize;
}
describe('passage controls use native review state',()=>{
  it('distinguishes unchecked prose from an extracted or deliberately validated empty result',()=>{
    expect(passageReviewState(view(),empty,false,false).action).toBe('generate');
    expect(passageReviewState(view(),empty,true,false)).toMatchObject({hasReview:false,action:'extract'});
    expect(passageReviewState(view({originalEffects:JSON.stringify(empty)}),empty,true,false)).toMatchObject({hasReview:true,action:'validate'});
    expect(passageReviewState(view({stage:'ready',canAccept:true,effectsReview:{checked:true,entries:[]}}),empty,true,false)).toMatchObject({hasReview:true,checked:true,action:'accept'});
  });
  it('keeps failed extraction distinct from a successful zero-change result and retains repair access',()=>{
    expect(passageReviewState(view({stage:'failed',error:'Provider failed'}),empty,true,false)).toMatchObject({hasReview:false,action:'extract'});
    expect(passageReviewState(view({originalEffects:'Invalid retained response'}),null,true,false)).toMatchObject({hasReview:true,checked:false,action:'validate'});
    expect(passageReviewState(view(),null,true,false).hasReview).toBe(true);
  });
  it('makes approval review the next action and never enables acceptance from local approvals alone',()=>{
    const entries=[{kind:'note' as const,index:0,key:'one',approved:false,issues:[],displacedLinks:[]}];
    const checked=view({stage:'ready',effectsReview:{checked:true,entries}});
    expect(passageReviewState(checked,empty,true,false)).toMatchObject({pending:1,action:'review'});
    expect(passageReviewState(checked,empty,true,false,['one']).action).toBe('review');
    expect(passageReviewState({...checked,canAccept:true},empty,true,true,['one'])).toMatchObject({checked:false,action:'validate'});
    expect(passageReviewState({...checked,canAccept:true},empty,true,false,['one']).action).toBe('accept');
  });
  it('keeps wording validation separate from AI extraction',()=>{
    expect(passageReviewState(view({revision:{kind:'wording'} as PassageView_Serialize['revision']}),empty,true,false).action).toBe('validate');
  });
});

const draft=(id:string,extra:Partial<PassageSummary>={}):PassageSummary=>({id,title:'At the window',stage:'needsCheck',version:1,createdAt:id,revisionKind:null,...extra});
describe('draft list presentation',()=>{
  it('groups existing source chains and labels alternatives and revisions without grouping unrelated equal titles',()=>{
    const drafts=[draft('30',{sourceDraftId:'20',revisionDirection:'Shorten the opening'}),draft('20',{sourceDraftId:'10'}),draft('10'),draft('40')];
    const before=structuredClone(drafts),groups=groupPassageDrafts(drafts);
    expect(groups).toHaveLength(2);
    expect(groups[0].drafts.map(d=>d.label)).toEqual(['Draft 1','Alternative 2','Revision 3']);
    expect(groups[0].drafts[2].revisionDirection).toBe('Shorten the opening');
    expect(drafts).toEqual(before);
  });
  it('keeps siblings together after deleting their source, and separates accepted-passage targets',()=>{
    const groups=groupPassageDrafts([draft('20',{sourceDraftId:'deleted'}),draft('30',{sourceDraftId:'deleted'}),draft('40',{targetPassageId:'accepted-one',revisionKind:'wording'}),draft('50',{targetPassageId:'accepted-two',revisionKind:'events'})]);
    expect(groups).toHaveLength(3);expect(groups[0].drafts).toHaveLength(2);
    expect(groups[1].drafts[0].label).toBe('Wording 1');expect(groups[2].drafts[0].label).toBe('Event replacement 1');
  });
});
