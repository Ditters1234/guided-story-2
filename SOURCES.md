# Events and source citations

Event is an optional creation preset over shared nodes. Its reusable `detail.event` extension records **What happened**, an optional fictional **When**, and **Significance**. The preset adds no calendar, simulation, automatic action or separate event ledger. Creating or editing an Event establishes an author-supplied description; it never moves characters, changes inventory, pays debts or replays passage effects. Future intentions belong in plans or thread direction.

Project-local descriptive links connect an Event to **Participant**, **Occurred at**, **Caused by** and **Resulted in** nodes. These links do not establish physical placement. Custom presets with the Event extension use the same editor, AI tools and provenance behavior; the preset name does not control it. Existing story definitions are not silently replaced when the application catalogue grows.

## Using Sources

Every shared inspector has **Sources (count)**. Event additions authored manually or applied through world AI receive an author-established backstory marker. Other nodes can also carry citations.

- **Author / backstory** records an explicit author source note.
- **Accepted passage** selects an accepted passage, its exact current text version and one continuous 8–600 character quote. An optional world-change selector requires the quote to occur within that effect’s evidence as well as the passage.
- **Review citation → Apply source change** previews a native-validated world edit. It changes citation metadata, without rewriting the node or accepted history. Citation selection does not call an LLM.
- **Read exact source** displays the retained cited prose, even when the passage has since been polished. Earlier wording and nodes edited since citation are labelled.
- **Remove citation** removes that reference, retaining the node and all accepted history. Removing an Event’s last passage citation explicitly leaves it author-established; the review explains the replacement with an author source. An Event’s sole author-source marker is retained. Add a replacement source before removing that author note, or remove the Event node itself.

Historical Sources are read-only. Citations record provenance and an author assertion of relevance; structural checks do not certify the semantic correspondence between every node field and its quote. A node can be edited after it was cited; its earlier quote remains intact and the inspector flags the changed record rather than presenting the old quote as proof of the new details.

## Events extracted from prose

An effects request may propose a significant Event and descriptive links, using the usual saved schema. An Event is optional; no record is created for every action or passage. Its card uses the usual quote, editing/removal and individual approval controls. Acceptance attaches a native passage-version/effect citation from that approved evidence and publishes it with prose, effects, chapter membership and Before/After snapshots in the same manifest transaction. Approval alone changes nothing.

Ordinary physical effects remain separate evidenced proposals. Recording “the pump failed” as an Event does not itself change the pump’s condition or another record. World AI can develop empty Event fields and propose backstory, but cannot fabricate provenance metadata or change source bindings through editable node fields.

## Native representation and boundaries

`World.sourceAnnotations` is optional metadata over node IDs, with typed author/passages origins and a captured node fingerprint. It is omitted when empty, preserving the serialized shape of existing worlds and receipt verification. Removing a node clears its current annotations; historical snapshots remain unchanged. AI context keeps source annotations scoped to supplied nodes. Ordinary narrative projection includes the scoped source metadata, and applicable source/schema changes participate in captured checks. Source anchors’ complete prose is not automatically added to model context.

Each new accepted text version has a stable text-version identity, an event-generation identity and a checksummed immutable `source-anchors/` record. The anchor contains exact prose/effect quotes without world pointers, avoiding a passage → after-world → passage checksum cycle. One anchor is shared across that version’s citations. Wording edits preserve event identity, create a new text anchor and register earlier wording anchors. Event replacement uses a new event identity and cannot reuse superseded evidence as current proof.

Native validation checks citation targets, source identities, exact quotes, optional effect scope, checksums and the passage’s registered current/earlier text anchors. An arbitrary invented “earlier wording” ID or a valid-looking orphan quote file is not an accepted source. Current-world sources must refer to retained history; historical reading checks its passage boundary.

Forks copy anchors needed by retained worlds/text and retain only required earlier-wording anchors, with exact source bytes and stable identities. They exclude future events, future anchors, pending work and unneeded alternate wording. The source project remains unchanged; the copy opens independently. Backups preserve source anchors, retained versions and drafts through the explicit file allowlist, validating source associations before restore publication.

Limits are 32 annotations per node, 10,000 per world and 512 retained earlier-wording anchors per passage. A full Event source history is reported on its proposal card before acceptance; remove an unneeded source or the Event proposal to continue. Exact quotes allow whitespace differences but no paraphrases, stitched excerpts or inserted ellipses. Fictional time labels are descriptive. Sources do not grant characters knowledge of an Event or protect undisclosed information. Viewpoint-aware claim/belief/disclosure filtering remains a later feature.
