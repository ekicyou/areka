# ギャップ分析: install-before-boot

> 実測: 2026-10-31・本ブランチ（main `da020c10` の上・`b721f3b5`）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。SSP の実機の測りは行っていない（要件 10.2・10.4 の測りは討議の後で許しを取ってから）。
> 正典: SSP ヘルプ「起動オプション一覧」 https://ukatech.shillest.net/ukadoc-ssp/functions/option/ （`--install`・`--option callghost`・`initial_install.nar`）・ukadoc「install.txt」の `bootghost`・ukadoc「SHIORI Event」 https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallCompleteEx （「インストールが正常終了した際に発生」＝受け手は起動中のゴースト）。

## 1. 要約

- **手続きの本体はそのまま使える**: `install::procedure::run_order` は World もスレッドも知らず、口 `InstallPorts`（8 つ）だけで外とやりとりする。起動の前の口を 1 つ足せば、読み取り・検査・`accept` の照合・利用条件・展開・警告を今の決まりのまま通せる。展開の芯 `install::destination::install_into` も World を受けない。
- **起動の解決の継ぎ目も既にある**: `boot_config::resolve_boot_with` は根の解決の結果（`Result<ResolvedRoots, RootError>`）を引数で受ける。`roots::resolve_roots` を先に呼び、入れてから `resolve_boot_with` へ渡せば「根 → 入れる → ゴースト → バルーン」になる（根とゴーストの関数を割る必要はない）。
- **足りないもの**: ①起動の前の口（イベントを「送らない」をどう返すか・素性の無いゴースト・記録の受け皿の無さ）②`callghost` を「起動のゴーストの選び方」にする段（今は切り替えを頼む）③起動の前の消去・読み取りだけの断りの道 ④定常の後の道（`startup::on_steady`・`on_order_done` の起動の引数の腕）の撤去と、それを固めたテスト 45 本（`startup_tests.rs` 35・`desk_startup_tests.rs` 10）の作り直し。
- **新しく見つけた危険（調べが要る）**: 起動の前の利用条件と警告の画面は、`WinApp` が `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` を呼ぶ**前**に出る。そこで窓（MessageBox）を作ると、画面がぼやけるだけでなく、後の DPI の設定に響くおそれがある（§5 R1）。
- **規模と危うさ**: M（7〜10 タスク）・中。手続きと選び方は今の形の延長だが、定常の後の道の撤去とテストの作り直しが量の半分を占める。

## 2. 今の姿（要件に効く所）

### 2.1 起動の順（`crates/areka/src/main.rs` の `main`）

| 順 | 行（今） | 何をするか |
|---|---|---|
| 1 | 176〜183 | `cli::read`・`--help`・`--offline-dump` の早い戻り（要件 1.6 は今のままで満たす） |
| 2 | 185 | `install::startup::StartupInstall::new(opts.install)`（`--install` を絶対パスへ・exe のフォルダ） |
| 3 | 187 | `readonly::engage`（以後 `readonly::is_on()` が読める＝要件 7 の判定は起動の前でも使える） |
| 4 | 212〜221 | `resolve_boot(&opts.boot)`。Err なら `alert::raise` → 終了コード 1（このとき `StartupInstall` が落ち、`Drop` が warn `startup_install_not_submitted reason=boot_stopped`） |
| 5 | 223 | `write_failure::check_boot`（書けるかの確かめ） |
| 6 | 235 | MCP の待受 |
| 7 | 249 | `WinApp::with_exit_policy`（**ここで初めて** COM と DPI の設定・`crates/wintf/src/runtime/mod.rs`） |
| 8 | 255〜259 | 系の登録・`install::startup::install`（持ち物を World の資源へ） |

- `main.rs` は **986 行**（上限 1,000）。足す分は外へ置き、`main.rs` は呼ぶ所の差し替えに留める必要がある（制約）。2 と 4 を 1 つの呼び出しにまとめれば、行は増えないか減る。

### 2.2 起動の解決（`boot_config.rs`・`boot_resolve.rs`）

- `resolve_boot` は `roots::resolve_roots(&app_profile_dir)` を呼んで `resolve_boot_with` に渡すだけの薄い口（`boot_config.rs`）。**継ぎ目はここ**。
- `resolve_boot_with` の中: `--ghost` の指し先（`point_ghost`・一時の置き場を並びの先頭へ）→ 根の記録 → 位置の引数の検査 → 起動中の印（`read_session_mark`）→ 記憶（`read_last_ghost`）→ `boot_resolve::resolve_ghost` → `prepare_ghost`（`user_root::writable_ghost`）→ バルーン。
- `resolve_ghost` の段: 位置の引数 → 指し先（`GhostRoute::Pointed`）→ `--random` → 記憶 → 唯一（`Only`）→ 既定 `emo2` → 無作為。空の置き場に 1 体だけ入れば「唯一」で起きる（要件 4.2 の後半は今の選び方のままで満たす）。
- `resolve_boot_with` を呼ぶ所は本体 1・テスト 8 ファイル（`main_config_input_tests.rs` ほか）に約 37 か所。引数を足すと全部が動く（§4 の比べに効く）。

### 2.3 手続き（`install/procedure.rs`・`judge.rs`・`destination.rs`）

