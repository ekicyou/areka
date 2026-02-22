# Requirements Document

| 項目 | 内容 |
|------|------|
| **Document Title** | wintf バルーンシステム 親仕様（マスタープラン） |
| **Version** | 3.0 |
| **Date** | 2026-02-22 |
| **Parent Spec** | ukagaka-desktop-mascot |
| **Priority** | P0 (MVP必須) |
| **Spec Type** | 親仕様（Constitutional） |

---

## Introduction

本仕様書は wintf フレームワークにおけるバルーン（吹き出し）システムの**親仕様**である。

バルーンシステムは**複合ウィジェット**として構成される。単一のエンティティではなく、親エンティティから子孫エンティティへの階層構造で構築され、各エンティティが明確な**描画責務**を担う。本仕様はその描画責務の定義と子仕様分割の根拠を定める。

### 本仕様のゴール

1. バルーンシステムの**アーキテクチャ要件**（複合ウィジェット構造・描画責務）を定義する
2. 描画責務に基づく**子仕様分割**と**依存関係**を定義する
3. 子仕様が参照する**憲法的な基本原則**を確立する
4. 子仕様の進捗を追跡する**ロードマップ**を管理する

### 実装完了条件

本仕様の実装完了は、以下の**すべて**が満たされた時とする：

- すべての子仕様が完了していること
- ロードマップの全マイルストーンが達成されていること
- 統合検証（M-4）により子仕様間の連携が検証されていること

### 親仕様からのトレーサビリティ

本仕様は `ukagaka-desktop-mascot` の以下の要件をカバーする：

| 親要件ID | 内容 | 子仕様 |
|----------|------|--------|
| 3.1 | キャラクターに紐付いた吹き出しウィンドウを表示できる | balloon-core |
| 3.2 | 複数キャラクターそれぞれに独立した吹き出しを表示できる | balloon-core |
| 3.3 | 吹き出し内にテキストを表示できる | balloon-content |
| 3.7 | テキストがクリックされた時、リンクとしてアクションを実行できる | balloon-rich-text |
| 3.8 | テキスト表示中、ルビ（ふりがな）を表示できる | balloon-rich-text |
| 3.9 | 選択肢形式の入力をユーザーに提示できる | balloon-choice |
| 3.10 | ユーザーが選択肢をクリックした時、対応するイベントを発火する | balloon-choice |

---

## アーキテクチャ要件

### AR-1: 複合ウィジェット構造

バルーンシステムは**単一のエンティティで構成されてはならず**、親エンティティから子孫エンティティへの階層構造による**複合ウィジェット**として構成されなければならない。

各子孫エンティティは明確な描画責務を持ち、その描画責務の範囲内で自律的に動作する。

### AR-2: 描画責務の分離

バルーンシステムの視覚的構成は、以下の**描画責務**に分離される。各描画責務は独立して開発・テスト可能な単位とする。

| # | 描画責務 | 内容 | 担当子仕様 |
|---|---------|------|-----------|
| DR-1 | フレーム描画 | バルーンウィンドウの背景・形状・枠線の描画。スキンによる外観変更の受け口。 | balloon-core |
| DR-2 | ビューポート描画 | コンテンツ領域のクリッピング、スクロール位置管理。コンテンツ量に応じた表示制御。 | balloon-content |
| DR-3 | テキスト本文描画（基本） | 文字の基本描画、縦横書き、文字単位表示制御。 | (typewriter P0 ✅ 基礎機能完了) |
| DR-4 | テキスト装飾描画 | ルビ（ふりがな）、リンク（クリッカブルテキスト）のテキスト上への重畳描画。 | balloon-rich-text |
| DR-5 | 選択肢UI描画 | 選択肢ボタンのレイアウト・描画・ホバー状態フィードバック。 | balloon-choice |
| DR-6 | テキスト本文描画（エフェクト） | 文字単位エフェクト（フェードイン・アウト等）、アニメーション管理との連動、描画領域管理の強化。 | balloon-text-effects |

### AR-3: 描画責務間の独立性

各描画責務は他の描画責務の内部実装に直接依存せず、親子関係を通じたレイアウト情報の伝達のみにより連携するものとする。描画責務の境界を越える変更が必要な場合は、本親仕様を先に改訂すること。

