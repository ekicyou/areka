# Requirements Document

## Project Description (Input)

- **困っている人**: areka の開発者（本体 `crates/areka` を直し、本体を crate に分け、見直し〔ponytail の audit〕を受け持ちごとに回す人）。
- **今の状態**: `crates/areka/src/emo2_boot/` は、エンジン（kanade・sakura・seriko・emo・dola）と窓（wintf・placement）を毎フレームつなぐ本体の背骨で、本番 約 15,300 行・テスト 約 54,900 行を持つ。困りごとは 2 つある。⑴ 名前（`emo2_boot`・`Emo2Wiring`・`Emo2BootInputs`・`wire_emo2_boot` ほか）が特定のゴースト emo2 を指しているように読めるが、実体は汎用の実行基盤（ゴースト・シェル・バルーンの切替、reload、バルーンの可視性、各 cue の受け口、talk の時計）で、名前は最初の統合 spec（`areka-P0-emo2-boot`）の名残。⑵ 本体のほかの部分（`ghost_session`・`app_exit`・`boot_config`・`boot_resolve`・`input_events`・install・update・readme ほか）と参照し合っていて、このままでは crate に出せない。
- **変えたいこと**: crate は作らず、本体の中で次の 3 つを済ませる。名前を改める（モジュールは `runtime`）。外へ出られない部分（切替・読み直し・実行系の起動、install・update・readme の cue の受け口）を本体の別の場所へ移す。残る `runtime` の本番が本体の外の部分を参照しない形にする。振る舞いは何も変えない。次の `runtime-crate-split` が移すだけで済む形にする。

（出どころ: `.kiro/specs/runtime-module-untangle/brief.md`〔2026-10-28 起票・2026-11-13 棚卸㊱の再測定 main `069f1a48`〕。分け方・順番・共通の約束の正本は `.kiro/steering/roadmap.md`「テーマ別の決めごと」の「本体の crate 分け」。同じウェーブ C19 の約束は `roadmap.md`「ウェーブ編成」の C19 の行。）

## Introduction

本仕様は、areka 本体の中のモジュール `crates/areka/src/emo2_boot/` を `crates/areka/src/runtime/` へ改名し、外へ出られない部分を本体の新しいモジュール `crates/areka/src/session/` と、install・update・readme のそれぞれのモジュールへ移し、残る `runtime` の本番が本体のほかの部分を参照しない形にする。crate は作らない（`runtime-crate-split` の受け持ち）。

受益者は areka の開発者であり、この仕様での「目に見える成果」は次のとおり開発者の側から定める。

- `runtime` の本番のコードが、本体のほかの部分を 1 か所も参照していないことを grep で示せる（参照してよいのは `areka-placement` などの下の crate だけ）。
- 名前に emo2 が残るのは、既定のゴースト名とその使い手、実在のゴースト emo2 を指す検体だけで、その一覧を grep で示せる。
- 動くデスクトップマスコットとしての振る舞い（起動・切替・読み直し・終わり方・終わる段取りの順・フレームの中の順）が前と同じで、変わるのはログの文言と出所の名前、イベントの値の名前だけである。
- テストの数・警告・全体テスト・実機テストが、前と同じ合否を示す。

数（ファイル数・参照の数・行の番号）は brief の測り（main `069f1a48`）と、本仕様の要件の段での測り（main `4863cb66`）による参考値で、設計の段で測り直す。

## Boundary Context

