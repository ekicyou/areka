# 設計の検証報告: areka-P0-mouse-drag-events

> 2026-10-04。対象は `design.md`（`5bb2dc9b`）。設計が既存コードについて述べていることは、信用せずにワークツリーの実物で確かめた。指し先は「何の定義か」で書く。

## 総評

設計は既存の `OnMouseDoubleClick` の経路へ種類を 2 つ足すだけの形で、境界・依存の向き・記録の方針は今の作りと合っている。製品側の仕組みについて設計が頼っている前提は、下の表のとおり実物で確かめて、すべて成り立った。直すべき点は製品の仕組みではなく、**テストの土台の書き方（1 件）** と **危険の節・前提の書き漏れ（2 件）** で、どれも設計討議かタスク生成の段で手当てできる。判定は **GO（条件つき）**。

## 実物で確かめた前提

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| 位置の保存の受け手を呼んだ直後に `WindowPos.position` が最終位置になっている（決定 D1） | `crates/areka/src/placement/follow/window_move.rs` の `enqueue_window_set_pos`（`bypass_change_detection` で `wp.position` をその場で書く）・`drag_follow.rs` の `on_char_drag_end`（`enqueue_window_set_pos` を呼んでから保存） | 成り立つ。書き込みは同じ呼び出しの中で同期 |
| 取り消しの終了の位置は押した位置 | `crates/wintf/src/ecs/window_proc/keyboard.rs` の `WM_KEYDOWN`（ESC）・`WM_CANCELMODE`・`WM_ACTIVATE`・`WM_CAPTURECHANGED` の 4 か所とも `end_pos: start_pos`・`cancelled: true` | 成り立つ |
| 取り消しで窓が開始の位置へ戻り、終了の座標が開始と同じ値になる（要件 4.2） | `drag_follow.rs` の `policy_mapped_position`（`initial_inset + (cursor − drag_start_pos)` をアンカーへ射影）。取り消しでは `cursor == drag_start_pos` なので開始時の窓位置。`DraggingState` は `dispatch_drag_events` の終了の腕が受け手を呼んだ**後**で外す。キャラ窓は `spawn.rs` の `window_style`（`WS_POPUP`）で枠が無く、`initial_inset` は `WindowPos.position` と同じ値 | 製品では成り立つ（ただし下の問題 1・3） |
| 開始が配られる時点で窓はまだ動いていない | `mouse_move.rs`: 窓を動かすのは `DragStateSnapshot::Dragging` の腕だけ。`Dragging` へ進めるのは `dispatch.rs` の開始の腕の末尾の `update_dragging`（受け手を呼んだ後）。非 Free の `on_char_drag` は遷移の後に配られる `DragEvent` で動く | 成り立つ |
| 画面の位置 − `WindowPos.position` ＝ 窓の中の物理 px | `mouse_click.rs`・`mouse_move.rs` の「スクリーン座標を計算」（`x + pos.x`） | 成り立つ |
| Bubble の相だけ送り、保存の受け手は毎相呼ぶ、で保存は不変（要件 6） | `on_char_drag_end` は Tunnel で `false` を返すだけ。`dispatch_event_for_handler`（`crates/wintf/src/ecs/pointer/dispatch/mod.rs`）は今も同じ受け手を Tunnel・Bubble の 2 回呼んでいる。包みは同じ引数・同じ戻り値。`notify_drag` が触るのは `MouseWiring` と読み取りだけ | 成り立つ |
| 「開始を送った」印が無くても、開始の無い終了・重複は来ない（決定 D2） | `crates/wintf/src/ecs/drag/accumulator.rs` の `set_transition`（開始を積んでいない終了は捨てる・終了を積むと `current_dragging_entity` を空にするので 2 つ目の終了も捨てる）。開始を積むのは `mouse_move.rs` の `Preparing` の腕だけで、`start_dragging` は `Preparing` からしか進まない | 通常の経路では成り立つ（例外は下の問題 2） |
| `placement::spawn` の `OnDragEnd` を置き換えられる・付け直す所は他に無い | `spawn.rs` の `insert(OnDragEnd(on_char_drag_end))` は `spawn_ghost_windows` の中だけ。製品の呼び手は `ghost_session.rs` の 1 か所で、直後に `attach_char_pointer_handlers` | 成り立つ |
| kanade の側（`on_mouse` の先頭の防御・横断の腕・表は 46 語・行数） | `steady.rs` の `on_mouse`・`schedule/mod.rs` の `Input::Mouse` の腕・`events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 46)`。`MouseInput` は `Debug` を持つので `input = ?m` は書ける。`steady.rs` 929 行・`mod.rs` 937 行 | 成り立つ |

