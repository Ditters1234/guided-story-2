# Review and build plan

Initial review October 25, 2026; node-model revision October 28. Companion to [DESIGN_REVIEW.md](DESIGN_REVIEW.md). This is a design review, not evidence of a working application. Confirmed user decisions in the main proposal take precedence; technical defaults here guide implementation and may change if a feasibility check fails.

Recent dependencies and design references are assessed separately in [EXISTING_WORK_REVIEW.md](EXISTING_WORK_REVIEW.md), including exact inspected commits and reuse boundaries. Evaluate the desktop shell and typed bridge at Gate 0; retain independent story storage and acceptance semantics.

Implementation has begun; see [FOUNDATION_STATUS.md](FOUNDATION_STATUS.md) for completed checks and remaining Gate 0 work. The user's toolkit refinement is authoritative: named entity categories are presets over generic nodes/capabilities/typed links, not fixed persistence collections.

The foundation pass resolves relationship pinning as canonical attachment, with editable endpoints and native-validated removal/transfer previews. [WORLD_EDITING.md](WORLD_EDITING.md) records the implemented current-world semantics and limits. Node removal keeps dependent contents unplaced and preserves old saved snapshots; future passage forks remain the recovery/navigation design.

Shared [AI world authoring](AI_AUTHORING.md) and the bounded [passage/history/fork slice](STORY_AUTHORING.md) connect Gate 2's core path. Story mode dedicates its left pane to chapters/passages/drafts and its right pane to planning/review. Gate 3's initial wording and event-replacement paths preserve immutable versions and exact-source review. Gate 4 now has [chapter memory](MEMORY.md): reviewed summaries, older-note retrieval, visible gaps, budget limits and source-aware invalidation/forks. Initial [reviewed media](MEDIA.md) and optional LLM shot/prompt suggestions are also implemented. Validated folder backup/restore is implemented with full saved history, drafts and checksummed images. Belief filtering, further compaction and real-provider validation remain pending. See the foundation checkpoint for actual evidence and limits.

[Model targets](MODEL_TARGETS.md) records the confirmed approximately 30B/Qwen text priority and Krea 2/Z-Image/MOSS image direction. The pinned Guided Story Krea workflow now feeds the initial [media service](MEDIA.md): explicit preflight/submission/result checks, reviewed project-local assets, historical source facts and retained-history fork copies. Prompt assistance uses a separate bounded text request and explicit Apply to the image draft. The initial [folder backup/restore](BACKUPS.md) preserves retained history, media and saved drafts. Real-server/model compatibility remains pending. Keep task contracts compact before adding reasoning, longer windows or additional stages.

## Verdict

The prototype now has a bounded authoring/history/media loop; the next design priority is richer composable nodes. The major risks remain save integrity, state/version consistency, small-model context quality and an authoring interface becoming too crowded. Retain the existing acceptance/storage boundaries while improving what nodes contain and how tasks use their facts.

Keep these decisions fixed: fresh Windows-first Tauri/Svelte app; user-selected self-contained JSON projects; map-based character access; item-based clothing; optional portraits and passage illustrations; idea-to-reviewed-foundation and continuing manual/AI creation/revision/reroll; explicit Apply/Accept; automatic manual/draft saves; chapter contents and reading navigation; one Fork after this passage action.

## Next revision: composable world nodes

[WORLD_NODE_DESIGN.md](WORLD_NODE_DESIGN.md) is the detailed specification and code audit for this pass. Basic behavioral capabilities stay small; standard creation presets bundle useful descriptive extensions. Definitions drive the generic editor, AI contracts and task projections. A node is not a separate character/place/item persistence class, and its preset label cannot suppress other attached capabilities.

The initial capability/default-preset slice uses story format 2. The user permits starting fresh; no migration/import or compatibility work for older prototype stories is required. Existing folders are not overwritten or deleted automatically. Immutable passage/snapshot history, backups, forks and receipt recovery within the supported format remain mandatory.

Implement in these ordered slices:

1. **Capability contract:** field purpose/section/task metadata, choice/unset support, capability composition/removal, native behavioral registry separate from the optional preset catalogue, fresh-format fixtures.
2. **Useful defaults:** Character profile/personality/appearance/motivation/voice/state; Place/Item/Thread structure; shared compact inspector and search; checked-field manual/AI editing.
3. **Relationships and projections:** typed link properties, scoped relationship review, shared story/image fact assembly and media fingerprints. Structured appearance and garment details must actually reach image tools.
4. **Wider core catalogue:** Organization, Lore, World rule, Story brief, Visual style and source-linked Event; preserve the existing Narration style binding behavior. Presets are available, not automatically instantiated.
5. **Situated knowledge:** source-linked claims, character stance/acquisition and reader disclosure, with task/viewpoint/history filtering. Do not present a secret/known-by control as protective before the full context path exists.

