# ギャップ分析: areka-P0-frame-phases-after-exit

> 実測は 2026-09-28・本ブランチ（`a6f43ae9`＝main `f233f720` の上に spec 初期化のみ。ソースは `10a8d724` から不変）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。行番号は書かない。
> 入力: `requirements.md`（確定）・`brief.md`・`.kiro/steering/`（product・tech・structure・logging）。

## 1. 分析の要約

- **止める場所は 1 か所で足りることを確かめた。** 毎フレームの処理の全相（作業領域の同期・装着・拡大率・表示の指令の取り出し・バルーンの可視性・窓寸の反映・`\![move]`・重なりの指令・再配置・連鎖の確定と再解決・文字層）は、本番では `emo2_frame_system`（`crates/areka/src/emo2_boot/frame.rs`）の中からしか呼ばれない。ほかの system から同じ相を呼ぶ箇所は 0 件。
- **「終了が指示済み」なら「ゴースト窓はもう無い」が成り立つ。** 本番の終了指示は `quit_app`（`crates/areka/src/app_exit.rs`）の 1 か所だけで、そこでは必ず窓を消してから指示する。areka は `ExitPolicy::Explicit` で起動するので、wintf の「最後の窓が閉じたら終了」の仕掛けも働かない。
- **テストの形は既存の補助関数だけで組める。** `headless_wiring_with`・`zero_clock`・`capture_logs`・`count_level`（`frame_test_support.rs`）と、試験用の既存メソッド `Emo2Wiring::drain_received`（`frame/wiring.rs`）で、補助ファイルに何も足さずに書ける。
- **注意点が 1 つある: 判定は `sync_monitor_snapshot` より前に置く必要がある。** 素の World ではモニタが 0 台なので、作業領域の同期が「モニタ表が空」の WARN を 1 件出す。判定をこれより後ろに置くと、要件 3.1 の「WARN 0 件」がテストで満たせない。本番でも、作業領域の同期に続く再配置は窓を触るので、判定は全相より前が正しい。
- **推奨は「既存の関数を広げる」（案 A）。** 規模は極小（本番の差分は十数行）、リスクは低い。

## 2. 現状の調べ

### 2.1 毎フレームの処理の入口は `emo2_frame_system` だけか

`emo2_frame_system` の定義（`crates/areka/src/emo2_boot/frame.rs`）の本文は、次の順に相を呼ぶ。

1. `Emo2Wiring` を `World::remove_non_send` で取り出す（無ければ何もせず戻る）
2. `work_area_sync::sync_monitor_snapshot`（作業領域の同期）
3. `run_attach_phase`（装着）
4. `run_dpi_phase`（拡大率）
5. `work_area_sync::resnap_for_work_area_change`（作業領域が変わったときの再配置・変化が無ければ呼ばない）
6. `run_drain_phase`（表示の指令の取り出しと適用）
7. `run_balloon_visibility_phase`（バルーンの可視性）
8. `reconcile_reported_sizes`（窓寸の反映）
9. `run_move_drain_phase`（`\![move]` の適用）
10. `run_zorder_drain_phase`（重なりの指令の取り出しと、鎖の公開）
11. `resnap_shell_targets`（シェルの寸法変化での再配置）
12. `finalize_chain_once`（初期配置の連鎖の確定）
13. `realign_chain_once`（拡大率の遷移後の連鎖の再解決）
14. `run_text_scale_phase`（文字層の拡大率の追従）
15. `run_text_phase`（文字層の描画）
16. `Emo2Wiring` を戻す

上の 15 の関数を `crates/areka/src` の本番コード（`_tests.rs` と `frame_test_support.rs` を除く）で呼んでいるのは `emo2_frame_system` の本文だけ。`frame_test_support.rs` の中からの呼び出しはテスト用の多フレーム駆動装置のもので、本番には入らない。

