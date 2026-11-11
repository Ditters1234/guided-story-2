import {commands,unwrap} from './native';
import type {ImagePromptEdits,ImagePromptView} from './bindings';

export class ImagePromptSession {
  view=$state<ImagePromptView|null>(null);value=$state<ImagePromptEdits|null>(null);
  dirty=$state(false);saving=$state(false);error=$state('');
  private edits=0;private saved=0;private version=0;private timer:ReturnType<typeof setTimeout>|undefined;
  private flight:Promise<void>|null=null;
  private pending:{id:string;version:number;value:ImagePromptEdits;sequence:number}|null=null;
  constructor(readonly storyId:string){}
  get running(){return this.view?.stage==='running';}
  get ready(){return !!this.view?.canApply&&!this.dirty&&!this.saving&&!this.view.retained;}
  adopt(view:ImagePromptView){clearTimeout(this.timer);this.view=view;this.value=structuredClone($state.snapshot(view.edits));this.edits=this.saved=0;this.version=view.version;this.dirty=false;this.pending=null;this.error='';}
  async open(id:string){await this.flush();this.adopt(unwrap(await commands.getImagePrompt(this.storyId,id)));}
  async create(mediaId:string,version:number){await this.flush();this.adopt(unwrap(await commands.createImagePromptDraft(this.storyId,crypto.randomUUID(),mediaId,version)));}
  touch(){if(!this.view||!this.value||this.running||this.view.stage==='applied'||this.view.retained)return;this.edits++;this.dirty=true;clearTimeout(this.timer);this.timer=setTimeout(()=>{void this.flush().catch(()=>{});},350);}
  async refresh(){if(!this.view)return;await this.flush();const id=this.view.id,view=unwrap(await commands.getImagePrompt(this.storyId,id));if(this.view?.id===id&&!this.dirty&&!this.saving)this.adopt(view);}
  async flush(){clearTimeout(this.timer);if(this.flight)return this.flight;if(!this.view||!this.value)return;this.flight=this.saveLoop();try{await this.flight;}finally{this.flight=null;}}
  private async saveLoop(){this.saving=true;try{while(this.pending||this.saved<this.edits){const operation=this.pending??{id:this.view!.id,version:this.version,value:structuredClone($state.snapshot(this.value!)),sequence:this.edits};this.pending=operation;const view=unwrap(await commands.saveImagePromptDraft(this.storyId,operation.id,operation.version,operation.value));this.view=view;this.version=view.version;this.saved=operation.sequence;this.pending=null;this.error='';}this.dirty=false;}catch(caught){this.error=caught instanceof Error?caught.message:String(caught);throw caught;}finally{this.saving=false;}}
  dispose(){clearTimeout(this.timer);}
}