- **In scope**:
  - モジュール・型・関数の改名（要件 1 の表）と、ログの文言・イベントの値・テストの結線を指す名前の改名、それを照合する所の直し。
  - 本体に頼る部分の移動（要件 2 の原則による。今の測りでは `ghost_switch`・`reload`・`runtime_boot`・`switch_copy`・`same_folder_pin` と各テスト、`shell_balloon_switch` の要求の入口、`shell_carry` の `capture_for_reload`・`hand_over` を `src/session/` へ、`install_cue`・`update_cue`・`readme_cue` を `install/`・`update/`・`readme/` へ。本体に頼らない `reload_args`・`shell_balloon_resolve`・`shell_carry` の残りの部分・`shell_balloon_switch` の型の部分は `runtime` に残す）。
  - `runtime` に残る本番が本体の外を参照する所の切り離し。brief が挙げた 6 か所（`frame.rs` の `app_exit::quit_app`・`frame/switch.rs` の `BootContext`／`boot_resolve` の記憶を書く関数／`GhostSession`・`frame/status_report.rs` と `balloon_visibility_phase.rs` の `GhostSlot`・`balloon_visibility_phase.rs` の `BalloonWiring` のホバーの集合・`ShellBoxHover`／`settle_box_hover`・`wire_user_break` の呼び出し）に加えて、要件の段の測り（main `4863cb66`）で見つかった次の所も同じ規則で切る。
    - brief の測り（main `d6054e4d`）より後に足された本番の参照 4 ファイル: `frame/attach.rs`（`GhostSession`・`GhostSlot`）・`frame/scale_notice.rs`（`GhostSlot`・`ghost_session_scope_scale::{percent_text, scaling_text}`）・`frame/window_state.rs`（`boot_config::BootContext`）・`frame/wiring.rs`（`ghost_session_scope_scale::SharedScopeScaleTable`）。brief の再測定の「本体の外を引く本番の参照は増えていない」は、この 4 ファイルを数えていない。
    - 外へ移すファイルを `runtime` に残るファイルが引く所（`frame.rs`・`frame/switch.rs` が `ghost_switch`・`reload`・`shell_balloon_resolve`・`shell_balloon_switch` を、`frame/wiring.rs` が `shell_carry::ShellCarry` を、`mod.rs` と `spine.rs` が `runtime_boot` を、`change_cue.rs`・`switch_cue.rs` が `ghost_switch`・`shell_balloon_switch` の型を引く所ほか。要件 2.1 の原則で `runtime` に残す部分を引く所は、切らずに済む）。移した後に `runtime` から `session` を引く所になるので、brief の 6 番目（「runtime から session を参照する所が出たら、それも切る」）として切る。
  - 古い名前とパスの書き換え（`crates/`・`tools/`・`doc/`・`.kiro/steering/{structure,tech}.md`・網羅台帳の根拠の場所・1,000 行の番人の例外表ほか）、`roadmap.md` の新旧のパスの対応表、`audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` の brief の「受け持つファイル」の書き換え。
  - emo2 の残りの一覧と、次の spec への申し送り（crate の外のテストが使う支えの一覧ほか）。
  - 検証（テストの名前の一覧の比べ・警告 0 件・全体テスト・実機テスト 2 つ・終わる道の記録の順の比べ）。
