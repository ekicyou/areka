# ギャップ分析: audit-areka-app（ponytail-audit・2026-11-22）

## 1. 要約

- **やり方**: `/ponytail:ponytail-audit`（5.3.3）の作法から「最大 20 件」と「読まなかった所」を外してかけた（要件 1.2・1.3）。範囲は Boundary Context の In scope だけ（本体 94 ファイル・`areka-session` 74 ファイル・本体の `examples/`・`README.md`・`areka.manifest`、絵の置き場 `shell/`・`icon/`）。範囲を 9 組に分け、読むだけの下請けに全行を読ませた（各ファイル「全行読んだ」の申告つき・3 節）。報告をまとめて重なりを除き、番号を振り直した。「必ず直す」の 2 件と、下請けどうしで食い違った所（指摘 47）は、この書き手がコードを読んで確かめた。重い走行（cargo・実機）は回していない。
- **結果**: 指摘は **97 件**（必ず直す 2・直したほうがよい 74・できれば 21）。
  - 本番の振る舞いが間違うのは 2 件。⑴ 記憶のファイルが使えない場面で、効かない直し方の一文を出す（指摘 1＝預かり 5.1）。⑵ `--offline-dump` が、遠くに置いた element 1 つでシェル全体の下地を巨大にし、確保に失敗して落ちる（指摘 2）。
  - 指摘の芯は次の 4 つ。⑴ 落ちる力の無いテストと守りの抜け（本番の道を通らない「ゴーストの道」・歩く範囲の欠けた見張り・時間の決め打ちの待ち）。⑵ テストの支えの写し（`SharedFakeIo`・`code_only`・`prop_set_cue`・試験台・走査器）。⑶ crate 分けの後に残った古い説明（消えた名・置き場・数）とコメントの決まり違反（要件の番号・spec 名・経緯の語・`// SAFETY:` の欠け）。⑷ 要らないコード（SHIORI の試験用の脳 約 2,800 行・走らないテストを抱えた `mock-shell.rs`・誰も読まない絵 77 個・使い手の無い依存 `async-io`）。
- **SSP 互換・ukadoc の正典に関わる直し（要件 2.2 の 3 種類目）は 0 件**。起動オプションの読み方と、SHIORI へ送るイベントの名・参照・送る条件は、どの直しも変えない。外への約束のうち 3 種類目でないもの（要件 2.7）に触れる直しは 4 件ある。知らせの文（指摘 1）・`AREKA_SHIORI_DEMO`（指摘 5）・ログの行の語（指摘 4・95）。
- **預かった件 5.1〜5.21 は 5.21 の一部を除いて全部まだ在る**（5 節）。5.14 の 6 項目は、どれも範囲の外の使い手がテストだけだった。5.21 は件 2・3・4・7・9・ク が済み、件 1・8（`collision-probe/fixture.rs:23`）とキの言い方のずれが残る。
- **規模**: 見込みは 24〜30 タスク（9 節）。20 を越える。

## 2. 数え直し（要件 1.6）と行番号の当て直し（要件 1.7）

着手のときの HEAD は `ea229215`（親 `1b3ede54`＝`main`。差は spec の 2 ファイルだけ）。`wc -l` で数え、テストの数え方は要件と同じにした。

| 範囲 | ファイル | 本番 | テスト |
|---|---|---|---|
| 本体の残り（94） | 94 | 22 ファイル・5,665 行 | 72 ファイル・20,400 行 |
| `areka-session` の大半（74） | 74 | 28 ファイル・6,938 行 | 46 ファイル・11,917 行 |
| 合計 | 168 | 50 ファイル・12,603 行 | 118 ファイル・32,317 行 |
| 仮定 1 の分（`examples/` 11・`README.md`・`areka.manifest`） | 13 | 2,334 行 | — |
| 絵の置き場 | `shell/` 79・`icon/` 26 | — | — |

**要件の表と 1 行も違わない**。1,000 行に近いのは `tests/stay_on_top_real_run.rs` 966・`main_session_mark_tests.rs` 959・`boot_resolve_tests.rs` 873・`ghost_session.rs` 862・`examples/mock-shell.rs` 856・`session_tests/reload_tests.rs` 820。越えるものは 0。

行番号の当て直し（今の HEAD。「済み」は grep の式と 0 件を書いた）:

| 件 | brief／要件の行 | 今の行・状態 |
|---|---|---|
| 5.1 `MEMORY_WRITE_FIX` | `alert.rs:88` | 88（使う所 224・理由の行 220-223） |
| 5.2 見張り | `session_end_sync_send_tests.rs`・`readonly_body_tests.rs` | 在る（自前の走査 294-378・本体 115-154） |
| 5.3 戻しの継ぎ目 | `ghost_session.rs:213`・`boot_seam.rs:156` | 213-214・156（同じ件が `boot_seam.rs:7`・`:136` にも） |
| 5.4 `plant_minimal_ghost` | `boot_seam_restore_tests.rs:24` | 24（同じ古さが 11・34・118） |
| 5.5 `SharedFakeIo` | `boot_resolve_tests.rs:422`・`boot_seam_persist_wiring_tests.rs:14`・`switch_finish_tests.rs:61` | 421-422・13-14・60-61（3 つとも写し） |
| 5.6 `code_only` | `tick_gate_config_producers_tests.rs:68` | 68-73 |
| 5.7 `join` | `boot_resolve_tests.rs` 482・575・756・804、`boot_seam_persist_wiring_tests.rs` 80・123 | 動いていない |
| 5.8 写し | `runtime_tests/spine.rs:123` | 123（`capture_logs`）・92（`make_world_with_gpu`）・119（理由のコメント） |
| 5.9 `BootContext` | `menu/ghost_frame_tests.rs:43` | 43 |
| 5.10 | `app_exit_escape_tests.rs:73`・`ghost_session.rs:821` | 73・821-822 |
| 5.11 | `menu/mod.rs:223`・`:254` | **221-223**・254 |
| 5.12 | `zorder_wiring_tests.rs` 29・74、冒頭 4・5・10・11・13 | 動いていない（11 の `t_zwi06` は `areka-runtime` で `the_frame_system_is_ordered_before_the_chain_apply` に改名済み・184 行） |
| 5.13 `prop_set_cue` | 95・119・143 | 95・119・**142** |
| 5.16 | `reload_tests.rs` の 2 本 | 142・199 |
| 5.19 `Cargo.toml` | 62・66・75 | 62-63・66-68・74-76（ほか 24-26・80-86） |
| 5.21 件 1 | `examples/emo-present/balloon.rs:45` | 45-71（捨てる所 67-70・説明 44） |
| 5.21 件 2（`scg`・`atom`） | — | **済み**。`git grep -nwE "scg\|atom" -- crates/areka/src crates/areka-session/src crates/areka-runtime/src` は 4 件で、4 件とも `areka-runtime/src/placement_tests/transition_*`（追随の持ち物・件 2 の除外）。範囲の中は 0 件 |
| 5.21 件 3 | `frame_diag_route_tests.rs:19` | **済み**。`git grep -n "areka_placement::test_support::capture_logs" -- crates` 0 件 |
| 5.21 件 4 | `frame_transition_branch_tests.rs:501` ほか | **済み**。`git grep -n "4 つの見送り\|task 6.5\|タスク 6.5" -- crates` 0 件（今は「5 つの見送り点」） |
| 5.21 件 7 | `boot_config.rs:504`・`:563`・`smoke_exit.rs:43` ほか | **済み**。`git grep -nE "memory/\|MEMORY\.md" -- crates/areka-session/src/boot_config.rs crates/areka-session/src/smoke_exit.rs crates/areka-input/src/choice_drain.rs` 0 件 |
| 5.21 件 8 | `smoke_exit.rs` の task 2.3・`collision-probe/fixture.rs:23`・`main_restore_seam_tests.rs:24` | task 2.3 は**済み**（`git grep -nE "task [0-9]\|タスク" -- crates/areka-session/src/smoke_exit.rs` 0 件）。`crates/areka-runtime/examples/collision-probe/fixture.rs:23`「main.rs／他 example と同名」は**在る**（範囲の外・持ち主なし）。`main_restore_seam_tests.rs:24` は 5.4 と同じ件 |
| 5.21 件 9 | `drain_resnap.rs:596`・`:478` | **済み**。`git grep -n "ここ 1 か所だけ" -- crates/areka-runtime/src/frame/drain_resnap.rs` 0 件（今は 605 行「並べ直し（`reset_pos.rs`）も呼ぶ」）。届かない腕は 489 行に「`collect_chain_states` は返さない（解き直し専用・網羅のため置く）」と理由つきで置かれた |
| 5.21 キ | `boot_config.rs:617` | 言い方は直っている（`boot_config.rs:592-593` は `runtime_boot::boot_runtime` を名指し、呼び手 `runtime_boot.rs:138`・`ghost_session.rs:725` と合う）。残るずれは「窓の無い起動」の一語（指摘 53） |
| 5.21 ク | `examples/emo-present/setup.rs:88` | **済み**。`git grep -n build_boot_assets -- crates/areka/examples` 0 件（今の 87-88 は在る関数 `load_shell_target` を指す） |

## 3. 読んだ範囲（要件 1.3）

- 9 組の下請けが、割り当てのファイルを 1 行目から最後の行まで全部読んだ（各組の表に「全行読んだ＝はい」）。組の分け方は次のとおり。
  - G1: `areka-session` の本番の起動・会話の場・読み直し（`ghost_session*.rs`・`boot_*.rs`・`reload.rs`・`runtime_boot.rs`・`hooks.rs`・`lib.rs`・`body_systems.rs`・`switch_copy.rs`・`same_folder_pin.rs`・`Cargo.toml`・`build.rs`・`tests.manifest`）。
  - G2a: `areka-session` の本番の告知・終了・記録（`alert.rs`・`app_exit.rs`・`exit_wait.rs`・`ghost_quit.rs`・`log_history.rs`・`readonly.rs`・`session_end.rs`・`session_mark.rs`・`smoke_exit.rs`・`write_failure.rs`）とその一部のテスト 9 本。
  - G2b: `areka-session` の残りのテスト 10 本と、本体の `main.rs`・`main_*_tests.rs` 4 本。
  - G3: `areka-session` の起動・読み直しのテスト 26 本。
  - G4: 本体の SHIORI の試験用の脳 8 本・`perf_thread_report*`・`png_image*`・`thread_roles*`・`tick_gate_config*`・`file_drop*`・`sample_test_support.rs`・`scope_bind_fixture_test_support.rs`。
  - G5: 本体の `cli/`・`offline_dump/`・`Cargo.toml`・`build.rs`・`README.md`・`areka.manifest`。
  - G6a・G6b: 本体の `session_tests/` 26 本。
  - G7: 本体の `tests/` 16 本・`examples/` 11 本・`shell/`・`icon/`（`icon/README.md` は全行）。
- `shell/`・`icon/` の絵は中身を読まない（仮定 1）。置き場・名前・使われ方（`git grep` で `include_bytes!`・`build.rs`・`tools/`・テスト・例・文書の指す先）と説明の文とを突き合わせた（指摘 76・97）。
- 範囲の外は読むだけにした。使い手の数え・`areka-sylphya` の `persist`・`areka-emo-compose` の `plan_bounds`・`wintf` の `init_ui_com`・`log-capture-kit/tests/workspace_scan`・`areka-runtime` の `spine`・`frame_test_support`・`consumer_ledger_test_support`・`areka-placement` の `test_support`・本体の `menu/`・`runtime_tests/` の預かりの行。
- **Not checked（要件 1.3 で残さないもの）は無い**。ただし、どの判断も読んだ中身からのもので、cargo と実機の走行で確かめた指摘は 0 件。

## 4. audit の結果

**What this repo does**: 範囲は areka の本体（exe）の入口と、起動・切替・読み直し・終了を受け持つ層 `areka-session`。起動オプションを SSP と同じ書き方で読み、起動するゴーストとバルーンを決め、窓を出して SHIORI と実行系を結ぶ。読み直し・同じプロセスでの起こし直し・アプリの終了・OS のセッションの終了・記憶を書けない失敗の知らせ・読み取り専用の起動・記録の履歴・投げ込み・画面を出さない取り出し（`--offline-dump`）も持つ。**想定した負荷**: 利用者 1 人のデスクトップ・UI スレッド 1 本・毎秒約 60 回の巡回・ゴースト 1 体（キャラクターの窓 2 枚）・SHIORI の DLL を同期で呼ぶ・`--offline-dump` は 1 回きりの CLI。この負荷で壊れる所は無かった（指摘 2 は負荷でなく入力で壊れる）。

各指摘は「場所｜何が問題か（具体例）｜直し｜放っておくと」。末尾の［ ］は重なる預かりの件・外への約束・議題の番号（9 節）。

### 必ず直す（2 件）

1. **記憶のファイルが使えないときに、効かない直し方を出す** — `areka-session/src/alert.rs:88`・`:209-225`（`write_failure.rs:55-58` も）｜`sylphya.toml` が TOML として壊れている・`format-version` が違う・読めないとき、sylphya は理由を `PersistFailureCause::Io(文)` で返す（`areka-sylphya/src/persist/mod.rs:436-452`・記録 `persist_existing_unusable`）。知らせには「理由: …中身を消さないように書くのをやめました」の次に、どの理由でも `MEMORY_WRITE_FIX`「直し方: この置き場を書ける状態にしてください」が並ぶ。置き場はもともと書けるので、利用者が従っても直らない。型は `NoRoot`・`Io(String)` の 2 つだけで（`persist/mod.rs:207-212`）、今は文字列でしか見分けられない｜sylphya に理由の型（例 `PersistFailureCause::Unusable(String)`）を 1 つ足し、作るのは `mod.rs:451` の 1 か所だけにする。`alert.rs:220` の match に腕を足して「ファイルを直すか、別の名前に変える」向きの文を出し、置き場が書けないときの今の文は変えない。`write_failure.rs:55` は理由の文をそのまま記録する。`alert_memory_write_tests.rs` に 1 本足す｜起動の時点（起動中の印を書けず `main.rs:276` の `stop_boot`）か最初の保存で areka が終わり、利用者は効かない直し方をくり返す。［預かり 5.1・約束: 知らせの文・議題 2］
2. **`--offline-dump` が遠くの element 1 つで巨大な下地を確保して落ちる** — `crates/areka/src/offline_dump/render.rs:24-33`・`:98`、`offline_dump/mod.rs:217`｜下地は `shell_bounds`（全部の面の外形の和・`areka-emo-compose/src/plan_bounds.rs:43-76`。上限なし）から作り、面ごとに `vec![0u8; cw*ch*4]` を確保する。合成の側は 1 辺 16,384（`plan::MAX_EXTENT`）で止めるが、外形には止めが無い。例: どれかの面に `element1,overlay,x.png,100000,100000` があると、その面の合成は失敗するが外形には入り、下地は約 10 万×10 万になる。ほかの面ごとに約 40GB を確保しようとして失敗し、プロセスが abort する。終わりの info・CSV・決まった終了コード（0/1/2）はどれも出ない（読みから。回して確かめていない）｜`surface_stage` で、下地の幅か高さが合成と同じ上限を越えたら `error!` を 1 行出し、全部の面を失敗として返す（条件 1 つ）。越える外形の検体で終了コードが決まった値になることを見るテストを 1 本足す｜ゴーストを検査する CLI が、手を入れていない利用者のゴーストで記録も残さずに落ちる。［議題 3］

