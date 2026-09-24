# ギャップ分析: areka-P0-ghost-restart-unit

> 2026-09-24・要件確定後の実装ギャップ分析（`/kiro-validate-gap`）。対象のコードはブランチ `claude/areka-p0-ghost-restart-a27fd7`（main `5db3672a` 相当）で読んだ。引用は「何の定義か」（関数名・型名＋ファイルパス）で指す。本文書は選択肢と根拠を並べるもので、最終決定は設計ディスカッションで行う。

## 1. 要約

- **要件 1〜4 の 4 つの括り出しはいずれも既存の部品の並べ替えで成立し、新しい機構は要らない。** 終了順序は `fn main`（`crates/areka/src/main.rs`）の `app.run()?` の後ろ 4 段がそのまま関数の本体になる。8 か所の結線は「`insert_non_send`／`insert_resource`（状態）」と「`add_systems`（系の登録）」が同じ関数の隣り合う行に並んでいるだけなので、行を 2 つの関数へ分けるだけで分離できる。
- **`wire_emo2_boot` の `&mut World` 化は機械的である。** 中で `app.world().borrow_mut()` を 4 回行っているが、使っているのは `world_mut().insert_non_send`・`add_systems`・`wire_readme`・`wire_user_break` の 4 つで、いずれも `&mut World` で足りる（`EcsWorld::add_systems` は `Schedules` 資源への薄い包みで、`has_systems` は `EcsWorld::new` が既に真にしている）。既存の試験台 `SpineHarness`（`emo2_boot/spine.rs`）が同じ手順を `&mut World` で写している事実がその証拠になる。
- **最も設計判断を要するのは「窓を作る」側の経路である。** 今日の `open_startup_window` は窓の生成を `WintfTaskPool` 経由の非同期コマンド（`app.run()` の最初の tick で World へ適用）に積む。同じ経路は素の `&mut World` からも `world.get_resource::<WintfTaskPool>()` で辿れる（`EcsWorld::spawn` 自身がそう書いてある）ので、1 度目と 2 度目以降で同じ手順を使える。ただし「フレームの系の中から積む」場合は次の tick で適用される（同期ではない）。
- **要件 4（窓だけ閉じる操作）は `app_exit.rs` に関数を 1 つ足すだけだが、「呼び手を限定する形」が設計判断になる。** 候補は ⑴ 引数に「直後に起こす」ための値（起動の入力）を取らせて戻り値でしか続きが組めない形、⑵ 可視性を「起こし直しの単位」のモジュールへ `pub(in …)` で絞る形、⑶ 呼び手が完了の証を返す形。
- **決定論テストの土台は揃っている。** 偽の SHIORI は `ScriptedShioriBackend`＋`ShioriWiring::Custom`（`SpineHarness::boot_with` で実証済み）、系の数は `Schedule::systems_len()`（`frame_schedule_tests.rs`・`menu/mod_wiring_tests.rs` で使用中）、終了の指示は `wintf::AppExit::new()`＋`is_requested()`（`frame_ghost_quit_tests.rs`）で観測できる。無いのは「同じ World で 2 周する」テストと、「`app.run()` の失敗を偽で作る口」の 2 つ。

## 2. 現状調査（要件が触る既存資産）

### 2.1 終了順序（要件 1）

`fn main`（`crates/areka/src/main.rs`）の `app.run()?;` の後ろは次の 4 段で、すべて `main` のローカル変数を消費する。

| 段 | 消費する値 | 型 | 出所 |
|---|---|---|---|
| ① loop ticker の停止 | `loop_ticker` | `Option<mpsc::Sender<TickerMsg>>` | `Emo2BootOutcome.loop_ticker`（`emo2_boot/mod.rs`） |
| ② ゴースト実行系の終了 | `ghost_runtime` | `Option<GhostRuntime>` | `Emo2BootOutcome.ghost` または fallback の `areka_ghost::boot` |
| ③ seriko の join | `seriko_handle` | `Option<ActorHandle>` | `Emo2BootOutcome.seriko` |
| ④ perf の最終報告 | `perf_report` | `Option<ReportHandle>` | `perf_thread_report::start()`（`app.run()` より前・プロセスに 1 回） |

- `GhostRuntime::shutdown(self, reason: CloseReason)`（`crates/areka-ghost/src/runtime.rs`）は self を消費し `Result<(), GhostShutdownError>` を返す。`ReportHandle::stop_and_report_final(self)`（`perf_thread_report.rs`）も self を消費する。
- ②③ の失敗は `error!` の上で `Err(E_FAIL)` を返し以降を飛ばす（要件 1.5 はこれを変えない）。① の失敗は `debug!` で流す。
- **④は「プロセスに 1 回」の性質**（報告スレッドはプロセス寿命）で、①〜③（ゴーストごと）とは寿命が違う。要件 1.1 は 4 段を「1 つの関数」と書くが、④をゴーストごとの関数に含めると 2 周目で報告スレッドが既に無い。→ 議題 1。
- `app.run()` は `WinApp::run(&self) -> Result<()>`（`crates/wintf/src/runtime/mod.rs`）。doc の契約に「戻った後の `run()` は再入できない（登録表が World に無い）」とある。**本仕様の「2 周」は `run()` の中で起こる前提**（切替は `run()` を抜けない）であり、`run()` を 2 度回すことではない。決定論テストは `run()` を回さず World を直に駆動する（`SpineHarness` と同じ）。

### 2.2 「1 回だけ登録する」結線 8 か所（要件 2）

各関数の「状態」と「系の登録」を行単位で実測した。

