# Passage world-change audit

November 8, 2026. Reviewed the current working tree based on `dc37b24`, including the pre-existing uncommitted relationship, memory and review changes. This is an audit, not an implementation change. No private stories, saved connection settings or live providers were accessed.

Follow-up: the first implementation pass covers the structured-state prompt, explicit bounded reference suggestions, populated-field omission guards and review-card additions/coverage. The second adds sparse existing-node provider updates, expanded from the exact captured baseline into complete review records, and up to four independently validated evidence excerpts per change. See [passage authoring](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/STORY_AUTHORING.md>) and [verification checkpoint](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/FOUNDATION_STATUS.md>). The findings below record the audited behavior before those passes. Semantic completeness remains an author-review responsibility; real-model recall/precision has not been measured for these changes.

## Assessment

The concern is supported by the implementation. The pipeline is substantially stronger at validating and publishing the changes it receives than at finding all the changes a passage establishes. Its current scope can exclude the very existing records that need updating. Its prompt emphasizes restraint and physical operations more clearly than structured state maintenance. The output contract also makes small updates expensive and vulnerable to accidental field loss.

The existing generic nodes, schema metadata, scene references, extraction request, editable cards and atomic acceptance are sufficient foundations for improving this. These findings do not call for another fact store, simulation engine or autonomous review workflow.

## What runs today

1. `scene_world` captures selected viewpoint, focus, participants and references, relevant claims, directly carried/worn items and physical placement ancestors. It retains links whose endpoints are both captured.
2. An explicit extraction request receives that captured world, exact prose, starting positions, prior context and concrete node/link contracts. It returns node updates, link updates/removals and continuity notes.
3. Native validation checks quote matching, IDs, scope, schema constraints, placement/cardinality, planning-field restrictions and the resulting world. It does not compare the prose with the effects for semantic completeness.
4. The author reviews cards. Acceptance revalidates and publishes prose, effects, chapter membership and before/after snapshots together. Notes are retained history; they do not update node fields.

Sources: [scene capture](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story.rs:395>), [request assembly](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story.rs:922>), [native validation](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects.rs:217>), [acceptance](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story.rs:1384>).

## Findings and recommended changes

### 1. High impact: relevant existing nodes can be unavailable to extraction

Inventory discovery follows only items directly linked to selected characters. Placement expansion then travels toward holders/ancestors, rather than toward container contents or objects in the scene. Nonphysical links do not generally bring their other endpoint into scope.

**Verified example:** the fictional world contained Mara, Jon, their Workshop, a carried Satchel, a Remedy inside it, a Lantern stored in the Workshop and a connected `Find the cure` thread. The real extraction builder supplied only Workshop, Mara, Jon and Satchel. A manual Remedy quantity update was then rejected as outside the supplied scene.

This can look like poor reasoning while being an input/permission limitation. A name occurring in prose, previous prose or a binding does not supply the complete existing record or authorize its update. The model can also be tempted to create a duplicate for an existing object it cannot identify.

**Change within the existing system:** improve the existing Scene resources selector with bounded suggestions for selected-container contents, relevant scene objects and linked threads. Make adding those references easy before checking. Extend automatic inventory coverage only for a declared, bounded relevant neighborhood; prioritize explicit references, expose omissions and retain the existing context budget. Do not recursively include an entire location or all connected nodes. Any newly supplied IDs must enter the native captured scope and fingerprint before extraction/approval.

Sources: [direct inventory](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story/scene_context.rs:34>), [ancestor-only expansion](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story.rs:423>), [scope enforcement](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects.rs:283>), [existing reference selector](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/src/lib/SceneResources.svelte:43>).

### 2. High impact, low implementation cost: extraction instructions underemphasize structured state

The default prompt opens with “completed world changes,” extensively explains transfers/placement and warns against inferring lasting motives, emotions and objectives. These safeguards are useful, but there is no equally explicit positive checklist for maintaining existing condition, current intent, relationship details and thread progress. “Prefer the smallest useful set” can further bias toward a handoff plus a note.

