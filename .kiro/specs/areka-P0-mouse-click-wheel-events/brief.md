# Brief: areka-P0-mouse-click-wheel-events

> 2026-10-10 棚卸㉓で、roadmap の覚え書き「持ち主のいない SHIORI のイベント 2 群」の ⑴（単押しのクリックとホイール）から起票した。棚卸㉒で `mcp-user-response` の再測定から登記された件。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

キャラクターを 1 回クリックしたとき、ホイールを回したときに、areka は SHIORI へ何も知らせない。多くのゴーストは「つつく」「ホイールでなでる」の返事を辞書に持っているが、areka ではその返事が 1 度も呼ばれず、「何も喋らない」という形でだけ現れる。記録にも出ない。

## Current State

- 正典（ukadoc「SHIORI Event」の一覧を ukadoc MCP で確かめた。以下は要旨）:
  - `OnMouseClick`: 左・右（と中）のボタンを 1 回押して放した瞬間に、**`OnMouseUp` に反応が無かったとき**に起きる。Reference0／1＝カーソルの x／y（ローカル座標）・2＝常に 0・3＝本体 0／相方 1・4＝当たり判定の識別子・5＝左 0／右 1／中 2（中は互換のためで `OnMouseClickEx` への移行を勧める）・6＝入力の機器（`mouse`・`touch` など）。
  - `OnMouseClickEx`: 左・右以外のボタンで、`OnMouseUpEx` に反応が無かったときに起きる。Reference5＝`middle`・`xbutton1`・`xbutton2`。
  - `OnMouseWheel`: ホイールが回ったときに起きる。Reference2＝回転の量と向き・5＝常に 0・ほかは上と同じ並び。タッチパネルで `OnMouseGesture` に返事が無かったときの代わりとしても起きる（SSP 2.3.53 以降）。
  - `OnMouseMultipleClick`: 3 回以上の連続クリック。返事が無ければ `OnMouseClick` と `OnMouseDoubleClick` のふつうの流れに戻る、と書く＝**ダブルクリックのときも 1 回目の `OnMouseClick` は起きる**と読める（クリックを待って見分ける決まりは正典に無い）。
- 網羅台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）: `OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel` は `absent`・持ち主なし。**`OnMouseUp`・`OnMouseUpEx`・`OnMouseDown`・`OnMouseDownEx` も `absent`・持ち主なし**（`OnMouseClick` の前に来るはずのイベントが無い）。`OnMouseDoubleClick`・`OnMouseMove`・`OnMouseDragStart`・`OnMouseDragEnd` は `implemented`。
- kanade が送るマウスの出来事は 4 つ（`crates/areka-kanade/src/msg.rs` の `MouseEventKind`＝移動・ダブルクリック・ドラッグの開始と終了）。送ってよいイベントの表 `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`・793 行）に、上の 7 つの名前は無い。
- キャラクターの窓の押下の処理 `on_char_pointer_pressed`（`crates/areka/src/input_events/mod.rs`・571 行）は、単発のクリックと、中・拡張ボタンのダブルクリックを「送らない」と明記している。完了 `input-events` の要件が、最初の範囲の線として「`OnMouseWheel` は送らない（Reference2 の口だけ残して `0` を入れる）」「`OnMouseClick` の単発は送らない」と決めた。
- ダブルクリックの見分けは OS がしている。wintf の窓の手続きが OS のダブルクリックのメッセージを受けて、`PointerState.double_click`（`crates/wintf/src/ecs/pointer/types/mod.rs`）に 1 フレームだけ載せる。areka に時間を計って見分ける仕掛けは無い。
- ホイールは wintf に既にある。OS のホイールのメッセージを積んで `PointerState.wheel`（縦と横の回転量・1 フレームだけ有効）に載せる。areka の製品コードでこれを読む所は 0。
- ボタンを放した知らせ（wintf の `OnPointerReleased`）をキャラクターの窓で受けているのは、右クリックのメニューだけ（`crates/areka/src/menu/trigger.rs` の `on_char_pointer_released`。右ボタンの解放でメニューを開く照会を始める）。
- 「返事が無ければ次のイベントを送る」の前例は 2 つ: 選択肢の `OnChoiceSelectEx` → `OnChoiceSelect`（`crates/areka-kanade/src/schedule/choice.rs` の `CascadePlan::Canonical`）と、起動の `OnFirstBoot` → `OnBoot`（`schedule/boot.rs`）。
- 生きている決まり 10（roadmap）: 話の最中に届いたマウス系の返事は、今の話を置き換える（正典どおり）。`OnSecondChange` の返事だけ捨てる（`value_replaces_active_talk`）。

