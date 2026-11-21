# Implementation Plan

## Task Overview

テストのプロセスの落ちの報告の 3 本の写し（wintf・areka-placement・areka-runtime）を、テスト専用の新しい leaf の crate `crash-report-kit` の 1 本にまとめる。`areka-runtime` の `crash_report_test_support` は同じ名で開く薄い入口に替え、`areka-input`・本体・`spine_wait.rs` は差分 0 のまま新しい本体を使う。あわせて `audit-emo2-boot-frame` から預かった `areka-placement` の古い注と写しを直す。

- 並走の約束（C24）: `crates/areka-runtime/src/frame_test_support.rs`・`crates/areka-input/` の全部・`crates/areka/src/runtime_tests/mod.rs`・`crates/log-capture-kit/tests/file_length_guard_test.rs` には触らない。`crates/areka-placement/src/spawn.rs` は 350 行の説明だけを直す（末尾は `audit-areka-input-menu` が足す）。直しがこれらに及ぶとわかったら、触らずに止まり、相手のセッションと開発者へ案を出す（要件 7.5）。
- `Cargo.lock` は `cargo` に書かせ、手で直さない。

---

## Tasks

- [ ] 1. 置き場の crate と依存の行
- [x] 1.1 `crash-report-kit` を作り、落ちの報告の本体を移す
  - 新しい crate の `Cargo.toml` を設計の KitManifest のとおりに置く（`publish = false # テストだけが使う道具`・`[dependencies]` は `windows = { workspace = true }` だけ・`[features]` と `[dev-dependencies]` は持たない）
  - `areka-runtime` の今の本文（受け手 2 つ・MTA・持ち手の表・報告の行・1 行の溜め）を `lib.rs` へ移し、設計の CrashReportKit の「変えるのは次だけ」の分だけを変える（頭の `//!` を 1 本の説明に・`RaiseException` の宣言の `#[cfg(test)]` を外す・較正の口 `crash_on_an_unnamed_thread` を足す・持ち手の表の説明を「使い手が許可を取る・返す所から」に言い換える）。行の語・並び・書く先・`LINE_BYTES`・`CUT_MARK`・`HOLDER_ROWS`・`NAME_BYTES` は変えない
  - 隣の `lib_tests.rs` に、この crate 自身の据え付けの静的変数・檻 4 本（据え付け・MTA・最初の受け手は AV だけ・1 行の溜め）・装置なしの較正 1 本（`#[ignore]`）を移す
  - 旧の 3 本の写しはこのタスクでは消さない（タスク 2 で付け替える）
  - 新しい crate を組むと `cargo` が `Cargo.lock` に `crash-report-kit` の塊を書く（手で直さない・このタスクの差分に含める）
  - 終わった姿: `cargo test -p crash-report-kit` が緑で、4 本が通り較正 1 本が ignored と出る。`lib.rs` は 1,000 行に収まる。`cargo tree -p crash-report-kit -e normal` に出るワークスペースの crate が 0
  - _Dispatch: mechanical — 今ある本文を設計の示す差分だけで移す_
  - _Requirements: 1.1, 1.2, 1.3, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5_

- [x] 1.2 使い手 3 crate の依存の行を足す
  - wintf・areka-placement の `[dev-dependencies]` に `crash-report-kit` を path で 1 行ずつ（注は「テストの実行ファイルの落ちの報告の本体。テスト専用の leaf で、`[dependencies]` へは置かない。」の趣旨・spec の名を書かない）
  - `areka-runtime` は `[dependencies]` に任意の依存として置き `test-support` の機能に `"dep:crash-report-kit"` を足し、`[dev-dependencies]` にも置く（`shiori-host32-host` と同じ並べ方・注は設計の ConsumerManifests の趣旨）
  - `cargo` に `Cargo.lock` を書かせる
  - 終わった姿: `cargo tree -p areka-runtime -e normal` に `crash-report-kit` が出ず、`cargo tree -p areka-runtime -e normal --features test-support` に出る。`git diff origin/main -- Cargo.lock`（1.1 と合わせた累積）が `crash-report-kit` の新しい塊 1 つと wintf・areka-placement・areka-runtime の `dependencies` の 1 行ずつだけ
  - _Dispatch: mechanical — マニフェストの行を前例どおりに足す_
  - _Requirements: 1.4, 1.7, 7.3_

- [ ] 2. 使い手を 1 つの本体へ付け替える
- [x] 2.1 (P) wintf の据え付けを小さなファイルに替える
  - `crates/wintf/src/test_crash_report.rs` の中身を、`crates/areka-input/src/crash_report_tests.rs` と同じ形（`//!` 1 行・`crash_report_kit::install` を指す `.CRT$XCU` の静的変数・据え付けの確かめ 1 本）に替える。ファイルの名と `lib.rs` の `mod` の行は変えない
  - 「写しの相手」を挙げる説明を残さない
  - 終わった姿: `cargo test -p wintf --lib test_crash_report` で据え付けの確かめが緑。`crates/wintf/src/lib.rs` の差分が 0
  - _Boundary: CrateInstaller（wintf）_
  - _Dispatch: mechanical — areka-input の既存のファイルと同じ形に替える_
  - _Requirements: 2.1, 3.2, 4.1, 4.5, 5.1_

