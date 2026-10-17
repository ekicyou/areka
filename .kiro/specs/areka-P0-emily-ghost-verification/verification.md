# 検証の記録（areka-P0-emily-ghost-verification）

えみり（`emily4.nar`）を areka の実機で起こした検証の記録。各節は担当のタスクが埋める（同定・機械の構成・環境変数は 5.1、走行と崩れた所の表は 5.2〜6.2、数の洗い出しは 4.2）。

## 同定

準備（タスク 5.1）は 2026-10-17 22:21（日本時間）に行った。

| 項目 | 値 |
|---|---|
| コミット | `2af9e369f8449069bbb5219a1246b0a2292c2b26`（準備のときの `git rev-parse HEAD`。走行の前にコミットが進んだら、走行の節にその時のコミットを書く） |
| ビルド | debug 版（`cargo build -p areka`・`dev` の形）。`C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\debug\areka.exe` |
| 検体 | `vendors\sample_ghost\emily4.nar`（2,677,923 バイト）。sha256 `ebc87c6daa108116b9afdc6608059aa35ffbe183e0da83eb1b01888c824d39c0`（PowerShell の `Get-FileHash -Algorithm SHA256` で測った。`vendors/sample_ghost/README.md` に書かれた値と同じ） |

走らせる前の確かめ:

- 自動テスト: `cargo test -p areka-nar -p sample-ghost-kit -p log-capture-kit` は 436 本が通り、落ちた 0・無視 2（終了コード 0）。`cargo test -p areka` は 3,099 本が通り、落ちた 0・無視 2（終了コード 0）。
- 32 ビットの補助プログラム（要件 3.3）: `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc` の後に `pwsh -NoProfile -File C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\tools\sync-i686-helper.ps1` を回し、終了コード 0・出力は `helper is i686: target\debug\shiori-host32-helper.exe`。`areka.exe` の隣の `target\debug\shiori-host32-helper.exe` は i686 版。全体テストの後などで x64 版に替わることがあるので、走行の直前にもう一度回す。

## 機械の構成

| 項目 | 値 | 測り方 |
|---|---|---|
| OS | Windows 11 Pro・版 26H2・ビルド 10.0.26300.9711 | `cmd /c ver` と、レジストリ `HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion` の `DisplayVersion`・`CurrentBuild`・`UBR`（同じキーの `ProductName` は古い名残で「Windows 10 Pro」と出るが、ビルド 26300 は Windows 11） |
| 画面 1（主） | 2880×1800・拡大率 200%（DPI 192）・左上 (0, 0) | PowerShell の中で、プロセスをモニターごとの DPI に合わせる形（`SetProcessDpiAwarenessContext(-4)`）にしてから `EnumDisplayMonitors` と `GetDpiForMonitor` で読んだ。`HKCU:\Control Panel\Desktop\WindowMetrics` の `AppliedDPI` も 192 |
| 画面 2 | 2560×1600・拡大率 150%（DPI 144）・左上 (-2560, 195)（主の画面の左） | 同上 |

## 環境変数

areka が読む環境変数と、その値。パスはどれも `C:\` から始まる絶対パスで、ワークツリーの `target\` の下だけを使う（`C:\` の直下・`C:\tmp` は使わない・要件 3.1）。

| 名前 | 値 | 理由 |
|---|---|---|
| `AREKA_ROOT` | `C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\emily-verify\root` | 置き場の根。配布物と同じ形（下の木）。在ると書ける根（`%LOCALAPPDATA%\areka`）は並びに入らない |
| `AREKA_PROFILE_DIR` | `C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\emily-verify\profile` | areka の記憶の置き場（`crates/areka-user-data/src/lib.rs` の `PROFILE_DIR_ENV`）。準備の時点で空 |
| `RUST_LOG` | `info,areka::install=debug,areka::boot_resolve=debug,areka_nar=debug` | 判定に使う分岐（インストールの飛ばし・起動のゴーストとバルーンの決め方）の記録が出る水準（要件 3.4） |
| `AREKA_APP_SMOKE_EXIT_MS` | `1800000`（30 分） | 自動で終わる上限。1 回の走行の所要より長くし、安全弁としてだけ置く（要件 3.4） |
| `NO_COLOR` | `1` | 記録に色の制御文字を混ぜず、検索しやすい平文にする（`crates/areka/tests/smoke_boot_loop_exit.rs` と同じ扱い） |
| `AREKA_NO_ALERT` | 設定しない（起動の前に消す） | 設定すると警告のダイアログが出ない。インストールの警告（要件 1.12）を確かめるため |
| `AREKA_USER_ROOT` | 設定しない（起動の前に消す） | 書ける根を強いる変数。`target\debug\` は書けて winget の控えも無いので、設定しなければ書ける根は使わない形になり、本物の `%LOCALAPPDATA%\areka` に触らない（`crates/areka-user-data/src/lib.rs` の `decide`） |

記録の行き先: areka は記録を標準出力へ出す（`crates/areka/src/log_history.rs` の `init`）。起動の命令で `target\emily-verify\logs\` の下のファイルへ受ける。

根の形（準備の時点・上の 2 段）:

```text
target\emily-verify\
├─ profile\                  （空）
└─ root\
   ├─ ghost\
   │  └─ emo2\               （93 ファイル）
   └─ balloon\
      ├─ emo2-kakukaku\      （21 ファイル）
      └─ StayseeBalloon\     （29 ファイル）
