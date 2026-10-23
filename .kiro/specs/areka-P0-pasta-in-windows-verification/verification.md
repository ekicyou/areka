# ぱすたの実機の検証の記録

spec `areka-P0-pasta-in-windows-verification` の最初の走行の記録。手順は `doc/pasta-real-run.md`。時刻はすべて日本時間。

## 1. 同定

| 項目 | 値 |
|---|---|
| コミット | `c3964dba`（`git -C <w> rev-parse --short HEAD`。手順の 3 節の画面の読み方の直しは作業中の差分で、`areka.exe` には関わらない） |
| debug 版 | `<w>\target\debug\areka.exe`（33,629,696 バイト・2026-10-23 00:03:48 に `cargo build -p areka` で作った物） |
| i686 の補助 | `<w>\target\debug\shiori-host32-helper.exe`（263,168 バイト・2026-10-23 00:04:58）。`sync-i686-helper.ps1` が 1 回目に `placed i686 helper: ...`、2 回目に `helper is i686: target\debug\shiori-host32-helper.exe` を出し、どちらも `sync exit=0` |
| ぱすたの書庫 | `pasta-in-windows.nar` の `.sha256` の値 `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（書庫の中の `ghost/master/pasta.toml` は `version = "1.0.0"`。descript に版の行は無い） |
| 道具 | rustc 1.99.1・cargo 1.99.1 |

`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`。

### 準備の結果

手順の 3 節の準備の塊を、ワークツリーの外のフォルダ（このセッションの一時の作業フォルダ）を作業ディレクトリにした `pwsh` で、先頭の `$w` の行だけ替えてそのまま 2 回回した（2026-10-23 00:01〜00:04 と 00:05）。2 回目の出力:

```text
sample-fetch: pasta-in-windows: present
sample-fetch: fetched 0, present 1, failed 0
fetch exit=0
pasta sha256: ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6
...\target\pasta-verify\root\ghost\emo2
...\target\pasta-verify\root\ghost\pasta-in-windows
...\target\pasta-verify\root\balloon\emo2-kakukaku
...\target\pasta-verify\root\balloon\StayseeBalloon
build exit=0
build i686 exit=0
helper is i686: target\debug\shiori-host32-helper.exe
sync exit=0
```

- 根 `<w>\target\pasta-verify\root` に `ghost\emo2`・`ghost\pasta-in-windows`・`balloon\emo2-kakukaku`・`balloon\StayseeBalloon` の 4 つが揃った。縦書きのバルーンはゴーストのフォルダの中（`ghost\pasta-in-windows\pasta_tategaki\descript.txt`）に在る。本走行では根の `balloon\` に足していない。
- 記憶の置き場 `<w>\target\pasta-verify\profile` は空（0 件）。
- ぱすたの書庫は取り寄せの段の置き場（`<w>\target\nar-samples\fetched\`）に在り、最新の `.sha256` と合ったので取り直していない（`present 1`）。

### 書庫の中身の自動テストの較正（タスク 1.2）

`crates/sample-ghost-kit/tests/pasta_inner_balloon_test.rs` の較正のテスト `a_vertical_balloon_outside_the_ghost_folder_is_rejected`（縦書きのバルーンをゴーストのフォルダの外に置いた固定入力）が、置き場の誤りを捉えることを確かめた。

- 判定を「根から再帰して最初に見つかった `pasta_tategaki/descript.txt` でよい」形に緩めると、較正のテストが `a_vertical_balloon_outside_the_ghost_folder_is_rejected ... FAILED`（`1 passed; 1 failed`）になった。
- 判定を元に戻すと緑に戻った。
- 固定入力の `install.txt` には、設計の 2 行（`type,ghost`・`directory,pasta-in-windows`）に `name,` の行を足している。足さないと `NarArchive::open` が `MissingRequiredKey { key: "name" }` で断る。

## 2. 機械の構成

| 項目 | 値 |
|---|---|
| OS | Windows 11 Pro 26H2（10.0.26300・UBR 9711） |
| 画面 1 | `\\.\DISPLAY1`・主画面・位置 (0, 0)・2880×1800（物理画素）・DPI 192＝拡大率 200% |
| 画面 2 | `\\.\DISPLAY2`・位置 (-2560, 195)・2560×1600（物理画素）・DPI 144＝拡大率 150% |

- 位置と拡大率は、手順の 3 節の命令（プロセスを画面ごとの DPI に合わせてから `Screen.AllScreens` と `GetDpiForMonitor` を読む）で 2026-10-23 00:06 に読んだ。
- 拡大率の違う 2 つの画面（200% と 150%）が在るので、段 5 は回す。

## 3. 環境変数と記録の水準

手順の 4 節の起動の命令で設定する値。走行の前に替えたものは無い。段の判定に別の分岐が要って対象を足したときは、4 節（走行）にその値を書く。

| 名前 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT` | `<w>\target\pasta-verify\root` | 根 |
| `AREKA_PROFILE_DIR` | `<w>\target\pasta-verify\profile`（準備で毎回空にする） | 記憶の置き場。空なので、起動のイベントは `OnFirstBoot` になる |
| `RUST_LOG` | `info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug` | バルーンの決め方の分岐が記録に出る水準 |
| `AREKA_APP_SMOKE_EXIT_MS` | `3600000`（60 分） | 安全弁。7 段の所要（30〜45 分の見込み）より長く |
| `NO_COLOR` | `1` | 記録を色の制御文字の無い平文にする |
| `AREKA_NO_ALERT`・`AREKA_USER_ROOT` | 消す | 本物の利用者の置き場（`%LOCALAPPDATA%\areka`）に触らない |

起こし方は `<w>\target\debug\areka.exe --ghost "<w>\target\pasta-verify\root\ghost\pasta-in-windows"`（作業ディレクトリは `<w>\target\pasta-verify`）。

### この回で手順へ戻した直し

- 3 節: 拡大率を「Windows の設定の『ディスプレイ』で読む」から、命令で読む形（`GetDpiForMonitor` の DPI を 96 で割る）に替えた。設定の画面を開かずに、回す者が命令だけで記録に書けるようにするため。拡大率の違う画面が無いときは走らせる前に「段 5 は回していない」と書く旨も足した。

## 4. 走行

### 参考（代わりの置き方）

