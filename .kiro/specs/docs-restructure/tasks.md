# Implementation Plan — docs-restructure

| 項目 | 内容 |
|------|------|
| **Feature** | 憲法系ドキュメント及びREADME.md再構成 — タスク |
| **Version** | 1.0 |
| **Date** | 2026-02-14 |

---

## Phase C-1: プロジェクトの顔（外部向けドキュメント）

- [x] 1. 旧ドキュメントアーカイブ移動
- [x] 1.1 `doc/archive/` ディレクトリ作成と旧ファイル移動
  - `doc/archive/` ディレクトリを作成（既存の場合はそのまま使用）
  - `README.md` を `doc/DEVLOG_ORIGINAL_README.md` に git mv でリネーム保存
  - `.kiro/specs/ukagaka-desktop-mascot/ROADMAP.md` を `doc/archive/ROADMAP_ukagaka_meta.md` に git mv で移動
  - `doc/spec_backup_20251101_082532/` を `doc/archive/spec_backup_20251101_082532/` に git mv で移動（存在確認後）
  - git commit でアーカイブ移動を記録（"docs(archive): 旧ドキュメントを doc/archive/ に移動" 等）
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 2. Completed specs スキャンと機能リスト中間報告
- [x] 2.1 Completed specs スキャンスクリプト実行と中間報告ファイル生成
  - `.kiro/specs/completed/` ディレクトリ内の spec.json ファイルをスキャン
  - 各 spec.json から `feature_name` と `title` 情報を抽出
  - ECS, DirectComposition, 縦書き, 透過ウィンドウ, レイアウト, ポインターイベント等のカテゴリ別に分類
  - 中間報告ファイル `.kiro/specs/docs-restructure/completed_specs_scan_report.md` を出力
  - スキャン結果サマリー（56件完了、カテゴリ別機能リスト）を記載
  - _Requirements: 1.6_

- [x] 3. README.md 作成
- [x] 3.1 プロジェクトREADME.md の執筆
  - 新規 `README.md` を作成し、以下12セクションを記述：
    1. ヘッダー・バッジ（プロジェクト名、キャッチコピー）
    2. プロジェクト概要（areka の使命、伺かインスパイア）
    3. スクリーンショット/デモ（プレースホルダー: `shell/icon.png` 使用）
    4. 二層構造説明（wintf = UIフレームワーク / areka = マスコットアプリバイナリ）
    5. クレート構成（wintf, dola, areka（予定）+ 外部: pasta）
    6. 技術スタック（Rust 2024, bevy_ecs 0.18, windows 0.62.2, DirectComposition, Taffy 0.9）
    7. アルファリリース目標（ぱすたさん紹介、PASTA_PROFILE.md へリンク）
    8. 現在の到達点（completed_specs_scan_report.md から機能リストを転記、基盤レイヤー ~70% 完了）
    9. 開発ロードマップ概要（Phase A-E サマリー + ROADMAP.md へリンク）
    10. ビルド手順（cargo build, cargo run, cargo test）
    11. ドキュメントガイド（doc/, steering/ へのリンク集）
    12. ライセンス（MIT OR Apache-2.0）
  - 日本語で洗練された表現を使用
  - git commit でREADME.md 作成を記録
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6_

- [x] 4. PASTA_PROFILE.md 作成
- [x] 4.1 (P) ぱすたさんプロファイル文書の執筆
  - `doc/PASTA_PROFILE.md` を新規作成し、以下7セクションを記述：
    1. キャラクター概要（名前「ぱすたさん」、設定1段落）
    2. シェル構成（サーフェス一覧: base.png（xyz.png, 320×420px）+ x1-x5（口5パターン）+ y1-y6（目6パターン）+ z1-z2（眉2パターン）= 60表情）
    3. バルーン種別（縦書きタイプライター付き吹き出し）
    4. ゴースト種別（pasta DSL: 里々インスパイアのカスタムDSL）
    5. pasta DSL 概要（設計思想、基本構文紹介1セクション）
    6. 会話サンプル（最小限のスクリプト例）
    7. 外部参照（ekicyou/pasta リポジトリへのリンク）
  - シェル構成セクションに座標情報を完全記載（ekicyou/pasta の index.html から確定済み）
  - areka-P0-reference-ghost / shell / balloon spec の要件定義インプットとして機能する位置づけを明記
  - git commit で PASTA_PROFILE.md 作成を記録
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 5. クレートレベル README 作成
- [x] 5.1 (P) wintf クレート README 執筆
  - `crates/wintf/README.md` を新規作成し、以下6セクションを記述：
    1. クレート概要（Windows縦書きUIフレームワーク）
    2. 主要機能一覧（ECS統合, DirectComposition, 縦書き, 透過ウィンドウ, レイアウト, ポインターイベント）
    3. アーキテクチャ概要（COM → ECS → Message Handling 3層構造の簡潔な説明）
    4. サンプル実行方法（cargo run --example taffy_flex_demo 等）
    5. モジュール一覧（ecs/ 配下の各モジュール1行説明）
    6. 詳細設計参照（doc/spec/ 12章へのリンク）
  - プロジェクトルート README.md との重複を最小限に（クレート固有情報に焦点）
  - _Requirements: 8.1, 8.2, 8.5_

