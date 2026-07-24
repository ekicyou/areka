# Brief: areka-P0-sylphya

> **📌 2026-07-23 追記㊵陳腐化補正（本ブロックが以下の本文より優先）**: 本文の discovery は 2026-07-18＝W2 完了前。以後の確定事実へ読み替える——(1) **W1/W2 は全完了**: `sakura-dialogue-tags` ✅（本文「W1 要件討議中」は失効・R7 の sysvar 差替シーム `sysvar.rs` は実装済み＝sakura 側契約無改変で差し替え可能を実コード確認済み〔追記㉟〕）・`mayuna-compose` ✅・`input-events` ✅（「W2 mayuna が parsers に触れるため」等の並走警告は全て解消）。(2) **ウェーブ配置=W3**（追記㊵攻め再編）: 単独ウェーブでなく **`seriko-loop` ∥ `choice-render` と3本同居**——実測で互いに素（本 spec=新 crate `areka-sylphya`＋ghost `runtime.rs` provider 差替点＋main 結線のみ・kanade/sakura 側の編集は design で確定するが seriko/emo-text/emo-present とは非交差）。(3) 本文の lexer.rs/compile.rs 等の行アンカーは W1-W2 マージでドリフトの可能性＝design 時に settled main へ再突合。(4) **position-persist は W4**＝本 spec 完了後に requirements へ末尾「申し送り」節のデルタを適用してから `/kiro-design`（実施タイミングの正本は変わらず本 brief 申し送り節）。

## Problem

emo2 の撫で talk は `%username` が展開されずバルーンへ生文字列が露出する（`areka-P0-sakura-dialogue-tags` requirements.md:10）。この値を `GhostBootOptions` へ注入する案は「1 エントリだけの偽プロパティシステム」として開発者が却下済み。一方で `areka-P0-position-persist`（phase=requirements-generated・未承認）は窓位置・バルーンオフセット・起動記録・vanish 回数の 4 フィールドのために**専用のゴースト別ストア**を要件化しており（同 requirements.md:11-13）、放置すれば「名前で引ける値」という単一関心が **%系環境変数の解決器／永続ストア／IShioriHost プロパティストア（crates/areka/src/shiori_host.rs:74 に既存）** の 3 箱へ分裂する。「同じ関心を二度実装しない」というプロジェクト規律への違反が確定コースにある。

正典側の実態も単一システムを指す: ukadoc は「プロパティシステムを利用するには、環境変数と通常のタグの二つの方法がある」（list_propertysystem.html:337）と述べ、`%property[プロパティ名]` は環境変数一覧の 1 エントリ（list_sakura_script.html#_property_取得内容_）＝フラット語彙が点付き木への入口になっている。ただし「両者は単一名前空間」という正準宣言は無く（一次 HTML 全文照合済み・lineage 検証で確認）、統一は**正典と無矛盾な areka のアーキテクチャ判断**として本 spec が確立する。開発者裁定: areka は統一プロパティシステム **sylphya** を 1 つだけ作る。「プロパティ」とは名前で引けるものすべてであり、%フラット名前空間と点付き名前空間は**単一の系の二つの窓**である。

## Current State

