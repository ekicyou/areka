# Gap Analysis: wintf-dpi-aware-layout

## 概要

DPI 対応レイアウトの実装ギャップを分析し、現行コードの座標系設計と本来あるべき設計を比較する。特に `taffy_flex_demo` のレイアウトツリー全体について、現在値と修正後の期待座標値を全て算出する。

---

## 1. 現在の状態調査

### 1.1 座標系設計の現在の誤り

以下の3点が連鎖して問題を構成している：

| コンポーネント | 現在の動作 | 本来あるべき動作 |
|---|---|---|
| `update_arrangements_system` (L295-380) | `Arrangement.scale = (1.0, 1.0)` 固定（全エンティティ） | Window エンティティのみ `scale = DPI.scale` |
| `WM_WINDOWPOSCHANGED` handler (L289-332) | `BoxStyle.size = physical_px` （DPI除算なし） | `BoxStyle.size = physical_px / DPI.scale` （論理 px） |
| `WM_DPICHANGED` handler (L441-455) | `SWP_NOSIZE` 付き → サイズ変更なし | `SWP_NOSIZE` 除去 → `suggested_rect` のサイズを適用 |

### 1.2 影響ファイルの特定

| ファイル | 関数/箇所 | 変更種類 |
|---|---|---|
| `crates/wintf/src/ecs/layout/systems.rs` L295-380 | `update_arrangements_system` | **修正**: Window の scale を DPI から設定 |
| `crates/wintf/src/ecs/window_proc/handlers.rs` L289-332 | `WM_WINDOWPOSCHANGED` handler | **修正**: BoxStyle.size を論理 px で設定 |
| `crates/wintf/src/ecs/window_proc/handlers.rs` L441-455 | `WM_DPICHANGED` handler | **修正**: `SWP_NOSIZE` 除去 |
| `crates/wintf/examples/taffy_flex_demo.rs` L310-320 | Window BoxStyle | **修正**: サイズ縮小 + 子要素調整 |
| `crates/wintf/examples/taffy_flex_demo.rs` L260-295 | `run_demo` 待機時間 | **修正**: 60秒→1秒 |
| `crates/wintf/examples/taffy_flex_demo.rs` L943-1082 | `dump_all_windows_dpi` | **拡張**: INFO ログ出力追加 |

---

## 2. 設計上の座標系境界

修正後のシステムは以下の座標系境界を持つ：

```
LayoutRoot (物理 px 座標系, scale=1.0)
  ├─ Monitor エンティティ群 (物理 px)
  └─ Window (← ここが座標系境界)
       ├─ BoxStyle.size = 論理 px
       ├─ Arrangement.scale = DPI.scale（論理→物理変換）
       ├─ Arrangement.offset = 物理 px（WindowPos から直接設定）
       │
       └─ 子要素 (論理 px 座標系, scale=1.0)
            ├─ BoxStyle.size = 論理 px
            ├─ Arrangement.scale = (1.0, 1.0)
            └─ GA.bounds = 物理 px（親のDPI scaleが伝播）
```

**変換の数学的正当性**:

```
Window.GA.transform = LayoutRoot.GA.transform × (translation(phys_pos) × scale(DPI))
                    = identity × (translation(phys_pos) × scale(DPI))
                    = { M11: DPI_sx, M22: DPI_sy, M31: phys_x, M32: phys_y }

Child.GA.bounds.width = Child.Arrangement.size.width × Window.GA.transform.M11
                      = logical_px × DPI_scale
                      = physical_px  ✓

Child.GA.bounds.left  = Window.GA.bounds.left + Child.offset.x × DPI_scale
                      = phys_window_left + logical_offset × DPI_scale
                      = physical_px  ✓
```

---

## 3. `taffy_flex_demo` のレイアウトツリー構造

### 3.1 エンティティ階層

