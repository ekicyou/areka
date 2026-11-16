# Requirements Document

## Project Description (Input)

- **困っている人**: areka の開発者（本体 `crates/areka` を直し、見直し〔ponytail の audit〕を受け持ちごとに回す人）。
- **今の状態**: 前の spec `runtime-module-untangle`（2026-11-13 着地）で、本体の中のモジュール `runtime`（旧名 `emo2_boot`）の本番は本体のほかの部分を参照しなくなった。それでもまだ本体の中にあるので、runtime の 1 行を変えても本体ごと組み直され、テストも本体の 1 つの実行ファイルに入ったままになる。今の `crates/areka/src/runtime/` は 211 ファイル・74,693 行（本番 21,007・テストと支え 53,686。2026-11-16 main `715f5819` の測り）。
- **変えたいこと**: runtime を新しい crate `crates/areka-runtime/`（lib・`publish = false`・`areka-placement` に依存）へ移し、本体はそれを path 依存で使う。`target_map` とテスト専用の判定器 `transition_judge*` も `areka-runtime` に置く。本体の中で runtime を名指す道は全部 `areka_runtime::` にする。振る舞いは何も変えない。

（出どころ: `.kiro/specs/runtime-crate-split/brief.md`。分け方・順番・共通の約束・新旧のパスの対応表の正本は `.kiro/steering/roadmap.md`「テーマ別の決めごと」の「本体の crate 分け」。同じウェーブの約束の正本は同じ roadmap の「ウェーブ編成」の C21 の行。）

## Introduction

本仕様は、「本体の crate 分け」6 本の 3 本目として、areka 本体の bin crate の中にある実行系のモジュール（`crates/areka/src/runtime/`）を、新しい lib crate `areka-runtime` へそのまま移す。移すだけで、runtime の中身は直さない。前例は 1 本目の `placement-crate-split`（2026-11-06 着地）で、やり方をそれにそろえる。

受益者は areka の開発者であり、この仕様での「目に見える成果」は次のとおり開発者の側から定める。

- runtime の本番とテストが新しい crate にあり、本体はそれを依存として使っている。runtime を変えても、runtime の crate とそれを使う側だけが組み直され、runtime のテストは本体と別の実行ファイルで走る。
- runtime の境目が crate の公開面で守られている（外から使われない項目は外から見えない）。
- 動く ukagaka のアプリとしての振る舞い（窓・台詞・バルーン・切替・終了・ログの中身〔出所の名前を除く〕・MCP の `get_log` の分け方）が前と同じである。
- テストの数・警告・全体テスト・実機テストが、移す前と同じ合否を示す。

数（ファイル数・参照の数）は brief の測り（main `715f5819` ほか）と 2026-11-16 の作業の木の測りによる参考値で、設計の段で測り直す。

## Boundary Context

- **In scope**:
  - 新しい crate `crates/areka-runtime/` の新設と、runtime の本番とテストの移動（履歴を残す移動）。
  - runtime の公開面の付け替え（外から使われる項目だけを `pub` にする）と、本体の参照の書き換え（`mcp/{check_script,dump_balloon,dump_surface}.rs`・`input_events/user_break.rs`・`main.rs` ほか）。
  - テストの支えの開け方（`test-support` 機能）、テストの置き場の決め直し（本体を引く runtime のテスト・本体に残す落ちの報告の支え・ソースを文字で読むテスト）。
  - `target_map`・テスト専用の判定器 `transition_judge*` とその一族・境をまたぐテスト `reset_balloon_pos_tests.rs`・examples `collision-probe.rs` の `areka-runtime` への移し替え。
  - runtime を名指すパスの書き換え: 1,000 行の番人の例外表ほか `log-capture-kit` の表、ソースを文字で読むテスト、文書（`doc/COMPAT_ARCHITECTURE.md` はパスだけ・`doc/ssp-mcp/`・`doc/ukadoc-coverage/` の網羅台帳の根拠の場所と briefing・`.kiro/steering/structure.md`・`tech.md`）、`audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` の brief の「受け持つファイル」。
  - 本体の外の crate の説明の中に残る古い名前 `emo2_boot`・`Emo2Wiring`（2026-11-16 の作業の木で 14 ファイル・21 行・説明だけ）の書き換え。
  - ログの出所の名前の付け替え（`areka::runtime::…` → `areka_runtime::…`）と、名前で読んでいる所の書き換え。
  - 検証（テストの名前の一覧の比べ・警告 0 件・全体テスト・実機テスト・`get_log` の分け方の前後一致・`THIRD-PARTY-NOTICES.md` の作り直し）。
