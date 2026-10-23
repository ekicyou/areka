# 実機の記録（areka-P0-dot-sakura-specimen）

どっとさくら（`dot_sakura.nar`）を、検体の窓口が入れた根から areka の実機で 1 回起こした記録。手本は完了 `areka-P0-emily-ghost-verification` の `verification.md`。

**結論**: 華和梨の SHIORI は 32 ビットの載せ台で読み込めたが、areka が送ったすべての要求に状態 400 が返り、台詞は 1 つも出なかった。設計の 3 段の決め方の ⑵ で「SHIORI 2.x だけの見込み」となったので、要件 5.8 の枝に入り、残りの項目は「確かめていない」とした（本 spec の中では直さない）。同梱バルーン `bottle` は採られ、枠が出た。終了は `\-` の台本で通り、終了コードは 0。

## 1. 同定

| 項目 | 値 |
|---|---|
| 走行のときのコミット | `59b99e54f6ae6a20375cceb8027f5f3b4a28d9c4`（タスク 4.1 でビルドしたコミット `10893218` との差は `tasks.md` だけ。走行の後のコミット `3202c017` も `tasks.md` だけ） |
| ビルド | debug 版（`cargo build -p areka`・`dev` の形）。`C:\home\maz\git\areka\.claude\worktrees\areka-p0-dot-sakura-specimen-0f24e9\target\debug\areka.exe` |
| 32 ビットの補助プログラム | `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`（終了コード 0）。`areka.exe` の隣の `target\debug\shiori-host32-helper.exe` |
| 検体 | `vendors\sample_ghost\dot_sakura.nar`（1,425,534 バイト）。PowerShell の `Get-FileHash -Algorithm SHA256` は `0C602C6A431DCBEC48C02D934D98E342337D2DB9628095E0A6F90D3E83B7A472` |
| 取り寄せたときの値（タスク 1.1） | 2026-10-22 23:08:02（日本時間）に `target\dot-sakura-fetch\dot_sakura.nar` へ取り寄せ。1,425,534 バイト・SHA-256 `0c602c6a431dcbec48c02d934d98e342337d2db9628095e0a6f90d3e83b7a472`（brief の値と一致） |
| 取り出し直しの値（タスク 1.2・要件 1.4） | 書庫だけをコミット `5dc6288e` で入れた後、Git Bash の `git cat-file -p HEAD:vendors/sample_ghost/dot_sakura.nar \| sha256sum` が `0c602c6a…b7a472`（1.1 と一致・1,425,534 バイト）。`git check-attr text` は `unset`（改行の変換が効かない） |
| 配布物に入らないこと（タスク 1.2・要件 1.5） | `grep -rn dot_sakura tools/` は 0 件。配布物を組む `tools\package.ps1` が検体から取るのは `emo2` と `StayseeBalloon` だけ |

### 走らせる前の確かめ

- 登記の数の残り（タスク 2.1・要件 2.3）: `grep -rnE "9 つ|9 本|nine|8 行|8 本" crates/sample-ghost-kit/src/` で、検体の数として残る所は 0 か所。当たったのは `devroot_fetched_tests.rs` の「今の 9 本」（取り寄せ形の検体の数で、正しい）・`devroot_sweep_tests.rs` のスレッドの数の「8 本」・`lib_tests.rs` の冒頭の主張の数の「8 本」だけ（主張の数はタスク 2.2 の専用テストと一緒に直した）。
- 窓口と見張りのテスト（タスク 2.3・コミット `697c7c9a` の上・2026-10-22）:
  - `cargo test -p sample-ghost-kit` は終了コード 0。単体 83・`nar-sample-path` の bin 0・結合 `nar_sample_path_test` 6・ほかの結合 5 と 3、全部が通った。
  - `cargo test -p log-capture-kit --test sample_path_guard_test` は終了コード 0（21 本が通った）。
  - 書庫は断られず（要件 2.7 の枝に入らない）、同梱 `bottle` も入り（要件 2.8 の枝に入らない）、見張りも緑（要件 2.5・2.6 の枝に入らない）。
- 設計の形の 1 回（`cargo test -p sample-ghost-kit -p log-capture-kit`）: 実機の走行の**後**に、走行と同じコードのまま（`crates/` の下は `59b99e54` と同じ）回した。結果は下の「走行の後に回したテスト」。
- 32 ビットの補助プログラム（タスク 4.2・要件 5.2）: 机を取ってから、起動の直前に `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\areka-p0-dot-sakura-specimen-0f24e9\tools\sync-i686-helper.ps1` を回し、終了コード 0・出力は `helper is i686: target\debug\shiori-host32-helper.exe`。

