# ギャップ分析: mouse-click-wheel-events

- 日付: 2026-10-23（作業枝 `claude/kiro-start-mouse-click-wheel-events-b168db` の `179d6193`・ワークツリー `areka-p0-offline-surface-dump-0006ba`）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`roadmap.md` の C11 の表と並び順の決まり・`logging.md` の水準の表）・コード（下の各節）・ukadoc MCP・完了 `areka-P0-pasta-in-windows-verification` の SSP の測りの記録（`ssp-measurement.md` と、git の外の `areka-p0-pasta-windows-d4ed7a\target\pasta-verify\ssp\`）
- やり方: kiro-validate-gap（`rules/gap-analysis.md` の枠）。決めるのは設計と要件討議で、ここは材料と案だけを出す。SSP と画面は動かしていない。

## 0. 要約

- **kanade の側は前例どおりに足せる**。`OnMouseUp` → `OnMouseClick` の 2 段は、アンカーの `OnAnchorSelectEx` → `OnAnchorSelect`（`schedule/anchor.rs`・段の記憶 1 つ）と同じ形で新しいファイルに置ける。殻（`actor.rs`）は GET の応答を次の知らせより先に入れ直すので、要件 4.2（2 段の間に別の操作を挟まない）は今の作りで構造的に守られる。足すのは `MouseEventKind` の腕・送ってよい表の 7 語・組み立て 7 つ・`State` の欄 1 つ・失敗の免除の条件 1 つ。
- **brief の「wintf は触らない見込み」は外れる**。⑴ wintf はホイールの量を窓ごとの溜め（`WHEEL_BUFFERS`）に積むだけで、`PointerState.wheel` へ移す処理が無い（製品のどこでも `wheel` は 0 のまま・溜めは捨てられもしない）。⑵ wintf の押下の配りは左・右・中だけで、拡張ボタン 1・2 の押下は `OnPointerPressed` に届かない。⑶ wintf の `OnPointerEntered`／`OnPointerExited` は型だけで配られない（窓の外へ出たことは `PointerLeave` の印で読む＝バルーンの前例 `balloon_exit.rs`）。
- **右クリックのメニューとの合わせ目は `menu/` に手が入る**。キャラ窓の `OnPointerReleased` の部品は 1 窓に 1 つで、今はメニューの `trigger::on_char_pointer_released` が持っている（`menu/mod.rs` の `attach_release_handlers`）。クリックの解放を受けるには受け手を 1 本にまとめる必要があり、要件 5.2（抑止のときだけ右のクリックを送る）は `trigger.rs` の `poll_once`／`decide` の決着に「預かった右クリック」を足す形になる（右ダブルクリックの預かり `PendingDoubleClick` と同じ形）。C11 の約束「`menu/trigger.rs` は議題 3 で要ると決まったときだけ」は、議題 3 の推奨を採った時点で「要る」に当たる。
- **案は 3 つ**（4 節）。推しの材料が多いのは C（kanade は新しいファイル＋入口の数行・入力は新しいファイル群＋`mod.rs` の結線・wintf は溜めの移しと押下の配りの最小の 2 か所）。規模は M〜L（要件 12 本・タスク 14〜18 の見込み＝brief の 13〜16 より少し上）。危険は中（wintf の 2 か所と、同じ tick の中のドラッグの開始と解放の順）。
- **ukadoc で決まらない 5 点は、ukadoc でも手元の記録でも決まらなかった**（5 節）。手元の SSP の測りの記録（ぱすたの `pasta.log`）は 200 の応答の要求しか残さないので、`OnMouseUp`（ぱすたは 204）が来たかどうかの証拠にならない。在席の机で SSP の `ssp_shiori_log.txt` を使って測る必要がある。

## 1. 今ある物の調べ

### 1.1 kanade（`crates/areka-kanade/`）

| 物 | 在りか | 今の形 | 本 spec との関わり |
| --- | --- | --- | --- |
| マウスの知らせの型 | `src/msg.rs`（544 行）の `MouseInput`・`MouseEventKind`（125 行目〜）・`MouseButton` | 種類は `Move`・`DoubleClick{button}`・`DragStart`・`DragEnd` の 4 つ。`MouseButton` は `Left`／`Right` だけ | 腕を足す（クリック・ホイール・出入り）。中・拡張 2 つのボタンを運ぶ型が要る（下の 3.7） |
| 殻の写し | `src/actor.rs` 165 行目 `KanadeMsg::Mouse(m) => Input::Mouse(m)` | 中身を見ずに写すだけ | **触らずに済む**（C11 の約束「`actor.rs` に触らない」を守れる） |
| 定常での受理 | `schedule/steady.rs`（957 行・残り 43）の `on_mouse` | 閉じの握手の保留中は送らない（trace）。種類ごとに組み立てを 1 つ呼んで GET を 1 本積む | 腕を足す。2 段の判断は新しいファイルへ（行数の余裕が無い） |
| 定常以外の棄却 | `schedule/mod.rs`（673 行）188 行目 | `mouse_input_ignored` を trace で残して捨てる | 要件 8.3 の「受け付けない時期」はこのまま使える（記録の水準は 9 の議題） |
| 応答の腕 | `steady.rs` の `on_reply`（753 行目〜） | 先頭でアンカーの段を取り出し、台本なら今までの腕へ流す。台本は `value_replaces_active_talk(origin)`（`OnSecondChange` 以外は置き換え）で話を差し替える | 2 段の段の記憶を同じ位置で取り出す（入口の数行）。要件 8.1 の置き換えは今の述語のままで満たす |
| 失敗の扱い | `schedule/mod.rs` の `on_shiori_reply`（497 行目〜） | 定常の応答待ちの `Failed` は横断で `Unloading{Fault}`（ゴーストが落ちる）。例外は「選択肢の in-flight」と「アンカーの段がある間」（533 行目の条件） | 要件 1.6（`OnMouseUp` の失敗は 204 と同じに進む）は、この免除の条件に「マウスの段がある間」を足す形。**今のマウスのイベントの失敗は終了の系列へ倒れる**ので、2 段目（`OnMouseClick`）の失敗をどちらに倒すかは決まっていない（3.5） |
| 2 段の前例 | `schedule/anchor.rs`（142 行）・`schedule/choice.rs` の `CascadePlan::Canonical` | アンカーは帳簿も期限も持たず、`State.anchor: Option<AnchorStage>`（`types.rs` 222 行目）に「どの段の応答を待っているか」だけを置く。殻が応答をすぐ入れ直すので、段の記憶は 1 回の知らせの処理の中だけで生きる | `OnMouseUp` → `OnMouseClick` はこの形に倣える（`State` の欄は `types.rs`＝390 行に足す。brief の「`schedule/mod.rs` の状態の欄」は今は `types.rs`） |
| 組み立て | `schedule/events.rs`（888 行・残り 112）の `on_mouse_move`・`on_mouse_double_click`・`mouse_drag_get` | 7 個の Reference を固定の並びで作る純粋関数。`REF2_WHEEL_M1`＝"0"・`REF6_DEVICE_MOUSE`＝"mouse" | 7 つ足す。`mouse_drag_get` の形（名前だけ違う）に寄せれば 1 つの共通関数＋薄い 7 つで 40〜60 行に収まる見込み |
| 送ってよい表 | `events.rs` 115 行目 `ALLOWED_EVENT_IDS`（55 語） | 語ごとに ukadoc の URL のコメント | 7 語を足して 62 語。**手書きの数** `events_change_tests.rs` 43 行目 `assert_eq!(ALLOWED_EVENT_IDS.len(), 55)` を直す（後のウェーブの `balloon-communicatebox`・`shell-scale-linkage` も同じ数を動かす＝記憶「両枝が同じ誤った手書きの数を書くと git は黙って通す」） |
| 出どころ | `src/request_origin.rs` の `of_event` | `EventId::Static` は全部「ふつうのイベント」 | 足すものは何もしなくてよい |

### 1.2 入力の配線（`crates/areka/src/input_events/`・`menu/`）

| 物 | 在りか | 今の形 | 本 spec との関わり |
| --- | --- | --- | --- |
| 結線の資源 | `input_events/mod.rs`（576 行）の `MouseWiring`（NonSend） | 投函の端・窓ごとの移動の間引き・当たり判定の供給（実／模型の差し替え）・注入できる時計 `now_ms`・右ダブルクリックの預かり 1 件 | 窓ごとの「押下の控え」「ホイールの溜め」「今の当たり判定」を足す場所。時計の注入があるので要件 11.2 の時刻を注入するテストはこの形で書ける |
| 受け手の装着 | `attach_char_pointer_handlers` | キャラ窓に `OnPointerMoved`・`OnPointerPressed`・`OnDragStart`・`OnDragEnd` | `OnPointerReleased` は**ここでは付けていない**（メニューが付ける・下） |
| 移動 | `on_char_pointer_moved` | 箱の選択肢の行の上なら送らずに返る → 当たり判定を解く → 間引き（`throttle.rs`）→ `Move` | 出入り（要件 7）とホイール（要件 6）の入口に向く。wintf は `OnPointerMoved` を**窓に `PointerState` がある間は毎 tick 配る**（`dispatch_pointer_events` の「常に発火」）ので、毎 tick の状態（`wheel`・拡張ボタンの押下）をここで読める |
| 押下 | `on_char_pointer_pressed`（490 行目〜） | 強制退避 → 箱の前段（`on_box_pointer_pressed` が真なら返る）→ 左ダブルクリックは送る・右ダブルクリックは預かる・中／拡張のダブルクリックと単発は `false` で何もしない（526〜535 行目） | 単発の押下を「控える」腕が要る（要件 3.1〜3.4 の判定の材料）。箱の前段で消費された押下も控えに印を残す必要がある（3.4） |
| 移動の間引き | `throttle.rs`（276 行）の `plan_mouse_move` | 位置が変わり、かつ（初回・当たり判定の変化・100 ms 経過）なら送る。純関数＋状態は値型 | 当たり判定が変わった動きは間引かれずに送られる＝要件 7.6・7.7 と合う。ホイールの間引きは同じ考え方の別の純関数（`MOUSE_MOVE_MIN_INTERVAL_MS`＝100 を共有できる） |
| ドラッグ | `drag.rs`（181 行）の `on_char_drag_start`／`on_char_drag_end_and_notify` | 開始・終了を `MouseEventKind::DragStart`／`DragEnd` で送る。「開始を送った」印は持たない（設計 D2） | 要件 3.1 の「ドラッグになった押下」を知る材料。開始の受け手で押下の控えに印を付けるのが素直（下の 3.4 の順の問題あり） |
| 右クリックのメニュー | `menu/trigger.rs`（573 行）の `on_char_pointer_released` → `handle_release` | 右ボタンの解放だけを見る。ドラッグ中・結線前は預かった右ダブルクリックを捨てる。メニューの返事待ち（`popupmenu.visible` などの照会）を預け、毎 tick の `poll_menu_query` → `poll_once` → `decide(visibility, pending)` で「出す／抑止（預かりの右ダブルクリックを送る）」を決める | 要件 5.2 は「抑止のときだけ右のクリックを送る」＝`decide` の材料に「預かった右クリック」を足す形（右ダブルクリックと同じ）。要件 5.3（位置は放した時点）は預かりに放した時点の値を載せれば満たす |
| 解放の受け手の装着 | `menu/mod.rs` 280〜295 行目 `attach_release_handlers` | キャラ窓に `OnPointerReleased(trigger::on_char_pointer_released)` を入れる | **同じ型の部品は 1 窓に 1 つ**（`drag.rs` の `OnDragEnd` の包みと同じ事情）。クリックの解放を受けるには、ここを「入力のクリック＋メニュー」の包みに替えるか、メニューの受け手の中からクリックを呼ぶかのどちらか（3.3） |
| 窓の外へ出た | `balloon_exit.rs` の `clear_balloon_hover_on_leave`（排他システム） | wintf が配らない `OnPointerExited` の代わりに、`PointerLeave` の印を持つ entity を集め、`find_owner_window` で持ち主の窓を引いて処理する。`register_balloon_leave_system`（`ghost_session.rs` 75 行目から 1 回）で `dispatch_pointer_events` の後に登録 | 要件 7.4（窓の外へ出たら Leave）の前例。キャラ窓用に同じ形のシステムを足す |

### 1.3 wintf（`crates/wintf/src/ecs/pointer/`・`window_proc/`）

| 物 | 在りか | 今の形 | 本 spec との関わり |
| --- | --- | --- | --- |
| ホイールの受け | `window_proc/mouse_dblclick_wheel.rs` の `WM_MOUSEWHEEL`／`WM_MOUSEHWHEEL` | `wParam` の上位 16 ビット（符号つき・1 目盛り 120）を**窓の entity** の `WHEEL_BUFFERS` に足す。`lParam`（画面座標）は捨てる | 要件 6.2 の「1 目盛り＝120・上が正」は OS の値そのまま |
| ホイールの移し | `pointer/buffers.rs` の `transfer_buffers_to_world` | 位置・ボタン・修飾キーは `PointerState` へ移すが、**`WHEEL_BUFFERS` を移す処理も空にする処理も無い**（`grep` で読み手 0）。`PointerState.wheel` は既定値のまま毎 tick 0 に戻される（`systems.rs` 25 行目） | **Missing**。brief の「`PointerState.wheel` に載せる」は誤り。wintf に移しを足さない限り、areka はホイールを一切読めない。移す先は「その窓の中で `PointerState` を持つ entity」（`PointerState` は当たった子の entity に付く＝窓の entity とは限らない） |
| 押下の配り | `pointer/dispatch/mod.rs` の `dispatch_pointer_events` | `OnPointerPressed` は `left_down || right_down || middle_down` のときだけ配る。拡張ボタンの押下（`xbutton1_down` など）は立つが Pressed は配られない。`OnPointerReleased` は `released.any()`（拡張ボタンを含む）で配る。同じ tick の押下＋解放は Pressed → Released の順 | 要件 2.1 の拡張ボタンのクリックは、押下を⑴ wintf の条件に拡張ボタンを足すか、⑵ areka の移動の受け手で `xbutton*_down` を読むかで拾う（3.2） |
| 解放の旗 | `buffers.rs` 186〜200 行目 | 解放を受けた entity に `PointerState` があるときだけ `released.*` を立てる。左の押下はドラッグの準備で**捕捉（capture）**を取る（`drag::start_preparing`）ので、左は窓の外で放しても押した窓に解放が届く。右・中・拡張は捕捉を取らない | 要件 3.3（別の窓・窓の外で放したらクリックにしない）の判定材料。左は「放した位置が窓の中か（当たり判定の解決）」、右・中・拡張は「押下の控えがその窓にあるか」で見分ける形になる |
| ダブルクリック | `mouse_dblclick_wheel.rs` の `handle_double_click_message` | OS の `WM_*BUTTONDBLCLK` を受け、`double_click` を 1 tick だけ立て、押下も記録する（2 回目の押下は `DoubleClick::Left` などつきの Pressed で届く）。窓の型は `CS_DBLCLKS` を内蔵 | 要件 3.2（2 回目の押下の解放は送らない）は、押下の控えに「ダブルクリックの押下だった」印を残せば判定できる。areka に時間を待つ仕掛けは要らない（要件 1.5 と合う） |
| ドラッグの状態 | `drag/state/mod.rs` | 押している間は `Preparing`／`JustStarted`／`Dragging`。解放で `JustEnded{entity, position, cancelled}` へ。**`JustEnded` は「閾値を越えたか（開始を配ったか）」を残さない**（`close_drag` の中の `started` は外に出ない） | 解放の時点の状態からは「ドラッグになったか」が読めない。areka 側で開始の受け手に印を付けるか、wintf の `JustEnded` に `started` を足すか（3.4） |
| 配りの順 | `ecs/world/mod.rs` 196〜208 行目 | `dispatch_drag_events` は `dispatch_pointer_events` の**後** | 同じ tick の中で閾値越え（開始）と解放が起きると、解放の受け手が先に走り、ドラッグの開始の印がまだ無い（3.4） |
| 出入り | `dispatch/mod.rs` の `OnPointerEntered`／`OnPointerExited` | 型だけで配られない。窓を出た（`WM_MOUSELEAVE`）ときと、同じ窓の中で `PointerState` が別の entity へ移ったときの**どちらでも** `PointerLeave` の印が付く（`window_proc/mouse_move.rs` 327〜399 行目） | 要件 7.4 の「窓の外へ出た」を読むには、`PointerLeave` を見たうえで「その窓に `PointerState` がまだ残っているか」で見分ける必要がある（3.6） |
| tick の起こし | `ecs/world/tick_wake.rs` | tick は旗（`POINTER` ほか）か期限（`arm_deadline(at)`）で起きる。何も無いと回らない | 要件 6.5（間隔が明けた時点で次の回転を待たずに送る）は、溜めを残したら `tick_wake::arm_deadline(前の送出＋100 ms)` を預け、毎 tick の系で吐き出す形になる。前例: `emo2_boot/balloon_visibility_phase.rs` 118 行目 |

### 1.4 網羅台帳・文書・テスト

- `doc/ukadoc-coverage/ledger/shiori.toml`: 7 行とも `status = "absent"`。`owner` が本 spec なのは `OnMouseClick`（3661 行目）だけで、ほかの 6 行（`OnMouseClickEx` 3686・`OnMouseEnter` 3843・`OnMouseLeave` 3938・`OnMouseUp` 4046・`OnMouseUpEx` 4068・`OnMouseWheel` 4090）は空。書き直す形は `OnMouseMove`（3983 行目・`implemented`・構築関数の在りかを書く）に倣う。`OnMouseWheel` の `introduced = "2.3.53"` と版番号の注記は今のまま。
- `doc/COMPAT_ARCHITECTURE.md` §8（122 行目「沈黙ルール対応表」）: 末尾に足す。
- `doc/pasta-real-run.md` の段 3（183 行目）・段 7（187 行目）: 確かめの表に本 spec の名前が既に書いてある。
- 今あるテストで本 spec が触れる物: `input_events_tests.rs` 398 行目 `handler_middle_xbutton_and_single_click_do_not_send`（押下の受け手は単発・中・拡張で何も送らず `false` を返す）・同 640〜669 行目の送出集合の檻。クリックを解放の時点で送るなら「押下では送らない」は変わらないが、押下を控えるために受け手が `true` を返すように変えると、`false` を見る檻は崩れる（要件 11.4「変えずに通る」との関係＝3.8）。

## 2. 要件ごとの資産の対応

印: **有**＝今ある物で足りる／**足**＝今ある形に足す／**Missing**＝無い／**Unknown**＝調べが要る／**Constraint**＝今の作りの縛り。

| 要件 | 今ある物 | 足りない物・縛り |
| --- | --- | --- |
| 1.1〜1.4 左右の 1 回のクリックの 2 段 | 組み立ての型（足）・2 段の前例 `anchor.rs`（足）・座標の空間は `resolve_hit_owned` の `surface_point`（有） | **Missing**: 解放を受ける入口（キャラ窓の解放はメニューが持つ）・押下の控え・kanade の段の記憶 |
| 1.5 待たない | OS のダブルクリック（有） | なし（解放の時点で送れば満たす） |
| 1.6 送信の失敗 → 2 段目へ | アンカーの免除（足） | **Constraint**: 今のマウスの失敗は `Unloading{Fault}`。免除の条件を足す。2 段目の失敗の扱いは決まっていない（3.5） |
| 1.7・5.1〜5.5 右のクリックとメニュー | 右ダブルクリックの預かり `PendingDoubleClick` と `decide`（足） | **Missing**: 右クリックの預かり・抑止の決着で送る腕。**Constraint**: 決着は返事が来た後の tick なので、右のクリックは放した後に来た `OnMouseMove` などより後に届きうる（3.3） |
| 2.1〜2.4 中・拡張 | 解放の旗は 5 つとも立つ（有） | **Missing**: 拡張ボタンの押下が Pressed に届かない（wintf）・中／拡張を運ぶ型。**Unknown**: SSP が中ボタンで `OnMouseClick`（Ref5＝2）を送るか（5 節 ⑷） |
| 3.1 ドラッグの後は送らない | 開始の受け手 `on_char_drag_start`（足） | **Constraint**: 同じ tick の閾値越えと解放は解放が先に配られる（3.4） |
| 3.2 ダブルクリックの 2 回目の解放 | `double_click` つきの押下（有） | 押下の控えに印を残す（足） |
| 3.3 窓の外で放す | 左は捕捉で押した窓に届く・右／中／拡張は届いた窓が違う | 押下の控えと、放した位置の当たり判定（窓の中か）で見分ける（足）。**Unknown**: 捕捉中に窓の外で放したとき、wintf の当たり判定が `PointerState` をどの entity に付けるか（`mouse_click.rs` 151〜175 行目の窓の entity への縮退の経路） |
| 3.4 箱の選択肢 | `on_box_pointer_pressed` が真を返す（有） | 箱で消費された押下を控えに印す（足） |
| 3.5・4.3・4.4 今の動きを変えない | 強制退避・左右ダブルクリック・ドラッグの今の経路（有） | 押下の受け手の戻り値を変えるなら檻の扱い（3.8） |
| 3.6・9.x 記録 | `tracing` の構造化（有） | 送らなかった理由の語彙を決める（足）。水準は 9 の議題 |
| 4.1・4.2 順 | 殻の同期の往復（有） | なし（2 段は次の知らせより先に終わる） |
| 6.1〜6.10 ホイール | 注入の時計・間引きの純関数の型（足）・`arm_deadline`（有） | **Missing**: wintf のホイールの移し・窓ごとの溜め・期限の吐き出しの系。**Unknown**: 6.10（入力の焦点の窓に届く設定・透けた所）を「その窓に `PointerState` があるか」で判定できるか（3.7） |
| 7.1〜7.11 出入り | 当たり判定の解決（有）・`PointerLeave` の前例（足） | **Missing**: 窓ごとの「今の当たり判定」（新しい `hover_region.rs`）・キャラ窓の離脱の系。**Constraint**: `PointerLeave` は窓の中の entity の移りでも付く（3.6）。箱の選択肢の行の上では移動を送らずに返る（その間の出入りの扱い＝3.6） |
| 8.1〜8.4 話の最中 | `value_replaces_active_talk`（有）・定常以外の棄却（有）・メニュー表示中は OS のモーダルで入力が来ない（有） | 2 段目を積むときにも閉じの握手の保留を見る（足） |
| 10.1〜10.4 台帳と §8 | 台帳の行（足）・生成の道具（有） | 7 行の `owner` は 6 行が空 |
| 11.1〜11.4 決定論テスト | 模型の当たり判定・注入の時計・kanade の純粋な `step`（有） | wintf の 2 か所を直すならその兄弟テスト（`buffers.rs` の中のテスト・`dispatch/tests.rs`） |
| 12.1〜12.5 実機 | 手順書の段 3・段 7（有） | 在席の机（人が要る） |

## 3. brief と要件の前提と、実物の食い違い（設計で必ず扱う）

### 3.1 ホイールは wintf で止まっている（Missing・wintf に手が入る）

`WM_MOUSEWHEEL` は窓の entity の溜めに積まれるが、`PointerState.wheel` へ移す処理が無い。brief の Out of Boundary「wintf（ホイールも放した知らせも既にある。触らない見込み）」は、ホイールについては成り立たない。直し方の候補:
- ⑴ wintf の `transfer_buffers_to_world` に移しを足す（その窓の中で `PointerState` を持つ entity へ・移したら溜めを空にする・`PointerState` が無ければ捨てる）。今の `wheel` の型と「1 tick だけ有効」の決まりをそのまま使える。10〜20 行＋兄弟のテスト。
- ⑵ areka 側で wintf の溜めを読む（溜めは `pub(crate)` で外から読めない＝結局 wintf を触る）。
- 溜めが一度も空にならないので、今は回した量が窓の entity ごとに溜まり続けている（実害は無いが、⑴ で一緒に直る）。

### 3.2 拡張ボタンの押下は配られない（Missing）

`OnPointerPressed` の条件に `xbutton1_down || xbutton2_down` が無い。候補:
- ⑴ wintf の条件に足す（1 行）。ほかの Pressed の受け手（バルーンの押下・箱・強制退避）が拡張ボタンの押下で誤って動かないかを確かめる（どれも `double_click` や `left_down` を見ている見込み・設計で照合）。
- ⑵ areka の移動の受け手で毎 tick の `xbutton*_down` を読み、押下の控えを作る（wintf を触らないが、押下の扱いが 2 か所に分かれる）。

### 3.3 解放の受け手は 1 窓に 1 つ・右のクリックはメニューの決着を待つ

- キャラ窓の `OnPointerReleased` はメニューの受け手が持つ。クリックを受けるには、⑴ 入力の側に包み（クリックの判定 → 右ならメニューの受け手を呼ぶ）を作って `attach_release_handlers` の装着をそれに替える（`drag.rs` の `OnDragEnd` の包みと同じ形・依存の向きは今の `menu → input_events` を保つ）か、⑵ メニューの受け手の先頭からクリックの判定を呼ぶ（依存の向きが `input_events` を呼ぶ形のままで済む）。どちらも `menu/mod.rs` か `trigger.rs` に手が入る。
- 要件 5.2 の「抑止と決まったときだけ送る」は、決着が返事の後の tick（照会は非同期・期限つき）なので、**右のクリックは放した後の `OnMouseMove`・出入りより後に SHIORI へ届く**ことがある。要件 4.2 は「2 段の間に挟まない」なので反しないが、§8 に書く「右のクリックとメニュー」の決めに、この遅れを含めて書く必要がある。
- 右ダブルクリックの今の流れ（解放 1 → 照会 → 押下 2 で右ダブルクリックを預かる → 解放 2 は照会中なので無視）に右クリックの預かりを足すと、抑止の決着で「右クリック（解放 1）→ 右ダブルクリック（押下 2）」の順に送れば要件 4.1 と同じ順になる。解放 2 は要件 3.2 で送らない。

### 3.4 「ドラッグになった押下」の見分けと同じ tick の順（Constraint・Unknown）

- 解放の時点の wintf の状態（`JustEnded`）は、閾値を越えたかを残さない。
- `dispatch_drag_events` は `dispatch_pointer_events` の後に走るので、areka が開始の受け手（`on_char_drag_start`）で印を付ける形だと、**同じ tick に閾値越えと解放が入った**とき、解放の受け手が先に走ってクリックを送り、その後に `OnMouseDragStart`・`OnMouseDragEnd` が届く。
- 候補: ⑴ areka で開始の受け手の印を使い、同じ tick の場合は受け入れる（§8 に書く）。⑵ wintf の `JustEnded` に `started` を足し、解放の受け手で読む（wintf の `drag/state/mod.rs` を触る＝⑥ `dpi-realign-remembered-chain` が触る `drag/dispatch.rs` とは別のファイル）。⑶ 解放の受け手で押した位置と放した位置の距離を wintf と同じ閾値で測る（閾値の写しを持つことになる）。

### 3.5 送信の失敗の扱い（Constraint）

今のマウスのイベントの送信の失敗（`ShioriOutcome::Failed`）は、定常では `Unloading{Fault}`（終了の系列）へ倒れる。要件 1.6 は `OnMouseUp`（`Ex`）の失敗だけを 204 と同じに扱うと決めている。2 段目（`OnMouseClick`）の失敗は、アンカーの前例（最終段も免除して何もしない）に倣うか、今のマウスと同じに終了の系列へ倒すかが決まっていない。

### 3.6 窓の外へ出たときと、出入りを見る時点（Constraint）

- `PointerLeave` は「窓を出た」と「同じ窓の中で `PointerState` が別の entity へ移った」の両方で付く。キャラ窓の離脱の系は、印を見た後に「その窓に `PointerState` がまだあるか」で見分ける必要がある（バルーンの前例は `hover_action(.., None, ..)` で同じ問題を吸収している＝キャラ窓は出入りのイベントを送るので吸収できない）。
- 移動の受け手は、箱の選択肢の行の上では `on_box_pointer_moved` が真を返して早く返る。この間に当たり判定が変わった場合の出入りの扱い（今の当たり判定を控え続けるか・箱の上は「無し」とみなすか）は要件に書かれていない。
- 要件 7.10（窓が無くなったら覚えを捨て、Leave を送らない）は、ゴーストの切り替え・終了・窓の作り直し（`ghost_session::reopen_ghost_windows`）で `MouseWiring` の窓ごとの覚えを捨てる点が要る。`MouseWiring` は boot ごとに作り直される（`wire_mouse_input`）ので、切り替えでは自然に捨てられる見込み。窓だけの作り直しは確かめが要る。

### 3.7 ホイールが届く窓（Unknown）

Windows 10 以降の既定（「ポイントしたときに非アクティブウィンドウをスクロールする」）ではホイールはカーソルの下の窓へ、切ってあると入力の焦点の窓へ届く。wintf は `lParam` の画面座標を捨てている。3.1 の ⑴ で「その窓に `PointerState` があるときだけ移す」にすれば、要件 6.10 の「カーソルの下に無い窓」は自然に落ちる見込み（`PointerState` はカーソルが窓の上に居る間だけある）。透けた所（α 0・当たりの外）の上では、窓の当たりの扱い（`WM_NCHITTEST`）でそもそも areka の窓に届かない見込み。どちらも設計で確かめる。

### 3.8 今のテストを「変えずに通す」との関係（Constraint）

`on_char_pointer_pressed` は単発・中・拡張の押下で `false` を返すことを檻が確かめている。押下の控えを押下の受け手で作り、戻り値を変えないなら檻は通る（控えは副作用だけにして `false` を返し続ける）。戻り値を `true` に変えると、要件 11.4 の「変えずに通る」に当たらなくなる。

### 3.9 そのほかの古くなった記述

- brief の「`schedule/mod.rs` の状態の欄」は、今は `schedule/types.rs` の `State`（390 行）。
- brief の「ダブルクリックのときも 1 回目の `OnMouseClick` は起きる」は、wintf の配り（1 回目の押下と解放は単発として届き、2 回目の押下に `double_click` が付く）とも合う。
- ukadoc の `OnMouseUp` の Reference5 は「中クリックは2(互換仕様：OnMouseClickExへの移行を推奨)」と書く。見出しは「左・右のマウスボタンが放された際に発生」だが、Reference5 の説明は中ボタンの `OnMouseUp` があり得る書き方である（要件 2.4 は中ボタンを `OnMouseUp`／`OnMouseClick` で送らないと決めている＝5 節 ⑷ の測りに `OnMouseUp` の中ボタンも含める）。

## 4. 実装の案

### 案 A: 今あるファイルに足す

- kanade: `steady.rs` の `on_mouse` に 2 段の判断を直に書き、`on_reply` に段の腕を足す。入力: `input_events/mod.rs` に押下の控え・クリック・ホイール・出入りを全部足す。メニュー: `trigger.rs` の受け手にクリックを足す。
- 利点: ファイルが増えない。
- 欠点: `steady.rs` は 957 行で 1,000 行の上限を越える（43 行の余裕に 2 段の判断と記録は入らない）。`input_events/mod.rs` も 576 行から 900 行台へ膨らむ。行数の番人に当たるので**実質的に取れない**。

### 案 B: 新しい部品として分ける（wintf の穴も全部 wintf で直す）

- kanade: 新しい `schedule/mouse_click.rs`（`anchor.rs` と同じ形の段の記憶・2 段の判断・記録）と、組み立ては `events.rs` へ。
- 入力: 新しい `input_events/click.rs`（押下の控えと解放の判定の純関数＋受け手）・`input_events/wheel.rs`（窓ごとの溜めと期限の純関数＋吐き出しの系）・`input_events/hover_region.rs`（窓ごとの今の当たり判定と出入りの純関数・離脱の系・後の `shell-mouse-cursor`・`shell-tooltip`・`shell-hover-animation` が読むだけで使える公開の形）。
- wintf: ホイールの移し（3.1 ⑴）・拡張ボタンの押下の配り（3.2 ⑴）・`JustEnded` の `started`（3.4 ⑵）を全部 wintf で直す。
- 利点: 判断が純関数に集まり、要件 11 の決定論テストが書きやすい。後の 3 本が当たり判定の追跡を作り直さずに済む。wintf の穴（ホイールが死んでいる）をその場で塞ぐ。
- 欠点: wintf を 3 か所触る（どれもその層の正しい直し方だが、C11 の表の「触るファイル」に wintf は無い＝⑥ と別のファイルかの確かめが要る）。

### 案 C: 混ぜる（kanade と入力は B・wintf は塞がないと動かない所だけ）

- B と同じ新しいファイル群。wintf はホイールの移し（3.1 ⑴）だけ必須で直す。拡張ボタンの押下は wintf の条件 1 行（3.2 ⑴）か areka の移動の受け手（3.2 ⑵）の安い方、ドラッグの見分けは areka の開始の印（3.4 ⑴・同じ tick は §8 に書く）か wintf の `started`（3.4 ⑵）を設計で選ぶ。
- 結線: `input_events/mod.rs` には受け手の装着・`MouseWiring` の欄・押下の受け手の控えの数行だけ。`menu/mod.rs` の解放の受け手の装着を包みに替え、`trigger.rs` の `decide`／`poll_once` に右クリックの預かりを足す。離脱の系とホイールの吐き出しの系の登録は `ghost_session.rs` の `register_systems`（バルーンの離脱の系の隣）か `menu/mod.rs` の系の登録の隣。
- 利点: 行数の上限を守り、wintf の触りを最小にできる。前例（アンカーの段・右ダブルクリックの預かり・バルーンの離脱の系・`arm_deadline`）が全部ある。
- 欠点: 判断が kanade（2 段）と入力（押下の控え・ホイール・出入り）とメニュー（右）の 3 か所に分かれる＝§8 の登記で 1 か所にまとめて書く必要がある。

### 案ごとの触るファイル（C の見込み）

| 層 | ファイル | 中身 |
| --- | --- | --- |
| kanade | `src/msg.rs` | `MouseEventKind` の腕（クリック・ホイール・出入り）・中／拡張のボタンの型 |
| kanade | `schedule/events.rs` | 表の 7 語・組み立て 7 つ（共通関数 1 つ） |
| kanade | 新 `schedule/mouse_click.rs`＋兄弟のテスト | 段の記憶と 2 段の判断・記録 |
| kanade | `schedule/steady.rs` | `on_mouse` の腕・`on_reply` の入口の数行 |
| kanade | `schedule/types.rs` | `State` の欄 1 つ |
| kanade | `schedule/mod.rs` | 失敗の免除の条件 1 つ（`state.anchor.is_some()` の隣） |
| kanade | `schedule/events_change_tests.rs` | 手書きの数 55 → 62 |
| 入力 | `input_events/mod.rs` | 装着・`MouseWiring` の欄・押下の受け手の控え・移動の受け手から出入りとホイールへ |
| 入力 | 新 `input_events/click.rs`・`wheel.rs`・`hover_region.rs`＋兄弟のテスト | 純関数と受け手・離脱の系・吐き出しの系 |
| 入力 | `input_events/drag.rs` | 開始の受け手で押下の控えに印（3.4 ⑴ を採るとき） |
| メニュー | `menu/mod.rs`・`menu/trigger.rs`＋兄弟のテスト | 解放の受け手の包み・右クリックの預かりと決着 |
| 系の登録 | `ghost_session.rs`（か `menu/mod.rs`） | 離脱の系・吐き出しの系 |
| wintf | `pointer/buffers.rs`（と中のテスト） | ホイールの移し |
| wintf | `pointer/dispatch/mod.rs`（と `dispatch/tests.rs`） | 拡張ボタンの押下の配り（3.2 ⑴ を採るとき） |
| 文書 | `doc/ukadoc-coverage/ledger/shiori.toml` の 7 行・報告の再生成・`doc/COMPAT_ARCHITECTURE.md` §8 | 要件 10 |

## 5. ukadoc で決まらない 5 点の証拠

ukadoc MCP で `OnMouseUp`・`OnMouseUpEx`・`OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel`・`OnMouseEnter`・`OnMouseLeave`・`OnMouseLeaveAll`・`OnMouseMultipleClick(Ex)`・`OnMouseGesture` を引き、`OnMouseUp`・`OnMouseLeave` の語で全文を検索した。SSP ヘルプ・YAYA・里々の文書も同じ語で引いた。

| 点 | ukadoc の書き方 | 手元の記録 | 判定 |
| --- | --- | --- | --- |
| ⑴ 当たり判定から当たり判定へ直に移ったときの `OnMouseLeave` と `OnMouseEnter` の順 | `OnMouseLeave`「マウスが当たり判定から出た際に発生。」・`OnMouseEnter`「マウスが当たり判定に入った際に発生。」。順は書かない | ぱすたの SSP の測り（`target\pasta-verify\ssp\stage3-pasta-log.txt`・完了 `areka-P0-pasta-in-windows-verification` の `ssp-measurement.md` 82 行目）は × に乗る → 離れる（当たり判定「閉じる」と「無し」の間）だけで、2 つの当たり判定の間を直に移る場面が無い | **決まらない**。SSP で測る |
| ⑵ ダブルクリックの 2 回目の解放で `OnMouseUp`／`OnMouseClick` が来るか | `OnMouseClick`「マウスボタンを1回押して放された瞬間」。`OnMouseMultipleClick` の「応答が無い場合、通常通りOnMouseClickとOnMouseDoubleClickの流れに戻る」は流れの在ることだけを言う | 無し | **決まらない**。SSP で測る |
| ⑶ ドラッグの終わりの解放で `OnMouseUp` が来るか | `OnMouseUp`「左・右のマウスボタンが放された際に発生。」（ドラッグの除外を書かない＝字面どおりなら来る）。`OnMouseClick` は「1回押して放された瞬間」 | 里々の既定の動き（里々の文書「デフォルトの動作」）が `OnMouseDown`・`OnMouseUp` で押しっぱなしを測り、離したときに「ホールド終了」を出す＝SSP が `OnMouseUp` をふつうに送っている傍証。ドラッグの後かどうかは書かれていない | **決まらない**。字面は「来る」寄り。SSP で測る |
| ⑷ 中ボタンで `OnMouseClick`（Ref5＝2）と `OnMouseClickEx` のどちらが来るか | `OnMouseClick` の見出しは「左・右ボタン（＋中ボタン）」、Reference5 は「中クリックは2(互換仕様：OnMouseClickExへの移行を推奨)」。`OnMouseUp` の Reference5 も同じ文。`OnMouseClickEx` は「左・右ボタン以外」で Reference5＝`middle` | 無し | **決まらない**。両方来る・Ex に返事が無いときだけ互換の側が来る・どちらか一方、のどれとも読める。SSP で測る（`OnMouseUp` の中ボタンも） |
| ⑸ `OnMouseWheel` の Reference2 の単位（1 目盛りで 120 か 1 か） | 「マウスホイールの回転量および回転方向。」。タッチパネルの代わりのときだけ「upなら正数、downなら負数」 | YAYA の文書「ホイール反応を自然なものにする」が「マウスの種類や設定によっては一回ホイール回しただけで2～3回連続」の返事が出ると書く＝SSP は OS の知らせごとに 1 回送り、間引かない傍証（単位は書かない）。里々の既定の動きは「同じ場所で 2 回以上ホイールが動いたら」（回数で数える） | **決まらない**。SSP で測る。あわせて「SSP は間引かない」なら要件 6.3（100 ms に 1 回）と里々の「2 回以上」の数え方の関係を議題に足す |

手元の記録の限り: ぱすたの `pasta.log` は**応答が 200 の要求だけ**を残す（`stage3-pasta-log.txt` の要求 8 件の応答はどれも `200 OK`・撫での往復でも `OnMouseMove` は 2 件だけ）。ぱすたは `OnMouseUp` に返事を持たない（204）ので、SSP が `OnMouseUp` を送ったかどうかはこの記録からは読めない。測るときは SSP の `ghost\master\ssp_shiori_log.txt`（記憶「SSP が SHIORI へ送った要求は全部 `ssp_shiori_log.txt` に残る」）を持つゴーストで、バイト位置を控えて差分を読む。

## 6. 並走のウェーブ（C11）との重なり

| 相手 | 相手の触るファイル（brief・roadmap の C11 の表） | 本 spec（案 C）との重なり |
| --- | --- | --- |
| ① `descript-window-keys` | `placement/{config,measure}.rs`・`emo2_boot/{assets,switch_assets}.rs`・`placement/follow/drag_follow.rs`・`emo2_boot/consumer_ledger.rs`・§8 | §8 の末尾の追記だけ（許す重なり） |
| ③ `readonly-mode` | `cli/`・`boot_config.rs`・`install/desk.rs`・`update/desk.rs`・`boot_resolve.rs`・書き込みの口・`menu/roots_frame.rs` | `menu/` の別のファイル（本 spec は `menu/mod.rs`・`trigger.rs`）＝0 |
| ④ `mcp-scope-rect-property` | sylphya・`areka-ghost`・新しい `emo2_boot` のファイル・`mcp/get_property.rs` | 0 |
| ⑤ `vertical-choice-hover-band` | emo-text の `choice.rs` ほか | 0 |
| ⑥ `dpi-realign-remembered-chain` | `placement/{chain_realign,chain_finalize,spawn,persist}.rs`・`emo2_boot/frame.rs`・`frame/{dpi,drain_resnap}.rs`・**wintf の `window_proc/mouse_move.rs`・`drag/dispatch.rs`**・§8 | 本 spec の wintf の候補は `pointer/buffers.rs`・`pointer/dispatch/mod.rs`（・3.4 ⑵ なら `drag/state/mod.rs`）＝別のファイル。**`mouse_move.rs`（`PointerLeave` を付ける所）と `drag/dispatch.rs` には触らない形にする**。§8 は許す重なり |
| ⑦ `mcp-strict-errors` | 案 1＝`mcp/{sakurascript,raise_event}.rs`・`areka-mcp/src/handler.rs`・kanade の `schedule/sstp.rs`（案 2 なら kanade の `msg.rs`） | 案 1 なら 0。案 2 を採ると kanade の `msg.rs` で重なる（C11 の約束で ⑦ は `msg.rs` に触らない） |
| ⑧ `shiori2-protocol` | `shiori-host32-host/src/{shiori3,client}.rs`・新 `shiori2.rs`・`areka-ghost/src/shiori_inproc.rs`・kanade の `actor.rs`（要るときだけ）・`shiori.toml`・§8 | 本 spec は `actor.rs` に触らない（1.1）。`shiori.toml` は別の行（許す重なり）。⑧ が版の切り替えの状態を kanade の `schedule/` に置くと決めたら重なる（約束で置かない） |

C11 の表が本 spec に挙げていない触る先: **wintf の `pointer/buffers.rs`（必須）・`pointer/dispatch/mod.rs`（候補）・`menu/mod.rs`・`menu/trigger.rs`・`schedule/types.rs`・`ghost_session.rs`（系の登録）・`input_events/drag.rs`**。どれも C11 のほかの 7 本の触る先に無い（上の表）。後のウェーブの `shell-mouse-cursor`（`input_events/` と追跡のファイル）・`balloon-scroll-arrows`（`input_events/mod.rs`）・`balloon-communicatebox`／`shell-scale-linkage`（送ってよい表の手書きの数）は、本 spec の着地の後に並ぶので今は重ならない。

## 7. 規模と危険

- **規模: M〜L**（タスク 14〜18 の見込み）。kanade（型・表・組み立て・2 段・失敗の免除）3〜4、クリックの入力（押下の控え・解放の判定・包み）3、右クリックとメニュー 2、ホイール（wintf の移し・溜めと期限・吐き出しの系）3、出入り（追跡・離脱の系）2〜3、台帳と §8 1、実機 1。brief の 13〜16 より、wintf の穴（3.1・3.2）とメニューの包み（3.3）の分だけ多い。
- **危険: 中**。前例が全部ある（2 段＝アンカー、預かり＝右ダブルクリック、離脱の系＝バルーン、期限で起こす＝`balloon_visibility_phase.rs`）。中にする理由は ⑴ wintf のポインタの層を触る（ホイールの移し先の entity の選び方）、⑵ 同じ tick の閾値越えと解放の順（3.4）、⑶ `PointerLeave` が窓の中の移りでも付くこと（3.6）、⑷ SSP の測りの答えしだいで要件 3・4・6・7 の文が動くこと。

## 8. 設計への申し送り

### Research Needed（設計の段で調べる）

1. wintf のホイールの移し先: 窓の entity の溜めを「その窓の中で `PointerState` を持つ entity」へ移す方法（`find_owner_window` の逆引き・`collect_entities_to_leave` の窓の絞り込みの再利用）。移したら溜めを空にする。
2. 拡張ボタンの押下を Pressed で配ったとき、今の Pressed の受け手（バルーン・箱・強制退避・メニュー）が誤って動かないか。
3. 捕捉中に窓の外で左を放したときの `PointerState` の付き先と `client_point` の値（`mouse_click.rs` 151〜175 行目）。要件 3.3 の判定に使えるか。
4. 同じ tick の閾値越えと解放の起こりやすさ（閾値は `window_proc/mouse_move.rs` のインラインの計算）と、3.4 の ⑴〜⑶ のどれで塞ぐか。
5. キャラ窓の `PointerLeave` から「窓を出た」だけを取り出す方法（その窓に `PointerState` が残っているか）と、窓の作り直し（`reopen_ghost_windows`）で `MouseWiring` の窓ごとの覚えを捨てる点。
6. ホイールの吐き出しの系の置き場所と、`arm_deadline` の期限で起きた tick で溜めを確実に吐き出せるか（記憶「時刻は正確に扱う」＝期限は前の送出＋100 ms ちょうど）。
7. SSP の 5 点の測り（5 節）。在席の机・SSP の `ssp_shiori_log.txt` を持つゴースト・測る前に開発者の許しを取る。

### 議題の材料（要件討議へ・番号は下の「決めの項目」と同じ）

下の「決めの項目」に挙げる。

## 決めの項目（要件討議・設計で答えが要る）

1. **wintf を触ってよいか**（brief の Out of Boundary の改め）。ホイールは wintf の移しが無いと一切動かない（3.1）。ほかに拡張ボタンの押下の配り（3.2 ⑴）とドラッグの `started`（3.4 ⑵）を wintf で直すか、areka 側で回るか。
2. **2 段目（`OnMouseClick`／`OnMouseClickEx`）の送信の失敗**を、アンカーと同じく 204 と同じに扱って終えるか、今のマウスのイベントと同じに終了の系列（`Unloading{Fault}`）へ倒すか（3.5・要件 1.6 は 1 段目だけを決めている）。
3. **右のクリックの遅れ**: メニューの照会の決着を待って送るので、放した後の `OnMouseMove`・出入りより後に届くことがある。これを受け入れて §8 に書くか（3.3）。
4. **解放の受け手のまとめ方**: 入力の側の包みに `attach_release_handlers` を替えるか、メニューの受け手の中からクリックを呼ぶか（3.3）。どちらも `menu/` に手が入る＝C11 の約束の「`menu/trigger.rs` は議題 3 で要るときだけ」は議題 3 の推奨を採った時点で「要る」になる。
5. **同じ tick の閾値越えと解放**（3.4）: クリックが送られた後に `OnMouseDragStart`／`OnMouseDragEnd` が届く場合を受け入れて §8 に書くか、wintf の `started` で塞ぐか。
6. **箱の選択肢の行の上での出入り**（3.6）: 移動の受け手が早く返る間、当たり判定の覚えをどう扱うか（要件 7 に書かれていない）。
7. **押下の受け手の戻り値**（3.8）: 控えを副作用だけにして `false` を返し続けるか（今の檻がそのまま通る＝要件 11.4）、`true` に変えて檻を直すか。
8. **中ボタンの扱いの範囲**（5 節 ⑷・3.9）: `OnMouseUp` の Reference5 にも「中クリックは2」があるので、SSP の測りに `OnMouseUp` の中ボタンを含め、答えによって要件 2.4 を直す。
9. **ホイールの間引きと里々の数え方**（5 節 ⑸）: SSP が知らせごとに 1 回送る（間引かない）なら、要件 6.3 の 100 ms に 1 回は、里々の既定の「同じ場所で 2 回以上」の反応を出にくくする。SSP の測りの答えと合わせて、間引きの幅を保つか見直すか。
10. **記録の水準**（要件 9.1〜9.3）: 今のマウスの送出の記録は trace（`mouse_get`・`shiori_request`）。「何も喋らない」の見分けに使うなら、送った・送らなかったの行を debug 以上にするか（要件 9.4 は info 以上の 1 回ごとの行を禁じるだけ）。
11. **送ってよい表の手書きの数**（1.1）: 55 → 62 を手で直すか、表から数える形へ改めるか（後のウェーブの 2 本も同じ数を動かす・棚卸㉙の裁定の 8 と同じ型の問い）。
12. **SSP の 5 点の測りを要件討議の前に行うか**（要件の Introduction の段取り）。手元の記録では 5 点とも決まらなかった（5 節）。在席の机と開発者の許しが要る。

## 9. SSP の測り（2026-10-23・要件討議の議題の承認を受けて）

- 測った物: SSP 2.9.x（`C:\wintools\ssp\ssp.exe`・pid 41060）・ゴースト「窓際のぱすたさんDRESSTEST」（`pasta.toml` 1.0.0 の検体・どのマウスのイベントにも 204 を返す）。当たり判定は `shell\master\surfaces.txt` の `collision1,239,282,344,420,胴`・`collision3,213,420,344,478,腰`。
- 測り方: 排他テストの机を取り、Win32 の `SendInput`（絶対座標の移動・ボタン・ホイール）で操作した。SSP が送った要求は、開発用パレットの「SHIORI呼出ログ有効」（10-21 から ON のまま）の `ghost\master\ssp_shiori_log.txt` の差分で数えた（測る前にバイト位置を控えた）。computer-use のカーソル移動は Claude のアプリの覆いの窓に吸われ、SSP に `WM_MOUSEMOVE` が届かなかったので使っていない（クリックは届いた）。
- 後始末: ドラッグで動いた窓は元の位置（`-775,218`）へ戻した。開いた右クリックのメニューは Esc で閉じた。ゴーストは測る前の「窓際のぱすたさん」へ戻した。SSP の設定は変えていない。

| 点 | 操作 | SSP が送った順（`ID`・Reference4・Reference5。R2 は 0 のとき略） | 要件との突き合わせ |
| --- | --- | --- | --- |
| ⑹ 窓と当たり判定に同時に入る | 窓の外 → 胴へ 1 回で移る | `OnMouseEnterAll`(胴) → `OnMouseEnter`(胴) → `OnMouseMove`(胴) | 7.13 と同じ（`EnterAll` が先・Reference4 は入った位置の当たり判定＝7.11 と同じ） |
| ⑴ 当たり判定から当たり判定へ直に移る | 胴 → 腰へ 1 回で移る | `OnMouseLeave`(胴) → `OnMouseEnter`(腰) → `OnMouseMove`(腰)。`OnMouseLeave` の Reference0／1 は**移った後の位置** | 7.3 と同じ順。出入りが移動より先（7.7）も同じ |
| ⑹ 当たり判定の上から窓の外へ出る | 腰 → 窓の外へ 1 回で移る | `OnMouseLeave`(**空**) → `OnMouseLeaveAll`(**空**)。Reference0／1 は**窓の外の位置**（ローカル座標で 250,854） | 順は 7.13 と同じ。**Reference4 が空で、要件 7.4・7.12（出た当たり判定の名前）と違う**。座標も 7.5（窓の中で最後に見た位置）と違う |
| 単発のクリック | 腰で左を押して放す | `OnMouseDown`(腰,0) → `OnMouseUp`(腰,0) → `OnMouseClick`(腰,0) | 1.1・1.2 と同じ |
| ⑵ ダブルクリック | 腰で左を 2 回（間 120 ms） | `OnMouseDown` → `OnMouseUp` → `OnMouseClick` → `OnMouseDoubleClick`（2 回目の押下・`OnMouseDown` は来ない）→ **`OnMouseUp`**（2 回目の解放・`OnMouseClick` は来ない） | 4.1 の順は同じ。**2 回目の解放で `OnMouseUp` が来る＝要件 3.2（Up も Click も送らない）と違う** |
| ⑶ ドラッグ | 腰で押して 60 px 動かして放す | `OnMouseDown` → `OnMouseDragStart` →（既定の位置の照会）→ `OnMouseDragEnd`。**`OnMouseUp`・`OnMouseClick` は来ない** | 3.1 と同じ |
| ⑷ 中ボタン | 腰で中を押して放す | `OnMouseDownEx`(middle) → `OnMouseDown`(2) → `OnMouseUpEx`(middle) → `OnMouseClickEx`(middle) → `OnMouseUp`(2) → `OnMouseClick`(2) | **両方の系列が来る**（`Ex` が先・続けて互換の Reference5＝2）。要件 2.4（互換の側は送らない）と違う |
| ⑷' 拡張ボタン 1 | 腰で押して放す | `OnMouseDownEx`(xbutton1) → `OnMouseUpEx`(xbutton1) → `OnMouseClickEx`(xbutton1) | 2.1〜2.3 と同じ（互換の側は来ない） |
| ⑸ ホイール 1 目盛り | 上へ 1 | `OnMouseWheel`(腰, R2=120) | 6.2 の単位と向き（1 目盛り＝120・上が正）と同じ |
| ⑸ ホイールを速く 3 目盛り | 下へ 3（30 ms おき） | `OnMouseWheel`(R2=-120) を 3 回（30 ms おき） | **SSP は間引かず、知らせごとに送る**。要件 6.3（100 ms に高々 1 回・足し合わせ）と違う（討議で決めた areka の決め） |
| 右クリック | 腰で右を押して放す（ぱすたはメニューを抑えない） | `OnMouseDown`(1) → `OnMouseUp`(1) → `OnMouseClick`(1) → メニューの照会（`sakura.popupmenu.visible` ほか）→ メニューが出る → `OnMouseLeave`(腰)・`OnMouseLeaveAll`(腰)（メニューが窓を覆ったため） | **メニューが出るときも、右の `OnMouseUp`／`OnMouseClick` をメニューより先に送る**。要件 5.1（メニューが有効な間は送らない・完了 `popup-menu-minimal` 1.5）と違う |

ほかに見えたこと（本 spec の範囲の外）: SSP は `OnMouseDown`・`OnMouseDownEx` を送る（brief の Out）。止まって約 0.5 秒で `OnMouseHover` と `tooltip` の照会を送る（`OnMouseHover` は Out・ツールチップは後の `shell-tooltip`）。

測りの答えを受けた要件の改め（2026-10-23・開発者が推奨の案で承認）: 差 1 ダブルクリックの 2 回目の解放は `OnMouseUp` だけ・差 2 中ボタンは `Ex` の 2 段に返事が無いときだけ互換の 2 段を続ける・差 3 右のクリックは放した時点でメニューより先に送る・差 4 窓の外へ出たときの Reference は要件のまま（§8 に違いを書く）。これで「決めの項目」の 3（右のクリックの遅れ）は無くなり、`menu/trigger.rs` の `decide`／`poll_once` に右クリックの預かりを足す必要も無くなった。4（解放の受け手は 1 窓に 1 つで、今はメニューが持つ）は残る＝`menu/mod.rs` の装着の替え方は設計で決める。

## 10. 設計の段の調べと決め（2026-10-23・kiro-spec-design）

- 発見の種類: 既存の仕組みへの拡張（light）。新しい外部の依存は無いので Web の調べはしていない。読んだ物: `rules/design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`・`design-principles.md`・設計の雛形・steering（`logging.md`・`structure.md` のテストの置き方と 1,000 行の目安・`roadmap.md` の C11 の表）。使った技能の指針: kiro-spec-design の規則一式と、作業の型（最小の差分で全部に届かせる）。

### 10.1 Research Needed 1〜6 の答え

| 番号 | 問い | 答え（コードで確かめた所） | 設計への写し |
|---|---|---|---|
| 1 | ホイールの移し先 | `WM_MOUSEWHEEL` は窓の entity の `WHEEL_BUFFERS` に積まれる（`window_proc/mouse_dblclick_wheel.rs`）。`PointerState` は窓の中のちょうど 1 つの entity に付く（`collect_entities_to_leave` が同じ窓の他の持ち主を外す）。 | `transfer_buffers_to_world` で `find_owner_window` がその窓を返す持ち主を探して `wheel` に入れ、溜めは毎回空にする。持ち主が居なければ捨てる（要件 6.10 もここで満たす） |
| 2 | 拡張ボタンの押下を配ったときの誤動作 | 押下の受け手は 3 つ（キャラ窓・バルーン・箱）。バルーンは `left_down` だけ、強制退避は `double_click == Left`、箱は `left_down` で選択・中断は `double_click == Left`。箱の「直前の押下が選択だったか」は中ボタンの押下でも今上書きされる。 | 条件に `xbutton1_down || xbutton2_down` を足すだけで、拡張ボタンは今の中ボタンと同じ扱いになる |
| 3 | 捕捉中に窓の外で左を放したとき | `WM_LBUTTONUP` は捕捉した窓に届き、当たり判定が外れて窓の entity の `PointerState` に窓の外の座標で載る（`mouse_click.rs` の縮退の枝・`mouse_move.rs` の当たり判定の無い枝）。 | 解放の点が `WindowPos.size` の矩形の外なら「窓の外で放した」 |
| 4 | 同じ tick の閾値越えと解放 | ドラッグの状態（`DRAG_STATE`）は WndProc の中で同期に変わる。閾値越え（`WM_MOUSEMOVE`）も解放（`end_dragging_on_release`）も tick の前に済むので、tick の中の解放の受け手は `JustEnded` を読める。ただし `JustEnded` は開始を配ったかを残さない（`close_drag` の中の `started` が外に出ない）。 | `close_drag` で `started` を thread_local の印に書き、`DragState::ended_after_start()` で読む。`JustEnded` に欄を足すと `clickthrough/controller_tests.rs` と `drag/reentry_tests.rs`（構造体の字面の組み立てと欄の全部を書いた分解）に手が入り「wintf は 3 ファイル」を越えるので、欄は足さない。新しい関数を公開の一覧（`drag/mod.rs`）に足さずに済むよう、公開済みの型 `DragState` の関連関数にする |
| 5 | 窓を出たことの見分けと、窓の作り直し | `PointerLeave` は窓を出たときと窓の中で持ち主が替わったときの両方で付く。窓を出ると（`WM_MOUSELEAVE`）その窓の持ち主は 0 になる。窓の作り直し（`reopen_ghost_windows`）は窓の entity を替える。 | `PointerLeave` を見ず、毎 tick「その窓に持ち主が居るか」で読む。覚えの鍵を窓の entity にし、消えた entity の覚えは黙って捨てる（要件 7.10）。ゴーストの切り替えは `MouseWiring` の作り直しで捨てられる |
| 6 | ホイールの吐き出しの置き場所と期限 | `tick_wake` が預かる期限は最も早い 1 つで、到来すると倒れる（`balloon_visibility_phase.rs` の注）。`OnPointerMoved` は持ち主の居る窓にしか配られないので、カーソルが出た後の吐き出しには使えない。 | `dispatch_pointer_events` の前の排他の系 `char_watch` が毎回吐き出しを確かめ、溜めが残る限り毎回預け直す。期限は前の送出＋100 ms ちょうど（注入の時計の ms） |

### 10.2 決めの項目（カテゴリー B）の答え

| 項目 | 決め | 理由 |
|---|---|---|
| 4 解放の受け手のまとめ方 | `menu/mod.rs` の `attach_release_handlers` が入れる受け手を、クリックの判定 → `trigger::on_char_pointer_released` の順に呼ぶ包みに替える。`trigger.rs` は直さない | 依存の向き（`menu` → `input_events`）を保つ。`trigger.rs` を直さないので、`trigger` を直に呼ぶ今の檻（`trigger_flow_tests.rs` ほか）がそのまま通る。装着の檻は部品の有無しか見ない（`mod_wiring_tests.rs`・`trigger_wired_tests.rs`） |
| 6 箱の選択肢の行の上での出入り | 出入りとホイールは `dispatch_pointer_events` の前の系がシェルの当たり判定で決めるので、箱の行の上でも同じに送る。移動の受け手の箱の前段（行の上では `OnMouseMove` を送らない）は変えない。§8 に登記 | 要件は箱の上の出入りを書いていない。特別扱いの判定（箱の行の上かの純粋な問い）は今のコードに無く、作ると箱の受け手の副作用（強調）と分ける仕事が増える |
| 7 押下の受け手の戻り値 | 変えない。押下の控えは副作用だけにして、単発・中・拡張では `false` を返し続ける | 今の檻（`handler_middle_xbutton_and_single_click_do_not_send` ほか）が変えずに通る（要件 11.4） |
| 10 記録の水準 | クリックの系（送った・止めた・送らなかった）は info、出入りとホイールの送出と和 0 は debug、受け付けない時期は今の trace、送信の失敗は error | `get_log` は info 以上を集める（`log_history::classify`）。クリックは操作 1 回に数行なので info でも埋まらず、MCP の自動試験で「送っていない」と「返事が無い」を見分けられる。出入りとホイールは動きにつれて出るので要件 9.4 の趣旨で debug |
| 11 送ってよい表の手書きの数 | 手で 55 → 64 にする。表から数える形には改めない | 手書きの数は「母数も判定する」檻。同じウェーブ（C11）で表に語を足す spec は他に居ないので、両枝が同じ数を書く危険は今は無い（後の `balloon-communicatebox`・`shell-scale-linkage` は本 spec の着地の後） |

### 10.3 案の比べ（設計の段で足した点）

| 案 | 中身 | 良い所 | 困る所 | 決め |
|---|---|---|---|---|
| 出入りとホイールを移動の受け手 `on_char_pointer_moved` に入れる | 当たり判定を 1 回で済ませ、出入り → 移動 → ホイールを送る | 系を足さない | 最初の移動で `OnMouseEnterAll` が増え、移動とドラッグの今の檻（`drag_tests.rs` の「最初の移動は届く」の 1 件・`shell_box_handler_tests.rs` の `[Move]` ちょうど 1 件ほか）が崩れる＝要件 11.4 に反する。窓を出た後の吐き出しもできない | 採らない |
| `dispatch_pointer_events` の前の排他の系にする | 毎 tick、持ち主から窓を引いて出入りとホイールを決める | 移動の受け手に触れない・出入りが同じ tick の移動より先（要件 7.7）・窓を出たことを持ち主の有無で読める・吐き出しも同じ所 | 当たり判定を移動の受け手と 2 回解く（動いたときだけ・窓 1〜2 枚） | 採る |
| ドラッグの見分けを areka の開始の受け手の印でする | `on_char_drag_start` で控えに印 | wintf を触らない | `dispatch_drag_events` は `dispatch_pointer_events` の後なので、同じ tick の閾値越えと解放でクリックが先に出る | 採らない（要件 3.1 の「同じ瞬間」） |

### 10.4 まとめの 3 つの見方（synthesis）

- 一般化: ボタン 5 つ × 「2 回目の解放か」の 10 通りを、段の列の表 1 つと「台本なら止める・それ以外は次へ」の規則 1 つにまとめた（要件 1.2・1.3・1.6・1.8・2.2〜2.5・3.2 が同じ規則）。出入りの 4 語とホイールも同じ Reference の組み立て `mouse_get`（今の `mouse_drag_get` を広げた物）で作る。
- 作るか借りるか: 段の記憶はアンカー（`schedule/anchor.rs`）の形、失敗の免除はアンカーと同じ条件の並び、期限で起こすのは `tick_wake::arm_deadline`、窓の持ち主の引き方は `find_owner_window`、純関数＋値型の状態は `throttle.rs` に倣う。新しい仕組みや依存は作らない。
- 削ったもの: 見張りの状態を読む公開の口（読み手が居ない・後の spec が足す）・`JustEnded` の欄の追加（関連関数で足りる）・`trigger.rs` の手直し（包みで足りる）・移動の受け手の手直し（系で足りる）・`MouseButton` の拡張（別の型 `ClickButton` で、ダブルクリックの組み立てに触れない）。

### 10.5 本 spec の中で決めた細部（§8 へ）

- 中ボタンのダブルクリックの 2 回目の解放は `OnMouseUpEx` → 返事が無ければ `OnMouseUp`(2)。要件 3.2（`Click` の系は送らない）と 2.4（`Ex` に返事が無ければ互換が続く）を両方守る列。SSP では測っていない。
- 2 つのキャラ窓の間を移ったときは、出る窓の出入り（`OnMouseLeave` → `OnMouseLeaveAll`）を入る窓より先に送る。
- ホイールと出入りの送信の失敗は、今の `OnMouseMove` と同じに終了の系列へ倒れる（要件が免除を求めるのはクリックの段だけ）。

### 10.6 残る危険

- 1 tick に「押下 1・解放 1・押下 2」が全部入ると、`OnMouseDoubleClick` が解放 1 のクリックより先に届く（wintf がその tick の押下と解放を 1 つの `PointerState` にまとめるため）。UI スレッドが詰まったときだけ起きる。直すには wintf の配りを出来事の列に改める必要があり、本 spec の範囲を越える（design の R1）。

## 11. 設計討議の決め（2026-10-23・開発者が推奨の案で承認）

- 検証（`design-validation.md`・判定 GO）の 3 件のうち、絵の透けた所でのホイール（6.10）とドラッグの終わりの読み方（3.1）は自明な直しとして `design.md` に入れた（`DragState::last_release_was_drag()`・見張りの系で持ち主が窓そのもののときはホイールを捨てる）。
- 議題 1（右のクリックの段の列がメニューの照会の 1 秒の枠を食う）: `trigger.rs` は直さず R4 として受け入れ、実機の確かめに 1 手順を足す。
- 議題 2（1 tick に押下・解放・押下が全部入ったときの順の崩れ・R1）: 本 spec では受け入れて §8 に書き、本 spec の完了の時に `/kiro-discovery` で「wintf のポインタの配りを出来事の列にする」を起票する。
