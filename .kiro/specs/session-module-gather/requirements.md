# Requirements Document

## Project Description (Input)

- **困っている人**: areka の開発者（本体 `crates/areka` を直し、見直し〔ponytail の audit〕を受け持ちごとに回す人）。
- **今の状態**: 起動・終了・切替の層（ゴーストを降ろして起こし直す単位・起動するゴーストとバルーンの判断・構成の入力・終わり方・切替と読み直し）は、本体で一番大きく残る塊なのに、`crates/areka/src/` の直下にばらばらに置かれ、`main.rs` の中にもその一部（`on_boot_ok`・`settle_session_mark`・`restore_merged_placements` ほか）がある。`ghost_session.rs` の `register_systems` は、本体だけのモジュールの系も登録している。本体は `main.rs` の出し直し `use areka_input as input_events;` で入力の呼び手を通したまま（`input-crate-split` の申し送り）。
- **変えたいこと**: crate に出す前に、この層を本体の中の 1 つのモジュール `crates/areka/src/session/` に集める。テストも一緒に移す。`main.rs` は `fn main` と最上位の段取りだけにし、本体だけのモジュールの系の登録は `main.rs` から呼ぶ。出し直しを外して呼び手を `areka_input::…` へ書き換える。参照の向きを逆にすることと crate を作ることは、次の `session-crate-split` で行う。振る舞いは何も変えない。

（出どころ: `.kiro/specs/session-module-gather/brief.md`〔`input-crate-split` からの申し送りと、最後の「2026-11-21 棚卸㊶の再測定」の節を含む〕。分け方・順番・共通の約束・新旧のパスの対応表の正本は `.kiro/steering/roadmap.md`「テーマ別の決めごと」の「本体の crate 分け」。同じウェーブの約束の正本は同じ roadmap の「ウェーブ編成」の C24 の行。）

## Introduction

本仕様は、「本体の crate 分け」6 本の 5 本目として、本体の bin crate の中で、起動・終了・切替の層を `crates/areka/src/session/` に集める。crate は作らず、参照の向きも変えない。移すだけで、中身は直さない。前例は `runtime-module-untangle`（2026-11-13・本体の中の移動）で、やり方をそれにそろえる。

受益者は areka の開発者であり、この仕様での「目に見える成果」は次のとおり開発者の側から定める。

- 起動・終了・切替の層の本番とテストが `crates/areka/src/session/` にまとまり、次の `session-crate-split` がこの 1 つのフォルダを crate へ移せる形になっている。
- `main.rs` は `fn main` と最上位の段取りだけで、本体だけのモジュールの系の登録は `main.rs` から呼ばれている。
- `session/` の外への参照の一覧（次の spec が向きを逆にする約 30 件）が、設計か研究の記録に残っている。
- 動く ukagaka のアプリとしての振る舞い（起動・切替・読み直し・終了・OS のセッションの終了・告知・起動中の印・記憶の書き込み・ログの中身〔出所の名前を除く〕・MCP の `get_log` の分け方）が前と同じである。
- テストの数・警告・全体テスト・実機テストが、移す前と同じ合否を示す。

数（行数・ファイル数・参照の数・行番号）は brief の再測定（main `31ac3eaf`）と 2026-11-21 の作業の木（main `f6405e09`）の測りによる参考値で、設計の段で測り直す。

## Boundary Context

- **In scope**:
  - src 直下の候補（`ghost_session*`〔`_runtime`・`_scope_rect` を含む〕・`boot_resolve*`・`boot_config*`・`app_exit*`〔`app_exit_escape_tests.rs` を含む〕・`session_end*`・`exit_wait*`・`write_failure*`・`readonly*`・`alert*`・`log_history*`・`shell_balloon_switch_session_*`・`ghost_session.rs` が `#[path]` で宣言する `boot_shell_tests.rs`）の本番とテストを `session/` の下へ移すこと。
  - `main.rs` の項目（`MarkInputs`・`Teardown`・`settle_session_mark`〔`session_mark_verdict`・`MarkVerdict` を含む〕・`restore_merged_placements`・`on_boot_ok`〔`insert_persist_wiring`・`record_last_used_at_boot` を含む〕・`record_last_used`・`boot_monitor_snapshot`）と、それを確かめる `main_*_tests.rs` を `session/` の下へ移すこと。`main.rs` の `pub(crate) use boot_config::{…}` の出し直しを外すこと。
  - roots の芯（`roots/` の `mod`・`resolve`・`report`・`user_root` とテスト）・readme（`readme.rs` と `readme/`）・cli の型 2 つ（`BootRequest`・`ModulePolicy`）を `session/` の下へ移すこと。
  - `register_systems` の本体だけのモジュールの系の登録を `main.rs` の呼び手へ移すこと。
  - `main.rs` の出し直し `use areka_input as input_events;` とその上の説明を外し、本体の呼び手（26 ファイル）を `areka_input::…` へ書き換えること。
  - ログの出所の名前の付け替えと、名前で読む所（`log_history` の決まり・`RUST_LOG` の文字列・real-run-kit の名指しの決まり・実機テスト・文書）の書き換え。
  - 集めるモジュールを名指すパスの書き換え（本体の約 186 ファイル・ほかの crate の説明・`tools/`・`doc/`・網羅台帳・`.kiro/steering/structure.md`・`tech.md`）と、`audit-areka-app`・`audit-areka-install-update` の brief の「受け持つファイル」の書き換え。
  - `session/` の外への参照の一覧の記録と、検証。
