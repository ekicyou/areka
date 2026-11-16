# ギャップ分析: runtime-crate-split

- 測った日: 2026-11-16。測った木: ワークツリーの枝 `claude/kiro-start-runtime-crate-split-5fadf0`（`2bddb06c`＝main `5b3a361c` に spec の初期化を 1 つ足しただけ・runtime のコードは main と同じ）。
- 測り方: Grep・Glob・Read と、使い捨ての python の数え上げ（コメントを除き、`crate::runtime::…` の道を use の木ごと開いて数えた）。数は「コンパイラに数えさせる前の見積り」で、確定のしかたは 6 章に書いた。
- 要件（requirements.md）と spec.json は書き換えていない。ここに書くのは事実と案で、決めは要件の討議と設計の段に回す。前例は完了した `placement-crate-split`（`.kiro/specs/completed/placement-crate-split/research.md`・`design.md`）。

---

## 1. 要約

- **移す形そのものは前例どおり素直**: runtime は 211 ファイル・74,693 行（本番 58 ファイル・テスト 138・テストの支え 9・背骨と判定の支え 6）。本番から本体のほかの部分を引く `crate::` は 0（本番の説明の中のリンク 11 か所だけ）。外の crate の使い方も要件 1.8 の一覧どおり（`areka-update`・`areka-mcp` ほかは 0）。新しい crate の骨組み・`git mv`・中の `crate::runtime::` の書き換え（73 ファイル・233 か所）・本体の書き換え（101 ファイル・218 か所＋`use crate::runtime;` 経由の素の `runtime::` 約 10 行）は機械の作業で済む。
- **一番大きい食い違い＝テストの置き場とテストの支え**（未決の議題 2・3・4 は 1 つの問題）: 本体を引く runtime のテストは **40 ファイル**（要件の「約 40」と一致）。うち 25 は親の本番モジュールの子（`use super::*`）で、私有の項目と runtime の中の支え（`frame` の `test_support` 931 行ほか）を使う。逆に、その支えは `log_capture_kit`・`sample_ghost_kit` を引くので、`test-support` 機能では開けない（前例と同じ番人の衝突）。どちらへ置いても「開ける」か「割る」かの仕事が出る。
- **`spine` は 3 層に割れる**: ⑴ 待ちの部品と GPU の許可（`spine_wait.rs`・std と `areka_actor` と落ちの報告だけ）⑵ 台本の SHIORI（`ScriptedShioriBackend`・`hold_support`・`shiori_host32_host` の型）⑶ 背骨の足場 `SpineHarness`（`log_capture_kit`・`sample_ghost_kit`・本体の `runtime_boot_options`・`KanadeNoticeRx`）。本体から使われるのは主に ⑴ ⑵（38 ファイル中 36）で、⑶ は 2 ファイルだけ。⑴ ⑵ は番人に触れずに機能で開けられる見込み、⑶ は開けない。
- **口の倒れ先（`FALLBACK`）に頼るテストは 15 ファイル**: 本体の状態（`GhostSlot`・`BootContext`・`SwitchInFlight`・`session::reload`）を World に入れ、口を据えずに runtime の相を回す。どれも上の 40 に入る。本体に置いても、`areka-runtime` は本体のテストのビルドでは `cfg(test)` が立たないので倒れ先は `NO_BODY`＋`error!` になり、そのままでは赤になる見込み。
- **実機テストの 3 本目は本物**（未決の議題 1）: `stay_on_top_real_run.rs` の `RUST_LOG` は `areka::runtime::frame::window_state=debug` ほか 6 つを名指し、その `debug!` の文（`[window_state] ゴーストの窓を今いる帯の最上へ出す`）を数える。書き換えないと 0 件で赤になる。`smoke_boot_loop_exit.rs`・`emo2_real_run` は既定の `info` で本文を読むだけで、出所の名前に頼らない。

---

## 2. 今の姿

### 2.1 runtime のファイルと行き先の候補

| まとまり | ファイル数 | 行き先の候補 |
|---|---|---|
| 本番（`mod.rs`・`frame/` の本番を含む） | 58 | `crates/areka-runtime/src/` |
| 本体を引かないテストと支え | 約 107 | `crates/areka-runtime/src/` |
| 本体を引くテスト（2.5 節） | 40 | **未定**（議題 4） |
| 背骨と判定の支え（`spine.rs`・`spine_wait.rs`・`spine_hold_support.rs`・`spine_conformance_{judge,script,support}.rs`） | 6 | **未定**（議題 2） |
| `crash_report_test_support.rs` | 1 | `areka-runtime`＋本体にも要る（2.9 節） |

- 本体の中の宣言: `main.rs` 58 行の `mod runtime;`。`main.rs` は 987 行（上限まで 13 行）。`runtime/mod.rs` は 850 行（`lib.rs` になる）。
- `session/runtime_boot_tests.rs` 264・268 行が `#[path = "../runtime/script_security_e2e_tests.rs"]`（166 行）・`readme_script_e2e_tests.rs`（138 行）を取り込む。2 本とも `session::ghost_switch_test_support` を引く＝本体の側（`session/`）へ移す（要件 3.7）。
- `examples/collision-probe.rs` 218・224 行が `#[path = "../src/runtime/target_map.rs"]`・`hit_region.rs`。付随の `collision-probe/` は 7 ファイル。使う crate は `areka_placement`・`wintf`・`areka_emo_{atlas,compose,present}`・`bevy_ecs`・`windows`・`async_io`・`sample_ghost_kit`・`tracing_subscriber`（どれもワークスペースの中か `Cargo.lock` に在る）。

### 2.2 外から使われる公開面（見積り）

数え方: 本体の `src`（runtime を除く）・`examples`・`tests` の `crate::runtime::…` を use の木ごと開いた（素の `runtime::…`・メソッドの呼び出しは数えていない）。

| 種類 | 数 | 内訳 |
|---|---|---|
| 外から使われる道（重なりを除く） | **124** | 本番のファイル 30 から 79、テストのファイル 70 からだけ 45 |
| `pub(crate) mod` → `pub mod` | **19**（本番）＋2（テスト） | `balloon_hover`・`change_cue`・`reload_args`・`reload_cue`・`scope_scale`・`session_hooks`・`shell_balloon_resolve`・`shell_box_assets`・`shell_box_hover`・`shell_carry`・`shell_cursor_assets`・`shell_cursor_table`・`skin_facts_copy`・`skin_switch`・`special_name`・`strict_judges`・`switch_assets`・`user_break_cue`・`window_state_cue`／テストの `spine`・`sample_test_support` |
| 外から使われない `pub(crate) mod` | 1＋私有 1 | `script_restriction`（要件 2.2 で残す）・私有の `balloon_background` |
| runtime の中の `pub(crate)` の宣言 | 301（テストでないファイル 236） | 外から使われるのはその一部。`pub(super)` は 324 |
| `#[cfg(test)]` の項目で外から使われるもの | **4** | `spine`（39 ファイル）・`sample_test_support`（6）・`frame::balloon_runtime_test_support`（2）・`assets::empty_definitions`（1）。本体のテストのビルドでは `areka-runtime` の `cfg(test)` は立たないので、どれも `cfg(any(test, feature = "test-support"))` へ替える要がある |
| `RuntimeWiring` のメソッド | 約 30 本の名前が外で呼ばれる | `new`・`presenter`・`runtime`・`drain_received`・`balloon_model_scopes` ほか。`apply_present` は `#[cfg(test)]` だが外の使い手は 0 |

