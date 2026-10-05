# 負荷の下の再現の記録

spec: areka-P0-ghost-session-test-load-flake（要件 1.1・1.2・1.4・5.1・6.1〜6.5、design「LoadRepro」）

手順は `tools/load-flake.ps1`。出力はすべてワークツリーの `target\load-flake\<Label>-<日時>\` の下だけに置かれる（`conditions.txt`・`build.log`・`round-N.log`・`summary.txt`）。

## 1. 手順

最初の実行（タスク 1.3・1.4）の前に決めた値。直した後（タスク 5.1・6.1）も同じ値で回す。

### 机

| 項目 | 値 | 根拠 |
|---|---|---|
| 論理 CPU の数 | 22 | `[Environment]::ProcessorCount`（2026-10-06 に読んだ値） |
| 負荷の前の CPU | 回ごとに `conditions.txt` の `cpu_before_load` に残る | 2026-10-06 の試運転では、負荷を起こす前から平均 73% だった（他のセッションが並走している机）。直す前と後を比べるときは、この値が大きく違わないことを確かめる |

### 引数

| 引数 | 値 | 意味・選んだ理由 |
|---|---|---|
| `-Label` | `before`（1.4）／`after`（5.1） | 出力のフォルダの名前 |
| `-Rounds` | 5 | 設計の既定どおり |
| `-Burners` | 44 | 論理 CPU の数 × 2（設計の既定）。既定に任せず数で書くのは、机が変わっても同じ本数で回すため |
| `-Filter` | 指定しない（実行ファイルの全部） | 対象の族（切替・起こし直し・上書きのインストール・終了・spine・MCP の一部）は実行ファイルの中に散らばっている。また、同じ実行ファイルの中で多数のテストが同時に CPU を取り合うこと自体が疑いの 1 つなので、絞ると条件が変わる |
| `-TestThreads` | 指定しない（libtest の既定＝論理 CPU の数） | 起票時に赤が出た条件のまま |
| `-RoundTimeoutMin` | 30 | 設計の既定どおり。越えた回は「上限越え」として残る（その回のテストのプロセスの木だけを止める） |
| `-NoCapture` | 付ける | 下の「`-NoCapture` を付ける理由」 |

1 回に回す範囲は `cargo test -p areka --bin areka -- --nocapture`（2026-10-06 の時点で 2,673 本）。

### コマンド

直す前（タスク 1.4）:

```
pwsh -NoProfile -File tools/load-flake.ps1 -Label before -Rounds 5 -Burners 44 -RoundTimeoutMin 30 -NoCapture
```

直した後（タスク 5.1）は `-Label after` だけを替える。

静かな机の所要時間（タスク 1.3・6.1。負荷なし・3 回）も `-NoCapture` で揃える。

```
pwsh -NoProfile -File tools/load-flake.ps1 -Label quiet-before -Rounds 3 -Burners 0 -RoundTimeoutMin 30 -NoCapture
```

### `-NoCapture` を付ける理由

- libtest は、緑で終わったテストの標準出力と標準エラーを捨てる。作業フォルダの後片付けの失敗（`sample-ghost-kit:` で始まる行・`devroot.rs` の `report_cleanup`）は標準エラーへ出すだけなので、`--nocapture` が無いと、赤になったテストの分しか `round-N.log` に残らない。要件 5.1 は赤に依らず os error 5 の起きた数を数えるので、付ける。
- 付けても赤のテストの名前は取れる。libtest は末尾の `failures:` の一覧を必ず出し、`load-flake.ps1` の `Read-Round` はそこから名前を拾う。違いは、失敗の文言が `---- 名前 stdout ----` の囲みでなく、その場の `panicked at` の行として流れの中に出ることだけ（どのテストの文言かは `thread '名前' panicked at` の名前で読む）。
- 付けると、テストが直す前から出している標準エラー（待ちの部品が直った後の `待ちの打ち切り` の行も含む）が全部 `round-N.log` に入る。直す前と後で同じく付けるので、条件は揃う。
- `conditions.txt` の `nocapture: yes` と `command:` の行で、付けて回したことが確かめられる。

確かめ（2026-10-06）: `pwsh -NoProfile -File tools/load-flake.ps1 -Label smoke-nocapture -Rounds 1 -Burners 0 -NoCapture -Filter alert_tests::root_missing_logs_one_error_with_absolute_root` が終了コード 0 で終わり、`conditions.txt` に `nocapture: yes` と `command: cargo test -p areka --bin areka -- "alert_tests::root_missing_logs_one_error_with_absolute_root" --nocapture` が載った。赤の名前の取り出しは、`--nocapture` の形の出力（その場の `panicked at` の行・`sample-ghost-kit: … (os error 5)` の行・空の囲みの `failures:` に続く名前の一覧）を `Read-Round` に読ませ、赤 1 件の名前・後片付けの行 1 件・os error 5 が 1 件と数えられることを確かめた。

### 回す前に確かめること

- `areka-test-threads-av` の測定と同時に走らせない（同じテストの実行ファイルを測る）。
- 同じワークツリーで別の `cargo test` を同時に走らせない（作業フォルダの棚 `target\nar-samples\work` を共有するため、os error 5 の読みが変わる。下の 2.4）。

### 対照（任意）

`--test-threads=1` で緑になるか（同じ実行ファイルの中の取り合いの疑い）を見るときだけ、同じ引数に `-TestThreads 1 -Label before-threads1` を足して回す。1 回は数時間かかりうるので `-RoundTimeoutMin` は延ばしてよいが、対照の結果は「直す前と後」の比較には使わない。

### 読み方の注意（直す前の記録）

直す前の待ちの部品は、打ち切られても文言を出さない（`spin_wait_until` は `false` を返すだけ）。今のコードにその文言を出す所は無いので、直す前の `summary.txt` の「`待ちの打ち切り` の行」は 0 件になる。直す前の赤で「その時に待っていた部品と締切」は、赤のテストの名前と `panicked at` の行の場所から、下の 2.1 の表で引く。

## 2. 数え上げ

2026-10-06 に `git grep -w` でワークスペース全体（本番のファイルを含む）を数えた。数えた時点のコミットは `95e8c4ab`。

### 2.1 待ちの部品を使うファイル

数えた語: `spin_wait_until`・`SPIN_WAIT`・`run_bounded`・`join_bounded`・`wait_steady`・`pump_until`・`pump_talking_until`・`pump_input_until`・`run_input_until`（テストのファイルが自前で持つ締切つきの待ちの名前）。加えて、テストのファイルが自前で持つ締切つきの待ちを、areka の crate の全部（`src` と `tests`）で `git grep -lE 'recv_timeout|Instant::now\(\) *\+|wait_timeout_while|wait_timeout\('` で探し、上の語の結果と突き合わせた。

結果: areka の crate で 45 ファイル（語で当たった 39 と、自前の締切だけで当たった 6）、他の crate で 51 ファイル。

自前の締切の検索は本番のファイルにも当たる（`emo2_boot/balloon_visibility_phase.rs`・`exit_wait.rs`・`perf_thread_report.rs`）が、これらは本番の処理の待ちで待ちの部品の利用ではないので、数えに入れない。

#### areka の crate（45 ファイル）

「足場経由」は、切替の足場 `SwitchRig` の待ち（`pump_until`・`pump_talking_until`・`pump_input_until`・`wait_steady`・`shutdown`）を呼ぶだけで、足場の直し（タスク 3.2）で待ち方が替わり、ファイルそのものは触らないもの。

| ファイル | 使う部品（回数） | 扱い |
|---|---|---|
| `emo2_boot/spine.rs` | 定義: `SPIN_WAIT`（30 秒）・`spin_wait_until`・`run_bounded`・`join_bounded`。`SpineHarness::shutdown_bounded` が `run_bounded`（10 秒）と `join_bounded`（10 秒） | 対象。部品を `spine_wait.rs` へ移す（2.1）・降ろしの待ちを目印つきへ（3.1） |
| `emo2_boot/ghost_switch_test_support.rs` | 定義: `SwitchRig::pump_until`・`pump_talking_until`・`pump_input_until`（中身は `spin_wait_until`）・`wait_steady`（`recv_timeout` 20 秒）・`shutdown`（`run_bounded` 20 秒） | 対象。足場の待ちを目印つきへ（3.2） |
| `emo2_boot/frame_ghost_quit_switch_tests.rs` | 自前の `wait_steady`（`recv_timeout` 20 秒）・`pump_until` | 対象。自前の待ちを芯へ寄せる（4.1） |
| `ghost_session_restart_tests.rs` | 自前の `run_input_until`（10 秒・空回しだけ）・`run_bounded` 20 秒（`shutdown_bounded`） | 対象。自前の待ちを芯へ寄せる（4.2） |
| `ghost_session_switch_fallback_tests.rs` | `spin_wait_until` を直接 2 か所（うち 1 か所は自前の `run_input_until` の中）・`wait_steady`・`pump_until`・`pump_talking_until` | 対象。直接の呼び出しを移す（4.3・下の 2.3） |
| `session_end_sync_send_tests.rs` | `spin_wait_until` を直接 2 か所・`run_bounded`（`WHOLE` 120 秒）・`wait_steady` | 対象。直接の呼び出しを移す（4.3） |
| `shell_balloon_switch_session_tests.rs` | `spin_wait_until` を直接 2 か所・`wait_steady` | 対象。直接の呼び出しを移す（4.3） |
| `shell_balloon_switch_session_lap_tests.rs` | `spin_wait_until` を直接 1 か所（`LapRig::frames_until`）・`wait_steady` | 対象。直接の呼び出しを移す（4.3） |
| `boot_shell_tests.rs` | `pump_until` | 足場経由 |
| `emo2_boot/ghost_switch_boot_event_tests.rs` | `wait_steady`・`pump_talking_until` | 足場経由 |
| `ghost_session_switch_memory_tests.rs` | `wait_steady`・`pump_talking_until` | 足場経由 |
| `ghost_session_switch_tests.rs` | `wait_steady`・`pump_until`・`pump_talking_until` | 足場経由 |
| `ghost_session_switch_translate_tests.rs` | `wait_steady`・`pump_talking_until` | 足場経由 |
| `install/desk_overwrite_tests.rs` | `wait_steady`・`pump_talking_until`・`pump_input_until` | 足場経由 |
| `install/worker_tests.rs` | `wait_steady`・`pump_input_until` | 足場経由 |
| `main_session_mark_tests.rs` | `pump_until` | 足場経由 |
| `session_end_tests.rs` | `wait_steady` | 足場経由 |
| `shell_balloon_switch_session_abort_tests.rs` | `wait_steady`（ほかに `LapRig::frames_until` を呼ぶ） | 足場経由（`frames_until` は lap のファイルで移す） |
| `shell_balloon_switch_session_balloon_tests.rs` | `wait_steady`（同上） | 足場経由 |
| `shell_balloon_switch_session_update_tests.rs` | `wait_steady`（同上） | 足場経由 |
| `update/desk_reload_tests.rs` | `wait_steady`・`pump_talking_until`・`pump_input_until` | 足場経由 |
| `update/worker_path_tests.rs` | `wait_steady`・`pump_talking_until`・`pump_input_until` | 足場経由 |
| `mcp/get_expression_table_tests.rs` | `wait_steady`。ほかに自前の `recv_timeout` 20 秒（kanade への照会の返事） | 足場経由。自前の 20 秒は対象外（赤の観測が無い・Non-Goals） |
| `session_end_deadline_tests.rs` | `spin_wait_until` を直接 4 か所・`wait_steady` | `wait_steady` は足場経由。直接の 4 か所は対象外（下の 2.3 の「移さない呼び出し」） |
| `mcp/dump_surface.rs` | `spin_wait_until` を 1 か所（本番のファイル。`#[cfg(test)]` の `WaitAnswer::wait_answer`） | 対象外（design の Out of Boundary。古い呼び名の形を保つので呼び手は触らない） |
| `mcp/dump_surface_gpu_test_support.rs` | `spin_wait_until` を 1 か所（`GpuRig::frames_until`） | 既定では対象外（下の 2.3） |
| `emo2_boot/spine_boot_smoke_tests.rs` | `spin_wait_until` を 1 か所 | spine の族。2.1 の例外（ファイルは触らない） |
| `emo2_boot/spine_close_wiring_tests.rs` | `spin_wait_until` を 1 か所・`SPIN_WAIT` の自前の締切 2 か所 | 同上 |
| `emo2_boot/spine_conformance_support.rs` | `SPIN_WAIT` の自前の締切（偽の時計つき）。`spin_wait_until` は兄弟のテストへ渡す取り込みだけ | 同上 |
| `emo2_boot/spine_conformance_support_tests.rs` | `spin_wait_until` を 4 か所・`SPIN_WAIT` | 同上 |
| `emo2_boot/spine_display_tests.rs` | `SPIN_WAIT` の自前の締切 | 同上 |
| `emo2_boot/spine_hold_support.rs` | `SPIN_WAIT`（固まりの解き忘れの上限・`wait_timeout_while`） | 同上 |
| `emo2_boot/spine_hold_tests.rs` | `spin_wait_until` を 1 か所 | 同上 |
| `emo2_boot/spine_move_cue_tests.rs` | `SPIN_WAIT` の自前の締切 | 同上 |
| `emo2_boot/spine_seriko_loop_tests.rs` | `spin_wait_until` を 1 か所・`SPIN_WAIT` の自前の締切 4 か所 | 同上 |
| `emo2_boot/spine_settle_tests.rs` | `SPIN_WAIT`（`settle_bounded` の檻の前提） | 同上 |
| `emo2_boot/spine_talk_close_tests.rs` | `spin_wait_until` を 1 か所・`SPIN_WAIT` の自前の締切 2 か所・`run_bounded` 10 秒・`join_bounded` 10 秒 | 同上（`join_bounded("spine s5 seriko join", …)` も 2.1 の例外） |
| `install/desk_pick_tests.rs` | 自前の `join_bounded`（`recv_timeout` 10 秒） | 対象外（`install/` の自前の `join_bounded`・赤の観測が無い） |
| `install/fetch_url_tests.rs` | 自前の `join_bounded`（`recv_timeout` 30 秒） | 同上 |
| `emo2_boot/frame/switch_tests.rs` | 自前の締切: `seriko_send_failure_empties_the_slot` が、seriko のアクターが閉鎖の便りを処理して終わるまで `Instant::now() + 5 秒` で `yield_now` だけの空回し（休みなし）。表の中でいちばん短い締切 | 切替の族。design の「条件つき（再現で赤になったファイルだけ）」のファイルで、既定では触らない。赤になればタスク 5.4 で芯へ寄せる。空回しだけで待つ形は 2.5 に反する形なので、1.4 の記録でこのテストの赤を特に見る |
| `emo2_boot/frame_ghost_quit_logsink_tests.rs` | 自前の締切: `connect_failure_reaches_quit_app_without_real_sink_wiring` が、停止の知らせが終了相へ届くまで `BOUND`（30 秒）で `run_ghost_quit_phase` を回す（5 ms ずつ休む） | 終了と切替の予約の族の隣だが、design の対象の一覧には無い。既定では触らない。赤になればタスク 5.4 |
| `emo2_boot/install_cue_tests.rs` | 自前の締切: `url_arm_starts_one_fetch_and_one_script_request_arrives` が、依頼が届くのを `recv_timeout(BOUND)`（30 秒）で待つ | 対象外（インストールの合図の族・赤の観測が無い）。赤になればタスク 5.4 |
| `thread_roles_tests.rs` | 自前の締切: `wait_for_role` が、スレッドの名簿に役割名が現れるまで呼び手の `limit`（5 秒）で待つ（5 ms ずつ休む） | 対象外（スレッドの名簿の族・赤の観測が無い）。赤になればタスク 5.4 |
| `menu/trigger_show_tests.rs` | `Instant::now() + 60 秒` を、`a_ready_plan_without_the_outer_world_reference_is_dropped_with_one_warning` がメニューの返事待ちの締切の値として本番の型へ渡す | 対象外。テストは待たない（締切は本番の返事待ちの値で、`poll_menu_query` を 1 回呼ぶだけ） |
| `menu/trigger_tests.rs` | `Instant::now() + QUERY_TIMEOUT`（1 秒）を、`in_flight_guard_lowers_the_flag_when_the_pending_query_is_dropped` が `PendingQuery::new` へ渡す | 対象外。同上（ほかのテストは基点の時刻を差し替えて判定し、実時間を待たない） |