- **Out of scope**:
  - 参照の向きを逆にすること・crate の新設（`session-crate-split`）。
  - ゴーストの単位を複数にすること（`multi-ghost-runtime`）。
  - 診断用のデモ 4 本（`shiori_host`・`shiori_session`・`shiori_demo`・`reference_brain`）を消すこと（`audit-areka-app`）。デモと `png_image`・`thread_roles`・`tick_gate_config`・`perf_thread_report`・`smoke_exit`・`install`・`update`・`mcp`・`menu`・`file_drop`・`offline_dump` と、`roots/` の `edit`・`desk`・`pick`、`cli/` の残り（読み方・説明・待ち受け）は本体に残す。
  - 振る舞いの変更。直したくなった不具合と古い説明は直さず、範囲を受け持つ audit へ申し送る。登録の順と起動の段取りの順は変えない。
  - 同じウェーブの約束で触らないファイル（要件 9）。
  - ほかの brief（パスに触れている約 70 本）の書き換え（roadmap の新旧のパスの対応表で読み替える）。歴史の文書（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/`）と日付つきの写しの中の古いパス。
- **Adjacent expectations**:
  - 上流: `input-crate-split`（2026-11-21 着地）。入力は `crates/areka-input`（出所は `areka_input`）にあり、強制退避の口 `areka_input::hooks::InputHooks` は `register_systems` の中の `session::hooks::install` → `app_exit::install_input_hooks` の順で据わる（固定のテスト `register_systems_installs_the_escape_hook`）。
  - 同じウェーブ C24 の相手: `window-resetwindowpos-hold`・`audit-areka-input-menu`・`test-crash-report-merge`。約束の中身は要件 9。並べ方そのものは roadmap が持つ約束で、この仕様の要件ではない。
  - 同じウェーブに置けない相手: `session-crate-split`（待つ側）・`audit-emo2-boot-switch`・`audit-areka-app`・`audit-areka-install-update`・`multi-ghost-runtime`・`balloon-text-visual-tree`。
  - 下流: `session-crate-split`（この仕様の着地と、要件 8 の一覧を待つ）。

## Requirements

### Requirement 1: src 直下のファイルとテストを `session/` へ集める

**Objective:** 開発者として、src 直下にばらばらに置かれた起動・終了・切替の層のファイルとテストを `session/` の下へ集めたい。それにより層の境目がフォルダの境目で見え、次の spec がこのフォルダを crate へ移せるため。

#### Acceptance Criteria

1. The 集め直し作業 shall In scope に挙げた src 直下の候補の本番とテストのファイル（名前が候補に当たらなくても候補が `#[path]` で宣言するテスト＝`boot_shell_tests.rs` を含む）を、git が移動として追える形（履歴を残す移動）で `crates/areka/src/session/` の下へ移す。
2. The 集め直し作業 shall 移したファイルの中身を書き換えず、パス（`mod` の宣言・`#[path]`・`use`・`crate::…` のパス・説明の中のパス）だけを直す。
3. When 移動を終えたとき, the 本体の crate `areka` shall src 直下に、移した候補のファイルを持たない。
4. The 集め直し作業 shall 移した項目の可視性を、呼び手から届くのに要る分だけ（`pub(crate)` まで）にし、`pub` を足さない。
5. The 集め直し作業 shall 移したファイルを含む本体のファイルを、1 ファイル 1,000 行の上限の中に収め、上限越えの例外表の項目を増やさない（2026-11-21 の測りで集めるパスは表に 0 件）。
6. The 集め直し作業 shall 移したテストを、確かめる相手のファイルと同じフォルダの兄弟（`*_tests.rs`）として置き、本体のテストの実行ファイルで前と同じく走らせる。

