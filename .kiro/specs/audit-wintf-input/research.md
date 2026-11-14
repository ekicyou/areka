# ギャップ分析: audit-wintf-input（wintf のドラッグ・ポインタ・クリック透過の見直し）

- 測った時点: `main` `30a343eb`（枝 `claude/kiro-start-audit-wintf-input-9a36a8`・`419631c2`）・2026-11-14。範囲の 24 ファイルは `30a343eb` から変わっていない（要件の段のコミットは spec の文書だけ）。
- 物差し: `/ponytail:ponytail-audit`（ponytail 5.3.3 の `SKILL.md` を読んで、その作法で行った。兄弟の完了した見直しと同じ版）。作法のうち 2 つを外した（要件 1.2・1.3）。
  - 「最大 20 件」を外した。指摘は全件を下に並べ、省いた件は無い。
  - 「読まなかった所（Not checked）」を残さない。範囲の 24 ファイルを全部、全行読んだ（2 節）。
- 番号は audit の番号（組をまたいで通し番号）。組は audit の 3 つ（必ず直す・直したほうがよい・できれば）のまま。
- 範囲の外のファイルは、指摘を判断するのに要る所だけを読んだ（見直してはいない）。読んだ所は各件の欄に書いた。
- この文書はコードを変えていない。テストもビルドも走らせていない（「今のテストは緑」は読みからの前提）。決めごとは要件の討議に回す。

## 1. 要約

- **何をする所か**: wintf の窓の手続き（`ecs/window_proc/`・範囲の外）が受けたマウスの知らせを、⑴ ポインタの溜め（`pointer/buffers.rs`）に貯めて tick の頭で `PointerState` へ移し、Tunnel→Bubble の順で受け手（`OnPointer*`）へ配る（`pointer/dispatch/`）。⑵ 左の押下からドラッグの状態（`drag/state/`・`thread_local!`）を進め、累積器（`drag/accumulator.rs`）を通して開始・移動・終わりの知らせを配る（`drag/dispatch.rs`）。World を借りられない間の終わりは `drag/reentry.rs` が扱う。⑶ カーソルを別スレッドで見張り（`clickthrough/monitor.rs`）、窓の α の当たりで `WS_EX_TRANSPARENT` を付け外しする（`clickthrough/controller.rs`）。⑷ `WM_NCHITTEST` の答えを座標ごとに 1 tick だけ控える（`pointer/nchittest_cache.rs`）。下流は本体 `crates/areka`（`input_events/`・`runtime/frame/dpi.rs`）・`crates/areka-placement`（ドラッグの追従・クリック透過の登録・バルーンの止め）・`crates/areka-emo-text`・`crates/pilot` と、wintf の窓の手続き。
- **想定した負荷**: 1 人の利用者の 1 台の机で、常駐のアプリを何時間〜何日も動かす。マウスの知らせは毎秒数百まで、tick はドラッグ中・動きのある間は画面の更新ごと（60 回/秒ほど）、止まっている間は起こされたときだけ。入力は OS から来るマウスの知らせだけで、信用できない外からの入力は無い。窓の数は 2〜数枚（シェルとバルーン）、ただしゴーストやシェルを切り替えるたびに窓とその中の entity を作り直す。
- **指摘の数**: 必ず直す 1 件・直したほうがよい 12 件・できれば 10 件＝**23 件**（テストの指摘は種類ごとに 1 件へまとめ、場所は全部を挙げた）。
- **見つかった型**: ⑴ 同じ仕事が 2 か所にあり、まれな並びで食い違う（ドラッグの状態の後始末が 2 つ＝1）。⑵ 溜めが減らない・失敗を黙って捨てる（2・3）。⑶ 説明と違う振る舞い（「冪等」の登録・「ローカル座標」・古い意味の分岐＝4・11・13）。⑷ 古い理由のまま残った dead_code の許しと、誰も使わないコード・推測で作った機能（5・9・10）。⑸ 利用者のカーソルを動かすテスト・落ちえないテスト・判定にテストが無い所（6・7・8）。⑹ 古くなった説明と小さな写し（12・14〜23）。
- **範囲の外への波及**: 1 は wintf の `tests/drag/dispatch_test.rs`・`ecs/world/mod.rs`・`ecs/mod.rs`（どれも完了済みの見直しの持ち物）に触る。9 の一部・11 は同じウェーブで走っている `audit-wintf-window`（`ecs/window_proc/`）と `audit-placement-*`（`crates/areka-placement/` のテスト）に触る直し方がある＝申し合わせ（要件 5.2）。C20 の約束 ⑶（`MonitorRecord`・`Monitor`・`DESPAWNED_SKIP_TAG`・`areka_placement::diag`）に当たる直しは 0 件（範囲の中にこれらの名は 0・grep）。依存を足す直しは 0 件。
- **大きさ**: 直しは見込みで 14〜20 タスク（8 節）。20 に収まる見込みなので、切らない案を推す。

## 2. 読んだ範囲（要件 1.3）

範囲の全 24 ファイル（7,893 行）を、進行役が全行を読んだ。下請けは使っていない。

- `drag/`（11）: `mod.rs`（194）・`accumulator.rs`（238）・`accumulator_tests.rs`（260）・`capture_guard.rs`（109）・`context.rs`（178）・`dispatch.rs`（382）・`reentry.rs`（111）・`reentry_tests.rs`（250）・`systems.rs`（101）・`state/mod.rs`（650）・`state/tests.rs`（848）。
- `pointer/`（8）: `mod.rs`（33）・`buffers.rs`（682）・`nchittest_cache.rs`（302）・`systems.rs`（176）・`dispatch/mod.rs`（280）・`dispatch/tests.rs`（560）・`types/mod.rs`（404）・`types/tests.rs`（326）。
- `clickthrough/`（5）: `mod.rs`（29）・`controller.rs`（479）・`controller_tests.rs`（702）・`monitor.rs`（334）・`registry.rs`（265）。
- 判断のために読んだ範囲の外（見直していない）: `ecs/world/mod.rs:120-216,370-400`（登録と順）・`ecs/window_proc/mouse_move.rs:150-330,464-479`・`mouse_click.rs:100-210,395-410`・`ecs/window/command.rs:850-867`（`find_owner_window`）・`ecs/window/window_handle.rs:145-163`・`win_style.rs:15-77`・`runtime/mod.rs:283-345,600-625,695-712`・`crates/areka-placement/src/follow/drag_follow.rs:60-135,268-300`・`balloon_lock.rs:85-135`・`spawn.rs:730-795`・`crates/areka/src/input_events/mod.rs:218-253,466-500`・`crates/log-capture-kit/tests/file_length_guard_test.rs`・完了した `event-mouse-basic`・`event-drag-system` の `requirements.md`・`roadmap-history.md:255`。
- 消せると書いた名は、どれもツリー全体（テスト・検体・設定・`.md`・`.ps1`・`.py`）で grep した。結果は各件の欄。
- 範囲の中の先送りの印: `#[allow(dead_code)]` が 2 か所（`drag/capture_guard.rs:52`・`clickthrough/mod.rs:14` のモジュール全体の `#![allow(dead_code, unused_imports)]`）。`shortcut:`・`ponytail:` の注釈は 0 件。どちらの許しも、残す 3 種類の 2 種類目（実在の理由の説明を付けた語彙の口）には当たらない見込み（5・9 の欄）。

## 3. 指摘の一覧

各件の欄: **場所**／**何か**／**困る場面**／**いちばん小さい直し**／**範囲の外への波及**／**残す 3 種類に当たるか**（要件 3.1 の 1＝裁定に反する・2＝わざと置いた先送りの印・3＝SSP 互換・ukadoc 正典）。

### 必ず直す

#### 1. 1 回の配りに「前のドラッグの終わり」と「次のドラッグの始まり」が重なると、次のドラッグを取り違える（`drag/systems.rs:12-27`・`drag/dispatch.rs:82-213,215-335`・`drag/state/mod.rs:362-417`・`drag/accumulator.rs:68-88`）

- **何か**: 累積器は 1 tick 分の知らせを積んだ順に返し、`dispatch_drag_events` がそれを順に配る。ここに 3 つの食い違いがある。
  - ⑴ 終わりの腕（`dispatch.rs:251-258`）は自分で `DraggingState` を外す。同じことを `cleanup_drag_state`（`systems.rs:12-27`・`world/mod.rs:202-206` で配りの後に登録）も `DragEndEvent` を読んで行う。積んだ順が「A の終わり → A の始まり」（離してすぐ同じシェルを掴み直す）だと、配りが入れ直した新しい `DraggingState` を、後から走る `cleanup_drag_state` が外す。
  - ⑵ 始まりの腕は、知らせの entity を見ずに「今の」ドラッグの状態へ `update_dragging` を呼ぶ（`dispatch.rs:210-212`）。並びが「A の始まり → A の終わり → B の始まり」だと、A の腕が B の状態を A の控え（A の窓の HWND・A の始めの位置）で `Dragging` へ進め、B の腕の呼び出しは位置を書き換えるだけになる。
  - ⑶ 終わりを積んでも、それまで貯めた移動量（`accumulated_delta`）は残る（`accumulator.rs:73-85`）。「A の終わり → B の始まり」だと、A の移動量で B に余分な `DragEvent` が 1 件配られる。