- **字句**: `crates/areka-parsers/src/sakura/lexer.rs:270-282` `scan_sysvar` が `%英数字_` を汎用 `Token::SysVar` として捕捉→ `model.rs:56` `Instruction::SystemVar(String)`（展開なしトークン）。`\%` エスケープは実装済（`lexer_tests.rs:232/241`）。`%m?`（`?`）・`%*`（`*`）・`%property[...]`（`[`）は走査規則上トークン化不能。
- **展開器**: 存在しない。`crates/areka-sakura/src/compile.rs:117-122` が `SystemVar` を M-boot 外タグとして debug ログのみで無視。
- **値の実源**: 名前系は着地済み——`crates/areka-parsers/src/package/resolve.rs:68-72` が descript の `name`/`sakura.name`/`kero.name` を `GhostNames`（`package/model.rs:59-66`）へ転記（`sakura.name2` と `name.allowoverride` は未処理・repo 内 grep 0 件）。画面系は `crates/wintf/src/ecs/window/monitor.rs:68-74` `Monitor`（bounds/work_area/dpi・**全て物理 px**）。時計は単調時計のみ（`crates/areka-ghost/src/ticker.rs:54` `clock: Box<dyn Fn() -> MonotonicMs + Send>`・既定 GetTickCount64）で**暦時計の実源は本番コードに皆無**。OS ユーザー名読取もゼロ。
- **SHIORI 照会**: `crates/shiori-host32-host/src/shiori3.rs:87-118` `build_request` は任意 ID を特別扱いなく `ID: <id>` に組める（`shiori3.rs:454-469` の檻で固定）＝`ID: username` の GET は機構的に今日可能。欠けるのは呼び手のみ。ただし `crates/areka-kanade/src/msg.rs:80-89` の `ShioriCall` は `id: &'static str` で動的 key 照会には制約。実機 emo2/pasta は username リソース GET に **204 No Content** を返す（logfile.txt:892-902 実測）＝実機で観測される値は縮退既定値のみ。
- **想定外の既存資産**: `crates/areka/src/shiori_host.rs:74` `ShioriHostSink` が「key は SSP プロパティシステムの dotted パス」（:72）の `Mutex<HashMap<String, HSTRING>>` ストア＋`GetProperty` 同期即答（:183・欠落 key=SHIORI_E_PROPERTY_NOT_FOUND）＋`SetProperty`（:199）＋充填口 `set_property_value`（:124）を実装済み＝sylphya の COM 側 serving surface の骨格。ただし充填源未結線・SHIORI4/IShiori 経路のみ。
- **永続化**: 本番コードにファイル書込は一切無し（全て test/golden/env-gate）。`crates/areka/src/placement/mod.rs:503-565` は「ghost.dat を読まない/書かない」を檻で固定。boot cascade は毎回無条件 OnFirstBoot・Ref0="0" 固定（`crates/areka-kanade/src/schedule/events.rs:42-47`）。
- **spec 状況**: `areka-P0-sakura-dialogue-tags`（W1）は R7 で「sakura は sylphya 読み口スナップショットを消費するだけ」と既に宣言済み（requirements.md:111-120）。`areka-P0-position-persist`（W3 予定）は専用ストア所有を前提に要件生成済み・未承認。
- **serde は既にビルドに入っている**: `crates/dola/Cargo.toml:15`（derive 付き）・`crates/wintf/Cargo.toml:27`（optional）。root の `[workspace.dependencies]` に未 hoist なだけで「新規依存の承認が要る」は誤り。
- **名前衝突なし**: "sylphya"/"sylph"/"syl" は repo 内 0 件。workspace は `crates/*` glob・publish = false。

## Desired Outcome

1. 単一クレート `crates/areka-sylphya` が **単一名前空間・key モデル・読み口 API・backing trait・persistent backing** を所有し、areka に「名前で引ける値」の解決機構がこれ 1 つだけ存在する。
2. フラット（%環境変数 26 語彙）と点付き（10 ルート枝）と SHIORI Resource 系の**全語彙が完全形で第一級保持**され、M1 は源のあるものだけ実導出・残りは差替シーム付きで縮退する（下記 Scope の語彙表が正本）。
3. `%username` が実機の撫で talk で正しく展開される（emo2 は 204→既定値だが、経路は本物の SHIORI 照会 backing を通る）。
4. `areka-P0-position-persist` は sylphya の persistent backing の**消費者**へ再切削され、専用ストアの二重実装が発生しない。
5. `ShioriHostSink` のプロパティストアが sylphya の serving surface として統合され、「同じ関心の 2 箱目」が消える。
6. 消費者は値がどの backing から来たかを**構造的に**知り得ない（下記 Approach の依存グラフ帰結）。

## Approach

**クレート配置は依存グラフが強制する（検証済み）**: 消費者は sakura（`%username` 展開）・ghost（窓位置/起動記録）・kanade（vanish 回数 Ref0）。依存の頂点は areka-ghost（→ sakura, kanade, parsers, talk, actor）であり、sylphya を ghost 内へ置くと areka-sakura → areka-ghost の循環が生じるため不可能。**sylphya は areka-parsers と同格の最下層クレート**として独立させるしかない。

