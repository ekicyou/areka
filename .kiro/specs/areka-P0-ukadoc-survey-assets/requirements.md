# Requirements Document

## Project Description (Input)
既存の伺か資産（ゴースト／シェル／バルーン）は descript.txt・surfaces.txt・install.txt・updates2.dau という定義ファイル群で成り立つ。areka は適合対象ゴースト emo2 が使うキーだけを読み、それ以外のキーの扱い（黙って捨てる／記録を残す／エラーにする）が台帳化されていない。nar のインストールとネットワーク更新は実装がまったく無く、「既存ゴーストを入手して入れる」導線が無い限り製品としての入口が成立しない。本 spec は ukadoc の「資産の定義と配布・更新」ドメイン 542 項目を全数仕訳し、台帳 `doc/ukadoc-coverage/ledger/assets.toml`・ドメイン別報告・ブリーフィング文書を成果物とする調査 spec である。areka の実行時の振る舞いは 1 行も変えない。

## Introduction

本 spec は「ukadoc 網羅調査」6 本のうちの調査 spec 1 本であり、**資産の定義と配布・更新**（ゴースト／シェル／サーフェス／バルーン／インストール／プラグイン／ヘッドライン／更新ファイル）を担当する。

成果物は次の 4 つで、**実行時の振る舞いを変える変更は 1 つも含まない**。

1. **台帳** `doc/ukadoc-coverage/ledger/assets.toml` — 542 項目の全数に状態・世代・別名・担当 spec・優先度・テーマ・関連・備考を書いたもの。
2. **ドメイン別報告** `doc/ukadoc-coverage/report/assets.md` — 台帳から機械で再生成するもの（上流の道具が着地している場合）。
3. **ブリーフィング文書** `doc/ukadoc-coverage/briefing-assets.md` — 人手で書く読み物。未知の記述の扱い・SERIKO/MAYUNA の世代別対応表・nar インストールとネットワーク更新の導線を載せる。
4. **ソースの doc コメント** — 「実装済み」と判定した項目の定義箇所に置く正典 URL 1 行。**本 spec が `crates/` に触れるのはこれだけ**である。

### 上流の契約（先に凍結済み・本 spec はこれに従う）

台帳の形式・状態語彙・担当ページの分割・仕訳の規則・報告の構成は、上流 spec `areka-P0-ukadoc-survey-toolkit` の**承認済み要件**（`.kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md`・付録 A / 付録 B を含む）で既に凍結されている。本 spec はそれを**再発明せず参照する**。以降「上流契約」と書いたらこの文書を指す。

本 spec の着手条件は上流契約の**要件確定**であり、上流の実装完了ではない（上流契約 Introduction・要件 2.1 が明記）。したがって道具が未着地の間も台帳とブリーフィングは書ける。

### 担当範囲（上流契約 要件 3.1 の割り当て・2026-09-02 にスナップショットで再実測して一致を確認）

| カテゴリ | ページ | 件数 |
|---|---|---|
| descript | descript_balloon 162・descript_shell_surfaces 137・descript_shell 102・descript_ghost 74・descript_install 15・descript_plugin 13・descript_headline 9・descript_shell_surfacetable 6 | 518 |
| protocol | spec_update_file 9 | 9 |
| file_structure | manual_balloon／manual_directory／manual_ghost／manual_install／manual_owner_draw_menu／manual_shell／manual_translator／manual_update 各 1 | 8 |
| dev_guide | dev_bind／dev_nar／dev_ownerdraw／dev_shell／dev_shell_error／dev_update／memo 各 1 | 7 |
| **合計** | **24 ページ** | **542** |

### 現状の実測（2026-09-02・スナップショット `generatedAt` = 2026-08-24T04:08:57.881Z・`version` = 1）

**正典（ukadoc）側**