- `run_order(order, ports)` → 書庫ごとに `archive_steps`。`ports.raise` が `Raised::Closed` を返すと**その書庫を `Abandoned` にして並びの残りもやめる**。`ports.ghost_facts()` が `None` でも同じく `Closed`。
  - ⇒ 起動の前の口は、`raise` に `Closed` を返してはならず、`ghost_facts` に `None` を返してはならない（返すと要件 2.6「残りを続ける」に反する）。
- `send` は口の答えを受けたあと必ず info `install_event`（「イベントを送りました」・`raised`）を残す。`OnInstallCompleteEx` に `NoReply` が返ると `OnInstallComplete` も「送る」。
  - ⇒ 口が `NoReply` を返すだけだと、送っていないのに「送りました」の記録が 2〜4 件出る（要件 3.1・6.2 と記録が食い違う）。
- `judge_accept(manifest, facts)`: `accept` があれば `sakura_name`／`install_accept` と照合（素性が空なら必ず `Refused`）。`accept` の無いシェル・追加ファイルは `AcceptMissing`（失敗 `invalid type`）。
  - ⇒ 要件 2.5 は、空の素性を渡すだけで今の決まりどおりに断れる。ただし記録 `install_refused` の `running` が空の綴りになり「受け取るゴーストが居なかった」とは読めない（記録の足しが要る）。
- `destination_of`: 素性の `folder` が `None` なら必ず `Elsewhere`＝`overwrite_running` は呼ばれない（起動の前に使っている最中のフォルダは無い）。
- `install_into(roots, archive, target_ghost, running)`（`destination.rs`）: World を受けない。`CatalogDestinations::new` は一時の置き場を並びから除くので、`--ghost` の一時の置き場を足す前の並びで入れても、窓へ落としたとき（`BootContext.roots`＝一時の置き場つき）と同じ根に入る（要件 1.4）。見つかる根が 1 つも無いと error `install_no_root` で失敗に倒れる。

### 2.4 本物の口（`install/worker.rs` の `DeskPorts`）と窓口（`install/desk.rs`）

| 口 | 今の中身 | 起動の前に使えるか |
|---|---|---|
| `begin_archive` | 門 `WorkGate` に名前を置く（終了の待ち） | 門は無い（終了の待ちの前）＝何もしないでよい |
| `raise` | 窓口へ頼み、kanade の答えを待つ | ゴーストが居ない＝送らない（返し方は §4 決め 3） |
| `ghost_facts` | 窓口が `BootContext` と `GhostSlot` から作る | 根の並びだけを持つ空の素性を作る |
| `ask_terms` | `alert::ask_yes_no(title, body, alert::suppressed())`（hwnd 無しの MessageBox） | **そのまま呼べる**（議題 3 の推奨案の裏づけ）。ただし DPI の危険（R1） |
| `install_elsewhere` | 門の出入り＋`install_into` | `install_into` を直に呼ぶ |
| `overwrite_running` | 窓口へ頼む | 来ない（`destination_of` が `Elsewhere`） |
| `record` | 窓口の `record_installed`: ゴースト→`record_last_installed`（資源 `LastInstalledGhost`）、バルーン→`record_installed_balloon`＋`remember_balloon`（今のゴーストの記憶）、シェル→`record_installed_shell`、全部→`names::update`（置換語） | World が無い。どれも**プロセスの中だけの資源**（記憶へは書かない）＝起動の前に入れた分は、運ばなければ消える（§4 決め 6） |
| `warn_skipped` | `alert::warn_ok`（hwnd 無し） | そのまま呼べる（R1 は同じ） |

### 2.5 起動の持ち物（`install/startup.rs`）

- 再使用できる純粋な部品: `sources_to_delete`（消す書庫・同じパスは 1 回）・`end_word`・`Pending::initial_install`（大小を問わない名前は `is_file` の結果まかせ）・`delete_sources`（1 本ずつ・失敗は warn）・`decide_call_ghost` の「最後に入ったゴーストのフォルダ」を探す部分。
- 起動の前では要らなくなる部品: `on_steady`・`ArgsFollowup`・`discard_for_exit`・`on_order_done` の起動の引数の腕・`call_ghost`（切り替えを頼む）・`callghost_request`・`CallGhostSkip::{Closing, Switching, SwitchedMeanwhile, AlreadyRunning}`・`SwitchAcceptCount` の読み（読み手は `startup.rs` だけ）。
- `Drop` の `record_not_submitted("boot_stopped")`: 起動の前の道が `pending` を取り出せば、ゴーストが見つからずに止まっても `Drop` は何も出さない＝要件 6.3 は「取り出す」だけで満たす。根が決まらないとき（要件 1.7）は取り出さないので今の warn が残る（理由の語を `root_missing` などに分けるかは設計）。

### 2.6 読み取りだけ（`install/mod.rs` の `submit`・`readonly.rs`）

- 断りは `submit` の先頭で `readonly::of(world)` を見て info `readonly_refused`（`what=install`・`origin`・`count`）。起動の前は World が無いので `readonly::is_on()` で同じ記録を出す口が要る（`submit` の断りの部分を `bool` を受ける小さな関数へ切り出せば 1 か所にできる）。

### 2.7 テスト・実機

