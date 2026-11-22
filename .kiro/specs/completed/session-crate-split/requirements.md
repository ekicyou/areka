# Requirements Document

## Project Description (Input)

- **困っている人**: areka の開発者（本体 `crates/areka` を直し、見直し〔ponytail の audit〕を受け持ちごとに回す人）。
- **今の状態**: `session-module-gather`（PR#494・2026-11-22 着地）で、起動・終了・切替の層は本体の `crates/areka/src/session/`（122 ファイル・本番 約 8,600 行・テスト 約 25,000 行）に集まった。それでも、集めた層の本番は本体だけのモジュール（menu・install・update・`file_drop`・`smoke_exit`）を 28 行で直に呼んでいて、crate に出せない。たとえば窓を開いた直後にメニューの解放・投げ込み・近道の受け手を差す所、実行系が立った後にメニューを結線する所、切替の節目に install と update の机へ知らせる所、読み直しと切替の前に机が忙しいかを問う所、受け口の列に install と update の受け口を入れる所、告知を出すかの判断で煙の自動終了の設定を読む所がそうである。本体だけのモジュールの系の登録は、`main.rs` の `BODY_SYSTEMS`（`session::ghost_session::BodySystems` の関数の欄 2 つ）を `register_systems` へ渡す形になっている。
- **変えたいこと**: 本体の中で先に呼ぶ向きを逆にし、`session/` から本体の外への参照が 0 になったのを grep で示してから、集めた層の本番とテストを新しい crate `crates/areka-session/`（lib・`publish = false`・`areka-input`・`areka-runtime`・`areka-placement` に依存）へ移す。本体は「UI スレッドからエンジンへつなぐ」部分（main と起動オプション・MCP・メニュー・インストール・更新・投げ込み・オフラインの面の書き出し・SHIORI のデモ）だけになる。振る舞いは何も変えない。

（出どころ: `.kiro/specs/completed/session-crate-split/brief.md`〔最後の「2026-11-22 棚卸㊷の再測定（main `d4db9eb0`）」の節を含む。数はこの節が古い節より優先する〕。分け方・順番・共通の約束・新旧のパスの対応表の正本は `.kiro/steering/roadmap.md`「テーマ別の決めごと」の「本体の crate 分け」。同じウェーブの約束の正本は同じ roadmap の「ウェーブ編成」の C25 の行。外への参照の一覧の元は `.kiro/specs/completed/session-module-gather/research.md` の 14.3・14.4。）

## Introduction

本仕様は、「本体の crate 分け」6 本の 6 本目（最後）として、本体の `crates/areka/src/session/` を新しい lib crate `areka-session` へ移す。前例は `placement-crate-split`・`runtime-crate-split`・`input-crate-split` で、やり方をそれにそろえる。前例と違うのは次の 3 つ。

1. 移す前に、集めた層から本体へ向かう呼び出しの向きを逆にする（`runtime-module-untangle` → `runtime-crate-split` と同じ順を 1 本の中で行う）。逆にするのは、2026-11-22 の測りで本番 28 行（仕組み C1〜C5）と、新しく見つかった `alert.rs:286` の 1 行。
2. 本体だけの項目を引く集めた層のテストを、逆にした口の偽物で session の中で確かめるものと、本体に残して本物どうしをつなぐものに分ける。
3. 本体の中で `session::…` を引く約 117 ファイルの呼び手を、新しい crate のパスへ書き換える。

受益者は areka の開発者であり、この仕様での「目に見える成果」は次のとおり開発者の側から定める。

- 集めた層の本番とテストが新しい crate にあり、本体はそれを依存として使っている。`areka-session` に置くテストは本体と別の実行ファイルで走る（本体の本物どうしをつなぐテストは要件 5 のとおり本体に残る）。
- `areka-session` は本体を参照しない。終えた後の依存の向きは `areka`（bin）→ `areka-session` → `areka-input` → `areka-runtime` → `areka-placement` の一方向。
- 動く ukagaka のアプリとしての振る舞い（起動・切替・読み直し・終わり方・メニュー・投げ込み・インストールと更新の机・告知・ログの中身〔出所の名前を除く〕・MCP の `get_log` の分け方）が前と同じである。呼ぶ順も、1 フレームの中での呼ぶ時も変わらない。
- テストの数・警告・全体テスト・実機テストが、移す前と同じ合否を示す。

数（ファイル数・行番号・参照の数）は brief の最後の節と 2026-11-22 の作業の木（main `00877299`）の測りによる参考値で、設計の段で測り直す。

## Boundary Context

