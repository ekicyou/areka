# Requirements Document

| 項目               | 内容                                    |
| ------------------ | --------------------------------------- |
| **Document Title** | wintf バルーンコア 子仕様 要件定義書    |
| **Version**        | 2.0                                     |
| **Date**           | 2026-02-26                              |
| **Parent Spec**    | wintf-P0-balloon-system                 |
| **Priority**       | P0 (MVP必須)                            |
| **Spec Type**      | 子仕様（Tier 1 基盤レイヤー）           |
| **Status**         | 🚧 Draft - v2.0 責務分離 + v3.0 議事待ち |

---

## Introduction

本仕様書は `wintf-P0-balloon-system`（バルーンシステム親仕様）の子仕様であり、バルーン**描画ウィジェット**の**エンティティ階層構築・フレーム描画基盤・表示制御**を担う Tier 1 基盤レイヤーの要件を定義する。

> **設計原則**: バルーンはあくまで**描画コンポーネント（ウィジェット）**であり、ウィンドウ管理や配置制御の責務を持たない。バルーンをどのウィンドウに配置するか、どう動かすかは外部システムの責務である。キャラクターとバルーンは同一ウィンドウに配置される可能性も、別ウィンドウに配置される可能性もあり、その協調制御は外部が行う。

### スコープ

親仕様の以下の要件・アーキテクチャ要件を本子仕様でカバーする:

| 親要件 | 内容                           | 描画責務 | 本仕様での対応                           |
| ------ | ------------------------------ | -------- | ---------------------------------------- |
| R1     | バルーンウィジェット生成       | —        | Req 1, 2                                 |
| R2     | フレーム描画                   | DR-1     | Req 3, 4                                 |
| ~~R3~~ | ~~配置制御~~                   | —        | **スコープ外**: 外部システムの責務に移管 |
| R4     | 表示制御                       | —        | Req 5                                    |
| AR-1   | 複合ウィジェット構造           | —        | Req 1                                    |
| AR-2   | 描画責務の分離（基盤定義）     | DR-1     | Req 4                                    |
| AR-3   | 描画責務間の独立性（基盤保証） | —        | Req 7                                    |

> **親仕様 R3（配置制御）について**: 本子仕様では、バルーンの配置・追従・デスクトップ境界反転をスコープ外とする。これらはバルーンウィジェットの描画責務ではなく、外部の協調制御システムの責務である。親仕様 R3 は別途見直しが必要。

### 依存関係

- **前提**: `wintf-P0-event-system` ✅（完了済み）
- **ブロッカー**: `wintf-P0-cue-system`（演出キュー配送基盤）—— コンテンツコマンド配送メカニズムの設計がバルーンのコンポーネント構成原則に影響するため、v3.0 要件最終化の前提
- **後続**: 他のすべてのバルーン子仕様（balloon02〜 balloon08）が本仕様に依存

### 設計制約（inherited-context.md より）

- `on_add` フック内で `DeferredWorld::commands()` は使用可能（`on_window_add` 実証済み）
- BalloonContentArea は BalloonFrame の子（`ChildOf(balloon_frame)`）、Balloon の直接の子ではない
- ULW / DComp 両描画モード対応が必要
- Balloon はルートエンティティであり Visual コンポーネントを持つ。自前のサーフェスは透明で、コマンドリストは作らず、実際の描画は子エンティティに委譲する
- Balloon は純粋な描画ウィジェットであり、ウィンドウ管理・配置制御・キャラクターとの紐づけの責務を持たない
- dola はバルーン内部サブシステムとしても利用可能（タイミング計算、グリフ表示制御等）

> **詳細な引継ぎコンテキスト**: [inherited-context.md](./inherited-context.md) を参照

---

## v3.0 要件更新に向けた議事コンテキスト

> 以下は v2.0 レビュー議事（2026-02-26）で確定した設計判断。`/kiro-spec-requirements wintf-P0-balloon01-core` で v3.0 要件を生成する際に反映すること。

### D1: ファサードパターン原則

外部からバルーンに与えるすべての設定情報は、**Balloon ルートエンティティのコンポーネント**として受け取る。内部エンティティ（BalloonFrame, BalloonContentArea）への手動伝播を利用者に要求しない。

**モデル**:
```
Balloon entity  (ルート = 外部 API)
├── Balloon             : マーカー + on_add + TextDirection（静的）
├── BalloonSkinDef      : フレーム外観          ← balloon01-core
├── BalloonStyleMap     : 感情値→スタイル KV     ← balloon01-core or balloon03
├── [キュー配送]       : cue-system が定義   ← wintf-P0-cue-system
├── Visual              : 表示制御              ← 既存
└── BoxStyle            : レイアウト制約        ← 既存

内部 (自動spawn、外部非公開)
└── BalloonFrame → BalloonContentArea → GlyphContainer → GlyphEntity×N
```

