# 実機の記録: areka-P0-open-external-tags（タスク 6.2）

## 0. 最終の判定

**合**。R1（`e9331f6c`）では関連付けの無いファイル（⑼）が否だった。OS の「アプリの選択」の窓が出て、失敗も記録されなかった。修正 `def14d1a` で、`os_port.rs` が `ShellExecuteExW` の前に `AssocQueryStringW`（`ASSOCF_INIT_IGNOREUNKNOWN`）で動詞の関連付けを引き、無ければ 1155 を返すようにした。その後の R3 で ⑼ が合になり、③④⑥⑦ の退行も無かった（5 章）。①②⑤⑧⑩ と、記録・吹き出しの項目は R1・R2 の結果のままとした。修正は「関連付けが無いときは OS を呼ばない」分岐を足しただけで、それ以外の経路は R3 で ③④⑥⑦ が同じように開いたことで確かめた。

以下の 1〜4 章は修正前の R1・R2 の記録で、⑼ の否もそのまま残す。

## 1. 走行の条件

| 項目 | 値 |
|---|---|
| 版 | コミット `e9331f6c`（6.1 まで）から組んだ debug 版 `areka.exe`（x64）と `shiori-host32-helper.exe`（i686） |
| 根 | `<ワークツリー>\target\oet\root`（`nar-sample-path emo2` の展開 `target\nar-samples\manual\emo2` の `ghost\emo2`・`balloon\emo2-kakukaku` を写したもの） |
| 検体 | 既定ゴースト emo2（pasta・32bit）。**根の写しの** `ghost\master\scripts\pasta\shiori\event\boot.lua` だけを書き替え、`OnBoot`・`OnFirstBoot` が試験の台本（下の 1.1）を文字列のまま返すようにした（pasta の `EVENT.fire` は文字列の戻り値をそのまま応答にする）。`vendors/` の元のゴーストは触っていない |
| 開く先 | `<ワークツリー>\target\oet\files`（`a.txt`・`sub\b.txt`・`x.zzqqnoassoc`）。`assoc .zzqqnoassoc` は「関連付けが見つかりません」 |
| 記憶 | 根を新しく作った（`profile`・`tmp` は空） |
| `RUST_LOG` | `info,areka=debug,areka::readme=debug,areka::mcp=debug,areka_mcp=debug` |
| 自動終了 | R1 `AREKA_APP_SMOKE_EXIT_MS=240000`・R2 `60000`（どちらも発火して終わった） |
| 起動 | `target\oet\run.ps1`（`AREKA_*`／`WINTF_*` を全部外し、`AREKA_ROOT=<根>`・`AREKA_PROFILE_DIR=<根>\profile`・`TMP`／`TEMP=<根>\tmp`・`NO_COLOR=1` を付けて `Start-Process <根>\areka.exe "<根>\ghost\emo2"`）。記録は `target\oet\run-R1.log`・`run-R2.log` |
| 待受 | `MCP: 待受を始めた url=http://127.0.0.1:9801/api/mcp/v1`（R1・R2 とも）。起動の前に 9801〜9830 を待ち受けるプロセスは 0 件、SSP も動いていなかった。返った `get_log` の本文の根が `target\oet\root` なので、答えたのは自分の起こした areka |
| MCP の呼び方 | `target\oet\mcp.sh`（curl で `initialize` → `notifications/initialized` → `tools/call`） |

走行（pid は自分で起こしたもの・`runs.txt` 逐語）:

```
R1 pid=8740 start=2026-10-06T11:52:52.8021348+09:00 exit_ms=240000
R2 pid=34332 start=2026-10-06T11:57:03.9966149+09:00 exit_ms=60000
R3 pid=12252 start=2026-10-06T12:14:07.5270558+09:00 exit_ms=90000
```

終わり方（R1・R2・R3 とも）: `[quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=Smoke closed=4` → `shiori-actor: 正規 clean shutdown 完了（unload → helper 正常終了 exit(0)）` → `MCP: 待受を閉じた addr=127.0.0.1:9801`。こちらから止める操作はしていない。

### 1.1 流した台本

R1（`F` は `<ワークツリー>\target\oet\files` の絶対パス。`\_w[1500]` で 1.5 秒ずつ間を空けた 1 本の台詞）:

```
\0\s[0]開く系のテストを始めるよ。\_w[1500]\n
①URL\j[https://example.com/]を開いた。\_w[1500]\n
②browser\![open,browser,https://example.com/?browser]を開いた。\_w[1500]\n
③file 全体\![open,file,%SystemRoot%\notepad.exe]を開いた。\_w[1500]\n
④file 名前\![open,file,notepad.exe]を開いた。\_w[1500]\n
⑤explorer フォルダ\![open,explorer,F\sub]を開いた。\_w[1500]\n
⑥explorer ファイル\![open,explorer,F\a.txt]を開いた。\_w[1500]\n
⑦editor\![open,editor,F\a.txt]を開いた。\_w[1500]\n
⑧mailer\![open,mailer,test@example.invalid]を開いた。\_w[1500]\n
⑨関連付けなし\![open,file,F\x.zzqqnoassoc]を流した。\_w[1500]\n
⑩断る形\j[nope]を流した。\_w[1500]\n
まだ喋ってるよ。\_w[2000]\nもう少し喋るよ。\_w[2000]\nおわり。\_w[3000]\e
```

（実物は改行なしの 1 行。ここでは読みやすさのため `\n` の後で折った）

R2（失敗の記録が `error` に名付きで残ることの確認）:

```
\0\s[0]失敗の記録を確かめるよ。\_w[1500]\n⑪無いファイル\![open,file,F\missing.zzz]を流した。\_w[1500]\n⑫無いファイルを編集\![open,editor,F\missing.txt]を流した。\_w[1500]\nおわり。\_w[2000]\e
```

## 2. 判定（R1・R2・修正前）

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑴ | `\j[https://example.com/]` が既定のブラウザで開く（要件 1.1） | **合**（Edge に「Example Domain」のタブ。`open_external kind="url"`） |
| ⑵ | `\![open,browser,…]` が既定のブラウザで開く（要件 3.1） | **合**（同上・`kind="url"`・`destination=https://example.com/?browser`） |
| ⑶ | `\![open,file,%SystemRoot%\notepad.exe]` が環境変数を展開して実行する（要件 2.1・2.3） | **合**（`destination=C:\WINDOWS\notepad.exe`・メモ帳が起きた） |
| ⑷ | `\![open,file,notepad.exe]` を OS のパス探索で実行する（要件 2.4） | **合**（`destination=notepad.exe`・親が areka（pid 8740）の `Notepad.exe` pid 31328 が起きた） |
| ⑸ | `\![open,explorer,フォルダ]` がそのフォルダを開く（要件 4.1） | **合**（エクスプローラーの窓 `sub`・場所 `file:///…/target/oet/files/sub`） |
| ⑹ | `\![open,explorer,ファイル]` が親のフォルダをそのファイルを選んで開く（要件 4.2） | **合**（窓 `files`・選択中の項目が `…\target\oet\files\a.txt`） |
| ⑺ | `\![open,editor,…]` が「編集」の関連付けで開く（要件 5.1） | **合**（`verb="edit"`・`Notepad.exe …\target\oet\files\a.txt`（pid 34528・親 8740）。メモ帳の窓「a.txt - メモ帳」） |
| ⑻ | `\![open,mailer,…]` が既定のメールソフトで新しいメールを開く（要件 6.1） | **合**（`destination=mailto:test@example.invalid`・`olk.exe mailto:test@example.invalid`（pid 22496・親 8740）の窓「新規メール」。送信はしていない） |
| ⑼ | 関連付けの無いファイルは OS の窓が出ず、`error` に記録だけ残る（要件 5.4 相当の 2.5・7.4・7.6） | **否**（「アプリの選択」の窓（`OpenWith.exe` pid 36408）が出た。`open_external_failed` は 0 行で、`status` に成功の `open_external` が 1 行残った。下の 3 章 ⑼） |
| ⑽ | 断る形 `\j[nope]` は開かず `warn!` 1 行（要件 1.7） | **合**（`open_external_rejected reason=UnknownJumpId`・`get_log error` の #23） |
| ⑾ | 渡すたびに `info` 1 行・種類・解決した行き先・ゴースト名・元の綴り（要件 7.2） | **合**（受理した 9 形に 1 行ずつ・9 行。どの行にも `kind`・`destination`・`ghost=えも？？`・`tag`） |
| ⑿ | 成功の行が `get_log` の `status` で読める（要件 7.3） | **合**（#14〜#22 の 9 行） |
| ⒀ | 失敗が `get_log` の `error` でゴースト名付きで読める（要件 7.4） | **合**（R2 の #14・#15 `[Error] えも？？ : … open_external_failed … reason="not_found"`）。ただし ⑼ の OS の経路では失敗にならず記録されない |
| ⒁ | 開いている間も吹き出しが動く（要件 7.5） | **合**（どの `open_external` の後も、台本どおりの間隔で次の文字が 0.3〜2 秒以内に出ている。下の 3 章 ⒁） |
| ⒂ | メッセージボックスを出さない（要件 7.6） | areka 自身の窓は 0。**OS の「開く方法を選ぶ」窓が ⑼ で出た**ので、要件 7.6 の括弧書きに反する |

