# Story-wide AI context

Use **AI context** in the World toolbar or Story composer, or **AI context → Manage / preview** in a node's inspector. Any world node, including a custom preset, can supply instructions or reference facts to selected AI tools. A plain **Note** preset and **New context note** action provide a convenient source for narration preferences, world rules, author guidance or background material. They use the same node/binding machinery as characters, items and places.

Each inclusion is a story-level binding to its source node, separate from physical map links:

- **Enabled:** temporarily stop inclusion while keeping its configuration.
- **Include as:** Instructions are appended as labelled story instruction sections to the tool's editable base system message. Reference facts are supplied separately as `includedReferenceFacts` in the task input.
- **AI tools:** world creation/rerolls, passage writing, wording edits, effects extraction, chapter memory and image-prompt assistance. Select all selects the currently supported tools; an empty selection sends nothing.
- **Order:** lower numbers appear first, with stable node-ID ordering for ties. Each role preserves that order in its own prompt section.
- **Content:** choose Story notes, Visible description and/or Capability fields. Capability values use the project's field labels, including custom fields. Node-reference values do not expand referenced nodes. No linked nodes or hidden container contents are followed through this binding.

The source node owns the text/fields. Editing them here or in the shared inspector changes the same record. Configuration and manual source edits autosave with the world. **Remove inclusion** leaves the node intact; removing a node through the normal validated operation also removes its inclusion. A disabled inclusion retains task, role, field and order choices. Up to 32 nodes can have bindings.

## Review what is included

Choose an AI tool in **Preview**. Cards show the selected source content in authored order. Expand **Assembled system instructions** to see the current base message plus instruction nodes, and **Reference facts supplied with the task** to inspect the separate reference sections. Scene direction, history, response schemas and other scoped task data are assembled by their existing tools; this is not a whole-request scene preview. Preview makes no inference request.

A narration note can be Instructions for Passage writing and Wording edits, while magic rules can be Reference facts for world creation, passages and extraction. Existing structured capabilities can provide person, tense or any other authored fields; there is no engine-specific narration collection. A bound node is not automatically a permitted edit target, a present participant or knowledge shared by every character. Apply/Accept and the native task schemas still validate proposed changes.

## Budgets and history

Included material is required context. It is added before optional story memory is budgeted and is never silently truncated/dropped. Requests that cannot fit report the included source names. Empty selected content blocks the matching request with a named repair message; an unfinished note can be filled in, excluded from that tool or disabled. There is also a 128 KB aggregate required-content bound.

Binding configuration lives in the world's optional `aiContext` field. Source nodes and bindings therefore travel with the story, immutable passage snapshots, backups and forks. Empty bindings are omitted so legacy stored-world receipts retain their original representation. Existing worlds load without a conversion; creating a Note in an older project explicitly adds that plain preset when needed. Older binaries cannot read worlds with the new nonempty field.

New passages use the current story bindings. Relevant included-source changes stale pending narrative checks, including changes made during generation; retained prose is not discarded. Bindings only for unrelated tools do not stale a checked passage. Historical wording/event revisions and passage image-prompt assistance resolve context from the saved passage boundary. Node image-prompt assistance uses current story context and captures the selected content. Image assistance affects the LLM suggestion; direct ComfyUI prompts remain explicitly edited/applied in the image composer.

Generated memory drafts capture their included context and require it to remain current before Apply. Accepted memory retains that provenance. A fork excludes a summary whose captured context differs from the fork boundary, avoiding copying later included knowledge into an earlier branch. Earlier accepted prose/world files are never rewritten when context nodes change.