- **Out of scope**:
  - crate の新設（`runtime-crate-split`）。input と session の crate 化と session の集め直し（`input-crate-split`・`session-module-gather`・`session-crate-split`）。
  - 振る舞いの変更。終わり方・切替の段取りの順・フレームの中の順は変えない。直したくなった不具合や要らない項目は、その場で直さず範囲を受け持つ audit（`audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` ほか）へ申し送る。
  - `GhostSlot` と止まる流れを複数のゴースト向けに作り直すこと（`multi-ghost-runtime`）。
  - 既定のゴースト名 `"emo2"`（`boot_resolve.rs` の `DEFAULT_GHOST_FOLDER`）を変えること。実在のゴースト emo2 を指す名前（`emo2_root`・`acquire_emo2`・`Emo2Absent`・`EMO2_ADJUST`・`AREKA_EMO2_REAL_RUN`・`tests/emo2_real_run*`・`emo2_first_load_repeat` ほか）を変えること。emo2 を含まない名前（`BootWiringError` ほか）を変えること。
  - `runtime` のテストのコードが本体の外を参照する所を 0 にすること（数えるだけ・決めるのは `runtime-crate-split`）。
  - 本体の外の crate の説明の文の中の `emo2_boot` の名指し（`crates/areka-placement` の 26 ファイル〔`src/test_crash_report.rs` 1 行目を含む〕・emo-text・emo-present・kanade・wintf の説明・parsers・seriko・ghost・shiori-host32-host・ukadoc-survey・`log-capture-kit` の `src/event.rs` の説明と `src/event_tests.rs` の出所の見本の値）の書き換えと、本体の外の crate のテストの名前の中の `emo2_boot` の語（`crates/areka-emo-text/src/actor_lookahead_tests.rs` の `emo2_boot_talk_reveals_without_moving_glyphs`）の改め（パスがもう一度変わる `runtime-crate-split` で直す）。ただし要件 5.3 に挙げる、パスが切れると働きが変わる所は直す。
  - `emo2_boot` に触ると書いているほかの約 70 本の brief の書き換え（`roadmap.md` の新旧のパスの対応表で読み替える）。
  - 歴史の文書（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/`・日付を名に持つ写し `doc/roadmap-overview-2026-10-16.html`）の中の古い名前とパス。
- **Adjacent expectations**:
  - 上流: `placement-crate-split` は 11-06 に着地した（PR#433）。`emo2_boot/` に `crate::placement::` は残っていない。下の crate `areka_placement` を引くのは引いてよい側。
  - 同じウェーブ C19 の約束（roadmap が持つ並べ方の約束で、この仕様の要件ではない）: 見直しの 5 本は `crates/areka/` に触らず自分の crate の公開面は足すだけ。`audit-placement-core`・`audit-placement-follow` は C19 に入れない（本体の `placement_tests/` の 9 ファイルを分け合うため）。この仕様が本体の外で書き換えるのは、wave の行と要件 5 に挙げたファイルだけ。
  - 下流: `runtime-crate-split` は、この仕様が残す形（`runtime` の本番が本体の外を参照しない・テストの支えの一覧）を前提に、`src/runtime/` を crate `areka-runtime` へ移すだけにする。`audit-emo2-boot-*` 3 本は `runtime-crate-split` の後に新しいパスで着手する。

## Requirements

### Requirement 1: 名前の改め

**Objective:** 開発者として、汎用の実行基盤の名前から特定のゴーストを指す語を外したい。それにより、名前を読んだ人が「emo2 専用の部分」と取り違えず、次の spec で crate 名 `areka-runtime` と揃うため。

#### Acceptance Criteria

1. The 本仕様の作業 shall 次の表のとおり名前を改める。

   | 今 | 改めた後 |
   |---|---|
   | モジュール `emo2_boot`（`crates/areka/src/emo2_boot/`） | `runtime`（`crates/areka/src/runtime/`） |
   | `Emo2Wiring` | `RuntimeWiring` |
   | `Emo2BootInputs`・`Emo2BootOutcome` | `RuntimeInputs`・`RuntimeOutcome` |
   | `wire_emo2_boot` | `wire_runtime` |
   | `register_emo2_frame_system`・`emo2_frame_system` | `register_frame_system`・`frame_system` |
   | ログの文言の `emo2-boot:`・`emo2 attach:`・`emo2 text:`（と `ghost_session.rs` の `emo2-boot wire` の 2 行） | `emo2` の語を外した形（例 `runtime:`・`runtime attach:`・`runtime text:`・`runtime wire`） |
   | イベントの値 `choice_leave_no_emo2`・`choice_moved_no_emo2`・`choice_pressed_no_emo2`・`balloon_wheel_no_emo2` | `choice_leave_no_runtime`・`choice_moved_no_runtime`・`choice_pressed_no_runtime`・`balloon_wheel_no_runtime` |
   | テストの中の結線を指す名前（`headless_emo2_wiring` ほか） | `emo2` の語を `runtime` の語へ |

2. The 本仕様の作業 shall 既定のゴースト名 `DEFAULT_GHOST_FOLDER = "emo2"` とその使い手（既定のゴーストかどうかの判定）、実在のゴースト emo2 を指す名前（検体・実機テストの名前と環境変数）、emo2 を含まない名前（`BootWiringError` ほか）を変えない。
3. When 改名する名前が実在のゴースト emo2 を指すのか結線を指すのか、名前だけで決まらないとき, the 本仕様の作業 shall 使われ方（検体のフォルダや実在のゴーストの資産を読むか、結線の型と関数を指すか）で判定し、判定した名前と理由を設計か研究の記録に残す。
4. The 本仕様の作業 shall 名前を改めたテストを、新旧の名前の対応表で説明する。
5. When ログの文言やイベントの値を改めたとき, the 本仕様の作業 shall それを照合している所（`tests/smoke_boot_loop_exit.rs` の結線の成立の印・`tests/emo2_real_run/common.rs`・`balloon_pointer_handler_tests`・`balloon_leave_tests`・`assets_tests`・`balloon_background_tests`・`reload_skin_tests`・`reload_tests`・`tests/emo2_real_run/common_tests.rs`・`doc/ssp-mcp/log-convention.md` ほか）を、改めた後の文言と値へ一緒に直す。
6. The 本仕様の作業 shall ログの文言とイベントの値のうち、改める語（`emo2` → `runtime`）のほかの部分（文の本体・フィールドの名前と値・ログの段）を変えない。

### Requirement 2: 外へ移すファイル

**Objective:** 開発者として、`runtime` から外へ出られない部分（本体の起動と切替の層に頼る部分、install・update・readme の持ち主に頼る部分）を本体の別の場所へ移したい。それにより、`runtime` に残るのが本体に頼らない部分だけになり、次の spec が移すだけで済むため。

#### Acceptance Criteria

1. The 本仕様の作業 shall `runtime` から本体の新しいモジュール `crates/areka/src/session/` へ移すものを、本体に頼る部分（本体の `runtime` の外を引く型・関数とそのテスト）だけにし、本体に頼らない部分は `runtime` に残す。今の測り（main `4863cb66`・`research.md`）に当てはめると、`ghost_switch`・`reload`・`runtime_boot`・`switch_copy`・`same_folder_pin` の本番のファイルと各テスト、`shell_balloon_switch` の要求の入口、`shell_carry` の `capture_for_reload`・`hand_over` を移し、`reload_args`・`shell_balloon_resolve`・`shell_carry` の残りの部分・`shell_balloon_switch` の型の部分を `runtime` に残す。どのファイルをどこで分けるかは設計で決め、設計の段の測り直しで頼る・頼らないが変わったものは、この原則で振り分ける。
2. The 本仕様の作業 shall `install_cue`・`update_cue`・`readme_cue` の本番のファイルと各テストを、それぞれの持ち主のモジュール `crates/areka/src/install/`・`update/`・`readme/` へ移す。
3. The 本仕様の作業 shall 移すファイルを、git が移動として追える形（履歴を残す移動）で移す。1 つのファイルを `runtime` と `session` に分けるときは、行の多い側を元のファイルの移動（または残し）とし、分けた先を要件 5.7 の新旧のパスの対応表に並べて書く。
4. The 本仕様の作業 shall cue の受け口の型（`*_cue.rs` の同型の形）と、`consumer_ledger.rs` の表の行（どの名前をどの受け口が受けるか）を変えない。`consumer_ledger.rs` の説明の文の中の、移した受け口へのリンク（`super::install_cue::InstallCueSink` ほか 6 か所）は、移した先のパスへ書き換える。
5. Where 移すファイルの中に、`runtime` に残るファイルが引く型や関数がある場合, the 本仕様の作業 shall その型や関数を `runtime` 側に置くか、引く向きを逆にするかして、要件 3.1 を満たす（どちらにするかは設計で決める）。
6. The 本仕様の作業 shall `crash_report_test_support.rs`（テストの実行ファイルの落ちの報告）を `src/runtime/` へ一緒に移すだけにし、中身を変えない。

### Requirement 3: runtime の本番が本体の外を参照しないこと

**Objective:** 開発者として、`runtime` の本番のコードが本体のほかの部分を 1 か所も参照しない形にしたい。それにより、次の spec で `src/runtime/` を crate へ移したとき、参照し合いが残らず移すだけで済むため。

#### Acceptance Criteria

1. When 本仕様の作業を終えたとき, the `runtime` モジュールの本番のコード shall 本体の `runtime` の外の部分を参照する所を 0 件にする。数えるのは、`crate::` で始まるパス（`crate::runtime::` を除く）と、`runtime` の外へ抜ける `super::`（`super::super` ほか）の両方とし、`session`・`install`・`update`・`readme` への参照も外の参照に数える。
2. The `runtime` モジュールの本番のコード shall 参照してよい相手を、ワークスペースの下の crate（`areka-placement`・wintf・エンジンの crate ほか、本体の `areka` に依存しない crate）と外の crate だけにする。
3. The 本仕様の作業 shall 要件 3.1 の 0 件を、本番のコード（`#[cfg(test)]` の中とテストのファイルを除く）の grep の結果で示し、数え方（grep の式・除いたもの）を記録に残す。
4. The 本仕様の作業 shall 説明の文（`//`・`//!`・`///`）の中の本体の外のパスの名指しを、要件 3.1 のコードの参照と分けて数え、残すものは一覧にする。
5. The 本仕様の作業 shall `runtime` のテストのコードが本体の外を参照する所を、ファイルごとの数で数えて記録に残す（0 にすることは求めない）。
6. When 参照を切った後に終わる道（メニューの終了・窓を閉じる・煙の自動終了 `AREKA_APP_SMOKE_EXIT_MS`）を通ったとき, the areka shall 前と同じ順で終わる段取りを踏む。
7. When 参照を切った後にゴースト・シェル・バルーンの切替や読み直しが起きたとき, the areka shall 前と同じ順で切替と読み直しの段取りを踏み、毎フレームの中の順も前と同じに保つ。
8. If 参照を切るために、向きを逆にした知らせや口を使う場合, then the areka shall その知らせや口で終了・切替・差し込みを 1 フレームも遅らせず、知らせを出したのと同じフレームの中で前と同じ処理を終える。
9. The 本仕様の作業 shall 本体の中のプロセスで共有する状態（`balloon_visibility_wait.rs` の `OnceLock`・`hover_inject.rs` の `OnceLock`・`frame/switch.rs` の `AtomicU64`）の数と働きを変えず、どこに置いたかを記録に残す。

