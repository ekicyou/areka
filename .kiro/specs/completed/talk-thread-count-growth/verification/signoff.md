# 直した後の実機での数え直し（要件 1.5・4.4・7.8）

- **実施日**: 2026-11-28 00:00〜00:03 JST（ログの時刻は UTC で 2026-11-27 15:00〜15:03）
- **ビルドしたコミット**: `4eaabe8e8284a097245573099a7fb9ddbc491364`（直した後の HEAD・作業ツリーは未変更）
- **ビルド**: `cargo build -p areka`（debug）と `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`（debug）。32bit SHIORI ヘルパは `target\debug\` の `areka.exe` の隣へ写した（sha256 が元と一致）
- **判定**: **合**。報告の `threads=` も OS の数もトークと切り替えの回数に比例して増えず、差も広がらない。`role=actor:sakura-talk-*` の畳んだ行の `exited=12` は、登録の `debug!` の 12 行 − 最後の 1 枚で生きている 0 本と合う

## 手順

タスク 1（`isolation.md`）と同じ手順に、ゴーストの切り替えを 1 回足した。違いは置き場を 2 体のゴーストが入る根にしたことと、ログの置き場だけ。

1. 走らせる前に `netstat -ano` で `127.0.0.1:9276` を握るプロセスが無いこと、`areka.exe` が起きていないことを確かめた（どちらも無し）。SSP（pid 33684）が 9801・9821 を握っていたので、areka の MCP は 9802 で待ち受けた。
2. 切り替え先のゴーストが目録に載るように、ワークツリーの `target\tc\sw` を根にして、次を置いた（完了 `areka-P0-ghost-shell-balloon-switch` の実機確認と同じ組み立て）。
   - `ghost\emo2`: タスク 1 の `target\tc\emo2` の複製
   - `ghost\R_POST_and_KOMAINU`: `C:\home\maz\tmp\loadu\ascii\R_POST_and_KOMAINU` の複製（`name,Ｒポストと狛犬`・SHIORI は里々 `satori.dll`＝i686 のヘルパ経由）
   - `balloon\emo2-kakukaku`・`balloon\StayseeBalloon`
3. `AREKA_*`・`WINTF_*` の環境変数を全部外したうえで、次を足して `Start-Process` で起こした（作業フォルダは `target\debug`）。
   - `AREKA_PERF_THREAD_REPORT_SEC=10`
   - `RUST_LOG=info,areka::perf=debug,wintf::thread_registry=debug`
   - `NO_COLOR=1`・`AREKA_NO_ALERT=1`
   - `AREKA_ROOT` は `...\target\tc\sw`（タスク 1 から足したのはこれだけ）
   - `AREKA_PROFILE_DIR`・`TEMP`・`TMP` はワークツリーの `target\tc\profile`・`target\tc\tmp`
   - 引数はゴースト `...\target\tc\sw\ghost\emo2` とバルーン `...\target\tc\sw\balloon\emo2-kakukaku`（どちらも絶対パス）
   - 標準出力・標準エラー出力は `target\tc\logs-signoff\run.log`・`err.log`（タスク 1 のログ `target\tc\logs\` は残した）
4. 起こしたプロセスは pid 30260。`netstat` で 9802 の待ち受けがこの pid であること、MCP の `get_active_ghost_list` が「えも？？」を返すことを確かめた。
5. 1 対の取り方はタスク 1 と同じ。待ち始めの時点の最新の `snap=` より後の `perf(process)` の 1 枚が出るのを待ち、その `threads=` と、出た直後に PowerShell で読んだ `(Get-Process -Id 30260).Threads.Count` を 1 対とした。トークは areka の MCP の `sakurascript` で `\0\s[0]スレッドの数を数えています。\e` を流し、`get_status` で `talking` が消えたのを見てから次の 1 枚を待った。
6. 流した順:
   - emo2 へトーク 3 回
   - MCP の `sakurascript` で `\0\![change,ghost,Ｒポストと狛犬]\e` を流してゴーストを切り替えた（`check_script` で診断 0 件を見てから）。ログに `ghost_switch_requested`（15:01:39.846）→ `ghost_switch_booted` ghost=R_POST_and_KOMAINU attempt=Target（40.142）→ `ghost_switch_done`（40.286）が出て、`get_active_ghost_list` は「Ｒポストと狛犬」を返した。画面の操作はしていない
   - R_POST へトーク 3 回、その後に最後の 1 枚を 1 つ待った
7. 止めたのは、`Get-Process -Id 30260` のパスがこのワークツリーの `target\debug\areka.exe` であると確かめた pid 30260 だけ。止めた後に `areka.exe`・`shiori-host32-helper.exe` がどちらも 0 件であることを確かめた。走行のログに ERROR の段の行は 0 件、`wintf::thread_registry` の WARN の行も 0 件。

MCP で流したのは、トーク 6 回と切り替えの台本 1 回の計 7 回。

## 対の数の表

| 時点 | snap | t_s | 報告の `threads=` | OS の `Threads.Count` |
|---|---|---|---|---|
| 起動の後の 1 枚（起動の挨拶は snap=1 より前に終わり、ゴースト自身の時刻のトーク `talk_id=2` が再生中） | 3 | 30 | 16 | 91 |
| 次の 1 枚（`talk_id=2` が終わった後） | 4 | 40 | 15 | 92 |
| 次の 1 枚（時刻のトーク `talk_id=3` が再生中） | 5 | 50 | 16 | 93 |
| MCP のトーク 1 の後（emo2） | 6 | 60 | 15 | 89 |
| MCP のトーク 2 の後（emo2） | 7 | 70 | 15 | 89 |
| MCP のトーク 3 の後（emo2） | 8 | 80 | 15 | 89 |
| ゴーストの切り替え（emo2 → R_POST）の後 | 10 | 100 | 15 | 89 |
| MCP のトーク 4 の後（R_POST） | 12 | 120 | 15 | 89 |
| MCP のトーク 5 の後（R_POST） | 13 | 130 | 15 | 89 |
| MCP のトーク 6 の後（R_POST） | 14 | 140 | 15 | 87 |
| 最後の 1 枚 | 15 | 150 | 15 | 87 |

### 報告の `threads=` の全系列（10 秒ごと）

| snap | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `threads=` | 15 | 15 | 16 | 15 | 16 | 15 | 15 | 15 | 15 | 15 | 15 | 15 | 15 | 15 | 15 |

- 報告の `threads=` は 15〜16 のあいだを動くだけで、16 になった 2 枚（`snap=3`・`5`）はその時点で喋っていたトークのスレッド（`sakura-talk-2`・`sakura-talk-3`）が生きた 1 本として名簿にいた枚。直す前（タスク 1）は同じ長さの走行で 16 から 27 へ増え続けていた。
- OS の数は 87〜93 の幅で上下するだけで、トーク 6 回と切り替え 1 回のあいだに増えなかった（最初が 91、最後が 87）。
- 差（OS − 報告）は 72〜77 の幅で、広がっていない。最初の対で 91 − 16 = 75、切り替えをまたぐ 6 対はどれも 89 − 15 = 74、最後の対で 87 − 15 = 72。

## 畳んだ行の抜き書き

`role=actor:sakura-talk-*` の畳んだ行は全 15 枚に出た。最初と最後の 1 枚:

```
2026-11-27T15:00:25.704503Z DEBUG areka::perf: perf(thread): スレッド別 CPU snap=1 t_s=10 tid=0 name=- role=actor:sakura-talk-* cpu_us=0 kernel_us=0 user_us=0 exited=1
2026-11-27T15:02:45.832537Z DEBUG areka::perf: perf(thread): スレッド別 CPU snap=15 t_s=150 tid=0 name=- role=actor:sakura-talk-* cpu_us=0 kernel_us=0 user_us=0 exited=12
```

`exited=` の全系列:

| snap | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `exited=` | 1 | 1 | 1 | 2 | 2 | 4 | 5 | 7 | 9 | 9 | 10 | 10 | 11 | 12 | 12 |

`exited=` はトークが終わるたびに増え、減らない。そのあいだ報告の `threads=` は増えていない（終わったトークのスレッドは名簿から外れて、系統の合計へ畳まれている）。

## 本数の突き合わせ

- **MCP で流した回数**: 7 回（トーク 6 回・切り替えの台本 1 回）
- **ログの登録の `debug!`**（target `wintf::thread_registry`・「スレッドを名簿へ登録した」）のうち、系統が `actor:sakura-talk-*` の行: **12 行**
- **最後の 1 枚（`snap=15`）で生きている `sakura-talk-<番号>`**: **0 本**（`snap=15` の `perf(thread)` に `role=actor:sakura-talk-<番号>` の行が無い）
- **12 − 0 = 12** で、畳んだ行の **`exited=12`** と合う

登録の 12 行と、トークを始めた行（`talk_id=`）の対応:

| ゴースト | 起こし方 | `talk_id` | 本数 |
|---|---|---|---|
| emo2 | 起動の挨拶（`boot_talk`） | 1 | 1 |
| emo2 | ゴースト自身の時刻のトーク（`steady_talk` origin=`OnSecondChange`） | 2・3・7 | 3 |
| emo2 | MCP の `sakurascript` のトーク（`sstp_send_started`） | 4・5・6 | 3 |
| emo2 | MCP の `sakurascript` の切り替えの台本（`sstp_send_started`） | 8 | 1 |
| R_POST | 切り替えで起きたときの挨拶（`boot_talk`） | 1 | 1 |
| R_POST | MCP の `sakurascript` のトーク（`sstp_send_started`） | 2・3・4 | 3 |
| 計 | | | 12 |

- MCP の 7 回は、emo2 の `talk_id=4`・`5`・`6`・`8` と R_POST の `talk_id=2`・`3`・`4` の 7 本にあたる。残りの 5 本は MCP から流していないトーク（起動と切り替え後の挨拶 2 本・時刻のトーク 3 本）。
- `talk_id` は起きたゴーストごとに 1 から数え直すので、R_POST の 4 本は emo2 と同じ名前（`sakura-talk-1`〜`4`）で別の TID のスレッドとして登録された。系統の合計は名前ではなく系統で畳むので、同じ名前が 2 度出ても `exited=` は 12 と数えた。
- `talk_id=3`（emo2 の時刻のトーク）は MCP のトーク `talk_id=4` に差し替えられた（`talk_done_stale` の行あり）。差し替えられたトークのスレッドも終わって畳まれている。
- 切り替えは `raise_event=false`（`--option=raise-event` なし）なので、送り出しの台詞のトークは起きていない。

## 判定: 合

- 対の数: 報告の `threads=` は 15〜16、OS の数は 87〜93 の幅で、どちらもトーク 6 回と切り替え 1 回に比例して増えなかった。差も広がらなかった（要件 7.8）。
- 畳んだ行: `role=actor:sakura-talk-*` の行が毎回出て、`exited=` が終わったトークの数だけ増えた（要件 4.4）。
- 本数: `exited=12` ＝ 登録の 12 行 − 生きている 0 本。名簿に残った `sakura-talk-<番号>` は 0 本で、タスク 1 の「トークのスレッドは OS の上では終わっていて、名簿の項目だけが残る」という判定を、終わったスレッドを 1 本ずつ畳んだ結果で直接確かめた。切り替えをまたいでも同じ（要件 1.5）。

## 残したもの

- 走行のログ: ワークツリーの `target\tc\logs-signoff\run.log`・`err.log`（`target\` の下なので、消えたら同じ手順で作り直す）
- 置き場: `target\tc\sw`
