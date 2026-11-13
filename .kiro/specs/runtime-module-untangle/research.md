# 研究の記録: runtime-module-untangle

## ギャップ分析（2026-11-13・`/kiro-validate-gap`）

- **測った木**: 枝 `claude/kiro-start-runtime-module-untangle-2bc9c5` の `bb70a8a5`（親は main `4863cb66`。spec の文書を足しただけで、コードは main と同じ）。
- **測り方**: `crates/areka/src/emo2_boot/mod.rs` から `mod` の宣言をたどり（`#[path]` も追う）、親が `#[cfg(test)]`／`#[cfg(all(test, …))]` で宣言したファイルをテストのファイルとした。本番のファイルの中の `#[cfg(test)]` の項目も除いた。説明の文と文字列は消してから数えた。`use crate::{a::b, c}` の束は 1 つずつに開き、本文の中の `crate::x::y` も 1 回ずつ数えた（「回」は使う所の数）。`super::` は、モジュールの深さ（`#[path]` の分も入れた）を超えて `emo2_boot` の外へ抜けるものだけを数えた。
- **数の性質**: 要件 3.3 の「grep の式」はまだ決めていない。ここの数は上の数え方の値で、設計の段で式を決めて測り直す（研究が要る項目 R1）。

### 1. 分析の要約

- 今の `emo2_boot` は**本番 61 ファイル**（非空行 18,395・本番のファイルの中のテストを含む）と**テスト 175 ファイル**（非空行 60,271）。brief の 51 ファイルより 10 多いのは、そのあとに足されたファイル（`balloon_visibility_*`・`frame/attach.rs`・`frame/scale_notice.rs` ほか）の分。
- 要件 3.1 の今の値は **18 ファイル・97 回**。外へ抜ける `super::super` は本番では 0 回で、全部が `crate::` で始まる。移す 12 ファイルの分が 76 回、`runtime` に残るファイルの分が **8 ファイル・21 回**（brief が挙げた 4 ファイル＋要件の段で見つけた 4 ファイル）。
- 移した後は、`runtime` に残るファイルが移したファイルを引く所が **8 ファイル・20 回**になる（ほかに `consumer_ledger.rs` の説明の文の中のリンクが 6 か所）。これを切るのが、この spec でいちばん重い仕事。
- 移すファイルのうち `reload_args.rs`（146 行）と `shell_balloon_resolve.rs`（239 行）は、本体の外を引かない。`shell_carry.rs`（388 行）も外を引くのは `GhostSlot` の 1 か所だけで、使い手の大半は `runtime` に残る `frame/attach.rs`。この 3 本は `runtime` に残すと切る所が減る（議題 1）。
- 切り方は 3 つでほぼ済む。⑴ `GhostSlot` と起動の文脈を引く所は、session が据える「関数の口」（同期で呼ぶ・持たない）にする。⑵ 終わる相（`run_ghost_quit_phase`）と実行系の起動（`runtime_boot`）は、使い手ごと session へ移すか、引数で受け取る。⑶ 純粋な型と関数（`ShellBoxHover`・`settle_box_hover`・倍率の表・切替の要求の型）は `runtime` へ下ろす。どれも同じフレームの中で今と同じ順に呼ぶので、1 フレームの遅れは出ない。規模は **20〜23 タスク**（brief の 15〜18 より増える）。

### 2. 今の形

#### 2.1 要件 3.1 の今の値（本番・ファイルごと）

| ファイル | 回 | 引く相手（項目） | 扱い |
|---|---:|---|---|
| `ghost_switch.rs` | 34 | `app_exit::{ExitOrigin, WindowsClosed, close_windows_for_restart, quit_app}`・`boot_config::{BootContext, ConfigInputs, CurrentGhost, resolve_balloon_for_ghost}`・`boot_resolve::{DEFAULT_GHOST_FOLDER, GhostDecision, GhostRoute, NoBalloon, pick_index, running_name, write_switch_drop}`・`ghost_session::{GhostBootInputsSource, GhostSlot, RuntimeRebuild, boot_ghost_strict, commit_ghost_windows, rebuild_runtime_in_place, reopen_ghost_windows}`・`install::desk::{on_steady, run_overwrite_between}`・`readonly::of`・`record_last_used`・`roots::user_root::{exe_dir, writable_copy}`・`session_end::SessionEnded`・`update::desk::{on_steady, on_switch_end}`・`write_failure::fatal` | session へ移す |
| `reload.rs` | 10 | `boot_config::BootContext`・`boot_resolve::running_name`・`exit_wait::{self, WorkGate}`・`ghost_session::GhostSlot`・`install::desk::is_busy`・`log_history::TARGET_ERROR`・`readme::{resolve_path, set_path}`・`update::desk::is_busy` | session へ移す |
| `runtime_boot.rs` | 8 | `input_events::user_break::wire_user_break`・`install::desk::raw_sender`・`is_benign_boot_error`・`readme::{ReadmeRequest, resolve_path, wire_readme}`・`update::desk::raw_sender`・`write_failure::sender` | session へ移す |
| `install_cue.rs` | 7 | `install::{InstallOrigin, RawInstallRequest, fetch_url}`・`install::judge::{ScriptRefusal, ScriptRequest, script_request}`・`readonly::is_on` | `install/` へ移す |
| `shell_balloon_switch.rs` | 6 | `boot_config::BootContext`・`boot_resolve::pick_index`・`exit_wait::{self, WorkGate}`・`ghost_session::GhostSlot`・`update::desk::is_busy` | session へ移す（型は議題 3） |
| `switch_copy.rs` | 4 | `roots::user_root::{CopyPlan, copy_for_write, copy_unless_readonly, plan_copy}` | session へ移す |
| `same_folder_pin.rs` | 2 | `boot_resolve::{GhostDecision, running_name}` | session へ移す |
| `readme_cue.rs` | 2 | `readme::{ReadmeRequest, destination::classify}` | `readme/` へ移す |
| `update_cue.rs` | 2 | `update::{RawUpdateRequest, TargetKind}` | `update/` へ移す |
| `shell_carry.rs` | 1 | `ghost_session::GhostSlot`（`replay` の中の `seriko_sink()`） | 議題 1 |
| `frame/switch.rs` | 7 | `boot_config::BootContext`・`boot_resolve::{BalloonDecision, BalloonRoute, record_last_balloon, record_last_shell}`・`ghost_session::{GhostSession, GhostSlot}` | 残る・切る |
| `balloon_visibility_phase.rs` | 4 | `ghost_session::GhostSlot`・`input_events::balloon::BalloonWiring`・`input_events::shell_box::{ShellBoxHover, settle_box_hover}` | 残る・切る |
| `frame/scale_notice.rs` | 3 | `ghost_session::GhostSlot`・`ghost_session::ghost_session_scope_scale::{percent_text, scaling_text}` | 残る・切る |
| `frame.rs` | 2 | `app_exit::{ExitOrigin, quit_app}` | 残る・切る |
| `frame/attach.rs` | 2 | `ghost_session::{GhostSession, GhostSlot}` | 残る・切る |
| `frame/status_report.rs` | 1 | `ghost_session::GhostSlot` | 残る・切る |
| `frame/window_state.rs` | 1 | `boot_config::BootContext` | 残る・切る |
| `frame/wiring.rs` | 1 | `ghost_session::ghost_session_scope_scale::SharedScopeScaleTable` | 残る・切る |
| **計** | **97** | 移す 12 ファイルの中で外を引くもの 10 ファイル 76 回・残るもの 8 ファイル 21 回 | |

- 外を引かない移すファイル: `reload_args.rs`（0 回）・`shell_balloon_resolve.rs`（0 回。引くのは `shell_balloon_switch::{SkinKind, SkinSpec}` だけ）。
- `mod.rs`・`spine.rs` ほかの残りの 43 ファイルは、本番で外を引かない（`mod.rs` の `crate::is_benign_boot_error` は説明の文だけ）。

#### 2.2 brief に無い 4 ファイル（要件の段の指摘の確かめ）

4 つとも本番のコードで引いている。項目は要件の Boundary Context の書きどおり。

| ファイル | 引く項目 | 使い方 |
|---|---|---|
| `frame/attach.rs` | `GhostSession`・`GhostSlot`（16 行目） | 456〜458 行目: `GhostSlot` → `GhostSession::seriko_sink` を取り、`skin_facts_copy::copy` の事実を seriko へ渡す（1 か所）。 |
| `frame/scale_notice.rs` | `GhostSlot`（19 行目）・`ghost_session_scope_scale::{percent_text, scaling_text}`（20 行目） | 60〜63 行目: `kanade()` の送り手の写しを取る。48・127 行目: 倍率の文字（純粋な関数）。 |
| `frame/window_state.rs` | `boot_config::BootContext`（26 行目） | 200〜202 行目: `ctx.current.ghost.folder` を記録の欄に入れるだけ（無ければ `"-"`）。 |
| `frame/wiring.rs` | `ghost_session_scope_scale::SharedScopeScaleTable`（35 行目） | 188・246・251 行目: `Emo2Wiring` の欄の型（`Arc<RwLock<ScopeScaleTable>>`）。 |

`ghost_session_scope_scale.rs`（169 行）の中で、表の型（`ScopeScaleTable`・`SharedScopeScaleTable`・`ScopeScales`）と文字の関数（`percent_text`・`scaling_text`）は `areka_emo_present` と `emo2_boot::target_map` しか使わない。sylphya のプロパティにつなぐ `wire`・`ScopeScaleSource` だけが session の側の仕事。

#### 2.3 移した後に `runtime` から移したファイルを引く所（指摘 b）

本番のコード（説明の文と `#[cfg(test)]` を除く）で、移す 12 ファイルを指す所。

| 残るファイル | 回 | 引く項目 | 外を引くか（引かれる側） | 切り方の候補 |
|---|---:|---|---|---|
| `frame.rs` | 2 | `ghost_switch::{self, SwitchInFlight}`（`on_notice`・`on_ghost_stopped`） | 引く（`ghost_switch` は外を 34 回） | 終わる相 `run_ghost_quit_phase` を session へ移す（議題 2）。使う所は 234・238・239 行目で、全部この相の中。 |
| `frame/switch.rs` | 9 | `ghost_switch::SwitchInFlight`・`reload`（`skin_phase_ended`・`skin_is_reload`・`skin_applied`）・`shell_balloon_resolve::SkinCandidate`・`shell_balloon_switch::{SkinKind, SkinSwitchInFlight, SkinSwitchStage, SwapFinish, skin_ref_name, skin_ref_path}` | `SwitchInFlight`・`reload` は引く。`SkinCandidate`・`shell_balloon_switch` の型と `skin_ref_*` は引かない | 型は `runtime` に置く。`SwitchInFlight` は在るかどうかだけ＝口で問う。`reload` の 3 つは後始末の口へ（議題 3）。 |
| `frame/attach.rs` | 1 | `shell_carry`（`decide`・`decide_binds`・`replay`） | `shell_carry` は `GhostSlot` を 1 回 | `shell_carry` を `runtime` に残す（議題 1）。 |
| `frame/wiring.rs` | 1 | `shell_carry::ShellCarry` | 同上 | 同上。 |
| `mod.rs` | 2 | `runtime_boot::{CueSinkSources, boot_runtime}` | `runtime_boot` は外を 8 回 | `CueSinkSources` は `runtime` に置き、`boot_runtime` は引数で受ける（議題 4）。 |
| `change_cue.rs` | 1 | `ghost_switch::ChangeRequestRaw`（欄 `name`・`raise_event` だけの型） | 型そのものは引かない | 型を `change_cue.rs` に置き、`ghost_switch` が引く。 |
| `switch_cue.rs` | 2 | `shell_balloon_switch::{SkinKind, SkinRequestRaw}` | 型そのものは引かない | 型を `runtime` に置く。 |
| `reload_cue.rs` | 2 | `reload_args::{ScriptReload, parse_script}` | `reload_args` は外を引かない | `reload_args` を `runtime` に残す（議題 1）。 |
| **計** | **20** | | | |

