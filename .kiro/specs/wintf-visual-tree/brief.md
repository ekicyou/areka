# Brief: wintf-visual-tree

> 2026-10-21 `/kiro-discovery` で起票。下の「推す形」と「議題」は、着手のときの要件の段で覆してよい。
>
> 出どころ（2026-10-21 開発者）: 「現在、`crates\wintf\src\ecs\visual` 層のコード群は実装準備されているが採用されていない。これらを新しいビジュアル層・描画層として取り込んでほしい。D2D合成の多様化に必要と考える。このコード群で実現可能と思われる機能を推測し、必要な改善を行うスペックを起票せよ。実施フェーズはwintf/dola改修のタイミングとする。」
> 同じ日の答え: 起票は 2 本に分ける（本 spec＝描画の木を本番へ上げる・`wintf-visual-damage-redraw`＝差分の描き直し）。描画の木は**今の WUC の描画面の下に置く**（描画面ごとに 1 本の木を 1 枚に描く。窓や描画面どうしの重ね合わせは今の WUC の木のまま）。

## Problem

- wintf の描画は、描画面 1 枚に命令の列（`GraphicsCommandList`）を 1 本だけ、`SOURCE_OVER` 固定で描く（`crates/wintf/src/ecs/graphics/systems/render.rs` の `render_surface`）。描画面の中で「部品ごとに重ね方・変形・切り抜きを変えて重ねる」口が無い。
- そのため emo のシェルの合成は CPU の整数の転写（`areka-emo-compose`・`overlay` だけ）に閉じていて、ukadoc の描画メソッド・`--clipping`・`--scaling`・`--alpha` を D2D で描く道（`draw-methods-canon`・`draw-methods-blend`・`element-clipping-option`）が乗る土台が無い。
- その土台の芽として、開発者が 2026-08-19〜09-03 に `crates/wintf/src/ecs/visual/` を用意したが、本番から使われていない。`wintf-d2d-composite-rules` の議題 1（「作りかけの描画の木を本番に上げるか、今の `GraphicsCommandList` の側に足すか」）も未決のまま。今回の開発者の指示で「上げる」に決まった。

## Current State（起票時の実測・main `213a3747`）

- **コンパイルはされるが使われていない**: `crates/wintf/src/ecs/mod.rs:13` の `pub mod visual;`。外からの参照は 0（型の名前を `crates/` 全体で探した）・system は 0・スケジュールへの登録も 0・テストも 0。`draw` は私的な `mod draw` で、`build` は `#[allow(dead_code)]` で呼ばれていない。最後の手入れは 2026-09-03（`c999acc5`）。
- **揃っている部品**:
  - `VisualDrawContent`（`draw_content.rs`）: 閉じた `ID2D1CommandList`・局所の外接矩形（96 DPI の DIP）・`D2D1_INTERPOLATION_MODE`・`D2D1_COMPOSITE_MODE`。外接矩形を `GetImageLocalBounds` で求める作り方も在る。
  - `VisualTransform`（`transform.rs`）＋`numerics::Transform2D`: WinUI の Composition と同じ形（基準点・拡大・回転・行列・平行移動）。
  - `VisualClip`（`clip.rs`）: 矩形（`Aabb`）か任意の `ID2D1Geometry`。
  - `VisualFlags`（`visual_flags.rs`）: `VISIBLE` だけ（`OPACITY_GROUP`・`CACHE_AS_BITMAP` の席を予約）。
  - `WorldVisual`（`world_visual.rs`）・`VisualVersion`（`version.rs`）: 積み上げた世界の行列と外接矩形・見た目の版（変形では上げない）。埋める system は無い。
  - `draw/builder.rs` の `build`: 木を深さ優先（画家の順）にたどり、平らな `Vec<DrawCommand>`（描く 1 件・矩形の切り抜きの出入り・図形の切り抜きの出入り）にする。回転した矩形の切り抜きは図形へ上げる。画面と先祖の切り抜きの外は間引く。
