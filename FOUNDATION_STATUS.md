# Desktop foundation checkpoint

October 25, 2026. Early implementation of Gate 0 and a small part of manual world authoring. This is not a completed story engine or a release-ready application.

## Implemented

- Windows Tauri/Svelte shell with resizable Paneforge panes, Svelte Flow canvas, searchable tree and shared inspector.
- A shared world-node model. Places, characters, items and threads are presets; nodes carry capabilities and typed links. Project-local type, field and relation definitions are included in saved world snapshots.
- Custom node preset creation with optional inherited capabilities and custom text fields. Schema-driven inspectors support text, numeric, Boolean and node-reference properties; broader schema-authoring controls remain future work.
- Addable capabilities, including inventory, containers and wearability. Built-in semantic validation remains enforced regardless of a node's display type. Container is an extension of a node, not a separate persistence collection.
- Native-validated structural previews: create/edit/move/detach links, remove nodes while retaining contents, clear typed references, and remove capabilities after resolving dependent connections. Wear/possession transfers and custom physical links use the same atomic rules. See [world editing](WORLD_EDITING.md).
- Connection controls in the shared inspector, canvas connection creation, stable canonical canvas edge IDs, project-local relationship definition creation, and ancestor revelation when following connections. Thread grouping cannot create projection cycles; deep layouts/tree ordering use iterative traversal.
- New/open project commands, manual autosave, recoverable invalid form drafts, immutable world/manifest records and checksum-validated JSON reads. Readable app example data is fictional.
- OS project locks, stale-revision protection, idempotent request receipts, flushed same-filesystem head publication, previous-head recovery and independent layout versions. A closed/moved project has no dependency on its old absolute location.
- Tauri Specta-generated TypeScript command contracts, using a published compatible RC set. No hand-maintained duplicate canonical frontend schema.
- Local image preview in the character inspector. It is explicitly temporary; it does not save portraits or call ComfyUI.

## Verification

- Native storage/domain suite: **27 tests passed**, plus one ignored helper test invoked by the interruption test as child processes. Covers five actual process-exit boundaries, uncertain-acceptance replay, failed publication, locks, moved folders, damaged-head recovery, checksums, invalid drafts, custom schema roundtrips, movement/removal semantics, custom containment cycles and malformed saved layouts.
- Frontend domain projection: **9 tests passed**. Covers collapsed contents, nested search paths, wear versus possession, retained canonical edge IDs/direction, custom relationships, safe thread grouping, ancestor revelation and a 1,500-node containment chain.
- Svelte/TypeScript check: **0 errors and 0 warnings**. Frontend production build passed.
- The Windows native app was compiled and launched. Typed bindings were generated through its executable. Native UI checks verified the map/inspector, native folder picker opening, and creation of a Weather system preset with Forecast/Intensity fields. Renaming its node to River storm and editing Forecast saved the expected data in an isolated JSON project at revision 3.
- This foundation pass used a separate copy of the fictional verification project. The Windows UI opened the connection editor, changed both endpoints and its notes, reviewed/applied the native preview and autosaved revision 8. Reading the resulting JSON confirmed the new endpoints/notes. Clean Alt+F4 shutdown completed, and reopening retained revision 8 and the incoming relationship; the project-opening map fit was also checked visually. This supplements, rather than replaces, the native process-interruption tests. Destructive controls and every custom definition/drag interaction have not all been exercised through desktop automation.

No real LLM or ComfyUI requests were made. No personal story library or original Guided Story data was changed. The prototype is not yet an installer; native release packaging, scale/accessibility checks and power-loss behavior need separate validation.

## Next slice

Build the reviewed idea-to-foundation and first-passage pipeline over this node toolkit. Add richer character capabilities without introducing a second cast editor or fixed entity collections. Follow the main build plan for acceptance, historical state, memory, media and passage forks. Broader UI interaction/accessibility and packaged-release checks remain necessary; a prototype foundation pass is not release certification.
