# 設計バリデーション: areka-P0-app-shell

> **検証日**: 2026-07-05 / **言語**: ja / **対象**: `.kiro/specs/areka-P0-app-shell/design.md`
> 本書は要件（確定済み）・研究記録・ステアリングに対する設計品質レビュー（非対話）。判定は GO/NO-GO。

## レビューサマリ

設計は「アプリ起動の器」に責務を限定した最小骨格として要件（R1–R6）を漏れなく追跡しており、Requirements Traceability 表が全受入基準を Component/Flow へ写像している。中核判断 DD7（窓ゼロでは `WinApp::run()` に入らず `Ok(())` 復帰）は wintf 実装（`window_registry.rs:135` の `removed_any && registry.is_empty()` 空**遷移**ゲート、および `mod.rs:272` の `run()` が `block_on(shutdown_future)` で空遷移までブロックする事実）で裏付けられ、実測に一致する。DD1–DD7 は相互に整合し、モック UI の `examples/mock-shell.rs` 退避も DD6 解釈①（緑ゲート＝SHIORI e2e のみ）を保つ。実装準備は整っている。

## Critical Issues（最大 3）

### 🟡 Issue 1: `wire_engines` シームのシグネチャが未確定でコンパイル可能形が設計に無い
- **Concern**: `wire_engines(/* ghost-setup が確定 */)` は引数・戻り値を「ghost-setup が結線時に定める」とするが、本仕様で `main` が呼び出す以上、本仕様の実装時点で確定した具体シグネチャ（最小形＝引数なし・戻り値なしの `fn wire_engines()`）が必要。設計本文は「空実装の関数 1 個」（DD5）と述べる一方、Service Interface 節はプレースホルダのままで、実装者が確定形を判断する余地が残る。
- **Impact**: 中。実装は自明（`fn wire_engines() {}` を `main` が呼ぶ）だが、シグネチャ未確定はタスク化・レビュー時の解釈ぶれを生み、Revalidation Triggers（シームのシグネチャ変更は下流再点検）と噛み合わない。確定シグネチャが明記されれば下流 ghost-setup の起点も安定する。
- **Suggestion**: 本仕様での確定形を「`fn wire_engines()`（引数・戻り値なし・no-op）」と明示し、ghost-setup が中身と引数を後で拡張する旨を注記に留める（形の確定＝本仕様、内容＝下流）。
- **Traceability**: R4.2（空の接続点提供）。
- **Evidence**: design.md「`wire_engines`（空の接続点）」§ Service Interface / DD5。

### 🟡 Issue 2: R2.4（UI ランタイム起動）を `WinApp::new()` のみで満たす再定義の受入証跡が手動確認頼み
- **Concern**: DD7 は R2.4 の達成点を `run()` ではなく `WinApp::new()` の成功へ移す。これは wintf 実装上妥当（`new()` が COM/DPI 初期化・World 生成・shutdown hook 結線を担う）だが、骨格が `run()` に入らないため「UI ランタイムが起動した」ことを自動テストで固定する手段が設計に無く、E2E/Manual の目視（`cargo run -p areka` が構成ログ→正常終了）に依存する。R4.1 と R2.4 のどちらも黙って破らないことは論理的に示されているが、回帰ガードは薄い。
- **Impact**: 低〜中。`new()` 成功＝起動という再定義は要件文（「UI ランタイムを起動する」）に反しない範囲の解釈であり、ハング回避のため合理的。ただし将来 `run()` 誤呼び出し（Risks で言及）を検出する自動チェックが無いと、下流結線時に回帰が手動レビュー任せになる。
- **Suggestion**: Testing Strategy に「骨格 `main` 経路が `run()` を無条件に呼ばないこと」を担保する軽量チェック（例: `resolve_config_inputs` 単体テストに加え、`main` が窓ゼロで正常 return する制御を実装レビュー項目として明記、または smoke 実行を CI 手順化）を 1 行追加する。要件充足は既に成立しているため必須ではない。
- **Traceability**: R2.4（UI ランタイム起動）／R4.1（未結線で正常終了）。
- **Evidence**: design.md「UI ランタイムの終了規律」／System Flows「DD7 の制御分岐」／Components「骨格 `main`」Validation。

## Design Strengths

1. **中核判断の実測裏付けが厳密**: DD7 の根拠が wintf の実コード（空遷移ゲート `removed_any && is_empty()`・固定テスト `reconcile_removes_entries_and_fires_hook_only_on_empty_transition`）まで遡って検証され、「窓ゼロ `run()` はハングする」という非自明な落とし穴を制御分岐で正しく回避している。R2.4/R4.1 の両立が論理的に閉じている。
2. **境界純度と分離の実証**: モック UI／残置 SHIORI 資産／退避 example の三片分離が「相互参照ゼロ（実測）」で裏付けられ、退避がモック側のみで完結する。DD6 解釈①（緑ゲート＝SHIORI e2e）と `examples/mock-shell.rs` 同居テストの扱いが整合し、window-placement リジェクトの教訓（mock を本番コードへ持ち込まない）も守られている。

## 最終判定

- **Decision**: **GO**
- **Rationale**: DD1–DD7 は相互整合かつ要件追跡可能で、中核の DD7 制御分岐は wintf 実装で実測裏付け済み。指摘 2 件はいずれも実装準備を止めない軽微な明確化（シーム確定シグネチャの明記・回帰ガードの補足）であり、アーキテクチャ上の不整合や要件欠落は無い。
- **Next Steps**: `/kiro-spec-tasks areka-P0-app-shell` へ進む。設計ディスカッションで Issue 1（`wire_engines` 確定シグネチャ）を明記し、Issue 2（`run()` 非呼び出しの実装レビュー項目化）をタスクの検証欄へ織り込むと万全。
