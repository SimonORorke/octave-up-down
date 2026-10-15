// Creates the Windows NTFS directory junction(s) that let the Slint LSP / Live-Preview
// resolve `@sleek-ui/...` imports. For the full rationale, see docs\slint-lsp-sleek-ui-setup.md.
//
// Unlike vivi_ui (a published crate resolved via the Cargo registry), sleek-ui is a local
// sibling folder (`../sleek-ui`) referenced directly in build.rs. So there is no version to
// read from Cargo.lock and no registry to glob: the junction target is simply the fixed
// relative path to the sibling repository.
const fs = require('fs');
const path = require('path');

// The target is the sleek-ui library directory inside the sibling `sleek-ui` repository,
// always resolved relative to this script's location so it works regardless of where the
// repository is checked out.
const sleekUiTarget = path.join(__dirname, '..', 'sleek-ui', 'ui', 'sleek-ui');
if (!fs.existsSync(sleekUiTarget)) {
    console.error(`sleek-ui target not found at ${sleekUiTarget}.`);
    console.error('Make sure the sleek-ui repository is checked out as a sibling of this project.');
    process.exit(1);
}
console.log(`sleek-ui -> ${sleekUiTarget}`);

// SAFETY CHECK: a junction makes `ui/.../@sleek-ui` transparently resolve to the *real* files
// of the sibling sleek-ui repository. If this project's `.gitignore` does not exclude it, git
// (and IDE actions like "discard"/"revert" on untracked files) will see those real files as new,
// untracked content of THIS repo. Discarding them then deletes the real files through the
// junction - which can silently wipe out the shared sleek-ui repository for every project that
// points at it. This has actually happened (see docs/slint-lsp-sleek-ui-setup.md), typically
// after copying this script into another repo without also copying the matching .gitignore
// entries. So we refuse to create any junction until both entries are present.
const gitignorePath = path.join(__dirname, '.gitignore');
const requiredGitignoreLines = ['/ui/@sleek-ui', '/ui/**/@sleek-ui'];
const gitignoreContent = fs.existsSync(gitignorePath) ? fs.readFileSync(gitignorePath, 'utf8') : '';
// Compare against actual (trimmed) lines, not a substring of the whole file. A naive
// `gitignoreContent.includes(line)` check would also match a *commented-out* line such as
// `# /ui/@sleek-ui`, since the required text is still a substring of that line — silently
// defeating the safety check it's meant to enforce.
const gitignoreLines = gitignoreContent.split(/\r?\n/).map((l) => l.trim());
const missingGitignoreLines = requiredGitignoreLines.filter((line) => !gitignoreLines.includes(line));
if (missingGitignoreLines.length > 0) {
    console.error('Refusing to create junction(s): .gitignore is missing required line(s):');
    for (const line of missingGitignoreLines) console.error('  ' + line);
    console.error(
        'Without these, git will treat the real sleek-ui files reached through the junction as ' +
        "untracked files of THIS repo, and discarding/reverting them will delete the real files " +
        'in the shared sibling sleek-ui repository. Add the missing line(s) to .gitignore, then re-run.'
    );
    process.exit(1);
}

const projectUi = path.join(__dirname, 'ui');  // always relative to the script's location

// A junction is always (re)created directly in `ui/`, for `main_window.slint`. Any other
// subdirectory of `ui/` whose Slint files also import `@sleek-ui` (e.g. `ui/components`) must
// be passed as an extra command-line argument, relative to `ui/`:
//
//   node make_junction.js components
//   node make_junction.js components some/nested/dir
//
// A Windows NTFS junction stores an *absolute* path to its target (unlike a relative symlink),
// so a junction created several levels deep under `ui/` resolves to exactly the same target as
// the one at `ui/@sleek-ui` — no relative-path adjustment is needed for the subdirectory depth.
const subdirs = ['.', ...process.argv.slice(2)];

let hadError = false;
for (const subdir of subdirs) {
    const link = path.join(projectUi, subdir, '@sleek-ui');

    try {
        // The subdirectory itself (e.g. `ui/common`) might not exist yet — fs.symlinkSync
        // fails with ENOENT ("no such file or directory, symlink ... -> ...") if its parent
        // directory is missing, which looks like a dangling-junction problem but isn't one.
        // Create the parent directory first so the junction can always be placed inside it.
        fs.mkdirSync(path.dirname(link), { recursive: true });

        // lstatSync (not existsSync) so a *dangling* junction — one whose target no longer
        // exists, e.g. after the sibling repository moves — is still detected and removed.
        // existsSync follows the link and reports false for a dangling junction, which leaves
        // the stale entry in place and makes symlinkSync throw EEXIST.
        try {
            fs.lstatSync(link);
            // A READONLY junction cannot be removed (rmdir/rmSync fail with "Access is denied"),
            // which mimics a process lock. Clear the readonly bit on the junction itself first.
            try { fs.chmodSync(link, 0o666); } catch (e) { /* best effort */ }
            fs.rmSync(link, { recursive: true, force: true });
            console.log('Removed existing:', link);
        } catch (e) {
            if (e.code !== 'ENOENT') throw e;
        }
        fs.symlinkSync(sleekUiTarget, link, 'junction');
        console.log('Junction created:', link);
    } catch (e) {
        console.error('Error for', link, ':', e.message);
        hadError = true;
    }
}

if (hadError) process.exit(1);