- 担当 24 ページの合計は**ちょうど 542 件**。ukadoc 全体 1,749 件・38 ページ・カテゴリ 6 種はいずれも上流契約の記載と一致した。
- 542 件のうち **15 件はアンカーを持たないページ全体の項目**（`manual_*` 8・`dev_*` 6・`memo` 1）。1 項目が 1 ページ分の説明にあたるため、他の 527 件（キー 1 個ずつ）と粒度が大きく違う。
- 見出しは一意ではない。542 件に対し相異なる見出しは 477 種で、**重複群 40・関与 105 件**。ほとんどはページ跨ぎ（`charset,文字コード` が 7 ページに現れるなど）で、**ページ内の重複は `descript_shell_surfaces` の `bind` 1 組だけ**である。
- descript 系 518 件のうち、見出しが「キー,説明」の形をしているのは 425 件（キー名 349 種）。残り 93 件は読点を持たない語（`menu.disable.font.color.r`・SERIKO の間隔語 `sometimes`／`always`／`runonce`／`never`・合成メソッド語 `base`／`overlay`／`blend-*` など）。
- 本文に SSP 版番号（`x.y.z` の形）が現れる件数はページによって大きく違う（surfaces 71/137・shell 22/102・balloon 32/162・ghost 11/74・install 2/15・更新ファイル 0/9）。**版番号の抽出規則は上流の道具が凍結するため、本要件では抽出件数を固定しない。**
- 他ベースウェア専用の注記は少数で、いずれもページ全体項目に集中する（MATERIA 8・CROW 5）。廃止の注記は `descript_shell_surfaces` に 3 件。

**areka 側（file:line は 2026-09-02 に再検証済み）**

