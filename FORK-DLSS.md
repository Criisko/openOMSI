# DLSS, kept here on purpose

The DLSS Super Resolution / DLAA change lives in the project's pull request
[#942](https://github.com/openOMSI-Project/openOMSI/pull/942), which is not merged upstream.
This fork carries it, so that is the one thing a merge here must never quietly lose. Two
copies of it are kept in *this* repository, so that closing or deleting the pull request on
GitHub cannot take it away:

- `FORK-DLSS.patch` - the whole change as one patch, ready to apply;
- the branch `dlss-upstream` and the tag `dlss-pr942-2026-10-03`, both on the same commit in
  this fork's own GitHub repository.

## What the patch is

The author's commit `6eeaff7` ("DLSS Super Resolution and DLAA through NVIDIA Streamline",
Nussaccino, taken on 2026-10-03), which sits on the project's `1ef9649`: fifteen files,
2233 insertions. `crates/omsi-render/src/dlss.rs` and `dlss_motion.wgsl` are the runtime,
the rest is its loading, the motion vectors of the depth prepass, the setting and the
launcher's rows. Nothing of NVIDIA's is in it - the player supplies the DLLs, as the user
guide says.

`git apply --check` of the patch against a worktree of `1ef9649` succeeds: it applies
exactly to the commit it was made on.

## Bringing DLSS onto a newer project version

```powershell
git fetch https://github.com/Criisko/openOMSI.git dlss-upstream
git merge dlss-upstream
```

That is a normal merge of a feature branch on top of the project's `main`, so a conflict
can only be where the project changed the same lines - the one place worth reading. From
this clone, `git am FORK-DLSS.patch` on a tree of the project's `main` does the same.

## Which revision this fork's `main` carries

An earlier revision of the same work, the one the game here has been running: its motion
shaders are still inside `shader.wgsl` instead of `dlss_motion.wgsl`, and its language
strings are the earlier set. That revision is in this fork's own history - it arrived with
the first commit of the fork's history, which is the author's branch as it stood then - and
is tagged `dlss-ours-2026-10-03`. Its `dlss.rs`, `upscale.wgsl`, `shader.wgsl` and
`crates/omsi-render/Cargo.toml` are byte-identical to that earlier commit.

## The rest of the arrangement

`.github/workflows/sync-upstream.yml` merges the project's `main` into this fork's `main`
every night; a merge that conflicts fails that job and leaves the fork exactly where it was.
Before `.github/workflows/windows.yml` publishes anything, it checks that what this fork adds
is still whole - the DLSS file, the keys that carry it where they are read, the view toggle,
the look smoothing, and the project's own two contracts (the settings round trip, and every
launcher row named in `by_tab()`). A failure there publishes nothing, so a merge that silently
dropped part of a change is a red run and no update, rather than a release without the feature.
