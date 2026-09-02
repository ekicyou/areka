# Brief: areka-P0-ukadoc-survey-toolkit

> 起票: 2026-09-02（`/kiro-discovery` Path D・開発者要望「ukadoc を読み込み、主だった SHIORI Event/Resource・プロパティシステム・Install/Update 設定・nar 仕様などを網羅的に分類し、項目間の繋がりを評価し、areka を製品品質にするために必要な順に実装項目を洗い出してブリーフィングとして整理する調査 spec を作れ。網羅調査に必要な仕組みは Rust で作ることも検討せよ。大規模なら分割せよ」）。
> **ukadoc 網羅調査 6 本のうち 1 本目＝唯一コードを書く spec。** 残り 5 本（`ukadoc-survey-shiori`／`ukadoc-survey-assets`／`ukadoc-survey-sakura-script`／`ukadoc-survey-property`／`ukadoc-coverage-roadmap`）は本 spec の道具で台帳を書く調査 spec であり、実行時コードを 1 行も触らない。
> **開発者追記（2026-09-02）**: 「ukadoc をとにかく網羅的に一度調査して仕訳する。最新の仕様を優先し、古い仕様に新しい書式があれば新書式を優先して旧書式はエイリアスとする。網羅調査→仕訳と関連の検索→最終的な優先度決定と開発仕様立ち上げ→ロードマップ調整、の順。調査段階は並行実施できるように」——本 spec は**仕訳の規則と台帳の形式を凍結する役**であり、規則は下記「仕訳の規則」節が全 survey spec 共通の正本。
> **種別**: 道具 spec（新規 crate 1 本＋`doc/ukadoc-coverage/` 配下の台帳）。既存 crate・実行時挙動は非接触。

## Problem

ukadoc は 1,749 項目（37 ページ）ある。これまでの areka は「emo2 が実際に使う分だけ実装する」規律で M1 を組み上げてきたため、正典全体に対して **何が実装済み／語彙だけ登記済み／縮退／未着手／対象外なのかを一望できる台帳が存在しない**。個別 spec の brief（2026-08-27 のプロパティ系 3 本・文字装飾系 3 本など）はそれぞれ手作業で ukadoc を数え直しており、同じ数え直しが spec ごとに繰り返されている。

手作業で 1,749 項目を分類すると、⑴ 数え漏れ・重複起票が避けられない、⑵ 「実装済み」の根拠（file:line）が陳腐化しても気づけない（記憶 doc-claims-need-file-line-verification＝doc の事実誤認だけで差し戻し 5 回の前例）、⑶ 調査 spec を分割したとき台帳の形式が spec ごとにばらつく。

## Current State

### ukadoc 側（2026-09-02 実測）

- **正典の入手経路**: ukadoc MCP サーバー `ukagaka-doc-mcp`（npm グローバル・`%APPDATA%\npm\node_modules\ukagaka-doc-mcp\data\index.json`・2.7MB・`generatedAt: 2026-08-24T04:08:57Z`・`version: 1`）。構造は `{version, generatedAt, entries[]}`・entry は `{id, title, source, category, content, url}`。
- **規模**: 全 2,983 entry のうち ukadoc 1,749（yaya_wiki 448・satori_wiki 745・aosora_wiki 41 は対象外）。ukadoc 本文 364K 字・本文長の中央値 119 字・p90 384 字・最大 8,763 字。
- **ページ別件数**（id の第 2 セグメント＝ページ・調査 spec の分割単位）:

| category | ページ | 件数 |
|---|---|---|
| shiori_event | list_shiori_event 290／list_shiori_event_ex 168／list_shiori_resource 159／list_plugin_event 19／memo_shiorievent 1 | 637 |
| descript | descript_balloon 162／descript_shell_surfaces 137／descript_shell 102／descript_ghost 74／descript_install 15／descript_plugin 13／descript_headline 9／descript_shell_surfacetable 6 | 518 |
| sakurascript | list_sakura_script | 342 |
| protocol | list_propertysystem 188／spec_shiori3 26／spec_update_file 9／spec_fmo_mutex 6／spec_web 3／spec_sstp 2／spec_dll・spec_headline・spec_plugin 各 1 | 237 |
| file_structure | manual_directory／manual_ghost／manual_shell／manual_balloon／manual_install／manual_update／manual_owner_draw_menu／manual_translator | 8 |
| dev_guide | dev_nar／dev_update／dev_shell／dev_bind／dev_ownerdraw／dev_shell_error／memo | 7 |