- **無い部品**: `DrawCommand` の列を D2D の装置へ描く係・スケジュールへの配線・`WorldVisual` を埋めて `VisualVersion` を上げる system・テスト。
- **直すべき傷**（起票時の読みで見えたもの・ギャップ分析で確かめ直す）:
  1. hash に COM のポインタの番地を使う（`clip.rs`・`draw_content.rs`・`builder.rs` の `hash_draw`）。命令の列が解放されて同じ番地が使い回されると、違う絵を同じと見る。
  2. `build` が `VisualFlags` を読まない（見えない節も描く）。世界の行列を自前で積み、`WorldVisual` を使わない。
  3. 軸に沿うかの判定が 2 通り（`Matrix3x2Ext::axis_aligned` は厳密・`builder::is_rect_preserving` は 1e-4 の許容で 90 度回転も認める）。
  4. `builder.rs` の矩形の交わり・空の判定・変形が `numerics/aabb.rs` の同じ働きと二重。
  5. `geometry_bounds` が `GetBounds` の失敗を `unwrap_or_default()` で 0 の矩形に化かし、記録なしに子の木ごと間引く（「記録の無い失敗の道」）。
  6. 深さの番兵と輪の検出が無い再帰。
  7. `VisualClip`・`VisualDrawContent` の `unsafe impl Send/Sync` に根拠の注記が無い（`GraphicsCommandList` は複数スレッドの factory を根拠に書いている）。
  8. 名前の衝突: `ecs::visual`（この層）と `ecs::graphics::visual`（今の `Visual` 部品・`ecs` の根で再公開）が別物。
- **今の描画の道**: `Draw` スケジュールでウィジェットが `GraphicsCommandList` を記録 → `PreRenderSurface` で描画面を作る → `RenderSurface` の `render_surface` が `BeginDraw`→`Clear`→`DrawImage(SOURCE_OVER)` → `Composition` で WUC の木へ階層・位置・不透明度・切り抜き（`ClipShape`＝矩形と角丸）を写す。位置と拡大の正本は layout の `Arrangement`／`GlobalArrangement`。WUC の描画面は部分の矩形の `BeginDraw` を断る（`render.rs:60-61`・実測）。
- **emo の今**: `areka-emo-compose` が CPU で 1 枚に合成 → `areka-emo-present` の `display.rs` がその 1 枚を命令の列にして描画面の実体（`mount.rs`）に載せる。文字の層（`areka-emo-text`）は自前のスワップチェーンで、wintf の描画の道を通らない。

## Desired Outcome

- wintf の描画面の実体を根として、その下に描画の木の節（`VisualDrawContent`・`VisualTransform`・`VisualClip`・`VisualFlags`）を置けば、木全体が画家の順で 1 枚の描画面に描かれる。節ごとに重ね方（`D2D1_COMPOSITE_MODE`）・補間・変形（回転・拡大・反転を含む）・切り抜き（矩形・任意の図形）が効く。
- 木を持たない描画面（今のウィジェット・emo の今の 1 枚）は、今と同じ `GraphicsCommandList` の 1 本の `SOURCE_OVER` で描かれる（振る舞いを変えない）。
- 上の 8 つの傷が直り、`WorldVisual` と `VisualVersion` を毎フレーム正しく埋める system が在る（差分の描き直し `wintf-visual-damage-redraw` が使う）。
- 描画の木の画素の答えを、D2D を CPU の描き手（WARP）で動かす決定論のテストで固定できる（その足場をこの spec で作る）。数の部品（`numerics`・`build` の間引きと切り抜きの組み立て）は装置なしの純粋なテストで網羅する。

## Approach

**推す形**（要件の段で覆してよい）:

1. **描画面の下に木を置く**（開発者 10-21）: 描画面の実体（`SurfaceGraphics` を持つもの）を木の根とし、その子孫のうち描画の木の部品を持つ節を `build` で平らにする。WUC の子の Visual の木（窓・描画面どうし）はそのまま。どの子孫を「描画面の中の節」と見るか（WUC の Visual を持たない子だけ、など）は設計で決める。
2. **描く係を足す**: `render_surface` の中で、根に木があれば `DrawCommand` の列を描く（描く 1 件＝`SetTransform(世界の行列)`＋`DrawImage(命令の列, 補間, 重ね方)`・矩形の切り抜き＝`PushAxisAlignedClip`・図形の切り抜き＝`PushLayer` の図形の覆い）。木が無ければ今のまま。
3. **今の部品との二重を解く**: 位置の正本は layout（`Arrangement`／`GlobalArrangement`）のまま。描画面の中の節の局所の変形（`VisualTransform`）と、描画面の根の変形（DPI の倍率と位置）の掛け合わせ方を設計で決める。`Visual.is_visible`／`VisualFlags::VISIBLE`、`Visual.clip`（WUC の切り抜き）／`VisualClip`（描画面の中の切り抜き）は、効く場所が違う（WUC の木か描画面の中か）と文書に書き分ける。
4. **傷を直す**: hash は番地でなく版（`VisualVersion` か命令の列ごとの通し番号）で取る・`numerics` の働きへ寄せる・失敗は `error!`＋描かずに飛ばす理由を記録・深さの番兵・`Send/Sync` の根拠の注記・名前の衝突の解き方（設計で決める）。
5. **決定論のテスト**: 装置の要らない部分（`Transform2D`・`Aabb`・`build` の順・間引き・切り抜きの組み立て）は純粋なテスト。画素の答えは WARP の D2D の装置に描いて固定する（`wintf-d2d-composite-rules` の 10-15 の決め「画像合成に関するテストは CPU に切り替えるべき」をこの spec から適用する）。実 GPU のテストは「画面に出るか」の確かめだけ。

**採らない方向（起票時）**:

- 窓ごとに 1 枚へ平らにして WUC の子の Visual の木をやめる（開発者 10-21 に「描画面の下に木」を選んだ）。
- 今の `GraphicsCommandList` に合成の規則の欄を足して描画の木を捨てる（開発者 10-21「取り込んでほしい」）。
- 差分の描き直し（LIS・描き直す範囲・スクロールの写し）をこの spec で作る（`wintf-visual-damage-redraw` へ分けた）。

## Scope

- **In**: `crates/wintf/src/ecs/visual/`（`draw/` を含む）と、そこからだけ使われる `crates/wintf/src/numerics/` の働きの本番化と傷の直し・描く係・`render_surface` の木の分岐・スケジュールへの配線（`WorldVisual` を埋め `VisualVersion` を上げる system を含む）・`ecs` の根での公開の形・WARP の決定論のテストの足場・純粋なテスト・wintf の example 1 本（木で重ね方・回転・切り抜きを描く）・`.kiro/steering/structure.md`／`tech.md` の描画の道の記述。
- **Out**: 差分の描き直し（`wintf-visual-damage-redraw`）／`D2D1_BLEND_MODE`・不透明度の層・自前の合成の入れ物・1 ピクセルの読み戻し（`wintf-d2d-composite-rules`）／emo の合成を描画の木へ移すこと（`draw-methods-canon`）／ukadoc の語（`blend-*`・`--clipping` ほか）の対応／文字の層（`areka-emo-text`）の描き方／WUC の木の側の切り抜き（`Visual.clip`・`clip_sync_system`）の作り替え／f16 の途中の入れ物（`compose-f16-precision`）。

## Boundary Candidates

- 描画の木の部品と数の部品（`ecs/visual/*`・`numerics`）＝データと純粋な計算。
- 平らにする係（`build`）と描く係（D2D の装置への再生）。
- 本番の描画の道への差し込み（`render_surface` の分岐・スケジュール・`WorldVisual`／`VisualVersion` の system）。
- 決定論のテストの足場（WARP の装置・画素の見本）。

## Out of Boundary

