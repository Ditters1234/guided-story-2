# Schema architecture and compatibility audit

Reviewed November 7, 2026 against the current working tree, including the uncommitted preset/template changes. Production code and schema files were not changed. The pre-existing working-tree edits were preserved. Diagnostic experiments live only in ignored `build/architecture-audit/` and `target/architecture-audit/`.

The existing architecture is suitable. Generic nodes, project-local definitions, native validation, explicit context selection and reviewed state publication already provide the necessary foundation. The immediate release blocker is an incompatibility between relationship JSON and the Rust model. Other findings concern semantic projections, diagnostics, test assumptions and context configuration; none requires a new engine, clock, simulator or persistence subsystem.

## Evidence and limits

| Check | Result |
| --- | --- |
| Current-tree `npm run check` | Passed: zero errors/warnings. |
| Current-tree `npm test -- --reporter=dot` | Passed: 17 files, 98 tests. |
| Current-tree native template validation test | Failed at `World::default()`: unknown relationship field `description`, schema line 1710. |
| Isolated-copy native suite, removing only unsupported relationship descriptions from the copied presets | 331 passed, 9 failed, 11 ignored, 6 filtered out. This is diagnostic evidence, **not a passing result for the application**. |
| Six additional isolated-copy probes | Passed; verified nonphysical roots/roundtrip, context boundaries, explicit bindings, relationship projection and saved-schema independence. |
| Request-size instrumentation in isolated copy | Confirmed two size-regression failures; measurements below. Instrumented assertions were replaced with logging only in that copy. |
| Desktop walkthrough / real models | Not performed. No real-provider generation or personal story data used. |

The isolated suite initially encountered temporary-directory access failures. Repeating with TEMP/TMP inside the workspace resolved those filesystem failures. Of the remaining nine failures, three are the catalogue-related test failures detailed below; six are local HTTP fixture tests that could not complete their expected transport behavior in this restricted environment. Those six do not establish schema or provider defects. The other relevant storage, history, fork, backup and protection tests passed in the diagnostic copy.

Reproduction for the actual blocker:

```powershell
node scripts/cargo.mjs test -p story-project location_starters_validate_with_nested_places_and_single_inventory_placements -- --nocapture
```

Diagnostic logs: `build/architecture-audit/native-tests-workspace-temp.txt` and `build/architecture-audit/request-measurements.txt`. Additional probes: `build/architecture-audit/crates/story-project/src/architecture_audit_probes.rs`. The copy removes relation-definition descriptions solely to get past the first blocker; that is not the proposed fix.

## Confirmed bugs and diagnostic defects

### C1 — Relationship descriptions cannot deserialize; bundled defaults panic (high)

