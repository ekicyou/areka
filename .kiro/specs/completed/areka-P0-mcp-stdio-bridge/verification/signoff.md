# 実機確認の記録（タスク 7・要件 8.5）

**状態: ⑴〜⑹ と、後から足した ⑺（areka も控えも無い机）とも合格（2026-10-11）**

## 確かめた物と机

| 項目 | 値 |
|---|---|
| 確かめた日 | 2026-10-10（下ごしらえ）・2026-10-11 0:44〜0:49（Desktop を起動し直した後）・2026-10-11 8:13（チャット欄） |
| 版 | areka 0.0.2・コミット `96f4393`・未コミットの変更 0（zip の `BUILD-INFO.txt`） |
| zip | `target\package\areka-0.0.2-x64.zip`（SHA256 `b7251e2ee930d06c9bb3906acb56fc19c06be150dc2a8d183434b138ad3f703c`） |
| 展開した場所 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-mcp-stdio-bridge-63f45d\target\t7\areka` |
| Claude Desktop | Store 版 `Claude_2.31226.1.0_x64`。本体を落として起動し直した時刻は 2026-10-11 0:44:36 |
| Desktop の設定 | `claude_desktop_config.json` の `mcpServers` に `areka` を足した（開発者の手・`command` は展開した場所の `areka-mcp-bridge.exe`・`env` なし）。ほかの項目（`gtd`・`ssp`・`github`）はそのまま |
| SSP | 開発者が起こした（0:45:16）。9801 と 9821 の両方で待ち受けていた。エージェントは SSP とその中継・控えに触っていない |
| 呼び出した口 | Desktop の中で動いている Claude Code のセッションに並んだ `areka` のツール（中継の親プロセスは Desktop 本体 `Claude.exe`）。記録は Desktop の `mcp-server-areka.log`（`%LOCALAPPDATA%\Claude\Logs`）から読んだ。時刻は記録の UTC を日本時間に直してある |

## 段取り（Desktop の起動し直しを 1 回にした）

開発者の机では別のセッションが Desktop の中で動いていて、起動し直せるまで時間がかかる。そこで起動し直しを 1 回で済ませる順にした。

1. 下ごしらえ（起動し直しなし）: zip を作る（`-Check` 付き）→ 展開 → areka を起こして help の設定例を取る → 展開した中継を 1 度手で areka に通して控えを作る → areka を止める。
2. 開発者: help の設定例を Desktop の設定に写し、**areka は止めたまま** Desktop を本体ごと落として起動し直す。SSP を起こす。
3. 起動し直しの後（Desktop はもう落とさない）: 控えでツールが並ぶ → areka を起こすと呼べる → areka を止めて呼ぶと理由の読めるエラー → areka を起こし直すとそのまま呼べる。

**タスクの文との違い**: ⑷ の控えを作ったのは「前の Desktop が起こした中継」でなく「下ごしらえで手で起こした同じ中継の exe」（同じ exe・同じ置き場・同じ 1 行 1 要求のやりとり）。

**1 度目の起動し直しは空振りだった**: 窓を閉じて開き直しただけでは本体がタスクバーに残り、設定は読み直されなかった（中継は起きず、`mcp-server-areka.log` も出来なかった）。本体ごと落として起動し直して初めて中継が起きた。

## 6 点の結果

| 点 | 中身 | 結果 |
|---|---|---|
| ⑴ | Desktop に areka のツールが並び、`get_status` など 1 本以上が呼べる | **合格**。11 本が並び、`get_status` は `balloon(0=0/1=0)`、`get_active_ghost_list` は `えも？？` を返した（Code のセッションからと、Desktop の普通のチャット欄からの両方で同じ結果） |
| ⑵ | Desktop が話した版 | **`2025-11-25`**（中継の記録 `answered the first inquiry method="initialize" requested=2025-11-25 answered=2025-11-25`） |
| ⑶ | areka を止めて呼ぶと理由の読めるエラーが出て、起動し直すと Desktop を起動し直さずに呼べる | **合格**。止めて呼ぶと下の文のエラー。起こし直した後の `get_status` と `get_active_ghost_list` は成功。Desktop の本体は 0:44:36 に起きたものがそのまま |
| ⑷ | areka を止めたまま Desktop を起動し直すと控えでツールが並び、その後 areka を起動すると呼べる | **合格**。areka が居ない間に `initialize` と `tools/list` へ控えで答え（記録 `areka not found; answered from the cache`）、ツール 11 本が並んだ。areka を起こした後の呼び出しは成功 |
| ⑸ | SSP が 9801 に居る机で `get_active_ghost_list` が areka のゴーストを返す | **合格**。SSP が 9801 と 9821 の両方を持つ机で、areka は 9802 を取り、中継は 9801 と 9821 を飛ばして 9802 につないだ。返ったのは zip に入っている既定のゴースト `えも？？` |
| ⑹ | 配布スクリプトの `-Check` が緑 | **緑**（`tools/package.ps1 -Check`・コミット `96f4393`・全段 OK・記録の判定 7 行とも PASS・終了コード 0。出力は `target\t7\package-check.txt`） |

### areka が居ないときに返ったエラーの文（⑶・2 回とも同じ）

```
Server not available: areka-mcp-server not found on 127.0.0.1 ports 9801, 9821, 9802, 9822, 9803, 9823, 9804, 9824, 9805, 9825, 9806, 9826, 9807, 9827, 9808, 9828, 9809, 9829, 9810, 9830; port 9801 answered as "ssp-mcp-server"; port 9821 answered as "ssp-mcp-server"
```

### 順を追った記録（日本時間・2026-10-11）

| 時刻 | したこと | 見えたこと |
|---|---|---|
| 0:44:36 | 開発者が Desktop を本体ごと起動し直した（areka は居ない） | 0:44:53 と 0:44:54 に中継が 2 つ起きた（親はどちらも Desktop 本体）。候補は既定の 20 個、控えの置き場は展開した場所の `profile\areka\mcp-bridge` |
| 0:44:55 | （Desktop が自分で）`initialize`・`tools/list` | どちらの中継も控えで答えた。`requested=2025-11-25`。ツール 11 本が並んだ |
| 0:45:16 | 開発者が SSP を起こした | SSP が 9801 と 9821 で待ち受け |
| 0:46:33 | `get_status`（areka は居ない） | 上の文のエラー。記録に `skipped a candidate: it answered as another server port=9801 name="ssp-mcp-server"` と 9821 の同じ行 |
| 0:46:41 | areka を起こした | areka は 9802 で待ち受け |
| 0:46:57 | `get_active_ghost_list`・`get_status` | `えも？？`・`balloon(0=0/1=0)`。記録に 9801・9821 を飛ばした行と `connected to areka port=9802` |
| 0:47:11 | areka を止めた（窓を閉じる合図・自分で終了） | 待ち受けは SSP の 2 つだけ |
| 0:47:18 | `get_status` | 上の文のエラー。記録に `the request did not reach areka port=9802 reason=connect failed: timed out`・`forgot the link to areka port=9802`・探し直しの行 |
| 0:47:24 | areka を起こし直した | areka は 9802 |
| 0:47:39 | `get_status`・`get_active_ghost_list` | 成功。控えの 2 つが 0:47:39 に書き直された |
| 0:48:20 | areka を止めてすぐ起こし直した（間に呼び出しなし） | areka は 9802 |
| 0:48 過ぎ | `get_status` | 成功。中継のプロセスは 0:47:38 に起きたものがそのまま（記録の行は増えていない＝探し直しも失敗も無し） |
| 8:13:24 | 開発者が Desktop の普通のチャット欄から、areka のツールの一覧・`get_active_ghost_list`・`get_status` を頼んだ | 11 本（`check_script`・`dump_balloon`・`dump_surface`・`get_active_ghost_list`・`get_expression_table`・`get_log`・`get_property`・`get_status`・`raise_event`・`reload`・`sakurascript`）・`えも？？`・`balloon(0=0/1=0)`。チャットの分の中継は 0:44:53 に起きたものがそのまま（7 時間あまり呼び出しなし）で、最初の呼び出しで `connected to areka port=9802` |
| 8:14:34 | areka を止めた（窓を閉じる合図・自分で終了） | 待ち受けは SSP の 2 つだけに戻った |

## 実機で分かったこと

- **エラーで答えた後、Desktop の側が中継を起こし直すことがある**。0:46:33 と 0:47:18 にエラーで答えた後、次の呼び出しの直前（0:46:56・0:47:38）に中継の記録に `finished quit=Closed`（標準入力が閉じた）が出て、新しい中継が起きて `initialize` から始め直した。閉じたのは Desktop の側で、理由はこちらからは分からない。新しい中継は areka を見つけて答えたので、利用者から見た結果（Desktop を落とさずに呼べる）は変わらない。
- **同じ中継のまま新しい areka へ届くこと**は、0:48:20 の確かめで別に見た（エラーを挟まずに areka を起こし直すと、中継のプロセスは替わらずに次の呼び出しが通った）。
- **中継が 2 つ起きる**。Desktop は同じ設定の項目から中継を 2 つ起こした（チャットの分と Code の分と見られる）。記録は同じファイルに混ざり、控えは同じ置き場を 2 つが使った（中身は壊れていない）。
- **窓を閉じただけでは設定は読み直されない**（上の「空振り」）。説明書の「起動し直す」は本体ごと落とす意味だと分かるように書くとよい（完了のときの文書の手入れへ送る）。
- Desktop の記録の置き場は、この机では `%LOCALAPPDATA%\Claude\Logs`（`%APPDATA%\Claude\logs` は 2026-08 で止まっている古い置き場）。

## 下ごしらえで見えたこと（Desktop を通さない確かめ・2026-10-10）

- 展開した zip の `areka.exe` は、ほかに誰も居ない机で 9801 を取った。help（`http://127.0.0.1:9801/api/mcp/help`）の Claude Desktop の節の設定例は、展開した場所の `areka-mcp-bridge.exe` の絶対パスを `command` に書き、`env` は無い（既定の候補の番号なので要らない）。
- 展開した zip の中継を手で起こし（`AREKA_MCP_PORT=9801`・`AREKA_PROFILE_DIR` は展開した場所の `profile\areka`）、`initialize`（`2025-06-18`）・`notifications/initialized`・`tools/list`・`tools/call`（`get_status`）の 4 行を渡した。標準出力は 3 行（通知には何も書かない）: 名乗りは `areka-mcp-server` 0.0.2・版は `2025-06-18`、ツールは 11 本、`get_status` は `balloon(0=0/1=0)`。終了コード 0。
- 控えは `target\t7\areka\profile\areka\mcp-bridge` に 2 つ出来た（`server-info.json` 625 バイト・`tools.json` 6,780 バイト＝ツール 11 本）。
- areka は窓を閉じる合図で自分で終わり、9801〜9830 で待ち受けるものは 0 に戻った。