- **In scope**:
  - 本体へ向かう呼び出しの向きを逆にすること（C1 窓を開いた直後の差し込み 3 件・C2 実行系が立った後の結線 10 件・C3 切替の節目の知らせ 5 件・C4 机が忙しいかの問い 3 件・C5 受け口 2 件と `use` の行・`alert.rs:286` の煙の自動終了の設定の読み取り）。
  - 新しい crate `crates/areka-session/` の新設と、`session/` の 122 ファイル（`readme/`・`roots/` を含む）の移動（履歴を残す移動）。本体と新しい crate の `Cargo.toml`・`Cargo.lock`。
  - 公開面の付け替え（外から使われる項目だけを `pub` にする）と、本体の呼び手の書き換え（`session::` を引く約 117 ファイル〔根の 9・`install/` 20・`mcp/` 20・`menu/` 19・`update/` 11・`runtime_tests/` 24・`roots/` 6・`cli/` 4・`offline_dump/` 4〕と `tests/emo2_mcp_real_run.rs`・`tests/smoke_boot_loop_exit.rs`）。
  - テストの分け方と移し替え・テストの支えの置き場・`test-support` 機能。
  - ソースを読むテストの付け替え（`runtime_tests/zorder_wiring_tests.rs` の `include_str!` 2 行〔33・68 行〕と受け口の列の字面・`crate_path` で歩く見張り 4 本・`CARGO_MANIFEST_DIR` で読むテスト）。
  - ログの出所の名前の付け替え（`areka::session::…` → `areka_session::…`）と、名前で読む所の書き換え（`log_history` の決まり・real-run-kit の名指し・`RUST_LOG` の文字列・手順書）。
  - 集めた層のパスを名指す所の書き換え（コメント・文書・`structure.md`・`tech.md`・`audit-areka-app` と `audit-areka-install-update` の brief の「受け持つファイル」）。
  - 検証（テストの名前の一覧の比べ・警告 0 件・全体テスト・実機テスト・`get_log` の分け方の前後一致・`THIRD-PARTY-NOTICES.md` の作り直し）。
- **Out of scope**:
  - 振る舞いの変更。起動の段取り・切替の段取り・終わり方の順は変えない。直したくなった不具合・古い説明は直さず、`audit-areka-app`・`audit-areka-install-update` へ申し送る。
  - ゴーストの単位を複数にすること（`multi-ghost-runtime`）。
  - menu・install・update・mcp・roots の残り（`edit`・`desk`・`pick`）・cli・offline_dump の crate 化（どれも本体がつなぐ役そのもので、本体に残す）。
  - `areka-input` の `pub` を直に呼ぶ入力の 5 件（`attach_char_pointer_handlers`・`balloon::attach_balloon_pointer_handlers`・`wire_mouse_input`・`choice_drain::wire_choice_drain`・`balloon::wire_balloon_choice`）と、`register_systems` の中の入力の系の登録の向き。下の crate への呼び出しなので逆にしない。
  - C6（OS が窓を閉じる求めの `MouseWiring`）。`session/app_exit.rs` は `areka_input::MouseWiring` を引くだけで、逆にする所は 0 件（2026-11-22 確かめ）。
  - 外から使われない `pub(crate)` の項目の削除。
  - 集めた層に触れると書いているほかの brief の書き換え（roadmap の新旧のパスの対応表で読み替える）。
  - 歴史の文書（`.kiro/steering/roadmap-history.md`・`.kiro/specs/completed/`）と、日付つきの写し（`doc/roadmap-overview-2026-10-16.html`）の中の古いパス。
  - 同じウェーブの約束 ⑶ で触らない所: `crates/areka-placement/`・`crates/dola/`・`crates/areka-ghost/`・`crates/areka-seriko/`・本体の `crates/areka/src/placement_tests/`。`crates/areka-runtime/`・`crates/areka-kanade/` は、パスを名指すコメントのほかは触らない。
- **Adjacent expectations**:
  - 上流: `session-module-gather`（PR#494・2026-11-22 着地）。集めた層は `crates/areka/src/session/` にあり、本体だけのモジュールの系の登録は `main.rs` の `BODY_SYSTEMS` を渡す形になっている。向きを逆にする口の前例は `session::ghost_session::BodySystems`（関数の欄）・`crates/areka-runtime/src/session_hooks.rs` の `SessionHooks`・`crates/areka-input/src/hooks.rs` の `InputHooks`。
  - 同じウェーブ C25 の約束（roadmap の C25 の行）: ⑴ `test-name-work-tags` は本体の `runtime_tests/zorder_wiring_tests.rs` に触らない（このファイルの `include_str!` の行と字面の書き換えはこの仕様が持つ）。⑵ `dola-cue-test-builder` は呼び手の名（runtime の `carrier_cue` ほか）を残し、呼び手の 22 ファイルに触らない（この形なら、この仕様との重なりは 0）。⑶ この仕様は上の Out of scope のとおり。⑷ `doc/COMPAT_ARCHITECTURE.md` §8 は、この仕様のパスの行と `test-name-work-tags` の名の行が別々の既にある行に当たる（許す重なり）。⑸ `Cargo.lock` の席はこの仕様が持つ。並べ方そのものは roadmap の約束で、この仕様の要件ではない。
  - 同じウェーブに置けない相手: `audit-areka-app`・`audit-areka-install-update`・`audit-emo2-boot-switch`（この仕様の着地を待つ側）・`multi-ghost-runtime`・`doc-compat-architecture-rename`。
  - 下流: `audit-areka-app`・`audit-areka-install-update` はこの仕様の着地を待つ。`multi-ghost-runtime` は、この仕様の後に新しい crate の形で測り直して取る。

## Requirements

### Requirement 1: 本体へ向かう呼び出しの向きを逆にする

**Objective:** 開発者として、集めた層の本番から本体だけのモジュールを直に呼ぶ所を、本体が据えた口を通して呼ぶ形に逆にしたい。それにより集めた層が本体を参照せずに crate へ出せ、しかも起動・切替・読み直しの振る舞いが前と同じに保たれるため。

#### Acceptance Criteria