---

## 子仕様分割

本仕様は以下の子仕様に分割される。各子仕様は独立した設計・タスク・実装サイクルを持つ。分割は上記の**描画責務**に基づく。

| # | 子仕様 | 描画責務 | スコープ | 依存 | フェーズ |
|---|--------|---------|----------|------|---------|
| 1 | `wintf-P0-balloon-core` | DR-1: フレーム描画 | ウィンドウ生成・配置・表示制御・フレーム描画基盤 | event-system ✅ | B-1: 最優先 |
| 2 | `wintf-P0-balloon-content` | DR-2: ビューポート描画 | コンテンツ領域管理・typewriter統合・スクロール | balloon-core, typewriter ✅ | B-2 |
| 3 | `wintf-P0-balloon-rich-text` | DR-4: テキスト装飾描画 | ルビ・リンク（リッチテキスト拡張） | balloon-content | B-3 |
| 4 | `wintf-P0-balloon-choice` | DR-5: 選択肢UI描画 | 選択肢専用バルーンウィンドウ（ChoiceBalloon） | balloon-core, event-system ✅ | B-3（rich-textと並行可） |
| 5 | `wintf-P0-balloon-text-effects` | DR-6: テキストエフェクト描画 | 文字単位エフェクト・アニメーション連動・描画領域管理 | typewriter ✅, balloon-content | B-4 |

```
依存関係:

                                    ┌─► balloon-content ──┬─► balloon-rich-text
                                    │        ↑            │
  event-system ✅ ──► balloon-core ──┤   typewriter ✅ ────┴─► balloon-text-effects
                                    │
                                    └─► balloon-choice
```

### スコープ（全体）

**含まれるもの:**
- バルーンウィンドウのフレーム描画基盤・生成・配置・管理（balloon-core / DR-1）
- コンテンツ領域のビューポート描画・typewriter統合・スクロール（balloon-content / DR-2）
- テキスト本文の基本描画（typewriter P0 ✅ / DR-3）
- テキスト装飾のルビ・リンク描画（balloon-rich-text / DR-4）
- 選択肢専用バルーンウィンドウの描画（balloon-choice / DR-5）
- テキスト本文のエフェクト描画・アニメーション連動（balloon-text-effects / DR-6）

**含まれないもの:**
- バルーンスキンの定義（`areka-P0-reference-balloon` の責務）
- テキスト入力ボックス（親仕様スコープ外、将来の別仕様で対応）

---

## Requirements

> **凡例**: 各要件のヘッダに `[子仕様名]` を付記し、どの子仕様（描画責務）で実装されるかを明示する。

---

### 子仕様 1: balloon-core（DR-1: フレーム描画）

バルーンウィンドウのフレーム描画基盤とライフサイクル管理を担う最基盤レイヤー。ウィンドウの背景・形状の描画責務を持ち、他のすべてのバルーン子仕様がこの上に構築される。

---

#### Requirement 1: バルーンウィンドウ生成 [balloon-core]

**Objective:** 開発者として、キャラクターに紐付いたバルーンウィンドウを生成・管理したい。それによりキャラクターの発言表示の基盤を確立できる。

##### Acceptance Criteria

1. **The** Balloon Core **shall** キャラクターエンティティに紐付いたバルーンウィンドウを生成できる
2. **The** Balloon Core **shall** 複数のキャラクターそれぞれに独立したバルーンウィンドウを生成できる
3. **The** Balloon Core **shall** バルーンウィンドウを透過ウィンドウとして生成できる
4. **When** バルーンが不要になった時, **the** Balloon Core **shall** ウィンドウリソースを適切に解放する
5. **The** Balloon Core **shall** バルーンウィンドウをECSエンティティとして管理する

---

#### Requirement 2: バルーン配置制御 [balloon-core]

**Objective:** 開発者として、バルーンをキャラクターの近傍に自動配置したい。それによりどのキャラクターの発言かが視覚的に明確になる。

##### Acceptance Criteria