### 走行の後に回したテスト

`cargo test -p sample-ghost-kit -p log-capture-kit`（2026-10-22・コミット `3202c017`・机は要らない crate のテスト）: 終了コード 0。テストの組 13 で、通った 219・落ちた 0・無視 2（無視の 2 は `log-capture-kit` の `capture_calibration_test` の `#[ignore]` の 2 本）。`sample-ghost-kit` の単体 83（専用テスト `dot_sakura_nar_installs_the_ghost_and_its_balloon_without_warnings` を含む）・結合 `nar_sample_path_test` 6、`log-capture-kit` の `sample_path_guard_test` 21 が通った。

## 2. 機械の構成

| 項目 | 値 | 測り方 |
|---|---|---|
| OS | Windows 11 Pro・版 26H2・ビルド 10.0.26300.9711 | `cmd /c ver` は `Microsoft Windows [Version 10.0.26300.9711]`。レジストリ `HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion` の `DisplayVersion`=26H2・`CurrentBuild`=26300・`UBR`=9711 |
| 画面 1（主） | 2880×1800・拡大率 200%（DPI 192）・左上 (0, 0) | えみりの記録と同じ測り方。PowerShell の中で、スレッドをモニターごとの DPI に合わせる形（`SetThreadDpiAwarenessContext(-4)`）にしてから `EnumDisplayMonitors`・`GetMonitorInfo`・`GetDpiForMonitor` で読んだ（2026-10-22・走行の後）。`HKCU:\Control Panel\Desktop\WindowMetrics` の `AppliedDPI` も 192。DPI を意識しない読み方では 1440×900 に見える |
| 画面 2 | 2560×1600・拡大率 150%（DPI 144）・左上 (-2560, 195)（主の画面の左） | 同上。DPI を意識しない読み方では 1707×1067・左上 (-1707, 130) に見える |

areka は起動のとき、主の画面の DPI から倍率を決めた（下の行・シェルとバルーンの作者の基準 DPI は 96）。

```text
2026-10-22T14:36:55.496326Z  INFO areka::placement: placement: 起動時 k₀ を導出（primary モニタ DPI ÷ 作者基準 DPI・D7） primary_dpi=192 shell_author_dpi=96 balloon_author_dpi=96 k_shell=2.0 k_balloon=2.0 k_shell_ratio=ScaleRatio { num: 2, den: 1 } k_balloon_ratio=ScaleRatio { num: 2, den: 1 }
```

## 3. 環境変数

パスはどれも `C:\` から始まる絶対パスで、ワークツリーの `target\` の下だけを使った（`C:\` の直下・`C:\tmp` は使わない・要件 5.1）。

| 名前 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT` | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-dot-sakura-specimen-0f24e9\target\nar-samples\manual\dot_sakura` | 検体の窓口（`nar-sample-path dot_sakura`）が入れた根 |
| `AREKA_PROFILE_DIR` | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-dot-sakura-specimen-0f24e9\target\dot-sakura-verify\profile` | 空の記憶から始める（起動中の印が無い） |
| `RUST_LOG` | `info,areka::boot_resolve=debug,areka_nar=debug,ghost-boot=debug,shiori-host32=debug,shiori-charset=debug` | 判定に使う分岐（ゴーストとバルーンの決め方・SHIORI の読み込みと補助プログラムの向き・文字コードの決め方）の記録が出る水準（要件 5.3）。面と描画の崩れは `info` で出る WARN と ERROR で拾う |
| `AREKA_APP_SMOKE_EXIT_MS` | `1800000`（30 分） | 安全弁。1 回の走行（実際は 2 分 14 秒）より長い |
| `NO_COLOR` | `1` | 記録に色の制御文字を混ぜない |
| `AREKA_NO_ALERT` | `1` | 失敗の告知のダイアログで走行が止まらないようにする（告知は記録に残る）。メニューからのインストールはしない |
| `AREKA_USER_ROOT` | 設定しない（起動の前に消した） | 本物の `%LOCALAPPDATA%\areka` に触らない。記録の `user_data_layout` は `mode="beside" reason="none" user_root=` |

### 判定に使う分岐の行が出たか

`RUST_LOG` で開けた先ごとに、走行の記録に行が出たかを数えた。記録の水準を開けて回し直すことはしていない（机は返した後）。

