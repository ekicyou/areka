# 6.1 かけ直し 4 回目の指摘（30 件・必ず直す 0）と仕分け

監査役の推奨の直し方で直す。外への約束が変わるもの（S2・G2）は、audit 系の議題は推奨案で承認してよいという取り決め（11-04）に従い進行役が推奨案を採った。道具の振る舞いに関わる指摘（S2・G2）が残っているので、5 回目の打ち切り（要件 9.6）の判断は 5 回目の結果で行う。

## sample-ghost-kit（束 A）
- S1 devroot.rs ~330-347 テスト専用の `sample_nar` に「唯一の式」の説明が残り偽・~350 のリンク切れ → 説明（`# Errors` を含む）を `sample_nar_for` へ移し、`sample_nar` には「テストが名前から引く入口」の 1 行。リンクを直す（`cargo doc -p sample-ghost-kit --no-deps --document-private-items` を RUSTDOCFLAGS=-D warnings で緑に）
- S2 devroot.rs ~306-310 `retire_lease` で、並走する掃除が先に札を消すと消去待ちの札に対する `remove_file` が os error 5 になり、偽の「cleanup failed … os error 5」の行が出る（3 回目の S1 の後は通ったテストでも出る・`tools/load-flake.ps1` の数を水増しする）→ `PermissionDenied` の後に `symlink_metadata(lock)` も失敗する（消去待ちか、もう無い）ときは黙って通す。テスト 1 本（`FILE_SHARE_DELETE` 付きで札をもう 1 つ開いて先に消し、消去待ちにしてから `retire_lease` が何も書かない）。同じ形の偽の行が `discard_tree_with` の `gc-…lock` の札でも出るなら同じに
- S3 bin/nar-sample-path.rs ~29-32「`tools/perf/` は人がその値を写す」が偽（`perf-tool-base.ps1` の `Get-NarSamplePath` が起こす・`--fetch-list` は `sample-fetch.ps1`）→ 「読み手（`tools/` の台本）は 0 以外なら止まり、要る鍵の行を読む」の類
- S4 devroot.rs ~668・685 の `[`cached_root`]`（`#[cfg(test)]`）→ `[`cached_root_in`]`
- S5 devroot.rs ~31・489-502 の説明「前の世代の棚」と中身（`CACHE` 以外の `cache…` を全部退ける）がずれる → 説明を「`CACHE` でない `cache…` の棚（新しい世代も含む）」に（挙動はそのまま）
- S6 devroot.rs ~177-178 `WorkDir` の「`areka-nar` のテスト」「唯一の窓」→ 「テストが空の根として借りる、札付きの作業フォルダ」の類
- S7 lib.rs ~421-422 `named_descript` が BOM の後ろの `name,` を見落とす → 1 行目だけ BOM を除いて照らし、置き換えるときは BOM を残す。テスト 1 本
- U1 lib_tests.rs ~720-742 `the_io_error_is_shown_in_ascii_only` に `\u{` の確かめを足す
- U2 devroot_tests.rs ~352-377 の後片付けの行のテストが ASCII を確かめない → パスを日本語にし `is_ascii` と `\u{` を確かめる
- U3 devroot_cache_tests.rs ~522-525・devroot.rs ~409-412「この経路だけ」「唯一」→ 言い直す
- U4 devroot_sweep_tests.rs ~415-416「決定論で」→ 外し「窓に当たるかは割り当てしだい」
- U5 nar_writer_tests.rs ~182-185 `Corrupt::Size` の段で `comp_size` を確かめる
- U6 nar_writer.rs ~334-343 の名前が UTF-8 でない枝に檻が無い → 対になっていないサロゲート（`OsStringExt::from_wide(&[0xD800])`）の名前のファイルで `InvalidData` と ASCII の文を確かめるテスト（NTFS に置けなければ理由を控えて足さない）
- U7 lib_tests.rs ~126-159 の名前の一覧の等値が `SAMPLES` の第 2 の手書きの表 → 一覧の等値をやめる（`.nar` が在る行は Vendored の対応は `only_vendored_rows_have_a_nar_in_the_repository` が持つ）。名前を中身に合わせる
- U8 lib_tests.rs ~2-16 の module doc の一覧の抜けと「1 本」の誤り → 直す

## real-run-kit（束 B）
- R1 Cargo.toml ~27 `areka-mcp` の説明「偽の子だけが使う」が偽 → `mcp_tests` の同じプロセスの受け口も使うと書く
- R2 mcp_tests.rs ~2「4 本」→ 数を外す
- R3 mcp_tests.rs ~206「同じ過程に」→ 「同じプロセスに」
- R4 launch_tests.rs ~67・72 の代入をなぞる `deadline` の確かめ → 消す

