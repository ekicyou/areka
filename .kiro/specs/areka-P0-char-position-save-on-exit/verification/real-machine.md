# 実機の記録: areka-P0-char-position-save-on-exit（タスク 6.2・要件 5.5）

> 拡大率（DPI）は変えずに確かめる。拡大率を変えたあとの詰め直しが 2 回目の起動から効かなくなる後退は、この spec では受け入れて `areka-P0-dpi-realign-remembered-chain` として起票済み（design.md「知っている後退」）。

## 1. 走行の条件

| 項目 | 値 |
|---|---|
| 版 | （記入）から組んだ debug 版 `areka.exe`（x64）と `shiori-host32-helper.exe`（i686） |
| 根 | `<ワークツリー>\target\cpsoe`（`real-machine-run.ps1 -Prepare` で `nar-sample-path emo2` の展開から写したもの） |
| 検体 | emo2（適合ゴースト） |
| 記憶 | R1 は起動の前に `profile` とゴーストの `profile\areka` を消す。R2・R3 は前の回の記憶を残す |
| `RUST_LOG` | `info,areka::persist::save=info,areka::persist::restore=info` |
| 安全弁 | `AREKA_APP_SMOKE_EXIT_MS=900000` |
| 起動 | `.kiro/specs/areka-P0-char-position-save-on-exit/real-machine-run.ps1 -Run <項目>`。記録は `target\cpsoe\run-<項目>.log`、記憶の写しは `before-<項目>.toml`・`after-<項目>.toml` |
| 操作 | 開発者（1 項目ずつ GO を待って起動） |

## 2. 手順

| 項目 | 開発者の操作 | 機械の判定（`-Check`） | 目で見ること |
|---|---|---|---|
| R1 | 起動して落ち着いたら、本体（`\0`）だけをドラッグして離す → メニューの「終了」 | 並べ終えた時点の「書いた」が scope 0・1 の 2 行／ドラッグの確定の保存は scope 0 だけ／記憶に `[window.0]`・`[window.1]` | 相方が本体の左隣に並ぶ |
| R2 | 起動 → 相方の位置を見る → 「終了」 | 並べ直し（`chain_finalize: 実表示寸で連鎖を再解決`）0 行／「書いた」0 行・「記憶があるので書かない」が scope 0・1／記憶が起動の前後で同じ | 本体は R1 で動かした位置、相方は R1 で閉じたときの位置（本体の隣へ並べ直されない） |
| R3 | R2 と同じ | R2 と同じ | 並びが R2 と変わらない |

## 3. 結果

（開発者の確認のあとに記入）
