# Requirements Document

## Project Description (Input)
ukadoc（SSP 公式仕様書）のうち「ベースウェアと SHIORI／外部との対話面」に属する項目を網羅的に分類する。困っているのは areka の開発者と、既存ゴースト資産（里々／YAYA 製）を areka で動かしたい利用者である。現状 areka が SHIORI へ送るイベントは適合ゴースト emo2 が使う分だけで組まれており、正典のどれを送っていて、どれを送っていないのかが台帳化されていない。既存ゴーストは SSP が送るイベントの存在を前提に辞書を書いているため、送られないイベントは「反応が無い」という形で静かに壊れる。本 spec は上流の `areka-P0-ukadoc-survey-toolkit`（要件承認済み）が凍結した台帳の契約に従って `doc/ukadoc-coverage/ledger/shiori.toml` を人手で埋め、ブリーフィング文書 `doc/ukadoc-coverage/briefing-shiori.md` を書き、未分類を 0 にする。areka の実行時の振る舞いは 1 行も変えない。

## Introduction

本 spec は「ukadoc 網羅調査」6 本のうち 2 本目の**調査 spec** である。上流 `areka-P0-ukadoc-survey-toolkit` が凍結した契約（台帳の項目形式・状態語彙・仕訳の規則・テーマ 8 つ・優先度の 4 軸・実装済みの証拠の置き方）を**継承して適用する側**であり、契約そのものは作らない。

作るものは 3 つある。

1. **台帳** `doc/ukadoc-coverage/ledger/shiori.toml` — 担当する 677 項目すべてに状態・世代・担当 spec・優先度の仮置き・テーマ・繋がり・備考を書き、未分類を 0 にする。
2. **ドメイン別報告** `doc/ukadoc-coverage/report/shiori.md` — 上流の道具が台帳から機械で作り直すもの。道具が着地した後に、台帳の持ち主である本 spec が再生成して置く。
3. **ブリーフィング文書** `doc/ukadoc-coverage/briefing-shiori.md` — 人が読むための文書。「既存ゴーストが黙って壊れる」順に未対応の群を並べ、群ごとに「それが無いと利用者は何を失うか」と「その群を成立させる最小の基盤」を書く。

これに伴う**唯一のコード接触**は、実装済みと判定した項目の定義箇所に正典 URL のコメントを 1 行置くことである（定義そのものには `///`、配列の要素や関数本体の文のように Rust が `///` を受け付けない場所では `//`。要件 9.1）。実行時の振る舞いは変えない。

### なぜ今これが要るか

areka が SHIORI へ送るイベントは 11 種類しかない。既存ゴーストの辞書は SSP が送るイベントを前提に書かれているので、送られないイベントは例外にもログにもならず、「その場面で何も喋らない」という形でだけ現れる。利用者からは不具合に見えず、開発者からは何が欠けているか見えない。どれを送っていないのかを 1 項目ずつ並べない限り、この静かな壊れ方の全体像は掴めない。

### 現状の実測（2026-09-02・本 spec 着手時にすべて再検証済み）

**正典側**（スナップショット `%APPDATA%\npm\node_modules\ukagaka-doc-mcp\data\index.json`・`generatedAt` = `2026-08-24T04:08:57.881Z`。ukadoc の 38 ページのうち 12 ページが本 spec の担当）

- `list_shiori_event` 290・`list_shiori_event_ex` 168・`list_shiori_resource` 159・`list_plugin_event` 19・`memo_shiorievent` 1・`spec_shiori3` 26・`spec_fmo_mutex` 6・`spec_web` 3・`spec_sstp` 2・`spec_dll` 1・`spec_plugin` 1・`spec_headline` 1＝**677**（全件を実測で確認。担当外の 26 ページと合わせて 1,749 を過不足なく尽くす）。
- 版番号（`2.x.y` の形）を本文に含む項目は `list_shiori_event` **78 / 290**・`list_shiori_resource` **9 / 159**・`list_shiori_event_ex` **0 / 168**（正規表現 `[0-9]+\.[0-9]+\.[0-9]+` による実測）。
- `list_shiori_event` には `On` で始まらない項目が **26 件**同居する（`basewareversion`・`property.get`／`property.set`・`hwnd`・`uniqueid`・`capability`・`installed*`・`*pathlist`・`enable_log` など）。**送る側のページに、引く側・通知側の項目が混ざっている**。
- `spec_dll` 1・`spec_plugin` 1・`spec_headline` 1・`memo_shiorievent` 1 の 4 件は**アンカーの無いページ全体で 1 項目**（id は `ukadoc:spec_dll` の形）。他ページの 1 項目より粒度が粗い。
- `spec_shiori3` は 26 項目のうち見出しが重複する組が 2 つある（`Charset` はリクエスト側とレスポンス側、`Sender` も同様）。見出しだけで突き合わせると 2 項目が潰れる。
- SAORI の独立したページは ukadoc に**存在しない**。SAORI の語が出るのは 4 項目だけで、仕様面は `spec_dll`（DLL 共通仕様）が担っている。
- 正典 URL の形は `https://ssp.shillest.net/ukadoc/manual/<ページ>.html#<アンカー>`（アンカーの無いページは末尾の `#` 以降が無い）。

**areka 側**（本ワークツリーで実測。行番号は実測値）

