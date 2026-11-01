# 測りの記録: areka-test-binary-stability

> 条件を先に書き、結果を後から足す。重い走行（全体テスト・負荷の走行）の前に所要時間の見込みを開発者へ伝え、伝えたことをその節に書く（要件 9.2）。

**基準の採り直しの手順（要件 9.5）**: 測りの途中で Adjacent の spec（`temp-path-exceptions-to-target`・`install-live-target-hazards`・`background-result-tick-wake`・`balloon-element-order`・`mcp-user-response`・`zorder-chain-residue`）が main に着地したら、その時点の main を取り込んでから、静かな机の全体テストを 1 回回し直して ⑷ を採り直す（前の値は消さず、採り直した値を並べて書く）。⑾ の 1.20 倍の線は、最後に採った ⑷ と比べる。

## ⑴ 机と道具の確かめ

- 2026-11-01: 置き場の `areka-impl-watch.exe` は 2026-10-19 18:25 に置かれた。main（`5f5a654d`）で `crates/areka-impl-watch` を最後に変えたコミットは `406a4a57`（2026-10-19 08:28）で、それより後に置かれている＝main の最新と揃う。以後の main の変更（`0cd4577f`→`5f5a654d`）は steering の文書 3 ファイルだけ。
- 机の id は `kiro-start-areka-test-binary-stability-dec94e`・repo は `areka`。全体テストは排他テストの机と単独の机（`test,sole`・`stopped: none`）を握って回す。

## ⑵ 待ちの数え直し

- 数えたコミット: `66f91513`（2026-11-01）。pathspec の較正: `git ls-files crates/areka/src | wc -l` が 706 件、下のどの grep も 1 件以上を返す（0 件の行は無い）。

**目印の無い待ち ⑴ `spin_wait_until` の直の呼び出し（15 か所）**

コマンド: `git grep -n "spin_wait_until(" -- crates/areka/src`（定義 `emo2_boot/spine_wait.rs:338` の 1 行を除く）

| ファイル（`crates/areka/src/` の下） | 行 |
|---|---|
| `emo2_boot/spine_boot_smoke_tests.rs` | 35・131 |
| `emo2_boot/spine_close_wiring_tests.rs` | 91 |
| `emo2_boot/spine_conformance_support_tests.rs` | 470 |
| `emo2_boot/spine_conformance_support_stage_tests.rs` | 96・157・277 |
| `emo2_boot/spine_hold_tests.rs` | 39 |
| `emo2_boot/spine_seriko_loop_tests.rs` | 233 |
| `session_end_deadline_tests.rs` | 111・139・331・354 |
| `mcp/dump_surface.rs`（`WaitAnswer`） | 415 |
| `ghost_session_restart_tests.rs` | 74 |

- design「WaitMigration（15 か所）」の表との違い: ファイルを分けた後、表の「470・635」「元 696・816」は今 `spine_conformance_support_tests.rs` の 470 と `spine_conformance_support_stage_tests.rs` の 96・157・277 になった（数は 15 のまま）。

**目印の無い待ち ⑵ 段の駆動器の待ち（`StageSink::progress_probe` の既定 `None` を通る）**

コマンド: `git grep -n -e "impl StageSink for" -e "fn progress_probe" -- crates/areka/src/emo2_boot`

- 既定: `emo2_boot/spine_conformance_support.rs:438`（`None` を返す）。待ちは同じファイルの 546（`sink.progress_probe()`）→ 556（`wait_until_with`・`None` なら `Progress::Unknown`＝`［進みは不明］`）。
- 既定のまま（`progress_probe` を書いていない）の実装: `emo2_boot/spine_conformance_support.rs:443`（`SpineHarness`）・`emo2_boot/spine_conformance_support_tests.rs:83`（`FakeStageSink`）・`emo2_boot/spine_conformance_support_tests.rs:565`（`HoldingSink`）。
- 目印を書いている実装は `emo2_boot/spine_conformance_lap_tests.rs:270`（`LapSink`）だけ。

**その外の文言の無い待ち（要件 7.6・数えて記録するだけ・本 spec の走行で赤になったものだけ移す）**

自前の `recv_timeout` — コマンド: `git grep -n "recv_timeout(" -- crates/areka/src`（17 件。待ちの芯の中の `emo2_boot/spine_wait.rs:297`（`wait_recv`）・`:373`（`run_bounded`）と、本番の `perf_thread_report.rs:295`・`:384` の 4 件を除く 13 件）

