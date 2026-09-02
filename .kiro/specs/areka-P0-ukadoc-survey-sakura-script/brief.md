# Brief: areka-P0-ukadoc-survey-sakura-script

> 起票: 2026-09-02（`/kiro-discovery` Path D・ukadoc 網羅調査 6 本の 4 本目）。同日の開発者追記「さくらスクリプトも大事。ukadoc をとにかく網羅的に一度調査して仕訳すべき」を受け、当初の `ukadoc-survey-script-property`（さくら 342＋プロパティ 188 の合併）を **さくらスクリプト単独の全数調査**へ格上げし、プロパティは `ukadoc-survey-property` へ分離した。
> **種別**: 調査 spec（台帳＋ブリーフィング節・実行時コード非接触）。`ukadoc-survey-toolkit` が凍結した台帳形式で `doc/ukadoc-coverage/ledger/sakura-script.toml` を書く。
> **所有範囲＝「さくらスクリプト全語彙」**: list_sakura_script **342 項目**（版番号付き 55）。

## Problem

さくらスクリプトはゴーストが「話す・動く・尋ねる・装う」ための唯一の言語であり、既存ゴースト資産の辞書はこの 342 語彙の上に書かれている。areka の parser が意味写像するのは十数タグで、残りは `Instruction::Raw` へ素通し＝**書いてあるのに何も起きない**。しかも語彙には世代がある——同じ機能に旧書式と新書式が併存し（例: `\w8` と `\_w[ms]`・`\s0` と `\s[ID]`・`\![sound,play,...]` と旧 `\_v[...]`／`\8[...]`・`\![raise]` と `\![embed]`）、ukadoc は両方を載せている。**最新仕様を優先し、新書式を正典・旧書式をエイリアス**として仕訳しない限り、実装順も検証順も決められない。

## Current State

### ukadoc 側（2026-09-02 実測）

- 342 項目の内訳（title の先頭で機械分類・要件フェーズで確定）: `\![...]` 汎用コマンド群（`set`／`get`／`execute`／`embed`／`raise`／`open`／`update`／`sound`／`lock`／`unlock`／`enter`／`leave`／`bind`／`anim`／`move`／`notify`／`reload`／`vanish`／`quicksession`／`raiseplugin`／`notifyplugin`／`*`〔マーカー〕ほか）・スコープ／サーフェス（`\0`〜`\p[n]`・`\s`）・待機／改行／消去（`\w`・`\_w`・`\n`・`\c`・`\x`・`\t`・`\e`）・選択肢／アンカー（`\q`・`\_a`・`\__q`）・装飾（`\f[...]` 43 項目）・カーソル／位置（`\_l`・`\_b`・`\_q`・`\_s`）・`%` 環境変数（`%username`・`%property[...]`・`%month` 等）。
- 版番号付き 55 項目＝新旧の判別材料。版番号の無い 287 項目は「初期からある」か「版番号未記載」かの区別が要る（旧書式側に版番号が無いことが多い）。

### areka 側（2026-09-02 実測・file:line は着手時に再検証すること）

- **字句**（`areka-parsers/src/sakura/lexer.rs:34-59`）: `Tag`／`Bare`／`Shorthand`（`w`・`b`・`p`）／`SysVar`／`Text`／`Raw`。エスケープ・引数クォート・未閉じ吸収は実装済み。
- **意味写像**（`sakura/decode.rs`）: bare＝`\e` `\c` `\-` `\n` `\0`/`\h` `\1`/`\u`（:174-186）／正準タグ＝`\w[n]` `\_w[ms]` `\n[...]` `\p[n]` `\s[..]` `\b[..]` `\_l[x,y]` `\q[...]` `\![...]`（:191-222）／短縮 `\wN` `\bN` `\pN`／`%keyword`。**それ以外は `Instruction::Raw` へ素通し**（:220・:186）＝`\i` `\f` `\_a` `\x` `\t` `\_q` `\_s` `\4`〜`\7` `\j` `\v` `\8` `\C` `\*` は未実装（追跡 spec は `text-decoration-canon`・`anchor-tag-canon`・`choice-marker-styling`・`cursor-tag-canon`）。
- **`\![...]` の名前別消費**: parser は `\![move]` のみ typed（:249）・他は `GenericCommand{name, raw_args}`（:325）→ compile は `CueCommand::command_carrier`（`areka-sakura/src/compile.rs:168,178`）→ **消費者台帳 `ConsumerLedger::canonical()` は 4 登録のみ**（`areka/src/emo2_boot/consumer_ledger.rs:223-238`＝`move`／`bind`／`set,zorder`／`reset,zorder`）。`open`／`raise`／`vanish`／`execute`／`update` は消費者未登録。時間指令 allowlist（`quicksection`／`set,balloonwait`／`set,choicetimeout`／`set,balloontimeout`／`embed`／`sound,wait`／`wait,syncobject`）は M1 非実導出（`doc/COMPAT_ARCHITECTURE.md:129`）。M-boot 外タグは `debug!` で無視（`compile.rs:203`）。
- **既存の所有宣言（転記元）**: 完了 spec（sakura-dialogue-tags・choice-render・choice-select-events・kero-balloon・balloon-vertical-canon・scope-zorder-pinning ほか）の COMPAT §8 登記／M2 ゲート brief（`text-decoration-canon`・`anchor-tag-canon`・`choice-marker-styling`・`cursor-tag-canon`・`sakura-time-directives`・`surfaces-basepos`・`balloon-canon-residue`）。

## Desired Outcome

