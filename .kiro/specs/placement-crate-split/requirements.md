# Requirements Document

## Project Description (Input)

- **困っている人**: areka の開発者（本体 `crates/areka` を直し、見直し〔ponytail の audit〕を受け持ちごとに回す人）。
- **今の状態**: `crates/areka` は「他の crate への橋渡しに徹する」本体のはずが、`src` だけで本番 約 60,500 行・テスト 約 146,300 行を抱える、lib を持たない bin crate になっている。そのため、どのモジュールも `pub(crate)` で互いの中を触れて境目が守られない（参照し合いが 17 組）、どこを 1 行変えても crate 全体を組み直し、テストは 1 つの実行ファイル（`cargo test -p areka --bin areka`）に全部入る、見直しの受け持ちを crate の境目で切れずパスで割るしかない、という困りごとがある。窓の配置（`placement`）はこのうち一番切り出しやすい塊で（本番 32 ファイル 約 12,900 行・本体のほかの部分への依存 0）、6 本に分けた「本体の crate 分け」の 1 本目に当たる。
- **変えたいこと**: 配置を新しい crate `crates/areka-placement/`（lib・`publish = false`）へ移し、本体はそれを path 依存で使う。振る舞いは何も変えない。配置の公開面は外から使われる分だけ `pub` にし、依存の向きの規約の文を実際の参照と一致させる。

（出どころ: `.kiro/specs/placement-crate-split/brief.md`。分け方・順番・共通の約束・新旧のパスの対応表の正本は `.kiro/steering/roadmap.md`「テーマ別の決めごと」の「本体の crate 分け」。）

## Introduction

本仕様は、areka 本体の bin crate の中にある窓の配置のモジュール（`crates/areka/src/placement/`）を、新しい lib crate `areka-placement` へそのまま移す。移すだけで、配置の中身は直さない。

受益者は areka の開発者であり、この仕様での「目に見える成果」は次のとおり開発者の側から定める。

- 配置の本番とテストが新しい crate にあり、本体はそれを依存として使っている。
- 配置の境目が crate の公開面で守られている（外から使われない項目は外から見えない）。
- 動く ukagaka のアプリとしての振る舞い（窓の置き場・追従・記録の出所の扱いを除くログの中身・保存される位置）が前と同じである。
- テストの数・警告・全体テスト・実機テストが、移す前と同じ合否を示す。

数（ファイル数・参照の数）は brief の測り（main `18e4cb30` ほか）による参考値で、設計の段で測り直す。

## Boundary Context

- **In scope**:
  - 新しい crate `crates/areka-placement/` の新設と、配置の本番とテストの移動（履歴を残す移動）。
  - 配置の公開面の付け替え（外から使われる項目だけを `pub` にする）と、本体の参照の書き換え。
  - テストの支えの開け方（`test-support` 機能）、テストと examples の置き場の移し替え。
  - 配置を名指すパスの書き換え: 1,000 行の番人の例外表ほか `log-capture-kit` の表、ソースを文字列で読むテスト、配置の中の自分を指すパス、文書（`doc/`・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所・`.kiro/steering/structure.md`）、`tools/perf` の見本、ソースの説明の中のパス。
  - ログの出所の名前の付け替えと、名前で読んでいる所の書き換え。
  - `audit-placement-core`・`audit-placement-follow` の brief の「受け持つファイル」の新しいパスへの書き換え。
  - 依存の向きの規約の文と、実際の参照の食い違い 5 件の解消。
  - 検証（テストの名前の一覧の比べ・警告 0 件・全体テスト・実機テスト 1 本・性能の判定の道具の確かめ・`THIRD-PARTY-NOTICES.md` の作り直し）。
