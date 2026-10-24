---
name: kiro-next
description: 'Roadmap inventory ("棚卸") and next-wave planning for the areka Kiro workflow, run after main has advanced. Syncs the harness worktree branch with remote main, identifies in-flight specs and reserves the files they touch, re-measures against current main only the current version''s specs and the version-free fillers a spare seat may take (a scan script picks them; a few subagents; a brief gets a "棚卸NNの再測定" section only when something changed, and keeps only the newest one), applies trivial doc/comment/path fixes immediately, splits oversized specs, promotes brief-less memo items into briefs, builds a strict dependency tree, ranks its head specs (descendant count → priority class A/B/C/D → filing age), picks up to 8 mutually non-overlapping specs as the next wave from the current version only (roadmap "版の段"; spare seats only for version-free fillers that cannot delay that version) and outlines the next three waves after it (ending at /release when it comes earlier), rewrites roadmap.md / roadmap-history.md / focus.md, commits on the worktree branch WITHOUT pushing, and reports the wave as a Fable/Opus /kiro-start code block. PR creation and squash merge happen only behind an explicit developer approval gate. Use when: /kiro-next, 棚卸, mainが進んだので棚卸, 次のウェーブ, next wave. DO NOT USE FOR: starting a spec (use /kiro-start), filing one new idea (use /kiro-discovery), completing a spec (use /kiro-complete).'
disable-model-invocation: true
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, Agent, SendMessage, WebFetch, ToolSearch, AskUserQuestion
argument-hint: '[extra instructions]'
---

# kiro-next — roadmap inventory and next wave

<instructions>

## Core Task
Run one full roadmap inventory ("棚卸") on the current harness worktree branch and propose the next parallel wave. The developer's standing instruction is fixed and is the default behaviour of this skill (no arguments needed):

