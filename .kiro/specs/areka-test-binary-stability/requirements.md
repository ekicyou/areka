# Requirements Document

## Project Description (Input)
開発者と実装の手順（`/kiro-impl` の査読・`/kiro-complete` の全体テスト）は、`areka` のテストの実行ファイル（`cargo test -p areka --bin areka`）を負荷の下で回すと、赤になる・プロセスごと落ちることがあり、偽の赤と本物の欠陥を見分けられない。今は次の件が残っている。

- **件 1**: テストのプロセスが `STATUS_ACCESS_VIOLATION`（`0xc0000005`）で落ちる。`--test-threads=4` で再現し、負荷の下では既定のスレッドの数でも 3 度落ちた（`ghost-session-test-load-flake` の `load-repro.md` の 4.7〜4.9）。どのテストで落ちたかは出ず、原因は分かっていない。落ちる頃に走っていたのは `emo2_boot::frame` の族（各スレッドで MTA を張り、GPU の装置と Compositor を作るテスト）。
- **件 2**: テストの待ちのうち、steering `tech.md` の「テストの待ちと、負荷の下で許す赤」に揃っていないものが 5 種類残る（目印が UI の側の仕事を数えない待ち・`join_bounded` の写し 3 つ・`areka-nar` の短い掴みの檻・上限の無い GPU の許可の待ち・目印の無い `spin_wait_until` の直の呼び出し 15 か所）。
- **件 3**: `cargo test -p wintf --lib` のプロセスが、MTA を張るテストと ASTA の Compositor を作るテストが同時に走るとき `0xc0000005` で落ちる（8 回で 2 回）。
- **件 4**: `ghost_switch_test_support.rs` の検体を組む間の `remove_dir` が io エラーで 1 回赤。
- **件 5**: 起動中の印のテスト 2 本が、並べて走らせたときだけ 1 回ずつ赤。
- ほかに、負荷の下で `spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits` が `［止まった］` で 2 度赤になった観測がある。

変えたいこと: 落ちる組と原因を特定して直し（再現しなければ条件と結果を記録して閉じ）、待ちを steering の線に揃え、最後に負荷つきの全体の走行（5 回）を 1 度だけ回して、許さない赤もプロセスの落ちも無いことを確かめる。正本は `brief.md`（2026-11-01 棚卸㉝の再測定の節を含む）。

## Introduction

`areka` のテストの実行ファイルは、全体テスト（`tools/test-all.ps1`）と実装の手順の査読で毎日回る。完了した `ghost-session-test-load-flake` が、テストの待ちを待ちの部品（`crates/areka/src/emo2_boot/spine_wait.rs`）へ集め、CPU を論理 CPU の 2 倍で埋めた負荷の再現（`tools/load-flake.ps1`）の下で許す赤の線を steering `tech.md` に書いた。その線では、負荷の下でもゴーストの足場と GPU の足場の `［止まった］` だけを許し、待ちの文言の無い赤・`［進みは不明］`・`［進んではいた］`・プロセスの落ちは許さない。

ところが、線に揃っていない待ちと、原因の分からないプロセスの落ちが残っているため、全体テストの赤が出るたびに「負荷のせいか欠陥か」を人が調べ直すことになり、開発の速さ（v0.1.0 のゴール）を直に削っている。落ちたテストの名前が出ないので、スレッドの数を絞って負荷を下げる手も使えない。

本 spec は、測ることが本体の spec である。静かな机（排他テストの机と単独の机）で、ほかの spec と並べずに 1 本だけで進める。件 1 の落ちが件 2 の確かめの回を無効にするので、**件 1 → 件 2 の順**で進め、件 3〜5 は同じ切り分けの手順の中で見る。最後の負荷つきの全体の走行は**全部の件で 1 度だけ**にする。

## Boundary Context

