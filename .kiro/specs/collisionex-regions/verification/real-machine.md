# 実機の記録: collisionex-regions（タスク 6.1・要件 6.1・6.3・8.5・8.6・8.7）

## 1. 走行の条件

| 項目 | 値 |
|---|---|
| 版 | コミット `de561dd1d`（コードのタスクはすべて済み）から組んだ debug 版 `areka.exe`（x64）と `shiori-host32-helper.exe`（i686） |
| 根 | `<ワークツリー>\target\cxr-signoff\root`（`real-machine-run.ps1 -Prepare` で `nar-sample-path claudia` の展開から写したもの）。製品のコードとリポジトリの検体は無改変・根の写しへの手当ても無し |
| 検体 | クローディア（`sample-ghost-kit` の `claudia`・YAYA・32bit） |
| 記憶 | 起動の前に `profile` とゴーストの `profile\areka` を消した |
| `RUST_LOG` | `info,areka=debug,kanade=trace,wintf::ecs::drag=debug,areka_emo_present::presenter::hit=debug`（`hit_region_client` の `debug!` は `areka=debug` の前方一致でも開くが、的を明示した） |
| 安全弁 | `AREKA_APP_SMOKE_EXIT_MS`（走行の値は 3 章） |
| 起動 | `.kiro/specs/collisionex-regions/real-machine-run.ps1 -Run R1`（完了 `areka-P0-mouse-drag-events` の同名の道具の写し）。記録は `target\cxr-signoff\run-R1.log` |
| 操作 | `.kiro/specs/collisionex-regions/real-machine-drive.ps1 -AppPid <起こした pid>`（`SendInput`・絶対座標・仮想デスクトップ） |
| 机 | 排他テストの机を進行役が取ってから起動し、走行の後すぐ返した（要件 8.7） |

## 2. 点の求め方

クローディアの `surface0`（`surface0.png` は 333×500）の当たり判定（`surfaces.txt` から逐語）:

```
collisionex0,Fan,polygon,212,209,250,206,280,222,287,243,235,256
collisionex1,Head,rect,106,31,225,108
collisionex2,Face,rect,125,110,206,165
collisionex3,Bust,rect,131,188,200,238
collisionex4,Hair,rect,69,138,125,288
collisionex5,Hair,rect,200,138,250,206
collisionex6,Foot,ellipse,142,472,163,494
collisionex7,Foot,ellipse,163,472,184,494
collisionex8,Skirt,polygon,54,400,267,400,290,446,265,472,60,472,35,444
```

| 点 | サーフェスの座標 | 当たる形 | 期待する名前 |
|---|---|---|---|
| Head | 165,70 | `collisionex1` の `rect` の中だけ | `Head` |
| Bust | 165,213 | `collisionex3` の `rect` の中だけ | `Bust` |
| Skirt | 160,436 | `collisionex8` の `polygon` だけ（`rect` はどれも y≤288） | 空 |
| Foot | 152,483 | `collisionex6` の `ellipse` だけ | 空 |
| Fan | 262,232 | `collisionex0` の `polygon` だけ（`Hair` の `rect` は x≤250） | 空 |

