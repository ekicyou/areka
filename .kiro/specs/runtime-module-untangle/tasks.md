# Implementation Plan

各段のあいだで `cargo check --workspace --all-targets` が警告 0 件で通る順に並べる（design.md「Migration Strategy」の S0〜S8）。段の差分は混ぜない。新しい試験の関数は足さず、固定は既存の試験への assert で足す。振る舞いを変えたくなった所は直さず、受け持つ audit の brief へ申し送る（9.3 で記録する）。

- [ ] 1. 着手の測り（コードを触る前の「前」を取る）
- [x] 1.1 テストの名前の一覧と警告の数を取る
  - `cargo test --workspace --no-fail-fast -- --list` の `: test$` の行から名前の末尾の多重集合を作り、`research.md` に置く（比べるための「前」）
  - `cargo check --workspace --all-targets` の警告が 0 件であることを記録する
  - 完了の形: `research.md` に着手の main の SHA・テストの数・名前の多重集合の置き場・警告 0 件が書かれている
  - _Requirements: 8.1, 8.2_

- [ ] 1.2 終わる道 3 つの記録の順（前）を実機で取る
  - メニューの終了・窓を閉じる・煙の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）の 3 つを、同じ `RUST_LOG` で 1 回ずつ走らせ、終わる段取りの行の順を抜き出す
  - 実機の根と一時フォルダはワークツリーの `target\` の下・絶対パスで走らせ、画面の操作は排他テストの机で順を取って、終わったらすぐ返す
  - 完了の形: `research.md` に 3 つの道ごとの行の順（出所・文言・段）と、走らせた `RUST_LOG` が書かれている
  - _Requirements: 3.6, 8.5_

- [x] 1.3 外の参照の数え方を今の木で較正する
  - 本番のファイルの集合（テスト・テストの支え・`spine*`・`*_cases` を除く）が、親が `#[cfg(test)]` で宣言していないファイルの集合と一致することを確かめる
  - `#[path]` の宣言をたどって、宣言の深さ 1 の集合 D1 と深さ 2 の集合 D2 を一覧にする（深さ 3 以上の本番のファイルがあれば式を 1 本足す）
  - 式 1〜3 を今の木に当て、外への参照だけが出て `runtime` の中を指す `super::super::frame` ほかが出ないことを確かめ、今の数を記録する
  - 完了の形: `research.md` に D1・D2 の一覧と、式 1〜3 の今の行数（外の参照だけ）が書かれている
  - _Requirements: 3.1, 3.3_

- [ ] 2. 名前の改め（機械的な改名だけの段）
- [ ] 2.1 モジュールを `runtime` へ改名し、パスで読む所を全部追従させる
  - モジュールのフォルダを履歴の残る移動で改名し、本体の中の `emo2_boot` のパス（`main.rs`・`ghost_session*`・`mcp/`・`menu/`・`input_events/`・`install/`・`update/`・`readme/`・`offline_dump/`・`session_end.rs`・`app_exit.rs`・`boot_config.rs`・`log_history.rs`・`placement_tests/` の 9 ファイルほか）を書き換える
  - ソースを文字列で読むテスト（`include_str!`・`CARGO_MANIFEST_DIR`・`crate_source_test_support` 経由・`tick_gate_config_producers_tests` のパス 8 本）と `examples/collision-probe.rs` の `#[path]` 2 つを新しいパスへ
  - 本体の外でパスが切れると働きが変わる所（1,000 行の番人の例外表 2 か所・`workspace_scan_test`・`wintf` の `test_crash_report.rs` 1 行目・`tools/load-flake.ps1` 47 行目）を直し、例外表の項目は増やさない
  - 落ちの報告の支えは中身を変えずに一緒に動く
  - 説明の文の名前とパスを直す: `crates/areka/Cargo.toml` のコメント 2 行（依存の行は触らない）・`examples/emo-present.rs`・`examples/emo-present/reconcile.rs`・`tests/emo2_real_run.rs`
  - 完了の形: 警告 0 件で通り、文字列で読むテストと番人が新しいパスのファイルを見つけて緑（見つけられなければ赤になる今の作りは変えない）
  - _Requirements: 1.1, 2.6, 5.1, 5.2, 5.3, 5.4, 8.6, 8.7_

