# Implementation Plan

- 作業の組 G1〜G25 は design.md の「タスクの見込み」の単位。指摘の番号は research.md 4 節の 1〜104、件は brief の預かった件 1〜8（要件 5）。直し方とテストの正本は design.md の「指摘ごとの直し」の表。
- `install/desk.rs`・`install/worker.rs`・`update/` を分け合う組が多いので、どのタスクも順に回す（design.md「タスクの見込み」の順）。`(P)` は付けない。
- 範囲の外で触るのは design.md「Modified Files（範囲の外）」のファイルと、要件 7.13 の例外 (a)〜(c) だけ。例外 (a)〜(c) を除き、`Cargo.toml`・`Cargo.lock`・`crates/dola/`・`crates/wintf/`・`vendors/`・`main.rs`・`areka-session` の `body_systems*`・`log_history*`・`reload*`・`alert*`・`exit_wait*`・`lib.rs` には、どのタスクも触らない。
- 新しく足す記録の語・変える語は design.md「設計の段で決めた事項」1 の表のとおり。表に無い語を足さない。
- 消す・名を変えるときは、消す前に `git grep` で `.rs`・`doc/`・`tools/`・`.kiro/steering/` の字面を確かめる（design.md「消す・名を変えるときの照合」）。
- 先のタスクがファイルを触るとき、そのファイルの番号の札や経緯の語（9・15〜17・29・47・56 ほか）を一緒に直してよい。残りは担当のタスクが拾う。

- [x] 1. 本番の芯を直す（UI の待ち・背景のスレッドの倒れ・取得・上書き・ゴーストの道のテスト）

- [x] 1.1 入れた後の記録の待ちに上限を持たせる（G1・指摘 1・37・件 1）
  - 窓口の記録の段を `settle_record` に切り出し、`barrier_within(EXIT_WAIT_LIMIT)` で待つ。期限切れは `install_record_unsettled` の error を 1 件残して続ける（design.md「`settle_record`」）。
  - `mcp/get_property_tests.rs` の 2 か所の `barrier()` を `barrier_within(SPIN_WAIT)` にする（指摘 37）。
  - 門で止める偽の `PersistIo` を単独の書き手に差し、`settle_record` を別のスレッドで呼んで `join_thread_bounded` で待つテストが緑で、`install_record_unsettled` が 1 件。同じテストが今の `barrier()` では止まって赤になることを先に確かめてある。
  - _Dispatch: standard — 門で止める偽の書き手でテストを組む所が残る_
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 5.1, 6.2, 6.4_

- [x] 1.2 インストールの背景のスレッドが倒れても窓口が固まらないようにする（G4・指摘 5・69）
  - 依頼ごとに守り手（門と窓口への送出端）を置き、panic で `install_worker_panicked` の error → 門の `end` → `DeskAsk::WorkerPanicked` を送る（design.md「倒れの守り手」）。
  - 窓口は閉じた後の頼みを捨てる腕より前で `WorkerPanicked` を拾い、`busy`・`worker` を戻す。`install_worker_gone` の腕でも `worker = None` にする。上書きの段が `Extracting` のときの `resume_between` の呼び出しは 1.6 で足す。
  - `OrderDone` の使わない欄と `allow` を消し、欄の無い腕にする（指摘 69・`desk_tests.rs`・`worker_tests.rs` の呼び手も直す）。
  - 新しい `desk_worker_gone_tests.rs` で、守り手を持たせたスレッドの panic で `WorkerPanicked` が届いて門の名が空になること、窓口に流すと `is_busy` が偽で次の依頼で起こし直すことが緑。
  - _Dispatch: standard — 守り手と窓口の新しい腕を組む_
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 4.3, 6.1, 6.2, 6.3, 7.1_

- [x] 1.3 更新の背景のスレッドが倒れても窓口が戻るようにする（G4・指摘 23）
  - 依頼ごとに守り手を `ports` より先に作り、panic で `update_worker_panicked` の error → 門の `end` → `DeskAsk::WorkerPanicked` の順に行う。`OrderDone` は倒れない道でだけ送る（design.md「倒れの守り手」の `update`）。
  - 窓口は `WorkerPanicked` で `worker = None` にし、段を `Idle` に戻す。
  - 新しい `update/worker_panic_tests.rs` で、偽の `NewFetch` の `get` が panic すると `WorkerPanicked` が届き、門が書く段に残らず、段が `Idle` になることが緑。
  - _Dispatch: standard — 守り手の落ちる順を含めて組む_
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 6.1, 6.2, 6.3_

