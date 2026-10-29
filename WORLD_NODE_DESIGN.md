# Composable world nodes — revision design

Revised October 28, 2026. This records the node-model target and its implementation slices. It refines [DESIGN_REVIEW.md](DESIGN_REVIEW.md) and the [build plan](REVIEW_AND_BUILD_PLAN.md). The checkpoint below separates implemented behavior from later design.

The confirmed direction is a shared world-node model with small behavioral capabilities, reusable descriptive extensions and useful bundled presets. Characters remain in the world map/shared inspector. The user has explicitly allowed starting fresh: compatibility with older prototype story formats and conversion of existing stories are not requirements for this revision. Preserve transaction integrity and immutable history within the new format.

## Initial implementation checkpoint

The first capability/default-preset slice is implemented in story format 2: optional typed fields with purpose/task/stability/choice/reference/coverage metadata, native contract registry separate from the preset catalogue, sparse defaults, reviewed attach/remove/retype, compact shared field sections, typed custom definition forms and structured search. Section names/order come from capability names and the preset's ordered capability list; fields retain their declared order.

Character/Place/Item/Thread bundles and Organization/Lore/World rule/Story brief presets use the shared machinery. Normal passage context receives human-labelled story fields; image preparation and optional image-prompt assistance share captured structured appearance, garment design/condition and coverage filtering. Group discovery also searches bounded structured text locally, with its existing transmission/write limits intact. Explicit context bindings retain their selected content.

The relationship slice now adds typed edge properties and project-local relationship fields, selected-field link-only/mixed AI revision, individually approved rich-link passage effects and story/visual projections. Standard fields cover attitudes/reasons/tension, membership roles/obligations, debts/terms/status, routes/access/obstacles and current wearing configuration. Native physical semantics remain intact. The Event/source slice now supplies the standard Event preset, typed node citations, registered immutable text anchors, native authoring/acceptance guards and exact-source reading through the shared inspector. Sources survive wording, history, backups and applicable forks without replaying events. Visual style/direct-image bindings and viewpoint-aware claims/disclosure remain planned. Editing/removing existing schema definitions and advanced genre bundles are not included in this first UI slice. Verification details are in [FOUNDATION_STATUS.md](FOUNDATION_STATUS.md).

## Findings and decisions

The current character and place capabilities are empty behavioral markers; the thread capability adds only a resolved flag. This is a useful starting separation of behavior from content, but the supplied presets leave personality, appearance, intent and setting function buried in general notes. Adding fields alone will not improve generation if context assembly, image preparation and review ignore them.

Keep the shared-node foundation. Develop the standard presets, field semantics, relationship records, inspector and task projections together. A useful character should be available in one creation action, without requiring the author to configure its components first. A custom preset must receive the same authoring, validation, history and projection support.

## 1. Node, capability, preset and instance

| Layer | Responsibility | Example |
| --- | --- | --- |
| Base node | Stable ID, preset/type ID, name, supplementary story/visual notes and attached component values | Mara's identity remains the same after a rename or an added extension. |
| Behavioral capability | A small explicit native contract that enables validated operations | Character participation, scene location, counted inventory, holding contents, container, wearable. |
| Descriptive extension | A reusable capability with typed fields and declarative authoring/context metadata | Motivation, voice, appearance, condition, place layout. |
| Preset | An initial bundle of capabilities and display/creation guidance | Character bundles participation, profile, appearance, motivation, voice, condition and holding possessions. |
| Instance | One actual story-local node, with its own attached capabilities and values | A minor character can retain only the details the author needs. |

Both kinds of capability use the same project-local definition/value mechanism. “Extension” is the author-facing name for adding detail or behavior, not an executable plugin or a second persistence system. Provider clients, jobs, credentials and settings remain services outside nodes.

Rules:

1. Native behavior checks capabilities, never a preset's name or a fixed entity collection. Built-in inventory/placement contracts remain explicit and cannot be weakened by custom metadata.
2. Presets apply on creation. Editing a preset does not retroactively change existing instances. Retyping an instance previews additions; removing data or capabilities is a separate explicit choice validated against dependent links/references.
3. Capabilities can be combined where their actual contracts permit it. Presentation cannot choose one exclusive “kind” and thereby hide other supported features. Invalid combinations are rejected for a stated structural reason.
4. Adding an extension initializes only its declared defaults. Descriptive values normally remain unset. Removing an extension previews data loss and dependent links/references; it never silently converts structured data into notes.
5. No live inheritance of facts from templates, species, cultures or organizations. Those are references. A template creates independent values; editing it never redesigns existing individuals.
6. Custom definitions are portable JSON. No scripts, CSS, arbitrary expressions, model clients or executable validators live in a story definition.

