# 実機の確かめ（mouse-click-wheel-events タスク 6）

ぱすたの手順書 `doc/pasta-real-run.md` の段 3（撫で・× に乗る・ボール）と段 7（× で終わる）を回した記録。右クリックのメニュー・ダブルクリック・ドラッグの確かめ（要件 12.5）と、設計の R4 の確かめを足してある。

以下の `<w>` はワークツリー `C:\home\maz\git\areka\.claude\worktrees\areka-p0-offline-surface-dump-0006ba` を指す。撮った物と記録はどれも `<w>\target\` の下にある。

## 1. 同定と機械の構成

| 項目 | 値 |
|---|---|
| コミット | `f448c4b3`（debug 版 `cargo build -p areka`・i686 の補助は sync 済み） |
| ぱすたの書庫 | sha256 `4c354864760c107a88f7ffd32df875bfbc6af4e22d778d54fb2ce68f1189c12b`（2026-10-24 に取り寄せ。縦書きのバルーンがゴーストのフォルダの中 `balloon\pasta_tategaki` に入る版） |
| OS | Windows 11 Pro 10.0.26300 |
| 画面 | DISPLAY1（主）: 0,0 から 2880x1800・200%。DISPLAY2: -2560,195 から 2560x1600・150% |
| 記録の水準 | `RUST_LOG=info,kanade=debug,areka::input_events=debug,areka::boot_resolve=debug`（`AREKA_APP_SMOKE_EXIT_MS` は run1 が 1500000、r4 が 600000。`PASTA_DEBUG` は消した） |
| 画面の操作 | `tools/mouse-input.ps1`（SendInput）。ダブルクリックだけは、同じ API（SetCursorPos と mouse_event）で 2 回を 90 ms の間隔で押す一時の道具を使った（`mouse-input.ps1` に二度押しの動作が無いため） |

## 2. 走行

| 回 | 根 | 日時（日本時間） | pid | 終わり方 |
|---|---|---|---|---|
| run1 | `<w>\target\pasta-verify\root`（手順書 3 節の形・記憶は空） | 2026-10-24 10:20:58〜10:28:49 | 25400 | × のクリック → 終わりの台詞 → `app_exit origin=KanadeStopped(Quit)`・終了コード 0 |
| r4 | `<w>\target\mcwe-r4\root`（run1 の根の写しにぱすたの辞書を 2 か所足した検体・3.3 節） | 2026-10-24 10:29〜10:30 | 17220 | 確かめの後に、自分が起こしたこの番号の areka だけを外から止めた |
| run2 | run1 と同じ根（記憶は run1 の続き） | 2026-10-24 10:57〜10:58:22 | 37756 | ダブルクリックの確かめ（3.4 節）の後、× のクリック → `app_exit origin=KanadeStopped(Quit)`・終了コード 0 |

- 記録は `<w>\target\pasta-verify\logs\run1.stdout.log`・`run2.stdout.log` と `<w>\target\mcwe-r4\logs\r4.stdout.log`。
- ぱすたの記録は `<w>\target\pasta-verify\root\ghost\pasta-in-windows\ghost\master\profile\pasta\logs\pasta.log`（run1 と run2 の両方が書き足した）。
- 撮った物は `<w>\target\pasta-verify\shots\run1\`・`shots\run2\` と `<w>\target\mcwe-r4\shots\r4\`。
- run2 の数: panic 0、ERROR 0。
- run1 の数: panic 0、ERROR 0、WARN 548。WARN はどれも `areka_seriko`（mustselect の脱衣指示を無視）と `areka_emo_atlas`（全透明の element）の行で、ぱすたの資産に由来する。この spec のクリックと出入りの行は 1 つも無い。

### 開発者の手の操作について

run1 では、私（AI）が画面を操作し始める前に、在席の開発者の手と見られる操作が記録に入っている。

- 01:21:51 に窓の枠（scope 1）をドラッグした。
- 01:21:54 にボールをクリックした。
- 01:22:33〜35 に × に乗って離れた。
- 01:22:36 にボールをダブルクリックした。

どれも私の送ったものではない。初回の起動で身体が映ったのは、01:21:54 のこのクリックである。私が打ったボールのクリックは 01:25:10 で、ボールのトークが出た。下の表では、どちらが打ったかを分けて書く。

## 3. 結果

### 3.1 段 3・段 7（要件 12.1〜12.3）

| 確かめ | 結果 | areka の記録（`mouse_event_sent`・時刻は UTC） | ぱすたの記録（`pasta.log` の要求） | 撮った物 |
|---|---|---|---|---|
| × に乗ると赤くなる（12.1・件 10） | 同じ（私の操作） | `01:24:24.223903 id="OnMouseEnter" scope=0 region="閉じる"` → 返った台本は `\![bind,閉じる,赤,1]\e` | `01:24:24.224450 OnMouseEnter r3=0 r4=閉じる` | `3-06-on-close.png`（赤い ×）・`3-0y-strip.png` の左 |
| × から離れると戻る（12.1） | 同じ（私の操作） | `01:24:27.197879 id="OnMouseLeave" scope=0 region="閉じる"` → 返った台本は `\![bind,閉じる,赤,0]\e` | `01:24:27.198837 OnMouseLeave r3=0 r4=閉じる` | `3-07-left-close.png`（灰色の ×）・`3-0y-strip.png` の右 |
| 初回の起動でボールのクリックで身体が映る（12.2・件 12） | 同じ（クリックは開発者の手と見られる・私は打っていない） | `01:21:54.054460 id="OnMouseUp" scope=0 region="セキュリティボール" ref5="0"`、`01:21:54.055543 id="OnMouseClick" … ref5="0"` | `01:21:54.058590 OnMouseClick r3=0 r4=セキュリティボール r5=0` | `3-01-on-close.png`（身体が映り、ボールのトーク「繋がってる」が出ている。私が最初に撮った絵） |
| ボールのクリックでボールのトークが出る（12.2・件 11） | 同じ（私の操作） | `01:25:10.456443 id="OnMouseUp" … region="セキュリティボール" ref5="0"`、`01:25:10.457574 id="OnMouseClick" … ref5="0"` | `01:25:10.459912 OnMouseClick r3=0 r4=セキュリティボール r5=0` | `3-09-ball-talk.png`（「そこのボールで、私の姿を映してます。」） |
| 撫でと自然なトークが回る（12.2） | 同じ（私の操作） | 撫での反応 `label=Ghost:OnMouseMove` の台本が 01:25:47・01:25:55・01:26:01・01:26:08 に出た。時間で出る話（`OnSecondChange`）の台本も出た | `OnMouseMove` 4 件 | `3-10-stroke.png` |
| × のクリックで終わりの台詞の後に終わる（12.3・件 13） | 同じ（私の操作） | `01:28:38.634615 id="OnMouseEnter" … region="閉じる"`、`01:28:41.549674 id="OnMouseUp" … region="閉じる" ref5="0"`、`01:28:41.550542 id="OnMouseClick" … region="閉じる" ref5="0"` → 台本の末尾は `\_w[300]\-\e` → `01:28:49.069316 app_exit origin=KanadeStopped(Quit) closed=4`・`areka exit=0` | `01:28:38.635219 OnMouseEnter r4=閉じる`、`01:28:41.552207 OnMouseClick r3=0 r4=閉じる r5=0` | `7-01-on-close.png`（赤い ×）・`7-02-farewell.png`（「もう閉めちゃうんですか。それでは、」）・`7-03-after.png` |

注:
- 話の最中に × に乗ったとき（01:22:42 の Enter）は、ぱすたが自分で反応を止める（`window.pasta` は `Status: talking` のときホバーを扱わない）。areka は送っていて、記録に行がある。ぱすたの作りどおりで、崩れではない。
- ボールの真ん中の半透明の所（580,943）を押すと、クリックは奥の窓の枠に落ちた（`01:24:48 id="OnMouseClick" scope=1`）。areka は α が 0.5 以上の画素だけを当たりにする（2026-10-22 開発者の裁定）ので、方針で受けない違いである。輪の線の上（607,909）を押すと、ボールに当たった。

### 3.2 メニュー・ダブルクリック・ドラッグ（要件 12.5）

| 確かめ | 結果 | areka の記録 | 撮った物 |
|---|---|---|---|
| 右クリックでメニューが今のとおり出る | 同じ | `01:27:57.065950 id="OnMouseUp" scope=0 region="胸右" ref5="1"`、`01:27:57.066877 id="OnMouseClick" … ref5="1"`、`01:27:57.076763 [menu] shown event="menu_shown" scope=0 items=15`。ぱすたは右のクリックに台本を返さない（`window.pasta` は button≠0 で nil）ので、メニューの照会がそのまま効いた | `6-01-menu.png`・`6-02-menu-closed.png`（外を押して閉じた後） |
| ダブルクリックが今のとおり効く | 同じ。run1 では入力の層まで確かめ、SHIORI に届くことは run2 で確かめた（3.4 節） | `01:26:51.461396 [handle_double_click_message] Double-click detected … double_click=Left`、1 回目の解放は `OnMouseUp`・`OnMouseClick`（胸右）、2 回目の解放は `01:26:51.524756 mouse_click_not_sent reason="second_of_double" scope=0 button="0"`（2 段目を送らない・設計どおり） | `3-11-doubleclick.png` |
| ドラッグが今のとおり効く | 同じ | `01:28:20.541486 [DragStartEvent]`、`01:28:21.507950 mouse_click_not_sent reason="drag" scope=0 button="0"`、`01:28:21.508544 [DragEndEvent] … cancelled=false`、`01:28:21.509094 sticky_follow trigger="drag" partners=[1] dx=200 dy=0` | `4-01-after-drag.png` |

### 3.3 R4（右のクリックの台本とメニューの抑止）

検体は run1 の根の写しで、ぱすたの辞書に次の 2 か所だけを足した（`<w>\target\mcwe-r4\root\ghost\pasta-in-windows\ghost\master\dic\`）。

- `window.pasta` の on_mouse_click: スコープ 0 で button=="1" のとき、`act:raw_script([=[\0R4-right-click-script\e]=])` を返す。
- `menu.pasta`: `REG["sakura.popupmenu.visible"] = function(act) return "0" end`。

ボール（250,1636）を右クリックした結果:

```text
2026-10-24T01:30:02.750771Z  INFO kanade: … event="mouse_event_sent" id="OnMouseUp" scope=0 region="セキュリティボール" ref5="1"
2026-10-24T01:30:02.751640Z  INFO kanade: … event="mouse_event_sent" id="OnMouseClick" scope=0 region="セキュリティボール" ref5="1"
2026-10-24T01:30:02.753834Z  INFO areka::log::script: \0R4-right-click-script\e\e ghost=窓際のぱすたさん label=Ghost:OnMouseClick
2026-10-24T01:30:02.759027Z  INFO areka::menu::captions: [menu] suppressed by popupmenu.visible event="menu_suppressed_by_visible" scope=0 id="sakura.popupmenu.visible"
```

- `menu_shown` は 0 件、`menu_resource_query_unanswered`（「全件既定名」の警告）も 0 件だった。
- 台本を出し、メニューを抑えるまでは、9 ms で済んだ。枠の 1,000 ms は食わない。
- 撮った物 `<w>\target\mcwe-r4\shots\r4\r4-01-after-rightclick.png`・`r4-02-crop.png`。バルーンに台本の字が出ていて、メニューは出ていない。
- 結果: 合格。

### 3.4 ダブルクリックが SHIORI に届いたか

結論: 届く。run1 の記録だけでは示せなかったので、run2 で窓の枠（scope 1）をダブルクリックし、ぱすたが受けて台本を返すことを確かめた（下の「run2 での確かめ」）。崩れ（退行）は無い。

#### run1 の記録だけでは示せなかった理由

1. ぱすたの記録は、台本を返した要求だけを残す。全部の要求を残すわけではない。
   - `pasta.log` の要求は 24 件しかない。一方で `OnSecondChange` は毎秒送っている（約 8 分の走行）。
   - 届いたのに記録に無い要求がある。areka は `OnMouseEnter` を 25 回送ったが、`pasta.log` にあるのは、台本（bind）を返した「閉じる」の 5 件だけ。セキュリティボール・胴・胸右に入ったときは、ぱすたが nil を返すので残っていない。
   - `OnMouseClick` も、areka が送った 7 回のうち、記録にあるのは台本を返した 5 件だけ。scope 1 の 01:24:48 と胸右の 01:26:51 は無い。
   - ぱすたの `OnMouseDoubleClick` の受け口（`tategaki.pasta` 201 行）は `if var.r3 ~= "1" then return nil end` で、窓（scope 1）でだけ台本を返す。今回のダブルクリックは scope 0（胸右とボール）なので nil になり、届いても `pasta.log` には残らない。記録に無いのは、届いていても同じ形になる。
2. areka の側にも、このイベントを送ったと書く行は無い。これはコードがそう作ってある。
   - 入力の層の `crates/areka/src/input_events/mod.rs` の `send_double_click`（288 行）は、送れなかったときに warn を出すだけで、送れたときの行は無い。この spec はこの経路を変えていない（設計の「今のテスト」）。
   - kanade の `crates/areka-kanade/src/schedule/steady.rs` の `on_mouse`（81 行〜）は、`Move`・`DoubleClick`・`DragStart`・`DragEnd` の GET を組み立てるが、記録の行を出さない。`mouse_event_sent` を出すのは、この spec が足した解放・ホイール・出入りの系だけ（設計の記録の表）。
   - 同じ道を通る `OnMouseMove` も送った行が無い。それでも撫での台本は 4 回返っているので、行が無いことは届かなかったことを意味しない。
3. 入力の層は二度押しを拾っている。`Double-click detected`（01:22:36.190315・01:26:51.461396）が出ていて、その押下の解放で kanade が `second_of_double` を記録した。この旗は、二度押しとして届いた押下にだけ立つ。
   - この旗を立てるのは、押下の受け手 `on_char_pointer_pressed`（`input_events/mod.rs` 522 行〜）の 557 行の `click::note_press` である。同じ受け手が続けて、左のダブルクリックなら 596 行で `send_double_click` を呼ぶ。
   - 間で抜ける道は 3 つあり、どれもこの走りには当たらない。
     - 箱の選択肢を押したとき（`by_box`）。そのときバルーンに選択肢は無かった。
     - 結線の前。前後のクリックは送れていた。
     - scope が取れないとき。同じ窓のクリックは scope=0 で送れていた。
   - 送れなかったときの `mouse_send_failed` の行は 0 件だった。

#### run2 での確かめ（2026-10-24 10:57〜10:58 JST）

ぱすたは scope 1 のダブルクリックにだけ台本（「縦書き試作」）を返す。そこで窓の枠の白地（720,432）をダブルクリックした。

```text
areka（run2.stdout.log）
2026-10-24T01:57:40.905725Z  INFO kanade: … event="mouse_event_sent" id="OnMouseUp" scope=1 ref5="0"
2026-10-24T01:57:40.907331Z  INFO kanade: … event="mouse_event_sent" id="OnMouseClick" scope=1 ref5="0"
2026-10-24T01:57:40.964602Z  INFO wintf::ecs::window_proc::mouse_dblclick_wheel: [handle_double_click_message] Double-click detected window_entity=29v0 target_entity=42v0 double_click=Left x=163 y=295
2026-10-24T01:57:40.975197Z  INFO kanade: 応答にスクリプト——再生起動 event="steady_talk" talk_id=4 origin="OnMouseDoubleClick"
2026-10-24T01:57:40.981279Z  INFO areka::log::script: … label=Ghost:OnMouseDoubleClick
2026-10-24T01:57:41.005858Z  INFO kanade: … event="mouse_event_sent" id="OnMouseUp" scope=1 ref5="0"
2026-10-24T01:57:41.006761Z  INFO kanade: ダブルクリックの 2 回目の解放——クリックの段は送らない event="mouse_click_not_sent" reason="second_of_double" scope=1 button="0"

