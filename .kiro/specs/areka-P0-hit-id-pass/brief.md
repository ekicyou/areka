# Brief: areka-P0-hit-id-pass

> 2026-10-15 起票（`/kiro-discovery`）。下の「推す形」と「議題」は、着手のときの要件の段で覆してよい。
>
> 出どころ（2026-10-15 開発者）: 「αに関連して、当たり判定用に『MRT（IDパス描画）？』とかを導入して、ヒットテストIDだけを持ったサーフェスを持つこともありだと思う。ID自体は8/16/32bitから選べるべきか？これはwintf層ですべきか、areka層ですべきか疑問がある。」「シェル仕様で、Headなどのリージョン範囲を、画像で取り込む仕様があるはずです。これに対応するためにもヒットテストIDが欲しいと考えました」

## Problem

- ukadoc の当たり判定には、画像で範囲を取り込む書き方が在る: `collisionex*,ID,region,ファイル名,R,G,B(,領域反転)`（SSP 2.5.19〜・「指定した画像ファイルの指定色の領域を当たり判定として扱う」・「大量に(100～)使用すると全体的に動作が重くなるので注意」）。areka はこれを読まない。`Head` などの範囲を画像で描いたシェルでは、撫で・クリックの Reference4（当たり判定の名前）が届かない。
- 当たり判定の図形（矩形・楕円・円・多角形・画像の領域）を 1 枚の「ID だけを持つ絵」に描いておけば、当たり判定は「その座標の 1 ピクセルを読む」だけにまとまる。重なりは描く順で決まり、決めてある「当たり判定の重なりは画家のアルゴリズム（後に定義したものが手前）」とそのまま合う。

## Current State（起票時の実測・main `574bf0a6`）

- **areka の当たり判定は CPU の図形の判定**: `crates/areka-emo-compose/src/hit.rs` の `hit_region`／`hit_region_in`／`hit_region_scaled_in`（矩形と名前・後に定義したものが手前）。読み手 `crates/areka-parsers/src/shell/decode.rs` の `decode_collisions` は `collisionN`（矩形）だけを読み、`collisionex` は読み飛ばす。
- **`collisionex` の 4 つの図形は `collisionex-regions` が受け持つ**（矩形・楕円・円・多角形）。同 spec の brief の棚卸㉓の節は「`region` は画像を読む仕組みが要るので、本 spec では読めた行を記録して使わず、別に起票するのが筋」と書いている＝`region` の持ち主はまだ居ない。
- **wintf には CPU の色分け画像の当たり判定が既に在る**: `crates/wintf/src/ecs/layout/hit_region/mod.rs` の `ColorMapData`（PNG を読み、画素 → 領域 ID の `index_map: Vec<u8>`・ID 0＝無名・名前の表）と `HitRegionMap::from_color_map`（完了 `event-hit-test-named-regions`）。areka はこれを使っていない（使い手は wintf の例と `hit_test` の中だけ）。ID は 8 ビット。
- **クリックの透過は CPU の α**: `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`（閾値 128）と emo-present の `presenter/budget.rs`・`cache.rs`（合成済みの CPU の画素から作る）。合成を D2D へ移す（`draw-methods-canon`）と、この CPU の画素は無くなる。読み戻しの口は `wintf-d2d-composite-rules` が作る（1 ピクセル・同期）。

## Desired Outcome

- `collisionex` の `region` を読み、画像の指定色の領域（領域反転を含む）に当たったとき、その名前が Reference4 に載る。
- 当たり判定の図形を窓ごとの「ID の描き先」に描き、当たり判定は 1 ピクセルを読んで ID → 名前を引く。当たったかどうかは色の側の α（`wintf-d2d-composite-rules` の読み戻し）で決める（`reduce`・`asis` で α が変わった所は ID が残っていても透ける）。
- 拡大率（DPI・`\![set,scaling]`）や element の変形が入っても、ID の描き先は色の絵と同じ変形で描くので、図形ごとの逆変換を書かずに済む。
- 決定論のテストで、図形ごとの内外・重なり・`region` の色と反転を固定する。

## Approach

**推す形**（要件の段で覆してよい）:

- **層の分け方＝仕組みは wintf、意味は areka**（開発者の問い「wintf層か、areka層か」への答え）。
  - wintf: 窓ごとの 2 枚目の描き先（ID の描き先）・ID の描き方（縁のぼかしなし・最近傍・上書き・ブレンドなし）・1 ピクセルの読み戻し（色の α と ID）・ID の幅。ukadoc の語は知らない。
  - areka: どの ID がどの当たり判定の名前か（表）・`collision`／`collisionex` の定義から ID の描き先に描く図形を組むこと・`region` の画像を読んで指定色の領域を ID の形にすること。
- **ID の幅は選べなくてよい**（開発者の問い「8/16/32bit から選べるべきか」への答え・起票時の見立て）。D2D には MRT も整数の描き先も無い。B8G8R8A8 を α なし（`D2D1_ALPHA_MODE_IGNORE`）で使えば 24 ビットの ID がそのまま使える。8 ビット（`A8`）はメモリが 4 分の 1 だが、正典が `region` を 100 以上使う書き方を想定しているので余裕が少ない。幅を 1 つに決めれば決めごとも 1 つ減る。
- **`region` の画像は読み込みのときに CPU で形にする**: 指定色の画素だけを残した α のマスク（反転を含む）を作っておき、ID の描き先には「ID の色の塗り × マスク」で描く。wintf の `ColorMapData` の読み方（WIC・色 → ID）を流用できる。
- **ID の描き先は合成と別に描く**: D2D は 1 回に 1 枚しか描けないので、色の合成と同じ変形で 2 回目を描く。描き直すのはサーフェス・当たり判定の定義・変形が変わったときだけ。