- **ゴースト／シェルの descript は受理キーが 13 形**。`crates/areka-parsers/src/package/resolve.rs:69-83`（`name`／`sakura.name`／`sakura.name2`／`kero.name`／`shiori`／`seriko.defaultsurfacedirectoryname` の 6 キー）＋ 同 `:111-121` の定数と `:146-218` の照合（`sakura|kero.bindgroupN.default`／`.name`／`bindoptionN.group` の 6 形）＋ `charset`（`crates/areka-parsers/src/charset/prescan.rs:54`・クレート内で唯一の大小文字を無視する照合）。正典 74（ゴースト）＋102（シェル）に対して 1 割に満たない。
- **バルーンの descript は完全一致引きで 30 キー**（`crates/areka-parsers/src/balloon/parse.rs:70-160`・照合箇所は 31）。正典 162 に対して 2 割に満たない。`vertical` は正典キー（`:116` に SSP 2.8.80 と明記）であり、areka 独自の拡張は `writing_mode`（`:110`）と `budoux_newline`（`:113`）の 2 つ。
- **未知の記述は黙って捨てるのが既定**。KV 化（`crates/areka-parsers/src/kv/parse.rs:20`）は最初の読点で分割し（`:26`）、同じキーは後勝ちで上書きし（`:39`）、分類も記録もしない。バルーンは完全一致引きゆえ自然に無視され、その旨が `balloon/parse.rs:9`・`:39`・`:124-125` に明文で書かれている。
- **`areka-parsers` クレート全体で記録を残す経路は 1 つだけ**＝`package/resolve.rs:296-300` の警告（bindgroup の名前宣言にパーツ名が無い）。エラー記録は 0 件。
- **install.txt は「触れない」と宣言済み**（`package/resolve.rs:7-8`）。結果に影響しないことを固定するテストが `crates/areka-parsers/src/package/validation_tests.rs:113`・`:123`・本体 `:128`。なお `resolve.rs:8` の宣言文自体が陳腐化しており、実際には読んでいる `sakura.name2` が列挙から漏れている。
- **surfaces.txt の認識語**（`crates/areka-parsers/src/shell/decode.rs`）は `descript`(:118)／`kero.surface.alias`(:122)／`surface.append`(:127)／`surface`(:132)／`element`(:197)／`overlay`(:198)／`collision`(:234)／`animation`(:310)／`interval`(:323)／`pattern`(:334)／`bind`(:387)／`random`(:388)／`bind+random`(:391)／`animation-sort`(:501)／`collision-sort`(:502)／`ascend`(:495)／`descend`(:496)。`charset` は照合されず素通りする。
- **間隔語 3 種のうち実際に動くのは 2 種**。転記は `decode.rs:385-397`（未知語は忠実転記）、駆動は `crates/areka-seriko/src/table.rs:105-136` で `random` と `bind+random` のみ。`bind` は静的な bind ゆえ非駆動で `:111-116` に記録が出る。
- **合成メソッドは実導出が `overlay` だけ**（`crates/areka-emo-compose/src/method.rs:142` で解決・`:129-131` が実装済み判定・未知語は `:160-161` で記録を残して未知として吸収）。`base` は語彙だけの受け口（`:45`・`:153`）。
- **当たり判定は矩形のみ**。`collisionex`（円・楕円・多角形）は `decode.rs:234-236` で**何も記録せずに読み飛ばす**。
- **install／update／nar は実装がまったく無い**。`crates/` 配下で `updates2.dau` 0 件・`updates.txt` 0 件・`.nar` 0 件・Rust から参照される `OnUpdate` 0 件（5 件はテスト用ゴースト辞書 `crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/dic/update.pasta` の中だけ）。zip 展開の依存も無く（圧縮系の依存は PNG 読み込みの推移依存のみ）、ネットワーク入出力も 0 件。
- **プラグイン／ヘッドラインは名前の予約だけ**。`crates/areka-sylphya/src/vocab/dotted.rs:24`（`headlinelist`）・`:25`（`pluginlist`）ほかに現れるのみで、`:103-104` が「M1 は名前の予約に留め」と明記している。descript の解析も PLUGIN 通信も無い。
- **`doc/ukadoc-coverage/` はまだ存在しない**（上流の道具が未着地）。repo 全体で正典 URL を書いた doc コメントは **0 件**であり、本 spec が置く URL が最初の 1 件になる。
- 既存の判断記録は `doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表（見出し `:122`・データ行 `:128-207` の 80 行）と `doc/emo2-conformance-scope.md` の見直し表（データ行 `:82-88` の 7 行・うち `:82` が SERIKO の縮小行）。80 行のうち**本ドメインの項目に触れるものが 44 行**あり、そのうち 16 行が「未実装／語彙記録／M1 非受理／非追従」＝縮退の転記元になる。**install／update／nar に触れる行は 0 行**である。

### brief と実測・上流契約の食い違い（本要件で採るのは実測と上流契約）

1. **項目数と分類名**: brief は「file_structure 8・dev_guide 7」とカテゴリ名で書いているが、上流契約の割り当てはページ単位（`manual_*` 8 ページ・`dev_*` 6 ページ＋`memo`）である。指す集合は同じで、合計 542 も一致した。本要件はページ名で書く。
2. **spec_update_file の見出し**: brief は「ファイル削除」と書くが、実際の見出しは「**ファイル走査**」である。
3. **descript_install の内訳**: brief の 12 個の名前付きキーは正しいが、相対パス系は 1 項目ではなく **3 項目**（`相対パス`／`相対パス,オプション1,...`／`相対パス,ignore`）である。
4. **areka 側の受理キー数**: brief の「7 系統」は少ない。実際は **13 形**で、`charset` と bindgroup 系 3 種の内訳が漏れている。
5. **バルーンの受理キー数**: brief の「29 キー」は実際には **30 キー**（`windowposition.limit` が漏れている）。また brief は `vertical` を areka 拡張に数えているが、`vertical` は正典キーである。
6. **file:line のずれ**: brief の `kv/parse.rs:21` は現在 `:20`（関数）／`:26`（分割）／`:39`（後勝ち）、`balloon/parse.rs:10` の明文は現在 `:9`。他の参照は一致した。
7. **surfaces.txt の認識語**: brief の一覧に `overlay`・`bind+random`・`animation-sort`・`collision-sort` が漏れており、逆に `charset` は実際には照合されていない。
8. **隣接 spec の残語彙**: brief は `balloon-canon-residue` を「残語彙 10 項目」と書くが、当該 brief は **12 項目**を番号付きで持つ（項目 11・12 は 2026-08-29 に追加され、当該 brief の Scope 行が追随していない）。
9. **隣接 spec の数**: brief の Adjacent は 3 本だが、本ドメインの descript キーを名指しで所有する未着手 spec が**ほかに 4 本**ある（`text-decoration-canon`／`anchor-tag-canon`／`choice-marker-styling`／`charset-canon`）。
10. **上流の欄名**: 上流 spec の brief は `owner_spec`・`deprecated_by` という欄名を使っているが、承認済み要件の付録 A が凍結した欄名は **`owner`・`supersedes`** である。本要件は付録 A に従う。
11. **報告のファイル構成**: 上流 spec の brief は単一の `report.md` を挙げているが、承認済み要件 7.1／7.2 が凍結したのは `report/<ドメイン>.md` 4 本＋`report/summary.md` である。本要件は後者に従う。

## Boundary Context

- **In scope**:
  - 担当 24 ページ 542 項目の全数仕訳（状態・世代・別名／後継・担当 spec・仮の優先度・テーマ・関連・備考）を `doc/ukadoc-coverage/ledger/assets.toml` に書くこと。
  - 定義ファイル種別ごとの「未知の記述の扱い」の登記。
  - SERIKO/MAYUNA（`descript_shell_surfaces` 137 項目）の SSP 世代別対応表。
  - nar のインストールとネットワーク更新の導線ブリーフィング（何が要るかを並べるところまで）。
  - 「実装済み」と判定した項目の定義箇所へ置く正典 URL の doc コメント。
  - 上流の道具が着地している場合の `doc/ukadoc-coverage/report/assets.md` の再生成。
- **Out of scope**:
  - 実装・設計。nar の展開ライブラリの選定、更新機構の設計、プラグイン／ヘッドラインの実装可否の判断。
  - 他 3 ドメインの台帳（`shiori.toml`・`sakura-script.toml`・`property.toml`）と、そこに属する項目の仕訳。
  - 全体の報告 `report/summary.md` と束の解説 `linkage.md` の作成（統合担当 `ukadoc-coverage-roadmap` の仕事）。
  - 段階 A〜E の最終順序の決定（同上）。
  - 上流の道具（調査用クレート）の実装、カタログ・テーマ定義・README の作成。
  - SSP 実機との挙動比較。隣接 spec の brief の書き換え。
  - nar の作成側（配布者向け機能）の実装可否。台帳には載せるが導線ブリーフィングでは対象外候補として扱う。
- **Adjacent expectations**:
  - 上流契約が台帳の形式・状態語彙・ページ割り当て・仕訳の規則・報告の構成を凍結している。本 spec はそれを変更しない。
  - 並走する調査 spec 3 本（shiori／sakura-script／property）とページの重なりは無い（4 本の brief で相互に明示されており、本要件でも再確認した）。共有する編集対象ファイルも無い。
  - `emo2-conformance-e2e`（W12）とは共有ファイル 0。同 spec は `doc/ukadoc-coverage/` にも descript／surfaces／balloon の解析コードにも触れない。
  - 本ドメインの項目の一部は既に隣接 spec が担当を宣言している。台帳はそれを担当として記録するだけで、二重に起票しない。
  - 導線は本ドメインの外へ伸びる。さくらスクリプト側の実行タグは sakura-script 台帳、`OnUpdate` 系イベントは shiori 台帳が持つ。本 spec はそれらを関連として指す。
  - SAORI は ukadoc に独立ページが無く、shiori 台帳が DLL 共通仕様の 1 項目として扱う（shiori spec の brief で決着済み）。本 spec は扱わない。

## Requirements

### Requirement 1: 台帳ファイルの新設と全数収容
**Objective:** 統合担当として、資産ドメイン 542 項目の全数が 1 つの台帳に漏れなく載っていてほしい。それにより網羅率と残りが数で読める。

#### Acceptance Criteria
1. The 資産ドメイン調査 shall 台帳を `doc/ukadoc-coverage/ledger/assets.toml` の 1 ファイルだけに置き、他の台帳ファイルを作らない。
2. The 資産ドメイン調査 shall 台帳の各項目を上流契約 付録 A の形（`[entry."<項目 id>"]` に続けて欄を 1 行ずつ書く複数行の塊）で書き、1 項目を 1 行に詰めない。
3. The 資産ドメイン調査 shall 台帳の冒頭にドメイン名 `assets` と、上流契約 要件 3.1 が本ドメインへ割り当てた 24 ページの一覧を記す。
4. When 本 spec が完了する, the 資産ドメイン調査 shall 台帳に項目の塊をちょうど 542 個持ち、その項目 id の集合が「スナップショットで取得元が ukadoc であり、かつページが担当 24 ページに属する id」の集合と完全に一致する。
5. The 資産ドメイン調査 shall 項目を id の文字順に並べ、同じ id を 2 回書かない。
6. The 資産ドメイン調査 shall 担当 24 ページ以外のページに属する id を台帳に書かない。
7. Where 上流の道具が未着地でカタログ `doc/ukadoc-coverage/catalog.toml` が存在しない, the 資産ドメイン調査 shall 上流契約 付録 B の手順でスナップショットから id を直接写し、件数が 542 であることを確かめる。
8. The 資産ドメイン調査 shall id とアンカーの文字列を見た目で直さずそのまま写す（見出しを符号化した部分を含む）。
9. The 資産ドメイン調査 shall ukadoc の本文を台帳へ書き写さない（書くのは areka 側の判定と、その根拠を指す文だけ）。
10. The 資産ドメイン調査 shall アンカーを持たないページ全体の項目 15 件（`manual_*` 8・`dev_*` 6・`memo`）も他の項目と同じ形で収容し、粒度が粗いことを備考に記す。

### Requirement 2: 全項目の仕訳と状態語彙
**Objective:** 統合担当として、未分類が 1 件も残っていない台帳がほしい。それにより優先度の議論を全数の上で行える。

#### Acceptance Criteria
1. When 本 spec が完了する, the 資産ドメイン調査 shall 台帳の全 542 項目について、状態・登場した版・担当 spec・優先度・テーマ・関連・備考の欄をすべて備えた状態にする。
2. When 本 spec が完了する, the 資産ドメイン調査 shall 状態が「未分類」の項目を 0 件にする（台帳を `unclassified` で検索して 1 件も出ないこと）。
3. The 資産ドメイン調査 shall 状態に上流契約 要件 2.2 の 7 語（`implemented`／`vocabulary-only`／`degraded`／`absent`／`alias`／`not-applicable`／`unclassified`）以外を書かない。
4. When 項目の状態を `alias` とする, the 資産ドメイン調査 shall 別名の参照先に正典側の id を書き、その指す先の項目の状態が `alias` でないようにする。
5. Where 同じ機能に新旧の書式がある, the 資産ドメイン調査 shall 上流契約 要件 4.1 の順（正典本文の注記 → SSP 版番号 → 人手の判断）で正典と別名を決め、どの手掛かりで決めたかを備考に書く。
6. Where 項目が SSP 以外のベースウェア専用の記述である, the 資産ドメイン調査 shall `not-applicable` の候補として扱い、その根拠を備考に書く。
7. When 項目の本文に SSP 版番号が無い, the 資産ドメイン調査 shall 登場した版の欄を空にし、最も古い項目とは決めつけない。
8. When 項目の状態を `degraded` とする, the 資産ドメイン調査 shall 縮退の転記元（`doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表の行、または `doc/emo2-conformance-scope.md` の見直し表の行）を備考に示す。
9. The 資産ドメイン調査 shall 沈黙ルール対応表のうち本ドメインの項目に触れる 44 行を全数読み、そのうち「未実装／語彙記録／M1 非受理／非追従」と書かれた 16 行に対応する台帳の項目を漏らさず登記する。

