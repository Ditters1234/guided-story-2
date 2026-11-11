<script lang="ts">
  let {title,disabled=false,onDelete}:{title:string;disabled?:boolean;onDelete:()=>Promise<void>}=$props();
  let confirming=$state(false);
</script>

<button type="button" class="text-action danger-text draft-delete" disabled={disabled} aria-label={`Delete draft: ${title}`} onclick={()=>{confirming=true;}}>Delete draft</button>
{#if confirming}
  <div class="draft-delete-confirm" role="alert">
    <p>Delete “{title}”? This removes the draft, including its unaccepted edits.</p>
    <div><button type="button" class="secondary-button danger-text" disabled={disabled} onclick={()=>{void onDelete().finally(()=>{confirming=false;});}}>Confirm delete</button><button type="button" class="text-action" disabled={disabled} onclick={()=>{confirming=false;}}>Cancel</button></div>
  </div>
{/if}

<style>
  .draft-delete { flex-shrink: 0; padding: 8px; font-size: .75rem; }
  .draft-delete-confirm { grid-column: 1 / -1; flex-basis: 100%; min-width: 0; padding: 10px; border: 1px solid var(--border); border-radius: 5px; font-size: .85rem; background: var(--surface); }
  .draft-delete-confirm p { margin: 0 0 10px; overflow-wrap: anywhere; }
  .draft-delete-confirm div { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; }
</style>