- `install/startup_tests.rs`（756 行・35 本）と `install/desk_startup_tests.rs`（473 行・10 本）は定常の後の道を固めている。案 A（議題 1）なら大半が「陳腐化テスト」＝除外し、起動の前の道で作り直す（`obsolete-vs-broken-test-policy`）。
- 手続きの偽の口 `install/procedure_test_support.rs`（320 行）が既にある＝起動の前の口の判定（イベント 0 件・`accept` の断り・利用条件の拒否）は同じ型で書ける。
- 実機 `crates/areka/tests/startup_install_real_run.rs`: emo2 の居る根に `--install` と `callghost,deletesource`。今は記録 `startup_callghost_requested` と `switch_balloon_resolved` を確かめている＝案 A では切り替えが起きないので、この 2 つの確かめが変わる（要件 9.11）。空の根の新しい実機の試験（要件 9.10）は `real_run_kit` の `Launch::SampleRoot` が「検体の居る根」しか組めないなら、空の根を組む口が要る（調べ R4）。
- 説明書 `dist/README.txt` の 40・44・45・73〜78・82 行（要件 8）。`doc/pasta-real-run.md` の 248 行目（新しい書庫は窓へ落として入れる）は要件の範囲外だが、着地すれば直せる（§6 決め 9）。

## 3. 要件と資産の対応

| 要件 | 使える資産 | 足りない所 | 印 |
|---|---|---|---|
| 1.1・1.2 根の後・ゴーストの前に順に入れる | `resolve_roots`／`resolve_boot_with` の継ぎ目・`Pending::initial_install`・`run_order` | 2 つを呼ぶ所（`main.rs` の外の新しい関数） | Missing |
| 1.3 1 回だけ | `Pending` の `take` | 定常の後の道の撤去（`desk::on_steady` からの呼び出し） | Missing |
| 1.4 落としたときと同じ置き場 | `install_into`・`CatalogDestinations`（一時の置き場を除く） | なし | — |
| 1.5・1.6 | 今の `Drop`（入れる物が無ければ黙る）・`--offline-dump` の早い戻り | なし | — |
| 1.7 根が決まらない | `Drop` の warn | 理由の語（`boot_stopped` のままか分けるか） | Constraint |
| 1.8 窓を出さない | 起動の前は窓がまだ無い | 利用条件と警告の画面だけは出る（R1） | Unknown |
| 2.1〜2.4 今の手続き・画面 | `run_order`・`alert::ask_yes_no`・`warn_ok`・`suppressed` | 起動の前の口 | Missing |
| 2.5 受け取るゴーストを要する書庫 | `judge_accept`（空の素性で必ず断る・`AcceptMissing`） | 「受け取るゴーストが居なかった」と読める記録 | Missing |
| 2.6 失敗の後も続ける | `run_order` は失敗では止まらない | 口が `Closed` を返さないこと | Constraint |
| 2.7 バルーンの記憶に書かない | — | 口の `record` で info 1 件 | Missing |
| 3.1 イベントを送らない | — | `raise` の返し方（§4 決め 3）・`install_event` の記録 | Missing |
| 3.2 後から送らない | — | 何もしない（運ばない） | — |
| 3.3 起動の後の道は今のまま | `DeskPorts`・窓口 | 変えないこと（メニュー・台本・落とすは `submit` のまま） | Constraint |
| 4.1・4.2 入れた後の置き場で選ぶ | `resolve_boot_with` は目録をその場で読む | なし（入れた後に呼べば足りる） | — |
| 4.3〜4.8 `callghost` を選び方に | `decide_call_ghost` の「最後に入ったゴースト」・`GhostRoute::Pointed`・`point_ghost` | 選び方の段（§4 決め 4）・使わない理由の新しい語（指定あり・ゴースト無し・読み取りだけ） | Missing |
| 4.7 前回がきれいに終わらなかった | 印は読んでも、指し先の段は印より先 | 指し先の形で入れるなら今のまま満たす | — |
| 4.9 `initial_install.nar` は選び方を変えない | 出どころの分け | なし | — |
| 4.11 入れた後も 0 体 | `AlertScene::GhostMissing` | `pending` を取り出した後なら `boot_stopped` は出ない | — |
| 5.1〜5.7 消す | `sources_to_delete`・`delete_sources` | 起動の前の道から呼ぶ（`closing` の腕は要らない） | Missing（移すだけ） |
| 6.1・6.2 記録 | `startup_install_done`（`origin`・`results`）・`end_word` | 「ゴーストを選ぶ前」と読める欄か語（例 `stage=before_boot`） | Missing |
| 6.3 | 上の 2.5 | なし | — |
| 6.4・6.5 | 今の記録・既定のゴーストの台詞の道 | 起動の前の道の失敗の記録 | Missing |
| 7.1〜7.4 読み取りだけ | `readonly::is_on`・`readonly_refused` の語 | World の無い断りの口・`callghost` を使わない info | Missing |
| 8 説明書 | `dist/README.txt` の該当行 | 文の書き換え | Missing |
| 9.1〜9.9 決定論のテスト | `procedure_test_support.rs`・`boot_config_*_tests.rs` の組み立て・`TempPath::under_target` | 起動の前の道のテスト（新しい兄弟のファイル）・陳腐化した 45 本の除外 | Missing |
| 9.10 空の根の実機 | `real_run_kit`・`sample_ghost_kit` | 空の根を組む口があるか（R4） | Unknown |
| 9.11 今の実機の試験 | `startup_install_real_run.rs` | 切り替えの確かめ 2 つの差し替え | Missing |

