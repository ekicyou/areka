# 設計レビュー: areka-P0-ghost-restart-unit

> 2026-09-24 `/kiro-validate-design`（非対話・サブエージェント）。対象は `design.md`（コミット `7085a696`）・`requirements.md`・`research.md`・`brief.md`・steering（`structure.md`・`logging.md`・`tech.md`）。設計が「今日の形」として挙げる事実は、ブランチ `claude/areka-p0-ghost-restart-a27fd7` の実ソースで 1 つずつ引き直した（下の「検証した主張」）。引用は「何の定義か」（関数名・型名＋ファイルパス）で指す。

## レビュー要約

設計は既存の部品の並べ替えに徹しており、8 か所の結線・終了順序・借用の形・窓だけ閉じる操作のいずれも、実ソースと突き合わせて成立する（新しい機構なし・kanade／`areka-ghost`／wintf／examples は無改変で済む）。要件 1〜7 の受入基準はすべて設計要素へ辿れ、本番ソースの字面で形を固定している 3 本のテストの追随先も具体的で正しい。ただし、2 周テストの「前の状態が残っていない」の判定式が実際のチャネルの振る舞いと食い違っており（緑のまま欠陥を見逃す形）、また「閉じた証」の型が起こし直しの実際の手順で 2 つの消費先へ届かない（消費が 1 回しかできない）。この 2 点は設計ディスカッションで確定させる必要がある。

## クリティカルな問題（2 件）

### 🔴 Critical Issue 1: 2 周テストの「受信端がつながっている」判定が、古い受信端でも真になり得る

**Concern**: 設計は判定 ⑵ の根拠を「`Emo2Wiring` の停止通知の受信端と `UserBreakWiring` の旗の受信端がつながっている（1 周目の値なら送出端が 1 周目の終了で落ちて `Disconnected` になる）」と書く。しかし実ソースでは、1 周目の終了で古い受信端には**未読の値が残る**——kanade は終了系列の `StopSelf` で `KanadeStopped` を必ず 1 件送る（`spawn_kanade_with_stop_sink`・`crates/areka-kanade/src/actor.rs`）し、`NoUserBreakCueSink` はトークごとに `TalkStarted` を送る（`crates/areka/src/emo2_boot/user_break_cue.rs`）。テストはフレームの系（`run_ghost_quit_phase`・`drain_no_user_break_signals`）を 1 度も回さないので、これらは読まれずに溜まる。`std::sync::mpsc::Receiver::try_recv` は溜まった値がある限り `Ok(_)` を返し、`Disconnected` は**空になってから**しか返らない。したがって「`Disconnected` でなければ新しい」という述語は、古い受信端に対しても `Ok(_)` を見て「つながっている」と答える。

**Impact**: 要件 6.1 ⑵ が固定すべき判断（載せ替えが前のものを残さない）が、載せ替えを丸ごと省いた実装でも緑になり得る。要件 6.1 の唯一の目的がこの判断の固定なので、検査の意味が失われる。

**Suggestion**: 「つながっている」の述語を「`try_recv` を `Err` が出るまで回し、最後の `Err` が `Empty` なら新しい・`Disconnected` なら古い」と定義する（`#[cfg(test)]` の口 `kanade_stop_connected`／`flag_source_connected` の本文をそう書く）。あわせて、実装時に**載せ替えをわざと省いた状態で 1 度赤を確認する**（較正）ことを Testing Strategy に 1 行足す。判定を集めてから 1 回で主張する方針はそのまま。

**Traceability**: 要件 6.1 ⑵・2.3（載せ替えは前のものを残さない）
**Evidence**: design.md「Testing Strategy › 決定論テスト（新規 3 本）› 1. 2 周テスト › 判定 ⑵」および「足す小さな口」・research.md §8.2「2 周テストで 1 周目と 2 周目の状態を見分ける面」

### 🔴 Critical Issue 2: `WindowsClosed` は 1 回しか消費できないのに、起こし直しの手順では 2 つの関数が消費する

**Concern**: `WindowsClosed` は欄が私有・`Clone`／`Copy` なし・`#[must_use]` の値で、「消費先は `reopen_ghost_windows` と `reboot_ghost` の 2 つに限る」と定めている。ところが起こし直しの流れ（設計の sequence diagram・#13 が踏む順）は「降ろす → 閉じる → **窓を積む（`reopen_ghost_windows(world, cfg, closed)`）→ 載せ替える（`reboot_ghost(world, closed, …)`）**」で、同じ 1 つの証を 2 つの関数へ順に渡す。値渡しの証は 1 度目で消えるので、この手順は型検査を通らない。2 周テストは `reopen_ghost_windows` を呼ばない（`WintfTaskPool` を挿さない）ので、この矛盾はテストでは露わにならず、#13 の設計時に初めて表面化する。