## 確かめ ⑺（2026-10-11・areka も控えも無い机・タスク 8.3）

きっかけ: ⑴〜⑹ の後、中継を開発者の cargo の `bin` フォルダ（`c:\rust\cargo\bin`）へ入れて設定を書き換え、areka を起こさずに Desktop を起動し直したところ、控えの置き場が新しく空だったため、中継が最初の問い合わせにエラーで答え、Desktop は areka の項目を始められなかった（`Couldn't start for Cowork and Code sessions`。8:44〜9:24 に 8 回）。開発者の裁定 6 で「areka も控えも無いときは中継が組み込みの答えで答える」に改め（タスク 8.1・8.2）、直した中継で確かめ直した。

| 項目 | 値 |
|---|---|
| 版 | areka 0.0.2・コミット `1f99909`・未コミットの変更 0 |
| 配布スクリプトの検査 | `tools/package.ps1 -Check` が全段 OK・記録の判定 7 行とも PASS・終了コード 0（出力は `target\t8\package-check.txt`。zip の SHA256 は `f51117f5470b64fb439db95e06e980025b3d4a41242a50fb350fd7d3650e3d67`） |
| 中継 | `cargo install --path …\crates\areka-mcp-bridge --locked` で `c:\rust\cargo\bin\areka-mcp-bridge.exe` を置き換えた（9:35・3,182,080 バイト。動いている中継が 0 だったのでそのまま置き換わった） |
| Desktop の設定 | `areka` の `command` は `areka-mcp-bridge.exe`（PATH から。開発者の手） |
| 起動し直す前の机 | areka は動いていない・9801〜9830 で待ち受けるもの 0・SSP も動いていない・中継のプロセス 0・控えの置き場は無い |
| Desktop | Store 版 `Claude_2.31226.1.0_x64`。開発者が本体ごと落として起動し直した（9:36:02） |

