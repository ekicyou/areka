# Brief: areka-P0-mouse-drag-events

> 2026-10-04 `/kiro-discovery` で起票（開発者「悪役令嬢クローディアがドラッグ中に足の浮いた絵になるのがかっこいい・areka では出ない・どのイベントか調べて areka 側で対応せよ」）。本文のソースの指し先は起票時（`66eaa2b5`）の実物＝着手時に引き直すこと。

## Problem

- **利用者**: キャラクターをドラッグで動かしても、ゴーストが反応しない。SSP では、ゴースト「悪役令嬢クローディア」がドラッグの間だけ足の浮いた絵に替わり、離すと元の顔に戻ってひとこと言う。areka ではこれが一切起こらない。
- **原因（調べた結果）**: クローディアは SHIORI イベント `OnMouseDragStart`・`OnMouseDragEnd` でこれをしている。areka はこの 2 つを送っていない。
  - ゴーストの辞書 `ghost/master/dic/normal/yaya_mouse.dic`（YAYA）の該当箇所:
    - `OnMouseDragStart`: Reference5 が `0`（左ボタン）以外なら何もしない。Reference3 が `0` なら `\0\s[29]\e`、`1` なら `\1\s[19]\e` を返す（浮いた絵へ替えて黙る）。
    - `OnMouseDragEnd`: 同じ判定のあと、Reference3 ごとに台詞を返す（`\s[10]`・`\s[7]` などへ戻し、「足が床に着いていませんでしたわ」などと言う）。
  - 絵の大きさはどれも 333×500 で同じ。ドラッグ中に窓の大きさは変わらない。
- 網羅台帳では 2 つとも `absent`（`doc/ukadoc-coverage/ledger/shiori.toml` の `OnMouseDragStart:1`・`OnMouseDragEnd:1`・優先度 A12・価値「触れ合い」）。台帳の記述は「ベースウェアがこのイベントを発火する経路が areka に無い」。

## 正典（ukadoc `list_shiori_event` の `OnMouseDragStart`／`OnMouseDragEnd`）

- 発生: マウスでドラッグし始めた／し終えたとき。どちらもパッシブモードでは抑えられる。
- Reference0／1＝カーソルの x／y（ローカル座標）・Reference2＝ホイールの回転量・Reference3＝本体 0／相方 1（SSP では 2 以降もある）・Reference4＝当たり判定の識別子・Reference5＝左クリック 0／右クリック 1・Reference6＝入力デバイス（`touch`・`pen`・`eraser`・`mouse`）。
- 並びは既にある `OnMouseDoubleClick` と同じ 7 つ。

## Current State

- **kanade（送る側）**: マウスのイベントは `OnMouseMove` と `OnMouseDoubleClick` の 2 つだけ。
  - 組み立ては `crates/areka-kanade/src/schedule/events.rs` の `on_mouse_move`・`on_mouse_double_click`。
  - 送ってよい名前の表は同じファイルの `ALLOWED_EVENT_IDS`（46 個）。個数は `events_tests.rs` のテストで固定されている。
  - 知らせの型は `crates/areka-kanade/src/msg.rs` の `MouseInput`／`MouseEventKind { Move, DoubleClick{button} }`。
  - 振り分けは `crates/areka-kanade/src/schedule/steady.rs` の `on_mouse`。終了の握手の待ちの間だけ送らない。トーク中も `Status: talking` で送る。
- **areka（受ける側）**: `crates/areka/src/input_events/mod.rs` の `MouseWiring` が wintf のポインタの知らせを kanade へ渡す。
  - スコープは `CharWindowMarker.scope` から取る。当たり判定と面の座標は `resolve_hit_owned`（窓のクライアントの物理 px を渡す）で引く。
  - ハンドラはキャラクター窓にだけ付ける（`attach_char_pointer_handlers`）。バルーン窓からはマウスのイベントを送らない。
- **ドラッグ（wintf）**: 左ボタンだけで始まる（`DragConfig` の既定）。閾値（5 px）を越えたら開始、離しか取り消しで終了。
  - `drag-click-without-move`（完了）で、開始を積んでいない終了は累積器が捨てるようになった。動かさないクリックでは開始も終了も届かない。
  - `crates/wintf/src/ecs/drag/dispatch.rs` が `DragStartEvent`／`DragEndEvent`（画面の物理 px・終了は `cancelled` 付き）を順に配り、`OnDragStart`／`OnDragEnd` を呼ぶ。
- **areka のドラッグの受け手**: `crates/areka/src/placement/spawn.rs` がキャラクター窓に `OnDrag`・`OnDragEnd(on_char_drag_end)` を付ける（終了は位置の保存）。**`OnDragStart` はどこにも付いていない**。
- **パッシブモード**: 状態の語として `crates/areka-kanade/src/status.rs` にあるだけで、入る経路がまだ無い（`SEAM(Req6.1/6.3)`）。

## Desired Outcome

