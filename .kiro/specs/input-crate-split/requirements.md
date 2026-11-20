# Requirements Document

## Project Description (Input)

- **困っている人**: areka の開発者（本体 `crates/areka` を直し、見直し〔ponytail の audit〕を受け持ちごとに回す人）。
- **今の状態**: 入力の出来事（ポインタからマウスのイベント・選択肢・ドラッグ・中断・ホイール）は本体の中で一番まとまった持ち場の 1 つなのに、本体と一緒に組み直され、テストも本体の 1 つの実行ファイルに入っている。`crates/areka/src/input_events/` は 50 ファイル（本番 3,932 行・テスト 9,319 行。main `d6054e4d` の測り・2026-11-20 main `67e0c70e` でファイル数は同じ）。本体を引くのは本番の 3 ファイル（`mod.rs` の `app_exit::{ExitOrigin, quit_app}`・`shortcut.rs` の menu・`file_drop.rs` の install と `ghost_session::GhostSlot`）だけ。後の session（起動・終了・切替の層）が input を呼ぶので、input は session より先に出す。
- **変えたいこと**: input（`shortcut.rs`・`file_drop.rs` を除く）の本番とテストを新しい crate `crates/areka-input/`（lib・`publish = false`・`areka-runtime` と `areka-placement` に依存）へ移す。`shortcut.rs` は本体の menu へ、`file_drop.rs` は本体に残す。Ctrl+左ダブルクリックの強制退避が呼ぶ `quit_app` は、本体へ向かう呼び出しの向きを逆にする。振る舞いは何も変えない。

（出どころ: `.kiro/specs/input-crate-split/brief.md`〔最後の「2026-11-20 棚卸㊵の再測定」の節を含む〕。分け方・順番・共通の約束・新旧のパスの対応表の正本は `.kiro/steering/roadmap.md`「テーマ別の決めごと」の「本体の crate 分け」。同じウェーブの約束の正本は同じ roadmap の「ウェーブ編成」の C23 の行。）

## Introduction

本仕様は、「本体の crate 分け」6 本の 4 本目として、areka 本体の bin crate の中にある入力の出来事のモジュール（`crates/areka/src/input_events/`）を、新しい lib crate `areka-input` へ移す。移すだけで、入力の中身は直さない。前例は `placement-crate-split`（2026-11-06）と `runtime-crate-split`（2026-11-17）で、やり方をそれにそろえる。前例と違うのは、⑴ input の中に本体を引くファイルが 2 本（`shortcut.rs`・`file_drop.rs`）あり、それを本体の中で置き直すこと ⑵ 強制退避の `quit_app` の呼び出しの向きを逆にすること ⑶ 同じウェーブの約束で、本体の呼び手のパス（`crate::input_events::…`）を出し直しで残すこと、の 3 つ。

受益者は areka の開発者であり、この仕様での「目に見える成果」は次のとおり開発者の側から定める。

- input の本番とテストが新しい crate にあり、本体はそれを依存として使っている。input を変えても、input の crate とそれを使う側だけが組み直され、input のテストは本体と別の実行ファイルで走る。
- input の crate は menu・install・session（本体）を参照しない。終えた後の依存の向きは `areka`（bin）→ `areka-input` → `areka-runtime` → `areka-placement` の一方向。
- 動く ukagaka のアプリとしての振る舞い（マウスのイベント・選択肢・ドラッグ・中断・ホイール・カーソルの形・位置初期化の近道・窓への投げ込み・強制退避・ログの中身〔出所の名前を除く〕・MCP の `get_log` の分け方）が前と同じである。
- テストの数・警告・全体テスト・実機テストが、移す前と同じ合否を示す。

数（ファイル数・参照の数）は brief の測りと 2026-11-20 の作業の木の測りによる参考値で、設計の段で測り直す。

## Boundary Context

