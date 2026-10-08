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

2026-10-06 に `git grep -w` でワークスペース全体（本番のファイルを含む）を数えた。数えた時点のコミットは `95e8c4ab`。2026-10-07 に main（`067375e3`・PR#249〜#261）を取り込んだ後（`47b66594`）に同じ検索で数え直し、増えた分を足した（取り込みで消えたファイルは 0）。

### 2.1 待ちの部品を使うファイル

数えた語: `spin_wait_until`・`SPIN_WAIT`・`run_bounded`・`join_bounded`・`wait_steady`・`pump_until`・`pump_talking_until`・`pump_input_until`・`run_input_until`（テストのファイルが自前で持つ締切つきの待ちの名前）。加えて、テストのファイルが自前で持つ締切つきの待ちを、areka の crate の全部（`src` と `tests`）で `git grep -lE 'recv_timeout|Instant::now\(\) *\+|wait_timeout_while|wait_timeout\('` で探し、上の語の結果と突き合わせた。

結果: areka の crate で 51 ファイル（語で当たった 43 と、自前の締切だけで当たった 8）、他の crate で 52 ファイル。取り込み前は 45（39 と 6）と 51 で、増えた 6 つは表の末尾の「main の取り込みで増えた」の行。

自前の締切の検索は本番のファイルにも当たる（`emo2_boot/balloon_visibility_phase.rs`・`exit_wait.rs`・`perf_thread_report.rs`）が、これらは本番の処理の待ちで待ちの部品の利用ではないので、数えに入れない。

#### areka の crate（51 ファイル）

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
| `emo2_boot/ghost_switch_balloon_tests.rs`（main の取り込みで増えた） | `wait_steady`・`pump_talking_until` | 足場経由 |
| `mcp/dump_surface_tests.rs`（同上） | `wait_steady`・`pump_talking_until`・`pump_input_until`（各 3）。ほかに自前の `recv_timeout` 20 秒（固まりを解いた後に裏の処理が終わった知らせ） | 足場経由。自前の 20 秒は対象外（`mcp/get_expression_table_tests.rs` と同じ扱い・赤の観測が無い） |
| `mcp/get_status_tests.rs`（同上） | `wait_steady`・`pump_talking_until`・`pump_input_until`。`spin_wait_until` はコメントの中の名指しだけ | 足場経由 |
| `mcp/mcp_tests.rs`（同上・足場は前から使っていた） | `pump_talking_until` | 足場経由 |
| `emo2_boot/film_playback_e2e_tests.rs`（同上） | 自前の締切: `recv` が seriko の受け口から指令を `recv_timeout` 10 秒で 1 件受ける | 対象外（動く絵の族・赤の観測が無い）。赤になればタスク 5.4 |
| `readme/opener_submit_tests.rs`（同上） | 自前の締切: 断る口の記録に URL が載るまで `Instant::now() + 10 秒` で待つ（1 ms ずつ休む） | 対象外（外へ開く口の族・赤の観測が無い）。赤になればタスク 5.4 |

「spine の族」の自前の締切（`Instant::now() + SPIN_WAIT` の繰り返し）は Tick を注入する待ちで、design の Non-Goals と「2.1 の例外」に当たる。`SPIN_WAIT` の名前と値は `spine_wait.rs` から出し直されるので、これらのファイルは触らない。

#### 他の crate（52 ファイル・すべて対象外）

どれも areka の部品とは別物の、同じ名前の自前の `run_bounded`／`join_bounded` か、コメントの中の名指しだけ（requirements の Out of scope・design の Non-Goals）。

| crate | ファイル | 中身 |
|---|---|---|
| `areka-actor`（1） | `src/spawn.rs` | 本番のファイルの中のテストのモジュールの自前の `run_bounded` |
| `areka-ghost`（17） | `src/relay.rs`（本番のファイルの中のテストのモジュール）・`src/dispatcher_choice_tests.rs`・`src/dispatcher_choice_timeout_tests.rs`・`src/dispatcher_slot_tests.rs`・`src/dispatcher_test_support.rs`・`src/runtime_tests.rs`・`src/ticker_tests.rs`・`tests/ghost/inproc_e2e_test.rs`・`tests/ghost/real_pasta_test.rs`・`tests/ghost/spine_e2e_test_s1_boot_success.rs`・`…_s2_connect_failure.rs`・`…_s3_helper_liveness_detected.rs`・`…_s4_close_handshake.rs`・`…_s5_close_deadline.rs`・`…_s6_full_disconnect.rs`・`…_s7_second_boot_record_present.rs`・`tests/ghost/sylphya_integration_test.rs` | 自前の `run_bounded`／`join_bounded`。s1・s3 の `spin_wait_until` はコメントの中の名指しだけ |
| `areka-kanade`（34） | `src/actor_tests.rs` と、`tests/kanade/` の `common/`（`common_bounded.rs`・`common_mock_sakura.rs`・`common_smoke.rs`・`common_window_actor.rs`・`mod.rs`）・`boot_test.rs`・`choice_test*.rs`（8）・`close_test*.rs`（4）・`external_status_test.rs`・`failure_test.rs`・`full_run_test.rs`・`idle_pump_test.rs`・`mouse_test*.rs`（5）・`prefetch_test.rs`・`real_helper_test.rs`・`resource_query_test.rs`・`status_query_test.rs`（main の取り込みで増えた）・`steady_test.rs`・`translate_test.rs` | 自前の `run_bounded`／`join_bounded`（`common_bounded.rs` の定義を共有） |

`wait_steady`・`pump_until`・`pump_talking_until`・`pump_input_until` を語として使うファイルは、areka の crate の外には無い（`areka-emo-text`・`shiori-host32-host` などに当たるのは `pump_until_idle` などの別の名前）。

### 2.2 起票時の数との差

| 数えたもの | 起票時 | 今 | 差 |
|---|---|---|---|
| 切替の足場 `SwitchRig` を名指しするファイル（定義の `ghost_switch_test_support.rs` を除く） | 28 | 31 | +3 |
| `spin_wait_until` か `SPIN_WAIT` を名指しするファイル（areka の crate） | 20 | 21 | +1 |

2026-10-06（`95e8c4ab`）の時点では差 0 だった。差はどれも 2026-10-07 の main の取り込みで増えたテストのファイルで、足場の +3 は `emo2_boot/ghost_switch_balloon_tests.rs`・`mcp/dump_surface_tests.rs`・`mcp/get_status_tests.rs`、名指しの +1 は `mcp/get_status_tests.rs`（コメントの中の名指しだけで、呼び出しは無い）。どれも足場経由で、直接の呼び出しは増えていない。補足:

- 足場を名指しせずに使うファイルが 2 つある。`mcp/dump_surface_gpu_tests.rs` と `mcp/dump_balloon_gpu_tests.rs` は `GpuRig`（`mcp/dump_surface_gpu_test_support.rs`）の中の `SwitchRig` を使う。間接の利用を含めると 33。
- `spin_wait_until` か `SPIN_WAIT` をワークスペース全体（`*.rs`）で語として探すと 23 ファイル（`spin_wait_until` だけなら 19 ＝ areka 17・areka-ghost 2）。areka の 21 から増えた 2 つは `areka-ghost` の `spine_e2e_test_s1_boot_success.rs`・`spine_e2e_test_s3_helper_liveness_detected.rs` で、どちらもコメントの中の名指しだけ（呼び出しは無い）。
- 1.4 の要件の部品（`run_bounded`・`join_bounded`・`wait_steady`・`pump_*`・自前の待ち）まで広げると、areka の crate で 51 ファイル（上の 2.1）。

### 2.3 直接の `spin_wait_until` のうち移すもの（移す先の一覧）

対象の族のファイルで `spin_wait_until` を直接呼び、足場か偽の SHIORI の観測口（`ScriptedShioriHandle`）が手元にある呼び出し。取り直した数は 7 か所・4 ファイルで、設計の時点の数と同じ（main の取り込みの後も同じ）。

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

関数の呼び出しを、テストの関数から入口まで定義の名前でたどって数えた（`SwitchRig::new`・`SpineHarness::boot`／`boot_live`／`boot_with`・`acquire_emo2`・`LapRig` と `GpuRig` の作り口などを経由するもの）。この表は 2026-10-06（`95e8c4ab`）の数で、main の取り込みでは数え直していない（増えた足場のテストの分だけ、`areka` の「作って捨てる」が少し多くなる向き）。たどりは字面の照合なので、`use super::*` などの取り込み越しの呼び出しを取りこぼしうる（少なめに出る向き）。「作って捨てる」はテストごとに複製を作り、終わりで `Drop` が木を消すもの。「共有の 1 つ」はプロセスに 1 つの `LazyLock` の複製を読むだけのもの。

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