| # | 関数 | 状態（ゴーストごと） | 系の登録（プロセスに 1 回） | 「1 度」の前提コメント |
|---|---|---|---|---|
| 1 | `wire_emo2_boot`（`emo2_boot/mod.rs`） | `Emo2Wiring` を `insert_non_send`（手順 6）。内部で #3・#4 を呼ぶ | `add_systems(Update, emo2_frame_system.after(update_typewriters))` | あり（「`main` から 1 度しか呼ばれない」） |
| 2 | `menu::wire_menu_with`（`menu/mod.rs`） | `MenuWiring` を `insert_non_send`（組込 2 項目の登記込み） | `Input` へ `trigger::poll_menu_query.after(dispatch_pointer_events)` | あり（`wire_menu` doc「1 回だけ呼ぶ」） |
| 3 | `readme::wire_readme`（`readme.rs`） | `ReadmeWiring { path, rx, missing_logged }` を `insert_non_send` | `Input` へ `drain_readme_requests.after(dispatch_pointer_events)` | あり（`wire_emo2_boot` 側のコメント） |
| 4 | `user_break::wire_user_break`（`input_events/user_break.rs`） | `UserBreakWiring` を `insert_non_send` | `Input` へ `drain_no_user_break_signals.before(dispatch_pointer_events)` | あり（「1 回の実行につき 1 度だけ」） |
| 5 | `choice_drain::wire_choice_drain`（`input_events/choice_drain.rs`） | `ChoiceForwarder { kanade }` を `insert_non_send` | `Input` へ `drain_choice_selections.after(dispatch_pointer_events)` | あり（「1 回・同期」） |
| 6 | `balloon::wire_balloon_choice`（`input_events/balloon.rs`） | `BalloonWiring`・`ChoiceSelectionInbox` を `insert_non_send` | `register_balloon_leave_system`（`Input` へ `clear_balloon_hover_on_leave.after(dispatch_pointer_events)`） | あり（「`main.rs` から 1 回・同期呼出」） |
| 7 | `placement::spawn::wire_zorder_pair`（`placement/spawn.rs`） | `ZOrderPairStrategy` を `insert_resource`＋起動時ログ | `FrameFinalize` へ 3 本の `.chain()` | あり（「schedule 実行外で 1 回だけ同期に呼ぶ」） |
| 8 | `open_startup_window`（`main.rs`） | 監視の 2 源（`insert_resource` ×2）・窓の生成・受け口の装着・復元（非同期コマンドの中） | `FrameFinalize` へ `register_ghost_windows_click_through`・`attach_os_close_request`、および #7 の呼出。smoke の自動終了 | なし（doc に「同じ `FrameFinalize` schedule へ結線する」） |
| 9 | `input_events::wire_mouse_input`（`input_events/mod.rs`） | `MouseWiring` を `insert_non_send` のみ | なし | — |

観察:

- **系の登録はすべて `world.resource_mut::<Schedules>().add_systems(...)` 1 行**（#1 のみ `EcsWorld::add_systems` 経由だが中身は同じ `Schedules` への委譲）。分離は「その 1 行を別関数へ移す」に尽きる。
- **状態の載せ替えは `insert_non_send`／`insert_resource` の意味論で無料**（bevy は同型の既存資源を置き換えて古い方を drop する）。`Receiver` を持つ状態（`ReadmeWiring.rx`・`UserBreakWiring.flag_rx`・`ChoiceSelectionInbox`・`Emo2Wiring` の 5 本の受信端）は置き換えで古い受信端が落ち、古いゴーストの送出端は `Err` を返すようになる。**順序の制約**: 古いゴーストを降ろしてから載せ替える（逆だと降ろす途中の送出が新しい受信端へ届く）。
- 系はすべて自己防御（`get_non_send` が `None` なら `trace!`＋無操作）なので、**登録したまま状態だけ入れ替えても系は新しい状態を見る**。`emo2_frame_system` は `remove_non_send::<Emo2Wiring>()`→戻すの形で、載せ替え後も同じ。
- 「1 度」の前提コメントは実測 **7 か所**（要件は「6 か所」と書く。上の表の #1〜#7）。要件 2.5 は数ではなく「明記しているコメントすべて」と読めば矛盾しない。
- #2 `MenuRegistry` は後続 spec が「ゴースト」枠へ登記する口（`menu::register`）を持つ。**載せ替えで `MenuWiring` を新品に置き換えると、本仕様の範囲外の登記も消える**。今日は登記者が本体の 2 項目だけなので害は無いが、#13 が乗るときの契約を設計で決めておく必要がある（→ 議題 4）。

### 2.3 起動の結線の借用の形（要件 3）

- `wire_emo2_boot(app: &WinApp, …)` は `app.world().borrow_mut()` を 4 回行う（手順 6 の `insert_non_send`・`add_systems`・`wire_readme`・`wire_user_break`）。4 回とも `world_mut()` で得た `&mut World` を渡すか `EcsWorld::add_systems` を呼ぶだけ。**`&WinApp` を `&mut World` に替えても他の引数・戻り値は変えずに済む。**
- `open_startup_window(app: &WinApp, cfg: &ConfigInputs)` が `WinApp` を要する理由は 3 つ: ⑴ `app.world().borrow_mut().add_systems(...)`（`&mut World` で代替可）、⑵ `app.world().borrow().spawn(|tx: CommandSender| async move { … })`（`EcsWorld::spawn` は `self.world.get_resource::<WintfTaskPool>()` を引いて `task_pool.spawn(f)` するだけなので、**素の `&mut World` からも同じ資源を引いて同じことができる**）、⑶ smoke の `Rc::downgrade(&app.world())`（`Rc<RefCell<EcsWorld>>` の弱参照が要る＝これだけは `WinApp` 側に残す。要件 3.5 が「1 度目でだけ」と定めているので矛盾しない）。
- 非同期コマンドの中身（`insert_resource(snapshot/dpi_table)` → `spawn_ghost_windows` → `clear_default_char_pos` → `attach_char_pointer_handlers` → `menu::attach_release_handlers` → `attach_balloon_pointer_handlers`）は**全部 `&mut World` を取る関数**。つまり「窓を作る」本体は既に `fn(&mut World)` の形で、非同期コマンドは単にそれを最初の tick へ運ぶ包み。
- `menu::attach_release_handlers` の doc は既に「窓を作り直す spec は作り直した窓へもう一度呼ぶ」と書いている（2 度目の呼出を想定済み）。
- `wire_emo2_boot` の `derive_scopes()` は `vec![0, 1]` 固定（DD-12 の申し送り）。本仕様は触らないが、2 体目のゴーストが 1 スコープでもそのまま `[0, 1]` を返す。窓と資産の不一致は `plan_attachments` が縮退させるので落ちない。**振る舞いを変えない範囲では現状維持**（#13 の課題）。

