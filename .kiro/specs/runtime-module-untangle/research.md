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

## 終わる道 3 つの記録の順（前）（2026-11-13・タスク 1.2・要件 3.6・8.5）

### 17. 実機で取った終わる段取りの行の順（前）

- 測った木: `9175b9e7c`（`crates/` は 15 節の土台 `4863cb66` のまま）。`cargo build -p areka` で作った `target\debug\areka.exe`（11-13 10:03）。隣に `shiori-host32-helper.exe` が在る。
- 走らせた前に 127.0.0.1:9276 を握るものが無いことを `netstat -ano` で確かめた。
- 3 つとも同じ `RUST_LOG` で走らせた:
  ```
  info,areka::app_exit=debug,areka::emo2_boot::frame=debug,areka::runtime::frame=debug,areka::session::ghost_quit=debug
  ```
  今の出所（`areka::emo2_boot::frame`）と移した後の出所（`areka::runtime::frame`・`areka::session::ghost_quit`）を両方名指しているので、9.6 でも同じ文字列のまま使う。名指した出所が木に無くても害は無い。
- 実機の根・記憶の置き場・写しは、どれもワークツリーの `target\test-roots\rmu-<道>\` の下に置いた。パスは絶対パスで渡した。
- 結果:

| 道 | pid | 引き金 | 終わり（`app_exit` の `origin`） | 終了コード |
|---|---|---|---|---|
| 煙の自動終了 | 6516 | `AREKA_APP_SMOKE_EXIT_MS=15000`（窓が開いてから 15 秒） | `Smoke`（closed=4） | 0 |
| 窓を閉じる | 33548 | 窓が開いてから 15 秒後に `taskkill /PID <pid>`（`/F` なし） | `KanadeStopped(Quit)`（closed=4） | 0 |
| メニューの終了 | 29844 | 窓が開いてから 15 秒過ぎに右クリック →「終了」 | `KanadeStopped(Quit)`（closed=4） | 0 |

#### 17.1 道ごとの行の順

列は「段 | 出所 | 文言 | event」。出所は tracing の target で、`actor{…}` の span は外した。文言は欄（`key=value`）より前の本文。行の抜き出し方は 17.3 節のとおり。

メニューの終了（`rmu-menu`）:

```
INFO | kanade | OnClose GET を発行し握手を開始 | close_handshake_begin
INFO | kanade | OnClose 応答スクリプト——close talk を再生起動し完了を待機 | close_talk_start
INFO | kanade | reason=Quit——終了系列（Quit）へ | talk_done_quit
INFO | shiori-actor | 正規 clean shutdown 完了（unload が正常終了を返した） | unload_clean
INFO | shiori-actor | 停止指示（Close）を受領——即時停止（接続資材を RAII teardown） | close
INFO | areka::emo2_boot::frame | kanade の終了系列が完了した: 全ゴースト窓を閉じる | ghost_quit
INFO | wintf::ecs::app | [App] Last window closed. |
INFO | areka::app_exit | [quit_app] 全窓を閉じ、終了を指示した | app_exit
INFO | wintf::runtime::message_loop | [AppExit] exit requested |
DEBUG | areka::emo2_boot::frame | 終了が指示済み——毎フレームの処理を読み飛ばす | frame_phases_skipped_after_exit
INFO | areka::ghost_session | seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止） |
INFO | ghost-shutdown | persist flush confirmed |
INFO | ghost-shutdown | ghost shutdown sequence completed |
INFO | areka_mcp::server | MCP: 待受を閉じた |
```

窓を閉じる（`rmu-close`）:

```
INFO | wintf::ecs::window_proc::lifecycle | [WM_CLOSE] 閉鎖要求を利用側の関数へ渡す | os_close_request
INFO | areka::app_exit | [os_close] OS の閉鎖要求をメニューの「終了」と同じ終了要求として kanade へ送る | os_close_request
INFO | kanade | close 指示——active talk あり・保留記録（TalkDone を待つ） | steady_close_pending
INFO | kanade | talk 完了——保留 close を消化し握手開始 | steady_talk_done_close
INFO | kanade | OnClose GET を発行し握手を開始 | close_handshake_begin
INFO | kanade | OnClose 応答スクリプト——close talk を再生起動し完了を待機 | close_talk_start
INFO | kanade | reason=Quit——終了系列（Quit）へ | talk_done_quit
INFO | shiori-actor | 正規 clean shutdown 完了（unload が正常終了を返した） | unload_clean
INFO | shiori-actor | 停止指示（Close）を受領——即時停止（接続資材を RAII teardown） | close
INFO | areka::emo2_boot::frame | kanade の終了系列が完了した: 全ゴースト窓を閉じる | ghost_quit
INFO | wintf::ecs::app | [App] Last window closed. |
INFO | areka::app_exit | [quit_app] 全窓を閉じ、終了を指示した | app_exit
INFO | wintf::runtime::message_loop | [AppExit] exit requested |
DEBUG | areka::emo2_boot::frame | 終了が指示済み——毎フレームの処理を読み飛ばす | frame_phases_skipped_after_exit
INFO | areka::ghost_session | seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止） |
INFO | ghost-shutdown | persist flush confirmed |
INFO | ghost-shutdown | ghost shutdown sequence completed |
INFO | areka_mcp::server | MCP: 待受を閉じた |
```

煙の自動終了（`rmu-smoke`）:

```
INFO | wintf::ecs::app | [App] Last window closed. |
INFO | areka::app_exit | [quit_app] 全窓を閉じ、終了を指示した | app_exit
INFO | wintf::runtime::message_loop | [AppExit] exit requested |
INFO | areka::smoke_exit | smoke 自動 close: ゴースト窓を despawn しました |
DEBUG | areka::emo2_boot::frame | 終了が指示済み——毎フレームの処理を読み飛ばす | frame_phases_skipped_after_exit
INFO | areka::ghost_session | seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止） |
WARN | kanade | 強制終了指示——終了系列（Forced）へ直行 | force_quit
INFO | shiori-actor | 正規 clean shutdown 完了（unload が正常終了を返した） | unload_clean
INFO | shiori-actor | 停止指示（Close）を受領——即時停止（接続資材を RAII teardown） | close
INFO | ghost-shutdown | persist flush confirmed |
INFO | ghost-shutdown | ghost shutdown sequence completed |
INFO | areka_mcp::server | MCP: 待受を閉じた |
```

#### 17.2 9.6 で比べるときの読み方

- 出所の読み替え（design.md「LogTargetFollow」）: 終わる相の 3 つの記録（`ghost_quit`・`ghost_quit_no_windows`・`ghost_quit_extra`）は `areka::emo2_boot::frame` から `areka::session::ghost_quit` へ移る。毎フレームの相の `frame_phases_skipped_after_exit` は `areka::emo2_boot::frame` から `areka::runtime::frame` へ移る。文言の改め（要件 1.1 の表）が終わる段取りの行に当たれば、ここに足して読む。
- 1 つのスレッドの中の順はそのまま比べる。主のスレッドの行は `areka::*`・`wintf::*`・`ghost-shutdown`・`areka_mcp::server` で、kanade の行と shiori-actor の行もそれぞれの中で比べる。actor（`kanade`・`shiori-actor`）の行と主のスレッドの行の前後は、走りごとに入れ替わることがある。煙の道の `force_quit`／`unload_clean` が `loop ticker を Close` の後に来るのも、終わりの統括が kanade を止める順による。
- 窓を閉じる道の `steady_close_pending`→`steady_talk_done_close` の 2 行は、閉じる要求が届いたときに台詞を再生中だった（起動の台詞が続いていた）から出た。再生中でなければこの 2 行は無く、`close_handshake_begin` から始まる（メニューの道がそうだった）。この 2 行の有る無しは段取りの順の違いではない。
- メニューの「終了」は areka の側に行を残さない（`menu::request_close` は記録を持たない）。最初の行は kanade の `close_handshake_begin`（再生中なら `steady_close_pending`）になる。

#### 17.3 抜き出し方（`exitlines.ps1`）

`stdout.log` の全部の行に固定の正規表現を当て、当たった行を記録の順に並べる。隣り合う同じ行を 1 つに畳む処理は置いていない（この 3 走では同じ行の並びが出なかった）。起動の記録は 1 行も当たらなかったので、引き金の行から切る処理も置いていない。`stderr.log` には `[helper] SHIORI 初期化の入口: loadu` の 1 行しか無い（tracing は全部 `stdout.log` に出る）。

```powershell
#Requires -Version 7
# Extract the exit-sequence lines of one exitrun.ps1 run: level | source | message | event, in log order.
param([string]$Path)
$run = "C:\home\maz\git\areka\.claude\worktrees\kiro-start-audit-emo-text-layout-draw-68feba\target\test-roots\rmu-$Path"
$events = 'os_close_request|steady_close_pending|steady_talk_done_close|close_handshake_begin|close_talk_start|close_talk_done_quit|talk_done_quit|force_quit|unload_clean|unload_non_clean|unload_failed|close|ghost_quit|ghost_quit_no_windows|ghost_quit_extra|app_exit|app_exit_again|frame_phases_skipped_after_exit'
$plain = '\[App\] Last window closed\.|\[AppExit\] exit requested|smoke 自動 close: ゴースト窓を despawn|loop ticker を Close|ghost-shutdown:|MCP: 待受を閉じた'
$re = "event=`"($events)`"|$plain"
foreach ($line in Get-Content "$run\stdout.log" -Encoding utf8) {
    if ($line -notmatch $re) { continue }
    # <time> <LEVEL> [span: ]<source>: <message> [fields]
    if ($line -notmatch '^\S+\s+(\w+)\s+(?:actor\{[^}]*\}:\s+)?([^\s:]+(?:::[^\s:]+)*|[\w-]+):\s+(.*)$') { continue }
    $level, $source, $rest = $Matches[1], $Matches[2], $Matches[3]
    $event = if ($rest -match 'event="([^"]+)"') { $Matches[1] } else { '' }
    $message = ($rest -replace '\s+\w+=.*$', '').Trim()
    "$level | $source | $message | $event"
}
```

走らせ方: `pwsh -NoProfile -File exitlines.ps1 -Path menu`（`close`・`smoke` も同じ）。9.6 で文言が改まった行は、`$plain` の綴りを改めた文言に合わせて読む。event の名前は変えない（要件 4）。

#### 17.4 走らせた道具（`exitrun.ps1`）と手順

煙の道と窓を閉じる道は、道具を前で走らせると終わりまで待って `exited pid=… code=…` を出す。メニューの道は道具を裏で走らせる。道具は窓が開いてから 15 秒待ち、その pid の見えている窓の矩形を出してから終わりを待つ。その間に手で次の操作をする。

1. 窓の一覧のうち下端が作業領域の下端に着くものがキャラの窓（今回は `rect=1392,904,2064,1704`）。`tools/mouse-input.ps1 -Action shot` で撮り、体の画素の点を決める（今回は (1592,1304)）。
2. **先にカーソルをその点へ動かす**（`-Action move`）。そのあと `WindowFromPoint` で窓の pid が子の pid と同じかを確かめる。キャラの窓はカーソルが乗るまでクリックを素通しにしているので、動かす前に確かめると下の窓（今回は explorer）が返る。
3. pid が同じなら `-Action rightclick`。1 秒待って `-Action shot` でメニューを撮り、「終了」（いちばん下の項目）の点を決める（今回は (1684,1280)）。
4. その点でも `WindowFromPoint` の pid が子の pid と同じか（メニューの窓も areka の窓）を確かめてから `-Action click`。