- [x] 2.2 (P) areka-placement の据え付けを小さなファイルに替える
  - `crates/areka-placement/src/test_crash_report.rs` を 2.1 と同じ形に替える。`lib.rs` の `mod test_crash_report;` の行とその説明は変えない
  - 終わった姿: `cargo test -p areka-placement --lib test_crash_report` で据え付けの確かめが緑
  - _Boundary: CrateInstaller（areka-placement の test_crash_report.rs だけ）_
  - _Dispatch: mechanical — areka-input の既存のファイルと同じ形に替える_
  - _Requirements: 2.1, 3.2, 4.1, 4.5, 5.1_

- [x] 2.3 (P) areka-runtime の入口を同じ名の再公開に替える
  - `crates/areka-runtime/src/crash_report_test_support.rs` を、設計の RuntimeEntry の形（`crash_report_kit` の `hold`・`install`・`installed`・`release` の再公開・`#[cfg(test)]` の据え付けの静的変数・据え付けの確かめ 1 本・装置ありの較正 1 本）だけにする。装置ありの較正は許可を取り `GraphicsCore` を作ってから `crash_report_kit::crash_on_an_unnamed_thread()` で落とす
  - `lib.rs` の `pub mod` の宣言・`spine_wait.rs`・説明で名を挙げる 4 ファイルは変えない
  - 終わった姿: `cargo test -p areka-runtime --lib crash_report_test_support` が緑（較正は ignored）。`cargo test -p areka-input --lib crash_report` と `cargo test -p areka --lib runtime_tests::crash_filter_is_installed_before_the_tests_start` が差分 0 のまま緑（定義が 1 つになったことの grep は 5.1 で記録する）
  - _Boundary: RuntimeEntry_
  - _Dispatch: mechanical — 設計が入口の中身を示している_
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.5, 7.2_

- [ ] 3. areka-placement の預かりの件
- [x] 3.1 (P) `code_only` の定義を 1 つにする
  - 定義を `areka-placement` の `test_support` の直下へ上げ、`zorder_pair_deferred` には同じ名の `pub use` を残す（placement の呼び手は差分 0）
  - `zorder_pair_deferred` の説明にある「`measure_template::code_only` とは中身が違うので寄せない」の注を、上げた定義の側へ移す（コメントだけ）
  - `areka-runtime` の `source_text_test_support` の定義を、placement の `test_support::code_only` の `pub use` に替える（runtime・本体の呼び手は差分 0）
  - 終わった姿: `git grep -n "fn code_only" -- crates/areka-placement/src/test_support.rs crates/areka-runtime/src/source_text_test_support.rs` が placement の 1 件だけ。`code_only` を使う placement・runtime・本体のテストが組めて緑
  - _Boundary: PlacementSupportMerge（placement の test_support.rs・runtime の source_text_test_support.rs）_
  - _Dispatch: mechanical — 設計が置き場と再公開の形を示している_
  - _Requirements: 6.3_

- [ ] 3.2 `window_move_lines` を placement のテストの支えへ移す
  - 3.1 と同じ `test_support.rs` を触るので、3.1 の後に続けて行う
  - 引数を記録の文（`&str`）の並びに替えた `window_move_lines` を placement の `test_support` に足す。`log-capture-kit` を引かない
  - `follow_window_move_diag_tests.rs` の私有の写しを消し、呼び手 7 か所を記録の文の並びを渡す形に替える（`follow::test_support` と取り違えない）
  - `crates/areka-runtime/src/frame_test_support.rs` の写しには触らない
  - 終わった姿: `follow_window_move_diag_tests` の既存のテストが今と同じに緑。`cargo check -p areka-placement --features test-support` が通る。`frame_test_support.rs` の差分が 0
  - _Dispatch: mechanical — 今の本文の引数を替えて移す_
  - _Requirements: 6.4, 7.1_

- [ ] 3.3 整合待ちの見張りの檻を、警告の文が変わったら赤になる形にする
  - 3.1・3.2 の後に行う（摂動の確かめで `follow/window_move.rs` を一時に変え、同じ作業場で `areka-placement` を組むため並走できない）
  - 鳴る側の 1 本 `a_system_write_that_bypasses_its_deferral_still_trips_the_watch` を、捕捉の中で `catch_unwind` し警告の文をちょうど 1 件と数えてから panic を戻す形にする（前例 `crates/areka-runtime/src/frame_work_area_resnap_hold_tests.rs` の同じ働きのテスト）。`#[should_panic]` と release での ignore は残す
  - 終わった姿: debug の走行でこの 1 本が緑。`follow/window_move.rs` の警告の文を一時的に 1 字変えるとこの 1 本が赤になり、戻すと緑に戻る（一時の変更はコミットしない）。タスクを閉じる前に `git diff -- crates/areka-placement/src/follow/window_move.rs` が空
  - _Dispatch: mechanical — 既存の隣の前例の形を写す_
  - _Requirements: 6.1_