### Requirement 2: `main.rs` の項目の移動

**Objective:** 開発者として、`main.rs` の中にある起動・終了の層の項目を `session/` へ移し、`main.rs` を `fn main` と最上位の段取りだけにしたい。それにより入口のファイルが段取りの順だけを読める大きさになり、層の項目が層の中にそろうため。

#### Acceptance Criteria

1. The 集め直し作業 shall `main.rs` の `MarkInputs`・`Teardown`・`settle_session_mark`（`session_mark_verdict`・`MarkVerdict` を含む）・`restore_merged_placements`・`on_boot_ok`（`insert_persist_wiring`・`record_last_used_at_boot` を含む）・`record_last_used`・`boot_monitor_snapshot` を、`session/` の下へ移す。
2. When 移動を終えたとき, the `main.rs` shall `fn main`・`finish_after_run`・`install_boot_context`・`first_boot_origin`・`boot_first_ghost` と、モジュールの宣言と最上位の段取りだけを持つ（brief の見込みは約 470 行・非空行）。（**仮定**: `after_run`・`AfterRun` は `main.rs` に残す。OPEN QUESTION 1）
3. The 集め直し作業 shall `main.rs` の `pub(crate) use boot_config::{ConfigInputs, default_app_profile_dir, default_helper_exe_path, ghost_boot_options, is_benign_boot_error}` の出し直しを外し、呼び手が移した先のパスを指すようにする。（**仮定**: 出し直しは残さない。OPEN QUESTION 4）
4. The 集め直し作業 shall 移した項目を確かめるテスト（`main_restore_seam_tests.rs`・`main_persist_wiring_seam_tests.rs`・`main_monitor_snapshot_seam_tests.rs`・`main_session_mark*_tests.rs`・`main_config_input_tests.rs`・`main_ghost_wiring_tests.rs`・`main_module_policy_tests.rs` ほか）を、確かめる相手と一緒に移すか `main.rs` に残すかを 1 本ずつ決め、どちらにしたかを記録する。`main.rs` に残る項目（`finish_after_run` ほか）を確かめるテストは `main.rs` に残す。
5. The areka shall 起動の段取り（系の登録・口の据え付け・起動の文脈・最初のゴーストの起動・`run()`・`run()` の後の告知と印の始末・降ろす処理）を、前と同じ順で行う。

### Requirement 3: roots の芯・readme・cli の型 2 つ

**Objective:** 開発者として、集める層が使う roots の芯・readme・cli の型 2 つを `session/` の下へ移したい。それにより集めた層が、本体に残る install・menu ほかに頼らずに済む形へ近づくため。

#### Acceptance Criteria

1. The 集め直し作業 shall `roots/` の `mod.rs`・`resolve.rs`・`report.rs`・`user_root.rs` とそれぞれのテストを `session/` の下へ移し、`edit.rs`・`desk.rs`・`pick.rs` とそれぞれのテストは本体に残す。
2. The 集め直し作業 shall `readme.rs` と `readme/` の下のファイル（本番とテスト）を `session/` の下へ移す。
3. The 集め直し作業 shall `cli/mod.rs` の `BootRequest` と `ModulePolicy` を `session/` の下へ移し、`cli` がそれを引く向き（`cli` → `session`）にする。
4. The areka shall 起動オプションの読み方（位置の引数・旗・`--readonly`・`--install`・`--offline-dump`・SHIORI の読み方の指定ほか）の結果と、根の並びの決め方・説明書を開く振る舞いを変えない。
5. When 移動を終えたとき, the 集め直し作業 shall 本体に残る `roots/` の `edit`・`desk`・`pick` と `session/` の roots の芯が、前と同じ項目を同じ意味で使っていることを、テストの合否で示す。

### Requirement 4: 系の登録の移し替え

**Objective:** 開発者として、`register_systems` が登録している本体だけのモジュールの系を、`main.rs` の呼び手から登録したい。それにより集めた層が本体だけのモジュールを名指さずに済み、次の spec で向きを逆にする件数が減るため。

#### Acceptance Criteria