本走行の後に回した「補いの走り」の結果（タスク 4.2・要件 4.2）。根の写しの `balloon\` に、ゴーストのフォルダの中の縦書きのバルーン `ghost\pasta-in-windows\pasta_tategaki` を写した別の根（`<w>\target\pasta-verify-alt\root`）で、段 1〜2 だけを回した。ぱすたの本来の置き方（ゴーストのフォルダの中のバルーン）ではないので、ここの結果は段の 3 値（同じ／違う／回せなかった）の数に入れない。目視で決める項目（列と枠の並び・重なりの順）は、開発者の判定は 4.3 で書く。ここは事実だけ。

回したのは 2 回（alt1 と alt2）。alt1 は縦書きのバルーンで起動しなかったので、準備を 1 つ替えて alt2 で回し直した。設計の「1 回だけ」に当たる補いの走りは alt2 である。alt1 は、写したゴーストの記憶のために狙った起動（縦書きのバルーンでの `OnFirstBoot`）にならなかったので、補いの走りとして数えない。どちらも `Start-Process` に `-WindowStyle Hidden` を足した（空のコンソールの窓を出さないため。ぱすたの窓とバルーンは普通に出た）。`RUST_LOG` は 3 節の値のまま。

#### alt1（2026-10-23 08:29:26〜08:39:16）

- pid 32956・終了コード 0（`logs\alt1.pid.txt`・`logs\alt1.exitcode.txt`）。起こし直しなし。
- 縦書きのバルーンで起動しなかった訳: 準備で記憶の置き場 `$v\profile` は空にしたが、ゴーストの記憶は根の中の `root\ghost\pasta-in-windows\ghost\master\profile\areka\sylphya.toml` に在り、本走行（run1）の終わりの値（`[last] balloon = "StayseeBalloon"`・`[window.0] x=-1725 y=194`・`[boot] count=1`）ごと写されていた。取り寄せた書庫には `profile` のフォルダは無い（run1 が作った物）。そのため起動は `OnBoot`（`OnFirstBoot` でない）、バルーンは `balloon_resolved route=Memory` で `StayseeBalloon` に決まり、根の `balloon\pasta_tategaki` は選ばれなかった。ぱすたの記録 `pasta.log` も run1 の分ごと写っていた（`logs\alt1.pasta.log` の alt1 の分は 08:29 以降）。
- 段 2 の途中で、右クリックメニューの「バルーン → 窓際のぱすたさん縦書き」で替え（`skin_switch_requested kind=Balloon to=pasta_tategaki origin=Menu` → `skin_switch_done`）、段 2 を縦書きのバルーンで回し直した（撮った物 `shots\alt1\2b-*`・`2d-*`）。本文と選択肢・2 ページの縦書き試作の列は重ならずに並んだ。中身は alt2 と同じ形なので、下の alt2 にまとめる。
- 数（`logs\alt1.stdout.log` 8835 行）: panic 0・致命 0・ERROR 0・WARN 137（seriko 128・`areka_emo_present::balloon` の縮退 6・`areka_emo_atlas` 3）。

#### alt2（2026-10-23 08:42:31〜08:46:48）

- 準備で 1 つ替えた: 写した根のゴーストの記憶 `$v\root\ghost\pasta-in-windows\ghost\master\profile` を、起動の前に消した。`-WindowStyle Hidden` は alt1 と同じ。
- pid 15924・終了コード 0（`logs\alt2.pid.txt` `areka pid=15924 start=2026-10-23T08:42:31`・`logs\alt2.exitcode.txt` `areka exit=0 end=2026-10-23T08:46:48`）。起こし直しなし。終わりは × が効かず、右クリックメニューの「終了」で終えた（`OnClose` → `unload_clean` → `app_exit closed=4` → `session_mark_cleared`）。
- 起動の決め方: `ghost_resolved route=Pointed`・`balloon_resolved route=Descript dir=...\root\balloon\pasta_tategaki`。起動のイベントは `OnFirstBoot`。`descript_balloon_not_found` の行は 0。ぱすたの窓は既定の位置 (0,864)。
- 数（`logs\alt2.stdout.log` 3989 行・`alt2.stderr.log` 1 行）: panic 0・致命 0・ERROR 0・WARN 100（seriko の `mustselect カテゴリの脱衣指示を無視` 96・`areka_emo_atlas` 4〔全透明の `pasta/null.png` 2・縦書きのバルーンの全透明の `balloonk0.png`（set=0・436x450）2〕）。`areka_emo_present::balloon` の縮退（`面がデフォルト定義側…`）の WARN と `boot_resolve` の WARN は 0。標準エラーは `[helper] SHIORI 初期化の入口: loadu` の 1 行。

段 1（起動）:

- 台詞が縦書きのバルーンの縦の列に出た。右の枠に「‥‥、も‥‥しもーし？」「あれ、映ってない？」「ね、そこのボール叩いてくださいな。」、`\_l[-283,0]` の後の左の枠に「えっと、そこで」「フワフワしてるボール。」。列どうし・2 つの枠どうしは重ならない（`1-boot-primary-full.png`・`1-boot-balloon0.png`）。
- ぱすた（`\0`）の × の印の板が、右の枠の 1 列目の上端に重なり、字の一部が隠れた。見た目の前後は、ぱすた（× の板）が手前・バルーンが奥（`1-boot-primary-full.png`・`2-choice-zorder.png`・`2b-after-choice.png`）。
- 窓の枠（`\1`・面 10）は、ぱすたの居る主画面でなく副画面の右寄りに別に出た（`1-boot-surface1.png`・`1-boot-second-full.png`。run1 と同じ）。そのため、窓（奥）→ バルーン → ぱすた（手前）の 3 つの並び全体は見えなかった。

段 2（縦書きの会話）:

- `選択肢テスト`: 本文が右の枠に縦の 2 列、`\_l[-253,0]` の後の選択肢 4 つが左の枠に縦の列で右から 晴れてる・雨だよ・曇ってる・ないしょ と並び、重ならない（`2-choice-zorder.png`）。`\![*]` の印は見えない。
- 選択の待ちは、表示の終わりから 30 秒で `OnChoiceTimeout` になる。1 回目はクリックが 4 秒遅れて時間切れになった。2 回目は「雨だよ」の列が反転し（`2b-choice-hover.png`）、クリックで `choice_selected id=雨だよ` → `OnChoiceSelectEx` →「そういえば薄暗いような？」が右の枠に縦の 1 列で出た（`2b-after-choice.png`）。
- `縦書き試作`: 1 ページ目・`\c` の後の 2 ページ目とも、右の枠と左の枠（`\_l[-283,0]` の後）の列が重ならずに並んだ（`2-tategaki-burst-01..10.png`・`2-tategaki-balloon0-end.png`）。どのページでも、右の枠の 1 列目の上端に × の板が重なった。

置き場: 記録は `<w>\target\pasta-verify-alt\logs\alt1.*`・`alt2.*`、撮った物と控えは `<w>\target\pasta-verify-alt\shots\alt1\`・`shots\alt2\`（それぞれ `notes.md`）。

手順の穴: 手順書（`doc/pasta-real-run.md`）4 節の補いの走りの準備は、本走行の根を写すときにゴーストの記憶（`ghost\master\profile\`）も一緒に写し、消していない。このため alt1 は前の走行の記憶で起動した。手順書の直しはタスク 8.1 で行う。

## 5. 崩れた所の表

本走行 run1（`<w>\target\pasta-verify\shots\run1\notes.md`・`logs\run1.*`）と、参考の補いの走り alt2（`<w>\target\pasta-verify-alt\shots\alt2\notes.md`・`logs\alt2.*`）で見えた崩れを、SSP の基準（`ssp-measurement.md`）と比べて 1 件ずつ仕分けた（2026-10-23・タスク 5.1・要件 5.2・5.3・5.6・7.3）。引き取る spec は、設計の Out of Boundary の一覧と要件の Adjacent expectations で決め、一覧に無い件は兄弟の spec の `brief.md` の持ち分と照らした。どれにも当たらない件は、設計の「仕分けと申し送り」の線のとおり本 spec が受け持ち、直すか起票するかをタスク 5.3 で決める。

- 記録の行の時刻は世界標準時（`Z`）のまま引いた。日本時間は 9 時間足す。
- 目視で「同じ／違う」を決める項目（重なりの順・メニューの見た目・縦書きの並び）は、開発者の判定（タスク 4.3）がまだ無い。下の表は、記録と撮った物から言える事実だけで仕分けた。判定待ちの所は表の中に「開発者の判定待ち」と書いた。
- 「参考」と書いた件は補いの走り（alt2）だけで見えた件で、段の 3 値の数には入れない。

### 崩れた所

| 番号 | 症状 | 根拠 | 原因 | 扱い | 引き取る spec／裁定 |
|---|---|---|---|---|---|
| 1 | 縦書きのバルーンが見つからず、既定の横書きのバルーン（`StayseeBalloon`）で出た。SSP はゴーストのフォルダの中の `pasta_tategaki` を使った | run1 の記録 `2026-10-22T23:17:53.448361Z` の `boot_resolve` の WARN `descript_balloon_not_found value="窓際のぱすたさん縦書き"`・get_log の `#3 … balloon_resolved route=Default dir=...\root\balloon\StayseeBalloon`（`shots\run1\1-boot-getlog.txt`）。SSP は `ssp-measurement.md` 2 節の段 1 | `crates/areka/src/boot_resolve.rs:294`〜`297`。descript の `balloon` の名前を、根の `balloon\` の一覧（`listed`）の中だけで探し、ゴーストのフォルダの中を探さない | 引き取る spec | `areka-P0-ghost-inner-balloon` |
| 2 | 横書きのバルーンで、`\_l[-283,0]` の後の段が前の行に重なって描かれた（起動の段・`縦書き試作` の 2 ページとも）。`\_l[-253,0]` の後の選択肢 4 つも本文の 2 行目に重なった。縦書きのバルーン（alt2）では重ならない | `shots\run1\1-boot-balloon0.png`（「あれフ映ってるボール」）・`2-choice-zorder.png`・`2-choice-balloon0.png`・`2-tategaki-frames-page1.png`・`2-tategaki-balloon0-end.png`。参考: `shots\alt2\1-boot-balloon0.png`・`2-choice-zorder.png`・`2-tategaki-burst-03.png` | 1 の結果。台本の `\_l[x,y]` は「現スコープ側のカーソル位置を XY の座標に移動する」（ukadoc）で、ぱすたは縦書きの左の枠へ移るために y を 0 にしている。横書きのバルーンでは y=0 が 1 行目の高さになるので重なる。原因の場所は 1 と同じ（`boot_resolve.rs:294`〜`297`） | 引き取る spec | `areka-P0-ghost-inner-balloon` |
| 3 | 選択肢の前の `\![*]` の印が出ない（run1・alt2 とも）。SSP は 4 つの選択肢それぞれの前に印が出た | `shots\run1\2-choice-zorder.png`・参考 `shots\alt2\2-choice-zorder.png`。SSP は `ssp-measurement.md` 2 節の段 2（`stage2-choice-shot.png`） | `crates/areka-parsers/src/sakura/decode.rs:127`。`fold_choice_marker` が印を「無視した」として直後の `\q` に畳み、印は何も描かない | 引き取る spec | `areka-P0-script-small-tags` |
| 4 | ぱすたの窓が画面の下端を越えた。窓は (0,864)-(1108,2232) で、主画面の高さ 1800 を越え、ボールの下半分がタスクバーに隠れた | run1 の記録 `2026-10-22T23:17:55.378678Z` `merge_scope restore scope=0 … default_char_y=864 … char_w=640 char_h=840`・hit_test の `bounds=(0,864)-(1108,2232)`（`logs\run1.stage3-excerpt.log`）。作業領域の下端は 1704（同じ記録の `work_area=RectPx { … bottom: 1704 }`） | `crates/areka/src/placement/measure.rs:173`。起動の位置を決めるときの窓の大きさを、`\0` は面 0 で測る（決め打ち）。ぱすたの面 0 は `pasta/xyz.png`（320×420）なので 200% で 640×840 になり、下端に揃えた y が 1704−840＝864 になった。実際に出る面 1000・1002 は 554×684（200% で 1108×1368）。ぱすたは descript に `sakura.seriko.defaultsurface,1000` を書いているが、areka は読まない（`\1` は面 10 で測るので 1108×1368・y＝336 で合っている） | 引き取る spec | `areka-P0-descript-window-keys`（`sakura.seriko.defaultsurface` と大きさの測りの決め打ちの置き換えは、同 brief の範囲に在る） |
| 5 | （参考）縦書きのバルーンの右の枠の 1 列目の上端に、ぱすたの × の板が重なり、字の一部が隠れた | alt2 の記録 `2026-10-22T23:42:32.443025Z` `Clamp 起動時関門 … scope=0 … from=PointPx { x: 80, y: 980 } to=PointPx { x: 80, y: 804 } balloon_size=SizePx { w: 872, h: 900 }`・`shots\alt2\1-boot-primary-full.png`・`2-choice-zorder.png`・`2b-after-choice.png` | 4 の結果。バルーンのぱすたからのずれ（80,116）は SSP（150% で 60,87＝等倍で 40,58）と同じだが、ぱすたの窓が 4 のとおり低すぎたため、バルーンの下端（980＋900＝1880）が作業領域を越え、起動の関門が 176 上へ寄せて × の板にかかった。原因の場所は 4 と同じ（`measure.rs:173`）。前後（ぱすたが手前・バルーンが奥）は SSP の `\![set,zorder,s0,b0,s1]` と同じ向き | 引き取る spec | `areka-P0-descript-window-keys` |
| 6 | 窓の枠（`\1`・面 10）が `\0` に重ならず、副画面に出た（run1・alt2 とも）。SSP は `scope(0)` と `scope(1)` の rect が同じ（`-782,201,49,1227`） | run1 の記録 `2026-10-22T23:17:58.829725Z` `move_applied scope=1 … to_x=0 to_y=864` の直後 `23:17:58.830231Z` `chain_finalize: 実表示寸で連鎖を再解決（初期配置の確定・scg 7.1） scope=1 from_x=0 to_x=-1108`。alt2 も `23:42:34.243709Z`・`23:42:34.244103Z` で同じ。`shots\run1\1-boot-full-a.png`・`1-boot-surface1.png` | `crates/areka/src/placement/chain_finalize.rs:121`。起動の最後の並べ直しは、既定の位置のままかを x だけで比べる（`default_x != Some(current_x)`）。ぱすたの台本の `\![move,--X=0,--Y=0,--base=0,…]` は `\1` を `\0` の左上（0,864）へ動かしたが、x は既定の 0 のままで y だけが変わったので「動かされていない」と読まれ、`\0` の左隣（x＝-1108・副画面）へ並べ直された | 直した | 本 spec（タスク 5.4・コミット `97186487`）。2026-10-23 開発者の承認（すべてのゴーストの起動の位置に効くので、直す前に仰いだ・要件 6.4）。もとは持ち主なし（`areka-P0-window-link-directives` の brief は、組の窓を拡大率の後の詰め直し〔`placement/chain_realign.rs`〕から外す件だけを名指しし、この起動の並べ直し〔`chain_finalize.rs`〕と台本の移動の食い違いは持ち分に無かった）。直した結果は 6 節 |
| 7 | 同じ画面の中のドラッグで、ぱすた（`\0`）だけが動き、窓の枠（`\1`）が付いてこない。拡大率の違う画面へのドラッグでも同じ。SSP は 2 つが揃って動いた | run1 の記録 `2026-10-22T23:23:03.468835Z` `char DragEnd 保存 scope=0 char_x=800 char_y=464`（`\1` の保存の行は無い）・`shots\run1\4-before-drag-full.png`・`4-after-drag-full.png`・`5-after-move-full.png`。SSP は `ssp-measurement.md` 2 節の段 4 | `crates/areka/src/placement/config.rs:107`・`135`。シェルの `seriko.sticky-window,0,1` を生のまま写すだけで、ドラッグに組の相手を連れて動かす所が無い | 引き取る spec | `areka-P0-window-link-directives` |
| 8 | 拡大率 150% の画面へ移ると、ぱすたの窓が 1108 から 831 の幅へ縮んだ。SSP は大きさを変えない（831×1026 のまま・`scaling` 150,150） | run1 の記録 `2026-10-22T23:23:21.153099Z` `char DragEnd 保存 scope=0 char_x=-1725 char_y=194 … char_w=Some(831)`。SSP は `ssp-measurement.md` 2 節の段 5（`stage5-rects.txt`） | 方針の裁定。areka は画面の拡大率に合わせて絵を拡げ縮めする（DPI 追従・倍率＝画面の DPI ÷ 作者の基準の DPI）のが基本の設計で、SSP の「画面が替わっても大きさを変えない」には合わせない | 方針で受けない | 2026-07-18 開発者（DPI 追従が基本の設計・SSP とは別の考え方）。出どころ: `.kiro/steering/roadmap-history.md` の M-dpi の行と追記㉜、完了 `areka-P0-emo-dpi-scaling` |
| 9 | 拡大率の違う画面へドラッグして落とすと、ぱすたが落とした点より上（左上）に置かれた。落とした点 (-1500,1000) に対し、掴んだボールの中心はおよそ (-1554,802) | run1 の記録 `2026-10-22T23:23:19.129935Z` の掴んだ点 `client_x=225 client_y=806 … bounds=(800,464)-(1908,1832)`・`23:23:21.152609Z` `DragEndEvent … x=-1500 y=1000`・`23:23:21.153099Z` `char_x=-1725 char_y=194`（`logs\run1.stage4-5-excerpt.log`）。`shots\run1\5-after-move-full.png` | `crates/wintf/src/ecs/window_proc/mouse_move.rs:302`〜`303`（`new_x = initial_window_pos.x + (current_pos.x - start_pos.x)`・y も同じ形）。ドラッグの窓の位置を「始めの窓の左上＋カーソルの移動量」で決め（始めの窓の左上は `crates/wintf/src/ecs/drag/dispatch.rs:149`〜`159` で控える）、拡大率が変わって窓が 0.75 倍に縮んでも、掴んだ点の窓の中の位置（225,806）を縮めない（-1500−225＝-1725・1000−806＝194 と合う） | 引き取る spec | `areka-P0-dpi-realign-remembered-chain`（設計の Out of Boundary「拡大率の違う画面をまたぐドラッグ」） |
| 10 | × に乗っても × が赤くならない（run1 の段 3・段 7）。SSP は `OnMouseEnter`（`Reference4: 閉じる`）が届き、応答の `\![bind,閉じる,赤,1]` で赤くなった | `shots\run1\3-x-hover.png`・`3-x-nohover.png`・`7-x-hover.png`。ぱすたの記録 `logs\run1.pasta.log` に `OnMouseEnter` の行が 0。SSP は `ssp-measurement.md` 2 節の段 3・段 7 | `crates/areka-kanade/src/schedule/events.rs:114`。送ってよいイベントの表 `ALLOWED_EVENT_IDS` に `OnMouseEnter`・`OnMouseLeave` が無く、areka は送らない | 引き取る spec | `areka-P0-mouse-click-wheel-events` |
| 11 | ボールと × を押しても何も起きない（run1 の段 3・段 7、alt2 の終わり）。窓には当たっている。SSP は `OnMouseClick` が届き、ボールのトークと終わりの台詞が出た | run1 の記録 `2026-10-22T23:22:07.960045Z` `hit_test result … screen_x=225 screen_y=1630 bounds=(0,864)-(1108,2232) button=Left is_down=true`（`logs\run1.stage3-excerpt.log`）。get_log の台本に新しい行なし・`logs\run1.pasta.log` に `OnMouseClick` が 0。`shots\run1\3-ball-1..5.png`・`7-after-x-1..4.png`・参考 `shots\alt2\9-after-x.png` | `crates/areka/src/input_events/mod.rs:483`・`526`〜`531`。キャラの窓の押下の受け口 `on_char_pointer_pressed` は、単発のクリックを送らない（完了 `input-events` の要件 7.3 の線） | 引き取る spec | `areka-P0-mouse-click-wheel-events` |
| 12 | ぱすたの身体が最後まで映らなかった（面 1002＝ボールと × だけのまま）。そのため顔を撫でた入力は窓を素通りしてデスクトップに届き、`OnMouseMove` は出ず、自然なトーク（ランダムトーク）も 1 つも出なかった | `shots\run1\1-boot-surface0.png`・`3-after-stroke-1.png`（ごみ箱が選ばれた色）・`6-after-menu-surface0.png`（面 1002 のまま）。get_log の台本は `#18` の次が `#83`（`raise_event`）。面 1002 の `window/canvas.png` は × の範囲の外が α 0（中を数えた: α 0 の画素 377,464・α 241〜255 の画素 1,472） | 11 の結果。ぱすたの初回の起動の台本（`dic/boot.pasta` の `＊OnFirstBoot`）は、最後に `\0\s[1002]` を出して「ボールのクリックで映る」のを待ち、待つ間は印 `初回待ち` で毎秒のイベント（`OnSecondChange`）の応答を止める。ボールのクリック（`OnMouseClick`）が届かないので、待ちが終わらない。原因の場所は 11 と同じ（`input_events/mod.rs:483`） | 引き取る spec | `areka-P0-mouse-click-wheel-events` |
| 13 | × で終われず、メニューの「終了」で終えたとき、終わりの台詞が出なかった。SSP は × の `OnMouseClick` で「…ここからは、画面ごしのお別れだね。…」が出た | run1 の記録 `2026-10-22T23:25:02.132856Z` `OnClose GET を発行し握手を開始` → `23:25:02.136443Z` `OnClose 応答なし（204）——追加イベントなしで無言終了へ`・`shots\run1\7-farewell-1..5.png`。SSP は `ssp-measurement.md` 2 節の段 7 | 11 の結果。ぱすたの辞書に `OnClose` の受け口は無く（`dic\` の中に `OnClose` の語が 0）、SSP の終わりの台詞は × の `OnMouseClick` の応答から出た。`OnClose` に 204 が返ったのはぱすたの答えどおり。原因の場所は 11 と同じ（`input_events/mod.rs:483`） | 引き取る spec | `areka-P0-mouse-click-wheel-events` |
| 14 | 右クリックメニューに「着せ替え」の項目が無く、着せ替えを選ぶ操作を回せなかった。SSP は着せ替えのメニュー（口・目・眉・うで・まばたき・ボール・閉じる - 赤）が出て、選ぶと面が変わった | run1 の記録 `2026-10-22T23:23:44.627931Z` `[menu] shown event="menu_shown" scope=0 items=13`（`logs\run1.stage6-excerpt.log`）・`shots\run1\6-menu.png`・`6-menu-shell-sub.png`。SSP は `ssp-measurement.md` 2 節の段 6 | `crates/areka/src/menu/mod.rs:49`。メニューの枠 `Frame` に着せ替えの枠が無い。シェルの `sakura.menu,auto` を読む所も無い | 引き取る spec | `areka-P0-popup-menu-shell-look` |
| 15 | 右クリックメニューが、シェルの `menu.*` の絵と色を使わない普通の Windows のメニューで出た。SSP もシェルの絵と色で「普通の Windows のメニューに見える」ので、見た目が違うかは開発者の判定待ち | `shots\run1\6-menu.png`。SSP は `stage6-menu-shot.png` | `crates/areka/src/menu/mod.rs:49` の枠のまま普通のメニューを組み、シェルの `menu.*` の 21 の鍵を読む所が無い（網羅台帳で absent） | 引き取る spec | `areka-P0-popup-menu-shell-look` |
| 16 | 当たり判定ごとのマウスの形（身体は `system:hand`）を areka は出さない見込み。走行ではマウスの形を読み取っていない（`shots\run1\notes.md` の段 3） | SSP の `stage3-cursor.txt`（身体は `custom`・× とボールは `HAND`）。areka の側は記録なし | シェルの `surfaces.txt` の `sakura.cursor` ブレス（`mouseup*`）を読む所が無い（網羅台帳で absent・`areka-P0-shell-mouse-cursor` の brief の Current State） | 引き取る spec | `areka-P0-shell-mouse-cursor` |
| 17 | 台本の `\_u[0x3000]`（全角の空白を作ってバルーンを出し、重なりの順を指定してから `\c` で消す段取り）を、areka は文字にしない。見た目への効きは走行の記録からは言えない（バルーンとぱすたの前後は alt2 で SSP と同じ向き） | 起動の台本の原文（run1 の記録 `2026-10-22T23:17:58.394554Z` の `areka::log::script`） | 読み手 `crates/areka-parsers/src/sakura/decode.rs` の `decode_tag` に `\_u` の腕が無く、知らないタグとして `Raw` になり、台本の組み立て（`crates/areka-sakura/src/compile.rs:291`）で捨てる | 引き取る spec | `areka-P0-script-small-tags` |
| 18 | 台本の `\![bind-noevent,…]` に受け手が無い。見た目への効きは走行の記録からは言えない | 網羅台帳の行（10 節）・ぱすたの辞書と Lua の台本 | 着せ替えの受け手は名前が `bind` ちょうどのものだけを受ける（`crates/areka-seriko/src/actor.rs:492`） | 引き取る spec | `areka-P0-script-small-tags` |
| 19 | 台本の `\![set,windowstate,!stayontop]` に受け手が無い。見た目への効きは走行の記録からは言えない | 起動の台本の原文（run1 の記録 `2026-10-22T23:17:58.394554Z`） | 受け手の表 `crates/areka/src/emo2_boot/consumer_ledger.rs:27` の決まり（名簿に無い第 1 引数 `windowstate` は担当なし） | 引き取る spec | `areka-P0-stay-on-top-directives` |
| 20 | 外の機能から位置を読む `get_property` の `currentghost.scope(0).rect`・`scope(1).rect`・`balloon.scope(0).rect` が `NG:Cannot find such property name.` で答えた。ぱすたの動きではなく、走行の測り方（窓の位置を撮影と記録の `bounds=` で読み替えた）に効いた | `shots\run1\notes.md` の「走行」 | `crates/areka/src/mcp/get_property.rs:48`。値を出す所が無い名前は `NotFound` で断る（その値を出す所がまだ無い） | 引き取る spec | `areka-P0-mcp-scope-rect-property`（v0.1.0・2026-10-23 起票）。前の持ち主 `areka-P0-currentghost-property-tree`（v0.3.0）・`areka-P0-currentghost-property-others`（いつか）は v0.1.0 の外なので、開発者の決め（6 節）で起票した |
| 21 | 記録の 9 割が `apply(ShowSurface): 表示・マスクを更新` の INFO 行だった（6,073 行のうち面 1002 が 5,989 行・毎秒十数行・どれも `cache_hit=false`）。判定に要る行が埋もれる | `logs\run1.stdout.log`（6,546 行）を数えた。alt2 でも 3,650 行 | 再描画の回数そのものは、面 1002 のボールの 3 枚（`window/ball/eback.webp`・`base_rings.webp`・`efront.webp`）が動く WebP（どれも `ANIM` の塊を持つ）で、コマが替わるたびに合成し直す決まりどおり（完了 `areka-P0-animated-image-playback`）。記録の量は、適用 1 回ごとに INFO を 1 行出すこと（`crates/areka-emo-present/src/presenter/show.rs:402`） | 引き取る spec | `areka-P0-animated-surface-log-flood`（v0.1.0・2026-10-23 起票・6 節）。もとは持ち主なし（`areka-P0-real-run-log-noise` の範囲は文字の層の毎フレームの `debug!` 2 種類と同梱の emo2 の WARN だけ、`areka-P0-present-emit-tail-latency` は適用の時間の尾を扱い「動く絵の適用の回数を減らすこと」を範囲の外にしている）。`areka-P0-real-run-log-noise` へはまとめない（同日の開発者の決め） |
| 22 | α の小さい画素の当たり（SSP では当たり、areka では通り抜ける薄い所）。本走行と補いの走りで見つかった件は 0。面 1002 で撫でる入力が素通りした所（12）は α が 0 の所で、SSP も当たらない見込みの所なので、この件に入れない | 7 節・`window/canvas.png` の α を数えた値（12 の根拠） | 方針の裁定。当たりは α が 0.5 以上の画素だけ（7 節） | 方針で受けない | 2026-10-22 開発者。出どころ: 本 spec の設計の話し合いの議題 1（コミット `3f539637`）と要件 7.1・7.2（7 節の「裁定」） |

### 数え

- 表の件数は 22。扱いの内訳: 引き取る spec 19・方針で受けない 2（8・22）・直した 1（6・タスク 5.4）。19＋2＋1＝22。直した 6 を「本 spec が引き取る」と読めば、引き取る先のある件は 20・方針で受けない件は 2。
- **引き取る spec の名前も方針の裁定も無い件は 0 件**。持ち主の居なかった 2 件のうち、6 は本 spec のタスク 5.4 で直し、21 は新しい spec `areka-P0-animated-surface-log-flood` が引き取る。20 は引き取る spec を v0.1.0 の新しい spec `areka-P0-mcp-scope-rect-property` へ替えた（6 節・2026-10-23 開発者の決め）。
- 引き取る spec ごとの件数: `areka-P0-mouse-click-wheel-events` 4（10〜13）・`areka-P0-ghost-inner-balloon` 2（1・2）・`areka-P0-descript-window-keys` 2（4・5）・`areka-P0-script-small-tags` 3（3・17・18）・`areka-P0-popup-menu-shell-look` 2（14・15）・`areka-P0-window-link-directives` 1（7）・`areka-P0-dpi-realign-remembered-chain` 1（9）・`areka-P0-shell-mouse-cursor` 1（16）・`areka-P0-stay-on-top-directives` 1（19）・`areka-P0-mcp-scope-rect-property` 1（20）・`areka-P0-animated-surface-log-flood` 1（21）。合わせて 19。
- 開発者の判定待ちの目視の項目: 15（メニューの見た目）・重なりの順（窓 → バルーン → ぱすた。6 のため窓の枠が別の画面に在り、3 つの並びは主画面で見えていない。ぱすたとバルーンの前後は alt2 で SSP と同じ向き）・縦書きの列と枠の並び（alt2・参考）。判定で崩れが見つかれば、この表に件を足す。

### 崩れではないと分かったこと

記録だけでは崩れに見えるが、調べて崩れではないと分かった物。表の数に入れない。

- **起動の台本が `\0\s[1002]`（身体の無い面）で終わる**: ぱすたの初回の起動の台本どおり（`dic/boot.pasta` の `＊初回の呼びかけ` の `shokai_hikaru` が `\0\s[1002]` を出す）。SSP の基準の起動は記憶のある起動で `OnBoot` が届き、別の台本（`\s[1000]` で始まり身体が映る）だった（`ssp-measurement.md` 1 節・`target\pasta-verify\ssp\stage1-pasta-log.txt` の応答の Value）。areka は記憶の置き場が空なので `OnFirstBoot` を送った（正典どおり）。映らないまま進まなかったのは 12 の件。
- **`OnSecondChange` がぱすたの記録に 1 行も無い**（10 節の残りの問い）: areka は毎秒送っている（`crates/areka-kanade/src/schedule/events.rs:429` の `on_second_change`）。ぱすたの記録 `pasta.log` は、応答の本文を返した要求だけを残す。SSP の測りでも、毎秒届くはずの `OnSecondChange` は段ごとに 0〜3 行しか残っていない（`stage1`〜`stage7-pasta-log.txt`）。run1 で areka が確かに送り 204 が返った `OnClose` も、`logs\run1.pasta.log` に 1 行も無い。run1 の間は、ぱすたの `初回待ち` の印（12）が `OnSecondChange` の応答を止めていたので、記録に残る応答が無かった。areka の崩れではない。網羅台帳の `OnSecondChange` の行（implemented）は記録と食い違わない。
- **`OnClose` に 204 が返った**: ぱすたの辞書に `OnClose` の受け口が無いため（13）。
- **seriko の WARN 96 行**（`mustselect カテゴリの脱衣指示を無視`）: ぱすたの台本の `\![bind,<部位>,<名>,0]` を、正典どおり無視したもの（網羅台帳の `sakura.bindoption*.group` の行）。
- **`areka_emo_present::balloon` の縮退の WARN 6 行**（run1・`StayseeBalloon` の `\1` の面 2・3）: 1 の結果で出た既定のバルーンの定義の作りで、縦書きのバルーンの alt2 では 0 行。
- **`areka_emo_atlas` の WARN**（`pasta/null.png`・縦書きのバルーンの `balloonk0.png` が全透明）: ぱすたの中身の絵が全透明なため。

### 手順の側の件（areka の崩れではない）

- 本走行で、起動の直後に areka.exe の空のコンソールの窓が主画面の手前に開き、ぱすたの窓を覆った（`shots\run1\notes.md`・`1-boot-full-a.png`）。手順書の `Start-Process` が窓を隠さないため。補いの走りでは `-WindowStyle Hidden` を足して出なくなった。手順書の直しはタスク 8.1。
- 補いの走りの準備でゴーストの記憶を写してしまう件（4 節の「手順の穴」）。手順書の直しはタスク 8.1。
- `get_property` の rect が読めないため、窓の位置を撮影と記録で読み替えた（20 の件）。手順書には読み替え方を書く（タスク 8.1）。

## 6. 直しの見積もりと 20 本の線

5 節の表で持ち主の居なかった 2 件（6・21）と、引き取る spec が v0.1.0 の外に居た件（20）を、1 タスクで閉じて本 spec で直すものと、`/kiro-discovery` で起票するものに分けた（2026-10-23・タスク 5.3・要件 5.4・6.2・6.3）。あわせて、走行の崩れではないが、ぱすたが使う機能で持ち主が v0.1.0 の外に居たものも、申し送りとして起票した。

### 開発者の決め（2026-10-23・チャット）

- 「0.1.0までに解決しない場合、独立して起票せよ」
- 「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」
- 崩れ 6 は本 spec のタスク 5.4 で直す（承認）。直すとすべてのゴーストの起動の位置に効くので、直す前に決めを仰いだ（要件 6.4）。
- 崩れ 20 は新しい spec `areka-P0-mcp-scope-rect-property`（v0.1.0）、崩れ 21 は新しい spec `areka-P0-animated-surface-log-flood`（v0.1.0）が引き取る。21 は版に縛らない列の `areka-P0-real-run-log-noise` へはまとめない。

### 件ごとの扱い

| 件 | 1 タスクで閉じるか | 扱い | 見積もり | 引き取る spec／決めの出どころ |
|---|---|---|---|---|
| 6 | 閉じる。判定 1 か所（x だけで比べる所を x と y で比べる）と、既定の位置を控えて渡す所の直しで、触るファイルは計画の 5.4 に挙げた 6 本 | 直した（タスク 5.4・コミット `97186487`） | 1 タスク。今ある 5.4 が受け持ち、タスクは足さない | 本 spec。2026-10-23 開発者の承認（上の決め） |
| 21 | 本 spec では直さない。2 件目の直しを足すと本 spec が 20 タスクを超える。直す所の `crates/areka-emo-present/src/presenter/show.rs` は `areka-P0-present-emit-tail-latency`・`areka-P0-mcp-strict-errors` も触り、同じウェーブに置けない | 引き取る spec（起票） | S（2〜4）（正本のロードマップの spec 台帳の見立て） | `areka-P0-animated-surface-log-flood`（v0.1.0）。2026-10-23 開発者の決め |
| 20 | 本 spec の範囲の外（sylphya に動く値の口を作る仕事）。前の持ち主 `areka-P0-currentghost-property-tree`（v0.3.0）・`areka-P0-currentghost-property-others`（いつか）は v0.1.0 の外 | 引き取る spec（起票） | M（8〜12）（同上） | `areka-P0-mcp-scope-rect-property`（v0.1.0）。2026-10-23 開発者の決め（「0.1.0までに解決しない場合、独立して起票せよ」） |

### 20 本の線の判定

- 今の計画は 19 タスク（小タスク 18 と、小タスクを持たない 6）。直しの 1 件目（6）は今ある 5.4 が受け持つので、タスクは足さず **19 タスクのまま**。20 本の線は越えない。
- 2 件目を本 spec で直すと 20 を超えるので、21 は本 spec へ足さず、開発者の決めのとおり起票した（要件 6.3。黙って起票へ回したのではない）。
- 5.3 で足すタスクは 0。

### 直した結果（崩れ 6・タスク 5.4）

- 直した所: 起動の最後の並べ直し `finalize_chain`（`crates/areka/src/placement/chain_finalize.rs`）が、組の窓が既定の位置のままかを x と y の両方で比べるようにした。既定の位置を集めて渡す所は `crates/areka/src/emo2_boot/frame/drain_resnap.rs` の `collect_chain_states`。
- 赤から緑のテスト: `chain_finalize_tests::vertically_moved_scope_is_not_pulled_back`。直す前は `[ChainMove{scope:1,new_x:-108},…]` を返して失敗し、直した後は緑。`frame_chain_finalize_persist_tests` のテスト 11c は前提を直した。`cargo test -p areka --bin areka` は 3336 件 ok。
- 上書きした完了 spec の要件（要件 6.6）: 完了 `areka-P0-scope-chain-gap` の要件 7.3 は書かれたとおりに当てた（x だけで比べていた判定を x と y へ広げた。7.2 は変えていない）。完了 `areka-P0-char-position-save-on-exit` の要件 1.7 の振る舞いは変えておらず、テスト 11c の前提と「x だけ」と書いた注釈だけを直した。拡大率の後の詰め直しの道も同じ判定を使う（振る舞いはほぼ変わらない）。
- 実機の回し直し（タスク 5.5・rerun1・2026-10-23 01:25〜01:30Z・`<w>\target\pasta-verify-rerun\shots\rerun1\notes.md`）の要点: 記録に `scope=1` の `chain_finalize` の行は 0 件で、`\1` は台本どおり `\0` の左上（0,864）へ動いたまま、ぱすたの奥に出た（`1-boot-primary-ghost.png`）。途中で開発者が手でぱすたと窓の枠をドラッグしたので、それより後の位置は手で置いた位置。段ごとの結果はタスク 5.5 で書く。

### 申し送り（走行の崩れではない・ぱすたが使う機能で、持ち主が v0.1.0 の外に居たもの）

10 節の突き合わせで「本 spec の範囲の外」とした項目のうち、ぱすたが使うのに持ち主が v0.1.0 の外に居た（または居なかった）ものを、2026-10-23 に `/kiro-discovery` で v0.1.0 の spec として起票した（上の開発者の決め）。崩れの表の数には入れない。

| 引き取る spec | ぱすたの使い方 | 前の持ち主 | 見積もり | 台帳の行（10 節） |
|---|---|---|---|---|
| `areka-P0-shell-scale-linkage` | `\![get,property,…]` で `currentghost.scope(0).scaling`・`currentghost.balloon.scope(0).scaling`・`currentghost.scope(0).currentmonitor.dpi` を聞く・`OnShellScaling`／`OnBalloonScaling` を受ける・ゴーストの descript の `balloon.syncscale,true` | `\![get,property]` は `areka-P0-property-query-channels`（いつか）・`scope(ID).scaling` と `currentmonitor.dpi` は `areka-P0-currentghost-property-others`（いつか）・`balloon.scope(ID).scaling` は `areka-P0-currentghost-property-tree`（v0.3.0）・`balloon.syncscale` は `areka-P0-shell-balloon-scale-setting`（v0.4.0）・2 つのイベントは持ち主なし | M（10〜14） | 7 行 |
| `areka-P0-balloon-scroll-arrows` | 縦書きのバルーンの descript の `arrow0.x/y`・`arrow1.x/y` | `areka-P0-balloon-markers`（版に縛らない列）。台帳の宛先は `areka-P0-balloon-canon-residue` | M（11〜13） | 4 行 |
| `areka-P0-balloon-communicatebox` | 縦書きのバルーンの descript の `communicatebox.x/y/width/height` | 持ち主なし | M〜L（12〜18） | 4 行 |

見積もりは正本のロードマップの spec 台帳の見立て（`.kiro/steering/roadmap.md` の「pasta-in-windows-verification の申し送り」）。各 spec の brief に、証拠（本記録の節と表の欄）が書かれている。

`currentghost.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi`・`currentghost.scope(ID).rect` の前の持ち主は、brief と正本のロードマップでは `areka-P0-currentghost-property-others`、台帳の宛先では `areka-P0-currentghost-property-tree` で、食い違っていた。この食い違いは今回の付け替えより前からあったもので、今回どちらからも引き取る spec へ移したので、2 つの spec が同じ項目を持つことにはなっていない。

### 申し送りの漏れが無いことの確かめ（10 節の残りの行）

10 節の突き合わせの表のうち、ぱすたが使い、実装済みでなく、宛先が空か v0.1.0 の外の spec である行を全部挙げ、1 行ずつ扱いを決めた。上の 3 本と 5 節の表に入った行は除いた。「判定待ち」の見比べの材料は、案 A の回し直し rerun1（`<w>\target\pasta-verify-rerun\shots\rerun1\`・`notes.md`。代わりの操作で縦書きのバルーンと身体を出した）で、開発者の判定はまだ無い。判定で崩れが出たら、5 節の表に足して v0.1.0 の持ち主か新しい起票へ渡す。

| 行（10 節） | 今の値（宛先と版） | 扱い | 理由・見比べる物 |
|---|---|---|---|
| ゴースト `type`・シェル `type` | absent・宛先なし | 見た目・振る舞いに効かない | 書庫の中の種類の宣言。areka は置き場（`ghost\`・`shell\`）で種類を決め、ぱすたの値（`ghost`・`shell`）と同じになる |
| ゴースト `craftmanurl`・シェル `craftmanurl` | absent・宛先なし | 見た目・振る舞いに効かない | 作者の連絡先の URL で、ぱすたの絵・台詞・当たり・位置のどれにも使われない |
| ゴースト `icon` | absent・宛先なし | 見た目・振る舞いに効かない | ベースウェアがタスクバーなどに出す印の絵で、ぱすたの窓の中の絵・台詞・当たり・位置には効かない |
| シェル `seriko.alignmenttodesktop`（ぱすたは `free`）・`sakura.defaultx`（`0`） | absent・宛先なし | 判定待ち（案 A の rerun1 の見比べ／案 B） | areka は読む（`crates/areka/src/placement/config.rs:207`〜`221` で `free` を `Alignment::Free` に・`223`〜`227` で `defaultx`）。台帳の absent は、ぱすたが使わない別の意味（`top`・`left`・`right` など）の分。見比べる物: `1-boot-primary-ghost.png`（手のドラッグの前の起動の位置） |
| シェル `sakura.balloon.offsetx`（`476`）・`offsety`（`58`） | absent・宛先なし | 判定待ち（案 A の rerun1 の見比べ／案 B） | areka は両方そろったときに読む（`config.rs:252`〜`262`）。台帳の absent は、ぱすたが使わない片方だけの指定などの分。見比べる物: `2-choice-zorder.png`・`2-tategaki-burst-03.png` |
| surfaces の descript ブレスの `version`（ぱすたは `1`） | absent・宛先なし | 見た目・振る舞いに効かない | areka は宣言を読まずに新しい書式で読む。ぱすたは新しい書式（`1`）を宣言しているので、読み方は同じになる |
| `animation*.interval` の値 `bind` | vocabulary-only・宛先なし | 判定待ち（案 A の rerun1 の見比べ／案 B） | 着せ替えは静的に重なる。メニューから着せ替えを選ぶ操作は 5 節の 14（`areka-P0-popup-menu-shell-look`）、× の赤は 10（`areka-P0-mouse-click-wheel-events`）が引き取る。見比べる物: `1s-subst-surface0.png` |
| `\p[ID]` | degraded・宛先なし | 見た目・振る舞いに効かない | 縮退は番号が 2 以上の話し手だけ（台帳の `note`）。ぱすたの話し手は `\0` と `\1` の 2 人で、その道は正常 |
| リソース `sakura.recommendsites` | vocabulary-only・宛先なし | 判定待ち（案 B） | SSP はメニューを組むときに求めた。メニューの見た目は 5 節の 15 と一緒に判定する（rerun1 はメニューを「終了」のためにだけ開いた・`9-menu-for-quit.png`） |
| バルーン `font.shadowcolor.*` | vocabulary-only・`areka-P0-text-align-shadow-canon`（いつか） | 判定待ち（案 A の rerun1 の見比べ／案 B） | 見比べる物: `2-tategaki-burst-03.png`・`2-tategaki-burst-09.png` |
| バルーン `anchor.font.color.*` | absent・`areka-P0-anchor-style-canon`（版に縛らない列） | 判定待ち（案 A の rerun1 の見比べ／案 B） | ぱすたの台本の選択肢は `\q` で、リンク（`\_a`）の字は走行に出ていない。見比べる物: `2-choice-zorder.png` |
| バルーン `cursor.*` の 9 行（`cursor.style`・`cursor.blendmethod`・`cursor.pen.color.*`・`cursor.notselect.style`・`cursor.notselect.font.color.*`） | absent か vocabulary-only・`areka-P0-choice-marker-styling`（いつか） | 判定待ち（案 A の rerun1 の見比べ／案 B） | 選んでいる選択肢の見た目。rerun1 では選ぶ前の反転を撮れていない（`notes.md` の段 2）。見比べる物: `2-choice-zorder.png`・`2b-after-choice-1.png` |
| バルーン `use_self_alpha`・シェル `seriko.use_self_alpha` | degraded・完了 `areka-P0-self-alpha-declaration` | 判定待ち（案 A の rerun1 の見比べ／案 B） | 見比べる物: `1s-subst-zorder.png`・`2-choice-zorder.png` |
| バルーン `vertical` | degraded・完了 `areka-P0-balloon-vertical-canon` | 判定待ち（案 A の rerun1 の見比べ／案 B） | 見比べる物: `2-tategaki-burst-03.png`・`2-tategaki-burst-09.png` |
| バルーン `font.name` | degraded・完了 `areka-P0-balloon-font-descript-keys` | 判定待ち（案 A の rerun1 の見比べ／案 B） | 見比べる物: `2-tategaki-burst-03.png` |
| element 定義 `element*` | degraded・`areka-P0-draw-methods-canon`（v0.2.0） | 判定待ち（案 A の rerun1 の見比べ／案 B） | ぱすたの描画メソッドは `overlay` だけ（実装済み）。見比べる物: `1s-subst-surface0.png`・`1s-subst-surface1.png` |
| `collision*` | degraded・完了 `areka-P0-shell-parse` | 判定待ち（案 B） | ボールと × の当たりは取れた。顔を撫でる当たりは身体の出る起動（5 節の 12 の後）で見る |
| `animation*.interval`・`animation*.pattern*` | degraded・完了 `areka-P0-shell-parse` | 判定待ち（案 A の rerun1 の見比べ／案 B） | 見比べる物: `1s-subst-surface0.png`（ボールの動く絵） |
| `\![move,…]` | degraded・完了 `areka-P0-move-canon` | 判定待ち（案 A の rerun1 の見比べ／案 B） | rerun1 では `\1` が台本どおり `\0` の左上（0,864）へ動き、起動の並べ直しで送り直されなかった（上の「直した結果」）。見比べる物: `1-boot-primary-ghost.png` |

- 合わせて 19 欄。「見た目・振る舞いに効かない」が 5 欄・「判定待ち」が 14 欄。宛先が空のまま、扱いの無い行は 0。

### 縦書きのバルーンの鍵（判定できなかったもの）と、その段取り

縦書きのバルーンの descript の `font.name`・`font.shadowcolor.*`・`cursor.*`・`anchor.font.color.*`・`vertical`・`use_self_alpha` は、シェルがまともに出ていないため、SSP と同じかを判定できなかった。

- 開発者の言葉（2026-10-23）: 「画像に関しては全然違う。比較とか言うレベルじゃない。シェルがまともに出てない。」・「画像をまず直してもらわないと裁定は難しいね。」
- 案 A（いま・承認済み）: タスク 5.5 で、6 を直した後に段 1・2 を回し直す。そのとき縦書きのバルーンを根の `balloon\` へ写し、MCP で代わりの操作 `\0\s[1000]\1\s[10]` を打って身体と窓の枠を出す。代わりの操作の結果は別に書き、段の 3 値の数には入れない。描き方の崩れが見つかれば、5 節の表に足して v0.1.0 の持ち主か新しい起票へ渡す。
- 案 B（最後の裁定）: 設計のとおり、兄弟の spec が着地した後の回し直しで決める。
- 結果はまだ無い（5.5 で書く）。

## 7. α の決まり

ukadoc が書いていない所（絵の薄い画素の当たり）についての areka の決め。コードは変えていない。

### 決まり

- **当たり**: 画素の α が 0.5 以上（8 ビットの値で 128 以上）の所だけを当たりにする。α が 0.5 未満の所は当たりにしない（7.1）。
  - シェルとバルーンの絵の当たり: `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs` の `ALPHA_THRESHOLD: u8 = 128`。
  - ウィジェットの当たり: `crates/wintf/src/ecs/layout/hit_test/mod.rs` の 2 か所の `128.0 / 255.0`（見た目の不透明度 × 塗りの α と比べる）。
- **描き方**: α が 1 以上の画素はすべて描く。今の描き方のまま（7.2）。アトラスの切り詰め（`trim.rs` の α > 0）は当たりとは別の話で、これも触っていない。
- **SSP との違い**: SSP は α が 0 でなければ当たる見込み。これは開発者の見立てで、測っていない（7.4）。areka はこれに合わせない。

### 裁定

- 日付: 2026-10-22（開発者）。
- 出どころ: 本 spec の設計の話し合いの議題 1（コミット `3f539637`「α の当たりは 0.5 以上のまま SSP に合わせず、α の測りを外す」）と、要件 7.1・7.2。
- 理由（開発者の言葉）: 「ヒットテストのルールはwintfの決定にも影響する。α＜0.5の領域はそもそもヒットテスト領域にすべきではない。」
- 描き方: SSP が α 8 以下の画素を描かなかった件は SSP のバグで解消済みのため、写さない（同日の開発者の裁定・要件 7.2）。
- 走行で「SSP では当たるのに areka では通り抜ける薄い所」が見えたら、5 節の崩れた所の表に「方針で受けない」として、この裁定の日付と出どころをつけて書く（7.3）。

### 境のテストの確かめ（2026-10-23・コミット `c8c80bd6` の上）

既存の境のテストを変えずに回した。どれも緑。机は要らない（クレートのテストだけ）。

| 命令 | 確かめる所 | 結果 |
|---|---|---|
| `cargo test -p wintf --lib test_threshold_boundary_128` | `alpha_mask.rs` で α がちょうど 128 の画素が当たる | 1 件 ok・失敗 0 |
| `cargo test -p wintf --lib test_hit_test_entity_bounds_alpha_boundary` | `hit_test/tests_ex/opacity.rs` の線のすぐ上と下（`_above`・`_below`） | 2 件 ok・失敗 0 |
| `cargo test -p wintf --lib hit_test::tests_ex::opacity` | `opacity.rs` の全部 | 6 件 ok・失敗 0 |

`alpha_mask.rs` と `hit_test/mod.rs` の `main` との差分は 0 行（`git diff main -- <2 つのファイル>` の出力が空）。

## 8. 申し送り

5 節の崩れた所の表の「引き取る spec」の件と、SSP の測りで見えた兄弟の問いの答え（`ssp-measurement.md` 3 節）を、兄弟の spec の `brief.md` の末尾へ申し送った（2026-10-23・タスク 6・要件 1.4・5.5・5.8）。

- 開発者の決め（2026-10-23・チャット）: 「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」
- 書いた節の名前は「本 spec からの申し送り（2026-10-23・`areka-P0-pasta-in-windows-verification` のタスク 6）」。各節には、出どころ（本記録と測りの記録・撮った物の置き場・時刻は `Z`）、件ごとの症状・根拠（撮った物の名前と記録の行と時刻）・原因（file:line）、SSP の基準、着地で変わるはずの所（`doc/pasta-real-run.md` 5 節の表の欄）を書いた。`dpi-realign-remembered-chain` は、タスク 5.3 で足した同じ日の節（「2026-10-23 `pasta-in-windows-verification` からの申し送り」）が末尾に在ったので、新しい節を立てずにその節へ足した。
- 申し送り先の完了済みの spec は 0 本（`.kiro/specs/completed/` に同じ名前は無い）。完了済みの兄弟のために `/kiro-discovery` で起票した物は無い。

### 兄弟の brief へ書いたもの（2026-10-23）

| brief | 件（5 節の番号） | 要点 | 測りの答え（`ssp-measurement.md`） |
|---|---|---|---|
| `areka-P0-ghost-inner-balloon` | 1・2 | 縦書きのバルーンをゴーストのフォルダの中から見つけない（`boot_resolve.rs:294`〜`297`）。その結果、横書きのバルーンで `\_l[x,0]` の後が重なる。根の `balloon\` へ写した alt2・rerun1 では重ならない | 3 節の 1 行目（SSP はゴーストのフォルダの中の `pasta_tategaki` を使った）・2 節の段 2 |
| `areka-P0-script-small-tags` | 3・17・18 | 選択肢の前の `\![*]` が出ない（run1・alt2・rerun1）・`\_u[0x3000]` を捨てる・`\![bind-noevent]` に受け手が無い | 3 節の 2 行目（印が 4 つの選択肢の前に出た） |
| `areka-P0-descript-window-keys` | 4・5 | 起動の位置を決める窓の大きさを面 0 で測る（`measure.rs:173`）ため、ぱすたの窓が下端を越え、alt2 ではバルーンが × の板にかかった。`sakura.seriko.defaultsurface,1000` を読まない | 3 節の 4 行目（`balloon.dontmove`・バルーンはぱすたに付いたまま動いた）・2 節の段 1 |
| `areka-P0-window-link-directives` | 7（6 は本 spec で直した） | 6（起動の並べ直しが x だけで比べていた）は本 spec のタスク 5.4・コミット `97186487` で直したことと、直した後の形。残りの持ち分は 7（ドラッグで `\1` が付いてこない・`config.rs:107`・`135`） | 3 節の 5 行目（`sticky-window` で 2 つが揃って動いた）・2 節の段 4 |
| `areka-P0-dpi-realign-remembered-chain` | 9 | 拡大率の違う画面へドラッグして落とすとき、掴んだ点の窓の中の位置が拡大率に合わせて縮められない（`crates/wintf/src/ecs/window_proc/mouse_move.rs:302`〜`303`）。この brief の Scope にはまだ無いので、要件の段で範囲に入れること。8（大きさが縮む）は方針で受けない | 3 節の 3 行目（SSP は大きさを変えず、`OnShellScaling` も送らない）・2 節の段 5 |
| `areka-P0-mouse-click-wheel-events` | 10〜13 | `OnMouseEnter`／`OnMouseLeave` を送らない（`events.rs:114`）・単発のクリックを送らない（`input_events/mod.rs:483`）。その結果、身体が映らず、× で終われず、終わりの台詞が出ない | 3 節の 7 行目・2 節の段 3・段 7 |
| `areka-P0-popup-menu-shell-look` | 14・15 | 着せ替えの枠が無い・`menu.*` を読まない（`menu/mod.rs:49`）。15 の見た目は開発者の判定待ち。`sakura.recommendsites` も 15 と一緒に判定する | 3 節の 8 行目・2 節の段 6 |
| `areka-P0-shell-mouse-cursor` | 16 | `sakura.cursor` ブレスを読まない。areka の側はマウスの形を読んでいない | 3 節の 6 行目（身体は `system:hand`・× とボールは既定の `system:finger`） |
| `areka-P0-stay-on-top-directives` | 19 | `\![set,windowstate,!stayontop]` に受け手が無い（`consumer_ledger.rs:27`） | 3 節の終わり（`\1\v` は通しに出ず、見えた答えは無い） |

### 起票した spec（2026-10-23・タスク 5.3 の `/kiro-discovery`）

5 本とも起票のときに brief へ証拠（本記録の節と表の欄）が書かれている。タスク 6 で 5 本の brief を読み直し、症状・根拠・原因・SSP の基準・着地で変わる所が揃っていたので、足した物は無い。

| spec | 引き取る件 | 版 |
|---|---|---|
| `areka-P0-mcp-scope-rect-property` | 5 節の 20（`get_property` の rect が読めない）。SSP の基準（2 節の段 1・段 4・段 5 の rect）は brief の Current State に在る | v0.1.0 |
| `areka-P0-animated-surface-log-flood` | 5 節の 21（`apply(ShowSurface)` の INFO 行が記録の 9 割） | v0.1.0 |
| `areka-P0-shell-scale-linkage` | 6 節の申し送り（倍率と DPI のプロパティ・`OnShellScaling`／`OnBalloonScaling`・`balloon.syncscale`） | v0.1.0 |
| `areka-P0-balloon-scroll-arrows` | 6 節の申し送り（縦書きのバルーンの `arrow0.x/y`・`arrow1.x/y`） | v0.1.0 |
| `areka-P0-balloon-communicatebox` | 6 節の申し送り（縦書きのバルーンの `communicatebox.x/y/width/height`） | v0.1.0 |

前の持ち主の brief（`areka-P0-currentghost-property-tree`・`areka-P0-currentghost-property-others`・`areka-P0-property-query-channels`・`areka-P0-balloon-canon-residue`・`areka-P0-balloon-markers`・`areka-P0-shell-balloon-scale-setting`・`areka-P0-balloon-scroll-fade`・`areka-P0-inputbox-user-input`・`areka-P0-real-run-log-noise`）には、タスク 5.3 で付け替えを書いた。

### 「ぱすた開発」への申し送り（待受のぶつかり・要件 5.8）

- 送った日: 2026-10-23（開発者の了承を得て、セッション `local_c4b2ef26-2601-42fe-a8d9-465fcd644677` へ）。中身: SSP と areka で同時にぱすたを起こすと、ぱすたのデバッグの待受 127.0.0.1:9276 がぶつかる。いまの逃げ方は片方だけを起こすこと。ぱすたの側で避けるか、いつ避けるかを問うた。
- 答え（2026-10-23 に受け取った）の要点:
  - ぱすた（ゴースト）は何も変えない。配る物と開発の物の `pasta.toml` には `[debug]` が無く、既定はデバッグが切れているので、配られたぱすたは待受を開かず、利用者の所ではぶつからない。
  - 待受を開いているのは、開発者の利用者の環境変数 `PASTA_DEBUG=1` と `PASTA_DEBUG_PORT=9276` で、ぱすたの決まりどおり `pasta.toml` より優先される。このため、開発者の PC では、ぱすたで作ったゴースト（emo2 も）はどれも 9276 を取り合う（1 つの SSP の中の 2 体でも）。
  - 直しなしの逃げ方: areka を `PASTA_DEBUG` を消したシェルから起こす（PowerShell で `$env:PASTA_DEBUG = $null`）か、両方をデバッグしたいときは `PASTA_DEBUG_PORT` を別の番号（例 9277）にする。ぱすたの決まりから判断したもので、実機ではまだ試していない（手順の段で確かめる）。
  - 空いた番号へ自動でずらすなどのぱすたのエンジンの変更は、pasta のリポジトリの判断で、ゴーストのセッションでは決めない。環境変数を付けたままにするかは開発者が決める。
- 本 spec の扱い: areka の側では直さない。手順書 `doc/pasta-real-run.md` の「SSP のぱすたを閉じてから」（2 節の表の「SSP のぱすたを閉じてあること」の行）は、タスク 8.1 で、起動の塊の中で `PASTA_DEBUG` を消す形に置き換え、次の走行で効くことを確かめる。

### 漏れの確かめ

- 5 節の表の「引き取る spec」の 19 件は、上の 2 つの表のどれかに入っている: 1・2（`ghost-inner-balloon`）・3・17・18（`script-small-tags`）・4・5（`descript-window-keys`）・7（`window-link-directives`）・9（`dpi-realign-remembered-chain`）・10〜13（`mouse-click-wheel-events`）・14・15（`popup-menu-shell-look`）・16（`shell-mouse-cursor`）・19（`stay-on-top-directives`）・20（`mcp-scope-rect-property`）・21（`animated-surface-log-flood`）。直した 6 は `window-link-directives` へ直したことを伝えた。方針で受けない 8・22 は申し送りの対象ではない（9 の申し送りに 8 を崩れとして扱わないことを添えた）。申し送りの届いていない「引き取る spec」の件は 0 件。
- `ssp-measurement.md` 3 節の 8 行の問いは、申し送り先の brief へ答えを書いた（8 行とも上の表の右の欄）。`\1\v` は答えが無いことを `stay-on-top-directives` へ書いた。
- 申し送り先の 14 本（上の 2 つの表）は、正本のロードマップ `.kiro/steering/roadmap.md` の版の段の v0.1.0 の行に名前があり、どれも未完了。**v0.1.0 の外の申し送り先は 0 本**。
- 6 節の「申し送りの漏れが無いことの確かめ」の判定待ちの 14 欄は、開発者の判定がまだ無いので申し送っていない。判定で崩れが出たら、5 節の表に足して v0.1.0 の持ち主か新しい起票へ渡す。

## 9. 手順の置き場と最後の確かめ

- 置き場: `doc/pasta-real-run.md`（spec のフォルダの外。完了で本 spec が `completed/` へ動いてもパスは変わらない・要件 8.7）。版の段からの道しるべは、`.kiro/steering/roadmap.md` の節「版の段」の表の直後の 1 行「v0.1.0 の版の門の最後の確かめ」。
- 最後の確かめ: 「SSP と同じレベル」の最後の確かめは、兄弟の spec の着地の後に、本 spec の外でこの手順を回し直して行う（v0.1.0 の版の門の最後の確かめ・要件 8.6）。そのための別の spec は起こさない（2026-10-22 の要件の話し合い）。結果は版の門の記録に書く。手順の 5 節の表の「今の引き取る spec」の欄が、どの段がどの spec の着地で変わるはずかを示す。
- 清書で手順へ戻した直し（2026-10-23・タスク 8.1）:
  - 机の命令の識別とリポジトリを、`git rev-parse` から求める 1 行に替え、`request` の前に `watch` を立てる旨を足した（3.1 のレビューの宿題）。
  - SSP の版を、測った版（2.9.23）と「基準は測りの記録の版」に替えた。段 1 の SSP の基準を測りの値（ゴーストを替えて起こし直した起動の `OnBoot`・07:59:49）に、段 5 の SSP の基準を測りの記録 段 5 の値（大きさは 831x1026 のまま・並びは保たれる・バルーンのずれ 60,87）に替えた（2.1）。
  - 起動の `Start-Process` に `-WindowStyle Hidden` を足した（4.1・run1 で空のコンソールの窓がぱすたを覆った）。
  - 「SSP のぱすたを閉じてから」を、起動の塊で `PASTA_DEBUG` を消す形に替えた（8 節の「ぱすた開発」の答え）。9276 を取り合わないことはまだ確かめていない。確かめの命令を手順の 4 節に置き、確かめが済むまでは SSP のぱすたを閉じる逃げ方を残した。
  - 補いの走りの準備に、写した根のゴーストの記憶 `ghost\pasta-in-windows\ghost\master\profile` を消す 1 行を足した（4.2・alt1）。本走行の準備は、根を毎回置き場の展開から組み直すので、前の走行の記憶は残らない（そのことを手順の 3 節に書いた）。
  - 位置は `areka-P0-mcp-scope-rect-property` の着地まで撮影と記録の `bounds=` の行で読むこと（5 節の 20）、`areka-P0-mouse-click-wheel-events` の着地までの代わりの操作（案 A・段の 3 値に数えない）、段 2 の選択肢の 30 秒の時間切れ（`OnChoiceTimeout`）の注意を足した。
  - 7 段の表の「今の引き取る spec」の欄と範囲の外の一覧を、5 節と 8 節の引き取る spec（`mcp-scope-rect-property`・`animated-surface-log-flood`・`shell-scale-linkage`・`balloon-scroll-arrows`・`balloon-communicatebox` を含む）に合わせた。

## 10. 網羅台帳との突き合わせ

ぱすたが使う項目を、取り寄せた書庫を展開した木（`target\pasta-verify\root\ghost\pasta-in-windows`。1 節の `.sha256` の書庫と同じ中身）から拾い、網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の行と突き合わせた（2026-10-23・タスク 7.1・要件 9.1・9.2・9.4）。突き合わせた相手は、測りの記録 `ssp-measurement.md`・最初の走行の控え `target\pasta-verify\shots\run1\notes.md` と記録 `target\pasta-verify\logs\run1.*`・この記録の 1〜3 節と 7 節である。7.1 の時点では崩れた所の表（5 節）がまだ無かったので、原因が記録に書かれていない崩れの分は台帳を変えなかった。5.1 の後、下の表の行のうち 5.1 へ渡していたもの（`seriko.alignmenttodesktop`・`OnSecondChange`）は、5 節の仕分けを指すように直した（台帳の値は変えていない）。

### 拾いの命令（使い捨て・道具にはしない）

ワークツリーの根で Git Bash から回した。台本のタグは、`.pasta` の辞書の中の生の台本だけでは辞書の言語が組み立てるタグ（`\n[数値]`・`\q`・`\_l` など）が拾えないので、走行と測りで実際に出た台本（ぱすたの記録の `Value:` の行と `raise_event` の答えの `Script:` の行）からも拾った。逆斜線は `\x5c` で書いている。

```
R=target/pasta-verify/root/ghost/pasta-in-windows
# descript の鍵（install.txt・ゴースト・シェル・縦書きのバルーン）
for f in install.txt ghost/master/descript.txt shell/master/descript.txt pasta_tategaki/descript.txt; do
  rg -v '^\s*(//|$)' $R/$f | cut -d, -f1 | sed -E 's/[0-9]+/N/' | sort -u; done
