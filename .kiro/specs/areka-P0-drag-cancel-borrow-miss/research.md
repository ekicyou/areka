# ギャップ分析: areka-P0-drag-cancel-borrow-miss

> 2026-10-04 `/kiro-validate-gap`。対象は確定済みの `requirements.md`（要件 1〜7）と、ワークツリーの実物（`27536314` の時点）。本書は分析と選択肢だけを書き、決定はしない。
>
> **追記（同日・要件ディスカッション議題 1 の裁定）**: §6 の議題 1 は **(b)＝境界を `runtime/wndproc_bridge.rs` まで広げ、ドラッグに関わる 5 種だけ入口を通して本番の道として直す（案 C を組む）** に決まった。`requirements.md` はこの筋で書き直し、入口の要件を新しい要件 1 として先頭に足したため、**本書の要件番号は改訂後の番号より 1 つ小さい**（本書の要件 n＝改訂後の要件 n+1。例: 本書の 1.3 → 改訂後 2.3、本書の 6.1 → 改訂後 7.1、本書の 7.1 → 改訂後 8.1）。議題 2〜5 は設計の判断として `/kiro-spec-design` へ持ち越す。

---

## 0. 先に読むこと（要件の前提とコードの実物の食い違い）

要件と brief は「画面更新（tick）が World を借りている最中に同期で届いたメッセージを、**wintf のハンドラが受け取り**、World を使えずに種を積まずに先へ進む」という筋で書かれている。ところが本番の経路では、**その条件のメッセージはハンドラまで届かない**。

- 本番の窓の手続きは `crates/wintf/src/runtime/wndproc_bridge.rs` の `make_wndproc` が作るクロージャだけである（`crates/wintf/src/runtime/window_factory.rs` の `create_window` が `LibWindow::new_ex` へ渡す。`dispatch_window_message` を本番で呼ぶのはこのクロージャだけ）。
- このクロージャは、`dispatch_window_message` を呼ぶ**前に** `world.try_borrow()` を試し、失敗したら `None` を返して既定の手続き（`DefWindowProcW`）へ任せる（同ファイルの冒頭の「安全スキップ規律（要件 4.3）」・テスト `closure_safe_skips_when_world_borrowed` が固定している）。例外は `WM_ENDSESSION`（wParam 真）の `warn!` だけ。
- tick は `crates/wintf/src/runtime/tick_bridge.rs` の `tick_one_frame_with` で `world.try_borrow_mut()` を握ったまま回る。その間に OS が同期で送ったメッセージは、上のクロージャの入口で捨てられる。
- クロージャの入口で `try_borrow()` が成功した直後に `dispatch_window_message` へ入るので、ハンドラの中の最初の `world.try_borrow()`（共有の借用）は、ハンドラ自身が可変の借用を握っていない限り失敗しない。キー・取り消し系の 4 つのハンドラ（`keyboard.rs` の `WM_KEYDOWN`・`WM_CANCELMODE`・`WM_ACTIVATE`・`WM_CAPTURECHANGED`）と、左ボタンを離したときの予備の枝（`mouse_click.rs` の `handle_button_message` の「フォールバック」の枝の 2 つの `try_borrow()`）の**失敗の枝は、本番の経路からは届かない**。
  - 唯一の入れ子は、`handle_button_message` の当たり判定の枝が `try_borrow_mut()` を握ったまま `end_dragging` を呼び、`CaptureGuard` の `ReleaseCapture` が同期で `WM_CAPTURECHANGED` を送るときである。このメッセージもクロージャの入口で捨てられるが、状態はすでに休む状態（`JustEnded`）へ移っているので害は無い。
  - 外側が共有の借用だけを握っている入れ子では、クロージャの入口も各ハンドラの `try_borrow()` も成功する（当たり判定の枝の `try_borrow_mut()` だけが失敗して予備の枝へ落ち、予備の枝は World を使えるので正しく終える）。
- **本番で実際に起こる再入の形**は「メッセージがまるごと捨てられ、ドラッグの状態も累積器も何も変わらない」である。たとえば tick の中のシステム `create_windows`（`crates/wintf/src/ecs/window/window_system.rs`）が窓を作ると、新しい窓の活性化で、ドラッグ中の窓へ `WM_ACTIVATE`（非活性化）が同期で届きうるが、これは捨てられてドラッグは続く。害が出うるのは `WM_CAPTURECHANGED` が捨てられた場合で、捕捉を失ったのに状態は `Dragging` のまま残り、左ボタンを離した知らせが別の窓へ行くと、その後ボタンを押さずに窓の上を動かしただけで窓が付いて動きうる（症状は要件 3.3 に似るが、仕組みが違う）。
- 要件 7.1 の「再入の状態を作って本物のハンドラへメッセージを渡す」テストは、`dispatch_window_message` を直に呼べば作れるが、それは本番が通らない道である（開発規律「檻は到達する経路を踏ませよ」と食い違う）。`make_wndproc` のクロージャを通すと、今はハンドラが呼ばれないので、要件が想定する赤（置き去り・次のクリックの保存）は出ず、別の形（状態がそのまま残る）になる。
- 完了 spec `areka-P0-drag-click-without-move` の `tasks.md` の申し送り（本 spec の起票元）も、このクロージャの安全スキップには触れていない。