1. The 分割作業 shall 集めた層の本番（`session/` の下の本番のファイル。`*_tests.rs`・`*test_support*.rs`・`#[cfg(test)]` の塊を除く）から、`session/` の外の本体の項目（menu・install・update・`file_drop`・`smoke_exit` ほか）を名指す所（2026-11-22 の測りで 28 行と `alert.rs:286` の 1 行）を無くし、本体が据えた口を通して同じ処理を呼ぶ形にする。
2. When キャラ窓とバルーン窓を開いた直後（`ghost_session.rs` の窓を作るクロージャの中）, the areka shall 右クリックメニューの解放の受け手（`menu::attach_release_handlers`）・投げ込みの受け手（`file_drop::attach_file_drop_receivers`）・位置初期化の近道の受け手（`menu::shortcut::attach_shortcut_receivers`）を、前と同じクロージャの中で、前と同じ窓へ差す（C1・3 件）。
3. When 実行系が立った後（`ghost_session_runtime.rs` の結線）, the areka shall メニューの結線（`menu::wire_menu`）・メニューの各枠の登記 8 本（ゴースト・シェル・バルーン・インストール・置き場・ネットワーク更新・シェル位置初期化・バルーン位置初期化）・置換語の載せ直し（`install::names::reseed`）を、入力の結線（`areka_input::wire_mouse_input`）の後・起動成功の記憶（`boot_seam::on_boot_ok`）の前に、起こし直すたびに前と同じだけ行う（C2・10 件）。
4. When 切替の節目（落ち着いたとき・切替の終わり・上書きの間）に至ったとき, the areka shall install と update の机への知らせ（`install::desk::on_steady`・`update::desk::on_steady`・`update::desk::on_switch_end`〔2 か所〕・`install::desk::run_overwrite_between`）を、前と同じ節目に、前と同じ引数で届ける（C3・5 件）。
5. When 読み直し（`reload.rs`）とシェル・バルーンの切替（`shell_balloon_switch.rs`）が始まる前に机が忙しいかを問うとき, the areka shall install と update の机が忙しいかの答え（`install::desk::is_busy`・`update::desk::is_busy`）を、前と同じ時点の状態で得て、忙しいときは前と同じく断る（C4・3 件）。
6. When 実行系の起動（`runtime_boot.rs`）が受け口の列を作るとき, the areka shall install と update の受け口（`InstallCueSink`・`UpdateCueSink` と、それぞれの机の `raw_sender`）を、受け口の列の前と同じ位置に入れる（C5・2 件と `use` の 2 行）。
7. When 告知を出すかを決めるとき（`alert.rs:286`）, the areka shall 煙の自動終了の設定（`AREKA_APP_SMOKE_EXIT_MS`）があるときは前と同じく告知を抑える。`smoke_exit.rs`（本番 72 行）は本体の項目を `session::app_exit` のほかに引かないので、向きを逆にせず、テストごと `areka-session` へ移す。本体の `install/startup.rs:235`・`main.rs:267` は移した先を呼ぶ（2026-11-22 要件の討議 議題 2 で確定）。
8. The 分割作業 shall 逆にした口を、新しい仕組みを作らず、本体の口の前例（`BodySystems` の関数の欄・`SessionHooks`・`InputHooks`）にそろえる。C1〜C5 は 5 つとも、本体が中身を持つ関数のポインタの束を World の資源として据え、今と同じ呼び出しの中で同期に呼ぶ。知らせ（イベント・チャネル・observer）・install と update が立てる「忙しい」の資源・trait の口は取らない（1 フレーム遅れる・変わった時点が分からない値をプッシュにする・前例が無い）。束を 1 つにするか 2 つにするかと欄の細部は設計の段で決める（2026-11-22 要件の討議 議題 1 で確定）。
9. The 分割作業 shall `areka-input` の `pub` を直に呼ぶ入力の項目（窓の直後と実行系の後の入力の 5 件・`register_systems` の中の入力の系の登録）と、`session/app_exit.rs` の `MouseWiring`（C6）を、逆にしない。
10. When 向きを逆にし終えたとき（crate へ移す前）, the 分割作業 shall `session/` の下の本番のファイルが `session/` の外の本体の項目を名指していないこと（本番 0 行）を grep の結果で示し、テストだけに残る参照を一覧で記録する。
11. If 本番の組み立てで口が据わっていない状態がありうる場合, then the areka shall 黙って何もしないことはせず、`error!` の記録を残す（ログ無しの失敗の経路を作らない）。本番の組み立ての道で口が据わることは決定論のテストで示す。

### Requirement 2: 呼ぶ順と呼ぶ時を変えない

**Objective:** 開発者として、向きを逆にした後も、登録・差し込み・結線・知らせの順と時が前と同じであることを、テストで確かめたい。それにより「向きを逆にしただけ」が言葉でなく証跡で示され、1 フレームの遅れも入らないため。

#### Acceptance Criteria

1. The areka shall 系の登録（`register_systems` の中の順＝`menu_poll` が `user_break` の後・`balloon` の前、`after_input` の中の install → update → mcp）・窓の直後の差し込み・実行系の後の結線・受け口の列の順を、今の順と同じにする。
2. The 分割作業 shall 1 の順が今と同じであることを、呼ばれた順を記録する決定論のテストで示す。
3. The areka shall 逆にした口を通る呼び出しを、今と同じ呼び出しの中で（同じ系・同じハンドラ・同じクロージャの中で、1 フレームも遅らせずに）行う。逆にした知らせや口が、切替・終了・差し込みを 1 フレームでも遅らせない。
4. The areka shall 起こし直し（読み直し・切替・再起動）のたびに、逆にした口を今と同じ回数だけ通す。
5. The areka shall 逆にした道の節目に残すログの記録（中身と順）を、前と同じにする。

