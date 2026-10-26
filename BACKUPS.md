# Project backup and restore

**Project → Export backup** saves a consistent copy of the open story in a new folder under the selected destination. Valid manual work and saved proposal editors flush first. A recoverable invalid world form can be backed up when its draft is saved. Finish/cancel active text generation and finish image HTTP operations/save retained replies first; queued ComfyUI jobs can remain queued.

**Project → Restore backup** selects the outer backup folder, validates it and shows its title, revision, file count and size. Choose a destination and **Restore to new folder**. **Open restored story** uses the normal project-opening/unsaved-work guards. Restore also works before opening a project. Neither operation replaces an existing project folder.

## Contents and format

The format is an ordinary folder, with `backup.json` recording a format version, story identity/revision and every included project-relative file's size and SHA-256. `project/` contains the original JSON records and binary images. An operation marker supports interrupted copies. Copy or rename the complete outer folder to move the backup; no original absolute project path is required. Restore creates a working copy while keeping the backup unchanged.

Included data: current head, retained revisions/worlds, accepted passages and earlier wording/event versions, chapter memory/evidence, accepted media/images, saved AI/passage/memory/image/prompt candidates and versions, receipts, editor drafts, map layout and recovery files. Invalid manual form values and malformed model responses remain draft data; backup does not apply them. Damaged recovery files are preserved as checked bytes, without claiming successful recovery.

Global appearance, connection profiles, encrypted keys, locks, temporary writes, unrelated files and reference checkouts are excluded. Recognized project directories use an explicit allowlist; unexpected files inside them fail validation. The result reports omitted unrelated entries. Saved jobs retain their original endpoint, workflow, prompt ID and provider provenance as story data. Backups are unencrypted local copies containing the story's private notes, drafts and retained model output.

Full restore retains story/local IDs and operation receipts so saved work remains valid, in a separate physical folder. **Fork after this passage** remains the distinct action for a new story identity and trimmed history. Switching physical folders resets the UI session even when story IDs match.

## Validation and interruption

Native validation checks bounded paths/files/sizes, duplicate paths including case variants, checksums, supported JSON schemas, world validation, draft identities, reference closure and historical passage/memory/media index identities. Referenced images are decoded under existing format/dimension/allocation limits and checked against recorded hashes/dimensions. Symlinks and Windows reparse points are rejected. Restore pins the exact manifest fingerprint reviewed by the author. Changed contents require a fresh selection and validation. Checksums detect changes; they are not a digital signature or a semantic story-quality check.

Copies use a sibling `.incomplete` staging folder. Files are flushed and the copy is validated before directory publication. Windows publication uses a write-through move without replacement. A destination lock serializes transfers into the same parent; export holds source project operations idle and serialized. Neither action sends provider requests. Restored known/uncertain image jobs retain their reconciliation state and never resubmit automatically.

Retry in the same dialog keeps the operation ID and captured revision/fingerprint. Matching copied files can be reused; only UUID-named atomic-write remnants in the owned staging tree are removed. Conflicting incomplete files are kept and reported. Completed retries return their original result and preserve later edits in a restored project. After interruption, a hidden incomplete folder can remain, but no partial copy is presented as complete. After reopening the app, a new transfer starts with a fresh operation; automatic transfer discovery/cleanup is not implemented. Completed restore envelopes can also remain as hidden staging bookkeeping. No recursive deletion is performed.

## Bounds and evidence

The prototype allows 25,000 included files and less than 4 GiB total, a 16 MiB per-record read bound, and existing 8 MiB/4096-pixel image bounds. Broken required historical records or unsupported versions block transfer without changing the source. Progress shows a busy state without a byte meter or in-app cancellation. Keep the app open until completion. Compression/single-file archives, prose-only export, scheduled backups, cleanup, migrations and large-library/release power-loss testing remain separate work.

Native fixtures verify independent roundtrip with custom nodes, old wording, memory, images, invalid saved forms and queued jobs; changed/missing/extra files; reviewed-fingerprint changes; path/size/schema/identity guards; occupied/nested/locked destinations; safe temporary-write cleanup and idempotent replay. Six actual process exits cover copy, validation and publication for export and restore. Desktop checks cover the Project menu and export dialog at 120% text. The full folder-picker/restore/open UI walkthrough remains pending.
