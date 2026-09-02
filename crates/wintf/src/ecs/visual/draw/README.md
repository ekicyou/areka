# visual描画戦略

## 差分描画の基本

### 描画配列
今回・前回の描画内容をvecでキャッシュする。

#### 描画列の概念

+ 大雑把には`u64`のhash値配列として扱う。
+ lisを利用し、描画要素の追加・削除・移動を検出

#### 宣言
```rust
enum DrawCommand{
    Draw(DrawItem),
    PushClipRect(ClipRect),
    PopClipRect,
    PushClipGeometryEntity(ClipGeometryEntity),
    PushClipGeometryRect(ClipGeometryRect),
    PopClipGeometry,
}

struct DrawItem{
    hash: u64,
    world_mat: Matrix3x2,
    entity: Entity,
    world_aabb: Aabb,
}

struct ClipRect{
    world_aabb: Aabb,
}

struct ClipGeometryEntity{
    world_mat: Matrix3x2,
    entity: Entity,
    world_aabb: Aabb,
}

struct ClipGeometryRect{
    world_mat: Matrix3x2,
    geometry: ID2D1Geometry,
    world_aabb: Aabb,
}


```