- 送出イベントは **11 件**（`crates/areka-kanade/src/schedule/events.rs:70`＝`ALLOWED_EVENT_IDS`・要素は `:71-81`）: `OnInitialize`／`OnFirstBoot`／`OnBoot`／`basewareversion`／`OnSecondChange`／`OnClose`／`OnMouseMove`／`OnMouseDoubleClick`／`OnChoiceSelectEx`／`OnChoiceSelect`／`OnChoiceTimeout`。これに `\q` 由来の任意名（`On` で始まるものだけ許可＝`events.rs:103-104`）が加わる。`OnTalk`／`OnHour` は恒久的に含めない（同旨の記載が `events.rs:60`）。**この 11 件は逐語一致のテストで固定されている**（`crates/areka-kanade/src/schedule/events_tests.rs:177-192`）。
- 照会リソースは **1 件**（`crates/areka-kanade/src/schedule/resources.rs:32`＝`ALLOWED_RESOURCE_IDS` = `["username"]`・逐語テストは `resources.rs:113-121`）。一方で正典 159 件の名前は `crates/areka-sylphya/src/vocab/shiori_resource.rs:45`（`SHIORI_RESOURCE_IDS`・159 要素・件数固定テストは同ファイル `:222` と `crates/areka-sylphya/src/ledger_key_determinism_tests.rs:200-205`）に**逐語で登記済み**。つまり語彙はあるが実照会は 1 件。
- ヘッダの組み立ては 1 か所（`crates/shiori-host32-host/src/shiori3.rs:91`＝`build_request`）。送るのはリクエスト行・`Charset`（UTF-8 のみ）・`Sender`・`Status`（値があるときだけ）・`ID`・`Reference0..N`・`SecurityLevel: local`（固定値・`:124`）。送っていないものについての記載が `shiori3.rs:86-87` にあり、そこに挙がるのは `SenderType`／`SecurityOrigin`／`X-SSTP-PassThru` の 3 つ（`BaseID` は挙がっていない）。`BaseID` は `crates/` のソースに 1 件も無い。
- 応答は `Value`／`ErrorLevel`／`ErrorDescription` の 3 つだけを解釈し、それ以外は読み飛ばす（`shiori3.rs:178` の `parse_response`・ヘッダの走査は `:202-218`）。
- 既にある正典側のカタログ: `doc/shiori/fragments/events/` が **287 項目**（うち名前が `On` で始まるもの 261）・`doc/shiori/fragments/resources/` が **159 項目**・`doc/shiori/fragments/_shared.toml:37-39` の予約ヘッダがリクエスト 10・レスポンス 13。`list_shiori_event_ex` に対応する断片は 1 件も無い（`doc/` 全体で 0 件）。
- 外部との連携はどれも未着手: SSTP の実装は無い（バルーンの `sstpmessage.*` を未知のキーとして無視するテストと、`shiori3.rs:86` の「送らない」の記載だけ）・FMO は `crates/` のソースに 0 件・SAORI も 0 件（M1 から明示的に外した経緯が `doc/emo2-conformance-scope.md:11` と `:83`）・HEADLINE と PLUGIN は sylphya の根の名前（`crates/areka-sylphya/src/vocab/dotted.rs:24-25`）とボタン文言の語彙だけ。
- `OnMenuExec` は `crates/` に 1 件も無い。`\![raise]` は 9 件すべてテストの中の文字列で、本番の受け手は無い（`crates/areka/src/emo2_boot/consumer_ledger.rs:221-236` が登録するのは `move`／`bind`／`set,zorder`／`reset,zorder` の 4 つ）。
- `doc/ukadoc-coverage/` はまだ存在しない。ソースに正典 URL を書いた行も 0 件（「ukadoc」の語だけを含む行は 156 件ある）。

### 上流から継承する契約（本 spec は再定義しない）

| 契約 | 出典（`.kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md`） |
|---|---|
| 台帳の 1 項目＝`[entry."<項目 id>"]` のキー付きテーブル。欄は `status`・`introduced`・`alias_of`（任意）・`supersedes`（任意）・`owner`・`priority`・`values`・`links`・`note` | 要件 2.1／付録 A |
| 状態語彙 7 つ（`implemented`／`vocabulary-only`／`degraded`／`absent`／`alias`／`not-applicable`／`unclassified`） | 要件 2.2 |
| 台帳に証拠の欄は無い（証拠は検査の出力に現れる） | 要件 2.3 |
| 担当ページの割り当てと 677 件 | 要件 3.1 |
| 新旧の書式の向きを決める順序（本文の注記 → 版番号 → 人手） | 要件 4.1 |
| 版番号が無い項目は「世代不明」（最古と決めつけない） | 要件 4.2 |
| 関連の種別 6 つ（`alias_of`／`supersedes`／`triggers`／`configures`／`queries`／`same-feature`） | 要件 4.3 |
| テーマ 8 つ（気配・触れ合い・掛け合い・装い・記憶・交わり・気配り・更新）と付与規則 1 つ | 要件 4.4〜4.6 |
| 優先度の 4 つの根拠と序列。段階 A〜E の最終決定は `ukadoc-coverage-roadmap` | 要件 4.7・4.8 |
| 実装済みの証拠＝ソースの定義箇所に置いた正典 URL 1 行の doc コメント | 要件 5.1〜5.4・5.7 |
| ドメイン別報告 `report/<ドメイン>.md` は持ち主が再生成し、台帳との一致を整合検査が確かめる | 要件 7.1・7.4 |
| 全体の報告 `report/summary.md` と `linkage.md` は統合担当の持ち物 | 要件 7.2・7.6・7.9 |
| 台帳・報告・ブリーフィングの日本語は平易な語に限る | 要件 9.5 |

## Boundary Context