### Requirement 3: 未知の記述の扱いの登記
**Objective:** 将来の実装者として、定義ファイル種別ごとに「読まれない記述がどう扱われるか」が 1 か所に書かれていてほしい。それにより気づかれないまま落ちている項目が残らない。

#### Acceptance Criteria
1. The 資産ドメイン調査 shall 定義ファイル種別ごと（ゴーストの descript／シェルの descript／surfaces.txt／バルーンの descript／install.txt／プラグインの descript／ヘッドラインの descript／updates2.dau の 8 種）に 1 節を設け、未知の記述に出会ったときの現在の扱いを「黙って捨てる」「記録を残す」「エラーにする」のいずれか 1 つに分類して `doc/ukadoc-coverage/briefing-assets.md` に書く。
2. The 資産ドメイン調査 shall 各節の分類に file:line の根拠を添える。
3. The 資産ドメイン調査 shall `areka-parsers` に記録を残す経路が 1 つしか無いこと（`crates/areka-parsers/src/package/resolve.rs:296`）と、それ以外の経路がすべて無言であることを明記する。
4. The 資産ドメイン調査 shall 当たり判定の `collisionex` が何も記録せずに読み飛ばされること（`crates/areka-parsers/src/shell/decode.rs:234-236`）を、対応する台帳の項目の備考に file:line 付きで登記する。
5. When 項目の扱いが「黙って捨てる」に当たる, the 資産ドメイン調査 shall その項目が上流契約 要件 4.7 の壊れ方 ⑴（黙って壊れる）に当たるかを判定し、判定の根拠として「どの記録が出るか・出ないか」を備考に書く。
6. The 資産ドメイン調査 shall 各節に「その記述を読むのは誰か」（転記層で止まるのか、下流のどのエンジンまで届くのか）と「成立に要る基盤は何か」（例: オーナードローメニュー・更新機構）を書き添える。
7. The 資産ドメイン調査 shall この登記のために areka の実行時の振る舞いを変えない（記録を増やす変更・分類を足す変更はしない）。