- **Out of scope**:
  - runtime の振る舞いの変更。直したくなった不具合・要らない項目・古い説明（brief の `audit-placement-follow` の申し送りの「範囲の外の古い参照」のうちパスの書き換えで済まないもの）は、その場で直さず `audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` へ申し送る。
  - 外から使われない `pub(crate)` の項目の削除。
  - 名前や依存の付け替え（前の spec で済んでいること。残っていたら前の spec の抜けとして直す＝これは In）。
  - input と session の crate 化と session の集め直し（`input-crate-split`・`session-module-gather`・`session-crate-split`）。
  - `tools/` の下のファイル（同じウェーブの約束 ⑴。`tools/load-flake.ps1` の `-Binary` の選択肢は `audit-tools-scripts` と申し合わせる）。
  - runtime に触れると書いているほかの brief（約 70 本）の書き換え（roadmap の新旧のパスの対応表で読み替える）。
  - 歴史の文書（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/`）の中の古いパス。
- **Adjacent expectations**:
  - 上流: `runtime-module-untangle` が着地していること（2026-11-13 に済み）。runtime の本番は本体のほかの部分を参照しない形になっていること（同 spec の到達点）。
  - 同じウェーブ C21 の約束（roadmap の C21 の行の ⑴〜⑸）: ⑴ この仕様は `tools/` に触らず、`doc/COMPAT_ARCHITECTURE.md` はパスの書き換えだけにする。`tools/load-flake.ps1` の `-Binary` の選択肢は `audit-tools-scripts` と申し合わせる（この仕様が先に着地したら `audit-tools-scripts` が選択肢を足し、`audit-tools-scripts` が先ならこの仕様の完了の申し送りで拾う）⑶ `balloon-flip-side-rules` は `crates/areka/src/runtime/` の下のファイルに触らず、記録の語（`flipped` ほか）を変えない（変えるなら申し合わせる）⑷ `Cargo.lock` の席はこの仕様だけが持つ ⑸ `audit-tools-scripts` はこの仕様と、`tools/load-flake.ps1` の選択肢のほかに書き換えるファイルが重ならない。⑵ は `wintf-surface-auto-promotion`・`pilot-windows-rs-next` が `Cargo.toml` に触らない約束で、この仕様の `Cargo.lock` の席を守る。並べ方そのものは roadmap が持つ約束で、この仕様の要件ではない。
  - 同じウェーブに置かない相手（roadmap の測り）: `balloon-text-visual-tree`・`audit-placement-follow-residue`（C22 へ）。
  - 下流: `input-crate-split`（次の 1 本）、`audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch`・`audit-test-kits`・`session-crate-split` ほか、v0.2.0 の作り直しで runtime に触る spec（`dola-interrupt-ticks` ほか）は、この仕様の着地を待ち、新しい crate の上で行う。

## Requirements

### Requirement 1: 新しい crate への移動

**Objective:** 開発者として、runtime の本番とテストを新しい crate `areka-runtime` へ移し、本体がそれを依存として使う形にしたい。それにより runtime の境目が crate の境目で守られ、runtime を変えても runtime の crate とそれを使う側だけが組み直されるため。

#### Acceptance Criteria

1. The 分割作業 shall `crates/areka-runtime/` を lib の crate として新設し、`publish = false` の印とその理由を同じ行に書き、`version.workspace = true` ほかワークスペースのほかの crate（`areka-placement` ほか）の慣習に従わせる。
2. The 分割作業 shall `crates/areka/src/runtime/` の本番とテストのファイルを、git が移動として追える形（履歴を残す移動）で `crates/areka-runtime/` の下へ移す。
3. The 分割作業 shall 今の `runtime/mod.rs` の中身を新しい crate の根（`lib.rs`）とする。
4. When 移動を終えたとき, the 本体の crate `areka` shall runtime を `areka-runtime` への path 依存として使い、`crates/areka/src/runtime/` を持たない。
5. The `areka-runtime` クレート shall `areka-placement` に path 依存で依存し、本体の crate `areka` には依存しない。
6. The 分割作業 shall ワークスペースの根の `Cargo.toml` を変えない。
7. The 分割作業 shall ワークスペースの外から新しい依存を足さない（足してよいのは、ワークスペースの中の path 依存と、`Cargo.lock` に既にある crate だけ）。
8. The `areka-runtime` クレート shall 本番の依存を、今の runtime が本番で使う crate（tracing・bevy_ecs・wintf・dola・areka-emo-present・areka-ghost・areka-sakura・areka-parsers・areka-emo-text・areka-emo-compose・areka-seriko・areka-kanade・areka-actor・areka-emo-atlas・areka-sylphya・thiserror・windows・areka-placement。正確な一覧は設計の段で測る）に収め、ワークスペースのほかの crate と同じ形で受ける（外の crate は `workspace = true`、ワークスペースの中の crate は `{ path = "../…" }`）。
9. The `areka-runtime` クレート shall テストでだけ使う crate（log-capture-kit・temp-path-kit・sample-ghost-kit・areka-update・shiori-host32-host ほか）を本番の依存に入れない（3.4 の決まりに従う）。
10. The `areka-runtime` クレート shall 1 ファイル 1,000 行の上限と、その例外表の決まりを、本体と同じく守る。例外表の項目は増やさず（パスの書き換えだけ）、書き換えや整形で上限を超えるファイルは分けて収める。

### Requirement 2: 公開面の付け替えと本体の参照

**Objective:** 開発者として、runtime の公開面を外から使われる分だけに絞り、本体の中で runtime を名指す道を新しい crate の名前にそろえたい。それにより runtime の外からは公開面しか触れなくなり、runtime の中を直すときに外への影響を公開面だけで見積もれるため。

#### Acceptance Criteria

1. The 分割作業 shall runtime の外（本体の本番・テスト・examples）から使われている `pub(crate)` の項目（モジュール・型・関数・定数・再公開・メソッド。主な面は `RuntimeWiring` とその関数・`frame` の `ZOrderAbsentReports`・`run_ghost_quit_phase`・`TalkLifecycleSignal`・`NoUserBreakSignal`・`HitRegion`・`resolve_hit_region`・`ConsumerLedger`・`target_map`）だけを `pub` にする。正確な一覧は設計の段で測る。
2. The 分割作業 shall runtime の外から一度も使われていない `pub(crate)` の項目を、`pub(crate)` のまま残し、削除しない。
3. The 分割作業 shall runtime の中で自分を指すパス（`crate::runtime::…`）を、新しい crate の根を指すパスへ書き換える。
4. The 分割作業 shall 本体の中で runtime を指すパス（`crate::runtime::…`・`runtime::…`）を、全部新しい crate を指すパス（`areka_runtime::…`）へ書き換える。
5. If 外から使われている項目を `pub` にすると公開面の型が非公開の型を名指すことになる場合, then the 分割作業 shall その非公開の型も、外から名指せる最小の範囲で公開し、どの項目をそのために開けたかを記録する。
6. If runtime の本番のファイルの説明の中のリンクが本体の項目（`crate::session::…`・`crate::is_benign_boot_error` ほか）を指している場合, then the 分割作業 shall そのリンクを、新しい crate から解ける形（本体を名指す地の文など）に書き換え、意味を変えない。
7. When 移動を終えたとき, the 分割作業 shall `areka-runtime` の本番のコードが本体の項目を参照していないことを grep の結果で示す。

### Requirement 3: テストの置き場とテストの支え

**Objective:** 開発者として、runtime の外のテストが使う支えを写しを作らずに使い続け、テストを置く crate を「そのテストが触る範囲を含む一番下の crate」にそろえたい。それにより支えが二重にならず、下の crate に上の crate への依存が入り込まず、本体と `areka-runtime` のどちらのテストの実行ファイルも前と同じ守りを持つため。

#### Acceptance Criteria

1. The `areka-runtime` クレート shall runtime の外のテストが使う支え（`spine`・`spine::hold_support`・`sample_test_support`・`frame` のバルーンの実行の支えほか。正確な一覧は設計の段で測る）を、`test-support` という名の機能を付けたときだけ外から使える形で開ける（`placement-crate-split` と同じやり方）。
2. The 本体の crate `areka` shall `areka-runtime` の `test-support` 機能を、テストのビルドの依存（dev-dependency）でだけ付ける。
3. The 分割作業 shall テストの支えの写しを作らず、1 つの支えを 1 か所だけに置く（見本のゴーストの入口 `sample_test_support` は、ワークスペースの慣習どおり crate ごとに 1 つ）。
4. The 分割作業 shall 共有のテスト用の crate（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）を、どの crate の本番の依存表（optional を含む）にも書かず、今の番人（`with_default_guard_test.rs`・`sample_path_guard_test.rs`）の決まりを変えない。（仮定: 3.1 と 3.4 が `spine` で両立しない場合の扱いは未決の議題 2 で決める。）
5. The 分割作業 shall runtime のテストのうち本体を引くもの（2026-11-16 の作業の木で、`crate::session`・`crate::ghost_session`・`crate::input_events`・`crate::restore_merged_placements`・`crate::app_exit` ほかを引くファイルが約 40）を、境をまたぐテストとして上の crate（本体）に置くか、本体を引かない形に移すかを 1 本ずつ決め、どちらの場合も前と同じ合否を示すようにする。（仮定: 共通の約束「境をまたぐテストは上の crate に置く」に従い、本体に置く。）
6. The 分割作業 shall runtime の口のテストの倒れ先（`session_hooks.rs` の `#[cfg(test)]` の `FALLBACK` が本体の口 `session::hooks::HOOKS` を読む所）を、`areka-runtime` のテストから本体が見えなくなっても、口を読むテストが前と同じ合否を示す形に決め直す。（仮定: 本体の口を要るテストは本体に置き、`areka-runtime` の中のテストの倒れ先は本番と同じ `NO_BODY` にする。未決の議題 3。）
7. The 分割作業 shall 本体の `session/runtime_boot_tests.rs` が `#[path = "../runtime/…"]` で取り込む e2e の 2 本（`script_security_e2e_tests.rs`・`readme_script_e2e_tests.rs`）の置き場を、本体のファイルとして本体の側へ寄せ、本体が下の crate のファイルを `#[path]` で取り込まない形にする。
8. The 本体のテストの実行ファイルと `areka-runtime` のテストの実行ファイル shall どちらも、テストを走らせる前にプロセスの寿命の MTA を立て、テストの途中で落ちたときに同じ形の落ちの報告を標準エラーへ出す（今は runtime の中の `crash_report_test_support.rs` だけが据えているので、分けた後も本体の側から落ちの報告が消えないようにする。前例は `crates/areka-placement/src/test_crash_report.rs` で、その 1 行目と `crates/wintf/src/test_crash_report.rs` の 1 行目の「写しの相手」の行を置いた先のパスへ合わせる）。
9. The 分割作業 shall 逆向きに runtime のテストが使っていた本体の支え（`placement::test_support`〔今は `areka-placement`〕・`shared_test_support`・`scope_bind_fixture_test_support`・`install::fetch_url_test_support`・`crate_source_test_support` ほか）を、それを使うテストの置き場に合わせて、写しを作らずに引ける形にする。
10. The `areka-runtime` クレートのテストの実行ファイル shall 本体と同じく、テストの数え方（`cargo test --workspace -- --list`）で拾われ、全体テストの道具 `tools/test-all.ps1` を直さずに走る。