- **In scope**:
  - 新しい crate `crates/areka-input/` の新設と、input の本番とテストの移動（履歴を残す移動）。同梱のカーソルの絵 `crates/areka/cursor/` の `crates/areka-input/cursor/` への移動と、それを名指す `tools/cursor-art.ps1` ほかの書き換え。
  - `tools/load-flake.ps1` の `-Binary` への `areka-input-lib` の追加。
  - `shortcut.rs`（とそのテスト）の本体の menu への移し替えと、`file_drop.rs`（とそのテスト 3 本）の本体の中での置き場。
  - input の公開面の付け替え（外から使われる項目だけを `pub` にする）、本体の `main.rs` の出し直し（`use areka_input as input_events;` の形）、本体の参照の書き換え（`ghost_session.rs`・`ghost_session_runtime.rs`・`app_exit.rs`・`menu/{mod,trigger}.rs` のうち出し直しで通らない所）。
  - 強制退避の `quit_app` の呼び出しの向きを逆にすること。
  - テストとテストの支えの移し替え（`placement_tests/spawn_assembly_tests.rs` の `areka-input` への移し替えを含む）。
  - ログの出所の名前の付け替え（`areka::input_events::…` → `areka_input::…`）と、名前で読んでいる所の書き換え。
  - input を名指すパスの書き換え: `crates/log-capture-kit/src/event.rs`・`event_tests.rs` の説明、本体の外の crate の説明（`areka-placement`・`areka-runtime` の `balloon_hover.rs`・`hit_region.rs`・`shell_box_hover.rs`・examples `collision-probe`・`areka-emo-text`）、本体の説明（`install/mod.rs`・`readme.rs`）、文書（`doc/COMPAT_ARCHITECTURE.md`・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml`・`doc/anchor-compat.md`・`doc/pasta-real-run.md`・`doc/ideas/`）、`.kiro/steering/structure.md`、`audit-areka-input-menu` の brief の「受け持つファイル」。
  - 検証（テストの名前の一覧の比べ・警告 0 件・全体テスト・実機テスト・`get_log` の分け方の前後一致・`THIRD-PARTY-NOTICES.md` の作り直し）。
- **Out of scope**:
  - 入力の振る舞いの変更。直したくなった不具合・古い説明（`input_events/mod.rs` の「The Hand／collisionex は実装しない（7.4）」ほか）は直さず、`audit-areka-input-menu` へ申し送る。
  - menu の crate 化（menu は install・update・roots・session を引くので本体に残す）。
  - session の集め直しと crate 化（`session-module-gather`・`session-crate-split`）。
  - 外から使われない `pub(crate)` の項目の削除。
  - 同じウェーブの約束で触らないファイル: `crates/areka-runtime/src/frame*`（`frame/wiring.rs` の説明の中の `input_events` の語を含む）・本体の `runtime_tests/`・`session/runtime_boot*`・`session/ghost_quit*`。
  - input に触れると書いているほかの brief（約 30 本）の書き換え（roadmap の新旧のパスの対応表で読み替える）。
  - 歴史の文書（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/`）と、日付つきの写し（`doc/roadmap-overview-2026-10-16.html`）の中の古いパス。
- **Adjacent expectations**:
  - 上流: `runtime-crate-split`（2026-11-17 着地）。input が使う runtime と placement の項目は既に `pub`（brief の再測定）。
  - 同じウェーブ C23 の約束（roadmap の C23 の行の ⑴〜⑹）: ⑴ この仕様は本体の `main.rs` に `use areka_input as input_events;` の形の出し直しを残して呼び手のパスを変えず、`crates/areka-runtime/src/frame*`・本体の `runtime_tests/`・`session/runtime_boot*`・`session/ghost_quit*` に触らない（できないなら次のウェーブへ）。⑸ `log-capture-kit/tests/file_length_guard_test.rs` の上限越えの表に触るのは `audit-emo2-boot-frame` だけ（2026-11-20 の測りで input のパスは表に 0 件）。⑹ `Cargo.lock` の席はこの仕様だけが持つ。並べ方そのものは roadmap が持つ約束で、この仕様の要件ではない。
  - 同じウェーブに置けない相手: `session-module-gather`・`audit-areka-input-menu`。`multi-ghost-runtime` とも同時に走らせない。
  - 下流: `session-module-gather`（次の 1 本）・`audit-areka-input-menu`（この仕様の着地を待つ）。

## Requirements

### Requirement 1: 新しい crate への移動

**Objective:** 開発者として、input の本番とテストを新しい crate `areka-input` へ移し、本体がそれを依存として使う形にしたい。それにより input の境目が crate の境目で守られ、input を変えても input の crate とそれを使う側だけが組み直されるため。

#### Acceptance Criteria