- [x] 1.4 URL からの取得を 1 本の長く生きるスレッドの待ち行列にし、終了の門に乗せる（G5・指摘 2・12）
  - `RawInstallRequest` をパスと URL の 2 つの腕にし、台本の受け口は URL を窓口へ送るだけにする（`StartFetch`・`fetch_for` を消す・読み取りだけの断りは受け口に残す）。
  - 窓口が待ち行列を持ち、取得のスレッド `install-fetch` へ一度に 1 件だけ渡す。取得のスレッドは URL ごとの守り手で、成功・失敗・panic のどの道でも門の `end` と `FetchDone` を送る。送れなければ起こし直し、それも失敗したら `install_fetch_failed` を残してその URL を捨てて次へ進む（design.md「取得の待ち行列」）。
  - 門 `install-fetch` を `install::register` で登記し、終了では積んだ残りを `discard_for_exit` が `install_fetch_abandoned` で 1 件ずつ残す。`gate.begin` が偽なら `install_fetch_closing` を残して抜ける。`MakeFetch` は作り口に替える。
  - `fetch_url.rs` の 1〜5 行の doc と 146 行のコメントを事実に直す。`doc/` の文書の直しは 2.1 でまとめて行う。
  - 新しい `fetch_queue_tests.rs` で次が緑: 取得の途中で `begin_close` すると `exit_wait_abandoned` に URL が 1 件。取得中に 2 本目を投げても作り口は同時に 1 つで、2 本とも順に依頼になる。終了で積んだ残りが 1 件ずつ記録される。取得の口が Err のときも panic のときも次の URL が取りに行かれる。偽の作り口だけを使い、外のホストへつながない。
  - _Dispatch: standard, effort high — 守り手・起こし直し・終了の始末の組み合わせが細かい_
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 4.3, 4.7, 6.2, 6.3, 6.4, 6.13, 7.1, 7.7, 7.8, 8.1_

- [x] 1.5 `switch_to` を「降ろした」と「起こす」の間で待てるように分ける（G7・指摘 13 の `areka-session` の側）
  - 触る前に `audit-emo2-boot-switch` の着地の有無を確かめ、着地していれば `main` を取り込んでから直す。触るのは `ghost_switch.rs` の `switch_to`（597〜708 行の当たり）と末尾の mod の行と、新しい `ghost_switch_between.rs`・`ghost_switch_between_tests.rs` だけ（`take_down`・`boot_into` の中身と向こうの行は変えない。ただし次の 1 か所だけは例外）。
  - `take_down` で `session.shutdown` が失敗したときも `info!(ghost_switch_down_ms, "ゴーストを降ろした")` が出て、1 回の失敗で ERROR 2 行と「降ろした」の INFO が並ぶ（`audit-emo2-boot-switch` のかけ直しの audit が見つけた件。向こうは直さない）。info を成功のときだけ出す。失敗の検査は、shutdown が Err のときにこの INFO が 0 件であることを数える 1 本。
  - 最初に、待ちの間の材料（`WindowsClosed`・`ShellCarry`）が借用を持たない値で NonSend に置けることを確かめる。置けなければ止めて設計へ戻す。
  - 起こす後半を `boot_target` に寄せ、`hold_between`・`resume_between` と NonSend `BetweenHold` を置く。`switch_to` は上書きの口を呼ぶ前に古い印を消し、呼んだ後に印があれば待って戻る。`resume_between` は待ちが無ければ warn を 1 件残す（design.md「`BetweenHold`」）。`SwitchStage` と `BodySystems::overwrite_between` の形は変えない。
  - `ghost_switch_between_tests.rs` の 4 本（待つ・続けて起こす・呼ばなければ待たない・起こせなければ既定へ戻す）が緑で、`ghost_switch_tests.rs` の既存のテストも緑。
  - _Dispatch: standard, effort high — 範囲の外のファイルの段を、形を変えずに分ける_
  - _Requirements: 3.1, 4.1, 4.2, 6.3, 6.4, 6.7, 6.13, 7.2, 7.6_

- [x] 1.6 起動中のゴーストへの展開を背景のスレッドへ出す（G7・指摘 13・62 の本体の側）
  - `overwrite::Stage` を `Held`・`Busy`・`Requested`・`Extracting`・`Resumed` の 5 つにし、`run_between` は `hold_between` を呼んで「降ろした」を返すだけにする。返信が送れなければ印を消してその場で起こす。
  - 背景のスレッドは「降ろした」を受けて `enter_write` → `install_into` → `leave_write` → `end` を行い、`DeskAsk::Extracted` を頼んで定常の返事を待ってから `Ran(result)` を返す。`install_overwrite_done` は背景のスレッドが残す（design.md「上書きの 2 往復」）。
  - 窓口は `Extracted` で `resume_between` を呼ぶ。段が合わなければ `install_overwrite_unexpected` を残して返事だけする。`WorkerPanicked` を受けたとき段が `Extracting` なら `resume_between` を呼ぶ（1.2 の腕に足す）。終了では `Extracting` が `"extracting"` を返し、`Resumed` は返信端を落とす。
  - `desk_overwrite_tests.rs` の doc と 582〜605 の表明（指摘 62）、`desk_tests.rs` 690 行のテストを新しい形に直す。
  - 新しい `desk_overwrite_between_tests.rs` で次が緑: 降ろした → 展開 → 起こす → 定常で返す。展開中に終了が始まる。背景が倒れても切替が続く。窓 0 の間に窓口の `drain` が回ることを 1 回確かめる。今の UI で展開する形では赤になる。
  - _Depends: 1.2, 1.5_
  - _Dispatch: standard, effort high — 2 往復の段と終了・倒れの道の組み合わせ_
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 4.3, 4.8, 6.1, 6.2, 6.3, 6.4, 6.7, 7.1, 7.7_

