# Slint LSP and Live-Preview: Setting Up @sleek-ui Imports

## Background

This project uses [sleek-ui](https://github.com/uAtomicBoolean/sleek-ui), a local Slint component
library (a sibling repository, not a published crate). Slint files import from it using the
`@sleek-ui` library prefix, for example:

```slint
import { UText, UButton } from "@sleek-ui/widgets.slint";
import { UAppTheme } from "@sleek-ui/app-theme.slint";
```

At **compile time** this works automatically: `build.rs` passes a library path mapping
`"sleek-ui" -> <manifest_dir>/../sleek-ui/ui/sleek-ui` to `slint_build::CompilerConfiguration`.

The **Slint LSP** (used by the RustRover IDE plugin and Slint Live-Preview) does not go through the
Rust build system, so it has no knowledge of the `@sleek-ui` library path. Without extra setup it
reports errors such as:

```
Cannot find requested import "@sleek-ui/widgets.slint" in the library search path
```

and Slint Live-Preview shows "Preview is out of date".

## Fix: Windows NTFS Directory Junction

The Slint LSP resolves `@sleek-ui/...` imports by looking for a subdirectory literally named
`@sleek-ui` relative to the directory of the importing `.slint` file. Creating a Windows NTFS
*directory junction* (a transparent filesystem redirect) at that location makes the LSP follow it
to the actual `sleek-ui` library files in the sibling repository.

> **Important:** The junction must be a proper Windows NTFS junction, **not** a Git Bash / MSYS2
> symlink (created with `ln -s`). Git Bash symlinks are invisible to native Windows applications
> like the Slint LSP. Node.js's `fs.symlinkSync(target, link, 'junction')` creates a true NTFS
> junction.

Only one junction is needed, because only `../ui/main_window.slint` imports `@sleek-ui` in this
project:

| Junction location | Files it serves |
|---|---|
| `ui/@sleek-ui` | `main_window.slint` |

The junction points directly to `ui/sleek-ui` inside the sibling `sleek-ui` repository
(`../sleek-ui/ui/sleek-ui`, relative to this project's root) — the same directory `build.rs` maps
to the `"sleek-ui"` library path.

Unlike `vivi_ui` (used by `pitchgrid_continuum`), `sleek-ui` is **not** a published crate: it is a
local sibling folder on disk, referenced directly by relative path. So there is no version to read
from `Cargo.lock` and no Cargo registry cache to search — the junction target is simply the fixed
relative path to the sibling repository.

## Creating the Junction

A Node.js script, `make_junction.js` in the project root, creates the junction automatically.
Run it once after cloning the repository (with `sleek-ui` checked out as a sibling directory):

```
node make_junction.js
```

Node.js is available from the JetBrains runtime bundled with RustRover, or can be installed
separately from [nodejs.org](https://nodejs.org).

If the script can't find the `sleek-ui` target directory it prints a clear message; the usual
cause is that the `sleek-ui` repository has not been checked out as a sibling of this project (or
has been checked out under a different name).

After running the script, **restart** RustRover (or reload the Slint LSP) so the plugin picks up
the new directory structure.

## What the Script Does

The script does one thing; see `make_junction.js` for the full source. In outline:

```javascript
// 1. The target is the fixed relative path to the sibling repository — no version lookup,
//    no registry glob, since sleek-ui is not a Cargo dependency.
const sleekUiTarget = path.join(__dirname, '..', 'sleek-ui', 'ui', 'sleek-ui');

// 2. (Re)create the junction, robustly removing any existing one first.
const link = path.join(__dirname, 'ui', '@sleek-ui');
try {
    fs.lstatSync(link);                              // lstat: detects a *dangling* junction too
    try { fs.chmodSync(link, 0o666); } catch (e) {}  // clear READONLY so removal can't be denied
    fs.rmSync(link, { recursive: true, force: true });
} catch (e) { if (e.code !== 'ENOENT') throw e; }
fs.symlinkSync(sleekUiTarget, link, 'junction');
```

> The script uses `__dirname` to locate the project root, so it works regardless of where the
> repository is checked out (as long as `sleek-ui` remains a sibling directory).

> **Two non-obvious failure modes the removal logic guards against** (carried over from the
> `pitchgrid_continuum` script, which hit both in practice after the machine's C: drive / Windows
> profile was replaced):
>
> 1. **Dangling junction.** If a junction's target no longer exists (e.g. the sibling repository
>    was moved or renamed), `fs.existsSync(link)` *follows* the junction and returns `false`, so
>    the stale junction is never removed and `symlinkSync` then throws `EEXIST`. Re-running the
>    script appears to do nothing. Use `fs.lstatSync` (does not follow) instead.
> 2. **READONLY junction.** A junction carrying the READONLY attribute cannot be removed — `rmdir`
>    / `fs.rmSync` fail with "Access is denied", which mimics a process lock (killing the LSP /
>    file watcher does not help). Clear the readonly bit (`fs.chmodSync(link, 0o666)`) before
>    removing. Note: ordinary directories ignore the readonly bit for deletion; only reparse points
>    are blocked by it.

## Git

The junction directory is listed in `.gitignore` so it is never committed:

```
/ui/@sleek-ui
```

The script `make_junction.js` **is** committed to the repository so collaborators can run it.

## Slint Live-Preview: Additional Requirement

Slint Live-Preview (in the RustRover Slint plugin) only has access to files that are currently
**open in editor tabs**. To preview `MainWindow` in `main_window.slint`, all files it imports must
be open simultaneously:

- `../sleek-ui/ui/sleek-ui/widgets.slint`
- `../sleek-ui/ui/sleek-ui/app-theme.slint`

Open them all in tabs, then trigger a refresh in the Live-Preview panel.
