// Creates the Windows NTFS directory junction that lets the Slint LSP / Live-Preview
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

// Create the junction.
const projectUi = path.join(__dirname, 'ui');  // always relative to the script's location
const link = path.join(projectUi, '@sleek-ui');

try {
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
    process.exit(1);
}