- **Out of scope**:
  - 配置の振る舞いの変更。直したくなった不具合・要らない項目・wintf との重なり（`MonitorSnapshot` が包む `Monitor`・`diag::MonitorRecord` と wintf の `transition_diag::MonitorRecord`・`RectPx`）は、その場で直さず `audit-placement-core`・`audit-placement-follow` へ申し送る。
  - 外から使われない `pub(crate)` 項目（約 31 件）の削除。
  - 配置を wintf へ寄せること（wintf は crates.io へ出す汎用の crate で、配置は ukagaka の意味と areka 系の依存を持つ）。
  - `emo2_boot` ほか本体のほかの部分の分け方（「本体の crate 分け」の後の 5 本）。
  - 配置に触れると書いているほかの約 19 本の brief の書き換え（roadmap の新旧のパスの対応表で読み替える）。
  - 歴史の文書（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/`）の中の古いパス。
- **Adjacent expectations**:
  - 上流: v0.1.0 の版の門を越えていること、`areka-test-binary-stability` が分ける前の形で測り終えていること（どちらも 11-04 までに済み）。
  - 同じウェーブ C18 の約束: 見直しの 6 本は `crates/areka/` とこの仕様が触る本体の外のファイルに触らない。`Cargo.lock` の席はこの仕様が持つ。`doc-compat-architecture-rename`・`multi-ghost-runtime`・`doc-tree-tidy`・`/release` の版上げ・`crates/areka` を触るほかの仕様とは同じウェーブに置かない（roadmap が持つ並べ方の約束で、この仕様の要件ではない）。
  - 下流: `runtime-module-untangle`（次の 1 本）、`audit-placement-core`・`audit-placement-follow` はこの仕様の着地を待つ。`transition_judge*` はこの仕様のあいだ本体に残り、`runtime-crate-split` が `areka-runtime` のテスト専用へ移す。

## Requirements

### Requirement 1: 新しい crate への移動

**Objective:** 開発者として、配置の本番とテストを新しい crate `areka-placement` へ移し、本体がそれを依存として使う形にしたい。それにより配置の境目が crate の境目で守られ、配置を変えても配置の crate とそれを使う側だけが組み直されるため。

#### Acceptance Criteria

1. The 分割作業 shall `crates/areka-placement/` を lib の crate として新設し、`publish = false` の印とその理由を同じ行に書き、`version.workspace = true` ほかワークスペースのほかの crate の慣習に従わせる。
2. The 分割作業 shall `crates/areka/src/placement/` の本番とテストのファイルを、git が移動として追える形（履歴を残す移動）で `crates/areka-placement/` の下へ移す。
3. The 分割作業 shall 今の `placement/mod.rs` の中身を新しい crate の根（`lib.rs`）とする。
4. When 移動を終えたとき, the 本体の crate `areka` shall 配置を `areka-placement` への path 依存として使い、`crates/areka/src/placement/` を持たない。
5. The 分割作業 shall ワークスペースの根の `Cargo.toml` を変えない。
6. The 分割作業 shall ワークスペースの外から新しい依存を足さない（足してよいのはワークスペースの中の path 依存だけ）。
7. The `areka-placement` クレート shall 本番の依存を、今の配置が本番で使う外の crate（wintf・bevy_ecs・tracing・areka-parsers・areka-emo-compose・areka-emo-present・areka-sylphya・areka-emo-atlas・areka-ghost・windows・thiserror）に収め、どれもワークスペースのほかの crate と同じく `workspace = true` で受ける（`windows` の機能はワークスペースの表のものをそのまま受け、本番のソースが使う機能は `Win32_Foundation`・`Win32_UI_WindowsAndMessaging` の範囲に収める）。
8. The `areka-placement` クレート shall 本体の crate `areka` に依存しない。
9. The `areka-placement` クレート shall 1 ファイル 1,000 行の上限と、その例外表の決まりを、本体と同じく守る。例外表の項目は増やさず（パスの書き換えだけ）、書き換えや整形で上限を超えるファイルは分けて収める。

### Requirement 2: 公開面の付け替え

**Objective:** 開発者として、配置の公開面を外から使われる分だけに絞りたい。それにより配置の外からは公開面しか触れなくなり、配置の中を直すときに外への影響を公開面だけで見積もれるため。

#### Acceptance Criteria

1. The 分割作業 shall 配置の外（本体の本番・テスト・examples）から使われている `pub(crate)` の項目（モジュール・型・関数・定数・再公開・メソッド）だけを `pub` にする。
2. The 分割作業 shall 配置の外から一度も使われていない `pub(crate)` の項目を、`pub(crate)` のまま残し、削除しない。
3. The 分割作業 shall 配置の中で自分を指すパス（`crate::placement::…`）を、新しい crate の根を指すパスへ書き換える。
4. The 分割作業 shall 本体の中で配置を指すパスを、新しい crate を指すパス（`areka_placement::…`）へ書き換える。
5. If 外から使われている項目を `pub` にすると公開面の型が非公開の型を名指すことになる場合, then the 分割作業 shall その非公開の型も、外から名指せる最小の範囲で公開し、どの項目をそのために開けたかを記録する。

### Requirement 3: テストの支えとテストの置き場

**Objective:** 開発者として、配置の外のテストが使う支えを写しを作らずに使い続けたい。また、テストを置く crate を「そのテストが触る範囲を含む一番下の crate」に揃えたい。それにより支えが二重にならず、境をまたぐテストが下の crate に上の crate への依存を持ち込まないため。

#### Acceptance Criteria

1. The `areka-placement` クレート shall 配置の外のテストが使う支え（`test_support` の `capture_logs`・`LogEvent`・`ExpectField`・`expect_one` と、`placement_shared_test_support` の `balloon_root`・`emo2_root`・`emily_shaped_scopes`・`with_com_initialized`・`WA` ほか）を、`test-support` という名の機能を付けたときだけ外から使える形で開ける。
2. The 本体の crate `areka` shall `areka-placement` の `test-support` 機能を、テストのビルドの依存（dev-dependency）でだけ付ける。
3. The 分割作業 shall テストの支えの写しを作らず、1 つの支えを 1 か所だけに置く。
4. Where `capture_logs` の汎用の部分を `log-capture-kit` へ下ろす場合, the 分割作業 shall 下ろした部分を `areka-placement` の側に残さず、どちらか 1 か所だけに置く。
5. The 分割作業 shall 配置のテストのうち本体のほかの部分を使う 3 本（`reset_balloon_pos_tests.rs`→`emo2_boot::switch_assets`・`spawn_assembly_tests.rs`→`input_events::attach_char_pointer_handlers`・`transition_judge_tests.rs`→`emo2_boot::target_map`）を本体に残す。
6. The 分割作業 shall `#[cfg(test)]` だけの判定器 `transition_judge.rs`・`transition_judge_offset.rs`・`transition_judge_verdict.rs`（とそのテストの一族＝`transition_judge*.rs`・`transition_signoff_tests.rs`・`transition_signoff_procedure_tests.rs`）を、配置のフォルダから本体のテスト専用のモジュールへ移し、本番のビルドに含めない。
7. The 分割作業 shall 本体の `emo2_boot` のテストのうち `transition_judge*` を使う 2 本（`frame_transition_atomicity_tests.rs`・`frame_transition_branch_tests.rs`）が、移した先の判定器を使って前と同じ合否を示すようにする。
8. The `areka-placement` クレートのテストの実行ファイル shall 本体と wintf のテストの実行ファイルと同じく、テストを走らせる前にプロセスの寿命の MTA を立てる。
9. When `areka-placement` のテストの実行ファイルがテストの途中で落ちたとき, the そのテストの実行ファイル shall 本体と wintf のテストの実行ファイルと同じ形の落ちの報告を標準エラーへ出す（3.8 と 3.9 は、本体の `emo2_boot/crash_report_test_support.rs`・wintf の `test_crash_report.rs` に並ぶ 3 つ目の写しで満たす）。
10. Where 揺れるテストを回す道具 `tools/load-flake.ps1` を新しい crate のテストに向ける場合, the 道具 shall `-Binary` で `areka-placement` のテストの実行ファイルを選べる。