### Requirement 4: target_map・判定器・境をまたぐテスト・examples

**Objective:** 開発者として、表示対象の写像 `target_map`・テスト専用の判定器 `transition_judge*`・配置と runtime の両方を使うテスト・examples を、それぞれ触る範囲を含む一番下の crate へ置きたい。それにより、前の spec が本体に一時的に残したものが、本来の置き場へ収まるため。

#### Acceptance Criteria

1. The `areka-runtime` クレート shall `target_map`（scope から表示対象 `TargetId` への写像の正本）を持ち、本体の使い手（`input_events/user_break.rs`・`mcp/{check_script,dump_balloon,dump_surface}.rs` とそのテスト）が `areka_runtime::target_map` を通して前と同じ値を得るようにする。
2. The 分割作業 shall 本体のテスト専用のモジュール `crates/areka/src/placement_tests/` の判定器 `transition_judge.rs`・`transition_judge_offset.rs`・`transition_judge_verdict.rs` とそのテストの一族（`transition_judge*.rs`・`transition_judge_test_support.rs`・`transition_signoff_tests.rs`・`transition_signoff_procedure_tests.rs`）を、`areka-runtime` のテスト専用へ移し、本番のビルドに含めない。
3. The 分割作業 shall runtime のテストのうち判定器を使う 2 本（`frame_transition_atomicity_tests.rs`・`frame_transition_branch_tests.rs`）が、移した先の判定器を使って前と同じ合否を示すようにする。
4. The 分割作業 shall 配置と runtime の両方を使う `placement_tests/reset_balloon_pos_tests.rs`（`switch_assets` を使う）を、`areka-runtime` へ移す。
5. The 分割作業 shall `placement_tests/` に残るほかのテスト（`spawn_assembly_tests`・`measure_bake_*`・`measure_template_tests`・`spawn_zorder_pair_deferred_tests`・分岐の対応表 2 本ほか）のうち、本体を引かず runtime と配置だけを使うものを `areka-runtime` へ移し、本体を引くものは本体に残す。どちらにしたかを 1 本ずつ記録する。
6. The 分割作業 shall `target_map.rs`・`hit_region.rs` を `#[path]` で取り込む examples `collision-probe.rs`（と付随の `collision-probe/` フォルダ）を `areka-runtime` の examples へ移し、`#[path]` をやめて crate の依存（`areka-runtime`・`areka-placement` の公開面）で読む形にする。
7. When examples を組んだとき, the 本体と `areka-runtime` の examples shall `cargo check --workspace --all-targets` で警告なしに通る。
8. The 分割作業 shall `audit-placement-follow` の申し送りの「テストだけが使う `areka-placement` の公開」（`on_balloon_drag`・`sync_drag_writer`・`WORK_AREA_CLAMP_TAG`・`AnchorSource`・`WorkAreaResolution`・`work_area_for_window_with_origin`・`move_window_with_route`・`MonitorSources::is_empty`・`MonitorDpiEntry`・`transition_diag.rs` の語と欄）を、判定器を移すときに 1 件ずつ扱い（公開のまま残す／`areka-placement` の `test-support` 機能の下へ移す／使うテストを crate の中へ移す）、どれにしたかを記録する。本番の公開面の形と振る舞いは変えない。（仮定: 未決の議題 6。）

