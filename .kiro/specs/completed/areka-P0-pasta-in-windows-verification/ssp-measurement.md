# ぱすたの SSP での測りの記録

spec `areka-P0-pasta-in-windows-verification` の、SSP でのぱすたの全体の通しの測り（タスク 2.1）の記録。areka の走行の結果を比べる基準にする。時刻はすべて日本時間。ぱすたの記録 `pasta.log` の時刻は世界標準時（`Z`）で残っているので、9 時間を足して書いた。

撮った物と記録の控えは git の外の `target\pasta-verify\ssp\` に在る。下ではワークツリーからの相対の名前で書く（`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`）。

## 1. 測りの条件

| 項目 | 値 |
|---|---|
| 日時 | 2026-10-23 07:58〜08:13 |
| SSP の版 | `SSP/2.9.23 (20261021-5; Windows NT 10.0.26300)`（ssp MCP の `get_property` で `baseware.version` を読んだ値）。**要件（Requirement 1）は 2.9.18 と書いているが、測ったのは 2.9.23**。開発者の機械の SSP が上がっていたため。手順書（`doc/pasta-real-run.md`）はタスク 8.1 で 2.9.23 に直した。本書の「SSP」はすべて 2.9.23 を指す |
| SSP の置き場 | `C:\wintools\ssp\ssp.exe` |
| 書庫の同定 | `target\nar-samples\fetched\pasta-in-windows.nar.sha256` の中身は `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6  pasta-in-windows.nar`。書庫の中の `ghost/master/pasta.toml` は `version = "1.0.0"`（descript に版の行は無い） |
| SSP に入っているぱすたが同じ版か | `C:\wintools\ssp\ghost\pasta-in-windows` と、この書庫から組んだ走行の根（実機で走らせるために、ゴーストとバルーンを置いて組んだフォルダ）`target\pasta-verify\root\ghost\pasta-in-windows` の 73 ファイルを sha256 で突き合わせ、足りないファイル 0・中身の違い 0 |
| ゴーストの名前 | `窓際のぱすたさん`（どの道具の呼び出しにも `ghost_name` で渡した）。名前の似た別のゴースト（`窓際のぱすたさんDRESSTEST`・開発用の `窓際のぱすたさんDEBUG`）が SSP に入っているが、名前が違うので取り違えは無い。`currentghost.path` が `C:\wintools\ssp\ghost\pasta-in-windows\` であることも確かめた |
| 机 | 測りのあいだ排他テストの机（実機の画面とマウスを 1 つのセッションだけが使うための順番の札）を進行役（この spec の作業を進めて、測りの操作をしたセッション）が持ち、測り終えてすぐ返した |
| 許可 | 2026-10-23 のチャットで開発者が「つかってよい。操作は任せます。」と答えた。これで SSP の起動・ゴーストの切り替え・SendInput を進行役が行った。SSP の読み直し（reload）は打っていない |
| 道具 | `tools/mouse-input.ps1`（`move`・`click`・`rightclick`・`drag`・`shot`）・ssp MCP（`get_property`・`raise_event`・`dump_surface`・`dump_balloon`・`get_log`） |
| SHIORI の要求の記録 | このぱすたの `C:\wintools\ssp\ghost\pasta-in-windows\ghost\master\` には `ssp_shiori_log.txt` が無い（DRESSTEST の `ghost\master\` にだけ在る）。代わりに、ぱすた自身が要求と応答を逐語で残す `C:\wintools\ssp\ghost\pasta-in-windows\ghost\master\profile\pasta\logs\pasta.log` を段ごとに切り出して `stageN-pasta-log.txt` に残した |
| 画面 | 起動したときのぱすたは 150% の画面（左上 -2560,195・2560x1600）に立った。主の画面は 200%（DPI 192） |
| 起動の段の起動イベント | SSP を起こしたときはクローディアが立っていた。`\![change,ghost,窓際のぱすたさん]` でぱすたへ替え、`\-` で SSP を閉じ、07:59:47 に起こし直した。このため起動の段は記憶のある起動で、SSP は `OnBoot` を送った（段 1） |
| 測りの後に残った変化 | 許された切り替えと起こし直しの結果として、**SSP 自身が「最後のゴースト」をぱすたとして保存した**（測りの前はクローディア）。進行役は SSP の本体・設定・SSP が保存するファイルを手で書き換えていない。クローディアへ戻すかは開発者が決める |
| 測らなかった物 | α の当たりの線と α の描き方（Requirement 1 の 2 項） |
| 縦書きのバルーンの出どころ（2026-10-23 直し） | 測りの時は「ゴーストのフォルダの中の `pasta_tategaki` を使った」と書いたが誤り。SSP の根の `balloon\` に同じバルーンの写し `C:\wintools\ssp\balloon\pasta_tategaki` が在り、SSP はそれを使っていた。写しが無ければ SSP も既定のバルーンで出る（`areka-P0-ghost-inner-balloon` の SSP の測り・main #364）。配布の書庫（10-20 版）は `install.txt` にバルーンの鍵が無く、書庫を入れただけの利用者の手元にこの写しは無い。**つまり段 1・段 2 の基準の縦書きのバルーンは、利用者が配布の書庫から得られない置き方の上で測ったもので、run1 と比べた前提（同じ書庫なら SSP は縦書きで出る）は成り立たない**。ぱすたの側が `install.txt` を直した（未リリース・`verification.md` 5 節の 1） |

ukadoc との照らし合わせ:

- `\![change,ghost,ゴースト名]` — https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_change%2Cghost%2C%E3%82%B4%E3%83%BC%E3%82%B9%E3%83%88%E5%90%8D_ 「そのゴーストへの切り替えを行う。」。食い違い: なし（ぱすたへ替わった）。
- `\-` — https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_- 「本体を終了する。」。食い違い: なし。

## 2. 全体の通しの基準（7 段）

段の順は Requirement 4 と手順書 5 節の表と同じ。各項目の「SSP の版」と「書庫の同定」は 1 節の値（`SSP/2.9.23 (20261021-5; Windows NT 10.0.26300)`・`ad473d5d…a43a6`＝`pasta.toml` 1.0.0）で、全部の段で同じ。

### 段 1 起動

- 日時: 2026-10-23 07:59:49（`OnBoot` の要求の時刻。`pasta.log` の `2026-10-22T22:59:49.909927Z`）。撮影は 08:01
- SSP の版: 2.9.23（1 節）
- 書庫の同定: `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）
- 道具と引数: SSP を起こし直す（1 節）→ `get_property`（`ghost_name` = `窓際のぱすたさん`。面の番号・バルーンの名前・`scope(0)`／`scope(1)` とバルーンの rect）・`dump_surface`（`\0`・`\1`）・`dump_balloon`（`\0`）・`mouse-input.ps1 shot`
- 結果:
  - SSP が送った起動イベントは `OnBoot`（`Reference0: パスタ`）。台本の頭は `\0\s[1000]\1\s[10]` で、`\![set,zorder,s0,b0,s1]`・`\![set,windowstate,!stayontop]` を含む。
  - `\0` は面 1000、`\1` は面 10（窓の枠）。
  - バルーンは `窓際のぱすたさん縦書き`（SSP の根の `balloon\` に在った写し `C:\wintools\ssp\balloon\pasta_tategaki`。測りの時に「ゴーストのフォルダの中」と書いたのは誤りで、2026-10-23 に直した・1 節）。
  - `scope(0)` と `scope(1)` の rect はどちらも `-782,201,49,1227`（窓の枠とぱすたが同じ左上に重なる）。バルーンの rect は `-722,288,-68,963`。
  - 台詞は「‥‥あ、点いた。おはようございます。ビルの隙間から、朝日がちょっとだけ見えました。」。
  - **撮ったときは台詞が出終わってバルーンが閉じていた**ので、`stage1-balloon0.png` は空のバルーンで、`stage1-shot.png` にも台詞は写っていない。台詞の逐語の根拠は `stage1-pasta-log.txt` の応答の Value。重なりの順（窓が奥 → バルーンの字 → ぱすたが手前）は段 2 の撮影で見える。
- 根拠: `target\pasta-verify\ssp\stage1-pasta-log.txt`（`ID: OnBoot` の要求と応答の Value の逐語）・`stage1-shot.png`・`stage1-surface0.png`（554x684）・`stage1-surface1.png`・`stage1-balloon0.png`（空）
- ukadoc:
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBoot 「起動した際に発生。」・Reference0 は「起動時のシェル名。」。食い違い: なし（`Reference0: パスタ`）。
  - https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_set%2Czorder%2C%E3%82%B9%E3%82%B3%E3%83%BC%E3%83%97ID%2C%E3%82%B9%E3%82%B3%E3%83%BC%E3%83%97ID%2C_ 「指定スコープのウインドウを左側から順に手前側に配置する。」。食い違い: なし（`s0,b0,s1` のとおり、ぱすた → バルーン → 窓の枠の順に手前。段 2 の撮影で確かめた）。
  - https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#sakura.seriko.defaultsurface%2C%E6%95%B0%E5%80%A4 「本体側のゴーストのデフォルトのサーフェス番号。」（ぱすたの descript は `sakura.seriko.defaultsurface,1000`）。食い違い: なし（`\0` は面 1000。ただし台本も `\s[1000]` を出すので、この段だけでは descript の効きと台本の効きを分けられない）。

### 段 2 縦書きの会話

- 日時: 2026-10-23 08:02:02（`選択肢テスト` の応答）・08:02:26（`OnChoiceSelectEx`）・08:03〜08:04（`縦書き試作` の撮影）
- SSP の版: 2.9.23（1 節）
- 書庫の同定: `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）
- 道具と引数: `raise_event`（`ghost_name` = `窓際のぱすたさん`・`event` = `選択肢テスト`。ボール #11 と同じ問い）→ `mouse-input.ps1 click -552 362`（「雨だよ」）→ `raise_event`（`event` = `縦書き試作`）・`shot`
- 結果:
  - `raise_event` の答えは `OK:the ghost returned a script and it is being played.`。台本は縦書きの列を `\n[180]` で区切り、`\_l[-253,0]\n[150]` の後に `\![*]\q[晴れてる,晴れてる,選択肢テスト_1]` など選択肢を 4 つ並べる。
  - 選択肢 4 つが、それぞれ印付きで左の枠に並んだ（`stage2-choice-shot.png`）。
  - 「雨だよ」を押すと `OnChoiceSelectEx`（`Reference0: 雨だよ`・`Reference1: 雨だよ`・`Reference2: 選択肢テスト_1`）が届き、「そういえば薄暗いような？」が出た（`stage2-after-shot.png`）。
  - `縦書き試作` では、右の枠と左の枠（`\_l[-283,0]`）が崩れずに並んだ（`stage2-tategaki-frames-shot.png`）。
  - 重なりは窓の枠（奥）→ バルーンの字 → ぱすた（手前）。