| 時刻 | したこと | 見えたこと |
|---|---|---|
| 9:36:07・9:36:08 | （Desktop が自分で）中継を 2 つ起こし、`initialize`・`tools/list` | 記録に `areka not found; answered from the built-in definitions method="initialize"` と `method="tools/list"`（中継 2 つとも）。`requested=2025-11-25`。この回に `Couldn't start` の行は 0。ツール 11 本が並んだ |
| 9:36:50 | `get_status`（areka は居ない） | エラー `Server not available: areka-mcp-server not found on 127.0.0.1 ports 9801, 9821, …, 9830`（SSP が居ないので名乗りの続きは無い） |
| 9:36:59 | areka を起こした（新しい zip を `target\t8\areka` へ展開したもの・コミット `1f99909`） | areka は 9801 で待ち受け |
| 9:37:10 | `get_status`・`get_active_ghost_list` | `talking,balloon(0=0/1=0)`・`えも？？`。記録に `connected to areka port=9801`。Desktop の本体は 9:36:02 に起きたものがそのまま。控えの 2 つが 9:37:10 に出来た |
| 9:37:54 | areka を止めた（窓を閉じる合図・自分で終了） | 待ち受けるもの 0 に戻った |

**結果: 合格**（areka も控えも無いのに Desktop が areka の項目を始めてツールが並び、areka を起こすと Desktop を起動し直さずに呼べた）。

