# 実機の非退行（タスク 6.2）

> **コミット ID についての注記**: 本記録のコミット ID は、2026-09-17 に本ブランチを `origin/main`（#149 `5f64a4c9`・#150 `eb4ef6ab`）の上へ rebase する**前**のものである。rebase で ID は変わったが、各コミットの中身（本 spec の差分）は同じ。取り込み後の全体テストは `final-check.md` 末尾の追記を参照。

- 日時: 2026-09-17（日本時間 21:19〜21:26）
- コミット: `da03908321997bb8538b58033bb14600d0ee32ad`（作業ツリーに未コミットの変更なし）
- 対象要件: 1.5, 2.7, 5.1, 5.2, 5.3, 5.4
- 読み方の出所: e2e 手順書 `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/lap-procedure.md` の §5.7（語の表と「項目 13 の 5 語の読み方」）・§2.2（32bit 版の確かめ方）・§3.2（起動の形）

## 結論

終了挨拶を経た走行（走行 3）で、4 つの語は期待どおり **`unload_clean` 1 行・`unload_failed` 0 行・`helper_exited` 0 行・`connect_failed` 0 行** だった。
有界の自動終了だけで終えた走行 1・2 でも 4 語は同じ 1・0・0・0 だった。

## 1. ビルドと 32bit の脳

| 項目 | 値 |
|---|---|
| ビルド種別 | debug（`dev` プロファイル） |
| 本体 | `cargo build -p areka --bin areka`（終了コード 0） |
| 32bit の脳 | `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`（終了コード 0・PowerShell で実行） |
| 置いた場所 | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-host32-window-thread-4094f2\target\debug\shiori-host32-helper.exe`（走行の直前に `target\i686-pc-windows-msvc\debug\` からコピー） |
| 32bit 版である根拠 | PE ヘッダの machine 欄が `014C`（i386）。大きさ 273,408 バイト＝i686 版と同じ（x64 版は 317,952 バイト・`final-check.md` §1） |

## 2. 資産（すべて絶対パス）

| 役割 | 絶対パス |
|---|---|
| 実バイナリ | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-host32-window-thread-4094f2\target\debug\areka.exe` |
| ゴースト（argv[1]） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-host32-window-thread-4094f2\crates\pilot\examples\shiori-host-32\fixtures\emo2` |
| バルーン（argv[2]） | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-host32-window-thread-4094f2\crates\pilot\examples\shiori-host-32\fixtures\emo2\emo2-kakukaku` |

絶対パスは `(Resolve-Path …).Path` で作った。生ログ冒頭の `resolved config inputs ghost_root=… balloon_root=…` に同じ絶対パスが出ている。
主モニタの実効 DPI は生ログの `primary_dpi=192`（拡大率 200%）。

## 3. 環境変数

| 変数 | 走行 1 | 走行 2 | 走行 3 |
|---|---|---|---|
| `RUST_LOG` | `info,kanade=trace,areka_kanade=trace` | 同左 | 同左 |
| `AREKA_APP_SMOKE_EXIT_MS` | `90000` | `150000` | `150000`（上限として付けた。実際はそれより前に終了操作で終わった） |
| `AREKA_TICK_GATE` | 未指定（既定） | 未指定（既定） | 未指定（既定） |
| `AREKA_CHOICE_HOVER_INJECT` | 未指定（既定） | 未指定（既定） | 未指定（既定） |

直接起動には番犬（ハング判定の締切）が付かない。終わりを与えるのは有界の自動終了だけである（手順書 §3.4）。

## 4. 走行

### 4.1 自動終了は終了挨拶を通らない（走行 1・2 で分かったこと）

`AREKA_APP_SMOKE_EXIT_MS` の自動終了は窓を直接閉じる（`crates/areka/src/main.rs` の自動 close ゲート→`despawn_smoke_targets`）。
窓が閉じたことを終端として終了処理が走るので、kanade は**強制終了系列**へ入る。生ログでは `event="force_quit"` 1 行・`method=NOTIFY id=OnClose` 1 行で、`method=GET id=OnClose`・`event="close_talk_start"`・`event="ghost_quit"` はどれも 0 行だった。
つまり自動終了だけでは**終了挨拶は再生されない**（手順書 §5.7 の「`ghost_quit` も `force_quit` も…有界自動終了で終わっている」の読みと一致）。