この食い違いは、本 spec の範囲（境界）と、テストが踏む経路を決め直す議題になる（§6 の議題 1）。以下の分析は、要件の文面どおりの範囲と、本番の経路まで含めた範囲の両方を扱う。

---

## 1. 今の作り（現状の調査）

### 1.1 関係するファイルと役目

| 役目 | ファイルと定義 | 行数 |
|---|---|---|
| wndproc の入口（再入の安全スキップ） | `crates/wintf/src/runtime/wndproc_bridge.rs` の `make_wndproc` | 約 200（テスト込み） |
| メッセージの配送表 | `crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message`（入口で `tick_wake::mark` を立てる＝World に依らない） | 334 |
| キー・取り消し系 4 ハンドラ | `crates/wintf/src/ecs/window_proc/keyboard.rs` の `WM_KEYDOWN`・`WM_CANCELMODE`・`WM_ACTIVATE`・`WM_CAPTURECHANGED` | 263 |
| ボタンのハンドラ | `crates/wintf/src/ecs/window_proc/mouse_click.rs` の `handle_button_message`（当たり判定の枝と予備の枝） | 573 |
| 移動のハンドラ（範囲外・参照のみ） | `crates/wintf/src/ecs/window_proc/mouse_move.rs` の `WM_MOUSEMOVE`（処理全体が `try_borrow_mut()` の中） | 602 |
| ドラッグの状態機械 | `crates/wintf/src/ecs/drag/state/mod.rs` の `DragState`・`thread_local! DRAG_STATE`・`start_preparing`・`start_dragging`・`update_dragging`・`end_dragging`・`cancel_dragging` | 574 |
| 累積器（wndproc から ECS への箱） | `crates/wintf/src/ecs/drag/accumulator.rs` の `DragAccumulator`・`DragAccumulatorResource`（`Arc<Mutex<…>>` を包む `Clone` の資源）・入口の判断 `set_transition` | 181 |
| 知らせを配る段 | `crates/wintf/src/ecs/drag/dispatch.rs` の `dispatch_drag_events`（`flush` して `Started`／`Ended` を積んだ順に配る・`DraggingState` と `WindowDragging` を付け外し） | 382 |
| 累積器を World へ入れる所 | `crates/wintf/src/ecs/world/mod.rs` の `EcsWorld::new` の中の `insert_resource(DragAccumulatorResource::new())` の 1 行 | — |
| 捕捉の RAII | `crates/wintf/src/ecs/drag/capture_guard.rs` の `CaptureGuard`（`hwnd` を持つ・`mark_released`） | 104 |
| 終了の受け手（範囲外・参照のみ） | `crates/areka/src/placement/follow/drag_follow.rs` の `on_char_drag_end`・`on_balloon_drag_end` | — |

### 1.2 ハンドラごとの World の使い方

| ハンドラ | World で何を読むか | 借用が取れないとき（今） |
|---|---|---|
| `WM_KEYDOWN`（ESC） | 累積器を取り出して `Ended{cancelled:true, end_pos:start_pos}` を積む | 積まずに `cancel_dragging()`（状態は `JustEnded`） |
| `WM_CANCELMODE` | 同上 | 同上 |
| `WM_ACTIVATE`（非活性化） | `JustStarted`／`Dragging` のときだけ同上。あわせて `try_borrow_mut()` で沈降観測の目印 | 積まずに `cancel_dragging()`。目印も付かない（目印は本 spec の範囲外） |
| `WM_CAPTURECHANGED` | 同上（先に `mark_released`） | 積まずに `cancel_dragging()` |
| `handle_button_message`（離し） | (a) 画面の座標を作るための `WindowPos`、(b) 当たり判定、(c) `Preparing`／`JustStarted` の窓の一致（`find_owner_window`）、(d) 累積器 | (a) 窓の中の座標のまま、(b) 予備の枝へ、(c) 一致しない扱い＝`end_dragging()` を呼ばない、(d) `Dragging` なら積まずに `end_dragging()` |

- 4 つのキー・取り消し系は、`end_pos` に押した位置（`start_pos`）を入れている。要件 1.3 は「再入でない場合と同じ」なので、この値は変えない前提でよい。
- 離しの `end_pos` は「離した窓の中の座標＋`WindowPos.position`」で作る。World が無いと窓の中の座標がそのまま入る＝要件 1.3 の「画面の座標と取り違えない」に反する形になる。
- 失敗の枝はどれも記録を残さない（要件 6.1 の欠落）。`DragAccumulatorResource` の各メソッドも `Mutex` の毒化で黙って何もしない（要件 6 に隣接する小さな穴）。

### 1.3 慣習