### 直したほうがよい（74 件）

**A. 本番の失敗の道・要らない本番のコード**

3. **UI 基盤の初期化の失敗が記録も告知も無く終わる** — `main.rs:196`｜`WinApp::with_exit_policy(..)?` は、wintf の `init_ui_com` が `RPC_E_CHANGED_MODE` 以外の `CoInitializeEx` の失敗を記録せずに `Err` で返す（`wintf/src/runtime/mod.rs:115-127`・`:170`）。release は `windows_subsystem = "windows"` なので標準エラーも見えない。ほかの起動の失敗（167・175・250・276 行）はどれも告知と `error!` を出す｜`.inspect_err(|e| tracing::error!(event = "winapp_init_failed", error = %e, …))?` の 1 行を足す｜まれな起動不能の原因がログに 1 行も残らない（「ログ無し失敗経路の禁止」の違反）。
4. **`areka-session` の記録の文が `[main]` を名乗る** — `areka-session/src/session_mark.rs:113`、`boot_seam.rs:90`・`:113`｜出所は `areka_session::session_mark`・`areka_session::boot_seam` で、`session_mark_untouched_argv` は OS のセッションの終了（`session_end.rs:144`）からも出る。同じ event 名を `main.rs:523-526` が起動の時点で別の意味に使う｜文の頭を `[session_mark]`・`[boot_seam]` にする（読み手を grep で確かめてから）｜実機のログで出所を取り違える。［約束: ログの行の語・議題 4］
5. **SHIORI の試験用の脳 8 ファイルは本番で動かない** — `shiori_host.rs`・`shiori_host_case_fold_tests.rs`・`shiori_session.rs`・`shiori_demo.rs`・`reference_brain.rs`・`shiori_e2e_tests.rs`・`shiori_lifecycle_e2e_tests.rs`・`shiori_reference_e2e_tests.rs`（計 2,767 行）、`main.rs:24-39`・`:72-89`・`:234-238`｜既定の起動では `main.rs:236` が `AREKA_SHIORI_DEMO` を 1 回読み、debug を 1 行出して戻るだけ。本番の役目はほかにある（6 節 (1)）｜8 ファイルと `main.rs` の該当 約 39 行を消す。外から受け口を使う 2 か所（指摘 14 と範囲の外の `mcp/get_property_tests.rs`）を先に直す。`windows-core`（`Cargo.toml:48`）は使い手が残らないので外す｜出荷しない約 2.8k 行と 40 本のテストが、消えた名（`Load`・`Unload`）を書いたまま残る。［預かり 5.20・要件 1.8・約束: `AREKA_` 環境変数・議題 1］
6. **`ShioriSession::from_parts` が受け口の型を確かめずに `unsafe` で取り出す** — `shiori_session.rs:155-162`・`:269-277`｜safe の公開関数がどの `IShioriHost` でも受け取り、`host_sink()` が `AsImpl::<ShioriHostSink>` で取り出す。`ShioriHostSink` 以外を渡すと、遅延の答えと `Drop` のときに別の型のメモリを読み書きする（UB）。SAFETY の文「`activate` 内で構築した」は `from_parts` の道では成り立たない。綴りも `// Safety:`（`shiori_host.rs:593`・`shiori_session.rs:274,387`・`shiori_demo.rs:58,67,94`）で、注釈の無い `unsafe` も多い（`shiori_session.rs:567`・`reference_brain.rs:292-463` の 15 か所・e2e 3 本の 4 か所）｜指摘 5 で消す。残すなら `from_parts` の引数を `ShioriHostSink` にし、`// SAFETY:` をそろえる｜安全なはずの呼び出しで、追いにくいメモリ破壊が起きうる。［議題 1］
7. **脳のまわりの説明が今の ABI と違う** — `main.rs:25`・`:29-30`（`Load`・`Unload`）・`:37`・`shiori_demo.rs:3`（activate→）・`shiori_session.rs:110`（「from_parts 経由（demo/main）」）・`shiori_host.rs:65-66`（委譲の二重）・`reference_brain.rs:10`・`:248-249`（経緯）・`shiori_demo.rs:26,37`（`DemoError::Timeout` の `#[allow(dead_code)]` と「先行定義」。実際は 160・181 行で使い、意味は「期待した知らせが無い」）｜指摘 5 で消えるものは消す。`main.rs` だけ残る場合は該当の doc 行を消す｜`Load`/`Unload` が在ると誤読される。［議題 1］
8. **使われていない依存 `async-io`** — `crates/areka/Cargo.toml:53`｜`git grep -n -w "async_io\|async-io" -- crates/areka` は Cargo.toml の 1 行だけ（`areka-session` は自分で持つ）｜53 行を消す（`Cargo.lock` は `areka-session` が引くので変わらない見込み）｜次の依存の見直しでまた数え直しになる。［預かり 5.19］
9. **テストだけの `areka-user-data` が本番の表にある** — `Cargo.toml:37-39`｜使い手は `session_tests/readonly_run_tests.rs:27` の 1 本だけ｜説明ごと `[dev-dependencies]` へ移す（出荷物は `areka-session` が引くので変わらない）｜本体が書ける根を直に扱うように読める。［預かり 5.19］
10. **例だけの `tracing-subscriber` が本番の表にある** — `Cargo.toml:52`｜使い手は `examples/` の 3 本だけ。本番の初期化は `areka_session::log_history::init`｜`[dev-dependencies]` へ移す｜本体が記録の出口を自分で組むように読める。［預かり 5.19］
11. **依存の説明が今の使い手と合わず、spec 名・要件の番号・経緯の語を書く** — `Cargo.toml:24-26`・`:62-63`（placement の採寸＝今の使い手は `offline_dump`）・`:66-68`（runtime 統合結線＝今は `mcp`・`offline_dump`・`install`・`update`・`menu`）・`:74-76`（runtime の実 sink＝今は `mcp/dump_surface.rs`・`offline_dump/mod.rs`）・`:80-86`（使い手に別の crate のファイルを挙げる）、経緯と番号の行 24・28・31・34・37（`areka-P0-user-data-root・要件 4.6`）・40・42・54・59・63・66-68・75・81・88・97・101・104・106・113｜各行を「今の使い手とその依存が要るわけ」の 1 行にする｜次に依存を見直す人が、消えた使い手を手がかりに数え違える。［預かり 5.19］
12. **マニフェストの写し 2 つに、食い違いを止める仕組みが無い** — `crates/areka-session/tests.manifest`・`crates/areka/areka.manifest`｜中身の差は 2 行目のコメントだけ（XML の要素はバイト単位で同じ）。片方だけ版を変えると、テストの exe と本番の exe が別の申告で走る｜`areka-session/build.rs:17` の読み先を `../areka/areka.manifest` にして写しを消す（−9 行）か、2 行目を除いて同じであることを見る見張りを 1 本置く｜本番だけ壊れる申告の変更を、テストが見逃す。［預かり 5.18・議題 7］
13. **crate の README が古い** — `crates/areka/README.md:7`・`:9`・`:15-19`｜「Version 0.0.1」（今は 0.1.0）・「published for name reservation purposes」（今は `publish = false`）・「Implemented in the alpha (2026-10-02)」の一覧（10-02 のまま・「alpha」は使わない語）｜版の行と一覧を消し、根の README へ案内するだけにする（crates.io の 0.0.1 が名の確保だけ、の 1 文は残す）｜crate のページを読んだ人が古い版と機能を信じる。

**B. 落ちる力の無いテスト・守りの抜け**

14. **「ゴーストの道」と名乗るテストが本番の道を通らない** — `session_tests/ghost_session_scope_scale_boot_tests.rs:4`・`:108-132`（`mcp_and_ghost_paths_answer_the_same_scale`）｜`ShioriHostSink` の `GetProperty` を「ゴーストの道」として MCP の答えと比べる。本番のゴーストは `\![get,property]` の受け口 `areka_ghost::prop_get_sink::PropGetCueSink` か SHIORI4 DLL の InProcHost で読む。例えば `PropGetCueSink` の問い手を取り違えてもこのテストは緑｜`ghost_path` を `PropGetCueSink::new(reader, ghost_asker_id(..), tx)` で組み、合図を 1 つ流して `KanadeMsg::PropertyEvent` の references を読む形にする（無い値は本番どおり `""`）｜本番の道が壊れても落ちない。これを直すと指摘 5 の受け口の範囲の中の最後の使い手が外れる。［議題 1］
15. **旗の生産者を探す走査が、本体から分けた crate を歩かない** — `tick_gate_config_producers_tests.rs:184-197`・`:236`｜`walk_roots` は本体・`areka-runtime`・`areka-input` の 3 つだけで、`areka-session`・`areka-placement` を歩かない。例: `areka-session/src/ghost_session.rs` に `tick_wake::mark(PRESENT)` を足して表に載せなくても緑で、後でその呼び出しが消えても誰も気づかない（話している途中で表示が止まる形）。184 行は経緯の文、236 行は `areka-input` を書き落とす｜配列に 2 つ足し、説明は歩く根の名を並べた文にする｜名簿の完全さを守る見張りに黙って抜ける穴が残る。
16. **読み直しのテストで本体の口を据えるのが 2 本だけ** — `session_tests/reload_tests.rs:142`・`:199`（試験台 `rig()` は 47-69）｜本体のテストでは `areka-session` が `cfg(test)` なしで組まれるので、`BodySystems::of` の `#[cfg(not(test))] error!(body_systems_missing)` が生きている（`body_systems.rs:50-52` で確かめた）。据えない残りの 8 本は、`request_reload` を通るたびに「結線の誤り」の error を 1 件出している（数えていないので緑）｜`rig()` に `world.insert_resource(crate::BODY_SYSTEMS);` を 1 行足し、142・199 行を消す｜新しいテストで warn 以上を数えると理由の分からない 1 件のずれで赤になり、本物の据え忘れが見分けられない。［預かり 5.16］
17. **束の無い World の受け口の数の表明が常に真** — `session_tests/body_systems_tests.rs:55`・`:57`（説明 :38）｜戻りの型が `[Box<dyn BootCueSink>; 2]` なので長さはいつも 2。受け口の中身が `DiscardSink` から何に変わっても緑｜`len()` の要素と「受け口は 2 つで答える」を消す（5.17 で揃えると決まったら、そのときに中身の振る舞いを見る）｜「窓口が無いときと同じ」と読める名のまま差を見ていない。［預かり 5.17］
18. **構造体に入れた値を読み返すだけのテスト 2 本** — `session_tests/boot_config_input_tests.rs:4-13`・`:426-466`｜`config_inputs_holds_resolved_roots`・`boot_context_holds_the_resolved_current_ghost` は本番がどう壊れても赤にならない。意味のある値は `:296-316` と `boot_config_roots_tests.rs:99-124` が見ている｜2 本とも消す｜守っているように見えるだけのテストが残る。
19. **印を書けたかを捨ててから「印が無い」を表明する** — `session_tests/boot_config_option_tests.rs:116`・`session_end_tests.rs:97`・`:124`｜`let _ = write_session_mark(..)` の後で None を表明する。置き場に書けず印が最初から無くても緑｜`.expect("印を書く")` にする（揃えるなら Some を期待する所 `boot_config_callghost_tests.rs:97`・`boot_config_option_tests.rs:82,127,391`・`boot_config_input_tests.rs:362,402` も）｜印を読まない・消すという判断が壊れても、書けない環境では気づけない。
20. **全 crate を歩く見張りが `crates/<x>/src` の 1 段しか見ない** — `session_tests/readonly_body_tests.rs:117`・`:125-147`｜説明は「ワークスペースのクレートの源（試験も含む）を全部見る」だが、`crates/*/tests/`・`examples/`・入れ子の crate（`crates/pilot/examples/*/src`）を見ない。例: `crates/areka-ghost/tests/x.rs` が `withhold_everything(` を呼んでも緑｜5.2 の移しで `workspace_scan::walk_workspace_sources`（`crates/**`）に替える（今の当たりの増えは 0 件）｜「試験のプロセスでは旗を立てない」約束の抜け道が残る。［預かり 5.2・議題 6］
21. **期限つきの降ろしの見張りの説明が範囲と食い違う** — `session_tests/session_end_deadline_body_tests.rs:28-31`・`:112-113`（`:11-12` も）｜4 crate だけを歩くのに「本番ソースで…呼ぶのは 3 行だけ」と書き、`areka-ghost/src/runtime.rs:285` の `shiori_probe.arm(budget)` を数えていない。理由の「分ける前に」は経緯の語｜説明を「呼び手の側の 4 crate を歩く（`areka-ghost` の実装の中は数えない）」にする｜読み手が「ワークスペースに 3 行だけ」と誤解する。
22. **2 周のテストの説明が古く、余分な登記が何も確かめていない** — `session_tests/ghost_session_restart_tests.rs:446-447`・`:479-491`｜説明は「組込 2 項目と『ゴースト』枠だけ」「7 枠すべて」、表明（556-586）は 12 枠。481-491 の余分な「シェル」枠の登記は、本番の `wire_body_after_runtime`（`main.rs:397-410`）が毎回登記し直すので落ちる力を足さない｜481-491 とそこでしか使わない use を消し、説明を「12 枠」「2 周目の登記が置き換えを 1 件も残さない」に直す｜効かない足場が他のテストへ写される。
23. **「準備だけでは窓が生えない」の対照が走る順の偶然に頼る** — `session_tests/ghost_session_restart_tests.rs:156`・`:162-171`｜作業プールは `WintfTaskPool::new()`（スレッド複数）。準備が誤って窓のタスクを投げるようになったとき、目印のタスクが先に着けば 0 枚のまま緑｜このテストだけ `WintfTaskPool::with_threads(1)` にする（rect・scale の試験台と同じ）｜後戻りを回すたびに見逃したり捕まえたりする（決定論の決まりに反する）。
24. **組 {0,1} の判定が字の有無しか見ない** — `tests/window_link_real_run.rs:473-479`｜行が `descript_seed` と字 `'0'`・`'1'` を含むかで見るが、行頭の時刻（`2026-11-…`）にいつも 0 と 1 がある。組が `[{2, 3}]` でも通る｜`groups=` の欄の値（`sticky_group_ledger.rs:174`）に `{0, 1}` が入るかで見る｜組の取り違えがこの判定を通り抜ける。
25. **実画面のテストの判定の前の待ちが決め打ちの時間** — `tests/stay_on_top_real_run.rs:593`・`:778`・`:795`・`:811`・`:834`・`:855`・`:919`、`tests/window_link_real_run.rs:481`・`:527`・`:600`・`:607`・`:634`・`:665`・`:743`｜台本の後に 0.8〜6 秒眠ってから重なりや矩形を読む。混んだ回で `\![set,zorder,1,0]` の後 1 秒に並びが替わらなければ FAIL。待つべき記録の行は `RUST_LOG` で開けてある｜それぞれの記録の行を `wait_new` で待つ形に替える｜赤のたびに人が排他の机を取り直して回し直す。
26. **smoke の待ちの上限が窓口の既定 60 秒のまま** — `tests/smoke_boot_loop_exit.rs:126-135`（コメント 130・242）｜保険の自動終了 500ms は最初の窓の準備から数える（`areka-session/src/smoke_exit.rs:10`）ので、60 秒の中身はほぼ emo2 の起動（i686 の仲介と pasta の読み込み）と終わる相。このバイナリの 5 本は並んで走り、emo2 の起動 3 本が同時に動く。同じ子を立てる emo2 のテストは 120 秒（`emo2_real_run/common.rs:13`）｜`smoke()` に `.deadline(Duration::from_secs(120))` を足し、130・242 行を合わせる（値は測っていない）｜混んだときだけ締切切れで落ちる揺れが続く。［仮定 11］
27. **書き出しの exe のテストに待ちの上限が無く、開発者の根を受け継ぐ** — `tests/offline_dump_exe_test/common/mod.rs:124-139`｜`Command::output()` で上限なく待ち、`AREKA_ROOT`・`AREKA_PROFILE_DIR`・`AREKA_USER_ROOT`・`LOCALAPPDATA` は開発者の値のまま。例: `failure_branches_test.rs:179` の値なしの `--offline-dump` の読み方が変わって通常の起動に落ちると、exe は開発者の根でゴーストを起こし、窓を開いたまま終わらない｜`.envs(envs)` の前に `AREKA_APP_SMOKE_EXIT_MS` と `cwd` の下を指す根を既定で置く（呼び手が上書きできる順）｜後退がハングと開発者の記憶への書き込みとして出る。
28. **記憶を書かない守りの 2 つの分岐にテストが無い** — `areka-session/src/boot_resolve.rs:729-737`（`last_used_folder_missing`）・`boot_seam.rs:108-116`（`last_used_shell_folder_missing`）｜2 つの event 名はテストに 0 件。データの書き込みを止める分岐｜`GhostDecision { route: Memory, folder: None, .. }` で何も書かれず warn 1 件、`record_last_used` に `Path::new(r"C:\")` で何も書かれず error 1 件、を見る 2 本｜空の名を書く退行が赤にならない。
29. **記憶の書き手の待ちが panic を捨て、上限なく待つ** — `areka-session/src/boot_resolve_tests.rs:482`・`:575`・`:756`・`:804`、`boot_seam_persist_wiring_tests.rs:80`・`:123`｜`let _ = parts.handle.join();` は `ActorError::Panicked` を捨てる｜`areka_runtime::spine::join_bounded("sylphya", parts.handle).expect(..)` にする（型は合う・dev の `areka-runtime/test-support` で届く・本体の `switch_finish_tests.rs:300` と同じ形）｜書き手が落ちる退行が赤にならず、詰まれば机でテストがぶら下がる。［預かり 5.7］
30. **起動中の印の判定の表が判定を持つ crate に無く、本体のファイルが 1,000 行に迫る** — `main_session_mark_tests.rs:393-637`・`main_session_mark_write_tests.rs:259-297`｜`session_mark_verdict_table`・`settle_session_mark_applies_the_verdict`・`clear_mark_unwritable_tells_running_once` は本体の部品を使わない。`session_mark.rs` にはテストが 0 本で、`cargo test -p areka-session` では判定の退行が赤にならない。`main_session_mark_tests.rs` は 959 行｜`areka-session/src/session_mark_tests.rs` へ移す（約 715 行に減る）｜次に数十行足すと番人を越える。
31. **例が片方だけ書かれたバルーンの位置の調整を捨てる** — `examples/emo-present/balloon.rs:40-71`（説明 :10-11・:44、`emo-present.rs:19-22`）｜`(Some(x), Some(y))` のときだけ返す。配置の crate は書かれない軸を 0 にして読む（`areka-placement/src/config.rs:261-275`）。`sakura.balloon.offsetx,30` だけなら本番は (30,0)、例は既定のまま｜`(ox.is_some() || oy.is_some()).then(|| (ox.unwrap_or(0), oy.unwrap_or(0)))` にし、:44 の文を消す｜手元の目視の確かめが製品と違う結論を出す。［預かり 5.21 件 1］