### Requirement 4: SERIKO/MAYUNA の世代別対応表
**Objective:** 互換ベースウェアの開発者として、`descript_shell_surfaces` 137 項目が SSP のどの世代の機能で、areka がどこまで追えているかを一覧で見たい。それにより「完全マップ」という目標と実物の差が数で分かる。

#### Acceptance Criteria
1. The 資産ドメイン調査 shall `doc/ukadoc-coverage/briefing-assets.md` に `descript_shell_surfaces` 137 項目の世代別対応表を 1 節として置き、各行に「項目 id・見出し・登場した版・areka の状態」を載せる。
2. The 資産ドメイン調査 shall 表の版番号を台帳の登場した版の欄から取り、台帳と食い違わせない。
3. The 資産ドメイン調査 shall 間隔語のうち実際にアニメーションを駆動するのは `random` と `bind+random` の 2 語だけであり `bind` は駆動しないことを、転記側（`crates/areka-parsers/src/shell/decode.rs:385-397`）と駆動側（`crates/areka-seriko/src/table.rs:105-136`）の file:line 付きで表に記す。
4. The 資産ドメイン調査 shall 合成メソッド語について、実導出が `overlay` のみであること（`crates/areka-emo-compose/src/method.rs:129-131`）を各項目の状態に反映する。
5. The 資産ドメイン調査 shall 当たり判定が矩形のみであることを反映し、円・楕円・多角形の項目を未対応として登記する。
6. The 資産ドメイン調査 shall 対応表の縮小の出所として `doc/emo2-conformance-scope.md:82` を引用する。
7. The 資産ドメイン調査 shall 見出しが `bind` で重複する 2 項目を、id で区別したまま別々の行に載せる。