ぱすた（pasta.log）
2026-10-24T01:57:40.974685Z DEBUG pasta::shiori: ### SHIORI request ###
ID: OnMouseDoubleClick
Reference0: 81
Reference1: 147
Reference2: 0
Reference3: 1
Reference4:
Reference5: 0
```

- ぱすたが `OnMouseDoubleClick`（Reference3＝1・Reference5＝0）を受け、台本を返した。kanade はそれを `origin="OnMouseDoubleClick"` の話として再生した。
- 画面には「縦書き試作」（「長い長い、記憶の糸。生まれてから、続いてる、一本の長い記憶。」）が出た。撮った物は `<w>\target\pasta-verify\shots\run2\d-01-frame-dblclick.png`。
  - この絵の右下には、ほかのセッションの Windows の通知の欄（Claude の通知）が写っている。ぱすたの窓とバルーンには重なっておらず、確かめには効かない。
- 解放の列は設計どおりだった。1 回目の解放は OnMouseUp → OnMouseClick、2 回目の解放は OnMouseUp の後に `second_of_double` で 2 段目を送らない。

続けて、ぱすたの身体（scope 0・1107,864）でもダブルクリックした。

```text
2026-10-24T01:57:59.921032Z  INFO kanade: … event="mouse_event_sent" id="OnMouseUp" scope=0 region="胸右" ref5="0"
2026-10-24T01:57:59.922036Z  INFO kanade: … event="mouse_event_sent" id="OnMouseClick" scope=0 region="胸右" ref5="0"
2026-10-24T01:58:00.026659Z  INFO wintf::…: [handle_double_click_message] Double-click detected window_entity=27v0 target_entity=40v0 double_click=Left x=550 y=727
2026-10-24T01:58:00.054441Z  INFO kanade: … event="mouse_event_sent" id="OnMouseUp" scope=0 region="胸右" ref5="0"
2026-10-24T01:58:00.055510Z  INFO kanade: … event="mouse_click_not_sent" reason="second_of_double" scope=0 button="0"
```

- 撮った物は `<w>\target\pasta-verify\shots\run2\d-02-body-dblclick.png`。scope 0 ではぱすたが nil を返すので、台本は出ない（ぱすたの作りどおり）。
- その後 × をクリックした。`01:58:14.613361 id="OnMouseUp" … region="閉じる"`・`01:58:14.614159 id="OnMouseClick" … region="閉じる"` の後、`01:58:22.144121 app_exit origin=KanadeStopped(Quit)` で終わり、`areka exit=0` だった。panic 0、ERROR 0。

## 4. 崩れた所と範囲の外

| 番号 | 症状 | 根拠 | 扱い |
|---|---|---|---|
| 1 | 新しいぱすたの書庫を、手順書 3 節の `Open-Sample 'pasta-in-windows'`（`nar-sample-path`）が入れた後に exit=2 で断る。出た文は「installed ["ghost/pasta-in-windows", "ghost/pasta-in-windows/balloon/pasta_tategaki"] but the registry declares ghost/pasta-in-windows」 | `crates/sample-ghost-kit/src/lib.rs` 184 行の登記の `balloons: &[]` が、ゴーストの中に入るバルーンを知らない（2026-10-24 の書庫は `install.txt` に `balloon.directory` を持つ） | この spec の範囲の外。今回は道具が入れ終えたフォルダ `<w>\target\nar-samples\manual\pasta-in-windows\ghost\pasta-in-windows` を、そのまま根へ写して回した。完了の棚卸で扱う（sample-ghost-kit の登記か、手順書 3 節を直す） |
| 2 | ボールの半透明の所を押すと、奥の窓の枠に当たる | 3.1 節の注 | 方針で受けない（当たりは α ≥ 0.5・2026-10-22 開発者の裁定） |
| 3 | run1 の記録だけでは、ダブルクリックが SHIORI に届いたことを示せなかった（ぱすたの記録は台本を返した要求だけを残し、areka はこのイベントの送った行を出さない） | 3.4 節 | run2 で確かめ済み（`pasta.log` の `ID: OnMouseDoubleClick`・kanade の `origin="OnMouseDoubleClick"`・`d-01-frame-dblclick.png`）。崩れではない |

## 5. 判定

| 要件 | 判定 | 判定した人 |
|---|---|---|
| 12.1 × に乗ると赤・離れると戻る | 同じ | AI（記録と撮った物）。目視の最終の判定は開発者 |
| 12.2 ボールのトーク・初回の身体・撫でと自然なトーク | 同じ（初回の身体は開発者の手と見られるクリックで映った） | 同上 |
| 12.3 × で終わりの台詞の後に終わる | 同じ（終了コード 0） | 同上 |
| 12.4 記録を並べて残す | この文書 | — |
| 12.5 メニュー・ダブルクリック・ドラッグ | 同じ（ダブルクリックが SHIORI に届くことは run2 で確かめた・3.4 節） | 同上 |
| R4 | 合格 | 同上 |