- **In scope**:
  - 担当 12 ページ・677 項目の台帳 `doc/ukadoc-coverage/ledger/shiori.toml` を人手で埋め、未分類を 0 にすること。
  - 各項目の発火条件の源・関連の登記（`links`）と、伺からしさのテーマ・優先度の仮置き。
  - ブリーフィング文書 `doc/ukadoc-coverage/briefing-shiori.md`。
  - 実装済みと判定した項目の定義箇所へ正典 URL のコメントを 1 行置くこと（実行時の振る舞いは変えない）。
  - 上流の道具が着地した後の `doc/ukadoc-coverage/report/shiori.md` の再生成。
- **Out of scope**:
  - 実装（未対応の項目を送れるようにする作業は 1 行も行わない）。
  - 他ドメインの台帳（`assets.toml`／`sakura-script.toml`／`property.toml`）と、そこに属する項目の判定。
  - 段階 A〜E の最終順序・束の名付け・全体の報告・`linkage.md`（いずれも `ukadoc-coverage-roadmap` の持ち物）。
  - 台帳の項目形式・状態語彙・関連の種別・テーマの定義そのものの改訂（上流の要件の改訂を要する）。
  - SSTP のポート待ち受け・FMO・PLUGIN・HEADLINE の実装可否の判断（要件の材料を整えるところまで）。
  - 隣接 spec の brief や `.kiro/steering/roadmap.md` の書き換え。
- **Adjacent expectations**:
  - 上流 `ukadoc-survey-toolkit` は台帳の形式を要件確定の時点で凍結済みであり、本 spec は道具の実装完了を待たずに台帳を書き始める。機械の検査は後から追いつく。
  - 並走する調査 spec 3 本とは共有ファイルを持たない。本 spec の編集集合は「自分の台帳 1 本＋自分のドメイン別報告 1 本＋自分のブリーフィング文書 1 本＋ソースの URL コメント」に限られる。
  - `ukadoc-coverage-roadmap` は本 spec の台帳を、ドメインを跨ぐ束の材料として読む。本 spec は他ドメインの id を `links` の相手として指してよいが、その行は作らない。
  - `areka-P0-status-execution-states` は `Status` ヘッダの実行状態語彙を、`areka-P0-property-query-channels` は `property.get`／`property.set` と照会の経路を既に所有している。本 spec は同じ項目に担当 spec としてその名前を書くだけで、判断を上書きしない。
  - 既存の `doc/shiori/fragments/`（287＋159）と `crates/areka-sylphya/src/vocab/`（159）は置き換えない。本 spec は正典 id との対応を確かめるだけで、それらのファイルを書き換えない。

## Requirements

### Requirement 1: 担当範囲の確定と完成条件
**Objective:** 調査の担当者として、自分が埋めるべき項目の集合が最初に確定していてほしい。それにより数え直しや取りこぼしが起きない。

#### Acceptance Criteria
1. The SHIORI ドメイン台帳 shall `doc/ukadoc-coverage/ledger/shiori.toml` 1 ファイルだけを持ち、上流 要件 3.1 が本ドメインに割り当てた 12 ページ（`list_shiori_event`・`list_shiori_event_ex`・`list_shiori_resource`・`list_plugin_event`・`memo_shiorievent`・`spec_shiori3`・`spec_fmo_mutex`・`spec_web`・`spec_sstp`・`spec_dll`・`spec_plugin`・`spec_headline`）に属する 677 項目だけを収める。
2. The SHIORI ドメイン台帳 shall 項目 id をスナップショットの id と 1 文字も違わない形で写し、読みやすさのために書き換えない。
3. The SHIORI ドメイン台帳 shall 上流 付録 A の欄・型・並び順（id の文字順）をそのまま使い、独自の欄・独自の状態語彙・独自の関連の種別を追加しない。
4. When 本 spec の実装が完了する, the SHIORI ドメイン台帳 shall 状態が `unclassified` の項目を 0 件にする。
5. When 台帳の項目数を数える, the SHIORI ドメイン調査 shall ページ別の件数が上記の内訳と一致することを確かめる。
6. If 台帳のページ別件数がスナップショットの実測と食い違う, then the SHIORI ドメイン調査 shall 台帳を確定させず、食い違ったページ名と件数を示して原因を先に解消する。
7. The SHIORI ドメイン調査 shall 担当外のページに属する項目の行を台帳に作らない。

### Requirement 2: 送る側（SHIORI イベント 290）の仕訳
**Objective:** areka の開発者として、正典のイベントのうち何を送っていて何を送っていないかを 1 件ずつ知りたい。それにより既存ゴーストが黙って反応しない箇所の全体像が掴める。