**最下層配置が backing シームを強制し、それが機能になる**: sylphya（最下層）は名前空間・key モデル・読み口 API・backing trait・persistent backing（std::fs のみ・上流 areka 依存なし）を所有する。SHIORI 照会 backing と live 導出 backing は、SHIORI 接続と wintf/Monitor アクセスを所有する **ghost（頂点）が据え付ける**。sakura は読み口 API（凍結スナップショット）だけを消費する。「消費者は backing を知らない」は規律で警備する事項ではなく、**依存グラフから自動的に帰結する**。

- 読み口は 2 形: (a) talk 決定論用の**凍結スナップショット**（名前→値・`StartTalk` で手渡し＝dialogue-tags R7 契約の供給側）、(b) 逐次解決 API（ghost/kanade/IShioriHost serving surface 用）。
- persistent backing は temp→rename 原子的書込・寛容読取・バージョン付き形式・ゴースト単位識別キーを sylphya 側で実装（position-persist research.md:74/159 の要件を吸収。直列化は同 research B1 案＝自前 KV＋`areka-parsers::kv` 再利用が有力・serde 採用も可＝既にビルド内）。
- 縮退の統一規則: 未解決名は「素通し（`%名前` をテキスト出力・記録）」または「既定値」へ決定論的に落ち（dialogue-tags R7.4/7.5 と同型）、点付きは NOT_FOUND を返す。全縮退は差替シーム（backing 登録）で後日実導出に置換可能。

## Scope

### In

- クレート `crates/areka-sylphya` 新設: 名前空間・key モデル・読み口 API（スナップショット＋逐次）・backing trait・backing 登録機構。
- **語彙の完全形・第一級保持**（本 brief の心臓部）。以下の表が全語彙であり、M1 はこのうち「実導出」行だけを本物の源から導出し、残りは**語彙を落とさずに**縮退させる（縮退＝key モデル上は第一級・解決時に素通し/既定値/NOT_FOUND・backing 差替シーム付き）。

**フラット語彙（ukadoc list_sakura_script.html「環境変数」節・一次 HTML L5992-6335 で全数検証済み・26 トークン＋構文 2）**

| トークン | 意味 | backing | M1 | 典拠 |
|---|---|---|---|---|
| `%month` `%day` `%hour` `%minute` `%second` | 現在月/日/時/分/秒 | live-derived（暦時計） | 縮退（素通し）——暦時計の注入シームが本番コードに無い（ticker.rs:54 は単調時計のみ）。シーム新設＝追跡宿題 | list_sakura_script.html#_month〜#_second |
| `%username` | ユーザー名 | shiori-queried | **実導出**: SHIORI Resource `username`（list_shiori_resource.html#username）への GET 照会 backing。emo2/pasta は 204（logfile.txt:892-902）→既定値縮退（areka 裁量＋対応表記録・正典は既定値未規定＝未確認） | list_sakura_script.html#_username |
| `%selfname` | 本体側の名前 | live-derived（descript `sakura.name`） | **実導出**: 源着地済み（resolve.rs:68-72→GhostNames） | #_selfname・descript_ghost.html L389-391 |
| `%selfname2` | 本体側の名前その2 | live-derived（descript `sakura.name2`） | **実導出**（parse 拡張要: resolve.rs は sakura.name2 未読取）。未定義時の展開は正典未規定→縮退規則を areka 裁量記録 | #_selfname2・descript_ghost.html L401-403 |
| `%keroname` | 相方側の名前 | live-derived（descript `kero.name`・SSP は省略時 sakura.name） | **実導出**: 源着地済み（resolve.rs:71） | #_keroname・descript_ghost.html L411-415 |
| `%screenwidth` `%screenheight` | スクリーン幅/高さ | live-derived（wintf Monitor） | 縮退——源は実在（monitor.rs:68-74）だが**物理/論理 px 契約が未確定**（2026-07-05 placement DPI 欠陥の同型ハザード）。契約確定込みの実導出は Boundary Candidate | #_screenwidth #_screenheight |
| `%exh` | OS 連続起動時間 | live-derived | 縮退（源=GetTickCount64 は実在するが表示書式が正典未規定・消費者なし） | #_exh |
| `%et` `%wronghour` | 間違った稼働時間/時刻ネタ | live-derived | 縮退（「間違い」の生成アルゴリズム正典未規定＝未確認） | #_et #_wronghour |
| `%ms` `%mz` `%ml` `%mc` `%mh` `%mt` `%me` `%mp` `%m?` `%dms` | 単語ランダム系 10（人/無機物/集合/社名/店名/技/食物/地名/非限定/〜に〜する〜） | **unknown**（候補テーブルの出所を ukadoc は一切規定しない・SHIORI Resource 一覧にも不在＝未確認） | 縮退（素通し）。`%m?` は現行 lexer で `?` がトークン化不能＝lexer 拡張も縮退扱いに含めて語彙だけ予約 | #_ms〜#_dms・節冒頭 L5995 |
| `%lastghostname` `%lastobjectname` | インストール時用（最後にイベントを行ったゴースト/オブジェクト名） | live-derived（インストール文脈） | 縮退（インストール機能自体が M1 外・文脈外の値は正典未規定） | #_lastghostname #_lastobjectname |
| `%*` | `\![*]` と同機能＝表示タグの別記法 | —（構文・プロパティではない） | 語彙表に**構文として**記録のみ（プロパティ解決の対象外） | #_*2 |
| `%property[プロパティ名]` | 点付き木への汎用ゲートウェイ構文 | 指し先次第の多態 | 点付き解決 API 自体は M1 実装。さくらスクリプト中の `%property[...]` 展開は縮退（lexer が `[` で停止・bracket 拡張は Boundary Candidate） | #_property_取得内容_・list_propertysystem.html L330,337-342 |

