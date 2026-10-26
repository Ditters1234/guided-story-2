# Node images and passage illustrations

Images share one service and review flow. Any world node, including a custom preset, can have a selected image. Character portraits stay in the map and shared inspector. Accepted passage illustrations appear with their passage in the reader.

## Use images

- In a saved project's inspector, open **Image → Generate / choose image**. Choose **New shot**, write or edit its prompt, then **Generate image**. **Import image** uses the same candidate review flow for a local PNG, JPEG or WebP.
- In Story, read an accepted passage and choose **Illustrate passage**. Before/After selects the immutable source state; up to four subjects supply visible descriptions and worn garments. Changing source/subjects requires **New shot** before submission.
- After submission, **Check result** reads the existing job and downloads its finished image. Review it, then **Use as node image** or **Accept illustration**. Generation/import alone never publishes an image or advances story events.
- **Reroll image** creates another saved candidate, retaining the prompt/controls and choosing a new 32-bit seed when the profile provides a seed binding. Previous candidates and accepted files remain available. Selecting or clearing a displayed node image preserves its image history.

Unsent prompt/control edits autosave. Submitted prompts, exact prepared graphs, workflow profiles, original endpoints and returned prompt IDs are retained separately from accepted assets. Saved candidates reopen; reading/navigation never starts image generation or a result check. The app prevents closing/switching during an image HTTP operation or while a received result remains unsaved. A queued server job can remain saved while the author writes or switches projects.

If captured appearance exceeds the default prompt allowance, the candidate keeps its source facts and asks for a shorter manual prompt. It does not silently trim those facts or submit an oversized request. Invalid unsigned numeric input resets to the last valid value on leaving its field.

## Configure ComfyUI

**Settings → Images** stores its connection outside story folders. The initial direct, unauthenticated endpoint is `http://127.0.0.1:8188`. Test connection performs availability checks without inference. Generation preflight verifies the submitted graph's node classes, output class and supplied enum choices, including installed loader filenames. Missing dependencies block submission; the app does not replace custom savers, loaders or model families.

The transport follows ComfyUI's [`/system_stats`, `/object_info`, `/prompt`, `/history/{prompt_id}` and `/view` routes](https://docs.comfy.org/development/comfyui-server/comms_routes). One explicit POST submits a workflow. History checks and downloads use that job's original endpoint, even if connection settings later change. The completed history graph must match the submitted graph before its output is downloaded. See [ComfyUI's server implementation](https://github.com/Comfy-Org/ComfyUI/blob/master/server.py).

Guided Story's pinned Krea 2 profiles preserve their original custom nodes and parameters; see [workflow provenance and bindings](workflows/README.md). Settings can load a JSON profile containing an API graph and explicit positive/negative/seed/dimension/steps/CFG/output bindings. Z-Image, MOSS and other families require appropriate separate profiles. No Z-Image/MOSS graph or live compatibility is claimed as bundled. Unsupported overrides fail explicitly. Blank controls retain profile defaults. Bound dimensions/steps/CFG defaults and overrides are validated; graph numbers outside JavaScript's safe numeric range are rejected rather than rounded through the UI.

## Historical appearance and publication

A candidate captures an immutable world reference, exact accepted passage reference when applicable, and selected subjects' visible descriptions plus worn item descriptions, coverage and layers. Private story notes and unrelated records do not become prompt text. Appearance details and manual prompts remain author-editable; the app does not redesign the source node from an image.

Before/After are passage boundaries, not a reconstruction of every intermediate moment. The author reviews framing, pose, garment visibility and any intermediate details in the prompt. Layer ordering and coverage are reference facts, not a complete physical occlusion simulation. Custom appearance fields currently need inclusion in Visible description or the manual prompt; there is no automatic semantic classification of arbitrary capability fields. Carried items are not assumed visible merely because they are carried.

Relevant appearance/outfit or exact passage-version changes block submitting an unsent candidate. Already generated images retain their captured source and can be preserved; they are marked as mismatched when appropriate. Current inspectors can show a selected older image with an appearance warning. Historical maps/inspectors use it only when captured appearance/outfit matches that state; otherwise they show a placeholder. Illustrations from superseded text versions remain accessible with an earlier-version label, not silently attached as current evidence.

Acceptance publishes immutable media metadata and a story-index reference through the existing atomic head/receipt protocol. Image files are flushed first, stored under generated project-relative IDs with their actual PNG/JPEG/WebP format and SHA-256, and verified before publication/preview. A media commit preserves current prose, world and unrelated work. Media selection/publication alone does not invalidate checked passage context.

Forks copy accepted media only when their source world belongs to retained passage history and their passage reference matches a retained accepted version. They remap references and copy checked binary assets into the independent folder. Future-only images, superseded passage images and pending candidates/jobs are omitted. Portrait selections must additionally match the fork boundary appearance. Broken required accepted assets report an error rather than being silently replaced.

## Recovery and current limits

Submission is recorded before POST. A lost/malformed reply, interrupted submission or uncertain server error becomes **Submission uncertain**; it is never resent automatically. Recover the prompt ID from ComfyUI and **Attach prompt ID**, then check its result. A saved queued job survives restart. Empty history remains unresolved: it can mean still queued or cleared server history; it is not permission to submit again. In-memory replies that cannot be written block closing until saved. Rechecking an already saved download/acceptance is idempotent.

The initial slice supports one image from the bound output node, images up to 8 MB and 4096 pixels per dimension, bounded decoding, and previews up to 768 pixels (64 for map thumbnails). It allows 100 unaccepted candidates and 2,000 accepted images. Candidate JSON is capped at 2 MB; API graphs at 1 MB. No global ComfyUI interrupt or queue deletion is issued: server-side cancellation remains in ComfyUI. There is no hidden polling or retry loop.

Optional LLM shot/prompt assistance, evidence-anchored intermediate shots, reference-image conditioning, authenticated image connections, batch-image selection, full-resolution viewing/export controls and cleanup remain future work. Tests use fictional assets and local HTTP fixtures; real model quality, installed custom nodes/weights, historical visual accuracy and packaged-release behavior need separate validation.
