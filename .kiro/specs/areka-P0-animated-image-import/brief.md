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


## 2026-10-05 `animated-image-playback` からの申し送り（同 spec の要件 5.2）

> この節は `areka-P0-animated-image-playback` が生きている間、同 spec の側で正しく保つ。着手時に引き直すこと。

- **経緯**: `animated-image-playback` は要件討議（2026-10-05）まで `import` を範囲に持ち、受け入れ基準 10 項を書いていた。同日の棚卸㉒の裁定で本 spec へ切り出されたので、書いた基準を下にそのまま渡す（要件の下書きとして使える。正典の逐語は同 spec の requirements.md の C4）。
- **同 spec の要件討議で決まり、`import` にも効くこと**:
  - 時刻は正確に扱う（開発者 2026-10-05）。待ち時間は丸めない。画面の更新が遅れたら過ぎた時間の分だけ進める。
  - シェルとバルーンに設計上の違いは無い（開発者 2026-10-05）。面の種類で能力を仕切らない。
  - 置くだけで動く自動アニメーションはファイルの繰り返し回数を守る（「合計 N 回」の絵は表示のたびに始め直す）。`import` は正典どおり回数を使わない。この違いは対応表（`doc/COMPAT_ARCHITECTURE.md` §8）に並べて書く。
- **繰り返しの仕組みと、動く絵をコマの列へ分解する仕組みの形**（型・関数の名前）: `animated-image-playback` の設計の段で決まりしだい、ここへ書き足す（未記入）。
- **ギャップ分析の材料**: `animated-image-playback` の research.md 3.2 節（`import` の今の読まれ方と、直さないと残る 3 か所）・6 章の 6（末尾のコマ・混在・`surface.append` の行にだけ現れる絵）。

### 渡す受け入れ基準の下書き（`animated-image-playback` の旧 要件 5・原文のまま）

### Requirement 5: 描画メソッド `import`

**Objective:** シェルの作者として、`animation*.pattern*,import,ファイル名,ウエイトmsec,X,Y` と書いて、動く絵をアニメーションの 1 本として取り込みたい。そうすれば、動く絵を始めるきっかけ（interval）と位置を、ほかのアニメーションと同じ書き方で決められる。

#### Acceptance Criteria

1. When pattern定義の描画メソッドが `import` で、ファイル名が動く絵（APNG・動く WebP）を指す, the areka shall その絵の全部のコマとコマごとの待ち時間を、そのアニメーションのコマとして取り込む（C4）。
2. When `import` で取り込んだアニメーションが始まる, the areka shall pattern定義のウエイト（ミリ秒）だけ待ってから 1 枚目のコマを表示し、以後はファイルのコマごとの待ち時間で進める（C4「冒頭待機時間」）。
3. The areka shall 取り込んだコマを、コマの左上が pattern定義の X,Y に来る位置に、`overlay` と同じ重ね方で描く（C4）。
4. The areka shall 取り込んだアニメーションをいつ始め、繰り返すかどうかを、そのアニメーションの interval だけで決め、ファイルの繰り返し回数を使わない（C4「無視される」）。interval が `always` なら最後のコマの後に頭（冒頭の待ちを含む）へ戻って繰り返し、`random` などの抽選の語なら 1 回再生して次の抽選を待つ。
5. The areka shall `import` で取り込んだアニメーションを、同じサーフェスのほかのアニメーションと並べて動かし、互いに止めない（C4「並列で動作し共存可能」）。
6. If `import` のファイル名が指すファイルが無い、または読めない, then the areka shall その pattern定義を描かず、サーフェスの番号・アニメーションの番号・ファイル名・理由を `warn!` で記録し、同じサーフェスのほかの絵とアニメーションは今までどおり描く。
7. When `import` のファイル名が指す絵が静止画（動く GIF・1 枚へ縮められた絵を含む）である, the areka shall その 1 枚を、ウエイトと X,Y に従う 1 コマのアニメーションとして `overlay` で描き、絵が動く絵として扱われなかったことを `warn!` で 1 回記録する。
8. The areka shall `import` の pattern定義だけが名指しする画像ファイル（element定義にも `surface*.png` にも現れないファイル）も、取り込める。
9. The areka shall `import` のファイルに、読み込みの側の 3 つの上限（コマの枚数・絵 1 つの画素の量・シェルの合計の画素の量）を、element定義の動く絵と同じに効かせる。
10. The areka shall 1 つのアニメーションに `import` の pattern定義とほかの pattern定義が混ざっているときの振る舞いを設計で定め、正典が黙っている箇所の記録（要件 10.2）に記す。どう定めても、記録を残さずに pattern定義を捨てる経路は 0 本とする。