- **MCP 検索の限界**: `search_docs` は 1 回 50 件上限・ページングなし（`limit: 500` は入力検証エラー）。`total` は返るので件数は取れるが、**網羅列挙は MCP 経由では不可能**＝スナップショット JSON を直接読む道具が要る。
- **版番号の密度**（本文に `2.x.xx` 形式の SSP 版番号を含む entry）: list_propertysystem 98/188・descript_shell_surfaces 71/137・list_shiori_event 65/290・list_sakura_script 55/342・descript_balloon 31/162・list_shiori_event_ex 0/168。版番号は「SSP 世代」分類の機械抽出源になる。
- **項目間参照の密度**（粗い実測＝他 entry の title 文字列を本文に含む）: 194/1,749 entry・277 辺。機械抽出だけでは繋がりの 1 割程度しか拾えない＝繋がり評価は**機械抽出＋人手の登記**の二層が要る。

### areka 側（2026-09-02 実測・サブエージェント調査＝file:line は着手時に再検証すること）

- **正典側の機械可読資産は既に 2 系統ある**（本 spec はこれを置き換えず、繋ぐ）:
  - `doc/shiori/fragments/`（38 フラグメント＝`_shared.toml`＋events 29＋resources 8・**event entry 287／resource entry 159／field 802／silence_ruling 9**・結合順は `_manifest.toml`）。各 entry は `kind`/`category`/`dispatch`/`response`/`provenance`/`description`。`doc/shiori/README.md` に生成器の受入基準 AC-G1〜G6 が確定済みだが**生成器コードは未実装（スコープ外と宣言）**。
  - `crates/areka-sylphya/src/vocab/`（981 行）＝状態付き語彙台帳。`FLAT_VOCAB` 26（`flat.rs:32`）・`DOTTED_ROOTS` 10（`dotted.rs:21`）・`GENERIC_PROP_NAMES` 17（`dotted.rs:44`）・`SET_EFFECTIVE` 21（`dotted.rs:80`）・`SHIORI_RESOURCE_IDS` 159（`shiori_resource.rs:77`）。件数固定テスト `ledger_key_determinism_tests.rs:201-204`。
- **実装側の白表は小さい**: 送出イベント `ALLOWED_EVENT_IDS` 11 件（`areka-kanade/src/schedule/events.rs:76-88`・const 文字列表・**件数固定テスト無し**）／照会リソース `ALLOWED_RESOURCE_IDS` 1 件（`schedule/resources.rs:31`）／`\![...]` 消費者台帳 `ConsumerLedger::canonical()` 4 登録（`areka/src/emo2_boot/consumer_ledger.rs:221-238`）。
- **機械可読正本が無い領域**: さくらスクリプトタグ（`areka-parsers/src/sakura/decode.rs` の match アームのみ）・surfaces.txt/SERIKO/MAYUNA（`shell/decode.rs`）・descript キー（`package/resolve.rs`・`balloon/parse.rs`）。ここは本 spec の catalog（スナップショット由来）が初の正本になる。
- **ukadoc 同梱の先例**: `.kiro/specs/completed/areka-P0-shiori-protocol/ukadoc/` は HTML を `.gitignore` で非同梱・`SOURCES.md` に URL・取得日・sha256 のみ（第三者著作物）。本 spec の「本文を repo に入れない・ハッシュのみ」方針はこの先例に従う。
- **未対応ログの語彙**: `縮退` 79 件・`無視` 15・`未知` 11・`未対応` 2（`warn!`/`debug!`/`info!` 行）。英語 `unsupported`/`deferred` は 0。代表点＝`areka-emo-compose/src/method.rs:160`（未知の合成メソッド）・`areka-sakura/src/compile.rs:203`（M-boot 外タグ無視）・`areka-seriko/src/table.rs:270-272`（非駆動 interval）。evidence スキャンはこの語彙と `target:`／`event =` 構造化フィールド（91 種）を手掛かりにする。
- **既存の対応表**: `doc/COMPAT_ARCHITECTURE.md` §8 沈黙ルール対応表＝データ行 80 件（項目／裁量／根拠／出典 spec）・`doc/emo2-conformance-scope.md` §6 rescope 表。台帳の `status=degraded` と `note` の転記元。

## 仕訳の規則（全 survey spec 共通・本 spec の要件で凍結）