- 本番だけの面（本体の本番 30 ファイルが使う）は要件 2.1 の主な面（`RuntimeWiring`・`ZOrderAbsentReports`・`TalkLifecycleSignal`・`NoUserBreakSignal`・`HitRegion`・`resolve_hit_region`・`ConsumerLedger`・`target_map`）に加えて、`CueSinkSources`・`RuntimeBootError`・`RuntimeInputs`・`skin_switch` の 11 名・`shell_balloon_resolve` の 10 名・`shell_cursor_table` の 5 名・`reload_args` の 5 名ほか。`run_ghost_quit_phase` は今は `session/ghost_quit.rs` の持ち物（`runtime-module-untangle` で移った）で、runtime の公開面ではない＝要件 2.1 の一覧の 1 語は古い。
- テストだけが使う 45 の道（`adapter`・`assets`・`frame`・`move_cue`・`reload_args`・`shell_*`・`spine`・`switch_assets`・`talk_clock`・`target_map`・`window_state_cue`・`zorder_cue`）は、`pub` にするか `test-support` の下に置くかの選択がある（前例は `pub` にした）。
- `private_interfaces`（Rust 2024 の既定の警告）: `pub` にした関数が `pub(crate)` の型を名指すと警告になる（要件 2.5）。確定はコンパイラに数えさせる。

### 2.3 テストの支えと番人（未決の議題 2）

**`spine` の 3 層**（`spine.rs` 931 行＋兄弟 20 ファイル・計 9,123 行）

| 層 | 中身 | 頼る crate／本体 | 外の使い手（ファイル） | `test-support` で開けるか |
|---|---|---|---|---|
| ⑴ 待ちと GPU の許可 | `spine_wait.rs`（631 行）の `wait_until`・`wait_until_with`・`wait_recv`・`run_bounded`・`run_bounded_watching`・`join_thread_bounded`・`Progress`・`WaitFailure`・`GpuPermit` | std・`areka_actor`・`crash_report_test_support`（許可の持ち手の表） | 本体 約 30＋runtime の中の非 spine 13 | **開ける見込み**（番人の crate を引かない） |
| ⑵ 台本の SHIORI | `ScriptedShioriBackend`・`Builder`・`Handle`・`RecordedCall`・`HOMEURL_RESOURCE`・`hold_support::HoldAt` | `areka_kanade`・`shiori_host32_host` の型（本体では dev-dependency。`areka-kanade`・`areka-ghost` の本番の依存なので出荷の閉包には既に在る・番人は無い） | 本体 約 25 | **開ける見込み**（`shiori-host32-host` を optional で持つ形。本体の `Cargo.toml` の説明の文は直す） |
| ⑶ 背骨の足場 | `SpineHarness`・`spine_conformance_*`・`spine_test_support.rs` | `log_capture_kit::capture_lines`・`sample_ghost_kit::SampleRoot`・本体の `session::runtime_boot::runtime_boot_options`（`GhostBootOptions` を組むだけ・中は `runtime::strict_judges::production` と `areka_ghost` の型）・`session::ghost_quit::KanadeNoticeRx`（受信端を入れる資源） | 本体 2（`ghost_session_reload_tests.rs`・`ghost_session_restart_tests.rs`）＋runtime の中の spine のテスト 13 | **開けない**（番人 2 つと、本体の 2 項目） |

- 本体の外のテストで `runtime::spine` を書くのは **38 ファイル・44 か所**（brief の 28・46 は 11-13 より前の数）。`spine::hold_support` 2（`session_end_deadline_tests`・`session_end_sync_send_tests`）。`mcp/dump_surface.rs` は本番のファイルの中の `#[cfg(test)]` から `spine::wait_until`（426 行）を呼ぶ。
- **runtime の中のほかの支えも同じ衝突を持つ**: `frame` の `test_support`（`frame_test_support.rs` 931 行・`log_capture_kit::{CapturedEvent, capture_lines}`）・`frame/zorder_drain_test_support.rs`（`capture_lines`）・`sample_test_support.rs`（`SampleRoot`）。本体へ置くテストがこれらを使うなら（2.5 節＝25 ファイルが `test_support::` を使う）、同じ割り方（番人の crate を引く部分と引かない部分）が要る。`frame_test_support.rs` で番人の crate を引くのは `capture` の包み（117 行）と `CapturedEvent` を受ける 2 関数（912・921 行）だけ。
- `frame_chain_finalize_persist_test_support.rs` は本体の `crate::insert_persist_wiring`（`main.rs`）を呼ぶ＝下の crate には置けない（使い手の `frame_chain_finalize_persist_*` 4 本は 2.5 節の 40 に入る）。
- 見本の入口 `sample_test_support`（39 行・`emo2_root`・`emo2_balloon_root`・`acquire_emo2`）は、ワークスペースの慣習どおり crate ごとに 1 つ（要件 3.3）。本体の外の使い手 6 ファイル（`ghost_session_{reload,restart}_tests`・`session/ghost_switch_test_support`・`session/runtime_boot_tests`・`placement_tests/{measure_bake_count,reset_balloon_pos}_tests`）。`acquire_emo2` は `spine.rs` も使う。
- 逆向きの支え（要件 3.9）: runtime のテストが使う配置の `test_support` は今は **2 ファイル**（`frame_window_link_drain_tests` の `WA`・`with_com_initialized`、`spine_third_scope_talk_tests` の `emily_shaped_scopes`。brief の 24 回は前の形）。`shared_test_support` は 0、`scope_bind_fixture_test_support` は 2（`assets_tests`・`assets_shell_tests`＝本体の支え）、`install::fetch_url_test_support` は 0、`crate_source_test_support` は 1（`film_playback_e2e_tests`）。`areka-runtime` の dev-dependency に `areka-placement`（`test-support` つき）が要る。
- 番人（要件 3.4）: `with_default_guard_test.rs` の `the_shared_crate_never_appears_in_a_production_dependency_table` と `sample_path_guard_test.rs` の同名の番人。`with_default_guard_test.rs` 474 行の `KNOWN_DEV_DEPENDENTS`（較正の一覧）は網羅を求めない（足さなくても赤にならない）。どちらの番人の表にも runtime のパスは無い。

### 2.4 口の倒れ先（未決の議題 3）

