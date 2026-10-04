# Implementation Plan

- [ ] 1. `surfacetable.txt` の読み手
- [x] 1.1 `surfacetable.txt` の文面を行の列へ転記する読み手を作り、`shell` から公開する
  - `boxes` の読み手と同じ置き方で、新しいファイルに読み手と返す型を置き、`shell/mod.rs` へ `mod`・テストの `mod`・`pub use` の 3 か所だけを足す（`model.rs`・`decode.rs`・`Cargo.toml` は触らない）
  - 設計の「行の読み分け」の表のとおりに読む: 落とす空白は ASCII の空白とタブだけ・最初の `,` の前だけを整え後ろは書かれたとおり・見出し語は大小を区別しない・ID と `scope` は ASCII の数字だけ
  - `__disabled` の行・`__parts` の行・名前を省略した行も転記する（載せる・載せないは決めない）。`scope` が行の後に書かれても同じ `group` の行へ当てる
  - 読めない行は行番号だけを書かれた順に返す。知らない `charset` の名前の行もここに入る。読み手は記録もファイルの読み取りもしない
  - 完了の形: 兄弟のテストで、表の各行（空行・`//`・`{`・`}`・`version`・`option`・`charset` の既知と未知・`group`・`scope` の前後と重複と `group` の外・数値の行・名前の省略・行末の `}`・閉じていない `{`・`Charset`/`GROUP` の綴り・タブと空白の字下げ・読めない行の各種）が 1 つずつ緑になり、`cargo test -p areka-parsers` が通る
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.8, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8_

- [ ] 2. 表の組み立てとツールの中身
- [ ] 2.1 既定の名前 15 件と、転記から表の文字列を組む規則を作る
  - 既定の 15 件を埋め込みの定数で持つ（言語の設定を読まない）
  - 書かれた ID の集合は転記の全行から作り、`__disabled` の行と `__parts` の行を除いたシェルの行と、集合に無い既定の行を 1 本にして（スコープ, ID）で安定に並べる
  - 見出し行・区切り行・各行を `\r\n` で終え、スコープは `\0`・`\1`・`\p[n]` で書く。空の列は何も挟まない
  - 2.4 で配線するまで本番の呼び手が無いため、テストでないビルドの dead_code の警告は 2.4 まで出てよい（この段の合否はテストで判定する）
  - 完了の形: 実測 14 の検体（`ssp-measurements.md` の検体 1 の文面）を読み手に通した結果が、同ファイルの SSP の答えの全文と `\r\n` 込みで一致するテストが緑になる。空の転記で既定の 15 件だけの全文（検体 2）も一致する
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.1, 3.2, 3.6, 3.7, 4.1, 4.2, 4.3, 7.1_
  - _Depends: 1.1_

- [ ] 2.2 実測ごとの行と既定の重ね合わせの境目をテストで固定する
  - 検体 3・5・8・9 は `ssp-measurements.md` の文面のまま、検体 4・6・7 は同じ形の小さい入力で、期待値の行を確かめる（`\p[2]`・同じスコープの `group` が 2 つで ID の順に混ざる・`__disabled` と `__parts` が載らない・`scope` の無い `group`・グループ名が空・`group` の外の行・名前の省略・閉じていない `{`・行末の `}`）
  - 既定が消える 3 つ（スコープ 1 に書かれた ID・`__disabled` の中の ID・名前を省略した ID）と消えない 1 つ
  - `option,DisableNoDefineSurfaces` の有無で同じ文字列・同じ ID が 2 度なら両方が書かれた順に載る・既定にも行にも無い ID は載らない
  - 完了の形: 上の各場合のテストが緑になり、規則の 1 か所（例: 集合を載せる行だけから作る）を変えると少なくとも 1 本が赤になる
  - _Requirements: 1.4, 1.5, 2.7, 2.8, 2.9, 2.11, 3.3, 3.4, 3.8, 4.4, 5.9, 5.10, 7.2, 7.3, 7.5_