### Requirement 5: runtime を名指すパスと古い名前の書き換え

**Objective:** 開発者として、runtime の古いパスと古い名前を名指す所を全部新しいパスへ直したい。それにより表・テスト・文書が実在しないパスを指したまま黙って通ることがなくなるため。

#### Acceptance Criteria

1. The 分割作業 shall `crates/log-capture-kit/tests/` の表（`file_length_guard_test.rs` の 1,000 行の例外表と較正の項目・`workspace_scan_test.rs`・`temp_path_guard_test.rs`・`with_default_guard_test.rs` ほか）にある runtime と移すテストのパスを、移した先のパスへ書き換え、理由の文の主語が変わるものは新しい crate に合わせる。
2. The 分割作業 shall ソースを文字で読むテスト（`include_str!`・`CARGO_MANIFEST_DIR`・`crate_source_test_support` を使うもの。`frame_schedule_tests.rs` の `../session/ghost_quit.rs`・`zorder_wiring_tests.rs` の `../session/runtime_boot.rs`・`../ghost_session.rs`・`../main.rs`・`strict_judges_tests.rs` の `doc/ssp-mcp/areka-tools.md`・`film_playback_e2e_tests.rs`・`balloon_visibility_phase_zorder_chain_tests.rs` ほか）を、`placement-crate-split` の原則で置き、移した先のファイルを読むようにする: 自分の crate のファイルは自分の crate のテストが読む。別の crate のファイルを読むのは、コードの依存と同じ向き（上の crate が自分の依存する下の crate を読む）に限り、下の crate（`areka-runtime`）が上の crate（本体）のファイルを読むことはしない。
3. The 本体の crate `areka` shall 下の crate のファイルを文字で読むとき、`placement-crate-split` が置いた 1 つの読み口（crate の名前と crate の中のパスを並べて書く形）を通して読み、相対の道を散らさない。
4. If ソースを文字で読むテストが読む先のファイルを見つけられない場合, then the そのテスト shall 黙って通らず赤になる。crate ごとに割ったソースの木を歩くテストは、合わせて移す前と同じ範囲を歩き、範囲が黙って狭まらない。
5. The 分割作業 shall 文書の中の runtime のパスを、移した先のパスへ書き換える: `doc/COMPAT_ARCHITECTURE.md`（パスの書き換えだけ・約 38 か所）・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所（約 48 か所）・`doc/ukadoc-coverage/briefing-assets.md`・`briefing-shiori.md`・`doc/ssp-mcp/areka-tools.md`・`doc/emo2-conformance-scope.md` ほか。網羅台帳は別々の行だけを書き換え、`doc/ukadoc-coverage/report/` は手で直さず作り直す。
6. The 分割作業 shall `.kiro/steering/structure.md` に新しい crate `areka-runtime` の節を足し、本体の節の `runtime/` の記述を移した先へ合わせ、`.kiro/steering/tech.md` の runtime のパスを書き換える。
7. The 分割作業 shall `audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` の brief の「受け持つファイル」を、移した先のパスへ書き換える。
8. The 分割作業 shall 本体の外の crate の説明（とテストの見本の値のうち説明だけのもの）に残る古い名前 `emo2_boot`・`Emo2Wiring`（`areka-parsers`・`areka-emo-text`・`areka-ghost`・`areka-kanade`・`areka-seriko`・`log-capture-kit`・`ukadoc-survey`・`wintf` の 14 ファイル・21 行）を、今の名前と移した先の場所へ書き換え、動きを変えない。
9. When 書き換えを終えたとき, the 分割作業 shall 古いパス `crates/areka/src/runtime/`（と `src/runtime/` のうち本体の runtime を指すもの・`placement_tests/transition_judge` ほか移したテストのパス）を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す（歴史の文書 `roadmap-history.md`・`.kiro/specs/completed/` と、新旧のパスの対応表を持つ `roadmap.md` は除く。`wintf` と `dola` の自分の `runtime/` は当たりにしない）。
10. When 書き換えを終えたとき, the 分割作業 shall `rg -n 'emo2_boot|Emo2Wiring' crates -g '!crates/areka/**'` が 0 件であることを示す。
11. The 分割作業 shall `tools/` の下のファイルを変えない（同じウェーブの約束 ⑴）。
12. If 書き換えを終えた後に `tools/` の下に runtime の古いパスか古い出所の名前が見つかった場合, then the 分割作業 shall それを直さず、`audit-tools-scripts` との申し合わせとして完了の申し送りに記録する（2026-11-16 の作業の木の測りでは 0 件）。
13. When この仕様の完了の申し送りを書くとき, the 分割作業 shall `tools/load-flake.ps1` の `-Binary` に `areka-runtime` のテストの実行ファイルを足すことを、`audit-tools-scripts` の着地の前後に応じて申し送る（`audit-tools-scripts` が先に着地していれば、この仕様の完了の申し送りで拾う先の spec を名指す。後なら `audit-tools-scripts` が足す）。