1. main has advanced → inventory the roadmap and re-measure the specs the current version and the diff call for (lightweight scan, ported from pasta's `/kiro-next` 2026-10-24; pasta developer: 「もう少し軽量スキャンでやれないか」「スクリプトの力も借りてトークン消費を抑えられないかな」 — Steps 2–3).
2. Apply roadmap-listed trivial fixes (no brief needed) immediately and close them.
3. Of the items that exist only in the roadmap (no brief), launch briefs for those that warrant one; adjust granularity; split specs whose load is too high.
4. Build a strict dependency tree, rank its head specs, and pick at most 8 as the next wave — **from the current version only** (the youngest version in the roadmap's "版の段" that is not released yet), plus version-free fillers that cannot delay that version (Step 6 version gate). When the current version has no spec left and can be released, the wave is `/release <X.Y.Z>` instead (Step 6 release gate). After the current wave, outline the next three waves of the same version as start commands too (current wave → next 1, 2, 3), ending at `/release <X.Y.Z>` when it comes earlier (ported from pasta 2026-10-24; developer: 「/releaseを最終とする、最大３ウェーブ先まで開始コマンドを作るようにしてほしい」「今のウェーブの開始コマンド→次の３ウェーブの開始コマンド、にしてほしい」).
5. Keep "版の段" to the live rows only: drop released versions and emptied groups (Step 7).
6. Report the wave in the fixed code-block format, split into Fable and Opus (or the `/release` block), followed by the outlined waves in the same block.

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
- **Git Bash has no `$TMPDIR`.** Always use the absolute scratchpad path in POSIX form for temp files.
- **No `cargo` for measurement.** The inventory is Markdown work (Grep/Glob/Read and git only); the one `cargo` run is Step 8's `ukadoc-survey` test.
- **Respect quiet-desk holds.** If another session asks to stop heavy work (e.g. a load-measurement spec), reply "停止しました" (in the persona language) to its `from` address with `SendMessage`, keep doing only Markdown reading/writing, and defer every `cargo` run until it says resume.
- Do not use `spawn_task` chips for out-of-scope findings; they become briefs or roadmap memo items here.

## Step 0 — Resolve context
- Repository root = current worktree. Steering: `.kiro/steering/`. Specs: `.kiro/specs/` (completed under `completed/`). Default branch from `git symbolic-ref refs/remotes/origin/HEAD` (fallback `main`).
- Scratch directory = the session scratchpad (never `/tmp`, never `C:\`).
- **Session title** (developer 2026-10-17: 「セッション名を「{リポジトリ名}棚卸」に」): first thing in Step 0, rename this session to `<repo>棚卸`, where `<repo>` is the main checkout's folder name (`basename "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"`, e.g. `areka` → `areka棚卸`). Call `mcp__ccd_session_mgmt__set_session_title` with `session_id` = `self` (load it with `ToolSearch` `select:mcp__ccd_session_mgmt__set_session_title` if deferred). Skip silently when the title already matches; if the tool is missing or the rename is declined, say so in one line and go on.
- Load memory relevant to inventories: the merge-coordinator memory (who is in flight), the previous inventory memory, the priority-class memory, the parallel-spec report format.
- Clear this repository's desk records once (last item of Step 0, before the sync in Step 1 and before any inventory work). At inventory time nobody works in this repository, so leftover records of dead sessions are removed here. Run this one Bash call in the foreground, exactly once:

```bash
R=$(basename "$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")")
if [ -z "$AREKA_IMPL_WATCH_HOME" ]; then echo "desk: skip no-home"
elif [ ! -f "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" ]; then echo "desk: skip no-exe"
elif ! [[ "$R" =~ ^[A-Za-z0-9._-]{1,100}$ ]]; then echo "desk: skip bad-name $R"
else "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" clear --repo "$R" 2>&1; echo "exit=$?"; fi
```

  - Reading the result: `exit=0` → relay the output's one line as is (`cleared repo=<name>; left: <ids>` or `left: none`). `desk: skip …`, `exit=1` (including a refused environment variable or home folder), or `exit=2` (an exe that does not have `clear --repo` yet) → tell the developer one line and continue the inventory. In no case does this step stop the inventory.
  - Never call the full `clear` (no arguments) and never touch another repository's records. Do not start `watch` (do not become a desk participant) and do not read `status`.
  - If the owner of a cleared record was in fact still running, that session's own desk rules re-establish its watch and apply again; this step adds no rule of its own.

## Step 1 — Sync with remote main
1. Call `mcp__ccd_host__sync_with_base_branch` (base = default branch).
2. If it refuses (e.g. credentials in the origin URL), do it yourself: `git fetch origin <default>` (masked output) → `git merge --ff-only FETCH_HEAD`. If not fast-forwardable, STOP and report (never force).
3. Verify: `git ls-remote origin refs/heads/<default>` equals `HEAD`. Record the base SHA (`<BASE>`) for every section heading below.

## Step 2 — Lightweight scan
- **Start with the scan script.** The skill's scripts live in its own folder, `<skill dir>/scripts/` (`<skill dir>` = the folder of this SKILL.md). Run `python -I <skill dir>/scripts/scan.py --out <scratch>` at the worktree root. Its output gives the previous inventory (number, date, base `<PREV>`) and the current base, the landed specs, branches whose names match an unfinished spec (in-flight candidates), the current version and its unfinished specs, the ledger-vs-folder check, counts, the roadmap headings with line ranges, the specs to re-measure, and the version-free fillers the diff moved, with a reason each; it writes `<scratch>/diff-files.md` (`git diff --name-only <PREV>..<BASE>`) and `<scratch>/facts-<spec>.md` (the files each picked brief's latest measurement names: tracked or not, lines, changed). Use these instead of hand listing; do by hand only what the output flags or cannot know. `--self-test` checks the script.
- `roadmap.md` is ~500 KB (2026-10). Never read it whole: read by the scan's line ranges only the wave table and serial lanes ("ウェーブ編成", "直列の列"), "版の段", "覚え書き", "直接修正候補", "生きている決まり", the previous verdict section, and the ledger rows of the picked specs (`grep -n` by name). Read a theme section only when a picked or landed spec belongs to it, or when its text must change.
- **Fast path** (wave advance): when everything in the previous wave landed, nothing new was filed or decided in the roadmap since `<PREV>` (the diff under `.kiro/steering/` and `.kiro/specs/*/brief.md` comes only from the landed specs and their completion), and the previous inventory outlined the next waves (`### その先のウェーブ`), the inventory only advances: re-measure the specs of the outlined next wave, re-check its 入る条件, shift the outline by one, and skip Step 5 unless the re-measurement asks for a split. Say in the report that the fast path was used.
- **In-flight specs**: start from the scan's "in flight?" line (a name match only); confirm each with `git log --oneline -3 <branch>` and whether its spec folder is still unfinished; a branch already merged is not in flight. For each in-flight spec branch: `git diff --name-only <default>...<branch> | grep -v '^\.kiro/'` → save to `<scratch>/inflight-files.md`. Also read each in-flight spec's brief/design for files it still plans to touch. All of these are **reserved**.
- Determine the next inventory number NN (previous verdict section number + 1, from the scan's `prev 棚卸NN` line).

## Step 3 — Parallel re-measurement (subagents)
1. Write `<scratch>/remeasure-common.md` (template below).
2. **Pick the specs to re-measure** (unfinished, not in flight). Only these; every other spec carries its last measurement forward untouched:
   - every spec of the current version (the scan's "re-measure" list) — together they fill the current wave and the next three waves (the version gate keeps later versions out);
   - of the scan's "fillers the diff moved" list, only the fillers a spare seat may really take: run this second pick after the current-version reports are in and the wave's current-version heads are known, and only when seats stay empty — then take the fillers in the Step 6 sort order, enough for the empty seats plus a few spares. No empty seat → no filler is re-measured.
   - Later-version specs are never re-measured here; they are re-measured when their version becomes the current one.
   Give each group prompt the paths of its specs' `<scratch>/facts-<spec>.md`. Partition the picked specs into groups of ~6 following the serial lanes; ≤ 6 specs → one group. Every picked spec in exactly one group — check the count adds up. List the picked specs and the reason for each in `<scratch>/results.md` before launching.
3. Launch the groups **in one message, in the background** (`Agent`, `model: "opus"`), plus the fixes agent only when `leftovers.py` says so (below). Each group prompt = "read the common file and follow it" + its spec list + 3–5 lines of lane-specific focus (which landed specs changed its premises, which in-flight branches touch its files, near-cap files, suspected splits). Expect two to four agents on a normal run; more than five means the pick was too wide — recheck it.
4. As each report arrives, append a compact summary to `<scratch>/results.md` (name | class | size | model | head? | blocking overlaps | split). Do not keep raw reports in context.
5. If agents die (e.g. HTTP 429 weekly limit) and the developer says resume: run `git status` and `grep -l '棚卸NNの再測定' .kiro/specs/*/brief.md` to detect partial writes, then relaunch the same prompts.

**Common instruction template** (fill `<…>`):
```
# Common instructions — roadmap inventory NN re-measurement
Worktree: <path> (run everything here). Base: <default> <BASE> (<date>). Previous re-measurement: inventory NN-1 on <sha>. Specs landed since then: <list> (see .kiro/specs/completed/areka-P0-<name>/).
In-flight specs (NOT candidates; their files are reserved): <list with one-line notes>. Changed files: <scratch>/inflight-files.md. Also read their briefs for planned files.
Read first: only these line ranges of .kiro/steering/roadmap.md — <line ranges of the wave rules, serial lanes, allowed overlaps and living rules> — not the whole file; each assigned brief (and requirements/design/tasks if present); each spec's <scratch>/facts-<spec>.md. A brief's latest "棚卸NNの再測定" section is the baseline; older ones are history.
Files changed since the previous inventory: <scratch>/diff-files.md.
LIGHTWEIGHT RULE: verify only claims the landed specs or the changed files could have moved; do not re-verify what the baseline already established.
For EACH spec: (1) verify the brief against current main with Grep/Glob/Read and git log/diff only — no cargo, no heavy commands; note stale claims and what landed specs already did. (2) touched source files on main (mark new files; wc -l files near the 1,000-line cap). (3) size in tasks (cap 20); if >20 propose a concrete split (names, boundary, files per half) — never re-split a spec already carved once just for barely crossing 20. (4) dependencies: functional (unfinished only) and file overlap with other unfinished specs including in-flight ones (any shared source file outside the allowed overlaps blocks the same wave). (5) priority class: A = developer asked for it (brief/theme cites the developer's request/question/instruction as origin), B = bug, C = ukadoc salvage or non-bug carry-over, D = dream / 据え置き / 保留 / 取り下げ — give one-phrase evidence. (6) requirements model: Opus by default (developer 2026-10-15: Opus 5.5 is strong; real Fable cases are few). Fable only if the spec concretely hits one of: a new cross-crate boundary or public contract with no precedent (adding to or following an existing one does not count); newly designing concurrency, threading or timing/ordering guarantees (merely riding the existing actors or dola does not count); a skeleton-deciding fork that no precedent settles (e.g. canon readings that change the skeleton). Size, several crates, many open questions, real-machine checks or bug root-causing alone are not Fable. Name the criterion hit and where; when unsure, Opus. (7) immediate-fix candidates (stale comments, broken paths, doc errors) with file:line — do NOT fix code. (8) Only when something changed since the baseline, write at the end of the brief.md a section "## <date> 棚卸NNの再測定（main <BASE>）" in Japanese plain words, ≤15 lines: 前提の変化 / 触るファイル / 規模 / 先に要るもの / 優先度の区分 / 要件定義のモデル / 分割の案 / 見つけた穴・古くなった記述. Keep only the newest re-measurement section: delete the older "棚卸…の再測定" sections of that brief (git history keeps them), but carry into the new section any line of theirs that is still true and still needed (an open hole, a promise, a split plan). Never touch the brief's other sections (Problem … Constraints, 件 N, 申し送り, 棚卸NNの分割). When nothing changed, leave the brief untouched and say "unchanged" in the return. Preserve line endings. Edit no other file. Do not commit.
Return (English, compact), one block per spec:
<name> | class (evidence) | size | model
 touches: … | functional deps: … | file-overlap with: spec(file), … | split: … | stale/holes: … | immediate fixes: …
Last line: "briefs written: N/M (unchanged: K)".
```

**Leftovers: script first, agent only when large.** Run `python -I <skill dir>/scripts/leftovers.py --out <scratch>/leftovers.md`. It extracts, from the specs that landed since `<PREV>`, only the sections and lines that record leftovers (Implementation Notes, 範囲外, 申し送り, リリース, deferrals; their briefs and requirements are skipped as inputs), each with file:line, and runs the mechanical checks (old `.kiro/specs/<name>/` paths of the landed specs, the completed count). When its last line says `read directly`, the controller reads that file itself, classifies each item as below, does the rest of the fixes agent's list itself (it is greps and one crates.io lookup), and greps steering and READMEs for wording the landed specs made stale — no agent. When it says `use an agent`, launch the agent below on that file instead of the raw spec folders.

**Fixes & brief-less agent** (only when leftovers.py says `use an agent`; read-only, may WebFetch one crate version). Read `<scratch>/leftovers.md` rather than the spec folders; scope is the diff since `<PREV>`, not the whole repository. Classify each "覚え書き" item that a landed spec or the diff touches as (a) trivially fixable now with exact file:line and change, (b) needs a brief (S+ or a decision that can be a requirements agenda item), (c) still blocked (developer decision / missing consumer); scan Implementation Notes and research "範囲外" of specs completed since the last inventory for orphaned leftovers; find stale statements in steering (`focus.md`, `product.md`, `structure.md`, `tech.md`), `README.md`, `dist/README.txt`; grep `.kiro/specs/<name>/` paths in `crates/` and `doc/` that moved to `completed/`; check `image-webp` on crates.io (remove the git pin when ≥ 0.2.5, per the roadmap memo); list `.rs` files ≥ 950 lines among the `.rs` files in `<scratch>/diff-files.md`; count completed entries and brief folders.

## Step 4 — Immediate fixes (controller, right away)
- Apply only class-(a) fixes: doc/comment/path/count/date corrections and wrong pointers in the roadmap. Use the Edit tool.
- Before editing a file an in-flight branch also edits, diff that branch to confirm a different hunk; if a fix lands in a reserved file, hand it to the owning spec's brief instead.
- Never "fix" anything that changes behaviour, security posture (e.g. UIPI), bytes sent to SHIORI, or needs a design decision — those stay in the memo or become briefs.
- Ledger `owner` corrections are not immediate fixes: they move together with `roadmap-draft.md` `owner_count` in each spec's requirements phase — record them in the briefs.

## Step 5 — Granularity: splits and new briefs (subagents)
- Run this step only when Step 3 reported a split or an item was classed (b). Otherwise skip it and write "なし" in the theme section "棚卸NNの分割と起票".
- **Split** a spec when the re-measurement says >20 tasks and it was never carved out before (also when a human wait, e.g. an external review, would sit inside one PR). Do not split specs already carved once for barely crossing 20; mention a parallelism-only split as a developer decision instead.
- **Promote** memo items classed (b). Keep (c) items in the memo with their reason.
- Launch only the agents needed (Opus, background): one performs the splits (append "## <date> 棚卸NNの分割" to the original — In/Out, what moved where, order, size, files — and create the new brief), one writes the promoted briefs. House style: `# Brief: <name>`, Problem / Current State / Desired Outcome / Approach / Scope (In/Out) / Boundary Candidates / Out of Boundary / Upstream / Downstream / Existing Spec Touchpoints / Constraints, first line naming the origin (棚卸NN, split from / memo item), and a closing measurement section (files, size, deps, class, model, agenda). Canon claims must be checked with the ukadoc MCP (`search_docs` takes ONE word). New briefs are CRLF.
- New spec folders are safe for `ukadoc-survey` (it checks `roadmap-draft.md` `[[spec]]` rows exist as folders, not the reverse). Never move or delete brief folders.

## Step 6 — Dependency tree and the next wave
**Edges** (strict): functional dependencies + any shared source file (other unfinished specs and in-flight specs) + the roadmap's serial-lane order. Allowed overlaps that do NOT create edges: appending to `doc/COMPAT_ARCHITECTURE.md` §8; generated files (`THIRD-PARTY-NOTICES.md`, `doc/ukadoc-coverage/report/`); steering; different rows of `doc/ukadoc-coverage/ledger/*.toml`; different rows of `dist/README.txt`. A conditional overlap ("only if …") counts as an edge unless the wave row states a promise that removes it.
**Single-seat resources** (one spec per wave): `Cargo.lock` / `THIRD-PARTY-NOTICES.md` / `tech.md` dependency additions (a pending `/release` version bump holds this seat); the emo-text `lib.rs` pure-file list (only one spec may add an emo-text source file per wave).
**Excluded from the tree**: 据え置き, 保留, 取り下げ予定 specs (neither candidates nor counted as descendants). **Not parallelised**: specs whose core work is measurement (latency/perf/load) — they pollute and are polluted by other work.
**Version gate** (developer 2026-10-16: "ゴールが近いので他の版の割り込みは避けて"・"版に縛らない列は含めてもよいが、その場合、0.0.3 の着地（ウェーブ数）が増えることのないように"):
- The **current version** = the youngest version row (`vX.Y.Z`) in roadmap "版の段" that is not released yet (no tag `vX.Y.Z` on origin: `git ls-remote --tags origin refs/tags/vX.Y.Z` prints nothing). While it is open, the wave takes **no spec from any later version** (v0.1.0 does not enter while v0.0.3 is open), even when seats are empty.
- Spare seats may be filled only from the **version-free column** ("版に縛らない") and only with a **filler** that cannot increase the number of waves the current version needs to land: it is a head; it shares no source file with the wave's current-version specs (allowed overlaps aside); it takes no single-seat resource the current version uses or will need next; it is expected to finish within the wave (its upper size estimate ≤ the lower estimate of the wave's largest current-version spec — the wave ends when its slowest spec lands — and no human/external wait inside it); it needs no quiet desk, no quiet before/after full test run and no long load run (measurement-core specs are never fillers). When unsure, leave the seat empty.
- When the current version has no startable head (all wait on in-flight specs), the wave is fillers only; still no later-version spec.
- Report the current version, how many more waves it needs (critical path of its remaining specs), and that the fillers keep that count.
**Release gate** (developer 2026-10-23: 「ウェーブの対象が０本となり、リリースできるなら、/releaseを開始コマンドとせよ」):
- The current version can be released when it has **no unfinished spec** (every spec of its row landed, none in flight) and every last check the roadmap lists for that version (e.g. a real-run procedure under "版の段") is recorded as done. Then the wave is the release: the start-command block holds only `/release <X.Y.Z>` — no spec of a later version and no filler (the version bump holds the `Cargo.lock` seat; the next inventory after the tag picks the next version). The version row stays in "版の段" until the tag exists.
- No spec left but a listed last check not done → not releasable yet: the wave is fillers only (as when no head is startable), and the check goes to the report as the developer's manual step.
**Procedure**:
1. Heads = specs with no unfinished predecessor (in-flight specs count as unfinished predecessors).
2. Descendant count = number of unfinished specs transitively depending on the head (approximate is fine; state "約").
3. Sort heads by descendant count desc → class A > B > C > D → older filing date (higher in the roadmap).
4. Take every current-version head first (greedily from the top, skipping overlaps); then fill the remaining seats with fillers per the version gate, in the same sort order; stop at 8.
5. For every chosen spec write its touched files and the **same-wave promises** (files it must not touch). List the heads not chosen with one-line reasons, and outline the following wave's candidates.
6. Model per chosen spec from the re-measurement (Opus by default; Fable only with the criterion named per Step 3 (6)).
7. **Outline the next three waves** (after the chosen one; current-version specs only — fillers are chosen per inventory, never outlined): assume the chosen wave landed, repeat steps 1–4 on what is left, and record for each outlined wave its specs, their models and its 入る条件 (what must land first, which files it waits for). Number them on from the current wave (e.g. current C12 → outlined C13, C14, C15). Stop at the wave that empties the current version: if it can then be released (release gate), the next wave is `/release <X.Y.Z>`, and nothing is outlined after it (the next version waits for the tag). If the version needs more than three more waves, the outline ends after three without `/release`; say how many waves remain. When the chosen wave is already the release, there is nothing to outline.

## Step 7 — Write the roadmap (one script)
Do all roadmap edits in one Python script in the scratch directory (run with `python -X utf8`; read with `newline=''`, split on the file's own line ending — the working copy of `roadmap.md` may be LF while `roadmap-history.md`/`focus.md` are CRLF; assert every anchor/replace target exists):
- Append to `roadmap-history.md` a section "## <date> 棚卸NN退避（`roadmap.md` から逐語で移した・main `<BASE>`）" holding verbatim: the old wave table and its sub-notes, the old serial lanes, memo bullets promoted to briefs, the old "直接修正候補", the previous verdict section.
- In `roadmap.md`: new "ウェーブ編成（着手順の正本・<date> 棚卸NN）" (rules, remaining in-flight row, the new wave row with files and promises, next-wave candidates, 保留, and right after the table the wave's start commands as the section's first fenced code block — a one-line bold caption `**<Cn> の開始コマンド**（…）:` then the block exactly as in Step 10 item 4; the overview page reads the first fenced block of this section and shows it with a copy button, developer 2026-10-16 「ウェーブのコード表示をコピペボタン付きで」; `/kiro-wave` reads the same block, and the spec ledger's `Fable` column, with `.claude/skills/kiro-wave/scripts/read-wave.py` to start the wave — if you change this shape, change that script and its self-test in the same commit); right after that block, the outlined waves (Step 6 procedure 7) in the subsection `### その先のウェーブ（<date> の見込み）`, before the section's other `###` subsections (`read-wave.py` reads only the first block, so `/kiro-wave` starts only the current wave; the overview page joins the current wave's block and every block of this subsection, separated by a blank line, into one block with one copy button — its marker is `secNextWaves` in `overview/roadmap-overview.strings.json`): one line saying the waves are an outline the next inventory re-checks, a table `| ウェーブ | spec | 入る条件 | 要件定義 |`, and one fenced block per outlined wave in the Step 10 item 4 form with the caption `Wave <Cn>（見込み）<k>本（<version> <k>本）` (or `Wave <Cn>（見込み）リリース（<version> の spec は 0 本）` with `/release <X.Y.Z>`); omit the subsection when there is nothing to outline; "版の段" keeps live rows only (developer 2026-10-23: 「０本になったバージョンターゲットは消して整理して欲しい。生きてるバージョンターゲットが見れれば良い」) — remove the row of every released version (its tag exists) and of every non-version group whose specs are all landed (e.g. a side group or a "moved out" group), copy each removed row verbatim (with the table header) into this inventory's history section, and drop its term from the count line (never write a `0＋`); the standing columns 「版に縛らない」 and 「いつか」 stay even when empty; an unreleased version with 0 specs stays (release gate); serial lanes with targeted edits (splits and new specs inserted, inventory note); spec ledger heading with the new real count and its history, split notes on the original rows, new rows before the first 据え置き row; memo minus promoted items; new "直接修正候補" (what was fixed, what was handed to specs); living rule 1 if the priority rules changed; new "棚卸NNの裁定" section (what was re-measured, fixes, splits and why, promotions and what stayed, the wave and why, Fable/Opus split, broken same-wave promises, holes found, decisions needed); a theme section "棚卸NNの分割と起票" (Existing Spec Updates + Specs (dependency order)) before "## 予約"; `updated_at` and the header note.
- Then `focus.md` (counts line: completed entries, spec.json-active, brief count, priority rule), and stale bits of `product.md` / `structure.md`. Grep for old wording of changed rules and for references to moved sections.
- Verify with `python -I <skill dir>/scripts/finish.py --overview <scratch>\roadmap-overview-latest.html` and require `finish: all ok`. It checks the line endings of every changed or new file (a CRLF file stays CRLF, no CR CR), prints the wave `read-wave.py` reads (and runs its self-test when it changed), and builds the overview page (running its self-test first when `overview/` changed; a `WARN` fails the check — fix the roadmap). Also glance at `git diff --numstat`: a whole-file diff means broken line endings.

## Step 8 — Verify and commit
- Run `cargo test -p ukadoc-survey` (skip and say so while a quiet-desk hold is active; run it when resumed).
- Commit on the worktree branch, no push: `docs(roadmap): 棚卸NN＝<what landed>の後の再測定・<n> 本を分割・<m> 本を起票・ウェーブ <Cn> を <k> 本で組む`, with a short bullet body and the required attribution trailer.
- **Overview page** (developer 2026-10-16: 「`roadmap-overview-2026-10-16.html` っぽく、最新の状況を html に」「URL をセッションで公開」): `finish.py --overview` (Step 7) already built it from the same roadmap content (the page carries the file's content hash, the same before and after the commit); rebuild only if the roadmap changed after that, with `pwsh -NoProfile -File <worktree>\.claude\skills\kiro-next\overview\roadmap-overview.ps1 -OutFile <scratch>\roadmap-overview-latest.html` (expect `OK wrote …`; `WARN` lines are roadmap mismatches to mention in the report, `FAIL` means no page — report it and go on). The file stays in the scratch directory and is never committed (no new `doc/roadmap-overview-*.html` from this skill). Read the whole generated file before publishing (the Artifact rule for files Claude did not write), then publish it with the `Artifact` tool as a new private artifact: `file_path` = that file, `title` = `roadmap-overview-latest`, `icon` = `map`, `description` = `棚卸NN（main <BASE>）の時点のロードマップの俯瞰`. Keep the returned URL for the report. If publishing is refused or fails, say so in the report and go on.

## Step 9 — Memory
- Update the priority-class memory if the rules changed; write the inventory memory (base SHA, counts, wave, splits, new briefs, pending items, branch/commit) and add it to the "進行中" part of `MEMORY.md`.

## Step 10 — Report (persona language, conclusions only)
1. Immediate fixes done (and what was handed to specs).
2. Briefing: re-measured count (and why those were picked; whether the fast path was used), splits, new briefs, resulting brief count.
3. Dependency-tree summary: largest heads and why they were or were not taken.
4. The wave, exactly in this form (one code block; first the specs of the current version — the version the wave really advances — then the fillers that entered the spare seats; inside each group Fable first; omit an empty Fable/Opus line and an empty filler group; no per-spec notes inside). Developer 2026-10-16: the report must show which specs are the real next version and which are cut-ins:
```
Wave <Cn> <k>本（<version> <a>本＋割り込み <b>本）
【本来の次の版 <version>】
Fable
/kiro-start <name>
Opus
/kiro-start <name>

【割り込み（版に縛らない・<version> の着地を遅らせない）】
Opus
/kiro-start <name>
```
   When the release gate (Step 6) holds, the block is instead exactly:
```
Wave <Cn> リリース（<version> の spec は 0 本）
【本来の次の版 <version>】
/release <X.Y.Z>
```
   When waves are outlined (Step 6 procedure 7), the report shows the current wave and the outlined waves in that ONE code block, one blank line between waves, with the captions of Step 7 (`Wave <Cn>（見込み）<k>本（<version> <k>本）`, …, `Wave <Cn>（見込み）リリース（<version> の spec は 0 本）`), so one copy takes them all (pasta developer 2026-10-24: 「３つを１つのコードとしてコピーしたいので、枠は１つで良い」). Put each outlined wave's 入る条件 as one line each right after the block, not inside it.
   `/kiro-wave` lands the inventory (PR → squash merge, after one approval dialog) and then starts the Opus specs of the current wave only as separate sessions, leaving the Fable ones to the developer; the outlined waves start after the next inventory confirms them. Say so in one line. For the release block it starts nothing: `/release` runs only when the developer types it — say so instead.
5. Decisions needed from the developer (none of them block the wave) and any manual steps.
6. The overview page URL from Step 8, as one Markdown link whose text carries the inventory number and base SHA: `[ロードマップの俯瞰（棚卸NN・main <BASE>）](<url>)`. Never a bare URL or a code block (pasta developer 2026-10-17: 「URLリンクにすればよい」).

Write this whole report as ordinary text **before** the approval gate's question (and before any `wrap_up` call). The gate's question never replaces the report (developer 2026-10-16: 「ウェーブを教えてもらってないのに？」 — the wave was missing when only the approval box was shown).

## Step 11 — Approval gate (STOP here)
- After the report, STOP. Do not push, open a PR or merge.
- Only when the developer explicitly instructs it (e.g. 「PRしてスクワッシュマージ」) push the branch, open the PR (body ends with the required attribution line), then show the PR number, title and changed-file summary and ask for confirmation once more before `gh pr merge --squash`. After merging, report the main commit.
- If the run was a dry run or the developer says not to merge the inventory, move the commit to a local backup branch (`git branch <name> <sha>`) instead of leaving it on the branch that will be merged.

</instructions>
