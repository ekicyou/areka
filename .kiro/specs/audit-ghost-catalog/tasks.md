# 実装計画: audit-ghost-catalog

> 設計の「タスクの束と順」の 18 束を、実装の順に 1〜18 へ並べ直した（括弧の中は設計の束の名）。指摘の番号（#1〜#91）は `research.md` §3、直し方と縛るテストの名は `design.md` の「指摘ごとの直し方」が正本。
>
> 全タスクに共通の完了の条件（各タスクの箇条では繰り返さない）:
> - 消すコード・ファイルは、消す前にツリー全体（テスト・fixture・設定・文字列での参照を含む）を grep して使われていないことを確かめる（要件 3.3）。
> - 触ったファイルはどれも 1,000 行を越えない（例外表に載る `inproc_e2e_test.rs` を除く）。足して越えそうなら兄弟のファイルへ（要件 5.1）。
> - 新しい失敗の道は `error!`＋`Err`（要件 5.2）。依存を足さず `Cargo.lock` を変えない（要件 5.5）。`doc/`・`docs/` を名指す行を足さず変えない（要件 5.7）。
> - 新しいテストは時間待ちを使わない決定論で、一時フォルダは `TempPath`／ワークツリーの `target\` の下（要件 4.2・4.3）。
> - 触ったクレート（範囲の外を含む）の `cargo test -p <crate>` が緑。
> - 範囲の外のファイルは「範囲の外で触るファイル」の表の中身だけを変える（要件 6.2）。
> - `areka-ghost/src` の共有の道具（`test_support.rs`）と `lib.rs` の `mod` の行を変えるのは 9 だけ。10〜14 は新しい道具を自分の触るテストのファイルの中に置く（並んで走る 11・12・14 とぶつからないため）。

## `areka-nar`

- [ ] 1. `plan_tests.rs` を減らし、テストの写しを 1 つにまとめる（T1）
  - #60 の写しの道具を消し、#61 の install.txt の頭と #12 の `root_with_ghost`・「読む→検証→解く」の道具を親の `plan_tests.rs` に 1 つずつ置く。`lib_tests` の木の読みの道具を共有にし、`install_tests`・`install_commit_tests` の写しを消す
  - #9 の「根へ何も書かない」の比べをバイト列に直し、較正（`b"1"`→`b"2"`）で赤になることを確かめる
  - #14: 直しの後に `plan_tests.rs` が 950 行を下回らなければ、「書き込みをしない」の塊を兄弟の `plan_no_write_tests.rs` へ移す
  - 完了の状態: `plan_tests.rs` が 950 行未満（または兄弟へ移して 950 行未満）で、同じ形の道具の写しが `areka-nar` のテストに残っていない
  - _Requirements: 3.1, 4.5, 5.1, 9.1_

- [ ] 2. `areka-nar` の落ちないテストと走査の写しを直す（T2）
  - #13 の「入出力をしない」の走査を `lib_tests` の 1 つの道具にまとめ、3 つのテストがそれを使う。較正は残す
  - #10 の上限の宣言を上限ちょうどにし、#65 を書庫の組み立て道具の書き出しへ、#70 の二重の `Option` を真偽へ直す
  - #66・#67・#69 の落ちようのないテスト・定数どうしの比べ・重複を消し、#68 の自前の 1,000 行の検査を消す（全体の番人が受け持つ）
  - 完了の状態: `lib_tests.rs` が設計の見立て（約 880 行）以下に減り、上限の比べを `>=` に変えると赤になる
  - _Requirements: 3.1, 4.5, 9.1, 9.5_

- [ ] 3. 同じ根へ同時に入れる 2 本目を棚の錠で断る（T3）
  - 展開の作業場が棚の `.lock` を共有なしで開いて持ち、開けなければ錠のパスを添えた段の失敗として返す（公開面の出口が `error!` を 1 回出す）。棚の片付けは錠のファイルを飛ばす
  - 作業場を捨てると錠が閉じて `.lock` が消えることを縛るテストで確かめ、消えない作りなら捨てるときに消す 1 手を足す
  - #11 の棚の散らかりを片付ける枝のテストを足す
  - 完了の状態: 錠を外すと `a_second_work_area_on_the_same_root_fails_and_leaves_the_first_intact` が赤になり、`the_shelf_lock_is_released_when_the_work_area_is_dropped`・`a_stray_file_on_the_shelf_is_swept` が緑
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 5.2, 9.1_

- [ ] 4. 投げ込みの判定を書庫の末尾と目次だけで読み、大きすぎる書庫を読む前に断る（T4）
  - 終端記録の位置決めと目次の読みを、全体のバイト列と「末尾と目次だけ」の両方で使える形に分ける。今の入口の署名と断りの順は変えない
  - 判定は末尾の窓と目次だけを読み、ローカルの頭は見ない。`open` と判定は書庫の大きさの上限（宣言の総和の上限の 2 倍）を読む前に見て、越えれば大きすぎるとして断る。断りに載せる位置はファイルの頭からの位置に戻す
  - 判定の doc に「ローカルの頭だけが壊れた書庫は判定が在ると答え、`open` が断る」を書く
  - 完了の状態: `peek_reads_only_the_table_of_contents`・`peek_finds_the_end_record_behind_a_long_comment`・`open_refuses_an_archive_longer_than_the_limit_before_reading` が緑で、既存の判定と `open` の一致のテストが変えずに通る
  - _Requirements: 3.1, 3.2, 4.1, 4.2, 5.2, 9.1_

- [ ] 5. 名前の検証と比べ方を 1 つにそろえる（T5）
  - #3 の上付き数字の `COM`・`LPT` を予約名に足す
  - #64 の大小を無視する比べを `names.rs` の 1 つの関数に置き、計画・展開・中身の取り出しの比べをそれへ替える（書庫や宛先の名前どうしを ASCII だけで比べる所を残さない）
  - #6 のファイルと、別のエントリが暗に作るフォルダの衝突を大小の衝突として断る。明示のフォルダのエントリとその子は衝突にしない
  - 完了の状態: `superscript_com_and_lpt_names_are_reserved`・`a_file_collides_with_a_folder_another_entry_implies`・`an_explicit_folder_entry_and_its_children_do_not_collide`・`a_full_width_companion_folder_is_carved_out_case_insensitively`・`refresh_keep_matches_non_ascii_names_case_insensitively` が緑
  - _Requirements: 3.1, 4.1, 9.1_

- [ ] 6. 同梱の宛先・宛先の中のリンク・確定のテストを直す（T6）
  - #7 の 2 つの同梱が大小を無視して同じ宛先を指す書庫を、宛先の名前の拒みとして書庫ごと断る（語彙は増やさない）
  - #8 の既に在る宛先の中のリンク（ジャンクションを含む）をたどらず、そのパスを添えた段の失敗にする
  - #62 の doc と確かめの 1 行、#63 の書き換えの拒みの待ちをテストのときだけ 0 にする差し替え（本番の 2 秒は変えない）
  - #71 の読み手の無い `charset_declared` を消し、縛る assert と範囲の外の `crates/areka/src/install/judge_tests.rs` の 1 か所を直す
  - 完了の状態: `two_companions_naming_the_same_folder_refuse_the_archive`・`a_link_inside_an_existing_destination_fails_the_stage_with_its_path` が緑で、掴みっぱなしのテストが 2 秒待たずに同じ結果で通り、`cargo test -p areka` の install のテストも緑
  - _Requirements: 3.1, 3.2, 4.1, 5.2, 6.2, 9.1, 9.6, 9.7, 9.8, 9.9_

## `areka-user-data`

- [ ] 7. (P) 記憶の写しを自分の中へ入れず、古い作業フォルダを片付ける（T7）
  - #15 の写す先か作業の根が写す元の下にあれば、写す前に入力の誤りとして断る
  - #16 の作業フォルダの番地が先に在れば次の番号で取り直し、棚の直下の 24 時間より古い項目を写しの初めに片付ける（時刻は引数で渡す）。消せない物は既存の警告を出して続ける
  - #20 の先を越された判定を関数に出してテストし、#74 の木の歩きを再帰から積み上げに替える
  - 完了の状態: `refuses_to_copy_into_itself`・`a_taken_work_address_is_skipped`・`stale_work_folders_are_swept_and_fresh_ones_kept`・`a_raced_destination_returns_raced_and_drops_the_work`・`deep_trees_copy_on_a_small_stack` が緑
  - _Boundary: areka-user-data copy_
  - _Requirements: 3.1, 4.1, 4.2, 4.3, 5.2, 9.1_

- [ ] 8. 置き場の決めで使えない環境変数を捨て、失敗の道を記録で確かめる（T8）
  - #73 のテストの道具を新しい `test_support.rs` に 1 つずつ置き、写しを消す
  - #18 の空・相対の `AREKA_PROFILE_DIR` を、値を見る 4 か所すべてで入口の 1 回の検査の結果に揃え、入口で `error!` を 1 回出して書ける根の `profile\areka` へ進む。範囲の外の `crates/areka`・`crates/areka-mcp-bridge` のテストが空・相対の値の結果を見ていれば期待を直す
  - #19 の一覧の読み失敗を既存の警告で記録し、#21 の失敗の道のテストを記録の捕まえで包む。#72 の使わない引数を消し、範囲の外の `crates/areka/src/readonly_run_tests.rs` を直す
  - 完了の状態: `an_empty_or_relative_profile_env_is_ignored_with_an_error`・`readonly_reads_beside_when_the_profile_env_is_relative`・`establish_still_migrates_when_the_profile_env_is_empty`・`locate_answers_migration_pending_when_the_profile_env_is_empty`・`an_unreadable_listing_entry_is_logged` が緑で、`cargo test -p areka`・`-p areka-mcp-bridge` も緑
  - _Requirements: 3.1, 3.2, 4.1, 5.2, 6.2, 9.1, 9.4_

## `areka-ghost`（`src`）

- [ ] 9. (P) `src` の単体テストの道具を 1 か所にまとめる（T12）
  - #36 の有界の実行・システム変数・記録の sink・空の sink・ゴーストの fixture・有界の終了・OnBoot を待つ輪を新しい `test_support.rs` に 1 つずつ置き、写しと「取り込めない」の注を消す。dispatcher に固有の物だけを `dispatcher_test_support.rs` に残す（空になれば消す）
  - #84 の終了を有界の終了の道具へ替える
  - 完了の状態: 同じ形の道具の写しが `areka-ghost/src` に残らず、`runtime_tests.rs` が減って `cargo test -p areka-ghost` が緑
  - _Boundary: areka-ghost src test support_
  - _Requirements: 3.1, 5.3, 9.1_

- [ ] 10. `runtime_tests.rs` を減らし、記憶の口の写しを包括の実装へ替える（T13）
  - #37 の偽の起動の設定を 1 つの道具にし、#38 の偽の SHIORI を空の配線へ、#39 の終了を呼ばないテストを消し、#80 の空振りの 2 行と誤った注を消す
  - #40 の範囲の外の `crates/areka-sylphya/src/persist/io.rs` に `Arc<T>` への包括の実装（3 つの口を中身へ渡す）とそのテストを足し、範囲の中の 2 つの写しを消して直に渡す。範囲の外の写しは残す
  - 完了の状態: `runtime_tests.rs` が約 780 行に減り、`an_arc_forwards_all_three_methods` が緑で、`cargo test -p areka-sylphya` と `-p areka-ghost` が緑
  - _Depends: 9_
  - _Requirements: 3.1, 3.2, 5.1, 5.3, 6.2, 9.1, 9.5, 9.12_

- [ ] 11. (P) dispatcher のテストを減らし、刻みのテストを的に合わせる（T14）
  - #75 の話の組み立てと畳みを道具に置き換えてから、#31 の刻みのテストを同期の檻で `Tick(7_000)`→`Tick(7_500)` の形に直し、競る受け取りの 1 本を直す
  - #76 の必ず真の assert を消す
  - #79: `audit-parsers-sakura` の `areka-sakura` の警告のテストが `main` に在れば、ここで自分の側の写しとそれだけが使う道具を消す。無ければ 18.1 へ回す
  - 完了の状態: `dispatcher_choice_tests.rs` が約 790 行に減り、刻みの割り算を `/ 100.0` に変えると赤になる
  - _Boundary: areka-ghost dispatcher tests_
  - _Depends: 9_
  - _Requirements: 3.1, 4.5, 5.1, 9.1, 9.5, 9.14_

- [ ] 12. (P) 一覧と設定の読みを直す（T9）
  - #22 の OnBoot のシェル名を一覧と同じ読み方（鍵の大小を無視・空の値は無い）で決める。parsers の小文字の表が `main` に在ればそれへ乗り換えて自前の版を消し、無ければ自前の版を文字コードを引数に取る crate 内の関数にして使う（乗り換えは 18.1）
  - #23 の descript の読みを NotFound の形にし、#25 の一覧の名を決める公開の関数を置いて範囲の外の `crates/areka/src/mcp/resolve.rs` もそれを使う。#55 の同梱のバルーンを 1 回だけ復号する
  - #56 の 2 つの関数を消し、範囲の外の `crates/areka/src/boot_resolve*_tests.rs` の 4 つを直す。#78 の 4 行と注を消す
  - #24・#32・#35・#77 のテストを足し・まとめ・消す
  - 完了の状態: `shell_name_key_is_read_case_insensitively`・`an_empty_shell_name_falls_back_to_the_folder_name`・`a_descript_that_is_a_folder_warns_in_both_readers`・`listed_name_falls_back_to_the_absolute_root_without_a_trailing_separator` が緑で、`cargo test -p areka` も緑
  - 8 と同じ `crates/areka` のテストに触ることがあるので、8 の後に始める
  - _Boundary: areka-ghost catalog, config_
  - _Depends: 8, 9_
  - _Requirements: 3.1, 3.2, 4.1, 5.4, 6.1, 6.2, 9.1, 9.10, 9.14_

- [ ] 13. 同じプロセス内の SHIORI の読み込みを直す（T10）
  - 着手の前に `main` を取り込み、一時フォルダの番人の例外表の行から件数を数え直す（`audit-emo-present` も行を持つ）
  - #26 のファイル全体の allow とその段落を消し、出た警告の物を確かめて消す。#27 の範囲の外の `shiori-host32-host` の答えの読みを公開して使い、写しを消す
  - #28 の DLL を絶対パスで読み、DLL のフォルダを依存の探し先に入れて作業フォルダと PATH を外す。#29 のフォルダの整えを helper の道と同じ関数でそろえる
  - #33 の警告を記録の捕まえで見る。#34 の手写しの接続の道具を消し、`TempPath` の下に DLL を写した fixture から組む。#4 の src の 2 つのテストの一時フォルダを `TempPath` にし、例外表の 2 行を消して件数を合わせる
  - 完了の状態: `a_relative_dll_path_is_refused`・`inproc_connect_accepts_a_relative_mount_folder` が緑で、一時フォルダの番人（`cargo test -p log-capture-kit`）と `-p shiori-host32-host`・`-p areka-ghost` が緑
  - 10 と同じ `runtime_tests.rs`（#34 の DLL の探し）に触るので、10 の後に始める
  - _Depends: 10_
  - _Requirements: 3.1, 3.2, 4.1, 4.3, 5.2, 6.2, 9.1, 9.3, 9.11_

- [ ] 14. (P) 刻み・記録・sylphya の小さい直しをする（T11）
  - #30 の sylphya の barrier が死んだときの記録を `error!` に（続行は変えない）。`sylphya_wiring.rs` は 164〜172 行目だけを触り、`doc/` を名指す行は変えない
  - #53 の欄の改名、#54 の古い注、#58 の落ちないテスト、#59 の写しの本体、#81〜#83 の定数でも緑のテスト・名前のずれ・時間待ちの assert を直す
  - 完了の状態: `a_dead_sylphya_barrier_is_logged_as_error` が緑で、`ticker_tests.rs` に時間待ちの assert が残らない
  - _Boundary: areka-ghost sylphya_wiring, ticker, sink_
  - _Depends: 9_
  - _Requirements: 3.1, 4.1, 4.2, 5.2, 5.7, 9.1, 9.5_

## `areka-ghost`（`tests/`）

- [ ] 15. 共有ゴーストに錠を付け、その根を `target\` の下へ移す（T15）
  - 着手の前に `main` を取り込み、一時フォルダの番人の例外表の行から件数を数え直す
  - #45 の共有ゴーストを錠つきの持ち物で渡し、取った後で前の記録を消す。I1・I2 と組みの確かめはその持ち物を終了の確かめまで持つ
  - #5 の共有ゴーストの根を `<ワークスペース>\target\test-roots\areka_shiori4_test_ghost_shared` へ移す。#4 の I3 の一時フォルダを `TempPath` にする。例外表の 2 行と使われなくなる種別とその件数を消し、較正で外す 1 件を `crates/areka/src/install/fetch_url.rs` へ替える
  - #48 の使わない 4 つの物と名指す doc、#87 の檻の (c) 以外、#88 の古い注と理由の切れた allow を消す
  - 完了の状態: 錠を外すと `the_shared_ghost_is_not_reset_while_another_test_holds_it` が赤になり、一時フォルダの番人と較正 `dropping_a_known_exception_turns_the_guard_red` が緑で、`inproc_e2e_test.rs` の行数を測って記録する
  - _Depends: 13_
  - _Requirements: 3.1, 3.2, 4.1, 4.2, 4.3, 6.2, 9.1, 9.3_

- [ ] 16. 結合テストの入れ物と道具を 1 か所にまとめる（T16）
  - #42 の `BoxedBackend` を `recorder.rs` の 1 つにし、3 つのテストの写しと要らなくなった `use` を消す
  - #43 の helper の exe の探し、#44 の有界の実行と終了を `common.rs` の 1 つにして写しと待ちの輪を使い替える。#89 の有界の実行が panic と止まりを分けて報告する
  - #46・#47 の実物の pasta のテストを `SampleRoot` の形にそろえ、捨てている失敗を `expect` に、待ちの後に見たことを確かめる。#85 の駆動の輪を 1 つにし、#86 のずれの assert と #88 の理由の切れた allow を消す
  - 終わりに `inproc_e2e_test.rs` の行数を測る。1,000 行以下なら、1,000 行の番人の例外表に行を持つ並走の `audit-emo-compose`・`audit-emo-present`・`audit-seriko` へ先に知らせてから、範囲の外の `crates/log-capture-kit/tests/file_length_guard_test.rs` の表からその 1 行を消し、件数を表の行から数え直して合わせる
  - 完了の状態: 範囲に `BoxedBackend` の定義が 1 つだけで、I1・I2・snapshot・real_pasta_online が寄せる前と同じ結果で通り、`run_bounded_reports_a_panic_as_a_panic` が緑で、1,000 行の番人が緑
  - _Requirements: 3.1, 5.1, 5.3, 6.1, 6.2, 7.1, 7.2, 7.3, 9.1_

- [ ] 17. spine の結合テストの写しをまとめ、S7 を `TempPath` へ移す（T17）
  - 着手の前に `main` を取り込み、一時フォルダの番人の例外表の行から件数を数え直す
  - #50〜#52 の fixture・有界の合流・期待の組み立て・待ちの輪を親の `spine_e2e_test.rs` に 1 つずつ置き、子と `sylphya_integration_test` がそれを使う
  - #44 の spine の子の有界の実行と終了の写しを `common.rs` の道具へ使い替える
  - #49 の S5 を締切つきで起こして締切の道を見る。#4 の S7 を `TempPath` の下へ移し、例外表の 1 行を消して件数を合わせる
  - #90 の写しの型を消して捕まえた記録を直に見る。#91 の doc の文言を直す
  - 完了の状態: S5 が締切の止まりを見て緑で、spine の子に写しの道具が残らず、一時フォルダの番人が緑
  - _Requirements: 3.1, 4.3, 4.5, 6.2, 9.1, 9.3_

## 仕上げ

- [ ] 18. 相手待ちの直しを入れ、audit をかけ直し、全体テストを通す（T18）
- [ ] 18.1 `audit-parsers-sakura` の着地を取り込み、待っていた 2 件を仕上げる
  - `main` を取り込み、parsers の小文字の表と `areka-sakura` の警告のテストが入っているかを確かめる。入っていなければ、ここで止めて開発者へ報告する（自前の写しを残したまま完了しない）
  - #22 の自前の小文字の表を消して、一覧とシェル名の読みの両方を parsers の表へ乗り換える。#79 が 11 で済んでいなければ、ここで自分の側の写しを消す
  - 先方の `resolve` の撤去と行がぶつかっていれば、取り込みの中で合わせる
  - 完了の状態: `catalog.rs` に自前の小文字の表が無く、`shell_name_key_is_read_case_insensitively` が乗り換えの後も緑で、`dispatcher_choice_timeout_tests.rs` に移した警告のテストが無い
  - _Depends: 11, 12_
  - _Requirements: 6.1, 9.10, 9.14_

- [ ] 18.2 同じ範囲へ audit をかけ直し、新しい指摘を 0 件にする
  - 範囲のファイルの数と行数を測り直してから、3 つのクレートへ `/ponytail:ponytail-audit` を要件 1 と同じ作法（上限なし・全部読む）でかける
  - 出た指摘のうち直さない 3 件（#17・#41・#57）は数えない。新しい指摘は直し、直しが生んだ指摘も含めて 0 件になるまで繰り返す。直さずに残す形のものが出たら直さずに議題にする（要件 2 の作法）。直しが 20 タスクを越えそうなら残る Nice to have を 2 本目の spec へ回す議題にする
  - かけ直しの結果（回数・各回の件数・最後の 0 件・直さない 3 件の表）を `research.md` に「かけ直しの結果」として残す
  - 完了の状態: `research.md` に最後のかけ直しが新しい指摘 0 件だったことが記録されている
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.4, 8.1, 8.2, 8.3, 9.2, 9.13_

- [ ] 18.3 全体テストを通し、範囲の約束を確かめる
  - `pwsh -NoProfile -File tools/test-all.ps1` を 1 回回す（裏で走らせ、時間を先に伝える）
  - `main` との差分で `Cargo.lock`・`vendors/`・`doc/`・`docs/` を名指す行が変わっていないことと、範囲の外の変更が「範囲の外で触るファイル」の表の中だけであることを確かめる
  - 完了の状態: 全体テストが緑で終わり、差分の確かめで約束の外の変更が 0 件
  - _Requirements: 4.4, 5.5, 5.6, 5.7, 6.2, 6.3, 8.4_
