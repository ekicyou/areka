# ギャップ分析: areka-P0-drag-click-without-move

> 2026-10-03 `/kiro-validate-gap`。対象は確定済みの `requirements.md`（要件 1〜5）。ソースの指し先はこの日のワークツリー（`cbc394e9`）の実物で、行番号ではなく「何を定める行か」で指す（行番号は目安として括弧に添える）。

## 分析の要約

- **終了の知らせを積む所は 6 か所で、そのうち 5 か所が「動かし始める前」（`Preparing`）でも積む。** `mouse_click.rs` の左ボタンを離したときの 2 つの枝（当たり判定が取れた枝・取れなかった予備の枝）と、`keyboard.rs` の ESC（`WM_KEYDOWN`）・`WM_CANCELMODE`・`WM_CAPTURECHANGED` である。残る 1 か所の `WM_ACTIVATE` だけが既に `Preparing` を除いている（先例）。brief と棚卸⑳の記述はすべて実物と一致した。
- **終了の知らせ（`DragEndEvent`）を受けて位置を書く所は areka の 2 つの受け手だけ**（`on_char_drag_end`・`on_balloon_drag_end`）。位置の記憶へ書く関数 `persist_entries` の製品の呼び手もこの 2 つだけで、ほかの読み手（右クリックメニュー・クリック透過・ゴーストへのマウスのイベント・バルーンの表示の段）は知らせでなくドラッグの状態か `WindowDragging` の印を読む。よって wintf で知らせを止めれば、1 か所の修正で両方の保存が止まる。
- **wintf 側で止める形は 3 案ある。** (A) 5 か所それぞれに先例の形で `Preparing` を除く、(B) 「終了を積むか」の判断を `drag/` の関数 1 つに寄せて 6 か所がそれを通る、(C) 累積器（`DragAccumulator`）の `set_transition` が「開始を積んでいない終了」を捨てる。いずれも areka の縮退（要件 2.4）と速いドラッグ（要件 2.3）を壊さない。違いは主に「直す前に赤になる areka 側のテスト（要件 5.1）を、修正前の HEAD でコンパイルできる形で組めるか」と「既存テストへの響き」にある。
- **areka 側だけで分ける案（brief の (b)）は単独では要件 3 を満たさない**（要件 3.1・3.2 は wintf が知らせを配らないことを求める）。参考として残す。
- **調べ残し**: 速いドラッグで開始の知らせが 1 枠の上書きで消える件（多窓の縮退の本当の原因の仮説）・クリックのたびに走っていた `Arrangement.offset` の同期が消えることの影響・当たり判定が取れた枝を決定論テストで通す組み立て・実機の `RUST_LOG` の target 名。

---

## 1. 今の作り（調べた事実）

### 1.1 ドラッグの状態と、終了・開始の知らせが生まれる所

ドラッグの状態はスレッドごとの 1 個（`crates/wintf/src/ecs/drag/state/mod.rs` の `DRAG_STATE`）で、`Idle` → `Preparing`（左押下）→ `JustStarted`（閾値到達）→ `Dragging`（次の画面更新で開始を配った後）→ `JustEnded`（離す・取り消し）と移る。`JustEnded` から出るのは次の左押下だけ（同ファイルの `DragState` の doc・完了 spec `areka-P0-wintf-drag-state-rest-contract` の決め）。

状態の移り変わりと、ECS へ渡す「知らせの種」（`DragTransition`）は別々に作られている。wndproc の各ハンドラが「状態を移す関数」と「累積器へ種を積む `set_transition`」を並べて呼ぶ形である。

**開始の種（`DragTransition::Started`）を積む所は 1 か所だけ**:
- `crates/wintf/src/ecs/window_proc/mouse_move.rs` の `WM_MOUSEMOVE` の `Preparing` の腕。閾値（`DragConfig.threshold`・距離の 2 乗で比べる）に届いたとき `start_dragging` で `JustStarted` へ移し、`set_transition(Started)` を積む（236〜254 行目付近）。

**終了の種（`DragTransition::Ended`）を積む所は 6 か所**（ワークスペース全体を `DragTransition::Ended` で検索した結果。テストと累積器の定義を除く）:

| # | 場所（何を定める所か） | どの状態から積むか | `Preparing` を含むか | `cancelled` |
|---|---|---|---|---|
| 1 | `window_proc/mouse_click.rs` の `handle_button_message` の左解放・**当たり判定が取れた枝**（`should_end` の match と、その下の `set_transition(Ended)`・176〜213 行目付近） | `Dragging`（HWND 一致）・`Preparing`／`JustStarted`（対象の持ち主の窓が一致） | **含む** | false |
| 2 | 同関数の**予備の枝**（当たり判定が取れないとき・245〜282 行目付近） | 同上 | **含む** | false |
| 3 | `window_proc/keyboard.rs` の `WM_KEYDOWN`（ESC の分岐・33〜56 行目付近） | `Dragging`・`Preparing`・`JustStarted` | **含む** | true |
| 4 | 同ファイルの `WM_CANCELMODE`（79〜102 行目付近） | 同上 | **含む** | true |
| 5 | 同ファイルの `WM_ACTIVATE`（非活性化・140〜173 行目付近） | `Dragging`・`JustStarted` のみ。`Preparing` は **`cancel_dragging` だけ呼んで種を積まない**（「drag prepare, resetting」の debug 行） | **除く（先例）** | true |
| 6 | 同ファイルの `WM_CAPTURECHANGED`（230〜252 行目付近） | `Dragging`・`Preparing`・`JustStarted` | **含む** | true |

補足:
- 6 か所とも、状態は種を積んだ直後に `end_dragging`（#1・#2）か `cancel_dragging`（#3〜#6）で `JustEnded` へ移る。状態の移り変わりと捕捉（`CaptureGuard`）の解放は種を積むかどうかと独立なので、種だけを止めても要件 3.4（状態の移り変わりと捕捉の解放は今どおり）は保てる。
- 離したときの `end_dragging` は、状態を `JustEnded` にしてから `CaptureGuard` を落とす（`state/mod.rs` の `end_dragging` の「borrow 解放後にドロップ」のコメント）。その `ReleaseCapture` が同期で送る `WM_CAPTURECHANGED`（#6）は、状態が既に `JustEnded` なので `was_dragging=false` で何もしない。**離したときに終了の種が二重に積まれることは今も無い。**
- 左押下でドラッグの準備に入るのは `mouse_click.rs` の左押下の分岐（`find_ancestor_with_drag_config` で `DragConfig` を探し `start_preparing` を呼ぶ・145〜167 行目付近）だけ。`WM_LBUTTONDBLCLK`（`mouse_dblclick_wheel.rs`）と右押下は準備に入らない（`start_preparing` の呼び手は製品で 1 か所）。

