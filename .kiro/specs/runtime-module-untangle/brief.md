# Brief: runtime-module-untangle

> 2026-10-28 `/kiro-discovery` で起票。開発者「`emo2_boot` / `Emo2Wiring` / `Emo2BootInputs` / `wire_emo2_boot` は特定ゴースト（emo2）に依存せず、実体は汎用の実行基盤（ゴースト・シェル・バルーン切替、reload、バルーン可視性、各 cue 受け口、talk 時計）。名前は最初の統合 spec（areka-P0-emo2-boot）の名残。spec B で crate 名・型名のリネーム方針（例: areka-runtime 系）を要件として決め、emo2 固有の記述は既定ゴースト名（boot_resolve の DEFAULT_GHOST_FOLDER）とテスト fixture に限定されることを実測で確認すること」。6 本のうちの 2 本目。分け方と共通の約束は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測（非空行）＝着手のときに測り直す。

## Problem

`emo2_boot` は、エンジン（kanade・sakura・seriko・emo・dola）と窓（wintf・placement）を毎フレームつなぐ、本体の背骨になっている。困りごとは 2 つある。

- 名前が特定のゴーストを指しているように読める。
- 本体のほかの部分と参照し合っている。

参照し合いが残ったままでは crate に出せない。この spec は crate を作らず、本体の中で次の 3 つを済ませる。次の `runtime-crate-split` は、移すだけで済む。

- 名前を改める。
- 外へ出られない部分を外へ移す。
- 残る部分が本体の外の部分を参照しない形にする。

## Current State

- **大きさ**: 本番 51 ファイル 約 15,279 行（直下 40・`frame/` 11。本番ファイルの中のテスト 1,025 行を除いた数）。テスト約 54,905 行。`*_cue.rs` は 9 本（change・install・move・readme・reload・switch・update・user_break・zorder）。
- **外への依存**: 51 ファイルのうち 37 ファイル（10,761 行）は placement しか参照しない。残る 14 ファイル（5,543 行）が本体のほかの部分を参照する。
  - 本体の外へ移せるもの: `ghost_switch`・`reload`（+`reload_args`）・`shell_balloon_switch`（+`shell_balloon_resolve`）・`runtime_boot`・`switch_copy`・`same_folder_pin`・`shell_carry`・`install_cue`・`update_cue`・`readme_cue`。
  - 背骨に残るもの: `frame.rs`・`frame/switch.rs`・`frame/status_report.rs`・`balloon_visibility_phase.rs`。
  - 参照先: placement 75 回/20 ファイル（`placement-crate-split` の後は下の crate への依存）、`ghost_session`（`GhostSlot`・`GhostSession`・起動と窓の操作 6 種）、`boot_resolve`、`boot_config`、`app_exit`（`quit_app`・`ExitOrigin`・`close_windows_for_restart`・`WindowsClosed`）、`exit_wait`、`write_failure`、`readonly`、`session_end`、`log_history`、`main.rs`（`record_last_used`・`is_benign_boot_error`）、install・update・readme・roots・input_events。
- **参照し合いの組**: ghost_session・app_exit・session_end・boot_config・install・update・readme・input_events の 8 つ。
  - input_events と引き合うのは 3 か所。
    - `balloon_visibility_phase` が `BalloonWiring` のホバーの集合と `shell_box::{ShellBoxHover, settle_box_hover}` を引く。
    - `runtime_boot` が `wire_user_break` を引く。
  - `frame/switch.rs` は `GhostSlot` のほかに、`boot_config::BootContext` と `boot_resolve::{BalloonDecision, BalloonRoute, record_last_balloon, record_last_shell}` を引く。
  - `frame/status_report.rs`・`balloon_visibility_phase.rs`・`shell_carry.rs` は、`GhostSlot` から `seriko_sink()` と `kanade()` を取り出すだけ。
