# 実機の記録: areka-P0-char-position-save-on-exit（タスク 6.2・要件 5.5）

> 拡大率（DPI）は変えずに確かめる。拡大率を変えたあとの詰め直しが 2 回目の起動から効かなくなる後退は、この spec では受け入れて `areka-P0-dpi-realign-remembered-chain` として起票済み（design.md「知っている後退」）。

## 1. 確かめるゴーストを決めた経緯（2026-10-07）

最初は emo2 で R1 を回した。並べ終えた時点で本体（scope 0）は書かれたが、相方（scope 1）は「並べ終える前に動かされたので書かない」になった。細かい記録を開けて回し直すと、相方を動かしたのは emo2 自身の起動の台詞だった。

- emo2 の `ghost\master\dic\boot.pasta` は、起動のたびに `\1\![move,-353,,,0,base,base]` を出す（相方を本体から左へ 353 の所へ動かす SHIORI の移動の指示）。
- 記録: `apply_move_directive: move 適用完了 scope=1 base_scope=0 from_x=1340 from_y=904 to_x=1404 to_y=904`（`route=MoveCue`）の直後に、`並べ終えた時点の保存: 並べ終える前に動かされたので書かない scope=1 current_x=1404 default_x=1340`。

要件 1.7（SHIORI の移動の指示で動いた窓は覚えない）どおりの動きである。emo2 の相方は、記憶があってもなくても起動のたびに台本が本体の左へ置き直すので、要件 2.1 の「相方が前回の位置に立つ」は emo2 では確かめられない。開発者の裁定は「クローディアで」（要件 5.5 を改めた）。クローディアの台本は移動の指示を出さない（`ghost\master` を検索して 0 件）。もとの症状（完了 spec `areka-P0-mouse-drag-events` の実機 R7）もクローディアで出ていた。

emo2 の 1 回目の記録（`target\cpsoe\keep\run-R1-first.log`）は、要件 1.7 の実機の証跡として使う: 並べ終えた時点の「書いた」は scope 0 の 1 行・ドラッグの確定の保存は scope 0 の 1 行・記憶は `[window.0]` だけ・ERROR 0 行。

## 2. 走行の条件

| 項目 | 値 |
|---|---|
| 版 | main（`067375e3`）を取り込んだ後（`4e432463` 以降）から組んだ debug 版 `areka.exe`（x64）と `shiori-host32-helper.exe`（i686） |
| 根 | クローディアは `<ワークツリー>\target\cpsoc`、emo2 は `<ワークツリー>\target\cpsoe`（どちらも `real-machine-run.ps1 -Sample <検体> -Prepare` で `nar-sample-path` の展開から写したもの） |
| 記憶 | R1 は起動の前に `profile` とゴーストの `profile\areka` を消す。R2・R3 は前の回の記憶を残す |
| `RUST_LOG` | `info,areka::persist::save=info,areka::persist::restore=info` |
| 安全弁 | `AREKA_APP_SMOKE_EXIT_MS=900000` |
| 起動 | `real-machine-run.ps1 -Sample claudia -Run <項目>`。記録は根の `run-<項目>.log`、記憶の写しは `before-<項目>.toml`・`after-<項目>.toml` |
| 操作 | 開発者（1 項目ずつ GO を待って起動） |

## 3. 手順（クローディア）

| 項目 | 開発者の操作 | 機械の判定（`-Check`） | 目で見ること |
|---|---|---|---|
| R1 | 起動して落ち着いたら、本体（`\0`）だけをドラッグして離す → メニューの「終了」 | 並べ終えた時点の「書いた」が scope 0・1 の 2 行／ドラッグの確定の保存は scope 0 だけ／記憶に `[window.0]`・`[window.1]` | 相方が本体の左隣に並ぶ |
| R2 | 起動 → 相方の位置を見る → 「終了」 | 並べ直し（`chain_finalize: 実表示寸で連鎖を再解決`）0 行／「書いた」0 行・「記憶があるので書かない」が scope 0・1／記憶が起動の前後で同じ | 本体は R1 で動かした位置、相方は R1 で閉じたときの位置（本体の隣へ並べ直されない） |
| R3 | R2 と同じ | R2 と同じ | 並びが R2 と変わらない |

