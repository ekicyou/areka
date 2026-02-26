# Requirements Document

| 項目               | 内容                                 |
| ------------------ | ------------------------------------ |
| **Document Title** | wintf バルーンコア 子仕様 要件定義書 |
| **Version**        | 1.0                                  |
| **Date**           | 2026-02-26                           |
| **Parent Spec**    | wintf-P0-balloon-system              |
| **Priority**       | P0 (MVP必須)                         |
| **Spec Type**      | 子仕様（Tier 1 基盤レイヤー）        |
| **Status**         | 🚧 Draft - v1.0 初版                  |

---

## Introduction

本仕様書は `wintf-P0-balloon-system`（バルーンシステム親仕様）の子仕様であり、バルーンウィンドウの**生成・フレーム描画基盤・配置制御・表示制御**を担う Tier 1 基盤レイヤーの要件を定義する。

### スコープ

親仕様の以下の要件・アーキテクチャ要件を本子仕様でカバーする:

| 親要件 | 内容                           | 描画責務 |
| ------ | ------------------------------ | -------- |
| R1     | バルーンウィンドウ生成         | —        |
| R2     | フレーム描画                   | DR-1     |
| R3     | 配置制御                       | —        |
| R4     | 表示制御                       | —        |
| AR-1   | 複合ウィジェット構造           | —        |
| AR-2   | 描画責務の分離（基盤定義）     | DR-1     |
| AR-3   | 描画責務間の独立性（基盤保証） | —        |

### 依存関係

- **前提**: `wintf-P0-event-system` ✅（完了済み）
- **後続**: 他のすべてのバルーン子仕様（balloon02〜balloon08）が本仕様に依存

### 設計制約（inherited-context.md より）

- `on_add` フック内で `DeferredWorld::commands()` は使用可能（`on_window_add` 実証済み）
- BalloonContentArea は BalloonFrame の子（`ChildOf(balloon_frame)`）、BalloonWindow の直接の子ではない
- BalloonAnchor は `anchor: Entity` フィールド方式（Relation API 不採用）
- ULW / DComp 両描画モード対応が必要

> **詳細な引継ぎコンテキスト**: [inherited-context.md](./inherited-context.md) を参照

---

## Requirements

### Requirement 1: 複合ウィジェットエンティティ階層の構築

**Objective:** 開発者として、BalloonWindow の spawn 時に複合ウィジェットとしてのエンティティ階層が自動構築されるようにしたい。それにより各描画責務エンティティが正しい親子関係で配置され、後続子仕様が安定した基盤上に実装できる。

**親要件トレース**: R1（1.5 ECS管理）、AR-1（複合ウィジェット構造）

#### Acceptance Criteria

1. **When** `BalloonWindow` コンポーネントがエンティティに追加された時, **the** Balloon Core **shall** `on_add` フックにより `BalloonFrame` 子エンティティを `ChildOf(balloon_window)` として自動 spawn する
2. **When** `BalloonWindow` コンポーネントがエンティティに追加された時, **the** Balloon Core **shall** `on_add` フックにより `BalloonContentArea` 子エンティティを `ChildOf(balloon_frame)` として自動 spawn する
3. **The** Balloon Core **shall** エンティティ階層を `BalloonWindow → BalloonFrame → BalloonContentArea` の3層構造で構築する
4. **The** Balloon Core **shall** `on_add` フック内の子エンティティ spawn を `DeferredWorld::commands()` による遅延実行で行う（`on_window_add` パターン準拠）
5. **The** Balloon Core **shall** 各子エンティティに `Visual` および `Arrangement` コンポーネントを自動挿入する

---

### Requirement 2: バルーンウィンドウ生成

**Objective:** 開発者として、キャラクターエンティティに紐付いたバルーンウィンドウを生成・管理したい。それによりキャラクターの発言表示の基盤を確立できる。

**親要件トレース**: R1（1.1〜1.5）

#### Acceptance Criteria

