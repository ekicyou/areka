# ギャップ分析: startup-install-option

> 測った日: 2026-10-29・本ブランチ（main `932d2c30` の上・`c018b2bd`）。コードは読むだけ（ビルド・テストは回していない）。
> 行番号はこの時点のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号は添え物。

## 1. まとめ（5 点）

- **入口の口と手続きは揃っている**: 依頼の唯一の口 `install::submit`（`crates/areka/src/install/mod.rs` 71 行目）は、書庫の並び 1 件を窓口の待ち行列に積み、読み取りだけなら積む前に断る。手続き `run_order`（`install/procedure.rs` 119 行目）は書庫ごとの終わり方 `ArchiveEnd` の並びを返す。起動の引数と `initial_install.nar` は、出どころの語を 2 つ足して `submit` へ渡せば、手続き・イベント・利用条件・警告・readonly の断りがそのまま効く。
- **足りない物は 3 つ**: ⑴ 起動オプションの読み手が `--install` の値と `callghost`・`deletesource` を「まだ扱わない」で捨てている（`cli/parse.rs` 93 行目・366 行目）。⑵ 書庫ごとの終わり方が UI スレッドへ届かない（背景のスレッドが `run_order` の結果を `debug!` に出すだけで、窓口へは中身の無い `DeskAsk::OrderDone` を送る＝`install/worker.rs` 67〜71 行目）。`deletesource`・`initial_install.nar` の消去と `callghost` の判断にはこれが要る。⑶ 「最初の定常到達で 1 回だけ」依頼を出す所が無い。
- **定常到達の口は `ghost_session.rs` に触らずに使える**: 定常到達は `ghost_switch::on_notice`（`emo2_boot/ghost_switch.rs` 491 行目）から毎回 `install::desk::on_steady`（`install/desk.rs` 127 行目）へ届く。初回の起動の定常到達も同じ道を通る（予約の外の腕・511〜515 行目）。`desk.rs` の中で「1 回だけ取り出す」形にすれば、`ghost_session.rs`（935 行）も `ghost_switch.rs`（926 行）も触らない。
- **候補の形**: 新しいファイル 2 つ（`cli/install_args.rs` と `install/startup.rs`）に判断を寄せ、今あるファイルには呼ぶ行だけを足す形（案 B）が、1,000 行の上限と並走の約束に最も合う。`main.rs` は 981 行（残り 19 行）なので、World へ値を渡す 1〜2 行か、プロセスの印（`readonly::is_on` と同じ形）のどちらかを選ぶ。
- **調べが要る所**: `callghost` の切り替えと、ゴースト自身が `OnInstallCompleteEx` の応えで出す `\![change,ghost,lastinstalled]` の競り合い。記録の出どころの語が、今は「依頼を受けた」「書庫を始めた」の 2 つの記録にしか載らないこと（要件 2.5）。SSP で `callghost` を測るか（打つ前に開発者の許し）。

## 2. 今あるもの（読み直し）

### 2.1 起動オプションの読み手（完了 `startup-options-core`・`readonly-mode`）

| 物 | 今 | 定義点 |
|---|---|---|
| `--install`（`-i`・`/i`） | 表の行 `spec("install", Some('i'), Arity::Many, Kind::Unsupported)`。値は「複数（次の起動オプションまで）」で読み、`ParseNote::Unsupported` を出して値ごと捨てる | `crates/areka/src/cli/parse.rs` の `TABLE`（93 行目）・`parse` の `Kind::Unsupported` の腕（334〜338 行目） |
| `--option` の語 | `kiosk`・`callghost`・`deletesource` は `ParseNote::WordPending`（warn `option_word_pending`）。`readonly` などの効く語は `effective` に並ぶ | 同じファイルの `--option` の語の分岐（343〜382 行目・366 行目） |
| `--offline-dump` との組み | `others` に名前を集め、`--offline-dump` があれば `OfflineIgnored` を出して `effective` から外す。今 `others` に入るのは `Ghost`・`Random`・`Option`・`SstpListen` と位置の引数だけ（327〜332 行目）＝`Kind::Unsupported` の `--install` は入らない | 同じ `parse`（449〜478 行目） |
| 読んだ中身 | `StartupOptions { boot, listen, modules, help, offline, readonly }`。インストールの欄は無い | `crates/areka/src/cli/mod.rs` の `StartupOptions`（16〜26 行目） |
| `main` の使い方 | `cli::read()` → `--help`・`--offline-dump` は窓より前に戻る → `readonly::engage` → 起動の解決 → `WinApp` → 系の登録 → `readonly::install(world)` → 起動の文脈 → 窓 → 最初のゴースト | `crates/areka/src/main.rs` の `main`（166〜329 行目）。981 行 |
| 相対パスの読み方 | `--offline-dump` は `std::path::absolute` で今のフォルダから絶対に直す。areka はどこでも `set_current_dir` を呼ばない | `crates/areka/src/offline_dump/mod.rs` 95・129・235 行目 |
| 今のテストで `--install` を「扱わない」と固定している所 | `unsupported("install")` を期待する 3 か所と、`callghost`・`deletesource` を `pending` と期待する 1 か所 | `crates/areka/src/cli/parse_tests.rs` 111〜118・285〜287・438〜444・512〜578 行目（674 行＝足すなら兄弟の新しいファイル） |

### 2.2 インストールの窓口と手続き（完了 `ghost-install`・`file-drop`・`nar-install` ほか）