This is a **likely model-quality contributor**, not a measured model failure in this audit. The schema already supplies field help and `state`/`stable` metadata. The necessary state fields already exist.

**Change:** revise the effective Effects prompt to ask for a comparison against current values, using the supplied schema:

- Final location, carried/worn placement and explicit quantity changes.
- Injuries, damage, wetness and other persistent current condition, including recovery/clearing when established.
- Explicitly established current feelings and immediate objectives. An objective becoming current is different from its intended action being completed; a gesture alone remains insufficient evidence.
- Evidenced changes to existing relationship details.
- Selected thread status and unresolved issues when the passage advances or resolves them.
- Newly established records where necessary, and continuity notes for useful history or uncertainty that should not replace a structured state update.

Use two short examples showing several justified updates from one passage and an unchanged field preserved. Do not require an update to every mentioned node, infer private states, automate knowledge acquisition, or turn plans into completed events. Keep one extraction call and explicit approval. Saved custom Effects prompts must remain effective; updating the bundled default alone will not change an override.

Sources: [default prompt](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/prompts/effects.txt:1>), [condition/current intent](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/schema/world-presets.json:581>), [thread fields](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/schema/world-presets.json:957>), [metadata projection](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/prompt_context.rs:41>), [prompt overrides](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/text_prompts.rs:95>).

### 3. High impact: a small update can erase unrelated optional facts

`putNode` requires a complete node and replaces the saved node. The provider schema requires component objects, but usually does not require their populated optional fields. Native validation protects type/capability sets and planning fields, but does not generally reject omission of a populated optional story field.

**Verified example:** an injury update omitted Mara's existing `detail.intent.objective = Repair the pump`. It passed validation, could be approved, and acceptance removed that unrelated objective. The review diff does expose the deletion; it is not protected automatically.

**First change:** require previously populated fields in the provider response and reject accidental omission natively with a per-card diagnostic. Distinguish omission from an explicit reviewed clearing. Keep existing type/capability and planning restrictions. This fits the existing full-record contract.

**Later optimization:** if output copying remains a quality bottleneck, let the existing provider materializer overlay explicitly supplied changed fields onto the captured baseline, producing the same complete native `putNode` for review and validation. Explicit clears must remain distinguishable. This is a transport refinement, but larger than the first fix and unnecessary to start.

Sources: [provider field requirements](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects/schema.rs:96>), [whole-node replacement](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects.rs:382>), [existing diff](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/src/lib/effects-ui.ts:90>).

### 4. Medium impact: “checked” does not establish correctness or completeness

**Verified:** an empty model extraction was accepted for prose explicitly handing the Satchel from Mara to Jon. The accepted prose recorded the handoff, while the world still placed the Satchel with Mara. Separately, a matching quote about waiting passed native validation for an unsupported broken-arm update. The latter still required author approval.

These are limits of the checker, not evidence of a broken transaction. Quote existence does not prove entailment, and a validator cannot detect an effect that was never proposed. Empty changes must remain valid for passages that establish no world changes.

**Change:** label the status as proposal validation and show structured-change and note counts separately. Give zero-update results a visible review message. In the existing review pane, show captured resources alongside proposed updates so omissions are easier to notice. Do not claim that another native structural check certifies prose continuity, or block all empty results.

Sources: [quote matching](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects.rs:149>), [validation path](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story.rs:1176>), [empty approval set](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story.rs:1285>), [current status wording](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/src/lib/StoryWorkspace.svelte:66>).

### 5. Medium impact, low implementation cost: review is better at fixing proposals than adding missing ones

Cards support editing/removing what the model supplied. There is no ordinary control to add an omitted node update, connection or continuity note; the available alternatives are Advanced JSON, another extraction, or clearing all changes. Connection selectors also expose the full world even though native effect scope is narrower. New-node cards show descriptions by default rather than their structured fields; existing diffs truncate values to 220 characters.