記録 `target\load-flake\before-20261008-001257\`（`conditions.txt`・`build.log`・`round-1.log`〜`round-5.log`・`summary.txt`）。上の「1. 手順」のコマンドのとおり `-Label before -Rounds 5 -Burners 44 -RoundTimeoutMin 30 -NoCapture` で回した。

### 3.1 条件

| 項目 | 値（`conditions.txt`・`summary.txt`） |
|---|---|
| コミット | `d6d77336`（未コミットの変更なし）。Rust のコードは今の `HEAD` と同じ（`d6d77336` からの差分は spec の文書と `tools/load-flake.ps1` だけ） |
| 回したもの | `cargo test -p areka --bin areka -- --nocapture`（2,802 本・無視 2 本） |
| 時刻 | 2026-10-08 00:12:57〜00:56:56 |
| 論理 CPU・負荷の子 | 22・44 本（止めた後の残り 0） |
| 負荷の前の CPU | 平均 2.5%・最大 5.2%（5 回の読み） |
| 負荷の最中の CPU | 平均 91.0%・最大 92.6%（5 回の読み） |

### 3.2 回ごとのまとめ

| 回 | 所要時間（`wall_sec`） | うち cargo の準備（ログの 1 行目の `Finished … in`） | libtest の時間（`finished in`） | 結果 | 赤 | libtest の「60 秒を越えて走っている」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 297.8 | 4.81 秒 | 251.83 秒 | 2,788 通過・12 失敗・2 無視 | 12 | 20 行 |
| 2 | 819.2 | 8 分 15 秒 | 265.37 秒 | 2,791 通過・9 失敗・2 無視 | 9 | 14 行 |
| 3 | 251.8 | 2.69 秒 | 219.87 秒 | 2,795 通過・5 失敗・2 無視 | 5 | 3 行 |
| 4 | 666 | 10.67 秒 | 510.33 秒 | 2,763 通過・37 失敗・2 無視 | 37 | 53 行 |
| 5 | 576.5 | 9.12 秒 | 461.53 秒 | 2,782 通過・18 失敗・2 無視 | 18 | 42 行 |
| **計** | 中央値 576.5・最大 819.2 | | 中央値 265.37・最大 510.33 | 5 回とも赤 | **81** | 132 行 |

- 5 回とも上限（30 分）を越えず、どの回も `test result:` の行まで出て終わった（果てしなく終わらない回は無い）。
- `summary.txt` の `wait_cut_lines`（待ちの打ち切りの行）は 5 回とも 0。上の「読み方の注意」のとおり、直す前の待ちの部品は文言を出さないので 0 が正しい。待っていた部品と締切は、下の 3.3・3.4 で `panicked at` の場所から引いた。
- 赤のテストの名前は、各回の末尾の `failures:` の一覧と `summary.txt` の一覧が一致する（12・9・5・37・18）。赤の 81 件は、どれも `thread '赤のテストの名前' panicked at …` の行がちょうど 1 つある（スレッドの名前で引いた）。
- 2 回目の 819.2 秒のうち、cargo がテストを始める前に 8 分 15 秒かかっている（ログの 1 行目が ``Finished `test` profile … in 8m 15s``。`Compiling` の行は無い）。回ごとの所要時間を比べるときは libtest の時間を見る。

### 3.3 待ちの部品の索引

下の表の記号で、3.4・3.5 の「待ち」を書く。定義は 2.1 の表と同じ。

| 記号 | 待ちの部品 | 締切 | 打ち切られたときの形 |
|---|---|---|---|
| A | `SwitchRig::shutdown`（`ghost_switch_test_support.rs`） | `run_bounded` の 20 秒 | `spine.rs:573` の `assert!` が `'置き場のゴーストを降ろす' did not complete within 20s (possible hang)` で落ちる |
| B | `SpineHarness::shutdown_bounded`（`spine.rs`） | `run_bounded` の 10 秒 | 同じ場所が `'spine ghost shutdown' did not complete within 10s (possible hang)` で落ちる |
| C | `ghost_session_restart_tests.rs` の自前の `shutdown_bounded` | `run_bounded` の 20 秒 | 同じ場所が `'1 周目を降ろす' did not complete within 20s (possible hang)` で落ちる |
| D | `SwitchRig::pump_talking_until` | `spin_wait_until` の 30 秒（`SPIN_WAIT`） | `false` を返すだけ。呼び手の確かめが落ちる |
| E | `SwitchRig::pump_until` | 同上 | 同上 |
| F | `SwitchRig::wait_steady` | `recv_timeout` の 20 秒 | `false` を返すだけ（20 秒届かないときと、先に別の知らせが届いたときの両方） |
| G | `LapRig::frames_until`（`shell_balloon_switch_session_lap_tests.rs`・合成の Tick を注入） | `spin_wait_until` の 30 秒 | `false` を返すだけ |
| H | `GpuRig::frames_until`（`mcp/dump_surface_gpu_test_support.rs`・合成の Tick を注入） | `spin_wait_until` の 30 秒 | `false` を返すだけ |
| I | `spin_wait_until` の直接の呼び出し | 30 秒 | `false` を返すだけ |
| J | spine の族の自前の締切（`Instant::now() + SPIN_WAIT`・駆動器の `clock() + SPIN_WAIT`） | 30 秒 | 自前の文言で落ちる |
| K | 待ちではない（待ちは届き、その後の確かめが食い違った） | — | — |

### 3.4 `panicked at` の場所ごとの読み（34 か所）

「偽になった値」は、`assert_eq!` の `left` と `right` を見比べて食い違った値。待ちの返り値が `false` だった値を書き、それに続いて食い違った値（待ちが届かなかったので、その後の状態がまだ来ていない値）は「続いて」として添える。

| # | `panicked at` | 赤の数（回） | 文言の要点 | 偽になった値と待ち |
|---|---|---|---|---|
| 1 | `emo2_boot\spine.rs:573:5`（`run_bounded` の `assert!`） | 24（回 3・4・5） | `'置き場のゴーストを降ろす' … within 20s` 20 件・`'spine ghost shutdown' … within 10s` 3 件・`'1 周目を降ろす' … within 20s` 1 件 | 降ろしの待ちそのものの打ち切り。文言で A 20・B 3・C 1 に分かれ、呼び手はスレッドの名前で読む。降ろしは確かめの `assert_eq!` より前に呼ぶ作りなので、それより前の待ちが届いていたかはこの記録からは分からない |
| 2 | `ghost_session_switch_fallback_tests.rs:153:5` | 3（回 1・2・4） | `切替先の失敗で既定ゴーストへ戻らない（…）` | 「既定の定常まで届いた」（`pump_to_welcomed`＝D）。続いて、既定の呼出列が空・予約が残る・今のゴーストが B・切替先の失敗の記録 0 件。3 回とも同じ形 |
| 3 | `ghost_session_switch_fallback_tests.rs:219:5` | 2（回 4・5） | `既定ゴーストの無い根で致命にならない（…）` | 「終了の指示」（`pump_talking_until(exit_requested)`＝D）。続いて、最初の出所が無い・予約が残る・切替先の失敗と致命の記録 0 件 |
| 4 | `ghost_session_switch_fallback_tests.rs:275:5` | 2（回 1・4） | `既定ゴーストの失敗が今日の失敗の経路で終わらない（…）` | 「終了の指示」（D）。回 1 は切替で起こした数 2（既定まで起きた）、回 4 は 1 |
| 5 | `ghost_session_switch_fallback_tests.rs:434:5` | 3（回 1・4・5） | `同期の失敗で既定の窓だけにならない（…）` | 「既定の定常まで届いた」（`pump_to_default_steady`＝E）。回 1・4 は続いて既定の `OnBoot` がまだ無い |
| 6 | `ghost_session_switch_fallback_tests.rs:501:5` | 2（回 1・4） | `切替先の閉包が着く前の失敗で孤児の窓が生えた（…）` | 「既定の定常まで届いた」（E）。回 4 はその前の「A の定常」（F）も偽 |
| 7 | `ghost_session_switch_tests.rs:328:5` | 1（回 4） | `1 周が通らない（…）` | 「B の定常まで届いた」（`pump_to_welcomed`＝D）。続いて予約が残る |
| 8 | `ghost_session_switch_tests.rs:414:5` | 3（回 2・4・5） | `起動記録の無い B の起動の根が OnFirstBoot でない（…）` | 「B の定常まで届いた」（D）・続いて予約が残る。回 2・4 は B の呼出列が空、回 5 は B の起動の系列（`OnFirstBoot` まで）は届いていた |
| 9 | `ghost_session_switch_memory_tests.rs:296:5` | 1（回 4） | `既定へ戻す間と既定の定常のあとの記憶が崩れた（…）` | 「戻しの間」に届いた（`pump_to_welcoming`＝D） |
| 10 | `ghost_session_switch_memory_tests.rs:327:5` | 1（回 4） | `致命のあとの記憶が崩れた（…）` | 「終了の指示」（D）。続いて後始末の判定が `no_exit_origin` |
| 11 | `emo2_boot\frame_ghost_quit_switch_tests.rs:146:5` | 1（回 2） | `(A の定常・受理・再生の完了を届けた・予約が消えた・…)` | 「予約が消えた」（`SwitchRig::pump_until`＝E）。続いて切替の完了の記録 0 件 |
| 12 | `emo2_boot\ghost_switch_balloon_tests.rs:65:5` | 1（回 2） | `assert_eq!(flow, (Accepted, true))`（文言は捕まえた記録） | `flow` の 2 つ目（`switch_to_b` の中の `pump_talking_until`＝D） |
| 13 | `emo2_boot\ghost_switch_boot_event_tests.rs:191:5` | 1（回 2） | 同じ形 | `flow` の 2 つ目（`switch_to_a` の中の `pump_talking_until`＝D） |
| 14 | `emo2_boot\ghost_switch_boot_event_tests.rs:62:5` | 2（回 4） | `A が定常に着く` | `running_a` の `wait_steady`（F） |
| 15 | `shell_balloon_switch_session_tests.rs:97:5` | 1（回 4） | `(定常, 判定, 届いたか, OnShellChanging の Reference, 降ろせた)` | 「定常」（F）と「届いたか」（92 行目の `spin_wait_until` の直接の呼び出し＝I） |
| 16 | `shell_balloon_switch_session_lap_tests.rs:637:5` | 1（回 4） | `((定常, 台詞が始まった, 受理, …), …)` | 「定常」（F）と「台詞が始まった」（`frames_until(CREEP)`＝G）。その後の待ちは届いた |
| 17 | `shell_balloon_switch_session_balloon_tests.rs:275:5` | 4（回 1・2・4・5） | `((定常, 往復が終わった, 起動の回数, 呼出列), …)` | 「往復が終わった」（`frames_until(CREEP)`＝G）。4 回とも、2 つ目のバルーン（`emo2-kakukaku` へ戻る側）の記録が欠けた同じ形 |
| 18 | `shell_balloon_switch_session_abort_tests.rs:427:5` | 2（回 1・5） | `(…, (ゴースト切替の判定, B が迎え入れられシェル切替の印も無い), …)` | 「B が迎え入れられ…」（`frames_until(UNBOUNDED)`＝G） |
| 19 | `shell_balloon_switch_session_abort_tests.rs:493:5` | 2（回 1・5） | `((定常, 差し替わった, B が迎え入れられた), …)` | 「B が迎え入れられた」（`frames_until(UNBOUNDED)`＝G） |
| 20 | `shell_balloon_switch_session_update_tests.rs:132:5` | 1（回 4） | `((定常, 差し替わった), …)` | 「差し替わった」（`swap_both`＝`frames_until(UNBOUNDED)`＝G） |
| 21 | `shell_balloon_switch_session_update_tests.rs:218:5` | 1（回 4） | `((定常, 差し替わった, 読み直しが終わった), …)` | 「定常」（`steady_rig` の `wait_steady`＝F）と「差し替わった」（G）。読み直しは届いた |
| 22 | `install\desk_overwrite_tests.rs:110:5` | 1（回 1） | `依頼が終わる（期限切れ）: [捕まえた記録]` | `finished`（`pump_talking_until`＝D） |
| 23 | `install\desk_overwrite_tests.rs:368:5` | 1（回 2） | `(予約の間は待つ・下りた tick に頼み直す・頼んだ切替先・途中の要求・一周した): …` | 「一周した」（`pump_talking_until`＝D） |
| 24 | `install\desk_overwrite_tests.rs:379:5` | 1（回 3） | `展開した結果が返る` | K。368 行の確かめは通っている（`pump_talking_until` は届き、答えも返った）が、返った答えが `Overwritten::Ran(Ok(_))` でない。文言に答えの中身が出ないので、`Ran(Err(_))`・`NotRunning`・`Closed` のどれだったかはこの記録からは分からない |
| 25 | `install\desk_overwrite_tests.rs:590:5` | 2（回 1・5） | `依頼が終わり、終了しない: [捕まえた記録]`（`assert!(finished && !exited)`） | どちらが偽かは文言に出ない。捕まえた記録に終了の指示の記録（`app_exit`）が無いので、`finished`（`install_through` の中の `pump_talking_until`＝D）が偽と読む |
| 26 | `update\desk_reload_tests.rs:545:5` | 2（回 2・3） | `既定ゴーストへ戻る: [捕まえた記録]`（`assert!(welcomed && !exited)`） | 同じ読みで `welcomed`（`pump_talking_until`＝D）。記録には切替先の失敗（`ghost_switch_target_fault`）と `ghost_switch_booted` 2 件まで載っている |
| 27 | `mcp\dump_surface_tests.rs:432:5` | 4（回 1・2・3・5） | `（その場の答え, A の定常, B の定常まで届いた, …）` | 「B の定常まで届いた」（`pump_talking_until`＝D）。回 5 はその前の「A の定常」（F）も偽 |
| 28 | `mcp\dump_surface_tests.rs:528:5` | 2（回 1・5） | `（A の定常, B の定常まで届いた, …）` | 「B の定常まで届いた」（D）。回 5 は「A の定常」（F）も偽 |
| 29 | `mcp\dump_balloon_gpu_tests.rs:312:5` | 2（回 1・4） | `（字が現れ切った, 隠す前の可視, 隠れた, …）` | 「隠れた」（`GpuRig::frames_until`＝H） |
| 30 | `mcp\dump_balloon_gpu_tests.rs:261:5` | 1（回 5） | `（96 の事実, 144 の事実, 96 の大きさ, 144 の大きさ）` | DPI 96 の回の「話し終えた」（`speak`＝`GpuRig::frames_until`＝H） |
| 31 | `mcp\dump_surface_gpu_tests.rs:50:18` | 1（回 5） | `本文＋画像 1 枚の形ではない: [Text("NG:No surface has been shown in this scope yet")]` | 待ちの返り値 `shown`（`GpuRig::frames_until`＝H）は確かめずに先へ進む作り。答えが「面がまだ出ていない」なので、H が届かないまま打ち切られたと読む |
| 32 | `emo2_boot\spine_hold_tests.rs:38:5` | 2（回 4） | `Get("OnBoot")`／`Notify("OnClose")`: `the call should be holding until unblocked` | 39 行目の `spin_wait_until(\|\| handle.holding())`（I）。固まる側のスレッドはその後 30 秒解かれずに落ちた（下の 3.7 の「赤でない `panicked at` の行」） |
| 33 | `emo2_boot\spine_seriko_loop_tests.rs:214:5` | 2（回 4） | `OnBoot talk が scope0 shell surface 1000（require_bind=…）を有界内に表示しない（…）` | `drive_shell_shown` の自前の締切（`Instant::now() + SPIN_WAIT`・Tick を注入して 200 µs ずつ休む）（J） |
| 34 | `emo2_boot\spine_conformance_lap_tests.rs:389:25` | 1（回 4） | `段「撫で」の完了条件が有界時間内に成立しない（注入 [28000, …]・採取 0 件・注入時刻 28000ms）` | 駆動器の自前の締切（`spine_conformance_support.rs` の `clock() + SPIN_WAIT`）（J）。注入の列は同じ 28000 ms が 8,446 個 |

### 3.5 回ごとの赤

「行」は `round-N.log` の `panicked at` の行。「作業フォルダ」は、赤の文言（`panicked at` の行から `stack backtrace:` まで）に出た `target\nar-samples\work\<番号>-<連番>`（そのテストが使った複製）と、同じ木を指す後片付けの失敗の行。文言にパスが出ないテストは照合できない（「出ない」）。

#### 1 回目（`round-1.log`・赤 12）

| テスト | 場所 | 待ち | 行 | 作業フォルダと後片付けの行 |
|---|---|---|---|---|
| `ghost_session::switch_tests::fallback_tests::target_connect_fail_boots_default_with_halt_and_no_alert` | 2 | D | 1408 | `27912-100`・1430 行の「残骸の退避」（赤の後） |
| `ghost_session::shell_balloon_switch_session_balloon_tests::script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` | 17 | G | 1554 | `27912-70`・無し |
| `ghost_session::switch_tests::fallback_tests::async_target_fault_before_window_closure_leaves_only_default_windows` | 6 | E | 1587 | `27912-96`・1623 行（赤の後） |
| `ghost_session::switch_tests::fallback_tests::default_ghost_fault_after_fallback_exits_through_shiori_fault_path` | 4 | D | 1641 | `27912-97`・1679・1703〜1706 行の 5 行（赤の後） |
| `mcp::dump_balloon::dump_balloon_gpu_tests::hidden_balloon_returns_the_same_pixels` | 29 | H | 1775 | 出ない |
| `install::desk::overwrite_tests::overwriting_the_running_ghost_takes_it_down_installs_and_boots_it_again` | 22 | D | 1804 | `27912-123`・無し |
| `ghost_session::switch_tests::fallback_tests::sync_target_boot_failure_leaves_only_default_windows` | 5 | E | 1821 | `27912-95`・無し |
| `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` | 18 | G | 1872 | 出ない |
| `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_after_a_shell_swap_unloads_the_session` | 19 | G | 1952 | 出ない |
| `install::desk::overwrite_tests::an_overwritten_ghost_that_cannot_boot_falls_back_to_the_default_ghost` | 25 | D（読み） | 1982 | `27912-122`・無し |
| `mcp::dump_surface::dump_surface_tests::switch_after_the_copy_still_answers_the_copied_picture` | 28 | D | 3270 | 出ない |
| `mcp::dump_surface::dump_surface_tests::switch_to_another_ghost_answers_not_active_without_mcp_errors` | 27 | D | 3298 | 出ない |

#### 2 回目（`round-2.log`・赤 9）

| テスト | 場所 | 待ち | 行 | 作業フォルダと後片付けの行 |
|---|---|---|---|---|
| `emo2_boot::frame::ghost_quit_switch_tests::stop_with_handoff_under_reservation_switches_without_exit` | 11 | E | 925 | `34296-9`・948・951 行（赤の後） |
| `emo2_boot::ghost_switch::balloon_tests::switching_to_a_ghost_whose_descript_names_a_balloon_uses_it_and_records_the_descript_route` | 12 | D | 975 | `34296-11`・1000 行（赤の後） |
| `emo2_boot::ghost_switch::boot_event_tests::a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails` | 13 | D | 1018 | `34296-13`・1043・1045 行（赤の後） |
| `ghost_session::switch_tests::fallback_tests::target_connect_fail_boots_default_with_halt_and_no_alert` | 2 | D | 1516 | `34296-109`・1563・1620・1621・1634 行（赤の後） |
| `ghost_session::shell_balloon_switch_session_balloon_tests::script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` | 17 | G | 1541 | `34296-77`・1564・1625・1627・1635 行（赤の後） |
| `ghost_session::switch_tests::switch_to_b_without_boot_record_sends_first_boot_not_ghost_changed` | 8 | D | 1995 | 出ない |
| `install::desk::overwrite_tests::a_busy_overwrite_retries_on_the_tick_the_reservation_clears_and_runs_through` | 23 | D | 2295 | `34296-158`・2468 行（赤の後） |
| `mcp::dump_surface::dump_surface_tests::switch_to_another_ghost_answers_not_active_without_mcp_errors` | 27 | D | 3251 | 出ない |
| `update::desk::reload_tests::a_failed_reload_switch_drops_the_remembered_folder_and_the_tail` | 26 | D（読み） | 3293 | `34296-281`・無し |

#### 3 回目（`round-3.log`・赤 5）

| テスト | 場所 | 待ち | 行 | 作業フォルダと後片付けの行 |
|---|---|---|---|---|
| `emo2_boot::spine::boot_smoke_tests::spine_preview_reaches_text_layer_through_production_chain` | 1 | B | 770 | 出ない |
| `emo2_boot::spine::conformance_support::driver_tests::kanade_tick_raises_one_second_change_where_the_waiting_injection_raises_none` | 1 | B | 791 | 出ない |
| `install::desk::overwrite_tests::a_busy_overwrite_retries_on_the_tick_the_reservation_clears_and_runs_through` | 24 | K | 1357 | 出ない |
| `mcp::dump_surface::dump_surface_tests::switch_to_another_ghost_answers_not_active_without_mcp_errors` | 27 | D | 3114 | 出ない |
| `update::desk::reload_tests::a_failed_reload_switch_drops_the_remembered_folder_and_the_tail` | 26 | D（読み） | 3148 | `32492-260`・無し |

#### 4 回目（`round-4.log`・赤 37）

| テスト | 場所 | 待ち | 行 | 作業フォルダと後片付けの行 |
|---|---|---|---|---|
| `emo2_boot::ghost_switch::boot_event_tests::a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails` | 1 | A | 762 | 出ない |
| `emo2_boot::spine::conformance_lap_tests::conformance_lap_walks_every_stage_to_its_completion` | 34 | J | 790 | 出ない |
| `emo2_boot::ghost_switch::boot_event_tests::a_switch_without_a_boot_event_boots_the_target_with_on_ghost_changed_as_today` | 14 | F | 812 | 出ない |
| `emo2_boot::ghost_switch::boot_event_tests::a_switch_with_a_boot_event_boots_the_target_with_that_event_instead_of_changed_or_boot` | 14 | F | 831 | 出ない |
| `emo2_boot::spine::hold_support::tests::held_get_waits_until_unblocked_then_times_out` | 32 | I | 850 | 出ない |
| `emo2_boot::spine::hold_support::tests::held_notify_waits_until_unblocked_then_times_out` | 32 | I | 869 | 出ない |
| `emo2_boot::frame::ghost_quit_switch_tests::stop_with_handoff_under_reservation_switches_without_exit` | 1 | A | 892 | 出ない |
| `ghost_session::shell_balloon_switch_session_tests::session_holding_seriko_sink_shuts_down_within_bound` | 1 | A | 1151 | 出ない |
| `ghost_session::restart_tests::boots_twice_in_one_process_without_double_registration` | 1 | C | 1172 | 出ない |
| `ghost_session::strict_tests::logsink_arm_sets_fallback_flag_and_uses_wiring_app_dir` | 1 | A | 1193 | 出ない |
| `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` | 1 | A | 1215 | 出ない |
| `ghost_session::shell_balloon_switch_session_tests::menu_balloon_frame_marks_the_current_balloon_and_selecting_requests_a_switch` | 1 | A | 1236 | 出ない |
| `ghost_session::shell_balloon_switch_session_tests::menu_shell_frame_marks_the_mounted_shell_and_selecting_requests_a_switch` | 1 | A | 1257 | 出ない |
| `emo2_boot::spine::text_scale_tests::text_scale_phase_syncs_boxes_on_the_shell_window` | 1 | B | 1278 | 出ない |
| `emo2_boot::spine::seriko_loop_tests::spine_e2e_sakura_blink_after_bind_one_cycle_golden` | 33 | J | 1299 | 出ない |
| `emo2_boot::spine::seriko_loop_tests::spine_e2e_sakura_blink_default_off_emits_nothing` | 33 | J | 1318 | 出ない |
| `ghost_session::shell_balloon_switch_session_lap_tests::script_shell_switch_round_trips_with_raise_event` | 1 | A | 1351 | 出ない |
| `ghost_session::boot_shell_tests::missing_remembered_shell_boots_default_and_rewrites_memory` | 1 | A | 1372 | 出ない |
| `ghost_session::boot_shell_tests::remembered_hidden_shell_reaches_all_resolutions_decided_once` | 1 | A | 1406 | 出ない |
| `ghost_session::shell_balloon_switch_session_tests::menu_shell_request_raises_on_shell_changing_with_the_current_shell_as_ref1` | 15 | F・I | 1427 | `35832-120`・無し |
| `ghost_session::shell_balloon_switch_session_lap_tests::plain_script_shell_switch_skips_changing_and_waits_for_the_script_end` | 16 | F・G | 1450 | `35832-104`・1478〜1483 行の 6 行（赤の後） |
| `ghost_session::shell_balloon_switch_session_update_tests::a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon` | 21 | F・G | 1486 | `35832-114`・無し |
| `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_after_a_shell_swap_unloads_the_session` | 1 | A | 1511 | 出ない |
| `ghost_session::shell_balloon_switch_session_update_tests::update_targets_follow_the_swapped_shell_and_balloon` | 20 | G | 1537 | `35832-116`・無し |
| `ghost_session::switch_tests::rig_boots_a_and_records_its_boot_sequence` | 1 | A | 1560 | 出ない |
| `ghost_session::switch_tests::switch_to_b_without_boot_record_sends_first_boot_not_ghost_changed` | 8 | D | 1583 | 出ない |
| `ghost_session::switch_tests::fallback_tests::async_target_fault_before_window_closure_leaves_only_default_windows` | 6 | F・E | 1616 | `35832-126`・無し |
| `ghost_session::switch_tests::script_change_tag_switches_a_to_b_and_reaches_steady` | 7 | D | 1652 | `35832-142`・無し |
| `ghost_session::shell_balloon_switch_session_balloon_tests::script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` | 17 | G | 1732 | `35832-110`・無し |
| `ghost_session::switch_tests::fallback_tests::target_fault_without_default_ghost_is_fatal` | 3 | D | 1912 | `35832-131`・無し |
| `ghost_session::switch_tests::fallback_tests::target_connect_fail_boots_default_with_halt_and_no_alert` | 2 | D | 1935 | `35832-133`・2031〜2033 行（赤の後） |
| `ghost_session::switch_tests::memory_tests::switch_fatal_leaves_default_last_ghost_and_target_mark` | 10 | D | 1958 | 出ない |
| `ghost_session::switch_tests::fallback_tests::default_ghost_fault_after_fallback_exits_through_shiori_fault_path` | 4 | D | 1983 | `35832-127`・2029・2030 行（赤の後） |
| `ghost_session::switch_tests::fallback_tests::sync_target_boot_failure_leaves_only_default_windows` | 5 | E | 2007 | `35832-128`・2050〜2052 行（赤の後） |
| `ghost_session::switch_tests::memory_tests::default_fallback_keeps_target_mark_until_default_steady` | 9 | D | 2159 | 出ない |
| `install::desk::record_tests::several_ghosts_leave_the_last_in_last_installed` | 1 | A | 2623 | 出ない |
| `mcp::dump_balloon::dump_balloon_gpu_tests::hidden_balloon_returns_the_same_pixels` | 29 | H | 2728 | 出ない |

#### 5 回目（`round-5.log`・赤 18）

| テスト | 場所 | 待ち | 行 | 作業フォルダと後片付けの行 |
|---|---|---|---|---|
| `emo2_boot::frame::ghost_quit_switch_tests::stop_with_handoff_under_reservation_switches_without_exit` | 1 | A | 576 | 出ない |
| `ghost_session::shell_balloon_switch_session_lap_tests::plain_script_shell_switch_skips_changing_and_waits_for_the_script_end` | 1 | A | 1263 | 出ない |
| `ghost_session::switch_translate_tests::switch_sends_each_on_translate_to_its_own_ghost` | 1 | A | 1284 | 出ない |
| `emo2_boot::ghost_switch::boot_event_tests::a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails` | 1 | A | 1312 | 出ない |
| `install::desk::record_tests::a_balloon_only_install_rewrites_the_running_ghosts_balloon_memory` | 1 | A | 1338 | 出ない |
| `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` | 18 | G | 1359 | 出ない |
| `install::desk::overwrite_tests::an_overwritten_ghost_that_cannot_boot_falls_back_to_the_default_ghost` | 25 | D（読み） | 1411 | `27624-144`・無し |
| `ghost_session::switch_tests::switch_to_b_without_boot_record_sends_first_boot_not_ghost_changed` | 8 | D | 1576 | 出ない |
| `ghost_session::shell_balloon_switch_session_balloon_tests::script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` | 17 | G | 1615 | `27624-89`・1639・1643 行（赤の後） |
| `ghost_session::switch_tests::fallback_tests::target_fault_without_default_ghost_is_fatal` | 3 | D | 1747 | `27624-123`・1833 行（赤の後） |
| `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_after_a_shell_swap_unloads_the_session` | 19 | G | 1885 | 出ない |
| `ghost_session::switch_tests::fallback_tests::sync_target_boot_failure_leaves_only_default_windows` | 5 | E | 1970 | `27624-118`・無し |
| `install::names::tests::reboot_reseeds_the_values_into_the_new_ghost` | 1 | A | 2041 | 出ない |
| `mcp::mcp_tests::real_unit_resolves_by_sakura_name_and_ascii_case` | 1 | A | 2062 | 出ない |
| `mcp::dump_surface::dump_surface_gpu_tests::shown_surface_matches_the_composed_pixels_at_native_size` | 31 | H（読み） | 2103 | 出ない |
| `mcp::dump_surface::dump_surface_tests::switch_after_the_copy_still_answers_the_copied_picture` | 28 | F・D | 2928 | 出ない |
| `mcp::dump_surface::dump_surface_tests::switch_to_another_ghost_answers_not_active_without_mcp_errors` | 27 | F・D | 3027 | 出ない |
| `mcp::dump_balloon::dump_balloon_gpu_tests::balloon_at_dpi_144_keeps_native_size_and_text_position` | 30 | H | 3480 | 出ない |

### 3.6 後片付けの失敗の行（os error 5）

数え方は 5.4 と同じく 2 通りを並べる。「`summary.txt` の数」は道具のそのままの値（行の頭が `sample-ghost-kit:` で、同じ行に `os error 5` がある行）。「戻して読んだ数」は、ファイルの中の `（次の走行で回収する）`（`report_cleanup` の文の決まった部分）を 1 行 1 件として数え、os error 5 は `アクセスが拒否されました`（os error 5 の文）の数で数えた。どの回も 2 つの数は一致し、他の理由の失敗（別の os error の番号）は 0 件。

| 回 | 行（`summary.txt`） | うち os error 5（`summary.txt`） | 行（戻して読んだ） | うち os error 5（戻して読んだ） | 残骸の退避 | 退避した残骸の削除 | 作業フォルダの後始末 | 生存の札の後始末 |
|---|---|---|---|---|---|---|---|---|
| 1 | 45 | 45 | 45 | 45 | 45 | 0 | 0 | 0 |
| 2 | 94 | 93 | 94 | 94 | 93 | 0 | 0 | 1（`34296-96.lock`） |
| 3 | 63 | 63 | 64 | 64 | 62 | 1（`gc-32492-167`） | 0 | 1（`32492-188.lock`） |
| 4 | 70 | 68 | 70 | 70 | 69 | 0 | 1（`35832-24`） | 0 |
| 5 | 76 | 75 | 76 | 76 | 75 | 1（`gc-27624-109`） | 0 | 0 |
| **計** | **348** | **344** | **349** | **349** | **344** | **2** | **1** | **2** |

- 漏れの形（5.4 と同じく、他のテストの出力が行の途中に割り込む）。2 回目の 1041 行は行の頭の後に他のテストの名前、種類の名前の途中にその `ok` が入り（`残骸の退避okに失敗した`）、`(アクセスが拒否されました。 (os error 5))` が次の行にある。3 回目は 1514 行で行の頭がテストの名前の後ろに来て、本文が 1516 行にある。4 回目は 1127 行で文の途中に他のテストの `test … ok` が入り、`(os error` と `5))` が行をまたいで分かれている。2024 行は行の頭の直後に失敗の積み上げ（`stack backtrace`）の行が入って本文が 2029 行にある。5 回目の 1753 行は種類の名前の途中に積み上げの行が入り、`(アクセスが拒否されました。 (os error 5))` が次の行にある（パスの番号も混ざって読めない）。
- 指す木: どの回も、パスの番号はその回のテストのプロセスの番号だけ（27912・34296・32492・35832・27624）。前のプロセスの残りを指す行（2.4 の組 D）は無い。`work\gc-…` を指す「残骸の退避」の行は 0 で、`gc-` を指すのは「退避した残骸の削除」の 2 行だけ。「残骸の退避」344 行はどれも `work\<番号>-<連番>` を指す。同じ木を何度も指す（別々のテストの掃除が同じ木の退避に続けて失敗する）ことが多く、指した木（札を含む）の数は 30・45・36・29・41。木の連番は 1 回のうちに 212〜281 まで進んでいる。
- 静かな机（5.4）は 3 回で 633 行（戻して読んだ数で 1 回 191〜222 行）だったのに対し、負荷の下は 1 回 45〜94 行で、負荷の下の方が少ない。

赤との関わり（調べ方と結果）:

- 赤の文言が os error 5 やファイルの読み書きの失敗であるものは 0 件。赤 81 件の文言（`panicked at` の行から `stack backtrace:` まで）に `os error`・`アクセスが拒否`・`Access`・`denied` の語は 1 つも無い。ただし場所 24（`desk_overwrite_tests.rs:379`）は答えの中身が文言に出ないので、展開の失敗だったかどうかは文言からは分からない。
- 同じ木: 赤の文言に自分の作業フォルダのパスが出たのは 81 件のうち 30 件。そのうち 15 件は、同じ木を指す「残骸の退避」の行がその回にあり、15 件とも行はその赤の `panicked at` の行より後に出ている（回ごとに 1 回目 3/7・2 回目 6/7・3 回目 0/1・4 回目 4/11・5 回目 2/4）。赤のテストの木を指す後片付けの行が、その赤より前に出た例は無い。
- 前後の並び: 赤の `panicked at` の行の前 5 行以内にある後片付けの行は 13 行（9 件の赤の前）。どれも、その赤の文言に出た作業フォルダとは別の木を指す（作業フォルダが文言に出ない 1 件＝1 回目の 3298 行は照合できない）。
- 回ごとの数は赤の数と並ばない（赤 12・9・5・37・18 に対して、後片付けの行 45・94・64・70・76）。

### 3.7 回をまたいだまとめ

**何回も赤になったテスト**（`summary.txt` の `failed_tests_by_name`）: 赤になったテストは 52 種類（2×4＋7×3＋9×2＋34×1＝81 件）。4 回が 2 種類（`mcp::dump_surface::dump_surface_tests::switch_to_another_ghost_answers_not_active_without_mcp_errors`・`ghost_session::shell_balloon_switch_session_balloon_tests::script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot`）、3 回が 7 種類（`frame::ghost_quit_switch_tests::stop_with_handoff_under_reservation_switches_without_exit`・fallback の `sync_target_boot_failure_leaves_only_default_windows`・`target_connect_fail_boots_default_with_halt_and_no_alert`・abort の 2 本・`switch_to_b_without_boot_record_sends_first_boot_not_ghost_changed`・`boot_event_tests::a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails`）、2 回が 9 種類、1 回だけが 34 種類。同じテストでも回によって落ちる待ちが替わるものがある（例: `stop_with_handoff_under_reservation_switches_without_exit` は 2 回目が E、4・5 回目が A）。

**待ちの部品ごとの赤の数**（1 件の赤に 2 つの待ちが偽のときは、先に偽になった方で数えた）:

| 待ち | 1 回目 | 2 回目 | 3 回目 | 4 回目 | 5 回目 | 計 |
|---|---|---|---|---|---|---|
| A 足場の降ろし（20 秒） | 0 | 0 | 0 | 13 | 7 | 20 |
| B spine の降ろし（10 秒） | 0 | 0 | 2 | 1 | 0 | 3 |
| C 起こし直しのテストの降ろし（20 秒） | 0 | 0 | 0 | 1 | 0 | 1 |
| D `pump_talking_until`（30 秒） | 6 | 7 | 2 | 7 | 3 | 25 |
| E `pump_until`（30 秒） | 2 | 1 | 0 | 1 | 1 | 5 |
| F `wait_steady`（20 秒） | 0 | 0 | 0 | 6 | 2 | 8 |
| G `LapRig::frames_until`（30 秒） | 3 | 1 | 0 | 2 | 3 | 9 |
| H `GpuRig::frames_until`（30 秒） | 1 | 0 | 0 | 1 | 2 | 4 |
| I `spin_wait_until` の直接（30 秒） | 0 | 0 | 0 | 2 | 0 | 2 |
| J spine の族の自前の締切（30 秒） | 0 | 0 | 0 | 3 | 0 | 3 |
| K 待ちではない | 0 | 0 | 1 | 0 | 0 | 1 |
| 計 | 12 | 9 | 5 | 37 | 18 | 81 |

F が偽だった赤 8 件のうち 6 件は、後の待ち（E・G・D・I）も偽（F だけが偽なのは場所 14 の 2 件）。足場の待ち（A・D・E・F・G）で 67 件、spine の族（B・I の 2 件・J）で 8 件、MCP の描画の足場（H）で 4 件、起こし直し（C）で 1 件、待ちでないもの 1 件。降ろしの打ち切り（A・B・C）は 3〜5 回目にだけ出ている。

**設計の対象の族に入っていない赤**（タスク 5.4 の材料）:

| テスト | 赤の数 | 待ち | 2.1・2.3 での扱い |
|---|---|---|---|
| `mcp::dump_balloon::dump_balloon_gpu_tests::hidden_balloon_returns_the_same_pixels` | 2（回 1・4） | H | 2.3 の「移さない直接の呼び出し」（`GpuRig::frames_until`）。再現で赤になったら `LapRig::frames_until` と同じ移す先を使う、とした族 |
| `mcp::dump_balloon::dump_balloon_gpu_tests::balloon_at_dpi_144_keeps_native_size_and_text_position` | 1（回 5） | H | 同上 |
| `mcp::dump_surface::dump_surface_gpu_tests::shown_surface_matches_the_composed_pixels_at_native_size` | 1（回 5） | H（読み） | 同上 |
| `emo2_boot::spine::hold_support::tests::held_get_waits_until_unblocked_then_times_out`・`…::held_notify_waits_until_unblocked_then_times_out` | 2（回 4） | I | 2.1 の spine の族の例外（ファイルは触らない）。spine の族の赤は降ろし（B）の側と見ていたが、降ろしでない直接の待ちで赤になった |
| `emo2_boot::spine::seriko_loop_tests::spine_e2e_sakura_blink_after_bind_one_cycle_golden`・`…::spine_e2e_sakura_blink_default_off_emits_nothing` | 2（回 4） | J | 同上（自前の Tick 注入の締切。design の Non-Goals） |
| `emo2_boot::spine::conformance_lap_tests::conformance_lap_walks_every_stage_to_its_completion` | 1（回 4） | J | `spine_conformance_lap_tests.rs` は 2.1 の表に無い（待ちは `spine_conformance_support.rs` の駆動器の自前の締切） |

ほかに、待ちは足場の降ろし（A）か spine の降ろし（B）で、設計の直し（タスク 3.2・3.1）が届くが、ファイルが 2.1 の表に無いもの（2.1 は `shutdown` の語を数えていない）: `ghost_session_strict_tests.rs`（1）・`install/desk_record_tests.rs`（2）・`install/names_tests.rs`（1）・`emo2_boot/spine_text_scale_tests.rs`（1・B）。2.1 で「赤になればタスク 5.4」としたほかのファイル（`emo2_boot/frame/switch_tests.rs`・`frame_ghost_quit_logsink_tests.rs`・`install_cue_tests.rs`・`thread_roles_tests.rs`・`film_playback_e2e_tests.rs`・`readme/opener_submit_tests.rs`）は 5 回とも赤 0。特に見るとした `frame/switch_tests.rs` の `seriko_send_failure_empties_the_slot`（5 秒の空回しだけの待ち）は 5 回とも `ok`。`session_end_sync_send_tests.rs` と `session_end_deadline_tests.rs` も赤 0（後者は「60 秒を越えて走っている」の知らせが 4 行あるが緑）。

**所要時間と赤の数**: libtest の時間の短い順に、3 回目 219.87 秒・赤 5、1 回目 251.83 秒・赤 12、2 回目 265.37 秒・赤 9、5 回目 461.53 秒・赤 18、4 回目 510.33 秒・赤 37。赤はおおむね時間の長い回ほど多い（ただし 1 回目と 2 回目は逆で、短い 1 回目の方が赤が多い）。「60 秒を越えて走っている」の知らせも同じ順に 3・20・14・42・53 行で、おおむね時間の長い回ほど多い（ただし 1 回目と 2 回目は逆）。静かな机（5.4）の 1 回は 62.4〜66.4 秒。

**打ち切りでも確かめの食い違いでもないもの**:

- 果てしなく終わらない回・上限越えは無い（上の 3.2）。
- 待ちでない赤は場所 24 の 1 件だけ（3 回目 `a_busy_overwrite_retries_on_the_tick_the_reservation_clears_and_runs_through`。同じテストは 2 回目には D の打ち切りで赤）。
- 赤でない `panicked at` の行: 1・2・3・5 回目は毎回同じ 8 行（緑のテストがわざと起こすもの。`placement\follow\window_move.rs:688` が 4 行・`emo2_boot\switch_assets_tests.rs:264`・`log_history_convention_tests.rs:123`・`mcp\dump_surface_tests.rs:139`・`mcp\dump_surface_tests.rs:316`）。4 回目はこれに 2 行足して 10 行で、足された 2 行は名前の無いスレッドの `emo2_boot\spine_hold_support.rs:90:13` `ScriptedShioriBackend: the held call was never unblocked within 30s`（1000 行・1029 行）。場所 32 の 2 件の赤（850 行・869 行）が固まりを解かずに終わった後、固まっていた側が 30 秒の上限で落ちたもの。
- 2 回目は、cargo がテストを始めるまでに 8 分 15 秒かかった（上の 3.2）。

## 4. 直した後の記録

直した後は赤が残った（下の 4.1）。残った赤はタスク 5.4 で直し、同じ引数で回し直した 2 回目を 4.2 に書いた。2 回目にも赤が残った（テストのプロセスの中のスレッドの生まれと消えの取り合い・テストの台本の競り合い）ので、タスク 5.2・5.5 の後に同じ引数で回し直した。GPU の装置の許可の数を 2・8・4 と替えた回し、タスク 5.4 の開き直し、main の取り込みの後の回し、main の先端との比べを、4.3 に時刻の順で書く。最後の記録（深夜の回）は 4.4 に書く。

### 4.1 直した後の 1 回目

記録 `target\load-flake\after-20261008-074239\`（`conditions.txt`・`build.log`・`round-1.log`〜`round-5.log`・`summary.txt`）。「1. 手順」のコマンドの `-Label` だけを替えて `-Label after -Rounds 5 -Burners 44 -RoundTimeoutMin 30 -NoCapture` で回した（`conditions.txt` の `rounds: 5`・`burners: 44`・`round_timeout_min: 30`・`nocapture: yes`・`filter: (none: whole binary)`・`test_threads: (not passed)`）。5.1 の測り方の順のとおり、静かな机の 3 回と全体テストの後に続けて回した（`target\load-flake\after-run-20261008-073435\progress.txt`）。静かな机の 3 回と全体テストの所要時間は、タスク 6.1 が 5 に書く。

#### 条件

| 項目 | 直した後（`conditions.txt`・`summary.txt`） | 直す前（3.1） |
|---|---|---|
| コミット | `498446d7`（`dirty: no`）。タスク 4.4 までの直しが入ったコミット | `d6d77336` |
| 回したもの | `cargo test -p areka --bin areka -- --nocapture`（2,814 本・無視 2 本。下の「テストの本数」） | 2,802 本・無視 2 本 |
| 時刻 | 2026-10-08 07:42:39〜08:06:32 | 2026-10-08 00:12:57〜00:56:56 |
| 論理 CPU・負荷の子 | 22・44 本（止めた後の残り 0） | 22・44 本（残り 0） |
| 負荷の前の CPU | 読めなかった。`cpu_before_load: n/a (分母の値が負のカウンターが検出されました。)`（性能カウンターの読みが失敗した文をそのまま残した）。`progress.txt` では、全体テストが終わった秒（07:42:39）にそのまま始まっている | 平均 2.5%・最大 5.2% |
| 負荷の最中の CPU | 平均 90.9%・最大 91.0%（5 回の読み） | 平均 91.0%・最大 92.6% |

負荷の前の CPU は比べられない。負荷の最中の CPU は直す前とほぼ同じ。

#### 回ごとのまとめ

| 回 | 所要時間（`wall_sec`） | うち cargo の準備（ログの 1 行目の `Finished … in`） | libtest の時間（`finished in`） | 結果 | 赤 | libtest の「60 秒を越えて走っている」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 294.2 | 1.37 秒 | 251.74 秒 | 2,811 通過・1 失敗・2 無視 | 1 | 8 行 |
| 2 | 292.7 | 2.48 秒 | 270.15 秒 | 2,811 通過・1 失敗・2 無視 | 1 | 10 行 |
| 3 | 272.1 | 2.40 秒 | 243.98 秒 | 2,811 通過・1 失敗・2 無視 | 1 | 12 行 |
| 4 | 263.1 | 9.46 秒 | 234.16 秒 | 2,811 通過・1 失敗・2 無視 | 1 | 10 行 |
| 5 | 285.3 | 2.64 秒 | 259.89 秒 | 2,811 通過・1 失敗・2 無視 | 1 | 10 行 |
| **計** | 中央値 285.3・最大 294.2 | | 中央値 251.74・最大 270.15 | 5 回とも赤 | **5** | 50 行 |

- 5 回とも上限（30 分）を越えず（`rounds_timed_out: 0`）、どの回も `test result:` の行まで出て終わった。
- 赤は 5 回とも同じ 1 本 `mcp::dump_balloon::dump_balloon_gpu_tests::hidden_balloon_returns_the_same_pixels`（各回の末尾の `failures:` の一覧と `summary.txt` の `failed_tests_by_name` が一致）。
- 直す前（3.2・3.7）と並べると、赤は 81 件・52 種類から 5 件・1 種類になった。直す前に赤になった 52 種類のうち、この 1 本を除く 51 種類は、5 回とも `test 名前 ... ok` の行がある（直す前の `summary.txt` の `failed_tests_by_name` の名前を、直した後の各回の `round-N.log` で引いた）。直す前に 3〜5 回目にだけ出た降ろしの打ち切り（A・B・C）、spine の族の待ち（3.4 の場所 32〜34）、待ちでない赤（場所 24）も、5 回とも出ていない。
- 所要時間は、`wall_sec` の中央値が 576.5 → 285.3 秒・最大が 819.2 → 294.2 秒、libtest の時間の中央値が 265.37 → 251.74 秒・最大が 510.33 → 270.15 秒。「60 秒を越えて走っている」の知らせは 132 → 50 行。

#### 待ちの打ち切りの行

`summary.txt` の `wait_cut_lines` は 5 回とも 3 行（計 15 行）。`round-N.log` の全体を、行のどこかにある文言の先頭で数えると、`［止まった］` 0・`［進んではいた］` 0・`［進みは不明］` 各回 1・`［相手が居ない］` 各回 2 で、`summary.txt` の 3 行と同じもの。

| 行 | 出したところ | 読み |
|---|---|---|
| `待ちの打ち切り［相手が居ない］: 「落ちる処理」— …秒待ったところで、相手が何も送らずに終わった` | 檻のテスト `emo2_boot::spine::wait_tests::a_vanished_worker_panics_with_the_partner_gone_text` のスレッドの `panicked at crates\areka\src\emo2_boot\spine_wait.rs:311:9`（`run_bounded_watching` の panic の文） | 緑のテストがわざと起こす文言（`catch_unwind` で受けて、文の先頭を確かめる）。5 回とも、このテストに `... ok` の行がある。静かな机の 3 回（`target\load-flake\quiet-after-20261008-073437\summary.txt`）にも毎回同じ 2 行がある |
| `'落ちる処理' did not complete within 30s (possible hang) — 待ちの打ち切り［相手が居ない］: …` | 同じテストのスレッドの `panicked at …\spine_wait.rs:358:9`（`run_bounded` の panic の文） | 同上 |
| `待ちの打ち切り［進みは不明］: 「crates\areka\src\mcp\dump_surface_gpu_test_support.rs:114」— 30.0 秒までに届かなかった（進みの目印の無い待ち）` | `GpuRig::frames_until` の中の `spin_wait_until`（114 行はその呼び出しの行） | 残った赤の待ち（下の「残った赤の読み」） |

- 行の場所: `［相手が居ない］` の 2 行は 806・850 行（1 回目）、810・868 行（2 回目）、804・845 行（3 回目）、801・842 行（4・5 回目）。`［進みは不明］` は 1783・2196・1817・1831・2226 行。5 回目の 2226 行は、他のテストの `test placement::follow::visibility_char_wiring_tests::undetermined_old_size_is_treated_as_unknown_rect_and_clamps ...` の後ろに割り込んでいるが、`tools/load-flake.ps1` は行のどこかで拾うので数に入っている。
- 赤でない `panicked at` の行は各回 12 行（各回の 13 行から赤の 1 行を除く）。直す前の毎回の 8 行（3.7）と同じ場所の 8 行に、上の檻の 4 行（`spine_wait.rs:311`・`:358` と、わざと落とす名前の無いスレッドの `spine_wait_tests.rs:282`・`:291`）が足されたもの。直す前の 4 回目にあった `spine_hold_support.rs:90` の 2 行は無い。

#### 残った赤の読み

| 回 | `panicked at` の行 | `［進みは不明］` の行 | 偽になった値 |
|---|---|---|---|
| 1 | 1791 | 1783 | 「隠れた」 |
| 2 | 2261 | 2196 | 「隠れた」 |
| 3 | 1895 | 1817 | 「隠れた」 |
| 4 | 1939 | 1831 | 「隠れた」 |
| 5 | 2338 | 2226 | 「隠れた」 |

- 場所は `mcp\dump_balloon_gpu_tests.rs:312:5`（3.4 の場所 29 と同じ）。文言 `（字が現れ切った, 隠す前の可視, 隠れた, 降ろせた, 前の答え, 後の答え, 前後で食い違った画素の数）` の `left` と `right` で食い違うのは「隠れた」（`false`）だけで、5 回とも同じ形（字が現れ切った `true`・隠す前の可視 `Some(true)`・降ろせた `true`・前と後の答えはどちらも `OK:balloon of scope 0 as last drawn …`・食い違った画素 `Some(0)`）。「隠れた」は `gpu.frames_until(1, |world| balloon_visible(world) == Some(false))` の返り値で、待ちは 3.3 の H（`GpuRig::frames_until`・30 秒）。
- 待ちの文言との結び付け: `［進みは不明］` の行には、スレッドの名前も呼び手の場所も出ない（`frames_until` に `#[track_caller]` が無いので、場所はどの呼び手でも `frames_until` の中の 114 行になる）。それでも、`spin_wait_until`（`spine_wait.rs`）は `false` を返す前に必ず 1 行出すので、「隠れた」が `false` だった回には `［進みは不明］` の行が 1 行以上ある。どの回もその行はちょうど 1 行なので、その 1 行がこの赤の待ちの行。並びも、どの回もその行は赤の `panicked at` の行より前（8〜112 行前）にある。
- 読み分けの表（design「LoadRepro」の、直した後に赤が出たときの読み方）に当てると、先頭は `［進みは不明］`＝「目印の無い待ちが 30 秒」で、次にすることは「その待ちに目印を渡せるかを調べ、対象に加える」。
- 行き先は **タスク 5.4**。`GpuRig::frames_until` は 2.3 の「移さない直接の呼び出し」に挙げ、「再現でこの族が赤になったら同じ移す先を使う（タスク 5.4）」とした待ちで、1.4 でも 2 回赤だった（3.7 の「設計の対象の族に入っていない赤」）。5.4 の条件（1.4 か 5.1 の記録で赤になったのに 4.x で移していない待ちのファイル）に当たる。
- タスク 5.2（足場の同時の数）には回さない。`［進んではいた］` の赤は 0 件（5 回の `round-N.log` の全体で 0 行）。
- タスク 5.3（本番の順序の取り違え）には回さない。待ちの文言の無い赤は 0 件（赤の 5 件は、どれも自分の待ちの `［進みは不明］` の行がある）、`［止まった］` の赤も 0 件。5.2・5.3 の条件は、この回の記録では満たされない。
- 直す前より増えた理由の読み（静的。反復の数は記録に出ないので、5.4 で確かめる）: 直す前はこの 1 本の赤は 5 回のうち 2 回（1・4 回目）、直した後は 5 回とも。`GpuRig::frames_until` は 1 巡で合成の時刻を高々 `step_ms`（このテストは 1 ms）しか進めない（`TICK_EVERY` 1 ms ごとに Tick を 1 つ・`dump_surface_gpu_test_support.rs`）。台本 `LINE_THEN_HIDE`（`\0\s[1101]表示の確かめ\_w[1500]\b[-1]\e`）は `\_w[1500]` の後で隠すので、隠れるまでに合成の時刻で 1,500 ms、つまり 1,500 巡以上が要る（30 秒のうちに 1 巡 20 ms 以下）。直す前の `spin_wait_until`（`d6d77336` の `spine.rs`）は `yield_now` を 1,000,000 回（`SPIN_YIELD_BUDGET`）空回ししてから休みに落ちたので、`Input`・`Update` の段と GPU の描画を回すこの重い巡では、30 秒のうちに休みへ落ちなかった。直した後の芯は、60 ms（`DENSE_SPIN`）を過ぎると巡ごとに 1 ms（`BACKOFF_SLEEP`）休む。負荷の下では 1 ms の休みが実際には長くなりうるので、巡の数が減り、合成の時刻が 1,500 ms に届かなかった、と読める。芯の doc（`spine_wait.rs` の `SPIN_WAIT`）が「非対象」に挙げる「注入 Tick 列そのものが仕事量であるループ」と同じ形の待ち。目印に何を渡すか（相手の進みか、注入した合成の時刻か）と合わせて、5.4 で決める。
- 赤の文言にファイルの拒否・`os error` は無い。このテストは作業フォルダを使わない（文言にパスが出ない）。

