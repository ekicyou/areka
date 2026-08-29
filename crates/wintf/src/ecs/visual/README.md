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
    Draw(Content),
    PushMatrix(Matrix),
    PopMatrix,
    PushClipRect(Content),
    PopClipRect,
    PushClipGeometry(Content),
    PopClipGeometry
}

struct DrawCommandItem{
    command: DrawCommand,
    hash: u64,
}

struct Content{
    hash: u64,
    world_aabb: Aabb,
    entity: Entity,
}

struct Matrix{
    hash: u64,
    mat: Matrix3x2,
}

struct DirtyCheckItem{
    render_hash: u64, // world_mat, world_aabb, world_clip, content,
    entity: Entity,
}

struct MatrixStack{
    
}


struct Render{
    mat_stack: Vec<>
}

```


