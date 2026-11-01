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
- 6.4 の後: 既定 `None` を消し、4 つの実装の全部が `progress_probe` を書く（`SpineHarness`＝SHIORI の呼び出しの数＋採った表示の指令の数・`FakeStageSink`／`HoldingSink`＝受けた注入の数・`LapSink` は今どおり）。**例外（要件 7.3）**: `spine_conformance_support_tests.rs` の `stage_driver_returns_named_timeout_instead_of_silent_success` だけは `FakeStageSink::silent_unmarked`（`None`）で回す。この檻は打ち切りの文言が `［進みは不明］` で始まることそのものを確かめるので、目印を持つと打ち切りが `［止まった］` になり確かめたい形を踏めない。

- 6.5 の後: ⑴ の残り 11 か所を目印つきの待ち（`wait_until`／`wait_recv`・`Progress::Count`）へ移した。目印は、spine の起動・閉じ・保持＝SHIORI の `call_count()`（起動の字の層の待ちはそれに捕まえたログの行の数を足す）、seriko の輪＝受けた指令の数、終わりの締切＝`call_count()`（354 の予約の発火は `unblock_calls()`）、作業プールの見張り（`ghost_session_restart_tests.rs`）＝テストの側に足した口（`Input` 段の吸い出しを包んだ `drain_counting` が届いた閉包の数を `AtomicU64` で数える・要件 7.3。作業プールのスレッドで走り始めた数は製品の閉包の中なので数えていない）。MCP の答えの待ちは `wait_answer(&self, progress: Progress<'_>)` にし、呼び手 13 か所（design の「8 か所」を行で数え直すと 13）がそれぞれ自分の足場の目印を渡す（`GpuRig`・`SwitchRig`＝`progress_probe()`、`dump_surface_tests.rs` の符号化のスレッドの檻＝仕事が走り始めた数）。grep `spin_wait_until(` は定義の 1 行だけになった。
- **例外（要件 7.3）**: `mcp/check_script_tests.rs` の `dispatch_and_wait` は `Progress::Unknown` で待つ。空の World には数える足場が無く、答えの仕事は製品の中の閉包で符号化のスレッドへ渡るので、テストの側から走り始めを数える口も足せない（口を足すには製品の `check_script::handle` に継ぎ目が要る）。

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

- 6.5 で移したもの（要件 7.6・本 spec の走行で赤になったもの）: `emo2_boot/frame/drain_resnap_tests.rs` の 111（3 秒の自前の `recv_timeout`）だけ。X1 の r12 で「drain が戻らない: Timeout」の赤になった（⑹ の「X1 のほかの赤」）。`wait_recv` へ移し、目印は drain が適用した数。⑹ のほかの `［止まった］` の赤は既に目印つきの待ちで、この 3 つの表の行ではない（`ghost_switch_test_support.rs`・`main_session_mark*` に当たるものは 6.6）。残りの行は完了のときの起票の材料のまま。

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

**較正の取り直し**（2026-11-01 20:59〜21:00・最初の受け手（`AddVectoredExceptionHandler`）を足した後・作業ツリーの直し（未コミット）・負荷なし・各 1 回・`-Ignored`）

- 訳: Y1 の落ち 4 回が報告を出さなかったので、AV を最初の受け手で書く形にした（design「CrashReport」）。受け手が替わったので較正し直した。
- **areka**（`target\load-flake\calib2-areka-20261101-205950`・終了コード `0xC0000005`・1.2 秒・`AREKA-CRASH` の行 51・`code=` の行 1）: 落ちたスレッドは名前なし、持ち手の行は `AREKA-CRASH-HOLDER tid=37944 name="emo2_boot::crash_report_test_support::crash_report_names_the_running_test"`、生きているスレッドは 9 本（`main` と較正のテストの名前のスレッドを含む）。積み上げは `4: ...::write_report`・`5: ...::first_chance`・`9: RaiseException`・`10: ...::crash_report_names_the_running_test::closure$0`。
- **wintf の lib**（`target\load-flake\calib2-wintf-20261101-210013`・終了コード `0xC0000005`・0.5 秒・`AREKA-CRASH` の行 47・`code=` の行 1）: 生きているスレッドは 6 本（`main` と `test_crash_report::crash_report_names_the_running_test` を含む）。積み上げは `4: wintf::test_crash_report::write_report`・`5: wintf::test_crash_report::first_chance`・`8: RtlRaiseException`・`9: RaiseException`・`10: ...::closure$0`。
- どちらも報告を書いたのは最初の受け手（`first_chance`）で、最後の受け手は AV を書き直さない（`code=` の行が 1 本）。

**最後の形での較正**（2026-11-01 21:18〜21:19・進行役が排他テストの机を握って回した・伝えた見込み: 約 2 分（21:15 頃・進行役から開発者へ）・コミット `7bfc2b22` の上の作業ツリーの直し（`dirty: yes`・行を 1 度の書き込みで出す形・`AREKA-CRASH region` の行・`MEMORY_BASIC_INFORMATION` の大きさの確かめまで入った形）・負荷なし・各 1 回・`-Ignored`）

- 訳: 上の「較正の取り直し」と Y1 の 2 回目は、行を 1 度の書き込みで出す形と region の行を入れる前のコードで回した（積み上げの `write_report` の行の番号が今のソースと合わない）。要件 1.2 の較正を、4.2 の前に最後の形でやり直した。
- **areka**（`target\load-flake\calib3-areka-20261101-211855`・終了コード `0xC0000005`・0.8 秒）: `AREKA-CRASH` の行 51・`code=` の行 1・持ち手の行 1（`AREKA-CRASH-HOLDER tid=1908 name="emo2_boot::crash_report_test_support::crash_report_names_the_running_test"`）・生きているスレッドの行 9。積み上げは `4: ...::write_report`（`crash_report_test_support.rs:371`＝`Backtrace::force_capture`）・`5: ...::first_chance`（同 `:269`＝`write_report` の呼び出し）・`8: RtlRaiseException`・`9: RaiseException`・`10: ...::crash_report_names_the_running_test::closure$0`。
- **wintf の lib**（`target\load-flake\calib3-wintf-20261101-211904`・終了コード `0xC0000005`・0.5 秒）: `AREKA-CRASH` の行 47・`code=` の行 1・生きているスレッドの行 6。積み上げは `4: wintf::test_crash_report::write_report`（`test_crash_report.rs:264`＝`Backtrace::force_capture`）・`5: wintf::test_crash_report::first_chance`（同 `:187`）・`8: RtlRaiseException`・`9: RaiseException`・`10: ...::closure$0`。
- 2 回とも、行の途中に `AREKA-CRASH` が入った行は 0（`AREKA-CRASH` を含む行はどれも行の頭から始まる）、切りの印 ` <cut>` は 0（見込みどおり・どの行も 1024 バイトに届かない）。`AREKA-CRASH region` の行は 0（見込みどおり・較正の落ちは `RaiseException` の中で、`module=` は `C:\WINDOWS\System32\KERNELBASE.dll+0xC569A` と出るので、region の行の条件（モジュールが空）に当たらない）。region の行が初めて出るのは、モジュールに当たらない番地で落ちる Y2 の見込み。
- 積み上げの行の番号は今のソースの行と合う（`write_report` の `force_capture` の行と、`first_chance` の `write_report` の呼び出しの行）＝較正したのは最後の形のバイナリ。

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

