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

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**: 数は 10-07 と同じ＝`TempPath::new` は 80 ファイル・240 か所（main `ee3af616` で数え直した）。`TempPath::under_target` を使うのは今も 2 ファイル（`crates/areka-ghost/src/catalog_standard_balloon_tests.rs`・`crates/areka/src/boot_config_balloon_tests.rs`）。`ghost-session-test-load-flake` の申し送り（上書きの失敗を見るテストの一時フォルダが OS の一時フォルダに在る）は `crates/areka/src/install/desk_overwrite_tests.rs` の呼び出しで、この数の内。
- **crate 別（ファイル／か所）**: `areka` 46／168（内訳: `install/` 8・42、根の直下 6・37、`emo2_boot/` 9・31、`menu/` 6・16、`placement/` 7・14、`readme/` 4・9、`input_events/` 1・8、`update/` 3・4、`mcp/` 1・3、結合テスト `tests/` 1・4）・`areka-ghost` 22／39（`src` 12 ファイル・`tests/ghost` 10 ファイル）・`areka-emo-present` 5／13・`areka-parsers` 3／6・`areka-sylphya` 1／3・`temp-path-kit` 2／10・`log-capture-kit` 1／1。
- **触るファイル**: 入口の中身だけ替える案なら `crates/temp-path-kit/src/lib.rs`（148 行）・`lib_tests.rs`・見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs`（783 行）の 3 本で 240 か所が一度に動く＝ほかの spec の兄弟のテストと 1 行も重ならない。呼び出しを 1 つずつ書き換える案は 80 ファイルに触り、`install/`・`menu/`・`readme/`・`emo2_boot/`・`placement/` の兄弟のテストがほかの spec と重なるので、取るなら crate ごとに段を分ける。
- **規模**: 入口を替える案で S〜M（5〜8）。書き換える案は M（10〜14）。
- **仕事の芯は機械的な置き換え**で測定ではない。確かめは静かな机の全体テスト 1 回（約 5 分）を前と後に 1 度ずつ（置き場が動くと所要時間が変わりうるので比べる）。負荷の再現は要らない。
- **先に要るもの**: なし。入口を替える案なら、ファイルの重なりは 0 でどの席にも入る。ただし `areka-test-binary-stability` の件 1・2 とは同じウェーブに置かない（240 か所の読み書きの場所が変わり、あちらの比べる基準が動く）。
- **優先度の区分**: C（バグでない持ち越し。根拠は開発の決まり「一時フォルダは `target\` の下だけ」で、開発者がこの spec を名指しで頼んだ記録は無い）。
- **要件定義のモデル**: Opus。
- **分割の案**: なし（20 を超えない）。
- **見つけた穴・古くなった記述**:
  - 入口を通らずに OS の一時フォルダを直に引く所が、見張りの例外表に 19 ファイルある（`areka-ghost` 5・`areka` 3・`areka-emo-present` 1・`shiori-host32-helper` 3・`shiori-host32-host` 6・`shiori-host32-testdll-loadu` 1）。brief の 240 か所はこれを数えていない。「`%TEMP%` に何も作らない」を満たすにはここも要る。うち `crates/areka/src/install/fetch_url.rs` は本番のコード（ダウンロードの置き場・テストの根ではない）＝対象から外す。`shiori-host32-host/tests/` の `lifecycle_cyclic_e2e.rs`・`lifecycle_kill_e2e.rs` は `clippy-199-lints` の段 1 と重なる。
  - 置き場が `target\test-roots` になると、ワークツリーの中ではパスが OS の一時フォルダより約 40 字長くなる。札（名前の材料）が 60 字を超えるテストがあり（`areka-ghost` の `runtime_tests.rs` ほか）、実物の `pasta.dll` を読む結合テスト（`tests/ghost/real_pasta_test.rs`）は短いパスが前提＝長さの上限を先に確かめ、要るなら名前を短くする。
  - `under_target` は `CARGO_TARGET_DIR` を見ず、いつもワークスペースの根の `target` を使う（検体の置き場 `sample-ghost-kit` の引き方とは別）。決まりには合うが、brief の「正しく引けることを確かめる」はこの違いを決める話になる。

## 2026-10-16 棚卸㉖の再測定（main `4cf510c1`）

- 前提の変化: OS の一時フォルダに根を作る入口（`TempPath::new`）は 80 ファイル・240 か所 → **81 ファイル・237 か所**（`areka` 46 ファイル・164 か所〔4 減った〕・`areka-impl-watch` 1 か所〔`store_ports_tests.rs` の 667 行目・増えた〕。ほかのクレートは同じ）。`target\` の下に作る入口（`TempPath::under_target`）を使うテストは 2 ファイル → **29 ファイル**に増えた（`areka-impl-watch` 10・中継 4・`baseware-root-list` が足した `roots/`・`menu/`・`boot_config` ほかのテスト 13・既定バルーンの 2）。入口の部品（`crates/temp-path-kit/`）と見張り（`crates/log-capture-kit/tests/temp_path_guard_test.rs`・例外 19 件のまま）は 10-10 から動いていない。
- 前後の全体テストの比べは要らない（棚卸㉓・㉕の見立てを改める）: 比べる理由は「置き場が動くと所要時間が変わりうる」だけで、望む結果（`%TEMP%` に何も作らない）の確かめではない。`target\` の下の根で走るテストはすでに 29 ファイル在り、C7 の 8 本の完了の全体テストはそれで緑だった。確かめは、どの spec とも同じ最後の全体テスト 1 回で足りる。時間が気になるなら、その 1 回の所要時間を C7 の完了の記録と並べて書けばよく、余分な走行も静かな机も要らない。
- 触るファイル（入口の中身だけ替える案）: `crates/temp-path-kit/src/lib.rs`（148 行）・`lib_tests.rs`（165 行）・見張り `temp_path_guard_test.rs`（783 行）。237 か所は一度に動き、書き換えは要らない。例外表の 19 ファイル（直に OS の一時フォルダを引く所）まで直すなら、`shiori-host32-*` の 11 本・`areka-ghost` の 5 本・`areka` の `placement/` のテスト 2 本・emo-present の 1 本が足される（`install/fetch_url.rs` は本番のダウンロードの置き場なので外す）。
- 規模: 入口を替える案で 4〜7 タスク（例外表まで入れると 8〜11）。新しい依存は足さない。人の手の確かめは要らない。
- 先に要るもの: なし（先頭）。入口を替える案なら今の版の 2 本と重なり 0（例外表まで入れても、`placement/` のテスト 2 本は `user-data-root` が触る `placement/persist.rs` とは別のファイル）。
- 今の版の席に入れられるか: 入口を替える案の上限 7 は `user-data-root` の下限 12 以下＝`user-data-root` と同じウェーブなら入れられる。`write-failure-notice`（下限 6）だけのウェーブでは超える。
- 気をつけること: パスが長くなる。ワークツリーの `target\test-roots\` は OS の一時フォルダより約 50 字長い（ワークツリーの名前の長さで変わる）。短いパスが要る実物の `pasta.dll` の結合テスト（`areka-ghost` の `tests/ghost/real_pasta_test.rs`）と、札が長いテストを先に確かめる。
- 優先度の区分: C（開発の決まり「一時フォルダは `target\` の下だけ」による持ち越し）。
- 要件定義のモデル: Opus。
- 分割の案: なし。
- 見つけた穴・古くなった記述: brief 本文の「呼ぶのは 80 ファイル・240 か所」「`under_target` を使うのは 2 ファイル」は上のとおり古い。例外表のうち `shiori-host32-host/src/process_host.rs` は `dev-helper-x64-clobber` が 10-16 に触った（例外の行はそのまま）。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 入口の部品 `crates/temp-path-kit/src/lib.rs`（148 行・`new` 101 行目・`under_target` 115 行目）・`lib_tests.rs`（165 行）・見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs`（783 行）は 10-16 から変わっていない。OS の一時フォルダに根を作る `TempPath::new` は 81 ファイル・237 か所→**82 ファイル・238 か所**（`areka` 46・`areka-ghost` 23・`areka-emo-present` 5・`areka-parsers` 3・`temp-path-kit` 2・`areka-sylphya` 1・`areka-impl-watch` 1・`log-capture-kit` 1）。`target\` の下に作る `under_target` を使うのは 29→**62 ファイル**（実機の試験の窓口 `real-run-kit` 6・`areka-impl-watch` 11・中継 4・`areka-user-data` 2 などが増えた）。
- 触るファイル（入口の中身だけ替える案）: `crates/temp-path-kit/src/lib.rs`・`lib_tests.rs`・`crates/log-capture-kit/tests/temp_path_guard_test.rs` の 3 本。238 か所は一度に動き、書き換えは要らない。例外表の 19 ファイルまで直すなら、棚卸㉖の節のとおり足される。
- 規模: 入口を替える案で 4〜7（例外表まで入れると 8〜11）のまま。
- 先に要るもの: 無い＝先頭。
- v0.1.0 の spec との重なり（隙間に入れられるか）: 入口を替える案の 3 本は、版の段の v0.1.0 の行の 23 本のどの brief にも挙がっていない＝重なり 0（`pasta-in-windows-verification`・`dot-sakura-specimen` が挙げる `log-capture-kit` のファイルは別の `sample_path_guard_test.rs`）。1 本だけの席（依存の追加・emo-text の純粋なファイルの表）も取らない。人や外の待ち・静かな机・前後の全体テストの比べも要らない（棚卸㉖の改めのまま）＝**v0.1.0 の間の隙間の候補になる**。条件: そのウェーブで一番大きい v0.1.0 の spec の下限が 7 以上（例外表まで入れるなら 11 以上）。`areka-test-binary-stability` を静かな机で測っている間は着地させない（全部のテストの一時の根の置き場が動き、測りの基準が変わる）。
- 例外表まで入れる案のときの重なり: `shiori-host32-host/tests/` の 2 本が版に縛らない `clippy-199-lints` の段 1 と重なる。v0.1.0 とは 0 のまま。
- 気をつけること: パスが長くなる（棚卸㉖の節のとおり）。実物の `pasta.dll` を読む `crates/areka-ghost/tests/ghost/real_pasta_test.rs` は今も `TempPath::new`（108 行目）＝入口を替えると `target\test-roots\` の下へ移る。先に長さを確かめる。
- 優先度の区分: C のまま。
- 要件定義のモデル: Opus のまま。
- 分割の案: なし。
- 見つけた穴・古くなった記述: 棚卸㉖の節の数（81 ファイル・237 か所・`under_target` 29 ファイル）は上のとおり古い。見直しの `audit-test-kits`（v0.1.0 の後）が `temp-path-kit` を受け持つ＝本 spec が先に着地すれば、見直しは新しい形を読む（順は困らない）。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: 入口の部品 `crates/temp-path-kit/src/lib.rs`（148 行・OS の一時フォルダに作る `new` は 101 行目・`target\` の下に作る `under_target` は 115 行目）・`lib_tests.rs`（165）・見張り `crates/log-capture-kit/tests/temp_path_guard_test.rs`（783）は 10-22 の後も変わらない。`TempPath::new` の呼び出しは 82 ファイル・238 か所→**79 ファイル・234 か所**（`areka` 41・`areka-ghost` 23・`areka-emo-present` 6・`areka-parsers` 4・`temp-path-kit` 2・`areka-sylphya` 1・`areka-impl-watch` 1・`log-capture-kit` 1）。`under_target` の名前が出るファイルは 62→**76**（C10 の新しいテストが `target\` の下を使った）。
- **触るファイル**（入口の中身だけ替える案）: 上の 3 本だけ。234 か所は書き換えずに一度に動く。例外表の 19 ファイル（直に OS の一時フォルダを引く所）まで直すなら、棚卸㉖の節のとおり足される。
- **規模**: 入口を替える案で 4〜7（例外表まで入れると 8〜11）のまま。
- **先に要るもの**: 無い（先頭）。v0.1.0 との重なり: 0（上の 3 本を挙げる v0.1.0 の brief は無い）。1 本だけの席は取らない。後の版では見直しの `audit-test-kits` が `temp-path-kit` を受け持つ（先にこの spec が着地しても困らない）。
- 机・待ち: 静かな机・前後の全体テストの比べ・長い負荷の走行は要らない（最後の全体テスト 1 回）。ただし `areka-test-binary-stability` を静かな机で測っている間は着地させない（全部のテストの一時の根の置き場が動き、測りの基準が変わる）＝隙間に入れるのは、その測りと時期が重ならないウェーブだけ。
- 気をつけること: パスが長くなる。実物の `pasta.dll` を読む `crates/areka-ghost/tests/ghost/real_pasta_test.rs` は今も 108 行目で `TempPath::new`＝入口を替えると `target\test-roots\` の下へ移る。ワークツリーの名前の長さで 50 字前後のびるので、先に長さを確かめる。
- **優先度の区分**: C のまま（開発の決まり「一時フォルダは `target\` の下だけ」による持ち越し）。
- **要件定義のモデル**: Opus のまま。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 棚卸㉘の節の数（82 ファイル・238 か所・`under_target` 62 ファイル）は上のとおり古い。