### Requirement 5: nar インストールとネットワーク更新の導線ブリーフィング
**Objective:** 製品化を判断する開発者として、「既存ゴーストを入手して入れて、あとから更新する」までに何が要るかを順に並べたものがほしい。それにより M2 の入口となる機能の大きさを見積もれる。

#### Acceptance Criteria
1. The 資産ドメイン調査 shall `doc/ukadoc-coverage/briefing-assets.md` を人手で書く文書として作り、機械で再生成しない（上流契約 要件 7.7 が手編集を禁じているのは `doc/ukadoc-coverage/report/` 配下の報告であり、本文書はそれに当たらない）。
2. The 資産ドメイン調査 shall 導線を「入手 → 展開 → 配置 → 起動 → 更新 → 削除」の順に並べ、各段に必要な正典項目の id を列挙する。
3. The 資産ドメイン調査 shall 各段について、最小成立要件（install.txt の解釈・zip の展開・配置の規則・updates2.dau の照合・delete.txt・更新イベントとの繋がり）と、areka の現状（実装ゼロ）を対比して書く。
4. When 導線が本ドメイン外の正典項目に依存する, the 資産ドメイン調査 shall その項目を台帳の関連の欄で指し、assets 台帳へ項目そのものを複製しない。
5. The 資産ドメイン調査 shall 少なくとも、さくらスクリプトのインストール実行タグと更新データ作成タグ（いずれも sakura-script 台帳）および `OnUpdate` 系のイベント（shiori 台帳）への関連を持つ。
6. The 資産ドメイン調査 shall areka 側の現状を実測値として書く（`crates/` 配下で `updates2.dau`・`updates.txt`・`.nar`・Rust から参照される `OnUpdate` がいずれも 0 件、zip 展開の依存なし、ネットワーク入出力なし）。
7. The 資産ドメイン調査 shall install／update／nar について既存の判断記録が 1 件も無いこと（沈黙ルール対応表に該当行 0 行）と、既存の言及がいずれも「対象外」の宣言であることを明記する。
8. The 資産ドメイン調査 shall 導線に要る機能を M2 の候補として並べるだけとし、実装方式（展開ライブラリの選定など）を決めない。
9. The 資産ドメイン調査 shall nar の作成側（配布者向けの機能）を導線の対象外候補として区別し、台帳には優先度だけを付ける。
10. The 資産ドメイン調査 shall 将来 spec を切り出すときの自然な境界を 3 つ挙げて記す:「定義ファイルの解釈（既存の転記層の拡張）」「配布と更新（新しい基盤）」「surfaces.txt の SERIKO/MAYUNA（単独で 1 本になる大きさ）」。境界を挙げるだけで spec は起票しない。

### Requirement 6: 実装済みの証拠をソースに置く
**Objective:** 台帳を読む人として、「実装済み」と書かれた項目の根拠がソース側に実在することを確かめたい。それにより根拠が古びたまま気づかない状態を避けられる。