### Requirement 4: examples の置き場

**Objective:** 開発者として、配置の中身を `#[path]` で取り込んでいる examples を、crate の依存で使う形にしたい。それにより examples がソースのファイルの置き場に縛られなくなるため。

#### Acceptance Criteria

1. The 分割作業 shall 配置だけを使う `window-placement.rs` を `areka-placement` の examples へ移し、配置を `#[path]` で取り込まず、crate の公開面で使う形にする。
2. The 分割作業 shall 配置と `emo2_boot` の両方を `#[path]` で読む `collision-probe.rs` を本体の examples に残し、配置の部分は `areka-placement` を crate の依存として使う形にする。
3. When examples を組んだとき, the 本体と `areka-placement` の examples shall `cargo check --workspace --all-targets` で警告なしに通る。

### Requirement 5: 配置を名指すパスの書き換え

**Objective:** 開発者として、配置の古いパスを名指す所を全部新しいパスへ直したい。それにより表・テスト・文書・道具が実在しないパスを指したまま黙って通ることがなくなるため。

#### Acceptance Criteria

1. The 分割作業 shall `crates/log-capture-kit/tests/` の表（`file_length_guard_test.rs` の 1,000 行の例外表・`temp_path_guard_test.rs`・`with_default_guard_test.rs`）にある配置のパスを、移した先のパス（本体のテスト専用のモジュールへ移すものは本体の側のパス）へ書き換え、理由の文の主語が変わるもの（`with_default_guard_test.rs` の「areka は当該フィーチャを有効にしない」ほか）は新しい crate に合わせる。
2. The 分割作業 shall ソースを文字列で読むテスト（`include_str!`・`CARGO_MANIFEST_DIR` を使う `measure_template_tests`・`zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests`・`spawn_zorder_pair_deferred_tests`・`transition_signoff_procedure_tests`・`zorder_property_deferral_tests`）を、次の原則で置き、移した先のファイルを読むようにする: 自分の crate のファイルは自分の crate のテストが読む。別の crate のファイルを読むのは、コードの依存と同じ向き（上の crate が自分の依存する下の crate を読む）に限り、下の crate が上の crate のファイルを読むことはしない。
3. The 分割作業 shall 要件 5.2 の原則に合わせて次のように置く: `measure_template_tests` は確かめていることで割り、採寸と表示の外形の一致と配置のファイル（`measure.rs`・`measure_first_frame.rs`・配置の examples の `window-placement.rs`）の走査を配置の crate に、本体のファイル（`emo2_boot/assets.rs`・本体のテストの支え・本体の examples）の走査を本体に置く。`spawn_zorder_pair_deferred_tests` は crate ごとに割り（wintf の `zorder_pair_deferred_vocabulary_tests.rs` と同じ形）、配置のファイルの名簿と配置の `src` の木の走査を配置に、`emo2_boot` のファイルの名簿と本体の `src` の木の走査を本体に置く。分岐の対応表 `zorder_group_branch_coverage_tests`・`zorder_inversion_branch_coverage_tests` は、1 つの分岐を覆うテストが crate をまたいで並ぶ表なので割らずに本体へ移し、「どの分岐も 1 本以上のテストで覆われている」の確かめを保つ。
4. The 本体の crate `areka` shall 下の crate のファイルを文字で読むとき、相対の道（`../../../wintf/src/…`）を散らさず、crate の名前と crate の中のパスを並べて書く 1 つの読み口（例: `crate_source!("wintf", "src/ecs/window/zorder_chain.rs")`）を通して読む。
5. If ソースを文字列で読むテストが読む先のファイルを見つけられない場合, then the そのテスト shall 黙って通らず赤になる。crate ごとに割ったソースの木を歩くテストは、合わせて移す前と同じ範囲（本体と配置の両方の `src`）を歩き、範囲が黙って狭まらない。
6. The 分割作業 shall 文書（`doc/COMPAT_ARCHITECTURE.md`・`doc/ukadoc-coverage/` の `briefing-assets.md` と網羅台帳 `ledger/*.toml` の根拠の場所・`.kiro/steering/structure.md`）にある配置のパスを、移した先のパスへ書き換える。
7. The 分割作業 shall ソースと道具の説明の中にある配置のパス（wintf・`pilot`・`ukadoc-survey`・本体の `emo2_boot`・`tools/perf/judge-followup.py` ほか）を、移した先のパスへ書き換える。
8. The 分割作業 shall `tools/perf/fixtures/` と `tools/perf/fixtures-loop/` の見本の `run.log` を手で書き換えず、見本を作る `tools/perf/fixtures/generate.py` を直して作り直す。
9. The 分割作業 shall `audit-placement-core`・`audit-placement-follow` の brief の「受け持つファイル」を、移した先のパスへ書き換える。
10. When 書き換えを終えたとき, the 分割作業 shall 古いパス `crates/areka/src/placement/`（と `src/placement/`）を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す（歴史の文書 `roadmap-history.md`・`.kiro/specs/completed/` と、新旧のパスの対応表を持つ `roadmap.md` は除く）。

