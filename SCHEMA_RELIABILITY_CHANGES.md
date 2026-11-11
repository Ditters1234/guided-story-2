# Schema reliability changes and starter-context options

Implemented November 7, 2026 following review of `ARCHITECTURE_COMPATIBILITY_AUDIT.md`. No starter inclusion defaults, supported story format, simulation behavior or saved project schemas were changed. Existing working-tree changes were preserved.

## Reviewed template and relationship follow-up

The subsequent review approved updating the shipped templates rather than building automatic migrations, keeping ownership endpoint checks advisory, adding an explicit local-conditions action, preserving explicit AI creation-type choices and displaying connection direction. These are now implemented through the existing importer, shared editors, map and scene reference controls.

- School, homestead and spaceship now author respectively 29/30/15 explicit two-way routes with access restrictions and 27/14/16 direct local-conditions links with coverage exceptions. Containment remains independent. Homestead outdoor weather is not linked to enclosed rooms or the high tunnel; ship interior conditions exclude the exposed shuttle berth. No reverse edges, possession links, characters or knowledge are synthesized.
- A small optional keyed `edges` section extends the template JSON because the original node/parent-only format could not express these reviewed route/conditions facts. Original templates without it still parse. The importer resolves fresh IDs, checks relationship/endpoint/field paths and runs unchanged full-world validation. Template routes must author a valid direction; existing saved worlds with omitted optional direction remain valid.
- The shared connection editor and inspector advise on unusual ownership endpoints by capability. No new blocking endpoint restriction is imposed and no inventory data is modified.
- **Add conditions to scene context** lists only directly linked environment nodes for the exact selected scene focus, shows coverage and adds a normal reference only on click. It respects the 12-reference limit and permits ordinary removal. No inherited or automatic inclusion is enabled. Existing AI creation-type defaults remain unchanged.
- Map routes gain a source arrowhead only for explicit two-way direction; the inspector shows the same direction. Missing direction is labeled unspecified, and all other relationships retain source-to-target orientation. Display never rewrites canonical endpoints or creates reverse edges.

Current automated results: 354 native tests passed, 17 intentionally ignored; 105 frontend tests passed; Svelte/TypeScript check passed with zero errors/warnings; production build and bundled-schema/preset/template gate passed. Native tests include template create/reopen/commit, ownership versus physical possession, exact scoped conditions, absent-direction compatibility, importer diagnostics and existing recovery/replay/backup/fork/request-size checks. The sandbox initially blocked the default temp directory and six mock HTTP checks; workspace-local temp storage and a permitted run outside network restrictions resolved them. No real provider generation was run. Vite still reports its existing large-chunk advisory; Rust falls back to copying incremental cache files when hard linking is unavailable.

Desktop check: the built debug app against the production UI preview created a fictional homestead, showed the new route/conditions connections, displayed two-way inspector symbols with access restrictions, and added/removed the linked environment through ordinary scene references. The action was initially unselected and did not publish a world revision (revision 0 throughout). No provider or passage acceptance was invoked. The test app used isolated preferences; its verified fictional project was moved from the app's default folder into ignored `build/audit-ui/projects` after closing. A dev-mode Vite watcher encountered a locked build-temp file, so this smoke check used the built preview instead. Pixel-level verification of every canvas arrow and a packaged release run were not performed.

Earlier audit options and counts below describe their respective checkpoints; this follow-up is the current state.

## Review groups