**理由**: Typewriter の成功パターン（`Typewriter` + `TypewriterTalk`）の正統な拡張。外部システムは Balloon エンティティに対してだけ操作すればよく、内部の BalloonFrame / BalloonContentArea を知る必要がない。

**検討した代替案**:
- Pattern A (フラットコンポーネント): API簡便だがルート肥大化・責務混在
- Pattern B (階層分散): 責務分離完璧だが内部構造露出が設計原則に反する
- Pattern C (ファサード + 内部伝播) ⭐: **採用**。API簡便さと責務分離のバランス
- Pattern D (コンパニオンエンティティ): ECS的に自然だが過度に複雑

### D2: TextDirection はバルーンの静的属性

`Balloon` コンポーネントに `TextDirection` フィールドを含める。1つのバルーン内で縦書き・横書きは混在しない。TextDirection の変更はコンテンツ全削除を伴う破壊的操作。

### D3: 感情値ベースの BalloonStyleMap

フォント・文字サイズ等のスタイル設定を、感情値（任意の文字列、例: 「笑顔」「怒り」）をキーとした KV マップ + デフォルト値で管理する。

- 構造: `HashMap<String, TextStyle>` + `default: TextStyle`
- TextStyle には: font_family, font_size, color 等
- アニメーション指示で感情値を切り替えることも可能だが非推奨。感情値による設定を推奨
- 帰属先: balloon01-core か balloon03-content かは v3.0 で決定

### D4: dola はバルーン内部サブシステムとしても利用可能

dola は「外部の調停層が使う」だけでなく、バルーン内部のタイミング計算（グリフ表示タイミング、タイプライター効果等）にも直接使える。`#[cfg(feature = "dola")]` での条件コンパイルは維持。

### D5: コンテンツコマンド配送は cue-system に外部化

バルーンへのコンテンツコマンド（テキスト表示、Wait、スタイル変更等）の配送メカニズムは `wintf-P0-cue-system` で汎用基盤として設計する。バルーン固有ではなく、キャラクター演技指令にも同一インターフェースが必要なため横断的関心事として分離。

**検討済みの配送方式**:

| 方式                       | 概要                                            | 評価                                   |
| -------------------------- | ----------------------------------------------- | -------------------------------------- |
| A: コンポーネント差し替え  | `XxxTalk::new(commands)` で丸ごと insert        | Typewriter踏襲。「追加」が不自然       |
| B: 子エンティティ追加      | `Command` entity を `ChildOf` で spawn          | ECS的に自然。順序保証設計が必要        |
| C: Messages\<T\>           | bevy_ecs メッセージキュー                       | 既存パターン(Drag系)。送り先特定が課題 |
| D: VecDeque コンポーネント | ルートに `CueQueue(VecDeque)` を付与、直接 push | 明白。Changed 検出が Mut\<T\> 必須     |

### v3.0 で追加すべき要件（候補）

1. **ファサードパターン原則**: ルートエンティティ = 外部 APIの原則を確立する要件
2. **TextDirection の Balloon コンポーネント化**: 静的属性として Balloon に含める
3. **BalloonStyleMap の位置づけ**: balloon01-core か balloon03 かを決定
4. **cue-system との結合点**: BalloonContentArea をキュー消費の拡張スロットとして定義
5. **dola 内部利用の設計制約**: バルーン内部サブシステムとしての dola 利用を許容する制約追加

---

## Requirements

### Requirement 1: 複合ウィジェットエンティティ階層の構築

**Objective:** 開発者として、Balloon の spawn 時に複合ウィジェットとしてのエンティティ階層が自動構築されるようにしたい。それにより各描画責務エンティティが正しい親子関係で配置され、後続子仕様が安定した基盤上に実装できる。

**親要件トレース**: R1（1.5 ECS管理）、AR-1（複合ウィジェット構造）

**設計方針**: `Balloon` はルート Visual ウィジェットとして透過サーフェスを持つが、自身の `GraphicsCommandList` は作成せず、全ての描画を子エンティティに委譲する。この委譲モデルにより既存ウィジェット（Rectangle, Label, BitmapSource等）の再利用と新規ウィジェットの独立開発が可能となる。

#### Acceptance Criteria

1. **When** `Balloon` コンポーネントがエンティティに追加された時, **the** Balloon Core **shall** `on_add` フックにより `BalloonFrame` 子エンティティを `ChildOf(balloon)` として自動 spawn する
2. **When** `Balloon` コンポーネントがエンティティに追加された時, **the** Balloon Core **shall** `on_add` フックにより `BalloonContentArea` 子エンティティを `ChildOf(balloon_frame)` として自動 spawn する
3. **The** Balloon Core **shall** エンティティ階層を `Balloon → BalloonFrame → BalloonContentArea` の3層構造で構築する
4. **The** Balloon Core **shall** `on_add` フック内の子エンティティ spawn を `DeferredWorld::commands()` による遅延実行で行う（`on_window_add` パターン準拠）
5. **The** Balloon Core **shall** 各子エンティティに `Visual` および `Arrangement` コンポーネントを自動挿入する

