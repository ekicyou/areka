# Brief: areka-P0-property-query-channels

> 起票: 2026-08-27（`areka-P0-balloon-vertical-canon`〔bvc〕要件ディスカッション議題 4 の開発者指示による `/kiro-discovery` 再入・プロパティ系 3 spec 分割の 1 本目）
> **分割の要**: プロパティ正典の負荷分解線は「照会経路 vs 値の木」である。経路はどれも全か無かの欠落インフラ（イベント発生型・%埋め込み・IPC 運搬）であり、木の各枝は publish シームが 1 つあれば安い。経路を本 spec に集約することで、木側の spec が「ゴーストはこれをどう読むのか」を再審議せずに済む。

## Problem

SSP プロパティシステムの照会経路が areka に **1 本も無い**。値を sylphya（統一プロパティ機構）へ掲示できても、ゴースト・外部からそれを読む/書く手段が本番構成に存在しない——bvc ギャップ分析 §3.6-3 が「最大の未確定」として掘り当てた穴であり、`.vertical` 固有ではなくプロパティ照会全体の穴である。

## Current State

2026-08-27 実測（bvc 討議中のサーベイ・file:line は当日検証値）。

**正典の照会経路 6 本**（ukadoc）:
| # | 経路 | 応答の返り方 |
|---|---|---|
| 1 | `\![get,property,イベント名,プロパティ名,...]` | 指定イベントが発生し Reference0+ に引数順で値・`SenderType: property` |
| 2 | `\![set,property,プロパティ名,値]` | 書き込みのみ・応答なし |
| 3 | `%property[プロパティ名]` | 表示時のインライン文字列置換（バルーン表示専用） |
| 4 | `\![embed,イベント名,r0,...]` | 1 回のスクリプト実行内でイベントの Result に置換（#1 の同期消費相手） |
| 5 | `.ext.拡張プロパティ名`（`activeghostlist`/`pluginlist`・2.7.85） | **逆方向**——ベースウェアが SHIORI/PLUGIN イベント `property.get`／`property.set` を発生 |
| 6 | 非スクリプト同期読み（里々 `get_property` 関数 Mc172-1+ 等） | **輸送路が snapshot に未記載**（`EXECUTE`/`GetProperty` 0 ヒット）——設計前にライブ ukadoc/SSP 実測で確定必須 |

**areka の現状**:
- `\!` 汎用キャリアは逐語転写済みで全 sink へ届いている——`decode_passthrough_bang`（`areka-parsers/src/sakura/decode.rs:321-326`）→ `GenericCommand` → `dola CueCommand::Custom`（`dola/src/cue/command.rs:163-166`）→ 全 sink ブロードキャスト（`runtime.rs:223-224`）。**`\![get,property,…]` は今日も各 sink に届いて全員に無視されている**。消費者は CueSink 実装 1 個＋`consumer_ledger.rs:96` 1 行＋`emo2_boot/mod.rs:420` の sinks vec 1 行で立つ。⚠ 台帳はコマンド名粒度＝`"get"` 登記は全 `\![get,*]` を引き受ける（zsp の `"set"` と同じ論点）。
- **イベント発生の届け先型が無い**——`KanadeMsg`（`areka-kanade/src/msg.rs:119-156`）に Raise/汎用イベント variant が無く、`ShioriCall::Get` は全て `schedule/` 内部で組まれる。最も近い雛形＝UI スレッド発の `KanadeMsg::Choice`（`choice_drain.rs:71` → `schedule/choice.rs:60`）。任意名イベントは `EventId::Choice`（`schedule/events.rs:387-393`）が先例・egress は `On` 接頭辞で通る（`actor.rs:268-286`）。
- **`%property[...]` は字句解析できない**——`scan_sysvar`（`lexer.rs:273-285`）は `[A-Za-z0-9_]+` のみで `[` で止まる＝`%property[x]` は `SysVar("property")`＋リテラル `[x]` に割れる。`%` の角括弧引数形は新規の lexer 仕事。sysvar 解決は per-talk snapshot（`sysvar.rs:65-83`・`emo2_boot/mod.rs:426` で sylphya から供給）。
- **host32 IPC はプロパティを運ばない**——3 crate に property 0 件・ワイヤタグは閉集合（`shiori-host32-ipc/src/lib.rs:42-55`＝Hello/Load/Request/Response/Unload）。`IShioriHost::GetProperty/SetProperty`（`shiori-abi/src/interface.rs:153/:159`）の実装 3 つのうち sylphya 接続済みは env-gate デモ経由の `ShioriHostSink`（`areka/src/shiori_host.rs:247/:267`・`main.rs:194`）のみ・`InProcHost` は非接続 `RefCell<HashMap>`（`shiori_inproc.rs:276/:289`）・本番 emo2（`ShioriWiring::Helper`）には読み口が**全く無い**。
- sylphya の SET 経路の台帳が snapshot 比で 5 件古い——`SET_EFFECTIVE` 21（`vocab/dotted.rs:72`）は `seriko.zorder`・`seriko.sticky-window`（2.8.78）・サウンド SET 3 葉（2.8.72）を先取りしていない。サウンド語彙 ≈18 葉は族ごと不在。`property.get`/`property.set` の名前自体は予約済み（`dotted.rs:106-109`）。

