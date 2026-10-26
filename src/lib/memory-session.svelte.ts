import {commands,unwrap} from './native';
import type {MemoryEdits,MemoryView} from './bindings';

export class MemorySession {
  id=$state<string|null>(null);view=$state<MemoryView|null>(null);value=$state<MemoryEdits|null>(null);
  dirty=$state(false);saving=$state(false);error=$state('');
  private version=0;private edits=0;private saved=0;private timer:ReturnType<typeof setTimeout>|undefined;
  private flight:Promise<void>|null=null;
  private pending:{id:string;version:number;edits:MemoryEdits;sequence:number}|null=null;
  constructor(readonly storyId:string){}
  get running(){return this.view?.stage==='running';}
  get readyToApply(){return !!this.view?.canApply&&!this.dirty&&!this.saving;}
  adopt(view:MemoryView){clearTimeout(this.timer);this.id=view.id;this.view=view;this.value=structuredClone($state.snapshot(view.edits));this.version=view.version;this.edits=this.saved=0;this.pending=null;this.dirty=false;this.error='';}
  async open(id:string){await this.flush();this.adopt(unwrap(await commands.getMemoryDraft(this.storyId,id)));}
  async create(chapterId:string){await this.flush();this.adopt(unwrap(await commands.createMemoryDraft(this.storyId,crypto.randomUUID(),chapterId)));}
  async clear(){await this.flush();clearTimeout(this.timer);this.id=null;this.view=null;this.value=null;this.pending=null;this.version=this.edits=this.saved=0;this.dirty=false;this.error='';}
  touch(){if(!this.id||!this.value||this.running||this.view?.stage==='applied')return;this.edits++;this.dirty=true;clearTimeout(this.timer);this.timer=setTimeout(()=>{void this.flush().catch(()=>{});},350);}
  async refresh(){if(!this.id)return;await this.flush();const id=this.id,view=unwrap(await commands.getMemoryDraft(this.storyId,id));if(this.id===id&&!this.dirty&&!this.saving)this.adopt(view);}
  async flush(){clearTimeout(this.timer);if(this.flight)return this.flight;if(!this.id||!this.value)return;this.flight=this.saveLoop();try{await this.flight;}finally{this.flight=null;}}
  private async saveLoop(){this.saving=true;try{while(this.pending||this.saved<this.edits){
    const operation=this.pending??{id:this.id!,version:this.version,edits:structuredClone($state.snapshot(this.value!)),sequence:this.edits};this.pending=operation;
    const view=unwrap(await commands.saveMemoryDraft(this.storyId,operation.id,operation.version,operation.edits));
    this.view=view;this.version=view.version;this.saved=operation.sequence;this.pending=null;this.error='';
  }this.dirty=false;}catch(error){this.error=error instanceof Error?error.message:String(error);throw error;}finally{this.saving=false;}}
  dispose(){clearTimeout(this.timer);}
}
