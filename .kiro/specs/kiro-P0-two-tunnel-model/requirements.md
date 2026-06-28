# Requirements Document

| 項目 | 内容 |
|------|------|
| **Feature** | kiro-P0-two-tunnel-model |
| **Type** | プロセス支援仕様（前例: `kiro-P0-roadmap-management`） |
| **Language** | ja |
| **Date** | 2026-06-28 |

---

## Introduction

本仕様は、エージェント駆動開発における**誤った方向の可逆性（reversibility）**を最優先する開発規律「**二坑モデル**」を、ルール（steering 文書）＋インフラ（知見クレート `crates/pilot`・CI 依存方向ガード・workflow ゲート）として確立する。

### 背景

エージェント駆動開発の開発者は、次の本質的痛点を抱える。

1. **見てから違う**: 出てきた物を見て初めて方向の誤りに気づく（事前に完全には言語化できない反応型）。
2. **コード肥大でやり直し地獄**: 誤りに気づいた時には実装が育って絡み合い、手戻りが高くつく。
3. **エージェントは消さず足す**: コードを削除せず追加し続け、負の遺産（負債）が累積する。

現状の kiro spec ライフサイクルは「違う」が**実装後（コード肥大後）**に発覚する経路を構造的に塞いでおらず、探索的コードと production コードの境界規律も未整備である。最適化すべきは「手応えの速さ」ではなく「誤りの可逆性」であり、それを支える規律とインフラが要る。

### 二坑モデルの要旨

- **先進坑（pilot・使い捨て）**: 手順の正当性・方向性・実現可能性を確認する。捨てる前提。成果＝知見（go/違う/直す＋学び）。細粒度・独立ゆえ多重並列で掘れる。
- **本坑（main・完成品）**: 既存 kiro spec ライフサイクル。直列・慎重・PR マージ。
- **命綱（不変条件）**: 出荷グラフ上のいかなるクレートも先進坑コードに依存しない（葉ノード隔離）。
- **ハードゲート**: 各本坑 spec は方向を確定する先進坑の go 判定を前提依存に持ち、go まで本坑は着手不能（BLOCKED）。

### 本仕様の位置づけ

本仕様はプロセス支援仕様であり、技術的な機能実装ではなく、開発プロセスとエージェント連携の仕組みを定義する。成果物は steering 文書・`crates/pilot` クレートと規約・CI 設定・workflow 統合の形で提供される。

---

## Boundary Context

- **In scope**:
  - 二坑モデルの steering 文書化（`.kiro/steering/` に正規文書として確立）。
  - 知見クレート `crates/pilot` の新設（Cargo.toml・workspace 統合・`examples/<spec>/` 規約・テンプレ example＋README 雛形）。
  - CI 統合（知見クレートの `cargo build --examples` ＋ production→pilot 依存禁止の機械チェック）。
  - workflow.md への二坑統合（先進坑フェーズ・go ハードゲート・依存マップ重点検証ルール・削除/隔離規律）。
- **Out of scope**:
  - 既存ロードマップの二坑分解（M1 を pilot/main へ割り直す作業）。本モデル確立後の後続 discovery（別作業・本仕様ではない）。
  - 個別の先進坑/本坑 spec の実装そのもの（本仕様はモデルとインフラの確立まで）。
  - 並列実行基盤（workflow/agent fan-out）の新規開発（既存の Agent/Workflow 機構を運用で使う）。
  - production クレート（wintf/dola/areka/shiori-abi 等）への機能追加。
- **Adjacent expectations**:
  - `.kiro/steering/workflow.md` を拡張する（二坑ゲート・依存マップ検証を上乗せ）。本仕様の規律は既存ブランチ/完了規約と整合し、これを置き換えない。
  - `.kiro/steering/roadmap.md` は spec に pilot/main 種別と go ゲート依存を表現できるよう拡張余地を持つ。
  - `kiro-P0-roadmap-management`（completed）は改変せず整合のみ。完了仕様 `completed/` は不変として尊重する。
  - karpathy-guidelines スキル（add-only 肥大の抑制）と思想が一致し援用する。

---

