# 較正記録: 直す前の構造で赤、直した後で緑

> **これは退行ではなく較正である。** 段階 1 のコミットは、意図して赤のテスト 6 本を含む。受信ループを直す前の構造でテストが赤になることを示し、段階 2（待ちの形の変更）で同じテストが緑になることで、テストが本当に欠陥を検出できることを確かめる（設計 C7・要件 4.7）。完了検証では、段階 1 の赤を退行と読まないこと。
>
> - 段階 1 の最終コミット: `193b1a09`（タスク 2.5）
> - 段階 1 の範囲: `7feb8db1`（タスク 1.1）〜`193b1a09`（タスク 2.5）。途中に実装メモの docs コミット `e33dfdce` を含む
> - squash マージ後は、これらのコミットは履歴から消える

## 1. 段階 1 の構造（受信ループは無改変）

段階 1 で入れたのは、手空き通知の契約（`ShioriBackend::on_idle`・既定は何もしない）、保守周期 `IDLE_INTERVAL`、`ShioriConnection::on_idle` の委譲、`pump_pending_messages`、テスト 6 本である。待ちの形は変えていない。

- `git diff 7feb8db1~1 193b1a09 -- crates/areka-kanade/src/shiori/real.rs` の差分は `use std::time::Duration;`、`on_idle` の既定実装、`IDLE_INTERVAL`、`ShioriConnection::on_idle`、`idle_tests` モジュール宣言の追加だけ。`run_shiori_loop` には触れていない
- `193b1a09` 時点の `run_shiori_loop` の受信は `while let Ok(msg) = rx.recv() {` のまま（無期限に待つ形）。したがって手空きの機会は一度も来ない

各テストコミットの本文には、赤が意図したものだと書いてある（`git log` から逐語で引用）。

| コミット | タスク | 本文 |
|---|---|---|
| `74d438f0` | 2.1 | 段階 1（受信ループ無改変）ゆえ Test-A・Test-C は意図して赤。 |
| `bf587920` | 2.2 | 段階 1（受信ループ無改変）ゆえ Test-B・Test-E は意図して赤。 |
| `1ef8dea7` | 2.4 | 段階 1（受信ループ無改変）ゆえ Test-D は意図して赤。 |
| `193b1a09` | 2.5 | 段階 1（受信ループ無改変）ゆえ Test-F は意図して赤。 |

## 2. 段階 1 の実行結果（赤）

- 実行日: 2026-09-17（JST）
- コミット: `193b1a099293d372795d9d465cce5c4e21dd7c0b`（作業ツリーに変更なし）
- 環境: Windows 11 Pro 10.0.26200・x64・debug ビルド
- 2 つのコマンドは並行させず、1 つずつ順に走らせた

### 2.1 窓を作らないテスト（Test-A・B・C・E）

- コマンド: `cargo test -p areka-kanade --lib idle_tests`
- 開始 20:26:56 → 終了 20:27:19（壁時計で約 23 秒。うちビルド 17.94 秒）
- exit code: **101**
- 結果: `test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 286 filtered out; finished in 2.25s`

| テスト | 位置づけ | 赤の診断文（逐語） |
|---|---|---|
| Test-A `shiori::real::idle_tests::on_idle_is_called_while_idle` | **較正対象**（手空きの契約） | `何も送らずに 2.0032731s 待ったが手空きの通知が届かなかった（観測した手空き回数=0・上限=2s・保守周期=500ms）` |
| Test-B `shiori::real::idle_tests::helper_exit_during_idle_reports_shiori_down_once` | **較正対象**（手空き中の死活報告・要件 4.10） | `helper を異常終了させ何も送らずに 2.0016158s 待ったが ShioriDown が届かなかった（受信結果=Err(Timeout)・観測した手空き回数=0・上限=2s・保守周期=500ms）` |
| Test-C `shiori::real::idle_tests::requests_after_idle_are_served_in_order` | 較正対象ではない | `要求を送る前に手空きの証拠を 2.0102969s 待ったが届かなかった（観測した手空き回数=0・上限=2s）——手空きを挟んだ順序を主張できない` |
| Test-E `shiori::real::idle_tests::no_liveness_report_after_clean_unload_even_when_idle` | 較正対象ではない | `正規終了の後、手空きが 2 回回る証拠を 2.0058573s 待ったが得られなかった（idles_before=0・idles_after=0・ShioriDown=0 通・上限=2s）——報告が来ないことを主張できない` |

Test-C と Test-E は、テストが空振りで緑にならないよう「手空きが実際に起きた」証拠を待ってから本題を確かめる形になっている。段階 1 ではその証拠が来ないので赤になるが、これは本題の欠陥を検出した赤ではない。だから較正の対象には数えない（設計 C7）。