**X1・X1b・Y1 の条件**（2026-11-01・走らせる前に書いた・コミット `c432692b`）

| 走行 | 範囲 | 負荷 | スレッドの数 | 回数 | 1 回の上限 |
|---|---|---|---|---|---|
| X1 | 上の 70 の道（`areka` の実行ファイル・989 本） | 論理 CPU の 2 倍＝44 本（論理 CPU 22・`load-flake.ps1` の `-Burners` の既定） | 既定（`--test-threads` を渡さない） | 報告つきの落ちが 1 回出たら止める・最大 20 | 10 分 |
| X1b | X1 と同じ（X1 が 20 回で 0 件のときだけ） | なし（`-Burners 0`） | 4（`-TestThreads 4`＝`--test-threads 4`） | X1 と同じ止め方・最大 20 | 10 分 |
| Y1 | `begin_draw_roundtrip`・`alpha_mask_resource`・`bitmap_source`（`wintf` の lib） | なし | 既定 | 報告つきの落ちが 1 回出たら止める・最大 20 | 5 分 |

- X1・X1b は `-SetEnv AREKA_TEST_PERMIT_HOLD=1` を渡し、回ごとの `summary.txt` に写る `AREKA-PERMIT-HOLD` の最後の 1 行（＝その回の許可の持ち時間の最長）を ⑻ の材料に採る。Y1 は `wintf` の lib なので渡さない（`wintf` に許可は無い）。
- 止め方の作り: `load-flake.ps1` には「落ちの報告が出たら止める」口が無い。`-Rounds 20` だと落ちた後も回り続けるので、`-Rounds 1` を 1 回ずつ呼び、その回の `summary.txt` の `rounds_with_crash_report:` が 1 以上なら止める（下の繰り返し）。呼ぶたびにビルドの確かめ（作り済みなら数秒）と CPU の採り（負荷の前と最中に 5 秒ずつ）と負荷の子の起こし直しが足されるが、落ちの後の回（X1 で最大 1 回あたり約 2.4 分）を回さずに済む方が安い。
- `-Filter` の値は上の「X1 の全体」の囲みの `-Filter emo2_boot::frame,...` の行をこの記録から読み、写しを作らない。
- third scope（`emo2_boot::spine::third_scope_talk_tests`）は X1 の範囲に入っているので、赤になればその回の `summary.txt` の赤の名前に出る（⑽ に写す）。

コマンド（どのディレクトリからでも動く形・PowerShell 7）:

```powershell
# X1（X1b は -Label を x1b-r に、-Burners 0 -TestThreads 4 を足す）
$wt = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-test-binary-stability-dec94e'
$f = ((Get-Content -Encoding utf8 "$wt\.kiro\specs\areka-test-binary-stability\load-repro.md") -match '^-Filter emo2_boot::frame,')[0] -replace '^-Filter ', ''
for ($n = 1; $n -le 20; $n++) {
    $label = 'x1-r{0:d2}' -f $n
    pwsh -NoProfile -File "$wt\tools\load-flake.ps1" -Label $label -Rounds 1 -RoundTimeoutMin 10 -SetEnv AREKA_TEST_PERMIT_HOLD=1 -Filter $f
    $d = Get-ChildItem "$wt\target\load-flake" -Directory -Filter "$label-*" | Sort-Object Name | Select-Object -Last 1
    if ((Get-Content -Encoding utf8 "$($d.FullName)\summary.txt") -match '^rounds_with_crash_report: [1-9]') { break }
}

# Y1
for ($n = 1; $n -le 20; $n++) {
    $label = 'y1-r{0:d2}' -f $n
    pwsh -NoProfile -File "$wt\tools\load-flake.ps1" -Label $label -Rounds 1 -RoundTimeoutMin 5 -Binary wintf-lib -Burners 0 -Filter begin_draw_roundtrip,alpha_mask_resource,bitmap_source
    $d = Get-ChildItem "$wt\target\load-flake" -Directory -Filter "$label-*" | Sort-Object Name | Select-Object -Last 1
    if ((Get-Content -Encoding utf8 "$($d.FullName)\summary.txt") -match '^rounds_with_crash_report: [1-9]') { break }
}
```

所要時間の見込み（走らせる前・系列ごとに「最大の回数 × 1 回の見込み」）:

| 系列 | 1 回の見込み | 見積もりの元 | 最大 |
|---|---|---|---|
| X1 | 約 3 分 | 前の記録の負荷の下の全体 1 回が約 10 分（5 回で約 50 分・design「測りの条件」）。4153 本のうち 989 本（約 0.24）で約 2.4 分、呼ぶたびの手間（ビルドの確かめ・CPU の採り 10 秒・負荷の子の起こし）約 0.5 分を足す | 20 × 3＝60 分 |
| X1b | 約 2 分 | 静かな机の `areka` の実行ファイルは 4153 本で 93.8 秒（⑷）＝989 本で約 22 秒。スレッドを 4 本に絞ると数倍に延びる見込みで約 1.5 分、手間を足す | 20 × 2＝40 分（X1 が 0 件のときだけ） |
| Y1 | 約 0.5 分 | 較正の `wintf` の lib は 1 本で 9.1 秒（⑸）。3 つの絞りと手間を足す | 20 × 0.5＝10 分 |

- 最悪（X1 が 20 回とも 0 件で X1b も 20 回・Y1 も 20 回）: 約 110 分。X1 で落ちが出れば X1b は回さないので最悪で約 70 分。前の率（負荷の下でおおむね 5 回に 1 回）なら X1 は 5 回前後・約 15 分の見込み。
- 1 回の上限（X1・X1b 10 分・Y1 5 分）に毎回当たる場合の上限は、X1 200 分・X1b 200 分・Y1 100 分（上の見込みの外・当たったらその回は「上限越え」と書かれる）。
- 開発者へ伝えた時刻: 2026-11-01 16:49（見込み約 15 分・最悪 70 分・X1b 込みで 110 分）。机は 16:50 に握った。X1 の 1 回目の後の言い直し: 2026-11-01 17:30 頃に進行役から開発者へ「1 回 8.5〜9.5 分・X1 の残り約 2 時間 20 分＋X1b 約 1 時間」（1 回目は 557 秒）。見込みの 1 回約 3 分は約 3 倍外れた。外れの元は、前の記録の「負荷の下の全体 1 回約 10 分」を本数の比で縮めたこと（X1 の範囲は COM と GPU の装置を開く重いテストに偏っていて、本数の比では縮まらない）。

**X1b の回数と上限の改め**（2026-11-01 20:05 頃・開発者の承認）