- [x] 5.2 (P) dola クレート README 執筆
  - `crates/dola/README.md` を新規作成し、以下5セクションを記述：
    1. クレート概要（Declarative Orchestration for Live Animation）
    2. 対応フォーマット（JSON（デフォルト）, TOML, YAML（feature flags））
    3. 基本的な使用例（Storyboard/Transition の構成例）
    4. API概要（主要な型: Storyboard, Transition, Easing 等の一覧）
    5. feature flags（json, toml, yaml の説明）
  - _Requirements: 8.3, 8.4, 8.5_

- [x] 5.3 Phase C-1 git commit とレビュー準備
  - 5.1 / 5.2 の変更を git commit（"docs(crates): wintf/dola README.md 作成"）
  - Phase C-1 完了を確認：README.md, PASTA_PROFILE.md, crates/wintf/README.md, crates/dola/README.md がすべて作成済み
  - Phase C-2 進行前の中間レビューポイント
  - _Requirements: 8.6_

---

## Phase C-2: 内部設計文書（開発者向けドキュメント）

- [x] 6. CONSTITUTION.md 作成
- [x] 6.1 (P) プロジェクト憲法文書の執筆
  - `doc/CONSTITUTION.md` を新規作成し、以下6セクションを記述：
    1. ミッション宣言（areka プロジェクトの存在理由とビジョン）
    2. 設計理念（5原則以内: ECS駆動, 責務分離, 段階的拡張, 日本語ファースト, AI協調開発）
    3. 責務境界（プラットフォーム憲法: プラットフォーム/ゴースト/シェル/バルーンの責務分離表、ukagaka-desktop-mascot requirements.md から引用・再構成）
    4. クレート構成と責務（wintf, dola, areka, pasta の責務定義）
    5. 技術的意思決定記録（ECS+DirectComposition選定理由, MCP採用方針）
    6. スコープ外事項（アクセシビリティ強制、コンテンツ国際化等の明示的除外）
  - 既存ドキュメント（steering/, research.md, design.md）と重複する内容は参照リンクで原典を指示（二重管理回避）
  - MCP = Model Context Protocol（AI連携プロトコル）であることを明記
  - git commit で CONSTITUTION.md 作成を記録
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 7. ROADMAP.md 作成
- [x] 7.1 (P) ロードマップ文書の執筆
  - `doc/ROADMAP.md` を新規作成し、以下8セクションを記述：
    1. プログレスサマリー（Phase A-E の完了率バー、56件完了/N件残の数値）
    2. Phase A: 基盤完成（event-system 残件, animation-system (dola→wintf統合)）
    3. Phase B: 表示層（balloon-system, window-placement）
    4. Phase C: コンテンツ（reference-shell, reference-balloon, reference-ghost (ぱすたさん)）
    5. Phase D: アプリ統合（areka バイナリクレート作成, system-tray, persistence, package構造）
    6. Phase E: アルファ出荷（統合テスト, README最終化, リリースビルド）
    7. 依存関係図（Mermaid記法のクリティカルパス図: EVT→BLN, ANIM→SHELL, etc.）
    8. 子仕様対応表（各フェーズに対応する .kiro/specs/ ディレクトリ名）
  - `.kiro/specs/` および `.kiro/specs/completed/` をスキャンし、アクティブ仕様とバックログ仕様を列挙
  - 旧 ROADMAP.md（ukagaka-desktop-mascot）が doc/archive/ に移動済みであることを前提に作成
  - pasta スクリプトエンジンは「✅ 完了（外部リポジトリ: ekicyou/pasta）」と記載
  - git commit で ROADMAP.md 作成を記録
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7_