Each slice must cover native validation, UI, AI schemas/protections, context fingerprints and history where relevant. Use the fictional end-to-end fixture and completion criteria in the node specification. Richer schemas alone do not establish narrative quality. Keep advanced genre bundles, live inheritance, faction simulation and workflow engines outside this pass.

Current checkpoint: the capability/default bundles, typed custom forms and shared task/media projections are implemented, together with Organization/Lore/World rule/Story brief, typed relationships, Event/source annotations and Visual style bindings. Initial shared claims, character stances and narration-access filtering are implemented with explicit author controls; automatic acquisition/disclosure inference remains later work. See [verified evidence](FOUNDATION_STATUS.md).

## Findings resolved in the proposal

| Priority | Gap found | Resolution |
| --- | --- | --- |
| High | Editing generated prose could leave accepted effects from an earlier draft. | Hash/version exact prose, source world, plan and applicable settings; edits invalidate downstream checks. Accept only a matching checked candidate. |
| High | One coarse revision would make layout or portrait changes invalidate unrelated writing. | Separate committed project revision from generation-source fingerprints and independent workspace/draft versions. Revalidate and merge only explicitly independent metadata changes. |
| High | JSON atomicity was described, but its tests appeared at the end of delivery. | Prove head publication, process interruption, request deduplication and project ownership locking before feature work. |
| High | The save system did not explicitly separate autosaved valid records from invalid or unaccepted candidate edits. | Maintain distinct canonical, form-draft and AI-proposal stores; autosaving a proposal never applies it. Multi-record operations remain atomic. |
| High | Thin presets hide story-relevant information in general notes; richer fields could still be ignored by image/context code. | Bundle reusable extensions and update editor, AI protection/schema, task projections and source fingerprints together. See the node revision's source audit. |
| High | Comparing every saved world to the entire current default catalogue turns new optional presets into mandatory schema changes. | Separate native behavioral contracts from optional project-local catalogue definitions; use an explicit new format for this revision. |
| High | Historical image moments and forks could leak future state. | Passage before/after snapshots are authoritative boundaries; intermediate image facts require source evidence/review. Fork plans, memories, media and settings must be anchored to retained history. |
| Medium | Depletion and deletion behavior was unspecified. | Explicit stock units, conserved transfers and inactive zero-stock records; resolve current dependents before retiring holders. Preserve historical IDs. |
| Medium | Contradiction detection and wording-only polish sounded infallible. | Structural validation is strict; semantic review is fallible and advisory. Show exact diffs and source links; event edits use the event-revision path. |
| Medium | Chapter-plan generation could become an extra hidden LLM call. | Opening the composer uses inherited local defaults; an AI planning request is explicit. |
| Medium | Templates, presets and extensive visual controls could delay the core writing loop. | Preserve them in the design, but implement convenience layers after the first complete reliable workflow. |

## Minimal architecture

- **UI:** Svelte + TypeScript + Vite. Prototype Svelte Flow for the map; collapse/expand is our own data projection, not a second copy of world records. Keep the searchable tree usable throughout.
- **Native backend:** Rust/Tauri owns project access, validation, commits, provider requests and durable job lifecycles. UI calls typed commands and receives progress events; it does not write canonical JSON directly.
- **Shared contracts:** versioned Rust types/validation with generated frontend bindings or a checked equivalent. Avoid independently maintained incompatible frontend/backend schemas. Backend validation is authoritative.
- **Services:** persistence, world editing, authoring proposals, context/memory, prose generation, media and forks have clear module boundaries. Avoid a generic plugin/workflow engine.
- **Testability:** text/image providers sit behind adapters with deterministic fake responses and cancellation. Native tests should exercise save and generation behavior without launching the UI or using real models.

The Svelte Flow/WebView2 rendering and Windows filesystem behavior need small feasibility probes. Do not claim measured speed, package size or large-world performance before those probes. If a supporting library fails, retain the product behavior and replace the implementation choice.

## Version and state contracts

1. Every persisted object has a schema version and stable story-local ID. A story has one authoritative committed manifest; accepted text and world snapshots are immutable versions.
2. The project revision serializes commits. Generation additionally records hashes of its actual world/settings/plan/source inputs. Reading position, map layout and choosing a portrait are not world changes.
3. World edits and narrative-relevant changes stale a dependent proposal. Independent media/layout updates can coexist, but acceptance always loads the latest manifest and preserves unrelated metadata; never overwrite a newer manifest wholesale from a stale UI copy.
4. Candidate editing has its own version. Scope, protected fields and kept manual candidate edits are explicit; reroll does not erase saved facts or protected candidate work.
5. Debounced manual autosave commits a valid record or explicit multi-record operation, not each transient keystroke. Invalid inputs remain drafts. Before generation, flush valid pending changes; if a save fails, stop and retain typing.
6. Protection is enforced for AI effects and proposals. Deliberate manual author edits remain possible, with visible protection status and current validation. No hidden unpinning by a model.
7. Past-state inspectors are read-only. Writing commands act on the current source or a named supported replacement baseline, never the passage being browsed by accident.
8. Current world truth and character knowledge differ. Background references and plans establish neither physical access nor known facts. No offscreen simulation is required.

