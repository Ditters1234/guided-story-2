# Appearance settings

Open the gear button in the application header. **Appearance** selects System, Linen (light), Dark, Sepia or a saved custom theme, sets text size from 85% to 140%, and enables reduced motion. System follows the OS color preference, including changes while the app is open. The OS reduced-motion preference is also respected.

Text size scales interface text, form controls, node labels and connection labels. Map zoom remains independent. Themes use shared color tokens across the map, nodes, inspector, settings and editing dialogs.

## Custom themes

In **Custom themes**, choose **New from current**, name the theme, change its color tokens and control style, then **Save theme**. Saving selects it. Existing custom themes can be edited or removed. Up to 12 custom themes can be stored.

**Export JSON** downloads the editor's theme as a portable `.theme.json` file. **Import JSON** loads a file into the editor for review; save it to add/select it. **Edit JSON** exposes the same document directly. The theme document contains `schemaVersion: 1`, an ID (`custom.` plus a UUID), name, mode (`light` or `dark`) and these `colors`:

`background`, `surface`, `surfaceAlt`, `text`, `muted`, `border`, `accent`, `accentText`, `selection`, `canvas`, `canvasDots`, `connection`.

All colors use `#RRGGBB`. Themes are data, with no executable CSS, fonts, URLs or scripts. Importing an exported built-in theme gives it a separate custom ID. [schema/themes.json](schema/themes.json) contains the built-in palettes and provides complete examples. User-selected color combinations are not automatically certified for contrast.

## Storage and expansion

Desktop preferences autosave to `preferences.json` in Tauri's per-user application config directory. They stay outside story folders and snapshots. Writes use the existing atomic publication primitive, a settings lock, an independent version and replay-safe retry. Invalid/unsupported files are reported and retained; they are not silently overwritten with defaults. Closing settings or the app flushes pending appearance changes.

The settings dialog has Appearance, Custom themes and a working [LLM connection page](AI_AUTHORING.md). Image API settings remain future work. API credentials do not belong in portable themes or story files.

The browser-only preview applies settings for the current tab without desktop persistence. Debug desktop runs can set `GUIDED_STORY_PREFERENCES_DIR` to an isolated test directory; release builds always use the per-user app config location. Custom-theme editor fields remain temporary until **Save theme**.