1. The 分割作業 shall `crates/areka-input/` を lib の crate として新設し、`publish = false` の印とその理由を同じ行に書き、`version.workspace = true` ほかワークスペースのほかの crate（`areka-placement`・`areka-runtime`）の慣習に従わせる。
2. The 分割作業 shall `crates/areka/src/input_events/` の本番とテストのファイル（`shortcut.rs`・`file_drop.rs` とそれぞれのテストを除く）を、git が移動として追える形（履歴を残す移動）で `crates/areka-input/src/` の下へ移す。
3. The 分割作業 shall 今の `input_events/mod.rs` の中身を新しい crate の根（`lib.rs`）とする。
4. When 移動を終えたとき, the 本体の crate `areka` shall input を `areka-input` への path 依存として使い、`crates/areka/src/input_events/` を持たない。
5. The `areka-input` クレート shall `areka-runtime` と `areka-placement` に path 依存で依存し、本体の crate `areka` には依存しない。
6. The 分割作業 shall ワークスペースの根の `Cargo.toml` を変えない。
7. The 分割作業 shall ワークスペースの外から新しい依存を足さない（足してよいのは、ワークスペースの中の path 依存と、`Cargo.lock` に既にある crate だけ）。
8. The `areka-input` クレート shall 本番の依存を、今の input（移す分）が本番で使う crate（brief の測りで tracing・wintf・bevy_ecs・areka-emo-text・areka-kanade・areka-sakura・areka-emo-compose・areka-ghost・areka-emo-present と `areka-runtime`・`areka-placement`。加えて `windows`〔`balloon_wheel.rs`・`cursor_answer.rs` の 3 か所。brief の「windows は 0」はギャップ分析の測りで誤りと分かった。ワークスペースの依存の機能の中に収まり、外の依存は増えない〕。`areka-nar` は `file_drop.rs` だけが使うので移す分には入らない〔ギャップ分析の実測 0〕。正確な一覧は設計の段で測り直す）に収め、ワークスペースのほかの crate と同じ形で受ける（外の crate は `workspace = true`、ワークスペースの中の crate は `{ path = "../…" }`）。
9. The `areka-input` クレート shall テストでだけ使う crate を、機能を付けないビルドで付く依存に入れない。番人の 3 つの crate（log-capture-kit・sample-ghost-kit・temp-path-kit）は optional も含めて本番の依存表に書かない（3.4 の決まりに従う）。
10. The `areka-input` クレート shall 1 ファイル 1,000 行の上限と、その例外表の決まりを、本体と同じく守る。例外表の項目は増やさない（2026-11-20 の測りで input のパスは表に 0 件・同じウェーブの約束 ⑸）。
11. The 分割作業 shall 同梱のカーソルの絵の置き場 `crates/areka/cursor/`（`hand.cur`・`grip.cur`・元の `.svg`・`README.md`）を、履歴を残す移動で `crates/areka-input/cursor/` へ移し、`cursor_answer.rs` の `include_bytes!` が今と同じ形の相対パスで解けるようにする。あわせて、その置き場を名指す所（`tools/cursor-art.ps1` とその自分の検査・`README.md`・網羅台帳 `assets.toml` の `system:grip`・`system:hand` の根拠の場所・`doc/COMPAT_ARCHITECTURE.md` §8）を移した先へ書き換え、`tools/cursor-art.ps1` の自分の検査を緑のまま保つ。絵のバイトは変えない。（要件の討議 議題 2・2026-11-20）

### Requirement 2: `shortcut.rs`・`file_drop.rs` の置き場

**Objective:** 開発者として、input の中で本体（menu・install・session）を引く 2 本のファイルを本体の中に置き直したい。それにより input の crate が本体を参照せずに済み、依存の向きが一方向に保たれるため。

#### Acceptance Criteria

1. The 分割作業 shall `shortcut.rs`（位置初期化の近道・本番 83 行）とそのテスト `shortcut_tests.rs` を、本体の menu の下（例 `crates/areka/src/menu/shortcut.rs`）へ、履歴を残す移動で移す。
2. The 分割作業 shall `file_drop.rs`（窓への投げ込み・本番 332 行）とそのテスト（`file_drop_tests.rs`・`file_drop_mime_tests.rs`・`file_drop_wiring_tests.rs`）を、本体の中に履歴を残す移動で置く。本体の中のどこに置くかは設計の段で決める（**仮定**: 本体の src 直下か、install の近く。下の 2.5 の分け方の決まりを満たす所に限る）。
3. When 2 本を置き直したとき, the 本体の呼び手（`ghost_session.rs` の `file_drop::attach_file_drop_receivers`・`shortcut::attach_shortcut_receivers` ほか）shall 置き直した先を指し、前と同じ時・同じ窓に受け手を付ける。
4. When 移動を終えたとき, the 分割作業 shall `areka-input` の本番のコードが本体の項目（menu・install・ghost_session・app_exit ほか）を参照していないことを grep の結果で示す。
5. The 分割作業 shall 置き直した 2 本の本番のログの行が、MCP の `get_log` の 5 種別の分け方（`log_history` の決まり）で、移す前と同じ種別に当たる（またはどれにも当たらない）ようにする。置き場によって出所が決まりの行（`areka::install` ほか）に新しく当たるなら、その置き場は採らないか、前と同じ分け方を保つ手当てを設計の段で決める。
6. The 分割作業 shall 位置初期化の近道（Ctrl＋Shift＋F12・Ctrl＋Shift＋F10）と窓への投げ込み（`OnFileDrop2`・`OnDirectoryDrop`・`.nar` のインストールへの依頼）の振る舞いを変えない。

