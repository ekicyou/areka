# ギャップ分析: areka-P0-real-run-mcp-kit

- 分析日: 2026-10-18（ブランチ `claude/areka-p0-real-run-mcp-c9b8bd`・main `e2370df8` の上・要件は生成済み・未承認）
- 読んだもの: `requirements.md`・`brief.md`・`spec.json`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C9 の行 ⑥）・実機の試験 3 本・`sample-ghost-kit`・`temp-path-kit`・`areka-mcp-bridge`（`http.rs`・`scan.rs`・`service.rs`・`config.rs`・`tests/real_receiver_test.rs`）・`areka-mcp`（`server.rs`・`port.rs`）・`areka/src/app_exit.rs`・wintf の `WM_CLOSE` の受け手・`log-capture-kit/tests/` の見張り 3 本・`tools/sync-i686-helper.ps1`・`tools/test-all.ps1`・完了 spec の実機の記録（`taskkill` の先例）
- 本書は判断の材料と選択肢を並べる。どれを採るかは要件の討議と設計で決める。

---

## 1. 分析の要約

- **足場の部品はほぼ全部ワークスペースに在る**。MCP の呼び出しは中継の `areka_mcp_bridge::scan::scan`（公開の関数・rmcp のクライアントを自前の HTTP の送り手で立てて `Link` を返す）がそのまま使え、偽の受け口は中継の通しのテストと同じく本物の `areka_mcp::start(&[0], 登録表, None)` に偽のツールを 1 本載せれば作れる。閉じ方は `taskkill /PID <番号>`（強制なし）で `on_ghost_os_close` の道へ乗ることが、完了 spec の実機の記録（`mcp-get-status`・`mcp-author-tools` の signoff）で 2 度確かめられている。
- **いちばん大きい穴は「検体の窓口を本番の依存から締め出す見張り」との折り合い**。見張り（`sample_path_guard_test.rs` の `the_sample_gateway_never_appears_in_a_production_dependency_table`）は、ワークスペースの全クレートの `[dependencies]` に `sample-ghost-kit` が載ることを違反とする。新しいクレートが検体の名前で起動する（要件 1.1 ⑴）には `[dependencies]` に `sample-ghost-kit` を書くことになり、**今の見張りがそのまま赤になる**。見張りの直し方は設計の判断（下の議題 1）。
- **記録の段を絞った走り（`RUST_LOG=warn,…`）では、待受を始めた行（`info`）と終わり方の行（`[quit_app]` の `info`）が標準出力に出ない**。要件 4.4（絞っていても待受の始まりを観測）と 5.3（閉じ方で終わったか自動終了で終わったかの見分け）は、窓口が記録の段に手を入れるか、記録以外の観測（つないで確かめる）を使うかの判断が要る。
- **窓口のライブラリは `env!("CARGO_BIN_EXE_areka")` を自分では展開できない**（その名前は areka のクレートの統合テストの中でだけ使える）。`areka.exe` の場所を呼び手から受け取る形が要る。
- 規模は **M**（新しいクレート 1 つ・移し替え 3 本・見張りの直し・手引きの 1 節・決定論のテスト）、危うさは **中**（閉じる知らせを送ってよい時点・見張りの直し・並走の spec が `sample-ghost-kit/src/devroot.rs` を触ること）。

---

## 2. 今ある資産（Current State）

### 2.1 実機の試験 3 本の今の足場

| 足場 | `emo2_real_run.rs`（249 行） | `mcp_get_log_real_run.rs`（348 行） | `smoke_boot_loop_exit.rs`（508 行） |
|---|---|---|---|
| 走る条件 | `AREKA_EMO2_REAL_RUN` を明示したときだけ | 常時 | 常時（5 本の `#[test]`） |
| 検体 | `LazyLock` でプロセスの間 1 つ保持 | 試験ごとに取得 | 試験ごとに取得 |
| 32bit の仲介 | **揃えない**（手で置く前提） | 自前の `is_i686_pe`・`ensure_helper_beside_areka`（57・71 行） | 自前の `is_i686_pe`・`ensure_i686_artifact`・`ensure_helper_beside_areka`（147・162・204 行・`OnceLock` で 1 回） |
| 試験用 32bit の DLL | なし | なし | `ensure_i686_artifact("shiori_loadu.dll", "HOST32_TESTDLL_LOADU_DLL", …)`（④ 失敗方向） |
| 環境変数 | `AREKA_USER_ROOT` だけ外す（`AREKA_ROOT`・`AREKA_PROFILE_DIR` は開発者のものが入る） | `AREKA_ROOT`・`AREKA_USER_ROOT` を外し、`AREKA_PROFILE_DIR` を一時の置き場へ | `AREKA_ROOT`・`AREKA_PROFILE_DIR`・`AREKA_USER_ROOT` を外し、方向ごとに足す（① は記憶の置き場を指定しない＝**開発者の本物の記憶の置き場を使う**） |
| MCP の番号 | 指定なし（既定の 9801 から早い者勝ち） | `127.0.0.1:0` で空きを得て放し、`AREKA_MCP_PORT` へ | 指定なし（並走する ①②④ の子が 9801 を取り合い、後の子は隣へ移る） |
| 子の出力 | パイプ。終わるまで読まずに `try_wait` で見回る | ファイル（`CARGO_TARGET_TMPDIR` の下） | パイプ。終わるまで読まずに `try_wait` で見回る |
| 終わり方 | 自動終了 3 秒を待つ（番犬 120 秒） | 答えの後も自動終了 20 秒を待つ（番犬 60 秒） | 自動終了 500 ms を待つ（④ は 20 秒・番犬 60 秒） |
| 一時の置き場 | なし | `target\tmp\mcp-get-log-<pid>`（固定名＋番号） | `TempPath::new`（**OS の一時フォルダ**）と `TempPath::under_target`（⑤ だけ） |
| MCP の呼び出し | なし | 手書きの HTTP/1.1・手書きの JSON の復号（`decode_json_string`） | なし |
| モニタ 0 台の受理 | なし（実表示の前提） | 「起動窓を開けません」＋「モニタ列挙に失敗」で非 0 を受理 | 同じ判定を `accepted_as_no_monitor` に持つ |

気づいた点（移し替えで直るもの・直すと何かが変わるもの）:

