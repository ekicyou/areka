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
