# Existing work worth using or studying

Reviewed October 25, 2026. Priority window: roughly late July–October 2026. Dates below distinguish source activity, releases and paper publication; a recently crawled page is not evidence of recent development. Recommendations are not dependency approvals or claims that a library has passed our tests.

Adoption direction accepted October 25: proceed with Svelte Flow, selective desktop/UI reuse, a Tauri Specta compatibility test and the focused reference reading below. Exact dependency versions and imported files still require implementation checks; no third-party application code has been integrated yet.

## Recommendation

Use established UI/desktop components, selectively reuse a small desktop starter, and keep our story model, save protocol and acceptance rules purpose-built. Keep Guided Story as the main behavioral baseline. Study recent writing/memory work without adopting an autonomous-agent engine, another application's database or a new vector-store service.

| Work | Recent evidence | Suggested role |
| --- | --- | --- |
| Svelte Flow / xyflow | Version 1.7.0 released September 24, 2026 | Direct map dependency, subject to desktop feasibility test |
| FrostyBee Tauri/Svelte template | Inspected source commit August 31, 2026 | Selective desktop shell/UI reuse; not story persistence |
| Tauri Specta | Current official docs specify compatible release-candidate pins | Candidate typed command/event bridge; no recent release date claimed |
| NarraWorld paper | Submitted September 26, 2026 | High-priority story-memory design/evaluation reference |
| Pagekin / AI Novel Writer | v1.3.2 release listed October 22, 2026 | Workflow and recovery reference |
| KKKenChow AI Novel Writer | Inspected source commit September 9, 2026 | JSON/context/memory reference; selected algorithms may inform our implementation |
| How LLMs Build Fictional Worlds | Submitted September 2, 2026 | Narrative-quality evaluation reference |

## Candidates for the application itself

### Svelte Flow