- [ ] 2.2 型・関数・テストの結線の名前を改める
  - `RuntimeWiring`・`RuntimeInputs`・`RuntimeOutcome`・`wire_runtime`・`register_frame_system`・`frame_system` と、テストの中の結線を指す名前（`headless_emo2_wiring` ほか）を改める
  - 既定のゴースト名とその使い手・実在のゴースト emo2 を指す名前・emo2 を含まない名前は変えない。名前だけで決まらないものは使われ方で判定し、判定と理由を `research.md` に書く
  - 名前を字面で照合するテスト（`frame_schedule_tests` の `pub fn wire_emo2_boot(` と毎フレームの相・終わる相の登録の字面）を改めた名前へ一緒に直す
  - 名前を改めたテストを新旧の対応表にして `research.md` に置く
  - 完了の形: 警告 0 件で通り、対応表と判定の表が `research.md` にある
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6_

- [ ] 2.3 ログの文言とイベントの値を改め、照合する所を一緒に直す
  - 文言 `emo2-boot:`・`emo2 attach:`・`emo2 text:`・`emo2-boot wire` の 2 行と、イベントの値 `*_no_emo2` の 4 つを `runtime` の語へ改める。文の本体・欄・段は変えない
  - 照合する所（`smoke_boot_loop_exit` の成立の印・`emo2_real_run/common.rs` と `common_tests.rs`・`balloon_pointer_handler_tests`・`balloon_leave_tests`・`assets_tests`・`balloon_background_tests`・`reload_skin_tests`・`reload_tests`）を同じ文言と値へ直す
  - 完了の形: 警告 0 件で通り、照合するテストが改めた文言と値で緑
  - _Requirements: 1.1, 1.5, 1.6_

- [ ] 3. `runtime` に型を下ろす
- [ ] 3.1 切替の型を `runtime` の新しいファイルへ下ろす
  - `SkinKind` ほかの切替の型と `skin_ref_name`・`skin_ref_path` を中身を変えずに切り出し、`runtime` の中は直に引き、入口のファイルは呼び手のために出し直す
  - 行の多い側（要求の入口）を元のファイルのまま残し、少ない側（型）を新しいファイルにする
  - 完了の形: `runtime` の中で切替の型を引く所が新しいファイルを指し、`menu/`・`mcp/`・テストの書き換えなしで警告 0 件で通る
  - _Requirements: 2.3, 2.5_

- [ ] 3.2 切替の要求の型と cue の束と起動の誤りを `runtime` へ下ろす
  - `ChangeRequestRaw` を `change_cue` へ、`CueSinkSources`・`RuntimeBootError` を `runtime` のモジュールの根へ下ろし、元の所は出し直しで呼び手を変えない
  - 誤りの分類と記録（`warn!`／`error!` の振り分け）は起動の側に残す
  - 完了の形: 残るファイルが 3 つの型を `runtime` の中から引き、警告 0 件で通る
  - _Requirements: 2.5_

- [ ] 3.3 バルーンのホバーの集合と箱の滞在を `runtime` の型にする
  - ホバーの集合を `BalloonWiring` から分けた `BalloonHover` にし、`BalloonWiring` を置く所と同じ所・同じ時に並べて置く（本番の結線 1 か所・テスト 13 ファイル）
  - 書く所（滞在・離脱）は `BalloonHover` へ書き、可視性の相は `BalloonHover` を読む。無いときの `error!`（1 回・文言そのまま）と掃除の扱いは今と同じ
  - `ShellBoxHover`・`settle_box_hover` を中身を変えずに `runtime` へ移し、入力の側は出し直しで呼び手とテストを変えない
  - 完了の形: ホバーの滞在・離脱・掃除・冪等の既存の試験が緑で、可視性の相が入力の層を引かない
  - _Requirements: 3.1_

- [ ] 3.4 倍率の表と文字の関数を `runtime` へ下ろす
  - 表と `percent_text`・`scaling_text` を中身を変えずに移し、結線の側は `wire` と源の型だけを持って残りを出し直す
  - `frame/wiring.rs`・`frame/scale_notice.rs` が `runtime` の中の表を引く
  - 完了の形: 警告 0 件で通り、倍率の表を引く `runtime` の本番が本体の結線の側を名指さない
  - _Requirements: 3.1_

