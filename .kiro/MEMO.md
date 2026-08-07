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
      + center: Vector2 // 中心座標
      + mat: Matrix3x2  // 変換行列
      + scale: Vector2  // スケール
      + rot: f32        // 回転
      + offset: Vector2 // view_boxのスクロール位置
    + clip：option 描画クリップ領域のAABB
    + content：content entity
    + children：n個のvisual entity
  + Dirty
    + world_prev：option world_visual entity
    + world_now：option world_visual entity
    + dirty_flag：ダーティならtrue



```rs:types:draw_command

pub struct DrawContent{
  local_aabb: Aabb,
  command_list: ID2D1CommandList,
}

pub enum Clip{
  Rect(D2D_RECT_F),
  Geometry(geom: ID2D1Geometry, local_aabb: D2D_RECT_F),
}

pub enum DrawCommand{
  Content(DrawContent),
  PushMatrix(Matrix3x2),
  PopMatrix(),
  PushClip(Clip),
  PopClip(),
}

```

```rs:types:numerics
pub use windows_numerics::*;
use windows::Win32::Graphics::Direct2D::Common::*;

pub const EPS: f32 = 1e-4;

// ============================================================================
// 座標変換
// ============================================================================

pub type Aabb = D2D_RECT_F;

#[derive(Clone, Copy, Debug)]
pub struct Transform2D {
    /// 追加変換（せん断など）。単位行列なら実質無効。
    pub mat: Matrix3x2,
    /// 回転・スケールの中心
    pub center: Vector2,
    /// スケール (sx, sy)
    pub scale: Vector2,
    /// 回転（ラジアン）
    pub rot: f32,
    /// スクロール位置（並進）
    pub offset: Vector2,
}

// ============================================================================
// 描画コマンド・クリップ
// ============================================================================

#[derive(Clone)]
pub struct PaintContent {
    /// build時カリング用
    pub aabb: Aabb,
    /// ID2D1Image。DrawImage で描く
    pub command_list: ID2D1CommandList,
}

#[derive(Clone)]
pub enum Clip {
    /// 矩形クリップ。
    /// replay時に current mat の軸整列性を見て Aa/Layer を決める。
    ///   rect: Aa経路（current空間そのまま）
    ///   geom: Layerフォールバック用の同一矩形 RectangleGeometry（生成時1回キャッシュ）
    Rect { rect: D2D_RECT_F, geom: ID2D1Geometry },
    /// 非矩形（角丸・星形等）。常に Layer。
    Shape { geom: ID2D1Geometry, aabb: Aabb },
}

#[derive(Clone)]
pub enum PaintCommand {
    Content(PaintContent),
    PushMatrix(Matrix3x2),
    PopMatrix,
    PushClip(Clip),
    PopClip,
}

// ============================================================================
// 描画スタック
// ============================================================================

/// mstack の1フレーム。world mat と、累積が軸整列を保存しているか（単調フラグ）。
#[derive(Clone, Copy)]
struct MatFrame {
    mat: Matrix3x2,
    exact: bool,
}

/// クリップの解除APIを振り分けるための種別。
#[derive(Clone, Copy)]
enum ClipKind {
    Aa,    // PushAxisAlignedClip → PopAxisAlignedClip
    Layer, // PushLayer → PopLayer
}

/// cstack の1エントリ。解除種別と push 前の有効クリップAABB（pop 復元用）。
#[derive(Clone, Copy)]
struct ClipFrame {
    kind: ClipKind,
    saved_clip_aabb: Aabb,
}


```

