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