そこで要件 5.1 の「終了挨拶を経て解放する」を満たす走行 3 を別に行った。
走行 3 は手順書 §3.5 の正規の終了操作（Ctrl+左ダブルクリック）を、キャラ窓の上でスクリプトから合成して送った。自動終了は上限として残した（有界）。

### 4.2 起動コマンド

走行 1（自動終了のみ・PowerShell）:

```powershell
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$env:RUST_LOG = "info,kanade=trace,areka_kanade=trace"; $env:AREKA_APP_SMOKE_EXIT_MS = "90000"
& $AREKA $GHOST $BALLOON 2>&1 | Out-File -FilePath <ログ> -Encoding utf8
```

走行 2・3（`Start-Process` で起動し、標準出力と標準エラーを別ファイルへ受けて後で連結）:

```powershell
$env:RUST_LOG = "info,kanade=trace,areka_kanade=trace"; $env:AREKA_APP_SMOKE_EXIT_MS = "150000"
$p = Start-Process -FilePath $AREKA -ArgumentList "`"$GHOST`"", "`"$BALLOON`"" `
       -RedirectStandardOutput <out.log> -RedirectStandardError <err.log> -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 45
# 走行 3 のみ: 最も大きいキャラ窓の中心線上を下から順にカーソルで当たり、
# WindowFromPoint の所有プロセスが $p.Id と一致した点で
# keybd_event(VK_CONTROL 押下) → 左クリック 2 回（80 ms 間隔）→ VK_CONTROL 解放
$p.WaitForExit()
```

走行 2 は合成操作のスクリプトが型の誤りで 1 度も押せず（`clicked=False`）、自動終了で終わった。走行 3 はその誤りを直して再実行したもので、`click at 2446,1540 (hit pid match)`・`clicked=True` を出した。

`$AREKA`・`$GHOST`・`$BALLOON` は §2 の絶対パス。生ログはリポジトリの外（セッションの一時置き場）に置いた。

### 4.3 走行時間と終了コード

| 走行 | 開始 | 終了 | 所要 | 終了の経路 | 終了コード | 生ログ行数 |
|---|---|---|---|---|---|---|
| 1 | 21:19:52.99 | 21:21:31.85 | 98.9 秒 | 自動終了（90 秒）→強制終了系列 | 0 | 424 |
| 2 | 21:22:38.42 | 21:25:11.97 | 153.6 秒 | 自動終了（150 秒）→強制終了系列 | 0 | 549 |
| 3 | 21:25:31.35 | 21:26:19.25 | 47.9 秒 | Ctrl+左ダブルクリック（起動 45 秒後）→終了挨拶→解放 | 0 | 263 |

走行後、`areka` と `shiori-host32-helper` のプロセスが残っていないことを `Get-Process areka,shiori-host32-helper` で確かめた（該当なし）。

## 5. 4 つの語の実測行数

| 語 | 期待 | 走行 3（終了挨拶あり） | 走行 1 | 走行 2 |
|---|---|---|---|---|
| `event="unload_clean"` | 1 | **1** | 1 | 1 |
| `event="unload_failed"` | 0 | **0** | 0 | 0 |
| `event="helper_exited"` | 0 | **0** | 0 | 0 |
| `event="connect_failed"` | 0 | **0** | 0 | 0 |

0 行の 3 語は、ログに出なかったことをそのまま 0 と書いている。観測点が点いていることは、同じ走行で対になる `event="unload_clean"` が 1 行出ていること（解放の報告を書く受信ループそのものが最後まで動いていた）で確かめた。

走行 3 の終了系列を読むための補助の語（手順書 §5.7「項目 13 の 5 語」）:

| 語 | 期待 | 走行 3 | 走行 1 | 走行 2 |
|---|---|---|---|---|
| `method=GET id=OnClose` | 1 | 1 | 0 | 0 |
| `method=NOTIFY id=OnClose` | 0 | 0 | 1 | 1 |
| `event="close_talk_start"` | 1 | 1 | 0 | 0 |
| `event="ghost_quit"` | 1 | 1 | 0 | 0 |
| `event="force_quit"` | 0 | 0 | 1 | 1 |
| `steady_unexpected_reply` | 0 | 0 | 0 | 0 |
| `event=boot_input_ignored` | 0 | 0 | 0 | 0 |

走行 1・2 の欄は §4.1 のとおり自動終了の経路なので、`force_quit` 1・`GET` 0 が正しい姿である（退行ではない）。

### 5.1 数え方

語そのもの（`unload_clean` など・引用符や `event=` を付けない素の部分文字列）で `grep -c` を取った。

```bash
cat real-machine-run3.out.log real-machine-run3.err.log > run3.raw.log
sed 's/\x1b\[[0-9;]*m//g' run3.raw.log > run3.clean.log
for w in unload_clean unload_failed helper_exited connect_failed; do
  echo "$w raw=$(grep -c -- "$w" run3.raw.log) clean=$(grep -c -- "$w" run3.clean.log)"
done
```

- 色の制御文字を取り除く前（raw）と後（clean）の数は、全走行・全語で一致した。制御文字（ESC）は 3 本の生ログのどれにも 0 個だった（出力先がファイルのため色が付かない）。
- `grep -c` は 0 件のとき終了コード 1 を返すが、印字される数 `0` をそのまま記録した。
- `event="…"` 付きの綴りでも数え直し、同じ数になった（走行 1: `unload_clean` 1・`force_quit` 1）。

## 6. 終了挨拶を経て解放したことの根拠（走行 3 の生ログ・時刻は UTC）

| 時刻 | 行の要旨 |
|---|---|
| 12:26:16.874759 | `Ctrl+左ダブルクリック: 終了指示を送る（窓は握手の完了後に閉じる） event="close_requested"` |
| 12:26:16.875088 | `close 指示——active talk なし・即握手開始 event="steady_close_now" reason="user"` |
| 12:26:16.875349 | `OnClose GET を発行し握手を開始 event="close_handshake_begin" reason="user"` |
| 12:26:16.875599 | `SHIORI 送出 event="shiori_request" method=GET id=OnClose references=["user"]` |
| 12:26:16.878287 | `OnClose 応答スクリプト——close talk を再生起動し完了を待機 event="close_talk_start" talk_id=3` |
| 12:26:19.038060 | `reason=Quit——終了系列（Quit）へ event="talk_done_quit" talk_id=3` |
| 12:26:19.088948 | `shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)） event="unload_clean"` |
| 12:26:19.095197 | `kanade の終了系列が完了した: 全ゴースト窓を閉じる event="ghost_quit" cause=Quit` |
| 12:26:19.158749 | `ghost-shutdown: ghost shutdown sequence completed` |

照会（GET）→ 終了挨拶の再生開始 → 再生完了 → 解放成立 → 窓を閉じる、の順に並んでいる。終了挨拶の再生は約 2.2 秒だった。

## 7. 書き添えること

- **長い手空きは実機の走行では生じなかった。** 定常相では kanade が `OnSecondChange` を約 1 秒ごとに送るため、SHIORI への要求（`event="shiori_request"`）と解放の行の間隔の最大は、走行 1 で 1.53 秒・走行 2 で 1.59 秒・走行 3 で 2.21 秒（`OnClose` の照会から解放まで）だった。要件 1.3 の「20 秒以上要求が無い」状態は実機のこの 3 走行では作れておらず、その条件は決定論テスト（`calibration.md`）が受け持つ。本記録が示すのは、待ちの形を変えた後も解放と死活報告が退行していないこと（要件 1.5・2.7・5.1〜5.3）である。
- 走行 3 の終了操作は人の手ではなく、合成した入力である。押した点は所有プロセスが areka であることを確かめてから押した。ログ上は人の操作と同じ `event="close_requested"` の経路を通っている。
- ゴーストの永続状態（`profile\areka\`）は消していない（本記録は初回起動の系列を観測対象にしない）。
