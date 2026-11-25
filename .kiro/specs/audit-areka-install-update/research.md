# ギャップ分析: audit-areka-install-update（ponytail-audit・2026-11-22）

## 1. 要約

- **やり方**: `/ponytail:ponytail-audit` の作法から「最大 20 件」と「読まなかった所」を外してかけた（要件 1.2・1.3）。範囲は Boundary Context の In scope の 124 ファイルだけ（要件 1.1）。範囲を 6 つの区画（S1〜S6）に分け、読むだけの下請けに 1 区画ずつ全行を読ませた。この報告の書き手は、区画の報告を 1 本の通し番号へ並べ直し、「必ず直す」の 4 件と、直しが範囲の外へ及ぶ指摘を全部コードで確かめた（3 節・4 節の終わり）。テストは 1 本も回していない。
- **結果**: 指摘は **103 件**（必ず直す 4・直したほうがよい 56・できれば 43）。省いた件は無い。101 件は 6 つの区画の audit、2 件（102・103）は並んで走る `audit-areka-app` が自分の見直しの中で見つけてこの spec の範囲のファイルとして回してきたもので、この書き手がコードで確かめて足した（番号を動かさないため末尾に足した）。
  - 必ず直す 4 件は、どれも「UI スレッドを止める・終了で記録が残らない・答えが約束と食い違う」の形: 入れた後の記録が期限の無い `barrier()` で UI を止める（1・件 1）、URL からの取得のスレッドが終了の門に乗らず、捨てたことが記録に残らない（2）、装置が替わった後に隠れた target の `dump_surface`／`dump_balloon` が失敗する（3）、置き場の変更のたびに UI スレッドが全部の根を走査する（4）。
  - 芯はほかに 3 つ: ⑴ 背景のスレッド（`install`・`update`）が panic すると窓口の「仕事中」が戻らない（5・23）⑵ 期限の無い待ちのテストと、落ちる力の無いテスト（19〜21・26・37・84 ほか）⑶ テストの写しと、CLAUDE.md の `## Comments` に反する番号と古い説明（95 行・11 ファイルの番号ほか）。
- **預かった件 1〜8**: 全部まだ在る。audit の指摘と重なるのは件 1・3・4・5・6・7（5 節）。件 2 は直しで `update_cue`・`readme_cue` に `info!` を足す指摘が 0 件、件 8 は区画 S1・S2 が本体の側の振る舞い（`raw_sender`・`url` の腕）を確かめたが新しい指摘は無い。
- **議題**: 11 本（9 節）。前の版の議題 0（audit をどの段でかけるか）は、この回で audit をかけたので消した。新しく足したのは、完了した spec の決めを改める直し 2 本（議題 8・9）、外への約束（ログの語・MCP の答えの文・タグの受け方）を変える直しの一括の裁定（議題 10）、わざと置いた先送りの印（議題 11）。
- **数と行番号**: 範囲の数は要件の表と一致した（124 ファイル・本番 10,315 行・テスト 25,838 行）。brief と要件に書いた行番号は 1 つも動いていない。前の版の 4 節の数え違い（要件の番号の言及・`#[allow]`）は 4 節の「数え直した数」で直した。

## 2. 数え直し（要件 1.6）と行番号の当て直し（要件 1.7）

数えた HEAD は `32a86bd78`（親 `1b3ede541`＝要件の表の `main`）。audit をかけた HEAD は `125116389`（その後のコミットはこの報告だけ）。範囲の `.rs` を `wc -l` で数えた。テストは名前が `_tests.rs`・`_test_support.rs` で終わるもの。

| crate | ファイル | 本番 | テスト |
|---|---|---|---|
| 本体（`install/`・`update/`・`mcp/`・`roots/`・`roots.rs`・`session_tests/readme_body_tests.rs`） | 104 | 42 ファイル・8,417 行 | 62 ファイル・22,689 行 |
| `areka-session`（`roots/`・`readme/`・`readme.rs`・`readme_tests.rs`） | 20 | 9 ファイル・1,898 行 | 11 ファイル・3,149 行 |
| 合計 | 124 | 51 ファイル・10,315 行 | 73 ファイル・25,838 行 |

要件の Introduction の表と一致する。範囲に `.rs` でないファイルは無い。`crates/areka-session/src/roots.rs` は無い（`roots/mod.rs` が入口）。1,000 行に近いもの（上から）: `update/worker_tests.rs` 967・`install/startup_tests.rs` 854・`mcp/get_expression_table_tests.rs` 808・`install/desk_tests.rs` 788・`mcp/dump_surface_tests.rs` 721・`update/desk.rs` 706（本番でいちばん大きい）。越えるものは 0。`update/worker_tests.rs` と `install/startup_tests.rs` へテストを足す直し（23・26）は新しいファイルへ分ける（要件 6.1）。

| 件 | brief／要件の行 | 今の行（`125116389`） |
|---|---|---|
| 件 1 | `install/desk.rs` 381 の `barrier()`・`areka-sylphya/src/actor.rs` 562 の `barrier_within` | 381・562（動いていない） |
| 件 2 | `areka-session/src/log_history.rs` 131（`areka_session::readme`）・136（`UNKEPT_TARGETS`） | 131・136 |
| 件 3 | `install/mod.rs` 132・`update/mod.rs` 224・`mcp/mod.rs` 48 | 132・224・48 |
| 件 4 | `install/names_tests.rs` 29 | 29 |
| 件 5 | `areka-session/src/readme/readme_cue_tests.rs` 24 | 24 |
| 件 6 | `install/desk_tests.rs` 730・`readme/opener_submit_tests.rs` 31・寄せ先の候補 `reload_test_support.rs` 17 | 730・31・17 |
| 件 7 | `install/desk_tests.rs` 385・554・`update/desk_resolve_tests.rs` 558 | 385・554・558 |
| ほか | `install/judge.rs` 90（`doc/COMPAT_ARCHITECTURE.md` の名指し）・`areka-session/src/roots/user_root.rs` の `writable_ghost` の doc | 90・199〜200（完了のときに直した形のまま） |

## 3. 読んだ範囲（要件 1.3）

範囲の 124 ファイルは、どれも区画の下請けが全行を読んだ（各ファイル「全行」の申告つき）。読まなかった所は残していない。

- **S1**（`install/` の 14 ファイル）: `before_boot_ports.rs`・`before_boot_ports_tests.rs`・`desk.rs`・`desk_exit_tests.rs`・`desk_overwrite_tests.rs`・`desk_pick_tests.rs`・`desk_record_tests.rs`・`desk_tests.rs`・`destination.rs`・`destination_temporary_tests.rs`・`destination_tests.rs`・`judge.rs`・`judge_tests.rs`・`mod.rs`。
- **S2**（`install/` の 23 ファイル）: `fetch_url.rs`・`fetch_url_test_support.rs`・`fetch_url_tests.rs`・`install_cue.rs`・`install_cue_tests.rs`・`names.rs`・`names_tests.rs`・`overwrite.rs`・`pick.rs`・`pick_tests.rs`・`procedure.rs`・`procedure_branch_tests.rs`・`procedure_skip_tests.rs`・`procedure_test_support.rs`・`procedure_tests.rs`・`startup.rs`・`startup_flow_tests.rs`・`startup_tests.rs`・`submit_readonly_tests.rs`・`terms.rs`・`terms_tests.rs`・`worker.rs`・`worker_tests.rs`。
- **S3**（`update/` の 19 ファイル）: `desk.rs`・`desk_reload_tests.rs`・`desk_resolve_tests.rs`・`desk_tests.rs`・`mod.rs`・`procedure.rs`・`procedure_reload_tests.rs`・`procedure_test_support.rs`・`procedure_tests.rs`・`readonly_refusal.rs`・`readonly_refusal_tests.rs`・`refs.rs`・`refs_tests.rs`・`update_cue.rs`・`update_cue_tests.rs`・`update_tests.rs`・`worker.rs`・`worker_path_tests.rs`・`worker_tests.rs`。
- **S4**（`mcp/` の 17 ファイル）: `dump_surface.rs`・`dump_surface_tests.rs`・`dump_surface_image.rs`・`dump_surface_image_tests.rs`・`dump_surface_judge.rs`・`dump_surface_judge_tests.rs`・`dump_surface_gpu_test_support.rs`・`dump_surface_gpu_tests.rs`・`dump_balloon.rs`・`dump_balloon_tests.rs`・`dump_balloon_overlay.rs`・`dump_balloon_overlay_tests.rs`・`dump_balloon_gpu_tests.rs`・`get_expression_table.rs`・`get_expression_table_tests.rs`・`mod.rs`・`mcp_tests.rs`。
- **S5**（`mcp/` の 21 ファイル）: `get_active_ghost_list.rs`・`get_active_ghost_list_tests.rs`・`get_property.rs`・`get_property_tests.rs`・`get_status.rs`・`get_status_tests.rs`・`get_log.rs`・`get_log_tests.rs`・`reload.rs`・`reload_tests.rs`・`check_script.rs`・`check_script_tests.rs`・`check_script_judge.rs`・`check_script_judge_tests.rs`・`resolve.rs`・`resolve_tests.rs`・`raise_event.rs`・`raise_event_tests.rs`・`sakurascript.rs`・`sakurascript_tests.rs`・`sakurascript_log_tests.rs`。
- **S6**（根の一覧・説明書の 30 ファイル）: 本体の `roots.rs`・`roots/desk.rs`・`desk_tests.rs`・`edit.rs`・`edit_tests.rs`・`edit_temporary_tests.rs`・`edit_write_failure_tests.rs`・`pick.rs`・`pick_tests.rs`・`session_tests/readme_body_tests.rs`、`areka-session` の `roots/mod.rs`・`resolve.rs`・`resolve_tests.rs`・`report.rs`・`report_tests.rs`・`user_root.rs`・`user_root_tests.rs`・`readme.rs`・`readme_tests.rs`・`readme/destination.rs`・`destination_tests.rs`・`opener.rs`・`opener_tests.rs`・`opener_execute_tests.rs`・`opener_submit_tests.rs`・`opener_test_support.rs`・`os_port.rs`・`os_port_tests.rs`・`readme_cue.rs`・`readme_cue_tests.rs`（`readme_script_e2e_tests.rs` は範囲の外なので外した）。
- **範囲の外は読むだけ**（指摘を確かめるため。指摘には数えない）:
  - 下請け: `areka-session` の `user_root.rs`・`ghost_switch.rs`・`ghost_session.rs`・`exit_wait.rs`・`log_history.rs`・`reload.rs`・`runtime_boot.rs`・`alert.rs`・`reload_test_support.rs`・`write_failure.rs`、`areka-actor` の `spawn.rs`、`areka-ghost` の `catalog.rs`・`catalog_roots.rs`、`areka-kanade` の `actor.rs`・`sstp.rs`・`actor_sstp.rs`・応答の写し、`areka-sylphya` の `actor.rs`・`reader.rs`・`write_failure.rs`・`persist/withhold.rs`、`areka-update` の `work.rs`・`lib.rs`・`winhttp.rs`、`areka-runtime` の `spine_wait.rs`、`areka-mcp` の `tools/get_log.rs`・`bridge.rs` の該当部分、`areka-emo-present` の `presenter/refresh.rs`・`presenter/read.rs`・`presenter/snapshot.rs`、本体の `main.rs` の結線・`menu/roots_frame.rs`・`menu/mod.rs`・`cli/help.rs`、`temp-path-kit`・`log-capture-kit`、human-panic。
  - この書き手: `areka-emo-present` の `presenter/refresh.rs`（180〜210）・`presenter/snapshot.rs`（全体）・`presenter_test_support.rs`（`invalidate_ok`）、`areka-session` の `exit_wait.rs`（`register_gate`・`begin_close`）・`reload_test_support.rs`（全体）・`body_systems.rs`（60〜75）、`ghost_switch.rs`・`shell_balloon_switch.rs`・`reload.rs` の `update_busy` の当たり、`alert.rs`・`alert_tests.rs` の `ExeLocationUnavailable` の当たり、`areka-ghost` の `catalog.rs`（295〜360）、`log-capture-kit` の `capture.rs`（`capture` の doc）、本体の `Cargo.toml`（依存の行）・`png_image.rs` の置き場・`session_tests/shell_balloon_switch_session_update_tests.rs`（125〜130・236〜262）、`doc/COMPAT_ARCHITECTURE.md` 344 行・`doc/ukadoc-coverage/ledger/sakura-script.toml` 1398〜1418 行・`shiori.toml` 7188 行、完了した spec の決め（`areka-P0-ghost-install` の design・`areka-P0-network-update` の requirements と design・`areka-P0-write-failure-notice` の requirements・`areka-P0-baseware-root-list` の design・`areka-P0-mcp-reload` の requirements）、roadmap の C26 の行、`audit-areka-app`・`audit-emo2-boot-switch`・`link-context-copy`・`balloon-link-hover` の brief。

## 4. audit の結果

**What this repo does**: この範囲は、ゴースト・シェル・バルーン・追加ファイルの `.nar` を入れる手続き（台本の `\![execute,install,path|url]`・ファイルの投げ込み・起動の引数から、UI スレッドの窓口と背景のスレッド `install` が kanade とイベントを往復して書き込む）、ネットワーク更新（窓口と背景のスレッド `update` が `areka-update` のエンジンを対象ごとに回し、変わったら同じゴーストへの切替で読み直す）、MCP のツールの受け口（UI スレッドの Input の段で要求を汲み、重い仕事は別のスレッドへ出して答える）、ベースウェアの根（置き場）の並びとその足し外し、ゴーストの説明書を OS の既定のアプリで開くことを受け持つ。**想定した負荷**: 利用者 1 人のデスクトップ・UI スレッド 1 本と背景のスレッドが数本・MCP の相手は AI の利用者 1 人（呼び出しはほぼ順番・答えの待ちは 10 秒）・更新は 1 度に 1 件。書庫・取ってきたデータ・ゴーストの台本・利用者の置き場のファイルは外から来る入力として扱った。この負荷で壊れるのは、UI スレッドで遅い I/O を待つ所（1・4・6・28）と、ゴーストが URL の取得を出し続けたときのスレッドとメモリ（12）。

番号は 1 本の通し番号。並びは組（必ず直す → 直したほうがよい → できれば）の順、同じ組の中は区画の順（S1 → S6）。かっこの中は区画の報告の番号（追いかけるため）。`audit-areka-app` から回ってきた 2 件は組に関わらず末尾（102・103・どちらも「直したほうがよい」）に置き、かっこに「app」と書いた。各指摘は「場所｜何が問題か（具体例）｜直し」。末尾の［ ］は重なる預かりの件・議題・ほかの指摘の番号。

### 必ず直す（4 件）

1. **（S1-1）** `install/desk.rs:380-388`｜入れた後の記録 `record_installed` は、窓口の `drain`（UI スレッド）の中で `runtime.sylphya_publisher().barrier()` を期限なしで待つ。今のゴーストの記憶の書き手が遅い I/O で詰まると（ウイルス対策の走査中・ネットワークドライブの上のプロファイル）、書き手が戻るまで UI スレッドが止まり、窓もメニューも固まる。止まった書き手は Err で返るが、遅いだけの書き手は返らない｜`barrier_within(期限)` に替える。期限切れは `barrier_within` 自身が `error!` を出すので、呼び手は今の `install_record_unsettled` の `error!` を残してそのまま答える。期限の値と決定論の試し方は R2。［件 1・37 と同じ種類（テストの側）］（この書き手が確かめた）
2. **（S2-1）** `install/fetch_url.rs:144-154`（1〜5 行の doc と 146 行の `// 手放す＝待たない。` を含む）｜取得のスレッド `install-fetch` を起こしたまま手放している。終了の門（`WorkGate`）に乗らず、join もされない。例: 256 MB の書庫を落としている途中で終了すると、`exit_wait_abandoned` に URL が載らず、スレッドはプロセスの終わりで書きかけのまま消える。「URL からのインストールの依頼を捨てた」という記録が 1 行も残らない（黙った失敗）｜取得にも門を持たせる。`install::register` で `WorkGate` をもう 1 つ `exit_wait::register_gate(world, "install-fetch", gate, |_| {})` で登記し、取得の口に渡す。取得の頭で `gate.begin(url)`（偽なら warn を残して取りに行かない）、どの道で抜けても `gate.end()`。**書く段（`enter_write`）には入らない**＝終了は待たずに `exit_wait_abandoned` に URL を 1 件残す（完了 `areka-P0-network-update` の要件 7.5「取得の途中の物を待たずに終え」のまま。`begin_close` は書く段でない門の名を warn に残して待たない＝この書き手が `exit_wait.rs:124-152` で確かめた）。1 つの門が持つ名は 1 つなので、取得が同時に 1 本である形（12 の直し）と組む。doc の「門は持たない」と 146 行のコメントは「終了で待たない・やめた URL を記録に残す」に直す。テストを 1 本足す: 取得の途中で `begin_close` すると `exit_wait_abandoned` に URL が 1 件載る。［12・R4（済み）］
3. **（S4-1）** `mcp/dump_surface.rs:280-284`・`mcp/dump_balloon.rs:56`｜装置が替わると（ドライバの更新・TDR・スリープからの復帰）、`areka-emo-present/src/presenter/refresh.rs:187-204` の `refresh_after_device_change` が全部の target の合成メモを捨て、描き直すのは見えている target だけ。隠しているキャラクター（`\s[-1]`）や、話し終えて隠れたバルーン（ふつうの状態）は `last_shown` が `Some((id, None))` になる（`snapshot.rs:44-45` はこれを想定内の状態と書く）。この状態で `dump_surface`（surface の省略）や `dump_balloon` を呼ぶと `NG:the last shown picture is no longer kept` が返り、想定外の失敗として `error!` が 1 件出る。答えの本文が約束する「kept even if the balloon is hidden now」とも食い違う。テストも無い｜`areka-emo-present` の `EmoPresenter` に、`compose_alone` と同じ形で `compose_last_shown(target)` を 1 つ足す（控えてある `last_show` の (surface_id, binds, pattern) を新しい `Composer` で合成し直す）。2 か所の `Some((_, None))` の腕でこれを呼び、成功なら今の道（`picture`／`balloon_job`）へ渡す。範囲の中だけで済ませる代わりの形（`compose_alone(target, id)` を呼ぶ）は、`last_show` の着せ替えは使うがアニメーションの段を既定に戻すので、最後に見せた絵と一致しない＝採らない。`areka-emo-present` の見直し（`audit-emo-present`）は完了しており、C26 のほかの 2 本の持ち物でもないので、この spec が直す（要件 7.6・依存は変わらない）。テストは R7。（この書き手が確かめた）
4. **（S6-1）** `roots/edit.rs:293-312`（`apply` の後半）｜置き場の追加・外す・先頭へ動かすをメニューで選ぶたびに、UI スレッドが並びの全部の根へ `r.dir().is_dir()` と `areka_ghost::catalog_roots::survey(&roots)`（全部の根のゴーストとバルーンを走査し descript を読む）を行う。例: ネットワークのフォルダ（`\\nas\ukagaka`）を置き場に足した後で NAS を落とすと、次に置き場を変えたとき、SMB の時間切れ（数十秒）まで UI スレッドが止まりマスコットが固まる。結果は記録（`root_listed` の `found`・`roots_hidden_*`）にしか使わない｜記録の部分（`status` を組む所から `log_hidden` まで）だけを、`roots.clone()` を持たせた短命のスレッドへ移す。起動の文脈の書き換えと記憶へ書く所は今のまま。スレッドは記録だけを持つ（終了で失うのは記録の行だけ）＝終了の扱いは R8。（この書き手が確かめた。呼び手は `menu/roots_frame.rs:76・118` ほか＝UI スレッド）

### 直したほうがよい（56 件＝下の 54 件と、末尾の 102・103）

**A. 背景のスレッドの倒れ方と UI スレッドの重い仕事（インストール）**