### Requirement 6: ログの出所の名前

**Objective:** 開発者として、配置のログの出所の名前が新しい crate の名前に揃い、名前で読む所が全部それに追従してほしい。それにより `RUST_LOG` で配置の診断を開く・実機テストがログを拾う・道具がログを読むことが、移した後も前と同じく働くため。

#### Acceptance Criteria

1. The `areka-placement` クレート shall モジュールのパスから付く本番のログの出所を、新しい crate の名前（`areka_placement::…`）で出す。
2. The 分割作業 shall モジュールのパスを写した直書きの出所 `"areka::placement::diag"`（`diag.rs`）を、新しい出所の名前（`areka_placement::diag`）に合わせる。
3. When `RUST_LOG` に `areka_placement=debug` を与えたとき, the areka shall 配置の診断のログ（`diag` を含む）を出す。
4. The 分割作業 shall 語彙として付けた直書きの出所（`"areka::persist::*"` ほか、モジュールのパスに頼らないもの）を変えない。
5. The 分割作業 shall ログの出所を名前で読む所（`diag_tests.rs`・`follow_window_move_diag_tests.rs`・`measure_bake_count_tests.rs` と `measure_bake_log_tests.rs` の `"areka::placement::measure"`・`crates/areka/tests/window_link_real_run.rs` の `RUST_LOG`・wintf の `ecs/window_proc/window_pos_tests.rs` の `RUST_LOG`・wintf の `ecs/window_proc/lifecycle.rs` の説明・`tools/perf/judge-perf.py` の説明ほか）を、新しい出所の名前へ書き換える。
6. When 書き換えを終えたとき, the 分割作業 shall 古い出所の名前 `areka::placement` を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す（歴史の文書と、新旧のパスの対応表を持つ `roadmap.md` は除く）。
7. The 分割作業 shall 性能の判定の道具（`tools/perf/judge-perf.py`・`judge-followup.py` ほか）がログの出所の名前に頼って行を読んでいるかを確かめ、結果を記録する。
8. If 性能の判定の道具がログの出所の名前に頼って行を読んでいる場合, then the 分割作業 shall その読み方を新しい名前へ直し、道具の自分の確かめ（SelfTest）を回して緑を示す。

