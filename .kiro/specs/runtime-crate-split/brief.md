# Brief: runtime-crate-split

> 2026-10-28 `/kiro-discovery` で起票。開発者「areka 本体 crate (crates/areka) の肥大を解消するため、`placement` と `emo2_boot` を独立 crate へ分離したい」。6 本のうちの 3 本目。分け方と共通の約束は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測で、`runtime-module-untangle` の後の形は見込み＝着手のときに測り直す。

## Problem

`runtime-module-untangle` で、本体の中の `runtime` モジュールは本体の外の部分を参照しなくなる。それでもまだ本体の中にあるので、本体と一緒に組み直され、テストも本体の 1 つの実行ファイルに入ったままになる。

## Current State

- `runtime-module-untangle` の後の `crates/areka/src/runtime/` の見込みは、本番 約 12,000 行・テスト 約 45,000 行。前の spec で session と install・update・readme へ移す 10 ファイル（本番 約 3,300 行）を除いた数。
- 外の crate の使われ方（今の emo2_boot・本番 ファイル/回）は次のとおり。
  - tracing 37/102・bevy_ecs 24/42・wintf 21/32・dola 13/34
  - areka-emo-present 15/23・areka-ghost 11/23・areka-sakura 18/21・areka-parsers 12/16
  - areka-emo-text 11/13・areka-emo-compose 10/12・areka-seriko 10/11・areka-kanade 10/10
  - areka-actor 5/6・areka-emo-atlas 4/4・areka-sylphya 4/4・thiserror 4/4・windows 2/4
  - areka-mcp・areka-nar・areka-user-data・areka-talk は 0。
  - テストだけで使うのは log-capture-kit・temp-path-kit・sample-ghost-kit・areka-update（1 ファイル）・shiori-host32-host（`spine` の `ScriptedShioriBackend`）。
- **`target_map`**（scope から表示対象 `TargetId` への写像の正本・std と `areka-emo-present::TargetId`・`areka-sakura::ActorKey` だけを使う）の使い手は次のとおり。
  - 本番: emo2_boot の中 15 ファイル、input の `user_break.rs:22`、mcp の `check_script.rs`・`dump_balloon.rs`・`dump_surface.rs`。
  - テスト: mcp の GPU のテスト 2 本、placement の `transition_judge_tests`、`shell_balloon_switch_session_*_tests` 3 本。
- **crate の外のテストが使う支え**（今の数）は次のとおり。
  - `ghost_switch_test_support`: 34 ファイル・45 回。前の spec で session へ移る。
  - `spine`: 28 ファイル・46 回。`mcp/dump_surface.rs:415` は本番ファイルの中の `#[cfg(test)]` から `spin_wait_until` を呼ぶ。
  - `spine::hold_support`: 2。`sample_test_support`: 2 ファイル・8 回。
  - 逆向きに emo2_boot のテストが外の支えを使う所もある。`placement::test_support` 24 回・`shared_test_support` 2・`scope_bind_fixture_test_support` 2・`install::fetch_url_test_support` 1。
- **本体に残る `transition_judge*`**（`placement-crate-split` で placement から本体のテスト専用へ移した判定器）は、emo2_boot のテスト 2 本と自分のテストが使う。
- **境をまたぐテスト**: placement の `reset_balloon_pos_tests.rs` は `switch_assets` を使う。`placement-crate-split` で本体に残したもの。
- **examples**: `collision-probe.rs` が `#[path]` で `target_map.rs`・`hit_region.rs` を取り込んでいる。
- **ログの出所**: `runtime-module-untangle` の後は `areka::runtime::…`。移すと `areka_runtime::…` に変わる。
- **`runtime-module-untangle` からの申し送り**（2026-11-13・パスはその着地の形。数は同 spec の `research.md` の検証の記録）:
  - **落ちの報告の支え**: `crates/areka/src/runtime/crash_report_test_support.rs`（宣言は `runtime/mod.rs` の `mod crash_report_test_support;`・使い手は `runtime/spine_wait.rs`）は、テストの実行ファイルごとに `.CRT$XCU` の初期化子で据える。実行ファイルが本体と `areka-runtime` に分かれると、**両方に要る**（本体の側には今は無い＝runtime と一緒に移すだけだと、本体のテストの実行ファイルが落ちの報告を失う）。前例は `crates/areka-placement/src/test_crash_report.rs`（1 行目の「写しの相手」の行を、置いた先のパスへ合わせる。`crates/wintf/src/test_crash_report.rs` の 1 行目も）。
  - **テストが本体の外を引く所**（crate を分けると引けなくなる）:
    - 口のテストの倒れ先: `runtime/session_hooks.rs` の `#[cfg(test)] const FALLBACK: SessionHooks = crate::session::hooks::HOOKS;`（`runtime-module-untangle` 設計の議題 1＝テストの World は本体の口そのものを読む）。本番の倒れ先は `NO_BODY`。`areka-runtime` のテストからは `session` が見えないので、テストの倒れ先を決め直す。
    - 文字列で読むファイル: `runtime/frame_schedule_tests.rs` の `include_str!("../session/ghost_quit.rs")`・`runtime/zorder_wiring_tests.rs` の `include_str!("../session/runtime_boot.rs")`・`"../ghost_session.rs"`・`"../main.rs"`・`runtime/strict_judges_tests.rs` の `include_str!("../../../../doc/ssp-mcp/areka-tools.md")`（相対の深さが変わる）・`runtime/film_playback_e2e_tests.rs` の `crate::crate_source_test_support::crate_path`（支えの宣言は本体の `main.rs`）。
    - `transition_judge`: `runtime/frame_transition_atomicity_tests.rs`・`runtime/frame_transition_branch_tests.rs` が `crate::placement_tests::transition_judge` を引く（上の「本体に残る `transition_judge*`」）。
    - 骨組みの起動の選択肢: `runtime/spine.rs` が `crate::session::runtime_boot::runtime_boot_options` を呼ぶ（テストの支えの骨組みが実行系を起こすとき）。
    - 逆向きの e2e: `session/runtime_boot_tests.rs` が `#[path = "../runtime/script_security_e2e_tests.rs"]`・`#[path = "../runtime/readme_script_e2e_tests.rs"]` で 2 本を session のテストとして宣言している（ファイルは `runtime/` に残した）。crate を分けるときは置き場（本体の側へ寄せるか）を決める。