5. **（S1-2）** `install/desk.rs:297-299・548-576`（直す場所は `install/worker.rs:71-86`）｜`busy` を下ろすのは `DeskAsk::OrderDone` だけ。背景のスレッド `install` が `run_order` の途中で panic すると（profile は `panic = 'unwind'`・human-panic のフックは止めない）OrderDone は来ず、⑴ 次の依頼は待ち行列に積まれるだけ（記録は `install_order_queued` の info だけ）⑵ `is_busy` が真のままで読み直しが断られ続ける ⑶ 門の名が残り、終了が上限まで待たされる。`install_worker_gone` の腕は「busy が偽でスレッドが死んでいる」ときしか通らず、通っても `worker` に死んだ送出端が残る｜`spawn_worker` の閉包に drop の番人を置き、巻き戻しで落ちたら `gate.end()` と窓口への「スレッドが居なくなった」の頼みを送る。窓口はそれを受けたら `error!` を 1 件残し、`busy = false`・`worker = None`（次の依頼で起こし直す）。`install_worker_gone` の腕でも `worker = None`。desk_tests に 1 本足す。［23 と同じ形］
6. **（S1-3）** `install/desk.rs:361-369・409-430・435-449`｜入れた後の記録で UI スレッドが根の目録を丸ごと読み直す。`ghost` では `catalog_spelling` が `find_ghost` を 2 回呼ぶ（`ghost_name` に 1 回・`folder` に 1 回）。`find_ghost` は `list_ghosts` 経由で全部の根の全ゴーストの descript.txt を読む。`balloon` では `find_balloon` の後に `note_hidden_by_ghost` の `balloons_for` がもう一度 `list_balloons` を読む。ゴーストが数十体あると記録 1 件で数十〜百のファイル読みが UI スレッドに乗る｜背景のスレッドの `DeskPorts::record`（`self.roots` を持つ）で `catalog_spelling` と隠れの判定を済ませ、綴り直した値を `DeskAsk::Record` で運ぶ。窓口は `remember_balloon`・`place_last_installed`・記録の出力だけにする。`ghost` の目録の引き当ては 1 回にまとめる。［1 と同じ関数］
7. **（S1-4）** `install/desk.rs:533-545`｜kanade へ送れないとき（kanade が止まった）、返信端が落ちて背景のスレッドは「閉じた」扱いで書庫を途中でやめる。この分かれ目を固定するテストが無い（`install_event_send_failed` を見るテストは 0 本）｜desk_tests に 1 本足す。`put_ghost` の受信端を落としてから `ask_raise` → `drain` し、答えの受信端が Err になること、`warn!(install_event_send_failed)` が 1 件出ることを確かめる。
8. **（S1-5）** `install/mod.rs:131-133`｜doc は呼び手を `ghost_session::register_systems` と書くが、実際の道は `register_systems` → `BODY_SYSTEMS.after_input`（`register_body_after_input`）→ `install::register`｜「`BODY_SYSTEMS` の `after_input` から」に直す（1 行）。［件 3・71・81 と同じ直し・70・49 は同じ種類］
9. **（S1-6）** `install/desk_record_tests.rs:175`｜表明の文に「shell-balloon-switch 要件 1.11」がある（CLAUDE.md の `## Comments` に反する）｜括弧を消して「バルーンの lastinstalled の控えは目録の綴り」だけにする。［数え直した数（下）］
10. **（S1-7）** `install/destination.rs:30・114・139`・`destination_tests.rs:97・342・345・347`（テストの名 `unmarked_list_answers_the_same_root_as_today`）・`352`・`desk_record_tests.rs:244・271・276`・`desk_overwrite_tests.rs:11・608・650-651`、ほかに区画 S1 が範囲の外の気づきとして挙げた `install/overwrite.rs:7`（「切替の道筋は今日のまま進む」）｜「今の失敗の経路」「`areka-nar` の今の `error!`」「今の答えのまま」「今どおり」「今と同じ根」「今日の切替の道」「今日の引数のまま」「今日の `switch_fatal`」など、変える前と比べた言い方が doc・テストの文・テストの名にある。次に変わった時点で何を指すか分からなくなる｜振る舞いそのものを書く（例「呼び手は error `install_no_root` を残し、書く前の I/O の失敗として返す」「先頭の根」「同梱のゴーストの根を答える」「既定へ戻す・既定なら致命の経路」）。テストの名は `unmarked_list_answers_the_bundled_root` などにする。
11. **（S1-8）** `install/desk_tests.rs:730-754`｜`boot_context` は `areka_session::reload::test_support::boot_context` の写し。違いはバルーン `B` とプロファイルの置き場だけで、呼ぶ 7 か所はどれも `"A"`｜写しを消し、既にある補助を呼ぶ（違いが結果に効かないかは R3）。［件 6・議題 4・27・45・58］

**B. URL からの取得（`fetch_url`）**

12. **（S2-2）** `install/fetch_url.rs:144`・`install/install_cue.rs:154`｜`url` の腕は受けるたびに新しいスレッドを起こし、本数に上限が無い。1 本ごとに本文を丸ごと（上限 256 MB）メモリに持つ。例: ゴーストが毎秒のイベントで `\![execute,install,url,…]` を返し続けると、遅い回線では取得が終わる前にスレッドが毎秒 1 本ずつ増え、数十本 × 数百 MB でメモリが尽きる｜取得を長く生きるスレッド 1 本にまとめ、URL を `mpsc` で積んで順に落とす（2 の門はそのスレッドが持つ・終了が始まったら積んだ残りの URL も `exit_wait_abandoned` と同じ扱いで記録に残す）。断る形（取得中の 2 本目を `warn!(install_fetch_busy)` で断る）は、ゴーストから見た振る舞い（2 本目のインストールが起きない）が変わるので採らない（9 節「議題にしなかった候補」）。テストを 1 本足す: 取得中に 2 本目を投げても取得口は同時に 1 つしか作られず、2 本とも順に依頼になる。［2・議題 10 の (g)］
13. **（S2-3）** `install/overwrite.rs:173-179`（4〜7 行の doc）｜起動中のゴーストへ入れるときの展開（`install_into`）を UI スレッドで同期に走らせている。その間、背景のスレッド `install` は返事待ちで何もしていない。例: シェルを何着も持つ 100 MB 超のゴーストを `ghost`／`supplement` で上書きすると、書き終えるまで UI スレッドのメッセージの汲み上げが止まり、OS のシャットダウンの問い合わせやタイマーに答えられない｜展開を背景のスレッドへ戻す。`run_between` は「降ろし終えた」を返信端で背景へ知らせるだけにし、背景のスレッドが `install_into` を走らせて結果を返す。切替は結果が来てから起こす。完了 `areka-P0-ghost-install` の設計の決め 3（「間を空けない・UI スレッドで同期に展開」）を改める直しで、切替の入口 `areka-session/src/ghost_switch.rs:606` の `switch_to`（`audit-emo2-boot-switch` の持ち物）の進め方に及ぶ。［議題 8・議題 3・R5］
14. **（S2-4）** `install/procedure.rs:270-286`（105〜112 行の `ArchiveEnd` の doc・158〜166 行の `install_abandoned`）｜書き終えて記録も済んだ後に、締めの知らせが `Closed` で返ることがある（終了が始まった、または worker の `RaiseOutcome::Failed`）。そのとき書庫の終わり方は `Abandoned` になり、warn が「書庫の手続きを途中でやめました」と出る。例: `OnInstallCompleteEx` の往復中に終了すると、ゴーストは入っているのに記録は「やめた」。`Abandoned` の doc「締めの知らせ 0 件」とも食い違い、`Failed` のときの理由の文「終了が始まった・ゴーストが居ない」も事実と違う｜書いた後の `Closed` では `Installed(items)` を返し、依頼の残りの書庫だけを止める（`archive_steps` が「止めるか」の旗を添えて返す）。doc を直し、`procedure_branch_tests.rs:549`・`procedure_skip_tests.rs:307` の期待を合わせる。もう 1 つの形（`install_abandoned` に `installed = true` を載せて文を分ける）より事実に近い。［議題 10 の (a)］
15. **（S2-5）** `install/procedure_tests.rs:79・97・133・155・399・436・442・446・473`｜テストの失敗の文に要件や設計の番号（「（2.2・2.6）」「（設計で決めたこと 14）」「（12.8）」ほか）が 9 か所｜括弧の番号だけを消す（文の本文は残す）。［数え直した数］
16. **（S2-6）** `install/procedure_branch_tests.rs:141・150・177・197・212・245・319・321・355・372・419・436・478`｜同じく 13 か所（「（3.2）」「（4.6・4.7・4.8）」「（5.4）」ほか。212 行は文がまるごと `"4.5"`）｜番号を消す。212 行は「受諾は Reference なしで送る」のような平文にする。［数え直した数］
17. **（S2-7）** `install/terms_tests.rs:62・78・85・129・145・187`｜同じく 6 か所（85 行は `"4.2"`、129・145 行は `"4.9"` と文がまるごと番号）｜番号を消す。85 行は「terms.txt が先」、129・145 行は「上限を越えたら切る」のような平文にする。［数え直した数］
18. **（S2-8）** `install/install_cue_tests.rs:100-105・118-124`｜doc の「待ち行列の中身は `install` の外から読めない」は誤り。このテストは `crate::install` の中にあり、`install::queued_orders`（`mod.rs:123-130`・`pub(crate)`・`cfg(test)`）を読める（`submit_readonly_tests.rs` と `file_drop_wiring_tests.rs` が使っている）。今は記録の数え（出どころと本数）で代わりに判定しているので、運ぶパスが化けても緑のまま｜`assert_eq!(install::queued_orders(&world), vec![InstallOrder { archives: vec![PathBuf::from(ABSOLUTE)], origin: InstallOrigin::Script }])` に置き換え、doc の 3 行と記録の数えを消す。（この書き手が `queued_orders` を確かめた）
19. **（S2-9）** `install/names_tests.rs:29`｜`let _ = parts.handle.join();` が記憶の書き手（sylphya）のスレッドの panic を捨てる。待つ期限も無い｜`areka_runtime::spine::join_bounded("sylphya", parts.handle).expect("書き手が panic しない")` にする（前例 `runtime_tests/frame/switch_finish_tests.rs:300`）。`audit-areka-app` も自分の見直しの中で同じ所を見つけて回してきた（同じ件・数は 1 つ）。［件 4・20・21・26・37 は同じ種類］
20. **（S2-10）** `install/install_cue_tests.rs:248・289`｜取得のスレッドを期限の無い `handle.join()` で待つ。289 行は依頼を受け取る前に join するので、スレッドが止まるとテストは赤にならず永久に止まる（同じ範囲の `fetch_url_tests.rs:187` は `join_thread_bounded` を使う）｜2 か所とも `join_thread_bounded("install-fetch の終わり", None, handle)` にする。［件 4 と同じ種類・19］
21. **（S2-11）** `install/worker_tests.rs:86・134・141・339・367・388・405・409・428・442`｜期限の無い `recv()`／`join()` が並ぶ。例: 背景のスレッドが頼みを送らずに止まる退行が入ると、339 行の `asks.recv()` が永久に待ち、全体テストが止まる｜`recv()` は `recv_timeout(上限)` か `areka_runtime::spine::wait_recv`、`ActorHandle` の join は `join_bounded`、`thread::JoinHandle` の join は `join_thread_bounded` にする。［件 4 と同じ種類・19］
22. **（S2-12）** `install/fetch_url_tests.rs:111-128`（対象は `fetch_url.rs:72-91` の名前の掃除）｜URL の末尾からファイル名を作る掃除は信用の境（ゴーストの台本の URL → ディスクのファイル名）にあるのに、テストは空白と空の末尾しか試していない。掃除を崩して `\` を通しても（`matches!(c, '.' | '_' | '-' | '\\')`）どのテストも落ちず、`https://x/..\..\evil.nar` が一時フォルダの外へ書く｜1 件足す: `https://example.com/a/..\\..\\x:y.nar` を落とすと、`path.parent() == Some(dir)` で、名前の末尾が `.._.._x_y.nar` になる。

**C. ネットワーク更新（`update/`）**

23. **（S3-1）** `update/worker.rs:76-95`（`spawn_worker` の閉包）・`update/desk.rs:417-425`（`WorkerGone`）｜背景のスレッド `update` が手続きの途中で panic すると（`areka_update::run` や `copy_for_write` の中の不具合）、スレッドは黙って終わり `OrderDone` が届かない。panic を拾う所は無い（`run_inbox`・`spawn_actor`・human-panic 2.0.8 のどれも拾わない・`panic = 'unwind'`）。段は `Running`／`AwaitingExec` のまま戻らず、`WorkerGone` の起こし直しは本番で通らない（通るのは死んだ受信端を差した `desk_tests.rs:416` だけ）。`is_busy` が真のまま残り、プロセスの終わりまでシェル・バルーンの切替（`shell_balloon_switch.rs:132`）と `\![reload]`（`reload.rs:311`）が断られ続ける。エンジンの中で倒れると門の `writing` が立ったままで、終了は毎回 3 秒待ち切る。`error!` は 1 件も残らない｜閉包の先頭で、`ports` より先に「落ちたら終わったを頼む」守り手を作る（`ports` が先に落ちて通信中の数が先に戻る）。守り手の Drop は、`std::thread::panicking()` なら `error!(update_worker_panicked)` と `gate.end()` をしてから `ask_desk(OrderDone)`。92 行の `ask_desk` はこの守り手に置き換える。テストは取得口の `get` が panic する偽物で「`OrderDone` が届く・門が書く段に残らない」を確かめる（置き場は新しいファイル）。［5 と同じ形・R10］
24. **（S3-2）** `update/worker.rs:183`・`update/procedure.rs:231-234`・`update/desk.rs:519`｜SHIORI の `homeurl` を空と見なす決まりが 3 か所で違う。照会の口と窓口の写しは空文字だけを空とし、手続きは空白だけでも空とするが、先へ渡すのは trim する前の値（kanade の `outcome_of` は値に手を加えない）。例 1: SHIORI が " " を返し descript に homeurl が無いと、写しは `Some(" ")` でメニューの「更新」は選べるのに、手続きは `no_homeurl` で飛ばし総括も送らない（押しても何も起きない）。例 2: `"https://example/ghost/ "` を返すと、エンジンが `https://example/ghost/ /updates2.dau` を取りに行って失敗する｜`worker.rs:183` と `desk.rs:519` で `value.trim()` してから空を None にし、`procedure.rs:234` の `.filter(|url| !url.trim().is_empty())` は消す。照会の写しと窓口の写しに 1 例ずつテストを足す（新しいファイル）。
25. **（S3-3）** `update/procedure.rs:186-190`・`update/desk.rs:205-217`｜同じ語 `event = "update_reload_requested"` が別々の 2 つの事実を記録する。背景のスレッドは「窓口へ読み直しを頼んだ」（info・verdict なし）、窓口は「切替の入口へ頼んだ結果」（Accepted なら info・ほかは warn・verdict つき）。1 回の読み直しで同じ語の行が 2 つ並び、片方が warn「受け付けられなかった」でも語では見分けられない（`worker_path_tests.rs:200-206` はこれを避けるため出所で絞っている）｜手続きの側の語を `update_reload_asked` に改める。合わせて直す所: `procedure_reload_tests.rs:88・175`・`worker_path_tests.rs:200-205` の出所での絞りを外す（`:323` は今の語のまま）・`doc/ukadoc-coverage/ledger/shiori.toml:7188` の説明の語。もう 1 つの読み手 `session_tests/shell_balloon_switch_session_update_tests.rs:243`（`audit-emo2-boot-switch` の持ち物・期待 1 件）がどちらの側を数えているかは R6。［議題 10 の (c)・議題 3］
26. **（S3-4）** テストの期限の無い待ち: `update/worker_tests.rs` の `handle.join()`（575・609・634・712・780・816）・`asks.recv()`／`rx.recv()`（108・214・538・557・565・591・601・605・625・626・696・703・710・768・775・810・813）・`thread::spawn(..).join()`（128・224・294）、`update/desk_tests.rs:442` の `rig.kanade.recv()`、`update/worker_path_tests.rs:225` の `ReplyReceiver::recv()`｜直しが間違うとテストが赤にならずに止まる。例: 背景のスレッドが `Started` の前に返事を待つイベントを 1 件足すと、テストは `asks.recv()` で、スレッドは返事待ちで互いを待ち、全体テストが終わらない｜既にある道具に置き換える（`handle.join()` → `join_bounded("update", handle)`・mpsc の `recv()` → `wait_recv`・`ReplyReceiver::recv()` → `recv_timeout`・std のスレッドの join → `join_thread_bounded`）。`worker_tests.rs` は 967 行なので、置き換えで行が増えるなら 73 で減らす分と合わせて 1,000 行を越えないことを確かめる。［件 4 と同じ種類・19］
27. **（S3-5）** `BootContext` の組み立ての写し 4 つ（`update/desk_tests.rs:49-71`・`desk_resolve_tests.rs:326-348`・`readonly_refusal_tests.rs:56-78`・`update_cue_tests.rs:67-89`）と `descript.txt` を書く補助の写し 4 つ（`desk_tests.rs:143-150`・`readonly_refusal_tests.rs:122-129`・`desk_resolve_tests.rs:89-92`・`update_cue_tests.rs:60-63`）｜`BootContext`・`ConfigInputs`・`GhostDecision` に欄が増えると 4 つを同じに直すことになる｜`update/` に `#[cfg(test)]` の支えのファイルを 1 つ作り、`boot_context(temp, base, ghost, balloon)` と `write_descript` を置いて 4 か所から呼ぶ。既にある `areka_session::reload::test_support::boot_context` はゴーストが `A`・バルーンが `kaku` に決め打ちで（この書き手が `reload_test_support.rs:17-41` で確かめた）、ここのテストのゴーストの名（`emo`・`pasta`・`bare`）はテストの中身に効くので、そのままは使えない。上書きの形（`BootContext { .., ..boot_context(..) }`）は上の段の欄（`roots`・`roots_origin`・`app_profile_dir`・`helper_exe`）しか替えられず、名を持つ `current` は丸ごと組み直しになるので写しが減らない。［件 6 と同じ種類・議題 4・45 との食い違いの解き方は 4 節の終わり］

**D. MCP の受け口（`mcp/`）**

28. **（S4-2）** `mcp/get_expression_table.rs:74-98・104-120`｜`std::fs::read` と構文の解析を UI スレッドで行う。MCP のツールで UI スレッドのままファイルを読むのはこれだけ（`dump_surface` は重い仕事を別のスレッドへ出す）。例: ゴーストのフォルダが OneDrive のオンデマンドのファイルだと、取り寄せが終わるまで止まりマスコットが固まる｜`handle` の中でシェルのフォルダを `PathBuf` に写し、名前つきのスレッドで `reply.send(outcome::value(render(&load(&dir))))` を行う。起こせなければ `error!` と `NG:`。フォルダが引けないときの道（`None`）は今のまま。`real_unit_answers_from_the_current_shell_each_call_without_side_effects` は `try_answer` の代わりに `wait_answer` で待つ形にする。［R8］
29. **（S4-3）** `mcp/get_expression_table_tests.rs:49・102・145・165・171・192・202・232・259・310・342・416・435・457・471` とテストの名 `sample_1_…`・`sample_3_…`〜`sample_9_…`・定数 `SAMPLE_1`・`SAMPLE_8_ROWS`｜「実測 14」「検体 1〜9」は完了した spec の作業文書（`completed/areka-P0-mcp-expression-table/ssp-measurements.md`）の番号で、`doc/ssp-mcp/survey.md` には無い（CLAUDE.md の `## Comments` に反する）｜コメントから番号を消して中身の言葉だけにし（例「SSP の実測の全文」）、テストの名と定数を中身で付け直す（例 `sample_1_matches_the_ssp_answer_in_full` → `two_groups_match_the_ssp_answer_in_full`）。［数え直した数］
30. **（S4-4）** `mcp/dump_surface.rs:353-367`（`check_size`）｜試すテストが無い。中の `width == 0` の確かめを消しても赤にならない。確かめが無いと、幅 0・高さ 1 以上・バイト数 0 の絵が通り `png_image.rs:28` の `rgba.chunks_exact(0)` が panic する（符号化のスレッドが受けて `NG:the encoding thread panicked`）。バイト数が合わないと `chunks_exact` が端数を黙って捨て、壊れた PNG を返す｜`dump_surface_tests.rs` に決定論のテストを 1 本足す（(0,1,空)・(1,0,空)・(2,2,長さ 15) が `Err`、(1,1,長さ 4) が `Ok`）。［78］
31. **（S4-5）** `mcp/dump_surface_gpu_test_support.rs:410-439`｜`wic_decode_rgba` が本体の `png_image_tests.rs:166-195` と一字一句同じ写し｜`png_image.rs` のテストのモジュールを `#[cfg(test)] pub(crate) mod tests` にし `wic_decode_rgba` を `pub(crate)` にして、こちらから呼び写しを消す（約 −30 行）。**区画 S4 は書いていないが、`crates/areka/src/png_image.rs`・`png_image_tests.rs` は範囲の外で `audit-areka-app` の持ち物**（この書き手が置き場を確かめた）。向きを逆にしても（こちらの写しを `pub(crate)` にして向こうが使う）向こうのファイルに触る。［議題 3］
32. **（S4-6）** `mcp/dump_surface_gpu_test_support.rs:392-408`｜`decode_png` が一時フォルダの組み立てを自前で持つ（`static NEXT`・`target/test-tmp`・最後の `let _ = remove_dir_all`）。ワークスペースで 1 か所と決めた `temp_path_kit::TempPath` があり、本体の dev 依存に入っている。今の形では WIC の復号が `expect` で panic したときにフォルダが残る｜`let dir = TempPath::new("dump-gpu"); let file = dir.child("dump.png");` にして `NEXT` と `remove_dir_all` を消す（約 −8 行）。
33. **（S4-7）** `mcp/dump_surface_gpu_test_support.rs:55-65・321-371`｜`spawn_windows`・`pump_messages`・`WORK_AREA`・`TICK_EVERY`・`PUMP_MAX` は `areka-session/src/shell_balloon_switch_session_lap_test_support.rs:32-39・191-223・289-307` のほぼ写し（違うのは DPI を呼び手が選べるかだけ）。写しはずれ始めている（あちらの `frames_until` は UI の側の進みも数える）。頭の doc（6〜7 行）は `spawn_windows` が `shell_balloon_switch_session_lap_tests.rs` にあると書くが、実物は `_lap_test_support.rs`｜あちらの `spawn_windows` に `dpi: u16` を取らせて `pub` にし、`pump_messages` も `pub` にする（このモジュールは `test-support` で `pub mod`・本体の `session_tests` が既に使う）。こちらの写しと定数を消し、doc の行を直す（約 −45 行）。あちらは `audit-emo2-boot-switch` の持ち物＝この spec は doc の行だけ直し、あちらの形の変更は申し送る。［議題 3］