- **パイプのまま終わるまで読まない形は、出力が多いと子が書けずに止まる**（パイプの容量を超えた時点で子の書き込みが待ちになり、番犬の締切まで終わらない）。`emo2_real_run` は `kanade=trace` を開けているので出力が多い。窓口がファイルへ流す形（要件 3.1）にすると、この恐れが消える。
- `smoke` の `ensure_i686_artifact` は探すときに `p.is_file()` で判定している（`mcp_get_log` は `is_i686_pe`）。x64 の `shiori_loadu.dll` が debug に在ると、それを拾ってしまう。要件 2.5 は「同じ規則（32bit か）」なので、窓口では 32bit の判定で揃う。
- 移し替えると、`emo2_real_run` は開発者の `AREKA_ROOT`・`AREKA_PROFILE_DIR` を受け継がなくなり（要件 1.5）、MCP の番号も空いた番号になる。`smoke` ① も開発者の本物の記憶の置き場を使わなくなり、②③④ の一時の置き場は OS の一時フォルダから `target\` の下へ移る。**確かめの中身（要件 7.2）は変わらないが、子の環境は変わる**。どれも「開発者の環境に左右されない」方向の変化。

### 2.2 検体の窓口（`sample-ghost-kit`）

- `SampleRoot::acquire(名前)` が、`<target>\nar-samples\work\<pid>-<連番>\` に原本の写しを作って配る。読み口は `root()`・`folder()`・`balloon(名前)`（全部借用）と `add_shell_copy`。値を捨てると `WorkDir` の `Drop` が木と生存の札（`.lock`）を消す。
- **値を残す口（`keep`・`persist` のたぐい）は無い**。`std::mem::forget` で値を捨てずにおくと札のファイルの取っ手もプロセスの終わりまで開いたままになり、プロセスが終わった後の次の取得の掃除（`sweep`）が札を消せて、木ごと片付けてしまう＝**証拠として残らない**。
- `WorkDir` の後始末は、木が既に無い（`NotFound`）ことを「目的が達成された」として黙って通す（`devroot.rs` の `report_cleanup`・`release_with` の説明「原本として `cache/` へ移した後は木が既に無い」）。だから、**子が終わった後に窓口が写しのフォルダを別の場所へ移せば（`rename`）、`SampleRoot` の破棄は札だけを消して静かに終わる**。`lib.rs` には触らずに済む。ただしこれは `devroot.rs` の中のふるまいに頼る形で、同じウェーブの ⑤ `sample-url-fetch` は `sample-ghost-kit/src/{lib,lib_tests,devroot}.rs` を触る（roadmap の C9 の行）。
- 写しの置き場の `target` は `CARGO_TARGET_DIR` → 実行ファイルの祖先の `target` の順で決まる（`devroot::find_target_dir`）。

### 2.3 一時パスの窓口（`temp-path-kit`）

- `TempPath::under_target(札)` が `<ワークスペース>\target\test-roots\areka-<札>-<pid>-<連番>` を作り、`Drop` で消す。中身は `PathBuf` 1 つだけなので、`std::mem::forget` すれば**消えずに残る**（取っ手を持っていないので副作用が無い）。
- 置き場の `target` は `temp-path-kit` の `CARGO_MANIFEST_DIR` の 2 つ上から組む（`CARGO_TARGET_DIR` は見ない）。検体の写しの `target` と食い違う場合がある（`CARGO_TARGET_DIR` を別のドライブへ向けたとき）。

### 2.4 MCP の呼び出しの先例（`areka-mcp-bridge`）

- `lib.rs` が `pub mod http; pub mod scan; pub mod service; pub mod config;` を出している。
- `scan::scan(&[番号], &Timeouts)` が、つないで名乗り（`areka-mcp-server`）を確かめ、rmcp のクライアント（`RunningService<RoleClient, BridgeClient>`＝`Link`）を立てて返す。`Link` の `peer()` に rmcp 3.5.0 の `call_tool(CallToolRequestParams)` があり、答えは `CallToolResult`（`content` と `is_error`）。
- `Timeouts`（つなぐ・確かめる・答えを待つ の 3 つ）は公開で、差し替えられる。
- 依存は `rmcp =3.5.0`（`client`・`transport-streamable-http-client`）・`tokio`・`futures`・`http`・`sse-stream`・`httparse`・`serde_json`。**どれも `Cargo.lock` に在る**。窓口が中継のライブラリに依存すれば、窓口の `Cargo.toml` に足すのは `areka-mcp-bridge`・`tokio`（実行の器）・必要なら `rmcp`・`serde_json` で、外の依存は増えない。
- 偽の受け口の先例: 中継の `tests/real_receiver_test.rs` が、本物の `areka_mcp::start(&[0], registry, None)` にツールを 1 本（`is_error` を切り替えられる）登録して使っている。窓口の決定論のテストも同じ形が使える。

### 2.5 MCP の待受の記録の行（`areka-mcp/src/server.rs`）

| 場面 | 行 | 段 |
|---|---|---|
| 待受を始めた | `MCP: 待受を始めた`（`url=`）／先の候補が使用中なら `MCP: 待受を始めた（先の候補が使用中なので移った）` | `info` |
| 束ねられなかった | `MCP: どの候補のポートも束ねられなかった（待ち受けない）`（`tried=`・`error=`） | `error` |
| 待ち受けない設定 | `MCP: 待ち受けない（AREKA_MCP_PORT が 0）` | `info` |

- 記録の出所の名前は `areka_mcp::server`。`AREKA_MCP_PORT` に 1 つの番号を渡すと候補はその 1 つだけで、取られていれば `error` の行が出て待ち受けない（要件 4.5・9.2 の観測点）。
- areka の標準出力の層は `RUST_LOG` で絞られ、履歴の層は絞られない（`log_history.rs`）。`RUST_LOG=warn,areka::boot_config=info`（今の `mcp_get_log`）では「待受を始めた」の行は**標準出力に出ない**。
- 待受を始めた行・取られていた行の文は公開の定数ではない（`areka-mcp` の中の文字列）。窓口が文を写すと、文が変わったときに黙ってずれる。

### 2.6 閉じ方（`app_exit.rs`・wintf の `WM_CLOSE`）

- ゴーストの窓（キャラ窓・バルーン窓）が `WM_CLOSE` を受けると、wintf の受け手が窓に差された `OnCloseRequest` を呼び、areka の `on_ghost_os_close` がメニューの「終了」と同じ終了の要求（`CloseReason::User`）を kanade へ送る → `OnClose` → 別れの台詞 → `quit_app(KanadeStopped(Quit))` → 終了コード 0。kanade と結線していない起動では `quit_app(OsClose)` で直ちに閉じる。
- `OnCloseRequest` は、窓に HWND が付いた後の `FrameFinalize` の段で `attach_os_close_request` が差す。**差される前に `WM_CLOSE` が届いた窓は、従来どおり窓 1 枚だけ消える**（wintf の受け手の「持たない窓は despawn」の腕）。閉じる知らせを送ってよい時点の見極めが要る。
- 外のプロセスからの送り方の先例: 完了 `mcp-get-status`・`mcp-author-tools` の実機の記録で、自分が起こした pid にだけ `taskkill /PID <番号>`（強制なし）を送り、終了の挨拶 → 「きれいに終わったので起動中の印を消しました」→「待受を閉じた」で終わったことが記録されている。`taskkill`（強制なし）はそのプロセスの最上位の窓すべてへ `WM_CLOSE` を送るので、areka には終了の要求が窓の数だけ届くが、実機では問題なく終わっている。
- 子のプロセスの取っ手（`std::process::Child`）を持っている間は、Windows はその番号を別のプロセスへ使い回さない。だから「`try_wait` でまだ生きていることを見てから、その番号へ送る」形なら、窓口が起こした子以外へ届く恐れは無い（要件 5.6）。
- 終わり方の記録: `[quit_app] 全窓を閉じ、終了を指示した`（`event="app_exit"`・`origin=Smoke` か `origin=KanadeStopped(Quit)` か `origin=OsClose`）。段は `info`、出所は `areka::app_exit`。`RUST_LOG=warn` では標準出力に出ない。

### 2.7 見張り（`log-capture-kit/tests/`）

- `sample_path_guard_test.rs`: ⑴〜⑷ の綴り（展開先の名前空間 `nar-` `samples` を含む）を `crates/sample-ghost-kit/src/` の外で禁じ、`sample-ghost-kit` がどのクレートの本番の依存表（`[dependencies]`・`[build-dependencies]`・`[target.….dependencies]`）にも載らないことを見る。除外するのは `sample-ghost-kit` 自身の `Cargo.toml` だけ。**新しいクレートの `[dependencies]` に載せると違反**。また、新しいクレートを areka の `[dependencies]` に載せても今の見張りは気付かない（直に `sample-ghost-kit` を名指ししていないため）＝要件 10.3 の「同じ強さ」を保つには、新しいクレートも締め出す必要がある。
- `temp_path_guard_test.rs`: `std::env::temp_dir` の呼び出しを窓口の外で禁じる。窓口が `target\` の下だけを使えば当たらない。
- `file_length_guard_test.rs`: 1 ファイル 1,000 行。窓口は役ごとにファイルを分ける必要がある。
- 見張りのファイルは `log-capture-kit/tests/` にあり、C9 のほかの 7 本の触るファイルには挙がっていない（roadmap の C9 の行を `guard`・`log-capture-kit` で引いて 0 件）。ただし ⑥ の触るファイルの欄にも挙がっていないので、触るなら roadmap の欄の追記が要る。

### 2.8 32bit の仲介の揃え方

- 規則は 2 か所に写しがある（`mcp_get_log`・`smoke`）。全体テストの最後の段は別に PowerShell の `tools/sync-i686-helper.ps1`（同じ規則の自分の検査つき）で隣を揃える。Rust の本番の判定 `classify_helper_image`（`shiori-host32-host/src/process_host.rs`）は私有で使えない。
- 書きかけを起動させない形（一時名に写してから改名）は 2 本とも持っている。同じプロセスの中で 1 回にする形（`OnceLock`）は `smoke` だけ。

---

## 3. 要件ごとの対応表

記号: **在る**＝そのまま使える／**寄せる**＝写しを窓口へ集める／**無い**＝新しく作る／**未知**＝調べが要る／**縛り**＝既存の作りからの制約

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1.1 起動するものの 4 通り | 3 本がそれぞれ argv・`AREKA_ROOT` を組む | **無い**（指定の型）。⑶ に「検体の写しの根」を含むので、検体の名前から根を渡す形も要る（`smoke` ②⑤） |
| 1.2 検体の新しい写し | `SampleRoot::acquire` | **在る**。窓口が持つ |
| 1.3 起こす前に写しを書き換える | `smoke` ④ が descript と DLL を差し替え | **無い**（起こす前の段を呼び手へ渡す形） |
| 1.4 回の置き場 | `TempPath::under_target`・`CARGO_TARGET_TMPDIR` | **寄せる**＋失敗の回に残す口が**無い**（要件 6） |
| 1.5 開発者の環境を受け継がない | 3 本で組がばらばら | **寄せる** |
| 1.6 明示の値を優先（壊した値も） | `smoke` ⑤ が記憶の置き場に普通のファイル | **無い**（上書きの口） |
| 1.7 告知の抑止・着色を切る・保険の自動終了 | `AREKA_NO_ALERT`・`NO_COLOR`・`AREKA_APP_SMOKE_EXIT_MS` | **寄せる** |
| 1.8 自動終了の既定 ＜ 窓口の締切 | 3 本ばらばら（3 秒／120 秒・20 秒／60 秒・500 ms／60 秒） | **無い**（既定の 2 つの値と、その大小を固定する確かめ） |
| 1.9 Windows の形の絶対パス・短いパス | 検体は絶対パス。`target\` の下は約 70〜90 字 | **縛り**: 置き場の名前を短く保つ（今は pasta 0.3.5 以降で長いパスでも黙らないが、作者の Lua の `io.open` は 260 字で落ちうる） |
| 1.10 起こせなかったら場所と理由 | 3 本とも `spawn` の失敗で panic | **寄せる** |
| 2.1〜2.4 仲介の揃え方 | 2 本に写し | **寄せる**（`is_file` と `is_i686_pe` の食い違いを 32bit の判定に揃える） |
| 2.5 試験用の 32bit の成果物 | `smoke` の `ensure_i686_artifact` | **寄せる** |
| 2.6 書きかけを起動させない | 一時名＋改名 | **寄せる**。別のプロセスが同時に改名するときの失敗（置き先を誰かが使っている）の扱いは**未知**（後述） |
| 3.1 標準出力・標準エラーを別々のファイルへ | `mcp_get_log` だけ | **寄せる** |
| 3.2〜3.5 文字列を待つ・締切・先に終わった | `mcp_get_log` のループに混ざっている | **無い**（待ちの部品と、「締切切れ」と「子が先に終わった」を見分ける戻り値） |
| 3.6 途中でも読める | ファイルなら読める | **在る**（ファイルにすれば） |
| 3.7 決まった時間の眠りを持たない | 3 本とも見回りの間隔（50・250 ms）で眠る | **縛り**: 見回りの間隔は眠りだが、待ちの終わりは観測で決まる。要件の読み方の確認が要る（議題 8） |
| 4.1 空いた番号 | `mcp_get_log` の `free_port` | **寄せる**（取ってから放すので、子が束ねるまでの間に取られうる＝9.2） |
| 4.2 ツールを名前と引数で呼ぶ | 手書きの HTTP | 中継の `scan::scan` と rmcp の `call_tool` が**在る** |
| 4.3 待受の始まりを待ってから呼ぶ | なし（つながるまで呼び直す） | **無い** |
| 4.4 段を絞っていても観測 | — | **無い**。`info` の行が標準出力に出ない（2.5） |
| 4.5 待受を始められなかった | — | `error` の行は絞っても出る。**無い**（見つけて直ちに失敗を返す部品） |
| 4.6 ツールの失敗を見分ける | — | `CallToolResult.is_error` が**在る** |
| 4.7 つながらない・読めない・締切 | — | 中継の `HttpError`・`ServiceError` が**在る**。窓口の文へ写す部品は**無い** |
| 4.8 途中で子が終わった | — | **無い**（呼び出しの間も子の終わりを見る） |
| 4.9 文字列が現れるまで呼び直す | `mcp_get_log` のループ | **寄せる** |
| 5.1 利用者と同じ閉じ方 | `on_ghost_os_close`・`taskkill`（強制なし）の先例 | **無い**（窓口から送る部品）。送ってよい時点は**未知**（2.6） |
| 5.2 自分で終わるのを待つ | 3 本の番犬 | **寄せる** |
| 5.3 閉じ方か自動終了かの見分け | `[quit_app]` の行の `origin=` | 行は**在る**が、絞った段では出ない（2.6） |
| 5.4 締切で止める | `child.kill()` | **寄せる** |
| 5.5 閉じないまま終わったら子を残さない | なし（panic の途中で子が残りうる） | **無い**（破棄の段で閉じる→止める） |
| 5.6 自分が起こした子だけ | `Child` の取っ手 | **在る**（取っ手を持つ間は番号が使い回されない） |
| 6.1〜6.4 落ちた回の証拠 | なし（全部消える） | **無い**。検体の写しの残し方は議題 5 |
| 7.1〜7.7 移し替え | — | **無い**。3 本の確かめの中身は 2.1 の表のとおり。`smoke` は 1 ファイルに 5 本の試験がある |
| 8.1〜8.3 起動の約束の綴りを隠す | — | **無い**（窓口の中だけで環境変数と位置の引数へ写す） |
| 9.1〜9.2 並べて立てる | `TempPath`・検体の写しは番号と連番で一意 | 番号の取り合いの観測は 4.5 と同じ部品 |
| 10.1 決定論のテスト（偽の子・偽の受け口） | 中継の偽の受け口・本物の受け口の先例 | **無い**（偽の子） |
| 10.2 説明 | `doc/` に試験の手引きは**無い** | **無い**（手引きの置き場を決める＝新しい 1 節か新しいファイルか） |
| 10.3 配布物の依存に入らない・見張りの強さ | 見張りは `sample-ghost-kit` の直の依存だけを見る | **縛り**（2.7）。議題 1 |
| 10.4 外の依存を足さない | 中継の依存は全部 `Cargo.lock` に在る | **在る** |
| 10.5 公式 SDK・非同期を見せない | 中継の `scan`（async） | 窓口の中で tokio を回す器が**無い** |
| 10.6 `target\` の下だけ | `TempPath::under_target`・検体の写し | **在る**（2.3 の `target` の食い違いに注意） |

---

## 4. 実装の選択肢

### 選択肢 A: 既存のクレートに足す（`sample-ghost-kit` に `run` を置く）

- brief の案 ⑴。C9 の約束（`crates/sample-ghost-kit/src/lib.rs` に触らない・道具は新しいクレートに置く）と衝突する（`lib.rs` に `mod run;` の 1 行が要る）。
- `sample-ghost-kit` が「検体を名前で引く窓口」に加えて、rmcp・tokio・中継を引く道具になる。`sample-ghost-kit` を引く全クレート（10 以上）のテストのビルドに中継と rmcp が入る。
- 見張りとの折り合いは不要（`sample-ghost-kit` 自身なので）。
- ✅ 見張りを直さずに済む ❌ ウェーブの約束と衝突・役が 2 つになる・依存が広がる

### 選択肢 B: 新しいクレート（例 `crates/real-run-kit/`）に全部を置く

- C9 の約束どおり。依存の向きは `real-run-kit` → `sample-ghost-kit`・`areka-mcp-bridge`（→ `areka-mcp`）・`temp-path-kit`。使う側は `crates/areka/Cargo.toml` の `[dev-dependencies]` に 1 行。
- 中に置くもの（目安・1,000 行の目安でファイルを分ける）: 起動の指定と環境の組み立て／回の置き場と証拠／32bit の成果物の揃え方／出力の流しと待ち／MCP の呼び出し（tokio の器をここに閉じる）／閉じ方と止め方。決定論のテスト用の偽の子（下の議題 10）。
- 見張りを直す（議題 1）。
- ✅ 約束どおり・役がはっきり・使う側の依存は 1 行 ❌ 見張りの直しが要る・ファイルが増える

### 選択肢 C: 新しいクレート＋検体の窓口には依存しない（呼び手が検体を取る）

- 窓口はパスだけを受け取る（検体の写しの根・ゴーストのフォルダ・バルーンのフォルダ）。呼び手が `SampleRoot::acquire` を自分で呼んで渡す。
- 見張りは直さずに済む。ただし要件 1.1 ⑴（検体の名前で立てる）・1.2（窓口が写しを取る）・6.1（写しを残す）を窓口の外へ出すことになり、**要件の改めが要る**。写しを残すのはパスの改名でできる（呼び手の `SampleRoot` が窓口より後に捨てられる順にしないと、写しが先に消える＝並べ方に頼る壊れやすい形）。
- ✅ 見張りに触らない ❌ 要件 1.1 ⑴・1.2 を満たさない・呼び手の書き方に頼る

### 推しの向き（判断は討議で）

選択肢 B が要件とウェーブの約束の両方に沿う。B を採る場合の分かれ目は、見張りの直し方（議題 1）・閉じ方の手段（議題 2）・記録の段（議題 3・4）・写しの残し方（議題 5）。

---

## 5. 規模と危うさ

- **規模: M**（5〜8 日相当）。部品はほぼ在るものの寄せ集めだが、偽の子・閉じ方の時点の見極め・見張りの直し・移し替え 3 本（`smoke` は 5 本）・手引きが並ぶ。brief の見立て（8〜12 タスク）と合う。
- **危うさ: 中**。
  - 閉じる知らせを `OnCloseRequest` が差される前に送ると、窓 1 枚だけが消えて半端なゴーストになる（2.6）。送ってよい時点の目印を決めないと、揺れる試験になる。
  - 見張りの直しは「同じ強さ」を保つ形にしないと、要件 10.3 を満たさない。
  - 写しを改名で残す形は `devroot.rs` の中のふるまいに頼り、同じウェーブの ⑤ が `devroot.rs` を触る。
  - `taskkill`（強制なし）が見えない最上位の窓（偽の子の窓）にも `WM_CLOSE` を送るかは調べが要る（議題 10）。

---

## 6. 設計で決めること（議題の候補）

1. **見張りとの折り合い（要件 10.3）**: 新しいクレートの `[dependencies]` に `sample-ghost-kit` を載せると `sample_path_guard_test.rs` が赤になる。案 ⑴ 見張りに「テスト専用の道具のクレート」の例外表（件数を別の定数に逐語で持つ今の作法）を足し、同時に**その道具のクレートが本番の依存表に載らないこと**も見張る（＝`sample-ghost-kit` を間接に引く道を 1 本ずつ塞ぐ）。案 ⑵ 見張りを「配布物のクレートの本番の依存を `Cargo.toml` から辿って、`sample-ghost-kit` に届かないこと」に強める（間接の道もまとめて塞ぐ・走査器に依存の辿りが増える）。案 ⑶ 選択肢 C（窓口は検体の窓口に依存しない）＝要件 1.1 ⑴・1.2 の改め。どれでも、見張りのファイルは roadmap の ⑥ の触るファイルの欄に無いので、欄の追記が要る（C9 のほかの 7 本は触らない）。
2. **閉じ方の手段と送る時点（要件 5.1・5.5）**: 案 ⑴ `taskkill /PID <子の番号>`（強制なし・標準の道具・依存なし・実機の先例あり・最上位の窓すべてへ送る）。案 ⑵ 窓口が `windows` クレート（ワークスペースに在る・`Win32_UI_WindowsAndMessaging` も既に有効）で子の最上位の窓を数え、1 枚へ `WM_CLOSE` を送る（届く数を 1 にできる・`unsafe` が窓口に入る）。案 ⑶ `mcp-kanade-tools` の着地の後に台本 `\-` で閉じる（本 spec の時点では使えない）。送る時点は、`OnCloseRequest` が差された後と言える目印（例: 「本物のゴースト窓を開きました」より後の記録・起動の挨拶の始まりなど）を待つか、届いても閉じなかったら締切の内で送り直すか。窓が 1 枚も無い子（起動の途中・窓を開けずに止まる子）へは届かないので、そのときは締切で止める（5.4）。
3. **閉じ方か自動終了かの見分け（要件 5.3）**: 目印は `[quit_app]` の行の `origin=`（`KanadeStopped(Quit)`・`OsClose`・`Smoke`）。この行は `info` なので、呼び手が `RUST_LOG=warn` に絞ると出ない。案 ⑴ 窓口が `RUST_LOG` の末尾に `areka::app_exit=info` を足す（出所を絞った足し方なので、ほかの行は増えない）。案 ⑵ 終了コードと所要時間から推す（自動終了の時刻との前後）＝確かではない。案 ⑴ を採ると、標準出力に 1〜2 行増えることを `mcp_get_log` の「標準出力に出ないはずの行」の確かめと突き合わせる（増えるのは `[quit_app]` の行だけで、確かめの対象の「本物のゴースト窓を開きました」は増えない）。
4. **待受の始まりの観測（要件 4.3・4.4・4.5）**: 案 ⑴ 窓口が `RUST_LOG` の末尾に `areka_mcp::server=info` を足し、「待受を始めた」の行を待つ（要件の字面どおり）。案 ⑵ 記録の行の代わりに、つないで名乗りを確かめる（中継の `scan` そのもの）を締切の内でくり返し、`error` の行（束ねられなかった）が出たら直ちに失敗を返す（段を絞っても `error` は出る）。⑵ は要件 4.3 の「記録に待受を始めた行が現れるまで」の字面と違うので、⑵ を採るなら要件の改め。どちらでも、行の文は `areka-mcp` の中の文字列で公開の定数ではない＝写すと黙ってずれる。文を `areka-mcp` の公開の定数にするのは本番の作りの変更（Out of scope の「記録の行の変更」に当たるか）の確認が要る。偽の子が本物の `areka_mcp::start` を呼ぶ形（議題 10）にすれば、窓口の決定論のテストが「文がずれたら赤」になる。
5. **検体の写しを残す形（要件 6.1・C9 の約束）**: 案 ⑴ 落ちた回だけ、子が終わった後に写しのフォルダを回の置き場へ改名で移す（写しの破棄は `NotFound` を黙って通すので札だけ消える・複写が要らない・`devroot.rs` の中のふるまいに頼る・⑤ が `devroot.rs` を触る・`CARGO_TARGET_DIR` が別のドライブだと改名できない→複写へ落とす分かれ道が要る）。案 ⑵ 最初から写しを回の置き場へ丸ごと複写して使い、`SampleRoot` はすぐ捨てる（回のものが全部 1 つのフォルダにまとまる・emo2 で数 MB の複写が走りごとに増える・置き場が短いパスになる）。案 ⑶ 落ちた回だけ写しを回の置き場へ複写してから捨てる（破棄の段で複写が走る＝panic の途中の破棄で失敗を panic にしない注意が要る）。`std::mem::forget` で写しを捨てずにおく形は、次の取得の掃除で消えるので証拠にならない（2.2）。
6. **回の置き場の作り方（要件 1.4・6）**: 案 ⑴ `TempPath::under_target` を使い、落ちた回だけ `std::mem::forget` して残す（`temp-path-kit` に触らない・取っ手が無いので副作用なし）。案 ⑵ 窓口が自前で `target\` の下に作る（`target` の決め方を検体の写しと揃えられる）。失敗の判定は、破棄の段の `std::thread::panicking()`（窓口が失敗させたときも呼び手の `assert!` が落ちたときも panic の途中で破棄される）で足りるか。破棄の段では panic しない（二重の panic はプロセスを落とす）。
7. **`areka.exe` の場所の渡し方（要件 1.1・1.10・2.1）**: 窓口のライブラリは `env!("CARGO_BIN_EXE_areka")` を展開できない（その名前は areka のクレートの統合テストの中だけ）。案 ⑴ 呼び手が `env!("CARGO_BIN_EXE_areka")` を最初の引数で渡す。案 ⑵ 窓口が呼び手の場所で展開されるマクロを出す。案 ⑶ 窓口が実行ファイルの祖先の `target` から `<target>\<profile>\areka.exe` を組む（ビルドされていない場合の失敗の文が要る・偽の子へ差し替える口も要る）。偽の子へ差し替える口（要件 10.1）は、どの案でも要る。
8. **出力の流し方と「眠りを持たない」の読み方（要件 3.1・3.7）**: 案 ⑴ 子の出力をファイルへ直に向け、短い間隔で読み直して探す（今の `mcp_get_log` の形・見回りの間隔は眠りだが、待ちの終わりは観測で決まる）。案 ⑵ パイプで受け、読み手のスレッドがファイルへ写しながら待ち手へ知らせる（眠りが無い・子の終わりの知らせも別の仕組みが要る）。要件 3.7 が禁じるのは「決まった時間だけ眠ってから進む」ことで、見回りの間隔はそれに当たらないと読んでよいかの確認。
9. **MCP の呼び出しの形（要件 4.2・10.5）**: 案 ⑴ 中継の `areka_mcp_bridge::scan::scan` で `Link` を立てて `call_tool` を呼ぶ（中継のライブラリに依存する・名乗りの確かめも使い回せる）。案 ⑵ 中継の `OwnHttp` だけを使い、rmcp のクライアントを窓口が自分で立てる（中継の `scan` の「最初に areka と名乗った候補」の探し方を使わない）。tokio の器は、呼び出しのたびに `current_thread` を作るか、走りごとに専用のスレッドを 1 本持つか。引数の型は、`serde_json::Value` を窓口の公開の型に出すか（areka の `[dev-dependencies]` に `serde_json` が無い＝窓口が `json!` を出し直すか、areka に `serde_json` を足すか・どれも `Cargo.lock` に在る）、文字列で受けるか。
10. **偽の子の作り（要件 10.1）**: 案 ⑴ 窓口のクレートに試験用の小さな実行ファイル（`[[bin]]`）を置き、環境変数で振る舞いを選ぶ（指定の行を出す・本物の `areka_mcp::start` に偽のツールを載せて待ち受ける・指定の終了コードで終わる・終わらない・見えない最上位の窓を 1 枚持って `WM_CLOSE` で終わる）。窓口の統合テストは `CARGO_BIN_EXE_<名前>` で場所を得る。案 ⑵ 試験のバイナリ自身を特別な引数で起こし直して偽の子にする（実行ファイルが増えない・書き方が読みにくい）。調べが要る点: `taskkill`（強制なし）は**見えない**最上位の窓にも `WM_CLOSE` を送るか（偽の子で `taskkill` の道を決定論で確かめられるか）。
11. **失敗の返し方（要件 3.4・4.5〜4.8・5.4）**: 窓口の失敗は panic（試験を失敗させる）にするか `Result` を返すか。要件は「試験を失敗させる」（1.10・2.3・2.4・3.4・5.4）と「失敗を返す」（4.5〜4.8・3.5）を書き分けている。呼び手が受理する場面（モニタ 0 台・`smoke` ③④⑤ の非 0 の終わり）を書けるよう、「子が先に終わった」は値で返す（3.5）。
12. **既定の時間（要件 1.8・3.3・5.1）**: 窓口の待ちの既定の締切と保険の自動終了の既定の値（今の 3 本は 60〜120 秒の番犬と 0.5〜20 秒の自動終了）。`smoke` は自動終了そのものを確かめるので 500 ms の上書きを保つ（7.4）。`emo2_real_run` は 3 秒の上書きを保つ（7.6）。`mcp_get_log` は閉じ方で閉じるので、保険の自動終了は既定でよい（7.5）。
13. **移し替えで子の環境が変わること（要件 7.2・7.7）**: `emo2_real_run` は `AREKA_ROOT`・`AREKA_PROFILE_DIR` を受け継がなくなり、MCP の番号が空いた番号になる。`smoke` ① は開発者の本物の記憶の置き場を使わなくなる。確かめの中身は変わらないが、変わることを移し替えの記録に書くか。
14. **空いた番号の取り合い（要件 9.2）**: 空きを得て放してから子が束ねるまでの間に取られる窓は消せない（areka は `AREKA_MCP_PORT=0` を「待ち受けない」に使うので、OS に選ばせる値を渡せない・本番の変更は Out of scope）。要件どおり「取られていたと分かる失敗」で止めるか、窓口が別の番号で立て直すか（要件の改め）。
15. **手引きの置き場（要件 10.2）**: `doc/` に試験の手引きのファイルは今は無い。新しいファイル（`doc/` の一覧の `doc/README.md` に 1 行）にするか、既存の文書の 1 節にするか。実機の試験を 1 本ずつ回す机の説明は `doc/impl-watch.md`（① `impl-watch-test-desk` の持ち物）を指すだけにする。
16. **steering の更新**: 新しいクレートを `structure.md` のクレートの一覧に足す（`log-capture-kit`・`temp-path-kit` の並び）。`tech.md` の依存の登記（外の依存は増えないが、テスト専用の道具がワークスペースの中の依存を引くこと）を書くか。どちらも完了の段で行うか、設計の段で行うか。

---

## 7. 調べが要るもの（Research Needed）

- `taskkill`（強制なし）が、見えない最上位の窓・`WS_EX_TOOLWINDOW` の窓にも `WM_CLOSE` を送るか（偽の子と本物の areka の窓の両方）。
- `OnCloseRequest` が差される時点と、記録に出る目印の前後（「本物のゴースト窓を開きました」の行は窓を作る閉包の中＝HWND が付く前かもしれない）。閉じる知らせを送ってよい最も早い目印を記録から決める。
- 同じ知らせが窓の数だけ届いたとき（`taskkill` の案）、kanade が 2 通目以降をどう扱うか（実機では問題なく終わった記録があるが、決まりとして確かめる）。
- 32bit の仲介を、別の試験のプロセスが同時に改名で置くとき（置き先が別の子に使われている最中）の失敗の扱い（`rename` が「使用中」で失敗したら、置き先が既に 32bit かを見直して通すか）。
- `CARGO_TARGET_DIR` を別のドライブへ向けたときの、検体の写し（`devroot`）と回の置き場（`temp-path-kit`）の `target` の食い違い。
- rmcp 3.5.0 の `Peer<RoleClient>::call_tool` の答えの形（`CallToolResult` の `content` の文の取り出し・`is_error` の `None` の扱い）と、中継の `Timeouts` を窓口の締切に合わせる方法。

---

## 8. 設計の段への申し送り

- 選択肢 B（新しいクレート）を前提に、議題 1〜16 のうち答えで作業が変わるもの（1・2・4・5・7・10・14）を要件の討議に挙げる。
- 使い回す部品: `SampleRoot::acquire`／`TempPath::under_target`／`areka_mcp_bridge::scan::scan` と `Timeouts`／本物の `areka_mcp::start(&[0], …)`（偽の受け口）／`Child` の取っ手（自分が起こした子の確かめ）／`taskkill`（強制なし）の実機の先例。
- 移し替えのとき、3 本の確かめ（2.1 の表と要件 7.2）を 1 つずつ照らして、消えた確かめが 0 であることを記録する。

---

## 9. 設計の段の調べと決めたこと（2026-10-18・`/kiro-spec-design -y`）

### 9.1 要約

- **Feature**: `areka-P0-real-run-mcp-kit`
- **Discovery Scope**: Extension（ワークスペースに在る部品の寄せ集め＋新しいテスト専用のクレート 1 つ）。調べは軽い形（`design-discovery-light`）で、外の新しい依存は 0 個なので外の文書の調べは rmcp 3.5.0 のソース（`Cargo.lock` が固定する版）の確かめだけにした。
- **Key Findings**:
  - rmcp 3.5.0 の `Peer<RoleClient>::call_tool(CallToolRequestParams)` がそのまま使える（`CallToolRequestParams::new(name).with_arguments(map)`・答えは `CallToolResult`）。中継の `scan::scan` の `Link` の `peer()` から呼べる。
  - ゴーストの窓は同じ tick の中で HWND が付き（wintf `create_windows`＝排他の段）、同じ tick の `FrameFinalize` で `OnCloseRequest` が差される。OS の閉じる知らせは tick の外で届くので、届いた時点のゴーストの窓には受け手が付いている。窓が無い子へは `taskkill`（強制なし）が失敗する＝送り直しで扱える。
  - 窓口が頼る記録の行は 2 つの出所に集まる: `areka_mcp::server`（待受の 3 行）と `areka::app_exit`（`[os_close]` の受け取りの行・`[quit_app]` の終わり方の行）。`RUST_LOG` にこの 2 つを `=info` で足せば、呼び手が段を絞っても見える。`mcp_get_log` の「標準出力に出ないはずの行」（出所 `areka::ghost_session`）は増えない。

### 9.2 調べの記録

#### rmcp 3.5.0 のツールの呼び方
- **Context**: 要件 4.2・10.5（公式 SDK で話す・手で組まない）。
- **Sources Consulted**: `c:\rust\cargo\registry\src\…\rmcp-3.5.0\src\model.rs`（`CallToolRequestParams` 4247 行〜・`#[non_exhaustive]`・`new`・`with_arguments`）・`src\service\client.rs`（`call_tool` 1966 行〜）・`crates/areka-mcp-bridge/src/scan.rs`（`Link`＝`RunningService<RoleClient, BridgeClient>`）。
- **Findings**: `CallToolRequestParams` は `#[non_exhaustive]` なので構造体の式では作れず、`new`＋`with_arguments` で作る。`call_tool` は `Result<CallToolResult, ServiceError>`。`is_error` は `Option<bool>`。
- **Implications**: 窓口は rmcp を `client` だけで引けば足りる（中継が `server`・`transport-streamable-http-client` を有効にしているので、機能はワークスペースで統合される）。

