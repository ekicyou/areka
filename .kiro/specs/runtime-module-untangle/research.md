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