**C. 写し（一緒に直すべきもの）**

32. **`SharedFakeIo` の写し 3 つ** — `areka-session/src/boot_resolve_tests.rs:419-429`・`boot_seam_persist_wiring_tests.rs:11-21`・本体の `runtime_tests/frame/switch_finish_tests.rs:60-61`｜`areka_placement::test_support::SharedFakeIo`（`test_support.rs:46`）と振る舞いが同じ。説明の「persist.rs の write-through 檻と同流儀」も古い｜写しを消し `SharedFakeIo::new(Arc::new(FakePersistIo::new()))` を使う（両 crate の dev で `areka-placement/test-support` は開いている）｜写しが増え、直すときに漏れる。［預かり 5.5］
33. **`code_only` の写し** — `tick_gate_config_producers_tests.rs:67-73`｜`areka_runtime::source_text_test_support::code_only`（実体 `areka-placement/src/test_support.rs:382`）と 1 字も違わない（引数の名だけ違う）｜6 行を消して use する｜片方だけ直すと判定が分かれる。［預かり 5.6］
34. **`prop_set_cue` の写し 3 つ** — `session_tests/readonly_run_tests.rs:95-105`・`readonly_sites_tests.rs:119-129`・`write_failure_sites_tests.rs:142-153`｜`areka_runtime::consumer_ledger::test_support::carrier_cue(name, args)` と違うのは演者（"0" と "sakura"）だけで、`PropSetCueSink::emit` は演者を読まない（`areka-ghost/src/prop_sink.rs:47-`）。本体の dev の `areka-runtime/test-support` で届く｜3 つとも消して `carrier_cue(PROP_SET_CUE_NAME, &[&key.to_canonical_key(), value])` を使う｜合図の形が変わると 3 か所を直す。［預かり 5.13］
35. **本体の結線ハーネスの写しと、写す理由の古い説明** — `runtime_tests/spine.rs:92-104`（`make_world_with_gpu`）・`:119`・`:123-126`（`capture_logs`）｜119 行は「向こうの `capture_logs` は `cfg(test)` だけで立つため本体側からは読み込めず」と書くが、今は `areka-runtime/src/frame.rs:255-257` の `#[cfg(any(test, feature = "test-support"))] pub mod test_support` で、`frame_test_support.rs:117` の `capture_logs`・`:921` の `gpu_on` は本体から届く。違いは `gpu_on` が COM を MTA で初期化する（プロセスの MTA の中では `S_FALSE` で通る）ことと、`AppExit` を据えないこと｜`capture_logs` を use に替え、`make_world_with_gpu` は `gpu_on` を呼んだ後に `AppExit` を据える 3 行にする｜写しと理由の説明がずれていく。［預かり 5.8］
36. **`BootContext` を組む補助の写し（この spec の分）** — `menu/ghost_frame_tests.rs:43`｜同じ型の写しが範囲の外に 3 つ（`install/desk_tests.rs:730`・`areka-session/src/ghost_switch_tests.rs:84`・`readme/opener_submit_tests.rs:31`）。`areka-session/src/reload_test_support.rs:17` の `boot_context(temp, root)` は形が違う（今のゴースト "A" 固定・バルーン "kaku"・置き場 `temp/profile`。こちらは引数のゴースト・"StayseeBalloon"・`root/profile`）。ghost_frame の分は `balloon_frame_tests.rs:14`・`shell_frame_tests.rs:19` も借りる｜仮定 6 のとおり、形が合わないので寄せずに理由を 1 行残す（まとめ直しは 3 本の着地の後）か、`reload_test_support` に `(root, current)` の形を 1 つ足して寄せる｜写しが 5 つのまま育つ。［預かり 5.9・議題 10］
37. **読み取り専用・書けない場面の行列のテストの補助の写し** — `tree`（`readonly_run_tests.rs:53-70`＝`readonly_sites_tests.rs:47-64`）・`read_back`（`:73-82`＝`:67-76`）・実行系の記憶の書き手の取り出し 4 か所（`readonly_run:84-93`・`readonly_sites:131-140`・`write_failure_sites:171-179`・`boot_resolve_mark_text:94-102`）・`unwritable`／`writable`（`boot_resolve_mark_text:44-57`＝`write_failure_sites:69-82`）・`routes`（`readonly_sites:93-97`＝`write_failure_sites:117-121`）・送り出しの段の予約（`write_failure_sites:123-140`＝`session_end_tests:183-200`）・通すだけの `drain_desk`（`write_failure_sites:63-66`）｜`session_tests/mod.rs` か `sites_tests` に `pub(crate)` で 1 つずつ置き、`drain_desk` は消して `drain_system` を直に呼ぶ｜木の比べ方が片側だけずれると「木が変わらない」の判定が弱くなる。
38. **起動の構成のテストの検体の作り方の写し** — `put_memory`（`format-version = 1` の手書き・`boot_config_balloon_tests.rs:51-63`・`boot_config_input_tests.rs:201-213`・`boot_config_roots_tests.rs:75-87`）・`save_scope`（`callghost:36-44`・`option:42-50,307-315`・`input:330-338`）・`put_ghost`（`callghost:18-22`・`option:23-27`）・`no_pick`（7 か所）｜`session_tests/mod.rs` の `resolve_boot_from` の隣に 1 つずつ置く｜記憶の書式が変わると手書き 3 か所が黙って古い書式のまま残る。
39. **会話の場のテストの試験台と小道具の写し** — `ghost_session_scope_rect_boot_tests.rs:26,30-45,48-56,83-95`＝`ghost_session_scope_scale_boot_tests.rs:27,46-61,64-74,89-101`＝`ghost_session_switch_tests.rs:167-195`（`A_TO_B`・`ghost_a`・`ghost_b`・`lap_rig`・`ask`）、`systems_lens`（`restart_tests.rs:434-441`＝`switch_tests.rs:198-205`）、停止通知の受け口の loop（`restart_tests.rs:522-530`＝`switch_tests.rs:241-246`）、`emo2_balloon`（`ghost_session_reload_tests.rs:28-33`＝`restart_tests.rs:389-394`）、`scripted_wiring`／`scripted_inputs`、`kinds`（`switch_tests.rs:31-40`＝`areka_session::ghost_session::shell_balloon_switch_session_lap_test_support::kinds`）｜`switch_tests` の側を `pub(super)` にして使い回し、`kinds` は公開のものを use する｜台本や受け口が変わったとき片方だけ直る。
40. **`headless_wiring` の写し** — `areka-session/src/reload_skin_tests.rs:52-79`｜本体の `session_tests/shell_balloon_switch_tests.rs:78`（範囲の外）と 1 字違わない。読み直しは `RuntimeWiring` の有無しか見ない（`shell_balloon_switch.rs:289`）｜`areka_runtime::frame::shared_test_support::wiring().1` を据える 1 行にする｜`BootAssets` に欄が増えるたびに手で直す。
41. **`fake_handle` の写し** — `areka-session/src/ghost_session_scope_rect_test_support.rs:16-20`｜`areka_placement::test_support::fake_handle`（`test_support.rs:81`）と同じ。使い手は `ghost_session_scope_rect_tests.rs:338` と本体の `session_tests/ghost_session_scope_rect_boot_tests.rs:17` だけ｜写しを消して直に使う｜同じ。
42. **png のテストが一時フォルダの道具を写す** — `png_image_tests.rs:146-163`・`:214-224`｜自前の `TargetTemp` は `temp_path_kit::TempPath::new` と同じ仕事。札 `dump-surface-image-wic` は mcp から移る前の古い名｜`TempPath::new("png-wic")` にして 18 行を消す｜後始末の規則が道具とずれる。
43. **全 crate を歩く見張りの自前の走査器** — `areka-session/src/session_end_sync_send_tests.rs:294-378`｜自前の `workspace_production_sources`・`sync_send_sites`（`//` の行だけ飛ばす・ブロックコメントと行末コメントは数える）は `log-capture-kit/tests/workspace_scan/mod.rs` の `walk_workspace_sources`・`scan_sources`・`scan_tokens` と同じ仕事｜静的な後半（221-499）を `crates/log-capture-kit/tests/` の新しい見張りへ移す（「テストのモジュールの中か」の欄と鍵ごとの件数の突き合わせは見張りの側に残す）。前半の再現は試験台が要るので残す。`session_end.rs:53` と `crate_source_test_support.rs:6-7` の例外の列も直す｜走査の規則が見張りごとにずれる。［預かり 5.2・議題 6］
44. **起動中の印の書き込みのテストの World の写し** — `main_session_mark_write_tests.rs:23-53`｜`main_session_mark_tests.rs:67-98` の `world_with` を A・`Memory`・単位 1 つに固定した写し｜親の子モジュールにして `world_with(..)` を呼ぶ（fallback と同じ形）｜`BootContext` の欄が増えるたびに 2 か所を直す。
45. **実画面のテストの補助の写し** — `per_monitor`・`mouse`・`copy_dir`・`lines_with`・`wait_new`・`json`・`Notes`（`stay_on_top_real_run.rs:151-155,278-301,389-413,507-518`＝`window_link_real_run.rs:63-67,103-127,199-236,765-768`、`copy_dir` は `startup_install_real_run.rs:31-43` にも）｜2 本の `copy_dir` はパスなしの `unwrap()`、startup の写しはパスと理由で落とす｜`tests/real_screen/common.rs` へ寄せて `#[path]` で引く（`copy_dir` は startup の形を残す）。`stay_on_top_real_run.rs` の 966 行も減る｜片方だけ直る・番人を越える。
46. **本体の部品を使わないテストが本体に置かれている** — `session_tests/boot_config_module_policy_tests.rs:42-55`（2 本）・`boot_config_balloon_tests.rs:98-255`（7 本）・`boot_config_input_tests.rs:226-262`（3 本）・`boot_config_roots_tests.rs:127-229`（4 本）・`boot_resolve_mark_text_tests.rs:175-234`（2 本）・`session_end_tests.rs:204-343`（4 本）｜`session_tests/mod.rs:1-4` の置き場の決まり（「本体の部品も使うもの」）に合わない。`areka-session` の口のテストが 2 つの crate に散る｜`areka-session` の同じ口のテストへ移す｜`areka-session` だけを直す人が見落とす。

