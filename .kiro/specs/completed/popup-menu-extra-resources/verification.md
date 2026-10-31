# popup-menu-extra-resources 検証の記録

## 4.1 3 クレートのテストと変えないファイル（要件 5.5・6.2・3.3）

2026-11-01・`cargo test -p areka` の実機の結合テストは排他テストの机を持って回した。

| 命令 | 結果 |
|---|---|
| `cargo test -p areka-kanade` | 緑・706・86・0 通過・0 失敗 |
| `cargo test -p ukadoc-survey` | 緑・601・0・6・113・5 通過・0 失敗 |
| `cargo test -p areka` | 1 回目は 4161 通過・1 失敗（`smoke_boot_loop_exit` の 1 本が i686 の `shiori_loadu.dll` 未ビルドで `real-run-kit` の `i686.rs:77` で落ちた・本 spec の差分とは無関係）。`cargo build -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` の後、赤の実行ファイルだけ回し直して 5 通過・0 失敗 |
| `tools/test-all.ps1`（最終検証・commit `86ed7ceb`） | 全段緑（fmt・x64 ワークスペース・i686・エンコード・ロードマップの検査ほか） |

`git diff main...HEAD --stat` に `crates/areka/src/menu/plan.rs`・`mod.rs`・`win32.rs`・`trigger.rs`・`update_frame.rs`、kanade の `actor_resources.rs`・`actor_resources_tests.rs`・`actor.rs`・`shiori/real.rs`、`shiori-host32-host` の `shiori2.rs` は現れない（grep で 0 件）。

## 4.2 「．さくら」を areka と SSP で開いた右クリックメニュー（要件 1.6・3.4・5.6〜5.8）

2026-11-01（日本時間 01:54〜02:00）・排他テストの机を持って 1 回。記録の時刻は UTC（areka の記録の書き方のまま）。

### 置き場と命令（areka の側）

