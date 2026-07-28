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
  + visual_eq
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