Keep the existing built-in capability IDs where their meanings still fit. In the fresh format, thread eligibility can also be a small marker; thread status and descriptive development fields live in its bundled extension. A capability's data is attached once per node, even when several preset bundles recommend it. Section ordering does not create an inheritance hierarchy or another copy of its values.

## 2. Field contract

Retain typed text, long text, number, Boolean, node reference and node-reference-list fields. Add bounded single-choice fields for statuses and other constrained choices. Reference selectors must show names and allow capability filters; stored references always use IDs. Use links for meaningful relationships with direction or their own information, rather than parallel node-reference fields.

Definitions need optional, validated metadata for help/purpose, section/order, task use (story, visual or author planning), and whether a field describes a lasting trait or changing state. The same definitions drive inspector controls, AI field selection, generated output schemas and native validation. Descriptive metadata guides generation; it grants neither write scope nor instruction authority.

Absence means unspecified. Optional text may be empty; optional numeric/Boolean values must distinguish unset from zero/false. Native-required operational fields, such as inventory quantity/unit, retain their explicit requirements. Do not make a biography complete by requiring invented ages, motives, measurements or history. Required components in a creation preset do not mean that all their descriptive fields must be filled.

Stable-trait/state metadata aids context and review, not automatic permission to overwrite a field. Name, placement and quantity changes still require the selected action's scope. Checked-field protection remains enforced. Add persistent fact locks only as an explicit later feature, not as an inferred consequence of “stable.”

### One owner for each fact

| Information | Authoritative owner | Other views |
| --- | --- | --- |
| Name | Base node | Search, map and prose context reference the same ID/name. |
| Physical identity | Appearance/anatomy extension values | Readable appearance summary is computed, not separately edited. |
| Clothing design and condition | Garment item extensions | Character outfit summaries resolve worn item IDs. |
| Location, storage and wearing | Physical links | Inventory/location panels are derived. |
| Social relationship | A typed link and its properties | Shown from either endpoint without copying its content. |
| Character belief | A claim plus its character/perspective relationship | It is not automatically world truth. |
| Accepted occurrence | Accepted passage/effects and exact source versions | Significant event nodes reference those sources. |
| Future intent | Character intention or an explicitly marked author plan | Neither guarantees that an event occurs. |

Base story/visual notes remain available for supplementary detail. They do not become independently editable copies of assembled profiles. Generated summaries combine structured values and applicable notes deterministically, without a hidden model call. Conflicting free text may need an advisory warning and author resolution; structural validation cannot prove semantic consistency.

## 3. Standard character bundle

The normal Character preset includes participation and holding possessions, plus the following descriptive extensions. Descriptive fields are optional; collapsed empty sections do not demand completion.

| Extension | Initial fields | Intended use |
| --- | --- | --- |
| Profile | Pronouns, actual/apparent age, occupation or social role, relevant background | Identity and context. Species/culture can be linked when those records exist. |
| Appearance | Species/form description, build, complexion/surface, face, hair, eyes, distinguishing features | Persistent visible facts; flexible enough for nonhuman characters. |
| Personality | Values, temperament, habits, strengths/limitations, meaningful contradictions | How this individual tends to choose and behave. |
| Motivation | Longer-term wants, reasons/stakes, obstacles | Ongoing drives, usable by organizations and other intentional entities too. |
| Voice | Speaking style, conversational habits, short example | Distinct dialogue, not a compulsory accent or catchphrase. |
| Condition and intent | Current physical condition/injuries, emotional state, immediate objective | Changing state at the selected story boundary. |

Detailed anatomy is an optional appearance extension with region descriptions and freeform detail, preserving the author's explicit wording for adult physical features. Appearance is separate from garments, temporary pose/expression and per-image composition. Relevant visibility and coverage determine which details enter image prompts. Detailed profiles need not be repeated in ordinary prose.

Do not add clothing strings, an independent carried-items list, a second location field, or copied relationship biographies. Expose these through linked records. Optional extensions can add skills, abilities or other story-specific detail. Animals and other individual creatures can use character capabilities without a separate hardcoded creature engine.

Depth follows narrative use. A passing shopkeeper can have a name and one distinguishing trait. A principal character benefits from a concrete objective, pressure, relationships and voice. AI creation should suggest useful specifics rather than lists of generic adjectives. Character change/arc is optional author planning; no mandatory trauma, secret, flaw or epiphany.

