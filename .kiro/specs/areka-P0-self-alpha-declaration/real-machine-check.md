# 実機の確かめ（areka-P0-self-alpha-declaration・タスク 5）

本物の areka（debug 版）を起動し、⑴ 手持ちの資産の見た目が本仕様の前と同じこと（要件 8.1〜8.3）、⑵ 透過の宣言の `info!` が「起動と採寸で 1 行ずつ」出ること（要件 7.1）、⑶ 画像だけのシェルが起動し、画像だけで組んだ記録が出ること（要件 6.1・7.3）、⑷ バルーンの宣言を `0`／`full` に書き換えた写しが、宣言どおりの絵と記録になること（要件 2.3・2.4）を確かめた。

見た目は目で見比べる代わりに、MCP の `dump_surface`（面の番号を指定して単体で合成した初期状態の絵）と `dump_balloon`（最後に描いたバルーンの背景と文字）が返す原寸の透過 PNG を、**本仕様の前の版で同じ写しを動かして撮った PNG と画素ごとに**比べた。判定はすべて道具（`real-machine-run.ps1 -Check`）が記録と画素を数えて決める。

- 実施日: 2026-10-06 11:50〜12:25（JST）
- 今の版: ワークツリーの HEAD `50ad5977`（作業ツリーは本書と道具の 2 ファイルのほかは無変更）
- 今までの版: 本仕様の最初のコードのコミット `4db4b666` の親 `29a11b38`（コードは merge-base `ec072853` と同じ。その後は本仕様の文書のコミットだけ）

## 1. 走らせ方

| 項目 | 値 |
|---|---|
| 道具 | 本フォルダの `real-machine-run.ps1`（`-Prepare`／`-Run <場合…|all> -Build head|base`／`-Check`）。端末へは ASCII だけを出す。記録の行の中身（日本語）は `target\sad-rm\check-detail.txt` へ書く |
| 置き場 | すべてワークツリーの `target\sad-rm\` の下（`C:\` 直下・`C:\tmp` は使っていない） |
| 今までの版の組み方 | `git archive 29a11b38` を `target\sad-rm\baseline-src` に展開し、`cargo build -p areka --bin areka --target-dir target\sad-rm\baseline-target`。作業ツリーと git の状態には触れていない（展開のとき Windows の tar が日本語の名前の `.pdn` 1 つで終了コード 1 を返すが、ビルドに使わないファイル） |
| 補助プロセス | `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` の 1 本を両方の版に置いた（PE の機種 `014C`）。`crates/shiori-host32-helper`・`shiori-host32-ipc`・`shiori-abi` は 2 つの版で差分 0 を道具が確かめている |
| 実行体 | `target\sad-rm\bin\head\areka.exe`・`bin\base\areka.exe`（どちらも機種 `8664`）の隣に補助プロセス |
| 検体 | `nar-sample-path` で `emo2`・`R_POST_and_KOMAINU`・`konnoyayame`・`claudia`・`StayseeBalloon` を 1 本ずつ順に展開し、`target\sad-rm\pristine` へ写した。走らせるたびに、使うゴーストとバルーンだけを `target\sad-rm\run\<場合>-<版>\root` へ写し直す（前の回の記憶を持ち越さない） |
| emo2 の台本 | 写しの `ghost\master\scripts\pasta\shiori\event\boot.lua` を差し替え、起動の台詞 `\0\s[0]self alpha check. scope zero.\1\s[10]scope one.\e` だけを話し、ランダムトーク・マウス・時報などは黙らせた（バルーンの文字を 2 つの版で同じにして画素で比べるため。前例の `second_change.lua` は今の emo2 に無いので、起動のイベントの置き場の `boot.lua` を差し替えた） |
| 起動 | `areka.exe "<root>\ghost\<名>" "<root>\balloon\<名>"`（ゴーストとバルーンを絶対パスで渡す・`-WindowStyle Hidden`・`Start-Process -PassThru` で pid を記録） |
| 環境変数 | `AREKA_*`／`WINTF_*` を全部外した上で `NO_COLOR=1`・`RUST_LOG=info,areka_emo_atlas=debug,areka::mcp=debug`（宣言の記録は `info!`／`warn!`、抜き色の記録は `areka_emo_atlas` の `debug!`）・`AREKA_APP_SMOKE_EXIT_MS=100000`（有界の自動終了）・`AREKA_NO_ALERT=1`・`AREKA_MCP_PORT=39871`（よその areka と待受がぶつからないように明示）・`AREKA_ROOT`＝root・`AREKA_PROFILE_DIR`＝root\profile・`TMP`／`TEMP`＝root\tmp |
| 撮り方 | 待受が答えてから 20 秒待ち（台詞が出終わるまで）、シェルの `surfaces.txt` の見出しと `surface<数字>.png` から集めた面の番号すべてを `dump_surface {scope:0, surface:N}` で、続けて `dump_balloon {scope:0}`・`{scope:1}` を撮った。待受へは `POST http://127.0.0.1:39871/api/mcp/v1` の `tools/call` を直に送った |
| 記録 | `run\<場合>-<版>\run.log`（標準出力）・`run.err.log`・`out\*.png`・`out\answers.txt`（撮影ごとの答えの本文）・`exit.txt`、走行の一覧は `target\sad-rm\runs.txt` |