- `runtime/session_hooks.rs`（78 行）: `SessionHooks::of` は口が World に無いとき、本番（`#[cfg(not(test))]`）は `error!(event = "session_hooks_missing")` を出して `NO_BODY`、テスト（`#[cfg(test)]`）は記録を出さず本体の `crate::session::hooks::HOOKS` を返す。
- 口を読む本番の所: `balloon_visibility_phase.rs`・`frame/attach.rs`・`frame/scale_notice.rs`・`frame/status_report.rs`（2）・`frame/switch.rs`・`frame/window_state.rs`・`shell_carry.rs`。
- 口を据えるのは `ghost_session::register_systems` の先頭（`ghost_session.rs` 68 行）の 1 か所だけ。runtime のテストで据えるものは 0。
- **倒れ先に頼る runtime のテスト（本体の状態を World に入れて口を据えない）: 15 ファイル** ＝ `balloon_visibility_phase_notice_tests`・`frame/status_report_stage_tests`・`frame/status_report_tests`・`frame/switch_finish_tests`・`frame/switch_reload_tests`・`frame/switch_tests`・`frame/window_state_tests`・`frame_attach_carry_tests`・`frame_move_order_tests`・`frame_scale_notice_integration_tests`・`frame_stay_on_top_lifecycle_tests`・`frame_visibility_integration_tests`・`readme_script_e2e_tests`・`shell_balloon_resolve_tests`・`shell_carry_tests`。全部 2.5 節の 40 に入る。
- 分けた後の形の事実: 本体のテストを組むとき、依存の `areka-runtime` は `cfg(test)` を立てずに組まれる（`test-support` 機能は立てられる）。よって本体に置いたテストが口を据えないと、倒れ先は `NO_BODY`＋`error!` になる（ログを捕まえるテストでは余計な error 行・本体の状態を読む確かめは偽になる）。本体の外のテストで口を据えずに runtime の相を回すものは、`shell_balloon_switch_session_lap_tests.rs` の 1 本が候補（`frame_system` を呼び `register_systems` の名が無い・支え経由で据えているかは未確認＝6 章）。

### 2.5 本体を引く runtime のテスト 40（未決の議題 4）

| 引く本体の部分 | ファイル（代表） | 数 |
|---|---|---|
| `ghost_session`（`GhostSlot`・`GhostSession`） | `balloon_visibility_phase_notice_tests`・`frame/status_report*_tests`・`frame/switch*_tests`・`frame_attach_carry_tests`・`frame_move_order_tests`・`frame_scale_notice_integration_tests`・`frame_visibility_integration_tests`・`shell_carry_tests`・`readme_script_e2e_tests` | 12 |
| `input_events`（`balloon`・`shell_box`・`char_watch`・`user_break`・`MouseWiring`） | `balloon_visibility_*_tests` 5・`frame/switch_cursor_tests`・`frame_shell_box_*_tests` 2・`frame_visibility_integration_tests` | 9 |
| `session`（`ghost_switch`・`shell_balloon_switch`・`reload`・`switch_copy`・`ghost_quit`・`ghost_switch_test_support`） | `frame/switch*_tests`・`frame_schedule_tests`・`frame_stay_on_top_lifecycle_tests`・`shell_balloon_resolve_tests`・`switch_cue_tests`・`spine_close_wiring_tests`・`spine_third_scope_talk_tests`・e2e 2 本 | 13 |
| `app_exit`・`restore_merged_placements`・`insert_persist_wiring` | `frame_chain_finalize_*_tests` 5・`frame_schedule_tests`・`spine_third_scope_talk_tests` | 7 |
| `boot_config`・`boot_resolve`・`roots` | `frame/switch_finish_tests`・`frame/window_state_tests` | 2 |
| `install`・`update`・`readme` の cue／`desk` | `consumer_ledger_agreement_tests`・`zorder_wiring_tests` | 2 |
| `log_history`・`mcp::check_script_judge` | `strict_judges_tests` | 1 |
| `scope_bind_fixture_test_support`・`crate_source_test_support`・`menu::reset_pos_frame` | `assets_tests`・`assets_shell_tests`・`film_playback_e2e_tests`・`frame_move_order_tests` | 4 |
| `placement_tests::transition_judge`（判定器） | `frame_transition_atomicity_tests`・`frame_transition_branch_tests` | 2（判定器を移せば本体を引かなくなる） |

（1 ファイルが複数の行に入る。重なりを除いて 40。）

- 40 のうち **25 が `use super::*`**（親の本番モジュールの子のテストで、私有の項目に届く）。ほかもほぼ全部 `super::` で親の項目を引く（`super::run_switch_phase`・`super::{finalize_chain_once_with, resnap_with}`・`super::super::BalloonVisibilityState` ほか）。上の crate へ移すと、私有の項目は外から見えなくなる。
- 40 のうち **25 が runtime の中の支え（`frame` の `test_support::`・`chain_finalize_persist_test_support::` ほか）を使い**、19 が `log_capture_kit`・`sample_ghost_kit`・`temp_path_kit`・`sample_test_support` を直に使う。
- 本体を引く部分は本物の部品（入力の出来事の受け手・置き場のゴースト・終了の手続き・切替の予約）で、偽物に替えるとテストの中身が変わる（要件 7.1）。
- 1,000 行: `frame/switch_finish_tests.rs` は 999 行。本体へ移すと `super::`（7 字）が `areka_runtime::frame::`（22 字）になり、折り返しで上限を超えるおそれがある（例外表は増やせない＝分けて収める）。逆に `crate::runtime::`（16 字）→ `areka_runtime::`（15 字）・`crate::`（7 字）は短くなる。

### 2.6 `placement_tests/` と判定器の行き先（要件 4.2〜4.5・未決の議題 7）

`crates/areka/src/placement_tests/` は 26 ファイル（`mod.rs` を含む）。

| ファイル | 本体を引く所 | 文字で読む所 | 行き先の候補 |
|---|---|---|---|
| 判定器の一族 15（`transition_judge.rs` 999 行・`_offset.rs` 813・`_verdict.rs` 870・`_test_support.rs`・テスト 9・`transition_signoff_tests.rs`・`transition_signoff_procedure_tests.rs`） | `transition_judge.rs`・`transition_judge_tests.rs` が `crate::runtime::target_map` だけ | `transition_judge_offset_tests`（兄弟）・`transition_signoff_procedure_tests`（`../../.kiro/specs/completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md` と `src/placement_tests/transition_judge_offset_signoff_tests.rs`） | `areka-runtime` のテスト専用（要件 4.2）。本体を引かなくなる。`.kiro/` を読む相対の深さは同じ（`crates/<x>/`）。中の `src/placement_tests/…` の名指しは移した先へ |
| `reset_balloon_pos_tests.rs` | `runtime::switch_assets`・`runtime::sample_test_support` | なし | `areka-runtime`（要件 4.4） |
| `measure_bake_log_tests.rs` | `runtime::assets` | なし | `areka-runtime` |
| `measure_bake_count_tests.rs` | `runtime::assets`・`runtime::sample_test_support`・`crate_source_test_support::crate_path("areka-emo-compose", …)` | 下の crate の見本 | `areka-runtime`（ただし下の crate を読む読み口が要る＝2.8 節） |
| `measure_template_tests.rs` | なし | `../runtime/{frame_test_support,assets_tests,assets}.rs`・`../../examples/emo-present/setup.rs`（本体の examples）・`../../examples/collision-probe/setup.rs` | **本体**（本体の examples を読むので下の crate には置けない）。runtime の 3 つは読み口 `crate_source!("areka-runtime", …)` で |
| `spawn_assembly_tests.rs` | `input_events` | なし | 本体（`input-crate-split` で決め直す） |
| `spawn_zorder_pair_deferred_tests.rs` | なし | `CARGO_MANIFEST_DIR` からの `src/runtime/*` の名簿と `src` の木の走査 | `areka-runtime` に移すと、自分の `src` を歩けば名簿の全部（`zorder_cue`・`frame/zorder_drain`・`frame/zorder_descript`・`window_state_cue`・`frame/window_state`）が入る。本体の `src` には runtime の外に `zorder` の名のファイルが 0＝範囲は狭まらない。本体に残すなら木の走査を `areka-runtime` の `src` へ向け直す |
| 分岐の対応表 2 本（`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`）＋`zorder_branch_coverage_test_support.rs` | `crate_source_test_support` | runtime（相対 `../runtime/…`）・`areka-placement`・`wintf`（読み口 26 か所） | どちらも置ける。本体なら runtime を読み口で読む。`areka-runtime` なら下の 2 crate を読む読み口が `areka-runtime` に要る |