`emo2_frame_system` を schedule に載せるのは `register_emo2_frame_system`（`crates/areka/src/emo2_boot/mod.rs`・`Update` に `.after(update_typewriters)`）の 1 か所、呼び手は `ghost_session::register_systems`（`crates/areka/src/ghost_session.rs`）でプロセスに 1 回。

→ **判定が取りこぼす相は無い。** 判定を本文の先頭に置けば、要件 1.1 に並ぶ相はすべて止まる。加えて、要件に名前の無い作業領域の同期と再配置（上の 2・5）、装着（3）、拡大率（4）も同時に止まる。

`presenter.apply` をほかから呼ぶ箇所も洗った。本番で呼ぶのは `run_drain_phase`・`run_attach_phase`（`frame/attach.rs`）・`run_balloon_visibility_phase`（`balloon_visibility_phase.rs`）の 3 か所で、どれも `emo2_frame_system` の中。`Emo2Wiring::apply_present` は `#[cfg(test)]` だけ。`input_events/balloon.rs`・`input_events/mod.rs`・`input_events/user_break.rs` は `Emo2Wiring` を `get_non_send` で**読むだけ**で、表示は適用しない。

### 2.2 `quit_app` の呼び手と「窓を消してから指示する」

本番の呼び手は 10 か所・出所は 7 種類（テストファイルを除く grep）。

| 呼び手（定義） | ファイル | 出所 |
|---|---|---|
| `quit_as_today` | `emo2_boot/frame.rs` | `KanadeStopped`（メニューの終了・別れの台詞のあと） |
| `on_ghost_stopped`（予約なし・受理されず・既定の失敗・失敗以外の停止の 4 か所） | `emo2_boot/ghost_switch.rs` | `KanadeStopped`（切替の後の終了ほか） |
| `fatal` | `emo2_boot/ghost_switch.rs` | `GhostFallbackFailed`（既定ゴーストへ戻せない致命） |
| ポインタのバブル処理の強制退避の腕 | `input_events/mod.rs` | `Escape` |
| `end_session_within` | `session_end.rs` | `SessionEnd` |
| smoke の自動終了の非同期タスク | `main.rs` | `Smoke` |
| `on_ghost_os_close`（`MouseWiring` が無い腕） | `app_exit.rs` | `OsClose` |

`quit_app` の本文は、最初の行で私有部品 `despawn_app_windows` を呼び（全 `GhostWindowMarker` の entity を despawn・子の surface と文字層の枠も連鎖で消える）、その後で `AppExit::request_exit` を呼ぶ。`despawn_app_windows` は私有で、窓を消さずに指示する経路は構造上作れない。

wintf 側で `request_exit` を呼ぶのは `WinApp::wire_shutdown_hook`（`crates/wintf/src/runtime/mod.rs`）の「最後の窓が閉じたら終了」の仕掛けだけで、これは `ExitPolicy::OnLastWindowClose` のときしか差さない。areka は `main.rs` で `WinApp::with_exit_policy(ExitPolicy::Explicit)` を使うので、この経路は無い。

→ **「終了が指示済み」⇒「ゴースト窓はもう無い」が全出所で成り立つ。** 判定の前提（要件の Adjacent expectations）は現状のコードで保たれている。

補足:
- `fatal` は切替の途中（`close_windows_for_restart` の後、次のゴーストを起こせなかった）で呼ばれる。このとき World の `Emo2Wiring` は古いものか新しいものか経路によって違うが、判定は結線の中身を見ないのでどちらでも止まる。
- 切替（`switch_to`・`switch_to_default`）は `close_windows_for_restart` を使い、`AppExit` に触れない（同関数の説明文と定義で確認）。よって切替の途中で判定が真になることは無く、要件 2.3 は判定の形そのものから満たされる。

### 2.3 終了指示の後に何巡走るか

`WinApp::run`（`crates/wintf/src/runtime/mod.rs`）は `MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(...))` が戻った直後に `running` の旗を下ろし、以後の起床では巡を回さない。したがって:

- **巡の中で終了が指示された場合**（メニューの終了＝`Update` 段の `ghost_quit_system`、強制退避＝`Input` 段）: 同じ巡の残り（`Update` の `emo2_frame_system` 以降）が走る。読み飛ばしの記録は 1 行。判定は受信端を見ないので、指令が残っていたかどうかに関わらず出る（議題 4）。
- **巡の外で指示された場合**（smoke の自動終了＝非同期タスク、OS のセッションの終了＝窓の手続き）: 次の巡が来る前に `block_on` が戻れば 0 行、来れば 1 行。

要件 1.2（巡ごとに 1 行）・要件 4.4（出なくても不合格としない）と食い違わない。記録が大量に並ぶ心配は無い。

### 2.4 同じ巡で消えた窓を読むほかの系（境界外・報告のみ）

| 系・資源 | 段 | 状況 |
|---|---|---|
| `GhostWindows`（資源） | — | `quit_app` は外さない（外すのは `close_windows_for_restart` だけ）。今は終了の巡で `run_move_drain_phase`・`run_zorder_drain_phase`・`finalize_chain_once` が消えた entity を指す `GhostWindows` を読みうる。**本件の判定でこの 3 つも止まる**ので、本件は状況を良くする側に働く。 |
| `ZOrderChainPlan`（資源）と `apply_zorder_chain`（`crates/wintf/src/ecs/window/zorder_chain_apply.rs`） | `FrameFinalize` | `quit_app` は外さない。適用系は計画の `dirty` が立った巡だけ働く。判定があれば終了の巡に新しい鎖は公開されない。去る窓の切離し（`detach_cross_owner_links_for_departing`）は計画を待たずに走るが、全窓が消えると被所有側の印も一緒に消えるので対象が無い。見送りの記録は `debug!`。今回のログに症状は無い。 |
| `establish_owner_links`・`apply_zorder_pair_maintenance`（wintf） | `FrameFinalize` | 未読。今回のログに症状なし。 |
| `Input` 段の取り出し 6 本（説明書・切替要求・中断・メニュー・バルーンの離脱・選択肢） | `Input` | 強制退避は `Input` 段の中で `quit_app` を呼ぶので、同じ段の後続がそのあと走りうる。どれも受信端か照会の結果を取り出すだけで、表示は適用しない。今回のログに症状なし。 |
| wintf の `Update` より後の段（配置の伝播・面の生成・描画・合成） | 各段 | query で entity を拾う作りなので、despawn 済みの窓は現れない。 |

→ 要件の Out of scope（「`quit_app` が外さない資源が終了の巡の後段で消えた窓を読むかどうか」）のとおり、ここは報告だけ。新たに登記すべき症状は見つからなかった。

### 2.5 予定のテストの形が既存の補助で動くか

| 使うもの | 定義 | 確認したこと |
|---|---|---|
| `headless_wiring_with(rx, clock)` | `frame_test_support.rs`（`pub(super)`） | 素の `EmoPresenter::new()`（target 0 件）で結線を組む。GPU・COM 不要。 |
| `zero_clock()` | 同上 | 固定の時計。 |
| `capture_logs(f)` | 同上 | `log_capture_kit::capture_lines(LineFormat::LevelTargetFields, f)`。log-capture-kit は **TRACE を含む全レベル**を拾う（`crates/log-capture-kit/src/capture.rs` の説明）ので、`debug!` の読み飛ばしの記録も数えられる。 |
| `count_level(logs, "ERROR")` | 同上 | `level=ERROR` の含有で数える。 |
| 未装着の target への `Hide` の ERROR | `EmoPresenter` の `Hide` 適用（`crates/areka-emo-present/src/presenter/hub.rs`） | `error!(?target_id, "apply(Hide): 未装着ターゲット")`。`reply: None` でも出る。適用されたことを見る印になる。 |
| `wiring.attached = true` | `Emo2Wiring` の `attached` 欄（`frame/wiring.rs`・`pub(super)`） | `frame` の子モジュールから書ける（`frame_drain_text_tests.rs` の `run_drain_phase_gates_on_attach_then_drains_all_in_fifo_order` が同じことをしている）。 |
| 受信端の残件 | `Emo2Wiring::drain_received`（`frame/wiring.rs`・`#[cfg(test)]`・`pub(crate)`）、または `rx` 欄（`pub(super)`） | どちらも既存。補助ファイルに足さずに数えられる。 |
| `wintf::AppExit::new()`・`request_exit()` | `crates/wintf/src/runtime/message_loop.rs`（公開） | `Rc` を持つので `insert_non_send` で挿す。既存の `frame_ghost_quit_tests.rs` ほかと同じ。 |