1. **最新仕様を優先する**。同じ機能に複数の書式・名前があるとき、ukadoc の版番号（`2.x.xx`）が最も新しい書式を**正典**とし、それ以外は **`alias`**（`alias_of` で正典 id を指す）。版番号が無い項目は「初期から存在」として最も古い世代に置く。SSP 以外のベースウェア専用記述（MATERIA／CROW 等の注記）は `not-applicable` 候補。
2. **実装は正典だけ**。alias は正典への写像で受ける（parser／語彙表の 1 か所）。これにより実装 spec の対象数が縮み、既存資産の旧書式も壊れない。
3. **世代を記録する**: `introduced`（版番号・無ければ空）・`deprecated_by`（後継 id・あれば）。世代別の対応表は report が機械生成する。
4. **関連（links）は種別付き**: `alias_of`（旧→新）／`supersedes`（新→旧）／`triggers`（タグ・操作→イベント）／`configures`（descript キー→挙動・タグ・イベント）／`queries`（タグ・イベント→プロパティ）／`same-feature`（束＝同じ機能の別面）。機械抽出（本文の相互言及）は候補提示まで・登記は survey spec の人手。
5. **状態語彙**: `implemented`／`vocabulary-only`／`degraded`／`absent`／`alias`／`not-applicable`／`unclassified`。alias の状態は正典側で判定し、alias 行自身は「写像の有無」だけを持つ。
6. **優先度の根拠は 3 つに限定**（coverage-roadmap が使う）: 壊れ方（黙って壊れる＞明示エラー＞見た目差）・影響する既存資産の広さ（標準辞書が使うか・世代の古さ）・依存基盤の共有度。

**台帳の 1 行（TOML）**: `id`（catalog id）・`status`・`introduced`・`alias_of`／`deprecated_by`（任意）・`evidence[]`（`path:line` と期待トークン）・`owner_spec`・`priority`（A〜E 段階＋数値）・`links[] = {kind, to}`・`note`。**この形式は本 spec の要件確定時点で凍結し、survey 4 本は本 spec の実装完了を待たず着手できる**（検査は後から追いつく）。

## 段階（開発者追記の 4 段と spec の対応）

| 段 | 内容 | 担当 | 並行 |
|---|---|---|---|
| ① 網羅調査 | 1,749 項目の全数台帳化（catalog＋areka 証跡） | toolkit（器）＋survey 4 本 | survey 4 本は並走（台帳ファイル別） |
| ② 仕訳と関連の検索 | 新旧仕訳（alias）・links 登記・無所有の炙り出し | survey 4 本（各ドメイン内）＋coverage-roadmap（ドメイン跨ぎ） | ドメイン内は①と同時・跨ぎは③の前 |
| ③ 優先度決定と開発仕様立ち上げ | 束→優先順→上位の束から開発 spec の brief 起票 | coverage-roadmap（`/kiro-discovery` 再入を含む） | 直列（e2e 完了後） |
| ④ ロードマップ調整 | M2+ マイルストーンとウェーブへ反映 | 棚卸セッション（別セッション一括裁定） | 直列 |

## Desired Outcome

- ukadoc 1,749 項目の**全数**が 1 つの台帳形式に載り、各項目に「分類・areka 状態・根拠 file:line・担当 spec・世代・alias・優先度・繋がり」を書ける器がある。
- 台帳の整合（id の実在・全数の網羅・根拠 file:line の実在）が `cargo test` で機械検査される。根拠が陳腐化したら赤になる。
- 調査 spec 4 本が同じ道具・同じ形式で台帳を書ける。台帳から `doc/ukadoc-coverage/report.md`（網羅率・未分類件数・優先度別一覧）が再生成できる。
- ukadoc スナップショットが更新されたとき（MCP パッケージ更新）、差分（追加・削除・本文変更）が機械で出る。

## Approach

**新規 crate `crates/ukadoc-survey`（lib＋bin・既存 crate 非依存・既存 crate から非参照）**を建て、次の 4 機能を持たせる。

