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
  - 3 本の写しを 1 本にまとめること・その置き場の crate の `Cargo.toml` と `Cargo.lock` の該当の行。
  - 3 つの crate（wintf・areka-placement・areka-runtime）の `mod` の宣言と、使い手（`areka-runtime` の `spine_wait.rs`、説明の中で名を挙げる `wire_tests.rs`・`assets_tests.rs`・`assets_shell_tests.rs`・`assets_markers_tests.rs`、本体の `crates/areka/src/runtime_tests/mod.rs`）の付け替え。
  - まとめたことを確かめるテスト（要件 5）。
  - `audit-emo2-boot-frame` から預かった `areka-placement` の件（要件 6）。
  - 置き場の crate の依存の説明（steering の `structure.md` の該当の節）を実物に合わせること。
- **Out of scope**:
  - 落ちの報告の中身や書く先を変えること。
  - `write_thread_name` が `GetThreadDescription` の返した文字列を `LocalFree` しない件（`crash_report_test_support.rs` 433 行の `shortcut:` の注・意図的な先送り・`audit-emo2-boot-spine` の research 5 節 ⑵）。
  - windows-rs の移行そのもの。
  - `carrier_cue` の写しのうち `crates/areka-ghost/src/prop_sink.rs`（118 行）と `crates/areka-seriko/src/actor_test_support.rs`（124 行）。合図の型 `TalkCue`・`CueCommand` は `dola`（`crates/dola/src/cue/command.rs` の 99・288 行）にあり、落ちの報告の置き場（ワークスペースの中の crate に依存しない leaf）へは、その leaf が `dola` に依存しない限り寄せられない＝範囲の外のまま（仮定 A3・議題 3）。
  - 申し送りに名指されていない `code_only` の写し（wintf の 8 本・`areka-emo-present` の 1 本・本体の 2 本。本体の `tick_gate_config_producers_tests.rs` の分は `audit-areka-app` の持ち物）と、`areka-placement` の `measure_template::code_only`（ブロックコメントも落とす別の働きで、写しではない）。
  - `areka-runtime` の `frame_work_area_resnap_hold_tests.rs` 60 行の `INVARIANT_WARNING` の手写し（申し送りに名指されていない・`areka-runtime` の `frame*` の側。仮定 A7・議題 7）。
  - 1,000 行の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs` の例外表。
- **Adjacent expectations**:
  - `crates/areka-input/src/crash_report_tests.rs`（5・10 行）は `areka_runtime::crash_report_test_support::{install, installed}` で落ちの報告を据えている（`input-crate-split` PR#486 で足された 4 本目の使い手・brief の Current State には載っていない）。同じウェーブの `audit-areka-input-menu` が `crates/areka-input/` を受け持つので、この spec はその入口の名と働きを変えず、`crates/areka-input/` のファイルに触らない（仮定 A5・議題 5）。
  - 本体の `runtime_tests/mod.rs` は `session-module-gather` が触らない約束なので、この spec が付け替えてよい。
  - `audit-areka-app` は本体の `code_only` の写しを `areka_runtime::source_text_test_support::code_only` を使う形に寄せる。この spec の要件 6.3 の直しでも、その道で `code_only` が引ける形を保つ。
  - windows-rs の API の数え（`crates/pilot/examples/pilot-windows-rs-next/q9-api-count`）は、`*_test_support`・`test_*` の名のファイルと、名が `-kit` で終わる crate をテストの道具に数える（`count.rs` 68〜86 行）。まとめた本体も、本番の数に入らない所に置く（要件 1.5）。

## Requirements

### Requirement 1: 落ちの報告の本体を 1 か所にまとめる

**Objective:** As a 開発者, I want 落ちの報告の本体をワークスペースに 1 か所だけ持ちたい, so that 1 か所を直せば全部のテストの実行ファイルに効き、写しの直し忘れで食い違わない

#### Acceptance Criteria

1. The 落ちの報告 shall 例外の受け手・プロセスの寿命の MTA の据え付け・報告の行の組み立てと書き出しの本体を、ワークスペースの中に 1 か所だけ持つ（wintf・areka-placement・areka-runtime に自分の写しが残らない）
2. The 落ちの報告の置き場 shall ワークスペースの中のどの crate にも依存しない（wintf が上位の crate に依存しない向きを壊さない）
3. Where 落ちの報告の置き場が既にほかの目的で使われている crate であるとき, the 落ちの報告の置き場 shall 落ちの報告を使わない使い手に、落ちの報告のための依存（Win32 の関数を呼ぶ crate）を新しく引かせない
4. The 落ちの報告の置き場 shall ワークスペースの外の crate を新しく増やさない（`THIRD-PARTY-NOTICES.md` が変わらない）
5. The 落ちの報告の置き場 shall windows-rs の API の数え（`q9-api-count`）がテストの道具に数える所である（本番の数が変わらない）
6. The まとめた本体 shall 1 ファイル 1,000 行の目安に収まり、1,000 行の番人の例外表に載らない
7. The 落ちの報告の置き場 shall テストだけが使う（本番のビルドの依存に現れない）

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
3. The 許可の持ち手の表 shall `areka-runtime` だけが持ち、wintf と areka-placement は表を持たない
4. The 許可の持ち手の表 shall 載せ降ろしの入口（`hold` が行の番号を返し入りきらなければ `usize::MAX`・`release` が行を空ける）の働きと、表の大きさ（16 行・名前 160 バイト）を今と同じにする

表の置き方（共通の本体に表を差し込む口を足すか、`areka-runtime` の側に薄く残すか）は設計の段で決める（仮定 A2）。

### Requirement 4: 使い手を 1 つの本体へ付け替える

**Objective:** As a 開発者, I want 落ちの報告を据える全部のテストの実行ファイルが 1 つの本体を使ってほしい, so that 写しが消えた後も、どのテストの実行ファイルでも落ちの報告が据わっている

#### Acceptance Criteria

1. The wintf・areka-placement・areka-runtime の各テストの実行ファイル shall まとめた本体で落ちの報告を据える
2. The `areka-input` と本体のテストの実行ファイル shall 今と同じ入口（`areka_runtime::crash_report_test_support::install`・`installed`）で、まとめた本体の落ちの報告を据える
3. The `areka-runtime` の背骨の待ちの部品（`spine_wait.rs` の `GpuPermit`） shall 許可を取る所と返す所で、今と同じく持ち手の表へ載せ降ろしする
4. The 使い手の説明（`areka-runtime` の `wire_tests.rs`・`assets_tests.rs`・`assets_shell_tests.rs`・`assets_markers_tests.rs`・`spine_wait.rs` の中で落ちの報告を名で挙げる行） shall まとめた後の置き場を正しく指す
5. The まとめた本体と使い手 shall 「写しの相手」を挙げる説明を残さない（写しが無くなるため）
6. The 置き場の crate の依存の説明（steering の `structure.md` の該当の節） shall まとめた後の依存と合う

### Requirement 5: まとめたことを確かめるテスト

**Objective:** As a 開発者, I want 今の写しが持つテストと較正を、まとめた後も同じだけ走らせたい, so that まとめても落ちの報告が据わり、報告の形が変わっていないことを事実で確かめられる

#### Acceptance Criteria

1. The 落ちの報告を据える各テストの実行ファイル（wintf・areka-placement・areka-runtime・areka-input・本体） shall 受け手がテストより先に据わっていることを確かめるテストを 1 本ずつ持つ
2. The まとめた本体のテスト shall プロセスの寿命の MTA が立っていること（COM を初期化していない新しいスレッドが暗黙の MTA に居る）を確かめる
3. The まとめた本体のテスト shall 最初の受け手が書くのはアクセス違反だけであること（`OutputDebugString` の例外・C++ の例外は書かない）を確かめる
4. The まとめた本体のテスト shall 1 行の溜めが短い行をそのまま改行つきで出し、長い行を字の境目で切って印を付けること（上限を越えない・切った後の書き足しを捨てる）を確かめる
5. The 較正（わざとプロセスを落とす `#[ignore]` のテスト） shall 今と同じ 2 つの形で走らせられる: 装置を作らずに名前の無いスレッドで落とす形と、`areka-runtime` で許可を取り `GraphicsCore` を作った後に落として持ち手の行にそのテストの名前が出る形
6. When まとめ終えたとき, the 検証 shall ワークスペースの中に例外の受け手を据える本体の定義が 1 つだけであることを確かめて記録に残す（常設の見張りは足さない・仮定 A9・議題 9）