1. **The** Balloon Core **shall** バルーンをキャラクターウィンドウの近傍に自動配置できる
2. **The** Balloon Core **shall** バルーンの配置方向（上/下/左/右）を指定できる
3. **When** キャラクターウィンドウが移動した時, **the** Balloon Core **shall** バルーンの位置を追従させる
4. **When** バルーンがデスクトップ領域外に出る場合, **the** Balloon Core **shall** 配置方向を自動反転してデスクトップ内に収まるよう調整する
5. **The** Balloon Core **shall** バルーンとキャラクター間のオフセット距離を設定できる

---

#### Requirement 3: バルーン表示制御 [balloon-core]

**Objective:** 開発者として、バルーンの表示状態を制御したい。それにより会話の開始・終了に応じた表示管理ができる。

##### Acceptance Criteria

1. **The** Balloon Core **shall** バルーンの表示/非表示を制御できる
2. **When** バルーンが表示された時, **the** Balloon Core **shall** キャラクターウィンドウの前面に表示する
3. **When** バルーンが非表示にされた時, **the** Balloon Core **shall** ウィンドウを非表示にしつつエンティティは保持する
4. **The** Balloon Core **shall** バルーンのサイズを設定できる

---

### 子仕様 2: balloon-content（DR-2: ビューポート描画）

バルーン内のコンテンツ領域のビューポート描画を担う。コンテンツのクリッピング・スクロール表示を管理し、typewriter ウィジェットの配置基盤を提供する。

---

#### Requirement 4: コンテンツ領域管理 [balloon-content]

**Objective:** 開発者として、バルーン内にコンテンツ領域を定義したい。それによりテキストやウィジェットの配置基盤を確立できる。

##### Acceptance Criteria

1. **The** Balloon Content **shall** バルーンウィンドウ内にコンテンツ領域（テキスト表示エリア）を定義できる
2. **The** Balloon Content **shall** コンテンツ領域のマージン・パディングを設定できる
3. **The** Balloon Content **shall** コンテンツ量に応じてバルーンサイズを自動調整できる
4. **The** Balloon Content **shall** コンテンツ領域の最大サイズ制約を設定できる

---

#### Requirement 5: Typewriter統合 [balloon-content]

**Objective:** 開発者として、バルーン内に完了済みのTypewriterウィジェットを配置してテキスト表示したい。それにより既存のタイプライター効果をバルーンで利用できる。

##### Acceptance Criteria

1. **The** Balloon Content **shall** バルーンのコンテンツ領域にTypewriterウィジェットを配置できる（Typewriter既存の縦書き・横書き機能はそのまま利用可能）
2. **When** TypewriterTalkが設定された時, **the** Balloon Content **shall** コンテンツ領域内でテキストをレイアウトして表示する
3. **The** Balloon Content **shall** フォント、サイズ、色のスタイル設定をTypewriterに委譲できる

---

#### Requirement 6: テキストスクロール [balloon-content]

**Objective:** 開発者として、長文テキストをバルーン内でスクロール表示したい。それによりコンテンツ領域に収まらない長文も閲覧できる。

##### Acceptance Criteria

1. **When** テキストがコンテンツ領域の高さを超えた時, **the** Balloon Content **shall** スクロール表示を有効にする
2. **The** Balloon Content **shall** タイプライターの表示進行に追従してスクロール位置を自動調整する
3. **The** Balloon Content **shall** マウスホイールによるスクロール操作をサポートする
4. **The** Balloon Content **shall** ページ送り（スクロール位置をコンテンツ領域の高さ分だけ移動するPageDown相当の操作）をサポートする

---

### 子仕様 3: balloon-rich-text（DR-4: テキスト装飾描画）

テキスト本文の上に重畳される装飾要素の描画を担う。DirectWriteのテキストレイアウト拡張とイベントシステムとの統合を必要とする、技術的に高度なレイヤー。

---

#### Requirement 7: ルビ（ふりがな）表示 [balloon-rich-text]

**Objective:** 開発者として、テキストにルビ（ふりがな）を付加したい。それにより漢字の読み方を示し、テキストの可読性を向上させる。

##### Acceptance Criteria

1. **The** Balloon Rich Text **shall** テキストの指定範囲にルビ（ふりがな）を表示できる
2. **The** Balloon Rich Text **shall** 横書き時のルビ配置（親文字の上側）をサポートする
3. **The** Balloon Rich Text **shall** 縦書き時のルビ配置（親文字の右側）をサポートする
4. **The** Balloon Rich Text **shall** ルビのフォントサイズを親文字に対して自動調整できる
5. **The** Balloon Rich Text **shall** ルビ情報をTypewriterのIRトークンとして受け渡しできる