1. **正規化（catalog）**: スナップショット JSON → `doc/ukadoc-coverage/catalog.toml`（id・page・title・category・SSP 版番号（抽出）・本文ハッシュ・url）。**本文そのものは repo に入れない**（ukadoc の著作物を丸ごと同梱しない・ハッシュで変更検出だけ行う＝`shiori-protocol` の `SOURCES.md` 先例）。スナップショットの所在は環境変数 `AREKA_UKADOC_SNAPSHOT`（既定＝npm グローバルの実パス）。既存の `doc/shiori/fragments`（SHIORI event/resource の契約カタログ）と sylphya 語彙表は**置き換えず**、catalog id からそれらの entry 名へ結ぶ対応列を持つ（同じ項目を 2 か所で数えない）。
2. **証跡スキャン（evidence）**: areka ソース全域を走査し、台帳の「根拠」候補を機械で集める——SHIORI イベント名の文字列リテラル・`\![...]` コマンド名の消費側 `name` 選別・descript キー表・sylphya 語彙表・「未対応」系ログ行（走査規則はサブエージェント実測の慣習に合わせて design で確定）。
3. **台帳（ledger）**: `doc/ukadoc-coverage/ledger/<domain>.toml`（調査 spec 1 本＝1 ファイル＝共有ファイル 0 で並走可）。行の形式と `status` 語彙は上記「仕訳の規則」節で凍結。
4. **検査と報告（check / report）**: `cargo test -p ukadoc-survey` で ⑴ ledger の id ⊆ catalog ⑵ catalog 全 id が ledger のどこかに 1 回だけ現れる（未分類は `unclassified` として明示・件数を台帳に固定し減少のみ許す） ⑶ `evidence` の file:line が現在の作業木に実在し、行に期待トークンを含む ⑷ links の両端が実在・`alias_of` の先は `alias` でない（alias の連鎖禁止）・`introduced` の版番号は catalog の抽出値と矛盾しない。`report.md`（状態分布・世代別対応表・alias 一覧・束の連結成分）は決定論的に再生成し、差分ゼロをテストで検査する。

**規模**: 中。crate 1 本（目安 1,000 行未満・行数番人の対象）・TOML/JSON の読み書きは既存依存（`toml`・`serde`・`serde_json`）で足りる。

**却下した代替**: ⒜ Python スクリプト（`tools/perf` 方式）——台帳検査を `cargo test` のゲートに入れられず陳腐化を止められない。⒝ MCP 検索の繰り返し——50 件上限・ページング無しで網羅列挙不能。⒞ 台帳を Markdown 表で手書き——1,749 行の表は機械検査できず、spec ごとに形式が割れる。

## Scope

- **In**:
  - `crates/ukadoc-survey`（catalog／evidence／ledger／check／report）。
  - `doc/ukadoc-coverage/`（catalog.toml・ledger/ の空雛形 4 本・report.md・README）。
  - 全 1,749 id の初期台帳＝全行 `unclassified`（調査 spec が埋める）。
  - 証跡スキャンの走査規則（areka の実慣習に合わせる・design で確定）。
- **Out**:
  - 項目の分類・優先度付け・繋がり評価そのもの（調査 spec 4 本）。
  - ukadoc 本文の repo 同梱。yaya/里々/蒼空 wiki（対象外・catalog にも載せない）。
  - areka 実行時コードの変更（1 行も触らない）。

## Boundary Candidates

- catalog（正典の写し）と ledger（areka の判定）は別ファイル・別責務。catalog は機械生成のみ・ledger は人手＋機械検査。
- evidence スキャンは「候補提示」まで。判定（status）は調査 spec の人手。

## Out of Boundary

- ukadoc MCP サーバー自体の改修・スナップショット再生成（外部パッケージ）。
- SSP 実機との挙動比較（各機能 spec の実機サインオフ）。

## Upstream / Downstream

- **Upstream**: なし（新規 crate・既存 crate 非依存）。ukadoc スナップショット（外部・2026-08-24 生成）。
- **Downstream**: `ukadoc-survey-shiori`／`ukadoc-survey-assets`／`ukadoc-survey-sakura-script`／`ukadoc-survey-property`（台帳を書く・**本 spec の要件確定後に並走開始可**）→ `ukadoc-coverage-roadmap`（統合）。将来の全機能 spec（着手時に台帳の該当行を `implemented` へ更新する義務＝`/kiro-complete` の DoD 候補）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `tools/perf`（Python 道具群・方式の先例だが非接触）／`log-capture-kit` の行数番人 `file_length_guard_test.rs`（新 crate も対象・1,000 行未満で建てる）／`emo2-conformance-e2e`（W12・共有ファイル 0＝新規 crate＋`doc/ukadoc-coverage/` のみ）。

## Constraints

- Rust 2024・ワークスペース `crates/*` 自動包含＝`cargo test --workspace` に乗る。テストは決定論のみ（ネットワーク・実機不使用）。
- スナップショットが無い環境でもテストが赤にならないこと（catalog.toml が repo 内の正本・スナップショット読みは再生成 bin のみ）。
- 台帳の語彙は平易な語で（「未対応」「語彙のみ」「縮退」「対象外」）。符牒を持ち込まない。
- ロードマップ本文（`.kiro/steering/roadmap.md`）は本 spec では触らない（棚卸⑫と同時進行中・追記(89) のみ）。