### 2.4 「全窓を閉じるが終了しない」操作（要件 4）

- `despawn_app_windows(world) -> usize`（`app_exit.rs`・私有）は `GhostWindowMarker` を持つ entity を集めて despawn する。`quit_app` はこれを呼んでから `AppExit::request_exit()`。
- `AppExit`（`crates/wintf/src/runtime/message_loop.rs`）は `is_requested()` を公開しており、「終了の指示が立っていない」を World から直接観測できる。
- `WinApp::run` の手順 4.5 は「指示で戻るとき登録表に残った窓を壊す」だけで、`run()` の途中で窓が 0 になっても `ExitPolicy::Explicit` では戻らない（完了 `app-lifetime-separation`）。**窓 0 で生きていることは既に保証されている。**
- `run_ghost_quit_phase`（`emo2_boot/frame.rs`）は停止通知を受けると必ず `quit_app` へ進む。本仕様はここを変えない（要件 4.6）。#13 が「切替のときは `quit_app` を呼ばない分岐」を足す場所（#55 とも共有）。

### 2.5 起動成功時の後処理（要件 2.6）

- `on_boot_ok(world, runtime, ghost_decision, balloon_decision)`（`main.rs`）は既に `&mut World` を取り、wired・fallback の両腕から呼ばれる。載せ替えの単位から呼ぶのに追加の変更は要らない。`GhostDecision`／`BalloonDecision` は起動前の解決の結果で、2 周目のテストでは合成値を渡す必要がある（型は `boot_resolve.rs`。合成の作りやすさは未確認 → 調査項目）。

### 2.6 本番ソースの字面で形を固定している既存テスト（要件 6.5）

| テスト | 見ている字面 | 本仕様で動く行 |
|---|---|---|
| `emo2_boot/zorder_wiring_tests.rs` `t_zwi08` | `main.rs` の `let StartupDescriptValues { author_dpi, zorder_raw, } = match open_startup_window(&app, &cfg) {`・`let zorder_raw = prepared.zorder_raw.clone();`・`author_dpi, zorder_raw.as_deref(), );` | `open_startup_window` の分割で呼出行が変わる。`wire_emo2_boot` の引数が `&mut World` になっても末尾 2 引数の字面は保てる |
| 同 `t_zwi05`・`t_zwi06`・`t_zwi09` | `mod.rs` の channel／sink／`sinks: vec![…]`／`add_systems(Update, emo2_frame_system.after(update_typewriters));`／`seed_zorder_descript_base` の位置 | 登録の 1 行を別関数へ移すと `t_zwi06`・`frame_schedule_tests.rs` の `BOOT_REGISTRATION` は**同じファイル内**に残る限り緑（`include_str!("mod.rs")` は関数境界を見ない）。別ファイルへ移すと赤 |
| `emo2_boot/frame_schedule_tests.rs` `t_n10` | `mod.rs` に `add_systems(Update, emo2_frame_system.after(update_typewriters));` があり、`pub fn wire_emo2_boot(` がある | 関数名を変えると赤。`&mut World` 化は `pub fn wire_emo2_boot(` の字面を保てる |
| `placement/spawn_zorder_chain_wiring_tests.rs` | `spawn.rs` に `pub fn wire_zorder_pair(world: &mut World) {` と `FrameFinalize, ( establish_owner_links, … ) .chain(),` | `wire_zorder_pair` を「登録」側に残せば無変更。「状態」（`ZOrderPairStrategy` の挿入）を分けるなら関数内の行が減るだけで字面は残る |

structure.md の規約「`include_str!` で本番ファイル本文を読む構造テストは兄弟テストファイルも走査対象に列挙する」——**本仕様が登録の行を新しいファイルへ移す場合、これらの走査先も新しいファイルへ追随させる**（要件 6.5「新しい形の字面へ追随」）。

### 2.7 テストの土台

- **偽の SHIORI**: `ScriptedShioriBackend::builder()…build()`＋`ShioriWiring::Custom(Box::new(move || Ok(Box::new(backend) as Box<dyn ShioriBackend>)))`（`SpineHarness::boot_with`）。`TickerMode::Disabled` で tick を注入駆動。`ExitKind::Clean` の unload まで台本化できる＝**降ろす関数が呼ばれたことを `ScriptedShioriHandle` の発火列（`OnClose`・`Unload`）で観測できる**（要件 6.3 の「降ろす関数が呼ばれた」の証拠になる）。
- **偽の資産**: `SpineHarness` は実 emo2 検体（`acquire_emo2()`＝`sample_test_support`）で `build_boot_assets` を組む。`frame_test_support::synth_assets` は GPU 無しの合成資産。2 周テストがどちらを使うかは設計で決める（実検体なら `wire_emo2_boot` 本体を呼べる可能性がある。要件 3.2 は「その形で呼ぶ決定論テストがコンパイルして通る」を求めるので、**本体を呼ぶ方が要件に直接応える**。ただし `wire_emo2_boot` は `spawn_emo_text`（UI pump スレッド前提・テストスレッドで可＝`SpineHarness` 実績）と `spawn_loop_ticker`（実時計・16ms）を起こす。ticker は `TickerMsg::Close` で止められる）。
- **系の数**: `world.resource::<Schedules>().get(Input).systems_len()`（`menu/mod_wiring_tests.rs`）。`Update`・`FrameFinalize` も同じ。**2 周後の各段の `systems_len()` が 1 周後と同じ**が要件 6.1 ⑴ の判定式になる。
- **状態の置き換え**: `Receiver` を持つ状態は「古い送出端から送ると `Err`」で識別できる（`UserBreakWiring` なら `flag_rx` の対の送出端）。`MouseWiring`・`MenuWiring` は kanade の `Sender` を持つので「新しいゴーストの受信端へ届く／古いゴーストの受信端へ届かない」で識別できる。
- **終了の指示**: `world.insert_non_send(wintf::AppExit::new())`＋`is_requested()`（`frame_ghost_quit_tests.rs::world_with_ghost_windows`）。
- **smoke**: `crates/areka/tests/smoke_boot_loop_exit.rs` は 3 方向とも子プロセスで `AREKA_APP_SMOKE_EXIT_MS`＋`AREKA_NO_ALERT=1` を渡して終了コードとログの目印を見る。本仕様の変更は「ログの語彙・順序が同じ」で緑を保つ（要件 5.3）。