- **外から emo2_boot への参照**: src 直下の本番 6 ファイル/16 回（`ghost_session` 10）、mcp 5/11、input_events 7/11、install 2/3、menu 3/3、update 2/2、readme 1/2、offline_dump 1/1。
- **名前**:
  - 名前に emo2 を含む本番の識別子は 7 つ。どれも名残。

    | 識別子 | 定義の場所 | 使われ方 |
    |---|---|---|
    | `Emo2Wiring` | `frame/wiring.rs:55` | 本番 34 ファイル/131 回・テスト 41/164 |
    | `Emo2BootInputs` | `mod.rs:273` | |
    | `Emo2BootOutcome` | `mod.rs:246` | |
    | `wire_emo2_boot` | `mod.rs:414` | コードで呼ぶのは `ghost_session.rs:855` とテストだけ |
    | `emo2_boot` | モジュール | |
    | `emo2_frame_system` | `frame.rs:284` | |
    | `register_emo2_frame_system` | `mod.rs:689` | |

  - ログの文言で emo2 を含む本番の行は 26。
    - emo2_boot が 21（`emo2-boot:`・`emo2 attach:`・`emo2 text:`）。
    - `ghost_session` が 2。
    - input_events のイベントの値が 3（`choice_{leave,moved,pressed}_no_emo2`）。
  - 文言や出所を照合している所。
    - `tests/smoke_boot_loop_exit.rs:51`・`tests/emo2_real_run/common.rs:17-18`
    - `balloon_pointer_handler_tests` 4・`balloon_leave_tests` 1
    - `log_history.rs:127`（本番の決まり）・`log_history_tests` 4
    - `assets_tests:458`・`balloon_background_tests:246`・`reload_skin_tests:327`・`reload_tests:580`・`tests/emo2_real_run/common_tests.rs:28`
    - `doc/ssp-mcp/log-convention.md:55,134`
    - `tools/*.ps1` で照合しているものは 0。
- **emo2 だけに向けた動き**: 本番では、既定のゴーストかどうかの判定（`ghost_switch.rs:424,637,697,714`）だけ。値は `boot_resolve.rs:32` の `DEFAULT_GHOST_FOLDER = "emo2"`。`derive_scopes()` の `vec![0,1]` は汎用の形（sakura と kero）。emo2 専用の面の番号や名前の定数は無い。
- **テスト側の名前**: emo2 を含む定義は 158。
  - 結線の名残: `headless_emo2_wiring` など。
  - 実在のゴースト emo2 を指すもの: `emo2_root`・`acquire_emo2`・`Emo2Absent`・`EMO2_ADJUST`・`AREKA_EMO2_REAL_RUN`・`tests/emo2_real_run*`・`emo2_first_load_repeat` など。
- **プロセスの中で共有する状態**（本番）: `balloon_visibility_wait.rs:72` の `OnceLock`・`hover_inject.rs:187` の `OnceLock`・`frame/switch.rs:57` の `AtomicU64`。

## Desired Outcome

- `crates/areka/src/emo2_boot/` が `crates/areka/src/runtime/` になり、本番コードが `runtime` の外の本体を参照する数が **0**。`crate::`（`crate::runtime::` を除く）と、外へ抜ける `super::super` の両方で数える。参照してよいのは、`areka-placement` などの下の crate だけ。
- 切替・読み直し・実行系の起動は、本体の `crates/areka/src/session/`（新しいモジュール）に集まっている。cue の受け口のうち install・update・readme の 3 本は、それぞれの持ち主のモジュールにある。
- 名前に emo2 が残るのは、既定のゴースト名とその使い手、実在のゴースト emo2 を指す検体だけ。それを grep の一覧で示す。
- 振る舞いは何も変わらない（「検証」）。

## Approach

- **名前**（要件の段で確定・推奨）:

  | 今 | 改めた後 |
  |---|---|
  | `emo2_boot`（モジュール） | `runtime` |
  | `Emo2Wiring` | `RuntimeWiring` |
  | `Emo2BootInputs`・`Emo2BootOutcome` | `RuntimeInputs`・`RuntimeOutcome` |
  | `wire_emo2_boot` | `wire_runtime` |
  | `register_emo2_frame_system`・`emo2_frame_system` | `register_frame_system`・`frame_system` |
  | ログの `emo2-boot:`・`emo2 attach:`・`emo2 text:` | `emo2` の語を外す（例 `runtime:`・`runtime attach:`・`runtime text:`） |
  | イベントの値 `choice_*_no_emo2` | `choice_*_no_runtime` |
  | テストの `headless_emo2_wiring` など結線を指す名前 | `runtime` の語へ |

  `BootWiringError` のように emo2 を含まない名前は変えない。実在のゴースト emo2 を指す名前は変えない。crate の名前は `areka-runtime`（次の spec で作る）。runtime の中の `runtime_boot.rs` は session へ移るので、語が二重になる所は残らない。
- **外へ移す**:
  - `ghost_switch`・`reload`（+`reload_args`）・`shell_balloon_switch`（+`shell_balloon_resolve`）・`runtime_boot`・`switch_copy`・`same_folder_pin`・`shell_carry` と各テストを、`src/session/` へ移す。
  - `install_cue`・`update_cue`・`readme_cue` を、`install/`・`update/`・`readme/` へ移す。cue の受け口の型（`*_cue.rs` の同型・`consumer_ledger.rs` の表の行）は変えない。