## Desired Outcome

ゴーストが**任意の**プロパティを読み書きできる——`\![get,property]` でイベント越しに、`%property[...]` で表示に埋め込んで、`\![set,property]` で SET 有効項目に書いて。値の中身（木）は他 spec の所有だが、経路は本 spec が全部敷き、値なしは値なしとして正しく返る。

## Approach

sink 新設（get/set）＋ kanade への「参照付きイベント発生」型の新設＋ `%` 角括弧引数形の lexer 拡張＋ sylphya SET 分類の経路接続。経路 6（非スクリプト同期読み）はライブ正典の実測で輸送路を確定してから、host32 IPC 運搬の新設 or 登記付き先送りを設計で裁定。

## Scope

- **In**:
  - 経路 1〜4 の実装（get sink・set sink・`%property[...]`・`\![embed]` の対）と `SenderType: property`。
  - `KanadeMsg` への参照付きイベント発生型の新設（`Choice` 雛形）。
  - sylphya 台帳の正典追随（`SET_EFFECTIVE` 21→26・サウンド語彙族の登記・件数檻の更新）——SET 経路の所有者として。
  - 経路 6 の輸送路のライブ実測と裁定（host32 IPC 運搬 or 登記付き先送り）。
  - 経路 5（`.ext.*` 逆方向イベント）は**語彙登記のみ**（発火条件が activeghostlist/pluginlist＝多重ゴースト・プラグイン基盤に依存＝カタログ spec の解禁と連動）。
  - consumer_ledger の `"get"`/`"set"` 粒度の裁定（zsp と同型・先着の裁定に揃える）。
- **Out**:
  - 値の木の実導出（`currentghost.*`＝`areka-P0-currentghost-property-tree`／`system.*`・カタログ群＝`areka-P0-property-catalog-lists` が所有）。
  - SSTP ホスティング（M2 予約・port 9801）。

## Boundary Candidates

- スクリプト側経路（1〜4）と非スクリプト経路（6・IPC）は独立に着地可能な 2 シーム。
- 台帳追随（SET 26・サウンド語彙）は独立タスクに切れる。

## Out of Boundary

- どの枝にどの値を載せるか（木側 spec の所有）。
- `\![get/set]` の property 以外のサブコマンド。

## Upstream / Downstream

