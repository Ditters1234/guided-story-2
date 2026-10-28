import {commands,unwrap} from './native';
import type {PassageEdits,PassagePlan,PassageView_Serialize} from './bindings';

export class PassageSession {
  id=$state<string|null>(null);
  value=$state<PassageEdits|null>(null);
  view=$state<PassageView_Serialize|null>(null);
  dirty=$state(false);saving=$state(false);error=$state('');
  private version=0;private edits=0;private saved=0;
  private timer:ReturnType<typeof setTimeout>|undefined;
  private flight:Promise<void>|null=null;
  private pending:{id:string;version:number;edits:PassageEdits;sequence:number}|null=null;
  constructor(readonly storyId:string){}
  get running(){return this.view?.stage==='narrating'||this.view?.stage==='checking';}
  get readyToAccept(){return !!this.view?.canAccept&&!this.dirty&&!this.saving;}
  async open(id:string){await this.flush();this.adopt(unwrap(await commands.getPassageDraft(this.storyId,id)));}
  async create(plan:PassagePlan){await this.flush();this.id=crypto.randomUUID();this.value={plan:structuredClone(plan),prose:'',effectsText:'{"changes":[],"notes":[],"suggestChapterEnd":false}',closeChapter:false,approvedEffects:[]};this.view=null;this.version=0;this.edits=this.saved=0;this.pending=null;this.touch();await this.flush();}
  async reroll(){await this.flush();if(!this.id||!this.view)throw Error('Open a saved passage candidate first.');this.adopt(unwrap(await commands.clonePassageDraft(this.storyId,this.id,this.view.version,crypto.randomUUID())));}
  clear(){clearTimeout(this.timer);this.id=null;this.value=null;this.view=null;this.version=0;this.edits=this.saved=0;this.pending=null;this.dirty=false;this.error='';}
  adopt(view:PassageView_Serialize){clearTimeout(this.timer);this.id=view.id;this.view=view;this.value=structuredClone($state.snapshot(view.edits));this.version=view.version;this.edits=this.saved=0;this.pending=null;this.dirty=false;this.error='';}
  touch(){if(!this.id||!this.value||this.running||this.view?.stage==='accepted')return;this.edits++;this.dirty=true;clearTimeout(this.timer);this.timer=setTimeout(()=>{void this.flush().catch(()=>{});},350);}
  async refresh(){if(!this.id)return;await this.flush();const id=this.id;const view=unwrap(await commands.getPassageDraft(this.storyId,id));if(this.id===id&&!this.dirty&&!this.saving)this.adopt(view);}
  async flush(){clearTimeout(this.timer);if(this.flight)return this.flight;if(!this.id||!this.value)return;this.flight=this.saveLoop();try{await this.flight;}finally{this.flight=null;}}
  private async saveLoop(){this.saving=true;try{
    while(this.pending||this.saved<this.edits){
      const operation=this.pending??{id:this.id!,version:this.version,edits:structuredClone($state.snapshot(this.value!)),sequence:this.edits};this.pending=operation;
      const result=unwrap(await commands.savePassageDraft(this.storyId,operation.id,operation.version,operation.edits));
      this.version=result.version;this.view=result;this.saved=operation.sequence;this.pending=null;this.error='';
    }this.dirty=false;
  }catch(error){this.error=error instanceof Error?error.message:String(error);throw error;}finally{this.saving=false;}}
  dispose(){clearTimeout(this.timer);}
}