「spine の族」の自前の締切（`Instant::now() + SPIN_WAIT` の繰り返し）は Tick を注入する待ちで、design の Non-Goals と「2.1 の例外」に当たる。`SPIN_WAIT` の名前と値は `spine_wait.rs` から出し直されるので、これらのファイルは触らない。

#### 他の crate（51 ファイル・すべて対象外）

どれも areka の部品とは別物の、同じ名前の自前の `run_bounded`／`join_bounded` か、コメントの中の名指しだけ（requirements の Out of scope・design の Non-Goals）。

| crate | ファイル | 中身 |
|---|---|---|
| `areka-actor`（1） | `src/spawn.rs` | 本番のファイルの中のテストのモジュールの自前の `run_bounded` |
| `areka-ghost`（17） | `src/relay.rs`（本番のファイルの中のテストのモジュール）・`src/dispatcher_choice_tests.rs`・`src/dispatcher_choice_timeout_tests.rs`・`src/dispatcher_slot_tests.rs`・`src/dispatcher_test_support.rs`・`src/runtime_tests.rs`・`src/ticker_tests.rs`・`tests/ghost/inproc_e2e_test.rs`・`tests/ghost/real_pasta_test.rs`・`tests/ghost/spine_e2e_test_s1_boot_success.rs`・`…_s2_connect_failure.rs`・`…_s3_helper_liveness_detected.rs`・`…_s4_close_handshake.rs`・`…_s5_close_deadline.rs`・`…_s6_full_disconnect.rs`・`…_s7_second_boot_record_present.rs`・`tests/ghost/sylphya_integration_test.rs` | 自前の `run_bounded`／`join_bounded`。s1・s3 の `spin_wait_until` はコメントの中の名指しだけ |
| `areka-kanade`（33） | `src/actor_tests.rs` と、`tests/kanade/` の `common/`（`common_bounded.rs`・`common_mock_sakura.rs`・`common_smoke.rs`・`common_window_actor.rs`・`mod.rs`）・`boot_test.rs`・`choice_test*.rs`（8）・`close_test*.rs`（4）・`external_status_test.rs`・`failure_test.rs`・`full_run_test.rs`・`idle_pump_test.rs`・`mouse_test*.rs`（5）・`prefetch_test.rs`・`real_helper_test.rs`・`resource_query_test.rs`・`steady_test.rs`・`translate_test.rs` | 自前の `run_bounded`／`join_bounded`（`common_bounded.rs` の定義を共有） |

