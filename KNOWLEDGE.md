# Claims, character stances and narration

**Add node → Knowledge / claim** creates an ordinary shared node with the reusable Claim capability. Its fields separate a proposition, its author assessment, narration access and related subjects. A custom preset can include the same capability. Creation supplies no proposition, sets **unconfirmed / withheld**, and neither creates a belief nor advances events. Existing schemas install the optional definitions only on explicit creation; historical schemas and receipts remain unchanged. The optional native contracts are pinned separately in `schema/knowledge-core.json`.

**Author assessment** is established, unconfirmed or disproved. It is independent of anyone’s belief. For example, “the pump is cursed” can be disproved while Mara believes it. Related subjects indicate relevance, not knowledge, presence or ownership.

Use **Attach / link → Character stance** (displayed as **held by**) from the claim to a character. Choose believes, doubts or rejects. There is one such link per claim/character pair. **How learned / why believed** is an author description; an optional **Acquisition reference** can identify an Event, letter or other existing node. Neither possession of that source nor proximity to another character creates a stance. Proposition evidence uses the claim’s existing **Sources** control, with exact passage/version citations and edited-since-source indicators. The acquisition description/reference is not an independently verified exact-passage acquisition receipt.

## Narration access

- **withheld:** omit the claim’s record, proposition and connections from prose/wording context, including author perspective.
- **narrator:** include for a viewpoint with a recorded stance, or for explicitly selected Author / omniscient perspective. Reader disclosure remains false. Narrator information does not grant knowledge to other characters.
- **revealed:** available as reader information, even when the viewpoint has no recorded stance. This still does not establish character knowledge.

Access and author assessment are deliberate author metadata edits. They do not claim an event occurred or automatically mark a generated reveal as accomplished. Ordinary connection effects can propose stance changes for supplied claims through the usual individually approved passage cards. Prose effects cannot rewrite a claim’s author-planning profile or invent author truth. World AI tools can propose checked claim fields through their normal review/Apply pipeline. Automatic claim creation, truth adjudication and automatic disclosure/acquisition inference are not part of this slice.

## Scene context and history

In Story’s **Scene settings**, choose **Claim perspective** and expand **Claim context**. The native preview shows included propositions, assessments, reader disclosure, viewpoint/participant stances, custom Story fields, source warnings and omission reasons. Explicit references take priority; retrieval considers selected subjects and recorded stances, with at most 20 relevant claims. It does not expand every linked record or transmit the whole world. The author-facing preview may identify an omitted claim by name; that omission name is not put into the model request.

Prose receives claims in a separate labelled section, rather than as ordinary established-world records. Native projection removes claim records, claim links and excluded claim references from the ordinary prose payload. Required prose context bindings that conflict with the saved perspective block generation with an actionable message; required material is not silently dropped. The effective editable base prompts explain the distinction between author truth, character stance and reader knowledge.

Recent passages cited by an excluded claim, or containing effects/notes that reference it, are omitted. Typed memory points referencing excluded claims, source-linked withheld passages and summaries with excluded captured instruction sources are filtered; coverage reports omissions. Exact original wording/effects are required for a wording request. If they structurally reference an excluded claim, AI wording blocks instead of trimming the source; manual wording edits remain possible. Historical revisions use their saved before-world, not later discoveries.

World edits and stance changes participate in the scoped source fingerprint. Relevant edits invalidate pending checks; unrelated claims do not. Accepted before/after worlds are immutable. **Fork after this passage** retains that boundary’s claims, stances, source records and access, excluding later world edits. Full backups retain both earlier snapshots and later current work, with the normal native schema/reference/checksum audit and staged restore.

## Scope of protection

This is structural filtering for claim records in prose and wording, not a semantic secrecy filter for arbitrary writing. Direction, chapter plans, ordinary biographies, acquisition descriptions and unclassified prose can repeat the same information. Keep information intended to be withheld in separate claim nodes and cite its sources. Review generated prose for unintended inference or disclosure.

Effects extraction, node/group authoring and memory maintenance are author-review tools and can receive their explicitly scoped author material. Image tools keep their existing captured appearance and explicit image-context permissions; narration access does not silently override an author’s selected image references. The UI labels this scope. A model can still make unsupported deductions or misunderstand a stance; native structure does not certify narrative meaning.