- [ ] 4. 本体の口（SessionHooks）
- [ ] 4.1 口の束を作り、差し替えの相を口に替える
  - 本体の新しいモジュール `session` を作り（宣言だけの入口）、`runtime` に口の束の型・`NO_BODY`・本番とテストで倒れ先を分ける `FALLBACK`・`of()` を置く。本番で据え忘れたときは呼ぶたびに `error!(session_hooks_missing)` を出して `NO_BODY` を返す
  - `session` に口の中身と据え付けを置き、`register_systems` の先頭で最初の巡より前に据える。プロセスで共有する状態は増やさない（回数を絞る `OnceLock` は使わない）
  - 口の欄はこのタスクで読む分（差し替えの相が使う分）だけを置き、4.2 で読む分は 4.2 で足す（使われない欄で警告を出さない）
  - 差し替えの相が本体の状態（送り手・切替の予約・今のシェルとバルーン・記憶・読み直しの印・ゴーストの名前）を読み書きする所を、相の入口で 1 度取った口から同期で呼ぶ形にする。後始末の段の並び・記録の文・欄・段・出所は変えず、口の偽・`None` は今と同じ `warn!`／`error!` の枝へ進む。`NEXT_EPOCH` はこのファイルに残す
  - `boots_twice_in_one_process_without_double_registration` に「口が据わっている」の assert を、既存の `frame/switch_tests.rs` の試験に `NO_BODY` の答えの assert を足す（試験の数は変えない）
  - 完了の形: 警告 0 件で通り、差し替えの相のファイルから本体の層への `use` が消え、`switch_tests`・`switch_finish_tests`・`reload_skin_tests`・切替の会話の試験・再起動の試験が今の記録と順のまま緑
  - _Requirements: 2.5, 3.1, 3.7, 3.8, 3.9_

- [ ] 4.2 残る相と読み直しの持ち越しを口に替える
  - このタスクで読む口の欄を足し、口は全部で 10 本になる（`NO_BODY` の assert もその分を足す）
  - 状態の報告・結びつけ・倍率の知らせ・窓の状態・可視性の相が `GhostSlot`・`BootContext` を読む所を口へ替える
  - 読み直しの持ち越しは、取り込みが門の答えを引数で受け（偽なら今と同じく取らない）、呼び手が門を渡す。戻すときは口で seriko の送り手を引く。記録の文と出所は変えない
  - 完了の形: 7 ファイルの本番から本体の状態の型を名指す所が消え、可視性・倍率・読み直しの既存の試験が緑
  - _Requirements: 2.1, 3.1, 3.7, 3.8_

- [ ] 5. 終わる相と起動の継ぎ目
- [ ] 5.1 終わる相を相ごと `session` へ移す
  - 通知の受け口・終わる相の本体・system・登録の関数を中身と記録の文を変えずに `session` の新しいファイルへ移し、登録は毎フレームの相の直後に `.before(frame_system)` のまま行う
  - 試験 3 本を名前から `frame_` を外したファイルへ履歴の残る移動で移し、試験の関数の名前は変えない
  - `frame_schedule_tests` が読む字面を、毎フレームの相は `runtime` の根、終わる相は新しいファイルへ向ける
  - `frame.rs` を文字列で読むほかのテスト（`frame_harness_tests`・`frame_move_order_tests`・`frame_work_area_{resnap,sync}_tests`・`zorder_wiring_tests`）が、終わる相を切り出した後も同じ照合で通ることを確かめ、要るなら読む相手を直す
  - 完了の形: `frame_schedule_tests` と終わる相の試験 3 本が緑で、終わる相の出所が `areka::session::ghost_quit` になる
  - _Requirements: 2.5, 3.1, 3.6, 3.8, 4.3_

- [ ] 5.2 実行系の起動を引数で受ける継ぎ目を作る
  - `runtime` に起動の関数の型を置き、`wire_runtime` の最後の引数で受けて手順 5 で呼ぶ。失敗のときに落とす順（tick の受け口・会話の受け口・源・seriko の握り）は今と同じ
  - 呼び手は本番の起動の関数を渡す。総称にはせず、`wire_runtime` の字面の照合を保つ
  - 完了の形: `runtime` の根と骨組みから起動の側への参照が消え、`zorder_wiring_tests`・起動の試験・再起動の試験が緑
  - _Requirements: 2.1, 2.5, 3.1, 3.8_

