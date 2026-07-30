# LLM参照を禁じる。このドキュメントは人間用です。LLMは参考にせず、書き込みも禁止


## emo_worldのコンポーネント構造

emo_worldの設計に整理が必要。以下の概念を中心に検討せよ。

### 要件の検討
#### 1つのemo_worldはどこまでを責務（データ保持）とするか？
1. 1ウィンドウ内の描画まで。ウィンドウ表示（位置など）は責務外。
2. 1ウィンドウ内の描画、およびウィンドウ表示を責務。
3. 全ウィンドウ（シェル）の描画を責務とする。

#### emo_worldで扱うコンポーネントは？
+ window entity
  + virtual_window：仮想窓、シェル原点（中央下）を軸。
  + window：実窓の管理。
  + update_rect：virtual_windowに対するRECTを持つ。
  + views：n個のview entityを持つ。
  + render：レンダリングツリー

+ view entity
  + update_rect：virtual_windowに対するRECTを持つ。
  + surface：シェル / バルーンの描画。1個のsurface entityを持つ。

+ surface entity
  + rect：描画矩形を持つ。
  + picture：画像を持つ。
  + animations：n個のanimation entityを持つ。
  + text_area：タイプライター領域

+ animation entity
  + animation：アニメーション。n個のelement entityを持つ。

+ element entity
  + update_rect：virtual_windowに対するRECTを持つ。
  + element：1個のsurface entityとアニメーション情報を持つ。


+ visual entity
  + visual
    + offset(i32,i32)：view_boxのスクロール位置
    + mat：変換行列
    + render：render entity
    + children：n個のvisual entity
  + visual_dirty
    + dirty_flag：更新を検出すればtrue
    + root_mat：ルートからの変換行列
    + aabb：（render領域rect * root_mat）

+ render entity
  + rect：描画矩形
  + command_list：描画コマンド


## lis crateを使う



## viewbox + visualの管理

### 重要要件

+ 物理ピクセル（i32）単位でスクロールを管理したい
+ viewboxがスクロールしたときに、
  ビットマップをスクロール貼り付けしてダーティを最小化したい。

### viewbox

#### 最終的な焼き付けに必要な情報
結果として、ワールド（ウィンドウ物理描画情報）に対して、以下の情報を必要とする。

+ 描画枠
  + RECT：ワールド座標に対する、外枠の位置
+ 内部変換
  + 物理スクロール量（i32,i32）
  + 物理スクロール量を除いた、ワールド変換行列


#### 変換戦略
+ 未変化を含めて、もっとも大きなスクロール変化領域を確定
  + old DOMと、new DOMの比較
    + 物理スクロール量だけが変化→スクロール予約




#### パラメーター

+ viewbox：論理情報
  + layout（論理位置、サイズ）
  + scroll（論理スクロール量）
  + scale
+ world_viewbox：物理ピクセル焼き付け情報
  + rect（ワールド座標に変換したRECT）
  + scroll（物理ピクセルにスナップしたワールドスクロール量）
  + mat（ワールド変換した変換行列、物理スナップの影響も込みの値）


## ダーティ管理
これがよさげ。
https://github.com/goliajp/rust-damage-rects/blob/master/README.ja.md


## viewport管理
参考になるか？
https://github.com/forest-rs/understory/tree/main/understory_view2d






---

## 1. 全体像

論理レイアウトから物理ピクセル焼き付けまでを、責務を分離した層の直列パイプラインとして構成する。

```
Taffy               … Style → Layout（論理レイアウト、親相対座標）
   ↓
viewbox 正規化       … 座標系・スクロール・スケール・回転・クリップの器
   ↓
物理pxスナップ        … 論理→物理変換 ＋ 用途別スナップ方針
   ↓
f16 オフスクリーン正本 … インクリメンタル描画状態の真の保持者
   ↓
damage-rects         … ダーティ矩形の蓄積・合体（複数矩形＝和集合）
   ↓
包括AABB + 閾値       … 部分再描画 / 全域再描画の切替
   ↓
D2D クリップ描画       … PushAxisAlignedClip（積集合）で描画範囲を絞る
   ↓
Present1             … dirty rects（複数=和集合）で DWM 合成/転送を最小化
```

**設計の中心思想**: 各層は「自分の関心事」だけを持ち、隣接層に漏らさない。特に「レイアウト」「座標系(スクロール/変換)」「ダーティ」「合成」を明確に分離する。

---

## 2. レイヤー構成と責務分担

### 2.1 各要素が「やること / やらないこと」