#### Acceptance Criteria
1. The SHIORI ドメイン台帳 shall `list_shiori_event` の 290 項目それぞれに状態を与える。
2. Where 項目が areka の送出イベント表（`crates/areka-kanade/src/schedule/events.rs:70-82`）の 11 要素のいずれかに対応する, the SHIORI ドメイン台帳 shall その項目を `implemented` とし、要件 9 に従ってソース側の定義箇所に正典 URL のコメントを置く（台帳の行に URL は書かない・要件 9.8）。
3. Where 項目が送出イベント表に無い, the SHIORI ドメイン台帳 shall その項目を `absent` とし、`note` に「areka はこのイベントを送らない・ログにも例外にも現れない」という壊れ方の根拠を書く。
4. The SHIORI ドメイン台帳 shall `list_shiori_event` に同居する「`On` で始まらない 26 項目」（`basewareversion`・`property.get`／`property.set`・`hwnd`・`uniqueid`・`capability`・`installed*`・`*pathlist`・`enable_log` ほか）について、`note` に送信の向き（ベースウェアからの通知なのか、SHIORI から引く値なのか）を書き分ける。
5. Where 項目が `basewareversion` である, the SHIORI ドメイン台帳 shall 正典での所在が `list_shiori_event`（通知イベント）であることに従って行を置き、リソース側には置かない。
6. The SHIORI ドメイン調査 shall areka の内部でだけ使う名前（`OnTalk`・`OnHour`・`OnMenuBack`）について台帳の行を作らず、最も近い正典項目の `note` に areka 側の扱いを書く。
7. When 更新に関わる項目を仕訳する, the SHIORI ドメイン台帳 shall 名前が `OnUpdate` で始まる 26 項目を 1 つの群として扱い、群としての壊れ方・テーマ・優先度を揃える。
8. The SHIORI ドメイン台帳 shall 各イベントの発火条件の源（descript のキー・プロパティ・さくらスクリプトのタグ・OS の事象・利用者の操作）を `links` に登記する。
9. When 状態を決める, the SHIORI ドメイン調査 shall 判断の根拠として areka 側の `file:line` を `note` に書き、書く前にその場所を実際に読んで確かめる。
10. The SHIORI ドメイン台帳 shall `memo_shiorievent` の 1 項目にも状態を与え、それがイベント一覧の補足であることを `note` に書く。
11. When 台帳を確定させる, the SHIORI ドメイン調査 shall 正典の 290 項目と既存のカタログ `doc/shiori/fragments/events/` の 287 項目を項目 id 単位で突き合わせ、差の 3 件（着手時の実測では `OnArchiveViewerOpen`・`OnMediaPlayerOpen`・`OnPictureViewerOpen`。いずれも正典にあって断片に無く、逆向きは 0 件）を該当する行の `note` に記録する。

### Requirement 3: 引く側（SHIORI リソース 159）の仕訳
**Objective:** M2 の画面まわりの要件を立てる人として、正典のリソースのうち何が実際に引かれていて何が名前だけなのかを知りたい。それによりメニューや管理画面の要件源として使える。

