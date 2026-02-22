# Requirements Document

| 項目 | 内容 |
|------|------|
| **Document Title** | wintf バルーンシステム 要件定義書（マスタープラン） |
| **Version** | 2.0 |
| **Date** | 2026-02-22 |
| **Parent Spec** | ukagaka-desktop-mascot |
| **Priority** | P0 (MVP必須) |

---

## Introduction

本仕様書は wintf フレームワークにおけるバルーン（吹き出し）システム全体の要件を定義するマスタープランである。キャラクターの発言を視覚的に表示し、ユーザーとの対話を実現することを目的とする。

本仕様は**4つの子仕様に分割して段階的に実装**される。各要件は担当する子仕様を明示し、依存関係と実装順序を定義する。

### 親仕様からのトレーサビリティ

本仕様は `ukagaka-desktop-mascot` の以下の要件をカバーする：

| 親要件ID | 内容 | 子仕様 |
|----------|------|--------|
| 3.1 | キャラクターに紐付いた吹き出しウィンドウを表示できる | balloon-core |
| 3.2 | 複数キャラクターそれぞれに独立した吹き出しを表示できる | balloon-core |
| 3.3 | 吹き出し内にテキストを表示できる | balloon-content |
| 3.7 | テキストがクリックされた時、リンクとしてアクションを実行できる | balloon-rich-text |
| 3.8 | テキスト表示中、ルビ（ふりがな）を表示できる | balloon-rich-text |
| 3.9 | 選択肢形式の入力をユーザーに提示できる | balloon-input |
| 3.10 | ユーザーが選択肢をクリックした時、対応するイベントを発火する | balloon-input |

### 子仕様分割

本仕様は以下の4つの子仕様に分割される。各子仕様は独立した設計・タスク・実装サイクルを持つ。

| # | 子仕様 | スコープ | 依存 | 実装フェーズ |
|---|--------|----------|------|-------------|
| 1 | `wintf-P0-balloon-core` | ウィンドウ管理・配置・ライフサイクル | event-system ✅ | B-1: 最優先 |
| 2 | `wintf-P0-balloon-content` | テキスト表示領域・typewriter統合・スクロール | balloon-core, typewriter ✅ | B-2 |
| 3 | `wintf-P0-balloon-rich-text` | ルビ・リンク（リッチテキスト拡張） | balloon-content | B-3 |
| 4 | `wintf-P0-balloon-input` | 選択肢UI・入力ボックス（インタラクション） | balloon-core, event-system ✅ | B-3（rich-textと並行可） |

```
依存関係:

  event-system ✅ ─┬─► balloon-core ─┬─► balloon-content ─► balloon-rich-text
  typewriter ✅ ────┘                └─► balloon-input
```

### スコープ（全体）

**含まれるもの:**
- バルーンウィンドウの生成・配置・管理（balloon-core）
- テキスト表示領域とtypewriter統合（balloon-content）
- ルビ（ふりがな）表示（balloon-rich-text）
- リンク（クリッカブルテキスト）（balloon-rich-text）
- 選択肢UI（balloon-input）
- 入力ボックス（balloon-input）

**含まれないもの:**
- タイプライター表示制御（`wintf-P0-typewriter` ✅ 完了済み）
- バルーンスキンの定義（`areka-P0-reference-balloon` の責務）
- 縦書きテキストレンダリング詳細（`wintf-P0-typewriter` ✅ 完了済み）

---

## Requirements

> **凡例**: 各要件のヘッダに `[子仕様名]` を付記し、どの子仕様で実装されるかを明示する。

---

### 子仕様 1: balloon-core（ウィンドウ管理・配置）

バルーンウィンドウのライフサイクル管理とキャラクターへの紐付けを担う最基盤レイヤー。他のすべてのバルーン子仕様がこの上に構築される。

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

### 子仕様 2: balloon-content（テキスト表示・typewriter統合）

バルーン内のコンテンツ領域を管理し、typewriter ウィジェットとの統合を担う。テキスト表示の基本的なレイアウト（折り返し・スクロール）を提供する。

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