The MIT-licensed core supplies node/edge interactions and custom Svelte nodes. Its September release is maintenance evidence, not a new story-world system. Use it for rendering and interaction; our records, typed links, expansion rules and validation remain authoritative. Sources: [release](https://svelteflow.dev/whats-new/2026-09-24), [repository and license](https://github.com/xyflow/xyflow).

First test: expand nested places/containers, show portrait nodes, preserve dragged positions and use a keyboard/tree fallback in Windows WebView2. Avoid copying paid example code merely because the core library is MIT.

Automatic layout is separate. Start with a predictable hierarchy layout; evaluate [ELK.js integration](https://svelteflow.dev/examples/layout/elkjs) only if linked neighborhoods need it. [ELK.js](https://github.com/kieler/elkjs) has its own EPL-2.0 license and integration cost; it is not a prerequisite for the first map.

### FrostyBee Tauri/Svelte template

Inspected commit: `c6a8b5644d51b1b3a64d6510509926fefb0b2187`, August 31, 2026. MIT, with separate third-party notices. It includes resizable sidebars, preferences, command palette, generated Rust/TypeScript bindings and a broad set of desktop plugins. [Source](https://github.com/frostybee/tauri-svelte-template/tree/c6a8b5644d51b1b3a64d6510509926fefb0b2187).

Reuse candidates: pane shell, dialogs, shortcut patterns and typed bridge. Strip demos and unused autostart, updater, deep-link and auxiliary-window features. Its single-instance plugin is not a substitute for locking an individual project folder.

The inspected JSON helper returns defaults on read/parse failures and uses a simple temporary-file rename. It does not implement our multi-record publication, flush, revision or last-valid-story recovery contract. Keep that helper out of canonical story storage. This is a source inspection finding, not a runtime failure report. [Helper](https://github.com/frostybee/tauri-svelte-template/blob/c6a8b5644d51b1b3a64d6510509926fefb0b2187/src-tauri/src/commands/json_store.rs).

The starter's declared Rust minimum and dependency pins need reconciliation with the selected Tauri version. [Tauri 2.12's September announcement](https://v2.tauri.app/blog/tauri-2.12/) raises its Rust minimum to 1.90. Do not assume an August starter builds unchanged against newer dependencies. No dependencies were installed and no starter tests were run here.

### Tauri Specta

Generates frontend types and wrappers from native commands/events. This directly addresses duplicate backend/frontend contracts. Current docs use Specta release candidates and require matching exact pins; the starter uses older RC versions. Evaluate one consistent version set rather than mixing examples. Bindings do not replace runtime validation. [Official docs](https://specta.dev/docs/tauri-specta), [repository](https://github.com/specta-rs/tauri-specta).

Recommendation: a small feasibility test with our snapshot/proposal types before adopting it. If serialization compatibility fails, choose another checked type-generation approach while preserving backend authority. No need for a general RPC framework or extra local HTTP server.

## References for story craft, memory and review

### NarraWorld — Shared Worlds, Private Minds

September 26, 2026 preprint. It separates world facts, character beliefs, open developments and hypothetical continuations, with evidence links and budgeted memory assembly. [Paper](https://arxiv.org/abs/2609.32401), [full text](https://arxiv.org/html/2609.32401v1).

Apply the separation and source-traceability ideas to our existing design. Add tests where a character believes something false, a planned reveal has not happened, and a fork must exclude future discoveries. Do not import its whole planning/retrieval pipeline or assume its reported results transfer to the user's local model. No reusable code repository/license was verified in this review; treat it as research reference.

### Pagekin, formerly AI Novel Writer

Recent primary release record: [v1.3.2](https://github.com/EthanYoQ/AI-Novel-Writer/releases/tag/v1.3.2), October 22. Its desktop app documents reviewed revisions, paragraph comparisons, job visibility and retained failed output. It uses Electron and local SQLite as well as project files, so its architecture is not our chosen stack. Desktop license: GPL-3.0. [README](https://github.com/EthanYoQ/AI-Novel-Writer/blob/master/README_en.md), [license](https://github.com/EthanYoQ/AI-Novel-Writer/blob/master/LICENSE).

Study the review/diff and recovery interactions. Do not copy its desktop code into our app as part of this recommendation; choose reuse terms deliberately if that changes. Its separately licensed small plugin is not equivalent to the desktop engine. Recent releases establish activity, not proof of reliable behavior in our environment.

### KKKenChow AI Novel Writer

Inspected commit: `e60a3501c63ab78880bd4a04fdca5c17ef16b9fa`, September 9, 2026. MIT. Python application with JSON storage, deterministic context selection and chapter-ledger rebuild/invalidation. [Source](https://github.com/KKKenChow/ai-novel-writer/tree/e60a3501c63ab78880bd4a04fdca5c17ef16b9fa).

Useful ideas: preserve per-chapter memory deltas and rebuild derived ledgers after changes. Adapt to our passage IDs and evidence, not name-keyed identity. Its JSON loader can fall back to an empty structure on failure, and its single-file saves do not satisfy our historical multi-file commit contract. Older English documentation still describes a vector-store architecture, so prioritize inspected source over that page. Reference selectively; do not add a Python backend. No runtime tests were run.

### How LLMs Build Fictional Worlds

September 2, 2026 preprint. For the tested models/texts, the authors report a tendency toward atmospheric/perceived space compared with human fiction's stronger action/environment interaction. [Paper](https://arxiv.org/abs/2609.02482).

Our inference: test whether map facts become meaningful action—using a room's layout, obtaining an item, opening a container—rather than repeated descriptive inventories. Add this to narrative-quality evaluation, not as a mandatory classifier or an assumed result for every model.

## Lower-priority or rejected candidates

- [Aetherbound](https://github.com/FGYue/Aetherbound): interesting persistent-state specification, but only a small public specification repository was observed and its README explicitly grants no license. Do not treat it as an importable open-source engine.
- [Covel mirror surfaced in search](https://github.com/insygna-research/census-ackness-covel): describes early-access autonomous plugin agents, parallel per-turn updates and RPG mechanics. Canonical upstream lineage and recent release date were not established. Not selected; its architecture would expand our scope and conflict with controlled authoring/acceptance.
- Complete agent orchestration frameworks or vector databases: no demonstrated need for them in the current plan. Reconsider only after measured context failures justify the added system.
- Mature supporting crates/components remain eligible even if older than the priority window. Recency alone is not a reason to replace a proven dependency.

## What to bring in now

1. Keep Svelte Flow as the leading map dependency, with a Windows feasibility gate.
2. Use the FrostyBee checkout as a source reference; extract only the desktop/UI pieces that pass our own checks when implementation starts.
3. Test Tauri Specta against the selected Tauri/Serde types; pin a compatible set.
4. Add NarraWorld, Pagekin and the JSON novel tool to the design/reference reading list. Keep our storage and story-state implementation independent.
5. Extend acceptance fixtures with belief-versus-truth, useful spatial action, memory rebuild and future-information exclusion cases.

At the research checkpoint, no application code had been integrated. Implementation now uses Svelte Flow and a small adapted sidebar wrapper, attributed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md); current verification is in [FOUNDATION_STATUS.md](FOUNDATION_STATUS.md). The two public MIT reference checkouts and the Guided Story checkout stay outside the application source/build/package. Preserve upstream licenses/notices for actual reuse.