`wait_steady`・`pump_until`・`pump_talking_until`・`pump_input_until` を語として使うファイルは、areka の crate の外には無い（`areka-emo-text`・`shiori-host32-host` などに当たるのは `pump_until_idle` などの別の名前）。

### 2.2 起票時の数との差

| 数えたもの | 起票時 | 今 | 差 |
|---|---|---|---|
| 切替の足場 `SwitchRig` を名指しするファイル（定義の `ghost_switch_test_support.rs` を除く） | 28 | 28 | 0 |
| `spin_wait_until` か `SPIN_WAIT` を名指しするファイル（areka の crate） | 20 | 20 | 0 |

差は無い。補足:

- 足場を名指しせずに使うファイルが 2 つある。`mcp/dump_surface_gpu_tests.rs` と `mcp/dump_balloon_gpu_tests.rs` は `GpuRig`（`mcp/dump_surface_gpu_test_support.rs`）の中の `SwitchRig` を使う。間接の利用を含めると 30。
- `spin_wait_until` か `SPIN_WAIT` をワークスペース全体（`*.rs`）で語として探すと 22 ファイル（`spin_wait_until` だけなら 18 ＝ areka 16・areka-ghost 2）。areka の 20 から増えた 2 つは `areka-ghost` の `spine_e2e_test_s1_boot_success.rs`・`spine_e2e_test_s3_helper_liveness_detected.rs` で、どちらもコメントの中の名指しだけ（呼び出しは無い）。
- 1.4 の要件の部品（`run_bounded`・`join_bounded`・`wait_steady`・`pump_*`・自前の待ち）まで広げると、areka の crate で 45 ファイル（上の 2.1）。