## Desired Outcome

- キャラクターを 1 回クリックすると、正典どおりの Reference で `OnMouseClick`（左・右・中）か `OnMouseClickEx`（中・拡張）が SHIORI に届く。
- ホイールを回すと `OnMouseWheel` が届く。回転の量と向きは Reference2 に入る。
- ダブルクリック・ドラッグ・右クリックのメニューの今の動きは変わらない。
- 送った・送らなかったが記録で分かる。網羅台帳の 3 行が実物と合う。

## Approach

kanade のマウスの出来事に「クリック」と「ホイール」を足し、送ってよいイベントの表に名前を足す。入力の側は、キャラクターの窓の「放した」知らせとホイールの値から出来事を作る。クリックかどうかは「押してから放すまでにドラッグにならなかった」で決め、時間を待って見分けることはしない（正典はダブルクリックの前の 1 回目のクリックも送る）。`OnMouseUp` を先に送るかどうかは議題 1。

## Scope

- **In**: `OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel` の送出・kanade の表と Reference の組み立て・入力の配線・ホイールの間引き・網羅台帳の 3 行・決定論テスト（ボタンごと・ドラッグの後は送らない・ダブルクリックとの順・話の最中）。
- **Out**: `OnMouseDown`・`OnMouseDownEx`（押した瞬間）・`OnMouseUp`・`OnMouseUpEx`（議題 1 で入れると決めたときだけ In）・`OnMouseDoubleClickEx`・`OnMouseMultipleClick`・`OnMouseEnter`／`Leave`／`Hover`・`OnMouseGesture`（タッチパネルの代わりの `OnMouseWheel` も含む）・バルーンの上のクリックとホイール（バルーンのスクロールは別の話）・入力の機器の見分け（Reference6 は今と同じ決まりで入れる）。

## Boundary Candidates

- kanade の出来事と表（`crates/areka-kanade/src/msg.rs` の `MouseEventKind`・`schedule/events.rs` の表と組み立て・`schedule/steady.rs` の調停）。
- 入力の配線（`crates/areka/src/input_events/mod.rs`・間引きの `throttle.rs`・新しいファイル）。
- 右クリックのメニューとの順（`crates/areka/src/menu/trigger.rs`）。

## Out of Boundary

- wintf（ホイールも放した知らせも既にある。触らない見込み）。
- 当たり判定の解決（今の解決をそのまま使う）。
- ドラッグの判定（`crates/areka/src/input_events/drag.rs`。結果を見るだけ）。

## Upstream / Downstream

- **Upstream**: なし（今すぐ着手できる）。
- **Downstream**: `mcp-user-response`（利用者のしたこととして、クリックとホイールをエージェントへ返せるようになる）・`sakura-time-critical`（`\t` の間はマウスのイベントを止める。止める対象に 3 つが加わる）。

## Existing Spec Touchpoints

- **Extends**: なし（`input-events`・`mouse-drag-events` は完了済みで消化できない＝新しい spec）。`mouse-drag-events` の brief は「`OnMouseClick` などを足すとき、同じ `MouseEventKind` と表へ行を足す」と申し送っている。
- **Adjacent**: kanade の列で `schedule/events.rs` に触る `mcp-kanade-tools`・`anchor-tag-canon`・`balloon-link-hover`・`balloon-break-position`・`mcp-user-response`・`popup-menu-residue`・`property-query-channels`・`sakura-time-critical`・`script-security-level`・`update-check-options`。`input_events/mod.rs` に触る `collisionex-regions`・`extra-character-windows`・`balloon-markers`。

## Constraints