### Requirement 3: 新しい crate への移動

**Objective:** 開発者として、集めた層の本番とテストを新しい crate `areka-session` へ移し、本体がそれを依存として使う形にしたい。それにより集めた層の境目が crate の境目で守られ、集めた層を変えても集めた層の crate とそれを使う側だけが組み直されるため。

#### Acceptance Criteria

1. The 分割作業 shall `crates/areka-session/` を lib の crate として新設し、`publish = false` の印とその理由を同じ行に書き、`version.workspace = true` ほかワークスペースのほかの crate（`areka-placement`・`areka-runtime`・`areka-input`）の慣習に従わせる。
2. The 分割作業 shall `crates/areka/src/session/` の本番とテストのファイル（122 ファイル・`readme/`・`roots/` を含む）を、git が移動として追える形（履歴を残す移動）で `crates/areka-session/src/` の下へ移し、今の `session/mod.rs` の中身を新しい crate の根（`lib.rs`）とする。
3. When 移動を終えたとき, the 本体の crate `areka` shall 集めた層を `areka-session` への path 依存として使い、`crates/areka/src/session/` を持たない。
4. The `areka-session` クレート shall `areka-input`・`areka-runtime`・`areka-placement` に path 依存で依存し、本体の crate `areka` には依存しない。
5. The 分割作業 shall ワークスペースの根の `Cargo.toml` を変えず、ワークスペースの外から新しい依存を足さない（足してよいのは、ワークスペースの中の path 依存と、`Cargo.lock` に既にある crate だけ）。
6. The `areka-session` クレート shall 本番の依存を、今の集めた層が本番で使う crate に収め、ワークスペースのほかの crate と同じ形で受ける（外の crate は `workspace = true`、ワークスペースの中の crate は `{ path = "../…" }`）。正確な一覧は設計の段で測る。
7. The `areka-session` クレート shall テストでだけ使う crate を、機能を付けないビルドで付く依存に入れない。番人の 3 つの crate（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）は optional も含めて本番の依存表に書かない。
8. The `areka-session` クレート shall 1 ファイル 1,000 行の上限と、その例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の決まりを、本体と同じく守る。例外表の項目は増やさない。向きを逆にする直しは、上限に近いファイル（`ghost_switch.rs` 928 行・`ghost_session.rs` 897 行・`ghost_switch_tests.rs` 982 行・`main_session_mark_tests.rs` 957 行ほか）を上限の中に収める。
9. The 分割作業 shall crate の名前を `areka-session` とする。`session_end.rs`（OS のログオンのセッションの終わりの受け手）とそのテストのファイルの名前は改めず、`//!` の見出しの「OS のセッションの終了」で読み分ける（2026-11-22 要件の討議 議題 5 で確定）。

### Requirement 4: 公開面と本体の参照

**Objective:** 開発者として、集めた層の公開面を外から使われる分だけに絞り、本体の呼び手を新しい crate のパスへ書き換えたい。それにより集めた層の外からは公開面しか触れず、依存の向きが一方向に保たれるため。

#### Acceptance Criteria

1. The 分割作業 shall 集めた層の外（本体の本番・テスト・`tests/` の統合テスト）から使われている `pub(crate)` の項目だけを `pub` にする。正確な一覧は設計の段で測る。
2. The 分割作業 shall 集めた層の外から一度も使われていない `pub(crate)` の項目を、`pub(crate)` のまま残し、削除しない。
3. The 分割作業 shall 集めた層の中で自分を指すパス（`crate::session::…`）を、新しい crate の根を指すパスへ書き換える。
4. The 本体の crate `areka` shall 本体の中で集めた層を引く呼び手（`session::…`・`crate::session::…`）を、新しい crate のパス（`areka_session::…`）へ書き換える。`main.rs` に `use areka_session as session;` の形の出し直しは置かない。前の 2 本（`input-crate-split` の出し直しを `session-module-gather` が外した）の終わりの形にそろえる（2026-11-22 要件の討議 議題 6 で確定）。
5. If 外から使われている項目を `pub` にすると公開面の型が非公開の型を名指すことになる場合, then the 分割作業 shall その非公開の型も、外から名指せる最小の範囲で公開し、どの項目をそのために開けたかを記録する。
6. If 集めた層の本番のファイルの説明の中のリンクが本体の項目（`crate::menu::…`・`crate::install::…` ほか）を指している場合, then the 分割作業 shall そのリンクを、新しい crate から解ける形（本体を名指す地の文など）に書き換え、意味を変えない。
7. When 移動を終えたとき, the 分割作業 shall `areka-session` のソースが本体の crate の項目を参照していないことと、`areka-session` の依存表に `areka` が無いことを grep の結果で示す。

### Requirement 5: テストの分け方とテストの支え

**Objective:** 開発者として、本体だけの項目を引く集めた層のテストを、逆にした口の偽物で session の中で確かめるものと、本体に残して本物どうしをつなぐものに分けたい。それにより下の crate に上の crate への依存が入り込まず、支えが二重にならず、どのテストの実行ファイルも前と同じ守りを持つため。