### 2.3 直接の `spin_wait_until` のうち移すもの（移す先の一覧）

対象の族のファイルで `spin_wait_until` を直接呼び、足場か偽の SHIORI の観測口（`ScriptedShioriHandle`）が手元にある呼び出し。取り直した数は 7 か所・4 ファイルで、設計の時点の数と同じ。

移す先の決まり（design「SwitchRig の待ち」）:

- 足場があり、条件が足場を読むだけなら `SwitchRig::wait_for`（`&self` で借りる）。
- 足場があっても、条件の中で足場を書き換える（`rig.world.run_schedule(…)` を呼ぶ）呼び出しは、`wait_for(&self, …)` では借用が重なる。先に `SwitchRig::progress_probe()`（World を借りない目印の関数）を取ってから、`wait_until(呼び出しの場所, Progress::Count(&probe), 条件)` を呼ぶ。
- 足場を持ち込めない別のスレッドの中の待ちは、`wait_until(…, Progress::Count(&|| handle.call_count()), …)`（`ScriptedShioriHandle::call_count`）。

| # | ファイル | 呼んでいる関数（テスト） | 待っているもの | 移す先 |
|---|---|---|---|---|
| 1 | `ghost_session_switch_fallback_tests.rs` | 自前の `run_input_until`（呼び手は `sync_target_boot_failure_leaves_only_default_windows` が 2 回・`async_target_fault_before_window_closure_leaves_only_default_windows` が 1 回） | `Input` の段を回しながら、作業プールの閉包が届いて窓が生えること（`window_count(w) > 0`） | 足場あり・条件が足場を書き換える → `progress_probe()` を先に取り `wait_until(…, Progress::Count(&probe), …)` |
| 2 | `ghost_session_switch_fallback_tests.rs` | `async_target_fault_before_window_closure_leaves_only_default_windows` | 作業プール `WintfTaskPool` に命令が 2 件溜まること（`drain_commands` で集める） | 足場あり・条件は足場を読むだけ → `SwitchRig::wait_for` |
| 3 | `session_end_sync_send_tests.rs` | `reproduce`（呼び手は `teardown_join_does_not_wait_for_a_sync_send_to_the_ui_window`） | 送り手の同期の送信が UI 役のキューに届くこと（`send_pending`） | 足場あり・条件は足場を読まない → `SwitchRig::wait_for` |
| 4 | `session_end_sync_send_tests.rs` | `reproduce` の中の解き手のスレッド（同上のテスト） | 送信の旗が立ち、偽の SHIORI が `OnClose` の通知で固まったこと（`handle.holding()`） | 別のスレッド → `wait_until(…, Progress::Count(&|| handle.call_count()), …)` |
| 5 | `shell_balloon_switch_session_tests.rs` | `menu_shell_request_raises_on_shell_changing_with_the_current_shell_as_ref1` | 偽の SHIORI が `OnShellChanging` を受けたこと | 足場あり → `SwitchRig::wait_for` |
| 6 | `shell_balloon_switch_session_tests.rs` | `menu_shell_frame_marks_the_mounted_shell_and_selecting_requests_a_switch` | 同上 | 足場あり → `SwitchRig::wait_for` |
| 7 | `shell_balloon_switch_session_lap_tests.rs` | `LapRig::frames_until`（呼び手は同じファイルの `script_shell_switch_round_trips_with_raise_event`・`plain_script_shell_switch_skips_changing_and_waits_for_the_script_end`・`menu_switch_to_the_current_shell_rebuilds_and_raises_both_events` と、`shell_balloon_switch_session_abort_tests.rs`・`…_balloon_tests.rs`・`…_update_tests.rs`） | 合成の Tick を送り、`Input`・`Update` の段を回しながら、呼び手の条件 | 足場あり・条件が足場を書き換える → `progress_probe()` を先に取り `wait_until(…, Progress::Count(&probe), …)`。この関数は dispatcher へ合成の時刻を送るので、送り方は足場の時刻の注入（タスク 3.3）と同じ決まり（送り先は `DispatcherMsg::Tick` だけ）に従う |