**D. 事実と違う説明（消えた名・古い置き場・数）**

47. **記憶からの戻しの継ぎ目の名が古い** — `areka-session/src/ghost_session.rs:213-214`・`boot_seam.rs:156`（同じ件 `boot_seam.rs:7`・`:136`）｜「`load_restored_state` → 純関数 `apply_restored_placements`」と書くが、本当の流れ（`boot_seam.rs:22-36`・この書き手が確かめた）は `load_restored_state` → `restore_and_judge` → `apply_balloon_limit` → `restored_scope_marks`。`apply_restored_placements`（`areka-placement/src/persist.rs:344`）は生きているが起動の道では呼ばれない（本番の呼び手は `reseed.rs:73` だけ）。下請けの 1 組は「生きた関数なので古くない」と報告したが、名指す流れが違うので古い。`:7`・`:136` は呼び手を `open_ghost_windows` と書くが、`restore_merged_placements` を呼ぶのは `prepare_ghost_windows`（`ghost_session.rs:230`。`open_ghost_windows` はそれを呼ぶ）｜2 か所を今の流れに書き換え、呼び手は `prepare_ghost_windows` にする｜呼ばれない関数を追い、バルーンの制限の段を見落とす。［預かり 5.3］
48. **バルーンの結線が挿すものの数え違い** — `ghost_session.rs:821-822`｜`BalloonWiring`／`ChoiceSelectionInbox` の 2 つと書くが、`areka-input/src/balloon.rs:407-412` は `BalloonWiring`・`BalloonHover`・`ChoiceSelectionInbox`・`ShellBoxHover` の 4 つを挿す｜4 つを並べる｜切替のときに hover の状態も新しくなることが読めない。［預かり 5.10］
49. **消したテストを名で指す** — `areka-session/src/app_exit_escape_tests.rs:69-74`｜73 行が入力の crate の側で消えた `handler_left_double_click_without_ctrl_does_not_despawn_and_sends` を名指す（`git grep` で crates の中に残るのはこの 1 行）。70 行の「Ctrl+左ダブルクリックのいずれも」も中身と合わない｜役目（入力の crate の左右のダブルクリックの送出のテスト）で指す文にし、70 行を「非退避操作のいずれも」にする｜無いテストを探す。［預かり 5.10］
50. **入力の口についての事実と違う説明** — `areka-session/src/app_exit_tests.rs:182-183`｜「送り口は `pub(crate)` の構築関数」（`MouseWiring::new` は `pub`・`areka-input/src/lib.rs:102`）・「`world_with_wiring` は私有で流用できない」（公開の `areka_input::pointer_test_support::world_with_wiring` が在り、`app_exit_escape_tests.rs:15` が使う）｜括弧の文を消す｜同じ補助を書き直す人が出る。［預かり 5.10 の件 2 の残り］
51. **`plant_minimal_ghost` の説明が古く、要らないシェルを置く** — `areka-session/src/boot_seam_restore_tests.rs:24`・`:34`・`:118`・`:11`｜persist.rs の `plant_minimal_ghost` は消えた。`load_restored_state` は `read_ghost_names` でゴーストの descript を読むだけでシェルを解かない（`persist.rs:284-303`）。merge のテストは `persist_restore_tests.rs`｜説明を「`load_restored_state` が読むゴーストの descript だけを置く」にし、:34 の `shell/master` を消す。:118・:11 も直す｜シェルを解く形へ戻る退行をどのテストも見つけない。［預かり 5.4］
52. **バルーンのはみ出す向きのコメントが逆** — `boot_seam_restore_tests.rs:212`｜(30,300) にずらし −50 でバルーンは x=−20〜180。はみ出すのは左で、表明（202・238 の x=0）とも食い違う｜「左の辺を 20px はみ出し、0 へ寄せられる」にする｜補正の向きを読み違える。
53. **`areka-session` の本番の古い説明（まとめ）** — `lib.rs:54`（「`pub(crate)`」＝実際は `pub mod`）・`lib.rs:57`／`boot_resolve.rs:3`（「ゴースト 6 分岐」＝今は 9 通り。`:166` の段の並びにも callghost が無い）・`boot_config.rs:6-8`（実在しない `boot_config_input_tests.rs` と曖昧な `runtime` を挙げる使い手の表）・`:425-426`（同じ文が 2 行・1 行目が途切れる）・`:468`（`default_helper_exe_path` を「純粋」＝`exe_location()` は fs を読む）・`:588`（「窓の無い起動」＝`ghost_session.rs:725` は窓を開いた後の倒れ先）・`:616`（`tests/smoke_boot_loop_exit.rs` は本体の crate）・`boot_resolve.rs:590`（呼び手 `main::settle_session_mark`＝今は `session_mark::settle_session_mark`）・`boot_seam.rs:51`（前とつながらない文の切れ端）・`ghost_session.rs:87`・`:147-149`（欄は 3 つなのに「どちらも」）・`:283-285`（example の `#[path]` include の理由＝`git grep "#\[path" -- crates/*/examples` 0 件）・`:640-643`（seriko の送り手「3 本」＝ 613 行の束が 4 本目）｜コメントだけ直すか消す｜終了の順序の要の段を読み違える・無いファイルを探す。
54. **本体の入口の `//!` が事実と違う** — `main.rs:5`（本番アプリを「ぱすたさん」のもの＝既定のゴーストは emo2）・`:13`（「`placement` モジュールが所有」＝今は crate `areka-placement`）・`:18`（「boot 失敗は非致命…止めない」＝起動中の印を書けなければ 276 行で止める）・`:308`（後始末の列に 342 行の「書いている最中の仕事を待つ」が無い）、同じ種類が `examples/mock-shell.rs:3`・`:102`｜今の事実に直す｜終了の順を誤解する。
55. **終了の層の古い説明** — `areka-session/src/app_exit.rs:212`（直前の 211 行で `CharDragSession` を外した直後に「areka が外す物は無い」）・`:311-312`（「`placement` は `crate::` パスを持てない（example の `#[path]` include）」＝今は依存の向き）・`:174`（「`crates/areka/src` の資源の挿入を洗い出した」＝今は 3 crate）｜212 を消し、311-312 は依存の向きで、174 はパスを消す｜読み手が外すべき物を見落とす。
56. **終了の待ちの説明が古い** — `areka-session/src/exit_wait.rs:8-9`・`:17`｜「install も update も…」＝門は 4 つ（install・update・`reload.rs:142`・`shell_balloon_switch.rs:63`）。`EXIT_WAIT_LIMIT` を「展開の終わりを待つ上限」＝ `main.rs:292` は全部の門に使う｜「どの背景の仕事も」「背景の仕事の書き終わりを待つ上限」にする｜門を足す人が待ちを見落とす。
57. **表明の文が事実と違う** — `areka-session/src/ghost_quit_tests.rs:88`｜「通知を消化したフレームは以後の相を飛ばす」＝戻り値を読む本番は無く（`session_end.rs:106` は捨てる）、相を回さないのは窓が 0 になるから（`ghost_quit.rs:23-25`）｜「停止の通知を消化した」にする｜誤った前提で本番を変える。
58. **bevy の版と行番号が古い** — `areka-session/src/app_exit_tests.rs:65-67`｜`bevy_ecs-0.18.1` の行を挙げる＝今は 0.19.1｜版と行番号を外し「`World::despawn` は `log` の `warn!` を使う」だけにする｜無い行を探す。
59. **doc のリンクが違う物を指す** — `areka-session/src/log_history_capture_tests.rs:3`｜[`WINDOW`] は写しの窓そのもの（`log_history.rs:453`）で、テストが握るのは `hold()`（`:562`）｜「[`hold`] を握って」にする｜新しいテストが窓の Mutex を取り違える。
60. **smoke の写しの理由が古い** — `tests/smoke_boot_loop_exit.rs:52-53`｜「bin crate の定数はテストから参照できない」＝`SHIORI_FAULT_TITLE` は今は `areka-session/src/alert.rs:83` の非公開の定数｜「`areka_session::alert` の非公開の定数と同じ綴りを写す」にする｜写す理由を誤解する。
61. **本体の起動のテストの説明が古い** — `session_tests/boot_config_input_tests.rs:19`（「13 分岐」＝ 15 経路）・`boot_config_option_tests.rs:78`（「引数なしは記憶」＝印があるので既定）・`boot_config_module_policy_tests.rs:57-58`（「切替の先」＝2 回とも `BootOrigin::Plain`）・`log_history_convention_body_tests.rs:1`（「本体の `src`」＝ 3 crate と areka-ghost）・`main_session_mark_tests.rs:3-5`（`settle_session_mark` を main の関数のように書く）｜説明を今の中身に合わせる（module_policy は `ChangedFrom(..)` で呼ぶ手もある）｜説明を信じて抜けを見落とす。
62. **`boot_resolve_tests.rs` の `//!` が事実と違い、番人に近い** — `areka-session/src/boot_resolve_tests.rs:1-7`（873 行）｜「プロセス・fs・乱数には触れない」＝子の `mod last_used`（405-873）は実 fs を書き、`pick_index_stays_in_range`（397）は本物の `RandomState` を回す｜冒頭を今の中身に合わせ、`mod last_used` を `boot_resolve_last_used_tests.rs` に分ける｜並列や根の置き場を誤って判断する・次の 1 本で番人を越える。
63. **例の巡回の説明が違う・同じ行が 2 回** — `examples/emo-present.rs:16-18`・`:93`・`examples/emo-present/state.rs:19`、`examples/emo-present/window.rs:24-26`｜「surface0 → surface1000 → Hide →（反復）」＝実際は surface0 を 1 回出した後に surface1000 で bind 1400 を出し入れするだけ（`state.rs:37-61`）。93 行の「将来の kanade/seriko 結線」は本番では済み。window.rs は同じ文が 2 行｜説明を直し、93 行の将来の話と重なった 1 行を消す｜例で見る物を誤解する。
64. **起動オプションの読み取りの doc が今のふるまいと違う** — `cli/parse.rs:209`（`Repeated` は「`--option`・`--ghost` 以外の重ね」＝`--install` も値を足して出さない・`parse_install_tests.rs:48-61`）・`:41`（`Kind::Unsupported` は「正典に載っているが扱わない」＝表に無い `--dump-xxx` もこれになる・`lookup_long` 162 行）｜doc を今のふるまいに合わせる（読み方は変えない）｜`--install` の重ねに警告があると思い込む。
65. **メニューの呼び手の説明** — `menu/mod.rs:221-223`・`:254`｜関数ポインタ経由で実行の上では正しいが、名で呼び手を引くと `main.rs:362` の `BODY_SYSTEMS` しか出ない。221 行の「`ghost_session::boot_ghost` からゴーストごとに」は 2 段上の名で、読み直しの道（`ghost_session_runtime.rs:142` の `rebuild_runtime_in_place`）が抜ける｜221 行を「本体の `wire_body_after_runtime`（`BODY_SYSTEMS.after_runtime_wired`）から、起動と読み直しのたびに」、222・254 行を「`BODY_SYSTEMS.menu_poll` として `register_systems` から」にする（説明の文だけ）｜読み直しのときにも呼ばれることが読めない。［預かり 5.11・議題 11］

**E. コメントの決まり（CLAUDE.md `## Comments`）**