- **困る場面**: ⑴ 画面の更新 1 回（約 16 ms）の間に、離して、同じシェルを押し直して 5 px 動かす（素早い掴み直し）。新しいドラッグには `DraggingState` が無いまま進む。吸着したシェル（areka が書き手・`move_window = false`）は `areka-placement` の `policy_mapped_position`（`drag_follow.rs:283-289`）が `DraggingState` を読めずに何もしないので、**そのドラッグの間シェルが指に付いてこない**（記録は `debug!` 1 行だけ・説明は「実 flow では起きない」と書く）。止めた指で次の画面更新を頼む `rearm_tick_while_dragging` も止まる。⑵ 1 回の更新の間にシェルのドラッグを終えてバルーンを掴み始めると、バルーンのドラッグがシェルの窓を動かす（`mouse_move.rs:302-318` が控えの HWND へ `SetWindowPos`）。⑶ B が動く前に `OnDrag` が 1 回来る。どれも速い操作か、tick が遅れたときだけに起きる。
- **いちばん小さい直し**: ⑴ `cleanup_drag_state` を消す（終わりの腕が既に外している・登録 `world/mod.rs:202-206`・再公開 `ecs/mod.rs:28`・`drag/mod.rs:29`・テスト `crates/wintf/tests/drag/dispatch_test.rs:492-545` の 2 本も消す）。⑵ 始まりの腕は、ドラッグの状態の entity が知らせの entity と同じときだけ `update_dragging` を呼ぶ（`update_dragging` に entity を渡して中で比べる形でもよい）。⑶ 累積器が終わりを積むとき `accumulated_delta` を 0 に戻す（終わりの後の移動量はどのドラッグのものでもない・最後の区間の `DragEvent` が配られないのは今と同じ＝下流は `OnDragEnd` で埋めている）。テスト＝「A の始まりを配った後、1 回の配りに A の終わりと A の始まりを積み、Input のスケジュールを回した後も A に `DraggingState` が在る」（直す前は落ちる）と、「A の始まり・A の終わり・B の始まりを 1 回で配ると、B の状態の HWND が B の窓のもの」「終わりの後に B へ `DragEvent` が来ない」。
- **範囲の外への波及**: `cleanup_drag_state` は公開の関数で、使い手は上の登録・再公開・テストだけ（ツリー全体の grep・`.md` は完了した spec の記録だけ）。`crates/wintf/tests/drag/`（完了した `audit-wintf-runtime` の持ち物）と `ecs/world/mod.rs`・`ecs/mod.rs`（完了した見直しの持ち物）に触る。走っている spec の持ち物には触らない。
- **残す 3 種類**: 当たらない（振る舞いはまれな並びで正しくなるだけ・SSP に無い仕組み）。

### 直したほうがよい

#### 2. ポインタの溜め 3 つから entity の鍵が消えず、毎 tick 全部を回す（`pointer/buffers.rs:20-35,142-292`）

- **何か**: `POINTER_BUFFERS`・`BUTTON_BUFFERS`・`MODIFIER_STATE` は entity を鍵にした表で、中身を空にするだけで鍵を消さない（`buffer.clear()`・`buf.reset()`・修飾キーは消さない）。`WHEEL_BUFFERS` だけは毎回 `drain` する。`transfer_buffers_to_world` は 3 つの表の全部の鍵について毎 tick `world.get_mut::<PointerState>` を引く。
- **困る場面**: ゴーストやシェルを切り替えるたびに窓と中の entity を作り直すので、触ったことのある entity の鍵が何日でも残り、tick ごとの手間が使った履歴の長さに比例して増える（1 件は小さく、困るのは長く動かしたとき）。消えた entity の修飾キーも残り続ける。
- **いちばん小さい直し**: 3 つとも `WHEEL_BUFFERS` と同じく `drain` で取り出してから移す（1 tick 分だけ渡すのは今と同じ・`PointerBuffer` は今も移した後に空にしている）。テスト＝移した後に 3 つの表が空（直す前は鍵が残って落ちる）。
- **範囲の外への波及**: なし（`pub(crate)` の表・呼び手は窓の手続きの `push_*`・`record_*`・`set_modifier_state` で、形は変わらない）。
- **残す 3 種類**: 当たらない。

#### 3. 窓の位置が取れないと、黙って (0,0) を始めの位置にして窓を動かす（`drag/dispatch.rs:89-133`・`drag/state/mod.rs:379-393`）

- **何か**: 始まりの腕は、`WindowPos.position` が無いときや `client_to_window_coords` が `Err` を返したとき、記録を出さずに始めの位置を (0,0) のままにする。それが `WindowDragContext` と `DraggingState.initial_inset` へ入り、窓の手続きは `(0,0) + (カーソル − 押した点)` へ窓を動かす。`update_dragging` も控えが無い・読めないとき、記録なしで空の HWND と (0,0) にする。
- **困る場面**: 位置がまだ決まっていない窓（作った直後で `WindowPos.position` が無い）を掴むと、窓が画面の左上へ飛ぶ。何が起きたかの記録は 1 行も残らない（プロジェクトの決まり「記録の無い失敗の道を作らない」に反する）。
- **いちばん小さい直し**: 始めの位置が取れないときは `warn!`（窓の entity と理由）を出し、`move_window = false` にして窓を動かさない（受け手へのドラッグの知らせは配る）。`update_dragging` の「控えが無い」も `warn!` を 1 行。テスト＝`WindowPos.position` の無い窓で始まりを配ると、`warn!` が 1 行出て控えの `move_window` が偽（`log-capture-kit`）。
- **範囲の外への波及**: なし（`WindowDragContext` の形は変わらない・`areka-placement` の `balloon_drag_held` は控えの `move_window` を読むが、位置の取れない窓を掴めないのは今も同じ）。
- **残す 3 種類**: 当たらない。

#### 4. 同じ窓を同じ HWND で登録し直すと、透過が外れなくなる（説明の「冪等」と違う）（`clickthrough/registry.rs:62-81`）

- **何か**: `register` は同じ entity の再登録で、HWND が同じでも `last_applied` を `Opaque` に、`layered_applied` を偽に戻す。窓が今 `WS_EX_TRANSPARENT` 付きでも、記録だけが「不透過」になる。`ClickThroughRegistryHandle::register`（`controller.rs:395-400`）と `areka-placement` の `spawn.rs:756-764` の説明は「同一 Entity 再登録を dedupe するため冪等」と書く。
- **困る場面**: カーソルがキャラの外にある間（窓は透過）に同じ窓を登録し直し、カーソルをキャラの上へ動かす。望みは `Opaque` で記録も `Opaque` なので差分の守りが何もしない＝窓は透過のままで、**キャラをクリックできない**（カーソルが一度外へ出て戻るまで）。今の呼び手（`register_ghost_windows_click_through`）は `Added<WindowHandle>` で 1 回だけ呼ぶので今は表に出ていないが、公開の口の説明が約束していることと違う。
- **いちばん小さい直し**: HWND が変わったときだけ `last_applied`・`layered_applied` を戻す（同じ HWND なら何もしない）。テスト＝透過を適用した後に同じ HWND で登録し直し、当たりの上で評価すると `WS_EX_TRANSPARENT` が外れる（直す前は落ちる）。
- **範囲の外への波及**: なし（形は変わらない・HWND が変わる再登録の振る舞いは今と同じ）。
- **残す 3 種類**: 当たらない。

#### 5. クリック透過のモジュール全体の dead_code の許しが、古い理由のまま残って使われないコードを隠す（`clickthrough/mod.rs:10-14`・`controller.rs:347-365`・`monitor.rs:111-114`・`registry.rs:102-108`・`mod.rs:24-29`）

- **何か**: `#![allow(dead_code, unused_imports)]` の理由は「`start` の結線は task 3.2 で行う・まだ全消費されない」。結線は済んでいる（`runtime/mod.rs:315-345` の `wire_click_through`・`:389` で使う）。許しの陰で、製品から使われないものが残る: `ClickThroughHandle` の `register`・`remove`・`registry`・`wake_event`・`wake`・`latest_cursor`（製品は `_click_through` として持って捨てるだけ・使い手はテストだけ）、`CursorMonitorBridge::event`（使い手 0）、`ClickThroughRegistry::last_applied`（テストだけ）、`mod.rs` の再公開 `evaluate_targets`・`prune_dead_targets`・`resolve_transition`・`ClickThroughTarget`（中のファイルは直に使う）。
- **困る場面**: 新しく足した関数が使われなくても警告が出ず、使われないコードが黙って増える（今もこの 10 個ほど）。
- **いちばん小さい直し**: 許しと NOTE を消し、コンパイラが指すものを消すか `#[cfg(test)]` にする（テストだけが使う `register`・`remove` は、テストが `ClickThroughRegistry` を直に触る形へ寄せて消すのが小さい）。
- **範囲の外への波及**: なし（`pub(crate)` のものだけ。公開の `ClickThroughRegistryHandle` は使い手が多い＝残す）。
- **残す 3 種類**: 2 種類目には当たらない見込み（理由の説明が過去の話で、今は実在しない）。

#### 6. 普通のテストが利用者のマウスカーソルを動かし、しかも落ちえない（`clickthrough/monitor.rs:285-333,264-283,140-182`）

- **何か**: `notify_on_move_and_ui_reads_latest` は `SetCursorPos(137, 149)` で本物のカーソルを動かす。動かせなかったとき・知らせが来なかったときは `trace!` を出して通るので、確かめていることは「(0,0) でない」だけ。`latest_cursor_reads_stored_position` は 60 ms 待った後、`latest_cursor()` が `unpack(latest_pos)` と同じかを見る（同じ式を 2 度書いた比べ）。差分の守り（同じ位置なら知らせない・`:165`）を確かめるテストは無い。
- **困る場面**: `cargo test` のたびに、開発者のカーソルが画面の左上へ飛ぶ。同じ机で別のセッションが `SendInput` の実機テスト（排他テストの机の外で走る全体テストと重なることがある）を回していると、その押下が別の所へ落ちて赤になる。差分の守りを消して 12 ms ごとに知らせる形に壊しても、どのテストも赤にならない。
- **いちばん小さい直し**: ループの 1 回分を「カーソルを読む関数を受け取って、変わったときだけ書いて知らせる」小さな関数に切り、偽のカーソルを渡して「同じ位置が 2 回なら知らせは 1 回」「書いてから知らせる」を確かめる（決定論）。`SetCursorPos` のテストと同語反復のテストは消す。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 7. 落ちえない・名前と違うことを確かめるテスト（偽の安心）