## Requirements

### Requirement 1: 二坑モデルの steering 文書化

**Objective:** 開発者/AI として、二坑モデルの規律（先進坑/本坑の役割・命綱・ゲート・隔離方針）を正本として参照したい。それにより以降の全 spec が一貫した可逆性優先の規律で進められる。

#### Acceptance Criteria

1. The Two-Tunnel Steering **shall** 先進坑（使い捨て検証）と本坑（完成品）の定義と役割分担を `.kiro/steering/` 配下の文書に記載する。
2. The Two-Tunnel Steering **shall** 「最適化対象は手応えの速さでなく誤った方向の可逆性である」という方針を明文化する。
3. The Two-Tunnel Steering **shall** 何を先進坑にするか（方向・実現可能性・手順が怪しい所だけ。よく分かっている所は直に本坑）の判断基準を記載する。
4. The Two-Tunnel Steering **shall** 命綱（葉ノード隔離の不変条件）・ハードゲート（go 判定前提依存）・依存マップ検証・削除/隔離規律の各規律へ到達できる参照を含む。
5. Where steering 文書が AI に常時読み込まれる構成を採る場合, the Two-Tunnel Steering **shall** 詳細手順を別文書へ委譲しコンテキスト消費を抑える形式とする。

---

### Requirement 2: 知見クレート `crates/pilot` の確立

**Objective:** 開発者/AI として、先進坑コードと先進坑記録を集約する専用の場所を持ちたい。それにより探索的残骸を一箇所に検疫し production を常時クリーンに保てる。

#### Acceptance Criteria

1. The Pilot Crate **shall** `crates/pilot` として新設され、ワークスペースのメンバーに統合される。
2. The Pilot Crate **shall** `publish = false` として設定され、配布対象から除外される。
3. The Pilot Crate **shall** 出荷グラフ上の葉ノード（他のどのクレートからも依存されない）として位置づけられる。
4. The Pilot Crate **shall** 先進坑コードを `examples/<spec-name>/` の単位（1 仕様 = 1 フォルダ）で格納し、各フォルダに `main.rs` と `README.md` を持つ規約を定める。
5. While 複数の先進坑が並列に進行している場合, the Pilot Crate の規約 **shall** 1 仕様 = 1 フォルダ構成により相互の merge 衝突が発生しない構造を保証する。
6. The Pilot Crate **shall** 新しい先進坑が即座に着手できるテンプレート example（雛形 `main.rs` と README 雛形）を提供する。
7. The Pilot Crate **shall** 最小依存を保ち、ワークスペースの 32bit 可搬性を崩さない。

---

### Requirement 3: 先進坑の一次記録（README 3 幕）と traceability

**Objective:** 開発者/AI として、先進坑で得た知見を構造化された一次記録として残したい。それにより本坑 design が知見を参照でき、検証結果を二重化せずに済む。

#### Acceptance Criteria

1. The Pilot README **shall** 各 `examples/<spec-name>/README.md` を当該先進坑の一次記録（正本）として位置づける。
2. The Pilot README **shall** 「動機（なぜ掘るか・対応する本坑 spec の名指し）→ 概要（何を作ったか・実行法）→ 検証結果（go/違う/直す ＋ 学び ＋ 日付）」の 3 幕構成で記述する規約を定める。
3. The Pilot README **shall** 対応する本坑 spec を名指しすることで先進坑↔本坑の traceability を確立する。
4. The Pilot README **shall** 各先進坑の実行法（例: `cargo run -p pilot --example <spec>`）を記載する。
5. When 本坑 spec の design が先進坑の知見を参照する場合, the 本坑 design **shall** README の検証結果を参照し、同じ検証結果を重複して記述しない。
6. Where subagent が `.md` を直接書けないハーネス制約がある場合, the Pilot README の運用 **shall** 代替手段（親による書き込み等）で README を確実に生成できる手順を示す。

---

### Requirement 4: CI 強制（examples ビルド ＋ 依存方向ガード）

**Objective:** 開発者/AI として、二坑モデルの不変条件を人手でなく機械で守りたい。それにより規律が形骸化せず確実に維持される。