### Requirement 4: ログの出所

**Objective:** 開発者として、ログの出所の名前が改めたモジュールの名前に揃い、名前で読む所が全部それに追従してほしい。それにより、記録の分け方・`RUST_LOG` での開き方・実機テストのログの拾い方が、改めた後も前と同じく働き、黙って外れないため。

#### Acceptance Criteria

1. The areka shall `runtime` のモジュールのパスから付くログの出所を `areka::runtime::…` で出し、`session`・`install`・`update`・`readme` へ移したファイルのログの出所を、移した先のモジュールのパスで出す。
2. The 本仕様の作業 shall 出所を名前で読む決まり（`log_history.rs` の `rule("areka::emo2_boot::ghost_switch", …)` ほか）を、移した先の出所の名前へ直し、その決まりのテスト（`log_history_tests`）と `doc/ssp-mcp/log-convention.md` を一緒に直す。
3. When ログの出所の名前が変わった後に areka が記録を書くとき, the areka shall 前と同じ行を、前と同じ記録の分け方（`log_history` の種類と扱い）で書く。
4. The 本仕様の作業 shall 出所を名前で読むほかの所（`RUST_LOG` の文字列〔`crates/areka/tests/stay_on_top_real_run.rs` の `areka::emo2_boot::…` 6 つほか〕・テストのフィルタ・`tools/load-flake.ps1` の例 `-Filter emo2_boot::frame`・実機の手順書）を、新しい名前へ直す。
5. The 本仕様の作業 shall 語彙として付けた直書きの出所（`"areka::persist::*"`・`"areka::log::*"`・`"areka::perf"` ほか、モジュールのパスに頼らないもの）を変えない。
6. When 書き換えを終えたとき, the 本仕様の作業 shall 古い出所の名前 `areka::emo2_boot` を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep し、残りが Boundary Context の Out of scope に挙げた所（`log-capture-kit` の `src/event_tests.rs` の出所の見本の値ほか）と歴史の文書だけであることを示す。