- **場所と何か**（どれも開いて確かめた）:
  - `clickthrough/controller_tests.rs:632-644`（`shutdown_condition_world_weak_upgrade_none`）: 標準の `Rc`/`Weak` の振る舞いだけを確かめる。ループの終わりは `:653-702` が本物で確かめている。
  - `drag/accumulator_tests.rs:121-135`・`drag/context.rs:163-177`: `clone` した `Arc<Mutex>` が同じ中身を指すこと（標準の事実）。
  - `drag/capture_guard.rs:83-108`: テストだけが使う `is_released`（指摘 9）と `Debug` の文字列を確かめる。`ReleaseCapture` を呼ぶか呼ばないかは `state/tests.rs:780-812` が記録で本物を確かめている。
  - `pointer/dispatch/tests.rs:3-38,164-171`: `Phase` の 3 つの関数と型の大きさ（コンパイラの事実）。`:164-171` は `:11-17` と同じ。
  - `pointer/types/tests.rs:112-164`: 単位の印の等しさ・`Default`・`new`・列挙の値が違うこと・`Hash`（どれもコンパイラか `derive` の事実）。
  - `drag/state/tests.rs:380-442`: 製品の使い手の無い `check_threshold`（指摘 9）の 5 本。
- **いちばん小さい直し**: 上を消す（`check_threshold` の 5 本は関数と一緒に）。残る本物のテストで、それぞれの仕組みは覆われている（各行の後半）。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 8. `WM_NCHITTEST` の答えの判定（押している間は HTCLIENT）にテストが無い（`pointer/nchittest_cache.rs:138-149`）

- **何か**: 「当たりがある・または左を押している間は HTCLIENT、どちらでもなければ HTTRANSPARENT」の分岐を確かめるテストが無い。今の 6 本（`:184-301`）は控えの表の出し入れだけを見る。
- **困る場面**: `||` を `&&` に書き違えても、押したまま α の外へ出た所の答えを逆にしても、全体テストは緑のまま。
- **いちばん小さい直し**: 分岐を `fn answer(hit: bool, held: bool) -> LRESULT` に切り、4 つの組み合わせを 1 本で確かめる（指摘 20 の定数の直しと同じタスク）。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 9. 使われないコード（ツリー全体を grep して確かめた）

- ⑴ `check_threshold`（`drag/state/mod.rs:611-647`）: 製品の呼び手 0（説明もそう書く）。同じ計算は窓の手続き `mouse_move.rs:228-233` に手で書いてある。説明の「P62 に記録」の P62 はツリーのどこにも無い。
- ⑵ `DOUBLE_CLICK_BUFFERS`（`pointer/buffers.rs:30-31`）: 書く所も読む所も 0（テストの片付け `:311` だけ）。ダブルクリックは窓の手続きが `PointerState` へ直に書く。
- ⑶ `hit_test_placeholder`・`hit_test_with_local_coords`（`pointer/types/mod.rs:363-397`・テスト `types/tests.rs:300-326`）: 「Phase 1 の仮」で、呼び手は自分のテストだけ（完了した spec の検証の記録にテスト名が載るだけ）。
- ⑷ `CaptureGuard::is_released`（`drag/capture_guard.rs:51-55`）: `#[allow(dead_code)]` 付きで、使い手はこのファイルのテストだけ。説明は何をするかだけで、残す理由を書いていない。
- ⑸ `DragConfig.right_button`・`middle_button`（`drag/mod.rs:50-53`）: 読む所が 0（押下で準備に入るのは左だけ・`mouse_click.rs:145-160`）。`true` にしても何も起きない。範囲の外では `window_proc/mouse_dblclick_wheel_tests.rs:213` が `right_button: true` を書く。
- ⑹ 3 つのドラッグの知らせの `is_primary`（`drag/dispatch.rs:17-18,32-33,47-48`）: いつも `true` で、読む所が 0。範囲の外では `areka-placement` のテスト（`balloon_lock_drag_tests.rs:244`・`follow_test_support.rs:51,105,116`・`balloon_limit_flip_tests.rs:315`）が `is_primary: true` を書く。
- **困る場面**: ⑸ 利用者が右ボタンのドラッグを有効にしたつもりで何も起きない。ほかは読む人と直す人の手間だけ。
- **いちばん小さい直し**: ⑴〜⑷ は消す（⑴ はテスト 5 本ごと・⑷ は `#[cfg(test)]` でなく消してテストも消す）。⑸ ⑹ は消すか残すかが議題（下の欄）。
- **範囲の外への波及**: ⑴〜⑷ はなし。⑸ は `audit-wintf-window`（C20 で走っている）のテスト 1 行、⑹ は `audit-placement-follow`・`audit-placement-core`（C20 で走っている）のテスト 5 か所に触る＝申し合わせ（要件 5.2）。⑶ ⑸ ⑹ は wintf の公開の形を減らす（議題 6）。
- **残す 3 種類**: ⑴〜⑷ は当たらない（⑷ の許しは理由を書いていない＝2 種類目に当たらない見込み）。⑸ ⑹ は **1 種類目の候補**: 完了した `event-drag-system` の要件 1.2・1.5・1.6 は「左・右・中のどのボタンでもドラッグ準備に入る」「ボタンの種別を識別できる」「ボタンごとに有効・無効を設定できる」と書く（今の実装は左だけ）。消すのはこの要件を閉じること、右・中のドラッグを作るのは新しい機能（範囲の外）＝議題 2。

#### 10. ポインタの速さと時刻は誰も読まない（推測で作った機能）・座標を f32 へ往復させる（`pointer/types/mod.rs:73-91,155-159,162-181,212-293`・`pointer/buffers.rs:50-65,144-179`）

- **何か**: 位置を最大 5 件の `PositionSample`（f32 の座標と時刻）として貯め、最新 2 件から速さを出して `PointerState.velocity`（`CursorVelocity`）へ入れる。速さを読む所はツリーに 0（`examples` を含む grep）。`PointerState.timestamp` は作ったときの `Instant::now()` のまま一度も書き換わらず（「タイムスタンプ」と書くが中身は作った時刻）、読む所も 0。座標は窓の手続きの i32 を `as f32` で入れ、移すときに `as i32` へ戻す（そのための「飽和」の説明 `buffers.rs:156-161` とテスト 2 本 `:519-569`）。
- **困る場面**: 速さを使うつもりで読むと、1 tick の中に 2 回以上動いたときしか 0 でない値が入る（貯めは毎 tick 空にする）。時刻を読むと、窓に入った時刻が返る。どちらも誤りに気づけない。
- **いちばん小さい直し**: entity ごとに「最新の位置（i32 の点）」だけを持つ形にし、`PositionSample`・`PointerBuffer`・`calculate_velocity`・`CursorVelocity`・`velocity`・`timestamp` と、そのテスト（`types/tests.rs` の速さと貯めの 9 本・`buffers.rs` の 3 本）を消す。`PointerState` の `Default` は `derive` にできる。
- **範囲の外への波及**: `CursorVelocity` は `ecs/mod.rs:33` で再公開している（その 1 語を消す・完了した見直しの持ち物）。窓の手続きは `..Default::default()` で組むので直しは要らない（`push_pointer_sample` の引数を i32 にするなら `mouse_move.rs:383,436` の 2 行＝`audit-wintf-window` と申し合わせ）。公開の形を減らす（議題 6）。
- **残す 3 種類**: **1 種類目の候補**: 完了した `event-mouse-basic` の要件 3 は「速さを含める・最大 5 件の履歴」を「撫でる操作の検出のため」と書く。ただし後の決め（`roadmap-history.md:255`「撫で＝OnMouseMove 連打の解釈は SHIORI 側の領分」）で、撫での見分けは本体の仕事ではなくなった。夢の段の `window-snap` の brief（`:33`）は「`CursorVelocity` はポインタの速さ」と名指しするが、使うとは書いていない＝議題 3。

#### 11. `PointerState.local_point` は名前と違い、いつも窓の中の座標（`pointer/types/mod.rs:121-123`・`pointer/buffers.rs:164`）

- **何か**: 説明は「エンティティローカル座標」だが、移すときに `local_point = client_point` を入れるだけ（窓の手続きが作るときも同じ値）。完了した `event-mouse-basic` の要件 4 は「`GlobalArrangement.bounds` の左上を引いてローカル座標にする」と書く。
- **困る場面**: 窓の (100,100) に置いた子の部品の受け手が、自分の左上を (0,0) と思って `local_point` を読むと (100,100) を受け取る。`crates/wintf/examples/taffy_flex_demo/`・`crates/areka/examples/emo-present/input.rs` は `local_point` を「部品の中の座標」として記録に出している（値は窓の中の座標）。製品の受け手（areka）は `client_point` だけを読む。
- **いちばん小さい直し**: 案 甲＝移すときに当たった entity の `GlobalArrangement.bounds` の左上を引いて、本当のローカル座標を入れる（完了した要件 4 どおり・形は変わらない）。案 乙＝`local_point` を消し、受け手は `client_point` を使う（公開の形を減らす）。
- **範囲の外への波及**: 案 甲は窓の手続きの 3 か所（`mouse_click.rs:119`・`mouse_dblclick_wheel.rs:90`・`mouse_move.rs:364,422`）が作るときの値も揃える要がある＝`audit-wintf-window` と申し合わせ。案 乙はその 4 行と examples（完了した `audit-wintf-runtime` の持ち物）に触る。
- **残す 3 種類**: 案 乙は 1 種類目の候補（完了した要件 4 を閉じる）。案 甲は当たらない＝議題 4。

#### 12. ドラッグの始まりと終わりの窓探しが、同じ仕事の関数 `find_owner_window` の写し（`drag/dispatch.rs:89-133,260-327`）

- **何か**: どちらの腕も `ChildOf` をたどって `Window` を持つ entity を探すループを手で書く。同じ仕事は `ecs/window/command.rs:850-867` の公開の `find_owner_window` がしており、範囲の中でも `buffers.rs:265` が使う。
- **困る場面**: 窓の見つけ方（例えば親の付け替え）を片方だけ直すと、始まりと終わりで別の窓を相手にする（始まりで `WindowDragging` を付けた窓と、終わりで外す窓が違う）。
- **いちばん小さい直し**: 2 つのループを `find_owner_window(world, entity)` の 1 行にする（約 40 行減る・指摘 1・3 と同じタスク）。
- **範囲の外への波及**: なし（`command.rs` は読むだけ）。
- **残す 3 種類**: 当たらない。