### Prose run lifecycle

Use queued → running → checking → ready-for-review → accepted/discarded, with blocked/failed/cancelled/interrupted/stale states as appropriate. Retain complete prose as soon as it exists, before effects extraction. Editing prose returns it to needs-checking. An accepted proposal cannot be accepted twice or edited in place.

Start with one active text-provider request per project and a separate optional image job. Multiple saved candidates may exist, but only a candidate checked against the current applicable source can be accepted. The project remains browsable/editable during requests; edits can stale a result. Closing the app preserves drafts and marks unfinished text work interrupted; do not silently resend a request on restart.

The ordinary prose path is one narration request plus one effects/memory extraction request. Optional short-text expansion, targeted continuity review and one repair must be visible, bounded and cancelable. Initial automatic ceiling: six text calls per passage attempt (narration, at most one expansion, extraction, triggered review, at most one repair and at most one re-review). Skip inapplicable stages. Test these counts; never introduce unbounded loops, automatic provider switching or hidden network retries. A manual recheck is a new explicit attempt with its own bounded budget, using retained prose. World proposals similarly have declared bounded stages and retained artifacts. A large initial world may be built through explicit staged requests rather than one oversized response; its final records still receive user review.

Prose may introduce new characters, places or items. Extraction returns temporary references for new records; the backend allocates IDs, resolves cross-references and presents creations with other effects. Require prose evidence for established changes. Mentioning a person/item does not establish presence or possession. Missing or ambiguous stock amounts cannot become invented exact updates.

### Context and long-story memory

Use deterministic selection first: current viewpoint and participants, location path, relevant carried/worn resources, selected references/threads, required author constraints and recent prose. Fetch only relevant container contents rather than every descendant. Keep a compact entity directory to resolve mentions without supplying all profiles. Narration, extraction and image preparation have different projections.

Reserve response/schema overhead before optional context. Required constraints that do not fit block with a useful report. Do not increase the context window or add model calls as a substitute for fixing selection. Debug views show selection reasons and estimated budgets.

Source-linked passage notes and reviewed completed-chapter summaries now provide bounded older context, with coverage and stale-state tracking. Further compaction remains pending. Summaries never override current structured facts. If memory maintenance fails, retain accepted prose, mark coverage incomplete and offer retry; no silent claim of complete history. Forks omit future coverage and polish invalidates affected sources. This is part of the first-release plan, not a promise that bounded recall supports a novel indefinitely.

### Media contracts

Resolve shot facts from explicit current state or the selected accepted passage version. Present before/after as a default source, with author-reviewed overrides for intermediate moments. A generated picture is not proof that an unrecorded fact occurred. Prompt preparation is optional and explicit; capture the exact final prompt/workflow and endpoint at submission.

Accepted media metadata and portrait selections are referenced by immutable manifest versions and a separate media fingerprint. Image completion does not change a world snapshot or stale text generation by itself. Historical portrait applicability is based on captured appearance, not only upload date. Unknown applicability uses a placeholder or clear historical mismatch indication.

After restart, an accepted ComfyUI prompt ID can be checked at its original endpoint. If submission outcome is unknown, show that uncertainty; do not submit another image automatically. Image rerolls create new candidates. Keep actual image formats, stable relative asset paths and bounded image previews.

## Fork boundary checks

The sole action keeps the chosen passage inclusively and its saved after-world. It does not include subsequent manual edits, discoveries, plans or generated candidates. Keep the applicable settings and plan versions captured for that boundary; current connection profiles remain app settings. Keep accepted text versions and media intentionally associated with retained history, preserving provenance and excluding later story-only content.

A fork gets a new story ID and a new operation namespace; story-local entity IDs may remain stable. Materialize a self-contained folder, validate all references and only then register it in the library. A failed copy leaves the source untouched and no apparently complete destination. Opening the copy must work with the source folder unavailable.

There is intentionally no pre-first-passage fork mode. Latest-passage replacement can rewrite the first passage while its baseline remains valid. Recovery of arbitrary pre-story manual edits is not a promised global undo feature; preserve backups/drafts, but do not silently add another fork interface.

## Screen responsibilities

