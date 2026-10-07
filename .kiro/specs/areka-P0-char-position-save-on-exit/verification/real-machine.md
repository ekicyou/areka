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

## 4. 結果

（開発者の確認のあとに記入）