- ほかに、`consumer_ledger.rs` の説明の文のリンク 6 か所（`super::readme_cue::ReadmeCueSink`・`super::install_cue::InstallCueSink`・`super::update_cue::UpdateCueSink` が 2 回ずつ）がパスを失う。`cargo check` は通るが `cargo doc` のリンクの警告になる（`tools/test-all.ps1` は `cargo doc` を回さない）。説明の文の書き換えで済む。
- `install_cue`・`update_cue`・`readme_cue` を引く本番は `runtime_boot.rs` の 3 行だけ（`InstallCueSink`・`UpdateCueSink`・`ReadmeCueSink`）。`runtime_boot` も session へ移るので、3 本の移動で `runtime` から外を引く所は増えない。
- テストの側で残るファイルが移すファイルを引く所（数えるだけ）: `spine.rs` が `runtime_boot::runtime_boot_options`、`ghost_switch_test_support.rs` が `frame::{KanadeNoticeRx, run_ghost_quit_phase}` ほか。

#### 2.4 移すファイルが `runtime` の中から引くもの（見える範囲の広げ）

- 移すファイルが引く `runtime` の中の私有のモジュール: `change_cue`・`reload_cue`・`switch_cue`・`window_state_cue`（`mod.rs` で `mod x;`＝私有）と `special_name`（同じく私有。`ghost_switch.rs` が `pub(crate) use super::special_name::{…}` で出し直している）。session から引くには `pub(crate) mod` へ広げる。
- `pub(in crate::emo2_boot)` の項目は 31（`frame/wiring.rs` 14・`frame/status_report.rs` 7・`balloon_visibility_phase.rs` 3 ほか）。移す本番は `Emo2Wiring` の在る・無いを見るだけで欄には触らないが、`shell_carry::hand_over` は `wiring.shell_carry` に書く（議題 1 の分け方で決まる）。移すテストが欄を触る所は設計の段で数える（R3）。
- 広げた分は、次の `runtime-crate-split` で crate の公開の形になる。

#### 2.5 移すファイルへ本体の外（`emo2_boot` の外）から来る参照

パスを書き換えるだけの所（本番とテストの合計・`crate::emo2_boot::<名>` の出現）: `ghost_switch` 43・`switch_copy` 8・`shell_balloon_switch` 7・`shell_balloon_resolve` 6・`reload` 4・`runtime_boot` 3・`reload_args` 2。多いのは `main_session_mark_write_tests.rs` 5・`ghost_session.rs` 4・`mcp/reload_tests.rs` 3・`log_history_tests.rs` 3。`offline_dump/mod.rs` は `shell_balloon_resolve` を引く（steering `structure.md` の `offline_dump/` の行が「使ってよいもの」に `emo2_boot::shell_balloon_resolve` を、「使ってはならないもの」に `ghost_session`・`boot_resolve` ほかを挙げている。session へ移すと、`offline_dump` が session を引く形になる＝議題 1）。

#### 2.6 テストのコードが本体の外を引く所（要件 3.5 の今の値）

54 ファイル・約 256 回（同じ数え方。`mod.rs` の中の `mod wire_tests { use super::*; }` の 3 回は数え方の誤りなので除いた）。相手の多い順に `boot_resolve` 61・`ghost_session` 45・`boot_config` 32・`app_exit` 24・`input_events` 17・`readme` 12・`roots` 10・`update` 10・`install` 8・`scope_bind_fixture_test_support` 8・`restore_merged_placements`（`main.rs`）7・`placement_tests` 6・`log_history` 4。多いファイルは `ghost_switch_fallback_tests.rs` 20・`update_cue_tests.rs` 16・`ghost_switch_reload_tests.rs` 16・`ghost_switch_test_support.rs` 15・`reload_tests.rs` 14・`shell_balloon_switch_tests.rs` 13・`switch_copy_tests.rs` 11。多くは移すファイルのテストなので、移した後の `runtime` のテストの数はずっと減る。`frame_transition_atomicity_tests.rs`（5 回）と `frame_transition_branch_tests.rs`（1 回）は `crate::placement_tests::transition_judge` を引く（要件 7.2）。

#### 2.7 `runtime` の外のテストが使う `runtime` のテストの支え（要件 7.1・指摘 e）

`emo2_boot` の外のファイルで `emo2_boot::<テストのモジュール>` を名指す所（`use` の行の数。`spine::X` のような素の使い方は数えていない＝下限）。

| 支え | 回 | ファイル | 使う項目 |
|---|---:|---|---|
| `ghost_switch_test_support` | 40 | 35（`ghost_session_*_tests` 8・`shell_balloon_switch_session_*_tests` 5・`mcp/*` 8・`install/*` 4・`session_end_*` 3・`update/*` 2・`main_session_mark*` 2 ほか） | モジュールごと 38・`BALLOON` 2 |
| `spine` | 35 | 31（`mcp/*` 9・`install/*` 4・`input_events/*` 3・`session_end_*` 3・`ghost_session_*` 6・`shell_balloon_switch_session_*` 4 ほか） | `RecordedCall` 11・モジュールごと 10・`Progress` 5・`GpuPermit` 3・`ScriptedShioriBackendBuilder` 2・`join_thread_bounded` 2・`wait_until` 1・`wait_recv` 1 |
| `sample_test_support` | 4 | 4（`ghost_session_reload_tests`・`ghost_session_restart_tests`・`placement_tests/measure_bake_count_tests`・`placement_tests/reset_balloon_pos_tests`） | `acquire_emo2` 2・モジュールごと 2 |
| `spine::hold_support` | 2 | 2（`session_end_deadline_tests`・`session_end_sync_send_tests`） | `HoldAt` 2 |
| `frame::balloon_runtime_test_support` | 2 | 2（`input_events/balloon_pressed_tests`・`balloon_wheel_tests`） | `manual_balloon_runtime` 1・モジュールごと 1 |

- `frame_test_support` を `emo2_boot` の外から名指す所は 0。
- `ghost_switch_test_support` は `ghost_switch` と一緒に session へ移るので、`runtime-crate-split` の後は「session の側の支え」になり、境をまたがない。境をまたぐのは `spine`・`sample_test_support`・`spine::hold_support`・`frame::balloon_runtime_test_support` の 4 つ（43 回・39 ファイル）。
- `mcp/dump_surface.rs`（本番のファイル）の 3 回は、その中の `#[cfg(test)]` の部分。

#### 2.8 パスや名前を文字列で持つ所（要件 5 の列挙に無いもの）

- `crates/areka/src/tick_gate_config_producers_tests.rs`: `"emo2_boot/…"` のパス 8 本（`adapter.rs`・`balloon_visibility_phase.rs`・`frame/scale_text.rs`・`hover_inject.rs`・`move_cue.rs`・`talk_lifecycle.rs`・`window_state_cue.rs`・`zorder_cue.rs`）を 18 か所で持つ（`include_str!` とその名）。全部 `runtime` に残るファイル。
- `crates/areka/examples/collision-probe.rs` 218・224 行目: `#[path = "../src/emo2_boot/target_map.rs"]`・`#[path = "../src/emo2_boot/hit_region.rs"]`。書き換えないと `cargo check --all-targets` が赤（黙っては通らない）。
- `crates/areka/tests/stay_on_top_real_run.rs` 51〜54 行目: `RUST_LOG` の文字列に `areka::emo2_boot::…` が 6 つ（要件 4.4 の「`RUST_LOG` の文字列」に当たる）。
- `emo2_boot` の中のテストの `include_str!`: 移すファイルを読むのは `zorder_wiring_tests.rs` の `include_str!("runtime_boot.rs")` の 1 か所（ほかに `../ghost_session.rs`・`../main.rs`）。相対のパスなので、移すと読めずに赤になる（黙っては通らない）。
- `placement_tests/` の文字列のパス 49 か所（`zorder_group_branch_coverage_tests` 30・`zorder_inversion_branch_coverage_tests` 8・`spawn_zorder_pair_deferred_tests` 8・`measure_template_tests` 3）。要件 5.1 の 9 ファイルに入っている。
- `crates/areka-emo-text/src/actor_lookahead_tests.rs` 343 行目: テストの名前 `emo2_boot_talk_reveals_without_moving_glyphs`（説明の文ではなく識別子）。要件の Out of scope は「本体の外の crate の説明の文」だけを挙げているので、要件 5.9 の grep に残る（議題 6）。
- `crates/log-capture-kit/src/event_tests.rs` の出所の見本の値 `"areka::emo2_boot"` 5 か所は、要件の Out of scope に挙がっている（直さない）。

#### 2.9 プロセスで共有する状態（要件 3.9）

`balloon_visibility_wait.rs` の `OnceLock`・`hover_inject.rs` の `OnceLock`・`frame/switch.rs` の `AtomicU64`（`NEXT_EPOCH`）。3 本とも `runtime` に残るファイルにあるので、どの案でも置き場は `src/runtime/` の同じファイルのまま（`frame/switch.rs` を session へ移す案だけ、`NEXT_EPOCH` が session へ移る）。

### 3. 要件と今のコードの対応