| Finding | Change | Main files |
| --- | --- | --- |
| C1 | Optional relationship descriptions roundtrip; absent descriptions remain omitted in the original serialization order. Core checks ignore only explanatory descriptions and extensible fields; endpoint, placement, single-target and other existing protections stay pinned. | `crates/story-project/src/model.rs`, generated `src/lib/bindings.ts`, constructor fixtures |
| C2 | Native bundled loading returns filename/serde/validation errors. Validation no longer expects core or knowledge JSON to parse. Wearing compatibility reads the pinned stored core shape without consulting optional presets or recursively loading defaults. Bundled data is validated without applying saved-file interpretation upgrades. | `model.rs`, `knowledge.rs`, `location_templates.rs` |
| C3 | Authoring/effects catalogues retain relationship descriptions and field help. Narrative/planning/wording packets explain supplied relation types once, with help/stability for the applicable supplied fields. Connection editing displays meaning; custom relationship creation can author it. Prompt contract versions changed to stale pending work assembled under earlier projections. | `prompt_context.rs`, `projection.rs`, `knowledge.rs`, connection dialogs, authoring/story fingerprints |
| C4 | Starter tests distinguish physical roots/placements from the eight legitimate supporting roots, without requiring fake visual descriptions. Existing independent-copy/create/open/commit checks remain. | `location_templates/tests.rs` |
| C5 | Strict import parsing and assembly identify template IDs, node keys, components, fields, expected kinds and parent keys. Invalid overrides are checked against the complete allocated node index, followed by the normal full-world validator. Native errors include more field paths, and matching node names/IDs are mapped back to template keys. The JSON format is unchanged. | `location_templates.rs`, `model.rs` |
| C6 | Redundant catalogue labels use their identical field key; distinct/custom labels remain. Identical concrete provider field/component contracts share local `$defs` after scoped references are bound. This preserves per-preset capability sets, required fields, limits and scoped identities; it does not introduce an all-capability node branch. Literal `const`/`enum` data is not rewritten. Some repeated guidance was shortened. Existing size ceilings remain unchanged. | `prompt_context.rs`, `authoring/references.rs`, `authoring/creation.rs`, `story_effects/schema.rs` |

`World::default()` remains the infallible convenience used by fixtures/examples; it delegates to validated bundled loading and necessarily expects success. The application importer and the release checker use `World::bundled() -> Result<World, String>`. Runtime contract-validation paths no longer panic on JSON parse errors. No empty-world fallback was introduced.

The build gate is the small `check_bundled_schemas` native binary using the real runtime structures and validators. It parses/validates the bundled world, instantiates every preset's defaults and assembles/validates every location template. `npm run build` invokes it before Vite; Tauri's normal desktop build already invokes that script. Manual packaging invokes it independently before creating the output folder. No new dependencies or duplicate schema validator were added. Direct development `cargo check` is a compiler check, not this data preflight; run `npm run schema:check` independently when needed.

## visualDescription verification and correction

It previously entered actual passage prompts as `records[].visualNotes`. Wording also supplied complete accepted PutNode effects containing `visualDescription`; explicit context bindings could include it as well.

The ordinary narrative/planning record projection now omits those extra image notes. Wording omits that field only from its outgoing accepted-effects projection, retaining the complete original effects and historical records. An explicit narrative binding that selects the image-only field returns an actionable preflight error rather than silently dropping required material or rewriting the binding. The editor explains this restriction. Image appearance/prompt preparation still uses the value. World-authoring and effects-review tasks can still receive complete records to preserve them; they are author-review tools rather than prose generation.

Structured fields explicitly marked Story + Visual remain available to narrative tasks as declared; this correction concerns the supplementary `visualDescription` property. Information copied into ordinary story notes, prose or instructions cannot be semantically classified by its original source field.

Tests inspect actual passage, planning and wording request payloads with a unique visual sentinel, check that it remains available to image projection, and verify that canonical accepted effects still contain it. Separate tests cover explicit binding rejection without modifying saved bindings.

## Request measurements

Bytes include system/user messages and separately supplied response schema, using the existing fixtures. The earlier audit measurements removed unsupported descriptions in an ignored copy; the current measurements retain those descriptions and guidance.

| Task | Audit measurement | Current | Existing ceiling |
| --- | ---: | ---: | ---: |
| World creation, schema transport | 46,396 | 44,251 | 46,000 |
| Effects, JSON transport | 69,405 | 58,378 | 60,000 |
| Effects, schema transport | 69,387 | 58,360 | 60,000 |

World JSON is 36,260 / 36,500; ordinary prose is 9,899 / 10,200; planning is 5,411 / 5,700. These are deterministic size/preflight results, not evidence of real-model quality. Local `$defs` retain concrete schemas; a real configured constrained decoder still needs its usual short compatibility probe. No provider switching or retry behavior changed.

## I1 investigation — options for review, not implemented defaults

Follow-up: Option B is now implemented as **New story → Review starter AI context…**. Every proposed source starts disabled; the author enables and reviews bindings with the existing controls/native previews before using them. Only selected bindings enter the normal new-project transaction. The options below retain the rationale from the original investigation; no automatic inclusion, template-format extension or existing-project migration was introduced.

Follow-up verification: 102 frontend tests pass across 18 files, with Svelte/TypeScript checks and the production build/data preflight passing. The new tests cover all starter catalogues, unchanged node facts/schemas, explicit selection, rereview and native-preview failures. An isolated desktop check verified initially disabled choices, a native time-reference preview, Cancel discarding edits, confirmed selections surviving folder-picker cancellation, and a created 46-node starter with exactly its selected binding on the original time node. The app's normal AI context manager displayed that saved binding. The fictional test folder is retained under ignored `build/starter-context-ui/projects/`; no real-provider calls were made. Native persistence code was unchanged by this follow-up.

