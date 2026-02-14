# Requirements Document

| 項目 | 内容 |
|------|------|
| **Document Title** | event-hit-test-named-regions 要件定義書 |
| **Version** | 1.0 (Draft) |
| **Date** | 2026-02-14 |
| **Parent Spec** | wintf-P1-clickthrough / event-hit-test |
| **Author** | AI-DLC System |

---

## Introduction

本仕様書は wintf フレームワークにおける名前付きヒット領域システムの要件を定義する。親仕様「event-hit-test」を拡張し、1つのエンティティ上に複数の名前付き領域を定義して、部位ごとに異なるヒット判定を実装できるようにする。

### 背景

デスクトップマスコットアプリケーションでは、キャラクターの部位ごとに異なる反応が必要である。例えば、頭を撫でると喜ぶ、手を触ると手を振る、などの細かいインタラクションを実現するため、矩形・多角形・画像マッピングなど複数の方法で名前付きヒット領域を定義できる機能が必要。

### スコープ

**含まれるもの**:
- `HitTestMode::NamedRegions` バリアントの追加
- 矩形（Rectangle）による名前付きヒット領域定義
- カラーマップ画像による名前付きヒット領域定義（色＝領域名マッピング）
- 多角形（Polygon）による名前付きヒット領域定義（オプショナル）
- ヒット結果に領域名を含む拡張API
- 外部ファイル（JSON）からのヒット領域定義読み込み
- 領域の優先順位（重なり時の処理）

**含まれないもの**:
- アニメーション連動のフレームごとヒット領域切り替え（将来仕様）
- 3Dヒットテスト
- ネットワーク経由でのヒット領域定義配信

### 前提条件

- 親仕様「event-hit-test」が実装済み（✅ 完了）
- 「event-hit-test-alpha-mask」が実装済み（✅ 完了）
- 「event-hit-test-cache」（nchittest_cache）が実装済み（✅ 完了）
- `GlobalArrangement.bounds` による物理ピクセル座標が利用可能

### 領域定義方式の概要

本仕様では以下の3つの領域定義方式をサポートする：

1. **矩形領域（Rectangle）**: 座標ベースの矩形で領域を定義。最もシンプルで軽量
2. **カラーマップ画像（ColorMap）**: 領域ごとに色分けしたPNG画像を与え、色＝領域名のマッピングで定義。複雑な形状を直感的に作成可能
3. **多角形領域（Polygon）**（オプショナル）: CSSの `polygon()` 記法に準じた頂点リストで任意形状を定義。HTMLイメージマップの `<area shape="poly">` に相当

### NamedRegions の挙動モデル

`HitTestMode::NamedRegions` は2段階の判定を行う：

1. **エンティティヒット判定**（Bounds基準）: エンティティの `GlobalArrangement.bounds` 内かを判定
2. **領域名解決**（エンティティにヒットした場合のみ）: `HitRegionMap` で定義された領域内かを判定し、領域名を返す

**重要**: 領域外ピクセル（カラーマップのマッピング外色、矩形/多角形の外側）でも、エンティティの bounds 内であれば**無名ヒット**（`region: None`）となる。これは正常な挙動であり、「エンティティにヒットしないのに領域だけがヒットする」ことはない

### 技術的調査結果

**座標系**:
- `GlobalArrangement.bounds` は物理ピクセル座標（LayoutRoot基準）
- ヒット領域定義はローカル座標系（エンティティの左上を原点、DIP単位）で行う
- スクリーン座標→ローカル座標変換は `GlobalArrangement.bounds` からの線形スケーリングで実現する（AlphaMask実装と同方式。`Matrix3x2::inverse()` は windows-numerics に不在のため、軸平行変換前提で bounds ベースの変換を使用）
- カラーマップ画像はピクセル座標で参照（画像の実サイズに基づく）

**多角形判定アルゴリズム**:
- Ray Casting法（半直線交差法）による点の内外判定が標準的
- 計算量 O(n)（n = 頂点数）で十分な性能

**CSS任意形状参考**:
- CSS `clip-path: polygon(x1 y1, x2 y2, ...)` — 頂点リストによる多角形定義
- CSS `clip-path: circle(r at cx cy)` — 円形（将来拡張候補）
- CSS `clip-path: ellipse(rx ry at cx cy)` — 楕円形（将来拡張候補）
- HTML Image Map `<area shape="poly" coords="...">` — 同様の頂点リスト方式

