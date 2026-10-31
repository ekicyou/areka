# Brief: dressup-change-events

> 2026-10-31 起票（`script-small-tags` の要件の討議の議題 3。開発者が推奨案「この spec では送らず、送る spec を起票する」を一括承認した）。網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `OnDressupChanged`（2257 行目）・`OnNotifyDressupInfo`（4418 行目）はどちらも absent・持ち主なし。正典は ukadoc MCP で 2026-10-31 に引き直した。版の置き場は次の棚卸で決める。

## Problem

- **ゴーストの作者**: 着せ替えが変わったことを SHIORI で知れない。辞書に `OnDressupChanged`・`OnNotifyDressupInfo` の返事を書いても、areka では一度も呼ばれない（「何も喋らない」という形でだけ現れる・記録も出ない）。
- **違いが見えない**: `\![bind,…]` は実行の後にこの 2 つを知らせ、`\![bind-noevent,…]` は知らせない、が正典の違い。areka はどちらからも送らないので、2 つの指令の違いが外から見えない（`script-small-tags` が 2 つの名前を受け手まで区別して運ぶところまで作る）。

## Current State

- **正典（ukadoc・要旨）**:
  - `OnDressupChanged`（https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnDressupChanged ）: 着せ替えが切り替えられたときに起きる。何回も知らせるときは最後の 1 回が [GET]、ほかは [NOTIFY]。変わった分が多い（今のところ 100 要素）ときは送らない（その受け取りは `OnNotifyDressupInfo`）。Reference0＝キャラクターの ID・Reference1＝パーツの名前・Reference2＝着たら 1、脱いだら 0・Reference3＝カテゴリの名前・Reference4＝台本からなら `script`、メニューからなら `user`（2.5.90〜）。
  - `OnNotifyDressupInfo`（https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnNotifyDressupInfo ）: 着せ替えの情報を知らせる。起動のときだけ [NOTIFY]、メニューからの切り替えなど利用者の操作なら [GET]。Reference* は 1 つの着せ替えの定義ごとに、バイト値 1 で区切った［キャラクターの ID・カテゴリの名前・パーツの名前・オプション・着ている 1／脱いでいる 0・サムネイルの道］。`OnDressupChanged` の後に起きる。
  - `\![bind,カテゴリ名,パーツ名,数値]`: 「スクリプト実行後、SHIORIイベントOnDressupChangedが通知され、その後にSHIORIイベントOnNotifyDressupInfoが通知される。」`\![bind-noevent,…]`（SSP 2.8.23）: 「\![bind]と同様に着せ替えパーツを操作するが、イベントを発生させない。」
- **areka**:
  - 着せ替えの受け手は seriko（`crates/areka-seriko/src/actor.rs`・`BIND_COMMAND`）。状態は `crates/areka-seriko/src/state.rs` の `apply_bind`・`apply_bind_exclusive`。
  - イベントを組み立てる所（`kanade` の `schedule/events.rs`）にこの 2 つの構築関数が無く、送ってよいイベントの表（`ALLOWED_EVENT_IDS`）にも名前が無い。
  - ゴーストの読み直しの持ち越し `crates/areka/src/emo2_boot/shell_carry.rs` は、今の着せ替えの状態を `\![bind,…]` と同じ合図で流し直す（369 行目あたりで `BIND_COMMAND` を使う）。送るようにすると、読み直しのたびにイベントが出てしまう。
  - 着せ替えのメニューはまだ無い（`popup-menu-shell-look` が `sakura.menu,auto` を持つ）。

## Desired Outcome

- `\![bind,…]` で着せ替えが変わった後に、`OnDressupChanged`（変わったパーツごと・最後が [GET]）と `OnNotifyDressupInfo` が正典の順と Reference で送られる。
- `\![bind-noevent,…]` からは 1 つも送らない。
- 起動のときに `OnNotifyDressupInfo` が [NOTIFY] で 1 度送られる。
- ゴーストやシェルの読み直しの持ち越しで着せ替えを流し直すときは送らない（「イベントを起こさない」側に寄せる）。
- 着せ替えのメニューができたら、メニューからの切り替えでも送る（Reference4 は `user`）。メニューがまだ無いなら、その受け口だけを用意して申し送る。
- 網羅台帳の 2 行が実物と合う。

## Approach

- seriko の着せ替えの受け手が、着せ替えの結果（変わったパーツの列と、どこから来たか＝台本／`bind-noevent`／持ち越し／メニュー）を返し、送る側（kanade のイベントの組み立て）がそれを見て送るかを決める。分け方は要件の段で決める。
- 100 要素で送らない決まり・[GET] と [NOTIFY] の使い分け・起動のときの 1 度を要件で決める。正典で読めない点（台本の 1 本の中の複数の `\![bind]` の束ね方・トーク中に送る時機）は、議題の前に SSP の MCP で測る。

## Scope

- **In**: 2 つのイベントの組み立てと送出・`bind` と `bind-noevent` と持ち越しの分け・起動のときの `OnNotifyDressupInfo`・決定論のテスト・網羅台帳の 2 行。
- **Out**: 着せ替えのメニューそのもの（`popup-menu-shell-look`）・`\![bind,…]` の形の読み（`script-small-tags`）。

## Boundary Candidates

- 着せ替えの受け手（seriko）が返す「着せ替えの結果」
- イベントの組み立てと送出（kanade）
- ゴーストの読み直しの持ち越し（`shell_carry.rs`）の「送らない」目印

## Out of Boundary

- 着せ替えの適用の決まり（`mustselect` などのカテゴリの決まり・名前の解決）
- メニューの見た目と組み立て

## Upstream / Downstream

- **Upstream**: `script-small-tags`（`bind-noevent` の受け手・2 つの名前の区別・カテゴリ単位の形とトグル形）。
- **Downstream**: `popup-menu-shell-look`（メニューからの着せ替えで Reference4 `user`）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `script-small-tags`（seriko の `actor.rs` の着せ替えの受け手）・`popup-menu-shell-look`（着せ替えのメニュー）・kanade のイベントを足す spec（`schedule/events.rs`）。

## Constraints

- 送る道に失敗があれば記録を残す（黙って落とさない）。
- seriko の `actor.rs` は 1,000 行の上限が近い（908 行・`script-small-tags` の着地でさらに増える）＝足す物は兄弟のファイルへ。
- 正典で読めない点は、議題の前に SSP の MCP で測る。