```
pwsh -NoProfile -File .kiro/specs/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Prepare
pwsh -NoProfile -File .kiro/specs/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Run all -Build head
pwsh -NoProfile -File .kiro/specs/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Run all -Build base
pwsh -NoProfile -File .kiro/specs/areka-P0-self-alpha-declaration/real-machine-run.ps1 -Check
```

### 場合

| 場合 | ゴースト（シェルの宣言） | バルーン（宣言） | 版 |
|---|---|---|---|
| emo2-kakukaku | emo2（`1`） | emo2-kakukaku（`1`） | 今・今まで |
| emo2-staysee | emo2（`1`） | StayseeBalloon（`1`） | 今・今まで |
| emo2-claudia | emo2（`1`） | claudia（`1`） | 今・今まで |
| emo2-claudia_v | emo2（`1`） | claudia_vertical（`1`） | 今・今まで |
| rpost | R_POST_and_KOMAINU（宣言なし・PNG はすべて RGB） | StayseeBalloon | 今・今まで |
| konnoyayame | konnoyayame（宣言なし・PNG はすべてパレット） | StayseeBalloon | 今・今まで |
| claudia | claudia（`1`） | claudia（`1`） | 今・今まで |
| imageonly | emo2 の写しのシェルを `descript.txt`（`seriko.use_self_alpha,1`）と `surface0.png`（R_POST の `surface0000.png`・RGB）だけにしたもの | StayseeBalloon | 今 |
| staysee-0 | emo2 | Staysee の写しの `use_self_alpha` を `0` に（絵は α あり） | 今 |
| staysee-full | emo2 | 同 `full`（絵は α あり） | 今 |
| staysee-rgb-1 | emo2 | Staysee の写しの `balloon*.png` を赤紫（255,0,255）の上に合成した α なしの絵（色の型 2）・宣言 `1` | 今 |
| staysee-rgb-full | emo2 | 同じ α なしの絵・宣言 `full` | 今 |

`R_POST_and_KOMAINU`・`konnoyayame`・`claudia` は自分の台本（里々・YAYA）で動かした。バルーンの写しの宣言の書き換えは `descript.txt` の `use_self_alpha,` の行の値だけをバイトのまま差し替えた（Shift_JIS の行を壊さない・`use_input_alpha,1` の行はそのまま）。

## 2. 結果

`-Check` の判定 133 項目がすべて合格（**RESULT: PASS**）。走行 19 回（今の版 12・今までの版 7）はどれも有界の自動終了（`app_exit` origin=Smoke 1 行）で終了コード 0 だった。

### ⑴ 見た目が今までと同じ（要件 8.1・8.2・8.3）

同じ写し・同じ台本で 2 つの版を動かし、撮れた PNG を画素ごとに比べた。

| 場合 | 撮れた絵（面＋バルーン 2） | 画素の違う絵 | 片方にしか無い絵 | 抜き色の記録（今／今まで） |
|---|---|---|---|---|
| emo2-kakukaku | 61（面 59） | **0** | 0 | 2／2（`purple/a/null.png`） |
| emo2-staysee | 61 | **0** | 0 | 2／2 |
| emo2-claudia | 61 | **0** | 0 | 2／2 |
| emo2-claudia_v | 61 | **0** | 0 | 2／2 |
| rpost | 12（面 10） | **0** | 0 | 20／20（面の絵 10 枚 × 読み込み 2 回） |
| konnoyayame | 20（面 18） | **0** | 0 | 36／36 |
| claudia | 17（面 15） | **0** | 0 | 0／0 |