```
Window (Column, 600×420 論理 px) [REQ-5で縮小]
  ├─ FlexContainer (Row, SpaceEvenly, Center, h=160, grow=0, shrink=0, margin=10all)
  │    ├─ RedBox (200×100, grow=0, shrink=0, basis=200)
  │    │    └─ SeikatuImage (64×64, margin: L=68, T=18, B=18, R=auto)
  │    ├─ GreenBox (Column, 100×100, grow=1, shrink=1, basis=auto)
  │    │    └─ GreenBoxChild (50×50)
  │    └─ BlueBox (100×100, grow=2, shrink=1, basis=auto)
  ├─ RegionTest-Container (Row, SpaceEvenly, Center, grow=1, margin: L=10,R=10,T=0,B=10)
  │    ├─ RegionRectBox (140×150 → 要縮小)
  │    ├─ RegionPolygonBox (140×150 → 要縮小)
  │    ├─ RegionMixedBox (140×150 → 要縮小)
  │    ├─ RegionColorMapBox (140×150 → 要縮小)
  │    └─ RegionFallbackBox (140×150 → 要縮小)
  └─ ClickThrough-Container (Row, SpaceEvenly, Center, h=120, margin: L=10,R=10,T=0,B=10)
       ├─ ClickThroughBox (150×100, HitTest::none)
       ├─ NormalHitBox (150×100, HitTest::bounds)
       └─ AlphaBoundaryBox (150×100, HitTest::bounds, opacity=0.5)
```

### 3.2 Taffy レイアウト計算（修正後 Window 600×420 論理 px）

#### 3.2.1 Window 直下の Column レイアウト

使用可能サイズ: 600×420 論理 px

| 子 | margin (T/B) | 固定高さ | grow | shrink |
|---|---|---|---|---|
| FlexContainer | 10 / 10 | 160 | 0 | 0 |
| RegionTest | 0 / 10 | auto | 1 | - |
| ClickThrough | 0 / 10 | 120 | 0 | 1 |

**計算**:
- 全margin合計 (主軸=縦): (10+10) + (0+10) + (0+10) = 40
- 固定項目の高さ: 160 + 120 = 280
- 残余空間: 420 − 40 − 280 = 100
- RegionTest: grow=1 → 高さ **100 論理 px**

| 子 | Taffy location (x, y) | Taffy size (w × h) |
|---|---|---|
| FlexContainer | (10, 10) | 580 × 160 |
| RegionTest | (10, 180) | 580 × 100 |
| ClickThrough | (10, 290) | 580 × 120 |
| **合計** | | 10+160+10+100+10+120+10 = **420** ✓ |

#### 3.2.2 FlexContainer 内の Row レイアウト (580×160)

| 子 | basis | grow | shrink | 計算後幅 |
|---|---|---|---|---|
| RedBox | 200 | 0 | 0 | **200** |
| GreenBox | auto→100 | 1 | 1 | 100 + 180×(1/3) = **160** |
| BlueBox | auto→100 | 2 | 1 | 100 + 180×(2/3) = **220** |
| **合計** | 400 | | | **580** ✓ |

余白 = 580 − 400 = 180 → grow で吸収 → SpaceEvenly は実質無効

cross-axis (align-items=Center, container h=160): 各アイテム h=100、y = (160−100)/2 = **30**

| 子 | Taffy location (x, y) | Taffy size (w × h) |
|---|---|---|
| RedBox | (0, 30) | 200 × 100 |
| GreenBox | (200, 30) | 160 × 100 |
| BlueBox | (360, 30) | 220 × 100 |

#### 3.2.3 RedBox 内 (Row default, 200×100)

| 子 | margin | Taffy location (x, y) | Taffy size (w × h) |
|---|---|---|---|
| SeikatuImage | L=68, R=auto, T=18, B=18 | (68, 18) | 64 × 64 |

#### 3.2.4 GreenBox 内 (Column, 160×100)

| 子 | Taffy location (x, y) | Taffy size (w × h) |
|---|---|---|
| GreenBoxChild | (0, 0) | 50 × 50 |

#### 3.2.5 RegionTest-Container (Row, SpaceEvenly, Center, 580×100)

**⚠ REQ-5 問題点: 現行の子要素サイズでは収まらない**

