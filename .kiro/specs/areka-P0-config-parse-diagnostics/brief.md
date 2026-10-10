# Brief: areka-P0-config-parse-diagnostics

> 2026-10-10 棚卸㉓で、roadmap の覚え書き「設定の読み取りが捨てた行を言えない」（2026-10-05 `mcp-author-tools` の要件ディスカッションで登記）から起票した。調べた中身の正本は `.kiro/specs/completed/areka-P0-mcp-author-tools/research.md` の 2.5 節。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

ゴーストやバルーンの設定ファイル（`descript.txt`・`surfaces.txt`）に読めない行や、areka が読まないキーがあっても、areka は黙って飛ばす。行番号も記録も残らない。作者は「絵が出ない」「余白がおかしい」という見た目から原因を推理するしかない。MCP で `reload` して `get_log` を読んでも、手がかりは出てこない。

## Current State

- `key,value` の読み取り `parse_kv`（`crates/areka-parsers/src/kv/parse.rs`・43 行）は、カンマの無い行とキーが空の行を飛ばし、同じキーは後勝ちで上書きする。飛ばした行・上書きした行・行番号はどこにも残らない。返すのは `BTreeMap` だけ。
- `surfaces.txt` の読み取り `shell::parse`（`crates/areka-parsers/src/shell/parse.rs`・28 行）は字句（`lexer.rs`）と意味（`decode.rs`・576 行）をつなぐだけで、常に `Shell` を返す。孤立した括弧・閉じていないブレス・知らない先頭語の行は `decode` が黙って捨てる（`research.md` 2.5 節の表）。
- バルーンの descript の読み取り `balloon::parse`・`parse_str`（`crates/areka-parsers/src/balloon/parse.rs`・252 行）は、キーを完全一致で引き、無い・数でない・範囲外は `None`。何も報告しない。後ろの文字の層が一部を `warn!` にする（`crates/areka-emo-text/src/balloon_overrides.rs`）。
- 値で返す前例は 1 つだけ: `parse_surfacetable`（`crates/areka-parsers/src/shell/surfacetable.rs`）が、読めない行の行番号を `SurfaceTable::unreadable` に入れて返し、使う側（`crates/areka/src/mcp/get_expression_table.rs`）が記録にする。
- `areka-parsers` は `tracing` に依存していて、読み取りの中から記録を出している所も 2 つある（`charset/decode.rs` の `debug!`・`package/resolve.rs` の着せ替えの不完全な行の `warn!`）。＝「読み取りは記録を出さない」という決まりは、コードの上では徹底されていない。
- 「areka が読むキー」の一覧はコードのどこにも無い。各読み取りが要るキーを完全一致で引くだけで、引かれなかったキーを集める仕組みが無い（`research.md` 2.5 節）。
- 読み取りの層の決まり: `.kiro/steering/structure.md` は「parse は忠実な転記層」と書く（展開や組み立ては下流）。「記録を出してはならない」とまでは書いていない。

## Desired Outcome

- 設定ファイルの読めない行（行番号つき）と、areka が読まなかったキーが、記録に残る。
- 作者が MCP の `reload`（`mcp-reload`）と `get_log` で、読み直しの直後にそれを確かめられる。
- 読み込みと確かめが同じ読み取りを通る（確かめ用の別の読み取りを作らない）。
- 正しい設定ファイルでは記録が 1 行も増えない。

## Approach

`SurfaceTable::unreadable` と同じ考え方で、読み取りに「捨てたものも返す入口」を足し、既存の入口はそれを呼んで捨てたものを落とす（既存の呼び手を書き換えずに済む）。読まなかったキーは「読んだキーを覚える入れ物」で数える。記録を出すのは読み取りを呼ぶ側（ゴーストとバルーンを載せる所）。値で返すか読み取りの中で記録するかは議題 1 で決める。

## Scope