1. The 集め直し作業 shall `ghost_session.rs` の `register_systems` が呼ぶ本体だけのモジュールの系の登録（brief の再測定で 11 本＝`readme`・`areka_input` の 6 本〔`user_break`・`balloon`・`shell_box_handler`・`choice_drain`・`char_watch`・`balloon_wheel`〕・`menu`・`install`・`update`・`mcp`）を、`main.rs` の呼び手（今は `main.rs:242`）へ移す。（**仮定**: 11 本とも移す。OPEN QUESTION 2）
2. The areka shall 系の登録の呼び出しの順（`session::hooks::install`・`app_exit::install_input_hooks`・`register_frame_system`・`wire_kanade_stop`・`write_failure::install`・各モジュールの登録・`FrameFinalize` への足し込み・`wire_zorder_pair`）を、全体として前と同じにする。
3. The areka shall 強制退避の口を、前と同じく `session::hooks::install` のすぐ後に `app_exit::install_input_hooks` で据え、固定のテスト `register_systems_installs_the_escape_hook` を前と同じ合否で通す。
4. The areka shall 系の登録を、前と同じくプロセスに 1 回だけ行う。
5. When LogSink の起動のとき, the areka shall 前と同じ系を登録し、状態の無い系は前と同じく無操作で戻る。

### Requirement 5: 入力の出し直しを外す

**Objective:** 開発者として、`main.rs` の出し直し `use areka_input as input_events;` を外し、本体の呼び手を入力の crate の名前で直に書きたい。それにより本体の中に実在しないモジュールの名前が残らず、次の spec で集めた層が入力の crate を直に使えるため。

#### Acceptance Criteria

1. The 集め直し作業 shall `crates/areka/src/main.rs` の `use areka_input as input_events;` と、その上の古くなった説明（今は 59〜60 行の 2 行）を外し、出し直しを名指す `crates/areka/Cargo.toml:44` の説明を出し直しの無い形へ直す。
2. The 集め直し作業 shall 本体の `crate::input_events::…` の呼び手（brief の再測定で 26 ファイル・`runtime_tests/` の 8 本と `menu/` を含む）を、`areka_input::…` へ書き換え、呼ぶ項目を変えない。
3. When 書き換えを終えたとき, the 集め直し作業 shall `crates/areka/` の中に、モジュールのパスとしての `input_events`（`crate::input_events`・`input_events::`）が 0 件であることを grep の結果で示す。説明の中で入力のモジュールを `input_events` の語で指す所（2026-11-21 の測りで `ghost_session.rs:276`・`menu/mod.rs:182`）は `areka_input` へ直す。もとから実在しないファイルを指す `app_exit_tests.rs:182` の `input_events_tests.rs` は直さず、要件 10.1 の申し送りに載せる。
4. The 集め直し作業 shall 本体の `runtime_tests/mod.rs` を書き換えずに 5.2 を済ませる（要件 9.3）。

### Requirement 6: 集めるモジュールを名指すパスと受け持ちの書き換え

**Objective:** 開発者として、集めたモジュールの古いパスを名指す所を、移した先のパスへ直したい。それにより説明・文書・台帳が実在しないパスを指したまま黙って通ることがなくなるため。

#### Acceptance Criteria