#### Acceptance Criteria

1. The CI Pipeline **shall** 知見クレートの先進坑コードを `cargo build --examples` 相当でビルドし、腐敗（ビルド破綻）を検出する。
2. The CI Pipeline **shall** production クレートが知見クレート（先進坑コード）に依存していないことを機械的に検証する。
3. If production クレートから知見クレートへの依存が検出された場合, then the CI Pipeline **shall** 当該変更を失敗として扱う。
4. If 知見クレートの先進坑コードがビルドに失敗した場合, then the CI Pipeline **shall** 当該変更を失敗として扱う。
5. The CI Pipeline **shall** これらのチェックを既存の CI ワークフローに統合し、変更時に自動で実行する。

---

### Requirement 5: 命綱（葉ノード隔離）と削除/隔離規律

**Objective:** 開発者/AI として、先進坑コードを「捨てやすい」状態に保ちつつ知見を失わずに済ませたい。それにより負の遺産を累積させずに探索を続けられる。

#### Acceptance Criteria

1. The Two-Tunnel Discipline **shall** 「出荷グラフ上のいかなるクレートも先進坑コードに依存しない（葉ノード隔離）」を不変条件として明記する。
2. While 葉ノード隔離の不変条件が満たされている場合, the Two-Tunnel Discipline **shall** 先進坑コードを物理削除する代わりに知見クレートへ隔離保全することを許可する。
3. The Two-Tunnel Discipline **shall** 本坑が先進坑の知見を「見てクリーンに掘り直す」ことを規定し、先進坑コードをコピペ donor として流用することを禁止する。
4. The Two-Tunnel Discipline **shall** 先進坑コードの品質基準は使い捨て前提で緩くてよいが、葉ノード隔離だけは機械（CI）で厳守する旨を明記する。
5. The Two-Tunnel Discipline **shall** 検疫所効果（探索的残骸を知見クレート一葉へ集約し production を常時クリーンに保ち、定期剪定を可能にする）を明文化する。

---

### Requirement 6: ハードゲート（go 判定の前提依存）

**Objective:** 開発者/AI として、方向が未確定なまま本坑を掘ることを構造的に不可能にしたい。それにより「見てから違う」の発覚を実装前へ前倒しできる。

#### Acceptance Criteria

1. The Hard Gate **shall** 各本坑 spec が方向を確定する先進坑の go 判定を前提依存として持つことを規定する。
2. While 前提の先進坑が go 判定に至っていない場合, the Hard Gate **shall** 当該本坑 spec を着手不能（BLOCKED）として扱う規律を定める。
3. The Hard Gate **shall** go 判定を開発者が出力を見て下す（人間判断）ものとし、自動判定にしないことを規定する。
4. The Hard Gate **shall** go 判定の前提依存を spec 上で表現する記法（例: `_Depends(confirmed): pilot`）を定める。
5. Where 本坑 spec の方向・実現可能性・手順が十分に確実な場合, the Hard Gate **shall** 先進坑を経ず直接本坑に着手することを許容する（掘りすぎ防止）。

---

### Requirement 7: 先進坑⟷本坑 依存マップの重点検証

**Objective:** 開発者/AI として、分解時に先進坑と本坑の依存関係が健全であることを厳密に検証したい。それにより不確実な本坑が go ゲートを持たずに進む事態を防げる。

#### Acceptance Criteria

1. The Dependency Map Validation **shall** 被覆（不確実な本坑は必ず対応する go ゲートを持つ）を検証する。
2. The Dependency Map Validation **shall** 孤児なし（どの先進坑・本坑も依存関係上で孤立しない）を検証する。
3. The Dependency Map Validation **shall** 循環なし（依存グラフが DAG である）を検証する。
4. The Dependency Map Validation **shall** 各エッジに合否基準（go/違う/直す を判定する基準）が明示されていることを検証する。
5. If 依存マップ検証を通過しない場合, then the Dependency Map Validation **shall** 当該本坑 spec を ready にしない。

---

### Requirement 8: workflow への二坑統合