- [x] 1.7 MCP のテストを本番の道に乗せる（G15・指摘 36・102）
  - `get_property_tests.rs` のゴーストの道を `PropGetCueSink::new(reader, asker, tx)` で組み直し、`carrier_cue` の `NAMES` の合図を 1 つ流して `KanadeMsg::PropertyEvent` の references を読む。値の無いものの書き方を直す（design.md 指摘 102 の行）。`ShioriHostSink` を使う所が範囲の中に残らない。
  - `check_script_tests.rs` に、本物のシェルを装着させて `\s[99999]` に `missing_surface` が 1 件になる実 GPU のテストを足す（`GpuPermit` で順を取る）。
  - 2 つのテストが緑。`git grep ShioriHostSink` で範囲の中の読み手が 0 件。済んだことを `audit-areka-app` のセッションへ知らせる（向こうの脳の削除の順のため）。
  - _Dispatch: standard — 本番の受け口でテストを組み直す_
  - _Requirements: 3.1, 3.3, 4.1, 4.6, 4.8, 7.2, 7.12_

- [x] 2. 本番の小さな直しを入れる

- [x] 2.1 取得の小さな直しと文書の行を直す（G6・指摘 22・65〜68・12 の文書）
  - スキームを `eq_ignore_ascii_case` で比べる（`judge.rs`・`fetch_url.rs`・指摘 65）。消した数が 0 なら `install_fetch_swept` を出さない（66）。書き込みの失敗で書きかけを消し、消せなければ `debug!`（67）。
  - `install/worker.rs` に `remove_downloaded(order, dir)` を置き、`run_order` の後に `download_dir()` で呼ぶ。出どころが `Script` で `dir` の下の物だけを消す（68）。
  - `doc/COMPAT_ARCHITECTURE.md` 344 行と `doc/ukadoc-coverage/ledger/sakura-script.toml` 1398〜1418 行を、取得の仕組み（1.4）と書庫を消すこと（68）と記録の条件（66）に合わせる。COMPAT_ARCHITECTURE はこの 1 行だけを直す。
  - 次のテストが緑: `HTTPS://` の URL が受けられる（今は赤）・古いファイルが無いとき `install_fetch_swept` が 0 件（今は赤）・書き込みの失敗の後にファイルが残らない（今は赤）・`Script` の下の物は消え、`dir` の外と `Script` でない物は残る（今は赤）・`..\\..\\x:y.nar` の名前と置き場（22）。一時フォルダはどれも `TempPath`。
  - _Dispatch: standard — 書庫を消す関数と信頼の境界の確かめを足す_
  - _Requirements: 3.1, 4.1, 4.2, 4.5, 4.7, 6.2, 6.5, 7.4, 7.7, 7.8, 8.1_

- [x] 2.2 インストールの UI のスレッドで目録を読まないようにする（G8・指摘 6・60・61）
  - `areka-ghost` の `catalog.rs` に descript を 1 回読んで sakura の名前と `install.accept` を返す公開の関数を足し、`install_accept` を消してテストも寄せる（60・`catalog_tests.rs`）。
  - 窓口は素性の問い合わせに `dir` だけを答え（`GhostFacts` に `dir`）、背景のスレッドが新しい関数で読む。綴り直しと隠れの判定を `DeskPorts::record` へ移し、`DeskAsk::Record` で運ぶ（6）。
  - `copied_root` の項目の根を答えに使う（61）。
  - 次が緑: `catalog_tests` で 1 回の読みが 2 つを返し、読めない descript で warn が 1 件。`desk_record_tests` の 6 本が新しい形で緑。`destination_tests` の書ける根の印の場合が同じ答え。
  - _Dispatch: standard — 読む場所を背景へ移し、窓口との頼みの形を変える_
  - _Requirements: 3.1, 3.3, 4.1, 4.8, 6.4, 7.1, 7.6_

- [x] 2.3 手続きの終わり方を直し、抜けたテストを足す（G9・指摘 7・14・18・63・64）
  - 書いた後に締めが閉じたときは `Installed(items)` を返し、残りの書庫だけを止める（`archive_steps` が止めるかの旗を添える・doc を直す・14）。
  - `install_cue_tests.rs` で `queued_orders` で比べ、doc の 3 行と記録の数えを消す（18）。
  - `install_order_refused` が 1 件・`verdict` が `Empty`／`NoDesk`／`Closing` の表明にする（63）。
  - `put_ghost` の受信端を落としたときの `install_event_send_failed`（7・`desk_worker_gone_tests.rs`）と、置き場が空の窓口の `install_balloon_not_remembered`（64）のテストを足す。
  - `procedure_branch_tests.rs:549`・`procedure_skip_tests.rs:307` の期待が `Installed` で緑（今は `Abandoned` で赤）。足したテストも緑。
  - _Depends: 1.2_
  - _Dispatch: standard — 手続きの止め方に旗を足す新しい分岐がある_
  - _Requirements: 3.1, 4.1, 4.2, 4.8, 6.2, 7.7_

