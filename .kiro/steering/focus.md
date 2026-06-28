---
inclusion: always
updated_at: 2026-06-26
---

# Focus - ロードマップ管理

arekaアルファリリースロードマップと`.kiro/specs/`配下の仕様ポートフォリオを整合させるための運用ガイド。

---

## ROADMAP 参照タイミング

- **セッション開始時**: 次に取り組む仕様を確認
- **仕様完了時**: 次の仕様を決定
- **仕様作成時**: 依存関係を確認
- **仕様の棚卸し時**: 直下・待機・完了・却下の各配置先が妥当か確認

## ROADMAP 更新タイミング

- **仕様フェーズ変更時**: phase列を更新
- **新規仕様作成時**: 実行計画に追加
- **仕様完了時**: 進捗サマリーを更新
- **仕様を却下した時**: ROADMAP対象外であることを確認し、`_rejected/`に隔離

## フォルダー配置ルール

| 状態 | 配置先 |
| ------ | ------ |
| アクティブ（P0） | `.kiro/specs/` 直下 |
| 待機（P1-P3） | `.kiro/specs/backlog/` |
| 完了 | `.kiro/specs/completed/` |
| 却下 | `.kiro/specs/_rejected/` |

## 件数集計ルール

進捗件数は **`spec.json` の `phase` 値ではなく、配置フォルダを基準**に数える（`phase` 値は履歴上ズレるため当てにしない）。

| 配置 | 計上区分 |
| ------ | ------ |
| `.kiro/specs/` 直下（completed/backlog/_rejected 以外） | アクティブ（P0） |
| `.kiro/specs/backlog/` | 待機（P1-P3） |
| `.kiro/specs/completed/` | 完了 |
| `.kiro/specs/_rejected/` | 却下（集計対象外・参考） |
| `spec.json` を持たないディレクトリ（例: `shape-*`） | 構想段階（Phase 0）として別掲 |

- 直下に `phase=completed` のまま残る仕様（例: 旧メタ仕様）があれば `completed/` への移動候補として棚卸しに挙げる
- 棚卸しの基準実数は roadmap.md（2026-06-28 時点）: 完了97 / アクティブP0 17（＋brief のみ構想9） / 待機21 / 却下3

## 運用上の注意

- `.kiro/specs/`直下には、進行中の仕様ディレクトリだけでなく、調査メモや戦略文書が単体Markdownとして置かれることがある
- ROADMAPと進捗集計の対象は、原則として`spec.json`を持つ仕様ディレクトリ
- `completed/`配下には履歴上の古いphase値を含む仕様が残るため、集計時は配置場所を優先して判断する

## 参照先

📍 `.kiro/steering/roadmap.md` … **ロードマップ正本**（kiro 標準テンプレート・`inclusion: manual` で非常駐。`/kiro-discovery` 再入・`/kiro-spec-batch` が標準パスで参照）
📍 `.kiro/specs/*/spec.json` … 各仕様の phase/approvals（件数は配置フォルダ基準で数える）
📍 `doc/COMPAT_ARCHITECTURE.md` … 設計判断の正本
📍 `doc/ROADMAP.md` … 旧パスのポインタ stub（正本は steering/roadmap.md）
