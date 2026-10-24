# Brief: mouse-hover-event

> 2026-10-24 起票（`/kiro-discovery`）。出どころは `dpi-realign-remembered-chain` のタスク 6.3 の実機の確かめ。開発者「マウスホバーのイベントも出て無いですよね。これもスペックは存在するか？」→ `OnMouseEnter`／`OnMouseLeave` は `mouse-click-wheel-events` の範囲、`OnMouseHover` だけ持ち主が無いと答えたところ、開発者「OnMouseHover を /kiro-discovery で起票して。」

## Problem

キャラクターの当たり判定の上でカーソルを止めて待っても、areka は SHIORI へ何も知らせない。`OnMouseHover` に返事を用意したゴーストは、その返事が 1 度も呼ばれず、「何も喋らない」という形でだけ現れる。記録にも出ない（網羅台帳の note のとおり、組み立てる所が無い）。

## Current State

- 正典（ukadoc「SHIORI Event」の `OnMouseHover`・https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseHover ）: 「マウスが静止している際に発生。」Reference0／1＝カーソルの x／y（ローカル座標）・2＝常に 0・3＝本体 0／相方 1（SSP/CROW では 2 以降もある）・4＝当たり判定の識別子・5＝常に 0・6＝入力の種類（`touch`・`pen`・`eraser`・`mouse`）。**何ミリ秒止まれば起きるか・1 回止まるごとに 1 回か繰り返すか・当たり判定の外で起きるか・ドラッグ中やトークの最中に起きるかには黙る**。
- 網羅台帳（`doc/ukadoc-coverage/ledger/shiori.toml` の `ukadoc:list_shiori_event:OnMouseHover:1`）: `absent`・持ち主なし・priority A12・values「触れ合い」。
- kanade の送ってよいイベントの表 `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`）にこの名前は無く、組み立てる関数も無い。
- 当たり判定の上の出入りを追う所は、`mouse-click-wheel-events`（v0.1.0・作業中）が `crates/areka/src/input_events/` に作る予定（案 `hover_region.rs`）。`OnMouseEnter`／`OnMouseLeave` もそこが送る。静止の判定はその上に乗る。
- カーソルの動きは `crates/areka/src/input_events/mod.rs` の `on_char_pointer_moved` が受け、`OnMouseMove` を間引いて送る（`input_events/throttle.rs`）。
- ぱすた（`pasta-in-windows`）の辞書は `OnMouseHover` を使わない＝v0.1.0 のゴール「ぱすたが SSP と同じに動く」には当たらない。

## Desired Outcome

- キャラクターの当たり判定の上（と、判定の無い所の扱いは要件の段で決める）でカーソルが止まったら、正典の Reference の並びで `OnMouseHover` を送る。
- 止まってから送るまでの時間・繰り返し・ドラッグ中とトークの最中の扱いを、SSP で測った値に合わせて決めてある（正典が黙る所は SSP を一次情報とする）。
- 網羅台帳の行を `implemented`（持ち主 `mouse-hover-event`）へ改め、決定論のテストで「止まると 1 回送る」「動いている間は送らない」「当たり判定の識別子と座標」を固定する。

## Approach

- **最初の段は SSP で測ること**（開発者の機械の SSP・`ghost\master\ssp_shiori_log.txt` の差分で要求を読む・reload を打つときは毎回許可を取る）: ⑴ 止まってから `OnMouseHover` が来るまでの時間 ⑵ 止まり続けたとき繰り返すか ⑶ 当たり判定の無い所で来るか（Reference4 は空か）⑷ ドラッグ中・ボタンを押している間・トークの最中に来るか ⑸ `OnMouseMove` との並び。答えで決める（要件の段の裁定）。
- 静止の判定は、`mouse-click-wheel-events` が作る「今どの当たり判定の上か」を追う所に時計を足す形を推す。時刻は正確に扱う（areka の設計の大原則・待ち時間は丸めない）。
- 送る経路は `mouse-click-wheel-events` が足す単押しのクリックと同じ（kanade の `MouseEventKind` と `ALLOWED_EVENT_IDS` に 1 つ足す）。

## Scope

- **In**: `OnMouseHover` の発火（キャラクターの窓）・SSP での測り・静止の判定と時計・kanade への 1 語の追加・網羅台帳の行・決定論のテスト・`doc/COMPAT_ARCHITECTURE.md` §8 の行（正典が黙る所の決め）。
- **Out**: `OnMouseEnter`／`OnMouseLeave`・単押しのクリック・ホイール（`mouse-click-wheel-events`）。`OnMouseEnterAll`／`OnMouseLeaveAll`。ホバーで見た目を変えるシェルのアニメーション（`shell-hover-animation`）・カーソルの形（`shell-mouse-cursor`）・ツールチップ（`shell-tooltip`）。バルーンの上のホバー（決めるなら要件の段で）。

## Boundary Candidates

- 静止の判定（時計と「動いたら数え直す」）: `input_events/` の当たり判定の追跡の上。
- イベントの組み立て: kanade の `schedule/events.rs` と `msg.rs` の `MouseEventKind`。

## Out of Boundary

- 当たり判定の解決そのもの（`emo2_boot/hit_region.rs`）は変えない。
- `OnMouseMove` の間引き（`throttle.rs`）の決まりは変えない。

## Upstream / Downstream

- **Upstream**: `mouse-click-wheel-events`（当たり判定の上の出入りを追う所・クリックと同じ送り方を先に作る）。
- **Downstream**: `shell-tooltip`（止まって待つ時間の決めを共有できる見込み）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `mouse-click-wheel-events`（`input_events/`・kanade の `msg.rs`・`schedule/events.rs` を分け合う＝その着地の後に置く）・`shell-hover-animation`・`shell-mouse-cursor`・`shell-tooltip`（同じ追跡を読む）。

## Constraints

- 優先度の区分: A（開発者が起票を頼んだ）。版: v0.1.0 には入れない（ぱすたは使わない）。どの版に置くかは次の棚卸（`/kiro-next`）で決める。
- 先に要るもの: `mouse-click-wheel-events` の着地（同じファイルを触る＝同じウェーブに置かない）。測りは開発者の機械の SSP。
- 規模の見立て: S（4〜6。SSP の測り 1・静止の判定 1〜2・kanade 1・台帳と記録とテスト 1〜2）。
- 要件定義のモデル: Opus（先例＝`mouse-click-wheel-events` の単押しのクリック）。