| 物 | 今 | 定義点 |
|---|---|---|
| 依頼の型 | `InstallOrder { archives: Vec<PathBuf>, origin: InstallOrigin }`。出どころは `Menu`・`Script`・`WindowDrop` の 3 つ | `install/mod.rs` 33〜45 行目 |
| 唯一の口 | `submit(world, order)`: readonly なら `info!(readonly_refused, what=install, origin, count)` で断る → 書庫 0 本は `Empty`・窓口無しは `NoDesk`・終了中は `Closing` で `warn!(install_order_refused)` → 積めば `info!(install_order_queued, origin, count)` | `install/mod.rs` の `submit`（71〜110 行目） |
| 窓口 | World の NonSend `InstallDesk`。待ち行列・背景のスレッド・手元のイベントの頼み・定常到達の回数・門・上書きの預かり | `install/desk.rs` の `InstallDesk`（52〜77 行目）。560 行 |
| 定常到達の口 | `on_steady(world)`: 回数を数えて、起動中のゴーストへの上書きの段を進めるだけ | `install/desk.rs` の `on_steady`（127〜132 行目） |
| 定常到達の出どころ | `ghost_switch::on_notice` の `KanadeNotice::Steady` の腕の末尾で、予約の有無に関わらず毎回 `install::desk::on_steady` と `update::desk::on_steady` を呼ぶ。初回の起動・切り替えの後・読み直しの後・上書きで起こし直した後の、どの定常到達でも呼ばれる | `emo2_boot/ghost_switch.rs` の `on_notice`（491〜521 行目）。呼び手は `emo2_boot/frame.rs` 216 行目だけ |
| 背景のスレッド | `run_order` を走らせて結果 `ends` を `debug!(install_order_done)` に出し、窓口へは `DeskAsk::OrderDone`（中身なし）を送る。窓口はそれで `busy` を下ろすだけ | `install/worker.rs` の `spawn_worker`（60〜80 行目）・`install/desk.rs` の `answer`（292 行目） |
| 書庫ごとの終わり方 | `ArchiveEnd::{Installed(Vec<InstalledItem>), Refused, Declined, Failed(FailureWord), Abandoned}`。並びは書庫の順。`Abandoned` が出たらそこで抜ける（並びは短くなる） | `install/procedure.rs` の `ArchiveEnd`（103〜110 行目）・`run_order`（119〜144 行目） |
| 入った物の中身 | `InstalledItem { kind: "ghost"/"shell"/"balloon"/"supplement", name, place }`。先頭が本体。`place` は入れた先のフォルダのパス | `install/judge.rs` の `InstalledItem`（151〜155 行目）・`procedure.rs` の `installed_items`（423〜447 行目） |
| 入れた後の記録 | 書庫ごとに `DeskAsk::Record` で窓口へ。ゴーストなら `record_last_installed`（資源 `LastInstalledGhost` を最後の 1 件に置き換える） | `install/desk.rs` の `record_installed`（355〜395 行目）・`emo2_boot/ghost_switch.rs` の `record_last_installed`（234〜241 行目） |
| 利用者へ確かめる段 | 「入れますか」は無い。出る画面は、`terms.txt`／`terms.md` があるときの利用条件の はい／いいえ（`terms_step`）と、同梱を飛ばしたときの警告（`warn_skipped`）だけ。どちらも背景のスレッドで出す | `procedure.rs` の `terms_step`（290〜328 行目）・`worker.rs` の `ask_terms`（184〜186 行目）・`warn_skipped`（234〜237 行目） |
| 告知の抑止 | 環境変数 `AREKA_NO_ALERT` が立っていると、利用条件は拒否（`answer=suppressed`・warn）として扱い、警告は出さない | `procedure.rs` 319〜321 行目・`crates/areka/src/alert.rs` の `suppressed`（261 行目） |
| ゴーストが居ないとき | 手続きは `ghost_facts` が None なら「閉じた」として `Abandoned`（`install_abandoned` warn） | `procedure.rs` の `archive_steps`（192 行目）・`run_archive`（148〜167 行目）・`desk.rs` の `ghost_facts`（448〜466 行目） |
| 定常でないとき | `RaiseOutcome::NotSteady` なら、窓口が次の定常到達の後に送り直す＝定常の前に積んでも手続きは定常まで待つ | `worker.rs` の `raise`（165〜172 行目）・`desk.rs` の `send_held`（469〜511 行目） |
| 終了が始まったとき | `discard_for_exit` が待ち行列・手元の頼み・預かりを捨てて `install_pending_discarded`（warn）を 1 件 | `install/desk.rs` の `discard_for_exit`（268〜286 行目）。呼び手は `exit_wait::begin_close` |
| 起動中のゴーストへの上書き | 同じフォルダへの切り替え（出どころ `Automatic`・知らせなし）を自分で頼み、降ろした間に展開して起こし直す | `install/overwrite.rs` の `take`・`request`（49〜130 行目） |
| 記録に出どころが載る所 | `install_order_queued`・`install_order_refused`・`readonly_refused`・`install_begin` の 4 つ。`install_done`・`install_failed`・`install_refused`・`install_terms`・`install_abandoned` には出どころが無い（書庫のパスだけ） | `mod.rs` 75〜107 行目・`procedure.rs` 174〜179・208・259・312〜323・399 行目 |

### 2.3 切り替えの道（完了 `ghost-shell-balloon-switch`）

| 物 | 今 | 定義点 |
|---|---|---|
| 唯一の入口 | `request_ghost_switch(world, SwitchRequest { ghost, raise_event, origin, boot_event })` → `SwitchVerdict::{Accepted, Busy, NotFound, NoContext}`。切り替え中（`SwitchInFlight` が在る）なら `Busy` で重ねない | `emo2_boot/ghost_switch.rs` の `request_ghost_switch`（244〜323 行目） |
| メニューの「ゴースト」の頼み方 | `GhostSpec::Folder(folder)`・`raise_event: true`・`origin: ChangeOrigin::Manual`・`boot_event: None` | `crates/areka/src/menu/ghost_frame.rs` 51〜59 行目 |
| 出どころの語 | `ChangeOrigin::{Manual, Automatic}`。`OnGhostChanging` の Reference1 に `manual`／`automatic` として載る | `crates/areka-kanade/src/change.rs` 26〜43 行目 |
| 最後に入れたゴースト | 台本の特別な名前 `lastinstalled` は資源 `LastInstalledGhost` を読む（どの出どころの依頼でも、最後に入ったゴースト 1 件に置き換わる） | `emo2_boot/special_name.rs` 118 行目・`ghost_switch.rs` 281〜297 行目 |

### 2.4 起動の解決・exe の場所・readonly

| 物 | 今 | 定義点 |
|---|---|---|
| ゴーストが 0 体 | `resolve_ghost` が `Err(NoGhost)` → `resolve_boot_with` が告知の場面へ → `main` が `alert::raise` して終了コード 1。World も窓口も立たない | `crates/areka/src/boot_resolve.rs` の `resolve_ghost`（164〜242 行目・224 行目）・`boot_config.rs` の `resolve_boot`（200〜211 行目）・`main.rs` 210〜219 行目 |
| exe の本当の場所 | `boot_config::exe_location()`（`OnceLock`・リンクを辿る・解けなければ warn 1 回）。親のフォルダは `roots::user_root::exe_dir()` が返す（環境変数を読まず物も作らない） | `crates/areka/src/boot_config.rs` 145〜156 行目・`crates/areka/src/roots/user_root.rs` 159〜163 行目 |
| readonly の印 | プロセスの旗 `readonly::is_on()`（`engage` だけが立てる）と World の印 `readonly::of(world)`（`install` が足す） | `crates/areka/src/readonly.rs` 33〜47 行目 |

### 2.5 説明書と実機の足場