---

### Requirement 2: バルーンウィジェットの構成

**Objective:** 開発者として、バルーンを独立した描画ウィジェットとして spawn できるようにしたい。それにより外部システムがバルーンの配置・協調制御を自由に構成できる基盤を確立できる。

**親要件トレース**: R1（1.1〜1.5）

**設計方針**: Balloon コンポーネントはキャラクターへの参照や配置ロジックを持たない。バルーンは純粋な描画ウィジェットであり、どのウィンドウに配置するか、どう動かすかは外部からの指令による。

#### Acceptance Criteria

1. **The** Balloon Core **shall** `Balloon` コンポーネントを spawn するだけで、描画可能なバルーンウィジェットを生成できる
2. **The** Balloon Core **shall** 複数のバルーンエンティティを独立して生成できる
3. **The** Balloon Core **shall** バルーンエンティティを bevy_ecs エンティティとして管理する
4. **When** バルーンエンティティが despawn された時, **the** Balloon Core **shall** 子エンティティを適切に解放する

---

### Requirement 3: バルーンスキン定義インターフェース

**Objective:** 開発者として、バルーンの外観パラメータ（背景・枠線・しっぽ）をスキン定義として受け取るインターフェースを持ちたい。それによりスキン実装を差し替え可能な拡張ポイントを確立できる。

**親要件トレース**: R2（2.1）、AR-2（DR-1 フレーム描画責務）、AR-3（描画責務間の独立性）

#### Acceptance Criteria

1. **The** Balloon Core **shall** `BalloonSkinDef` コンポーネントを通じて、バルーンの背景定義（単色 or 画像）を受け取るインターフェースを提供する
2. **The** Balloon Core **shall** `BalloonSkinDef` コンポーネントを通じて、枠線パラメータ（色・幅・角丸半径）を受け取るインターフェースを提供する
3. **The** Balloon Core **shall** `BalloonSkinDef` コンポーネントを通じて、しっぽ定義（角度・サイズ・オフセット）を受け取るインターフェースを提供する
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

### Requirement 5: バルーン表示制御

**Objective:** 開発者として、バルーンの表示状態（表示/非表示）を制御したい。それにより外部システムが会話の開始・終了に応じた表示管理を行える。

**親要件トレース**: R4（4.1〜4.4）

#### Acceptance Criteria

1. **The** Balloon Core **shall** バルーンの表示/非表示を `Visual.is_visible` を通じて制御できる
2. **When** バルーンが非表示にされた時, **the** Balloon Core **shall** ECS エンティティとその子階層は保持する

---

### Requirement 6: エラーハンドリングと堅牢性

**Objective:** 開発者として、バルーン生成時のエラーが適切にハンドリングされ、システム全体の安定性が維持されるようにしたい。それにより不正な入力や状態変化に対しても予測可能な挙動を保証できる。

**親要件トレース**: inherited-context.md エラーハンドリング戦略

#### Acceptance Criteria

1. **If** `BalloonSkinDef` が `BalloonFrame` に付与されていない場合, **the** Balloon Core **shall** デフォルトスキンを適用してフレームを描画する
2. **If** スキン定義のパラメータが不正な場合, **the** Balloon Core **shall** `tracing::warn!` でログ出力し、デフォルトスキンにフォールバックする

---

### Requirement 7: モジュール配置と拡張性

**Objective:** 開発者として、バルーンコアのコードが既存の wintf アーキテクチャに適合した場所に配置され、後続子仕様の拡張が容易であるようにしたい。それにより保守性の高いモジュール構造を確立できる。

**親要件トレース**: AR-2（描画責務分離）、AR-3（描画責務間独立性）

#### Acceptance Criteria

1. **The** Balloon Core **shall** `ecs/widget/balloon/mod.rs` に `Balloon` コンポーネントと `on_add` フックを配置する
2. **The** Balloon Core **shall** `ecs/widget/balloon/frame.rs` に `BalloonFrame` と `BalloonSkinDef` コンポーネントを配置する
3. **The** Balloon Core **shall** 後続子仕様（balloon02〜balloon08）が `BalloonContentArea` の `ChildOf` 階層に新規エンティティを追加できる拡張ポイントを維持する（特別な拡張機構は不要、ECS の `ChildOf` パターンで十分）
4. **The** Balloon Core **shall** 既存の wintf レイヤー構造（COM → ECS → Message Handling）の依存方向に違反しない