34. **（S5-1）** `mcp/get_status_tests.rs:92・94・101・103・127・130・173・176・202・205・237・241`｜テストの名 `m1_`〜`m6_` と doc の「M1:」〜「M6:」は、完了した spec の design（`completed/areka-P0-mcp-get-status/design.md` の Testing Strategy）の番号そのもの（CLAUDE.md の `## Comments` に反する）｜6 本の名から `mN_` を外し、doc の「MN: 」を消す（例 `no_state_is_an_empty_body`）。［数え直した数］
35. **（S5-2）** `mcp/get_status.rs:24-32`｜「降りた」の文言を求める式と kanade の送り口を引く式が、`sakurascript.rs:44-54` の `script_target` と一字違わず同じ（`raise_event` は `script_target` を使い、`get_status` だけが写しを持つ）。文言の決まりを片方だけ直すと 3 本の答えが食い違う｜`let ScriptTarget { kanade, gone } = …script_target(world, args.ghost_name.as_deref());` に置き換えて 9 行を消す。3 本で使うので `resolve.rs` へ移す。
36. **（S5-3）** `mcp/check_script.rs:39-50`｜表示の結線のある腕（装着の前は `None` で待ち、装着の後は `GhostWindows::scopes` を `u32` に直して `skin_facts_copy::copy` で写す）を、どのテストも通らない（`check_script_tests.rs:247` は `RuntimeWiring` を外した「窓の無いゴースト」の腕だけ）。例: `!wiring.attached()` の判定が逆になる、または scope を取り損ねると、本物のゴーストで `missing_surface`／`missing_balloon` が 1 件も出なくなるのに全部緑｜`dump_surface_gpu_test_support.rs` の足場で本物のシェルを装着させ、無い surface の `\s[99999]` に `missing_surface` が 1 件出るテストを 1 本足す（実 GPU）。
37. **（S5-4）** `mcp/get_property_tests.rs:153・368`｜`publisher.barrier().expect(..)` は期限なしで待ち、書き手が詰まるとテストは止まり続ける｜2 か所とも `publisher.barrier_within(期限).expect(..)` にする（期限は 1 と一緒に R2 で決める）。［件 4 と同じ種類・1・19］
38. **（S5-5）** `mcp/get_status_tests.rs:19-75`・`raise_event_tests.rs:22-96`・`sakurascript_tests.rs:21-90`｜偽の kanade の足場が 3 本にほぼ同じ形で写されている（`ghost()`・`rig()`・`answer()`・`warn_or_worse()` は 3 本とも、`text()` は `raise_event_tests.rs:98`・`sakurascript_tests.rs:92`、`record_one()` は同 293・266、`strict_records()` は同 380・360）｜`mcp/kanade_fake_test_support.rs`（`#[cfg(test)]`）を 1 本作って補助を 1 回ずつ置き、3 本はそれを `use` する（約 −100 行）。`strict_records` は 84 で消すので移さない。［84］
39. **（S5-6）** `mcp/check_script_judge.rs:20-28`｜`MSG_*` の 8 個は `strict::MSG_*` を指す別名の定数で値を足していない。使うのはこのファイルとそのテストだけで、テストは `MSG_DEFAULTED` などを既に `strict` から直に引き、引き方が 2 通りに割れている｜8 個の定数と 20 行の注釈を消し、`use areka_sakura::strict::{MSG_UNKNOWN_TAG, ..}` にする。`doc/ssp-mcp/areka-tools.md:6` の「check_script_judge.rs の定数はこれを指す別名」の一文も同じ変更で直す（文書の持ち主は C26 のほかの 2 本でない＝この spec が直す）。

**E. 根の一覧と置き場の編集（本体の `roots/`）**

40. **（S6-2）** `roots/desk.rs:71-145`（`start_pick`・`send_pick`）｜`install/desk.rs:184-268` の `start_pick`・`send_pick` とほぼ同じ写し（旗の確かめ・上げ下げ・スレッドの起こし方・送れないときの warn）。写しの間で同じ決まりの扱いが違う: 告知を抑えているときの記録は、置き場の側が `info!(roots_pick_cancelled, reason="no_alert")`、インストールの側が `warn!(install_pick_suppressed)`（どちらも完了した spec の設計の決め）。`AREKA_NO_ALERT` で同じように画面を出さないのに、水準と形が食い違う｜`install/desk.rs` の側に「旗・スレッド名・抑止・選ぶ口・選んだ物の送り方」を受ける関数を 1 つ置き、両方の窓口から呼ぶ。抑止の記録の水準と形は議題 10 の (d) で決め、両方のテストを合わせる。［89・議題 10 の (d)］
41. **（S6-3）** `roots/edit.rs:268-269`｜コメントは「書けなければ `save_scope` が error を残す」と言うが、実際は書き手の失敗が `WriteFailureTx` を通って `write_failure::fatal` へ渡り areka が終わる（下の `None` の腕と同じ帰結）。読み手は「error が出るだけ」と取り違える｜コメントを「書けなければ書き手が致命の机へ送る（`None` の腕と同じく終わる）」に直す。
42. **（S6-4）** `roots/edit.rs:33-37・230`｜`RemoveVerdict::Removed` の欄 `removed` は本番で `Removed { roots, .. }` と捨てられ、テストだけが読む。外したときの記録は変更後の並び（`root_listed`）しか残らず、何を外したかが記録から読めない。先頭へ動かすときは `info!(roots_moved_to_front, path)` を残すので扱いが揃っていない｜`remove_root` で `info!(event = "roots_removed", path = %removed.display())` を残す（記録が要らないと決めるなら欄 `removed` をテストごと消す）。［議題 10 の (e)］
43. **（S6-5）** `roots/edit.rs:1-4`｜見出しは「判断（純粋）」「値だけで判断する」と書くが、`decide_add`（95 行）は `ghost_store().is_dir()`・`balloon_store().is_dir()` でファイルシステムを読む｜見出しを「足す判断だけは足すフォルダの直下を見る」と事実に合わせる。
44. **（S6-6）** `roots/edit_tests.rs:357-373` と `roots/edit_write_failure_tests.rs:45-89`｜2 本がほぼ同じことを確かめる（記憶の置き場がファイル・足す・並びは変わる・`roots_memory_write_failed` の error 1 件）。後者は `FirstExit` まで見るので前者を含む｜`edit_tests.rs` の `add_takes_effect_and_logs_error_when_memory_unwritable` を消す。［91］
45. **（S6-7）** `roots/edit_tests.rs:279-305`（`apply::world`）・`roots/edit_write_failure_tests.rs:17-43`（`world`）・`roots/desk_tests.rs:36-58`（`rig` の中）｜同じ `BootContext` の字面が 3 つ（ゴースト `ga`・バルーン `B`・`RootsOrigin::Default`）。件 6 と同じ種類で、範囲の中の写しは件 6 の 2 つより多い｜既にある `areka_session::reload::test_support::boot_context(temp, &BasewareRoot::new(root))` を使い、違いだけを上書きする（例 `BootContext { roots_origin: RootsOrigin::Default, app_profile_dir, ..boot_context(..) }`。向こうの既定は `EnvVar` なので上書きしないと置き場の変更は全部断られる・本体の dev 依存は `test-support` を開けてある）。上書きできるのは上の段の欄だけで、ゴーストの名は `A` になる＝`ga` の名がテストの結果に効かないことを R3 で確かめる。効くなら 27 と同じく範囲の中の名を取る補助へ寄せる。［件 6・議題 4・11・27・58］

**F. 根の芯（`areka-session/src/roots/`）**

46. **（S6-8）** `areka-session/src/roots/user_root.rs:116-118`｜`roots.user_root()` が `None` なら記録を残さずに `dir` を返す（記録の無い失敗の道）。今は届かない（直前の `plan_copy` が同じ判定で `Copy` を返したときしか通らない）｜`CopyPlan::Copy` に書ける根の `dir` も載せ、この `let … else` を消す。（この書き手が 110〜118 行を確かめた）
47. **（S6-9）** `areka-session/src/roots/report.rs:44`｜doc に要件の番号「（9.1）」｜「（9.1）」を消す。［数え直した数］
48. **（S6-10）** `areka-session/src/roots/mod.rs:3-4`・`roots/resolve.rs:1・87`｜「純粋・値の注入だけでテストする」「純粋な判断」と書くが、`resolve_roots_from` は外を読む（180 行の `dir.is_dir()`、221 行の `std::path::absolute` はカレントのフォルダを読み `warn!` を出す）｜doc を「環境変数と記憶は値で受ける・根が在るかと相対の解決だけはその場で見る」に直す。

**G. 説明書（`readme.rs`・`readme/`）**

49. **（S6-11）** `areka-session/src/readme.rs:63`｜「呼び手は boot 成功後の `wire_runtime`」は古い（`wire_runtime` は `areka-runtime` にあり `areka-session` を呼べない）。実際の呼び手は `areka-session/src/runtime_boot.rs:194` の `install_lines`｜「呼び手は `runtime_boot::install_lines`（実行系の成立の後）」に直す。［件 3 と同じ種類・8］
50. **（S6-12）** `areka-session/src/readme.rs:75-85・122-124`｜⑴ `set_path`（descript の読み直し）は `missing_logged` を戻さない。例: 起動時に説明書が無くて 1 度記録した後、読み直しで `readme` キーが別の無いファイルを指すと `readme_missing` はもう出ず、新しい置き場の不在が記録に残らない ⑵ doc の「毎 tick の照会」は事実と違う（呼び手はメニューの `readme_item`＝写しを取るたびだけ）｜`set_path` で `wiring.missing_logged.set(false)` を足し、doc を「メニューの写しを取るたび」に直す。読み直しの後に 1 回記録されるテストを 1 本足す。（この書き手が `set_path` を確かめた）
51. **（S6-13）** `areka-session/src/readme/readme_cue.rs:103`｜本番の記録の文に「（要件 4.6）」があり、利用者の記録にそのまま出る｜文から「（要件 4.6）」を消す（記録の語 `event` は変えない）。［数え直した数］
52. **（S6-14）** `areka-session/src/readme/readme_cue.rs:13・97`・`readme_cue_tests.rs:58`｜「α では語彙だけを持ち」と、版の段の使わなくなった呼び名で書く｜「列挙が要るので語彙だけを持ち、警告して何もしない」にする。
53. **（S6-15）** `areka-session/src/readme/os_port.rs:5`｜「`crates/areka/src` で `ShellExecute` を綴るのはこのファイルだけ」は古い（このファイルは `areka-session` にあり、見張りの `readme_body_tests.rs` は 4 つの crate の木を見る）｜「本体・`areka-runtime`・`areka-input`・`areka-session` の `src` で綴るのはこのファイルだけ」に直す。
54. **（S6-16）** `areka-session/src/readme/os_port.rs:53-55・59・65・110・177-178`｜`#[cfg_attr(test, allow(dead_code))]` が 5 か所、doc はその理由を「テストのビルドに呼び手が無い」と書く。どちらも事実と合わない: `WindowsShell` と `init_com_for_shell` は `pub mod readme` の下の `pub` の項目で dead_code の警告はもともと出ず、本体の `cli/help.rs:49-51` が実際に呼ぶ（この書き手が確かめた）。私有の `wide`・`ERROR_NO_ASSOCIATION`・`has_verb` は `pub` の型の trait の実装から使われる。先送りの印でもない（要件 2.2 の 2 種類目に当たらない）｜5 つの `allow` と 53〜54 行・177 行の説明を消す。外した後に clippy で警告が出ないことを確かめる（R9）。
55. **（S6-17）** `areka-session/src/readme/os_port.rs:180`｜SAFETY の「対の CoUninitialize はスレッドの終わりまで呼ばない」は事実と違う（スレッドの終わりにも呼ばない・`serve` から戻ってもそのまま抜ける）｜「対の CoUninitialize は呼ばない（このスレッドは開く要求を受け終えるまで生き、終わりで COM ごと片づく）」と事実の形に直す（`// SAFETY:` は残す）。

**H. テストの番号と写し（根・説明書）**

56. **（S6-18）** 要件の番号を書いたテストの文 21 か所: `roots/edit_tests.rs` 327・332・393・414・440、`areka-session/src/readme_tests.rs` 84・135・144・148・177・183・189・197・247・356・362・366、`readme/readme_cue_tests.rs` 40・58・63・160｜各文から「（要件 N.N…）」「・要件 N.N」の部分だけを消す。［数え直した数・52・98 と同じ行あり］
57. **（S6-19）** `areka-session/src/readme/readme_cue_tests.rs:24-27`・`areka-session/src/readme_tests.rs:47-50`｜`capture_lines(LineFormat::…, f)` を包むだけの私有の関数が 2 つ（`capture_logs` と `capture`）。`readme_tests.rs` の分は件 5 と同じ種類の 2 つ目で、前の版には挙がっていなかった｜両方とも包みを消し、呼ぶ所を `capture_lines(..).1` にする（`areka-session` のほかのテストと同じ流儀）。［件 5］
58. **（S6-20）** `areka-session/src/readme/opener_submit_tests.rs:31-55`｜`reload::test_support::boot_context` とほぼ同じ写し（ゴースト `g`・バルーン `b`・`GhostRoute::Memory`）｜件 6 の直し方に従う（既にある補助を読む・違いが効かないかは R3）。［件 6・議題 4・11・27・45］

### できれば（43 件）

**インストール**

59. **（S1-9）** `install/desk_tests.rs:385・554`｜`BODY_SYSTEMS` を足す 1 行が 2 本のテストにある｜議題 5 の推奨どおり寄せずに残し、理由を記録に書く。この区画に新しく足す直しは無い。［件 7・議題 5］
60. **（S1-10）** `install/desk.rs:498-499`｜素性に答えるたびに `catalog::sakura_name(dir)` と `catalog::install_accept(dir)` が UI スレッドで同じ `ghost/master/descript.txt` を 2 回読む（`master_descript_keys` は `areka-ghost` の私有）。読めないときは同じ `warn!` が 2 件並ぶ｜窓口は `dir` だけを答え、背景のスレッドで読む。あわせて `areka-ghost` の `catalog.rs` に 2 つを 1 回の読みで返す公開の関数を 1 つ足し、それを呼ぶ（`audit-ghost-catalog` は完了・C26 のほかの 2 本の持ち物でない＝この spec が直す・依存は変わらない）。（この書き手が `catalog.rs:302-325・358` を確かめた）
61. **（S1-11）** `install/destination.rs:94-122`｜根を尋ねられるたびに、書ける根の印があると `copied_root` が `find_ghost`／`find_balloon` で目録を丸ごと読み、続く `install_root` がもう一度丸ごと読む（背景のスレッドで、問い合わせのたびに 2 回）｜`copied_root` で見つけた項目の `root` をそのまま答えに使い、見つからないときだけ `install_root` を呼ぶ。
62. **（S1-12）** `install/desk_overwrite_tests.rs:66-70`｜`clues` の doc は「`areka_nar` の失敗の記録は背景のスレッドで出るので capture に映らない」と言うが、同じファイルの 582〜605 は捕まえた `areka_nar` の ERROR を 1 件確かめている（起動中のゴーストへの展開は UI スレッドで走るので映る）｜「よそへの展開の失敗（`install_failed`・`areka_nar`）は背景のスレッドで出るので映らない」に絞る。13 を直すと展開も背景へ移るので、そのときは doc と 582〜605 の表明をまとめて直す。［13］
63. **（S1-13）** `install/desk_tests.rs:203-226`｜断りの 3 つの場合は「WARN が 1 件」しか見ていない。記録の名 `install_order_refused` や `verdict` の欄が変わっても、別の warn に置き換わっても通る｜`events_named(&events, "install_order_refused")` が 1 件で、`verdict` 欄が `Empty`／`NoDesk`／`Closing` であることを確かめる。
64. **（S1-14）** `install/desk.rs:452-460`｜今のゴーストが居ないときにバルーンの記憶を書けない分かれ目（`warn!(install_balloon_not_remembered)`）を固定するテストが無い｜置き場が空の窓口で `remember_balloon` を呼び、warn が 1 件出るテストを `desk_record_tests.rs` に 1 本足す。
65. **（S1-15）** `install/judge.rs:246`｜URL の腕の判定は `starts_with("http://")`／`"https://"` で大文字小文字を区別する。`\![execute,install,url,HTTPS://example.com/a.nar,nar]` は `BadUrl` で断られるが、URL のスキームは大文字小文字を区別しない（RFC 3986）｜先頭を `eq_ignore_ascii_case` で比べ、`judge_tests.rs` に大文字の場合を 1 つ足す。`fetch_url.rs` の側（範囲の中）の扱いもそろえる。［議題 10 の (h)］

66. **（S2-13）** `install/fetch_url.rs:133-138`｜`sweep_old` は消した数が 0 でも、取得のたびに `info!(install_fetch_swept)`「7 日より古い一時ファイルを消しました」を出す｜`if removed > 0 { … }` で囲む。［議題 10 の (b)］
67. **（S2-14）** `install/fetch_url.rs:64-67`｜書き込みの途中で失敗すると（ディスクがいっぱい）、書きかけのファイルを消さずに残す｜`fs::write` が Err の腕でそのパスを `remove_file` し、消せなければ `debug!` を 1 件残す（終了の途中の落としかけは今の 7 日の掃除のまま＝完了 `areka-P0-network-update` の要件 7.5 と食い違わない）。
68. **（S2-15）** `install/fetch_url.rs:3-5`（片付ける場所は `install/worker.rs:73-76`）｜落としてインストールし終えた書庫も一時フォルダに残り、掃除は「次の取得のとき」にしか走らない。例: URL で 3 本（各 50 MB）入れて以後 URL を使わなければ、150 MB が `%TEMP%\areka\download` に残り続け、doc の「7 日」も守られない｜`spawn_worker` で `run_order` を終えた後、出どころが `Script` で `download_dir()` の下にある書庫を消す。テストを 1 本足す。完了 `areka-P0-network-update` の要件 6.6（「その場では消さない＝手続きが読み終える時点を知る口が無い」）を改める直し。［議題 9］
69. **（S2-16）** `install/worker.rs:54-62・78`（窓口の側は `install/desk.rs:297`）｜`DeskAsk::OrderDone` の欄 `order`・`ends` は本番では誰も読まず、`#[cfg_attr(not(test), allow(dead_code))]` で警告を消してテストのためだけに運ぶ。`worker_tests.rs:344-355` の確かめも 348 行の `raised == ["OnInstallBegin", "OnInstallFailure"]` で既に分かる。印の理由は「試験だけが読む」で、先送りの印ではない（要件 2.2 の 2 種類目に当たらない）｜2 つの欄と `allow` を消し、`OrderDone` を欄の無い腕にする（`update/worker.rs:56` と同じ形）。`worker_tests.rs:350-355` を消し、`install/desk_tests.rs:262・279` も合わせる（どちらも範囲の中）。［5 の直しも `OrderDone` を触る］
70. **（S2-17）** `install/names.rs:52`・`install/names_tests.rs:131`｜呼び手の説明が実際と違う。`reseed` を呼ぶのは本体の `main.rs:412` の `wire_body_after_runtime` で、道は `boot_wired` → `rewire_runtime_consumers` → `BODY_SYSTEMS.after_runtime_wired`。テストの「（`boot_wired` の 1 行）」は事実と違う｜「（`BODY_SYSTEMS` の `after_runtime_wired`＝本体の `wire_body_after_runtime` から）」に直す。［件 3 と同じ種類・8］

**ネットワーク更新**

