# Guided Story 2

A Windows desktop tool in development for crafting illustrated stories through an editable world map. Built around Tauri + Svelte, configurable model APIs and self-contained JSON story projects.

**Status: world, passage and media authoring prototype.** The Tauri/Svelte app has an expandable world map, shared inspector, editable nodes/links, custom presets/capabilities and JSON project storage. Shared AI tools propose individual records or connected groups with explicit Apply. Story mode supports reviewed prose, chapters, historical snapshots, independent forks, wording/event revisions and source-linked chapter memory. Node images and passage illustrations support reviewed import or ComfyUI submission. See [foundation status](FOUNDATION_STATUS.md), [world editing](WORLD_EDITING.md), [AI authoring](AI_AUTHORING.md), [passage authoring](STORY_AUTHORING.md), [chapter memory](MEMORY.md), and [media](MEDIA.md) for behavior and limits.

## Manual testing

See [the testing guide](MANUAL_TESTING.md) for first launch, offline checks and configuring your own LLM/ComfyUI servers. A standalone Windows testing build runs directly from its executable, with the interface embedded and no development server. Build one with `npm run manual:build`; it creates a fresh ignored `build/manual-test-*` folder containing only the app, guide, third-party notices and build metadata. Keep story projects outside that folder.

## Run from source

Requires Node.js 24+, Rust 1.90+ (MSVC target), Windows C++ build tools and WebView2. The workspace-local Rust toolchain, when present in `.tools/`, is detected by the scripts; it is not included in Git.

```powershell
npm ci
npm run desktop
```

The desktop app reopens the last successfully opened story. With no saved choice, or if that folder cannot open, it shows **New story / Open story**. New stories start empty; the fictional Brasswater example is an optional checkbox during creation. Each project gets its own folder. A browser-only preview is available with `npm run dev`; native saving requires the desktop app. `npm run desktop -- -- --project "C:\path\to\project"` opens that explicit folder instead of the remembered story.

The header's Project menu opens/creates stories and offers validated folder backup/restore. The settings gear opens appearance, API and [editable AI prompt](PROMPTS.md) controls. **Settings → Prompts** edits the base system instructions for each text tool, with Save and Restore default. Custom themes can be edited and shared as JSON. App preferences and encrypted Windows credentials stay separate from story folders. Choose the offline demo provider to try AI proposal review without making API calls. See [backups](BACKUPS.md), [appearance settings](APPEARANCE.md) and [AI authoring](AI_AUTHORING.md).

`npm run verify` runs the frontend/native checks and tests plus the frontend production build. Individual checks remain available. `npm run bindings` regenerates checked-in frontend contracts from Rust.

## The intended experience

Start with a story idea and let an LLM propose settings, characters, places, possessions and a scene plan. Review and revise that foundation, then write the first passage through the same composer used for every later passage.

- Expand locations into subplaces, characters, carried/worn items, containers and connected story threads.
- Edit everything through the map and one shared entity inspector; character nodes can display portraits.
- Use manual edits or AI creation, revision and reroll tools for individual fields, records or connected groups. A character can be created with their outfit, satchel and its contents.
- Treat clothing as inventory items with one physical relationship: worn, carried, or stored. Wearing implies inventory membership; narration and image preparation share the same garment records.
- Write one passage at a time, grouped into chapters. Review prose and world changes before accepting them.
- Browse a chapter-based table of contents with Previous/Next controls and passage-linked historical world states.
- Create an independent story using **Fork after this passage**.
- Generate optional character portraits and passage illustrations through ComfyUI.

Valid manual edits and unfinished drafts autosave. AI world proposals require **Apply**. Passage world changes and continuity notes have individual editable approval cards; **Accept passage** publishes the reviewed set with its prose. Each accepted passage retains its before/after world snapshots. Projects live in user-selected folders; connection settings remain separate.