### 2.8 行数（要件 5.6）

`main.rs` 774・`emo2_boot/mod.rs` 727・`frame.rs` 459・`frame/wiring.rs` 352・`app_exit.rs` 154（`wc -l` 実測・要件と一致）。`main.rs` から終了順序と `open_startup_window` の「窓を作る」側を出せば **150〜250 行減る**見込み。

## 3. 要件と資産の対応（ギャップの印: Missing／Unknown／Constraint）

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| 1.1〜1.3・1.6 終了順序を関数へ | `main.rs` の 4 段・`Emo2BootOutcome`（3 ハンドル） | **Missing**: 3 ハンドルを束ねる型（または引数 3 つの関数）と、`main` からの呼出 |
| 1.4・7.1 `app.run()` 失敗の腕 | `app.run()?` の `?` | **Missing**: `let run = app.run(); <降ろす>; run?` の形。#55 は要件未生成（brief のみ・2026-09-24 実測）なので裁定 1 の前提は成立している |
| 1.5 段の失敗の扱い | 各段の `error!`＋`Err` | 変更なし |
| 1.7 `Drop` を採らない | `GhostRuntime` は `Drop` 未実装 | 変更なし（採らない） |
| 2.1・2.2 8 か所の分離 | 上の表 #1〜#8 | **Missing**: 各関数の `add_systems` 行を別関数へ。2 度目の扱い（無視＋記録 or 1 度しか呼ばれない形）は設計 |
| 2.3 前の状態を残さない | `insert_non_send` の置換意味論 | **Constraint**: 降ろしてから載せ替える順序。`MenuWiring` の登記の扱い（議題 4） |
| 2.4 `wire_mouse_input` | 状態のみ | 変更なし（載せ替えの単位から呼ぶだけ） |
| 2.5 コメント書き換え | 7 か所（要件は 6） | **Missing**: 書き換え。数は「該当箇所すべて」 |
| 2.6 `on_boot_ok` | `&mut World` 済み | 変更なし |
| 3.1・3.2 `&mut World` 化 | `wire_emo2_boot` の 4 回の借用 | **Missing**: 署名変更。`main_ghost_wiring_tests`／`wire_tests` の呼出も追随（`wire_emo2_boot_falls_back_to_unwired_on_missing_ghost_root` は `WinApp::new()` を作って渡している→ `app.world().borrow_mut().world_mut()` に変えるだけ） |
| 3.3・3.4 `open_startup_window` の分割 | 非同期コマンドの中身は `&mut World` 関数群 | **Missing**: 「登録」と「窓を作る」の 2 関数。窓を作る側の経路（同期／非同期コマンド）は議題 2 |
| 3.5 smoke は 1 度目だけ | `wintf::executor::spawn_local`＋`Rc::downgrade(&app.world())` | 「登録」側（`WinApp` を持つ `main`）に残す |
| 4.1〜4.5 窓だけ閉じる操作 | `despawn_app_windows`（私有） | **Missing**: 公開の口 1 つ＋ライフサイクル事象の語彙（`event = "…"`）。呼び手の限定の形は議題 3 |
| 4.6 終了経路を変えない | `quit_app`・`run_ghost_quit_phase`・`on_ghost_os_close`・smoke | 変更なし |
| 5.1〜5.3 見え方不変・smoke 緑 | smoke 3 方向・実機ログの語彙 | **Constraint**: ログの語彙と順序を 1 文字も変えない（`info!` の本文は smoke の目印） |
| 5.4 kanade・ghost・wintf・examples 不変 | — | **Constraint**: `EcsWorld::spawn` を使わず `WintfTaskPool` を直接引く場合も wintf は無変更で済む（`pub struct WintfTaskPool`・`pub fn spawn`） |
| 5.6 1,000 行 | 774／727 | 減る方向 |
| 5.7 `GhostBootOptions` に欄を足さない | 構造体リテラル 27 か所・16 ファイル（`grep` 実測・要件と一致） | 変更なし |
| 6.1 2 周テスト | `SpineHarness`・`ScriptedShioriBackend`・`systems_len`・`AppExit::new` | **Missing**: テスト本体。試験台の再利用の形は議題 5 |
| 6.2 窓だけ閉じるテスト | `frame_ghost_quit_tests.rs` の `world_with_ghost_windows` と同型 | **Missing**: テスト 1 本（`app_exit_tests.rs` へ） |
| 6.3 失敗の腕のテスト | — | **Missing**: `app.run()` の結果を引数に取る純粋な関数（`Result<()>` を渡す）にすれば偽の `Err` を渡せる。`WinApp::run` は改変不可（要件 5.4） |
| 6.5 字面テスト 3 本の追随 | 2.6 節 | **Constraint**: 新しい字面へ更新。登録の行を別ファイルへ移すなら走査先も移す |
| 6.6 兄弟ファイル配置 | structure.md の `<stem>_<モジュール名>.rs` | 変更なし |
| 7.2・7.3 完了 spec 3.6 を上書きしない | `despawn_app_windows` は私有のまま | 議題 3 の形次第。「`quit_app` と同じ私有部品を使う」（4.5）は関数を同じファイルに置けば自然に満たす |