- [ ] 2.3 シェルのフォルダから `surfacetable.txt` を読み、後退を記録する
  - `surfacetable.txt` だけを読み（`surfaces.txt`・画像・`descript.txt` は読まない）、`charset` を見て読み取り、無ければ Shift_JIS で読む
  - 無いときは記録せず空の転記、開けないときは `warn!` して空の転記、読めない行があれば行番号の列を持つ `warn!` を 1 回だけ出す（前置き `[get_expression_table]`）
  - 同じ一時フォルダに `surface.alias`ブレスと surface*ブレスの `name` を書いた `surfaces.txt` を置いても表が変わらず、`surfaces.txt` にだけ定義された ID が載らないことも確かめる
  - 完了の形: 一時フォルダ（`temp-path-kit`）で、Shift_JIS（`charset` なし）・`charset,Shift_JIS`・`Charset,UTF-8`・BOM 付き UTF-8・BOM なし `charset,UTF-8` のそれぞれでキャラクタ名と説明が元の字のとおりの表になり、ファイルが無い（`warn!` 0 件）・同名のフォルダで開けない（`warn!` 1 件）・読めない行あり（表が返り `warn!` 1 件で行番号を持つ）のテストが `log-capture-kit` で緑になる
  - _Requirements: 1.6, 2.10, 2.12, 3.5, 5.1, 5.2, 5.7, 5.8, 7.4, 7.6_

- [ ] 2.4 ツールの入口のダミーを本物の配線に置き換える
  - 呼ばれるたびに稼働中のゴーストの今のシェルのフォルダを引き、読んで組んだ表を `OK:` の付かない素の値で 1 回だけ答える。引けなければ `warn!` して既定の 15 件で答える
  - World は読むだけで、送り口・SHIORI・サーフェスに触れず、ファイルを書かない。`ghost`・`args` は使わず、`handle` の引数の並びは変えない
  - `NG:not implemented yet` を期待する今のテストを捨てる
  - 完了の形: 空の World で呼ぶと既定の 15 件の全文が返り、`is_error` が偽で、`warn!` が 1 件出るテストが緑になる
  - _Requirements: 1.7, 3.5, 5.9_

- [ ] 3. 結合の確かめ
- [ ] 3.1 実行系つきの単位で、同時の呼び出し・副作用なし・切替の後を確かめる
  - 置き場は `get_expression_table_tests.rs`（1,000 行に近づいたら `get_expression_table.rs` から `#[path]` で 2 本目を繋ぐ。`mcp/mod.rs` には足さない）。型は `mcp_tests.rs` の `real_unit_answers_get_active_ghost_list_in_one_frame`
  - `SwitchRig` で単位を起こし（`rig.boot`）、`mcp::install` で受け口を据え、`wait_steady` で落ち着かせてから、今のシェルのフォルダへ `surfacetable.txt` を置き、偽の SHIORI の呼出の記録（`rig.calls`）の「前」を採る
  - 受け口へ 2 件続けて送って `Input` の段を 1 回回す → 2 件とも同じ表で答え、呼出の記録が増えていない
  - リグの一時の根（ワークツリーの `target\` の下）に別の `surfacetable.txt` のあるフォルダを作り、`set_shell_dir` で替えて呼ぶ → 替えた後の表で答える
  - 最後に `rig.shutdown()` が真であることを確かめる
  - 完了の形: この結合テストが緑になる
  - _Requirements: 1.7, 6.1, 6.2, 6.4_

- [ ] 3.2 触らない約束と全体のテストを確かめる
  - `crates/areka/src/mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/src/**`・すべての `Cargo.toml`・`crates/areka-parsers/src/shell/model.rs`・`decode.rs` に main からの差分が無いことを、pathspec が実在するファイルを指すと確かめてから `git diff` で示す
  - 入口の解決の失敗の文言を見張る今のテスト（`mcp_tests.rs`）が変更なしで緑のまま
  - 完了の形: `cargo test -p areka-parsers` と `cargo test -p areka` と `cargo clippy -p areka -p areka-parsers --all-targets -- -D warnings` が通り、どのテストも SSP・ネットワークに頼らない
  - _Requirements: 6.3, 7.7_
