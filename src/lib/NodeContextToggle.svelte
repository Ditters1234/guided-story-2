<script lang="ts">
  import type {NodeContextChoice} from './writing-inputs';
  let {choice,name,compact=false,disabled=false,onToggle}:{choice:NodeContextChoice;name:string;compact?:boolean;disabled?:boolean;onToggle:()=>void}=$props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions (Stops graph selection; the native checkbox supplies keyboard interaction.) -->
<label class="node-context-toggle nodrag nopan" class:compact class:included={choice.included} class:context-warning={choice.warning} title={choice.reason} onclick={event=>event.stopPropagation()} onpointerdown={event=>event.stopPropagation()}>
  <input type="checkbox" checked={choice.checked} disabled={disabled||choice.disabled} aria-label={`${name}: ${choice.label}. ${choice.reason}`} onchange={onToggle}/>
  {#if choice.locked}<svg class="context-lock" viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.6" aria-label="Required context"><rect x="3" y="7" width="10" height="7" rx="1.5"/><path d="M5 7V5a3 3 0 0 1 6 0v2"/></svg>{/if}
  {#if !compact||choice.warning}<span>{choice.label}</span>{/if}
</label>