#### 13. 押下の記録の「離した」の枝は何も変えず、説明とテストが古い意味のまま（`pointer/buffers.rs:181-239,417-446`・`pointer/systems.rs:113-139`）

- **何か**: `PointerState.*_down` は「この tick に押された」の旗で、配りの後に `dispatch_pointer_events` が毎 tick 下ろす（`dispatch/mod.rs:262-272`・`types/mod.rs:125` の説明）。ところが移す所は「押下が無く解放があれば `*_down = false`」の枝（`buffers.rs:220-237`）を持ち、説明は「エッジ検出・既存の状態を維持」と、押している間ずっと真の古い意味で書く。旗は既に偽なので、この枝は何も変えない。テスト `buffers.rs:417-446` は「イベントなしでは押下状態を維持」、`systems.rs:113-139` は「ボタン状態は transient ではなく保持」と、製品が見せない振る舞いを固定する。
- **困る場面**: 読んだ人が「押している間は `left_down` が真」と思って受け手を書くと、押し続けている間の tick で偽を受け取る。テストがその誤った読みを裏付ける。
- **いちばん小さい直し**: 枝と古い説明を消し、2 本のテストを「移すと押した tick だけ真・配りの後は偽」に直す。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

### できれば

#### 14. 1 クリック・1 ドラッグごとに `info!` を出す（`pointer/buffers.rs:75-80,92-97`・`drag/dispatch.rs:190-195,229-235,301-306`）

- **何か**: 押す・離すのたびに「デバッグ用に info レベルで出力」の 2 行、ドラッグの始まり・終わりと窓の位置の同期で 1〜3 行を `info!` で出す。`.kiro/steering/logging.md` は `info!` を「ライフサイクルイベント」とする。
- **困る場面**: 1 日に何千回とクリックすると、既定の記録の段（info）で記録のファイルがクリックの数だけ伸び、起動・終了などの本当の出来事が埋もれる。
- **直し**: `debug!` にする（文言と欄は変えない）。**波及**: 記録の行を数えるテストが範囲の外にあるかは実装で grep する（今の grep では `[ButtonBuffer]`・`[DragStartEvent]` の文言を読むテストは 0）。**残す 3 種類**: 当たらない。

#### 15. `DraggingState.initial_inset` の説明が違い、名前も中身と違う（`drag/mod.rs:79-80`・`drag/dispatch.rs:155-160`）

- **何か**: 説明は「ドラッグ開始時の BoxStyle.inset (left, top)」だが、中身はドラッグを始めたときの窓の位置（`dispatch.rs:159`）。使い手（`areka-placement/src/follow/drag_follow.rs:271-295` は「フィールド名は歴史的経緯」と注記・`areka/src/runtime/frame/dpi.rs:179-230`・窓の手続き `mouse_move.rs:464-479`）はどれも窓の位置として読む。
- **直し**: 説明を事実に合わせる（範囲の中）。名前を `initial_window_pos` へ改めるかは議題 7（範囲の外の 3 クレートに触る）。**残す 3 種類**: 当たらない。

#### 16. 古くなった説明・行き先の無い参照

- `pointer/nchittest_cache.rs:25-27,142-144`「handlers.rs L820-876」＝そのファイルは無い（同じ説明が 2 度）。`pointer/buffers.rs:38`「handlers.rs から使用」も同じ。
- `drag/state/mod.rs:613-621`「P62 に記録」「W7a 境界」「本ループ」＝P62 はツリーに無い（指摘 9 ⑴ で関数ごと消えれば説明も消える）。
- `clickthrough/mod.rs:10-13`・`controller.rs:253-266,276-279,427-429`「task 3.2 の責務」「後続タスク 3.2」＝済んでいる。`controller.rs:55-57`「タスク 3.1」。
- `clickthrough/controller.rs:150-152`「`Window` コンポーネント喪失も刈り取る」＝刈るのは entity が消えたときだけ（`:105-108`・テスト `controller_tests.rs:513-525` は逆を固定）。
- `drag/context.rs:1-4,44-46`・`drag/state/mod.rs:358-360`「ECS スレッド→wndproc スレッド間」＝どちらも同じ UI スレッド（`thread_local!` の控えと同じスレッドで動くことが前提）。
- `drag/context.rs:18`「ウィンドウ枠を含むスクリーン座標」と `drag/state/mod.rs:87`「クライアント領域スクリーン座標」が同じ値を別々に書く（中身は前者・`dispatch.rs:105-114`）。
- `drag/accumulator.rs:126`「Dragging 状態の時のみ Some」＝始まりを積んでから終わりを積むまで Some（`:33` が正しい）。
- `pointer/systems.rs:65-68`「Added も Changed に含まれるのでスキップ」＝スキップしていない。
- **直し**: 事実に合わせるか消す。**波及**: なし。**残す 3 種類**: 当たらない。

#### 17. デバッグの見張りの「ダブルクリック」の枝は決して出ず、後始末が 2 か所で同じことをする（`pointer/systems.rs:22-26,70-77`・`ecs/world/mod.rs:208-216`）

- **何か**: `debug_pointer_state_changes` は配りの後に回る（`world/mod.rs:216` の `.after(dispatch_pointer_events)`）。配りは最後に `double_click` を下ろす（`dispatch/mod.rs:269`）ので、見張りがダブルクリックを見ることは無い。FrameFinalize の `clear_transient_pointer_state` も同じ `double_click` を下ろす（配りの後なので何も変えない）。
- **直し**: 見張りのダブルクリックの枝と、`clear_transient_pointer_state` の `double_click` の 1 行を消す（ホイールは Input より後の系（`areka` の `char_watch.rs`・`balloon_wheel.rs`）が読むので FrameFinalize で下ろすのは残す）。**波及**: なし（`world/mod.rs` の順は変えない）。**残す 3 種類**: 当たらない。

#### 18. `DragStateSnapshot` は `DragState` の写しで、3 か所を一緒に直す（`drag/state/mod.rs:113-230`）

- **何か**: 捕捉の守り（`CaptureGuard`）を持つので `DragState` は `Clone` できず、守りを除いた写しの列挙と、写す関数（`snapshot`・各腕を `..` で受ける）と、同じ真偽表の `is_button_held` を 2 つ持つ（約 120 行）。
- **困る場面**: `Dragging` に欄を足して `DragStateSnapshot` に足し忘れても、`snapshot` は `..` で受けるのでコンパイルが通り、窓の手続きとクリック透過からその欄が見えない。
- **直し**: 守りを列挙の外（自分の `thread_local!` の 1 枠）へ出せば `DragState` は `Copy` にでき、写しの列挙と写す関数が消える。触るのは範囲の外の写しの読み手（`mouse_move.rs`・`controller.rs`・wintf の窓の手続きのテスト・`areka` の `menu/trigger_flow_tests.rs`）。差分が大きく、守りを落とす順（借用の外で落とす）の約束を作り直す要がある＝やるかは議題 8。**残す 3 種類**: 当たらない。

#### 19. `OnPointerMoved` は動かなくても毎 tick 配られる（`pointer/dispatch/mod.rs:221-228`）

- **何か**: `PointerState` を持つ全部の entity に、位置が変わったかに関わらず毎 tick `OnPointerMoved` を配る（説明も「常に発火」）。
- **困る場面**: カーソルをシェルの上に置いたまま、ゴーストが動きのある間（60 回/秒の tick）、本体の `on_char_pointer_moved`（`areka/src/input_events/mod.rs:466-500`）が当たり判定の解決を毎回繰り返す（送るかどうかは下流の間引きが位置で決める）。手間は小さい。
- **直し**: 移すときに「この tick に位置が変わった」の旗を立て、旗が立った entity にだけ配る。**波及**: 受け手は本体の `input_events/`（キャラ・バルーン）と examples。毎 tick 呼ばれることを当てにした受け手が無いかを、直す前に確かめる。**残す 3 種類**: **3 種類目の候補**: 本体はこの知らせから `OnMouseMove` を SHIORI へ送る。ukadoc（<https://ukatech.shillest.net/ukadoc-spec/list_shiori_event.html#OnMouseMove>）の原文は「マウスが移動した際に発生。」で、直しはこれに近づく向き。ただし下流の間引き（`plan_mouse_move`）が止まっている間にも送っているなら、送る回数が変わる＝議題 9（SSP の MCP で止まっている間の `OnMouseMove` の数を測るかも議題）。

#### 20. HTCLIENT・HTTRANSPARENT を自前で定義している（`pointer/nchittest_cache.rs:22-28`）

- **何か**: `windows` クレートの `WindowsAndMessaging::{HTCLIENT, HTTRANSPARENT}` があり、wintf の `ecs/cursor/answer_tests.rs:5,58` が使っている。
- **直し**: 自前の定数 2 つと説明を消して `windows` の定数を使う（指摘 8 と同じタスク）。**波及**: なし。**残す 3 種類**: 当たらない。

#### 21. `Window` を外して閉じた窓が、クリック透過の対象に残り続ける（`clickthrough/controller.rs:87-108,183-200`）

- **何か**: 刈るのは entity が消えたときだけ。wintf は `Window` を外すことも窓を閉じる道として扱う（`runtime/mod.rs:610-615,700-706` の試験・`reconcile_window_registry`）。その窓は対象に残り、画面の更新ごとと カーソルの移動ごとに、壊れた HWND へ `ScreenToClient` を撃っては失敗して飛ばす（記録は `trace!`）。
- **困る場面**: 壊れた HWND の値が別の窓に使い回されると、その窓（他のアプリのものもありうる）の `WS_EX_TRANSPARENT` を付け外しする。今の areka は窓を `despawn` で閉じるので当たらない。
- **直し**: 刈る条件に「`WindowHandle` が無い・または対象の HWND と違う」を足す。テスト `controller_tests.rs:513-525`（`Window` の無い entity も残す）は逆を固定しているので直す。「`Window` を持たない汎用の対象」を続けて許すかは議題 10。**波及**: なし（形は変わらない）。**残す 3 種類**: 当たらない。

