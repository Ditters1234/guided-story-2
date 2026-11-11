import type { Preferences, ThemeColors, ThemeDefinition } from './bindings';
import presets from '../../schema/themes.json';

export const builtInThemes = presets as ThemeDefinition[];
export const colorKeys = Object.keys(builtInThemes[0].colors) as (keyof ThemeColors)[];
export const defaultPreferences = (): Preferences => ({schemaVersion:1,version:0,themeId:'system',textScale:100,reducedMotion:false,customThemes:[]});
export function resolveTheme(preferences: Preferences, systemDark: boolean): ThemeDefinition {
  const id = preferences.themeId === 'system' ? systemDark ? 'dark' : 'light' : preferences.themeId;
  return [...builtInThemes, ...preferences.customThemes].find((t)=>t.id===id) ?? builtInThemes[0];
}
export function copyTheme(theme: ThemeDefinition): ThemeDefinition {
  return {...structuredClone(theme),id:'custom.'+crypto.randomUUID(),name:'My theme'};
}

const isObject = (value: unknown): value is Record<string,unknown> => !!value && typeof value==='object' && !Array.isArray(value);
export function parseThemeJson(json: string): ThemeDefinition {
  if (json.length > 64*1024) throw new Error('Theme files must be smaller than 64 KB.');
  const value: unknown = JSON.parse(json);
  if (!isObject(value) || value.schemaVersion!==1 || typeof value.name!=='string' || !value.name.trim() || value.name.length>60 ||
      !['light','dark'].includes(String(value.mode)) || typeof value.id!=='string' || !isObject(value.colors)) throw new Error('Use a version 1 theme with a name, ID, mode and colors.');
  const colors=value.colors;
  if (Object.keys(value).some((key)=>!['schemaVersion','id','name','mode','colors'].includes(key)) ||
      Object.keys(colors).length!==colorKeys.length || colorKeys.some((key)=>typeof colors[key]!=='string' || !/^#[0-9a-f]{6}$/i.test(String(colors[key])))) throw new Error('Include all theme color tokens using #RRGGBB values. CSS and URLs are not supported.');
  const builtin = builtInThemes.some((t)=>t.id===value.id);
  if (!builtin && !/^custom\.[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value.id)) throw new Error('Custom theme IDs must use custom. followed by a UUID.');
  return {...value,id:builtin?'custom.'+crypto.randomUUID():value.id,name:value.name.trim()} as ThemeDefinition;
}

export function applyAppearance(preferences: Preferences, systemDark: boolean, root = document.documentElement) {
  const theme=resolveTheme(preferences,systemDark);
  const tokens: Record<keyof ThemeColors,string> = {background:'background',surface:'surface',surfaceAlt:'surface-alt',text:'ink',muted:'muted',border:'border',accent:'accent',accentText:'accent-text',selection:'selection',canvas:'canvas',canvasDots:'canvas-dots',connection:'connection'};
  for (const key of colorKeys) root.style.setProperty('--'+tokens[key],theme.colors[key]);
  root.style.setProperty('--text-scale',String(preferences.textScale/100));
  root.style.colorScheme=theme.mode;
  root.dataset.themeMode=theme.mode;
  root.dataset.reducedMotion=String(preferences.reducedMotion);
}

export function downloadTheme(theme: ThemeDefinition) {
  const url=URL.createObjectURL(new Blob([JSON.stringify(theme,null,2)+'\n'],{type:'application/json'}));
  const link=document.createElement('a'); link.href=url; link.download=theme.name.toLowerCase().replace(/[^a-z0-9]+/g,'-')+'.theme.json'; link.click();
  setTimeout(()=>URL.revokeObjectURL(url),1000);
}