- X1b は最大 10 回（もとは 20 回）。訳: X1 が 20 回で報告つきの落ち 0 件。前の率（負荷の下でおおむね 5 回に 1 回）なら、10 回で 1 度も落ちない見込みは約 11%（0.8^10）。X1 の 20 回と合わせれば、X1b も 0 件なら件 1 を閉じるに足りる。
- X1・X1b の 1 回の上限は 20 分（もとは 10 分・`-RoundTimeoutMin 20` を系列の呼び出しで渡す。`load-flake.ps1` の既定 30 分はそのまま）。訳: X1 の 1 回は 7.5〜10 分で、10 分の上限に 20 回のうち 5 回当たった。
- `load-flake.ps1` の直し（4.1 の中）: 上限越えで回の木を止めた直後に `round-N.log` を読み、まだ書き手が握っていて読めずにスクリプトごと落ち、その回の結果が `summary.txt` に残らないことがあった（X1 で 3 回）。止める前に子孫を握って全部の終わりを待ち、ログが放されるのを待ってから（最大 60 秒）共有で読むようにした。放されなかったときは回の欄に `log_released: no` と書く。
- 伝えた見込み（進行役から開発者へ・2 回）: 20:00 頃「終わりは 20:45〜21:10 ごろ（X1b 1 回 1.5〜3 分の推定で 20 回 30〜60 分＋Y1 約 10 分）」。20:03 頃の推奨の中で「推奨どおりなら Y1 と X1b で最悪およそ 1.5 時間（21:45 ごろまで）」（20:05 頃に承認されたのはこちら）。1.5 時間の裏の見積もりは進行役の「Y1 約 10 分＋X1b 10 回 × 1 回約 8 分（X1 の 1 回の実測 7.5〜10 分に寄せた）」。走らせる順は Y1 → X1b。
- X1b のコマンド: 上の繰り返しで `-Label` を `x1b-r`、回数を 10、`-RoundTimeoutMin 20 -Burners 0 -TestThreads 4` にする。

**X1 の結果**（2026-11-01 16:50〜20:01（r20 の `summary.txt` の終わりは 20:00:57）・コミット `c432692b`・`target\load-flake\x1-r01-*`〜`x1-r20-*`・負荷 44 本・既定スレッド・上限 10 分）

| 回 | 終了コード | 秒 | 緑／赤 | 待ちの打ち切りの行 | `AREKA-CRASH` |
|---|---|---|---|---|---|
| r01 | 101 | 557 | 984／4 | 4 | 0 |
| r02 | 0 | 526.7 | 988／0 | 0 | 0 |
| r03 | 0 | 500.4 | 988／0 | 0 | 0 |
| r04 | 101 | 452.5 | 987／1 | 0 | 0 |
| r05 | 0 | 526.6 | 988／0 | 0 | 0 |
| r06 | 0 | 535.5 | 988／0 | 0 | 0 |
| r07 | 0 | 497.3 | 988／0 | 0 | 0 |
| r08 | 101 | 584.5 | 987／1 | 1 | 0 |
| r09 | 101 | 525.9 | 987／1 | 1 | 0 |
| r10 | 101 | 534.7 | 987／1 | 1 | 0 |
| r11 | 上限越え | 約 600 | （途中まで）966／14 | 0（途中で切れた） | 0 |
| r12 | 101 | 492.7 | 986／2 | 0 | 0 |
| r13 | 上限越え（killed） | 609.2 | 984／4 | 4 | 0 |
| r14 | 101 | 506.5 | 985／3 | 1 | 0 |
| r15 | 101 | 525.8 | 984／4 | 4 | 0 |
| r16 | 上限越え | 約 600 | （途中まで）958／12 | 0（途中で切れた） | 0 |
| r17 | 101 | 509.9 | 987／1 | 0 | 0 |
| r18 | 上限越え（killed） | 603.6 | 986／2 | 2 | 0 |
| r19 | 上限越え | 約 600 | （途中まで）904／1 | 0（途中で切れた） | 0 |
| r20 | 101 | 509 | 987／1 | 0 | 0 |

- 落ちの報告: 20 回とも 0 件（`rounds_with_crash_report: 0`。r11・r16・r19 は `round-1.log` を直に探して `AREKA-CRASH` 0 行）。
- r11・r16・r19 は上限越えの直後に、直す前の `load-flake.ps1` が `round-1.log` を読めずに `script failed` になり、`summary.txt` に回の欄が無い。表の値は `round-1.log` の `test ... ok`／`FAILED` の行を数えたもので、libtest が赤の文を最後にまとめて出す前に切れたので、赤の文は無い（上の「改め」の直しはこの回のため）。負荷の子は 3 回とも全部止まっていた。
- 赤の中身は ⑽（third scope）と「X1 のほかの赤」（下）。

**X1b の結果**（2026-11-01 20:09〜20:35・`target\load-flake\x1b-r01-*`〜`x1b-r10-*`・負荷なし・`--test-threads 4`・上限 20 分・直した `load-flake.ps1`・10 回ともコミット `bfa0b364`）

| 回 | r01 | r02 | r03 | r04 | r05 | r06 | r07 | r08 | r09 | r10 |
|---|---|---|---|---|---|---|---|---|---|---|
| 秒 | 135.9 | 150.1 | 150.9 | 128.8 | 125.7 | 125.2 | 138 | 209.2 | 169 | 141.3 |

- 10 回とも終了コード 0・988 緑／0 赤・待ちの打ち切り 0・上限越え 0・`AREKA-CRASH` 0・`log_released: no` 0。

**Y1 の結果**（2026-11-01 20:06〜20:09・`target\load-flake\y1-r01-*`〜`y1-r20-*`・`wintf` の lib の 52 本・負荷なし・既定スレッド・上限 5 分）

- コミット: r01〜r11 は `c432692b`、r12〜r20 は `bfa0b364`（系列の途中で道具の直しがコミットされた。どの回も作業ツリーには直した `load-flake.ps1` があり（`conditions.txt` の `dirty: yes`）、上限越えも無いので、結果への影響は無い）。

- 20 回のうち 4 回（r06・r07・r12・r20）でテストのプロセスが落ちた。終了コード `-1073741819`（`0xC0000005`）。cargo の行は「process didn't exit successfully: `...\wintf-4b394095060166ca.exe begin_draw_roundtrip alpha_mask_resource bitmap_source` (exit code: 0xc0000005, STATUS_ACCESS_VIOLATION)」。`test result:` の行は無い。
- 落ちた 4 回とも、`AREKA` を含む行が `round-1.log` に 1 行も無い（落ちの報告が出なかった）。止め方は「報告つきの落ち」なので止まらず、20 回を回し切った。⑸ の較正（`--ignored --exact` で較正のテスト 1 本）では `wintf` の lib の報告は出ていた。
- 落ちた回に終わっていなかったテストは、4 回とも `com::wuc::tests::begin_draw_roundtrip` を含む。ほかに `ecs::widget::bitmap_source::tests::test_bitmap_source_graphics_new` などの bitmap_source の組と `ecs::widget::bitmap_source::task_pool::tests::with_threads_one_builds_a_single_thread_pool`。終わっていたのは r06 23 本・r07 5 本・r12 35 本・r20 35 本（52 本のうち）。
- ほかの 16 回は緑（52 passed・1 回 0.1 秒前後）。

**Y1 の 2 回目**（2026-11-01 21:00:26〜21:00:44・`target\load-flake\y1b-20261101-210026`・最初の受け手を足した後・作業ツリーの直し（未コミット）・`wintf` の lib の 52 本・負荷なし・既定スレッド・`-Rounds 20 -RoundTimeoutMin 5`）