- 1 フレーム遅らせて見分ける作りにしない（開発者の方針）。クリックは放した時点で決まる。
- 話の最中の返事は今の話を置き換える（生きている決まり 10）。1 回のクリックの返事で話が替わるのは正典どおりで、止めるのはゴーストの側か `\t`。
- ホイールは 1 回の操作で多くの知らせが来る。SHIORI を呼びすぎない（移動の間引き `throttle.rs` と同じ考え方）。ただし回転の量は捨てずに足し合わせる。
- kanade の `msg.rs` は 926 行・`schedule/steady.rs` は 950 行で上限（1,000 行）に近い。足す前に行数を確かめ、要れば新しいファイルへ置く。
- 実機の確かめは画面のロックを外した机で（入力を送るテストはスクリーンセーバーの間は走らない）。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**: `crates/areka-kanade/src/schedule/events.rs`（793）・`msg.rs`（926）・`schedule/steady.rs`（950・返事が無ければ次を送る形にするとき）・`crates/areka/src/input_events/mod.rs`（571）・`throttle.rs`（276）か新しいファイル・`crates/areka/src/menu/trigger.rs`（573・右クリックの順を変えるとき）と各兄弟のテスト・`doc/ukadoc-coverage/ledger/shiori.toml` の 3 行（`OnMouseUp` の組を入れるなら 5 行）・`doc/COMPAT_ARCHITECTURE.md` §8（areka が決めた点の登記）。
- **規模**: S〜M（6〜10 タスク）。`OnMouseUp`・`OnMouseUpEx` を先に送る形にすると M（10〜13）。
- **先に要るもの**: なし。同じウェーブに置けない相手は Adjacent のとおり（kanade の `schedule/events.rs` と `input_events/mod.rs`）。
- **優先度の区分**: C（ukadoc の拾い残し。完了 `input-events` が範囲の外と決めた持ち越し）。利用者から見える穴（つついても喋らない）ではあるが、決めて送っていないもので、バグではない。
- **要件定義のモデル**: Fable（正典の「`OnMouseUp` に反応が無いとき」の扱い・右クリックとメニューの順・ホイールの間引きは開発者に聞く分かれ目）。
- **議題**:
  1. **`OnMouseUp` をどうするか**。正典は「`OnMouseUp` に反応しない場合に `OnMouseClick`」と書くが、areka は `OnMouseUp` を送っていない。案 A: `OnMouseUp`（と `OnMouseUpEx`）も本 spec で送り、返事が無いときだけ `OnMouseClick` を続ける（正典どおり・選択肢の 2 段と同じ形）。案 B: `OnMouseUp` は送らず、いつも `OnMouseClick` を送る（「反応が無かった」とみなす・軽い・`OnMouseUp` で返事をするゴーストは少ない見込み）。
  2. **クリックとダブルクリックとドラッグの見分け**。今のダブルクリックは OS が見分けて 2 回目の押下で届く。推しは「放した時点で、ドラッグになっていなければ `OnMouseClick` を送る。2 回目の押下では今までどおり `OnMouseDoubleClick` を送る」（待たない・正典の流れと同じ）。ダブルクリックの 2 回目の解放で `OnMouseClick` をもう 1 回送るかどうかを決める。
  3. **右ボタンのクリックとメニューの順**。右ボタンの解放は今メニューを開く。`OnMouseClick`（Reference5＝1）を送るか、送るならメニューの前か後か、返事があったらメニューを出さないか。
  4. **ホイールの送る頻度**。1 フレーム分を足し合わせて 1 回にするか、移動と同じ間引きの時間を置くか。足し合わせた量をそのまま Reference2 に入れてよいか。
- **ukadoc の照合**: ukadoc「SHIORI Event」の一覧の `OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel`・`OnMouseUp`・`OnMouseDoubleClick`・`OnMouseMultipleClick` を ukadoc MCP で確かめた。
- **覚え書きとの違い**: 覚え書きは 3 つのイベントだけを挙げるが、正典の `OnMouseClick` は `OnMouseUp` の後に続くイベントで、その `OnMouseUp` も台帳で持ち主が無い。右ボタンの解放は既にメニューが使っている。

## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）

- 前提の変化:
  - `anchor-tag-canon`（10-10）が着地し、kanade の同じファイルを触る相手が 1 つ減った。行数: `schedule/events.rs` 845・`msg.rs` 930・`schedule/steady.rs` 957（残り 43 行）・`schedule/mod.rs` 969。送ってよいイベントの表は 53 語。
  - 「返事が無ければ次のイベントを送る」の前例が 3 つになった: アンカーの `OnAnchorSelectEx` → `OnAnchorSelect`（kanade の `schedule/anchor.rs`）。選択肢と違い帳簿も期限も持たず、「どの段の応答を待っているか」だけを覚える軽い形。議題 1 の案 A（`OnMouseUp` → `OnMouseClick`）はこの形に倣える＝判断は新しいファイルに置き、`schedule/steady.rs` に足すのは応答の腕の入口の数行で済む見込み。
  - `crates/areka/src/input_events/mod.rs`（571）・`throttle.rs`（276）・`crates/areka/src/menu/trigger.rs`（573）は変わっていない。`anchor-tag-canon` が触った入力のファイル（`input_events/balloon*.rs`・`choice_drain.rs`・`shell_box*.rs`）は本 spec の範囲の外。
