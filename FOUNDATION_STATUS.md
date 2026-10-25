# Desktop foundation checkpoint

October 25, 2026. Early implementation of Gate 0 and a small part of manual world authoring. This is not a completed story engine or a release-ready application.

## Implemented

- Windows Tauri/Svelte shell with resizable Paneforge panes, Svelte Flow canvas, searchable tree and shared inspector.
- A shared world-node model. Places, characters, items and threads are presets; nodes carry capabilities and typed links. Project-local type, field and relation definitions are included in saved world snapshots.
- Custom node preset creation with optional inherited capabilities and custom text fields. Schema-driven inspectors support text, numeric, Boolean and node-reference properties; broader schema-authoring controls remain future work.
- Addable capabilities, including inventory, containers and wearability. Built-in semantic validation remains enforced regardless of a node's display type. Container is an extension of a node, not a separate persistence collection.
- New/open project commands, manual autosave, recoverable invalid form drafts, immutable world/manifest records and checksum-validated JSON reads. Readable app example data is fictional.
- OS project locks, stale-revision protection, idempotent request receipts, flushed same-filesystem head publication, previous-head recovery and independent layout versions. A closed/moved project has no dependency on its old absolute location.
- Tauri Specta-generated TypeScript command contracts, using a published compatible RC set. No hand-maintained duplicate canonical frontend schema.
- Local image preview in the character inspector. It is explicitly temporary; it does not save portraits or call ComfyUI.

## Verification

- Native storage/domain suite: **17 tests passed**, plus one ignored helper test invoked by the interruption test as child processes. Covers five actual process-exit boundaries, uncertain-acceptance replay, failed publication, locks, moved folders, damaged-head recovery, checksums, invalid drafts and custom schema roundtrips.
- Frontend domain projection: **4 tests passed**. Covers collapsed contents, nested search paths, explicit worn/carried distinctions and custom typed relationships.
- Svelte/TypeScript check: **0 errors and 0 warnings**. Frontend production build passed.
- The Windows native app was compiled and launched. Typed bindings were generated through its executable. Native UI checks verified the map/inspector, native folder picker opening, and creation of a Weather system preset with Forecast/Intensity fields. Renaming its node to River storm and editing Forecast saved the expected data in an isolated JSON project at revision 3.
- Desktop automation was stopped on the user's physical Escape keypress. A full UI restart/reopen walkthrough and the later close-permission adjustment were not reverified through UI automation. Core reopen/moved-folder tests pass; do not present those as equivalent to complete UI coverage.

No real LLM or ComfyUI requests were made. No personal story library or original Guided Story data was changed. The prototype is not yet an installer; native release packaging, scale/accessibility checks and power-loss behavior need separate validation.

## Next slice

Finish Gate 0 with the remaining native UI lifecycle checks, then build the reviewed idea-to-foundation and first-passage pipeline over this node toolkit. Add richer character capabilities and explicit relationship/movement tools without introducing a second cast editor or fixed entity collections. Follow the main build plan for acceptance, historical state, memory, media and passage forks.
