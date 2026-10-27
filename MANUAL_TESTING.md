# Guided Story 2 — manual testing

## Start

Double-click **Guided Story 2.exe** in this folder. This Windows build includes the interface; Node, Rust and the development server are not needed to run it. Windows and the Microsoft Edge WebView2 runtime are required. This is a development testing build, without an installer or automatic updates. `BUILD.json` identifies the source revision and executable checksum.

The opening map is a read-only fictional example. Choose **Project → New project**, give your story a name and choose a parent folder. New stories start empty unless you select the example. Each story has its own portable folder. Choose **Project → Open project** to reopen it later. Keep your working stories outside this testing-build folder so you can replace the executable independently.

The settings gear contains Appearance, Custom themes, LLM and Images. Existing device settings are preserved. Appearance and API settings stay separate from story folders. The offline demo provider lets you test proposal review without an LLM; its output is deterministic sample data, not a quality demonstration.

## First pass: no model required

1. **World editing.** Add a place, character and item. Edit their names and descriptions. Attach the character to the place; put the item on the character or inside a container. Check the map and inspector agree. Try a custom node preset or relationship if useful.
2. **Save/reopen.** Close normally, reopen the executable and open the project folder. Check your edits, layout and clothing/possessions survived. Valid edits autosave; invalid form input is retained as a draft and should show an error rather than silently entering the world.
3. **AI proposal review.** Select Offline demo in LLM settings. In **AI world tools**, describe an addition or connected group. Generate, edit the proposal and Apply. Proposed records should not appear in the established world before Apply. Try a targeted revision/reroll from a node's inspector.
4. **Story.** Switch to Story and choose **Generate passage** in the toolbar above the center editor. You can leave passage/chapter direction blank; check the editable location, viewpoint and present cast in the right pane. Alternatively provide direction or write prose manually. Generate creates a draft. **Check world changes with AI**, review the cards and approve each included change/note, then **Accept passage**. The contents list should update only after acceptance. Write another passage, optionally close the chapter, and inspect the next chapter's editable plan.
5. **History.** Use Contents and Previous/Next. Inspect Before/After world snapshots; they should be read-only. Return to Current world to edit. Try Edit wording, or Fork after this passage into a separate folder. Changing earlier events uses a fork; latest-event replacement has a separate reviewed action.
6. **Images.** In a saved node's Image section, choose Generate / choose image and import a small PNG/JPEG/WebP. Review it and choose Use as node image. For an accepted passage, use Illustrate passage. Import/generation alone should not publish an image. AI prompt tools use the LLM connection and Apply only prompt text; image generation remains separate.
7. **Backup/restore.** Export a backup from the Project menu into a different parent folder. Choose Restore backup and select the outer backup folder, then restore into a new folder and Open restored story. Verify prose, history, images and saved drafts; edit the restored copy and reopen it. The original and backup should remain unchanged. Select the outer backup folder, not its `project` child.

After generating, the footer should say **Draft not accepted · check world changes first.** Choose **Check world changes with AI** beside the disabled Accept button. Each proposed change and note should appear as its own card. Try editing a description, capability field or connection target, removing an unwanted card, and approving the rest. Removing a proposed new place/item should preview dependent proposals before removal. **Validate reviewed changes** checks edits without another API call. Approval must survive closing/reopening, but editing an approved card must require its approval again; unrelated approvals should remain. Accept must stay disabled until every included card is approved and the set passes validation. Checking and approving alone must leave the established story unchanged. Editing checked prose should disable Accept again. For a deliberate unchanged world, choose **Use no world changes**; it clears the proposals and validates without an AI call, then still requires Accept.

If evidence validation fails, the affected cards should show **Source quote · needs correction**. Use **Choose quote from passage** or edit a continuous 8–600 character excerpt, reapprove that card and **Validate reviewed changes**. AI-added ellipses that skip or join text are invalid; literal ellipses already present in the prose are allowed. **Advanced JSON** retains raw editing and indexed native diagnostics for repair, but should be collapsed during normal review. The saved passage should remain intact throughout; generating new proposals is a separate explicit action.

