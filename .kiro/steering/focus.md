---
inclusion: always
updated_at: 2026-07-09
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
- 棚卸しの基準実数（2026-07-05 更新・マージ後の実地確認＋全体精査）: 完了 **116**（`areka-P0-actor-foundation`・`wintf-ulw-removal`・`areka-P0-host32-request`・`areka-P0-emo-atlas` が新規完了アーカイブ） / **active = 0** / **brief-only = 7**（既存4: `areka-P0-app-shell`・`areka-P0-emo-compose`・`areka-P0-emo-present`・`areka-P0-window-placement`〔emo-present ゲート下〕＋新規3: **`areka-P0-host32-lifecycle`**〔①最終・死活語彙の正本〕・**`areka-P0-kanade`**〔運行表・talk 起動契約の正本・actor-foundation✅ で解禁〕・**`areka-P0-sakura-engine`**〔再生出力契約の正本・script 直入力で単体観測〕）。**M1 M-boot 進捗 約 10/20**。**即並走可能フロント5本**: app-shell／emo-compose／host32-lifecycle／kanade／sakura-engine（契約の正本連鎖で並走担保: lifecycle→kanade→sakura→seriko/emo-text-layer）。既存 brief の申し開きは実シンボル参照へ再調整済み（07-05）。正本 roadmap.md ポートフォリオ節＋所有マップ
- 棚卸しの基準実数（2026-07-05 更新②・`host32-lifecycle` 完了後）: 完了 **117**（`areka-P0-host32-lifecycle` 新規完了アーカイブ＝①shiori トラック全完了 pilot✅/ipc✅/shiori-load✅/request✅/lifecycle✅） / **active = 0** / **brief-only = 6**（`areka-P0-app-shell`・`areka-P0-emo-compose`・`areka-P0-emo-present`〔emo-compose ゲート下〕・`areka-P0-window-placement`〔emo-present ゲート下〕・`areka-P0-kanade`・`areka-P0-sakura-engine`）。**即並走可能フロント4本**: app-shell／emo-compose／kanade／sakura-engine（`host32-lifecycle` は消化・死活報告 API を kanade が正本消費）。正本 roadmap.md 追記④
- 棚卸しの基準実数（2026-07-05 更新③・`app-shell`＋`kanade` 完了後）: 完了 **119**（追記⑤ `areka-P0-app-shell`＝アプリ骨格・追記⑥ `areka-P0-kanade`＝runtime 制御階層③運行表/talk 起動契約の正本を新規完了アーカイブ） / **active = 0** / **brief-only = 4**（`areka-P0-emo-compose`・`areka-P0-emo-present`〔emo-compose ゲート下〕・`areka-P0-window-placement`〔emo-present ゲート下〕・`areka-P0-sakura-engine`）。**即並走可能フロント2本**: emo-compose／sakura-engine（`app-shell` は消化・`kanade` 完了で talk 起動契約 StartTalk/TalkDone が先決＝下流 sakura-engine 並走続行可）。正本 roadmap.md 追記⑤⑥
- 棚卸しの基準実数（2026-07-05 更新④・`sakura-engine`＋`emo-compose` 完了後）: 完了 **121**（追記⑦ `areka-P0-sakura-engine`＝runtime 制御階層④再生出力契約・追記⑧ `areka-P0-emo-compose`＝emo 直列2・合成コア〔fold→plan→blit→Composer facade・実 emo2 fixture golden・ログ発火/エラー写像も決定論的回帰檻化〕を新規完了アーカイブ） / **active = 0** / **brief-only = 2**（`areka-P0-emo-present`〔**emo-compose 完了でゲート解除＝新フロント**〕・`areka-P0-window-placement`〔emo-present ゲート下〕）。**即並走可能フロント1本**: emo-present（`emo-compose`・`sakura-engine` は消化・下流 emo-present が `ComposedSurface`／正規化 Surface 公開形を正本消費／sakura は再生出力契約を seriko・emo-text-layer へ先決）。正本 roadmap.md 追記⑦⑧
- 棚卸しの基準実数（2026-07-05 更新⑤・再入精査②＝runtime 最終フロント確立）: 完了 **121**（不変） / **active = 0** / **brief-only = 4**（`areka-P0-emo-present`〔実シンボル再調整済み〕・**`areka-P0-seriko-engine`**〔新規 brief・sakura `TalkCue`/`SurfaceSink` 正本✅＋emo-compose `AliasMap`/`BindSet` 正本✅で解禁〕・**`areka-P0-ghost-setup`**〔新規 brief・**talk 契約フォーク（kanade `quit:bool`／永続 channel vs sakura `TalkEndReason` 3値／per-talk spawn）の統一 WS-A＋結線の背骨 WS-B を所有**〕・`areka-P0-window-placement`〔emo-present ゲート下・再調整済み〕）。**M-boot 16/21（残5）**。**即並走可能フロント3本**: emo-present／seriko-engine／ghost-setup（非衝突: example＋emo 新層／新設 crates/areka-seriko／main.rs 結線＋talk 授受面。保護規約=ghost-setup WS-A は `TalkCue`/sink trait を凍結）。正本 roadmap.md 追記⑨
- 棚卸しの基準実数（2026-07-06 更新⑥・`seriko-engine` 完了後）: 完了 **122**（`areka-P0-seriko-engine`＝⑤ シェルアニメーションエンジンを新規完了アーカイブ。新設 crates/areka-seriko） / **active = 0** / **brief-only = 3**（`areka-P0-emo-present`・`areka-P0-ghost-setup`・`areka-P0-window-placement`〔emo-present ゲート下〕）。正本 roadmap.md 追記⑩
- 棚卸しの基準実数（2026-07-09 更新⑦・`ghost-setup` 完了後）: 完了 **123**（`areka-P0-ghost-setup`＝⓪ ghost エンジン起動〜終了統括を新規完了アーカイブ。WS-A talk 契約統一〔新設 `areka-talk`〕＋WS-B 結線の背骨〔新設 `areka-ghost`〕・決定論 spine e2e S1〜S6・env ゲート実 pasta 追験） / **active = 0** / **brief-only = 2**（`areka-P0-emo-present`・`areka-P0-window-placement`〔emo-present ゲート下〕）。正本 roadmap.md 追記⑪
- 棚卸しの基準実数（2026-07-09 更新⑧・`emo-present` 完了後＝⑥emo トラック全完了・並走ブランチ統合の反映）: 完了 **124**（`areka-P0-emo-present` 新規完了アーカイブ＝emo 直列3/3・表示結線。実装完了後の実機まばたきデモ検証で `ComposeCache` のキャッシュ仕様バグ〔surface id 単独キーが bind 差分に衝突〕を発見・是正＝合成入力（surface_id＋BindSet）完全一致キーの容量1メモ化スロットへ再設計） / **active = 0** / **brief-only = 1**（`areka-P0-window-placement`〔**emo-present 完了でゲート解除＝新フロント**〕）。**M-boot 19/21（残2: emo-text-layer・window-placement）**。**即並走可能フロント2本**: window-placement／emo-text-layer（`emo-present`・`seriko-engine`・`ghost-setup` は消化）。正本 roadmap.md 追記⑫

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