**カラーマップ画像方式**:
- WIC（Windows Imaging Component）でPNGを読み込み、各ピクセルの色値をルックアップ
- 色→領域名のマッピングテーブルで判定
- 「伺か」のsurfaces.txtにおける `collision` 定義（矩形）の発展形

---

## Requirements

### Requirement 1: HitTestMode の拡張

**Objective:** 開発者として、名前付きヒット領域による判定モードを設定したい。それにより1エンティティ内の部位ごとに異なるヒット判定を行える。

#### Acceptance Criteria

1. The HitTest System shall `HitTestMode` enumに `NamedRegions` バリアントを追加する
2. When `HitTestMode::NamedRegions` が設定されている時, the HitTest System shall エンティティのBounds判定を行った後、`HitRegionMap` で領域名を解決する
3. When `HitTestMode::NamedRegions` が設定されているが `HitRegionMap` コンポーネントが存在しない時, the HitTest System shall Bounds判定のみ行い、領域名なし（`region: None`）として扱う
4. The HitTest System shall 既存の `HitTestMode::None`, `HitTestMode::Bounds`, `HitTestMode::AlphaMask` の動作を維持する

---

### Requirement 2: 矩形による名前付きヒット領域定義

**Objective:** 開発者として、矩形座標で名前付きヒット領域を定義したい。それにより簡潔にキャラクターの部位を定義できる。

#### Acceptance Criteria

1. The HitTest System shall 矩形（左上座標 + 幅 + 高さ）による名前付きヒット領域定義をサポートする
2. The 矩形ヒット領域定義 shall 領域名（文字列）、x座標、y座標、幅、高さを保持する
3. The 矩形座標 shall エンティティのローカル座標系（左上原点、DIP単位）で指定する
4. When マウス座標が矩形領域内にある時, the HitTest System shall その領域の名前をヒット結果として返す
5. The HitTest System shall 1つのエンティティに対して複数の矩形ヒット領域を定義できる

---

### Requirement 3: カラーマップ画像による名前付きヒット領域定義

**Objective:** 開発者として、色分けしたPNG画像で名前付きヒット領域を定義したい。それにより複雑な形状の領域を画像編集ソフトで直感的に作成できる。

#### Acceptance Criteria

1. The HitTest System shall カラーマップ画像（PNG）による名前付きヒット領域定義をサポートする
2. The カラーマップ方式 shall 画像ファイルパスと色→領域名マッピングテーブルを保持する
3. The 色→領域名マッピング shall RGB値（アルファを無視）をキーとして領域名を返す
4. When マウス座標に対応する画像ピクセルの色がマッピングテーブルに存在する時, the HitTest System shall その色に対応する領域名をヒット結果として返す
5. When マウス座標に対応する画像ピクセルの色がマッピングテーブルに存在しない時, the HitTest System shall 無名ヒット（`region: None`）として扱う（エンティティにはヒットする）
6. The カラーマップ画像 shall エンティティの描画サイズと同じスケールで参照される（画像サイズとエンティティサイズが異なる場合はスケーリングして座標変換する）
7. The HitTest System shall カラーマップ画像をWIC（Windows Imaging Component）を使用して読み込む
8. The HitTest System shall カラーマップ画像の読み込み結果を内部にキャッシュし、フレームごとの再読み込みを回避する

---

### Requirement 4: 多角形による名前付きヒット領域定義（オプショナル）

**Objective:** 開発者として、多角形の頂点リストで名前付きヒット領域を定義したい。それにより矩形では表現できない精密な領域を定義できる。

#### Acceptance Criteria

1. Where 多角形ヒット領域機能が有効な場合, the HitTest System shall 頂点リスト（`Vec<(f32, f32)>`）による名前付き多角形ヒット領域定義をサポートする
2. The 多角形ヒット領域定義 shall 領域名（文字列）と頂点座標リストを保持する
3. The 多角形頂点座標 shall エンティティのローカル座標系（左上原点、DIP単位）で指定する
4. The 多角形 shall 3頂点以上の閉じた多角形を要求し、最初と最後の頂点は自動的に接続される
5. When マウス座標が多角形領域内にある時, the HitTest System shall Ray Casting法（半直線交差法）を使用して内外判定を行う
6. The 多角形定義 shall CSSの `polygon()` 記法に準じた形式をサポートする（例: `"polygon(0 0, 100 0, 100 100, 0 100)"`）
7. The HitTest System shall 1つのエンティティに対して矩形・多角形・カラーマップを混在して定義できる