| 物 | 今 | 定義点 |
|---|---|---|
| まだ扱わない物の文 | `--option` の語 `kiosk・callghost・deletesource` と `--install・--fmo-name・--ssf・--windowmode` を 1 文に並べる | `dist/README.txt` 62 行目 |
| readonly の「断る物」 | 「.nar のインストール（メニュー・窓へ落とす・ゴーストの台詞から頼んだもの、どれも）・URL からのインストール・ネットワーク更新」。`--install` と `initial_install.nar` は無い | `dist/README.txt` 79 行目 |
| 実機の窓口 | `RealRun::arg` で起動の引数を足せる。子には必ず `AREKA_NO_ALERT=1` を渡す（＝利用条件つきの書庫は拒否になる）。位置の引数のゴーストが先、`arg` の引数が後に並ぶ | `crates/real-run-kit/src/launch.rs` の `arg`（124 行目）・`child_command`（191〜229 行目） |

## 3. 要件ごとの突き合わせ

印: **在る**＝今のまま使える／**足す**＝作る物／**不明**＝設計で調べる／**縛り**＝今の形から来る制約

| 要件 | 使える物 | 足りない物・印 |
|---|---|---|
| 1.1〜1.2 値を順に・相対パス | `Arity::Many` の読み方（315〜323 行目）・`std::path::absolute` の型（offline_dump） | **足す**: `Kind::Install`（または `Kind::Unsupported` から外す腕）と、`StartupOptions` のインストールの欄。絶対に直すのは読み手か受け手か（設計）。**縛り**: `-` か `/` で始まる値は起動オプションの名前として読まれる（`is_option`・173 行目）＝`/c/...` の綴りは使えない（今の説明書と同じ） |
| 1.3 値なし | `Arity::Many` で値 0 本になる | **足す**: warn の記録 1 件と「依頼を出さない」。`submit` へ 0 本を渡すと `install_order_refused`（`Empty`）が出てしまうので、手前で止める |
| 1.4 扱う語 | `--option` の語の分岐 | **足す**: `callghost`・`deletesource` を `kiosk` から分けて効く側へ（`effective` に並べる）。**縛り**: 今のテスト 438〜444 行目が `pending` を期待している |
| 1.5 `--install` なしの語 | `parse` は読み終えてから組みを見られる（`offline_given` の後の段と同じ所） | **足す**: warn 1 件（新しい `ParseNote`） |
| 1.6 `--offline-dump` との組み | `others` と `OfflineIgnored`。`--option` の語は名前 `option` が `others` に入るので既に効かない | **足す**: `install` を `others` に入れる（今は `Kind::Unsupported` で入らない）。`main` は `--offline-dump` で窓より前に終わるので、何も入れず何も消さないは自然に成る |
| 1.7 在らない書庫で止めない | 手続きの `NarArchive::open` の失敗 → `fail_nar` → `OnInstallFailure` | **在る** |
| 2.1 最初の定常到達で 1 回 | `desk::on_steady` は毎回の定常到達で呼ばれる | **足す**: 1 回だけ取り出す持ち物（`Option` を `take` する形なら自然に 1 回）と、`main` から World へ値を渡す道（下の 4.2）。**縛り**: 初回の起動で本物の結線が成らず `LogSink` へ倒れたときは定常到達が来ない見込み（**不明**・設計で確かめる） |
| 2.2〜2.4 同じ手続き・イベント・失敗で続ける | `submit` → `run_order` | **在る**（出どころは手続きを分けない＝`mod.rs` 38 行目の注） |
| 2.5 記録に出どころ | `InstallOrigin` を `?origin` で出す 4 つの記録 | **足す**: 語 2 つ（例 `StartupArgs`・`InitialInstall`）。**不明**: 「入った・失敗した・断った」の記録（`install_done`・`install_failed`・`install_refused`）には今 `origin` が無い。⑴ 手続きの記録に `origin` を足す（`fail` などの引数が増え、`procedure.rs`・兄弟のテストに波及）か、⑵ 「書庫を始めた」の記録（`install_begin` は `origin` を持つ）と書庫のパスで突き合わせて読めれば足りるとするか、⑶ 起動の引数の依頼が終わったときに、書庫ごとの終わり方と出どころを並べた記録を 1 件足すか（下の 4.3 と一緒に作れる）＝議題 6 |
| 2.6 待たせない | 背景のスレッド・窓口の送り直し | **在る**（議題 4 で「待たせない」なら足す物は無い） |
| 2.7 定常の前に終了 | `discard_for_exit`（終了の待ちの登記） | **足す**: 出していない起動の依頼が残っていたら warn 1 件（`discard_for_exit` から呼ぶ 1 行か、`install/startup.rs` を別に登記するか） |
| 2.8 途中で終了 | `run_order` の `Abandoned` と `install_abandoned` | **在る** |
| 3.1〜3.6 `deletesource` | `run_order` が `ArchiveEnd` の並びを返す（書庫の順と同じ添字・`Abandoned` で短くなる） | **足す**: 並びと書庫のパスを UI スレッド（または背景のスレッド）で受けて消す所（下の 4.3）・同じパスの重なり・消せないときの warn・info の記録。**縛り**: 書庫を入れ終えた後に締めの知らせ（`OnInstallCompleteEx`）の段で「閉じた」になると、ファイルは入っているのに終わり方は `Abandoned`（`procedure.rs` 269〜285 行目の `closing?`）＝要件 3.1 のとおりなら残す（安全側）。**不明**: 同じ書庫を綴り違い（大小・相対と絶対）で 2 回書いたときに同じと見るか |
| 4.1 exe の隣を 1 回見る | `roots::user_root::exe_dir()`（`exe_location` の親） | **足す**: 見る所。Windows のファイルの名前は大小を区別しないので、`exe_dir.join("initial_install.nar").is_file()` で大小を問わず当たる（記録に出す名前は書いた綴りになる）。**縛り**: 開発中の exe は `target\debug\` に在る＝テストは exe の場所を差し込めないと本物のフォルダを見る（下の 6） |
| 4.2〜4.6 同じ手続きで入れて、入ったら消す | `submit` と 3.x と同じ消す所 | **足す**: 語 `InitialInstall`・消す所（`deletesource` と同じ部品を使える）。無いときは debug 以下だけ |
| 4.7 両方あるときの順 | 待ち行列は届いた順（`queue.push_back`） | **足す**: 同じ tick に `initial_install.nar` を先に、起動の引数を後に `submit` する（2 件の依頼） |
| 4.9 `--offline-dump` | `main` が窓より前に戻る（181〜183 行目） | **在る**（見る所を World の側に置けば自然に成る） |
| 4.10 利用条件と警告の画面 | 今の手続き | **在る**（議題 3 の仮の形なら足す物は無い）。**縛り**: `AREKA_NO_ALERT` の下では利用条件は拒否になる＝実機の窓口の試験で利用条件つきの書庫は入らない |
| 5.1 `callghost` の切り替え | `request_ghost_switch`・メニューの頼み方（`Folder`・`raise_event: true`・`Manual`） | **足す**: 終わり方の並びから「最後に入ったゴースト」を選ぶ純粋な関数（`Installed(items)` の先頭が `kind == "ghost"` の書庫の `place` のフォルダ名）と、それを頼む所。**不明**: `OnGhostChanging` の Reference1 を `manual` にするか `automatic` にするか（議題 7） |
| 5.2 起動中のゴーストだった | `BootContext.current.ghost.folder`・`same_folder_name` | **足す**: 比べて頼まない＋info。上書きは窓口が自分で同じフォルダへの切り替えを済ませている（`overwrite.rs`） |
| 5.3 途中で切り替えが始まった・終わった | `SwitchInFlight` の有無（在れば入口が `Busy`） | **不明**: 「始まって終わった」を見分ける物が無い。案: 依頼を出したときの今のゴーストのフォルダを控え、終わったときと比べる（上書きの再起動は同じフォルダなので混ざらない。A→B→A は見分けられない）。**縛り**: ゴースト自身が `OnInstallCompleteEx` の応えで `\![change,ghost,lastinstalled]` を出すと、その切り替えは `OrderDone` の前後どちらにも来うる＝競り合い（議題 8） |
| 5.4〜5.8 | 入口の判定と記録 | **足す**: 頼まなかった理由の info・頼んだ info（出どころ `callghost`）。2 体目は作らない（今の areka は 1 体だけ）＝**在る** |
| 6.1〜6.5 readonly | `submit` の先頭の断り（`readonly::of`）・`readonly_refused` に `origin` | **在る**。消す所と切り替えの所は、`submit` が `ReadOnly` を返したら何もしない形にすれば自然に成る。**足す**のはテストだけ |
| 7.1 ゴーストが居ない | 起動の解決が `Err` → `main` が告知して終わる | **足す**: warn 1 件。**縛り**: World ができる前に終わるので、`main.rs` の `Err(scene)` の腕（残り 19 行）か、`boot_config::resolve_boot`（薄い口）で出す。後者は起動の依頼を見る道（プロセスの印など）が要る |
| 7.2〜7.4 失敗の伝え方 | 記録の型（`error!`・`warn!`）・メッセージボックスを出さない決まり | **足す**: 足す道の記録。`submit` が `Queued` 以外を返したら消さず切り替えない |
| 8.1〜8.5 説明書 | `dist/README.txt` 62・79 行目 | **足す**: 62 行目から 3 つを外し、新しい段落。79 行目に 2 つ足す。**縛り**: 同じウェーブの `default-balloon-selfmade`・`bridge-store-user-root` も `dist/README.txt` を触る（別の行＝許す重なり） |
| 9.1〜9.7 決定論のテスト | `cli/parse_*_tests.rs` の型・`install/submit_readonly_tests.rs`・`install/desk_tests.rs` の窓口の rig・`procedure_test_support.rs` の偽の口 | **足す**: 兄弟の新しいテストのファイル。今のテストの書き換え（2.1 の表の 4 か所・`DeskAsk::OrderDone` を形で読む 4 か所＝`desk_tests.rs` 250・262 行目・`worker_tests.rs` 344・398 行目） |
| 9.8 実機 | `real-run-kit` の `RealRun::arg`・MCP の `get_active_ghost_list` | **足す**: 実機の試験 1 本。**不明**: `initial_install.nar` の実機の確かめは exe の隣に置く必要がある＝`target\debug\` に写しを置くか、exe を `target\` の下の別の所へ写して起こすか（helper の隣の決まりがある）。決定論のテストだけにする手もある |

## 4. 実装の形の候補

### 4.1 全体の形

#### 案 A: 今のファイルに書き足す

- 触る: `cli/parse.rs`（`--install` の腕と 2 つの語）・`cli/mod.rs`（欄）・`install/mod.rs`（語 2 つ）・`install/desk.rs`（1 回だけの依頼・終わり方を受けて消す・`callghost` の判断を全部）・`install/worker.rs`（終わり方を窓口へ渡す）・`main.rs`（値を渡す）・`dist/README.txt`。
- ✅ ファイルが増えない。
- ❌ `desk.rs`（560 行）に、消す・切り替える・1 回だけ出す判断が入り、窓口の役目（World の仕事を 1 か所に）から外れる。`cli/parse.rs`（645 行）も膨らむ。判断を純粋な関数で切り出しにくく、テストが窓口の rig に寄る。

#### 案 B: 新しいファイルに判断を寄せ、今あるファイルには呼ぶ行だけ（brief の Approach の形）

- 新しい `crates/areka/src/cli/install_args.rs`＋兄弟のテスト: `--install` の値の並び（絶対に直す）・`callghost`・`deletesource`・組みの記録（1.3・1.5）。`parse.rs` は腕から呼ぶだけ。
- 新しい `crates/areka/src/install/startup.rs`＋兄弟のテスト: 起動の依頼の持ち物（`initial_install.nar` の有無・起動の引数の並び・2 つの語）・最初の定常到達で 1 回だけ `submit` する口・終わり方の並びから「消す書庫」と「切り替え先」を決める純粋な関数・消す所・`callghost` を頼む所・終了で残っていたときの記録。
- 今のファイルに足す行: `install/mod.rs`（`mod startup;`・語 2 つ）・`install/desk.rs`（`on_steady` から 1 行・`answer` の `OrderDone` の腕から 1 行・`discard_for_exit` から 1 行）・`install/worker.rs`（`OrderDone` に終わり方と依頼を載せる）・`main.rs`（1〜2 行）・`dist/README.txt`。
- ✅ `ghost_session.rs`・`ghost_switch.rs` に触らない（同じウェーブの約束）。判断が純粋な関数になり、要件 9 のテストがそのファイルの兄弟に置ける。`desk.rs` は 10 行ほどの増え。
- ❌ ファイルが 2 つ（＋テスト 2〜3 つ）増える。窓口と `startup.rs` の間の受け渡しの形を決める必要がある。

#### 案 C: 消す所を背景のスレッドへ寄せる（案 B の変形）

- `InstallOrder` に「入ったら消す」の印を足し、背景のスレッドが `run_order` の直後に消す。`callghost` だけ UI スレッドで扱う。
- ✅ 消すのが手続きの終わりと同じ所で起き、UI スレッドで I/O をしない。
- ❌ `InstallOrder { .. }` を組む所すべて（`input_events/file_drop.rs`・`install/fetch_url.rs`・`emo2_boot/install_cue.rs`・`install/desk.rs` の `drain`・`send_pick` と、テストの 10 ファイルほど）に欄が増える。`callghost` のためには結局終わり方を UI スレッドへ渡す道が要るので、道が 2 本になる。

### 4.2 起動の値を World へ渡す道（どの案でも要る）

| 形 | 中身 | 長所 | 短所 |
|---|---|---|---|
| H1: `main` から World へ | `readonly::install(world)` と並べて `install::startup::install(world, opts.install)` の 1 行 | 値の流れが目で追える。テストは World に持ち物を足すだけ | `main.rs` の残り 19 行から 1〜2 行を使う。要件 7.1（ゴーストが居ないときの記録）は別に `main` の `Err` の腕へもう 1 行要る |
| H2: プロセスの印 | `cli::read` が読み終えたときに静的な所（`OnceLock`）へ置き、`install::startup` と `boot_config::resolve_boot` が読む（`readonly::is_on` と同じ形） | `main.rs` を触らない。要件 7.1 の記録を `resolve_boot` の中で出せる | プロセスに 1 つの値はテストで差し替えにくい（判断の関数は引数で受け、印を読むのは薄い口だけにする必要がある） |

### 4.3 書庫ごとの終わり方を受ける道

| 形 | 中身 | 長所 | 短所 |
|---|---|---|---|
| R1: `OrderDone` に載せる | `DeskAsk::OrderDone { order, ends }`。窓口の `answer` が `busy` を下ろした後、出どころが起動の 2 つなら `startup.rs` へ渡す | 道が 1 本。依頼と結果が 1 つの知らせで揃う | 形で読むテスト 4 か所の書き換え（`{ .. }` を足す） |
| R2: 新しい頼みを足す | `DeskAsk::OrderEnded { order, ends }` を `OrderDone` の前に送る | 今のテストを書き換えない | 知らせが 2 つになり、順の約束が増える |
| R3: 記録の頼みから拾う | 書庫ごとの `DeskAsk::Record`（入った物の種類とフォルダ） | 足す物が少ない | 書庫のパスが載っていない＝`deletesource` に使えない。断った・拒んだ・失敗が分からない |

### 4.4 「最初の定常到達で 1 回」の形

| 形 | 中身 | 長所 | 短所 |
|---|---|---|---|
| T1: `on_steady` で取り出す | `desk::on_steady` から `startup::on_steady(world)` を呼び、持ち物を `take` して `submit` する（2 回目からは空） | 要件 2.1 のとおり。ゴーストが定常に着かなければ出さない＝要件 7.1 と同じ考え方 | 1 行だけ `desk.rs` に足す |
| T2: 起動のときに積んでおく | 系の登録の後・`app.run()` の前に `submit` する。手続きは定常の前なら送り直しで待つ（`NotSteady`） | 定常到達の口が要らない | 定常に着かない走り（結線が成らない）では手元の頼みが残り続け、窓口が「忙しい」のまま（読み直しの入口が断る＝`is_busy`）。記録の出どころの順が要件 2.1 の言い方と合わない |

## 5. 規模と危うさ

- **規模: M（6〜9 タスク）**。起動オプションの読み手（1）・出どころの語と起動の依頼の持ち物（1）・終わり方の受け渡し（1）・消す所（1）・`callghost`（1〜2）・`initial_install.nar`（1）・説明書（1）・実機（1）。議題 2 の別案（ゴーストが居ないときに起動の解決の前に入れる）を取ると、窓のアプリの外から手続きを呼ぶ道が要って 3〜4 タスク増える。
- **危うさ: 中**。手続きそのものは変えず、今の口へ渡すだけなので土台は低い。中にするのは、⑴ `callghost` とゴースト自身の切り替えの競り合い、⑵ 書庫を消す（取り消せない）判断が終わり方の読み違いで誤ると利用者の書庫が消える、⑶ `main.rs` と `ghost_switch.rs` が上限の近くで、結線の場所の選び方を誤ると約束を破る、の 3 つ。

## 6. テストの置き場と根

- 起動オプション: `cli/parse_install_tests.rs`（新・`parse_readonly_tests.rs` と同じ型）。今の `parse_tests.rs` の 4 か所は期待を書き換える。
- 判断の関数（消す書庫・切り替え先・頼まない理由）: `install/startup_tests.rs`（新）。`ArchiveEnd` を並べるだけで決まる純粋な関数にすれば、ファイルにも World にも触らずに 5 つの終わり方を 1 つずつ確かめられる。
- 消す所: 書庫の写しは `target\` の下の一時の根（`temp-path-kit`）に作る。本物の書庫も `vendors/` の検体も使わない（要件 9.7）。
- `initial_install.nar`: 見るフォルダを引数で受ける関数にし、テストは `target\` の下のフォルダを渡す（`exe_dir()` は開発中は `target\debug\` を返すので、薄い口以外では読まない）。
- readonly: `install/submit_readonly_tests.rs`（93 行）に出どころの語 2 つの行を足すか、`startup_tests.rs` に置く。
- 窓口との結線（1 回だけ・終わり方の受け渡し・2 件の順）: `install/desk_tests.rs` は 766 行＝足さずに兄弟の新しいファイル（例 `desk_startup_tests.rs`）。

## 7. 設計の段へ持ち越す調べ（Research Needed）

1. 初回の起動で本物の結線が成らず `LogSink` へ倒れたとき、定常到達（`KanadeNotice::Steady`）が来るか。来ないなら起動の依頼は出ないまま終わる（要件 7.1 の記録をどこで出すか）。
2. ゴースト（例 ぱすたさん・emo2）が `OnInstallCompleteEx` に `\![change,ghost,lastinstalled]` を返すか。返すなら `callghost` の切り替えと競る。窓口の `OrderDone` と、台本の切り替えの要求（`ChangeRx` の取り出し）のどちらが先に UI スレッドへ届くかの順。
3. SSP で `--install x.nar --option callghost` のときに、送り出す側へ `OnGhostChanging` が出るか・Reference1 は何か・切り替えか 2 体目か（**打つ前に開発者の許し**・`ssp_shiori_log.txt` で差分を読む）。
4. 同じ書庫を綴り違い（大小・相対と絶対・短い名前）で並べたときに同じと見る比べ方（`std::path::absolute` の後に大小を問わず比べれば足りるか）。
5. `initial_install.nar` の実機の確かめを、exe の隣（`target\debug\`）に写しを置いて行ってよいか、決定論のテストだけにするか。
6. readonly の走りで、`exe_dir` が書けない場所（例 `Program Files`）に置かれた `initial_install.nar` が入った後に消せないとき、次の起動のたびに上書きが繰り返されることの記録の量（warn が毎回 1 件で足りるか）。

## 8. 要件討議へ渡す議題（答えで作業が変わる物）

brief の議題 1〜4 と要件の段の議題 5 は要件 10 のとおり。この分析で増えたのは 6〜9。

1. （要件 10.1）`callghost` を切り替えに読むか・裁定どおり記録だけにするか。
2. （要件 10.2）ゴーストが 1 体も無いときは入れない（仮）か、起動の解決の前に入れるか。
3. （要件 10.3）`initial_install.nar` でも利用条件と警告を出す（仮）か。実機の窓口は `AREKA_NO_ALERT=1` なので、利用条件つきの書庫は拒否になる点も合わせて。
4. （要件 10.4）待たせない（仮）か。
5. （要件 10.5）`initial_install.nar` を先に別の依頼で・2 つの語は `--install` にだけ効く（仮）か。
6. **記録の出どころの語を、どこまでの記録に載せるか**（要件 2.5）。今 `origin` が載るのは「受けた」と「書庫を始めた」だけ。⑴ 手続きの記録（入った・失敗・断った・利用条件）全部に足す（`procedure.rs` と兄弟のテストに波及）⑵ 「書庫を始めた」の記録との突き合わせで足りるとする ⑶ 起動の依頼が終わったときに、書庫ごとの終わり方と出どころを並べた記録を 1 件足す（推し: ⑶＝手続きに触らず、`deletesource` の判断と同じ所で出せる）。
7. **`callghost` の切り替えの出どころの語（`OnGhostChanging` の Reference1）を `manual` にするか `automatic` にするか**。メニューと同じ道なら `manual`（利用者が起動の行で選んだ）。`automatic` は台本の切り替えの語。
8. **`callghost` とゴースト自身の切り替えが競ったとき**。⑴ 依頼が終わった時点で切り替えの予約が在れば頼まない（入口の `Busy` に任せる）・終わった後に来たゴーストの切り替えは今どおり ⑵ 依頼が終わってから一拍待ってゴーストの切り替えが無ければ頼む（待ちの長さを決める必要がある・時刻の扱いの大原則に触れる）。推し: ⑴（同じゴーストへ向かうなら結果は同じ・後から来た側が `Busy` で記録に残る）。
9. **起動の値を World へ渡す道**: `main.rs` に 1〜2 行（残り 19 行）を使うか、`readonly::is_on` と同じプロセスの印にして `main.rs` を触らないか。要件 7.1（ゴーストが居ないときの記録）をどこで出すかと一緒に決まる。

## 9. 設計の段への申し送り

- 推す形: 案 B（新しい `cli/install_args.rs`・`install/startup.rs`）＋R1（`OrderDone` に終わり方を載せる）＋T1（`on_steady` で 1 回だけ取り出す）。H1／H2 は議題 9 の答えで決める。
- 守る約束: `ghost_session.rs`（935 行）と `ghost_switch.rs`（926 行）に触らない（定常到達は `desk::on_steady` で受ける）。`main.rs` は 981 行。`desk.rs` に判断を入れず呼ぶ行だけ。`dist/README.txt` は 62 行目と 79 行目と新しい段落だけ（同じウェーブの 2 本と別の行）。依存は足さない。
- 要件討議（2026-10-29）で、§8 の 6（記録の出どころの語をどこに載せるか・要件 2.5 は「記録から読める」に直した）・8（`callghost` とゴースト自身の切り替えの競り合いの見分け方）・9（起動の値を World へ渡す道と、要件 7.1 の記録の置き場）は設計の段で決める事項に回した。§4 の案 A／B／C・R1〜R3・T1／T2・H1／H2 と §7 の調べ 1〜6 も同じ。
- 消す判断は「終わり方が `Installed` の書庫だけ」を純粋な関数 1 つに閉じ込め、5 つの終わり方を 1 つずつ檻に入れる（取り消せない操作なので、判断の分岐はすべて固定する）。

---

# 設計の段の調べと決め（2026-10-29・kiro-spec-design）

> 設計の段で、§4 の案 A／B／C・R1〜R3・T1／T2・H1／H2、§7 の調べ 1〜6、§8 の議題 6・8・9 を決めた。
> 行番号は本ブランチ `fbf3d87e` のもの。コードは読むだけ（ビルド・テストは回していない）。
> 使った手引き: スキル `kiro-spec-design` の `rules/design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、ひな形 `.kiro/settings/templates/specs/design.md`・`research.md`、steering の `structure.md`（テストの兄弟のファイル）・`logging.md`（記録の形と `log-capture-kit`）。外の調べ（Web）は要らない（新しい依存なし・今ある部品だけ）。