## 重大な問題（3 件まで）

### 🔴 問題 1: テスト A4（取り消し）は、設計が書いた土台のままでは期待どおりにならず、D1 を外しても赤にならない

**懸念**: 設計の土台は「偽の `WindowHandle`＋`dispatch_drag_events` へ種を積む」。この組み合わせでは、開始の腕の枠の座標変換（`WindowHandle::client_to_window_coords`）が偽の HWND で失敗し、`DraggingState.initial_inset` が `(0,0)` のまま入る（`crates/areka/src/placement/follow_drag_end_gate_tests.rs` の `destination` の doc が明記している）。すると取り消しで `on_char_drag_end` が窓を送る先は「開始の位置」ではなく「`(0,0)` をアンカーへ射影した位置」になり、A4 の期待「`DragEnd` の座標が `DragStart` と同じ値」は製品と無関係の理由で外れる。加えて A4 の手順は「開始 → 取り消しの終了」だけで、途中で窓を動かす一手が無い。窓が動いていなければ保存の前と後で窓の位置が同じなので、「保存の後の窓の位置から引く決まり」（D1）を外しても A4 は赤にならない。

**影響**: 要件 4.2 の「取り消しの終了は開始と同じ値」と要件 9.2（決まりを外すと赤）が、決定論のテストでは固定されない。実機 R6 だけが頼りになる。

**提案**: A4 の土台と手順を設計に書き足す。⑴ 開始を配った後で `DraggingState.initial_inset` を開始時の窓位置で上書きする（`follow_test_support.rs` の `dragging_state` と同じ、製品の意味の値）。または窓を「`(0,0)` の射影＝開始の位置」になる場所に置く、のどちらかを明記する。⑵ 取り消しの前に窓を実際に動かす（非 Free は `accumulate_delta`＋`update_position` を積んで配り `on_char_drag` に書かせる・Free は窓の手続きの代わりに `WindowPos.position` を書く）。⑶ 取り消しの直前に「窓の位置 ≠ 開始の位置」を assert してから終了を配る。A6 の期待値の式（保存の後の窓の位置）も同じ `initial_inset` の事情に触れておく。

**Traceability**: 要件 4.2・9.1 ⑶⑸・9.2
**Evidence**: design.md「Testing Strategy › areka」の土台の段落と A4・A6 の行／決定 D1

### 🔴 問題 2: 決定 D2 の「残る穴」の書き方が半分で、動かさないクリックに遅れた `OnMouseDragEnd` が出る経路が書かれていない

**懸念**: 設計は穴を「取り消しの終了が積まれず、終了の無い開始が続く」とだけ書く。実物では、終了を積み損ねると `DragAccumulator.current_dragging_entity` が `Some` のまま残る。`set_transition` の終了の番人は「`None` かどうか」だけを見て、どの窓のドラッグかを見ない。そのため**次の動かさないクリック**（`mouse_click.rs` の `handle_button_message` が `Preparing` からでも終了を積む）の終了が番人を通り、`dispatch_drag_events` が配り、包みが `OnMouseDragEnd` を送る（座標はそのクリックの位置）。位置の保存も走る。

**影響**: まれな経路だが、要件 3.1（動かさないクリックでは送らない）と要件 6.3 に反する形で現れる。ゴーストから見ると「開始 → かなり後のクリックで終了」となり、実機で見たときに本 spec の不具合と取り違える。

