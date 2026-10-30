import {commands,unwrap} from './native';
import type {ImageWorkflowRequest,MediaTarget,MediaView} from './bindings';

export class MediaSession {
  view=$state<MediaView|null>(null);value=$state<ImageWorkflowRequest|null>(null);
  dirty=$state(false);saving=$state(false);error=$state('');
  private edits=0;private saved=0;private version=0;private timer:ReturnType<typeof setTimeout>|undefined;
  private flight:Promise<void>|null=null;
  private pending:{id:string;version:number;value:ImageWorkflowRequest;sequence:number}|null=null;
  constructor(readonly storyId:string){}
  adopt(view:MediaView){clearTimeout(this.timer);this.view=view;this.value=structuredClone($state.snapshot(view.draft.request));this.edits=this.saved=0;this.version=view.draft.version;this.dirty=false;this.pending=null;this.error='';}
  async open(id:string){await this.flush();this.adopt(unwrap(await commands.getImageCandidate(this.storyId,id)));}
  async create(target:MediaTarget){await this.flush();this.adopt(unwrap(await commands.createImageCandidate(this.storyId,crypto.randomUUID(),target)));}
  touch(){if(!this.view||!this.value||this.view.draft.stage!=='draft')return;this.edits++;this.dirty=true;clearTimeout(this.timer);this.timer=setTimeout(()=>{void this.flush().catch(()=>{});},350);}
  async refresh(){if(!this.view)return;await this.flush();const id=this.view.draft.id,view=unwrap(await commands.getImageCandidate(this.storyId,id));if(this.view?.draft.id===id&&!this.dirty&&!this.saving)this.adopt(view);}
  async flush(){clearTimeout(this.timer);if(this.flight)return this.flight;if(!this.view||!this.value)return;this.flight=this.saveLoop();try{await this.flight;}finally{this.flight=null;}}
  private async saveLoop(){this.saving=true;try{while(this.pending||this.saved<this.edits){const operation=this.pending??{id:this.view!.draft.id,version:this.version,value:structuredClone($state.snapshot(this.value!)),sequence:this.edits};this.pending=operation;const view=unwrap(await commands.saveImageCandidate(this.storyId,operation.id,operation.version,operation.value));this.view=view;this.version=view.draft.version;this.saved=operation.sequence;this.pending=null;this.error='';}this.dirty=false;}catch(caught){this.error=caught instanceof Error?caught.message:String(caught);throw caught;}finally{this.saving=false;}}
  dispose(){clearTimeout(this.timer);}
}