根・記憶・一時フォルダ・記録・撮った物は、すべてこのワークツリーの `target\pmer-4-2\` の下（`root\ghost\dot_sakura`・`root\ghost\emo2`・`root\balloon\areka-balloon`・`root\balloon\emo2-kakukaku`・空の `profile\`・`tmp\`・`logs\`・`shots\`）。検体は `nar-sample-path` で開いた `dot_sakura`・`emo2`・`areka-balloon` の写し。ビルドは `cargo build -p areka`・`cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`・`tools/sync-i686-helper.ps1`（`helper is i686`）。起こし方は完了 `shiori2-protocol` の検証の記録 5.1 と同じ形で、替えたのは次の所だけ。

- `--ghost "<w>\target\pmer-4-2\root\ghost\dot_sakura"`・`AREKA_ROOT`／`AREKA_PROFILE_DIR`／`TMP`／`TEMP` を `target\pmer-4-2` の下・`AREKA_APP_SMOKE_EXIT_MS=1800000`
- `RUST_LOG=info,ghost-boot=info,shiori2=debug,areka::menu=trace,areka_kanade::resource=trace,kanade=trace`
- 右クリックは `SetCursorPos` で狙いの点へ置いてから `SendInput` 系（`mouse_event`）で右ボタンの押し・放しを送った。押す直前の時刻（UTC）を `logs\run1.clicks.txt` に書いた。`tools/mouse-input.ps1` はこの機械（主画面 2880×1800・左に 3413×2133 の 2 枚目）で座標が毎回 1 画素ずれて自分で断るため使わなかった（下の「範囲の外で見えたこと」）
- 終わりは MCP の `raise_event OnClose`。areka は外から止めずに終了コード 0 で終わった

記録: `target\pmer-4-2\logs\run1.stdout.log`・`run1.clicks.txt`・`run1.exitcode.txt`（`areka exit=0`）。撮った物: `target\pmer-4-2\shots\run1\1-menu.png`〜`4-menu.png`。

### areka の項目名（要件 5.6）

4 回とも同じ並び（`menu_shown` の `items=17`・区切りを含む）:

ゴースト ▸／シェル ▸／バルーン ▸／シェル位置初期化／バルーン位置初期化／―／**気まぐれ更新(U)**／インストール…／置き場を追加…（灰）／置き場を先頭へ ▸（灰）／置き場を外す ▸（灰）／―／説明書／―／終了

- 「ネットワーク更新」の項目名は「気まぐれ更新(U)」。「．さくら」の辞書 `dict-keeps.txt` 761 行の `resource.updatebuttoncaption`（既定の気分では `dot/homeUrl.txt` 24 行の `UpdateCaption : "気まぐれ更新(&U)"`）の言葉と一致する。`&U` は項目の頭文字の印として出ている。
- 1 回の表示の問い合わせの並び（kanade の TRACE `shiori_request`）: `sakura.popupmenu.visible` → `ghostrootbutton.caption` → `shellrootbutton.caption` → `balloonrootbutton.caption` → `switchmovetodefaultpositionbutton.caption` → `resetballoonpositionbutton.caption` → **`updatebuttoncaption` → `updatebutton.caption`** → `ghostinstallbutton.caption` → `readmebutton.caption` → `closebutton.caption`。古い名前は新しい名前の直前に 1 回だけ（要件 1.1・1.7）。
- 空・値なしの記録（`menu_resource_empty`・`debug`・1 行）の `ids` は 9 名で、`updatebutton.caption` を含み `updatebuttoncaption` を含まない。新しい名前が引けず古い名前が値を返したので、古い名前の値が項目名になった（要件 1.3）。
- 他の項目名は今までどおり areka の既定名（「．さくら」はそれらの資源を持たない）。

### 所要（要件 1.6・Performance）

照会を送る時に記録の行は無いので、右クリックを送った時刻から `menu_shown`（`info`）の時刻までを所要とした（照会の往復と写し・メニューの組み立てを含む上からの値）。

| 回 | 右クリック（UTC） | `menu_shown`（UTC） | 所要 | 上限 1,000 ms までの余裕 |
|---|---|---|---|---|
| 1 | 16:56:24.890 | 16:56:24.957 | 67 ms | 933 ms |
| 2 | 16:56:38.983 | 16:56:39.022 | 39 ms | 961 ms |
| 3 | 16:56:44.782 | 16:56:44.815 | 33 ms | 967 ms |
| 4 | 16:56:51.034 | 16:56:51.072 | 38 ms | 962 ms |

- 2 つの名前の往復は 1 回目で 0.6 ms 差（`updatebuttoncaption` 16:56:24.946988 → `updatebutton.caption` 16:56:24.947621）。どの回も 1 ms 未満。
- 時間切れ（`menu_resource_query_unanswered`）は 0 件（記録全体）。
- 右クリックが台詞の最中（`talking`）にも重なったが、メニューは毎回開いた。

### SSP の側（要件 5.6）

- 使っているセッション（ぱすた開発）に「急がなくてよい・切れ目で」を先頭にして断り、「どうぞ・今は SSP で何も測っていない」の答えを得てから行った。
- 着手の時点で SSP は止まっていた（`ssp.exe` のプロセス無し・9801 は areka が使用）。areka を閉じた後に SSP を起こし、起動したぱすたさん（`窓際のぱすたさんDEBUG`）を降ろさず、SSP の MCP の `sakurascript` で `\![call,ghost,．さくら]\e` を送って横に呼び出した。
- 呼ぶ前に `C:\wintools\ssp\ghost\dot_sakura\ghost\master\ssp_shiori_log.txt` の大きさ 166,364 バイトを控え、済んだ後に 309,189 バイトまでを読んだ（写し: `target\pmer-4-2\logs\ssp_shiori_log.tail.txt`）。
- 撮った物: `target\pmer-4-2\shots\ssp\0-call.png`・`1-menu.png`。

SSP の項目名（上から）:

さくらのお勧め(R) ▸／ポータルサイト(P) ▸／**気まぐれ更新(U)**　Ctrl+U／―／燃やすゾ！／―／RSS/ヘッドライン(H) ▸／プラグイン(P) ▸／メールチェック(M) ▸／シェル倍率(E) ▸／バルーン倍率(D) ▸／機能(F) ▸／設定(O) ▸／―／ゴースト切り替え(G) ▸／他のゴーストを呼ぶ（Z）▸／シェル(S) ▸／着せ替え(C) ▸（灰）／バルーン(B) ▸／最近使ったもの(Y) ▸／―／情報(A) ▸／Language ▸／アイコン化(I)　Ctrl+I／―／ゴースト本体移動／―／終了(W)　Ctrl+W／全て終了(X)　Ctrl+Q

- 「ネットワーク更新」の項目名は areka と同じ「気まぐれ更新(U)」。
- SSP の問い合わせの並び（SHIORI/2.x の相手・`GET String SHIORI/2.5`）: 起動の資源（`KAWARI.kdt`・`version`・`name`・`craftman`・メニューの色・`username` …）の後、`sakura.recommendsites`・`sakura.portalsites`・`kero.recommendsites`・`sakura.popupmenu.visible`・`sakura.popupmenu.type`・`recommendrootbutton.visible`・`sakura.recommendbuttoncaption`・`portalrootbutton.visible`・`sakura.portalbuttoncaption`・`homeurl`・**`updatebutton.visible`・`updatebuttoncaption`**・`readmebutton.visible`・`readmebuttoncaption`・`readmebutton.caption`・`vanishbuttonvisible`・`vanishbuttoncaption`・… と続き、以降は各項目の `*.visible`・`*.caption` の組（区間の `ID:` は計 168）。
- **SSP は SHIORI/2.x の「．さくら」に `updatebuttoncaption` を問い、`updatebutton.caption` は 1 度も問わなかった**（測 1 の 2.x 版）。`readmebutton` は古い名前と新しい名前の両方を問う。areka は新しい名前を先に引き、引けないときだけ古い名前に落ちるので、「．さくら」での項目名は同じになる。
- 済んだ後: 「．さくら」へ `\-` を送って閉じ、`get_active_ghost_list` が `窓際のぱすたさんDEBUG` だけを答えることを確かめた。SSP は着手前に止まっていたので、ぱすたさんにも `\-` を送って SSP を止め、元の状態に戻した（ぱすたのデバッグの待受 9276 も空いていることを確かめた）。SSP の外の実物（`C:\wintools\ssp\ghost` の中身）には何も足していない。

### 既知の差（要件 5.7）

- SSP にあって areka に無い項目のうち、本 spec の ⑵（windows-reactor 待ち）: 「燃やすゾ！」（消滅通告・`vanishbuttoncaption`／`vanishbuttonvisible`）・「さくらのお勧め」（`sakura.recommendbuttoncaption`／`sakura.recommendsites`）・「ポータルサイト」（`sakura.portalbuttoncaption`／`sakura.portalsites`）。直さない。
- それ以外の SSP の項目（RSS・プラグイン・倍率・設定・着せ替え・履歴・情報・アイコン化など）は SSP の機能の項目で、本 spec の範囲の外。
- 項目の並びと「ゴースト ▸」などの既定名は areka のメニューの今の作り（`menu/plan.rs`）のまま。本 spec は変えていない（要件 3.3）。

### 範囲の外で見えたこと（完了の棚卸へ）

- `tools/mouse-input.ps1` が、この機械（主画面 2880×1800・左に負の座標の 2 枚目）で、送ったカーソルの位置が狙いより毎回 1 画素小さくなり（例: 狙い 2640,1460 → 2639,1459）、自分で「input refused」として止まる。正規化の丸め（`ceil(p*65536/size)`）がこの構成の Windows の戻し方と合っていない見込み。本 spec では `SetCursorPos` で代えた。
- `cargo test -p areka` の実機の結合テストは、i686 の `shiori-host32-helper` に加えて `shiori-host32-testdll-loadu` も先に建てておかないと `real-run-kit` の `i686.rs:77` で落ちる（4.1 の Implementation Notes）。
