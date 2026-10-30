import type { Preferences } from './bindings';
import { defaultPreferences } from './appearance';
import { commands, desktop, unwrap } from './native';

export class AppearanceSettings {
  value = $state<Preferences>(defaultPreferences());
  ready = $state(false);
  saving = $state(false);
  dirty = $state(false);
  error = $state('');
  private edits = 0;
  private saved = 0;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private running: Promise<void> | null = null;
  private pending: {preferences:Preferences; edit:number} | null = null;

  async load() {
    this.ready=false;
    clearTimeout(this.timer);
    try {
      if (this.running) await this.running.catch(()=>{});
      this.value = desktop ? unwrap(await commands.loadPreferences()) : defaultPreferences();
      this.edits=this.saved=0; this.pending=null; this.ready=true; this.dirty=false; this.error='';
    } catch (error) { this.ready=false; this.error=String(error instanceof Error?error.message:error); }
  }
  update(changes: Partial<Pick<Preferences,'themeId'|'textScale'|'reducedMotion'|'customThemes'>>) {
    if (!this.ready) return;
    this.value={...this.value,...changes}; this.edits++; this.dirty=true;
    clearTimeout(this.timer);
    this.timer=setTimeout(()=>{void this.flush().catch(()=>{});},300);
  }
  async flush() {
    clearTimeout(this.timer);
    if (this.running) return this.running;
    if (!this.ready || (!this.pending && this.saved===this.edits)) return;
    this.running=this.saveLoop();
    try { await this.running; } finally { this.running=null; }
  }
  private async saveLoop() {
    this.saving=true;
    try {
      while (this.pending || this.saved<this.edits) {
        const operation=this.pending??{preferences:structuredClone($state.snapshot(this.value)),edit:this.edits};
        this.pending=operation;
        const saved=desktop ? unwrap(await commands.savePreferences(operation.preferences.version,operation.preferences)) : {...operation.preferences,version:operation.preferences.version+1};
        this.value={...this.value,version:saved.version}; this.saved=operation.edit; this.pending=null; this.error='';
      }
      this.dirty=false;
    } catch (error) { this.error=String(error instanceof Error?error.message:error); throw error; }
    finally { this.saving=false; }
  }
}