現行サイズ 140×150 の場合:
- 主軸: 5×140=700 > 580 → flex-shrink で各アイテム (700−580)/5=24 縮小 → **116×150**
- 交差軸: 150 > 100 → Center: y=(100−150)/2=−25 → **上下25pxオーバーフロー**

**→ REQ-5 で子要素を比例縮小する必要あり**

**推奨サイズ: 100×90 論理 px**（縮小率: 幅 0.71×、高さ 0.60×）

100×90 の場合:
- 主軸: 5×100=500 < 580 → SpaceEvenly: gap = (580−500)/6 ≈ 13.3
- 交差軸: 90 < 100 → Center: y=(100−90)/2=5

| 子 | Taffy location (x, y) | Taffy size (w × h) |
|---|---|---|
| RegionRectBox | (13.3, 5) | 100 × 90 |
| RegionPolygonBox | (126.7, 5) | 100 × 90 |
| RegionMixedBox | (240.0, 5) | 100 × 90 |
| RegionColorMapBox | (353.3, 5) | 100 × 90 |
| RegionFallbackBox | (466.7, 5) | 100 × 90 |

**⚠ HitRegionMap 座標も縮小率に合わせて再定義が必要**（後述）

#### 3.2.6 ClickThrough-Container (Row, SpaceEvenly, Center, 580×120)

- 主軸: 3×150=450 < 580 → SpaceEvenly: gap = (580−450)/4 = 32.5
- 交差軸: 100 < 120 → Center: y=(120−100)/2=10

| 子 | Taffy location (x, y) | Taffy size (w × h) |
|---|---|---|
| ClickThroughBox | (32.5, 10) | 150 × 100 |
| NormalHitBox | (215.0, 10) | 150 × 100 |
| AlphaBoundaryBox | (397.5, 10) | 150 × 100 |

---

## 4. 全エンティティの期待座標値（修正後）

### 4.1 環境前提

| モニター | DPI | scale | 物理サイズ | 論理サイズ |
|---|---|---|---|---|
| 右 (DISPLAY2, プライマリ, 4K) | 120 (125%) | 1.25 | 3840×2160 | 3072×1728 |
| 左 (DISPLAY1, 非プライマリ) | 192 (200%) | 2.00 | 2880×1800 | 1440×900 |

### 4.2 Window 1 (125% DPI, 右モニター, 物理pos=(100,100))

全ての Arrangement/GA 値を列挙する。

#### Window 1 ルート

| 項目 | 値 |
|---|---|
| BoxStyle.size | 600 × 420 論理 px |
| Arrangement.offset | (100, 100) 物理 px |
| Arrangement.scale | **(1.25, 1.25)** |
| Arrangement.size | (600, 420) |
| GA.transform | M11=1.25, M22=1.25, M31=100, M32=100 |
| GA.bounds | left=100, top=100, right=100+600×1.25=**850**, bottom=100+420×1.25=**625** |
| GA.bounds size | **750 × 525 物理 px** |

#### Window 1 → FlexContainer

| 項目 | 値 |
|---|---|
| BoxStyle.size | auto × 160 (Taffy: 580 × 160) |
| Arrangement.offset | (10, 10) 論理 px |
| Arrangement.scale | (1.0, 1.0) |
| Arrangement.size | (580, 160) |
| GA.transform.M11 | 1.25 (parent scale 伝播) |
| GA.bounds.left | 100 + 10×1.25 = **112.5** |
| GA.bounds.top | 100 + 10×1.25 = **112.5** |
| GA.bounds size | 580×1.25 × 160×1.25 = **725 × 200** 物理 px |

#### Window 1 → FlexContainer → RedBox

| 項目 | 値 |
|---|---|
| BoxStyle.size | 200 × 100 論理 px |
| Arrangement.offset | (0, 30) |
| Arrangement.size | (200, 100) |
| GA.bounds.left | 112.5 + 0×1.25 = **112.5** |
| GA.bounds.top | 112.5 + 30×1.25 = **150.0** |
| GA.bounds size | **250 × 125** 物理 px |

