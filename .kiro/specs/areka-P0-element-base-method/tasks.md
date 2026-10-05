# Implementation Plan

> 用語: 「描ける語」＝ element定義で areka が描く描画メソッドの語（`overlay`・`base`、箱として読む areka 独自の `balloon`）。「描けない行」＝ `surface*`ブレス・`surface.append*`ブレスの中の、第 2 欄が描ける語でない element定義の行（欄が空の行を含む）。「3 つの転記」＝画像の読み手（`Element`）・箱の転記（`parse_boxes`）・描けない行の転記（`parse_undrawn_elements`）。いずれも design.md と同じ。
>
> 共通の決まり: テストは実装と同じフォルダの兄弟ファイルに置き、本番ファイルには `#[cfg(test)] #[path = "…"] mod …;` の接続の宣言だけを足す。1 ファイル 1,000 行以下。読み手は記録を出さない（記録は `load_shell_target` の 1 か所だけ）。画像は `MemoryDecoder`、記録の捕捉は `test_support::capture_events`、一時フォルダは `temp-path-kit` で決定論的に回す。実機の根・検体の写し・一時フォルダはワークツリーの `target\` の下だけに置く。
>
> 触らないファイルの振る舞い（設計の Out of Boundary）: `fold.rs` の処理・`base_image.rs`・`plan.rs`・`blit.rs`・`method.rs` の `is_implemented`／`known_method`・`boxes.rs`・`manifest.rs`・`build_shell_target*` の署名・`Element`／`Shell`／`ShellTarget` の型。既存のテストの期待値は、下の 1.1 で付け替える 3 本を除いて 1 本も書き換えない。
>
> 完了時（`/kiro-complete`）に回すもの（本書のタスクにしない）: 残りの描画メソッドの追跡用 spec の `/kiro-discovery` での起票、台帳の担当をその spec の名前へ直すこと、roadmap の本 spec の行（「読み手に描画メソッドの欄を足して」）を実際の形へ直すこと。

- [ ] 1. 読み手で `base` を値にし、描けない行を一覧にする
- [x] 1.1 画像の element定義として値にする語を `overlay` と `base` の 2 語にし、縮退を固定していたテストを付け替える
  - `decode.rs` の `decode_elements` の判定（第 2 欄が `overlay` と完全一致）を、語を受けて真偽を返す 1 関数 `is_image_element_method`（`pub(super)`・`overlay` と `base` だけ真・完全一致）に置き換える。`Element` に写す欄と並べ替えは変えない
  - 説明文を新しい約束に直す: `decode.rs`・`model.rs` の `Element`／`Surface.elements`（「`overlay` と `base` の行」）・`boxes_tests.rs` の `image_reader_ignores_box_lines`（判定は変えない）
  - 既存の 3 本を付け替える（消さない）: `decode_tests_lenient_input_tests.rs` の 2 本を「`base` は値になる」「`replace` は吸収されて隣の行は残る」へ、`validation_tests.rs` の `surface200` の `element0,base,bg.png` の 1 本を「`base` は値になる」へ。`unknown.block.head` の中の `base` の行はそのまま
  - `decode_tests_method_matrix_tests.rs` に足す: ⑴ `surface*`ブレスの `element0,base`・`element1` 以降の `base`・`surface.append*`ブレスの `base`・数字だけの欄の `base`・複数の番号を並べた見出しを含む文面の `parse` が、`,base,` を `,overlay,` に書き替えた文面の `parse` と等しい。較正として、`base` の行を除いた文面より element の件数がちょうど `base` の行の数だけ多い ⑵ `replace`・空の欄・ukadoc に無い語・`Overlay`（大文字）・`add`・`bind` の行を含む文面の `parse` が、その行を除いた文面の `parse` と等しい
  - 完了の姿: `cargo test -p areka-parsers` が緑で、`decode_elements` の判定を元の `overlay` だけに戻すと ⑴ と付け替えた 3 本が赤になる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 2.4, 2.5, 3.1, 3.5_

- [ ] 1.2 描けない行の転記を足す
  - `crates/areka-parsers/src/shell/undrawn.rs` を新設し、`UndrawnElementLine`（`heading`・`element`・`method` の 3 つの文字列）と `parse_undrawn_elements(text)` を置く。字句解析は `lexer::lex`、閉じたブレスだけを見て、見出しの判定は読み手と同じ順（`surface.append` を `surface` より先）
  - 対象は、キーが `element` で始まり、第 2 欄が `is_image_element_method` で偽で、かつ `balloon` でない行。第 2 欄が無ければ語は空文字列。見出しは欄を `,` でつなぎ直した原文で持ち、展開しない（1 行 1 件）。element番号は `element` に続く文字列を原文のまま
  - `shell/mod.rs` に `mod undrawn;`・`#[cfg(test)] mod undrawn_tests;` と `UndrawnElementLine`・`parse_undrawn_elements` の公開を足す
  - 兄弟の `undrawn_tests.rs` で: 1.1 ⑵ の各語が見出し・element番号・語つきで 1 件ずつ返る／`surface0,1` と `surface.append0-1` の見出しで 1 行が 1 件・見出しが原文のまま／数字でない element番号が原文のまま／`overlay`・`base`・`balloon` だけの文面で空／`surface*` でないブレスの中・ブレスの外の行は対象外
  - 同じファイルに「3 つの転記が行を漏れなく重なりなく分ける」を 1 本: 全種類の語を混ぜた文面で、手で数えた `element` の行の数＝`Element` の件数＋箱の element定義の件数＋`UndrawnElementLine` の件数。見出しは番号 1 つだけにし、数えない行（`descript`・`balloon.*`・`kero.surface.alias`・未知の見出しのブレス・ブレスの外・閉じずに終わる `surface*`ブレスの中の `element` の行）も混ぜる
  - 完了の姿: `cargo test -p areka-parsers` が緑で、見出しの判定の順を入れ替える・`balloon` の除外を外す、のどちらでも `undrawn_tests.rs` のどれかが赤になる
  - _Requirements: 2.1, 2.2, 2.6, 4.2_