- 根拠: `target\pasta-verify\ssp\stage2-script.txt`（`raise_event` の答えの逐語）・`stage2-pasta-log.txt`（`ID: 選択肢テスト` と `ID: OnChoiceSelectEx` の要求と応答の逐語）・`stage2-choice-shot.png`・`stage2-after-shot.png`・`stage2-tategaki-p1-shot.png`・`stage2-tategaki-frames-shot.png`
- ukadoc:
  - https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_q_%E3%82%BF%E3%82%A4%E3%83%88%E3%83%AB%2CID%2Cr2%2Cr3..._ 「r*はOnChoiceSelectExのReference*に格納される。」。食い違い: なし（3 番目の引数 `選択肢テスト_1` が `Reference2` に入った）。
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnChoiceSelectEx Reference0 は「選択肢のテキスト（ラベル）。」。食い違い: なし。
  - https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_*_ 「マーカーを表示する。マーカーはバルーンによって設定されている。」。食い違い: なし（選択肢の前に印が出た）。
  - https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#__l_x%2Cy_ 「現スコープ側のカーソル位置をXYの座標に移動する。」。食い違い: なし（`\_l[-253,0]`・`\_l[-283,0]` で左の枠へ移った）。

### 段 3 撫で・× に乗る・ボール

- 日時: 2026-10-23 08:04:56（顔の `OnMouseMove` と撫での応答）・08:06（× に乗った撮影）・08:07:30（ボールの `OnMouseClick`）
- SSP の版: 2.9.23（1 節）
- 書庫の同定: `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）
- 道具と引数: `mouse-input.ps1 move`（顔の上を往復）・`move`（× の印の上）・`shot`・カーソルの形の読み取り（`stage3-cursor.txt` の 5 点）・`click`（ボール）・ssp MCP `get_log`
- 結果:
  - 顔を撫でると `OnMouseMove`（`Reference4: 顔`・`Reference6: mouse`）が届き、「…そのてのひら、ガラスの向こうまで届きそうだね。」が出た（`get_log` の台本 #27・#29 と `stage3-pasta-log.txt` の応答の Value の逐語）。台詞は約 2.5 秒で閉じ、撮影には写らなかった。続けて撫でると、2 回目は間を空ける決まり（ぱすたの辞書の作り）で黙ることがある。
  - × の印に乗ると `OnMouseEnter`（`Reference4: 閉じる`）が届き、× の地が赤くなった（`stage3-x-hover-shot.png`）。離れると `OnMouseLeave`（`Reference4: 閉じる`）が届いた。
  - マウスの形（`stage3-cursor.txt`）: ×（-84,258）とボール（-615,805）は `HAND`（Windows の標準の指さしの形・ハンドル `0x1001F`）、身体（-410,450 と -300,640）は `custom`（同じハンドル `0x46160A0D`）、窓の白地（-700,1000）は `ARROW`。
  - この形はシェルの `surfaces.txt` の `sakura.cursor` ブレスで決まる。ブレスには `mouseup0,顔,system:hand` から `mouseup19,胸左,system:hand` までの 20 行があり、身体の当たり判定 20 個すべてに `system:hand`（手を開いた形）を当てている。`閉じる`（×）と `セキュリティボール` の行は無く、ほかの範囲のカーソルのブレス（`kero.cursor` など）も無い。読み方: 身体の `custom` は、シェルが指定した `system:hand` を SSP が自分の手の形のカーソルとして出したもの（Windows の標準のカーソルのどれでもないので `custom` と読めた）。× とボールは指定が無いので既定の `system:finger` になり、これが Windows の標準の指さしの形（`HAND`）として読めた。
  - ボールを押すと `OnMouseClick`（`Reference4: セキュリティボール`）が届き、ボールのトークが出た（`stage3-ball-shot.png`）。台詞は `stage3-pasta-log.txt` の 201 行目、この要求への応答の Value の逐語（UTF-8 で読んだ。置き換えの文字 U+FFFD は 0）から `\![bind,…]` と `\_w[…]` を除くと「そこのボールで、私の姿を映してます。」「つぶやきとか、チャット風景とか。」「私の姿だけを流すように細工してますけど。」（`\_l[-283,0]` で左の枠へ）「私の部屋、ちょっと殺風景なので‥‥。」「んー？」「覗き込んでも見えないよ？」で、`\e` で終わる。
  - 届いたイベントは `OnMouseMove`・`OnMouseEnter`・`OnMouseLeave`・`OnMouseClick`。
- 根拠: `target\pasta-verify\ssp\stage3-pasta-log.txt`（`ID: OnMouseMove`・`OnMouseEnter`・`OnMouseLeave`・`OnMouseClick` の要求と、撫での応答とボールのトークの応答の逐語）・`stage3-cursor.txt`・`target\pasta-verify\root\ghost\pasta-in-windows\shell\master\surfaces.txt` の `sakura.cursor` ブレス（書庫から組んだ物で、SSP に入っているぱすたと中身が同じ。1 節）・`stage3-before-shot.png`・`stage3-stroke-after-shot.png`・`stage3-x-hover-shot.png`・`stage3-ball-shot.png`
- ukadoc:
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseMove 「マウスが移動した際に発生。」・Reference4 は「当たり判定の識別子。」。食い違い: なし。
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseEnter 「マウスが当たり判定に入った際に発生。」。食い違い: なし。
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseLeave 「マウスが当たり判定から出た際に発生。」。食い違い: なし。
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseClick Reference5 は「左クリックは0、右クリックは1、中クリックは2」。食い違い: なし（`Reference5: 0`）。
  - https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#mouseup*%2C%E5%BD%93%E3%81%9F%E3%82%8A%E5%88%A4%E5%AE%9AID%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D 「当たり判定の上に乗った時表示するカーソルファイルを設定。」。既定は「system:finger（当たり判定名がbustの場合のみsystem:hand）」。食い違い: なし（身体は指定どおり `system:hand`、指定の無い × とボールは既定の `system:finger`）。
  - https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#system%3Ahand 「手」。食い違い: なし（身体の形は `HAND` とは別のハンドルの `custom` で、指さしの既定とは別の形が出ている。形そのものの絵は撮っていない）。

### 段 4 2 つの窓を一緒にドラッグ

- 日時: 2026-10-23 08:08
- SSP の版: 2.9.23（1 節）
- 書庫の同定: `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）
- 道具と引数: `mouse-input.ps1 drag -300 640 -1000 640`（身体をつかんで同じ 150% の画面の中で左へ）・`shot`（前と後）・`get_property`（`scope(0)`／`scope(1)` の rect）
- 結果: `scope(0)` と `scope(1)` がどちらも `-1482,201,-651,1227` へ揃って動いた（段 1 の `-782,201,49,1227` から左へ 700）。ぱすたと窓の枠は離れず、並びと重なりも保たれた。ドラッグに伴うイベントは `stage4-pasta-log.txt` に出ていない（`OnSecondChange` だけ）。
- 根拠: `target\pasta-verify\ssp\stage4-before-shot.png`・`stage4-after-shot.png`・`stage4-pasta-log.txt`・`stage5-rects.txt` の 1 行目（`before (150% monitor): scope0 -1482,201,-651,1227 scope1 same`）
- ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_set%2Csticky-window%2C%E3%82%B9%E3%82%B3%E3%83%BC%E3%83%97ID%2C%E3%82%B9%E3%82%B3%E3%83%BC%E3%83%97ID%2C_ 「指定スコープのウインドウをドラッグ中やmoveタグを実行中にセットで動くようにする。」（ぱすたのシェルの descript は `seriko.sticky-window,0,1`）。食い違い: なし。