pid を確かめるのに使った式（PowerShell）:

```powershell
Add-Type -Namespace P -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(System.Drawing.Point p); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid); [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);' -ReferencedAssemblies System.Drawing.Primitives
[P.W]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
$h = [P.W]::WindowFromPoint([System.Drawing.Point]::new(<X>, <Y>)); $p = [uint32]0; [P.W]::GetWindowThreadProcessId($h, [ref]$p) | Out-Null; "hwnd=$h pid=$p"
```

道具の全文（走らせた形のまま）:

```powershell
#Requires -Version 7
# One exit-path real run of areka (runtime-module-untangle 1.2 / 9.6).
# -Path smoke|close|menu. start: boots and (smoke) waits, (close) taskkill without /F, (menu) prints windows and returns.
param([string]$Path)
$ErrorActionPreference = 'Stop'
$wt  = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-audit-emo-text-layout-draw-68feba'
$run = "$wt\target\test-roots\rmu-$Path"
$RustLog = 'info,areka::app_exit=debug,areka::emo2_boot::frame=debug,areka::runtime::frame=debug,areka::session::ghost_quit=debug'

function Wait-Exit {
    if (-not $child.WaitForExit(180000)) { Write-Output "timeout pid=$($child.Id)"; exit 1 }
    Write-Output "exited pid=$($child.Id) code=$($child.ExitCode) at $(Get-Date -Format o)"
}

if (Test-Path $run) { Remove-Item -Recurse -Force $run }
New-Item -ItemType Directory -Force "$run\root", "$run\profile" | Out-Null
Copy-Item -Recurse "$wt\target\nar-samples\manual\emo2" "$run\s"
$ghost   = "$run\s\ghost\emo2"
$balloon = "$run\s\ghost\emo2\balloon\emo2-kakukaku"
$l = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0); $l.Start(); $port = $l.LocalEndpoint.Port; $l.Stop()

$env:AREKA_ROOT = "$run\root"; $env:AREKA_PROFILE_DIR = "$run\profile"; $env:AREKA_MCP_PORT = "$port"
$env:AREKA_USER_ROOT = $null
$env:AREKA_NO_ALERT = '1'; $env:NO_COLOR = '1'; $env:PASTA_DEBUG = '0'; $env:RUST_LOG = $RustLog
# smoke fires 15 s after the first windows; the other two keep it as a 120 s safety exit.
$env:AREKA_APP_SMOKE_EXIT_MS = if ($Path -eq 'smoke') { '15000' } else { '120000' }

$child = Start-Process -FilePath "$wt\target\debug\areka.exe" -ArgumentList "`"$ghost`"", "`"$balloon`"" -RedirectStandardOutput "$run\stdout.log" -RedirectStandardError "$run\stderr.log" -WindowStyle Hidden -PassThru
$null = $child.Handle  # keep the handle so ExitCode is readable
$child | Add-Member ProcessId $child.Id
$child.Id | Set-Content "$run\pid.txt"
Write-Output "started pid=$($child.ProcessId) port=$port run=$run"