## まとめ

- **調べの種類**: 軽い調べ（今ある仕組みの拡張）。手続き・切り替え・readonly の断りは今のまま使い、入口と後始末だけを足す。
- **要の発見**:
  - 定常到達の知らせは、SHIORI の結線（本物の sink か `LogSink` か）に関わらず、kanade の起動の段が `basewareversion` の後に 1 件出す（`crates/areka-kanade/src/schedule/boot.rs` 133 行目）。初回の起動の定常到達も `ghost_switch::on_notice` → `install::desk::on_steady` を通る（`ghost_switch.rs` 494〜517 行目）。＝T1 で足りる。
  - 終了の後始末 `exit_wait::begin_close` は、利用者の終了（`main.rs` 348 行目）と OS のセッションの終了（`session_end.rs` 101 行目）の 2 か所から呼ばれ、どちらも `install::desk::discard_for_exit` を呼ぶ。`app.run()` が戻った後は取り出しの系 `drain` が回らない＝終了の後に届いた `OrderDone` は捌かれない。
  - 書庫ごとの終わり方 `ArchiveEnd` は背景のスレッドで `run_order` が返すが、窓口へは中身の無い `DeskAsk::OrderDone` しか届かない（`worker.rs` 67〜71 行目・`desk.rs` 292 行目）。

