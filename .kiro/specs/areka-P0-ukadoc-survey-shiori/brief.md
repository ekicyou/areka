# Brief: areka-P0-ukadoc-survey-shiori

> 起票: 2026-09-02（`/kiro-discovery` Path D・ukadoc 網羅調査 5 本の 2 本目）。
> **種別**: 調査 spec（台帳＋ブリーフィング節・実行時コード非接触）。`ukadoc-survey-toolkit` の道具で `doc/ukadoc-coverage/ledger/shiori.toml` を書く。
> **所有範囲＝「ベースウェアと SHIORI／外部の対話面」**: SHIORI Event 290・Event Ex 168・Resource 159・Plugin Event 19・memo 1・SHIORI/3.0 ヘッダ 26・外部連携プロトコル 14（SSTP 2・FMO 6・x-ukagaka-link 3・DLL 共通／PLUGIN／HEADLINE 各 1）＝**677 項目**。

## Problem

areka が SHIORI へ送るイベントは emo2 が使う分だけで組まれており、正典 290 本（＋拡張 168・Resource 159）のどれを送っていて、どれを送っていないのかが台帳化されていない。既存ゴースト資産（里々／YAYA 製）は SSP が送るイベントの存在を前提に辞書を書いているため、**送られないイベントは「反応が無い」という形で静かに壊れる**。Resource（メニュー文言・推奨サイト・既定位置・`getaistate` 等）はオーナードローメニューやゴースト管理 UI の入力であり、M2 の UI 面の要件源でもある。

## Current State

### ukadoc 側（2026-09-02 実測・toolkit brief の集計と同一スナップショット）

- **list_shiori_event 290**: 先頭 80 件の内訳＝起動・終了・切替（OnFirstBoot〜OnShellChanging・OnBalloonChange）／窓状態（OnWindowStateRestore／Minimize・OnFullScreenApp*・OnVirtualDesktopChanged・OnCacheSuspend/Restore）／システム（OnInitialize・OnDestroy・OnSysResume/Suspend・OnBasewareUpdating/Updated）／入力系（OnTeach*・OnCommunicate*・OnUserInput*・inputbox.autocomplete・OnSystemDialog*）／時刻（OnSecondChange・OnMinuteChange・OnHourTimeSignal）／消滅（OnVanish*）／選択肢・アンカー（OnChoice*・OnAnchor*）／サーフェス（OnSurfaceChange/Restore・OnOtherSurfaceChange）／マウス（OnMouseClick〜OnMouseDragStart・Ex 付き）。残り 210 件はネットワーク更新・インストール・SSTP・ヘッドライン・スケジュール・通知・バルーン・その他。版番号付き 65/290。
- **list_shiori_event_ex 168**: 外部アプリ連携（OnApplication*・OnBattery*・CrystalDiskInfo・Elin・Elona おまけ・きのこ・その他プラグイン由来）。版番号 0/168＝SSP 本体機能ではなく**プラグイン／外部アプリが送るイベントの目録**。製品品質への寄与は低く、分類は「対象外（外部送信元）」を基本とし、SSTP/PLUGIN 経由の**受け口が存在すれば通る**種として一括扱いする候補。
- **list_shiori_resource 159**: `version`／`craftman(w)`／`name`／`homeurl`／`username`／`sakura.defaultx` 族（`char*.` 汎用形）／`*.recommendsites`／`*.portalsites`／各ボタン caption／`popupmenu.*`／`getaistate(ex)`／`legacyinterface`／`menu.*.bitmap.filename`／`menu.*.font.color.*` など。**オーナードローメニュー（roadmap M2 予約）の要件源**。版番号 8/159。
- **list_plugin_event 19**: `version`／`installed*`／`*pathlist`／OnGhostBoot／OnGhostExit／OnMenuExec／OnInstallComplete／`property.get`／`property.set` 等＝PLUGIN ホスティング（M2 予約）を建てるときの受け口目録。
- **spec_shiori3 26**: ヘッダ（`Reference*`・`Sender`・`SenderType`〔2.5.05〕・`SecurityLevel`・`SecurityOrigin`・`Charset`・`BaseID`・`X-SSTP-PassThru-*`・`MarkerSend` 等）。
- **外部連携プロトコル 14**: SSTP/1.x request/response・FMO（名前と文字コード・サイズ・データ本体・終端・ID・キー/値）・`x-ukagaka-link` 3 種（event／install／homeurl）・DLL 共通仕様（Global Memory）・PLUGIN/2.0・HEADLINE/2.0。

### areka 側（2026-09-02 実測・file:line は着手時に再検証すること）

