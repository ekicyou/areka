# 6.1 かけ直し 5 回目の指摘（34 件・必ず直す 0）と仕分け

監査役は 6 人です。4 回目までの 5 人に加え、`examples/` を読む切れ端 E を足しました。4 回目までは `examples/` がどの切れ端にも入っていませんでした。

指摘は監査役の推奨の直し方で直します。外への約束が変わるもの（G1・G2・G5・L1・S1・E2・E3）は、audit 系の議題は推奨案で承認してよいという取り決め（11-04）に従い、進行役が推奨案を採りました。

「必ず直す」は 0 件です。ただし道具の振る舞いに関わる指摘が残っています（見張りの判定 G1・G2・G5、`capture_under_filter` の panic の L1、後片付けの行の S1・S5、見本の道具の E1〜E3）。そのため、この回では打ち切りの条件（要件 9.6）を満たしません。5 回目を直したあとで、6 回目を 1 回かけます。

連鎖（前の回の直しが呼んだもの）は、束ごと Fable に任せて矛盾を解かせます。束 B と束 D がそれに当たります。

## sample-ghost-kit（束 A）
- S1 devroot.rs ~529 `discard_older_cache_shelves` の `entries.flatten()` が、読めない項目を黙って捨てる → `reclaim_stale` と同じ `report_cleanup("list the namespace", ..)`
- S2 devroot.rs ~434-453 `cached_root` の説明が `cached_root_in` の役目を書いていて、偽が 2 つある（連鎖: 4 回目 S4）→ 役目と `# Errors` を `cached_root_in` へ移し、偽の 2 文を直す。`cached_root` にはテストの入口の 1 行を残す
- S3 devroot.rs ~896 の `[`sample_nar`]` → `[`sample_nar_for`]`（連鎖: 4 回目 S1）
- S4 devroot.rs ~180-183 `WorkDir` の説明が同じことを 2 度言う → 1 段落に畳む（連鎖: 4 回目 S6）
- S5 devroot.rs ~610-627 `discard_tree_with` の `gc-…lock` の札が、`AlreadyExists` のとき次の連番で取り直さない → `in_namespace_with` と同じに、`AlreadyExists` なら次の連番で取り直す
- S6 Cargo.toml:4 の description → `NarBuilder` と `WorkDir` を足す
- U1 tests/pasta_inner_balloon_test.rs ~57-74 の「外に置いた形」が偽（実際はゴーストのフォルダの中で、`balloon/` ではない所）→ 文を直す（連鎖: 1 回目 U3）
- U2 nar_writer_tests.rs ~309-320 `folding_a_missing_tree_fails_loudly` → 無いフォルダの名前を日本語にし、`is_ascii` と `\u{` を確かめる
- U3 lib_tests.rs ~684「（かけ直し 4 回目 S7）」→ spec の名を付ける
- U4 devroot_fetched_tests.rs:1 の `[`sample_nar`]` → `[`sample_nar_for`]`（連鎖: 4 回目 S1）
- U5 sha256_tests.rs ~3-5 の数（「試験値 4 つ」「3 通り」）→ 数を外し、境目の 2 本を書く
- U6 lib_tests.rs ~396「小さい方」→「リポジトリに置くゴーストのうち `.nar` が最も小さいもの」
- U7 lib_tests.rs ~401「表示と同じ綴り」→「Debug の綴り」
- U8 devroot_tests.rs ~434-443 の対照 → `os error 32` に締め、5 の枝に檻が無いことを `shortcut:` で書く（4 回目の「5 回目で見る」）
- U9 fetched_tests.rs ~305-306 中身の無い `is_ascii_only(what)` → `*what == "read the fetched checksum file"`
- E1 examples/fold-samples.rs ~292-296 `strip_folder` が、2 段以上の取り出し元と、大小の違う非 ASCII の名前を照らせない → 各段を `to_uppercase` で照らす。テスト 1 本（2 段の `balloon.source.directory` で不一致が 0）
- E2 fold-samples.rs ~145-150 追跡ファイルが 0 件のとき、日本語の panic で落ちる → ASCII の `Err`（`no tracked files under <path> (git add the unfolded tree first)`）。終了コードは 101 から 1 に
- E3 fold-samples.rs ~280 `installed_path` の panic → `Result` にし、ASCII の `unsupported kind`
- E4 fold-samples.rs ~135・fold-samples_tests.rs ~4 で `main` と `run` を取り違えている → `run` に

