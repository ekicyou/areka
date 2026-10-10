# 実機確認の記録（タスク 7・要件 8.5）

**状態: 途中（下ごしらえまで済み・Claude Desktop の起動し直し待ち）**

## 確かめる物と机

| 項目 | 値 |
|---|---|
| 確かめた日 | 2026-10-10（下ごしらえ）・起動し直しの後の分は未実施 |
| 版 | areka 0.0.2・コミット `96f4393`・未コミットの変更 0（zip の `BUILD-INFO.txt`） |
| zip | `target\package\areka-0.0.2-x64.zip`（SHA256 `b7251e2ee930d06c9bb3906acb56fc19c06be150dc2a8d183434b138ad3f703c`） |
| 展開した場所 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-mcp-stdio-bridge-63f45d\target\t7\areka` |
| Claude Desktop | Store 版 `Claude_2.31226.1.0_x64`（下ごしらえの時点で起動中） |
| 机の状態（下ごしらえの時点） | 9801〜9830 で待ち受けているもの 0（SSP も areka も動いていない） |

## 段取り（Desktop の起動し直しを 1 回にした）

開発者の机では別のセッションが Desktop の中で動いていて、起動し直せるまで数時間かかることがある。そこで起動し直しを 1 回で済ませる順にした。

1. 下ごしらえ（起動し直しなし・済み）: zip を作る（`-Check` 付き）→ 展開 → areka を起こして help の設定例を取る → 展開した中継を 1 度手で areka に通して控えを作る → areka を止める。
2. 開発者: help の設定例を Desktop の設定に写す。SSP を起こす（9801 を SSP に取らせる）。**areka は止めたまま** Desktop を起動し直す。
3. 起動し直しの後（Desktop はもう落とさない）: 控えでツールが並ぶ（⑷ の前半）→ areka を起こすと呼べる（⑷ の後半・⑴・⑵・⑸）→ areka を止めて呼ぶと理由の読めるエラー（⑶ の前半）→ areka を起こし直すとそのまま呼べる（⑶ の後半）。

**タスクの文との違い**: ⑷ の控えを作ったのは「前の Desktop が起こした中継」でなく「下ごしらえで手で起こした同じ中継の exe」（同じ exe・同じ置き場・同じ 1 行 1 要求のやりとり）。

## 6 点の結果

| 点 | 中身 | 結果 |
|---|---|---|
| ⑴ | Desktop に areka のツールが並び、`get_status` など 1 本以上が呼べる | 未実施 |
| ⑵ | Desktop が話した版（中継の標準エラーの記録 `answered the first inquiry … requested=…` から） | 未実施 |
| ⑶ | areka を止めて呼ぶと理由の読めるエラーが出て、起動し直すと Desktop を起動し直さずに呼べる | 未実施 |
| ⑷ | areka を止めたまま Desktop を起動し直すと控えでツールが並び、その後 areka を起動すると呼べる | 未実施 |
| ⑸ | SSP が 9801 に居る机で `get_active_ghost_list` が areka のゴーストを返す | 未実施 |
| ⑹ | 配布スクリプトの `-Check` が緑 | **緑**（`tools/package.ps1 -Check`・コミット `96f4393`・全段 OK・記録の判定 7 行とも PASS・終了コード 0。出力は `target\t7\package-check.txt`） |

## 下ごしらえで見えたこと（Desktop を通さない確かめ）

- 展開した zip の `areka.exe` は、ほかに誰も居ない机で 9801 を取った。help（`http://127.0.0.1:9801/api/mcp/help`）の Claude Desktop の節の設定例は、展開した場所の `areka-mcp-bridge.exe` の絶対パスを `command` に書き、`env` は無い（既定の候補の番号なので要らない）。
- 展開した zip の中継を手で起こし（`AREKA_MCP_PORT=9801`・`AREKA_PROFILE_DIR` は展開した場所の `profile\areka`）、`initialize`（`2025-06-18`）・`notifications/initialized`・`tools/list`・`tools/call`（`get_status`）の 4 行を渡した。標準出力は 3 行（通知には何も書かない）: 名乗りは `areka-mcp-server` 0.0.2・版は `2025-06-18`、ツールは 11 本、`get_status` は `balloon(0=0/1=0)`。終了コード 0。
- 中継の標準エラーには `connected to areka port=9801` と `answered the first inquiry method="initialize" requested=2025-06-18 answered=2025-06-18` が残った。
- 控えは `target\t7\areka\profile\areka\mcp-bridge` に 2 つ出来た（`server-info.json` 625 バイト・`tools.json` 6,780 バイト＝ツール 11 本）。
- areka は窓を閉じる合図で自分で終わり、9801〜9830 で待ち受けるものは 0 に戻った。

## 完了のときの申し送り

- 署名の spec と winget の spec へ: 配布 zip の実行ファイルが 4 本になった（`areka-mcp-bridge.exe` が増えた）。
