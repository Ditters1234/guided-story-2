import {afterEach,beforeEach,describe,expect,it,vi} from 'vitest';
import type {MemoryEdits,MemoryView} from './bindings';
const api=vi.hoisted(()=>({get:vi.fn(),save:vi.fn()}));
vi.mock('./native',()=>({commands:{getMemoryDraft:api.get,saveMemoryDraft:api.save},unwrap:(r:{status:string;data?:unknown;error?:string})=>{if(r.status!=='ok')throw Error(r.error);return r.data;}}));
import {MemorySession} from './memory-session.svelte';
const point=()=>({text:'Mara recovered the key.',passageId:'passage',evidence:'Mara discovered the key beneath the window.',nodeIds:[]});
const edits=():MemoryEdits=>({direction:'',output:{points:[point()],noLastingFacts:false},rawJson:''});
const view=(value=edits(),version=1):MemoryView=>({id:'memory',chapterId:'chapter',title:'Chapter 1',version,sources:[],nodes:[],stage:'review',edits:value,originalOutput:'',provider:'',error:'',stale:false,canApply:true,seededFromStale:false});
const ok=(data:MemoryView)=>({status:'ok',data});
describe('memory draft queue',()=>{
  beforeEach(()=>{vi.useFakeTimers();api.get.mockReset().mockResolvedValue(ok(view()));api.save.mockReset().mockImplementation(async(_story:string,_id:string,version:number,value:MemoryEdits)=>ok(view(value,version+1)));});
  afterEach(()=>{vi.clearAllTimers();vi.useRealTimers();});
  it('keeps nested point edits and additions made while an earlier save is pending',async()=>{
    const session=new MemorySession('story');await session.open('memory');let finish!:(v:ReturnType<typeof ok>)=>void;api.save.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
    session.value!.output.points[0].text='Earlier wording.';session.touch();expect(session.readyToApply).toBe(false);const saving=session.flush();
    session.value!.output.points[0].text='Newer wording survives.';session.value!.output.points.push({...point(),text:'A second point.'});session.touch();finish(ok(view({...edits(),output:{points:[{...point(),text:'Earlier wording.'}],noLastingFacts:false}},2)));await saving;
    expect(api.save).toHaveBeenCalledTimes(2);expect(api.save.mock.calls[0][3].output.points).toHaveLength(1);expect(api.save.mock.calls[1][3].output.points).toHaveLength(2);expect(session.value!.output.points[0].text).toBe('Newer wording survives.');expect(session.dirty).toBe(false);
  });
  it('retries an uncertain saved snapshot exactly before advancing newer edits',async()=>{
    const session=new MemorySession('story');await session.open('memory');api.save.mockRejectedValueOnce(Error('Uncertain outcome'));
    session.value!.output.points[0].text='First snapshot.';session.touch();await expect(session.flush()).rejects.toThrow('Uncertain');session.value!.output.points[0].text='Second snapshot.';session.touch();await session.flush();
    expect(api.save.mock.calls[0]).toEqual(api.save.mock.calls[1]);expect(api.save.mock.calls[2][2]).toBe(2);expect(session.value!.output.points[0].text).toBe('Second snapshot.');expect(session.error).toBe('');
  });
  it('does not replace new typing with a late source-refresh response',async()=>{
    const session=new MemorySession('story');await session.open('memory');let finish!:(v:ReturnType<typeof ok>)=>void;api.get.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
    const refreshing=session.refresh();await Promise.resolve();await Promise.resolve();session.value!.output.points[0].evidence='A quote being edited now.';session.touch();finish(ok(view()));await refreshing;
    expect(session.value!.output.points[0].evidence).toBe('A quote being edited now.');expect(session.dirty).toBe(true);await session.flush();
  });
});
