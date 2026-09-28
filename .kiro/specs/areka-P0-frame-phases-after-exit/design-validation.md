# 設計の検証: areka-P0-frame-phases-after-exit

> 検証日 2026-09-28。対象は本ブランチの `design.md`（`e151235f`）。
> 突き合わせた相手は設計の説明文ではなく**実際のソース**（main `10a8d724` から `crates/` の差分 **0 ファイル**。対照として `10a8d724` 自身のコミットは `crates/` に 33 ファイルの差分があり、数え方が働いていることを確かめた）。
> コードは「何の定義か」（関数名・型名・テスト名＋ファイル）で指す。
> 本検証は読みによる静的な確認であり、テストの実行は **0 回**（実行は実装の段の仕事）。

## 1. 要約

設計は「`emo2_frame_system` の入口に判定を 1 つ足す」だけの修正として釣り合いが取れており、設計が依っている事実はソースですべて確かめられた。食い違いは **0 件**、設計や実装の中身を変える必要のある重大な指摘は **0 件**。判定は **GO**。実装へ申し送る小さな注意が 2 件ある（どちらも作業の中身は変えない）。

## 2. ソースとの突き合わせ

### 2.1 設計が依る事実（すべて成立・不成立 0 件）

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| 毎フレームの相を呼ぶのは `emo2_frame_system` の本文だけ。並びは「結線を取り出す → 作業領域の同期 → 装着 → 拡大率 → …→ 結線を戻す」 | `crates/areka/src/emo2_boot/frame.rs` の `emo2_frame_system` | 成立 |
| 終了の相は毎フレームの相より前に、同じ `Update` に載る | `crates/areka/src/emo2_boot/mod.rs` の `wire_kanade_stop`（`ghost_quit_system.before(emo2_frame_system)`）。登録の字面は `frame_schedule_tests.rs` の `ghost_quit_system_without_the_frame_system_quits_on_one_notice` が固定済み | 成立 |
| 本番で終了を指示するのは `quit_app` だけ。窓を消してから指示する | `crates/areka/src/app_exit.rs` の `quit_app`（最初に `despawn_app_windows`、最後に `request_exit`）。areka の本番コードで `request_exit` を呼ぶ箇所は **1 か所**。`quit_app` の呼び手は 10 か所 | 成立 |
| wintf 側から終了が指示される経路は無い | `crates/areka/src/main.rs` の `WinApp::with_exit_policy(ExitPolicy::Explicit)`。wintf の `wire_shutdown_hook` は `OnLastWindowClose` のときだけ指示を仕込む | 成立 |
| `close_windows_for_restart` は終了を指示しない | `app_exit.rs` の同関数（`AppExit` に触れない）と、`app_exit_tests.rs` の `close_windows_for_restart_closes_all_windows_without_requesting_exit` | 成立 |
| `wintf::AppExit` の `new`・`request_exit`・`is_requested` は公開 | `crates/wintf/src/runtime/message_loop.rs` の `AppExit`。`wintf::AppExit` の名前で届く（`app_exit.rs` が同じ名前で使っている） | 成立 |
| 未登録の対象への `Hide` は ERROR を 1 件出す（`reply: None` でも） | `crates/areka-emo-present/src/presenter/hub.rs` の `apply_hide`（`tracing::error!`） | 成立 |
| 素の World では作業領域の同期が WARN を 1 件出す | `crates/areka/src/emo2_boot/frame/work_area_sync.rs` の `sync_monitor_snapshot_with`（モニタ 0 台の腕・`tracing` の `warn!`） | 成立 |
| 補助 4 つと `drain_received` は既存で、補助ファイルに足さずに書ける | `frame_test_support.rs` の `headless_wiring_with`・`zero_clock`・`capture_logs`・`count_level`（いずれも `pub(super)`）、`frame/wiring.rs` の `Emo2Wiring::drain_received`（試験用）と `attached` 欄（`pub(super)`） | 成立 |
| 行数 | `frame.rs` 509 行・`frame_test_support.rs` 900 行 | 成立 |
| `frame.rs` を `include_str!` で読む検査は 4 本。素の全文から字面を探すのは 2 本で、避ける字面は 4 つ | 2.3 節 | 成立（呼び名に小さなずれ 1 件・注意 1） |

### 2.2 個別に掘った 5 点