## 4. 実装の案

### 案 A: 今の部品を広げる（`startup.rs` と `boot_config.rs` に足す）

- `startup.rs` に `run_before_boot(&mut self, roots) -> BeforeBoot` を足し、起動の前の口も同じファイルへ。`boot_config.rs` の `resolve_boot` に持ち物を渡す引数を足す。
- ✅ 新しいファイル 0。❌ `startup.rs` は今 472 行で、定常の後の部品を消しても口と道で 500〜600 行へ戻る。`boot_config.rs`（662 行）に install の知識が入り、層の向き（起動の解決が install を知る）が逆になる。

### 案 B: 起動の前の道を新しい兄弟のファイルに置く（推す形の候補）

- 新しいファイル `install/before_boot.rs`（印: 新）: 起動の前の口 `BootPorts`（`InstallPorts` の実装）・道 `run(startup, roots) -> BeforeBootResult`（依頼の順・読み取りだけの断り・消去・`callghost` の相手の決め）・兄弟のテスト `before_boot_tests.rs`。
- 継ぎ目: `boot_config` に「根を受けてゴースト以降を解く口」を足すか、今の `resolve_boot_with` をそのまま使い、`main.rs` の `resolve_boot(&opts.boot)` を `install::before_boot::resolve_boot(&opts.boot, startup_install)` へ差し替える（`main.rs` の行はほぼ同じ）。
- `startup.rs` は持ち物と純粋な判断（`sources_to_delete`・`end_word`・最後に入ったゴースト）だけに痩せ、定常の後の部品は消す。`desk.rs` は `on_steady`・`discard_for_exit`・`answer` の `OrderDone` から `startup::` の呼び出しを外す。
- ✅ 層の向きが今の形（install が起動の解決を呼ぶ側）と同じ・テストが閉じる・`main.rs` の上限を守れる。❌ ファイルが 2 つ増える・`DeskPorts` と `ask_terms`／`warn_skipped` の 2 行が重なる（共有するかは設計）。

### 案 C: 窓のアプリを先に立て、今の窓口の道を起動の前へ寄せる

- `WinApp` を起動の解決より前へ動かし、窓口（`desk`）と背景のスレッドを起動の前から動かす。口は窓口の `Facts` に「ゴーストが居ない」腕を足す。
- ✅ 画面が DPI の設定の後に出る（R1 が消える）・道が 1 本。❌ `main` の順の組み替え（起動の解決の失敗を窓のアプリの後で告げる形・`write_failure::check_boot`・MCP の順）が `main.rs` の上限を超えやすい。窓口は系の登録（`ghost_session::register_systems` の中）に依る＝**`ghost_session.rs` に触らない約束（③ `placement-measure-bake-once`）とぶつかりやすい**。規模は L 寄り。

### 比べ

| | A | B | C |
|---|---|---|---|
| 新しいファイル | 0 | 2（本体＋テスト） | 0〜1 |
| `main.rs` の行 | ±0〜+3 | ±0（差し替え） | +10 以上の組み替え |
| 約束（`ghost_session.rs`） | 触らない | 触らない | 触るおそれ |
| R1（DPI） | 残る | 残る（口の中で手当てできる） | 消える |
| 規模 | M | M | L |

## 5. 調べが要ること（Research Needed）