### Requirement 6: ログの出所の名前

**Objective:** 開発者として、runtime のログの出所の名前が新しい crate の名前にそろい、名前で読む所が全部それに追従してほしい。それにより `RUST_LOG` で runtime の診断を開く・実機テストがログを拾う・MCP の `get_log` がログを分けることが、移した後も前と同じく働くため。

#### Acceptance Criteria

1. The `areka-runtime` クレート shall モジュールのパスから付く本番のログの出所を、新しい crate の名前（`areka_runtime::…`）で出す。
2. The 分割作業 shall モジュールのパスを写した直書きの出所（`"areka::runtime::…"` の形のもの）があれば、新しい出所の名前に合わせる。
3. The 分割作業 shall 語彙として付けた直書きの出所（`"areka::persist::*"`・`"areka::log::*"`・`"areka::perf"` ほか、モジュールのパスに頼らないもの）を変えない。
4. The 分割作業 shall ログの出所を名前で読む所を、新しい出所の名前へ書き換える: `RUST_LOG` の文字列（`crates/areka/tests/stay_on_top_real_run.rs` ほか）・テストの中の出所の定数と絞り込み（`balloon_background_tests.rs`・`assets_tests.rs`・`shell_cursor_assets_tests.rs` ほか）・`log_history.rs` の決まりとそのテスト（`log_history_tests.rs` の `areka::runtime`）・`doc/ssp-mcp/log-convention.md`・実機の手順書・ログの行の見本（`emo2_real_run/common_tests.rs` ほか）。
5. When `RUST_LOG` に `areka_runtime=debug` を与えたとき, the areka shall runtime の診断のログを出す。
6. The areka shall MCP の `get_log` の 5 種別への分け方（`log_history` の決まり）で、移す前に runtime の出所の行が当たっていた種別（またはどれにも当たらないこと）を、移した後の `areka_runtime::…` の行でも同じにする。
7. When 書き換えを終えたとき, the 分割作業 shall 古い出所の名前 `areka::runtime`（`areka::runtime::`・`areka::runtime=`・行末の `areka::runtime` の形）を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す（歴史の文書と、新旧のパスの対応表を持つ `roadmap.md` は除く）。
8. The 分割作業 shall ログの本文（メッセージの文字列。実機テストが読む印 `runtime: 実 sink 結線が成立しました（wire 成立）` ほか）を変えない。

