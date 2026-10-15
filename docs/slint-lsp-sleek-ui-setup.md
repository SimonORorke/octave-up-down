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
> symlink (created with `ln -s`) and **not** a `.lnk` shortcut. Both are invisible to (or not
> followed by) native Windows applications like the Slint LSP. Node.js's
> `fs.symlinkSync(target, link, 'junction')` creates a true NTFS junction.

A junction is needed in every subdirectory of `ui/` that contains `.slint` files importing
`@sleek-ui`:

| Junction location | Files it serves |
|---|---|
| `ui/@sleek-ui` | `main_window.slint` |
| `ui/components/@sleek-ui` | `components/text.slint` |

Each junction points directly to `ui/sleek-ui` inside the sibling `sleek-ui` repository
(`../sleek-ui/ui/sleek-ui`, relative to this project's root) — the same directory `build.rs` maps
to the `"sleek-ui"` library path. A Windows NTFS junction stores an **absolute** path to its
target (unlike a relative symlink), so the same target works no matter how deep under `ui/` the
junction is created.

Unlike `vivi_ui` (used by `pitchgrid_continuum`), `sleek-ui` is **not** a published crate: it is a
local sibling folder on disk, referenced directly by relative path. So there is no version to read
from `Cargo.lock` and no Cargo registry cache to search — the junction target is simply the fixed
relative path to the sibling repository.

## Creating the Junction

A Node.js script, `make_junction.js` in the project root, creates the junction(s) automatically.
Run it once after cloning the repository (with `sleek-ui` checked out as a sibling directory):

```
node make_junction.js
```

This always (re)creates `ui/@sleek-ui`. To also create a junction in a subdirectory of `ui/`
(such as `ui/components`), pass it as an extra argument, relative to `ui/`; multiple
subdirectories can be passed at once:

```
node make_junction.js components
node make_junction.js components some/other/dir
```

Re-run the script (with the corresponding argument) whenever a new subdirectory under `ui/` starts
importing from `@sleek-ui`.

Node.js is available from the JetBrains runtime bundled with RustRover, or can be installed
separately from [nodejs.org](https://nodejs.org).

If the script can't find the `sleek-ui` target directory it prints a clear message; the usual
cause is that the `sleek-ui` repository has not been checked out as a sibling of this project (or
has been checked out under a different name).

After running the script, **restart** RustRover (or reload the Slint LSP) so the plugin picks up
the new directory structure.

## What the Script Does

The script does one thing, for one or more locations; see `make_junction.js` for the full source.
In outline:

```javascript
// 1. The target is the fixed relative path to the sibling repository — no version lookup,
//    no registry glob, since sleek-ui is not a Cargo dependency.
const sleekUiTarget = path.join(__dirname, '..', 'sleek-ui', 'ui', 'sleek-ui');

// 2. `ui/` plus any subdirectories passed as command-line arguments.
const subdirs = ['.', ...process.argv.slice(2)];

// 3. (Re)create a junction in each one, creating the subdirectory first if needed and
//    robustly removing any existing junction first.
for (const subdir of subdirs) {
    const link = path.join(__dirname, 'ui', subdir, '@sleek-ui');
    fs.mkdirSync(path.dirname(link), { recursive: true });  // ui/<subdir> might not exist yet
    try {
        fs.lstatSync(link);                              // lstat: detects a *dangling* junction too
        try { fs.chmodSync(link, 0o666); } catch (e) {}  // clear READONLY so removal can't be denied
        fs.rmSync(link, { recursive: true, force: true });
    } catch (e) { if (e.code !== 'ENOENT') throw e; }
    fs.symlinkSync(sleekUiTarget, link, 'junction');
}
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

The junction directories are listed in `.gitignore` so they are never committed:

```
/ui/@sleek-ui
/ui/**/@sleek-ui
```

The script `make_junction.js` **is** committed to the repository so collaborators can run it.

## Ui Subdirectories in Project

In each subdirectory of the project's `ui` directory that contains Slint files importing from
`@sleek-ui`, run `make_junction.js` with that subdirectory's path (relative to `ui/`) as an
argument, for example:

```
node make_junction.js components
```

This creates a real NTFS junction directly inside `ui/components`, which the Slint LSP follows
just like the one at `ui/@sleek-ui`. A `.lnk` shortcut does **not** work here: the Slint LSP does
not resolve Windows shell shortcuts, only directory junctions/reparse points, so a shortcut named
`@sleek-ui.lnk` is silently ignored and imports keep failing. Both locations are already covered
by the `/ui/**/@sleek-ui` entry in `.gitignore`, so nothing further needs to be added there.

## Pitfall: Copying This Script to Another Project Without `.gitignore`

**This actually happened and deleted real files in the shared `sleek-ui` repository — read this
before copying `make_junction.js` anywhere else.**

A junction makes `ui/.../@sleek-ui` transparently resolve to the *real* files of the sibling
`sleek-ui` repository. If the `.gitignore` of the project you run the script in does **not**
exclude that junction path, git (and IDE actions like "discard"/"revert" on untracked files) will
see the real `sleek-ui` files reached through the junction as new, untracked content of *that*
project. Discarding/reverting those "new" files then **deletes the real files** on disk through
the junction — and since `sleek-ui` is a single shared sibling repository, every other project
that also junctions into it (e.g. this one) loses those files too.

This is exactly what happened when `make_junction.js` and this doc were copied into another
project (`octave-up-down`) without also copying the matching `.gitignore` entries: running
`make_junction.js` there made git report dozens of new uncommitted files (the real `sleek-ui`
library content, seen through the un-ignored junction); discarding them in that project deleted
the real files from the shared `sleek-ui` repository, breaking the build here as well.

To prevent this from happening again, `make_junction.js` now **refuses to create any junction**
unless the running project's `.gitignore` already contains both:

```
/ui/@sleek-ui
/ui/**/@sleek-ui
```

If you copy this script to another project, **always copy/merge these two `.gitignore` lines
first**. If you forget, the script stops with an explicit error instead of silently creating an
un-ignored junction. As an extra habit, run `git status` immediately after running
`make_junction.js` in any project — if it reports unexpected new files, stop and fix `.gitignore`
before touching (and especially before discarding) anything.

## Pitfall: Subdirectory Argument for a Directory That Doesn't Exist Yet

Passing a subdirectory argument (e.g. `node make_junction.js common`) used to fail with a
confusing error if `ui/common` did not already exist as a plain directory:

```
Error for ...\ui\common\@sleek-ui : ENOENT: no such file or directory, symlink '...\sleek-ui\ui\sleek-ui' -> '...\ui\common\@sleek-ui'
```

This looks like a dangling-junction problem, but it isn't: `fs.symlinkSync` requires the link's
*parent* directory to already exist, and nothing in the script created it. `make_junction.js` now
creates `ui/<subdir>` (via `fs.mkdirSync(..., { recursive: true })`) before placing the junction
inside it, so the subdirectory no longer needs to pre-exist.

## Pitfall: Commenting Out a Required `.gitignore` Line Doesn't Disable the Check

The `.gitignore` safety check used to look for each required line as a plain substring of the
whole file:

```javascript
const missingGitignoreLines = requiredGitignoreLines.filter((line) => !gitignoreContent.includes(line));
```

That means a *commented-out* line, such as `# /ui/**/@sleek-ui`, still satisfied the check: the
required text `/ui/**/@sleek-ui` is a substring of the commented line too, so `includes()` returned
`true` and the script happily created the junction anyway — exactly the un-ignored situation the
check exists to prevent.

`make_junction.js` now splits `.gitignore` into individual lines, trims each one, and requires an
**exact** line match:

```javascript
const gitignoreLines = gitignoreContent.split(/\r?\n/).map((l) => l.trim());
const missingGitignoreLines = requiredGitignoreLines.filter((line) => !gitignoreLines.includes(line));
```

A commented-out or otherwise altered line is correctly reported as missing. If you need to
temporarily disable the check to reproduce a problem, delete or rename the line entirely rather
than commenting it out — either way, the script will now refuse to create junctions, as expected.

## Slint Live-Preview: Additional Requirement

Slint Live-Preview (in the RustRover Slint plugin) only has access to files that are currently
**open in editor tabs**. To preview `MainWindow` in `main_window.slint`, all files it imports must
be open simultaneously:

- `../sleek-ui/ui/sleek-ui/widgets.slint`
- `../sleek-ui/ui/sleek-ui/app-theme.slint`

Open them all in tabs, then trigger a refresh in the Live-Preview panel.
