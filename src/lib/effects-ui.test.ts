import {describe,it,expect} from 'vitest';
import type {PassageEffects,PassageEffect} from './bindings';
import {emptyWorld,exampleWorld,makeNode} from './model';
import {parseEffects,evidenceIssue,passageQuotes,removalPlan,removeProposals,nodeDiff,proposedNodes,effectTitle} from './effects-ui';
const none=():PassageEffects=>({changes:[],notes:[],suggestChapterEnd:false});
describe('structured passage effects',()=>{
  it('keeps malformed shapes out of card controls without losing their raw text',()=>{
    for(const text of ['{unfinished',JSON.stringify({changes:[{kind:'putNode',node:{},evidence:'quote'}],notes:[],suggestChapterEnd:false}),JSON.stringify({...none(),notes:[{text:'x',evidence:'x',nodeIds:null}]}),JSON.stringify({...none(),changes:[{kind:'unsupported',evidence:'quote'}]})])expect(parseEffects(text)).toBeNull();
    const w=exampleWorld(),e={...none(),changes:[{kind:'putNode' as const,node:w.nodes[0],evidence:'Source quotation.'}]};expect(parseEffects(JSON.stringify(e))).toEqual(e);
  });
  it('rejects stitched quotes but preserves literal ellipses, unicode and whitespace',()=>{
    const prose='Mara said, “Wait... stay.”\nShe closed\t the door.';expect(evidenceIssue(prose,'“Wait... stay.”')).toBe('');expect(evidenceIssue(prose,'She closed the door.')).toBe('');expect(evidenceIssue(prose,'Mara said... closed the door.')).not.toBe('');expect(evidenceIssue(prose,'She opened the door.')).not.toBe('');
  });
  it('offers only continuous bounded source excerpts, including long Unicode passages',()=>{
    const prose='Mara waited. '+('🌲'.repeat(1300))+' Jon returned.';const choices=passageQuotes(prose);expect(choices.length).toBeGreaterThan(2);for(const quote of choices){expect(prose.includes(quote)).toBe(true);expect(evidenceIssue(prose,quote)).toBe('');}expect(choices.every(q=>[...q].length<=600)).toBe(true);
  });
  it('previews transitive new-node dependencies using typed references and preserves unrelated prose fields',()=>{
    const w=emptyWorld();w.componentTypes.push({id:'custom.parent',name:'Parent',description:'',fields:[{key:'parent',label:'Parent',kind:'nodeRef',required:true,defaultValue:null}]});
    const room=makeNode(w,'core.place','Room','new:room'),door=makeNode(w,'core.item','Door','new:door'),kept=makeNode(w,'core.item','Book','new:book');door.components['custom.parent']={parent:room.id};kept.description='The text new:room is not a typed reference.';
    const effects:PassageEffects={...none(),changes:[room,door,kept].map(node=>({kind:'putNode',node,evidence:'A complete source quote.'})),notes:[{text:'Door note',evidence:'A complete source quote.',nodeIds:[door.id]},{text:'Book note',evidence:'A complete source quote.',nodeIds:[kept.id]}]};
    effects.changes.push({kind:'putLink',link:{id:'new:link',typeId:'core.related',sourceId:door.id,targetId:room.id,description:''},evidence:'A complete source quote.'});
    const before=structuredClone(effects),plan=removalPlan(w,effects,'change',0);expect(plan).toEqual({changes:[0,1,3],notes:[0]});const next=removeProposals(effects,plan);expect(next.changes).toEqual([effects.changes[2]]);expect(next.notes).toEqual([effects.notes[1]]);expect(effects).toEqual(before);expect(w.nodes).toEqual([]);
  });
  it('removing an update leaves connections to the established node intact',()=>{
    const w=exampleWorld(),person=w.nodes.find(n=>n.components['core.character'])!;const effect:PassageEffect={kind:'putNode',node:{...person,description:'Updated note.'},evidence:'A complete source quote.'};const connection=w.edges.find(e=>e.sourceId===person.id)!;
    const effects:PassageEffects={...none(),changes:[effect,{kind:'putLink',link:connection,evidence:'A complete source quote.'}]};expect(removalPlan(w,effects,'change',0)).toEqual({changes:[0],notes:[]});expect(w.nodes.find(n=>n.id===person.id)?.description).toBe(person.description);
  });
  it('removing a proposed move also previews its paired old-placement removal',()=>{
    const w=exampleWorld(),old=w.edges.find(e=>e.typeId==='core.present-at')!,room=makeNode(w,'core.place','Study','new:study'),unrelated=w.edges.find(e=>e.typeId==='core.related')!;
    const effects:PassageEffects={...none(),changes:[{kind:'putNode',node:room,evidence:'Mara entered the study.'},{kind:'putLink',link:{...old,targetId:room.id},evidence:'Mara entered the study.'},{kind:'removeLink',id:old.id,evidence:'Mara entered the study.'},{kind:'removeLink',id:unrelated.id,evidence:'A separate source quote.'}]};
    expect(removalPlan(w,effects,'change',0)).toEqual({changes:[0,1,2],notes:[]});expect(removalPlan(w,effects,'change',1)).toEqual({changes:[1,2],notes:[]});
  });
  it('removes a note independently without altering other proposals',()=>{
    const effects:PassageEffects={...none(),notes:[{text:'One',evidence:'Source text one.',nodeIds:[]},{text:'Two',evidence:'Source text two.',nodeIds:[]}]};const next=removeProposals(effects,removalPlan(emptyWorld(),effects,'note',0));expect(next.notes.map(n=>n.text)).toEqual(['Two']);expect(effects.notes).toHaveLength(2);
  });
  it('shows human names and schema field labels in generic before/after comparisons',()=>{
    const w=exampleWorld(),coat=w.nodes.find(n=>n.components['core.wearable'])!,next=structuredClone(coat);next.components['core.wearable'].layer=3;next.visualDescription='A different color.';const person=w.nodes.find(n=>n.components['core.character'])!;w.componentTypes.push({id:'custom.refs',name:'Refs',description:'',fields:[{key:'literal',label:'Literal',kind:'text',required:false,defaultValue:''},{key:'target',label:'Target',kind:'nodeRef',required:false,defaultValue:null}]});coat.components['custom.refs']={literal:'old',target:null};next.components['custom.refs']={literal:person.id,target:person.id};const effects:PassageEffects={...none(),changes:[{kind:'putNode',node:next,evidence:'Source text.'}]};const nodes=proposedNodes(w,effects),diff=nodeDiff(w,nodes,next);expect(diff.find(r=>r.label==='Refs · Literal')?.after).toBe(person.id);expect(diff.find(r=>r.label==='Refs · Target')?.after).toBe(person.name);expect(diff.find(r=>r.label==='Wearable · Layer')).toEqual({label:'Wearable · Layer',before:'2',after:'3'});expect(effectTitle(w,effects,effects.changes[0])).toContain(coat.name);expect(w.nodes.find(n=>n.id===coat.id)?.components['core.wearable'].layer).toBe(2);expect(nodes.filter(n=>n.id===coat.id)).toHaveLength(1);
  });
});