| 開けた先 | 出た行 | 読み方 |
|---|---|---|
| `areka::boot_resolve`・`areka::boot_config` | `ghost_resolved`・`balloon_resolved`・`session_mark_written`・`last_used_recorded`・`session_mark_cleared` | 出た。ゴーストとバルーンの決め方はこれで判定できる |
| `areka_nar` | 0 行 | 走行の中でインストールをしていない（根は窓口が前もって入れた）ので、出ないのが正しい。インストールの結果は窓口の専用テストで見る |
| `ghost-boot` | `charset_initial`（`charset="Shift_JIS" source="default"`）・`ghost boot sequence completed` | 出た |
| `shiori-host32` | 0 行 | この先の行は、補助プログラムが i686 でないとき（`helper_not_i686`・ERROR）か向きを確かめられないとき（`helper_image_unverified`・WARN）にだけ出る作り（`crates/shiori-host32-host/src/process_host.rs` の `report_helper_image`）。i686 のときは何も出さない。**`load` の成否を述べる行は、どの水準でも出ない作り**なので、読み込めたことは補助プログラムの標準エラーの 1 行と、その後の SHIORI の応答で見た（5 節） |
| `shiori-charset` | 0 行 | この先の行は、応答の `Charset` ヘッダで文字コードを替えたとき・強制の食い違いのときにだけ出る。全部の応答が 400 だったので、文字コードの切り替えは起きなかった |

### 根の形（タスク 4.1 で作り直した形）

`cargo run -q -p sample-ghost-kit --bin nar-sample-path -- dot_sakura` は終了コード 0 で、`root=C:\home\maz\git\areka\.claude\worktrees\areka-p0-dot-sakura-specimen-0f24e9\target\nar-samples\manual\dot_sakura` を印字した。

```text
target\nar-samples\manual\dot_sakura\
├─ ghost\
│  └─ dot_sakura\      （158 ファイル・直下に delete.txt・install.txt・readme.txt・updates.txt と ghost\・shell\）
└─ balloon\
   └─ bottle\          （38 ファイル）
```

