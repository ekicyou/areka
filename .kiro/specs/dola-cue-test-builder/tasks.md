# Implementation Plan

- [ ] 1. 基盤: 付け替えの前の記録と `dola` の組み立て
- [ ] 1.1 付け替えの前のテストの数を控える
  - コードに触る前に、`dola`・areka-ghost・areka-seriko・areka-runtime・本体 `areka` のそれぞれで、テストの一覧（`cargo test -p <crate> -- --list`）を単独で取り、テストの数と無視されたテストの数を控える
  - 控えはワークツリーの `target\` の下に置く（リポジトリの差分に入れない）
  - 5 つの crate の前の数が、後で比べられる形で `target\` の下に残っている
  - _Dispatch: mechanical — 決まったコマンドを回して数を控えるだけ_
  - _Requirements: 4.2_

- [ ] 1.2 `dola` に `test-support` 機能と正準のキャリアの合図の組み立てを足し、檻で固定する
  - `dola` の機能の表に依存を持たない空の `test-support` 機能を足し、テストだけが使う組み立てを開けること・使い手は dev の依存でだけ付けることを 1 行のコメントで書く。docs.rs の機能の表は変えない
  - `cue` の中に、テストのビルドか `test-support` 機能のときだけ組まれる支えのモジュールを足し、演者・名・トークンの並びを受けて発火時刻 0・長さ 0 の正準のキャリアの合図を返す組み立てを 1 つだけ置く（演者の既定や演者を省く 2 つめの口は置かない・doc テストの例は付けない）
  - 兄弟のテストファイルに、設計の T1（形）と T2（空のトークン・`--key=value`・順を保った往復）の 2 本を置く
  - `cargo test -p dola` で檻の 2 本を含めて緑、`cargo check -p dola` と `cargo check -p dola --features test-support` がどちらも警告 0 で通る
  - _Depends: 1.1_
  - _Dispatch: mechanical — 本文・署名・檻の期待値を設計が与えている_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 5.6_

- [ ] 1.3 (P) `dola` の README に `test-support` 機能の段落を足す
  - About の段落の直後に、設計の下書きどおりの英語の段落を 1 つ置く（Key features の箇条には足さない）
  - README に機能の働き（テストだけが使う組み立てを開く・dev の依存でだけ付ける・本番に付けない）が読める
  - _Boundary: dola README_
  - _Dispatch: mechanical — 文面を設計が与えている_
  - _Requirements: 2.5_

- [ ] 2. 使い手 3 crate の写しを `dola` の組み立てへ付け替える
  - 2.1〜2.3 の編集は並走してよいが、各 crate の `cargo test` は同じ `target\` を使うので 1 本ずつ順に回す
- [ ] 2.1 (P) areka-ghost の写し 2 つを付け替える
  - dev の依存の `dola` の行に `test-support` 機能を足し、上のコメントをキャリアの合図の組み立ても引くと読める文に直す（本番のコードは `dola` を直に引かない、の趣旨は残す）
  - `prop_sink.rs` のテストのモジュールの私有の組み立てを消し、`dola` の組み立てを同じ名で取り込む。`prop_get_sink_tests.rs` の `carrier(name, tokens)` は演者 `"0"` を渡すだけの 1 行にする
  - 呼び手の行と、他のテストが使う取り込みは変えない
  - `cargo test -p areka-ghost` を単独で回して緑、警告 0
  - _Boundary: areka-ghost_
  - _Depends: 1.2_
  - _Dispatch: mechanical — 撤去の後の形を設計の表が与えている_
  - _Requirements: 2.3, 3.1, 3.4, 3.6, 4.1, 4.3_

- [ ] 2.2 (P) areka-seriko の写しを再輸出に付け替える
  - dev の依存に `test-support` 機能付きの `dola` の行を 1 行のコメントとともに足す
  - `actor_test_support.rs` の `carrier_cue` の本文を消し、同じ位置で `dola` の組み立てを同じ名で再輸出する。正準でない合図の組み立てと他の取り込みは変えない
  - 呼び手の 5 ファイルが無変更のまま組める
  - `cargo test -p areka-seriko` を単独で回して緑、警告 0
  - _Boundary: areka-seriko_
  - _Depends: 1.2_
  - _Dispatch: mechanical — 撤去の後の形を設計の表が与えている_
  - _Requirements: 2.3, 3.1, 3.2, 3.5, 4.1, 4.3_

- [ ] 2.3 (P) areka-runtime の写し 2 つを付け替え、機能を前送りする
  - dev の依存に `test-support` 機能付きの `dola` の行を足し（`areka-placement` の dev の行と同じ書き方・1 行のコメント）、自身の `test-support` 機能に `dola` の `test-support` を前送りする
  - `consumer_ledger_test_support.rs` の `actor_carrier_cue` を `dola` の組み立ての別名の再輸出にし、`carrier_cue(name, args)` の演者 `"0"` の 1 行と `broken_custom_cue` は変えない
  - `frame_window_link_drain_tests.rs` の `carrier` を消して `dola` の組み立てを `carrier` の名で取り込み、使われなくなる取り込みを外す
  - 呼び手の 12 ファイルが無変更のまま組め、`cargo test -p areka-runtime` を単独で回して緑、警告 0
  - _Boundary: areka-runtime_
  - _Depends: 1.2_
  - _Dispatch: mechanical — 撤去の後の形と manifest の行を設計が与えている_
  - _Requirements: 2.3, 3.1, 3.2, 3.3, 3.5, 3.6, 4.1, 4.3_

- [ ] 3. 付け替えの確かめ
- [ ] 3.1 各 crate の単独のテストで、前と同じテストが全部緑であることを確かめる
  - `dola`（`--features toml` の段も）・areka-ghost・areka-seriko・areka-runtime・本体 `areka` を 1 回ずつ単独で回す（`--workspace` で機能が合わさって書き忘れが隠れないように）
  - 1.1 の控えと `-- --list` の数を比べ、`dola` が檻の 2 本だけ増え、他の 4 crate は同じ数で、無視されたテストの数が増えていない
  - 本体のテストは本体の `Cargo.toml` を変えずに runtime の `carrier_cue`・`actor_carrier_cue` を引いて緑
  - _Depends: 1.1, 2.1, 2.2, 2.3_
  - _Dispatch: mechanical — 決まったコマンドを回して数を比べるだけ_
  - _Requirements: 2.1, 3.3, 3.4, 4.2_

- [ ] 3.2 本番・公開・差分の範囲を確かめる
  - `cargo tree -p areka -e normal,features -i dola` の出力に `test-support` が出ない
  - `pwsh -NoProfile -File tools/crates-io.ps1` が通り、`git diff --exit-code Cargo.lock` が 0
  - `git diff --name-only main...HEAD` が設計の File Structure Plan の 13 ファイルと spec の文書だけで、本体 `crates/areka/`・根の `Cargo.toml`・`command.rs`・呼び手の 22 ファイル・本体の session の `prop_set_cue`・`TalkCue { at: 0.0, … }` の直書きのテストが入っていない
  - 写しの 5 ファイルから発火時刻 0 と `command_carrier` の対の本文が消え、差分で足したコメントに spec 名・要件番号・タスク番号が無い
  - 確かめの結果がすべて期待どおり（`prop_set_cue` の申し送りは完了の段で `audit-areka-app` の brief に書く）
  - _Depends: 3.1_
  - _Dispatch: mechanical — 決まったコマンドと grep を回すだけ_
  - _Requirements: 2.3, 2.4, 2.6, 3.1, 3.5, 4.1, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