1. **The** Balloon Content **shall** バルーンのコンテンツ領域にTypewriterウィジェットを配置できる
2. **The** Balloon Content **shall** 縦書きテキスト表示をサポートする（TypewriterのDirectWrite統合を利用）
3. **The** Balloon Content **shall** 横書きテキスト表示をサポートする
4. **When** TypewriterTalkが設定された時, **the** Balloon Content **shall** コンテンツ領域内でテキストをレイアウトして表示する
5. **The** Balloon Content **shall** フォント、サイズ、色のスタイル設定をTypewriterに委譲できる

---

#### Requirement 6: テキストスクロール [balloon-content]

**Objective:** 開発者として、長文テキストをバルーン内でスクロール表示したい。それによりコンテンツ領域に収まらない長文も閲覧できる。

##### Acceptance Criteria

1. **When** テキストがコンテンツ領域の高さを超えた時, **the** Balloon Content **shall** スクロール表示を有効にする
2. **The** Balloon Content **shall** タイプライターの表示進行に追従してスクロール位置を自動調整する
3. **The** Balloon Content **shall** マウスホイールによるスクロール操作をサポートする
4. **The** Balloon Content **shall** ページ送り（コンテンツ領域単位の表示切替）をサポートする

---

### 子仕様 3: balloon-rich-text（ルビ・リンク）

テキスト表示にリッチテキスト要素を追加する。DirectWriteのテキストレイアウト拡張とイベントシステムとの統合を必要とする、技術的に高度なレイヤー。

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

### 子仕様 4: balloon-input（選択肢・入力）

バルーン内にインタラクティブなUI要素を配置する。テキスト表示とは独立した入力ウィジェット群を提供し、ユーザーからの応答を受け取る。

---

#### Requirement 9: 選択肢UI [balloon-input]

**Objective:** 開発者として、ユーザーに選択肢を提示したい。それによりインタラクティブな会話分岐を実現できる。

##### Acceptance Criteria

1. **The** Balloon Input **shall** 選択肢形式の入力をバルーン内にユーザーに提示できる
2. **The** Balloon Input **shall** 複数の選択肢を縦並びで表示できる
3. **When** ユーザーが選択肢をクリックした時, **the** Balloon Input **shall** 選択肢IDを含むイベントを発火する
4. **When** マウスが選択肢上にある時, **the** Balloon Input **shall** ホバー状態を視覚的にフィードバックする
5. **The** Balloon Input **shall** キーボード操作（上下キー、Enter）での選択をサポートする
6. **When** 選択肢が表示されている間にテキスト領域がスクロールされた場合, **the** Balloon Input **shall** 選択肢の表示位置を適切に維持する

---

#### Requirement 10: 入力ボックス [balloon-input]

**Objective:** 開発者として、ユーザーからテキスト入力を受け取りたい。それにより自由形式の応答を取得できる。

##### Acceptance Criteria

1. **The** Balloon Input **shall** テキスト入力ボックスをバルーン内に表示できる
2. **When** ユーザーがテキストを入力してEnterを押した時, **the** Balloon Input **shall** 入力内容を含むイベントを発火する
3. **The** Balloon Input **shall** 入力ボックスのプレースホルダーテキストを設定できる
4. **The** Balloon Input **shall** 入力文字数の制限を設定できる
5. **When** 入力ボックスが表示された時, **the** Balloon Input **shall** 自動的にキーボードフォーカスを取得する

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
| `wintf-P0-typewriter` | ✅ 完了 | テキスト表示、文字単位制御、IR型定義 |
| `wintf-P0-event-system` | ✅ 完了 | ヒットテスト、イベント配信 |

### 依存される仕様

| 仕様 | 依存内容 |
|------|----------|
| `areka-P0-reference-balloon` | バルーンスキンの適用 |

---

## Glossary

| 用語 | 定義 |
|------|------|
| **バルーン** | キャラクターの発言を表示する吹き出しウィンドウ |
| **コンテンツ領域** | バルーンウィンドウ内のテキスト・ウィジェット配置エリア |
| **ルビ** | 漢字等の上または横に付けるふりがな |
| **選択肢** | ユーザーがクリックして選ぶ複数の選択肢ボタン |
| **リンク** | クリック可能なテキスト領域 |
| **入力ボックス** | ユーザーがテキストを入力するフィールド |
| **Typewriter** | 完了済み仕様 `wintf-P0-typewriter` が提供する文字単位表示ウィジェット |
| **IRトークン** | Typewriterが受け取る構造化入力データ（Stage 1 IR） |