- [x] 2.4 更新の本番の小さな直しを入れる（G10・指摘 24・25・76・77）
  - `homeurl` を 2 か所で `trim` してから空を None にし、手続きの重ねた `filter` を消す（24）。
  - 背景の側の語を `update_reload_asked` に改め、`procedure_reload_tests.rs:88・175`・`worker_path_tests.rs:200-205・323` と台帳 `doc/ukadoc-coverage/ledger/shiori.toml:7188` を同じ変更で直す（25）。
  - 列の型の別名を `update/mod.rs` に置いて 7 か所で使い、`refs::Tail` を `Ref34` に改める（76）。なぞるだけのコメントと古い doc を直す（77・`desk_tests.rs:3-5` を含む）。
  - 新しい `homeurl_tests.rs` で照会の写しと窓口の写しの両方が `" "` を None・`"https://x/ "` を `"https://x/"` にして緑（今は赤）。2 つの語が別々に 1 件ずつの表明が緑。
  - _Dispatch: mechanical — 直し方と期待の値を design が与えている_
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 7.1, 7.7, 7.8_

- [x] 2.5 隠れたキャラクターの `dump_surface`・`dump_balloon` が答えるようにする（G12・指摘 3）
  - `areka-emo-present` の `EmoPresenter` に `compose_last_shown` を足し、`snapshot_tests.rs` に控えを捨てた後に `Some(Ok)` を返すテストを足す。
  - `dump_surface.rs` の `_` の腕を「一度も見せていない」と「控えが捨てられた」に分け、後者で `compose_last_shown` を呼ぶ。`dump_balloon.rs` も同じにする。
  - 実 GPU のテストで、隠した target に `InvalidateCache` を送り、控えの surface が残り絵だけが無いことを表明してから、`dump_surface`（surface の省略）と `dump_balloon` が `OK` で絵を返す（今は `NG:` で赤）。
  - _Dispatch: standard — 範囲の外の crate の公開の口と実 GPU のテストを組む_
  - _Requirements: 3.1, 4.1, 4.2, 4.6, 6.4, 7.6_

- [x] 2.6 表情の表を UI の外で読み、絵の大きさの確かめを 1 つにする（G12・指摘 28・30・78）
  - `get_expression_table.rs` で、シェルのフォルダを写して名前つきのスレッドで `load` と `render` を行う。起こせなければ `expression_table_spawn_failed` の error と `NG:`（`encode_elsewhere` と同じ形）。
  - `dump_balloon.rs` の大きさの確かめを `check_size` に寄せる（78）。`check_size` の (0,1,空)・(1,0,空)・(2,2,長さ 15) が Err、(1,1,長さ 4) が Ok のテストを足す（30）。
  - 実ユニットのテストを `wait_answer` で待つ形にし、`handle` を呼んだスレッドの記録に表の読みと解析の記録が 0 件であることを `capture` で見るテスト（読めないシェルのフォルダを差す。読むスレッドは `encode_elsewhere` と同じく dispatcher を持たせないので、記録は `capture` の窓に入らない。本番に JoinHandle を控えない）が緑（今は UI のスレッドに出て赤）。
  - _Dispatch: standard — 読む仕事をスレッドへ出し、呼んだスレッドの記録でテストを組む_
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 6.2, 6.4, 7.7_

- [x] 2.7 MCP の小さな本番の直しを入れる（G13・指摘 35・39・82・83・88）
  - `script_target` を `resolve.rs` へ移して 3 本で使い、写しを消す（35）。
  - `check_script_judge.rs` の別名の定数を消して `strict::MSG_*` を直に使い、`doc/ssp-mcp/areka-tools.md:6` を直す（39）。
  - `get_log` の余分な腕を消し、`active` を渡す（82・83）。
  - `reload` の断りの文に `install` を足し、`reload_tests.rs:135・205` を直す（88）。
  - 既存の `get_log_tests.rs:392` と、直した `reload_tests.rs` の表明が緑。
  - _Dispatch: mechanical — 移す先と文を design が与えている_
  - _Requirements: 3.1, 3.3, 4.8, 7.1, 7.7_

- [x] 2.8 置き場の変更で UI が根を見に行かないようにし、判断の関数に置き場の決まりを置く（G16・指摘 4・41〜43・90・104）
  - `roots/edit.rs` の `apply` の記録の段（`status`・`survey`・`log_roots`・`log_hidden`）を、dispatcher を持たせた短命のスレッド `roots-log` へ移す。起こせなければ `roots_log_spawn_failed` の error（4）。
  - 外すと `info!(roots_removed, path)` を残す（42）。コメントと見出しを事実に直し（41・43）、包みを消す（90）。
  - `decide_remove`・`decide_front` に「書ける根の印の項目は外せない」「印があるとき実行ファイルの隣は先頭へ動かせない」を置き、断りの腕を足す。`menu/roots_frame.rs` には触らない（104）。
  - テストの待ち方: 本番の NonSend に JoinHandle を控えない。`log_capture_kit::capture` の記録はスレッドの名を持たず窓を閉じるまで読めないので、このテストは `tracing::subscriber::with_default` で自前のスレッドの受け手（記録ごとに `std::thread::current().name()` を `Arc<Mutex<Vec<_>>>` に残す・前例は `areka-ghost` ほかのテスト）を据える。窓の中で `apply` を呼び、`roots-log` の記録が現れるまで `SPIN_WAIT` の上限で見直し（越えたら赤）、呼んだスレッドの記録が 0 件であることを見る。`install_global_capture_all` は使わない。
  - 次が緑: `apply` を呼んだスレッドの記録に `root_listed`・`roots_hidden_*` が 0 件（今は赤）。外すと `roots_removed` が 1 件で `path` が外した根。2 つの決まりの断りのテスト。今の `root_listed`・`roots_hidden_count` の表明は、上の待ち方で記録の段の記録を見る形へ移して緑。
  - _Dispatch: standard — 記録の段をスレッドへ出し、決まりの断りの腕を足す_
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 6.2, 6.4, 7.7_

