# Requirements Document

## Project Description (Input)

開発者は、テストの実行ファイルが落ちたときに落ちた場所とスレッドの一覧を書き残す「落ちの報告」の仕組みを、1 か所でだけ直せるようにしたい。今は、ほぼ同じ写しが 3 本ある。`crates/wintf/src/test_crash_report.rs`（487 行）・`crates/areka-placement/src/test_crash_report.rs`（490 行）・`crates/areka-runtime/src/crash_report_test_support.rs`（587 行）で、計 1,564 行（2026-11-21 `main` `f6405e09`・`wc -l`）。wintf と areka-placement の写しの違いは説明の数行だけで、areka-runtime の 1 本はそれに GPU の装置の許可の持ち手の表（`hold`・`release` と、報告の `AREKA-CRASH-HOLDER` の行）と、本体のテストの実行ファイルが使う `installed` を足した形である。1 本を直すと残りの 2 本も同じように直す必要があり、直し忘れが黙って食い違う。windows-rs の移行（v0.2.0）でも同じ書き換えを 3 回することになる。

直した後の姿: 落ちの報告の本体が 1 か所にだけ在り、落ちの報告を据えるテストの実行ファイル（wintf・areka-placement・areka-runtime・areka-input・本体の各 crate のテスト）はそれを呼ぶだけになる。許可の持ち手の表は、使う crate（`areka-runtime`）だけが持つ形で残る。テストの振る舞い（落ちたときに書く中身・書く先）は変わらない。

あわせて、`audit-emo2-boot-frame` の範囲の外で見つけた `areka-placement` の古い注と写し（同 spec の `research.md` の 6 節ウ〜カと 12.5・brief の「audit-emo2-boot-frame からの申し送り」）を、この spec が `areka-placement` に触るときにまとめて直す。申し送りのイ（`follow/window_move.rs` の「見送りの 4 点」）は PR#488 でその文ごと消えており、直すものは残っていない（brief の再測定）。

出どころは ✅ `audit-emo2-boot-spine`（PR#476）の範囲の外の件「エ」で、優先度の区分は A（見直しの指摘の取り残し・2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。

## Introduction

この spec は、テストのプロセスの落ちの報告（プロセスの読み込みの初めに例外の受け手とプロセスの寿命の MTA を据え、捕まえられなかった例外で落ちたときに、例外のコード・番地・落ちたスレッドの名前とモジュール・生きているスレッドの一覧・積み上げを標準エラーの OS のハンドルへ直に書く仕組み）の 3 本の写しを 1 本にまとめるためのものである。まとめるのはテストの道具だけで、areka の本番の振る舞いは何も変えない。

まとめた後も変えてはならない「外への約束」は次のとおりとする。

- 報告の行の語（`AREKA-CRASH`・`AREKA-CRASH-HOLDER`・`AREKA-CRASH-THREAD`・`AREKA-CRASH-STACK`・`AREKA-MTA-KEEPER`）と形。`tools/load-flake.ps1` が `AREKA-CRASH` を含む行を字面で拾う（193 行）。
- 書く先（標準エラーの OS のハンドルへ直に・libtest の捕まえと `std` の標準エラーの錠を通らない）と、ふだんの緑の走行では何も書かないこと。
- 落ちの報告を据える各テストの実行ファイルで、受け手とプロセスの寿命の MTA が libtest の main より前に据わっていること。
- 本体のテストと `areka-input` のテストが使う入口（`areka_runtime::crash_report_test_support::install`・`installed`）と、`areka-runtime` の背骨の待ちの部品（`spine_wait.rs` の `GpuPermit`）が使う許可の持ち手の表の載せ降ろし（`hold`・`release`）。

この spec はウェーブ C24 で、`window-resetwindowpos-hold`・`session-module-gather`・`audit-areka-input-menu` と並んで走る。並走の約束（brief の「同じウェーブの相手との約束（C24）」）を守る（要件 7）。

## Boundary Context