- **In scope**:
  - 件 1・件 3: 落ちたテストを名指す仕掛け・再現の手順・GPU の装置の許可を 1 つにした対照・原因の特定・テストの土台の修正（本番のコードの欠陥が見つかった場合はその修正）・記録。
  - 件 2: 5 種類の待ちと、そのための足場・偽の SHIORI の観測口・檻。GPU の装置を作るのに許可を取らない足場（`emo2_boot/frame_test_support.rs` の `manual_balloon_runtime`）を許可を取る形に揃えること。
  - 件 4・件 5 と `spine_third_scope_talk` の赤: 同じ切り分けの手順での再現・読み分け・足場の側に原因があれば修正。
  - 負荷の再現と、修正の前後の静かな机の全体テストの記録。
- **Out of scope**:
  - 本番のコードの振る舞いのうち、件 1・件 3 の原因でないもの（`areka-nar` の `rename_patiently` の試し直しの間隔と上限を含む）。`#[cfg(test)]` の部分は範囲に入る。
  - 許す赤の線そのものと、その理由の書き換え（要件 4.2 で範囲を広げる分を除く）。
  - 要件 7 の 15 か所の外にある文言の無い待ち（自前の `recv_timeout` の締切・`Instant` の締切・`run_bounded` の呼び手）の直し。数えて記録し、本 spec の走行で赤になったものだけ直す（要件 7.6）。残りは完了のときに `/kiro-discovery` で 1 本に起票する。
  - `tools/test-all.ps1` の並列度の変更。
  - `wintf --test graphics` の一度だけの `STATUS_ACCESS_VIOLATION`（2026-10-04・覚え書きのまま。件 3 は別の実行ファイル＝lib なので範囲に入る）。
  - テストの一時フォルダの置き場（`test-roots-under-target` の持ち物・着地済み）。
  - `areka` の実行ファイルの外にある同じ名前の `join_bounded`（`crates/areka-ghost/tests/ghost/` の 3 本・`crates/areka-kanade/tests/kanade/common/` の 2 本）。steering の待ちの部品と許す赤の線は `areka` の実行ファイルの中のテストを指し、これらの実行ファイルでは負荷の下の赤が観測されていないため（2026-11-01 議題 4 で確認）。
  - `areka-seriko` の `bind::tests::bind_resolver_from_groups_reads_three_kinds_of_scope` の一度だけの赤（`areka` の実行ファイルの外・再現の手がかりが無い）。切り分けの走行の中で再び赤になったときに文言を記録するだけにする。
  - 依存の追加・文字の層のクレートへの変更。既にある `windows` クレートの機能（`Win32_System_Diagnostics_Debug`・`Win32_System_Diagnostics_ToolHelp` など）を `areka`・`wintf` の `Cargo.toml` に足すことは依存の追加に数えない（版は変えない・2026-11-01 議題 3）。
  - WER のダンプの置き場の設定（システムの設定の変更）と、`procdump` などの外の道具の取り寄せ。
- **Adjacent expectations**:
  - 上流の `ghost-session-test-load-flake`（完了）の待ちの部品・打ち切りの文言（`［止まった］`・`［進んではいた］`・`［進みは不明］`・`［相手が居ない］`）・許す赤の線をそのまま使う。
  - 測りの間に、同じテストのファイルや一時の根の置き場を動かす spec（`temp-path-exceptions-to-target`・`install-live-target-hazards`・`background-result-tick-wake`・`balloon-element-order`・`mcp-user-response`・`zorder-chain-residue`）が main に着地しないことを当てにする。
  - 本物の窓が出る試験（`AREKA_EMO2_REAL_RUN=1` の結合テスト・`crates/areka/tests/mcp_get_log_real_run.rs`）とは同じ時間に回さない。
  - **分割**: 要件の段では件 1 の原因が分かっていないため、件 2 を切り出さない。件 1 の原因が本番のコードに在ると分かり、見立てが 1 spec 20 タスクを超える見込みになった時点で、件 2 を別の spec（Opus）へ切り出す（要件 1.8）。

## Requirements

### Requirement 1: 件 1＝`areka` のテストのプロセスの落ちを特定して直す
**Objective:** As a 開発者と実装の手順, I want テストのプロセスが落ちたときにどのテストで落ちたかが分かり、原因が直っていること, so that 落ちを本番の欠陥の兆しと見分けられ、スレッドの数を絞って負荷を下げる手が使える