## 調べ（§7 の 1〜6 の答え）

### 調べ 1: `LogSink` へ倒れたときに定常到達が来るか
- **見た所**: `areka-kanade/src/schedule/boot.rs` 125〜135 行目（`Phase::BootVersion` + `Notified` → `Action::Notice(KanadeNotice::Steady)`）・`emo2_boot/frame.rs` 205〜218 行目（`run_ghost_quit_phase` が `Stopped` 以外を `ghost_switch::on_notice` へ）。
- **分かったこと**: 定常到達は kanade の起動の段が出す。sink の結線（`wire_emo2_boot` の成否）とは関わらない。kanade が走り SHIORI が `OnBoot` と `basewareversion` に答えれば、`LogSink` の起動でも定常到達は来る。定常到達が来ない走りでは起動の依頼は出ず、終了の後始末で要件 2.7 の記録が出る。
- **設計への効き**: T1（`on_steady` で 1 回だけ取り出す）を取る。定常到達が来ない走りのための別の道は作らない。

### 調べ 2: ゴーストが `OnInstallCompleteEx` に `\![change,ghost,lastinstalled]` を返すときの順
- **見た所**: `install/procedure.rs` 269〜285 行目（締めの知らせの後に `Installed` を返す）・`worker.rs` 64〜79 行目（`run_order` の後に `OrderDone`）・`emo2_boot/ghost_switch.rs` 71〜93 行目（台本の切替要求は `ChangeRx` から入力の段で入口へ）・244〜256 行目（予約が在れば `Busy`）。
- **分かったこと**: 背景のスレッドは kanade の返事（SHIORI の応え）を受けた時点で次へ進む。応えの台本の中の `\![change,...]` は、トークがその所まで進んだときに受け口から送られる。だから多くは `OrderDone` が先に UI スレッドへ届く。ただし応えが短い・2 本目以降の書庫が残っている、などで台本の切替要求が先に届くこともある。どちらが先かは決まらない。
- **設計への効き**: 待ちの時間を置かない（時刻の大原則）。依頼を出した時点と終わった時点の今のゴーストを比べ、終わった時点に切替の予約が在るか、今のゴーストが変わっていれば `callghost` を頼まない。`callghost` が先に受理されたら、後から来たゴーストの切替要求は今の入口の `Busy`（warn `ghost_switch_busy`）で記録に残る（議題 8 の ⑴）。