**Code:** [RelationDefinition and World::default](crates/story-project/src/model.rs#L149), [updated relationship JSON](schema/world-presets.json#L1710), [frontend catalogue clone](src/lib/model.ts#L10).

`RelationDefinition` has `deny_unknown_fields` and no `description` member. The updated catalogue supplies this property on `core.related`, `story.route-to`, the causal relationships, `story.view-of`, `story.owned-by` and `story.can-hear-from`. `World::default()` parses that catalogue with `.expect("Invalid built-in presets")`.

**Consequences:** Native template creation panics before it can return its normal `Result`. Rust tests using defaults fail at the same point. Empty/Brasswater worlds cloned by the frontend also contain the unsupported property and cannot deserialize into native `World` command arguments. Frontend checks nevertheless pass because the JSON-to-World assertion and structural typing do not enforce Rust's unknown-field restriction. This does not prove that every existing project fails to open: ordinary loading uses its saved definitions and the separate native contracts.

**Smallest fix:** Add an optional/defaulted relation-definition description, omit it on serialization when absent/empty, regenerate bindings and update struct constructors. Preserve omission for old saved definitions so receipt representations do not change. Retain strict unknown-field rejection. Do not delete the new descriptions or globally enable ignored fields.

**Important follow-on:** `World::validate()` compares a saved core relationship with its pinned contract after clearing only `fields` ([model.rs](crates/story-project/src/model.rs#L353)). Merely adding `description` makes the descriptive `core.related` differ from the description-free core contract. Exclude this new explanatory property from the existing core comparison while retaining all currently pinned behavioral properties. Do not weaken placement, endpoint or single-target checks.

### C2 — Result-returning validation still panics on malformed bundled contracts (medium)

**Code:** [World::validate](crates/story-project/src/model.rs#L341), [knowledge contract loading](crates/story-project/src/knowledge.rs#L90), [wearing interpretation](crates/story-project/src/model.rs#L275).

`World::validate()` uses `expect` for `world-core.json`; `validate_knowledge()` uses `expect` for `knowledge-core.json`. Their current files parse, but malformed JSON or incompatible fields in either bundled file panic inside an API advertised as `Result<(), String>`. `Default` checks deserialization only, not semantic validity. The wearing compatibility path also calls `Self::default()` and unwraps the worn-by lookup, coupling saved-file interpretation to the optional catalogue. A malformed bundled worn-by flag can re-enter that default-loading path before validation.

**Consequences:** Catalogue/contract mistakes can bypass useful open/create/validation diagnostics. Invalid defaults can survive compilation because `include_str!` only embeds text.

**Smallest fix:** Introduce fallible bundled-schema parsing within the current model module and use it from user-facing native entry points. Return the filename plus serde line/column and validation detail. Propagate contract parse errors from validation. Resolve wearing's expected behavioral contract from the pinned core rather than optional presets, using fallible lookup. Keep any infallible convenience default confined to code whose inputs have been checked; test it explicitly. No migration framework is needed for this repair.

### C3 — Relationship meaning is absent from authoring descriptors and prose projections (medium)

**Code:** [relationship descriptors](crates/story-project/src/prompt_context.rs#L88), [story_link](crates/story-project/src/projection.rs#L70), [passage request assembly](crates/story-project/src/story.rs#L933), [connection editor](src/lib/WorldEditDialog.svelte#L59).

Relationship descriptors explicitly emit ID, label, endpoint capabilities, physical/single flags and field descriptors; they have no relationship-description projection. Prose/planning links emit source/target, label, connection notes and nonempty applicable property values, but drop field help/stability. Prose receives no accompanying relationship catalogue: that section is added for effects extraction only. Node narrative fields, in contrast, already carry help and stability.

**Consequences:** Adding description support alone will not deliver the carefully written directional/knowledge caveats to models. Existing prose links such as “A can hear from B” rely on an ID/label and model interpretation. World/effects authoring receives field help, including route-direction help, but still lacks the relation-level meaning. The editor offers generic connection help rather than the new definition's explanation.

**Smallest fix:** Project nonempty relation descriptions in the current descriptor helper and show them in connection editing. In narrative/planning context, include a compact meaning/field-help projection for the relationship types actually present in the captured links, sharing it once per type if practical. Preserve current task filtering and source boundaries; do not send the full world or every relation definition. Update corresponding prompt fingerprints/contracts so pending checks cannot silently retain the earlier semantic packet.

### C4 — Template validation test rejects legitimate nonphysical roots (medium, test defect)

**Code:** [template assertions](crates/story-project/src/location_templates/tests.rs#L7).

The test requires every node to have nonempty `visualDescription`, exactly one root, and a physical link on every other node. Each current template intentionally has nine roots: one physical location plus time, environment, narration, visual style, brief, organization, lore and rule. Nonphysical nodes correctly have empty visual notes.

**Consequences:** This test still fails after C1 is bypassed. Its root assertions would fail next. Requiring fake visual descriptions or physical parents to satisfy it would damage the intended schema model.

**Smallest fix:** Assert a single physical location root and the expected independent nonphysical roots separately. Require visual content only where the template promises physical appearance, and placement only on intended place/item children. Keep no-cast, unique-ID, nested-container, native-validation, independent-copy and project-reopen checks. Update the hardcoded template-count expectation deliberately when adding catalogues.

### C5 — Import errors frequently omit the identifying template/node/path (medium, diagnostic defect)

**Code:** [template parser/importer](crates/story-project/src/location_templates.rs#L30), [native value validation](crates/story-project/src/model.rs#L535), [frontend create catch](src/App.svelte#L273).

Parser errors return serde line/column, which is useful. Unknown preset, extension, duplicate key and missing parent errors are generic. Final validation is passed through without a template ID or mapping from allocated UUID back to template key. Some value errors name the node and field label, but omit component ID, field key, expected kind and offending value. Parsing the whole catalogue also means one malformed unrelated template prevents all template selection.

**Consequences:** A broken starter may only say “unavailable extension” or “invalid field” even though the importer knows its exact template/key. The UI correctly catches returned errors and keeps the create dialog open, but cannot invent missing diagnostic context. A panic from C1 bypasses this ordinary error return.

**Smallest fix:** Prefix importer errors with template ID/name; include `nodes[key]`, preset/component/field IDs and parent key at their existing lookup points. Preserve the UUID-to-key mapping for final errors, and enrich native field errors with component/key and expected kind. For example: `template school / nodes[environment] / detail.environment.weather: expected longText; received number`. Keep the existing JSON shape and normal frontend catch/display. Prefer existing error strings plus context before introducing another error subsystem or dependency.

### C6 — Catalogue growth breaches existing compact-request regression limits (medium)

**Code:** [world/group size checks](crates/story-project/src/authoring/tests/creation_quality.rs#L57), [narrative size checks](crates/story-project/src/story/tests/planning.rs#L4), [effects creation variants](crates/story-project/src/story_effects/schema.rs#L126).

After removing unsupported descriptions only in the diagnostic copy:

| Existing fixture | Measured bytes, system + user + supplied schema | Test ceiling |
| --- | ---: | ---: |
| World creation, schema mode | 46,396 | 46,000 |
| Passage effects, JSON mode | 69,405 | 60,000 |
| Passage effects, schema mode | 69,387 | 60,000 |

Prose (9,899), planning (5,411), the item-group and revision samples remained within their respective limits. These are byte-regression failures, not evidence of a failed real-model response or a default runtime budget overflow. The furnished-scene 32K preflight test passed in the copy.

**Smallest fix:** Inspect currently offered effect-node/relation variants and repeated guidance. Compact task descriptors/duplicate schema information within existing helpers and preserve required contracts. Add time/environment only where allowed/relevant to the task. Re-measure with relation meanings restored before deciding whether a small intentional limit adjustment is justified. Do not simply increase the configured context window, drop native validation or remove support for custom types.

## Likely integration issues requiring targeted tests and review

### I1 — Imported supporting nodes are stored but have no operational context inclusion

The importer creates default-schema nodes and physical links, then validates. It never creates `aiContext` bindings or `story.conditions-at` links ([import loop](crates/story-project/src/location_templates.rs#L56)). All three templates contain the eight independent supporting presets. Their imported narration/visual nodes do not pass through the frontend shortcuts that install bindings ([shortcut functions](src/lib/ai-context.ts#L19)).

[scene_world](crates/story-project/src/story.rs#L395) starts with selected resources, relevant claims and direct actor inventory, then adds physical ancestors. It neither selects global time by preset nor follows a conditions-at link into an environment node. A diagnostic probe confirmed that even adding such a link alone does not include the environment. Selecting both supporting records as scene references does include them and their applicable link.

**Storytelling implication:** A starter's saved time/weather, world rules and prose/image style can remain unused until the author selects references or configures bindings. This is confirmed behavior; treating every such omission as a bug would assume an automatic-inclusion policy that the existing architecture intentionally avoids.

**Recommended test/decision:** Exercise a starter-to-first-passage/image flow and inspect the exact native context. Agree on intended template defaults. If starter creation should include narration/visual guidance, use the existing bindings and their current task defaults in the same new-world result. Time can use an explicitly reviewed Reference binding or scene reference. Local conditions need an explicit selected reference/appropriate scoped link policy, with visible coverage. Do not bind all nodes to every tool or assume automatic parent-area weather inheritance.

A binding supplies content, not edit permission or scene membership. In particular, putting the time in a binding lets narration read it but does not authorize effects to update that existing time node if it is outside the scene's effect scope. Test this boundary rather than quietly broadening effects permissions.

### I2 — Route direction is descriptive, and missing values have no runtime fallback

The new field has an optional `two-way` default. New editor defaults use it, but `World::validate()` does not materialize defaults into existing edges; narrative projection emits only stored values. An older saved project retains its older route definition without the field. There is no route-traversal algorithm or reverse-edge synthesis in application logic.

Test A→B with one-way, two-way, missing direction and asymmetric access. Establish what an unspecified legacy route means before offering a reviewed schema/edge update. Do not reinterpret every old route as two-way merely because the new catalogue has that creation default. C3 is necessary for clear prompting but cannot guarantee model compliance.

### I3 — Ownership endpoints are intentionally advisory

`story.owned-by` has unrestricted endpoints and multiple targets. Its description says item/place → character/organization; native validation and UI filtering do not enforce those alternatives. This supports shared ownership and avoids expanding the current single-capability constraint model, but also allows nonsensical endpoints. Treat this as a known advisory contract; add examples/warnings and tests first. A small optional validation improvement can be reviewed separately if strict ownership endpoint unions are wanted.

### I4 — Visual/planning task metadata is not a universal confidentiality filter

Ordinary narrative component projection excludes Visual-only and Planning-only values; planning includes Story and Planning. Image appearance includes Visual fields only. The explicit AI-binding content assembler includes selected component values regardless of `uses` ([content](crates/story-project/src/ai_context.rs#L78)). This is how Planning-only narration instructions intentionally reach passage writing. Authors can likewise explicitly bind visual instructions to passage tasks. Generic node notes, including `visualDescription` in narrative records, are outside per-field `uses` filtering.

Keep these explicit controls. Test that ordinary selection does not leak structured visual/planning-only fields and that template/shortcut defaults use the correct tasks. Do not impose blanket metadata filtering on bindings that would disable narration instructions. Copied facts in ordinary notes remain ordinary context, as the existing knowledge documentation explains.

### I5 — New types are supported, but not selected in every default AI creation scope

Manual add, schema-driven inspector and checked-field authoring work for the new component kinds. Native creation schemas derive types/fields from the selected catalogue. However, frontend and native foundation defaults explicitly favor place/character/item/narration/rule/brief capabilities ([frontend selection](src/lib/authoring-ui.ts#L16), [native selection](crates/story-project/src/authoring/creation.rs#L3)). Time/environment/organization/lore/visual style are not all selected by default.

That is not inability to author them. Test explicit typed creation, mixed-group selection and revision for time/environment and all new relationships in JSON, prompt and schema transports. Preserve saved request scope and unchecked-field protection; do not silently enlarge older requests.

## Relationship semantics and physical consistency

| Relationship | Current definition and application behavior | Audit conclusion |
| --- | --- | --- |
| `story.route-to` | Place → place, nonphysical. Source→target is forward; stored direction permits one-/two-way travel conceptually. | No movement simulation or reverse-edge expansion. Values survive projection; relation meaning/field help need C3. Map currently draws one target arrow for every edge, even a two-way route. |
| `story.view-of` | Place looking from → place looked toward; nonphysical and directional. | Does not move anyone or acquire knowledge. Its caveat currently fails to reach prompt descriptors. |
| `story.can-hear-from` | Listening place → sound-origin place. This arrow is the reverse of sound propagation. | No hearing/understanding event is inferred in native code. Explicit prompt examples are particularly useful here. |
| `story.owned-by` | Owned thing → owner; nonphysical, multiple owners possible. | Does not contribute to inventory/placement. Endpoints are advisory (I3). Ownership and physical possession can legitimately differ. |
| `story.conditions-at` | Environment-capable node → place, nonphysical; optional coverage/exceptions. | Validates endpoint capabilities but does not select/apply/inherit conditions automatically (I1). |

No native path was found that turns visibility, audibility, co-location or ownership into a character stance. Knowledge remains claim/held-by state, and passage prompts already warn against acquiring knowledge from proximity, possession or another character's belief. A model may still infer something unsupported; semantic quality must be tested separately from structural guarantees.

Physical rules are coherent: one physical edge per source across physical relation types, no physical cycles, counted container quantity one, worn items require item capability and quantity one, and worn-by is itself inventory placement. `core.stored-in` covers carried/stored items; no redundant possession edge is required. Nonphysical ownership/route/sight/sound/conditions links do not compete with placement. Existing operations replace physical placements atomically in preview; generic links leave placement unchanged. Preserve these checks unchanged.

## Schema evolution and supported-format compatibility

**Already correct:** Every world snapshot saves its definitions. Normal open loads and validates that saved schema; it does not replace it with `World::default()`. Optional new presets do not become mandatory. The frontend offers missing built-in node presets and explicitly installs missing components only when selected ([model helpers](src/lib/model.ts#L11)); it does not overwrite an existing local definition. The claim preset separately installs its pinned relationship when needed. It does not automatically install every newly bundled relationship.

| Change | Current compatibility effect / required action |
| --- | --- |
| Add optional preset/component/relation to bundled presets | Existing saved worlds keep their definitions and instances. Use explicit installation when authors want it. No automatic migration needed. |
| Change bundled descriptive extension fields/choices | Existing projects keep the old saved extension. New projects get the update. Do not overwrite same-ID local schemas to “refresh” them. |
| Change a saved local field kind, remove a field or add a required field | Existing values may become invalid; optional non-null defaults are creation helpers, not backfills. Preview/validate affected nodes/edges before committing. |
| Change saved relationship endpoint requirements, physical flag or single-target | Can invalidate existing endpoints, cycles or multiple targets. Require validation of the complete candidate world. Do not reinterpret historical snapshots. |
| Change pinned core capability definition | Exact comparison rejects old definitions, including otherwise descriptive changes such as labels/help/defaults. Avoid casual edits to `world-core.json`; an intentional breaking contract requires an explicit format/version decision. |
| Change pinned core relationship | Existing comparison pins everything except extensible fields. C1's new explanatory description must be handled deliberately without loosening behavior. |
| Change Claim/held-by pinned contract | `knowledge-core.json` validates exact definitions when present. Optional absence is supported; changed present definitions can invalidate old projects. |
| Introduce new Rust enum variants/properties | New code must parse old omitted forms and preserve serialization; old binaries may reject new worlds because unknown fields/enum values are strict. State forward-read limits explicitly. |

The supported story format is 2 ([store version/load](crates/story-project/src/store.rs#L13)). Older prototype-format migration is outside the approved requirement. Within format 2, retain immutable files, checksums, receipts and historical definitions. If a future contract change truly requires conversion, publish a separately validated converted copy or new revision under an explicitly chosen format policy; never rewrite old snapshot bytes or replace original folders. In particular, do not add descriptive defaults during deserialization if doing so changes stored receipt serialization. Test absent description, empty description and populated description distinctly.

## Location import path and format limits

The full path is frontend bundled preview/selection → `locationTemplateWorld` Tauri command → strict catalogue parsing → selected ID lookup → default schema → allocate all template-key UUIDs → preset/component defaults plus overrides → optional parent conversion → complete `World::validate()` → frontend folder selection → native project create/validate/publication. The importer itself performs no project writes. Project creation validates before creating a fresh destination folder and never replaces an existing project.

Parent conversion already supports parentless nodes: it only runs for `Some(parent)`. Place-capable children use `core.inside`; item-capable children use `core.stored-in`; other children with a physical parent are rejected. Thus standalone time, environment, narration, visual style, brief, organization, lore and rule nodes are supported. Container/holder capability and parent correctness are checked by normal world validation. All current starters validate and create/reopen in the diagnostic copy.

The current JSON format can express nodes, component overrides and physical parents only. It cannot declare arbitrary typed edges or AI bindings; those are neither parsed nor synthesized. Component node references also use native values, not a documented template-key alias substitution. Current templates do not need such references. Retain the format for C1–C6. Only if reusable starter-specific route/conditions links or configurable context inclusion are explicitly required should a small optional `edges`/binding extension be considered, with the same native ID allocation and validation. Do not force those meanings into `parent`.

## Areas already working correctly

- Native authority: project create/open/commit validate complete worlds; unknown data is rejected rather than silently discarded. Generic extension values, choices, references, names, quotas and relation capabilities are checked.
- Generic persistence/editor: the eight requested nonphysical presets use ordinary nodes/components, can remain roots, roundtrip, and edit through the shared inspector. No new persistence collections are necessary.
- Project-local extension independence: optional schema additions do not rewrite old projects, accepted worlds or receipts. Native core and optional claim protections remain separate from the preset catalogue.
- Metadata projections: Story/Visual/Planning field values reach their intended ordinary projections. Environment temperature is story-only; weather/season/light/surfaces also support visual projection. Brief and visual-style structured fields are planning-only. Narration instructions deliberately use explicit bindings.
- AI authoring: schema-derived concrete node/link fields, per-preset branches, endpoint/unchecked-field protection, bounded context and explicit Apply are already implemented. New relationships use the existing typed-property pipeline.
- State/history boundaries: passage effects require review and atomic acceptance; author-only/planning-only profiles are protected from prose-derived changes. Wording/history/source/fork/backup tests passed in the diagnostic run. No automatic simulation or observation-to-knowledge inference was found.
- Import safety/UI: default merging, parentless nodes, physical-parent validation, independent IDs, no-write assembly, native project validation, and frontend returned-error display are present. The deficiencies are C1/C4/C5 and the inclusion decisions in I1.

## Optional design improvements

1. Display two-way routes with a second arrow or direction text while retaining source/target as the stored orientation. This is presentation, not graph traversal.
2. Offer an advisory owned-by endpoint warning using the existing editor; strict unions can wait unless the author needs them.
3. Make starter preview wording cover supporting records as well as “locations and objects”, and expose context-inclusion choices through the existing context controls.
4. Keep a small frozen format-2 compatibility fixture set rather than relying solely on worlds generated from today's defaults. Include custom schemas and absent optional properties.

## Ordered implementation plan — pending findings review

1. **Repair the blocker and preserve serialization.** Add relation descriptions compatibly, preserve absent forms, update core comparison, generated bindings and affected constructors. Add direct tests for all bundled documents and current starter assembly. Prove empty/Brasswater/template native deserialization. Do not remove descriptions to get a green test.
2. **Make bundled failures diagnostic.** Reuse fallible parsing at native user boundaries; replace validation-time expects; decouple wearing interpretation from optional presets. Add intentionally malformed/unknown-field/invalid-contract fixtures and verify filename/path diagnostics without panics or project writes.
3. **Repair import diagnostics and assertions.** Name template/node/component/field in errors. Replace physical-only template assumptions. Parameterize unknown preset/component/parent, duplicate key, invalid value and illegal physical-parent tests. Preserve the existing template JSON shape. Cover frontend rejected-command display, busy reset and no destination publication on failure.
4. **Complete relationship semantics through existing projections.** Include relation descriptions and applicable field help in authoring, passage/planning/wording packets and connection editing. Test one-way/two-way/missing routes, listening-place → sound-origin orientation, ownership versus storage, conditions coverage and no automatic knowledge/state changes. Verify captured-source fingerprints and budgets.
5. **Review starter/context defaults, then implement only the chosen policy.** Use current bindings/reference selection for time, narration, style, brief/rules and environment. Test first passage, effects, planning, historical wording and image guidance separately. Confirm bindings do not silently authorize edits, select unrelated nodes or apply weather to every child place.
6. **Lock compatibility behavior with fixtures.** Open earlier supported-format schemas without adding defaults; explicitly install optional presets without replacing local definitions; reject behavioral core weakening. Cover changed required fields/choices/endpoints, omitted route direction and description omission in original receipts. Run create/reopen/commit/backup/fork checks. If persistence representation changes, include interruption and replay tests.
7. **Restore compact-request limits and gate release packaging.** Re-measure selected tasks with real descriptions restored. Keep complete native validation and scoped concrete schemas; justify any ceiling change with measurements. Make the release/manual-package workflow run the relevant verification before packaging. Currently `npm run verify` includes native tests, but `manual:build`/`rebuild` and Tauri's `beforeBuildCommand` do not enforce those checks, so a compiled executable can contain invalid embedded JSON.
8. **Verify the unchanged architecture end to end.** Run frontend tests/checks, native tests/check and production build after fixes. Then perform a short fictional desktop starter → context preview → reviewed passage/image flow. Report desktop evidence separately. A short, explicitly authorized local-model probe can subsequently assess storytelling behavior; it is not needed to verify parsing/storage.

Recommended automated release coverage reuses the current modules: parse every bundled file into its real Rust structures, validate the default schema, instantiate every preset with declared defaults, import every template, roundtrip definitions/instances, compare frozen supported-format receipts, and snapshot task packets for selected new relationship/component values. Run these before packaging rather than duplicating the Rust model in a separate build-script validator.