- [ ] 2. 入口で警告を出し、絵と大きさを通しで固定する
- [ ] 2.1 `base` の土台に `overlay` を重ねた外形と画素、今の見え方の不変を檻にする
  - `crates/areka-emo-present/src/shell_target_element_base_tests.rs` を新設し、`shell_target.rs` の末尾に接続の宣言を足す
  - ⑴ `surface26 { element0,base,body.png,0,0 / element1,overlay,face.png,X,Y }` を `build_shell_target` で焼いて合成し、外形が `body.png` の実寸、土台の位置の画素が `body.png` の色、部品の位置の画素が `face.png` の色。`surface26.png` を大きさも色も違う絵として復号器に入れ、それが使われない（外形が動かない・`shadowed` に 26 が載る）ことを較正にする
  - ⑵ `element0,base,surfaceN.png,0,0` を持つ文面と、そのブレスごと除いた文面（`surface*.png` が土台に敷かれる経路）とで、合成した外形と画素が等しい
  - ⑶ `element0,replace,x.png` と `surface*.png` を持つサーフェスの合成結果が、その行を除いた文面と等しく、`x.png` は焼かれていない
  - ⑷ `element0,base,missing.png`（復号器に入れない）に `overlay` の部品を重ねた文面で、`target.bake_errors` が `missing.png` を名指す 1 件で `,overlay,` に書き替えた文面と等しく、部品は描かれ、合成結果も書き替えた文面と等しい（記録の `warn!` はこの `bake_errors` を `load_shell_target` が出すので、中身の一致で足りる）。較正として、`missing.png` を復号器に入れると `bake_errors` が 0 件になる
  - 完了の姿: `cargo test -p areka-emo-present` で ⑴〜⑷ が緑で、1.1 の判定を元の `overlay` だけに戻すと ⑴ が赤になる
  - _Requirements: 1.1, 1.2, 1.3, 1.7, 2.4, 2.5, 3.2, 3.4, 4.1_
  - _Depends: 1.1_

- [ ] 2.2 `load_shell_target` で描けない行を読み込み 1 回につき 1 行 1 件 `warn!` する
  - `load_shell_target` が、`parse`・`parse_boxes` に渡すのと同じ文面で `parse_undrawn_elements` を 1 度呼び、入れ子・箱の報告と同じ場所で 1 件につき `warn!` を 1 行出す。本文「shell: areka が描けない描画メソッドの element定義を描かない」、欄は `heading`・`element`・`method`。`ShellTarget` と `build_shell_target*` は変えない
  - 2.1 のファイルに足す（`TempPath` に surfaces.txt を置き `capture_events` の中で呼ぶ。焼く段の脱落の `warn!` が混ざらないよう、どちらの文面でも描ける行が指す画像はすべて `MemoryDecoder` に入れる）: `surface0,1` の見出しの下に描けない行 1 行と `overlay`・`base`・`balloon` の行（`balloon` には正しい `balloon.*`ブレスを添える）を置いた文面で、`warn` 以上の記録が本 spec の 1 行だけ（`heading`・`element`・`method` の値まで一致）／描ける語だけの文面で `warn` 以上が 0 行／同じ文面で `load_shell_target` を 2 回呼ぶと本 spec の記録がちょうど 2 倍
  - 完了の姿: 上のテストが緑で、`warn!` の 1 行を消すと赤になる
  - _Requirements: 2.1, 2.2, 2.3, 2.6, 4.2_
  - _Depends: 1.2_