66. **作業番号の札つきのテストの名** — `runtime_tests/zorder_wiring_tests.rs:29`（`t_zwi05_…`）・`:74`（`t_zwi08_…`）、冒頭 4・5・10・11・13 行（`zwi05`・`zwi08`・`t_zwi06`・`zwi01／02`・`zwi06／07`・`zwi09`）｜札を外して `the_boot_wires_the_channel_the_sink_and_the_handoff`・`the_entry_point_carries_the_shell_setting_into_the_wiring` にし、冒頭は改名表の名（11 行＝`the_frame_system_is_ordered_before_the_chain_apply`、13 行＝`areka-runtime/src/zorder_wiring_tests.rs` の 93・148・184・233・282 行の 5 本）へ直す。`git grep -n "t_zwi0" -- . ':!vendors' ':!.kiro'` の当たりはこのファイルだけ（`doc/` に当たりは無い）｜番号が指す先とずれる。［預かり 5.12］
67. **設計の番号をテストの名に持つ** — `areka-session/src/alert_dpi_tests.rs:3-5`・`:36`・`:67`・`:69`・`:83`・`:85`・`:102`・`:104`・`:126`・`:145-162`（`d1_wraps_and_restores_previous_context_in_order` ほか 6 本）｜`d1_`〜`d5_` の頭と説明の「D1〜D5」を外す（外しても名は重ならない・字面で読む道具は 0 件）｜同上。
68. **名（テスト・関数）に経緯や作業文書の番号** — `offline_dump/bind_spec_tests.rs:103`（`research_10_4_thirteen_cases`）・`file_drop_mime_tests.rs:60`（`spot_checks_against_the_design_table`）・`session_tests/boot_config_option_tests.rs:80`（`without_options_the_answer_is_as_today`）・`areka-session/src/readonly_tests.rs:87`（`boot_check_without_readonly_is_the_current_check`）・`log_history_tests.rs:643`（`moved_modules_keep_their_kinds_by_real_module_path`）・`ghost_quit.rs:67`・`:81`（私有の関数 `quit_as_today`）｜中身の名に替える（例 `named_binds_on_scope_zero_cases`・`quit_without_reservation`）｜同上。
69. **`areka-session` のコメント・表明の文に要件の番号・spec 名・設計の札** — `alert.rs:87`（「user-data-root の」）・`alert_tests.rs:37`・`:43`・`app_exit_escape_tests.rs:34`・`:41`・`:116`・`app_exit_tests.rs:155`・`:162`・`ghost_quit_tests.rs:89`・`:92`・`:193`・`boot_seam_restore_tests.rs:97`・`:102`・`:136`・`:140`・`:182`・`:234`・`:239`・`:244`・`:255`・`boot_seam_persist_wiring_tests.rs:44`（「C4/C5」）・`ghost_session_scope_rect_test_support.rs:8`（「SSP の段 1 の値」）・`ghost_session.rs:228`（`windowposition-limit`）・`:354`（`shiori-fault-notice`）・`boot_config.rs:193-194`（「後続の `shell-balloon-switch`」＝着地済み・今の口は `hooks::set_current_balloon`）｜番号と spec 名を消して中身だけ残す｜`git grep` で偽の手がかりになる。
70. **本体のコメント・文字列に要件の番号・作業の札・spec 名** — `main.rs:86`（D1）・`reference_brain.rs:371`・`:400`・`shiori_reference_e2e_tests.rs:91,96,114,146,161`・`shiori_lifecycle_e2e_tests.rs:96`・`shiori_host.rs:180`（M1）（この 4 本は指摘 5 で消えうる）・`perf_thread_report_tests.rs:72`（設計 C14）・`png_image_tests.rs:219`（要件 5.3）・`session_tests/boot_config_input_tests.rs:426`（据え付けは 8.5）・`readonly_sites_tests.rs:504`・`write_failure_sites_tests.rs:412`（要件 1 の表）・`tests/stay_on_top_real_run.rs:446,467,652-941`（判定名の `8.1a`〜`8.4`・`R-bg`・`R1`〜`R3`）・`tests/emo2_real_run.rs:8,12,79,92-93,131`（`M-boot`・`DoD 非前提`・`window-placement`）・`examples/emo-present/reconcile.rs:248,256,277,285,294`（panic の文の `R6.2`・`R8.2`・`R8.3`）・`examples/clickthrough_two_rects.rs:89`（`=R5`）・`examples/mock-shell.rs:831`（`A1-V`）・`examples/emo-present/fixture.rs:24`（段 ③）｜番号と札を消し、中身の言葉だけを残す（`judgments.txt` を読む道具は 0 件）｜同上。
71. **本番のコメントに変わった経緯の語** — `areka-session/src/app_exit.rs:67`・`session_end.rs:5`（今日の）・`write_failure.rs:141`（今の）・`boot_config.rs:72`（今のふるまいへ戻る）・`:427`（今どおり）・`boot_resolve.rs:131`（今と同じ綴り）・`lib.rs:54`（`main.rs` の 1,000 行規約のため切り出した）・`alert.rs:284`・`:355`（日付）、本体の `cli/listen.rs:9`（今までどおり）｜比べる語を外し、決まった振る舞いだけを書く｜比べる相手の「今」が分からない説明が残る。
72. **テストのコメント・表明の文に変わった経緯の語** — `areka-session` の `app_exit_escape_tests.rs:8,49`・`alert_tests.rs:213`・`alert_memory_write_tests.rs:5`・`ghost_quit_switch_tests.rs:172,199,285`・`session_end_deadline_tests.rs:28,269,300`・`session_end_sync_send_tests.rs:226,230,253`・`write_failure_late_tests.rs:75,88`・`readonly_tests.rs:85,95`・`boot_seam_restore_tests.rs:113`・`boot_resolve_balloon_tests.rs:451`・`boot_resolve_option_tests.rs:6,132,178`・`boot_shell_tests.rs:44,323`・`runtime_boot_tests.rs:3,116`・`script_security_e2e_tests.rs:6,109`・`boot_config_ghost_wiring_tests.rs:47`、本体の `session_tests/boot_config_callghost_tests.rs:89,93,126`・`boot_config_option_tests.rs:78,151`・`boot_config_roots_tests.rs:4,166-167`・`boot_config_balloon_tests.rs:147`・`boot_config_input_tests.rs:264-265,294,343`・`session_end_tests.rs:238,281`・`session_end_deadline_body_tests.rs:30`・`ghost_session_strict_tests.rs:73`・`ghost_session_switch_fallback_tests.rs:7,122,246,296`・`reload_tests.rs:798`・`cli/parse_install_tests.rs:116,189`・`examples/emo-present/systems.rs:12`・`window.rs:8`（donor から移植）・`tests/emo2_real_run/common.rs:9-10`（「45 秒のまま」・何の線か書かない「35 秒の線」）｜同上（common.rs は「閉じ方の測りの最大より長く、締切より短い 45 秒」のように今の理由だけにする）｜同上。
73. **`unsafe` に `// SAFETY:` が無い** — `areka-session/src/ghost_session_scope_rect.rs:256-268`・`ghost_session_scope_rect_tests.rs:256,258,259`・`tests/stay_on_top_real_run.rs`（24 か所・0 件。例 152・159-172・333-336・618）・`tests/window_link_real_run.rs`（23 か所・0 件。例 64・71-91・282-288・697-699）（脳の 8 本は指摘 6）｜各ブロックに前提を 1 行書く（`embedded_icon_test.rs` が手本）｜`unsafe` が何に頼るかが残らない。
74. **コードを言い直すだけのコメント** — `main.rs:278`・`:284`（括弧の中の「`begin_close` より前」は理由なので残す）・`:375`・`:382`・`:392`・`:398-410`（「同じ理由で起こすたびにやり直す」を 6 回）・`:415`｜4 本の doc と 278・284 を消し、398-410 は 396 行の 1 行にまとめる｜順を変えたら doc だけが古くなる。

**F. 要らないコードと置き場**

75. **`mock-shell.rs` は退避の例で、その中の約 440 行のテストは 1 度も走らない** — `examples/mock-shell.rs:416-856`（全 856 行）｜全体テストは `cargo test --workspace`（`tools/test-all.ps1:75`）で、例の `#[cfg(test)]` は組まれない。emo-present は「donor から移植」のコメントで名指すだけ。`.kiro/specs/menu-thumbnail-preview/brief.md:19` が「小さな絵の窓の作り方」の手本として名指す｜例ごと消し、emo-present のコメントを直し、menu-thumbnail-preview の brief へ申し送る（残すならテストの mod だけ消す）｜走らないテストが守られている誤解を残し、番人の近くに居座る。［議題 9］
76. **誰も読まない絵 77 個（約 5.0MB）** — `crates/areka/shell/`｜コードが読むのは `base.png`（`examples/clickthrough_two_rects.rs:61`・`mock-shell.rs:51`）と `org/2.zip`（`areka-nar/src/crc32_tests.rs:51`）だけ。`0〜6.png`・`icon.png`・`x/y/z*.png`・`xyz.png`・`org/` の .pdn 5・.ai・.design・.jpg 2・PNG 群を読むコード・道具・テストは 0（`git grep` 済み）。`shell/0〜6.png`・`base.png` は `org/2/` と同じバイト。`doc/pasta-profile.md:20-49` がファイル名を挙げる（指摘 97）｜使われない 77 個を消す（git の履歴に残る）か、開発者の元絵として残す｜取り出しのたびに 5MB を運び、どれが生きた絵か分からない。［議題 8］

### できれば（21 件）

77. `areka-session/src/body_systems.rs:37-45` の `BodySystems::of` は条件ごとに 2 つ（test-support で `pub`・本番で `pub(crate)`）と私有の `installed_or_none`。外の使い手はテスト 1 か所（`session_tests/body_systems_tests.rs:43`）で、型も全欄も `pub`｜`pub fn of` 1 つにして `installed_or_none` を消す。［預かり 5.15・議題 5］
78. `areka-session/Cargo.toml:31-32` の `areka-emo-present` は本番のコードが名指さない（`ghost_session_scope_scale_test_support.rs:3` とテストだけ）｜`optional = true` にして `test-support` で開き、dev にも置く（依存のグラフは変わらない）。
79. `areka-session/src/ghost_session.rs` は 862 行。次に足すときは `GhostSession` の impl（430-659）を兄弟のファイルへ出す。
80. 件数を数える閉包が 3 か所（`main_session_mark_fallback_tests.rs:22-30,86-95`・`main_session_mark_tests.rs:613-616`）｜fallback の `count_event` を親へ上げる。
81. `main.rs:515-531` は `BootContext` を 2 度引く｜1 度引いて `(argv_session, app_profile_dir)` を取る。
82. `areka-session/src/log_history.rs:579` の `pub(crate) mod capture_tests;` は外の使い手 0｜`mod capture_tests;`。
83. `areka-session/src/smoke_exit.rs:52-54` の `is_empty()` の早い戻りは、`"".parse::<u64>()` が `Err` なので要らない｜1 行にする（`smoke_exit_tests.rs:14` が振る舞いを固定している）。
84. `boot_seam_restore_tests.rs:15-22,26,109,143,258` は作ったばかりの一意の `TempPath` を `remove_dir_all` し、3 本の終わりでも消す（Drop が受け持つ）。`unique_temp_dir` は包むだけ、札 `main-restore-seam` は古い名｜4 行と包みを消して `TempPath::new("restore-seam-…")` を直に呼ぶ。
85. `ghost_session_scope_rect_tests.rs:442-443,450,476-477` は同じ mod の `frame()`・`plant()` を使わずに組み直す｜補助を使う。
86. `switch_copy_tests.rs:173`・`:285` の `rx.recv().unwrap()` は写しが詰まると止まり続ける｜`recv_timeout(Duration::from_secs(30))`。
87. `tick_gate_config.rs:54-63,84-86` は環境変数の値を 2 度読み解く｜`TickGateSetting::Set(enabled)` で直に据え、`tick_gate_from_env_value` と試験の 4 行を消す（振る舞いは同じ）。
88. `crates/areka/Cargo.toml:104-107` の `areka-placement`・`areka-runtime` の `test-support` は `areka-session` の `test-support`（111 行）から連鎖で開く｜4 行を消せる（はっきり書く形を残すならそのまま）。［預かり 5.19］
89. `crates/areka/Cargo.toml:14-16` の `[[bin]]` は Cargo の既定と同じ｜3 行を消せる。
90. `session_tests/mod.rs:54-55` の `#[path = "write_failure_sites_tests.rs"] mod sites_tests;` は名を変えるだけ、`write_failure_sites_tests.rs:413` の `kinds.len() == 13` は型で決まっている｜`mod write_failure_sites_tests;` にし、表明を消す。
91. `session_tests/ghost_session_reload_tests.rs:72-170` は `areka-session` の `ghost_session_runtime_tests.rs:61-113` とほぼ同じ面を重い emo2 の起動で見る｜こちらだけの 2 面（split の後は送出端なし・2 度目の split は None）を `areka-session` の側へ足し、本体の 1 本を消す。
92. `ghost_session_reload_tests.rs:69-70` の「① で」は本番 `ghost_session.rs:568` の段の番号で、このファイルに無い｜「降ろしの最初の段（loop ticker の停止）で」。
93. `session_tests/reload_tests.rs:377-382` の 1 回目の `drain` の結果を見ていない（「受信端が無ければ無操作」が表明されていない）｜`capture` で包み、記録 0 件と `ReloadInFlight` の無いことを見る。
94. `ghost_session_restart_tests.rs:223-225,288` の `boot_windows_with_sticky_line` は `kero.balloon.dontmove,1` にも使う｜`boot_windows_with_descript_line` にし、説明を「渡した 1 行を書き足し」にする。
95. 起動の倒れ先の 3 つの記録（`areka-session/src/ghost_session.rs:717`・`:726-734`）に `event` 欄が無く、`ghost_session_strict_tests.rs:61-69` は本文の部分一致で数える｜記録に `event` 欄を足し、テストは欄で数える。［約束: ログの行（欄を足すだけ）・議題 4］
96. 開発者へ渡すコマンドが相対の形（`tests/emo2_real_run.rs:50,55,109-110`・`tests/emo2_first_load_repeat.rs:28-29`）｜`--manifest-path <絶対パス>\Cargo.toml` と絶対パスの exe にそろえる。
97. `icon/README.md:29,34` は「通知領域のアイコンを出す側が組の 2 番を番号で引きます」と今の形で書くが、組 2 を読むコードはまだ無い（読むのは埋め込みのテストだけ）。`doc/pasta-profile.md:20-49` は絵のファイル名を挙げるが置き場（`crates/areka/shell/`）を書かず、`icon.png` を「システムトレイ等」と書く（使うコードは無い）｜README は「引く予定（`tasktray-icon`）」にし、pasta-profile.md は置き場を 1 行足して「システムトレイ等」を消す（指摘 76 で消すなら行ごと直す）。

**Verdict**: 本番はおおむね健全。まず指摘 1・2（利用者に見える誤りと落ちる道）と 3（記録の無い起動不能）を直し、次に落ちる力の無いテスト（14〜31）を直す。
**Lean**: 消せる行の見込み 約 −4,500 行（SHIORI の脳 約 2,800・`mock-shell.rs` 856・テストの写しと重なり 約 500・長い説明の削り 約 300）と絵 77 個、依存 −2（`async-io`・脳を消すなら `windows-core`）・本番から dev へ移す依存 2（`areka-user-data`・`tracing-subscriber`）。
**Not checked**: 無し（範囲は全部読んだ）。ただし cargo・実機の走行は 1 度も回していない。指摘 2 の abort・指摘 16 の error の行・指摘 23 の `with_threads(1)` で投げた順に走ることは、読みからの判断。

## 5. 預かった件 5.1〜5.21 の照合（要件 5・要件 1.5）

| 件 | 出どころ（brief の節） | 今も在るか・場所 | 重なる指摘 |
|---|---|---|---|
| 5.1 | 追記（2026-11-06 `audit-sylphya-actor`） | 在る。`alert.rs:88`・`:224` | 1 |
| 5.2 | `placement-crate-split` の件 1 | 在る。`session_end_sync_send_tests.rs:294-378`・`readonly_body_tests.rs:115-154` | 20・43 |
| 5.3 | `audit-placement-core` の件 5 | 在る。`ghost_session.rs:213-214`・`boot_seam.rs:156`（`:7`・`:136` も） | 47 |
| 5.4 | 同 件 8 | 在る。`boot_seam_restore_tests.rs:24`（`:11`・`:34`・`:118` も） | 51 |
| 5.5 | 同 件 10 | 在る。3 つとも写し（`switch_finish_tests.rs:60-61` も 1 字違わない） | 32 |
| 5.6 | `audit-emo2-boot-spine` のオ・`audit-emo2-boot-frame` のオ | 在る。68-73 | 33 |
| 5.7 | `audit-emo2-boot-frame` のコ | 在る。6 か所とも行は同じ。ツリー全体の `let _ = .*handle\.join()` は 13 行（範囲の外は 7 節） | 29 |
| 5.8 | 同 サ | 在る。119 行の理由は今は誤り（`frame::test_support` は test-support で本体から届く） | 35 |
| 5.9 | 同 シ | 在る。形が違うので仮定 6 の「寄せずに理由を残す」に当たりうる | 36 |
| 5.10 | `audit-areka-input-menu` の件 1・件 3 | 在る。件 2 の文（`input_events_tests.rs`）は消えたが、同じ 182-183 行が別の誤りを持つ | 48・49・50 |
| 5.11 | `session-module-gather` の件 2 | 在る。221-223・254 | 65 |
| 5.12 | 棚卸㊷ | 在る。29・74・冒頭 | 66 |
| 5.13 | `dola-cue-test-builder` | 在る。3 か所とも寄せられる | 34 |
| 5.14 | `session-crate-split` | 6 項目とも、範囲の外の使い手は本体のテストだけ（6 節 (2)） | —（議題 5） |
| 5.15 | 同 | 使われない `pub(crate)` は 0 件（G1 が 52 件を名ごとに grep）。`BodySystems::of` は 1 つにできる | 77 |
| 5.16 | 同 | 在る。142・199 | 16 |
| 5.17 | 同・約束 ⑴ | `audit-areka-install-update` の決め待ち（6 節 (3)） | 17（表明の弱さ） |
| 5.18 | 同 | 在る。差は 2 行目のコメントだけ・見張り無し | 12 |
| 5.19 | `runtime-crate-split` | 在る（6 節 (4)） | 8・9・10・11・88 |
| 5.20 | Upstream | 材料は 6 節 (1) | 5・6・7・14（議題 1） |
| 5.21 | 古い節（`audit-placement-core`・`audit-emo2-boot-spine`） | 件 1 在る（範囲の中）・件 8 の `collision-probe/fixture.rs:23` 在る（範囲の外）・件 2・3・4・7・9・ク 済み・キ はほぼ済み（2 節） | 31・53 |