### 段 5 拡大率の違う画面への移動

- 日時: 2026-10-23 08:09
- SSP の版: 2.9.23（1 節）
- 書庫の同定: `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）
- 道具と引数: `mouse-input.ps1 drag -1000 640 1300 700`（150% の画面から 200% の主の画面へ）・`get_property`（`scope(0)`／`scope(1)` の rect・`currentghost.scope(0).currentmonitor.dpi`・`currentghost.scope(0).scaling`・バルーンの rect）・`shot`。**この段は `dump_surface` を取っていない**。根拠は `get_property` の値（`stage5-rects.txt`）と撮影
- 結果:
  - `scope(0)` と `scope(1)` はどちらも `818,261,1649,1287`。大きさ 831x1026 は 150% の画面に居たときと同じで、**SSP は拡大率の違う画面へ移っても大きさを変えない**。
  - `currentmonitor.dpi` は 192、`scope(0).scaling` は `150,150`。
  - バルーンの rect は `878,348,1532,1023`。ぱすたの左上から 60,87 の所で、起動したとき（150% の画面）と同じ。
  - 並びと重なりは保たれた。
  - 移動に伴うイベントは `stage5-pasta-log.txt` に出ていない（在るのは `OnSecondChange` と、下の `raise_event` で出した `ID: 選択肢テスト`（08:09:15）と、その選択の `OnChoiceSelectEx`（08:09:40）だけ）。
  - この段でもう一度 `選択肢テスト` を出し、「晴れてる」を選ぶと「へえ、いいですね。」が出た。
- 根拠: `target\pasta-verify\ssp\stage5-rects.txt`（`after drag to primary: scope0 818,261,1649,1287 scope1 818,261,1649,1287`・`currentmonitor.dpi=192 scaling=150,150`・`balloon scope0 878,348,1532,1023 (offset 60,87 ...)`）・`stage5-after-shot.png`・`stage5-balloon-shot.png`・`stage5-pasta-log.txt`
- ukadoc:
  - https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#currentghost.scope(ID).scaling 「現在のゴーストのスコープIDの実効拡大率。」。食い違い: なし（150 のまま。ukadoc は画面の拡大率に合わせて変わるとは書いていない）。
  - https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnShellScaling 「シェル倍率が変更された際に発生。」。食い違い: なし（倍率が変わっていないので届かなかった）。

### 段 6 右クリックメニュー

- 日時: 2026-10-23 08:09〜08:12
- SSP の版: 2.9.23（1 節）
- 書庫の同定: `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）
- 道具と引数: `mouse-input.ps1 rightclick`（ぱすたの身体）・`click`（着せ替え → 目 → びっくり）・`shot`・`dump_surface`（`\0`。選んだ後に 2 回）
- 結果:
  - 普通の Windows のメニューに見える色のメニューが出た（`stage6-menu-shot.png`）。
  - 着せ替えのメニューには、口・目・眉・うで・まばたき・ボール・「閉じる - 赤」が並んだ（`stage6-dress-menu-shot.png`）。
  - 目 → びっくり を選ぶと面が変わった。`stage6-surface0-after-pick1.png` は起動したときの写し `stage1-surface0.png` と、顔の範囲 66,216〜299,445 で 5742 画素違う。
  - 同じ項目をもう一度選ぶと外れて戻った（`stage6-surface0-after-pick2.png` の違いは、ボールの回る絵の範囲 66,362〜173,445 だけ）。
  - ぱすたは `OnSecondChange` の応答で着せ替えの `\![bind,…]` を毎分返し直す（`stage6-pasta-log.txt` の 08:09:49 と 08:12:03 など）。このため、メニューで選んだ着せ替えは長くは残らない。
  - 切り出した `stage6-pasta-log.txt` に着せ替えの通知（`OnDressupChanged`）の要求は無い。在るのはメニューを組むための `ID: sakura.recommendsites`（08:09:52・08:10:59）と `OnSecondChange`。