```

`emo2` と `emo2-kakukaku` は `cargo run -q -p sample-ghost-kit --bin nar-sample-path -- emo2`、`StayseeBalloon` は `cargo run -q -p sample-ghost-kit --bin nar-sample-path -- StayseeBalloon` が開いた `target\nar-samples\manual\` の下から写した（`tools/package.ps1` の組み立てと同じ 3 つ）。えみりの `.nar` は根に写していない（名前に `emily` を含む物は 0 件）。走行の中で、メニューの「インストール…」から `C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\vendors\sample_ghost\emily4.nar` を入れる。

## 走行

### 起動の命令（予定・まだ回していない）

PowerShell から、どのフォルダで打っても同じに動く形。`$v` と `$w` は絶対パスだけから組む。起動したプロセスの番号を出すので、止めるときはこの番号だけを使う（要件 3.7）。

```powershell
$w = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680'
$v = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-areka-p0-dev-cad680\target\emily-verify'
pwsh -NoProfile -File "$w\tools\sync-i686-helper.ps1"; "sync exit=$LASTEXITCODE"
$null = New-Item -ItemType Directory -Force -Path "$v\logs"
Remove-Item Env:AREKA_NO_ALERT, Env:AREKA_USER_ROOT -ErrorAction SilentlyContinue
$env:AREKA_ROOT = "$v\root"
$env:AREKA_PROFILE_DIR = "$v\profile"
$env:RUST_LOG = 'info,areka::install=debug,areka::boot_resolve=debug,areka_nar=debug'
$env:AREKA_APP_SMOKE_EXIT_MS = '1800000'
$env:NO_COLOR = '1'
$p = Start-Process -FilePath "$w\target\debug\areka.exe" -WorkingDirectory $v -RedirectStandardOutput "$v\logs\run1.stdout.log" -RedirectStandardError "$v\logs\run1.stderr.log" -PassThru
"areka pid=$($p.Id)"
```

2 回目の走行（直した後・要件 5.6）は、根と記憶を作り直してから、記録のファイル名を `run2.*` にして同じ命令で回す。

## 崩れた所の表

## 数の洗い出し

タスク 4.2 で、ソースと文書の全体を `14 種`・`14 変種`・`fourteen`・`7 つ`・`7 本`・`seven` で検索した（2026-10-17・`git grep -n -I -E "14 種|14 変種|fourteen|7 つ|7 本|seven"`）。本 spec で変わった数は 2 つ——拒否の閉じた語彙（14 種 → 13 種）と、登記した検体の数（7 → 8）。外した場所を除いて 183 行が当たり、全部を下のどれかに片付けた（直した 2 行は直した後は当たらないので、183 行はどれも、この数とは別の数か、変わったことそのものを書いた行）。**片付いていない行は 0 件。**

### 検索から外した場所と理由

- `.kiro/specs/completed/` の下: 終わった spec の当時の記録で、書き直すと当時の事実が消える。上書きした完了 spec の要件は、書き直す代わりに `doc/COMPAT_ARCHITECTURE.md` §8 に番号つきで書いた（要件 6.5・8.2）。
- 本 spec のフォルダ（`.kiro/specs/areka-P0-emily-ghost-verification/`）: 「14 種 → 13 種」「7 → 8」という変化そのものを書いている文書なので、古い数が出るのが正しい。
- `target/`: ビルドの出力で、追跡していない（`git grep` は見ない）。
- `vendors/` の下: 当たりは 0 件だった（検体の README も含めて）。

### 直した行

- `doc/COMPAT_ARCHITECTURE.md` §8「インストールの失敗理由の語の写し」の行: 「拒否の 14 種」と「14 種を 1 つずつ組んで」を 13 種へ。`invalid type` の並びから `CompanionSourceMissing` を外した。
- `doc/ukadoc-coverage/ledger/shiori.toml` の `OnInstallFailure` の備考: 「areka-nar の拒否の 14 種」を 13 種へ。
- タスク 1.2・2・4.1 で直した分（今は当たらない）: 語彙のテスト・`error_tests.rs`・`judge_tests.rs` の 14 → 13、`procedure.rs` の口の数 7 → 8、`sample-ghost-kit` の登記のテストの 7 → 8。検索で、これらの古い数が残っていないことを確かめた。

### この数とは別の数（残した行と理由）

どれも、拒否の語彙の数でも、登記した検体の数でもない。

**ソース（`crates/`・`tools/`・`.claude/`）**

| 場所 | 数えているもの |
|---|---|
| `crates/areka-nar/src/error_tests.rs:372`（`manifest_warnings_are_seven_and_carry_their_key`） | 記録だけを残す警告（`ManifestWarning`）の種類 7。拒否の語彙とは別の型で、本 spec は警告の種類を足していない（飛ばした同梱は結果の別の欄に載せた）ので 7 のまま |
| `crates/areka/src/install/procedure_branch_tests.rs:336`・`crates/areka/src/install/fetch_url_tests.rs:144`（`..._seven_days`） | 日数 7 日（残った中身を消すまでの日数・古い落とし物を消す日数） |
| `crates/areka/src/update/refs_tests.rs:421`（`unsent_seven_words_never_appear`） | 更新の知らせで送らない語 7 つ |
| `crates/areka/src/log_history_tests.rs:139`（`..._fourteen_rows_in_order`） | 記録の履歴の規則の表の 14 行 |
| `crates/areka-parsers/src/balloon/parse_tests.rs:861`（`font_base_key_table_has_fourteen_entries...`） | バルーンの descript の書体の鍵 14 |
| `crates/sample-ghost-kit/src/devroot_sweep_tests.rs:501` | 同じ検体を取る 8 本のスレッドのうち、残りの 7 本（検体の数ではない） |
| `crates/areka/src/mcp/mcp_tests.rs:27・143・146` | MCP の道具のうち、相手を省略できる 7 本 |
| `crates/areka/src/mcp/get_property_tests.rs:6` | 聞くプロパティの名前 7 つ |
| `crates/areka/src/mcp/get_expression_table_tests.rs:343`（「検体 7」・上の 6 語では当たらず、検体の数の言い回しを広げて探したときに出た行） | 表情の表のテストの見本の 7 番目（登記した検体ではない） |
| `crates/areka/src/emo2_boot/spine_conformance_script.rs:618・694・714`・`crates/areka-kanade/src/schedule/events_tests.rs:350・443` | マウスのイベントの Reference 7 つ |
| `crates/areka/src/ghost_session_switch_fallback_tests.rs:420`（`Ref6/7`） | Reference の番号 6 と 7 |
| `crates/areka/src/emo2_boot/user_break_cue.rs:70` | 起動の受け口の 7 本目 |
| `crates/areka/src/emo2_boot/balloon_visibility_phase_reappear_tests.rs:10・267`・`crates/areka/src/emo2_boot/frame_balloon_offset_follow_tests.rs:42・47`・`crates/areka/src/placement/spawn_zorder_pair_deferred_tests.rs:9`・`crates/wintf/src/ecs/window/zorder_chain.rs:355` | そのファイルの中のテストの本数 |
| `crates/areka/src/placement/placement_prepare_tests.rs:343・344・487・491`（`seven_sixths`） | 拡大率 7/6 を入れた変数の名前 |
| `crates/areka-emo-atlas/src/animated_tests.rs:432` | 動く絵を縮める理由 7 つ |
| `crates/areka-emo-compose/src/nesting_report_tests.rs:80` | アニメーションの ID の語 7 つ |
| `crates/areka-emo-text/src/state_cluster_tests.rs:34` | 絵文字の部品の数 7 |
| `crates/areka-emo-text/tests/decoration_readback_test.rs:102`・`crates/areka-emo-text/tests/decoration_readback/mod.rs:78` | DirectWrite が範囲で持てる飾りの鍵 7 つ |
| `crates/areka-ghost/src/catalog_tests.rs:69` | ゴーストの素性の項目 7 つ |
| `crates/areka-mcp-bridge/src/service_tests.rs:188・189` | 要求の id に使った文字列 `"seven"` |
| `crates/areka-seriko/src/looper_film_count_tests.rs:66` | 16 ミリ秒の刻み 7 回 |
| `crates/wintf/src/ecs/window/command_batch_tests.rs:206`・`command_transition_tests.rs:140`・`zorder_chain_diag_tests.rs:101` | 窓の位置の引数 7 つ・窓を作る引数 7 つ・重なりの順の札 7 つ |
| `crates/ukadoc-survey/` の 17 行（`catalog/mod.rs:5`・`documents/derive_tests.rs:445`・`evidence/candidates.rs:512`・`evidence/resolve.rs:307`・`ledger/read_tests.rs:439`・`model.rs:238・278`・`model_tests.rs:127`・`report/tally_tests.rs:112・197`・`tests/consistency/briefing_arms.rs:717`・`checks.rs:765・771`・`linkage_checks.rs:6・23・508`・`spec_checks.rs:39`） | 網羅台帳の道具の数（状態の綴り 7 つ・検査の腕 7 つ・必須の欄 7 つ・正典の URL 7 つ・逃がし文字 7 つ・完了 spec の持ち主の宛先 14 種） |
| `tools/perf/judge-perf.py:260・1287・1431` | 「267 本」の一部（間隔の本数） |
| `.claude/agents/perf-analyze.md:39` | スケジュールの 7 本 |

**文書**

| 場所 | 数えているもの |
|---|---|
| `doc/COMPAT_ARCHITECTURE.md:287・290・300` | テストの名前の中の 7 日（`..._seven_days`）と、送らない語 7 つ（`unsent_seven_words_never_appear`） |
| `doc/COMPAT_ARCHITECTURE.md:333`（本 spec で足した行） | 「14 種 → 13 種」と、変わったことそのものを書いている |
| `README.md:95`・`doc/CONSTITUTION.md:76` | areka のエンジン 7 つ |
| `README.md:107`・`doc/release-notes/v0.0.2.md:21` | MCP の道具 10 本のうち動く 7 本 |
| `doc/impl-watch.md:27・43` | 机の決まり 7 つ |
| `doc/ssp-mcp/areka-tools.md:65` | 台本の診断の種類 7 つ |
| `doc/roadmap-overview-2026-10-16.html:70` | 10-16 時点のロードマップの写し（檻 7 本・Reference 7 つ・申し送り 7 本） |
| `doc/ukadoc-coverage/README.md:144` | 台帳の状態の綴り 7 つ |
| `doc/ukadoc-coverage/briefing-assets.md:381〜485`（「実例に使う 7 本のファイル」） | emo2 の系統の検体の中にある、正典の書式で書かれたファイル 7 本（検体の数ではない。同じ節が「検体 `R_POST_and_KOMAINU.nar` は数えていない」と断っている） |
| `doc/ukadoc-coverage/briefing-assets.md:814・816` | バルーンの書体の正典の見出し 14 種 |
| `doc/ukadoc-coverage/briefing-property.md:955` | 「7,007 本」の一部（テストの本数） |
| `doc/ukadoc-coverage/briefing-sakura-script.md:8` | 説明書の節 7 つ |
| `doc/ukadoc-coverage/briefing-shiori.md:59`・`doc/ukadoc-coverage/ledger/shiori.toml:30` | 優先度が空の群 7 つ |
| `doc/ukadoc-coverage/briefing.md:1725・1746・2435・2592`・`doc/ukadoc-coverage/roadmap-draft.md:1489・1494` | 「27 本」の一部（spec の本数） |
| `doc/ukadoc-coverage/ledger/property.toml` の 13 行（177・192・1496・1526・1559・1729・1764・2277・2292・2658・2673・2688・2703） | プロパティの名前のルートの枝 7 本 |
| `doc/ukadoc-coverage/linkage.md:153・2406` | 束の残り 7 本・束の欄 7 つ |
| `.kiro/steering/product.md:63`・`.kiro/steering/roadmap.md` の 13 行 | ウェーブ C8 の 7 本・spec の本数（107 本 など）・メニューの枠 7 つ・Reference 7 つなど |
| `.kiro/steering/roadmap-history.md` の 31 行 | 過去のウェーブの記録（その時点の spec・テスト・ファイルの本数）。拒否の語彙と検体の数を言う行は無い |
| ほかの spec の `brief.md` の 28 行（`check-script-arg-checks`・`coverage-roadmap-refresh`・`property-name-case-fold` ほか、19 本の spec） | その spec の数（診断の種類 7 つ・メニューの枠 7 つ・「バルーンのリンクと OS の連携」の 7 本・テストの本数など）。拒否の語彙と検体の数を言う行は無い |

### 配布物の許可表

`tools/package.ps1` の許可表は `ghost/` の下が `emo2` だけ、`balloon/` の下が `emo2-kakukaku` と `StayseeBalloon` だけで、`emily` の綴りは 0 件（大小を無視して数えた）。えみりは配布物に入らない（要件 2.7・変更 0）。
