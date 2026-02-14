# Gap Analysis Report — docs-restructure

| 項目 | 内容 |
|------|------|
| **Feature** | docs-restructure |
| **Date** | 2026-02-14 |
| **Scope** | 憲法系ドキュメント・README・ロードマップの再構成 |

---

## 1. Current State Investigation

### 1.1 既存ドキュメント資産マップ

| 資産 | 場所 | 内容 | 鮮度 |
|------|------|------|------|
| README.md | `/README.md` | wintfフェーズリスト（モチベ維持用） | ⚠️ 古い（フェーズ2-6が未チェックのまま、実装は大幅に進行済み） |
| 設計仕様書12章 | `doc/spec/01-12` | bevy_ecs UIフレームワーク設計 | ✅ 2025-11 再編成済み |
| WinVisual.md | `doc/WinVisual.md` | DirectComposition統合設計 (1230行) | ✅ bevy_ecs版に更新済み |
| MIGRATION_SUMMARY.md | `doc/MIGRATION_SUMMARY.md` | slotmap→bevy_ecs移行記録 (356行) | ✅ 歴史的記録（完了） |
| REORGANIZATION_SUMMARY.md | `doc/REORGANIZATION_SUMMARY.md` | ドキュメント再編成記録 (118行) | ✅ 歴史的記録（完了） |
| rune-persistence-guide.md | `doc/rune-persistence-guide.md` | Rune永続化ガイド (335行) | ✅ script-engine実装ガイド |
| spec_backup_20251101 | `doc/spec_backup_20251101_082532/` | 再編成前バックアップ14ファイル | 🗄️ アーカイブ候補 |
| steering/product.md | `.kiro/steering/product.md` | wintfの製品概要 | ⚠️ arekaアプリ層未記載 |
| steering/structure.md | `.kiro/steering/structure.md` | プロジェクト構造定義 | ⚠️ dolaクレート未記載 |
| steering/tech.md | `.kiro/steering/tech.md` | 技術スタック | ⚠️ windows 0.62.1→0.62.2、Rust edition 2021→2024 の修正が必要（bevy_ecs 0.18.0/taffy 0.9.2 は正確） |
| steering/focus.md | `.kiro/steering/focus.md` | ロードマップ管理ポインタ | ⚠️ 旧ROADMAP参照 |
| ukagaka-desktop-mascot/ | `.kiro/specs/ukagaka-desktop-mascot/` | メタ仕様（全31要件、設計書1259行、タスク287行、リサーチ360行、ROADMAP） | ✅ Phase 1完了（子仕様原案32件作成） |
| shell/ 素材 | `shell/` | ぱすたさん画像素材（base + x5/y6/z2パーツ + 合成済み7枚 + icon） | ⚠️ メタデータファイルなし |

### 1.2 スペック進捗サマリー

| カテゴリ | 件数 | 内訳 |
|---------|------|------|
| 完了 (completed/) | 57件 | ECS基盤、レイアウト、グラフィックス、ポインター、ドラッグ、typewriter、image-widget、dola、script-engine等 |
| アクティブ P0 | ~12件 | event-system(進行中)、animation/balloon/window-placement/system-tray/persistence/package-manager/mcp-server等(要件ドラフト) |
| バックログ P1-P3 | ~13件 | legacy-converter, devtools, llm-integration, voice等 |

### 1.3 重要な発見事項

#### ✅ Resolved: pasta クレートは別リポジトリに分離済み

`areka-P0-script-engine` は spec.json で `phase: "complete"`, `implementation: 100%` と記録されており、これは正確。コード実装は別リポジトリに分離されている。

- **リポジトリ**: https://github.com/ekicyou/pasta
- **結論**: `crates/` 配下に不在なのは別リポジトリ化によるものであり、問題なし
- **ロードマップ記載**: 「✅ 完了（外部リポジトリ）」として記載可能

#### 🟡 Warning: shell/ メタデータ不在

shell/ ディレクトリにはPNG画像のみ存在し、descript.txt / manifest.toml / surfaces.txt 等のメタデータファイルがない。リファレンスシェル仕様の策定時に定義が必要。

#### 🟡 Warning: dola-animation-system の位置づけ