- 計 196 ファイル。根の下は `ghost\dot_sakura` と `balloon\bottle` だけで、`emo2`・`StayseeBalloon` は置いていない。
- 一番深いファイルの絶対パスは 173 文字（`…\ghost\dot_sakura\ghost\master\dotvent_OpeningTalkDebug.txt`）で、260 文字に収まる。
- 走行の後、`ghost\dot_sakura` は 159 ファイルになった。増えた 1 つは areka がゴーストごとに書く記憶 `ghost\dot_sakura\ghost\master\profile\areka\sylphya.toml`（83 バイト）。
- 記憶・記録・撮った物の置き場は `target\dot-sakura-verify\` の下の `profile\`・`logs\`・`shots\`。

## 4. 走行

### 起動の命令

下の命令（`target\dot-sakura-verify\run1.ps1`）をそのまま裏で回した。どのフォルダで打っても同じに動く形で、`$wt` と `$v` は絶対パスだけから組む。同じ呼び出しの中で終わりを待ち、終了コードを書き出す（えみりの 1 回目で終了コードを取り損ねた反省）。

```powershell
$ErrorActionPreference = 'Stop'
$wt = 'C:\home\maz\git\areka\.claude\worktrees\areka-p0-dot-sakura-specimen-0f24e9'
$v = "$wt\target\dot-sakura-verify"
Remove-Item Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
$env:AREKA_ROOT = "$wt\target\nar-samples\manual\dot_sakura"
$env:AREKA_PROFILE_DIR = "$v\profile"
$env:RUST_LOG = 'info,areka::boot_resolve=debug,areka_nar=debug,ghost-boot=debug,shiori-host32=debug,shiori-charset=debug'
$env:AREKA_APP_SMOKE_EXIT_MS = '1800000'
$env:NO_COLOR = '1'
$env:AREKA_NO_ALERT = '1'
$p = Start-Process -FilePath "$wt\target\debug\areka.exe" -WorkingDirectory $v -RedirectStandardOutput "$v\logs\run1.stdout.log" -RedirectStandardError "$v\logs\run1.stderr.log" -PassThru
"pid=$($p.Id) start=$((Get-Date).ToString('yyyy-MM-ddTHH:mm:sszzz'))" | Set-Content "$v\logs\run1.pid.txt"
$p.WaitForExit()
"$($p.ExitCode)" | Set-Content "$v\logs\run1.exitcode.txt"
"exit=$($p.ExitCode) end=$((Get-Date).ToString('yyyy-MM-ddTHH:mm:sszzz'))"
```

### 1 回の走行（2026-10-22 23:36:52〜23:39:06・日本時間）

- コミット: `59b99e54`。areka の番号 37048（`logs\run1.pid.txt` は `pid=37048 start=2026-10-22T23:36:52+09:00`）。
- 机: 排他テストの机を 23:36:35 ごろに取り（`granted test`）、23:39:16 に返した（`returned test`）。
- 終了コード: 0（`logs\run1.exitcode.txt`）。終わった後、番号 37048 のプロセスは無かった（`Get-Process -Id`）。
- 記録: `target\dot-sakura-verify\logs\run1.stdout.log`（21,198 行）・`run1.stderr.log`（補助プログラムの 1 行 `[helper] SHIORI 初期化の入口: load`）。
- 操作: MCP の道具だけ。画面のクリック・SendInput・メニューからのインストール・ネットワーク更新はしていない。
- 撮った物: `target\dot-sakura-verify\shots\01-dump-balloon-scope0.png`・`02-dump-balloon-scope1.png`（MCP の `dump_balloon` の画像）。文書と PR には貼らない（要件 5.9）。

### 流れ（時刻は記録の UTC・日本時間は +9 時間）

| 時刻 | 起きたこと |
|---|---|
| 14:36:54.452 | 根を `AREKA_ROOT` から読んだ（`root_listed … origin="env_var" … found=true`） |
| 14:36:54.549 | ゴーストを経路 `Only` で `ghost\dot_sakura` に決めた（`ghost_resolved`） |
| 14:36:54.579 | バルーンを経路 `Companion` で `balloon\bottle` に決めた（`balloon_resolved`） |
| 14:36:54.592 | MCP の待受を始めた（`http://127.0.0.1:9801/api/mcp/v1`） |
| 14:36:55.496 | 倍率 2.0 を決めた（主の画面の DPI 192） |
| 14:36:55.533 | シェル `master` の透過の扱いを決めた（`treatment="none" declared=false`＝`seriko.use_self_alpha` の宣言なし） |
| 14:36:57.410 | シェル `master` の面の画像の一覧（`recognized=41 used=41 shadowed=0`） |
| 14:36:57.430〜.489 | バルーン `bottle` の面の系列を、1 人目は `balloons0〜3`、2 人目は `balloonk0〜1` と `balloons2〜3` に決めた |
| 14:36:57.617 | 自動終了の上限 1,800,000 ms を置いた |
| 14:36:57.628 | 起動中の印を書いた（`session_mark_written ghost="．さくら"`） |
| 14:36:58.221 | SHIORI の通信の初めの文字コードを Shift_JIS に決めた（`source="default"`） |
| 14:36:58.236 | ゴーストの起動の手順が終わった（`ghost boot sequence completed`） |
| 14:36:58.340 | 起動の指示を受けて `OnInitialize` を送った（`boot_start`） |
| 14:36:58.955 | `username` の要求に状態 400（以後、すべての要求が 400） |
| 14:36:58.959〜.961 | `OnFirstBoot`・`OnBoot` に状態 400 |
| 14:36:58.962 | 挨拶の台本が 1 つも無いまま、起動の記録だけのトークを流した（`epilogue-only 起動記録トークを再生起動（挨拶トーク皆無）`） |
| 14:36:58.965 | 起動の系列が終わり、ふだんの運転に入った（`boot_complete`） |
| 14:36:59.316 | `homeurl` の要求に状態 400 |
| 14:36:59.343〜.365 | 1 人目と 2 人目のバルーンの窓に面 0 を出した（`TargetId(1)`・`TargetId(3)`・素の大きさ 326×169 と 326×96） |
| 14:36:59.345 | `cursor.blendmethod ROP 系は M1 未対応` の WARN が出始めた（終わりまで 20,764 行） |
| 14:36:59.349〜.373 | バルーン `bottle` の文字の描画範囲が潰れて、このバルーンの文字は描かないと判断した（1 人目・2 人目） |
| 14:36:59.510〜 | `OnSecondChange` に毎秒 400（終わりまで 127 回） |
| 14:37:29.830〜.851 | MCP のつながり（観察の道具の呼び出し） |
| 14:39:03.908 | MCP の `raise_event OnClose` → `OnClose` に状態 400。MCP の答えは `OK:the ghost returned no script.` |
| 14:39:06.164 | MCP の `sakurascript` で `\-` を送った（`sstp_send_started talk_id=2`） |
| 14:39:06.167 | `OnTranslate` に状態 400（展開済みの元の台本のまま進んだ） |
| 14:39:06.176 | `talk_done_quit talk_id=2` |
| 14:39:06.206 | SHIORI をきれいに降ろした（`unload_clean`＝`unload` が正常に返った） |
| 14:39:06.249 | 全窓を閉じて終了を指示した（`app_exit origin=KanadeStopped(Quit) closed=4`） |
| 14:39:06.337 | 起動中の印を消した（`session_mark_cleared`） |
| 14:39:06.476 | MCP の待受を閉じた |

### MCP で取った物

