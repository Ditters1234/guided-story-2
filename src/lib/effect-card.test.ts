import {describe,it,expect} from 'vitest';
import {render} from 'svelte/server';
import EffectCard from './EffectCard.svelte';
import EffectsReviewEditor from './EffectsReviewEditor.svelte';
import SceneResources from './SceneResources.svelte';
import {emptyWorld,makeNode} from './model';
import type {PassageEffects,EffectReviewEntry} from './bindings';

describe('native relationship replacement review',()=>{
  it('renders every source excerpt for repair with bounded add/remove controls',()=>{
    const world=emptyWorld(),node=makeNode(world,'core.character','Mara');world.nodes=[node];
    const prose="Glass cut Mara's hand. She chose to reach the clinic.";
    const effects:PassageEffects={changes:[{kind:'putNode',node,evidence:["Glass cut Mara's hand.",'She arrived at the clinic.']}],notes:[],suggestChapterEnd:false};
    const props={world,effects,prose,kind:'change' as const,index:0,approvedKeys:[],onChange:()=>{},onApprove:()=>{}};
    const {body}=render(EffectCard,{props});
    expect(body).toContain('Evidence quote 1');expect(body).toContain('Evidence quote 2');expect(body).toContain('Quote 2:');expect(body).toContain('Add source quote');expect(body).toContain('Remove quote 2');
    const four:PassageEffects={...effects,changes:[{kind:'putNode',node,evidence:['Glass cut','cut Mara',"Mara's hand.",'reach the clinic.']}]};
    const full=render(EffectCard,{props:{...props,effects:four}}).body;
    expect(full).toContain('Evidence quote 4');expect(full).not.toContain('Add source quote');expect(full).toContain('Remove quote 4');
  });
  it('shows full long values and populated structured fields on proposed new nodes',()=>{
    const world=emptyWorld(),node=makeNode(world,'core.character','New arrival','new:arrival');
    const long='Visible condition. '.repeat(30)+'Exact final detail.';node.components['detail.condition']={condition:long};node.components['detail.intent']={objective:'Reach the clinic'};
    const effects:PassageEffects={changes:[{kind:'putNode',node,evidence:'A new arrival waited outside.'}],notes:[],suggestChapterEnd:false};
    const {body}=render(EffectCard,{props:{world,effects,prose:'A new arrival waited outside.',kind:'change',index:0,approvedKeys:[],onChange:()=>{},onApprove:()=>{}}});
    expect(body).toContain('Current condition');expect(body).toContain('Reach the clinic');expect(body).toContain('Exact final detail.');expect(body).toContain('effect-full-value');
  });
  it('shows note-only and untouched scene coverage without implying an empty result is complete',()=>{
    const world=emptyWorld(),person=makeNode(world,'core.character','Mara'),other=makeNode(world,'core.character','Jon'),outside=makeNode(world,'core.character','Outside scope');world.nodes=[person,other,outside];
    const effects:PassageEffects={changes:[],notes:[{text:'Mara waited.',evidence:'Mara waited.',nodeIds:[person.id]}],suggestChapterEnd:false};
    const {body}=render(EffectsReviewEditor,{props:{world,effects,scopeIds:[person.id,other.id],prose:'Mara waited.',review:undefined,approvedKeys:[],onChange:()=>{},onApprove:()=>{}}});
    expect(body).toContain('0 proposed changes');expect(body).toContain('No structured changes proposed');expect(body).toContain('Continuity note only');expect(body).toContain('No proposed update');expect(body).not.toContain('Outside scope');expect(body).toContain('Add missing change or note');
  });
  it('renders explicit reference suggestions and their coverage separately from captured resources',()=>{
    const world=emptyWorld(),room=makeNode(world,'core.place','Workshop'),item=makeNode(world,'core.item','Lantern');world.nodes=[room,item];world.edges=[{id:crypto.randomUUID(),typeId:'core.stored-in',sourceId:item.id,targetId:room.id,description:''}];
    const plan={chapterTitle:'Chapter 1',chapterPlan:'',title:'Passage 1',direction:'',viewpointId:null,locationId:room.id,participantIds:[],referenceIds:[],targetWords:450};
    const {body}=render(SceneResources,{props:{world,plan,updating:false,onToggle:()=>{}}});
    expect(body).toContain('Suggested references (1)');expect(body).toContain('Lantern');expect(body).toContain('not sent until added');expect(plan.referenceIds).toEqual([]);
  });
  it('shows the connection being updated and the separately displaced connection',()=>{
    const world=emptyWorld();
    const mara=makeNode(world,'core.character','Mara'),jon=makeNode(world,'core.character','Jon'),lena=makeNode(world,'core.character','Lena');
    world.nodes.push(mara,jon,lena);
    world.relationTypes.push({id:'custom.single',name:'Single connection',label:'follows',description:'',sourceCapability:null,targetCapability:null,physicalPlacement:false,singleTarget:true});
    const old={id:crypto.randomUUID(),typeId:'custom.single',sourceId:mara.id,targetId:lena.id,description:'Earlier connection'};
    const updated={id:crypto.randomUUID(),typeId:'core.related',sourceId:mara.id,targetId:jon.id,description:''};
    world.edges.push(old,updated);
    const effects:PassageEffects={changes:[{kind:'putLink',link:{...updated,typeId:old.typeId},evidence:'Mara waited beside Jon.'}],notes:[],suggestChapterEnd:false};
    const entry:EffectReviewEntry={kind:'change',index:0,key:'review',approved:false,issues:[],displacedLinks:[old]};
    const before=structuredClone(world);
    const {body}=render(EffectCard,{props:{world,effects,prose:'Mara waited beside Jon.',kind:'change',index:0,entry,approvedKeys:[],onChange:()=>{},onApprove:()=>{}}});
    expect(body).toContain('Before:');expect(body).toContain('After:');expect(body).toContain('Replaces:');expect(body).toContain('Lena');expect(body).toContain('will be removed.');
    expect(world).toEqual(before);
  });
});