#### 後片付けの失敗の行（os error 5）

数え方は 6 の「直す前と後の数え方」のとおり。`summary.txt` の `cleanup_lines`（行のどこかの `sample-ghost-kit:`）・`（次の走行で回収する）` の数・`アクセスが拒否されました` の数は、どの回も一致した。

| 記録 | 回 | 行 | うち os error 5 | 行の場所・種類・指す木 |
|---|---|---|---|---|
| 負荷の下（`after-20261008-074239`） | 1・3・4・5 | 0 | 0 | — |
| 負荷の下 | 2 | 1 | 1 | 120 行・残骸の退避・`work\27596-1` |
| 静かな机（`quiet-after-20261008-073437`） | 1 | 0 | 0 | — |
| 静かな机 | 2 | 1 | 1 | 122 行・残骸の退避・`work\35228-159` |
| 静かな机 | 3 | 1 | 1 | 123 行・退避した残骸の削除・`work\gc-16884-1` |
| **計** | | 負荷の下 1・静かな机 2 | 負荷の下 1・静かな机 2 | |

6 の見分けの表の行に当てて、直す前（6 の表）と並べる。

| 行の種類 | 指す木 | 静かな机 3 回（前 → 後） | 負荷の下 5 回（前 → 後） | 見分けの表の行 |
|---|---|---|---|---|
| 残骸の退避 | `work\<その回のプロセスの番号>-<連番>` | 623 → 0 | 344 → 0 | 1 |
| 残骸の退避 | `work\gc-…` | 4 → 0 | 0 → 0 | 2 |
| 残骸の退避 | `work\<前の回のプロセスの番号>-<連番>` | 1 → 1（`35228-159`） | 0 → 1（`27596-1`・読み） | 3 |
| 退避した残骸の削除 | `work\gc-…` | 4 → 1（`gc-16884-1`・読み） | 2 → 0 | 前は 1・2 の巻き添え。後の 1 行は 3（下） |
| 作業フォルダの後始末 | `work\<その回のプロセスの番号>-<連番>` | 1 → 0 | 1 → 0 | 1 の巻き添え |
| 生存の札の後始末 | `work\<その回のプロセスの番号>-<連番>.lock` | 0 → 0 | 2 → 0 | 1 の巻き添え |
| 計 | | 633 → 2 | 349 → 1 | |