1. The 集め直し作業 shall 本体の中で集めるモジュールを名指すファイル（brief の再測定で 186 本・`runtime_tests/` の 26 本〔`mod.rs` を除く〕・`file_drop.rs`・`file_drop_wiring_tests.rs`・`menu/shortcut_tests.rs` を含む）のパスを、移した先へ書き換える。
2. The 集め直し作業 shall ソースを読むテスト（`include_str!` と `CARGO_MANIFEST_DIR` で読むもの。`runtime_tests/zorder_wiring_tests.rs` の `../ghost_session.rs`・`../main.rs`、`readonly_tests.rs` の較正と絞り込み〔`:82`・`:92`・`:153`〕、`session_end_deadline_tests.rs` の表〔`"areka/ghost_session.rs"`・`"areka/session_end.rs"`〕、`readme_tests.rs:424` の `"areka/readme/os_port.rs"` ほか）を、移した先のパスと、移した後に項目がある所を読むように直し、前と同じ事実を確かめさせる。絞り込みが移した後に黙って空になる（赤にならずに確かめる力を失う）形を残さない。`main.rs` を読む較正の写し（`session_end_sync_send_tests.rs:437`・`:445`）は、`main.rs` が残るので直さない。
3. The 集め直し作業 shall 本体の外の crate の説明に残る本体のパス（`crates/areka-runtime/src/placement_tests/transition_judge_offset_signoff_tests.rs:57` の `boot_config.rs`・`crates/areka-kanade/src/schedule/script_log.rs:16` の `log_history.rs`・`crates/real-run-kit/` の説明・`crates/pilot/examples/pilot-windows-rs-next/REWRITE_TABLE.md`・`crates/areka/tests/emo2_real_run.rs:58` の `default_helper_exe_path()` の置き場の説明ほか）を、移した先のパスへ書き換え、動きを変えない。
4. The 集め直し作業 shall `tools/` の中の説明のパス（`tools/package.ps1` の目印の出どころ・`tools/perf/judge-perf.py`）を、移した先へ書き換え、道具が字面で読む文（ログの目印の本文ほか）は変えない。
5. The 集め直し作業 shall 文書の中の集めるモジュールのパスを、移した先へ書き換える: `doc/COMPAT_ARCHITECTURE.md`（§8 の既にある行のパスだけ）・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所（2026-11-21 の測りで約 33 か所）・`doc/ukadoc-coverage/briefing-assets.md`・`doc/README.md`・`doc/click-through.md`・`doc/testing.md`・`doc/ssp-mcp/log-convention.md`・`doc/pasta-real-run.md`・`doc/ideas/` の下の文書（`doc/ideas/novel-redesign-2026-11-20/` を含む）。残るファイル `main.rs` の名で移す項目を指す行（2026-11-21 の測りで `doc/COMPAT_ARCHITECTURE.md:189` の `restore_merged_placements`・`:318`・`:319` の `session_mark_verdict`）は、ファイル名の grep に当たらないので項目の名で引いて 1 件ずつ直す（同じ行の `after_run` ほか `main.rs` に残る項目は直さない）。網羅台帳は別々の行だけを書き換え、`doc/ukadoc-coverage/report/` は手で直さず作り直す。
6. The 集め直し作業 shall `.kiro/steering/structure.md` の本体の節（src 直下の起動と終了のファイル・`session/`・`roots/`・`cli/`・`readme`）と、`.kiro/steering/tech.md`・`.kiro/steering/logging.md`（2026-11-21 の測りで `:103` の `log_history.rs`）の本体のパスを、移した先へ合わせる。
7. The 集め直し作業 shall `audit-areka-app` と `audit-areka-install-update` の brief の「受け持つファイル」を、移した先のパスへ書き換える。（**仮定**: 受け持ちの持ち主は変えず、パスだけを書き換える＝roots の芯と readme は `audit-areka-install-update` のまま。OPEN QUESTION 3）
8. When 書き換えを終えたとき, the 集め直し作業 shall 集めたモジュールのディレクトリを付けた古いパス（`crates/areka/src/ghost_session.rs`・`areka/src/ghost_session.rs`・`src/ghost_session.rs` の形ほか）を、`crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md`・`.kiro/steering/logging.md` で grep して 0 件であることを示す。ディレクトリを付けない素のファイル名（`ghost_session.rs` だけの書き方。2026-11-21 の測りで触れない crate に 9 か所・`areka-ghost`・`wintf` に 2 か所）は、6.9 のモジュールの名前と同じく移した後も読めるものとして当たりにせず、書き換えない。除くのは、歴史の文書（`roadmap-history.md`・`.kiro/specs/completed/`）・新旧のパスの対応表を持つ `roadmap.md`・日付つきの 1 枚の写し（`doc/roadmap-overview-2026-10-16.html` の形。`doc/ideas/novel-redesign-2026-11-20/` は当たらず書き換える側）・同じウェーブの約束で触らない所（`crates/areka-input/src/user_break.rs:201`〔`audit-areka-input-menu` がパスを書かない形に直す〕）だけで、除いた所は一覧で残す。
9. The 集め直し作業 shall モジュールの名前だけで本体の層を指す説明（`crates/areka-runtime/src/lib.rs`・`crates/areka-placement/src/lib.rs` の「本体の `ghost_session`」・本体の `runtime_tests/mod.rs` の 25 行ほか）を、移した後も読めるものとして 6.8 の当たりにせず、書き換えない。

### Requirement 7: ログの出所の名前

**Objective:** 開発者として、集めたモジュールのログの出所の名前が移した先のモジュールのパスにそろい、名前で読む所が全部それに追従してほしい。それにより `RUST_LOG` で診断を開く・実機テストがログを拾う・MCP の `get_log` がログを分けることが、移した後も前と同じく働くため。