- [ ] 3.4 古い道と置き場・武装の説明を直す
  - 3.3 の一時の変更が戻った後に行う（`follow/window_move.rs` を両方が触るため）
  - `crates/areka-placement/` の下の古い道 `runtime::…`（21 ファイル 44 行・一覧は research の 2.8 節）を、設計の「古い道の書き換えの決まり」で直す。行ごとに、書き換え後の道の最後の項目が今その置き場に定義されていることを grep で確かめる。本当に `crates/areka/` を指す「本体の」は残す。`spawn.rs` は 350 行だけ
  - `chain_rule_tests.rs` 5 行目の置き場と、`chain_realign.rs` の `arm_chain_realign` の説明（160・167〜169 行）を設計の趣旨で書き直す
  - コメントだけを直し、コードは変えない
  - 終わった姿: `crates/areka-placement/` の中で、`areka_` が前に付かない `runtime::` の道を探す grep が 0 件。`git diff -- crates/areka-placement` の増減の行のうちこのタスクの分がすべてコメントの行。`cargo test -p areka-placement` が緑
  - _Dispatch: standard — 44 行それぞれの今の置き場を調べて書く_
  - _Requirements: 6.2, 6.5, 6.6, 6.7_

- [ ] 4. (P) steering の `structure.md` を実物に合わせる
  - 「Temp Path Kit Crate」の節の後に、設計の SteeringDoc の 5 項目（Location・Purpose・Modules・Dependencies・Consumers）で `crash-report-kit` の節を足す
  - runtime の節の Dependencies の任意の依存の句に `crash-report-kit` を並べる
  - 終わった姿: `structure.md` に `crash-report-kit` の節があり、書いた依存と使い手が 1.2 の `Cargo.toml` と合う
  - _Boundary: SteeringDoc_
  - _Depends: 1.2_
  - _Dispatch: mechanical — 設計が節の中身を示している_
  - _Requirements: 4.6_

- [ ] 5. 仕上げの確かめと記録
- [ ] 5.1 静的な確かめを走らせて記録する
  - 設計の CompletionRecord の 5.6・6.7・6.8・7.1・7.2・7.4・7.5・8.2 の確かめを走らせ、結果を tasks.md の Implementation Notes に書く（`fn SetUnhandledExceptionFilter` が 1 件・`.CRT$XCU` が 6 件・約束のファイルの差分が空・`Cargo.lock` の差分の形・預かりの説明の直しがコメントの行だけ）
  - `frame_test_support.rs:886` の `window_move_lines` の写しは付け替えていない、と記録し、完了の未解決の棚卸の件として残す（寄せ先は placement の `test_support::window_move_lines`）。`carrier_cue` の残り（決定 5）と runtime の `INVARIANT_WARNING` の手写し（決定 6）も同じ棚卸の件として並べる
  - 終わった姿: Implementation Notes に各確かめのコマンドと出力の要点が並び、どれも期待どおり
  - _Dispatch: mechanical — 設計が確かめのコマンドと期待を示している_
  - _Requirements: 5.6, 6.7, 6.8, 7.1, 7.2, 7.3, 7.4, 7.5, 8.2_

- [ ] 5.2 較正の 2 つの形を走らせて記録する
  - 装置なし（`crash-report-kit` の較正）と装置あり（`areka-runtime` の較正）を 1 度ずつ走らせる。装置ありは本物の GPU を使うので、排他のテストの机（`areka-impl-watch`）を取ってから走らせ、終わったらすぐ返す
  - 終わった姿: どちらもプロセスが落ち、装置なしは `AREKA-CRASH code=0xC0000005`・`AREKA-CRASH-THREAD`・`AREKA-CRASH-STACK` が出て `AREKA-CRASH-HOLDER` が出ず、装置ありは加えて `AREKA-CRASH-HOLDER tid=… name="…crash_report_names_the_running_test"` が出る。出た行を Implementation Notes に写す
  - _Dispatch: mechanical — 設計が走らせ方と期待の行を示している_
  - _Requirements: 2.5, 3.1, 3.2, 5.5, 8.3_

- [ ] 5.3 全体テストを緑で通し、ライセンスの一覧を作り直す
  - `pwsh -NoProfile -File tools/test-all.ps1 -License` を 1 度走らせる（排他のテストの机の決まりに従う）
  - 終わった姿: 全体テストが緑で、`THIRD-PARTY-NOTICES.md` の差分は `crash-report-kit 0.1.0` の 1 行だけ。その差分をコミットに含め、結果を Implementation Notes に書く
  - _Dispatch: mechanical — 決まったコマンドを走らせて記録する_
  - _Requirements: 1.4, 8.1_

---

## Implementation Notes

- 2.3: `areka` には lib のターゲットが無い。本体のテストは `cargo test -p areka --bins <名>` で回す（`--lib` は「no library targets」で落ちる）。
- 3.1: `test_support.rs` には別の働きの `measure_template::code_only` も残る（設計どおり）。「`fn code_only` が placement の 1 件」は `//` の行を落とす方の定義が 1 件、の意味で読む。
