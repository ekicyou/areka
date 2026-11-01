# 実機の確かめ（タスク 9.2・要件 5.10・8.1〜8.4）

- 走行: `ssl1`（2026-11-01 08:55:03〜09:00:59 日本時間。記録の時刻は UTC で、2026-10-31T23:55〜2026-11-01T00:00）
- 判定: 段 1〜3 は合格。段 4（本体とバルーンを DPI の違う画面に分ける）は回せなかった。ぱすた開発との約束は、開発者の裁定（2026-11-01・案 A）で「今の areka では起きない形」として 9.2 を閉じた。

## 1. 同定と機械の構成

| 項目 | 値 |
|---|---|
| コミット | `6d8d2cc3`（ブランチ `claude/kiro-start-shell-scale-linkage-773a37`）・`target\debug\areka.exe`（debug 版）・i686 の補助は同期済み |
| ぱすた | ゴーストの版 26110101（`ghost\master\dic\version.pasta` の 7 行）・書庫の sha256 `b363b88c21bc7d64ba2300bed52231373155d603ebde0f9d5381c459e4fc099e` |
| OS | Windows 11 Pro 10.0.26300 |
| DISPLAY1（主） | 物理画素で 0,0 から 2880x1800（作業領域 0,0〜2880,1704）・dpi 192（200%） |
| DISPLAY2 | 物理画素で -2560,195 から 2560x1600・dpi 144（150%） |
| 机 | 排他テストの机を持って回し、終わってすぐ返した（要件 8.1） |

画面の値は、スレッドを画面ごとの DPI 対応（`SetThreadDpiAwarenessContext(-4)`）にして `EnumDisplayMonitors`・`GetMonitorInfo`・`GetDpiForMonitor` で読んだ（6 節の 1）。

## 2. 環境変数と起こし方

`doc/pasta-real-run.md` の 4 節の起動の塊を使い、次の 2 つだけを替えた。

| 名前 | 値 |
|---|---|
| `AREKA_ROOT` | `<w>\target\pasta-verify\root` |
| `AREKA_PROFILE_DIR` | `<w>\target\pasta-verify\profile`（空。起動のイベントは `OnFirstBoot`） |
| `RUST_LOG` | `info,areka=debug,areka_ghost=debug,areka_kanade=debug`（判定の分岐の段・設計の Monitoring） |
| `AREKA_APP_SMOKE_EXIT_MS` | `1800000` |
| `NO_COLOR` | `1` |
| `PASTA_DEBUG`・`AREKA_NO_ALERT`・`AREKA_USER_ROOT` | 消す |

- ゴーストは `--ghost <w>\target\pasta-verify\root\ghost\pasta-in-windows` で直に起こした。
- MCP が SSP でなく areka を指していることは、`check_script` の答え（`OK:0 diagnostics`）で確かめた。
- 窓の移し方は SendInput のドラッグ（6 節の 2 の、直した写しの `mouse-input.ps1`）。
- 終わらせ方は MCP の `sakurascript` で `\0\-\e` を送った。終了コードは 0（`areka exit=0`、09:00:59）。

## 3. 段ごとの 4 つの値

値は MCP の `get_property` で読んだ。4 つの名前は `currentghost.scope(0).scaling`・`currentghost.balloon.scope(0).scaling`・`currentghost.scope(0).currentmonitor.dpi`・`currentghost.balloon.scope(0).currentmonitor.dpi`。

| 段 | 操作 | シェルの倍率 | バルーンの倍率 | シェルの DPI | バルーンの DPI | 本体の rect | バルーンの rect | 窓の居る画面 | 結果 |
|---|---|---|---|---|---|---|---|---|---|
| 1 起動の直後 | 最初の台詞が出終わるまで待つ | 200,200 | 200,200 | 192 | 192 | 0,336,1108,1704 | 80,452,952,1352 | DISPLAY1（200%） | 一致 |
| 2 同じ画面の中のドラッグの後 | 身体を (330,1326)→(730,1226) へドラッグ | 200,200 | 200,200 | 192 | 192 | 400,236,1508,1604 | 480,352,1352,1252 | DISPLAY1（200%） | 一致 |
| 3 拡大率の違う画面へ移した後 | (730,1226)→(-1300,1200) へドラッグ | 150,150 | 150,150 | 144 | 144 | -1548,457,-717,1483 | -1488,544,-834,1219 | DISPLAY2（150%） | 一致 |
| 4 本体とバルーンを分ける | (-1300,1200)→(-152,1200) へドラッグ（境をまたがせる） | 200,200 | 200,200 | 192 | 192 | -483,209,625,1577 | 0,325,872,1225 | どちらも DISPLAY1 | 回せなかった（5 節） |