| 要件 | 今のコードで当たる所 | ギャップ |
|---|---|---|
| 1 名前 | 識別子 7 つ・ログの文言・イベントの値 4 つ・テストの名前（brief の測り） | **Constraint**: `ghost_session.rs`（945 行）・`main.rs`（989 行）は 1,000 行の近く。改名で行が伸びると上限を超えうる（要件 8.6）。 |
| 2.1〜2.3 移す | 12 本の本番・テスト（下の 4 章） | **Missing**: 移すファイルが引く私有のモジュール 5 つの見える範囲（2.4 節）。**Unknown**: `reload_args`・`shell_balloon_resolve`・`shell_carry` を移すか（議題 1）。 |
| 2.4 受け口の形 | `*_cue.rs` の同型・`consumer_ledger.rs` の表 | 3 本の受け口の型は動かすだけで済む。`consumer_ledger.rs` の説明のリンク 6 か所を直す。 |
| 2.5 残るファイルが引く型 | 2.3 節の 20 回 | **Missing**: 切り方（4 章）。 |
| 2.6 落ちの報告 | `crash_report_test_support.rs`（`mod.rs` の `#[cfg(test)]`） | 改名で一緒に動くだけ。 |
| 3.1〜3.3 外を引かない | 2.1 節の 97 回 | **Missing**: 数える式（R1）。 |
| 3.4 説明の文 | 本体の外のパスの名指し（例 `hit_region.rs` の `crate::is_benign_boot_error`） | 数えて一覧にするだけ。 |
| 3.5 テストの参照 | 2.6 節 | 数えるだけ。 |
| 3.6 終わる道の順 | `frame.rs` の `run_ghost_quit_phase`→`quit_app`・`ghost_quit_system.before(emo2_frame_system)`（`wire_kanade_stop`） | 議題 2。 |
| 3.7 切替と読み直しの順 | `frame/switch.rs` の `finish_shell`・`finish_balloon` の後始末の順 | 議題 3。 |
| 3.8 1 フレームも遅らせない | 6 章の表 | どの推奨案も同期で呼ぶ。 |
| 3.9 共有する状態 | 2.9 節 | 置き場は変わらない。 |
| 4 ログの出所 | `log_history.rs` 127 行目 `rule("areka::emo2_boot::ghost_switch", …)`・`stay_on_top_real_run.rs` の `RUST_LOG`・`tools/load-flake.ps1` 47 行目 | **Constraint**: 項目を別のファイルへ分けて移すと、その記録の出所も変わる（例 後始末の口の中の `warn!` が `areka::session::…` になる）。`log_history` の決まりは `ghost_switch` の 1 本だけなので、ほかは記録の分け方に響かない（R4 で確かめる）。 |
| 5 古い名前とパス | 2.5・2.8 節 | **Missing**: `tick_gate_config_producers_tests.rs`・`examples/collision-probe.rs`・`stay_on_top_real_run.rs` は要件 5 の列挙に無い（「ほか」に入る）。emo-text のテストの名前は議題 6。 |
| 6 emo2 の残り | `DEFAULT_GHOST_FOLDER` の使い手（`ghost_switch.rs` 424・637・697・714 行目） | 使い手が session へ移る。分け方は要件どおり。 |
| 7 申し送り | 2.6・2.7 節 | 表はここにある。設計の段で測り直す。 |
| 8 検証 | `cargo test -- --list`・`cargo check`・`tools/test-all.ps1`・実機 2 つ | 終わる道の記録の順は、議題 2 の推奨案なら変わる所が無い。 |

### 4. 切り方の案（参照の組ごと）

#### 4.1 `GhostSlot` の中身を引く所（`frame/status_report.rs`・`balloon_visibility_phase.rs`・`frame/attach.rs`・`frame/scale_notice.rs`・`frame/switch.rs`・`shell_carry.rs`）

使う中身は `kanade()`（`Sender<KanadeMsg>`）・`seriko_sink()`（`SerikoSink`）・`memory_publisher()`・`set_shell_dir()`・`ghost_name()`／`ghost_dir()`（`frame/switch.rs` だけ）。`GhostSlot` を書き換える所は本体に 10 か所以上ある（`main.rs`・`ghost_session_runtime.rs`・`session_end.rs`・切替ほか）。

- **案 A 関数の口**: `runtime` に NonSend の型（関数のポインタの束）を置き、session が起動の結線で 1 度だけ据える。`runtime` は毎回その口で `GhostSlot` の中を借りて読む（`for<'w> fn(&'w World) -> Option<&'w Sender<KanadeMsg>>` の形）。前例は wintf の `CursorQuery(pub fn(&World, Entity, &CursorAsk) -> Option<CursorReply>)`（上の層が据える口）と `exit_wait.rs` の `on_close: fn(&mut World)`。
  - ✅ 持たないので、`GhostSlot` の入れ替えと食い違わない。seriko の送り手の持ち主は `GhostSession` だけのまま（`Emo2BootOutcome` の説明の「降ろした後も World に残る置き場に持たせると、seriko の join が戻らない」＝areka-P0-shell-balloon-switch 要件 8.5 を守る）。
  - ✅ 同期で呼ぶので 1 フレームも遅れない。
  - ❌ 口が据えられていない World（今の試験の多く）の扱いを決める必要がある。今は「`GhostSlot` が無い」と同じ縮退（`None`）にすれば、記録の文も同じにできる。
- **案 B 写しの資源**: session が `GhostSlot` を書くたびに、`runtime` の資源へ送り手の写しを入れ直す。
  - ❌ seriko の送り手の写しを `runtime` の資源が持つと、上の要件 8.5 に反して seriko の join が戻らない。
  - ❌ 書く所が 10 か所以上あり、1 か所でも漏れると古い送り手へ送る。
- **案 C 特性の箱**: `Box<dyn …>` を NonSend に置く。中身は案 A と同じで、関数のポインタより重いだけ。

**推奨: 案 A**（理由: 要件 8.5 を守れる唯一の案で、同期で遅れが無く、前例がある）。

#### 4.2 終わる相（`frame.rs` の `quit_app` と `ghost_switch::{on_notice, on_ghost_stopped}`）

`run_ghost_quit_phase`（`frame.rs` 223 行目）は、毎フレームの相とは別の排他 system `ghost_quit_system` から呼ばれ、`wire_kanade_stop`（`mod.rs` 723 行目）が `ghost_quit_system.before(emo2_frame_system)` で登録する。呼び手は `ghost_session::register_systems`（`ghost_session.rs` 69 行目）。ほかに `session_end.rs` 110 行目が直に呼ぶ。`KanadeNoticeRx` を使うのもこの相だけ。

- **案 A 相ごと session へ移す**: `run_ghost_quit_phase`・`quit_as_today`・`ghost_quit_system`・`KanadeNoticeRx`・`wire_kanade_stop` を session へ移し、登録の順の相手は `runtime` の `frame_system` を引く（session → runtime の向き）。
  - ✅ system も登録の順（`.before(frame_system)`）も今と同じなので、終わる段取りの順も、毎フレームの相が「終了が指示済み」を見る巡も同じ。1 フレームの遅れは出ない。
  - ✅ `frame.rs` から `app_exit` と `ghost_switch` の参照が両方消える（2.1 節の 2 回＋2.3 節の 2 回）。
  - ❌ 試験 `frame_ghost_quit_tests.rs`・`frame_ghost_quit_switch_tests.rs`・`frame_ghost_quit_logsink_tests.rs`（3 本・700 行）が session へ移るか、`runtime` に残って session を引く（テストの参照なので数えるだけ）。記録の出所が `areka::session::…` に変わる（要件 4.1 のとおり）。
- **案 B 知らせで向きを逆にする**（brief の書き）: `runtime` が「終われ」の知らせを出し、session の system が受けて `quit_app` を呼ぶ。
  - ❌ 受け手が後の system なら、その巡の毎フレームの相は「終了が指示済み」を見ずに 1 巡走る。受け手を前に置くと、知らせを出すのも前でなければならず、結局 A と同じ形になる。1 フレームの遅れか、終わる段取りの順の変化が起きやすい。
- **案 C 関数の口で `quit_app` を呼ぶ**: 4.1 節の口に `quit_app` を足す。順は変わらないが、`ghost_switch::on_notice`・`on_ghost_stopped` も口にする必要があり、口が増える。

**推奨: 案 A**（理由: 相そのものがゴーストの運行の段取りで、移しても system と登録の順が変わらず、終わる道の記録の順が構造の上で同じだと示せる）。

#### 4.3 差し替えの相（`frame/switch.rs`）

この相は毎フレームの相の中（drain の直後）で走り、`Emo2Wiring` の欄・`reconcile_reported_sizes`・`select_balloon_face`・配置の置き直しを使う。本体に頼るのは次の所だけ。

- `step_waiting`・`await_gap_again`・`raise_changed`: `kanade()`（4.1 節の口で済む）。
- `commit`: `SwitchInFlight` が在るか（255 行目・ゴースト切替が勝つ）と `seriko_sink()`。
- `finish_shell` の終わり: `set_shell_dir` → 読み直しなら `reload::skin_applied` で戻る → `record_last_shell` → `OnShellChanged`。
- `finish_balloon` の終わり: 読み直しなら `reload::skin_applied` で戻る → `BootContext.current.balloon` の書き換え → `record_last_balloon` → `OnBalloonChange`。
- `run_switch_phase` の終わり: `reload::skin_phase_ended`。
- 型: `SkinKind`・`SkinSwitchInFlight`・`SkinSwitchStage`・`SwapFinish`・`skin_ref_name`・`skin_ref_path`（`shell_balloon_switch.rs` の 1〜160 行目と 444〜457 行目。本体を引かない）・`SkinCandidate`（`shell_balloon_resolve.rs`）。

- **案 A 相は `runtime` に残し、本体に頼る終わりを「後始末の口」へ**: 型を `runtime` に置く（`shell_balloon_switch.rs` を分ける）。`finish_shell`・`finish_balloon` の、本体に頼る終わりの段（上の矢印の列）をひとまとめにして、session が据える口（関数のポインタ）で同じ場所から呼ぶ。`SwitchInFlight` の有無と `reload` の 3 つも口で問う。
  - ✅ 後始末の段の順はそのまま（口の中で同じ順に呼ぶ）。同期で呼ぶので遅れない。`NEXT_EPOCH` は `runtime` に残る。
  - ❌ 口が 5〜6 本になる（4.1 節の口と同じ束にまとめられる）。記録の `warn!(skin_memory_not_recorded)`・`warn!(skin_switch_event_not_sent)` ほかの出所が session へ変わる。
- **案 B 相ごと session へ移し、毎フレームの相から口で呼ぶ**: `frame/switch.rs` を session へ移し、`runtime` は同じ順の場所で口を呼ぶ。
  - ❌ 相が使う `Emo2Wiring` の欄（`pub(in crate::emo2_boot)` の 14）と frame の関数を session へ開く必要があり、次の spec で crate の公開の形が大きく広がる。
  - ❌ 差し替えの試験 7 本（`frame/switch_*_tests.rs`）の置き場も変わる。
- **案 C 知らせで向きを逆にする**: 後始末の終わりを知らせにして、後の system で受ける。
  - ❌ `OnShellChanged`・`LastShell` の記録・読み直しの次の段が 1 フレーム以上遅れうる（要件 3.8 に反する）。

**推奨: 案 A**（理由: 相の中身は窓と文字の層の仕事で `runtime` に属し、本体に頼るのは終わりの段だけなので、口 1 つで順を変えずに切れる）。

#### 4.4 起動の文脈（`frame/window_state.rs` の `BootContext`）