- [ ] 6. ファイルの移動
- [ ] 6.1 本体に頼るファイルを `session` へ移す
  - 切替・読み直し・実行系の起動・写し・同じフォルダの留め・切替の要求の入口の本番と各テスト（切替の試験の支えを含む）を履歴の残る移動で `session` へ移す
  - 中身の書き換えは、`runtime` に残るものへのパス・型の置き場・名前の改めだけ。見える範囲の広げはコンパイルが求めた分だけで、足した分を `research.md` に書く
  - 移した先を文字列やパスで引く所を一緒に直す: `zorder_wiring_tests.rs` の `include_str!("runtime_boot.rs")` → `../session/runtime_boot.rs`・`spine.rs` の起動の選択肢の引き口（テストの支え）
  - 既定のゴーストかどうかの判定（`DEFAULT_GHOST_FOLDER` の使い手）は移した切替のファイルの中に残る
  - 完了の形: 警告 0 件で通り、切替・読み直しの試験が今の段取りの順のまま緑で、`runtime` のフォルダに移したファイルが無い
  - _Requirements: 2.1, 2.3, 3.7, 6.4_

- [ ] 6.2 install・update・readme の cue の受け口を持ち主へ移す
  - 3 つの受け口と各テストを持ち主のモジュールへ履歴の残る移動で移し、持ち主が宣言する。組み立ての並びは変えない
  - 受け口の同型の形と台帳の表の行は変えず、台帳の説明の文のリンク 6 か所を移した先のパスへ書き換える
  - 完了の形: 3 つの受け口の試験と台帳の試験が緑で、`runtime` に cue の受け口の持ち主を名指す所が無い
  - _Requirements: 2.2, 2.3, 2.4_

- [ ] 7. ログの出所の追従
- [ ] 7.1 記録の分け方の決まりを移した出所へ追従させる
  - 決まり `areka::session::ghost_switch`（Status）へ改め、決まりを当てる前に引く「残さない出所」の表に `areka::install::install_cue` を置く
  - 既存の `unrelated_info_is_not_kept` に、その出所の info は残らず warn は誤りになる assert を足す。行の数と順（14 行）は変えない
  - 完了の形: `log_history_tests` が緑で、移した出所の行が前と同じ種類と扱いで分かれる
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 7.2 出所を名前で読むほかの所を新しい名前へ直す
  - `stay_on_top_real_run.rs` の `RUST_LOG` 6 つ・`tools/load-flake.ps1` の例・出所を照合するテスト（`areka::runtime::…`・`areka::session::reload`）・`doc/ssp-mcp/log-convention.md` を直す
  - `log-convention.md` は出所に加えて、2.3 で改めたログの文言とイベントの値もここで直す
  - 語彙として付けた直書きの出所（`areka::persist::*` ほか）は変えない
  - 完了の形: 出所を照合するテストが新しい出所で緑で、`log-convention.md` が新しい出所を指す
  - _Requirements: 1.5, 4.2, 4.4, 4.5_

- [ ] 8. 文書と申し送り
- [ ] 8.1 (P) 文書と steering と網羅台帳の古いパスと名前を書き換える
  - `doc/COMPAT_ARCHITECTURE.md`・`doc/emo2-conformance-scope.md`・`doc/ssp-mcp/`（`log-convention.md` は 7.2 が持つので除く）・`doc/ideas/`・`.kiro/steering/structure.md`・`tech.md` を移した先のパスと改めた名前へ
  - 網羅台帳の根拠の場所を根拠のファイルが実際に移った先へ書き換え、台帳から作る生成物（`briefing-*.md` ほか）は生成器で作り直す（手で直さない）
  - 完了の形: 台帳の検査と生成物の差分が緑・きれいで、文書の古いパスが残らない
  - _Boundary: DocSweep_
  - _Depends: 6.2, 7.2_
  - _Requirements: 5.5, 5.6_

- [ ] 8.2 (P) roadmap の対応表と brief の受け持つファイルと申し送りを書く
  - `.kiro/steering/roadmap.md` に新旧のパスの対応表（design.md「Supporting References」の表）を置く
  - `audit-emo2-boot-frame`・`audit-emo2-boot-spine`・`audit-emo2-boot-switch` の brief の「受け持つファイル」を移した先のパスへ
  - `runtime-crate-split` の brief に、落ちの報告の支えが本体と `areka-runtime` の両方に要ることと、テストが本体の外を引く所（口のテストの倒れ先・文字列で読むファイル・`transition_judge`・骨組みの起動の選択肢）を書く
  - 完了の形: 対応表で `emo2_boot` を名指すほかの brief が読み替えられ、4 本の brief が新しいパスと申し送りを持つ
  - _Boundary: DocSweep, Handover_
  - _Depends: 6.2, 7.2_
  - _Requirements: 5.7, 5.8, 7.2, 7.3_