- 342 項目すべてに status・根拠・担当 spec（既存 brief 名または「所有者なし」）・世代・優先度が付き、`unclassified` が 0。
- **新旧書式の仕訳が完了**: 同一機能の書式群ごとに正典（新書式）1 つと `alias` 群が `alias_of` で結ばれ、実装 spec は正典だけを実装し alias は写像で受ける、という方針が台帳から機械的に引ける。
- **`\![...]` は消費側 name ごと**に台帳化（記憶 areka-bang-commands-generic-carrier＝typed 個別新設禁止・消費側 name 選別）。「キャリアは通るが誰も消費しない」名前が列挙される。
- 関連の検索: 各タグが発火するイベント（`\q`→OnChoiceSelect・`\![raise]`→任意イベント・`\![get,property]`→指定イベント）・参照する descript キー（`\f` と `font.*`・`\_l` と `origin.*`）・プロパティ（`%property[...]`）が `links` に登記される。
- ブリーフィング節（`doc/ukadoc-coverage/briefing-sakura-script.md`）: 「書いてあるのに何も起きない」順＝既存ゴースト資産の標準辞書（里々／YAYA テンプレート）が使う頻度の高い語彙から並べた未実装一覧と、その群を成立させる最小基盤。

## Approach

- toolkit が凍結した仕訳規則（最新優先・新書式正典・旧書式 alias・版番号＝世代・版番号なしは世代不明）をそのまま適用する。書式群の同定は title の機械分類→人手確定。
- 実装済みの証拠は toolkit の規則 7（ソースの ukadoc URL）＝`implemented` と判定したタグの定義箇所（`sakura/decode.rs` の match の腕・`consumer_ledger.rs` の 4 登録）へ URL の doc コメントを置く。実行時挙動を変えない doc コメントのみ・本 spec の唯一のコード接触。
- **着手条件**: toolkit の要件確定後・実装完了を待たない。他 survey と並走。
- 既存 brief 7 本と COMPAT §8 を先に転記し、無所有の残余だけを人手で分類する（brief は書き換えず差分を記録）。
- 使用頻度の根拠は、里々／YAYA の標準テンプレート辞書（ukadoc MCP の satori_wiki／yaya_wiki カテゴリで裏取り可）が使うタグを参照値にする。実ゴースト資産の走査は対象外（ライセンス・入手の問題）。

## Scope

- **In**: 342 項目の台帳・書式群と alias 仕訳・`\!` 消費側 name 表・関連登記・無所有一覧・ブリーフィング節。
- **Out**: 実装・プロパティ 188（`ukadoc-survey-property`）・既存 brief の書き換え・SSP 実機比較。

## Boundary Candidates

- 「タグ（構文）」「`\!` コマンド名（意味）」「`%` 環境変数」は別節。
- 装飾 `\f` 43 項目は既存 3 brief の所有どおり（本 spec は台帳化と alias 仕訳のみ）。

## Out of Boundary

- 既存 M2 ゲート brief の優先順位変更（`ukadoc-coverage-roadmap` の裁定事項）。

## Upstream / Downstream

- **Upstream**: `ukadoc-survey-toolkit`（台帳形式・仕訳規則の凍結＝**要件確定後に着手可・実装完了を待たない**）。既存 brief 7 本・COMPAT §8。
- **Downstream**: `ukadoc-coverage-roadmap`（無所有一覧・alias 仕訳・優先度）。既存 brief 7 本（是正候補の受け手）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 上記 7 本＋`ukadoc-survey-property`（`%property[...]`・`\![get/set,property]` は本 spec がタグとして、property spec が木として持つ＝id 単位で二重計上しない）／`emo2-conformance-e2e`（W12・共有ファイル 0）。

## Constraints

- 台帳は `doc/ukadoc-coverage/ledger/sakura-script.toml` 1 ファイルのみ（他 survey と共有ファイル 0＝並走可）。
- 既存 brief の件数と catalog が食い違うときは catalog（id）を正とし、brief 側の差を記録する。
- 「所有者なし」は憶測で既存 brief に押し込まない（記憶 deferral-requires-verified-owner）。

> **📌 2026-09-02 開発者指針「伺からしい価値観を持つ要素の優先度を上げる」（toolkit 規則 6・9 に凍結・裁定は議題 5〜7）**: 分類軸に **テーマ付与**（`values[]`）を加える。本ドメインの代表例: **掛け合い**＝`\0`／`\1`（`\h`／`\u` は alias）・`\w`／`\_w`・`\_q`・`\q`・`\n`／`\c`／`\x`＝M1 でほぼ実装済み＝URL の doc コメントを置く先／**装い**＝`\s[]`（`\sN` は alias）・`\i[]`・`\![bind,...]`・`\![change,shell|balloon]`・`\f` 族 43（既存 3 brief の所有どおり）／**気配**＝`\4`／`\5`（重なり回避）・`\![set,balloontimeout]`／**記憶**＝`\![open,inputbox|teachbox|communicatebox]`・`\![vanishbymyself]`／**更新**＝`\![updatebymyself]`・`\![update,...]`・`\![execute,http-get]`（更新の道具）／**触れ合い**＝`\![set,alignmentondesktop]`・`\![move]`（キャラが歩く）／**交わり**＝`\![set,otherghosttalk]`・`\![call,ghost]`・`\![raise,...]` のうち他ゴースト宛。**テーマ 0**＝`%property[...]`・`\![get,property]`／`\![set,property]`・`\![embed]`・`\![raise]`（自ゴースト宛）・`\![lock,repaint]` 等の配管。「書いてあるのに何も起きない」（壊れ方⑴）が先・テーマは同段階内の並びと繰り上げに使う。

