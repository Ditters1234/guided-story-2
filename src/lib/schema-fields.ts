import type {FieldDefinition_Serialize,FieldKind,FieldUse} from './bindings';
export type FieldDraft={key:string;label:string;kind:FieldKind;help:string;uses:FieldUse[];stability:'stable'|'state';choices:string;referenceCapability:string;coveredBy:string};
export function fieldDefinitions(fields:FieldDraft[]):FieldDefinition_Serialize[]{
 return fields.map(f=>({key:f.key,label:f.label.trim(),kind:f.kind,required:false,defaultValue:null,metadata:{help:f.help.trim()||undefined,uses:[...f.uses],stability:f.stability,choices:f.kind==='choice'?f.choices.split(',').map(v=>v.trim()).filter(Boolean):[],referenceCapability:['nodeRef','nodeRefs'].includes(f.kind)?f.referenceCapability||undefined:undefined,coveredBy:f.uses.includes('visual')?f.coveredBy.split(',').map(v=>v.trim()).filter(Boolean):[]}}));
}
