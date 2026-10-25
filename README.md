# Guided Story 2

A Windows desktop tool in development for crafting illustrated stories through an editable world map. Built around Tauri + Svelte, configurable model APIs and self-contained JSON story projects.

**Status: early desktop foundation.** The Tauri/Svelte prototype has an expandable world map, shared inspector, manual node creation, custom node presets/text fields, addable capabilities and JSON project storage. AI writing, saved portraits, passage history and forks are planned; they are not implemented yet. See [foundation status](FOUNDATION_STATUS.md) for verification and limits.

## Run the prototype

Requires Node.js 24+, Rust 1.90+ (MSVC target), Windows C++ build tools and WebView2. The workspace-local Rust toolchain, when present in `.tools/`, is detected by the scripts; it is not included in Git.

```powershell
npm ci
npm run desktop
```

The opening world is a fictional example. Create a project to save/edit it, or start with an empty world. Each project gets its own folder. A browser-only preview is available with `npm run dev`; native saving requires the desktop app. Existing project folders can also be opened with `npm run desktop -- -- --project "C:\path\to\project"`.

Checks: `npm run check`, `npm test`, `npm run build`, `npm run native:test`, and `npm run native:check`. `npm run bindings` regenerates checked-in frontend contracts from Rust.

## The intended experience

Start with a story idea and let an LLM propose settings, characters, places, possessions and a scene plan. Review and revise that foundation, then write the first passage through the same composer used for every later passage.

- Expand locations into subplaces, characters, carried/worn items, containers and connected story threads.
- Edit everything through the map and one shared entity inspector; character nodes can display portraits.
- Use manual edits or AI creation, revision and reroll tools for individual fields, records or connected groups. A character can be created with their outfit, satchel and its contents.
- Treat clothing as actual inventory items, with wear assignments and derived outfit descriptions shared by narration and image preparation.
- Write one passage at a time, grouped into chapters. Review prose and world changes before accepting them.
- Browse a chapter-based table of contents with Previous/Next controls and passage-linked historical world states.
- Create an independent story using **Fork after this passage**.
- Generate optional character portraits and passage illustrations through ComfyUI.

Valid manual edits and unfinished drafts autosave. AI world proposals require **Apply**; generated prose requires **Accept passage**. Each accepted passage retains its before/after world snapshots. Projects live in user-selected folders; connection settings remain separate.

## Design and build documents

- [Product design](DESIGN_REVIEW.md): confirmed decisions, authoring flows, records, media and history.
- [Review and build plan](REVIEW_AND_BUILD_PLAN.md): state contracts, risks, implementation gates and acceptance walkthroughs.
- [Existing-work assessment](EXISTING_WORK_REVIEW.md): recent references, inspected commits, licensing notes and selective reuse recommendations.
- [Reference checkout manifest](references/README.md): reproducible upstream revisions for local research checkouts.

## Technical direction

- **Desktop:** Tauri + Svelte, Windows first.
- **Supporting tools:** TypeScript/Vite, Svelte Flow, Paneforge and Rust, with Tauri Specta generating command/type bindings.
- **Text:** configurable OpenAI-compatible Chat Completions service.
- **Images:** configurable ComfyUI workflow API.
- **Storage:** versioned JSON with atomic revision publication, recovery and per-project locking. Binary images remain separate files.

The first implementation gate proves desktop/map feasibility and reliable JSON project create/open/save behavior. Then build one complete world → reviewed passage → historical state → independent fork workflow before expanding the feature set.

## References and reuse

[Guided Story](https://github.com/Ditters1234/guided-story) is the behavioral reference; this is a fresh implementation. The approved reuse direction is Svelte Flow for map interactions, selected MIT desktop/UI components from the [FrostyBee starter](https://github.com/frostybee/tauri-svelte-template), and a compatibility test for [Tauri Specta](https://specta.dev/docs/tauri-specta). Recent writing tools and memory research are assessed in the existing-work document.

Downloaded reference repositories are ignored, not vendored into application source. A small sidebar wrapper is adapted from the MIT starter; [third-party notices](THIRD_PARTY_NOTICES.md) preserve its attribution. Story storage and domain code are independent. A project-wide source license has not yet been selected.

Do not commit personal stories, generated private media, API keys, connection overrides or diagnostic output containing them. Development checks should use fictional fixtures and isolated project folders.