- 伝えた見込み: 較正 2 回と合わせて約 5 分（2026-11-01 20:58 頃・進行役から開発者へ）。
- 20 回のうち 6 回（r01・r06・r07・r11・r13・r14）で `0xC0000005` で落ち、6 回とも報告を出した（報告の無い落ちは 0）。ほかの 14 回は緑（52 passed）。
- 落ちたスレッドは 6 回とも `com::wuc::tests::begin_draw_roundtrip`（`AREKA-CRASH-THREAD` の一覧の名前で読んだ）。生きているスレッドは 6 本で、名前のあるのは `main` とこのテストだけ。
- `module=` は 6 回とも空（例: `AREKA-CRASH module=+0x7FFEFE823878`）＝落ちた番地はどの読み込まれたモジュールにも当たらない。番地は `0x7ffefe823878`（r01・r14）・`0x7ffefe863028`（r06）・`0x7ffefe828cc5`（r07・r13）・`0x7ffefe8087f5`（r11）で、`0x7ffefe80xxxx`〜`0x7ffefe86xxxx` の中。
- 積み上げは `write_report`・`first_chance`・`RtlWow64GetCurrentMachine` 2 つ・`KiUserExceptionDispatcher` で止まり、落ちた番地より先へ辿れない（巻き戻しの表の無い番地）。前の Y1 で最後の受け手まで届かなかった訳と合う見立て（確かめてはいない）。
- 報告の行が libtest の出力と混ざって切れていた（例: `thread="com::wuc::tests::begin_draw_roundtritest ecs::layout::... p"`）。これを受けて、行を 1 度の書き込みで出す形と、モジュールが空のときの `AREKA-CRASH region` の行を足した（design「CrashReport」・Y2 で読む）。

**件 1 と件 3 の扱い**

- 件 1（`areka` の実行ファイルの落ち）: 再現せずとして閉じる（要件 1.9・2.4）。条件と結果: X1（上の 70 の道・負荷 44 本・既定スレッド）20 回で報告つきの落ち 0 件、X1b（同じ範囲・負荷なし・4 スレッド）10 回で 0 件。前の率（5 回に 1 回）なら、30 回で 1 度も落ちない見込みは約 0.1%（0.8^30）。落ちの報告はそのまま残り、以後に落ちれば持ち手と生きているテストを名指す。
- 件 3（`wintf` の lib の落ち）: 再現した（20 回で 4 回）。ただし落ちの報告が出なかったので、まだ開いている。次の手は、4.2 の Y2 の前に「この絞りで `wintf` の lib の落ちの報告が出ない訳」を調べること。
  - 2026-11-01 追記: 最初の受け手を足した Y1 の 2 回目（上）で、20 回のうち 6 回が落ち、6 回とも報告を出した。落ちたスレッドは毎回 `com::wuc::tests::begin_draw_roundtrip`、落ちた番地はモジュールに当たらない。まだ開いている（Y2 で領域の行を読む）。

**X2 を回さない訳**（2026-11-01・4.2）

- X2（許可の数 1）は件 1 の対照で、件 1 は X1 20 回＋X1b 10 回で落ち 0 として閉じた（上）。絞る元の持ち手の行も生きているテストの一覧も無いので回さない。件 1 の X3 も同じ訳で回さない。

**Y2 の条件**（走らせる前・2026-11-01・4.2）

- 対照の訳: 件 3 の落ちた番地はどの読み込まれたモジュールにも当たらず、積み上げは `KiUserExceptionDispatcher` で止まる（巻き戻しの表が無い）＝「放されたモジュールのコードを呼んだ」見立て。原因の表の候補 B（一時的な MTA の解体の後の解放）か D に当たる。B ならプロセスの寿命の MTA を立てると落ちが消える。
- 条件: Y1 と同じ（`wintf` の lib・`-Filter begin_draw_roundtrip,alpha_mask_resource,bitmap_source`・負荷なし・既定スレッド・1 回の上限 5 分）に `-SetEnv AREKA_TEST_MTA_KEEPER=1` を足す。`crates/wintf/src/test_crash_report.rs` の `install`（読み込みの初め・libtest の main より前）が `CoIncrementMTAUsage` を呼ぶ。
- 止め方: 落ちたら 1 回目で止める（落ちが残る＝B ではない）。落ちなければ 20 回（落ちが消える）。止め方の作りは X1・Y1 と同じ（`-Rounds 1` を 1 回ずつ呼び、`summary.txt` の `rounds_with_crash_report:` が 1 以上なら止める）。
- 口が働いた印: `install` は `CoIncrementMTAUsage` が失敗したときだけ `AREKA-MTA-KEEPER failed hr=...` を標準エラーへ書く（成功では何も書かない）。各回の `round-1.log` にこの行が無いことと、`conditions.txt` の `set_env: AREKA_TEST_MTA_KEEPER=1` を印にする。
- 所要時間の見込み: 1 回約 0.5 分（Y1 の 20 回が 20:06〜20:09・y1b の 20 回が 18 秒だったので実際はもっと短い見込み）× 最大 20 回＝最大約 10 分。

コマンド（どのディレクトリからでも動く形・PowerShell 7・上の Y1 の繰り返しの `-Label` を `y2-r` にし、`-SetEnv AREKA_TEST_MTA_KEEPER=1` を足したもの）:

```powershell
$wt = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-test-binary-stability-dec94e'
for ($n = 1; $n -le 20; $n++) {
    $label = 'y2-r{0:d2}' -f $n
    pwsh -NoProfile -File "$wt\tools\load-flake.ps1" -Label $label -Rounds 1 -RoundTimeoutMin 5 -Binary wintf-lib -Burners 0 -SetEnv AREKA_TEST_MTA_KEEPER=1 -Filter begin_draw_roundtrip,alpha_mask_resource,bitmap_source
    $d = Get-ChildItem "$wt\target\load-flake" -Directory -Filter "$label-*" | Sort-Object Name | Select-Object -Last 1
    if ((Get-Content -Encoding utf8 "$($d.FullName)\summary.txt") -match '^rounds_with_crash_report: [1-9]') { break }
}
```

- 走らせる前のコードの読み（見立て・まだ決めていない・⑺ の下ごしらえ）:
  - 落ちるスレッド `com::wuc::tests::begin_draw_roundtrip`（`crates/wintf/src/com/wuc.rs`）は COM を自分で初期化せず、`create_dispatcher_queue_controller(DQTAT_COM_ASTA)`（`DQTYPE_THREAD_CURRENT`）で自分のスレッドを ASTA にし、`Compositor::new`・`GraphicsCore::new`（`crates/wintf/src/ecs/graphics/core.rs`・D3D11／D2D／DWrite）・`ICompositorInterop::create_graphics_device`・`CreateDrawingSurface`・`DrawingSurfaceInteropExt::begin_draw`／`end_draw` を同じスレッドで作って使い、関数の終わりで宣言の逆の順に捨てる（`_dq` の `DispatcherQueueController` が最後）。作るのも捨てるのも同じスレッドなので、テストの中には候補 D の形（別のスレッドで捨てる）は無い。
  - 並ぶ 2 つの絞りは一時的な MTA を立てて壊す: `alpha_mask_resource`（`crates/wintf/src/ecs/layout/hit_test/tests_ex/alpha_mask_resource.rs` の `with_com_initialized`＝`CoInitializeEx(COINIT_MULTITHREADED)`→閉包→`CoUninitialize`・閉包の中で `com::wic::wic_factory`＝`CoCreateInstance(CLSID_WICImagingFactory)`）と `bitmap_source`（`crates/wintf/src/ecs/widget/bitmap_source/tests.rs` の `with_com_initialized`・同じ形）。
  - `bitmap_source` の `WicCore::new`（`wic_core.rs`）は `ensure_process_mta`＝`CoIncrementMTAUsage` でプロセスの寿命の MTA を立てる（前例 ② の直し・DD-9）。これより前は、MTA は並ぶテストの一時的なものだけで、最後の `CoUninitialize` のたびに MTA ごと解体されうる。`com::wic::wic_factory` はこの守りを通らない。
  - よって B の経路の見立ては「`with_com_initialized` の `CoUninitialize` で一時的な MTA が解体され、COM がそこで読み込んだ DLL を放す → `begin_draw_roundtrip` のスレッドがその DLL のコードを呼ぶ（番地がどのモジュールにも当たらない）」。Y2 で落ちが消えれば B、残れば領域の行（`AREKA-CRASH region`）と生きているスレッドで D・C を読む。

