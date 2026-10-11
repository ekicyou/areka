---
name: release
description: 'Run one release of areka: version bump (+0.0.1 or the given version) → highlights file → local checks → version-bump PR → squash merge (approval A) → tag v{version} push (approval B) → watch the release and crates.io runs → record. This skill is only the entry: the procedure is the recurring spec areka-P0-release-cycle (tasks.md and design.md), executed in the main context. Use when: /release, /release 0.0.3, リリースを打つ, 途中で止まったリリースを続ける. DO NOT USE FOR: changing the release CI or the tools (a normal spec does that), writing only the highlights file (doc/release-notes/README.md), completing an ordinary spec (kiro-complete), the first manual winget submission.'
disable-model-invocation: true
argument-hint: '[X.Y.Z] (omit to add 1 to the third number of the current version)'
---

# release — one release of areka

<instructions>

## Core Task
Perform exactly one release by executing the recurring spec `areka-P0-release-cycle` from top to bottom. `/release` is the same thing as `/kiro-impl areka-P0-release-cycle` without task numbers; an argument such as `0.0.3` or `v0.0.3` is the developer's version instruction for the task that decides the next version.

This file holds no step, no command and no judgment table. The source of truth is:

- `.kiro/specs/areka-P0-release-cycle/tasks.md` — the tasks, their commands and each task's 「判定」. Its head section 「走らせ方（この spec だけ）」 is binding.
- `.kiro/specs/areka-P0-release-cycle/design.md` — the tables the tasks point at (「`/kiro-impl` の読み替え」, the stages under 「Components and Interfaces」, 「再開のときの事実の読み方」, 「止まり方の決まり」).

Do not copy steps from those files into this skill. When the procedure changes, change the spec; this file changes only when the entry itself changes.

## Steps

1. Read `tasks.md` in full, then in `design.md` the sections 「`/kiro-impl` の読み替え」, the preface of 「Components and Interfaces」, 「再開のときの事実の読み方」 and 「止まり方の決まり」. Check that `spec.json` has `approvals.tasks.approved: true`; if not, stop and report.
2. Do not load the `kiro-impl` skill. Its autonomous loop (a subagent per task, a commit per task, Implementation Notes, `/kiro-validate-impl`) is replaced by the 「読み替え」 table: zero subagents, the main context runs every task itself.
3. Run the tasks in order. Before each task, re-read that task in `tasks.md` and the design section it names; take commands from there with only the placeholders (`{版}`, `{旧版}`, `{remote}`, `{枝}`, `{sha}`) replaced. Do not write commands from memory and do not add to them.
4. Stop and wait for the developer only where the spec says so: approval A (squash merge), approval B (tag push), and the stops the tasks define (a failed 「判定」, a red run, newly merged subjects after approval A). An unclear answer is not an approval — ask again and do nothing.
5. If the session was interrupted, the developer types `/release` again in the same worktree; follow 「再開のときの事実の読み方」 before anything else so that the merge and the tag push never happen twice.
6. End with a report in Japanese: version, Release URL, the two runs, what was recorded, anything noticed on the way, and the handover sentence the last task produces for the record commit.

## Rules

- One command per tool call for anything that leaves the machine: `git push`, `gh pr create`, `gh pr merge`, `gh run rerun`, `gh release ...`. Do not join them with `&&` or `;`, do not prefix `cd`, do not append `echo`. A joined line no longer matches the developer's allow rules (`Bash(gh pr merge:*)` and the like) and the permission check refuses it.
- If the permission check refuses a command, do not rephrase it and do not route around it. Stop, show the command and the reason, and let the developer decide (add an allow rule, or run it themselves).
- Never use `/kiro-validate-impl` or `/kiro-complete` for a release, never open a record-only PR, never commit `tasks.md` checkmarks.
- Never move or delete a pushed tag, never publish the same version twice, never use `git push --tags`, `--follow-tags` or `--force`.
- Do not edit `tools/`, `.github/workflows/` or any crate while releasing. A problem found on the way goes into the final report; an out-of-scope one is ticketed with `/kiro-discovery` as the tasks describe.
- A state the tasks and the design do not cover: do not guess. Stop and report what was observed.

</instructions>