### Requirement 7: 依存の向きの規約

**Objective:** 開発者として、配置の中の依存の向きの規約の文が、新しい crate の中の実際の参照と一致していてほしい。それにより規約の文を読めば、どのモジュールがどれを引いてよいかが正しく分かるため。

#### Acceptance Criteria

1. The `areka-placement` クレート shall 根（`lib.rs`）に依存の向きの規約の文を持ち、その文が新しい crate の中の実際の参照と一致する。
2. The 分割作業 shall 規約の文と実際の参照の食い違い 5 件（`resolver.rs` が `tracing`・`wintf::ecs::DPI`・`config`・`follow::OffsetBase` を引く／`diag.rs` が `bevy_ecs::Entity`・`wintf::ecs::ZOrderPairStrategy` を引く／`config.rs` が `tracing` を引く／`measure.rs` が根の `PlacementError` を引き根と参照し合う／`spawn` と `follow` が互いに引き合う）を 1 件ずつ解消する。
3. Where 食い違いを参照の側で直すことが項目の置き場を移すだけで済む場合, the 分割作業 shall 参照を規約に合わせて直す。
4. Where 食い違いを参照の側で直すと項目の置き場を移すだけで済まない場合, the 分割作業 shall 参照を変えず、規約の文を実際の参照に合わせて書き直す。
5. The 分割作業 shall 食い違いの解消で配置の振る舞いを変えない。

### Requirement 8: 振る舞いを変えないことの検証

**Objective:** 開発者として、移した後も areka の振る舞いとテストの合否が前と同じであることを、決めた手順の証跡で確かめたい。それにより「移しただけ」が言葉でなく証跡で示されるため。

#### Acceptance Criteria

1. The 分割作業 shall 配置の振る舞い（窓の置き場・追従・保存と復元・DPI の追従・前後の並び・ログの中身）を変えない。直したくなった不具合は直さず、`audit-placement-core`・`audit-placement-follow` へ申し送りとして記録する。
2. When 着手のときと完了のとき, the 分割作業 shall `cargo test --workspace --no-fail-fast -- --list` を取り、`: test$` の行から名前の末尾（最後の `::` から後ろ）の多重集合を作り、前後で一致することを示す。
3. If 前後の多重集合が一致しない場合, then the 分割作業 shall 名前を改めたテストを対応表で 1 件ずつ説明し、説明できない差を残さない。
4. The 分割作業 shall `cargo check --workspace --all-targets` の警告を 0 件のまま保つ。
5. When 完了の前に, the 分割作業 shall `pwsh -NoProfile -File tools/test-all.ps1` が緑であることを示す。
6. When 完了の前に, the 分割作業 shall ログの出所を `RUST_LOG` で名指す実機テスト `crates/areka/tests/window_link_real_run.rs` を、新しい出所の名前で 1 回走らせて緑を示す。
7. The 分割作業 shall 実機で回すテストを上の 1 本だけにし、ほかは構造の証跡（テストの名前の一覧の比べ・警告・全体テスト・grep）で足りるとする。
8. The 分割作業 shall `THIRD-PARTY-NOTICES.md` を `cargo about generate` で作り直し、手で書かない。