#### Acceptance Criteria
1. When `areka` のテストのプロセスが `STATUS_ACCESS_VIOLATION` で落ちたとき, the `areka` のテストの実行ファイル shall 落ちたスレッドの名前（＝テストの名前）と呼び出しの積み上げを、少なくとも 1 行、走行の出力か記録のファイルに残す。
2. The 本 spec の作業 shall 1.1 の仕掛けを、再現の走行を始める前に入れ、仕掛けが落ちたテストを名指せることを、再現の走行の前に確かめる。
3. When 再現を始めるとき, the 本 spec の作業 shall 負荷の作り方・スレッドの数・回す範囲・回数・1 回の上限の時間を、始める前に spec の記録へ書き、`emo2_boot::frame` の族（自分で COM と GPU の装置を開くテストのファイルと、2026-11-01 に加わった `frame_scale_notice_integration_tests.rs` を含む）に絞って回す。
4. The 本 spec の作業 shall GPU の装置の許可を同時に 1 つにした対照の走行を、今の数（同時に 4 つ）の走行と同じ条件で採り、落ちの起き方の違いを記録する。
5. When 落ちる組と原因が特定されたとき, the 本 spec の作業 shall 落ちる組・落ちた場所・原因の経路を、コードの定義の名前で spec の記録に書く。
6. If 原因がテストの土台（スレッドに縛られる Windows の資源の扱い・同時に作ってはならない資源の組）に在ると分かったとき, then the 修正 shall 土台を直すか、その族を互いに同時に走らない形にし、確かめの内容とテストの本数を減らさない。
7. If 原因が本番のコードの欠陥だと分かったとき, then the 修正 shall 修正の前に落ちか赤になり修正の後に緑になるテストを 1 本以上添えて、その欠陥を直す。
8. If 原因が本番のコードに在り、本 spec の見立てが 20 タスクを超える見込みになったとき, then the 本 spec の作業 shall 件 2 を別の spec へ切り出すことを開発者へ報告する。
9. If 1.3 で定めた条件と回数で落ちが 1 度も再現しなかったとき, then the 本 spec の作業 shall 試した条件と結果を spec の記録に書いて件 1 を閉じる。
10. The 1.1 の仕掛け shall テストのプロセスの中で完結し、システムの設定の変更や外の道具の取り寄せを要さない。

### Requirement 2: 件 3＝`wintf` の lib のテストのプロセスの落ち
**Objective:** As a 開発者, I want `wintf` の lib のテストが、MTA を張るテストと ASTA の Compositor を作るテストの組で落ちないこと, so that `wintf` に新しいテストを書くとき、落ちを避けるために部屋の種類を選び替えなくてよい

#### Acceptance Criteria
1. The 本 spec の作業 shall `cargo test -p wintf --lib` の、MTA を張るテスト（`ecs::layout::hit_test::tests_ex::alpha_mask_resource` 系・`ecs::widget::bitmap_source::tests` 系）と `com::wuc::tests::begin_draw_roundtrip` を同時に走らせる組を、要件 1 と同じ切り分けの手順（落ちたテストを名指す仕掛け・回数を決めた走行）で回し、結果を件 1 の対照と並べて記録する。
2. If 件 3 の原因が件 1 と同じだと分かったとき, then the 本 spec の作業 shall 件 1 の修正の後に、2.1 の組を、修正の前に落ちを観測したのと同じ条件と回数で回し、落ちが無いことを示す。
3. If 件 3 の原因が件 1 と別だと分かったとき, then the 本 spec の作業 shall 要件 1.5〜1.7 と同じ扱い（記録・土台の修正・本番の欠陥ならテストを添えた修正）で直す。
4. If 2.1 の組を定めた回数回しても落ちが 1 度も再現しなかったとき, then the 本 spec の作業 shall 試した条件と結果を記録して件 3 を閉じる。

### Requirement 3: 目印が UI の側の仕事を数えない待ち（件 2 の項目 1）
**Objective:** As a 開発者, I want 切り替えの取りやめを確かめる待ちが、UI の側の仕事を進みとして数えること, so that 負荷の下の `［止まった］` が本当に止まったことを意味し、30 秒の壁時計の打ち切りと区別できる