- **残る背骨の参照を切る**（6 か所・切り方は設計で決める）。
  1. `frame.rs` の `app_exit::quit_app`: 知らせで向きを逆にする。終わる段取りの順は変えない。
  2. `frame/switch.rs` の `BootContext`・`boot_resolve` の記憶を書く関数・`GhostSession`: 向きを逆にするか、本体に頼る部分を session へ移す。
  3. `frame/status_report.rs`・`balloon_visibility_phase.rs` の `GhostSlot`: 中身（`seriko_sink()`・`kanade()`）を、session が runtime 側の資源に据える形にする。
  4. `balloon_visibility_phase.rs` の input の `BalloonWiring` のホバーの集合: 別の NonSend の型に分けて runtime に置く。`BalloonWiring` ごと下ろすと `Sender<ChoiceSelection>` まで連れて行くため、集合だけを分ける。
  5. `ShellBoxHover`・`settle_box_hover`（純粋な関数）: runtime へ下ろす。
  6. `wire_user_break` の呼び出し: `runtime_boot` と一緒に session へ移る。runtime から session を参照する所が出たら、それも切る。
- **完了の目印**: `crates/areka/src/runtime/` の本番で、外の本体を指す参照が 0 件であることを grep で示す。テストの参照は次の spec で決めるので、ここでは数えるだけにする。次の spec が使う「crate の外のテストが使う支え」の一覧（`spine`・`frame_test_support`・`sample_test_support` など、使っている所と回数）を、設計か研究の記録に残す。

## Scope

- **In**: モジュールと型と関数の改名・ログの文言とイベントの値の改名と、それを照合する所の直し・10 ファイルの移し先への移動・背骨の 6 か所の参照の切り離し・文書と steering の名前とパスの書き換え・`audit-emo2-boot-*` 3 本の brief の「受け持つファイル」の書き換え（`src/runtime/` と `src/session/` へ）・emo2 の残りの一覧。
- **Out**: crate の新設（`runtime-crate-split`）・input と session の crate 化（後の 3 本）・背骨の中身の直し（audit へ申し送る）・`GhostSlot` を複数のゴースト向けに作り直すこと（`multi-ghost-runtime`）。

## Boundary Candidates

- 改名（名前だけ・文言と照合する所まで）。
- 外へ移す 10 ファイル。
- 背骨の参照の切り離し 6 か所。
- 文書と steering。
- 検証。

## Out of Boundary

- 振る舞いの変更。終わり方・切替の段取りの順・フレームの中の順は変えない。
- 既定のゴースト名 `"emo2"` を変えること（今の既定のゴーストの名前そのもの）。

## Upstream / Downstream

- **Upstream**: `placement-crate-split`（emo2_boot の 84 ファイルの `crate::placement::` を書き換えるので、同じファイルに触る＝直列）。
- **Downstream**: `runtime-crate-split`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch`: 受け持つファイルのパスを書き換える。着手は `runtime-crate-split` の後。
  - emo2_boot に触ると書いている約 70 本: brief は書き換えず、roadmap の新旧のパスの対応表で読み替える。特に `consumer_ledger.rs` に行を足す spec の列。
  - `multi-ghost-runtime`: `GhostSlot` と止まる流れを作り直す。同じウェーブに置かない。どちらが先でも、後の方が新しい形で測り直す。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、そのまま要件に入れる。この spec で回す実機テストは、ログの文言や出所を名前で読む次の 2 つを 1 回ずつ。
  - `tests/smoke_boot_loop_exit.rs`
  - `tests/emo2_real_run`（`AREKA_EMO2_REAL_RUN`）

  `quit_app` の向きを逆にするので、終わる道（メニューの終了・窓を閉じる・煙の自動終了 `AREKA_APP_SMOKE_EXIT_MS`）の記録の順が前後で同じことも、実機の記録で示す。
- **フレームを遅らせない**: 向きを逆にした知らせが、終了や切替を 1 フレームでも遅らせてはならない。状態の持ち方で同じフレームに収める（記憶の決めごと「1 フレーム遅らせる解は取らない」）。
- **ログの出所**: `log_history.rs:127` の決まりが出所 `areka::emo2_boot` を見ている。改名で `areka::runtime` に変わるので、決まりとテストと `doc/ssp-mcp/log-convention.md` を一緒に直す。直さないと、記録の分け方が黙って外れる。
- **共通の約束**: roadmap の同じ節のとおり（`crates/areka` を触るほかの spec と同じウェーブに置かない・1,000 行の上限など）。
- **Fable**: Opus で足りる。