```rs:transform

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            offset: Vector2 { X: 0.0, Y: 0.0 },
            center: Vector2 { X: 0.0, Y: 0.0 },
            scale: Vector2 { X: 1.0, Y: 1.0 },
            rot: 0.0,
            mat: Matrix3x2::identity(),
        }
    }
}

impl Transform2D {
    /// 最終アフィン（ローカル → 親）を Matrix3x2 で得る。
    /// 合成順（WinUI Composition 準拠・公式）:
    ///   V = TransformMatrix · Scale · Rotation · Offset
    pub fn to_affine(&self) -> Matrix3x2 {
        let scale = Matrix3x2::scale_around(self.scale.X, self.scale.Y, self.enter);
        let rot = Matrix3x2::rotation_around(self.rot, self.center);
        let offset = Matrix3x2::translation(self.offset.X, self.offset.Y);
        self.mat * scale * rot * offset
    }
}


// ============================================================================
// Matrix3x2拡張
// ============================================================================

pub trait Matrix3x2Ext {
    fn transform_point(&self, p: Vector2) -> Vector2;
    fn transform_aabb(&self, aabb:Aabb) -> Aabb;

    /// 単純クリップできるかどうかを判定。
    /// 純スケール/並進(M12=M21=0) or 90°/270°回転(M11=M22=0) なら true。
    fn preserves_axis_alignment(&self, eps: f32) -> bool;
}

impl Matrix3x2Ext for Matrix3x2 {
    /// 点を Matrix3x2（行ベクトル規約 p' = p * M）で変換。
    #[inline]
    fn transform_point(&self, p: Vector2) -> Vector2 {
        Vector2::new(
            p.x * self.m11 + p.y * self.m21 + self.m31,
            p.x * self.m12 + p.y * self.m22 + self.m32,
        )
    }

    /// ローカルAABBの4隅を mat で変換し、
    /// その外接AABB（world/device空間）を返す。
    /// 回転が入ると膨らむが「広め＝安全側」なのでカリング判定には無害。
    fn transform_aabb(&self, aabb: &Aabb) -> Aabb {
        let pts = [
            self.transform_point(Vector2::new(aabb.left, aabb.top)),
            self.transform_point(Vector2::new(aabb.right, aabb.top)),
            self.transform_point(Vector2::new(aabb.left, aabb.bottom)),
            self.transform_point(Vector2::new(aabb.right, aabb.bottom)),
        ];
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for (x, y) in pts {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        Aabb { left: min_x, top: min_y, right: max_x, bottom: max_y }
    }

    #[inline]
    fn preserves_axis_alignment(&self, eps: f32) -> bool {
        let axis_scale = self.M12.abs() < eps && self.M21.abs() < eps;
        let axis_rot90 = self.M11.abs() < eps && self.M22.abs() < eps;
        axis_scale || axis_rot90
    }
}

```

```rs:aabb
// ============================================================================
// AABB ユーティリティ（D2D_RECT_F ベース）
// ============================================================================

pub trait AabbExt{
    /// 積集合
    fn aabb_intersect(&self, dst: &Self) -> Self;

    /// AABBが空ならtrue
    fn aabb_is_empty(&self) -> bool;
}

impl AabbExt for Aabb{
    #[inline]
    fn aabb_intersect(&self, dst: &Self) -> Self {
        Self {
            left: self.left.max(b.left),
            top: self.top.max(b.top),
            right: self.right.min(b.right),
            bottom: self.bottom.min(b.bottom),
        }
    }

    /// AABBが空ならtrue
    #[inline]
    fn aabb_is_empty(a: &Self) -> bool {
        a.right <= a.left || a.bottom <= a.top
    }
}
```

```rs:clip

impl Clip{
    #[inline]
    pub fn aabb(&self) -> Aabb {
        match self {
            Clip::Rect { rect, .. } => *rect,
            Clip::Shape { aabb, .. } => *aabb,
        }
    }
}

```