- **Upstream**: sylphya（機構は完備・`NotFound`/`NotSettable` 縮退は既定で正しい）・`\!` 汎用キャリア（転写済み）・zsp（consumer_ledger `"set"` 粒度の先着裁定）。
- **Downstream**: `areka-P0-currentghost-property-tree`・`areka-P0-property-catalog-lists`（本 spec が経路のゲート）・bvc（`.vertical` の「照会できる」の最終成立）・里々/YAYA 互換（経路 6）。

## Existing Spec Touchpoints

- **Extends**: なし（新設経路）。
- **Adjacent**: `areka-P0-sakura-time-directives`（M2 ゲート・compile 側 allowlist＝層違いで非衝突・zsp 追記(82)⑤ 参照）。

## Constraints

- ウェーブ配置: **M2 解禁ゲート**（emo2 は property 照会を使わない見込み＝e2e 非ブロック・要件段階で emo2 辞書の grep 確認）。プロパティ 3 spec の先頭（他 2 本のゲート）。
- 経路 6 の輸送路はライブ ukadoc/SSP 実測なしに設計しない（snapshot は無記載＝bvc SC 系と同じ「snapshot だけで裏取りしない」規律）。
- 決定論テスト必達・値なし（NotFound）経路も檻に入れる。

---

> **📌 2026-09-02 棚卸⑫（W12 裁定枠 B 候補＝依存ツリー最長の先頭・XL＝分割推奨）**——アンカー再測定: `decode.rs:321`・`command.rs:163`・`runtime.rs:223-224`・`msg.rs:119-156`・`choice_drain.rs:71`・`schedule/choice.rs:60`・`events.rs:391`・`actor.rs:274`・`lexer.rs:273-285`・`sysvar.rs:65-83`・`shiori-host32-ipc/lib.rs:42-55`・`interface.rs:153/:159`・`shiori_host.rs:247/:267`・`main.rs:194`・`shiori_inproc.rs:276/:289`・`dotted.rs:72`（21 項）・`:106-109` ＝**全命中**。**ずれ（zsp 合流由来）**: `consumer_ledger.rs:96`→**:224/:227/:230/:233**（`"move"`／`"bind"`／`("set",Some("zorder"))`／`("reset",Some("zorder"))`）・`emo2_boot/mod.rs:420`→**:447-453**（5 sink）・`:426`→**:454**。
> **前提の更新 2 点**: ⑴ **consumer_ledger の粒度は zsp が「選別子つき」で先着**（`("set", Some("zorde"))` 形）＝brief の「コマンド名粒度」前提は要更新・`\![get,property]`／`\![set,property]` も同形で登記できる。⑵ **`crates/areka-ghost/src/prop_sink.rs` に `PropSetCueSink`（内部キャリア `areka.prop.set` を名前自己選別→`SylphyaPublisher::persist_put`）が本番配線ごと実在**（`areka-ghost/src/runtime.rs:601` で登録済み）＝`\![set,property]` sink の同型雛形。
> **⚠ 三重所有**: `seriko.zorder` の SET 台帳を本 spec（21→26）・`zorder-property`（「dotted.rs に入れない・本 brief が語彙正本」）・`currentghost-property-tree`（`seriko.*` 一括）が食い違って主張。棚卸⑫の推奨＝**台帳行 1 本は本 spec⑶ が持ち、値の導出は zorder-property 単独・tree は zorder を除外**（着手前に 1 度で裁定）。
> **分割の継ぎ目（開発者裁定）**: ⑴ スクリプト経路 1〜4（sink 2 本＋lexer `%property[…]`＋`\![embed]` 対）／⑵ 非スクリプト経路 6＋host32 IPC 運搬（**輸送路のライブ実測が前提条件**・snapshot 無記載）／⑶ sylphya SET 台帳 21→26＋件数檻（S・単独着地可）。W12 に載せるなら ⑴＋⑶。**共有ファイル**: ⓪ lexer 修正と `lexer.rs`／`decode.rs`（⓪ 先着で解消）・e2e とは crate 同居（`areka-ghost` src/ 対 tests/）＝sink 追加が spine の sink 数を変え得る点は本 spec の保存義務。cursor-tag／toolkit とは 0。**要件定義は Fable 推奨**。


