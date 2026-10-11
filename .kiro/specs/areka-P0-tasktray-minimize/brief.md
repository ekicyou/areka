# Brief: areka-P0-tasktray-minimize

> 2026-10-10 `/kiro-discovery`（開発者「タスクトレイの土台仕様を起票したうえで、上流の ukadoc 項目を後続仕様にせよ」）で起票した。土台は `areka-P0-tasktray-icon`。正典は ukadoc MCP で確かめた。着手時に引き直す。
>
> **起票の経緯**: 開発者が名指しした ukadoc の項目のうち、`icon.minimize,ファイル名` と `alwaystrayiconvisiblebutton.caption` は、アイコン化（最小化）が無いと使いどころが無い。アイコン化そのもの（タグ・2 つのイベント・メニューの項目）も網羅台帳で持ち主なしだったので、まとめて 1 本にした。範囲が広いと見たら、要件の段で切る。

## Problem

areka にはアイコン化（最小化）が無い。キャラクターを一時的に画面から片付けたい利用者は、終了するしかない。台本の `\![set,windowstate,minimize]` は黙って消え、`OnWindowStateMinimize`・`OnWindowStateRestore` は 1 度も呼ばれない。最小化のときのアイコン（`icon.minimize`）と、メニューの「アイコン化」「常にトレイアイコンを表示」の文言（`hidebutton.caption`・`alwaystrayiconvisiblebutton.caption`）も使われない。

## Current State

- 正典（要旨）:
  - `\![set,windowstate,minimize]`: 最小化する（ukadoc「さくらスクリプト」）。
  - `OnWindowStateMinimize`: 最小化が指示されたときに起きる。Reference0（SSP だけ）＝理由。`system`（OS からの強いられた最小化）・`script`（台本から）・`sakuraapi`・`user`（メニューの操作）。
  - `OnWindowStateRestore`: 最小化が解かれたときに起きる。Reference0 は同じ。
  - descript の `icon.minimize,ファイル名`: そのゴーストの最小化したときのアイコン。無ければ `icon` と同じ。
  - リソース `hidebutton.caption`＝「アイコン化」の名称。`alwaystrayiconvisiblebutton.caption`＝「常にトレイアイコンを表示」の名称（ukadoc「SHIORI Resource」）。
  - SSP のヘルプ「右クリックメニュー」の「アイコン化」: ゴーストを最小化する。戻すには、タスクトレイのアイコンをダブルクリックする。
  - ukadoc「SHIORI/3.0」の `Status [SSP拡張]`: 最小化の間は `minimizing` が付く。
  - 周りの項目: `\![raiseother]`・`\![notifyother]` は、相手が最小化の間は `minimized` で失敗する。全画面のアプリによる最小化は `OnFullScreenAppMinimize`・`OnFullScreenAppRestore` が上書きする。簡易ユーザー切り替えでも `OnWindowStateMinimize` が起きる。
- 網羅台帳（`doc/ukadoc-coverage/ledger/`）: `\![set,windowstate,minimize]`・`OnWindowStateMinimize`・`OnWindowStateRestore`・`icon.minimize,ファイル名` は `absent`・持ち主なし。`hidebutton.caption`・`alwaystrayiconvisiblebutton.caption` は語彙の登記だけ・持ち主なし。`assets.toml` の `icon.minimize` の行は「最小化の経路も通知領域の画面も無い」と書いている。
- areka のコード:
  - 最小化の状態は無い。窓を隠す・戻す呼び出しも無い（OS の `ShowWindow` を呼ぶのは、窓を作るときの表示だけ）。`\![set,windowstate,…]` の受け口は無い（`crates/areka/src/emo2_boot/consumer_ledger.rs` が「担当なし」の例に挙げている）。
  - キャラクターの窓はタスクバーにボタンを出さない窓（`WS_EX_TOOLWINDOW`・`crates/areka/src/placement/spawn.rs`）。OS の「最小化」の操作の対象にならない。
  - メニューの枠は 7 つで、「アイコン化」の枠は無い（`crates/areka/src/menu/mod.rs` の `Frame::ORDER`）。
  - 通知領域のアイコンとダブルクリックの受け口は、土台（`areka-P0-tasktray-icon`）が持つ。`.ico` の読み込みは `areka-P0-tasktray-ghost-icon` が持つ。
- BTS の台帳（`areka-P0-ssp-bts-salvage` の `bts-ledger.md`）: 最小化の間の動きに触れる要望がある（`ValueNotify` の束＝最小化の間でも台本を動かしたい。BTS 0000286・0000332）。

## Desired Outcome