#### Acceptance Criteria
1. When 項目の状態を `implemented` とする, the 資産ドメイン調査 shall その項目の定義箇所に正典 URL 1 行の doc コメントを置く（上流契約 要件 5.1〜5.3 の書き方）。
2. The 資産ドメイン調査 shall URL を定義箇所だけ（許可表の要素・分岐の腕・語彙表の 1 行）に置き、呼び出し側には置かない。
3. The 資産ドメイン調査 shall 1 項目につき 1 行・説明文を伴わない書き方に従い、定義行 1 行ずつを超える増量をしない。
4. Where 正典の名前をそのまま並べた語彙表である, the 資産ドメイン調査 shall 表の先頭にページ URL を 1 つ置く書き方を使ってよい。
5. When 見出しが複数のページで重複する（40 群・105 件）, the 資産ドメイン調査 shall 名前による突き合わせに頼らず、その項目の id に対応する URL をそのまま書く。
6. Where 項目が未実装である, the 資産ドメイン調査 shall ソース側に何も書かない。
7. The 資産ドメイン調査 shall 置いた URL がスナップショット（またはカタログ）の URL と 1 文字も違わないことを確かめる。
8. The 資産ドメイン調査 shall doc コメントを追加した前後で既存テストの結果を変えない（追加できる変更の範囲そのものは要件 10.1・10.2 が定める）。
9. The 資産ドメイン調査 shall URL を置く対象を本ドメインの項目に限り、他ドメインの項目の定義箇所には置かない。

### Requirement 7: 隣接 spec の担当の取り込みと二重起票の禁止
**Objective:** 開発者として、既に担当が決まっている項目が調査によってもう一度起票されることを避けたい。それにより同じ作業が 2 か所で数えられない。

#### Acceptance Criteria
1. When 項目を既存 spec が担当している, the 資産ドメイン調査 shall その spec 名を担当の欄に書き、新しい追跡先を作らない。
2. The 資産ドメイン調査 shall 少なくとも次の担当を台帳へ取り込む: `areka-P0-balloon-canon-residue`（バルーンの残語彙）・`areka-P0-surfaces-basepos`（`point.basepos.x`／`.y`）・`areka-P0-text-decoration-canon`（バルーン descript の `font.*` 系）・`areka-P0-anchor-tag-canon`（`anchor.*.font.*` 系）・`areka-P0-choice-marker-styling`（バルーン descript の `cursor.*`）・`areka-P0-charset-canon`（`shiori.encoding`／`shiori.forceencoding` と surfaces.txt のファイル別 `charset`）・`areka-P0-scope-zorder-pinning`（シェル descript の `seriko.zorder`）・`areka-P0-windowposition-limit`（ゴースト側 `windowposition.*`）・`areka-P0-kero-balloon`／`areka-P0-balloon-visibility`／`areka-P0-balloon-vertical-canon`／`areka-P0-balloon-offset-dpi`（バルーンの系列名・表示寿命・単位空間）・`areka-P0-bindoption-exclusivity`（`bindoptionN.group`）・`areka-P0-package-mount`（ゴースト descript の起点と install.txt の対象外宣言）・`areka-P0-shell-parse`／`areka-P0-balloon-parse`（転記層の範囲）。
3. When `areka-P0-balloon-canon-residue` の残語彙を取り込む, the 資産ドメイン調査 shall 当該 brief が番号を付けている 12 項目すべてを対象とし、当該 brief の Scope 行が書く「10 項目」を採らない（項目 11・12 は 2026-08-29 に追加され、Scope 行が追随していない）。
4. If 調査の過程で隣接 spec の brief の記述に誤りが見つかる, then the 資産ドメイン調査 shall その brief を書き換えず、是正候補として `doc/ukadoc-coverage/briefing-assets.md` に記録する。
5. The 資産ドメイン調査 shall 担当が決まっていない項目の担当の欄を空のままにし、担当の割り当てを `ukadoc-coverage-roadmap` に委ねる。
6. When 名前が同じで id が異なる項目が別ドメインにある, the 資産ドメイン調査 shall それらを同一視せず、備考に区別を書く（例: 本ドメインの descript の `homeurl` と、shiori ドメインの SHIORI リソースの `homeurl`）。

### Requirement 8: テーマの付与と優先度の仮置き
**Objective:** 統合担当として、項目ごとに「伺からしさ」のテーマと仮の優先度が付いていてほしい。それにより束ごとの段階付けを機械の集計から始められる。