---

> **📌 2026-09-11 棚卸⑬（M1 完成後の再編・分割 ⑴⑵⑶ を実施）**——本 spec は **⑴ スクリプト経路 1〜4**（get sink・set sink・`%property[...]` の lexer 拡張・`\![embed]` の対・`KanadeMsg` の参照付きイベント発生型・consumer_ledger の選別子つき登記）**だけ**を持つ。⑵ 非スクリプト経路 6＋host32 IPC 運搬＋`.ext.*` 語彙は **`areka-P0-property-ipc-transport`**、⑶ sylphya SET 台帳 21→26＋サウンド語彙＋件数檻は **`areka-P0-sylphya-set-ledger`** へ切り出した（各 brief 参照）。本 spec は `dotted.rs` と host32 系 crate に**触れない**。三重所有の仮裁定＝台帳行は `sylphya-set-ledger`・値の導出は `zorder-property` 単独・tree は `seriko.*` から `zorder` を除外（roadmap「棚卸⑬の仮裁定」節）。編成＝**W14**（`decode.rs`／`lexer.rs` は W13 の `text-decoration-canon`／`sakura-tag-word-boundary` の後・`kanade/schedule` は W13 の `kanade-boot-talkdone-drop` の後）。パスの是正 3 件（棚卸⑬実測）: `choice_drain.rs` は `crates/areka/src/input_events/choice_drain.rs`・`sysvar.rs` は `crates/areka-sakura/src/sysvar.rs`・`shiori_inproc.rs` は `crates/areka-ghost/src/shiori_inproc.rs`。`scan_sysvar` は `lexer.rs` の `fn scan_sysvar`（行は :322 付近へ移動・主張は不変）。規模 XL → **M〜L**。要件定義は Fable。