### 1.2 累積器（`crates/wintf/src/ecs/drag/accumulator.rs`）

- 種の置き場 `pending_transition` は **1 枠**で、`DragAccumulator::set_transition` は上書きする。
- 同じ関数は、`Started` を積むと `current_dragging_entity = Some(entity)`、`Ended` を積むと `None` にする。`flush` はこの値を消さない（`flush` は累積の差分を 0 に戻し、種を取り出すだけ）。**つまり `current_dragging_entity` は「開始の種を積み、まだ終了の種を積んでいない間」だけ `Some` で、製品でこれを変える所は `set_transition` しかない。** これは「開始を積んだドラッグか」をそのまま答える既存の値である（案 C の拠り所）。
- `Preparing` のまま離すと、`Started` は一度も積まれず `current_dragging_entity` は `None` のまま `Ended` が積まれる＝「開始の無い終了」。

### 1.3 配る所（`crates/wintf/src/ecs/drag/dispatch.rs` の `dispatch_drag_events`）

`Ended` の腕（215〜335 行目付近）が行うこと:
1. `DragEndEvent` を `Messages<DragEndEvent>` へ書き、`OnDragEnd` へ Tunnel／Bubble で配る（`[DragEndEvent] Dispatching` の info 行）。
2. 対象の `DraggingState` を外す（無ければ何もしない）。
3. 持ち主の窓の `WindowDragging` の印を外し、`Arrangement.offset` を `WindowPos.position` へ直接そろえる（「Direct Arrangement.offset sync」）。
4. `WindowDragContextResource` を空にする。

`Preparing` のまま離した場合、2・4 は元から空振り（`DraggingState`・`WindowDragging`・`WindowDragContext` は `Started` の腕でしか入らない）。**3 の `Arrangement.offset` の同期だけは、今は動かさないクリックのたびにも走っている**（調べ残し R2）。

`Started` の腕（84〜213 行目付近）は `DraggingState` と `WindowDragging` を入れ、`DragStartEvent` を配り、`update_dragging` で `JustStarted` → `Dragging` へ移す。

`dispatch_drag_events` と `cleanup_drag_state` は `crates/wintf/src/ecs/world/mod.rs` の Input スケジュールに登録され（後者は前者の後）、`Messages<DragEndEvent>` は FrameFinalize で `update` される。

### 1.4 `DragEndEvent` を読む所（全数）

ワークスペース全体を `DragEndEvent`／`OnDragEnd` で検索した結果、製品で読む所は次の 3 つだけ:

| 読み手 | 何をするか | 動かさないクリックで呼ばれると |
|---|---|---|
| `crates/areka/src/placement/follow/drag_follow.rs` の `on_char_drag_end`（`crates/areka/src/placement/spawn.rs` で全キャラ窓へ `OnDragEnd` として結線・514 行目付近） | 最終位置を求めて窓へ置き、バルーンを追従させ、位置を記憶へ書く | 位置を書く（症状の本体） |
| 同ファイルの `on_balloon_drag_end`（`spawn.rs` でバルーン窓へ結線・457 行目付近） | 相対位置を記憶へ書き、キーワードで決めた位置の素材（`BalloonKeywordBase`）を外し、はみ出しの補正をかける | 相対位置を書き、素材を退役させる（同じ形・未観測） |
| `crates/wintf/src/ecs/drag/systems.rs` の `cleanup_drag_state`（`MessageReader<DragEndEvent>`） | 対象の `DraggingState` を外す | 空振り（`Preparing` では入っていない） |

ほかに `crates/wintf/examples/taffy_flex_demo/drag.rs` の `on_container_drag_end` が info 行を出すだけの受け手として在る（例。修正後は動かさないクリックでこの行が出なくなる）。

**知らせでなく状態を読む所（修正の影響を受けない）**:
- `crates/areka/src/menu/trigger.rs` の右解放の扱い（`snapshot_drag_state().is_button_held()` で押している間だけ無視・209 行目付近）。
- `crates/wintf/src/ecs/clickthrough/controller.rs` の `resolve_transition`（`Dragging | JustStarted` だけを「移動中」と読む）。
- `crates/wintf/src/ecs/pointer/nchittest_cache.rs` の当たり判定（押している間か）。
- `crates/areka/src/emo2_boot/balloon_visibility_phase.rs`（`WindowDragging` の印を読む。印は `Started` の腕でしか入らないので、動かさないクリックでは元から立たない）。
- ゴーストへ伝えるマウスのイベント（`crates/areka/src/input_events/`）は `DragEndEvent`・`DraggingState`・`WindowDragging` のどれも読まない（検索で 0 件）。

### 1.5 位置の記憶へ書く所

- 製品で `persist_entries`（`crates/areka/src/placement/persist.rs` の定義・485 行目付近）を呼ぶのは、`on_char_drag_end` の保存の腕（「char DragEnd 保存」の info 行・target `areka::persist::save`）と `on_balloon_drag_end` の保存（「balloon DragEnd 保存」）の 2 か所だけ。位置を書く観測点がドラッグの終了だけである（完了 spec `areka-P0-position-persist` 要件 1.9「ユーザーの明示的なドラッグ確定によってのみ更新する」）ことはコードでも成り立っている。
- `on_char_drag_end` の縮退: 最終位置は `policy_mapped_position`（`DraggingState` から生の座標を戻す関数）で求め、`DraggingState` が無いと「DraggingState 不在のため生ドラッグ座標を復元できない（写像スキップ）」の debug 行を出して `None` を返す。受け手はそのとき今の `WindowPos.position` へ縮退して**必ず保存する**（同関数の「最終位置の第一義は DraggingState からの…」のコメント）。brief の A1r・A2r の 3 件はこの道筋と一致する。
- この縮退を固定している既存テストは `crates/areka/src/placement/follow_drag_end_persist_tests.rs` の `dragged_char_persists_even_without_dragging_state_at_dragend`（`DraggingState` 無しの scope0 と有りの scope1 を並べ、両方が保存・復元されることを見る）。受け手を直接呼ぶ形なので、wintf 側の修正では赤くならない＝要件 2.4 の「修正の前後とも通る」に使える。
- 動かさないクリックで `on_balloon_drag_end` が呼ばれると、保存に続いて `retire_keyword_base_on_save`（キャラ窓の `BalloonKeywordBase` を外す）と `apply_release_limit_correction`（`BalloonLimit` が有効ではみ出していれば表示位置を補正）も走る。後者は「クリックしただけでバルーンが動く」ことがありうる経路で、要件 1.4（クリックの前の位置から動かさない）にも関わる。wintf で知らせを止めればこれらも走らなくなる。