## 4. Places, items and threads

| Preset | Bundled descriptive structure | Derived relationships |
| --- | --- | --- |
| Place | Purpose/use; layout/access; resources and obstacles; routines/customs; sensory appearance; current conditions | Containing place, routes, occupants, furnishings, relevant groups and threads. |
| Item | Function; requirements/limitations; design/material; current condition; optional provenance/significance | Holder or wearer, container contents, relevant users/events. Quantity/unit remain inventory facts. |
| Wearable item shortcut | Item bundle plus coverage/layer and optional cut, fit, colour/pattern | One worn-by link or one stored-in link. |
| Thread | Central question/goal/situation; stakes; unresolved issues; optional author direction; status | Involved nodes, source developments and passages. |

Use bounded thread statuses: open, active, paused, resolved, abandoned. Track the most recent established development through source references rather than another editable history summary. Desired payoffs remain plans. Simple goal/quest/mystery presets can reuse this bundle; no separate quest engine is needed.

Places and items should help establish what can happen: a window overlooking the approach, a sticky cellar door, a working pump, an empty fuel can. They need not all have secrets or conflicts. Routines describe usual behavior, never proof that offscreen events took place. Routes are links; containment does not prove navigability or travel.

Container remains an addable capability. Occupied interiors of large vehicles can be linked place nodes; the vehicle is one physical object with one placement. Avoid creating a second competing placement merely to display its interior.

## 5. Broader preset catalogue

These are planned bundles, not new persistence collections and not a requirement to populate every story with them.

| Catalogue group | Presets | Minimum responsibilities |
| --- | --- | --- |
| People and groups | Character; Organization | Organization purpose, interests, resources, relevant history; member/leader/headquarters links. Family, business and faction are specialisations, without autonomous simulation. |
| Places and things | Place; Item; Wearable shortcut | Scene use, appearance and established physical relationships. Container is available as an extension/shortcut. |
| Developments and knowledge | Thread; Lore/background; Event | Lore explains context; threads hold unresolved developments; events represent significant established occurrences with sources. |
| Rules and guidance | World rule; Story brief; Narration style; Visual style | Separate fictional constraints from author direction, prose presentation and image presentation. |

Lore holds background and significance with links to relevant nodes. A World rule records a fictional constraint, scope and exceptions. A local custom need not become a universal law. Story brief holds premise, genre, themes and author aims; project title remains project metadata, chapter/scene plans retain their own plans. Do not create duplicate editable premise/preferences fields elsewhere when adding the brief.

Narration style retains person, tense and instructions. Visual style holds image medium/rendering/palette/composition guidance; it cannot change anatomy, worn items or other source facts. Both use explicitly reviewed task bindings. Image-preparation inclusion must work for direct image preparation as well as optional LLM prompt assistance; selecting it never initiates a text request. Creating a rule, lore node or style node does not implicitly include every field in every prompt. Keep the existing narration creation shortcut for Passage/Wording and show the chosen inclusion.

Events require a deliberate boundary: manually authored backstory may establish a past occurrence; a passage event points to exact accepted passage/effect sources. Creating/editing an event node must never replay effects, alter those passage snapshots or establish an unapproved future occurrence. A substantial event can connect participants, places, causes and consequences. We do not need a new event node for every action sentence or every accepted passage.

Optional later bundles: Culture/tradition, Species/creature kind, Ability/technique/technology, Condition definition, Material/resource. Culture and species are independent. A condition definition is separate from an individual's current condition; a material definition is separate from a counted stock item. Books/letters can combine an item with readable content. The contents of a letter can be false, and possession does not establish that anyone read it.

A plain Note remains the escape hatch. New/Open starts with no instantiated nodes. The picker can group presets and offer More presets; it must not become an encyclopaedia checklist.

## 6. Relationships, knowledge and source records

### Rich typed links

Keep simple connections as edges. Extend relationship definitions with optional typed property definitions and edges with corresponding property values, reusing the field renderer/schema validator. Node capabilities remain the mechanism for node extensions; a relationship's own fields belong to its relation definition. Do not maintain a relationship node and a link as duplicate sources of truth.

Examples: member-of (role/responsibility), feels-toward (attitude/reason), owes (obligation/terms), route-to (access/obstacles), worn-by (current worn configuration). A social relationship can be directional and asymmetric. Symmetric bonds can use one shared link displayed at both ends; asymmetric attitudes use separate directed links. No required numerical affinity scores.