## Desired Outcome

- `crates/areka-runtime/`（lib・`publish = false`）が runtime の本番とテストを持ち、`areka-placement` に依存する。`crates/areka` は path 依存でそれを使う。
- `target_map` と、テスト専用の `transition_judge*` が `areka-runtime` にある。
- 本体の中で runtime を名指しする道は、全部 `areka_runtime::` になっている。
- 振る舞いは何も変わらない（「検証」）。

## Approach

- 新しい crate を作り、`git mv` で履歴を残して移す。`runtime/mod.rs` の中身は `lib.rs` になる。外から使われる `pub(crate)` だけを `pub` にする。
  - 主な面: `RuntimeWiring` とその `pub(crate) fn` 約 10 本（`runtime()`・`presenter()` ほか）・`frame` の `ZOrderAbsentReports`・`run_ghost_quit_phase`・`TalkLifecycleSignal`・`NoUserBreakSignal`・`HitRegion`・`resolve_hit_region`・`ConsumerLedger`。
  - 正確な一覧は着手のときに測る。
- **テストの支え**: `spine` などの外のテストが使う支えは、`areka-runtime` の `test-support` 機能で開ける。`placement-crate-split` と同じやり方にそろえ、写しを作らない。
- **境をまたぐテスト**: `reset_balloon_pos_tests.rs`（placement と runtime を使う）と `transition_judge*` の判定器とそのテストを、`areka-runtime` へ移す。
- **examples**: `collision-probe.rs` は `areka-runtime` の examples へ移し、`#[path]` をやめて crate の依存で読む。
- **ログの出所**: `log_history.rs` の決まり、`RUST_LOG` の文字列、`doc/ssp-mcp/log-convention.md`、実機の手順書の `areka::runtime` を `areka_runtime` に直す。

## Scope

- **In**: crate の新設と移動・公開面・本体の参照の書き換え・テストと examples と支えの移し替え・`test-support` 機能・ソースを文字列で読むテストのパス・1,000 行の番人の表（emo2_boot の 1 件ほか）・ログの出所・文書のパス（`doc/COMPAT_ARCHITECTURE.md` 38 件・`doc/ukadoc-coverage/ledger/*.toml` の根拠の場所 約 54 件ほか）・`structure.md` の新しい crate の節・`audit-emo2-boot-*` 3 本の「受け持つファイル」の書き換え。
- **Out**: 名前や依存の付け替え（前の spec で済んでいること。残っていたら前の spec の抜けとして直す）・input と session（後の 3 本）。

## Boundary Candidates

- crate の骨組みと移動。
- 公開面とテストの支え。
- `target_map`・`transition_judge*`・境をまたぐテスト・examples。
- ログの出所と文書。
- 検証。

## Out of Boundary

- 振る舞いの変更。中身の直しは audit の 3 本へ申し送る。

## Upstream / Downstream

- **Upstream**: `runtime-module-untangle`。
- **Downstream**: `input-crate-split`。`audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` はこの spec の着地を待つ。v0.2.0 の作り直しで runtime に触る spec（`dola-interrupt-ticks` → `talk-anchor-interrupt-ticks` → `instant-to-interrupt-ticks` ほか）も、新しい crate の上で行う。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: audit の 3 本（受け持つファイル）。emo2_boot に触ると書いている約 70 本は、roadmap の新旧のパスの対応表で読み替える。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、そのまま要件に入れる。この spec で回す実機テストは、出所を名前で読む次の 2 つを 1 回ずつ。
  - `tests/smoke_boot_loop_exit.rs`
  - `tests/emo2_real_run`

  `log_history` の分け方（MCP の `get_log` の 5 種別）が前後で同じことも示す。