## 4. 結果（2026-10-07・クローディア・3 回とも合格）

版は `f977964b`（main を取り込んだ `4e432463` に文書の直しだけを足したもの）から組んだ debug 版。`runs.txt`（逐語）:

```
R1 pid=23476 start=2026-10-07T23:14:23.0738240+09:00 exit_ms=900000
R1 exit=0 end=2026-10-07T23:14:54.0870247+09:00
R2 pid=29284 start=2026-10-07T23:18:43.9910347+09:00 exit_ms=900000
R2 exit=0 end=2026-10-07T23:19:01.5847793+09:00
R3 pid=32748 start=2026-10-07T23:19:54.5668578+09:00 exit_ms=900000
R3 exit=0 end=2026-10-07T23:20:06.7376266+09:00
```

3 回とも開発者がメニューの「終了」で閉じた（終了コード 0・安全弁は発火していない）。

### R1（記憶なし → 本体だけドラッグ → 終了）: 合格

- 並べ終えた時点で 2 行が書かれた（要件 1.1・4.3）:
  - `並べ終えた時点の保存: 書いた scope=0 char_x=2214 char_y=704 saved_x=2214 saved_y=704 char_w=666 anchor=Free`
  - `並べ終えた時点の保存: 書いた scope=1 char_x=1548 char_y=704 saved_x=1548 saved_y=704 char_w=666 anchor=Free`
- ドラッグの確定は本体だけ: `char DragEnd 保存 scope=0 char_x=2157 char_y=466 saved_x=2157 saved_y=466`。
- 終了後の記憶（`after-R1.toml`）は `[window.0] x="2157" y="466"`・`[window.1] x="1548" y="704"`。ERROR 0 行。

### R2（記憶あり → 起動 → 終了）: 合格

- 戻した位置: `merge_scope restore scope=0 … saved_win_x=Some(2157) saved_win_y=Some(466) … char_x=2157 char_y=466`、`merge_scope restore scope=1 … saved_win_x=Some(1548) saved_win_y=Some(704) default_char_x=1548 default_char_y=704 char_x=1548 char_y=704`。本体はドラッグした位置、相方は R1 で閉じたときの位置（要件 2.1）。
- 並べ直しの行（`chain_finalize: 実表示寸で連鎖を再解決`）は 0 行。並べ終えた時点は `記憶に位置があるので書かない` が scope 0・1（要件 1.2）。
- 相方の記憶の値は既定の位置（1548, 704）とたまたま同じだったが、「記憶あり」として並べ直されなかった（要件 2.6 の場面を実機で踏んだ）。
- 記憶は起動の前後でバイトまで同じ（`before-R2.toml` ⇔ `after-R2.toml`）。ERROR 0 行。

### R3（R2 と同じ）: 合格

- R2 と同じ行が出て、並べ直し 0 行・「書いた」0 行・記憶は起動の前後で同じ。`after-R1.toml` と `after-R3.toml` もバイトまで同じ（並びが変わらない・要件 2.3 の形）。ERROR 0 行。

### emo2（要件 1.7）

1 章のとおり、emo2 の起動の台詞の `\1\![move,-353,,,0,base,base]` で動いた相方は、並べ終えた時点で書かれなかった（`並べ終える前に動かされたので書かない scope=1 current_x=1404 current_y=904 default_x=1340 default_y=904`）。本体は並べ終えた時点とドラッグの確定で書かれた。ERROR 0 行。

### 拡大率

3 回とも拡大率は変えていない。拡大率を変えたあとの詰め直しの後退は `areka-P0-dpi-realign-remembered-chain` の持ち分。