**提案**: 受ける側の手当ては足さない（D2 のまま・wintf は境界の外）。危険の節と D2 に、この「遅れた終了」の現れ方を 1 行足し、`areka-P0-drag-cancel-borrow-miss` が「積み損ねた後の `current_dragging_entity` の残り」まで直すことを向こうの要件で確かめ、直さないなら起票する。Revalidation Triggers の該当行にも同じ語を入れる。

**Traceability**: 要件 3.1・6.3・8.2、Boundary の「開始の知らせを伴わない終了の知らせは来ないものとして扱う」
**Evidence**: design.md「Design Decisions › D2」「危険と手当て › `drag-cancel-borrow-miss` の穴」「Revalidation Triggers」

### 🔴 問題 3: 「取り消しで窓が開始の位置へ戻る」は、触らないと約束したファイルの、テストで固定されていない振る舞いに乗っている

**懸念**: D1 と要件 4.2 は「取り消しでは保存の受け手が窓を開始の位置へ戻す」を前提にする。実物の `on_char_drag_end` の doc は逆に「cancel も同写像で確定する——開始位置への復元は将来領分」と書いている。今戻るのは、wintf が取り消しの終了に押した位置を載せ、`policy_mapped_position` がそこから再導出するという 2 つの事実の合わせ技で、`placement` の既存テストに取り消しの腕を固定するものは無い（`follow_drag_end_gate_tests.rs`・`follow_drag_tests.rs` に `cancelled: true` の配りは無い）。もう 1 つ、`DraggingState` が無いときの縮退の腕（今の `WindowPos.position` をそのまま使う）では窓は戻らず、終了の座標は開始と違う値になる。

**影響**: `placement` 側の誰かが doc を信じて取り消しの扱いを変えると、要件 4.2 が黙って崩れる。問題 1 の A4 が唯一の檻になるので、A4 が弱いままだと二重に無防備。

**提案**: ⑴ Revalidation Triggers の「`on_char_drag_end` が窓の最終位置を書かなくなったとき」に「取り消しで押した位置から再導出しなくなったとき・`DraggingState` 不在の縮退の腕を通るとき」を足す。⑵ `doc/COMPAT_ARCHITECTURE.md` §8 の「取り消しの Reference0／1」の行に「`DraggingState` を読めない防御の腕では、動かした先の窓から見た値になる」を書くか、書かないと決めた理由を D1 に残す。⑶ `on_char_drag_end` の doc の食い違いは `placement` に触れない約束の外なので、起票に回す。

**Traceability**: 要件 4.2・7.4、Boundary の「位置の保存の中身は変えない」
**Evidence**: design.md「Design Decisions › D1」「Components › `on_char_drag_end_and_notify`」の事後条件／「Revalidation Triggers」

## 良い点

- **位置の保存を包みの先頭で同じ引数のまま呼ぶ形**。送るかどうかの判断がすべて保存の後ろにあるので、送り先が無い・送出に失敗した・送らないと決めた、のどれでも保存が変わらないことが構造で言える。A7 が「包みあり／なし」を同じ操作で比べる組み立ても、要件 6 の「前と同じ」をそのまま判定にしている。
- **新しい状態を持たない**。開始と終了の対は wintf の運ぶ箱の 1 か所に任せ、kanade 側は既存の `on_mouse` の `match` に腕を足すだけ。網羅の `match` と送ってよい表の照合があるので、足し忘れはコンパイルか K3 で止まる。

## 判定

**GO（条件つき）**

**理由**: 製品側の設計が頼る前提（同期の書き込み・相の扱い・対の保証・置き換えの順）は実物で成り立ち、要件 1〜9 の割り当てに穴は無い。残るのはテストの土台の書き足し 1 件と、危険・前提の書き足し 2 件で、作りを変えずに直せる。

**次の一手**:

1. 設計討議で問題 1 を取り上げ、A4（と A6）の土台と手順を design.md へ書き足す（タスク生成の前に）。
2. 問題 2・3 は危険の節・D1・D2・Revalidation Triggers への追記で足りる。`areka-P0-drag-cancel-borrow-miss` の持ち場の確認と、`on_char_drag_end` の doc の食い違いの起票を討議で決める。
3. その後 `/kiro-spec-tasks areka-P0-mouse-drag-events`。