`frame_drain_text_tests.rs` の `emo2_frame_system_removes_runs_and_reinserts_wiring` が、素の World に結線だけを挿して `emo2_frame_system` を丸ごと回しても落ちないことを既に示している。予定のテストは、これに `attached = true`・`AppExit`・`Hide` 1 件を足した形になる。

素の World で `attached = true` のまま全相を回したとき（対照側）の各相の振る舞いを読んだ:

- 装着: `attached` が立っていれば最初に戻る。
- 拡大率: `Changed<DPI>` の窓が無いので何もしない。
- 表示の指令: `Hide` 1 件を適用 → **ERROR 1 件**。
- バルーンの可視性: `balloon_models` が空なので対象 0 件。
- 窓寸の反映: target 0 件で何もしない。
- `\![move]`・重なり・再配置・確定・再解決: `GhostWindows` が無いので戻る。
- 文字層: `balloon_models` が空、`FrameTime` が無いので描画しない。
- **作業領域の同期: モニタが 0 台なので `warn!`（「モニタ表が空（列挙異常）」）が 1 件出る**（`sync_monitor_snapshot_with` の定義）。

→ 対照側は「ERROR 1 件」は成り立つが、**WARN は 0 件にならない**。要件 3.2 は対照側の WARN を問わないので要件とは矛盾しないが、対照側で WARN 0 件を確かめる書き方をすると赤になる。

### 2.6 読み飛ばした巡の「WARN 0 件」は満たせるか

- 判定そのものは `World::get_non_send::<AppExit>()` の読み取りと `debug!` 1 行だけで、WARN 以上を出す箇所は無い。
- 判定を `emo2_frame_system` の**最初**（`sync_monitor_snapshot` より前）に置けば、読み飛ばした巡で `emo2_frame_system` が出すログは読み飛ばしの記録 1 行だけになる。ERROR 0・WARN 0 は満たせる。
- 判定を `sync_monitor_snapshot` より後ろに置くと、素の World のテストで上の「モニタ表が空」の WARN が出て要件 3.1 が赤になる。本番ではモニタがあるので WARN は出ないが、同期の後に続く `resnap_for_work_area_change` は窓を書くので、判定を後ろにする理由も無い。
- `request_exit()` は `info!("[AppExit] exit requested")` を出すが、テストでは捕捉の外で呼べばよい（捕捉の中で呼んでも INFO なので件数の判定には効かない）。

→ **満たせる。ただし判定の位置は「全相より前」かつ「`sync_monitor_snapshot` より前」に限る。**

### 2.7 既存テストへの影響