---

#### Requirement 8: リンク（クリッカブルテキスト） [balloon-rich-text]

**Objective:** 開発者として、テキスト内にクリック可能なリンクを設定したい。それによりユーザーのアクションをトリガーできる。

##### Acceptance Criteria

1. **The** Balloon Rich Text **shall** テキスト内の指定範囲をクリッカブルなリンクとして設定できる
2. **When** リンクがクリックされた時, **the** Balloon Rich Text **shall** リンクIDを含むイベントを発火する
3. **The** Balloon Rich Text **shall** リンクの外観（色、下線等）をカスタマイズできる
4. **When** マウスがリンク上にある時, **the** Balloon Rich Text **shall** ホバー状態を視覚的にフィードバックする
5. **The** Balloon Rich Text **shall** リンクのヒットテスト領域をDirectWriteのテキスト位置情報から算出する
6. **The** Balloon Rich Text **shall** リンク情報をTypewriterのIRトークンとして受け渡しできる

---

### 子仕様 4: balloon-choice（DR-5: 選択肢UI描画）

選択肢UIの描画を担う**独立した専用バルーンウィンドウ（ChoiceBalloon）**を提供する。テキストバルーンとは別ウィンドウとして生成され、同キャラクターに紐付いて配置される。選択肢ボタンのレイアウト・描画・インタラクションを責務とする。

---

#### Requirement 9: 選択肢UI [balloon-choice]

**Objective:** 開発者として、ユーザーに選択肢を選択肢専用バルーンで提示したい。それによりテキスト表示を妨げずにインタラクティブな会話分岐を実現できる。

##### Acceptance Criteria

1. **The** Balloon Choice **shall** 選択肢専用バルーンウィンドウ（ChoiceBalloon）を生成し、キャラクターに紐付いて配置できる
2. **The** Balloon Choice **shall** 複数の選択肢を縦並びで表示できる
3. **When** ユーザーが選択肢をクリックした時, **the** Balloon Choice **shall** 選択肢IDを含むイベントを発火する
4. **When** マウスが選択肢上にある時, **the** Balloon Choice **shall** ホバー状態を視覚的にフィードバックする
5. **The** Balloon Choice **shall** キーボード操作（上下キー、Enter）での選択をサポートする

---

### 子仕様 5: balloon-text-effects（DR-6: テキストエフェクト描画）

テキスト本文に対する視覚エフェクト・アニメーション連動を担う。typewriterの基本描画機能（DR-3）を拡張し、文字単位のエフェクト表現とdolaアニメーションシステムとの統合を提供する。

---

#### Requirement 10: 文字単位エフェクト [balloon-text-effects]

**Objective:** 開発者として、テキスト表示に文字単位のエフェクト（フェードイン・アウト等）を適用したい。それによりテキスト演出の表現力を向上させる。

##### Acceptance Criteria

1. **The** Balloon Text Effects **shall** 文字単位でのフェードイン（不透明度0→1）エフェクトを適用できる
2. **The** Balloon Text Effects **shall** 文字単位でのフェードアウト（不透明度1→0）エフェクトを適用できる
3. **The** Balloon Text Effects **shall** エフェクトの開始タイミング・継続時間を文字ごとに設定できる
4. **The** Balloon Text Effects **shall** 複数エフェクトの同時適用（例: フェード＋スライド）をサポートする
5. **The** Balloon Text Effects **shall** エフェクト適用中の文字の描画領域を適切に管理し、クリッピング問題を回避する

---

#### Requirement 11: アニメーション統合 [balloon-text-effects]

**Objective:** 開発者として、テキストエフェクトをdolaアニメーションシステムと連動させたい。それによりタイムライン制御されたテキスト演出を実現できる。

##### Acceptance Criteria

1. **The** Balloon Text Effects **shall** dolaアニメーション定義ファイルからテキストエフェクトを読み込める
2. **The** Balloon Text Effects **shall** dolaのイージング関数をエフェクトに適用できる
3. **When** dolaストーリーボードが再生された時, **the** Balloon Text Effects **shall** タイムラインに同期してエフェクトを実行する
4. **The** Balloon Text Effects **shall** アニメーションの一時停止・再開・逆再生に対応する
5. **The** Balloon Text Effects **shall** TypewriterTalkの進行とdolaアニメーションを協調させる

