# AI world authoring

The world workspace's **AI world tools** button creates a node or a group from an idea. Any node's inspector offers **Revise / reroll with AI**. Both use the same schema-driven service and review panel, including custom node types and capability fields. AI support is a service over world nodes; connection clients, credentials and job state are not embedded in each node.

## Create, review and apply

1. Open a saved project and configure **Settings → LLM**. The offline demo exercises the review flow without contacting a model.
2. Choose a preset, or **World / mixed group from an idea**. Enable related nodes to request a character with garments and carried items, a furnished room, or another group. The current response limit is 20 nodes and 80 links.
3. Add direction and optional context references. Only targets and explicitly selected reference records are sent, with the project's type/capability/relationship definitions. Expanding the map does not expand model context.
4. Generate once, then review the proposed records and links. Edit their fields manually, add/remove proposed group members and links, or edit retained JSON when a response needs repair. Invalid proposals stay saved and editable.
5. **Apply proposal** validates the whole resulting world and commits the group together. Until then, the established world is unchanged.

For revisions, check the fields AI may change. Name is protected by default. Unchecked fields, node types, capability sets and existing links cannot change in a revision proposal. Custom fields use the same rules as built-in fields. Changed references may point only to explicitly exposed records. Creation can attach new nodes to selected existing references; it cannot move or rewrite those existing nodes.

**New candidate / reroll** starts a separate saved draft with the same direction and scope. It retains the previous candidate, including manual edits. **New creation** starts a fresh creation request from any review. The saved-proposals list reopens older work. Revision of existing relationships continues through the manual connection tools; AI relationship transfers are not part of this slice.

This prototype retains up to 200 proposals per story. A proposal cleanup/archive interface is still pending.

## Settings and API behavior

The native client sends non-streaming requests to a configurable [Chat Completions API](https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create). Enter the API base URL, model and optional key. A base ending in `/chat/completions` is also accepted. Advanced controls select JSON object, JSON Schema (non-strict), or prompt-only output, and `max_tokens` or `max_completion_tokens`. Server support varies; no real-provider compatibility has yet been certified.

The configured context/output allowance is checked before submission using a conservative byte-based estimate, not an exact provider tokenizer. A generation makes one provider request, with no automatic retries, redirects, repairs or provider switching. Timeout, response size and group size are bounded. The connection test is an explicit, short request; opening settings does not contact a provider. The offline demo is deterministic sample data, not an LLM.

Connection settings live in `llm-connection.json` in the per-user app config directory, separately from stories and appearance preferences. **Save connection**, closing settings and normal app shutdown flush pending connection edits. Windows DPAPI encrypts API keys for the current OS user in separate credential files. Keys are not returned to the frontend after saving and are not written into proposals or story snapshots. Copying a story does not copy connection credentials.

## Drafts, interruption and source changes

Prompts, scope, original extracted provider text and manually edited candidate JSON have separate saved fields in the story's `proposals/` directory. Candidate edits autosave with independent versions. Complete thinking/code-fence wrappers are stripped from response text; the full HTTP envelope and provider reasoning are not stored. Truncated output is retained and cannot be applied until manually edited/reviewed or replaced with a new candidate.

One generation can run at a time. The panel can close while generation continues and the world remains editable. Cancel stops the local request; a remote server may continue its computation. Normal shutdown stops active work before closing. After abrupt termination, a saved running job becomes interrupted when the project reopens; it is never resent automatically. A result that cannot be saved is retained in the running app with an error, and project switching/closing is blocked until its save succeeds. Abrupt termination during that disk failure can still lose the in-memory result.

Each generation captures an immutable source snapshot and a fingerprint of the selected records, schema, direction, scope, connection settings and prompt contract. Relevant changes block Apply; unrelated record edits and map layout changes can coexist. Applying revalidates against the latest world and preserves unrelated work. Request receipts reconcile an apply that committed before its status write completed, so retrying does not duplicate a group.

This implements world foundation/revision tools. Story prose, passage acceptance/effects, chapter planning/history, forks, saved portraits and the image API remain future work.