- `emo2_frame_system` を呼ぶ既存テスト（`frame_drain_text_tests.rs`・`frame_text_scale_tests.rs`・`frame_transition_branch_tests.rs`・`frame_visibility_integration_tests.rs`）は、どれも World に `AppExit` を挿さない。判定は「受け口が無ければ未指示」（要件 1.7）なので、振る舞いは変わらない。
- `AppExit` を挿す既存テスト（`frame_ghost_quit_tests.rs`・`frame_ghost_quit_switch_tests.rs`・`frame_ghost_quit_logsink_tests.rs`・`frame_schedule_tests.rs`・`ghost_switch_test_support.rs`・`ghost_session_restart_tests.rs`・`spine.rs`）は、終了の指示の後に `emo2_frame_system` を回していない（`spine.rs` は相を個別に呼ぶ）。影響なし。
- `frame.rs` の本文を `include_str!` で読む構造の検査（`zorder_wiring_tests.rs`・`frame_work_area_sync_tests.rs`・`frame_work_area_resnap_tests.rs`・`frame_harness_tests.rs`）は、相どうしの前後と呼び出しの件数だけを見る。先頭に判定を足しても崩れない。
- `crates/areka/tests/smoke_boot_loop_exit.rs` は「結線が成立した」印の有無と終了コードを見る。終了の巡の相が止まっても変わらない。
- 受信端はどれも上限の無い `mpsc::channel`（`wire_emo2_boot` の定義で確認）で、本番で `reply: Some` を載せる指令も無い。受信端に指令を残したまま終えても、送り手が詰まったり返事を待ち続けたりすることは無い。

## 3. 要件と既存資産の対応

| 要件 | 既存資産 | 差 | 種別 |
|---|---|---|---|
| 1.1 終了後は全相を止める | `emo2_frame_system`（全相の唯一の入口） | 判定が無い | 欠落 |
| 1.2 `frame_phases_skipped_after_exit` を debug で 1 行 | 前例 `ghost_quit_no_windows`（`quit_as_today`）・`DESPAWNED_SKIP_TAG`（`despawn_app_windows`） | 記録が無い | 欠落 |
| 1.3 その巡で ERROR 0・WARN 0 | — | 判定を `sync_monitor_snapshot` より前に置くこと | 制約 |
| 1.4 結線を残す | `Emo2Wiring` は `remove_non_send`→各相→`insert_non_send` | 取り出す前に戻れば何もしなくて済む。取り出した後に戻ると戻し忘れの危険 | 制約 |
| 1.5 出所を問わない | `quit_app` の 10 か所・7 出所・`ExitPolicy::Explicit` | 判定が `AppExit` 1 つを見れば全出所に効く | 充足（前提） |
| 1.6 未指示なら今日どおり | — | 判定が偽なら何もしない | 欠落（判定と同時に満たす） |
| 1.7 受け口が無ければ未指示 | `get_non_send::<AppExit>()` は `Option` | `is_some_and` の形で満たす | 欠落（判定と同時に満たす） |
| 2.1 生きた窓の DPI 欠落は ERROR のまま | `derive_scale`（`crates/areka-emo-present/src/scale.rs`） | 触らない | 充足 |
| 2.2 後始末の順序を保つ | `main.rs` の終了統括 | 触らない | 充足 |
| 2.3 切替の途中は記録 0 行 | `close_windows_for_restart` は `AppExit` に触れない | 判定の形から自動的に満たす | 充足 |
| 2.4 終了の相の説明文を直す | `run_ghost_quit_phase` の説明文の「他の相を走らせても……」の一文 | 書き直しが要る | 欠落 |
| 3.1〜3.3 決定論テスト | `frame_test_support.rs` の 4 関数・`Emo2Wiring::drain_received` | 新しい兄弟のテストファイル 1 本 | 欠落 |
| 3.4 兄弟ファイル・1,000 行・補助を足さない | `frame_test_support.rs` は 900 行 | 足さずに書ける（2.5） | 制約 |
| 3.5 `-p log-capture-kit` も回す | `crates/log-capture-kit/tests/file_length_guard_test.rs` | 手順の約束 | 制約 |
| 4.1〜4.4 実機の確認 | emo2 検体・debug 版 | 実機の記録を残す | 欠落 |

## 4. 実装の選択肢

### 案 A: `emo2_frame_system` の先頭に判定を足す（既存を広げる）