| 道具 | 答え |
|---|---|
| `get_active_ghost_list` | `．さくら`（この走行の areka につながったことの確かめ。`ghost_resolved` の `dot_sakura` と、`session_mark_written` の `ghost="．さくら"` と同じゴースト） |
| `get_status` | 空（待機。話していない） |
| `get_property` | `baseware.name` は `areka`。`currentghost.name` などの `currentghost.*` は `NG:Cannot find such property name.`（areka の MCP の今の作り・8 節の 6） |
| `dump_surface`（1 人目・2 人目） | どちらも `NG:No surface has been shown in this scope yet` |
| `dump_balloon`（1 人目） | `bottle` の枠の画像（326×169）。薄い桃色の角の丸い四角に、右へ突き出たしっぽ、右下に薄い瓶の絵。文字は無い。四隅に抜き色の赤は見えない |
| `dump_balloon`（2 人目） | `bottle` の枠の画像（326×96）。1 人目と同じ作りの低い枠で、文字は無い。四隅に抜き色の赤は見えない |
| `raise_event OnClose` | `OK:the ghost returned no script.`（台本に `\-` は無い） |
| `sakurascript` `\-` | 受け付けられ、areka が終わった |

## 5. 項目ごとの結果

| 項目 | 結果 | 根拠 |
|---|---|---|
| インストールの結果と警告（`delete.txt`・`updates.txt` の扱い） | 出た（入った・警告 0） | 窓口の根に `ghost\dot_sakura`（直下に `delete.txt`・`updates.txt`・`readme.txt`・`install.txt` の普通のファイル）と `balloon\bottle` だけが在る（3 節）。起動の記録は `2026-10-22T14:36:54.549440Z INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Only dir=…\target\nar-samples\manual\dot_sakura\ghost\dot_sakura` と `2026-10-22T14:36:54.579299Z INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Companion dir=…\target\nar-samples\manual\dot_sakura\balloon\bottle`。警告は、専用のテスト `dot_sakura_nar_installs_the_ghost_and_its_balloon_without_warnings` が飛ばした同梱 0・警告 0・残った作業フォルダ 0・最上位の `delete.txt` と `updates.txt` が本体の直下の普通のファイル、を固定していて、走行の前（タスク 2.3）と後に回して通った。インストールの警告のダイアログは、飛ばした同梱が空のときは出ない手続き。走行の中ではインストールをしていない |
| 華和梨の SHIORI を 32 ビットの載せ台が読めたか | 出た（読めた） | 補助プログラムは i686（起動の直前の `sync-i686-helper.ps1` が `helper is i686`・記録の `helper_not_i686` は 0 件）。補助プログラムの標準エラーに `[helper] SHIORI 初期化の入口: load` が 1 行。その後、要求ごとに SHIORI の応答（状態 400）が返り（`2026-10-22T14:36:58.955569Z WARN … event="shiori_error_response" method="GET" id=username error=shiori error: SHIORI error status 400`）、終わりに `2026-10-22T14:39:06.205637Z INFO actor{actor=shiori}: shiori-actor: 正規 clean shutdown 完了（unload が正常終了を返した） event="unload_clean"`。読み込みの成否そのものを述べる行は出ない作り（3 節） |
| SHIORI の版（2.x か 3.0 か） | 出た（2.x だけの見込み） | 設計の 3 段で決めた。⑴ 起動のイベントでも `raise_event` でも台本は返らなかった（`epilogue-only … 挨拶トーク皆無`・MCP の `OK:the ghost returned no script.`）。⑵ `shiori_error_response` は 132 行で、全部が `SHIORI error status 400`（`username` 1・`homeurl` 1・`OnFirstBoot` 1・`OnBoot` 1・`OnSecondChange` 127・`OnClose` 1）。ほかに `OnTranslate` の 400 が 1 行（`translate_reply kind="error_response"`）。読めない応答（`Parse`）の行は 0。よって「全部の要求が 400」で「2.x だけの見込み」。areka は `GET … SHIORI/3.0` だけを送り、応答の版の字は読まない。`shiori.dll` を開いて読むことはしていない。**要件 5.8 の枝に入る**（6 節） |
| 起動したか（起動のイベントの台詞） | 出ない | 台本が返らず、台詞は出なかった。`2026-10-22T14:36:58.962630Z INFO actor{actor=kanade}: kanade: epilogue-only 起動記録トークを再生起動（挨拶トーク皆無） event="boot_talk" talk_id=1`。MCP の `get_status` は空。areka のアプリとしての起動の系列は `boot_complete`（14:36:58.965）まで通った |
| 1 人目と 2 人目の窓にシェル `master` の面 | 確かめていない | 要件 5.8 の枝（SHIORI 2.x だけの見込み）のため。走行の中で台本が 1 つも来ず、面を出す指示が無かった。MCP の `dump_surface` は 1 人目・2 人目とも `NG:No surface has been shown in this scope yet`。記録の面を出す行（`apply(ShowSurface)`）はバルーンの窓（`TargetId(1)`・`TargetId(3)`）の 2 行だけで、キャラクターの窓への行は 0。シェル `master` の面の画像は 41 枚が一覧に載った（14:36:57.410） |
| α の無い絵の抜き色の透過 | 確かめていない | 要件 5.8 の枝のため。シェルの面が 1 度も出ていないので、`dump_surface` の四隅を見られなかった。シェルの透過の扱いは `treatment="none" declared=false`（14:36:55.533・宣言なし）と決まったところまで。なお、`dump_balloon` のバルーンの枠では四隅に抜き色の赤は見えなかった（シェルの項目の根拠にはしない） |
| 同梱バルーン `bottle` が採られて出たか | 出た | 記録の `2026-10-22T14:36:54.579299Z INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Companion dir=…\balloon\bottle` と、`2026-10-22T14:36:57.430353Z INFO areka_emo_present::balloon: balloon: scope 別バルーン系列の解決が完了 balloon_dir=…\balloon\bottle scope=0 chain=["balloonp0def", "balloons"] faces=[(0, "balloons", "balloons0.png"), …]`。MCP の `dump_balloon` は 1 人目・2 人目とも `bottle` の枠（瓶の絵・文字なし・4 節）。ただし文字の描画範囲が潰れていて、文字は描かれない作りになっていた（8 節の 3）。`get_property` でバルーンの名前は取れなかった（`currentghost.*` が無い） |
| 終了まで通ったか | 出た（`\-` の台本で終わった） | `raise_event OnClose` には台本が返らなかった（`OnClose` も 400・MCP の答えは `OK:the ghost returned no script.`。`\-` は無い）。そこで `sakurascript` で `\-` を送り、`2026-10-22T14:39:06.176416Z INFO actor{actor=kanade}: kanade: reason=Quit——終了系列（Quit）へ event="talk_done_quit" talk_id=2` → `2026-10-22T14:39:06.249017Z INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=KanadeStopped(Quit) closed=4` → `session_mark_cleared`。終了コード 0。番号 37048 のプロセスは残っていない。**メニューの終了の道は確かめていない**（画面の操作をしない手順のため）。`OnClose` の台本に `\-` が無いことは崩れに数えない（ゴーストの側の作り・設計の決め） |
| ネットワーク更新 | 確かめていない | `updateurl` は開発者の古いサイトを指し、外へ出る操作は本 spec の目的の外（要件 5.11）。メニューも `\![updatebymyself]` も送っていない |

