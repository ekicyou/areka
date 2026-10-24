# 設計の検証: mouse-click-wheel-events

- 対象: `design.md`（2026-10-23 生成）・`requirements.md`（承認済み）・`research.md`
- 検証の日: 2026-10-23
- 判定: **GO**（下の 3 件は設計の討議で直せる大きさで、作りの向きは変わらない）

## 設計の総評

前例（アンカーの 2 段・バルーンの離脱の系・`throttle.rs` の純関数＋値の状態・`arm_deadline`）を組み合わせ、判断を純関数に集めた設計で、要件 1〜12 の各項目が部品と口へ割り当てられている。土台になる主張をコードで確かめ、どれも成り立った（下の表）。直すべき点は、右クリックのメニューの照会の時間の枠、ホイールの要件 6.10 の後半の抜け、ドラッグの状態を読む時点の 3 件だけである。

## コードで確かめた主張

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| 右のクリックの段の列がメニューの照会より先に SHIORI へ届く（要件 5.2） | `ghost_session_runtime.rs` で `MouseWiring` と `MenuWiring` は同じ `runtime.kanade()` の写し。`actor.rs` は受け箱を 1 通ずつ処理し、`drive_translating` が応答を入れ直して動作が尽きるまで回る。`ResourceQuery` は運行表を通らず殻がその場で答える | 成り立つ（包みが先に投函すれば、列が終わってから照会に答える） |
| 新しい系を `dispatch_pointer_events` の前に置ける（要件 7.7） | `wintf/src/ecs/world/mod.rs` で `dispatch_pointer_events` は `Input` の排他の系。areka は既に `.after(dispatch_pointer_events)` で 3 つの系を足している（`menu`・`balloon`・`shell_box_handler`） | 成り立つ |
| 失敗の免除に 1 条件を足せる（要件 1.6・1.8） | `schedule/mod.rs` の `on_shiori_reply` の条件 `state.anchor.is_some() \|\| choice in-flight` → `steady::step`。`steady::on_reply` の先頭がアンカーの腕 | 成り立つ（同じ形で `state.mouse_click.is_some()` を足せる） |
| `PointerState` は 1 窓に高々 1 つで、窓を出ると外れる | `window_proc/mouse_move.rs` は WndProc の中で古い持ち主から外して新しい持ち主へ付け直す（当たらなければ窓の entity）。`WM_MOUSELEAVE` で外す | 成り立つ |
| ホイールの量が誰にも渡らない | `WM_MOUSEWHEEL` は窓の entity の `WHEEL_BUFFERS` に足すだけで、`transfer_buffers_to_world` は読まず、空にもしない。`PointerState.wheel` は `FrameFinalize` の `clear_transient_pointer_state` で 0 に戻る | 成り立つ |
| 拡張ボタンの押下が配られない | `dispatch_pointer_events` の条件が `left_down \|\| right_down \|\| middle_down` だけ | 成り立つ |
| 閾値を越えたかが休みに残らない | `drag/state/mod.rs` の `close_drag` は `started` を種を積むのにだけ使い、`JustEnded` に残さない。2 回目の押下（`WM_LBUTTONDBLCLK`）も `start_preparing` を呼ぶ | 成り立つ |
| `MouseEventKind` の腕を足して直す `match` は 2 か所 | `steady::on_mouse` と `input_events/drag.rs` の `notify_drag` だけ | 成り立つ |
| 1 ファイル 1,000 行の余裕 | `steady.rs` 957・`events.rs` 888・`ghost_session.rs` 917・`input_events/mod.rs` 576・`menu/mod.rs` 373・`drag/state/mod.rs` 638・`drag/state/tests.rs` 812・`pointer/buffers.rs` 558・`pointer/dispatch/tests.rs` 557 | 見込みの足し分（`steady.rs` +10・`events.rs` +25 ほど）で上限を越えない |
| 手書きの数 55 | `events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 55)`。9 語はどれも表に無い | 成り立つ |

## 重大な問題（3 件）

🔴 **重大な問題 1**: 右のクリックの段の列が、メニューの照会の待ちの枠（1 秒）を食う
**問題**: 照会の期限 `QUERY_TIMEOUT`（`menu/captions.rs`・1,000 ms）は投函の時刻から数える。殻は照会より先に右のクリックの段の列（`OnMouseUp` → `OnMouseClick`、台本が返れば翻訳の往復も）を終わらせるので、その時間がそのまま枠から引かれる。期限を過ぎると、メニューは全部を既定の名前にして出し、`popupmenu.visible`＝0 の抑止も効かない。
**影響**: 右クリックを自分で受けてメニューを抑えるゴーストで、`OnMouseClick` の処理が重いと、抑えたはずのメニューが出る。要件 5.3（右のクリックへの返事によらずメニューは今のとおり）に反する経路だが、設計の「Open Questions / Risks」に無い。
**提案**: `trigger.rs` を直さない線は保ったまま、R4 として登記する（枠の 1 秒のうち段の列に使える分・期限を過ぎたときに出る記録の行 `menu` の「全件既定名」の警告）。実機の確かめ（要件 12.5）に「右のクリックに台本を返し、かつ `popupmenu.visible`＝0 のゴーストでメニューが出ない」を 1 手順足す。枠が足りないと分かったら、そのときに `trigger.rs` を触る spec を起票する。
**当たる要件**: 5.2・5.3・12.5
**設計の箇所**: 「System Flows」の左・右のクリック／「ReleaseWrap」の Ordering／「Open Questions / Risks」

