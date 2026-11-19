# 6.1 かけ直し 3 回目の指摘（34 件・必ず直す 0）と仕分け

監査役の推奨の直し方で直す。外への約束が変わるもの（S1・S4・S5・R5）は、audit 系の議題は推奨案で承認してよいという取り決め（11-04）に従い進行役が推奨案を採った。

## sample-ghost-kit（束 A）
- S1 devroot.rs ~819-831 `report_cleanup` が `eprintln!` で、通ったテストでは libtest が捨てる → 書き先を `&mut dyn Write` で受け、本番は `std::io::stderr()`。テスト 1 本（`Vec<u8>` に書かれた字面が `sample-ghost-kit: cleanup failed (` で始まり、NotFound では何も書かない）。頭 `sample-ghost-kit:` と `os error N` は保つ（`tools/load-flake.ps1` が字面で読む）。load-flake.ps1 の 32-33 行の説明が古びる点は範囲の外（`/kiro-complete` の棚卸で扱う）
- S2 lib.rs:43「`[dev-dependencies]` からのみ引く」→ real-run-kit の例外を書く（Cargo.toml の description と合わせる）
- S3 nar_writer.rs ~7-9 の `ghost-install` は crate ではない → `areka` の `[dev-dependencies]`／`areka` のテスト
- S4 fold-samples.rs ~87・97・151・153・165・171・193 と nar_writer.rs `fold_into` ~329・344・348 の入出力の失敗にパスが付かない → 少なくとも 151 行と `fold_into` の `read_dir`・`read` にパスを添えた `map_err`（`fold_tree` の `io::Error` の文が変わる・種別は同じ）
- S5 bin/nar-sample-path.rs ~72 が 2 つ目より後の引数を黙って捨てる → `nar-sample-path: unexpected argument <x>`（ASCII）で 2。較正 1 本（余分な引数のときの終了コードが 0 から 2 に・package.ps1 は 1 つしか渡さない）
- S6 fold-samples.rs ~337「ghost/emo2=110」→ `ghost/<名>=<件数>`
- S7 devroot.rs ~24-27 の木の図の `gc-…` の説明と札 → 「退けた木（古い原本・残骸・前の世代の棚）」と `gc-…lock`
- U1 nar_sample_path_test.rs ~74-76・~205 の読み手の古い主張 → package.ps1 の `Read-SamplePaths` が読む・読み手は終了コードで止まる
- U2 fold-samples_tests.rs ~3-4「どれも fold_one を回し」→ `parse_args` の 1 本を除くと書く
- U3 devroot_cache_tests.rs ~166「（完了状態の 2 本）」→ 括弧を消す
- U4 古びる数（devroot_sweep_tests.rs ~14「下の 3 本」・fetched_tests.rs ~48「読みの 2 本」・lib_tests.rs ~313「4 通り」）→ 名前を書くか数を消す
- U5 module doc の要件番号に spec の名が無く一覧が中身と合わない（lib_tests.rs ~2・devroot_cache_tests.rs ~2・devroot_sweep_tests.rs ~1・devroot_tests.rs ~15）→ 「番号は spec `areka-P0-nar-install` のもの」を 1 行、一覧をテストが挙げる番号に合わせる（git log -S で確かめる）
- U6 lib_tests.rs ~125-128・171・189 `current_rows_are_registered_as_vendored` と「今の行」→ 改名と言い換え
- U7 fold-samples_tests.rs ~79-102 の check の道の 0 の側 → 書き替える前に `fold_one(.., true)? == 0` を 1 行
- U8 devroot_sweep_tests.rs ~664 の日本語の `what`（"残骸"）→ `"leftover tree"`
- U9 lib_tests.rs ~741-773 `the_other_errors_are_shown_in_ascii_only` が `is_ascii` だけ → 各文に `\u{` を含むことを足す

## real-run-kit（束 B）
- R1 close.rs ~211-223 `send_close` が taskkill の cp932 の出力を `from_utf8_lossy` で文に入れる → 文を捨て終了コードだけ残す・説明の「文は診断に添える」を消す
- R2 この spec の指摘・決めの番号に spec の名が無い（run.rs ~4・211、close.rs ~101、launch_tests.rs ~86・270、mcp_tests.rs ~234・270・330、output_tests.rs ~175・186、fake_mcp_test.rs ~120・135、fake_wait_close_test.rs ~182・202・211）→ `audit-test-kits 指摘 N`・`audit-test-kits 決め DN` の形に
- R3 mcp_tests.rs ~1-2 の module doc が中身と合わない → 同じプロセスの本物の受け口での 4 本を書く
- R4 fake_mcp_test.rs ~123 `an_mcp_error_reply_is_refused_on_the_same_link` は同じつなぎかを確かめない → `an_mcp_error_reply_is_refused_and_the_next_call_still_answers` などに改名
- R5 偽の子の `ENV_NAMES` に `PASTA_DEBUG` が無い → 足し、結合テスト `child_receives_env_and_args` に `PASTA_DEBUG` が `0` の確かめ（偽の子の env の出力が 1 行増える）
- R6 Cargo.toml ~21「例外 1 件」→ 数を外す
- R7 launch.rs ~378 外部 crate の行番号 `:246` → 外す
- R8 launch_tests.rs ~237「段が info より細かくても」→ 「段が何であっても」