### 2.2 窓を作るテスト（Test-D・F）

- コマンド: `cargo test -p areka-kanade --test kanade idle_pump_test`
- 開始 20:27:29 → 終了 20:28:31（壁時計で約 62 秒。うちビルド 15.75 秒）
- exit code: **101**
- 結果: `test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 50 filtered out; finished in 45.04s`

| テスト | 位置づけ | 赤の診断文（逐語） |
|---|---|---|
| Test-D `idle_pump_test::sync_send_to_idle_window_returns_within_bound` | **較正対象**（速い窓・要件 4.3） | `待機中のホスト窓への同期送出が上限内に成功復帰する（要件 1.2）: delivered=false elapsed=1.999874s bound=2s uptime_lower_bound=2.049459s result=Err(SendFailed) shiori_down_first=None shiori_down_second=false total=2.0518148s` |
| Test-F `idle_pump_test::abortifhung_send_reaches_window_idle_for_twenty_seconds` | **較正対象**（遅い窓・要件 4.4） | `送出①が届く（20 秒の待機後・要件 1.3）: first=Err(SendFailed) first_elapsed=5.0008962s second=Err(SendFailed) second_elapsed=20.1µs idle=20s send_bound=5s uptime_lower_bound_at_first=20.0327284s uptime_lower_bound_at_second=45.0338219s send_failures=2 total=45.0358955s` |

読み方:

- Test-D: 送出が上限の 2 秒いっぱいまで待たされて `SendFailed` になった。すぐ終了する代わりのプロセスを起こしているのに `ShioriDown` も来ていない（`shiori_down_first=None`）。手空きの機会が一度も来ていないからである
- Test-F: 送出①は上限の 5 秒待たされて失敗した。送出②は 20.1µs で即座に失敗した。OS が窓を「応答なし」と判定した後に出る形そのものである
- `uptime_lower_bound*` はプロセス生存時間の**下限値**である。テストファイル内で最初に使ったときの時刻（`OnceLock<Instant>`）から数えている。kanade に Win32 API の crate を足さないため、本当のプロセス生存時間は取れない

### 2.3 較正として要求される 4 本

| 要件 | テスト | 段階 1 |
|---|---|---|
| 手空きの契約 | Test-A | 赤 |
| 手空き中の死活報告（4.10） | Test-B | 赤 |
| 速い窓（4.3） | Test-D | 赤 |
| 遅い窓（4.4） | Test-F | 赤 |

Test-C と Test-E も段階 1 では赤だが、上で書いたとおり較正の対象ではない。

### 2.4 摂動による確認（出所: 各タスクのレビュー時に再現）

「受信ループを最終形に一時的に変えると緑になる」「判断を壊すと赤に戻る」ことは、タスク 2.1〜2.5 の実装とレビューのときに一時変更で確かめてある（変更は手で元に戻し、`git diff --quiet crates/areka-kanade/src` が 0 であることを確認済み）。このタスクでは再実行していない。

## 3. 段階 2 の実行結果（緑）

タスク 4.3 で、段階 1 と同じ 2 つのコマンドを走らせた結果である。

- 実行日: 2026-09-17（JST）
- コミット: `5fa186b9c176339d2b6fcf5c01a8ab826ae73537`（タスク 4.2＝受信ループを「500 ms で待ちを打ち切り、手空きなら `on_idle` を呼ぶ」形へ変更した後。作業ツリーに変更なし）
- 環境: 段階 1 と同じ（Windows 11 Pro 10.0.26200・x64・debug ビルド）
- 2 つのコマンドは並行させず、1 つずつ順に走らせた
- 名前で絞っているので他のテストは filtered out になる。passed の数がそれぞれ 4 と 2 であること（対象が 0 本で緑になっていないこと）を確かめた

### 3.1 窓を作らないテスト（Test-A・B・C・E）

- コマンド: `cargo test -p areka-kanade --lib idle_tests`
- 開始 20:40:39 → 終了 20:40:41（壁時計で約 2 秒。ビルドは既に済んでおり 0.21 秒）
- exit code: **0**
- 結果: `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 286 filtered out; finished in 1.52s`

| テスト | 結果 |
|---|---|
| Test-A `shiori::real::idle_tests::on_idle_is_called_while_idle` | ok |
| Test-B `shiori::real::idle_tests::helper_exit_during_idle_reports_shiori_down_once` | ok |
| Test-C `shiori::real::idle_tests::requests_after_idle_are_served_in_order` | ok |
| Test-E `shiori::real::idle_tests::no_liveness_report_after_clean_unload_even_when_idle` | ok |

### 3.2 窓を作るテスト（Test-D・F）