| 要素 | やること | やらないこと |
|---|---|---|
| **Taffy** | Flexbox/Grid/Block の論理レイアウト（location/size） | スクロール offset、スケール、回転、変換、描画 |
| **viewbox** | 座標系、スクロール量、スケール、回転、クリップ、子viewboxのツリー | シーングラフの中身描画（visual treeに委譲） |
| **visual tree** | viewboxローカル座標での実描画要素 | スクロール・スケール・回転（すべてviewboxが担う） |
| **DXGI scroll rect** | （不採用）present段でのbitblt委譲 | f16中間面のスクロール、複数領域、zoom同時進行 |
| **Present1 dirty rects** | DWM合成/転送の最小化（複数矩形=和集合） | アプリのD2D描画スキップ（それは自前クリップの責務） |

### 2.2 マスコット層とバルーン層の正反対ポリシー

| | マスコット（キャラ層） | バルーン（UI層） |
|---|---|---|
| 本質 | 滑らかに動く・回転・拡縮する絵 | 静的矩形＋スクロールするテキスト |
| 描画矩形スナップ | **しない**（滑らかさ優先, Apple型） | 枠/テキストは要素別 |
| 変換 | サブピクセル平行移動・回転・スケール | 原点は整数スナップ |
| スクロール | 基本なし | 物理px整数スナップ必須 |
| DComp visual | 変換付き（回転/スケール） | 整数オフセット |

---

## 3. Taffy 利用方針

### 3.1 高レベルAPIの構造体

- **`TaffyTree<NodeContext>`**: ツリー本体・計算エンジン（エントリポイント）
- **`Style`**（入力, 39フィールド）: display / size / flex_* / align_* / grid_* など、CSS準拠
- **`Layout`**（出力）: `order` / `location` / `size` / `content_size` / `scrollbar_size` / border/padding/margin

### 3.2 確定した理解

- **`Layout.location` は親相対**。ワールド座標ではない。ルートから祖先を累積して絶対座標を得る。
- **`Layout.order` は Taffy が自動設定**し、**兄弟内でユニーク**（親の子リスト内インデックス）。ツリー全体でユニークではない。描画順は「親→子の再帰走査 × 兄弟内order」で確定。
- **Taffy はスクロール offset を管理しない**（overflow の"レイアウト副次効果"のみ実装。scroll offset・スクロールバー描画は非対応）。
- **Taffy は拡大縮小(scale/zoom/transform)を管理しない**（Floem等も自前実装）。
- **`round_layout` は単位非依存**（入力座標系の整数に丸めるだけ）。通常は論理px入力なので実質「論理pxスナップ」。

### 3.3 実装上の選択

- 自作 visual/viewbox ツリーに組み込むため、**Low-level API（`LayoutPartialTree` トレイト実装）** を採用。
- **Taffy の丸めは viewbox 単位で ON/OFF**:
  - 等倍(1論理=1物理)viewbox → `enable_rounding()` 可
  - スケール/回転する viewbox → `disable_rounding()`（論理px整数化が物理で無意味＆有害なため）

---

## 4. viewbox 正規化モデル

ブラウザ/Flutterコンポジタの「スクロール/クリップノード + ペイントレイヤー」分離と同型。

- **viewbox（器）**: scroll_offset(論理) + content_size、物理pxスナップ境界、clip矩形、変換(回転/スケール)、子viewboxリスト
- **visual tree（中身）**: 実描画要素、viewboxローカル座標配置（=Taffy Layout）

**不変条件**:
1. visual tree は viewbox ローカル座標のみを知る（スクロール/スケール/回転を知らない）
2. スクロールスナップは viewbox の責務（Taffyとは無関係）
3. Taffy 丸めは「1論理=1物理の viewbox」でだけ有効
4. viewbox = DComp visual 候補。ただし回転/スケールする独立層のみDComp化し、それ以外はD2D一枚合成に畳む

### 参考クレート
- **`understory_view2d`**: headless な pan/zoom/clamp/座標変換 の viewport プリミティブ。viewbox 責務とほぼ一致（新興 0.1.0, 要API安定性注意）
- **`damage-rects`**: ダーティ矩形の蓄積・合体・全画面fallback閾値
- 両者を viewbox 薄い接着層で繋ぐ3層構成が理想

---

## 5. 物理ピクセル焼き付けと丸め

### 5.1 焼き付けパイプライン

```
1. Taffy Layout（未丸め・論理・親相対 f32）※unroundedを正として保持
2. 祖先 location を累積 → ワールド論理座標（+ 各viewboxで scroll_offset 注入）
3. スケール適用（DPI × zoom）→ 物理座標(f32) ※回転visualは境界でオフスクリーン焼き
4. 物理空間で「絶対エッジ丸め」→ 整数物理ピクセル
      size = round(右端) − round(左端)
5. クリップ矩形も同じ丸めで整数化 → D2D 描画
```