- 触るファイル: 棚卸㉓のとおり（案 A なら kanade に新しいファイルと、`schedule/mod.rs` の状態の欄 1 つ）。
- 規模: S〜M（6〜10 タスク）・案 A なら M（10〜13）のまま。
- 先に要るもの: なし。同じウェーブに置けない相手: `mcp-kanade-tools`（`msg.rs`・`schedule/events.rs`。案 A なら `schedule/mod.rs` も）・`inputbox-user-input`・`script-security-level`・`sakura-time-critical` ほか kanade の `schedule/events.rs` に触る spec、`input_events/mod.rs` に触る spec（Adjacent のとおり）。`mcp-kanade-tools` が `msg.rs` と `schedule/mod.rs` を分けた後に着手するのが安い。重なり 0: `mcp-reload`・`mcp-receiver-bugs`・`mcp-stdio-bridge-residue`・`check-script-arg-checks`。
- 優先度の区分: C のまま。要件定義のモデル: Fable のまま。
- 見つけた穴・古くなった記述: Adjacent の `anchor-tag-canon` は着地済み。本文 Current State の「前例は 2 つ」は 3 つ。Constraints の行数（`msg.rs` 926・`schedule/steady.rs` 950）は上の数に読み替える。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 前提の変化: **v0.3.0 から v0.1.0 へ移した**（下の理由）。行数: `schedule/events.rs` 850・`msg.rs` 930・`schedule/steady.rs` 957・`schedule/mod.rs` 969・`crates/areka/src/input_events/mod.rs` 575（`emily-ghost-verification` が scope の検査を足した）・`throttle.rs` 276・`menu/trigger.rs` 573。単発のクリックは今も送らない（`input_events/mod.rs` の `on_char_pointer_pressed` の説明）。
- 棚卸㉗で足した範囲（理由と証拠）:
  - **v0.1.0 へ移した理由**: ぱすたは × の印の左クリック（`OnMouseClick`・当たり判定「閉じる」・Reference5＝0）で終わり、ボールの左クリック（当たり判定「セキュリティボール」）でボールのトークを出す（ゴースト「窓際のぱすたさん」の `dic/window.pasta`・`dic/ball.pasta`）。areka ではどちらも起きない。
  - **`OnMouseEnter`・`OnMouseLeave` を In へ**（本文の Out から移す）: ぱすたは × の印に乗ると `\![bind,閉じる,赤,1]`、離れると `0` で × を赤くする（同じ `window.pasta`・Reference3＝スコープ・Reference4＝当たり判定の名前を見る・`Status` の talking の間は何もしない）。正典（ukadoc）: `OnMouseEnter`＝マウスが当たり判定に入った・Reference0〜6 は `OnMouseMove` と同じ並び（5 は常に 0）。`OnMouseLeave` も同じ形。入力の配線は同じ（窓ごとに「今どの当たり判定の上か」を覚え、変わったときに Leave → Enter）。3〜4 タスク増える。
  - 窓ごとの当たり判定の追跡は `shell-mouse-cursor`（新）と後の `shell-tooltip` も使う＝この spec が先に作り（案: 新しいファイル `crates/areka/src/input_events/hover_region.rs`）、後の 2 本は使うだけにする。
- 触るファイル: 本文のとおり＋追跡の新しいファイル・`schedule/events.rs` の表と組み立てに 2 つ・台帳 `shiori.toml` の 2 行（`OnMouseEnterAll`・`OnMouseLeaveAll` も入れるなら 4 行）。
- 規模: 10〜16 タスク（議題 1 が案 B なら 10〜13・案 A なら 13〜16）。
- 先に要るもの: なし。同じウェーブに置けない相手: `shell-mouse-cursor`（新・`input_events/` の追跡）・kanade の `schedule/events.rs`・`msg.rs` を触る v0.1.0 の spec（`mcp-kanade-tools`・`mcp-strict-errors`・`mcp-reload`・`script-security-level`）・`input_events/mod.rs` を触る `balloon-markers`・`collisionex-regions`（v0.2.0）。
- 優先度の区分: **A に改める**（C から。開発者の指示＝v0.1.0 のゴール・ぱすた）。要件定義のモデル: **Opus に改める**（棚卸㉔までは Fable。10-15 の基準で、`OnMouseUp` → `OnMouseClick` の 2 段は選択肢・アンカーの前例があり、先例の無い骨格に当たらない）。分割の案: なし。
- 議題を 1 つ足す: 5. `OnMouseEnterAll`／`OnMouseLeaveAll`（窓に入った・出た）も入れるか（ぱすたは使わない）。SSP で測る: 当たり判定から当たり判定へ直に移ったときの Leave と Enter の順・窓の外へ出たときの Leave。
- 古くなった記述: 本文 Out の「`OnMouseEnter`／`Leave`／`Hover`」は「`OnMouseHover`」だけになる。Downstream に `shell-mouse-cursor`・`shell-tooltip`・`pasta-in-windows-verification` を足す。