## real-run-kit（束 B・連鎖を含むので Fable）
- R1 tests/fake_wait_close_test.rs ~53-54・62-65 のコメントに既定の「60 秒」が写っている。1 回目 R5 の決めと字面でぶつかり、既定が 30 秒以下になると檻が黙って効かなくなる（連鎖: 4 回目 C2）→ 秒数を外し、限界を `shortcut:` で書く。4 回目の C2 の整理と 1 回目 R5 の両方と辻褄が合う形にする
- R2 launch.rs ~370・94-95 `compose_rust_log`・`rust_log` の見出しの説明が、振る舞いと表に合わない → 振る舞いに合わせて直す（挙動は変えない）
- R3 launch.rs ~4-5・197・227 の `"AREKA_MCP_PORT"` の手書き 2 か所 → `areka_mcp::PORT_ENV` を使い、説明を直す
- R4 i686_tests.rs ~489-498・527-531 の ASCII の確かめが、パスの部分では空振りする → 置き先のフォルダ名を日本語にし、`\u{` を確かめる
- R5 tests/fake_evidence_test.rs ~207-214・235 で、ASCII に逃がしたパスを本物のパスとして読む → `target_test_roots()` の中身から、`ascii` の写しが一致するものを選ぶ
- R6 mcp_tests.rs:1-4・output_tests.rs:1-2 の module doc の一覧の抜けと取り違え → 直す（連鎖: 3 回目 R3・4 回目 R2）
- R7 launch_tests.rs ~183 `sample_name_is_used_for_the_copy` の名前が偽 → `sample_name_is_given_only_for_sample_launches` などに改名

## log-capture-kit src（束 C）
- L1 filter.rs:94 `EnvFilter::new` が、綴りの誤った指令を黙って捨てる → `try_new` と panic。`#[should_panic]` を 1 本足し、wintf の `capture_under_filter` の呼び手のテストも回して確かめる
- L2 lib.rs:71「値の取り出しは 3 本」→ 数を外す（連鎖: 4 回目 L2）

## log-capture-kit の見張り（束 D・連鎖を含むので Fable）
- G1 with_default_guard_test.rs ~217-229 `declares_env_filter_feature` が 4 回目の直しで後退した。複数行の配列の `[features]` で `"log-capture-kit/env-filter"` と書いた形を見落とす（連鎖: 4 回目 G2）→ 行ごとの名指しの条件を戻して足す。較正の見本に陽性を 1 つ足す
- G2 同じ関数が、下位表の名前を変えて共有 crate を引く形（`[dev-dependencies.lck]` と `package = "log-capture-kit"`）を見ない → section 単位の印を持つ。見本を 1 つ足す。G1 と一緒に直す
- G3 doc_tree_guard_test.rs ~244-269 `check_all` が、読み取りが空のときも `StaleException` を並べる → `!tree.direct.is_empty()` を条件に足す
- G4 workspace_scan_test.rs:114「下の 2 本」→ 名前を書く
- G5 with_default_guard_test.rs ~313-319 の手書きの `FILES_THAT_MUST_NOT_SPELL_THE_TOKENS` が、見張りの一部しか見ない → `crates/log-capture-kit/tests/` の下から、例外表に載るものを除いて引く
- G6 workspace_scan/mod.rs ~150-156・625-633 で、コメントの除去を何度もやり直す → 除去済みの本文を `OnceLock` に持たせる（当たりは同じ。前後の時間を控える）