# surfaces.txt のブレスと element 定義・当たり・アニメの鍵
rg -v '^\s*(//|$)' $R/shell/master/surfaces.txt | rg -o '^\s*[a-z.*]+[0-9]*(\.[a-z]+[0-9]*)?(,[a-z+]+)?' \
  | sed -E 's/[0-9]+/N/g;s/^\s+//' | sort | uniq -c
# さくらスクリプトのタグ（辞書と Lua の台本）
rg -o --no-filename '\x5c(!\[[a-z_-]+(,[a-z_-]+)?|_[a-z]|[a-z0-9*-])' $R/ghost/master/dic $R/ghost/master/scripts | sort | uniq -c
# さくらスクリプトのタグ（走行と測りで出た台本）
T=target/pasta-verify
rg --no-filename '^\s*(Value|Script):' $T/ssp/stage*-pasta-log.txt $T/logs/run1.pasta.log $T/ssp/stage2-script.txt $T/shots/run1/1-boot-getlog.txt \
  | rg -o '\x5c(![\[][a-z_*-]+(,[a-z_-]+)?|_[a-z]|[a-z0-9*-])([\[][^\],]*)?' | sed -E 's/\[[0-9-]+$/[N/' | sort | uniq -c
# イベントの名前（辞書が受ける名前と、走行と測りで届いた要求）
rg -o --no-filename '\bOn[A-Z][A-Za-z]+' $R/ghost/master/dic $R/ghost/master/scripts | sort | uniq -c
rg -o --no-filename '^ID: [^ \r]+' $T/ssp/stage*-pasta-log.txt $T/logs/run1.pasta.log | sort | uniq -c
```

台帳の行は、拾った鍵とタグとイベントの名前をカタログ `doc/ukadoc-coverage/catalog.toml` の見出し（`title`）と照らして id を引いた（使い捨ての Python の数行。台帳の id はカタログから写した物だけを使った）。

### 拾った一覧

- **install.txt**（4）: `charset`（ファイルの綴りは `Charset`）・`type`・`name`・`directory`
- **ゴーストの descript**（16）: `charset`・`type`・`id`・`name`・`sakura.name`・`kero.name`・`sakura.seriko.defaultsurface`・`balloon`・`balloon.dontmove`・`balloon.syncscale`・`shiori`・`craftman`・`craftmanw`・`craftmanurl`・`icon`・`homeurl`
- **シェルの descript**（38）: `charset`・`type`・`name`・`craftman`・`craftmanw`・`craftmanurl`・`seriko.use_self_alpha`・`seriko.alignmenttodesktop`・`sakura.defaultx`・`sakura.balloon.alignment`・`seriko.sticky-window`・`sakura.balloon.offsetx`・`sakura.balloon.offsety`・`menu.*` の 21（`background`・`foreground` の `bitmap.filename`・`alignment`・`font.color.r/g/b`、`sidebar` の `bitmap.filename`・`alignment`、`disable.font.color.r/g/b`・`separator.color.r/g/b`・`frame.color.r/g/b`）・`sakura.bindgroup*.name`・`sakura.bindgroup*.default`・`sakura.bindoption*.group`・`sakura.menu`
- **縦書きのバルーンの descript**（`pasta_tategaki`・48）: `charset`・`type`・`name`・`craftman`・`craftmanw`・`use_self_alpha`・`vertical`・`origin.x/y`・`validrect.left/top/right/bottom`・`wordwrappoint.y`・`font.name`・`font.height`・`font.color.r/g/b`・`font.shadowcolor.r/g/b`・`anchor.font.color.r/g/b`・`cursor.style`・`cursor.blendmethod`・`cursor.brush.color.r/g/b`・`cursor.pen.color.r/g/b`・`cursor.font.color.r/g/b`・`cursor.notselect.style`・`cursor.notselect.font.color.r/g/b`・`arrow0.x/y`・`arrow1.x/y`・`communicatebox.x/y/width/height`
- **surfaces.txt**: `charset`・descript ブレス（`version`）・surface*ブレス 34 個・`sakura.cursor` ブレス（`mouseup*` 20 行・値は `system:hand`）・element 定義 `element*`（描画メソッドは `overlay` だけ・50 行）・`collision*`（26 行）・`animation*.interval`（`bind` 27・`bind+random,4` 2）・`animation*.pattern*`（描画メソッドは `overlay` だけ・32 行）
- **さくらスクリプトのタグ**（24）: `\0`・`\1`・`\s[ID]`・`\e`・`\-`・`\c`・`\n`・`\n[数値]`・`\p[ID]`・`\w5`／`\w8`・`\_w[時間]`・`\_u[0x3000]`・`\_l[x,y]`・`\f[default]`・`\f[color,…]`・`\q[タイトル,ID,r2]`・`\![*]`・`\![bind,…]`・`\![bind-noevent,…]`・`\![set,zorder,…]`・`\![reset,zorder]`・`\![set,windowstate,!stayontop]`・`\![move,…]`・`\![get,property,…]`。`\v` は辞書の注釈の中にだけ在り、台本には出ない（使わない項目として数えない）。
- **イベントとリソース**（13）: `OnBoot`・`OnFirstBoot`・`OnChoiceSelectEx`・`OnSecondChange`・`OnMouseMove`・`OnMouseEnter`・`OnMouseLeave`・`OnMouseClick`・`OnMouseDoubleClick`・`OnMouseDragEnd`・`OnShellScaling`・`OnBalloonScaling`・リソース `sakura.recommendsites`。areka が送ったが辞書に受け口の無い `OnClose`・`OnBalloonTimeout` は数えない。

台帳に行の無いもの（突き合わせの相手が無いので数えない）: surface*ブレス・descript ブレス・`sakura.cursor` ブレスそのもの（中の鍵は上の行で見る）・ぱすた独自のイベント `OnTalk`・`OnPastaShellScale`・`OnPastaScaleCheck`・`raise_event` で出した `選択肢テスト`・`縦書き試作`。

### 突き合わせの表

「今の値」は突き合わせる前の `status`・`owner`。「台帳の行」は id の頭の `ukadoc:` を省いた。値の同じ行は 1 つの欄にまとめた（台帳の行は 154、表の欄は 101）。引き取る spec の名前は、設計の Out of Boundary・要件の Adjacent expectations・手順 `doc/pasta-real-run.md` 5 節の表の兄弟の spec から取った。

| 項目 | 台帳の行 | 今の値 | 記録の結果 | 食い違い | 直した値 |
|---|---|---|---|---|---|
| ゴースト `charset` | `descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1` | implemented・（空） | UTF-8 で読めた（文字化け 0） | なし | — |
| ゴースト `type` | `descript_ghost:type_2c_7a2e_5225:1` | absent・（空） | 走行と測りに崩れの記録なし（読む経路が無いことと合う） | なし | — |
| ゴースト `id` | `descript_ghost:id_2cID_540d:1` | implemented・areka-P0-baseware-root-layout | 走行と測りに崩れの記録なし | なし | — |
| ゴースト `name` | `descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1` | implemented・areka-P0-baseware-root-layout | 起動の記録 `ghost_resolved` でぱすたを起こした | なし | — |
| ゴースト `sakura.name` | `descript_ghost:sakura.name_2c_540d_524d:1` | implemented・areka-P0-package-mount | 走行と測りに崩れの記録なし | なし | — |
| ゴースト `kero.name` | `descript_ghost:kero.name_2c_540d_524d:1` | implemented・areka-P0-package-mount | 走行と測りに崩れの記録なし | なし | — |
| ゴースト `sakura.seriko.defaultsurface` | `descript_ghost:sakura.seriko.defaultsurface_2c_6570_5024:1` | absent・（空） | SSP は `\0` が面 1000。areka は読む経路が無い（手順 5 節の段 1 の引き取り先 `descript-window-keys`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-descript-window-keys"`・`note` に引き取りの 1 行 |
| ゴースト `balloon` | `descript_ghost:balloon_2c_30d0_30eb_30fc_30f3_540d:1` | implemented・areka-P0-ghost-standard-balloon | areka は `descript_balloon_not_found` を記録して既定のバルーンへ進んだ。行の `note` はこの場合の記録とゴーストの中を探さないこと（`areka-P0-ghost-inner-balloon`）を既に書いている | なし | — |
| ゴースト `balloon.dontmove` | `descript_ghost:balloon.dontmove_2ctrue:1` | absent・（空） | SSP はバルーンがぱすたに付いて動いた。areka は読む経路が無い（引き取り先 `descript-window-keys`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-descript-window-keys"`・`note` に引き取りの 1 行 |
| ゴースト `balloon.syncscale` | `descript_ghost:balloon.syncscale_2ctrue:1` | absent・（空） | 本 spec の範囲の外（倍率の連動）。走行では見ていない | あり（5.3・2026-10-23）（宛先が空。ぱすたが使う機能で、v0.1.0 の中で片づけるために起票した spec が引き取る・6 節） | `owner = "areka-P0-shell-scale-linkage"`・`note` に引き取りの 1 行 |
| ゴースト `shiori` | `descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1` | implemented・areka-P0-package-mount | `pasta.dll` を読み込み、要求に答えた | なし | — |
| ゴースト `craftman` | `descript_ghost:craftman_2c_4f5c_8005_540d:1` | implemented・areka-P0-baseware-root-layout | 走行と測りに崩れの記録なし | なし | — |
| ゴースト `craftmanw` | `descript_ghost:craftmanw_2c_4f5c_8005_540d:1` | implemented・areka-P0-baseware-root-layout | 走行と測りに崩れの記録なし | なし | — |
| ゴースト `craftmanurl` | `descript_ghost:craftmanurl_2cURL:1` | absent・（空） | 走行と測りに崩れの記録なし（読む経路が無いことと合う） | なし | — |
| ゴースト `icon` | `descript_ghost:icon_2c_30d5_30a1_30a4_30eb_540d:1` | absent・（空） | 走行と測りに崩れの記録なし（読む経路が無いことと合う） | なし | — |
| ゴースト `homeurl` | `descript_ghost:homeurl_2cURL:1` | implemented・areka-P0-network-update | ネットワーク更新は回していない。走行と測りに崩れの記録なし | なし | — |
| install.txt `charset`・`type`・`name`・`directory` | `descript_install:charset_2c_6587_5b57_30b3_30fc_30c9:1`<br>`descript_install:type_2c_7a2e_5225:1`<br>`descript_install:name_2c_30aa_30d6_30b8_30a7_30af_30c8_540d:1`<br>`descript_install:directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1` | implemented・areka-P0-nar-install | 書庫の中身のテスト（1.2）と走行の根の組み立てで読めた | なし | — |
| シェル `charset` | `descript_shell:charset_2c_6587_5b57_30b3_30fc_30c9:1` | implemented・（空） | UTF-8 で読めた | なし | — |
| シェル `type` | `descript_shell:type_2c_7a2e_5225:1` | absent・（空） | 走行と測りに崩れの記録なし（読む経路が無いことと合う） | なし | — |
| シェル `name` | `descript_shell:name_2c_30b7_30a7_30eb_540d:1` | implemented・areka-P0-ghost-setup | メニューのシェルの下に「パスタ」が出た | なし | — |
| シェル `craftman`・`craftmanw` | `descript_shell:craftman_2c_4f5c_8005_540d:1`<br>`descript_shell:craftmanw_2c_4f5c_8005_540d:1` | implemented・areka-P0-baseware-root-layout | 走行と測りに崩れの記録なし | なし | — |
| シェル `craftmanurl` | `descript_shell:craftmanurl_2cURL:1` | absent・（空） | 走行と測りに崩れの記録なし（読む経路が無いことと合う） | なし | — |
| シェル `seriko.use_self_alpha` | `descript_shell:seriko.use_self_alpha_2c_5024:1` | degraded・areka-P0-self-alpha-declaration | α の決まりは 7 節。行の `note` は 7.2 で当たりの決まりを書いた | なし | — |
| シェル `seriko.alignmenttodesktop` | `descript_shell:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1` | absent・（空） | areka のぱすたの窓は画面の下端を越えた（段 1）。下端を越えたのは面 0 で大きさを測ったため（5 節の 4）。areka はこの鍵を読む（`crates/areka/src/placement/config.rs:207`〜`221`・ぱすたの値 `free` は `Alignment::Free`）。台帳の absent は、ぱすたが使わない別の意味（`top`・`left`・`right` など）の分で、記録と食い違わない（5.3 で理由を直した） | なし | — |
| シェル `sakura.defaultx` | `descript_shell:sakura.defaultx_2cX_5ea7_6a19:1` | absent・（空） | 走行と測りに崩れの記録なし。areka はこの鍵を読む（`config.rs:223`〜`227`・`defaultx` は `defaultleft` より強い）。ぱすたの値 `0` は areka が扱う範囲。台帳の absent は、ぱすたが使わない別の意味（`defaultleft` との区別など）の分で、記録と食い違わない（5.3 で理由を直した） | なし | — |
| シェル `sakura.balloon.alignment` | `descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1` | implemented・（空） | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| シェル `sakura.balloon.offsetx`・`offsety` | `descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1`<br>`descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1` | absent・（空） | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る。案 A の rerun1 は 6 節）。areka はこの鍵を、x と y がそろったときに読む（`config.rs:252`〜`262`）。ぱすたは両方を書く（`476`・`58`）。台帳の absent は、ぱすたが使わない片方だけの指定などの分で、記録と食い違わない（5.3 で理由を直した） | なし | — |
| シェル `seriko.sticky-window` | `descript_shell:seriko.sticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1` | vocabulary-only・（空） | SSP は 2 つの窓が揃って動いた。areka は窓の枠（`\1`）が別の画面に出て、ドラッグでも付いてこなかった（段 1・4・5。引き取り先 `window-link-directives`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-window-link-directives"`・`note` に引き取りの 1 行 |
| シェル `menu.background.*`（絵・位置・文字の色） | `descript_shell:menu.background.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1`<br>`descript_shell:menu.background.alignment_2c_4f4d_7f6e:1`<br>`descript_shell:menu.background.font.color.r_2c_6570_5024:1`<br>`descript_shell:menu.background.font.color.g_2c_6570_5024:1`<br>`descript_shell:menu.background.font.color.b_2c_6570_5024:1` | absent・（空） | areka は普通の Windows のメニューで、シェルの絵と色を使わない（段 6。引き取り先 `popup-menu-shell-look`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-popup-menu-shell-look"`・`note` に引き取りの 1 行 |
| シェル `menu.foreground.*`（絵・位置・文字の色） | `descript_shell:menu.foreground.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1`<br>`descript_shell:menu.foreground.alignment_2c_4f4d_7f6e:1`<br>`descript_shell:menu.foreground.font.color.r_2c_6570_5024:1`<br>`descript_shell:menu.foreground.font.color.g_2c_6570_5024:1`<br>`descript_shell:menu.foreground.font.color.b_2c_6570_5024:1` | absent・（空） | 同上（段 6） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-popup-menu-shell-look"`・`note` に引き取りの 1 行 |
| シェル `menu.sidebar.*`（絵・位置） | `descript_shell:menu.sidebar.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1`<br>`descript_shell:menu.sidebar.alignment_2c_4f4d_7f6e:1` | absent・（空） | 同上（段 6） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-popup-menu-shell-look"`・`note` に引き取りの 1 行 |
| シェル `menu.disable.font.color.*` | `descript_shell:menu.disable.font.color.r:1`<br>`descript_shell:menu.disable.font.color.g:1`<br>`descript_shell:menu.disable.font.color.b:1` | absent・（空） | 同上（段 6） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-popup-menu-shell-look"`・`note` に引き取りの 1 行 |
| シェル `menu.separator.color.*` | `descript_shell:menu.separator.color.r_2c_6570_5024:1`<br>`descript_shell:menu.separator.color.g_2c_6570_5024:1`<br>`descript_shell:menu.separator.color.b_2c_6570_5024:1` | absent・（空） | 同上（段 6） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-popup-menu-shell-look"`・`note` に引き取りの 1 行 |
| シェル `menu.frame.color.*` | `descript_shell:menu.frame.color.r_2c_6570_5024:1`<br>`descript_shell:menu.frame.color.g_2c_6570_5024:1`<br>`descript_shell:menu.frame.color.b_2c_6570_5024:1` | absent・（空） | 同上（段 6） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-popup-menu-shell-look"`・`note` に引き取りの 1 行 |
| シェル `sakura.bindgroup*.name` | `descript_shell:sakura.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1` | implemented・areka-P0-mayuna-compose | 台本の `\![bind]` で部位が着せ替わった。着せ替えのメニューが無いのは `sakura.menu` の行 | なし | — |
| シェル `sakura.bindgroup*.default` | `descript_shell:sakura.bindgroup_2a.default_2c_6570_5024:1` | implemented・areka-P0-mayuna-compose | 走行と測りに崩れの記録なし | なし | — |
| シェル `sakura.bindoption*.group` | `descript_shell:sakura.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1` | implemented・areka-P0-bindoption-exclusivity | `mustselect` の部位を外す指示を正典どおり無視した（WARN 96 行。崩れではない） | なし | — |
| シェル `sakura.menu` | `descript_shell:sakura.menu_2cauto_307e_305f_306fhidden:1` | absent・（空） | SSP は着せ替えのメニューが出た。areka はメニューに「着せ替え」が無い（段 6。引き取り先 `popup-menu-shell-look`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-popup-menu-shell-look"`・`note` に引き取りの 1 行 |
| バルーン `charset` | `descript_balloon:charset_2c_6587_5b57_30b3_30fc_30c9:1` | implemented・（空） | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `type`・`name`・`craftman`・`craftmanw` | `descript_balloon:type_2c_7a2e_5225:1`<br>`descript_balloon:name_2c_30d0_30eb_30fc_30f3_540d:1`<br>`descript_balloon:craftman_2c_4f5c_8005_540d:1`<br>`descript_balloon:craftmanw_2c_4f5c_8005_540d:1` | implemented・areka-P0-baseware-root-layout | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `use_self_alpha` | `descript_balloon:use_self_alpha_2c_5024:1` | degraded・areka-P0-self-alpha-declaration | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `vertical` | `descript_balloon:vertical_2c0_2f1:1` | degraded・areka-P0-balloon-vertical-canon | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `origin.x` | `descript_balloon:origin.x_2c_5ea7_6a19_20_2a1:1` | implemented・areka-P0-balloon-vertical-canon | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `origin.y` | `descript_balloon:origin.y_2c_5ea7_6a19_20_2a1:1` | implemented・areka-P0-balloon-parse | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `validrect.*` | `descript_balloon:validrect.left_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:validrect.top_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:validrect.right_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:validrect.bottom_2c_5ea7_6a19_20_2a1:1` | implemented・areka-P0-balloon-parse | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `wordwrappoint.y` | `descript_balloon:wordwrappoint.y_2c_5ea7_6a19_20_2a1:1` | implemented・areka-P0-balloon-vertical-canon | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `font.name` | `descript_balloon:font.name_2c_30d5_30a9_30f3_30c8_540d:1` | degraded・areka-P0-balloon-font-descript-keys | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `font.height`・`font.color.*` | `descript_balloon:font.height_2c_6570_5024:1`<br>`descript_balloon:font.color.r_2c_6570_5024:1`<br>`descript_balloon:font.color.g_2c_6570_5024:1`<br>`descript_balloon:font.color.b_2c_6570_5024:1` | implemented・areka-P0-text-decoration-canon | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `font.shadowcolor.*` | `descript_balloon:font.shadowcolor.r_2c_6570_5024:1`<br>`descript_balloon:font.shadowcolor.g_2c_6570_5024:1`<br>`descript_balloon:font.shadowcolor.b_2c_6570_5024:1` | vocabulary-only・areka-P0-text-align-shadow-canon | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `anchor.font.color.*` | `descript_balloon:anchor.font.color.r_2c_6570_5024:1`<br>`descript_balloon:anchor.font.color.g_2c_6570_5024:1`<br>`descript_balloon:anchor.font.color.b_2c_6570_5024:1` | absent・areka-P0-anchor-style-canon | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `cursor.style` | `descript_balloon:cursor.style_2c_5f62_72b6:1` | absent・areka-P0-choice-marker-styling | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `cursor.blendmethod`・`cursor.pen.color.*` | `descript_balloon:cursor.blendmethod_2c_30b3_30de_30f3_30c9:1`<br>`descript_balloon:cursor.pen.color.r_2c_6570_5024:1`<br>`descript_balloon:cursor.pen.color.g_2c_6570_5024:1`<br>`descript_balloon:cursor.pen.color.b_2c_6570_5024:1` | vocabulary-only・areka-P0-choice-marker-styling | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `cursor.brush.color.*`・`cursor.font.color.*` | `descript_balloon:cursor.brush.color.r_2c_6570_5024:1`<br>`descript_balloon:cursor.brush.color.g_2c_6570_5024:1`<br>`descript_balloon:cursor.brush.color.b_2c_6570_5024:1`<br>`descript_balloon:cursor.font.color.r_2c_6570_5024:1`<br>`descript_balloon:cursor.font.color.g_2c_6570_5024:1`<br>`descript_balloon:cursor.font.color.b_2c_6570_5024:1` | implemented・areka-P0-choice-marker-styling | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `cursor.notselect.style`・`cursor.notselect.font.color.*` | `descript_balloon:cursor.notselect.style_2c_5f62_72b6:1`<br>`descript_balloon:cursor.notselect.font.color.r_2c_6570_5024:1`<br>`descript_balloon:cursor.notselect.font.color.g_2c_6570_5024:1`<br>`descript_balloon:cursor.notselect.font.color.b_2c_6570_5024:1` | absent・areka-P0-choice-marker-styling | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| バルーン `arrow0.*`・`arrow1.*` | `descript_balloon:arrow0.x_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:arrow0.y_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:arrow1.x_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:arrow1.y_2c_5ea7_6a19_20_2a1:1` | absent・areka-P0-balloon-canon-residue | 本 spec の範囲の外（スクロールの矢印） | あり（5.3・2026-10-23）（宛先が v0.1.0 の外。ぱすたが使う機能で、起票した spec が引き取る・6 節） | `owner = "areka-P0-balloon-scroll-arrows"`・`note` に引き取りの 1 行（前の担当の文は「前の担当 spec は」に替えた） |
| バルーン `communicatebox.*` | `descript_balloon:communicatebox.x_2c_5ea7_6a19:1`<br>`descript_balloon:communicatebox.y_2c_5ea7_6a19:1`<br>`descript_balloon:communicatebox.width_2c_30b5_30a4_30ba:1`<br>`descript_balloon:communicatebox.height_2c_30b5_30a4_30ba:1` | absent・（空） | 本 spec の範囲の外（`communicatebox`） | あり（5.3・2026-10-23）（宛先が空。ぱすたが使う機能で、起票した spec が引き取る・6 節） | `owner = "areka-P0-balloon-communicatebox"`・`note` の「担当は決まっていない。」を引き取りの 1 行に替えた |
| surfaces `charset` | `descript_shell_surfaces:charset_2c_6587_5b57_30b3_30fc_30c9:1` | implemented・areka-P0-charset-canon | UTF-8 で読めた | なし | — |
| surfaces の descript ブレスの `version` | `descript_shell_surfaces:version_2c_2a:1` | absent・（空） | 走行と測りに崩れの記録なし（読む経路が無いことと合う） | なし | — |
| element 定義 `element*` | `descript_shell_surfaces:element_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30d5_30a1_30a4_30eb_540d_2cX_5ea7_6a19_2cY_5ea7_6a19_28_2c_30aa_30d7_30b7:1` | degraded・areka-P0-draw-methods-canon | 面が組み上がった。`pasta/null.png`（全透明）が切り詰めで 0 寸になる WARN 2 行（崩れではない） | なし | — |
| element 定義の描画メソッド `overlay`（element・pattern） | `descript_shell_surfaces:overlay:1` | implemented・（空） | 走行と測りに崩れの記録なし | なし | — |
| `collision*` | `descript_shell_surfaces:collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1` | degraded・areka-P0-shell-parse | ボールと × の当たりは取れた（`hit_test` の記録） | なし | — |
| `animation*.interval` の値 `bind` | `descript_shell_surfaces:bind:1` | vocabulary-only・（空） | 着せ替えは静的に重なった。× の赤（animation1500）は `OnMouseEnter` が無いので出ない（`mouse-click-wheel-events` の件） | なし | — |
| `animation*.interval` の値 `random`（`bind+random,4`） | `descript_shell_surfaces:random_2c_6570_5024:1` | implemented・（空） | 走行と測りに崩れの記録なし | なし | — |
| `animation*.interval` | `descript_shell_surfaces:animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1` | degraded・areka-P0-shell-parse | 走行と測りに崩れの記録なし | なし | — |
| `animation*.pattern*` | `descript_shell_surfaces:animation_2a.pattern_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30b5_30fc_30d5_30a7_30b9_756a_53f7_2c_30a6_30a7_30a4_30c8_2c:1` | degraded・areka-P0-shell-parse | 走行と測りに崩れの記録なし | なし | — |
| `sakura.cursor` ブレスの `mouseup*` | `descript_shell_surfaces:mouseup_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1` | absent・（空） | SSP は身体で `system:hand` の形が出た。areka はマウスの形を読んでいない（段 3。引き取り先 `shell-mouse-cursor`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-shell-mouse-cursor"`・`note` に引き取りの 1 行 |
| カーソルの形 `system:hand` | `descript_shell_surfaces:system_3ahand:1` | absent・（空） | 同上（段 3） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-shell-mouse-cursor"`・`note` に引き取りの 1 行 |
| `\0` | `list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1` | implemented・areka-P0-kero-balloon | 走行と測りに崩れの記録なし | なし | — |
| `\1` | `list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1` | implemented・areka-P0-kero-balloon | `\1` は面 10 になった。窓の枠が別の画面に出たのは `seriko.sticky-window` の行 | なし | — |
| `\s[ID]` | `list_sakura_script:_5cs_5bID_756a_53f7_5d:1` | implemented・（空） | 台本どおりの面になった（最後の `\s[1002]` で身体の無い面） | なし | — |
| `\e` | `list_sakura_script:_5ce:1` | implemented・（空） | 走行と測りに崩れの記録なし | なし | — |
| `\-` | `list_sakura_script:_5c-:1` | implemented・（空） | areka では × の `OnMouseClick` が無く、終わりの台本は出なかった（`mouse-click-wheel-events` の件） | なし | — |
| `\c` | `list_sakura_script:_5cc:1` | implemented・areka-P0-cursor-tag-canon | `縦書き試作` の 2 ページ目が消して書き直された | なし | — |
| `\n` | `list_sakura_script:_5cn:1` | implemented・（空） | 走行と測りに崩れの記録なし | なし | — |
| `\n[数値]` | `list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1` | implemented・（空） | 走行と測りに崩れの記録なし | なし | — |
| `\p[ID]` | `list_sakura_script:_5cp_5bID_756a_53f7_5d:1` | degraded・（空） | 走行と測りに崩れの記録なし | なし | — |
| `\w5`・`\w8` | `list_sakura_script:_5cw_6642_9593:1` | implemented・（空） | 走行と測りに崩れの記録なし | なし | — |
| `\_w[時間]` | `list_sakura_script:_5c_w_5b_6642_9593_5d:1` | implemented・（空） | 走行と測りに崩れの記録なし | なし | — |
| `\_u[0x3000]` | `list_sakura_script:_5c_u_5b0x0000_5d:1` | absent・（空） | 台本に在る。areka は文字にしない（引き取り先 `script-small-tags`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-script-small-tags"`・`note` に引き取りの 1 行 |
| `\_l[x,y]` | `list_sakura_script:_5c_l_5bx_2cy_5d:1` | implemented・areka-P0-cursor-tag-canon | 横書きの既定のバルーンで、`\_l[-283,0]` の後の段が前の行に重なった。縦書きのバルーンで出なかったための重なりで、`\_l` の読みの崩れとは記録から言えない（縦書きのバルーンは `ghost-inner-balloon` の件） | なし | — |
| `\f[default]`・`\f[color,…]` | `list_sakura_script:_5cf_5bdefault_5d:1`<br>`list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1` | implemented・areka-P0-text-decoration-canon | 走行と測りに崩れの記録なし | なし | — |
| `\q[タイトル,ID,r2]` | `list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1` | implemented・（空） | 選択肢が出て、押すと `OnChoiceSelectEx` が届き続きが出た（段 2） | なし | — |
| `\![*]` | `list_sakura_script:_5c_21_5b_2a_5d:1` | absent・（空） | SSP は選択肢の前に印が出た。areka は印が見えない（段 2。引き取り先 `script-small-tags`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-script-small-tags"`・`note` に引き取りの 1 行 |
| `\![bind,…]` | `list_sakura_script:_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1` | implemented・（空） | 部位が着せ替わった。`mustselect` の部位を外す指示は正典どおり無視（WARN 96 行） | なし | — |
| `\![bind-noevent,…]` | `list_sakura_script:_5c_21_5bbind-noevent_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1` | absent・（空） | 台本に在る。areka は受け手が無い（引き取り先 `script-small-tags`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-script-small-tags"`・`note` に引き取りの 1 行 |
| `\![set,zorder,…]` | `list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1` | implemented・areka-P0-scope-zorder-pinning | 窓の枠が別の画面に出たので重なりの順は主画面で見えない（`seriko.sticky-window` の行） | なし | — |
| `\![reset,zorder]` | `list_sakura_script:_5c_21_5breset_2czorder_5d:1` | implemented・areka-P0-scope-zorder-pinning | 同上 | なし | — |
| `\![set,windowstate,!stayontop]` | `list_sakura_script:_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1` | absent・（空） | 台本に在る。areka は受け手が無い（引き取り先 `stay-on-top-directives`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-stay-on-top-directives"`・`note` に引き取りの 1 行 |
| `\![move,…]` | `list_sakura_script:_5c_21_5bmove_5d:1` | degraded・areka-P0-move-canon | `--option=ignore-sticky-window` 付きで `\0` を動かした。窓の枠が付いてこないのは `seriko.sticky-window` の行 | なし | — |
| `\![get,property,…]` | `list_sakura_script:_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1` | absent・areka-P0-property-query-channels | 走行と測りに崩れの記録なし（受け手が無いことと合う。本 spec の範囲の外の倍率の連動に使う） | あり（5.3・2026-10-23）（宛先が v0.1.0 の外。ぱすたが使う機能で、起票した spec が引き取る・6 節） | `owner = "areka-P0-shell-scale-linkage"`・`note` に引き取りの 1 行 |
| `OnBoot` | `list_shiori_event:OnBoot:1` | implemented・（空） | SSP は記憶のある起動で `OnBoot`。areka は空の記憶で `OnFirstBoot` を先に送り、`OnBoot` は飛ばした（正典どおり） | なし | — |
| `OnFirstBoot` | `list_shiori_event:OnFirstBoot:1` | implemented・（空） | 届いて台本が返った（段 1） | なし | — |
| `OnChoiceSelectEx` | `list_shiori_event:OnChoiceSelectEx:1` | implemented・（空） | 届いて続きの台本が返った（段 2） | なし | — |
| `OnSecondChange` | `list_shiori_event:OnSecondChange:1` | implemented・（空） | SSP は毎秒届いた。areka も毎秒送っている。ぱすたの記録に行が無いのは、ぱすたが本文を返した応答だけを記録し、run1 の間は「初回待ち」で応答を止めていたため。崩れではない（5 節の「崩れではないと分かったこと」） | なし | — |
| `OnMouseMove` | `list_shiori_event:OnMouseMove:1` | implemented・（空） | 面 1002 に身体が無く、撫でる入力は窓を素通りした（段 3）。送出の崩れとは記録から言えない | なし | — |
| `OnMouseEnter`・`OnMouseLeave` | `list_shiori_event:OnMouseEnter:1`<br>`list_shiori_event:OnMouseLeave:1` | absent・（空） | SSP は × で届いて赤くなった。areka は送らない（段 3・7。引き取り先 `mouse-click-wheel-events`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-mouse-click-wheel-events"`・`note` に引き取りの 1 行 |
| `OnMouseClick` | `list_shiori_event:OnMouseClick:1` | absent・（空） | SSP はボールと × で届いた。areka は窓に当たったが送らない（段 3・7。引き取り先 `mouse-click-wheel-events`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-mouse-click-wheel-events"`・`note` に引き取りの 1 行 |
| `OnMouseDoubleClick` | `list_shiori_event:OnMouseDoubleClick:1` | implemented・（空） | 走行で回していない。走行と測りに崩れの記録なし | なし | — |
| `OnMouseDragEnd` | `list_shiori_event:OnMouseDragEnd:1` | implemented・areka-P0-mouse-drag-events | 走行で回していない（ドラッグは記録 `DragEnd 保存` まで）。走行と測りに崩れの記録なし | なし | — |
| `OnShellScaling`・`OnBalloonScaling` | `list_shiori_event:OnShellScaling:1`<br>`list_shiori_event:OnBalloonScaling:1` | absent・（空） | 本 spec の範囲の外（倍率の連動）。SSP も倍率が変わらず送らなかった | あり（5.3・2026-10-23）（宛先が空。ぱすたが使う機能で、起票した spec が引き取る・6 節） | `owner = "areka-P0-shell-scale-linkage"`・`note` に引き取りの 1 行 |
| リソース `sakura.recommendsites` | `list_shiori_resource:sakura.recommendsites:1` | vocabulary-only・（空） | SSP はメニューを組むときに求めた。areka は求めない（語彙のみと合う） | なし | — |
| （5.3 で足した欄）プロパティ `currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi`（ぱすたが `\![get,property,…]` で聞く値。7.1 の拾いには入れていなかった） | `list_propertysystem:currentghost.scope_28ID_29.scaling:1`<br>`list_propertysystem:currentghost.balloon.scope_28ID_29.scaling:1`<br>`list_propertysystem:currentghost.scope_28ID_29.currentmonitor.dpi:1` | vocabulary-only・areka-P0-currentghost-property-tree | 本 spec の範囲の外（倍率の連動）。SSP は段 5 で `scaling` 150,150・`currentmonitor.dpi` 192 を答えた | あり（5.3・2026-10-23）（宛先が v0.1.0 の外。ぱすたが使う機能で、起票した spec が引き取る・6 節） | `owner = "areka-P0-shell-scale-linkage"`・`note` に引き取りの 1 行 |
| （5.3 で足した欄）プロパティ `currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect`（ぱすたは使わない。走行の測りが MCP の `get_property` で読もうとした値） | `list_propertysystem:currentghost.scope_28ID_29.rect:1`<br>`list_propertysystem:currentghost.balloon.scope_28ID_29.rect:1` | vocabulary-only・areka-P0-currentghost-property-tree | areka は `NG:Cannot find such property name.` で答えた（5 節の崩れ 20）。SSP は答えた | あり（5.3・2026-10-23）（宛先が v0.1.0 の外。崩れ 20 を起票した spec が引き取る・6 節） | `owner = "areka-P0-mcp-scope-rect-property"`・`note` に引き取りの 1 行 |

