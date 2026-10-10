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

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 数は 10-07 と同じ＝`TempPath::new` は 80 ファイル・240 か所（main `ee3af616` で数え直した）。`TempPath::under_target` を使うのは今も 2 ファイル（`crates/areka-ghost/src/catalog_standard_balloon_tests.rs`・`crates/areka/src/boot_config_balloon_tests.rs`）。`ghost-session-test-load-flake` の申し送り（上書きの失敗を見るテストの一時フォルダが OS の一時フォルダに在る）は `crates/areka/src/install/desk_overwrite_tests.rs` の呼び出しで、この数の内。
- **crate 別（ファイル／か所）**: `areka` 46／168（内訳: `install/` 8・42、根の直下 6・37、`emo2_boot/` 9・31、`menu/` 6・16、`placement/` 7・14、`readme/` 4・9、`input_events/` 1・8、`update/` 3・4、`mcp/` 1・3、結合テスト `tests/` 1・4）・`areka-ghost` 22／39（`src` 12 ファイル・`tests/ghost` 10 ファイル）・`areka-emo-present` 5／13・`areka-parsers` 3／6・`areka-sylphya` 1／3・`temp-path-kit` 2／10・`log-capture-kit` 1／1。
- **触るファイル**: 入口の中身だけ替える案なら `crates/temp-path-kit/src/lib.rs`（148 行）・`lib_tests.rs`・見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs`（783 行）の 3 本で 240 か所が一度に動く＝ほかの spec の兄弟のテストと 1 行も重ならない。呼び出しを 1 つずつ書き換える案は 80 ファイルに触り、`install/`・`menu/`・`readme/`・`emo2_boot/`・`placement/` の兄弟のテストがほかの spec と重なるので、取るなら crate ごとに段を分ける。
- **規模**: 入口を替える案で S〜M（5〜8）。書き換える案は M（10〜14）。
- **仕事の芯は機械的な置き換え**で測定ではない。確かめは静かな机の全体テスト 1 回（約 5 分）を前と後に 1 度ずつ（置き場が動くと所要時間が変わりうるので比べる）。負荷の再現は要らない。
- **先に要るもの**: なし。入口を替える案なら、ファイルの重なりは 0 でどの席にも入る。ただし `areka-test-threads-av`・`test-wait-marker-gaps` とは同じウェーブに置かない（240 か所の読み書きの場所が変わり、あちらの比べる基準が動く）。
- **優先度の区分**: C（バグでない持ち越し。根拠は開発の決まり「一時フォルダは `target\` の下だけ」で、開発者がこの spec を名指しで頼んだ記録は無い）。
- **要件定義のモデル**: Opus。
- **分割の案**: なし（20 を超えない）。
- **見つけた穴・古くなった記述**:
  - 入口を通らずに OS の一時フォルダを直に引く所が、見張りの例外表に 19 ファイルある（`areka-ghost` 5・`areka` 3・`areka-emo-present` 1・`shiori-host32-helper` 3・`shiori-host32-host` 6・`shiori-host32-testdll-loadu` 1）。brief の 240 か所はこれを数えていない。「`%TEMP%` に何も作らない」を満たすにはここも要る。うち `crates/areka/src/install/fetch_url.rs` は本番のコード（ダウンロードの置き場・テストの根ではない）＝対象から外す。`shiori-host32-host/tests/` の `lifecycle_cyclic_e2e.rs`・`lifecycle_kill_e2e.rs` は `clippy-199-lints` の段 1 と重なる。
  - 置き場が `target\test-roots` になると、ワークツリーの中ではパスが OS の一時フォルダより約 40 字長くなる。札（名前の材料）が 60 字を超えるテストがあり（`areka-ghost` の `runtime_tests.rs` ほか）、実物の `pasta.dll` を読む結合テスト（`tests/ghost/real_pasta_test.rs`）は短いパスが前提＝長さの上限を先に確かめ、要るなら名前を短くする。
  - `under_target` は `CARGO_TARGET_DIR` を見ず、いつもワークスペースの根の `target` を使う（検体の置き場 `sample-ghost-kit` の引き方とは別）。決まりには合うが、brief の「正しく引けることを確かめる」はこの違いを決める話になる。