**AI context** lets any node supply story-wide instructions or reference facts to selected tools, with ordering, content-field selection and an assembled preview. Use a plain Note or a custom preset for narration preferences, world rules or other guidance. These bindings travel with the story and historical snapshots. See [story-wide context](AI_CONTEXT.md).

**Narration style** is a standard Add node preset with narrative person, tense and multiline instructions. Its creation shortcuts include it for passage writing and wording edits; its reusable capability, AI tools and context controls are shared with other world nodes.

**Composable node presets** bundle optional profile, appearance, personality, goals, voice and state extensions. Places, items and threads also have structured details. The shared inspector uses expandable sections; **Manage extensions** adds/removes extensions or changes a preset through native review. **Define extension** and **Define a custom node type** support typed fields, choices, reference filters and story/image/planning use. Clothing remains linked inventory, and structured visible fields reach image preparation.

This revision uses **story format 2**. Start a new story folder; older prototype stories are preserved but are not migrated. App appearance, API settings and credentials remain separate and retain their existing formats.

## Design and build documents

- [Product design](DESIGN_REVIEW.md): confirmed decisions, authoring flows, records, media and history.
- [Composable-node revision](WORLD_NODE_DESIGN.md): implemented initial slices, target relationship/knowledge design, task projections and remaining acceptance slices.
- [Model targets](MODEL_TARGETS.md): Qwen-class 30B defaults, scoped tasks and Guided Story ComfyUI workflow profiles.
- [Review and build plan](REVIEW_AND_BUILD_PLAN.md): state contracts, risks, implementation gates and acceptance walkthroughs.
- [Existing-work assessment](EXISTING_WORK_REVIEW.md): recent references, inspected commits, licensing notes and selective reuse recommendations.
- [Reference checkout manifest](references/README.md): reproducible upstream revisions for local research checkouts.

The initial node revision includes Character, Place, Item, Wearable item, Thread, Organization, Lore, World rule, Story brief, Narration style and Note presets. Presets are available without populating empty stories. Rich relationship properties, source-linked Event and Visual style presets, and situated knowledge remain later slices of the design.

## Technical direction

- **Desktop:** Tauri + Svelte, Windows first.
- **Supporting tools:** TypeScript/Vite, Svelte Flow, Paneforge and Rust, with Tauri Specta generating command/type bindings.
- **Text:** configurable OpenAI-compatible Chat Completions service, prioritizing Qwen-class local models around 30B.
- **Images:** configurable ComfyUI workflow API targeting Krea 2, Z-Image and MOSS families. Guided Story's Krea 2 templates, preflight, explicit submission/result checks and accepted project-local images are implemented. Separate workflow profiles preserve model-specific graphs. Real-provider compatibility remains unverified.
- **Storage:** versioned JSON with atomic revision publication, recovery and per-project locking. Binary images remain separate files.

The prototype connects world → reviewed passage → historical state → independent fork, with immutable text revisions and guarded latest-event replacement. Chapter memory adds reviewed older context with coverage tracking. Media captures exact source states, outfits and submitted workflows; optional LLM shot/prompt suggestions require review and Apply to an image draft. Validated folder backups preserve history, images and saved drafts in a separate restored copy. Knowledge controls, real-provider probes and release validation remain pending.

## References and reuse

[Guided Story](https://github.com/Ditters1234/guided-story) is the behavioral reference; this is a fresh implementation. The approved reuse direction is Svelte Flow for map interactions, selected MIT desktop/UI components from the [FrostyBee starter](https://github.com/frostybee/tauri-svelte-template), and a compatibility test for [Tauri Specta](https://specta.dev/docs/tauri-specta). Recent writing tools and memory research are assessed in the existing-work document.

Downloaded reference repositories are ignored, not vendored into application source. A small sidebar wrapper is adapted from the MIT starter; [third-party notices](THIRD_PARTY_NOTICES.md) preserve its attribution. Story storage and domain code are independent. A project-wide source license has not yet been selected.

Do not commit personal stories, generated private media, API keys, connection overrides or diagnostic output containing them. Development checks should use fictional fixtures and isolated project folders.