- 抜き色の記録（`bake: element を抜き色（左上の 1 画素と同じ色）で透過しました`）は、絵の名前・抜いた色（b・g・r・a）まで 2 つの版で同じ並びだった。
- 宣言の無い `R_POST_and_KOMAINU`・`konnoyayame` は、今の版で宣言なしの決まり（α を持たない絵は抜き色）を通り、今までの `1` 固定と 1 画素も違わない（要件 8.2・9.3）。
- 要件 8.3 の emo2-kakukaku の残りの版（`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`）は起動していない。書庫を開いて確かめると、どちらも PNG が emo2-kakukaku とバイトまで同じで、宣言も `use_self_alpha,1` だった。違うのは descript.txt と balloon*s.txt の文字の配置の設定だけなので、透過の扱いは emo2-kakukaku の結果で足りる。
- 撮影の失敗（`isError` か JSON-RPC の誤り）は全走行で **0 件**。
- `rpost`・`konnoyayame`・`claudia` のバルーンはゴーストの台本の台詞を写すが、今回の 2 つの版では台詞が同じで、バルーンの絵も一致した。

### ⑵ 宣言の記録（要件 7.1）

`self_alpha: 透過の扱いを決めた` の `info!` は、今の版の 12 走行すべてで **シェル 2 行・バルーン 2 行**（読み込みの入口が起動の組み立てと採寸で 1 回ずつ＝各 1 行）。宣言の値が読めない `warn!`（`self_alpha: 透過の宣言の値が読めない…`）は **0 行**。今までの版では `self_alpha:` の行は 0 行（入口が無い版なので当然）。

| 場合 | シェルの `treatment`／`declared` | バルーンの `treatment`／`declared` |
|---|---|---|
| emo2-*・claudia・imageonly | `1`／true | `1`／true |
| rpost・konnoyayame | `none`／false | `1`／true |
| staysee-0 | `1`／true | `0`／true |
| staysee-full・staysee-rgb-full | `1`／true | `full`／true |
| staysee-rgb-1 | `1`／true | `1`／true |

記録の例（rpost・`<root>` は `target\sad-rm\run\rpost-head\root`）:

```
INFO areka_emo_present::self_alpha: self_alpha: 透過の扱いを決めた kind="shell" key="seriko.use_self_alpha" dir=<root>\ghost\R_POST_and_KOMAINU\shell\master treatment="none" declared=false
INFO areka_emo_present::self_alpha: self_alpha: 透過の扱いを決めた kind="balloon" key="use_self_alpha" dir=<root>\balloon\StayseeBalloon treatment="1" declared=true
```

`.pna` を使わずに表示した `warn!`（`ignored_pna=` の行）は、シェル・バルーンとも全走行で **0 行**（手持ちの検体に `.pna` は 0 個）。

### ⑶ 画像だけのシェル（要件 6.1・7.3）

`imageonly` は起動し、有界の自動終了で終了コード 0。画像だけで組んだ `info!` が読み込み 1 回につき 1 行（起動と採寸で **2 行**）、どちらも `surfaces_txt="missing" surfaces=1`。

```
INFO areka_emo_present::shell_target: shell: surfaces.txt が面を定義しないので、面の画像だけで面を組んだ（要件 7.3） shell_dir=<root>\ghost\imageonly\shell\master surfaces_txt="missing" surfaces=1
```

`dump_surface {surface:0}` は 236×462 の絵を返し、元の `surface0.png`（RGB）の左上と同じ色の画素だけが透明（α=0 が 59,831・不透明 49,201・半透明 **0**）。期待の α（左上と 4 バイトが同じなら 0・ほかは 255）と食い違う画素は **0**。

この走行の ERROR 65 行はすべて、emo2 のゴーストが面 0 しか無いシェルに無いものを頼んだための「無いので読み飛ばす」系だった（`bind の (カテゴリ, パーツ) を名前解決できず` 54 行・`surface を解決できず` 6 行・面 10 と 1000 の `SurfaceNotFound`／合成失敗 5 行の内訳で、emo2 の着せ替えと面の別名・`\1` の面 10・起動の面 1000 を指すもの）。どれも理由つきで記録され、それ以外の ERROR は **0 行**。採寸は `measure: スコープの採寸合成に失敗（scope0 の寸法で代替） scope=1 surface_id=10` の `warn!` 1 行で、相方の面が無いときの今の代わりの道を通った（design の Risks に書いたとおり・本仕様では変えない）。

### ⑷ バルーンの宣言の描き分け（要件 2.3・2.4）

撮ったバルーンの絵を、同じ大きさの元の絵（スコープ 0 は `balloons*.png`、スコープ 1 は `balloonk*.png` から食い違いの最も少ないもの）と比べ、宣言ごとの期待の α と食い違う画素を数えた。どの行も元の絵は `balloons0.png`（335×205）／`balloonk0.png`（335×135）。