#### Acceptance Criteria

1. The 分割作業 shall 本体だけの項目を引く集めた層のテストのファイル（2026-11-22 の測りで 21 本＝research.md 14.4 の 19 本と、根の関数を引く `session_end_tests.rs`・`boot_config_module_policy_tests.rs`。加えて `boot_config.rs` の `#[cfg(test)]` の口 `resolve_boot_from` が `crate::cli::parse::parse` を引き、それを使う session のテスト 6 本＝`boot_config_{balloon,copy,input,option,roots,callghost}_tests.rs`）を 1 本ずつ、`areka-session` に置くか本体に残すかを決め、どちらにしたかとその理由を記録する。
2. The 分割作業 shall 本体の部品の振る舞いを確かめているテスト（menu・install・update・mcp の本物の結果を見るもの）を、本体の部品を偽物に替えて中身を変えることはせず、本体に残す。
3. The 分割作業 shall 根の関数（`after_run`・`finish_after_run`・`boot_first_ghost`・`install_boot_context`）を引くテスト 7 本（`session_end_tests.rs`・`ghost_session_switch_fallback_tests.rs`・`ghost_session_switch_memory_tests.rs`・`readonly_run_tests.rs`・`readonly_sites_tests.rs`・`write_failure_sites_tests.rs`・`boot_config_module_policy_tests.rs`）を、結線の守りとして本体に残す。CLI の読み取り（`resolve_boot_from` が引く `crate::cli::parse::parse`）を通る 6 本（`boot_config_{balloon,copy,input,option,roots,callghost}_tests.rs`）も、テストの中身を変えずに本体に残し、`resolve_boot_from` は本体のテストの支えへ移す。1 本のファイルの中で一部のテストだけが本体を要るもの（`reload_tests.rs`・`shell_balloon_switch_tests.rs` ほか）は、テストの名前の末尾を変えずにファイルを分けてよい（2026-11-22 要件の討議 議題 4 で確定）。
4. Where 集めた層のテストが `crate::BODY_SYSTEMS` を渡している場合（5 か所＝`app_exit_escape_tests.rs:140`・`ghost_session_reload_tests.rs:71・179`・`ghost_session_restart_tests.rs:455`・`ghost_switch_test_support.rs:237`）, the 分割作業 shall `ghost_switch_test_support.rs` の `SwitchRig` を通るテスト（2026-11-22 の測りで、session の 32 ファイルと本体の install・mcp・update・`runtime_tests/`・`main_session_mark*` の 17 ファイル）が、前と同じ系の登録で走るようにする。`SwitchRig` は検体（`sample_ghost_kit::SampleRoot` から取った根・ゴーストのフォルダ・同梱バルーンの道と、その寿命を持つ値）と系の登録の組を呼び手から受ける形にする。各 crate は、検体を取って渡す数行の作り口を `#[cfg(test)]` で持つ。session の中のテストは逆にした口の偽物の組を、本体のテストは本物の `BODY_SYSTEMS` を渡す。どのテストが本物の組を要るかは 1 本ずつ決め、5.1 の記録に入れる（2026-11-22 要件の討議 議題 3 で確定）。
5. The 分割作業 shall 逆にした口の偽物で確かめるテストを session の中に置き、要件 2 の順と時（呼ばれた順・同じ呼び出しの中で呼ばれること）を確かめる。
6. The 分割作業 shall テストの支えの置き場 3 つ（`crate_source_test_support`〔`crate_source!`・`crate_path`・session の 4 ファイル 7 か所〕・`sample_test_support::acquire_emo2`〔session の 4 ファイル〕・`runtime_tests::spine::SpineHarness`〔session の 2 ファイル〕）の扱いを決め、写しを作らない。`crate_source_test_support` は番人の crate に頼らないので、`areka-session` の `test-support` 機能で開ける置き場へ移し、本体はそこから使う。`sample_test_support` は `sample-ghost-kit` に頼り、番人の決まり（5.9）で `test-support` に入れられないので、session のテストは窓口 `SampleRoot::acquire("emo2")` を直に呼び（前例＝`areka-runtime` 自身の `#[cfg(test)]` の支え）、本体の `sample_test_support` は本体に残す。`SpineHarness` を使う 2 本（`ghost_session_reload_tests.rs`・`ghost_session_restart_tests.rs`）は `BODY_SYSTEMS`・menu も引くので本体に残し、`SpineHarness` は本体に置いたままにする（2026-11-22 要件の討議 議題 3 で確定）。
7. The 分割作業 shall 集めた層のテストが使う下の crate の支え（`areka-input`・`areka-runtime`・`areka-placement` の `#[cfg(test)]` だけの項目）を、下の crate の `test-support` 機能を `areka-session` の dev-dependency で付けて使い、写しを作らない。
8. Where 本体のテストが集めた層の `#[cfg(test)]` だけの支え（`SwitchRig` ほか。ただし番人の crate に頼る部分は 5.4・5.6 のとおり呼び手の側に置く）を使う場合, the `areka-session` クレート shall それを `test-support` という名の機能を付けたときだけ外から使える形で開け、本体はその機能をテストのビルドの依存（dev-dependency）でだけ付ける。
9. The 分割作業 shall 共有のテスト用の crate（`log-capture-kit`・`sample-ghost-kit`・`temp-path-kit`）を、どの crate の本番の依存表（optional を含む）にも書かず、今の番人（`with_default_guard_test.rs`・`sample_path_guard_test.rs` ほか）の決まりを変えない。
10. The `areka-session` クレートのテストの実行ファイル shall テストを走らせる前にプロセスの寿命の MTA を立て、テストの途中で落ちたときに、本体と同じ形の落ちの報告を標準エラーへ出す。落ちの報告は `crash-report-kit` を使い、新しい写しを作らない。
11. The `areka-session` クレートのテストの実行ファイル shall テストの数え方（`cargo test --workspace -- --list`）で拾われ、全体テストの道具 `tools/test-all.ps1` を直さずに走る。
12. The 分割作業 shall 揺れるテストを繰り返す道具 `tools/load-flake.ps1` の `-Binary` に `areka-session-lib`（`-p areka-session --lib`）を足し（選べる値の一覧・切り替え・自分の検査の表と説明）、道具の自分の検査を緑のまま保つ（`input-crate-split` の要件の討議 議題 3 の前例にそろえる・2026-11-22 要件の討議 議題 7 で確定）。
13. The 分割作業 shall ソースを読むテスト（`include_str!`・`CARGO_MANIFEST_DIR`・`crate_path`・`crate_source!` で読むもの。`runtime_tests/zorder_wiring_tests.rs` の 33・68 行・`readonly_tests.rs`・`session_end_deadline_tests.rs`・`session_end_sync_send_tests.rs`・`readme_tests.rs`・`log_history_convention_tests.rs`・`boot_shell_tests.rs` ほか）を、移した後も前と同じファイルの集まりを読む形に付け替え、見る範囲を狭めない。ソースを読むテストは依存の向きで自分より下の crate だけを読む（`crate_source_test_support` の決まり）ので、本体のソース（`main.rs` ほか）を読む見張りは本体に置き、歩く根の並びに `areka-session` を足す。
14. When 受け口の列の書き方が向きの逆転で変わるとき, the 分割作業 shall `runtime_tests/zorder_wiring_tests.rs` の `t_zwi05` が押さえる受け口の列の字面を新しい書き方に合わせ、13 個の受け口が前と同じ順に並ぶことを押さえ続ける。