#### Window 1 → FlexContainer → RedBox → SeikatuImage

| 項目 | 値 |
|---|---|
| BoxStyle.size | 64 × 64 |
| Arrangement.offset | (68, 18) |
| GA.bounds.left | 112.5 + 68×1.25 = **197.5** |
| GA.bounds.top | 150.0 + 18×1.25 = **172.5** |
| GA.bounds size | **80 × 80** 物理 px |

#### Window 1 → FlexContainer → GreenBox

| 項目 | 値 |
|---|---|
| BoxStyle.size | 100 × 100 (Taffy展開後: 160 × 100) |
| Arrangement.offset | (200, 30) |
| Arrangement.size | (160, 100) |
| GA.bounds.left | 112.5 + 200×1.25 = **362.5** |
| GA.bounds.top | 112.5 + 30×1.25 = **150.0** |
| GA.bounds size | **200 × 125** 物理 px |

#### Window 1 → FlexContainer → GreenBox → GreenBoxChild

| 項目 | 値 |
|---|---|
| BoxStyle.size | 50 × 50 |
| Arrangement.offset | (0, 0) |
| GA.bounds.left | 362.5 + 0 = **362.5** |
| GA.bounds.top | 150.0 + 0 = **150.0** |
| GA.bounds size | **62.5 × 62.5** 物理 px |

#### Window 1 → FlexContainer → BlueBox

| 項目 | 値 |
|---|---|
| BoxStyle.size | 100 × 100 (Taffy展開後: 220 × 100) |
| Arrangement.offset | (360, 30) |
| Arrangement.size | (220, 100) |
| GA.bounds.left | 112.5 + 360×1.25 = **562.5** |
| GA.bounds.top | 150.0 |
| GA.bounds size | **275 × 125** 物理 px |

#### Window 1 → RegionTest-Container

| 項目 | 値 |
|---|---|
| Arrangement.offset | (10, 180) |
| Arrangement.size | (580, 100) |
| GA.bounds.left | 100 + 10×1.25 = **112.5** |
| GA.bounds.top | 100 + 180×1.25 = **325.0** |
| GA.bounds size | **725 × 125** 物理 px |

#### Window 1 → ClickThrough-Container

| 項目 | 値 |
|---|---|
| Arrangement.offset | (10, 290) |
| Arrangement.size | (580, 120) |
| GA.bounds.left | 100 + 10×1.25 = **112.5** |
| GA.bounds.top | 100 + 290×1.25 = **462.5** |
| GA.bounds size | **725 × 150** 物理 px |

---

### 4.3 Window 2 (200% DPI, 左モニター)

左モニター bounds を (left_phys, top_phys, 1440wide, 900high) とする。
`find_non_primary_monitor_origin` は (left+w/10, top+h/10) = (left+144, top+90) を返す。

#### ⚠ 200%モニターでのオーバーフロー検証

| 項目 | 値 |
|---|---|
| ウィンドウ物理サイズ | 600×2.0 = 1200, 420×2.0 = 840 |
| 配置位置Y | top + 90 |
| 下端 | top + 90 + 840 = top + **930** |
| モニター下端 | top + **900** |
| **オーバーフロー** | **30 物理 px はみ出し** |

**対策案**:
- `find_non_primary_monitor_origin` の Y マージンを小さくする（10%→3%程度）
- または Window 高さを **400 論理 px** に縮小（400×2=800, 90+800=890 < 900 ✓）
- または 位置特定ロジックをウィンドウサイズ考慮に変更

**推奨: Window サイズを 600×400 論理 px に変更**  
→ 200%モニター: 1200×800 物理px、配置 Y=90 → 890 < 900 ✓  
→ 125%モニター: 750×500 物理px（いずれにせよ収まる）

#### Window 2 ルート（600×400 論理 px 推奨の場合）

600×400 の場合、RegionTest 高さは:
- 使用可能: 400 − 40 − 280 = **80** 論理 px