- [x] 2.9 フォルダを選ぶ画面の出し方を 1 つにする（G16・指摘 40・89・92）
  - 選ぶ画面の共通の関数を `install/desk.rs` に 1 つ置き（旗・スレッド名・抑止・選ぶ口・送り方を受ける）、`roots/desk.rs` の写しを寄せる。抑止の記録は info・`reason = "no_alert"`（語は変えない）。
  - `PickedFolder` ほかの使わない列挙を消し（89）、`roots.rs` は 3 つの名前だけを出す（92）。
  - 両方の窓口のテストで、抑止の行が INFO で `reason = no_alert` になって緑（今のインストールの側は WARN で赤）。`pick_tests.rs` も緑。
  - _Dispatch: standard — 2 つの窓口の写しを引数で違いを受ける 1 関数へ寄せる_
  - _Requirements: 3.1, 3.3, 4.1, 4.2, 6.6, 7.1, 7.7_

- [x] 2.10 置き場の芯の小さな直しを入れる（G17・指摘 46〜48・95・96）
  - `CopyPlan::Copy` に書ける根の `dir` を載せ、`let … else` を消す（46）。`plan_copy` のテストが新しい欄を見る。
  - 番号を消し（47）、doc を事実に直す（48）。`kind_name` を `pub(super)` にして写しに使う（95）。
  - `decode_memory(r"\\?\C:\x", None)` が項目 1 つ・控え 0 のテストを足して緑（96）。`roots/` の既存のテストも緑。
  - _Dispatch: mechanical — 直し方と期待の値を design が与えている_
  - _Requirements: 3.1, 4.1, 6.2_

- [x] 2.11 説明書の本番の小さな直しを入れる（G19・指摘 49〜55・97）
  - 呼び手の説明を「`runtime_boot::install_lines`」に直す（49）。`set_path` で `missing_logged` を戻し、doc を直す（50）。`readme_cue` の文を直す（`event` は変えない・51・52）。
  - `os_port.rs` の doc と `// SAFETY:` の文を事実に直し（`// SAFETY:` は残す・53・55）、5 つの `allow` と説明を消す（54）。`fn win32_code(hr: i32) -> u32` を切り出す（97）。OS の既定のアプリで開く形は変えない。
  - 次が緑: 読み直しの後に別の無いファイルで `readme_missing` が 1 件（今は 0 で赤）。`0x80070002 → 2`・`0x80004005 → そのまま`。`cargo clippy -p areka-session --tests` で警告 0。
  - _Dispatch: mechanical — 文と小さな関数の形を design が与えている_
  - _Requirements: 3.1, 4.1, 4.2, 5.3, 6.12_

- [ ] 3. テストの待ち・写し・番号を直す

- [x] 3.1 `install/` のテストの期限の無い待ちを無くす（G2・指摘 19〜21・件 4）
  - `names_tests.rs:29` を `join_bounded("sylphya", parts.handle).expect(..)` にする（19・件 4）。
  - `install_cue_tests.rs`・`worker_tests.rs` の待ちを `join_thread_bounded`・`recv_timeout`・`wait_recv`・`join_bounded` にする（20・21）。
  - 3 つのファイルに上限の無い待ちと `let _ = …join()` が残らず、テストが全部緑。
  - _Dispatch: mechanical — 既にある待ちの道具へ置き換える_
  - _Requirements: 3.1, 4.3, 5.4, 6.3_

- [x] 3.2 `update/` の背景のスレッドのテストの待ちと手順を寄せる（G3・指摘 26・73）
  - `worker_tests.rs`・`desk_tests.rs`・`worker_path_tests.rs` の自前の待ちを既にある道具に置き換える（26）。`run_one` に `roots`・`exe_dir` を足して写しを消す（73）。
  - `update/worker_tests.rs` が 1,000 行の内に収まり（見込み約 940 行）、テストが全部緑。
  - _Dispatch: mechanical — 既にある道具と引数の足しで寄せる_
  - _Requirements: 3.1, 4.3, 6.1_

