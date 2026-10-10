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
- **繰り返しの仕組みと、動く絵をコマの列へ分解する仕組みの形**（型・関数の名前）: 下の「繰り返しと分解の仕組みの形」に記した（2026-10-05 に設計から写し、2026-10-06 に `animated-image-playback` が実装した実物と照らして書き直した）。同 spec は `import` の読み手（`crates/areka-parsers/`）と描画メソッドの表（`crates/areka-emo-compose/src/method.rs`）に変更 0 で終わった＝`import` の今の扱いは切り出しの時点のまま。
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

### 繰り返しと分解の仕組みの形（2026-10-06 `animated-image-playback` の実装の後・実物と照らした名前）

- **引き金**: `LoopTrigger::Always { period_ms, laps }`（`crates/areka-seriko/src/table.rs`）。interval の見分けは `is_always_interval`（`crates/areka-emo-compose/src/nesting.rs`・`always` の単独の完全一致）の 1 関数を、合成・見える部品・seriko の表が共有する。表を組むのは `AnimationTable::from_world_and_films`（`from_world` はここへ委ねる）。表に `always` も動く絵の子も無ければ `AnimationTable::is_continuous()` が偽で、足した道を通らない。
- **計算**: `lap_of`・`always_at`（`crates/areka-seriko/src/timeline.rs`・答えは `AlwaysView`）。開始の時刻からの経過だけで今のコマを決める。
- **時計**: 一番上のサーフェスの `always` は `LoopRuntime`（`crates/areka-seriko/src/looper.rs`）の再生の表、部品と動く絵の子は `PartClocks`（`crates/areka-seriko/src/parts.rs`）。時計の鍵は (スコープ, 面の種類 `Slot`) × (部品 `PartKey`, animation の番号)。時計は「見えたと分かった出来事の時刻で、乱数を引かずに生まれる」＝ `LoopRuntime::refresh(scope, slot, at_ms, states)`。出来事の時刻は `SerikoClock`（`crates/areka-seriko/src/actor.rs`・`spawn_seriko_clocked` で注入・刻みと同じ時計）。回数つきの時計は見えなくなったら `LoopRuntime::drop_finite` で捨てる。
- **経過 0**: 経過 0 の絵は合成が定義から描く。求め方は `rest_index`・`rest_pattern`（`nesting.rs`）、描くのは `crates/areka-emo-compose/src/plan_always.rs` の `always_rest_target`（手書きの `always`）と `push_film_op`（動く絵の子）。seriko は `always_at(…, 0)` と同じコマを欄に載せない。欄の読みは `Cell`（`crates/areka-emo-compose/src/pattern.rs`）の 4 つ: `Rest`（載っていない）・`Frame`（サーフェスを指すコマ）・`Picture`（絵を直接指すコマ）・`Blank`（消えている）。
- **外形**: `always` は全部の pattern が入る（`plan_extent.rs` の `flatten_extent`・`plan` の子のモジュール）。動く絵の子は子の原寸（`FilmSheet::original`）を、画像の element と同じ 1 行で数える。見える部品にも経過 0 の先が入る（`NestTable` の 1 行 `SurfaceParts` の `always_rest`）。
- **抽選の対象から外す場所は 2 つ**: `LoopRuntime::on_tick` の抽選の輪と、`parts.rs` の `gate`。
- **動く絵は「子」になる**: 分解は `crates/areka-emo-compose/src/film.rs` の `decompose`（`EmoWorld::bind_atlas` の束縛の直後に呼ぶ）。子の定義は `FilmSheet`（`id`・`path`・`frames`・`delays_ms`・`laps`・`original`・`rest`）で、面の表 1 つぶんを Resource の `FilmSheets` が持ち、`EmoWorld::film_sheet`・`film_sheets`・`film_skips` で引く。分解しなかった絵は `FilmSkip`（理由 `FilmSkipReason`）。子の番号 `FilmId`（親の絵の `ElementId` の値）・鍵 `PartKey::Film(FilmId)`・子を置く element の種類 `ElementKind::Film(FilmId)` は `nesting.rs` に在る。子は seriko の表で animation 0 の `always` を 1 本持ち（コマは `LoopFrame { surface_id: -1, picture: Some(絵の番号), .. }`）、部品の時計で回る。合成は `push_film_op` が欄の `Cell::Picture` の絵を置く（`PatternState::set_film` で欄に載る）。
- **`import` は「pattern定義が動く絵の子を指す」形に載れる**。今、pattern定義のコマが指せるのは作者のサーフェスの番号だけである（絵を直接指す `LoopFrame::picture`・`Cell::Picture` は子の中のコマ 1 枚で、子そのものを指すコマは無い）。足すのは ①読み手が `import` のファイル名を落とさずに運ぶこと ②そのファイルを焼く一覧に載せること ③コマの指す先に「動く絵の子」を足すこと（`PatternFrame`（`pattern.rs`）・`LoopFrame`（`table.rs`）・`plan.rs` の `flatten_surface` のコマの腕・`NestTable::visible_parts` のコマの辺）。子が見える部品になれば、時計・欄・経過 0・外形は `animated-image-playback` のものがそのまま効く。
- そのままでは合わない所が 3 つ:
  - **冒頭の待ち**: `import` の pattern のウエイトは、子の時計とは別に pattern の側で持つ。
  - **繰り返し回数を使わない**（正典）: 子の定義は回数を持つ（`FilmSheet::laps` → `LoopTrigger::Always` の `laps`）。`import` が指す子は回数なし（`laps: None`）で回す必要がある。
  - **同じ画像を element定義と `import` の両方で使うとき**: 子の鍵は画像 1 つにつき 1 つ（`FilmId`＝親の絵の番号・`animated-image-playback` の要件 3.4 のため）なので、時計も欄も 1 つになり、2 つは同じコマで揃ってしまう。`import` は始まりも回数も違うので、`import` の子は別の鍵（例: `PartKey::Film` に「どの pattern が取り込んだか」を足す）にする。鍵を広げるのは `import` の側の仕事である。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: `animated-image-playback`（10-07）が着地し、待つものが無くなった。上の「繰り返しと分解の仕組みの形」は今の main と一致（`FilmSheet`・`PartKey::Film`・`LoopFrame` の `picture`・`Cell::Picture`）。`import` の行は今も読み手でファイル名が 0 に化け、記録が出ない。合成の語の表（`crates/areka-emo-compose/src/method.rs`）は `import` を知らず、未知の語として警告する。
