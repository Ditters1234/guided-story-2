# Editable AI prompts

Open **Settings → Prompts**, choose an AI tool and edit its full **System instructions**. **Save prompts** stores all edited tools; **Done** also saves pending edits before closing. The Story toolbar and AI world tools footer have a **Prompts** shortcut, opening the relevant writing/world instructions. Wording drafts open the wording instructions.

| AI tool | Used for |
| --- | --- |
| World creation & rerolls | Individual nodes, connected groups and selected-field revisions, including custom presets |
| Passage writing | First and later passages through the shared composer |
| Scene and chapter planning | Editable direction suggestions for the current scene or chapter |
| Wording edits | Polishing accepted prose while preserving events |
| World-change extraction | Proposed node/link changes and continuity notes from the current passage |
| Chapter memory | Source-linked summaries of accepted chapter history |
| Image-prompt assistance | LLM suggestions for shot notes, positive prompts and supported negatives |

The editor shows the tool's base system message. A custom message replaces that base in full. **Show built-in default** provides a read-only reference; **Restore default** stages that tool's original instructions for saving without changing other tools. Story-level [AI context bindings](AI_CONTEXT.md) add explicitly selected instruction nodes and separate reference facts; their manager previews the assembled message. There is no placeholder syntax: the app supplies direction, scoped context and output schemas separately. Scene directions, chapter plans and image-candidate positive/negative prompts keep their existing editors.

These defaults apply to all stories on this device. They affect future requests; editing or saving them makes no LLM/ComfyUI call and does not rewrite accepted prose or world facts. Prompt settings participate in the existing source fingerprints. A dependent pending candidate/check may need revalidation or a fresh reviewed generation after a change, as with other connection changes. The offline demo remains deterministic sample output rather than a model following creative instructions.

Native output schemas, evidence matching, context bounds and Apply/Accept validation still run. Instructions count toward the existing input budget; an oversized request fails before calling the provider. Keep the JSON/evidence requirements appropriate to extraction, world creation and memory tasks when customizing those tools.

Passage and planning requests include a native `writingBrief`: current passage direction, stopping point, remaining chapter direction, optional author recap and opening/continuing status. Built-in prompts receive these direction values once. Recent prose identifies its chapter relationship, and no older saved chapter plan overrides the edited draft. Saved custom system messages remain unchanged; their established `plan.direction`, `plan.chapterPlan` and `currentChapter.plan` paths remain available, with the chapter alias drawn from the current draft. Custom prompts can adopt `writingBrief` explicitly or use **Restore default** to take the revised built-in writing instructions. Recap text is author guidance, never independent evidence for world effects.

The built-in world base provides common instructions. The request builder supplies the active creation/revision operation, permissions and schema as task data. Saved custom world instructions continue to replace the shared base; **Restore default** adopts the revised common wording. Native preflight validates assembled scope and output contracts before generation. It checks structural consistency, while interpretation of freeform instructions still requires author review.

## Saving and compatibility

Prompt overrides use an independently versioned section in device-local `llm-connection.json`. They share its locked atomic publication but do not change the transport version. Saving an older open API form preserves current prompt edits; prompt saves preserve transport settings and encrypted credentials. The prompt-editor command returns template text and prompt revision only, without API credentials. Conflicting prompt edits require a deliberate reload; failed saves leave edits visible. Loading/saving limits bound each override, their total and encoded size.

Uncustomized legacy connection settings retain their serialized shape. Existing built-in instructions were moved unchanged into text resources. Older binaries may reject the new prompt settings fields after overrides are saved; use the current source/app version. Current configuration is device-local; captured generation settings/provenance retain the existing project-history behavior.
