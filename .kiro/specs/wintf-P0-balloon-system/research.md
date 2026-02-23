# リサーチ & 設計決定ログ

## サマリー
- **対象仕様**: wintf-P0-balloon-system
- **ディスカバリ範囲**: Complex Integration（既存ECSアーキテクチャへの大規模機能統合）
- **主要ファインディング**:
  1. 「1グリフ＝1エンティティ」方式はULWモードの既存描画パイプラインに変更なしで統合可能
  2. dola↔wintf統合はCargoフィーチャフラグによるオプショナル依存＋ECSリソースで実現
  3. コンテンツ領域はtaffy flexboxの標準ChildOf階層により、P1ポートレート等の後付け拡張に自然対応

---

## リサーチログ

### グリフエンティティの描画方式 (D3)

- **背景**: Req 6.2のグリフ分割後、各グリフエンティティがどのように1文字を描画するかが未決定（gap G25）
- **調査ソース**: DirectWrite API (`IDWriteTextLayout`, `IDWriteTextRenderer`, `DrawGlyphRun`)、既存typewriter実装 (`typewriter_draw.rs`)
- **調査結果**:
  - **Option A: Per-char IDWriteTextLayout** — 各グリフエンティティが1文字分の`IDWriteTextLayout`を保持し、自身のCommandListに`DrawTextLayout`で描画。位置は親GlyphContainerの共有TextLayoutから`HitTestTextPosition`で取得
  - **Option B: カスタム IDWriteTextRenderer** — 共有TextLayoutに対してカスタムレンダラを実装し、`DrawGlyphRun`コールバックを各エンティティのCommandListにルーティング。COMインターフェース実装が必要
  - **Option C: DrawGlyphRun直接呼出** — 共有TextLayoutからグリフラン情報を抽出し、per-entityで`DrawGlyphRun`を直接呼出
- **影響**:
  - Option Aは最もシンプルで、エンティティ単位の自己完結性が高い。カーニング精度は若干劣るが、バルーンテキスト（通常20〜200文字）では視覚的影響は無視可能
  - Option Bはテキストシェーピング品質が最高だが、COM `IDWriteTextRenderer` トレイト実装の複雑さが高い
  - Option Cはグリフラン境界をまたぐ文字処理が困難

### dola↔ECS統合アーキテクチャ (D4, D7)

- **背景**: wintf↔dolaはCargo.tomlレベルで未接続（gap G18）。DolaRuntimeのECSリソース化とプロパティバインディングの設計が必要
- **調査ソース**: `dola/src/runtime/facade.rs` (DolaRuntime API)、`dola/src/variable.rs` (AnimationVariableDef)、bevy_ecs Resource/System パターン
- **調査結果**:
  - `DolaRuntime` は `load_document → start → update(dt) → subscribe` の完全なライフサイクルを持つ
  - `subscribe(variable_name)` → `variable_id: i64` 取得、`update(time)` → `changes: Vec<(i64, EvaluatedValue)>` で差分配信
  - bevy_ecs の `Resource` として `DolaRuntime` をラップし、毎フレーム `update()` を呼び出す ECS システムで同期可能
  - バインディング対象は `Visual.opacity`, `Visual.is_visible`, `Arrangement.offset` の3プロパティに標準化
- **影響**:
  - wintf の Cargo.toml に `dola = { path = "../dola", optional = true }` をフィーチャフラグで追加
  - `ecs/dola_bridge/` モジュールを `#[cfg(feature = "dola")]` で条件コンパイル
  - areka クレートが `wintf = { features = ["dola"] }` で有効化

### BalloonAnchor ECS表現 (D1)

- **背景**: バルーンをキャラクターエンティティに紐付ける ECS 表現の設計
- **調査ソース**: bevy_ecs 0.18 ドキュメント、既存 `ChildOf` 関係パターン
- **調査結果**:
  - bevy_ecs 0.18 の Relation API は安定性が不明（リサーチ項目 R8）
  - 既存パターンでは `ChildOf(parent)` でエンティティ関係を表現
  - BalloonAnchor は「描画親子」ではなく「論理的な紐付け」であり、ChildOfとは意味が異なる
  - コンポーネント内に `Entity` 参照を持つ方式（`anchor: Entity`）が最もシンプルかつ安定
- **影響**: `BalloonWindow` コンポーネントに `anchor: Entity` フィールドを持たせる。Relation API が安定した段階で移行を検討

### スクロールクリッピング方式 (D6)

- **背景**: コンテンツ領域のスクロール表示にはクリッピングが必要（gap G8, G9）
- **調査ソース**: Direct2D API (`PushAxisAlignedClip`, `PushLayer`)、既存描画パイプライン
- **調査結果**:
  - `PushAxisAlignedClip` はバルーンの矩形コンテンツ領域に最適（軽量・軸平行）
  - `PushLayer` はアルファマスク等の複雑なクリッピングに対応するが、バルーンには過剰
  - オフスクリーンレンダリングはULWモードでは不要（全面再描画のため）
  - 既存 `composite_render_system` にクリッピング対応を追加する必要あり