（注: `\%` は検証裁定により語彙から除外——環境変数ではなく「さくらスクリプトのエスケープ」節のエスケープ記法（list_sakura_script.html L601）。areka では実装済: lexer_tests.rs:232/241）

**点付き語彙（ukadoc list_propertysystem.html・一次 HTML 99,727B 全文で検証済み・ルート枝は 10 本ちょうど＝calendarlist 等は存在しない）**

| 枝/要素 | 内容 | backing | M1 | 典拠 |
|---|---|---|---|---|
| `system` | OS/HW/時刻ライブ情報（year〜dnd.mode・os.*・cpu.*・memory.*・monitor.*・power.*・disk.*・network.*・theme.*） | live-derived | 縮退（NOT_FOUND）——暦時計/HW 照会シーム未整備 | list_propertysystem.html system.* 全項 |
| `baseware` | `baseware.version`・`baseware.name` の 2 項のみ | live-derived | **実導出**（自明・点付き解決の最小実証） | 同 baseware 節 |
| `ghostlist` | インストール済み全ゴースト（括弧名選択/.index(ID)/.current/.count） | live-derived（username・shiori.変数名のみ shiori-queried） | 縮退 | 同 ghostlist 節 |
| `activeghostlist` | 起動中ゴースト＋ **ext 亜枝**（後述） | live-derived／ext は shiori-queried | 縮退 | 同 activeghostlist 節 |
| `currentghost` | 最大の枝（汎用プロパティ名・status・shelllist・scope(ID).*・balloon.*・mousecursor 群・seriko.cursor/tooltip・surfacelist）。`status` は SHIORI Status ヘッダと同一語彙の下位枝 | live-derived（username・shiori.変数名は shiori-queried） | 縮退（`name`/`sakuraname`/`keroname` の実導出は源着地済みゆえ Boundary Candidate） | 同 currentghost 節 |
| `balloonlist`／`headlinelist` | インストール済みバルーン/ヘッドライン一覧（独立した 2 本のルート枝） | live-derived | 縮退 | 同各節 |
| `pluginlist` | プラグイン一覧＋ ext 亜枝 | live-derived／ext は plugin-queried | 縮退 | 同 pluginlist 節 |
| `history` | 最近使ったもの（ghost/balloon/headline/plugin × 3 形） | persistent（永続明文は無し＝未確認） | 縮退 | 同 history 節 |
| `rateofuselist` | 使用率統計（通常/weekly/monthly 各 12 種） | persistent（週報/月報集計は永続が構造的必須） | 縮退 | 同 rateofuselist 節 |
| 汎用プロパティ名 17 種 | name/sakuraname/keroname/craftmanw/craftmanurl/path/thumbnail/update_result/update_time/homeurl/username/shiori.変数名/index/menu[SET]/sakura.bind.menu[SET]/kero.bind.menu[SET]/char*.bind.menu[SET] | リスト依存（username・shiori.変数名は shiori-queried） | 縮退（key モデルには第一級で保持） | 同 汎用プロパティ名 節 |
| セレクタ 5 形 | ①括弧名選択 ②`.index(ID)` ③`.current` ④`.count` ⑤数値括弧 `scope(ID)` 等 | —（key モデルの文法） | **M1 で key モデルに完全実装**（解決は枝ごとの backing 次第） | 同各所＋記述例 |
| SET 有効群 | `surface.num`（SET=\s[] 等価）・`animation.num`（SET=\i[] 連続等価・追加実行）・`seriko.defaultsurface`・mousecursor 群 10 項・seriko.cursor/tooltip 4 項・menu/bind.menu 4 項 | live-derived | 縮退（書込 API の**型シーム**のみ予約・実書込は M2）。SET 無効項目への書込失敗挙動は正典未規定＝areka 裁量記録 | 同各所（HTML:897 ほか） |
| ext 亜枝（所有権逆転） | `activeghostlist(...).ext.*`／`pluginlist(...).ext.*` ＝ SHIORI/PLUGIN イベント `property.get`/`property.set` を対象側に発生させて取得・設定（SSP 2.7.85） | shiori-queried（所有者がゴースト/プラグイン側） | 縮退（語彙・イベント名のみ予約） | list_shiori_event.html#property.get・list_plugin_event.html#property.get |