| ファイル | 行 |
|---|---|
| `emo2_boot/film_playback_e2e_tests.rs` | 290 |
| `emo2_boot/frame/drain_resnap_tests.rs` | 111 |
| `emo2_boot/frame_scale_notice_integration_tests.rs` | 254 |
| `emo2_boot/install_cue_tests.rs` | 263 |
| `ghost_session_reload_tests.rs` | 45 |
| `ghost_session_runtime_tests.rs` | 51 |
| `install/desk_pick_tests.rs` | 155・160 |
| `install/fetch_url_tests.rs` | 180 |
| `mcp/check_script_tests.rs` | 205 |
| `mcp/get_expression_table_tests.rs` | 729 |
| `roots/desk_tests.rs` | 155・160 |

`Instant` の締切 — コマンド: `git grep -nE "Instant::now\(\) *\+" -- crates/areka/src`（18 件。本番の起こしの予約 `emo2_boot/balloon_visibility_phase.rs:120`・`input_events/char_watch.rs:254` と、待たずに締切の値を作るだけの `menu/trigger_show_tests.rs:526`・`menu/trigger_tests.rs:191` の 4 件を除く 14 件）

| ファイル | 行 |
|---|---|
| `emo2_boot/frame/switch_tests.rs` | 641 |
| `emo2_boot/frame_ghost_quit_logsink_tests.rs` | 78 |
| `emo2_boot/spine_close_wiring_tests.rs` | 113・166 |
| `emo2_boot/spine_display_tests.rs` | 30 |
| `emo2_boot/spine_move_cue_tests.rs` | 117 |
| `emo2_boot/spine_seriko_loop_tests.rs` | 58・88・191・452 |
| `emo2_boot/spine_talk_close_tests.rs` | 262 |
| `emo2_boot/spine_third_scope_talk_tests.rs` | 241 |
| `readme/opener_submit_tests.rs` | 198 |
| `thread_roles_tests.rs` | 26 |

`run_bounded`（総時間で打ち切る古い呼び名）の呼び手 — コマンド: `git grep -n "run_bounded(" -- crates/areka/src`（定義 `emo2_boot/spine_wait.rs:366` は `run_bounded<F` の形なのでこの grep に当たらない）: `session_end_sync_send_tests.rs:205` と `emo2_boot/spine_wait_tests.rs:310`（`run_bounded` そのものの檻）の 2 件。`run_bounded_watching`（`Progress` を渡す・文言あり）の呼び手は数えない。

## ⑶ 件 5 の赤の文

- 条件（design「測りの条件」の 件 5 の行）: 範囲 `session_mark`（`areka` の実行ファイル・`--list` の部分一致で 26 本＝`session_mark_tests` 12・`session_mark_tests::fallback` 5・`session_mark_write_tests` 4・`boot_resolve::boot_resolve_tests::last_used` 2・`config_input_tests::switch_balloon` 2・`boot_config::callghost_tests` 1）・負荷なし・既定のスレッドの数・10 回・1 回の上限 10 分。排他テストの机（`test`）を握って回す。
- コマンド: `pwsh -NoProfile -File tools/load-flake.ps1 -Label case5 -Burners 0 -Rounds 10 -RoundTimeoutMin 10 -Filter session_mark`。`-NoCapture` は付けない（赤のテストの捕まえた出力と panic の文は libtest の `failures:` の囲みに残るので要らない・付けると緑のテストの出力が混ざる）。`-RoundTimeoutMin` は道具の既定が 30 分なので 10 を明示する。
- 見込み: 1 回 1 分未満（26 本・実行ファイルは作り済み）、10 回で約 5 分（上限は 10 回 × 10 分）。
- 伝えた見込み: 10 回で約 5 分（2026-11-01 16:33・会話で開発者へ）。
- 結果（2026-11-01 16:34・コミット `56d4f20d`（`conditions.txt` は dirty: yes）・`test` を握って 1 度だけ）: 10 回とも緑。毎回 26 passed・0 failed・終了コード 0・1 回 6.0〜6.7 秒・`待ちの打ち切り` の行 0・後片付けの失敗の行 0・`AREKA-CRASH` 0。記録は `target\load-flake\case5-20261101-163446\summary.txt`（回ごとの出力は同じフォルダの `round-1.log`〜`round-10.log`）。
- 走った時の未コミットの変更は `load-repro.md` だけ（並走のタスク 6.1 が `crates/areka` を触り始めたのは走行の後・`spine_wait_tests.rs` の更新は 16:40）なので、走った実行ファイルは `56d4f20d` のコードと同じ。⑵ の行の番号は `66f91513` で数えたもので、`crates/areka/src` は `66f91513` と `56d4f20d` で同じ。
- 赤の 2 本の文は 10 回で 1 度も出なかったので、写す文は無い。要件 8.7 に沿って、条件と結果を書いてひとまず閉じる。件 5 は、最後の負荷 5 回（タスク 7.2）でもう一度見る。