- ドラッグの状態は `thread_local!` の 1 つだけ（単一ドラッグ）。`CaptureGuard` を `RefCell` の借用の外で落とす決まり（`end_dragging`・`cancel_dragging` の `_guard`）がある。
- Win32 の同期の送信を World の借用の外へ出す前例がある（`mouse_move.rs` の `deferred_set_window_pos`、`tick_bridge.rs` の借用の後の `flush_window_pos_commands`）。
- テストは兄弟ファイル（`keyboard_tests.rs`・`mouse_click_tests.rs`・`accumulator_tests.rs`・`state/tests.rs`）。`EcsWorld::new()` を作り、テストごとに新しい `DragAccumulatorResource` を `insert_resource` で入れ直し、`dispatch_window_message` を直に呼び、`dispatch_drag_events` を 1 回回して `Messages<DragEndEvent>` を `drain` で数える型。`DRAG_STATE` はテストの先頭で `cancel_dragging()` を呼んで掃く。`HWND` は空（null）。
- 1 ファイル 1,000 行以下・新しいテストは兄弟ファイル（`structure.md`）。記録の level は `logging.md`（回復できる縮退は `warn!`）。

---

## 2. 要件ごとの対応表（Missing／Unknown／Constraint）

| 要件 | 今の資産 | ギャップ |
|---|---|---|
| 1.1 4 種の取り消しで終了 1 回 | `keyboard.rs` の 4 ハンドラ・`set_transition` | **Missing**: World の無いときに累積器へ届く道が無い。**Constraint**: §0 のとおり、本番ではそもそもハンドラが呼ばれない |
| 1.2 離しで終了 1 回 | `handle_button_message` の予備の枝 | **Missing**: 同上 |
| 1.3 中身を同じに・画面の座標 | 取り消しは `start_pos`＝World 不要。離しは `WindowPos` が要る | **Missing**: 離しの画面の座標を World 無しで作る手段。**Unknown**: どの手段が再入でない場合と同じ値になるか（§5 調べること 2） |
| 1.4 遅らせない | 累積器は tick ごとに `flush`。`tick_wake::mark` は配送表の入口で World に依らず立つ | 累積器へ積めさえすれば、次の `dispatch_drag_events` で配られる＝満たせる見込み。**Unknown**: tick の途中で積んだ種が「その回」か「次の回」かの扱い（§5 調べること 3） |
| 1.5 ドラッグ中の印を外す | `dispatch_drag_events` の `Ended` の腕 | 1.1・1.2 が満たされれば既存の腕で外れる。ギャップ無し |
| 2.1 再入の後の動かさないクリックで終了 0 件 | `set_transition` の入口の判断 | 1.1・1.2 が満たされれば `current_dragging_entity` が置き去りにならず、満たされる。**Missing**（1.1・1.2 と同根） |
| 2.2 閾値前の取り消しで終了 0 件 | 入口の判断 | 今も満たす（`current_dragging_entity` が `None` なら捨てる）。修正の前後とも緑の見込み |
| 2.3 次のドラッグで開始→終了 1 回ずつ | 入口の判断・待ち行列 | 1.1・1.2 と同根 |
| 3.1 離しで状態を休ませ捕捉を解放 | `end_dragging` | **Missing**: `Preparing`／`JustStarted` の窓の一致を World 無しで調べる手段 |
| 3.2 違う窓の離しでは終えない | `Dragging` は `hwnd` の一致、他は `find_owner_window` | 今の判断を保つ。World 無しの一致の手段を足すなら、同じ答えになることが前提（§5 調べること 4） |
| 3.3 押していないのに始めない | `mouse_move.rs` はボタンを見ずに閾値と比べる | 3.1 で準備を残さなければ満たす。`mouse_move.rs` は境界の外（触らない） |
| 4.1〜4.3 areka の保存 | 受け手の既存テスト（`follow_drag_tests.rs`・`follow_drag_end_persist_tests.rs`） | 受け手は変えない。wintf の知らせのテストと合わせて確かめる |
| 5.1〜5.3 ふつうの条件は不変 | 既存テスト一式 | 置き場を変える案では、既存テストの組み立て（テストごとに累積器を入れ直す型）が変わりうる＝**Constraint** |
| 6.1 読めなかったことの記録 | 無し | **Missing**: 失敗の枝の記録 1 行（メッセージの種類・対象） |
| 7.1〜7.3 決定論テスト | 兄弟テストの型 | **Constraint**: §0。直に呼ぶか、`make_wndproc` を通すかで、赤の形と境界が変わる |
| 7.6 実機 | — | 再入は狙って起こせない（要件どおり）。ふつうの条件の確かめだけ |
| 7.7 完了 spec の上書きの記録 | `doc/COMPAT_ARCHITECTURE.md` §8 | 案によっては、完了 spec の「累積器は World の資源」という前提の書き換えが生じうる |

---

## 3. 実装の選択肢

どの案も、まず「累積器へ World 無しで届く道」と「離しの画面の座標・窓の一致を World 無しで得る道」の 2 つが要る。案の違いは、その道を**どこに 1 か所で**作るかである。