## 6. 判断の材料

**(1) SHIORI の試験用の脳（要件 1.8・5.20）**

| 観点 | 結果 |
|---|---|
| 本番の道 | `main.rs` から届くのは `main.rs:236` の `shiori_demo::run_demo_if_enabled()` だけ。既定の起動では `demo_enabled()` が偽で、debug を 1 行出して戻る。`AREKA_SHIORI_DEMO` を有効にしたときだけ、窓を開く前に main スレッドで同期に 1 周する。`ShioriHostSink`・`ShioriSession`・`ReferenceBrain` の本番の使い手はデモだけ。`#[unsafe(no_mangle)] shiori_factory` は exe の中なので外へ出ず、引く者もいない |
| テスト | 8 ファイルの中に 40 本（host 9・case_fold 1・session 8・demo 4・reference_brain 8・e2e 1・lifecycle 3・reference_e2e 6）。外から受け口を使うテストが 2 本＝`session_tests/ghost_session_scope_scale_boot_tests.rs:153`（範囲の中・指摘 14）と `mcp/get_property_tests.rs:243`（範囲の外・`audit-areka-install-update`） |
| 文書 | `git grep -n "AREKA_SHIORI_DEMO\|shiori_demo\|reference_brain\|shiori_host\|shiori_session" -- . ':!vendors'` で、`doc/`・`tools/`・`.claude/skills/`・README の当たりは 0 件（`AREKA_SHIORI_DEMO` の案内はどこにも無い）。当たりは `.kiro/steering/structure.md:276`（「`shiori_create` 入口」は古い＝今は `shiori_factory`）・`roadmap-history.md` の 3 行と `doc/roadmap-overview-2026-10-16.html:70`（経緯）。消すと古くなるコードの中の言及: `areka-ghost/Cargo.toml:29-32`・`areka-sylphya/src/actor.rs:125`・`log-capture-kit/src/event.rs:146`・`event_tests.rs:18`・`shiori4-testdll/src/lib.rs`（8 か所）・`session_tests/mod.rs:3`・本体の `Cargo.toml:24-26` |
| 本物との重なり | 実 DLL の読み込みと受け口は `areka-ghost/src/shiori_inproc.rs`（`InProcLibrary` が `GetProcAddress("shiori_factory")` で引き `InProcHost` を渡す）。違いは 3 つ＝Raise は warn だけ・Complete は常に UNKNOWN_TOKEN・プロパティは自前の HashMap（sylphya を見ない）。見本の脳は `shiori4-testdll`（自分で「reference_brain と同律」と書く）。ABI の遅延と complete は `shiori-abi` の自前のテスト。大小を畳む動きは sylphya の `key_case_fold_tests.rs`・`actor_case_fold_tests.rs`。`ShioriSession` の「一度に 1 件・遅延の期限」は本番に対応するものが無い（InProc は遅延を扱わない） |
| 消すと消えるもの | 2,767 行と `main.rs` の約 39 行・テスト 40 本・依存 `windows-core`（本番の `shiori-abi` の使い手も無くなる。`shiori-abi` は `areka-ghost` ほかが使うので crate は残る） |
| 代わりのテスト | ABI: `shiori-abi/tests/mock_brain_roundtrip.rs`（5 本）と `src/ergonomic.rs`。実 DLL: `areka-ghost/tests/ghost/inproc_e2e_test.rs`（4 本）・`shiori_inproc_tests.rs`（6 本）・`shiori_inproc_adapter_tests.rs`（11 本）。ゴーストの道: 指摘 14 の形に直す |
| 関わる先送りの印 | `areka-ghost/src/shiori_inproc.rs:246-247`「ShioriHostSink は移設しない・native 消費時に host 注入シームごと再設計」。今の `InProcHost` は sylphya に委ねないので、SHIORI4 DLL のゴーストが `GetProperty("baseware.version")` などを聞くと PropertyNotFound になる。消す前に、sylphya へ委ねる形（`shiori_host.rs:238-258` の 20 行）を `InProcHost` へ写す件を残すか決める（範囲の外・持ち主なし） |

**(2) `session-crate-split` が条件なしの `pub` にした項目（要件 5.14）**

| 項目 | 範囲の外の本番の使い手 | テストだけ（ファイル:行） |
|---|---|---|
| `GhostSession::seriko` | なし | `session_tests/ghost_session_reload_tests.rs:118` |
| `GhostSession::loop_ticker` | なし | 同 `:119` |
| `PreparedWindows::spawn` | なし | `session_tests/ghost_session_restart_tests.rs:207-208,341` |
| `reload::drain` | なし（`areka-runtime/src/reload_cue.rs:15` は doc だけ） | `session_tests/reload_tests.rs:382,392,673,678,751` |
| `write_failure::drain_system` | なし（crate の中の本番は `write_failure.rs:36` の登録） | `session_tests/write_failure_sites_tests.rs:65,105,318` |
| `boot_seam::insert_persist_wiring` | なし | `session_tests/readonly_run_tests.rs:206`・`readonly_sites_tests.rs:360`・`write_failure_sites_tests.rs:294` |

6 つとも外の使い手は本体のテストだけ。同じ形の `session_end::end_session_within`・`end_session_from` もある。どれも crate の中の本番が使う（欄・登録・呼び出し）ので、`#[cfg]` で消すことはできない。狭めるなら「test-support では `pub`・本番では `pub(crate)`」の 2 つの定義（`BodySystems::of` と同じ形＝5.15 で 1 つにしたい形）か、test-support の口に包みを足すことになり、行は増える（`pub(crate)` の項目は `pub use` で出し直せない＝E0364）。

**(3) `NO_BODY` の受け口（要件 5.17）**

本番では `register_systems`（`ghost_session.rs:49`）が最初に束を据えるので、`NO_BODY` の枝は配線を誤ったときにしか通らない。束が無いとき `installed_or_none` は呼ばれるたびに `error!(body_systems_missing)` を出し（`cfg(test)` の `areka-session` 自身のテストでは出さない）、次の形で答える。メニュー・入力の後・窓の差し込み・実行系の結線の後・切替の終わり・定常・入れ替えの一周は何もしない。`update_busy`／`install_busy` は偽。`cue_sinks` は `DiscardSink` 2 つで、インストール・更新の合図は黙って捨てられる。本体の窓口が無いときの答え（`main.rs:424-432`）は `InstallCueSink`・`UpdateCueSink` が受信端の無い送出端を持つ形で、ここが差。`areka-session` は本体の型を名指せないので、揃えるなら本体の側か、テストの表明の側で揃えることになる。読み直しが `NO_BODY` から使うのは `update_busy`／`install_busy` だけなので、`cue_sinks` だけを揃える決めなら `reload_test_support.rs` を直す所は無い。`log_history.rs:136` の `UNKEPT_TARGETS = ["areka::install::install_cue"]` は、その target に完全一致する info の行を履歴に残さない表（`doc/ssp-mcp/log-convention.md:28` が名で読む）。

**(4) 本体の依存の数え直し（要件 5.19）**

`git grep` で `crate名::`・`crate名;`・`crate名 as`・マクロ・derive を探した（ハイフンは `_`）。`*_tests.rs`・`*test_support*`・`runtime_tests/`・`placement_tests/`・`session_tests/`・`tests/`・`examples/` をテストとした。build-dependencies は無い。

| 依存 | 本番の使い手（ファイル数・例） | テスト | 判定 |
|---|---|---|---|
| wintf | 14（`main.rs`・`file_drop.rs`） | 57 | 要る |
| shiori-abi | 4（`shiori_host.rs`・`shiori_session.rs`） | 6 | 要る（脳を消すとテストだけ＝議題 1） |
| areka-ghost | 19 | 52 | 要る |
| areka-kanade | 16 | 65 | 要る |
| areka-parsers | 5 | 23 | 要る |
| areka-sylphya | 8 | 17 | 要る |
| areka-nar | 11 | 15 | 要る |
| areka-update | 5 | 10 | 要る |
| areka-mcp | 15 | 17 | 要る |
| areka-user-data | 0 | 1（`session_tests/readonly_run_tests.rs`） | dev へ移せる（指摘 9） |
| areka-placement | 6 | 47 | 要る |
| areka-runtime | 7 | 82 | 要る |
| areka-input | 2（`menu/mod.rs`・`menu/trigger.rs`） | 16 | 要る |
| areka-session | 39 | 116 | 要る |
| windows-core | 4（`reference_brain.rs`・`shiori_host.rs` ほか脳の中だけ） | 4 | 要る（脳を消すと要らない） |
| human-panic | 1（`main.rs` の `setup_panic!`） | 0 | 要る |
| thiserror | 3（`install/fetch_url.rs` ほかの derive） | 0 | 要る |
| tracing | 52 | 76 | 要る |
| tracing-subscriber | 0 | 3（例だけ） | dev へ移せる（指摘 10） |
| async-io | 0 | 0 | 要らない（指摘 8） |
| miniz_oxide | 1（`png_image.rs`） | 1 | 要る |
| bevy_ecs | 35 | 67 | 要る |
| windows（Dialogs ほか） | 10（`main.rs`・`install/pick.rs`） | 23 | 要る |
| areka-emo-atlas・emo-compose・seriko・emo-text・sakura・actor・dola・emo-present | 2〜13 ずつ（`offline_dump`・`mcp`・`install`・`update`・`menu`・`thread_roles.rs`） | 11〜27 | 要る（説明が古い＝指摘 11） |
| dev: shiori-host32-host・log-capture-kit・temp-path-kit・sample-ghost-kit・real-run-kit・image | 0 | 5・95・62・23・7・3 | dev で要る |
| dev: areka-placement・areka-runtime の test-support | — | — | `areka-session` の test-support からの連鎖と重なる（指摘 88） |
| dev: areka-input・areka-session の test-support | — | `flag_source_connected`・`log_history::hold` ほか | 要る |

外の crate が依存から消えるのは無い見込み（`async-io` は `areka-session` が、`windows-core` は `windows` 系が引き続き引く）ので、`THIRD-PARTY-NOTICES.md` の作り直しは要らない見込み（設計で `cargo tree` で確かめる）。

**(5) ワークスペース全体を歩く見張りの移し（要件 5.2）**

- `workspace_scan` は `log-capture-kit` の `tests/` の中のモジュールで、ライブラリでない。本体や `areka-session` の単体テストからは use できないので、見張りの静的な部分を `crates/log-capture-kit/tests/` の新しいファイルへ移すことになる。
- 口: `walk_workspace_sources() -> Vec<String>`（`crates/**/*.rs`・昇順・区切り `/`・`tests/`・`examples/`・兄弟のテストも含む）・`scan_sources(&[String], &[T]) -> Vec<Hit{path,line,token}>`（ブロックコメントも除く・文字列は残す）・`scan_tokens`（純粋）・`allowed_paths`・`unlisted`・`stale_entries`・`render`。使い方の手本は `temp_path_guard_test.rs`。
- `session_end_sync_send_tests.rs` の「テストのモジュールの中か」の欄と鍵ごとの件数は `workspace_scan` に無いので、見張りの側に 10 行ほど残す。前半の再現（`reproduce`）は試験台が要るので `areka-session` に残す。
- `readonly_body_tests.rs` の `withhold_everything` の見張りは、定義の行を除く仕組みが無いので、許可表に `areka-session/src/readonly.rs`・`areka-sylphya/src/persist/withhold.rs` を載せる。歩く範囲は今より広がる（`tests/`・`examples/`・入れ子）が、今の当たりの増えは 0 件。`engage` の 2 本（51-113）は全体を歩かないので本体に残せる。
- 移した後は `crate_source_test_support.rs:6-7` の「名前を付けた例外 3 本」の列（3 本 → 1 本。3 本目の `emo2_boot/frame_attach_tests.rs` は前の spec で済んだかを設計で確かめる）と `session_end.rs:53` の参照を直す。

## 7. 範囲の外の件（要件 7.10・7.12）