## 4. 実装アプローチの選択肢

### 案 A: 既存ファイルの中で分ける（最小差分）

- `main.rs`: 終了順序を `fn shutdown_ghost(session: GhostSession, reason: CloseReason) -> Result<()>` 相当へ。`open_startup_window` を `register_startup_systems(world)`（クリック透過・OS 閉鎖要求・`wire_zorder_pair`）と `open_ghost_windows(world, cfg) -> StartupDescriptValues`（準備→監視の 2 源→復元→窓の生成→受け口の装着）に分ける。smoke は `main` に残す。
- `emo2_boot/mod.rs`: `wire_emo2_boot(world: &mut World, …)`。`add_systems(Update, …)` の行を `pub fn register_emo2_frame_system(world)` へ移し、同じファイルに置く（字面テストが緑のまま）。
- 各 `wire_*`: `add_systems` の行を `register_*` へ切り出し、同じファイルに置く。
- `app_exit.rs`: 関数を 1 つ足す。
- 長所: 触るファイルが要件の列挙どおりで、字面テストの走査先を変えずに済む。行数は増えない（分けるだけ）。
- 短所: 「登録を 1 か所から呼ぶ」入口が無いと、登録関数が 8 か所に散ったまま `main` が 8 回呼ぶ形になる。`main.rs` は減らない（終了順序を出しても、登録の呼出が増える）。

### 案 B: 「起こし直しの単位」を新しいファイルにまとめる

- 新規 `crates/areka/src/ghost_session.rs`（名前は仮）に ⑴ `register_systems(world)`（8 か所の登録側を順に呼ぶ 1 本）、⑵ `boot_ghost(world, inputs) -> GhostSession`（窓を作る→`wire_emo2_boot`→`wire_mouse_input`→`wire_menu`→`on_boot_ok`→`wire_balloon_choice`→`wire_choice_drain`、fallback 込み）、⑶ `GhostSession::shutdown(self, reason)`（①②③）、⑷ 要件 4 の操作（`app_exit.rs` に置き、`pub(in …)` でこのモジュールへ限定する候補）を置く。`main` は `register_systems` → `boot_ghost` → `app.run()` → `shutdown` → perf の 5 行になる。
- 長所: 「登録は 1 回・載せ替えは n 回」が**構造で読める**（登録関数の呼び手が 1 つ）。`main.rs` が大きく減る。#13 が乗る面が 1 ファイルに集まる（roadmap B3「#13 の設計は #58 の形を見てから」）。2 周テストはこのモジュールの兄弟ファイル `ghost_session_restart_tests.rs` に置ける。
- 短所: 新ファイル 1 つ。`main.rs` の字面を見ている `t_zwi08` は `boot_ghost` 側の呼出行を見るよう走査先を変える必要がある（structure.md の規約どおり）。`wire_emo2_boot` の呼出が `main.rs` から消えるので、`t_zwi08` の 3 つ目の主張（`author_dpi, zorder_raw.as_deref(), );`）は新ファイルへ移す。

### 案 C: 折衷（A の分割＋B の入口 2 本だけ）

- 各 `wire_*` の分割は案 A のとおり同じファイル内で行い、新ファイルには `register_systems`（登録 8 本を順に呼ぶ）と `boot_ghost`／`GhostSession` だけを置く。要件 4 の操作は `app_exit.rs`。
- 長所と短所は B とほぼ同じ。違いは「登録関数の定義は元のファイルに残す」点で、字面テストの走査先の変更が `main.rs` 由来の 1 本（`t_zwi08`）に限られる。

**規模**: いずれも **M（brief の 6〜7 タスクと整合）**。**リスク: 低〜中**——新機構は無く既存関数の並べ替えだが、⑴ 見え方不変をログの語彙で守る必要があり（smoke が見張る）、⑵ 2 周テストが実検体＋実スレッド（seriko・emo-text）を同じプロセスで 2 度起こすので、後片付け（`tick_sink` の drop → seriko join・`SpineHarness::shutdown_bounded` の手順）を写し損ねると hang する。有界 join（`join_bounded`・`run_bounded`）を必ず使う。

## 5. 設計フェーズへ持ち越す議題

