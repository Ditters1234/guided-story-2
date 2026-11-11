import { describe, expect, it } from 'vitest';
import { applyAppearance, builtInThemes, copyTheme, defaultPreferences, parseThemeJson, resolveTheme } from './appearance';

describe('portable appearance settings',()=>{
  it('follows the system only when requested and preserves explicit choices',()=>{
    const prefs=defaultPreferences();
    expect(resolveTheme(prefs,true).id).toBe('dark');
    expect(resolveTheme(prefs,false).id).toBe('light');
    expect(resolveTheme({...prefs,themeId:'sepia'},true).id).toBe('sepia');
  });
  it('roundtrips custom JSON and gives imported built-ins separate identities',()=>{
    const theme=copyTheme(builtInThemes[1]);
    theme.name='Night garden';
    expect(parseThemeJson(JSON.stringify(theme))).toEqual(theme);
    const imported=parseThemeJson(JSON.stringify(builtInThemes[1]));
    expect(imported.id).toMatch(/^custom\./);
    expect(imported.colors).toEqual(builtInThemes[1].colors);
    expect(builtInThemes[1].id).toBe('dark');
  });
  it('rejects incomplete themes, future schemas, arbitrary CSS and extra properties',()=>{
    const theme=copyTheme(builtInThemes[0]);
    for(const invalid of [
      {...theme,schemaVersion:2}, {...theme,name:''}, {...theme,css:'body{display:none}'},
      {...theme,colors:{...theme.colors,surface:'url(https://example.com)'}},
      {...theme,colors:{surface:'#ffffff'}}, {...theme,colors:{...theme.colors,extra:'#ffffff'}},
    ]) expect(()=>parseThemeJson(JSON.stringify(invalid))).toThrow();
    expect(()=>parseThemeJson(' '.repeat(65537))).toThrow(/64 KB/);
  });
  it('applies shared theme tokens, text size and motion without changing preferences',()=>{
    const prefs={...defaultPreferences(),themeId:'dark',textScale:130,reducedMotion:true};
    const before=structuredClone(prefs),values=new Map<string,string>();
    const root={style:{setProperty:(key:string,value:string)=>values.set(key,value),colorScheme:''},dataset:{}};
    applyAppearance(prefs,false,root as unknown as HTMLElement);
    expect(values.get('--canvas')).toBe(builtInThemes[1].colors.canvas);
    expect(values.get('--ink')).toBe(builtInThemes[1].colors.text);
    expect(values.get('--text-scale')).toBe('1.3');
    expect(root.dataset).toEqual({themeMode:'dark',reducedMotion:'true'});
    expect(prefs).toEqual(before);
  });
});