### Requirement 7: 振る舞いを変えないことの検証

**Objective:** 開発者として、移した後も areka の振る舞いとテストの合否が前と同じであることを、決めた手順の証跡で確かめたい。それにより「移しただけ」が言葉でなく証跡で示されるため。

#### Acceptance Criteria

1. The 分割作業 shall runtime の振る舞い（窓・台詞・バルーン・切替・読み直し・終了・前後の並び・ログの中身）を変えない。直したくなった不具合は直さず、範囲を受け持つ `audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` へ申し送りとして記録する。理由の文が移した後に事実でなくなる `#[allow(...)]`（「areka は lib を持たない bin」ほかを理由にするもの）も、属性と文を変えず、一覧を同じ 3 本へ申し送る。
2. When 着手のときと完了のとき, the 分割作業 shall `cargo test --workspace --no-fail-fast -- --list` を取り、`: test$` の行から名前の末尾（最後の `::` から後ろ）の多重集合を作り、前後で一致することを示す。
3. If 前後の多重集合が一致しない場合, then the 分割作業 shall 名前を改めたテストを対応表で 1 件ずつ説明し、説明できない差を残さない。
4. The 分割作業 shall `cargo check --workspace --all-targets` の警告を 0 件のまま保つ。
5. When 完了の前に, the 分割作業 shall `pwsh -NoProfile -File tools/test-all.ps1` が緑であることを示す。
6. When 完了の前に, the 分割作業 shall ログを名前で読む実機テスト `crates/areka/tests/smoke_boot_loop_exit.rs`・`crates/areka/tests/emo2_real_run`（brief が名指した 2 つ）と、`RUST_LOG` で `areka::runtime::…` を名指す `crates/areka/tests/stay_on_top_real_run.rs` を、新しい出所の名前で 1 回ずつ走らせて緑を示す。（仮定: 3 本目は 2026-11-16 の測りで見つけたもの。未決の議題 1。）
7. The 分割作業 shall 実機で回すテストを 7.6 のものだけにし、ほかは構造の証跡（テストの名前の一覧の比べ・警告・全体テスト・grep）で足りるとする。この仕様は知らせや口の向きを逆にしないので、「向きを逆にした道」の実機の走行は無い。
8. The 分割作業 shall MCP の `get_log` の分け方が前後で同じこと（6.6）を、`log_history` のテストで示す。
9. The 分割作業 shall `THIRD-PARTY-NOTICES.md` を `cargo about generate` で作り直し、手で書かない。