### 案 A: 累積器の複製を wndproc の側（`thread_local!`）にも持たせ、各ハンドラがそこへ積む

- `drag/` の中（`state/mod.rs` の `DRAG_STATE` と並べる、または `accumulator.rs`）に、累積器の複製を置く `thread_local!` と、登録と取り出しの関数を足す。`world/mod.rs` の `insert_resource` の 1 行の所で、同じ複製をそこへも登録する（`DragAccumulatorResource` は `Arc` を包む `Clone` なので、同じ実体を 2 か所から指すだけ）。
- 5 つのハンドラは、World の借用の有無に関わらず、`thread_local!` の側から累積器を取って積む。
- 利点: brief の案 1 そのもの。差分が小さい。累積器の役目（wndproc と ECS の間の箱）に合う。`dispatch.rs` の `flush` は World の資源のままでよい。
- 欠点: 積む場所が 5 つのハンドラに散ったまま（直すのも 5 か所）。テストはテストごとに新しい累積器を `insert_resource` で入れ直しているので、登録の 1 か所を通らない入れ方だと複製とずれる＝既存テストの組み立ての手直しが要る。1 スレッドに World が複数ある場合（テスト）に、どの World の累積器を指すかの決まりが要る。

### 案 B: 状態の移り変わりと種を積むことを 1 か所にまとめる（`end_dragging`／`cancel_dragging` が `Ended` を積む）

- 案 A の `thread_local!` の置き場を使ったうえで、`Ended` の種を積むのを、5 つのハンドラから、すべてのハンドラが通る `drag/state/mod.rs` の `end_dragging`・`cancel_dragging` の中へ移す。状態を `JustStarted`／`Dragging` から休む状態へ移す瞬間に、同じ所で `Ended` を積む。`Preparing` から移すときは積まない（今の入口の判断と同じ答え）。
- 「開始の知らせを配ったドラッグか」は `DragState` の側（`JustStarted`／`Dragging`）がすでに知っているので、状態と種がずれる形そのものが無くなる（状態の持ち方を変えて 0 フレームで解く・共有の 1 か所で直す、の制約に最もよく合う）。累積器の入口の判断（`set_transition` の `current_dragging_entity`）は二重の守りとして残せる。
- 利点: 直す所が 1 か所。再入の有無で道が分かれない。今後ハンドラが増えても置き去りが起きない。
- 欠点: brief は `drag/state/` の移り変わりを境界の外としている（要件の境界は「ドラッグの部分」を入れているので文面上は触れるが、移り変わりの関数の中身に手を入れる）。`end_dragging` の位置の引数を、今の離しの `end_pos`（画面の座標）と同じにする必要がある。既存テストの「`start_dragging` は累積器に何も積まない・テストでは `Started` を手で積む」という前提は保たれるが、`end_dragging`／`cancel_dragging` を直に呼ぶ既存テスト（`state/tests.rs`・各ハンドラのテストの掃除の `cancel_dragging()`）が累積器へ種を積むようになる＝掃除の手順の見直しが要る。

### 案 C: wndproc の入口（`make_wndproc`）まで含めて、本番の再入の道を直す

- §0 のとおり、本番では再入のメッセージがハンドラへ届かない。案 A／B だけだと、直るのは本番で通らない枝である。
- 入口のクロージャで World が借りられないとき、ドラッグに関わる 5 種（`WM_KEYDOWN` の ESC・`WM_CANCELMODE`・`WM_ACTIVATE` の非活性化・`WM_CAPTURECHANGED`・`WM_LBUTTONUP`）だけは、World を使わないドラッグの処理（案 A か B で World 無しに直したもの）を呼ぶ。他のメッセージは今どおり捨てる。
- 利点: 要件が想定した「再入でハンドラが World を使えない」という場面が本番の道になり、テストも `make_wndproc` を通して書ける（「檻は到達する経路を踏ませよ」に合う）。本番で実際に起こりうる「`WM_CAPTURECHANGED` が捨てられて状態が残る」穴も同時に塞がる。
- 欠点: `crates/wintf/src/runtime/wndproc_bridge.rs` は、要件の境界（`crates/wintf/src/ecs/` の下だけ）の外。境界を広げる判断が要る。棚卸㉑の照合では、C3 で wintf を触る他の spec は無い（brief の再測定の節）ので、並走の衝突は見当たらない。入口のクロージャの安全スキップの決まり（同ファイル冒頭の要件 4.3）と、そのテスト `closure_safe_skips_when_world_borrowed` の主張を一部書き換えることになる＝完了 spec の約束の上書きとして `doc/COMPAT_ARCHITECTURE.md` §8 への記録が要りうる。

### 案 D（参考・弱い）: 次の `start_preparing`、または配る段で、置き去りを片づける

- brief の案 2。`start_preparing` で累積器の `current_dragging_entity` を消す、または `dispatch_drag_events` で「状態は休んでいるのに累積器は対象を持つ」を見つけて `JustEnded` の中身（対象・位置・取り消しの印）から `Ended` を作る。
- 利点: 置き場を変えない。
- 欠点: 前者は要件 1（閾値を越えたドラッグの終了を落とさない）を満たさない。後者は、取り消しと次の押しが同じ画面更新の間に重なると `JustEnded` が上書きされて情報が消え、離しの位置も窓の中の座標のまま。どちらも要件を満たしきれないので、主案には向かない。