### 1.6 クリックの手順ごとの今の流れ

| 手順 | 届くメッセージ | 今の終了の種 | 修正後（どの案でも） |
|---|---|---|---|
| 1 回のクリック | 左押下 → 左解放 | 左解放で 1 件（`Preparing` から） | 0 件 |
| ダブルクリック | 左押下 → 左解放 → `WM_LBUTTONDBLCLK` → 左解放 | 1 回目の左解放で 1 件。`DBLCLK` は準備に入らないので 2 回目の左解放は状態が `JustEnded` で `should_end=false` | 0 件 |
| 左クリックの後の右クリック | 左押下 → 左解放 → 右押下 → 右解放 | 左解放で 1 件（右は準備に入らない） | 0 件。右解放でメニューを出す判断は `is_button_held()` を読むだけなので変わらない |
| 押して ESC／メニューやダイアログ（`WM_CANCELMODE`）／捕捉の喪失 | 押下 → 各メッセージ | 1 件（`cancelled=true`） | 0 件 |
| 押して非活性化（Alt+Tab） | 押下 → `WM_ACTIVATE(WA_INACTIVE)` | 0 件（先例） | 0 件 |
| 閾値を越えて動かして離す | 押下 → 移動（`Started`）→ …→ 左解放 | 1 件 | 1 件 |
| 速いドラッグ（閾値を越えて離すまでが 1 回の画面更新の間） | 押下 → 移動（`Started`）→ 左解放（`Ended`） | `Started` が 1 枠の上書きで消え、`Ended` だけ 1 件 | 1 件（どの案も `JustStarted` からの終了は残す） |

### 1.7 速いドラッグと「開始の無い終了」の残り

- 閾値到達で積んだ `Started` が配られる前に左解放が来ると、`set_transition(Ended)` が `Started` を上書きする。`dispatch_drag_events` は `Ended` の腕だけを通り、`DragStartEvent`・`DraggingState`・`WindowDragging` は一度も出ない。キャラ窓は `move_window=false` で、ドラッグ中の移動は `on_char_drag`（`DragEvent` で動く）に頼るので、この場合は窓が 1 px も動かないまま、受け手の縮退が**今の位置（＝押す前の位置）を 1 件保存する**。
- 要件 2.3 はこの場合も「保存 1 件」を求めているので、今の扱いを残すことが要件と整合する。ただし要件 3 の目的（「必ずそれに先立つ開始の知らせと対になって届く」）は、この場合だけ満たされないまま残る（要件 3.3 が求めるのは終了を落とさないことまで）。
- 棚卸⑳の仮説「多窓で `DraggingState` が先に落ちる縮退の本当の原因はこの上書きかもしれない」は、静的には半分だけ合う。上書きの場合は `DragEvent` も来ないので窓は動かないが、縮退のテストのコメントは「連続 `on_char_drag` が既に最終位置へ動かし済み」の場面を想定している。`DraggingState` を外す製品の所は `dispatch.rs` の `Ended` の腕と `cleanup_drag_state` の 2 つだけで、ほかに先に落とす道は静的には見つからなかった（調べ残し R1）。

### 1.8 既存のテストと、触りうるファイルの行数

**ドラッグの判断に関わる既存の決定論テスト**:

| ファイル（行数） | 見ているもの | 動かさないクリックの判断を通るか |
|---|---|---|
| `crates/wintf/src/ecs/window_proc/keyboard_tests.rs`（70） | `WM_ACTIVATE` を `crate::ecs::dispatch_window_message` 経由で配り、`JustStarted` からの非活性化で `Ended{cancelled:true}` が積まれること | 通らない（`Preparing` の場面が無い） |
| `crates/wintf/src/ecs/drag/state/tests.rs`（539） | 状態関数の移り変わり（`end_dragging` は `Preparing` から `JustEnded` へ、など）・休む決まり | 通らない（種を見ない）。要件 3.4 の確かめに使える |
| `crates/wintf/src/ecs/drag/accumulator.rs` の中のテスト | 累積器の `set_transition`・`flush` | `Ended` を積む前に必ず `Started` を積んでいる |
| `crates/wintf/tests/drag/dispatch_test.rs`（426） | 配る所の `Started`／`Ended` の腕 | **`Started` を積まずに `Ended` だけを積むテストが 2 本**（`dispatch_ended_removes_state_marker_and_syncs_offset`・`dispatch_ended_cancelled_propagates_flag`） |
| `crates/wintf/tests/layout/boxstyle_coordinate_separation_test/drag_lifecycle.rs`（379） | 配る所と `WindowDragging`・`Arrangement` の変化 | **`Started` を積まずに `Ended` だけを積むテストが 3 本**（`test_drag_end_syncs_window_pos_changed`・`test_drag_end_clears_context_resource`・`test_window_dragging_removed_on_drag_end`）。`test_window_dragging_full_lifecycle` の `Ended` は前に `Started` を積んでいる |
| `crates/areka/src/placement/follow_drag_end_persist_tests.rs`（712） | 受け手を直接呼んでの保存・復元（バルーン・往復・縮退） | 通らない（受け手を直接呼ぶ） |
| `crates/areka/src/menu/trigger_flow_tests.rs`（905） | 左クリックの後の右解放でメニューへ進むこと（`JustEnded` で休む前提） | 状態だけを見る。要件 4.2 の確かめに使える |

