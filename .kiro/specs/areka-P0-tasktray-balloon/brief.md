# Brief: areka-P0-tasktray-balloon

> 2026-10-10 `/kiro-discovery`（開発者「タスクトレイの土台仕様を起票したうえで、上流の ukadoc 項目を後続仕様にせよ」）で起票した。土台は `areka-P0-tasktray-icon`。正典は ukadoc MCP で確かめた。着手時に引き直す。

## Problem

台本が `\![set,trayballoon,…]` で OS の通知を出そうとしても、areka では何も出ない。通知を押したとき・消えたときに届くはずの `OnTrayBalloonClick`・`OnTrayBalloonTimeout` も 1 度も呼ばれない。「更新が終わった」「エラーが起きた」をキャラクターが見えていないときに知らせる手段が、ゴーストに無い。

## Current State

- 正典（ukadoc「さくらスクリプト」「SHIORI Event」・要旨）:
  - `\![set,trayballoon,オプション,オプション,オプション...]`: トレイアイコンからバルーン（OS の通知）を出す。オプションは次の 4 つ。
    - `--text=テキスト`: 出す文字列。複数書くと複数行。**必須**。
    - `--title=タイトル`: 題。
    - `--icon=アイコン設定名`: `info`・`error`・`warning`・`none`。
    - `--timeout=秒数`: 出している時間。10〜30 秒だけ。Windows 7 以降は無視される。
  - `OnTrayBalloonClick`: バルーンを押して消したときに起きる。Reference0＝題・Reference1＝出した文字列。
  - `OnTrayBalloonTimeout`: 時間切れか閉じるボタンで消えたときに起きる。Reference は同じ。
  - ukadoc「SHIORI/3.0」の `ValueNotify [SSP拡張 2.5.35]` は、通せるタグに `\![set,trayballoon]` を挙げる。
- 網羅台帳（`doc/ukadoc-coverage/ledger/`）: タグと 2 つのイベントはどれも `absent`・持ち主なし。イベントの行は「発火する経路が無いので 1 度も呼ばれない」と書いている。
- BTS の台帳（`areka-P0-ssp-bts-salvage` の `bts-ledger.md`）に、SSP の作者が説明した挙動が 3 行ある。「決める話」の候補に入っている。
  - BTS 0000331: 知らせが遅れて出るのは OS の通知の仕様。遅れを無くす指定は、通知が捨てられることがあるので使わない。
  - BTS 0000407: `--text` は必須。OS の通知の設定が切られていると出ない。
  - BTS 0000561: OS は、通知が画面から消えるより早く時間切れを知らせてくる。SSP は話し始めを 1 秒遅らせて和らげた。
- areka のコード: 土台（`areka-P0-tasktray-icon`）が通知領域のアイコンと知らせの受け口を持つ。kanade が送ってよいイベントの表（`crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS`）に、2 つの名前は無い。

## Desired Outcome

- 台本の `\![set,trayballoon,--text=…]` で、OS の通知が出る。題・アイコンの種類・複数行が正典どおりに出る。
- 通知を押すと `OnTrayBalloonClick` が、時間切れか閉じるボタンで消えると `OnTrayBalloonTimeout` が、正典どおりの Reference で SHIORI に届く。
- `--text` が無いタグ・OS が通知を断った場合は、記録を残して何も出さない。
- 網羅台帳の 3 行が実物と合う。

## Approach

土台のアイコンに、OS の通知を出す口と、押した・消えたの知らせを受ける口を足す。タグは `\!` の受け口を 1 つ足し、知らせは kanade のイベントとして送る。出した通知の題と文字列を覚えておき、知らせが来たら Reference に入れる。

## 議題（要件の段で決める）

1. **時間切れの知らせを受けてから話し始めるまで**。BTS 0000561 のとおり、OS の知らせは通知が消えるより早い。SSP と同じく待つか、待たずに送るか。areka は「時刻は正確に扱う」「1 フレーム遅らせる解は取らない」が原則なので、待つなら理由を要件に書く。
2. **通知が出ている間に次の `\![set,trayballoon,…]` が来たとき**。置き換えるか、並べるか、後のものを捨てるか。置き換えたとき、前の通知の `OnTrayBalloonTimeout` を送るか。
3. **OS が通知を出さなかったとき**（通知の設定が切られている・集中モード）。areka は出なかったことを知る手段が限られる。イベントは送らず、記録だけにするか。
4. **`--timeout`**。今の Windows では無視される。読み捨てて記録するだけでよいか。
5. **`--icon` に無い語・長すぎる文字列**の扱い。
6. **外から届いた台本（SSTP・MCP）からも通知を出せるか**。台本が外へ影響する度合いの段（`script-impact-tiers`）と合わせて決める。

## Scope

- **In**: `\![set,trayballoon,…]` の受け口と 4 つのオプションの読み取り／OS の通知を出す／押した・消えたの知らせを受けて `OnTrayBalloonClick`・`OnTrayBalloonTimeout` を送る／kanade の表と Reference の組み立て／網羅台帳の 3 行／決定論テスト（オプションの読み取り・知らせからイベントを決める所）と実機の確かめ。
- **Out**: 土台そのもの／アイコンと文言の差し替え（`areka-P0-tasktray-ghost-icon`）／`ValueNotify` そのもの（BTS の台帳の候補）／areka 自身が出す知らせ（更新・エラーを通知領域で知らせること。失敗はゴーストの台詞で伝えるのが方針）／OS の新しい通知の仕組み（トースト通知の専用 API）への作り替え。

## Boundary Candidates

- `\!` の受け口（`set,trayballoon`）と引数の判定。
- wintf の通知領域の部品への追加（通知を出す・知らせを出来事にする）。
- kanade のイベント（`schedule/events.rs` の表と組み立て）。

## Out of Boundary

- メニュー。
- キャラクターのバルーン（名前は似ているが別物。こちらは OS の通知）。

## Upstream / Downstream

- **Upstream**: `areka-P0-tasktray-icon`。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: kanade の列で `schedule/events.rs` に触る spec（`mcp-kanade-tools`・`anchor-tag-canon`・`mouse-click-wheel-events` など）と直列。`script-impact-tiers`（議題 6）。`check-script-arg-checks`（引数の判定の形）。

## Constraints

- 利用者の邪魔をしすぎない。通知は OS の設定に従い、OS が断ったものを別の手段で出し直さない。
- 話の最中に届いたイベントの返事の扱いは、今の決まり（roadmap「生きている決まり」）に合わせる。
- 黙って壊れる経路を残さない（`--text` が無い・断られた・読み捨てたを記録する）。
- 実機の確かめは、OS の通知の設定を入れた机で行う。手順に設定の見方を書く。