## log-capture-kit src・temp-path-kit（束 C）
- L1 event.rs ~98-101・event_tests.rs ~285-286 の `field_str` を置く理由が 3 crate について偽 → 本当の理由（生値の完全一致で判定する消費側のため）に
- L2 lib.rs ~178-189「そのほかの窓口」に `expect_one`・`ExpectField::expect_field` を足す
- T1 temp-path-kit lib.rs ~35-36「テストが落ちて残ったフォルダ」が振る舞いと違う → 強制終了と消せなかったときだけ残る、と

## log-capture-kit の見張り（束 D）
- G1 temp_path_guard_test.rs ~85・496「実在: `make_unique_temp_dir`」が偽（消えた）→ 実在の主張から外す（見本の文字列は残す）
- G2 with_default_guard_test.rs ~222-228 `declares_env_filter_feature` が複数行の配列で書いたインライン表の宣言を見落とす → crate 名を含む鍵の行から次の鍵の行までを値として読む（`kit_default_enables_env_filter` と同じ形）。較正の見本を `the_env_filter_feature_is_detected_only_when_it_is_actually_declared` に（判定が厳しくなる・今の当たり 0）
- G3 workspace_scan/mod.rs ~379-391 `scan_tokens` が検体の見張りで遅い → 頭の文字ごとの組を 1 度だけ作り二分探索で引く（当たりは同じ・前後の時間を控える）
- G4 「例外表にあるのに当たりが無い」判定が 2 か所（with_default `stale_entries`・temp_path の手書き）→ `AllowEntry` で受ける総称の形にして mod.rs へ
- G5 3 つの見張りの「走査語を逐語で書かない」の違反の行を手で組む → `Hit::new` と `render` で
- G6 sample_path_guard_test.rs で語の一覧を 3 回組む → `all_tokens()` を 1 つ
- G7 sample_path_guard_test.rs ~840-847 `each_gateway_user_is_actually_pulled_in_as_a_dev_dependency` の後半の手書きの名指し → 消す
- G8 doc_tree_guard_test.rs ~194 の手書きの `doc/README.md` → `{GUIDE}`

## 束 B の直しの控え（承認済み）
- 文と代入をなぞる確かめの削除だけ（較正なし・名前の付いたものの削除なし）
- 5 回目で見る: R4 で `.deadline()` の値を直に見る確かめが消えたので、締切の設定が黙って無視されても `hang_hits_deadline_and_child_is_stopped` は既定の 60 秒で終わるだけで赤にならない（経過時間を見ていない）

## 束 C の直しの控え（承認済み）
- 説明の直しだけ（較正なし・削除なし）

## 束 D の直しの控え（承認済み・G3 は連鎖 1 回目 G1 → 4 回目 G3 として Fable が回をまたいで確かめた）
- 較正: G2 は複数行の配列で書いた宣言を陽性、次の鍵で値が終わる形を陰性として `the_env_filter_feature_is_detected_only_when_it_is_actually_declared` に入れた。今の木での当たり 0
- G3: 頭の文字ごとの組を 1 度だけ作る形に。新旧の当たりの一致を一時テストで確かめた（430,356 件が同じ・一時テストは消した）。`sample_path_guard_test` の時間は 18〜24 秒から 6.6〜9.4 秒へ。1 回目 G7・2 回目 G8・3 回目 G2・G3 の決めは残っている
- G2 の限界は `shortcut:` のコメントで書いた（値は次に `=` を含む行で終わる）
- 消したもの: G7 の後半の手書きの名指しの確かめ（名前の付いたテストの削除・改名は無し）