**Change:** add an existing-node update, connection or note through the current card editor, initialized from the captured baseline. Keep one final update per record. Restrict new choices to supplied IDs plus proposed nodes, while preserving permitted unchanged references; offer the existing Add reference flow for missing targets. Show populated structured fields for new nodes and expandable full diffs for long values. All additions use the same save, evidence, approval and atomic-acceptance path.

Sources: [review actions](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/src/lib/StoryWorkspace.svelte:239>), [card display/editor](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/src/lib/EffectCard.svelte:34>).

### 6. Medium impact, larger change: one quote per record cannot support distant changes well

Each node/link update has one continuous 8–600 character evidence string. Duplicate record IDs are prohibited. If a character is injured early and explicitly changes objective much later, one permitted excerpt may not cover both changes. The prompt's advice to split unsupported claims conflicts with splitting updates to the same record.

**Change after the cheaper fixes:** allow a small bounded set of exact excerpts on the existing card, still representing one final record update. This extends existing evidence handling rather than adding an Event/fact system. It needs coordinated review-key, source-anchor, historical receipt, backup and fork checks; do not merely relax quote matching or allow duplicate whole-record updates.

Sources: [effect representation](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects.rs:12>), [quote limits](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects.rs:149>), [duplicate rejection](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story_effects.rs:273>), [source quote capture](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/sources.rs:117>).

## Recommended delivery order

| Order | Existing area | Concrete result |
| --- | --- | --- |
| 1 | Effects prompt + deterministic fixtures | Explicit structured-state checklist, useful examples, multi-node expected before/after differences. |
| 2 | Scene resources + capture | Bounded relevant inventory/reference coverage and visible omissions; no unselected-world expansion. |
| 3 | Schema + native diagnostics | Populated optional facts cannot disappear through omission. |
| 4 | Review pane/cards | Add missing proposals, inspect complete values, distinguish validation from completeness. |
| 5 | Provider projection | Reduce copying/catalogue overhead only where measured, preserving required instructions and full native validation. |
| 6 | Card evidence | Bounded multiple quotes if simultaneous distant changes remain a limitation. |

The existing small request-size fixture measured **58,360 bytes** across system/user/schema for effects versus **9,899 bytes** for prose in schema mode. That is input size, not tokenizer output, latency or a model-quality score. The extractor includes definitions for many possible new presets as well as actual targets. Improve task focus before increasing context/output allowances. Preserve generic/custom types and concrete per-target contracts.

Source: [request-size fixture](<C:/Users/n_tin/Documents/ChatGPT/Guided Story 2/crates/story-project/src/story/tests/planning.rs:4>).

## Verification and next quality checks

Executed against this working tree:

- `node scripts/cargo.mjs test -p story-project story::tests -- --nocapture`: **79 passed, 4 ignored**. Includes acceptance, staleness, relationship guards, revision/fork and interrupted-publication checks.
- `node scripts/cargo.mjs test -p story-project story_effects::schema::tests -- --nocapture`: **13 passed**.
- `npm test -- src/lib/effects-ui.test.ts src/lib/effect-card.test.ts`: **11 passed**. These are automated tests, not desktop interaction checks.
- Four temporary native integration probes: **4 passed**, confirming the limitations described in findings 1, 3 and 4. Fixtures used temporary fictional folders and prepared requests; no network calls. Probe source was removed after execution.

No desktop walkthrough, full application build or real-model quality evaluation was performed for this audit. Existing tests demonstrate substantial safety coverage, but their passing results do not establish extraction recall.

For implementation, use one compact fictional passage with known initial state and expected diffs covering: a handoff, a consumed nested item, injury/recovery, an explicit current-objective change, an existing relationship-detail change and a selected thread resolution. Also assert preservation of unrelated facts, no invented quantities and no updates from mere mentions. Test scope and validation deterministically. Then, only in a separate explicit short provider probe, measure how many expected updates the model actually proposes and whether it invents extras. Keep those model-quality observations separate from automated correctness results.
