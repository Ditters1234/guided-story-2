# Guided Story 2

A planned Windows desktop tool for crafting illustrated stories through an editable world map. Built around Tauri + Svelte, configurable local/model APIs and self-contained JSON story projects.

**Status: design and implementation planning.** There is no runnable application, installer or tested desktop prototype yet. This repository records the agreed product design, recent-work assessment and phased build plan.

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
- **Proposed supporting tools:** TypeScript/Vite, Svelte Flow and Rust; evaluate Tauri Specta for generated command/event bindings.
- **Text:** configurable OpenAI-compatible Chat Completions service.
- **Images:** configurable ComfyUI workflow API.
- **Storage:** versioned JSON with atomic revision publication, recovery and per-project locking. Binary images remain separate files.

The first implementation gate proves desktop/map feasibility and reliable JSON project create/open/save behavior. Then build one complete world → reviewed passage → historical state → independent fork workflow before expanding the feature set.

## References and reuse

[Guided Story](https://github.com/Ditters1234/guided-story) is the behavioral reference; this is a fresh implementation. The approved reuse direction is Svelte Flow for map interactions, selected MIT desktop/UI components from the [FrostyBee starter](https://github.com/frostybee/tauri-svelte-template), and a compatibility test for [Tauri Specta](https://specta.dev/docs/tauri-specta). Recent writing tools and memory research are assessed in the existing-work document.

Downloaded reference repositories are ignored, not vendored into application source. No third-party application code has been integrated yet. Preserve upstream notices when importing code; a reference's license does not automatically define this project's license. A project-wide source license has not yet been selected.

Do not commit personal stories, generated private media, API keys, connection overrides or diagnostic output containing them. Development checks should use fictional fixtures and isolated project folders.