### Requirement 6: ログの出所の名前

**Objective:** 開発者として、集めた層のログの出所の名前が新しい crate の名前にそろい、名前で読む所が全部それに追従してほしい。それにより `RUST_LOG` で起動や切替の診断を開く・実機テストがログを拾う・MCP の `get_log` がログを分けることが、移した後も前と同じく働くため。

#### Acceptance Criteria

1. The `areka-session` クレート shall モジュールのパスから付く本番のログの出所を、新しい crate の名前（`areka_session::…`）で出す。
2. The 分割作業 shall モジュールのパスを写した直書きの出所（`"areka::session::…"` の形のもの。`log_history.rs` の決まりの行〔`boot_config`・`boot_resolve`・`ghost_session`・`ghost_switch`・`readme`〕・`log_history_tests.rs`・`log_history_convention_tests.rs`・`reload_tests.rs`・`reload_skin_tests.rs` ほか）を、新しい出所の名前に合わせる。語彙として付けた直書きの出所（`"areka::log::error"`・`"areka::persist::*"`・`"areka::perf"` ほか）と、本体に残るモジュールの出所（`areka::install`・`areka::update` ほか）は変えない。
3. The 分割作業 shall ログの出所を名前で読む所を、新しい出所の名前へ書き換える: real-run-kit の `launch.rs` の `RELIED_TARGETS`（`"areka::session::app_exit"`）・`launch_tests.rs`・`close_tests.rs`・`bin/real-run-fake-areka.rs`・`crates/areka/tests/emo2_mcp_real_run.rs` の `rust_log`・`BOOT_CONFIG_PREFIX` と説明・`doc/pasta-real-run.md` の `RUST_LOG`（127・141 行）・`doc/ssp-mcp/log-convention.md` の決まりの表・`doc/testing.md`。
4. When `RUST_LOG` に `areka_session::boot_config=info` の形で新しい出所を与えたとき, the areka shall 移す前に `areka::session::boot_config=info` の形で出ていたものと同じだけのログを出す。
5. The areka shall MCP の `get_log` の 5 種別への分け方で、移す前に集めた層の出所の行が当たっていた種別（`boot_config`・`boot_resolve`・`ghost_session`・`ghost_switch`・`readme` は status ほか）を、移した後の `areka_session::…` の行でも同じにする。
6. When 書き換えを終えたとき, the 分割作業 shall 古い出所の名前 `areka::session`（`areka::session::`・`areka::session=`・行末の形）を `crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す（歴史の文書と、新旧のパスの対応表を持つ `roadmap.md` は除く）。
7. The 分割作業 shall ログの本文（メッセージの文字列と構造化の欄の名前と値）を変えない。

### Requirement 7: 集めた層を名指すパスと受け持ちの書き換え

**Objective:** 開発者として、集めた層の古いパスを名指す所を、移した先のパスへ直したい。それにより説明・文書・台帳・見直しの受け持ちが実在しないパスを指したまま黙って通ることがなくなるため。

#### Acceptance Criteria

1. The 分割作業 shall 本体の外の crate のコメントに残る集めた層のパス（`crates/areka-runtime/src/talk_clock.rs:229`・`user_break_cue.rs:79`・`placement_tests/transition_judge_offset_signoff_tests.rs:57`・`crates/areka-kanade/src/schedule/script_log.rs:16`。2026-11-22 の作業の木で、`src/` を付けない書き方の `crates/areka-runtime/src/lib.rs:391`・`strict_judges.rs:5` も見つかった）を、コメントだけ書き換え、コードを変えない（同じウェーブの約束 ⑶）。
2. The 分割作業 shall 本体のコメントに残る集めた層のパス（`crates/areka/tests/emo2_real_run.rs:58`・`crates/areka/Cargo.toml:81`・`runtime_tests/spine_move_cue_tests.rs:11` ほか）を、移した先のパスへ書き換える。
3. If 同じウェーブの約束 ⑶ で触らない crate（`crates/areka-placement/` ほか）のコメントが集めた層のパスを名指している場合（2026-11-22 の作業の木で `areka-placement` の `diag.rs:46`・`persist.rs:330`・`resolver.rs:97`・`resolver_resolve_balloon_tests.rs:123` の `session/boot_seam.rs`）, then the 分割作業 shall それを書き換えず、7.7 の grep の除外の一覧に載せ、完了の申し送りで `audit-areka-app`（`session-module-gather` の research.md 14.6 の 6 と同じ受け手）へ渡す。
4. The 分割作業 shall 文書と道具の中の集めた層のパスを、移した先のパスへ書き換える: `doc/COMPAT_ARCHITECTURE.md`（§8 を含むパスの行だけ・2026-11-22 の測りで 38 か所）・網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所（`sakura-script.toml` 18・`property.toml` 10・`shiori.toml` 9・`assets.toml` 1）・`doc/ukadoc-coverage/briefing-assets.md`・`doc/testing.md`・`doc/ssp-mcp/log-convention.md`・`doc/pasta-real-run.md`・`doc/ideas/` の下の文書・`tools/package.ps1` のコメント（3 か所）・`tools/perf/judge-perf.py` の説明（1 か所）。網羅台帳は別々の行だけを書き換え、`doc/ukadoc-coverage/report/` は手で直さず作り直す。
5. The 分割作業 shall `.kiro/steering/structure.md` に新しい crate `areka-session` の節を足し、本体の節の `session/` の記述を移した先へ合わせ、`log-capture-kit` ほか使い手の crate の数に `areka-session` を数え入れる。`.kiro/steering/tech.md` の集めた層のパスも移した先へ合わせる。
6. The 分割作業 shall `audit-areka-app` と `audit-areka-install-update` の brief の「受け持つファイル」を、移した先のパス（`crates/areka-session/src/` の下の該当するファイルと、本体に残るテスト）へ書き換える。所見の文の中の古いパスは書き換えず、各 audit の要件の段で読み替える。
7. When 書き換えを終えたとき, the 分割作業 shall 古いパス `crates/areka/src/session/` と、`src/` を付けない書き方（`session/x.rs`）で本体の集めた層を指すものを、`crates/`・`tools/`・`doc/`・`.kiro/steering/structure.md`・`.kiro/steering/tech.md` で grep して 0 件であることを示す。除くのは、歴史の文書（`roadmap-history.md`・`.kiro/specs/completed/`）・新旧のパスの対応表を持つ `roadmap.md`・日付つきの写しだけで、除いた所（7.3 の `areka-placement` の 4 か所を含む）は一覧で残す。

### Requirement 8: 振る舞いを変えないことの検証

**Objective:** 開発者として、移した後も areka の振る舞いとテストの合否が前と同じであることを、roadmap の「本体の crate 分け」の共通の約束の手順の証跡で確かめたい。それにより「向きを逆にして移しただけ」が言葉でなく証跡で示されるため。

#### Acceptance Criteria

1. The 分割作業 shall 移す・改名する・向きを逆にするだけにし、起動・切替・読み直し・終わり方・メニュー・投げ込み・インストールと更新の机・告知・ログの中身の振る舞いを変えない。直したくなった不具合と古い説明は直さず、範囲を受け持つ audit（`audit-areka-app`・`audit-areka-install-update`）へ申し送りとして記録する。理由の文が移した後に事実でなくなる `#[allow(...)]`（「areka は lib を持たない bin」ほかを理由にするもの）も、属性と文を変えず、一覧を同じ audit へ申し送る。
2. When 着手のときと完了のとき, the 分割作業 shall `cargo test --workspace --no-fail-fast -- --list` を取り、`: test$` の行から名前の末尾（最後の `::` から後ろ）の多重集合を作り、前後で一致することを示す。
3. If 前後の多重集合が一致しない場合, then the 分割作業 shall 名前を改めたテストと新しく足したテスト（呼ぶ順の確かめ・口が据わる確かめ・落ちの報告の確かめほか）を対応表で 1 件ずつ説明し、説明できない差を残さない。
4. The 分割作業 shall `cargo check --workspace --all-targets` の警告を 0 件のまま保つ。
5. When 完了の前に, the 分割作業 shall `pwsh -NoProfile -File tools/test-all.ps1` が緑であることを示す。
6. When 完了の前に, the 分割作業 shall ログの出所を名前で読む実機テスト `crates/areka/tests/emo2_mcp_real_run.rs` を、新しい出所の名前で 1 回走らせて緑を示す。
7. When 着手のとき（向きを逆にする前）と完了の前に, the 分割作業 shall 向きを逆にした道を実機で同じ手順で 1 回ずつ回し、記録の順が前後で同じであることを示す: 起動・メニューからのゴーストの切替・MCP の `reload`・メニューの終了・煙の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）・窓への `.nar` の投げ込み。
8. The 分割作業 shall OS のセッションの終わり（`WM_ENDSESSION`）を、実機では起こせないので、今の決定論のテストで足りるとする。
9. The 分割作業 shall 実機で回すテストを 8.6・8.7 のものだけにし、ほかは構造の証跡（テストの名前の一覧の比べ・警告・全体テスト・grep・決定論のテスト）で足りるとする。
10. The 分割作業 shall MCP の `get_log` の分け方が前後で同じこと（6.5）を、`log_history` のテストで示す。
11. The 分割作業 shall `THIRD-PARTY-NOTICES.md` を `cargo about generate` で作り直し、手で書かない。