1. ~~**perf の最終報告（④）の置き場。**~~ **要件ディスカッションで解決済み（2026-09-24）**: 要件 1.1 を「ゴーストごとの 3 段（①②③）を関数に・④はプロセスに 1 回なので `fn main` の末尾（関数の直後）に残す」へ改めた。`Option<ReportHandle>` を 2 周目に `None` で渡す形は「1 回きり」を引数で表すことになるので採らない。
2. **「窓を作る」側の経路。** → **設計で決定（§8 決定 2）: ⒜。** ⒜ 今日どおり `WintfTaskPool` の非同期コマンドに積む（素の `&mut World` から `get_resource::<WintfTaskPool>()` で同じ経路を使える。1 度目の見え方が変わらない。2 度目以降は「次の tick で窓が出る」＝フレームの系の中から積んでも借用が衝突しない）。⒝ `&mut World` へ同期に直接 `spawn_ghost_windows` する（HWND の生成は `run()` の reconcile で起きるので画面上の順序は変わらない見込みだが、`Added<WindowHandle>` を捉える系の登録より先に spawn しても取りこぼさないことは既に doc が保証している。**見え方が変わらないことは smoke と実機で確かめる必要がある**）。要件 3.4「1 度目と同じ手順」は ⒜ の方が字義どおり。
3. **要件 4 の操作の呼び手の限定の形。** → **設計で決定（§8 決定 3）: ⒜（証を返す形）。⒝ は Rust の可視性が祖先モジュールにしか絞れないので兄弟には使えない。** ⒜ 引数に「直後に起こすための入力」（起動の入力の束）を取り、戻り値の型を「閉じた枚数」ではなく「次の起動を続けるための証」にする（`quit_app` の代わりに呼んで終わりにできない）。⒝ 可視性を `pub(in crate::ghost_session)`（案 B）で絞る（`app_exit.rs` の外からは起こし直しの単位しか呼べない。最も短い）。⒞ 型で証を返さず、doc と `#[must_use]` だけ。要件 4.4「終了経路のどこからも呼べない」は ⒝ が構造で満たし、⒜ は「呼べるが続きを組まざるを得ない」形。
4. **`MenuWiring` の載せ替えと登記の扱い。** 新品に置き換えると後続 spec の登記（`menu::register`）も消える。⒜ 載せ替えで新品にし、登記は「ゴーストを起こすたびに登記し直す」を #13 側の契約にする（`MenuRegistry` の doc に 1 行）。⒝ `MenuWiring` のうち kanade の送出端だけを差し替える口を足す（本仕様の範囲外の機能を足すことになる）。本仕様は登記者が 2 項目（本体）だけなので ⒜ が最小。**要件ディスカッション（2026-09-24）の整理**: 要件 2.3 が `MenuWiring` を「新しいゴーストのもので置き換え、前のものを残さない」と定めているので、要件の側は既に ⒜ を選んでいる。設計で決めるのは残り＝「ゴーストを起こすたびに登記し直す」を #13 の契約として `MenuRegistry`（または `menu::register`）の doc に 1 行で書く位置。⒝ は本仕様の範囲外の機能になるので採らない。
5. **2 周テストの試験台。** → **設計で決定（§8 決定 5）: `wire_emo2_boot` 本体を `boot_ghost` 経由で 2 度呼ぶ。`SpineHarness` は拡張せず、台本つき偽 SHIORI・検体・有界待機の部品だけを `pub(crate)` で借りる。** ⒜ `SpineHarness` を拡張して `wire_emo2_boot` 本体を呼ぶ（要件 3.2 に直接応える。`spawn_loop_ticker` の実時計 ticker が起きるので `TickerMsg::Close` で必ず止める。`SpineHarness` は今 `wire_emo2_boot` を写しているが本体は呼ばない）。⒝ `SpineHarness` の手順をそのまま 2 度回し、`wire_emo2_boot` 本体は「`&mut World` で呼ぶ 1 本の別テスト」（存在しない根で fallback を返す `wire_tests` の既存テストを `&mut World` に変えるだけ）で要件 3.2 を満たす。⒝ は `wire_emo2_boot` の本体を 2 周に含めないので「本体が 2 度呼べる」の証拠は弱い。
6. **登録の 2 度目の扱い（要件 2.2）。** → **設計で決定（§8 決定 6）: ⒜。** ⒜ 登録関数は `main` から 1 度だけ呼ばれる形にし、実行時の見張りは持たない（最小）。⒝ 登録済みの印（Resource 1 つ）を置き、2 度目は `warn!`（または `debug!`）で無視する（判断分岐が 1 つ増え、そのテストが 1 本要る）。2 周テストの判定式（`systems_len()` が同じ）は ⒜⒝ どちらでも成り立つ。
7. **`GhostDecision`／`BalloonDecision` の合成。** → **設計で決定（§8 決定 7）: 合成して `on_boot_ok` を本当に通す。** 2 周テストが `on_boot_ok` を通すには起動前の解決の結果が要る。合成しやすい形か（`boot_resolve.rs`）は未確認→調査項目。通さない（`insert_persist_wiring` だけを通す）なら要件 2.6 の「載せ替えの単位から呼べる形に保つ」はコンパイルで示せる。

## 6. 調査項目（Research Needed）

- `boot_resolve::GhostDecision`／`BalloonDecision` をテストで合成する最短の形（`LastUsed::record` が sylphya へ投函するので、偽の資産でも `sylphya_publisher()` は生きている＝書き込み先は一時フォルダに向ける必要がある。`AREKA_PROFILE_DIR` 相当の扱いは `GhostBootOptions.app_profile_dir: Some(default_app_profile_dir())`——テストは `None` にできる〈`SpineHarness` 実績〉）。
- `WintfTaskPool` が `World::new()` の素の World に無い場合（試験台）に「窓を作る」側がどう振る舞うべきか（`EcsWorld::spawn` は資源が無いと**黙って何もしない**。本仕様の関数は log-first の規律上 `warn!` を残すべきか、テスト用に同期経路を許すか＝議題 2 と連動）。
- 2 周で `spawn_emo_text`（UI pump アクター）を 2 度起こしたときの後片付け（`SpineHarness` は `text_pump` を drop で終える。2 周目の前に 1 周目の pump を drain して落とす必要があるか）。
- `zorder_wiring_tests.rs` `t_zwi08` の走査先を新ファイルへ移す場合の対照（「説明文が落ちていない」の番兵の語）を新ファイルの doc に用意すること。

## 7. 次の段階

`/kiro-design areka-P0-ghost-restart-unit` で設計へ。設計は上の議題 1〜7 に答え、案 A／B／C のいずれか（推しは **案 B または C**＝登録の入口が 1 本・起こし直しの単位が 1 ファイル）を選ぶ。実装は #55 `shiori-fault-notice` の着地後（B2）で、着手時に本文書の 2 節を settled main で引き直す。

→ 2026-09-24 設計生成済み（`design.md`）。決定は §8。

## 8. 設計フェーズの調査と決定（2026-09-24）