- 変えるファイル: `crates/areka/src/emo2_boot/frame.rs`（判定と `debug!`・`run_ghost_quit_phase` の説明文の一文・新しいテストの接続宣言）と、新しい兄弟のテストファイル `crates/areka/src/emo2_boot/frame_exit_gate_tests.rs`。
- `frame.rs` は 509 行で、足しても十数行。1,000 行の上限に余裕がある。
- 判定の細かな置き場は次の 3 通りがある（設計の議題 1）:
  - A-1: 本文の最初の行（`Emo2Wiring` の有無を見る前）。結線の無い構成（LogSink の起動）でも、終了の巡に読み飛ばしの記録が出る。
  - A-2: `Emo2Wiring` の有無を取り出さずに確かめた後、取り出す前。記録は「相が走るはずだった巡」だけに出る。
  - A-3: 取り出した後。早く戻る前に戻し直す必要があり、戻し忘れると要件 1.4 に反する。利点は無い。
- 長所: 差分が最小。全相が 1 つの判定で止まる。既存の相の並びと既存テストに影響しない。
- 短所: `emo2_frame_system` が「アプリの終了」を知る。ただし相の並びの持ち主がここなので、場所としては自然。

### 案 B: 判定だけを別の小さな system に分け、`run_if` で `emo2_frame_system` を止める（新しい部品）

- bevy の実行条件（`run_if`）で `AppExit` が指示済みなら `emo2_frame_system` を実行しない形。登録は `register_emo2_frame_system`（`emo2_boot/mod.rs`）を変える。
- 長所: 相の本文に手を入れない。
- 短所:
  - 触らないファイルの一覧にある `emo2_boot/mod.rs` を変えることになる。
  - 実行条件で止めると `emo2_frame_system` の本体が呼ばれないので、読み飛ばしの記録を残す場所が別に要る（条件の関数の中でログを出すことになり、分かりにくい）。
  - テストが `emo2_frame_system` を直接呼ぶ形だと判定を通らない。schedule を組む形のテストが要り、既存の補助から外れる。
  - `AppExit` は `NonSend` なので、実行条件から読むには `Option<NonSend<AppExit>>` を使う形になり、既存の流儀（排他 system が `&mut World` から読む）から外れる。

### 案 C: 表示の出し手の側で「窓が無ければ読み飛ばす」（採らない案の再確認）

- brief の「採らない案」で理由まで確定済み。表示の適用しか止まらず、文字層・可視性・窓寸・`\![move]`・重なりは消えた窓を触り続ける。`crates/areka-emo-present/**` は境界の外。
- 同じく、`derive_scale` の ERROR を下げる案・終了の相で `Emo2Wiring` を取り除く案も brief で退けられている（生きた窓の本物の異常を消す・ほかの出所が止まらない）。本分析でもこれを覆す事実は見つからなかった。

### 比較

| 観点 | 案 A | 案 B | 案 C |
|---|---|---|---|
| 全相を止める | ○ | ○ | × |
| 触るファイル | `frame.rs`＋新テスト | `mod.rs`＋記録の置き場＋新テスト | 境界外 |
| 読み飛ばしの記録の置き場 | 判定と同じ所 | 実行条件の中（分かりにくい） | — |
| 既存の補助でテスト | ○ | △（schedule を組む） | — |
| brief・要件との整合 | ○ | △（触らない一覧に `mod.rs`） | × |

## 5. 規模とリスク

- **規模: S の下端（1 日未満・brief の見立てどおり XS）。** 本番の差分は `frame.rs` の十数行、テストは新規 1 本（見込み 100〜150 行）、それに実機の確認 1 回。
- **リスク: 低。** 既存の流儀（排他 system の早期の戻り・`debug!` での正常系の読み飛ばし）をなぞるだけで、依存も増えない。前提の「指示済み⇒窓は無い」は `quit_app` の構造と `ExitPolicy::Explicit` で保たれている。

## 6. 設計への申し送り

### 推奨