記録の欄 `ghost` の値（`ctx.current.ghost.folder`、無ければ `"-"`）を読むだけ。

- **案 A**: 4.1 節の口に「今のゴーストのフォルダ名」を足す。
- **案 B**: session が `runtime` の資源に名前の写しを置く（切替のたびに書き直す＝漏れると記録の名前が古くなる）。

**推奨: 案 A**（理由: 4.1 節と同じ理由。書く所の漏れが起きない）。

#### 4.5 倍率の表（`frame/wiring.rs`・`frame/scale_notice.rs`）

- **案 A**: `ScopeScaleTable`・`SharedScopeScaleTable`・`ScopeScales`・`percent_text`・`scaling_text` を `runtime` へ下ろす。`ghost_session_scope_scale.rs` には `wire`・`ScopeScaleSource` だけを残し、`runtime` の型を引く。
- **案 B**: `ghost_session_scope_scale.rs` ごと `runtime` へ移す（`SylphyaReader` の結線まで `runtime` に入る）。

**推奨: 案 A**（理由: 表と文字は純粋で `runtime` の型しか使わない。結線は session の仕事のまま）。`kanade()` は 4.1 節の口。

#### 4.6 入力の側（`balloon_visibility_phase.rs`）

- `ShellBoxHover`・`settle_box_hover`（`input_events/shell_box.rs`・`areka_emo_compose`／`areka_emo_text` の型だけ）: `runtime` へ下ろす（brief の 5 番）。`input_events` が `runtime` を引く向きになる。
- `BalloonWiring` のホバーの集合（`balloon_hover: HashSet<usize>`・読むのは `is_balloon_hovered`、消すのは `clear_balloon_hover`）: 別の NonSend の型に分けて `runtime` に置き、`input_events`（`balloon_moved.rs` の `set_balloon_hover`・`balloon_exit.rs` の `clear_balloon_hover`）がそれを書く（brief の 4 番）。
  - **Constraint**: `observe_hover_wiring` は `BalloonWiring` が無いと `error!` を 1 回出し「抑止なし」として扱う。分けた型を `BalloonWiring` と同じ所・同じ時に据えないと、この記録が出る条件が変わる。`BalloonWiring` を据える試験の組み立ても、分けた型を一緒に据える必要がある（R5）。
  - 同期の読み書きなので、遅れは出ない。

#### 4.7 実行系の起動（`mod.rs` の `runtime_boot::boot_runtime`）

`wire_emo2_boot` は手順 5 で `boot_runtime` を呼び、成立したら手順 6 で `Emo2Wiring` を据える。失敗のときは seriko の送り手を落とす（順に意味がある）。呼び手は `ghost_session.rs` 865 行目だけ。

- **案 A 起動を引数で受ける**: `wire_runtime` が `FnOnce(&mut World, &CueSinkSources) -> Result<GhostRuntime, …>` を受け、手順 5 の場所で呼ぶ。`CueSinkSources` は `runtime` に置く（中身は `SerikoSink`・`EmoTextSink`・`TalkClock` と `runtime` の型の送り手だけ）。session（`ghost_session.rs`）が `runtime_boot::boot_runtime` を閉じて渡す。
  - ✅ 手順の順も失敗のときの後始末も同じ。同期なので遅れない。
  - ❌ `Emo2BootInputs` の `shiori`・`ticker`・`app_profile_dir`・`boot_origin` は `boot_runtime` だけが使うなら、閉じた中へ移すか、欄のまま残すかを決める（R6）。
- **案 B `wire_runtime` を 2 つに分ける**: 手順 1〜4（準備）と手順 6〜7（据える）に分け、session が間で起動する。
  - ❌ 呼び手の形が変わり、失敗のときの後始末（seriko の送り手を落とす）が session と `runtime` にまたがる。

**推奨: 案 A**（理由: 呼び手 1 か所の変更で、順と後始末がそのまま残る）。`wire_user_break` の呼び出しは `runtime_boot` と一緒に session へ移る（brief の 6 番）。

#### 4.8 cue の受け口が引く要求の型（`change_cue.rs`・`switch_cue.rs`・`reload_cue.rs`）

- `ChangeRequestRaw`（`name`・`raise_event` だけ）を `change_cue.rs` に置き、`ghost_switch` が引く。
- `SkinKind`・`SkinRequestRaw` を `runtime` に置く（4.3 節の型と同じ所）。
- `reload_args` は `runtime` に残す（議題 1）。残さない場合は `ScriptReload`・`parse_script` だけを `runtime` に置く。

型を動かすだけなので遅れは出ない。受け口の同型の形と `consumer_ledger.rs` の表の行は変わらない（要件 2.4）。

#### 4.9 `shell_carry.rs`

使い手は 2 つに分かれる。`decide`・`decide_binds`・`replay`・`ShellCarry` は `frame/attach.rs`・`frame/wiring.rs`（残る）が、`capture_for_reload`・`hand_over` は `ghost_switch.rs`（移す）だけが使う。本体を引くのは `replay` の `GhostSlot`（`seriko_sink()`）と、`capture_for_reload` の `super::reload::carries_shell_state`。

- **案 A ファイルは `runtime` に残し、2 つの関数を session へ**: `capture_for_reload`・`hand_over` を session（`ghost_switch.rs` か `reload.rs` の隣）へ移し、`replay` の `seriko_sink()` は 4.1 節の口。`hand_over` が書く `wiring.shell_carry` は `runtime` の関数（例 `RuntimeWiring::set_shell_carry`）を通す。
- **案 B 要件 2.1 のとおり session へ移す**: `runtime` に残る `frame/attach.rs`・`frame/wiring.rs` が引く `ShellCarry`・`decide`・`decide_binds`・`replay` を要件 2.5 で `runtime` に残すと、移るのは 2 つの関数だけになり、案 A と同じ形になる。

**推奨: 案 A**（議題 1 と一緒に決める）。

### 5. 全体の進め方の案

- **案 A 要件 2.1 の字面どおり 12 本を全部移し、要る型と関数だけを要件 2.5 で `runtime` に残す**
  - ✅ 要件を改めずに済む。
  - ❌ `reload_args`・`shell_balloon_resolve` は中身の大半が `runtime` に残り、`shell_carry` も 2 つの関数しか移らない。git の移動の履歴（要件 2.3）が分かれたファイルの側に付かない。`offline_dump` が session を引く形になる（steering `structure.md` の「使ってはならないもの」と食い違う）。
- **案 B 外を引かない 2 本（`reload_args`・`shell_balloon_resolve`）は `runtime` に残し、`shell_carry` は 4.9 節の案 A で分ける**（要件 2.1 の改めが要る）
  - ✅ 切る所が減る（2.3 節の 20 回のうち `reload_cue` の 2 回・`frame/switch.rs` の `SkinCandidate` の 1 回・`frame/attach.rs`・`frame/wiring.rs` の 2 回が、切らずに済む）。`offline_dump` の決まりと合う。
  - ❌ 要件 2.1 と brief の「外へ移せるもの」の列を改める。
- **案 C ハイブリッド**: 案 B に加えて、`shell_balloon_switch.rs` の型の部分（1〜160 行目・444〜457 行目）も別のファイルとして `runtime` に残す（4.3 節）。移るのは要求の入口（`wire_switch_rx`・`drain_switch_requests`・`request_skin_switch*`・`accept_skin`・`SwitchContext`・`discard_for_exit`）だけ。

**推奨: 案 C**（理由: 本体を引く部分だけを session へ移すという spec の目的にいちばん合い、切る所と口の数が最も少ない）。

### 6. 規模と危うさ

- **規模: L（1〜2 週）**。改名と移動は機械的だが、`runtime` に残るファイルの参照の切り離しが 8 ファイル・41 回（2.1 節の 21 回＋2.3 節の 20 回）あり、口の据え付けと試験の組み立ての直しが要る。
- **危うさ: 中**。新しい型の作りは無く、前例（wintf の `CursorQuery`）がある。危ういのは、口が据えられていない World の縮退の記録を今と同じにすること（4.1 節）と、ホバーの集合を分けたときに記録が出る条件を同じにすること（4.6 節）。
- **タスクの見積もり: 20〜23**（brief の 15〜18 から増える。増えた分は、要件の段で見つけた 4 ファイルと、移した後の `runtime` から session への参照の 20 回）。内訳の目安: 着手の測り 1・モジュールの改名と全部のパスの書き換え 2・型と関数の改名 1・ログの文言とイベントの値 1・テストの名前と対応表 1・関数の口の型と据え付け 1・`GhostSlot`／`BootContext` を口へ（status_report・balloon_visibility_phase・attach・scale_notice・window_state）1・倍率の表とホバーの型を下ろす 1〜2・差し替えの相の型と後始末の口 2・終わる相を session へ 1・起動を引数で受ける 1・要求の型と `shell_carry` の分け 1・session へ移す 1〜2・install／update／readme へ移す 1・ログの出所の決まりと `RUST_LOG` 1・文書と steering と台帳と roadmap の対応表と audit の brief 1〜2・0 件の grep・emo2 の残り・支えの一覧・申し送り 1・検証（テストの一覧の比べ・警告・全体テスト・実機 2 つ・終わる道の記録の順）1。記憶の決めごと「一度切り出した spec を、上限 20 を少しまたぐ理由でさらに削らない」により、分ける案は出さない。

**1 フレームの遅れの見込み（指摘 c）**

| 切る所 | 推奨案 | 遅れの見込み | 理由 |
|---|---|---|---|
| `frame.rs` の `quit_app`（終わる相） | 相ごと session へ（4.2 A） | なし | system と `.before(frame_system)` の登録が今と同じ。知らせにする案（4.2 B）だと、毎フレームの相が「終了が指示済み」を見ずに 1 巡走る見込みが高い。 |
| `frame/switch.rs` の後始末・`reload` の 3 つ・`SwitchInFlight` | 後始末の口（4.3 A） | なし | 同じ場所で同期で呼ぶ。知らせにする案（4.3 C）だと `OnShellChanged`・`LastShell`・読み直しの次の段が遅れる。 |
| `GhostSlot` の中身（5 ファイル） | 関数の口（4.1 A） | なし | 毎回その場で借りて読む。写しの案（4.1 B）は遅れはしないが古い送り手を持ちうる。 |
| `BootContext`（`frame/window_state.rs`） | 関数の口（4.4 A） | なし | 記録の欄の値だけ。 |
| 倍率の表・`ShellBoxHover`・`settle_box_hover` | 下ろす（4.5・4.6） | なし | 型と関数の置き場が変わるだけ。 |
| ホバーの集合 | 型を分ける（4.6） | なし | `input_events` が書き、同じフレームの後の相が読む今の順のまま。 |
| `runtime_boot::boot_runtime` | 引数で受ける（4.7 A） | なし | 起動の結線の中で同期で呼ぶ（フレームの外）。 |
| 要求の型（`change_cue`・`switch_cue`・`reload_cue`） | 型を動かす（4.8） | なし | 型だけ。 |