#### 22. 中の溜めの型が公開の形に出ている（`pointer/mod.rs:22`・`pointer/types/mod.rs:208-361`）

- **何か**: `pub mod pointer` の `pub use types::*` で、中の溜めの型 `PointerBuffer`・`PositionSample`・`ButtonBuffer`・`WheelBuffer`（と指摘 9 ⑶ の仮の当たり判定）が wintf の公開の形に出ている。範囲の外の使い手は 0（grep）。
- **直し**: `pub(crate)` へ下げる（10 で消えるものを除けば `ButtonBuffer`・`WheelBuffer` の 2 つ）。**波及**: 公開の形を減らす（議題 6）。**残す 3 種類**: 当たらない。

#### 23. 小さな形

- `drag/context.rs:33-39` の `clear` は `*self = Self::default()` の 1 行。
- `clickthrough/registry.rs:25-29` の手書きの `Default` は `#[derive(Default)]` と `#[default]` で済む。
- `clickthrough/controller.rs:267-317` の `ClickThroughController` は関数 1 つだけの空の型（`start` を自由な関数にすれば型が消える・呼び手は `runtime/mod.rs:324` と自分のテスト）。
- **直し**: 上のとおり。**波及**: `ClickThroughController` を消すなら `runtime/mod.rs`（完了した見直しの持ち物）の 1 行と `use`。**残す 3 種類**: 当たらない。

### 決めごととして指摘に数えなかったもの

- 終わりの直前の移動区間に `OnDrag` が来ない（累積器が離しで対象を先に消す）: 下流がそれを知って `OnDragEnd` の位置で埋めている（`areka-placement/src/follow/drag_follow.rs:111-116`）。指摘 1 ⑶ の直しもこの形を変えない。
- World を借りられない間の左の離しで、離しの記録（`OnPointerReleased`）を飛ばす（`reentry.rs:8-11`）: 設計で決めた扱いで、扱ったときは `warn!` を 1 行残す。
- 錠の毒化で値を捨てる口（`accumulator.rs:148-159`・`context.rs:61-77`）: 錠の中で panic しうる所が無い＝困る場面が作れない。
- カーソルの見張りが 12 ms ごとに起きる（`monitor.rs:12-16,33-34`）: 説明に理由と手当て（差分の守り）が書いてある決め。
- `start_preparing` の `SetCapture` の戻り値を見ない（`capture_guard.rs:24-27`）: `SetCapture` は失敗を返さない（前の持ち主を返す）。
- 範囲の外の気づき（直さない）: 閾値の計算の写しは窓の手続き `mouse_move.rs:228-233` にある（`audit-wintf-window` の持ち物・9 ⑴ では範囲の中の写しだけを消す）。`mouse_click.rs:150-156` も押すたびに `info!` を出す。どちらも `audit-wintf-window` へ伝える。

## 4. 要件と手持ちの対応（Requirement-to-Asset Map）

| 要件 | 手持ち | ギャップ |
|---|---|---|
| 1 洗い出し | 本文書（23 件・全ファイル読了） | なし（要件 1.5 の書き足しは討議の後） |
| 2 全部直す | 1〜8・12〜14・16・17・20・23 は範囲の中か、完了した持ち物の小さな手入れで閉じる | **Constraint**: 9 ⑸⑹・10・11・19 は完了した spec の要件か SSP の振る舞いに関わる。9 ⑸⑹・10・11・15・18 は走っている spec の持ち物に触る直し方がある |
| 3 残す裁定 | 候補は 9 ⑸⑹・10・11（1 種類目）・19（3 種類目） | **Unknown**: 19 は下流の間引きが止まっている間に送るかを確かめる（要件 3.2 の ukadoc の引用は 3 節の 19） |
| 4 直した後 | 全体テスト `tools/test-all.ps1`・`log-capture-kit`（記録の捕まえ）・`TICK_WAKE_TEST_LOCK`・本物の HWND を作るテストの部品（`controller_tests.rs:171-207`）が揃っている | **Missing**: 1〜4・6・8 の「壊れたら落ちるテスト」は今 0 本 |
| 5 範囲の外 | 呼び手は grep で確かめ済み（各件の欄） | **Constraint**: 1 は `tests/drag/`・`world/mod.rs`・`ecs/mod.rs`。9 ⑸・10・11 は `ecs/window_proc/`（`audit-wintf-window`）。9 ⑹ は `areka-placement` のテスト（`audit-placement-*`）。15 の改名は `areka-placement`・`areka`。`ecs/tooltip/` に触る直しは 0 件（要件 5.5） |
| 6 C20 の約束 | 約束 ⑶ の名は範囲に 0。依存の追加 0（`Cargo.lock` は変わらない）。`vendors/` に触らない | なし |
| 7 守る決まり | 1,000 行に近いのは `drag/state/tests.rs` 848・`controller_tests.rs` 702・`buffers.rs` 682・`drag/state/mod.rs` 650 | 足すテストは兄弟の新しいファイルへ（特に `state/tests.rs`）。3 は記録の無い失敗の道を閉じる直し |
| 8 大きさ | — | **Constraint**: 見込み 14〜20 タスク（8 節） |

- 要件で「コード」に数えた `tests.rs` の 3 本について: 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）はテストのファイルもコードのファイルも同じに数える（例外表にテストのファイルが載っている）。数え分けはどの指摘の判断にも効かない。効くのは行数そのもので、`drag/state/tests.rs`（848 行）へテストを足すと 1,000 行に近づく＝新しいテストは兄弟の新しいファイルへ置く。

## 5. 直し方の選択肢

- **案 A（今のファイルの中で直す）**: 全件を今のファイルの中で直す。新しいファイルはテストの兄弟のファイル（ドラッグの配りの重なり・カーソルの見張りの 1 回分・NCHITTEST の答え）だけ。18 はやらず、9 ⑸⑹・10・11 は裁定どおり。
  - ✅ 構造を変えない・差分が小さい・走っている spec の持ち物に触るのは裁定で決めた所だけ。❌ 18 の写しは残る。
- **案 B（ドラッグの状態を作り直してから直す）**: 18 を先に入れ（守りを列挙の外へ）、1 ⑵ の「今のドラッグか」の比べと 3 の警告を新しい形の上で書く。
  - ✅ 写しと `mem::replace` の繰り返しが消え、`drag/state/mod.rs` が短くなる。❌ 窓の手続き（`audit-wintf-window` の持ち物）の写しの読み手を全部直す＝C20 のうちは申し合わせが重い。差分が大きい。
- **案 C（混ぜる）**: 案 A で全部を直し、18 だけを議題 8 の裁定で後へ回す（`audit-wintf-window` の着地の後に、窓の手続きと一緒に直す spec として起票する）。

## 6. 手間と危うさ

- **手間**: M（3〜7 日）。1 件ずつは小さいが、消すテストと書き直すテストが多い（7・10・13）。
- **危うさ**: Low〜Medium。2・12・13・16・17・20・23 は振る舞いを変えない。1 は速い掴み直しの振る舞いが正しくなるだけ。3 は位置の取れない窓を動かさなくなる。4 は再登録の振る舞いが説明どおりになる。19 は SHIORI へ送る回数が変わりうる（議題 9）。10・11・22 は wintf の公開の形を減らすか変える（議題 6）。

## 7. 設計の段への申し送り

- 推す並び: 必ず直す（1）とその周り（3・12）→ ポインタの溜め（2・10・13・14・17・22）→ クリック透過（4・5・21・23）→ テスト（6・7・8・20）→ 説明（15・16）→ 裁定しだいの 9 ⑸⑹・11・19・18。
- 決めること: 9 ⑸⑹・10・11 の裁定（議題 2〜4）・公開の形を減らすか（議題 6）・15 の改名（議題 7）・18（議題 8）・19（議題 9）・21 の汎用の対象（議題 10）。
- 1 のテストは、Input のスケジュール（配りの後に `cleanup_drag_state` が回る今の並び）を本物どおりに組んで回す。`crates/wintf/tests/drag/dispatch_test.rs` の World の組み立てが使えるかを設計で見る。

## 8. タスクの数の見込みと切り方（要件 8.1）

| まとまり | 指摘 | タスクの見込み |
|---|---|---|
| ドラッグの配り | 1・3・12 | 2（1 で 1・3＋12 で 1・テストつき） |
| ドラッグの状態ほか | 9 ⑴⑷・15（説明）・18 | 1（18 をやるなら＋1〜2） |
| ドラッグの設定と知らせの欄 | 9 ⑸⑹ | 0〜1（裁定しだい） |
| ポインタの溜めと型 | 2・9 ⑵⑶・10・13・14・17・22 | 2〜3 |
| ローカル座標 | 11 | 1（案 甲・乙とも） |
| 動きの知らせ | 19 | 0〜1（裁定しだい） |
| NCHITTEST | 8・20 | 1 |
| クリック透過 | 4・5・21・23 | 1〜2 |
| カーソルの見張りのテスト | 6 | 1 |
| 偽の安心のテスト | 7 | 1 |
| 古い説明 | 16 | 1 |
| 確かめ | — | 2（audit のかけ直し・全体テスト） |
| 計 | 23 件 | **14〜20**（18 をやらなければ 14〜18） |

- **切らない（推す）**: 20 を越えない見込み。どの指摘も wintf の 3 つのフォルダの中で、触るファイルが重なる（`dispatch.rs`・`buffers.rs`・`types/mod.rs`）ので、2 本に切ると互いの rebase が増える。
- **切るなら**: 本 spec＝必ず直す＋直したほうがよい（1〜13・約 11〜14）。2 本目（同じ範囲）＝できれば（14〜23・約 4〜6）。2 本目は本 spec の着地を待つ（同じファイルを触る）。

## 9. 並走の兄弟（ウェーブ C20）に及ぶ所