> **📌 2026-09-17 `areka-P0-sylphya-set-ledger` が **完了**（PR#151・`.kiro/specs/completed/areka-P0-sylphya-set-ledger/`）**——⑶ は着地済み。本文の「`SET_EFFECTIVE` 21→26・サウンド語彙族の登記・件数檻の更新」（Current State・In・台帳追随の項）は**本 spec の範囲に残らない**。実数は **21→25**（26 ではない＝開発者裁定 2026-09-13・`seriko.zorder` は語彙表へ載せない）で、サウンド 18 葉は書き込みの仕分けが読まない記録用の表 `SOUND_PROP_NAMES` として登記された。アンカーのずれ: `dotted.rs:72`（21 項）→ `pub const SET_EFFECTIVE`（25 項）の定義行・`:106-109` → `pub const EXT_EVENT_GET`／`EXT_EVENT_SET` の定義行。⚠ 同 spec の説明文に記録した「正典に根拠のない areka 固有の作り 2 点」（葉の名前だけで正準語彙とみなす判定／設定可能語彙が末尾形で持たれ実キー `currentghost.sound(要素名).pause` 等と突き合わされない）の是正は、フルキーが初めて仕分けへ流れ込む**本 spec が引受先**（`dotted.rs` の `SOUND_PROP_NAMES` の説明文を参照）。所有の相互参照は `doc/COMPAT_ARCHITECTURE.md` §8 の【所有の相互参照】行。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 中〜高（後ろの 3〜4 本の門）。**着手の前に Current State を書き直す**（下の 3 点）。
- **崩れた前提**: ⑴ 「kanade にイベントを起こす型が無い」は誤りになった＝α が `KanadeMsg::RaiseEvent{id, references, method, reply}`（`areka-kanade/src/msg.rs`）を入れた。ただし `ALLOWED_EVENT_IDS`（`schedule/events.rs`・今 44 語）に無い名前は落とす。`\![get,property,<ゴーストが決めたイベント名>]` は任意の名前を起こす必要がある＝**許可の表をどう通すかを要件の最初に決める**（`mcp-kanade-tools` の `raise_event` も同じ迂回が要る＝一度で設計する）。⑵ 消費者の台帳は今、選び手つきで登記する（例 `("set", Some("zorder"))`）。⑶ ⑶（SET の台帳）は完了 `sylphya-set-ledger` で済み（21 → 25。本文の「26」は古い）。
- 変わっていない点: `%property[` はまだ字句にならない（`sakura/lexer.rs` の `scan_sysvar` が `[` で止まる）・`get`／`embed` を消費する者は居ない・`PropSetCueSink` は在る（`areka-ghost/src/prop_sink.rs`）。
- **規模**: `\![embed]`（コンパイル済みの台本の途中へ SHIORI の返事を差し込む）を含めると 20 タスクを超える見込み＝**要件の段で `\![embed]` を別へ切る**（切り出すなら `sakura-time-directives` の C 群と一緒）。含めなければ M（12〜16）。
- **触るファイル**: `areka-kanade/src/{msg.rs 894, schedule/{events,change,mod 859}.rs}`・`areka-parsers/src/sakura/{lexer,decode}.rs`・`areka-sakura/src/sysvar.rs`・`areka-ghost/src/prop_sink.rs`・`crates/areka/src/emo2_boot/{consumer_ledger.rs 859, mod.rs 878}`。kanade の `schedule/` を触る spec（`translate-pipeline`・`balloon-lifecycle-events`・`network-update-canon-order`）とは同時に走らせない。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: `\![embed]` を含めると 20 を超える（22〜26）＝**切る**。案: ⒜ 本 spec＝`\![get,property]`・`\![set,property]`・`%property[…]`・`SenderType: property` の送出＝M（13〜17）／⒝ `\![embed]`＝新しい spec（仮名 `sakura-embed-directive`）＝M（10〜14）。⒝ は台本を途中で分けて SHIORI の返事を差し込み、続きを組み直す工事で、`sakura-time-directives` の C 群（実行時に長さが決まる待ち）と同じ形だが、C 群の他の語と違って消費する者（SHIORI）が既に居る＝今でも作れる。順は ⒜→⒝（⒝ は ⒜ が足す「任意の名前のイベント」と `SenderType` の口を使う・台本のコンパイルの列にも入る）。**brief は今分けてよい**: `\![embed]` は前回「切るなら `sakura-time-directives` の C 群と一緒」とされたが、C 群は「消費する者が現れるまで置く」ので、そこへ入れると `\![embed]` だけが理由なく止まる。
- 前提の状態: 着地済み（`translate-pipeline` の `State` の 2 つの欄と `Action::Translate`／`Input::TranslateDone` を受け取る）。
- 崩れた前提／古くなった位置:
  - 「`KanadeMsg` にイベントを起こす型が無い」は誤り（前回どおり）。`KanadeMsg::RaiseEvent`（返事つき）と `KanadeMsg::AwaitTalkGap`（印のイベントを送ってその台詞の終わりを待つ）が在る。どちらも `events::allowed_static` で許可の表に無い名前を捨てる。`\![get,property,<ゴーストが決めた名前>,…]` を通すには、選択肢の任意名と同じく出所つきの名前（`msg.rs` の `EventId` に `Choice(String)` と並ぶ腕を 1 つ）を足すのが素直。`mcp-kanade-tools` の `raise_event`（`crates/areka/src/mcp/raise_event.rs` は今ダミー）も同じ迂回が要る＝一度で設計する（前回どおり）。
  - **`SenderType` はどこからも送っていない**（`crates/shiori-host32-host/src/shiori3.rs` の `build_request` の説明に「M1 最小のため送出しない」）。`SenderType: property` を載せるには、kanade の `ShioriCall`（`msg.rs`）→ 殻（`actor.rs`）→ `shiori-host32-host` の `ShioriRequest`（`client.rs` の 2 か所）と `build_request`、x64 の同じ組み立て（`crates/areka-ghost/src/shiori_inproc.rs` の `build_request` の呼び出し）まで欄を通す。brief の触るファイルにこのクレートが無い。ukadoc は「`OnTranslate` は元のイベントの属性を引き継ぐ」とも書く＝`schedule/translate.rs` の `SourceEvent` にも載せる。
  - 消費者の台帳 `crates/areka/src/emo2_boot/consumer_ledger.rs` は 859 行（中にテストを持つ）。登記は `("get", Some("property"))`・`("set", Some("property"))` の選び手つき。
  - `scan_sysvar` は `crates/areka-parsers/src/sakura/lexer.rs` の `fn scan_sysvar`（415 行のファイル）。主張は変わらない（`[` で止まる）。
  - `%property[…]` の値の源: 今の `%` の解決は台詞ごとの写し（sylphya の `SylphyaReader::talk_snapshot`）で、点つきの名前は引けない。`resolve_dotted_str` を表示の時に引く口が要る。
  - sylphya の SET は `RuntimeCommandSink` が未登録（`crates/areka-sylphya/src/actor.rs`）＝`\![set,property]` で運行の値（`seriko.*`・`mousecursor.*` など）へ書いても届く先が無い。本 spec は経路だけで、届け先は値の側の spec（`zorder-property` ほか）が登録する、と要件で書き分ける。