1. **Library/open project:** new idea, start manually, open folder and recent stories; show loading/recovery failures without replacing data.
2. **World workspace:** focused expandable map + searchable tree + one entity inspector. Same inspector handles canonical records and visually distinct proposed records. No cast management screen.
3. **Composer/reader:** shared center area switches among scene plan, draft review and accepted prose. Chapter contents and Previous/Next remain available. Historical-state banner and Current world action keep context obvious.
4. **Proposal review:** compact field diff for a name reroll, entity preview for a redesign and dependency-aware grouped preview for a bundle. Use Apply consistently for world proposals and Accept passage for prose.
5. **Images:** portrait controls in the character inspector; Illustrate in the passage reader; common shot/prompt editor and job status.
6. **Settings/activity:** connection tests, model/workflow configuration, narrative defaults and inspectable jobs/errors. No mandatory developer controls in the ordinary writing flow.

## Delivery gates

| Gate | Deliverable | Must pass before broadening |
| --- | --- | --- |
| 0: Desktop and storage proof | Windows Tauri window; map node/inspector/image preview; JSON project create/open/save. | Process interruption at each commit boundary, locked project, failed replacement, duplicate request and moved-folder tests. |
| 1: Manual world | Zero-passage project, places, characters, item garments, wear/storage and autosaved drafts. | Nested containers, layered outfits, quantities, invalid drafts, protected fields and historical snapshot immutability. |
| 2: First complete authoring loop | Idea proposal or manual world → review → write passage → checked acceptance → contents/navigation → fork. | First and later passages use one pipeline; blocked extraction keeps prose; source edits stale candidates; fork opens independently. |
| 3: Continuing AI tools and chapters | Field/entity/bundle create/revise/reroll; next-chapter plans; latest replacement and wording polish. | Scope preservation, partial dependency validation, text-edit rechecks, chapter transitions and evidence invalidation. |
| 4: Images and sustained stories | Portraits, passage illustrations, historical shot facts, chapter memory, export/backup and restore. | Historical outfit accuracy, image restart recovery, memory coverage/invalidation, self-contained restore and chosen real-provider compatibility. |
| 5: Release readiness | Windows installer, keyboard/focus behavior, large fictional fixture and complete lifecycle walkthrough. | Clean install/open/restart/fork on Windows; bounded context/calls; accessibility and representative model/workflow probes with isolated data. |

No lengthy live-model runs are needed to prove storage or domain behavior. Test providers deterministically first, then use a small isolated local LLM/ComfyUI probe for actual wire compatibility. Report automated tests, UI checks and real-provider evidence separately.

## Acceptance walkthroughs

- **Foundation:** a premise creates an editable world; revise one character and room; restart before applying. Draft survives, and no passage or image has been created.
- **Character bundle:** generate a clothed herbalist with a satchel and remedies; reroll one garment; apply. Only intended fields change and every item has valid storage/wear links.
- **State transition:** accept a passage handing over a coat and consuming one counted remedy. Before/after views, current inventory and outfit projection agree; depleted stock remains historically traceable.
- **Changed candidate:** edit a checked passage's handoff into a different event. Accept stays disabled until that exact text is rechecked; old effects cannot commit.
- **Concurrent edits:** move a node/select a portrait during generation; valid text remains usable. Change a relevant garment while it runs; old world checks cannot be accepted.
- **History:** polish an old passage, inspect its image and fork after it. No future memory or outfit leaks into the independent copy.
- **Chapter:** keep a chapter open, then close it with a passage; Next navigates written text only and the next writing action offers the chapter plan without an automatic LLM request.
- **Interruption:** fail extraction, fail a JSON head replacement and restart after uncertain acceptance. Prose is retained; the accepted world is whole and committed once.
- **Portability:** move/export the project and open it without the original directory or private connection configuration. Accepted text, historical states and images remain accessible.

## Explicitly deferred

Mac/Linux releases, browser/mobile access, collaboration/cloud sync, generic workflow editing, geographic maps and travel simulation, faction simulation, full in-place historical event rewrites, operation-level undo, old Guided Story import, older prototype story migration, automatic reference-image conditioning, elaborate genre/template libraries and storage garbage collection. A descriptive Organization preset is part of the node revision; it does not imply a faction simulation. Simple local outfit presets and reusable character templates follow the core loop.

## Readiness boundary

The initial capability/default-preset, typed-relationship, Event/source, Visual style/direct-image and situated-claim slices are implemented. Continue with end-to-end manual testing and remaining explicit authoring gaps; automatic acquisition/disclosure proposals are separate work. Representative provider validation remains explicit later evidence, not an assumption supplied by a design document. The foundation checkpoint distinguishes tested storage/domain/UI behavior from real-provider compatibility, sustained-story quality and release readiness.
