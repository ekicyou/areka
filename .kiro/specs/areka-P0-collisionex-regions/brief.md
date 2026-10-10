# Brief: areka-P0-collisionex-regions

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-mouse-drag-events` の完了時の棚卸（`.kiro/specs/completed/areka-P0-mouse-drag-events/tasks.md` の Implementation Notes・`verification/real-machine.md` 4 章 ⑵）。開発者の方針（2026-10-05）「実機で areka が未対応だったためにうまくいかなかった件はすべて起票」。

## Problem

- **利用者・ゴーストの作者**: 当たり判定を `collisionex`（矩形・円・楕円・多角形）で書いたシェルでは、areka のマウスのイベントの Reference4（当たり判定の名前）がいつも空になる。クローディアの当たり判定は全部 `collisionex` なので、撫で（`OnMouseMove`）・ダブルクリック・ドラッグのどれでも「どこを触ったか」がゴーストに届かない。
- `mouse-drag-events` の実機では、R1・R2 の 4 回のドラッグで Reference4 が全部空だった（クローディアのドラッグの台詞は当たり判定を見ないので確認には響かなかった）。

## Current State

- 読み手 `crates/areka-parsers/src/shell/decode.rs` の `decode_collisions` は、番号が数字だけの `collisionN`（矩形）だけを値にし、`collisionex` の行は記録なしに読み飛ばす。
- 当たり判定の解決は `crates/areka-emo-compose/src/hit.rs`（矩形と名前）。areka の `input_events/mod.rs` の doc にも「`collisionex` は実装しない（7.4）」とある。
- 台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `collisionex` の行は `absent`・担当なし（優先度 A12・価値「触れ合い」）。

## Desired Outcome

- `collisionex*,名前,rect|ellipse|circle|polygon,…` を読み、当たり判定の解決が形に応じて内外を判定する。`collision` と `collisionex` の重なりは今の画家の順（後に書いたものが手前）に揃える。
- 読めない行（形の名前が未知・座標の数が足りない）は記録を 1 件残す。
- 決定論のテストで 4 つの形の内外と重なりを固定し、実機でクローディアの撫でとドラッグの Reference4 に名前（`Head`・`Bust` など）が載ることを確かめる。

## Approach

ukadoc の `collisionex` の書式（`descript_shell_surfaces`）を引き直し、読み手に形の種類を足して、`hit.rs` の判定を形ごとに広げる。入れ子のサーフェス（`surface-element-nesting` で持ち込む子の当たり判定）にも同じ型で乗せる。

## Scope

- **In**: `collisionex` の読み・形ごとの内外判定・重なりの順・記録・台帳の行・`doc/COMPAT_ARCHITECTURE.md`・決定論のテスト・実機の確認。
- **Out**: `animation*.collision*` の当たり判定（アニメーション中の当たり判定）・カーソルの形（`The Hand`）。

## Boundary Candidates

- 読み手（`shell::{model,decode}`）
- 当たり判定の解決（`areka-emo-compose` の `hit.rs`）

## Out of Boundary

- マウスのイベントの送り方そのもの（kanade・`input_events`）は今のまま。名前が載るだけ。

## Upstream / Downstream

- **Upstream**: 完了 `shell-parse`・`emo-compose`・`surface-element-nesting`（子の当たり判定の持ち込み）。
- **Downstream**: 撫で・クリック系のイベント全般（Reference4 を使うゴースト）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `element-base-method`・`element-clipping-option`（同じ `decode.rs`）——同時に走らせない。

## Constraints

- 段は**優先**（シェルの element の列・既存のゴーストの当たり判定がまるごと効かない）。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（7〜10 タスク）。切らない。
- 前提の状態: 働きの前提（`surface-element-nesting`）は着地済み。roadmap の依存 `element-base-method` は働きの依存ではなく、読み手の 2 ファイル（`shell/model.rs`・`shell/decode.rs`）を分け合うための順番待ち。`element-base-method` の着地の後に着手できる。
- 崩れた前提／古くなった位置:
  - Current State は今も正しい。読み手の `decode_collisions` は番号が数字だけの行だけを `Collision` にし、`collisionex0` のような行は記録なしに飛ばす。キャラクター窓の押下のハンドラ `on_char_pointer_pressed` の doc にも「collisionex は実装しない（7.4）」が残る。
  - `surface-element-nesting` が子の当たり判定を親へ持ち込む `hit_import.rs` を足した。その中の `regions_of` は矩形の 4 辺へ位置のずれを直接足して写す＝形を足すと、ここも形ごとにずらす必要がある（brief の「入れ子にも同じ型で乗せる」の実体）。
  - 表示層の `presenter/hit.rs` は点の側を縮めてから `hit_region_in`／`hit_region_scaled_in` に渡す＝形が増えても縮め方は変わらない。
  - クローディア（`vendors/sample_ghost/claudia.nar`）の `surfaces.txt` は `collisionex` が 93 行・`element*,base` が 5 行（実数え）。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/shell/{model.rs, decode.rs}`（`Collision` の型・`decode_collisions`）
  - `crates/areka-emo-compose/src/{hit.rs, hit_import.rs}` と兄弟のテスト（型を別の列にするなら `normalized.rs`・`fold.rs`・`world.rs` も）
  - `crates/areka/src/input_events/mod.rs`（doc の 1 行だけ）
  - `doc/ukadoc-coverage/ledger/assets.toml`（`collisionex*` の行と、`collision*` の行の注記）・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）:
  1. 今の `Collision` に形の欄を足すか、別の型の別の列にするか。推しは前者。`collision` と `collisionex` の重なりを「後に書いたものが手前」に揃えるには、2 つを書いた順に 1 つの列へ並べる必要があり、別の列では順が失われる。前者なら構造体リテラルの直しは本番 2 か所（`decode.rs`・`hit_import.rs`）と試験の約 10 ファイル。
