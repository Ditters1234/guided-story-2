# Passage authoring and history

The Story workspace uses the same composer for the first passage and every later passage. Its left pane holds chapters, passages and saved drafts; the center is for prose; the right pane holds scene planning, world-change review or accepted-passage details. Switching to World restores the world tree/map/shared inspector. This is a bounded prototype, not yet the full editing/media/memory design.

## Write and review

1. Open **Story** and edit the chapter title/direction, passage title, viewpoint, location, participants, references and word target. New passages inherit an open chapter's plan; closing a chapter offers a new editable plan on the next writing action. Opening a composer makes no model request.
2. Write prose manually or choose **Generate passage**. Generation makes one explicit narration request using the configured connection. **Generate new candidate** preserves the existing draft and creates a separate candidate.
3. Edit the retained prose, then choose **Check world changes with AI** for one explicit extraction request. Alternatively edit effects JSON, or choose **Use no world changes**, followed by **Validate my effects**. Manual validation makes no API call.
4. Review proposed creations/updates, links, evidence and continuity notes. Inspect/edit the effects JSON for complete field values. The AI can suggest a chapter ending; **Close chapter after this passage** remains the author's choice.
5. **Accept passage** publishes prose, chapter membership, notes and the resulting world together. Generated prose, saved drafts and successful checks do not advance the established story by themselves.

Drafts autosave separately from accepted records, including incomplete titles. Generate/Validate enforce usable titles and a 100–1,500 word target. The word target guides generation; it is not a minimum-length acceptance rule. The offline demo is short deterministic sample text, visibly labeled, and makes no model request. Context details retain narration/effects provider labels; no credentials are stored in the story.

Editing prose, plan or effects invalidates its check immediately. Changes to selected world context, recent history or connection settings also block acceptance until rechecked. Changing only the chapter-closing choice does not need another model request. Explicit manual validation checks against the current world; inspect any stale effects before deliberately validating them again. Acceptance revalidates against the latest world and preserves unrelated current edits.

## Scope and effects

Context contains selected viewpoint/participants/location/references, their placement ancestors and directly carried/worn items. It does not expand every container or send the entire map. The current limits are 12 participants, 12 references and 80 contextual nodes. Recent context is the last two accepted passages plus their notes and the active chapter plan. Older-story summaries and character-knowledge filtering are pending; this is not yet adequate memory for an indefinitely long story.

Effects use the project's generic nodes, capabilities and typed links. They can update scoped nodes, introduce up to 20 new nodes using temporary aliases, add/move/remove links and retain up to 12 continuity notes, within an 80-change batch. Existing node types/capability sets cannot be redesigned by extraction. Schema changes and node deletion use the manual world tools. Placement/wear changes receive the same native validation as manual editing, including storage reconciliation and cycle checks.

Every effect/note needs an exact evidence quotation of 8–600 characters from that passage, ignoring whitespace differences. Evidence matching and structural validation cannot prove that an interpretation is correct or complete. Review remains necessary. This slice does not implement general pinned-fact protection or semantic contradiction certification.

## Read, inspect and fork

Chapter **Contents**, **Previous** and **Next** navigate accepted prose. Navigation never generates text. Each passage retains immutable before/after world references. Its **Before/After** controls show those snapshots in the shared tree/map/inspector with a read-only banner. **Current world** returns to current editing. Historical selection, expansion and layout do not save over current layout or facts.

**Fork after this passage** asks for a destination parent folder and creates an independent story, keeping the chosen passage inclusively and its after-world. It copies retained text and required historical world records, rebuilds chapter plans/closing status from retained passages, allocates a new story/operation namespace, and excludes later accepted passages, later manual edits, pending drafts/proposals and app credentials. Entity/passage IDs and original provenance remain stable. A staging folder is validated before publication; failure leaves the source untouched and any incomplete staging folder identifiable. No pre-first-passage fork or operation-level undo is introduced.

Accepted wording edits and latest-passage event replacement are still pending. Earlier-event changes continue to use an independent fork.

## Persistence and interruption

New projects start with zero passages. Existing world-only projects load without conversion; their optional story reference remains absent until the first acceptance. New accepted story indexes and passage/world records are checksummed immutable JSON. The manifest publishes their references through the existing atomic head protocol. Manual world saves preserve the narrative reference. Receipts and the accepted index reconcile an acceptance whose final draft-status write was interrupted, including after subsequent unrelated world work.

Saved candidates have independent versions and archived prior drafts. One text request can run at a time across world proposals and passages. Narration is persisted before extraction; failed, cancelled or truncated extraction retains prose and original returned output. A source change during extraction cannot silently rebase its check. Reopening marks unfinished jobs interrupted and never resends them. Results blocked by a disk-write failure remain in the running app, with project switching/closing blocked until saved; abrupt termination during that failure can lose the in-memory result.

The index currently allows 5,000 accepted passages and 1,000 chapters; up to 200 unaccepted drafts can be retained. Draft archives have no cleanup interface yet. Historical references are checked when read/forked, rather than eagerly loading every past world at project open. No image assets or long-story summaries exist in this slice.
