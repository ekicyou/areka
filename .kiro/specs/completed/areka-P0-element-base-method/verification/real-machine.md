# 実機の記録: areka-P0-element-base-method（タスク 4.2・要件 4.3・3.2）

## 1. 走行の条件

| 項目 | 値 |
|---|---|
| 版 | コミット `fbf7b556`（4.1 まで）から組んだ debug 版 `areka.exe`（x64）と `shiori-host32-helper.exe`（i686） |
| 根 | `<ワークツリー>\target\ebm\root`（`nar-sample-path claudia` の展開 `target\nar-samples\manual\claudia` をそのまま写したもの） |
| 検体 | クローディア（`vendors/sample_ghost/claudia.nar`・YAYA・32bit）。**無改変**。根の `ghost\claudia\shell\master\surfaces.txt` の SHA256 の頭 16 字 `4e45351719327bac` は `.nar` の中の `shell/master/surfaces.txt` と一致。`element0,base,` の行 5 本（surface6・26・29・11・19）はそのまま |
| 記憶 | 起動の前に根の `profile`・`tmp`・ゴーストの `profile\areka` を消した |
| `RUST_LOG` | `info,areka=debug,areka_emo_present=debug,areka::mcp=debug,areka_mcp=debug` |
| 自動終了 | `AREKA_APP_SMOKE_EXIT_MS=600000`（10 分で発火して終わった） |
| 起動 | `AREKA_*`／`WINTF_*` を全部外し、`AREKA_ROOT=<根>`・`AREKA_PROFILE_DIR=<根>\profile`・`TMP`／`TEMP=<根>\tmp`・`NO_COLOR=1` を付けて `Start-Process <根>\areka.exe "<根>\ghost\claudia" -PassThru`。記録は `target\ebm\run-R1.log` |
| 待受 | `MCP: 待受を始めた url=http://127.0.0.1:9801/api/mcp/v1`。起動の前に 9801〜9830 を待ち受けるプロセスは 0 件、SSP も動いていなかった |
| 画面 | 内蔵の画面・拡大率 200%（`k_shell=2.0`） |

組み立てと根づくり:

```
cargo build -p areka --bin areka -j 2
cargo build -p shiori-host32-helper --target i686-pc-windows-msvc -j 2
cargo run -q -p sample-ghost-kit --bin nar-sample-path -- claudia   # root=…\target\nar-samples\manual\claudia
# その中身と areka.exe・shiori-host32-helper.exe を target\ebm\root へ写す
```

走行（pid は自分で起こしたもの）:

```
R1 pid=30708 start=2026-10-05T23:19:44.4060002+09:00 exit_ms=600000
```

終わり方: `[quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=Smoke closed=4` → `shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)）` → `MCP: 待受を閉じた addr=127.0.0.1:9801`。こちらから止める操作はしていない。

SHIORI（YAYA）は 32bit の補助プロセスで読み込まれ、台詞を話して表情を変えた（下の 3 章の `\s[0]`・`\s[7]`・`\s[4]` など）。

## 2. 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | `\s[6]`・`\s[11]`・`\s[26]` のキャラクターが 333×500 | **合**（`dump_surface` の 3 面とも 333×500。`\s[26]` は起動の台詞で画面にも 333×500 で出た） |
| ⑵ | 土台の絵と顔の部品が重なっている | **合**（土台だけの面との差が部品の置き場所の中だけ。絵を添付） |
| ⑶ | 本 spec の警告が 0 行 | **合**（「shell: areka が描けない描画メソッドの element定義を描かない」0 行） |
| ⑷ | `shadowed=` の行が 2 回出て、どちらも前より 2 多い（19・29） | **合**（前 `used=12 shadowed=0` → 今 `used=10 shadowed=2`。2 回とも。載ったのは 19 と 29） |
| ⑸ | surface19・29 が前と同じに見える | **合**（絵の比較で判定。画面での見比べはしていない。下の 3 章 ⑸） |

当たり判定は見ていない（タスクの決まりどおり）。

## 3. 詳細

### ⑴ 3 面の大きさ

MCP の `dump_surface` を curl で呼んだ（`initialize` の後に `tools/call`。`target\ebm\mcp.sh`）。`surface` を指定した呼び出しは「その面だけを初期の状態で描いた絵」を返す（画面は変わらない）。

| 呼び出し | 本文 | PNG |
|---|---|---|
| `{"scope":0,"surface":6}` | `OK:scope 0, surface 6 rendered alone in its initial state (not what is on the screen now)` | 333×500 |
| `{"scope":0,"surface":26}` | `OK:scope 0, surface 26 rendered alone in its initial state (not what is on the screen now)` | 333×500 |
| `{"scope":1,"surface":11}` | `OK:scope 1, surface 11 rendered alone in its initial state (not what is on the screen now)` | 333×500 |