## 設計の段への申し送り

1. **数の取り直し**（議題ではなく申し送り）: 公開にする `pub(crate)` の一覧・runtime の外のテストが使う支えの一覧（brief の `spine` 28 ファイル 46 回・`spine::hold_support` 2・`sample_test_support` 2 ファイル 8 回は 11-13 より前の数）・本体を引く runtime のテスト（2026-11-16 の作業の木で約 40 ファイル。`runtime-module-untangle` の research の §24 は 77 行）・`target_map` の使い手・文書の中のパス（`doc/` 95 か所・`crates/` 13 か所・steering 3 か所）を、設計の段で数え直す。
2. **brief の古くなった記述**: `mcp/dump_surface.rs:415` の `spin_wait_until` は、今は `crate::runtime::spine::wait_until`（426 行目）。
3. **brief の触るファイルの一覧に無かった所**（申し送り・同じウェーブの兄弟の触るファイルと重ならないことは設計の段で確かめる）: `crates/areka/tests/stay_on_top_real_run.rs`（`RUST_LOG` に `areka::runtime::…`）・`crates/areka/tests/emo2_real_run/common_tests.rs`（ログの行の見本）・`crates/log-capture-kit/tests/workspace_scan_test.rs`（`runtime/spine_display_tests.rs` のパス）・`crates/areka/src/log_history_tests.rs`・`doc/emo2-conformance-scope.md`・`doc/ukadoc-coverage/briefing-shiori.md`・`doc/ssp-mcp/areka-tools.md`・`crates/areka-placement/src/test_crash_report.rs` と `crates/wintf/src/test_crash_report.rs` の 1 行目。
4. **本体の中の runtime の説明に残る `emo2_boot` の 2 行**（`runtime/mod.rs:69`・`spine.rs:33`）は、完了した spec の設計のファイル名 `tests/emo2_boot_spine_test.rs` を引いた説明で、歴史の引用として残す（仮定。5.10 の grep は本体の外だけを数える）。

## 未決の議題（要件の討議で決める）