| 件 | 場所 | 持ち主らしい spec | 扱いの案 |
|---|---|---|---|
| 告知を抑える判定の写し | `crates/areka/src/install/startup.rs:220`（`before_boot_alert_suppressed`）＝`alert.rs:286` の `raise` の中と同じ決まり | `audit-areka-install-update` | 申し送り |
| `ShioriHostSink` を「ゴーストの道」にするテスト | `crates/areka/src/mcp/get_property_tests.rs:144-300` | `audit-areka-install-update` | 申し送り（議題 1 で脳を消すなら先に直してもらう） |
| `let _ = …handle.join()` | 本体の `install/names_tests.rs:29` | `audit-areka-install-update` | 申し送り |
| `SharedFakeIo` の写し・`join` | `areka-runtime/src/move_cue_apply_move_sticky_tests.rs:200`・`:255`・`move_cue_place_tests.rs:321`・`:386` | `audit-emo2-boot-switch` | 申し送り |
| `headless_wiring` の写し | 本体の `session_tests/shell_balloon_switch_tests.rs:78` | `audit-emo2-boot-switch` | 申し送り（指摘 40 と同じ直し） |
| 記録の文の経緯の語 | `areka-session/src/ghost_switch.rs:522`「今日どおり終わる」 | `audit-emo2-boot-switch` | 申し送り（ログの語＝裁定が要る） |
| 送れたかを確かめる前に組み立てを起こす | `areka-session/src/shell_balloon_switch.rs` の `accept_skin` | `audit-emo2-boot-switch` | 申し送り |
| `let _ = …handle.join()` | `areka-placement/src/persist_finalize_save_tests.rs:106` | 持ち主なし（見直し済み） | 議題 12 |
| ログの文に要件の番号 | `areka-emo-present/src/shell_target.rs:494`「（R6.1）」（`boot_shell_tests.rs:192`・`shell_target_boxes_tests.rs:172` が字面で突き合わせる） | 持ち主なし | 議題 12（ログの語） |
| 外形に上限が無い | `areka-emo-compose/src/plan_bounds.rs:43-76` | 持ち主なし | 指摘 2 は本体の側で止める。ほかに使い手は無い（`git grep "shell_bounds("` は `offline_dump/mod.rs:217` だけ） |
| 自動終了の数え始めの説明 | `real-run-kit/src/run.rs:21`（「子の起動から」＝実際は最初の窓の準備から） | 持ち主なし | 議題 12 |
| 例の本数の説明 | 本体の `placement_tests/measure_template_tests.rs:64-65` | 持ち主なし（この spec は見直さない） | 議題 12 |
| 「段 ③」の札 | `areka-emo-compose/src/sample_test_support.rs:3,5`・`areka-ghost/tests/ghost/inproc_fixture.rs:77`・`areka-placement/examples/window-placement.rs:112`・`areka-placement/src/sample_test_support.rs:10` ほか | 持ち主なし | 議題 12 |
| 本体にあった頃の名指し | `areka-runtime/src/lib.rs:347`・`:400`（「本体の `is_benign_boot_error`」）・`:443`（「本体の `ghost_session::BootShellChoice`」）・`areka-placement/src/lib.rs:30`・`error.rs:13`・`follow/work_area.rs:231` | 持ち主なし（`session-crate-split` のやり残し） | 議題 12 |
| 名指しの理由が古い | `areka-runtime/examples/collision-probe/fixture.rs:23`「main.rs／他 example と同名」（今は `areka-session/src/smoke_exit.rs:41`） | 持ち主なし | 議題 12（5.21 件 8） |
| `InProcHost` の `GetProperty` が sylphya を見ない | `areka-ghost/src/shiori_inproc.rs:246-247` | 持ち主なし | `/kiro-discovery` で起票の候補（6 節 (1)） |
| crate の説明に spec 名 | `areka-user-data/Cargo.toml:4` | 持ち主なし | 議題 12 |
| `mock-shell.rs` を手本に名指す | `.kiro/specs/menu-thumbnail-preview/brief.md:19` | `menu-thumbnail-preview` | 指摘 75 で消すなら brief へ申し送り |
| steering の古い記述 | `.kiro/steering/structure.md:276`（「SHIORI 契約チェーン e2e も同居」「`shiori_create` 入口」） | この spec（完了のときの steering の直し） | 議題 1 の答えで直す |

## 8. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今のコードの資産 | ギャップ |
|---|---|---|
| 1 洗い出し | この報告（97 件） | 無し（Not checked 無し・cargo は回していない） |
| 2 確定 | 4 節・9 節の議題 | Constraint: 外への約束に触れる 4 件（指摘 1・4・5・95）は要件 2.7 の確定が要る。3 種類目（SSP・ukadoc）は 0 件 |
| 3 直し切る | 指摘ごとの直しの案 | Missing: 写しの寄せ先（指摘 37・38・39・45）の置き場と名は設計で決める。Unknown: 指摘 5 の消す順（範囲の外のテストが先） |
| 4 テストで縛る | 決定論の支え（`TempPath`・`log-capture-kit`・`SwitchRig`・`GpuPermit`・偽の SHIORI） | Missing: 指摘 1・2・3・14・15・28 の新しいテストと、16・17・19・23・24 の表明の立て直し。指摘 25・26 は実画面のテストで、全体テストの段（要件 8.3）でだけ確かめられる |
| 5 預かった件 | 5 節 | 5.14 は要件の形（戻すか絞る）と材料（どれも絞ると行が増える）が合わない＝議題 5。5.17 は相手の決め待ち |
| 6 決まり | 1,000 行の番人（966・959・873・862・856・820） | Constraint: 足すテストは新しいファイルへ。指摘 30・45・62・75 で余裕が増える。依存は足さない（指摘 1 は既にある crate の enum に腕を足すだけ） |
| 7 範囲の外 | 7 節 | Constraint: 指摘 1（`areka-sylphya`）・35・36・65・66（本体の `runtime_tests/`・`menu/`）・43（`log-capture-kit/tests/`）は範囲の外のファイルを触る。どれも要件 5 の件か直しに要る呼び出し元で、C26 のほかの 2 本の持ち物は 0。`Cargo.toml` は C26 でこの spec だけの席（指摘 8〜11・88・89） |
| 8 副作用 | 実機の確かめは要らない（どの直しも決定論のテストか、全体テストの中の既存の実機の段で確かめられる） | 無し。指摘 26 の値は測らない（仮定 11） |
| 9 仕上げ | — | かけ直しの作法は要件どおり |

## 9. 直し方の案・規模・設計と要件の討議への持ち越し

### 直し方の案

- **案 A（今あるファイルの中で直す）**: 本番の直しはその場で、テストの写しは既にある支え（`session_tests/mod.rs`・`sample_test_support.rs`・`reload_test_support.rs`・上流の test-support）へ寄せる。✅ 差分がいちばん小さい・構造を変えない ❌ 本体の `session_tests/` に `areka-session` だけのテストが残り（指摘 30・46）、番人の余裕が増えない。
- **案 B（置き場を正す移しを先に）**: `areka-session` の口だけを使うテスト（指摘 30・46・91）を `areka-session` へ、全体を歩く見張り（指摘 20・43）を `log-capture-kit/tests/` へ移してから、残りを直す。✅ 判定を持つ crate の中で退行が赤になる・番人の余裕が増える ❌ 移すファイルが多く、`#[path]` と `mod.rs` の付け替えが増える。
- **案 C（A を基本に、移しは要件 5.2 と番人に関わる所だけ）**: 指摘 20・30・43（預かり 5.2 と番人 959 行）は移し、指摘 46・91 は議題で決める。脳（指摘 5）は範囲の外のテストの直しの順を申し合わせてから消す。
- **推す案**: 案 C。

### 規模と危うさ

- **規模**: L（1〜2 週）。タスクの見込みは **24〜30**（組: 必ず直す 2・本番の失敗の道と記録 1・脳の削除 2・依存と README とマニフェスト 1・落ちる力の無いテストの立て直し 本体 2〜3・`areka-session` 2・実画面のテスト 2・写しの寄せ 3〜4・見張りの移し 1・テストの移し 2・古い説明 2〜3・コメントの決まり 2・要らないコードと絵 1・範囲の外の預かり（`runtime_tests/`・`menu/`）1・`NO_BODY` 1・かけ直しの検証 1〜2）。20 を越える。
- **危うさ**: 中。本番の振る舞いが変わるのは指摘 1・2・3・4 だけ。ほかは本番を変えない。指摘 1 は見直し済みの crate の公開の enum に腕を足す（match の網羅を直す呼び手は `alert.rs:220`・`write_failure.rs:55` とテスト）。指摘 25 は実画面のテストで、全体テストでしか確かめられない。

### 議題（答えで作業が変わるもの・推す答えつき）

1. **SHIORI の試験用の脳を消すか（要件 5.20・指摘 5・6・7・14）** — 推す答え: 消す（約 2,800 行・テスト 40 本・`AREKA_SHIORI_DEMO`・`windows-core`）。代わりのテストは 6 節 (1) にある。順は、指摘 14 を直し、範囲の外の `mcp/get_property_tests.rs:144-300` は `audit-areka-install-update` のセッションに同じ形への直しを申し合わせてから消す（向こうが直せないなら `shiori_host.rs` だけ残して、向こうの着地の後に消す件として申し送る）。`AREKA_SHIORI_DEMO` が消えるのは要件 2.7 の環境変数の変化で、読み手は 0 件（6 節 (1)）。`structure.md:276` の記述も同じ変更で直す。`InProcHost` が sylphya を見ない件は `/kiro-discovery` で別に起票する。
2. **記憶のファイルが使えない場面の直し方の一文（指摘 1・要件 5.1）** — 推す答え: `areka-sylphya`（見直し済み・持ち主なし）の `PersistFailureCause` に理由の型を 1 つ足して出し分ける（要件 7.7 の「直しに要る範囲の外の変更」として扱う）。文字列の中身で見分ける案は、sylphya の文が変わると黙って外れるので推さない。字面は設計で決める（仮定 9）。
3. **`--offline-dump` の巨大な下地（指摘 2）** — 推す答え: 本体の `surface_stage` で合成と同じ上限を越えたら `error!` を出して面を全部失敗にする（本体だけで直る）。`areka-emo-compose` の `shell_bounds` に上限を入れる案は範囲の外の crate を変えるので推さない。終了コードの意味（全部の面が失敗）は SSP の説明書の `--offline-dump` の決まりの中に収まるかを設計で ukadoc（SSP の起動オプション一覧）に当てる。
4. **ログの行の語を変える直し（指摘 4・95）** — 推す答え: 変える（`[main]` → `[session_mark]`・`[boot_seam]`、倒れ先の記録に `event` 欄を足す）。読み手（実機の確かめ・`tools/`・手順書・`doc/ssp-mcp/log-convention.md`）を設計で grep し、あれば同じ変更で直す（要件 7.9）。
5. **`session-crate-split` が開けた `pub` 6 項目（要件 5.14）と `BodySystems::of`（5.15）** — 事実: 6 項目とも外の使い手は本体のテストだけだが、どれも crate の中の本番が使うので、狭めるなら 2 つの定義か包みが要り、行が増える（6 節 (2)）。これは 5.15 で `BodySystems::of` の 2 つの定義を 1 つにする向きと逆になる。推す答え: 6 項目は `pub` のまま残すと裁定して理由を記録し（要件 2.5）、`BodySystems::of` も `pub fn` 1 つにまとめる（指摘 77）。要件 5.14 の文（「戻すか絞る」）は裁定に合わせて改める。
6. **全体を歩く見張りの移し（要件 5.2・指摘 20・43）** — 推す答え: 移す。移すと歩く範囲が `crates/**`（`tests/`・`examples/`・入れ子）へ広がるが、今の当たりの増えは 0 件なので、要件 5.2 の「見る中身は変えない」の中と読む。`readonly_body_tests.rs` の `engage` の 2 本は全体を歩かないので本体に残す。
7. **マニフェストの写し（要件 5.18・指摘 12）** — 推す答え: `areka-session/build.rs` が `../areka/areka.manifest` を読み、`tests.manifest` を消す（写しが無くなる・−9 行）。crate の外のファイルを読むのを避けたいなら、2 行目を除いて同じであることを見る見張りを 1 本置く。
8. **使われない絵 77 個（指摘 76）** — 推す答え: 消す（git の履歴に残る・コードの使い手は 0）。`base.png`（例 2 本）と `org/2.zip`（`areka-nar` のテスト）は残す。元絵（.pdn・.ai・.design）を開発者が手元で使っているなら、残すと裁定する。
9. **`mock-shell.rs` を消すか（指摘 75）** — 推す答え: 消す（自分で退避と名乗る・テストは走らない）。`menu-thumbnail-preview` の brief へ「手本は git の履歴の `crates/areka/examples/mock-shell.rs`」と申し送る。残すなら走らないテストの mod だけ消す。
10. **`BootContext` の補助の写し（要件 5.9・指摘 36）** — 推す答え: 形が合わないので仮定 6 のとおり寄せずに理由を 1 行残す（5 か所のまとめ直しは C26 の 3 本の着地の後）。
11. **メニューの呼び手の説明（要件 5.11・指摘 65）** — 推す答え: 直す（説明の文だけ・`menu/` の中身は触らない）。
12. **持ち主のいない範囲の外の古い説明（要件 5.21・7.10・7 節）** — 推す答え: 説明の文（コメント・doc）だけのものは、この spec がその場で直す（`collision-probe/fixture.rs:23`・`areka-runtime/src/lib.rs:347,400,443`・`areka-placement` の 3 か所・`real-run-kit/src/run.rs:21`・`placement_tests/measure_template_tests.rs:64-65`・「段 ③」の札・`areka-user-data/Cargo.toml:4`）。コードの直し（`areka-placement` の `join`）とログの語（`shell_target.rs:494`）は、この spec では直さずに覚え書きとして回す。
13. **本体の部品を使わないテストの移し（指摘 30・46・91）** — 推す答え: 移す（判定を持つ crate の中で退行が赤になる）。指摘 30 は番人の余裕のためにも要る。
14. **`NO_BODY` の受け口（要件 5.17）** — 推す答え: 仮定 7 のまま。`audit-areka-install-update` の決めをセッションの伝言で受け取り、決まらなければ最後のタスクにして、着地までに決まらなければ向こうへ返す。決まるまで、指摘 17 の落ちる力の無い表明は消すだけにする。
15. **規模（要件 2.6・仮定 8）** — 見込み 24〜30 で 20 を越える。推す答え: 2 本目を切らずに 1 本のまま進める（記憶の決めごと「一度 spec に切り出した後は、規模を理由にした分割を後出しで提案しない・20 タスクは discovery の分け方の目安で spec の上限ではない」）。切るなら「必ず直す・本番の直し・脳の削除」と「テスト・説明・コメント」で分けられる。
16. **先送りの印に当たるか（要件 2.2 の 2 種類目）** — 範囲の中の理由つきの `#[allow(dead_code)]` は 2 つ。`perf_thread_report.rs:224` の `delta`（`perf-rank.py` の規則の基準・本物の先送りの印）は残す。`offline_dump/render.rs:47-50` の `RenderError::Write` の中身（本番は `Err(_)` で読み捨てる）は先送りの印と読めるので残す（推す答え）。`shiori_demo.rs:26,37` の `DemoError::Timeout` は事実と違う印なので指摘 7 で直す。`shortcut:`・`ponytail:` の注釈は範囲に 0 件。

### Research Needed（設計で確かめる）

- 指摘 2: `--offline-dump` で全部の面が失敗したときの終了コードが、SSP の起動オプション一覧の決まりのどれに当たるか（ukadoc で確かめる）。
- 指摘 3: `log_history::init` が `main.rs:196` より前に済んでいるか（記録が出口に届くか）。
- 指摘 4・95: 変える記録の文と欄の読み手（`tools/`・`doc/ssp-mcp/log-convention.md`・実機のテスト）。
- 指摘 8〜10: 依存を外し・移した後の `cargo tree` と `Cargo.lock` の差（`THIRD-PARTY-NOTICES.md` を作り直す要があるか）。
- 指摘 23: `WintfTaskPool::with_threads(1)` で投げた順に走るか（bevy_tasks の待ち行列の振る舞い）。
- 指摘 26: 120 秒が足りるか（測らない・emo2 のテストと同じ値にそろえる根拠だけ）。
- 指摘 43・20: 3 本目の例外（`emo2_boot/frame_attach_tests.rs`）が前の spec で済んでいるか（`crate_source_test_support.rs:6-7` の列の直し方）。
- 議題 1: `mcp/get_property_tests.rs` の直しを `audit-areka-install-update` のセッションが受けられるか（申し合わせ）。

