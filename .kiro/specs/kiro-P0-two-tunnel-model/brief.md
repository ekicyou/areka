# Brief: kiro-P0-two-tunnel-model

## Problem
エージェント駆動開発で開発者が抱える本質的な痛点：
- **「見てから違う」が多い**: 出てきた物を見て初めて方向の誤りに気づく（事前に完全には言語化できない反応型）。
- **コードが膨らむとやり直しが地獄**: 誤りに気づいた時には実装が育って絡み合い、手戻りが高くつく。
- **エージェントは消さず足すばかり**: コードを削除せず追加し続ける傾向で、負の遺産（負債）の累積が洒落にならない。

最適化すべきは「手応えの速さ」ではなく **誤った方向の可逆性（reversibility）**。「どうせ途中で『違う』が出る」を前提に、①発覚を安く ②間違ったコードを捨てやすく する開発規律と、それを支えるインフラが要る。

## Current State
- kiro spec ライフサイクル（requirements→design→tasks→impl→complete）は存在するが、「違う」が**実装後（コード肥大後）**に発覚する経路を構造的に塞いでいない。
- 探索的コードと production コードの境界規律が未整備で、使い捨て前提の検証コードが production へ流用・累積するリスクがある（負の遺産）。
- 前例: `kiro-P0-roadmap-management`（completed・プロセス支援仕様）が steering 文書を成果物として確立済み。本仕様も同型のプロセス支援仕様。
- 本モデルは本仕様着手前の discovery 議論＋scratchpad 叩き台（`two-tunnel-DRAFT.md`）で方向確定済み（＝本モデル自身の「先進坑」が go 判定済み）。

## Desired Outcome
**二坑モデル**（先進坑＝使い捨て検証 / 本坑＝完成品）を、ルール（steering）＋インフラ（知見クレート・CI・workflow ゲート）として確立し、以降の全 spec が「誤りの可逆性を最優先」する規律の下で進む。

## Approach: 二坑モデル
- **二つの坑**:
  - **先進坑（pilot・使い捨て）**: 手順の正当性・方向性・実現可能性を確認。捨てる前提。成果＝知見（go/違う/直す＋学び）。throwaway worktree＋scratchpad。**細粒度・独立ゆえ多重並列（Max~20x）でドコドコ掘る**。
  - **本坑（main・完成品）**: 既存 kiro spec ライフサイクル。直列・慎重・PR マージ。
- **命綱（不変条件）**: *出荷グラフ上のいかなるクレートも先進坑コードに依存しない（葉ノード隔離）*。これを満たせば先進坑コードは削除でなく**知見クレートへ隔離保全**してよい。本坑は知見を**見てクリーンに掘り直す**（コピペ donor にしない）。
- **ハードゲート（方向未確定では本坑を掘れない）**: 各本坑 spec は方向を確定する先進坑の **go 判定を前提依存**に持つ（`_Depends(confirmed): pilot`）。go まで本坑は着手不能（BLOCKED）。「未確定で進む」が構造的に不可能。
- **先進坑⟷本坑 依存マップ（分解時の重点検証対象）**: 被覆（不確実な本坑は必ず go ゲートを持つ）・孤児なし・循環なし（DAG）・各エッジに合否基準明示。このマップ検証を通らない限り本坑 spec を ready にしない。
- **知見クレート `crates/pilot`**（確定）: `publish=false`・**葉ノード**。`examples/<spec-name>/{README.md, main.rs}`（1 仕様=1 フォルダ＝20x 並列でも merge 衝突ゼロ）。README は3幕〈**動機**(なぜ掘る・対応 spec 名指し)→**概要**(何を作った・実行法)→**検証結果**(go/違う/直す＋学び＋日付)〉で先進坑の**一次記録＝正本**。CI で `cargo build --examples`（腐敗検出）。
- **CI 強制（確定）**: production が知見クレートに依存しないことを機械チェック（依存方向ガード）。
- **何を先進坑にするか**: 方向・実現可能性・手順が怪しい所だけ。よく分かっている所は直に本坑（掘りすぎ防止）。

## Scope
- **In**:
  - 二坑モデルの steering 文書化（`.kiro/steering/` に正規文書として確立）。
  - 知見クレート `crates/pilot` の新設（Cargo.toml・workspace 統合・`examples/<spec>/` 規約・テンプレ example＋README 雛形）。
  - CI 統合（知見クレートの `cargo build --examples`＋production→pilot 依存禁止の機械チェック）。
  - workflow.md への二坑統合（先進坑フェーズ・go ハードゲート・依存マップ重点検証ルール・削除/隔離規律）。
- **Out**:
  - 既存ロードマップの二坑分解（M1 を pilot/main へ割り直す作業）→ 本モデル確立後の **discovery Path D**（別作業・spec でない）。
  - 個別の先進坑/本坑 spec の実装そのもの（本仕様はモデルとインフラの確立まで）。
  - 並列実行基盤（workflow/agent fan-out）の新規開発（既存の Agent/Workflow 機構を運用で使う）。

## Boundary Candidates
- steering 文書（モデル規律）
- 知見クレート `crates/pilot`（コード・規約・テンプレート）
- CI 統合（examples ビルド＋依存方向ガード）
- workflow 統合（ゲート・依存マップ検証手順）

## Out of Boundary
- ロードマップ内容の二坑再分解（後続 discovery）。
- production クレート（wintf/dola/areka/shiori-abi 等）への機能追加。
- 既存 spec の実装。

## Upstream / Downstream
- **Upstream**: `kiro-P0-roadmap-management`（completed・プロセス支援仕様の前例。focus.md/roadmap.md 運用と整合）。`.kiro/steering/workflow.md`（ブランチ/完了規約・本仕様が二坑ゲートを上乗せ）。
- **Downstream**: 以降の全 spec（本モデルの先進坑/本坑規律の下で進む）。直近では M1 ロードマップ二坑分解、`areka-P0-shiori-reference` 等が最初の適用対象になり得る。

## Existing Spec Touchpoints
- **Extends**: `.kiro/steering/workflow.md`（二坑ゲート・依存マップ検証を追加）、`.kiro/steering/roadmap.md`（spec に pilot/main 種別と go ゲート依存を表現）。
- **Adjacent**: `kiro-P0-roadmap-management`（completed・改変しない・整合のみ）。karpathy-guidelines スキル（add-only 肥大の抑制と思想が一致・援用）。

## Constraints
- Rust 2024・既存ワークスペース。`crates/pilot` は `publish=false`・**葉ノード**（最小依存・32bit 可搬性を崩さない）。
- 知見クレートのコード品質は「雑でよい（使い捨て）」が、**葉ノード隔離**だけは機械（CI）で厳守。
- subagent は `.md` を Write/Edit 不可（ハーネス既知）。並列 pilot が README.md を書く運用は PowerShell here-string か親書き込み前提。
- 完了仕様 `completed/` は不変（改訂は継承記述で表現）。