### 7. 設計の段へ持ち越す研究（Research Needed）

- **R1**: 要件 3.3 の数える式。`#[cfg(test)]` の項目と `#[path]` の深さを grep だけでは除けない。ここで使った「宣言の木をたどる」数え方を道具にするか、`cargo` の出力（例 `-Zunpretty` は使えない＝安定版）に頼らない別の式にするかを決める。
- **R2**: 関数の口が据えられていない World（今の試験の多く）の縮退。今の `GhostSlot` が無いときの記録の文（`reload_shell_carry_unavailable` ほか）と同じ扱いにできるかを、口ごとに確かめる。
- **R3**: 移すテストが `pub(in crate::emo2_boot)`・`pub(super)` の項目を触る所の数と、広げる見える範囲の一覧。
- **R4**: 項目を分けて移したときに記録の出所が変わる行の一覧（後始末の口・終わる相・`shell_carry` の 2 つの関数）と、それを名前で読む所（`log_history`・試験の出所の照合・`doc/ssp-mcp/log-convention.md`）。
- **R5**: ホバーの集合を分けた型を据える所（本番の結線と、`BalloonWiring` を据える試験の組み立て）の一覧。
- **R6**: `Emo2BootInputs` の欄のうち `runtime` が自分で使うもの（起動を引数で受けた後に `RuntimeInputs` に残す欄）。
- **R7**: 改名で 1,000 行を超えうるファイル（`ghost_session.rs` 945 行・`main.rs` 989 行・`emo2_boot/mod.rs` 833 行）の行の伸び。口を据える関数は `src/session/` に置き、`ghost_session.rs` に足さない。

### 8. 議題の候補（要件の討議へ）

> 要件の討議（2026-11-13）での分け: 1 は開発者に諮る議題で、推奨案（5 章の案 C）の承認で確定した（要件 2.1 を「本体に頼る部分だけを移す」原則に改めた）。2〜5 と 7 章の R1〜R7 は切り方（how）なので設計の段（`/kiro-design`）で決める（要件 2.5・3.8 が枠を持つ）。6 は自明な修正として要件の Out of scope と要件 6.2・5.9 に足した。7 は議題にしない（記憶の決めごと「一度切り出した spec を、上限 20 を少しまたぐ理由でさらに削らない」）。

1. **外を引かない移すファイルを `runtime` に残すか**（要件 2.1 の改め）。対象は `reload_args.rs`（外 0・使い手は `reload_cue.rs`〔残る〕・`reload.rs`・`runtime_boot.rs`・`mcp/reload.rs`）・`shell_balloon_resolve.rs`（外 0・使い手は `frame/switch.rs`〔残る〕と `offline_dump`）・`shell_carry.rs`（外は `GhostSlot` の 1 回・主な使い手は `frame/attach.rs`〔残る〕）。
   - **推奨**: 3 本とも `runtime` に残し、`shell_carry.rs` の `capture_for_reload`・`hand_over` だけを session へ移す（5 章の案 C）。理由: 切る所が 5 回減り、`offline_dump` が session を引かずに済み（steering の決まりと合う）、git の移動の履歴も分かれない。
2. **終わる相（`quit_app`）の切り方**: 知らせで向きを逆にする（brief の書き）か、相ごと session へ移すか。
   - **推奨**: 相ごと session へ移す（4.2 A）。理由: system と登録の順が今と同じで、1 フレームの遅れも終わる段取りの順の変化も構造の上で起きない。brief の「知らせで向きを逆にする」はこの形に読み替える。
3. **差し替えの相（`frame/switch.rs`）の切り方**: 相を `runtime` に残し、本体に頼る終わりの段を後始末の口にするか、相ごと session へ移すか。あわせて `shell_balloon_switch.rs` を型（`runtime`）と要求の入口（session）に分けるか。
   - **推奨**: 相は `runtime` に残し、後始末の口で切る。`shell_balloon_switch.rs` は型と入口に分ける（4.3 A・5 章の案 C）。理由: 相の中身は窓と文字の層の仕事で、本体に頼るのは終わりの段だけ。相ごと移すと `RuntimeWiring` の欄を広く開くことになり、次の spec の公開の形が膨らむ。
4. **`GhostSlot`・`BootContext` を引く所の切り方**: 関数の口（session が 1 度据える）か、写しの資源か。
   - **推奨**: 関数の口（4.1 A・4.4 A）。理由: seriko の送り手の持ち主を `GhostSession` だけに保つ（areka-P0-shell-balloon-switch 要件 8.5）ことができ、`GhostSlot` を書く 10 か所以上と食い違わない。前例は wintf の `CursorQuery`。
5. **実行系の起動（`runtime_boot`）の切り方**: `wire_runtime` が起動を引数で受けるか、`wire_runtime` を 2 つに分けるか。
   - **推奨**: 引数で受ける（4.7 A）。理由: 呼び手は `ghost_session.rs` の 1 か所で、手順の順と失敗のときの後始末（seriko の送り手を落とす）がそのまま残る。
6. **`crates/areka-emo-text/src/actor_lookahead_tests.rs` のテストの名前 `emo2_boot_talk_reveals_without_moving_glyphs`** の扱い: 要件 1 の規則で改める（テストの名前の対応表に載せる）か、要件の Out of scope に「本体の外の crate のテストの名前」として足すか。
   - **推奨**: Out of scope に足し、要件 6 の一覧に「本体の外の crate の説明だけ」の仲間として載せ、`runtime-crate-split` で直す。理由: C19 の約束で、この spec が本体の外で書き換えるのは要件 5 に挙げたファイルだけで、emo-text の名前は働きに響かない。
7. **規模の見積もり 20〜23 タスク**を、brief の 15〜18 から改めて受け入れるか。
   - **推奨**: 受け入れて 1 本のまま進める。理由: 増えた分は要件の段で見つけた参照の切り離しそのもので、分けると改名と切り離しが別の PR にまたがり、各段階で main が緑のままという進め方（roadmap の「本体の crate 分け」）が崩れる。記憶の決めごと「一度切り出した spec を、上限 20 を少しまたぐ理由でさらに削らない」にも合う。

## 設計の段の調べと決め（2026-11-13・`/kiro-spec-design -y`）

- **測った木**: 枝 `claude/kiro-start-runtime-module-untangle-2bc9c5` の `b172cab8`（コードは main `4863cb66` と同じ）。
- **調べの種類**: light（既存の本体の中の付け替え。新しい外の依存なし）。外の資料は要らない（前例はリポジトリの中にある）。
- **使った手引き**: `.claude/skills/kiro-spec-design/rules/` の design-principles・design-discovery-light・design-synthesis・design-review-gate。ほかの skill（UI・アクセシビリティほか）は当たらない。

### 9. 設計の段で確かめたこと

#### 9.1 `GhostSlot`・`BootContext`・`SwitchInFlight`・`ReloadInFlight` を `runtime` が読む所（4.1・4.3・4.4 の測り直し）

| 残るファイル | 読むもの | 使い方 |
|---|---|---|
| `frame/status_report.rs` 148・253 | `GhostSlot` → `seriko_sink()`・`kanade()` | 送るだけ |
| `balloon_visibility_phase.rs` 764 | `GhostSlot` → `kanade()` | 送るだけ |
| `frame/attach.rs` 456 | `GhostSlot` → `seriko_sink()` | 事実を seriko へ |
| `frame/scale_notice.rs` 60 | `GhostSlot` → `kanade()` | 送り手の写し |
| `shell_carry.rs` 325（`replay`） | `GhostSlot` → `seriko_sink()` | 面を出し直す |
| `frame/window_state.rs` 200 | `BootContext.current.ghost.folder` | 記録の欄（無ければ `-`） |
| `frame/switch.rs` | `kanade()`（218・599）・`seriko_sink()`（307）・`memory_publisher()`（586）・`set_shell_dir()`（500）・`ghost_name()`／`ghost_dir()`（621）・`SwitchInFlight` の有無（255）・`reload::{skin_is_reload, skin_applied}`（514・554）・`reload::skin_phase_ended`（78）・`BootContext.current.balloon` の書き換え（558）・`record_last_shell`／`record_last_balloon`（518・573） | 後始末の段と待ちの段 |

`GhostSession` の読み口の形は `kanade(&self) -> Option<&Sender<KanadeMsg>>`・`seriko_sink(&self) -> Option<&SerikoSink>`・`memory_publisher(&self) -> Option<&SylphyaPublisher>`・`set_shell_dir(&mut self, PathBuf) -> bool`・`ghost_name(&self) -> Option<String>`・`ghost_dir(&self) -> &Path`（`ghost_session.rs` 477〜612 行目）。

#### 9.2 前例 `CursorQuery`

`crates/wintf/src/ecs/cursor/mod.rs` 53 行目 `#[derive(Resource, Clone, Copy)] pub struct CursorQuery(pub fn(&World, Entity, &CursorAsk) -> Option<CursorReply>);`。下の層が型を持ち、上の層（`input_events/cursor_answer.rs` 72 行目）が `insert_resource(CursorQuery(answer_char_cursor))` で据える。関数のポインタは `Send + Sync + Copy` なので、`NonSend` ではなく `Resource` で足りる。

#### 9.3 終わる相の登録と記録の分け方

- 登録は `ghost_session::register_systems`（67〜69 行目）が `register_emo2_frame_system` → `wire_kanade_stop` の順に呼ぶ。`wire_kanade_stop` は `ghost_quit_system.before(emo2_frame_system)` を `Update` へ足す。
- 終わる相の記録（`ghost_quit`・`ghost_quit_no_windows`・`ghost_quit_extra`）の出所は今 `areka::emo2_boot::frame`。`log_history::RULES`（`log_history.rs` 115〜132 行目）のどの行にも当たらない（`areka` は完全一致だけ）ので、記録に残らない。
- 相を `session` の別のモジュール `session::ghost_quit` へ移せば、出所は `areka::session::ghost_quit` で、やはりどの行にも当たらない（同じ分け方）。`session::ghost_switch` の中へ置くと、決まり `areka::…::ghost_switch`（Status）に当たって記録に残るようになる＝分け方が変わるので置かない。

#### 9.4 cue の受け口を持ち主へ移したときの記録の分け方（要件 4.3 の確かめ）