### Requirement 5: 古い名前とパスの書き換え

**Objective:** 開発者として、`emo2_boot` の古いパスと名前を名指す所を、新しいパスと名前へ直したい。それにより、表・テスト・文書・道具が実在しないパスや名前を指したまま黙って通ることがなくなるため。

#### Acceptance Criteria

1. The 本仕様の作業 shall 本体の中で `emo2_boot` と改める名前を引く所（`ghost_session.rs`・`main.rs`・`mcp/`・`input_events/`・`install/`・`update/`・`readme/`・`menu/`・`offline_dump/`・`session_end.rs`・`app_exit.rs`・`boot_config.rs`・`log_history.rs`・`placement_tests/` の 9 ファイル〔`measure_bake_count_tests`・`measure_bake_log_tests`・`measure_template_tests`・`reset_balloon_pos_tests`・`spawn_zorder_pair_deferred_tests`・`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`・`transition_judge`・`transition_judge_tests`〕ほか）を、改めた後のパスと名前へ書き換える。
2. The 本仕様の作業 shall ソースを文字列で読むテスト（`include_str!`・`CARGO_MANIFEST_DIR` を使うもの・`crate_source_test_support.rs` の読み口を通るもの・`frame_schedule_tests` の `pub fn wire_emo2_boot(` の照合・`tick_gate_config_producers_tests.rs` の `"emo2_boot/…"` のパス 8 本・`zorder_wiring_tests.rs` の `include_str!("runtime_boot.rs")` ほか）と、ソースのパスを `#[path]` で指す例（`crates/areka/examples/collision-probe.rs` の `target_map.rs`・`hit_region.rs`）を、改めた後のパスと名前を読むようにする。
3. The 本仕様の作業 shall 本体の外で、パスが切れると働きが変わる所を直す: 1,000 行の番人の例外表 `crates/log-capture-kit/tests/file_length_guard_test.rs`（63・89 行目）・`crates/log-capture-kit/tests/workspace_scan_test.rs`（320 行目）・`crates/wintf/src/test_crash_report.rs` 1 行目の写しの相手のパス・`tools/load-flake.ps1` 47 行目。
4. If ソースを文字列で読むテストや番人の表が、書き換えた後のパスのファイルを見つけられない場合, then the そのテストや番人 shall 黙って通らず赤になる。
5. The 本仕様の作業 shall 文書（`doc/COMPAT_ARCHITECTURE.md`・`doc/emo2-conformance-scope.md`・`doc/ssp-mcp/`・`doc/ukadoc-coverage/` の `briefing-*.md` と網羅台帳 `ledger/*.toml` の根拠の場所・`doc/ideas/`）と steering（`.kiro/steering/structure.md`・`.kiro/steering/tech.md`）にある古いパスと名前を、移した先のパスと改めた後の名前へ書き換える。
6. The 本仕様の作業 shall 網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所を、根拠のファイルが実際に移った先（`src/runtime/`・`src/session/`・`src/install/`・`src/update/`・`src/readme/`）へ書き換え、台帳から作る生成物は手で直さず作り直す。
7. The 本仕様の作業 shall `.kiro/steering/roadmap.md` に、`emo2_boot` の新旧のパスの対応表（古いパス → 移した先のパス）を置き、`emo2_boot` に触ると書いているほかの brief がその表で読み替えられるようにする。
8. The 本仕様の作業 shall `audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` の brief の「受け持つファイル」を、移した先のパス（`src/runtime/`・`src/session/` ほか）へ書き換える。
9. When 書き換えを終えたとき, the 本仕様の作業 shall 古いパス `emo2_boot`（`crates/areka/src/emo2_boot/`・`emo2_boot/`・`emo2_boot::`）と改める前の名前（要件 1.1 の表の左の列）を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep し、残りが Boundary Context の Out of scope に挙げた本体の外の crate の説明の文とテストの名前と歴史の文書だけであることを示す（新旧のパスの対応表を持つ `roadmap.md` も除く）。