- 判定器を使う runtime のテスト 2 本（要件 4.3）は判定器を引く以外に本体を引かない＝判定器と同じ crate に置けば、本体を引かないテストになる。
- 1,000 行: `transition_judge.rs`・`transition_judge_tests.rs` はどちらも 999 行。道は短くなる向き（`crate::runtime::` → `crate::`）だが、モジュールの道（`crate::placement_tests::…`）の新しい名前の長さしだい。`transition_judge_verdict_tests.rs`（1,024 行）は例外表の項目（パスの書き換えだけ）。
- 置き場の名前（例: `crates/areka-runtime/src/placement_tests/`・`transition_judge/`）は前例の D1 と同じく設計の段で決める。下流の `audit-placement-follow-residue`（C22）は `placement_tests/transition_*` を触る＝roadmap の対応表で読み替える。

### 2.7 テストだけが使う `areka-placement` の公開（要件 4.8・未決の議題 6）

| 項目 | 本体の中の使い手（名前で数えた） | 分けた後の使い手の crate |
|---|---|---|
| `on_balloon_drag` | `runtime/frame_balloon_offset_follow_tests.rs` | `areka-runtime` |
| `sync_drag_writer` | `input_events/drag_sticky_tests.rs` | 本体 |
| `WORK_AREA_CLAMP_TAG` | `runtime/frame_chain_realign_adjacency_tests.rs` | `areka-runtime` |
| `AnchorSource` | `input_events/drag_sticky_tests`・`placement_tests/reset_balloon_pos_tests`・`placement_tests/spawn_assembly_tests`・`session/ghost_switch_reload_tests` | 本体 3・`areka-runtime` 1 |
| `WorkAreaResolution` | `runtime/frame_dpi_reproject_tests.rs` | `areka-runtime` |
| `work_area_for_window_with_origin` | `runtime/frame_dpi_reproject_tests`・`runtime/frame_transition_branch_tests` | `areka-runtime` |
| `move_window_with_route` | `runtime/frame_work_area_resnap_hold_tests.rs` | `areka-runtime` |
| `MonitorSources::is_empty` | 名前では拾えない（メソッド） | 設計で確かめる |
| `MonitorDpiEntry` | 名前の使い手 0（公開の欄 `MonitorDpiTable.entries` の要素の型・`main_monitor_snapshot_seam_tests.rs` が欄を読む） | 本体 |
| `transition_diag` の語・欄 | 判定器の一族 15・runtime のテスト 8・**runtime の本番 2**（`frame/balloon_offset_follow.rs`・`frame/work_area_sync.rs`） | `areka-runtime` が大半 |

- 判定器を `areka-runtime` へ移しても、使い手はどれも `areka-placement` の外のテストのまま（`areka-runtime` のテストか本体のテスト）。「使うテストを crate の中へ移す」は、使うテストが runtime の部品も使うので取れない。現実の案は「公開のまま」か「`areka-placement` の `test-support` 機能の下へ」（`areka-runtime` と本体の両方が dev-dependency で付ける）。
- `transition_diag` は runtime の本番が使う語もあるので、全部を機能の下へは移せない（本番が使う語は公開のまま）。

### 2.8 ソースを文字で読むテストと読み口（要件 5.2〜5.4）

runtime の中で文字で読むのは 15 ファイル。

| 読む先 | ファイル | 分けた後 |
|---|---|---|
| 自分（runtime）のファイル | `balloon_visibility_phase_zorder_chain_tests`・`frame/zorder_drain_tests`・`frame_attach_tests`（`frame/attach.rs`）・`frame_harness_tests`・`frame_move_order_tests`・`frame_transition_branch_tests`・`frame_work_area_resnap_tests`・`frame_work_area_sync_tests`・`window_state_cue_tests`・`zorder_cue_tests` | 同じ crate に置けば相対の道は変わらない |
| 本体のファイル | `frame_schedule_tests`（`../session/ghost_quit.rs`）・`zorder_wiring_tests`（`../session/runtime_boot.rs`・`../ghost_session.rs`・`../main.rs`） | 下の crate から上を読めない＝本体に置く（どちらも 2.5 節の 40 に入る） |
| 文書 | `strict_judges_tests`（`../../../../doc/ssp-mcp/areka-tools.md`） | `areka-runtime` に置くなら深さが 1 つ減る。ただし本体（`log_history`・`mcp`）を引く＝本体に置く見込み |
| 下の crate | `film_playback_e2e_tests`（`crate_path("areka-emo-compose", …)`） | `areka-runtime` に置くなら読み口が要る |
| ワークスペースを歩く | `frame_attach_tests`（`crates` を走査）・`frame_chain_finalize_persist_test_support`（根を `CARGO_MANIFEST_DIR` の 2 つ上で） | 深さは同じ。`crate_source_test_support.rs` の説明は `runtime/frame_attach_tests.rs` を例外の 3 本の 1 つとして名指す＝パスを書き換える |

- 本体の読み口 `crate_source_test_support.rs`（`crate_source!`・`crate_path`）の使い手は 7 ファイル。`areka-runtime` へ移すテストのうち下の crate を読むのは `film_playback_e2e_tests`（1 か所）・`measure_bake_count_tests`（1 か所）と、分岐の対応表 2 本を移すならその 26 か所。前例は「1 か所だけなので配置の側に読み口は作らない」とした。
- 読み口のマクロは `env!("CARGO_MANIFEST_DIR")` を呼び出し側で展開する（`macro_rules!`）ので、1 つの読み口をどちらの crate からも使える（`crates/<x>/../<krate>` は同じ所を指す）。読み口を `areka-runtime` の `test-support` に置いて本体がそれを使う・共有の kit に下ろす・`areka-runtime` には読み口を作らず相対で読む、の案がある（写しを作らない要件 3.3 との兼ね合い）。
- 「黙って通らない」（要件 5.4）: `include_str!` の先が無いとコンパイルが止まり、`crate_path` は panic する。危ないのは木の走査（`spawn_zorder_pair_deferred_tests`）だけで、2.6 節のとおり置き場で決まる。

### 2.9 落ちの報告と MTA（要件 3.8）

