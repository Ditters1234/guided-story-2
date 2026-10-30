import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Preferences } from './bindings';
const api=vi.hoisted(()=>({load:vi.fn(),save:vi.fn()}));
vi.mock('./native',()=>({desktop:true,commands:{loadPreferences:api.load,savePreferences:api.save},unwrap:(result:{status:string;data?:Preferences;error?:string})=>{if(result.status!=='ok')throw Error(result.error);return result.data;}}));
import { AppearanceSettings } from './appearance.svelte';
import { defaultPreferences } from './appearance';
const ok=(data:Preferences)=>({status:'ok' as const,data});

describe('appearance save queue',()=>{
  beforeEach(()=>{
    vi.useFakeTimers(); api.load.mockReset().mockResolvedValue(ok(defaultPreferences()));
    api.save.mockReset().mockImplementation(async(version:number,prefs:Preferences)=>ok({...prefs,version:version+1}));
  });
  afterEach(()=>{vi.clearAllTimers();vi.useRealTimers();});
  it('preserves edits made while an earlier settings write is in flight',async()=>{
    const settings=new AppearanceSettings(); await settings.load();
    let finish!: (value:ReturnType<typeof ok>)=>void;
    api.save.mockImplementationOnce(()=>new Promise((resolve)=>{finish=resolve;}));
    settings.update({themeId:'dark'}); const saving=settings.flush();
    settings.update({textScale:125});
    finish(ok({...defaultPreferences(),themeId:'dark',version:1})); await saving;
    expect(api.save).toHaveBeenCalledTimes(2);
    expect(api.save.mock.calls[1][0]).toBe(1);
    expect(api.save.mock.calls[1][1]).toMatchObject({themeId:'dark',textScale:125});
    expect(settings.value).toMatchObject({version:2,themeId:'dark',textScale:125});
    expect(settings.dirty).toBe(false);
  });
  it('retries the exact uncertain operation before saving later edits',async()=>{
    const settings=new AppearanceSettings();await settings.load();
    api.save.mockRejectedValueOnce(Error('Transport unavailable'));
    settings.update({themeId:'sepia'});await expect(settings.flush()).rejects.toThrow();
    expect(settings.error).toContain('Transport unavailable');expect(settings.dirty).toBe(true);
    settings.update({textScale:140});await settings.flush();
    expect(api.save.mock.calls[0]).toEqual(api.save.mock.calls[1]);
    expect(api.save.mock.calls[2][0]).toBe(1);
    expect(settings.value).toMatchObject({themeId:'sepia',textScale:140,version:2});
    expect(settings.error).toBe('');
  });
  it('does not overwrite unreadable preferences with defaults',async()=>{
    api.load.mockRejectedValueOnce(Error('Corrupt preferences preserved'));
    const settings=new AppearanceSettings();await settings.load();
    settings.update({themeId:'dark'});await settings.flush();
    expect(settings.ready).toBe(false);expect(api.save).not.toHaveBeenCalled();
    expect(settings.error).toContain('preserved');
  });
});