| 項目 | 値（600×400 の場合）| 値（600×420 の場合）|
|---|---|---|
| BoxStyle.size | 600 × 400 | 600 × 420 |
| Arrangement.scale | (2.0, 2.0) | (2.0, 2.0) |
| GA.bounds size | **1200 × 800** | **1200 × 840** |

以下は **600×420 論理 px** での計算（REQ要件の上限値、位置調整で対応想定）:

#### Window 2 の Taffy レイアウト

Taffy の計算結果は Window 1 と**完全に同一**（同じ BoxStyle → 同じ論理 px レイアウト）。

#### Window 2 の全 GA 座標値（DPI scale=2.0）

LayoutRoot child base = (W2x, W2y) として:

| Entity | Arr.offset (論理) | GA.bounds size (物理) | GA.bounds.left | GA.bounds.top |
|---|---|---|---|---|
| Window | (W2x, W2y) phys | **1200 × 840** | W2x | W2y |
| FlexContainer | (10, 10) | **1160 × 320** | W2x + 20 | W2y + 20 |
| RedBox | (0, 30) | **400 × 200** | W2x + 20 | W2y + 20 + 60 |
| SeikatuImage | (68, 18) | **128 × 128** | W2x + 20 + 136 | W2y + 80 + 36 |
| GreenBox | (200, 30) | **320 × 200** | W2x + 20 + 400 | W2y + 80 |
| GreenBoxChild | (0, 0) | **100 × 100** | (same as GreenBox) | (same) |
| BlueBox | (360, 30) | **440 × 200** | W2x + 20 + 720 | W2y + 80 |
| RegionTest | (10, 180) | **1160 × 200** | W2x + 20 | W2y + 360 |
| ClickThrough | (10, 290) | **1160 × 240** | W2x + 20 | W2y + 580 |

### 4.4 要件確認基準との照合

| 確認項目 | Window 1 (125%) | Window 2 (200%) | 判定 |
|---|---|---|---|
| GA.bounds 幅 = BoxStyle.width × DPI_scale | 600×1.25=**750** | 600×2.0=**1200** | ✅ |
| GA.bounds 高 = BoxStyle.height × DPI_scale | 420×1.25=**525** | 420×2.0=**840** | ✅ |
| RedBox GA幅 = 200 × DPI_scale | **250** | **400** | ✅ |
| RedBox GA高 = 100 × DPI_scale | **125** | **200** | ✅ |
| 両 Window の論理 px サイズ | 600×420 | 600×420 | ✅ 同一 |
| 両 Window の物理 px サイズ | 750×525 | 1200×840 | ✅ DPI比例 |

---

## 5. 現在のコード vs 修正後コードの差分マップ

### 5.1 `update_arrangements_system` (systems.rs L295-380)

**現在**:
```rust
// L321: 常に (1.0, 1.0)
let scale = LayoutScale::default();
```

**修正後**:
```rust
let scale = if window.is_some() {
    if let Some(ref d) = dpi {
        LayoutScale { x: d.scale_x(), y: d.scale_y() }
    } else {
        LayoutScale::default()
    }
} else {
    LayoutScale::default()
};
```

**影響**: Window エンティティのみ scale が変わる。子エンティティは (1.0, 1.0) のまま。

### 5.2 `WM_WINDOWPOSCHANGED` handler (handlers.rs L289-332)

**現在**:
```rust
// L298-299: 物理 px をそのまま設定
let physical_width = client_size.cx as f32;
let physical_height = client_size.cy as f32;
let new_size = Some(BoxSize {
    width: Some(Dimension::Px(physical_width)),
    height: Some(Dimension::Px(physical_height)),
});
```

**修正後**:
```rust
let physical_width = client_size.cx as f32;
let physical_height = client_size.cy as f32;
// DPI で除算して論理 px に変換
let logical_width = physical_width / dpi.scale_x();
let logical_height = physical_height / dpi.scale_y();
let new_size = Some(BoxSize {
    width: Some(Dimension::Px(logical_width)),
    height: Some(Dimension::Px(logical_height)),
});
```