- `crash_report_test_support.rs`（588 行・`#[cfg(test)]`）は `.CRT$XCU` の `#[used]` 静的（127・128 行）で据わり、GPU の許可の持ち手の表（`hold`・`release`）を持つ。`spine_wait.rs` の `GpuPermit::new`・`drop` が `crash_report::hold`／`release` を呼ぶ＝**待ちの部品（2.3 節 ⑴）と一体**。
- `areka-placement`・`wintf` の写し（`test_crash_report.rs`・どちらも `#[cfg(test)]`・表なし）の 1 行目は、写しの相手として `crates/areka/src/runtime/crash_report_test_support.rs` を名指す。
- 分けた後: `areka-runtime` のテストの実行ファイルは移した写しを持つ。本体のテストの実行ファイルは、今は runtime の写しに頼っている（本体の `cfg(test)` で入る）ので、分けると失う。案は ⒜ 本体に 4 つ目の写しを置く ⒝ `GpuPermit` と一緒に `areka-runtime` の `test-support` で開き、本体のテストの実行ファイルにも初期化子が入るようにする。⒝ は「上流の rlib の `#[used]` の `.CRT$XCU` 静的が本体のテストの exe に残るか」と「機能つきの `areka-runtime` のテストの実行ファイルで 2 重に据わらないか」が未確認（6 章）。⒜ で本体が `GpuPermit` を `areka-runtime` から使うと、持ち手の表は `areka-runtime` の側にあり、本体の写しの表とつながらない。

### 2.10 ログの出所と実機テスト（要件 6・7.6・未決の議題 1）

- runtime の本番の出所は全部モジュールのパスから付く（`target:` の直書きは 0・`"areka::persist::*"` ほか語彙の直書きも 0）＝要件 6.2・6.3 の当てはまる所は 0 件。
- 名前で読む所（`areka::runtime` の出現・`crates/`・`tools/`・`doc/`・steering）:
  - `crates/areka/tests/stay_on_top_real_run.rs` 51〜54 行（`RUST_LOG` に `areka::runtime::{window_state_cue,zorder_cue,frame::window_state,frame::zorder_drain,frame::zorder_descript,frame::switch}=debug`）
  - `crates/areka/src/runtime/shell_cursor_assets_tests.rs` 21 行（`TARGET`）・`assets_tests.rs` 459 行・`balloon_background_tests.rs` 245 行（`target=areka::runtime::balloon_background `）
  - `crates/areka/src/log_history_tests.rs` 124 行（`classify(Level::INFO, "areka::runtime")` が `None`）
  - `crates/areka/tests/emo2_real_run/common_tests.rs` 28 行（ログの行の見本）
  - `doc/ssp-mcp/log-convention.md` 135 行
  - `tools/` は 0 件（要件 5.12 は発動しない見込み）。real-run-kit の `launch.rs` の名指し（`areka_mcp::server`・`areka::app_exit`）にも runtime は無い。
- `log_history.rs` の `RULES` は runtime を名指さない（`areka` は完全一致だけ）。runtime の info は今も残らず、警告以上は出所に関わらず error の種別＝移しても同じ（要件 6.6 は前後で同じになる見込み・書き換えはテストの 1 行だけ）。
- `EnvFilter` は素の前方一致: `areka=debug` は `areka_runtime` も開く。`areka::runtime=debug` は開かない。
- **実機テスト 3 本の読み方**:
  - `smoke_boot_loop_exit.rs`: 本文 `runtime: 実 sink 結線が成立しました（wire 成立）`（`info!`）を読む。`RUST_LOG` は出所の名前を名指さない＝出所が変わっても読める。
  - `emo2_real_run`: `RUST_LOG = "info,kanade=trace,wintf::…=debug"`。runtime の info を本文で読む＝出所の名前に頼らない。
  - `stay_on_top_real_run.rs`: `[window_state] ゴーストの窓を今いる帯の最上へ出す`（`frame/window_state.rs` 148 行の `debug!`）ほかを数える。`RUST_LOG` の `areka::runtime::frame::window_state=debug` で開いている＝**書き換えないと 0 件で赤**。出所を名前で読む実機テストは、この 1 本だけ。
  - ほかの実機テスト（`window_link_real_run.rs` は `areka_placement=debug`・`emo2_mcp_real_run.rs` は `warn,areka::boot_config=info,…`）は runtime を名指さない。

### 2.11 パスと古い名前の書き換え先（要件 5）

| 場所 | 件数 | 備考 |
|---|---|---|
| `doc/COMPAT_ARCHITECTURE.md` | `src/runtime/` を含む行 40・出現 64（うち `wintf`・`dola` の自分の `src/runtime/` が 4＝本体の runtime は約 60。brief の 38 は前の数）。`runtime/x.rs` の形の出現は 73 で、`src/` 付きは 62＝約 10 が `src/` を付けない書き方 | パスの書き換えだけ（約束 ⑴）。`src/` の無い書き方をどこまで直すかは grep の範囲（要件 5.9）の読みしだい |
| `doc/ukadoc-coverage/ledger/*.toml` | 行 48・出現 53（`sakura-script` 34 行・`shiori` 7・`property` 6・`assets` 1） | 要件の「約 48」と一致。`report/` は作り直し |
| `doc/ukadoc-coverage/briefing-assets.md`・`briefing-shiori.md`・`doc/ssp-mcp/areka-tools.md`・`doc/emo2-conformance-scope.md`・`doc/click-through.md` | 7・1・2・1・1 | `doc/click-through.md` は要件の一覧に無い |
| `.kiro/steering/structure.md`・`tech.md` | 3（278 行の `runtime/`・281 行の `placement_tests/` ほか）・1（90 行の `spine_wait.rs`） | `areka-runtime` の節を足す |
| `crates/` の説明（runtime の外） | `areka-emo-present/src/presenter/refresh.rs`・`areka-emo-text/src/state_decoration_reset_tests.rs`・`areka-placement/src/lib.rs`・`areka-placement/src/test_crash_report.rs`・`wintf/src/test_crash_report.rs`・`shiori-host32-host/src/shiori3_charset_tests.rs`・`areka/tests/emo2_real_run.rs`・`areka/examples/collision-probe.rs`（4）・`placement_tests/spawn_zorder_pair_deferred_tests.rs`（8） | `dola/tests/runtime/…` の 1 件は dola の自分の `runtime/`＝当たりにしない |
| `crates/log-capture-kit/tests/` | `file_length_guard_test.rs` 3（例外表の `frame_transition_branch_tests.rs`・`transition_judge_verdict_tests.rs` と較正の項目）・`workspace_scan_test.rs` 1（320 行の `spine_display_tests.rs`） | `temp_path_guard_test.rs`・`with_default_guard_test.rs`・`sample_path_guard_test.rs` には runtime のパスが 0（要件 5.1 の「ほか」は空） |
| `placement_tests/` を名指す所 | `wintf/src/runtime/tick_bridge.rs`・`wintf/src/ecs/window/zorder_pair_test_support.rs`・`areka-emo-present/src/presenter/transition_record.rs`・`areka-placement/src/{measure_template_tests,measure_test_support,spawn_zorder_pair_deferred_tests}.rs`・`doc/COMPAT_ARCHITECTURE.md`・`structure.md`・`file_length_guard_test.rs` | 移す判定器のパスの書き換え（要件 5.9） |
| `audit-emo2-boot-{frame,spine,switch}` の brief | 「受け持つファイル」の行 5・5・3 | 本体へ置く runtime のテスト（議題 4）の置き場も受け持ちに入れるかが要る（下の 2.12） |
| 古い名前 `emo2_boot`・`Emo2Wiring`（本体の外） | **14 ファイル・21 行**（要件と一致） | 下の注 |