71. **（S3-6）** `update/mod.rs:223-224`｜doc が呼び手を `ghost_session::register_systems` と書く。実際は `main.rs:376` の `register_body_after_input`（`BODY_SYSTEMS` の `after_input`）｜「`BODY_SYSTEMS` の `after_input` から」の 1 行に直す。［件 3・8・81］
72. **（S3-7）** `update/desk_reload_tests.rs:83-90`（`ask_reload`）｜`update/desk.rs:553-560` の `ask_reload_for_test` と中身が同じ｜私有の写しを消し、`super::ask_reload_for_test` を呼ぶ（呼びは 6 か所）。
73. **（S3-8）** `update/worker_tests.rs:684-694・756-766・799-808`｜バルーンのテスト 3 本が、背景のスレッドを起こして仕事を渡す手順を手で書き直す（同じ手順は `run_one`（507〜529）にあり、違いは `roots`・`exe_dir` を渡すかだけ）。ファイルは 967 行で 1,000 行の番人に近い｜`run_one` に `roots`・`exe_dir` の引数を足し、3 本から呼ぶ（約 −30 行）。［26］
74. **（S3-9）** テストの補助の写し: イベント名で絞る補助が 7 ファイル（`update/desk_tests.rs:164`・`desk_reload_tests.rs:126`・`procedure_tests.rs:77`・`procedure_reload_tests.rs:19`・`update_cue_tests.rs:109`・`worker_tests.rs:95`・`worker_path_tests.rs:123`。ほかに `desk_resolve_tests.rs:111・507・575` で同じ形を手で書く）・`run` の写し（`procedure_tests.rs:84`・`procedure_reload_tests.rs:26`）・呼び出しの列を綴りにする補助（`desk_reload_tests.rs:93` の `ids`・`worker_path_tests.rs:101` の `kinds`）・`refs_of` の写し（`desk_reload_tests.rs:119`・`worker_path_tests.rs:113`）｜27 の支えのファイルに 1 つずつ置き、各所から呼ぶ。［27］
75. **（S3-10）** `update/update_tests.rs`（全 21 行）・`update/mod.rs:234-235`｜綴りの 5 語は、ほかのテストが既に固定している（ghost は `refs_tests.rs:467`、shell は `:293`、balloon は `:259`、manual は `:250`、script は `:260`、ほかに `procedure_tests.rs:257`）｜ファイルと `mod` 宣言を消す。
76. **（S3-11）** `update/desk.rs:37` の `type Tail`（後送りの列）・`update/refs.rs:124` の `struct Tail`（Reference3・4）｜「Tail」が 2 つの別物を指す。`procedure.rs` は `refs::Tail` を import しつつ列の変数も `tail`（141 行）と名付け、`run_target` では構造体の変数も `tail`（226 行）。列の型 `Vec<(&'static str, Vec<String>)>` は `procedure.rs:32・141・224`・`worker.rs:53・244`・`procedure_test_support.rs:27` の 6 か所で書き下し、`desk.rs:259` はその slice の形｜列の型の別名を `mod.rs` へ移して 7 か所で使い、`refs` の構造体を `Ref34` に改名する。
77. **（S3-12）** コメント: なぞるだけで消せばよいもの＝`update/desk.rs:184`・`procedure.rs:88・157・176・194・229・271`。古い・事実と違う doc＝`update_cue.rs:30`「α では受けない」（使わない語・「受けない」に直す）・`readonly_refusal.rs:3-4`「readonly の SSP の測り（2026-10-23）どおり」（日付は作業の記録を指す・SSP の振る舞いだけを書く）・`procedure_test_support.rs:163`「4 経路の台本。」（続く台本は 6 つ・行を消す）・`worker.rs:47` の `DeskAsk::Started`（メニューの場合しか書かない・台本からの要求は `OnUpdateProcessExec` を送らずに `Started` を頼む）・`desk_tests.rs:3-5`（見出しは「背景のスレッドは起こさない・どの受け取りも待たない」と書くが 432〜449 のテストは本物のスレッドを起こし `recv()` で待つ）｜書いたとおりに直す。

**MCP**

78. **（S4-8）** `mcp/dump_balloon.rs:162-164`｜大きさの決まりが `check_size` と二重。こちらは `w*h*4` をあふれの確かめなしで掛け、0 の寸法も通す｜`check_size(TOOL, scope, &bytes, *w, *h)?;` の 1 行に置き換える。［30］
79. **（S4-9）** `mcp/dump_surface_gpu_tests.rs:45-62・123-148`・`mcp/dump_balloon_gpu_tests.rs:105-131`｜「本文＋画像 1 枚を読む」と「食い違った画素を数える」が 3 か所に同じ形である（`shown_surface_matches_the_composed_pixels_at_native_size` は同じファイルの `read_picture` を使わず手で書き直す）｜`differing` を 1 つ `dump_surface_gpu_test_support.rs` に置いて両方のファイルから使い、最初のテストは `read_picture`／`picture_of` へ寄せる（約 −15 行）。
80. **（S4-10）** `mcp/mcp_tests.rs:452-472`｜`production_files_have_no_not_implemented_yet` は、昔あった仮の実装の文言をソースの字面から探すテスト。その文言を返す本番のコードはもう無く（この書き手がツリー全体を grep した＝残るのは `areka-mcp` のテストと `get_status_tests.rs:239` の doc だけ）、振る舞いが壊れても赤にならない（陳腐化テスト）｜消す（−21 行）。
81. **（S4-11）** `mcp/mod.rs:48`｜doc が呼び手を「`ghost_session::register_systems` から」と書く。名目上の渡し手は `main.rs:375-380` の `BODY_SYSTEMS` の `after_input`｜「`BODY_SYSTEMS` の `after_input` から」に 1 行で直す。［件 3・議題 7・8・71］

82. **（S5-7）** `mcp/get_log.rs:46・49`｜`Some("") => return outcome::ng(CANNOT_FIND)` の腕は要らない（`resolve::resolve` は空文字にも空白だけにも `CANNOT_FIND` を返す。46 行と `get_log_tests.rs:392` の注釈も自分で「同じ Cannot find」と書く）。49 行の `Err(_) => CANNOT_FIND` も、解決が返す理由と同じ字の書き直し｜46 行を消し、49 行を `Err(reason) => return outcome::ng(reason)` にする。
83. **（S5-8）** `mcp/get_log.rs:19-24`｜振り分け（`mod.rs:72`）が要求ごとに求めた `active` を捨て、`get_log::handle` の中でもう 1 度 `resolve::active(world)` を引く｜`handle(world, active, args, reply)` にして `mod.rs:109` から渡し、20〜23 行を消す（`mod.rs` も範囲の中）。
84. **（S5-9）** `mcp/raise_event_tests.rs:379-432`・`sakurascript_tests.rs:359-366・419-461`｜「strict の記録が増えない」2 本は、どう壊しても赤にならない（足場は偽の kanade＝`mpsc` の送出端で再生の仕組みが無く、MCP の `handle`・`drain` は `RECORD_TARGET` へ 1 行も出さないので、数は本番のコードによらず必ず 0）｜2 本と、それだけが使う `strict_records` を消す（約 −80 行）。［38］
85. **（S5-10）** `mcp/sakurascript_log_tests.rs:1-53`（`sakurascript.rs:125-127` から据える）｜中身は `areka_kanade::script_log::emit` と `areka_session::log_history::draft` の組み合わせだけで、本体のコードを 1 行も通らない。module の doc の「get_log で読める」も事実と違う（`get_log` は呼ばない）｜テストを `areka-session` の `log_history` のテストへ移し、doc を「kanade の script の記録が下書きで script 種別になる」に直す。`sakurascript.rs:125-127` の据え付けは消す。`log_history*` は `audit-areka-app` の持ち物＝移す先のテストは向こうへ申し送り、こちらの削除は向こうの着地と順を合わせる。［議題 3］
86. **（S5-11）** `mcp/check_script_tests.rs:18-44`｜手で組んだ事実の `ByHand` と `aliases()` は、`check_script_judge_tests.rs:12-50` の `Table` と `ghost()` と同じ表（scope 0 が 0・3・10、scope 1 が 10・11、別名の `笑顔` が 3、バルーンが 0:[0,2]・1:[1]）｜`Table`／`ghost()` を `pub(super)` にして `check_script_tests` から使い、`ByHand` を消す。
87. **（S5-12）** `mcp/check_script_judge_tests.rs:79`｜見出しの「種類の表の 14 行」は数が違う（下に並ぶのは row01〜row13 の 13 行で、14〜16 行目は 233 行、17 行目は 277 行の見出しの下・module の doc は「17 行」）｜「種類の表の 1〜13 行目」にする。
88. **（S5-13）** `mcp/reload.rs:17`｜本番では、インストールの途中に `reload` を呼ぶと `RefusedReason::Installing` で断られる（`install_busy` は `main.rs:371` で結線）。そのときの答え `NG:Cannot reload now (another switch, reload or update is in progress)` はインストールを挙げないので、AI は更新が走っていると読み違える。この文は SSP の実測でなく areka の文で、完了 `areka-P0-mcp-reload` の要件 3（requirements.md 102 行）が字で決めた｜文を `(another switch, reload, update or install is in progress)` にし、`reload_tests.rs:135・205` の期待を同じ字にする（ほかに同じ字を使う所は無い＝区画 S5 とこの書き手が grep した）。［議題 10 の (f)］

**根の一覧と置き場の編集**

89. **（S6-21）** `roots/desk.rs:22-26・56`｜変種が 1 つしかない `enum PickedFolder { Chosen(PathBuf) }`｜列挙を消し `Sender<PathBuf>` にする（40 でまとめるならその中で消える）。［40］
90. **（S6-22）** `roots/edit.rs:62-65`｜`is_temp` は `root.is_temporary()` を呼ぶだけの包み（メニューの側は直に呼ぶ）｜包みを消し、3 か所を `r.is_temporary()` にする。
91. **（S6-23）** `roots/edit.rs:279-289`｜記憶を書けなかった失敗 1 件で、記録の形が腕によって違う。ゴーストが居ない腕は `roots_memory_write_failed` と `memory_write_failed`（`fatal`）の error を 2 行残し（2 行目は同じ dir と reason の繰り返し）、居る腕は `memory_write_failed` だけ｜`roots_memory_write_failed` の `error!` を消して `fatal` の 1 行にそろえ、`edit_write_failure_tests.rs` の判定も 1 行に直す。ただし完了 `areka-P0-write-failure-notice` の要件 5.5（requirements.md 160 行）が「`roots_memory_write_failed` の名前を変えない」と決めている。［議題 10 の (i)・44］
92. **（S6-24）** `roots.rs:6`｜`pub(crate) use edit::*;` は判断の型と `decide_*` まで crate に見せるが、外から使うのは `add_root`・`remove_root`・`move_to_front` だけ（ツリー全体で 0 件）｜その 3 つの名前だけを出し、判断の型と関数は module の中に閉じる。
93. **（S6-25）** `roots/edit_tests.rs:21-27` と `roots/desk_tests.rs:63-69`（`roots_of`）、`edit_tests.rs:307-312`（`named`）と `desk_tests.rs:80-85`（`events_named`）｜同じ module のテストに同じ補助の写しが 2 組｜`roots` のテスト用の補助に 1 つずつ置いて両方から使う（45 の寄せと同じ所）。
94. **（S6-26）** `areka-session/src/roots/resolve.rs:67-72・135-140`｜`RootError::ExeLocationUnavailable` の欄 `tried`・`origin` は作る所が 1 つで、いつも `Vec::new()` と `RootsOrigin::Default`（値を持たない欄）｜2 つの欄を消し、読み手を `EnvAllEmpty` と同じ形（定数）にし、テスト 3 か所も直す。**区画 S6 は書いていないが、読み手の `areka-session/src/alert.rs:150` と `alert_tests.rs:51` は `audit-areka-app` の持ち物**（この書き手が grep で確かめた）＝`pub` の列挙の形を変えるので、要件 7.1・7.2 の申し送りになる。［議題 3］
95. **（S6-27）** `areka-session/src/roots/report.rs:134-137` と `roots/user_root.rs:94-99`｜`StoreKind` から "ghost"・"balloon" を返す対応が同じ crate に 2 つ｜`user_root.rs` の `kind_name` を `pub(super)` にして `report.rs` から使う。
96. **（S6-28）** `areka-session/src/roots/resolve.rs:267-271`｜長いパスの接頭辞 `\\?\` を剥がして見る分岐にテストが無い。剥がしを消すと記憶の `\\?\C:\…` の項目が `?` のせいで「invalid」として黙って飛ばされるが、落ちるテストが無い｜`decode_memory(r"\\?\C:\x", None)` が項目 1 つで控えが 0 件になるテストを 1 本足す。

**説明書**

97. **（S6-29）** `areka-session/src/readme/os_port.rs:157-165`｜HRESULT を Win32 の符号（下位 16 ビット）へ戻す分岐が unsafe の呼び出しの中に埋まり、テストが無い（値は記録の `code` 欄に出る）｜`fn win32_code(hr: i32) -> u32` に切り出し、`0x80070002 → 2` と `0x80004005 → そのまま` の 2 例を `os_port_tests.rs` で確かめる。
98. **（S6-30）** `areka-session/src/readme/readme_cue_tests.rs:108-123・125-147・151-162・240-284`｜個別のテスト 5 本（`non_carrier_cue_is_benign_skip`・`unopenable_custom_addressed_to_open_…`・`dropped_receiver_is_logged_…`・`…_to_jump_…`・`…_to_others_…`）は、表のテスト `every_readme_cue_line_carries_the_prefix_and_an_event`（326〜403）と同じ場面の水準と件数を重ねて見ている｜表の各行に「送った件数 0」の判定を 1 つ足し、5 本を消す。［56］
99. **（S6-31）** `areka-session/src/readme/opener_tests.rs:17-56`・`opener_execute_tests.rs:17-48`・`opener_submit_tests.rs:25-29`｜`dest()`（`classify(..).expect().expect()`）が 3 か所、`Fixture`（最小のゴーストと `OpenContext`）が 2 か所（差は `readme.txt` とシェル `s` の有無だけ）｜`opener_test_support.rs` に `dest` と文脈を組む補助を 1 つずつ置いて 3 つのファイルから使う。
100. **（S6-32）** `areka-session/src/readme/opener_test_support.rs:116-161`｜偽物そのもののテスト 3 本は、本物のテストが既に通す道を重ねて見ている（返す符号は `opener_execute_tests.rs` の `os_refusals_…`・環境変数の大文字小文字は `opener_tests.rs` の `open_file_expands_env_vars_…`・組み立ては全部のテストが使う）｜`mod tests` を消す。
101. **（S6-33）** `areka-session/src/readme/destination_tests.rs:382`｜テストの名は `…_returns_six_forms_…` だが、確かめているのは 7 件｜名前を `…_returns_open_forms_in_order_…` にする（この関数 `link_destinations` の印は議題 11）。

### `audit-areka-app` から回ってきた指摘（直したほうがよい・2 件）

102. **（app-1）** `mcp/get_property_tests.rs:144-300`（156 行の `use crate::shiori_host::ShioriHostSink;`・236〜300 行の `mcp_and_ghost_paths_answer_the_same_window_rects`）｜「ゴーストの道」を、本体の SHIORI の試験用の脳 `ShioriHostSink`（`IShioriHost` の `get_property`）で作っている。本番でゴーストが値を読む道は台本の受け口 `areka_ghost::prop_get_sink::PropGetCueSink`（合図を受けて `KanadeMsg::PropertyEvent` を kanade へ送る）で、`ShioriHostSink` は通らない＝「MCP とゴーストが同じ値を答える」を本番の道で確かめていない。`audit-areka-app` は脳の 8 ファイルを消すと決めた＝このテストが残ると向こうは `shiori_host.rs` を消せない（この書き手が 156・236〜300 行を確かめた・`PropGetCueSink::new(reader, asker, kanade)` は `areka-ghost/src/prop_get_sink.rs:35`）｜ゴーストの道を `PropGetCueSink::new(runtime.sylphya_reader().clone(), ghost_asker_id(&runtime.mount().shiori.dir), tx)` で組み直し、`NAMES` の合図を 1 つ流して受信端の `KanadeMsg::PropertyEvent` の references を読み、MCP の答えと突き合わせる。値なしの表し方が `PropertyNotFound` から references の形へ変わるので期待を書き直す（R11）。直し方の手本は `audit-areka-app` が同じ形に直す `session_tests/ghost_session_scope_scale_boot_tests.rs:108-132`（今はまだ `ShioriHostSink` を使う形）。**順の縛り**: この直しは向こうの脳の削除より前か同時に着地する。間に合わなければ向こうは `shiori_host.rs` だけを残し、この spec の着地の後に消す。［議題 3］
103. **（app-2）** `install/startup.rs:217-222`（`before_boot_alert_suppressed`）｜起動の前の告知の抑止の決まり（`AREKA_NO_ALERT` の抑止か、`AREKA_APP_SMOKE_EXIT_MS` の有界の自動終了の走行）を、`areka-session/src/alert.rs:286` の `raise` の中の `suppressed || crate::smoke_exit::smoke_exit_ms().is_some()` と同じに写している。片方だけ直すと、起動の前の確かめの箱（`before_boot_ports.rs` の `ask_yes_no`・`warn_ok`）と起動の失敗の告知で抑止の決まりが食い違う。区画 S1 も、窓口と背景のスレッドの選ぶ画面（`install/desk.rs:177`・`install/worker.rs:192・243`）は `AREKA_NO_ALERT` しか見ないと気づきに書いた（この書き手が `startup.rs:210-236`・`alert.rs:270-290` を確かめた）｜決まりを `alert.rs` の公開の関数 1 つ（例: 「この走行で告知を抑えるか」＝`suppressed() || smoke_exit::smoke_exit_ms().is_some()`）に置き、`raise` とこの spec の起動の前の道がそれを呼ぶ。`before_boot_alert_suppressed` と `startup_flow_tests.rs:264-267` の表明は消し、決まりのテストは `alert.rs` の側へ移す。`alert.rs` は `audit-areka-app` の持ち物＝関数を足すのは向こうへ申し送り、この spec は向こうの関数ができてから写しを消す。選ぶ画面の 3 か所も同じ関数へ寄せるかは、自動終了の走行で選ぶ画面を出さなくなる（振る舞いが変わる）ので設計で確かめる（R12）。［議題 3］
104. **（app-3・設計の討議で足した）** `roots/edit.rs` の `decide_remove`・`decide_front`｜「書ける根の印の項目は外せない」「印があるとき実行ファイルの隣は先頭へ動かせない」の決まりが `menu/roots_frame.rs:73-75・109` にしかなく、判断の関数はどちらも受け付ける。今の呼び手はメニューだけだが、ほかの入口を足した日に黙って通る（区画 S6 の気づき・`audit-areka-app` は `menu/` に触らないと返した）｜同じ決まりを判断の関数に置き、断りの腕とテストを足す。メニューには触らない（直したほうがよい）。

**Verdict**: 本番の振る舞いを壊すのは 4 件（1〜4）。まず 1〜4、背景のスレッドの倒れ方（5・23）、取得の上限（12）、期限の無いテストの待ち（19〜21・26・37）を直す。
**Lean**: 見立てのある区画の和で約 −430 行（S3 約 −110・S4 約 −120・S5 約 −200）。S1・S2・S6 の見立ては無い（写しの寄せと要らないテストの削り＝44・75・80・84・98・100 ほかで減る）。依存の減りは 0。

### 数え直した数（前の版の 4 節の訂正）

範囲の 124 ファイルを grep で数え直した（`125116389`）。

| 項目 | 前の版 | 数え直した数 | 指摘 |
|---|---|---|---|
| 「要件 N.N」の形の番号 | 5 ファイル 23 か所 | 5 ファイル 23 行（`readme_tests.rs` 12・`roots/edit_tests.rs` 5・`readme/readme_cue_tests.rs` 4・`readme/readme_cue.rs` 1・`install/desk_record_tests.rs` 1） | 9・51・56 |
| 括弧や引用だけの番号・設計の番号 | 数えていない | 4 ファイル 29 行（`install/procedure_branch_tests.rs` 13・`install/procedure_tests.rs` 9・`install/terms_tests.rs` 6・`areka-session/src/roots/report.rs` 1） | 15・16・17・47 |
| 完了した spec の設計の札（M1〜M6） | 数えていない | 1 ファイル 12 行（`mcp/get_status_tests.rs`＝テストの名 6・doc 6） | 34 |
| 作業文書の検体の番号（実測 N・検体 N・`sample_N`・`SAMPLE_N`） | 数えていない | 1 ファイル 31 行（`mcp/get_expression_table_tests.rs`＝番号の文 15・名と定数 16・テストの名 8 本） | 29 |
| 合計 | 23 | 11 ファイル 95 行 | |
| `#[allow(...)]`（`cfg_attr` の中を含む） | 0 | 8（`install/worker.rs:58・60`・`areka-session/src/readme/destination.rs:223`・`readme/os_port.rs:55・59・65・110・178`） | 69・54・議題 11 |
| `shortcut:`・`ponytail:` の注釈 | 0 | 0 | — |
| `unsafe` の行 | 15 | 15（`roots/pick.rs` 6・`install/pick.rs` 4・`readme/os_port.rs` 3・`mcp/dump_surface_gpu_test_support.rs` 2・どれも `// SAFETY:` つき） | 55（文の中身） |
| 本番の `let _ =` | 4 | 4（`install/procedure.rs:142`・`install/worker.rs:248`・`mcp/dump_surface.rs:252`・同 426〔`#[cfg(test)]`〕） | 0 件（どれも記録が残るか doc の意図どおり＝S2・S4 が確かめた） |