**Objective:** 開発者/AI として、二坑モデルを既存の開発ワークフローへ組み込みたい。それにより先進坑フェーズとゲートが日常の spec 駆動に自然に乗る。

#### Acceptance Criteria

1. The Workflow Integration **shall** `.kiro/steering/workflow.md` に先進坑フェーズを追加し、既存フェーズフロー（requirements→design→tasks→implementation→complete）との関係を示す。
2. The Workflow Integration **shall** go ハードゲートを workflow に組み込み、本坑着手の前提条件として位置づける。
3. The Workflow Integration **shall** 依存マップ重点検証ルールを workflow に組み込む。
4. The Workflow Integration **shall** 削除/隔離規律（葉ノード隔離・隔離保全・掘り直し禁止）を workflow に組み込む。
5. The Workflow Integration **shall** 既存のブランチ＆マージ戦略（PR ベース・main 直 push 禁止）と完了手順を変更せず、それらと整合する形で二坑規律を上乗せする。
6. The Workflow Integration **shall** 先進坑が細粒度・独立ゆえ多重並列で掘れることを運用記述として含め、新規の並列実行基盤を開発せず既存の Agent/Workflow 機構を用いる前提を示す。

---

## Non-Functional Requirements

### NFR-1: 軽量性・既存ワークフロー互換

The Two-Tunnel Model **shall** 既存の kiro-style ワークフロー（spec ライフサイクル・steering・PR ベースマージ）を活用し、それらを置き換えずに上乗せする形で成立する。

### NFR-2: 葉ノード可搬性

The Pilot Crate **shall** 最小依存・`publish = false`・葉ノードを維持し、ワークスペースの 32bit 可搬性とビルド健全性を損なわない。

### NFR-3: 機械的厳守

The Two-Tunnel Model **shall** 命綱（葉ノード隔離）を人手の規律でなく CI による機械チェックで厳守する。

### NFR-4: 完了仕様の不変尊重

The Two-Tunnel Model **shall** `completed/` 配下の完了仕様（`kiro-P0-roadmap-management` 等）を改変せず、整合のみを保つ。

### NFR-5: テキストベース・Git 追跡可能

The Two-Tunnel Model の成果物（steering 文書・README 一次記録・規約）**shall** テキストベースで保存し、Git 履歴で変更を追跡可能にする。

---

## Appendix

### A. 想定される成果物

| 成果物 | 形式 | 配置場所 |
|--------|------|----------|
| 二坑モデル steering 文書 | Markdown | `.kiro/steering/` |
| workflow 二坑統合 | Markdown（既存拡張） | `.kiro/steering/workflow.md` |
| 知見クレート | Rust crate | `crates/pilot/`（`Cargo.toml`, `examples/<spec>/{main.rs, README.md}`, テンプレ example） |
| CI 依存方向ガード ＋ examples ビルド | CI 設定 | 既存 CI ワークフロー |

### B. 用語と境界連続性

- discovery = `Boundary Candidates`（brief.md）
- requirements = 本書の `Boundary Context`（In/Out/Adjacent）
- design = `Boundary Commitments`（後続）
- tasks = `_Boundary:_`（後続）

### C. 設計フェーズへ委譲する論点（要件外）

以下は user-observable behavior ではなく実装方針の選択であり、design フェーズで詰める（要件のスコープ曖昧性ではない）。

- CI 依存方向ガードの具体実装（`cargo metadata` 解析 / `cargo-deny` 等の選定）。
- pilot worktree のライフサイクル（いつ捨てるか）の運用詳細。
- テンプレ example の具体的な形（雛形コードと README 雛形の詳細）。

### D. 関連仕様・参照

- **前例**: `completed/kiro-P0-roadmap-management`（プロセス支援仕様の型・focus.md/roadmap.md 運用と整合）。
- **拡張対象**: `.kiro/steering/workflow.md`, `.kiro/steering/roadmap.md`。
- **思想援用**: karpathy-guidelines スキル（add-only 肥大の抑制）。
- **本仕様の一次記録**: `.kiro/specs/kiro-P0-two-tunnel-model/brief.md`（discovery アーク・確定設計判断・継続用セッション記憶）。