#### 待受の行と終わり方の行
- **Context**: 要件 4.3〜4.5・5.3。
- **Sources Consulted**: `crates/areka-mcp/src/server.rs`（`start` 33〜93 行）・`crates/areka/src/app_exit.rs`（`quit_app` 102〜135 行・`on_ghost_os_close` 257〜289 行）・`crates/areka/src/log_history.rs` 409〜414 行（未設定の `RUST_LOG` は `info`）。
- **Findings**: 待受の行は `MCP: 待受を始めた`（移った版は同じ頭で始まる）・`MCP: どの候補のポートも束ねられなかった（待ち受けない）`（`error`）・`MCP: 待ち受けない（AREKA_MCP_PORT が 0）`。`quit_app` は最初の出所だけを `info`（`event="app_exit"`・`origin=?origin`）で出し、2 度目からは `debug`。`on_ghost_os_close` は `info`（`event="os_close_request"`）。
- **Implications**: 窓口は綴りを私有の定数で持つ。待受の綴りは偽の子が本物の `start` を呼ぶことで檻に入る。終わり方の綴りは偽の子では檻にならないので、移し替えた実機の試験に 2 つの確かめを足す（設計 D-19）。

#### 閉じる知らせを送ってよい時点
- **Context**: `research.md` 2.6・7 の「調べが要るもの」。
- **Sources Consulted**: `crates/wintf/src/ecs/window/window_system.rs`（`create_windows` は排他の段で `WindowHandle` を即時に挿す）・`crates/wintf/src/runtime/window_factory.rs` 150〜198 行（生成中の窓の手続きは World の借用中で飛ばされる）・`crates/areka/src/ghost_session.rs` 76〜82 行（`attach_os_close_request` は `FrameFinalize`）。
- **Findings**: HWND が付く tick と受け手が差される tick は同じ。`taskkill` の知らせは窓の手続きで tick の外に届く。
- **Implications**: 時点の目印の行を待たずに、`taskkill` を受け取りの行か子の終わりまで 1 秒ごとに送り直す形でよい（設計 D-3）。表示しない窓へ `taskkill` が届くかは、偽の子を相手に最初の実装のタスクで確かめる（届かなければ偽の子の窓を画面の外の 1×1 の道具窓にする・窓口の外は変わらない）。

