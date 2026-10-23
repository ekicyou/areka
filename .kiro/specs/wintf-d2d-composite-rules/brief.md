# Brief: wintf-d2d-composite-rules

> 2026-10-15 起票（`/kiro-discovery`）。`draw-methods-canon` の棚卸㉓の分割の節に「Direct2D の道が選ばれたら、そのとき wintf だけの spec を起票する」と書いてあった spec がこれ。下の「推す形」と「議題」は、着手のときの要件の段で覆してよい。
>
> 出どころ（2026-10-15 開発者）: 「今作ってもらった仕様（`compose-f16-precision`）と関連して、D2D合成アルゴリズムの改善も入れる予定なのです。wintfではコマンドリストの合成ルールが現在固定ですが、これをD2D合成ルールを個別に指定できるようにして、高度な合成を実現したいと考えている。ukadocを読んで、サーフェス合成ルールに関する仕様も確認せよ。」
> 続けて同日の決め: 「画像合成に関するテストは、CPUに切り替えるべきだと思う」「（当たり判定は）座標さえわかれば、１ピクセルだけGPUから書き戻すようなことはできないか？」

## Problem

- wintf は、`Visual` ごとに 1 本の命令の列（command list）を描画面へ描くときの合成の規則が固定されている。`crates/wintf/src/ecs/graphics/systems/render.rs` の `render_surface` が `DrawImage(command_list, None, None, LINEAR, SOURCE_OVER)` と書いている。描く側は「どう重ねるか」を選べない。
- ukadoc の描画メソッド（`overlay` 以外の重ね方と `blend-*` の 28 種）と element定義のオプション（`--alpha`・`--clipping`・`--scaling`）を areka が描くには、重ね方・ブレンド・不透明度・切り抜き・変形を、描くものごとに指定できる口が要る。今は emo の CPU の整数の合成（`areka-emo-compose`）が `overlay` だけを描いている。
- 開発者の方針（2026-10-05）は「合成の方法は Direct2D が支える全部に対応できるよう、wintf も含めて広げる」。2026-10-15 に開発者が「D2D の合成を入れる予定」と決めたので、`draw-methods-canon` の議題 1（CPU の整数の道か Direct2D の道か）は Direct2D の道になった。

## Current State（起票時の実測・main `574bf0a6`）

- **固定されている所は 1 か所**: `render.rs` の `render_surface`（上の `DrawImage`）。ほかに `crates/wintf/src/com/d2d/mod.rs` の補助 `draw_image` も `SOURCE_OVER` 固定。
- **受け口の芽が在る**: `crates/wintf/src/ecs/visual/draw_content.rs` の `VisualDrawContent` は `interpolation_mode` と `composite_mode`（`D2D1_COMPOSITE_MODE`）の欄を持つ。ただしこれを使う描画の木（`crates/wintf/src/ecs/visual/draw/`）は `#![allow(dead_code)]` の作りかけで、本番の `render_surface` は読んでいない。
- **命令の写し取りは揃っている**: `crates/wintf/src/com/d2d/command_sink.rs` の `RecCommandSink` は `SetPrimitiveBlend`（3 世代）と `BlendImage`（`D2D1_BLEND_MODE`・`ID2D1CommandSink5`）を受ける。
- **emo 側の今**: 合成は CPU（`crates/areka-emo-compose/src/blit.rs`・u8 の SourceOver）。結果の `ComposedSurface` を `crates/areka-emo-present/src/display.rs` が B8G8R8A8 の D2D の絵に載せて命令の列に 1 枚描き、wintf がそれを描画面へ描く。描画面は WUC の `CompositionDrawingSurface`（B8G8R8A8・α 掛け済み）で、直接は読み戻せない。
- **CPU の画素に頼っている読み手**: 当たり判定の α マスク（`crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`・emo-present の `presenter/budget.rs`・`cache.rs`）、MCP の `dump_surface`（`presenter/read.rs`）、バルーンの背景色（`emo2_boot/balloon_background.rs` が焼いた絵の (0,0) を読む）。合成を GPU へ移すと、この 3 つの出どころが無くなる。