`dola` クレートは完了済み（`crates/dola/` にコード実在）だが、ROADMAP.md には `wintf-P0-animation-system` として「要件生成済み・未承認」のまま。dola → wintf 統合の区別が不明瞭。

---

## 2. Requirements Feasibility Analysis

### Requirement-to-Asset Map

| Req# | 要件 | 既存資産 | ギャップ | タグ |
|------|------|---------|---------|------|
| **1** | README.md 再構成 | `README.md`（古いフェーズリスト） | 全面書き換え必要。テンプレート不在 | **Missing** |
| **2** | CONSTITUTION.md 策定 | steering/, ukagaka-desktop-mascot/requirements.md 責務境界表、research.md 設計決定 | 新規ファイル。素材は分散して存在 | **Missing** (素材あり) |
| **3** | ROADMAP.md 再構成 | ukagaka-desktop-mascot/ROADMAP.md (Mermaid図あり)、tasks.md | 新規ファイル。旧ROADMAPから大幅再構成 | **Missing** (素材あり) |
| **4** | ARCHITECTURE.md 整備 | steering/structure.md、doc/spec/README.md | 新規ファイル。structure.md が原典として機能 | **Missing** (原典あり) |
| **5** | steering 整合性更新 | steering/ 7ファイル | product.md/structure.md/tech.md/focus.md の更新。形式は既存を踏襲 | **Constraint** (既存形式維持) |
| **6** | 旧ドキュメントアーカイブ | doc/spec_backup_20251101, README.md, ROADMAP.md | `doc/archive/` 新規作成、git mv 操作 | **Missing** (単純操作) |
| **7** | ぱすたさんプロファイル | shell/ 素材、script-engine 設計 (pasta DSL)、rune-persistence-guide.md | 新規ファイル。素材・DSL設計は存在 | **Missing** (素材あり) |

### 技術的ニーズ

| カテゴリ | 内容 |
|---------|------|
| データモデル | なし（全てMarkdownドキュメント操作） |
| API/サービス | なし |
| UI/コンポーネント | なし |
| ビジネスルール | ドキュメント間の参照整合性、二重管理回避 |
| 非機能要件 | git履歴の追跡可能性 |

### 複雑性シグナル

- **本質**: ドキュメント作成・編集作業（コード変更なし）
- **主な難しさ**: 分散した情報の正確なサルベージと一貫した再構成
- **リスク**: 情報の漏れ・矛盾、旧ドキュメントとの不整合

---

## 3. Implementation Approach Options

### Option A: 一括書き換え（Big Bang）

**概要**: 全7要件を1フェーズで一気に実施

| 観点 | 評価 |
|------|------|
| 作業順序 | Req6(アーカイブ) → Req1(README) → Req2(憲法) → Req3(ロードマップ) → Req4(アーキテクチャ) → Req5(steering) → Req7(ぱすた) |
| ファイル操作 | git mv × 3-4件、新規作成 × 5件、更新 × 4件 |
| コミット | アーカイブ1回 + 新規ドキュメント1回 + steering更新1回 = 3コミット |

**Trade-offs**:
- ✅ 1セッションで全完了、一貫性が高い
- ✅ コミット数が少なく綺麗
- ❌ レビュー負荷が大きい（差分が巨大）
- ❌ 途中で方針転換すると手戻りが大きい

### Option B: 段階的実施（Incremental）

**概要**: 3フェーズに分割

| フェーズ | 内容 | コミット |
|---------|------|---------|
| B-1 | Req6(アーカイブ) + Req1(README) | アーカイブ移動 + 新README |
| B-2 | Req2(憲法) + Req3(ロードマップ) + Req4(アーキテクチャ) | 3ドキュメント新規作成 |
| B-3 | Req5(steering更新) + Req7(ぱすたプロファイル) | 更新 + 新規 |

**Trade-offs**:
- ✅ フェーズごとにレビュー・修正可能
- ✅ B-1完了時点でREADMEが即座に改善
- ❌ フェーズ間の整合性管理が必要
- ❌ 3回のレビューサイクル

### Option C: README優先 + 残り一括（Hybrid） **← 推奨**

**概要**: 最も影響の大きいREADMEを先行、残りをまとめて実施

