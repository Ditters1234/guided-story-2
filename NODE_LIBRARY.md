# Reusable node library

The library is a normal folder of JSON files. It is read at runtime: new entries, edits and shared packs need no compilation or application update. In World mode, open **Library**, then **Choose folder…**. The default is the `node-library` subfolder of the app configuration directory; the chosen folder is remembered on this device. Keep it outside story projects and build folders so it survives replacing the executable.

Select one node or a group on the map, then use **Selection → Save selection to library…**, or right-click **Save to library…**. Give it a name and optional description, review what is included and save. Each entry gets a new JSON file; existing files are never overwritten by this action. External editors may edit/rename library files. Use **Refresh** to discover additions or changes while the app is running. The supplied `examples/node-library` files (also in the testing build's **Library examples** folder) are optional examples you can copy into your own library.

Drag an entry's card onto the canvas, or choose **Add…** to add near the visible map. Both open **Add from library**: Review changes, then Apply. Cancel leaves the story unchanged. Dropping onto a story node sets only the map position; use the ordinary connection tools to attach it afterward.

Each insertion creates independent node and relationship IDs, retaining the entry's names and values. Relationships and typed references within the group follow the new instances, including nested storage and wearing. Relative map positions are preserved. Current story nodes and history are untouched; normal autosave publishes the imported group and any missing schema definitions together. Imported stories stand alone: moving, editing or removing the library files cannot change their instances, snapshots, backups or forks.

Only the explicitly selected nodes are exported. A collapsed parent's hidden children are not selected automatically. Connections to unselected nodes are omitted; external typed references and schema defaults are cleared with review notices. Required references that cannot be cleared block export: select the referenced nodes or revise the reference first. Text notes are retained verbatim; the exporter does not find or remove facts written in free text.

Images, story citations, author-source annotations, AI context inclusion and pending jobs are excluded. An imported Event gets new manual author provenance through the usual native pipeline; it does not cite another story or replay events. Importing style/instruction nodes does not enable them for AI tasks; configure AI context explicitly in the destination story.

Custom node types, extensions, typed relationship fields and their schema dependencies travel in the JSON. Matching definitions are reused; missing definitions are added on Apply. Conflicting definitions with the same ID block import with an error rather than replacing the story's definition. Repair the IDs and their uses in the library JSON, or use a compatible story. Custom schema defaults tied to copied nodes get independent definitions per insertion; native `core.*`/standard `detail.*` behavior cannot be renamed to bypass its contracts. Instance-specific defaults on those definitions need an unset optional default or a custom definition before importing.

The folder browser reports invalid files individually. A selected file is pinned by its checksum until loaded: if it changed, refresh and select its new version. The reviewed data is then captured in the import dialog; later file edits affect future imports only. Historical maps are read-only. Nothing invokes an AI provider.

## JSON format

Library format 1 is a data-only envelope over the application's existing world records:

```json
{
  "schemaVersion": 1,
  "name": "A reusable group",
  "description": "Shown in the library browser",
  "world": {
    "nodes": [],
    "edges": [],
    "nodeTypes": [],
    "componentTypes": [],
    "relationTypes": []
  },
  "positions": {}
}
```

This shows the envelope only; an actual entry needs at least one valid node, its required definitions and all native capability/relationship contracts. Start with an exported entry or one of the complete examples. Node/link IDs are UUIDs local to the entry. Edge `sourceId`/`targetId`, typed node references and reference defaults must resolve inside the entry. `positions` is optional and maps those node IDs to `{ "x": 0, "y": 0 }` values; without it, the usual hierarchy layout is used. Unknown fields, damaged references and unsupported versions produce errors rather than being discarded.

One regular `.json` file represents one menu option, whether it has one node or many. Files are directly inside the chosen folder; subfolders and symlinks are not loaded. Limits are 16 MiB per file, 500 entries and 64 MiB inspected per refresh. Ordinary world/schema limits still apply. Hidden staged `.tmp` files from interrupted writes are excluded. Library settings stay in device configuration; story backups contain the imported instances, not the external library folder. Back up/share that folder separately.
