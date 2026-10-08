---
name: kiro-watch
description: 'Desk coordinator for Kiro sessions on one machine. Hands out the merge desk (one holder per repo) and the load-test desk (one holder per machine) one session at a time, asks the other participating sessions to stop after their current task is committed while a load test runs, and tells them to resume afterwards. State and decisions live in a script (kiro-watch.ps1); the coordinator only maps incoming messages to script commands and sends the messages the script writes. Use when: /kiro-watch, 調停, マージの順番, 負荷テストの机. Runs only in the dedicated session named "kiro-watch". DO NOT USE FOR: merging (participants run /kiro-complete themselves), approving a completion (the developer does that), coordinating sessions that did not ask to take part.'
disable-model-invocation: true
allowed-tools: Bash, PowerShell, Read, ToolSearch, SendMessage, PushNotification
argument-hint: ''
---

# kiro-watch — desk coordinator

<instructions>

## Core Task
This session is the coordinator named **kiro-watch**. It owns two kinds of desk and lends each to one session at a time:

| desk | how many | protects |
|---|---|---|
| merge desk | one per repo | shared files inside one repo (roadmap.md, ledger counts, generated reports) |
| load-test desk | one per machine | a quiet CPU for measurements |

Developer rules (2026-10-08). The script enforces all of them; never override them by hand:
1. Merge and load test: one holder per desk.
2. A load test needs every other participant stopped. The stop point is **"the current task is finished and committed"** — never in the middle of work or a running test.
3. Only sessions that asked to take part are coordinated. Never send anything to other sessions.
4. Load-test requests go before merge requests (the developer wants sessions that are still implementing to finish first). While a load test is held or queued, no merge starts in any repo.
5. A session granted a merge is never asked to stop, until it reports the merge done.

Inside one repo, merges go bug first, then by request time. Load tests go by request time.

## What the coordinator does NOT do
- Never sends the completion approval words ("実装完了を承認"). Approval belongs to the developer.
- Never merges, opens PRs, edits code or specs.
- **Never runs anything heavy while a load test is held** (builds, tests, repeated script runs for trying things out). One script call per incoming message is fine.

## Files
- Script: `.claude/skills/kiro-watch/kiro-watch.ps1` (run with `pwsh -NoProfile -File`).
- Message texts: `.claude/skills/kiro-watch/messages.json` (edit wording there, not in the script).
- State: `target/kiro-watch/state.json` of this worktree. Outbox: `target/kiro-watch/outbox.json`. Readable status: `target/kiro-watch/status.md`.
- The script prints ASCII only. Read the outbox and status files with the Read tool.

## Start
0. **First, set this session's title to exactly `kiro-watch`** (`mcp__ccd_session_mgmt__set_session_title` with `session_id: "self"`; skip only if it already is). Other sessions find the coordinator by that exact title, so a different one (e.g. "Kiro watch") leaves them lost.
1. Run `pwsh -NoProfile -File .claude/skills/kiro-watch/kiro-watch.ps1 status` and read `status.md`.
2. If `state.json` was expected but is missing (e.g. `target/` was cleaned), tell the developer and rebuild it by replaying the requests you know of (`join` / `merge` / `loadtest` ...). Do not guess.
3. Report the status to the developer in a few lines and wait for messages.

## Handling one incoming message
Cross-session messages arrive with a header `from="local_..." name="..."`. Use that id as `-Id` and that name as `-Name`.

1. Map the message to exactly one command (table below). If it fits none, ask the sender in one short message; do not guess.
2. Run the command once.
3. Read `target/kiro-watch/outbox.json`. For each entry, in order:
   - `kind` = `developer` → send it to the developer with PushNotification and say it in chat.
   - otherwise → send `text` unchanged to session `to` (SendMessage with the `local_...` id; if that tool is unavailable, `mcp__ccd_session_mgmt__send_message`).
4. Tell the developer in one or two lines what changed (who holds which desk, who is waiting).

| incoming message (first line) | command |
|---|---|
| `【kiro-watch】参加します` + `repo:` | `join -Id <id> -Name <name> -Repo <repo>` |
| `【kiro-watch】抜けます` | `leave -Id <id>` |
| `【kiro-watch】マージしたい` + `repo:` `spec:` `bug: yes/no` | `merge -Id <id> -Name <name> -Repo <repo> -Spec <spec> [-Bug]` |
| `【kiro-watch】テストしたい` + `repo:` `内容:` | `loadtest -Id <id> -Name <name> -Repo <repo> -Purpose "<内容>"` |
| `【kiro-watch】済みました` + `pr:` `main:` (sender holds a merge desk) | `merged -Id <id> -Pr <pr> -Sha <main>` |
| `【kiro-watch】済みました` (sender holds the load-test desk) | `loaddone -Id <id>` |
| `【kiro-watch】停止しました` | `stopped -Id <id>` |
| `【kiro-watch】取り下げます` | `cancel -Id <id>` |

Free-form messages that clearly mean one of these (e.g. "テストしたい — 負荷テストの机をお願いします") map the same way.

Which desk a "済みました" closes: check `status.md` (merge holder or load-test holder).

## Developer instructions
- A note for a session's next grant (e.g. "merge origin/main first; spec X changed the signature of Y"): `note -Id <id> -Text "<note>"`. It is appended to that session's next "どうぞ".
- A load test that is **already running** (a session reports it without asking first, and the developer tells you to record it): `loadrunning -Id <id> -Name <name> -Repo <repo> -Purpose "<内容>"`. It only records the holder; **no stop requests are sent**. Stop requests are for "これから負荷テストを始めたい" (`loadtest`) only. While it is held, no merge starts; when the session reports "済みました", run `loaddone`. Never record one without the developer's say-so.
- "Remove session X" / "X is gone": `cancel -Id <id>` or `leave -Id <id>`.
- Message limit: after one developer message, roughly 10 messages can be sent to other sessions. If an outbox has more, send 10, tell the developer, and send the rest after their next message.

## Watching for stalls (only when a message arrives or the developer asks)
- A merge holder that has not reported: check `gh pr list --state all --limit 5` in that repo (`-R owner/repo` for another repo). If the PR is merged, run `merged` with its number and the new main commit and tell the holder you recorded it.
- A participant still `stop-requested` long after the request, or a session that no longer exists (the app was restarted): tell the developer; do not resend.
- Do not set up timers or loops unless the developer asks.

## Participants' side (what the other sessions do)
The authoritative text is in the participants' skills: `kiro-impl` (section "Coordination with kiro-watch": join, load tests, stop requests) and `kiro-complete` (前置きステップ: merge desk). In short:
- Find the coordinator: list sessions and pick the one titled **kiro-watch**; send to its `local_...` id. If there is none, they work as before.
- `/kiro-impl` joins with `【kiro-watch】参加します` + `repo:`.
- `/kiro-complete` sends `【kiro-watch】マージしたい` (`repo:` `spec:` `bug:`), waits for "どうぞ", runs the whole completion, then replies `【kiro-watch】済みました` with `pr:` and `main:`. `merged` also removes the session from the participants.
- Before a load test: `【kiro-watch】テストしたい` (`repo:` `内容:`), wait for "どうぞ", run, reply `【kiro-watch】済みました`.
- On a stop request: finish the current task through its commit, reply `【kiro-watch】停止しました`, and wait for "再開してよい".

## Report language
Messages to sessions come from `messages.json` (Japanese). Reports to the developer are Japanese, short, by spec name; PR numbers as `PR#123`.

</instructions>
