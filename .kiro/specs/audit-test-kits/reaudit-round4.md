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