---

## ガバナンス要件

子仕様と親仕様の関係を規定する。

### GR-1: 子仕様から親仕様への準拠

各子仕様は、本親仕様で定義されたアーキテクチャ要件（AR-1〜AR-3）および描画責務の定義に準拠しなければならない。

### GR-2: ロードマップ更新義務

各子仕様の完了時に、本親仕様のロードマップを更新し、完了状態を反映しなければならない。これは各子仕様のタスクに含められるものとする。

### GR-3: 描画責務境界の尊重

子仕様の設計・実装において、他の子仕様の描画責務境界を侵害しないこと。描画責務の変更が必要な場合は、本親仕様を先に改訂すること。

---

## ロードマップ

| マイルストーン | 子仕様 | 状態 | 前提条件 |
|---------------|--------|------|---------|
| M-1: バルーンウィンドウ基盤 | balloon-core | 未着手 | event-system ✅ |
| M-2: テキスト表示パイプライン | balloon-content | 未着手 | M-1 完了, typewriter ✅ |
| M-3a: リッチテキスト装飾 | balloon-rich-text | 未着手 | M-2 完了 |
| M-3b: 選択肢バルーン | balloon-choice | 未着手 | M-1 完了 |
| M-3c: テキストエフェクト・アニメーション | balloon-text-effects | 未着手 | M-2 完了, typewriter ✅ |
| M-4: 統合検証 | （親仕様） | 未着手 | M-3a, M-3b, M-3c 完了 |

---

## Non-Functional Requirements

### NFR-1: パフォーマンス

1. バルーン表示・配置変更時の描画遅延は16ms（60fps相当）以内であること
2. 長文テキストのスクロールが滑らかであること（60fps維持）
3. 複数バルーンの同時表示時にも描画性能が劣化しないこと

### NFR-2: 互換性

1. Windows 10 (1803) 以降をサポートすること
2. 高DPI環境でバルーンのスケーリングが正しく動作すること
3. マルチモニター環境でバルーンの配置が正しく機能すること

### NFR-3: ECS統合

1. すべてのバルーン要素はbevy_ecsのエンティティ・コンポーネントとして実装されること
2. 既存のwintfウィンドウ管理・グラフィックスパイプラインと統合されること
3. 既存のイベントシステム（event-system ✅）を再利用すること

---

## Dependencies

### 依存する仕様

| 仕様 | 状態 | 依存内容 |
|------|:----:|----------|
| `wintf-P0-typewriter` | ✅ 完了 | テキスト本文描画（DR-3）、文字単位制御、IR型定義 |
| `wintf-P0-event-system` | ✅ 完了 | ヒットテスト、イベント配信 |

### 依存される仕様

| 仕様 | 依存内容 |
|------|----------|
| `areka-P0-reference-balloon` | バルーンスキンの適用（DR-1フレーム描画の外観定義） |

---

## Glossary

| 用語 | 定義 |
|------|------|
| **バルーン** | キャラクターの発言を表示する吹き出しウィンドウ |
| **複合ウィジェット** | 親エンティティから子孫エンティティへの階層構造で構成されるウィジェット。各エンティティが固有の描画責務を持つ |
| **描画責務** | 複合ウィジェット内の各エンティティが担う、特定の視覚要素の描画に関する責任範囲 |
| **ChoiceBalloon** | 選択肢のみを表示する専用バルーンウィンドウ。テキストバルーンとは独立した別ウィンドウとして同キャラクターに紐付いて配置される |
| **コンテンツ領域** | バルーンウィンドウ内のテキスト・ウィジェット配置エリア（ビューポート） |
| **ルビ** | 漢字等の上または横に付けるふりがな |
| **選択肢** | ユーザーがクリックして選ぶ複数の選択肢ボタン |
| **リンク** | クリック可能なテキスト領域 |
| **Typewriter** | 完了済み仕様 `wintf-P0-typewriter` が提供する文字単位表示ウィジェット（DR-3担当） |
| **IRトークン** | Typewriterが受け取る構造化入力データ（Stage 1 IR） |