1. **The** Balloon Core **shall** `BalloonWindow` コンポーネントの `anchor` フィールドにより、キャラクターエンティティとの紐付けを保持する
2. **The** Balloon Core **shall** 複数のキャラクターそれぞれに独立したバルーンウィンドウを生成できる（1キャラクター：Nバルーン対応）
3. **When** `BalloonWindow` コンポーネントが追加された時, **the** Balloon Core **shall** 透過ウィンドウ（`WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`）として Win32 ウィンドウを生成する
4. **The** Balloon Core **shall** バルーンウィンドウを bevy_ecs エンティティとして管理し、HWND と Entity の双方向マッピングを既存の `Window` システムに準拠して維持する
5. **When** バルーンエンティティが despawn された時, **the** Balloon Core **shall** 関連する Win32 ウィンドウリソースおよび子エンティティを適切に解放する

---

### Requirement 3: バルーンスキン定義インターフェース

**Objective:** 開発者として、バルーンの外観パラメータ（背景・枠線・しっぽ）をスキン定義として受け取るインターフェースを持ちたい。それによりスキン実装を差し替え可能な拡張ポイントを確立できる。

**親要件トレース**: R2（2.1）、AR-2（DR-1 フレーム描画責務）、AR-3（描画責務間の独立性）

#### Acceptance Criteria

1. **The** Balloon Core **shall** `BalloonSkinDef` コンポーネントを通じて、バルーンの背景定義（単色 or 画像）を受け取るインターフェースを提供する
2. **The** Balloon Core **shall** `BalloonSkinDef` コンポーネントを通じて、枠線パラメータ（色・幅・角丸半径）を受け取るインターフェースを提供する
3. **The** Balloon Core **shall** `BalloonSkinDef` コンポーネントを通じて、しっぽ定義（方向・サイズ・オフセット）を受け取るインターフェースを提供する
4. **The** Balloon Core **shall** `BalloonSkinDef` コンポーネントを通じて、コンテンツ領域のパディング定義を受け取るインターフェースを提供する
5. **If** スキン定義のパラメータが不正な場合, **the** Balloon Core **shall** デフォルトスキン（単色白背景・角丸なし）にフォールバックする

---

### Requirement 4: フレーム描画の委譲設計

**Objective:** 開発者として、スキン定義に基づいてバルーンのフレーム描画を子ウィジットに委譲する仕組みを持ちたい。それによりバルーンの視覚表現を既存ウィジットの再利用と新規描画ウィジットの追加で柔軟に実現できる。

**親要件トレース**: R2（2.2〜2.5）

**設計方針**: balloon01-core はルートウィジット設計としてスキン定義の管理と描画委譲構造を担う。実際の描画（背景・枠線・角丸・しっぽ）は BalloonFrame の子ウィジットエンティティに委譲する。既存ウィジット（BitmapSource 等）を活用し、不足する描画ウィジットは本仕様内または孫仕様で対応する。

#### Acceptance Criteria

1. **The** Balloon Core **shall** `BalloonFrame` がスキン定義に基づき、描画を担う子ウィジットエンティティを spawn・管理する委譲構造を提供する
2. **The** Balloon Core **shall** `SkinBackground::Image` 指定時に既存の `BitmapSource` ウィジットを描画子エンティティとして活用する設計とする
3. **The** Balloon Core **shall** 背景・枠線・角丸・しっぽの描画を、子ウィジットへの委譲により実現する（具体的な描画ウィジットは本仕様内または孫仕様で対応）
4. **When** `BalloonSkinDef` コンポーネントが変更された時, **the** Balloon Core **shall** 描画子ウィジットの再構築によりフレームの再描画を実現する
5. **The** Balloon Core **shall** 委譲先の描画子ウィジットが既存の `GraphicsCommandList` パイプラインに乗る設計とし、ULW モードと DComp モードの両方で動作する

---

### Requirement 5: バルーン配置制御

**Objective:** 開発者として、バルーンをキャラクターウィンドウの近傍に自動配置し、キャラクターの移動に追従させたい。それによりどのキャラクターの発言かが視覚的に明確になる。

**親要件トレース**: R3（3.1〜3.5）