- キャラクター窓を閾値を越えてドラッグし始めたら `OnMouseDragStart` が 1 回、離したら（取り消しも含む）`OnMouseDragEnd` が 1 回、ukadoc の並びの 7 つの Reference で SHIORI へ届く。
- 動かさないクリックではどちらも届かない（`drag-click-without-move` の決まりを守る）。
- 実機でクローディアがドラッグ中に `\s[29]`（相方は `\s[19]`）になり、離すと台詞を言う。
- 届くこと・届かないことを決定論のテストで見る（開始 → 終了の順・動かさないクリックで 0 件・Reference の並び）。
- 網羅台帳の 2 行を `absent` から実装済みへ直す。

## Approach

既にある `OnMouseDoubleClick` の経路をそのまま伸ばす（推奨・採用）。
- kanade: `MouseEventKind` に開始・終了の 2 つを足し、組み立ての関数（`on_mouse_double_click` と同じ形）と `ALLOWED_EVENT_IDS` の 2 行、`on_mouse` の腕を足す。
- areka: `input_events` でキャラクター窓に `OnDragStart` を付け、`OnDragEnd` は今の `on_char_drag_end`（位置の保存）を呼んだうえで知らせを送る包みに替える（`placement` から `crate::` を引かない層の決まりを守るため）。
- 座標: 開始は押した位置（`DragStartEvent` の位置）、終了は離した位置。どちらも画面の物理 px から窓の原点を引いてクライアント px にし、`resolve_hit_owned` で当たり判定と面の座標を引く。

退けた案:
- wintf に新しい知らせを足す案: 開始・終了の知らせは既に wintf にあり、足す理由がない。
- `on_char_drag_end` の中から直接送る案: `placement` が `crate::` へ依存して層の決まりを破る。

## Scope

- **In**: `OnMouseDragStart`・`OnMouseDragEnd` の組み立て・送ってよい名前の表・kanade への知らせの型・areka のキャラクター窓での開始と終了の捕まえ方・決定論のテスト・網羅台帳の 2 行・実機でクローディアを使った確認。
- **Out**:
  - パッシブモードでの抑え（入る経路がまだ無い。縮退の継ぎ目として印だけ残す）。
  - 右ボタンのドラッグ（wintf の既定で始まらない。Reference5 は今は常に `0`。右ドラッグを有効にするかは別の話）。
  - バルーン窓のドラッグ（ukadoc のイベントはキャラクターのもの。今の「バルーンからマウスのイベントを送らない」決まりのまま）。
  - タッチ・ペン（Reference6 は今の他のマウスイベントと同じく `mouse`）。
  - `OnMouseClick`・`OnMouseDown`／`Up` など、まだ無い他のマウスイベント（網羅の段階 A の「撫で」の群れ＝`touch-events-canon` の下書きの側）。

## Boundary Candidates

- kanade の側: 知らせの型・組み立て・送ってよい名前の表・振り分け
- areka の側: キャラクター窓の開始と終了の捕まえ方と、位置の保存との並び

## Out of Boundary

- wintf のドラッグの状態機械と累積器（`drag-click-without-move`・`drag-cancel-borrow-miss` の持ち場）
- 位置の保存そのもの（`on_char_drag_end` の中身）
- パッシブモードの出入り（網羅の段階 A の「動作モードの出入り」）

## Upstream / Downstream

- **Upstream**: `drag-click-without-move`（完了・動かさないクリックで開始も終了も届かない）・`translate-pipeline`（完了・kanade の進行の列の直前）
- **Downstream**: 将来の「撫で」の群れ（`OnMouseClick` などを足すとき、同じ `MouseEventKind` と表へ行を足す）

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec `areka-P0-input-events`〔`OnMouseMove`・`OnMouseDoubleClick` の経路〕・`event-drag-system`〔wintf の開始・終了の知らせ〕・`drag-click-without-move` の約束の上に乗る）
- **Adjacent**:
  - `balloon-lifecycle-events`（C3・同じ `crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS` とその個数のテストへ行を足す＝**同時に走らせない**）。
  - `drag-cancel-borrow-miss`（wintf の累積器・触るファイルは重ならない）。

## Constraints

- 直列の列「kanade の進行」（`events.rs`・`steady.rs`・`msg.rs`）に属する。同じ列の spec と同時に走らせない。
- 1 フレーム遅らせる解は取らない。
- 決定論のテストは x64 の偽の境界で組む。実機の根と一時フォルダはワークツリーの `target\` の下だけ。
- 規模の見立ては S（6〜10 タスク）。
- 段は**優先**（起票の日に開発者が上げた）。roadmap の C2-⑦。C2-① `shell-balloon` と `crates/areka/src/input_events/` を分け合うので、着手の前に `shell-balloon` の実際の変更と照合する。
  - 10-04 に shell balloon セッションから聞いた範囲: `input_events/mod.rs` に mod 宣言 2 行（`shell_box`・`shell_box_handler`）と、`on_char_pointer_moved`・`on_char_pointer_pressed` の先頭に箱の上の操作をシェルへ送らない早期 return を足した。`attach_char_pointer_handlers` の本体には触っていない。
  - 箱の中の押下は、選択肢と中断以外はシェルへの操作（`shell-balloon` 要件 9.1）。ドラッグの送出は wintf の `OnDragStart`／`OnDragEnd` の側に置くので、この前段とは別の経路になる。箱の上から始めたドラッグもシェルのドラッグとして届くことを、着手のときに確かめる。