$deadline = (Get-Date).AddSeconds(60)
while (-not (Select-String -Path "$run\stdout.log", "$run\stderr.log" -Pattern '本物のゴースト窓を開きました' -Quiet -ErrorAction SilentlyContinue)) {
    if ((Get-Date) -gt $deadline) { Write-Output 'error no real windows in 60 s'; exit 1 }
    Start-Sleep -Milliseconds 300
}
Write-Output 'real windows open'
switch ($Path) {
    'smoke' { Wait-Exit }
    'close' { Start-Sleep -Seconds 15; taskkill /PID $child.ProcessId | Out-Null; Write-Output "taskkill sent at $(Get-Date -Format o)"; Wait-Exit }
    'menu'  {
        Start-Sleep -Seconds 15
        Add-Type -Namespace RmuW32 -Name U -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
public struct RECT { public int L, T, R, B; }
'@
        [RmuW32.U]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
        $target = [uint32]$child.ProcessId; $found = [System.Collections.Generic.List[string]]::new()
        [RmuW32.U]::EnumWindows({ param($h, $x)
            $o = [uint32]0; [RmuW32.U]::GetWindowThreadProcessId($h, [ref]$o) | Out-Null
            if ($o -eq $target -and [RmuW32.U]::IsWindowVisible($h)) {
                $r = New-Object RmuW32.U+RECT; [RmuW32.U]::GetWindowRect($h, [ref]$r) | Out-Null
                $found.Add("window hwnd=$h pid=$o rect=$($r.L),$($r.T),$($r.R),$($r.B)")
            }
            $true }, [IntPtr]::Zero) | Out-Null
        $found | ForEach-Object { Write-Output $_ }
        Wait-Exit
    }
}
```

## 型・関数・テストの結線の名前の改め（2026-11-13・タスク 2.2・要件 1.1〜1.4・1.6）

- 改めた所: `crates/areka` の中だけ（`src`・`examples`・`tests` の説明の文を含む・111 ファイル）。字面の置き換えの規則は次の 10 本で、残りが 0 件であることを `rg` で確かめた。
  - `Emo2Wiring` → `RuntimeWiring`・`Emo2BootInputs` → `RuntimeInputs`・`Emo2BootOutcome` → `RuntimeOutcome`・`wire_emo2_boot` → `wire_runtime`・`emo2_frame_system` → `frame_system`（`register_emo2_frame_system` → `register_frame_system` もこれで改まる）・`emo2_wiring` → `runtime_wiring`（`headless_emo2_wiring` → `headless_runtime_wiring` を含む）・`with_emo2` → `with_runtime`・`emo2_absent` → `runtime_absent`・`Emo2Absent` → `RuntimeAbsent`・`emo2_sink_of` → `runtime_sink_of`
- 字面で照合するテストは同じ規則で一緒に改まった: `frame_schedule_tests.rs` の `pub fn wire_runtime(`・`add_systems(Update, frame_system.after(update_typewriters));`・`add_systems(Update, ghost_quit_system.before(frame_system));`、`zorder_wiring_tests.rs` の `pub fn wire_runtime(`・`pub fn frame_system(world: &mut World) {`・`frame_system.before(apply_zorder_chain)`。
- 型の名前を中に持つログの文 5 本（`input_events` の `balloon_exit.rs`・`balloon_moved.rs`・`balloon_pressed.rs`・`balloon_wheel.rs`・`mod.rs` の「`Emo2Wiring` 不在（boot 前／失敗）…」）は、型の改名と同じく `RuntimeWiring` の語だけが変わった（要件 1.6: 文の本体・欄・段は同じ）。これを照合するテストは無い。2.3 の文言（`emo2-boot:` ほか）とイベントの値（`*_no_emo2`）はここでは触らない。
- 触らなかったもの: 本体の外の crate の説明の文（`areka-placement` の `chain_finalize.rs`・`lib.rs`・`persist.rs`・`persist_io_wiring_tests.rs`・`placement_prepare_tests.rs` と `doc/ukadoc-coverage/ledger/assets.toml`＝Out of scope・8.x の掃き出しで一覧に載せる）、`runtime/mod.rs` のテストの存在しない根の文字列 `areka-nonexistent-*-root-emo2boot-5v1`（名前ではなく、存在しないことだけが意味の値）。

### 名前だけでは決まらないものの判定（要件 1.3・13 節の続き）

| 名前 | 所 | 使われ方 | 判定 |
|---|---|---|---|
| `emo2()` | `src/install/judge_tests.rs` | フォルダ名・名前が `"emo2"` の `GhostFacts` を組む＝実在のゴースト emo2 の事実 | 変えない |
| `emo2(rel)` | `examples/emo-present/fixture.rs` | 検体 emo2 のフォルダの下のパスを返す | 変えない |
| `spawn_emo2` | `tests/emo2_real_run/common.rs` | 実機で検体 emo2 を起こす | 変えない |
| `with_emo2` | `src/input_events/shell_box_handler_tests.rs` | `headless_emo2_wiring` を World に置く＝結線 | `with_runtime` |
| `emo2_sink_of` | `src/runtime/consumer_ledger_agreement_tests.rs` | `runtime` の 11 の受け口かを返す＝結線 | `runtime_sink_of` |
| `leave_/moved_/pressed_emo2_absent_degrades_*` | `src/input_events/balloon_leave_tests.rs`・`balloon_pointer_handler_tests.rs` | 「`Emo2Wiring` を置かない」縮退の試験＝結線の不在 | `*_runtime_absent_degrades_*` |
| `Degrade::Emo2Absent` | `src/input_events/balloon_pointer_handler_tests.rs` | 同上（`Emo2Wiring` 不在の場合分け） | `Degrade::RuntimeAbsent` |

### 名前を改めたテストの新旧の対応表（要件 1.4・9.4 の多重集合の差の説明）

`#[test]` の fn だけ。12 本。名前の末尾（最後の `::` から後ろ）の新旧で、ほかのテストの名前は変わらない。

| 旧 | 新 | 所 |
|---|---|---|
| `emo2_frame_system_drives_text_scale_phase_every_frame` | `frame_system_drives_text_scale_phase_every_frame` | `runtime/frame_text_scale_tests.rs` |
| `emo2_frame_system_runs_dpi_phase_without_writes_when_unattached` | `frame_system_runs_dpi_phase_without_writes_when_unattached` | `runtime/frame_text_scale_tests.rs` |
| `emo2_frame_system_removes_runs_and_reinserts_wiring` | `frame_system_removes_runs_and_reinserts_wiring` | `runtime/frame_drain_text_tests.rs` |
| `wire_emo2_boot_falls_back_to_unwired_on_missing_ghost_root` | `wire_runtime_falls_back_to_unwired_on_missing_ghost_root` | `runtime/mod.rs` |
| `move_cue_sink_reaches_emo2_wiring_receiver` | `move_cue_sink_reaches_runtime_wiring_receiver` | `runtime/frame_attach_tests.rs` |
| `talk_playback_reaches_emo2_wiring_lifecycle_receiver` | `talk_playback_reaches_runtime_wiring_lifecycle_receiver` | `runtime/frame_attach_tests.rs` |
| `without_emo2_wiring_does_nothing` | `without_runtime_wiring_does_nothing` | `input_events/balloon_wheel_tests.rs` |
| `moved_without_emo2_wiring_does_not_record_hover` | `moved_without_runtime_wiring_does_not_record_hover` | `input_events/balloon_hover_flag_tests.rs` |
| `leave_clears_balloon_hover_even_without_emo2_wiring` | `leave_clears_balloon_hover_even_without_runtime_wiring` | `input_events/balloon_hover_flag_tests.rs` |
| `leave_emo2_absent_degrades_with_debug_and_leaves_hover` | `leave_runtime_absent_degrades_with_debug_and_leaves_hover` | `input_events/balloon_leave_tests.rs` |
| `moved_emo2_absent_degrades_with_debug_not_error` | `moved_runtime_absent_degrades_with_debug_not_error` | `input_events/balloon_pointer_handler_tests.rs` |
| `pressed_emo2_absent_degrades_with_debug_not_error` | `pressed_runtime_absent_degrades_with_debug_not_error` | `input_events/balloon_pointer_handler_tests.rs` |

## ホバーの集合を置く所の包含の確かめ（2026-11-13・タスク 3.3・要件 3.1）

`BalloonWiring` を world へ置く所の集合 ⊆ `BalloonHover` を置く所の集合を、`crates/areka/src` で grep して確かめた（置き忘れ 0）。

- `BalloonWiring` を置くファイル（式: `insert_non_send(BalloonWiring::new` または `BalloonWiring` の値を入れた変数の `insert_non_send(bw)`／`insert_non_send(wiring)`）: 12（本番 1＝`input_events/balloon.rs` の `wire_balloon_choice`・テスト 11）。
- `BalloonHover::default()` を置くファイル: 同じ 12。差（前者にあって後者に無いもの）は 0。
- 置く回数もファイルごとに一致する（本番 1・テスト 30。`balloon_pointer_handler_tests.rs` 10・`balloon_visibility_phase_tests.rs` 10・`balloon_visibility_phase_box_tests.rs` 2・残り 8 ファイル各 1）。
- 設計の「13 ファイル 45 か所」との差: `balloon_pass_through_tests.rs` は `BalloonWiring::new` を作るが world へ置かない（`send_selection` を直に呼ぶ）ので数に入らない。設計の 45 か所はこの式の数（30）と一致しない（設計の段の数え方は記録が無い）が、包含は上の式で 0 を示した。`wire_balloon_choice` を呼ぶテストは本番の関数が両方を置く。
- 外す所: 本番で外す所は無い（`BalloonWiring` と同じ）。

## 本体に頼るファイルの移動（2026-11-13・タスク 6.1・要件 2.1・2.3・3.7・6.4）

### 移したファイル（23 本・`git mv`・名前は変えない）

`runtime/` → `session/`: `ghost_switch.rs`・`ghost_switch_{balloon,boot_event,drop_write,fallback,notice,reload,rig_wait,roots,talk_clock}_tests.rs`・`ghost_switch_tests.rs`・`ghost_switch_test_support.rs`・`reload.rs`・`reload_tests.rs`・`reload_skin_tests.rs`・`runtime_boot.rs`・`runtime_boot_tests.rs`・`same_folder_pin.rs`・`same_folder_pin_tests.rs`・`shell_balloon_switch.rs`・`shell_balloon_switch_tests.rs`・`switch_copy.rs`・`switch_copy_tests.rs`。宣言は `runtime/mod.rs` から `session/mod.rs` へ同じ見える範囲と説明の文のまま移した。

- 中身の書き換え: `super::X`（`X` が `runtime` に残るもの）→ `crate::runtime::X`、`crate::runtime::X`（`X` が移したもの）→ `crate::session::X`（本体の 69 ファイルも同じ置き換え）・`ghost_session.rs` の 4 か所を `crate::session::…` に（947 行のまま）・`spine.rs:770` の `runtime_boot_options` の引き口・`runtime/mod.rs` の `wire_tests` の `boot_runtime`・`zorder_wiring_tests.rs` の `include_str!("../session/runtime_boot.rs")`。
- `runtime_boot_tests.rs` が `#[path]` で宣言する `script_security_e2e_tests.rs`・`readme_script_e2e_tests.rs` は設計の対応表（下の行に無いもの＝`runtime`）どおり `runtime/` に残し、`#[path = "../runtime/…"]` へ直した。9.1 の `modtree.py` はこの 2 本を `UNDECLARED` と出す（テストなので式の集合には入らない）。
- 既定のゴーストの判定（`DEFAULT_GHOST_FOLDER` の使い手）は `session/ghost_switch.rs` の中に残る（6.4）。
- 出所の追従（振る舞いを保つため・2.1 と同じ中間の扱い）: `log_history.rs` の決まりと `log_history_tests.rs` の 3 か所と `doc/ssp-mcp/log-convention.md:55`（決まりの表と文書の表を突き合わせる `log_history_convention_tests` のため）を `areka::session::ghost_switch` へ、`reload_tests.rs:580`・`reload_skin_tests.rs:328` の照合の出所を `areka::session::reload` へ改めた。7.1 は `UNKEPT_TARGETS` と assert を足すだけ、7.2 はほかの出所を直す。

### 広げた見える範囲（9.6 の一覧・コンパイルが求めた分だけ）

| 項目 | 前 → 後 | 要る所 | 段 |
|---|---|---|---|
| `session::ghost_quit::ghost_quit_system` | `pub(super)` → `pub(crate)` | `runtime/frame_schedule_tests.rs` | 5.1 |
| `runtime/mod.rs` の `mod change_cue`・`reload_cue`・`special_name`・`switch_cue`・`window_state_cue` | 私有 → `pub(crate)` | `session/runtime_boot.rs`・`ghost_switch.rs`・`reload.rs` | 6.1 |
| ~~`runtime/mod.rs` の `mod install_cue`・`readme_cue`・`update_cue`~~ | 私有 → `pub(crate)`（6.2 で `runtime` から外し、広げは消えた） | 6.2 から持ち主の宣言 `install/mod.rs`・`update/mod.rs`・`readme.rs` の `pub(crate) mod`（`session/runtime_boot.rs` の `runtime_sinks` と `runtime/consumer_ledger_agreement_tests.rs` が引く） | 6.1→6.2 |
| `RuntimeWiring` の欄 `shell_carry` | `pub(in crate::runtime)` → `pub(crate)` | `session/ghost_switch_reload_tests.rs` | 6.1 |

- （6.1 の時点）8 つの `mod` は 1 つずつ私有へ戻して `cargo check -p areka --all-targets` が誤りになることを確かめた（8 つとも要る）。

### 式 1〜3（6.1 の後・16.5 のコマンドで `B=crates/areka/src/runtime`・式 1 は `crate::(?!runtime\b)`）

- 本番のファイルの集合と宣言の木が一致（`SAME`・60 本＝61 − 移した本番 6 ＋ 3.x・4.1 で足した 5）。D1 37 本・D2 22 本・深さ 3 以上の本番 0。
- 式 2・式 3: **0 行**。
- 式 1: **8 行**。
  - `install_cue.rs:25`・`:26`・`:48`・`readme_cue.rs:39`・`:40`・`update_cue.rs:22`（6 行）＝受け口の持ち主を名指す所。**6.2** で持ち主へ移すと消える。
  - `mod.rs:837`（`#[cfg(test)] mod wire_tests` の中の `crate::session::runtime_boot::boot_runtime`）・`session_hooks.rs:78`（`#[cfg(test)] const FALLBACK = crate::session::hooks::HOOKS`・設計どおり）（2 行）＝`#[cfg(test)]` の項目の中。**9.1** で行ごとに示す。
- `runtime` の本番で `session` を名指すほかの行は説明の文だけ（`special_name.rs:3`・`mod.rs:176` の `session::runtime_boot::boot_runtime`・`mod.rs` の `wire_kanade_stop` のリンク）＝要件 3.4 の一覧（9.1）に載せる。

## 境界の 0 件と数（後）（2026-11-13・タスク 9.1・要件 3.1〜3.5・3.9・7.2）

### 18. 完了の木で当て直した式 1〜3

- 測った木: `263fdc964`（8.2 の後）。コマンドは 16.5 節のとおりで、置き換えを戻した形（`B=crates/areka/src/runtime`・式 1 は `crate::(?!runtime\b)`）。`rg` は PCRE2 の入った 15.0.0。
- 除いたもの（本番のファイルの集合を決める glob）: `*_tests.rs`・`*test_support*.rs`・`spine*.rs`・`*_cases.rs`（16 節と同じ・足していない）。

#### 18.1 本番のファイルの集合と D1・D2

- 宣言の木（`modtree.py`）と glob で残る集合は **一致**（`SAME`）。宣言されたファイル 207 本・ディレクトリの `*.rs` 209 本。
  - 差の 2 本 `readme_script_e2e_tests.rs`・`script_security_e2e_tests.rs` は `UNDECLARED`。`runtime/mod.rs` からは宣言されず、`session/runtime_boot_tests.rs:264`・`:268` が `#[path = "../runtime/…"]` で宣言する（6.1 で `runtime_boot_tests.rs` を `session/` へ移し、この 2 本は設計の対応表どおり `runtime/` に残した）。モジュールとしては `session` の下のテストなので、`runtime` の本番の集合にも `runtime` のテストの数にも入らない（中身は 18.3 の末尾に別に書く）。glob でも `*_tests.rs` で外れる。
- 深さごとの本数: 本番＝深さ 0 が 1 本（`mod.rs`）・深さ 1 が 34 本・深さ 2 が 22 本（計 57 本）。テスト＝深さ 1 が 7 本・2 が 103 本・3 が 34 本・4 が 5 本・5 が 1 本（計 150 本）。深さ 3 以上の本番は **0 本**（式は足さない）。
- 16 節（前・61 本）からの差: 移した本番 9 本（`session/` へ `ghost_switch.rs`・`reload.rs`・`runtime_boot.rs`・`same_folder_pin.rs`・`shell_balloon_switch.rs`・`switch_copy.rs`、持ち主へ `install_cue.rs`・`update_cue.rs`・`readme_cue.rs`）を外し、新しく入れた 5 本（`balloon_hover.rs`・`scope_scale.rs`・`session_hooks.rs`・`shell_box_hover.rs`・`skin_switch.rs`）を入れた＝61 − 9 ＋ 5 ＝ 57。
- D1（34 本）: `adapter.rs` `assets.rs` `balloon_background.rs` `balloon_hover.rs` `balloon_visibility.rs` `change_cue.rs` `consumer_ledger.rs` `frame.rs` `hit_region.rs` `hover_inject.rs` `move_cue.rs` `reload_args.rs` `reload_cue.rs` `scope_scale.rs` `script_restriction.rs` `session_hooks.rs` `shell_balloon_resolve.rs` `shell_box_assets.rs` `shell_box_hover.rs` `shell_carry.rs` `shell_cursor_assets.rs` `shell_cursor_table.rs` `skin_facts_copy.rs` `skin_switch.rs` `special_name.rs` `strict_judges.rs` `switch_assets.rs` `switch_cue.rs` `talk_clock.rs` `talk_lifecycle.rs` `target_map.rs` `user_break_cue.rs` `window_state_cue.rs` `zorder_cue.rs`
- D2（22 本・16.3 と同じ顔ぶれ）: `balloon_visibility_{decision,phase,wait}.rs`・`move_cue_{args,geometry,place}.rs`・`frame/{attach,balloon_face,balloon_offset_follow,dpi,drain_resnap,move_hold,move_shift,scale_notice,scale_text,status_report,switch,window_state,wiring,work_area_sync,zorder_descript,zorder_drain}.rs`

#### 18.2 式 1〜3 の結果（要件 3.1・3.2・3.3）

| 式 | 当てた所 | 前（16.4） | 後 |
|---|---|---|---|
| 式 1 `crate::(?!runtime\b)` | 本番の 57 本 | 60 行 | **2 行**（どちらも `#[cfg(test)]` の項目の中＝本番は 0） |
| 式 2 `super::super::` | D1 の 34 本 | 0 行 | **0 行** |
| 式 3 `super::super::super::` | D2 の 22 本 | 0 行 | **0 行** |

式 1 に残った 2 行（行ごとの確かめ）:

- `runtime/mod.rs:834` `crate::session::runtime_boot::boot_runtime,` — `mod.rs:755` の `#[cfg(test)]` と `:756` の `mod wire_tests {`（ファイルの末尾 850 行まで）の中。`wire_runtime` に起動の関数を渡すテストの引数。
- `runtime/session_hooks.rs:78` `const FALLBACK: SessionHooks = crate::session::hooks::HOOKS;` — 直前 `:77` の `#[cfg(test)]` が付いた項目。本番は `:73`〜`:74` の `#[cfg(not(test))] const FALLBACK = NO_BODY`（設計どおり・テストの World は本体の口をそのまま読む）。
- 1.3 の注どおり、インラインのテストのモジュールだけでなく本番のファイルの中の `#[cfg(test)]` の fn・use・impl・const も見た。式 1 に当たるのは上の 2 行だけで、ほかの本番の行は 0。
- 深さ 0 の `mod.rs` の `super::` は 2 行（`mod.rs:757`・`:758`）で、どちらも `wire_tests` の中の `use super::…`（`runtime` 自身）＝外への参照ではない。
- 束ねた形 `use crate::{…}` は本番に 0 行（式 1 の 0 に入っている）。
- よって `runtime` の本番のコードが本体の `runtime` の外（`session`・`install`・`update`・`readme` を含む）を参照する所は **0 件**。参照してよい相手（要件 3.2）はワークスペースの下の crate と外の crate だけになった。上の 2 行は 18.3 のテストの数へ回す。

#### 18.3 テストのコードが本体の外を引く所（要件 3.5・7.2）

数え方: テストのファイル（宣言の木で `T` の 150 本）の各ファイルに、式 1 と「宣言の深さ d のファイルから `super::` を d＋1 回続けて `runtime` の外へ抜ける行」を当て、ファイルごとに数えた（使い捨ての読み手 `testrefs.py` を一時フォルダで回した・リポジトリには置かない）。インラインの `mod … { }` の中は深さが 1 つ増えるが、`super::` で抜ける行は 0 本なので数は変わらない（インラインのモジュールを持つのは `strict_judges_tests.rs` だけ）。本番のファイルの中の `#[cfg(test)]` の項目は 18.2 の 2 行。

- 合計: **79 行**（テストのファイル 40 本で 77 行・本番のファイルの中の `#[cfg(test)]` の項目 2 本で 2 行）。`super::` で外へ抜ける行は 0 行で、79 行は全部 `crate::` で始まる。0 にすることは求めない（要件 3.5）。
- ファイルごとの数（`runtime/` からの相対パス・括弧は宣言の深さ）:
  - 5 行: `frame/switch_finish_tests.rs`（3）・`frame/switch_tests.rs`（3）
  - 4 行: `frame_chain_finalize_persist_tests.rs`（2）・`frame_scale_notice_integration_tests.rs`（3）・`frame_stay_on_top_lifecycle_tests.rs`（2）
  - 3 行: `consumer_ledger_agreement_tests.rs`（2）・`frame/switch_cursor_tests.rs`（4）・`frame/window_state_tests.rs`（3）・`frame_chain_finalize_persist_nowrite_tests.rs`（2）・`frame_shell_box_arrow_tests.rs`（4）・`frame_visibility_integration_tests.rs`（2）
  - 2 行: `balloon_visibility_phase_box_tests.rs`（3）・`frame_chain_finalize_persist_restart_tests.rs`（2）・`frame_move_order_tests.rs`（2）・`frame_schedule_tests.rs`（1）・`shell_carry_tests.rs`（2）・`spine.rs`（1）・`spine_third_scope_talk_tests.rs`（2）・`strict_judges_tests.rs`（2）
  - 1 行: `assets_shell_tests.rs`・`assets_tests.rs`・`balloon_visibility_lifecycle_e2e_tests.rs`・`balloon_visibility_phase_notice_tests.rs`・`balloon_visibility_phase_reappear_tests.rs`・`balloon_visibility_phase_tests.rs`・`film_playback_e2e_tests.rs`・`frame/status_report_stage_tests.rs`・`frame/status_report_tests.rs`・`frame/switch_reload_tests.rs`・`frame_attach_carry_tests.rs`・`frame_chain_finalize_persist_test_support.rs`・`frame_chain_finalize_remembered_tests.rs`・`frame_chain_finalize_restart_tests.rs`・`frame_shell_box_integration_tests.rs`・`frame_transition_atomicity_tests.rs`・`frame_transition_branch_tests.rs`・`shell_balloon_resolve_tests.rs`・`spine_close_wiring_tests.rs`・`switch_cue_tests.rs`・`zorder_wiring_tests.rs`（21 本）
  - `#[cfg(test)]` の項目: `mod.rs`（`wire_tests`）1 行・`session_hooks.rs`（`FALLBACK`）1 行
- 引く相手（`crate::` の次の段・テストのファイルの 77 行）: `session` 18・`ghost_session` 14・`input_events` 13・`restore_merged_placements` 7・`app_exit` 6・`update` 2・`scope_bind_fixture_test_support` 2・`roots` 2・`placement_tests` 2（`frame_transition_atomicity_tests.rs`・`frame_transition_branch_tests.rs` の `crate::placement_tests::transition_judge`・要件 7.2 の名指し）・`boot_resolve` 2・`boot_config` 2・`readme` 1・`menu` 1・`mcp` 1・`log_history` 1・`install` 1・`insert_persist_wiring` 1・`crate_source_test_support` 1。`runtime-crate-split` で crate を分けるときに、テストの置き場か支えを決める材料になる。
- 宣言の木の外の 2 本（18.1）: `readme_script_e2e_tests.rs` は 4 行（`crate::ghost_session`・`crate::readme` 2・`crate::session::ghost_switch_test_support`）、`script_security_e2e_tests.rs` は 1 行（`crate::session::ghost_switch_test_support`）。どちらも `session::runtime_boot_tests` の子なので上の 79 行には入れていない（`runtime-crate-split` への申し送りは 8.2 で済み）。

#### 18.4 説明の文の名指し（要件 3.4）

式 1〜3 の `^(?!\s*//)` を `^\s*//` に替えた 3 本を本番の集合に当てた。式 2・式 3 の説明の版は **0 行**、式 1 の説明の版は **16 行**。残すもの（どれも説明の文・rustdoc のリンクで、コードの参照ではない）:

| 所 | 名指し | 残す理由 |
|---|---|---|
| `consumer_ledger.rs:90`・`:152` | `crate::readme::readme_cue::ReadmeCueSink` | 台帳の行を受ける受け口へのリンク（要件 2.4 で移した先へ書き換えたもの） |
| `consumer_ledger.rs:108`・`:189` | `crate::install::install_cue::InstallCueSink` | 同上 |
| `consumer_ledger.rs:111`・`:196` | `crate::update::update_cue::UpdateCueSink` | 同上 |
| `hit_region.rs:45`・`:47`・`:51`・`:57` | `crate::`（語としての「`crate::` フリー」の決まり） | パスの名指しではなく、このファイルが `crate::` を使わない決まりの説明 |
| `hit_region.rs:56` | `crate::is_benign_boot_error` | `#[path]` で取り込めない理由の説明 |
| `mod.rs:380`・`:439` | `crate::is_benign_boot_error` | 起動の失敗を分ける所の説明（R7.4） |
| `mod.rs:442`・`:647` | `crate::session::ghost_quit::wire_kanade_stop` | 終了の受け口を据える所へのリンク（5.1 の注） |
| `special_name.rs:3` | `crate::session::ghost_switch` | 純移動の出所の説明 |

補足（3 本の式では拾わない、`crate::` の付かない名指し・説明の文だけ）: `session::` 4 行（`mod.rs:173`・`session_hooks.rs:5`・`:18`・`skin_switch.rs:86`）、`ghost_session::` 15 行（`frame.rs:198`・`mod.rs:187`・`:188`・`:335`・`:427`・`:428`・`:445`・`:452`・`:453`・`:483`・`:485`・`frame/wiring.rs:300`・`:301`・`:332`・`:333`）、`input_events::` 6 行（`balloon_hover.rs:3`・`:6`・`hit_region.rs:18`・`:20`・`shell_box_hover.rs:3`・`:19`）、`app_exit::` 1 行（`frame.rs:177`）、`install::` 1 行（`mod.rs:375`）、`super::super::input_events::balloon` 1 行（`frame/wiring.rs:322`・深さ 2 の `super::super` は `runtime` 自身なので字面は古い・コードではないので式 3 にも当たらない）。計 28 行。どれも呼び手や持ち主を説明する文で、残す（式: `'^\s*//.*(?<!crate::)(?<!runtime::)\b(session|install|update|readme|ghost_session|app_exit|session_end|input_events|boot_config)::'` を本番の集合に当てた）。

#### 18.5 プロセスで共有する状態 3 つの置き場（要件 3.9）

| 状態 | 前（`4863cb66`） | 後（`263fdc964`） |
|---|---|---|
| `static RESOLVED: OnceLock<f64>` | `emo2_boot/balloon_visibility_wait.rs:72` | `runtime/balloon_visibility_wait.rs:72` |
| `static CONFIG: OnceLock<HoverInjectConfig>` | `emo2_boot/hover_inject.rs:187` | `runtime/hover_inject.rs:187` |
| `static NEXT_EPOCH: AtomicU64` | `emo2_boot/frame/switch.rs:60` | `runtime/frame/switch.rs:56` |

- 3 つとも同じファイル（モジュールの改名 `emo2_boot` → `runtime` の分だけパスが変わった）に、同じ型と初期値のまま残った。`NEXT_EPOCH` の行が 60 → 56 になったのは、同じファイルの上の `use` から本体を引く行（`crate::boot_config`・`crate::boot_resolve`・`crate::ghost_session` ほか）を外した分。`frame/switch.rs` は `runtime` に残したので、2.9 節の「`NEXT_EPOCH` が session へ移る」案は取っていない。
- 数も変わっていない: `crates/areka/src` 全体の `static`／`thread_local!` の行数をファイルごとに前後で比べた差は、`emo2_boot/` → `runtime/` の改名と、テストの `install_cue_tests.rs` の `install/` への移動（2 行・テストの中の状態）だけ。`session/` と新しく足した 5 本（`session_hooks.rs` ほか）には `static` は 0 行（`SessionHooks` の口は `Resource` で World に置く・設計どおり）。
- 確かめのコマンド: `git grep -c -P '^\s*(pub(\([^)]*\))?\s+)?static\s|thread_local!' <木> -- crates/areka/src` を `4863cb66` と `HEAD` で取り、`<木>:` を落として `diff`。

#### 18.6 測り直しのコマンドと読み手（再現の確かめ）

- 16.5 節のコマンドを `B=crates/areka/src/runtime`・式 1 `crate::(?!runtime\b)` にして回す（`modtree.py` は 16.5 節のまま）。説明の版は 3 本の式の `^(?!\s*//)` を `^\s*//` に替える。
- テストの数の読み手 `testrefs.py`（一時フォルダに置いて回す・リポジトリには置かない・引数は `B` と `modtree.py` の出力）:

```python
import re, sys, os
root, tree = sys.argv[1], sys.argv[2]
rows = [l.split() for l in open(tree, encoding='utf-8')]
e1 = re.compile(r'^(?!\s*//).*\bcrate::(?!runtime\b)')
tot = {}
for d, k, f in rows:
    if k != 'T': continue
    d = int(d)
    sup = re.compile(r'^(?!\s*//).*\b(super::){%d}' % (d + 1))
    lines = open(os.path.join(root, f), encoding='utf-8').read().splitlines()
    c = sum(1 for l in lines if e1.search(l)); s = sum(1 for l in lines if sup.search(l))
    if c or s: tot[f] = (d, c, s)
for f, (d, c, s) in sorted(tot.items(), key=lambda x: -(x[1][1] + x[1][2])):
    print(f'{c+s:3} crate={c} super={s} depth={d} {f}')
print('FILES', len(tot), 'CRATE', sum(v[1] for v in tot.values()), 'SUPER', sum(v[2] for v in tot.values()))
```

- 2 回目に回した結果も同じ（`SAME`・本番 57 本・D1 34・D2 22・式 1 は 2 行・式 2 と式 3 は 0 行・説明の版 16／0／0 行・テスト 40 本 77 行）。

## 古い名前の掃き出しと emo2 の残りの一覧（2026-11-13・タスク 9.2・要件 1.2・4.6・5.9・6.1〜6.4）

- 測った木: `5d2683753`（9.1 の後）に、20.5 で改めた 11 行を足した木。`rg` は 18 節と同じ VS Code 同梱の 15.0.0。

### 19. 掃き出しの grep（要件 4.6・5.9）

#### 19.1 古い出所 `areka::emo2_boot`（要件 4.6）

`rg -n 'areka::emo2_boot' crates tools doc .kiro/steering/structure.md .kiro/steering/tech.md` ＝ **6 行・2 ファイル・どれも本体の外**。

| ファイル | 行 | 分け |
|---|---|---|
| `crates/log-capture-kit/src/event_tests.rs` | 42・53・277・388・407 | Out of scope に挙げた出所の見本の値（`tracing` に食わせる target の文字列・照合の相手は本体ではない） |
| `crates/areka-placement/src/diag.rs` | 136 | Out of scope に挙げた `areka-placement` の説明の文（字面は古い・`runtime-crate-split` で直す） |

本体（`crates/areka`）・`tools/`・`doc/`・steering の 2 本は **0 行**。

#### 19.2 古いパスと改める前の名前（要件 5.9）

`rg -n 'emo2_boot|Emo2Wiring|Emo2BootInputs|Emo2BootOutcome|wire_emo2_boot|register_emo2_frame_system|emo2_frame_system|emo2-boot|emo2 attach:|emo2 text:|_no_emo2\b|headless_emo2_wiring'` を同じ 5 か所に当てた ＝ **104 行・58 ファイル**（20.5 で改める前は 106 行）。

| 所 | ファイル・行 | 分け |
|---|---|---|
| 本体の外の crate（11 crate） | 52 ファイル・93 行（`areka-placement` 25 ファイル 56 行・`areka-emo-text` 8／11・`log-capture-kit` 2／6・`areka-kanade` 3／5・`areka-emo-present` 3／4・`areka-ghost` 2／2・`areka-parsers` 2／2・`areka-seriko` 1／2・`ukadoc-survey` 2／2・`wintf` 2／2・`shiori-host32-host` 1／1） | Out of scope の説明の文とテストの名前（`actor_lookahead_tests.rs` の `emo2_boot_talk_reveals_without_moving_glyphs`）と見本の値（`log-capture-kit` の出所・`ukadoc-survey/src/evidence/candidates_tests.rs:345` の `choice_pressed_no_emo2`＝抽出器に食わせる文字列・2.3 の注） |
| 本体 `crates/areka` | 4 ファイル・7 行 | 歴史の名指しだけ（下の表） |
| `.kiro/steering/structure.md` | 269・271 の 2 行 | 改名の記し（「旧名 `emo2_boot/`（`runtime-module-untangle` 2026-11 で改名）」「`emo2_boot/` から分けた」）。今のパスを指す文ではなく、古い名前で探した人を新しい名前へ渡す歴史の 1 句 |
| `doc/` | `doc/archive/discussion-260702-m1-roadmap.md:126`・`doc/roadmap-overview-2026-10-16.html:70` | 歴史の文書 |
| `tools/`・`.kiro/steering/tech.md` | 0 行 | — |

本体に残る 7 行（どれも完了 spec の名前か、その設計の中のファイル名だけ）:

| 行 | 当たる語 | 分け |
|---|---|---|
| `Cargo.toml:60`・`:69`・`:74` | `areka-P0-emo2-boot`（完了 spec の名） | 歴史の名指し（60 行目の頭の `emo2-boot 統合結線` は 20.5 で改めた） |
| `src/main.rs:54` | `areka-P0-emo2-boot` | 同上（頭の `emo2 統合結線` は 20.5 で改めた） |
| `src/runtime/mod.rs:1` | `areka-P0-emo2-boot`・「emo2 が起動して喋る」（M-boot の節目＝実在のゴースト emo2） | 歴史の名指し・実在の emo2 |
| `src/runtime/mod.rs:69`・`src/runtime/spine.rs:33` | `tests/emo2_boot_spine_test.rs` | 完了 spec `areka-P0-emo2-boot` の設計の File Structure に書かれたファイル名（実在したことのない名・設計との差を説明する文） |

### 20. emo2 の残りの一覧（要件 6.1・6.2）

`rg -n -i 'emo2' crates tools doc` ＝ **2,134 行**（20.5 で改める前は 2,143 行・差の 9 行は 20.5 の 11 行のうち `emo2` の語が消えた行）。

#### 20.1 分け方（上から順に当て、最初に当たったもの）

1. `doc/archive/` と日付を名に持つ写し `doc/roadmap-overview-2026-10-16.html` → **④ 歴史の文書**。
2. 完了 spec の名（`areka-P0-emo2-*`）と文書の名（`emo2-conformance-scope`・メモリの見出し `areka-emo2-signoff-needs-absolute-paths`）を除いて読み、本体の外の行に 19.2 の古い名前が残る → **③ 本体の外の crate の説明とテストの名前**（`areka-emo-text` のテストの `EMO2_BOOT_TALK`・`EMO2_BOOT_FINAL`＝emo2 の起動のトークの写しは古い名前ではないので ③ に入れない＝②）。
3. 本体の行で古い名前が完了 spec の設計のファイル名 `emo2_boot_spine_test` だけ → **④**。
4. 2 の除き方をした後に `emo2` が残らない（完了 spec の名・文書の名だけの行） → **④**。
5. 本体の行でなお古い名前が残る → **分けられない**（20.5 で改める）。
6. 既定のゴースト名とその使い手のファイル（20.3） → **① 既定のゴースト名とその使い手**。
7. 残り → **② 実在のゴースト emo2 を指す検体**（検体 `SampleRoot::acquire("emo2")`・`emo2_root`・`acquire_emo2`・`emo2-kakukaku`・実機テスト `tests/emo2_real_run*`・`emo2_first_load_repeat`・環境変数 `AREKA_EMO2_*`・`EMO2_ADJUST`・`vendors/sample_ghost/emo2.nar`・emo2 の資産を写した期待値・emo2 の名を見本の値に使うテスト・emo2 を測りの的にした道具と文書）。

1〜5 は機械で当て（読み手は 22 節）、6・7 の境と、② に紛れうる結線の語は目で読んで確かめた（20.4）。

#### 20.2 結果（後・2,134 行）

| 所 | ① 既定のゴースト | ② 実在の emo2 | ③ 外の crate の説明・テスト名 | ④ 歴史 | 分けられない |
|---|---|---|---|---|---|
| `crates/areka`（本体） | 55 | 518 | — | 17 | **0** |
| 本体の外の crate（29 crate） | 2 | 1,181 | 93 | 55 | **0** |
| `tools/` | 17 | 51 | — | — | **0** |
| `doc/`（`archive/` の外） | — | 111 | — | 22 | **0** |
| `doc/archive/` | — | — | — | 12 | **0** |
| 計 | 74 | 1,861 | 93 | 106 | **0** |

- 本体の外の crate の内訳（行）: `areka-placement` ②207・③56／`areka-emo-text` ②150・③11・④46／`areka-seriko` ②124・③2／`areka-parsers` ②106・③2／`areka-emo-compose` ②102／`areka-emo-atlas` ②87／`areka-nar` ②75／`areka-ghost` ②61・③2・④1／`areka-emo-present` ②59・③4／`sample-ghost-kit` ②59／`pilot` ②36／`areka-sylphya` ①2・②23／`areka-mcp` ②21／`real-run-kit` ②20／`areka-kanade` ②14・③5・④4／`areka-user-data` ②9／`shiori-host32-host` ②9・③1／`areka-update` ②8／`log-capture-kit` ②5・③6／`shiori4-testdll` ②3／`ukadoc-survey` ③2・④1／`shiori-host32-helper` ④2／`wintf` ②1・③2／`areka-sakura` ②1／`shiori-host32-testdll` ②1／`shiori-host32-testdll-loadu` ④1。
- `doc/` の内訳（行）: `COMPAT_ARCHITECTURE.md` ②28・④2／`ukadoc-coverage/` ②27・④13／`emo2-conformance-scope.md` ②19・④1／`testing.md` ②19・④1／`ssp-mcp/` ②9／`pasta-real-run.md` ②5／`choice-cascade-compat.md` ②3・④4／`README.md` ②1／`roadmap-overview-2026-10-16.html` ④1／`archive/` ④12。どの行も実在のゴースト emo2（検体・SSP で測った emo2・適合の的）か完了 spec の名で、結線を指して emo2 の語を使う行は無い。
- `tools/` は `package.ps1`（配布物に入れる既定のゴースト `ghost/emo2` と同梱バルーン・17 行＝①）と `tools/perf/`（測りの的のゴースト emo2 のアニメ定義と検体の窓口・51 行＝②）だけ。

#### 20.3 ① 既定のゴースト名とその使い手（74 行）

- 本番: `crates/areka/src/boot_resolve.rs:32` の `DEFAULT_GHOST_FOLDER = "emo2"` と、同じファイルの説明の文。
- 既定のゴーストの判定を見るテスト: `boot_resolve_tests.rs`（`ghost_only_emo2_is_only_not_default`・`ghost_default_emo2_when_listed_among_many` ほか）・`boot_resolve_option_tests.rs`・`boot_config_roots_tests.rs`・`roots/user_root_tests.rs`（実行ファイルの隣の同梱の既定のゴースト）・`readonly_tests.rs`・`install/destination_tests.rs`・`ghost_session_switch_fallback_tests.rs`（落ちたときに戻る既定ゴーストの窓）。
- 本体の外: `areka-sylphya/src/persist/format.rs`（`LastGhost` の書式の例と見本の値 `"emo2"`）・`tools/package.ps1`（配布物の既定のゴースト）。

#### 20.4 目で読んで判定した行（要件 1.3 の使われ方の規則）

- **② とした（実在のゴースト emo2 を指す）**:
  - `src/main_ghost_wiring_tests.rs:48`「本番結線は emo2＝Helper 経路のまま」— 実在の emo2 の SHIORI（32bit の pasta.dll）が通る経路のこと。
  - `src/runtime/spine.rs:460`・`:497`・`src/input_events/balloon_wiring_tests.rs:158`・`src/placement_tests/spawn_assembly_tests.rs:244` の「emo2 相当」— emo2 の検体と同じ値（2 スコープ・下端・窓の題）を写したテストの支え。
  - `src/runtime/frame_visibility_integration_tests.rs:71`「実 emo2 資産で結線資源を組む」— 組む材料が emo2 の検体であること（結線の名は `RuntimeWiring`）。
  - `update/`・`install/`・`readme/`・`log_history*_tests.rs`・`cli/parse_tests.rs`・`runtime/reload_args_tests.rs` の `"emo2"` — 実在のゴースト emo2 の名を見本の値に使うテスト（既定かどうかは判定していない）。
  - `areka-emo-text` の `EMO2_BOOT_TALK`・`EMO2_BOOT_FINAL` — emo2 の起動のトークの写し（`emo2_boot` の結線ではない）。
- **③ とした（本体の外・`runtime-crate-split` で直す）**: `ukadoc-survey/src/evidence/candidates_tests.rs:345` の `debug!(event = "choice_pressed_no_emo2")`（抽出器に食わせる見本の文字列・本体の値と照合しない・2.3 の注）・`log-capture-kit/src/event_tests.rs` の出所の見本の値・`areka-placement` の `Emo2Wiring`・`wire_emo2_boot` の言及（2.2 の注）。
- **分けられなかった行（11 行・20.5 で改めた）**: 機械の 5 の段で 4 行（`Cargo.toml:60`・`log_history.rs:134`・`runtime/mod.rs:816`・`:817`）、目で読んで 7 行（`main.rs:54`・`ghost_session.rs:862`・`runtime/consumer_ledger.rs:215`・`:546`・`runtime/frame_scale_notice_integration_tests.rs:29`・`runtime/frame_visibility_integration_tests.rs:591`・`:602`）。どれも `emo2` の語で結線や実行系（モジュール `emo2_boot`・型 `Emo2Wiring`）を指していた。

#### 20.5 改めた行（要件 6.3・要件 1 の規則で `emo2` の語を `runtime` の語へ）

| 行 | 前 | 後 | 指していたもの |
|---|---|---|---|
| `crates/areka/Cargo.toml:60` | `# emo2-boot 統合結線（areka-P0-emo2-boot・R10.8）` | `# runtime 統合結線（…）` | 実行系の結線（spec の名は歴史として残す） |
| `crates/areka/src/main.rs:54` | `/// emo2 統合結線（areka-P0-emo2-boot）` | `/// runtime 統合結線（…）` | モジュール `runtime` の宣言の説明 |
| `crates/areka/src/ghost_session.rs:862` | `// emo2 統合結線（task 5.2・…wire_runtime…）` | `// runtime 統合結線（…）` | `wire_runtime` の呼び出し |
| `crates/areka/src/log_history.rs:134` | `` `install_cue` は emo2_boot から `` | `` `install_cue` は runtime から `` | 移す前のモジュール（古いパス） |
| `crates/areka/src/runtime/consumer_ledger.rs:215`・`:546` | `emo2 の外の担当`・`受け口は emo2 の外（areka_ghost）` | `runtime の外の…` | モジュールの外（受け口は `areka_ghost`） |
| `crates/areka/src/runtime/mod.rs:816`・`:817`（テスト） | `C:/areka-nonexistent-{ghost,balloon}-root-emo2boot-5v1` | `…-runtime-5v1` | 結線の失敗を見る無いパスの名（照合する所は無い・この 2 行だけ） |
| `crates/areka/src/runtime/frame_scale_notice_integration_tests.rs:29` | `檻: GPU の World・emo2 の結線・…` | `runtime の結線` | 檻が持つ `RuntimeWiring` |
| `crates/areka/src/runtime/frame_visibility_integration_tests.rs:591`・`:602` | `（GPU の World・emo2 の結線・置き場のゴースト）` | `runtime の結線` | 同上 |

どれも説明の文・コメント・テストの中の値で、本番の振る舞いは変わらない（ログの文言・イベントの値・テストの名前は変えていない＝9.4 の多重集合に差は出ない）。確かめ: `cargo check --workspace --all-targets`（rustc の警告 0・マニフェストの警告は前からの 3 件）・`cargo test -p areka -j 4`（全部 ok・FAILED 0・本体の単体 3,171 passed／6 ignored）・`cargo fmt --all -- --check`（差 0）。

### 21. emo2 だけに向けた動き（要件 6.4）

- 本体の本番で `"emo2"` の文字列を持つのは `boot_resolve.rs:32` の `DEFAULT_GHOST_FOLDER` だけ（`git grep -n -i '"emo2'` をテスト・支え・例・`tests/` を除いて当てた。`runtime/mod.rs:793`・`spine.rs:657`〜`:663` に当たる行は `#[cfg(test)]` のモジュールの中。本体の外は検体の台帳 `sample-ghost-kit` と説明の文とテストの値だけ）。
- `DEFAULT_GHOST_FOLDER` の本番の使い手は 3 ファイルで、どれも「既定のゴーストかどうか／既定のゴーストへ戻す」の判定:
  - `boot_resolve.rs:247`（起動のゴーストの決め方の段 4「既定」・この定数を引くのはこの段だけ）・`:519`〜`:543`（`LastGhost` を既定へ戻す記憶の書き込みとその記録）。
  - `session/ghost_switch.rs:421`・`:458`・`:634`・`:701`・`:718`〜`:752`（切替先が落ちたとき既定のゴーストへ戻すか・戻す先が既定か・既定が目録に無い／起こせない記録）。
  - `main.rs:619`〜`:627`（既定のゴーストへ戻すのに失敗して終わるときの告知の名と根）。
- `runtime/` の本番は `DEFAULT_GHOST_FOLDER` も `"emo2"` も引かない（`runtime/` の本番で `emo2` に当たる行は、説明の文で実在の emo2 の資産を例に挙げる所だけ・20.4）。

### 22. 測り直しのコマンド（Git Bash・ワークツリーの根で）

```bash
RG=".../ripgrep-universal/bin/win32-x64/rg.exe"   # 18 節と同じ
"$RG" -n 'areka::emo2_boot' crates tools doc .kiro/steering/structure.md .kiro/steering/tech.md
"$RG" -n 'emo2_boot|Emo2Wiring|Emo2BootInputs|Emo2BootOutcome|wire_emo2_boot|register_emo2_frame_system|emo2_frame_system|emo2-boot|emo2 attach:|emo2 text:|_no_emo2\b|headless_emo2_wiring' crates tools doc .kiro/steering/structure.md .kiro/steering/tech.md
"$RG" -n -i 'emo2' crates tools doc > emo2.txt && python -I classify.py emo2.txt
```

分けの読み手 `classify.py`（一時フォルダに置いて回す・リポジトリには置かない）:

```python
import re, sys, collections
OLD = re.compile(r'emo2_boot|emo2-boot|Emo2Wiring|Emo2Boot|emo2boot|wire_emo2_boot|emo2_frame_system|_no_emo2\b|headless_emo2_wiring')
SPEC = re.compile(r'areka-P0-emo2-[a-z0-9-]+|emo2-conformance-scope|emo2-signoff-needs-absolute-paths')
DEFAULT_FILES = ('crates/areka/src/boot_resolve.rs', 'crates/areka/src/boot_resolve_tests.rs',
    'crates/areka/src/boot_resolve_option_tests.rs', 'crates/areka/src/boot_config_roots_tests.rs',
    'crates/areka/src/roots/user_root_tests.rs', 'crates/areka/src/readonly_tests.rs',
    'crates/areka/src/install/destination_tests.rs', 'crates/areka/src/ghost_session_switch_fallback_tests.rs',
    'crates/areka-sylphya/src/persist/format.rs', 'tools/package.ps1')
cats = collections.Counter(); unclassified = []
for raw in open(sys.argv[1], encoding='utf-8'):
    path, _, rest = raw.partition(':')
    path = path.replace('\\', '/'); text = SPEC.sub('', rest.partition(':')[2])
    inbody = path.startswith('crates/areka/')
    if path.startswith('doc/archive/') or path == 'doc/roadmap-overview-2026-10-16.html': c = '4'
    elif OLD.search(text) and not inbody: c = '3'
    elif OLD.search(text) and 'emo2_boot_spine_test' in text: c = '4'
    elif not re.search('emo2', text, re.I): c = '4'
    elif OLD.search(text): c = '?'; unclassified.append(raw.rstrip())
    elif path in DEFAULT_FILES: c = '1'
    else: c = '2'
    cats[c] += 1
print(dict(sorted(cats.items())), sum(cats.values()))
for u in unclassified: print('UNCLASSIFIED', u)
```

- 改める前の木（`5d2683753`）では `{'1': 74, '2': 1868, '3': 93, '4': 104, '?': 4}`（2,143 行・機械の 5 の段の 4 行）、後は `{'1': 74, '2': 1861, '3': 93, '4': 106}`（2,134 行・分けられない行 0）。
- 機械の段は語の形で当てるので、`emo2` の語で結線を指す地の文（「emo2 統合結線」「emo2 の結線」「emo2 の外」）は ② に紛れる。20.4 の 7 行はそれを目で拾ったもの。次に測るときも、本体の ② の行のうち「結線」「統合」「の外」を含む行は目で読む。

## テストの支えの一覧と audit への申し送り（2026-11-13・タスク 9.3・要件 7.1・7.4）

### 23. `runtime` の外のファイルが名指す `runtime` のテストの支え（要件 7.1）

`crates/areka/src/` のうち `runtime/` の外（`session/` を含む）で、`runtime::<支え>` を名指す行を `rg -c` で数えた（`93aca049f`）。行の数なので、1 行の `use …::{A, B}` は 1 と数える。`crates/areka/` の外で名指すのは `crates/areka/Cargo.toml` 74 行目の注の 1 行だけ（コードではない）。

```
RG=<rg.exe>; cd crates/areka/src
"$RG" -P -c 'runtime::spine\b(?!::hold_support)' . -g '!runtime/**'
"$RG" -P -c 'runtime::spine::hold_support' . -g '!runtime/**'
"$RG" -P -c 'runtime::sample_test_support' . -g '!runtime/**'
"$RG" -P -c 'runtime::frame::balloon_runtime_test_support' . -g '!runtime/**'
"$RG" -P -c 'runtime::(frame::(test_support|chain_finalize_persist_test_support|zorder_drain::test_support)|spine::(test_support|conformance_support)|balloon_visibility(_phase)?::test_support|crash_report_test_support)' . -g '!runtime/**'
```

| 支え | 行 | ファイル | 使う所（ファイル） | 使う項目 |
|---|---:|---:|---|---|
| `spine`（`hold_support` を除く） | 42 | 38 | `mcp/*` 10（`dump_surface.rs` 3・`dump_surface_tests.rs` 3・ほかの 4 ファイル 1 行ずつ）・`session/*` 7・`install/*` 4・`input_events/*` 3・`ghost_session_*` 6・`shell_balloon_switch_session_*` 4・`session_end_*` 3・`update/*` 2・`boot_shell_tests`・`main_session_mark_tests`・`readonly_run_tests` | `RecordedCall` 14・モジュールごと 13・`Progress` 5・`GpuPermit` 3・`join_thread_bounded` 2・`ScriptedShioriBackendBuilder` 2・`wait_until` 1・`wait_recv` 1・`WaitFailure` 1 |
| `sample_test_support` | 6 | 6 | `ghost_session_reload_tests`・`ghost_session_restart_tests`・`session/ghost_switch_test_support`・`session/runtime_boot_tests`・`placement_tests/measure_bake_count_tests`・`placement_tests/reset_balloon_pos_tests` | `acquire_emo2` 4・モジュールごと 2 |
| `spine::hold_support` | 2 | 2 | `session_end_deadline_tests`・`session_end_sync_send_tests` | `HoldAt` 2 |
| `frame::balloon_runtime_test_support` | 2 | 2 | `input_events/balloon_pressed_tests`・`input_events/balloon_wheel_tests` | `manual_balloon_runtime` 1・モジュールごと 1 |
| `frame::test_support`・`frame::chain_finalize_persist_test_support`・`frame::zorder_drain::test_support`・`spine::test_support`・`spine::conformance_support`・`balloon_visibility::test_support`・`balloon_visibility_phase::test_support`・`crash_report_test_support`（`#[path]` で付けたモジュールのパス） | 0 | 0 | — | — |

- 合わせて 52 行・40 ファイル（支えごとのファイルの和は 48。`spine` と一緒に別の支えも引く次の 8 ファイルを重ねて数えない: `hold_support` の `session_end_deadline_tests`・`session_end_sync_send_tests`、`sample_test_support` の `ghost_session_reload_tests`・`ghost_session_restart_tests`・`session/ghost_switch_test_support`・`session/runtime_boot_tests`、`frame::balloon_runtime_test_support` の `input_events/balloon_pressed_tests`・`input_events/balloon_wheel_tests`）。
- 設計の段（2.7 節・`use` の行・`emo2_boot` の外）からの増え: `spine` 35→42・`sample_test_support` 4→6。主に `ghost_switch`・`runtime_boot` とそのテストが `session/` へ移り、`runtime` の外になったため（`session/` の 9 行＝`spine` 7・`sample_test_support` 2）。2.7 節は `use` の行だけを数えた下限で、今回は名指す行を全部数えたので、数え方の差も入る。`mcp/dump_surface.rs`（本番のファイル）の 3 行は前と同じくその中の `#[cfg(test)]` の部分。
- `session/ghost_switch_test_support.rs` は `session` の側の支えだが、それ自身が `runtime::spine` と `runtime::sample_test_support::acquire_emo2` を引く。`runtime-crate-split` で境をまたぐのは表の 4 つ（52 行）と、この支えの中の 2 行（表に含む）。

### 24. audit への申し送り（要件 7.4）

作業中に直したくなった不具合と要らない項目は直さず、受け持つ audit の brief の末尾の節「runtime-module-untangle からの申し送り（2026-11-13）」へ書いた（5 件）。受け持つ audit が無いもの（2 件）は、完了の棚卸で `/kiro-discovery` で起票する。本体の外の crate の説明の文（1 件）は `runtime-crate-split` が直す（brief には書かない）。

| # | 中身 | 種類 | 申し送った先 |
|---:|---|---|---|
| 1 | `runtime/frame/wiring.rs` 322 行目の説明の文 `super::super::input_events::balloon` が古い（本当は `crate::input_events::balloon`） | 要らない項目 | `audit-emo2-boot-frame` |
| 2 | 揺れるテスト `runtime::spine::third_scope_talk_tests::spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits`（`-j 4` の 1 回で赤） | 不具合の候補 | `audit-emo2-boot-spine` |
| 3 | `session/shell_balloon_switch.rs` 37〜40 行目の出し直しが設計の 5 型より広い（10 個） | 要らない項目 | `audit-emo2-boot-switch` |
| 4 | `update::update_cue`・`readme::readme_cue` が記録の決まり `areka::update`・`areka::readme` の前方一致の下に入った＝`info!` を足すと記録に残る | 気を付ける所 | `audit-areka-install-update` |
| 5 | `crates/areka-placement/src/diag.rs` 136 行目の説明の文の古い出所 `areka::emo2_boot::frame` | 要らない項目 | `runtime-crate-split`（要件の Out of scope・本体の外の crate の説明の文はそこで直す） |
| 6 | cargo のマニフェストの警告（使われない依存）: `shiori-abi` の `windows`・`shiori-host32-host` の `event-listener` | 要らない項目 | `audit-shiori-host32` |
| 7 | cargo のマニフェストの警告（使われない依存）: workspace の `async-executor`（根の `Cargo.toml`・受け持つ audit が無い） | 要らない項目 | 完了の棚卸で `/kiro-discovery` 起票 |
| 8 | 前からある古い行番号（`runtime/assets.rs:55`・`:82`・`:291`・`frame/zorder_descript.rs:61`・`spine_conformance_script.rs:589` ほか）が網羅台帳 `doc/ukadoc-coverage/ledger/` と手書きの briefing-*.md に残る（merge-base でも同じ・`doc/` を受け持つ audit が無い） | 要らない項目 | 完了の棚卸で `/kiro-discovery` 起票 |

## テストの名前の比べと静的な門（後）（2026-11-13・タスク 9.4・要件 1.4・8.1・8.2・8.6・8.7）

### 25. テストの名前の多重集合（後）と 15 節の「前」との比べ（要件 1.4・8.1）

- 測った木: `5ad1afc922c2912909c89d89439978ef548e799f`（9.3 の後）。道具は 15 節と同じ `cargo 1.100.0 (3acc79c0c 2026-11-05)`。作り方も 15 節と同じ（`cargo test --workspace --no-fail-fast -- --list`＝並べるだけでテストは走らない）。
- 置き場: `.kiro/specs/runtime-module-untangle/test-name-tails-after.txt`。
  - 行の数: 13,926（前と同じ）。異なる名前の数: 13,813（前と同じ）。`: benchmark$` の行は 0 行。
  - SHA-256: `f413befe34efe5970430b1572e74a235648fc292ab5eb882f388acca6efabcbe`
- `diff test-name-tails-before.txt test-name-tails-after.txt` の差は「−12 行・＋12 行」だけで、その 12 組は「名前を改めたテストの新旧の対応表」の 12 行とそのまま一致する。機械でも確かめた: 対応表の 12 行を `sed` の置き換え（`s/^旧$/新/`）にして「前」へ当て、バイト順に並べ直したものが「後」とバイトで一致する（`cmp` が差なし）。
  ```sh
  awk '/^### 名前を改めたテストの新旧の対応表/{f=1;next} f&&/^## /{f=0} f&&/^\| `/' research.md | awk -F'`' '{print "s/^"$2"$/"$4"/"}' > map.sed   # 12 行
  sed -f map.sed test-name-tails-before.txt | LC_ALL=C sort | cmp - test-name-tails-after.txt   # 差なし
  ```
- 土台の取り込みによる差: 無い。`git log --merges 4863cb66..HEAD` は 0 件（このブランチは土台 `4863cb66` から取り込みも rebase もしていない・`git merge-base HEAD origin/main` も `4863cb66` のまま）。`crates/areka`・`.kiro`・`doc`・`tools`・`.claude` の外でこのブランチが変えたのは 4 ファイル（`areka-placement/src/test_crash_report.rs`・`wintf/src/test_crash_report.rs` の説明の文、`log-capture-kit/tests/file_length_guard_test.rs`・`workspace_scan_test.rs` のパスの文字列 `emo2_boot/`→`runtime/`）で、どれもテストの名前を変えない。

### 26. 警告・1,000 行の番人・Cargo の差（要件 8.2・8.6・8.7）

- 警告（要件 8.2）: `cargo check --workspace --all-targets` は成功（終わりの値 0）。rustc の警告は **0 件**。マニフェストの警告は 15 節と同じ 3 件のまま（workspace の `async-executor`・`shiori-abi` の `windows`・`shiori-host32-host` の `event-listener`・24 節の 6・7 で申し送り済み）。
- 1,000 行の番人（要件 8.6）: `cargo test -p log-capture-kit` は全部緑（`file_length_guard_test` 6 passed を含む・失敗 0）。例外表の項目は土台 `4863cb66` も後も 5 件（`frame_transition_branch_tests.rs`・`window_move.rs`・`transition_judge_tests.rs`・`transition_judge_verdict_tests.rs`・`pilot-clickthrough-alpha-toggle/main.rs`）で増えていない。`git diff 4863cb66 -- crates/log-capture-kit/tests/file_length_guard_test.rs` の差は、例外表の 1 項目と較正の定数 `CALIBRATION_DROPPED_ENTRY` のパスの `emo2_boot/`→`runtime/` だけ（項目の数は同じ）。
- Cargo（要件 8.7）: `git diff 4863cb66 -- Cargo.lock` は空。`git diff 4863cb66 -- '*Cargo.toml'` の差は `crates/areka/Cargo.toml` のコメント 3 行（`emo2-boot 統合結線`→`runtime 統合結線`・`emo2-boot の実 sink 結線`→`runtime の実 sink 結線`・`emo2_boot::spine`→`runtime::spine`）だけで、依存・機能・版は変わっていない。

## 27. 全体テスト（9.5・2026-11-13）

- `pwsh -NoProfile -File tools/test-all.ps1` を 1 回（commit `d1ed02f97`・排他テストの机を取って）。13 段のうち赤は「x64 workspace tests」の 1 段だけ。ほかの 12 段（fmt --check・i686・encoding ほか）は緑。
- 赤のテストは本 spec が触っていない 2 つの crate だけ: `areka-ghost`（lib 5 本・`tests/ghost` 6 本）と `shiori4-testdll`（lib 1 本 `built_cdylib_appears_at_single_canonical_deps_dir`）。`git diff 4863cb66 -- crates/areka-ghost crates/shiori4-testdll` は空。`crates/areka` を含むほかの crate は全部緑。
- 赤の文: 「built test DLL が正準位置に不在: …\target\debug\build\areka-ghost\<hash>\out\shiori4_testdll.dll」。テストの実行ファイルが `target\debug\build\<pkg>\<hash>\out\` に置かれている（cargo 1.100.0・2026-11-05 の build-dir の置き方）ため、`current_exe` から求める正準位置（deps）がずれる。DLL 自身は `target\debug\deps\shiori4_testdll.dll` にある。`.cargo/config.toml` に build-dir の指定は無い。
- 決まり（触っていない crate だけが赤 → 赤の crate だけもう 1 回）どおり `cargo test -p areka-ghost -p shiori4-testdll` を回し直し、同じ 5 本が同じ文で赤（変化なし）。本 spec の受け持ちではないので申し送りとし、本 spec の全体テストは緑とみなす。完了の棚卸で `/kiro-discovery` に起票する（道具の版の上がりで落ちたテストの直し）。
- 8.1 で 1 度だけ赤だった揺れのテスト `spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits` は今回緑。
- 追記（同日）: 開発者より「main では解決している」。`origin/main`（`d6777fc56`）を取り込み（merge `7c4abcb71`・衝突は `tech.md` の 1 か所＝main の文言に新しいパス `runtime/spine_wait.rs` を当てた）、`cargo test -p areka-ghost -p shiori4-testdll` は全部緑、`cargo test -p areka` は 3171 passed・0 failed、`cargo check --workspace --all-targets` は rustc の警告 0（マニフェストの 3 件は前のまま）、fmt は差なし。上の 2 crate の赤は申し送り不要になった（起票しない）。全体テストの通しは最終の検証（`/kiro-validate-impl`）で回す。

## 終わる道 3 つの記録の順（後）と実機テスト 2 つ（2026-11-13・タスク 9.6・要件 3.6・8.4・8.5・8.8）

### 28. 実機テスト 2 つ（要件 8.4・8.8）

- 測った木: `e9c953d3a`（main の取り込みの後）。`cargo build -p areka`（`target\debug\areka.exe`・11-13 16:35）と `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` の後、排他テストの机を取って 1 回ずつ回した。走らせる前に 127.0.0.1:9276 を握るものが無いことを `netstat -ano` で確かめた。

| テスト | 回し方 | 結果 |
|---|---|---|
| `smoke_boot_loop_exit` | `cargo test -p areka --test smoke_boot_loop_exit` | 5 passed・0 failed（①〜⑤ の 5 方向） |
| `emo2_real_run` | `$env:AREKA_EMO2_REAL_RUN="1"; cargo test -p areka --test emo2_real_run -- --nocapture` | 14 passed・0 failed（実走 `emo2_real_run_boots_talks_and_exits_zero` を含む） |

- `emo2_real_run` の時間の 1 行: `wire_ms=850 attach_ms=1054 wrap_ms=1935 close_ms=19391 talk_to_wrap_ms=81 attach_to_talk_ms=820 primary_dpi=192`。

### 29. 終わる道 3 つの行の順（後）と 17 節（前）との比べ（要件 3.6・8.5）

- 道具・`RUST_LOG`・手順は 17 節と同じ。違うのは根だけで、`target\test-roots\rmu-after-<道>\` に置いた（17 節の道具の `rmu-$Path` を `rmu-after-$Path` に替えた写しで走らせた）。`RUST_LOG` は 17 節の文字列のまま:
  ```
  info,areka::app_exit=debug,areka::emo2_boot::frame=debug,areka::runtime::frame=debug,areka::session::ghost_quit=debug
  ```
- 終わる段取りの行の文言は前から変わっていない（`git grep` で確かめた: `session/ghost_quit.rs:93`・`runtime/frame.rs:211`・`app_exit.rs:136`・`:299`・`ghost_session.rs:655`・`smoke_exit.rs:37`）。ゆえに `exitlines.ps1` の `$plain` は 17.3 節のまま使った。

| 道 | pid | 終わり（`app_exit` の `origin`） | 終了コード | 行の数（後／前） |
|---|---|---|---|---|
| 煙の自動終了 | 30200 | `Smoke`（closed=4） | 0 | 12／12 |
| 窓を閉じる | 12872 | `KanadeStopped(Quit)`（closed=4） | 0 | 18／18 |
| メニューの終了 | 27896 | `KanadeStopped(Quit)`（closed=4） | 0 | 16／14 |

比べ方: 抜き出しを UTF-8（BOM なし）でファイルへ出し、行末の空白を落とし、出所を design.md「LogTargetFollow」の表で前の名へ読み替えて（`areka::session::ghost_quit`→`areka::emo2_boot::frame`・`areka::runtime::frame`→`areka::emo2_boot::frame`）、17.1 節の行と位置どうしで比べた。

- **煙の自動終了**: 12 行すべて同じ順・同じ文言・同じ event。
- **窓を閉じる**: 18 行すべて同じ順・同じ文言・同じ event（`steady_close_pending`→`steady_talk_done_close` の 2 行は前も後も在る）。
- **メニューの終了**: 後は前より 2 行多い。多い 2 行は先頭の `steady_close_pending`→`steady_talk_done_close`（「終了」を押したとき台詞を再生中だった）。17.2 節のとおりこの 2 行の有る無しは段取りの順の違いではない。この 2 行を除くと 14 行すべて前と同じ順・同じ文言・同じ event。
- 読み替えた出所の行は後で次のとおり出た: `INFO | areka::session::ghost_quit | kanade の終了系列が完了した: 全ゴースト窓を閉じる | ghost_quit`（窓を閉じる・メニュー）と `DEBUG | areka::runtime::frame | 終了が指示済み——毎フレームの処理を読み飛ばす | frame_phases_skipped_after_exit`（3 つとも）。`areka::emo2_boot::frame` の行は 1 行も無い。
- **結論: 3 つの道とも、終わる段取りの行の順は前（17 節）と後で同じ**。違いは出所の名の読み替えだけで、それは LogTargetFollow の表のとおり。

後のメニューの道の行（読み替える前）:

```
INFO | kanade | close 指示——active talk あり・保留記録（TalkDone を待つ） | steady_close_pending
INFO | kanade | talk 完了——保留 close を消化し握手開始 | steady_talk_done_close
INFO | kanade | OnClose GET を発行し握手を開始 | close_handshake_begin
INFO | kanade | OnClose 応答スクリプト——close talk を再生起動し完了を待機 | close_talk_start
INFO | kanade | reason=Quit——終了系列（Quit）へ | talk_done_quit
INFO | shiori-actor | 正規 clean shutdown 完了（unload が正常終了を返した） | unload_clean
INFO | shiori-actor | 停止指示（Close）を受領——即時停止（接続資材を RAII teardown） | close
INFO | areka::session::ghost_quit | kanade の終了系列が完了した: 全ゴースト窓を閉じる | ghost_quit
INFO | wintf::ecs::app | [App] Last window closed. |
INFO | areka::app_exit | [quit_app] 全窓を閉じ、終了を指示した | app_exit
INFO | wintf::runtime::message_loop | [AppExit] exit requested |
DEBUG | areka::runtime::frame | 終了が指示済み——毎フレームの処理を読み飛ばす | frame_phases_skipped_after_exit
INFO | areka::ghost_session | seriko: loop ticker を Close しました（終了順序①・SERIKO 再生ループ停止） |
INFO | ghost-shutdown | persist flush confirmed |
INFO | ghost-shutdown | ghost shutdown sequence completed |
INFO | areka_mcp::server | MCP: 待受を閉じた |
```

煙の自動終了と窓を閉じるの後の行は、読み替えると 17.1 節の行と 1 行ずつ同じなので写さない（出所が変わった行は上の 2 種類だけ）。

### 30. メニューの道の手順の学び

- 1 回目は失敗した（pid 36988・保険の 120 秒の自動終了 `origin=Smoke` で終わった・行の比べには使っていない）。右クリックでメニューは出た（`menu_shown items=11`）が、手順の段ごとに別の `pwsh` を起こし、撮った絵を見てから次へ進んだため、「終了」の点を確かめた時には `WindowFromPoint` が explorer（メニューの下の窓）を返し、約 30 秒後にカーソルを動かして確かめ直した時にはメニューが消えていて、左クリックはキャラの体への普通のクリック（`OnMouseClick`）になった。害は無い。机を持ったままのやり直しはせず、机を返してから道具を直した。
- 直した形（2 回目・成功）: 子を起こすところから「終了」を押すところまでを 1 つのプロセスで続ける。`tools/mouse-input.ps1` は `&` で同じプロセスの中から呼び、段のあいだに別の `pwsh` を起こさない。どの点も**先にカーソルを動かしてから** `WindowFromPoint` で pid を確かめる（メニューの窓もキャラの窓と同じく、カーソルが乗る前は下の窓が返る）。「終了」の点は、子の pid でかつキャラの窓とは別の窓（＝メニューの窓）のときだけ押す。2 回目は体の点＝キャラの窓（hwnd 28052968）、「終了」の点＝別の areka の窓（hwnd 23464632）で、押して終わった。
- 道具（`exitrun-menu.ps1`）は 17.4 節の道具の根を `rmu-after-$Path` に替え、menu の枝の窓の一覧を出した後に次を足したもの（17.4 節との差の全部）:

```powershell
        # 9.6 retry: the whole menu operation in this one process (no pwsh spawned between steps).
        Add-Type -Namespace RmuW32 -Name P -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(System.Drawing.Point p); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);' -ReferencedAssemblies System.Drawing.Primitives
        function PidAt([int]$x, [int]$y) { $h = [RmuW32.P]::WindowFromPoint([System.Drawing.Point]::new($x, $y)); $o = [uint32]0; [RmuW32.P]::GetWindowThreadProcessId($h, [ref]$o) | Out-Null; "$h/$o" }
        $tool = "$wt\tools\mouse-input.ps1"
        # The character window is the one whose bottom touches the work area bottom (1.2: 1392,904,2064,1704).
        $char = $found | Where-Object { $_ -match 'rect=1392,904,2064,1704$' }
        if (-not $char) { Write-Output 'abort character window rect differs from 1.2'; Wait-Exit; return }
        $bx, $by, $qx, $qy = 1592, 1304, 1684, 1280   # body point and the Quit item, as in 1.2
        & $tool -Action move -X $bx -Y $by; Start-Sleep -Milliseconds 300
        $a = PidAt $bx $by; Write-Output "body $a"
        if ($a -notmatch "/$target$") { Write-Output 'abort body pid mismatch'; Wait-Exit; return }
        & $tool -Action rightclick -X $bx -Y $by; Start-Sleep -Milliseconds 1000
        & $tool -Action shot -X 1392 -Y 904 -X2 2064 -Y2 1704 -Out "$run\menu.png"
        & $tool -Action move -X $qx -Y $qy; Start-Sleep -Milliseconds 300
        $b = PidAt $qx $qy; Write-Output "quit $b"
        # The Quit item must be on an areka window other than the character window itself.
        if ($b -match "/$target$" -and $b -notmatch "^$(($char -replace '^window hwnd=(\d+).*$','$1'))/") {
            & $tool -Action click -X $qx -Y $qy
        } else { Write-Output 'abort quit point is not the menu window (not clicking)' }
```

- 走らせ方: `pwsh -NoProfile -File exitrun-menu.ps1 -Path menu`（前で走らせると子が終わるまで戻らない）。座標は物理画素（主のモニタは 200%）で、窓の矩形もメニューの位置も 1.2 と同じだった。

## 31. 最終の検証（kiro-validate-impl・2026-11-13）

- 判定: GO。全体テスト `tools/test-all.ps1` は commit `ac4aaaac7`（main 取り込み後）で 14 段すべて緑（x64 workspace tests 650 s・`smoke_boot_loop_exit` を含む）。
- タスクをまたぐ結線（口 10 本の使い手 7 相・据え付けの順・`BootRuntimeFn` の唯一の呼び手・終わる相の登録の順・記録の分け方）・要件 52 項目の網羅・File Structure Plan と依存の向き（式 1 は `#[cfg(test)]` の 2 行だけ）・境界（Cargo はコメント 3 行・Cargo.lock 差なし）はいずれも指摘 0。足した行に TBD/TODO・秘密の字面は 0。
- 完了の棚卸で `/kiro-discovery` に起票するもの: 根の `Cargo.toml` の使われない依存 `async-executor` の警告、網羅台帳と briefing-*.md の前からの古い行番号（24 節の 7・8 番）。