Operational meaning is reserved for registered native contracts. Custom fields such as “duration” or “capacity” are descriptive unless the app explicitly implements and validates that operation. A display label cannot make an edge a valid physical placement or a new automation rule.

Link edits use the same explicit scope, field protections, source capture, review and atomic application rules as node edits. Field addresses identify node/link kind, stable record ID and field/component key; labels and inspector ordering never identify an editable fact. Passage review cards bind approval to complete rich-link contents, applicable schema, evidence and dependencies, just as for node changes. Simple existing reference links in creation groups remain read-only with respect to existing records/placements. Richer relationship fields do not expand a group's write permissions.

### Knowledge and belief

Plan a Knowledge/claim preset over shared nodes once perspective-aware context is implemented. It identifies the proposition, its author-assessed status (established, unconfirmed, disproved), related subjects and source annotations. Character belief/knowledge links identify who holds it, their stance and any acquisition source. Reader disclosure is separate from character knowledge. Believing a disproved claim must be representable.

Do not ship a “secret” checkbox that implies working viewpoint protection while context still sends all node fields. Distinguish information withheld from a model task from information the writer may use but a character does not know; show that distinction in preview. Omniscient narration and author planning have different disclosure needs from a limited viewpoint. A known-by label alone does not grant knowledge to nearby characters. A character's immediate intention is present state; an author's possible continuation is planning material. Hypotheses and future plans must not be assembled as established facts. Semantic leakage checks remain advisory; source/visibility permissions are enforced structurally.

Use native-validated, typed source annotations for exact passage/version/effect or manual-author provenance. Passage IDs are not arbitrary node references or unvalidated text fields. Existing commit receipts and immutable snapshots remain authoritative. Memory summaries remain reviewed derived records, not a second editable event ledger. Wording edits preserve/label earlier evidence anchors; event replacement and forks retain only applicable source versions.

## 7. Shared inspector and map

Show a compact overview and expandable sections supplied by attached capabilities. Common character sections are Profile, Appearance, Goals, Voice, Current state, Belongings and Connections. Other presets compose suitable sections from the same machinery. Empty optional sections stay collapsed; basic creation never requires visiting an extension manager.

Add extension is a secondary control with a concise description and a preview of new fields or required capabilities. Removal previews lost values and references. The ordinary editor uses named inputs and per-field proposal differences; JSON remains an advanced diagnostic/repair route. Do not add a separate cast panel or open many static capability cards above the useful editing fields.

The map remains location-first but all nodes remain discoverable through search/tree and focused connected views. A nonphysical node can stand alone or appear in a related neighborhood without becoming physically inside a place. Membership, scene references and context bindings never imply placement. Display grouping, hierarchy expansion and layout remain noncanonical workspace state.

Search and summaries must consider the new structured fields. Focused link expansion is explicit and bounded. Additional fields must not cause every node to display its entire profile or all relationships on the canvas.

## 8. AI authoring and context contract

One schema-driven pipeline serves built-in and custom presets. It derives allowed fields, values, links and output schema from the captured project definitions. Adding a descriptive extension does not require a new model client or a bespoke generation service.

Actions include Develop profile, Suggest motivations, Develop voice, Describe appearance, Develop relationship, Create outfit items, and Flesh out place/contents. These are scoped shortcuts over the shared tool. They work with empty selected fields using name/type/context; directions are optional for existing-node development. Reroll preserves IDs, unrelated fields and physical links. New garment creation is an explicit linked group, not a side effect of revising appearance.

| Task | Prioritised projection |
| --- | --- |
| Node/group creation and revision | Requested field definitions/help, checked editable source, required guidance, permitted existing references and a bounded discovery index where applicable. |
| Passage writing | Scene objectives/pressures, participant profile/voice/state, relevant social links and usable surroundings/items; applicable knowledge and instructions. |
| Effects extraction/review | Exact prose and source facts needed to substantiate a proposed change; temporary conditions do not justify an invented personality rewrite. |
| Image preparation | Captured physical identity, visible condition, worn item design/configuration, selected shot and explicit image-style guidance. |
| Wording revision and memory | Applicable narration instructions and exact retained source versions; preserve the existing task's event boundaries. |

A shared native projector must feed manual image preparation and optional LLM image-prompt assistance, so both see the same appearance/outfit facts. Include all relevant extension values in media fingerprints/provenance. Keep physical-appearance applicability separate from a request's style/settings fingerprint: changing a style can stale a pending prompt proposal without claiming the character's body changed or invalidating accepted image history. Do not assemble rich appearance for the UI while submitting only the old visual-description string.