**SHIORI Resource 語彙（list_shiori_resource.html・dt 全 159 項目・shiori-queried backing の名前族として key モデルに第一級保持）**: SHIORI 情報 5（version/craftman/craftmanw/name/log_path）・ゴースト情報 43（homeurl/useorigin1/**username**/sakura|kero|char*.default{x,y,left,top}/recommendsites 系/popupmenu 系/getaistate(ex)/legacyinterface ほか）・更新情報 1・オーナードローメニュー画像 3＋文字色群＋ `*button.caption` 91 種＋同数の `*button.visible` ファミリ・tooltip 2。M1 実導出は **username 1 件のみ**（%username の供給源）。他は縮退（照会経路自体は build_request が任意 ID を通すため、実導出追加は backing 登録だけで済む構造にする）。特殊エントリ「-」の実リソース ID は未確認。

- **persistent backing の M1 実導出 key**（areka 独自名前空間・ukadoc は ghost.dat の中身を規定しない＝baseware 自由）: 窓位置（scope 別）・バルーン相対オフセット（scope 別）・起動記録・vanish 回数——position-persist の 4 フィールドを sylphya の key として収容し、原子的書込（temp→rename）・寛容読取・バージョン付き形式・ゴースト単位識別キーを実装する。
- 消費側結線: ghost が SHIORI 照会/live 導出 backing を据付け、`StartTalk` へ渡す凍結スナップショット（dialogue-tags R7 契約）を sylphya から生成。`ShioriHostSink`（shiori_host.rs:74）のストアを sylphya 読み口へ統合。
- 決定論檻: 全解決・縮退・凍結・往復（persistent）を注入シーム経由の決定論単体テストで網羅。

### Out

- 縮退指定の全項目の実導出（上表のとおり・各差替シームと追跡宿題を残す）。
- `\![vanish]` 実装（カウント増分の発生源＝M2）・ゴースト切替・多重ゴースト。
- SSTP EXECUTE `GetProperty`/`SetProperty`（spec_sstp.html・SSTP サーバ自体が M1 外）。
- SHIORI/PLUGIN イベント `property.get`/`property.set` の発火実装（ext 亜枝の実働・M2）。
- 単語ランダム系の候補辞書（出所自体が未確認）。
- `%username` 以外の展開を dialogue-tags 側スコープへ押し戻すこと（展開器そのものは dialogue-tags R7 の領分・sylphya は値源と読み口のみ）。

## Boundary Candidates

