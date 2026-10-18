# areka-P0-move-canon 実機の確かめ（タスク 6.2）

要件 6.1・6.2・9.4・9.5 の実機の証跡。emo2 の 2026-10-17 版で「エモの位置調整」と「シェル位置初期化」を選び、記録と窓の矩形から判定する。

## 手順

- 道具: `.kiro/specs/areka-P0-move-canon/real-machine-run.ps1`（PowerShell から `-Prepare` → `-Run Adjust -Tag <名>` → `-Run Reboot -Tag <名>`）。
- 検体: `vendors/sample_ghost/emo2.nar`（更新日 2026-10-17 20:30）を `nar-sample-path` で展開し、`target\move-canon-signoff\root` へ写した。ゴーストの雑談を止めるため、写しから `dispatch.pasta`・`hour.pasta` を外した。areka は debug、32bit の補助プロセスは i686（`014C` を確かめた）。
- 根・記録はすべて作業ツリーの `target\move-canon-signoff\` の下（要件 9.5）。ゴーストの根は絶対パスで渡した。書ける根（`%LOCALAPPDATA%\areka`）は子プロセスだけ `target\move-canon-signoff\<名>\localappdata` へ向けた。位置の記憶はゴーストの profile（`root\ghost\emo2\ghost\master\profile\areka\sylphya.toml`）に書かれる。
- 起こし方: `RUST_LOG=info,kanade=trace,areka::input_events=debug`（`move_applied`・`move_arg_substituted`・`move_frame_substituted`・`reset_window_pos`・`reset_pos_menu_outcome` はすべて info／warn なので出る）。`AREKA_NO_ALERT=1`、有界の自動終了は `AREKA_APP_SMOKE_EXIT_MS`（Adjust 90 秒・Reboot 25 秒）。
- 動く距離を作るため、`Adjust` の前に位置の記憶を消し、エモ（`\1`）だけを x=1000 へずらした記憶を置いた。むらさき（`\0`）は既定の位置で出る。
- 操作: 開発者の許可を得て、走行のあいだだけ SendInput を使った。むらさきをダブルクリックすると、エモのバルーンにメニューが出る。そこで「エモの位置調整」→「調整」を選んだ（`choice_selected` で確認）。次に、むらさきを右クリックして出る OS のメニューで「シェル位置初期化」を押した。押したのは、自分が起こした areka の窓とそのメニューだけ。
- 窓の矩形は、per-monitor DPI aware のスレッドで `GetWindowRect`（物理画素）と `GetDpiForWindow` を使って取った。

## 走行 1（実測の拡大率 200%）— 2026-10-18

- 拡大率: 全部の窓で `GetDpiForWindow` が **192（200%）**。areka の記録も `k_ratio=ScaleRatio { num: 2, den: 1 } k=2.0 author_dpi=96`。作業領域（物理）は 0,0〜2880,1704。
- 記録: `target\move-canon-signoff\scale1\run-Adjust.log`・`run-Adjust.steps.txt`・`run-Reboot.log`・`run-Reboot.steps.txt`。

### 「エモの位置調整」（要件 6.1・6.2・9.4）

```
event="move_applied" scope=1 base=Scope(0) base_offset=RefPoint { x: Left, y: Bottom } move_offset=RefPoint { x: Right, y: Bottom } anchor=Some(Bottom) base_left=2064 base_top=610 base_right=2828 base_bottom=1704 from_x=664 from_y=904 script_x=1392 script_y=904 to_x=1392 to_y=904
```

移動の後の窓の矩形（物理画素・左,上,右,下）:

| 窓 | 矩形 |
|---|---|
| エモ（`\1`）のキャラクター | 1392,904,2064,1704 |
| むらさき（`\0`）のキャラクター | 2064,610,2828,1704 |

| 判定の項目 | `\1` | `\0` | 結果 |
|---|---|---|---|
| `\1` の右端＝`\0` の左端 | 右端 2064（`to_x` 1392＋幅 672） | 左端 `base_left`=2064 | **一致** |
| `\1` の下端＝`\0` の下端 | 下端 1704（`to_y` 904＋高さ 800） | 下端 `base_bottom`=1704 | **一致** |

- エモは x=664 から 1392 へ 728 画素動いた（`from_x`→`to_x`）。窓の実際の矩形は `to_*` と同じ。
- この走行の記録では、`move_arg_substituted` も `move_frame_substituted` も **0 件**。`move_skipped` も 0 件、ERROR の行も 0 件。
- **判定: 200% で要件 6.1 は成り立つ。**

### 「シェル位置初期化」と起こし直し（要件 8.3〜8.6・8.8・9.2）

```
event="reset_pos_menu_outcome" outcome="no_reply"
event="reset_window_pos" scope=0 from_x=2064 from_y=610 to_x=2116 to_y=610
event="reset_window_pos" scope=1 from_x=1392 from_y=904 to_x=1444 to_y=904
並べ終えた時点の保存: 書いた scope=0 char_x=2116 char_y=610 saved_x=2498 saved_y=610 char_w=764 anchor=Bottom
並べ終えた時点の保存: 書いた scope=1 char_x=1444 char_y=904 saved_x=1780 saved_y=904 char_w=672 anchor=Bottom
```

- メニューの項目は既定の名前「シェル位置初期化」で出た（ゴーストは `switchmovetodefaultpositionbutton.caption` に名前を返さなかった）。並びはバルーンの次の 4 番目。
- `OnResetWindowPos` は送られたが、emo2 は返事をしなかった（`no_reply`）。そのため areka が並べ直した（要件 8.5）。
- 並べ直しの後の窓の矩形: むらさき 2116,610,2880,1704（右端が作業領域の右端 2880）、エモ 1444,904,2116,1704。並べ直した後も、`\1` の右端＝`\0` の左端＝2116、下端はどちらも 1704。
- 起こし直し（`-Run Reboot`・記憶は消さない）: 記録は `merge_scope restore scope=0 … saved_win_x=Some(2498) … char_x=2064 char_y=330 char_w=868` と `scope=1 … saved_win_x=Some(1780) … char_x=1444 char_y=904`。起動直後の面の大きさが入れ替わった後、窓の矩形は次のとおり。

| 窓 | 並べ直しの直後 | 起こし直しの後 | 結果 |
|---|---|---|---|
| むらさき（`\0`）のキャラクター | 2116,610,2880,1704 | 2116,610,2880,1704 | 同じ |
| エモ（`\1`）のキャラクター | 1444,904,2116,1704 | 1444,904,2116,1704 | 同じ |
| むらさきのバルーン | 1848,352,2648,800 | 1848,352,2648,800 | 同じ |
| エモのバルーン | 1736,754,2312,1160 | 1736,754,2312,1160 | 同じ |

- **判定: 並べ直した位置に、次の起動でも出る（要件 8.8）。バルーンもキャラクターに付いて動いた（要件 8.9）。**
- 参考: 位置の記憶が無い起動では、むらさきの既定の位置を最初の面の幅 868 で解く（x=2012）。面が幅 764 に替わった後は x=2064 に残る。並べ直しは今の幅 764 で解くので x=2116（右端 2880）になる。今の大きさで解くという要件 8.6 のとおり。

### 走行 1 で起こした areka

PID 34432・36936・27492・33860・35256・33444。どれも有界の自動終了で、終了コード 0 で終わった（`target\move-canon-signoff\runs.txt`）。強制終了は 0 件、走行の後に残った areka は無い。

- 34432 と 33860: 選択肢の行を押した記録が出なかった（押下・離し・ポインタの退避が同じフレームに重なった）。道具を「先に行へ運んで強調させる」「押下と離しを 80ms 空ける」に直した。
- 36936: 右クリックがメニューを開かなかった（先にポインタを乗せていなかった）。
- 27492: 操作は通ったが、前の走行の位置の記憶が残っていた。エモがはじめから正しい位置にいて、動く距離が 0 だった。
- 判定に使ったのは 35256（Adjust）と 33444（Reboot）。

## 走行 2（実測の拡大率 100%）— 2026-10-18

- 拡大率: 開発者が主モニタを 100% へ切り替えた後に回した。全部の窓で `GetDpiForWindow` が **96（100%）**。areka の記録も全部 `k_ratio=ScaleRatio { num: 1, den: 1 } k=1.0 author_dpi=96`。作業領域（物理）は 0,0〜2880,1752。
- 手順は走行 1 と同じ。エモの記憶は x=300 にずらした（`-SeedEmoX 300`）。100% では窓が小さく、x=1000 では動く距離が短いため。
- 記録: `target\move-canon-signoff\scale2\run-Adjust.log`・`run-Adjust.steps.txt`・`run-Reboot.log`・`run-Reboot.steps.txt`。

### 「エモの位置調整」（要件 6.1・6.2・9.4）

```
event="move_applied" scope=1 base=Scope(0) base_offset=RefPoint { x: Left, y: Bottom } move_offset=RefPoint { x: Right, y: Bottom } anchor=Some(Bottom) base_left=2472 base_top=1205 base_right=2854 base_bottom=1752 from_x=132 from_y=1352 script_x=2136 script_y=1352 to_x=2136 to_y=1352
```

移動の後の窓の矩形（物理画素・左,上,右,下）:

| 窓 | 矩形 |
|---|---|
| エモ（`\1`）のキャラクター | 2136,1352,2472,1752 |
| むらさき（`\0`）のキャラクター | 2472,1205,2854,1752 |

| 判定の項目 | `\1` | `\0` | 結果 |
|---|---|---|---|
| `\1` の右端＝`\0` の左端 | 右端 2472（`to_x` 2136＋幅 336） | 左端 `base_left`=2472 | **一致** |
| `\1` の下端＝`\0` の下端 | 下端 1752（`to_y` 1352＋高さ 400） | 下端 `base_bottom`=1752 | **一致** |

- エモは x=132 から 2136 へ 2004 画素動いた（`from_x`→`to_x`）。窓の実際の矩形は `to_*` と同じ。
- この走行の記録では、`move_arg_substituted` も `move_frame_substituted` も **0 件**。`move_skipped` も 0 件、ERROR の行も 0 件。
- **判定: 100% で要件 6.1 は成り立つ。**

### 「シェル位置初期化」と起こし直し（要件 8.3〜8.6・8.8・9.2）

```
event="reset_pos_menu_outcome" outcome="no_reply"
event="reset_window_pos" scope=0 from_x=2472 from_y=1205 to_x=2498 to_y=1205
event="reset_window_pos" scope=1 from_x=2136 from_y=1352 to_x=2162 to_y=1352
並べ終えた時点の保存: 書いた scope=0 char_x=2498 char_y=1205 saved_x=2689 saved_y=1205 char_w=382 anchor=Bottom
並べ終えた時点の保存: 書いた scope=1 char_x=2162 char_y=1352 saved_x=2330 saved_y=1352 char_w=336 anchor=Bottom
```

- 項目は走行 1 と同じく既定の名前「シェル位置初期化」で、4 番目に出た。emo2 は `OnResetWindowPos` に返事をせず（`no_reply`）、areka が並べ直した。
- 並べ直した後も、`\1` の右端＝`\0` の左端＝2498、下端はどちらも 1752。むらさきの右端は作業領域の右端 2880。
- 起こし直し（`-Run Reboot`）の記録は `merge_scope restore scope=0 … saved_win_x=Some(2689)` と `scope=1 … saved_win_x=Some(2330) … char_x=2162 char_y=1352`。窓の矩形は次のとおり。

| 窓 | 並べ直しの直後 | 起こし直しの後 | 結果 |
|---|---|---|---|
| むらさき（`\0`）のキャラクター | 2498,1205,2880,1752 | 2498,1205,2880,1752 | 同じ |
| エモ（`\1`）のキャラクター | 2162,1352,2498,1752 | 2162,1352,2498,1752 | 同じ |
| むらさきのバルーン | 2364,1076,2764,1300 | 2364,1076,2764,1300 | 同じ |
| エモのバルーン | 2308,1277,2596,1480 | 2308,1277,2596,1480 | 同じ |

- **判定: 100% でも、並べ直した位置に次の起動で出る。バルーンもキャラクターに付いて動いた。**

### 走行 2 で起こした areka

PID 33860（Adjust・OS が走行 1 の番号を使い回した）と 17256（Reboot）。どちらも有界の自動終了で、終了コード 0 で終わった。強制終了は 0 件、走行の後に残った areka は無い。操作の打ち直しは無かった。

## 総合の判定

| 要件 | 判定 | 根拠 |
|---|---|---|
| 6.1 エモの右下をむらさきの左下にそろえる | **成り立つ** | 200% と 100% のどちらも、`move_applied`（scope=1・base=Scope(0)）の `to_*` と窓の矩形から「`\1` の右端＝`\0` の左端」「`\1` の下端＝`\0` の下端」が一致した。エモはどちらの走行でもずれた位置から動いた（728 画素・2004 画素）。 |
| 6.2 100% と 200% の両方で成り立つ | **成り立つ** | 実測の拡大率は走行 1 が 200%（DPI 192・k=2.0）、走行 2 が 100%（DPI 96・k=1.0）。 |
| 9.4 実機の証跡を残す | **満たす** | この文書と `target\move-canon-signoff\scale1`・`scale2` の記録。道具は `real-machine-run.ps1`。 |
| 9.5 根と検体は `target\` の下だけ | **満たす** | 根・検体・書ける根・位置の記憶・一時フォルダ・記録は、どれも作業ツリーの `target\move-canon-signoff\` の下。 |

あわせて、2 つの拡大率の両方で次も確かめた。

- 「シェル位置初期化」の項目が出る（8.1・8.2）。
- `OnResetWindowPos` を送り、返事が無いときに並べ直す（8.3・8.5）。
- 並べ直しを記録して記憶に書く（9.2）。
- 起こし直すと同じ位置に出る（8.8）。
- バルーンが付いて動く（8.9）。

## 申し送り

なし（走行 1・2 とも手間の大きい壁に当たらなかった）。