段 1〜3 は、4 つの値がどれも窓の居る画面の拡大率と一致した（200% は `200,200` と 192、150% は `150,150` と 144）。本体の大きさは 1108 から 831 へ、画面の拡大率に合わせて変わった。

## 4. 届いたイベントと記録（要件 5.10・8.2・8.3）

### 4.1 `OnPastaShellScale` の往復

ぱすたの `ASK_SCALE`（`\![get,property,OnPastaShellScale,currentghost.scope(0).scaling,currentghost.scope(0).currentmonitor.dpi]`）は、どの段でも名前 2 つのまま kanade へ渡った。

| 時刻（UTC） | 段 | 記録の行 |
|---|---|---|
| 23:58:05.317 | 1（ボールのクリックの後の最初の黙っている秒） | `event="property_get_sent" name="OnPastaShellScale" count=2` |
| 23:58:32.317 | 2（ドラッグの終わりの後の付け直し） | 同上 |
| 23:59:14.318 | 3（移した後の付け直し） | 同上 |
| 2026-11-01T00:00:10.318 | 4 の試み（戻した後の付け直し） | 同上 |

### 4.2 倍率の変わりのイベント（150% の画面へ移したとき）

| 時刻（UTC） | 記録の行 |
|---|---|
| 23:59:07.300028 | `scope_scale_table_updated scope=0 window=Shell scaling=150,150 dpi=Some(144)`（scope=1 も同じ時刻） |
| 23:59:07.300450 | `event="scale_event_raised" id="OnShellScaling" scope=0 before=200 after=150` |
| 23:59:07.300605 | `event="scale_event_raised" id="OnShellScaling" scope=1 before=200 after=150` |
| 23:59:08.197609 | `event="scale_event_raised" id="OnBalloonScaling" scope=0 before=200 after=150` |
| 23:59:08.298670 | `[DragEndEvent] Dispatching entity=30v0 x=-1300 y=1200 cancelled=false` |

- `OnShellScaling` はスコープ 0 で届いた。窓の DPI が変わった時点（ドラッグの途中）に送られ、ドラッグの終わり（`DragEndEvent`）は別に届いた（要件 5.10）。
- 表は、イベントより先の同じフレームで書かれた。
- 200% の画面へ戻した段 4 の試みでも同じ形だった。00:00:04.358 に `OnShellScaling` の scope 0 と 1、`OnBalloonScaling` の scope 0（どれも 150→200）が出て、00:00:04.368 に `DragEndEvent` が来た。

### 4.3 ぱすたの付け直しと倍率の確かめ

- ドラッグのたびに、次の黙っている `OnSecondChange` で、ぱすたが窓あわせと重ね順の台本を出した。台本は `\0\s[1000]\1\s[10]\![move,--X=0,--Y=0,…,--option=ignore-sticky-window]\0\_u[0x3000]\![reset,zorder]\![set,zorder,s0,b0,s1]\![set,windowstate,!stayontop]\c` に `ASK_SCALE` を続けた形。出た時刻は 23:58:32・23:59:14・00:00:10 の `steady_talk origin="OnSecondChange"`。
- 倍率の変わりのイベントの後の黙っている秒に、`OnPastaScaleCheck` が名前 5 つで出た。
  - 23:59:15.214 と 00:00:11.213 に `event="property_get_sent" name="OnPastaScaleCheck" count=5`
  - 聞いた名前: `scope(0).scaling`・`balloon.scope(0).scaling`・`baseware.name`・`scope(0).currentmonitor.dpi`・`balloon.scope(0).currentmonitor.dpi`