### 直したことと残り

- 直した台帳の行は **34 行**（`assets.toml` 27・`sakura-script.toml` 4・`shiori.toml` 3）。どれも宛先（`owner`）が空で、崩れが兄弟の spec の持ち分だった行で、`owner` に引き取る spec の名前を書き、`note` の「担当は決まっていない。」をその名前の 1 行に替えた（その句が無い行は `note` の終わりに同じ 1 行を足した）。`status` は 1 行も変えていない（どの行も、今の値が記録と合っていた）。
  - `areka-P0-descript-window-keys` 2・`areka-P0-window-link-directives` 1・`areka-P0-popup-menu-shell-look` 22・`areka-P0-shell-mouse-cursor` 2・`areka-P0-script-small-tags` 3・`areka-P0-stay-on-top-directives` 1・`areka-P0-mouse-click-wheel-events` 3。
- 台帳の宛先は、`doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` に名前が在ることを検査が求める（`crates/ukadoc-survey/tests/consistency/spec_checks.rs` の腕 c と腕 f）。7 本はどれも表に無かったので、`none = true`（束はまだ割り当てない）と理由と件数の行を 7 行足し、`[briefs].count` を 52 から 59 へ、`snapshot_on` を 2026-10-23 へ直した（同じ文書の「2026-10-23 の追加」の段落）。あわせて、34 項目が `linkage.md` の束の `members` で属する 8 つの束（「メニュー」22・「窓の配置と重なり」3・「撫で」3・「マウスの矢印」2・「descript の転記」「選択肢の目印」「切替」「バルーンの文字」1 件ずつ）の段階の表の行と、節を持つ 2 つの束の節の「依存する既存 spec」に、7 本を件数付きで足した。候補 spec 名の案はどの束でも ⑴（全数）にも ⑵（過半）にも当たらないので変えていない。
- 引き取る spec が台帳に既に書かれていた行（ゴーストの `balloon` の `note` の `areka-P0-ghost-inner-balloon`）と、本 spec の範囲の外の行（`balloon.syncscale`・`OnShellScaling`・`OnBalloonScaling`・スクロールの矢印・`communicatebox`）は変えていない。ぱすたが使わない行は 1 行も変えていない（要件 9.4。差分は上の 34 行だけ）。
- タスク 5.3（2026-10-23）で、v0.1.0 の中で片づけるために起票した spec（6 節）へ **17 行**の宛先を移した（`assets.toml` 9・`property.toml` 5・`shiori.toml` 2・`sakura-script.toml` 1）。`status` は変えていない。上の段落が「変えていない」と書いた範囲の外の行のうち、ぱすたが使う行（ゴーストの `balloon.syncscale`・`OnShellScaling`・`OnBalloonScaling`・`arrow0.x/y`・`arrow1.x/y`・`communicatebox.x/y/width/height`）は、ここで宛先を書いた。
  - `areka-P0-shell-scale-linkage` 7（ゴーストの `balloon.syncscale`・`\![get,property,…]`・`OnShellScaling`・`OnBalloonScaling`・`currentghost.scope(ID).scaling`・`currentghost.balloon.scope(ID).scaling`・`currentghost.scope(ID).currentmonitor.dpi`）・`areka-P0-mcp-scope-rect-property` 2（`currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect`）・`areka-P0-balloon-scroll-arrows` 4（`arrow0.x/y`・`arrow1.x/y`）・`areka-P0-balloon-communicatebox` 4（`communicatebox.x/y/width/height`）。
  - 前の宛先から移したのは 10 行（`areka-P0-currentghost-property-tree` 5・`areka-P0-balloon-canon-residue` 4・`areka-P0-property-query-channels` 1）、宛先が空だったのは 7 行。`note` の書き方は 7.1 と同じで、前の宛先を持つ行には「（前の宛先は …）」を添え、前の担当を書いた文は「前の担当 spec は」に替えた。
  - ぱすたが使わない行は変えていない。例外は `currentghost.scope(ID).rect`・`currentghost.balloon.scope(ID).rect` の 2 行で、ぱすたの動きではなく走行の測りが読めなかった崩れ 20 の項目なので、引き取る spec へ移した。シェルの descript の `sakura.`・`kero.`・`char*.balloon.syncscale` と、`communicatebox.*` の残りの 7 行・`\![open,communicatebox]`・`\![close,communicatebox]`・`OnCommunicate`・`OnCommunicateInputCancel`・`arrow.filename` は、ぱすたが使わないので変えていない（引き取る spec が着手したときに自分で宛先を書く）。
  - 4 本は `doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` に無かったので、`none = true` と理由と件数の行を 4 行足し、宛先が減った 3 本の `owner_count` を直し（`areka-P0-currentghost-property-tree` 64→59・`areka-P0-balloon-canon-residue` 21→17・`areka-P0-property-query-channels` 6→5）、`[briefs].count` を 59 から 63 へ直した。段階の表の 5 行（「窓の配置と重なり」・「バルーンの付属画像」・「切替」・「環境の照会」・「コミュニケート」）と「窓の配置と重なり」の節の「依存する既存 spec」を数え直し、束の決め方の規則で 2 本の束を引き直した（`areka-P0-currentghost-property-tree` は「マウスの矢印」へ、`areka-P0-balloon-canon-residue` は「SSTP」・段階 D へ）。同じ文書の「2026-10-23 の 2 回目の追加」の段落に書いた。`areka-P0-animated-surface-log-flood` は台帳に宛先を持たない。