| フェーズ | 内容 | 理由 |
|---------|------|------|
| C-1 | Req6(アーカイブ) + Req1(README) + Req7(ぱすたプロファイル) | プロジェクトの「顔」を即座に整備。ぱすたさん情報はREADMEに含む必要があるため同時実施 |
| C-2 | Req2(憲法) + Req3(ロードマップ) + Req4(アーキテクチャ) + Req5(steering) | 内部設計文書群を一括整備 |

**Trade-offs**:
- ✅ 最も価値の高い改善（README）が最速で反映
- ✅ 2回のみのレビューサイクルで管理可能
- ✅ フェーズ1完了時点で外部公開可能な状態になる
- ❌ フェーズ2がやや重い

---

## 4. Effort & Risk

| 項目 | 評価 | 根拠 |
|------|------|------|
| **Effort** | **M (3-7日)** | 新規Markdownドキュメント5件作成 + 4件更新 + git操作。コード変更なしだが情報整理の精度が求められる |
| **Risk** | **Low** | 既存パターン（Markdown + git）の範囲内。アーキテクチャ変更なし。失敗時もgit revertで完全復元可能 |

### リスク詳細

| リスク | 影響 | 対策 |
|--------|------|------|
| ~~script-engine 完了状態の誤記載~~ | ~~ROADMAP/READMEの信頼性低下~~ | ✅ 解決済み: 別リポジトリ (ekicyou/pasta) に分離済み |
| 情報の散逸・矛盾 | ドキュメント間の不整合 | 相互参照リンクの徹底、原典主義（steering = 信頼源） |
| ぱすたさんプロファイルの情報不足 | リファレンス実装の要件が不明確 | 最小限プロファイル + 「未定」マークで暫定公開 |

---

## 5. Research Needed（設計フェーズへの申し送り）

| # | 項目 | 理由 | 影響する要件 | 状態 |
|---|------|------|-------------|------|
| R-1 | `areka-P0-script-engine` の実装状態確認 | 別リポジトリ (https://github.com/ekicyou/pasta) に分離済み。ロードマップでは「完了（外部リポ）」と記載 | Req 3 (ROADMAP) | ✅ 解決済 |
| R-2 | pasta DSL の最新文法仕様 | ぱすたさんプロファイルに会話サンプルを記載するにあたり、DSL構文の確定が必要。pasta リポジトリを参照 | Req 7 (ぱすたプロフィール) | ℹ️ 設計フェーズ |
| R-3 | shell/ 素材のパーツ合成座標 | base.png に対する x/y/z パーツのオーバーレイ座標が未定義。PASTA_PROFILE.md に記載すべきか | Req 7 | ℹ️ 設計フェーズ |
| R-4 | ukagaka-desktop-mascot ROADMAP.md の廃止手順 | 旧ROADMAPを参照している他のスペック（focus.md等）への影響 | Req 3, Req 5 | ℹ️ 設計フェーズ |

---

## 6. Recommendations for Design Phase

### 推奨アプローチ: **Option C（Hybrid）**

1. **Phase C-1**（README + アーカイブ + ぱすたプロファイル）を先行実施
   - プロジェクトの外部向け「顔」を最速で整備
   - shell/ 素材の存在をREADMEに反映し、プロジェクトの具体性を示す

2. **Phase C-2**（憲法 + ロードマップ + アーキテクチャ + steering）を一括実施
   - 内部設計文書の整合性を一気に回復
   - steering 更新は最後に行い、他ドキュメントとの整合を確保

### Key Decisions for Design Phase

| # | 決定事項 | 選択肢 |
|---|---------|--------|
| D-1 | ~~script-engine のロードマップ上の扱い~~ | ✅ 解決済み: 「完了（外部リポジトリ ekicyou/pasta）」と記載 |
| D-2 | 旧ROADMAP.md の処理 | (a) doc/archive/ へ移動 (b) ROADMAP.md 内に折りたたみで保持 |
| D-3 | ドキュメント言語 | (a) 全て日本語 (b) README英語 + 他は日本語 |
| D-4 | doc/spec/ 12章の扱い | (a) 現状維持（ARCHITECTURE.mdから参照） (b) 統合 |
