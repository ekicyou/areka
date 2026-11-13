# visual描画戦略

## 差分描画の基本

### 描画配列
今回・前回の描画内容をvecでキャッシュする。

#### 描画列の概念

+ 大雑把には`u64`のhash値配列として扱う。
+ lisを利用し、描画要素の追加・削除・移動を検出

#### 型の定義
型の定義はこの文書に写さない。正本はソース（`command.rs`・`builder.rs`・`clip_geometry.rs`・`../draw_tree.rs`）と `.kiro/specs/completed/wintf-visual-tree/design.md`。

#### 節にする Visual
中身の無いまとめ役の Visual は、`WorldVisual` が無いと描画面の持ち主になり、子孫を訪れない。節にするには `VisualTransform`（恒等でも）を付ける。

#### 層の旗と混ぜ方
描く中身の混ぜ方（`MixMode`）は、重ね方（`DrawImage` の 13 種）かブレンド（`ID2D1DeviceContext6::BlendImage` の 26 種・Windows 10 1903 以降。描き方が無ければ 1 度だけ `error!` を残して `SOURCE_OVER`）のどちらか 1 つ。不透明度の層は透明から始まり（`D2D1_LAYER_OPTIONS1_NONE`）、中の混ぜ方は同じ層の中で先に描いた画素に効き、降ろすとき不透明度を掛けて重なる。図形の切り抜きの層は背景の写しから始まり（`D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND`）、積む時点だけ基本の混ぜ方を `COPY` にするので、中の混ぜ方は切り抜きの形に依らず下の画素に効き、外側は変わらない。描画の木の答えは描画面ごとの入れ物（`../../composite.rs`）に合成してから WUC の描画面へ写し、入れ物の画素は `SurfaceReadback`（`../../readback.rs`）で読み戻せる。