#### Acceptance Criteria

1. **The** Balloon Core **shall** `BalloonPlacement` により配置方向（Auto/Right/Left/Above/Below）を指定できる
2. **The** Balloon Core **shall** `BalloonWindow.anchor` が参照するキャラクターウィンドウの `WindowPos` に基づき、バルーンの位置を自動算出する
3. **When** キャラクターウィンドウの `WindowPos` が変更された時, **the** Balloon Core **shall** `placement_system` によりバルーンの位置を追従させる
4. **When** 算出されたバルーン位置がデスクトップ領域外に出る場合, **the** Balloon Core **shall** 配置方向を自動反転してデスクトップ内に収まるよう調整する
5. **The** Balloon Core **shall** `BalloonWindow.offset` によりキャラクターとバルーン間のオフセット距離を設定できる

---

### Requirement 6: バルーン表示制御

**Objective:** 開発者として、バルーンの表示状態（表示/非表示/サイズ）を制御したい。それにより会話の開始・終了に応じた表示管理ができる。

**親要件トレース**: R4（4.1〜4.4）

#### Acceptance Criteria

1. **The** Balloon Core **shall** バルーンの表示/非表示を `Visual.is_visible` を通じて制御できる
2. **When** バルーンが表示された時, **the** Balloon Core **shall** バルーンウィンドウをキャラクターウィンドウの前面（`WindowPos(TopMost)`）に表示する
3. **When** バルーンが非表示にされた時, **the** Balloon Core **shall** Win32 ウィンドウを非表示にしつつ ECS エンティティとその子階層は保持する
4. **The** Balloon Core **shall** バルーンウィンドウのサイズを `WindowPos` を通じて設定できる

---

### Requirement 7: エラーハンドリングと堅牢性

**Objective:** 開発者として、バルーン生成・配置時のエラーが適切にハンドリングされ、システム全体の安定性が維持されるようにしたい。それにより不正な入力や状態変化に対しても予測可能な挙動を保証できる。

**親要件トレース**: inherited-context.md エラーハンドリング戦略

#### Acceptance Criteria

1. **If** `BalloonWindow.anchor` が無効なエンティティを参照している場合, **the** Balloon Core **shall** バルーンを非表示状態にする（パニックしない）
2. **If** `BalloonWindow.anchor` が参照するエンティティに `WindowPos` が存在しない場合, **the** Balloon Core **shall** 配置計算をスキップし、前回の位置を維持する
3. **If** `BalloonSkinDef` が `BalloonFrame` に付与されていない場合, **the** Balloon Core **shall** デフォルトスキンを適用してフレームを描画する
4. **If** Win32 ウィンドウの生成に失敗した場合, **the** Balloon Core **shall** エラーを `tracing::error!` でログ出力し、エンティティをクリーンアップする

---

### Requirement 8: モジュール配置と拡張性

**Objective:** 開発者として、バルーンコアのコードが既存の wintf アーキテクチャに適合した場所に配置され、後続子仕様の拡張が容易であるようにしたい。それにより保守性の高いモジュール構造を確立できる。

**親要件トレース**: AR-2（描画責務分離）、AR-3（描画責務間独立性）

#### Acceptance Criteria

1. **The** Balloon Core **shall** `ecs/widget/balloon/mod.rs` に `BalloonWindow` コンポーネントと `on_add` フックを配置する
2. **The** Balloon Core **shall** `ecs/widget/balloon/frame.rs` に `BalloonFrame` と `BalloonSkinDef` コンポーネントを配置する
3. **The** Balloon Core **shall** `ecs/widget/balloon/placement.rs` に `placement_system` を配置する
4. **The** Balloon Core **shall** 後続子仕様（balloon02〜balloon08）が `BalloonContentArea` の `ChildOf` 階層に新規エンティティを追加できる拡張ポイントを維持する（特別な拡張機構は不要、ECS の `ChildOf` パターンで十分）
5. **The** Balloon Core **shall** 既存の wintf レイヤー構造（COM → ECS → Message Handling）の依存方向に違反しない