### 区画の報告を突き合わせて直したこと

- **同じ所・同じ直しの重なり**（どれも番号は別に残し、互いを指した）: 件 3 の doc（8・71・81）と同じ種類（70・49）／期限の無い待ち（本番 1、テスト 19・20・21・26・37）／`boot_context` の写し（11・27・45・58）／ログの捕捉の包み（57）／背景のスレッドの倒れ方（5・23）／`check_size`（30・78）／選ぶ画面の写し（40・89）／`strict_records`（38 で移さず 84 で消す）／`readme_cue_tests.rs` の同じ行（52・56・98）／`roots_memory_write_failed` の 2 本（44・91）。
- **27（S3-5）と 45（S6-7）の食い違い**: S3-5 は「既にある補助はゴースト `A` の決め打ちで `update/` には使えない」、S6-7 は「上書きで違いだけ替えられる」と書いた。`reload_test_support.rs:17-41` を読むと両方とも正しい。上書きできるのは上の段の欄（`roots`・`roots_origin`・`app_profile_dir`・`helper_exe`）だけで、名を持つ `current` は丸ごと組み直しになる。よって 1 つの直し方にそろえた: **名がテストに効かない所（11・45・58）は既にある補助＋上の段の上書き、名が効く所（27）は範囲の中に名を引数に取る補助を 1 つ**。どれが「効かない」かは R3 で確かめる。1 か所へのまとめは議題 4。
- **区画が範囲の外への及びを書かなかったもの**: 31（`png_image.rs`・`png_image_tests.rs` は `audit-areka-app`）・94（`alert.rs`・`alert_tests.rs` は `audit-areka-app`）・25（読み手 `session_tests/shell_balloon_switch_session_update_tests.rs:243` は `audit-emo2-boot-switch`）。
- **区画が「範囲の外」と書いたが範囲の中のもの**（区画の外という意味だった）: `install/mod.rs` の `register`（2）・`install/desk_tests.rs:262・279`（69）・`install/judge.rs`（S2 の気づき）・`fetch_url.rs`（65）・`mcp/mod.rs:109`（83）。
- **完了した spec の決めとの突き合わせ**（区画は書いていない）: 2 は書く段に入らない形なら完了 `areka-P0-network-update` の要件 7.5 と食い違わない（そう書き足した）。13 は完了 `areka-P0-ghost-install` の設計の決め 3、68 は完了 `areka-P0-network-update` の要件 6.6、88 は完了 `areka-P0-mcp-reload` の要件 3、91 は完了 `areka-P0-write-failure-notice` の要件 5.5、40 は完了 `areka-P0-ghost-install` の設計（822 行）と `areka-P0-baseware-root-list` の設計（719 行）の記録の決めに当たる＝議題 8・9・10。
- **3 のテストの案の訂正**: S4-1 は `presenter_test_support::invalidate_ok` を使えると書くが、これは `areka-emo-present` の中の `pub(super)` で本体のテストからは届かない（本体の dev 依存も `areka-emo-present` の `test-support` を開けていない）。R7。
- **`audit-areka-app` から回ってきた 4 件**（同じウェーブの引き継ぎの書き置き）: 2 件は新しい指摘として足した（102・103）。1 件は 19 と同じ所（件 4）。残る 1 件は議題 1・2 の決めを向こうへ伝えること（9 節）。
- **細かい訂正**: `link_destinations` の `allow` は `destination.rs:223`（222 はコメント）。S1-7 に `install/overwrite.rs:7`（区画 S1 の気づき）を足した。

### 範囲の外への及びと、直す人（要件 7）

C26 の約束（roadmap の C26 の行）: ⑴ ほかの 2 本の持ち物は持ち主が直す ⑵ `Cargo.toml`・`Cargo.lock` は `audit-areka-app` だけ ⑶ `doc/COMPAT_ARCHITECTURE.md` §8 は既にある行だけ ⑷ `crates/dola/`・`wintf` に触らない。

| 指摘 | 及ぶ先 | 持ち主 | 扱い |
|---|---|---|---|
| 3 | `areka-emo-present/src/presenter/snapshot.rs`（公開の関数を 1 つ足す） | 見直しは完了（`audit-emo-present`）・C26 の 2 本の持ち物でない | この spec が直す（要件 7.6・依存は変わらない） |
| 60 | `areka-ghost/src/catalog.rs`（公開の関数を 1 つ足す） | 見直しは完了（`audit-ghost-catalog`） | この spec が直す |
| 13 | `areka-session/src/ghost_switch.rs` の `switch_to` | `audit-emo2-boot-switch` | 議題 8（直すか）・議題 3（順） |
| 33 | `areka-session/src/shell_balloon_switch_session_lap_test_support.rs` | `audit-emo2-boot-switch` | 申し送り・議題 3 |
| 25 | `session_tests/shell_balloon_switch_session_update_tests.rs:243`（読み手） | `audit-emo2-boot-switch` | R6 で数える側を確かめ、要るなら申し送り（議題 3） |
| 31 | 本体の `png_image.rs`・`png_image_tests.rs` | `audit-areka-app` | 申し送り・議題 3 |
| 85 | `areka-session` の `log_history` のテスト | `audit-areka-app` | 申し送り・議題 3 |
| 94 | `areka-session/src/alert.rs:150`・`alert_tests.rs:51` | `audit-areka-app` | 申し送り・議題 3 |
| 102 | 本体の `shiori_host.rs` ほか脳の 8 ファイル（向こうが消す・こちらは使うのをやめるだけ） | `audit-areka-app` | この spec が自分のテストを直す。向こうの削除と順を合わせる（議題 3） |
| 103 | `areka-session/src/alert.rs`（決まりの関数を 1 つ足す） | `audit-areka-app` | 申し送り・議題 3 |
| 25 | `doc/ukadoc-coverage/ledger/shiori.toml:7188` | C26 の 2 本の持ち物でない | この spec が直す（議題 10） |
| 39 | `doc/ssp-mcp/areka-tools.md:6` | 同上 | この spec が直す |
| 12・65・68 | `doc/COMPAT_ARCHITECTURE.md` 344 行（§8 の既にある行）・`doc/ukadoc-coverage/ledger/sakura-script.toml` 1416〜1418 行 | 約束 ⑶ の許す重なり | この spec が直す（行の追加は無い・議題 9・10） |
| 2 | `areka-session` の `exit_wait::register_gate`（使うだけ） | `audit-areka-app` | 触らない（公開の口を呼ぶだけ） |
| 54 | 本体の `cli/help.rs`（呼び手・読むだけ） | `audit-areka-app` | 触らない |

`Cargo.toml`・`Cargo.lock`・`crates/dola/`・`wintf` に及ぶ指摘は 0。

## 5. 預かった件 1〜8 の照合（要件 5・要件 1.5）

| 件 | 今の形 | 重なる指摘 | 範囲の外への及び |
|---|---|---|---|
| 1 `barrier()` | `install/desk.rs:381`。`record_installed` は UI スレッドで期限の無い `barrier()` で書き手を待つ | 1（テストの側の同じ種類は 37） | 無し（`areka-sylphya` は読むだけ） |
| 2 `UNKEPT_TARGETS` | `update_cue`・`readme_cue` に `info!` は無く、103 件の直しのどれも `info!` を足さない（S3・S6 が確かめた。51・52 は文だけ） | 無し | 無し（議題 2） |
| 3 呼び手の説明 | 3 か所とも「呼び手は `ghost_session::register_systems`」 | 8（`install/mod.rs`）・71（`update/mod.rs`）・81（`mcp/mod.rs`）。同じ種類で新しく出たのは 70（`install/names.rs`）・49（`readme.rs`） | 無し（議題 7） |
| 4 `let _ = join()` | `install/names_tests.rs:29`（`audit-areka-app` も同じ所を回してきた） | 19（同じ種類は 20・21・26・37） | 無し |
| 5 `capture_logs` の写し | `readme/readme_cue_tests.rs:24`。同じ種類の 2 つ目が `readme_tests.rs:47` に在る | 57 | 無し |
| 6 `boot_context` の写し | `install/desk_tests.rs:730`・`readme/opener_submit_tests.rs:31`。同じ種類が範囲の中にさらに 7 つ（`update/` 4・`roots/` 3） | 11・58（同じ種類は 27・45） | 無し（`reload_test_support.rs` は読むだけ・議題 4） |
| 7 `BODY_SYSTEMS` の 1 行 | `install/desk_tests.rs:385・554`・`update/desk_resolve_tests.rs:558` | 59（`install/` の 2 本）。`update/` の 1 本は S3 が確かめて指摘にしなかった（テストの前提を名指す行・壊れていない） | 無し（議題 5） |
| 8 `NO_BODY` の差 | 6 節 (1)。S1 が `install/desk.rs:141-147` の `raw_sender`、S2 が `install/install_cue.rs:154` の `url` の腕（窓口が無くても取りに行く）を確かめた。新しい指摘は無い | 無し（2・12 の直しは取得の形を変えるが、窓口が無いときに取りに行くことは変えない） | 直す本文は `audit-areka-app`（議題 1） |

## 6. OPEN QUESTION の事実

**(1) 議題 1: `NO_BODY` と本体の「窓口が無い」受け口**
- `NO_BODY.cue_sinks`（`areka-session/src/body_systems.rs:71`）は `DiscardSink` を 2 つ返す。`DiscardSink::emit` は何もしない。束が無いことは `BodySystems::of` の `error!(body_systems_missing)` で残る（`#[cfg(not(test))]`）。
- 本体の `body_cue_sinks`（`main.rs:424-432`）は `InstallCueSink::new(install::desk::raw_sender(world))` と `UpdateCueSink::new(update::desk::raw_sender(world))`。窓口が無いと `raw_sender` は受信端の無い送出端（`mpsc::channel().0`）を返す（S1 が `desk.rs:141-147` で確かめた）。
- 振る舞いの差: `path` の腕は送れずに `warn!(install_cue_send_failed)`（`NO_BODY` は黙って捨てる・束の無いことは error で残る）。`url` の腕は `fetch_for` が本物の取得を起こし、**ネットワークから書庫を落としてから** `warn!(install_fetch_send_failed)`（`NO_BODY` は取りに行かない・S2 が `install_cue.rs:154` で確かめた）。更新の受け口は送れずに `warn!`（`NO_BODY` は黙って捨てる）。
- 通る道: 窓口は `after_input`（どのゴーストを起こすより前）で据わり、終了の `discard_for_exit` も窓口を外さない。本番では両方とも通らない。通るのは結線の誤りとテストの足場（`SwitchRig::new`）だけ。
- 字面の檻: `runtime_tests/zorder_wiring_tests.rs:56`（`audit-areka-app`）が `body_cue_sinks` の 2 行を字面で照合している。
- doc の食い違い: `NO_BODY` の doc「何もしない・偽・捨てる受け口 2 つ（窓口が無いときの答えと同じ）」（`body_systems.rs:60`）は、`url` の腕と記録の有無で事実と違う。

**(2) 議題 8: 起動中のゴーストへの展開（13）**
- 完了 `areka-P0-ghost-install` の design.md 142 行・決め 3: 「起動中のゴーストへ入れる形は『間を空けない』。`switch_to` の全窓を閉じた直後・起こす前に … 1 回呼び、UI スレッドで同期に展開する」。理由は「新しい枝 0。窓もゴーストも無いフレームが回らない」。退けた案は「間を空けて背景で展開する（`switch_to` を前後に割る・切替の段が増える・窓の無い間の OS の終了の受け手が要る）」。同 316 行は「展開は UI スレッドで同期に走るので、後始末が始まる時点では必ず終わっている」。
- 展開の間に止まるもの: 窓は全部閉じているので、見える物は固まらない。止まるのは UI スレッドのメッセージの汲み上げ（OS のシャットダウンの問い合わせ・タイマー・タスクトレイ）。展開の長さは書庫の大きさしだい（記録 `install_overwrite_done` の `ms`）。
- この spec の要件 6.4（UI スレッドを書き込みで塞がない）と、記憶の決め「台本の大きさを甘く見ない・上限で断らず UI スレッドの外へ出す」は、背景へ出す向きを推す。
- 及ぶ先: `areka-session/src/ghost_switch.rs:606` の `switch_to`（`audit-emo2-boot-switch`）。降ろした後に「展開の答えを待つ段」を足し、答えが来たら起こす。終了がその段で始まったら、展開を書く段として門で上限まで待つ（今の `exit_wait` の意味のまま）。

**(3) 議題 9: 落とした書庫の後片付け（68）**
- 完了 `areka-P0-network-update` の requirements.md 257 行・要件 6.6: 「落としたファイルを … 置き、同じ一時フォルダにある 7 日より古いファイルを次の取得のときに消す（その場では消さない＝手続きが読み終える時点を知る口が無い）」。
- 今のコード: 背景のスレッド `install` の `spawn_worker`（`install/worker.rs:71-86`）は `run_order(&order, …)` が戻った時点（＝手続きが書庫を読み終えた時点）を知っていて、`order` には書庫のパスと出どころ（`Script`）が在る。要件 6.6 の理由は今は成り立たない。
- 7 日の掃除は、終了の途中の落としかけ（要件 7.5）と、消せなかった物のために残る。

**(4) 2 と完了 `areka-P0-network-update` の要件 7.5**
- 要件 7.5（requirements.md 270 行）: 「終了の後始末に入る When, the areka shall `\![execute,install,url]` の取得の途中の物を待たずに終え、落としかけのファイルは要件 6.6 の掃除に任せる」。
- `exit_wait::begin_close`（`exit_wait.rs:124-152`）は、書く段（`writing`）の門だけを待つ。名（`label`）だけを持つ門は待たずに `warn!(exit_wait_abandoned, name, label)` を 1 件残す。よって 2 の直しは `begin`／`end` だけを使えば要件 7.5 のまま記録を残せる。1 つの門が持つ名は 1 つ（`label: Option<String>`）。

**(5) 議題 10: 外への約束を変える直しの読み手**（範囲の外の読み手を grep で探した）
- `install_abandoned`: `.rs` の外に読み手は無い（完了 spec の設計の表 927 行だけ）。
- `install_fetch_swept`: `doc/ukadoc-coverage/ledger/sakura-script.toml:1416`（「消した古いファイルの数は info!（install_fetch_swept）」）。
- `update_reload_requested`: `doc/ukadoc-coverage/ledger/shiori.toml:7188`・本体の `session_tests/shell_balloon_switch_session_update_tests.rs:243`（`audit-emo2-boot-switch`・`capture` は窓の中で起こしたアクターの記録も取る＝どちらの側を数えているかは R6）。
- `install_pick_suppressed`（warn）・`roots_pick_cancelled`（info・`reason=no_alert`）: `.rs` の外に読み手は無い。決めは完了 `areka-P0-ghost-install` の design.md 822 行と `areka-P0-baseware-root-list` の design.md 719 行。`log_history.rs` の `classify` は warn 以上を出所によらず「error」に数える＝今は告知の抑止（利用者が `AREKA_NO_ALERT` で決めたこと）が記録の一覧で error に並ぶ。
- `roots_memory_write_failed`: `.rs` の外に読み手は無い。完了 `areka-P0-write-failure-notice` の要件 5.5（requirements.md 160 行）が「事実と合っている物（…`roots_memory_write_failed`…）の名前を変えない」と決めた。
- MCP の `reload` の断りの文: 完了 `areka-P0-mcp-reload` の requirements.md 102 行が字で決めた。`install_busy` で断る道は後から足された（`areka-session/src/reload.rs:314`）。SSP の実測の文ではない。
- `\![execute,install,url]` の URL の形: 正典の文は「ファイルのダウンロードを一時ファイル用フォルダ内に自動的に行い、適宜インストール処理を行う」（`doc/COMPAT_ARCHITECTURE.md:344` の引用）で、URL のスキームの決まりに触れない。`http://`／`https://` の先頭の判定は §8 の行が「areka 裁量」と書く areka の決まり（完了 `areka-P0-network-update` の要件 6.3）。

**(6) 議題 11: `link_destinations` の印**
- `areka-session/src/readme/destination.rs:221-224`: `// 本番の呼び手はまだ無い（後続の機能が足す）＝それまでテストだけが呼ぶ。` と `#[cfg_attr(not(test), allow(dead_code))]`。
- 追う spec: `link-context-copy` の brief 75 行と `balloon-link-hover` の brief 72 行が「本番の呼び手になったら外す」と書く（どちらも未着手・`completed/` でない）。両 brief とも古いパス `crates/areka/src/readme/destination.rs` を書く（今は `crates/areka-session/src/readme/destination.rs`）。
- 指摘のうち、この印を消す直しは 0 件（101 はテストの名だけ）。かけ直しの audit が「要らないコード」として挙げうる。

**(7) 範囲の外で見つけた件**（指摘に数えない・要件 7.10 で持ち主の brief へ）
- `areka-session/src/ghost_switch.rs` の切替の入口は `update_busy` を見ない（`shell_balloon_switch.rs:132`・`reload.rs:311` は見る＝この書き手が grep で確かめた）。更新のエンジンが今のゴーストのフォルダを入れ替えている最中に、同じゴーストへの切替（台本の `\![change,ghost,<今のゴースト>]` など）が受け付けられると、SHIORI が書きかけのフォルダから起き直り得る。ただし更新自身の読み直しも同じ入口を通るので、見る形には更新の頼みを除く決まりが要る。持ち主 `audit-emo2-boot-switch`。
- `areka-session/src/log_history.rs:335` の `snapshot` は、種別の記録を最大 1,000 件（本文は 1 件 4,096 字まで）、プロセスで 1 つの排他を握ったまま写す（`get_log` は写した後で絞る）。持ち主 `audit-areka-app`。
- 「書ける根の印の項目は外せない」「印があるとき実行ファイルの隣は先頭へ動かせない」の決まりは `menu/roots_frame.rs:73-75・109` にしかなく、判断の `decide_remove`・`decide_front` はどちらも受け付ける（今の呼び手はメニューだけ）。持ち主 `audit-areka-app`。
- `areka-runtime/src/consumer_ledger.rs:77` の「本体の `readme::readme_cue::ReadmeCueSink`」は古い（今は `areka_session::readme::readme_cue`）。持ち主は設計で確かめる（C26 の `audit-emo2-boot-switch` の `areka-runtime` の受け持ちに入るか）。

## 7. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今ある物 | 足りない物・制約 |
|---|---|---|
| 1 洗い出し | この報告（103 件・124 ファイルの全行・数え直しと行番号） | 無し |
| 2 確定 | 4 節の 103 件・9 節の議題 | **Constraint**: 要件 2.2 の 1 種類目＝議題 8・9・10 の (c)(d)(f)(i)、2 種類目＝議題 11。3 種類目に当たるものは無い（10 の (h) は areka 裁量の行の中） |
| 3 直し切る | 既にある共通の道具（`join_bounded`・`join_thread_bounded`・`wait_recv`・`capture_lines`・`reload::test_support`・`barrier_within`・`exit_wait::register_gate`・`TempPath`） | **Missing**: 27・74 の `update/` の支えのファイル、38 の偽の kanade の支え、99 の `opener_test_support` の補助、3 の `compose_last_shown`、60 の `areka-ghost` の関数 |
| 4 テスト | 偽の取得の口（`StartFetch`・`MakeFetch`）・偽の SHIORI の土台・`target` の下の根（`TempPath`）・GPU の足場（`mcp/dump_*_gpu_test_support.rs`） | **Missing**: 1・2・3・5・7・12・22・23・24・30・36・50・63・64・65・68・96・97 のテスト。**Unknown**: R2（1 の期限切れ）・R7（3 のテストの置き場）・R10（panic の偽物） |
| 5 預かった件 | 5 節 | 件 2・8 は議題で閉じる。件 8 の本文は持ち主が違う |
| 6 決まり | 1,000 行の番人・`error!`＋`Err`・UI の外の重い仕事の形 | **Constraint**: 依存を足せない（C26 ⑵・3・60 は依存を変えない）。`update/worker_tests.rs`（967）・`install/startup_tests.rs`（854）へは足せない＝新しいファイル。4・28 の新しいスレッドの終わり方（R8） |
| 7 並走 | 4 節の終わりの表 | **Constraint**: 13・25・31・33・85・94・103 は C26 のほかの 2 本の持ち物に及ぶ。102 は向こうの削除と順を合わせる（議題 3） |
| 8 副作用 | 決定論の偽の口がそろっている | 実機・外のホストは使わない。3・36 は実 GPU のテスト（`GpuPermit` の既にある作法） |
| 9 かけ直し | 前例の作法（`completed/audit-areka-input-menu/research.md` 12 節） | 議題 11 で残すと決めた印は数えない（要件 2.5） |

## 8. 進め方の案