### ukadoc（正典）のサーフェスの合成の規則（`descript_shell_surfaces`・文は要約）

- 描画メソッドは element定義・pattern定義・着せ替えで使う。重ね方の語: `base`（土台を置き換える）・`overlay`・`overlay-fast`（ベースの不透明度に応じて重ねる）・`interpolate`（ベースの透明度に応じて重ねる）・`replace`（新しいレイヤの範囲の中を透明も含めて上書き）・`asis`（新しいレイヤの透過を無視）・`reduce`（不透明度を掛け合わせる・色は無視・範囲の外は透明）。
- `blend-*` は 28 種（Photoshop・CLIP STUDIO PAINT のレイヤーの合成に当たると書くだけで、式は無い）。`-fast` 付き 27（「ベースの不透明度に応じて」＝外形は変わらない）。古い別名 2（`overlaymultiply`・`overlayscreen`）。
- element定義のオプション: `--clipping,左,上,右,下`（2.8.15）・`--alpha,0-255`（2.8.37）・`--source,値`（2.8.41・PSD のレイヤー名など・`auto` と組む）・`--scaling,横,縦`／`--scaling,サイズ,基準辺`（2.8.73／2.8.82・負の値で反転）。pattern定義にも `--alpha` が在る。
- 関わる設定: `seriko.use_self_alpha`（`blend-*` には `full` を勧める）・`seriko.paint_transparent_region_black`・`seriko.force_disable_crossfade`（サーフェスの切り替えのクロスフェード・2.8.43）。

### D2D との対応（起票時の見立て・要件の段で確かめる）

| ukadoc | D2D |
|---|---|
| `overlay` | `D2D1_COMPOSITE_MODE_SOURCE_OVER` |
| `overlay-fast` | `SOURCE_ATOP` |
| `interpolate` | `DESTINATION_OVER` |
| `replace` | `BOUNDED_SOURCE_COPY` |
| `reduce` | `DESTINATION_IN`（範囲の外も透明になる＝正典どおり） |
| `base` | 土台を消してから描く（`SOURCE_COPY`） |
| `asis` | α を無視して描く（α の扱いを IGNORE にした絵、または色の行列の効果で α を 1 に） |
| `blend-*` | `D2D1_BLEND_MODE`（26 種）。28 種のうち 25 種が当たる（`blend-add`＝Linear Dodge）。当たらない 3 種＝`blend-dither`・`blend-add-glow`・`blend-color-dodge-glow` は自前の効果が要る |
| `-fast` 付き | ブレンドの結果をベースの α で切り抜く（効果の組み合わせ） |
| `--alpha`・`--clipping`・`--scaling` | 不透明度・元の矩形・変形の行列（反転は負の拡大） |

`D2D1_BLEND_MODE` は `ID2D1DeviceContext6::BlendImage`（Windows 10 1903〜）か、ブレンドの効果（`CLSID_D2D1Blend`）で使える。今 WUC の `BeginDraw` が返すのは `ID2D1DeviceContext3`。

## Desired Outcome

- wintf で、描くものごとに合成の規則（重ね方・ブレンドの種類・不透明度・元の矩形・変形）を指定して 1 枚の絵に重ねられる。指定しなければ今と同じ `SOURCE_OVER`。
- 合成の結果を wintf が自前の入れ物（テクスチャ）に持ち、そこから描画面へ出す。この入れ物から、指定した座標の 1 ピクセル（色と α）をその場で読み戻せる。
- 合成の答えの画素は、D2D を CPU の描き手（WARP）で動かす決定論のテストで固定できる。

## Approach

**推す形**（要件の段で覆してよい）:

1. **合成の規則の型を wintf に置く**: 重ね方（`D2D1_COMPOSITE_MODE` の 13）・ブレンド（`D2D1_BLEND_MODE` の 26）・不透明度・元の矩形・変形を持つ「描く 1 件」の型。ukadoc の語は知らない（語との対応は emo の側）。`VisualDrawContent` の欄を生かすか、`GraphicsCommandList` の側に足すかは設計で決める。
2. **固定を外す**: `render_surface` と補助 `draw_image` が型の値で描く。
3. **自前の合成の入れ物**: 合成の結果を wintf の持つ D2D の描き先（テクスチャ）に作り、それを描画面へ写す。途中の入れ物の精度（`compose-f16-precision`）もここで効かせられる（`ID2D1DeviceContext::SetRenderingControls` の buffer precision）。
4. **1 ピクセルの読み戻し**: 合成の入れ物から 1×1 を読み戻し用のテクスチャへ写して読む（`CopySubresourceRegion`→`Map`）。**その場で同期して読む**（前のフレームの値を使い回さない＝「1 フレーム遅らせる解は取らない」）。絵の外形の外と、前の合成から座標が変わらないときは読まない。クリックの透過（今の CPU の α マスク）をこれに置き換えられるようにする。
5. **決定論のテスト**: 合成の答えは D2D を WARP で動かして固定する（開発者 10-15）。実 GPU のテストは「画面に出るか」の確かめだけに残す。

**採らない方向（起票時）**:

- 合成を CPU の整数の転写のまま広げる（開発者 10-15 に D2D の道を決めた）。
- 前のフレームの α を読み戻して使い回す（時刻と状態を 1 フレーム遅らせる）。
- ukadoc の語を wintf に持ち込む（wintf は合成の規則だけを知る）。

## Scope

- **In**: wintf の合成の規則の型と、それで描く `render_surface`／`draw_image`／命令の列の組み立て／自前の合成の入れ物（描き先のテクスチャと描画面への写し）／1 ピクセルの読み戻し／`D2D1_BLEND_MODE` を使える D2D の口（`ID2D1DeviceContext6` へのキャストか効果）／WARP での決定論のテストの足場／wintf の既存のテストの更新。
- **Out**: ukadoc の語と規則の対応・element定義と pattern定義の読み・emo の合成の段の作り替え（`draw-methods-canon`・`draw-methods-blend`）／D2D に無いブレンド 3 種の自前の効果（`draw-methods-blend`）／当たり判定の ID の描き先（`hit-id-pass`）／f16 そのもの（`compose-f16-precision`）／クロスフェード（`seriko.force_disable_crossfade` の対象・別件）。

## Boundary Candidates

- 合成の規則の型（wintf の描画の値）。
- 描く側（`render_surface`・`draw_image`・命令の列）。
- 自前の合成の入れ物と描画面への写し（WUC の描画面・`com/wuc.rs`）。
- 読み戻し（1 ピクセル・同期）。
- 決定論のテストの足場（WARP の装置）。

## Out of Boundary

- ukadoc の語の意味（emo と読み手が持つ）。
- 当たり判定の名前の解決（`collision`・`collisionex` の図形の判定は emo の `hit.rs`）。
- 文字の層（`areka-emo-text`）の描き方。文字の層は自前のスワップチェーンで、本 spec の合成の口を通らない。

## Upstream / Downstream

- **Upstream**: wintf の WUC の口（`com/wuc.rs`・`com/dxgi.rs`）・`ecs/graphics/systems/{surface,render}.rs`・`ecs/visual/draw_content.rs`・`com/d2d/{mod,command_sink,command_types}.rs`。
- **Downstream**: `draw-methods-canon`（emo の合成を D2D の命令の列へ移す）・`draw-methods-blend`（`blend-*`）・`hit-id-pass`（読み戻しの口を使う）・`compose-f16-precision`（途中の入れ物の精度）・`element-clipping-option`（`--clipping`・`--alpha`・`--scaling` を描く口）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。`draw-methods-canon` の棚卸㉓の分割の節が予告していた「wintf だけの spec」。
- **Adjacent**:
  - `draw-methods-canon` — この spec の後に、emo の合成をこの口へ移す。
  - `compose-f16-precision` — 自前の合成の入れ物の形式と途中の精度を分け合う。
  - `hit-id-pass` — 読み戻しの口を使う。
  - wintf の当たり判定（`ecs/layout/hit_region/`・`widget/bitmap_source/alpha_mask.rs`）— クリックの透過を読み戻しへ置き換えるかは議題。