### 8.1 要約
- **Discovery Scope**: Extension（既存の部品の並べ替え・新しい機構なし）。外部依存の追加なし。
- **選んだ案**: **案 C（折衷）**。各 `wire_*` は元のファイルの中で「載せ替え（`wire_*`・名前は今日のまま）」と「登録（`register_*`・新設）」に分け、新ファイル `crates/areka/src/ghost_session.rs` に登録の入口 `register_systems` と起こし直しの単位（`open_ghost_windows`／`reopen_ghost_windows`・`boot_ghost`〔2 度目以降も同じ関数〕・`GhostSession::shutdown`・`finish_run`）だけを置く。
- **主な発見**:
  - `pub(in path)` は祖先モジュールにしか絞れない（Rust の可視性の規則）。`app_exit.rs` の項目を兄弟の `ghost_session` へ絞る ⒝ は不可能。
  - `wire_zorder_pair`（`placement/spawn.rs`）の「状態」`ZOrderPairStrategy` はプロセスに 1 回の設定値で、ゴーストごとの状態を持たない＝分割の対象ではなく登録側そのもの。字面テスト（`spawn_zorder_chain_wiring_tests.rs`）は無変更で緑。
  - クレートの根（`main.rs`）の私有関数（`on_boot_ok`・`restore_merged_placements`・`boot_monitor_snapshot`・`insert_persist_wiring`）は子モジュールから見える（`emo2_boot` が `crate::is_benign_boot_error` を呼ぶのと同じ規則）。兄弟テストが `main.rs` に繋がっているので動かさない。
  - `wire_emo2_boot` に偽の SHIORI を入れるには `GhostBootOptions` の `shiori`／`ticker`／`app_profile_dir` の 3 値を入力から取る必要がある（今日は関数の中で固定）。`GhostBootOptions` の欄は足さず（要件 5.7）、入口の型 `Emo2BootInputs` を `emo2_boot` に置く（依存方向を保つため）。
  - `areka_actor::spawn_ui`（文字層の UI アクター）は同じスレッドで何度でも `spawn_local` できる。pump を回さなければ何もせず、drop で落ちる（doc の Risks の記述）。
  - `EcsWorld::new`（`crates/wintf/src/ecs/world/mod.rs`）が `WintfTaskPool` を必ず挿す。`WintfTaskPool::spawn` は `pub`。素の `World` には無い。

### 8.2 調査ログ

#### 可視性で呼び手を絞れるか（議題 3 ⒝）
- **Context**: 要件 4.4「呼び手を限定する形」。
- **Sources**: Rust Reference「Visibility and Privacy」——`pub(in path)` の `path` は現在のモジュールの祖先でなければならない。
- **Findings**: `crate::app_exit` の祖先は `crate` だけ。`pub(in crate::ghost_session)` は書けない。`app_exit` を `ghost_session` の子に移せば書けるが、#55 と共有するファイルを動かすことになる。
- **Implications**: ⒜（戻り値の証 `WindowsClosed`・`#[must_use]`・消費先は `reopen_ghost_windows` の 1 つ）を採る。設計レビュー（2026-09-24）で「値渡しの証は 1 回しか消費できないのに 2 か所で消費する」矛盾が見つかり、`reboot_ghost` を撤去して消費先を 1 つにした。

#### 窓を作る側を素の `&mut World` から積めるか（議題 2）
- **Context**: `open_startup_window` は `app.world().borrow().spawn(...)` で非同期コマンドに積む。
- **Sources**: `EcsWorld::spawn`（`crates/wintf/src/ecs/world/mod.rs`）＝`self.world.get_resource::<WintfTaskPool>()` → `task_pool.spawn(f)`。`WintfTaskPool::spawn`（`crates/wintf/src/ecs/widget/bitmap_source/task_pool.rs`）は `pub`。
- **Findings**: 同じ資源を `world.get_resource::<WintfTaskPool>()` で引けば同じ経路。資源が無いとき `EcsWorld::spawn` は黙って何もしない。
- **Implications**: 1 度目・2 度目とも同じ経路（⒜）。資源が無いときは `error!`＋`Err(TaskPoolMissing)`（log-first）。

#### 2 周テストで 1 周目と 2 周目の状態を見分ける面
- **Context**: 要件 6.1 ⑵「前のものが残っていない」。
- **Findings**: 検体の複製は周ごとに別のフォルダ → `ReadmeWiring.path` が周を語る。`MenuRegistry` に 1 周目だけ余分な登記を入れれば新品かどうかが分かる（議題 4 の契約の実演にもなる）。`Receiver` を持つ状態（`Emo2Wiring.kanade_stop`・`UserBreakWiring.flag_rx`）は 1 周目の送出端（kanade・dispatcher の sink）が 1 周目の終了で落ちるので `Disconnected` と `Empty` で見分けられる——ただし 1 周目の受信端には未読の値（`KanadeStopped`・`TalkStarted`）が残るので、**空になるまで `try_recv` を回して最後の `Err` を見る**（設計レビューの指摘 1。1 度の `try_recv` では古い受信端も `Ok(_)` を返す）。kanade の `Sender` を持つ状態（`MouseWiring`・`MenuWiring` の送り口・`ChoiceForwarder`）と sylphya の投函端（`PersistWiring`）には副作用無しで生死を問う口が無い（std の `Sender` に生死の照会は無い）。
- **Implications**: 証拠の面は 4（path・登記・停止通知の受信端・旗の受信端）、面が無い状態は 0 面と明示。4 面は wired の腕の直線の手順で挿されるので、4 面が新しければ同じ手順を通ったことになる。

#### `GhostDecision`／`BalloonDecision` の合成（議題 7）
- **Findings**: `boot_resolve.rs` の両型は `pub(crate)` の欄だけ（`route: GhostRoute{Argv,…}`・`dir: PathBuf`・`folder: Option<String>`）。`LastUsed::record` は publisher へ投函するだけ。`app_profile_dir: None` は `SpineHarness` の既存の使い方。
- **Implications**: 合成して `on_boot_ok` を本当に通す。

### 8.3 案の評価