- **案 A（既にある物へ寄せる・推す）**: 預かった件と写しは既にある共通の道具に置き換え、本番の指摘は持ち主のファイルの中で「いちばん小さな完全な変更」で直す。新しいファイルは、テストを分ける所（1,000 行の番人）と、範囲の中の支え（27・38・99）だけ。範囲の外は 3・60 の公開の関数 1 つずつと、文書の既にある行だけ。C26 のほかの 2 本の持ち物は申し送る。利点: 並走の約束に収まる・差分が小さい。弱み: 申し送った分（31・33・85・94 と議題 4 のまとめ）は相手の着地しだいで残る。
- **案 B（共通の足場を範囲の中に新しく作る）**: `boot_context`・`BODY_SYSTEMS` の据え・ログの捕捉・偽の kanade を範囲の中の 1 つの `_test_support.rs` へ集める。弱み: crate が 2 つ（本体と `areka-session`）なので 1 つにならず、既にある `reload::test_support` と二重になる。
- **案 C（相手の持ち物も自分で直す）**: 31・33・85・94 と 13 の `switch_to` をこの spec が直す。弱み: C26 の約束 ⑴ を破る。

推す＝案 A（議題 3 の順で、相手が先に着地したものは自分で直す）。

**規模と危うさ**: 規模 L（1〜2 週）。指摘 103 件（前の版の見込み 80〜130 件の中）。危うさ 中: 大半は既にある型に沿う直しだが、UI スレッドの待ち（1・6・28・4）、背景のスレッドの倒れ方（5・23）、終了の道（2・12）に触り、議題 8 を直すなら切替の進め方（`switch_to`）に触る。

**タスク数の見込み**: 小タスク 22〜26（brief の 18〜28 の中）。内訳: 件 1 と待ちの上限（1・19〜21・26・37）2／背景のスレッドの倒れ方（5・23）1／取得（2・12・22・65〜68）2／インストールの UI の重い仕事と手続き（6・7・13・14・60・61・63・64・103）2〜3（13 を直すなら＋1）／更新（24・25・72〜77）1〜2／MCP の本番（3・28・30・35・36・78・82・83・88）2／MCP のテストの写しと削り（31〜33・38・79・80・84・86・87・102）1／根（4・40〜46・48・89〜96）2／説明書（49〜55・97）1／テストの写し（11・27・45・57・58・93・99）2／要らないテストの削り（44・75・98・100・85）1／コメントの決まりと番号（8〜10・15〜17・29・34・39・47・56・62・70・71・81・101）1〜2／預かった件の記録と申し送り 1／かけ直しと差分の確かめ 2。

## 9. 要件の討議への持ち越し

### 議題（答えで作業が変わるもの）

**2026-11-22 の要件の討議で、議題 1〜11 を全部推奨案で確定した**（確定の中身は `requirements.md` の「確定した指摘の一覧」と要件 5・7）。

1. **`NO_BODY` と本体の「窓口が無い」受け口の差**（要件 5.8・件 8）
   - 推す＝**そろえない**。`NO_BODY` の doc の「（窓口が無いときの答えと同じ）」を「本体の窓口の無いときとは `url` の腕が違う（本体は取りに行ってから送れずに warn・こちらは取りに行かない）」に直すか括弧を消すことを、持ち主の `audit-areka-app` へ申し送る（`session_tests/body_systems_tests.rs` は今のまま）。`audit-areka-app` は、この決めを伝言で受け取れば `areka-session/src/body_systems.rs`・本体の `session_tests/body_systems_tests.rs`・`reload_test_support.rs` を自分で直すと申し出ている＝決まったら向こうのセッションへ決めを伝えるだけで、この spec はそれらのファイルに触らない。
   - 理由: ⑴ 両方とも本番の道では通らない ⑵ 両方とも記録が残る（`body_systems_missing` の error／`install_cue_send_failed`・`install_fetch_send_failed` の warn）＝黙った失敗ではない ⑶ `NO_BODY` を本体へ寄せるのは依存の向きでできない ⑷ 本体を `NO_BODY` へ寄せると `raw_sender` の形と `main.rs` の `body_cue_sinks`・字面の檻を変えるが、本番の振る舞いは 1 つも変わらない。audit も、この差に新しい指摘を出さなかった（S1・S2 が確かめた）。
2. **`update_cue`・`readme_cue` に `info!` を足すときの記録**（要件 5.2・件 2）
   - 推す＝**仮置きのまま閉じる**。103 件の直しのどれも 2 つの受け口に `info!` を足さないので、`UNKEPT_TARGETS` へ足す申し送りは 0 件。この結果を記録に残し（要件 5.9）、`audit-areka-app` へ「`log_history.rs` の `UNKEPT_TARGETS` に足す物は無い」と伝える（向こうは決めを受けて `UNKEPT_TARGETS` を直すと申し出ている）。
   - 理由: 決めることが起きていない。後で足す直しが出たら、そのときに仮置きの形（`UNKEPT_TARGETS` へ足すことを `audit-areka-app` へ申し送る）で扱う。
3. **C26 のほかの 2 本の持ち物に及ぶ直しの順**（要件 7.2）
   - 当たる所: 13（`ghost_switch.rs` の `switch_to`・`audit-emo2-boot-switch`・議題 8 で直すとき）・33（`shell_balloon_switch_session_lap_test_support.rs`・同）・25 の読み手（`session_tests/shell_balloon_switch_session_update_tests.rs:243`・同・R6 で要るときだけ）・31（`png_image.rs`・`png_image_tests.rs`・`audit-areka-app`）・85（`log_history` のテスト・同）・94（`alert.rs:150`・`alert_tests.rs:51`・同）・103（`alert.rs` に抑止の決まりの関数を 1 つ・同）・議題 4 の補助の引数（`reload_test_support.rs`・同）・議題 1 の doc（`body_systems.rs`・同）。逆向きの縛りが 1 つ: 102（この spec のテスト）は、`audit-areka-app` が脳の 8 ファイルを消す前か同時に着地する。
   - 102 の推す答え＝**この spec が引き受けて直す**（向こうへ「受ける」と返す）。向こうの削除と同じ時期に着地できなければ、向こうは `shiori_host.rs` だけを残し、この spec の着地の後に消す（向こうが申し出た形）。どちらの順でも直す中身は同じなので、新しい議題にはしない。
   - 推す＝**相手が走っているあいだは触らずに申し送り、相手が `completed/` へ移った後なら `main` を取り込んでこの spec が直す**（仮置きのまま）。この spec の実装の終わりまでに相手が着地せず、相手も直さなかった分は、写しや古い形を残したまま理由を記録し、相手の brief へ件として書き足す（要件 7.9）。13 の `switch_to` の側は相手のファイルの進め方を変える大きい直しなので、相手のセッションと先に申し合わせる（どちらが入れるか・どちらが先に着地するか）。
   - 理由: C26 の約束 ⑴ を守りつつ、申し送りの宛先が無くなった件を持ち主の無い宿題にしない。`audit-emo2-boot-switch`（15〜24 タスク）はこの spec（22〜26）より先に着地する見込みが高い。
4. **`boot_context` の写しを 1 か所へまとめる仕事の持ち主**（要件 5.6・件 6）
   - 事実: 範囲の中の写しは件 6 の 2 つではなく **9 つ**（`install/desk_tests.rs` 1・`readme/opener_submit_tests.rs` 1・`roots/` 3・`update/` 4）。既にある `reload::test_support::boot_context` は名が `A`／`kaku` の決め打ちで、上書きできるのは上の段の欄だけ（4 節の終わり）。
   - 推す＝⑴ **名がテストに効かない 5 つ（11・45・58）は既にある補助＋上の段の上書きで消す**（R3 で確かめる・効けば ⑵ へ） ⑵ **名が効く `update/` の 4 つ（27）は `update/` の支えのファイルに名を引数に取る補助を 1 つ置いて寄せる** ⑶ **1 か所へのまとめ（既にある補助に名の引数を足し、⑵ の補助も消す）は、置き場の持ち主の `audit-areka-app` へ今申し送る**。向こうが C26 のうちに足せば、この spec は ⑵ を作らずにそれを使う。足さなければ ⑵ を残し、3 本の着地の後に残る写し（範囲の外の `menu/ghost_frame_tests.rs` ほか）と一緒に次の棚卸（`/kiro-next`）で拾う。要件 5.6 の文は「記録に残すまで」から「範囲の中の写しを 0 か 1 にする」へ書き換える。
   - 理由: 寄せ先の持ち主が自分のファイルに引数を足すだけなら C26 の重なりは増えない。この spec の側で範囲の外の補助を待たずに写しを 9 から 0〜1 へ減らせる。
5. **`BODY_SYSTEMS` の 1 行を足した 3 本のテスト**（要件 5.7・件 7）
   - 推す＝**寄せない（3 本の 1 行を残し、理由を記録に残す）**。59（S1-9）も同じ直しを書き、S3 は `update/` の 1 本を指摘にしなかった。
   - 理由: 1 行は「本体の口を通る道を試す」という前提を名指す明示の 1 行で、寄せ先（`Rig::new`・`world()`）へ入れると、同じ足場を使うほかのテスト（口が無いときの倒れ方・窓口の有無を見るもの）の前提も黙って変わる。減るのは 1 か所あたり 1 行。
6. **20 タスクを越える見込みのときに 2 本目へ切るか**（要件 2.6）
   - 推す＝**切らない（仮置きのまま）**。見込みは 22〜26（8 節）。
   - 理由: 後の開発者の決め（20 タスクは discovery の分け方の目安で spec の上限ではない・一度切り出した後は規模を理由に分けない）が brief の文より後。
7. **`mcp/mod.rs:48` の説明を、ファイルの持ち主のこの spec が直すこと**（要件 5.3・件 3）
   - 推す＝**仮置きのまま（この spec が 8・71・81 を同じ直しで直し、同じ種類の 70・49 も直して、`audit-areka-app` のセッションへ直したことを知らせる）**。文は「`BODY_SYSTEMS` の `after_input` から」の形。
   - 理由: ファイルの持ち主はこの spec。向こうの brief の件 2 が同じ行を挙げているので、二重に直さないよう知らせる。
8. **起動中のゴーストへの展開を UI スレッドの外へ出すか（13）**（要件 2.2 の 1 種類目＝完了 `areka-P0-ghost-install` の設計の決め 3 を改める）
   - 推す＝**出す（決め 3 を改める）**。展開を背景のスレッドへ戻し、`switch_to` を「降ろした」と「起こす」の間で展開の答えを待つ段に分ける。終了がその段で始まったら、展開を書く段として門で上限まで待つ（今の `exit_wait` の意味）。`switch_to` の側の変更は議題 3 の順で入れる（R5）。
   - 理由: ⑴ この spec の要件 6.4（UI スレッドを書き込みで塞がない）と、台本の大きさを甘く見ずに重い仕事を UI スレッドの外へ出す決めに合う ⑵ 書庫の上限は 256 MB で、展開の長さは書庫しだい＝OS の終了の問い合わせに答えられない時間に上限が無い ⑶ 決め 3 が退けた理由（段が増える・窓の無い間の終了の受け手が要る）は、今の門（書く段なら上限まで待つ）で受けられる。
   - 別案: 残す（裁定で残し、要件 9 の数から外す）。窓は全部閉じているので見える物は固まらず、`switch_to` と相手の spec に触らずに済む。直す量は 13 だけで済むが、UI スレッドの待ちに上限の無い所が 1 つ残る。
9. **URL から落としてインストールし終えた書庫をその場で消すか（68）**（要件 2.2 の 1 種類目＝完了 `areka-P0-network-update` の要件 6.6 を改める）
   - 推す＝**消す（要件 6.6 の「その場では消さない」を改める）**。`spawn_worker` で `run_order` を終えた後、出どころが `Script` で `download_dir()` の下にある書庫を消す。7 日の掃除は落としかけと消せなかった物のために残す。`doc/COMPAT_ARCHITECTURE.md:344`（§8 の既にある行）と `ledger/sakura-script.toml:1416-1418` の文を同じ変更で直す。
   - 理由: 要件 6.6 の理由（手続きが読み終える時点を知る口が無い）は今は成り立たない（6 節 (3)）。消さないと 1 本最大 256 MB が使わない限り残り続け、doc の「7 日」も守られない。
10. **外への約束（ログの語・MCP の答えの文・タグの受け方）を変える直しの一括の裁定**（要件 7.7・7.8。(c)(d)(f)(i) は要件 2.2 の 1 種類目も兼ねる）
    - 推す＝下の表のとおり。どれも読み手（6 節 (5)）を同じ変更で直す。
    - | 記号 | 指摘 | 変わる所 | 推す答え | 理由 |
      |---|---|---|---|---|
      | (a) | 14 | 書いた後に締めの知らせが閉じたとき、`warn!(install_abandoned)` が出なくなる（終わり方は `Installed`） | 変える | 入れ終えた物を「やめた」と記録するのは事実と違う。`.rs` の外に読み手は無い |
      | (b) | 66 | `info!(install_fetch_swept)` を消した数が 0 のときは出さない | 変える | 毎回「消しました count=0」が並ぶ。台帳の文（「消した古いファイルの数は info」）はそのまま正しい |
      | (c) | 25 | 背景のスレッドの側の語 `update_reload_requested` → `update_reload_asked` | 変える | 1 回の読み直しで同じ語が 2 つの事実を指す。台帳 `shiori.toml:7188` を直す。相手の持ち物のテストは R6 |
      | (d) | 40 | 告知の抑止の記録の水準と形をそろえる | **info にそろえ、両方とも `reason = "no_alert"` の欄を持つ形にする。語（`install_pick_suppressed`・`roots_pick_cancelled`）は変えない** | 抑止は利用者が決めたことで、warn だと記録の一覧で error に並ぶ。語を変えなければ読み手の照合は変わらない |
      | (e) | 42 | `info!(roots_removed, path)` を足す | 足す（欄 `removed` は残す） | 先頭へ動かすときの `roots_moved_to_front` とそろい、何を外したかが記録から読める。新しい語なので読み手は 0 |
      | (f) | 88 | MCP の `reload` の断りの文に `install` を足す | 変える（完了 `areka-P0-mcp-reload` の要件 3 の字を改める） | インストールで断るのに更新と読み違える。SSP の実測の文ではない（areka の文） |
      | (g) | 12 | 取得を長く生きるスレッド 1 本で順に落とす（終了で積んだ残りの URL も記録に残る） | 変える | ゴーストから見た振る舞い（全部の URL がいずれインストールされる）は変わらない。§8 の行の「短命のスレッド」の文を直す |
      | (h) | 65 | 大文字のスキーム（`HTTPS://`）の URL も受ける | 変える | 正典はスキームの決まりに触れず、先頭の判定は §8 の行が「areka 裁量」と書く areka の決まり＝SSP 互換・正典に関わらない（3 種類目に当たらない）。受ける側へ広げるだけで、今受けている URL は変わらない |
      | (i) | 91 | `error!(roots_memory_write_failed)` を消して `fatal` の 1 行にそろえる | **残す（裁定で残し、要件 9 の数から外す）** | 完了 `areka-P0-write-failure-notice` の要件 5.5 が「名前を変えない」と決めた。2 行は致命の道でだけ出る（めったに出ない）。消さずにそろえる形は無い（`fatal` の行は共通の口が出す） |
11. **`link_destinations` の先送りの印**（要件 2.2 の 2 種類目）
    - 推す＝**残す（わざと置いた先送りの印として裁定し、要件 9 のかけ直しの数から外す）**。あわせて、追う 2 本の brief（`link-context-copy` 75 行・`balloon-link-hover` 72 行）の古いパスを今のパス（`crates/areka-session/src/readme/destination.rs`）へ直す件を、2 本の brief へ書き足す（要件 7.10）。
    - 理由: 印には実在の理由（後続の機能が本番の呼び手になる）が書かれ、追う spec が 2 本生きている。どの指摘もこの印を消さない（6 節 (6)）が、かけ直しの audit が要らないコードとして挙げうるので、先に決めておくとかけ直しが閉じる。

### 議題にしなかった候補（確かめて、答えで作業が変わらないと分かったもの）

- **12 の直し方**（長く生きるスレッド 1 本に積むか、取得中の 2 本目を断るか）: 断る形はゴーストから見た振る舞い（2 本目のインストールが起きない）を変えるので、外への約束を変えない積む形を取る（要件 2.4）。積む形は 2 の門（1 つの門に名は 1 つ）とも合う。§8 の行の文の変わりは議題 10 の (g) に入れた。
- **2 と完了 `areka-P0-network-update` の要件 7.5**: 書く段に入らない門なら「待たずに終える」のまま記録を残せる（6 節 (4)）。改める決めは無い。
- **3 の `areka-emo-present` への関数の追加**・**60 の `areka-ghost` への関数の追加**: どちらも見直しは完了し、C26 のほかの 2 本の持ち物でない。要件 7.6（直しのために要る範囲の外の変更）に収まり、依存は変わらない。範囲の中だけで済ませる形は、3 は最後に見せた絵と一致せず、60 は同じファイルを 2 回読むのが残る。
- **33 の相手のファイルへの及び**: 議題 3 の順に入れた（新しい決めは要らない）。
- **`install/judge.rs` が平文の `http://` の URL も受けること**（S2 の気づき・指摘ではない）: 書庫には SHIORI の DLL が入り得るので途中で差し替えられうるが、正典は URL の形に触れず、完了 `areka-P0-network-update` の要件 6.1・6.3 が `http://` を受けると決め、ネットワーク更新の `homeurl` も `http://` を受ける（同 要件 11）。断ると SSP で動くゴーストの URL が areka で動かなくなる。指摘でもないので直さない。
- **`ghost_switch.rs` が `update_busy` を見ないこと**（S3 の気づき）: 範囲の外の件として持ち主 `audit-emo2-boot-switch` の brief へ書き足し、セッションへも知らせる（要件 7.10・6 節 (7)）。この spec の作業は変わらない。
- **54・69 の `#[allow]`**: 書かれた理由は「テストのビルドに呼び手が無い」「試験だけが読む」で、先送りの印ではない（要件 2.2 の 2 種類目に当たらない）。そのまま直す。
- **S2-3 の別の小さな形**（展開を UI スレッドのまま、メッセージを汲みながら待つ）: 入れ子のメッセージの巡回になり、終了の道を正規の実装にする決まり（要件 6.3）に反するので案にしない。

### Research Needed（設計で確かめる）

- **R1（済み）**: 範囲の 124 ファイルを 6 つの区画で全行読めた（3 節）。
- **R2**: 1 の期限の値と、期限切れを決定論で起こす方法。`areka-sylphya` の檻は止めの偽の IO（`actor_barrier_within_tests.rs` の `GatedFirstCommit`・私有）を使う。本体のテストから今のゴーストの記憶の書き手へ偽の IO を差せる公開の口が在るか（無ければ、`record_installed` の待ちを関数へ切り出し、`barrier_within` の結果を受ける側だけを試す）。期限の値は UI を止めてよい長さとして既にある値（`exit_wait::EXIT_WAIT_LIMIT` の 3 秒ほか）に合わせるか。37 のテストの期限は `areka_runtime::spine::SPIN_WAIT` にそろえるか（前例 `audit-areka-input-menu`）。
- **R3**: 11・45・58 で `reload::test_support::boot_context` に置き換えたとき、ゴーストの名（`A` ← `ga`／`g`）・バルーンの名（`kaku` ← `B`／`b`）・プロファイルの置き場（`temp` の下 ← `root` の下）・`GhostRoute::Memory` → `Default` の違いが、`install/desk_tests.rs` の 7 本・`roots/` の 3 つの足場を使うテスト・`readme/opener_submit_tests.rs` の 1 本の結果に効かないか。
- **R4（済み）**: 取得のスレッドを終了の門に乗せる直しは、`areka-session` の `exit_wait`（`audit-areka-app`）に手を入れずにできる（`register_gate` は公開・書く段に入らない門は待たずに名を記録に残す＝6 節 (4)）。条件は取得が同時に 1 本であること（12）。
- **R5**（議題 8 で直すとき）: `switch_to` を「降ろした」と「起こす」の間で分けたとき、⑴ kanade の保留（再生中の台詞の終わりを待つ）と段の並び ⑵ 窓もゴーストも無いフレームで回るシステム（入力・表示・MCP の受け口）が置き場の空を受けられるか ⑶ 終了がその段で始まったときの門の扱い ⑷ 展開の失敗のときの「既定へ戻す・既定なら致命」の道が今のまま通るか ⑸ 変わる行が `ghost_switch.rs`（982 行に近い `ghost_switch_tests.rs` を含む）のどこか。
- **R6**: 25 の改名が、本体の `session_tests/shell_balloon_switch_session_update_tests.rs:243`（`audit-emo2-boot-switch` の持ち物・期待 1 件）の数えに効くか（`capture` は窓の中で起こしたアクターの記録も取る＝背景のスレッドの側を数えていれば改名で 0 件になり、相手のファイルの直しが要る）。
- **R7**: 3 のテストの置き場。`presenter_test_support::invalidate_ok` は `areka-emo-present` の中の `pub(super)` なので、`compose_last_shown` の単体の確かめは `areka-emo-present` の中に置く。本体の側（`dump_balloon_gpu_tests.rs` ほか）で隠した target の控えを捨てた状態を作るには、公開の口（`PresentCommand::InvalidateCache` ほか）で届くか。届かなければ本体の側は答えの腕の分かれだけを試す。
- **R8**: 4・28 で足す短命のスレッドの終わり方。4 は記録だけを持つ、28 は答えの送出端だけを持つ（待つ側が去れば落ちる）。終了の門に乗せずに済むこと（失うのは記録の行と答えだけ）を設計に書き、テストの足場（`wait_answer`）で待てるか。
- **R9**: 54 で `allow` を外した後に、`cargo clippy -p areka-session --tests` で警告が出ないか（区画 S6 はコンパイルで確かめていない）。
- **R10**: 5・23 の panic のテスト。偽物の中で panic させたとき、テストの中で human-panic のフックや標準エラーへの出力がテストの結果を揺らさないか。
- **R11**: 102 で `PropGetCueSink` を通したとき、値なし（今の `PropertyNotFound`）と空の値（`test.empty`）が `KanadeMsg::PropertyEvent` の references でどう表れるか（区別できるか）。合図の組み立ては `dola` の試験用の口（`areka-runtime` の `consumer_ledger_test_support::carrier_cue` ほか）で足りるか（`dola` には触らない）。
- **R12**: 103 で選ぶ画面の 3 か所（`install/desk.rs:177`・`install/worker.rs:192・243`）も同じ決まりへ寄せるか。寄せると自動終了の走行（`AREKA_APP_SMOKE_EXIT_MS`）でインストールの選ぶ画面が出なくなる＝今の使われ方（実機の確かめ・`tools/`）でその画面を出す走行があるかを grep で確かめ、あれば寄せない（区画 S1 は困る場面を見つけていない）。