### 調べ 3: SSP で `callghost` を測るか
- 要件 10.1 のとおり測らない（測った答えがどちらでも作業は同じ）。

### 調べ 4: 同じ書庫の綴り違い
- **見た所**: `offline_dump/mod.rs` 95・129・235 行目（`std::path::absolute`）。Windows の `std::path::absolute` は `GetFullPathNameW` を使い、`.`・`..`・区切りの向きを整える（短い名前 8.3 は長くしない）。
- **決め**: 受け取った時点で `std::path::absolute` で絶対に直し、比べるときは文字列を小文字にして比べる。短い名前と長い名前の違い・リンクは同じと見ない。見損ねても、2 つ目の綴りは「既に無い」で消せなかった warn が 1 件増えるだけで、ほかの書庫を消すことは無い（安全側）。

### 調べ 5: `initial_install.nar` の実機の確かめ
- **決め**: 実機の確かめはしない。決定論のテストだけにする。理由: 開発中の exe は `target\debug\` に在り、そこはこのワークツリーのすべての試験が共有する。そこへ `initial_install.nar` を置くと、同じ時に走るほかの実機の試験の areka がそれを入れて消す（試験どうしが汚し合う）。exe を別の所へ写して起こす形は、隣の 32 ビットの仲介などの決まりを崩す。見る所は引数でフォルダを受ける関数にし、テストは `target\` の下のフォルダを渡す（要件 9.4・9.7）。

### 調べ 6: 消せない `initial_install.nar` の記録の量
- **決め**: 1 回の走りで、見るのは 1 回・消すのも 1 回なので、消せなければ warn が走りごとに 1 件。毎回上書きで入ることは説明書に書く（要件 4.5・8.3）。読み取りだけの走りは入れも消しもしない（断りの info 1 件だけ）。

## 形の比べ（§4 の結論）

| 候補 | 中身 | 取る／取らない | 理由 |
|---|---|---|---|
| 案 A | 今のファイルに全部書き足す | 取らない | `desk.rs` に消す・切り替える判断が入り、窓口の役目から外れる |
| 案 B | 判断を新しいファイルへ・今のファイルには呼ぶ行だけ | **取る（縮めて）** | `ghost_session.rs`・`ghost_switch.rs` に触らない。判断が純粋な関数になる |
| 案 C | 消す所を背景のスレッドへ | 取らない | `InstallOrder` を組む所すべてに欄が増える。`callghost` のために結局終わり方を UI へ渡す道が要り、道が 2 本になる |
| R1 | `OrderDone { order, ends }` | **取る** | 道が 1 本。依頼と結果が 1 つの知らせで揃う。書き換えるテストは 4 か所 |
| R2 | 別の知らせを足す | 取らない | 知らせの順の約束が増える |
| R3 | 記録の頼みから拾う | 取らない | 書庫のパスと失敗が分からない |
| T1 | `on_steady` で 1 回だけ取り出す | **取る** | 要件 2.1 のとおり・調べ 1 で定常到達が来ると確かめた |
| T2 | 起動のときに積んでおく | 取らない | 定常に着かない走りで窓口が忙しいままになる |
| H1 | `main` から World へ（2 行） | **取る** | 値の流れが目で追える。テストは World に持ち物を足すだけ。`main.rs` は 981 → 983 行 |
| H2 | プロセスの印 | 取らない | テストで差し替えにくい。`readonly` の印と違い、値（書庫の並び）を持つ |

### 縮めたこと（Simplification）
- 案 B の新しい `cli/install_args.rs` は作らない。`--install` の腕と 2 つの語は `cli/parse.rs` の今の腕に数行ずつで入り（645 → 約 690 行）、値の型 `InstallArgs` は `cli/mod.rs` に置く。テストだけ兄弟の新しいファイル `cli/parse_install_tests.rs` に置く（`parse_readonly_tests.rs` と同じ形）。
- 起動の依頼の持ち物・判断・消す所・`callghost` を頼む所は、新しい `install/startup.rs` 1 つにまとめる（約 250 行）。
- 「入ったら消す」の印を `InstallOrder` に足さない（案 C を取らない）。消す判断は終わり方の並びだけから決める純粋な関数 1 つ。

### 一般化（Generalization）
- `deletesource` と `initial_install.nar` の「入った書庫だけ消す」は同じ問題。純粋な関数 `sources_to_delete(archives, ends)` 1 つと消す所 1 つを両方が使う（違うのは、`initial_install.nar` は語なしでいつも消す、の 1 点）。
- 記録の出どころの語は、今の `InstallOrigin` に 2 つ足すだけで、`install_order_queued`・`install_order_refused`・`readonly_refused`・`install_begin` の 4 つの記録に自然に載る。

### 作るか借りるか（Build vs Adopt）
- 新しい依存は無い。パスの絶対化は `std::path::absolute`（`offline_dump` と同じ）、消すのは `std::fs::remove_file`、exe の場所は `roots::user_root::exe_dir()`、切り替えは `request_ghost_switch`、readonly の断りは `install::submit` の先頭。作るのは、1 回だけ出す持ち物と、消す／頼むの判断だけ。

## 決め（§8 の議題 6・8・9 ほか）

### 決め: 記録の出どころの語の載せ方（議題 6）
- **候補**: ⑴ 手続きの記録全部に `origin` を足す ⑵ 「書庫を始めた」の記録との突き合わせで足りるとする ⑶ 起動の依頼が終わったときに、書庫ごとの終わり方と出どころを並べた記録を 1 件足す。
- **取る**: ⑶。info `startup_install_done`（`origin`・書庫ごとの `(パス, 終わり方の語)`）。手続き（`procedure.rs`）に触らず、消す判断と同じ所で出せる。受けた（`install_order_queued`）と始めた（`install_begin`）には今のまま `origin` が載る。
- **引き換え**: 「入った」「失敗した」の 1 件ごとの記録（`install_done`・`install_failed`）には出どころが載らないまま。終わりのまとめ 1 件で読む。

### 決め: `callghost` とゴースト自身の切り替えの競り合い（議題 8）
- **候補**: ⑴ 終わった時点で予約が在る・今のゴーストが変わっていれば頼まない ⑵ 終わってから一拍待つ。
- **取る**: ⑴。依頼を出したときの今のゴーストのフォルダ名を控え（`ghost_at_submit`）、終わったときの今のゴーストと `same_folder_name` で比べる。上書きのための起こし直しは同じフォルダなので混ざらない。
- **引き換え**: 依頼の間に A → B → A と切り替わって戻ったときは見分けられない（切り替えが無かったと見て頼む）。切り替えの始まりを知らせる口（押す形）を作るには `ghost_switch.rs` に手を入れる必要があり、並走の約束と上限に触れるので取らない。コードに `shortcut:` の注で残す。

### 決め: 起動の値を World へ渡す道と要件 7.1 の記録の置き場（議題 9）
- **取る**: H1。`main.rs` に 2 行: ⑴ `--offline-dump` の早い戻りの後・起動の解決の前に `let startup_install = install::startup::StartupInstall::new(opts.install);`（相対パスを絶対に直し、exe の隣のフォルダを控える）⑵ `readonly::install(world)` の次に `install::startup::install(world, startup_install)`（World の資源へ移す）。
- 要件 7.1 の記録は、持ち物の `Drop` が出す: まだ依頼を出していない（`pending` が残る）持ち物が落ちたら、入れる物が在るときだけ warn `startup_install_not_submitted`（`reason=boot_stopped`）を 1 件。起動の解決が `Err` で戻ると `main` のローカルとして落ちる。起動の解決の後で止まる道（書けるかの確かめ・窓を開けない・最初のゴーストを起こせない）でも、`main` が戻るときに落ちて同じ記録が出る＝要件 7.2「黙って捨てない」をどの道でも満たす。`Err(scene)` の腕は変えない。
- 取らない形: 起動の解決の `Err` の腕にだけ記録の行を足す形（設計の下書きの 1 回目）。解決の後で止まる道で、入れなかったことが黙って捨てられる（設計の検査で見つけて直した）。

### 決め: 要件 2.7 の記録の置き場
- **取る**: `install::desk::discard_for_exit` の先頭から `startup::discard_for_exit(world)` を 1 行で呼ぶ。終了の後始末は利用者の終了と OS のセッションの終了の両方がここを通る。まだ取り出していない持ち物があれば、`pending` を取り出して warn `startup_install_not_submitted`（`reason=exit_before_steady`）を 1 件。取り出したので、後で World と一緒に落ちても `Drop` の記録は出ない（二重にならない）。
- `Drop` だけに任せない理由: OS のセッションの終了ではプロセスが落ちて `Drop` が走らないことがある。終了の後始末の道は明示の呼び出しで記録し、`Drop` は起動が止まった道の受け皿にする。

### 決め: 消すのは UI スレッドで
- `OrderDone` を捌く所（UI スレッド）で `std::fs::remove_file` を書庫の数だけ呼ぶ。書庫の数は起動の行に書いた数（数本）で、1 本の消去は小さい。背景のスレッドへ寄せる案 C は取らない（上）。

### 決め: 終了の間の `OrderDone`
- `app.run()` が戻った後は `drain` が回らないので、終了の間に終わった依頼の書庫は消さない（入っていても残る＝安全側）。手続き側の記録（`install_done`・`install_abandoned`）は今のまま残る。

### 決め: `--install` の重ね
- `--install a --install b` は `--install a b` と同じ（`--ghost` と同じく値を続けて足す・重ねの warn は出さない）。

## 危うさと手当て
- 書庫を消し違える（取り消せない）— 消す判断を純粋な関数 1 つに閉じ込め、終わり方の 5 つと「まだ扱っていない」を 1 つずつテストで固定する。消すのは `remove_file`（フォルダは消せない）で、並びに書いたパスと exe の隣の `initial_install.nar` だけ。
- `callghost` がゴーストや利用者の選択を上書きする — 終わった時点の予約と今のゴーストの比べで頼まない。A → B → A は見分けない（上の引き換え）。
- `main.rs`（981 行）の上限 — 2 行だけ（作る・据える）。`ghost_session.rs`・`ghost_switch.rs` は触らない。
- `parse.rs` の今のテストの書き換え — `--install` を「扱わない名前」の例にしていた 2 件は `--fmo-name` など別の扱わない名前へ、`callghost`・`deletesource` を `pending` と期待していた 1 件は `kiosk` だけへ直す。

## 参照
- SSP ヘルプ「起動オプション一覧」 https://ukatech.shillest.net/ukadoc-ssp/functions/option/ （要件の段で読み直し済み・この段では読み直していない）

## 設計の討議の決め（2026-10-29・開発者「承認」）
- 議題 1: 切り替えの見分けはフォルダ名の比べから、`accept_switch` での受付の数え（プッシュ）へ替えた。根拠: 切り替えを受け付ける時点は `ghost_switch.rs` の `accept_switch`（`SwitchInFlight` を立てる 1 か所・400 行目）で分かる。同じウェーブ C12 で `ghost_switch.rs` に触る兄弟は無い（roadmap の C12 の表の約束は `ghost_session.rs` だけ）。足すのは 2〜3 行（926 → 929 行ほど）。同じフォルダへの起こし直しは数えない。
- 議題 2: 終了の間に終わった依頼は消さず頼まず、warn `startup_followup_dropped`。要件 3.1 に終了の除きを足した。
- 議題 3: `callghost` は終わりの知らせの時点で頼む。後から来たゴースト自身の切り替えは `Busy` で退く（テストと説明書の 1 行）。
