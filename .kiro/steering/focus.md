---
inclusion: always
updated_at: 2026-07-02
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
- 棚卸しの基準実数（2026-06-28・clean slate 後）: 完了99（歴史） / **active = 0**（憶測仕様を全伐採・実装ファーストで着手時に作る）。backlog・`_rejected/`・旧戦略メモは削除（git 履歴に保全）。ロードマップは **M1 のみ**（M2+ は M1 完成後に再構築）
- 棚卸しの基準実数（2026-07-01 更新）: 完了 **103**（歴史） / **spec.json 有りの active = 0**（不変） / **直下の brief-only（spec.json 無し＝Phase 0 構想）= 7**（`/kiro-discovery` で just-in-time 生成した**着手可能フロント**: wintf 基盤層 `wintf-dcomp-to-wuc-migration`・`wintf-clickthrough-alpha-toggle`・`wintf-ulw-removal`／M1 parser `areka-P0-shell-parse`・`areka-P0-parser-foundation`（旧 balloon-parse・2026-07-02 開発リジェクト→共通基盤へリネーム・brief-only へ復帰）・`areka-P0-package-mount`／M1 host-32 `areka-P0-host32-ipc`）。`/kiro-start <name>` で本坑ライフサイクル入り＝その時点で spec.json が生えて active へ遷移。件数は配置フォルダ＋spec.json 有無基準（brief-only は Phase 0 別掲）
- 棚卸しの基準実数（2026-07-03 更新）: 完了 **112**（`areka-P0-host32-ipc`／`-parser-foundation`／`-shell-parse`／`-balloon-parse`／`-package-mount`／`-sakura-parse`／`-host32-shiori-load`・`wintf-dcomp-to-wuc-migration`・`wintf-clickthrough-alpha-toggle` 等） / **active = 0** / **brief-only = 7**（07-03 深掘り discovery で生成: `areka-P0-host32-request`・**emo 直列3分割** `areka-P0-emo-atlas`→`-emo-compose`→`-emo-present`（旧 emo-surface を粒度分割・αトリミングアトラス・packing クレート `rectangle-pack` 要承認）・`areka-P0-window-placement`・**`areka-P0-actor-foundation`**（通信横断基盤・機構/経路(kanade)/結線(ghost)の三分・kanade 先行依存）＋既存更新 `wintf-ulw-removal`）。**M1 M-boot 進捗 約 7/19**（②parsers 全完了・①shiori: pilot✅/ipc✅/shiori-load✅）。**07-05 是正: 即並走可能フロントは4本**（actor-foundation／host32-request／emo-atlas／ulw-removal）→ emo チェーン直列 → **window-placement は emo-present ゲート下**（demo 前提着手が実 DPI 座標破綻でリジェクト→本番ゴースト先行の原則・brief 改稿済み。正本 roadmap.md ポートフォリオ節）

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
