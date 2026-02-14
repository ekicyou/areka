# Research & Design Decisions — docs-restructure

## Summary
- **Feature**: `docs-restructure`
- **Discovery Scope**: Simple Addition（ドキュメントのみ、コード変更なし）
- **Key Findings**:
  - 現行 README.md は wintf 単体の古いロードマップであり、areka プロジェクト全体の顔になっていない
  - completed/ に 56 件の仕様が存在し、基盤レイヤーの成熟度を示す客観的指標がある
  - pasta スクリプトエンジンは別リポジトリ (ekicyou/pasta) に分離済み、spec.json の完了記録は正確

## Research Log

### ドキュメント資産の現状調査
- **Context**: 新ドキュメント群の設計にあたり、既存資産と情報源を特定
- **Sources Consulted**: プロジェクト全体のファイル構造、steering/ 7ファイル、doc/ 配下、spec 56件+19件アクティブ+18件バックログ
- **Findings**:
  - README.md: wintf フェーズリスト（Phase 1 のみ✅、2-6 未完了だが実際は大幅に進行済み）
  - doc/spec/ 12章: ECS UIフレームワーク設計仕様書（2025-11 再編成済み、現状維持の方針決定済み）
  - doc/WinVisual.md: DirectComposition統合設計（1230行、bevy_ecs版更新済み）
  - doc/MIGRATION_SUMMARY.md, REORGANIZATION_SUMMARY.md: 歴史的記録（完了）
  - doc/spec_backup_20251101_082532/: 再編成前バックアップ14ファイル（アーカイブ候補）
  - steering/: product.md に areka アプリ層未記載、structure.md に dola 未記載、tech.md のバージョン一部修正済み
- **Implications**: 情報素材は十分に存在するが分散。新ドキュメント群は既存素材の再構成が主作業

### 責務境界テーブルの確認
- **Context**: CONSTITUTION.md (Req 2) に記載する責務分離表の原典確認
- **Sources Consulted**: `.kiro/specs/ukagaka-desktop-mascot/requirements.md`
- **Findings**:
  - プラットフォーム / ゴースト / シェル / バルーンの4層責務が明確に定義済み
  - プラットフォーム = 描画+スクリプト実行+イベント配信
  - ゴースト = 会話+人格+記憶+LLM
  - シェル = 外観素材提供
  - バルーン = 会話UIスタイル提供
- **Implications**: CONSTITUTION.md は原典を引用・再構成する形で責務分離表を記載

### ROADMAP 素材の確認
- **Context**: 新 ROADMAP.md (Req 3) の素材として旧 ROADMAP を確認
- **Sources Consulted**: `.kiro/specs/ukagaka-desktop-mascot/ROADMAP.md`
- **Findings**:
  - 32件の子仕様、Tier 0-3 の依存階層、Mermaid図あり
  - Progress: 5 completed / 1 in-progress / 26 not-started（2025-12-10時点、古い）
  - 新 ROADMAP はアルファリリースに焦点を絞り、全 P0 仕様を網羅する必要あり
- **Implications**: 旧ROADMAPのMermaid図構造は参考になるが、フェーズ構成は刷新が必要

### クレート構成の確定
- **Context**: README/ARCHITECTURE で記載するクレート構成の正確な把握
- **Sources Consulted**: Cargo.toml, crates/ ディレクトリ、開発者確認
- **Findings**:
  - `crates/wintf` — Windows縦書きUIフレームワーク（ライブラリ）
  - `crates/dola` — Declarative Orchestration for Live Animation（ライブラリ）
  - `crates/areka` — 未作成、独立バイナリクレートとして作成予定
  - `examples/areka.rs` — ダミー、削除予定
  - 外部: `ekicyou/pasta` — pasta DSLスクリプトエンジン（別リポジトリ）
- **Implications**: 4クレート構成（3内部 + 1外部）として各ドキュメントに記載

### ドキュメント言語方針の確定
- **Context**: 全ドキュメントの記述言語
- **Sources Consulted**: 開発者確認（議題4）
- **Findings**: 全て日本語で統一（伺か文化圏・国内コミュニティ向け）
- **Implications**: README.md 含め全ファイル日本語

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Option A: 一括書き換え | 全8要件を1フェーズで実施 | 一貫性、少ないコミット | レビュー負荷大、方針転換困難 | — |
| Option B: 段階的実施 | 3フェーズに分割 | フェーズ毎にレビュー可能 | 3回のレビューサイクル | — |
| **Option C: Hybrid** | README優先+残り一括 | 最速で「顔」整備、2回のレビュー | Phase 2がやや重い | **採用** |

## Design Decisions

### Decision: 実施フェーズ構成
- **Context**: 8要件を効率的かつレビュー可能に実施する順序
- **Alternatives Considered**:
  1. Option A — 全要件一括
  2. Option B — 3フェーズ分割
  3. Option C — README優先 + 残り一括
- **Selected Approach**: Option C（Hybrid）
- **Rationale**: プロジェクトの「顔」(README) を最速で整備しつつ、内部文書は整合性を確保して一括実施
- **Trade-offs**: Phase 2 がやや重いが、内部文書の相互参照整合性が高くなる
- **Follow-up**: Phase 1 完了後に Phase 2 のスコープを再確認

### Decision: doc/spec/ 12章の扱い
- **Context**: ARCHITECTURE.md 新設時の既存設計仕様書の位置づけ
- **Selected Approach**: 現状維持 + ARCHITECTURE.md から参照リンク
- **Rationale**: 12章は wintf の詳細設計資産として価値が高く、移動・統合は不要。ARCHITECTURE.md は俯瞰図に徹する

### Decision: 二重管理回避の原則
- **Context**: 複数ドキュメント間の情報重複防止
- **Selected Approach**: 原典主義（Single Source of Truth）
- **Rationale**: steering/ を技術的真実の源泉とし、他ドキュメントは参照リンクで指す
- **Trade-offs**: ドキュメント閲覧時にクロスリファレンスが必要になるが、整合性は高い

## Risks & Mitigations
- 情報の散逸・矛盾 → 相互参照リンクの徹底、原典主義（steering = 信頼源）
- ぱすたさんプロファイルの情報不足 → 最小限プロファイル + 「未定」マークで暫定記載
- 旧ROADMAP参照の断絶 → focus.md を新ROADMAP に切り替え、旧ROADMAP はアーカイブ

## References
- [areka プロジェクト](https://github.com/ekicyou/areka) — メインリポジトリ
- [pasta DSL エンジン](https://github.com/ekicyou/pasta) — 外部スクリプトエンジン
- [ukagaka-desktop-mascot 要件書](.kiro/specs/ukagaka-desktop-mascot/requirements.md) — 責務境界テーブル原典
- [ukagaka-desktop-mascot ROADMAP](.kiro/specs/ukagaka-desktop-mascot/ROADMAP.md) — 旧ロードマップ（アーカイブ予定）
