import {describe,it,expect} from 'vitest';
import type {LlmConnection} from './bindings';
import {applyLlmPreset} from './llm-presets';
describe('LLM compatibility presets',()=>{
  it('changes only Unsloth transport controls while preserving current prompts, identity, sampling and budgets',()=>{
    const source:LlmConnection={schemaVersion:1,version:4,provider:'compatible',baseUrl:'http://127.0.0.1:8888/v1',model:'fictional-local',temperature:0.6,topP:0.9,topK:30,minP:0.02,thinking:'provider',maxTokens:4096,contextTokens:24000,timeoutSeconds:300,structuredOutput:'json',tokenParameter:'maxCompletionTokens',credentialRef:'fictional-key-reference',prompts:{version:7,overrides:{world:'Keep my custom instructions.'}}};const before=structuredClone(source),next=applyLlmPreset(source,'unsloth-gguf');
    expect(next).toEqual({...source,structuredOutput:'schema',thinking:'directOff',tokenParameter:'maxTokens'});expect(source).toEqual(before);expect(applyLlmPreset(next,'qwen').structuredOutput).toBe('schema');expect(applyLlmPreset(next,'generic').prompts).toEqual(source.prompts);
  });
});