**Y2 の結果**（2026-11-01 21:27:14〜21:29:52 ごろ・`target\load-flake\y2-r01-20261101-212714`〜`y2-r20-20261101-212952`・コミット `0b4200ce`（`dirty: yes`＝この記録の書き足しだけ）・`wintf` の lib の 52 本・負荷なし・既定スレッド・上限 5 分・`AREKA_TEST_MTA_KEEPER=1`）

- 伝えた見込み: 約 10 分（2026-11-01 21:26・進行役から開発者へ）。机は 21:26 ごろに握った。実際は約 3 分。
- 20 回とも、終了コード 0・52 passed／0 failed・テスト 0.10〜0.16 秒・待ちの打ち切り 0・上限越え 0・`rounds_with_crash_report: 0`・`AREKA-CRASH` の行 0（`region` の行も 0）。落ちなかったので、20 回を回し切った。
- 口が働いた印: 20 回とも `conditions.txt` に `set_env: AREKA_TEST_MTA_KEEPER=1` があり、`round-1.log` に `AREKA-MTA-KEEPER failed` の行は無い。
- 対照: MTA を立てない Y1 は 4/20、y1b は 6/20 落ちた。立てた Y2 は 0/20（落ちが消えた）。

**X1 のほかの赤**（third scope は ⑽・記録だけ・直しは決めない・後の 6.x・7.2 の材料）

文の頭で分けた。`A が定常に着く` や `assertion ... failed` の後追いの文は、同じテストの先の待ちの打ち切りの続き。

| 文の頭 | 回とテスト |
|---|---|
| 「既定のバルーンを根へ移す: Os { code: 5, kind: PermissionDenied, ... }」（件 4 の形） | r04 `install::desk::overwrite_tests::a_failed_commit_boots_the_same_ghost_with_its_old_contents_and_reports_failure`・r12 `emo2_boot::frame::ghost_quit_switch_tests::stop_with_handoff_under_reservation_switches_without_exit`・r14 `ghost_session::shell_balloon_switch_session_update_tests::a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon`・r14 `install::desk::record_tests::a_balloon_not_hidden_by_the_running_ghost_is_not_recorded`・r17 `emo2_boot::frame::stay_on_top_lifecycle_tests::a_shiori_reload_keeps_the_stay_on_top_state_and_the_script_group`・r20 `ghost_session::switch_tests::switch_to_self_takes_down_and_reboots_a` |
| 「待ちの打ち切り［止まった］: 「crates\areka\src\install\desk_overwrite_tests.rs:237」— 相手が 30.0 秒のあいだ状態を進めなかった（…進み 0 回…）」 | r01 `install::desk::overwrite_tests::a_switch_to_another_ghost_does_not_install_and_returns_the_archive`・同 `a_running_ghost_missing_from_the_catalog_is_installed_elsewhere_and_closed_once`・r15 `...::a_failed_commit_boots_the_same_ghost_with_its_old_contents_and_reports_failure` |
| 「待ちの打ち切り［止まった］: 「crates\areka\src\shell_balloon_switch_session_balloon_tests.rs:246」」（進み 0 回・2 回） | r01・r15 `ghost_session::shell_balloon_switch_session_balloon_tests::script_balloon_switch_round_trips_and_binds_the_next_talk_to_the_new_slot` |
| 「待ちの打ち切り［止まった］: 「crates\areka\src\shell_balloon_switch_session_abort_tests.rs:232」」（進み 1 回） | r01 `ghost_session::shell_balloon_switch_session_abort_tests::undecodable_shell_keeps_the_old_mount_and_records_one_error` |
| 「待ちの打ち切り［止まった］: 「crates\areka\src\ghost_session_switch_tests.rs:252」」（待ち始めから 61.7〜91.4 秒・進み 4〜5 回） | r13 `ghost_session::switch_tests::switch_to_b_without_boot_record_sends_first_boot_not_ghost_changed`・`...::fallback_tests::target_connect_fail_boots_default_with_halt_and_no_alert`・`...::script_change_tag_switches_a_to_b_and_reaches_steady` |
| 「待ちの打ち切り［止まった］: 「置き場のゴーストを降ろす」」（進み 2 回） | r13 `install::desk::overwrite_tests::a_switch_to_another_ghost_does_not_install_and_returns_the_archive` |
| 「待ちの打ち切り［止まった］: 「crates\areka\src\mcp\check_script_tests.rs:244」」（進み 0 回） | r15 `mcp::check_script::check_script_tests::real_unit_answers_two_scripts_without_running_them` |
| 「待ちの打ち切り［止まった］: 「放した符号化の仕事が走り終える」」（進み 0 回） | r15 `mcp::dump_surface::dump_surface_tests::abandoned_while_encoding_drops_the_pair_without_ui_errors` |
| 「待ちの打ち切り［止まった］: 「crates\areka\src\emo2_boot\frame_stay_on_top_lifecycle_tests.rs:157」」・「…:42」 | r18 `emo2_boot::frame::stay_on_top_lifecycle_tests::a_ghost_switch_starts_the_target_off_and_logs_the_switch_discard`・`...::a_ghost_reload_starts_off_with_no_script_group_and_logs_the_reload_discard` |
| 「送り手が送り続けるあいだ drain が戻らない（1 フレームが終わらない）: Timeout」（`recv_timeout` の自前の上限・⑵ の「その外の文言の無い待ち」の `frame/drain_resnap_tests.rs`） | r12 `emo2_boot::frame::drain_resnap::tests::drain_returns_while_the_sender_keeps_sending` |
| 上限越えで文が切れた（`FAILED` の行だけ） | r11 の 14 本（`emo2_boot::ghost_switch::balloon_tests` の 6 本・`boot_event_tests` の 2 本・`ghost_session::shell_balloon_switch_session_lap_tests` の 2 本・`ghost_session::switch_tests` の 2 本・`shell_balloon_switch_session_balloon_tests` の 1 本・third scope）・r16 の 12 本（`emo2_boot::ghost_switch::balloon_tests` の 4 本・`boot_event_tests` の 1 本・`reload_tests` の 2 本・`install::desk::overwrite_tests` の 3 本・`shell_balloon_switch_session_abort_tests::marked_talk_break_cancels_the_shell_switch_without_quitting`・`ghost_session::switch_translate_tests::switch_sends_each_on_translate_to_its_own_ghost`） |

- X1b（負荷なし・4 スレッド）の 10 回では、上のどれも赤にならなかった。

## ⑺ 原因の経路と直し

**件 3（`wintf` の lib の落ち）の原因: 候補 B（一時的な MTA の解体の後の解放）**（2026-11-01・4.2）