#### Acceptance Criteria
1. While 待っている相手が UI の側の仕事（台詞の起動・台詞の表示・定常の知らせ・取りやめの記録）を進めている, the `ghost_switch_while_waiting_wins_and_leaves_no_shell_switch_behind` の 2 つの待ち shall それを進みとして数え、`［止まった］` で打ち切らない。
2. The 修正 shall 目当ての状態に届いたことの確かめ（何が起き、何が起きないか）を弱めない。
3. When 足場の目印を変えたとき, the 修正 shall その目印を使う全てのテストを、同じ確かめの内容のまま新しい目印へ移し、移したテストが静かな机で緑のままであることを示す。

### Requirement 4: `join_bounded` の打ち切りの読み分け（件 2 の項目 2）
**Objective:** As a 開発者, I want スレッドの終わりを待つ `join_bounded`（spine の 1 つと `install/desk_pick_tests.rs`・`install/fetch_url_tests.rs` の写し 2 つ）の打ち切りが、許す赤か欠陥かを文言で読み分けられること, so that 素の World のテストが負荷の下で赤になったとき、許す赤の範囲の字面に当たらない赤を残さない

#### Acceptance Criteria
1. Where 待つ相手のスレッドの中の進みを目印にできる, the `join_bounded` shall その進みを目印にして待ち、相手が進み続けている間は `［止まった］` で打ち切らない。
2. While 負荷の再現（`tools/load-flake.ps1`）の下, the 許す赤 shall `areka` の実行ファイルの `join_bounded`（`desk_pick`・`fetch_url` の素の World のテストを含む）の、進み 0 回の `［止まった］` を含む。進みが 1 回以上ある `［止まった］` は含まない（2026-11-01 開発者の裁定・議題 1: 相手のスレッドが始まれない形は、今の線の理由＝ローダーの錠でスレッドの生まれと消えが列を作ること＝が足場の種類によらず当たるため）。
3. The 本 spec の作業 shall 4.2 の範囲を steering `tech.md` の許す赤の線に書き足す（今の理由は書き換えない）。
4. The 修正 shall 写しが残るときも、どの `join_bounded` も同じ打ち切りの文言の形（何を待っていたか・待った秒数・進み）を出すようにする。

### Requirement 5: `areka-nar` の短い掴みの檻（件 2 の項目 3）
**Objective:** As a 開発者, I want 「別のプロセスが短く掴んでいる行き先でも、移しは最後に確定する」ことを見る檻が負荷で揺れないこと, so that 負荷の下の赤を本番の試し直しの欠陥と取り違えない

#### Acceptance Criteria
1. While 掴みを放す側のスレッドの起動が負荷で遅れている, the `a_destination_held_briefly_by_another_process_still_commits` の檻 shall 放すまでの実時間の長さだけを理由に赤にならない。
2. The 檻 shall 掴みを放す時機を、実時間の経過でなく、移しの試みが掴みに当たったことの観測に結ぶ。
3. The 檻 shall 「掴みに当たった移しを試し直し、掴みが放されたあとに確定する」という確かめの内容を弱めない。
4. The 修正 shall 本番の `rename_patiently` の試し直しの間隔・上限・試し直す対象のエラーを変えない。

### Requirement 6: GPU の装置の許可の待ちと足場の揃え（件 2 の項目 4）
**Objective:** As a 開発者, I want GPU の装置の許可の待ちに上限があり、GPU の装置を作る足場が全部許可を取ること, so that 許可の待ち合いでテストが果てしなく終わらない状態を作らず、許可の数の対照（要件 1.4）が全部の足場に効く

