# Brief: balloon-append-tags

> 2026-10-31 `balloon-scroll-arrows` の完了の棚卸で起票（`/kiro-discovery`）。同 spec のタスク 8.2 の実機の走りで台本に `\C` を書いたところ、areka の `check_script` が `unknown_tag` と答えた。開発者の方針「実機で areka の未対応のためにうまくいかなかった件は、範囲外でもすべて起票」に従う。網羅台帳の同じ群「バルーンの追記と選択肢のタイムアウト抑止」の残り 1 件 `\*` も、持ち主が居ないので同じ spec に入れる。
> 起票の理由（`kiro-complete` 冒頭ステップ手順 3 の条件）: 規模が spec 1 本に相当する（バルーンを消さずに前のトークへ書き足す新しい道が要る）。

## Problem

- **ゴーストの作者**: 前のトークへ書き足すつもりで `\C` を書いても、areka ではバルーンがいったん空になってから文が出る。里々・YAYA の作例にも出てくるタグで、SSP で書き足しを前提に組んだ会話が areka では途切れて見える。
- **ゴーストの作者**: 選択肢の待ち時間を止める `\*` が効かず、選択肢が待ち時間で消える。作者の意図と逆に見える。

## Current State

- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml`:
  - `\C`（`ukadoc:list_sakura_script:_5cC:1`）: `status="absent"`・`owner=""`。壊れ方は「黙って壊れる」。読み手 `decode_bare` の既定の腕から `decode_passthrough_bare` へ落ちて `Raw` になり、台本の組み立て（`areka-sakura` の `compile.rs`）の残りの腕が cue を作らずに捨てる。組み立てが台本の先頭へ前置きする `ClearAll` でバルーンが空になってから表示される。記録は `debug!` で出る。
  - `\*`（`ukadoc:list_sakura_script:_5c_2a:1`）: `status="absent"`・`owner=""`。同じく素通しになる。
- 報告 `doc/ukadoc-coverage/briefing-sakura-script.md` の群「バルーンの追記と選択肢のタイムアウト抑止（2 件）」が、成立させる最小の基盤を「バルーンを消さずに書き足す道と、選択肢の待ち時間を止める道」と書いている。
- areka の `check_script`（MCP）は `\C` を `unknown_tag` と答える（2026-10-31・`balloon-scroll-arrows` タスク 8.2 の実機の走りの記録＝同 spec の `research.md`）。

## Desired Outcome

- `\C` を含む台本は、前のトークのバルーンの中身を消さず、その後ろへ書き足して表示する（ukadoc の定義どおり。細部は要件の段で ukadoc から引き直し、ukadoc で読めない点は SSP の MCP で測る）。
- `\*` を含む台本の選択肢は、待ち時間で消えない。
- 台帳の 2 行が `implemented`・持ち主＝本 spec になる。

## Approach

- 読み手（`areka-parsers` の `sakura/decode.rs`）に 2 つのタグの腕を足し、台本の組み立て（`areka-sakura` の `compile.rs`）で先頭の `ClearAll` を `\C` のときに置かない形にする。選択肢の待ち時間は、今の時間切れの判断の側で止める印を読む。直し方の細部は要件・設計の段で決める。

## Scope
- **In**: `\C` の書き足し・`\*` の選択肢の待ち時間の抑止・台帳の 2 行・`doc/COMPAT_ARCHITECTURE.md` §8 の登記。
- **Out**: バルーンの部分消去（`\c[char,…]`・`\c[line,…]`＝実装しない裁定・記憶 `areka-partial-clear-tags-not-implemented-bitmap-vs-glyph`）。手動の送り（`balloon-scroll-arrows` が持つ）。

## Boundary Candidates
- 読み手と組み立て（台本の構造）
- バルーンの中身を保つ側（文字の層の消去の扱い）
- 選択肢の時間切れの判断

## Out of Boundary
- 選択肢のマーカーの描画（`\![*]`＝`script-small-tags`）。
- 時刻の指令（`\_q` など＝`sakura-time-directives`）。

## Upstream / Downstream
- **Upstream**: `areka-parsers`（読み手）・`areka-sakura`（組み立て）・emo-text（文字の層の消去）。
- **Downstream**: 書き足しを前提に組んだ里々・YAYA のゴースト。

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `script-small-tags`（同じ読み手の `decode.rs` を触る・`\![*]`）・`sakura-time-directives`・`balloon-canon-residue`（バルーンの正典の残り）。

## Constraints
- 意味論は ukadoc から輸入する。ukadoc で読めない点は議題の前に SSP の MCP で測る（記憶 `no-ssp-measurement-import-semantics-from-ukadoc`）。
- 優先度の区分: C（ukadoc の拾い残し）。版の置き場は次の棚卸で決める。