- 触るファイル（並走の照合用・⒜）:
  - `crates/areka-kanade/src/msg.rs`（`EventId`・`ShioriCall`）・`schedule/events.rs`・`schedule/change.rs`（`on_raise_event`）・`schedule/translate.rs`・`actor.rs`
  - `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/shiori_inproc.rs`
  - `crates/areka-parsers/src/sakura/lexer.rs`（`%property[` の字句）・`crates/areka-sakura/src/sysvar.rs`
  - `crates/areka-ghost/src/prop_sink.rs`（set の受け口の雛形 `PropSetCueSink`）＋新規の get の受け口・`crates/areka-ghost/src/runtime.rs`（受け口の登録）
  - `crates/areka/src/emo2_boot/consumer_ledger.rs`・`emo2_boot/mod.rs`（883 行・受け口の並び）
  - `doc/ukadoc-coverage/ledger/sakura-script.toml`（4 行）
  - ⒝ を含めるなら追加で `crates/areka-sakura/src/compile.rs`・`crates/dola/src/cue/`・`crates/areka-kanade/src/schedule/`（台詞の途中の往復）
- 議題（答えで作業が変わるものだけ）: `\![embed]` を今別 spec に分けるか（上の案・分けるなら新しい brief を起こす）。
- 見つけた穴: 網羅台帳 `shiori.toml` の `property.get:1`・`property.set:1` の `owner` が本 spec のまま。棚卸⑬で `.ext.*` の運搬は `property-ipc-transport` へ移した＝台帳の持ち主が古い（実害なし・次に台帳を触る spec が直す）。並走の照合: `shiori-host32-host` を触るので `makoto-dll-host`・`property-ipc-transport` とも同時に走らせない（brief の「host32 系に触れない」は `SenderType` のために崩れる）。sylphya を読む `mcp-get-property`（C3-⑦）とは `areka-ghost/src/runtime.rs` を分け合う。

## 2026-10-04 棚卸㉑で切った後の範囲

- 残した範囲: 経路 1〜3（`\![get,property,…]`・`\![set,property,…]`・`%property[…]`）と、許可の表に無い任意の名前のイベントを出所つきで送る口・`SenderType`（`property` に加えて後続が使う `embed` の値も運べる形）の運搬。
- 規模: M（13〜17 タスク）。
- 移した先: `\![embed]`（経路 4）は新しい spec `areka-P0-sakura-embed-directive` へ（本 spec が前提）。網羅台帳の `\![embed,…]` の行の持ち主は、向こうが着地するときに直す。