### Requirement 3: 公開面と本体の参照

**Objective:** 開発者として、input の公開面を外から使われる分だけに絞り、本体の呼び手のパスは同じウェーブの約束どおり変えずに通したい。それにより input の外からは公開面しか触れず、同じウェーブのほかの spec が持つファイルにも触れずに済むため。

#### Acceptance Criteria

1. The 分割作業 shall input の外（本体の本番・テスト・`areka-input` へ移す境をまたぐテスト）から使われている `pub(crate)` の項目（brief の見込みは約 17 項目と 3 メソッドだったが、ギャップ分析の測りでモジュール 9・項目 26・メソッド 7 と、`test-support` で開けるメソッド 2。主な面は `MouseWiring`・`PendingDoubleClick`・`char_scope`・`click::on_char_pointer_released`・`attach_char_pointer_handlers`・`RegionSource`・`char_watch_with`・`drain_no_user_break_signals`・`register_char_cursor`・`BalloonWiring` ほか。正確な一覧は設計の段で測る）だけを `pub` にする。
2. The 分割作業 shall input の外から一度も使われていない `pub(crate)` の項目を、`pub(crate)` のまま残し、削除しない。
3. The 分割作業 shall input の中で自分を指すパス（`crate::input_events::…`）を、新しい crate の根を指すパスへ書き換える。
4. The 本体の crate `areka` shall `main.rs` で `areka-input` を `input_events` の名前で出し直し（`use areka_input as input_events;` の形）、本体の中の `crate::input_events::…` の呼び手がパスを変えずに通るようにする（同じウェーブの約束 ⑴）。
5. The 分割作業 shall `crates/areka-runtime/src/frame*`・本体の `runtime_tests/`・`session/runtime_boot.rs`・`session/runtime_boot_tests.rs`・`session/ghost_quit*` を変えない（同じウェーブの約束 ⑴）。
6. If 3.4 の出し直しでは通らない呼び手（`#[cfg(test)]` だけの項目を引くものほか）が 3.5 のファイルの中に見つかった場合, then the 分割作業 shall そのファイルを変えずに通す形（`areka-input` の `test-support` 機能で開けるなど）を設計で選び、それも無理なら roadmap の約束どおりこの仕様を次のウェーブへ回すことを開発者へ報告する。（ギャップ分析で、この場面が実際にあると分かった: `session/runtime_boot_tests.rs` が `#[cfg(test)]` だけの `UserBreakWiring::flag_rx` を使う。）
7. If 外から使われている項目を `pub` にすると公開面の型が非公開の型を名指すことになる場合, then the 分割作業 shall その非公開の型も、外から名指せる最小の範囲で公開し、どの項目をそのために開けたかを記録する。
8. If input の本番のファイルの説明の中のリンクが本体の項目（`crate::app_exit::…`・`crate::menu::…` ほか）を指している場合, then the 分割作業 shall そのリンクを、新しい crate から解ける形（本体を名指す地の文など）に書き換え、意味を変えない。

### Requirement 4: 強制退避の `quit_app` の向き

**Objective:** 開発者として、Ctrl+左ダブルクリックの強制退避が本体の `quit_app` を直に呼ぶ所を、本体が据えた口を通して呼ぶ形に逆にしたい。それにより input の crate が本体に依存せず、しかも退避の振る舞いと速さが前と同じに保たれるため。

#### Acceptance Criteria