## Constraints

- **決定論のテストは必達**。合成の答えのテストは WARP（開発者 2026-10-15「画像合成に関するテストは、CPUに切り替えるべき」）。WARP の答えは Windows の更新で 1 段ずれうるので、許容差（±1）か見本の作り直しの手順を要件で決める。
- **「時刻は正確に扱う」「1 フレーム遅らせる解は取らない」**: 読み戻しは同期で、その時点の合成の答えを返す。
- **GPU のテストの位置づけ**: 「GPU のテストは実 GPU」（2026-10-08 開発者）は揺れるテストの手当ての決まり。合成の答えの決定論のテストに限って WARP を使うのは 2026-10-15 の開発者の決め。実 GPU のテストは「画面に出るか」を確かめるものとして残す。
- **既定は今と同じ**: 規則を指定しない描き手（wintf のほかのウィジェット）は今の `SOURCE_OVER` のまま描かれる。
- **メモリ**: 自前の合成の入れ物は窓ごとに 1 枚増える（B8G8R8A8 なら描画面と同じ大きさ）。

## 議題（着手のときに開発者へ・答えで作業が変わるもの）

1. **規則の型の置き場**: 作りかけの描画の木（`ecs/visual/draw/`・`VisualDrawContent`）を本番に上げて使うか、今の `GraphicsCommandList` と `render_surface` の側に足すか。
2. **ブレンドの口**: `ID2D1DeviceContext6::BlendImage`（1903 以降が要る）か、ブレンドの効果（`CLSID_D2D1Blend`）か。areka が支える Windows の下限で決まる。
3. **クリックの透過の置き換え**: 1 ピクセルの読み戻しで今の CPU の α マスクを置き換えるか、併せ持つか。マウスが動くたびの同期の待ちの長さを測ってから決める。
4. **WARP の答えのずれ**: 許容差 ±1 にするか、見本の画像を作り直す手順を持つか。

## 2026-10-21 の書き足し（`wintf-visual-tree` の起票）

- **議題 1 は決まった**: 開発者（2026-10-21・`/kiro-discovery`）「`crates\wintf\src\ecs\visual` 層のコード群は実装準備されているが採用されていない。これらを新しいビジュアル層・描画層として取り込んでほしい。D2D合成の多様化に必要と考える」＝作りかけの描画の木（`ecs/visual/draw/`・`VisualDrawContent`）を本番に上げる。上げるのは先行の `wintf-visual-tree`（描画面の下に木を置き、節ごとに重ね方・補間・変形・切り抜きを描く）。
- **この spec の前提が変わる**: Upstream に `wintf-visual-tree` を足す（Dependencies: none → `wintf-visual-tree`）。合成の規則の型は `VisualDrawContent` の側に置く。重ね方（`D2D1_COMPOSITE_MODE`）・補間・変形・元の矩形の切り抜きの描き分けは `wintf-visual-tree` が先に描く。この spec に残るのは、ブレンド（`D2D1_BLEND_MODE`）・不透明度（`VisualFlags` の予約 `OPACITY_GROUP` を使うかは設計で）・自前の合成の入れ物と描画面への写し・1 ピクセルの読み戻し・残りの議題 2〜4。
- **WARP の足場は `wintf-visual-tree` が作る**（Approach 5 の足場はあちらで作り、ここは使う）。議題 4（WARP の答えのずれ）も、先に着手するあちらの要件で決める。
- **下流が 1 本増えた**: `wintf-visual-damage-redraw`（変わった範囲だけ描き直す）は、WUC の描画面が部分の `BeginDraw` を断るので、この spec の自前の合成の入れ物を描き先に使う。入れ物の形（大きさ・形式・写し方）は、部分の描き直しに使える形にする。