- [ ] 2.3 無改変のクローディアを DLL なしで読み、surface6・11・26 の大きさを固定する
  - `shell_target_test_support.rs` に `sample-ghost-kit` の `claudia` のシェルのフォルダを返す受け口を足す（`emo2_shell_dir` と同じ `LazyLock<SampleRoot>` の形・`.nar` は `SampleRoot::acquire` が展開する）。同じファイルの受け口の自己確認（`every_sample_receptor_points_at_a_real_shell_folder`）の一覧にも `claudia` を足す
  - 2.1 のファイルに 1 本: `claudia` を既存の emo2 の檻と同じ受け口（実際の画像の復号）で `load_shell_target` に読ませ、surface6・11・26 の外形が 333×500、本 spec の警告が 0 件
  - 完了の姿: このテストが緑で、1.1 の判定を元の `overlay` だけに戻すと surface26 の外形が 100×56 になって赤になる
  - _Requirements: 4.3, 2.6, 3.2_
  - _Depends: 1.1, 2.2_

- [ ] 3. 説明文と台帳を実際の状態に合わせる
- [ ] 3.1 (P) 合成器の説明文と COMPAT §8 を直す
  - `fold.rs` の `normalize_element` の説明文を「読み手が届けるのは `overlay` と `base` の行で、どちらも `Overlay` で置く」に、`method.rs` の `ComposeMethod::Base` の注記を「XY 無視は pattern定義の話で、element定義の `base` はこの値を通らない」に直す（処理は変えない）
  - `doc/COMPAT_ARCHITECTURE.md` §8 の表に 1 行: 項目「element定義の描画メソッド `base` の X,Y」・裁量「`overlay` の element定義と同じに扱う（位置として使う）」・根拠（`ukadoc:descript_shell_surfaces` の `base` の項は pattern定義に限って XY を無視すると書く／`element0` では置き換えられる側が空・`element1` 以降は正典が `overlay` に読み替える）・出典 spec
  - 完了の姿: §8 にその行があり、`cargo test -p areka-emo-compose` が期待値を変えずに緑
  - _Requirements: 4.4_
  - _Boundary: Compose 説明文, COMPAT_
  - _Depends: 1.1_

- [ ] 3.2 (P) 網羅台帳の 2 行を書き換え、報告を作り直す
  - 先に、書き換える前の台帳で `cargo run -p ukadoc-survey -- report` と `report-summary` を 1 度回し、報告の差分が 0 であることを確かめる（差分が出たら main 側の陳腐化として別のコミットにする）
  - `doc/ukadoc-coverage/ledger/assets.toml` の element定義の行: 状態は `degraded` のまま、注記を「`base` は描ける・ほかは描かずに `warn!` を残す・`overlay` と `base` 以外の `element0` を持つサーフェスで画像が土台に残るずれは残る」へ、記録を `warn!` へ、担当は本 spec（完了時に追跡用 spec の名前へ）
  - `ukadoc:descript_shell_surfaces:base:1` の行: 状態を `vocabulary-only` から `degraded` へ（element定義では描ける・pattern定義では未対応のまま）、担当は本 spec
  - `cargo run -p ukadoc-survey -- report` と `report-summary` で報告を作り直す（数を手で直さない）
  - 完了の姿: `cargo test -p ukadoc-survey` が緑で、報告の差分が上の 2 行の分（集計の `vocabulary-only` −1・`degraded` +1 を含む）だけ
  - _Requirements: 4.5_
  - _Boundary: 台帳_
  - _Depends: 2.2_

- [ ] 4. 通しの確認
- [ ] 4.1 ワークスペースの全テストと静的検査を通す
  - `cargo test --workspace`・`cargo clippy --workspace --all-targets` を回す。pattern定義の `base`（描かずに警告）の既存テストと、`overlay` だけのサーフェスの既存テスト（golden を含む）が期待値を変えずに通ることを確かめる
  - 完了の姿: 全テストが緑で、本 spec の差分に clippy の警告が 0 件、1.1 で付け替えた 3 本のほかに期待値の書き換えが無い
  - _Requirements: 3.1, 3.3, 3.4, 3.5_

- [ ] 4.2 無改変のクローディアで実機を確かめる
  - ワークツリーの `target\` の下の根で無改変の `claudia` を起動し、`\s[6]`・`\s[11]`・`\s[26]` を表示して、MCP の `dump_surface` かログでキャラクターが 333×500・土台の絵と顔の部品が重なっていることを数値と絵で取る
  - 同じ起動で本 spec の警告が 0 行、surface19・29 が前と同じに見えることを見る。`load_shell_target` は起動 1 回で 2 か所（`emo2_boot/assets.rs`・`placement/measure.rs`）から呼ばれるので、`shadowed=` の数の行は 2 回出て、どちらも前より 2（19・29）多い。当たり判定は見ない
  - 結果を spec のフォルダの `verification/real-machine.md` に残す
  - 完了の姿: 記録に 3 面の 333×500 と警告 0 件が書かれ、絵が添えられている
  - _Requirements: 4.3, 3.2_
  - _Depends: 4.1_