- どの点も `surface0.png` の不透明な画素（A=255）で、窓の当たりが透けない（System.Drawing で読んだ）。
- 表示の拡大率: 主の画面は 2880×1800・DPI 192（拡大率 2）。道具は Per-Monitor v2 で物理の点を扱い、本体の窓の大きさから k = 幅÷333（高さ÷500 と一致するものだけを候補にする）を求め、画面の点 = 窓の左上 + サーフェスの座標 × k とした。記録の `hit_region_client` の `k_ratio` と照らす。
- 本体の窓: pid が起こした areka で、333:500 の比の見えている窓のうちいちばん右（相方は本体の左隣に並ぶ）。送る前に、画面の点がその窓の矩形の中にあることを確かめ、点へ動かして 150 ms 待った後に `WindowFromPoint` の最上位の窓が選んだ本体の窓そのもの（同じ pid のバルーンの窓でもない）であることを確かめる（どちらかが違えばボタンも撫でも送らずに止める・2 段にした理由は 3 章の 1 回目）。
- 道具は送る前に、Per-Monitor v2 になっていること・`INPUT` の大きさが 40・共用体の位置が 8（x64）であることを確かめ、違えば止まる（3 回目の誤りの再発止め）。また、始めと各操作の直前に、`*.scr` のプロセスが無いことと、入力の desktop（`OpenInputDesktop` の名前）が `Default` であることを確かめ、違えば止まる（4 回目の再発止め・4 回目の後の空の走りで、動いていた `Mystify.scr` と desktop の名前 `Screen-saver` をどちらも捉えて止まることを確かめた）。
- ドラッグの距離 80 物理点は、wintf のドラッグの閾値（`DragConfig::default` の 5 物理点・キャラクター窓は `areka-placement` の `spawn.rs` で既定のまま・`check_threshold` は距離の 2 乗で比べる）を超える。
- 撫での往復（±8 物理点＝±4 サーフェスの点）の端も、同じ形の中にあり（Head・Bust は rect、Skirt・Fan は polygon、Foot は ellipse の中で rect の外）、`surface0.png` の不透明な画素であることを確かめた。
- 起動の済みは、記録に `id=OnFirstBoot` か `id=OnBoot` が出たことで見る（記憶を消すので OnFirstBoot が来て OnBoot は飛ばされる・2 回目の誤り）。自動終了は既定の 300000。
- 撫で: 点の左右 ±8 物理点を 40 ms ごとに 30 回往復。ドラッグ: 点で押し、10 回で 80 物理点動かして離す（左・右を交互にして窓をもとの近くへ戻す）。順は 撫で Head → Skirt → Foot → Fan、ドラッグ Bust → Skirt → Head → Foot。

## 3. 走行

### 1 回目（2026-11-17・入力は送れずに止まった）

- 起動の前に `netstat -ano` で 127.0.0.1:9276 を握るものが無いことを確かめた。
- `real-machine-run.ps1 -Run R1 -ExitMs 120000`。`runs.txt`（逐語）:

```
R1 pid=30108 start=2026-11-17T18:37:14.1920454+09:00 exit_ms=120000
R1 exit=0 end=2026-11-17T18:39:15.1217805+09:00
```

- `OnBoot` の後に、pid 30108 のプロセス名が `areka` であることを確かめてから操作の道具を動かした。候補の窓は 2 つ（`1548,704 666x1000 k=2` と `2214,704 666x1000 k=2`）で、右の窓を本体とした。拡大率は見込みどおり 2。
- 最初の点（撫で Head・画面 `2544,844`）の手前の確かめで、`WindowFromPoint` の窓の持ち主が `explorer.exe`（pid 27684）と出て、道具は何も送らずに止まった（逐語: 「Head の点 2544,844 の窓の持ち主が pid=27684（areka の pid=30108 でない）。送らずに止める」）。areka は 120 秒の自動終了で exit=0。
- 原因（読み取りだけで調べた）: areka のキャラクター窓の拡張スタイルは `0x2800B0`（`WS_EX_NOREDIRECTIONBITMAP`・`WS_EX_LAYERED`・`WS_EX_TOOLWINDOW`・`WS_EX_TRANSPARENT`・`WS_EX_ACCEPTFILES`）。`WS_EX_TRANSPARENT` 付きの層の窓は `WindowFromPoint` に映らないので、マウスが窓の上に来る前の確かめが「下の窓（デスクトップ）」を返した。点の誤りではなく、道具の確かめの作り方の誤り。
- 直し（机を返した後）: areka は wintf の `ecs/clickthrough/monitor.rs` の監視の糸が `GetCursorPos` を 12 ms ごとに読み、`controller.rs` がカーソルの下の画素の当たりで `WS_EX_TRANSPARENT` を付け外しする（不透明な画素の上でだけ外れて、窓にマウスの知らせが届く）。道具の確かめを 2 段にした: ⑴ 画面の点が、この pid の窓から選んだ本体の窓の矩形の中にあること（外なら何も送らない） ⑵ 点へ動かして 150 ms 待った後、`WindowFromPoint` の持ち主がこの pid であること（違えばボタンも撫でも送らない）。完了 `areka-P0-mouse-drag-events` の実機は開発者の手で操作しており、`SendInput` の道具は無かったので、倣う先は無い。

