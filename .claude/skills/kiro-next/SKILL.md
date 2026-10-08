---
name: kiro-next
description: 'Roadmap inventory ("棚卸") and next-wave planning for the areka Kiro workflow, run after main has advanced. Syncs the harness worktree branch with remote main, identifies in-flight specs and reserves the files they touch, re-measures every unfinished brief against current main with parallel subagents (each brief gets a "棚卸NNの再測定" section), applies trivial doc/comment/path fixes immediately, splits oversized specs, promotes brief-less memo items into briefs, builds a strict dependency tree, ranks its head specs (descendant count → priority class A/B/C/D → filing age), picks up to 8 mutually non-overlapping specs as the next wave, rewrites roadmap.md / roadmap-history.md / focus.md, commits on the worktree branch WITHOUT pushing, and reports the wave as a Fable/Opus /kiro-start code block. PR creation and squash merge happen only behind an explicit developer approval gate. Use when: /kiro-next, 棚卸, mainが進んだので棚卸, 次のウェーブ, next wave. DO NOT USE FOR: starting a spec (use /kiro-start), filing one new idea (use /kiro-discovery), completing a spec (use /kiro-complete).'
disable-model-invocation: true
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, Agent, SendMessage, WebFetch, ToolSearch, AskUserQuestion
argument-hint: '[extra instructions]'
---

# kiro-next — roadmap inventory and next wave

<instructions>

## Core Task
Run one full roadmap inventory ("棚卸") on the current harness worktree branch and propose the next parallel wave. The developer's standing instruction is fixed and is the default behaviour of this skill (no arguments needed):

1. main has advanced → inventory the roadmap, deep-dive the specs, brief thoroughly.
2. Apply roadmap-listed trivial fixes (no brief needed) immediately and close them.
3. Of the items that exist only in the roadmap (no brief), launch briefs for those that warrant one; adjust granularity; split specs whose load is too high.
4. Build a strict dependency tree, rank its head specs, and pick at most 8 as the next wave.
5. Report the wave in the fixed code-block format, split into Fable and Opus.

`$ARGUMENTS` (optional) carries extra developer instructions for this run; they override the defaults where they conflict.

## Communication Language
- **Think in English.** All internal reasoning, planning and subagent prompts are English.
- **Every developer-facing message uses the session's console language and persona** (the language/persona configured for the session; fall back to Japanese). This includes progress notes, questions, the final report and the approval gate.
- Text written into project files (briefs, roadmap, history, steering) uses the spec language (`spec.json.language`; this repository: Japanese) in plain everyday words — no project jargon, no abbreviations such as "STI/DMC", cite code by "what it is" plus path.
- Refer to specs by name, never by ledger numbers. `#数字` only as `PR#123`.

## Hard Rules
- **Never on the default branch.** If the current branch is the repository default branch, STOP and ask the developer to re-run inside a harness worktree.
- **No push, no PR, no merge without the approval gate (Step 11).** Commits stay local on the worktree branch.
- **Never touch files reserved by in-flight specs** except the allowed overlaps (Step 6).
- **Never print the remote URL** (it may carry a token). Mask with `sed -E 's#https://[^@]*@#https://***@#'`.
- **Preserve line endings.** Most files are CRLF. git-bash `sed -i` strips CR — use the Edit tool, `perl -pi`, or Python with `newline=''`. Verify with `file` and by counting `b'\r\r'` / lone `\n`.
- **Respect quiet-desk holds.** If another session asks to stop heavy work (e.g. a load-measurement spec), reply "停止しました" (in the persona language) to its `from` address with `SendMessage`, keep doing only Markdown reading/writing, and defer every `cargo` run until it says resume.
- Do not use `spawn_task` chips for out-of-scope findings; they become briefs or roadmap memo items here.

