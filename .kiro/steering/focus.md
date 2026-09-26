---
inclusion: always
updated_at: 2026-09-24
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
| 待機（α 後など・当面着手しない） | `.kiro/specs/` 直下に brief のまま置き、段は `roadmap.md` の台帳で持つ（`backlog/` は 2026-09 時点で使っていない） |
| 完了 | `.kiro/specs/completed/` |
| 却下 | `.kiro/specs/_rejected/` |

## 件数集計ルール

進捗件数は **`spec.json` の `phase` 値ではなく、配置フォルダを基準**に数える（`phase` 値は履歴上ズレるため当てにしない）。

| 配置 | 計上区分 |
| ------ | ------ |
| `.kiro/specs/` 直下（completed/backlog/_rejected 以外） | アクティブ（P0） |
| `.kiro/specs/completed/` | 完了 |
| `.kiro/specs/_rejected/` | 却下（集計対象外・参考） |
| `spec.json` を持たないディレクトリ（例: `shape-*`） | 構想段階（Phase 0）として別掲 |

- 直下に `phase=completed` のまま残る仕様（例: 旧メタ仕様）があれば `completed/` への移動候補として棚卸しに挙げる
- **過去の棚卸しの基準実数（2026-06-28〜2026-07-29 の 26 行）は `roadmap-history.md` の末尾「旧・focus.md 棚卸しの基準実数ログ」へ退避した**（2026-09-20）。本ファイルには**最新の 1 行だけ**を置き、更新するときは古い行をそちらへ足してから置き換える（常時読み込みのファイルを履歴で太らせない）。
- **spec は名前で呼ぶ**（2026-09-26 開発者指示）: 報告・brief・コミット・PR で spec を指すときは spec 名を書き、台帳の番号（「#数字」）は使わない。台帳の番号の列は `roadmap.md` から外した。「#数字」は `PR#185` のように接頭辞を付けた PR 番号にだけ使う。
- 棚卸しの基準実数（2026-09-26 更新㉖・棚卸⑰の実測・`main` `13b72893` 基準）: 完了 **201**（`completed/` 直下の実測エントリ数＝ディレクトリ 200＋`graphics-rendering-stability.md` 1）/ **spec.json 有りの active = 0** / **brief-only = 29**（`.kiro/specs/` 直下実測＝更新㉕の 29 − 09-24〜09-26 完了 3〔`shiori-fault-notice`・`ghost-restart-unit`・`pilot-balloon-asset-swap`〕＋ 棚卸⑰の起票 3〔`ghost-change-name-resolution`・`pilot-dropfiles-on-wuc-window`・`alpha-package`〕）。open PR 0 本＝着手中の spec は 0。M1 は 2026-09-11 に完成宣言済み・現行は α（M2・`roadmap.md` B3〜B7）。**着手の優先度は「バグ修正 → α に要る機能」**（2026-09-20 開発者指示・`roadmap.md`「棚卸⑮の裁定」）。棚卸の経緯の正本は `roadmap.md`／`roadmap-history.md`。

## 運用上の注意

- `.kiro/specs/`直下には、進行中の仕様ディレクトリだけでなく、調査メモや戦略文書が単体Markdownとして置かれることがある
- ROADMAPと進捗集計の対象は、原則として`spec.json`を持つ仕様ディレクトリ
- `completed/`配下には履歴上の古いphase値を含む仕様が残るため、集計時は配置場所を優先して判断する

## 参照先

📍 `.kiro/steering/roadmap.md` … **ロードマップ正本**（kiro 標準テンプレート・`inclusion: manual` で非常駐。`/kiro-discovery` 再入・`/kiro-spec-batch` が標準パスで参照）
📍 `.kiro/specs/*/spec.json` … 各仕様の phase/approvals（件数は配置フォルダ基準で数える）
📍 `doc/COMPAT_ARCHITECTURE.md` … 設計判断の正本
📍 `doc/ROADMAP.md` … 旧パスのポインタ stub（正本は steering/roadmap.md）
📍 `.kiro/steering/two-tunnel.md` … 二坑モデル規律の正本（`inclusion: manual` で非常駐）
📍 roadmap.md「エンジン固有名」節 … 7エンジン⓪〜⑥の固有名正本（**ghost / shiori / parsers / kanade / sakura / seriko / emo**・2026-07-02 確定）