### 2 回目（2026-11-17・直した道具・入力は送れずに終わった）

- 起動の前に 127.0.0.1:9276 を握るものが無いことを確かめた。1 回目の記録は `run-R1-first.log` に写して残した。
- `runs.txt`（逐語）:

```
R1 pid=9036 start=2026-11-17T18:41:58.1308942+09:00 exit_ms=120000
R1 exit=0 end=2026-11-17T18:43:58.8851056+09:00
```

- 起動の済みを待つ待ちが `id=OnBoot` だけを探していた。クローディアの起動は `id=OnFirstBoot`（記憶を消してあるため）で、そのあと OnBoot は送られない（「起動種別にスクリプト——OnBoot をスキップし」）。待ちが空振りのまま約 100 秒を使い切り、操作の道具を動かしたのは 18:43:56（自動終了の 2 秒前）。窓はもう閉じかけで、道具は「333x500 の比の窓が pid=9036 に無い」で何も送らずに止まった。pid 9036 のプロセス名が `areka` であることは動かす前に確かめた。
- 読み込みの記録は 1 回目と同じ（まだ働かない形の記録 1 行・surface28 の `base` の 3 行）。入力を送っていないので、`hit_region_client` の行は 0 件。
- 次の走行の手当て（机を返した後）: 起動の済みを `id=OnFirstBoot` でも見る。自動終了を 300000（既定）に戻して、道具の時間に余裕を持たせる。

### 3 回目（2026-11-17・入力は送れずに止まった）

- 起動の前に 127.0.0.1:9276 を握るものが無いことを確かめた。2 回目の記録は `run-R1-second.log` に写して残した。自動終了は既定の 300000。
- 起動の済みは `id=OnFirstBoot` で 2 秒後に見え、その 6 秒後に pid 31540 のプロセス名が `areka` であることを確かめてから操作の道具を動かした（18:45:21）。候補の窓は 1・2 回目と同じ 2 つ（`1548,704` と `2214,704`・666x1000・k=2）。
- 確かめの 1 段目（点が本体の窓の矩形の中）は通った。2 段目のために点へ動かす最初の `SendInput` が 0 を返し、道具は「SendInput failed」で止まった。入力は 1 件も送っていない。
- 原因（道具）: C# の `INPUT` の構造体に余分な `pad1`・`pad2` を足していたので、`Marshal.SizeOf` が 48 になる（x64 の `INPUT` は 40）。`SendInput` は大きさの違う構造体を受け付けない。1・2 回目は `SendInput` まで来なかったので、これは今回はじめて見えた。直すのは机を返した後（`pad1`・`pad2` を除く）。大きさ 48 と 40 は、入力を送らない別の小さな C# で確かめた。
- areka は 300 秒の自動終了で exit=0（`R1 exit=0 end=2026-11-17T18:50:02.8083700+09:00`）。areka と補助プロセスは残っていない。読み込みの記録は 1・2 回目と同じ（まだ働かない形の記録 1 行・surface28 の `base` の 3 行）。`hit_region_client` の行は 0 件。

### 4 回目（2026-11-17・スクリーンセーバーの間で入力は送れなかった）