### Requirement 6: emo2 の残りの一覧

**Objective:** 開発者として、改めた後に名前や文言に emo2 が残る所が、既定のゴースト名とその使い手と実在のゴースト emo2 を指す検体だけであることを、一覧で確かめたい。それにより、名残の取りこぼしと、残すべき名前の消し過ぎの両方を見つけられるため。

#### Acceptance Criteria

1. When 改名を終えたとき, the 本仕様の作業 shall `crates/`・`tools/`・`doc/` の中で `emo2`（大文字小文字を問わない）を含む識別子・ログの文言・イベントの値・パスを grep し、残る所を一覧にして記録に残す。
2. The 本仕様の作業 shall 一覧の各行を、「既定のゴースト名とその使い手」「実在のゴースト emo2 を指す検体」「本体の外の crate の説明とテストの名前だけ（`runtime-crate-split` で直す）」「歴史の文書」のどれかに分ける。
3. If 一覧の中に、4 つのどれにも当たらない行（結線や実行基盤を指して emo2 の語を使う所）が残る場合, then the 本仕様の作業 shall その名前を要件 1 の規則で改める。
4. The areka の本番のコード shall emo2 だけに向けた動きを、既定のゴーストかどうかの判定（`DEFAULT_GHOST_FOLDER` の使い手）だけに保つ。

