# Chapter memory and older context

Story memory combines accepted passage notes with reviewed chapter summaries. It supplements the last two passages of prose and the selected scene records. It does not simulate offscreen events, infer what every character knows, or override the current world.

## Prepare chapter memory

1. Open **Story → Chapter memory**, choose a completed chapter, and create a **New candidate**. Saved candidates can be reopened; manual edits autosave separately from published memory.
2. Write historical facts manually with **Add memory point**, or choose **Generate summary** for one explicit request using the existing LLM connection. The right pane shows accepted source text. Each point names its source passage, an exact quote, and optional related nodes.
3. Review the proposed facts and quotes, edit them, then **Apply memory**. Publication changes the memory reference atomically and leaves passages and world facts unchanged. Ordinary passage acceptance never starts a hidden summarization call.

Each summary holds up to 12 concise points, each with at most 1,000 characters of text and an 8–600 character source quote after whitespace normalization. Points can link up to 12 supplied node IDs; the directory includes up to 128 nodes referenced by the chapter's plans/notes or affected by its accepted changes. This includes introduced/changed nodes and the endpoints of changed/removed links, so an item handoff or newly established place can be named even when it was not a scene-plan reference. References are optional. An empty summary requires the explicit **No lasting facts to summarize** choice. Quote matching and structural checks cannot prove a factual interpretation correct or complete; author review remains necessary.

Generate uses only that chapter's accepted prose and a compact node-name directory, without current profiles, future passages, chapter plans or pending drafts. New generation candidates preserve prior saved work. The offline demo copies short source excerpts; it is not an LLM-quality example. Oversized chapters block before submission and can still receive manual points using individual source passages. Automatic chunking and multilevel summaries are not implemented.

The directory resolves canonical IDs/names from each accepted source's immutable before/after boundary, rather than guessing from retained effect aliases, using draft allocations or looking up today's world. Provider schemas restrict passage IDs and node IDs to the supplied sources/directory; native validation independently checks those references and source quotes. This does not expand the summary request to full world profiles or authorize future knowledge.

Malformed or truncated responses remain available in **Response JSON / manual repair** and **Original model output**. Parsing repaired JSON changes the candidate, not published memory. Failed/cancelled/interrupted requests retain drafts and never resend automatically. One text request runs at a time across world tools, prose and summaries.

## What the writer receives

- Current structured scene records and recent prose remain the primary context.
- Current summaries represent completed chapters. Older passages without a current summary can supply their accepted continuity notes instead.
- Selection favors explicit scene references, viewpoint, participants, location and other selected resources, then recency. Related-node IDs work with custom node presets too; name matching is a fallback for unlinked points.
- Each request includes at most 12 historical points within a maximum 6,000-byte JSON allowance. Required system/schema/scene/prose context and output are reserved first; optional memory shrinks to fit. Required context that still cannot fit blocks before a provider request.
- **Generation context** shows selected points and their source passages, current/missing/stale/unavailable summary counts, older passages without summaries/notes and available points omitted by limits. These counts describe source representation, not complete recall of every detail. A reviewed summary can deliberately contain no lasting facts.

The existing conservative byte-based token estimate is not the provider's tokenizer. This improves bounded older-context retrieval; it does not establish unlimited novel-length recall, semantic accuracy or measured 30B-model quality.

## Revisions, corruption and forks

Memory records cite exact accepted passage record versions. Wording edits, event replacement, changed chapter contents/title or reopening a chapter make its summary stale. Stale/unavailable summaries are excluded from generation. Accepted passage notes may still provide fallback history. A new candidate can start from the older summary for manual revision, visibly labeled; its quotes must match the new source text before applying. Reopened chapters must close again before creating replacement memory.

Passage checks fingerprint the selected memory and its request budget. Changing applicable memory invalidates dependent checked prose. Manual validation/rechecking remains explicit. Current-world edits and later passages are preserved when applying an older chapter's summary.

**Fork after this passage** copies only current summaries for fully retained, completed chapters, remapping their source references to the fork's immutable passage records. Partial, future, stale and unavailable summaries and pending candidates are omitted. The copy works without its source folder.

Chapter memory lives in immutable `memories/` JSON records referenced by the story index, with `memory-drafts/` and archived candidate versions separate. Atomic head publication and receipts reconcile an Apply whose status save was interrupted, even after newer summaries or world work. Earlier projects default to an empty memory map; old passage drafts require rechecking after the memory prompt-contract update. Native corruption errors preserve files and report unavailable coverage instead of silently resetting the story.

Up to 200 unpublished memory candidates can be retained. Cleanup, full historical-memory browsing, broader knowledge automation, general pinned facts and large-world performance/release certification remain future work. Recorded claims and their source-linked memory already obey the bounded perspective rules described in [Knowledge](KNOWLEDGE.md).
