# ギャップ分析レポート

| 項目 | 内容 |
|------|------|
| **対象仕様** | wintf-P0-balloon-system |
| **分析日** | 2026-02-22 |
| **対象バージョン** | requirements.md v2.1 |

---

## 1. 現状調査サマリ

### 1.1 関連資産の概要

| 分類 | ファイル/モジュール | 説明 |
|------|---------------------|------|
| ウィンドウ管理 | `ecs/window/components.rs` | `Window`, `WindowStyle`, `CompositionMode` + `on_window_add` フック |
| ウィンドウ位置 | `ecs/window/window_pos.rs` | `WindowPos` + `SetWindowPosCommand` キュー |
| モニター | `ecs/window/monitor.rs` | `Monitor { bounds, work_area, dpi }` — マルチモニター対応 |
| タイプライター | `ecs/widget/text/typewriter.rs` | `Typewriter`, `TypewriterTalk`, `TypewriterLayoutCache` |
| タイプライターIR | `ecs/widget/text/typewriter_ir.rs` | Stage 1 `TypewriterToken`, Stage 2 `TimelineItem` |
| ポインタ | `ecs/pointer/` | `PointerState`, `WheelDelta`, `Phase<T>` (Tunnel/Bubble) |
| ヒットテスト | `ecs/layout/hit_test.rs` | `HitTestMode`, `hit_test_in_window()` |
| ヒットリージョン | `ecs/layout/hit_region.rs` | `HitRegionMap` (rect/polygon/colormap) |
| レイアウト | `ecs/layout/` | `TaffyStyle`, `TaffyComputedLayout`, `LayoutRoot` 階層 |
| DirectWrite | `com/dwrite.rs` | `DWriteFactoryExt`, `DWriteTextLayoutExt` |
| キーボード | `ecs/window_proc/keyboard.rs` | WM_KEYDOWN (ESC のみ), WM_CANCELMODE, WM_ACTIVATE |
| メッセージハンドラ | `win_message_handler.rs` | WM_CHAR/WM_IME_*/WM_SETFOCUS スタブあり（ECS未接続） |
| モック実装 | `areka/src/main.rs` | `BalloonWindowMarker`, ハードコード配置、手動追従 |
| スケジュール | `ecs/world/mod.rs` | Input → Update → PreLayout → ... → FrameFinalize |

### 1.2 アーキテクチャパターン

- **3層構造**: COM ラッパー → ECS コンポーネント → メッセージハンドラ
- **on_add フック**: `Window`, `Typewriter` 等で自動コンポーネント挿入パターンが確立
- **コマンドキュー**: `SetWindowPosCommand` のような thread_local キューで World 借用衝突を回避
- **イベントシステム**: `Phase<T>` (Tunnel/Bubble) ディスパッチ、`OnPointer*` イベント群
- **命名規則**: GPU リソース=`XxxGraphics`, CPU リソース=`XxxResource`, 論理=サフィックスなし

### 1.3 制約事項

- `on_add` フック内では `Commands` が使えない（`DeferredWorld` のみ）
- `SetWindowPosCommand` は tick 終了後にフラッシュされる（即時反映不可）
- ULW 合成モードでは `UpdateLayeredWindow` のたびに全面再描画が必要
- PointerState の `WheelDelta` は蓄積されるが、スクロールウィジェットが未実装

---

## 2. 要件−資産マッピング