- `%property[...]` の lexer bracket 拡張（areka-parsers/sakura/lexer.rs 改修）: emo2 未使用・M1 消費者なしだが、点付き解決 API が M1 に在るなら安価。W2 mayuna が parsers に触れるため**ウェーブ干渉判断込み**で design 討議へ。
- `%screenwidth`/`%screenheight` の実導出: 源は実在。**物理/論理 px 契約の確定**（wintf Monitor は物理・placement DPI 欠陥の教訓）を先に済ませられるなら In へ。
- `currentghost` の `name`/`sakuraname`/`keroname` 点付き実導出（GhostNames 着地済みで安価）。
- 暦時計注入シームの新設を本 spec でやるか追跡 spec に譲るか（時刻系 5＋`system.year` 系の前提）。
- serde の workspace hoist と persistent 形式（自前 KV B1 案 vs serde 形式）の選択。
- `ShioriHostSink` ストア統合の時期（M1 で読み口を差すか・型シームだけ切って後続か）。
- `name.allowoverride`（descript_ghost.html L712）の解決規則をどの spec が持つか。

## Out of Boundary

- SSP `ghost.dat` バイナリ互換（不要・areka 自由形式）。
- ランダムサーフェス選択等の SHIORI 層責務（記憶: random surface は SHIORI/script 層）。
- 選択肢 UI・メニュー描画（choice-render/choice-select-events・W4/W5）。
- `OnTranslate` 再送などスクリプト変換パイプライン全体の再設計。
- ネットワーク系プロパティ（system.network.*）の実照会・ヘッドライン/更新系機能の実装。

## Upstream / Downstream

- **Upstream（sylphya が依存）**: なし（最下層）。候補として `areka-parsers`（KV 再利用・同格最下層ゆえ循環なし）。std::fs のみで永続化（上流 areka 依存ゼロ）。
- **Downstream（sylphya を消費）**:
  - `areka-sakura` — 凍結スナップショット（`StartTalk` 手渡し・dialogue-tags R7）。
  - `areka-ghost` — backing 据付（SHIORI 照会・live 導出）＋窓位置/オフセット/起動記録の読書き（position-persist 再切削後の結線）。
  - `areka-kanade` — OnFirstBoot ゲート判定と Reference0（vanish 回数）。
  - `crates/areka`（bin） — `ShioriHostSink` serving surface（shiori_host.rs:124 充填口の置換）。
  - 将来 — SSTP EXECUTE・`property.get`/`property.set`・`%property` 展開。

## Existing Spec Touchpoints

### Extends
- **`areka-P0-position-persist`**（未承認・W3）: ストア実体（形式・原子的 IO・寛容読取・識別キー・往復耐久）を sylphya が吸収。同 spec は復元意味論と結線に再切削（詳細は別途デルタ）。
- **`areka-P0-sakura-dialogue-tags`**（W1・要件討議中）: R7 の「sylphya 読み口スナップショット消費」契約の供給側を本 spec が実体化。
- **`ShioriHostSink` プロパティストア**（crates/areka/src/shiori_host.rs:72-74,124,183,199・「要件 10.2」系譜）: 2 箱目を作らず sylphya 読み口に統合。

### Adjacent
- host32 GET 経路（shiori3.rs:87-118・任意 ID 組立の檻 :454-469）——username 照会 backing が消費。`ShioriCall.id: &'static str`（msg.rs:80-89）の動的 key 制約は design 論点。
- wintf `Monitor`（monitor.rs:68-74・物理 px）・ticker 注入時計（ticker.rs:54）。
- W2 `input-events`（kanade＋spawn.rs）・W2 `mayuna-compose`（parsers＋sakura）——ghost/kanade/parsers の編集面が重なり得るためウェーブ直列化の対象。

## Constraints

- **決定論テスト必達**: 全判断分岐（解決・縮退・凍結・寛容読取・原子的書込）を注入シームで決定論化し x64 純粋テストで全網羅（偽境界注入・i686 常用回避）。暦時計等の新規外部依存は必ず注入シームを切る（素の SystemTime 直読禁止）。
- **ログ規律**: 無音失敗禁止・失敗は error!/warn!＋既定縮退・panic は致命限定＋直前ログ。永続化起因のいかなる失敗でも起動を停止させない（position-persist requirements.md:80 の規律を継承）。
- **正典忠実**: 正典は ukadoc（emo2 は fixture にすぎない）。正典が沈黙する箇所（既定値・SET 失敗挙動・%selfname2 未定義時等）は areka 裁量＋対応表記録。
- **語彙完全性規律**: 正典機能の先送りは「完全語彙＋縮退シーム＋追跡 spec＋roadmap 明記」の 4 点セット（素の最小化は不可）。
- **依存**: serde は既にビルド内（dola/wintf）——hoist するか自前 KV（B1）かは design で確定・「新規依存承認が要る」という前提は置かない。新規外部 crate 追加時のみ要承認。
- **環境変数**: 本番ランタイムが読む env は `AREKA_` 冠必須。
- **ウェーブ規律**: 少しでも干渉するならウェーブを分ける（共有ファイル 0・契約辺 0・ソフト依存も干渉に数える）。マージは kiro-complete の PR squash のみ。