- `OnSecondChange` は崩れではない（5 節の「崩れではないと分かったこと」）。台帳の「実装済み」は記録と食い違わないので、直していない。
- **食い違いの残りは 0 件**（ぱすたが使う項目の行 154 のうち、記録と食い違う行は 0）。

### 確かめ（2026-10-23）

| 命令 | 結果 |
|---|---|
| `cargo test -p ukadoc-survey`（宛先だけを書き、`roadmap-draft.md` の行を足す前） | 赤。`spec_checks::the_spec_table_agrees_with_the_directories_the_ledgers_and_the_bundles` ほか 1 件が FAILED で、「宛先 areka-P0-descript-window-keys が … roadmap-draft.md の [[spec]] にも … [[owner_completed]] にも無い（要件 7.4 ⑶）」などを 7 本の名前で出した |
| `cargo test -p ukadoc-survey`（7 行を足した後） | 緑（601・6・113・5 件 ok・失敗 0） |
| `cargo run -p ukadoc-survey -- check` | 終了コード 0・「食い違い 0 件」 |
| `cargo run -p ukadoc-survey -- report` と `report-summary` | 作り直した。`doc/ukadoc-coverage/report/` の中身の差分は 0（報告は `owner` を載せない） |
| 5.3: 作業の前の `cargo test -p ukadoc-survey` と `check` | 緑（601・6・113・5 件 ok・失敗 0）・終了コード 0・「食い違い 0 件」 |
| 5.3: 17 行の宛先だけを移し、`roadmap-draft.md` を直す前の `cargo test -p ukadoc-survey` | 赤。`spec_checks::the_spec_table_agrees_with_the_directories_the_ledgers_and_the_bundles` と `spec_checks::rewriting_one_owner_turns_the_count_and_destination_arms_red` が FAILED で、「[[spec]] areka-P0-currentghost-property-tree の owner_count は 64 と書いてあるが、台帳 4 本の数え直しは 59」（`areka-P0-balloon-canon-residue` 21 と 17・`areka-P0-property-query-channels` 6 と 5 も同じ形）と、「宛先 areka-P0-balloon-communicatebox が … roadmap-draft.md の [[spec]] にも … [[owner_completed]] にも無い（要件 7.4 ⑶）」などを 4 本の名前で出した |
| 5.3: `roadmap-draft.md` を直した後の `cargo test -p ukadoc-survey` | 緑（601・6・113・5 件 ok・失敗 0） |
| 5.3: `cargo run -p ukadoc-survey -- check` | 終了コード 0・「食い違い 0 件」 |
| 5.3: `cargo run -p ukadoc-survey -- report` と `report-summary` | 作り直した。`doc/ukadoc-coverage/report/` の中身の差分は 0 |