## ⑷ 修正の前の静かな机の全体テストの所要時間

- 伝えた見込み: 約 30 分（2026-11-01 15:29・会話で開発者へ）。
- 条件: コミット `7f1adece`（main `0cd4577f` の上に spec の文書だけ）・未コミット 0・`test,sole` を握る（`stopped: none`）・`pwsh -NoProfile -File tools/test-all.ps1`（引数なし）。
- 結果: 全段緑。2026-11-01 15:30:20 → 15:44:52。

| 計り | 秒 |
|---|---|
| 全体（`tools/test-all.ps1` の段の合計） | 872 |
| うち `x64 workspace tests` | 734 |
| `areka` の実行ファイル（`src\main.rs` の unittests・libtest の `finished in`） | 93.83（4153 passed・5 ignored） |

- 1.20 倍の線（⑾ で比べる）: 全体 1046 秒・`areka` の実行ファイル 112.6 秒。

## ⑸ 較正

- 伝えた見込み: 3 回で約 5 分（2026-11-01 16:19・会話で開発者へ）。排他テストの机（`test`）を握り、負荷なし（`-Burners 0`）・各 1 回・`-Ignored`（`--ignored --exact`）。据え付けは `.CRT$XCU` の初期化子のまま（足場の入口への替えは要らなかった）。
- 位置は較正のテストの中の `RaiseException(0xC0000005)` なので、`module=` は `KERNELBASE.dll`（`RaiseException` の中）を指す。

**areka**（`target\load-flake\calib-areka-20261101-162013`・終了コード `-1073741819`＝`0xC0000005`・1.3 秒・報告のあった回 1）:

```
AREKA-CRASH code=0xC0000005 addr=0x7fff0289569a tid=33756
AREKA-CRASH module=C:\WINDOWS\System32\KERNELBASE.dll+0xC569A thread=""
AREKA-CRASH-HOLDER tid=35192 name="emo2_boot::crash_report_test_support::crash_report_names_the_running_test"
AREKA-CRASH-THREAD tid=39244 name="main"
AREKA-CRASH-THREAD tid=35192 name="emo2_boot::crash_report_test_support::crash_report_names_the_running_test"
AREKA-CRASH-THREAD tid=33756 name=""
（ほか名前の無いスレッド 6 本）
AREKA-CRASH-STACK    4: areka::emo2_boot::crash_report_test_support::report
AREKA-CRASH-STACK   11: RaiseException
AREKA-CRASH-STACK   12: areka::emo2_boot::crash_report_test_support::crash_report_names_the_running_test::closure$0
```

- 落ちたスレッド（`tid=33756`）は名前なし、持ち手の行と生きているスレッドの一覧に較正のテストの名前が出た。`GraphicsCore` を作った後（GPU の DLL を読み込んだ後）も受け手が生きている。

**wintf の lib**（`target\load-flake\calib-wintf-20261101-162023`・終了コード `0xC0000005`・9.1 秒・報告のあった回 1）:

```
AREKA-CRASH code=0xC0000005 addr=0x7fff0289569a tid=2896
AREKA-CRASH module=C:\WINDOWS\System32\KERNELBASE.dll+0xC569A thread=""
AREKA-CRASH-THREAD tid=31504 name="main"
AREKA-CRASH-THREAD tid=25036 name="test_crash_report::crash_report_names_the_running_test"
AREKA-CRASH-THREAD tid=2896 name=""
（ほか名前の無いスレッド 3 本）
AREKA-CRASH-STACK    4: wintf::test_crash_report::report
AREKA-CRASH-STACK   12: wintf::test_crash_report::crash_report_names_the_running_test::closure$0
```