Task-use metadata narrows ordinary recall. Explicit binding fields, required sections and their budgets remain inspectable and enforced; changed selection cannot silently drop required guidance. If an explicit required binding conflicts with that task's source/disclosure permissions, report the conflict rather than sending restricted content or silently omitting the binding. Context bindings do not recursively expand linked nodes or authorize edits. Unspecified custom fields receive an inspectable story/reference fallback, not automatic global inclusion or instruction authority; authors may mark visual/planning use. Custom appearance fields marked visual enter the same image projector.

Keep the approximately 30B/Qwen target. Rich profiles increase output size: estimate budgets from requested bundles/fields and offer fewer useful records when necessary. Do not retain the old per-node size assumption blindly, increase context limits silently, or add hidden calls to fill every blank field. Show included/omitted context and selection reasons. Opening panels, attaching extensions and browsing the map never generate text.

Schema/semantic-metadata changes participate in captured source fingerprints and prompt-contract versions. Native validation rejects unselected field/link changes, invented IDs and stale sources; visible proposal editing does not bypass validation. Descriptive quality review is advisory, while structural integrity is mandatory.

## 9. Fresh format and publication

Use a new explicit story-format version for this revision. New projects and isolated fixtures use the revised definitions. The app may reject older prototype formats with a clear New/Open choice; no import or compatibility transformation is required. Do not delete or overwrite existing story folders automatically.

Within a supported story, definitions travel with snapshots, backups and forks. Application upgrades do not replace saved project definitions behind the author's back. Separate the small native behavioral-contract registry from the optional bundled preset catalogue: a new catalogue entry must not suddenly become a validation requirement for every saved world. Adding/updating definitions is an explicit current-world operation with dependency validation and source invalidation.

Keep stable IDs and atomic native publication. Preset/extension edits, manual property changes and grouped AI application retain their existing revision/receipt boundaries. A fork at an earlier passage uses that passage's schema and knowledge boundary; it does not import later definitions or field values. Interruption tests remain required for changed persistence, including new definition/link/source metadata.

## 10. Implementation slices and completion criteria

These slices are ordered dependencies. A slice is not complete when only a schema file or UI form exists.

| Slice | Deliverable | Required evidence |
| --- | --- | --- |
| A: Capability contract | Fresh format, field metadata/choice support, independent capability queries, native core registry vs optional catalogue; reviewed attach/remove/retype | Custom preset/node roundtrip; unset vs false/zero; invalid references/choices rejected; dependent removal blocked; interruption/replay and snapshot integrity. |
| B: Useful defaults | Rich Character/Place/Item/Thread bundles, generic compact inspector, search and focused authoring | Empty-field development; partial profiles valid; only checked fields change; custom character preset gets the same sections/tools; one outfit truth. |
| C: Relationships and context | Typed link properties, shared task projectors, capability-aware media facts/fingerprints | Asymmetric attitudes; scoped link review; image/manual-prompt parity; structured-only appearance reaches captured image facts; budgets/omissions and relevant staleness. |
| D: Wider core catalogue | Organization, Lore, World rule, Story brief, Visual style and source-linked Event; existing Narration style retained | No empty-node clutter; explicit bindings; local rule scope; group additions reuse permitted IDs; event nodes cannot advance/replay story events. |
| E: Situated knowledge | Claim records, character stance/acquisition, reader disclosure and source-aware retrieval | False belief stays distinct from truth; plans do not occur; viewpoint and historical/fork boundaries exclude future disclosures; stale sources reported. |

For each slice use deterministic provider adapters and fictional projects first. Desktop checks verify usable forms, keyboard access, scrolling, proposals and navigation. Small explicit real-model probes later assess narrative usefulness; automated storage/schema tests are not evidence of good prose. No large third-party engine, vector service, faction simulation or generic workflow editor is required.

### End-to-end acceptance fixture

Create a fictional homestead, workshop, two characters, a failing pump, a work coat and a bag. Give one character an immediate repair objective and a reluctance to seek help; give the other a different objective and speech style. The workshop has relevant tools and restricted access. Add one asymmetric relationship and a repair thread.