#### Acceptance Criteria
1. The SHIORI ドメイン台帳 shall `list_shiori_resource` の 159 項目それぞれに状態を与える。
2. Where 項目が areka の照会リソース表（`crates/areka-kanade/src/schedule/resources.rs:32`）の要素に対応する, the SHIORI ドメイン台帳 shall その項目を `implemented` とする。
3. Where 項目が `crates/areka-sylphya/src/vocab/shiori_resource.rs:45` の語彙表に載っているが実際には引かれていない, the SHIORI ドメイン台帳 shall その項目を `vocabulary-only` とする。名前の突き合わせでは空白の全角・半角の違いを同一とみなす（159 件のうち `(入力ボックス種類).defaultleft　(入力ボックス種類).defaulttop` の 1 件だけが、正典は全角空白・語彙表は半角空白で写されている。残る 158 件は逐語一致）。
4. The SHIORI ドメイン調査 shall `sakura.*`／`kero.*`／`char*.*` の 3 つの形を新旧の関係として扱わず、正典本文が示すとおり「本体側・相方側・2 人目以降または `\p[*]` 側」というスコープの違いとして仕訳する。
5. The SHIORI ドメイン台帳 shall メニューやゴースト管理の画面が入力として使うリソース群（`popupmenu.*`・各ボタンの文言・`menu.*.bitmap.filename`・`menu.*.font.color.*`・`*.recommendsites`・`*.portalsites`）を `links` と `values` で束ねられる形にし、`note` にそれが何の画面の材料かを書く。
6. When 縮退している項目を見つける, the SHIORI ドメイン台帳 shall `degraded` を用い、`note` に縮退の記録の転記元（`doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表・`doc/emo2-conformance-scope.md` の見直し表）を書く。

### Requirement 4: 外部が送る拡張イベント 168 の群単位の仕訳
**Objective:** 調査の担当者として、送信元が areka ではない項目に個別評価の手間をかけたくない。それにより本当に効く 290＋159 に時間を使える。

#### Acceptance Criteria
1. The SHIORI ドメイン台帳 shall `list_shiori_event_ex` の 168 項目を 1 つの群として扱い、群に共通の状態（送信元が areka ではない〔外部のアプリ・プラグイン・他のゴースト〕ことを理由とする `not-applicable`）を与える。
2. The SHIORI ドメイン台帳 shall 群に共通の `note` を置き、そこに「送信元は areka ではない」「areka が問われるのは受け口の有無だけ」という判断の理由を書く。送信元が外部のアプリやプラグインではなく他のゴースト（`\![raiseother]` で送り、ベースウェアが運ぶ）である 3 件（`OnRequestValues`・`OnGetValues`・可変名の返信イベント）については、areka にゴースト間の伝達そのものが無いことを理由として書き添え、`list_shiori_event` 側のゴースト間のやり取り（`OnCommunicate` 群）との繋がりを `links` で示す。
3. The SHIORI ドメイン台帳 shall 168 項目の `values` を空にする（上流 要件 4.6 の付与規則に照らして答えられるテーマが無いため）。
4. Where 受け口（`\![raiseplugin]` などの任意名イベントの経路）が areka に存在しない, the SHIORI ドメイン台帳 shall その事実を群の `note` に 1 度だけ書き、168 項目それぞれに `file:line` を求めない。
5. The SHIORI ドメイン調査 shall 168 項目のうち版番号を持つものが 0 件であることを確かめ、`introduced` をすべて空にする。

### Requirement 5: ヘッダ・PLUGIN の受け口・外部連携の仕訳
**Objective:** 互換の担当者として、SHIORI/3.0 のやり取りと外部との連携について、通っている経路と通っていない経路を分けて知りたい。それにより M2 で建てる受け口の順序を決められる。

#### Acceptance Criteria
1. The SHIORI ドメイン台帳 shall `spec_shiori3` の 26 項目それぞれに状態を与え、リクエスト側の 11 項目とレスポンス側の 15 項目を `note` で区別する。
2. Where 項目が areka が現に送るヘッダに対応する（リクエスト行・`Charset`・`Sender`・`Status`・`ID`・`Reference*`・`SecurityLevel`）, the SHIORI ドメイン台帳 shall その項目を `implemented` または `degraded` とし、固定値で送っている箇所（`SecurityLevel: local`・`Charset` は UTF-8 のみ）を `note` に書く。
3. Where 項目が areka が送らないヘッダに対応する（`SenderType`・`SecurityOrigin`・`BaseID`・リクエスト側の `X-SSTP-PassThru-*`）, the SHIORI ドメイン台帳 shall その項目を `absent` とし、`crates/shiori-host32-host/src/shiori3.rs:86-87` の記載に `BaseID` が挙がっていないことを `note` に書く。
4. Where 項目が areka が読み飛ばす応答ヘッダに対応する（`ValueNotify`・`Marker`・`BalloonOffset`・`Age`・`MarkerSend`・レスポンス側の `Reference*`・レスポンス側の `Charset`・`Sender`・`SecurityLevel`・`X-SSTP-PassThru-`）, the SHIORI ドメイン台帳 shall その項目を `absent` とし、読み飛ばしている箇所（`shiori3.rs:202-218`）を `note` に書く。
4a. Where 項目が areka が現に解釈する応答に対応する（ステータスコード・`Value`・`ErrorLevel`・`ErrorDescription`）, the SHIORI ドメイン台帳 shall その項目を `implemented` とし、解釈している箇所（`shiori3.rs:178` の `parse_response`・ヘッダの分岐は `:202-218`）に要件 9 のコメントを置く。これで 26 項目すべてが 5.2〜5.4a のいずれかに当てはまる（リクエスト側は 5.2・5.3、レスポンス側は 5.4・5.4a）。
5. When 見出しが同じ項目が同じページに 2 つある（`Charset`・`Sender`）, the SHIORI ドメイン調査 shall 項目 id で区別し、1 つの行にまとめない。
6. The SHIORI ドメイン台帳 shall `list_plugin_event` の 19 項目について、イベント・PLUGIN 向けのリソース・プロパティの照会・任意名イベントの枠という種別の違いを `note` に書き分ける。
7. The SHIORI ドメイン台帳 shall 外部連携の 14 項目（`spec_sstp` 2・`spec_fmo_mutex` 6・`spec_web` 3・`spec_dll` 1・`spec_plugin` 1・`spec_headline` 1）について、実装の可否ではなく「受け口が areka にあるか無いか」だけを判定する。
8. When `spec_dll`・`spec_plugin`・`spec_headline`・`memo_shiorievent` を仕訳する, the SHIORI ドメイン調査 shall それらがページ全体で 1 項目（アンカーの無い id・4 件）であり他ページの 1 項目より粒度が粗いことを `note` に書く（`memo_shiorievent` は要件 2.10 の注記と併記する）。
9. Where SAORI について記録する, the SHIORI ドメイン台帳 shall `spec_dll` の行の `note` にそれを書き、SAORI 用の独立した行を作らない（ukadoc に SAORI の独立したページが無いため）。その `note` には、areka がプロトコルを実装せず実装の主体は SHIORI 側であること、成立の条件（32bit の同じプロセスに同居・作業ディレクトリ・DLL の探索パス）を書く。

### Requirement 6: 新旧の書式と別名の向き
**Objective:** 実装 spec の起票者として、同じ機能に複数の名前があるとき「どれを実装すればよいか」が 1 つに決まっていてほしい。それにより実装の対象数が縮み、旧い書式のゴーストも壊れない。

#### Acceptance Criteria
1. When 同じ機能に複数の名前や書式がある, the SHIORI ドメイン調査 shall 上流 要件 4.1 の順序（正典本文の注記 → 版番号 → 人手の判断）で正典と別名を決める。
2. Where 項目が別名である, the SHIORI ドメイン台帳 shall 状態を `alias` とし、`alias_of` に正典側の id を書き、実装状態の判定を正典側の行に委ねる。
3. The SHIORI ドメイン台帳 shall 正典本文が「旧仕様」と明記している `OnFileDrop`・`OnFileDropped`・`OnFileDropEx` を `OnFileDrop2` の別名として扱う。
4. The SHIORI ドメイン調査 shall `OnFileDropping` を旧仕様の一群に含めない（ドラッグ中の通知であり、別の機能であるため）。
5. When `OnMouseClick` と `OnMouseClickEx` を仕訳する, the SHIORI ドメイン台帳 shall 両者を別名の関係とせず、正典本文が示すとおりボタンの種類による分担（左右は前者・拡張ボタンは後者・中ボタンだけが重なり後者への移行が推奨される）として `same-feature` で結び、重なりを `note` に書く。
6. The SHIORI ドメイン台帳 shall 廃止予定と明記されているヘッダ（旧名 `X-SSTP-Return-`）について、その注記がレスポンス側の `X-SSTP-PassThru-` の項目に付いていることに従って登記する。
7. If brief に書かれた新旧関係の例が正典本文と食い違う, then the SHIORI ドメイン調査 shall 正典本文に従い、食い違いを `note` に明記する。
8. The SHIORI ドメイン台帳 shall 別名の連鎖を作らない（`alias_of` の指す先が `alias` であってはならない）。
9. Where 項目に版番号が無い, the SHIORI ドメイン台帳 shall `introduced` を空にし、その項目を最も古いものとして扱わない。
10. When 1 つの項目の本文に版番号が 2 つ以上ある（担当範囲に 12 件。例: `OnNetworkStatusChange`・`ValueNotify`・`*.popupmenu.applybindtoself`）, the SHIORI ドメイン台帳 shall 項目そのものの登場を示す版番号を `introduced` に書き、残りの版番号とその意味（挙動の変更・引数の追加など）を `note` に書く。どれが登場の版か本文から判別できなければ、最も小さい版番号を `introduced` に書く。

### Requirement 7: 伺からしさのテーマと優先度の仮置き
**Objective:** 統合の担当者として、各項目が「無いと利用者がゴーストの何を失うか」で色分けされていてほしい。それにより件数の多さではなく体験の重さで順序を組める。

#### Acceptance Criteria
1. The SHIORI ドメイン台帳 shall `values` に上流 要件 4.4 が凍結した 8 つのテーマ名だけを書き、独自のテーマを作らない。
2. When テーマを付ける, the SHIORI ドメイン調査 shall 上流 要件 4.6 の付与規則（「この項目が無いと利用者はゴーストの何を失うか」に答えられるテーマだけを付ける）に従い、答えられないときは空にする。
3. Where 項目にテーマを付けた, the SHIORI ドメイン台帳 shall `note` に「無いと利用者が失うもの」を 1 文で書く。
4. The SHIORI ドメイン台帳 shall テーマを付けない既定の群（`list_shiori_event_ex` の 168・`OnBasewareUpdating`／`OnBasewareUpdated`・`property.get`／`property.set`・HEADLINE）について、その理由を `note` に書く。
5. The SHIORI ドメイン台帳 shall `priority` に段階 1 文字（A〜E）と数値を仮置きし、最終決定が `ukadoc-coverage-roadmap` にあることを前提とする。
6. The SHIORI ドメイン台帳 shall 各項目の壊れ方の段（黙って壊れる／明示的なエラーになる／見た目が違うだけ）を `note` に書き、その根拠として「どのログが出るか・出ないか」を添える。
7. The SHIORI ドメイン調査 shall 優先度の 4 つの根拠の序列（上流 要件 4.7）を入れ替えない。

### Requirement 8: 繋がりの登記
**Objective:** 統合の担当者として、イベント・リソース・設定キー・タグ・プロパティの繋がりが登記されていてほしい。それにより「この束が欠けると何が壊れるか」を機械で拾える。

#### Acceptance Criteria
1. The SHIORI ドメイン台帳 shall `links` の種別を上流 要件 4.3 の 6 つに限る。
2. When 項目の発火条件に他の項目が関わる, the SHIORI ドメイン台帳 shall その相手を `links` に登記する（設定キーからの `configures`・タグや操作からの `triggers`・プロパティへの `queries`・同じ機能の別の面としての `same-feature`）。
3. Where 相手が他ドメインの項目である, the SHIORI ドメイン台帳 shall その id を `links` の相手として書いてよいが、その項目の行を本台帳に作らない。
4. The SHIORI ドメイン調査 shall `links` に書く相手の id が正典に実在することを、書く前に確かめる。
5. The SHIORI ドメイン台帳 shall 繋がりに人手の名付けや解説を持たせない（束の名付けは統合担当の `linkage.md` が持つ）。

### Requirement 9: 実装済みの証拠をソースに置く（唯一のコード接触）
**Objective:** 台帳を読む人として、「実装済み」と書かれた行の根拠がソース側に実在してほしい。それにより整理や作り替えで根拠が消えたことに気づける。

#### Acceptance Criteria
1. Where 項目の状態を `implemented` とした, the SHIORI ドメイン調査 shall その項目の定義箇所に正典 URL のコメント（`ukadoc: <正典 URL>` の 1 行）を 1 行だけ置く。定義そのもの（配列や定数の宣言・語彙表の先頭）には `///` を使い、配列の要素や関数本体の文のように Rust が `///` を受け付けない場所（`unused_doc_comments` の警告が出ることを着手時に `rustc` で実測済み）には `//` を使う。上流 要件 5.1 の「doc コメント」はこの読み替えを含むものとして扱う（上流の証拠収集は URL の文字列を探すだけなので、どちらの書き方でも証拠として拾われる）。
2. The SHIORI ドメイン調査 shall URL を置く場所を定義箇所（許可表の要素・分岐の腕・語彙表の 1 行）に限り、呼び出し側には置かない。
3. Where 正典の名前をそのまま並べた語彙表である（`crates/areka-sylphya/src/vocab/shiori_resource.rs` の 159 要素など）, the SHIORI ドメイン調査 shall 表の先頭にページの URL を 1 つ置き、要素ごとの URL は置かない。
4. Where 項目が実装済みでない, the SHIORI ドメイン調査 shall ソース側に何も書かない。
5. The SHIORI ドメイン調査 shall 説明文を伴わない 1 行のコメントだけを追加し、実行時に評価される記述（処理・分岐・値）を 1 行も追加・変更・削除しない。
6. When コメントを追加する, the SHIORI ドメイン調査 shall 既存の逐語一致のテスト（`crates/areka-kanade/src/schedule/events_tests.rs:177-192`・`crates/areka-kanade/src/schedule/resources.rs:113-121`・`crates/areka-sylphya/src/vocab/shiori_resource.rs:222`・`crates/areka-sylphya/src/ledger_key_determinism_tests.rs:200-205`）が緑のままであることを確かめる。
7. When コメントを追加する, the SHIORI ドメイン調査 shall 触れたファイルが 1 ファイル 1,000 行の上限を超えないことを確かめる（上限の見張りは `crates/log-capture-kit/tests/workspace_scan/mod.rs:38` と `crates/log-capture-kit/tests/file_length_guard_test.rs:145`）。
8. The SHIORI ドメイン台帳 shall 証拠の欄を持たない（証拠は上流の検査の出力に現れる）。
9. If ある項目を `implemented` としたいのに定義箇所が特定できない, then the SHIORI ドメイン調査 shall その項目を `implemented` とせず、`vocabulary-only` または `degraded` として `note` に理由を書く。

