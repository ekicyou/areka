# Brief: areka-P0-ukadoc-survey-assets

> 起票: 2026-09-02（`/kiro-discovery` Path D・ukadoc 網羅調査 5 本の 3 本目）。
> **種別**: 調査 spec（台帳＋ブリーフィング節・実行時コード非接触）。`ukadoc-survey-toolkit` の道具で `doc/ukadoc-coverage/ledger/assets.toml` を書く。
> **所有範囲＝「資産の定義と配布・更新」**: descript_ghost 74・descript_shell 102・descript_shell_surfaces 137・descript_shell_surfacetable 6・descript_balloon 162・descript_install 15・descript_plugin 13・descript_headline 9・spec_update_file 9・file_structure 8・dev_guide 7＝**542 項目**。

## Problem

既存の伺か資産（ゴースト／シェル／バルーン）は descript.txt・surfaces.txt・install.txt・updates2.dau という定義ファイル群で成り立つ。areka は emo2 が使うキーだけを読み、それ以外のキーの扱い（黙って捨てる／警告する）が台帳化されていない。**nar のインストールとネットワーク更新はゼロ**（roadmap M2 予約）であり、「既存ゴーストを入手して入れる」導線が無い限り製品としての入口が成立しない。開発者要望が Install/Update 設定と nar 仕様を名指ししているのはこのため。

## Current State

### ukadoc 側（2026-09-02 実測）

