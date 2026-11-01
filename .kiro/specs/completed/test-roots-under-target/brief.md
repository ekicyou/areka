# Brief: test-roots-under-target

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
- **Adjacent**: `ghost-session-test-load-flake`・`areka-test-binary-stability` の件 1（同じテストを並べて回す・同時に走らせると原因の切り分けが濁る）

## Constraints
- 1 ファイル 1,000 行の番人を守る。
- 並べて回すテストの根が衝突しないこと（`under_target` は名前に一意の印を付けている＝そのまま使う）。
- 規模の見立て: 入口だけ替える案なら S（3〜5）、置き換える案なら M（8〜12）。

## 2026-10-31 棚卸㉜の再測定（main d374f901）
- **前提の変化**: 入口の部品 `crates/temp-path-kit/src/lib.rs`（148 行・OS の一時フォルダに作る `new` は 101 行目・`target\` の下に作る `under_target` は 115 行目）・`lib_tests.rs`（165）・見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs`（783・直に OS の一時フォルダを引く所の例外表 19 ファイル）は 10-23 から変わらない。`TempPath::new` の呼び出しは 79 ファイル・234 か所→**80 ファイル・240 か所**（`areka` 41・`areka-ghost` 23・`areka-emo-present` 7・`areka-parsers` 4・`temp-path-kit` 2・`areka-sylphya` 1・`areka-impl-watch` 1・`log-capture-kit` 1）。増えたのは ✅ `placement-measure-bake-once` の新しい `crates/areka-emo-present/src/shell_target_quiet_tests.rs` の 6 か所＝見張りが無いので、新しいテストは今も OS の一時フォルダに根を作る。`under_target` の名前が出るファイルは 76→96。
- **触るファイル**（入口の中身だけ替える案）: 上の 3 本だけ。240 か所は書き換えずに一度に動く。例外表まで直すなら、そのうち 18 ファイル（`shiori-host32-*`・`areka-ghost`・`areka` の `placement/` のテスト・emo-present。`crates/areka/src/install/fetch_url.rs` は本番のダウンロードの置き場なので外す）が足される。
- **規模**: 入口を替える案で 4〜7（例外表まで入れると 8〜11）のまま。
- **先に要るもの**: 無い（先頭）。v0.1.0 の `shell-scale-linkage` とは重なり 0（上の 3 本を挙げる brief は無い）。例外表まで入れる形だけ、`shiori-host32-host/tests/` の `lifecycle_cyclic_e2e.rs`・`lifecycle_kill_e2e.rs` が `clippy-199-lints` の段 1 と重なる。1 本だけの席は取らない。
- **机・待ち**: 静かな机・前後の全体テストの比べ・長い負荷の走行・人の待ちは要らない（最後の全体テスト 1 回）。ただし `areka-test-binary-stability`（ウェーブの外）を静かな机で測っている間は着地させない（全部のテストの一時の根の置き場が動き、測りの基準が変わる）。
- **隙間（割り込み）の席**: 入口を替える案の上限 7 は C15 の見込みの v0.1.0（`shell-scale-linkage` の下限 10）以下＝入れられる。棚卸㉛で外したのは、C14 の上限 6 を超えたためだけ。
- **気をつけること**: パスが長くなる（ワークツリーの名前の長さで 50 字前後）。実物の `pasta.dll` を読む `crates/areka-ghost/tests/ghost/real_pasta_test.rs` は今も 108 行目で `TempPath::new`、札が 60 字を超えるテスト（`areka-ghost` の `runtime_tests.rs` ほか）もある＝先に長さを確かめ、要るなら名前を短くする。`under_target` は `CARGO_TARGET_DIR` を見ず、いつもワークスペースの根の `target` を使う（決まりには合う・brief の「正しく引ける」はこの違いを決める話になる）。
- **優先度の区分**: C のまま（開発の決まり「一時フォルダは `target\` の下だけ」による持ち越し）。
- **要件定義のモデル**: Opus のまま。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 本文の「`under_target` を使うのは 2 ファイル」は古い（上のとおり）。見直しの `audit-test-kits`（v0.1.0 の後）が `temp-path-kit` を受け持つ＝先にこの spec が着地しても困らない。