## 6. SHIORI が読めない・2.x だけのとき

この走行は要件 5.8 の枝に入った。

- その旨: 華和梨の SHIORI（`shiori.dll`・32 ビット）は載せ台で読み込めたが、areka が SHIORI/3.0 で送った要求に、全部が状態 400 で返った。設計の 3 段の ⑵ により「SHIORI 2.x だけの見込み」とする。
- 根拠: 5 節の「SHIORI の版」の行（`shiori_error_response` 132 行が全部 `status 400`・`OnTranslate` の 400 が 1 行・読めない応答 0 行・台本が 1 つも返らない）。
- 残りの項目: 「1 人目と 2 人目の窓にシェル `master` の面」と「α の無い絵の抜き色の透過」を「確かめていない」とした。台本で面を出す指示が来ないので、どちらも見る手立てが無い。
- 本 spec の中では直さない（要件 5.8・5.10）。崩れとして 8 節の 1 に書き、起票へ回す。

## 7. ERROR と WARN の数

記録の全 21,198 行のうち、ERROR 0・WARN 20,940・INFO 258・DEBUG 0。`panicked` は 0。

| 件数 | 水準 | 中身 | 数から外すか |
|---|---|---|---|
| 20,764 | WARN | `areka_emo_text::choice: cursor.blendmethod ROP 系は M1 未対応: none 扱い（色ベース描画）へ縮退 blendmethod="mergepennot"`。14:36:59.345 から終わりの 14:39:06.197 まで、1 秒に約 160 行 | 外さない（8 節の 2）。`bottle` の `descript.txt` の `cursor.blendmethod,mergepennot` が元だが、この警告はコードの注記で「解決時 1 回」と決めてあり、毎フレーム出るのは areka の側 |
| 132 | WARN | `kanade: SHIORI がエラー応答——返事なし（204）と同じ扱いで会話を続ける event="shiori_error_response"`（全部 `status 400`） | 外さない（8 節の 1） |
| 1 | WARN | `kanade: OnTranslate がエラー応答——展開済みの元の台詞で進む event="translate_reply" source=sstp-send kind="error_response"` | 外さない（8 節の 1 と同じ原因） |
| 22 | WARN | `areka_emo_compose::method: 未知の合成メソッド名: Unknown シームへ吸収 method=move`（シェル `master` の `surfaces.txt` の `animation0.pattern1,move,0,10,0,6` など） | 外さない（8 節の 4）。`move` は正典の pattern定義の語 |
| 5 | WARN | 同じく `method=alternativestart`（`animation2.pattern0,alternativestart,(0,1)` など） | 外さない（8 節の 5）。`alternativestart` は正典の pattern定義の語 |
| 2 + 2 + 2 | WARN | バルーン `bottle` の文字の範囲の 3 種（`文字の描画範囲が潰れている（幅か高さが 0 以下）——このバルーンの文字は描かない … left=0.0 top=0.0 right=0.0 bottom=149.0`・`origin に指定した文字の書き始めの位置が、文字を描いてよい範囲（validrect）の外にある`・`折返し基準が描画範囲の外に解決された`）。1 人目と 2 人目で 1 回ずつ | 外さない（8 節の 3）。3 種とも、`validrect.right,0` を右端からの距離でなく左端の 0 として読んだ結果から出ている |
| 6 | WARN | `areka_emo_present::balloon: balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 / 3 prefix=balloons file=balloons2.png / balloons3.png`（3 回ずつ） | 外す。`bottle` は 2 人目用の `balloonk2`・`balloonk3` の絵を持たない（`balloonk0`・`balloonk1` だけ）ので、本体側の `balloons2`・`balloons3` へ落ちる決まった縮退の道。バルーンの中身から出るもの（えみりの記録の `StayseeBalloon` の 6 件と同じ扱い） |
| 4 | WARN | `areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="surface12.png" / "surface22.png" original_w=228 original_h=338`（2 回ずつ） | 外す。どっとさくらの中身から出るもの。α の無い絵を左上の色で抜くと全部が透明になる絵（作者の誤りは繕わない決め・2026-10-05） |