- 触るファイル: `crates/areka-parsers/src/shell/{model.rs, decode.rs}`（pattern がファイル名を運ぶ）・`crates/areka-emo-atlas/src/manifest.rs`（焼く一覧）・`crates/areka-emo-compose/src/{plan,plan_extent,nesting,pattern,film,method}.rs`・`crates/areka-seriko/src/{table.rs, parts.rs}`（コマの指す先・`import` の子の別の鍵・冒頭の待ち）・台帳 `assets.toml` の 1 行・`doc/COMPAT_ARCHITECTURE.md` §8。型 `Pattern` の直書きは約 20 ファイル。
- 規模: 10〜14 タスク（6〜9 から上振れ＝seriko の表と時計の鍵・外形まで及ぶと分かった）。切らない。
- 先に要るもの: 働きの上では無し。ファイルの重なり＝`seriko-trigger-intervals`（読み手の 2 ファイル・seriko の `table.rs`・`parts.rs`）・`collisionex-regions`（読み手の 2 ファイル）・`extent-element-offset`（`plan_extent.rs`）・`element-clipping-option`・`draw-methods-canon`（読み手・`plan.rs`・`method.rs`）。`placement-measure-bake-once`・`present-emit-tail-latency` とは重なり 0。
- 優先度の区分: A（「動く画像」は開発者の依頼 10-01 で、`import` は開発者が確定した範囲の中）。
- 要件定義のモデル: Fable（正典の読みと、読み手・焼く一覧・合成・seriko にまたがる形の選び方）。
- 分割の案: 無し（一度切り出した spec）。
- 見つけた穴・古くなった記述:
  - 「Out of Boundary: seriko の表と時計」は申し送りと食い違う。`import` のコマの指す先と別の鍵は seriko の `table.rs`・`parts.rs` に要る。
  - Constraints の触るファイルに `nesting.rs`・`pattern.rs`・`film.rs`・`plan_extent.rs`・seriko が無い。「想定 6〜9」は上の規模で読み替える。