- 3 行とも、回の始まり（ログの 120〜123 行。負荷の下の 2 回目は、それより前に `... ok` の行が 115 行）に出ている。直す前の表の 3 の 1 行（静かな机の 2 回目の 123 行・前の回のプロセスの木）と同じ位置。
- `work\35228-159`: 連番 159 は、プロセスが終わるまで札を握る複製の連番と同じ。これを読んだ時点の棚（`target\nar-samples\work\`）には、このワークツリーで最後に回ったテストのプロセスの木が `24984-8`・`24984-159`・`24984-161`・`24984-162`（それぞれ札つき）の形で残っている。静かな机の 1 回目のプロセスが残した木を、2 回目のプロセスの掃除が退けようとして拒まれた形と読む（表の 3）。
- `work\27596-1`（読み）: 番号 27596 は、この回のログの他の所に出ないので、どのプロセスの木かは記録からは決められない。ただ、直した後の決まりでは、その回のプロセスの木は、消し終えるまで札が握られる（6 の「直したこと」・檻 11）ので掃除の相手にならない。例外は `Drop` の削除が失敗して残った木で、そのときは先に「作業フォルダの後始末」の行が出るが、この回には無い。だから前の回（1 回目）のプロセスの木と読む（表の 3）。
- `work\gc-16884-1`（読み）: `gc-` の木の名前は、退けたプロセス自身の番号と連番（`devroot.rs` の `discard_tree_with`）。連番 1 なので、3 回目のプロセスが回の始まりに初めて退けた木（前の回の残りと読める）で、その木を消すときに拒まれた。直した後は、`gc-` の木は消し終えるまで札が握られ、並走する掃除は触らない（檻 12）ので、テストの側の妨げ合いの形ではない（表の 3）。
- 表の 3 の行が、2 本の掃除の重なりか、ウイルス対策などの外の手かは、直す前（6 の「表の 3 に当たる 1 行」）と同じく、記録からは分けられない。どちらでも次の取得の掃除が片付けるので、直す対象にはしない（要件 5.3）。
- 表の 1・2 とその巻き添え（直す前は静かな机 632 行・負荷の下 349 行）は、直した後は 0 行。
- 赤との関わり: 負荷の下の 1 行（2 回目の 120 行）は、その回の赤（2261 行）とは別のテストのもので、赤の文言にパスやファイルの拒否は出ない。

#### テストの本数（要件 2.4）

- `cargo test -p areka --bin areka -- --list`（`target\load-flake\after-run-20261008-073435\list.txt`）は `: test` の行が 2,814 行（`progress.txt` の `areka bin test count=2814`）。5 回の `test result:` も、2,811 通過＋1 失敗＋2 無視＝2,814 本。
- 直す前の `target\load-flake\before-run-20261007-235604\list.txt`（2,802 本・5.3）と名前で比べると、直す前の 2,802 本はすべて直した後にもある（消えた名前は 0）。増えた 12 本は、この spec の檻:
  - `emo2_boot::spine::wait_tests::` の 8 本（`a_reached_condition_succeeds_however_late`・`a_moving_partner_is_never_cut_off_before_the_cap`・`a_partner_that_stops_moving_is_reported_as_stalled`・`an_unmarked_wait_times_out_at_the_same_total_as_before`・`the_wait_starts_returning_the_cpu_after_the_dense_spin_window`・`the_four_failures_read_differently`・`a_receiver_returns_the_value_or_reports_a_vanished_sender`・`a_vanished_worker_panics_with_the_partner_gone_text`）
  - `emo2_boot::ghost_switch_test_support::talk_clock_tests::a_rig_clock_far_ahead_of_a_stalled_handshake_leaves_the_switch_unchanged`（檻 8）
  - `emo2_boot::ghost_switch_test_support::rig_wait_tests::` の 2 本（`a_rig_wait_that_never_arrives_names_the_callers_place`・`the_rig_progress_ignores_status_and_counts_every_factory_call`。檻 9・10）
  - `emo2_boot::spine::hold_support::tests::call_count_skips_status_queries_and_counts_every_other_call`
- 無視は 2 本のまま（5 回の `test result:` が `2 ignored`）。`git diff d6d77336 498446d7 -- crates/areka/src` に `#[ignore` を足し引きした行は 0。テストの削除や無視の印で赤を消してはいない。

### 4.2 直した後の 2 回目（タスク 5.4 の後）

記録 `target\load-flake\after-20261008-084021\`（`conditions.txt`・`build.log`・`round-1.log`〜`round-5.log`・`summary.txt`）。タスク 5.4 で `GpuRig::frames_until` の待ちを対象に加えた後に、4.1 と同じ引数 `-Label after -Rounds 5 -Burners 44 -RoundTimeoutMin 30 -NoCapture` で回した（`conditions.txt` の `rounds: 5`・`burners: 44`・`round_timeout_min: 30`・`nocapture: yes`・`filter: (none: whole binary)`・`test_threads: (not passed)`）。4.1 と同じく、静かな机の 3 回（`target\load-flake\quiet-after-20261008-083222\`）と全体テストの後に続けて回した（`target\load-flake\after-run-20261008-083203\progress.txt`）。

#### 条件

| 項目 | 2 回目（`conditions.txt`・`summary.txt`） | 1 回目（4.1） |
|---|---|---|
| コミット | `893fab43`（`dirty: no`）。タスク 5.4 までの直しが入ったコミット | `498446d7` |
| 回したもの | `cargo test -p areka --bin areka -- --nocapture`（2,814 本・無視 2 本。下の「テストの本数」） | 2,814 本・無視 2 本 |
| 時刻 | 2026-10-08 08:40:21〜09:13:44 | 2026-10-08 07:42:39〜08:06:32 |
| 論理 CPU・負荷の子 | 22・44 本（止めた後の残り 0） | 22・44 本（残り 0） |
| 負荷の前の CPU | 読めなかった。`cpu_before_load: n/a (分母の値が負のカウンターが検出されました。)`。`progress.txt` では、全体テストが終わった秒（08:40:21）にそのまま始まっている | 読めなかった（同じ文） |
| 負荷の最中の CPU | 平均 91.6%・最大 97.7%（5 回の読み） | 平均 90.9%・最大 91.0% |

負荷の前の CPU は 1 回目と同じく比べられない。負荷の最中の CPU は平均でほぼ同じ。

#### 回ごとのまとめ

| 回 | 所要時間（`wall_sec`） | うち cargo の準備（ログの 1 行目の `Finished … in`） | libtest の時間（`finished in`） | 結果 | 赤 | libtest の「60 秒を越えて走っている」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 362.7 | 10.79 秒 | 327.16 秒 | 2,812 通過・0 失敗・2 無視 | 0 | 29 行 |
| 2 | 389.8 | 1.88 秒 | 345.21 秒 | 2,810 通過・2 失敗・2 無視 | 2 | 38 行 |
| 3 | 384.6 | 11.16 秒 | 323.28 秒 | 2,809 通過・3 失敗・2 無視 | 3 | 32 行 |
| 4 | 348.3 | 2.37 秒 | 303.19 秒 | 2,810 通過・2 失敗・2 無視 | 2 | 29 行 |
| 5 | 488.4 | 1.94 秒 | 406.32 秒 | 2,796 通過・16 失敗・2 無視 | 16 | 51 行 |
| **計** | 中央値 384.6・最大 488.4 | | 中央値 327.16・最大 406.32 | 4 回が赤（1 回目は緑） | **23** | 179 行 |

- 5 回とも上限（30 分）を越えず（`rounds_timed_out: 0`）、どの回も `test result:` の行まで出て終わった。
- 「60 秒を越えて走っている」の知らせは、`round-N.log` の `has been running for over 60 seconds` の行の数（4.1 の 8・10・12・10・10 行も同じ数え方で数え直して一致を確かめた）。
- 赤は 16 種類（3 回が 3 種類・2 回が 1 種類・1 回だけが 12 種類＝3×3＋2×1＋12×1＝23 件）。各回の末尾の `failures:` の一覧と `summary.txt` の `failed_tests_by_name` が一致する。
  - 3 回: `emo2_boot::ghost_switch::balloon_tests::switching_to_a_ghost_with_numbered_companions_uses_the_first_and_records_the_companion_route`・`emo2_boot::ghost_switch::boot_event_tests::a_switch_with_a_boot_event_boots_the_target_with_that_event_instead_of_changed_or_boot`・`…::a_switch_without_a_boot_event_boots_the_target_with_on_ghost_changed_as_today`
  - 2 回: `emo2_boot::frame::ghost_quit_switch_tests::stop_with_handoff_under_reservation_switches_without_exit`
  - 1 回（どれも 5 回目）: `ghost_switch::balloon_tests::switching_to_a_ghost_whose_descript_names_a_balloon_uses_it_and_records_the_descript_route`・abort の 2 本（`ghost_switch_after_a_shell_swap_unloads_the_session`・`ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind`）・`fallback_tests::target_fault_without_default_ghost_is_fatal`・`install::desk::record_tests::a_shell_install_only_records_names`・`install::worker::worker_tests::production_path_sends_no_legacy_complete_after_a_complete_ex_script`・`mcp::dump_balloon::dump_balloon_gpu_tests::` の 3 本（`balloon_at_dpi_144_keeps_native_size_and_text_position`・`balloon_job_runs_on_the_encoding_body_without_error`・`scope_two_is_no_such_scope_and_calls_do_not_disturb_the_ghost`）・`mcp::dump_surface::dump_surface_gpu_tests::` の 3 本（`before_attachment_answers_after_the_frames_run`・`calls_do_not_disturb_the_ghost`・`hidden_character_returns_the_picture_before_hiding`）
- 16 種類のうち 8 種類（boot_event の 2 本・`stop_with_handoff_under_reservation_switches_without_exit`・descript のバルーンの 1 本・abort の 2 本・`target_fault_without_default_ghost_is_fatal`・`balloon_at_dpi_144_keeps_native_size_and_text_position`）は直す前の記録（3.7）でも赤だった。1 回目（4.1）ではどれも赤でない。前からある揺れが、1 回目では出なかったと読む。
- 5.4 で直した `mcp::dump_balloon::dump_balloon_gpu_tests::hidden_balloon_returns_the_same_pixels` は、5 回とも `... ok`。
- 所要時間は 1 回目より長い（`wall_sec` の中央値 285.3 → 384.6 秒・最大 294.2 → 488.4 秒、libtest の時間の中央値 251.74 → 327.16 秒・最大 270.15 → 406.32 秒、「60 秒を越えて走っている」の知らせ 50 → 179 行）。

#### 待ちの打ち切りの行

`summary.txt` の `wait_cut_lines` は 2・4・6・4・18 行（計 34 行）。`round-N.log` の全体を、行のどこかにある文言の先頭で数えると次のとおりで、`summary.txt` と同じもの。

| 文言の先頭 | 1 回目 | 2 回目 | 3 回目 | 4 回目 | 5 回目 | 計 |
|---|---|---|---|---|---|---|
| `［止まった］` | 0 | 2 | 4 | 2 | 16 | 24 |
| `［進んではいた］` | 0 | 0 | 0 | 0 | 0 | 0 |
| `［進みは不明］` | 0 | 0 | 0 | 0 | 0 | 0 |
| `［相手が居ない］` | 2 | 2 | 2 | 2 | 2 | 10 |

- `［相手が居ない］` の各回 2 行は、4.1 と同じ檻のテスト（`a_vanished_worker_panics_with_the_partner_gone_text`）がわざと起こすもの。
- 4.1 の `［進みは不明］`（`GpuRig::frames_until`）は 0 行になった（5.4 で芯へ移した）。
- `［止まった］` の 24 行を待ちの種類で分けると次のとおり（`what` は文言の「」の中。足場の待ちは呼び出しの場所が入る）。

| 待ち | `what` と回 | 行 | 進み 0 回 | 進み 1 回以上 |
|---|---|---|---|---|
| 起こした直後の定常の待ち（`SwitchRig::wait_steady`・`frame_ghost_quit_switch_tests.rs` の自前の待ち「A の定常到達」） | `ghost_switch_balloon_tests.rs:39`（2・4・5 回目×2）・`ghost_switch_boot_event_tests.rs:62`（2・3・4 回目）・`install\worker_tests.rs:466`（5 回目）・「A の定常到達」（3 回目） | 9 | 9 | 0 |
| 切替の途中の段を回す待ち（`pump_until`・`pump_talking_until`・`LapRig::frames_until`） | `frame_ghost_quit_switch_tests.rs:140`（3・5 回目）・`ghost_switch_boot_event_tests.rs:81`（3 回目・5 回目×2）・`ghost_session_switch_fallback_tests.rs:207`（5 回目）・`shell_balloon_switch_session_abort_tests.rs:417`・`:485`（5 回目） | 8 | 4 | 4（進み 2〜4 回・待ち始めから 58.7〜71.1 秒） |
| 降ろしの待ち（`SwitchRig::shutdown`「置き場のゴーストを降ろす」） | 5 回目×5 | 5 | 1 | 4（進み 2〜7 回・待ち始めから 30.0〜41.0 秒） |
| GPU の足場の答えの待ち（`GpuRig` の `answer_of`） | `mcp\dump_balloon_gpu_tests.rs:393`・`:243`（5 回目） | 2 | 2 | 0 |
| 計 | | 24 | 16 | 8 |

- 進み 0 回の 16 行は、どれも「待ち始めから 30.0 秒」（進みの無い時間の上限ちょうど）。待つ側は時間どおりに起きて打ち切っている。
- 赤との結び付け: 23 件の赤は、どれも自分の待ちの `［止まった］` の行を持つ（3 回目の `stop_with_handoff_under_reservation_switches_without_exit` は「A の定常到達」と `:140` の 2 行、ほかは 1 行ずつ）。18 件は `bool` を返す待ちが出した行で、その赤の `panicked at` の行より前にある。5 回目の降ろしの 5 件は `spine_wait.rs:311:9`（`run_bounded_watching` の panic）で、行は panic の文言そのもの（`panicked at` の次の行）。そのテストは `dump_balloon_gpu_tests` の `balloon_job_runs_on_the_encoding_body_without_error`、`dump_surface_gpu_tests` の 3 本、`install::desk::record_tests::a_shell_install_only_records_names`。
- 時刻の固まり: 2〜4 回目の `［止まった］` の行はどれもログの 604〜667 行、5 回目は 609〜658 行に 5 行・1609〜1728 行に 10 行・2265 行に 1 行で、切替・GPU・インストールの別々のテストが同じ時間帯にまとめて止まっている。
- 待ちの文言の無い赤・`［進んではいた］` の赤・`［進みは不明］` の赤は 0 件。

#### 赤の読み

**負荷の下の 23 件（`［止まった］`）: テストのプロセスの中のスレッドの生まれと消えの取り合い。行き先はタスク 5.2（改めた要件 2.7）。**

- 進み 0 回の意味: 足場がゴーストを起こす `boot_ghost` が戻ってから、kanade が定常の知らせ（`KanadeNotice::Steady`）を送るまでの起動の鎖（`crates/areka-kanade/src/schedule/boot.rs`: `OnInitialize` の通知 → `username` の取得 → `OnBoot` → `basewareversion` → 定常）は、どの段も偽の SHIORI（`ScriptedShioriBackend`）の呼び出しで、足場の目印に数えられる。進み 0 回で 30 秒は、新しく起こした kanade と SHIORI のスレッドが Rust のコードを 1 行も走らせられなかったことを意味する。
- スレッドのスタック（調べのために別に回した記録。コードは変えていない）: 同じコミット・同じ負荷（`-Rounds 3`）の再現 `target\load-flake\dbg-stacks-20261008-092736\` の最中に、テストのプロセスの全部のスレッドのスタックを 20 秒前後ごとに取った（`target\dbg-stacks\run1\scan.log` と、その中の 3 回を書き出した `dump-1.txt`〜`dump-3.txt`）。

| 書き出し | 時刻 | スレッド | 始まり（`LdrInitializeThunk`）の中で `NtWaitForSingleObject` に止まっている | 終わり（`LdrShutdownThread`）を含む | 描画装置のドライバ（`igd10umt64xe.DLL`）を含む | `TaskPool (n)` |
|---|---|---|---|---|---|---|
| `dump-1.txt` | 09:28:55 | 223 | 6 | 9 | 52 | 110 |
| `dump-2.txt` | 09:30:01 | 269 | 18 | 8 | 132 | 66 |
| `dump-3.txt` | 09:37:37 | 360 | 11 | 15 | 210 | 88 |

  - Windows では、スレッドの始まりと終わりがプロセスに 1 つのローダーの錠を取る。始まりで止まっているスレッドは、その錠を待っている。同じスレッドが続けて取った 2 回の読みでも始まりで止まったままだった（`dump-1.txt` の頭の `persist=[34484]` は 09:28:34 と 09:28:55 の読み、`dump-2.txt` の `persist=[12624]` は 09:29:31 と 09:30:01 の読みで同じスレッド）。`scan.log` の読みでは、プロセスのスレッドは最大 552 本。
  - 生まれと消えの多い出どころ: GPU のテストの D3D11 の装置ごとの描画装置のドライバのスレッド（`wintf` の `GraphicsCore` は、テストの debug ビルドでは装置を確かめの層つきで作る）・`WintfTaskPool::new()`（`crates/wintf/src/ecs/widget/bitmap_source/task_pool.rs`）が論理 CPU の数（22 本）だけ作るスレッド（閉包を 1 つも走らせない足場でも作る）・起こしたゴーストごとのアクターのスレッド。
- 待つ側の飢えではない: 進み 0 回の 16 行はどれも待ち始めから 30.0 秒ちょうどで、待つ側は時間どおりに起きている。止まった赤は、関係の無いテストどうし（切替・GPU・インストール）で同じ時間帯に固まって出る（上の「時刻の固まり」）。プロセス全体の錠の列と合う。
- 切替の途中・降ろしの待ち（進み 0〜7 回）も同じ読み: 降ろすときのスレッドの終わりと、切替で起こし直すときのスレッドの始まりが、同じ錠を待つ。
- 対照: 同じ負荷で `-Filter ghost_switch` だけを回した `target\load-flake\dbg-switchonly-20261008-094253\`（50 本・5 回・`-RoundTimeoutMin 10`）は、5 回とも緑で `wait_cut_lines` は 0。libtest の時間は 186.54・158.04・110.84・247.73・107.00 秒。切替のテストだけなら、負荷の下でも遅いだけで止まらない。
- 注意: スタックを取った再現の 2 回目は 47.4 秒で `0xc0000005`（`exit_code: -1073741819`）で終わった。既知の `areka-test-threads-av` か、スタックを取る道具の揺さぶりかは分けられない。1・3 回目の赤（`ghost_session::restart_tests::boots_twice_in_one_process_without_double_registration` の「1 周目を降ろす」20 秒・`emo2_boot::spine::text_scale_tests::spine_dpi_change_while_balloon_hidden_lands_on_next_show` の「spine seriko join」10 秒、どちらも `［進みは不明］`）も含めて、この記録は調べのためだけに使い、直した後の数には入れない。
- 1 回目（4.1）で出なかった理由の見立て（確かめていない）: 5.4 で GPU のテストの待ちが長くなり、GPU のテストが切替のテストと重なる時間が増えた。
- 本番の順序の取り違えは見当たらない（0 回の止まりは相手が 1 度も動けなかったもので、相手の処理の順序を見る材料が無い）。タスク 5.3 はこの結論で閉じる。
- 要件 2.7（改めた文）の条件に当たる: 待ち方の直しの後に、6.1 の負荷の条件で待ちの打ち切りの赤が残り、文言は `［止まった］` で、相手のスレッドが始まれなかった・終われなかった根拠（上のスタック）がある。行き先はタスク 5.2（作業のプールのスレッドの数と、GPU の装置の同時の数を絞る。2026-10-08 に開発者が選んだ案 A）。`［進んではいた］` の赤は 0 件なので、足場の同時の数の許可（RigPermit）は入れない。

**静かな机の 1 件（文言なし）: テストの台本の競り合い。行き先はタスク 5.5。**

- 同じコミットの静かな机の 3 回（`target\load-flake\quiet-after-20261008-083222\`）の 3 回目に、赤が 1 件あった（1・2 回目は緑）。`mcp::dump_surface::dump_surface_tests::switch_to_the_same_ghost_keeps_waiting_for_attachment` が `dump_surface_tests.rs:579:5` で、`（A の定常, 起こし直した A の定常まで届いた, A の起動の回数, …）` の 3 つ目が `left` 3・`right` 2（`round-3.log` の 2788 行）。待ちの打ち切りの行は無い（各回の 2 行は檻の `［相手が居ない］`）。
- 原因: どの起動でも台本 `\0A\![change,ghost,A]\e` を使う。起こし直した A は `OnGhostChanged` に 204 を返すので `OnBoot` へ進み、同じ切替の頼みをもう一度流す。その台詞は定常より先に始まり、定常で切替の予約（`SwitchInFlight`）が外れたのと同じ段の回りで頼みが汲み出されると受け付けられ、3 度目の起動になる。待ちの届いた条件（`calls >= 2` かつ予約なし）が後戻りしない観測になっていない（要件 2.6）。テストの側の競り合いで、本番の順序の取り違えではない。
- 直し方（タスク 5.5・テストだけ）: A に `\0A\e` を返す `OnGhostChanged` を持たせ、起こし直した A が何も頼まないようにする。確かめは変えない。
- この静かな机の 3 回と全体テスト（緑・286 秒・`after-run-20261008-083203\progress.txt` の `test-all exit=0 sec=286`）の所要時間の比べは、タスク 6.1 が 5 に書く。

#### 後片付けの失敗の行（os error 5）

数え方は 4.1 と同じ。`summary.txt` の `cleanup_lines`・`（次の走行で回収する）` の数・`アクセスが拒否されました` の数は、どの回も一致した。

| 記録 | 回 | 行 | うち os error 5 | 行の場所・種類・指す木 |
|---|---|---|---|---|
| 負荷の下（`after-20261008-084021`） | 1・3・5 | 0 | 0 | — |
| 負荷の下 | 2 | 1 | 1 | 121 行・残骸の退避・`work\9028-1` |
| 負荷の下 | 4 | 1 | 1 | 122 行・退避した残骸の削除・`work\gc-26480-1` |
| 静かな机（`quiet-after-20261008-083222`） | 1 | 1 | 1 | 125 行・退避した残骸の削除・`work\gc-9996-0` |
| 静かな机 | 2・3 | 0 | 0 | — |
| **計** | | 負荷の下 2・静かな机 1 | 負荷の下 2・静かな机 1 | |

- 3 行とも回の始まり（ログの 121〜125 行。それより前の `... ok` の行は 116・117・120 行）に出ている。4.1 の 3 行と同じ位置。
- 番号 9028・26480・9996 は、それぞれのログの他の所に出ないので、どのプロセスの木かは記録からは決められない。4.1 と同じく、直した後の決まり（6 の「直したこと」・檻 11・12）では、その回のプロセスの木と `gc-` の木は消し終えるまで札が握られるので、前の回の残りへの掃除の重なりか外の手（見分けの表の 3）と読む。直す対象にはしない（要件 5.3）。
- 見分けの表の 1・2（同じプロセスの中の妨げ合い）に当たる行は 0 行（直す前は静かな机 632 行・負荷の下 349 行）。
- 赤との関わり: ログの中の `アクセスが拒否されました` は、上の後片付けの行の分だけ。23 件の赤の文言にファイルの拒否は出ない。

#### テストの本数（要件 2.4）

- `cargo test -p areka --bin areka -- --list`（`target\load-flake\after-run-20261008-083203\list.txt`）は `: test` の行が 2,814 行（`progress.txt` の `areka bin test count=2814`）。名前の一覧は 4.1 の `list.txt` と同じ（差 0）。
- 5 回の `test result:` も、通過＋失敗＋無視（2）＝2,814 本。
- `git diff 498446d7 893fab43 -- crates/areka/src` に `#[ignore` を足し引きした行は 0。テストの削除や無視の印で赤を消してはいない。

