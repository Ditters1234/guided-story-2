import {describe,it,expect} from 'vitest';
import type {WorldEdge,PassageEffects} from './bindings';
import {emptyWorld,makeNode} from './model';
import {changeLinkType,setLinkProperty,linkFields,linkReferences} from './link-fields';
import {newLinkAuthoringRequest,editableLinkFields,parseCandidateForEditor} from './authoring-ui';
import {linkDiff,parseEffects,removalPlan} from './effects-ui';
import {fieldDefinitions,type FieldDraft} from './schema-fields';

function fixture(){
  const world=emptyWorld(),a=makeNode(world,'core.character','Nora'),b=makeNode(world,'core.character','Elias');world.nodes=[a,b];
  const edge:WorldEdge={id:crypto.randomUUID(),typeId:'story.feels-toward',sourceId:a.id,targetId:b.id,description:'Established notes',properties:{attitude:'Trusting'}};world.edges=[edge];
  return {world,edge,a,b};
}
describe('typed relationship authoring',()=>{
  it('offers empty declared fields in a link-only request without authorizing node or endpoint changes',()=>{
    const {world,edge}=fixture(),request=newLinkAuthoringRequest(world,edge.id);
    expect(request.targetIds).toEqual([]);expect(request.fields).toEqual([]);expect(request.contextIds).toEqual([]);
    expect(request.linkTargetIds).toEqual([edge.id]);expect(request.linkFields).toContainEqual({linkId:edge.id,property:'tension'});
    expect(editableLinkFields(world,[edge.id]).map(f=>f.label)).toContain('Nora → feels toward → Elias / Tension');
    expect(linkFields(world,edge).map(f=>f.key)).toEqual(['attitude','reasons','tension']);
  });
  it('changes type with declared defaults while preserving endpoints and notes',()=>{
    const {world,edge}=fixture(),next=changeLinkType(world,edge,'story.owes');
    expect(next).toEqual({...edge,typeId:'story.owes',properties:{}});expect(edge.properties).toEqual({attitude:'Trusting'});
    expect(changeLinkType(world,edge,edge.typeId)).toBe(edge);
  });
  it('keeps zero and false meaningful and removes a malformed extra field without mutating the original',()=>{
    const {edge}=fixture(),zero=setLinkProperty(edge,'amount',0),no=setLinkProperty(zero,'paid',false);
    expect(no.properties).toMatchObject({amount:0,paid:false});expect(edge.properties).toEqual({attitude:'Trusting'});
    expect(setLinkProperty(no,'amount',undefined).properties).not.toHaveProperty('amount');
  });
  it('rejects nested relationship values in both candidate and passage editors',()=>{
    const {edge}=fixture(),bad={...edge,properties:{attitude:{hidden:'invalid'}}};
    expect(parseCandidateForEditor(JSON.stringify({nodes:[],edges:[bad]}))).toBeNull();
    expect(parseEffects(JSON.stringify({changes:[{kind:'putLink',link:bad,evidence:'A continuous source quote.'}],notes:[],suggestChapterEnd:false}))).toBeNull();
    expect(parseCandidateForEditor(JSON.stringify({nodes:[],edges:[edge]}))?.edges).toEqual([edge]);
  });
  it('shows property changes and clearing values independently from moving a link',()=>{
    const {world,edge}=fixture(),next=setLinkProperty(setLinkProperty(edge,'attitude',null),'tension','Fear of disapproval');
    expect(linkDiff(world,world.nodes,next,edge)).toEqual([{label:'Attitude',before:'Trusting',after:'Unspecified'},{label:'Tension',before:'Unspecified',after:'Fear of disapproval'}]);
    expect(linkDiff(world,world.nodes,setLinkProperty(edge,'tension',null),edge)).toEqual([]);
  });
  it('follows typed link dependencies when removing a proposed node',()=>{
    const {world,edge}=fixture(),witness={...makeNode(world,'core.character','Witness'),id:'new:witness'};
    world.relationTypes.find(d=>d.id===edge.typeId)!.fields=[{key:'witness',label:'Witness',kind:'nodeRef',required:false,defaultValue:null}];
    const link=setLinkProperty(edge,'witness',witness.id),effects:PassageEffects={changes:[{kind:'putNode',node:witness,evidence:'The witness arrived.'},{kind:'putLink',link,evidence:'The witness arrived.'}],notes:[],suggestChapterEnd:false};
    expect(linkReferences(world,link)).toEqual([witness.id]);expect(removalPlan(world,effects,'change',0).changes).toEqual([0,1]);
  });
  it('shares custom field definitions with node extensions and keeps task uses independent',()=>{
    const draft:FieldDraft={key:'condition',label:' Condition ',kind:'choice',help:'Current access',uses:['story','planning'],stability:'state',choices:'open, restricted',referenceCapability:'',coveredBy:''};
    const definition=fieldDefinitions([draft])[0];expect(definition).toMatchObject({label:'Condition',required:false,defaultValue:null,metadata:{choices:['open','restricted'],uses:['story','planning']}});
    definition.metadata!.uses!.push('visual');expect(draft.uses).toEqual(['story','planning']);
  });
});
