# 6.1 かけ直し 1 回目の指摘（38 件・必ず直す 0）と仕分け

全部、監査役の推奨の直し方で直す。外への約束が変わるもの（S1・S2・S3・R1・R3・L3・G2・G4）は、audit 系の議題は推奨案で承認してよいという取り決め（11-04）に従い、進行役が推奨案を採った（要件 6.7・7.6 の議題を推奨案で承認・完了の報告で伝える）。

## sample-ghost-kit（束 A）
- S1 `SampleError` の `Io` 以外の表示（`Nar`・`UnknownSample`・`UnknownBalloon`・`RegistryMismatch`・`TargetDirNotFound`）が ASCII でない（lib.rs ~621-655）→ 値を `ascii(..)` に通す・日本語の `NarError` を入れた表示が ASCII だけのテストを 1 本（`the_io_error_is_shown_in_ascii_only` に並べる）。nar-sample-path の標準エラーの文が変わる（終了コードは同じ・字面で読む所 0）
- S2 `fold-samples` が端末へ出す文と `main` の誤り・`nar_writer.rs` の `fold_tree` の `InvalidData` の文が日本語（fold-samples.rs ~63-273・nar_writer.rs ~335）→ 英語の ASCII（パスは `ascii`）。「全ての検体で一致」は 1 つのフォルダだけを畳む今と合わないので「round trip matched」の類に。panic の文は家の形のまま
- S3 D4 の祖先の照らしが `..\x`・`.\x` に効かない（devroot.rs ~137-146）→ 頭の `CurDir`・`ParentDir` を外した `Normal` の段だけで `ends_with`（空なら今の absolute）。devroot_tests の相対のテストに `..\tgt`・`.\tgt` を足す
- S4 devroot.rs の関数の説明が棚を `cache/` と書く（~195・278・378・395・433・501-503）・fetched.rs の module doc の `cache\` も → [`CACHE`] に
- S5 説明の古びる数（lib.rs ~10-11「38 か所」・fold-samples.rs ~29-30 の数）→ 数を外す
- U1 `copy_master` の写しかけを消す枝に檻が無い（devroot_sweep_tests.rs ~112-139）→ 宛先に `stale.txt` を先に置く
- U2 nar_writer_tests.rs ~256 の `b"PK.."` に生の 0x03 0x04 → `b"PK\x03\x04"` の逃がしで書きバイトで確かめる
- U3 pasta_inner_balloon_test.rs の「名前の行が無い」枝が較正されていない（~28-36・49-98）→ 正しい場所に名前の行の無い `descript.txt` を直に書いて `is_err()`。外に置く場面も直に書く形に
- U4 は直さない（指摘にしない）: `staging_uses_the_verified_bytes_not_a_second_read` は 4.2 の較正で「据え付けが元のパスを読み直す」と赤になった＝確かめている
- U5 試料の `.nar` を書いて読み戻す往復の残り（devroot_cache_tests.rs `tiny_ghost_nar`・`bytes_of`、lib_tests.rs `probe_nar`・`fixtures`）→ バイトを直に渡す
- U6 fetched_tests.rs `tiny_nar` が `tiny_ghost_bytes` の写し → ただのバイトに
- U7 テストの説明と名前の偽と数（devroot_cache_tests.rs:13「6.6 MB」・devroot_fetched_tests.rs:5-7・nar_sample_path_test.rs:74・fetched_tests.rs:45・lib_tests.rs:6-14・135・158・166・devroot_fetched_tests.rs:94 のテスト名）→ 正しい文・数を外す・名前を中身に
- U8 「指摘 N」「檻 N」「決め DN」に spec の名が無い（監査役の挙げた各行）→ 出どころの spec の名を添える（`audit-test-kits 指摘 N` の形・ほかの crate の書き方に合わせる）
- U9 fetched_tests.rs ~285-287 `archive_path_of_rows` が先頭の行を Vendored と決めつける → `.find(|s| s.source == Vendored)`
- U10 nar_sample_path_test.rs ~161-201 の 2 回呼ぶテストが `emo2` を使う → `R_POST_and_KOMAINU`
- U11 devroot_tests.rs ~339 の要らない後始末 `remove_dir_all(&manual)` → 消す

## real-run-kit（束 B）
- R1 `place` の改名が落ちて置き先が 32bit なら黙って通す（i686.rs ~309-317）→ 落ちた後で `same_copy` を見て、違うなら ASCII の 1 行 `real-run-kit: could not replace <dest> (<err>); using the existing 32-bit copy` を標準エラーへ（`ascii` を通す）。テストを「同じなら黙る・違うなら行を出して通す」に分ける（標準エラーの行が 1 行増える）
- R2 launch_tests.rs ~38 `removes_four_developer_vars_and_sets_defaults` の名前が偽 → 中身に合わせて改名
- R3 `compose_rust_log` が数字の段（0〜5）を出所の名と取り違える（launch.rs ~388-391）→ 開く腕に `3|4|5`・閉じる腕に `0|1|2`、表に 2 行（子へ渡す RUST_LOG が数字の段のときだけ変わる）
- R4 fake_mcp_test.rs ~113-114 の説明が破棄の panic を書かない → 直す
- R5 fake_mcp_test.rs ~229-243 が既定の猶予 2 秒を手で写す → 時間の確かめを mcp_tests.rs へ移し、`exit_grace` 30 秒・短い締切の `Limits` で `Refused` が速く返ることを確かめる（serve() に型つきの入力の道具）。結合テストは `Refused` とつなぎの使い回しだけに
- R6 output.rs ~313「run とクレートの入口が再輸出する」が偽 → 直す
- 進行役の控えから: output_tests.rs ~34 の偽の待受行も本物の形（`url=http://127.0.0.1:<port>/api/mcp/v1`）に。D2 のテスト `relied_targets_…` に `RELIED_TARGETS.len()` と語の表の数の一致の確かめを足す（zip が黙って飛ばさないように）