- **R1（高）: 起動の前の MessageBox と DPI**。`alert::ask_yes_no`／`warn_ok` は hwnd 無しで出せるが、`WinApp::with_exit_policy` の `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` より前に窓を作ることになる。`areka.manifest` は comctl32 v6 の申告だけで DPI の申告が無い。確かめること: ①先に窓を作った後でも `SetProcessDpiAwarenessContext` が成功するか（Microsoft の文書は「UI を作る前に呼ぶ」と書く）②起動の前の画面だけ `SetThreadDpiAwarenessContext` で包めば足りるか ③あるいは DPI の設定を起動の前へ動かすか（wintf の runtime を触る＝影響が広い）。失敗すると `areka-dpi-following-core-design`（DPI 追従が基本設計）を壊す。決定論のテストでは画面を出さない（`AREKA_NO_ALERT`）ので、テストでは見えない＝実機で 1 回確かめる。
- **R2（中）: 空の根が「見つかる根」になるか**。`CatalogDestinations::new` は `roots.present()` の先頭を要する。ゴーストもバルーンも無い根（`AREKA_ROOT` に空のフォルダ）が `present` に入るか、`ghost\` が無いときに `areka-nar` が作業フォルダを掘れるかを `roots/resolve.rs` と `areka-nar` の配置で確かめる。入らなければ要件 1.1 の「空の置き場」が error `install_no_root` で止まる。
- **R3（中）: `callghost` の相手を `--ghost` の形で渡すときの差**。名前で渡すと `resolve_switch_target` が「`name` が先・フォルダ名が後」で引くので、別のゴーストの `name` が入れたフォルダ名と同じだと取り違える。絶対パスで渡すと `Pointed::Path` になり `prepare_ghost`（`user_root::writable_ghost`＝書けない根から利用者の根へ写す）を飛ばす。入れた先が exe の隣の書けない根になる場合があるか（`CatalogDestinations::copied_root`）を確かめる。
- **R4（低）: 空の根の実機の足場**。`real_run_kit` の `Launch` に検体の無い根（バルーンだけ・または何も無い）を組む形があるか。無ければ `before_spawn` で根を組む今の形（バルーンを写す）を空の根へ広げる。入れる書庫 `konnoyayame.nar` はバルーンを持たないので、空の根ではバルーンが 0 で `BalloonMissing` に止まる＝根にバルーンだけは置く必要がある（要件 9.10 の「空の根」は「ゴーストが 1 体も無い」の意）。
- **R5（低・議題 2・4）: SSP の振る舞い**。`ssp.exe --install`（起動中の SSP が無いとき）でどのイベントが誰へ届くか・`type,ghost` の `initial_install.nar` の後にどのゴーストが起きるか。SSP の MCP と `ghost\master\ssp_shiori_log.txt` の差分で測れる（打つ前に開発者の許し）。

## 6. 設計の段への申し送り（決めること）

1. **起動の前の道の置き場**（案 A／B／C）。B が層の向き・約束・`main.rs` の上限のどれにも合う。
2. **継ぎ目の形**: `resolve_boot_with` をそのまま使い、根を先に解いて渡す形（呼ぶ所 37 か所は動かない）か、`resolve_boot` を「根」と「ゴースト以降」の 2 つの口に割る形か。
3. **イベントを送らない口の返し方**: (a) `Raised::NoReply` を返す（`procedure.rs` に触らない・ただし「送りました」の記録 `install_event` が出て `OnInstallComplete` まで「送る」）(b) `Raised` に「送らない」の腕を足し、`send` が記録を「送らない」に分ける（`procedure.rs` の `send` と `Raised` の照合に数行・記録が正しい）。要件 3.1・6.2 の読める記録には (b) が合う。
4. **`callghost` の段の形**: (a) 起動の求めの写しの `ghost` に入れたゴーストを入れて今の指し先の段に乗せる（`resolve_ghost` に触らない・経路の記録は `Pointed`・R3 の差）(b) `GhostInputs` に段を 1 つ足し `GhostRoute` に `CallGhost` を足す（`boot_resolve.rs` に触る・記録が正しい・`main.rs` 846 行の `Switched` の比べなど経路の照合を確かめる）。どちらでも、使わない理由の語（指定あり・ゴーストが入らなかった・読み取りだけ）と、`--ghost` が当たらなかった（`Pointed::Miss`）ときを「指定あり」と数えるかを決める（要件 4.5 の読み）。
5. **定常の後の道の始末**: `startup::on_steady`・`ArgsFollowup`・`discard_for_exit`・`on_order_done` の起動の引数の腕・`call_ghost`（切り替え）を消し、`startup_tests.rs`・`desk_startup_tests.rs` の陳腐化した本を除く範囲。`InstallOrigin::{StartupArgs, InitialInstall}` は記録と断りの語として残す。
6. **起動の前に入れた「最後に入れた物」の控え**: 窓口の `record_installed` が立てる資源（`LastInstalledGhost`・`LastInstalledShell`・`LastInstalledBalloon`）と置換語（`names::update`）は、起動の前には World が無い。(a) 運ばない（起動の前のインストールは起動中のゴーストの出来事ではない＝要件 3.2 と同じ読み・`\![change,ghost,lastinstalled]` は起動の前に入れた物を指さない）(b) 起動の持ち物に控えて World へ移す（要件 3.2 の「後から送らない」には反しないが、要件に書かれていない振る舞いを足す）。→ **要件の討議（2026-10-31）で (b) 引き継ぐに確定**（要件 3.4・9.12・10.6）。運び方は設計で決める。
7. **画面と DPI（R1）**: 起動の前の画面を `SetThreadDpiAwarenessContext` で包むか、DPI の設定を前へ動かすか、確かめた上で今のままにするか。
8. **記録の語**: 起動の前と読める形（`startup_install_done` に欄を足すか、新しい語にするか）・根が決まらないときの `startup_install_not_submitted` の理由の語（`boot_stopped` のままか `root_missing` か）・受け取るゴーストが居なくて断ったときの記録（`install_refused` の `running` が空の綴りになる所）。
9. **範囲の端**: `doc/pasta-real-run.md` の 248 行目（新しい書庫を窓へ落として入れる手順）を、この spec の説明書の段で 1 行の `--install` に改めるか → **要件の討議（2026-10-31）で直すに確定**（要件 8.7・10.7）。
10. **読み取りだけの断りの共有**: `install::submit` の先頭の断り（World の印）と起動の前の断り（プロセスの旗）を 1 つの関数にまとめるか、起動の前の道に同じ記録を書くか。

11. **（要件の討議で設計へ回した）R2・R3・R4 の確かめ**: 空の根が「見つかる根」に入るか（R2）・`callghost` の相手の渡し方（R3）・空の根の実機の足場（R4）。

## 7. 規模と危うさ

- **規模: M（7〜10 タスク）**。手続き・置き場・選び方は今ある部品の組み直しで、新しい境界や並行は無い。量は「起動の前の口と道」「`callghost` の段」「定常の後の道の撤去と 45 本のテストの作り直し」「説明書と実機の 2 本」にほぼ均等に分かれる。
- **危うさ: 中**。今の型（`InstallPorts`・`resolve_boot_with` の引数）の延長で済むが、R1（起動の前の画面と DPI）は実機でしか見えず、外れると DPI の基本設計に響く。R2（空の根が見つかる根か）は要件 1.1 の前提そのもの。

## 8. 同じウェーブの約束との照合（C14）

- `ghost_session.rs`: 案 A・B は触らない。案 C は窓口の登録が `ghost_session::register_systems` の中にあるため触るおそれ（③ との約束に反しうる）。
- `install/desk_overwrite_tests.rs`: 触らない（⑥ も触らない約束）。`desk.rs` の `on_steady`・`discard_for_exit`・`answer` の `startup::` の呼び出しを外すと、`desk_overwrite_tests.rs` の前提（定常の数）が変わらないかを確かめる。
- `boot_config.rs`・`install/`: ③ は触らない約束＝この spec だけが触る。
- `main.rs`: 986 行。差し替え 1 か所に留める。
- `dist/README.txt`: この spec だけ。
- `emo2_boot/ghost_switch.rs`: `callghost` が切り替えを頼まなくなれば、触らずに済む見込み（`startup.rs` の `use` を外すだけ）。

## 次の段

- 要件の討議（`kiro-requirements-discussion`）で、要件 10 の議題 1〜4 に加え、本書 §6 の 6（「最後に入れた物」の控え）と 9（`doc/pasta-real-run.md`）を要件の議題に回すかを決める。残りは設計の段（`/kiro-design install-before-boot`）で決める。
  → 2026-10-31 の要件の討議で議題 1〜7 を全部推奨案で確定した。設計の段の調べと決めは下の §9。

## 9. 設計の段の調べと決め（2026-10-31・`kiro-spec-design -y`）

### 9.1 要約

- **調べの種類**: 軽い調べ（今ある仕組みの延長）。新しいクレートの依存は無く、外の文書は Microsoft の `SetProcessDpiAwarenessContext` の 1 本だけを読んだ。
- **使ったスキル・決まり**: `kiro-spec-design` の決まり（design-principles・design-discovery-light・design-synthesis・design-review-gate）・`.kiro/settings/templates/specs/design.md`・steering（product・tech・structure・logging・roadmap のウェーブ C14 の約束）。
- **大きな見つけ**:
  - 継ぎ目は今のままで足りる: `boot_config::resolve_boot_with` は根の結果を引数で受けるので、根を先に解いて入れてから渡せる。`boot_config` は「起こすゴーストのフォルダ名」を 1 つ受けるだけでよく、install を知らない。
  - R1 は文書で答えが出る: UI を作る前にプロセスの DPI の扱いを立てる決まりがある。`wintf` の今の 1 段を「1 回だけの口」として外へ出せば、起動の前の画面より先に立てられる。
  - R2・R4 は今のコードで解けた（空のフォルダも見つかる根・`real-run-kit` の `Launch::Empty`）。

### 9.2 調べの記録

#### R1: 起動の前の画面と DPI
- **きっかけ**: 起動の前の利用条件と警告の画面（`alert::ask_yes_no`・`alert::warn_ok`＝持ち主の窓の無い `MessageBoxW`）は、`WinApp::with_exit_policy` が `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` を呼ぶより前に出る。
- **見たもの**: Microsoft「SetProcessDpiAwarenessContext function」 https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setprocessdpiawarenesscontext ／`crates/wintf/src/runtime/mod.rs` の `WinApp::with_exit_policy`（成否を info／warn で残す）／`crates/areka/areka.manifest`（comctl32 v6 の申告だけ・DPI の申告なし）／`crates/areka/src/ghost_session_scope_rect.rs` の `os_window_rect`（`SetThreadDpiAwarenessContext` で包む先例）。
- **分かったこと**:
  - 文書は「DPI の扱いに依る API を呼ぶ前（プロセスで UI を作る前を含む）に呼ばなければならない」「一度立てると、以後の呼び出しは失敗する（`ERROR_ACCESS_DENIED`）」と書く（逐語の引用は一句だけ: "before creating any UI in your process"）。
  - 今の起動を止める告知（`alert::raise`）も `WinApp` の前に `MessageBoxW` を出すが、出した後すぐ終わるので、後の DPI の設定には響かない。起動の前のインストールは、画面の後に起動が続く＝初めて「UI の後に DPI を立てる」形になる。
- **決め**: `wintf` に `ensure_process_dpi_awareness()`（`std::sync::Once` で 1 回だけ・記録は今と同じ）を外へ出し、`with_exit_policy` と、起動の前の口の画面の直前（告知の抑止が無いときだけ）から呼ぶ。プロセスの既定が画面より先に立つ。
- **比べた案**: ⑴ `SetThreadDpiAwarenessContext` で画面を包む（先例あり・小さい）＝プロセスの既定を立てる前に UI を作ることは変わらず、文書の決まりの外。後の `SetProcessDpiAwarenessContext` が成るかは実機の振る舞いしだい＝取らない。⑵ 申告（manifest）で PerMonitorV2 を立てる＝`wintf` の呼び出しが毎回 `ERROR_ACCESS_DENIED` の warn を出し、その記録で DPI の失敗を見分ける今の観測（`wintf` の注釈の Req 7.4）が壊れる＝取らない。⑶ `WinApp` を起動の解決より前へ動かす（案 C）＝`main.rs` の上限と `ghost_session.rs` の約束にぶつかる＝取らない。
- **残る確かめ**: 実機で 1 回（利用条件つきの書庫を起動の前に入れ、画面がぼやけない・`DPI awareness set` が 1 回だけ・窓が今どおり DPI を追う）。決定論のテストは画面を出さないので見えない。SSP は動かさない。

#### R2: 空の根は「見つかる根」か
- **見たもの**: `crates/areka/src/roots/resolve.rs` の `resolve_roots_from`（`found: dir.is_dir()`）・`areka_ghost::BasewareRoots::present`（`is_dir`）・`crates/areka-nar/src/install.rs`（作業フォルダと格納先の親を `create_dir_all`）。
- **分かったこと**: 中身の無いフォルダも根として見つかり、`CatalogDestinations::new` は作れる。`ghost\` が無くても `areka-nar` が作る。⇒ 要件 1.1 の「空の置き場」は今のまま入る（`install_no_root` にならない）。根のフォルダそのものが無いときは、今の根の告知（`NoneFound`）で止まる（要件 1.7）。

#### R3: `callghost` の相手の渡し方
- **見たもの**: `boot_config::point_ghost`（名前は `name` が先・フォルダ名が後）・`boot_resolve::resolve_ghost`・`install::destination::CatalogDestinations::copied_root`。
- **分かったこと**: `--ghost` の写しに名前を入れると、別のゴーストの `name` が入れたフォルダ名と同じときに取り違える。絶対パスにすると `Pointed::Path` になり書く前の写し（`prepare_ghost`）を飛ばし、`BootRequest::ghost` が `String` なので綴りの変わるパスで崩れうる。
- **決め**: `GhostInputs` に `call_ghost`（フォルダ名）を足し、記憶の段と同じ突き合わせ（`same_folder_name`）で入れた後の目録から引く。`GhostInputs` を組む所は本体 1・テスト 2 だけ。経路は `GhostRoute::CallGhost`（網羅の `match` は今のコードに無い）。書く前の写しも記憶の書き込みもほかの起動と同じに通る（要件 4.10）。

#### R4: 空の根の実機の足場
- **見たもの**: `crates/real-run-kit/src/launch.rs` の `Launch::Empty`（回ごとの空の根 `<回>\root`・`evidence.rs` が作る）と `before_spawn`。
- **分かったこと**: 足場の追加は要らない。`Launch::Empty` に `before_spawn` で既定のバルーンを根の `balloon\` へ写せば、要件 9.10 の「ゴーストが 1 体も無い根」になる。

#### R5: SSP の振る舞い（議題 2・4）
- 要件の討議で ukadoc の読みで決めた（測らない）。設計でも SSP を動かさない。測った答えが違えば要件 3.2・4.9 とテスト 9.4・9.5 が変わる（要件 10.2・10.4 のとおり）。

#### そのほかのコードの読み（設計の前提）
- `run_order` は口が `Raised::Closed` を返すか `ghost_facts` が `None` なら並びの残りもやめる（`install/procedure.rs` の `run_archive`・`archive_steps`）。⇒ 起動の前の口はこの 2 つを返さない。
- `judge_accept`（`install/judge.rs`）は空の素性で `accept` を持つ書庫を必ず `Refused`、`accept` の無いシェル・追加ファイルを `AcceptMissing` にする。`destination_of` は `folder: None` で `Elsewhere`。⇒ 起動の前の口に照合の特別な腕は要らない。
- `GhostRoute` の読み手は `== Argv`（`main.rs` の `argv_session`・`boot_resolve::LastUsed::record`）・`== Switched`（`main.rs`）・記録の `?route` だけ。
- 窓口の `record_installed`（`install/desk.rs`）が置く物は、どれもプロセスの中だけの資源（`LastInstalledGhost`・`LastInstalledShell`・`LastInstalledBalloon`・`LastInstallNames`）。`names::update` は起動中のゴーストが居なければ載せず資源だけを置き、`names::reseed`（`ghost_session_runtime::rewire_runtime_consumers` から）が起こしたゴーストへ載せる。⇒ 控えを `main` の今の行（`install::startup::install`）で移せば、起動の後に今の道で届く。
- `SwitchAcceptCount`（`emo2_boot/ghost_switch.rs`）の読み手は `install::startup` だけ。⇒ この spec で読み手が消えるので外す。
- `main.rs` の `readonly_tests.rs` の走査は `resolve_boot(&` の綴りを探す。`install::startup::resolve_boot(&opts.boot, …)` でも当たる。
- `desk_overwrite_tests.rs` の前提（定常の数）: `desk::on_steady` の `steady_count` を数える所と `overwrite::on_steady` は残るので変わらない。

### 9.3 案の比べ（置き場）

| 案 | 中身 | 良い所 | 危うい所 | 決め |
|---|---|---|---|---|
| A | `startup.rs` と `boot_config.rs` に足す | 新しいファイル 0 | `boot_config` が install を知る（層の向きが逆） | 取らない |
| B | 起動の前の道は `install::startup`、口は新しい `install/before_boot_ports.rs`、起動の解決は「起こすゴーストのフォルダ名」だけ受ける | 層の向きが今のまま・`main.rs` は呼ぶ所 1 か所・テストが閉じる | ファイルが 2 つ増える（口と口のテスト） | **取る** |
| C | `WinApp` を前へ動かして窓口の道を使う | R1 が消える・道が 1 本 | `main.rs` の上限・`ghost_session.rs` の約束・規模 L | 取らない |

### 9.4 設計の決め（§6 の 11 点への答え）

1. **置き場**: 案 B。道と持ち物と純粋な判断は `install/startup.rs`（定常の後の部品を消した跡へ）、口は新しい `install/before_boot_ports.rs`。
2. **継ぎ目**: 根を `install::startup::resolve_boot` が先に解き、入れてから `boot_config::resolve_boot(resolved, request, call_ghost, app_profile_dir)` へ渡す。本体は `resolve_boot_with_call`、今の `resolve_boot_with` はテストだけの 1 行の口（37 か所の呼び出しを動かさない）。関数を「根」と「ゴースト以降」に割らない。
3. **イベントを送らない返し方**: (b)。`Raised::Unsent` を足し、`send` は `install_event` を出さず debug `install_event_unsent`。`NoReply` を返す (a) は「送りました」の記録が 2〜4 件出て要件 3.1・6.2 と食い違う。
4. **`callghost` の段**: (b)。`GhostInputs::call_ghost`（フォルダ名）と `GhostRoute::CallGhost`、段は `--random` の後・記憶の前。「指定あり」は起動の求めに値が在るか（`--ghost` が当たらなかったときも指定ありと数える＝要件 4.5 の字のまま）。使わない理由の語は `readonly`・`boot_target_given`（`target`＝`positional`・`ghost_option`・`random`）・`no_ghost_installed`。
5. **定常の後の道の始末**: `startup::on_steady`・`ArgsFollowup`・`discard_for_exit`・`on_order_done`・`call_ghost`・`callghost_request`・旧い `CallGhostFacts`・`CallGhostSkip` の旧い腕を消す。`desk.rs` の呼び出し 3 行を外す。`desk_startup_tests.rs`（10 本）は消し、`startup_tests.rs` は `sources_to_delete` の 5 本と `new_with`・`Drop` の 2 本を残して書き直す（振る舞いが無くなった陳腐化のテスト）。`SwitchAcceptCount` を外す。
6. **「最後に入れた物」の運び方**: 口の `record` が目録の綴りに直した控え `CarriedInstall` を `StartupInstall` に積み、`main` の今の行 `install::startup::install` が World の資源へ入れた順に移す（`LastInstalled*` と置換語 `%lastobjectname`・`%lastghostname`）。綴りの決まりは窓口の `record_installed` から純粋な `catalog_spelling` を切り出して共有する。
7. **画面と DPI**: 上の R1 の決め（`wintf::ensure_process_dpi_awareness` を 1 回だけの口に）。
8. **記録の語**: `startup_install_started`／`startup_install_done` に `stage="before_boot"`・根が決まらないときは `reason=root_missing`（`Drop` の `boot_stopped` は守りとして残す）・受け取るゴーストが居なくて断ったときは口が info `install_refused_no_ghost` を足す（手続きの `install_refused` の `running` は空のまま）・バルーンは info `install_balloon_memory_skipped`。
9. **範囲の端**: `doc/pasta-real-run.md` を直す（要件の討議で確定）。
10. **読み取りだけの断り**: `install::record_readonly_refused(origin, count)` に切り出し、`submit` と起動の前の道から呼ぶ（記録は 1 か所）。
11. **R2・R3・R4**: 上の §9.2 のとおり（R2＝足りる・R3＝フォルダ名で目録から引く・R4＝`Launch::Empty`）。

### 9.5 まとめ直し（synthesis）

- **一般化**: 「起動の前に入れる」は「窓口の外から手続きを呼ぶ」の 1 つの場合で、口 `InstallPorts` の 2 つ目の実装がそれを受ける。`bootghost`（`install-package-type`）が来ても、起こすゴーストの名前は同じ `call_ghost` の口で渡せる（受け口の形は決めない・今は作らない）。
- **作るか借りるか**: 手続き・置き場・照合・告知は全部今の部品を使う。新しく作るのは口 1 つと段 1 つと DPI の口の切り出しだけ。
- **削ったもの**: 窓口と背景のスレッドを起動の前に使う形（案 C）・`--ghost` の写しで渡す形・「入れますか」の画面・途中でやめる扱い（口が閉じないので `Abandoned` は出ない）・callghost の「切り替え中」「途中で切り替わった」「起動中のゴーストと同じ」の判断（起動の前には起きない）・`SwitchAcceptCount`。

### 9.6 危うさと手当て

- **DPI（R1）**: 文書の決まりに沿う形にしたが、画面の見た目と後の窓の DPI 追従は実機でしか見えない → 実機の確かめの段で 1 回見る（決まりの手順は design の Testing Strategy）。
- **振る舞いの変わり（ゴーストが居る起動）**: 起動の引数で入れる書庫がゴーストへ知らせなくなる・`accept` を持つ書庫が入らなくなる → 説明書に書く（要件 8.2）。要件の討議 議題 1 で受け入れ済み。
- **テストの作り直しの量**: 約 40 本を除いて作り直す → 除くのは振る舞いが無くなったものだけ（純粋な判断のテストは残す）。
- **`main.rs` の上限**: 呼ぶ所の差し替えだけで 986 行のまま。