- **共通の約束**: roadmap の同じ節のとおり。
- **Fable**: Opus で足りる。

## 2026-11-14 棚卸㊲の申し送り

- 完了した `runtime-module-untangle` の research の §24 は「本体の外のクレートの説明の中に残る古い名前（`emo2_boot`・`Emo2Wiring`）は `runtime-crate-split` が直す」と書いたが、この brief には書かれていなかった。2026-11-14 の main `df6f4e03` で、本体（`crates/areka/`）の外の 43 ファイル・76 行に残る（数え方: `rg -n 'emo2_boot|Emo2Wiring' crates -g '!crates/areka/**'`・例 `areka-emo-present/src/presenter/` の `read.rs`・`refresh.rs`・`target.rs`、`areka-emo-text/src/actor_attach.rs`・`state_decoration.rs`、`areka-ghost/src/prop_sink.rs`）。説明だけの直しで動きは変わらない。`areka-placement` の 4 行は棚卸㊲で直した。

## 2026-11-14 `audit-placement-follow` の申し送り（要件 2.9・7.9）

- **旧い名前は直した**: `areka-placement` の追従（`follow*`）と移り変わりの記録（`transition_diag*`）の受け持ちの 9 ファイル（`follow.rs`・`follow/window_move.rs`・`follow/work_area.rs`・`follow/move_shift.rs`・`follow/offset_space.rs`・`follow_transition_diag_tests.rs`・`follow_visibility_balloon_wiring_tests.rs`・`follow_balloon_limit_tests.rs`・`follow_default_pos_track_tests.rs`）の説明にあった `emo2_boot` 21 か所は、今の `runtime::…` の場所へ直した（受け持ちの 58 ファイルで `rg -n 'emo2_boot|Emo2Wiring'` が 0 件）。同じ crate の範囲の外の 13 ファイルに残る分は `audit-placement-core` の持ち物（`audit-placement-core` へ伝える一覧に載せた）。
- **狭めた公開**: `VisibilityVerdict`・`guard_visibility` は使い手がツリー全体（`vendors/` を除く）で 0 だったので `pub(crate)` にし、`follow.rs` の再公開を消した。`ChainPlacement`・`ChainClampOutcome` は名で呼べるよう `pub use` にした。
- **テストだけが使う公開（この spec では狭めない・判定器を `areka-runtime` へ移すときにまとめて扱う）**: 範囲の外の使い手が本体（`crates/areka/`）のテストだけの項目。狭めると本体のテストが壊れるので、テスト用の feature へ移すか、テストを crate の中へ移すかを、この spec の設計で決める。
  - `on_balloon_drag`（`follow/drag_follow.rs`）・`sync_drag_writer`（`follow/sticky_follow.rs`）
  - `WORK_AREA_CLAMP_TAG`（`follow/work_area_clamp.rs`）
  - `AnchorSource`（`follow/anchor.rs`）・`WorkAreaResolution`・`work_area_for_window_with_origin`（`follow/work_area.rs`）
  - `move_window_with_route`（`follow/window_move.rs`）
  - `MonitorSources::is_empty`（`follow/work_area.rs`）
  - `MonitorDpiEntry`（公開の欄 `MonitorDpiTable.entries` の要素の型。本体のテスト `main_monitor_snapshot_seam_tests.rs` が欄を読む。型だけを下げると公開の形が変わるので狭めなかった）
  - `transition_diag.rs` の語・欄・行の組み立ての大半（本体のテスト専用の判定器 `placement_tests/transition_*` が読む）
- **範囲の外の古い参照（本体の側・直していない）**:
  - `crates/areka/src/runtime/frame_transition_atomicity_tests.rs:57,150` の `transition_judge.rs:609-613` は、今は `summarize` の `KIND_WRITE` の腕（`path_a_writes` を数える所）。行の番号でなく名で指すのがよい。
  - 古い「唯一の位置ライター」: `crates/areka/src/runtime/frame/drain_resnap.rs`（`move_window_to` の呼び出しの上）・`crates/areka/src/runtime/frame_chain_finalize_tests.rs`（明示的な再配置の行）。今の `move_window_to` は位置の書き手の 1 つであって唯一ではない。
  - `crates/areka/src/runtime/frame_work_area_resnap_hold_tests.rs` の `the_invariant_watch_still_fires_for_a_write_that_bypasses_the_sites` は `debug_assert!` に依る `should_panic` で、`cargo test --release` では赤になる。`#[cfg_attr(not(debug_assertions), ignore = "debug_assert! に依る")]` を付けるのがよい。
  - `crates/areka/src/runtime/frame/move_shift.rs`（毎フレームの相）の注「当てられなかったとき（材料が無い）は口の中で warn! 済み」は、`audit-placement-follow` の audit のかけ直しで足した見送り（ドラッグ中の窓・組の相手・持ち主のバルーンがドラッグ中のキャラ窓は `debug!` で見送り、記録を変えずに後で当て直す）を書いていない。戻り値は捨てているので振る舞いに害は無い。注を事実に合わせるのがよい。
