# Brief: areka-P0-test-roots-under-target

> 2026-10-07 `ghost-standard-balloon` の完了時の棚卸で起票（`completed/areka-P0-ghost-standard-balloon/tasks.md` の Implementation Notes の 3・最終検証 ⑵）。

## Problem

開発の決まり「実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ」（掃除漏れが多いので `C:\` 直下も OS の一時フォルダも不可）に、既存のテストの多くが反している。`temp_path_kit::TempPath::new` は OS の一時フォルダ（`%TEMP%`）に根を作るので、テストが落ちたり途中で止まったりすると根が `%TEMP%` に残り、ワークツリーを消しても掃除されない。

## Current State

- `crates/temp-path-kit/src/lib.rs` に `TempPath::new`（OS の一時フォルダ）と `TempPath::under_target`（`<workspace>\target\test-roots`・`ghost-standard-balloon` のタスク 3 で追加）の 2 つの入口がある。
- `TempPath::new` を呼ぶのは 80 ファイル・240 か所（2026-10-07 の main＋本枝で `git grep -c "TempPath::new" -- crates` を数えた）。crate 別のファイル数: `areka` 46・`areka-ghost` 22・`areka-emo-present` 5・`areka-parsers` 3・`temp-path-kit` 2・`areka-sylphya` 1・`log-capture-kit` 1。
- `ghost-standard-balloon` の新しいテストだけは `under_target` を使っている。同じ spec が頼る `catalog_tests.rs` の同梱のテスト 3 本は「本体の差分 0 行」の約束で `new` のまま残した。

## Desired Outcome

- ワークスペースのテストが作る一時の根はすべて `target\` の下にでき、`%TEMP%` には何も作らない。
- 以後 `TempPath::new` を足したテストが黙って入らない（入口を 1 つにするか、`%TEMP%` を使う呼び出しを見張りのテストで赤にする）。

## Approach

`TempPath::new` の中身を `under_target` と同じ置き場へ替える（呼び出し 240 か所を書き換えずに済む・最も短い）か、`new` を消して呼び出しを `under_target` へ置き換えるかを要件で決める。`OUT_DIR` や `CARGO_TARGET_DIR` を使うビルドでも `target` の位置を正しく引けることを確かめる（`under_target` の現在の引き方を流用）。

## Scope
- **In**: `temp-path-kit` の入口の整理・既存の呼び出しの移行・`%TEMP%` を使わないことの見張り。
- **Out**: 一時の根を作らないテストの書き換え・`devroot::fresh_root`（既に `target\nar-samples`）の変更・実機の手順書。

## Boundary Candidates
- `temp-path-kit` の API（入口を 1 つにするか 2 つ残すか）
- 呼び出し側の機械的な置き換え（crate ごとに分けられる）

## Out of Boundary
- テストの中身・期待値の変更
- 1,000 行の番人などほかの見張りの変更

## Upstream / Downstream
- **Upstream**: `ghost-standard-balloon`（`TempPath::under_target` を足した）
- **Downstream**: 以後一時の根を作るすべてのテスト

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `ghost-session-test-load-flake`・`areka-test-threads-av`（同じテストを並べて回す・同時に走らせると原因の切り分けが濁る）

## Constraints
- 1 ファイル 1,000 行の番人を守る。
- 並べて回すテストの根が衝突しないこと（`under_target` は名前に一意の印を付けている＝そのまま使う）。
- 規模の見立て: 入口だけ替える案なら S（3〜5）、置き換える案なら M（8〜12）。