- 根拠: `target\pasta-verify\ssp\stage6-menu-shot.png`・`stage6-dress-menu-shot.png`・`stage6-eye-menu-shot.png`・`stage6-eye-pick-shot.png`・`stage6-after-dress-shot.png`・`stage6-after-dress-face.png`・`stage6-surface0-after-pick1.png`・`stage6-surface0-after-pick2.png`・`stage6-pasta-log.txt`
- ukadoc:
  - https://ssp.shillest.net/ukadoc/manual/descript_shell.html#menu.background.font.color.b%2C%E6%95%B0%E5%80%A4 「バックグラウンド文字色青(0～255)」（ぱすたのシェルは `menu.background.bitmap.filename`・`menu.*.font.color.*` を指定している）。食い違い: なし（指定した絵と色で普通のメニューに見える）。
  - https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_bind%2C%E3%82%AB%E3%83%86%E3%82%B4%E3%83%AA%E5%90%8D%2C%E3%83%91%E3%83%BC%E3%83%84%E5%90%8D%2C%E6%95%B0%E5%80%A4_ 「数値欄を空欄または省略するとメニュー操作時のようにON/OFFの繰り返し」。食い違い: なし（メニューで同じ項目を 2 回選ぶと付いて外れた）。同じ項目は「スクリプト実行後、SHIORIイベントOnDressupChangedが通知され」とも書くが、メニューから選んだときに通知されるかは書いていない。切り出した記録に `OnDressupChanged` は無いが、本 spec では改めて測らない。