1. Create the first character with their coat and bag; apply reviewed item/link records. Develop an empty Voice field without rewriting appearance, name or belongings.
2. Add a custom character preset and a visual field; confirm the same field protection, inspector, authoring and image-fact paths work without preset-specific branches.
3. Generate a short passage in which objectives and the workshop's access matter. Review proposed condition/item/link/thread changes individually; reject any unsupported change. Accept once.
4. Inspect before/after snapshots, move the coat in the current world, then inspect the past again. Earlier outfit and structured appearance remain unchanged; a historical image uses its captured source.
5. Add a household Organization and local rule without creating a second homestead or moving anyone. Group discovery remains bounded and read-only for established nodes.
6. When slice E exists, establish a false belief about the pump and a possible future reveal. Confirm neither becomes shared truth or an accomplished event; fork before disclosure and check exclusions.

Judge model output for distinct decisions/voices, useful interaction with places/items and continuity. These are explicit review questions, not infallible numerical story-quality scores.

## 11. Implementation audit anchors

Baseline source findings before the initial implementation. The checkpoint above records which gaps have since been addressed; remaining rows still guide later slices.

| Current source | Revision needed |
| --- | --- |
| [schema/world-presets.json](schema/world-presets.json) | Character/place markers are thin; thread has only resolved. Supply bundles while keeping native markers small. |
| [model.rs](crates/story-project/src/model.rs) | Fields already support references/long text. Add choice/metadata and rich-link values; separate mandatory native contracts from equality against the entire default catalogue. |
| [graph.ts](src/lib/graph.ts) | Exclusive display kind and description-only search cannot express all combinations or new fields. Keep placement separate from narrative grouping. |
| [Inspector.svelte](src/lib/Inspector.svelte), [PropertyInput.svelte](src/lib/PropertyInput.svelte) | Current generic capability controls can be reused, but need sections, unset/choice inputs, capability-driven features and derived summaries. |
| [WorldEditDialog.svelte](src/lib/WorldEditDialog.svelte) | Link editing currently handles endpoints/type/notes only. Rich properties need native previews and dependency handling. |
| [authoring.rs](crates/story-project/src/authoring.rs), [authoring-ui.ts](src/lib/authoring-ui.ts) | Preserve schema-derived field protection; extend schema/field metadata, editable addresses and captured link scopes. Revisit size estimates and sparse optional defaults. |
| [ai_context.rs](crates/story-project/src/ai_context.rs) | Current bindings include all component fields as a block. Add inspectable task/field projection without dropping explicit required inclusion. |
| [media.rs](crates/story-project/src/media.rs) | Shot appearance currently reads visualDescription; outfit capture reads garment text/coverage. Use structured extension projection and include its exact inputs in fingerprints. |
| [operations.rs](crates/story-project/src/operations.rs) | Reference cleanup/removal must include new link properties and source annotations according to their contracts. Historical records remain immutable. |

## References and limits

- Guided Story at `fdde895854875c431c8ae54eda637041e9a09bc3`: [character profiles](https://github.com/Ditters1234/guided-story/blob/fdde895854875c431c8ae54eda637041e9a09bc3/src/lib/character-profile.ts), [visual identity](https://github.com/Ditters1234/guided-story/blob/fdde895854875c431c8ae54eda637041e9a09bc3/src/lib/visual-identity.ts), [knowledge/context](https://github.com/Ditters1234/guided-story/blob/fdde895854875c431c8ae54eda637041e9a09bc3/src/lib/context-types.ts). Recover its profile separation, retaining our item-based wardrobe.
- [NarraWorld, September 2026](https://arxiv.org/abs/2609.32401): evidence-grounded separation of facts, beliefs, open developments and possibilities. Its results do not establish performance for our local model.
- [Pagekin](https://github.com/EthanYoQ/AI-Novel-Writer/blob/master/README_en.md) and the [pinned JSON writer](https://github.com/KKKenChow/ai-novel-writer/tree/e60a3501c63ab78880bd4a04fdca5c17ef16b9fa): structured profiles/current state, selective context and source-aware derived history. Reuse boundaries remain in [EXISTING_WORK_REVIEW.md](EXISTING_WORK_REVIEW.md).
- [How LLMs Build Fictional Worlds, September 2026](https://arxiv.org/abs/2609.02482): motivates evaluating useful character/environment interaction; not a universal claim about every model.
- [Snowflake Method](https://www.advancedfictionwriting.com/articles/snowflake-method/): motivation, concrete goals and obstacles are useful optional character-development prompts; its full plot formula is not mandatory.
- Additional design comparisons: [World Anvil organizations](https://www.worldanvil.com/learn/article-templates/organization) and [LegendKeeper linked events](https://www.legendkeeper.com/how-to-use-legendkeepers-timeline-feature/). These inform preset breadth and connected records, not code adoption or a requirement for their full systems.
