import type {LlmConnection} from './bindings';
export type LlmPreset='qwen'|'generic'|'unsloth-gguf';
export function applyLlmPreset(connection:LlmConnection,preset:LlmPreset):LlmConnection{
  if(preset==='unsloth-gguf')return {...connection,structuredOutput:'schema',thinking:'directOff',tokenParameter:'maxTokens'};
  return {...connection,...(preset==='qwen'?{temperature:0.7,topP:0.8,topK:20,minP:0,thinking:'chatTemplateOff' as const}:{temperature:0.8,topP:1,topK:null,minP:null,thinking:'provider' as const})};
}