- 起動の前に 127.0.0.1:9276 を握るものが無いことを確かめた。3 回目の記録は `run-R1-third.log` に写して残した。自動終了は既定の 300000。
- `id=OnFirstBoot` が 1 秒後に見え、その 6 秒後に pid 29416 のプロセス名が `areka` であることを確かめてから道具を動かした（18:53:01）。送る前の確かめ（Per-Monitor v2・`INPUT` 40・共用体 8）は通り、候補の窓は前と同じ 2 つ（`1548,704` と `2214,704`・666x1000・k=2）。
- 確かめの 1 段目は通り、2 段目のために点へ動かす最初の `SendInput` が 0 を返して「SendInput failed」で止まった。入力は 1 件も送っていない。
- 原因: その時、スクリーンセーバー（`Mystify.scr`・pid 25216）が動いていた（`tasklist` で確かめた）。スクリーンセーバーの間は別のデスクトップに切り替わっていて、`SendInput` は通らない。3 回目の失敗も、`INPUT` の大きさの誤りに加えて同じ理由だった見込みがある（3 回目はスクリーンセーバーを確かめていない）。スクリーンセーバーは起こしたものではないので止めていない。次は開発者が席にいてスクリーンセーバーが止まっている時に走らせ、道具を動かす前に `Mystify.scr` などが無いことを確かめる。
- areka は 300 秒の自動終了で exit=0（`R1 exit=0 end=2026-11-17T18:57:46.6030117+09:00`）。areka と補助プロセスは残っていない（スクリーンセーバーは動いたまま）。読み込みの記録は前と同じ（まだ働かない形の記録 1 行・surface28 の `base` の 3 行）。`hit_region_client` の行は 0 件。

### 5 回目（2026-11-17・開発者が在席・入力が届いた）

- 起動の前に 127.0.0.1:9276 を握るものと `*.scr` のプロセスが無いことを確かめた。4 回目の記録は `run-R1-fourth.log` に写して残した。自動終了は既定の 300000。記録は `run-R1.log`。
- `id=OnFirstBoot` が 1 秒後に見え、その 6 秒後に pid 11752 のプロセス名が `areka` であることを確かめてから道具を動かした（19:09:05）。送る前の確かめはどれも通り、候補の窓は前と同じ 2 つ（`1548,704` と `2214,704`・666x1000・k=2）。右の窓（hwnd 4195444）を本体とした。
- 道具の出力（逐語・候補の行は略）:

```
stroke Head surface=165,70 screen=2544,844 k=2 19:09:07.665
stroke Skirt surface=160,436 screen=2534,1576 k=2 19:09:10.903
stroke Foot surface=152,483 screen=2518,1670 k=2 19:09:14.109
stroke Fan surface=262,232 screen=2738,1168 k=2 19:09:17.340
drag Bust dir=-1 surface=165,213 screen=2544,1130 k=2 19:09:20.546
drag Skirt dir=1 surface=160,436 screen=2454,1576 k=2 19:09:24.164
drag Head dir=-1 surface=165,70 screen=2544,844 k=2 19:09:27.763
drag Foot dir=1 surface=152,483 screen=2438,1670 k=2 19:09:31.357
drive done 19:09:34.659
```

- areka は 300 秒の自動終了で exit=0（`R1 pid=11752 start=2026-11-17T19:08:50.5563077+09:00 exit_ms=300000` → `R1 exit=0 end=2026-11-17T19:13:51.2943351+09:00`）。areka と補助プロセスは残っていない。
- ドラッグのたびに本体の窓は 80 物理点動き（`2214,704` → `2134,704` → `2214,704` …）、左・右の交互でもとの位置へ戻った。

## 4. 記録の抜き書き

### 5 回目の `run-R1.log`

当たり判定の解決（`hit_region_client` の `debug!`・各点の最初の 1 行・`target=TargetId(0)`・前置きは略）。`k_ratio` は 2/1 で、道具の見込みの k=2 と合う:

```
2026-11-17T10:09:07.515865Z k_ratio=ScaleRatio { num: 2, den: 1 } client_x=330 client_y=140 surface_x=165 surface_y=70 region=Some("Head")
2026-11-17T10:09:10.753590Z k_ratio=ScaleRatio { num: 2, den: 1 } client_x=320 client_y=872 surface_x=160 surface_y=436 region=None
2026-11-17T10:09:13.950980Z k_ratio=ScaleRatio { num: 2, den: 1 } client_x=304 client_y=966 surface_x=152 surface_y=483 region=None
2026-11-17T10:09:17.194815Z k_ratio=ScaleRatio { num: 2, den: 1 } client_x=524 client_y=464 surface_x=262 surface_y=232 region=None
2026-11-17T10:09:20.393567Z k_ratio=ScaleRatio { num: 2, den: 1 } client_x=330 client_y=426 surface_x=165 surface_y=213 region=Some("Bust")
```