- コマンド: `cargo test -p areka-kanade --test kanade idle_pump_test`
- 開始 20:40:46 → 終了 20:41:30（壁時計で約 44 秒。うちビルド 3.30 秒）
- exit code: **0**
- 結果: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 50 filtered out; finished in 40.12s`

| テスト | 結果 |
|---|---|
| Test-D `idle_pump_test::sync_send_to_idle_window_returns_within_bound` | ok |
| Test-F `idle_pump_test::abortifhung_send_reaches_window_idle_for_twenty_seconds` | ok |

段階 1 の 45.04 秒から 40.12 秒へ縮んだのは、Test-F の送出①が上限 5 秒まで待たされなくなったためである（2 本は並行に走るので全体の所要は最も長い Test-F で決まり、Test-F は 20 秒の待機 2 回が所要の大半を占める）。

### 3.3 段階 1 の赤 → 段階 2 の緑（6 本の対照）

| テスト | 段階 1（`193b1a09`） | 段階 1 の要旨 | 段階 2（`5fa186b9`） | 較正対象か |
|---|---|---|---|---|
| Test-A 手空きの通知が届く | 赤 | 2 秒待っても手空き回数 0 | 緑 | **対象**（手空きの契約） |
| Test-B 手空き中の helper 異常終了を 1 回報告 | 赤 | 2 秒待っても `ShioriDown` が来ない（手空き回数 0） | 緑 | **対象**（要件 4.10） |
| Test-C 手空きを挟んでも要求を順に処理 | 赤 | 手空きの証拠が来ず本題に進めない | 緑 | 対象外（前提の証拠待ちで赤） |
| Test-E 正規終了後は手空きでも報告しない | 赤 | 手空き 2 回の証拠が来ず本題に進めない | 緑 | 対象外（前提の証拠待ちで赤） |
| Test-D 待機中の窓への同期送出が上限内に成功 | 赤 | 上限 2 秒いっぱい待たされ `SendFailed` | 緑 | **対象**（速い窓・要件 4.3） |
| Test-F 20 秒待機を挟んだ打ち切り旗つき送出が届く | 赤 | 送出① 5 秒で失敗・送出② 20.1µs で即失敗 | 緑 | **対象**（遅い窓・要件 4.4） |

較正として要求される 4 本（Test-A・B・D・F）はすべて「段階 1 で赤 → 段階 2 で緑」になった。`git diff --stat 193b1a09 5fa186b9 -- crates/` で変わったのは `crates/areka-kanade/src/shiori/real.rs` の 1 ファイルだけ（タスク 4.1 の死活監視の括り出しと、タスク 4.2 の受信ループの変更）。テストのファイル（`real_idle_tests.rs` と統合テスト側）は変わっていない。同じテストが赤から緑へ変わったので、この 4 本は待ちの形の欠陥を検出できている（設計 C7・要件 4.7）。

## 4. 負荷下の 1 回計測（タスク 4.4）

- 2026-09-17 20:47（`8672777f`＋送出の所要を印字する一時の 1 行・計測後に撤去し `git diff --quiet crates/` exit 0）／負荷条件＝e2e task 3.1 の「16 並列の負荷」を CPU を回し続ける PowerShell 16 本（150 秒で自己終了・計測後に自分が起こした 16 本だけを停止）で再現、CPU 使用率 100%、ほかに別の作業ツリーの `cargo test --workspace --no-run` が並走（負荷の種類は推測: task 3.1 の記録は並列数のみで負荷の種類もコマンドも書いていないため、同 spec task 4.1 の「CPU 負荷」の形に倣った）／`cargo test -p areka-kanade --test kanade sync_send_to_idle_window_returns_within_bound -- --nocapture` 1 回＝`1 passed; 51 filtered out; finished in 2.14s`・送出の所要 `elapsed=508.35ms`（上限 `SEND_BOUND`＝2 秒に対し余裕 1.49 秒＝上限の約 75%・所要は上限の約 25%。約 500 ms は手空きの窓が次の保守周期 `IDLE_INTERVAL`＝500 ms まで取り出さないことによる設計上の待ちで、負荷が無くても最大でこの程度かかる。負荷なしの所要は計っていないので負荷による上乗せ分は切り分けていない）・死活報告ちょうど 1 通・テスト全体 2.11 秒

> **追記（最終検証の指摘・コミット d8762a9e）**: 上の Test-D の診断文にある `shiori_down_second` と §4 の「死活報告ちょうど 1 通」は、当時のテストが 1 通目受領後に固定の短い期限（1.5 秒）で 2 通目を待っていたときの記録である。この形は要件 4.8 が禁じる「短い期限で発火しない」に当たるため外した。現在の Test-D は「死活報告が届く」だけを主張し、2 通目が無いことは手空きが回った証拠つきで Test-B が見張る。赤→緑の判定に使った送出の assert（`delivered && elapsed < SEND_BOUND`）は変えていない。