## 3. 詳細

### ⑴〜⑻ 開いたもの

`open_external` の行（`run-R1.log`・行頭の時刻を除いて逐語）:

```
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="url" destination=https://example.com/ ghost=えも？？ tag=\j[https://example.com/] verb="open"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="url" destination=https://example.com/?browser ghost=えも？？ tag=\![open,browser,https://example.com/?browser] verb="open"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="file" destination=C:\WINDOWS\notepad.exe ghost=えも？？ tag=\![open,file,%SystemRoot%\notepad.exe] verb="open"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="file" destination=notepad.exe ghost=えも？？ tag=\![open,file,notepad.exe] verb="open"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="folder" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\sub ghost=えも？？ tag=\![open,explorer,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\sub] verb="open"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="folder" destination=explorer.exe /select,"C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt" ghost=えも？？ tag=\![open,explorer,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt] verb="open"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="editor" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt ghost=えも？？ tag=\![open,editor,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt] verb="edit"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="mail" destination=mailto:test@example.invalid ghost=えも？？ tag=\![open,mailer,test@example.invalid] verb="open"
INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="file" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc ghost=えも？？ tag=\![open,file,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc] verb="open"
```

`\![…]` の引数の中の逆斜線（`%SystemRoot%\notepad.exe` の `\n` や `\target` の `\t`）は台本の解釈で崩れず、綴りのまま届いた。

起きたプロセスと窓（走行中に `Win32_Process`・`Get-Process`・`Shell.Application` で読んだもの）:

| 開いたもの | プロセス | 起動の時刻（現地） | 親 | 窓 |
|---|---|---|---|---|
| ⑶ `%SystemRoot%\notepad.exe` | `Notepad.exe` pid 29972 | 11:53:03 | 28160（`C:\WINDOWS\notepad.exe` の中継） | 「a.txt - メモ帳」（後の ⑺ がこの窓のタブに入った） |
| ⑷ `notepad.exe` | `Notepad.exe` pid 31328 | 11:53:04 | 8740（areka） | 既存のメモ帳の窓へタブとして合流（自分の窓は持たない） |
| ⑸ フォルダ | `explorer.exe` | 11:53:07〜 | — | `sub`（`file:///C:/home/maz/git/areka/.claude/worktrees/areka-p0-open-external-tags-701dc3/target/oet/files/sub`） |
| ⑹ ファイルを選んで | `explorer.exe` pid 33120（`/factory` 11:53:09） | 11:53:09 | — | `files`。選択中の項目 `…\target\oet\files\a.txt` |
| ⑺ editor | `Notepad.exe` pid 34528 `…\Notepad.exe C:\…\target\oet\files\a.txt` | 11:53:12 | 8740 | 「a.txt - メモ帳」 |
| ⑻ mailer | `olk.exe "…\olk.exe" mailto:test@example.invalid` pid 22496 | 11:53:14 | 8740 | 「新規メール」 |
| ⑴⑵ URL | 既に動いていた Edge（pid 22828） | — | — | 「Example Domain および他 13 ページ - 個人 - Microsoft Edge」 |

URL の 2 つは既存の Edge のタブに入るので、2 つのタブを数えては確かめていない（OS は 2 回とも失敗を返していない）。

### ⑼ 関連付けの無いファイル（否）

期待: OS の窓を出さず、`error!`（`open_external_failed`・`reason="os"`・`code=1155`）1 行。

実際:

- `status` に成功の行（上の 9 行目・#22）が出て、`error` の `open_external_failed` は 0 行（`run-R1.log` 全体で `open_external_failed` 0 件・ERROR 0 件）。
- 11:53:15（`open_external` の行は 02:53:15.754906Z）に `C:\WINDOWS\system32\OpenWith.exe -Embedding`（pid 36408・親 1912＝DCOM の起動役）が起き、窓「アプリの選択」が出た。走行が終わった後も残っている。

原因の見立て（コードは直していない）:

- `crates/areka/src/readme/os_port.rs` の `WindowsShell::shell_execute` は `fMask: SEE_MASK_FLAG_NO_UI`・`lpVerb: "open"` で `ShellExecuteExW` を呼ぶ。設計は「`SEE_MASK_FLAG_NO_UI` で OS のエラーの窓も『開く方法を選ぶ』窓も出ず、関連付けが無ければ 1155 が返る」としていた。
- このマシンの登録は次のとおりで、関連付けの無い拡張子は `HKCR\Unknown` の動詞へ落ちる。`Unknown\shell\Open` が在り、その実体が `OpenWith.exe`（`DelegateExecute`）なので、`lpVerb="open"` は「動詞が見つかった」として成功し、OS は 1155 を返さずに「アプリの選択」を出す。`SEE_MASK_FLAG_NO_UI` はエラーの窓を抑えるだけで、この経路には効かない。

```
HKEY_CLASSES_ROOT\Unknown\shell
    (既定)    REG_SZ    openas
HKEY_CLASSES_ROOT\Unknown\shell\Open
    MultiSelectModel    REG_SZ    Single
    ProgrammaticAccessOnly    REG_SZ
HKEY_CLASSES_ROOT\Unknown\shell\Open\command
    (既定)    REG_EXPAND_SZ    %SystemRoot%\system32\OpenWith.exe "%1"
    DelegateExecute    REG_SZ    {e44e9428-bdbc-4987-a099-40dc8fd255e7}
```

- 直し方の候補（判断は設計へ戻す）: 渡す前に関連付けを引いて（例 `AssocQueryStringW` に `ASSOCF_INIT_IGNOREUNKNOWN` を付け、拡張子と動詞で `ASSOCSTR_COMMAND` 等を引く）、`Unknown` に落ちるなら OS を呼ばずに 1155 として `error!` にする。`ASSOCF_INIT_IGNOREUNKNOWN` は「`Unknown` の既定へ落とさない」ための旗。`edit` の動詞（要件 5.4）も同じ経路を通るので、同じ判定が要る。決定論テストは偽の境界（`OsPort`）の外なので、この食い違いは常時テストでは見えない。

### ⑽ 断る形

```
WARN actor{actor=sakura-talk-1}: areka::emo2_boot::readme_cue: ReadmeCueSink: 開く系のタグを断った（開かない） event="open_external_rejected" tag=\j[nope] reason=UnknownJumpId
```

この行にはゴースト名の欄が無く、`get_log` では名が既定の `[SYSTEM]` になる（要件 1.7 は「ID と理由」なので要件どおり）。

### ⑾⑿ `get_log` の `status`（R1・11:54 ごろに取得・逐語・開く系の行だけ）

```
#14 2026/10/06 11:52 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="url" destination=https://example.com/ ghost=えも？？ tag=\j[https://example.com/] verb="open"
#15 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="url" destination=https://example.com/?browser ghost=えも？？ tag=\![open,browser,https://example.com/?browser] verb="open"
#16 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="file" destination=C:\WINDOWS\notepad.exe ghost=えも？？ tag=\![open,file,%SystemRoot%\notepad.exe] verb="open"
#17 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="file" destination=notepad.exe ghost=えも？？ tag=\![open,file,notepad.exe] verb="open"
#18 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="folder" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\sub ghost=えも？？ tag=\![open,explorer,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\sub] verb="open"
#19 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="folder" destination=explorer.exe /select,"C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt" ghost=えも？？ tag=\![open,explorer,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt] verb="open"
#20 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="editor" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt ghost=えも？？ tag=\![open,editor,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt] verb="edit"
#21 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="mail" destination=mailto:test@example.invalid ghost=えも？？ tag=\![open,mailer,test@example.invalid] verb="open"
#22 2026/10/06 11:53 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="file" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc ghost=えも？？ tag=\![open,file,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc] verb="open"
```

