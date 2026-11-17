# 6.1 かけ直し 2 回目の指摘（42 件・必ず直す 1）と仕分け

監査役の推奨の直し方で直す。外への約束が変わるもの（R1・R3・R7・S6・G1）は、audit 系の議題は推奨案で承認してよいという取り決め（11-04）に従い進行役が推奨案を採った。S7 だけは当てない（根の `Cargo.toml` と範囲の外の 2 crate を触る＝C22 の約束。`/kiro-complete` の棚卸で起票）。

## sample-ghost-kit（束 A）
- S1 devroot.rs ~334「窓口の 3 つの入口」が偽で数つき → 入口の名前（[`fresh_root`]・[`manual_root`]）を挙げ数を外す
- S2 Cargo.toml:4 description の「leaf crate」が偽 → 「…を唯一定義するテスト専用の crate（`[dev-dependencies]` からだけ引かれる）」の類
- S3 devroot.rs ~14-15 の module doc の CARGO_TARGET_DIR の説明が 1 回目の S3 に追いついていない → 「頭の `.`・`..` を外した名前の段で終わる最も近いもの」
- S4 lib.rs ~248-249「4 通りの食い違い」→ 数を外す
- S5 bin/nar-sample-path.rs ~29-31 の 2 つの主張（標準出力へ 1 行も出さない・`folder=` の 1 行を読む）が偽 → 正しい文に
- S6 fold-samples.rs ~69-78 が知らない引数を黙って無視する → `--from`・値・`--check` のほかは `unknown argument <x>` で断る（ASCII・終了コードは今の失敗と同じ）。較正のテスト 1 本
- S7 当てない（上記）
- U1 fold-samples_tests.rs に食い違いを数える道の較正が無い → 1 バイト違う書庫を置き `fold_one(.., check_only = true)` が 1 以上を返すテスト
- U2 devroot_cache_tests.rs の説明と失敗の文の `cache/`（~1・20・102・273・278・303・452・471・519・571）→ [`CACHE`]／「原本の棚」。~399 の古い世代の `cache\` は正しいので残す
- U3 lib_tests.rs ~715-716 の偽の理由（load-flake.ps1 が `Io` の表示を読む）→ 括弧の理由を消す
- U4 devroot_cache_tests.rs ~68-69 の数と英語の judge → 直す
- U5 devroot_fetched_tests.rs ~194「上の 2 本」→ 2 本の名前を書く
- U6 lib_tests.rs ~681-696 `a_descript_without_a_name_line_gets_one_appended` → 中身に合う名前（置き換えか足すか）
- U7 fetched_tests.rs ~242 `GOOD_SHA` が `GOOD` のハッシュでない → `ANY_SHA` などに改名し使う 2 本の上へ
- U8 刻印の手書きの写し（devroot_cache_tests.rs ~458-459・devroot_sweep_tests.rs ~307-312・~173）→ `stamp(&nar)` を使う（形を固定する ~179-183 は残す）
- U9 fold-samples_tests.rs:9「検体 emo2 と同じ形」が偽 → 直す

## real-run-kit（束 B）
- R1【必ず直す】i686.rs ~133 の警告の行が `eprintln!` で、通ったテストでは libtest が捨てる → `writeln!(std::io::stderr(), ..)` に直に（temp-path-kit と同じ）。檻のテストが戻り値しか見ない点は、標準エラーへの書き先を引数で受ける形にして書かれた字面を確かめるか、それができないなら理由を控える
- R2 「つなぎを使い回す」をどのテストも確かめていない（fake_mcp_test.rs ~160・~261-264、mcp.rs ~188・192）→ mcp_tests.rs の兄弟テストで `mcp.link.is_some()` を確かめ、結合テストの 2 つの説明を「使い回しは mcp_tests が確かめる」に
- R3 launch.rs ~388・393 で span を付けた指定（`areka_mcp::server[req]=debug`）を名指しに数える → `[` を含む指定は名指しに数えない・表に 1 行（その形のときだけ子へ渡す RUST_LOG が変わる）
- R4 output_tests.rs ~152・164・fake_wait_close_test.rs ~174 の要らない `create_dir_all` → 消す
- R5 evidence_tests.rs ~193・233-238 の必ず真の確かめと手助け → 1 行の確かめにし手助けを消す
- R6 古びる数（launch_tests.rs ~132-134 `four_launch_variants_…`・~41「ほかの 3 つ」・launch.rs ~213）→ 改名 `launch_variants_resolve_positional_args_and_root`・数を外す
- R7 i686.rs ~116・120 の一時ファイルの消しの失敗を黙って捨てる → 消せなかったら ASCII の 1 行を `std::io::stderr()` へ直に（116 は続く panic の文に添える）
- R8 close.rs ~343・output.rs ~91 の送り直しのたびに出力のファイルを頭から読む → `wait_any` が `&mut [Tail; 2]` を受け、`close_or_kill` で 1 度だけ作って使い回す

## log-capture-kit src・temp-path-kit（束 C）
- L1 probe_tests.rs ~4-6 の無効側の説明が事実と違う → 監査役の言い直しの類
- L2 Cargo.toml ~26-28「テスト専用の leaf 同士なので依存の向きは崩れない」が偽 → dev の輪を正しく書く
- L3 移行前のコードを「現行」「既存」「今書いている」と現在形で書く文（event_tests.rs ~47・58・69・71・144・167・177・188・198・339、event.rs ~23、lib.rs ~151、capture_calibration_test.rs ~23・103）→ 「移行前の」に。event.rs:150・event_tests.rs:18-19・121 は触らない。失敗の文も直す
- L4 expect.rs:3「中身を変えずに移した」→ 後で `#[track_caller]` を足したことを書く
- L5 expect_tests.rs ~8-15 の手組みの message の形が本物と違う → `debug: message.to_string(), str_raw: None`、説明の「同じ形」を直す
- L6 filter_tests.rs ~161・167・289・340 `TARGET_SHAPE` を 2 本が共有（説明は 1 テスト 1 宛先）→ 専用の定数
- L7 lib.rs ~170 のその時点の測りの括弧 → 消す
- L8 要件・設計・タスクの番号に spec の名が無い → 各ファイルの module doc に「番号は <spec> のもの」を 1 行（どの spec かは git log -S で確かめる）
- T1 temp-path-kit lib.rs ~16「この仕様の反復検証で…赤になっている」→ spec の名・過去形
- T2 lib_tests.rs ~172-177 の重なる `assert_ne!` → 消す

## log-capture-kit の見張り（束 D）
- G1 捕捉の見張りが波括弧の中の別名と dispatcher 側の別名を見ない → `concat!("with_", "default as")`・`concat!("set_", "default as")`・`concat!("set_global_", "default as")` の 3 語と較正（判定が厳しくなる・今の当たり 0）
- G2 `every_shape_of_a_bundled_balloon_path_is_detected` の名前が偽（7 形のうち 4 つだけ）→ 中身に合う名前に改める（例 `the_slash_and_join_shapes_of_a_bundled_balloon_path_are_detected`）
- G3 doc_tree_guard_test.rs ~611-616 `the_whole_doc_tree_reading_finds_something` も主検査と重なり注釈が偽 → テストを消す
- G4 temp_path_guard_test.rs:7「入口は 2 通りの綴り」→ 数を書かない文
- G5 workspace_scan_test.rs ~111-112・124「本番の 3 語」→ 「この見本の 3 語」
- G6 with_default_guard_test.rs ~105-107 の例外表の説明が中身と合わない・経緯の文 → 「載る理由は各項の理由欄が持つ」
- G7 with_default_guard_test.rs ~62 `TOKEN_SET_DEFAULT` の説明 → 両方の関数に当たると書く
- G8 workspace_scan/mod.rs ~101・376「決め D7」→ `audit-test-kits 決め D7`

## 束 D の直しの控え（承認済み）
- 較正: G1 は足した `… as` の 3 語を外す → `an_aliased_import_or_an_uncalled_direct_capture_is_detected` が赤。今の木での当たり 0
- 改名・消し: `every_shape_of_a_bundled_balloon_path_is_detected` → `the_non_backslash_shapes_of_a_bundled_balloon_path_are_detected`、`the_whole_doc_tree_reading_finds_something` を消した（どちらも名前の当たりはかけ直しの記録だけ）

## 束 A の直しの控え（承認済み）
- 較正: S6 は知らない引数を黙って飛ばす形に・U1 は食い違いを数えない形に → `unknown_arguments_are_refused_not_ignored`・`check_only_counts_a_one_byte_difference_from_the_vendored_archive` が赤
- 改名: `a_descript_without_a_name_line_gets_one_appended` → `named_descript_replaces_the_name_line_or_appends_one`、定数 `GOOD_SHA` → `ANY_SHA`（どちらも当たりは定義とこの記録だけ）
- S6 の副作用: `--from` を 2 度書くと後の値が勝つ（前は先の値・保守の道具なので受け入れた）

## 束 C の直しの控え（承認済み）
- 較正: L5 は手組みを古い形（引用符つき・`str_raw: Some`）に戻す → `ちょうど_1_件なら返し_項目も引ける` が赤。消したもの: temp-path-kit の重なる `assert_ne!` 1 つ（名前の付いたものは無し）
- 残り（3 回目で見る）: temp-path-kit の `要件 12.1` に spec の名の注記が無い（L8 は log-capture-kit だけを挙げた）

## 束 B の直しの控え（承認済み）
- 較正: R1 は行を書かない形に → `rename_failure_passes_with_an_ascii_line_when_dest_is_another_32bit_copy` が赤（書き先を引数で受け、`Vec<u8>` に書かれた字面を確かめる）。R7 は消しの失敗を捨てる → `remove_tmp_reports_only_a_failure_other_than_not_found` が赤。R2 は応答と断りの腕でつなぎを捨てる → `the_link_is_kept_after_a_reply_and_after_a_refusal` が赤。R3 は古い `split('[')` に戻す → `compose_rust_log_table` が赤。R8 は `wait_any` の頭で位置を戻す → `a_second_wait_continues_from_where_the_tails_stopped` が赤
- 改名・消し: `four_launch_variants_resolve_positional_args_and_root` → `launch_variants_resolve_positional_args_and_root`（tasks.md の過去の控えが旧名を挙げる・直さない）、手助け `target_test_roots_of` を消した（使い手は同じファイルの 1 だけ）