1. The 分割作業 shall input の強制退避の分岐（`on_char_pointer_pressed` の、結線前の Ctrl+左ダブルクリック・Ctrl+Shift+左ダブルクリック）から本体の `app_exit::{ExitOrigin, quit_app}` を直に名指す所を無くし、本体が据えた口を通して同じ終了の統合操作（出所 `Escape`）を呼ぶ形にする。口の形は、新しい仕組みを作らず、本体の口の束の前例（`crates/areka-runtime/src/session_hooks.rs` の `SessionHooks`）にそろえて設計の段で選ぶ。
2. When 強制退避の操作を受けたとき, the areka shall 同じハンドラの呼び出しの中で（1 フレームも遅らせずに）全窓を閉じて終了を指示し、ハンドラは前と同じく `true` を返す。
3. While ゴーストの起動に失敗した・結線の前である（`MouseWiring` が無い）とき, the areka shall 強制退避を前と同じく受け付ける（口は起動の成否に関わらず、キャラ窓がポインタを受ける前に据わっている）。
4. While 結線済みで Shift を押していないとき, the areka shall Ctrl+左ダブルクリックを前と同じく普通の左ダブルクリックとして扱い、終了を指示しない。
5. When 強制退避で終わるとき, the areka shall ログの記録（`mouse_escape_close` から `quit_app` の終了の指示・`run()` の復帰・終了の系列まで）を、移す前と同じ中身・同じ順で残す。
6. If 本番の組み立てで口が据わっていない場合, then the areka shall 黙って何もしないことはせず、`error!` の記録を残す（ログ無しの失敗の経路を作らない）。本番の組み立ての道で口が据わることは決定論のテストで示す。（**仮定**: 据わっていないときの終了の扱い〔記録だけで終わらない／別の終わり方へ倒す〕は設計の段で決める。本番では起こらない経路で、今の振る舞いと同じ結果を出す形を推す。）
7. The 分割作業 shall 強制退避の振る舞い（受け付ける条件・終了の出所 `Escape`・最初の出所を残す決まり・閉じる窓）を決定論のテストで前と同じに固定し、今ある強制退避のテストを前と同じ合否で通す。

### Requirement 5: テストの置き場とテストの支え

**Objective:** 開発者として、input のテストを input の crate に置き、crate をまたぐテストと支えを写しを作らずに使い続けたい。それにより支えが二重にならず、下の crate に上の crate への依存が入り込まず、どのテストの実行ファイルも前と同じ守りを持つため。

#### Acceptance Criteria

1. The 分割作業 shall input のテスト（移す分）を `areka-input` のテストとして置き、本体を引くもの（menu・install・ghost_session ほかを引くもの）は境をまたぐテストとして本体に残すかを 1 本ずつ決め、どちらにしたかを記録する。本体の部品を偽物に替えてテストの中身を変えることはしない。
2. The 分割作業 shall placement と input を使う `placement_tests/spawn_assembly_tests.rs`（`attach_char_pointer_handlers`）を、`areka-input` のテストへ移す。
3. The 分割作業 shall input のテストが使う下の crate の支え（`areka-placement` の `test_support`・`emily_shaped_scopes`、`areka-runtime` の `spine::GpuPermit`・`RuntimeWiring::new`・`BootAssets`・`PositionCue`・`TalkClock`・`ZOrderDirective` ほか。brief の測りで `#[cfg(test)]` だけの外の項目が 5 ファイル・12 か所）を、下の crate の `test-support` 機能を `areka-input` の dev-dependency で付けて使い、写しを作らない。
4. The 分割作業 shall 共有のテスト用の crate（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）を、どの crate の本番の依存表（optional を含む）にも書かず、今の番人（`with_default_guard_test.rs`・`sample_path_guard_test.rs`）の決まりを変えない。
5. Where 本体のテストが input の `#[cfg(test)]` だけの支え（`balloon_test_support`・`drag_test_support` ほか）を使う場合, the `areka-input` クレート shall それを `test-support` という名の機能を付けたときだけ外から使える形で開け、本体はその機能をテストのビルドの依存（dev-dependency）でだけ付ける。
6. The `areka-input` クレートのテストの実行ファイル shall テストを走らせる前にプロセスの寿命の MTA を立て、テストの途中で落ちたときに、本体と同じ形の落ちの報告を標準エラーへ出す。落ちの報告の本体は `areka-runtime` の `crash_report_test_support` の 1 つだけを据えて使い、新しい写しを作らない（写しの統合は `test-crash-report-merge` の持ち物）。
7. The `areka-input` クレートのテストの実行ファイル shall テストの数え方（`cargo test --workspace -- --list`）で拾われ、全体テストの道具 `tools/test-all.ps1` を直さずに走る。
8. The 分割作業 shall 揺れるテストを繰り返す道具 `tools/load-flake.ps1` の `-Binary` に `areka-input-lib`（`-p areka-input --lib`）を足し（選べる値の一覧・切り替え・自分の検査の表の 3 か所と説明）、道具の自分の検査を緑のまま保つ。（要件の討議 議題 3・2026-11-20。前例の受け手 `audit-tools-scripts` は着地済みで受け手が無いため、この仕様で足す）