- **In scope**:
  - 3 本の写しを 1 本にまとめること・その置き場の新しい leaf の crate `crates/crash-report-kit/`（決定 1）と、`Cargo.toml`（ワークスペースの members と使い手 3 crate）・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` の該当の行。
  - 3 つの crate（wintf・areka-placement・areka-runtime）の `mod` の宣言と、使い手（`areka-runtime` の `spine_wait.rs`、説明の中で名を挙げる `wire_tests.rs`・`assets_tests.rs`・`assets_shell_tests.rs`・`assets_markers_tests.rs`、本体の `crates/areka/src/runtime_tests/mod.rs`）の付け替え。
  - まとめたことを確かめるテスト（要件 5）。
  - `audit-emo2-boot-frame` から預かった `areka-placement` の件（要件 6）。
  - 置き場の crate の依存の説明（steering の `structure.md` の該当の節）を実物に合わせること。
- **Out of scope**:
  - 落ちの報告の中身や書く先を変えること。
  - `write_thread_name` が `GetThreadDescription` の返した文字列を `LocalFree` しない件（`crash_report_test_support.rs` 433 行の `shortcut:` の注・意図的な先送り・`audit-emo2-boot-spine` の research 5 節 ⑵）。
  - windows-rs の移行そのもの。
  - `carrier_cue` の写しのうち `crates/areka-ghost/src/prop_sink.rs`（118 行）と `crates/areka-seriko/src/actor_test_support.rs`（124 行）。合図の型 `TalkCue`・`CueCommand` は `dola`（`crates/dola/src/cue/command.rs` の 99・288 行）にあり、落ちの報告の置き場（ワークスペースの中の crate に依存しない leaf）へは寄せられない＝範囲の外のまま。完了の未解決の棚卸で「`dola` にテスト用の組み立てを置く」件として起票する（決定 5）。
  - 申し送りに名指されていない `code_only` の写し（wintf の 8 本・`areka-emo-present` の 1 本・本体の 2 本。本体の `tick_gate_config_producers_tests.rs` の分は `audit-areka-app` の持ち物）と、`areka-placement` の `measure_template::code_only`（ブロックコメントも落とす別の働きで、写しではない）。
  - `areka-runtime` の `frame_work_area_resnap_hold_tests.rs` 60 行の `INVARIANT_WARNING` の手写し（申し送りに名指されていない・`areka-runtime` の `frame*` の側）。同じファイルの 372〜410 行に在ることを数える檻が既に在るかを、完了の未解決の棚卸で確かめる（決定 6）。
  - 1,000 行の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表。
- **Adjacent expectations**:
  - `crates/areka-input/src/crash_report_tests.rs`（5・10 行）は `areka_runtime::crash_report_test_support::{install, installed}` で落ちの報告を据えている（`input-crate-split` PR#486 で足された 4 本目の使い手・brief の Current State には載っていない）。同じウェーブの `audit-areka-input-menu` が `crates/areka-input/` を受け持つので、この spec はその入口の名と働きを変えず、`crates/areka-input/` のファイルに触らない（決定 8・2026-11-21 に向こうのセッションと合意）。
  - `audit-areka-input-menu` は開発者の裁定で、areka-input の中の placement・runtime だけを試すテストを持ち主の crate へ移す（`crates/areka-placement/src/spawn.rs` の末尾の `mod` の並びに 1 組・新しい `spawn_assembly_tests.rs`・`areka-runtime` の `balloon_hover.rs`・`shell_box_hover.rs` の末尾と新しいテストのファイル）。この spec と重なるのは `spawn.rs` 350 行の説明の古い道（要件 6.2）だけで、行は離れている。
  - 本体の `runtime_tests/mod.rs` は `session-module-gather` が触らない約束なので、この spec が付け替えてよい。
  - `audit-areka-app` は本体の `code_only` の写しを `areka_runtime::source_text_test_support::code_only` を使う形に寄せる。この spec の要件 6.3 の直しでも、その道で `code_only` が引ける形を保つ。
  - windows-rs の API の数え（`crates/pilot/examples/pilot-windows-rs-next/q9-api-count`）は、`*_test_support`・`test_*` の名のファイルと、名が `-kit` で終わる crate をテストの道具に数える（`count.rs` 68〜86 行）。まとめた本体も、本番の数に入らない所に置く（要件 1.5）。

## Requirements

### Requirement 1: 落ちの報告の本体を 1 か所にまとめる

**Objective:** As a 開発者, I want 落ちの報告の本体をワークスペースに 1 か所だけ持ちたい, so that 1 か所を直せば全部のテストの実行ファイルに効き、写しの直し忘れで食い違わない

#### Acceptance Criteria

1. The 落ちの報告 shall 例外の受け手・プロセスの寿命の MTA の据え付け・報告の行の組み立てと書き出しの本体を、ワークスペースの中に 1 か所だけ持つ（wintf・areka-placement・areka-runtime に自分の写しが残らない）
2. The 落ちの報告の置き場 shall 本番の依存（`[dependencies]`）にワークスペースの中の crate を持たない（wintf が上位の crate に依存しない向きを壊さない）
3. The 落ちの報告の置き場 shall テスト専用の新しい leaf の crate `crash-report-kit`（`publish = false`・本番の依存は `windows` だけ）とし、ログの捕捉の crate `log-capture-kit` には入れない（`log-capture-kit` の見張り `with_default_guard_test.rs` と「`[dependencies]` へは決して置かない」の規律を緩めない）
4. The 落ちの報告の置き場 shall ワークスペースの外の crate を新しく増やさない（`THIRD-PARTY-NOTICES.md` の差分は、ワークスペースの member として `crash-report-kit` が 1 行載るだけ）
5. The 落ちの報告の置き場 shall 名が `-kit` で終わり、windows-rs の API の数え（`q9-api-count`）がテストの道具に数える（本番の数が変わらない）
6. The まとめた本体 shall 1 ファイル 1,000 行の目安に収まり、1,000 行の番人の例外表に載らない
7. The 落ちの報告の置き場 shall テストだけが使う（使い手は `[dev-dependencies]` から引くか、`areka-runtime` の `test-support` の機能のときだけ付く任意の依存で引く＝前例 `shiori-host32-host`。本番のビルドの依存に現れない）

### Requirement 2: 落ちたときの振る舞いを変えない

**Objective:** As a 開発者, I want まとめた後も、落ちたときに書く中身と書く先が今と同じであってほしい, so that 揺れの調べ（`tools/load-flake.ps1`）と手順がそのまま使える

#### Acceptance Criteria

1. When 落ちの報告を据えたテストの実行ファイルが読み込まれるとき, the 落ちの報告 shall libtest の main より前に、最初の受け手（AV）と最後の受け手（それ以外）を据え、プロセスの寿命の MTA を立てる
2. If プロセスの寿命の MTA を立てられないとき, then the 落ちの報告 shall 今と同じ `AREKA-MTA-KEEPER failed hr=…` の 1 行を標準エラーへ書く
3. When アクセス違反が起きたとき, the 落ちの報告 shall 最初の受け手で 1 度だけ報告を書き、例外を次へ回す（プロセスの振る舞いと終了コードを変えない）
4. When アクセス違反でない例外が捕まえられずに最後の受け手へ届いたとき, the 落ちの報告 shall 報告を書いてから、据える前に在った受け手へ回す
5. The 落ちの報告 shall 報告の行の語・並び・形（例外のコードと番地とスレッドの番号の行・モジュールとスレッドの名前の行・モジュールに当たらない番地の領域の行・生きているスレッドの一覧・積み上げ・1 行の上限での切りと印 ` <cut>`）を今と同じにする
6. The 落ちの報告 shall 報告を標準エラーの OS のハンドルへ 1 行 1 度の書き込みで直に書き、割り当ての無い行を先に書く順を今と同じにする
7. While テストが緑で走っているあいだ, the 落ちの報告 shall 何も書かない

### Requirement 3: 許可の持ち手の表は areka-runtime の使い手だけに残す

**Objective:** As a 開発者, I want GPU の装置の許可の持ち手の表を、使う crate だけが持つ形で残したい, so that 落ちた回にどのテストが GPU の装置の許可を持っていたかを、今と同じく報告から読める

#### Acceptance Criteria

1. When `areka-runtime` の許可の持ち手の表を使うテストの実行ファイル（`areka-runtime`・`areka-input`・本体）で報告を書くとき, the 落ちの報告 shall 今と同じく、表に載っている持ち手ごとの `AREKA-CRASH-HOLDER tid=… name="…"` の行と、表の錠が取れないときの `AREKA-CRASH-HOLDER busy` の行と、入りきらなかった数の `AREKA-CRASH-HOLDER missed=…` の行を書く
2. When wintf と areka-placement のテストの実行ファイルで報告を書くとき, the 落ちの報告 shall 今と同じく `AREKA-CRASH-HOLDER` の行を書かない
3. The 許可の持ち手の表 shall 表と載せ降ろしの入口（`hold`・`release`）をまとめた本体に置き、`areka-runtime` は同じ名（`crash_report_test_support::hold`・`release`）で開く。表に載せる（`hold` を呼ぶ）のは `areka-runtime` の系統（`spine_wait.rs` の `GpuPermit`）だけで、wintf と areka-placement は表を使わない（差し込む口は足さない・決定 2）
4. The 許可の持ち手の表 shall 載せ降ろしの入口（`hold` が行の番号を返し入りきらなければ `usize::MAX`・`release` が行を空ける）の働きと、表の大きさ（16 行・名前 160 バイト）を今と同じにする

### Requirement 4: 使い手を 1 つの本体へ付け替える

**Objective:** As a 開発者, I want 落ちの報告を据える全部のテストの実行ファイルが 1 つの本体を使ってほしい, so that 写しが消えた後も、どのテストの実行ファイルでも落ちの報告が据わっている

#### Acceptance Criteria

1. The wintf・areka-placement・areka-runtime の各テストの実行ファイル shall まとめた本体で落ちの報告を据える
2. The `areka-input` と本体のテストの実行ファイル shall 今と同じ入口（`areka_runtime::crash_report_test_support::install`・`installed`）で、まとめた本体の落ちの報告を据える
3. The `areka-runtime` の背骨の待ちの部品（`spine_wait.rs` の `GpuPermit`） shall 許可を取る所と返す所で、今と同じく持ち手の表へ載せ降ろしする
4. The 使い手の説明（`areka-runtime` の `wire_tests.rs`・`assets_tests.rs`・`assets_shell_tests.rs`・`assets_markers_tests.rs`・`spine_wait.rs` の中で落ちの報告を名で挙げる行） shall まとめた後の置き場を正しく指す
5. The まとめた本体と使い手 shall 「写しの相手」を挙げる説明を残さない（写しが無くなるため）
6. The steering の `structure.md` shall `crash-report-kit` の節（置き場・依存・使い手）を持ち、まとめた後の依存と合う

### Requirement 5: まとめたことを確かめるテスト

**Objective:** As a 開発者, I want 今の写しが持つテストと較正を、まとめた後も同じだけ走らせたい, so that まとめても落ちの報告が据わり、報告の形が変わっていないことを事実で確かめられる

#### Acceptance Criteria

1. The 落ちの報告を据える各テストの実行ファイル（wintf・areka-placement・areka-runtime・areka-input・本体と、まとめた本体の crate 自身） shall 受け手がテストより先に据わっていることを確かめるテストを 1 本ずつ持つ
2. The まとめた本体のテスト shall プロセスの寿命の MTA が立っていること（COM を初期化していない新しいスレッドが暗黙の MTA に居る）を確かめる
3. The まとめた本体のテスト shall 最初の受け手が書くのはアクセス違反だけであること（`OutputDebugString` の例外・C++ の例外は書かない）を確かめる
4. The まとめた本体のテスト shall 1 行の溜めが短い行をそのまま改行つきで出し、長い行を字の境目で切って印を付けること（上限を越えない・切った後の書き足しを捨てる）を確かめる
5. The 較正（わざとプロセスを落とす `#[ignore]` のテスト） shall 今と同じ 2 つの形で走らせられる: 装置を作らずに名前の無いスレッドで落とす形と、`areka-runtime` で許可を取り `GraphicsCore` を作った後に落として持ち手の行にそのテストの名前が出る形
6. When まとめ終えたとき, the 検証 shall ワークスペースの中に例外の受け手を据える本体の定義が 1 つだけであることを grep で確かめて記録に残す（常設の見張りは足さない・決定 7）