**`WM_LBUTTONUP`・`WM_LBUTTONDOWN`・`WM_CANCELMODE`・`WM_CAPTURECHANGED`・ESC をテストから配るものは 0 件**（テストのファイルを検索。`crates/wintf/examples/postmessage_click_test.rs` は実窓の例で決定論テストではない）。要件 5.2 のテストは新しく組む。組み立ては `keyboard_tests.rs` の形（`EcsWorld::new()`・`DragAccumulatorResource` を入れる・`dispatch_window_message`）がそのまま使える。ただし `dispatch_window_message` は `pub(crate)`（`crates/wintf/src/ecs/mod.rs` の再輸出）なので、**areka のテストからは呼べない**。

**行数**（`wc -l`・上限 1,000）:

| ファイル | 行数 | 余裕 | 備考 |
|---|---|---|---|
| `crates/wintf/src/ecs/window_proc/mouse_click.rs` | 569 | 431 | 中にテスト 92 行（`find_ancestor_with_drag_config`）。新しいテストは兄弟の `mouse_click_tests.rs` へ置き、`#[path]` の登録 2〜3 行を足す形が既存の流儀（`keyboard.rs` の末尾と同じ） |
| `crates/wintf/src/ecs/window_proc/keyboard.rs` | 263 | 737 | |
| `crates/wintf/src/ecs/window_proc/keyboard_tests.rs` | 70 | 930 | ESC・`WM_CANCELMODE`・`WM_CAPTURECHANGED`・`WM_ACTIVATE`（`Preparing`）の場面を足す先の候補 |
| `crates/wintf/src/ecs/drag/accumulator.rs` | 320 | 680 | 中にテスト約 150 行 |
| `crates/wintf/src/ecs/drag/state/mod.rs` | 574 | 426 | |
| `crates/wintf/src/ecs/drag/state/tests.rs` | 539 | 461 | |
| `crates/wintf/src/ecs/drag/dispatch.rs` | 382 | 618 | |
| `crates/areka/src/placement/follow/drag_follow.rs` | **936** | **64** | areka 側で判断を足す案を採ると上限に迫る |
| `crates/areka/src/placement/follow_drag_end_persist_tests.rs` | 712 | 288 | 赤テストを足すと 1 本あたり 150〜250 行（既存の 3 本の規模）で上限に迫る。兄弟の新しいファイルが無難 |
| `crates/areka/src/placement/follow.rs` | 217 | 783 | 新しいテストファイルの `#[path]` 登録 2 行の置き場 |
| `crates/areka/src/placement/follow_test_support.rs` | 313 | 687 | `drag_end_event_at`・`dragging_state`・`fake_handle` などの組み立て |
| `crates/wintf/tests/drag/dispatch_test.rs` | 426 | 574 | roadmap C1 の境界（`crates/wintf/src/ecs/{window_proc,drag}/`）の外 |
| `doc/COMPAT_ARCHITECTURE.md` | 319 | − | §8「沈黙ルール対応表」は 122 行目から |

1,000 行の機械の番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表あり・どの spec も例外表に触れない）。

---

## 2. 要件ごとの対応表

印: **済**＝今のままで成り立つ／**欠**＝足りない（作る）／**制約**＝守るべき既存の決め／**不明**＝調べ残し

| 要件 | 今ある物 | 差 |
|---|---|---|
| 1.1 キャラの動かさないクリックで保存 0 | 保存は `on_char_drag_end` だけ（1.5） | **欠**: 左解放の 2 つの枝（#1・#2）が `Preparing` から終了を積む |
| 1.2 バルーンの動かさないクリックで保存 0・素材を残す | 保存と退役は `on_balloon_drag_end` だけ | **欠**: 同上（同じ知らせで呼ばれる） |
| 1.3 ダブルクリック・右クリックの前の左クリック | `DBLCLK`・右押下は準備に入らない（1.6） | **欠**: 左解放 1 回分は 1.1 と同じ。2 回目の左解放は今も積まない＝**済** |
| 1.4 クリックの前の位置から動かさない | 受け手が呼ばれなければ `enqueue_window_set_pos`・`apply_release_limit_correction` は走らない | 1.1・1.2 を直せば満たす。テストで窓の位置も見る |
| 1.5 準備中の取り消しで書かない | `WM_ACTIVATE` は済（#5） | **欠**: ESC・`WM_CANCELMODE`・`WM_CAPTURECHANGED`（#3・#4・#6） |
| 2.1・2.2 本物のドラッグで保存 1 | `Dragging` から終了を積む・受け手が保存 | **済**（変えない） |
| 2.3 速いドラッグで保存 1 | `JustStarted` から終了を積む・`Started` は上書きで消えるが受け手の縮退が保存 | **済**／**制約**: どの案でも `JustStarted` からの終了は残す |
| 2.4 開始時の記録が先に失われても保存 1 | `on_char_drag_end` の縮退・既存テスト | **済**／**制約**: 縮退は外さない |
| 2.5 閾値を越えた後の取り消しは今どおり | `Dragging`・`JustStarted` からの取り消しは終了を積む | **済** |
| 3.1・3.2 wintf が配らない | なし | **欠** |
| 3.3 閾値を越えたら 1 回配る・重なっても落とさない | 1 枠の上書きで `Ended` が残る | **済**（`Started` が消えることは別。1.7・議題 3） |
| 3.4 状態の移り変わりと捕捉の解放 | `end_dragging`／`cancel_dragging` が種と独立に行う | **済**／**制約**: 状態関数は変えない。`state/tests.rs` の休む決まりのテストで確かめられる |
| 3.5 配らないと決めたとき debug 1 行（対象・離しか取り消しか） | `WM_ACTIVATE` の `Preparing` の腕に debug 行はあるが対象の窓を載せていない | **欠**: 対象の entity と離し／取り消しを載せる 1 行。置き場は案で変わる |
| 4.1 初回の位置合わせの後のクリックで次の起動は既定 | 1.1 を直せば満たす | 実機で確かめる（5.6） |
| 4.2 右クリックメニューの出方 | `menu/trigger.rs` は状態だけを読む | **済**（`trigger_flow_tests.rs` の既存テスト） |
| 4.3 ゴーストへのマウスのイベント | `input_events/` は知らせを読まない | **済**（触らない約束の範囲） |
| 4.4 閾値・追従・初回の位置合わせ・記憶の形式 | どの案も触らない | **制約**: 速いドラッグの開始の上書きを直すと追従が変わる（議題 3） |
| 5.1 areka 側の赤テスト（保存 0・素材が残る） | 受け手を直接呼ぶ組み立てはある | **欠**: wintf の判断を通って受け手まで届く組み立てが無い（議題 2） |
| 5.2 wintf 側の赤テスト（終了 0 件） | `keyboard_tests.rs` の組み立て | **欠**: 左解放・ESC・`WM_CANCELMODE`・`WM_CAPTURECHANGED` を配るテストが無い。左解放の当たり判定が取れた枝を通す組み立ては**不明**（R3） |
| 5.3 前後とも通るテスト | 縮退のテストはある | **欠**: 閾値を越えたドラッグ・速いドラッグ・越えた後の取り消しを、知らせの経路から受け手まで通すテストは無い |
| 5.4 状態と捕捉・メニュー | `state/tests.rs`・`trigger_flow_tests.rs` | **済**（既存で確かめられる） |
| 5.5 注入した入力・兄弟ファイル・1,000 行 | 1.8 の行数 | **制約** |
| 5.6 実機 | `alpha-release-signoff` の項目 12 の手順（完了 spec の `verification/acceptance-record.md` の A1・A2 の手順） | `RUST_LOG` の target 名は**不明**（R4） |
| 5.7 §8 への記録 | `doc/COMPAT_ARCHITECTURE.md` §8 に「上書き」の行の先例が複数ある | 記録が要るかは議題 4 |
| 5.8 全体テスト | `tools/test-all.ps1` | − |