If all cards are approved but validation is blocked, the header should explain that correction/validation is still required. A broken connection should be marked **Needs correction** with its reason on that card. Open **Edit** and use **Connection to update** to choose an existing connection or **Create new connection**. Correct the relationship/endpoints as needed: **Inside** is for place nesting; a character uses **Present at**. Reapprove edited cards and validate. Other approvals should survive, and the saved prose should remain intact. An update/removal pair using the same connection ID should mark both cards; remove the redundant proposal or explicitly create a separate connection.

## Your LLM

In **Settings → LLM**, select the compatible API provider, enter the base URL and optional key, then **Test connection**. Select a returned model and save. A single returned model is selected automatically. Testing fetches models without generating text. Approximately 30B/Qwen models are the initial target; adjust token budgets and API compatibility to your server. Unsupported sampling/thinking parameters produce an explicit error; the app does not silently retry another transport.

Start with one small node addition, then a short passage. Review the actual output and world effects before applying/accepting. Test a failed or cancelled request and confirm your existing prose/drafts remain. Model compatibility, generation quality and long-story continuity are part of this manual testing; they are not certified by the offline checks.

In a fresh empty story, AI world tools defaults to **World / mixed group from an idea**. Try a short premise with a location and two adults: check that the people have personal names and developed profiles, and that clothing appears as separate wearable items linked to each wearer. Select a proposed character to inspect **Worn items**, or use **Add worn item** and edit the new item's appearance/coverage/layer before Apply. Missing-outfit warnings are advisory so deliberately unclothed characters remain possible. Earlier accepted characters are preserved; generate a new proposal to test the updated creation behavior. Existing character rerolls protect Name unless you check it.

Each worn garment should have just one **Worn by** link and appear in the character's **Worn & carried** list. To take it off while keeping it, use **Edit / move** and change that connection to **Stored in / carried by** with the same character. Change its target to a bag to store it there. **Detach** leaves the item unplaced. Older valid saves with separate inventory/wear links load as one relationship; their historical files stay unchanged. Pending AI proposals/checks from the earlier contract need a fresh review/reroll.

## Your image server

**Settings → Images** uses a direct, unauthenticated ComfyUI connection. Select the Guided Story Krea 2 profile matching your installation, or import an appropriate graph/binding profile for your model. Test connection checks workflow classes and installed choices without generating an image. Krea's loaders, custom nodes and filenames must match the installed workflow. Z-Image/MOSS require their own profiles; they are not checkpoint swaps in the Krea graph.

Try one small image. Generate image submits once; **Check result** retrieves the saved job explicitly. Review before accepting. A queued job retains its original endpoint and prompt ID across restart. An uncertain submission must be reconciled using its prompt ID; it never resends automatically.

## What to report

Record the build revision from `BUILD.json`, the action you took, what you expected, what happened, the exact error and whether restarting changed it. For provider issues, include the model/workflow name and relevant compatibility settings. Keep API keys and private story content out of public bug reports.

Current limits include bounded chapter memory, no character-belief filtering or general pinned-fact protection, no image reference conditioning, and folder backups below 4 GiB/25,000 files. Backup progress has no byte meter/cancel control; keep the app open during a transfer. Automatic interrupted-transfer discovery/cleanup, full version-history browsing, broad accessibility/scale testing and installer/release validation remain unfinished. Structural checks do not certify narrative meaning or image accuracy.

## Build from source

With the repository's Node/Rust/Windows build prerequisites installed, run `npm ci`, `npm run verify`, then `npm run manual:build`. The last command builds a standalone release executable and creates a new ignored `build/manual-test-*` folder containing only the executable, this guide, third-party notices and build metadata. It never packages stories, connection settings or development fixtures. `npm run desktop` remains available for development.