### Requirement 6: audit-emo2-boot-frame から預かった areka-placement の件を片づける

**Objective:** As a 開発者, I want `areka-placement` に触るこの spec で、預かった古い注と写しをまとめて直したい, so that 見直しの指摘が取り残されない

#### Acceptance Criteria

1. The `crates/areka-placement/src/follow_window_move_hold_watch_tests.rs` の檻 shall 本番の警告の文（`follow/window_move.rs` 649 行の「整合待ちの札がある窓へ窓書込が到達した」）が変わったら赤になる（手写しの文を、在ることを確かめずに使わない）（申し送りウ）
2. The `areka-placement` の説明 shall `crates/areka-placement/` の中の古い道 `runtime::…`（`areka_runtime::` でないもの・2026-11-21 の測りで 21 ファイル 44 行。名指しの `config.rs` 106〜109 行・`follow/work_area.rs` 27・231 行・`follow_visibility_balloon_wiring_tests.rs` 812・816 行を含む・一覧は `research.md` 2.8 節）を、今在る項目を指す道で書く（例: `areka_runtime::wire_runtime`・メソッドは `RuntimeWiring::seed_zorder_descript_base`・本体のファイルは `crates/areka/src/ghost_session.rs`）。「本体の」の語は古い道と並ぶものだけを直し、本当に `crates/areka/` を指すものは残す（申し送りエ・決定 3）
3. The `areka-placement` の `test_support::zorder_pair_deferred::code_only` と `areka-runtime` の `source_text_test_support::code_only` shall 定義を 1 つにし、今の呼び手（`areka-placement` の `spawn_zorder_chain_wiring_tests.rs` ほか・`areka-runtime` と本体のテスト）は今の道のまま、または付け替えた道で同じ働きを使える（申し送りオ）
4. The `areka-placement` の `follow_window_move_diag_tests.rs` の `window_move_lines` shall 引数を `log_capture_kit::CapturedEvent` でなく文字列の並びに替えて、`test-support` の機能で組んだときにも組める `areka-placement` のテストの支え（`test_support`）へ移り、`areka-placement` のテストはそれを使う（申し送りカ・決定 4）
5. The `crates/areka-placement/src/chain_rule_tests.rs` の 5 行目 shall World の要る行の置き場を、今の `crates/areka-runtime/src/frame_chain_rule_tests.rs` として書く
6. The `crates/areka-placement/src/chain_realign.rs` の `arm_chain_realign` の説明（167〜169 行） shall 今の呼び手（`crates/areka-runtime/src/frame/dpi.rs`）が窓の種類と窓の寸の前後比べだけを見て呼ぶ形と合う（「書込を起こしたときに呼ぶ」「書込が起きたかの判断は呼び手が持つ」を残さない）
7. The 要件 6.2・6.5・6.6 の直し shall 説明だけを直し、コードの働きを変えない
8. When まとめ終えたとき, the 本 spec shall `areka-runtime` の `frame_test_support.rs` 886 行の `window_move_lines` の写し（要件 7.1 の約束で付け替えない）を、持ち主の確かな spec の brief へ申し送るか、完了の未解決の棚卸の件として記録に残す（決定 4）