- **In**: `parse_kv`・`shell::parse`・`balloon::parse` が捨てたもの（読めない行・上書きされた行・読まなかったキー）を取り出す口・載せる所での記録・決定論テスト（捨てる行ごとの赤と緑・正しいファイルで 0 行）。
- **Out**: 設定ファイルを検査する MCP のツール（起票時の名前は `validate_ghost`。本 spec の後でまだ要れば別に起票する）・画像など参照先のファイルが在るかの検査・読めない設定を直して読むこと（読み方は変えない）・`install.txt` と `surfacetable.txt`（後者は既に値で返している）。

## Boundary Candidates

- `key,value` の読み取り（`kv/parse.rs`）。
- `surfaces.txt` の読み取り（`shell/{lexer,decode,parse}.rs`）。
- バルーンの descript の読み取り（`balloon/parse.rs`）。
- 記録を出す所（ゴーストとバルーンを載せる側。場所は設計で決める）。

## Out of Boundary

- 読み取りの結果の型の意味（`Shell`・`BalloonModel` の中身は変えない）。
- 文字の層・seriko が後から出している記録（今のまま）。
- MCP のツールの表（ツールは足さない）。

## Upstream / Downstream

- **Upstream**: 働きの上では無い。作者が確かめる道は `mcp-reload`（未完了）が要る（無くても、起動のときの記録で確かめられる）。
- **Downstream**: 設定ファイルを検査する MCP のツール（要るなら）・`emily-ghost-verification` のような「ほかのゴーストを起こして確かめる」作業（原因探しが速くなる）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: シェルの element の列（`shell/decode.rs` に触る `draw-methods-canon`・`element-clipping-option`・`collisionex-regions`・`seriko-trigger-intervals`・`animated-image-import`・`shell-tooltip`）・文字とバルーンの列（`balloon/parse.rs` に触る `balloon-canon-residue`・`balloon-markers`・`anchor-style-canon`・`anchor-tag-canon`・`emo-text-canon-residue`）・`mcp-reload`。

## Constraints

- 読み取りは今までどおり失敗しない（`Result` を返さない・パニックしない）。読める所は今と同じに読む。
- 記録の洪水にしない。実在のゴーストは areka が読まないキーを数百行持つ（「ゴーストの大きさを甘く見ない」）。読まなかったキーは 1 ファイルにつき件数と先頭の何件かにまとめる、などの上限を要件で決める。
- 優先度は低い（覚え書きの登記のまま）。2 つの列と直列になるので、それらの列が空いたウェーブに入れる。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**: `crates/areka-parsers/src/kv/parse.rs`（43）・`shell/decode.rs`（576）・`shell/parse.rs`（28）・`shell/lexer.rs`（字句が行番号を持たないなら）・`balloon/parse.rs`（252）と各兄弟のテスト・記録を出す所（ゴーストとバルーンを載せる側。`crates/areka-parsers/src/package/` か `crates/areka/src/emo2_boot/` の載せ替えのファイルのどちらかを設計で選ぶ）。
- **規模**: M（9〜13 タスク）。
- **先に要るもの**: なし。同じウェーブに置けない相手は Adjacent の 2 つの列すべて（`shell/decode.rs` と `balloon/parse.rs`）＝単独で入れられるウェーブは少ない。
- **優先度の区分**: C（持ち越し。作者の調べものを助ける改良で、利用者から見える不具合ではない）。
- **要件定義のモデル**: Opus。
- **議題**:
  1. 値で返すか、読み取りの中で記録するか。推しは値で返す（前例 `SurfaceTable::unreadable`・読み取りを純粋なままにできる・テストが値を見るだけで済む）。読み取りの中で記録する案は、呼び手を変えずに済むが、ゴーストの一覧を作るときなど「読むだけ」の場面でも記録が出る。
  2. 読まなかったキーの数え方。各読み取りが引いたキーを覚える入れ物を通すか、areka が読むキーの一覧を 1 つ作るか。前者は読み取りと一覧がずれない。
  3. 記録の上限と水準（読めない行は `warn!`、読まなかったキーは `info!` か `debug!` か。`get_log` で見える水準にする必要がある）。
  4. 設定ファイルを検査する MCP のツールがまだ要るか（本 spec の後で開発者に聞く。本 spec では作らない）。
- **ukadoc の照合**: 不要（areka の記録の話で、正典の主張を含まない）。