```rs:painter

// ============================================================================
// Replayer
// ============================================================================

pub struct Painter<'a> {
    dc: &'a ID2D1DeviceContext,
    mstack: Vec<MatFrame>,  /// world transform ＋ exact（PushMatrix累積）
    cstack: Vec<ClipFrame>, /// Pop振り分け用（PushClipごと1ビット）
    clip_aabb: Aabb,        /// 現在の有効クリップ（デバイス空間）
}

impl<'a> Replayer<'a> {
    /// root は最上位の world 変換（通常 identity か、DPI等の基底）。
    pub fn new(dc: &'a ID2D1DeviceContext, root_mat: Matrix3x2) -> Self {
        Self {
            dc,
            root_mat,
            mstack: Vec::new(),
            cstack: Vec::new(),
            clip_aabb: D2D_RECT_F { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 },
        }
    }

    #[inline]
    fn top(&self) -> MatFrame {
        *self.mstack.last().expect("mstack underflow")
    }

    /// コマンド列を D2D 状態機械へ流す。
    /// 判断は全て build 側で済ませてある前提で、
    /// ここは薄く保つ（唯一の判定は「矩形クリップを Aa にできるか」だけ）。
    pub fn run(&mut self, cmds: &[DrawCommand]) {
        for cmd in cmds {
            match cmd {
                // --- 変換: 相対積み上げ。exact は単調AND ---
                DrawCommand::PushMatrix(local) => {
                    let parent = self.top();
                    let world = parent.mat * *local;
                    let exact = parent.exact && local.preserves_axis_alignment(EPS);
                    self.mstack.push(MFrame { mat: world, exact });
                    unsafe { self.dc.SetTransform(&world) };
                }
                DrawCommand::PopMatrix => {
                    self.mstack.pop();
                    unsafe { self.dc.SetTransform(&self.top().mat) };
                }

                // --- クリップ: current空間のまま。Aa/Layer を決めて cstack に記録 ---
                DrawCommand::PushClip(clip) => {
                    let frame = self.push_clip(clip);
                    self.cstack.push(frame);
                }
                DrawCommand::PopClip => {
                    let frame = self.cstack.pop().expect("unbalanced PopClip");
                    self.clip_aabb = frame.saved_clip_aabb;
                    match frame.kind {
                        ClipKind::Aa => unsafe { self.dc.PopAxisAlignedClip() },
                        ClipKind::Layer => unsafe { self.dc.PopLayer() },
                        ClipKind::Culled => (),
                    }
                }

                // --- 描画: current空間で command_list を DrawImage ---
                DrawCommand::Content(c) => {
                    unsafe {
                        self.dc.DrawImage(
                            &c.command_list,
                            None,
                            None,
                            D2D1_INTERPOLATION_MODE_LINEAR,
                            D2D1_COMPOSITE_MODE_SOURCE_OVER,
                        );
                    }
                }
            }
        }
        // 終了時、スタックは初期状態に戻っているべき（Push/Pop balance 検証）
        debug_assert_eq!(self.mstack.len(), 1, "unbalanced PushMatrix/PopMatrix");
        debug_assert!(self.cstack.is_empty(), "unbalanced PushClip/PopClip");
    }

    /// クリップを push し、解除に使う ClipKind を返す。
    /// 矩形は current mat が軸整列なら Aa、そうでなければ Layer フォールバック。
    fn push_clip(&mut self, clip: &Clip) -> ClipEntry {
        let saved = self.clip_aabb;
        let region = world_aabb(&clip.local_aabb(), &self.top().mat);
        let new_clip = aabb_intersect(&self.clip_aabb, &region);

        // 完全に画面外 → 実際の push を省略（この subtree の Content は全部カリングされる）
        if aabb_is_empty(&new_clip) {
            self.clip_aabb = new_clip; // 空を伝播（配下の交差判定は必ず false）
            return ClipEntry { kind: ClipKind::Culled, saved_clip_aabb: saved };
        }

        self.clip_aabb = new_clip;

        let kind = match clip {
            Clip::Rect { rect, geom } => {
                if self.top().exact {
                    self.dc.PushAxisAlignedClip(rect, D2D1_ANTIALIAS_MODE_ALIASED);
                    ClipKind::Aa
                } else {
                    // 任意角回転/せん断 → 矩形geometryをLayerで厳密クリップ
                    self.dc.PushLayer(&layer_params(geom), None);
                    ClipKind::Layer
                }
            }
            Clip::Shape { geom, .. } => {
                self.dc.PushLayer(&layer_params(geom), None);
                ClipKind::Layer
            }
        };
        ClipEntry { kind, saved_clip_aabb: saved }
    }
}






```




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