### Requirement 7: C24 の並走の約束と依存の席

**Objective:** As a 開発者, I want 同じウェーブのほかの spec と触る所が重ならないようにしたい, so that 並走しても合流で食い違わない

#### Acceptance Criteria

1. The 本 spec の変更 shall `crates/areka-runtime/src/frame_test_support.rs` に差分を作らない（`window-resetwindowpos-hold` との約束）
2. The 本 spec の変更 shall `crates/areka-input/` の下のファイルと、本体の `crates/areka/src/runtime_tests/mod.rs` に差分を作らない（`audit-areka-input-menu` の持ち場・入口の名を保つので要らない・決定 8）
3. The 本 spec shall ウェーブに 1 本だけの `Cargo.lock` の席を使い、`Cargo.lock` の差分を `crash-report-kit` の新しい `[[package]]` の塊と、wintf・areka-placement・areka-runtime の項の `dependencies` に 1 行ずつ増える分だけにする
4. The 本 spec の変更 shall `crates/log-capture-kit/tests/file_length_guard_test.rs` に差分を作らない
5. If 直しが要件 7.1・7.2 の約束のファイルに及ぶとわかったとき, then the 本 spec shall そのファイルに触らずに止まり、相手のセッションと開発者へ案を出す

### Requirement 8: 仕上げの確かめ