### 子仕様 1: balloon-core

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 1: ウィンドウ生成** | 1 | `Window` コンポーネント + `on_window_add` | バルーン専用コンポーネント（`BalloonWindow`）未定義。シェル↔バルーン ECS関係なし | Missing |
| | 2 | `Window` は複数生成可能 | 同上。キャラクター単位の多重バルーン管理ロジックなし | Missing |
| | 3 | `CompositionMode::ULW` で透過ウィンドウ生成済み | ✅ 既存パターンで対応可能 | — |
| | 4 | `on_remove` フック基盤あり | バルーン固有のクリーンアップロジック未定義 | Missing |
| | 5 | bevy_ecs でエンティティ管理済み | バルーン専用マーカー/関係コンポーネント未定義 | Missing |
| **Req 2: 配置制御** | 1 | モック: `BALLOON_OFFSET_X=335` ハードコード | 自動配置アルゴリズム未実装 | Missing |
| | 2 | なし | 配置方向（上/下/左/右）指定機能なし | Missing |
| | 3 | モック: `OnDrag` ハンドラで手動 `SetWindowPosCommand` | ECSシステムとしての自動追従未実装 | Missing |
| | 4 | `Monitor.work_area` でデスクトップ領域取得可能 | 自動反転ロジック未実装 | Missing |
| | 5 | なし | オフセット距離設定コンポーネント未定義 | Missing |
| **Req 3: 表示制御** | 1 | `WindowPos.show_window` / `hide_window` フラグ | ✅ 既存機構で対応可能 | — |
| | 2 | `WindowPos.zorder` (TopMost 等) | ✅ 前面表示は既存 ZOrder で可能 | — |
| | 3 | `WindowPos.hide_window` | ✅ ウィンドウ非表示+エンティティ保持は既存で可能 | — |
| | 4 | `WindowPos.size` + `TaffyStyle` | ✅ サイズ設定可能 | — |

### 子仕様 2: balloon-content

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 4: コンテンツ領域** | 1 | `LayoutRoot` 階層 + `TaffyStyle` | コンテンツ領域コンポーネント（マージン/パディング定義）未定義 | Missing |
| | 2 | `TaffyStyle` に margin/padding あり | ✅ taffy で設定可能 | — |
| | 3 | `TaffyStyle` の content-based sizing | サイズ自動調整ロジック（コンテンツ→ウィンドウへの反映）未実装 | Missing |
| | 4 | `TaffyStyle` に max_width/max_height | ✅ taffy の制約で設定可能 | — |
| **Req 5: Typewriter統合** | 1 | `Typewriter` コンポーネント + モック配置例あり | ✅ 既存パターンでバルーン子エンティティとして配置可（縦書き・横書き対応済み） | — |
| | 2 | `TypewriterTalk` + `TypewriterLayoutCache` | ✅ テキストレイアウト・表示パイプライン済み | — |
| | 3 | `Typewriter { font_family, font_size, ... }` | ✅ スタイル設定 Typewriter が保持 | — |
| **Req 6: スクロール** | 1 | なし | スクロールコンテナウィジェット未実装 | Missing |
| | 2 | なし | タイプライター進行追従のスクロール制御未実装 | Missing |
| | 3 | `WheelDelta` 取得可能 | ホイール→スクロール変換ロジック未実装 | Missing |
| | 4 | なし | ページ送り機構未実装 | Missing |

### 子仕様 3: balloon-rich-text

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 7: ルビ** | 1 | DirectWrite `IDWriteTextLayout` | ルビ用 DirectWrite API 未ラップ（`IDWriteTextLayout1` 以降の API 未使用） | Missing |
| | 2 | なし | 横書きルビ配置ロジック未実装 | Missing |
| | 3 | なし | 縦書きルビ配置ロジック未実装 | Missing |
| | 4 | なし | ルビフォントサイズ自動調整未実装 | Missing |
| | 5 | `TypewriterToken` (Stage 1 IR) | ルビ用トークン variant 未定義 (`TypewriterToken::Ruby` 等) | Missing |
| **Req 8: リンク** | 1 | `HitRegionMap` (rect/polygon) | テキスト位置ベースのヒットリージョン生成未実装 | Missing |
| | 2 | `Phase<T>` イベントシステム | リンクイベント定義未定義 | Missing |
| | 3 | なし | リンク外観カスタマイズ機構未実装 | Missing |
| | 4 | `OnPointerMoved` / `OnPointerEntered` | ホバー状態管理コンポーネント未定義 | Constraint |
| | 5 | `DWriteTextLayoutExt::hit_test_text_position` | テキスト座標→文字位置の逆引き API 未ラップ (`HitTestPoint` 等) | Missing |
| | 6 | `TypewriterToken` (Stage 1 IR) | リンク用トークン variant 未定義 (`TypewriterToken::Link` 等) | Missing |