- **古い名前の 21 行のうち説明だけでないもの**:
  - `areka-emo-text/src/actor_lookahead_tests.rs` 343 行の **テストの関数の名前** `emo2_boot_talk_reveals_without_moving_glyphs`。書き換えると名前の末尾の多重集合（要件 7.2）が変わる＝要件 7.3 の対応表に 1 件載る。
  - `log-capture-kit/src/event_tests.rs` の 5 行は、テストの中の出所の値（`tracing::info!(target: "areka::emo2_boot", …)` と期待の文字列）。動きは変わらないが「説明だけ」ではない。
- 本体の中の 2 行（`runtime/mod.rs` 69 行・`spine.rs` 33 行）は完了 spec の設計のファイル名 `tests/emo2_boot_spine_test.rs` の引用（要件の申し送り 4 のとおり）。

### 2.12 そのほかの気づき

- **依存**（要件 1.8・1.9）: 本番の crate は要件の一覧と一致（`areka_placement` 85・`bevy_ecs` 43・`tracing` 40・`areka_emo_present` 36・`dola` 35・`wintf` 30・`areka_sakura` 24 ほか・`areka_sylphya` 3・`thiserror` 4・`windows` 5）。テストだけで使うのは `log_capture_kit`・`sample_ghost_kit`・`temp_path_kit`・`shiori_host32_host`・`image`（`assets_markers_tests.rs`・本体の dev-dependency と同じ版で `Cargo.lock` に在る）。**`areka-update` は今は 0**（brief の「1 ファイル」は前の形）。examples のために `tracing-subscriber`・`async-io` が dev-dependency に要る。
- **`#[allow(...)]` の理由が古くなるもの**（要件 7.1）: `frame.rs` の `#[allow(unused_imports)]` 21・`frame/wiring.rs` 2 の理由の文が「`areka` は lib target を持たない bin クレート」。lib になると `pub use` は未使用の警告から外れ、`pub(crate) use` は警告のまま＝属性を外すと何が出るかはコンパイルで確かめる（要件は属性も文も変えず申し送る）。`#[allow(dead_code)]` 3 は理由が bin に頼らない。`collision-probe.rs` 37 行の「`areka` は bin crate ゆえ `use areka::...` は不可能」は examples を移すときに事実でなくなる。
- **文書のテスト**: runtime の本番の doc に囲みのコード（```）は 0。番号つきの列挙の続きの行（`//!    …`）はあるが字下げのコードの段ではない見込み＝新しい文書のテストは 0 の見込み（`cargo test --doc -p areka-runtime` で確かめる）。
- **本番の説明の中の本体へのリンク**（要件 2.6）: `consumer_ledger.rs` 6・`mod.rs` 4（442・647 行の `wire_kanade_stop`・834 行は `wire_tests` の中のコードで `crate::session::runtime_boot::boot_runtime` を渡す）・`special_name.rs` 1。`hit_region.rs` 45〜56 行は「`crate::` フリー規律」と `crate::is_benign_boot_error` を地の文で書く（examples の `#[path]` が理由＝やめると理由が変わる）。
- **`mod.rs` の `wire_tests`**（755 行〜）は本体の `session::runtime_boot::boot_runtime` を `wire_runtime` に渡す＝本体を引くテスト（2.5 節の 40 とは別に、`lib.rs` の中のテスト 1 つ）。
- **機能の連鎖**: `areka-runtime` の `test-support` で開く支えが配置の `test_support` を使うなら、`test-support = ["areka-placement/test-support"]` の連鎖が要る。`resolver = "2"` では、試験のビルドで本体の dev-dependency が付けた機能が本体の bin にも効く（前例の 2.9 と同じ・リリースのビルドには入らない）。
- **受け持ちの重なり**: 前例は本体へ移したテストを `placement_tests/` にまとめて audit の受け持ちが重ならないようにした。runtime のテストを本体へ置くなら、置き場（例: `crates/areka/src/runtime_tests/`）が `audit-emo2-boot-*` 3 本のどれの持ち物か、`audit-areka-app`（「ほかの 7 本の持ち物でないもの全部」）と重ならないかを決める（要件 5.7）。
- **同じウェーブの兄弟**: `balloon-flip-side-rules` は `crates/areka/src/runtime/` の下に触らない約束（roadmap C21 ⑶）。`tools/load-flake.ps1` の `ValidateSet` は今 `areka-bin`・`wintf-lib`・`areka-placement-lib`（69・210・211 行）＝`areka-runtime-lib` を足すのは `audit-tools-scripts` との申し合わせ（要件 5.13）。

---

## 3. 要件と資産の対応表

タグ: **Missing**＝今は無く作る／**Unknown**＝設計で調べる／**Constraint**＝既存の決まりが縛る／**OK**＝今の資産で足りる。

| 要件 | 今の資産 | タグ | メモ |
|---|---|---|---|
| 1.1〜1.6 crate の新設・`git mv`・根を `lib.rs`・path 依存・根の `Cargo.toml` 不変 | `areka-placement` の `Cargo.toml` の型・`members = ["crates/*"]` | OK | 前例どおり |
| 1.7〜1.9 依存 | 2.12 節 | OK | dev に `image`・`tracing-subscriber`・`async-io`・`shiori-host32-host` が要る（どれも `Cargo.lock` に在る） |
| 1.10 1,000 行 | 例外表 4 件 | **Constraint** | 999 行が 3 つ（`switch_finish_tests`・`transition_judge`・`transition_judge_tests`）。本体へ移す向きは道が伸びる |
| 2.1・2.2 外から使われる分だけ `pub` | 2.2 節 | **Unknown** | 確定はコンパイラ。`run_ghost_quit_phase` は今は runtime に無い |
| 2.3・2.4 道の書き換え | 233 か所／218 か所＋素の `runtime::` | OK | 機械の置換 |
| 2.5 連れて開く型の記録 | ― | **Missing** | `private_interfaces` で数える |
| 2.6 説明の中のリンク | 3 ファイル 11 か所＋地の文 | OK | |
| 2.7 本番が本体を引かない grep | 本番の `crate::` は 0（説明を除く） | OK | |
| 3.1〜3.4 `test-support`・写し・番人 | 2.3 節 | **Constraint** | ⑶ の足場と `frame` の支えの一部が番人とぶつかる |
| 3.5 本体を引く runtime のテスト | 40 ファイル | **Constraint** | 25 が `use super::*`・25 が中の支え・15 が倒れ先に頼る |
| 3.6 口の倒れ先 | `session_hooks.rs` | **Unknown** | 本体に置いても `NO_BODY` になる（2.4 節） |
| 3.7 e2e 2 本 | `#[path]` 2 行 | OK | `session/` へ |
| 3.8 落ちの報告 | runtime の写し 1 つ | **Missing** | 本体の側が失う。`GpuPermit` と一体（2.9 節） |
| 3.9 逆向きの支え | 配置の `test_support` 2 ファイル・本体の支え 3 | OK | 数は brief より少ない |
| 3.10 テストの数え方 | `tools/test-all.ps1` は crate 名を持たない | OK | |
| 4.1 `target_map` | 純粋な本番 | OK | |
| 4.2・4.3 判定器 | 15 ファイル | OK | 本体を引くのは `target_map` だけ |
| 4.4・4.5 `placement_tests/` の残り | 2.6 節 | **Unknown** | `measure_template_tests` は本体の examples を読む＝本体に残る。対応表 2 本はどちらにも置ける |
| 4.6・4.7 examples | `#[path]` 2 本 | **Missing** | dev の依存を足す |
| 4.8 配置のテストだけの公開 | 2.7 節 | **Unknown** | 使い手は移しても外のまま |
| 5.1 log-capture-kit の表 | 3＋1 件 | OK | ほかの表は 0 件 |
| 5.2〜5.4 文字で読むテスト・読み口 | 2.8 節 | **Unknown** | `areka-runtime` 側の読み口の要否 |
| 5.5〜5.7 文書・steering・brief | 2.11 節 | OK | `doc/click-through.md` が一覧の外 |
| 5.8・5.10 古い名前 | 14 ファイル 21 行 | **Constraint** | テストの関数の名前 1 つ（7.2 と兼ね合う） |
| 5.9 古いパスの grep 0 件 | ― | **Unknown** | `src/` の無い `runtime/x.rs` を数えるか |
| 5.11〜5.13 `tools/` | 0 件 | OK | |
| 6.1〜6.8 出所 | 2.10 節 | OK | 直書き 0・`RULES` に無い |
| 7.1 振る舞い・`#[allow]` | 2.12 節 | OK | 申し送る一覧は `frame.rs`・`frame/wiring.rs`・`collision-probe.rs`・`hit_region.rs` の説明 |
| 7.2・7.3 名前の多重集合 | 前例の比べ方 | **Constraint** | 落ちの報告の写しを足すとテストが増える（前例は 5 本）・テストの関数の改名 1 |
| 7.4・7.5・7.9 警告・全体テスト・NOTICES | 道具は揃う | OK | `areka-runtime 0.1.0` の行が増える |
| 7.6・7.7 実機 | 3 本 | OK | `stay_on_top_real_run` だけが出所の名前に頼る |

