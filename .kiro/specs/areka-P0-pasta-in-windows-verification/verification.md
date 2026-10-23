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

## 10. 網羅台帳との突き合わせ

ぱすたが使う項目を、取り寄せた書庫を展開した木（`target\pasta-verify\root\ghost\pasta-in-windows`。1 節の `.sha256` の書庫と同じ中身）から拾い、網羅台帳 `doc/ukadoc-coverage/ledger/*.toml` の行と突き合わせた（2026-10-23・タスク 7.1・要件 9.1・9.2・9.4）。突き合わせた相手は、測りの記録 `ssp-measurement.md`・最初の走行の控え `target\pasta-verify\shots\run1\notes.md` と記録 `target\pasta-verify\logs\run1.*`・この記録の 1〜3 節と 7 節である。崩れた所の表（5 節）はまだ無いので、原因が記録に書かれていない崩れは 5.1 の仕分けへ渡し、台帳は変えていない。

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
| ゴースト `balloon.syncscale` | `descript_ghost:balloon.syncscale_2ctrue:1` | absent・（空） | 本 spec の範囲の外（倍率の連動）。走行では見ていない | なし | — |
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
| シェル `seriko.alignmenttodesktop` | `descript_shell:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1` | absent・（空） | areka のぱすたの窓は画面の下端を越えた（段 1）。読む経路が無いことと合う。崩れの仕分けは 5.1 | なし | — |
| シェル `sakura.defaultx` | `descript_shell:sakura.defaultx_2cX_5ea7_6a19:1` | absent・（空） | 走行と測りに崩れの記録なし（読む経路が無いことと合う） | なし | — |
| シェル `sakura.balloon.alignment` | `descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1` | implemented・（空） | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
| シェル `sakura.balloon.offsetx`・`offsety` | `descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1`<br>`descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1` | absent・（空） | 縦書きのバルーンが見つからず既定のバルーンで出たので、走行では見えない（`ghost-inner-balloon` の着地の後の回し直しで見る） | なし | — |
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
| バルーン `arrow0.*`・`arrow1.*` | `descript_balloon:arrow0.x_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:arrow0.y_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:arrow1.x_2c_5ea7_6a19_20_2a1:1`<br>`descript_balloon:arrow1.y_2c_5ea7_6a19_20_2a1:1` | absent・areka-P0-balloon-canon-residue | 本 spec の範囲の外（スクロールの矢印） | なし | — |
| バルーン `communicatebox.*` | `descript_balloon:communicatebox.x_2c_5ea7_6a19:1`<br>`descript_balloon:communicatebox.y_2c_5ea7_6a19:1`<br>`descript_balloon:communicatebox.width_2c_30b5_30a4_30ba:1`<br>`descript_balloon:communicatebox.height_2c_30b5_30a4_30ba:1` | absent・（空） | 本 spec の範囲の外（`communicatebox`） | なし | — |
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
| `\![get,property,…]` | `list_sakura_script:_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1` | absent・areka-P0-property-query-channels | 走行と測りに崩れの記録なし（受け手が無いことと合う。本 spec の範囲の外の倍率の連動に使う） | なし | — |
| `OnBoot` | `list_shiori_event:OnBoot:1` | implemented・（空） | SSP は記憶のある起動で `OnBoot`。areka は空の記憶で `OnFirstBoot` を先に送り、`OnBoot` は飛ばした（正典どおり） | なし | — |
| `OnFirstBoot` | `list_shiori_event:OnFirstBoot:1` | implemented・（空） | 届いて台本が返った（段 1） | なし | — |
| `OnChoiceSelectEx` | `list_shiori_event:OnChoiceSelectEx:1` | implemented・（空） | 届いて続きの台本が返った（段 2） | なし | — |
| `OnSecondChange` | `list_shiori_event:OnSecondChange:1` | implemented・（空） | SSP は毎秒届いた。areka の走行ではぱすたの記録に要求の行が 0 で、自然なトークも出なかった。areka の記録の水準では送出の行（trace）が見えず、届かなかったのか送ったのかは記録から決められない。崩れの仕分けは 5.1 で行い、原因が分かったら台帳を直す | なし | — |
| `OnMouseMove` | `list_shiori_event:OnMouseMove:1` | implemented・（空） | 面 1002 に身体が無く、撫でる入力は窓を素通りした（段 3）。送出の崩れとは記録から言えない | なし | — |
| `OnMouseEnter`・`OnMouseLeave` | `list_shiori_event:OnMouseEnter:1`<br>`list_shiori_event:OnMouseLeave:1` | absent・（空） | SSP は × で届いて赤くなった。areka は送らない（段 3・7。引き取り先 `mouse-click-wheel-events`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-mouse-click-wheel-events"`・`note` に引き取りの 1 行 |
| `OnMouseClick` | `list_shiori_event:OnMouseClick:1` | absent・（空） | SSP はボールと × で届いた。areka は窓に当たったが送らない（段 3・7。引き取り先 `mouse-click-wheel-events`） | あり（宛先が空。崩れは兄弟の持ち分） | `owner = "areka-P0-mouse-click-wheel-events"`・`note` に引き取りの 1 行 |
| `OnMouseDoubleClick` | `list_shiori_event:OnMouseDoubleClick:1` | implemented・（空） | 走行で回していない。走行と測りに崩れの記録なし | なし | — |
| `OnMouseDragEnd` | `list_shiori_event:OnMouseDragEnd:1` | implemented・areka-P0-mouse-drag-events | 走行で回していない（ドラッグは記録 `DragEnd 保存` まで）。走行と測りに崩れの記録なし | なし | — |
| `OnShellScaling`・`OnBalloonScaling` | `list_shiori_event:OnShellScaling:1`<br>`list_shiori_event:OnBalloonScaling:1` | absent・（空） | 本 spec の範囲の外（倍率の連動）。SSP も倍率が変わらず送らなかった | なし | — |
| リソース `sakura.recommendsites` | `list_shiori_resource:sakura.recommendsites:1` | vocabulary-only・（空） | SSP はメニューを組むときに求めた。areka は求めない（語彙のみと合う） | なし | — |