#### Acceptance Criteria
1. The 資産ドメイン調査 shall テーマの欄に上流契約 要件 4.4 が凍結した 8 つの名前（気配・触れ合い・掛け合い・装い・記憶・交わり・気配り・更新）以外を書かない。
2. The 資産ドメイン調査 shall テーマの付与を「この項目が無いと利用者はゴーストの何を失うか」に答えられる場合だけに限り、答えられない項目のテーマの欄を空にする。
3. When 項目が更新機構に属する（`spec_update_file` の 9 項目・`descript_install` の `refresh`／`refreshundeletemask`／`*.refresh`／`*.refreshundeletemask` の 4 項目・`manual_update` の計 14 項目）, the 資産ドメイン調査 shall テーマに「更新」を含める。
4. Where 項目が配布者向けの機能である（`dev_nar`・`dev_update`・`dev_ownerdraw`・`dev_shell`・`dev_bind`・`dev_shell_error`・`memo`）、またはプラグインもしくはヘッドラインの descript である、またはトランスレータである, the 資産ドメイン調査 shall テーマを付けない扱いを既定とし、外れる場合はその理由を備考に書く。
5. The 資産ドメイン調査 shall 優先度の仮置きを上流契約 要件 4.7 の 4 つの根拠と固定した序列（壊れ方 ＞ テーマ ＞ 影響する既存資産の広さ ＞ 依存する基盤の共有度）で行う。
6. When 本 spec が完了する, the 資産ドメイン調査 shall 状態が `alias` と `not-applicable` 以外のすべての項目に、段階 1 文字（A〜E）＋数値の形の優先度を付ける。
7. The 資産ドメイン調査 shall 段階の最終順序を決めず、決定を `ukadoc-coverage-roadmap` に委ねることを台帳の冒頭に記す。
8. The 資産ドメイン調査 shall 各項目の備考に、壊れ方の段を選んだ根拠を書く。

### Requirement 9: 報告の再生成と整合検査
**Objective:** 開発者として、台帳が形式と整合の検査を通っている状態で完了してほしい。それにより後続の統合が壊れた台帳の上に立たない。

#### Acceptance Criteria
1. Where 上流の道具が着地している, the 資産ドメイン調査 shall `doc/ukadoc-coverage/report/assets.md` を道具で再生成し、台帳と同じコミットに含める。
2. Where 上流の道具が着地している, the 資産ドメイン調査 shall ワークスペースの標準テスト実行が通ること（上流契約 要件 6 の整合検査と 要件 7.4 の報告と台帳の一致検査を含む）を完了の条件にする。
3. If 本 spec の完了時点で上流の道具がまだ着地していない, then the 資産ドメイン調査 shall 報告の再生成を行わず、台帳とブリーフィングだけを成果物とし、その旨と再生成が要ることを `doc/ukadoc-coverage/briefing-assets.md` の冒頭に書く。
4. The 資産ドメイン調査 shall 全体の報告 `doc/ukadoc-coverage/report/summary.md` を作らず、更新もしない。
5. The 資産ドメイン調査 shall 束の解説 `doc/ukadoc-coverage/linkage.md` を作らず、更新もしない。
6. The 資産ドメイン調査 shall 報告ファイルを手で編集せず、食い違いは再生成で解消する。
7. When 関連を書く, the 資産ドメイン調査 shall 種別を上流契約 要件 4.3 の 6 つ（`alias_of`／`supersedes`／`triggers`／`configures`／`queries`／`same-feature`）に限り、相手の id がカタログに実在するものだけを指す。

### Requirement 10: 非接触と非重複
**Objective:** 並走する他の spec の担当者として、この調査が自分の作業に触れないと確信したい。それにより 4 本の調査を同時に進められる。

#### Acceptance Criteria
1. The 資産ドメイン調査 shall areka の実行時の振る舞いを変えない（`crates/` への変更は要件 6 の doc コメントの追加だけ）。
2. The 資産ドメイン調査 shall 編集する対象を `doc/ukadoc-coverage/ledger/assets.toml`・`doc/ukadoc-coverage/report/assets.md`・`doc/ukadoc-coverage/briefing-assets.md`・`crates/` 配下の doc コメントの 4 つに限る。
3. The 資産ドメイン調査 shall 他の 3 つの台帳（`shiori.toml`・`sakura-script.toml`・`property.toml`）と、カタログ・テーマ定義・`doc/ukadoc-coverage/README.md` を変更しない。
4. The 資産ドメイン調査 shall `.kiro/steering/roadmap.md` を変更しない。
5. The 資産ドメイン調査 shall `doc/COMPAT_ARCHITECTURE.md` を変更しない（沈黙ルール対応表への追記は実装する spec の仕事であり、調査 spec は読むだけである）。
6. The 資産ドメイン調査 shall 隣接 spec の brief・requirements・design を変更しない。
7. The 資産ドメイン調査 shall 台帳・報告・ブリーフィングで使う日本語を平易な語に限り、プロジェクト内でしか通じない言い回しを持ち込まない。
8. The 資産ドメイン調査 shall 上流契約が凍結した台帳の項目形式・状態語彙・ページの割り当て・仕訳の規則を変更しない（変更が要ると判断した場合は、変更せずに是正候補として記録する）。