- [x] 3.3 `update/` のテストの写しを支えのファイルへ寄せる（G11・指摘 27・72・74・75・件 6 の `update/`）
  - 新しい `update/update_test_support.rs` に、名を取る `boot_context(temp, root, ghost, balloon)`・`write_descript`（charset の頭と末尾の改行の形に統一）・イベント名で絞る補助・`run`・`ids`・`refs_of` を置き、`update/` の 4 つの写しと補助の写しを寄せる（27・74。`desk_resolve_tests.rs` の呼び手は頭を外す）。
  - `refs_of` の写し 3 つ（`install/desk_overwrite_tests.rs`・`update/desk_reload_tests.rs`・`update/worker_path_tests.rs`）と `worker_path_tests.rs` の `kinds` は、`areka-session` の `shell_balloon_switch_session_lap_test_support::kinds`（pub）と中身が同じ。本体のテストから引けるならそれを呼び、引けなければ支えのファイルの 1 つへ寄せる。
  - `desk_reload_tests.rs` は `super::ask_reload_for_test` を呼ぶ（72）。`update_tests.rs` と mod の行を消す（75）。
  - `update/` の中の `boot_context` の写しが支えの 1 つだけになり、テストが全部緑。`desk_resolve_tests.rs` 558 行の `BODY_SYSTEMS` の 1 行は残す（件 7）。
  - _Dispatch: mechanical — 置く補助の形を design が与えている_
  - _Requirements: 3.1, 3.3, 3.4, 5.6, 5.7_

- [ ] 3.4 MCP のテストの写しと要らないテストを片づける（G14・指摘 29・32・34・38・79・80・84・86・87）
  - 新しい `mcp/kanade_fake_test_support.rs` に偽の kanade の足場（`ghost`・`rig`・`answer`・`warn_or_worse`・`text`・`record_one`）を置き、`get_status_tests.rs`・`raise_event_tests.rs`・`sakurascript_tests.rs` の写しを寄せる（38）。
  - 番号の札と `mN_`・「MN:」を消し、名と定数を中身で付け直す（29・34）。`TempPath` に替える（32）。`differing` を支えへ移し `read_picture` に寄せる（79）。
  - `mcp_tests.rs` の 1 本（80）、2 本と `strict_records`（84）、`ByHand`（86・`Table`・`ghost()` を `pub(super)` に）を消し、「1〜13 行目」に直す（87）。
  - `mcp/` のテストが全部緑で、消した名の読み手が `git grep` で 0 件。名を変えたテストを範囲の外の doc が名指していない。
  - _Dispatch: mechanical — 寄せ先と消す物を design が与えている_
  - _Requirements: 3.1, 3.3, 3.4, 4.5, 4.8, 6.1_

- [ ] 3.5 `roots/` のテストの写しを寄せる（G18・指摘 44・45・93・件 6 の `roots/`）
  - 新しい `roots/roots_test_support.rs` に `roots_of`・`events_named` と、既にある `boot_context` に `roots_origin: Default`・`app_profile_dir` を上書きした `world` を置き、`edit_tests.rs`・`edit_write_failure_tests.rs`・`desk_tests.rs` の 3 つの写しを寄せる（45・93）。名 `ga` の表明 3 か所は呼ぶ前の `current` との比べにする。
  - `add_takes_effect_and_logs_error_when_memory_unwritable` を消す（44）。
  - `roots/` の中の `boot_context` の写しが 0 になり、テストが全部緑。
  - _Dispatch: mechanical — 寄せ先の形を design が与えている_
  - _Requirements: 3.1, 3.3, 3.4, 5.6_

- [ ] 3.6 説明書のテストの写しと番号を片づける（G20・指摘 56〜58・98〜101・件 5・件 6 の説明書）
  - `readme_cue_tests.rs` の私有の `capture_logs` を含む 2 つの包みを消し、`capture_lines(..).1` に替える（57・件 5）。
  - `opener_submit_tests.rs` の写しを `crate::reload::test_support::boot_context(tmp.path(), &root)` にする（58・件 6）。
  - 表の各行に「送った件数 0」を足して 5 本を消し（98）、`opener_test_support.rs` に `dest_of(name, args)`・`dest_url(url)` と文脈の補助を置いて寄せ（99）、その `mod tests` を消す（100）。番号の部分を消し（56）、`destination_tests.rs` のテストの名を改める（101）。
  - 説明書のテストが全部緑で、範囲の中に `capture_logs` の写しが残らない。
  - _Dispatch: mechanical — 寄せ先と消す物を design が与えている_
  - _Requirements: 3.1, 3.3, 3.4, 5.5, 5.6_

- [ ] 3.7 `install/` のテストの写しと説明を片づける（G21・指摘 9〜11・15〜17・件 6 の `install/`）
  - `desk_tests.rs` ほか 8 か所の `boot_context` の写し（`desk_exit_tests.rs` の 2 つを含む）を消し、`areka_session::reload::test_support::boot_context(root.dir(), &root)` を呼ぶ（11）。`desk_tests.rs` 385・554 行の `BODY_SYSTEMS` の 1 行は残す（件 7）。
  - 括弧の番号と番号の行を消して平文に（9・15〜17）、変える前と比べた言い方を振る舞いの文にし、テストの名を `unmarked_list_answers_the_bundled_root` に改める（10）。
  - `install/` の中の `boot_context` の写しが 0 になり、テストが全部緑。
  - _Dispatch: mechanical — 呼び替え先と文の形を design が与えている_
  - _Requirements: 3.1, 3.3, 5.6, 5.7_