---

## 3. 実装の選択肢

どの案も areka の受け手（`drag_follow.rs`）は変えず、縮退も残す。違いは wintf のどこで「開始を配っていないドラッグの終了」を止めるかである。

### 案 A: 5 か所それぞれに先例の形で `Preparing` を除く

- **変えるファイル**: `window_proc/mouse_click.rs`（左解放の 2 つの枝）・`window_proc/keyboard.rs`（ESC・`WM_CANCELMODE`・`WM_CAPTURECHANGED`）。`WM_ACTIVATE` の `Preparing` の腕と同じく、`Preparing` のときは種を積まずに状態だけ移し、debug 行を出す。
- **良い点**: 先例（#5）と同じ形で読みやすい。累積器・配る所・既存テストに響かない（`wintf/tests/` の 3 本はそのまま緑）。
- **悪い点**: 判断が 5 か所に散る（#5 を入れて 6 か所が同じ match を手で書く）。次に終了を積む所を足す人が同じ穴を作れる＝「根本を 1 か所で」に合わない。debug 行も 5 か所。
- **赤テスト**: wintf の中のテスト（`dispatch_window_message` で左解放・ESC などを配る）は修正前の HEAD でコンパイルでき、赤になる。**areka 側のテスト（要件 5.1）は wintf の判断（private な wndproc）に届かない**ので、修正前に赤になる形を組めない（areka 側で終了の種を手で積むと、修正後も種が積まれて赤のまま）。

### 案 B: 「終了を積むか」の判断を `drag/` の関数 1 つに寄せる

- **変えるファイル**: `drag/state/mod.rs` か `drag/mod.rs` に関数を 1 つ（例: 状態の写しと累積器を受けて「開始を配ったドラッグなら終了を積む、`Preparing` なら積まずに debug 行」）。6 か所はそれを呼ぶ形に置き換える（`mouse_click.rs`・`keyboard.rs` は短くなる）。
- **良い点**: 判断が 1 つの純関数に近い形になり、状態ごとの表で決定論テストを書ける。`WM_ACTIVATE` も同じ関数へ寄せれば 6 か所が揃う。累積器の意味は変えないので `wintf/tests/` の既存テストは緑のまま。
- **悪い点**: 6 か所の書き換えで差分は 3 案で最も大きい。
- **赤テスト**: 関数が公開（`pub`）なら areka のテストから呼べる。ただし修正前の HEAD にその関数は無いので、**「振る舞いを変えない切り出し」を先に 1 コミット置き**（今の判断＝`Preparing` も積む、をそのまま関数へ移す）、その上で赤テスト → 修正の順にすれば、赤テストは修正前のコードでコンパイルでき赤になる。

### 案 C: 累積器の `set_transition` で「開始を積んでいない終了」を捨てる

- **変えるファイル**: `drag/accumulator.rs` の `DragAccumulator::set_transition` だけ。`Ended` が来たとき `current_dragging_entity` が `None`（＝`Started` を積んでいない）なら種を置かず、対象の entity と `cancelled` を debug 行に残す（要件 3.5 の「対象の窓・離しか取り消しか」はこの 2 つで言える）。`Started` を積んだ後の `Ended`（`JustStarted`・`Dragging`）は `current_dragging_entity` が `Some` なので今どおり置かれる（速いドラッグも残る）。
- **良い点**: 6 か所すべてが同じ口（`set_transition`）を通るので、変える所が文字どおり 1 か所で、今後足される終了の所も自動で同じ決まりに従う。wndproc のハンドラは変えない。判断に使う値（1.2）は既に在る。
- **悪い点**: 累積器（「wndproc → ECS へ運ぶ箱」）に方針を持たせる形になる。公開型 `DragAccumulator` の意味が変わる（「`Started` 無しの `Ended` を受けない」）。**`Started` を積まずに `Ended` だけを積む既存テストが赤になる**（`crates/wintf/tests/drag/dispatch_test.rs` の 2 本・`crates/wintf/tests/layout/boxstyle_coordinate_separation_test/drag_lifecycle.rs` の 3 本＝計 5 本）。これらは roadmap C1 の境界（`crates/wintf/src/ecs/{window_proc,drag}/`）の外にある。直し方は「前に `Started` を積む」で、確かめる中身（`Ended` の腕の働き）は変わらない。
- **赤テスト**: areka のテストは公開済みの物（`DragAccumulatorResource::set_transition`・`wintf::ecs::drag::dispatch_drag_events`・受け手の結線）だけで組めるので、**修正前の HEAD でそのままコンパイルでき、赤になる**（`Started` を積まずに `Ended` を積む → 修正前は受け手が保存 1 件 → 修正後は 0 件）。製品で `Preparing` から `Ended` を積むのは wndproc だが、止める判断そのものは `set_transition` にあるので、テストは本物の判断を通る。wndproc 側（`Preparing` から終了の種が作られること）は wintf の中のテストで別に押さえる。

### 参考: areka 側で分ける（brief の (b)）