## 要件の段で決めること（議題の候補・推奨つき）

brief が「要件の段で決める」とした点。上の要件は推奨案で書いた。要件の討議で開発者が別の案を選べば、該当する受入基準を書き換える。

1. **依存の向きの規約の食い違い 5 件**（要件 7）。推奨（設計の段で「移すだけで済むか」を確かめて確定）:
   - ① `resolver.rs`: `config` の型を引数に取るのが配置式の本体で、`OffsetBase`・`DPI` は出力の型＝移すだけでは済まない見込み → **規約の文を書き直す**（「`resolver` は純粋〔World と窓を触らない〕・`config` の型と `DPI`・`OffsetBase` の値の型と `tracing` だけを引く」の形）。`OffsetBase` を `resolver` 側へ移して `follow` から再公開する形が移すだけで済むなら、その部分は参照を直す。
   - ② `diag.rs`: `Entity`・`ZOrderPairStrategy` は記録の中身の型＝**規約の文を書き直す**（「`diag` は記録の型と `tracing` だけ・`World` に触らない」の形）。
   - ③ `config.rs` の `tracing`: **規約の文を書き直す**（全段で `tracing` を許す一文）。
   - ④ `measure.rs` ↔ 根の `PlacementError`: `PlacementError` を独立のモジュールへ移し、根から再公開する＝移すだけで済む → **参照を直す**。
   - ⑤ `spawn` ↔ `follow`: 同じ段で違反ではない → **規約の文に「同じ段どうしは引き合ってよい」を明記**。
2. **落ちの報告と MTA の初期化子の 3 つ目の写し**（要件 3.8・3.9）。推奨は **置く**（移したテストには GPU と COM を使うものがあり、初期化子が無いと移す前の実行ファイルで直した落ち〔件 3〕が新しい実行ファイルで戻る＝振る舞いを変えないため）。
3. **`tools/load-flake.ps1` の `-Binary` に新しい実行ファイルを足すか**（要件 3.10）。推奨は **足す**（`areka-placement-lib` を `-p areka-placement --lib` に結ぶ 1 行。揺れたときに新しい実行ファイルを狙えないと測り直しができない）。
4. **`capture_logs` の汎用の部分を `log-capture-kit` へ下ろすか**（要件 3.4）。推奨は **下ろさない**（`test-support` 機能で開けるだけにする。下ろすのは移すだけを超える）。
5. **数の取り直し**（議題ではなく申し送り）: brief の「`placement::test_support` を外の 194 ファイルが使う」は、11-06 の測りで約 40 ファイル（`placement::test_support` か `placement::{…test_support…}` を書くファイル・配置の中を除く）。配置を指す本体のファイルは 173（配置自身を除く）。設計の段で数え直す。
6. **brief の触るファイルの一覧に無かった所**（申し送り・同じウェーブの兄弟の触るファイルと重ならないことは 11-06 に確かめた）: `crates/log-capture-kit/tests/with_default_guard_test.rs`（表に配置のパス 3 つ）・説明の中のパス（wintf の `api_owner_chain_probe_tests.rs`・`ecs/window/window_band_pipeline_real_tests.rs`・`ecs/window/zorder_pair_maintain_always_on_top_tests.rs`・`runtime/tick_bridge.rs`、`crates/pilot/examples/pilot-dropfiles-on-wuc-window/main.rs`、`crates/ukadoc-survey/src/evidence/candidates.rs`、本体の `emo2_boot/hit_region.rs`・`emo2_boot/frame_transition_atomicity_tests.rs`、`tools/perf/judge-followup.py`）。

## 要件の討議で決めたこと

- **議題 3（ファイルを文字として読むテストの置き場）**（2026-11-06 開発者「参照場所があちこちに飛ぶのはソース管理できなくなる。あとで読む人が素直に読めるソースコードの書き方をしないとダメ。それをするのが『整理』」・承認）: 自分の crate のファイルは自分の crate のテストが読み、別の crate を読むのはコードの依存と同じ向き（上が下を読む）に限る。measure_template_tests・`spawn_zorder_pair_deferred_tests` は crate ごとに割り、分岐の対応表 2 本は本体へ移して crate の名前とパスで読む読み口 1 つにまとめる（要件 5.2〜5.5）。初めの推奨「配置に残して `../../areka/…` を読む」は、下の crate が上の crate を読む逆向きの参照になるので退けた。