---

## 4. 実装の案

### 案 A: 別名で道を保ったまま移す（今の部品を延ばす）

- 新しい crate へ移し、本体の `main.rs` に `use areka_runtime as runtime;` を置く。本体の `crate::runtime::…` は通らない（`crate::` は自分の crate）が、`use` 済みの `runtime::…` は通る。
- ✅ 移動の差分と書き換えの差分が混ざらない。
- ❌ 要件 2.4 を満たさない。本体を引く 40 のテストの置き場の問題は何も解けない（別名ではテストの私有の項目への道も倒れ先も変わらない）。
- 使い道: 最終形にはできない。途中の 1 コミット（公開面をコンパイラに数えさせる段）としてだけ使える。

### 案 B: runtime のテストは全部 `areka-runtime` に置き、本体を引く部分を口と偽物で差し替える

- 40 のテストを `areka-runtime` に残し、`GhostSlot`・入力の受け手・終了の手続きなどの本体の部品の代わりに、テストの中で `SessionHooks` の偽物や同じ働きの小さな書き手を据える。
- ✅ 「runtime のテストは `areka-runtime` にある」（brief の Desired Outcome）に字面どおり合う。私有の項目を開かずに済む。本体のテストの実行ファイルに runtime の支えを開ける量が減る。
- ❌ テストが確かめる相手（本物の入力の受け手・本物の口の中身）が替わる＝要件 7.1「振る舞いの確かめを変えない」と要件 3.5 の「前と同じ合否」の意味が変わる。共通の約束「境をまたぐテストは上の crate に置く」から外れる。

### 案 C: 本体を引くテストは本体へ、支えは依存の向きで割って開く（組み合わせ・前例に近い）

1. 骨組み・`git mv`・中の道（`crate::runtime::` → `crate::`）＋一時の別名で、公開面をコンパイラ（E0603・E0616・E0624・E0364・`private_interfaces`）に数えさせて開く。
2. 本体の道を `areka_runtime::…` へ書き換え、別名を外す。
3. テストの支えを依存の向きで割る: 番人の crate を引かない部分（待ち・`GpuPermit`・台本の SHIORI・`frame` の支えの大半・`balloon_runtime_test_support`）を `test-support` で開き、番人の crate や本体を引く部分（`SpineHarness`・`capture` の包み・`chain_finalize_persist_test_support`）は使う側の crate に置く。
4. 本体を引く 40 のテストを本体のテスト専用のフォルダへ移す（ファイル単位、または本体を引く関数だけを割る）。私有の項目は前例 D9 の形（`test_support` という名の入口を `test-support` の下でだけ開く）で開ける。倒れ先に頼る 15 は本体の口を据える形にする（議題 3）。
5. 判定器・`placement_tests/` の残り・examples・番人の表・読み口・出所・文書・落ちの報告の写し。
6. 検証（多重集合・警告・全体テスト・実機 3 本・`cargo about generate`）。

- ✅ 共通の約束（境をまたぐテストは上・写しを作らない・番人を変えない）に沿う。各段で `cargo check --workspace --all-targets` を緑にでき、公開面の数え違いが起きない。
- ❌ 開ける私有の項目の数が読めない（25 ファイルが `use super::*`）。本体へ移すテストの数しだいでタスクが増える（brief の 10〜12 タスクを超える見込み）。

（案 B と C は 1 本ずつ混ぜられる。要件 3.5 は「1 本ずつ決める」と書いている。）

---

## 5. 規模と危うさ

- **規模: L（1〜2 週）**。ファイルの数は多いが大半は機械の置換。手のかかるのは、⑴ 40 のテストの置き場と私有の項目の開け方 ⑵ `spine` と `frame` の支えの割り方 ⑶ 倒れ先 ⑷ 落ちの報告の 2 つ目の据え方 ⑸ 999 行のファイル。brief の 10〜12 タスクは、議題 2〜4 の答えによって 2〜4 タスク増える見込み。
- **危うさ: 中〜高**。技術は既知（Cargo の機能・crate の分け方・前例あり）で、振る舞いは変えない。ただし前例（本体を引くテスト 5 本）に比べ、本体を引くテストが 40・倒れ先に頼るテストが 15 と多く、置き場の決めがテストの中身に触れやすい。テストが 2 つの実行ファイルに分かれ、落ちの報告と GPU の許可の数え（プロセスに 1 つ）が実行ファイルごとになる（`cargo test` は実行ファイルを順に回すので、同時に持つ数は変わらない見込み）。

---

## 6. 設計の段への申し送り

### 決めること（要件の討議の議題の候補）