- **影響**: `BalloonContentArea` にクリッピング矩形を持たせ、描画時に `PushAxisAlignedClip` で制限
- **補足（検証済み）**: `PushAxisAlignedClip` はD2Dの現在の変換行列（`SetTransform`）の影響を受ける。クリップ矩形はローカル座標で指定し、`composite_render_system` の `SetTransform` でスケール・移動が適用される。回転を含む場合はAABBに丸められるが、バルーンでは軸平行変換のみのため問題なし

### HitTestPoint API (G11)

- **背景**: リンクのヒットテストに座標→文字位置変換が必須。既存は逆方向（`HitTestTextPosition`: 文字位置→座標）のみラップ済み
- **調査ソース**: DirectWrite API `IDWriteTextLayout::HitTestPoint`、`com/dwrite.rs`
- **調査結果**:
  - `HitTestPoint(x, y)` → `(is_trailing_hit, is_inside, metrics)` を返す
  - `metrics.textPosition` で文字インデックスが取得可能
  - 縦書き時の精度は実装時に検証が必要（リサーチ項目 R4, R5）
- **影響（改訂）**: 1グリフ＝1エンティティ方式の採用により、**DirectWrite `HitTestPoint` APIのラップは不要**となった。各グリフエンティティが `Arrangement`（position + size）を持つため、`GlobalArrangement.bounds` による既存のエンティティレベルヒットテスト（`HitTestMode::Bounds`）でリンクの座標判定が可能。フロー: ポインタイベント → `hit_test_in_window` → グリフエンティティ特定 → `GlyphInfo.text_position` → `LinkRegion.text_range` マッチ

### コンテンツ領域の拡張性 (P1設計考慮)

- **背景**: P0設計でP1ポートレート等の非テキストインライン要素の拡張ポイントを確保する必要がある
- **調査ソース**: 既存 `ChildOf` 階層パターン、taffy flexbox、`BoxStyle` コンポーネント
- **調査結果**:
  - `BalloonContentArea` を taffy flexbox コンテナとし、`ChildOf` で子エンティティを配置
  - GlyphContainer は子エンティティの1つとして配置される（P0）
  - P1でPortraitWidget等を sibling として追加するだけで拡張可能
  - flexbox の `flex-direction` で配置方向を制御（row: 横並び、column: 縦並び）
- **影響**: 標準のECS `ChildOf` + taffy flexbox レイアウトで**ブロックレベル配置**（ポートレートをテキスト領域の横/上に配置）に対応。P0設計で意識すべきは、BalloonContentAreaがGlyphContainer以外の子も受け入れるレイアウト設計にすること
- **制約（taffy inline非対応）**: taffy 0.9.2 は `Display::Inline` を未サポート（Flex/Block/Grid/None のみ）。テキスト行内にインライン画像を埋め込むユースケースは、taffy ではなく DirectWrite の `IDWriteInlineObject` で対応する必要がある。P1ポートレートはテキスト領域と並列のブロック要素として設計されるため、taffy flexbox で十分。テキスト行内へのインライン埋め込みが将来必要になった場合は別途設計が必要

---

## アーキテクチャパターン評価

| 選択肢 | 説明 | 強み | リスク / 制約 | 備考 |
|--------|------|------|--------------|------|
| グリフ＝フルエンティティ (ULW) | 各グリフに Visual+CommandList を持たせる完全エンティティ | 既存パイプライン変更不要、dola統合が自然、per-glyph移動エフェクト | N×CreateCommandList コスト、DCompモード非推奨 | **採用** |
| グリフ＝論理エンティティ | グリフは位置+状態のみ保持、描画は親一括 | 描画コスト最小、DComp対応 | 専用描画システム必要、移動エフェクト困難 | 不採用 |
| グリフ＝データ配列 | エンティティ分割なし、Vec管理 | 最小エンティティ数 | ECSパターン不活用、独自設計 | 不採用 |

---

## 設計決定

### 決定: D3 — グリフ描画方式 (rev.1)

- **背景**: グリフエンティティが1文字を描画する具体的方式の選択
- **代替案**:
  1. Per-char IDWriteTextLayout — 各グリフが独自TextLayout保持
  2. カスタム IDWriteTextRenderer — COMインターフェース実装による描画ルーティング