`hit_region_client` は 3,176 行（`Some("Head")` 732・`Some("Bust")` 327・`None` 2,117）。`Some` の名前は `Head` と `Bust` だけで、楕円・多角形の名前（`Foot`・`Fan`・`Skirt`）は 1 行も無い。

SHIORI へ送ったマウスのイベント（`shiori_request` の `id` と `references`・時刻順・同じ撫での往復は 2 行ずつに略）:

```
10:09:07.536551 OnMouseMove      ["165", "70", "0", "0", "Head", "0", "mouse"]
10:09:07.674675 OnMouseMove      ["161", "70", "0", "0", "Head", "0", "mouse"]
10:09:10.758016 OnMouseMove      ["160", "436", "0", "0", "", "0", "mouse"]
10:09:10.922228 OnMouseMove      ["156", "436", "0", "0", "", "0", "mouse"]
10:09:13.956521 OnMouseMove      ["152", "483", "0", "0", "", "0", "mouse"]
10:09:14.115525 OnMouseMove      ["148", "483", "0", "0", "", "0", "mouse"]
10:09:17.206944 OnMouseMove      ["262", "232", "0", "0", "", "0", "mouse"]
10:09:17.349654 OnMouseMove      ["258", "232", "0", "0", "", "0", "mouse"]
10:09:21.068797 OnMouseDragStart ["165", "213", "0", "0", "Bust", "0", "mouse"]
10:09:21.841925 OnMouseDragEnd   ["165", "213", "0", "0", "", "0", "mouse"]
10:09:24.691699 OnMouseDragStart ["160", "436", "0", "0", "", "0", "mouse"]
10:09:25.470401 OnMouseDragEnd   ["160", "436", "0", "0", "", "0", "mouse"]
10:09:28.282741 OnMouseDragStart ["165", "70", "0", "0", "Head", "0", "mouse"]
10:09:29.058121 OnMouseDragEnd   ["165", "70", "0", "0", "", "0", "mouse"]
10:09:31.884173 OnMouseDragStart ["152", "483", "0", "0", "", "0", "mouse"]
10:09:32.659021 OnMouseDragEnd   ["152", "483", "0", "0", "", "0", "mouse"]
```

数: `OnMouseMove` は Reference4 が `Head` 14・`Bust` 2・空 43。`OnMouseDragStart` は `Bust` 1・`Head` 1・空 2（Skirt と Foot）。`OnMouseDragEnd` は 4 件とも空。

ドラッグの終わりの Reference4 が空の理由: クローディアは `OnMouseDragStart` に `\0\s[29]\e` で答える（記録の `areka::log::script: \0\s[29]\e ghost=悪役令嬢クローディア label=Ghost:OnMouseDragStart`）。`surface29` は `element0,base,surface29.png,0,0` だけで当たり判定を持たない。なので掴んでいる間とその後の解決は同じ点でも `region=None` になる（10:09:21.830355 の `surface_x=165 surface_y=213 region=None`）。続く `OnMouseLeave`（Reference4 `Bust`）と、空の `OnMouseDragEnd` はこれに合う。終わりの時に見えているサーフェスの当たり判定を引く振る舞いで、`collisionex` の読みとは関係ない。

ドラッグの閾値の記録: `[check_threshold] dx=-8 dy=0 distance_sq=64 threshold_sq=25 result=true`（最初の 8 物理点で閾値 5 を超えた）。

読み込みの記録は 1〜4 回目と同じ: まだ働かない形の記録が 1 行（下の 1 回目の抜き書きと同じ文）、surface28 の `base` の pattern の警告が 3 行。WARN は 11 行で、ほかは Shift_JIS の代替文字 5 行・補助プロセスの起動 1 行・自動終了の時の `kanade: 強制終了指示——終了系列（Forced）へ直行 event="force_quit" reason="user"`（10:13:51.085980Z）1 行。どれも当たり判定とは関係ない。ERROR は 0 行。

### 1 回目の `run-R1-first.log`