1. **実機テストの本数**（要件 7.6・未決の議題 1）: 出所の名前に頼るのは `stay_on_top_real_run.rs` の 1 本だけ（2.10 節）。案: ⒜ 3 本とも回す（要件の仮定・brief の 2 本は「名前で読む」ではないが起動の確かめとして残す）⒝ `stay_on_top_real_run.rs` の 1 本だけ（共通の約束「ログを名前で読む実機テスト」の字面どおり）⒞ `stay_on_top_real_run.rs`＋`smoke_boot_loop_exit.rs`（起動の 1 本を足す）。
2. **`spine` と runtime の中の支えの開け方**（要件 3.1〜3.4・未決の議題 2）: 案: ⒜ 3 層に割る＝⑴ 待ち・`GpuPermit` ⑵ 台本の SHIORI を `areka-runtime` の `test-support` で開き、⑶ `SpineHarness` とそれを使う spine のテスト（約 13）は本体へ（境をまたぐ支え）。`frame` の支えも同じく番人の crate を引く 3 関数だけを分ける ⒝ `spine` を丸ごと本体へ（要件の段の推奨 ①）＝⑴ を使う `areka-runtime` のテスト 13 も本体へ行くか、⑴ の写しが要る（要件 3.3 に反する）⒞ 番人の決まりを理由つきで狭く広げる（`areka-runtime` だけが `test-support` の下で optional に持つ）＝本体の 2 項目（`runtime_boot_options`・`KanadeNoticeRx`）の問題は残る ⒟ ⑶ の本体の 2 項目を引数で受ける形にして `areka-runtime` の `cfg(test)` に残し、本体の 2 ファイルは本体の側で同じ足場を組む＝写しになりやすい。
3. **口の倒れ先**（要件 3.6・未決の議題 3）: 倒れ先に頼るテストは 15・全部本体を引く。案: ⒜ 本体へ移したテストの World の組み立てで `session::hooks::install` を呼ぶ（テストの手順に 1 行足す・確かめの中身は同じ）・`areka-runtime` の倒れ先は `cfg(test)` でも `NO_BODY`（要件の仮定）⒝ `test-support` の下で「倒れ先を差し替える口」を `areka-runtime` に足し、本体のテストの実行ファイルが起動時に据える（仕組みが 1 つ増える）⒞ 倒れ先の `cfg(test)` の枝を消して、`areka-runtime` の中のテストでも記録（`error!`）を出す＝`areka-runtime` に残るテストで口を読むものが無いことを確かめた上で。
4. **本体を引く runtime のテスト 40 の置き場**（要件 3.5・未決の議題 4）: 案: ⒜ ファイルごと本体へ（要件の仮定・私有の項目を `test_support` の入口で開く量が最大）⒝ ファイルの中の本体を引く関数だけを本体へ割り、残りは `areka-runtime`（開ける量は減るが、割ったファイルの対が増え、名前の多重集合は変わらない）⒞ 一部（例: `consumer_ledger_agreement_tests`・`strict_judges_tests`・判定器の 2 本）は本体を引く部分が小さいので個別に扱う。あわせて本体の中の置き場の名前（例: `crates/areka/src/runtime_tests/`）と、`audit-emo2-boot-*` の受け持ち（要件 5.7）の書き方。
5. **`audit-placement-follow` の範囲の外の古い参照**（未決の議題 5）: 事実は brief のとおり（`frame_transition_atomicity_tests.rs` 57・150 行の行番号の指し・`drain_resnap.rs`・`frame_chain_finalize_tests.rs`・`frame_work_area_resnap_hold_tests.rs` 429 行の `should_panic`・`frame/move_shift.rs` の注）。要件の仮定（行の指しだけ直し、ほか 3 件は申し送る）に反する事実は見つからなかった。判定器のパスが変わるので、行の指しは移した後に必ず古くなる。
6. **テストだけが使う配置の公開**（要件 4.8・未決の議題 6）: 2.7 節のとおり、使い手は移しても `areka-placement` の外。案: ⒜ `areka-placement` の `test-support` の下へ（`areka-runtime`・本体の dev-dependency が付ける・`MonitorDpiEntry` と本番が使う `transition_diag` の語は公開のまま）⒝ 全部公開のまま残し、理由を記録する。
7. **`placement_tests/` の残りの置き場**（要件 4.5・未決の議題 7）: 事実から決まるもの＝`reset_balloon_pos_tests`・`measure_bake_log_tests` は `areka-runtime`、`spawn_assembly_tests`・`measure_template_tests` は本体。選べるもの＝`measure_bake_count_tests`（下の crate を読む 1 か所）・`spawn_zorder_pair_deferred_tests`（`areka-runtime` に移せば木の走査の範囲は同じ）・分岐の対応表 2 本＋支え（読み口 26 か所）。
8. **`areka-runtime` の読み口**（要件 5.2・5.3）: 下の crate を読む所を `areka-runtime` に置くなら、⒜ 読み口を `areka-runtime` の `test-support` に移して本体もそれを使う（1 つだけ・マクロは呼び出し側で展開されるので両方で働く）⒝ 共有の kit へ下ろす ⒞ `areka-runtime` には作らず相対の道で読む（前例の「1 か所なら作らない」）。
9. **落ちの報告の 2 つ目の据え方**（要件 3.8）: ⒜ 本体に写しを足す（4 つ目・GPU の許可の持ち手の表をどちらが持つかを決める）⒝ `GpuPermit` と一緒に `test-support` で開き、本体のテストの実行ファイルにも初期化子が入る形（下の調べること 1 の結果しだい）。
10. **古い名前のうちテストの関数の名前**（要件 5.10 と 7.2・7.3）: `actor_lookahead_tests.rs` の `emo2_boot_talk_reveals_without_moving_glyphs` を改めて対応表に載せるか、5.10 の grep の除外に入れるか。`log-capture-kit/src/event_tests.rs` の出所の値 5 行（説明でなくテストの値）を書き換えてよいか。
11. **`src/` の無い `runtime/x.rs` の書き方**（要件 5.9）: `doc/COMPAT_ARCHITECTURE.md` では `runtime/x.rs` の形 73 のうち約 10 が `src/` を付けない書き方。本体の runtime を指すものを全部直すか、`src/runtime/` を含むもの（40 行）だけ直すか。

### 調べること（設計の段）

1. 上流の rlib の `#[used]`＋`#[unsafe(link_section = ".CRT$XCU")]` の静的が、機能つきで組まれたときに本体のテストの exe へ残るか（MSVC のリンカ・rustc の扱い）。残るなら、`areka-runtime` 自身のテストの実行ファイルと本体のテストの実行ファイルで 2 重に据わらないか。
2. 公開面の確定（2.2 節の 124 の道・19 のモジュール・メソッド約 30）をコンパイラに数えさせる。本体へ移すテストが使う私有の項目の数も同じく数える（議題 4 の答えの材料）。
3. 本体の外のテストで、口を据えずに runtime の口を読む相を回すもの（`shell_balloon_switch_session_lap_tests.rs` ほか）が、支え経由で口を据えているか。
4. `frame/switch_finish_tests.rs`・`transition_judge.rs`・`transition_judge_tests.rs`（どれも 999 行）が書き換えと rustfmt の後に 1,000 行を超えないか。
5. `cargo test --doc -p areka-runtime` が 0 件か（2.12 節）。lib にしたときの死んだコードの警告（`pub(crate)` で crate の中から使われないもの）が 0 か。
6. `areka-runtime` を足した後の `structure.md` の「`log-capture-kit` を使う crate の数」ほか、数を書いた所。
7. 同じウェーブの兄弟（`balloon-flip-side-rules`・`wintf-surface-auto-promotion`・`pilot-windows-rs-next`・`audit-tools-scripts`）の設計が触るファイルと、この仕様が書き換える文書・表（`doc/COMPAT_ARCHITECTURE.md`・網羅台帳・`structure.md`）が重ならないか。

---

（次の手: 要件の討議で 6 章の「決めること」1〜11 の答えを決め、要件 3.1〜3.6・4.5・4.8・5.9・5.10・7.6 を必要なら書き換えてから `/kiro-design runtime-crate-split` へ。）