**Impact**: `reopen_ghost_windows`／`reboot_ghost`／`close_windows_for_restart` の署名は Revalidation Triggers に挙げた下流 3 本（#13・#50・#15）の契約そのものである。今の形のまま実装すると、下流が最初に踏む手順が組めない。

**Suggestion**: 証を**連鎖させる**形に決める。例: `reopen_ghost_windows(world, cfg, closed: WindowsClosed) -> Result<RestartWindows, OpenWindowsError>`（`RestartWindows { descript: StartupDescriptValues, proof: WindowsReopened }` の欄は私有）とし、`reboot_ghost(world, windows: RestartWindows, inputs, ghost, balloon)` がそれを消費する。2 周テストは `reopen` を通さないので、テスト用に `close → reboot` の直結を許すなら、その直結の口（`RestartWindows::without_windows(closed)` のような `#[cfg(test)]` の組み立て）を設計に明記する。どの形でも「証の消費先が起こし直しの側だけ」という裁定 2（要件 7.2）の趣旨は保てる。

**Traceability**: 要件 4.4（呼び手の限定）・3.4（2 度目も同じ手順で窓を作る）・4.2（戻り値の形）
**Evidence**: design.md「app_exit › close_windows_for_restart + WindowsClosed › Service Interface」「ghost_session › open_ghost_windows / reopen_ghost_windows」「System Flows › 起こし直し」

## その他の指摘（クリティカルではない・実装時の申し送り）

- `OpenWindowsError::TaskPoolMissing` は新しい判断分岐（`error!`＋`Err`）だが、それを踏むテストが無い。本番では `EcsWorld::new` が必ず `WintfTaskPool` を挿す（`crates/wintf/src/ecs/world/mod.rs`）ので実害は無いが、「判断分岐はテストで固定する」の方針と揃えるなら `ghost_session_restart_tests.rs` に素の `World` で `open_ghost_windows` を呼ぶ 1 本（`Err(TaskPoolMissing)` を主張）を足すか、足さない理由を設計に 1 行書く。
- `open_startup_window` の名は doc の中でも参照されている（`input_events/mod.rs`・`menu/mod.rs`・`placement/chain_finalize.rs`・`placement/spawn.rs`・`emo2_boot/mod.rs` の `derive_scopes` doc）。設計の grep 語（「1 度しか」ほか）には掛からないので、実装時に `open_startup_window` でも grep して言い換える。
- `register_emo2_frame_system` の中の登録行は `t_n10`／`t_zwi06` が `add_systems(Update, emo2_frame_system.after(update_typewriters));` の 1 行として押さえる。`world.resource_mut::<Schedules>().add_systems(...)` へ書き換えると rustfmt が鎖を折るが、`frame_schedule_tests.rs` の `register_like_the_boot` と同じ折り方なら末尾の 1 行は保たれる。実装後に必ず両テストを回す。
- `finish_run` は「必ず降ろしてから `run.and(shutdown)`」——`Result::and` は値を取るので `run.and(session.shutdown(...))` と書けば降ろす側は必ず評価される。`run.and_then(|_| ...)` の形にしないこと（`Err` の腕で降ろさなくなる）。

## 設計の強み

1. **分離が行単位で機械的であることを実ソースで裏付けている。** 8 か所すべてで「状態の `insert_non_send`／`insert_resource`」と「`add_systems` の 1 行」が隣り合い、`wire_zorder_pair` だけがゴーストごとの状態を持たない（`ZOrderPairStrategy` は設定値）——この見立ては正しく、案 C（分割は元のファイル・入口と単位だけ新ファイル）は字面テストの走査先の移動を `t_zwi08` 1 本に抑える最小形である。fallback 起動で `Input` の 5 系が登録されたままになる差も明示されており、5 系がすべて `&mut World` の排他系で状態不在なら早期に戻ることを確認した（`drain_readme_requests`・`drain_no_user_break_signals`・`poll_menu_query`・`clear_balloon_hover_on_leave`・`drain_choice_selections`）。
2. **降ろす順序を既存の試験台と同じにして hang の危険を引き継がない。** `GhostSession::shutdown` の ①②③ は `SpineHarness::shutdown_bounded`（`emo2_boot/spine.rs`）が実証済みの順（shutdown → 送出端の drop → seriko の有界 join）そのものであり、`finish_run(run, session)` を `Result` を引数に取る純粋な関数にしたことで、`WinApp::run` に触れずに #57 の腕（要件 1.4・6.3）を決定論で固定できる。