外した残りは ERROR 0・WARN 20,930（全部が 8 節の 1〜5 のどれかから出ている）。

なお、タスク 4.2 の控えに書いた件数（`ROP` 16,403・`shiori_error_response` 124・`OnSecondChange` 121）は上の表と食い違う。上の表は、走行が終わった後の記録のファイル（21,198 行）を数え直した値で、こちらを正とする。

## 8. 崩れた所

扱いの欄は、タスク 5.1 で開発者と回して決めた持ち主（2026-10-23）。roadmap.md は開発者の指示で触っていない。

| 番号 | 症状 | 根拠 | 原因の見立て | 扱い |
|---|---|---|---|---|
| 1 | ゴーストが何も話さない。起動・時間の通知・`OnClose`・`OnTranslate` のどれにも台本が返らない | `shiori_error_response` 132 行（全部 `status 400`）と `translate_reply` 1 行（5 節）。`epilogue-only … 挨拶トーク皆無`（14:36:58.962）。MCP の `raise_event OnClose` が `OK:the ghost returned no script.` | 華和梨 7.4 は SHIORI/2.x の口だけを持ち、areka は SHIORI/3.0 の要求だけを送るので、全部を 400 で断られる見込み（設計の 3 段の ⑵）。華和梨・SHIORI 2.x への対応は本 spec の範囲の外 | 起票した: `shiori2-protocol` の件 1（2026-10-23 開発者と `/kiro-discovery`・開発者の条件は v0.1.0 までに治すこと・版の段は次の棚卸で決める） |
| 2 | `cursor.blendmethod ROP 系は M1 未対応` の WARN が毎フレーム出て、記録の 98% を占める（2 分 12 秒で 20,764 行） | 7 節の 1 行目。`crates/areka-emo-text/src/choice.rs` の注記は「`cursor.blendmethod` が ROP 系（`none` 以外）→ warn-once（解決時 1 回）」 | 解決が毎フレーム走り、1 回だけ出すはずの警告が毎回出ている（areka の側）。`mergepennot` を描けないこと自体は決めた縮退 | 既存の spec の brief に足した: `real-run-log-noise`（「どっとさくらで見えた事実」の節・開発者の条件は v0.1.0 までに治すこと） |
| 3 | バルーン `bottle` の文字の描画範囲が幅 0 に潰れ、このバルーンには文字が描かれない作りになった（台詞が来ないので画面では見ていない） | 7 節の文字の範囲の 3 種（14:36:59.349〜.373）。`… left=0.0 top=0.0 right=0.0 bottom=149.0 image_size=(326.0, 169.0)`。`bottle` の `descript.txt` は `validrect.left,0`・`validrect.right,0`・`validrect.bottom,-20` | areka は `validrect` の負の値を反対の辺からの距離と読み、0 は左端の 0 と読む（`crates/areka-emo-text/src/region.rs`）。`bottom,-20` は 149 に解けたが、`right,0` は 0 のまま。正典で 0 が右端を指すかは ukadoc に書かれておらず（`validrect.right` の既定は「不明」）、確かめていない | 既存の spec の brief に足した: `default-balloon-selfmade`（「どっとさくらで見えた事実」の節・正典の読みは要件の段で SSP の MCP で測る） |
| 4 | シェル `master` の pattern定義の `move` が未知の合成メソッドとして読み捨てられる | 7 節の `method=move` 22 行（14:36:57.902〜）。`surfaces.txt` の `animation0.pattern1,move,0,10,0,6` など | areka の合成メソッドの表に `move`（位置の語）が無い | 起票した: `seriko-pattern-calls` の件 2（`draw-methods-canon` の brief の議題から移した旨をあちらの brief に追記） |
| 5 | シェル `master` の pattern定義の `alternativestart` が未知の合成メソッドとして読み捨てられる | 7 節の `method=alternativestart` 5 行（14:36:57.897〜）。`surfaces.txt` の `animation2.pattern0,alternativestart,(0,1)` など | areka は、アニメーションから別のアニメーションを呼ぶ語（`alternativestart` の族）を持たない | 起票した: `seriko-pattern-calls` の件 1（アニメーションを呼ぶ 7 語の族ごと） |
| 6 | MCP の `get_property` で `currentghost.*`（ゴースト・シェル・バルーンの名前）が `NG:Cannot find such property name.` | 4 節の MCP の表。答えたのは `baseware.name`=`areka` だけ | areka のプロパティに `currentghost.*` の値がまだ無い | 既存の spec の brief に足した: `currentghost-property-others`（名前は `currentghost.汎用`＝開発者が持ち主を決めた・版の段は今のまま） |
| 7 | キャラクターの窓に面が 1 度も出ない（`dump_surface` が 1 人目・2 人目とも `NG:No surface has been shown in this scope yet`） | 4 節の MCP の表。記録の `apply(ShowSurface)` はバルーンの窓の 2 行だけ | 面を出すのが台本の `\s` だけで、台本が来ないと何も出ない見込み。SHIORI が何も返さないときに既定の面を出すかの正典は、この走行では確かめていない。1 が直れば見えなくなる見込みが高い | 起票した: `shiori2-protocol` の件 2（件 1 が直った後に実機で確かめる） |