**(a) 予定のテストは `debug!` の読み飛ばしの記録を本当に観測できるか — できる。**
`capture_logs` は `log_capture_kit::capture_lines(LineFormat::LevelTargetFields, …)` を呼ぶ。捕捉の受け手（`crates/log-capture-kit/src/capture.rs` の `CaptureSubscriber`）は `enabled` が常に真で、TRACE を含む全水準を拾う。捕捉の窓は TRACE の対照イベントを自分で 1 件撃ち、拾えなければ panic するので、「記録 0 件」が捕捉の不調で静かに成り立つことは無い。行の形は `level=DEBUG target=… message=… event="…"` で、`frame_phases_skipped_after_exit` と `level=DEBUG` のどちらも字面で数えられる。`tracing` の水準を実行体の側で切り捨てる設定（`max_level_*`）はワークスペースの `Cargo.toml` に **0 件**。読み飛ばしの記録・`apply_hide` の ERROR・作業領域の同期の WARN はどれも `tracing` のマクロで、`log` クレート経由のものは **0 件**。捕捉した行から DEBUG を字面で数える前例も areka の中に在る（`frame/zorder_drain_tests.rs`・`move_cue_move_severity_log_tests.rs`）。
対照の側の「記録 0 件」も、同じ組み立てのテスト 1 が「1 件」を観測するので空振りにならない。

**(b) 「終了が指示済み」は素の World で作れるか — 作れる。**
`wintf::AppExit::new()` → `request_exit()` → `World::insert_non_send`。`request_exit` は旗を立て、INFO を 1 行出し、待ちの無い通知を撃つだけで、窓もメッセージループも要らない。同じ挿し方の前例は `frame_schedule_tests.rs`・`frame_ghost_quit_tests.rs`。INFO は捕捉の外で呼ぶ設計なので件数に混ざらない。

**(c) 入口で戻ることで、後続が当てにしていたものが欠けないか — 欠けない。**
- 終了の後始末（`main.rs` の `run()` より後）は `Emo2Wiring` を読まない（`main.rs` の中の言及は説明文の 1 か所だけ）。
- 入力の段が `Emo2Wiring` を読む箇所（`input_events/balloon.rs`・`input_events/mod.rs`・`input_events/user_break.rs`）は `get_non_send` で在るかどうかを見る。結線は取り除かれないので今日と同じ。
- 受信端に残した指令で送り手が詰まることは無い。受信端は上限の無い `mpsc::channel`。表示の指令に返事の口を載せる本番の送り手は **0 件**。
- 終了の指示の後に走る巡は、今日でも「同じ巡の残り」だけである。`WinApp::run` は待ちが戻った直後に `running` を下ろし、`run_async_tick` は下りていればフレームを回さない。読み飛ばすのは、もともと消えた窓しか相手にしていなかった 1 巡ぶんである。
- `emo2_frame_system` を終了の指示の後に呼ぶ既存のテストは **0 本**。丸ごと呼ぶ 4 ファイル（`frame_drain_text_tests.rs`・`frame_text_scale_tests.rs`・`frame_transition_branch_tests.rs`・`frame_visibility_integration_tests.rs`）は `AppExit` を挿さない（言及 0 件）。

**(d) 2 つの変異は予定の確認で赤になるか — なる。**

| 変異 | テスト 1 で外れる確認 |
|---|---|
| 判定を外す | ERROR 0 件（`Hide` が適用されて 1 件）・WARN 0 件（同期の 1 件）・記録 1 件（0 件）・受信端に 1 件（0 件）の 4 つ |
| 判定を作業領域の同期の後ろへ動かす | WARN 0 件（同期の 1 件）。結線を戻さずに戻る形なら「結線が World に残る」も外れる |

設計の表に無い変異も読みで確かめた。判定が常に真・「受け口が在るだけで真」の 2 つはテスト 2（未指示）が、「受け口が無いと真」はテスト 3（受け口なし）が赤にする。3 つの入力すべてに、外れる確認が少なくとも 1 つ在る。
対照の側は `attached = true` のまま全相を回す。`attached` を見る相は装着（立っていれば最初に戻る）と表示の指令の取り出しだけなので、未装着のまま全相を回して落ちないことを示している既存のテスト（`emo2_frame_system_removes_runs_and_reinserts_wiring`）との差は「`Hide` が適用される」の 1 点に限られる。ほかの相が出す記録に左右されないよう、`apply(Hide)` を含む ERROR だけを数える設計（D6）は妥当。

**(e) 要件の対応 — 20 個すべてに具体的な受け手が在る。抜け 0 件。**

| 要件 | 受け手 |
|---|---|
| 1.1・1.3・1.4 | 入口の判定（D1）＋テスト 1 |
| 1.2 | 読み飛ばしの記録（D2）＋テスト 1（件数と `level=DEBUG`） |
| 1.5 | 判定が `AppExit` だけを読むこと＋`quit_app` の構造（2.1 節で確認） |
| 1.6 | テスト 2 |
| 1.7 | 判定の式（D5）＋テスト 3 |
| 2.1・2.2 | 触らないファイルの一覧 |
| 2.3 | 判定の式＋既存のテスト `close_windows_for_restart_closes_all_windows_without_requesting_exit` |
| 2.4 | 説明文の書き直し（D4） |
| 3.1・3.2 | テスト 1・テスト 2 |
| 3.3 | 変異の確認（`signoff.md` に記録） |
| 3.4 | File Structure Plan |
| 3.5 | 実行の手順 2（`crates/log-capture-kit/tests/file_length_guard_test.rs` の実在を確認） |
| 4.1〜4.4 | 実機の確認の表（6 行） |