### 各案の比較

| | A | B | C（A か B と組む） | D |
|---|---|---|---|---|
| 要件 1・2 | 満たす | 満たす | 満たす | 一部だけ |
| 直す所 | 5 ハンドラ | 1 か所 | 1 か所＋入口 | 1 か所 |
| 本番の再入の道 | 直らない（通らない枝） | 直らない（通らない枝） | 直る | 直らない |
| 境界 | 内 | 内（`drag/state/` の中身に触れる） | 外へ広げる | 内 |
| 既存テストへの影響 | 累積器の入れ方の手直し | 掃除の手順の見直し | 入口のテストの主張の書き換え | 小 |

---

## 4. 規模と危険度

- **規模: S〜M**。案 A／B だけなら S（4〜7 タスク・既存の型の延長）。案 C を組むと、入口の振る舞いの変更とその檻の書き換えが加わり M 寄り。
- **危険度: 中**。技術は既知で差分も小さいが、§0 の前提の食い違いがあり、何を直すか（通らない枝か、本番の道か）を先に決めないと、テストが本番の道を踏まない「緑の檻」になる危険がある。

---

## 5. 調べること（設計へ持ち越す）

1. **本番で再入の同期メッセージが本当にハンドラへ届かないことの確証**: `make_wndproc` 以外に `dispatch_window_message` へ入る本番の道が無いこと（今の grep では無い）、`create_windows` など tick の中の Win32 呼び出しで、ドラッグ中の窓へ `WM_ACTIVATE`／`WM_CAPTURECHANGED`／`WM_CANCELMODE` が同期で届きうる道の洗い出し。
2. **離しの画面の座標を World 無しで得る手段**: 候補は (a) `ClientToScreen(hwnd, …)`（本番では正確・テストの空の `HWND` では失敗する＝その時の縮退と記録が要る）、(b) `DragState::Dragging` の `current_pos`（最後の移動の画面の座標・離した点と違いうる・`Preparing`／`JustStarted` には無い）、(c) 窓の画面上の原点を `thread_local!` 側にも写しておく。どれが「再入でない場合と同じ値」（要件 1.3）になるか。決定論のテストで本物の窓（`api.rs`・`win_style.rs` のテストのように隠れた窓を作る型）を使うかどうか。
3. **tick の途中で積んだ種の配られる回**: 再入の種が、その tick の `dispatch_drag_events` より後に積まれた場合に次の tick が必ず回るか（`dispatch_window_message` の入口の `tick_wake::mark` と、`rearm_tick_while_dragging` の `DRAG` の旗で足りるか）。要件 1.4・7.3 の「同じ回」の定義をテストでどう表すか。
4. **窓の一致を World 無しで調べる手段**: `CaptureGuard` が持つ `hwnd`（押した窓）と離した窓の `hwnd` を比べる形が、今の `find_owner_window(entity) == Some(window_entity)` と同じ答えになるか（ドラッグの設定を持つ祖先が窓の外に出ることが無いか）。同じ答えなら、`Dragging` と同じ「`hwnd` の一致」へ揃えられ、ふつうの条件の判断の根拠が変わる（要件 5 との関係）。
5. **1 スレッドに World が複数あるとき（テスト）の `thread_local!` の扱い**: 登録を `EcsWorld::new` の 1 か所に限るか、テストの累積器の入れ方を登録の関数へ寄せるか。`DRAG_STATE` が既に 1 スレッド 1 つの前提で動いていることとの整合。
6. **記録の level と文言**: 要件 6.1 の 1 行を `warn!`（回復できる縮退）にするか `debug!` にするか。案 C を取ると本番で実際に出る行になるので、頻度の見積もりも要る。

---

## 6. 議題の候補（要件ディスカッションへ）

1. **何を直すか（§0）**: 本番では tick の再入のメッセージは `make_wndproc` の入口で捨てられ、要件の想定（ハンドラが World 無しで動く）は起きない。(a) 要件の文面どおり、通らない枝を守りとして直し、テストは `dispatch_window_message` を直に呼ぶ、(b) 境界を `runtime/wndproc_bridge.rs` まで広げ、ドラッグに関わる 5 種だけ入口を通し、本番の道として直してテストも入口から踏む（案 C）、(c) spec の筋を「再入でメッセージが捨てられ状態が残る」へ書き直す、のどれを取るか。答えで境界・テスト・要件の本文（Introduction の症状の記述と Boundary Context）が変わる。
2. **種を積む所を 1 か所にまとめるか（案 A と案 B）**: 5 つのハンドラに残すか、`end_dragging`／`cancel_dragging` へ移すか。後者は brief の「`drag/state/` は境界の外」と食い違う。
3. **離しの画面の座標の出どころ**（§5 調べること 2）: `ClientToScreen`・最後の移動の位置・写しの原点のどれにするか。要件 1.3 の「同じ中身」の判定に直結する。
4. **窓の一致の判断を `hwnd` の一致へ揃えるか**（§5 調べること 4）: 揃えると World 無しで判断でき、ふつうの条件でも判断の根拠が変わる（答えは同じ見込み）。
5. **完了 spec の約束の上書きの記録**: 案 C なら入口の安全スキップ（`wndproc_bridge.rs` の要件 4.3）、案 B なら「累積器へ積むのはハンドラ」という前提が変わる。`doc/COMPAT_ARCHITECTURE.md` §8 への記録の要否。