### 直したことと残り

- 直した台帳の行は **34 行**（`assets.toml` 27・`sakura-script.toml` 4・`shiori.toml` 3）。どれも宛先（`owner`）が空で、崩れが兄弟の spec の持ち分だった行で、`owner` に引き取る spec の名前を書き、`note` の「担当は決まっていない。」をその名前の 1 行に替えた（その句が無い行は `note` の終わりに同じ 1 行を足した）。`status` は 1 行も変えていない（どの行も、今の値が記録と合っていた）。
  - `areka-P0-descript-window-keys` 2・`areka-P0-window-link-directives` 1・`areka-P0-popup-menu-shell-look` 22・`areka-P0-shell-mouse-cursor` 2・`areka-P0-script-small-tags` 3・`areka-P0-stay-on-top-directives` 1・`areka-P0-mouse-click-wheel-events` 3。
- 台帳の宛先は、`doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` に名前が在ることを検査が求める（`crates/ukadoc-survey/tests/consistency/spec_checks.rs` の腕 c と腕 f）。7 本はどれも表に無かったので、`none = true`（束はまだ割り当てない）と理由と件数の行を 7 行足し、`[briefs].count` を 52 から 59 へ、`snapshot_on` を 2026-10-23 へ直した（同じ文書の「2026-10-23 の追加」の段落）。あわせて、34 項目が `linkage.md` の束の `members` で属する 8 つの束（「メニュー」22・「窓の配置と重なり」3・「撫で」3・「マウスの矢印」2・「descript の転記」「選択肢の目印」「切替」「バルーンの文字」1 件ずつ）の段階の表の行と、節を持つ 2 つの束の節の「依存する既存 spec」に、7 本を件数付きで足した。候補 spec 名の案はどの束でも ⑴（全数）にも ⑵（過半）にも当たらないので変えていない。
- 引き取る spec が台帳に既に書かれていた行（ゴーストの `balloon` の `note` の `areka-P0-ghost-inner-balloon`）と、本 spec の範囲の外の行（`balloon.syncscale`・`OnShellScaling`・`OnBalloonScaling`・スクロールの矢印・`communicatebox`）は変えていない。ぱすたが使わない行は 1 行も変えていない（要件 9.4。差分は上の 34 行だけ）。
- `OnSecondChange` は、走行でぱすたの記録に要求の行が 0 だったが、areka の記録の水準では送ったかどうかが分からないので、台帳の「実装済み」と食い違うとは記録から言えない。5.1 の崩れた所の表で仕分け、原因が分かって台帳と食い違えば、そのときに直す。
- **食い違いの残りは 0 件**（ぱすたが使う項目の行 154 のうち、記録と食い違う行は 0）。

### 確かめ（2026-10-23）

| 命令 | 結果 |
|---|---|
| `cargo test -p ukadoc-survey`（宛先だけを書き、`roadmap-draft.md` の行を足す前） | 赤。`spec_checks::the_spec_table_agrees_with_the_directories_the_ledgers_and_the_bundles` ほか 1 件が FAILED で、「宛先 areka-P0-descript-window-keys が … roadmap-draft.md の [[spec]] にも … [[owner_completed]] にも無い（要件 7.4 ⑶）」などを 7 本の名前で出した |
| `cargo test -p ukadoc-survey`（7 行を足した後） | 緑（601・6・113・5 件 ok・失敗 0） |
| `cargo run -p ukadoc-survey -- check` | 終了コード 0・「食い違い 0 件」 |
| `cargo run -p ukadoc-survey -- report` と `report-summary` | 作り直した。`doc/ukadoc-coverage/report/` の中身の差分は 0（報告は `owner` を載せない） |