- 落ちる組: `com::wuc::tests::begin_draw_roundtrip` と、`with_com_initialized` の中で走るテスト（`alpha_mask_resource` と `bitmap_source` の絞り）が同じプロセスで並んで走る組。`begin_draw_roundtrip` は自分のスレッドを、`create_dispatcher_queue_controller(DQTAT_COM_ASTA)` の `DispatcherQueueController` で ASTA にしている。
- 落ちた場所: `com::wuc::tests::begin_draw_roundtrip` のスレッド（y1b の 6 回とも）。`0xC0000005` で落ち、番地はどの読み込まれたモジュールにも当たらない（`module=` が空）。番地は `0x7ffefe823878`・`0x7ffefe863028`・`0x7ffefe828cc5`・`0x7ffefe8087f5` で、`0x7ffefe80xxxx`〜`0x7ffefe86xxxx` の中。積み上げは `KiUserExceptionDispatcher` で止まる（巻き戻しの表が無い）。
- 原因の経路（コードの定義の名前で）:
  1. `ecs::layout::hit_test::tests_ex::alpha_mask_resource::with_com_initialized` と `ecs::widget::bitmap_source::tests::with_com_initialized` は、`CoInitializeEx(COINIT_MULTITHREADED)` → 閉包 → `CoUninitialize` で一時的な MTA を立てて壊す。`alpha_mask_resource` の側の閉包は `com::wic::wic_factory`（`CoCreateInstance(CLSID_WICImagingFactory)`）を呼び、`ensure_process_mta` を通らない。`ecs::widget::bitmap_source::tests` の側の閉包は `WicCore::new` → `ensure_process_mta`（`CoIncrementMTAUsage`）を通り、そこから先は MTA が立ったまま残る。よって落ちうる窓は、プロセスの初めから最初の `WicCore::new` までの間。
  2. プロセスの寿命の MTA がまだ立っていない間は、最後の `CoUninitialize` のたびに MTA ごと解体され、COM がその MTA で読み込んだ DLL を放す。
  3. 同じ時に `com::wuc::tests::begin_draw_roundtrip` のスレッドは、`Compositor::new`・`GraphicsCore::new`・`ICompositorInterop::create_graphics_device`・`CreateDrawingSurface`・`DrawingSurfaceInteropExt::begin_draw`／`end_draw`、関数の終わりの捨てる段を通る。このどこかで放された DLL のコードを呼び、どのモジュールにも当たらない番地で AV になる。
  - Y の絞り（52 本: `com::wuc::tests` 1・`ecs::layout::hit_test::tests_ex::alpha_mask_resource` 8・`...::tests_ex::alpha_mask` 1・`ecs::widget::bitmap_source::tests` 16・`...::alpha_mask::tests` 15・`...::alpha_mask::regenerate_tests` 10・`...::task_pool::tests` 1・`wintf` の lib の `--list` で数えた）で `Compositor::new`・`GraphicsCore::new` を呼ぶのは `com::wuc::tests::begin_draw_roundtrip` だけ（`ecs/layout/hit_test/tests_ex/` と `ecs/widget/bitmap_source/` に grep で 0 件）。`Compositor` は 1 つしか作られないので、候補 A（別々の MTA のスレッドに `Compositor` を並べる）の形は無い。
  - 作るのも捨てるのも同じスレッドなので、候補 D（作ったスレッドと別のスレッドで捨てる）の形はテストの中に無い。落ちた番地がドライバのモジュール（`igd*`・`d3d11*`・`dxgi*`）に当たらないので、候補 C の印も無い。
- 決めた対照: Y1 4/20・y1b 6/20（MTA を立てない）に対し、Y2 0/20（`AREKA_TEST_MTA_KEEPER=1`＝`test_crash_report.rs` の `install` がプロセスの初めに `CoIncrementMTAUsage`）。原因の表の B の印「X3・Y2 で落ちが消える」に当たる。前例 ②（`wic_core.rs` の `ensure_process_mta` の説明・DD-9）と同じ形。
- 確かめていないこと:
  - 放された DLL がどれか（Y2 では落ちず、`AREKA-CRASH region` の行が出なかった）。
  - 経路の 3 の、どの呼び出しで放されたコードに入るか（積み上げが落ちた番地より先へ辿れない）。
  - `DQTAT_COM_ASTA` の ASTA が MTA の解体に巻き込まれる仕組みの細部。
  - MTA を立てない側を、Y2 と同じコミット `0b4200ce` では回し直していない。y1b は `7bfc2b22` の作業ツリー（未コミットの直しあり）、Y1 は `c432692b`・`bfa0b364`。間の違いは落ちの報告の側（最初の受け手・1 度の書き込み・region の行）だけ。最初の受け手が無かった Y1 でも 4/20 落ちたので、結論は変わらない。
- 直し（5.1・当てた）: 候補 B の直しとして、テストのプロセスの初めに無条件でプロセスの寿命の MTA を立てる。`wintf` の `test_crash_report.rs` の `install`（`.CRT$XCU` の初期化子）から測りの間だけの口（`AREKA_TEST_MTA_KEEPER`）を外し、無条件に `CoIncrementMTAUsage` を呼ぶ（クッキーは返さない・失敗した時だけ `AREKA-MTA-KEEPER failed hr=...` を 1 行書く）。design のとおり、写しの相手の `areka` の `emo2_boot/crash_report_test_support.rs` の `install` にも同じ直しを当てた。
  - 檻: 2 つのファイルの `initializer_keeps_a_process_lifetime_mta`（COM を初期化していない新しいスレッドで `CoGetApartmentType` が `APTTYPE_MTA`・`APTTYPEQUALIFIER_IMPLICIT_MTA` を返す）。直す前は単独の走行で `CO_E_NOTINITIALIZED`（0x800401F0）の赤、直した後は緑。
  - 読み直し（design の Revalidation Triggers）: 振る舞いの変わるテストは無い。`com::wuc::tests::begin_draw_roundtrip` の「そのスレッドは COM 未初期化」の説明だけを「自分で初期化していない・暗黙の MTA に居る」に直した（ASTA の `DispatcherQueueController` はこれまでどおり作れる）。`cargo test -p wintf --lib` 1249 passed・`cargo test -p areka --bin areka` 4160 passed で、`AREKA-` の行は 0。
- 直しの後の確かめ（2026-11-01 21:44:04〜21:44:24・`target\load-flake\fix-y1-20261101-214404`・Y1 と同じ条件＝`wintf` の lib・`-Filter begin_draw_roundtrip,alpha_mask_resource,bitmap_source`・負荷なし・既定スレッド・`-Rounds 20 -RoundTimeoutMin 5`・測りの環境変数は渡さない（`set_env: (none)`））: **20 回のうち落ち 0 件**（`rounds_run: 20 / 20`・`rounds_red: 0`・`rounds_timed_out: 0`・`rounds_with_crash_report: 0`・`passed_total: 1040  failed_total: 0`・回ごとの `crash_lines (AREKA-CRASH): 0`）。直す前の Y1 4/20・y1b 6/20 に対し、Y2（口で MTA を立てた）0/20 と同じく落ちが消えた。
- 所要時間（要件 9.9・9.11）: 直しはプロセスの初めの呼び出し 1 回だけなので、時間は増えない見込み。上の 20 回の 1 回の壁時計は中央値 0.6 秒・最長 0.9 秒。静かな机の全体テストの所要時間と 1.20 倍の線は 7.2 で測る（⑾）。

**件 1（`areka` の実行ファイルの落ち）**

- 4.1 で再現せずとして閉じた（X1 20 回＋X1b 10 回で 0 件・⑹）。原因の候補は決めていない（分からない）。件 1 だけのための直しは要らない。件 3 の直し（B: テストのプロセスの初めにプロセスの寿命の MTA を立てる）は、design のとおり `areka` の写し（`emo2_boot/crash_report_test_support.rs`）にも当てた（5.1）。