この回で分かったこと:

- **Store 版の Desktop が起こした中継の控えは、Desktop のパッケージのフォルダへ回される**。中継の記録は置き場を `C:\Users\<利用者>\AppData\Local\areka\mcp-bridge` と書くが、ファイルの実物は `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\areka\mcp-bridge` に出来た（Store 版のアプリは、子プロセスが `%LOCALAPPDATA%` へ書いたものを自分のパッケージのフォルダへ回す）。中継の読み書きは同じ所へ回るので動きは変わらないが、手で控えを消したい人は探す場所が違う。zip の形（中継の隣に `areka.exe`）の控えは zip のフォルダの中なので、この話は当たらない。
- Desktop は、項目を始められなかった後も時々やり直していた（8:44〜9:24 に 8 回）。
- エラーで答えた後、Desktop の Code の側は次の呼び出しの前に中継を閉じて起こし直した（⑶ のときと同じ・結果は変わらない）。

## 後片付け

- areka は止めた（エージェントが起こしたものだけ）。SSP と Desktop には触っていない。
- Desktop の設定の `areka` の項目は開発者が外す（`command` がワークツリーの `target` の下を指しているため、ワークツリーを片付けると起動に失敗する。Desktop が起こした中継 2 つは Desktop を落とすまで動いたままで、その間は展開した場所の `areka-mcp-bridge.exe` を消せない）。

## 完了のときの申し送り

- 署名の spec と winget の spec へ: 配布 zip の実行ファイルが 4 本になった（`areka-mcp-bridge.exe` が増えた）。
- 説明書（`dist/README.txt` と help）の「Claude Desktop を起動し直す」に、タスクバーに残る本体ごと終了する旨を足すかを決める。