**注意**: `dpi` 変数は L153-170 で既に取得済み（`DpiChangeContext` 考慮あり）。

### 5.3 `WM_DPICHANGED` handler (handlers.rs L441-455)

**現在**:
```rust
let result = unsafe {
    crate::ecs::window::guarded_set_window_pos(
        hwnd, None,
        suggested_rect.left, suggested_rect.top,
        0, 0,  // SWP_NOSIZE で無視
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
};
```

**修正後**:
```rust
let width = suggested_rect.right - suggested_rect.left;
let height = suggested_rect.bottom - suggested_rect.top;
let result = unsafe {
    crate::ecs::window::guarded_set_window_pos(
        hwnd, None,
        suggested_rect.left, suggested_rect.top,
        width, height,  // suggested_rect のサイズを使用
        SWP_NOZORDER | SWP_NOACTIVATE,  // SWP_NOSIZE 除去
    )
};
```

### 5.4 `sync_window_arrangement_from_window_pos` (systems.rs L494-535)

**変更不要**。WindowPos.position は物理 px のまま、LayoutRoot (scale=1.0) の子なので offset = 物理 px で正しい。

### 5.5 `taffy_flex_demo.rs` の変更

#### 5.5.1 Window BoxStyle

**現在**: `size: 800×700`  
**修正後**: `size: 600×420` （または 600×400 推奨）

#### 5.5.2 RegionTest 子要素サイズ（要縮小）

**現在**: 各 140×150  
**修正後 (推奨)**: 各 100×90

影響する HitRegionMap 座標:

| リージョン | 現在の座標 | 修正後の座標 (100×90) |
|---|---|---|
| rect("top-left") | (0, 0, 70, 75) | (0, 0, 50, 45) |
| rect("top-right") | (70, 0, 70, 75) | (50, 0, 50, 45) |
| rect("bottom-left") | (0, 75, 70, 75) | (0, 45, 50, 45) |
| rect("bottom-right") | (70, 75, 70, 75) | (50, 45, 50, 45) |
| polygon 各頂点 | 0/70/140/75/150 | 0/50/100/45/90 |
| colormap | 64×64 PNG → 140×150 マッピング | 64×64 PNG → 100×90 マッピング |

#### 5.5.3 run_demo 待機時間

**現在**: 2s + 3s + 55s = 60s  
**修正後**: 2s + dump + 1s 後に close（REQ-6）

**注意**: レイアウト安定のための2秒ウェイトは維持し、その後のダンプ→1秒後に終了。

### 5.6 `dump_all_windows_dpi` の拡張（REQ-7）

**現在**: `println!` で標準出力  
**修正後**: `info!` マクロで `Arrangement.scale`, `GA.bounds`, `BoxStyle.size` をログ出力

---

## 6. 座標ラウンドトリップ検証

DPI 変更シナリオ（125% → 200%）の数学的検証:

```
[初期状態: 125% DPI]
BoxStyle.size = 600×420 logical
Arrangement.scale = (1.25, 1.25)
GA.bounds size = 750×525 physical
Win32 Window size = 750×525

[WM_DPICHANGED: new DPI=192 (2.0×)]
suggested_rect size = 600×2.0 × 420×2.0 = 1200×840 physical
SetWindowPos(1200, 840)  // SWP_NOSIZE 除去

[WM_WINDOWPOSCHANGED: physical=1200×840, DPI=192]
BoxStyle.size = 1200/2.0 × 840/2.0 = 600×420 logical  ← 元の論理サイズ維持 ✓
DPI component → 192
Arrangement.scale → (2.0, 2.0)
GA.bounds size = 600×2.0 × 420×2.0 = 1200×840 physical ✓

[逆方向: 200% → 125%]
suggested_rect size = 600×1.25 × 420×1.25 = 750×525 physical
SetWindowPos(750, 525)
BoxStyle.size = 750/1.25 × 525/1.25 = 600×420 logical ✓
```

**結論**: ラウンドトリップで論理サイズ 600×420 は完全に保存される。

---

## 7. 既知の制約と注意点