#### 移さない直接の呼び出し

| ファイル | 呼んでいる関数 | 移さない理由 |
|---|---|---|
| `mcp/dump_surface.rs` | `WaitAnswer::wait_answer`（`#[cfg(test)]`） | design の Out of Boundary（呼び手は変えない）。古い呼び名の形を保つので、打ち切りの文言だけが新しくなる |
| `mcp/dump_surface_gpu_test_support.rs` | `GpuRig::frames_until`（呼び手は `dump_surface_gpu_tests.rs`・`dump_balloon_gpu_tests.rs`） | 足場（`GpuRig` の中の `SwitchRig`）は手元にあるが、MCP の描画の族で、設計の対象の族に入っていない。形は `LapRig::frames_until` と同じなので、再現でこの族が赤になったら同じ移す先を使う（タスク 5.4） |
| `session_end_deadline_tests.rs` | `boot_a`・`cut_when_held`・`unbounded_shutdown_never_cuts`（2 か所） | このファイルは本番の実時間の締切そのものを確かめる（design の Non-Goals）。待っているのは偽の SHIORI の固まり（`holding`）と予約の発火で、赤の観測も無い |
| `emo2_boot/spine_*`（`spine_boot_smoke_tests.rs` 1・`spine_close_wiring_tests.rs` 1・`spine_conformance_support_tests.rs` 4・`spine_hold_tests.rs` 1・`spine_seriko_loop_tests.rs` 1・`spine_talk_close_tests.rs` 1） | 各テストの関数 | design「2.1 の例外」。待つ相手が描画と台詞の再生で、SHIORI の呼び出しを伴わない。spine の族の赤は `SpineHarness::shutdown_bounded`（10 秒）の側で、それはタスク 3.1 で目印つきになる |
| `emo2_boot/ghost_switch_test_support.rs` | `SwitchRig::pump_until`・`pump_talking_until`・`pump_input_until` | 足場の定義そのもの。タスク 3.2 で芯へ替える |