- 受け手が「このドラッグで開始を見たか」で保存を分ける。`DraggingState` の有無では要件 2.4（縮退）とぶつかるので、別の印（例: 開始の受け手が立てる印）が要る。速いドラッグでは開始の知らせが上書きで消えるので、この印も立たず要件 2.3 を落とす。
- `drag_follow.rs` は 936 行で余裕 64 行。
- **要件 3.1・3.2（wintf が配らない）を満たさないので、単独では採れない。**

### 案の比べ

| | 案 A | 案 B | 案 C |
|---|---|---|---|
| 変える所 | 5 か所 | 関数 1 つ＋呼び手 6 か所 | 1 か所 |
| 今後の終了の所への効き | 効かない | 呼べば効く | 自動で効く |
| areka の赤テストを修正前の HEAD で組めるか | 組めない | 切り出しを先にすれば組める | そのまま組める |
| 既存テストへの響き | なし | なし | `wintf/tests/` の 5 本を直す（境界の外） |
| 累積器の意味 | 変えない | 変えない | 変える |
| 差分の大きさ | 小 | 中 | 小（テストの直しを除く） |

---

## 4. 規模とリスク

- **規模: S（1〜3 日）**。変える製品コードはどの案も wintf の 1〜2 ファイルの数十行以内。テストは wintf 側（左解放・ESC・`WM_CANCELMODE`・`WM_CAPTURECHANGED`・`WM_ACTIVATE` の `Preparing`）と areka 側（キャラ・バルーンの保存 0、閾値を越えたドラッグ・速いドラッグ・取り消しの保存 1）で 6〜9 タスクに収まる見込み（棚卸⑳の見立てと一致）。
- **リスク: 低〜中**。既存の型に沿う修正で外部依存は無い。中に寄せる理由は 2 つ: ⑴ 動かさないクリックのたびに走っていた `Arrangement.offset` の同期（1.3 の 3）が消えることの影響が静的には読み切れていない（R2）、⑵ 左解放の当たり判定が取れた枝をテストで通す組み立てが未確認（R3）。

---

## 5. 設計で決めること（議題の候補）

1. **どの層で止めるか**: 案 A（5 か所それぞれ）・案 B（判断を関数 1 つへ寄せる）・案 C（累積器の `set_transition` で止める）。「根本を 1 か所で」に最も合うのは C、層の役割（運ぶ箱に方針を持たせない）を重く見るなら B。
2. **areka 側の赤テスト（要件 5.1）の組み方**: 修正前の HEAD でコンパイルでき、wintf の本物の判断を通って受け手の保存 0 件まで届く形が要る。案 C なら公開済みの物だけで組める。案 B なら振る舞いを変えない切り出しのコミットを先に置く。案 A なら areka 側は wintf の判断に届かない（試験用の公開の口を足すか、wintf の中のテストで「知らせ 0 件」を示して areka 側は受け手が呼ばれないことに頼るか、を決める）。
3. **速いドラッグで開始の知らせが消える件を本 spec で直すか**: 累積器の 1 枠を 2 枠（開始と終了を両方運ぶ）にすれば知らせは必ず対になるが、そのとき `DraggingState` が入り、キャラ窓が押す前の位置でなくカーソルの最終位置へ写されて保存される＝速いドラッグの追従の振る舞いが変わる（要件 4.4「追従は変えない」と食い違う）。要件 2.3・3.3 は今の扱い（終了だけ・保存 1 件）で満たせる。直さないなら、残ることを design に書く。
   - **要件ディスカッションで解決（2026-10-03・開発者の決定＝直す）**: 同じ画面更新の中でも開始 → 終了の順に両方を配り、速いドラッグでも窓を行き先へ置いてから保存する。要件 2.3（行き先へ置く）・新しい 3.4（開始を先に、終了を後に、1 回ずつ）・4.4（この場合だけ追従の例外）・5.4（修正の前に赤になるテスト）へ反映した。置き場の形（2 枠・待ち行列など）と、同じ配りの中で `DraggingState` が終了の受け手から見えるかは design で決める。R1（多窓の縮退の本当の原因）がこの上書きかどうかも design で確かめる。
4. **`doc/COMPAT_ARCHITECTURE.md` §8 への記録が要るか**: 完了 spec `event-drag-system` の要件 4.1 は「ドラッグ中にマウスボタンが解放された時」に終了を配ると定めており、本修正はむしろそれに揃える。一方、同 spec の要件 5.2 は「ドラッグがキャンセルされた時」に取り消しの印つきの終了を配ると定める。閾値に届く前の取り消しを「ドラッグのキャンセル」と読むなら上書き、読まないなら上書きではない。position-persist 要件 1.9 は守る側。どちらに読むかで §8 の行の要否が決まる（要件 5.7 は「食い違うとき」だけ記録を求める）。
   - **要件ディスカッションで解決（2026-10-03）**: 同 spec の用語集は「ドラッグ準備状態」＝閾値未達、「ドラッグ中状態」＝閾値を超えて開始された状態と分け、要件 5.1 も「ドラッグ中に Esc」と書く。閾値に届く前の取り消しは「ドラッグのキャンセル」に当たらない＝上書きではない。requirements.md の Introduction に根拠を書いた。§8 の行は要らない（要件 5.7 は条件つきのまま残す）。
5. **境界の外の既存テストをどう扱うか**（案 C を採るときだけ）: `crates/wintf/tests/drag/dispatch_test.rs` と `crates/wintf/tests/layout/boxstyle_coordinate_separation_test/drag_lifecycle.rs` の「`Started` 無しの `Ended`」を直すことを本 spec の境界に入れるか。C1 の他の 6 本は `crates/wintf/` に触らないので、ファイルの重なりは無い。
   - **要件ディスカッションでの補足（2026-10-03）**: 並走する spec と重ならないので、根本の 1 か所の直しに要るなら境界を `crates/wintf/tests/` へ広げてよい（開発方針「根本が境界の外でも並走が無ければ境界を広げて直す」）。広げるかどうか自体は案の選択とともに design で決める。
6. **要件 3.5 の debug 行の置き場と形**: 案 A は 5 か所、案 B は関数の中、案 C は `set_transition` の中。載せる値は対象の entity と離し／取り消し（`cancelled`）。target は wintf の既定（モジュールのパス）か固定の名前か。実機の `RUST_LOG` で開ける名前と合わせる。

---

## 6. 調べ残し（Research Needed）