- `install_cue.rs` 65 行目の `info!(event = "readonly_refused", what = "install_url", …)` は、今の出所 `areka::emo2_boot::install_cue` では決まりに当たらず記録に残らない。`install/install_cue.rs` へ移すと出所は `areka::install::install_cue` になり、決まり `rule("areka::install", Kind::Update, false)` に当たって**更新の記録として残るようになる**。
- `update_cue.rs`・`readme_cue.rs` は `warn!`・`debug!` だけで `info!` が無い（`warn!` は出所によらず誤りの種別・`debug!` は残らない）ので変わらない。`install_cue.rs` の `warn!`・`debug!` も同じ。
- 決め: `log_history.rs` に「残さない出所」の完全一致の表（1 行 `areka::install::install_cue`）を足し、`RULES` を当てる前に引く。要件 4.2（決まりを移した先の出所の名前へ直す）の中で、要件 4.1（出所は移した先のモジュールのパス）と 4.3（同じ分け方）の両方を満たす。`RULES` の行の数（14）とテストの名前（`rules_table_is_the_designed_fourteen_rows_in_order`）は変わらない。
- 手順書 `doc/pasta-real-run.md` 127・141 行目の `RUST_LOG` にある `areka::install=debug` は、移した後は `install_cue` の `debug!` も開く（出所の名前が変わることの帰結＝要件 4.1 のとおり）。手順書の目的（バルーンの決め方の分岐）には響かないので直さない。

#### 9.5 部分を `runtime` へ下ろすもの・`runtime` に残すもの（要件 2.1 の原則の当てはめ）

| 部分 | 今の場所 | 本体を引くか | 決め |
|---|---|---|---|
| `SkinKind`・`SkinRequestRaw`・`SkinSpec`・`SkinOrigin`・`SkinRequest`・`SkinVerdict`・`SkinSwitchInFlight`・`SkinSwitchStage`・`SwapFinish`（46〜158 行目）・`skin_ref_name`・`skin_ref_path`（444〜457 行目） | `shell_balloon_switch.rs` | 引かない | `runtime/skin_switch.rs`（新しいファイル・約 130 行）。残り（要求の入口 159〜443 行目・約 285 行）が行の多い側＝元のファイルの移動として `session/shell_balloon_switch.rs` へ |
| `ChangeRequestRaw` | `ghost_switch.rs` 46 行目 | 引かない | `runtime/change_cue.rs` |
| `CueSinkSources`（41〜58 行目）・`RuntimeBootError`（61〜66 行目） | `runtime_boot.rs` | 引かない（中身は下の crate と `runtime` の型） | `runtime/mod.rs`。残り（`runtime_sinks`・`boot_runtime`・`runtime_boot_options`・`install_lines`・`RuntimeLines`）が行の多い側＝元のファイルの移動 |
| `capture_for_reload` の門（225 行目 `reload::carries_shell_state`） | `shell_carry.rs` | 門だけが引く | 門を呼び手（`session/ghost_switch.rs` 650 行目）へ出す。関数は `runtime` に残り、引数 `carries: bool` を受ける。`hand_over` は `RuntimeWiring` だけを触る＝本体を引かないので `runtime` に残す（要件 2.1 の「今の測りに当てはめると」の列は `capture_for_reload`・`hand_over` を移すと書くが、同じ要件の「設計の段の測り直しで頼る・頼らないが変わったものは、この原則で振り分ける」に従う） |
| `ShellBoxHover`・`settle_box_hover` | `input_events/shell_box.rs` 138・163 行目 | 引かない（`areka_emo_compose`・`areka_emo_text` の型だけ） | `runtime/shell_box_hover.rs`。`shell_box.rs` は `pub(crate) use` で出し直す（呼び手とテストの書き換え 0） |
| `BalloonWiring` のホバーの集合（`balloon_hover`）と読み書きの 3 つ | `input_events/balloon.rs` 149・202〜220 行目 | 引かない | `runtime/balloon_hover.rs` の `BalloonHover`（NonSend）。メソッドの名前は同じ（`is_balloon_hovered`・`set_balloon_hover`・`clear_balloon_hover`） |
| `ScopeScaleTable`・`SharedScopeScaleTable`・`ScopeScales`・`percent_text`・`scaling_text` | `ghost_session_scope_scale.rs` 17〜48・146〜165 行目 | 引かない（`target_map::TargetWindow` と `areka_emo_present` の型だけ） | `runtime/scope_scale.rs`。`ghost_session_scope_scale.rs` は `wire`・`ScopeScaleSource`・`ScaleQuantity` を持ち、表と関数を `pub(crate) use` で出し直す |
| `special_name.rs`・`reload_args.rs`・`shell_balloon_resolve.rs`・`shell_carry.rs`（門のほか） | `emo2_boot` | 引かない | `runtime` に残す（議題 1 の確定どおり） |

#### 9.6 見える範囲の広げ（R3）

- 移すファイル（本番とテスト）が `pub(in crate::emo2_boot)` の項目を触る所は **1 か所**（`ghost_switch_reload_tests.rs` が `RuntimeWiring` の欄 `shell_carry` を読む）。31 項目のほかは触らない（`grep` で確かめた）。
- `mod` が私有で、移すファイルが引くもの: `change_cue`・`reload_cue`・`switch_cue`・`window_state_cue`・`special_name`。`pub(crate) mod` へ広げる。
- 終わる相の試験 3 本（`frame_ghost_quit_tests.rs` 300 行・`frame_ghost_quit_logsink_tests.rs` 110 行・`frame_ghost_quit_switch_tests.rs` 290 行）は `use super::*;` で `frame` の名前の束を拾っている。移した先で要る `use` を足す。
- 広げるのは `pub(crate)` までとし、コンパイルが求めた分だけにする。広げた項目は完了のときにこの記録へ一覧で足す。

#### 9.7 口を据える所とテストの World（R2）

- `GhostSlot` を置く所は 46 ファイル・65 か所、`BootContext`・`SwitchInFlight`・`ReloadInFlight` まで入れると 57 ファイル・109 か所（本番とテストの合計・`grep` の下限）。
- 決め: 口の束（`SessionHooks`・`Resource`）は本番では `ghost_session::register_systems` の先頭で 1 度据える（`main.rs` 257 行目＝`GhostSlot` を置く 532 行目より前・最初の巡より前）。口が据わっていない World での読み口は、`#[cfg(test)]` では本体の口そのもの（`session::hooks::HOOKS`）、本番では「本体の状態が何も無い」と同じ答え（`None`・偽・空の文字・何もしない）に倒す。
  - テストの World は今日と同じく本体の状態をそのまま読むので、57 ファイルに据え付けの 1 行を配らずに済み、振る舞いの差が出ない。
  - 本番の倒れ先は、今日の「`GhostSlot` が無い」「読み直しの印が無い」「起動の文脈が無い」の答えと同じで、その先の記録（`skin_shell_dir_not_set` ほか）も同じ。ただし本番では据え付けが最初の巡より前に済むので、倒れ先へは来ない。据え付けを外していないことは、既存の試験 `boots_twice_in_one_process_without_double_registration`（`ghost_session_restart_tests.rs` 445 行目）に `contains_resource::<SessionHooks>()` の assert を 1 行足して固定する（テストの数は変えない）。
  - `#[cfg(test)]` の側は `runtime` のテストのコードから本体を引く所として要件 3.5 の数に入れ、`runtime-crate-split` へ申し送る（crate が分かれると引けなくなる）。
- ホバーの集合 `BalloonHover` は、`BalloonWiring` を置く所（本番は `input_events/balloon.rs` 450 行目の `wire_balloon_choice`・テストは 13 ファイル 45 か所）に並べて置く。置かないと `observe_hover_wiring` の `error!` の条件が変わる（R5）。置き忘れは `grep` で 0 を示す（`BalloonWiring` を置くファイルの集合 ⊆ `BalloonHover` を置くファイルの集合）。

#### 9.8 `RuntimeInputs` の欄（R6）

`ghost_root`・`balloon_root` は `wire_runtime` の手順 1（資産の組立）が使い、`shiori`・`ticker`・`app_profile_dir`・`boot_origin` は起動（session の `boot_runtime`）だけが使う。決め: 欄は 6 つとも残し（名前だけ改める）、`wire_runtime` は受けた `RuntimeInputs` を今と同じく値のまま起動の関数へ渡す。呼び手（`ghost_session.rs` の `GhostBootInputs.wiring` と試験）の形は変わらない。

#### 9.9 1,000 行の見込み（R7）

| ファイル | 今 | 見込み | 理由 |
|---|---:|---:|---|
| `main.rs` | 989 | 991 以下 | `mod session;` と説明 1 行 |
| `ghost_session.rs` | 945 | 947 以下 | 口の据え付け 1 行・パスの書き換え |
| `runtime/mod.rs` | 833 | 約 860 | `CueSinkSources`・`RuntimeBootError`・起動の関数の型が入り、移すモジュールの宣言と `wire_kanade_stop` が出る |
| `session/ghost_switch.rs` | 934 | 934 以下 | `ChangeRequestRaw` が出て、門の 1 行が変わる |
| `frame.rs` | 671 | 約 580 | 終わる相が出る |
| `frame/switch.rs` | 703 | 約 690 | `session()` が口に替わる |
| `balloon_visibility_phase.rs` | 852 | 852 前後 | 読む型が替わるだけ |

テストのファイルで 1,000 行の近くのもの（例外表の `frame_transition_branch_tests.rs` ほか）に行を足すときは、足した後の行の数を完了の検証で数える。

#### 9.10 要件 3.3 の数え方（R1）

宣言の木をたどる道具は作らず、`rg`（PCRE2）の式 3 本と、本番のファイルの集合を決める除外の glob で数える。

- 本番のファイルの集合: `crates/areka/src/runtime/` の `*.rs` から、`*_tests.rs`・`*test_support*.rs`・`spine*.rs`・`*_cases.rs` を除いたもの。着手の測りで、この集合が「親が `#[cfg(test)]` で宣言していないファイル」の集合と一致することを、`mod` の宣言の一覧と突き合わせて 1 度確かめる（一致しなければ glob を足す）。
- 式 1（`crate::` で外を指す）: `rg -n --pcre2 <glob> '^(?!\s*//).*\bcrate::(?!runtime\b)' crates/areka/src/runtime`
- 式 2（直下のファイルから外へ抜ける `super::`）: `rg -n --pcre2 --max-depth 1 <glob> '^(?!\s*//).*\bsuper::super::' crates/areka/src/runtime`
- 式 3（`frame/` の下から外へ抜ける `super::`）: `rg -n --pcre2 <glob> '^(?!\s*//).*\bsuper::super::super::' crates/areka/src/runtime/frame`
- 3 本の出力が 0 行であることを示す。0 行でなければ、残った行が全部 `#[cfg(test)]` の項目の中にあることを行ごとに示し、その行は要件 3.5 の数へ回す。`use crate::{…}` の束ねた形は式 1 に当たるので、`runtime` の本番では使わない。
- 説明の文（要件 3.4）: 式 1〜3 の `^(?!\s*//)` を `^\s*//` に替えた 3 本で数えて一覧にする。
- テストのコード（要件 3.5）: 同じ式をテストのファイルの集合（上の除外の glob で除いたもの）と、本番のファイルの中の `#[cfg(test)]` の項目に当て、`rg -c` でファイルごとの数にする。

### 10. 合成（design-synthesis）