- 案 A。判定は `emo2_frame_system` の本文の**全相より前・`sync_monitor_snapshot` より前・`Emo2Wiring` を取り出す前**に置く（A-1 か A-2。どちらにするかは議題 1）。
- 判定の形は `world.get_non_send::<wintf::AppExit>().is_some_and(|e| e.is_requested())`（受け口が無ければ偽＝要件 1.7）。
- テストは `frame_drain_text_tests.rs` の `run_drain_phase_gates_on_attach_then_drains_all_in_fifo_order` と同じ組み立てで、`emo2_frame_system` を丸ごと回す。受信端の残件は既存の `Emo2Wiring::drain_received` で数える。読み飛ばしの記録の件数は、テストファイルの中の小さな絞り込み（`frame_text_scale_tests.rs` が自前の `count_level_containing` を持つのと同じやり方）で数え、補助ファイルには足さない。
- 対照側（未指示）では WARN の件数を確かめない。素の World ではモニタ表が空の WARN が 1 件出るため。

### 設計で決める項目

1. **判定の置き場の細部（A-1／A-2）。** A-1 は結線の無い構成でも終了の巡に記録が 1 行出る。A-2 は `Emo2Wiring` の有無を取り出さずに確かめてから判定する（相が走るはずだった巡だけ記録する）。どちらも要件は満たす。LogSink の起動のログに記録が出るのを良しとするかで決まる。
2. **読み飛ばしの記録に何を載せるか。** 要件は `event` の名前と debug の水準だけを決めている。最初の終了の出所（`FirstExit` 資源）を載せると「どの終了の後か」が 1 行で読める。受信端の残件は数えると取り出してしまうので載せられない（数えるだけの口は無い）。
3. **判定の位置を本文の並びでも確かめるか。** 挙動のテスト（要件 3.1〜3.3）は「判定が無い」「常に真」を捕まえる。「判定が `sync_monitor_snapshot` より後ろへずれた」は、素の World のモニタ表が空の WARN を通して 3.1 が偶然捕まえる形になる。既存の `include_str!` による並びの検査（`zorder_wiring_tests.rs` など）と同じやり方で「判定が `remove_non_send` と `sync_monitor_snapshot` より前」を明示的に確かめるかどうか。
4. **（要件の討議で解決済み・2026-09-28）要件 4.4 を「記録 1 件」を合格の条件とする形へ直した。以下は当時の指摘。** **要件 4.4 の但し書きと、判定の実際の出方の食い違い。** 要件 4.4 は「記録は終了の巡に指令が残っていた回にだけ出る」と書く。しかし判定は受信端を見ずに（見れば取り出してしまう）巡ごと読み飛ばすので、要件 1.2 のとおり作れば、メニューの終了では指令の有無に関わらず毎回 1 行出る（2.3）。記録が出ること自体は合否に使わない（4.4 の「出なくても不合格としない」）ので作業は変わらないが、起動前に告げる説明の文言（「指令が残っていた回にだけ出る」）は実際と合わない。要件の但し書きを直すか、実機の確認の手順書の側で言い換えるか。
5. **`emo2_frame_system` の説明文とモジュールの説明文にも判定を書くか。** 要件 2.4 が求めるのは `run_ghost_quit_phase` の説明文だけ。関数の説明文（相の並び）と `frame.rs` 冒頭のモジュールの説明（相の一覧）に「終了指示の後は全相を読み飛ばす」を書き足すかどうか。

### 調べ残し（実装時に確かめる）

- 対照側で ERROR がちょうど 1 件になること。上の読み（2.5）では `Hide` の 1 件だけだが、素の World で `attached = true` のまま全相を回すテストは今まで無いので、実装時に走らせて確かめる。
- 実機で読み飛ばしの記録が何行出るか。2.3 の見立てでは、メニューの終了は終了の相（`ghost_quit_system`）と毎フレームの処理が同じ `Update` の巡で続けて走るので、**指令が残っていたかどうかに関わらず毎回 1 行**出るはず。要件 4.4 のとおり出なくても不合格にはしない。
- 実機の確認は emo2 を絶対パスかつ短いパスで起動する（相対パスだと `pasta.dll` の読み込みで失敗する既知の罠）。`RUST_LOG` に `areka=debug` を含める。