#### 見張りとテスト専用のクレートの決まり
- **Context**: 要件 10.3・10.4・10.6。
- **Sources Consulted**: `crates/log-capture-kit/tests/sample_path_guard_test.rs` 641〜750 行・`tests/workspace_scan/mod.rs`（`workspace_manifests`・`production_dependencies_on`）・`tests/with_default_guard_test.rs` 冒頭（捕捉先を直接差す 3 語・`env-filter` を宣言してよいのは wintf だけ）・`tools/crates-io.ps1` 判定「理由」（出さないクレートは `publish = false # 理由` の行）・`tools/test-all.ps1` 段 2（i686 の成果物を常時のテストより先に建てる）。
- **Findings**: 本番の道で `sample-ghost-kit` に届くには、本番の表に `sample-ghost-kit` を持つクレートを通るしかない。`[dev-dependencies]` は推移しない。
- **Implications**: 例外表（`real-run-kit` 1 件）と「`real-run-kit` がどの本番の表にも載らない」検査で、今と同じ強さになる（共有の走査器 `workspace_scan/mod.rs` には触らない）。偽の子は `tracing-subscriber` を `workspace = true` で引き、`init()` で据える。

#### 既にある部品の使い回し
- `areka_user_data::copy_tree_atomic`（`crates/areka-user-data/src/copy.rs` 55 行）: 落ちた回の検体の写しの複写にそのまま使える（作業フォルダに組んで改名・既に在れば書かない）。`sample-ghost-kit` の写しの消え方（`devroot.rs` の `NotFound` を黙って通すふるまい）に頼らずに済む。
- `areka-user-data` の判断（`decide`）: `AREKA_USER_ROOT` が無く、実行ファイルの隣に書け、winget の控えが無ければ、書ける根は使わない（実行ファイルの隣＝`target\` の下）。窓口は `AREKA_USER_ROOT` を外したまま渡さない。

### 9.3 形の比べ

| 案 | 中身 | 強み | 弱み | 採否 |
|---|---|---|---|---|
| B 新しいクレート | `crates/real-run-kit/` に全部（`research.md` 4 章） | C9 の約束どおり・役が 1 つ・使う側は 1 行 | 見張りの直しが要る | 採る（D-1） |
| 閉じ方 `taskkill` | 強制なしの `taskkill /PID` を送り直す | 依存なし・`unsafe` なし・実機の先例 | 窓の数だけ届く（先例では問題なし） | 採る（D-3） |
| 閉じ方 `windows` で 1 枚へ | 子の最上位の窓を数えて 1 枚へ `WM_CLOSE` | 届く数が 1 | 窓口のライブラリに `unsafe`・窓の見分けが要る | 採らない |
| 写し 改名で移す | 落ちた回に写しのフォルダを改名 | 複写なし | `devroot.rs` の中に頼る（同じウェーブの ⑤ が触る）・別のドライブで失敗 | 採らない |
| 写し 落ちた回だけ複写 | `copy_tree_atomic` で `<回>\sample\` | 中に頼らない・成功の回は 0 | 落ちた回に複写の時間 | 採る（D-6） |
| 写し 最初から複写 | 回の置き場に丸ごと写して使う | 回が 1 つにまとまる | 走りごとに数 MB の複写 | 採らない |
| 待受の観測 つないで確かめる | 記録でなく `scan` をくり返す | 段に左右されない | 要件 4.3 の字面と違う | 採らない |

### 9.4 まとめ方の 3 つの目（design-synthesis）

- **まとめられるもの**: 「文字列を待つ」「待受の行を待つ」「閉じる知らせの受け取りを待つ」は、どれも「出力のファイルに文字列のどれかが現れる・子が終わる・締切」の 3 択。`output::wait_any` 1 つにまとめた。仲介と試験用の DLL の揃え方も 1 つの `ensure`（定めを引数で渡す）。
- **作るか使うか**: MCP の話し方は rmcp＋中継の `scan`（作らない）。偽の受け口は本物の `areka_mcp::start`（作らない）。置き場は `temp-path-kit`・`sample-ghost-kit`、複写は `copy_tree_atomic`（作らない）。閉じる知らせは Windows 標準の `taskkill`（作らない）。作るのは待ち・閉じ方の送り直し・証拠を残す判定・偽の子だけ。
- **削ったもの**: 呼び出しごとの締切の上書き（走りの締切 1 つで足りる）・引数の JSON の型（文で受ける）・`Launch` に根と位置の引数を同時に渡す形（使う試験が無い）・偽の子の窓と保険の自動終了の同時の手順（別々で確かめられる）。

### 9.5 危うさと手当て

- 偽の子の表示しない窓へ `taskkill` が届かない → 最初の実装のタスクで確かめ、届かなければ画面の外の道具窓に替える（窓口の外は変わらない）。
- 位置の引数で立てる回にも空の根を渡す（要件 1.4）ことで、根の列挙の記録の行が増えうる → 移し替えのタスクで実機の出力を見る（確かめの対象の行は変わらない）。
- 子が受け口を閉じてから終わるまでの間の輸送の失敗 → `exit_grace` の内に終われば `ChildExited`、超えれば `Unreachable`（どちらも締切の内）。
- 窓口が頼る記録の綴りが areka の側で変わる → 待受は偽の子の檻、終わり方は移し替えた実機の 2 つの確かめ（D-19）が赤で知らせる。
- 見張りのファイルは roadmap の C9 の ⑥ の触るファイルの欄に無い → 設計の決定 D-2 として記録し、欄の追記を棚卸の側へ申し送る。

### 9.6 参照したスキルと手引き

- `kiro-spec-design`（`rules/design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と、`.kiro/settings/templates/specs/design.md`・`research.md` の型を使った。
- steering は `product.md`・`tech.md`・`structure.md`（Temp Path Kit・Sample Ghost Kit・Log Capture Kit の節）・`roadmap.md`（「ウェーブ編成」C9 の行 ⑥ と 1 本だけの席）を読んだ。
- 開発の決まり（記憶）として、公式 SDK を使う・tokio を閉じ込める・待ちは観測で有界・試験の置き場は `target\` の下の短い絶対パス・自分が起こしたプロセスだけを止める・1,000 行・`std::env::temp_dir` は `temp-path-kit` の外で使わない・失敗を黙って通さない、を設計の縛りにした。

### 9.7 設計の段の審査の結果

- 機械の確かめ: `requirements.md` の 60 個の番号（1.1〜10.6）がすべて design.md の対応表にある・Boundary の 4 節と File Structure Plan が埋まっている・部品はすべて File Structure Plan にファイルがある。
- 判断の確かめで直したもの（1 回目）: `Run` ができる前の失敗でも回が残るよう `RunDirs` に自分の `Drop` を持たせた・手引きに書く項目を 10.2 のとおり並べた・偽の子の `smoke` と窓を別の手順にした・破棄で閉じたことを確かめる印のファイルを足した・`McpError` の文に出力の場所を入れない形にした・roadmap の欄に足すファイルを全部挙げた。2 回目の審査で残りは 0。
