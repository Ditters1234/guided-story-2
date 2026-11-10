# Passage-to-passage continuity review

The findings below describe the state before the focused implementation pass. The completed pass and verification are recorded at the end and in [FOUNDATION_STATUS.md](FOUNDATION_STATUS.md#november-8-passage-continuity-pass).

Reviewed November 8, 2026, against the current working tree, including its existing uncommitted changes. This is a source review and deterministic native verification, not a review of a private story or evidence from the running desktop build. No application behavior was changed.

## Assessment

The normal prose request includes the **last two accepted passages in full**, in chronological order, with exact record versions and accepted continuity notes. Those passages survive a project close/reopen. Optional chapter memory is a separate input; it is not required for the immediately preceding passage to appear. Required recent prose is not silently shortened to make the generation budget fit: an oversized required request blocks instead.

However, continuity is fragile. The app has no automatically maintained account of which chapter beats or conversations are already settled. Beyond the last two passages, it depends on accepted notes and explicitly reviewed chapter summaries. The AI planner has an even shorter memory than the prose writer. These are credible contributors to repetition, but they do not establish the cause of a particular model output without inspecting that generation's actual inputs.

## Findings

### 1. Completed chapter instructions continue to be presented as remaining work

**Priority: high for same-chapter repetition; confirmed behavior, inferred effect on generation.**

`next_passage_plan` copies the active chapter's plan and author progress unchanged (`crates/story-project/src/story.rs:524`). Acceptance writes the submitted progress text back; it does not derive progress from the accepted prose (`story.rs:1416`). `writing_brief::assemble` then labels the unchanged plan `chapterRemaining` (`story/writing_brief.rs:35`). The default writing prompt tells the model not to repeat completed beats, but the app does not resolve that tension for it.

An isolated probe accepted a passage that settled a ferry departure discussion. The next request still described “Discuss the ferry timetable and agree when to leave” as remaining work, with no `alreadyCovered` recap. This can encourage the same conversation again even while the previous prose is supplied.

**Recommendation:** assemble a compact, source-linked account of settled beats from reviewed accepted notes alongside the author's unchanged plan. Keep the author recap editable and separate from derived history. Do not silently rewrite chapter plans or treat plans as accomplished events.

### 2. AI planning discards older memory even when the writer can retrieve it

**Priority: high when using AI scene/chapter suggestions; confirmed context gap.**

`prepare_plan_fill` starts with a zero memory budget and explicitly removes `earlierMemory` (`crates/story-project/src/story/planning.rs:69–79`). It supplies only optional ending excerpts from the last two passages, capped at 3,000 characters each. There is no allowance for older accepted notes or reviewed summaries, even with ample remaining context space. Under tighter budgets, recent excerpts can also be omitted.

A probe gave the first passage an approved note that the ferry discussion was settled, then accepted two newer passages. Both scene and chapter planning omitted that note; the prose-generation request included it. A plan generated without this history can therefore ask the writer to revisit a settled topic.

**Recommendation:** use bounded, perspective-filtered older memory in planning after reserving required inputs. Expose planning omissions. This needs no additional provider call. A current passage's ending should receive explicit priority within optional history, with any missing handoff visible.

### 3. There is a sharp two-passage cutoff for conversations without accepted notes

**Priority: high for repetition of material three or more passages back; confirmed coverage limitation.**

Recent prose uses `.take(2)` (`crates/story-project/src/story.rs:802`). Older retrieval uses current completed-chapter summaries or accepted continuity notes, not the older prose itself (`memory/context.rs:99–197`). Summaries require a completed chapter and explicit generation/review/Apply. An open chapter cannot acquire a chapter summary through this path. Empty passage notes are valid, and their factual coverage is not checked for completeness.

An isolated probe accepted a settled conversation followed by two other passages, with no notes. The next request included the two latest passages exactly, omitted the conversation entirely, and correctly reported one unrepresented older passage. The text remains saved; it is absent from generation context.

Older recall is also bounded to 12 points and at most 6,000 bytes of point JSON, with less space available when required context grows. Ranking favors selected resources and recency, rather than explicitly prioritizing settled dialogue or unresolved final actions.

**Recommendation:** make the existing extraction task explicitly propose useful continuity notes for consequential conversations, decisions, promises and unfinished actions, subject to the existing individual review/Accept flow. Build a deterministic recent continuity section from those accepted notes, preserving source versions, budgets and perspective filtering. Retain explicit chapter-summary maintenance; do not introduce hidden summarization calls or an unbounded history dump.

### 4. Successful validation does not mean repetition was checked

**Priority: medium; confirmed quality limitation.**

`validate_passage` validates the prose's presence, effect structure/evidence, source fingerprints and world changes (`crates/story-project/src/story.rs:1152`). It does not compare dialogue/events with previous prose for repetition. The default prompt already instructs the model to continue the final beat, skip recaps and avoid replaying accepted actions. These instructions are not a semantic verification stage.

An isolated probe accepted the exact same conversation twice with no world changes. Both passages passed native validation and were published in the fictional fixture. This does not imply that a real model generated that duplication during the review.

**Recommendation:** add a local advisory warning for exact or substantial copied text against recent passages. Semantic repetition requires separate author review or a bounded, explicitly requested continuity review; it should not become an automatic provider retry or an infallible acceptance gate.

## Conditional loss of the immediate handoff

Claim-access filtering can withhold an entire recent passage if it cites an excluded claim or its effects reference one (`crates/story-project/src/story.rs:808`). The existing knowledge test verifies this behavior. The recent two-passage window is chosen before filtering, so no older passage is substituted into that window. This preserves the intended knowledge boundary and must not be bypassed to improve continuity.

The UI reports omission counts inside the collapsed “Continuity and providers” details (`src/lib/StoryWorkspace.svelte:257`). A prominent explanation when the latest accepted passage is unavailable would make this easier to diagnose. Any alternative handoff must itself obey the same source/access boundary.

Saved custom passage prompts also replace the default instructions rather than extending them (`crates/story-project/src/text_prompts.rs:78`). A customized prompt may lack the current continuation instructions. This review did not inspect private settings, so that remains a diagnostic possibility, not an observed cause.

## Recommended order

1. Make the exact latest accepted source and its inclusion/omission visible before generation, with an inspectable assembled request. Distinguish a prospective preview from captured generation inputs.
2. Give planning the same eligible older memory as writing, within its own budget.
3. Improve reviewed continuity-note coverage and assemble a compact passage handoff/settled-beat section without adding hidden model calls.
4. Add advisory copied-text detection and short fictional multi-passage quality checks.

Increasing the model context window alone does not address the planner's explicit memory removal, unchanged chapter instructions, or missing accepted notes.

## Verification

Existing native suites run successfully:

- `node scripts/cargo.mjs test -p story-project memory:: -- --nocapture`: 19 passed.
- `node scripts/cargo.mjs test -p story-project --lib story::tests::chapters::`: 4 passed; 1 explicit desktop-fixture test ignored.
- `node scripts/cargo.mjs test -p story-project --lib story::tests::planning::`: 2 passed.
- `node scripts/cargo.mjs test -p story-project --lib knowledge::tests::`: 12 passed.

Four additional temporary integration probes passed, covering exact recent prose after reopen plus older-conversation omission, unchanged chapter direction/progress, planner/writer memory divergence, and acceptance of duplicated prose. They used isolated temporary fictional projects and the offline adapter, inspected native prepared requests, and were removed after the review. They assert observed current behavior, not a proposed acceptance specification.

Total: **41 automated checks passed; 1 desktop fixture ignored.** No desktop interaction, new desktop build, live provider generation or real-model quality test was performed. Existing uncommitted application work was preserved. This review does not establish whether the user's running executable or saved custom prompts match this source tree.

## Focused implementation pass

Implemented shared bounded older-memory retrieval for scene/chapter planning, an exact native latest-passage ending with a composer preview and withheld-source warning, chapter-direction framing that avoids labeling old completed beats as remaining work, stronger default extraction instructions for consequential dialogue notes, visible older-history gaps and advisory local copied-text detection. Existing plans/custom prompts, explicit card review/acceptance, immutable history and task/source boundaries are preserved. No new provider calls, automatic summary maintenance or retries were introduced.

Verification on the combined working tree: 382 native tests passed (18 opt-in/helper tests ignored), 116 frontend tests passed, Svelte check and native app check passed, bundled schema validation and production frontend/debug/release Windows builds passed. Eight native continuity regressions were added. The isolated desktop walkthrough confirmed the ending preview, retrieved settled-conversation note and copied-text warning; local validation left acceptance available without advancing the three-passage history. The fresh release testing folder is `build/manual-test-0.1.0-2026-11-08T10-07-47-485Z`.

Older passages without accepted notes/current summaries can still have coverage gaps. Custom prompt text remains user-owned; Restore default adopts the revised task instructions. Copied-text detection is advisory and does not detect semantic repetition. No live-model quality probe was run in this pass.