## 検証した主張（設計の「今日の形」と実ソースの突合）

| 設計の主張 | 実ソース | 結果 |
|---|---|---|
| `fn main` の終了順序 ①〜④・`app.run()?` で失敗時は通らない | `crates/areka/src/main.rs` `fn main` | 一致 |
| `wire_emo2_boot` は `&WinApp` で `borrow_mut` 4 回 | `emo2_boot/mod.rs`（`insert_non_send`・`add_systems`・`wire_readme`・`wire_user_break`） | 一致 |
| 8 か所とも状態と登録が隣接する 1 行ずつ | `readme.rs`・`menu/mod.rs`・`user_break.rs`・`choice_drain.rs`・`balloon.rs`・`spawn.rs`・`main.rs` | 一致 |
| `wire_mouse_input` は状態のみ | `input_events/mod.rs` `wire_mouse_input` | 一致 |
| `despawn_app_windows` 私有・`quit_app` が呼んでから `request_exit` | `app_exit.rs` | 一致 |
| `WintfTaskPool` は `pub`・`Resource`・`spawn` は `pub`／`EcsWorld::spawn` は資源不在で黙って何もしない | `crates/wintf/src/ecs/widget/bitmap_source/task_pool.rs`・`ecs/world/mod.rs` | 一致（wintf 無改変で足りる） |
| `AppExit::new`／`is_requested` は公開 | `crates/wintf/src/runtime/message_loop.rs` | 一致 |
| `pub(in path)` は祖先にしか絞れない | Rust の可視性規則 | 一致（⒝ 不可の根拠は正しい） |
| smoke の目印は `WIRED`・`REAL_WINDOWS`・起動解決の 2 行・告知の文面 | `crates/areka/tests/smoke_boot_loop_exit.rs` | 一致（設計は本文を動かさない） |
| 3 本の字面テストの走査先と押さえる行 | `zorder_wiring_tests.rs` `t_zwi06`／`t_zwi08`・`frame_schedule_tests.rs` `t_n10`・`spawn_zorder_chain_wiring_tests.rs` | 一致（追随先の記述は正しい） |
| 行数 774／727／459／352／154 | `wc -l` | 一致 |
| #55 は brief のみ | `.kiro/specs/areka-P0-shiori-fault-notice/`（`brief.md` だけ） | 一致（裁定 1 の前提は成立） |
| 古い受信端は `Disconnected` になる | `actor.rs` の `notify_stop`・`user_break_cue.rs` の `TalkStarted` | **不一致**（未読の値が残る → Critical Issue 1） |
| 証の消費先は `reopen` と `reboot` の 2 つ | 値渡し・`Clone` なし | **矛盾**（1 回しか消費できない → Critical Issue 2） |

要件 5.4（kanade・`areka-ghost`・wintf・examples を触らない）: 設計が触るファイルの表は `crates/areka/src` 配下のみで、wintf の公開 API（`WintfTaskPool`・`AppExit`・`Schedules`・`executor::spawn_local`）だけを使う。抵触なし。
要件 6.5（3 本は削除せず追随）: `t_zwi08` は走査先を `ghost_session.rs` へ・`t_zwi06` は対照の字面を更新・`t_n10` と `spawn_zorder_chain_wiring_tests.rs` は無変更で緑。抵触なし。
要件のトレース: 1.1〜7.5 の全項目が Requirements Traceability の表で設計要素へ辿れる。欠落なし。

## 最終判定

**GO**（条件付き: 上の 2 件を設計ディスカッションで確定し、design.md の該当箇所を改めてからタスク生成へ進む）。

**根拠**: 構造上の対立（依存方向・既存 spec の裁定・触ってはならない範囲）は無く、新しい機構も無い。2 件はいずれも設計の局所（テストの述語 1 つ・型の受け渡し 1 か所）で閉じ、案 C の骨格を変えない。

**次の段階**: `/kiro-design-discussion areka-P0-ghost-restart-unit` で Critical Issue 1・2 を議題にし、確定後に `/kiro-spec-tasks areka-P0-ghost-restart-unit`。
