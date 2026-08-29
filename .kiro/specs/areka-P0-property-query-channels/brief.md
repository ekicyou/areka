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