### Requirement 6: audit-emo2-boot-frame から預かった areka-placement の件を片づける

**Objective:** As a 開発者, I want `areka-placement` に触るこの spec で、預かった古い注と写しをまとめて直したい, so that 見直しの指摘が取り残されない

#### Acceptance Criteria

1. The `crates/areka-placement/src/follow_window_move_hold_watch_tests.rs` の檻 shall 本番の警告の文（`follow/window_move.rs` 649 行の「整合待ちの札がある窓へ窓書込が到達した」）が変わったら赤になる（手写しの文を、在ることを確かめずに使わない）（申し送りウ）
2. The `areka-placement` の説明 shall `config.rs`（106〜109 行）・`follow/work_area.rs`（27・231 行）・`follow_visibility_balloon_wiring_tests.rs`（812・816 行）の古い道 `runtime::frame::…` と「本体の」を、今の置き場 `areka_runtime::frame::…` で書く（申し送りエ・名指された行の外は仮定 A6・議題 6）
3. The `areka-placement` の `test_support::zorder_pair_deferred::code_only` と `areka-runtime` の `source_text_test_support::code_only` shall 定義を 1 つにし、今の呼び手（`areka-placement` の `spawn_zorder_chain_wiring_tests.rs` ほか・`areka-runtime` と本体のテスト）は今の道のまま、または付け替えた道で同じ働きを使える（申し送りオ）
4. The `areka-placement` の `follow_window_move_diag_tests.rs` の `window_move_lines` shall `areka-runtime` からも引ける `areka-placement` のテストの支え（`test-support`）へ移る（申し送りカ・仮定 A4）
5. The `crates/areka-placement/src/chain_rule_tests.rs` の 5 行目 shall World の要る行の置き場を、今の `crates/areka-runtime/src/frame_chain_rule_tests.rs` として書く
6. The `crates/areka-placement/src/chain_realign.rs` の `arm_chain_realign` の説明（167〜169 行） shall 今の呼び手（`crates/areka-runtime/src/frame/dpi.rs`）が窓の種類と窓の寸の前後比べだけを見て呼ぶ形と合う（「書込を起こしたときに呼ぶ」「書込が起きたかの判断は呼び手が持つ」を残さない）
7. The 要件 6.2・6.5・6.6 の直し shall 説明だけを直し、コードの働きを変えない
8. When まとめ終えたとき, the 本 spec shall `areka-runtime` の `frame_test_support.rs` 886 行の `window_move_lines` の写し（要件 7.1 の約束で付け替えない）を、持ち主の確かな spec の brief へ申し送るか、完了の未解決の棚卸の件として記録に残す（仮定 A4）