### Requirement 7: 次の spec への申し送り

**Objective:** 開発者として、次の `runtime-crate-split` が crate の境目でテストの置き場とテストの支えを決めるための材料を、この仕様で残したい。それにより、次の spec が測り直しから始めずに済むため。

#### Acceptance Criteria

1. The 本仕様の作業 shall `runtime` の外のテスト（本体のほかのモジュール・`placement_tests/`・`session/` ほか）が使う `runtime` のテストの支え（`spine_test_support`・`frame_test_support`・`sample_test_support` ほか）を、支えごとに使っている所と回数の一覧にして、設計か研究の記録に残す。
2. The 本仕様の作業 shall `runtime` のテストが本体の外を参照する所（`frame_transition_atomicity_tests.rs`・`frame_transition_branch_tests.rs` が `crate::placement_tests::transition_judge` を引く所ほか）を、要件 3.5 の数とあわせて記録に残す。
3. The 本仕様の作業 shall `crash_report_test_support.rs` が、`runtime-crate-split` で実行ファイルが分かれると本体と `areka-runtime` の両方に要ることを、`runtime-crate-split` の brief へ申し送る。
4. When 直したくなった不具合や要らない項目を見つけたとき, the 本仕様の作業 shall その場で直さず、範囲を受け持つ audit の brief へ申し送り、申し送った先と中身を記録に残す。

### Requirement 8: 振る舞いを変えないことの検証

**Objective:** 開発者として、改名と移動と参照の切り離しが振る舞いを変えていないことを、テストと実機の記録で確かめたい。それにより、各段階で main が緑のまま次の spec へ渡せるため。

#### Acceptance Criteria

1. The 本仕様の作業 shall 着手のときと完了のときに `cargo test --workspace --no-fail-fast -- --list` を取り、`: test$` の行から名前の末尾（最後の `::` から後ろ）の多重集合を作って、前後で一致させる。名前を改めたテストは要件 1.4 の対応表で説明する。
2. The 本仕様の作業 shall `cargo check --workspace --all-targets` の警告を 0 件のまま保つ。
3. When 本仕様の作業を終えたとき, the 全体テスト `pwsh -NoProfile -File tools/test-all.ps1` shall 緑になる。
4. The 本仕様の作業 shall ログの文言や出所を名前で読む実機テスト `crates/areka/tests/smoke_boot_loop_exit.rs` と `crates/areka/tests/emo2_real_run`（`AREKA_EMO2_REAL_RUN`）を、完了の前に 1 回ずつ回して緑を示す。
5. The 本仕様の作業 shall 終わる道（メニューの終了・窓を閉じる・煙の自動終了 `AREKA_APP_SMOKE_EXIT_MS`）ごとに、終わる段取りの記録の順を実機のログで前後の両方取り、同じであることを示す（ログの文言と出所の名前の改めは、対応表で読み替えて比べる）。
6. The 本仕様の作業 shall 1 ファイル 1,000 行の上限と、その例外表の決まりを守る。例外表の項目は増やさず（パスの書き換えだけ）、書き換えや移動で上限を超えるファイルが出たら分けて収める。
7. The 本仕様の作業 shall crate を新設せず、ワークスペースの根の `Cargo.toml`・各 crate の `Cargo.toml` の依存・`Cargo.lock` を変えず、新しい外の依存を足さない。
8. The 本仕様の作業 shall 振る舞いの検証のほかは、構造の証跡（grep の結果・テストの名前の一覧の比べ・警告 0 件・全体テストの緑）で示し、要件 8.4 と 8.5 のほかに実機の走行を求めない。