- 倍率のずれの台詞（`＊バルーン倍率のずれ`。「吹き出しだけ」「揃えてもらえますか」を含む）は出なかった。areka の記録（`ssl1.stdout.log`）とぱすたの記録（`pasta.log`）を「揃えてもらえ」「吹き出しだけ」で探して 0 件（要件 8.3）。
- `balloon_syncscale_read`: `key=balloon.syncscale scope=all value="true" synced=true`（23:55:04.175）。

### 4.4 数

- panic: 0 件
- ERROR: 0 件
- WARN: 260 件
  - 256 件は `areka_seriko::actor::bind_arm`（ぱすたの台本の着せ替えの指示。本 spec の範囲の外）
  - 4 件は `areka_emo_atlas: bake`
  - 倍率に関わる WARN（`scale_event_not_sent`・`raise_event_not_steady`）は 0 件
- 終了コード: 0

## 5. 段 4: 本体とバルーンを DPI の違う画面に分ける（回せなかった）

本体を 2 つの画面の境にまたがせても、バルーンが本体の居る画面の作業領域の中へ押し戻され、2 つの窓が別の画面に分かれた形を作れなかった。

- 本体の rect は `-483,209,625,1577` で、中心は DISPLAY1 の上。バルーンは本体の左上から 80 の所（x=-403）に置かれるはずが、`0,325,872,1225` に置かれた。2 つの DPI はどちらも 192。
- 根拠の記録の行（2026-11-01T00:00:03.816〜00:00:10.225 に繰り返し出た）:

  ```text
  areka::placement::follow::window_move: [balloon-limit] Clamp runtime 関門: バルーン窓の書込位置を作業領域内へ補正した（寸・可視性・Z 順には触れない） scope=1 context="runtime-gate" how="pushed" entity=31v0 route=Some(BalloonFollow) from=PointPx { x: -1355, y: 209 } to=PointPx { x: 0, y: 209 } balloon_size=SizePx { w: 872, h: 900 } work_area=RectPx { left: 0, top: 0, right: 2880, bottom: 1704 }
  ```

  scope=0 のバルーンも、同じ `route=Some(BalloonFollow)` で `how="pushed"` の行が出た。バルーンが本体についていくとき（`BalloonFollow`）の書き込みの位置は、本体の居る画面の作業領域の中へ押し戻される。本体がどちらの画面に居ても、バルーンはその画面の中に収まるので、DPI の違う画面には分かれない。
- バルーンを本体から離して置く道も、今の areka には無い。`check_script` の答え:
  - `\![set,balloonoffset,-900,100]` → `"kind":"unknown_command","message":"no part of areka handles this \\! command; playback ignores it"`
  - `\![set,balloonalign,right]` → 同じく `unknown_command`
- ぱすた開発との約束（本体とバルーンが DPI の違う画面に分かれたとき、倍率のずれの台詞が出ないこと）は、分かれた形を作れないので実機では確かめていない。開発者の裁定（2026-11-01・案 A）: 今の areka ではバルーンが本体の画面へ押し戻されるので分かれた形は起きず、台詞の出る場面も無い。この段は合格とせず「起きない形」として記録し、バルーンを本体から離して置ける仕組み（`\![set,balloonoffset]` など）ができたときに確かめ直す（完了の棚卸で `/kiro-discovery` に起票）。
  - 分かれていない段 3 と段 4 の試みでは、`OnPastaScaleCheck` に答えたうえで台詞は出なかった（4.3）。
  - ぱすたの辞書は、窓ごとに「scaling × 96 ÷ DPI」で比べ、`baseware.name` が `areka` なら台詞を出さない形になっている。

## 6. SSP の基準との比べ（要件 8.4）

SSP の基準（`requirements.md` の「SSP の基準」の節。測りの記録 `ssp-measurement.md` 2 節の段 5・3 節の表）: 150% から 200% の画面へ移っても、`currentghost.scope(0).scaling` は `150,150`、`currentghost.scope(0).currentmonitor.dpi` は 192、大きさは 831×1026 のまま。`OnShellScaling` は届かない。