- [ ] 9. 検証
- [ ] 9.1 境界の 0 件と数を記録する
  - 完了の木で D1・D2 を 1.3 と同じ方法で作り直し（新しいファイルを入れ、移したファイルを外す）、式 1〜3 を当て、3 本の出力が 0 行であること（0 でなければ残りが全部 `#[cfg(test)]` の項目の中であることを行ごとに）を示す
  - 説明の文の名指しを 3 本の説明の版で一覧にし、テストが本体の外を引く所をファイルごとの数で数える
  - プロセスで共有する状態 3 つの置き場が変わっていないことを書く
  - 完了の形: `research.md` に式・除いたもの・0 行の結果・説明の文の一覧・テストの参照の数・共有状態の置き場が書かれている
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.9, 7.2_

- [ ] 9.2 古い名前の掃き出しと emo2 の残りの一覧を作る
  - 古い出所 `areka::emo2_boot` と古いパス・改める前の名前を `crates/`・`tools/`・`doc/`・steering の 2 本で grep し、残りが範囲外の所と歴史の文書だけであることを示す
  - `emo2` を大文字小文字を問わず grep し、各行を 4 つ（既定のゴースト名とその使い手・実在の emo2 の検体・本体の外の crate の説明とテストの名前・歴史の文書）に分ける。どれにも当たらない行は要件 1 の規則で改めて一覧を取り直す
  - 完了の形: `research.md` に掃き出しの結果と 4 つに分けた一覧があり、分けられない行が 0 で、emo2 だけに向けた動きが既定のゴーストの判定だけである
  - _Requirements: 1.2, 4.6, 5.9, 6.1, 6.2, 6.3, 6.4_

- [ ] 9.3 テストの支えの一覧と audit への申し送りを記録する
  - `runtime` の外のファイルが名指す `runtime` のテストの支えを `rg -c` で数え、支えごとに使う所と回数の表にする
  - 作業中に直したくなった不具合と要らない項目を、受け持つ audit の brief へ書き、申し送った先と中身を一覧にする（無ければ 0 件と書く）
  - 完了の形: `research.md` に支えの表と申し送りの一覧がある
  - _Requirements: 7.1, 7.4_

- [ ] 9.4 テストの名前の比べと静的な門
  - 完了の形でテストの名前の多重集合を取り、1.1 の「前」と比べる。差は 2.2 の対応表だけで説明できる
  - 警告 0 件・1,000 行の番人が緑（例外表の項目は増えていない）・`Cargo.toml` はコメントだけで `Cargo.lock` と依存は変わっていない
  - 完了の形: `research.md` に多重集合の差（対応表で説明済み）・警告 0 件・番人の緑・`Cargo` の差が無いことが書かれている
  - _Requirements: 1.4, 8.1, 8.2, 8.6, 8.7_

- [ ] 9.5 全体テストを回す
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回す（長いので裏で）。赤が出たら赤のテストだけに絞って直し、絞って回し直す
  - 完了の形: 全体テストが緑
  - _Requirements: 8.3_

- [ ] 9.6 実機テスト 2 つと終わる道の記録の順（後）
  - `smoke_boot_loop_exit` と `emo2_real_run`（`AREKA_EMO2_REAL_RUN`）を 1 回ずつ回す
  - 1.2 と同じ 3 つの道・同じ `RUST_LOG` で走らせ、終わる段取りの行の順を、出所と文言の対応表で読み替えて 1.2 と比べる
  - 実機の走行はこの 2 つと 1.2 だけ
  - 完了の形: 2 つの実機テストが緑で、`research.md` に 3 つの道の前後が同じ順であることが書かれている
  - _Requirements: 3.6, 8.4, 8.5, 8.8_

## Implementation Notes
- 1.1: `cargo check` は rustc の警告 0 件だが、cargo のマニフェストの警告（使われない依存）が 3 件前からある（workspace の `async-executor`・`shiori-abi` の `windows`・`shiori-host32-host` の `event-listener`。どれも `crates/areka` の外）。要件 8.2 は「rustc 0 件・マニフェストはこの 3 件のまま」で比べる。消すのは範囲外（要件 8.7 が依存の変更を禁じる）＝完了の棚卸で起票する。
- 1.3: 9.1 で式 1〜3 を当て直すときは、インラインのテストのモジュールだけでなく、本番のファイルの中の `#[cfg(test)]` の fn・use・impl のメソッドに当たる行も見る。