### Requirement 10: ドメイン別報告と整合検査
**Objective:** 開発者として、台帳と報告が食い違ったまま放置されない仕組みがほしい。それにより数字が黙って古くなることを避けられる。

#### Acceptance Criteria
1. Where 上流の道具が着地している, the SHIORI ドメイン調査 shall `doc/ukadoc-coverage/report/shiori.md` を台帳から再生成し、台帳と一緒に置く。
2. While 上流の道具がまだ着地していない, the SHIORI ドメイン調査 shall 報告が存在しないことを許し、台帳とブリーフィング文書だけで本 spec の完了を判定する。本 spec が道具より先に完了した場合、報告の初回生成は道具の着地（上流 要件 7.4 の検査が着地と同時に 4 本の報告を要求する）に伴う上流の仕事とし、完了済みの本 spec を再生成の担い手として残さない。
3. If 上流の整合検査が本ドメインについて赤になる, then the SHIORI ドメイン調査 shall 台帳側の修正または報告の再生成で解消し、報告を手で書き換えて合わせない。
4. The SHIORI ドメイン調査 shall `doc/ukadoc-coverage/report/summary.md`・`doc/ukadoc-coverage/linkage.md`・`doc/ukadoc-coverage/values.md`・`doc/ukadoc-coverage/catalog.toml`・他の 3 つの台帳を編集しない。
5. When 台帳を確定させる, the SHIORI ドメイン調査 shall 台帳に書いた状態語彙・テーマ名・関連の種別が上流の凍結した語彙のいずれかであることを確かめる。