- [ ] 4. 呼び手の説明を今の事実に直す（G22・指摘 8・70・71・81・件 3）
  - `install/mod.rs`・`install/names.rs`・`update/mod.rs`・`mcp/mod.rs` の呼び手の説明を、`BODY_SYSTEMS` の `after_input`・`after_runtime_wired` から呼ばれるという事実の文に直す。
  - `areka-runtime/src/consumer_ledger.rs:77` の「本体の `readme::readme_cue::ReadmeCueSink`」を 1 語だけ直す（設計の討議の確定）。
  - 4 つのファイルに `ghost_session::register_systems` の字面が残らない。`mcp/mod.rs` を直したことを `audit-areka-app` のセッションへ知らせる。
  - _Dispatch: mechanical — 文の中身を design と要件が与えている_
  - _Requirements: 5.3, 7.6_

- [ ] 5. 相手の着地に依る寄せ・記録・仕上げの確かめ

- [ ] 5.1 `audit-areka-app` の着地に依る寄せを入れる（G23・指摘 31・85・94・103・件 6 の残り・例外 (b)(c)）
  - 着地の有無を確かめ、着地していれば `main` を取り込んでから次を行う。着地していなければ写しと古い形を残し、理由を research.md に記録し、向こうの brief へ件として書く（要件 7.2・7.9）。
  - 31: `wic_decode_rgba` が `pub(crate)` になっていれば、こちらの写しを消す。85: `log_history` のテストに中身ができていれば、`sakurascript_log_tests.rs` と据え付けを消す。件 6: `boot_context_named` があれば、`update_test_support.rs` の `boot_context` を消してそれを使う。
  - 103: `alerts_suppressed()` があれば、`raise` と起動の前の道がそれを呼び、`before_boot_alert_suppressed` と `startup_flow_tests.rs:264-267` を消す。選ぶ画面の 3 か所は寄せない。
  - 94: 向こうが `alert.rs:150` を欄を読まない形にしていれば、`ExeLocationUnavailable` の欄を消し、同じコミットで `alert_tests.rs:51-56` の `tried`・`origin` の 2 行を消す（例外 (b)）。
  - 例外 (c): 向こうが先に着地して脳の残りを残していれば、1.7 の後に `shiori_host.rs`・`shiori_host_case_fold_tests.rs` と本体の `Cargo.toml` の dev の 2 行（`shiori-abi`・`windows-core`）を消す。`Cargo.lock` の変化はこの 2 行の分だけ。
  - 寄せた件と残した件の一覧が research.md にあり、ビルドとテストが緑。
  - _Depends: 1.7, 3.3_
  - _Dispatch: standard — 相手の着地の状態ごとに寄せるか残すかを決めて直す_
  - _Requirements: 3.3, 5.6, 7.2, 7.3, 7.9, 7.12, 7.13_

- [ ] 5.2 `audit-emo2-boot-switch` の着地に依る寄せを入れる（G23・指摘 33・例外 (a)）
  - 着地の有無を確かめ、着地していれば `main` を取り込む。`spawn_windows(rig, dpi)`・`pump_messages` が `pub` になっていれば、`dump_surface_gpu_test_support.rs` の写しと定数を消して呼ぶ。doc の行を直す（33）。
  - `SwitchVerdict::Updating` があれば、`install/overwrite.rs:129` の腕に名が入っていてビルドが緑であることを確かめる（例外 (a)・向こうが足す）。
  - 着地していなければ写しを残し、理由を research.md に記録して向こうの brief へ件として書く。
  - 寄せたか残したかが research.md にあり、MCP の実 GPU のテストが緑。
  - _Depends: 1.6_
  - _Dispatch: mechanical — 寄せる先と確かめる腕を design が与えている_
  - _Requirements: 4.6, 7.2, 7.9, 7.13_

- [ ] 5.3 預かった件の片づけと申し送りを記録する（G24・件 2・7・8・指摘 59・91・`link_destinations`）
  - research.md に次を記録する: 件 1〜8 の片づけと出どころと申し送った相手（要件 5.9）。件 2 の `UNKEPT_TARGETS` へ足す物が無いこと。件 7（指摘 59）の 3 本の 1 行を残す理由。件 8 をそろえない決め。91 と `link_destinations` の印を裁定で残すこと。
  - brief へ件を書き足す: `audit-areka-app`（件 2・件 3・件 8 の決めと、12 の直しで「窓口が無い」の `url` の腕も取りに行かなくなる事実）、`audit-emo2-boot-switch`（`ghost_switch.rs` の切替の入口が `update_busy` を見ない件）、`link-context-copy`・`balloon-link-hover`（古いパス・`completed/` でないことを確かめてから）。C26 の 2 本のセッションへも知らせる。
  - research.md の記録と 4 つの brief の書き足しがコミットにある。
  - _Dispatch: mechanical — 書く中身を design と要件が与えている_
  - _Requirements: 2.5, 5.2, 5.7, 5.8, 5.9, 7.9, 7.10_

