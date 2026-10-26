# World editing foundation

Pinning means a saved relationship between world nodes. Dragging a node changes only the map layout. Nodes remain generic: presets supply initial capabilities, and capabilities constrain typed connections.

## Controls

- Select a node and use **Connections → Attach / link**, or drag between node handles to open a new connection form.
- Use **Edit / move** on an incoming or outgoing connection, or click its canvas line. Change its source, relationship, target or notes. The saved connection keeps its ID.
- Use **Detach**, **Remove this node…**, or a capability's **Remove** control for removal.
- Choose **Review changes**, inspect the native-validated consequences, then **Apply changes**. Cancel leaves the world unchanged. An edited form invalidates its previous preview; applying also checks that the source world still matches.
- Add project-local relationship definitions through **Add node → Define a relationship…**. Choose source/target capability requirements, physical placement and one-target behavior.

The inspector lists incoming and outgoing connections even when endpoints are hidden by map collapse/search. Following a connection reveals its ancestor path. Canvas links retain canonical IDs and direction. Wearing is a physical placement, so a garment has one Worn by line and still appears in its wearer's inventory. Other narrative relationships can share endpoints.

## Structural rules

| Action | Result |
| --- | --- |
| Move physical placement | Replace the source's previous physical placement atomically. Nested contents stay with their holder. |
| Move a worn item to another holder | Replace Worn by with Stored in / carried by in the same operation. |
| Wear an item | Set one Worn by physical placement; replace any earlier holder, require quantity one and the relevant capabilities. |
| Take an item off but keep it | Edit its existing connection to Stored in / carried by with the same character. |
| Detach physical placement, including Worn by | Leave the source unplaced. |
| Remove a holder/node | Remove only that node. Keep direct contents unplaced, with their own nested contents intact. Remove incident links and clear typed references/default references. Leave matching free text untouched. |
| Remove a capability | Remove its field values only if the complete resulting world is valid. Resolve dependent links first. |
| Invalid/self/duplicate/cyclic connection | Reject the entire operation without altering the source. |

One physical parent applies across built-in and custom physical relations, including Worn by. Physical containment cycles are invalid; ordinary narrative relationships can form cycles. A custom one-target relation replaces that type's prior target without changing unrelated links. Built-in definitions cannot be weakened by extensions.

## Persistence boundary

Rust computes a complete preview from the supplied source world and validates the result. Apply changes feeds that complete world into the existing draft/autosave pipeline. Canonical saving validates again and uses revision checks, request deduplication and atomic head publication. Old world records remain immutable. The compatibility reader recognizes the original nonphysical Worn by definition and collapses its validated matching placement pair in memory, retaining the wear connection ID and merging both descriptions. Custom placement meanings are retained in the surviving notes. It rejects damaged pairs or notes too large to combine. Invalid property drafts remain editable. Opening does not publish a revision or rewrite old files/checksums; later saves write the simplified representation. Historical snapshots, forks and backups use the same reader. Completion receipts are checked against the original stored fields, preserving replay after an upgrade. New worlds reject dual physical placements.

Invalid ordinary property inputs remain recoverable editor drafts. Structural review forms are temporary until applied; they are not durable AI proposals. Current-world node removal is available now; passage snapshots, reader navigation and **Fork after this passage** still belong to the planned story pipeline. Stock splitting/merging, definition editing/removal and bulk deletion are outside this pass.

Workspace state is independent of world revisions. Deleted node positions/selection are pruned, and saved layouts are validated on both read and write. A malformed layout is reported and preserved rather than silently overwritten. Opening a project refits the map; subsequent field edits do not continually reset the viewport.