### Requirement 11: ブリーフィング文書
**Objective:** 開発の順序を決める人として、台帳の数字ではなく「利用者に何が起きるか」で読める文書がほしい。それにより次に何を作るかを判断できる。

#### Acceptance Criteria
1. The ブリーフィング文書 shall `doc/ukadoc-coverage/briefing-shiori.md` として置かれる。
2. The ブリーフィング文書 shall 未対応の項目を群にまとめ、「黙って壊れる」ものを先に、次にテーマの付いたものを、という順で並べる。
3. When 群を書く, the ブリーフィング文書 shall 群ごとに「利用者に何が起きるか（何を失うか）」「その群を成立させる最小の基盤」「台帳の項目 id」を書く。
4. The ブリーフィング文書 shall 未対応であることを結論として明記し、憶測の実装計画（設計・工程・見積り）を書かない。
5. The ブリーフィング文書 shall 段階 A〜E の最終順序を決めず、仮の位置づけであることを明記する。
6. The ブリーフィング文書 shall プロジェクトの内輪でしか通じない言い回しを使わず、平易な日本語で書く。

### Requirement 12: 非接触・非重複の境界
**Objective:** 並走している他の spec の担当者として、この調査が自分の作業に影響しないと確信したい。それにより 4 本が同時に進められる。

#### Acceptance Criteria
1. The SHIORI ドメイン調査 shall areka の実行時の振る舞いを変えない（追加するのは要件 9 のコメント行だけ）。
2. The SHIORI ドメイン調査 shall 他ドメインの台帳・報告・ブリーフィング文書を編集せず、他ドメインの項目の行を作らない。
3. Where 既存の spec（`areka-P0-status-execution-states`・`areka-P0-property-query-channels` ほか）が同じ項目を所有している, the SHIORI ドメイン台帳 shall `owner` にその spec 名を書くだけとし、その spec の判断を上書きしない。
4. If 既存の spec の brief に正典と食い違う記述を見つける, then the SHIORI ドメイン調査 shall その brief を書き換えず、是正の候補として `note` またはブリーフィング文書に記録する。
5. The SHIORI ドメイン調査 shall `.kiro/steering/roadmap.md` を変更しない。
6. The SHIORI ドメイン調査 shall `doc/shiori/fragments/` と `crates/areka-sylphya/src/vocab/` を書き換えず、正典 id との対応を確かめるだけにする。
7. The SHIORI ドメイン調査 shall ukadoc の本文を repo に取り込まず、台帳に写すのは項目 id・状態・版番号・関連・備考に限る。

---

## 付録: brief の記載と実測の食い違い（着手時の再検証・2026-09-02）

本 spec の brief に書かれた正典の列挙と areka 側の場所を、ukadoc のスナップショットと本ワークツリーで引き直した結果。**台帳には下表の「実測」を書く。**

### 正典側