- [x] 8. ARCHITECTURE.md 作成
- [x] 8.1 (P) アーキテクチャ概要文書の執筆
  - `doc/ARCHITECTURE.md` を新規作成し、以下8セクションを記述：
    1. クレート依存関係図（wintf, dola, areka, pasta のMermaid図）
    2. wintf 3層構造（COM → ECS → Message Handling）
    3. ECSモジュール一覧（window, graphics, layout, widget, pointer, drag 等の1-2行説明、steering/structure.md から参照）
    4. dola の責務（宣言的アニメーション定義フォーマット）
    5. areka アプリケーション層（バイナリクレートの責務概要（予定））
    6. pasta 外部連携（DSLスクリプトエンジンの役割と連携方式）
    7. 推奨読書順序（新規開発者向けのファイル順序ガイド: steering/product.md → ARCHITECTURE.md → doc/spec/README.md → 各章）
    8. 詳細設計参照（doc/spec/ 12章へのリンク集）
  - steering/structure.md を原典として参照し、差分のみを記載（二重管理回避）
  - ARCHITECTURE.md は「俯瞰図 + 推奨読書順序」に特化
  - git commit で ARCHITECTURE.md 作成を記録
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 9. steering/ 整合性更新
- [x] 9.1 steering/product.md 更新
  - areka アプリケーション層の記述を追加
  - アルファリリースターゲット（ぱすたさん）を追加
  - 既存の wintf 記述は保持、areka アプリ層セクションを追記
  - 既存のスタイル（セクション構成、記述の詳細度、フォーマット）を厳密に維持
  - _Requirements: 5.1_

- [x] 9.2 steering/structure.md 更新
  - dola クレートの記述を追加（serde + feature flags）
  - areka バイナリクレート（予定）の記述を追加
  - pasta 外部リポジトリの言及を追加
  - 既存の wintf 構造記述は保持、dola/areka セクションを追記
  - 既存のスタイルを厳密に維持
  - _Requirements: 5.2_

- [x] 9.3 steering/tech.md 更新
  - dola クレートの記述を追加（serde + feature flags: json, toml, yaml）
  - 既存のスタイルを厳密に維持
  - _Requirements: 5.3_

- [x] 9.4 steering/focus.md 更新
  - ロードマップ参照先を `.kiro/specs/ukagaka-desktop-mascot/ROADMAP.md` から `doc/ROADMAP.md` に切り替え
  - 最小限の変更（参照先 URL のみ変更）
  - _Requirements: 5.4_

- [x] 9.5 steering/ 更新の git commit
  - 9.1～9.4 の変更を git commit（"docs(steering): areka/dola 追加、ROADMAP 参照先更新"）
  - Phase C-2 完了を確認
  - _Requirements: 5.5_

---

## 検証・最終化

- [x] 10. ドキュメント品質検証
- [x] 10.1 全ドキュメント品質チェック
  - リンク検証: 全ドキュメント内の相対リンクが正しいパスを指していることを手動確認
  - セクション完全性: requirements.md の AC に対し、各ドキュメントの対応セクションが存在することを確認
  - 原典整合性: steering/ の記載内容と各ドキュメントの引用が矛盾しないことを確認
  - Mermaid構文検証: ROADMAP.md, ARCHITECTURE.md の Mermaid 図がレンダリング可能であることを確認
  - スタイル統一性: 全ドキュメントが日本語で記述され、フォーマルな技術文書のトーンにあっているか確認
  - 検証結果を記録し、不整合があれば修正
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 7.4, 7.5, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

---

## タスク概要

- **合計**: 10 メジャータスク、15 サブタスク
- **Phase C-1**: タスク 1～5（アーカイブ、完了スペックスキャン、README、PASTA_PROFILE、クレートREADME）
- **Phase C-2**: タスク 6～9（CONSTITUTION、ROADMAP、ARCHITECTURE、steering 更新）
- **検証**: タスク 10（品質検証）
- **並行実行可能**: タスク 4.1, 5.1, 5.2（Phase C-1）、タスク 6.1, 7.1, 8.1（Phase C-2）に `(P)` マーク付与
- **全 8 要件カバー済み**: Req 1～8 すべてのタスクに対応

---

## 実施ガイド

### Phase C-1 実施順序
1. タスク 1.1: アーカイブ移動（必須最初）
2. タスク 2.1: Completed specs スキャン
3. タスク 3.1: README.md 作成（2.1 の出力使用）
4. タスク 4.1, 5.1, 5.2: 並行実行可能
5. タスク 5.3: Phase C-1 レビュー

### Phase C-2 実施順序
1. タスク 6.1, 7.1, 8.1: 並行実行可能
2. タスク 9.1～9.5: steering/ 更新（順次実行）

### 最終検証
- タスク 10.1: 全ドキュメント品質チェック

### 注意事項
- Phase C-1 → Phase C-2 の順序を守る（focus.md の一時的不整合を許容）
- completed_specs_scan_report.md は中間成果物としてセッション継続性を維持
- すべての git commit は日本語のコミットメッセージで記録