- `audit-wintf-window`（`ecs/window*`）: 9 ⑸（`window_proc/mouse_dblclick_wheel_tests.rs:213` の 1 行）・10（`push_pointer_sample` の引数を i32 にするなら `mouse_move.rs:383,436`）・11（`mouse_click.rs:119`・`mouse_dblclick_wheel.rs:90`・`mouse_move.rs:364,422`）・18（写しの読み手 `mouse_move.rs`）で触る直し方がある＝申し合わせ（要件 5.2）。こちらからの気づき 2 つ（閾値の計算の写し・押すたびの `info!`）を伝える。
- `audit-placement-core`・`audit-placement-follow`（`crates/areka-placement/`）: 9 ⑹ を消すなら、テストの 5 か所の `is_primary: true` を消す＝申し合わせ。15 の改名なら `follow/drag_follow.rs`・`follow_test_support.rs` に触る（議題 7）。約束 ⑶ の名（`MonitorRecord`・`Monitor`・`DESPAWNED_SKIP_TAG`・`areka_placement::diag`）には触らない。`WindowDragContextResource`（`balloon_lock.rs:104-114` が読む）の形は変えない。
- `wintf-visual-damage-redraw`（`ecs/graphics/visual/draw/`・`composite.rs`・`systems/render.rs`・`numerics/matrix3x2.rs`）: どの直しも触らない。
- `wintf-winmsg-executor-import`（`Cargo.lock` の席）: 依存を足す直しは 0 件。
- そのほかの範囲の外: 1 で `crates/wintf/tests/drag/dispatch_test.rs`・`ecs/world/mod.rs`・`ecs/mod.rs`、10・22 で `ecs/mod.rs` の再公開、23 で `runtime/mod.rs`（どれも完了した見直しの持ち物）。11 の案 乙・19 で examples（`crates/wintf/examples/taffy_flex_demo/`・`crates/areka/examples/`）。19 で本体 `crates/areka/src/input_events/`（`audit-areka-input-menu` は C20 にいない・`input-crate-split` を待つ）。v0.2.0 の `instant-to-interrupt-ticks` は本 spec の着地を待つ（spec 台帳）。

## 10. 調べが要る所（Research Needed）

1. 指摘 19: 本体の `plan_mouse_move`（`crates/areka/src/input_events/` の間引き）が、位置が同じ呼び出しで送らないか（送るなら、毎 tick 配るのを止めると SHIORI へ送る `OnMouseMove` の数が変わる）。SSP が止まっているカーソルで `OnMouseMove` を送るかは、要るなら SSP の MCP と `ssp_shiori_log.txt` で測る。
2. 指摘 5: 許しを外したときにコンパイラが指すものの全部（3 節の一覧は読みからの見込み）。設計の段で一度コンパイルして確かめる。
3. 指摘 11 案 甲: 当たった entity の `GlobalArrangement.bounds` を移す時点（tick の頭・レイアウトの前）に読んで、表示と合うか（1 tick 前の配置で引くことになる）。
4. 指摘 1: `crates/wintf/tests/drag/dispatch_test.rs` の World に Input のスケジュールを組んで `cleanup_drag_state` を含めて回せるか（テストの組み方）。

## 11. 議題（要件の討議へ）

1. **切り方（要件 8）**: 推す＝切らない（14〜20 タスク）。
2. **使われないドラッグの設定と知らせの欄（指摘 9 ⑸⑹）**: 推す＝消す（`right_button`・`middle_button`・`is_primary`）。完了した `event-drag-system` の要件 1.2・1.5・1.6（右・中のドラッグ・ボタンの識別・ボタンごとの設定）は今も作られていない口で、設定しても何も起きないのは利用者を迷わせる。右・中のドラッグが要るなら新しい spec で作る。消すと `audit-wintf-window` と `audit-placement-*` のテストに触る＝申し合わせ。
3. **ポインタの速さと時刻（指摘 10）**: 推す＝消す。完了した `event-mouse-basic` の要件 3 の理由（撫での見分け）は、後の決め（撫では SHIORI 側の領分）で無くなった。`window-snap`（夢の段）が投げる速さを要るときは、ドラッグの知らせの位置から求められる。
4. **ローカル座標（指摘 11）**: 推す＝案 甲（完了した `event-mouse-basic` の要件 4 どおり本当のローカル座標を入れる・形は変わらない）。窓の手続きの 4 か所を揃えるので `audit-wintf-window` と申し合わせる。案 乙（消す）は完了した要件を閉じる。
5. **指摘 1 の直し方（どちらの後始末を残すか）**: 推す＝`cleanup_drag_state` を消す（公開の関数を 1 つ減らす）。残すなら、終わりの腕の側の外しを消して 1 か所にする案もあるが、受け手（`OnDragEnd`）が `DraggingState` を読めることが下流の約束（`drag_follow.rs:114-116`）なので、配りの中で外す今の形を正本にする。
6. **wintf の公開の形を減らす直し（9 ⑶⑸⑹・10・11 案 乙・22）**: wintf は crates.io へ出すクレートで、減らすのは次の版で壊れる変更になる。推す＝減らしてよい（0.x の版・v0.2.0 で wintf を作り直す途中・使い手はツリーの中だけで grep で確かめられる）。C20 の約束 ⑶ の名には触らない。
7. **`initial_inset` の改名（指摘 15）**: 推す＝この spec では説明だけを直し、改名は範囲の外の 3 クレート（`areka-placement` は C20 で走っている）に触るので、`audit-placement-follow` の着地の後に回す（`/kiro-complete` の未解決の棚卸で扱う）。
8. **`DragStateSnapshot` の写し（指摘 18）**: 推す＝この spec では直さず、`audit-wintf-window` の着地の後に窓の手続きと一緒に直す（案 C）。そのとき 3 種類のどれにも当たらないので、残す裁定ではなく後へ回す裁定になる（要件 2.2 との関係を討議で確かめる）。
9. **毎 tick の `OnPointerMoved`（指摘 19）**: 推す＝10 節の 1 を確かめてから直す。下流が位置の同じ呼び出しで送らないなら、外から見た振る舞いは変わらず、ukadoc の「マウスが移動した際に発生」に近づく。送っているなら、SSP で測ってから決める（3 種類目）。
10. **`Window` を持たない汎用の対象（指摘 21）**: 推す＝やめる（登録するのは wintf の窓だけ・`WindowHandle` の HWND と一致しない対象は刈る）。テスト `controller_tests.rs:513-525` を逆向きに直す。

## 12. 要件の討議の結果（2026-11-14）

- 議題 1〜10 は推奨案で確定した（正本は `requirements.md` の「確定した指摘の一覧」）。残すと裁定した指摘 0 件・ほかの spec へ回した指摘 0 件。
- 11 節から変えた推し: 議題 7（改名）と議題 8（写し）は「後へ回す」をやめ、この spec で直す（後へ回す持ち主の spec が無く要件 2.2 に反するため・並走の spec とは要件 5.2 で申し合わせる）。
- 10 節の 1 は閉じた: `plan_mouse_move`（`crates/areka/src/input_events/throttle.rs:54-66`）は位置が同じ呼び出しで送らない＝指摘 19 を直しても SHIORI へ送る `OnMouseMove` の数は変わらない（3 種類目に当たらない）。
- 設計の段に残る調べ: 10 節の 2（許しを外したときにコンパイラが指すものの全部）・3（ローカル座標を引く時点の配置）・4（指摘 1 のテストの組み方）。

## 13. 設計の段の調べ（2026-11-14・`/kiro-spec-design -y`）

- 物差し: `kiro-spec-design` の手順（軽い調べ＝既存の仕組みの拡張・新しい依存なし・外の調べは要らない）。使った決まり: `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`。体裁は完了した兄弟 `audit-emo-atlas` の `design.md`。ponytail（full）の「いちばん小さい完全な直し」。
- 直す行を設計の段で開き直した（`main` `30a343eb` から範囲は不変）: `drag/` の `mod.rs`・`systems.rs`・`dispatch.rs`・`accumulator.rs`・`capture_guard.rs`・`context.rs`・`state/mod.rs`・`state/tests.rs` の頭、`pointer/` の `buffers.rs`・`types/mod.rs`・`dispatch/mod.rs`・`systems.rs`・`nchittest_cache.rs`・`mod.rs`、`clickthrough/` の全 5 本。範囲の外は判断に要る所だけ: `ecs/window/command.rs:840-867`・`window_handle.rs:19-37,140-163,208-283`・`components.rs:96-100`・`layout/hit_test/mod.rs:260-520`・`layout/arrangement.rs:68-73`・`layout/systems/window_pos_systems.rs:135-169`・`window_proc/mouse_move.rs:140-480`・`mouse_click.rs:95-140`・`mouse_dblclick_wheel.rs:70-110`・`world/mod.rs:110-230`・`ecs/mod.rs:20-45`・`runtime/mod.rs:280-350,600-715`・`crates/wintf/tests/drag/`・`crates/areka/src/runtime/frame/dpi.rs:170-235`・`input_events/mod.rs:455-505`・`balloon_moved.rs`・`runtime/balloon_hover.rs`・`crates/areka-placement/src/follow/drag_follow.rs:265-300`・`crates/areka/tests/window_link_real_run.rs:49-51,495-535`・`tools/perf/judge-followup.py:98-131`・`doc/click-through.md`・完了した `event-mouse-basic` の要件 4。

### 10 節の調べの答え