- **送出イベント 11 本**（`areka-kanade/src/schedule/events.rs:76-88` `ALLOWED_EVENT_IDS`）: `OnInitialize`／`OnFirstBoot`／`OnBoot`／`basewareversion`／`OnSecondChange`／`OnClose`／`OnMouseMove`／`OnMouseDoubleClick`／`OnChoiceSelectEx`／`OnChoiceSelect`／`OnChoiceTimeout`。＋`\q` 由来の任意名（`On` 始まりのみ許可・:104）＋`OnMenuBack`（`msg.rs:511`）。恒久禁止 `OnTalk`／`OnHour`（emo2 が内部生成・:70-72）。**正典 290 に対し約 4%**。
- **照会リソース 1 本**（`schedule/resources.rs:31`＝`username`）。正典 159 の語彙は sylphya `SHIORI_RESOURCE_IDS` に逐語登記済み（件数固定テスト有り）＝**語彙のみ・実照会は 1 本**。`OnMenuExec` は不在。`\![raise]` はテスト文字列のみで本番ディスパッチ先なし。
- **ヘッダ組立は 1 点**（`shiori-host32-host/src/shiori3.rs:92` `build_request`）: request line・`Charset`（UTF-8 固定）・`Sender`・`Status`・`ID`・`Reference0..N`・`SecurityLevel: local`（固定）。**未送出**＝`SenderType`／`SecurityOrigin`／`X-SSTP-PassThru-*`／`BaseID`（:86-87 に明記・`BaseID` は crates 全体で 0 件）。応答側は `Value`／`ErrorLevel`／`ErrorDescription` のみ解釈・`Reference*`／`Marker`／`ValueNotify`／`BalloonOffset`／`Age`／`MarkerSend` は読み飛ばし（:219）。
- **正典側カタログ**: `doc/shiori/fragments/events/*.toml` 287 entry（`On*` 261）・`resources/*.toml` 159 entry・`_shared.toml` の予約ヘッダ（request 10／response 13）。**スナップショット 290 との差 3 件は本 spec で id 単位に突合する**（Ex 168・plugin 19 は fragments に無い＝新規）。
- **外部連携**: SSTP 実装なし（balloon の `sstpmessage.*` を未知キーとして無視するテストと「送出しない」の 1 行のみ）・FMO 0 件・SAORI 0 件（M1 から明示削除・`doc/emo2-conformance-scope.md:83`）・HEADLINE／PLUGIN は sylphya の根枝名と caption 語彙のみ。

## Desired Outcome

- 677 項目すべてに `status`（implemented／vocabulary-only／degraded／absent／not-applicable）と根拠 file:line、担当 spec（既存 or 新規候補）、優先度が付き、`unclassified` が 0。
- 各イベントの**発火条件の源**（descript キー・プロパティ・タグ・OS 事象）が `links` に登記され、`ukadoc-coverage-roadmap` が繋がり評価に使える。
- ブリーフィング節（`doc/ukadoc-coverage/briefing-shiori.md`）: 「既存ゴースト資産が黙って壊れる」順に並べた未実装イベント群と、その群を成立させる最小の基盤（例: 時刻イベント群＝タイマー 1 本／窓状態群＝OS 事象購読／ネットワーク更新群＝assets 側の更新機構）。

## Approach

- toolkit が凍結した仕訳規則（最新優先・新書式正典・旧書式 alias・版番号＝世代・種別付き links）を適用する。本ドメインの alias 例＝Resource の `sakura.*`／`kero.*` 固有形と `char*.*` 汎用形（汎用形を正典・固有形を alias）・`OnMouseClick` と `OnMouseClickEx`（Ex は alias ではなく後継＝`supersedes`・両方送る SSP 挙動を note に残す）・`X-SSTP-Return-`（廃止予定）と `X-SSTP-PassThru-`。
- **着手条件**: toolkit の要件確定（台帳形式の凍結）後・実装完了を待たない。他 survey と並走。
- 分類軸を固定してから埋める: ⑴ 送信方向（baseware→SHIORI／SHIORI→baseware〔Resource〕／plugin／SSTP 経由） ⑵ 発火源（ライフサイクル・時刻・入力・窓/OS・ネットワーク・外部アプリ） ⑶ SSP 世代（版番号） ⑷ 依存基盤（無いと群ごと不成立の機構）。
- Ex 168 は群単位で一括分類（外部送信元＝受け口の有無だけを問う）・個別評価しない。
- Resource 159 はオーナードローメニュー／ゴースト管理 UI の要件源として、UI 面の M2 候補 spec への入力に整形する。
- areka の証跡は toolkit の evidence スキャン結果を起点に、人手で status を確定する。

## Scope

- **In**: 上記 677 項目の台帳・繋がり登記・ブリーフィング節。
- **Out**: 実装（1 行も書かない）・他ドメインの台帳（descript／sakura／property）・SAORI（ukadoc に独立ページ無し＝DLL 共通仕様の 1 項目として assets ではなく本 spec の DLL 行で扱う）。

## Boundary Candidates

- 「SHIORI へ送る側」（Event）と「SHIORI から引く側」（Resource）は別節。
- 外部連携プロトコル 14 は「受け口の有無」だけを判定する独立小節。

## Out of Boundary

- SSTP ポートホスティング・FMO・PLUGIN・HEADLINE の実装可否判断（roadmap M2 予約のまま・本 spec は要件源の整形まで）。

## Upstream / Downstream

- **Upstream**: `ukadoc-survey-toolkit`（台帳形式・evidence スキャン）。既存: `areka-P0-status-execution-states`（`Status` ヘッダ台帳）・`areka-P0-property-query-channels`（`property.get`/`property.set`・`\![raise]`／`\![embed]` の経路）。
- **Downstream**: `ukadoc-coverage-roadmap`。将来の M2 候補（オーナードローメニュー・時刻イベント・窓状態イベント・ネットワーク更新イベント・SSTP/PLUGIN ホスティング）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `status-execution-states`（台帳 spec・`Status` 語彙と重ねない）／`property-query-channels`（`SenderType: property`・`property.*` イベント）／`emo2-conformance-e2e`（W12・共有ファイル 0）。

## Constraints

- 台帳は `doc/ukadoc-coverage/ledger/shiori.toml` 1 ファイルのみ（他調査 spec と共有ファイル 0＝並走可）。
- 根拠は必ず file:line（toolkit の検査で実在を機械確認）。
- 「未対応」は結論として明記し、憶測の実装計画を書かない（優先度と依存基盤の指摘まで）。