## Step 0 — Resolve context
- Repository root = current worktree. Steering: `.kiro/steering/`. Specs: `.kiro/specs/` (completed under `completed/`). Default branch from `git symbolic-ref refs/remotes/origin/HEAD` (fallback `main`).
- Scratch directory = the session scratchpad (never `/tmp`, never `C:\`).
- Load memory relevant to inventories: the merge-coordinator memory (who is in flight), the previous inventory memory, the priority-class memory, the parallel-spec report format.

## Step 1 — Sync with remote main
1. Call `mcp__ccd_host__sync_with_base_branch` (base = default branch).
2. If it refuses (e.g. credentials in the origin URL), do it yourself: `git fetch origin <default>` (masked output) → `git merge --ff-only FETCH_HEAD`. If not fast-forwardable, STOP and report (never force).
3. Verify: `git ls-remote origin refs/heads/<default>` equals `HEAD`. Record the base SHA (`<BASE>`) for every section heading below.

## Step 2 — Lightweight scan
- `roadmap.md` is ~200 KB. Read its structure with `grep -n '^#'`, then read only: the wave table and serial lanes ("ウェーブ編成", "直列の列"), the spec ledger table compactly (`awk -F'|'` printing truncated columns), "覚え書き", "直接修正候補", "生きている決まり", the previous verdict section, and the carry-over theme sections at the end.
- Inventory: `ls .kiro/specs/` (non-completed folders with `brief.md`, folders with `spec.json`), `ls .kiro/specs/completed | wc -l`.
- **In-flight specs**: `git worktree list` and `git branch -r --sort=-committerdate`. For each in-flight spec branch: `git diff --name-only <default>...<branch> | grep -v '^\.kiro/'` → save to `<scratch>/inflight-files.md`. Also read each in-flight spec's brief/design for files it still plans to touch. All of these are **reserved**.
- Determine the next inventory number NN (previous verdict section number + 1).

## Step 3 — Parallel re-measurement (subagents)
1. Write `<scratch>/remeasure-common.md` (template below).
2. Partition every unfinished, not-in-flight spec into ~7 groups of ~10 following the serial lanes (text & balloon split into two, shell element/animation, MCP/kanade/host32, balloon selection/root folders/install/distribution, properties/directives, tools/tests/residue). Every spec in exactly one group — check the count adds up.
3. Launch all groups plus one "fixes & brief-less items" agent **in one message, in the background** (`Agent`, `model: "opus"`). Each group prompt = "read the common file and follow it" + its spec list + 3–5 lines of lane-specific focus (which landed specs changed its premises, which in-flight branches touch its files, near-cap files, suspected splits).
4. As each report arrives, append a compact summary to `<scratch>/results.md` (name | class | size | model | head? | blocking overlaps | split). Do not keep raw reports in context.
5. If agents die (e.g. HTTP 429 weekly limit) and the developer says resume: run `git status` and `grep -l '棚卸NNの再測定' .kiro/specs/*/brief.md` to detect partial writes, then relaunch the same prompts.

**Common instruction template** (fill `<…>`):
```
# Common instructions — roadmap inventory NN re-measurement
Worktree: <path> (run everything here). Base: <default> <BASE> (<date>). Previous re-measurement: inventory NN-1 on <sha>. Specs landed since then: <list> (see .kiro/specs/completed/areka-P0-<name>/).
In-flight specs (NOT candidates; their files are reserved): <list with one-line notes>. Changed files: <scratch>/inflight-files.md. Also read their briefs for planned files.
Read first: roadmap.md wave rules + serial lanes + allowed overlaps; living rules; each assigned brief (and requirements/design/tasks if present).
For EACH spec: (1) verify the brief against current main with Grep/Glob/Read only — no cargo, no heavy commands; note stale claims and what landed specs already did. (2) touched source files on main (mark new files; wc -l files near the 1,000-line cap). (3) size in tasks (cap 20); if >20 propose a concrete split (names, boundary, files per half) — never re-split a spec already carved once just for barely crossing 20. (4) dependencies: functional (unfinished only) and file overlap with other unfinished specs including in-flight ones (any shared source file outside the allowed overlaps blocks the same wave). (5) priority class: A = developer asked for it (brief/theme cites the developer's request/question/instruction as origin), B = bug, C = ukadoc salvage or non-bug carry-over, D = dream / 据え置き / 保留 / 取り下げ — give one-phrase evidence. (6) requirements model: Fable if canon ambiguity, cross-engine architecture, developer-decision forks or timing/concurrency; else Opus. (7) immediate-fix candidates (stale comments, broken paths, doc errors) with file:line — do NOT fix code. (8) APPEND to the end of each brief.md a section "## <date> 棚卸NNの再測定（main <BASE>）" in Japanese plain words, ≤25 lines: 前提の変化 / 触るファイル / 規模 / 先に要るもの / 優先度の区分 / 要件定義のモデル / 分割の案 / 見つけた穴・古くなった記述. Preserve line endings. Edit no other file. Do not commit.
Return (English, compact), one block per spec:
<name> | class (evidence) | size | model
 touches: … | functional deps: … | file-overlap with: spec(file), … | split: … | stale/holes: … | immediate fixes: …
Last line: "briefs written: N/N".
```

**Fixes & brief-less agent** (read-only, may WebFetch one crate version): classify each "覚え書き" item as (a) trivially fixable now with exact file:line and change, (b) needs a brief (S+ or a decision that can be a requirements agenda item), (c) still blocked (developer decision / missing consumer); scan Implementation Notes and research "範囲外" of specs completed since the last inventory for orphaned leftovers; find stale statements in steering (`focus.md`, `product.md`, `structure.md`, `tech.md`), `README.md`, `dist/README.txt`; grep `.kiro/specs/<name>/` paths in `crates/` and `doc/` that moved to `completed/`; check `image-webp` on crates.io (remove the git pin when ≥ 0.2.5, per the roadmap memo); list `.rs` files ≥ 950 lines; count completed entries and brief folders.

## Step 4 — Immediate fixes (controller, right away)
- Apply only class-(a) fixes: doc/comment/path/count/date corrections and wrong pointers in the roadmap. Use the Edit tool.
- Before editing a file an in-flight branch also edits, diff that branch to confirm a different hunk; if a fix lands in a reserved file, hand it to the owning spec's brief instead.
- Never "fix" anything that changes behaviour, security posture (e.g. UIPI), bytes sent to SHIORI, or needs a design decision — those stay in the memo or become briefs.
- Ledger `owner` corrections are not immediate fixes: they move together with `roadmap-draft.md` `owner_count` in each spec's requirements phase — record them in the briefs.

## Step 5 — Granularity: splits and new briefs (subagents)
- **Split** a spec when the re-measurement says >20 tasks and it was never carved out before (also when a human wait, e.g. an external review, would sit inside one PR). Do not split specs already carved once for barely crossing 20; mention a parallelism-only split as a developer decision instead.
- **Promote** memo items classed (b). Keep (c) items in the memo with their reason.
- Launch two background agents (Opus): one performs the splits (append "## <date> 棚卸NNの分割" to the original — In/Out, what moved where, order, size, files — and create the new brief), one writes the promoted briefs. House style: `# Brief: areka-P0-<name>`, Problem / Current State / Desired Outcome / Approach / Scope (In/Out) / Boundary Candidates / Out of Boundary / Upstream / Downstream / Existing Spec Touchpoints / Constraints, first line naming the origin (棚卸NN, split from / memo item), and a closing measurement section (files, size, deps, class, model, agenda). Canon claims must be checked with the ukadoc MCP (`search_docs` takes ONE word). New briefs are CRLF.
- New spec folders are safe for `ukadoc-survey` (it checks `roadmap-draft.md` `[[spec]]` rows exist as folders, not the reverse). Never move or delete brief folders.

## Step 6 — Dependency tree and the next wave
**Edges** (strict): functional dependencies + any shared source file (other unfinished specs and in-flight specs) + the roadmap's serial-lane order. Allowed overlaps that do NOT create edges: appending to `doc/COMPAT_ARCHITECTURE.md` §8; generated files (`THIRD-PARTY-NOTICES.md`, `doc/ukadoc-coverage/report/`); steering; different rows of `doc/ukadoc-coverage/ledger/*.toml`; different rows of `dist/README.txt`. A conditional overlap ("only if …") counts as an edge unless the wave row states a promise that removes it.
**Single-seat resources** (one spec per wave): `Cargo.lock` / `THIRD-PARTY-NOTICES.md` / `tech.md` dependency additions (a pending recurring `release-cycle` bump holds this seat); the emo-text `lib.rs` pure-file list (only one spec may add an emo-text source file per wave).
**Excluded from the tree**: 据え置き, 保留, 取り下げ予定 specs (neither candidates nor counted as descendants). **Not parallelised**: specs whose core work is measurement (latency/perf/load) — they pollute and are polluted by other work.
**Procedure**:
1. Heads = specs with no unfinished predecessor (in-flight specs count as unfinished predecessors).
2. Descendant count = number of unfinished specs transitively depending on the head (approximate is fine; state "約").
3. Sort heads by descendant count desc → class A > B > C > D → older filing date (higher in the roadmap).
4. Greedily take heads from the top whose touched files do not overlap any already-taken head; stop at 8.
5. For every chosen spec write its touched files and the **same-wave promises** (files it must not touch). List the heads not chosen with one-line reasons, and outline the following wave's candidates.
6. Model per chosen spec from the re-measurement (Fable / Opus).

## Step 7 — Write the roadmap (one script)
Do all roadmap edits in one Python script in the scratch directory (read with `newline=''`, split on `\r\n`, assert every anchor/replace target exists):
- Append to `roadmap-history.md` a section "## <date> 棚卸NN退避（`roadmap.md` から逐語で移した・main `<BASE>`）" holding verbatim: the old wave table and its sub-notes, the old serial lanes, memo bullets promoted to briefs, the old "直接修正候補", the previous verdict section.
- In `roadmap.md`: new "ウェーブ編成（着手順の正本・<date> 棚卸NN）" (rules, remaining in-flight row, the new wave row with files and promises, next-wave candidates, 保留); serial lanes with targeted edits (splits and new specs inserted, inventory note); spec ledger heading with the new real count and its history, split notes on the original rows, new rows before the first 据え置き row; memo minus promoted items; new "直接修正候補" (what was fixed, what was handed to specs); living rule 1 if the priority rules changed; new "棚卸NNの裁定" section (what was re-measured, fixes, splits and why, promotions and what stayed, the wave and why, Fable/Opus split, broken same-wave promises, holes found, decisions needed); a theme section "棚卸NNの分割と起票" (Existing Spec Updates + Specs (dependency order)) before "## 予約"; `updated_at` and the header note.
- Then `focus.md` (counts line: completed entries, spec.json-active, brief count, priority rule), and stale bits of `product.md` / `structure.md`. Grep for old wording of changed rules and for references to moved sections.
- Verify: `b'\r\r'` count 0, no lone `\n` in CRLF files, `git diff --numstat` proportional to the edit (a whole-file diff means broken line endings).

## Step 8 — Verify and commit
- Run `cargo test -p ukadoc-survey` (skip and say so while a quiet-desk hold is active; run it when resumed).
- Commit on the worktree branch, no push: `docs(roadmap): 棚卸NN＝<what landed>の後の再測定・<n> 本を分割・<m> 本を起票・ウェーブ <Cn> を <k> 本で組む`, with a short bullet body and the required attribution trailer.

## Step 9 — Memory
- Update the priority-class memory if the rules changed; write the inventory memory (base SHA, counts, wave, splits, new briefs, pending items, branch/commit) and add it to the "進行中" part of `MEMORY.md`.

## Step 10 — Report (persona language, conclusions only)
1. Immediate fixes done (and what was handed to specs).
2. Briefing: re-measured count, splits, new briefs, resulting brief count.
3. Dependency-tree summary: largest heads and why they were or were not taken.
4. The wave, exactly in this form (one code block, Fable first; no per-spec notes inside):
```
Wave <Cn> <k>本
Fable
/kiro-start areka-P0-<name>

Opus
/kiro-start areka-P0-<name>
```
5. Decisions needed from the developer (none of them block the wave) and any manual steps.

## Step 11 — Approval gate (STOP here)
- After the report, STOP. Do not push, open a PR or merge.
- Only when the developer explicitly instructs it (e.g. 「PRしてスクワッシュマージ」) push the branch, open the PR (body ends with the required attribution line), then show the PR number, title and changed-file summary and ask for confirmation once more before `gh pr merge --squash`. After merging, report the main commit.
- If the run was a dry run or the developer says not to merge the inventory, move the commit to a local backup branch (`git branch <name> <sha>`) instead of leaving it on the branch that will be merged.

</instructions>