### Requirement 6: input を名指すパスと受け持ちの書き換え

**Objective:** 開発者として、input の古いパスを名指す所を、移した先のパスへ直したい。それにより説明・文書・台帳が実在しないパスを指したまま黙って通ることがなくなるため。

#### Acceptance Criteria

1. The 分割作業 shall `crates/log-capture-kit/src/event.rs`（150 行の説明）と `event_tests.rs`（20〜21 行・123 行の説明）の input のパスを、移した先のパスへ書き換える。
2. The 分割作業 shall 本体の外の crate の説明に残る input のパスと名前（`areka-placement` の `spawn.rs`・`follow.rs`・`follow/drag_follow.rs`・`follow_drag_end_gate_tests.rs`・`placement_char_clamp_tests.rs`・examples `window-placement.rs`、`areka-runtime` の `balloon_hover.rs`・`hit_region.rs`・`shell_box_hover.rs`・examples `collision-probe.rs`・`collision-probe/setup.rs`、`areka-emo-text` の `choice.rs`）を、移した先のパスと名前へ書き換え、動きを変えない。
3. The 分割作業 shall 本体の説明に残る input のパス（`install/mod.rs` の `input_events/file_drop.rs`・`readme.rs` の `input_events/choice_drain.rs`）を、移した先のパスへ書き換える。
4. The 分割作業 shall 文書の中の input のパスを、移した先のパスへ書き換える: `doc/COMPAT_ARCHITECTURE.md`（パスの書き換えだけ・約 20 か所）・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所（`assets.toml`・`sakura-script.toml`・`shiori.toml` の約 33 か所）・`doc/anchor-compat.md`・`doc/pasta-real-run.md`・`doc/ideas/` の下の文書（ギャップ分析の実測 6 ファイル）。網羅台帳は別々の行だけを書き換え、`doc/ukadoc-coverage/report/` は手で直さず作り直す。
5. The 分割作業 shall `.kiro/steering/structure.md` に新しい crate `areka-input` の節を足し、本体の節の `input_events/` の記述と `install/` の節の `input_events/file_drop.rs` を、移した先へ合わせる。`log-capture-kit` の使い手の crate の数（今は「21 crate」）にも `areka-input` を数え入れる。
6. The 分割作業 shall `audit-areka-input-menu` の brief の「受け持つファイル」を、移した先のパス（`crates/areka-input/src/`・本体の menu の下の `shortcut.rs` とそのテスト・本体の `file_drop.rs` とそのテストの置き場・本体の `menu*`）へ書き換える。`shortcut.rs`・`file_drop.rs` を見直しの範囲に残すかの判断（同 brief の「見つけた穴」⑴⑵）は同 spec の要件の段の持ち物で、この仕様は決めない。
7. When 書き換えを終えたとき, the 分割作業 shall 古いパス `crates/areka/src/input_events/` と、`src/` を付けない書き方（`input_events/x.rs`）で本体の input を指すものを、`crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す。除くのは、歴史の文書（`roadmap-history.md`・`.kiro/specs/completed/`）・新旧のパスの対応表を持つ `roadmap.md`・日付つきの写し `doc/roadmap-overview-2026-10-16.html`・同じウェーブの約束で触らない `crates/areka-runtime/src/frame/wiring.rs` の説明（3 か所）だけで、除いた所は一覧で残す。あわせて、モジュールの形（`input_events::throttle`・`input_events::cursor_answer` ほか）で input を名指すものを、`doc/` と本体の外の crate で grep して 0 件であることを示す（本体の中の `crate::input_events::…` は 6.8 のとおり当たりにしない）。
8. The 分割作業 shall 本体の中の呼び手のパス `crate::input_events::…`（3.4 の出し直しで通るもの）を 6.7 の当たりにしない。
9. The 分割作業 shall `crates/areka-runtime/src/frame/wiring.rs` の説明に残る古い input のパス（295・319・326 行の付近。`audit-emo2-boot-frame` の要件が直すのは 319 行だけ）を、要件の段のうちに `audit-emo2-boot-frame` のセッションへ「3 か所とも同じ扱いで」と頼む。When この仕様の完了の申し送りを書くとき, the 分割作業 shall 向こうが直さなかった所を、着地の前後に応じて `audit-emo2-boot-frame` の brief か後の本体の見直しの brief への申し送りとして記録する。（要件の討議 議題 4・2026-11-20）
10. When この仕様の完了の申し送りを書くとき, the 分割作業 shall `session-module-gather` の brief へ、本体の呼び手を `areka_input::…` へ書き換えて `main.rs` の出し直し `use areka_input as input_events;` を外すのは同 spec の持ち物であること（同 brief の Upstream の「input の参照が `areka_input::` になってから集める」はこの仕様では満たされない）を書き足す。（要件の討議 議題 1・2026-11-20）

### Requirement 7: ログの出所の名前

**Objective:** 開発者として、input のログの出所の名前が新しい crate の名前にそろい、名前で読む所が全部それに追従してほしい。それにより `RUST_LOG` で入力の診断を開く・実機テストがログを拾う・MCP の `get_log` がログを分けることが、移した後も前と同じく働くため。

#### Acceptance Criteria

1. The `areka-input` クレート shall モジュールのパスから付く本番のログの出所を、新しい crate の名前（`areka_input::…`）で出す。
2. The 分割作業 shall モジュールのパスを写した直書きの出所（`"areka::input_events::…"` の形のもの）があれば、新しい出所の名前に合わせる。語彙として付けた直書きの出所（`"areka::persist::*"`・`"areka::log::*"`・`"areka::perf"` ほか）は変えない。
3. The 分割作業 shall ログの出所を名前で読む所を、新しい出所の名前へ書き換える: `crates/areka/tests/emo2_mcp_real_run.rs` の `rust_log`（`areka::input_events::user_break=debug`）と説明・`crates/areka/tests/window_link_real_run.rs` の `RUST_LOG`（`areka::input_events=debug`）・実機の手順書 `doc/pasta-real-run.md` の `RUST_LOG` と説明（127・141・214・217 行）・input のテスト `cursor_answer_tests.rs` の `TARGET`（`"areka::input_events::char_watch::cursor_answer"`・ギャップ分析で見つけた 4 つ目の所）。
4. When `RUST_LOG` に `areka_input=debug` を与えたとき, the areka shall 入力の診断のログを、移す前に `areka::input_events=debug` で出ていたものと同じだけ出す。
5. The areka shall MCP の `get_log` の 5 種別への分け方で、移す前に input の出所の行が当たっていた種別（2026-11-20 の測りでは、どの決まりにも当たらない）を、移した後の `areka_input::…` の行でも同じにする（`shortcut.rs`・`file_drop.rs` の行は 2.5 のとおり）。
6. When 書き換えを終えたとき, the 分割作業 shall 古い出所の名前 `areka::input_events`（`areka::input_events::`・`areka::input_events=`・行末の形）を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す（歴史の文書と、新旧のパスの対応表を持つ `roadmap.md` は除く）。
7. The 分割作業 shall ログの本文（メッセージの文字列と構造化の欄の名前と値。`event = "mouse_escape_close"` ほか）を変えない。

### Requirement 8: 振る舞いを変えないことの検証

**Objective:** 開発者として、移した後も areka の振る舞いとテストの合否が前と同じであることを、決めた手順の証跡で確かめたい。それにより「移しただけ」が言葉でなく証跡で示されるため。

#### Acceptance Criteria

1. The 分割作業 shall 入力の振る舞い（マウスのイベント・選択肢・ドラッグ・中断・ホイール・カーソルの形・近道・投げ込み・強制退避・ログの中身）を変えない。直したくなった不具合と古い説明は直さず、`audit-areka-input-menu` へ申し送りとして記録する。理由の文が移した後に事実でなくなる `#[allow(...)]`（「areka は lib を持たない bin」ほかを理由にするもの）も、属性と文を変えず、一覧を同じ spec へ申し送る。
2. When 着手のときと完了のとき, the 分割作業 shall `cargo test --workspace --no-fail-fast -- --list` を取り、`: test$` の行から名前の末尾（最後の `::` から後ろ）の多重集合を作り、前後で一致することを示す。
3. If 前後の多重集合が一致しない場合, then the 分割作業 shall 名前を改めたテストと新しく足したテスト（落ちの報告の確かめ・口が据わる確かめ・`get_log` の分け方の確かめほか）を対応表で 1 件ずつ説明し、説明できない差を残さない。
4. The 分割作業 shall `cargo check --workspace --all-targets` の警告を 0 件のまま保つ。
5. When 完了の前に, the 分割作業 shall `pwsh -NoProfile -File tools/test-all.ps1` が緑であることを示す。
6. When 完了の前に, the 分割作業 shall ログの出所を名前で読む実機テスト `crates/areka/tests/emo2_mcp_real_run.rs`・`crates/areka/tests/window_link_real_run.rs` を、新しい出所の名前で 1 回ずつ走らせて緑を示す。
7. When 着手のときと完了の前に, the 分割作業 shall 強制退避の道を実機で同じ手順で 1 回ずつ走らせ、Ctrl+左ダブルクリックの退避で終わる記録の順（4.5）が、向きを逆にする前と後で同じであることを示す。
8. The 分割作業 shall 実機で回すテストを 8.6・8.7 のものだけにし、ほかは構造の証跡（テストの名前の一覧の比べ・警告・全体テスト・grep・決定論のテスト）で足りるとする。
9. The 分割作業 shall MCP の `get_log` の分け方が前後で同じこと（2.5・7.5）を、`log_history` のテストで示す。
10. The 分割作業 shall `THIRD-PARTY-NOTICES.md` を `cargo about generate` で作り直し、手で書かない。

