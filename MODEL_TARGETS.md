# Model targets and API defaults

Confirmed October 25, 2026: prioritize local LLMs around 30B parameters, including Qwen-class 27–35B models, and ComfyUI image generation using Krea 2, Z-Image or MOSS-Image families. Keep model IDs and workflow graphs configurable. Parameter count alone does not establish a model's context limit, server extensions, available VRAM or measured quality.

## Text

New connections use a Qwen-oriented non-thinking baseline: temperature **0.7**, top P **0.8**, top K **20**, min P **0**, output allowance **4,096**, context allowance **32,000**, and a 300-second timeout. This is a starting profile, not a claim that all Qwen generations or fine-tunes share optimal settings. It follows the [Qwen3-30B-A3B model card](https://huggingface.co/Qwen/Qwen3-30B-A3B) for non-thinking sampling. Keep budgets within the actual serving configuration; do not silently enable extended context.

Thinking controls are transport-specific. The preset uses `chat_template_kwargs: {enable_thinking: false}`, as documented for [Qwen served with vLLM](https://qwen.readthedocs.io/en/latest/deployment/vllm.html#thinking-non-thinking-modes). Settings also offer a top-level `enable_thinking: false`, matching Guided Story's adapter, and **Server default**, which omits the override. Generic API defaults omit top K/min P and thinking extensions. Unsupported parameters produce an error; there is no hidden retry or silent transport switch.

Existing connection files retain their saved temperature, model, endpoint, key and budgets. Missing sampling fields deserialize to generic compatibility values. **Settings → LLM → Sampling & API compatibility → Use Qwen 30B defaults** applies the new sampling preset explicitly. Presets leave endpoints, credentials, selected models and token budgets intact. Model discovery remains an explicit, inference-free GET; settings do not infer parameter count or change profiles from a model's name.

Optimize the task before increasing the model or context budget:

- One focused generation at a time, with explicit references and checked editable fields. World expansion does not include every descendant in a prompt.
- Revision requests include only the type/capability/relationship definitions needed to interpret their targets and selected references. Custom fields retain the same validation/protection rules.
- Supply a response schema once: in `response_format` for schema mode, or in the prompt for JSON/prompt mode. Budget the representation actually submitted.
- Prefer a compact group of up to eight new nodes unless the author requests more; the hard limit remains 20. Concise descriptions are a default, and explicit requests for more detail take precedence.
- Keep world creation, narration, effects extraction and optional image-prompt preparation as separate bounded tasks. First and later passages must share that pipeline. Do not add an agent/planning loop or reasoning request by default.
- Build future image captions from visible facts, pose, outfit items, composition and environment. Retain user-selected explicit appearance details and manual prompt edits. Image prompts contain neither UUID inventories nor story-planning instructions.

The [passage composer](STORY_AUTHORING.md) uses separate explicit narration and extraction requests, with selected scene records, directly carried/worn resources, the active chapter plan and two recent passages. [Chapter memory](MEMORY.md) adds up to 12 relevant historical points, within at most 6,000 bytes of JSON after reserving required context/output. Missing/stale/omitted coverage is visible. Summary generation is one explicit request, capped at the configured output allowance or 3,072 tokens, and sends only the chapter's accepted prose and a compact node-name directory. Oversized chapter requests block and support manual summaries. Character-belief filtering and automatic multilevel compaction remain future work.

These changes are verified with deterministic fixtures, not a quality benchmark against a real 30B model. Task-specific sampling, context retrieval and prompt length still need representative model probes. Do not claim throughput or VRAM savings without measuring them.

## Images

Use Guided Story's existing **ComfyUI API-format workflow** as the initial Krea 2 profile. The exact graphs and provenance are in [workflows/README.md](workflows/README.md). The desktop variant retains its installed-path conventions, custom save node and LoRA; do not substitute a different saver, encoder or model silently.

The [image service](MEDIA.md) keeps the reference endpoint default `http://127.0.0.1:8188` and its `/system_stats`, `/object_info`, `POST /prompt`, `/history/{promptId}` and `/view` contract. It persists the original endpoint, prompt ID and exact submitted graph for reconciliation. Submission and result checks are explicit; uncertain submissions never resend automatically.

One common image service uses per-workflow bindings for positive prompt, optional negative prompt, seed, dimensions, steps, CFG and output node. It preserves unbound graph values and captures the exact prepared graph at submission. Node portraits and story illustrations use this service with separate captured shot facts and editable prompts.

Z-Image and MOSS are separate workflow profiles, not checkpoint substitutions into the Krea graph. [ComfyUI's Z-Image documentation](https://docs.comfy.org/tutorials/image/z-image/z-image-turbo) distinguishes Turbo and Base variants; their sampling/negative-conditioning needs differ. [OpenMOSS's ComfyUI guide](https://github.com/OpenMOSS/MOSS-Image/blob/main/comfyui/README.md) distinguishes native and plugin workflows and warns against carrying over incompatible sampling nodes. Retain each imported workflow's model, encoder, VAE, sampler, scheduler, conditioning and default parameters. The Krea workflow's nine steps/CFG 1 do not become global image defaults.

Native preparation validates API graphs, bindings, supported overrides and effective bound control values. Settings, preflight, durable submission/reconciliation, local prompt preparation, bounded previews and accepted media storage are implemented as a prototype. Preflight checks `/object_info` for required node classes and installed enum/loader choices and blocks missing dependencies without rewriting the graph. Tests use local fixtures; actual custom-node/model compatibility and image quality remain unverified. Optional LLM prompt preparation is future work.