`status` の行は取り決めの target でないので、名と表示の語は既定の `STAT`。ゴースト名は本文の `ghost=えも？？` に入る（`log-convention.md` §3 の「取り決めの target でない行の `ghost` は本文に残る」どおり）。emo2 の descript の `name` は `えも？？`。

`get_log error`（R1・同時刻）の開く系の行:

```
#23 2026/10/06 11:53 [Error] [SYSTEM] : ReadmeCueSink: 開く系のタグを断った（開かない） event="open_external_rejected" tag=\j[nope] reason=UnknownJumpId
```

ほかの `error` の行は #4・#6（`purple/a/null.png` の全透明）と #13（折返し基準）で、どちらも前の spec の実機の記録で出ている emo2 の既知の警告。本 spec と関係ない。

### ⒀ `get_log` の `error`（R2・逐語）

```
#14 2026/10/06 11:57 [Error] えも？？ : [readme] could not resolve the destination to open event="open_external_failed" kind="file" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\missing.zzz tag=\![open,file,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\missing.zzz] reason="not_found" failure=NotFound("C:\\home\\maz\\git\\areka\\.claude\\worktrees\\areka-p0-open-external-tags-701dc3\\target\\oet\\files\\missing.zzz")
#15 2026/10/06 11:57 [Error] えも？？ : [readme] could not resolve the destination to open event="open_external_failed" kind="editor" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\missing.txt tag=\![open,editor,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\missing.txt] reason="not_found" failure=NotFound("C:\\home\\maz\\git\\areka\\.claude\\worktrees\\areka-p0-open-external-tags-701dc3\\target\\oet\\files\\missing.txt")
```

失敗の行は取り決めの target `areka::log::error` なので、名の欄がゴースト名 `えも？？` になる。R2 では OS のアプリも窓も起きていない（新しい `OpenWith.exe` も無い）。`reason="os"` の経路は実機では踏めなかった（⑼ で OS が失敗を返さないため）。

### ⒁ 開いている間も吹き出しが動く

`run-R1.log` の文字の配送（`areka_emo_text::state: Text cue 適用`・`actor=0`）と `open_external` を時刻順に並べたもの（時刻は UTC・`len` は文字数・`at` は台詞の中の秒）:

```
02:52:57.892518 Text cue 適用 len=4 at=2.15        ①URL
02:52:58.142976 Text cue 適用 len=5 at=2.35        を開いた。
02:52:58.145077 open_external kind=url
02:52:59.908412 Text cue 適用 len=8 at=4.1         ②browser
02:53:00.246111 Text cue 適用 len=5 at=4.5
02:53:00.248466 open_external kind=url
02:53:02.004962 Text cue 適用 len=8 at=6.25        ③file 全体
02:53:02.403017 Text cue 適用 len=5 at=6.65
02:53:02.409502 open_external kind=file
02:53:04.171962 Text cue 適用 len=8 at=8.4         ④file 名前
02:53:04.619561 Text cue 適用 len=5 at=8.8
02:53:04.623609 open_external kind=file
02:53:06.386911 Text cue 適用 len=14 at=10.55      ⑤explorer フォルダ
02:53:07.004345 Text cue 適用 len=5 at=11.25
02:53:07.013400 open_external kind=folder
02:53:08.833481 Text cue 適用 len=14 at=13.0       ⑥explorer ファイル
02:53:09.446031 Text cue 適用 len=5 at=13.7
02:53:09.456494 open_external kind=folder
02:53:11.212025 Text cue 適用 len=7 at=15.45       ⑦editor
02:53:11.588869 open_external kind=editor
02:53:11.620032 Text cue 適用 len=5 at=15.8
02:53:13.369399 Text cue 適用 len=7 at=17.55       ⑧mailer
02:53:13.664644 open_external kind=mail
02:53:13.698519 Text cue 適用 len=5 at=17.9
02:53:15.414694 Text cue 適用 len=7 at=19.65       ⑨関連付けなし
02:53:15.752180 Text cue 適用 len=5 at=20.0
02:53:15.754906 open_external kind=file
02:53:17.544542 Text cue 適用 len=4 at=21.75       ⑩断る形
02:53:17.695759 open_external_rejected
02:53:17.753592 Text cue 適用 len=5 at=21.95
02:53:19.484353 Text cue 適用 len=8 at=23.7        まだ喋ってるよ。
02:53:21.934690 Text cue 適用 len=8 at=26.1        もう少し喋るよ。
02:53:24.293627 Text cue 適用 len=4 at=28.5        おわり。
02:53:27.445985 kanade: talk 完了——定常運転へ復帰 event="steady_talk_done"
```

