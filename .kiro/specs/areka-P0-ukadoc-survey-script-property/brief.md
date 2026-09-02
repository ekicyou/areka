# Brief: areka-P0-ukadoc-survey-script-property

> 起票: 2026-09-02（`/kiro-discovery` Path D・ukadoc 網羅調査 5 本の 4 本目）。
> **種別**: 調査 spec（台帳＋ブリーフィング節・実行時コード非接触）。`ukadoc-survey-toolkit` の道具で `doc/ukadoc-coverage/ledger/script-property.toml` を書く。
> **所有範囲＝「スクリプト語彙とプロパティ木」**: list_sakura_script 342・list_propertysystem 188＝**530 項目**。
> **本ドメインは既に 13 本の M2 ゲート brief（2026-08-27 起票）が部分的に台帳化している。本 spec の主務は「新規の数え直し」ではなく「既存 brief の所有範囲を台帳へ転記し、どの brief にも属さない残余を炙り出す」こと。**

## Problem

さくらスクリプト 342 項目とプロパティ 188 項目は、areka で最も spec が集中している面である（完了 spec: sakura-dialogue-tags・choice-render・choice-select-events・kero-balloon・balloon-vertical-canon・scope-zorder-pinning ほか／M2 ゲート brief: プロパティ系 3・文字装飾系 3・`cursor-tag-canon`・`sakura-time-directives`・`surfaces-basepos`・`balloon-canon-residue`・`zorder-property`）。だからこそ **「誰かが持っているはず」で落ちる項目**が最も起きやすい。2026-08-27 の手作業サーベイ（プロパティ木 ≈180 項目・`\f` 43 項目・`\_l` 全語彙）は brief ごとに数え方が違い、正典 id との対応が機械で追えない。

## Current State

### ukadoc 側（2026-09-02 実測）

- **list_sakura_script 342**（版番号 55）: `\![*]` から始まる `\![...]` 汎用コマンド群（`\![set,...]`・`\![get,property,...]`・`\![execute,...]`・`\![embed,...]`・`\![raise,...]`・`\![open,...]`・`\![update,...]`・`\![sound,...]`・`\![lock/unlock,...]`・`\![enter/leave,...]`・`\![bind,...]`・`\![anim,...]`・`\![move,...]`・`\![notify,...]`・`\![reload,...]`・`\![vanish...]`・`\![quicksession,...]`・`\![raiseplugin/notifyplugin]` 等）＋スコープ／サーフェス／待機／改行／選択肢／アンカー／装飾（`\f`）／カーソル（`\_l`・`\_b`・`\_q`・`\_s`・`\_a`）／`%` 環境変数（`%property[...]`・`%username` 等）。
- **list_propertysystem 188**（版番号 98＝最も世代差が濃い）: 2026-08-27 サーベイ（`currentghost` ≈65・`system` 25・カタログ/履歴 8 根・汎用 17 葉・サウンド語彙）と件数が概ね一致（≈180 対 188）。差分 8 前後は本 spec で id 単位に確定する。
- **既存 brief の所有宣言（転記元）**: `property-query-channels`（照会経路）／`currentghost-property-tree`（`currentghost.*` ≈65）／`property-catalog-lists`（`system.*` 25・カタログ 5 根・`history`・`rateofuselist`・`currentghost.sound.*`・`.ext.*`）／`text-decoration-canon`（`\f` 17 項目＋基盤）／`anchor-tag-canon`（`\_a`＋装飾 16）／`choice-marker-styling`（`\f[cursor*]` 10）／`cursor-tag-canon`（`\_l` 全語彙）／`sakura-time-directives`（時刻系ディレクティブ）／`surfaces-basepos`（`\![move]` 系の basepos）／`balloon-canon-residue`（balloon 残語彙 10）／`zorder-property`（`currentghost.seriko.zorder`）。
- **完了 spec の裁量登記**: `doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」（zsp 16 行・bvc 節ほか）は台帳の `note`／`status=degraded` の転記元。

### areka 側（2026-09-02 実測・file:line は着手時に再検証すること）

- **字句**（`areka-parsers/src/sakura/lexer.rs:34-59`）: `Tag`／`Bare`／`Shorthand`（`w`・`b`・`p`）／`SysVar`／`Text`／`Raw`。エスケープ・引数クォート・未閉じ吸収は実装済み。
- **意味写像**（`sakura/decode.rs`）: bare＝`\e` `\c` `\-` `\n` `\0`/`\h` `\1`/`\u`（:174-186）／正準タグ＝`\w[n]` `\_w[ms]` `\n[...]` `\p[n]` `\s[..]` `\b[..]` `\_l[x,y]` `\q[...]` `\![...]`（:191-222）／短縮 `\wN` `\bN` `\pN`／`%keyword`。**それ以外は `Instruction::Raw` へ素通し**（:220・:186）＝`\i` `\f` `\_a` `\x` `\t` `\_q` `\_s` `\4`〜`\7` `\j` `\v` `\8` `\C` `\*` は未実装（追跡 spec は文字装飾系 3 本＋`cursor-tag-canon`）。
- **`\![...]` の名前別消費**: parser は `\![move]` のみ typed（:249）・他は `GenericCommand{name, raw_args}`（:325）→ compile は `CueCommand::command_carrier`（`areka-sakura/src/compile.rs:168,178`）→ **消費者台帳 `ConsumerLedger::canonical()` は 4 登録のみ**（`areka/src/emo2_boot/consumer_ledger.rs:223-238`＝`move`／`bind`／`set,zorder`／`reset,zorder`）。`open`／`raise`／`vanish`／`execute`／`update` は消費者未登録（テスト文字列のみ）。時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choicetimeout`／`set,balloontimeout`／`embed`／`sound,wait`／`wait,syncobject`）は M1 非実導出（`doc/COMPAT_ARCHITECTURE.md:129`）。M-boot 外タグは `debug!` で無視（`compile.rs:203`）。
- **プロパティ**（sylphya `vocab/`）: `FLAT_VOCAB` 26（実導出 4＝`username`／`selfname`／`selfname2`／`keroname`）・`DOTTED_ROOTS` 10・`GENERIC_PROP_NAMES` 17・`SET_EFFECTIVE` 21・`EXT_EVENT_GET/SET` 2（予約のみ）。状態型 `M1Status::{Derived, Degraded}`／`DegradePolicy::{PassThroughRaw, ConsumerDefault, NotFound}`（`vocab/mod.rs:11-55`）。**実導出は `baseware.*` のみ・他の根枝は NotFound 縮退**（`dotted.rs:4-6`）。件数固定テスト有り＝台帳 status の機械転記元として使える。

