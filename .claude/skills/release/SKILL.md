---
name: release
description: 'Run one release of areka without stopping for approval: version bump (+0.0.1 or the given version) → highlights file → local checks → version-bump PR → squash merge → tag v{version} push → watch the release and crates.io runs → final report with the full highlights text. The procedure lives in this skill (procedure.md). Stops only on a failed check, a red run, a state the procedure does not cover, or a refused command. Use when: /release, /release 0.0.3, リリースを打つ, 途中で止まったリリースを続ける, リリースの見どころの直し. DO NOT USE FOR: changing the release CI or the tools (a normal spec does that), writing only the highlights file (doc/release-notes/README.md), completing an ordinary spec (kiro-complete), the first manual winget submission.'
disable-model-invocation: true
argument-hint: '[X.Y.Z] (omit to add 1 to the third number of the current version)'
---

# release — one release of areka

<instructions>

## Core Task
Perform exactly one release by executing `procedure.md` (next to this file) from top to bottom in the main context. An argument such as `0.0.3` or `v0.0.3` is the developer's version instruction for the step that decides the next version.

`procedure.md` is the source of truth for every stage, command, judgment and table. This file holds only the promises and the outline. The former recurring spec `areka-P0-release-cycle` is a record under `.kiro/specs/completed/`; do not execute it.

## Outline (details are in procedure.md)

1. 前提を確かめる — work tree, main, open PRs on the 3 root files, current and next version, tags, handover notes in the roadmap.
2. 版を上げる — root `Cargo.toml` (2 lines), `Cargo.lock`, the date line of `dist/README.txt`, and the highlights file `doc/release-notes/v{版}.md`.
3. 手元で確かめる — full tests with `-License` (about 19 minutes), diff range table, startup check (opens a window), pre-publish check, highlights check.
4. 版上げの PR — commit the 5 files, open the PR, take in main if it moved, squash merge. No approval stop.
5. タグ — tag the squash commit and push that one tag. No approval stop.
6. 見守る — the release run and the crates.io run; on red, apply the table and let the developer decide.
7. 報告する — one report in Japanese that includes the full text of the highlights. The developer reads the highlights here and may ask for a fix; then follow 「公開の後に見どころの直しを頼まれたら」.

## Steps

1. Read `procedure.md` in full once. Do not load the `kiro-impl` skill, start no subagent, and write no progress file.
2. If the developer's request is a fix of the highlights of an already published version, run no stage: do only 「公開の後に見どころの直しを頼まれたら」 and stop.
3. Otherwise first follow 「再開のときの事実の読み方」: decide the starting stage from the real state (PR, tag, Release, runs, work tree), so that the merge and the tag push never happen twice. A finished release on this branch is never followed by a new version unless the argument names a newer version.
4. Run the stages in order. Before each stage, re-read that stage in `procedure.md`; take commands from there with only the placeholders (`{版}`, `{旧版}`, `{remote}`, `{枝}`, `{sha}`, `{確かめた main}`, `{番号}`, `{id}`) replaced. Do not write commands from memory and do not add to them.
5. Do not ask for approval on the normal path. Stop only where `procedure.md` says so (「止まり方の決まり」): a failed 「判定」, a red run, a state the procedure does not cover, a command refused by the permission check.
6. End with the stage 7 report.

## Rules

- One command per tool call for anything that leaves the machine: `git push`, `gh pr create`, `gh pr merge`, `gh run rerun`, `gh release edit`. Do not join them with `&&` or `;`, do not prefix `cd`, do not append `echo`. A joined line no longer matches the developer's allow rules (`Bash(gh pr merge:*)` and the like) and the permission check refuses it.
- If the permission check refuses a command, do not rephrase it and do not route around it. Stop, show the command and the reason, and let the developer decide (the allow rule to add is in 「開発者の設定（初めて打つ前に）」 of `procedure.md`).
- Never use `/kiro-validate-impl` or `/kiro-complete` for a release, never open a record-only PR, never edit the roadmap while releasing (what `/kiro-discovery` writes when a red run is ticketed is its own output, not part of the release).
- Never move or delete a pushed tag, never publish the same version twice, never use `git push --tags`, `--follow-tags` or `--force`, never run `gh release create`, `gh release upload` or `cargo publish` by hand.
- Do not edit `tools/`, `.github/workflows/` or any crate while releasing. A problem found on the way goes into the final report; an out-of-scope one is ticketed with `/kiro-discovery` after the developer decides.
- A red run is the developer's call: show the facts and the table row, then wait. Do not re-run by yourself.
- Run the stage 3 full tests (3.1, and its rerun in 4.2) and the startup check (3.3) holding the exclusive-test desk of `areka-impl-watch.exe`, as 「机の段」 of `procedure.md` says (never the merge desk). Waiting for the desk or for a resume is not a stop; when the desk cannot be used, skip it and keep going.
- The local full tests and the startup check are heavy. Run each once per release unless `procedure.md` tells you to run it again.

</instructions>