- **descript_install 15**: `charset`／`name`／`type`／`accept`／`directory`／`bootghost`／`refresh`／`refreshundeletemask`／`*.directory`／`*.source.directory`／`*.refresh`／`*.refreshundeletemask`／相対パス（`nonar`・`noupdate`・`ignore` オプション）。
- **spec_update_file 9**: updates2.dau の文字コード・行フォーマット・行種別・必須フィールド・拡張フィールド・URL エンコード・セキュリティチェック・ファイル削除・`ghost\master` へのコピー。
- **file_structure 8**: 全体構成（`manual_directory`＝ghost/master・shell/master・updates2.dau・*.ico/*.cur・menu_*.png・readme）・ゴースト・シェル・バルーン・インストール（**nar＝zip の拡張子違い**・install.txt があれば D&D でインストーラ機能・lzh/cab/自己解凍 exe は SSP 独自）・ネットワーク更新（updates2.dau／updates.txt／delete[N].txt／developer_options.txt／thumbnail.png）・オーナードローメニュー・トランスレータ。
- **dev_guide 7**: 配布ファイル作成（`dev_nar`）・ネットワーク更新対応（`dev_update`）・シェル作成・bind・オーナードロー・シェルエラー・memo。
- **descript_ghost 74**（版番号 8）: `homeurl`／`install.accept`／`secondchangeinterval`／`otherghosttalk`／`sakura.*`・`kero.*` 既定・`shiori`／`makoto`／`balloon`／`icon`／`cursor`／`seriko.*` ほか。
- **descript_shell 102**（版番号 22）・**descript_shell_surfaces 137**（版番号 71＝SERIKO/MAYUNA の世代差が最も濃い）・**surfacetable 6**。
- **descript_balloon 162**（版番号 31）: 既存の完了 spec（kero-balloon／balloon-visibility／balloon-vertical-canon 等）と M2 ゲート `balloon-canon-residue`（残語彙 10 項目）が部分的に台帳化済み。
- **descript_plugin 13／descript_headline 9**: PLUGIN・HEADLINE ホスティング（M2 予約）の定義ファイル。

### areka 側（2026-09-02 実測・file:line は着手時に再検証すること）

- **ghost/master descript の受理キーは 7 系統**（`areka-parsers/src/package/resolve.rs:69-83`・:111-135）: `name`／`sakura.name`／`sakura.name2`／`kero.name`／`shiori`／`seriko.defaultsurfacedirectoryname`／`sakura|kero.bindgroupN.*`・`bindoptionN.group`。正典 74 に対し約 1 割。
- **balloon descript は完全一致で 29 キー**（`balloon/parse.rs`）: `origin.*`／`validrect.*`／`wordwrappoint.*`／`windowposition.*`／`font.*`／`cursor.*`／`vertical`＋areka 拡張 `writing_mode`／`budoux_newline`。正典 162。
- **未知キーは無言で捨てる**: KV 化 `kv/parse.rs:21` は最初のカンマ分割・後勝ち・分類も警告もしない。balloon は完全一致引きゆえ自然に無視（`balloon/parse.rs:10`・:39 に明文）。例外＝bindgroup パーツ名欠落の `warn!`（`package/resolve.rs:296`）。**「黙って無い」が既定挙動**＝本 spec の未知キー方針登記の起点。
- **surfaces.txt**（`shell/decode.rs`）: 認識語＝`descript`／`surface`／`surface.append`／`kero.surface.alias`／`element`／`collision`／`animation`／`interval`／`pattern`／`bind`／`random`／`ascend`／`descend`。interval 駆動は `bind`／`random`／`bind+random` の 3 種のみ（他は `Interval::Other` へ忠実転記・駆動側は `debug!`）。pattern method は `areka-emo-compose/src/method.rs:142` で解決（未知は `warn!`＋`Unknown`・`base` はシームのみ）。collision は矩形のみ（`ellipse`／`circle`／`polygon` 0 件・`collisionex` は読み飛ばし）。`doc/emo2-conformance-scope.md:82` に「完全マップ→SERIKO/2.0＋MAYUNA bind・overlay・interval 3 種・矩形 collision」への縮小が明記。
- **install／update／nar は全面不在**: `updates2.dau` 0 件・`updates.txt` 0 件・`.nar` 0 件・`OnUpdate` 0 件・zip 展開なし（workspace 依存にも無し）・ネットワーク I/O なし。`install.txt` は「触れない」宣言（`package/resolve.rs:8`）と「結果に影響しない」テスト（`validation_tests.rs:113,123`）のみ。
- **plugin／headline descript**: 実装なし（sylphya の根枝名のみ）。

## Desired Outcome

- 542 項目すべてに status・根拠・担当 spec・優先度が付き、`unclassified` が 0。
- **未知キーの扱い方針**（黙って捨てる／ログに残す／エラー）が descript 種別ごとに台帳化され、「黙って無い」キーが残らない。
- ブリーフィング節（`doc/ukadoc-coverage/briefing-assets.md`）に **nar インストール導線とネットワーク更新の最小成立要件**（install.txt 解釈・zip 展開・配置規則・updates2.dau 照合・delete.txt・OnUpdate 系イベントとの繋がり）が、既存ゴースト資産を入れて動かすまでの順で並ぶ。
- surfaces.txt の SERIKO/MAYUNA 137 項目について、SSP 世代（版番号）別の対応表が出る（product.md「SERIKO/MAYUNA 完全マップ」の実測版）。

## Approach

- toolkit が凍結した仕訳規則（最新優先・新書式正典・旧書式 alias・版番号＝世代・種別付き links）を適用する。本ドメインは版番号が濃い（surfaces 71/137・balloon 31/162）＝新旧書式の併存が最も多い面。alias 例＝surfaces.txt の `surface.append` と旧 `surface` 追記法・SERIKO/1.x の `interval,talk`／`always`／`runonce` 系と SERIKO/2.0 の `animationN.interval`・collision の矩形と `collisionex`（後継＝`supersedes`）・`updates.txt` と `updates2.dau`（後継）・balloon の `sakura.*`/`kero.*` と `char*.*`。
- **着手条件**: toolkit の要件確定後・実装完了を待たない。他 survey と並走。
- 実装済みの証拠は toolkit の規則 7（ソースの ukadoc URL）＝`implemented` と判定した descript／surfaces キーの定義箇所（`package/resolve.rs` の定数・`balloon/parse.rs` のキー・`shell/decode.rs` の語）へ URL の doc コメントを置く。実行時挙動を変えない doc コメントのみ・本 spec の唯一のコード接触。
- 定義ファイル種別（ghost／shell／surfaces／balloon／install／plugin／headline／update）ごとに 1 節。各節の分類軸＝⑴ 読み手（parsers の転記層 or 下流エンジン） ⑵ SSP 世代 ⑶ 依存基盤（例: `menu.*`＝オーナードローメニュー・`*.refresh`＝更新機構） ⑷ 既存 spec の所有（COMPAT §8 の裁量登記を含む）。
- install/update/nar は「導線」として並べる（入手→展開→配置→起動→更新→削除）。各段に必要な正典項目とイベント（shiori 台帳の `links`）を繋ぐ。
- balloon 162 は完了 spec と M2 ゲート brief の登記を先に取り込み、差分だけ人手で埋める。

## Scope

- **In**: 上記 542 項目の台帳・未知キー方針の登記・導線ブリーフィング・SERIKO/MAYUNA 世代別対応表。
- **Out**: 実装・sakura／property／shiori 台帳・SSP 実機との挙動比較。

## Boundary Candidates

- 「定義ファイルの解釈（parsers）」と「配布・更新（アプリ層の新基盤）」は別節・別の M2 候補。
- surfaces.txt（SERIKO/MAYUNA）は単独で 1 節＝将来 spec 分割の自然な境界。

## Out of Boundary

- nar の作成側（`\![execute,createupdatedata]`・開発者機能）＝配布者向け機能は優先度のみ付けて対象外候補。
- トランスレータ・ヘッドライン本体の実装可否。

## Upstream / Downstream

- **Upstream**: `ukadoc-survey-toolkit`。既存: `areka-parsers`（descript/surfaces/balloon の転記層）・`balloon-canon-residue`・`surfaces-basepos`・完了 spec の COMPAT §8 登記。
- **Downstream**: `ukadoc-coverage-roadmap`。将来の M2 候補（nar インストール・ネットワーク更新・ゴースト/バルーン選択 UI・SERIKO/MAYUNA 世代拡充）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `balloon-canon-residue`（残語彙 10 項目＝台帳へ取り込み・二重起票しない）／`surfaces-basepos`（surfaces 転記層の追跡 spec）／`emo2-conformance-e2e`（W12・共有ファイル 0）。

## Constraints

- 台帳は `doc/ukadoc-coverage/ledger/assets.toml` 1 ファイルのみ。
- 根拠は file:line。parsers の「未知キー」ログ慣習は areka 側実測に従って記す。
- nar は zip（実体）＝展開ライブラリの選定は本 spec でしない（実装 spec の設計事項）。

> **📌 2026-09-02 開発者指針「伺からしい価値観を持つ要素の優先度を上げる」（toolkit 規則 6・9 に凍結・裁定は議題 5〜7）**: 分類軸に ⑸ **テーマ付与**（`values[]`）を加える。本ドメインの代表例: **更新**＝updates2.dau 9（spec_update_file）・install.txt の `*.refresh`／`refreshundeletemask`・delete.txt・`developer_options.txt`・descript `homeurl`・file_structure「ネットワーク更新」＝**新旧両軸で高い唯一の群・段階 B 先頭**（開発者 2026-09-02「ネットワーク更新に関する軸は優先すべきかも」）／**触れ合い**＝surfaces の `collision`（部位名＝撫での前提）・nar の D&D インストール（触れ合い＋更新の 2 テーマ）／**装い**＝descript_shell_surfaces 137（SERIKO まばたき・口パク・MAYUNA bind）・`menu.*`（着せ替えメニュー）・シェル／バルーン切替の descript キー／**気配**＝descript_ghost `secondchangeinterval`・balloon の表示寿命キー／**記憶**＝`install.accept`（初回導入）。**テーマ 0**＝dev_guide の作成側（`dev_nar`／`createupdatedata`・配布者向け）・plugin／headline の descript・トランスレータ。未知キー方針（黙って捨てる／ログ／エラー）は壊れ方⑴の判定材料＝テーマより先に並ぶ。

