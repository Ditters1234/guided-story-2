# Project working guidance

## Authority and scope

Read README.md, DESIGN_REVIEW.md and REVIEW_AND_BUILD_PLAN.md before implementation. Confirmed user decisions take precedence over proposed details. EXISTING_WORK_REVIEW.md records the approved selective reuse direction and its constraints. Keep this as a fresh application; do not silently transplant the original Guided Story engine or a third-party writing application.

## First implementation milestone

Start with Gate 0: Windows Tauri/Svelte shell, a small map/inspector/image preview and validated JSON project create/open/save. Prove interruption recovery, revision publication, duplicate-request handling and project locking before broad feature work. Dependencies and code reuse need their own checks; the planning documents are not evidence of passing builds or runtime behavior.

## Core invariants

- The native backend owns canonical validation and project writes. Autosaved form drafts and AI proposals are distinct from accepted story facts.
- AI world proposals require Apply; prose requires Accept passage. Edited prose invalidates extracted effects and checks.
- AI node authoring derives fields, capabilities and output contracts from the project schema. Keep provider clients/job state separate from node data; custom presets must use the same creation/revision/protection pipeline.
- Accepted passage versions and world snapshots are immutable. Current author edits must not change historical snapshots.
- Publish passage text, effects, chapter membership and before/after snapshots through the same manifest transaction. Manual world commits preserve its narrative reference. Narrative checks fingerprint exact prose/plan/effects and scoped source; ordinary navigation must never trigger generation.
- Use one Fork after this passage action. Copies must stand alone and exclude future story knowledge and pending operations.
- Clothing is inventory with wear assignments; do not introduce separately editable wardrobe strings.
- Use generic world nodes with reusable capabilities, project-local type/field definitions and typed links. Place/character/item/thread are presets, not hardcoded persistence collections. Container is an addable capability; custom nodes must roundtrip without engine changes. Keep built-in inventory/placement validation intact.
- Characters are accessed through the world map/shared inspector. Portrait generation does not redesign canonical facts.
- Scope model context by task, viewpoint and selected resources. Map expansion does not authorize sending the entire world.
- Prioritize approximately 30B/Qwen-class local models with compact scoped tasks and explicit server-compatible sampling/thinking controls. Preserve model-specific ComfyUI graphs; start from the pinned Guided Story Krea 2 templates and use separate graph profiles for Z-Image/MOSS. Do not silently swap loaders, conditioning, custom savers or sampler defaults between families. See MODEL_TARGETS.md.
- Preserve full retained prose on extraction/check failure. Never add unbounded automatic repairs, retries or provider switching.
- Reading navigation, layout and portrait selection must not accidentally advance or rewrite the story.
- Keep the workspace compact and functional. Avoid slogans, repeated headings, development-status copy, placeholder tabs and redundant legends; prioritize room for the map and editing controls.
- Story mode uses chapter/passage/draft navigation on the left and scene planning/review on the right, with prose in the center. World mode uses the world tree and shared inspector.
- Use the shared appearance tokens for UI colors and scalable text. Appearance preferences belong in the app config directory, not story snapshots. Portable themes are validated JSON data, not executable CSS.

## Source, data and verification

Keep upstream reference checkouts out of tracked application files and packages. Record source revisions and preserve applicable third-party notices for actual reuse. Keep private stories, media, keys and local connection settings out of Git. Do not select a project-wide license implicitly through a starter template.

Use isolated fictional projects and deterministic provider adapters for tests. Live provider tests must be short, explicit and separate from normal user data; do not run lengthy generations merely to verify UI/storage changes. Report automated checks, desktop checks and real-model evidence separately. Run checks appropriate to each change, including interrupted-write tests when persistence changes. Do not claim a build or feature works without verifying it.