---

## 7. 設計への申し送り（推奨の見立て・決定ではない）

- 制約（0 フレーム・共有の 1 か所で直す・黙って捨てない・決定論のテスト）に最もよく合うのは、**案 B（状態の移り変わりの所で種を積む）に、議題 1 の答えしだいで案 C（入口）を組む形**。
- 議題 1 を (a) で閉じる場合でも、テストが本番の道を踏まないことを設計書に明記し、入口のクロージャの安全スキップが本番の再入の実際の振る舞いであることを記録に残す必要がある。
- 要件 6.1 の記録は、World 無しの道を通るたびに 1 行（メッセージの種類・対象）を出す 1 か所に集める。`DragAccumulatorResource` の `Mutex` の毒化で黙って何もしない形も、同じ機会に記録へ寄せるかを設計で決める。

---

## 8. 設計フェーズの調査と決定（2026-10-04 `/kiro-spec-design`）

> 要件番号は改訂後の `requirements.md` の番号（本節より上の §0〜§7 の番号より 1 つ大きい）。調査の種類は「既存の仕組みの拡張」（light）。外部の新しい依存は 0 件。

### 8.1 まとめ

- **Discovery Scope**: Extension（wintf の入口・ドラッグの状態・累積器）。
- **Key Findings**:
  - 本番で `dispatch_window_message` を呼ぶのは `make_wndproc` のクロージャの 1 か所だけ。再入のメッセージは今は全部ここで捨てられる。
  - tick の中でメッセージを汲む呼び出しは 0 件。再入で届くのは同期のメッセージ（非活性化・捕捉の喪失・メニューやダイアログの割り込み）だけで、ESC キーと左ボタンを離すメッセージが再入で届く道は今は無い。
  - 「開始の知らせを配ったドラッグか」はドラッグの状態（`JustStarted`／`Dragging`）が知っている。状態を休ませる関数の中で終了の種を積めば、状態と種がずれる形が無くなる。

### 8.2 調査の記録

#### ESC キー・左ボタンを離すメッセージは再入で届くか（§5 調べること 1）

- **調べた所**: `TrackPopupMenuEx`・`MessageBoxW`・`PeekMessageW`／`DispatchMessageW`・`DoDragDrop`・`DialogBox` の呼び出しを `crates/wintf/src`・`crates/areka/src` で洗った。
- **結果**:
  - `TrackPopupMenuEx`（`areka/src/menu/win32.rs` の `show`）: `menu/trigger.rs` の `show_task`（tick の外のタスク）から呼ぶ。`display` の説明に「外側の World には触れない」とある。メニューの表示中に回る tick は、自分の回の間だけ World を借りる。
  - `MessageBoxW`（`areka/src/alert.rs` の `raise`・`ask_yes_no`）: `raise` は `main.rs` の起動の失敗と終了の後始末（tick の外）、`ask_yes_no` はインストールの背景スレッド（`install/worker.rs`）。UI スレッドの tick の中では 0 件。
  - `PeekMessageW` の汲み出し（`wintf/src/com/wuc.rs` の `pump_current_thread_messages`）: `drain_dispatcher_queue` の中だけ。tick の中からの呼び出しは 0 件。
  - `DoDragDrop`・`DialogBox`: 0 件。
- **意味**: 待ち行列を通る 2 種は、今のコードでは再入で届かない。要件どおり 5 種とも扱いを揃えるが、テストで作る再入（World を借りたまま入口へ渡す）だけがこの 2 種の再入の証拠になる。tick の中へ汲む呼び出しを足すと届くようになるので、設計書の Revalidation Triggers に入れた。

#### 同期の 3 種が再入で届く道

- `UISetup` の `create_windows` → `runtime/window_factory.rs` の `create_window`（`Window::new_ex` と `ShowWindow(hwnd, SW_SHOW)`）。新しい窓の活性化で、ドラッグ中の窓へ `WM_ACTIVATE`（非活性化）・`WM_CAPTURECHANGED` が同期で届きうる。
- `handle_button_message` の当たり判定の枝が `try_borrow_mut()` を握ったまま `end_dragging` を呼び、`CaptureGuard` の `ReleaseCapture` が同期で `WM_CAPTURECHANGED` を送る。これは左クリックのたびに起こる。状態はすでに休んでいるので、再入の扱いは何もしない（記録は `debug!`）。
- 窓の位置の書き込み（`SetWindowPos`）は `tick_one_frame_with` が借用を返した後の `flush_window_pos_commands` で行うので、再入の元にならない。