### 申し送りの見込み（要件 7.10）

- **`audit-areka-app` へ**: 議題 1 の決め（向こうが `body_systems.rs`・`body_systems_tests.rs`・`reload_test_support.rs` を直す）と議題 2 の結果（`UNKEPT_TARGETS` に足す物は無い）／102 を受けること（脳の 8 ファイルの削除と順を合わせる）／103 の抑止の決まりの関数を `alert.rs` に足すこと（足せばこの spec は `before_boot_alert_suppressed` を消す）／回してきた `install/names_tests.rs:29` は 19 で直すこと／議題 4 の既にある補助に名の引数を足すこと（足せばこの spec はそれを使う）／31 の `png_image.rs`・`png_image_tests.rs` の `wic_decode_rgba` の共有／85 の `sakurascript_log_tests.rs` の中身を `log_history` のテストへ／94 の `alert.rs:150`・`alert_tests.rs:51` の読み手の形／同 brief の「サ」（`readme/readme_cue_tests.rs` の `capture_logs`）・「シ」（`install/desk_tests.rs`・`readme/opener_submit_tests.rs` の `boot_context`）・件 2（`mcp/mod.rs:48`）はこの spec が直すこと／範囲の外の件 2 つ（`log_history.rs:335` の `snapshot` の排他・`menu/roots_frame.rs` にしかない決まり）。
- **`audit-emo2-boot-switch` へ**: 議題 8 で直すなら 13 の `switch_to` の段の分け方の申し合わせ／33 の `spawn_windows` に `dpi` を取らせ `pump_messages` と一緒に `pub` にすること／R6 で要るときの 25 の読み手の直し／`ghost_switch.rs` が `update_busy` を見ない件（範囲の外の件）／`reload::test_support::boot_context` が写しの寄せ先に使えること（`ghost_switch_tests.rs` の写しの寄せ先の選びに）。
- **`link-context-copy`・`balloon-link-hover` の brief へ**: 議題 11 の古いパスの直し。
- **持ち主を設計で確かめる**: `areka-runtime/src/consumer_ledger.rs:77` の古い説明。

## 10. 設計の段の調べ（2026-11-22・kiro-spec-design）

- **やり方**: 軽い調べ（既にある crate の直し・新しい依存なし）。外の資料は読んでいない。範囲の外のコードは、読むだけの下請け 3 本と、この書き手が読んだ。下請けは次の 3 本。
  - R2・R10・6・60・69。
  - R3・R6・R12・93・99。
  - R7・R8・R11・2・12・68。
  - この書き手は 13 の `switch_to`・`overwrite.rs`・R5 を読んだ。

  テストもコンパイルも回していない。
- **使った作法**: kiro-spec-design の rules（discovery-light・synthesis・review-gate・principles）、ponytail（いちばん小さな完全な変更）、CLAUDE.md の「## Comments」。
- **兄弟からの知らせ**（`audit-emo2-boot-switch` の設計の承認 `6a0823440`）:
  - 13 の `switch_to` の分け方は、この spec が入れる。向こうは `switch_to`・`take_down`・`boot_into` に触らず、`ghost_switch.rs` の 40〜44・66〜90・111・250〜333・423・522 行だけを直す。先に着地した側の後で、もう片方が `main` を取り込む。
  - 33: 向こうが `spawn_windows(rig, dpi: u16)` と `pump_messages` を `pub` にする。
  - 向こうは `SwitchVerdict::Updating` を足す。`install/overwrite.rs:118-130` の網羅の照合に、この名が要る。

### 10.1 Research Needed の答え

**R2: 1 の期限と決定論の試し方**
- 根拠: `areka-sylphya/src/actor.rs:556-581・615`・`areka-actor/src/reply.rs:86-93`・`areka-session/src/exit_wait.rs:18`・`areka-runtime/src/lib.rs:64-65`・`areka-ghost/src/sylphya_wiring.rs:123-130`。
- **期限**: `areka_session::exit_wait::EXIT_WAIT_LIMIT`（3 秒）を使う。`pub` で cfg が無く、本体の `install/worker.rs:27` が既に `exit_wait` を引いている。
  - `areka_runtime::spine::SPIN_WAIT`（30 秒）は `test-support` の中だけにあり、本番からは引けない。
  - `barrier_within` の本番の呼び手は今 0 件。テストの外の呼び手は `areka-input/src/drag_test_support.rs:252` の `SPIN_WAIT` だけ。
- **期限切れのとき**: `barrier_within` は `ReplyError::Timeout` と `error!`（「sylphya barrier timed out」）を出す。書き手が居なければ、`post_barrier` が warn と `Dropped` を出す。
- **試し方**: 今のゴーストの書き手へ偽の IO を差す公開の口は無い（`spawn_ghost_sylphya` は `FsPersistIo` の決め打ち）。
  - ただし `areka_sylphya::spawn_sylphya(init)` と `PersistIo` は公開である。テストは門で止める偽の IO で単独の書き手を起こせる。
  - 待ちを `settle_record(publisher)` に切り出し、別のスレッドで呼んで `join_thread_bounded` で待つ。今の `barrier()` なら止まるので赤になる。
  - `install_record_unsettled` を見るテストは今 0 本。
- **37 の行**: 期限の無い `publisher.barrier()` は今 `mcp/get_property_tests.rs:103・318` にある（4 節の 153・368 は古い）。

**R3: 名の違いが結果に効くか**
- 根拠: `areka-session/src/reload_test_support.rs:17-41`・`reload.rs:749-751`・`crates/areka/Cargo.toml:111`・`boot_config.rs:19-21・176-195`。
- **11**（`install/desk_tests.rs:730`）: 効かない。
  - 呼び手は 8 か所（`desk_exit_tests.rs:101・143` を含む）で、どれも `"A"`。
  - 本番のインストールが読むのは `roots`・`current.ghost.dir`・`folder` だけで、バルーンとプロファイルは読まない。
- **45**（`roots/` の 3 つ）: 本番は名を読まない（`apply` は `current` に触らない）。
  - `edit_tests.rs` の 330・443・447 行の表明は `ga` を名指すので、呼ぶ前の `current` との比べに直す。
  - 上書きが要る欄は 2 つ。`roots_origin: Default`（既にある補助は `EnvVar` で、置き場の変更を全部断る）と、`app_profile_dir`（`edit_write_failure_tests.rs:78` が比べる）。
- **58**: 効かない（`submit` は `BootContext.roots` だけを読む）。
- **27**（`update/` の 4 つ）: 効く（`update/desk.rs:339・605`・`update/mod.rs:196` が名を読む）。
  - 違うのは名だけで、ほかの欄は既にある補助と同じ。
  - `write_descript` の写しは 2 つの形に分かれている。charset の頭と末尾の改行を付けるものと、付けないもの。
- **上書きの形**: `BootContext`・`CurrentGhost`・`ConfigInputs` の欄はどれも `pub` で、本体の dev 依存は `areka-session` の `test-support` を開けている。
- **向こうの `ghost_switch_test_support::boot_context(root, current)`**: ファイルは在り、本体から届く。ただし呼び手が `CurrentGhost` を丸ごと組むので、`update/` の写しは減らない。よって使わない。

**R4（済み）**: 2 節のとおり。

**R5: `switch_to` を分けたとき**
- 根拠: `areka-session/src/ghost_switch.rs:597-708・756-858`・`switch_copy.rs:87-125`・`session_end.rs:1-8`・`install/overwrite.rs:143-193`。
- **⑴ kanade の保留**: 変わらない。流れは、切替の受理 → kanade が台詞の終わりを待つ → 停止の通知と中身 → `switch_to` のまま。
- **⑵ 窓もゴーストも無いフレーム**: 置き場 `GhostSlot` を読む本番の系は、どれも `get_non_send::<GhostSlot>()?` と `.0.as_ref()?` の `Option` で受ける。当たるのは本体の `install/`・`mcp/`・`update/`・`menu/`・`roots/`・`file_drop.rs` と、`areka-session` の `ghost_session_runtime.rs`。
  - 窓口の `drain` は毎 tick に汲むので、tick が回れば背景の答えは届く。
  - 窓 0 の間も tick が回るかは、確かめ切れていない（OPEN QUESTION 3）。
- **⑶ 終了**: 展開は背景のスレッドが `enter_write` の中で行うので、`begin_close` は上限まで待つ。待ちの材料は捨て、起こさない（今の写し待ちと同じ）。
  - OS のセッションの終了は窓の手続きで受けるので、窓 0 の間は受けられない。これは今の同期の展開と同じ（窓 0 で、汲み上げも止まる）。
- **⑷ 失敗**: 起こす後半（`boot_into` → 既定へ戻す → 致命）は同じ関数 `boot_target` が通るので、今のまま。
- **⑸ 変わる行**: `ghost_switch.rs` の `switch_to`（597〜708 行）の後半と、末尾の mod の行だけ。
  - `take_down`・`boot_into`・`SwitchStage`・`on_ghost_stopped` は変えない。
  - 待ちは新しい NonSend `BetweenHold` で表す。
  - `BodySystems::overwrite_between` の形（`fn(&mut World)`）は変えない。`body_systems.rs`・`main.rs` には触らない。
  - 前例は写し待ち（`switch_copy::boot_dir`・`poll`）。

**R6: 25 の改名と向こうのテスト**
- 根拠: `log-capture-kit/src/capture.rs:166-172`・`shell_balloon_switch_session_update_tests.rs:125・217-262`。
- **答え**: 1 件のままで、向こうのファイルの直しは要らない。
  - `shell_balloon_switch_session_update_tests.rs:217-262` は背景のスレッドを回さず、`ask_reload_for_test`（`update/desk.rs:554`）で窓口へ直に頼む。
  - `capture` が取るのは、呼んだスレッドと窓の中で起こしたアクターだけ（大域の受け手ではない）。
  - 数えは `event` の欄だけで絞る。
  - よって数えるのは窓口の側（`update/desk.rs:206`・Accepted の info）。
- **範囲の中の読み手**:
  - `desk_reload_tests.rs:224・320・386・548`: 窓口の側なので、そのまま。
  - `procedure_reload_tests.rs:88・175`: 背景の側なので、新しい語へ。
  - `worker_path_tests.rs:200-206`: 出所の絞りを外す。
  - `worker_path_tests.rs:323`: 無いことの列に新しい語を足す。
- **範囲の外の読み手**: 台帳 `shiori.toml:7188` だけ。`tools/` に読み手は無い。

**R7: 3 のテストの置き場**
- 根拠: `areka-emo-present/src/presenter/{target,show,snapshot,refresh}.rs`・`command.rs:65`・`areka-runtime/src/frame/wiring.rs:289-291`。
- 控え `last_show: Option<(u32, BindSet, PatternState)>` は、surface・着せ替え・アニメーションの段を全部持つ（`target.rs:144`・`show.rs:447`）。そのため `compose_last_shown` は `compose_alone`（`snapshot.rs:72-87`）と同じ形で書ける。
- 単体のテストは `snapshot_tests.rs` に置く。手本は 215 行の `last_shown_after_invalidate_cache_keeps_id_without_picture`。`invalidate_ok` は crate の中から使える。
- 本体の側は、`PresentCommand::InvalidateCache { target, reply: None }`（公開）を `RuntimeWiring::apply_present`（`test-support`）で送れば、隠した target の控えを捨てた状態を作れる。
- `dump_surface.rs:280` は `None` と `Some((_, None))` を 1 つの `_` で受けているので、腕を分ける。

**R8: 4・28 の短命のスレッド**
- 根拠: `roots/edit.rs:258・293-312`・`mcp/get_expression_table.rs:104-121`・`log-capture-kit/src/capture.rs:139`・`areka-actor/src/spawn.rs:115-121`。
- **4**: 記録だけの段は、`status`（根ごとの `is_dir`）・`survey(&roots).hidden`・`log_roots`・`log_hidden`。`roots` は手元の値なので、そのままスレッドへ移せる。
  - `capture` はスレッドに閉じる。テストの記録の表明（`edit_tests.rs:346-353・457`）は、スレッドに dispatcher を持たせたうえで、JoinHandle を待った後へ移す。
- **28**: 前例は `dump_surface.rs:209-234` の `encode_elsewhere`。`thread::Builder` の名前つきで、起こせなければ `error!` と `NG:` を返す。
  - テストは `#[cfg(test)]` の `WaitAnswer::wait_answer`（`dump_surface.rs:405-430`）で待てる。
- どちらも終了の門に乗せない。失うのは記録の行と、相手が居ない答えだけ。

**R9: 54 の `allow` を外した後の警告**
- 根拠: 4 節の 54。
- コンパイルでは確かめていない。
- 当たる項目は 2 種類。`pub mod readme` の下の `pub` の項目（`WindowsShell`・`init_com_for_shell`）と、`pub` の型の trait の実装から使われる私有の項目。本体の `cli/help.rs:49-51` が呼ぶ。
- 実装の段で `cargo clippy -p areka-session --tests` を回し、警告 0 を確かめる（タスクの確かめに入れた）。

**R10: 5・23 の panic のテスト**
- 根拠: `crash-report-kit/src/lib.rs:227-252`・`areka-actor/src/spawn.rs:107-172`・`install/worker.rs:67-87`・`update/worker.rs:70-96`・`update/desk.rs:417-425`。
- human-panic のフックを入れるのは `main.rs:124` だけで、テストでは入らない。
- crash-report-kit の初回の受け手は ACCESS_VIOLATION だけを書くので、std のスレッドの panic では動かない。
- `spawn_actor`・`run_inbox` に `catch_unwind` は無い。前例は `areka-runtime/src/spine_wait_tests.rs:315`（アクターの panic → `join_bounded` が `Panicked`）。
- **落とし穴**: 倒れた道で素の `OrderDone` を送ると、次の依頼が黙って消える。死にかけのスレッドの受信端がまだ生きている間に、窓口が次の依頼を送るためである。よって倒れた知らせは別の腕 `WorkerPanicked` にし、窓口は `worker = None` にする。
- **守り手の置き場**: `update` は依頼ごとに `ports` を作るので、守り手を `ports` より先に作る。`install` は `ports` がスレッドに 1 つなので、順は効かない。

**R11: 102 の値なしの表れ方**
- 根拠: `areka-ghost/src/prop_get_sink.rs`・`mcp/get_property_tests.rs:236-300`・`areka-runtime/src/consumer_ledger_test_support.rs:126`。
- `PropGetCueSink` は、見つからない値（`DottedResolution::NotFound`）を空の文字列にして、`KanadeMsg::PropertyEvent` の references に載せる。値なしと空の値は区別できない（`prop_get_sink_tests.rs:71-94`）。
- 今のゴーストの道の期待 `Err("PropertyNotFound")`（`currentghost.balloon.scope(2).rect`）は、空の文字列に書き直す。
- 合図は `areka_runtime::consumer_ledger::test_support::carrier_cue` で組める。本体が既に `install/install_cue_tests.rs:18` で使っており、dola には触らない。

**R12: 103 で選ぶ画面を寄せるか**
- 根拠: `areka-session/src/alert.rs:267-290・324-358`・`crates/real-run-kit/src/launch.rs:232-239`。
- 選ぶ画面は `install/desk.rs:177` だけ。`worker.rs:192`（利用条件の `ask_yes_no`）と `worker.rs:243`（`warn_ok`）は箱である。どれも `alert::suppressed()` を見る（`AREKA_NO_ALERT` だけ）。
- 自動終了の走行の使われ方:
  - real-run-kit（`launch.rs:232-239`）・`tools/package.ps1`（694〜695 行）・`doc/v0.1.0-gate-real-run.md` は、必ず `AREKA_NO_ALERT` と組む。
  - `tools/perf` はインストールしない。
  - `AREKA_NO_ALERT` を外すのは、`doc/pasta-real-run.md:128` の人が見る 60 分の走行だけ。メニューのインストールは使わないと 147 行が書く。
- **結論**: 寄せても今の使われ方は変わらない。ただし、自動終了の走行で箱が出なくなる振る舞いの変化はある。指摘 103 の的は起動の前の写しだけなので、寄せない（OPEN QUESTION 4）。

### 10.2 設計の段で新しく分かったこと

- **6 と 60 は同じ欄を使う**:
  - 背景のスレッドの `DeskPorts` は `roots` を既に持ち（`worker.rs:94`）、`run_order` は記録の前に必ず素性を聞く。前例は `before_boot_ports.rs:111-126`（背景で `catalog_spelling` を済ませる）。
  - 隠れの判定には今のゴーストの `dir` が要る。ところが `GhostFacts.running_dir` は、根の下のゴーストのときだけ `Some` である。そのため `GhostFacts` に常に在る `dir` を足す。60 も、窓口が `dir` だけを答える形で同じ欄を使う。
  - 残る差: 根と今のゴーストは「素性を聞いた時点」の値になる。手続きの途中の置き場の変更や切替は映らない。
- **69 の欄の読み手**: テストだけ（`worker_tests.rs:344・350-355・405`。`desk_tests.rs:262・279` は作るだけ）。`ends` の記録は `run_order` の戻り値から出せるので、欄は要らない。
- **60 の呼び手**:
  - `sakura_name`: `install/desk.rs:498`・`areka-session/src/ghost_switch.rs:355`・`same_folder_pin.rs:33`。
  - `install_accept`: `install/desk.rs:499` だけ。
  - 前例: `standard_balloon_keys` → `StandardBalloonKeys`（`catalog.rs:284-300`）。
- **12 の置き場**: `RawInstallRequest` の使い手は `install/` の中だけで、`main.rs` は `raw_sender` の戻りを名指さない。
  - URL を窓口の生の要求として運べば、窓口が待ち行列を持てる。`main.rs` と字面の檻（`zorder_wiring_tests.rs:56`）は変わらない。
  - その結果、窓口が無いときの `url` の腕は取りに行かなくなる。件 8 の事実の 1 つが変わる（OPEN QUESTION 2）。
- **68 の守り**: `path` の腕も出どころが `Script` である。そのため消すのは、出どころが `Script` で、かつ `download_dir()` の下にある書庫だけ。テストのために置き場を引数で渡す（`remove_downloaded(order, dir)`）。
- **`SwitchVerdict::Updating`**: `install/overwrite.rs:118-139` の照合は、`Accepted`／`Busy`／`NotFound | NoContext` の 3 つの腕で網羅している。
  - `Updating` が入るのは 129 行の `NotFound | NoContext` の腕（書庫を返してよそへ回す warn）で正しい。
  - この spec の頼みは出どころが `ChangeOrigin::Automatic` である。`Updating` が断るのはメニューの出どころだけなので、この腕は通らない。
- **`consumer_ledger.rs:77` の持ち主**: C26 の 3 本の受け持ちに無い。`audit-emo2-boot-switch` の `areka-runtime` の受け持ちは `switch_assets*`・`balloon_visibility*` ほかである。
- **1,000 行**: `install/desk_tests.rs`（788）・`ghost_switch_tests.rs`（982）・`update/worker_tests.rs`（967）には足さず、新しいテストのファイルへ置く（design.md の表）。

### 10.3 合成（synthesis）