- メニューの「アイコン化」か、台本の `\![set,windowstate,minimize]` で、キャラクターとバルーンが画面から消える。通知領域のアイコンは残る。
- 通知領域のアイコンのダブルクリックで、元の位置と重なり順に戻る。
- 消える前に `OnWindowStateMinimize`、戻ったときに `OnWindowStateRestore` が SHIORI に届く。
- アイコン化の間、通知領域のアイコンは `icon.minimize` の絵になる（無ければ `icon`、それも無ければ標準アイコン）。
- メニューの「アイコン化」の文言は、ゴーストが `hidebutton.caption` で決められる。
- 網羅台帳の該当の行が実物と合う。

## Approach

「アイコン化している」という状態をアプリに 1 つ持ち、入るときにゴーストの窓を隠し、出るときに戻す。入り口は 3 つ（メニューの項目・台本のタグ・通知領域のダブルクリックで戻す）で、どれも同じ 1 つの切り替えを通す。OS の最小化（アイコンの形に畳む）は使わない＝キャラクターの窓はタスクバーに居ないので、隠すだけにする。

## 議題（要件の段で決める）

1. **アイコン化の間、ゴーストは喋るのか・時計は進むのか**。SSP は「設定で喋らせることが可能」で、既定は黙る。areka は「マスコットは利用者の状況お構いなしに喋るもの」が方針だが、アイコン化は利用者が片付けた状態である。話の再生・`OnSecondChange` などの定期のイベント・SERIKO のアニメーション・描画をどこまで止めるか。
2. **アイコン化の最中に話していた台本**をどうするか（中断する・最後まで流してから隠す）。
3. **「常にトレイアイコンを表示」の項目を作るか**。areka の土台は、アイコンを常に出す。項目を作らないなら、`alwaystrayiconvisiblebutton.caption` は「対応しない」と理由を書く。
4. **Reference0 の理由**のうち、areka で起こり得るのはどれか（`user`・`script`。`system` と `sakuraapi` は経路が無い見込み）。
5. **アイコン化のまま終了したとき、次の起動**でどうするか（戻して起動する・アイコン化のまま起動する）。
6. **ゴーストの切替・インストールの投げ込み・外からの台本（SSTP・MCP）**が、アイコン化の間に来たときの動き。
7. **`Status` の `minimizing`** をこの spec で足すか。
8. **範囲の切り方**。上の議題で膨らむなら、「状態と 3 つの入り口」と「アイコン・文言・Status」に分ける。

## Scope

- **In**: アイコン化の状態と切り替え／メニューの「アイコン化」の枠と `hidebutton.caption`／`\![set,windowstate,minimize]` の受け口／通知領域のダブルクリックで戻す／`OnWindowStateMinimize`・`OnWindowStateRestore` の送出／`icon.minimize,ファイル名`／議題 3 の結論（`alwaystrayiconvisiblebutton.caption`）／網羅台帳の該当の行／決定論テストと実機の確かめ。
- **Out**: 土台と `.ico` の読み込み（前提の 2 本）／`\![set,windowstate,stayontop]`・`\![set,windowstate,!stayontop]`／`OnFullScreenAppMinimize`・`OnFullScreenAppRestore`（全画面のアプリを見て自動で最小化する働き）／簡易ユーザー切り替えの `OnSessionDisconnect`・`OnSessionReconnect`／`\![raiseother]`・`\![notifyother]` の `minimized` での失敗（多重ゴーストが前提）／キーボードのショートカット／`ValueNotify`。

## Boundary Candidates

- アイコン化の状態と、窓を隠す・戻す働き（areka の配置の層と wintf の窓の表示）。
- 3 つの入り口（メニューの枠・`\!` の受け口・通知領域のダブルクリック）。
- kanade のイベント（`schedule/events.rs` の表と組み立て）と、アイコン化の間の話と定期のイベントの扱い。
- アイコンの切り替え（`icon.minimize`）。

## Out of Boundary

- 通知領域の部品（土台）。
- キャラクターとバルーンの位置・重なり順の決め方（戻すときに今の決め方をそのまま使う）。

## Upstream / Downstream

- **Upstream**: `areka-P0-tasktray-icon`（戻す入り口）・`areka-P0-tasktray-ghost-icon`（`.ico` の読み込みと絵の差し替え）。
- **Downstream**: roadmap の予約「多重ゴースト」（最小化の相手への `\![raiseother]` の失敗）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `menu/` を触る `popup-menu-residue`。kanade の列で `schedule/events.rs` に触る spec と直列。重なり順を触る spec（`\![set,zorder,…]` の周り）。`mcp-get-status` が返す状態（`Status` に `minimizing` を足すなら揃える）。

## Constraints

- 窓を隠して戻すとき、位置・重なり順・拡大率への追従を崩さない。戻した直後の 1 フレームに古い絵や空の窓を見せない（1 フレーム遅らせる解は取らない）。
- アイコン化の間に溜まった時間の扱いは「時刻は正確に扱う」に従う（止めた分を飛ばすのか、過ぎた分だけ進めるのかを要件で決めて書く）。
- 黙って壊れる経路を残さない。
- 実機の確かめは、画面のロックを外した机で行う。