#### Acceptance Criteria

1. The areka shall 集めたモジュールの本番のログを、モジュールのパスから付く出所（`areka::session::…`）で出す。（**仮定**: モジュールの名前は今と同じ＝`areka::session::boot_config`・`areka::session::ghost_session`・`areka::session::app_exit`・`areka::session::readme` ほか。OPEN QUESTION 5）
2. The 集め直し作業 shall 直書きの出所を、次の決まりで 1 件ずつ扱う。モジュールのパスを写したもの（`"areka::boot_config"`・`"areka::boot_resolve"`・`"areka::ghost_session"`・`"areka::alert"`・`"areka::readme"`・`"areka::readme::opener"`・`"areka::app_exit"` ほか、集めるモジュールの名前を写したもの）は新しい出所へ合わせる。語彙として付けたもの（`"areka::persist::*"`・`"areka::log::*"`・`"areka::perf"`）と、既に `session/` にあるもの（`"areka::session::ghost_switch"`・`"areka::session::reload"`）と、本体の根の `"areka"` は変えない。どの直書きをどちらにしたかは一覧で記録する。
3. The 集め直し作業 shall ログの出所を名前で読む所を、新しい出所の名前へ書き換える: `log_history` の決まりの表（`RULES`・`UNKEPT_TARGETS`）とそのテスト・`crates/real-run-kit/src/launch.rs` の `RELIED_TARGETS` と `launch_tests.rs`・`close_tests.rs`・`bin/real-run-fake-areka.rs`・`crates/areka/tests/emo2_mcp_real_run.rs` の `rust_log` と標準出力の前置き（`"areka::boot_config: "`）と説明・`doc/testing.md`・`doc/ssp-mcp/log-convention.md`・`doc/pasta-real-run.md` の `RUST_LOG` と説明。
4. The areka shall MCP の `get_log` の 5 種別への分け方で、移す前に各出所の行が当たっていた種別（`areka::boot_config`・`areka::boot_resolve`・`areka::ghost_session`・`areka::readme` の下は status・どの決まりにも当たらない `areka::alert`・`areka::app_exit` ほかは残さない）を、移した後の出所の行でも同じにする。
5. If 移す項目の info の行の出所が移すことで本体の根 `areka`（完全一致で status に当たる）から外れる場合, then the 集め直し作業 shall その行の分け方を前と同じに保つ手当てを設計の段で決める。（2026-11-21 の測りで、`main.rs` から移す項目の info の行は 0 件。`after_run` の `session_end_already_handled` は `main.rs` に残す仮定のため当たらない）
6. When `RUST_LOG` に新しい出所の名前（例 `areka::session::boot_config=debug`）を与えたとき, the areka shall 移す前に古い出所の名前で出ていたものと同じだけのログを出す。
7. When real-run-kit の窓口へ info より絞った記録の段を渡したとき, the real-run-kit shall 前と同じく窓口が頼る行の出所（`areka_mcp::server` と、新しい出所の名前の終了の行）を info で足す。
8. When 書き換えを終えたとき, the 集め直し作業 shall 古い出所の名前（`areka::ghost_session`・`areka::boot_config`・`areka::boot_resolve`・`areka::app_exit`・`areka::alert`・`areka::readme`・`areka::session_end`・`areka::exit_wait`・`areka::write_failure`・`areka::readonly`・`areka::log_history`・`areka::roots`〔芯の分〕ほか）を、`crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す（歴史の文書と、新旧のパスの対応表を持つ `roadmap.md` は除く。本体に残る `roots/` の `edit`・`desk`・`pick` の出所は当たりにしない）。
9. The 集め直し作業 shall ログの本文（メッセージの文字列と構造化の欄の名前と値。`[main]` の前置きを含む）を変えない。

### Requirement 8: `session/` の外への参照の一覧

**Objective:** 開発者として、集め終えた `session/` が本体の外のモジュール（`install`・`update`・`menu`・`mcp`・`file_drop`・`smoke_exit` ほか）を名指す所の一覧を、記録として残したい。それにより次の `session-crate-split` が向きを逆にする件数と場所を測り直さずに着手できるため。

#### Acceptance Criteria

1. When 集め直しを終えたとき, the 集め直し作業 shall `session/` の本番のコードが `session/` の外の本体のモジュールを参照している所を、ファイルと行と参照する項目の一覧として、この仕様の設計か研究の記録に残す（brief の見込みは約 30 件）。
2. The 集め直し作業 shall 8.1 の一覧に、テストのコードだけが参照している所を本番と分けて載せる。
3. The 集め直し作業 shall 8.1 の参照の向きを、この仕様では逆にしない。

### Requirement 9: 同じウェーブの約束で触らない所

**Objective:** 開発者として、同じウェーブ C24 のほかの spec が持つファイルに触れずにこの仕様を終えたい。それにより 4 本が互いに競合せずに並んで着地できるため。

#### Acceptance Criteria

1. The 集め直し作業 shall `crates/areka-runtime/` の中で、`src/placement_tests/transition_judge_offset_signoff_tests.rs:57` の説明のパス 1 か所のほかを変えない（`lib.rs` も変えない）。
2. The 集め直し作業 shall `crates/areka-placement/`・`crates/areka-input/`・`crates/log-capture-kit/` を変えない（`crates/areka-input/src/user_break.rs:201` の `crates/areka/src/readme.rs` は `audit-areka-input-menu` がパスを書かない形に直す）。
3. The 集め直し作業 shall 本体の `crates/areka/src/runtime_tests/mod.rs` を変えず、そこに集めるモジュールを名指す行を足さない。
4. The 集め直し作業 shall `doc/COMPAT_ARCHITECTURE.md` の §8 で、既にある行のパスだけを書き換え、行を足さない・消さない。
5. The 集め直し作業 shall `Cargo.lock`・ワークスペースの根の `Cargo.toml`・`THIRD-PARTY-NOTICES.md` を変えず、新しい依存を足さない。
6. If 9.1〜9.5 のファイルに触らなければ済まない所が見つかった場合, then the 集め直し作業 shall そのファイルを変えずに通す形を設計で選び、それも無理なら、持ち主の spec のセッションへ相談したうえで開発者へ報告する。
7. When この仕様の完了の申し送りを書くとき, the 集め直し作業 shall `crates/areka-input/src/user_break.rs:201` が着地の時点でまだ `crates/areka/src/readme.rs` を名指しているなら、`audit-areka-input-menu` か後の見直しの brief への申し送りとして記録する。

### Requirement 10: 振る舞いを変えないことの検証

**Objective:** 開発者として、集めた後も areka の振る舞いとテストの合否が前と同じであることを、決めた手順の証跡で確かめたい。それにより「移しただけ」が言葉でなく証跡で示されるため。

#### Acceptance Criteria

1. The 集め直し作業 shall 起動・切替・読み直し・終了・OS のセッションの終了・告知・起動中の印・記憶の書き込み・ログの中身の振る舞いを変えない。直したくなった不具合と、この仕様の移動と関係なくもとから古い説明（2026-11-21 の測りで `doc/click-through.md:3`・`:136`・`tools/perf/invoke-perf-run.ps1:29`・`app_exit_tests.rs:182`）は直さず、範囲を受け持つ audit（`audit-areka-app`・`audit-areka-install-update`）へ申し送りとして記録する。
2. When 着手のときと完了のとき, the 集め直し作業 shall `cargo test --workspace --no-fail-fast -- --list` を取り、`: test$` の行から名前の末尾（最後の `::` から後ろ）の多重集合を作り、前後で一致することを示す。
3. If 前後の多重集合が一致しない場合, then the 集め直し作業 shall 名前を改めたテストと新しく足したテストを対応表で 1 件ずつ説明し、説明できない差を残さない。
4. The 集め直し作業 shall `cargo check --workspace --all-targets` の警告を 0 件のまま保つ。
5. When 完了の前に, the 集め直し作業 shall `pwsh -NoProfile -File tools/test-all.ps1` が緑であることを示す。
6. When 完了の前に, the 集め直し作業 shall ログの出所を名前で読む実機テスト 2 つ（real-run-kit を使い `areka::app_exit` を名指しの決まりで足す `crates/areka/tests/emo2_mcp_real_run.rs`・`crates/areka/tests/smoke_boot_loop_exit.rs`）を、新しい出所の名前で 1 回ずつ走らせて緑を示す。
7. The 集め直し作業 shall 実機で回すテストを 10.6 の 2 つだけにし、ほかは構造の証跡（テストの名前の一覧の比べ・警告・全体テスト・grep・決定論のテスト）で足りるとする（向きを逆にした道はこの仕様には無い）。
8. The 集め直し作業 shall MCP の `get_log` の 5 種別の分け方が前後で同じこと（7.4）を、`log_history` のテストで示す。

## 設計の段への申し送り

1. **数の取り直し**（議題ではなく申し送り）: 移すファイルの一覧と行数・`main.rs` の移す項目の行の範囲（brief の再測定で 527〜947 行）・`register_systems` の行（`ghost_session.rs:43`）と本体だけの系の本数・名指す本体のファイル（186 本）・`crate::input_events` の呼び手（26 ファイル）・網羅台帳の根拠の場所（約 33 か所）・`doc/COMPAT_ARCHITECTURE.md` §8 のパスの行（2026-11-21 の測りで `main.rs` を含めて 34 行）・直書きの出所の一覧を、設計の段で数え直す。
2. **roadmap の C24 の行の誤記**: ⑵ の名指すコメントの `crates/areka-kanade/src/script_log.rs:16` は、実在は `crates/areka-kanade/src/schedule/script_log.rs:16`（brief の再測定の書き方が正しい）。
3. **roadmap の C24 の行の触るファイルの一覧に無かった所**（2026-11-21 の作業の木の測り・同じウェーブの兄弟の触るファイルと重ならないことは設計の段で確かめる）: `tools/package.ps1`・`tools/perf/judge-perf.py`（説明だけ）・`doc/click-through.md`・`doc/README.md`・`doc/ukadoc-coverage/briefing-assets.md`・`doc/ideas/novel-redesign-2026-11-20/` の 5 ファイル・`.kiro/steering/tech.md`・`crates/pilot/examples/pilot-windows-rs-next/REWRITE_TABLE.md`。
4. **上限越えの例外表**: 2026-11-21 の測りで `crates/log-capture-kit/tests/file_length_guard_test.rs` の表と `temp_path_guard_test.rs` の表に集めるパスは 0 件＝`log-capture-kit` に触らずに済む見込み（9.2）。
5. **`main.rs` の info の行**: 移す項目のうち info を出すのは無い（`after_run` の `session_end_already_handled` は出所 `areka` で status に当たる。`after_run` を移すなら 7.5 の手当てが要る）。
6. **登録の順**: `readme` の登録は今 `session::ghost_switch` ほかの切替の登録より前にある。11 本を `main.rs` へ移しても全体の順を保つ形（`register_systems` を前後に分けるなど）は設計の段で決める（4.2）。

## OPEN QUESTIONS（要件の討議で決めること）

1. **`after_run`・`AfterRun` の置き場**: brief の項目の一覧には無いが、再測定の「出せる項目は 527〜947 行」の範囲に `after_run` が入る。`after_run` は `fn main` から呼ばれ、info の行（出所 `areka`＝status）を持つ。**仮定**: `main.rs` に残す（2.2）。移すなら 7.5 の手当てが要る。
2. **`main.rs` へ移す系の登録の範囲**: brief の再測定は「本体だけの系は 11 本」と数える。しかし `readme` はこの仕様で `session/` へ移り、入力は既に本体の下の crate `areka-input`（後の `areka-session` が引ける向き）にある。本当に本体に残るのは `menu`・`install`・`update`・`mcp` の 4 本。**仮定**: brief の字面どおり 11 本を移し、全体の順を保つ（4.1・4.2）。4 本だけにする案もある（移す行が減り、順を保つ手当ても小さくなる）。
3. **roots の芯と readme の受け持ち**: `audit-areka-install-update` が今 `roots*`・`readme*` を受け持つ。`session/` へ移した後も同じ audit が受け持つか、`session/` を受け持つ `audit-areka-app` へ寄せるか。**仮定**: 持ち主は変えずパスだけ書き換える（6.7）。
4. **`boot_config` の出し直し**: `main.rs` の `pub(crate) use boot_config::{…}` を外して呼び手を移した先へ向けるか、`main.rs` に出し直しを残すか。**仮定**: 外す（2.3・brief の「出し直しているだけ」の読み）。
5. **`session/` の下のモジュールの名前**: 移したモジュールの名前を今と同じにするか（`session::boot_config`・`session::ghost_session`・`session::roots`・`session::readme` ほか。本体に残る `roots/` の `edit`・`desk`・`pick` と名前が並ぶ）。名前は `RUST_LOG` で開く出所の名前として開発者の目に見える。**仮定**: 今と同じ名前（7.1）。