## 設計の段への申し送り

1. **数の取り直し**（議題ではなく申し送り）: 公開にする `pub(crate)` の一覧（brief の見込み 約 17 項目と 3 メソッド）・input のテストが使う下の crate の支え（brief の 5 ファイル・12 か所）・本体のテストが使う input の `#[cfg(test)]` の支え・input を名指す文書の中のパス（2026-11-20 の作業の木で `doc/COMPAT_ARCHITECTURE.md` 20 行・網羅台帳 33 行・`doc/anchor-compat.md` 6 行・`doc/ideas/` 6 ファイル）を、設計の段で数え直す。
2. **brief の古くなった記述**（brief の再測定の節のとおり）: 「runtime と placement の側で input が使う項目を `pub` にする」は済んでいる。`quit_app` の「`frame.rs` に入れた終わりの知らせの仕組み」は古く、終わる相は本体の `session/ghost_quit.rs`、向きを逆にする口の前例は `crates/areka-runtime/src/session_hooks.rs` の `SessionHooks`。
3. **roadmap の C23 の行の触るファイルの一覧に無かった所**（2026-11-20 の作業の木の測り・同じウェーブの兄弟の触るファイルと重ならないことは設計の段で確かめる）: `doc/pasta-real-run.md`（`RUST_LOG` に `areka::input_events=debug`＝出所を名前で読む 3 つ目の所・実機の手順書）・`crates/areka/src/install/mod.rs`・`readme.rs`（説明だけ）・`areka-placement` の 6 ファイル（説明だけ）・`areka-runtime` の examples 2 ファイル（説明だけ）・`areka-emo-text/src/choice.rs`（説明だけ）・`doc/COMPAT_ARCHITECTURE.md`・網羅台帳・`doc/anchor-compat.md`・`doc/ideas/`・`tools/load-flake.ps1`（下の OPEN QUESTIONS の 4）。
4. **出し直しで通る呼び手**: 本体の `ghost_session_*_tests.rs`・`shell_balloon_switch_session_*_tests.rs`・`app_exit_tests.rs`・`menu/*_tests.rs` ほかは `crate::input_events::…` のまま通る見込みで、変えない（3.4）。

