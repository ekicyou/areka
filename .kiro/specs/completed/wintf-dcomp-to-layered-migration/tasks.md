# 実装計画: wintf-dcomp-to-layered-migration

## タスク概要

本親仕様の実装フェーズでは、design.md の Migration Strategy に基づき4つの段階的子仕様を作成する。子仕様作成に先立ち、全子仕様が共通参照する統合指針文書を作成し、仕様間の一貫性・境界・前提条件を確保する。

---

## 実装タスク

- [x] 1. 子仕様統合指針文書の作成
  - 統合指針文書（migration-guide.md または integration-guide.md）を作成し、4つの子仕様が共通参照する実装指針・コンポーネント設計・システムフロー・移行戦略を明文化する
  - Phase 1-4 各子仕様の担当範囲・前提条件・完了基準（DoD）を詳細に定義する
  - DComp 廃止対象（RED）・書き換え対象（YELLOW）・再利用可能（GREEN）の3カテゴリ分類を整理する
  - 新パイプラインのアーキテクチャパターン（WindowD3D11Compositor, composite_render_system, ulw_present_system 等）を詳細化する
  - ハイブリッド段階アプローチ（Option C）の各段階での並行稼働方針・ロールバック戦略を確定する
  - D2D1 Bitmap Options, BLENDFUNCTION, DIBSection パラメータ等の技術リファレンスを整備する
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4, 2.5, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3, 8.4, 9.1, 9.2, 9.3, 9.4, 10.1, 10.2, 10.3_

- [x] 2. Phase 1 子仕様の作成
  - 統合指針文書を参照し、D2D1 合成スタック構築（新 GraphicsCore、合成ビットマップ、合成描画システム）の仕様を策定する
  - 仕様サイクル全工程を完了する（init → requirements → design → tasks）
  - 親仕様の設計に基づき、WindowD3D11Compositor コンポーネント、compositor_init_system, composite_render_system の詳細設計を子仕様に具体化する
  - CompositeContext による opacity 手動累積方式の仕様を確定する
  - 新パイプライン単体での描画検証基準（taffy_flex_demo 相当）を定義する
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 6.1, 10.1, 10.2_
  - _Dependencies: Task 1_

- [x] 3. Phase 2 子仕様の作成
  - 統合指針文書と Phase 1 子仕様を参照し、DComp パイプライン置換（ECS システム切り替え、スケジュール更新）の仕様を策定する
  - 仕様サイクル全工程を完了する（init → requirements → design → tasks）
  - 親仕様の設計に基づき、GraphicsCore からの DComp 初期化除去、world.rs のスケジュール切り替え、on_visual_add フック更新の詳細設計を子仕様に具体化する
  - DComp API 呼び出しゼロ検証基準（grep 検証、全 example 動作確認）を定義する
  - _Requirements: 2.3, 3.3, 5.1, 5.2, 5.3, 5.4, 6.2, 6.3, 10.1, 10.2_
  - _Dependencies: Task 1, Task 2_

- [x] 4. Phase 3 子仕様の作成
  - 統合指針文書と Phase 1-2 子仕様を参照し、UpdateLayeredWindow 統合（WS_EX_LAYERED、ULW 呼び出し、クリックスルー検証）の仕様を策定する
  - 仕様サイクル全工程を完了する（init → requirements → design → tasks）
  - 親仕様の設計に基づき、ulw_present_system 実装、WS_EX_LAYERED 切り替え、WM_PAINT/WM_SIZE ハンドラ更新の詳細設計を子仕様に具体化する
  - WS_EX_LAYERED 環境での WM_PAINT 発火動作検証要件を明確化する（research.md § Research Needed 参照）
  - alpha=0 クリックスルー検証基準を定義する
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 7.1, 7.2, 7.3, 10.1_
  - _Dependencies: Task 1, Task 2, Task 3_

- [x] 5. Phase 4 子仕様の作成
  - 統合指針文書と Phase 1-3 子仕様を参照し、切り替え式バックエンド実装（CompositionMode導入、ULW/DComp切り替え、DCompパイプライン復活登録）の仕様を策定する
  - 仕様サイクル全工程を完了する（init → requirements → design → tasks）
  - 親仕様の設計に基づき、CompositionMode enum導入、DCompシステムの条件付き復活登録、WS_EX_NOREDIRECTIONBITMAP / WS_EX_LAYERED動的切替の詳細設計を子仕様に具体化する
  - DCompバックエンドの動作検証基準（dcomp_demo.rs維持、DCompモードウィンドウでの描画確認）を定義する
  - 将来のWinRT Compositor拡張を見据えたenum設計方針を確定する
  - 最終検証基準（cargo test, cargo build --examples 全パス、ULW/DComp両モード動作確認）を定義する
  - _Requirements: 1.1, 2.5, 5.1, 10.1_
  - _Dependencies: Task 1, Task 2, Task 3, Task 4_

---

## 要件カバレッジサマリー

| 要件   | タスク        |
| ------ | ------------- |
| Req 1  | 1, 5          |
| Req 2  | 1, 3          |
| Req 3  | 1, 2          |
| Req 4  | 1, 4          |
| Req 5  | 1, 3, 5       |
| Req 6  | 1, 2, 3       |
| Req 7  | 1, 4          |
| Req 8  | 1, 5          |
| Req 9  | 1, 2, 3, 4, 5 |
| Req 10 | 1, 2, 3, 4, 5 |

全10要件がタスクにマッピング済み。