### 5.2 丸め方針（確定事項）

- **位置の丸め**: 累積変換に合流可能（＝正しい実装形）。ただし**未丸め累積を正として保持**すること。丸め済みを積み上げると累積誤差でノードが1pxずつ膨張するバグ（Taffy #501相当）。
- **サイズの丸め**: 累積変換に合流**不可**。「絶対両端の丸め差」という位置依存の非線形量。同じ論理幅でも位置端数で物理サイズが±1px変わり、これが隣接エッジの継ぎ目を消す。
- **膨張(inflate)**: union・変換・丸めをすべて終えた**最終AABBに1回だけ**適用。累積適用は膨らみすぎ。外向き丸め(floor/ceil)と統合すれば実質ゼロコスト。ラスタライズ境界(オフスクリーン焼き付け)をまたぐ場合のみ、その境界の出力に対して個別に1回。

### 5.3 スナップ方針（プラットフォーム哲学）

- **Windows(DirectWrite/ClearType)**: ピクセルグリッド整列（鮮鋭優先）。`IDWritePixelSnapping` 等、スナップが一級市民。
- **macOS/iOS(Quartz/CoreText)**: 原則スナップしない（デザイン忠実性優先）。HiDPI整数2倍スケール前提で成立。
- **本設計の採用**: 用途別に分ける
  - スクロール/レイヤー変換オフセット → **スナップする**（効率＆鮮鋭）
  - マスコット描画矩形 → **スナップしない**（滑らかさ優先）
  - バルーン内の細線/枠 → スナップ、テキスト → DirectWriteに委譲
  - バルーン追従位置 → 整数スナップ（案A: 静止時シャープ最優先）

---

## 6. ダーティリージョン管理

### 6.1 ダーティ集合の定義

```
ダーティ = 露出帯（スクロールで新出現）
         ∪ 変位/発生/消失した visual の（旧AABB ∪ 新AABB）
```

- 移動/発生/消失は「旧∪新」で統一（発生=旧が空、消失=新が空を包含）
- **旧AABBには scroll_offset を合流**（scroll後の座標系でダーティ化しないと残像が残る）
- 回転/スケールする visual は**変換後の軸整列外接AABB**を使う
- AA・サブピクセル対策に**最終1回の1〜2px膨張**
- **スクロール保護領域外の静的要素は再描画不要**（Present1が自動保存）。「スクロール外すべて」は過剰カウント。

### 6.2 部分再描画 / 全域切替

```rust
let bbox = dirty_rects.iter().fold(EMPTY, |a, r| a.union(r));
if bbox.area() > viewport.area() * THRESHOLD {   // 目安 40〜50%
    redraw_full();
} else {
    clip_and_draw(bbox);
}
```

- `damage-rects::area_upper_bound()` で安価に閾値判定
- **注意**: 包括AABBは隙間を含むため、対角に離れた矩形で膨張する。**包括AABB面積 vs 実面積和**を両方見て、膨張時のみ Layer和集合 / 個別クリップに逃がす。
- **スクロール最適化が効けば全域は稀**（最大の変化=スクロールをダーティ集合から除外できるため）。

### 6.3 参考実装
- `smithay::OutputDamageTracker`（移動/出入り/transform自動、オクルージョンスキップ）— 設計参考（Wayland依存で採用は非推奨）
- WebRender `invalidation/quadtree.rs`（タイル単位のquadtree無効化）— タイル量子化の参考

---

## 7. D2D クリッピング

### 7.1 確定した挙動

- **`PushAxisAlignedClip` はネストで積集合(A∩B∩C)**。和集合・広がりは不可。
- **変換適用後の軸整列AABBに丸められる**（回転下では矩形そのものでなくAABBに膨らむ）。
- Push/Pop は厳密LIFO。Layer とオーバーラップ不可。

### 7.2 採用する構成

- **viewbox外形クリップ ∩ ダーティクリップ**は積集合で正しく表現できる。
- **ダーティを最外で1回Push**し、ツリー走査中は viewbox クリップだけ扱う（ライフサイクル一致）。
- 走査中 `intersect → is_empty ならサブツリー枝刈り`（部分再描画を最大化）。
- ダーティは**恒等変換下・物理px整数**でPush（AABB化の膨らみ・端数を回避）。

### 7.3 複数ダーティ矩形の描画（和集合が必要なとき）

| 方法 | 走査 | 正確性 | コスト |
|---|---|---|---|
| ①外接AABB1枚クリップ | 1回 | 隙間もオーバードロー | 最軽量。既定 |
| ②PushLayer + GeometryGroup | 1回 | 正確な和集合 | Layerは重い |
| ③矩形ごとにPush＆描画 | N回 | 正確 | 走査N回で非推奨 |