## 要件の討議で決めたこと（2026-11-20・開発者「推奨どおり承認」）

1. **出し直し `use areka_input as input_events;` を残す期間**（議題 1）: この仕様では外さない（3.4）。本体の呼び手を `areka_input::…` へ書き換えて出し直しを外すのは `session-module-gather` の持ち物とし、完了のときに同 brief へ書き足す（6.10）。本体の中の `crate::input_events::…` は grep の当たりにしない（6.8）。
2. **同梱のカーソルの絵の置き場**（議題 2）: `crates/areka/cursor/` を `crates/areka-input/cursor/` へ移す（1.11）。
3. **`tools/load-flake.ps1` に `areka-input-lib` を足す**（議題 3）: この仕様で足す（5.8）。
4. **`frame/wiring.rs` の古いパス 3 か所**（議題 4）: 要件の段のうちに `audit-emo2-boot-frame` のセッションへ 3 か所とも同じ扱いで直すよう頼み、直されなかった所は完了の申し送りへ回す（6.9）。
5. **`doc/ideas/` の下の文書**: 書き換えの範囲に入れる（6.4・実測 6 ファイル）。日付つきの写し `doc/roadmap-overview-2026-10-16.html` は前例どおり除く（6.7）。
6. **設計の段へ回したもの**: `file_drop.rs` の置き場（2.2・2.5）・口が据わっていないときの終わり方（4.6）・`hit_region.rs` の説明と `hit-id-pass` との重なり（6.2）・実機の強制退避の取り方（8.7）は、`research.md` §6 の論点 2・10・11・12 へ回した。