**Objective:** As a 開発者, I want まとめ終えたことを全体のテストで確かめたい, so that 3 本を 1 本にしても、どのテストも前と同じに通ることを事実で受け取れる

#### Acceptance Criteria

1. When まとめ終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
2. When まとめ終えたとき, the 検証 shall 要件 7.1・7.2・7.4 のファイルに差分が無いことと、`Cargo.lock` の差分が要件 7.3 の分だけであることを、既定のブランチとの差分で確かめて記録に残す
3. When まとめ終えたとき, the 検証 shall 較正の 2 つの形（要件 5.5）を少なくとも 1 度ずつ走らせ、報告の行が要件 2.5・3.1・3.2 のとおりに出ることを記録に残す

## 討議で決めたこと（2026-11-21・開発者「推奨どおり承認」）

本文の「決定 n」はここを指す。

1. **置き場**: テスト専用の新しい leaf の crate `crash-report-kit` に置く（brief の推し ⑴ `log-capture-kit` から変えた）。⑴ では、`areka-input` と本体が `areka-runtime` の `test-support` を通って入口を呼ぶため、`areka-runtime` の `[dependencies]` に `log-capture-kit` を置くことになる。そうなると見張り `crates/log-capture-kit/tests/with_default_guard_test.rs:433` が赤になり、`structure.md` と使い手の `Cargo.toml` の注の規律も緩めることになる（`research.md` 2.3・4 節）。代償は、crate が 1 つ増えること・`Cargo.lock` に新しい塊・`THIRD-PARTY-NOTICES.md` に 1 行・`structure.md` に節。
2. **許可の持ち手の表**: 表と `hold`・`release` も本体に置き、`areka-runtime` は同じ名で開く。差し込む口は足さない（壊れかけのプロセスで走る所に関数ポインタの口を足さない・報告の行は今と同じ）。
3. **申し送りエの広さ**: `areka-placement` の古い道 44 行をすべて直す（指摘は全部直す・C24 で `areka-placement` の説明に触る相手は無い）。
4. **申し送りカ**: 引数を文字列の並びに替えて placement の `test_support` へ移す。`areka-runtime` の写しは C24 の約束で触らず、完了の棚卸で申し送る。
5. **`carrier_cue` の残り**: 範囲の外。完了の棚卸で `dola` のテスト用の組み立ての件として起票する。
6. **`areka-runtime` の側の `INVARIANT_WARNING` の手写し**: 範囲の外。完了の棚卸で、同じファイルの数える檻を確かめる。
7. **写しが戻ることの見張り**: 常設の見張りは足さず、まとめ終えたときに grep で 1 度確かめて記録に残す。
8. **`areka-input` と本体の入口**: `areka_runtime::crash_report_test_support::{install, installed}` の名と働きを保ち、`crates/areka-input/` と本体の `runtime_tests/mod.rs` に触らない（`audit-areka-input-menu` のセッションと合意）。