**ほかの形（議題 1）**: GPU の ID の描き先を作らず、wintf に在る CPU の `ColorMapData`（`index_map`）を `region` に使い、図形は今の CPU の判定のまま、点の側を逆変換して引く。GPU の読み戻しの待ちが無く、決定論も今のまま。代わりに、変形（拡大率・element の `--scaling`・反転）ごとに逆変換を書き、ID の表は 8 ビットのまま。

**採らない方向（起票時）**:

- 色の絵の画素の色そのものを当たり判定に使う（ukadoc の `region` は別の画像の指定色）。
- ID の幅を描き手ごとに選べる形（使い分ける者が居ない）。
- 前のフレームの ID を読み戻して使い回す（「1 フレーム遅らせる解は取らない」）。

## Scope

- **In**: `collisionex` の `region` の読み（読み手の型に `region` の欄＝ファイル名・R・G・B・反転）・`region` の画像の読み込みとマスク化・wintf の ID の描き先と ID の描き方と読み戻し・areka の ID の表と図形の組み立て・当たり判定の解決を「ID を読む」へ替えるか（議題 1）・網羅台帳の `region` の行・`doc/COMPAT_ARCHITECTURE.md`・決定論のテスト・実機の確かめ。
- **Out**: `collisionex` の 4 つの図形の読みと CPU の判定（`collisionex-regions`）／`animation*.collision*`・`animation*.collisionex*`（アニメーション中だけの当たり判定）／カーソルの形／色の合成の規則と色の側の読み戻しの口（`wintf-d2d-composite-rules`）。

## Boundary Candidates

- 読み手の `region` の欄（`shell/{model,decode}.rs`）。
- `region` の画像の読み込みとマスク化（色の一致・反転）。
- wintf の ID の描き先・描き方・読み戻し。
- areka の ID の表と図形の組み立て（emo の当たり判定の段）。
- 当たり判定の解決（`hit.rs`）の差し替え。

## Out of Boundary

- マウスのイベントの送り方そのもの（kanade・`input_events`）。名前が載るだけ。
- ukadoc の語の意味を変えること。
- 設定画面。

## Upstream / Downstream

- **Upstream**: `collisionex-regions`（`collisionex` の行を読む読み手と 4 つの図形）・`wintf-d2d-composite-rules`（自前の合成の入れ物と 1 ピクセルの読み戻しの口）・完了 `event-hit-test-named-regions`（wintf の `ColorMapData`）・完了 `surface-element-nesting`（子の当たり判定の持ち込み `hit_import.rs`）。
- **Downstream**: 撫で・クリック系のイベント全般（Reference4 を使うゴースト）・将来の `animation*.collisionex*`。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。`collisionex-regions` が「別に起票するのが筋」と書いた `region` を引き取る。
- **Adjacent**:
  - `collisionex-regions` — 読み手の `collisionex` の行。**`region` の行も型に運ぶ形にしてもらう**（本 spec が読み手に触らずに済むように＝`draw-methods-canon` と `draw-methods-blend` の分け方と同じ）。同 spec の brief に申し送りを足した。
  - `wintf-d2d-composite-rules` — 読み戻しの口と自前の入れ物。ID の描き先は同じ変形で並べて持つ。
  - `draw-methods-canon` — 合成を D2D へ移すと CPU の α マスクの出どころが無くなる。クリックの透過の置き換えはそちらと `wintf-d2d-composite-rules` の議題。
  - シェルの element の列の読み手の鎖（`shell/{model,decode}.rs` を分け合う spec 群）— 読み手に触るなら同時に走らせない。

## Constraints

- **決定論のテストは必達**。ID の描き先の答えも、合成と同じく WARP で固定する（開発者 2026-10-15「画像合成に関するテストは、CPUに切り替えるべき」）。ID は縁のぼかしなしで描くので、機種で揺れない見込み。
- **「1 フレーム遅らせる解は取らない」**: 読み戻しは同期。
- **正典**: 意味は ukadoc から取る。`region` の画像の座標はサーフェスの座標と同じに置く（正典は書いていない＝§8 に決めを記す）。
- **負荷**: 正典の「100 以上で重い」の注意に対し、画像の読み込みは 1 回、ID の描き先の描き直しは定義か変形が変わったときだけにする。

## 議題（着手のときに開発者へ・答えで作業が変わるもの）

1. **GPU の ID の描き先か、CPU の色分け画像か**: 推す形は GPU の ID の描き先（変形が入っても 1 ピクセル読むだけ・図形と画像をまとめられる）。wintf に在る CPU の `ColorMapData` を `region` に使う形なら、GPU の読み戻しは要らず、変形ごとの逆変換を書く。
2. **図形も ID の描き先へ寄せるか**: `collisionex-regions` の 4 つの図形と `collisionN` の矩形も ID の描き先に描いて当たり判定を一本化するか、図形は CPU の判定のまま `region` だけを ID の描き先にするか。
3. **ID の幅**: 24 ビット（B8G8R8A8 を α なし）で決め打ちにするか。メモリを優先して 8 ビット（`A8`・255 まで）にするか。
4. **`region` の画像の座標と大きさ**: 画像がサーフェスより小さい・大きいときの扱い（正典に記述が無い）。