**areka・許可の数 1**（`-SetEnv AREKA_TEST_GPU_SLOTS=1`・`target\load-flake\calib-areka-slots1-20261101-162336`）: `conditions.txt` に `set_env: AREKA_TEST_GPU_SLOTS=1`、持ち手の表は 1 行（`AREKA-CRASH-HOLDER tid=37876 name="...crash_report_names_the_running_test"`）。注意: 較正のテストは許可を 1 つしか取らないので、表が 1 行なのは許可の数 4 の走行と同じ。子に値が届くことの直の証は `conditions.txt` と、子の環境だけに置く `load-flake.ps1` の作り（2.3 の査読）で、値そのものの働きは 4.2 の X2 で見る。

## ⑹ X1〜X3・Y1〜Y2

**X1 の範囲**（2026-11-01・コミット `66f91513`）

- 拾い方: `git grep -lE "CoInitializeEx|GpuPermit::take|GraphicsCore::new" -- crates/areka/src` で 29 ファイル。モジュールの道は、各ファイルの `#[test]` の関数の名前を `cargo test -p areka --bin areka -- --list`（4161 本）の行の頭と突き合わせて決めた（ファイルの名前からは決めない）。
- X1 に入れない本番のコード 6 ファイル: `emo2_boot/switch_assets.rs`（資産づくりのスレッド）・`install/pick.rs`・`roots/pick.rs`・`readme/os_port.rs`・`offline_dump/mod.rs` の本体（同じ所のテストは下の `offline_dump::offline_dump_tests`）・`emo2_boot/spine_wait.rs`（注釈だけ）。
- テストを持たない足場 5 ファイル（呼び手の代わりに開く）: `emo2_boot/spine.rs`（`make_world_with_gpu`＝`SpineHarness`）・`emo2_boot/ghost_switch_test_support.rs`（`SwitchRig::new` の COM）・`emo2_boot/frame_balloon_runtime_test_support.rs`（呼び手は `emo2_boot::frame` の中だけ）・`mcp/dump_surface_gpu_test_support.rs`・`placement/placement_shared_test_support.rs`。足場の道で絞ると 0 本になるので、足場の分は呼び手のモジュールで入れる（下の「足場を通して開く」）。

**自分で開く**（15 の道・579 本）

| モジュールの道（`-Filter` の 1 つ） | 本数（`--list` の部分一致） | 元のファイル |
|---|---|---|
| `emo2_boot::frame` | 456 | `frame_attach_tests.rs`・`frame_visibility_integration_tests.rs`・`frame_scale_notice_integration_tests.rs`（`emo2_boot::frame::visibility_integration_tests::scale_notice_integration_tests`）と、`frame_balloon_runtime_test_support.rs` の呼び手を含む frame の族 |
| `emo2_boot::assets::` | 18 | `assets_tests.rs`（`::tests`）・`assets_shell_tests.rs`（`::shell_tests`）・`assets_markers_tests.rs`（`::markers_tests`） |
| `emo2_boot::crash_report_test_support` | 2 | `crash_report_test_support.rs` |
| `emo2_boot::film_playback_e2e_tests` | 3 | `film_playback_e2e_tests.rs` |
| `emo2_boot::wire_tests` | 3 | `emo2_boot/mod.rs` の `wire_tests` |
| `emo2_boot::runtime_boot::tests` | 9 | `runtime_boot_tests.rs`（5 本）＋子の `readme_script_e2e`・`script_security_e2e`（4 本） |
| `emo2_boot::switch_assets::tests` | 8 | `switch_assets_tests.rs` |
| `ghost_session::reload_tests` | 2 | `ghost_session_reload_tests.rs` |
| `ghost_session::restart_tests` | 8 | `ghost_session_restart_tests.rs` |
| `ghost_session::shell_balloon_switch_session_lap_tests` | 3 | `shell_balloon_switch_session_lap_tests.rs` |
| `input_events::balloon::choice_settle_tests` | 4 | `balloon_choice_settle_tests.rs` |
| `offline_dump::offline_dump_tests` | 13 | `offline_dump/offline_dump_tests.rs` |
| `offline_dump::render::render_tests` | 19 | `offline_dump/render_tests.rs` |
| `placement::measure::tests` | 27 | `placement/measure_tests.rs` |
| `png_image::tests` | 4 | `png_image_tests.rs` |

- 合わせて 579 本（重なり無し）。`emo2_boot::crash_report_test_support` の 2 本のうち、GPU を開く較正（`crash_report_names_the_running_test`）は `#[ignore]` なので `-Ignored` を付けない X1 では走らず、走るのは据え付けの確かめ（`crash_filter_is_installed_before_the_tests_start`）の 1 本だけ。