Imported supporting nodes persist and edit normally, but starter assembly creates no bindings. Scene selection adds selected references, claims, inventory and physical ancestors. A `conditions-at` edge alone does not select its source environment; neither a time preset nor a narration/style preset automatically grants inclusion.

| Record | Appropriate existing mechanism | Boundary to retain |
| --- | --- | --- |
| Time | Reference binding for Passage/Planning and optionally Wording, or an explicit scene reference | A binding supplies text without granting effects permission. Select the existing time node as a scene reference when an evidenced, reviewed update is intended. No clock advancement. |
| Local environment | Prefer an explicit scene reference; include its place and any authored conditions-at link in the same scoped scene | Do not apply outdoor weather to every room or inherit conditions automatically. A whole-story Reference binding is suitable only when its authored scope is truly global. |
| Narration style | Instructions for Passage/Wording, matching existing creation-shortcut defaults | Content is author guidance, not story state or a character's knowledge. |
| Visual style | Instructions for image preparation and Image prompt, matching existing defaults | Keep it out of passage/wording tasks and physical appearance facts. |
| Rules | Instructions for selected Passage/Planning/Wording tasks when they are author constraints; Reference when they are descriptive fictional facts | Use only the intended authored scope. Do not infer knowledge or authorize changes merely through inclusion. |
| Lore | Bounded Reference binding for selected narrative/planning tasks, or a scene reference | Bind only lore suitable for the chosen narration. Withheld knowledge belongs in the existing claim system. |
| Brief | Optional Instructions for Planning; Passage only if explicitly intended as creative direction | Keep intentions distinct from established events. |

The binding's field controls apply to that binding across its selected tasks. Do not select Extra visual notes on a binding that also targets narrative tasks. Ordinary node image facts can use the node's visual notes independently of story-wide inclusion.

**Option A — configure existing controls; no new code.** Keep starter defaults as they are. After creation, configure narration/visual bindings using the existing AI context manager, select a time reference/binding, and add the appropriate local environment/rule/lore references for the scene. This is the smallest option and exposes every task/role choice. It requires manual setup for each starter.

**Option B — opt-in starter guidance using current bindings.** Add a small reviewed choice during creation to include the starter's existing narration and visual nodes with the same tasks as today's shortcuts. Offer time/global rule/lore inclusion as separately visible choices; leave local environment as a scene reference. Apply those choices to the returned new world before the normal native create/validate operation. No duplicate nodes or template-format extension is necessary. This is the recommended small convenience if repeated setup is undesirable; it is not implemented yet.

**Option C — optional template-declared bindings.** Add an optional key-based binding section to the template format, allocate its node references with existing IDs and validate it with the world. This permits template-specific roles/tasks but adds format/parser/test surface. It is unnecessary for a shared starter opt-in and should wait for a demonstrated need. Arbitrary typed template edges would be a separate explicit extension, not a reinterpretation of `parent`.

None of these options needs automatic weather, clocks, physical-parent requirements, graph-wide context expansion or a new subsystem. Tests now lock down the unchanged defaults and demonstrate that explicit scene references include time/environment and their supplied condition link.

## Verification

- Complete native suite: **351 passed, 0 failed, 17 intentionally ignored**; binary/doc-test targets also pass. Includes schema compatibility, import failures, protection, snapshots, receipt replay, interruption recovery, backup/fork and prompt-payload tests.
- Frontend suite: **98 passed across 17 files**.
- Svelte/TypeScript: **zero errors/warnings**.
- Native desktop compile check and generated bindings: passed.
- Production frontend build, including the native bundled-data gate: passed. Vite retains its existing large-chunk advisory; Rust reports an incremental hard-link fallback in this environment.
- The six localhost HTTP fixture tests initially failed under restricted networking. They all passed when the native suite was allowed to access its local fixture servers. They did not call real LLM or ComfyUI services.
- Desktop interaction walkthrough, release executable packaging and real-model/ComfyUI compatibility probes were not performed. The ignored tests include explicit live probes and child-process helpers; interruption parent tests did run.

Local verification logs are in ignored `build/reliability-checks/`, including `native-complete.txt` and `request-sizes.txt`. The original audit remains a record of pre-fix findings, not the current verification status.