- 既定は**①外接AABB1枚**（`damage-rects::merged()`）。
- 露出帯とマスコットが遠く離れbboxが肥大する場合のみ**②Layer和集合**へ。
- **描画クリップと Present1 ダーティ報告は一致必須**（bbox描画ならbbox報告）。

---

## 8. スワップチェーン / 合成戦略

### 8.1 DXGI scroll rect：**不採用**

`DXGI_PRESENT_PARAMETERS { pScrollRect, pScrollOffset }` は「単一矩形を整数(dx,dy)平行移動でbitblt再利用」する仕組み。以下の癖により本用途では不採用。

| 制約 | 影響 |
|---|---|
| 単一 scroll rect のみ | 複数独立スクロール不可（本件は1領域なので可だが） |
| flip model専用・MSAA不可 | 構成制約 |
| **バッファローテーション** | 「ダーティ外は前フレーム保持」が自動保証されない。放置で1フレームおきに破綻 |
| f16中間面に届かない | scrollが効くのはpresent段のみ。f16正本の再利用には無関係 |
| zoom同時進行で無効 | 純平行移動前提 |

**却下の核心**: バッファローテーション対策でオフスクリーン正本が必須になり、その時点で「dx,dy平行移動」は正本側で自前実装済み。DXGI委譲の旨みが正本管理と二重化して相殺される。

### 8.2 採用する方式：f16 オフスクリーン正本 + Present1 dirty rects のみ

```
毎フレーム:
  1. f16 オフスクリーン正本を dx,dy（物理px整数）でスクロール（自前）
  2. 露出帯を f16 で再合成（不透明）
  3. マスコット等の動的オーバーレイを透明f16で描画
       dirty = 露出帯 ∪ マスコット(旧位置(scroll後) ∪ 新位置)
  4. 正本 → i8/10bit スワップチェーンへダーティ範囲を焼き付け（トーンマップ）
  5. Present1(dirtyRects) で DWM 合成/転送を最小化（scrollRectは使わない）
```

- **f16正本がインクリメンタル状態の真の保持者**。ローテーション問題を吸収。
- 支配的スクロール領域の平行移動は**整数px**なので精度損失ゼロ（色再計算を伴わない）。精度が要るのは露出帯・オーバーレイの合成のみ→そこだけf16。
- **Present1 dirty rects は採用**: 複数矩形=和集合をネイティブに受ける唯一の仕組み。DWM合成/リモートデスクトップ転送削減に純粋に効く。ただし「アプリのD2D描画スキップ」ではなく「DWM合成スキップ」である点に注意（描画スキップは自前クリップの責務）。
- 回転/スケールするマスコットは境界でオフスクリーンに焼いてから合成（軸整列が崩れAABBスナップ不可のため）。

---

## 9. 主要な設計判断まとめ

| 論点 | 結論 | 根拠 |
|---|---|---|
| レイアウトエンジン | Taffy（Low-level API） | 実績豊富・自作ツリーに埋め込み可 |
| スクロール/スケール/回転 | すべて viewbox が担う（Taffy外） | Taffyは論理レイアウトのみ |
| 座標系管理 | understory_view2d 相当を viewbox に | pan/zoom/clamp/変換の分離 |
| スナップ | 用途別（変換=する / マスコット描画=しない） | 滑らかさと鮮鋭の両立 |
| 丸め | 位置=累積合流可 / サイズ=合流不可 / 膨張=最終1回 | 継ぎ目防止・累積誤差防止 |
| ダーティ | 露出帯 ∪ 変位visual旧∪新、包括AABB+閾値 | 部分再描画最大化 |
| クリップ | PushAxisAlignedClip積集合、ダーティ最外Push+枝刈り | viewbox境界と描画予算の統合 |
| DXGI scroll rect | **不採用** | バッファローテーション・f16非対応・二重管理 |
| Present1 dirty rects | **採用** | 和集合ネイティブ・合成/転送削減 |
| インクリメンタル状態 | f16 オフスクリーン正本 | ローテーション吸収・精度保持 |

---

## 10. 未決事項 / 次の検討

- 包括AABB膨張検出（bbox面積 vs 実面積和の比）で Layer和集合へ切替える判定式の確定
- 回転マスコットのオフスクリーン焼き直しトリガ（角度変化量の閾値）
- f16正本のバッファ枚数とスワップチェーン焼き付けタイミングの詳細フロー
- understory_view2d のAPI安定性評価 / 必要ならfork方針