- **R1: 多窓で `DraggingState` が先に落ちる場面の本当の原因**。速いドラッグの 1 枠の上書き（1.7）は「窓が動かない」ので縮退のテストの想定（連続の追従で窓が動いた後）と合わない。`DraggingState` を外す製品の所は 2 つ（`dispatch.rs` の `Ended` の腕・`systems.rs` の `cleanup_drag_state`）だけだった。要件 2.4 は縮退を残すので本 spec の修正には響かないが、design で仮説として残すか、記録から確かめるかを決める。
- **R2: `Arrangement.offset` の同期が動かさないクリックで走らなくなる影響**。通常は `Changed<WindowPos>` を拾う同期（`window_pos_systems.rs` の `Without<WindowDragging>` の system）が受け持つので、クリックの同期に頼る経路は無いと見込むが、スクリプトで動かした窓の当たり判定がクリックで直っていた、という隠れた依存が無いかを design で確かめる。
- **R3: 左解放の当たり判定が取れた枝（#1）をテストで通す組み立て**。`hit_test_in_window`（`crates/wintf/src/ecs/layout/hit_test/mod.rs`）は窓に `WindowPos.position` が無ければ `None` を返すので、素の `EcsWorld` では予備の枝（#2）しか通らない。#1 を通すには配置の結果（`GlobalArrangement` など）を持つ entity が要る。案 B・C では両方の枝が同じ判断を通るので重さは下がるが、要件 5.2 の「閾値に届かないまま離したとき 0 件」をどちらの枝で示すかは決める。
- **R4: 実機の `RUST_LOG`**。保存の行は target `areka::persist::save`（info）、受け手の debug 行は既定の target（`areka::placement::follow::drag_follow`）、wintf の debug 行は既定の target（`wintf::ecs::drag::…`・`wintf::ecs::window_proc::…`）と見込む。要件 3.5 の行の target と合わせて design で文字列を決める。実機の根と一時フォルダはワークツリーの `target\` の下に置く。
- **気づいたこと（範囲の外）**: ESC（#3）と `WM_CANCELMODE`（#4）は、受け取った窓とドラッグ中の窓が同じかを確かめずに取り消す（左解放の枝は HWND か持ち主の窓で確かめている）。今回の症状には関わらないので範囲の外として記すだけにする。

---

## 7. 設計での決定（2026-10-03 `/kiro-spec-design`・調べ方は「拡張」の軽い手順）

> 対象はこの日のワークツリー（`89a8d836`）の実物。§1〜§6 の事実は読み直して全部そのままだった。ここでは §5 の議題と §6 の調べ残しに答えを出し、根拠を残す。設計の本文は `design.md`。

### 7.1 追加で調べた事実

- **`update_dragging` は `JustEnded` では何もしない**（`state/mod.rs` の `update_dragging` の `_ => {}` の腕）。速いドラッグで開始と終了を同じ配りの中で続けて配るとき、開始の腕が最後に呼ぶ `update_dragging` は、状態が既に `JustEnded`（離したときに `end_dragging` が移した）なので空振りする。新しい分岐は要らない。
- **開始の腕は `DraggingState` を Command でなく直接入れる**（`dispatch.rs` の `Started` の腕の `entity_mut.insert(DraggingState{..})`）。よって同じ `dispatch_drag_events` の呼び出しの中で続けて終了の腕が走れば、終了の受け手（`on_char_drag_end`）は `DraggingState` を読める。
- **`FlushResult.transition` を読む所はワークスペースで 3 か所だけ**（`accumulator.rs` の中のテスト・`dispatch.rs`・`keyboard_tests.rs`）。`crates/wintf/tests/` の 2 ファイルは `set_transition` → `dispatch_drag_events` の形で、`FlushResult` を読まない。置き場を待ち行列にしても外へは響かない。
- **`Started` を積まずに `Ended` を積むテストは §1.8 の 5 本で全部**（`DragTransition::Ended` を含むファイルをワークスペースで列挙: 製品 4・テスト 3 ファイル。`crates/wintf/tests/drag/` の他の 2 ファイルは `Ended` を使わない）。
- **wndproc の左押下の予備の枝（当たり判定が取れないとき）は `start_preparing` を呼ばない**（`mouse_click.rs` の `handle_button_message` の `is_down` の予備の枝は `record_button_down` だけ）。テストで `Preparing` に入れるには `start_preparing` を直接呼ぶ（`keyboard_tests.rs` と同じ形）。
- **左解放の `should_end`（`Preparing`／`JustStarted`）は `find_owner_window(world, entity) == Some(window_entity)` を見る**（`window/command.rs` の `find_owner_window` は自分か祖先の `Window` を返す）。テストの対象 entity は `Window` を持たせ、同じ entity を窓として `dispatch_window_message` へ渡せばよい。
- **areka の受け手の `enqueue_window_set_pos` は `WindowHandle` が無いと `false` を返す**（`follow/window_move.rs` の `enqueue_window_set_pos` の「実在するが `WindowHandle` 未付与＝窓生成前」の分岐）。一方、配る所の開始の腕は `WindowHandle` が有ると `client_to_window_coords`（`AdjustWindowRectExForDpi`）で枠込みの座標へ直し、偽の HWND では失敗して `initial_window_pos` が `(0,0)` のまま残る。areka の決定論テストでは「初期の窓位置 = (0,0)」として `DraggingState.initial_inset` が入る＝行き先の期待値は `project_anchor(anchor, cursor − drag_start, size)` で求める（テストの組み立ての都合であり製品の振る舞いではない）。
- **実機の記録の数え直し**（`C:\home\maz\lap-records\alpha-signoff-20261001\run-A*.log`・読むだけ）:

  | 走行 | `[DragEndEvent] Dispatching` | `[DragStartEvent] Dispatching` | `Direct Arrangement.offset sync` | `DragEnd 保存` | `写像スキップ` |
  |---|---|---|---|---|---|
  | A1r | 1 | 0 | 0 | 1 | 1 |
  | A2r | 2 | 0 | 0 | 2 | 2 |
  | A3r（本物のドラッグ） | 1 | 1 | 0 | 1 | 0 |
  | A2（前の zip・本物のドラッグ） | 1 | 1 | 0 | 1 | 0 |

  「開始 0・終了 1・写像スキップ 1」が 3 件とも一致＝縮退を踏んだ 3 件はすべて「開始の無い終了」である（R1）。`Direct Arrangement.offset sync` は本物のドラッグでも 0 件＝終了の腕の同期は観測した走行のどれでも値を変えていない（R2）。

### 7.2 議題への答え

| 議題 | 決定 | 根拠（短く） |
|---|---|---|
| 1 どの層で止めるか | **案 C**: `DragAccumulator::set_transition` が「`Started` を積んでいない `Ended`」を置かない | 6 か所すべてが通る唯一の口・判断に使う値 `current_dragging_entity` が既に在る・areka の赤テストを修正前の HEAD でコンパイルできる。累積器は「wndproc → ECS へ運ぶ箱」だが、運ぶ物の並びの約束（終了は開始の後）を箱が守るのは箱の責務の内と読む |
| 2 areka 側の赤テストの組み方 | 公開済みの `DragAccumulatorResource::set_transition`・`dispatch_drag_events`・`OnDragEnd(on_char_drag_end)` の結線だけで組む。保存の数は `FakePersistIo` の共有ストアを `load_scope` で読んで数える（既存の `on_balloon_drag_end_persists_balloon_offset_for_scope` と同じ） | 修正前にそのままコンパイルでき、`set_transition` の本物の判断を通る |
| 3 速いドラッグ | **置き場を待ち行列（`Vec<DragTransition>`）にする**。`FlushResult.transition: Option<_>` → `transitions: Vec<_>`。配る所は積んだ順に全部を処理する。同じ配りの中で `DraggingState` は終了の受け手から見える（7.1） | 2 枠（開始用・終了用）だと「終了 → 次の開始」が同じ画面更新に重なったときの順序を別に持つ必要が出る。並びをそのまま運ぶ待ち行列が最も素直で、長さは実用上 3 を超えない |
| 4 §8 への記録 | 要らない（要件ディスカッションで確定済み） | − |
| 5 境界の外の 5 本 | **境界を `crates/wintf/tests/drag/dispatch_test.rs`・`crates/wintf/tests/layout/boxstyle_coordinate_separation_test/drag_lifecycle.rs` へ広げる**。直し方は「`Started` を積んで 1 度配ってから `Ended` を積む」（同ファイルの `test_window_dragging_full_lifecycle` と同じ形）。確かめる中身（終了の腕の働き）は変えない | C1 の他の 6 本は `crates/wintf/` に触らない（roadmap C1 の接触ファイル）。開発方針「根本が境界の外でも並走が無ければ境界を広げて直す」 |
| 6 debug 行 | `DragAccumulator::set_transition` の中の 1 行。欄は `entity`（対象）と `cancelled`（離し＝false／取り消し＝true）。target は既定（`wintf::ecs::drag::accumulator`） | 止める判断がここにしか無いので行もここ 1 つ |

### 7.3 調べ残しへの答え

- **R1（多窓で `DraggingState` が先に落ちる本当の原因）**: 実機の記録では縮退を踏んだ 3 件すべてが「開始の無い終了」（7.1 の表）。`DraggingState` を外す製品の所は 2 つ（`dispatch.rs` の `Ended` の腕・`systems.rs` の `cleanup_drag_state`）で、どちらも終了の知らせの後に走る。静的にも実機の記録にも「開始を配った後に、終了より先に `DraggingState` が落ちる」経路は見つからなかった。結論: 観測された縮退は「動かさないクリック」か「速いドラッグの上書き」で、本 spec の 2 つの修正でどちらも入口で消える。縮退そのものは要件 2.4 で残す。実機の確かめ（要件 5.7）で修正後に `写像スキップ` が 1 件でも出たら、それは別の原因なので新しい spec として起票する。
- **R2（`Arrangement.offset` の同期が動かさないクリックで走らなくなる影響）**: 観測した 4 走行で `Direct Arrangement.offset sync` は 0 件（値を変えたことが無い）。平時の同期は `window_pos_systems.rs` の `sync_window_arrangement_from_window_pos`（`Changed<WindowPos>`・`Without<WindowDragging>`）が受け持ち、スクリプトや追従で窓を動かす経路はすべて `WindowPos` を書くのでそこで揃う。クリックの同期に頼る経路は無い。
- **R3（当たり判定が取れた枝をテストで通す組み立て）**: 案 C では止める判断が `set_transition` にあり、左解放の 2 つの枝は `should_end` の求め方が違うだけで、どちらも同じ `set_transition(Ended{cancelled:false})` を呼ぶ。wintf 側のテストは素の `EcsWorld` で通る予備の枝（当たり判定が取れない枝）で「終了の種 0 件」を示し、当たり判定が取れた枝は「同じ関数を同じ引数で呼ぶ」ことをコードの読みで押さえる（設計の責務表に明記）。
- **R4（実機の `RUST_LOG`）**: 保存の行は target `areka::persist::save`（info・`drag_follow.rs` の 2 か所）。受け手の debug 行は既定 target `areka::placement::follow::drag_follow`。要件 3.6 の行は既定 target `wintf::ecs::drag::accumulator`。配る所の info 行は `wintf::ecs::drag::dispatch`。よって `RUST_LOG=info,areka=debug,wintf::ecs::drag=debug`（`areka=debug` は α の実機の一周と同じ）。

### 7.4 設計のまとめ方（単純化）

- 新しい型・新しいモジュール・新しい公開関数は作らない。変える製品のファイルは `accumulator.rs`（待ち行列＋入口の判断＋debug 行）と `dispatch.rs`（`for` で全部を処理）と、`accumulator.rs` の doc の 2 行（`current_dragging_entity` の意味）だけ。wndproc のハンドラ（`mouse_click.rs`・`keyboard.rs`）と areka の受け手は変えない。
- 「2 枠」「終了だけを捨てる印」「receiver 側の印」はどれも待ち行列 1 本より部品が増えるので採らない。

### 7.5 気づいたこと（範囲の外・起票は求められたときに）

- 配る所の開始の腕は、`WindowHandle` が有って `client_to_window_coords` が失敗したとき `initial_window_pos` を `(0,0)` のまま使う（`dispatch.rs` の `Started` の腕）。製品では失敗しないが、失敗したときの縮退として `WindowPos.position` へ倒す方が筋がよい。本 spec では触らない。
- §6 末尾の ESC と `WM_CANCELMODE` が窓の一致を確かめない件は、そのまま範囲の外。