### 7.1 LayoutRoot 座標系の不整合

LayoutRoot は物理 px 座標系（`GetSystemMetrics` が物理 px を返すため）のまま。Window は論理 px。Taffy 計算では Window の `position: Absolute` により、LayoutRoot のサイズに制約されないため問題は発生しない。ただし、この「座標系の混在」は設計上のトレードオフとして認識すべき。

### 7.2 sync_window_arrangement_from_window_pos の offset 単位

Window.Arrangement.offset は**物理 px**（WindowPos からそのまま）。一方 Window.Arrangement.size は**論理 px**（Taffy から）。同一の Arrangement 構造体内で単位が混在する。

`GA = LayoutRoot.GA * Window.Arrangement` の計算では:
- LayoutRoot.scale = (1.0, 1.0) → `scaled_offset = offset × 1.0 = 物理 px`
- `bounds.right = bounds.left + size × result_scale = bounds.left + logical × DPI_scale = 物理 px`

数学的に正しいが、Arrangement のセマンティクスが統一されていない点は要注意。

### 7.3 Echo 判定ロジック

`WM_WINDOWPOSCHANGED` の echo/bypass ロジック (L207-215) は `DpiChangeContext` がある場合 bypass しない設計。修正後の `SWP_NOSIZE` 除去により `WM_WINDOWPOSCHANGED` で受け取るサイズが変わるが、既存の echo 判定ロジックとの整合性を確認すること。

### 7.4 ColorMap HitRegionMap のリスケール

`demo_region_colormap_64x64.png` は 64×64 ピクセルの画像。現行コードではこれを 140×150 の Box に適用している。Box が 100×90 に縮小されても、HitRegionMap の内部座標変換（Box のローカル座標 → 画像ピクセル座標）が自動的にスケーリングされるかを確認する必要あり。
→ **Research Needed**: `HitRegionMap::from_color_map` の座標マッピングロジック

---

## 8. 実装アプローチ

### Option A: 既存コンポーネント修正（推奨）

3つのシステム関数の修正と1つのデモファイル調整のみ。新規ファイル追加なし。

| 変更対象 | 変更量 | リスク |
|---|---|---|
| `update_arrangements_system` | ~10行 | 低: Window 判定は既存 |
| `WM_WINDOWPOSCHANGED` handler | ~5行 | 中: echo bypass との相互作用 |
| `WM_DPICHANGED` handler | ~5行 | 中: SWP_NOSIZE 除去の副作用 |
| `taffy_flex_demo.rs` | ~50行 | 低: サイズ値変更のみ |
| `dump_all_windows_dpi` | ~20行 | 低: ログ追加のみ |

**Trade-offs**:
- ✅ 最小変更量、既存設計パターン踏襲
- ✅ LayoutRoot 以下の物理 px 系は維持（既存テスト影響なし）
- ❌ Arrangement 内の単位混在は解消されない

### 工数・リスク

- **工数**: **S**（1-2日）— 変更箇所が特定済み、数学的検証完了
- **リスク**: **Medium** — DPI 変更時の echo/bypass 相互作用、Window 移動時の無限ループ回避ロジックとの整合性確認が必要

---

## 9. 設計フェーズへの引き継ぎ事項

### 決定済み事項
1. 座標系境界は Window エンティティ（論理/物理の変換点）
2. `Arrangement.scale` を DPI に設定するのは Window のみ
3. 全ての数学的変換は検証済み（ラウンドトリップ含む）
4. 期待座標値は全エンティティで算出済み

### Research Needed
1. **HitRegionMap colormap スケーリング**: 100×90 box での正常動作確認
2. **Echo/bypass の副作用**: `SWP_NOSIZE` 除去後の `WM_WINDOWPOSCHANGED` 発火パターン
3. **Window サイズ上限**: 600×420 vs 600×400（200%モニター配置位置依存）

### 推奨次ステップ
- `/kiro-spec-design wintf-dpi-aware-layout` で詳細設計へ進む
- 実装はこの gap analysis の差分マップ（セクション5）をそのままタスク化可能