成功の記録（逐語）:

```
DEBUG areka::mcp::dump_surface: [mcp] 絵を返す tool="dump_surface" scope=0 surface_id=6 width=333 height=500 base64_len=271792 ui_us=19048 encode_us=514313
DEBUG areka::mcp::dump_surface: [mcp] 絵を返す tool="dump_surface" scope=0 surface_id=26 width=333 height=500 base64_len=272932 ui_us=27131 encode_us=354488
DEBUG areka::mcp::dump_surface: [mcp] 絵を返す tool="dump_surface" scope=1 surface_id=11 width=333 height=500 base64_len=74104 ui_us=11722 encode_us=146694
```

画面の表示の記録でも、`\s[26]` は起動の台詞で本体の窓（`TargetId(0)`）に 333×500 で出た（この走行で 2 回）:

```
INFO actor{actor=emo-text}: areka_emo_present::presenter::show: apply(ShowSurface): 表示・マスクを更新 target_id=TargetId(0) surface_id=26 cache_hit=false … native_w=333 native_h=500 …
```

前の版との違い: `areka-P0-mouse-drag-events` の実機確認の R1（同じ無改変のクローディア・本 spec の前の版 `9602f640`）の記録 `mouse-drag-events-456cbe\target\mde-signoff\run-R1.log` では、同じ行が `surface_id=26 … native_w=100 native_h=56`、相方の `surface_id=11 … native_w=71 native_h=42` だった（上乗せの部品の大きさだけで描かれ、キャラクターが消えていた）。今回は `\s[6]`・`\s[11]` を台詞が画面に出す場面は無かったので、この 2 面は `dump_surface` の数字で判定した。

### ⑵ 土台と顔の部品の重なり

`surfaces.txt`（無改変）の書き方:

```
surface6  { element0,base,surface0.png,0,0   / element1,overlay,surface1000.png,112,100 }
surface26 { element0,base,surface0.png,0,0   / element1,overlay,surface1001.png,114,100 }
surface11 { element0,base,surface10.png,0,0  / element1,overlay,anthony_eyes11.png,130,270 }
```

部品の実寸は `surface1000.png`・`surface1001.png` が 100×56、`anthony_eyes11.png` が 71×42。土台の `surface0.png`・`surface10.png` は 333×500。

返った PNG を、土台だけの面（`\s[0]`・`\s[10]` を同じく `dump_surface` で取ったもの）と画素で比べた（色の差 2 を超える画素の数と、その外接の矩形）:

| 比べたもの | 違う画素 | 外接の矩形 | 部品の置き場所 |
|---|---|---|---|
| `\s[6]` と `\s[0]` | 2,128 | x=115..204, y=102..151 | x=112..211, y=100..155 |
| `\s[26]` と `\s[0]` | 2,330 | x=116..209, y=102..151 | x=114..213, y=100..155 |
| `\s[11]` と `\s[10]` | 1,283 | x=134..195, y=272..306 | x=130..200, y=270..311 |

違いは 3 面とも部品の置き場所の中だけで、その外は土台の絵と同じ。土台の絵の上に顔の部品が重なって描かれている。`\s[0]` と `surface0.png`、`\s[10]` と `surface10.png` は、α と乗算済みの色がすべての画素で一致した（較正）。

絵（`dump_surface` が返した PNG そのもの）:

- `\s[6]`（目を閉じた顔）: [dump-surface6.png](dump-surface6.png)
- `\s[26]`（目を開いた顔）: [dump-surface26.png](dump-surface26.png)
- `\s[11]`（相方・目の部品）: [dump-surface11.png](dump-surface11.png)
- 土台だけ: [dump-surface0.png](dump-surface0.png)・[dump-surface10.png](dump-surface10.png)
- 起動して台詞が進んだ後の画面（23:23:27・本体は `\s[7]`・相方は `\s[10]`。画面の写しから 2 体のあたりを切り出したもの）: [screen-boot.png](screen-boot.png)

### ⑶ 警告

| 記録の段 | 件数 |
|---|---|
| 本 spec の警告「shell: areka が描けない描画メソッドの element定義を描かない」 | 0 |
| WARN 全体 | 1 |
| ERROR | 0 |
| 標準エラー出力 | 1 行（`[helper] SHIORI 初期化の入口: loadu`。補助プロセスの案内） |

WARN の 1 行は自動終了の時のもの（逐語）:

```
WARN actor{actor=kanade}: kanade: 強制終了指示——終了系列（Forced）へ直行 event="force_quit" reason="user"
```