（行頭の `DEBUG actor{actor=emo-text}: areka_emo_text::state:` と行末の欄を省いた。右の語は台本の対応する文字）

台詞の中の秒（`at`）と壁時計の差は最初から最後まで約 55.7 秒で一定（`at=2.15` が 02:52:57.89、`at=28.5` が 02:53:24.29）。開く 9 回のどこでも文字の配送が遅れていない。OS への受け渡しは別のスレッド（`open-external`）で、`open_external` の行は `actor` の span の外から出ている。

参考: `loop ticker catch-up: skipped multiple boundaries` の INFO が R1 で 10 回出た（02:53:03〜02:54:01）。台詞の終わった後（02:53:33・35・47・54:01）にも出ており、開く処理の時刻とは対応しない（debug 版と、同じ机で別のワークツリーの areka が 1 つ動いていた負荷による）。

### 警告と標準エラー出力

| 記録の段 | R1 | R2 |
|---|---|---|
| ERROR | 0 | 2（R2 の ⑪⑫・狙ったもの） |
| WARN | 4（`null.png` 2・折返し基準 1・`open_external_rejected` 1） | 3（`null.png` 2・折返し基準 1） |
| 標準エラー出力 | 1 行（`[helper] SHIORI 初期化の入口: loadu`） | 同じ 1 行 |

## 4. 開いたまま残したもの

実機の確認（R1〜R3）で起きた次の窓は閉じていない（開発者が閉じる）。

- メモ帳「a.txt - メモ帳」（pid 29972。⑶⑷⑺ のタブ）
- エクスプローラーの窓 2 つ（`sub`・`files`）
- Edge の「Example Domain」のタブ 2 つ（既存の Edge の窓の中）
- Outlook（new）の「新規メール」（pid 22496・宛先 `test@example.invalid`・未送信）
- 「アプリの選択」（`OpenWith.exe` pid 36408・R1 の ⑼ で出たもの。R3 では新しく出ていない）
- R3 で増えたもの:
  - メモ帳のタブ 3 つ（`"C:\WINDOWS\notepad.exe"` pid 8744・`Notepad.exe` pid 39652・`…\files\sub\b.txt` の pid 23008）。どれも既存のメモ帳の窓（題名は今「b.txt - メモ帳」）に合流した
  - エクスプローラーの窓 `files` がもう 1 つ（a.txt を選んだ状態・`explorer.exe /factory` pid 2152）

## 5. 修正の後の再確認（R3）

### 5.1 条件

| 項目 | 値 |
|---|---|
| 版 | コミット `def14d1a`（⑼ の修正）から組んだ debug 版 `areka.exe`（x64・`-j 2`）。helper は R1 と同じ |
| 根・記憶・`RUST_LOG`・起動 | R1 と同じ根 `target\oet\root`（`areka.exe` だけ差し替え・記憶は R1・R2 の続き）。`run.ps1 -Run R3 -ExitMs 90000` |
| 待受 | `http://127.0.0.1:9801/api/mcp/v1`。起動の前に 9801〜9830 の待受は 0 件。別のワークツリーの areka（pid 31176）が動いていたが、返った本文の根は `target\oet\root` |
| 走行 | `R3 pid=12252 start=2026-10-06T12:14:07.5270558+09:00 exit_ms=90000`（自動終了で終わった・こちらから止めていない） |
| 起動の前の `OpenWith.exe` | R1 の pid 36408（11:53:15 起動）だけ |

台本（`F` は R1 と同じ）:

```
\0\s[0]直した後の確認だよ。\_w[1500]\n⑨関連付けなし\![open,file,F\x.zzqqnoassoc]を流した。\_w[1500]\n③file 全体\![open,file,%SystemRoot%\notepad.exe]を開いた。\_w[1500]\n④file 名前\![open,file,notepad.exe]を開いた。\_w[1500]\n⑥explorer ファイル\![open,explorer,F\a.txt]を開いた。\_w[1500]\n⑦editor\![open,editor,F\sub\b.txt]を開いた。\_w[1500]\nまだ喋ってるよ。\_w[2000]\nおわり。\_w[2000]\e
```