- **一般化**: 本体の状態を `runtime` から読む所（9.1 節の 7 ファイル）は全部「World から本体の状態を同期で引く」同じ形なので、1 つの口の束（`SessionHooks`・関数のポインタ 10 本）にまとめた。終わりの段だけを口にする案（研究 4.3 の案 A の「後始末の口」2 本）より本数は増えるが、口の 1 本ずつが今の 1 か所の読み書きと同じ答えを返すので、記録の文と出所が `runtime` に残り（要件 4 の書き換えが増えない）、口の無い World での答えが今の「状態が無い」と一致する。
- **作るか借りるか**: 前例 `wintf::ecs::cursor::CursorQuery`（`Resource` の関数のポインタ）をそのまま借りる。特性の箱（研究 4.1 の案 C）は採らない。知らせの線（研究 4.2 B・4.3 C）は 1 フレーム遅れるので採らない。
- **削ったもの**: 口の据え付けを各テストへ配る案（57 ファイル）・`wire_runtime` を 2 つに分ける案・`RuntimeInputs` の欄を分ける案・新しい試験の関数（要件 8.1 の数を変えない。足すのは既存の試験の assert だけ）。

### 11. 決め（Design Decisions）

- **D1 終わる相は相ごと `session::ghost_quit` へ移す**（研究 4.2 A・議題 2）: system と `.before(frame_system)` の登録と呼ぶ順が今と同じ＝1 フレームの遅れも終わる段取りの順の変化も構造の上で起きない。モジュールを `ghost_switch` と分けるのは 9.3 節の記録の分け方のため。
- **D2 差し替えの相は `runtime` に残し、本体の読み書きを口で引く**（研究 4.3 A・議題 3）: 口は今の 1 か所ごとの読み書きと 1 対 1（`set_shell_dir`・`set_current_balloon`・`record_last_skin`・`reload_skin_applied`・`skin_phase_ended`・`ghost_switch_in_flight`・`kanade`・`seriko_sink`・`ghost_ref_name`）。後始末の順は今の関数の中の並びのまま。
- **D3 `GhostSlot`・`BootContext` は口で読む**（研究 4.1 A・4.4 A・議題 4）: 口は持たず毎回 World から借りる＝seriko の送り手の持ち主は `GhostSession` だけのまま（areka-P0-shell-balloon-switch 要件 8.5）。
- **D4 `wire_runtime` は起動の関数を引数で受ける**（研究 4.7 A・議題 5）: 型は `fn(&mut World, RuntimeInputs, &CueSinkSources, Sender<KanadeNotice>, Option<&str>) -> Result<GhostRuntime, RuntimeBootError>`（関数のポインタ）。総称にしないのは、`frame_schedule_tests`・`zorder_wiring_tests` が字面 `pub fn wire_runtime(` を照合するため。
- **D5 部分を分けるファイルと下ろす型**: 9.5 節の表のとおり。
- **D6 `install_cue` の `info!` の分け方**: 9.4 節のとおり、`log_history` に「残さない出所」の完全一致の表を足す。
- **D7 口の無い World**: 9.7 節のとおり、`#[cfg(test)]` は本体の口、本番は「状態が無い」の答え。

### 12. 危うさと手当て

- 口の束の本番の倒れ先は試験で踏まれない → 倒れ先の答えは今の「状態が無い」と同じ定数で、据え付けは既存の試験の assert で固定する。
- `#[cfg(test)]` の倒れ先は crate が分かれると引けない → `runtime-crate-split` の brief へ申し送る（要件 7.2）。
- ホバーの集合の置き忘れで `error!` の条件が変わる → 置く所の集合の包含を `grep` で示す。
- 終わる相の試験 3 本は `use super::*;` に頼る → 移した後のコンパイルで要る `use` を足すだけ（テストの名前は変えない）。
- `install_cue` の記録の分け方 → 9.4 節の表で同じに保ち、既存の試験 `unrelated_info_is_not_kept` に assert を足して固定する。

### 13. 要件 1.3 の判定の下ごしらえ（`fn` の名前に emo2 を含むもの・`crates/areka`・設計の段の測り）

- 結線を指す（`runtime` の語へ改める）: `emo2_frame_system_drives_text_scale_phase_every_frame`・`emo2_frame_system_removes_runs_and_reinserts_wiring`・`emo2_frame_system_runs_dpi_phase_without_writes_when_unattached`・`leave_clears_balloon_hover_even_without_emo2_wiring`・`move_cue_sink_reaches_emo2_wiring_receiver`・`moved_without_emo2_wiring_does_not_record_hover`・`talk_playback_reaches_emo2_wiring_lifecycle_receiver`・`wire_emo2_boot_falls_back_to_unwired_on_missing_ghost_root`・`without_emo2_wiring_does_nothing`・`headless_emo2_wiring`（支え）・`emo2_frame_system`・`register_emo2_frame_system`・`wire_emo2_boot`。
- 実在のゴースト emo2 を指す（変えない）: `acquire_emo2`・`emo2_root`・`emo2_balloon`・`emo2_balloon_root`・`emo2_sample`・`emo2_first_load_repeat`・`emo2_real_run_boots_talks_and_exits_zero`・`both_cage_surfaces_compose_on_the_emo2_sample`・`build_boot_assets_*emo2*` 3 本・`emo2_adjust_aligns_bottom_right_to_bottom_left`・`emo2_bakes_each_still_image_once_per_stage`・`emo2_scripts_read_without_notes`・`ghost_default_emo2_when_listed_among_many`・`ghost_only_emo2_is_only_not_default`・`legacy_emo2_lands_where_it_did_before`・`stay_on_top_directives_on_emo2`。
- 名前だけでは決まらない（実装のときに使われ方で判定し、ここへ理由を足す）: `emo2`（2 つ）・`with_emo2`・`spawn_emo2`・`emo2_sink_of`・`leave_emo2_absent_degrades_with_debug_and_leaves_hover`・`moved_emo2_absent_degrades_with_debug_not_error`・`pressed_emo2_absent_degrades_with_debug_not_error`（「emo2 が無い」が `Emo2Wiring` の不在なら結線）。
- 本体の外の crate: `crates/areka-emo-text` の `EMO2_BOOT_TALK`・`EMO2_BOOT_FINAL`（emo2 の起動の台詞の写し＝実在のゴースト emo2 を指す検体）と `emo2_boot_talk_reveals_without_moving_glyphs`（本体の外の crate のテストの名前＝Out of scope）。

### 14. 設計の見直しの門（design-review-gate）

- 機械の確かめ: 要件の番号 52 個が全部 design.md の対応表に在る・境界の 4 節が埋まっている・File Structure Plan に具体のパスがある・部品は全部ファイルに当たる。
- 1 回目の直し: 要件 2.1 の当てはめの列から外れる `shell_carry` の扱いの理由を design.md に書き足した・申し送り（7.1〜7.4）の節を足した・`super::` の深さの読み直しを数え方に足した・検証の記録の置き場（本書）を File Structure Plan に足した。2 回目で通った。
- 要件の穴として差し戻すものは無い（`install_cue` の記録の分け方は要件 4.2 の中で 4.1 と 4.3 を両立させた＝9.4 節）。

## 設計の討議（2026-11-13）

- 検証の指摘 2 つ（式 2・3 の宣言の深さ・本番の倒れ先の記録）と、要件 2.1 の `shell_carry` の列の食い違いは、自明な修正として design.md と requirements.md を直した。
- 議題 1（`runtime` のテストで口が据わっていないときの倒れ先）: 推奨案で確定。`#[cfg(test)]` の `FALLBACK` は `session::hooks::HOOKS`。57 ファイルへ据え付けを配らない。テストが本体を引く所は要件 3.5 の数に入れ、`runtime-crate-split` へ申し送る。
- 議題 2（`install_cue` の記録の分け方）: 推奨案で確定。`log_history.rs` に完全一致の「残さない出所」の表 `UNKEPT_TARGETS`（`areka::install::install_cue` の 1 行）を足す。要件 4.1（出所はモジュールのパス）と 4.3（前と同じ分け方）を両立する唯一の形。

## 着手の測り（2026-11-13・タスク 1.1・要件 8.1・8.2）

### 15. テストの名前の多重集合と警告の数（前）

- 測った木: ブランチの土台 `4863cb667b9dd62b6d8dad5375b2bb1c09151da4`（`git merge-base HEAD origin/main`）。着手のときの `origin/main` は `2f3e51040d369c20d42b9e2d8ce8c8488f6462a1` で土台より先へ進んでいるが、このブランチは土台から `crates/`・`tools/` を変えておらず（`git diff --stat origin/main...HEAD -- crates tools` が空）、測ったのは土台のコードそのもの。完了の測り（9.4 の比べ）は、取り込んだ後の土台との差をここに足して読む。
- 道具: `cargo 1.100.0 (3acc79c0c 2026-11-05)`。
- テストの名前の多重集合の置き場: `.kiro/specs/runtime-module-untangle/test-name-tails-before.txt`（1 行 1 名・重複はそのまま残す・バイト順に並べた）。
  - 行の数（テストの数）: 13,926。異なる名前の数: 13,813。
  - SHA-256: `c9c83edf36e384042924c2cca7f86287f4e46f3a5d862190cc5311fdc5f00bd8`
  - 作り方（Git Bash・ワークツリーの根で）:
    ```sh
    cargo test --workspace --no-fail-fast -- --list > list.txt
    tr -d '\r' < list.txt | grep ': test$' | sed 's/: test$//; s/.*:://' | LC_ALL=C sort > test-name-tails-before.txt
    ```
  - 末尾は最後の `::` から後ろ（要件 8.1）。doc テストの行（65 行・`crates\…\x.rs - a::b (line N)`）は `b (line N)` か、`::` を含まなければ行全体が残る。`crates/areka` に doc テストは 0 行なので、本仕様のパスの改めで doc テストの名前は変わらない（行の番号がずれる所は `crates/areka` の外だけ）。
  - `: benchmark$` の行は 0 行。
- 警告（要件 8.2）: `cargo check --workspace --all-targets` は成功（終わりの値 0）。rustc の警告（コードの警告）は **0 件**。
  - ただし cargo のマニフェストの警告が 3 件出る（今の main のまま・本仕様の前からあるもの・cargo 1.100 の使われない依存の検査）:
    - `unused workspace dependency async-executor`（ワークスペースの `Cargo.toml`）
    - `unused dependency windows`（`shiori-abi`）
    - `unused dependency event-listener`（`shiori-host32-host`）
  - 本仕様が触るのは `crates/areka` の中なので、この 3 件は数を変えずに完了まで持つ（完了の比べでは「rustc の警告 0 件・マニフェストの警告はこの 3 件のまま」で読む）。この 3 件を消すかどうかは進行役の判断へ回す。
- 整形: `cargo fmt --all -- --check` は差なし。

## 外の参照の数え方の較正（2026-11-13・タスク 1.3・要件 3.1・3.3）

