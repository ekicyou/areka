# LLM参照を禁じる。このドキュメントは人間用です。LLMは参考にせず、書き込みも禁止


## emo_worldのコンポーネント構造

emo_worldの設計に整理が必要。以下の概念を中心に検討せよ。

### 要件の検討
#### 1つのemo_worldはどこまでを責務（データ保持）とするか？
1. 1ウィンドウ内の描画まで。ウィンドウ表示（位置など）は責務外。
2. 1ウィンドウ内の描画、およびウィンドウ表示を責務。
3. 全ウィンドウ（シェル）の描画を責務とする。

#### emo_worldで扱うコンポーネントは？
+ content entity
  + Content
    + command_list：描画内容
    + aabb：描画内容が占める領域

+ visual entity
  + Visual
    + transform(Transform2D)
      + offset：view_boxのスクロール位置
      + center：中心座標
      + scale：スケール
      + rot：回転
      + mat：変換行列
    + clip：option 描画クリップ領域のAABB
    + content：content entity
    + children：n個のvisual entity
  + Dirty
    + world_prev：option world_visual entity
    + world_now：option world_visual entity
    + dirty_flag：ダーティならtrue

+ world_visual entity
  + aabb
  + command_list
  + mat

+ window entity
  + Window
    + hwnd、他
  + RootVisual
    + root：ルートのvisual entity

## 仮想DOMの変更判断は lis crateを使う

## スクロール込みの描画

### 1. 今回world visualの計算
+ 全visualに対して以下の処理
  + 前回world visualを残す（転記）
  + visual⇒今回world visual（ワールド座標のvisual）を計算。
  + ダーティ判定１
    + visualのcontent（size + コマンドリスト）が変更された
  + ダーティ判定２
    + 前回と今回を比較し、D-offsetを計算
    + offset以外の行列変換に変化があった

### 2. スクロール量の決定
+ （1）でダーティになっていないvisualが判定対象
  + D-offset 毎にAABB面積を集計
  + 最大AABB面積のD-offset（a）をblit scrollとする

### 3. スクロール量の反映
+ 全world visualに処理
  + D-offsetがblit scrollと異なるworld visualをダーティにする

### 4. ダーティ領域の確定
+ 以下の領域をダーティとする
  + ダーティフラグが立ったworld visual（前回、今回の両方）
  + 削除されたvisual（前回）
  + 描画順変更があったvisual（前回・今回）
+ ダーティ前回AABBについて、実際の座標は -「blit scroll」すること
+ スクロール端AABB
+ ダーティ領域の計算
  + 全領域の加算合成（AABBではなく、矩形の和集合）

### 5. 描画
+ 領域をスクロール
+ ダーティ領域を、矩形単位で描画する
  + ダーティ矩形を透明にする
  + ダーティ矩形に今回AABBが重なるvisualを描画


## contentの管理
+ contentはvisualから参照されるentityである不変Component
+ contentはsizeとコマンドリスト（+コマンドリストが参照するD2Dリソース）を持つ