---

### Requirement 5: ヒット結果の拡張

**Objective:** 開発者として、ヒットテスト結果に領域名を含めたい。それにより部位ごとに異なるイベント処理を実装できる。

#### Acceptance Criteria

1. The HitTest System shall ヒット結果構造体 `HitTestResult` を提供する
2. The `HitTestResult` shall 以下のフィールドを持つ：
   - `entity: Entity` — ヒットしたエンティティ
   - `region: Option<String>` — ヒットした領域名（`NamedRegions`モード時のみ、`None`は無名ヒット）
3. The HitTest System shall 既存の `hit_test` / `hit_test_in_window` APIに加えて、`HitTestResult` を返す拡張API `hit_test_ex` / `hit_test_in_window_ex` を提供する
4. When `HitTestMode::Bounds` または `HitTestMode::AlphaMask` のエンティティがヒットした時, the 拡張API shall `region: None` を含む `HitTestResult` を返す
5. When `HitTestMode::NamedRegions` のエンティティがヒットした時, the 拡張API shall `region: Some("領域名")` を含む `HitTestResult` を返す
6. The HitTest System shall 既存の `hit_test` / `hit_test_in_window` APIの動作を変更しない（後方互換）

---

### Requirement 6: ヒット領域の優先順位

**Objective:** 開発者として、重なり合うヒット領域が存在する際の判定順序を理解したい。それにより意図した領域定義を記述できる。

#### Acceptance Criteria

1. When 複数の名前付き領域が重なるポイントでヒットテストが行われた時, the HitTest System shall 定義リストを前から順に評価し、最初にヒットした領域を返す（先勝ち）
2. When カラーマップ方式を使用する時, the HitTest System shall ピクセルの色から一意に領域名が決まるため、重複判定は発生しない
3. When 矩形領域・多角形領域・カラーマップ領域が混在する時, the HitTest System shall 定義順序による先勝ちルールを適用する

---

### Requirement 7: 外部ファイルからのヒット領域定義読み込み

**Objective:** 開発者として、ヒット領域定義をJSONファイルから読み込みたい。それによりコードを変更せずに領域定義を調整できる。

#### Acceptance Criteria

1. The HitTest System shall JSON形式の外部ファイルからヒット領域定義を読み込む機能を提供する
2. The JSON定義ファイル shall 以下の構造をサポートする：
   - 領域リスト（名前、形状タイプ、座標データ）
   - カラーマップ定義（画像ファイルパス、色→領域名マッピング）
3. When 外部ファイルの読み込みに失敗した時, the HitTest System shall エラーをログ出力し、`HitTestMode::Bounds` にフォールバックする
4. The JSON形式 shall `serde` によるデシリアライズをサポートし、型安全に読み込む
5. The HitTest System shall JSON定義ファイルのパスを `HitRegionMap` コンポーネントの生成時に指定できる
6. If JSON定義ファイルに不正な形状データ（頂点数不足の多角形、負のサイズの矩形など）が含まれる時, the HitTest System shall バリデーションエラーをログ出力し、該当領域をスキップする

---

### Requirement 8: HitRegionMap コンポーネント

**Objective:** 開発者として、ECSコンポーネントとしてヒット領域データをエンティティに紐づけたい。それによりECSのクエリでヒット領域を管理できる。

#### Acceptance Criteria

1. The HitTest System shall `HitRegionMap` コンポーネントを `ecs::layout` モジュールに提供する
2. The `HitRegionMap` shall 名前付きヒット領域のリストを保持する
3. The `HitRegionMap` shall ビルダーパターンによる構築APIを提供する（例: `.rect("head", x, y, w, h).polygon("body", &vertices)`）
4. The `HitRegionMap` shall `from_json(path)` メソッドによる外部ファイルからの構築をサポートする
5. The `HitRegionMap` shall `from_color_map(image_path, mapping)` メソッドによるカラーマップ画像からの構築をサポートする
6. The `HitRegionMap` shall `hit_test_region(local_x: f32, local_y: f32) -> Option<&str>` メソッドを提供し、ローカル座標から領域名を返す
7. When `HitRegionMap` が空（領域未定義）の時, the `hit_test_region` method shall `None` を返す