### 段 7 × で終わる

- 日時: 2026-10-23 08:12:54（× の `OnMouseEnter`・`OnMouseClick`）・08:13:03（ぱすたの `SHIORI.unload`）
- SSP の版: 2.9.23（1 節）
- 書庫の同定: `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）
- 道具と引数: `mouse-input.ps1 click 1516 318`（× の印）・`shot`・SSP のプロセスの終わりを 30 秒まで待つ（`WaitForExit`）
- 結果:
  - × に乗ると `OnMouseEnter`（`Reference4: 閉じる`）が届き、応答は `\![bind,閉じる,赤,1]\e`。押すと `OnMouseClick`（`Reference4: 閉じる`）が届いた。
  - 終わりの台詞「…ここからは、画面ごしのお別れだね。また、いつでも呼んで。」が出た（`stage7-farewell-shot.png`）。台本の終わりは `\_w[300]\-\e`。
  - 約 2 秒後に SSP のプロセスごと終わった（30 秒以内）。
  - **SSP の終了コードは読めなかった**（`stage7-exit.txt`: `exit code not readable from this handle`）。
- 根拠: `target\pasta-verify\ssp\stage7-pasta-log.txt`（`ID: OnMouseEnter`・`ID: OnMouseClick` の要求と応答の逐語・`SHIORI.unload called successfully`）・`stage7-farewell-shot.png`・`stage7-exit.txt`
- ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_- 「SSPで複数ゴーストが起動している場合、実行したゴーストのみ終了する。」。食い違い: なし（ぱすた 1 体だけが立っていたので SSP ごと終わった）。

## 3. 兄弟の問いで見えた答え

兄弟の spec の問いは、本 spec のために改めて測っていない。下は 2 節の通しの途中で見えた答えだけ。どの項目も SSP の版は 2.9.23、書庫の同定は `ad473d5dd1f142c168092e71b436419270b1cd16331ce19cc24f92a0c04a43a6`（1 節）。

| 問い | 申し送り先 | 日時 | 道具と引数 | 見えた答え | 根拠 | ukadoc の URL・逐語の引用・食い違い |
|---|---|---|---|---|---|---|
| ゴーストのフォルダの中の縦書きのバルーンを名前で見つけるか | `ghost-inner-balloon` | 2026-10-23 08:01 | `get_property`（バルーンの名前） | （2026-10-23 直し）SSP は `name,窓際のぱすたさん縦書き` のバルーンを使ったが、それは SSP の根の `balloon\` に在った写し（`C:\wintools\ssp\balloon\pasta_tategaki`）だった。測りの時に「ゴーストのフォルダの中の `pasta_tategaki` を使った」と書いたのは、根の写しを確かめていなかった誤り。写しが無ければ SSP もゴーストのフォルダの中を探さず、既定のバルーンで出る（`areka-P0-ghost-inner-balloon` の SSP の測り・main #364）。直す場所はぱすたの `install.txt`（`verification.md` 5 節の 1） | 段 1 の結果・`stage1-shot.png` | ukadoc の検索でこの見つけ方を書いた項目は見つからなかった。引用なし。食い違い: 判定できない |
| 選択肢の前の `\![*]` | `script-small-tags` | 2026-10-23 08:02 | `raise_event`（`選択肢テスト`）・`shot` | 選択肢 4 つの前に、それぞれ印が出た | `stage2-script.txt`・`stage2-choice-shot.png` | https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_!_*_ 「マーカーを表示する。」。食い違い: なし |
| 拡大率の違う画面へ移したときの大きさと `OnShellScaling` | `dpi-realign-remembered-chain` | 2026-10-23 08:09 | `drag -1000 640 1300 700`・`get_property` | 150% の画面から 200% の画面へ移っても、ぱすたの大きさ（831x1026）と `scope(0).scaling`（150,150）は変わらず、`OnShellScaling` も届かなかった。ぱすたと窓の枠とバルーンの並びは保たれた | `stage5-rects.txt`・`stage5-after-shot.png`・`stage5-pasta-log.txt` | https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnShellScaling 「シェル倍率が変更された際に発生。」。食い違い: なし（倍率が変わっていない） |
| `balloon.dontmove`（ぱすたの descript は `balloon.dontmove,true`） | `descript-window-keys` | 2026-10-23 08:08〜08:09 | `drag`（段 4・段 5）・`get_property`（バルーンの rect） | ぱすたを 2 回ドラッグした後も、バルーンはぱすたの左上から 60,87 の所に付いたまま動いた。利用者がバルーンを自分で動かす操作は試していない | `stage5-rects.txt` の 4 行目・`stage5-balloon-shot.png` | https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#balloon.dontmove%2Ctrue 「バルーン位置調整の標準設定をOFFにする。」。食い違い: なし（今回の操作の範囲では） |
| 2 つの窓を一緒に動かすこと（`seriko.sticky-window,0,1`） | `window-link-directives` | 2026-10-23 08:08 | `drag -300 640 -1000 640`・`get_property` | `scope(0)` と `scope(1)` が同じ量だけ揃って動いた。起動の台本の `\![move,…,--option=ignore-sticky-window]` で `\0` だけを動かしたときに `\1` が付いてくるかは測っていない | 段 4 の結果・`stage4-before-shot.png`・`stage4-after-shot.png` | https://ssp.shillest.net/ukadoc/manual/descript_shell.html#seriko.sticky-window%2C%E3%82%B9%E3%82%B3%E3%83%BC%E3%83%97ID%2C%E3%82%B9%E3%82%B3%E3%83%BC%E3%83%97ID%2C_ 「タグを実行しなくてもあらかじめ設定できる。」。食い違い: なし |
| 当たり判定ごとのマウスの形 | `shell-mouse-cursor` | 2026-10-23 08:04〜08:07 | `move`・カーソルの形の読み取り | 身体は、シェルの `surfaces.txt` の `sakura.cursor` ブレス（`mouseup0,顔,system:hand` 〜 `mouseup19,胸左,system:hand`）が当てた `system:hand`（手を開いた形。読み取りでは `custom`・`0x46160A0D`）。ブレスに行の無い × とボールは既定の `system:finger`（Windows の標準の指さしの形 `HAND`）。窓の白地は矢印（`ARROW`） | `stage3-cursor.txt`・`target\pasta-verify\root\ghost\pasta-in-windows\shell\master\surfaces.txt` の `sakura.cursor` ブレス | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#mouseup*%2C%E5%BD%93%E3%81%9F%E3%82%8A%E5%88%A4%E5%AE%9AID%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D 「当たり判定の上に乗った時表示するカーソルファイルを設定。」（既定は `system:finger`）。食い違い: なし |
| × を赤くする・× とボールのクリック | `mouse-click-wheel-events` | 2026-10-23 08:06〜08:12 | `move`・`click` | `OnMouseEnter`／`OnMouseLeave`（`閉じる`）と `OnMouseClick`（`セキュリティボール`・`閉じる`）が届き、× の地が赤くなり、ボールのトークと終わりの台詞が出た | `stage3-pasta-log.txt`・`stage7-pasta-log.txt`・`stage3-x-hover-shot.png` | https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseEnter 「マウスが当たり判定に入った際に発生。」。食い違い: なし |
| 右クリックメニューの色と絵・着せ替えのメニュー | `popup-menu-shell-look` | 2026-10-23 08:09〜08:12 | `rightclick`・`click`・`dump_surface` | メニューはシェルの絵と色で普通の Windows のメニューに見えた。着せ替えのメニューから選ぶと付いて、もう一度選ぶと外れた。ぱすたは毎分 `OnSecondChange` で着せ替えを返し直すので、選んだ着せ替えは長くは残らない（areka で比べるときは、選んだ直後に撮る） | 段 6 の結果・`stage6-pasta-log.txt` | https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnSecondChange 「1秒ごとに発生。」。食い違い: なし |

`\1\v`（`stay-on-top-directives`）は通しの台本に出てこなかったので、見えた答えは無い。
