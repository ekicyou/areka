---
name: kiro-watch-clear
description: 'Clear the whole desk state of areka-impl-watch (participants, merge desks and queues, load-test desk and queue, stop requests, wait and watch records) in one go, then show the resulting status. Calls `areka-impl-watch.exe clear` and `status` from the folder named by the environment variable AREKA_IMPL_WATCH_HOME; the previous state is kept as a backup file. Use when: /kiro-watch-clear, 机の状況を全部消す, 机を最初からやり直す, impl-watch を clear. DO NOT USE FOR: removing one session (use `areka-impl-watch.exe leave --id <id>`), only looking at the state (use `status`), the coordinator session of the kiro-watch skill (its state lives elsewhere), installing or updating the exe (see doc/impl-watch.md).'
allowed-tools: Bash, Read
argument-hint: ''
---

# kiro-watch-clear — clear the areka-impl-watch desks

<instructions>

## Core Task
Empty the desk state of `areka-impl-watch` and tell the developer what happened, in Japanese, in 2-3 lines.

`clear` asks nothing and acts at once: it moves the current `state.json` to `state.json.cleared-<UTC>` and starts from an empty state. Every running wait and watch then ends with exit code 3. The developer typing `/kiro-watch-clear` is the confirmation; do not ask again, and do not run this skill unless the developer asked for a full clear.

## Rules
- Tools: Bash and Read only. Run exactly the one command below, once.
- Use the inherited `AREKA_IMPL_WATCH_HOME` as it is. Never set, change, rewrite or convert it (no `AREKA_IMPL_WATCH_HOME=...` prefix, no `/c/...` form, no `cd`). The exe refuses a value that is not a `C:\...` absolute path: it ends with exit code 1 and one standard-error line starting with `AREKA_IMPL_WATCH_HOME must be an absolute path`, and reads and writes nothing. Report that line as it is; do not repair the value.
- Do not build, copy or install the exe from this skill. Do not edit files under the home folder by hand.

## Steps

Run this single Bash command:

```bash
if [ -z "$AREKA_IMPL_WATCH_HOME" ]; then echo "stop: no-home"
elif [ ! -f "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" ]; then echo "stop: no-exe"
else "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" clear && "$AREKA_IMPL_WATCH_HOME/areka-impl-watch.exe" status; echo "exit=$?"
fi
```

It does the four steps in order and stops at the first one that does not hold:

1. **Environment variable empty** → prints `stop: no-home`. Nothing was called and nothing was cleared. Go to "Stopped without clearing".
2. **Exe not in the home folder** → prints `stop: no-exe`. Nothing was called, nothing was cleared, no file was created. Go to "Stopped without clearing".
3. **`clear`** → prints `cleared; backup: <path>` (or `cleared; backup: none` when there was no state file to back up).
4. **`status`** (only after `clear` ended with 0) → prints the summary line `participants=0 load-holder=none load-queued=0 merging=0 merge-queued=0 not-working=0 waits=0`, the load desk line, and `status: <path of status.md>`.

The last line `exit=<n>` is the exit code of the last command that ran.

## Report (Japanese, 2-3 lines)

**`exit=0`**:
- Line 1: that everything was cleared, and the backup path exactly as printed after `backup:` (for `backup: none`, say there was no state to back up).
- Line 2: the summary in words (参加者 0・机の持ち主なし・待ち 0) taken from the `participants=...` line. If a number is not 0, say which one, as it is.
- Line 3 (optional): restoring from the backup is in `doc/impl-watch.md` 「6. 困ったとき」→「全部消す」.

The summary line is enough; Read the `status.md` named by `status:` only when that line is missing or the developer asks for details.

**Any `exit=<n>` other than 0** (a failure; 1 and 2 are the usual values, and every other non-zero value is treated the same way): report that it failed, which command failed, and attach the standard-error text unchanged. Do not retry and do not try to repair anything.
- No `cleared; backup:` line in the output → `clear` reported the failure and `status` was not called.
  - The standard-error line starts with `AREKA_IMPL_WATCH_HOME must be an absolute path` (or `AREKA_IMPL_WATCH_HOME cannot be created`) → the exe stopped before it opened the state. Say that nothing was cleared and that the value of the environment variable has to be fixed (`doc/impl-watch.md` 「2. 入れ方」).
  - Any other line → do not say whether the state was cleared or kept. Most failures happen before the state file is touched (it is kept), but one happens in between: the state file was already moved to `state.json.cleared-<UTC>` and the new empty state could not be written (then there is no `state.json`, and `status` prints `no state file`). Say it is unknown and that `areka-impl-watch.exe status` shows it once the cause is removed. A `status.md` that cannot be written is not such a failure: `clear` still ends with 0 and prints its line.
- A `cleared; backup:` line is there → the clear succeeded (give the backup path) and `status` failed.

## Stopped without clearing
For `stop: no-home` and `stop: no-exe`, tell the developer in Japanese, in 2-3 lines:
- which of the two it was (環境変数 `AREKA_IMPL_WATCH_HOME` が空／置き場所に `areka-impl-watch.exe` が無い),
- that nothing was cleared,
- that the way to install is in `doc/impl-watch.md` 「2. 入れ方」 (set the user environment variable once to a `C:\...` absolute path, build, copy the exe there, restart the Claude app).

Then stop. Read that section only if the developer asks for the steps.

</instructions>