- [ ] 5.4 同じ作法で audit をかけ直し、新しい指摘を 0 件にする（G25・要件 9.1〜9.3・9.6〜9.8）
  - 同じ範囲（124 ファイルに、新しく置いた範囲の中のファイルを足したもの）へ `/ponytail:ponytail-audit` を、上限なし・全部読むの作法でかけ直す（要件 1.1〜1.3 と同じ作法）。
  - 出た指摘は直しが生んだものも含めて直し、0 件まで繰り返す。91・`link_destinations` の印と、5.1・5.2 で申し送りで残したものは数えない。新しい指摘が要件 2.2 の 3 種類に当たるときは直さずに議題にする。
  - 5 回目の打ち切りと、Fable による総括・連鎖のまとめは要件 9.6〜9.8 のとおりに扱う。
  - 回ごとの組ごとの件数と、0 件または打ち切りの判定が research.md にある。
  - _Dispatch: standard — 指摘ごとに直し方を決めて直す_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 2.1, 2.2, 2.3, 2.4, 2.6, 3.2, 3.5, 9.1, 9.2, 9.3, 9.6, 9.7, 9.8_

- [ ] 5.5 既定のブランチとの差分で約束を確かめ、全体テストを緑にする（G25・要件 9.4・9.5）
  - 既定のブランチとの差分で次を確かめて research.md に記録する: `Cargo.toml`・`Cargo.lock`・`crates/dola/`・`crates/wintf/`・`vendors/` の差分が 0（例外 (c) を除く）。範囲の外の差分が design.md「Modified Files（範囲の外）」と例外 (a)〜(c) の分だけ。`doc/COMPAT_ARCHITECTURE.md` の差分が 344 行だけ。`install/judge.rs` が名指す文書の名が変わっていない。外への約束の変わった所が design.md「設計の段で決めた事項」1 の表の分だけ。新しいメッセージボックス・コンソールの文字コードの切り替え・1,000 行の番人の表への足しが 0。
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回して緑（実機の確かめは全体テストの段の中だけ。赤が出たら 2 回目からは赤のテストだけを回す）。静かな机の計測・画面の操作・本物の置き場・外のホスト・開発者の MCP は回さない。
  - 差分の確かめの記録と、全体テストの緑が research.md にある。
  - _Dispatch: mechanical — 確かめる項目と手順が決まっている_
  - _Requirements: 4.4, 6.1, 6.6, 6.8, 6.9, 6.10, 6.11, 7.3, 7.4, 7.5, 7.11, 8.1, 8.2, 8.3, 8.4, 8.5, 9.4, 9.5_

## Implementation Notes

- 1.4: `StartFetch` を消すと範囲の外の `runtime_tests/consumer_ledger_agreement_tests.rs` が壊れるので、`InstallCueSink::with_readonly(tx, false)` へ置き換えた（最小の直し）。5.5 の範囲の外の差分の確かめでは、この 1 ファイルを design.md「Modified Files（範囲の外）」に加えて数える。
- 1.4: 取得のスレッドの panic は表に無い語を足さず、既にある `install_fetch_failed` に理由を添えて残す（design.md の Components の `install_fetch_panicked` は使わない）。
- 1.4 → 2.1: `doc/COMPAT_ARCHITECTURE.md:427` が消えた `fetch_for` を名指している。2.1 で 344 行と一緒にこの行も直し、5.5 の「COMPAT_ARCHITECTURE の差分は 344 行だけ」は 344・427 の 2 行として確かめる。
- 1.5 → 1.6: 本体の側に「印を消す」公開の口は無い（`clear_hold` は `pub(super)`）。`run_between` は返信を送れたのを確かめてから `hold_between` を呼ぶ順にする。印を消す代わりに `resume_between` を呼ぶと `ghost_switch_no_hold` の warn が誤って出る。`ghost_switch.rs` は 970 行で上限まで 30 行。
- 1.6 → 2.1: `doc/COMPAT_ARCHITECTURE.md:325` も「UI スレッドで展開する」と古い。2.1 で 344・427 と一緒に直し、5.5 の確かめは 325・344・427 の 3 行として数える。
- 1.6 → 3.1: `desk_overwrite_between_tests.rs` の別スレッドの `answer.recv()` と `worker_tests.rs` の `asks.recv()`・`desk.join()` に上限の無い待ちが残る。3.1 で一緒に上限のある待ちへ替える。
- 2.2: `install_accept` を消したので、範囲の外の `doc/ukadoc-coverage/ledger/assets.toml` の install.accept の「根拠の場所」を新しい読み手（`master_install_keys`・`DeskPorts::ghost_facts`）へ直した。5.5 の範囲の外の差分の確かめで数える。
- 2.3: design.md「設計の段で決めた事項」1 の `install_abandoned` の行（書いた後に締めが閉じたときは出ない）は、書き終えた書庫についてだけの決め。同じ依頼のまだ始めていない後の書庫は、黙って捨てず既にある `install_abandoned`（同じ欄）で 1 件残す（要件 6.2）。
- 3.3: `boot_context_named` が `main` から入っていたので、`update/` のテストは支えのファイルに `boot_context` を置かず直に呼ぶ。5.1 の件 6 の `update/` の分は済み。共通の `run` は `FakePorts` が手続きの中に閉じているので `update/procedure_test_support.rs` に置いた。