**足場を通して開く**（55 の道・自分で開く分と重ならないのは 410 本）

- 入れる訳: design「測りの条件」X1 の狙いは「COM と GPU の装置を開くテスト」で、ファイルの名前ではない。これまでの落ちも足場を通して開くテスト（`SpineHarness` の `emo2_boot::spine::*`・`SwitchRig` の族）で出た。
- 拾い方: 足場の入口ごとに `git grep -l` で呼び手のファイルを拾い（`SpineHarness::`・`SwitchRig::`・`GpuRig`・`with_com_initialized`・`balloon_runtime_test_support::`）、自分で開く分と同じ突き合わせでモジュールの道に直した。道は、装置を開かない大きな組みを引き込まない範囲で短い頭にまとめた。たとえば `emo2_boot::spine::` は `wait_tests`・`hold_support::tests`・`settle_tests` の 24 本を引き込み、`ghost_session::` は `scope_rect`・`scope_scale` の 27 本を、`placement::` は 1203 本を引き込むので、まとめずに子の道で書く。
- 開くもの: `SpineHarness`（`emo2_boot/spine.rs` の `make_world_with_gpu`）・`GpuRig`（`mcp/dump_surface_gpu_test_support.rs`）・`manual_balloon_runtime`（`emo2_boot/frame_balloon_runtime_test_support.rs`）は COM と GPU の装置を開く。`SwitchRig::new`（`emo2_boot/ghost_switch_test_support.rs:208`）と `with_com_initialized`（`placement/placement_shared_test_support.rs:14`）は COM（WIC）だけで、GPU の装置は開かない。それでも COM を開くテストとして入れる。

| 足場 | 呼び手のモジュールの道（`-Filter` の 1 つずつ・括弧は `--list` の部分一致の本数） |
|---|---|
| `SpineHarness` | `emo2_boot::spine::boot_smoke_tests`（2）・`emo2_boot::spine::close_wiring_tests`（1）・`emo2_boot::spine::conformance_lap_tests`（1）・`emo2_boot::spine::conformance_support::driver_tests`（9）・`emo2_boot::spine::conformance_support::stage_tests`（2）・`emo2_boot::spine::display_tests`（4）・`emo2_boot::spine::move_cue_tests`（1）・`emo2_boot::spine::seriko_loop_tests`（5）・`emo2_boot::spine::talk_close_tests`（2）・`emo2_boot::spine::text_scale_tests`（3）・`emo2_boot::spine::third_scope_talk_tests`（1）。`ghost_session::reload_tests`・`restart_tests` は自分で開く分に入っている |
| `SwitchRig` | `emo2_boot::ghost_switch::balloon_tests`（6）・`emo2_boot::ghost_switch::boot_event_tests`（3）・`emo2_boot::ghost_switch::reload_tests`（11）・`emo2_boot::ghost_switch_test_support`（3）・`emo2_boot::reload::`（21）・`boot_resolve::boot_resolve_mark_text_tests`（3）・`ghost_session::boot_shell_tests`（5）・`ghost_session::ghost_session_runtime::tests`（2）・`ghost_session::ghost_session_scope_rect::tests::boot`（2）・`ghost_session::ghost_session_scope_scale::tests::boot`（3）・`ghost_session::strict_tests`（3）・`ghost_session::switch_tests`（15）・`ghost_session::switch_translate_tests`（1）・`ghost_session::shell_balloon_switch_session_`（15）・`install::desk::overwrite_tests`（7）・`install::desk::record_tests`（7）・`install::names::tests`（5）・`install::worker::worker_tests`（16）・`session_mark_tests`（17）・`session_mark_write_tests`（4）・`session_end::`（21）・`readonly::run_tests`（1）・`readonly::sites_tests`（1）・`update::desk::reload_tests`（6）・`update::worker_path_tests`（2）・`write_failure::sites_tests`（1）・`mcp::check_script::check_script_tests`（6）・`mcp::get_expression_table::get_expression_table_tests`（20）・`mcp::get_property::get_property_tests`（4）・`mcp::get_status::get_status_tests`（6）・`mcp::mcp_tests`（19）。`emo2_boot::frame::*` の呼び手は自分で開く分の `emo2_boot::frame` に入っている |
| `GpuRig` と `SwitchRig`（mcp の dump） | `mcp::dump_balloon::dump_balloon_`（10＝`dump_balloon_tests`・`dump_balloon_gpu_tests`）・`mcp::dump_surface::dump_surface_`（25＝`dump_surface_tests`・`dump_surface_gpu_tests`） |
| `with_com_initialized` | `placement::balloon_offset_supply_tests`（16）・`placement::face_windowposition::tests`（3）・`placement::measure::`（49・`::tests` の 27 本は自分で開く分と重なる）・`placement::char_clamp_tests`（2）・`placement::prepare_`（21）・`placement::windowposition_tests`（6）・`placement::reseed::tests`（7）・`placement::reset_`（12）。`emo2_boot::frame::window_link_drain_tests` は `emo2_boot::frame` に入っている |
| `manual_balloon_runtime` | `emo2_boot::balloon_visibility::phase::box_tests`（10）・`input_events::balloon::pressed::tests`（6）・`input_events::balloon_wheel::tests`（6）。frame の中の呼び手は `emo2_boot::frame` に入っている |