| # | brief の記載 | 実測 | 扱い |
|---|---|---|---|
| 1 | ページ別件数 12 ページ・合計 677 | 全件一致（677） | そのまま採用 |
| 2 | 版番号付き `list_shiori_event` 65／290・`list_shiori_resource` 8／159 | **78／290**・**9／159**（`[0-9]+\.[0-9]+\.[0-9]+` による実測）。`list_shiori_event_ex` の 0／168 は一致。担当 12 ページ全体では **98 件**（上記のほか `list_plugin_event` 2・`spec_shiori3` 4・`spec_dll` 1・`spec_fmo_mutex` 2・`spec_sstp` 1・`spec_web` 1）。版番号を 2 つ以上含む項目は **12 件** | 数え方の違いの可能性はあるが brief の値は再現しなかった。台帳では版番号を項目ごとに写すので合計値は使わない。複数の版番号の書き方は要件 6.10 |
| 3 | `OnTeach` 系 3 | 実在は `OnTeach`・`OnTeachStart`・`OnTeachInputCancel`。**`OnTeachInput` は存在しない** | 実在する 3 つで登記 |
| 4 | `basewareversion` は照会リソース | **`list_shiori_event` の通知イベント**（リソースのページには無い） | 要件 2.5 |
| 5 | `OnMenuExec` は SHIORI イベント | **`list_plugin_event` にだけ存在** | 要件 5.6 |
| 6 | `OnUpdate` 系 24（内訳 12＋Check 4＋Other 9） | **26**。`OnUpdatedataCreating`／`OnUpdatedataCreated` が欠落。Other 系は **8**（brief の内訳 12＋4＋9＝25 が合計 24 と合わない） | 要件 2.7 は 26 を対象とする |
| 7 | `OnMouseClickEx` は `OnMouseClick` の後継（両方送る） | ボタンの種類による**分担**。左右は `OnMouseClick`、拡張ボタンは `OnMouseClickEx`、中ボタンだけが重なり移行が推奨される | 要件 6.5 |
| 8 | Resource の `char*.*` が汎用形＝正典・`sakura.*`／`kero.*` が別名 | **新旧の関係ではない**。`sakura.`＝本体側・`kero.`＝相方側・`char*.`＝2 人目以降または `\p[*]` 側。廃止・旧仕様の記載は無い | 要件 3.4（別名にしない） |
| 9 | `OnFileDrop2` が正典・`OnFileDrop`／`OnFileDropped`／`OnFileDropEx` が別名 | 一致（3 つはいずれも本文が「旧仕様」で始まる）。ただし **`OnFileDropping` は旧仕様ではない別のイベント** | 要件 6.3・6.4 |
| 10 | `X-SSTP-Return-` は廃止予定 | 一致。注記は `spec_shiori3` の**レスポンス側** `X-SSTP-PassThru-` の項目に付く | 要件 6.6 |
| 11 | SAORI は DLL 共通仕様の 1 項目 | 一致。SAORI の独立したページは無く、言及は 4 項目のみ | 要件 5.9 |
| 12 | （記載なし） | `spec_shiori3` は見出しが重複する組が 2 つある（`Charset`・`Sender`） | 要件 5.5 |
| 13 | （記載なし） | `list_shiori_event` に `On` で始まらない項目が 26 件同居する | 要件 2.4 |

### areka 側

| # | brief の記載 | 実測 | 扱い |
|---|---|---|---|
| 14 | `ALLOWED_EVENT_IDS` は `events.rs:76-88` | **`events.rs:70`**（要素は `:71-81`） | 行番号を差し替え |
| 15 | `OnTalk`／`OnHour` の恒久禁止は `:70-72` | **`events.rs:60`** の記載（`:67`・`:97`・`:100` にも言及） | 行番号を差し替え |
| 16 | `\q` 由来の任意名は `On` 始まりのみ許可（`:104`） | 一致（判定は `:103-104`） | そのまま |
| 17 | 送出イベント 11 件の件数を固定するテストは無い | **存在する**（`crates/areka-kanade/src/schedule/events_tests.rs:177-192` が 11 要素を逐語一致で固定） | 要件 9.6（コメント追加で壊さないことを確かめる対象に加える） |
| 18 | 照会リソースは `resources.rs:31` | **`resources.rs:32`**。固定テストは `:113-121` | 行番号を差し替え |
| 19 | `OnMenuBack` は `msg.rs:511` で送出 | 出現は `msg.rs:511`・`:512`・`:522`・`:523`・`:536`・`:544` だが**すべてテストの中**（テストの塊は `msg.rs:317` から）。本番に送出箇所は無い | 要件 2.6（正典に無い名前・本番の送出も無い） |
| 20 | `build_request` は `shiori3.rs:92` | **`shiori3.rs:91`**。`SecurityLevel: local` は `:124` | 行番号を差し替え |
| 21 | 未送出の記載（`:86-87`）に `BaseID` が明記されている | **`BaseID` は挙がっていない**（挙がるのは `SenderType`・`SecurityOrigin`・`X-SSTP-PassThru`）。`BaseID` は `crates/` のソースに 0 件 | 要件 5.3 |
| 22 | 応答の読み飛ばしは `:219` | 解釈は `parse_response`（`:178`）・ヘッダの走査は `:202-218` | 行番号を差し替え |
| 23 | `doc/shiori/fragments/events/*.toml` 287 entry（`On*` 261）・`resources/*.toml` 159 entry・予約ヘッダ request 10／response 13 | 全件一致（予約ヘッダは `_shared.toml:37-39`） | そのまま。290 との差 3 件は本 spec が id 単位で突き合わせる |
| 24 | sylphya の `SHIORI_RESOURCE_IDS` に 159 件登記済み | 一致（`shiori_resource.rs:45`・件数固定は同 `:222` と `ledger_key_determinism_tests.rs:200-205`） | そのまま |
| 25 | `\![raise]` はテスト文字列のみ・本番の受け手なし | 一致（本番の受け手は `crates/areka/src/emo2_boot/consumer_ledger.rs:221-236` の 4 つで `raise` は無い） | そのまま |
| 26 | SSTP／FMO／SAORI／HEADLINE／PLUGIN は未実装 | 一致（SAORI を M1 から外した経緯は `doc/emo2-conformance-scope.md:11`・`:83`） | そのまま |