| 場合 | スコープ | α=0 | 半透明 | 不透明 | 左上の α | 期待 | 食い違い |
|---|---|---|---|---|---|---|---|
| emo2-staysee（`1`・α あり） | 0 | 9,617 | 2,445 | 56,613 | 0 | 元の α のまま | **0** |
| staysee-0（`0`・α あり） | 0 | 11,206 | **0** | 57,469 | 0 | α を捨て、赤緑青が左上と同じ画素だけ透明 | **0** |
| staysee-full（`full`・α あり） | 0 | 9,617 | 2,445 | 56,613 | 0 | 元の α のまま（`1` と同じ） | **0** |
| staysee-rgb-1（`1`・α なし） | 0 | 9,617 | 0 | 59,058 | 0 | 左上の色（赤紫）だけ透明 | **0** |
| staysee-rgb-full（`full`・α なし） | 0 | **0** | 0 | 68,675 | 255 | 全面不透明・左上の色も抜かない | **0** |
| emo2-staysee（`1`・α あり） | 1 | 8,567 | 2,190 | 34,468 | 0 | 元の α のまま | **0** |
| staysee-0 | 1 | 9,946 | **0** | 35,279 | 0 | α を捨てて抜く | **0** |
| staysee-full | 1 | 8,567 | 2,190 | 34,468 | 0 | 元の α のまま | **0** |
| staysee-rgb-1 | 1 | 8,567 | 0 | 36,658 | 0 | 左上の色だけ透明 | **0** |
| staysee-rgb-full | 1 | **0** | 0 | 45,225 | 255 | 全面不透明 | **0** |

- `0`: 元の絵の半透明（スコープ 0 で 2,445 画素）が **0** になり、左上の色（Staysee の透明な画素の色は黒 0,0,0）が抜けた。α=0 が 9,617 → 11,206 に増えたのは、元が半透明で色が黒だった画素も「左上と同じ色」として抜けたためで、期待どおり（要件 5.2・5.3・5.6）。
- `full`: α を持つ絵では `1` と画素まで同じ（両スコープとも違う画素 **0**・要件 4.1）。α を持たない絵では左上の赤紫も不透明のまま（α=0 が **0**・要件 4.2）で、同じ絵を `1` で描くと赤紫が抜ける（staysee-rgb-1）のと対になっている。
- 抜き色の記録は、`0` と α なし×`1` でバルーンの絵 6 枚（`balloonk0`・`balloonk1`・`balloons0`〜`3`）が加わり、`full` では 0 枚だった。

### ERROR・WARN の内訳（今の版／今までの版）

- ERROR: `imageonly` の 65 行（上の ⑶）のほかは全走行 **0 行**。
- WARN の種類と数は、比べた 7 つの場合のすべてで 2 つの版が同じだった。どれも本仕様の前からあるもの:
  - `bake: element が全透明（α=0）でトリム後 0 寸です` — emo2 の `purple/a/null.png`（読み込み 2 回で 2 行・検体の作り）
  - `balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2／3` — Staysee の `balloonk2`・`balloonk3` が無いため（6 行）
  - `areka_emo_text::actor::attach: 折返し基準が描画範囲の外に解決された` — emo2-kakukaku の定義（1 行）
  - `kanade: 強制終了指示——終了系列（Forced）へ直行 reason="user"` — 有界の自動終了の終わり方（各走行 1 行）
- 本仕様で足した `warn!`（宣言の値が読めない・`.pna` を使わない・シェルの descript.txt が読めない）は全走行で **0 行**。

### 起こしたプロセスの後始末

19 回とも自分で `Start-Process -PassThru` で起こした pid（`runs.txt` に記録）が有界の自動終了で自分から終わるのを待った。止める操作は 1 度もしていない。`runs.txt` には起動が 20 回ある。先頭の emo2-staysee-head（pid 25492・撮れた絵 2 枚）は、後で同じ場合を撮り直して置き場を作り直した試しの回で、これも自分で起こし、自分から終わった（終了コード 0）。走り終えた後、`target\sad-rm` の実行体から起きた areka のプロセスは **0 個**。よそのセッションのプロセスには触れていない。

## 3. 目で見る確かめについて

画面の撮影（computer-use）はしなかった。areka が画面に出す絵は、MCP が返したのと同じ合成の結果を窓へ描いたものなので、画素の一致（⑴）と期待の α との食い違い 0（⑶・⑷）を見た目の確かめとした。