タスク 5.1 の完了の姿（待ちの打ち切りの赤 0 件）は、この 2 回目では満たされない。タスク 5.2（スレッドの絞り）と 5.5（台本の競り合い）の後に、同じ引数で 3 回目を回して 4.3 に書く。

### 4.3 直した後の 3 回目以降（タスク 5.2・5.5・5.4 の開き直しの後と main の取り込みの後）

どれも 2026-10-08 の記録で、すべて `target\load-flake\` の下にある。負荷の下の引数は 4.1 と同じ（各回の `conditions.txt` の `rounds: 5`・`burners: 44`・`round_timeout_min: 30`・`nocapture: yes`・`filter: (none: whole binary)`・`test_threads: (not passed)`）。静かな机は `-Rounds 3 -Burners 0`。5.1 の測り方の順（先にビルド → 本数 → 静かな机の 3 回 → 全体テスト → 負荷の下）で続けて回し、その順と全体テストの時間は各回の `after-run-…\progress.txt` に残る。GPU の装置の許可の数は、各コミットの `crates/areka/src/emo2_boot/spine_wait.rs` の `GPU_SLOTS` の値。

#### 回しの一覧

| # | 回し | コミット（`conditions.txt` の `commit:`・どれも `dirty: no`） | 許可 | 本数 | 静かな机の中央値（`wall_sec`） | 全体テスト | 負荷の下の赤（1〜5 回目） | 順の記録 |
|---|---|---|---|---|---|---|---|---|
| 1 | タスク 5.2 を入れた直後 | `0d692ec8` | 2 | 2,815 | 95.2 秒 | 322 秒・緑 | 0・（落ちた）・0・0・0 | `after-run-20261008-105822` |
| 2 | 許可を 8 に | `e69d92b1` | 8 | 2,815 | 64.7 秒 | 301 秒・緑 | 0・1・0・0・1 | `after-run-20261008-124705` |
| 3 | 許可を 4 に | `1f494325` | 4 | 2,815 | 72.3 秒 | 283 秒・緑 | 0・0・0・1・0 | `after-run-20261008-134510` |
| 4 | タスク 5.4 の開き直しの後（負荷の途中で止めた・使わない） | `24aa8fc9` | 4 | 2,815 | 111 秒（机が静かでなかった） | 477 秒・緑 | 0・（落ちた）・0・（止めた） | `after-run-20261008-145914` |
| 6 | main の取り込みの後の 1 回目 | `14516a4e` | 4 | 2,895 | 78.0 秒 | 354 秒・赤（1,000 行の番人） | 6・11・5・1・0 | `after-run-20261008-155002` |
| 7 | main の取り込みの後の 2 回目 | `dc5e3734` | 4 | 2,895 | 85.8 秒 | 346 秒・緑 | 0・0・0・3・4 | `after-run-20261008-171351` |

- 番号は下の小見出しの番号（5 は main の取り込み、8・9 は main の先端との比べ）。
- 本数は `progress.txt` の `areka bin test count=`。全体テストは `progress.txt` の `test-all exit=… sec=…`。
- 「（落ちた）」は、テストのプロセスが `0xc0000005`（`exit_code: -1073741819`）で終わり、`test result:` の行が無い回。既知の `areka-test-threads-av` と読み、この spec の対象の外として扱う（4.2 の注意と同じ）。
- 静かな机は、どの回しでも 3 回とも緑（赤 0 件・待ちの打ち切りの行は各回の檻の `［相手が居ない］` 2 行だけ）。

#### 1. 許可 2（`0d692ec8`・タスク 5.2 を入れた直後）

記録: 静かな机 `quiet-after-20261008-105842`、負荷の下 `after-20261008-110903`。

- 静かな机: 95.2・101.2・93.5 秒（中央値 95.2・最大 101.2）。負荷の前の CPU は平均 12.6%・最大 26.5%。全体テストは 322 秒・緑。
- 負荷の下（11:09:03〜12:19:11）: 負荷の前の CPU は読めなかった（4.1 と同じ文）。負荷の最中は平均 90.8%・最大 93.1%。

| 回 | 所要時間（`wall_sec`） | うち cargo の準備（ログの 1 行目の `Finished … in`） | libtest の時間（`finished in`） | 結果 | 赤 | 「60 秒を越えて走っている」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 927.0 | 5.97 秒 | 882.27 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 68 行 |
| 2 | 122.1 | 36.01 秒 | — | `0xc0000005` で終わった（`test result:` の行なし） | — | 0 行 |
| 3 | 923.9 | 9.69 秒 | 835.25 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 75 行 |
| 4 | 1,492.8 | 8 分 35 秒 | 936.49 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 74 行 |
| 5 | 720.8 | 9.18 秒 | 683.16 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 66 行 |

- 赤は 0 件。待ちの打ち切りの行は、走り切った 4 回とも檻の `［相手が居ない］` 2 行だけで、`［止まった］`・`［進んではいた］`・`［進みは不明］` は 0 行。2 回目は落ちた（`areka-test-threads-av`・対象外）。
- libtest の時間は 4.2（中央値 327.16 秒）の 2〜3 倍。GPU のテストが 2 つずつしか並ばないので、その族が列を作ったと読む（design「足場のスレッドの絞り」の Risks）。
- 静かな机の中央値 95.2 秒が、当時の線（1.10 倍・70.51 秒・5.6）を大きく越えた。そこで、作業ツリーで許可の数だけを替えて、静かな机を 2 回ずつ回した（`tune-run-20261008-123748\progress.txt`・各記録の `conditions.txt` は `commit: 0d692ec8`・`dirty: yes`）。

| 許可 | 記録 | `wall_sec` | 中央値 |
|---|---|---|---|
| 4 | `tune-cap4-20261008-123816` | 77.9・71.6 | 74.75 |
| 8 | `tune-cap8-20261008-124106` | 66.6・71.5 | 69.05 |
| 16 | `tune-cap16-20261008-124344` | 70.0・67.6 | 68.8 |

8 と 16 はほぼ同じで、8 をコミットした（`e69d92b1`）。

#### 2. 許可 8（`e69d92b1`）

記録: 静かな机 `quiet-after-20261008-124727`、負荷の下 `after-20261008-125548`。

- 静かな机: 66.9・64.7・64.3 秒（中央値 64.7・最大 66.9）。負荷の前の CPU は読めなかった。全体テストは 301 秒・緑。
- 負荷の下（12:55:48〜13:32:05）: 負荷の前の CPU は平均 4.6%・最大 10.7%、最中は平均 88.6%・最大 97.6%。

| 回 | 所要時間（`wall_sec`） | うち cargo の準備 | libtest の時間 | 結果 | 赤 | 「60 秒を越えて」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 392.4 | 11.43 秒 | 357.75 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 28 行 |
| 2 | 485.3 | 2.54 秒 | 426.48 秒 | 2,812 通過・1 失敗・2 無視 | 1 | 40 行 |
| 3 | 413.0 | 4.97 秒 | 386.07 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 37 行 |
| 4 | 406.5 | 3.74 秒 | 365.71 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 23 行 |
| 5 | 444.2 | 3.35 秒 | 384.06 秒 | 2,812 通過・1 失敗・2 無視 | 1 | 28 行 |

- 赤は 2 件で、どちらも自分の `［止まった］` の行を持ち、進み 0 回・待ち始めから 30.0 秒。
  - 2 回目 `emo2_boot::ghost_switch_test_support::talk_clock_tests::a_rig_clock_far_ahead_of_a_stalled_handshake_leaves_the_switch_unchanged`: 「置き場のゴーストを降ろす」（`round-2.log` 773 行。`spine_wait.rs:311:9` の panic の文言そのもの）。
  - 5 回目 `emo2_boot::ghost_switch::boot_event_tests::a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails`: 「`crates\areka\src\emo2_boot\ghost_switch_boot_event_tests.rs:81`」（`round-5.log` 657 行）。赤の `panicked at` は 663 行の `ghost_switch_boot_event_tests.rs:191:5`。
- `［進んではいた］`・`［進みは不明］`・待ちの文言の無い赤は 0 件。
- 読み分けの表（design「LoadRepro」）に当てると、進み 0 回の `［止まった］` で、4.2 と同じ形（相手のスレッドが始まれない）。許可 8 では 2.7 の絞りが足りない。
- 許可 2 では負荷の下の赤が 0 件で静かな机が線を越え、8 では静かな机が線の内で負荷の下に赤が残った。両方を満たす数が 1.10 倍の線の内に無かったので、開発者が線を 1.20 倍へ改め、許可を 4 にした（5.6・`1f494325`）。

#### 3. 許可 4（`1f494325`）

記録: 静かな机 `quiet-after-20261008-134526`、負荷の下 `after-20261008-135352`、順 `after-run-20261008-134510\progress.txt`。

- 静かな机: 72.3・72.3・69.9 秒（中央値 72.3・最大 72.3）。負荷の前の CPU は平均 3.6%・最大 6.9%。1.20 倍の線（76.92 秒・5.6）の内。全体テストは 283 秒・緑（線 348 秒の内）。
- 負荷の下（13:53:52〜14:40:02）: 負荷の前の CPU は平均 6.1%・最大 9.9%、最中は平均 91.6%・最大 94.3%。

| 回 | 所要時間（`wall_sec`） | うち cargo の準備 | libtest の時間 | 結果 | 赤 | 「60 秒を越えて」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 570.3 | 3.02 秒 | 490.06 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 53 行 |
| 2 | 503.0 | 10.35 秒 | 465.76 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 57 行 |
| 3 | 630.4 | 1 分 39 秒 | 499.37 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 43 行 |
| 4 | 506.1 | 9.06 秒 | 463.30 秒 | 2,812 通過・1 失敗・2 無視 | 1 | 55 行 |
| 5 | 528.4 | 11.51 秒 | 478.82 秒 | 2,813 通過・0 失敗・2 無視 | 0 | 57 行 |

- 待ちの打ち切りの行は、5 回とも檻の `［相手が居ない］` 2 行だけ。`［止まった］`・`［進んではいた］`・`［進みは不明］` は 0 行。
- 赤は 1 件（4 回目）: `emo2_boot::spine::conformance_lap_tests::conformance_lap_walks_every_stage_to_its_completion`。`round-4.log` 1313 行の `spine_conformance_lap_tests.rs:389:25` で、文言は「段「サブメニューと戻り」の完了条件が有界時間内に成立しない」。待ちの文言の先頭は無い。内訳（1315 行）は、選択確定「Onエモの位置調整メニュー」の照会 1 件・talk 起動 累計 6 件・占有終端 650ms。
- 読み分けの表では「待ちの文言が無い赤」に当たる。ただ、この文言を出すのは段の駆動器 `run_stage_with`（`spine_conformance_support.rs`）の自前の締切（`clock() + SPIN_WAIT`）で、芯の待ちを通らないので先頭が付かない。照会は 1 件届いていて（選択は捨てられていない）、台詞は始まって再生の終わり 650 ms を知らせ、関門が開いて余韻に入った所で 30 秒が切れた。負荷で遅いだけで、本番の順序の取り違えではない。
- 行き先は **タスク 5.4 の開き直し**。この待ちは 1.4 の記録（3.7 の場所 34）で赤だったのに移していなかった。駆動器を芯の `wait_until_with` へ移した（`24aa8fc9`・tasks.md の Implementation Notes「5.4 の開き直し」）。
- 負荷の最中の 13:54〜14:04 ごろの約 4 分、調停役のセッションが軽い台本を回していた（この記録のフォルダには残っていない。開発者とのやりとりで知った）。重なったのは負荷の下の時間だけで、`wall_sec` から数えると 1 回目と 2 回目の初め（どちらも緑）にあたり、赤の出た 4 回目とは重ならない。44 本の負荷で CPU が平均 91.6% の中の軽い仕事なので、読みは変わらない。

#### 4. タスク 5.4 の開き直しの後（`24aa8fc9`・途中で止めた・使わない）

記録: 静かな机 `quiet-after-20261008-150119`、負荷の下 `after-20261008-151443`（`summary.txt` なし）、順 `after-run-20261008-145914\progress.txt`。

- 静かな机: 負荷の前の CPU が平均 60.5%・最大 65.0%（`conditions.txt` の `cpu_before_load`）で、机が静かでなかった。3 回は 111.0・111.4・90.3 秒（中央値 111）で緑。
- 全体テストは 477 秒・緑（`test-all.log` の `x64 workspace tests` が 414 秒）。静かな机とともに机の騒がしさが入っているので、所要時間の比べには使わない。
- 負荷の下（15:14:43 から・負荷の前の CPU 平均 7.5%・最大 9.6%、最中 平均 86.9%・最大 96.5%）: 1 回目は 702.9 秒（libtest 604.94 秒）で緑、2 回目は 130.6 秒で `0xc0000005` で落ちた（`round-2.log` の末尾 `exit code: 0xc0000005, STATUS_ACCESS_VIOLATION`・`areka-test-threads-av`・対象外）、3 回目は 649 秒（libtest 607.56 秒）で緑（秒は `after-run-20261008-145914\after-load.log`）。1・3 回目の待ちの打ち切りの行は檻の 2 行だけ。
- 4 回目の途中（`round-4.log` の最後の書き込みは 15:44:07）で、main を取り込むためにこちらで止めた。`after-load.log` の最後の行は、止めたときに道具が `round-4.log` を読めなかった文。
- 走り切っていないので、捨てたデータとして残すだけで、数には入れない。

#### 5. main の取り込み

- `f61c372d` で main（先端 `22460c1d`）を取り込んだ。入ったのは PR #262〜#268（`git log f61c372d^1..f61c372d^2` の `eab35d21`・`5b9896c7`・`069f720b`・`8d5c70fb`・`918f2262`・`790eae43`・`22460c1d`）。
- 取り込みの後の直し 2 つ:
  - `14516a4e`: main から入った檻 `frame_balloon_timeout_notice_e2e_tests.rs` が `gpu_frame_world` を使うので、GPU の装置の許可を最後の欄として持たせた（装置より後に返す）。
  - `dc5e3734`: main が `spine_conformance_lap_tests.rs` に 3 行足して 1,002 行になった（`git show f61c372d:` で数えた）。段の駆動器の関門のテスト（`HoldingSink`）は、巡回でなく駆動器を確かめるものなので、`spine_conformance_support_tests.rs` へそのまま移した（巡回のファイルは 911 行）。
- 本数は 2,815 → 2,895（`after-run-20261008-155002\list.txt` の `: test` の行）。main の先端は 2,882 本（下の 8 の `test result:` の 2,880 通過＋2 無視）で、直す前の基準 2,802 本（5.3）から 80 本増えている。2,895 − 2,882 ＝ 13 本がこの spec の檻。

#### 6. 取り込みの後の 1 回目（`14516a4e`）

記録: 静かな机 `quiet-after-20261008-155434`、負荷の下 `after-20261008-160428`、順 `after-run-20261008-155002\progress.txt`。

- 静かな机: 78.0・75.4・78.3 秒（中央値 78.0・最大 78.3）・3 回とも 2,893 通過・2 無視。負荷の前の CPU は平均 3.9%・最大 6.0%。
- 全体テスト: 354 秒・終了コード 1。赤は `x64 workspace tests`（290 秒）の中の `crates/log-capture-kit/tests/file_length_guard_test.rs` の 3 本で、`crates/areka/src/emo2_boot/spine_conformance_lap_tests.rs (1002 行)` が 1 ファイル 1,000 行の番人に掛かった（`test-all.log`）。`dc5e3734` で直した。
- 負荷の下（16:04:28〜16:59:52）: 負荷の前の CPU は平均 4.3%・最大 8.1%、最中は平均 91.0%・最大 91.2%。

| 回 | 所要時間（`wall_sec`） | うち cargo の準備 | libtest の時間 | 結果 | 赤 | 「60 秒を越えて」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 644.3 | 9.62 秒 | 554.80 秒 | 2,887 通過・6 失敗・2 無視 | 6 | 75 行 |
| 2 | 972.5 | 17.79 秒 | 894.61 秒 | 2,882 通過・11 失敗・2 無視 | 11 | 89 行 |
| 3 | 675.7 | 5.50 秒 | 592.28 秒 | 2,888 通過・5 失敗・2 無視 | 5 | 68 行 |
| 4 | 521.3 | 7.10 秒 | 467.39 秒 | 2,892 通過・1 失敗・2 無視 | 1 | 60 行 |
| 5 | 478.1 | 6.52 秒 | 437.67 秒 | 2,893 通過・0 失敗・2 無視 | 0 | 54 行 |
| **計** | 中央値 644.3・最大 972.5 | | 中央値 554.80・最大 894.61 | 4 回が赤 | **23** | 346 行 |

`round-N.log` の全体を、行のどこかにある文言の先頭で数えた数（`summary.txt` の `wait_cut_lines` 8・14・6・3・2 と同じもの）:

| 文言の先頭 | 1 回目 | 2 回目 | 3 回目 | 4 回目 | 5 回目 | 計 |
|---|---|---|---|---|---|---|
| `［止まった］` | 6 | 12 | 4 | 1 | 0 | 23 |
| `［進んではいた］` | 0 | 0 | 0 | 0 | 0 | 0 |
| `［進みは不明］` | 0 | 0 | 0 | 0 | 0 | 0 |
| `［相手が居ない］` | 2 | 2 | 2 | 2 | 2 | 10 |

- `［止まった］` の 23 行のうち 13 行は進み 0 回・待ち始めから 30.0〜30.1 秒。残る 10 行は進み 2〜7 回・待ち始めから 30.3〜60.5 秒（降ろしの待ち「置き場のゴーストを降ろす」、上書きのインストールの `desk_overwrite_tests.rs:105`・`:247`、読み直しの `desk_reload_tests.rs:167`・`:441`・`:534`、lap の `shell_balloon_switch_session_lap_tests.rs:495` など）。
- 赤 23 件（21 種類。2 回赤は `switching_to_a_ghost_whose_descript_names_a_balloon_uses_it_and_records_the_descript_route` と `a_switch_with_a_boot_event_boots_the_target_with_that_event_instead_of_changed_or_boot`）のうち 22 件は、自分の `［止まった］` の行を持つ（2 回目の `stop_with_handoff_under_reservation_switches_without_exit` は「A の定常到達」と降ろしの 2 行）。族は切替・降ろし・MCP・インストールの上書き・読み直し・`check_script`・起動の印と、関係の無いものが混ざる。
- 残る 1 件は 3 回目の `session_end::sync_send_tests::teardown_join_does_not_wait_for_a_sync_send_to_the_ui_window`（3231 行の `session_end_sync_send_tests.rs:224:5`・main から入ったテスト）で、待ちの文言が無い。確かめの中身は `teardown_before_send: false`・`send_ok: false`（期待はどちらも `true`）で、文言の括弧の中の「送信が期限で切れた」の形。
- 時刻の固まり: `［止まった］` の行は、1 回目が 2012〜2044 行に 3 行、2 回目が 564〜744 行に 6 行と 1704〜1756 行に 6 行、3 回目が 3260〜3263 行に 4 行。関係の無い族が同じ時間帯にまとめて止まる、4.2 と同じ形。

#### 7. 取り込みの後の 2 回目（`dc5e3734`）

記録: 静かな机 `quiet-after-20261008-171431`、負荷の下 `after-20261008-172440`、順 `after-run-20261008-171351\progress.txt`。

- 静かな机: 82.5・85.8・86.2 秒（中央値 85.8・最大 86.2）・3 回とも 2,893 通過・2 無視。負荷の前の CPU は平均 9.5%・最大 14.6%。
- 全体テスト: 346 秒・緑（`x64 workspace tests` 290 秒）。
- 負荷の下（17:24:40〜18:15:12）: 負荷の前の CPU は読めなかった。最中は平均 90.8%・最大 93.9%。

| 回 | 所要時間（`wall_sec`） | うち cargo の準備 | libtest の時間 | 結果 | 赤 | 「60 秒を越えて」の知らせ |
|---|---|---|---|---|---|---|
| 1 | 471.5 | 9.88 秒 | 429.07 秒 | 2,893 通過・0 失敗・2 無視 | 0 | 50 行 |
| 2 | 509.5 | 10.59 秒 | 478.59 秒 | 2,893 通過・0 失敗・2 無視 | 0 | 56 行 |
| 3 | 642.3 | 2 分 31 秒 | 416.13 秒 | 2,893 通過・0 失敗・2 無視 | 0 | 49 行 |
| 4 | 629.0 | 10.74 秒 | 588.42 秒 | 2,890 通過・3 失敗・2 無視 | 3 | 58 行 |
| 5 | 747.2 | 2.70 秒 | 706.88 秒 | 2,889 通過・4 失敗・2 無視 | 4 | 65 行 |
| **計** | 中央値 629.0・最大 747.2 | | 中央値 478.59・最大 706.88 | 2 回が赤 | **7** | 278 行 |

文言の先頭で数えた数（`summary.txt` の `wait_cut_lines` 2・2・2・5・5 と同じもの）は、`［止まった］` 0・0・0・2・2、`［進みは不明］` 0・0・0・1・1、`［進んではいた］` 0、`［相手が居ない］` 各回 2。

| 回 | 赤 | 待ちの行 |
|---|---|---|
| 4 | `emo2_boot::spine::text_scale_tests::text_scale_phase_syncs_boxes_on_the_shell_window` | `［止まった］`「spine ghost shutdown」・進み 2 回・待ち始めから 31.1 秒（1098 行・`spine_wait.rs:311:9`） |
| 4 | `ghost_session::shell_balloon_switch_session_abort_tests::close_request_while_waiting_drops_the_switch_and_exits_as_today` | `［止まった］`「置き場のゴーストを降ろす」・進み 0 回・30.0 秒（1120 行） |
| 4 | `emo2_boot::spine::text_scale_tests::spine_dpi_change_while_balloon_hidden_lands_on_next_show` | `［進みは不明］`「spine seriko join」・10.1 秒（1154 行・`spine_wait.rs:379:19`。`spine.rs` の `join_bounded("spine seriko join", 10 秒)`） |
| 5 | `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_after_a_shell_swap_unloads_the_session` | `［止まった］`「`shell_balloon_switch_session_abort_tests.rs:491`」・進み 0 回・30.0 秒（1120 行。赤の `panicked at` は 1370 行の `:501:5`） |
| 5 | `ghost_session::switch_tests::memory_tests::switch_memory_is_default_while_welcoming_then_target_at_steady` | `［止まった］`「置き場のゴーストを降ろす」・進み 2 回・33.0 秒（1140 行） |
| 5 | `emo2_boot::spine::talk_close_tests::spine_s5_close_handshake_consumes_onclose_and_joins_all_handles_bounded` | `［進みは不明］`「`spine_talk_close_tests.rs:302`」・30.0 秒（1175 行。赤は 1177 行の `:306:5` で、起動の 5 つの呼び出しが 0 件 `[]`） |
| 5 | `emo2_boot::spine::talk_close_tests::spine_s2_talk_drives_surface_switch_and_typewriter_reveal` | なし（1122 行の `spine_talk_close_tests.rs:167:5`・「S2: シェル面指令＋テキスト cue が有界内に runtime へ到達しない」。テストの自前の締切 `SPIN_WAIT` のループ） |

- 7 件は 4 回目の 1097〜1154 行と 5 回目の 1120〜1177 行（と 1370 行）に固まる。spine・切替・降ろしの別々のテストが同じ時間帯に止まる、4.2 と同じ形。
- `［進みは不明］` の 2 件も、相手の始まりが 1 度も見えていない（s5 は 30 秒で起動の呼び出しが 0 件）。進み 0 回の `［止まった］` と同じ時間帯に出ている。
- 外の走行の見張り: この回しの間、1 分ごとに、このワークツリーの外のパスの cargo・rustc・テストの実行ファイルを挙げた一覧を取った（このセッションの作業用フォルダの `outside-watch-20261008-171346.txt`・`target\` の外・17:13:47〜18:43:34 の 84 回の読み）。他のワークツリーのテストの実行ファイルは 1 度も出ていない。cargo は多くの読みで 2 つ出ているが、cargo の実行ファイルのパス（`c:\rust\cargo\bin\cargo.exe` など）にはワークツリーの名前が入らないので、この回し自身の cargo も一覧に入っている（見張りの限り）。外の rustc は 18:11:32 の 1 回の読みだけ（2 つ）。

#### 8. 静かな机の比べ（main の先端と直した後）

記録 `ab-run-20261008-182545\progress.txt`（18:25:45〜18:37:27）。次の 3 つの実行ファイルを、静かな机で 1 回ずつ交互に 3 巡回した（1 回は `cargo test -p areka --bin areka` の全部と同じテスト）。

- main: main の先端 `22460c1d` を別の作業フォルダに出し、別の target（`target\ab-main-target`）でビルドしたもの
- on: HEAD（`dc5e3734`）。main から入った comctl32 の 6 版の宣言（manifest）つき
- off: HEAD から manifest をつなぐリンクの引数を外したもの（`build.rs` を一時だけ替えて回し、後で戻した。`progress.txt` の `build.rs restored`）

| 巡 | main | on | off |
|---|---|---|---|
| 1 | 67.4 秒 | 79.1 秒 | 76.1 秒 |
| 2 | 72.9 秒 | 86.0 秒 | 77.2 秒 |
| 3 | 67.2 秒 | 74.4 秒 | 74.9 秒 |
| 中央値 | 67.4 秒 | 79.1 秒 | 76.1 秒 |
| 結果 | 3 巡とも 2,880 通過・0 失敗・2 無視 | 3 巡とも 2,893 通過・0 失敗・2 無視 | 3 巡とも 2,893 通過・0 失敗・2 無視 |

- on と main の中央値の比は 79.1 ÷ 67.4 ＝ 1.17。manifest を外した off は on より中央値で 3.0 秒短いだけ。
- 前の 2 回の試しは捨てた。`ab-run-20261008-181631` は main を同じ target（`target\debug`）でビルドしたので、ワークスペースの crate の成果物が同じ名前で上書きされ、続く off のビルドが 16 件のエラーで止まった（`build-off.log`）。ワークスペースの crate を消して建て直した。`ab-run-20261008-182141` は main を別の target でビルドしたが、どの回も 0.0 秒で終わってテストが走っていない。

#### 9. 負荷の下の比べ（取り込みの前と後）

記録 `ab-load-20261008-183836\progress.txt`（18:38:36〜19:39:36・負荷の子 44 本・止めた後の残り 0）。取り込みの前 `24aa8fc9` の実行ファイル（pre・別の作業フォルダ `target\ab-pre` を別の target でビルド・2,815 本）と HEAD の実行ファイル（cur・2,895 本）を、同じ負荷の下で交互に回した。

| 時刻の順 | 回 | 終了 | 所要時間 | libtest の時間 | 結果 | 赤 | `［止まった］` の行（うち進み 0 回） |
|---|---|---|---|---|---|---|---|
| 1 | pre1 | `0xc0000005` | 45.0 秒 | — | — | — | 0 |
| 2 | cur1 | 101 | 871.5 秒 | 812.63 秒 | 2,879 通過・14 失敗 | 14 | 14（10） |
| 3 | cur2 | 101 | 1,090.2 秒 | 1,036.15 秒 | 2,885 通過・8 失敗 | 8 | 8（4） |
| 4 | pre2 | 0 | 549.9 秒 | 515.92 秒 | 2,813 通過・0 失敗 | 0 | 0 |
| 5 | pre3 | 0 | 473.8 秒 | 442.49 秒 | 2,813 通過・0 失敗 | 0 | 0 |
| 6 | cur3 | 101 | 509.2 秒 | 488.88 秒 | 2,892 通過・1 失敗 | 1 | 0 |

- pre1 は 45 秒で落ちた（`areka-test-threads-av`・対象外）。
- cur1・cur2 の `［止まった］` の行は固まって出る（`run-cur-1.log` の 1419〜1449 行に 10 行、`run-cur-2.log` の 1231〜1237 行に 7 行）。4.2 のローダーの錠の形と同じ。
- cur2 の赤 8 件のうち 7 件は `［止まった］`（行は 8 行）。残る 1 件 `update::desk::reload_tests::a_failed_reload_switch_drops_the_remembered_folder_and_the_tail` は `［進んではいた］`（3413 行・「`desk_reload_tests.rs:534`」・上限 300 秒・待ち始めから 534.7 秒・進み 3 回・最後の進みは 0.0 秒前）。待つ側が上限を 234.7 秒過ぎてから打ち切っていて、待つ側も動けていなかった。
- cur3 の赤は `install::desk::overwrite_tests::a_running_ghost_missing_from_the_catalog_is_installed_elsewhere_and_closed_once`（`run-cur-3.log` 1458 行の `desk_overwrite_tests.rs:491:5`）。呼び出しの列の終わりが完了でなく `OnInstallFailure` で、待ちの文言は無い。行き先は **タスク 5.6**（起動したゴーストの記憶の書き込みと上書きの競り合い。読みは tasks.md の 5.6）。
- 読み（main の差分の静的な読み。回して確かめたものではない）: main の差分は、足場の通り道に、テストごとのスレッド・DLL・タイマーを足していない。comctl32 は取り込みの前から 5.82 版が読み込まれていて、manifest はそれを 6 版に替えるだけ。ツールチップのフレームの仕組みは足場では早く戻る。#264 の位置の書き込みは `SwitchRig` の中では走らない。増えた 80 本のテストは、スレッドの生まれと消えを少し増やす。
- 悪かった cur1・cur2 は、時刻の順でビルドの直後の最初の 2 回（pre1 は 45 秒で落ちた）で、最後の cur3 には `［止まった］` が 0 行だった。順の効きと混ざっているので、この比べは「取り込みで負荷の下が悪くなった」ことを示さない。止まりの固まりは 4.2 のローダーの錠の形と同じ。

#### 後片付けの失敗の行（os error 5）

`summary.txt` の `cleanup_lines` と `os_error_5` は、どの記録でも同じ数。

| 記録 | 行（＝os error 5） |
|---|---|
| 静かな机 `quiet-after-20261008-105842`（許可 2） | 2（1 回目 1386 行の「生存の札の後始末」`work\27408-114.lock`・2 回目の「退避した残骸の削除」`work\gc-1292-1`） |
| 負荷の下 `after-20261008-110903` | 1（4 回目・残骸の退避 `work\38600-4`） |
| 静かな机 `quiet-after-20261008-124727`（許可 8） | 0 |
| 負荷の下 `after-20261008-125548` | 3（2 回目 2・5 回目 1） |
| 静かな机 `quiet-after-20261008-134526`（許可 4） | 2（1・3 回目に残骸の退避 1 行ずつ） |
| 負荷の下 `after-20261008-135352` | 0 |
| 静かな机 `quiet-after-20261008-155434`（取り込みの後 1） | 2 |
| 負荷の下 `after-20261008-160428` | 0 |
| 静かな机 `quiet-after-20261008-171431`（取り込みの後 2） | 1 |
| 負荷の下 `after-20261008-172440` | 1 |

- 直す前（静かな机 633 行・負荷の下 349 行・6）と比べて、どの記録も 0〜3 行。4.1・4.2 に無かった種類は、許可 2 の静かな机の 1 回目の「生存の札の後始末」1 行で、回の始まりでなく途中（1386 行）に出ている。番号 27408 はこのログの他の所に出ないので、どのプロセスの札かは記録からは決められない。その回は緑で、赤との関わりは無い。
- どの赤の文言にもファイルの拒否は出ない。

#### テストの本数（要件 2.4）

- 取り込みの前は 2,815 本（`after-run-20261008-105822`・`-124705`・`-134510`・`-145914` の `list.txt` の `: test` の行）。4.2 の 2,814 本から増えた 1 本は、タスク 5.2 の檻 16 `emo2_boot::spine::wait_tests::gpu_slots_hold_at_most_the_capacity_and_a_returned_permit_frees_one`（`after-run-20261008-083203\list.txt` と `after-run-20261008-105822\list.txt` を名前で比べた差 1 本・消えた名前は 0）。檻 17 は `wintf` の `task_pool_tests.rs` にあり、この実行ファイルの外。
- 取り込みの後は 2,895 本（`after-run-20261008-155002`・`-171351` の `list.txt`）。各回の `test result:` も通過＋失敗＋無視（2）＝その本数。
- 無視は 2 本のまま。

#### 読み

- 静かな机は、許可 4 を入れた後のどの回しでも 3 回とも緑。
- 負荷の下に残るのは、進み 0 回の `［止まった］` が関係の無い族にまとめて出る赤（回し 2・6・7・9 の cur1・cur2）。4.2 と同じ 2.7 の種類（スレッドの始まり・終わりがローダーの錠を待つ）で、許可 4 でも出る。回し 3（許可 4・取り込みの前）では 5 回で 0 行だった。
- 待ちの文言の無い赤は 4 種類あった。lap の段の駆動器（回し 3）はタスク 5.4 の開き直しで芯へ移した。インストールの上書きの競り合い（回し 9 の cur3）はタスク 5.6。`session_end::sync_send_tests::teardown_join_does_not_wait_for_a_sync_send_to_the_ui_window`（回し 6・main から入ったテスト）と `spine_s2_talk_drives_surface_switch_and_typewriter_reveal`（回し 7・テストの自前の締切）は、まだどのタスクにも回していない（この記録の後、タスク 5.4 の 2 度目の開き直し `57063ec1` で対象に加えた）。
- `［進みは不明］` は回し 7 の 2 行（「spine seriko join」と `spine_talk_close_tests.rs:302`）。読み分けの表では「その待ちに目印を渡せるかを調べ、対象に加える」。進み 0 回の固まりと同じ時間帯に出ている。まだどのタスクにも回していない（この記録の後、`spine_talk_close_tests.rs:302` はタスク 5.4 の 2 度目の開き直しで目印を渡した。「spine seriko join」は記録つきの例外のままとしたが、閉じる前のレビューで案 A の 6.1 に反すると差し戻され、5.4 の 3 度目の開き直しで見張りのスレッドをやめて body の終わりを目印に芯で待つ形にした）。
- `［進んではいた］` は回し 9 の cur2 の 1 行だけで、5.1 の正式の回しには無い。足場の同時の数の許可（RigPermit）は入れていない（この後に入れて 4.4 の回を回し、外した）。
- 最後の記録（main の先端と直した後の静かな机の比べ・全体テスト・負荷の下 5 回）は、深夜に調停役の「どうぞ」を待って回し、4.4 に書く。

### 4.4 最後の記録（深夜の回）

2026-10-08 22:32〜2026-10-09 00:02 に、調停役の「どうぞ」を受けて続けて回した（順と時刻は `target\load-flake\midnight-run-20261008-223222\progress.txt`）。コミットは `c597a8f1`（`dirty=0`）で、足場の同時の数の許可（RigPermit・論理 CPU の半分）を `SwitchRig` に入れた形。この回の結果を見て RigPermit は外した（`0506a197` で戻した・下の「読み」）。だから、この回の所要時間は RigPermit つきの値で、外した後の値は下の「最後の所要時間（RigPermit を外した後）」で取り直す。

順: 先にビルド（x64 の温め 22:32〜22:37）→ main の先端 `22460c1d`（4.3 の 8 と同じ・別の作業フォルダを `target\ab-main-target` でビルド）と HEAD の実行ファイルを用意 → 静かな机の交互の比べ → 全体テスト → 負荷の下 5 回。

#### 静かな机の比べ（main の先端と HEAD・交互に 3 巡）

| 時刻の順 | 実行ファイル | `wall` | 結果 |
|---|---|---|---|
| 1 | main | 68.7 秒 | 2,880 通過・0 失敗・2 無視 |
| 2 | head | 90.3 秒 | 2,894 通過・0 失敗・2 無視 |
| 3 | head | 87.4 秒 | 同上 |
| 4 | main | 66.1 秒 | 2,880 通過・0 失敗・2 無視 |
| 5 | main | 64.7 秒 | 同上 |
| 6 | head | 87.0 秒 | 2,894 通過・0 失敗・2 無視 |

- 中央値は main 66.1 秒・head 87.4 秒で、比は 87.4 ÷ 66.1 ＝ 1.32。1.20 倍の線（79.3 秒）を越えた。4.3 の 8（RigPermit なし）の比 1.17 から延びた分は、RigPermit で足場が論理 CPU の半分ずつしか並ばなくなった分と読む。
- head の本数は 2,896 本（2,894 通過＋2 無視）。4.3 の 2,895 本から増えた 1 本は RigPermit の檻 `emo2_boot::spine::wait_tests::rig_slots_hold_half_the_cpus_and_one_thread_holds_one_rig_at_a_time`（RigPermit とともに外したので、今は 2,895 本）。
- head の 3 回とも緑。

#### 全体テスト

333 秒・終了コード 0（`progress.txt` の `test-all exit=0 sec=333`）。`test-all.log` の結果の表（`commit c597a8f1`）は、`x64 workspace tests` 276 秒・`i686 tests (host-32)` 43 秒・`fmt --check` 6 秒・`encoding check` 4 秒など、すべて `OK`。

#### 負荷の下 5 回（`after-20261008-225606`）

記録 `target\load-flake\after-20261008-225606\`（`conditions.txt`・`summary.txt`・`round-1.log`〜`round-5.log`）。引数は 4.1 と同じ（`rounds: 5`・`burners: 44`・`round_timeout_min: 30`・`nocapture: yes`・`filter: (none: whole binary)`・`test_threads: (not passed)`・`logical_cpus: 22`）。負荷の前の CPU は平均 4.3%・最大 5.9%、最中は平均 91.2%・最大 91.4%。止めた後の負荷の子の残りは 0。

| 回 | 所要時間（`wall_sec`） | libtest の時間 | 結果 | 赤 | `［止まった］` の行 |
|---|---|---|---|---|---|
| 1 | 879.1 | 797.56 秒 | 2,891 通過・3 失敗・2 無視 | 3 | 3 |
| 2 | 782.0 | 718.62 秒 | 2,893 通過・1 失敗・2 無視 | 1 | 1 |
| 3 | 824.9 | 774.27 秒 | 2,890 通過・4 失敗・2 無視 | 4 | 4 |
| 4 | 793.4 | 731.13 秒 | 2,888 通過・6 失敗・2 無視 | 6 | 6 |
| 5 | 698.5 | 658.94 秒 | 2,894 通過・0 失敗・2 無視 | 0 | 0 |
| **計** | 中央値 793.4・最大 879.1 | | 4 回が赤 | **14** | **14** |

`round-N.log` の全体を、行のどこかにある文言の先頭で数えた数: `［止まった］` 3・1・4・6・0（計 14）、`［進んではいた］` 0、`［進みは不明］` 0、`［相手が居ない］` 各回 2（檻の 2 行）。

| 回 | 赤 | 待ちの行（どれも `［止まった］`） |
|---|---|---|
| 1 | `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` | 「置き場のゴーストを降ろす」・進み 2 回・待ち始めから 30.7 秒（1151 行） |
| 1 | `ghost_session::shell_balloon_switch_session_abort_tests::close_request_while_waiting_drops_the_switch_and_exits_as_today` | 「置き場のゴーストを降ろす」・進み 0 回・30.0 秒（1172 行） |
| 1 | `mcp::dump_balloon::dump_balloon_gpu_tests::balloon_at_dpi_144_keeps_native_size_and_text_position` | 「置き場のゴーストを降ろす」・進み 2 回・32.8 秒（2013 行） |
| 2 | `emo2_boot::ghost_switch::boot_event_tests::a_boot_event_is_not_delivered_to_the_default_ghost_when_the_target_fails` | 「置き場のゴーストを降ろす」・進み 0 回・30.0 秒（806 行） |
| 3 | `install::desk::overwrite_tests` の 4 本（`a_switch_to_another_ghost_does_not_install_and_returns_the_archive`・`a_running_ghost_missing_from_the_catalog_is_installed_elsewhere_and_closed_once`・`an_overwritten_ghost_that_cannot_boot_falls_back_to_the_default_ghost`・`an_overwritten_default_ghost_that_cannot_boot_is_fatal`） | 「`desk_overwrite_tests.rs:236`」（足場 `running()` の `wait_steady`）・進み 2 回・31.8〜32.4 秒（1610・1630・1650・1670 行） |
| 4 | `ghost_session::shell_balloon_switch_session_abort_tests::undecodable_shell_keeps_the_old_mount_and_records_one_error` | 「`shell_balloon_switch_session_abort_tests.rs:227`」・進み 0 回・30.0 秒（1158 行） |
| 4 | `ghost_session::switch_tests::fallback_tests::target_connect_fail_boots_default_with_halt_and_no_alert` | 「`ghost_session_switch_fallback_tests.rs:134`」・進み 0 回・30.0 秒（1159 行） |
| 4 | `ghost_session::switch_tests::fallback_tests::target_fault_without_default_ghost_is_fatal` | 「`ghost_session_switch_fallback_tests.rs:206`」・進み 0 回・30.0 秒（1160 行） |
| 4 | `ghost_session::shell_balloon_switch_session_lap_tests::script_shell_switch_round_trips_with_raise_event` | 「`shell_balloon_switch_session_lap_tests.rs:495`」・進み 6 回・43.0 秒（1161 行） |
| 4 | `ghost_session::shell_balloon_switch_session_balloon_tests::script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` | 「`shell_balloon_switch_session_balloon_tests.rs:244`」・進み 0 回・30.0 秒（1169 行） |
| 4 | `ghost_session::switch_tests::memory_tests::argv_session_switch_follows_last_ghost_but_never_touches_mark` | 「`ghost_session_switch_memory_tests.rs:133`」・進み 0 回・30.0 秒（1170〜1180 行。他のテストの出力が割り込んで 3 行に分かれた） |

- 14 件のどれも、自分の `［止まった］` の行を持つ。7 件は進み 0 回・待ち始めから 30.0 秒、残る 7 件は進み 2〜6 回・30.7〜43.0 秒。待ちの文言の無い赤・`［進みは不明］`・`［進んではいた］` は 0 件。
- 新しく足した「最後の 1 回の確かめに N 秒」（タスク 5.4 の 2 度目の開き直し）は、14 行とも 0.0〜0.1 秒。足場の 1 段が同期で塞がった形（4.3 の 9 の `desk_reload` の 534.7 秒）は無く、待つ側は時間どおりに確かめていた。
- 確かめの食い違い（4 回目の `assert_eq!` の左右）は、どれも待ちが届かなかった印（定常・往復の終わり）だけが違い、呼び出しの列は期待の頭の部分のまま（例: lap は期待の列から最後の `GET OnShellChanged` が欠けただけ）。並びの入れ替えは無く、本番の順序の取り違えの形ではない。
- 族は切替・降ろし・インストールの上書き・MCP の GPU・起動イベント・戻り・記憶・lap・バルーンで、どれもゴーストの足場（`SwitchRig`・`LapRig`）か GPU の足場（`GpuRig`）を使う。行は時間の上で固まって出る（1 回目の 1151〜1172 行、3 回目の 1610〜1670 行、4 回目の 1158〜1180 行）。関係の無い族が同じ時間帯にまとめて止まる、4.2 と同じ形。
- 後片付けの失敗の行は 5 回とも 0（`cleanup_lines 0`・`os_error_5 0`、`round-N.log` の `sample-ghost-kit:` と `アクセスが拒否されました` も 0）。
- 本数: 5 回とも通過＋失敗＋無視（2）＝2,896 本（上の RigPermit の檻を含む）。

外の走行の見張り: この回しの間、1 分ごとに、このワークツリーの外のパスの cargo・rustc・テストの実行ファイルを挙げた一覧を取った（このセッションの作業用フォルダの `outside-watch-20261008-223217.txt`・22:32:17〜翌 00:31:39 の 111 回の読み）。他のワークツリーのテストの実行ファイルは 1 度も出ていない。cargo は回しの間の多くの読みで 2 つ出ているが、cargo の実行ファイルのパスにはワークツリーの名前が入らないので、この回し自身の cargo も一覧に入っている（4.3 の 7 と同じ）。rustc は温めのビルドの間の 22:33・22:34 の 2 回の読みだけ。回しが終わった後（00:05・00:06）に cargo が 2 つ出たが、回しの外の時間である。

#### 読み（RigPermit を外したこと）

- RigPermit を入れても、負荷の下の赤は 14 件で減らなかった（4.3 の 6・7・9 と同じ桁）。静かな机は main の先端の 1.32 倍に延びた。どちらにも効かないので外した（`0506a197`）。

#### 静かな机の調べ（赤の元）

上の結果の後、静かな机で GPU の装置を作って捨てるだけの小さな調べ（Intel Arc・debug 版）を回し、プロセスに読み込まれた DLL を段ごとに並べた（このセッションの作業用フォルダの `mods-*.txt`）。

- 装置を作る前は、Intel のドライバの DLL は 1 つも無い。最初の装置を作ると `igd10umt64xe`・`igc64`・`igc-default64` などのドライバの DLL と、D3D の検証層（`D3D11_3SDKLayers`・`DXGIDebug`）が入る。装置を捨てても、その後に 10 回作って捨てても、`igd10umt64xe`・`igc64`・`igc-default64` と検証層はプロセスに残る。
- これらはスレッドの生まれと消えのたびに OS のローダーの錠の中で初期化を受ける DLL で、その数は 14 から 26〜34 へ増えた。だから最初の GPU のテストの後は、GPU を使わないテストを含めて、すべてのスレッドの生まれと消えが重くなる。論理 CPU の 2 倍の負荷では錠を持つスレッドが待たされ、スレッドの始まりと終わりが 30 秒を越えて列を作る（4.2 のスタックの形）。
- 装置ごとのスレッドは 3 つの生まれと 3 つの消えで、1 回の実行の約 52 の装置を合わせても、全体のスレッドの生まれと消えの約 1%。足場の数や装置の数を絞っても、残る DLL が 1 回ごとに重くする形は変わらない。
- 効かない手: 装置の使い回し（共有の描画の文脈は 1 スレッド前提で、テストの中身が変わる）。検証層を外すこと（外しても残る DLL が 2 つ減るだけで、ドライバの DLL は残る・`mods-nodbg-*.txt`）。WARP への置き換えは採らない（テストは実の GPU で行う）。GPU のテストを別のプロセスへ分けても、負荷の条件は同じなので、そちらのプロセスで同じ列ができる。

#### 開発者の裁定（2026-10-09・案 A）

この残りを既知の残りとして許し、完了の線を改めた（要件 6.1・6.2、design の設計の決定「負荷の下に残る赤」、steering の `tech.md` の Testing「テストの待ちと、負荷の下で許す赤（申し開き…）」）。

- 許す赤: 負荷の再現（`-Burners 44`・22 論理 CPU）の下に限り、ゴーストの足場と GPU の足場を使うテストの、文言どおりの `［止まった］`（多くは進み 0 回・待ち始めから 30.0 秒）で、相手の処理に取り違えが無いもの。
- 許さない赤: 静かな机と `tools/test-all.ps1` のすべての赤。負荷の下でも、待ちの文言の無い赤・`［進みは不明］`・`［進んではいた］`・取り違えのある `［止まった］`。
- 上の 5 回は、残った 14 件がすべて許す赤に当たり、許さない赤は 0 件（RigPermit つきの回だが、RigPermit は赤の数を変えなかったので、外した後の形でも同じ読み）。

#### 最後の所要時間（RigPermit を外した後）

2026-10-09 07:19〜07:29 に、開発者の「後10時間くらいは静穏を維持できる」の後、調停役へ知らせて回した（記録 `target\load-flake\quiet-final-20261009-071957\progress.txt`）。コミットは `37253505`（RigPermit を外し・main の先端 `5fa58306`（コードは `22460c1d` と同じ・違いは `.kiro`・`.claude`・`CLAUDE.md` だけ）を取り込み・steering の注を足した後）。先にビルドを済ませ（x64 の温め 12 秒）、main の先端（`target\ab-main-target`）と HEAD の実行ファイルを 1 分の間を置いてから交互に回した。

| 時刻の順 | 実行ファイル | `wall` | 結果 |
|---|---|---|---|
| 1 | main | 66.8 秒 | 2,880 通過・0 失敗・2 無視 |
| 2 | head | 76.1 秒 | 2,893 通過・0 失敗・2 無視 |
| 3 | head | 72.4 秒 | 同上 |
| 4 | main | 83.5 秒 | 2,880 通過・0 失敗・2 無視 |
| 5 | main | 73.7 秒 | 同上 |
| 6 | head | 79.8 秒 | 2,893 通過・0 失敗・2 無視 |

- 中央値は main 73.7 秒・head 76.1 秒で、比は 76.1 ÷ 73.7 ＝ 1.03。線は 73.7 × 1.20 ＝ 88.4 秒（前の最大 83.5 秒はこれより下）で、目安の内に収まった。調整（空回しの窓・GPU の装置の許可の数）は要らない。
- head の本数は 2,895 本（2,893 通過＋2 無視）＝4.3 と同じ（RigPermit の檻は外した）。main の先端より 15 本多いのは、この spec で足した檻。
- head の 3 回とも緑。静かな机では許す赤も出ていない。
- 4.4 の深夜の回（RigPermit つき・1.32 倍）と 4.3 の 8（`dc5e3734`・1.17 倍）より縮んだのは、RigPermit を外したことと、この朝の机の揺れ（main の 66.8〜83.5 秒）による。比べは同じ机で交互に取った値だけで読む。

全体テスト（静かな机・`tools/test-all.ps1`・同じコミット）: 2 回回した。1 回目は 462 秒・終了コード 0 だが、x64 のワークスペースのテストの段に他の crate のテストのビルド（1 分 54 秒）が入っていた（計測の温めは `--bin areka` だけだった）ので所要時間には使わない（`test-all.log`）。先に `cargo test --workspace --no-run -j 4` を済ませた 2 回目は **295 秒**・終了コード 0・すべての段が緑（`fmt --check` 8 秒・x64 のワークスペースのテスト 240 秒・i686 のテスト（host-32）41 秒・`encoding check` 5 秒など・`test-all-2.log`・`test-all-2-result.txt`）。1,000 行の番人・一時パスの見張りを含めて緑で、例外表に行は足していない。直す前の 290 秒に対して 1.02 倍で、線 348 秒の内。

### 4.5 今のコードの負荷の回（閉じる前のレビューの差し戻しの後）

閉じる前のレビューが、4.4 の負荷の下 5 回は RigPermit つきの `c597a8f1` のもので、今のコードを負荷の下で回していないと差し戻した。同じレビューで、記録つきの例外だった `join_bounded`（「spine seriko join」）が負荷の下で `［進みは不明］` を出しうる（案 A の 6.1 で許さない）ことも差し戻された。そこで 5.4 の 3 度目の開き直しで `join_bounded` を芯の上へ移し（`9d69acba`）、調停役の「どうぞ」を受けて同じ引数で 5 回回した。

記録 `target\load-flake\after-20261009-075612\`（`progress.txt` は `target\load-flake\final-load-20261009-075611\`・`commit 9d69acba dirty=0`）。2026-10-09 07:56〜08:44。引数は 4.1 と同じ（`rounds: 5`・`burners: 44`・`round_timeout_min: 30`・`nocapture: yes`・`filter: (none: whole binary)`・`logical_cpus: 22`）。負荷の前の CPU は平均 4.2%・最大 10.1%、最中は平均 89.7%・最大 91.4%。止めた後の負荷の子の残りは 0。

| 回 | 所要時間（秒） | 結果 | `［止まった］` | `［進んではいた］` | `［進みは不明］` | 後片付けの行（うち os error 5） |
|---|---|---|---|---|---|---|
| 1 | 458.7 | 2,894 通過・0 失敗 | 0 | 0 | 0 | 0 |
| 2 | 581.0 | 2,890 通過・4 失敗 | 3 | 0 | 0 | 0 |
| 3 | 448.0 | 2,894 通過・0 失敗 | 0 | 0 | 0 | 0 |
| 4 | 484.9 | 2,888 通過・6 失敗 | 5 | 0 | 0 | 3（3） |
| 5 | 894.7 | 2,872 通過・22 失敗 | 21 | 0 | 0 | 0 |

`［相手が居ない］` は各回 2 行（檻 `a_vanished_worker_panics_with_the_partner_gone_text` の出力・緑）。

読み分け（赤 32 件・どの赤も `thread '<名前>'` の行か、待ちの行の「何を」の呼び出しの場所で持ち主を突き合わせた）:

- **許す赤 28 件**: どれも自分の `［止まった］` の行を持つ。切替・lap・バルーン・更新・中止・上書き（`desk_overwrite_tests.rs:236` の 6 件と `:103` の 1 件）・記憶・名前・MCP の GPU（`dump_surface_gpu`・`dump_balloon_gpu`）・`get_property`・`get_status`・`check_script`・install の worker。進みは 0〜8 回、待ち始めから 30.0〜61.9 秒。確かめの食い違いは、どれも待ちが届かなかった印（定常・差し替わった・迎え入れられた・終了の指示）だけで、呼び出しの列は期待と同じかその頭の部分。並びの入れ替えは 0 件。
- **許さない赤 4 件**（どれも待ちの文言が無い）:
  1. 2 回目 `mcp::dump_balloon::dump_balloon_gpu_tests::balloon_text_lies_on_the_background_at_the_text_area_origin` と 5 回目 `…::hidden_balloon_returns_the_same_pixels`: 降ろせた＝false だけが違う。GPU の足場の時計は実時間なので、負荷で台詞の後から降ろすまでが 30 秒（バルーンの時間切れの既定）を越え、本番どおり `OnBalloonTimeout` が SHIORI へ行った。偽の SHIORI（`ScriptedShioriBackend::get`）は台本に無い名前で panic し（2 回目 1694 行・5 回目 2072 行）、降ろしが `Err` を返した。本番の欠陥ではなく、偽の SHIORI の答えの欠け。
  2. 4 回目 `mcp::dump_surface::dump_surface_tests::abandoned_while_encoding_drops_the_pair_without_ui_errors`: テストの中の自前の `recv_timeout(20 秒)` が切れた（仕事が走り終えた＝false）。芯を通らない待ち。
  3. 5 回目 `install::desk::pick_tests::menu_order_is_the_script_order_with_only_the_origin_changed`: `desk_pick_tests.rs` の自前の `join_bounded`（見張りのスレッドを生んで `recv_timeout(10 秒)`）が切れた。5.4 の 3 度目の開き直しで直した spine の `join_bounded` と同じ形。
- 後片付けの行（4 回目の 3 行）: `sample-ghost-kit: 残骸の退避に失敗した（次の走行で回収する）: …\target\nar-samples\work\30680-106 (… os error 5)`。走行の途中で、別のテストが検体を取るときの掃除（`devroot.rs` の `sweep()`）が、生存の札の無い木の名前替えを 3 回とも拒まれた。掃除の失敗は取得を失敗にしないので、この行で赤になったテストは無い。直前に降ろしの止まりで打ち切られた `install::names::tests::reboot_reseeds_the_values_into_the_new_ghost` の残ったスレッドがファイルを握っていた形と読める（確かめてはいない）。打ち切られたテストの後の掃除は「次の走行で回収する」の設計どおり。

直し（5.4 の 4 度目の開き直し）: ⑴ 偽の SHIORI は台本に無い `OnBalloonTimeout` に 204 で答える（`homeurl`・`OnTranslate` と同じ扱い・台本があれば台本が先）。⑵ `abandoned_while_encoding…` の待ちを芯の `wait_recv` へ移し、仕事の閉包に入った数を目印にする。⑶ `desk_pick_tests.rs` と、同じ形の `fetch_url_tests.rs` の自前の `join_bounded` を、spine の `join_bounded` と同じく見張りのスレッドをやめて芯で待つ形にした。静かな机で `--bin areka` の全部が 2,896 本で緑。

LOAD_RERUN_RESULT

## 5. 所要時間

直す前はタスク 1.3、直した後はタスク 6.1 で書く（静かな机で `--bin areka` の全部を 3 回・全体テストを 1 回）。main の取り込みの後の最後の値（main の先端との交互の比べ）は、4.4 の「最後の所要時間（RigPermit を外した後）」に書く。

### 5.1 測り方（6.1 も同じ順で測る）

2026-10-07 23:55〜2026-10-08 00:13 に、次の順で続けて回した（記録 `target\load-flake\before-run-20261007-235604\progress.txt`）。

1. 静かさの確かめ（`tools/perf/check-quiet.ps1`）。出力は下の 5.2。
2. 先にビルドを済ませる。i686 の部品（`shiori-host32-helper`・`shiori-host32-testdll`）36 秒、x64 の `cargo test --workspace --no-run -j 4` 382 秒（`warm-i686.log`・`warm-x64.log`）。全体テストの所要時間にコンパイルの時間を入れないため。x64 が長いのは、main を取り込んで `Cargo.toml` の `[profile.dev]` が `debug = "line-tables-only"` に変わった後の最初のビルドだったから。
3. テストの本数を数える（`cargo test -p areka --bin areka -- --list`）。下の 5.3。
4. 負荷なしの 3 回（`pwsh -NoProfile -File tools/load-flake.ps1 -Label quiet-before -Rounds 3 -Burners 0 -RoundTimeoutMin 30 -NoCapture`）。下の 5.4。
5. 全体テストを 1 回（`pwsh -NoProfile -File tools/test-all.ps1` を外から時間を測って回す）。下の 5.5。

その後に負荷の下の再現（タスク 1.4）へ続けた。直した後（タスク 6.1）も、静かさの確かめ → 先にビルド → 本数 → 3 回 → 全体テストの順で測る。ビルドを先に済ませないと、全体テストの時間にコンパイルが入って前後が比べられない。

### 5.2 静かさの確かめ

測る直前（2026-10-07 23:55 JST）の `tools/perf/check-quiet.ps1` の出力（`target\load-flake\quiet-check\quiet-before.txt` の全文）:

```
[check-quiet] version=1.0.1
stage=before
time_utc=2026-10-07T14:55:46.922Z
sample_sec=20
machine_cpu_mean_pct=2.7
machine_cpu_max_pct=5.8
threshold_mean_pct=10.0
heavy_process_names=cargo,rustc,rust-analyzer,msbuild,link,cl,areka,python
heavy_processes_found=0
heavy_process_list=-
target_pid_excluded=-
verdict=QUIET
reason=ok
heavy_process_cpu_min_pct=1.0
heavy_process_presence=areka
heavy_process_busy=-
```

机の CPU は平均 2.7%・最大 5.8%（線は平均 10%）、重いプロセスは 0 で、判定は静か（`QUIET`）。

この道具は、重いプロセスが 1 つも無いときに止まっていた（空の一覧が関数から返ると空でなくなり、その中身の名前を読みに行って落ちる）。確かめの前にコミット `d6d77336` で 2 行の守りを足して直し、そのコミットの上で回した。

### 5.3 テストの本数（以後の「同じ本数」の基準）

| 項目 | 値 | 根拠 |
|---|---|---|
| `cargo test -p areka --bin areka` のテストの本数 | **2,802 本** | `target\load-flake\before-run-20261007-235604\list.txt`（`-- --list` の出力。`: test` で終わる行が 2,802 行、末尾に `2802 tests, 0 benchmarks`） |
| うち走らせないもの（無視の印） | 2 本 | 下の 3 回の `test result:` の行が、どれも `2800 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out` |

上の「1. 手順」にある「2,673 本」は 2026-10-06 の数。main を取り込んだ後の基準はこの 2,802 本で、直した後もこの本数で、無視は 2 本のままであることを確かめる（要件 2.4・7.3）。

### 5.4 対象の族（`--bin areka` の全部・負荷なし 3 回）

記録 `target\load-flake\quiet-before-20261008-000446\`（`conditions.txt`・`summary.txt`・`round-1.log`〜`round-3.log`）。条件はコミット `d6d77336`（未コミットの変更なし）、`nocapture: yes`、負荷の前の CPU は平均 4.4%・最大 7.1%（5 回の読み）。

| 回 | 所要時間（秒） | 結果 | 後片付けの失敗の行（`summary.txt` の数） | うち os error 5（`summary.txt` の数） | 混ざりを戻して読んだ行 | うち os error 5（戻して読んだ数） |
|---|---|---|---|---|---|---|
| 1 | 66.4 | 緑（2,800 本通過・赤 0） | 222 | 222 | 222 | 222 |
| 2 | 62.4 | 緑（2,800 本通過・赤 0） | 220 | 217 | 220 | 220 |
| 3 | 64.1 | 緑（2,800 本通過・赤 0） | 189 | 189 | 191 | 191 |
| **中央値** | **64.1** | | | | | |
| **最大** | **66.4** | | 合計 631 | 合計 628 | 合計 633 | 合計 633 |

- 所要時間は `summary.txt` の `wall_sec`（3 回とも上限越えなし、`待ちの打ち切り の行` は 0）。
- 後片付けの失敗の行の「`summary.txt` の数」は道具が出したそのままの値で、下の数え方の漏れを含む。「戻して読んだ」数は、`round-N.log` を目で読み、他のテストの出力が割り込んで切れた行をつなぎ直して数えたもの。
- 数え方の注意: `summary.txt`（`tools/load-flake.ps1` の数え方）は、行の頭が `sample-ghost-kit:` で、同じ行に `os error 5` がある行を数える。`--nocapture` では他のテストの出力が行の途中に割り込むので、次の形で漏れる。
  - 2 回目（`round-2.log`）: 3 行が割り込みで切れ、`os error 5` が同じ行に無い。996 行目は `os error` と `5` の間に他のテストの `ok` が入っている。1698 行目は行の頭だけが残り（後ろは他のテストの名前と `ok`）、本文は 1700 行目に行の頭なしで出ている。3067 行目は `(` で切れ、`アクセスが拒否されました。 (os error 5))` が次の行にある。だから 220 行すべてが os error 5 で、`summary.txt` の 217 は 3 少ない。
  - 3 回目（`round-3.log`）: 2 行が他のテストの出力と同じ行の途中から始まり、行の頭で数える `summary.txt` から漏れている（916 行目・1438 行目。本文はそれぞれ次の行）。ファイルの中で `sample-ghost-kit:` をどこでも探すと 191 行あり、すべて os error 5。
  - 1 回目は割り込みが無く、どちらの読み方でも 222 行・すべて os error 5。
- 戻して読むと、静かな机の 3 回で出た後片付けの失敗の行 633 行は、すべて os error 5（アクセスが拒否された）で、他の理由の失敗は 1 行も無い。文言の種類は「残骸の退避に失敗した」が 628 行、「退避した残骸の削除に失敗した」が 4 行（2 回目 3・3 回目 1）、「作業フォルダの後始末に失敗した」が 1 行（2 回目）。
- os error 5 の行は、静かな机で 3 回とも緑の回でも出ている。ここでは数だけを残し、読み解きと、`tools/load-flake.ps1` の行の頭で数える数え方の扱いは、タスク 4.4（下の 6）で行う。
- `summary.txt` の項目名は、この記録（直す前）では日本語（`上限越え (timed_out)`・`赤のテスト (failed_tests)` など）だが、コミット `0c046e92` 以後の走行では英数字（`timed_out`・`failed_tests`・`wait_cut_lines`・`cleanup_lines (sample-ghost-kit:)`・`os_error_5`）になる。中身は同じ。

### 5.5 全体テスト（1 回）

記録 `target\load-flake\before-run-20261007-235604\test-all.log`（末尾の結果の表）と `progress.txt`（`test-all exit=1 sec=290`）。コミット `d6d77336`（未コミットの変更なし）。

| 段 | 時間（秒） | 結果 |
|---|---|---|
| i686 の部品の用意（`add i686 target`） | 0 | 緑 |
| i686 の部品のビルド（`build i686 artifacts`） | 1 | 緑 |
| 整形の確かめ（`fmt --check`） | 5 | 緑 |
| x64 のワークスペースのテスト | 233 | 緑 |
| i686 のテスト（host-32） | 43 | 緑 |
| crates.io の公開前の確かめ | 2 | 緑 |
| 文字コードの確かめ（`encoding check`） | 5 | 赤 |
| **全体** | **290** | 終了コード 1 |

- 赤は文字コードの確かめの 1 段だけ。この spec の `tools/load-flake.ps1`（タスク 1.1）が、main から入った新しい決まり（道具の文字列はコメントの外で英数字だけ）に反して日本語の文字列を持っていた（5 件）。テストはすべて緑で、どの段も最後まで走ったので、290 秒は所要時間として使える（赤の段は 5 秒の静的な確かめで、時間への影響は無い）。
- `tools/load-flake.ps1` はコミット `0c046e92` で直し、`tools/encoding-check.ps1` は通るようになった。

### 5.6 直した後の線（タスク 6.1 で使う）

design の「Performance」の目安から、直す前の値で決まる線:

| 測るもの | 直す前 | 「目立って延びた」の線 | 直した後が満たすこと |
|---|---|---|---|
| 対象の族（3 回の中央値） | 中央値 64.1 秒・最大 66.4 秒 | 64.1 × 1.20 ＝ 76.92 秒。前の最大 66.4 秒はこの線より下なので、線は 76.92 秒のまま | 後の中央値が 76.92 秒以下 |
| 全体テスト | 290 秒 | 290 × 1.20 ＝ 348 秒 | 後が 348 秒以下 |

初めの線は 1.10 倍（対象の族 70.51 秒・全体テスト 319 秒）だった。GPU の装置の許可（タスク 5.2）を 2 つにすると負荷の下の赤は 0 件になったが、静かな机の中央値が 95.2 秒に延び、8 つでは 64.7 秒に収まったが負荷の下に赤が 2 件残った。両方を満たす数が 1.10 倍の線の内に無かったので、2026-10-08 に開発者が線を 1.20 倍へ改め、許可を 4 つにした（計時の記録は 6.1 で書く）。

main を取り込んだ後（4.3 の 5）は、「直す前」の値を取り直す。上の 64.1 秒と 290 秒は、取り込む前の main の 2,802 本（5.3）で測ったもので、取り込みで main から 80 本が入り、テストの顔ぶれが変わったため、そのまま比べられない。取り直しは、main の先端の実行ファイルと直した後の実行ファイルを、同じ机で交互に回して測る（4.3 の 8 と同じ形）。仮の読み: 4.3 の 8 の中央値で、直した後 79.1 秒 ÷ main の先端 67.4 秒 ＝ 1.17 で、1.20 倍の線の内（main の先端の 3 回の最大 72.9 秒は線 80.88 秒より下）。4.4 の深夜の回は RigPermit つきで 1.32 倍に延びた（その後 RigPermit を外した）ので、最後の値には使わない。最後の値は RigPermit を外した後に取り直し、4.4 の「最後の所要時間（RigPermit を外した後）」に書く（タスク 6.1）。

最後の値（2026-10-09・`37253505`）: 対象の族は main の先端 73.7 秒・直した後 76.1 秒（中央値）で 1.03 倍、線 88.4 秒の内。全体テストは 295 秒（直す前 290 秒の 1.02 倍・線 348 秒の内）・すべての段が緑。

## 6. 競合・os error 5 の結論・試して赤が 0 件だったこと

- 競合（タスク 5.3）: 本番の順序の取り違えは無かった。4.2 の進み 0 回の `［止まった］` は、相手のスレッドが始まれなかった・終われなかったもの（ローダーの錠の待ちのスタック）で、順序を見る材料ではない。main の取り込みの後の 5 件の赤（4.3）はどれも読み分けてテストの側で直した（タスク 5.4 の 2 度目の開き直し・5.6）。4.4 の最後の記録の 14 件は、すべて文言どおりの `［止まった］` で、確かめの食い違いは待ちが届かなかった印だけ（並びの入れ替えは無い）。競合の修正は無く、本番のファイルの差分は 0。
- os error 5 の結論: 下の「os error 5 の結論（タスク 4.4）」。直した後の最後の記録（4.4 の負荷の下 5 回）でも、後片付けの失敗の行は 0 行（os error 5 も 0）。
- 試して赤が 0 件だったこと: 当たらない。1.4 で赤が出た（5 回とも赤・81 件）ので、6.4 の分かれ道（再現しなかったとき）には入っていない。

### os error 5 の結論（タスク 4.4）

**結論**: 同じテストのプロセスの中の同時の利用による。テストの側（`sample-ghost-kit` の `devroot.rs`）の原因で、design「WorkDirCleanup」の見分けの表の 1（`Drop` の順）と 2（`gc-` の札）の両方に当たる。2 か所を直した（要件 5.2）。表の 3（テストの側でない）に当たりうる行は 1 行だけで、直す対象にはしない。

#### 行のパスを見分けの表に当てる

数えたのは 5.4（`target\load-flake\quiet-before-20261008-000446\round-1.log`〜`round-3.log`）と 3.6（`target\load-flake\before-20261008-001257\round-1.log`〜`round-5.log`）の行。行の数は `（次の走行で回収する）` の数で数え、種類と指す木は割り込みを戻して読んだ。

| 行の種類 | 指す木 | 静かな机 3 回 | 負荷の下 5 回 | 見分けの表の行 |
|---|---|---|---|---|
| 残骸の退避 | `work\<その回のプロセスの番号>-<連番>` | 623 | 344 | 1 |
| 残骸の退避 | `work\gc-…` | 4 | 0 | 2 |
| 残骸の退避 | `work\<前の回のプロセスの番号>-<連番>` | 1 | 0 | 3（下の「表の 3 に当たる 1 行」） |
| 退避した残骸の削除 | `work\gc-…` | 4 | 2 | 1・2 の巻き添え（下） |
| 作業フォルダの後始末 | `work\<その回のプロセスの番号>-<連番>` | 1 | 1 | 1 の巻き添え（下） |
| 生存の札の後始末 | `work\<その回のプロセスの番号>-<連番>.lock` | 0 | 2 | 1 の巻き添え（下） |
| 計 | | 633 | 349 | |

- 5.4 の「残骸の退避 628 行」は、上の 623＋4＋1。5.4 では木の形で分けていなかった。`gc-` の木を指す「残骸の退避」は静かな机の 2 回目にだけあり、936 行・938 行が `gc-29916-83`（934 行に同じ木の「退避した残骸の削除」の失敗がある）、3110 行・3111 行が `gc-29916-416`（同じ木の削除の失敗の行は無い）。負荷の下では 0（3.6 のとおり）。
- すべての行が os error 5（アクセスが拒否された）。他の番号の失敗は 0（5.4・3.6）。

#### 表の 1: `work\<番号>-<連番>` の「残骸の退避」が同時の利用である根拠

- 同じプロセスの中で、札を消せる（＝持ち主が居ないと見える）`<自分の番号>-<連番>` の木ができるのは、`impl Drop for WorkDir` の間だけ。直す前の `Drop` は札を閉じてから木を `remove_dir_all` で消していたので、消している間は札が誰にも握られていない。ほかの入口はどれも当たらない: `WorkDir::in_namespace` は札を先に作ってから木を作る。`LazyLock` の複製はプロセスの終わりまで札を握る。`stage_into_cache` の木は `rename` で棚から出る。消し残しの木（`Drop` の `remove_dir_all` が失敗したもの）は「作業フォルダの後始末」の行が出るが、静かな机と負荷の下で 1 行ずつしかない。
- だから、別のテストの取得が走らせる `sweep` は、消している最中の木の札を消せて（`owner_is_gone` が真）、その木を `discard_tree` で `gc-…` へ `rename` しようとする。`rename` は、子孫のファイルが開かれているフォルダを拒む（`discard_tree` の doc の実測）。消している側の `remove_dir_all` が中のファイルを開いているので、拒否が os error 5 になる。
- 同じ木を何度も指す行は、時間の上で固まっている。同じ木の最初と最後の行の間に挟まる `test … ok` の行の数は、どの回も中央値 0（90 パーセンタイルは 2〜9、最大は 6〜58）。消している間の短い窓に、並走する何本もの `sweep` が続けて当たる形で、木が長く居残っている形ではない。

#### 表の 2: `work\gc-…` の「残骸の退避」が同時の利用である根拠

- `discard_tree` は木を `gc-…` へ移してから消すが、`gc-` の木には札が無い。消している間に別のテストの `sweep` が「札の無い残骸」として同じ木を別の `gc-` へ移そうとし、拒まれて「残骸の退避」の行になる。`gc-29916-416` の 2 行がこの形。`gc-29916-83` の 2 行は、消す側の削除が先に失敗して札の無い木が残り、続く `sweep` が移そうとして拒まれた形（拒まれた理由は、消す側の手がまだ中に残っていたことと読める）。

#### 巻き添えの種類（読み）

次の 3 種類は、指す木だけでは表の行が決まらない。どれも「同じ木やその札を 2 人が同時に消している」ときに出る拒否（消しかけのファイルは、他の開き方を os error 5 で拒む）として読める。1 の窓で `sweep` の `rename` が通ってしまうと、消している側の `remove_dir_all`（手は中に残っている）と `sweep` の側の `remove_dir_all` が同じ木を同時に消す。2 の窓でも同じことが `gc-` の木で起きる。

- 退避した残骸の削除（`gc-…`）: 静かな机 4・負荷の下 2。
- 作業フォルダの後始末: 静かな机 1・負荷の下 1。
- 生存の札の後始末（`.lock`）: 負荷の下 2。札が閉じた後、`Drop` の札の削除と `sweep` の札の削除が重なった形。

どれも 1 と 2 の窓が閉じれば起きない（札が握られている間は、`sweep` は木にも札にも触らない）。

#### 表の 3 に当たる 1 行

静かな机の 2 回目の 123 行は、1 回目のテストのプロセス（12888）が残した木 `work\12888-311` の「残骸の退避」の失敗（2.4 の組 D）。1 回目のプロセスはもう終わっていて、2 回目のプロセスが始まった直後の行。始まった直後は、何本ものテストが同時に `sweep` して同じ残り物を移そうとするので、2 本の `sweep` が同じ木の `rename` で重なったか、ウイルス対策などの外の手が触れていたかのどちらかで、記録からは分けられない。どちらでも、先に移せた `sweep` か次の取得の `sweep` が片付ける。1,000 行近くのうち 1 行で、テストの側の妨げ合いの形（生きている木や消している最中の木を横から移す）ではないので、直す対象にしない（要件 5.3）。負荷の下の 5 回には、この形の行は無い。

#### 赤との関わり

3.6 のとおり。赤の文言にファイルの拒否は 0 件で、赤のテストの木を指す後片付けの行は、どれもその赤の `panicked at` の行より後に出ている。後片付けの失敗が赤を起こした形は見当たらない。今回の直しは、後片付けの行そのものを消すためのもの（要件 5.2）。

#### 同じウェーブで `sample-ghost-kit` を触る spec が無いことの確かめ（触る前）

- `git worktree list` の 11 個のワークツリー（main を含む）について、`git diff --stat main...<ブランチまたはコミット> -- crates/sample-ghost-kit` はすべて差分なし（名前のあるブランチ 8 本と、名前の無い 3 つ `97179d8f`・`5af6fbd4`・`408b3ad1`）。各ワークツリーの `git status --porcelain -- crates/sample-ghost-kit` も、すべて未コミットの変更なし。
- `.kiro/specs/*/tasks.md`・`design.md`・`brief.md`（`completed` を除く）で `sample-ghost-kit` を挙げるのは、この spec だけ。`.kiro/steering/roadmap.md` の 2 か所は、完了した spec の申し送り（`strip_folder`）と、この spec が引き取った os error 5 の記録だけ。

#### 直したこと（`crates/sample-ghost-kit/src/devroot.rs`）

- 表の 1: `WorkDir` の破棄の順を「木を消す → 札を閉じる → 札を消す」へ替えた。破棄の手順は、木を消す関数を受け取る私的な関数 `WorkDir::release_with` に分け、`Drop` は `remove_dir_all` を渡す。
- 表の 2: `discard_tree`（中身は `discard_tree_with`）が、移す前に `gc-<番号>-<連番>.lock` を札と同じ開き方（`create_new`・読むことだけを共有）で作り、木を消し終えてから閉じて消す。札を作れないときは移さずに残し、次の取得の `sweep` に任せる（そのときは「…の退避の札の作成に失敗した」の行が出る）。`sweep` は今の決まりのまま（札が消せない木は触らない）で、握られた札のある `gc-` の木を避ける。
- 札の開き方と、閉じて消す手順は、`open_lease`・`retire_lease` の 2 つの関数にまとめ、作業フォルダと `gc-` の木の両方がそれを使う。取得の形（`SampleRoot::acquire`・`WorkDir::new` の外から見える振る舞い）は変えていない。

#### 檻（`crates/sample-ghost-kit/src/devroot_sweep_tests.rs`）

- 檻 11 `a_work_dir_keeps_its_lease_while_its_tree_is_being_removed`: `release_with` に渡した「木を消す関数」の中から札の削除を試み、共有違反（os error 32）で拒まれること。破棄の後に棚に木も札も残らないこと。直す前は札の削除が通って赤（`木を消している間、札は共有違反で削除を拒むこと: Ok(())`）。
- 檻 12 `a_tree_moved_aside_keeps_a_held_lease_until_it_is_removed`: `discard_tree_with` に渡した「木を消す関数」の中で `sweep` を走らせても `gc-` の木が移されず、その札 `gc-….lock` が棚にあること。消し終えたら `gc-` の木も札も棚に残らないこと。直す前は並走する `sweep` が `gc-` の木を移してしまい赤（`消している間は gc- の木に握られた札があり、並走する掃除が退けないこと: []`）。
- 兄弟のテスト（`a_sweeper_running_alongside_never_disturbs_a_staging_tree_or_a_live_copy`・`the_lease_refuses_deletion_while_it_is_held` を含む `sample-ghost-kit` の全部）と `cargo test -p areka --bin areka`（2,812 本通過・無視 2 本＝2,814 本）は緑。

#### 直した後の目安（正式な記録ではない）

負荷なしで `cargo test -p areka --bin areka -- --nocapture` を 1 回だけ（`tools/load-flake.ps1` を通さずに）回した。後片付けの失敗の行は 0 行（直す前の静かな机は 1 回 191〜222 行）。較正: 同じ出力に、緑のテストがわざと起こす `panicked at` の行が 12 行あり、緑のテストの標準エラーは捨てられていない。正式な直した後の数は、タスク 6.1（静かな机）と 5.1（負荷の下）が下の数え方で取る。

#### 直す前と後の数え方（タスク 1.3・1.4 の申し送り）

- `tools/load-flake.ps1` の数え方を「行の頭が `sample-ghost-kit:`」から「行のどこかに `sample-ghost-kit:` がある」へ替えた（`Read-Round` の 1 か所）。`report_cleanup` の 1 回の出力に、この綴りはちょうど 1 つあるので、割り込みで行の頭がずれても 1 件として数えられる。直す前の 8 回の記録をこの決まりで数え直すと、どの回も「戻して読んだ数」（`（次の走行で回収する）` の数）と一致する（静かな机 222・220・191、負荷の下 45・94・64・70・76）。だから直す前と後は、`summary.txt` の `cleanup_lines` と、上の 5.4・3.6 の「戻して読んだ数」（静かな机 3 回の計 633・負荷の下 5 回の計 349）で比べる。
- `summary.txt` の `os_error_5` は、今も同じ行に `os error 5` がある行だけを数えるので、`(os error` と `5))` が行をまたぐと少なく出る（下限）。直した後に `cleanup_lines` が 0 でなければ、os error 5 の数は `round-N.log` の `アクセスが拒否されました` の数で取り、行のパスを上の表に当てる。道具の文字列は英数字だけの決まり（`tools/encoding-check.ps1`）なので、日本語の文の数え方は道具に入れない。