### Requirement 7: C24 の並走の約束と依存の席

**Objective:** As a 開発者, I want 同じウェーブのほかの spec と触る所が重ならないようにしたい, so that 並走しても合流で食い違わない

#### Acceptance Criteria

1. The 本 spec の変更 shall `crates/areka-runtime/src/frame_test_support.rs` に差分を作らない（`window-resetwindowpos-hold` との約束）
2. The 本 spec の変更 shall `crates/areka-input/` の下のファイルに差分を作らない（`audit-areka-input-menu` の持ち場・仮定 A5）
3. Where 置き場の crate に依存を足すとき, the 本 spec shall ウェーブに 1 本だけの `Cargo.lock` の席を使い、`Cargo.lock` の差分を置き場の crate の行だけにする
4. The 本 spec の変更 shall `crates/log-capture-kit/tests/file_length_guard_test.rs` に差分を作らない
5. If 直しが要件 7.1・7.2 の約束のファイルに及ぶとわかったとき, then the 本 spec shall そのファイルに触らずに止まり、相手のセッションと開発者へ案を出す

### Requirement 8: 仕上げの確かめ

**Objective:** As a 開発者, I want まとめ終えたことを全体のテストで確かめたい, so that 3 本を 1 本にしても、どのテストも前と同じに通ることを事実で受け取れる

#### Acceptance Criteria

1. When まとめ終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 緑で終わる
2. When まとめ終えたとき, the 検証 shall 要件 7.1・7.2・7.4 のファイルに差分が無いことと、`Cargo.lock` の差分が要件 7.3 の分だけであることを、既定のブランチとの差分で確かめて記録に残す
3. When まとめ終えたとき, the 検証 shall 較正の 2 つの形（要件 5.5）を少なくとも 1 度ずつ走らせ、報告の行が要件 2.5・3.1・3.2 のとおりに出ることを記録に残す

## 仮定と議題（要件の討議で決める）

要件の討議で決めるまでの仮置き。本文の「仮定 An」はここを指す。

- **A1（置き場）**: 置き場は ⑴ `log-capture-kit` に新しいファイルとして入れ、`windows`（ワークスペースの依存・`Cargo.lock` に在る）を既定では引かない任意の依存で足す、と仮に置く（brief の推し）。要件 1 は ⑴ と ⑵（テスト専用の新しい leaf の crate）のどちらでも満たせる形で書いた。
- **A2（許可の持ち手の表）**: 置き方は設計の段で決める（要件 3.5）。要件は結果（表は `areka-runtime` だけ・行の形は今と同じ）だけを決める。
- **A3（`carrier_cue` の残り）**: 落ちの報告の置き場へは寄せない（範囲の外のまま）と置く。
- **A4（申し送りカ）**: `areka-placement` の側の写しを `areka-placement` のテストの支えへ移すところまでを行い、`areka-runtime` の `frame_test_support.rs` の写しの付け替えは申し送る、と置く。
- **A5（`areka-input` の使い手）**: 入口 `areka_runtime::crash_report_test_support::{install, installed}` の名と働きを保ち、`crates/areka-input/` に触らない、と置く。
- **A6（申し送りエの広さ）**: 名指された 3 ファイル 6 行だけを直す、と置く。
- **A7（`areka-runtime` の側の `INVARIANT_WARNING` の写し）**: 範囲の外、と置く。
- **A8（本体の `runtime_tests/mod.rs`）**: 入口の道を保つ（A5）なら、本体の `runtime_tests/mod.rs` は変わらないことがある。brief の「触るファイル」に載っているが、触らずに済めば触らない、と置く。
- **A9（写しが戻ることの見張り）**: 常設の見張りは足さず、まとめ終えたときに 1 度確かめて記録に残す、と置く。