### 2.3 `frame.rs` を読む 4 本の検査

| ファイル・テスト | 見ているもの | 本件の影響 |
|---|---|---|
| `frame_work_area_sync_tests.rs` の `the_sync_is_called_before_the_scale_phase_in_the_frame_system` | 素の全文で `work_area_sync::sync_monitor_snapshot(world)` と `run_dpi_phase(&mut wiring, world)` が最初に現れる位置の前後 | 新しい説明文にこの字面を書かなければ無し |
| `frame_work_area_resnap_tests.rs` の `the_resnap_is_called_after_the_scale_phase_in_the_frame_system` | 素の全文で `run_dpi_phase(&mut wiring, world)`・`work_area_sync::resnap_for_work_area_change(`・`reconcile_reported_sizes(&mut wiring.presenter, world)` の前後 | 同上 |
| `zorder_wiring_tests.rs` の `t_zwi07_the_frame_calls_the_zorder_drain_right_after_the_move_drain` | 説明文を落とした本文での呼び出しの件数と前後、`pub fn emo2_frame_system(world: &mut World) {` の行、説明文の「donor パターン: remove→各フェーズ→insert」 | どちらの字面も変えない設計なので無し |
| `frame_harness_tests.rs` の `the_harness_tests_are_connected_under_an_x64_only_gate` | `#[path = "frame_harness_tests.rs"]` の直前が x64 限定の属性であること | 無し（注意 1） |

設計が挙げた「書かない字面 4 つ」は、上の 2 本が探す字面の全部と一致する。漏れは **0 件**。

## 3. 重大な指摘

**0 件。**

設計・実装・テストの中身を変える必要のある問題は見つからなかった。

### 実装への申し送り（作業の中身は変えない・2 件）

1. **`frame_harness_tests.rs` の検査は「相の並び」ではなく「接続宣言の直前の属性」を見ている。** 設計は 4 本をまとめて「並びの検査」と呼ぶが、この 1 本だけ見るものが違う。新しい接続宣言（`exit_gate_tests`）は設計どおり `drain_text_tests` の並びに置けば触れない。x64 限定の属性と `#[path = "frame_harness_tests.rs"]` の間には何も挟まないこと。
2. **対照の側は、実装時に 1 度、捕捉した行を全部見ておくこと。** `attached = true` のまま素の World で全相を回すテストは今まで **0 本**である。読みでは `apply(Hide)` の ERROR 1 件と同期の WARN 1 件だけのはずだが、設計は件数を `apply(Hide)` に絞っているので、ほかの行が出ても緑のままになる。想定外の行が出ていたら `signoff.md` に書き留める（本件の合否には使わない）。

## 4. 設計の良い点

1. **直す場所が根本の 1 か所に絞れている。** 終了の出所 7 種類・呼び手 10 か所を個別に手当てせず、全出所が必ず通る `quit_app` の順序（窓を消してから指示する）を前提に、相の並びの持ち主の入口で 1 度だけ判定する。前提が崩れる変更は Revalidation Triggers に挙がっている。1 巡遅らせる解も採っていない。
2. **判定の位置を、要件そのものの確認で固定している。** 「WARN 0 件」は要件 1.3 の確認であり、同時に「作業領域の同期が走らなかった」ことの観測にもなる。字面の検査を別に足さず、素の World の性質に依ることは引き受けたうえで、確かめ直す条件に書いてある。実機の側も「ERROR 0 件」だけでは判定が働いたか分からないことを認め、読み飛ばしの記録 1 件を証拠に据えている。

## 5. 判定

**GO**

- 理由: 設計が依る事実はソースですべて成立し（不成立 0 件）、要件 20 個に受け手が在り（抜け 0 件）、予定の確認は 2 つの変異と 3 つの入力の取り違えを赤にできる。新しい型・関数・補助・依存は 0 件で、修正の大きさと釣り合っている。
- 残る不確かさ: 本検証はテストを実行していない。対照の側の実際の記録と、実機での読み飛ばしの記録の件数（メニューの終了で 1 件）は、実装と実機の確認で確かめる。メニューの終了では、終了の指示による起床が次の画面更新の起床より先に積まれるので、指示の後に走る巡は 1 つという見立ては `run_async_tick` と `WinApp::run` の作りと合う。
- 次の手順: 設計の討議（重大な指摘は 0 件・申し送り 2 件）→ `/kiro-spec-tasks areka-P0-frame-phases-after-exit`。