- **10 節の 2（指摘 5 でコンパイラが指すもの）**: コードを変えない段なのでビルドしていない。規則（許しを外し、指されたものを消すか `#[cfg(test)]`）と見込みの一覧（3 節の 5 に、口が消えて使われなくなる `ClickThroughHandle` の欄 `registry`・`wake_event` を足した）を設計に書いた。実装の段で確かめる。
- **10 節の 3（ローカル座標を引く時点の配置）**: `GlobalArrangement.bounds` はスクリーン座標（`arrangement.rs:68-73`）。当たり判定はクライアント座標＋`WindowPos.position` を `bounds` と比べる（`hit_test_in_window_ex`）。移すのは tick の頭（レイアウトの前）なので、引く配置は当たり判定が entity を選んだのと同じ前の tick の配置＝表示と合う。ただしドラッグ中は窓の `Arrangement.offset` を写さない（`sync_window_arrangement_from_window_pos` の `Without<WindowDragging>`）ので `WindowPos.position` が `bounds` より先へ進む＝スクリーン座標の原点には窓の entity の `bounds` の左上を使う（差を取れば窓が動いても変わらない）。
- **10 節の 4（指摘 1 のテストの組み方）**: `tests/drag/dispatch_test.rs` の `make_world` は系の登録を持たない。wintf の中のテストは `EcsWorld::new()` を広く使っている（`window_proc/keyboard_tests.rs` ほか 20 本）ので、範囲の中の新しいファイル `drag/dispatch_order_tests.rs` で `EcsWorld::new()` を作り `run_schedule(Input)` で本番の登録どおりに回す。直す前は `cleanup_drag_state` が登録されているので赤になる。

### 設計の段で見つけたこと

- **`is_primary` の書き手は 13 か所**（ドラッグの知らせ・範囲の外）: `areka-placement` 7（`balloon_limit_flip_tests.rs:315`・`balloon_lock_drag_tests.rs:244`・`follow_test_support.rs:51,105,116`・`persist_finalize_save_tests.rs:391`・`spawn_follow_pipeline_tests.rs:40`）・`areka` 4（`examples/mock-shell.rs:794`・`input_events/drag_tests.rs:430`・`runtime/frame_balloon_offset_follow_tests.rs:830`・`runtime/frame_chain_finalize_persist_restart_tests.rs:79`）・wintf のテスト 2（`tests/drag/dispatch_test.rs:106,513`）。要件の討議の「5 か所」は 3 節の 9 ⑹ が挙げた 2 ファイルの数。`move_cue_place_tests.rs:55`・`reset_pos_tests.rs:206`・`follow_work_area_tests.rs:260`・`frame_test_support.rs:618`・`lib.rs:166`・`emo2_real_run/common.rs:106` はモニタの `is_primary`（触らない）。
- **`initial_inset` の読み手は 3 クレート 15 ファイル**（説明とテストの部品 `set_initial_inset` を含む・`design.md` の範囲の外の表）。
- **`[DragStartEvent] Dispatching` を読むテスト**: `crates/areka/tests/window_link_real_run.rs:508,531`（`RUST_LOG` は `info`）。指摘 14 で `debug!` にするので、その `RUST_LOG` に `wintf::ecs::drag::dispatch=debug` を足す。
- **perf の道具が読む記録**: `tools/perf/judge-followup.py` は `clickthrough: ex-style トグル適用` と欄 `desired`・`DesiredState` の綴りを読む（変えない）。行の番号の参照（`controller.rs:212`・`registry.rs:18-23`）はずれる＝`audit-tools-perf-judge` へ知らせる。起動の `debug!`（`ClickThroughController started`）はどの道具も読まない（`tools/perf/fixtures-loop/` の記録に残るだけ）。
- **`Window` を外しても `WindowHandle` は外れない**（`Window` に `on_remove` のフックが無い）。指摘 21 の刈り方は `Window` と HWND の同じ `WindowHandle` の両方を見る。
- **指摘 3 で `DraggingState` を入れないと**、偽の HWND で `client_to_window_coords` が失敗する範囲の外のテスト（`follow_drag_end_gate_tests.rs:248-251` ほか）が `DraggingState` を失う。窓を動かさないのは wintf の控えの `move_window` だけにした。
- **指摘 19 の副作用**: `OnPointerMoved` の本番の受け手は 2 つ（`on_char_pointer_moved`・`on_balloon_pointer_moved`）。キャラの窓は下流の間引きで SHIORI へ送る数が変わらない（議題 9）。バルーンの窓は、止めたカーソルの下で中身が替わったときの選択肢の強調とバルーン滞在の記録（`BalloonHover`・バルーンが自動で閉じるのを抑える観測）が、次に動くまで更新されなくなる。キャラの窓の見張り（`char_watch_tests.rs:173`「止まったまま当たり判定が替わっても、動くまで出ない」）と向きは揃う。設計は議題 9 どおりに書き、`design.md` の「未解決の危うさ」の 1 として設計の討議へ渡す。
- **指摘 18 の守りの順**: 守りを別の枠へ出すと、テストが状態を直に `Idle` へ戻したときに守りが残りうる。`start_preparing` は残った守りを先に落としてから捕捉を取る（逆順だと新しい捕捉を外す）。

### 並走の兄弟との申し合わせ（設計の段で受けた知らせ）

- **`audit-placement-core`（2026-11-14）**: この spec が先に実装して着地し、`balloon_lock_drag_tests.rs`・`balloon_limit_flip_tests.rs` の `is_primary: true` の行も消す。あちらはその後に、消した後の形の上でテストの部品（あちらの指摘 80）をまとめる。あちらが先に着地したら「後から着地する側が読み手を直す」どおりこちらが直す。
- **`audit-wintf-window`（2026-11-14・あちらの開発者が承認・`requirements.md` の議題 11）**: ⑴ `check_threshold` は残し（指摘 7 の消すテストから 5 本を外す）、説明だけ直す。あちらの 62 が窓の手続きの写しを呼び出しに替える。⑵ `ecs/window_proc/mouse_*` の作りの直しはあちら。こちらは型の形の変化への機械的な追従だけ＝ローカル座標は窓の手続きの作る所を変えず、`transfer_buffers_to_world` の最後で全部の `PointerState` に書き直す形にした（3 節の 11 の案 甲を範囲の中で実現する形）。⑶ 着地は先に着いた方がそのまま・後の方が rebase。⑷ `WindowPointerTracking` はあちらが消す。

### 合成（design-synthesis）

- **まとめ（一般化）**: ローカル座標の書き手を 1 か所（tick の頭の移し）にまとめ、窓の手続きの 4 か所・クリック・ダブルクリックの道を同じ規則で覆った。窓探しは `find_owner_window` の 1 つ（指摘 12）。
- **作るか使うか**: 定数は `windows` の `HTCLIENT`・`HTTRANSPARENT`（指摘 20）、窓探しは既存の `find_owner_window`、テストの World は既存の `EcsWorld::new`。新しく作るのは私的な小さな関数だけ（`drag_origin`・`local_point_of`・`answer`・`poll_once`）。
- **減らす**: `DragStateSnapshot`（約 120 行）・`cleanup_drag_state`・`ClickThroughController`・速さと 5 件の履歴・仮の当たり判定・`DOUBLE_CLICK_BUFFERS`・`ClickThroughHandle` の 6 つの口と 2 つの欄。足す公開の欄は `PointerState.moved` だけ。
- 取らなかった案: `DraggingState.initial_window_pos` を `Point` にする（議題 7 の範囲を越える・`dpi.rs` の置き直しの決めがある）／`moved` を前の位置との比べで立てる（比べのための控えが増える・下流の間引きの仕事）／指摘 3 で `DraggingState` を入れない（範囲の外のテストと下流の約束が崩れる）／`update_dragging` に entity を渡す（窓の手続きの呼び出しに触る）。

### 設計の見直しの門（review gate）

- 1 回目: 要件の番号 35 個のうち 4.2〜4.4 が表に行として無かった→行を分けて直した。`collision-probe` の例の `OnPointerMoved` を Revalidation Triggers に足した。
- 2 回目（議題 11 を受けて）: 指摘 9⑴・7・11 の節、範囲の外の表、申し合わせの表、タスクの見込みを直した。機械の確かめ（要件の番号・境界の 4 節・ファイルの計画・部品とファイルの対応）は全部通った。

---
読まなかった所は無い（範囲の 24 ファイルを全部読んだ）。テストもビルドも走らせていない。

## 14. 設計の討議の結果（2026-11-14）

- 議題 1（指摘 19）: 推奨案で確定（2026-11-04 開発者「audit系は推奨案でよい」の常の承認による）。配り方は変えず、毎 tick 配る理由を説明に書いて檻に入れる。理由＝バルーンの窓の受け手（`balloon_moved.rs:114-116`）が毎 tick の知らせでバルーン滞在（`BalloonHover`）を立て直し、バルーン可視性の相（`balloon_visibility_phase`）がそれで自動で閉じるのを抑える。滞在は非表示へ移るときに消される（`clear_balloon_hover`）ので、止めたカーソルの下で出直したバルーンは、毎 tick の配りが無いとカーソルを動かすまで閉じうる。要件の議題 9 の決めも合わせて改めた。
- 設計の検証の重い指摘 1（指摘 3 の直しが `balloon_drag_held` とぶつかる）は自明な直しとして直した＝控えの `move_window` は `DragConfig` のまま・位置の取れないことは `initial_window_pos: None` で伝え、`update_dragging` が `ctx.move_window && initial_window_pos.is_some()` で決める。
- 設計の検証の重い指摘 3（タスクの表の 8 行目が議題 11 と矛盾）・受け手の数（2→3）・`window-snap/brief.md:33` の文字列での参照も自明な直しとして直した。

## 15. 実装の後の audit のかけ直し（2026-11-15・タスク 7.1）

- 物差しは 2 節と同じ（`/ponytail:ponytail-audit`・ponytail 5.3.3・件数の上限なし・全部読む）。範囲は要件の 24 ファイルに、実装で足した `drag/dispatch_order_tests.rs` を加えた 25 ファイル（7,387 行・かけ直しの前）。25 本とも全行を読んだ。読まなかった所は無い。
- 数えるのは新しい指摘だけ。次は新しい指摘に数えない: 直し済みの 23 件・議題 11 で残した `check_threshold`・窓の手続きが `PointerState` を作る所（議題 11）・範囲の外の clippy の機械的な指摘（`collapsible_if` ほか・`clippy-199-lints` の持ち物）・`tooltip/`。
- 組は 3 節と同じ 3 つ（必ず直す・直したほうがよい・できれば）。番号は 3 節に続けて 24 から振る。