#### Acceptance Criteria
1. The GPU の装置の許可の待ち shall 打ち切りの上限を持つ。
2. If 許可の待ちが上限で打ち切られたとき, then the 許可の待ち shall 許可を待って止まったこと・待った秒数・持たれている許可の数と上限を、文言で分かる形で赤にする。
3. When 許可を 2 つ取るテストどうしが互いに待ち合ったとき, the 許可の待ち shall 6.1 の上限で打ち切って 6.2 の文言で赤にし、その形を檻で確かめる。
4. The GPU の装置を作る全ての足場 shall 装置を作る前に許可を取る（`frame_test_support.rs` の `manual_balloon_runtime` と、それを使う 4 本のテストのファイルを含む）。
5. The 本 spec の作業 shall 6.4 の揃えを、要件 1.4 の許可を 1 つにした対照を採る前に済ませる。
6. If 件 1 の修正が許可の数を変えるとき, then the 本 spec の作業 shall 6.1 の上限の値を、件 1 の修正の後に決める。
7. The 修正 shall 件 1 の修正が求める場合を除き、許可の数（同時に 4 つ）を変えない。

### Requirement 7: 目印の無い待ちを無くす（件 2 の項目 5）
**Objective:** As a 開発者, I want `crates/areka/src` のテストに `［進みは不明］` を出しうる待ちが残らないこと, so that 負荷の下で許さない赤になる待ちが、テストを書き足すたびに残り続けない

#### Acceptance Criteria
1. When 着手したとき, the 本 spec の作業 shall `crates/areka/src` のテストの目印の無い待ち（`spin_wait_until` の直の呼び出しと、目印の既定が無い段の駆動器で段を回す檻の待ち）を数え直し、ファイルと行で spec の記録に書く。
2. Where 待つ相手の進みを数える口がある（偽の SHIORI の呼び出しの数など）, the 該当の待ち shall 目印つきの待ちの部品へ移る。
3. Where 待つ相手の進みを数える口が無い, the 修正 shall 口を足すか、口を足せない理由をつけた例外として spec の記録に残す。
4. When 修正を終えたとき, the `crates/areka/src` のテスト shall 7.3 で例外として記録したものを除き、`［進みは不明］` を出しうる待ちを持たない。
5. The 修正 shall 移した待ちの確かめの内容を弱めない。
6. When 7.1 で数え直すとき, the 本 spec の作業 shall 15 か所の外にある文言の無い待ち（自前の `recv_timeout` の締切・`Instant` の締切・`run_bounded` の呼び手）もファイルと行で記録し、本 spec の走行で赤になったものを目印つきの待ちの部品へ移し、残りを完了のときの起票の材料として残す。

### Requirement 8: 並べて走らせたときだけの赤（件 4・件 5・`spine_third_scope_talk`）
**Objective:** As a 開発者, I want 並べて走らせたときだけ出る赤の原因が分かり、足場の側の原因が直っていること, so that 全体テストの単発の赤を回し直しで流さずに済む

#### Acceptance Criteria
1. The 検体を組む足場（`ghost_switch_test_support.rs` の、中身を移した直後のフォルダを消す所）shall 移した直後のフォルダの一時的な握りで検体の組み立てを失敗にしない。
2. When 件 4 を直すとき, the 本 spec の作業 shall 誰がフォルダを握っていたかの見立てとその根拠を spec の記録に書く。
3. When 着手したとき, the 本 spec の作業 shall 件 5 の 2 本（`first_boot_falling_back_pins_mark_with_one_warn`・`switch_marks_unwritable_end_with_write_failed_and_no_reservation`）の赤の文を、並べて走らせる走行で採り直して記録する。
4. If 件 5 の原因が並走する別のテストとの重なり（記録の捕まえ・印のファイルの置き場など）だと分かったとき, then the 修正 shall 並走しても互いに妨げない形に直す。
5. When 負荷の下で `spine_third_scope_talk_runs_to_the_end_and_the_farewell_quits` が赤になったとき, the 本 spec の作業 shall その赤が steering の許す赤に当たるか、足場の側の止まりかを文言と目印から読み分けて記録する。
6. If 8.5 の赤が足場の側の止まりだと分かったとき, then the 修正 shall その止まりを直す。
7. If 件 4・件 5 を定めた回数の走行で再現できなかったとき, then the 本 spec の作業 shall 試した条件と結果を記録して閉じる。

### Requirement 9: 測り方と最後の確かめ
**Objective:** As a 開発者, I want 測りが静かな机で、重い走行を最小の回数に抑えて行われ、最後に 1 度だけ全部の件がまとめて確かめられること, so that ほかの仕事を汚さず汚されずに、直ったことを信じられる