| Option | Description | Strengths | Risks / Limitations | Notes |
|---|---|---|---|---|
| 案 A | 既存ファイルの中だけで分ける | 字面テストの走査先を動かさない | `main` が登録を 8 回呼ぶ＝「登録は 1 回」が構造で読めない | 不採用 |
| 案 B | 登録関数の定義も新ファイルへ | 起こし直しの単位が 1 ファイル | 字面テスト（`t_zwi06`・`t_n10`・spawn）の走査先が増える | 不採用 |
| **案 C** | 分割は元のファイル・入口と単位だけ新ファイル | 登録の入口 1 本・字面テストの追随は `t_zwi08`・`t_zwi06` の 2 か所 | 新ファイル 1 つ | **採用** |

### 8.4 設計判断

#### 決定 2: 窓を作る側は今日と同じ非同期コマンドの経路（⒜）
- **Alternatives**: ⒝ `&mut World` へ同期に `spawn_ghost_windows`。
- **Selected**: 素の `&mut World` から `WintfTaskPool` を引いて積む。無ければ `error!`＋`Err`。
- **Rationale**: 1 度目の見え方が変わらない・フレームの系の中から積んでも借用が衝突しない・要件 3.4 を字義どおり満たす。
- **Trade-offs**: 2 度目以降は次の tick で窓が出る（今日の 1 度目と同じ）。同期が要ると分かれば、コマンドの中身が既に `fn(&mut World)` の並びなのでその並びを直接呼ぶ関数を足すだけ（本仕様では作らない）。

#### 決定 3: 呼び手の限定は戻り値の証（⒜）
- **Alternatives**: ⒝ 可視性（不可能）・⒞ doc と `#[must_use]` だけ。
- **Selected**: `close_windows_for_restart(world) -> WindowsClosed`（欄は私有・`#[must_use]`）。消費先は `reopen_ghost_windows` だけ（設計レビュー後に `reboot_ghost` を撤去）。
- **Rationale**: 続きを組まざるを得ない形。`despawn_app_windows` は私有のまま・`quit_app` は不変・終了経路に呼び手を置かない＝完了 spec 3.6 の上書きではない。
- **Trade-offs**: `#[must_use]` は警告どまり。構造の限定は「消費先が 1 つ」が本体。

#### 決定 5: 2 周テストは `wire_emo2_boot` 本体を通す
- **Alternatives**: ⒝ `SpineHarness` の手順を 2 度回す（本体を通さない）。
- **Selected**: `boot_ghost` を 2 度呼ぶ。偽の SHIORI は `Emo2BootInputs.shiori = ShioriWiring::Custom(台本つき)`、`ticker = Disabled`、`app_profile_dir = None`。`SpineHarness` は拡張せず、`spine` の部品（`ScriptedShioriBackend`・`standard_backend`・`run_bounded`・`spin_wait_until`）と `acquire_emo2` を `pub(crate)` に広げて借りる。
- **Rationale**: 要件 3.2 は「その形で呼ぶ決定論テスト（要件 6.1）」と明記する。⒝ では本体が 2 度呼べる証拠が無い。
- **Follow-up**: 実時計の loop ticker が周ごとに起きる——`shutdown` ① が必ず止める。hang 対策は `run_bounded`。

#### 決定 6: 登録の 2 度目は見張らない（⒜）
- **Selected**: `register_systems` の呼び手は `main` の 1 か所。起こし直しの経路に登録は無い。
- **Rationale**: 見張り（Resource＋`warn!`）は判断分岐が 1 つ増えテストが 1 本要るが、構造で読める。2 周テストの ⑴（`systems_len()` が同じ）が「載せ替え側は系を足さない」を固定する。

#### 決定 7: 起動前の解決の結果を合成して `on_boot_ok` を通す
- **Selected**: `GhostDecision { route: Argv, dir: 検体の根, folder: None }`・`BalloonDecision { route: Argv, dir: 同梱バルーン, folder: None }`。
- **Rationale**: 載せ替えの単位の本文を 2 周テストと本番で違えない。

#### 追加の決定: `Emo2BootInputs` は `emo2_boot` に置く
- **Rationale**: `ghost_session` → `emo2_boot` の依存方向を保つ（型を `ghost_session` に置くと `emo2_boot` が上向きに import する）。`ShioriWiring::Custom` は複製できないので値渡し。fallback（LogSink）の boot は `GhostBootInputs.helper_exe` から `ghost_boot_options` を組む（今日と同じ）。

### 8.5 リスクと対策
- 2 周テストの hang — 全待機を有界（`run_bounded`）にし、後片付けの順を `SpineHarness::shutdown_bounded` と同じにする（`GhostSession::shutdown` の ①②③ がその順）。
- fallback の起動で `Input` の 5 系が登録されたまま無操作になる — 自己防御（NonSend 不在で `trace!`）で見え方は不変。design に明示済み。
- `app_profile_dir: None` での `LastUsed::record` の縮退の記録 — テストは記録を判定しないが、実装時に `error!` が出ないことを 1 度目視する。
- `#[must_use]` は警告どまり — 消費先が 1 つしか無いことが限定の本体。
- `OpenWindowsError::TaskPoolMissing` は新しい判断分岐 — 作業プールの有無を配置の準備より先に確かめ、素の `World` で `Err` を主張する決定論テスト 1 本を足す（設計レビューの申し送り）。

### 8.6 参照
- Rust Reference「Visibility and Privacy」（`pub(in path)` の祖先制約）。
- `crates/wintf/src/ecs/world/mod.rs`（`EcsWorld::new`・`EcsWorld::spawn`・`EcsWorld::add_systems`）／`crates/wintf/src/ecs/widget/bitmap_source/task_pool.rs`（`WintfTaskPool::spawn`）／`crates/wintf/src/runtime/message_loop.rs`（`AppExit`）。
- `crates/areka/src/emo2_boot/spine.rs`（`SpineHarness::boot_with`・`shutdown_bounded`・`standard_backend`）／`crates/areka-actor/src/ui.rs`（`spawn_ui` の Risks）。
