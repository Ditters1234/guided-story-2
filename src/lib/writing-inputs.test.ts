import {describe,expect,it} from 'vitest';
import {emptyWorld,makeNode,withBuiltinNodeType} from './model';
import {capabilitySections} from './node-fields';
import {briefDraftFields,needsWritingFields,withWritingFields,selectWritingSource,passageNodeChoice,type DraftWorldContext} from './writing-inputs';
import type {PassagePlan} from './bindings';

const plan=():PassagePlan=>({chapterTitle:'Chapter 1',chapterPlan:'Old chapter direction',chapterProgress:'Old recap',title:'Draft',direction:'Old direction',stopAfter:'Old stop',viewpointId:'mara',locationId:'room',participantIds:['jon'],referenceIds:['lantern']});
describe('World writing inputs',()=>{
  it('shows chapter and passage direction on new briefs in fresh and older worlds',()=>{
    const fresh=emptyWorld();
    const legacy=structuredClone(fresh);
    const definition=legacy.componentTypes.find(d=>d.id==='detail.brief')!;
    definition.fields=definition.fields.filter(f=>['premise','genre','themes','aims'].includes(f.key));
    const oldBrief=makeNode(legacy,'core.brief','Saved brief');
    oldBrief.components['detail.brief'].premise='Repair a broken latch.';
    legacy.nodes.push(oldBrief);
    const snapshot=structuredClone(legacy);

    const upgraded=withBuiltinNodeType(legacy,'core.brief');
    for(const world of [fresh,upgraded]){
      const brief=makeNode(world,'core.brief','New brief');
      const fields=capabilitySections(world,brief,'story')[0].definition.fields;
      expect(fields.map(f=>f.key)).toEqual(['premise','genre','themes','aims','chapterDirection','passageDirection','stopAfter','alreadyCovered']);
    }
    expect(legacy).toEqual(snapshot);
    expect(upgraded.nodes).toEqual(snapshot.nodes);
    expect(needsWritingFields(legacy,oldBrief.id)).toBe(true);
    expect(withWritingFields(legacy,oldBrief.id)).toEqual(upgraded);
    expect(needsWritingFields(upgraded,oldBrief.id)).toBe(false);
  });
  it('explicitly extends existing definitions without changing node values, bindings or saved source worlds',()=>{
    const world=emptyWorld(),style=makeNode(world,'core.narration-style','Narration');world.nodes.push(style);
    world.componentTypes.find(d=>d.id==='core.narration-style')!.fields=world.componentTypes.find(d=>d.id==='core.narration-style')!.fields.filter(f=>!['viewpointId','knowledgePerspective'].includes(f.key));
    const before=structuredClone(world);expect(needsWritingFields(world,style.id)).toBe(true);
    const next=withWritingFields(world,style.id);expect(needsWritingFields(next,style.id)).toBe(false);
    expect(world).toEqual(before);expect(next.nodes).toEqual(before.nodes);expect(next.aiContext).toEqual(before.aiContext);expect(next.edges).toEqual(before.edges);
    expect(withWritingFields(next,style.id)).toEqual(next);
    expect(next.relationTypes.find(d=>d.id==='core.narration-pov')).toMatchObject({sourceCapability:'core.narration-style',targetCapability:'core.character',singleTarget:true,physicalPlacement:false});
    expect(next.relationTypes.find(d=>d.id==='core.narration-focus')).toMatchObject({targetCapability:'core.place',singleTarget:true,physicalPlacement:false});
    expect(next.componentTypes.find(d=>d.id==='core.narration-style')!.fields.some(f=>f.key==='viewpointId')).toBe(false);
  });
  it('preserves draft direction across brief selections and restores it after native resolution',()=>{
    const initial=plan();initial.consumedBriefPassage='previous';
    const brief=selectWritingSource(initial,'brief','brief');expect(brief).toMatchObject({briefNodeId:'brief',direction:initial.direction,chapterPlan:initial.chapterPlan});
    expect(brief.consumedBriefPassage).toBeUndefined();expect(brief.participantIds).toEqual(initial.participantIds);expect(brief.referenceIds).toEqual(initial.referenceIds);
    const narration=selectWritingSource(initial,'narration','style');expect(narration.viewpointId).toBeNull();expect(narration.narrationNodeId).toBe('style');expect(initial.viewpointId).toBe('mara');
    const savedDraftDirection={chapterPlan:initial.chapterPlan,chapterProgress:'Old recap',direction:initial.direction,stopAfter:'Old stop'};
    const detached=selectWritingSource({...brief,direction:'From the brief',savedDraftDirection},'brief',null);
    expect(detached).toMatchObject({...savedDraftDirection,briefNodeId:null});
    expect(detached.savedDraftDirection).toBeUndefined();
    const world=emptyWorld(),node=makeNode(world,'core.brief','Brief');world.nodes.push(node);
    expect(briefDraftFields(world,node.id).size).toBe(4);
    Object.assign(node.components['detail.brief'],{chapterDirection:'arc',alreadyCovered:'recap',passageDirection:'beat',stopAfter:'stop'});
    expect(briefDraftFields(world,node.id).size).toBe(0);
  });
  it('distinguishes explicit selections from required scene resources and story-wide guidance',()=>{
    const current:DraftWorldContext={id:'draft',title:'Draft',plan:plan(),disabled:false,context:{error:'',resources:[{nodeId:'coat',name:'Coat',role:'inventory'}],guidance:[{nodeId:'rules',name:'Rules',role:'instructions',order:0,fields:['description'],content:'Keep the setting grounded.'}]}};
    expect(passageNodeChoice(current,'lantern')).toMatchObject({checked:true,locked:false});
    expect(passageNodeChoice(current,'coat')).toMatchObject({checked:true,locked:true,label:'Cast inventory'});
    expect(passageNodeChoice(current,'mara')).toMatchObject({checked:true,locked:true,label:'Not sent'});
    expect(passageNodeChoice(current,'rules')).toMatchObject({checked:true,locked:true,label:'Story-wide'});
    expect(passageNodeChoice(current,'other')).toMatchObject({checked:false,locked:false});
    expect(passageNodeChoice(null,'other')).toMatchObject({checked:false,disabled:true});
    expect(passageNodeChoice(current,'lantern')).toMatchObject({checked:true,included:false,label:'Not sent',warning:true});
    expect(passageNodeChoice({...current,updating:true},'lantern')).toMatchObject({disabled:true,label:'Updating…'});
    expect(current.plan.referenceIds).toEqual(['lantern']);
  });
});

it('shows persistent passage bindings before a draft is open without implying other tools are passage context',()=>{
  const world=emptyWorld();world.aiContext=[{nodeId:'rules',enabled:true,role:'instructions',tasks:['passage'],fields:['description'],order:0},{nodeId:'visual',enabled:true,role:'instructions',tasks:['imagePrompt'],fields:['components'],order:0}];
  expect(passageNodeChoice(null,'rules',world)).toMatchObject({checked:true,locked:true,disabled:true,label:'Story-wide',included:false});
  expect(passageNodeChoice(null,'visual',world)).toMatchObject({checked:false,locked:false});
});