### 2.4 サンプルゴーストの展開の作業フォルダ（要件 5.1）

#### 作る・使う・退避する入口（`crates/sample-ghost-kit/src/`）

窓口 `SampleRoot::acquire` は `lib.rs` に、その先の `fresh_root` から後（`sweep`・`discard_tree`・`WorkDir` など）は `devroot.rs` にある。

棚は `<target>\nar-samples\work`（このワークツリーでは `target\nar-samples\work`）。同じワークツリーのテストのプロセスは、どの crate のものでもこの 1 つの棚を共有する。

| 入口 | 棚に対してすること |
|---|---|
| `SampleRoot::acquire`（→ `fresh_root` → `fresh_root_in`） | ① `sweep`: 棚の札を 1 つずつ消してみて、消せた札の木を `discard_tree` で `gc-<番号>-<連番>` へ移してから消す（退避）。札の無い木（`gc-` の木を含む）も退避する ② `cached_root_in`: 原本が無ければ `WorkDir` で組んで `cache\` へ移す（`stage_into_cache`）。刻印の違う古い原本は `reclaim_stale` → `discard_tree` で退避 ③ `WorkDir` を 1 つ作って原本を複写して配る |
| `WorkDir::new` | 札を先に作ってから木を作る。掃除（`sweep`）はしない |
| `impl Drop for WorkDir` | 札を閉じる → 木を消す（`remove_dir_all`）→ 札を消す |
| `LazyLock` に入れた `SampleRoot` | プロセスの終わりまで捨てない（`Drop` が走らない）。木と札はプロセスが終わった後に残り、次のプロセスの `sweep` が退避する |

#### 使うテストの数

関数の呼び出しを、テストの関数から入口まで定義の名前でたどって数えた（`SwitchRig::new`・`SpineHarness::boot`／`boot_live`／`boot_with`・`acquire_emo2`・`LapRig` と `GpuRig` の作り口などを経由するもの）。たどりは字面の照合なので、`use super::*` などの取り込み越しの呼び出しを取りこぼしうる（少なめに出る向き）。「作って捨てる」はテストごとに複製を作り、終わりで `Drop` が木を消すもの。「共有の 1 つ」はプロセスに 1 つの `LazyLock` の複製を読むだけのもの。

| crate（テストのプロセス） | 作って捨てるテスト | 共有の 1 つだけ使うテスト | 入口 |
|---|---|---|---|
| `areka`（`--bin areka`・結合テストを含む） | 約 143 | 約 124 | ほぼ全部が `SampleRoot::acquire`（掃除あり）。`install/terms_tests.rs` の 16 本だけ `WorkDir::new`（掃除なし）。結合テスト `tests/smoke_boot_loop_exit.rs`（3）・`tests/mcp_get_log_real_run.rs`（1）は別のプロセス |
| `areka-nar` | 約 72 | 0 | `WorkDir::new`（掃除なし） |
| `areka-update` | 約 70 | 0 | `WorkDir::new`（掃除なし）。`winhttp_real_tests.rs` の 1 本だけ `SampleRoot::acquire` を 2 回 |
| `sample-ghost-kit` | 約 32 | 0 | 私有の名前空間（`devroot_cache_tests.rs` の関数 `private_namespace` が返す、棚の上の `WorkDir`）の中で `fresh_root_in` などを呼ぶ。私有の中の掃除は共有の棚に届かないが、私有の名前空間そのものは共有の棚の木 |
| `areka-ghost` | 約 6 | 0 | `SampleRoot::acquire` |
| `areka-seriko` | 約 4 | 約 7 | `SampleRoot::acquire` |
| `areka-emo-text` | 0 | 約 55 | `LazyLock` |
| `areka-emo-compose` | 0 | 約 22 | `LazyLock` |
| `areka-emo-present` | 0 | 約 13 | `LazyLock` |
| `areka-parsers` | 0 | 約 12 | `LazyLock` |
| `areka-emo-atlas` | 0 | 約 11 | `LazyLock` |
| `pilot`（例の中のテスト） | 0 | 約 1 | `LazyLock` |
| 合計 | 約 327 | 約 245 | |

`--bin areka` の中だけで、テストごとに複製を作って捨てるテストが 100 本を越え、その全部が取得のたびに棚を掃除する。再現の手順（`--bin areka` の全部）は、この 1 つのプロセスの中の同時の走行を必ず含む。

#### os error 5 が起きうる同時の走行の組

Windows は、中のファイルが開かれているフォルダの `rename` を拒み（`devroot.rs` の `discard_tree` の doc の実測）、消している最中のフォルダへの操作は拒否（os error 5）になりうる。下の組は、どれも design「WorkDirCleanup」の見分けの表の行に当たる。

| # | 組 | 起きる窓 | 標準エラーの行が指す木 | 見分けの表の行 |
|---|---|---|---|---|
| A | 同じプロセスの中: テスト X の `Drop for WorkDir` と、同時に `SampleRoot::acquire` を呼んだテスト Y の `sweep`（`--bin areka` では `SwitchRig::new`・`SpineHarness::boot*` が取得のたびに起こす） | X は札を閉じてから木を消す。消している間、札は誰にも握られていないので、Y の `sweep` が札を消せて（`owner_is_gone`）、消している最中の X の木を `discard_tree` で移そうとする。`rename` が拒まれると Y の「残骸の退避」の行、X の側の `remove_dir_all` が拒まれると X の「作業フォルダの後始末」の行 | `work\<番号>-<連番>` | 1（`Drop` の順） |
| B | 同じプロセスの中: `discard_tree` で `gc-…` を消しているテスト X と、同時に `sweep` するテスト Y | `gc-` の木には札が無いので、Y の `sweep` は「札の無い残骸」として同じ木を別の `gc-` へ移そうとする。拒まれると Y の「残骸の退避」の行、X の側は「退避した残骸の削除」の行 | `work\gc-…` | 2（`gc-` の札） |
| C | 別々のプロセス: 同じワークツリーで同時に走る 2 つの `cargo test`（並べた `/kiro-impl` のレビューと全体テスト、など） | A・B と同じ窓が、プロセスをまたいで開く。`tools/test-all.ps1` の `cargo test --workspace` は 1 つの起動の中ではテストの実行ファイルを 1 つずつ順に回す（`-j 4` はビルドの並び）ので、全体テストの 1 回の中では起きない。起きるのは、同じ `target\` を使う別の起動と重なったときと、テストが起こした子のプロセスと重なったとき | A・B と同じ | 1・2 |
| D | 前のプロセスの残り: `LazyLock` の複製（木と、閉じた札）が前のプロセスの終わりに残り、次のプロセスの最初の `sweep` が退避する | 退避する木のファイルを誰かが開いている（ウイルス対策の検査など）と `rename` が拒まれる。同時に走るテストの組ではない | `work\<番号>-<連番>` で、その番号のプロセスはもう居ない | 3（テストの側の原因ではない） |

負荷はこの窓を広げる向きに働く（`remove_dir_all` が長くかかるほど、A・B の窓が長く開く）。

#### 同じ回の赤との関わり

- 後片付けの失敗は、今は `report_cleanup` が標準エラーへ 1 行出すだけで、赤にしない（取得も破棄も続く）。
- 上の A・B で退避されそうになるのは、札が閉じた後の木（持ち主のテストはもう破棄の最中）か、札の無い `gc-` の木なので、生きているテストの複製を横から消すことはない（生きている間は札が削除を拒む）。だから、退避の失敗が同じ回の他のテストを直接赤にする経路は、静的には見当たらない。関わりうるのは、ディスクの読み書きの負荷が増えることと、消し残しの木が棚に溜まることだけ。
- 行には木のパス（プロセスの番号と連番）しか出ず、テストの名前は出ない。`-NoCapture` の出力では行が流れの中に混ざるので、赤との関わりは「同じ回か」「赤のテストの `panicked at` の行と前後しているか」でしか並べられない。タスク 1.4 は、回ごとの赤と後片付けの行を並べて、この見立てに反する並び（後片付けの行の直後に、同じ木を使うテストの赤）が無いかを読む。

## 3. 直す前の記録

タスク 1.4 で書く（`-Label before` の回ごとの赤・文言・待っていた部品と締切・後片付けの行と赤との関わり）。

## 4. 直した後の記録

タスク 5.1 で書く（`-Label after` を同じ引数で回した結果）。

## 5. 所要時間

直す前はタスク 1.3、直した後はタスク 6.1 で書く（静かな机で `--bin areka` の全部を 3 回・全体テストを 1 回）。

## 6. 競合・os error 5 の結論・試して赤が 0 件だったこと

- 競合: タスク 5.3 で書く。
- os error 5 の結論: タスク 4.4 で書く（上の 2.4 の組と、1.4 の記録の行のパスを見分けの表に当てる）。
- 試して赤が 0 件だったこと: 1.4 で赤が 0 件のときに、タスク 1.4 と 5.1 で書く。