### 子仕様 4: balloon-input

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 9: 選択肢UI** | 1 | `OnPointerPressed` イベント | 選択肢ウィジェットコンポーネント未定義 | Missing |
| | 2 | `TaffyStyle` (flexbox column) | ✅ flexbox 縦並び可能 | — |
| | 3 | `Phase<T>` イベントシステム | 選択肢イベント定義未定義 | Missing |
| | 4 | `OnPointerEntered` / `OnPointerExited` | ホバー状態ウィジェット未実装 | Missing |
| | 5 | `keyboard.rs`: WM_KEYDOWN (ESC のみ) | キーボードナビゲーション基盤未実装（上下キー・Enter） | Missing |
| | 6 | なし | 選択肢とスクロールの座標統合未実装 | Missing |

---

## 3. ギャップサマリ

### 3.1 主要ギャップ一覧

| # | ギャップ | 影響範囲 | 深刻度 |
|---|---------|---------|--------|
| G1 | バルーン専用ECSコンポーネント群が未定義 | balloon-core 全体 | 高 |
| G2 | シェル↔バルーンのECSリレーション機構なし | Req 1, 2 | 高 |
| G3 | 自動配置・追従・反転アルゴリズム未実装 | Req 2 全AC | 高 |
| G4 | スクロールコンテナウィジェット未実装 | Req 6 全AC | 中 |
| G5 | DirectWrite ルビAPI未ラップ | Req 7 全AC | 高 |
| G6 | テキスト座標→文字位置の逆変換API未ラップ | Req 8 AC5 | 中 |
| G7 | キーボード入力フレームワーク未実装（ESC以外） | Req 9 AC5 | 中 |
| G8 | ~~キーボードフォーカス管理ECS未実装~~ | ~~Req 10 AC5~~ | P0スコープ外 |
| G9 | ~~WM_CHAR / WM_IME のECS接続なし~~ | ~~Req 10 全AC~~ | P0スコープ外 |
| G10 | TypewriterToken にルビ・リンク variant なし | Req 7 AC5, Req 8 AC6 | 中 |

### 3.2 既存資産の活用可能ポイント

| 資産 | 活用先 | 備考 |
|------|--------|------|
| `Window` + `on_window_add` フック | Req 1: バルーンウィンドウ生成 | 既存パターンの拡張で対応 |
| `CompositionMode::ULW` | Req 1 AC3: 透過ウィンドウ | そのまま利用可能 |
| `WindowPos` + `SetWindowPosCommand` | Req 2: 位置制御, Req 3: 表示制御 | コマンドキューパターン再利用 |
| `Monitor.work_area` | Req 2 AC4: デスクトップ境界判定 | 値の取得は済み。判定ロジックのみ新規 |
| `TaffyStyle` / taffy | Req 4: コンテンツ領域レイアウト | margin/padding/max 制約そのまま利用 |
| `Typewriter` + `TypewriterTalk` | Req 5: テキスト表示 | ほぼそのまま利用可能 |
| `WheelDelta` | Req 6 AC3: ホイールスクロール | 値の取得は済み。消費ロジックのみ新規 |
| `Phase<T>` イベントシステム | Req 8, 9: イベント発火 | イベント型の追加のみ |
| `OnPointerEntered/Exited` | Req 8 AC4, Req 9 AC4: ホバー | ポインタイベントフック再利用 |
| `HitRegionMap` | Req 8: リンクヒットテスト | テキスト座標からの自動生成が新規 |
| `win_message_handler.rs` スタブ | ~~Req 10: テキスト入力~~ | P0スコープ外。将来の別仕様で対応 |