自動終了（`origin=Smoke`）で全窓を閉じたときに出る行で、シェルの読み込みとは関係ない。前の R1 は開発者が閉じる前にこちらで止めた走行で、この行は出ていない。シェルを読む段の WARN（焼く段の脱落・コマの相手・入れ子・箱）は 0 行。

クローディアの `surfaces.txt` の `element` の行の第 2 欄は `base` と `overlay` だけなので、警告 0 行は 2.3 の檻（`claudia` を読ませて本 spec の警告 0 件）と同じ結果。

### ⑷ `shadowed=` の数

起動 1 回で 2 回出た。記録に呼び出し元は出ないが、1 回目は「placement: k₀ 倍後の物理窓寸で窓を生成する（起動採寸・要件 3.3）」の行より前（窓の大きさを測る `placement/measure.rs`）、2 回目はその後（起動の資産を組む `emo2_boot/assets.rs`）に出た:

```
2026-10-05T14:19:44.728675Z DEBUG areka_emo_present::shell_target: shell: element0 が在るため面の画像を土台に使わなかった（R6.2） surface_id=19 file="surface19.png"
2026-10-05T14:19:44.729177Z DEBUG areka_emo_present::shell_target: shell: element0 が在るため面の画像を土台に使わなかった（R6.2） surface_id=29 file="surface29.png"
2026-10-05T14:19:44.729579Z  INFO areka_emo_present::shell_target: shell: シェルの面の画像の一覧が終わった（R6.1） shell_dir=…\target\ebm\root\ghost\claudia\shell\master recognized=12 used=10 shadowed=2
2026-10-05T14:19:44.909196Z DEBUG areka_emo_present::shell_target: shell: element0 が在るため面の画像を土台に使わなかった（R6.2） surface_id=19 file="surface19.png"
2026-10-05T14:19:44.920176Z DEBUG areka_emo_present::shell_target: shell: element0 が在るため面の画像を土台に使わなかった（R6.2） surface_id=29 file="surface29.png"
2026-10-05T14:19:44.920399Z  INFO areka_emo_present::shell_target: shell: シェルの面の画像の一覧が終わった（R6.1） shell_dir=…\target\ebm\root\ghost\claudia\shell\master recognized=12 used=10 shadowed=2
```

前の版（`mouse-drag-events-456cbe\target\mde-signoff\run-R1.log`・無改変のクローディア）は 2 回とも `recognized=12 used=12 shadowed=0`。今回は 2 回とも `used=10 shadowed=2` で、どちらも 2 多い。

数の理由: surface19・29 は `element0,base,surface19.png`・`element0,base,surface29.png` で、自分の番号の画像を `base` で置いている。前は `base` の行が読み捨てられて element0 が無かったので、`surface19.png`・`surface29.png` が面の画像として土台に使われていた（`used` に数えた）。今は element0 が在るので面の画像としては使わず（`shadowed` に数える）、同じ画像を element0 として描く。surface6・26・11 の element0 は `surface0.png`・`surface10.png` を指しており、面 0・10 は element0 を持たないので、この 2 枚は前も今も `used` のまま。

参考: 前の実機確認の R2 以降（`base` を `overlay` に書き替えた写し）も `used=10 shadowed=2` で、今回の無改変の検体と同じ数になった。

### ⑸ surface19・29 の見え方

前の版では、この 2 面は「element0 が無いので `surface19.png`・`surface29.png` をそのまま土台に敷く」経路で描かれていた（前の R1 の表示の記録でも 333×500）。今回は同じ画像を element0（`base`・0,0）として描く経路になった。

`dump_surface` で取った今回の `\s[29]`（scope 0）と `\s[19]`（scope 1）を元の画像ファイルと画素で比べると、α と乗算済みの色がすべての画素で一致した（不透明な画素の色の差も 0）。返った PNG は乗算を戻して書くので、半透明の縁の画素では乗算を戻す前の色の丸めの分だけ値が動くが、見た目に効く乗算済みの値は同じ。前の経路は画像をそのまま敷くので、前の見え方とも同じになる。

- `\s[29]`: [dump-surface29.png](dump-surface29.png)（333×500）
- `\s[19]`: [dump-surface19.png](dump-surface19.png)（333×500）

画面にこの 2 面を出すにはキャラクターのドラッグが要る（ドラッグ中の絵が `\s[29]`・`\s[19]`）。computer-use でドラッグを始めようとしたところ、操作が中断された（`Batch aborted after 0 of 10 actions (user interrupt)`）。それ以上の画面の操作はしていないので、画面での見比べは無い。2 面の絵は上の画素の比較で判定した。