## 10. 設計の段の調べ（2026-11-22・`/kiro-spec-design`）

**Discovery の型**: light（既にあるコードの直し・新しい依存も外の API も無い）。読んだもの: `kiro-spec-design` の規則（design-principles・discovery-light・synthesis・review-gate）・設計の雛形・完了した見直しの設計 2 本（`audit-areka-input-menu`・`audit-emo2-boot-spine`）・ukadoc の MCP（SSP の説明書）・並走の兄弟 `audit-areka-install-update` の research.md（4 節の「範囲の外への及び」・9 節の「申し送りの見込み」・読むだけ）。cargo は回していない（読みと `git grep` だけ）。

### 10.1 Research Needed の答え

| 項目 | 答え | 根拠 |
|---|---|---|
| 指摘 2 の終了コード | 下地が上限を越えたら `error!` を 1 行出す。写しの窓の中なので `report::exit_code` が **2** を返す（新しい決まりは要らない） | SSP の説明書「起動オプション一覧」の `--dump-error-log` の終了コードの表「2：Errorあり（Criticalなし）」（https://ukatech.shillest.net/ukadoc-ssp/functions/option/ ）。`offline_dump/mod.rs` の `dump` は `capture_begin` → `surface_stage` → `capture_finish` → `report::exit_code` の順（`mod.rs:84-90`） |
| 指摘 2 の上限の値 | 合成の上限 `MAX_EXTENT`（16,384）は `areka-emo-compose/src/plan_extent.rs:17` の `pub(crate)` で、本体から名指せない。本体の `offline_dump/render.rs` に同じ値の定数を 1 つ置く（理由を 1 行）。面積の上限（`MAX_AREA`＝2^25）は当てない（議題 7 の確定は「幅か高さ」。シェル全体の外形は面の和なので、まともなシェルでも面ごとの面積の上限を越えうる） | `plan.rs:48` の `pub(crate) use extent::{MAX_AREA, MAX_EXTENT, ..}`。`areka-emo-atlas/src/decode.rs:39` も同じ値を `pub(crate)` で自前に持つ（前例） |
| 指摘 2 の赤のテスト | 10 万の位置の検体では、直す前のテストがプロセスごと落ちる（abort）。赤を先に立てるのは、幅だけが上限を越え面積は小さい検体（例 x＝20,000・高さ数十）で行う。直す前は PNG が書けて終了コード 0、直した後は 2 で PNG 無し | 読みからの判断 |
| 指摘 3 | `log_history::init()` は `main.rs:126`（`fn main` の 2 行目）で、`WinApp::with_exit_policy`（196 行）より前。`error!` は出口に届く | `main.rs:116-127` |
| 指摘 4 の読み手 | `[main]` の頭を字面で読む所は `tools/`・`doc/`・`.claude/skills/`・`crates/real-run-kit`・本体の `tests/` で 0 件（`git grep -F "[main]"` の当たりは `tools/crates-io.ps1:235` の YAML の `branches: [main]` だけ）。event 名は変えない | `git grep` |
| 指摘 95 の読み手 | 本文で数えるのは範囲の中の `session_tests/ghost_session_strict_tests.rs:66` だけ（直しで event の欄で数える形へ）。`doc/ssp-mcp/log-convention.md:55` は target `areka_session::ghost_session` を status として名で読むだけで、本文も event も読まない。欄を足しても変わらない | `git grep` |
| 指摘 8〜10 の依存 | 外の crate が依存の木から消えることは無い。`async-io` は `areka-session`・`areka-placement`・`areka-runtime`、`windows-core` は `areka-ghost`・`areka-emo-text`・`pilot`、`tracing-subscriber` は `areka-session`（本番）ほか、`areka-user-data` は `areka-session`・`areka-mcp-bridge` が引く。`Cargo.lock` は本体の `[[package]]` の依存の列だけが変わる。`THIRD-PARTY-NOTICES.md` は `tools/test-all.ps1 -License` の `cargo about generate` で作り直し、差が 0 であることを確かめる（手で直さない） | 各 `Cargo.toml` の `git grep` |
| 指摘 5 の `windows-core` | **脳の外にも使い手が 1 つある**。`crates/areka/tests/embedded_icon_test.rs:24` の `use windows_core::{HSTRING, PCWSTR};`。依存の行を消すなら、ここを `windows::core::{HSTRING, PCWSTR}` へ替える（`windows` は本体の依存に既にある・`main.rs:20` と同じ綴り） | `git grep -w windows_core -- crates/areka` |
| 指摘 5 の `shiori-abi` | 脳の外の使い手は `session_tests/ghost_session_scope_scale_boot_tests.rs:12-13`（指摘 14 で外れる）と範囲の外の `mcp/get_property_tests.rs:148-149`（`audit-areka-install-update` が外す）だけ。両方が外れれば本体の依存の行を消せる | 同上 |
| 指摘 23 | `WintfTaskPool::with_threads(1)` は `bevy_tasks` の `TaskPoolBuilder::num_threads(1)`（`wintf/src/ecs/widget/bitmap_source/task_pool.rs:48-49`）。`bevy_tasks` 0.19.1 は `async-executor` 1.14.0 の上にあり、外から投げたタスクは 1 本の待ち行列に投げた順に積まれ、働き手 1 本が順に走らせる。窓のタスクが最初の `await` より前に `CommandSender` へ送るなら、目印より先に送られる。`areka-session` の 10 本の試験台が同じ形（`with_threads(1)`）で既に順に頼っている | `Cargo.lock` の版・`git grep with_threads` |
| 指摘 26 | 値は測らない（仮定 11）。同じ子を立てる emo2 のテストの締切 120 秒（`tests/emo2_real_run/common.rs:13`）にそろえる根拠だけを書く | — |
| 指摘 20・43 の 3 本目の例外 | `areka-runtime/src/frame_attach_tests.rs:603-612` は今も `crates/` を自前で歩く（前の spec で済んでいない）。移した後の `crate_source_test_support.rs:6-7` の列は 3 本 → 1 本（`areka-runtime` の `frame_attach_tests.rs` だけ） | 読み |
| 指摘 20・43 の走査の範囲 | 同期の送信の見張りは「本番の源」だけを見る約束（今は `crates/<x>/src/**` から `_tests.rs`・`_test_support.rs` を除く）。`walk_workspace_sources` は `tests/`・`examples/`・兄弟のテストも返すので、そのまま使うとテストのファイルの当たりが増える（`parent_window_tests.rs` ほか 8 本）。移した見張りは列挙の結果を今と同じ条件（`crates/<x>/src/` の下・`_tests.rs`／`_test_support.rs` でない）で絞る。見る中身は変わらない。`withhold_everything` の見張りは議題 11 の確定どおり `crates/**` 全部を歩き、許可表は `areka-session/src/readonly.rs`（呼び出し）と `areka-sylphya/src/persist/withhold.rs`（定義の行。`scan_tokens` は `fn ` の後ろも当てる）の 2 つ | `git grep` の当たりの数え |
| 指摘 20・43 の走査語 | `log-capture-kit/tests/` の見張りは自分の走査語を逐語で書かない約束（`temp_path_guard_test.rs` の冒頭）。移す見張りも `concat!` で 2 片から組み、`files_that_must_not_spell_the_tokens` で自分を縛る | `workspace_scan/mod.rs:682-694` |
| 議題 1 の申し合わせ | 10.3 のとおり（向こうが先か同時に着地・遅れたら `shiori_host.rs` だけ残す） | 兄弟の伝言 |

### 10.2 設計の段で見つけた事実

- `doc/COMPAT_ARCHITECTURE.md` §8 の既にある行が、移す・名を変えるテストを名で指す。301 行（`main_session_mark_write_tests.rs`）・318 行（`main_session_mark_tests.rs` の `session_mark_verdict_table`・`settle_session_mark_applies_the_verdict`、`session_tests/boot_config_input_tests.rs`）・319 行（`session_end_sync_send_tests.rs` の `production_sync_sends_match_the_allowed_table`）・403〜405 行（`session_tests/boot_config_balloon_tests.rs`）。移した同じ変更で、その行のパスと名だけを直す（約束 ⑶）。
- `.kiro/steering/structure.md:460` が `log-capture-kit/tests/` の見張りを並べる。移した見張り 2 本を同じ変更で足す。
- 脳を消すと古くなる範囲の外の説明（どれも見直しの済んだ crate・C26 の 2 本の持ち物でない）: `areka-ghost/Cargo.toml:31`・`areka-ghost/src/shiori_inproc.rs:246`・`areka-sylphya/src/actor.rs:125`・`log-capture-kit/src/event.rs:146`・`event_tests.rs:18`・`shiori-abi/src/interface.rs:45`・`shiori4-testdll/src/lib.rs`（15・22・70・118・134・156・182・273 行）。説明の文だけを同じ変更で直す（要件 3.3・7.7）。
- 「段 ③」の札は 7 節の列のほかに `areka-nar/src/names_tests.rs:139`・`log-capture-kit/tests/sample_path_guard_test.rs`（12・85・229・236・324 行）にもある。議題 17 の確定（説明の文だけなら直す）に入れる。
- `placement_tests/measure_template_tests.rs:64-65` の「examples 3 本（本体の 2 本）」は、`load_shell_target` を呼ぶ例が本体 1 本（`emo-present`）・実行系 1 本（`collision-probe`）・配置 1 本（`window-placement`）なので「本体の 1 本と実行系の 1 本」が今の事実（`mock-shell.rs` は呼ばない）。
- 指摘 31 の例の読み手は、配置の crate の読み（`areka-placement/src/config.rs:261-275`）と同じ式に直す。例のテストは走らない（指摘 75）ので、本番の式は `areka-placement` の既存のテストが縛る。

### 10.3 並走の兄弟（`audit-areka-install-update`）からの伝言（2026-11-22・向こうの要件は承認済み `43031a5fc`）

- **`NO_BODY`（要件 5.17）は決まった**: 受け口はそろえない。`body_systems.rs:60` の「（窓口が無いときの答えと同じ）」だけが事実と違うので消す（本体の窓口が無いときは URL の腕で取りに行ってから送れずに warn する。`NO_BODY` は取りに行かない）。`UNKEPT_TARGETS` に足す物は無い（向こうは `info!` を足さない）。`reload_test_support.rs` は 5.17 としては直さない。
- **`mcp/get_property_tests.rs:144-300`** は向こうが `PropGetCueSink` を通す形に直し、こちらの脳の削除より先か同時に着地する。遅れたら `shiori_host.rs` だけを残し、向こうの着地の後に消す。
- **向こうの依頼 A**: `reload_test_support::boot_context` に、ゴーストの名とバルーンの名を引数で取る形を足す（向こうの写し 4 つを寄せる）。足すと、こちらの `menu/ghost_frame_tests.rs:43` の写しも `boot_context_named(root.dir(), root, current, "StayseeBalloon")` と同じ値になる（`temp` に `root.dir()` を渡すと置き場 `root/profile`・補助 `root/helper.exe` が一致）。議題 13（寄せない）の前提が変わるので、設計の討議の議題にする。
- **向こうの依頼 B**: `alert.rs` に「この走りで告知を抑えるか」（`suppressed() || smoke_exit::smoke_exit_ms().is_some()`）を答える公開の関数を 1 つ置く（向こうは `install/startup.rs:217-222` の写しを消してそれを呼ぶ）。
- **向こうから回ってきた件（こちらのファイルの中）**: ⑴ `png_image_tests.rs:166` の `wic_decode_rgba` を `pub(crate)` で共有（向こうの `mcp/dump_surface_gpu_test_support.rs:410-439` の写しを消すため） ⑵ `mcp/sakurascript_log_tests.rs`（本体のコードを通らない）の中身を `log_history` のテストへ ⑶ `alert.rs:150`・`alert_tests.rs:51` が、向こうが欄を減らす `RootError::ExeLocationUnavailable`（`tried`・`origin` はいつも空と既定）を読む ⑷ `log_history.rs:335` の `snapshot` が、最大 1,000 件×4,096 字をプロセスで 1 つの排他を握ったまま写す ⑸ 「書ける根の印の項目は外せない」「印があるとき実行ファイルの隣は先頭へ動かせない」の決まりが `menu/roots_frame.rs:73-75・109` にしかなく、判断の `roots/edit.rs` の `decide_remove`・`decide_front` は受け付ける（`menu/` は議題 2 で見直さない所・`roots/` は向こうの持ち物）。

### 10.4 Synthesis

- **一般化**: 写しの指摘（32〜45・5.5〜5.9・5.13）は「上流の test-support にもう在る補助へ寄せる」か「同じ crate の支え（`session_tests/mod.rs`・`reload_test_support.rs`・`tests/real_screen/common.rs`）に 1 つ置く」かの 2 つの形にまとめた。新しい補助の crate や trait は作らない。
- **Build vs Adopt**: 見張りの走査は自作をやめ `log-capture-kit/tests/workspace_scan` を使う（5.2）。ほかに外のものを入れる所は無い。依存は足さない。
- **簡素化**: 消すもの＝SHIORI の試験用の脳 8 ファイル・`mock-shell.rs`・絵 77 個・落ちる力の無いテスト・使い手の無い依存の行。`ghost_session.rs`（862 行）の `impl GhostSession` は兄弟のファイルへ移すだけ（形は変えない）。指摘 2 の上限は本体の 1 か所（合成の crate を変えない）。`snapshot` の排他は、記録を `Arc` で持って深い複製を排他の外へ出す（公開の形 `fn snapshot(Kind) -> Vec<Record>` は変えない。向こうの `mcp/get_log.rs:24` に触らない）。

## 11. 設計の討議の確定（2026-11-22・推奨どおり一括承認）

12 の議題を推奨どおり確定した（design.md の「設計の討議で確定したこと」）。根拠の要点:

- S8（台本の読み直しの断り）: 読み直しの出どころは `ReloadOrigin::{Script, Mcp}` の 2 つだけで、メニューからの読み直しは無い。`audit-emo2-boot-switch` が切り替えの断りを「ゴーストの台本からの要求は止めない」と決めたのにそろえ、更新中・インストール中の断りを MCP の要求だけに当てる。重なりと `NoContext` の断りは変えない。
- 議題 2: S1 の `boot_context_named` を足すと、`menu/ghost_frame_tests.rs:43` の写しは `temp` に `root.dir()` を渡して同じ値になるため、要件の議題 13 の「寄せない」の理由（形が合わない）が消えた。
- 議題 6: spec は squash の PR 1 本で着地するため、兄弟と同時の着地はできない。こちらが先なら残りを兄弟の PR で消す。こちらの着地で C26 の `Cargo.toml` の席は空く。