---

# 申し送り（discovery 帰結の具体デルタ・ウェーブ配置・roadmap 宿題）

> 以下は本 discovery（2026-07-18）が確定した、既存 spec への波及デルタと配置提案。position-persist の再切削は W3（繰り下げ後）の `/kiro-design` 実行前に本節を正本として適用する。

## areka-P0-position-persist への編集（requirements.md・W3 の `/kiro-design` 実行前に必須）

**原則**: 「復元の意味論と結線」（アンカー再射影・二層分離・OnFirstBoot ゲートの運行結線・Ref0 注入・観測点消費）は position-persist に**残存**。「ストアそのもの」（永続形式・原子的 IO・寛容読取・バージョニング・ゴースト識別キー・往復耐久性）は **sylphya へ移管**。

1. **Introduction（L5）**: 現行「⓪ ghost（ゴーストエンジン）が所有する **ゴースト単位の永続化層**を実装で埋め」→ 置換意図: 永続化層の実体（ストア）は `areka-P0-sylphya` の persistent backing が所有し、本 spec は「sylphya の persistent backing を消費して窓位置復元・OnFirstBoot ゲート・Ref0 注入を**結線する**spec」へ再定義する旨を明記。
2. **Boundary Context In scope（L11）**: 現行「破損・欠損永続ファイル・保存失敗・未知形式への寛容縮退（起動を殺さない）／永続状態のゴースト単位スコープ（他ゴーストと混同しない識別キー構造）」→ 置換意図: この 2 項は sylphya の保証を**消費**する側の記述に改める（「sylphya が保証する寛容読取・識別キーの下で、失敗時に既定位置解決へ縮退して起動を継続する」）。
3. **Boundary Context Out of scope（L12）**: 追加意図: 「永続ストア実装そのもの（バージョン付き形式・原子的 IO・寛容読取・ゴースト識別キー・往復耐久）＝`areka-P0-sylphya` の領分」を明示追加。
4. **Adjacent expectations（L13）**: 現行「永続状態の保存先（…）と内部形式は areka 独自のバージョン付き形式とし、具体は design で確定する」→ 置換意図: 保存先・内部形式の確定は **sylphya の design** に移り、本 spec は sylphya の key 契約（窓位置/オフセット/起動記録/vanish 回数の 4 key 族）を消費すると書き換える。
5. **R1.1-1.3（L22-24・即時確定/終了時フラッシュ/クラッシュ耐久）**: ライター結線（ドラッグ確定→sylphya へ書く・終了時フラッシュ要求）は残存、**耐久性保証そのもの**（書込中断で旧状態非破壊等）は「sylphya persistent backing の契約に依る」と参照形へ書き換え。
6. **R1.4-1.9・R2 全体・R3 全体・R4 全体・R5 全体**: **無改変で残存**（復元優先順位・scope 分離・アンカー毎起動再解決・単一真実源・オフセット基準点・OnFirstBoot ゲート運行・Ref0・再射影＝すべて結線/意味論の領分）。
7. **R6（L72-80・頑健性）**: 6.1 は「sylphya が読めない/無いと報告したとき既定へ縮退し起動継続」（消費側挙動）へ縮小。6.2-6.3（保存失敗の非破壊・部分失敗耐久）は sylphya へ移管し参照形に。6.4（永続化起因で起動を殺さない）は残存。
8. **R7（L82-88・ゴースト単位スコープ）**: 7.1 の識別キー構造・7.2 の未知形式縮退は sylphya へ移管。残存するのは「本 spec の消費 key が当該ゴーストの識別キー下に置かれること」の消費要件のみ。
9. **R8（L90-100・検証）**: 8.1（保存→復元往復）・8.2 前半（破損/欠損の寛容縮退）は sylphya の檻へ移管。8.2 後半（アンカー再射影）・8.3-8.6（ゲート・非書き戻し・オフセット不変・実機サインオフ）は残存。