**X1 の全体**: 70 の道・989 本（自分で開く 579＋足場を通して開く 410・`--list` の 4161 本に部分一致で数えた）。`load-flake.ps1` の `-Filter` にそのまま渡す形:

```
-Filter emo2_boot::frame,emo2_boot::assets::,emo2_boot::crash_report_test_support,emo2_boot::film_playback_e2e_tests,emo2_boot::wire_tests,emo2_boot::runtime_boot::tests,emo2_boot::switch_assets::tests,ghost_session::reload_tests,ghost_session::restart_tests,ghost_session::shell_balloon_switch_session_lap_tests,input_events::balloon::choice_settle_tests,offline_dump::offline_dump_tests,offline_dump::render::render_tests,placement::measure::tests,png_image::tests,emo2_boot::spine::boot_smoke_tests,emo2_boot::spine::close_wiring_tests,emo2_boot::spine::conformance_lap_tests,emo2_boot::spine::conformance_support::driver_tests,emo2_boot::spine::conformance_support::stage_tests,emo2_boot::spine::display_tests,emo2_boot::spine::move_cue_tests,emo2_boot::spine::seriko_loop_tests,emo2_boot::spine::talk_close_tests,emo2_boot::spine::text_scale_tests,emo2_boot::spine::third_scope_talk_tests,emo2_boot::ghost_switch::balloon_tests,emo2_boot::ghost_switch::boot_event_tests,emo2_boot::ghost_switch::reload_tests,emo2_boot::ghost_switch_test_support,emo2_boot::reload::,boot_resolve::boot_resolve_mark_text_tests,ghost_session::boot_shell_tests,ghost_session::ghost_session_runtime::tests,ghost_session::ghost_session_scope_rect::tests::boot,ghost_session::ghost_session_scope_scale::tests::boot,ghost_session::strict_tests,ghost_session::switch_tests,ghost_session::switch_translate_tests,ghost_session::shell_balloon_switch_session_,install::desk::overwrite_tests,install::desk::record_tests,install::names::tests,install::worker::worker_tests,session_mark_tests,session_mark_write_tests,session_end::,readonly::run_tests,readonly::sites_tests,update::desk::reload_tests,update::worker_path_tests,write_failure::sites_tests,mcp::check_script::check_script_tests,mcp::dump_balloon::dump_balloon_,mcp::dump_surface::dump_surface_,mcp::get_expression_table::get_expression_table_tests,mcp::get_property::get_property_tests,mcp::get_status::get_status_tests,mcp::mcp_tests,placement::balloon_offset_supply_tests,placement::face_windowposition::tests,placement::measure::,placement::char_clamp_tests,placement::prepare_,placement::windowposition_tests,placement::reseed::tests,placement::reset_,emo2_boot::balloon_visibility::phase::box_tests,input_events::balloon::pressed::tests,input_events::balloon_wheel::tests
```

- 拾いの限り: 呼び手は足場の入口の名前の grep で拾ったので、別の名前の手伝いの関数を通して足場を作るファイルは漏れうる。

## ⑺ 原因の経路と直し

## ⑻ 許可の待ちの上限の値と決めた訳

## ⑼ 件 4 の見立てと根拠

## ⑽ third scope の読み分け

## ⑾ 修正の後の静かな机の全体テストと 1.20 倍の線

## ⑿ 最後の負荷 5 回の赤の読み分け