### 5.2 判定

| # | 確かめたこと | 結果 |
|---|---|---|
| ⑼ | 関連付けの無いファイルで OS の窓が出ず、`open_external` の後に `open_external_failed reason="os" code=1155` が `error` に残る | **合**（走行の後の `OpenWith.exe` は R1 の pid 36408 だけで、新しく起きたものは 0。`get_log error` の #15 に `[Error] えも？？ : … reason="os" code=1155`） |
| ③ | `%SystemRoot%\notepad.exe` が起きる | **合**（`"C:\WINDOWS\notepad.exe"` pid 8744・親 12252・12:14:16） |
| ④ | `notepad.exe` がパス探索で起きる | **合**（`Notepad.exe` pid 39652・親 12252・12:14:19） |
| ⑥ | ファイルを選んだ状態でフォルダが開く | **合**（`explorer.exe /factory` pid 2152・12:14:21。新しい窓 `files` の選択中の項目が `…\target\oet\files\a.txt`） |
| ⑦ | `.txt` が「編集」で開く | **合**（`verb="edit"`・`Notepad.exe …\target\oet\files\sub\b.txt` pid 23008・親 12252・12:14:23。メモ帳の窓が「b.txt - メモ帳」になった） |
| ⒁ | 開いている間も吹き出しが動く | **合**（どの `open_external` の後も、次の文字が 0.02 秒以内に出ている。台詞の秒（`at`）と壁時計の差は、`at=0.0`（03:14:12.36）から最後の `at=15.35`（03:14:27.70）まで約 12.35 秒で一定） |

### 5.3 記録（逐語）

`run-R3.log`（⑼ の 2 行）:

```
2026-10-06T03:14:14.697868Z  INFO areka::readme::opener: [readme] handed the destination to the OS event="open_external" kind="file" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc ghost=えも？？ tag=\![open,file,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc] verb="open"
2026-10-06T03:14:14.706249Z ERROR areka::log::error: [readme] the OS refused to open the destination ghost=えも？？ event="open_external_failed" kind="file" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc tag=\![open,file,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc] reason="os" code=1155
```

`get_log status`（開く系の行）:

```
#14 2026/10/06 12:14 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="file" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc ghost=えも？？ tag=\![open,file,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc] verb="open"
#16 2026/10/06 12:14 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="file" destination=C:\WINDOWS\notepad.exe ghost=えも？？ tag=\![open,file,%SystemRoot%\notepad.exe] verb="open"
#17 2026/10/06 12:14 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="file" destination=notepad.exe ghost=えも？？ tag=\![open,file,notepad.exe] verb="open"
#18 2026/10/06 12:14 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="folder" destination=explorer.exe /select,"C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt" ghost=えも？？ tag=\![open,explorer,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\a.txt] verb="open"
#19 2026/10/06 12:14 [STAT] STAT : [readme] handed the destination to the OS event="open_external" kind="editor" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\sub\b.txt ghost=えも？？ tag=\![open,editor,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\sub\b.txt] verb="edit"
```

`get_log error`（開く系の行）:

```
#15 2026/10/06 12:14 [Error] えも？？ : [readme] the OS refused to open the destination event="open_external_failed" kind="file" destination=C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc tag=\![open,file,C:\home\maz\git\areka\.claude\worktrees\areka-p0-open-external-tags-701dc3\target\oet\files\x.zzqqnoassoc] reason="os" code=1155
```

成功の `info` が先に出て、失敗の `error` が続く。これは設計どおりで、`info` は OS へ渡す直前、`error` は境界が 1155 を返した後に出る。R1 では踏めなかった `reason="os"` の経路も、これで実機で踏んだ。

警告と標準エラー出力（R3）:

| 記録の段 | 件数 |
|---|---|
| ERROR | 1（⑼・狙ったもの） |
| WARN | 4（`null.png` 2・折返し基準 1・自動終了の `強制終了指示——終了系列（Forced）へ直行` 1） |
| 標準エラー出力 | 1 行（`[helper] SHIORI 初期化の入口: loadu`） |
