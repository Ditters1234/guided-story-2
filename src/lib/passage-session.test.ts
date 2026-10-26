import {afterEach,beforeEach,describe,expect,it,vi} from 'vitest';
import type {PassageEdits_Serialize as PassageEdits,PassageView_Serialize} from './bindings';
const api=vi.hoisted(()=>({get:vi.fn(),save:vi.fn(),clone:vi.fn()}));
vi.mock('./native',()=>({commands:{getPassageDraft:api.get,savePassageDraft:api.save,clonePassageDraft:api.clone},unwrap:(r:{status:string;data?:unknown;error?:string})=>{if(r.status!=='ok')throw Error(r.error);return r.data;}}));
import {PassageSession} from './passage-session.svelte';
const edits=():PassageEdits=>({plan:{chapterTitle:'Chapter 1',chapterPlan:'',title:'At the window',direction:'Wait for the rain.',viewpointId:null,locationId:null,participantIds:[],referenceIds:[],targetWords:450},prose:'First sentence.',effectsText:'{"changes":[],"notes":[],"suggestChapterEnd":false}',closeChapter:false});
const view=(value=edits(),version=1):PassageView_Serialize=>({memory:{points:[],coverage:{olderPassages:0,closedChapters:0,currentSummaries:0,missingSummaries:0,staleSummaries:0,unavailableSummaries:0,unrepresentedPassages:0,availablePoints:0,selectedPoints:0,omittedPoints:0,byteBudget:6000}},revision:null,id:'10000000-0000-4000-8000-000000000001',version,stage:'ready',edits:value,error:'',stale:false,changes:[],canAccept:true,suggestedChapterEnd:false,contextNodes:[],recentPassages:[],originalProse:'',originalEffects:'',narrationProvider:'',effectsProvider:''});
const ok=(data:PassageView_Serialize)=>({status:'ok',data});
describe('passage draft queue',()=>{
  beforeEach(()=>{vi.useFakeTimers();api.clone.mockReset();api.get.mockReset().mockResolvedValue(ok(view()));api.save.mockReset().mockImplementation(async(_story:string,_id:string,version:number,value:PassageEdits)=>ok(view(value,version+1)));});
  afterEach(()=>{vi.clearAllTimers();vi.useRealTimers();});
  it('saves prose typed during a previous write without replacing it with the earlier response',async()=>{
    const session=new PassageSession('story');await session.open(view().id);
    let finish!:(v:ReturnType<typeof ok>)=>void;api.save.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
    session.value!.prose='Earlier edit.';session.touch();expect(session.readyToAccept).toBe(false);const saving=session.flush();
    session.value!.prose='Later edit survives.';session.touch();finish(ok(view({...edits(),prose:'Earlier edit.'},2)));await saving;
    expect(api.save).toHaveBeenCalledTimes(2);expect(api.save.mock.calls[1][2]).toBe(2);expect(api.save.mock.calls[1][3].prose).toBe('Later edit survives.');expect(session.value!.prose).toBe('Later edit survives.');expect(session.dirty).toBe(false);
  });
  it('retries the exact uncertain save before advancing later typing',async()=>{
    const session=new PassageSession('story');await session.open(view().id);api.save.mockRejectedValueOnce(Error('Uncertain transport outcome'));
    session.value!.prose='Saved draft candidate.';session.touch();await expect(session.flush()).rejects.toThrow('Uncertain');
    session.value!.prose='A newer candidate.';session.touch();await session.flush();expect(api.save.mock.calls[0]).toEqual(api.save.mock.calls[1]);expect(api.save.mock.calls[2][2]).toBe(2);expect(session.value!.prose).toBe('A newer candidate.');expect(session.error).toBe('');
  });
  it('does not overwrite new typing when a refresh started before that edit',async()=>{
    const session=new PassageSession('story');await session.open(view().id);let finish!:(v:ReturnType<typeof ok>)=>void;api.get.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
    const refreshing=session.refresh();await Promise.resolve();await Promise.resolve();session.value!.prose='Typing while the source refreshes.';session.touch();finish(ok(view()));await refreshing;
    expect(session.value!.prose).toBe('Typing while the source refreshes.');expect(session.dirty).toBe(true);await session.flush();
  });
  it('saves the current candidate before rerolling and keeps the native revision purpose',async()=>{
    const session=new PassageSession('story');await session.open(view().id);
    const candidate={...view({...edits(),prose:''}),id:'10000000-0000-4000-8000-000000000002',revision:{kind:'wording' as const,passageId:'passage',source:{id:'source',sha256:'abc'},prose:'Original text.',world:{schemaVersion:1,nodes:[],edges:[],nodeTypes:[],componentTypes:[],relationTypes:[]}}};
    api.clone.mockResolvedValue(ok(candidate));session.value!.prose='Manual candidate retained before reroll.';session.touch();await session.reroll();
    expect(api.save.mock.calls[0][3].prose).toBe('Manual candidate retained before reroll.');expect(api.clone.mock.calls[0].slice(0,3)).toEqual(['story',view().id,2]);expect(session.view?.revision?.kind).toBe('wording');expect(session.value!.prose).toBe('');
  });
});
