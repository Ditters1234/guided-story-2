# Guided Story image workflows

At the user's explicit direction, these two ComfyUI **API-format JSON data files** are copied from [Ditters1234/guided-story](https://github.com/Ditters1234/guided-story/tree/fdde895854875c431c8ae54eda637041e9a09bc3/AI/Comfy%20Workflow), revision `fdde895854875c431c8ae54eda637041e9a09bc3` (October 21, 2026). Graph data is unchanged; line endings are normalized to the repository's LF policy. No upstream application engine, model weights or custom node code is bundled. Model/custom-node dependencies retain their own installation and license requirements.

| File | Role | SHA-256 |
| --- | --- | --- |
| `guided-story/portrait.comfy-desktop.api.json` | Default; encoder and VAE under the Krea 2 subfolder | `343770c7bf662bf82c3ebe41153b3c3584305deefb3c1b2702996d5885701141` |
| `guided-story/portrait.api.json` | Original encoder/VAE directory layout | `dbb59cf47b82aca2a9eae4667e9dd6b7ffc96ad0b3f08e55ca762135780394ad` |

Both select the upstream Sick Ollie Krea 2 diffusion model, the Qwen3-VL encoder, Qwen Image VAE, the existing model-only LoRA and `SeeSeeCivitaiSaveImage`. Defaults remain 896×1152, seed 42, nine steps, CFG 1, Euler/beta and denoise 1. Keeping these files does not install or certify those dependencies on a ComfyUI server.

The native `guided_story_image_workflow()` profile binds:

| Control | Node/input |
| --- | --- |
| Positive prompt | `4.text` |
| Width / height | `6.width` / `6.height` |
| Seed / steps / CFG | `7.seed` / `7.steps` / `7.cfg` |
| Output | node `9` |
| Negative prompt | Unsupported: node `5` zeroes conditioning |

Preparation clones the graph and changes only supplied bound values. A nonempty negative prompt fails explicitly for this profile. Unbound model/LoRA/encoder/scheduler/save settings remain intact. Another model family can provide its own API graph and bindings without engine changes; no universal nine-step sampler or hardcoded loader name is imposed on it. See [model targets](../MODEL_TARGETS.md).

These are pinned templates for the forthcoming media service. The current app does not submit them, download weights or execute ComfyUI nodes.