### 1 回目

| 組 | 件数 | 指摘 |
|---|---|---|
| 必ず直す | 0 | — |
| 直したほうがよい | 3 | 24・25・26 |
| できれば | 3 | 27・28・29 |
| 計 | 6 | 6 件とも直した（24 は進行役の許しの後に直した） |

- **24. ドラッグの知らせ 3 つの `Messages` を、製品の誰も読まない（直しが生んだ）**（`drag/dispatch.rs:172-174,217-219,335-337`・範囲の外の `ecs/world/mod.rs:139-141,378-398`）: 配りは知らせのたびに `Messages<DragStartEvent>`・`Messages<DragEvent>`・`Messages<DragEndEvent>` へ写しを書き、`EcsWorld` は 3 つを登録して毎フレーム `update()` を回す。読み手だった `cleanup_drag_state`（`MessageReader<DragEndEvent>`）をタスク 2.1 で消したので、ツリー全体（`examples` を含む grep）で製品の読み手は 0。読むのはテストだけ（wintf の `drag/dispatch_order_tests.rs`・`window_proc/keyboard_tests.rs`・`mouse_click_tests.rs`・`runtime/wndproc_bridge_drag_{cancel,release}_tests.rs`・`wndproc_bridge_drag_test_support.rs`・`tests/drag/dispatch_test.rs`、`areka-placement` の `balloon_lock_drag_tests.rs:99-101,444`・`follow_drag_end_gate_tests.rs:66-72`、`areka` の `input_events/drag_test_support.rs:160-162`）。いちばん小さい直し＝3 つの書き込みと登録と `update()` を消し、テストの数え方を受け手（`OnDragStart`・`OnDragEnd`・`OnDrag`）の呼ばれた数に替える。要件 3 の 3 種類には当たらない（完了した `event-drag-system` の要件にも、記憶の決めにも `Messages` の口は無い・SSP に無い仕組み）。直しが範囲の外の 10 ファイル（`ecs/world/mod.rs` と上のテスト 9 本・走っている `audit-placement-core` の 2 ファイルを含む）に及ぶので、いったん進行役へ渡し、その許し（10 ファイルと、同じ消しでコンパイラが指すファイル）と `audit-placement-core` への行の知らせの後に直した。
  - 製品: `drag/dispatch.rs` の 3 つの書き込みと 3 つの知らせの `derive(Message)`、`ecs/world/mod.rs` の 3 つの登録と `FrameFinalize` の `update()` 3 つを消した。配りの記録（`[DragStartEvent] Dispatching` ほか）の文言・欄・段は変えていない。
  - テストは数え方だけを替え、確かめることは変えていない（受け手の呼ばれた数で数える）。wintf の中のテストは、範囲の中に足した `drag/seen_test_support.rs`（`#[cfg(test)]`・受け手 3 つと溜め）の `watch`／`take` を使う: `drag/dispatch_order_tests.rs`・`window_proc/keyboard_tests.rs`・`mouse_click_tests.rs`・`runtime/wndproc_bridge_drag_test_support.rs`（対象の窓に受け手を付け、`dispatch` の読み出しを替えた）・`wndproc_bridge_drag_cancel_tests.rs`。wintf の結合テスト `tests/drag/dispatch_test.rs` は自分の受け手 3 つと溜め（`Seen`）を持つ。`areka-placement` の `balloon_lock_drag_tests.rs` は、止めたドラッグの終わりの受け手だけが出す記録（`balloon_drag_held`）が 1 件であることで終わりの受け手が 1 回呼ばれたことを確かめる形にし、`follow_drag_end_gate_tests.rs` と `areka` の `input_events/drag_test_support.rs` は要らなくなった登録の行と `use` を消した。
  - コンパイラが指した 10 ファイルの外のファイルは 1 本: `window_proc/mouse_dblclick_wheel_tests.rs`（同じ `Messages` の読み出し・同じ形で受け手の数に替えた）。
  - 数え方の替えが本物を見ていることの確かめ: 終わりの腕の受け手の配りを一時的に外すと、wintf のテストのうち終わりを 1 件と数える 10 本（`mouse_click_tests`・`wndproc_bridge_drag_{cancel,release}_tests` の t01・t02・t07・t07b・t10）が赤になり、戻すと緑。
- **25. 落ちえない・同じことを 2 度確かめるテスト（直しが生んだものを含む）**: `pointer/types/tests.rs` の 3 本（`PointerState` の既定＝4.1 で `derive` になって `derive` の事実だけになった・`ButtonBuffer` と `WheelBuffer` の 2 本＝`buffers.rs` の 2 本と同じ確かめ）、`pointer/nchittest_cache.rs` の 2 本（`test_cache_multiple_hwnds` と同じ 1／−1 の出し入れ）、`pointer/dispatch/tests.rs` の 2 本（受け手の無い配りは何もしない＝落ちえない・`test_dispatch_with_handler`＝毎 tick の檻の 1 回目と同じ）、`drag/context.rs` の `test_default_is_unset`（`derive` の事実）、`clickthrough/controller_tests.rs` の `start_then_drop_joins_worker_without_hanging` の中ほど（5.2 で表を直に触る形にした結果 `RefCell` の振る舞いだけを確かめる・本物の HWND は OS へ渡らない）。直し＝消した（残るテストがそれぞれの仕組みを覆う）。
- **26. `ClickThroughRegistryHandle` の `remove`・`len`・`is_empty` を製品の誰も使わない**（`clickthrough/controller.rs:356-369`）: 使い手はテスト 2 本だけ（`controller_tests.rs`・範囲の外の `runtime/mod.rs` の結線のテスト）。除去は `prune_dead_targets` が毎評価で行うので、説明の「二重の安全弁」も当たらない。直し＝`remove` を消し（`ClickThroughRegistry::remove` とそのテストも）、`len`・`is_empty` は `#[cfg(test)]` の `pub(crate)` にした。説明と `doc/click-through.md` の 2 行（109・149）、`runtime/mod.rs` の結線のテストの説明 1 行と `remove` の 2 行を直した。
- **27. `CaptureGuard` の手書きの `Debug` が要らなくなった（直しが生んだ）**（`drag/capture_guard.rs:64-71`）: 1.1 で守りが `DragState` の外へ出て、`Debug` を求める所が無くなった（その文字列を見るテストも 1.1 で消えた）。直し＝消した。
- **28. `update_dragging` の移動中の腕が、2 つの欄を変えるために 9 つの欄を組み直す**（`drag/state/mod.rs:289-311`）: 直し＝`match state` にして、移動中の腕は `prev_pos`・`current_pos` の 2 つだけを書き換える（約 20 行減る・振る舞いは同じ・今のテスト `test_update_dragging_dragging_updates_prev_pos` ほかが覆う）。
- **29. 手書きの `Default` 3 つ**（`drag/accumulator.rs` の `DragAccumulator`・`DragAccumulatorResource`、`drag/context.rs` の `WindowDragContextResource`）: 3 節の 23 の `registry.rs` と同じ形。直し＝`#[derive(Default)]` にし、`new()` は `Self::default()` にした。
- 数えなかったもの（決めごと）: ドラッグの知らせ 3 つと `DragTransition::Started` の `timestamp`・`DragState` の `start_time` は製品で読まれないが、設計の Non-Goals が残すと決めており（議題 3 で消すのは `PointerState` の速さと時刻だけ）、消すと完了した `event-drag-system` の要件 3.5（ドラッグ経過時間をイベント情報に含める）を閉じる＝要件 3 の 1 種類目に当たる。窓の位置が取れないときに `DraggingState.initial_window_pos` が (0,0) で入るのは設計の決めごと 5（`DraggingState` は今どおり入れる）。`drag_origin` の「`WindowHandle` が無ければ位置をそのまま使う」は 2.2 の実装の決め。

### 2 回目

| 組 | 件数 |
|---|---|
| 必ず直す | 0 |
| 直したほうがよい | 0 |
| できれば | 0 |
| 計 | 0 |

- 1 回目の直し（24 を除く 5 件）を入れた範囲の 25 ファイルを読み直した（直したファイルは全行・ほかは直しが及ぶ所）。直しが生んだ新しい指摘は 0。このとき 24 は進行役の決めを待っていた（新しい指摘には数えない）。
- 確かめ: `cargo test --workspace --all-targets --no-run`・`cargo test -p wintf`・`cargo test -p areka-placement`・`cargo test -p areka --bin areka`・`cargo test -p areka --example mock-shell`・`cargo test -p log-capture-kit --test file_length_guard_test`・`cargo clippy -p wintf --all-targets`（範囲の中の警告は前からの `collapsible_if` ほか＝`clippy-199-lints` の持ち物）・`cargo fmt --all -- --check`。

### 3 回目（24 を直した後）

| 組 | 件数 |
|---|---|
| 必ず直す | 0 |
| 直したほうがよい | 0 |
| できれば | 0 |
| 計 | 0 |

- 範囲は 24 の直しで足した `drag/seen_test_support.rs` を加えた 26 ファイル。24 の直しで変わった範囲の中の 3 本（`drag/dispatch.rs`・`drag/mod.rs`・`drag/dispatch_order_tests.rs`）と新しい 1 本は全行、ほかは 1・2 回目からの変わりが無いことを差分で確かめた。直しが生んだ新しい指摘は 0。
- 範囲の外で目に留まった古い説明（`ecs/world/mod.rs` の「ECS→wndprocスレッド間転送用」・`tests/drag/dispatch_test.rs` の頭の `state.rs` の参照）は範囲の外の持ち物として数えない（要件 5.4）。
- 確かめ: 2 回目と同じ 8 つのコマンドがどれも緑。

### 判定

- **0 件で終わった**（3 回目で新しい指摘 0・1 回目の 6 件は全部直した・5 回目での打ち切りは使っていない）。要件 3 の議題に当たる新しい指摘は 0（ドラッグの知らせの `timestamp` は設計の Non-Goals の決めごとで、数えなかったものに書いた）。