記録の警告以上は 11 行（ERROR は 0 行）。まだ働かない形の記録が 1 行（要件 8.6）:

```
2026-11-17T09:37:14.805245Z  WARN areka_emo_present::shell_target: shell: 楕円 32・円 0・多角形 28・region 0 行を型へ運んだが、まだ当たり判定として働かない（ID の描き先に塗るのは hit-id-pass）。最初の行はブレス `surface0` の `collisionex0,Fan,polygon,212,209,250,206,280,222,287,243,235,256`
```

`draw-methods-canon` が持つ surface28 の `base` の pattern の警告が 3 行（見込みどおり・この spec の外）:

```
2026-11-17T09:37:14.808930Z  WARN areka_emo_present::shell_target: shell: areka が描けない描画メソッドの pattern を描かない surface_id=28 animation_id=100 pattern=0 method="base"
2026-11-17T09:37:14.809294Z  WARN areka_emo_present::shell_target: shell: areka が描けない描画メソッドの pattern を描かない surface_id=28 animation_id=100 pattern=1 method="base"
2026-11-17T09:37:14.809636Z  WARN areka_emo_present::shell_target: shell: areka が描けない描画メソッドの pattern を描かない surface_id=28 animation_id=100 pattern=2 method="base"
```

ほかの 7 行は、Shift_JIS の読みの代替文字の警告 5 行（`areka_parsers::charset::decode`）・32bit の補助プロセスの起動の記録 1 行（`[helper] SHIORI 初期化の入口`）・自動終了の時の `kanade: 強制終了指示——終了系列（Forced）へ直行 event="force_quit" reason="user"`（09:39:14.990275Z）1 行。どれも当たり判定とは関係ない。入力を送っていないので、`hit_region_client` の行は 0 件。

## 5. 判定

- 1〜4 回目は道具と環境のせいで入力が届かず、判定に使っていない（3 章）。判定は 5 回目による。
- 矩形の名前が Reference4 に載ること（要件 6.1・8.5）: 合格。撫で（`OnMouseMove`）は Head の点の撫での 12 件がどれも `Head`。ドラッグの段では、Head の点で `\s[29]` の前に `Head` が 2 件・後に空が 1 件（10:09:28.343132Z）、Bust の点でも `\s[29]` の前に `Bust` が 2 件・後に空が 1 件（10:09:21.125235Z）。合わせて `Head` 14・`Bust` 2 で、空 43 のうちこの 2 件の空は着替えの後のもの。ドラッグの始め（`OnMouseDragStart`）は Bust の点で `Bust`・Head の点で `Head`。`collisionex` の `rect` の名前が `collision*` と同じ欄に載る。
- ドラッグの終わり（`OnMouseDragEnd`）: 4 件とも空。クローディアがドラッグの始めに当たり判定の無い `surface29` へ着替えるためで（4 章）、終わりの時の面に名前が無いので空が正しい。`collisionex` の行の名前が落ちたのではない（着替える前の解決は名前を返している）。クローディアで「終わりに名前が載る」場面は作れないので、これ以上は確かめない。
- 楕円・多角形だけの所で名前が載らないこと（要件 8.6 の前半）: 合格。Skirt（polygon）・Foot（ellipse）・Fan（polygon）の点の撫では 36 件とも空で、Skirt と Foot の点のドラッグの始めも空。解決の記録も `region=None` で、`Foot`・`Fan`・`Skirt` の名前は記録に 1 度も出ない。
- 読み込みの記録（要件 8.6 の後半）: 合格（まだ働かない形の記録 1 行・楕円 32・円 0・多角形 28・region 0）。surface28 の `base` の 3 行は `draw-methods-canon` の持ち分で見込みどおり。
- 拡大率: 記録の `k_ratio` 2/1 と、窓の大きさから求めた k=2 が一致。画面の点とサーフェスの点の対応（例 `client_x=330 client_y=140` → `165,70`）も見込みどおり。
- 排他テストの机（要件 8.7）: 各走行は進行役が机を取ってから起こし、終わると返した。
- 判定: 合格（ドラッグの終わりの空は上の理由による正しい振る舞い）。
