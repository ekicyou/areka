# かけ直し 1 回目・G1（入口・入出力・共通）

指摘 20 件（必ず直す 0・直したほうがよい 6・できれば 14）。どれも残す 3 種類に当たらない。読んだファイル 32 本すべて全行。

| id | 重さ | file:line | 何が違うか | 直し方 | 振る舞い |
|---|---|---|---|---|---|
| G1-1 | 直したほうがよい | src/cli/generate.rs:155-158・167-175・202-204・230-238・288-290 | ledger_init_with・report・priority_apply は 1 本書くたびに書き手の中で announce する。出力の先が途中で閉じると 2 本目の告知で Err になり 3・4 本目が書かれず、「新旧の混ざった報告が残らない」約束（182-183 行の注記）が破れる | 書き手から announce を外し、書き出しの輪が全部終わってから 1 行ずつ告げる | yes（閉じたときの告知と書き出しの順だけ・文面は同じ） |
| G1-2 | 直したほうがよい | src/cli/cli_tests.rs:151-156 | a_subcommand_on_its_own_draws_no_complaint が本物の evidence を走らせる（本物のカタログ・crates/** を歩く）。冒頭の「副手続きの中身はここで走らせない」と矛盾。同じ主張は tests/cli_streams.rs の candidates のテストが持つ | このテストを消す | no |
| G1-3 | 直したほうがよい | src/io/sources_tests.rs:24-28・43-48・170-181 | よそのクレートのパス（areka-sylphya の dotted.rs・shiori_resource.rs）と本文の "ukadoc" に頼る。よその spec の整理で偽の赤になる（files_tests.rs の方針とも食い違う） | 肯定側は「自クレート以外の .rs が 1 本以上」「/src/・/tests/ を含むパスがある」に替え、名指しと "ukadoc" の確かめを消す | no |
| G1-4 | 直したほうがよい | src/lib_test_support.rs:437-447 と src/cli/inspect.rs:360-371 | 取り出し→解決（flat_map(extract)→resolve）の写しが 2 か所。注記は「実行ファイルと同じ経路」と言うが手写し | 純粋な fn index_of(sources, catalog) -> EvidenceIndex を 1 つ置き、collect_evidence とテスト側がそれを呼ぶ | no |
| G1-5 | 直したほうがよい | src/cli/generate_tests.rs:191-251（冒頭の箇条）・109-127 | the_prologue_lists_the_pages_in_name_order・the_prologue_order_is_not_the_transcription_order_…・a_one_page_domain_still_declares_its_page は pages_of を呼ぶだけで assignment_tests の逐語テストの写し。no_id_is_dropped_and_no_id_is_placed_twice は直前の each_sample_id_lands_… に含まれる | 4 本を消し、冒頭の箇条から pages_of の行を外す | no |
| G1-6 | 直したほうがよい | src/io/paths.rs:88-92 | 「判定 ⑹ が全文一致で突き合わせる」—⑹ はもう無い。今は check::freshness が DomainReportStale で見る | 「check::freshness が DomainReportStale として全文一致で見張る」に直す | no |
| G1-7 | できれば | Cargo.toml:20・23 | 行番号つきの参照（tech.md:56・crates/dola/Cargo.toml:21-23）が古い | 行番号を消す | no |
| G1-8 | できれば | src/lib.rs:9-12 | 純粋層の一覧から documents が抜けている | [`documents`] を足す | no |
| G1-9 | できれば | src/diff_tests.rs:7-8 | 「差分の算出そのものがスナップショットを要る」は逆 | 「新しいカタログを建てる段がスナップショットを要る」に直す | no |
| G1-10 | できれば | src/tomlout_tests.rs:192 | 見出し「6.」—冒頭の一覧は 5 つ | 「5.」に | no |
| G1-11 | できれば | src/io/snapshot_tests.rs:117 | 「上の 2 本」—実際は 3 本 | 「上の 3 本」に | no |
| G1-12 | できれば | src/io/paths_tests.rs:5・65 | 「10 本すべて逐語で」—逐語は 9 関数 | 数を書かない形か正しい数に | no |
| G1-13 | できれば | src/assignment_tests.rs:247 | 「取り違わる」は壊れた語 | 「取り違えられる」に | no |
| G1-14 | できれば | src/cli/generate_tests.rs:418-452 | the_hand_written_columns_are_still_spelled_… は直前の逐語の等式に含まれる言い直し | 消す | no |
| G1-15 | できれば | src/model_tests.rs:226-234 | link_holds_a_kind_and_a_parsed_id は欄に入れた値を読むだけ | 消す | no |
| G1-16 | できれば | src/tomlout_tests.rs:334-348 | inline_table_keeps_the_given_order_and_does_not_sort は他の 2 本が捕まえる | 消す（351-355 の配列側は残す） | no |
| G1-17 | できれば | src/assignment_tests.rs:158-173・194-202 | domain_of_round_trips_every_assigned_page と unassigned_is_empty_when_every_page_is_assigned は同じ BTreeMap を 2 方向から読むだけ | 2 本とも消す | no |
| G1-18 | できれば | src/lib_test_support.rs:430・450 | sources()・domain_reports() が pub(crate) だが外から使われない | private に | no |
| G1-19 | できれば | src/cli/generate_tests.rs:502-505・521-524 | 文字列の \n が生の改行に化けてリテラルが 2 行に割れている | 1 行の逃がしの形に戻す（python でバイトを書く） | no |
| G1-20 | できれば | src/io/snapshot.rs:85・278-283 | ENTRY_FIELDS が位置の添字で引くためだけの配列 | 6 つの綴りを entry_string の呼び出しに直接書き、定数を消す | no |