## log-capture-kit src・temp-path-kit（束 C）
- L1 capture_tests.rs ~219-237 の冪等のテストが冪等を確かめていない（2.2 のレビューは多スレッドの同時呼びの確かめとして残すと決めた）→ 消さず、名前と説明を「多スレッドから同時に呼んでも panic せず、その後に窓が開ける」に改める
- L2 event.rs ~87-90・106-109・114-117・event_tests.rs ~364-365 が今は無い型と「同じ」と現在形で書く → 「移行前の〜と同じ」の出自の言い方に（event.rs:150・event_tests.rs:18-19・121 は触らない）
- L3 probe.rs ~104-106・130 は「逐語」と書いて `trim` する → `trim` を外す（probe.rs と probe_tests.rs:21）。前後に空白のある値を断るようになる（設定する所 0）
- L4 capture_tests.rs ~189-192 の重なる確かめ → 消す
- L5 event_tests.rs:33「7 件で」→ 数を外す
- L6 lib.rs:204「どの crate にも依存しない leaf」→ 通常の依存の話と dev-dependency を分けて正しく
- T1 temp-path-kit の写した `ascii` に直接のテストが無い → lib_tests に 1 本

## log-capture-kit の見張り（束 D）
- G1 `scan_tokens` のふるいが遅い（検体の見張りの主検査 44 秒）→ `firsts` を sort・dedup、内側を `order.iter().filter(|t| t.starts_with(ch))`（当たりは同じ）。前後の時間を控える
- G2 捕捉の見張り ⑴ が `use … as` の別名と `(` 無しの渡し方を見ない（with_default_guard_test.rs ~60-76）→ 修飾つきで `(` の無い語（`subscriber::with_default`・`subscriber::set_default`・`subscriber::set_global_default`、concat! で組む）を足し較正の対を 1 本（見張りの判定が厳しくなる・今の違反 0）
- G3 sample_path_guard_test.rs ~548 `all_four_shapes_of_a_bundled_balloon_path_are_detected` の four が偽 → 数を含まない名前に
- G4 一時パスの見張りが波括弧の中の別名（`use std::env::{temp_dir as x}`）を見ない（temp_path_guard_test.rs ~6・76-86）→ 語 `concat!("temp_", "dir as")` と較正 1 本（判定が厳しくなる・今の当たり 0）
- G5 doc_tree_guard_test.rs ~595「file_length_guard_test.rs と同じ形」が偽 → 括弧を消す
- G6 temp_path_guard_test.rs ~487・499 の何もしない `trim_end_matches('(')` → 消す
- G7 mod.rs ~367-368・temp_path_guard_test.rs ~81・474「修飾つきが裸を内側に含む」が偽 → 「後ろ半分と重なる」
- G8 temp_path_guard_test.rs の例外表の中の手書きの件数と履歴（~116-173・305・47-48・120-122）→ 消す（表と件数の定数は変えない）
- G9 doc_tree_guard_test.rs ~611-624 の 4 つの確かめのうち主検査と重なる 3 つ → `tree.all` の 1 つだけ残す