🔴 **重大な問題 2**: 要件 6.10 の「絵の透けた所の上」が設計で塞がれていない
**問題**: 設計は 6.10 を「持ち主が居ない窓の溜めは捨てる」（`pointer/buffers.rs`）で満たすとしている。だが wintf の当たり判定は α のマスクで、透けた画素の上では `PointerState` は窓の entity そのものへ移り（`mouse_move.rs` の「Window の空白部分」の枝）、持ち主は居ることになる。クリック透過（`WS_EX_TRANSPARENT`）の切り替えは別の監視が後から当てるので、その間はホイールの知らせも窓に届く。このとき `char_watch` は窓の上と見て、当たり判定が空の `OnMouseWheel` を送る。
**影響**: 透けた所の上でホイールを回すと、SSP では起きない `OnMouseWheel` が届くことがある。追跡表の 6.10 の行が実際の作りと合わない。
**提案**: `char_watch` で「持ち主が窓の entity そのもの（絵に当たっていない）」ならホイールを捨て、理由を trace で残す、の 1 行を設計に足す。`char_watch_tests.rs` に「窓の entity に `PointerState`・`wheel.vertical` あり → 送らない」の 1 件を足す。出入りの側（透けた所で `Leave`、その後の `WM_MOUSELEAVE` で `LeaveAll`）は今の設計のままでよいが、その順を §8 の 7 行目に書き添える。
**当たる要件**: 6.10・11.2
**設計の箇所**: 「Requirements Traceability」6.10 の行／「CharWatch」／「WintfHoles」

🔴 **重大な問題 3**: ドラッグの終わりを「解放の受け手が動く時点の今の状態」で読むので、1 tick に解放と次の押下が入ると、ドラッグの後にクリックを送る
**問題**: 左の判定は `snapshot_drag_state()` が `JustEnded` のときだけ `ended_after_start()` を見て、ほかは `Clean` にする。WndProc の中で状態は同期に変わるので、ドラッグの解放と次の左押下が同じ tick に入ると、受け手が読む時点の状態は次の押下の `Preparing` で、ドラッグだった解放が `Clean`＝クリックになる。設計の R1 はダブルクリックの順の崩れだけを書き、この場合を書いていない。
**影響**: UI スレッドが詰まった時（大きな面の読み込みなど）に、窓を動かした直後の押し直しで `OnMouseClick` が届く。ぱすたの × のように、クリックで終わるゴーストではドラッグの後に終わってしまう（要件 3.1 の「ドラッグの後は送らない」に反する）。
**提案**: `drag/state/mod.rs` の印を「最後に休みへ移したときの `started`（と `cancelled`）」とし、次の `start_preparing` で消さずに残す。読み口は今の状態が `JustEnded` かどうかによらずその印を返す（`ended_after_start` を「直前の左の解放はドラッグだったか」と読める名前に改めてもよい）。判定は「控えのある左の解放 → 直前の閉じがドラッグか取り消しなら `Dragged`」にする。触るファイルは議題 6 の 3 つのままで済む。`drag/state/tests.rs` に「`start_dragging` → 解放 → `start_preparing` の後も印が真」の 1 件を足し、R1 の説明にこの場合が塞がったことを書く。
**当たる要件**: 3.1・11.5
**設計の箇所**: 「PressNote・ClickJudge」の左のドラッグの読み方／「WintfHoles」の `drag/state/mod.rs`／「Open Questions / Risks」R1・R3

## 設計の良い点

1. **前例の形に乗った 2 段と失敗の免除**: 段の記憶を `State` に 1 つ置き、殻が応答を次の知らせより先に入れ直す性質で「段の間に他を挟まない」（要件 4.2）を満たす。アンカーと同じ免除の条件に 1 つ足すだけで、`actor.rs` に触らない。コードで確かめて、受け箱の順と照会の答え方まで設計の主張どおりだった。
2. **判断を純関数と値の状態に集め、境界を守った**: `plan_click`・`judge_release`・`plan_hover`・`plan_wheel` が表で決まり、受け手と系は「読む → 純関数 → 送る」だけ。`menu/trigger.rs` と移動の受け手を直さないので今の檻が変えずに通り（要件 11.4）、wintf は議題 6 の 3 ファイルに収まり、同じウェーブの `dpi-realign-remembered-chain` の持ち物に触らない。

## 最終の判定

- **判定**: **GO**
- **理由**: 作りの向き・層の分け方・依存の向きは今の構造と合い、要件の全項目に実装の道筋がある。3 件はどれも設計の文の数行と決定論のテスト 1〜2 件で直せ、触るファイルの範囲も変わらない。
- **次の手順**: 設計の討議（`/kiro-design-discussion mouse-click-wheel-events`）で上の 3 件を議題にし、決めた直しを `design.md` へ入れてから `/kiro-spec-tasks mouse-click-wheel-events` へ進む。