### 16. 本番のファイルの集合・D1・D2 と式 1〜3 の今の数（前）

- 測った木: `c78dfa3c6`（`crates/` は 15 節の土台 `4863cb66` のまま）。モジュールはまだ `emo2_boot`（改名はタスク 2.1）なので、9.10 節と design.md「外の参照の数え方」の式は **`crates/areka/src/runtime` を `crates/areka/src/emo2_boot` に、式 1 の `crate::(?!runtime\b)` を `crate::(?!emo2_boot\b)` に置き換えて**当てた。9.1 で測り直すときは置き換えを戻す（下のコマンドの `B` と式 1 の先読みの 2 か所）。
- モジュールの根は `main.rs:58` の `mod emo2_boot;`（`crates/areka` の bin）。

#### 16.1 本番のファイルの集合の確かめ（9.10 節の 1 度だけの突き合わせ）

- 宣言の木は、下の使い捨ての読み手（リポジトリには置かない・一時フォルダで回す）で `mod.rs` から `mod x;` の宣言をたどって作った。`#[path = "…"]` は宣言したファイルのディレクトリから、`#[path]` の無い宣言は Rust の規則どおり（`mod.rs` ならそのディレクトリ・`x.rs` なら `x/`）に解く。直前の属性に `#[cfg(…test…)]` があるか、親が既にテストなら「テスト」とする。
- 結果: 宣言されたファイル 236 本＝ディレクトリの `*.rs` 236 本（宣言されていないファイル 0 本・行頭に字下げのある `mod x;`＝インラインの `mod … { }` の中の宣言 0 本）。
  - 深さごとの本数（深さ 0 は `mod.rs`）: 本番＝深さ 0 が 1 本・深さ 1 が 38 本・深さ 2 が 22 本（計 61 本）。テスト＝深さ 1 が 8 本・2 が 125 本・3 が 36 本・4 が 5 本・5 が 1 本（計 175 本）。
- 除外の glob（`-g '!*_tests.rs' -g '!*test_support*.rs' -g '!spine*.rs' -g '!*_cases.rs'`）で残る 61 本と、「親が `#[cfg(test)]` で宣言していない」61 本は **一致**（`diff` が空）。glob は足さない。
- 深さ 3 以上の本番のファイルは **0 本**。式を足す必要は無い。
- 深さ 0 の `mod.rs` から `super::` で外へ抜ける行（`'^(?!\s*//).*\bsuper::'` を `mod.rs` に当てたもの）は 2 行（`mod.rs:741`・`mod.rs:742`）で、どちらも `#[cfg(test)] mod wire_tests { … }` の中の `use super::…`（`emo2_boot` 自身を指す）＝本番の外への参照ではない。

#### 16.2 D1（宣言の深さ 1 の本番のファイル・38 本）

`adapter.rs` `assets.rs` `balloon_background.rs` `balloon_visibility.rs` `change_cue.rs` `consumer_ledger.rs` `frame.rs` `ghost_switch.rs` `hit_region.rs` `hover_inject.rs` `install_cue.rs` `move_cue.rs` `readme_cue.rs` `reload.rs` `reload_args.rs` `reload_cue.rs` `runtime_boot.rs` `same_folder_pin.rs` `script_restriction.rs` `shell_balloon_resolve.rs` `shell_balloon_switch.rs` `shell_box_assets.rs` `shell_carry.rs` `shell_cursor_assets.rs` `shell_cursor_table.rs` `skin_facts_copy.rs` `special_name.rs` `strict_judges.rs` `switch_assets.rs` `switch_copy.rs` `switch_cue.rs` `talk_clock.rs` `talk_lifecycle.rs` `target_map.rs` `update_cue.rs` `user_break_cue.rs` `window_state_cue.rs` `zorder_cue.rs`

#### 16.3 D2（宣言の深さ 2 の本番のファイル・22 本）

- `balloon_visibility.rs` が `#[path]` で宣言: `balloon_visibility_decision.rs` `balloon_visibility_phase.rs` `balloon_visibility_wait.rs`
- `move_cue.rs` が `#[path]` で宣言: `move_cue_args.rs` `move_cue_geometry.rs` `move_cue_place.rs`
- `frame.rs` が宣言: `frame/attach.rs` `frame/balloon_face.rs` `frame/balloon_offset_follow.rs` `frame/dpi.rs` `frame/drain_resnap.rs` `frame/move_hold.rs` `frame/move_shift.rs` `frame/scale_notice.rs` `frame/scale_text.rs` `frame/status_report.rs` `frame/switch.rs` `frame/window_state.rs` `frame/wiring.rs` `frame/work_area_sync.rs` `frame/zorder_descript.rs` `frame/zorder_drain.rs`

#### 16.4 式 1〜3 の今の行数（外の参照だけ）

| 式 | 当てた所 | 今の行数 |
|---|---|---|
| 式 1 `crate::(?!emo2_boot\b)` | 本番の 61 本 | **60 行** |
| 式 2 `super::super::` | D1 の 38 本 | **0 行** |
| 式 3 `super::super::super::` | D2 の 22 本 | **0 行** |

- 式 1 の 60 行のファイルごとの内訳: `ghost_switch.rs` 16・`reload.rs` 9・`runtime_boot.rs` 8・`shell_balloon_switch.rs` 5・`install_cue.rs` 3・`frame/switch.rs` 3・`balloon_visibility_phase.rs` 3・`readme_cue.rs` 2・`frame/scale_notice.rs` 2・`update_cue.rs` 1・`switch_copy.rs` 1・`shell_carry.rs` 1・`same_folder_pin.rs` 1・`frame/wiring.rs` 1・`frame/window_state.rs` 1・`frame/status_report.rs` 1・`frame/attach.rs` 1・`frame.rs` 1。
  - 60 行は全部 `emo2_boot` の外（`app_exit`・`boot_config`・`boot_resolve`・`exit_wait`・`ghost_session`・`input_events`・`install`・`log_history`・`readme`・`readonly`・`roots`・`session_end`・`update`・`write_failure`・`crate::record_last_used`・`crate::is_benign_boot_error`）を指す。インラインのテストのモジュール（`adapter.rs:259`・`hit_region.rs:121`・`hover_inject.rs:207`・`mod.rs:740`・`talk_clock.rs:138`・`target_map.rs:62`）を持つファイルには 1 行も当たらない＝60 行は全部本番の項目の中。
  - 束ねた形 `use crate::{…}` は 0 行。複数行の `use crate::x::{` は 1 宣言 1 行で数える（`ghost_switch.rs:34`・`:38`）。
- `emo2_boot` の中を指す `super::super::` が出ないことを確かめた: D2 で `super::super::` のあとに `super::` が続かない行は 11 行（`balloon_visibility_phase.rs:47` の `super::super::frame`・`:48` の `super::super::target_map`・`frame/move_shift.rs:22`・`frame/move_hold.rs:23`・`:24`・`frame/wiring.rs:25`〜`:30`）で、どれも深さ 2 から `emo2_boot` の中（`frame`・`target_map`・`move_cue`・`balloon_visibility`・`shell_carry`・`switch_assets`・`talk_lifecycle`・`window_state_cue`・`zorder_cue`）を指す。式 3 はこの 11 行を拾わず、式 2 は D1 だけに当てるのでこれも拾わない。
- よって、要件 3.1 で 0 にする今の外の参照は **式 1 の 60 行だけ**（式 2・式 3 は今から 0 行）。

#### 16.5 測り直しのコマンド（Git Bash・ワークツリーの根で）

```sh
B=crates/areka/src/emo2_boot              # 改名の後は crates/areka/src/runtime
G=(-g '!*_tests.rs' -g '!*test_support*.rs' -g '!spine*.rs' -g '!*_cases.rs')
python -I modtree.py "$B" > tree.txt      # 下の読み手。行は「深さ P|T 相対パス」
awk '$2=="P"{print $3}' tree.txt | sort > prod.txt
(cd "$B" && rg --files --path-separator // "${G[@]}" | sort) | diff - prod.txt && echo SAME
D1=$(awk -v b="$B" '$1==1&&$2=="P"{print b"/"$3}' tree.txt)
D2=$(awk -v b="$B" '$1==2&&$2=="P"{print b"/"$3}' tree.txt)
awk '$1>=3&&$2=="P"' tree.txt             # 空なら式を足さない
rg -n --pcre2 "${G[@]}" '^(?!\s*//).*\bcrate::(?!emo2_boot\b)' "$B"   # 式 1（改名の後は runtime）
rg -n --pcre2 '^(?!\s*//).*\bsuper::super::' $D1                       # 式 2
rg -n --pcre2 '^(?!\s*//).*\bsuper::super::super::' $D2                # 式 3
rg -n --pcre2 '^(?!\s*//).*\bsuper::' "$B/mod.rs"                      # 深さ 0（16.1 の確かめ）
```

使い捨ての読み手 `modtree.py`（一時フォルダに置いて回す・リポジトリには置かない）:

```python
import os, re, sys
root = sys.argv[1]
decl = re.compile(r'^(\s*)(pub(\([^)]*\))?\s+)?mod\s+(\w+)\s*;')
out = {}
def walk(f, depth, test, own_dir):
    out[os.path.relpath(f, root).replace(os.sep, "/")] = (depth, test)
    lines = open(f, encoding='utf-8').read().splitlines()
    for i, l in enumerate(lines):
        m = decl.match(l)
        if not m: continue
        if m.group(1): print('INDENTED', f, i + 1, file=sys.stderr)
        attrs, j = [], i - 1
        while j >= 0 and re.match(r'^\s*(#\[|//)', lines[j]):
            attrs.append(lines[j]); j -= 1
        t = test or any(re.search(r'#\[cfg\(.*\btest\b', a) for a in attrs)
        p = [re.search(r'#\[path\s*=\s*"([^"]+)"', a) for a in attrs]
        p = [x.group(1) for x in p if x]
        name = m.group(4)
        if p: child = os.path.join(os.path.dirname(f), p[0])
        else:
            child = os.path.join(own_dir, name + '.rs')
            if not os.path.exists(child): child = os.path.join(own_dir, name, 'mod.rs')
        cd = os.path.join(os.path.dirname(child), name) if not child.endswith('mod.rs') else os.path.dirname(child)
        walk(os.path.normpath(child), depth + 1, t, cd)
walk(os.path.join(root, 'mod.rs'), 0, False, root)
for k, (d, t) in sorted(out.items()): print(d, 'T' if t else 'P', k)
allf = {os.path.relpath(os.path.join(a, b), root).replace(os.sep, "/") for a, _, fs in os.walk(root) for b in fs if b.endswith('.rs')}
print('UNDECLARED', sorted(allf - set(out)), file=sys.stderr)
```

- 読み手の限り: インラインの `mod … { }` の中の `mod x;` は解かない（出たら `INDENTED` を出す・今は 0 件）。`#[cfg(any(test, …))]` も「テスト」に数える。