- **一般化**:
  - 背景のスレッドの倒れ方（5・23）は、同じ形の守り手 1 つにした。窓口の腕も同じ `WorkerPanicked` にした。
  - 「UI スレッドで遅い I/O を待つ」所（1・4・6・13・28・60）は、どれも既にある形のどちらかへ寄せた。期限つきの待ちは 1。背景のスレッドへ出すのは 4・6・13・28・60。
  - 写しの寄せは、テストの支えのファイルを crate とモジュールごとに 1 つにした（`update/`・`mcp/`・`roots/`・`readme/`）。
- **採るか作るか**:
  - 採ったもの:
    - 待ちの上限: `barrier_within`・`EXIT_WAIT_LIMIT`・`join_bounded`・`join_thread_bounded`・`wait_recv`。
    - 門: `exit_wait::register_gate`。
    - 名前つきのスレッド: `encode_elsewhere` の形。
    - 切替の待ち: 写し待ち（`switch_copy`）の形。
  - 新しく作るもの:
    - `BetweenHold`。
    - 取得の待ち行列。
    - 守り手。
    - 公開の関数 2 つ（`compose_last_shown`・descript の 1 回読み）。
- **削り**:
  - `SwitchStage` に腕を足さない。
  - `BodySystems` の口の形を変えない。
  - 1 の期限を引数にしない。
  - 4 に試験用の止めの口を作らない。
  - 103 で選ぶ画面を寄せない。

### 10.4 危うさ

- 13 の待ちの間、窓 0 で tick が回るか（OPEN QUESTION 3）。回らなければ、背景の頼みで wintf の公開の起こしの口を叩く形を足す。
- 13 の待ちの材料（`WindowsClosed`・`ShellCarry`）を NonSend に置けるか。どちらも借用を持たない値に見えるが、実装の最初に確かめる。
- 12 で件 8 の事実の 1 つが変わる（OPEN QUESTION 2）。これは `audit-areka-app` の `NO_BODY` の doc の直しの文に効く。
- 1 のテストは、期限（3 秒）のぶんだけ待つ。

## 11. `audit-areka-app` の着地に依る寄せ（タスク 5.1・G23）

- 着地の確かめ: `audit-areka-app` は `main` へ着地済み（PR#504・`056b66813`）。`main` はこのブランチへ取り込み済み（`d5f2d103f`）で、`git log HEAD..origin/main` は 0 件。向こうは `completed/` へ移っている。

### 寄せた件

- **31**: `png_image_tests.rs` の `wic_decode_rgba` が `pub(crate)` になっていた。`mcp/dump_surface_gpu_test_support.rs` の写し（約 30 行）と、それだけが使っていた WIC の import を消し、`crate::png_image::tests::wic_decode_rgba` を呼ぶ。
- **85**: 向こうの `areka-session/src/log_history_tests.rs` に `kanade_emit_with_three_labels_drafts_as_script_records` が移っていた（中身は同じ）。`mcp/sakurascript_log_tests.rs` と `sakurascript.rs` の据え付けの 3 行を消した。
- **件 6**: `reload_test_support::boot_context_named` が入っていた。`update/` の分はタスク 3.3 で済み（`update_test_support.rs` に `boot_context` は無く、`update/desk_tests.rs`・`desk_resolve_tests.rs` が直に呼ぶ）。確かめだけ。
- **103**: 向こうの `alert.rs` に `alerts_suppressed()` が入っていた。起動の前の道（`install/startup.rs` の `resolve_boot`）がそれを呼び、`before_boot_alert_suppressed` と `startup_flow_tests.rs` の表明（`either_value_suppresses_the_alert_before_boot`）を消した。決まりのテストは向こうの `alert_tests.rs` の `with_smoke_suppresses_when_either_is_set`。`raise` は向こうの着地の形で既に同じ純粋な口 `with_smoke` を通るので、`alert.rs` には触らない（例外 (b) の外）。選ぶ画面の 3 か所は寄せない（OPEN QUESTION 4）。
- **94**: 向こうの `alert.rs` の読み手は `RootError::ExeLocationUnavailable { .. }` で欄を読まない形だった（`alert.rs` は無変更）。`roots/resolve.rs` の `tried`・`origin` の欄を消し、`resolve_tests.rs` の 2 か所・`report_tests.rs` の 1 か所と、例外 (b) の `alert_tests.rs` の `tried`・`origin` の 2 行を同じ変更で消した。
- **例外 (c)**: 向こうは脳の残り（`shiori_host.rs`・`shiori_host_case_fold_tests.rs`）を残して着地していた。1.7 の後に `ShioriHostSink` の読み手はこの 2 ファイルだけ。2 ファイルと、それを据える `main.rs` の `#[cfg(test)] mod shiori_host;`（と doc の 1 行）を消し、本体の `Cargo.toml` の dev の 2 行（`shiori-abi`・`windows-core`）をその注釈ごと消した。本体の中に `shiori_abi`・`windows_core` を使う所はほかに無い。`Cargo.lock` の差分は本体の依存の列の 2 行（`shiori-abi`・`windows-core`）だけ。

### 残した件

- なし（6 件とも向こうの着地の形があったので寄せた）。向こうの brief への書き足しは要らない（向こうは完了・残す件が無い）。

## 12. `audit-emo2-boot-switch` の着地に依る寄せ（タスク 5.2・G23）

- 着地の確かめ: `audit-emo2-boot-switch` は `main` へ着地済み（PR#503・`7f4778188`）。`main` はこのブランチへ取り込み済みで、`git fetch origin main` の後の `git log HEAD..origin/main` は 0 件。向こうは `completed/` へ移っている。

### 寄せた件

- **33**: `areka-session` の `shell_balloon_switch_session_lap_test_support.rs` で `spawn_windows(rig, dpi)` と `pump_messages()` が `pub` になっていた（`areka_session::ghost_session::shell_balloon_switch_session_lap_test_support` から届く）。`mcp/dump_surface_gpu_test_support.rs` の写し 2 本と、それだけが使っていた定数（`WORK_AREA`・`PUMP_MAX`）・import（`HWND`・`MSG` 系・`MonitorSnapshot`・`RectPx`・`spawn_ghost_windows`・`WindowHandle`・`DPI`）を消し、向こうの 2 本を呼ぶ。ファイル先頭の doc の行は、`lap_rig_of` を `shell_balloon_switch_session_lap_tests.rs` に、`spawn_windows` を本当の置き場の `shell_balloon_switch_session_lap_test_support.rs` に結び直し、写しでなく呼ぶ形へ直した（置き場の誤りも正した）。MCP の実 GPU のテスト（`dump_surface_gpu_tests`・`dump_balloon_gpu_tests`）は緑。
- **例外 (a)**: `SwitchVerdict::Updating` は向こうの着地で `ghost_switch.rs` に入っていた。`install/overwrite.rs` の `SwitchVerdict` の腕は `NotFound | NoContext | Updating` と名前を挙げており、ビルドは緑。こちらの変更は要らなかった。

### 残した件

- なし（向こうの brief への書き足しは要らない・`/kiro-complete` の棚卸へ申し送る件も無い）。

## 13. 預かった件の片づけと申し送り（タスク 5.3・G24）

### 件 1〜8 の片づけ（要件 5.9）

| 件 | 出どころ | 片づけ | 申し送った相手 | コミット |
|---|---|---|---|---|
| 1 `barrier()` | `audit-sylphya-actor` の audit #4（2026-11-06 開発者の承認） | 指摘 1。`record_installed` の待ちを期限つきの待ちに替え、切れたら `install_record_unsettled` を残して続ける | なし | `e39f99d9e` |
| 2 `UNKEPT_TARGETS` | `runtime-module-untangle` の申し送り（棚卸㊸で出所を `areka_session::readme` に読み替え） | 足す物は無い（下記） | `audit-areka-app`（完了済みのため本 spec の `/kiro-complete` の棚卸へ） | 記録のみ |
| 3 呼び手の説明 | `session-module-gather` の申し送りの件 2 | 指摘 8・70・71・81・49。`install/mod.rs`・`update/mod.rs`・`mcp/mod.rs`・`install/names.rs`・`areka-session/src/readme.rs` の説明を今の呼び手（`BODY_SYSTEMS` の `after_input`・`after_runtime_wired`、説明書は `runtime_boot::install_lines`）に直した | `audit-areka-app`（完了済みのため本 spec の `/kiro-complete` の棚卸へ） | `b6477c5cb`・`fd86dfce6` |
| 4 `let _ = join()` | 棚卸㊶（ほかの brief から寄せられた） | 指摘 19。`install/names_tests.rs` の待ちに上限を持たせ、書き手の panic を捨てない形にした | なし | `4d4538280` |
| 5 `capture_logs` の写し | 同 | 指摘 57。`readme/readme_cue_tests.rs` の私有の写しを共通の捕捉の補助に寄せた（`areka-session` の中に `fn capture_logs` の写しは残っていない） | なし | `a20ff49e8` |
| 6 `boot_context` の写し 9 | 同 | `install/desk_tests.rs`・`readme/opener_submit_tests.rs`・`roots/` は既にある `reload_test_support::boot_context` へ。`update/` の 4 つは `reload_test_support::boot_context_named`（`audit-areka-app` が着地で足した 4 引数の形）を直に呼ぶ形へ寄せ、範囲の中の補助は 0。範囲の中の `fn boot_context` の写しは残らない | なし（`audit-areka-app` が `boot_context_named` を足して済んだ） | `66889523f`・`ce6c8f54c`・`15be77e35`・`a20ff49e8`・`a6a452867` |
| 7 `BODY_SYSTEMS` の 1 行 | `session-crate-split` の申し送り（C26 の約束 ⑴） | 3 本とも残す（下記） | なし | 記録のみ |
| 8 `NO_BODY` | 同 | そろえない（下記） | `audit-areka-app`（完了済みのため本 spec の `/kiro-complete` の棚卸へ） | 記録のみ |

件 2・8 の相手 `audit-areka-app` と、件 3 の `mcp/mod.rs` の相手も同じ `audit-areka-app` は、`completed/` へ移っており走っているセッションが無い。申し送りの宛先は `/kiro-complete` の棚卸（本 spec の完了時の棚卸）へ回す（下記「申し送りの扱い」）。

### 件 2: `UNKEPT_TARGETS` へ足す物は無い

- `update_cue`（`areka::update…`）と `readme_cue`（`areka_session::readme…`）に `info!` は 1 件も無い（`git grep "info!"` が両ファイルで 0 件）。本 spec の直しのどれも足していない。新しく足した `info!` は `roots_removed`（`areka::roots::edit`）と `install_fetch_*` の既存の出所の下だけで、前方一致の `RULES` に載る。
- `UNKEPT_TARGETS` は今も `["areka::install::install_cue"]` の 1 要素（`areka-session/src/log_history.rs:136`）。**足す物は 0**。
- 後の直しで `update_cue`・`readme_cue` に `info!` を足すことになったときは、足した出所を `UNKEPT_TARGETS` へ足すことを `audit-areka-app`（完了後は `log_history.rs` の持ち主の spec）へ申し送る。

### 件 7（指摘 59）: 3 本の `world.insert_resource(crate::BODY_SYSTEMS);` を残す理由

- 残す行は `install/desk_tests.rs` の 385・564 行と `update/desk_resolve_tests.rs` の 530 行（行は今のもの）。
- 理由: 本体の口を通る道を試す前提を名指す 1 行で、共通の足場（`reload_test_support`）へ寄せると、同じ足場を使うほかのテストの前提が黙って変わる。1 行ずつ残すほうが前提が各テストに見える。この区画に新しく足す直しは無い。

### 件 8: `NO_BODY` と「窓口が無い」の受け口をそろえない

- `NO_BODY`（`areka-session/src/body_systems.rs`）の `cue_sinks` は `DiscardSink` 2 つで、本体が据わらない結線の誤りのときにだけ使う（`error!(body_systems_missing)` が先に出る）。本体の「窓口が無い」ときの受け口（受信端の無い送出端を持つ `InstallCueSink`）も本番の道では通らない。どちらも記録が残るので、**そろえない**と決める。
- 指摘 12 の直し（`d31e38a8b`）で、`InstallCueSink` の `url` の腕は取得のスレッドを起こさず、要求を窓口へ送るだけになった（`install/install_cue.rs` の「送るのは要求だけで、取りに行くのは窓口の取得の待ち行列」）。このため「窓口が無い」の `url` の腕も取りに行かなくなる。件 8 の差が 1 つ減る。
- `NO_BODY` の doc の「（窓口が無いときの答えと同じ）」は、持ち主の `audit-areka-app` が着地で消した（`056b66813`・PR#504。向こうの tasks.md 43 行・design.md 165 行）。今の `body_systems.rs` に無い（`git grep` が 0 件）。doc の直しで申し送る物は無い。

### 91 と `link_destinations` の印は裁定で残す

- **91**（`roots/edit.rs` の `error!(roots_memory_write_failed)`）: 残す（議題 10 の (i)）。完了 `areka-P0-write-failure-notice` の要件 5.5 が名前を変えないと決めており、2 行は致命の道でだけ出る。今も `roots/edit.rs:310` に在る。要件 9 の数から外す。
- **`link_destinations` の先送りの印**（`areka-session/src/readme/destination.rs:223` の `#[cfg_attr(not(test), allow(dead_code))]`）: 残す（議題 11・要件 2.5）。実在の理由（`link-context-copy`・`balloon-link-hover` が本番の呼び手になる）があり、2 本とも `.kiro/specs/` の下で生きている（`completed/` でない）。要件 9 のかけ直しの数から外す。

### 申し送りの扱い（要件 5.2・5.3・5.8・7.2・7.9・7.10）

`audit-areka-app` と `audit-emo2-boot-switch` は `main` へ着地して `completed/` へ移っている（PR#504・PR#503）。ほかに動いている spec のセッションは無いので、申し送らずにこの spec で片づけた（開発者の指示）。

- **件 2・件 3・件 8（`audit-areka-app` の持ち物と重なっていた件）**: 残る作業は無い。
  - 件 2: `UNKEPT_TARGETS` へ足す物は無い。後で `update_cue`・`readme_cue` に `info!` を足すときは出所を `UNKEPT_TARGETS` へ足す。
  - 件 3: `mcp/mod.rs` の呼び手の説明は本 spec が直した（`b6477c5cb`）。
  - 件 8: `NO_BODY` と「窓口が無い」の受け口はそろえない。指摘 12 の直しで「窓口が無い」の `url` の腕も取りに行かなくなる。`NO_BODY` の doc は `056b66813` で直っている。
- **`ghost_switch.rs` の切替の入口が `ChangeOrigin::Automatic` の要求に `update_busy` を見ない件**: 直さないと決めた。台本の要求・更新自身の読み直しを断らない形は、完了 `audit-emo2-boot-switch` の決め（design.md の D2・確定 1、requirements.md 5.16）と完了 `areka-P0-network-update` の要件 5.7 のとおり。更新の途中で別のゴーストへ切り替わっても、依頼を受けた時点の kanade の送出端の写し（`update/worker.rs` の `UpdateJob.kanade`）が閉じて `update_abandoned` で残りを捨て、読み直しは `other_ghost` で頼まず（`update/desk.rs` の `reload_folder`）、ファイルの確定は全か無か（`areka-update/src/commit.rs`）。檻は `ghost_switch_tests.rs` の `updating_refuses_menu_requests_only`・`desk_reload_tests.rs`・`procedure_reload_tests.rs`。ukadoc は更新中の `\![change,ghost,…]` を書いていない（https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-change-ghost-option-eq-raise-event）。台本も断る直しは要件 2.2 の 1 種類目（完了 spec の決めに反する）に当たる。

### 書き足した brief

- `.kiro/specs/link-context-copy/brief.md` と `.kiro/specs/balloon-link-hover/brief.md`: 古いパス `crates/areka/src/readme/destination.rs` を今のパス `crates/areka-session/src/readme/destination.rs` として読む件を書き足した（どちらも `completed/` でないことを確かめた。`crates/areka/src/readme/` は今は無い）。

### 知らせなかったもの

- C26 の 2 本のセッションへの知らせは行っていない。この実行には生きている相手のセッションが無く（両方とも `completed/` へ移った後）、知らせる宛先が無い。代わりに上の「申し送りの扱い」の内容を `/kiro-complete` の棚卸へ回す。

## 14. かけ直しの audit（タスク 5.4・G25）

同じ作法（`/ponytail:ponytail-audit` から「最大 20 件」と「読まなかった所」を外す・区画ごとに全行を読む）で、範囲の 124 ファイルに本 spec が新しく置いた範囲の中のファイルを足してかけ直した（消したファイル `update/update_tests.rs`・`mcp/sakurascript_log_tests.rs`・`shiori_host*` は外れる）。区画は 3 節の S1〜S6 と同じ割り方（S1 に `desk_overwrite_between_tests.rs`・`desk_worker_gone_tests.rs`・`fetch_queue_tests.rs`、S3 に `homeurl_tests.rs`・`update_test_support.rs`・`worker_panic_tests.rs`、S5 に `kanade_fake_test_support.rs`、S6 に `roots/roots_test_support.rs`・`areka-session` の `ghost_switch_between.rs`・`ghost_switch_between_tests.rs` を足した）。91 と `link_destinations` の印、11〜13 節で申し送った件は数えない。

### 1 回目（HEAD `23c28dca1` の後）

| 区画 | 必ず直す | 直したほうがよい | できれば | 2.2 の 3 種 |
|---|---|---|---|---|
| S1（`install/` の前半） | 0 | 3 | 2 | 0 |
| S2（`install/` の後半） | 0 | 6 | 4 | 0 |
| S3（`update/`） | 0 | 5 | 2 | 0 |
| S4（`mcp/` の絵と表） | 0 | 1 | 6 | 0 |
| S5（`mcp/` の残り） | 0 | 5 | 4 | 0 |
| S6（根・説明書・切替の待ち） | 0 | 5 | 4 | 0 |
| 計 | 0 | 25 | 22 | 0 |

- 半分ほどは前の直しが呼んだ連鎖（写しを寄せた後に残った写し・移した関数を指す古い doc・テスト用の継ぎ目）。要件 9.8 のとおり Fable で関連する指摘をまとめて直した（束 F1〜F4）。
- 直した主なもの: 「`event` の名で記録を引く」補助の写しを `crate::session_tests::events_named` の 1 つへ寄せた（`install/`・`update/`・`roots/` の 11 本・`update_test_support::named`・`roots_test_support::events_named`）。`roots-log` を 1 本の常駐のスレッドの待ち行列にして、記録が渡した順に並ぶようにした。`dump_surface` と `get_expression_table` のスレッドの起こし方を `work_elsewhere` の 1 つにした。homeurl の trim のテストを本番の経路の 2 本へ移し、テスト用の継ぎ目 2 つを消した。使われない `TermsNotice::clipped` を消した。古い doc・経緯の語・テストの名を直した。
- S4 の 1（`dump_surface_gpu_test_support.rs` の台詞の Tick の注入が `areka-session` の `SwitchRig::inject_talk_tick` の写し）は、1 回目は範囲の外として残し、2 回目で `inject_talk_tick` を `pub` にして寄せた（開発者の指示で申し送らずに直す）。
- 直しのコミット: `cb5f93de5`（F4）・`f171d96dc`（F1）・`f16431745`（F3）・`b77a51661`（F2）・`8ac1d450b`（design の追随）。

### 2 回目（HEAD `b705afee8`）

| 区画 | 必ず直す | 直したほうがよい | できれば | 2.2 の 3 種 |
|---|---|---|---|---|
| S1 | 0 | 1 | 1 | 0 |
| S2 | 0 | 0 | 4 | 0 |
| S3 | 0 | 2 | 5 | 0 |
| S4 | 0 | 1 | 2 | 0 |
| S5 | 0 | 1 | 4 | 0 |
| S6 | 0 | 2 | 5 | 0 |
| 計 | 0 | 7 | 21 | 0 |

- 連鎖は 4 件（1 回目の直しの後に残った doc・未使用の定数・`work` の効き方）。ほかは 1 回目に寄せ切れなかった写し（記録を `event` の名で引く式・`ghost()` の値・`count`・`plant_committed`）と、古い doc・経緯の語。
- 振る舞いの変わる直しは 1 件: メニューの「ネットワーク更新」が、読み直しの切替の後送り（`after_switch`）を持つ間も選べないようにした（`update/desk.rs` の `can_update` を `is_busy` と同じ見方へ・完了 `areka-P0-network-update` の要件 16「手続きが走っている間は選べない」のとおり・赤を立ててから直した）。
- 申し送らずに直した物（開発者の指示）: `SwitchRig::inject_talk_tick` を `pub` にして GpuRig の写しを消した。`areka-session` の `refs_of` を `pub` にして本体の `update_test_support::refs_of` をその先頭を返す形にした。`count(events, event, level)` の写し 5 本（範囲の外の `runtime_tests/frame/`・`session_tests/` を含む）を `session_tests::count` の 1 本へ寄せた。
- 直しのコミット: `2cbace450`（areka-session）・`b399a2e56`（MCP と Tick）・`dd5fa22eb`（install・update・roots ほか）。