#### Acceptance Criteria
1. The 本 spec の作業 shall 測りを静かな机（排他テストの机と単独の机）で行い、測る前に開発機の机の道具が main の最新と揃っていることを確かめる。
2. When 重い走行（全体テスト・負荷の再現）を回す前, the 本 spec の作業 shall 所要時間の見込みを開発者へ伝える。
3. While 修正の途中, the 本 spec の作業 shall 2 回目からの高負荷の走行を、赤が出たテストに絞って回す。
4. The 本 spec の作業 shall 修正の前と後に静かな机で全体テストを回し、全体と `areka` のテストの実行ファイルの所要時間を spec の記録に並べる（修正の前の基準は、`test-roots-under-target` の着地の後の main で採る）。
5. If 測りの間に Adjacent expectations に挙げた spec が main に着地したとき, then the 本 spec の作業 shall 比べの基準を採り直す。
6. When 全ての修正の後に負荷つきの全体の走行（5 回）を 1 度だけ回したとき, the `areka` のテストの実行ファイル shall プロセスの落ちと許さない赤を 1 件も出さずに終える。
7. If 9.6 の走行で赤が出たとき, then the 本 spec の作業 shall どの赤も文言から読み分けて記録し（回し直しの緑で置き換えない）、許さない赤かプロセスの落ちが 1 件でもあれば完了としない。
8. When 修正の後に静かな机で全体テスト（`tools/test-all.ps1`）を回したとき, the 全体テスト shall ワークスペース全体の見張りを含めて緑で終わる。
9. The 修正 shall 静かな机の `areka` のテストの実行ファイルの所要時間を、修正の前の基準の 1.20 倍（`ghost-session-test-load-flake` が許可の数を選んだときの線）より延ばさない。
10. The 再現の手順 shall 一時のファイルと負荷の子プロセスの作業場所をワークツリーの `target\` の下だけに置き、終わったときに自分が起こした負荷の子プロセスを止める。
11. If プロセスの落ち（件 1・件 3）を直すのに 9.9 の線を越える見込みになったとき, then the 本 spec の作業 shall その場で止まり、測った所要時間と直しの案を開発者へ示して裁定を受ける（線を黙って越えない・2026-11-01 議題 5）。

### Requirement 10: 直し方の決まりと共通の見張り
**Objective:** As a 開発者, I want 直しがテストの本質を外さず、共通の見張りを緑のまま保つこと, so that 赤を消したことが本当に直ったことを意味する

#### Acceptance Criteria
1. The 修正 shall 一定時間の sleep を足すこと・締切の秒数を延ばすことだけ・1 フレーム遅らせること・無視の印を付けること・テストの本数を減らすことのいずれでも赤や落ちを消さない（要件 1.2 の較正のためにわざと落とすテストを、ふだんの走行で回らない形にすることは、赤を消すことに当たらない）。
2. The 修正 shall GPU のテストを実のハードウェア GPU のまま回し、WARP などのソフトウェアの描画へ替えない。
3. The 修正 shall 本番のファイルの振る舞いを、件 1・件 3 で本番の欠陥が見つかった場合と `#[cfg(test)]` の部分を除いて変えない。
4. The 修正 shall 触ったすべてのファイルを 1,000 行以下に保ち、1 ファイル 1,000 行の番人の例外表に行を足さない。上限に近いファイル（`spine_conformance_support_tests.rs`・`frame_test_support.rs`・`spine_conformance_lap_tests.rs`）は、足す前に分け、`main.rs` には足さない。
5. The 修正 shall テスト用の一時パスを共通の一時パスの窓口を通して作り、その迂回の見張りを緑のまま保つ。
6. Where 待ちの部品の形を変える, the 修正 shall その部品を使う全てのテストを、同じ確かめの内容のまま新しい形へ移す。
7. If 件 1・件 3 の修正が、スレッドに縛られる資源を使うテストの書き方の新しい決まりを生んだとき, then the 本 spec の作業 shall その決まりを steering `tech.md` のテストの待ちと足場の節に書き足す（許す赤の理由は書き換えない）。