## areka-P0-sakura-dialogue-tags への編集（requirements.md・要件討議中の W1）

同 spec は既に sylphya 前提で書かれている（L35-36・L44・R7）ため、編集は「予定→確定」の同期のみ:

1. **Boundary Context L35**: 現行「＝`sylphya`（別 spec で新設）の領分」→ 置換意図: 「＝`areka-P0-sylphya`（brief 確定済・crate `crates/areka-sylphya`）の領分」へ名指しを確定形に。
2. **Adjacent expectations L44**: 現行「sylphya spec の新設と roadmap 宿題化は discovery 継続中（本 spec のブロッカーではない）」→ 置換意図: 「discovery 完了＝`areka-P0-sylphya` brief 確定。本 spec のブロッカーでないことは不変」へ更新。あわせて「W1 の暫定 provider（ghost が空/既定スナップショットを充填）は sylphya 着地時に sylphya 読み口からのスナップショット生成へ**差し替える**（sakura 側契約は無改変＝差替シーム）」を明記。
3. **R7（L111-120）**: AC は実質無改変で成立（R7.3 の「スナップショットを埋めるのは ⓪ ghost の責務」「値源は SHIORI リソース」は sylphya 移行後も真——ghost が sylphya 経由で埋める、と一段深くなるだけ）。R7.3 に「スナップショットの生成元は最終的に sylphya 読み口（凍結像）であり、sakura の契約は provider 差替で不変」の一文を追補。
4. **W1 が sylphya 不在で出荷できるもの（契約）**: スナップショット型（名前→値写像）・`StartTalk` 手渡し・`%username` 展開・既定値縮退・未解決名の素通し（R7.1-7.6）・決定論檻（R9.4）——emo2 は 204 固定ゆえ暫定 provider でも本実装でも観測は既定値で不変（logfile.txt:892-902）。**待つもの**: 実 SHIORI 照会 backing・`%selfname`/`%keroname` 等の実値解決・単一名前空間への集約・provider 差替。

## sylphya のウェーブ配置（推奨）

**W2 と W3 の間に sylphya 専用ウェーブを挿入**（現 W3 以降を 1 つ繰り下げ）——W1 でスナップショット契約が実物になった後・position-persist（現 W3）が persistent backing を消費する前に着地が必須で、ghost/kanade の据付結線が W2 `input-events`（kanade＋spawn.rs）と、parsers 候補面が W2 `mayuna`（parsers＋sakura）と干渉し得るため、全直列原則（少しでも干渉するならウェーブを分ける）により同居不可。

## roadmap 宿題

1. ウェーブ編成表（追記㉙・roadmap.md:180-189）へ sylphya ウェーブを挿入し、`position-persist` の上流充足欄に「← sylphya persistent backing」を追記・干渉ペア台帳（roadmap.md:189）へ sylphya⇄input-events〔kanade/ghost 結線〕・sylphya⇄mayuna〔parsers 候補面〕・sylphya→position-persist〔persistent backing 契約〕を追加。
2. position-persist requirements.md の上記再切削を W3（繰り下げ後）の `/kiro-design` 前に実施する宿題行。
3. dialogue-tags 暫定 provider → sylphya スナップショットへの差替宿題行（sylphya ウェーブ内タスク）。
4. 追跡 spec 群の登記（完全語彙＋縮退シームの 4 点セット則）: (a) 暦時計注入シーム＋時刻系（%month 系・system.year 系）実導出、(b) 画面系の物理/論理 px 契約確定＋実導出、(c) `%property[...]` lexer bracket 拡張＋点付き木の実導出拡充（system.*/ghostlist 系）、(d) 単語ランダム系の出所調査（backing 未確認のまま）、(e) SET 有効群の実書込（M2）、(f) SSTP EXECUTE GetProperty/SetProperty＋SHIORI/PLUGIN `property.get`/`property.set`（ext 亜枝実働・M2）。