#### tick の途中で積んだ種が配られる回（§5 調べること 3・要件 2.4）

- `EcsWorld::try_tick_world` は 13 本を `Input` → `Update` → … → `UISetup` → … → `FrameFinalize` の順に回す。`dispatch_drag_events` は `Input`（1 本目）。
- 再入の種は「積んだ後に最初に回る `dispatch_drag_events`」で配られる。`Input` より後（たとえば `UISetup`）で積んだ種は次の tick。これは、その tick の直後に再入でなく届いたメッセージと同じ回。`Input` の中で配る段より前に積んだ種はその tick で配られる（遅れは 0）。
- 次の tick が回る保証: (1) 再入の扱いが `tick_wake::mark` を立てる（今は捨てられたメッセージは旗を立てない。配送表の入口と同じ式を使う）。(2) `FrameFinalize` の `rearm_tick_while_dragging` が、`DraggingState` が残っている間 `DRAG` の旗を立てる。
- 取らなかった案: メッセージを覚えておいて借用が空いてからハンドラへ渡し直す案。ドラッグの状態が休むのが遅れ、その間の移動で窓が動くので取らない（1 フレーム遅らせる解に当たる）。

#### ドラッグ中の印の外し方（要件 2.5）

- `DraggingState`（対象）と `WindowDragging`（対象を含む窓）は、`dispatch_drag_events` の `Ended` の腕が外す。再入でない場合もメッセージの時点では外さず、次の配る段で外す。
- 再入でも終了の種さえ積めれば、同じ腕が同じ回に外す。メッセージの時点で World が要る処理は 0 件。印を読むのは areka の `emo2_boot/balloon_visibility_phase.rs`（`WindowDragging`）と `placement/follow/drag_follow.rs`（`DraggingState`）で、どちらも tick の中のシステム。

#### 離しの画面の座標（§5 調べること 2・要件 2.3）

- ふつうの道: `handle_button_message` の冒頭で「窓の中の座標＋`WindowPos.position`」。`WindowPos.position` はクライアント領域の左上の画面の座標（`window_handle.rs` の `client_to_window_coords` の引数の説明、`dispatch.rs` の `Started` の腕の注記）。
- `ClientToScreen(hwnd, 窓の中の座標)` は同じ値を OS から直に得る。`windows` クレートの機能 `Win32_Graphics_Gdi` は有効済み（`ScreenToClient` を `clickthrough/controller.rs` などが使っている）。
- 失敗するとき（テストの空の `hwnd`・壊れた窓）: 状態が持つ最後の画面の座標で終え、`warn!` を出す（要件 7.2）。
- 取らなかった案: (b) 状態の `current_pos` を主に使う＝離した点と違いうる。(c) 窓の原点を `thread_local!` へ写しておく＝写しを保つ所が増え、再入で `WM_WINDOWPOSCHANGED` が捨てられると古くなる。

#### 窓の一致（§5 調べること 4・要件 4.2）

- 今: `Dragging` は状態の `hwnd`（配る段が、対象を含む窓の `WindowHandle` から `WindowDragContextResource` 経由で写す）と離した窓の `hwnd` の一致。`Preparing`／`JustStarted` は `find_owner_window(対象) == 離した窓の entity`。
- 揃えた後: 3 つの状態とも「離した窓の `hwnd` == `CaptureGuard` が持つ `hwnd`（`start_preparing` へ渡った、押しを受けた窓）」。
- 同じ答えになる条件: 対象を含む窓が押した窓であること。`DragConfig` を付けている所は areka の `placement/spawn.rs`（キャラクターの窓・バルーンの窓の entity そのもの）と wintf の例 `taffy_flex_demo/setup.rs`（窓かその中の部品）で、どちらも満たす。満たさない使い方は 0 件。
- 完了 spec の `tests/window/multiwindow_event_test.rs` の `test_drag_hwnd_guard_owner_window_check` は `find_owner_window` の答えだけを見ており、緑のまま。説明文がハンドラの判断として `find_owner_window` を挙げているので、説明文だけ直す。

#### 1 スレッドに World が複数あるとき（§5 調べること 5）

- wndproc 側の控えは `thread_local!` に 1 つ。`install_drag_accumulator` が World の資源と控えへ同じ実体を置く。`EcsWorld::new` が呼ぶ。
- `cargo test` は 1 本 1 スレッドなので、テスト同士は混ざらない。1 本の中で World を 2 つ作ると、後の方が控えになる。
- `keyboard_tests.rs`・`mouse_click_tests.rs` は `EcsWorld::new()` の後に `insert_resource(DragAccumulatorResource::new())` で入れ直している。入れ直すと控えとずれるので、資源の複製を使う形へ手直しする。素の `World` へ累積器を入れて `set_transition` を直に呼ぶテスト（`wintf/tests/drag/dispatch_test.rs`・`tests/layout/…/drag_lifecycle.rs`・areka の `follow_drag_end_gate_tests.rs`）は、`end_dragging`・`cancel_dragging` で種を積むことに頼っていないので変更 0 行。
- `areka/src/menu/trigger_flow_tests.rs` は `Preparing` から `end_dragging` を呼ぶだけ（種は積まれない）。変更 0 行。