- 何をどう重ねるかの意味（ukadoc の語・SERIKO の pattern）は emo と読み手が持つ。wintf は D2D の値だけを知る。
- 前フレームとの比べ（`wintf-visual-damage-redraw`）。
- 当たり判定（`hit-id-pass`・`AlphaMask`）。

## Upstream / Downstream

- **Upstream**: 見直し（v0.2.0 と重なる分＝本 spec より先に取る）の `audit-wintf-graphics-world`（`ecs/visual/` を受け持つ）・`audit-wintf-runtime`（`numerics` を受け持つ）が先に着地する。どちらにも「`ecs/visual/` と、そこからだけ使われる `numerics` の働きを、未使用を理由に消さない」と書き足した（2026-10-21）。今の描画の道（`ecs/graphics/systems/{render,surface}.rs`・`com/wuc.rs`・`com/d2d/`）。
- **Downstream**: `wintf-d2d-composite-rules`（合成の規則の型を `VisualDrawContent` の側に置く・ブレンドと不透明度と自前の入れ物を足す）→ `draw-methods-canon`（emo の合成を描画の木へ移す）→ `draw-methods-blend`・`element-clipping-option`（`--clipping` を `VisualClip` で描く）。`wintf-visual-damage-redraw`（`WorldVisual`・`VisualVersion`・`DrawCommand` の列を使う）。v0.4.0 の `shell-balloon-scale-setting`（変形の掛け合わせ）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。`wintf-d2d-composite-rules` の議題 1 の答えを、この spec が先に形にする。
- **Adjacent**:
  - `wintf-d2d-composite-rules` — 本 spec の後。合成の規則の型（重ね方・補間）は本 spec が `VisualDrawContent` のまま本番へ上げ、ブレンド・不透明度・自前の入れ物・読み戻しはあちら。WARP の足場は本 spec が作り、あちらが使う。
  - `audit-wintf-graphics-world`・`audit-wintf-runtime`（見直し・本 spec より先）— 同じファイルを先に見直す。直しが `ecs/visual/` に及んだら、本 spec の傷の一覧と突き合わせる。
  - `compose-f16-precision` — 途中の入れ物の精度。本 spec は描画面の形式を変えない。

## Constraints

- **既定は今と同じ**: 木を持たない描画面の描き方・WUC の木の振る舞い・emo の今の表示を変えない。全体テストの前後で、既存のウィジェットと emo の画素のテストが同じ答え。
- **決定論のテストは必達**。画素の答えは WARP（WARP の答えが Windows の更新で 1 段ずれうる件の扱いは `wintf-d2d-composite-rules` の議題 4 と揃える＝先に着手する本 spec の要件で決める）。
- **DPI 追従が基本設計**: 局所の外接矩形は 96 DPI の DIP。拡大は D2D の変換の行列（09-11 裁定）。
- **記録の無い失敗の道を作らない**（`error!`＋`Err`、描かずに飛ばすなら理由を記録）。panic は致命のときだけ。
- 1 ファイル 1,000 行の番人を越えない。
- `wintf` は crates.io に出す汎用のライブラリ＝公開の型の名前と形は areka の外の使い手にも通じるものにする。areka の語を持ち込まない。
- 依存を足すなら `tech.md` へ登記して開発者の承認を得る（本 spec では足さない見込み）。

## 議題（着手のときに開発者へ・答えで作業が変わるもの）

1. **「描画面の中の節」の見分け方**: 描画面の実体の子孫のうち、WUC の Visual を持たない子を木の節とするか、専用の印の部品を置くか。
2. **名前の衝突の解き方**: `ecs::visual`（この層）を改名するか（例: `ecs::draw_tree`）、今の `ecs::graphics::Visual` の側を改名するか。公開の型の名前が変わる。
3. **WARP の答えのずれ**: 許容差 ±1 にするか、見本の画像を作り直す手順を持つか（`wintf-d2d-composite-rules` の議題 4 をこちらで先に決める）。