## log-capture-kit src・temp-path-kit（束 C）
- L1 capture_calibration_test.rs ~65-73 の子が `AREKA_LOG_CAPTURE_PROBES` を受け継ぐ → `run_child` の `Command` に `.env_remove("AREKA_LOG_CAPTURE_PROBES")`、probe_tests.rs ~5-7 の説明に無効側で ⒜ のうち窓の内側で他スレッドが先に登録する 1 本が順しだいで赤になり得ることを添える（レビューが測って 2 本ではなく 1 本と確かめた）
- L2 probe.rs ~150-153「3 本とも」→ 消す
- T1 temp-path-kit の「要件 12.1」に spec の名（lib.rs ~87・lib_tests.rs ~1・28）→ lib_tests の module doc に 1 行・lib.rs の括弧に spec の名

## log-capture-kit の見張り（束 D）
- G1 tests/ の 6 ファイルの要件・設計・タスクの番号に spec の名が無い（mod.rs ~1・407・491・519、workspace_scan_test.rs ~1・257・469・524、with_default_guard_test.rs ~1・14、file_length_guard_test.rs ~1（module doc の 1 行だけ・表には触らない）、temp_path_guard_test.rs ~1・12・276、sample_path_guard_test.rs ~1・93・232）→ 各 module doc に主な spec の 1 行、混ざる所は少ない側に spec の名を添える（git log -S で確かめる）
- G2 「走査語を逐語で書かない」の説明が開き括弧つきの語だけを言う（mod.rs ~28・with_default_guard_test.rs ~18-21・temp_path_guard_test.rs ~54-55）→ 括弧の有無を問わず
- G3 中括弧で別名無しに取り込み呼ばずに渡す形が 2 つの見張りをすり抜ける → 語は足さず、2 つの走査語の一覧の上に `shortcut:` のコメントで限界を書く
- G4 sample_path_guard_test.rs ~256-329 の `Hit`・`scan_sources`・`scan_text`・`render` の写し → `workspace_scan::scan_sources` の結果に語から形を引く関数を当てる（失敗の文の形は少し変わってよい）
- G5 doc_tree_guard_test.rs ~551-582 `a_docs_dir_turns_the_guard_red` が別の検査 2 つを抱える → 別のテストに分ける
- G6 temp_path_guard_test.rs ~359-375 の「2 ファイルとも読めている」→ `GATEWAY_FILES` の 2 つとも確かめる
- G7 with_default_guard_test.rs ~291 のテスト名の一部が偽 → `the_shared_crate_definition_really_holds_the_direct_calls`

## 束 C の直しの控え（承認済み）
- 較正: L1 は親で `AREKA_LOG_CAPTURE_PROBES=off` を立てて較正を回す → 直す前は `bare_capture_drops_what_hardened_capture_keeps` が赤、直した後は緑。無効側では `captures_event_whose_callsite_another_thread_registers_inside_the_window` だけが赤（`--test-threads=1` で測った）

## 束 B の直しの控え（承認済み）
- 較正: R1 は `send_close` を元の形（taskkill の文を添える）に戻す → `send_close_records_only_the_exit_code` が赤（cp932 の文が化けて入る）。R5 は `PASTA_DEBUG` の名を変える → `child_receives_env_and_args` が赤
- 改名: `an_mcp_error_reply_is_refused_on_the_same_link` → `an_mcp_error_reply_is_refused_and_the_next_call_still_answers`（本 spec の design.md・tasks.md が旧名を挙げる・記録として残す）

## 束 A の直しの控え（承認済み）
- 較正: S1 は NotFound の早い戻りを外す → `a_cleanup_failure_is_written_to_the_given_sink_and_not_found_writes_nothing` が赤。S4 は `fold_into` の `with_path` を外す → `folding_a_missing_tree_fails_loudly` が赤（文にパスが無い）。S5 は余分な引数を黙って捨てる形に → `an_extra_argument_exits_with_2` が赤（0 で終わる）
- 改名: `current_rows_are_registered_as_vendored` → `the_rows_with_a_nar_in_the_repository_are_vendored_and_the_rest_are_url_rows`（completed の 6 文書が旧名を挙げる・直さない）
- S4 は `fold_into` の 3 か所と fold-samples の写しの 2 か所にパスを添えた（ほかの `?` は「少なくとも」の外として残す）
- 範囲の外（`/kiro-complete` の棚卸で扱う）: `tools/load-flake.ps1` の 32-33 行の説明（「-NoCapture が無いと赤のテストの分しか残らない」）が S1 の後は古い

## 束 D の直しの控え（承認済み）
- 較正: G6 は窓口の 2 つ目のファイルを語の無いファイルに差し替える → `the_gateway_is_scanned_and_holds_no_entry_point` が赤（直す前の確かめは緑のまま＝穴だった）。G4 は `GATEWAY_DEFINITION_PREFIX` を壊して HEAD の版と新しい版の当たりを比べる → どちらも同じ 1 件（当たりは変わらない）
- 改名・分け: `the_shared_crate_definition_is_the_only_excluded_region_and_it_really_holds_the_calls` → `the_shared_crate_definition_really_holds_the_direct_calls`（completed の 4 文書が旧名を挙げる）、`a_docs_dir_turns_the_guard_red` から `an_empty_reading_is_named_and_turns_the_guard_red`・`an_unlisted_entry_and_a_row_naming_nothing_are_findings` を分けた（主張は同じ）
- file_length_guard_test.rs は module doc に 2 行（spec の名の 1 行と空行）を足しただけ・上限越えの表と件数と 4 ファイルは変わらない