#### 新しく生まれる危険: `DRAG_STATE` の入れ子の借用

- `start_preparing` は `update_drag_state` のクロージャの中（`DRAG_STATE` を可変で借りたまま）で `CaptureGuard::acquire` → `SetCapture` を呼ぶ。`SetCapture` は前の捕捉の持ち主へ `WM_CAPTURECHANGED` を同期で送る。
- 今は、この呼び出しが `handle_button_message` の World の可変の借用の中にあるので、入れ子のメッセージは入口で捨てられる。本 spec の後は入口を通り、`update_drag_state` が二重の借用で panic しうる。
- 前の持ち主が自分のスレッドの窓であるのは、`SetCapture` を呼ぶ所が `capture_guard.rs` の 1 か所だけなので、押している状態（＝ `start_preparing` が先に戻る）に限られ、実際には起こらない見込み。それでも構造で塞ぐ: 捕捉を取るのを借用の外へ出す。

### 8.3 設計の決定

#### Decision: 終了の種は、状態を休ませる関数の中で積む（案 B）

- **Alternatives**: (A) 5 つのハンドラが控えから累積器を取って積む。(B) `end_dragging`・`cancel_dragging` が積む。
- **Selected**: (B)。ハンドラの 6 か所（`keyboard.rs` 4・`mouse_click.rs` 2）は消す。
- **Rationale**: どの道もこの 2 関数を通る。開始を配ったかは状態が知っている。ふつうの道と再入の道が同じ関数を通るので、片方だけ直し忘れる形が無い。
- **Trade-offs**: brief は `drag/state/` を境界の外としていたが、要件の境界（「ドラッグの部分」）には入る。`mouse_move.rs` の開始の種は境界の外なので動かさず、累積器の入口の判断を二重の守りとして残す。
- **Follow-up**: 閾値前の取り消しで出ていた累積器の `debug!`「Ended without Started dropped」は出なくなる（見ているテストは 0 件）。

#### Decision: 累積器へ届く道は `thread_local!` の控え

- **Alternatives**: (1) `thread_local!` の控え。(2) `WndState` に累積器の複製を持たせて入口から渡す。(3) 累積器そのものを `thread_local!` にする。
- **Selected**: (1)。
- **Rationale**: (2) は `runtime/window_factory.rs`（境界の外）を触り、`end_dragging` などの引数が増える。(3) は累積器が `Send`/`Sync` の資源である前提と、素の `World` へ累積器を入れる他クレートのテストを壊す。(1) は `EcsWorld::new` の 1 行で済み、`DRAG_STATE` と同じ「1 スレッドに 1 つ」。

#### Decision: 再入の扱いは `ecs/drag/reentry.rs` に置く

- **Rationale**: `ecs/window_proc` は `ecs` の中だけに見える。`ecs/drag` は公開の module なので、`ecs/mod.rs`・`window_proc/mod.rs` を触らずに入口から呼べる。

#### Decision: 記録の level

- 終えた・取り消した: `warn!`（まれ・ドラッグ以外の処理を飛ばしている）。何もしなかった: `debug!`（左クリックのたびに通る）。読めなかった情報・控えが無い・`Mutex` の毒化: `warn!`。

#### Decision: `doc/COMPAT_ARCHITECTURE.md` §8 へ 1 行

- 完了 spec `wintf-winmsg-executor` 要件 4.3 の「再入は安全スキップ」を、5 種についてだけ「World を使わないドラッグの扱いをしてから既定の手続きへ委ねる」へ変えた、と記す。デッドロックと二重の画面更新を起こさない保証は保つ。`drag-click-without-move`・`event-drag-system` の要件（知らせの約束）は上書きしない（種を積む所は要件ではなく作り）。

### 8.4 まとめ直し（synthesis）

- **一般化**: 5 種の扱いは「取り消す」「捕捉を失って取り消す」「離して終える」の 3 つの動きに畳める。ふつうの道と再入の道は同じ 3 つの関数を通る。
- **作るか借りるか**: 座標は OS の `ClientToScreen` を使う（自前の写しを作らない）。
- **削ったもの**: 座標の出どころを差し替える口・入口でのメッセージの溜め直し・`WndState` の欄の追加・`mouse_move.rs` の変更。どれも要件に要らない。

### 8.5 危険と手当て

- 既存テストが累積器を入れ直していて控えとずれる → 手直しの対象を設計書に列挙した。ずれると終了の知らせが 0 件になり赤で気付ける。
- `ClientToScreen` を確かめるテストは実物の窓が要る → `api.rs`・`clickthrough/controller_tests.rs` と同じ「隠れた `Static` の窓」の作り方を使う。
- 左クリックのたびに再入の記録が出る → 何もしなかったときは `debug!`。