1. **実機テストの本数**: brief は `smoke_boot_loop_exit.rs`・`emo2_real_run` の 2 つを名指すが、2 つが読むのはメッセージの本文（`runtime: 実 sink 結線が…`）で、出所の名前ではない。2026-11-16 の測りで、`stay_on_top_real_run.rs` の `RUST_LOG` が `areka::runtime::…` を名指していた（出所を名前で読む実機テスト）。**仮定**: 3 本とも 1 回ずつ回す（要件 7.6）。
2. **`spine` の支えの開け方と番人の決まり**: brief は `spine` を `areka-runtime` の `test-support` 機能で開ける。ところが `spine` は `log_capture_kit`・`sample_ghost_kit`・`shiori_host32_host` と本体の `crate::session::ghost_quit::KanadeNoticeRx`・`crate::session::runtime_boot::runtime_boot_options` を引く。機能の下で開けるには、共有のテスト用の crate を本番の依存表（optional）に書くことになり、`placement-crate-split` の要件 3.4 の番人（`with_default_guard_test.rs`・`sample_path_guard_test.rs`）の決まりとぶつかる。**仮定**: 要件 3.1・3.3・3.4 を満たす形を設計で探し、両立しなければ ① `spine` を境をまたぐ支えとして本体の側に置く ② 番人の決まりを理由つきで狭く広げる、のどちらかを設計の議題で決める（要件の段の推奨は ①＝共通の約束「境をまたぐテストは上の crate に置く」に沿い、番人を変えない）。
3. **口のテストの倒れ先**: `session_hooks.rs` の `#[cfg(test)]` の `FALLBACK` は本体の口 `session::hooks::HOOKS` を読む（`runtime-module-untangle` の設計の議題 1）。`areka-runtime` のテストからは本体が見えない。**仮定**: 本体の口を要るテストは本体に置き、`areka-runtime` の中のテストの倒れ先は本番と同じ `NO_BODY` にする（要件 3.6）。どのテストが本体の口を要るかは設計で数える。
4. **本体を引く runtime のテストの置き場**: 約 40 ファイル（`crate::session`・`ghost_session`・`input_events`・`restore_merged_placements`・`app_exit` ほか）。brief の Desired Outcome は「`areka-runtime` が runtime の本番とテストを持つ」だが、共通の約束は「境をまたぐテストは上の crate に置く」。**仮定**: 本体に置く（要件 3.5）。数が多い場合に、本体を引く部分を口や支えで差し替えて `areka-runtime` に置く道を取るかは、設計で 1 本ずつ決める（振る舞いの確かめの中身は変えない）。
5. **`audit-placement-follow` の申し送りの「範囲の外の古い参照」**（`frame_transition_atomicity_tests.rs` の行の番号での指し・`drain_resnap.rs`・`frame_chain_finalize_tests.rs` の「唯一の位置ライター」・`frame_work_area_resnap_hold_tests.rs` の `should_panic` に `cfg_attr(not(debug_assertions), ignore …)` が無い件・`frame/move_shift.rs` の注）: **仮定**: 判定器のパスが変わる `frame_transition_atomicity_tests.rs` の行の指しだけは、パスの書き換え（要件 5）のついでに名で指す形に直し、ほかの 3 件は振る舞いを変えないため直さず `audit-emo2-boot-frame` へ申し送る（`cfg_attr` を足すのは `--release` のテストの合否を変えるので要件 7.1 の「直さない」に当たる）。
6. **テストだけが使う `areka-placement` の公開の扱い**（要件 4.8）: `audit-placement-follow` は「この spec の設計で決める」と申し送った。**仮定**: 判定器を `areka-runtime` へ移しても使い手は crate の外のテストのままなので、`areka-placement` の `test-support` 機能の下へ移すのを推奨とし、`MonitorDpiEntry` のように公開の欄の要素の型は公開のまま残す。最終の割り振りは設計で 1 件ずつ決める。
7. **`placement_tests/` に残るほかのテストの置き場**（要件 4.5）: 対応表は `spawn_assembly_tests` の置き場を（4）`input-crate-split` で決め直すとし、`emo2_boot` を使う 5 本の置き場は（3）＝この仕様で決めるとしている。2026-11-16 の測りで、`reset_balloon_pos_tests`・`measure_bake_count_tests`・`measure_bake_log_tests` は `crate::runtime` だけを引き（`measure_bake_count_tests` は下の crate のファイルを読む読み口 `crate_source_test_support` も使う）、`spawn_assembly_tests` は `crate::input_events` を引き、`measure_template_tests`・`spawn_zorder_pair_deferred_tests` は本体のファイルを文字で読み、分岐の対応表 2 本は crate をまたいで並ぶ表。**仮定**: `reset_balloon_pos_tests`・`measure_bake_*` は `areka-runtime` へ移し（読み口の置き場は設計で決める）、ほかは本体に残す。
