# Development Log

A simple running record of changes to COSMIC Application Board.

## 2026-09-18

- Released version 1.0.1 with a branded internal application ID and a clean public history.
- Prepared the first public release as version 1.0.0.
- Added multiple named Launchpad pages, each with its own persistent spatial layout.
- Added **Hide from Other Apps**, with hidden apps remaining searchable and a Settings action to restore them.
- Added a `.deb` package builder and corrected desktop integration for system-wide installations.
- Added **Open in COSMIC Store** to application right-click menus.
- Gave application-provided actions, such as New Window and Private Window, a clear section in the menu.

## 2026-09-17

- Simplified the board to always use its responsive automatic grid and removed the redundant manual column controls.
- Added a one-click setting that makes the Super key open Launchpad, with no terminal setup required.
- Added kinetic trackpad scrolling with velocity-based momentum, gradual friction,
  and a seamless transition from finger movement into coasting.
- Added live Settings controls for kinetic scrolling, momentum strength, glide, and
  release response so the motion can be tuned per device.
- Adopted the tested kinetic-scrolling defaults: 300% momentum, 260% glide, and a
  4 ms release response.
- Reworked Settings into a narrower, height-limited two-column layout that scrolls
  as more options are added.
- Changed Settings into an overlay so opening it no longer pushes or rearranges the
  application board underneath.
- Reset Settings whenever Launchpad closes or opens so reopening with the Super key
  always returns to the application board.
- Made ordinary typing begin or continue app search regardless of which board element
  currently has keyboard focus.
- Made Escape close Launchpad immediately from the board, Settings, or search.

## 2026-09-16

- Added a native Launchpad panel applet and registered it for COSMIC's panel/dock
  applet picker.
- Made the first click in the application-board area, including directly on an app,
  close Settings without launching or dragging that app.
- Removed a redundant D-Bus activation service that could create duplicate invisible
  launcher processes and make the Super shortcut appear unresponsive.
- Expanded the automatic grid to use the available horizontal space without removing
  an unnecessary full safety column.
- Reduced application-label text size, widened the label area, and fixed tile clipping
  so two-line names are actually visible and similarly named COSMIC apps remain
  distinguishable.
- Added a native hover tooltip that reveals an application's full name.
- Added an On/Off setting for application-name hover tooltips.
- Renamed the pinned section to **Launchpad**.
- Added **Add to Launchpad** and **Remove from Launchpad** to app right-click menus.
- Added instant, case-insensitive search by app name and category.
- Added a live search-result count and clear messages when nothing matches.
- Made search results compact instead of showing empty placement slots.
- Added Enter to launch the first search result and Escape to clear a search.

## 2026-09-15

- Created the COSMIC Application Board fork with a native COSMIC appearance.
- Added persistent freeform tile placement and drag-and-drop.
- Stabilized tile moving and swapping so unrelated apps no longer rearrange.
- Added automatic and manual responsive grid sizing.
- Added settings for board size, tile size, spacing, and labels.
- Added a visible Settings gear button.
- Fixed the clipped column on the right side of the grid.
- Replaced the old app categories with separate pinned and Other Apps sections.
- Hid pinned apps from Other Apps and centered incomplete rows.
- Added local desktop integration, D-Bus activation, and a Super-key shortcut.

New work should be added to the top dated section as short bullet points.