| 比べる所 | SSP | areka（ssl1） | 同じか・違いのわけ |
|---|---|---|---|
| `scaling` の数の形 | 「横,縦」の百分率（`150,150`） | 「横,縦」の百分率（`200,200`・`150,150`） | 同じ |
| `currentmonitor.dpi` の数の形 | 10 進の整数（192） | 10 進の整数（192・144） | 同じ |
| 画面を移ったときの `scaling` | 変わらない（利用者の設定の倍率） | 窓の居る画面の拡大率に追従して変わる（200 → 150） | 違う。areka の DPI 追従の方針（2026-07-18 開発者）による。`scaling` は ukadoc の「実効拡大率」で、DPI 追従の倍率 × 表示の倍率（今は 100%） |
| 画面を移ったときの大きさ | 831×1026 のまま | 1108 → 831（幅） | 違う。同じく DPI 追従の方針による |
| `OnShellScaling`・`OnBalloonScaling` | 届かない（倍率が変わらないため） | 窓の DPI が変わった時点で、スコープ 0（と 1）に届く | 違う。areka は倍率がじっさいに変わるので、正典の「倍率が変更された際に発生」に従って送る |
| `\![get,property]` の往復 | 指定のイベントが Reference に値を持って返る | 同じ（`OnPastaShellScale` が count=2、`OnPastaScaleCheck` が count=5） | 同じ |
| 倍率のずれの台詞 | 出ない（シェルとバルーンが同じ倍率のまま） | 出ない | 同じ |

## 7. 気づいたこと

1. **手順書の画面の測り方が pwsh では効かない。** `doc/pasta-real-run.md` の 3 節の塊は `SetProcessDpiAwarenessContext(-4)` を呼ぶが、pwsh のプロセスはすでに DPI 対応が決まっているので失敗し、論理座標と仮の DPI が返る。DISPLAY2 も 192 と出て、位置も -3413,260 / 3413x2133 という論理の値になった。`SetThreadDpiAwarenessContext(-4)` で測ると、正しい値（-2560,195 / 2560x1600 / 144）が出た。
2. **`tools\mouse-input.ps1` も同じ欠陥を持つ。** 121 行で `SetProcessDpiAwarenessContext(-4)` を呼ぶため、物理座標が丸められ、読み戻しが 1 画素ずれて失敗する（`error cursor at x=553 y=1599, wanted x=554 y=1600 (input refused or point off screen)`）。この走行では、`SetThreadDpiAwarenessContext` に替えた写しを scratchpad に置いて使った。リポジトリの道具は直していない（範囲の外。直すなら起票する）。
3. **起動の直後に `OnSecondChange` がぱすたに届かなかった。** 起動の台詞が 23:55:16 に終わり（`steady_talk_done`）、23:55:46 に `OnBalloonTimeout` が送られた。それでも、ボールをクリックするまで（23:57 台）`OnSecondChange` の応答の台本は出ず、ぱすたの記録にも無い。そのため最初の `OnPastaShellScale` は、ボールのクリックの後の 23:58:05 だった。起動の段の 4 つの値は MCP で読めたが、ぱすたが値を聞いたのはボールを押した後。範囲の外として申し送る。
4. **ぱすたの最初の画面はボールだけ。** `OnFirstBoot` の台本はボールを叩くよう頼み、身体は出ない。窓の枠の白地は当たりの外なので、身体をドラッグできるのはボールのクリック（23:57 台・`OnMouseClick`）の後になる。段 2 以降のドラッグはその後に行った。

## 8. 置き場

- 記録: `<w>\target\pasta-verify\logs\ssl1.stdout.log`・`ssl1.stderr.log`・`ssl1.pid.txt`・`ssl1.exitcode.txt`
- 撮った物: `<w>\target\pasta-verify\shots\ssl1\`（`1-boot.png`・`1b-body.png`・`1c-after-ball.png`・`4-straddle.png`）
- ぱすたの記録: `<w>\target\pasta-verify\root\ghost\pasta-in-windows\ghost\master\profile\pasta\logs\pasta.log`

（`<w>` はワークツリー `C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a`）