---

## 4. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**方針**: `Window` コンポーネントにバルーン固有フィールドを追加。Typewriter IR にルビ・リンク variant を直接追加。キーボードハンドラを `keyboard.rs` に追記。

- **対象ファイル**: `window/components.rs`, `typewriter_ir.rs`, `window_proc/keyboard.rs`
- **互換性**: `Window` 構造体が肥大化。既存のシェルウィンドウ生成に不要なフィールドが混入
- **メンテナンス性**: 単一責任原則に違反するリスク

**トレードオフ**:
- ✅ 新規ファイル最少、初期開発が速い
- ✅ 既存のフックパターンをそのまま利用
- ❌ `Window` コンポーネントの肥大化（シェル/バルーン/ダイアログが混在）
- ❌ バルーン固有ロジックが散在して保守困難

### Option B: 新規コンポーネント作成（推奨）

**方針**: バルーン専用コンポーネント群（`BalloonWindow`, `BalloonAnchor`, `BalloonPlacement` 等）を新規モジュールとして作成。スクロールコンテナ、選択肢UI、入力ボックスも独立ウィジェットとして実装。

- **新規モジュール構成**:
  - `ecs/widget/balloon/` — バルーンコア（ウィンドウ生成・配置・表示制御）
  - `ecs/widget/scroll.rs` — スクロールコンテナウィジェット
  - `ecs/widget/choice.rs` — 選択肢ウィジェット
  - `com/dwrite_ext.rs` — ルビ・テキストヒットテスト用 DirectWrite 拡張

- **統合ポイント**:
  - `BalloonWindow` の `on_add` フックで `Window` + `WindowStyle` を自動挿入（既存パターン踏襲）
  - `BalloonAnchor { target: Entity, direction, offset }` でシェル↔バルーン関係を ECS で表現
  - バルーン配置システムを `PreLayout` スケジュールに登録
  - 既存 `Phase<T>` イベントディスパッチに新イベント型を追加

- **責任境界**:
  - `BalloonWindow` = ライフサイクル管理のみ（生成・破棄）
  - `BalloonAnchor` = アンカー対象 + 配置パラメータ
  - `BalloonPlacement` = 計算結果（実配置方向・座標キャッシュ）
  - 配置システム = `BalloonAnchor` + `Monitor` → `WindowPos` 書き換え

**トレードオフ**:
- ✅ 明確な責任分離（ウィンドウ管理とバルーンロジックが独立）
- ✅ テスト容易性（バルーンコンポーネント単体でテスト可能）
- ✅ 子仕様ごとに独立した開発・テストが可能
- ❌ ファイル数増加（ナビゲーションコスト）
- ❌ インターフェース設計が必要

### Option C: ハイブリッドアプローチ

**方針**: balloon-core は新規コンポーネント（Option B）、balloon-content は既存 Typewriter の拡張（Option A 寄り）、balloon-rich-text / balloon-input は新規ウィジェット。

- **フェーズ分割**:
  1. balloon-core: 新規 `ecs/widget/balloon/` モジュール
  2. balloon-content: `Typewriter` の上に薄いラッパーコンポーネント + スクロールは新規
  3. balloon-rich-text: `TypewriterToken` に variant 追加 + `com/dwrite_ext.rs` 新規
  4. balloon-input: 完全新規ウィジェット

**トレードオフ**:
- ✅ 各層に最適なアプローチを選択
- ✅ balloon-content は Typewriter 統合が既存でほぼ動作するため最小限の追加で済む
- ❌ アプローチが混在して一貫性が低下するリスク
- ❌ TypewriterToken への variant 追加が Typewriter 仕様の責任範囲を曖昧にする

---

## 5. 子仕様別 工数・リスク評価