- **選択**: **カスタム IDWriteTextRenderer** (Option B)
- **根拠**:
  - 既存の `RecCommandSink`（`#[implement(ID2D1CommandSink5)]`）が COM インターフェース実装の実証済みパターン。複雑さの懸念は既存実績で解消
  - 共有 TextLayout からの `DrawGlyphRun` コールバックにより、カーニング・テキストシェーピング品質を完全保持
  - per-char TextLayout 生成コスト (N×CreateTextLayout) を排除し、パフォーマンスリスク R1 を軽減
  - `GlyphDrawData` としてキャプチャした描画データを各エンティティが `dc.DrawGlyphRun()` で再生する方式
- **トレードオフ**: COM 実装コスト（ただし RecCommandSink パターンで軽減済み）/ テキスト品質の完全保持・描画コスト削減
- **初期選択からの変更理由**: 設計レビューで既存 COM 実装パターン（`RecCommandSink`）の存在が確認され、Option B の複雑さ懸念が解消。品質・性能の両面で Option A を上回る

### 決定: D7 — dola_bridge ECSリソース設計

- **背景**: DolaRuntime のライフサイクルと ECS スケジュールの統合方式
- **代替案**:
  1. ECS Resource として直接保持 — `Res<DolaRuntimeResource>` でシステムからアクセス
  2. Per-entity dola instance — 各バルーンに独自 DolaRuntime
- **選択**: **共有 ECS Resource**
- **根拠**: DolaRuntime は document 単位でロードされ、複数バルーンが同一アニメーション定義を共有可能。リソース1つで全バルーンのアニメーションを一括管理
- **トレードオフ**: グローバル状態管理の複雑さ / アニメーション定義の再利用性
- **フォローアップ**: 複数document同時ロードが必要になった場合の拡張設計

### 決定: D8 — グリフエンティティのライフサイクル

- **背景**: テキスト変更時のグリフエンティティ群の管理戦略
- **選択**: **全再構築方式**（テキスト変更時に全グリフを despawn → 新グリフを spawn）
- **根拠**: テキスト変更はレイアウト全体に影響（全グリフ位置が変動）。差分更新の利益が薄い一方、実装複雑性が高い。グリフエンティティはULWモードで軽量（ECSコンポーネント群＋CommandListのみ）
- **トレードオフ**: 変更時に全エンティティ再生成 / 実装シンプル・状態管理明確

---

## リスクと対策

- **R1: per-entity DrawGlyphRun のスループット** — 100文字時の性能が未検証。D3 rev.1（CustomTextRenderer）により per-char CreateTextLayout コストは排除済みだが、N×CommandList + N×DrawGlyphRun の再生コストは残存。対策: プロトタイプで早期検証、ダーティフラグ最適化でアクティブグリフのみ再描画
- **R4: HitTestTextPosition の縦書き精度** — 縦書き時のグリフ矩形精度が未検証。対策: balloon03-content 設計フェーズで検証テスト実施
- **G18: wintf↔dola Cargo.toml接続** — 初回のクレート間依存追加。対策: フィーチャフラグによるオプショナル依存で影響範囲を限定

---

## 正誤表

### Errata E1: on_add フック内での Commands 使用可否

- **誤**: 設計レビュー時点で「on_add フック内では Commands 使用不可（DeferredWorld のみ）」と記載
- **正**: `DeferredWorld::commands()` は bevy_ecs 0.18 で使用可能。コマンドは遅延実行される
- **既存実証**: `on_window_add` (`ecs/window/components.rs` L187-L230) が `world.commands().queue(SetWindowParentToLayoutRoot)` で子エンティティ操作を実施。`on_visual_add` も同パターン
- **影響**: BalloonWindow の子エンティティ spawn は `world.commands().queue(SpawnBalloonChildren)` で実装可能。thread_local コマンドキューは不要
- **適用**: design.md の制約事項リスト・BalloonWindow 実装ノートを修正済み

### Errata E2: エンティティ階層図の BalloonContentArea 親子関係

- **誤**: Mermaid 図で `BW --> BCA`（BalloonContentArea が BalloonWindow の直接の子）
- **正**: `BF --> BCA`（BalloonContentArea は BalloonFrame の子）。コンポーネント構成パターンの `ChildOf(balloon_frame)` と整合
- **根拠**: BalloonFrame の BoxStyle padding が ContentArea の有効領域を定義するため、BCA は BF の子として配置される
- **適用**: design.md のエンティティ階層 Mermaid 図を修正済み

---

## 参考資料

- DirectWrite `IDWriteTextLayout::HitTestTextPosition` — [Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-hittesttextposition)
- DirectWrite `IDWriteTextLayout::HitTestPoint` — [Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-hittestpoint)
- Direct2D `PushAxisAlignedClip` — [Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1rendertarget-pushaxisalignedclip)
- bevy_ecs 0.18 — [docs.rs](https://docs.rs/bevy_ecs/0.18.0)
- taffy 0.9 — [docs.rs](https://docs.rs/taffy/0.9.2)