## ⑻ 許可の待ちの上限の値と決めた訳

**4.1 で採った許可の持ち時間の最長**（回ごとの `AREKA-PERMIT-HOLD` の最後の行・5.2 で値を決める材料・値はまだ決めていない）

X1（負荷 44 本・既定スレッド）: 55〜249 秒。

| 回 | ms | 持ち手 |
|---|---|---|
| r01 | 90406 | `emo2_boot::frame::visibility_integration_tests::shell_box_integration_tests::align_tests::window_and_box_swap_with_the_picture_when_the_script_surface_arrives_first` |
| r02 | 118555 | `emo2_boot::frame::visibility_integration_tests::new_ghost_holdings_start_with_a_fresh_ledger_after_a_switch`（以下「new_ghost_holdings」） |
| r03 | 96842 | new_ghost_holdings |
| r04 | 83081 | new_ghost_holdings |
| r05 | 116017 | new_ghost_holdings |
| r06 | 62267 | `...::align_tests::window_and_box_swap_with_the_picture_when_the_picture_arrives_first` |
| r07 | 81294 | `emo2_boot::spine::conformance_lap_tests::conformance_lap_walks_every_stage_to_its_completion`（以下「conformance_lap」） |
| r08 | 117472 | conformance_lap |
| r09 | 86111 | new_ghost_holdings |
| r10 | 105890 | conformance_lap |
| r11 | 249246 | new_ghost_holdings（上限越えの回・`round-1.log` から） |
| r12 | 99717 | new_ghost_holdings |
| r13 | 137177 | conformance_lap |
| r14 | 55230 | new_ghost_holdings |
| r15 | 104113 | `ghost_session::shell_balloon_switch_session_update_tests::a_reload_after_the_swaps_boots_with_the_remembered_shell_and_balloon` |
| r16 | 143516 | `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind`（上限越えの回・`round-1.log` から） |
| r17 | 215243 | new_ghost_holdings |
| r18 | 197500 | new_ghost_holdings |
| r19 | 116784 | new_ghost_holdings（上限越えの回・`round-1.log` から） |
| r20 | 143379 | conformance_lap |

X1b（負荷なし・4 スレッド）: 2.7〜8.3 秒。

| 回 | ms | 持ち手 |
|---|---|---|
| r01 | 3219 | `ghost_session::shell_balloon_switch_session_abort_tests::ghost_switch_after_a_shell_swap_unloads_the_session` |
| r02 | 3490 | `...::align_tests::window_and_box_swap_with_the_picture_when_the_script_surface_arrives_first` |
| r03 | 3280 | `...::ghost_switch_after_a_shell_swap_unloads_the_session` |
| r04 | 2776 | `...::align_tests::window_and_box_swap_with_the_picture_when_the_picture_arrives_first` |
| r05 | 2720 | `...::ghost_switch_after_a_shell_swap_unloads_the_session` |
| r06 | 2864 | `...::align_tests::box_only_after_hiding_the_window_reports_without_a_warning` |
| r07 | 2830 | `...::align_tests::window_and_box_swap_with_the_picture_when_the_picture_arrives_first` |
| r08 | 8267 | `mcp::dump_balloon::dump_balloon_gpu_tests::scope_two_is_no_such_scope_and_calls_do_not_disturb_the_ghost` |
| r09 | 4650 | `...::align_tests::window_and_box_swap_with_the_picture_when_the_script_surface_arrives_first` |
| r10 | 2974 | `...::ghost_switch_after_a_shell_swap_unloads_the_session` |

**5.1 の確かめの走行の許可の持ち時間の最長**: 無し（`fix-y1` は `wintf` の lib の走行で、`wintf` に許可は無い・回ごとの `permit_hold_max: (none)`）。5.2 の上限の材料は 4.1 の X1・X1b の値（上の表）を使う。

**決めた値（5.2）**: `GPU_PERMIT_IDLE` ＝ **500 秒**（`spine_wait.rs`）。

- 訳: 件 1 の直しは当てていない（⑺）ので、決め方（設計の議題 4）の材料は X1・X2 の値。X2 は回していない（件 1 が 4.1 で閉じた）ので X1 の最長 249.2 秒（r11）。その 2 倍 498.5 秒を丸めて 500 秒。30 秒は下回らない。
- `idle` の意味: 許可の持ち時間ではなく、**待っている側から見て許可が 1 つも返らなかった時間**。持たれている 4 つは待ち始めより前に取られているので、返りの無い時間は 4 つのうち最初に返るものの残りの持ち時間で、持ち時間の最長を越えない。2 倍はその上の余裕。
- 総時間の上限との食い違い（5.2 で見つけた）: 設計は許可の待ちも待ち始めから `WAIT_CAP`（300 秒）で打ち切る形だった。500 秒は 300 秒より長いので、そのままでは `［止まった］` より先に必ず総時間の上限に届き、負荷の下で列を待つ GPU のテストが返りの続くまま 300 秒を越えて `［進んではいた］` の赤になるおそれがあった。案は 1（許可の待ちだけの総時間の上限）・2（総時間の上限を置かない）・3（今のまま）。
- **裁定（2026-11-02・開発者「案 2 で」）**: 許可の待ちには総時間の上限を効かせず、`GPU_PERMIT_IDLE`＝500 秒のあいだ許可が 1 つも返らないときの `［止まった］` だけで打ち切る。理由: 返りが目印で、GPU のテストは有限なので返り続ける限り待ちはいつか終わる。要件 6.1 の上限は 500 秒の線で満たす。許可の待ち時間そのものは測っていない（測ったのは持ち時間だけ）。決め手 `judge_permit_wait` から総時間の判定と `PermitVerdict::Capped` を外した。ほかの待ちの `WAIT_CAP` は変えていない。

## ⑼ 件 4 の見立てと根拠

**観測**