## 連鎖 C2 の直しの控え
- 連鎖: 4 回目 R4（`launch_tests.rs` の代入をなぞる `deadline` の確かめを消した）→ 締切の設定が黙って無視されても赤になるテストが無くなった（束 B の控えの「5 回目で見る」）
- 直し: `crates/real-run-kit/tests/fake_wait_close_test.rs` の `expect_panic` が、渡された処理にかかった時間を測り、30 秒未満であることを確かめる。呼び手の 3 本（`hang_hits_deadline_and_child_is_stopped`＝`wait_for`、`deaf_window_is_stopped_after_deadline`＝`close`、`wait_exit_stops_hanging_child_after_deadline`＝`wait_exit`）はどれも `.deadline()` で 1〜2 秒を渡すので、指定が待ちに届かなければ既定の 60 秒まで待って赤になる。変えたのはこのファイルだけ（本体のコードは変えていない・依存の追加なし・テストの追加と改名と削除なし）
- 上限の余裕: 緑のときは 3 本とも 1〜4 秒で終わる。指定が効かないときは 60 秒以上。上限の 30 秒はどちらからも離れている（負荷で揺れない）
- 較正: `launch.rs` の `deadline()` の代入 `self.limits.deadline = limit;` を `let _ = limit;` に替えた（設定を黙って捨てる形）→ `cargo test -p real-run-kit --test fake_wait_close_test -- deadline` で 3 本とも赤（「短い締切の指定が効かず 61.2s かかった」が 2 本、`hang_hits_deadline_and_child_is_stopped` は待ちと破棄の 2 回で 121.3s）。戻して 14 本が緑（3.1 秒）。`launch.rs` に差分が残っていないことを `git status` で確かめた
- これまでの決めとの照らし合わせ: 4 回目 R4 の決め（代入をなぞる確かめは置かない）は残っている。足したのは `Limits` の値を読み返す確かめではなく、偽の子を起こして待ちが実際に短い締切で終わることを時間で見る確かめ。1 回目 R5（猶予の時間は `mcp_tests.rs` で型つきの `Limits` を渡して確かめる・結合テストに既定の秒数を手で写さない）とも矛盾しない: 30 秒は既定の 60 秒を写した数ではなく、渡した 1〜2 秒と既定の間に置いた上限で、既定が 30 秒以下に変わったときだけ見直しが要る。2 回目・3 回目の控えに締切を扱う決めは無い
- 確かめ: `cargo test -p real-run-kit` 緑・`rustfmt --check`（変えたファイル）差分なし・`cargo clippy -p real-run-kit --all-targets -- -D warnings` 警告なし・`cargo test -p log-capture-kit --test file_length_guard_test` 緑

## 束 A の直しの控え（承認済み・S2 は連鎖 3 回目 S1 → 4 回目 S2・U2 として Fable が回をまたいで確かめた）
- 較正: S2 は消去待ちの見分けの条件を殺す → `a_lease_already_pending_deletion_by_a_sweeper_writes_no_cleanup_line` が赤（査読でも取り直した）。S7 は BOM の枝を殺す → `named_descript_replaces_the_name_line_or_appends_one` が赤。U6 は `InvalidData` を `Other` に → `folding_a_name_that_is_not_utf8_fails_with_invalid_data_in_ascii` が赤
- S2 は仕分けの案（`symlink_metadata` も失敗するとき黙る）と違う形にした: この PC では消去待ちの札でも `symlink_metadata` が通る（テストの中で実測している）ので見分けに使えない。代わりに「`remove_file` が `PermissionDenied`、かつ権限 0・全共有（share 7）でも開けない」ときだけ黙る。`retire_lease` を `retire_lease_to(out, lease, lock)` に分け、`discard_tree_with` の `gc-…lock` も同じ道を通る
- S2 のテストは消去待ちを `FILE_FLAG_DELETE_ON_CLOSE` の手綱で作る（`FILE_SHARE_DELETE` で開いて `remove_file` する形は、今の Windows が名前をその場で外すので消去待ちにならない）。対照は握ったままの `WorkDir` の札で、行を書く
- S2 の効き: `cargo test -p sample-ghost-kit` の stderr の `cleanup failed` の行が 30 → 2。残る 2 行は意図した本物の失敗（`move aside the leftover tree`・`move aside the old master copy`・os error 5）
- 連鎖の結論: 矛盾なし。temp-path-kit・real-run-kit の i686・sample-ghost-kit の 3 つとも、書き先を `&mut dyn Write` で受け、本番は `std::io::stderr()` へ直に、ASCII で、`NotFound` は黙って通す。頭の `sample-ghost-kit:` と `os error N`（`tools/load-flake.ps1` が字面で読む）は `write_cleanup` の 1 か所のまま。握られた札は os error 32 で `PermissionDenied` の枝に入らず行を書く（実測）。黙るのは消去待ちか、札のパスがフォルダのときだけ（後者は本番では届かない）。`areka-P0-ghost-session-test-load-flake` 要件 5.2 の向きとも合う
- 改名: `the_rows_with_a_nar_in_the_repository_are_vendored_and_the_rest_are_url_rows` → `the_only_url_row_is_pasta_in_windows`（U7。古い名前の残りは reaudit-round3.md の控えの 1 件だけ）。消したもの: その中の Vendored の名前の手書きの一覧の等値。名前の付いたテストの削除は無し
- 足したテスト: `a_lease_already_pending_deletion_by_a_sweeper_writes_no_cleanup_line`（S2）・`folding_a_name_that_is_not_utf8_fails_with_invalid_data_in_ascii`（U6）
- 5 回目で見る: 「os error 5 だが権限 0 で開ける札は書く」の枝に檻が無い（作るには ACL の操作が要る）。対照の確かめは `os error ` までで数（32）を固定していない