扱いごとの件数（2026-10-23 確定）:

| 扱い | 件数 |
|---|---|
| 起票した | 4（1・4・5・7＝新しい spec 2 本） |
| 既存の spec の brief に足した | 3（2・3・6） |
| 方針で受けない | 0 |
| ゴーストの側の作り（繕わない） | 0 |

崩れに数えなかったもの:

- `OnClose` の台本に `\-` が無いこと（設計の決めでゴーストの側の作り。そもそも 1 のため台本が返っていない）。
- 7 節で外した WARN（バルーン面の縮退 6・全透明の element 4）。どちらもどっとさくらと `bottle` の中身から出る決まった道。
- 同梱バルーンの印（矢印・`online0〜15`・SSTP）と入力の箱 `balloonc*` は、台詞も入力も出ていないので症状を見ていない。見ていない物は「既知」として書かない（`balloon-markers`・`balloon-canon-residue`・`inputbox-user-input` へは足さない）。
- `alias.txt` の別名は、面が出ていないので症状を見ていない。

## 9. areka のコードの変更

`git diff --stat main...HEAD -- crates/`（2026-10-22・`HEAD` は `3202c017`）:

```text
 .../sample-ghost-kit/src/devroot_fetched_tests.rs  |   2 +-
 crates/sample-ghost-kit/src/lib.rs                 |   7 ++
 crates/sample-ghost-kit/src/lib_tests.rs           | 109 ++++++++++++++-------
 3 files changed, 83 insertions(+), 35 deletions(-)
```

`crates/` の下で変えたファイルは、許した 3 つ（`crates/sample-ghost-kit/src/lib.rs`・`lib_tests.rs`・`devroot_fetched_tests.rs`）だけ。要件 2.6 の枝（見張りの側の決まりの直し）には入っていないので、見張りのファイルは変えていない。どっとさくらを動かすための areka の本番のコードの変更は 0（要件 5.10）。