## Desired Outcome

- 530 項目すべてに status・根拠・**担当 spec（既存 brief 名または「所有者なし」）**・優先度が付き、`unclassified` が 0。
- **「所有者なし」項目の一覧**がブリーフィング節（`doc/ukadoc-coverage/briefing-script-property.md`）に出る。これが本 spec の主成果＝13 本の brief の網の目から落ちている項目。
- `\![...]` 汎用コマンドは **消費側 name ごとの対応状況**（記憶 areka-bang-commands-generic-carrier＝typed 個別新設禁止・消費側 name 選別）で台帳化され、「キャリアは通るが誰も消費しない」名前が列挙される。
- プロパティ 188 は 3 本の brief の所有宣言と id 単位で突合され、二重所有・無所有が 0 になる。

## Approach

- 先に既存 brief 13 本と COMPAT §8 を機械的に転記（brief に書かれた項目名→catalog id の対応表を作る・対応が付かない名前は brief 側の表記揺れとして記録）。
- 次に toolkit の evidence スキャン（sakura parser の受理タグ・`\!` 消費側の name 選別・sylphya 語彙表）で status 候補を付ける。
- 最後に残余（無所有）を人手で分類し優先度を付ける。
- `%` 環境変数（`%username`・`%property[...]` 等）は sakura 342 の一部として本 spec が持つ（照会経路の実装は `property-query-channels`）。

## Scope

- **In**: 上記 530 項目の台帳・既存 brief 突合表・無所有一覧・`\!` 消費側 name 表。
- **Out**: 実装・既存 brief の書き換え（差分は「brief 側の是正候補」として briefing に列挙するだけ）・shiori／assets 台帳。

## Boundary Candidates

- 「タグ（構文）」と「`\!` コマンド名（意味）」は別節。
- プロパティは「照会経路／木」の既存分割線に従う（brief 3 本の境界を再定義しない）。

## Out of Boundary

- 既存 M2 ゲート brief の優先順位の変更（`ukadoc-coverage-roadmap` の裁定事項）。

## Upstream / Downstream

- **Upstream**: `ukadoc-survey-toolkit`。既存 M2 ゲート brief 13 本・COMPAT §8・sylphya 語彙表。
- **Downstream**: `ukadoc-coverage-roadmap`（無所有一覧と優先度を受け取る）。既存 brief 13 本（是正候補の受け手）。

## Existing Spec Touchpoints

- **Extends**: なし（brief は書き換えない）。
- **Adjacent**: 上記 13 本すべて（転記元・非接触）／`emo2-conformance-e2e`（W12・共有ファイル 0）。

## Constraints

- 台帳は `doc/ukadoc-coverage/ledger/script-property.toml` 1 ファイルのみ。
- 既存 brief の項目数と catalog の件数が食い違うときは catalog（id）を正とし、brief 側の差を記録する（brief を黙って直さない）。
- 「所有者なし」は憶測で既存 brief に押し込まない（記憶 deferral-requires-verified-owner）。