## 設計の段への申し送り

1. **数の取り直し**（議題ではなく申し送り）: 逆にする本番の行（28 行＋`alert.rs:286`）・本体だけの項目を引くテストのファイル（21 本と `boot_config.rs` の `#[cfg(test)]` の口）・`SwitchRig` を通るファイル（2026-11-22 の測りで 49 本。brief は 47）・公開にする `pub(crate)` の一覧・本体で `session::` を引くファイル（brief 117）・文書の中のパス（`doc/COMPAT_ARCHITECTURE.md` 38・網羅台帳 38・`doc/ideas/` 8 ファイル）を、設計の段で数え直す。
2. **brief の古くなった記述**（brief の再測定の節のとおり）: Current State の「テスト 35 ファイル（roots の残り 47 件…）」は古く、今は 5.1 の数。C1〜C5 の行は brief の古い節から動いた（要件 1 の行番号は 2026-11-22 の作業の木で確かめた）。C1 の件数は、入力の 2 件が `areka_input` を直に呼ぶので逆にするのは 3 件、C2 は入力の `wire_mouse_input` を除いて 10 件。C6 は 0 件。brief の古い節の「C1・C2・C5 は trait 1 つ（メソッド 3 つ）」は、今は `BodySystems` の関数の欄が一番近い前例。
3. **新しく見つかった所**（2026-11-22 の作業の木の測り）: ⑴ `boot_config.rs:199〜214` の `#[cfg(test)]` の口 `resolve_boot_from` が `crate::cli::parse::parse` を引く（research.md 14.4 にあり、brief の 19 本には入らない）。⑵ `runtime_tests/zorder_wiring_tests.rs` の `t_zwi05` が `runtime_boot.rs` の受け口の列を `crate::install::desk::raw_sender(world)` の字面のまま押さえる（C5 の逆転で字面が変わる・5.14）。⑶ `readonly_tests.rs:109` が `CARGO_MANIFEST_DIR` から `src/main.rs` を読む・`session_end_deadline_tests.rs` が `"areka/session/…"` の鍵でファイルを名指す（5.13）。⑷ real-run-kit の `launch_tests.rs` の 246〜253 行は、前方一致の境目を確かめる「似ているが違う」出所（`areka::session::app`・`areka::session::app_exits`）を持つ。新しい名前に合わせて同じ境目を確かめる形にする。⑸ `tools/package.ps1`（3 か所）・`tools/perf/judge-perf.py`（1 か所）・`doc/ukadoc-coverage/briefing-assets.md`・`.kiro/steering/tech.md`（1 か所）が集めた層のパスを名指す（roadmap の C25 の行の触るファイルの一覧に無かった所・同じウェーブの兄弟の触るファイルと重ならないことは設計の段で確かめる）。
4. **口の前例**: `BodySystems`（`session/ghost_session.rs:36`・`main.rs:365` の `BODY_SYSTEMS`）・`SessionHooks`（`crates/areka-runtime/src/session_hooks.rs`）・`InputHooks`（`crates/areka-input/src/hooks.rs`）。C1 の 3 件は窓を作るクロージャの中で入力の差し込みの間に挟まる（`attach_char_pointer_handlers` → menu の解放 → `attach_balloon_pointer_handlers` → 投げ込み → 近道）ので、順を保つ形を設計で選ぶ。

## 要件の討議の決め（2026-11-22・開発者が全議題を推奨案で一括承認）

1. **口の形**（1.8）: C1〜C5 は関数のポインタの束を World の資源として据え、同期に呼ぶ。知らせ・「忙しい」の資源・trait は取らない。束の数と欄は設計の段。
2. **`smoke_exit.rs` の置き場**（1.7）: テストごと `areka-session` へ移す（向きを逆にしない）。
3. **`SwitchRig` と検体の支え**（5.4・5.6・5.8）: `SwitchRig` は検体と系の登録の組を呼び手から受けて `test-support` で開く。session のテストは `SampleRoot::acquire("emo2")` を直に呼ぶ。本体の `sample_test_support` は本体に残す。
4. **本体に残すテスト**（5.3）: 根の関数を引く 7 本・本物の結果を見るもの・CLI の読み取りを通る 6 本。一部だけ本体を要るファイルは名前の末尾を変えずに分けてよい。
5. **`session_end.rs` の名前**（3.9）: 改めない。
6. **本体の呼び手の書き換え**（4.4）: 出し直しを置かず `areka_session::…` へ書き換える。
7. **`tools/load-flake.ps1`**（5.12）: `areka-session-lib` を足す。