- 見つけた穴: 無し。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: `element-base-method`（10-05）が着地し、読み手の 2 ファイルの順番待ちが解けた。読み手の `decode_collisions` と当たり判定の `hit.rs`・`hit_import.rs` は棚卸㉒から変更 0。`wintf-tooltip`（10-08）が入り、後の `shell-tooltip` が当たり判定の名前を使う相手になった。
- 触るファイル: `crates/areka-parsers/src/shell/{model.rs, decode.rs}`、`crates/areka-emo-compose/src/{hit.rs, hit_import.rs}`（形ごとの内外の判定は新しいファイルへ出す）、`crates/areka/src/input_events/mod.rs` の説明の 1 行、台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 2 行、`doc/COMPAT_ARCHITECTURE.md` §8。型 `Collision` に欄を足すと、値を直書きしているテストが 12 ファイル（parsers 7・emo-compose 3・emo-present の `presenter_film_tests.rs`・`presenter_test_support.rs`）。
- 規模: 8〜11 タスク（7〜10 から 1 上げ＝下の `region` の扱い）。切らない。
- 先に要るもの: 働きの上では無し。読み手の 2 ファイルを `seriko-trigger-intervals`・`animated-image-import`・`element-clipping-option`・`draw-methods-canon` と分け合うので、どれとも同時に走らせない。`extent-element-offset`・`placement-measure-bake-once`・`present-emit-tail-latency` とは重なり 0。
- 優先度の区分: A（起票の根拠が開発者の指示「実機で未対応だった件はすべて起票」）。指示は包括のものなので、個別に見れば C（正典の拾い残し）。
- 要件定義のモデル: Opus（正典の書式がはっきりしている）。
- 分割の案: 無し。
- 見つけた穴・古くなった記述:
  - ukadoc の `collisionex` の形は 4 つでなく 5 つ。5 つ目は `region`（画像ファイルの指定した色の領域を当たり判定にする・SSP 2.5.19 から・書式は「ファイル名,R,G,B,領域反転」）。本文も台帳の行も 4 つしか書いていない。`region` は画像を読む仕組みが要るので、本 spec では読めた行を記録して使わず、別に起票するのが筋（議題に足す）。
  - 書式は `collisionex*,ID,タイプ,座標…` で、名前が 2 番目に来る（`collision*` は最後）。読み手で取り違えない。
  - 読み手を `shell/boxes.rs` と同じ「2 つ目の転記」にすれば `shell/{model,decode}.rs` に触れずに済むが、`collision` と書いた順を合わせるために行の順番を運ぶ必要がある（議題 1 の第 3 の案）。