- 起票の観測（2026-10-31・brief「件 4」）: `ghost_switch_test_support.rs` の `SwitchRig::new` で、既定のバルーンを根へ移す `rename` は通り、その直後の空になった `ghost\emo2\balloon\` の `remove_dir` だけが io のエラーで 1 回赤（`cargo test -p areka`・負荷なし）。単独の回し直しで緑。
- 4.1 の X1（負荷 44 本・20 回・⑹「X1 のほかの赤」）: 同じ足場の 1 つ前の操作、`rename` の `expect("既定のバルーンを根へ移す")` が `Os { code: 5, kind: PermissionDenied, ... }`（`ERROR_ACCESS_DENIED`）で 6 回赤（r04・r12・r14 の 2 本・r17・r20・6 本とも別のテスト）。`remove_dir` の赤は 0 回。負荷なし・4 スレッドの X1b 10 回では 0 回。
- 検体の根は 1 本のテストに 1 つ（`SampleRoot::acquire` が毎回新しい根に写す）なので、同じプロセスの別のテストがこの木を開くことは無い。テストの側は `rename` の前にこの木のどのファイルも開いたままにしていない（`acquire` の写しは閉じ終えている）。

**見立て**（確かめてはいない・掴み手の一覧は採っていない）

- 掴んでいたのはこのテストのプロセスの外の者で、写したばかりのファイルとフォルダを走査する検索のインデクサーかウイルス対策の実時間の走査、と読む。どちらも新しく書かれたファイルを書き込みの直後に開く。
- フォルダの `rename` と `remove_dir` は、フォルダかその中の要素を共有の削除（`FILE_SHARE_DELETE`）なしで開いている者が居ると `ERROR_ACCESS_DENIED`（5）で拒まれる。起票の回は子のファイルの走査が終わった後に親のフォルダだけが開かれていた形、X1 は移す時点でまだ中が開かれていた形で、同じ掴みの違う時点に当たったと読む。
- 負荷の下でだけ数が増える（X1 は 20 回で 6 回・X1b は 0 回）のは、走査する側も CPU を待つので、掴みの時間が延びて写しの直後の操作と重なりやすくなるため、と読む。
- 根拠: 拒みの番号が 5 で（ファイルが無い・使用中の 32 ではない）、拒まれたのはどれも「写した直後の木」への名前の変更と削除だけで、同じ木を読んで写す `copy_tree` の赤は 0 回、プロセスの中に掴み手が居ない（上の 3 つ目）。

**直し**（6.6・要件 8.1・10.1）

- 設計は `remove_dir` の 1 行を消す形だったが、X1 で `rename` も同じ掴みで拒まれたので、写した直後の木の名前を変える操作も消した。既定のバルーンは根の `balloon\emo2-kakukaku\` へ `copy_tree` で写し、A／B の複製は `copy_tree` の `skip` で `ghost\emo2\balloon\` を写さない（A／B にバルーンを持ち込まないのは今までと同じ）。足場に残る操作は、新しいフォルダを作ってファイルを写すだけになる。sleep も試し直しも足していない。
- 元の `ghost\emo2\balloon\emo2-kakukaku\` は根に残る。ゴーストの中のバルーンを読むのは今のゴーストの分だけ（`catalog_roots.rs` の `balloons_for`）で、`SwitchRig` のテストは `emo2` を今のゴーストにしない（47 ファイルを `"emo2"` で grep して 0 件）ので、誰も読まない。
- 空のフォルダが無いことを確かめるテストは無い（`SwitchRig` を使う 47 ファイルを `join("balloon")` で grep・在るのは A の中にバルーンを足す足場とテストだけ）ので、読み替えは要らなかった。
- 確かめ（2026-11-02・静かな机ではない普通の 1 回）: `SwitchRig` を使うモジュールの族（`ghost_switch session_mark ghost_session install:: mcp:: readonly session_end update:: boot_resolve boot_shell reload_skin stay_on_top ghost_quit write_failure third_scope`）861 本で 860 緑・1 無視・0 赤（70.8 秒）。
- 完了の状態の走行（2026-11-02 07:49〜・排他テストの机と単独の机を握って 1 度だけ・コミット `6cc27fd8`（`dirty: yes`＝この 6.6 の直し）・負荷なし・既定のスレッド・`-Rounds 10 -RoundTimeoutMin 10 -Filter ghost_switch,session_mark`・95 本）: 10 回とも緑。毎回 95 passed・0 failed・終了コード 0・1 回 15.3〜19.7 秒・中央値 18.45 秒・待ちの打ち切りの行 0・後片付けの失敗の行 0・`AREKA-CRASH` 0。記録は `target\load-flake\t66-20261102-074953\summary.txt`。

**件 5**（6.6 の確かめ）

- ⑶ のとおり、10 回の走行で赤の文は 1 度も出ず、要件 8.7 に沿って条件と結果を書いて閉じている。原因は分かっていないので、要件 8.4 の直し（捕まえの範囲・印のファイルの置き場を分ける）は当てていない。`main_session_mark_tests.rs` にも手を入れていない。最後の負荷 5 回（7.2）でもう一度見る。

## ⑽ third scope の読み分け

**4.1 の X1 で見たもの**（記録だけ・直しは決めない）

- 赤になった回: `emo2_boot::spine::third_scope_talk_tests::spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits` が X1 の 20 回のうち 6 回（r08・r09・r10・r11・r14・r19）。X1b の 10 回では 0 回。
- 文（文が残った r08・r09・r10・r14）: 「待ちの打ち切り［止まった］: 「起動の会話」— 相手が 30.0 秒のあいだ状態を進めなかった（待ち始めから 35.4 秒・それまでの進み 10 回・最後の 1 回の確かめに 0.0 秒）／起動の会話: 会話が止まった（再生の終わりまで進まない）」。4 回とも、止まったのは「起動の会話」、進みは 10 回、待ち始めからは 33.6〜39.1 秒。r11・r19 は上限越えで文が切れ、`FAILED` の行だけ残った。
- 足場: `SpineHarness`（`emo2_boot/spine.rs` の `make_world_with_gpu`、445 行で `GpuPermit::take`）＝COM と GPU の装置を開く足場。待ちは `spine_third_scope_talk_tests.rs` の `drive_talk`（`wait_until` の芯・`Progress::Count`）。進みの目印は SHIORI の呼び出しの数と、受けた面の指令・知らせの数の和。
- 読み分けの候補: (a) 重い負荷の下での止まり。進みが 10 回まで来た後、30 秒のあいだ呼び出しも指令も知らせも増えなかった。負荷なしの X1b では一度も赤にならない。(b) 足場の側の取り違え。会話は進んでいるのに、目印に数えるもの（受けた指令・知らせ）を取り落としているか、`done` の条件が満たされない形。今の文は件数だけで、どの指令まで届いていたか（`seen`）は出ないので、この記録だけでは (a) と (b) を分けられない。

**6.6 の読み分け**（要件 8.5・8.6）

- 読み: (b) の足場の側の止まり。注入の時刻が観測を追い越していた。`drive_talk` は 1 反復ごとに注入の時刻を 100 ms 進め、会話の時刻の上限（`TALK_SPAN_CAP_MS`＝10 秒）で頭打ちにする。反復は 200 µs の眠りだけなので、約 100 反復で頭打ちに届く。dispatcher は会話の起点（`base_now`）を、その会話が始まった後の最初の Tick の時刻に付ける（`crates/areka-ghost/src/dispatcher.rs` の `on_tick` の `get_or_insert`）。負荷の下で `OnBoot` の応答が着いて会話が始まるまでに 100 反復を越えると、起点が頭打ちの時刻に付き、その後の Tick はずっと同じ時刻なので経過は 0 秒のまま、0 秒の合図（会話の始まり・最初の面）より後が出ない。進みが 10 回まで来て止まる形と合う。
- 根拠: 同じ形の止まりを `spine_conformance_script.rs` の段の区間の注（「⑴ が予算を食い尽くして再生が凍った」）と `StageSink::may_advance_clock` が既に記していて、そちらは着地までは時刻を据え置く形で直してある。手で確かめた: 1 反復の進みを一時的に 10,000 ms にして頭打ちを最初の反復に寄せると、直す前は「待ちの打ち切り［止まった］: 「起動の会話」— 相手が 30.0 秒のあいだ状態を進めなかった（…それまでの進み 4 回…）」で赤（X1 の文と同じ頭）、直した後は同じ設定で緑（1.41 秒）。設定は 100 ms に戻した。
- 直し: `drive_talk` は、その会話の `TalkStarted` を見るまで注入の時刻を据え置いたまま Tick を投函し続け、見た後だけ進める。据え置いた時刻は観測を追い越せない。上限・刻み・確かめの内容は変えていない（要件 7.5・10.1）。

## ⑾ 修正の後の静かな机の全体テストと 1.20 倍の線

## ⑿ 最後の負荷 5 回の赤の読み分け
