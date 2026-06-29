# 設計バリデーションレポート — kiro-P0-two-tunnel-model

| 項目 | 内容 |
|------|------|
| **Feature** | kiro-P0-two-tunnel-model |
| **Phase** | design-validation（非対話実行） |
| **Language** | ja |
| **Date** | 2026-06-28 |
| **入力** | spec.json, requirements.md, design.md, research.md, .kiro/steering/* |
| **判定** | **GO** |

---

## レビューサマリ

本設計はプロセス支援仕様として高品質で、実装着手可能な水準にある。8 要件＋5 NFR の全 ID が Requirements Traceability 表に網羅され、二坑モデルを「steering 規律層 ＋ pilot 検疫所 ＋ 機械ゲート」の三層へ明確に分離し、各層を既存の確立済みパターン（focus→roadmap 二層・`shiori-abi` 葉ノード・`taffy_flex_demo` の examples 構成）に接地させている。要件ディスカッションで確定済みの 2 判断（R4＝ローカル DoD ゲート／R7＝手動チェックリスト）も完全に honor されており、CI 新設・自動化ツールは Non-Goals / Out of Boundary で明示的に排除されている。

---

## 検証済み事項

### (a) 事前確定 2 判断の honor 状況 — 両方とも完全充足

- **R4＝隔離ゲートはローカル DoD ゲート（GitHub Actions ではない）**:
  Key Decisions 表「隔離ゲートの乗り物（R4）＝ローカル workflow 完了ゲート（`/kiro-complete` DoD）」、Non-Goals「リモート CI（GitHub Actions 等）の新設」、Out of Boundary「`.github/` ディレクトリは本仕様では作成しない」で一貫して排除。隔離ゲート実行フロー（System Flows）も `/kiro-complete` DoD ゲート内に閉じており、要件 4-5 と整合。
- **R7＝依存マップは手動チェックリスト（自動ツールなし）**:
  Non-Goals「依存マップ検証（R7）の自動チェックツール化（手動チェックリスト規律として確立。自動化は本仕様の契約外）」、Components の two-tunnel.md「依存マップ重点検証（手動チェックリスト）」節（被覆/孤児/DAG/合否基準/不適合時 not-ready）、Error Handling「依存マップ検証不適合（運用エラー）…人間が分解時に適用」で一貫。要件 7-1〜7-6 を漏れなく文書規律に落としている。

### (b) 一つの未決設計選択（隔離ガード手段：cargo metadata vs cargo-deny）— 根拠は健全

設計は **cargo metadata 自前走査を採用し cargo-deny を不採用**とした。根拠は Key Decisions・research §9.2（Build vs Adopt）に明記され、(1) cargo-deny `bans` は outbound 表現が主で「特定クレートへの inbound（被依存）禁止」の表現可否が PoC 依存・不確実、(2) CI/ローカル両環境への別途インストールが二坑モデルの戒める依存負債になる、(3) metadata の resolve グラフ走査は追加依存ゼロで inbound-edge 不変条件を直接表現できる、という 3 点で論理的に一貫している。inbound/outbound 区別（pilot が他に依存＝許容、他が pilot に依存＝禁止）の判定ロジックも `resolve.nodes[].deps` 走査として Implementation Notes に具体化されており、Testing Strategy にも区別の正当性検証が含まれる。この未決選択は健全に決着している。

### (c) 要件カバレッジ（8 要件＋5 NFR）— 完全

Requirements Traceability 表で 1.1–8.6 の全 AC ＋ NFR-1〜NFR-5 が Components/Interfaces/Flows に対応付け済み。Components and Interfaces 表でも各コンポーネントの Req Coverage が示され、orphan component（要件に紐づかない部品）は存在しない。要件側の付録 C で design 送りとされた 3 論点（ガード手段・worktree ライフサイクル・テンプレ形）も research §9.3 で全て確定済み。

### (d) worktree submodule-init 制約の反映 — 明確に反映

`[patch.crates-io] pasta_core = vendors/pasta/...` が worktree で未populate となり cargo 系が全滅する制約（research §3・既知メモリと一致）が、Architecture「回避する技術的制約」、check-isolation.ps1 の手順 1「`git submodule update --init --recursive`（無条件・前段）」、隔離ゲートフロー、Testing Strategy「submodule 前段が機能する」回帰、要件 4-6 と複数箇所で一貫して反映されている。実コード（ルート Cargo.toml の patch 行・members ワイルドカード・taffy_flex_demo の examples 構成・shiori-abi の publish=false）も本レビューで実地確認し、設計前提と齟齬なし。

---

## クリティカルイシュー

**該当なし（critical issue ゼロ）。**

実装着手を妨げる構造的不整合・要件ギャップ・過大複雑性は検出されなかった。以下は GO を妨げない軽微な観察（design 改稿不要・タスク段階で吸収可能）：

- `check-isolation.ps1` の配置（`scripts/` 新設 vs `crates/pilot/` 同梱）は設計自身が「タスク段階の微調整事項」と明記しており、決定論的に呼べるパスであれば問題ない。
- PowerShell スクリプトを「OS 非依存ロジック（metadata 走査）」と位置づけるが、ホストは PowerShell 固定。リモート CI 移設（要件 4-7）時にロジックは再利用可だがホスト言語は書き換えになる点は設計も Implementation Notes で認識済みであり、本仕様スコープ外として許容範囲。

---

## 設計の強み

1. **既存パターンへの徹底的な接地と境界規律**: 三層すべて（steering 二層・葉ノード publish=false・examples サブフォルダ）が実在の前例に紐づき、本レビューで実コードと照合して齟齬ゼロを確認。二坑規律の正本を two-tunnel.md 一葉に集約し workflow.md は参照に留める「No Hidden Shared Ownership」設計は、要件 1-5（コンテキスト消費抑制）と要件 8（always 常駐コスト）の協調を既存 focus→roadmap 二層で自然に解いている。
2. **命綱（inbound-edge ゼロ）の機械執行が単一所有者に集約**: 唯一の機械執行不変条件を check-isolation.ps1 に一元化し、inbound/outbound の区別・submodule 前段・終了コード契約・腐敗検出までを単一スクリプトの責務として明確に定義。Testing Strategy が「一時的に依存を加えて fail を確認」という具体的検証手順まで示しており、規律の形骸化を構造的に防いでいる。

---

## 最終判定

- **判定: GO**
- **根拠**: 8 要件＋5 NFR を完全網羅し、事前確定 2 判断（R4 ローカル DoD ／ R7 手動チェックリスト）を完全に honor、未決の 1 設計選択（cargo metadata 採用）も健全な根拠で決着、worktree submodule-init 制約も複数箇所で一貫反映。実装着手を妨げる critical issue は存在しない。
- **次ステップ**: `/kiro-spec-tasks kiro-P0-two-tunnel-model` で実装タスクを生成する。
