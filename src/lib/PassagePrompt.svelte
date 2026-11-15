<script lang="ts">
  import type {PassagePromptAction, PassagePromptPreview} from './bindings';
  import {commands, unwrap} from './native';

  let {
    storyId, draftId, version, revision, active, pending, hasProse, wording, alternative
  }: {
    storyId: string;
    draftId: string;
    version: number;
    revision: number;
    active: boolean;
    pending: boolean;
    hasProse: boolean;
    wording: boolean;
    alternative: boolean;
  } = $props();

  let open = $state(false);
  let action = $state<PassagePromptAction>('generate');
  let preview = $state<PassagePromptPreview | null>(null);
  let error = $state('');
  let refresh = $state(0);
  const selectedAction = $derived(hasProse && !(wording && action === 'checkChanges') ? action : 'generate');
  const generateLabel = $derived(wording
    ? alternative ? 'Generate polish alternative' : 'Generate polish candidate'
    : alternative ? 'Generate alternative' : 'Generate passage');

  $effect(() => {
    preview = null;
    error = '';
    if (!open || !active || pending) return;
    void refresh;
    let current = true;
    void commands.previewPassagePrompt(storyId, draftId, version, revision, selectedAction)
      .then(unwrap)
      .then(result => { if (current) preview = result; })
      .catch(caught => {
        if (current) error = caught instanceof Error ? caught.message : String(caught);
      });
    return () => { current = false; };
  });
</script>

<details class="context-coverage passage-prompt" bind:open>
  <summary>Full prompt</summary>
  {#if open}
    {#if hasProse}
      <label class="field">Action
        <select value={selectedAction} onchange={event => { action = event.currentTarget.value as PassagePromptAction; }}>
          <option value="generate">{generateLabel}</option>
          <option value="revise">Revise draft with AI</option>
          {#if !wording}<option value="checkChanges">Check world changes</option>{/if}
        </select>
      </label>
    {/if}
    {#if pending || !active}
      <p class="field-help" role="status">Waiting for the current draft and settings.</p>
    {:else if error}
      <p class="field-help" role="status">{error}</p>
      <button class="text-action" onclick={() => { refresh++; }}>Refresh prompt</button>
    {:else if preview}
      {#if preview.offline}<p class="field-help">The offline demo uses a local sample response.</p>{/if}
      <label class="field">System message
        <textarea readonly rows={7} value={preview.system}></textarea>
      </label>
      <label class="field">User message
        <textarea readonly rows={10} value={preview.user}></textarea>
      </label>
      {#if preview.responseSchema}
        <label class="field">Response schema
          <textarea readonly rows={7} value={preview.responseSchema}></textarea>
        </label>
      {/if}
    {:else}
      <p class="field-help" role="status">Preparing prompt…</p>
    {/if}
  {/if}
</details>

<style>
  textarea {
    font-family: var(--font-mono, monospace);
    font-size: .75rem;
    line-height: 1.55;
    overflow-wrap: anywhere;
    resize: vertical;
  }
</style>
