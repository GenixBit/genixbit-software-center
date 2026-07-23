# Accessibility and keyboard navigation

GenixBit Software Center provides a deterministic keyboard path through every primary page and keeps visible text on the global refresh control so it has an explicit accessible name and mnemonic.

## Global shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+R` | Refresh local package, service, Activity and AppStream metadata |
| `Ctrl+F` | Focus the search field on the currently visible searchable page |
| `Ctrl+Q` | Quit the application |
| `Alt+1` | Dashboard |
| `Alt+2` | Discover |
| `Alt+3` | Installed |
| `Alt+4` | Updates |
| `Alt+5` | Activity |
| `Alt+6` | Software Stacks |
| `Alt+7` | Security |
| `Alt+8` | Services |
| `Alt+9` | System Profiles |
| `Alt+0` | Settings |

The Refresh button also exposes the `_Refresh` mnemonic in its visible label. Standard GTK keyboard behavior remains available for `Tab`, `Shift+Tab`, arrow-key selection, `Enter`, `Space`, and `Escape` where supported by the focused widget or dialog.

## Screen reader and focus audit

The native GTK and Libadwaita widgets preserve their platform accessibility roles. The audit requires:

- visible text for the global refresh action instead of an icon-only control;
- unique page names and unique page-navigation accelerators;
- a keyboard destination for every page in the sidebar;
- `Ctrl+F` mappings for every page that exposes a search entry;
- status text represented by GTK labels rather than custom-painted content;
- no keyboard-only action that bypasses the existing read-only safety boundaries; and
- no focus trap introduced by page navigation or search focus.

CI executes `scripts/validate-accessibility.py` and its focused unit tests. The validator compares the actual GTK stack pages with the shortcut map, checks the page-aware search mappings, validates the mnemonic refresh control, and verifies this documentation remains synchronized.

## Manual release checklist

Before publishing an image, test the release build with a supported Linux screen reader and keyboard only:

1. Traverse the header, sidebar, page controls, lists and dialogs with `Tab` and `Shift+Tab`.
2. Confirm every focused control has a visible focus indication.
3. Activate every page with `Alt+1` through `Alt+0`.
4. Confirm `Ctrl+F` focuses the correct search field and does nothing harmful on pages without search.
5. Confirm list rows and dialogs are operable with `Enter`, `Space`, arrow keys and `Escape` where applicable.
6. Confirm status, error and empty-state messages are announced after refreshes and filter changes.
7. Verify text remains usable at increased system font and display scaling.

This milestone changes navigation and accessibility behavior only. Install, remove, upgrade, downgrade, repository-refresh and service-control operations remain unavailable; package-changing operations remain fail-closed.
