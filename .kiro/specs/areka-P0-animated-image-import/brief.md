# Brief: areka-P0-animated-image-import

> 2026-10-05 棚卸㉒で `animated-image-playback` から切り出した（`import` を含めると 18〜22 タスクで上限を超え、しかも `import` だけが読み手・鍵の一覧・合成の命令という別の場所を触るため）。動く画像のテーマ（roadmap「動く画像」）の 1 本。

## Problem

SERIKO の pattern定義の描画メソッド `import`（`animation*.pattern*,import,ファイル名,…`＝サーフェスでなく画像ファイルを直接取り込む）が使えない。しかも読み手が黙って壊す: `crates/areka-parsers/src/shell/decode.rs` の `decode_animations` は pattern の 3 つ目の欄を数として読む（`Pattern.surface_id` は整数）ので、ファイル名は 0 に化け、記録も出ない（転記層は語を落とさない決まりに反する）。`import` のファイルは element でないので、鍵の一覧を作る `ManifestDeriver::derive`（`crates/areka-emo-atlas/src/manifest.rs`）にも載らず焼かれない。

## Current State

- 動く絵の全コマの読み込み（`animated-image-decode`）・部品の入れ子と独立した時計（`surface-element-nesting`）は着地済み。
- `animated-image-playback` が動く絵の自動再生・`always`・バルーンの面を持つ（本 spec を切り出した後の本体）。
- ukadoc の `import` の定義と、他の描画メソッドとの違いは要件の段で引き直す（正典の列挙は ukadoc MCP で確かめる）。

## Desired Outcome

`import` の行が名前を保ったまま読まれ、その画像が焼かれて、pattern の 1 コマとして描かれる。読めない・無い画像は記録を出して良性に飛ばす。

## Approach

読み手の pattern に「サーフェスの番号」と「画像のファイル名」の区別を持たせ、`import` の画像を鍵の一覧へ足して焼き、合成の命令で画像として置く。動く画像（APNG・WebP）を `import` したときの扱いは `animated-image-playback` の分解に乗せる。

## Scope

- **In**: `import` の転記（名前を落とさない）・鍵の一覧への登録・合成で置くこと・記録・決定論テスト・網羅台帳の行。
- **Out**: 自動再生と `always`（`animated-image-playback`）・`--clipping` ほかの element のオプション（`element-clipping-option`）・`base` の描画メソッド（`element-base-method`）。

## Boundary Candidates

- 読み手の pattern の形（`shell/{model,decode}.rs`）
- 焼く一覧（`manifest.rs`）
- 合成の命令（`plan.rs`・`method.rs`）

## Out of Boundary

- seriko の表と時計（`animated-image-playback`・`seriko-trigger-intervals`）。

## Upstream / Downstream

- **Upstream**: `animated-image-decode`・`surface-element-nesting`（完了）。`animated-image-playback`（本体）の後が自然（動く絵を `import` したときの分解を共有するため）。
- **Downstream**: `element-clipping-option`（同じ読み手・`manifest.rs`・`plan.rs`）。

## Existing Spec Touchpoints

- **Extends**: `animated-image-playback` の範囲から `import` を引き取る。
- **Adjacent**: シェルの element の列（parsers の `shell/{model,decode}.rs` と compose の `plan.rs` を触る `element-base-method`・`extent-element-offset`・`collisionex-regions`・`seriko-trigger-intervals`・`element-clipping-option` と同じウェーブに置かない）。

## Constraints

- 触るファイル: `crates/areka-parsers/src/shell/{model.rs, decode.rs}`・`crates/areka-emo-atlas/src/manifest.rs`・`crates/areka-emo-compose/src/{plan.rs, method.rs}`＋兄弟のテスト・`doc/ukadoc-coverage/ledger/assets.toml`。
- 読み手の `Pattern` の形を変えると、構造体を直書きしているテストが多くのクレートで壊れる（`element-base-method` の再測定の知見）。欄を足すより、既存の欄の型を広げる形を設計で比べる。

## 想定

- 規模 S〜M（6〜9 タスク）。○（要件で ukadoc の定義の読みと、`Pattern` の形の選び方の判断がある）。