| 子仕様 | 工数 | リスク | 根拠 |
|--------|------|--------|------|
| **balloon-core** | M (3–7日) | Low | 既存 `Window` + `WindowPos` パターンの拡張。`on_add` フック、コマンドキューのパターンが確立済み。配置アルゴリズムは新規だが技術的に明確 |
| **balloon-content** | M (3–7日) | Low | Typewriter 統合は既にモックで実証済み。スクロールは新規だが `WheelDelta` と taffy の制約で段階的に実装可能 |
| **balloon-rich-text** | L (1–2週) | High | DirectWrite ルビ API は `IDWriteTextLayout1` 以降の COM インターフェースが必要。テキスト座標↔文字位置の正確な逆変換は技術的に複雑。縦書き+ルビの組合せは検証が必要 |
| **balloon-input** | S (1–3日) | Low | Req 10（入力ボックス）をP0スコープ外としたことで、選択肢UIのみ。既存ポインタイベント + flexbox レイアウトで対応可能。キーボード操作のスコープ次第でMに変動 |

### 全体工数: L (2–4週)

---

## 6. 設計フェーズへの申し送り事項

### 6.1 推奨アプローチ

**Option B（新規コンポーネント作成）を推奨**。理由：

1. 子仕様の段階的実装に最も適合（モジュール境界が子仕様境界と一致）
2. 既存 `Window` / `Typewriter` への侵入的変更を最小化
3. `on_add` フックによる自動セットアップパターンが既に確立されており踏襲可能
4. 各子仕様の独立テストが容易

### 6.2 設計フェーズでの決定事項

| # | 決定事項 | 関連要件 |
|---|---------|---------|
| D1 | `BalloonAnchor` の ECS 表現（Relation vs コンポーネント内 Entity 参照） | Req 1, 2 |
| D2 | 配置システムのスケジュール位置（PreLayout? Update?） | Req 2 |
| D3 | スクロールコンテナの描画方式（クリッピング vs オフスクリーン） | Req 6 |
| D4 | ルビの実装方式（DirectWrite ネイティブ vs 手動配置） | Req 7 |
| D5 | キーボードナビゲーションの実装方式（Req 9 AC5 のスコープ次第） | Req 9 |
| ~~D6~~ | ~~IME 統合の範囲~~ | ~~Req 10~~ P0スコープ外 |
| D7 | TypewriterToken 拡張の責任境界（本仕様 vs typewriter仕様の改訂） | Req 7 AC5, Req 8 AC6 |

### 6.3 リサーチ項目

| # | 項目 | 理由 | 優先度 |
|---|------|------|--------|
| R1 | `IDWriteTextLayout1::SetPairKerning` / ルビ用 DirectWrite API の可用性 | windows-rs クレートでの API 提供状況が不明 | 高 |
| R2 | `IDWriteTextLayout::HitTestPoint` の精度（縦書き時） | 縦書きテキストでの座標→文字位置変換の信頼性 | 高 |
| ~~R3~~ | ~~WM_IME_COMPOSITION → bevy_ecs 統合のアーキテクチャ~~ | Req 10 P0スコープ外 | — |
| R4 | taffy のスクロールコンテナサポート状況 | taffy 0.9 で overflow/scroll がどこまで使えるか | 中 |
| R5 | `bevy_ecs` 0.18 の Relation API 安定性 | BalloonAnchor にRelation が使えるか | 低 |
| R6 | ULW 合成モードでのクリッピング描画パフォーマンス | スクロール時の再描画コストの見積もり | 中 |

---

## 7. 非機能要件ギャップ

| NFR | 現状 | ギャップ |
|-----|------|---------|
| **NFR-1: パフォーマンス** | ULW 全面再描画。Typewriter は毎フレームではなく変更時のみ再描画 | スクロール時の 60fps 維持が ULW で可能か要検証 (R6) |
| **NFR-2: 互換性** | DPI 対応済み (`Monitor.dpi`)。Win10 1803+ ターゲット | DirectWrite ルビ API の Win10 1803 互換性を確認要 (R1) |
| **NFR-3: ECS統合** | 既存パターン確立済み | ✅ Option B で要件充足可能 |
