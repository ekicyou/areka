# 6.1 かけ直し 6 回目の指摘（28 件・必ず直す 0・道具の振る舞い 0）と仕分け

監査役は 6 人（5 回目と同じ切れ端で、`examples/` の E を含む）。指摘ごとに [prod]（道具・見張り・exe の振る舞いが変わる）か [test/doc] かの印を付けさせた。

「必ず直す」は 0 件。[prod] の印が付いたのは E5 の 1 件だけで、これは 2 回目の決めを打ち消す往復の組なので当てない（下記）。当てる 27 件はどれも [test/doc] で、道具の振る舞いは変わらない。これで打ち切りの条件（要件 9.6: 「必ず直す」0・道具の振る舞い 0）を満たすので、6 回目の指摘を直したところでかけ直しを区切る。

連鎖（前の回の直しが呼んだもの）は束ごと Fable に任せ、矛盾を解かせる（束 B・束 D）。束 A と束 C は単なる追加と説明の直しなので、普通に直して閉じる。

## 当てないもの
- E5 fold-samples.rs ~118-124 `--from` を 2 度書くと先の値を捨てる → 当てない。2 回目の控え「S6 の副作用: `--from` を 2 度書くと後の値が勝つ（保守の道具なので受け入れた）」の決めを打ち消す往復の組になるため。2 回目の決めのままとする

## sample-ghost-kit（束 A）
- S1 devroot.rs ~23・27・31・37-38 の字面の `cache-2`（5 か所）→ [`CACHE`]（図の中は `<CACHE>/…`）
- S2 devroot.rs ~25 の木の図の `work\…` の役目に、確かめた書庫の写し（`<名>.nar`）を足す
- S3 devroot.rs ~706-707 `fresh_root` の「検体を得る唯一の入口は `SampleRoot`」→「使い捨ての複製を得る唯一の入口は `SampleRoot`（手で使う根は [`crate::manual_paths`]）」の類
- S4 lib.rs ~15-17「`[dev-dependencies]` からのみ引く」の「のみ」が、直後の例外と矛盾する（連鎖: 3 回目 S2）→「のみ」を外し、例外は real-run-kit だけと書く
- U1 パスの綴りが文に在ることを確かめていない 3 か所（nar_writer_tests.rs ~316-319・devroot_tests.rs ~368-371・lib_tests.rs ~727-730）。`\u{` があるだけだと、日本語の OS の文で満たされてしまう（連鎖: 4 回目 U1・U2、5 回目 U2。どれも途中で止まった直しの続き）→ 各テストに `ascii(<パス>)` の綴りが在ることの確かめを 1 行
- U2 tests/nar_sample_path_test.rs ~238-250 余分な引数が ASCII なので `ascii(&extra)` に檻が無い（連鎖: 3 回目 S5）→ 余分な引数を `"余分"` にし、`\u{4F59}\u{5206}` の行を待つ
- U3 fetched_tests.rs ~48 `ANY_SHA` の「どのバイト列のハッシュでもない」→「このファイルで使うどのバイト列のハッシュでもない」
- E1 fold-samples_tests.rs ~3-5 の module doc「`parse_args` の 1 本を除き」が偽（連鎖: 5 回目 E3）→ 数を外し、`parse_args` と `installed_path` を直に呼ぶものを除くと書く
- E2 fold-samples_tests.rs ~159-177 `no_tracked_files_is_an_ascii_error_not_a_panic` の名前と説明が偽（パスは `main` の出口で写す）（連鎖: 5 回目 E2）→ 名前と説明を「固定の英語の文の `Err`（パスは `main` の `ascii` が写す）」に
- E3 fold-samples.rs ~72-73・86-91 の「確かめは作業の置き場を消す前に」が、中身（`folder_name` と追跡ファイル 0 件の確かめは消した後）と合わない → 振る舞いは変えず、コメントを「引数の読みは消す前に」へ狭める
- E4 fold-samples.rs ~37-40 の module doc の写像 `<根>/ghost/<directory>/…` → `<根>/<格納先>/<directory>/…`（ゴーストは `ghost`、バルーンは `balloon`）

## real-run-kit（束 B・連鎖を含むので Fable）
- R1 Cargo.toml ~27 `areka-mcp` の使い手に、窓口の launch（`PORT_ENV` を引く）を足す（連鎖: 5 回目 R3）
- R2 close.rs ~6「時間はすべて引数で受ける（`Limits` に頼らない）」が偽（`stop_on_drop` は `&Limits` を受ける）→ 文を直す（コードは変えない）
- R3 run.rs ~19「今の番犬と同じ 60 秒」→「各待ちの始まりから数える締切。」（秒数は `Limits::default` が持つ）
- R4 launch.rs ~142 `spawn` の `# Panics` の「置き場は落ちた回として残る」が広すぎる → 置き場を作った後に落ちたときに絞る
- R5 mcp_tests.rs ~1-6 の module doc の試験の組分けが、3 回直しても中身と合わない（連鎖: 3 回目 R3・4 回目 R2・5 回目 R6。直すたびに次の指摘を呼ぶ往復）→ 組分けの一覧をやめて 1 行（「`mcp` の兄弟のテスト。偽の子との試験は `tests/fake_mcp_test.rs`」の類）にし、往復を止める
- R6 tests/common/mod.rs ~18-25 `alive(pid)` が番号だけで引く → 番号の使い回しで偽の赤が出得ることと、揺れたら起動時刻で照らす形に替えることを `shortcut:` で書く

## log-capture-kit src・temp-path-kit（束 C）
- L1 filter_tests.rs ~5-11・278-285 の module doc「3 点」に、5 回目 L1 で足した約束（解釈できない指令は panic）が入っていない（連鎖: 5 回目 L1）→ 数を外し、⒟ を足すか、別の見出しの下に移す
- L2 Cargo.toml:4 の description に、`capture_under_filter`（feature `env-filter`）と判定の補助（`expect_one`・`ExpectField`）を足す
- T1 lib_tests.rs ~430-453 の消せないフォルダの知らせのテストで、パスが ASCII なので `ascii(..)` に檻が無い → 日本語の子のフォルダを作り、その中のファイルを削除を拒む共有モードで開いてから `remove_or_report` を掛け、`\u{` とパスの綴りを確かめる
- T2 lib_tests.rs:1-3 の module doc がファイル全体を「要件 12.1」の自己テストと書く → 見出しを限り、spec `audit-test-kits` で足したテストを 1 行で書く

## log-capture-kit の見張り（束 D・連鎖を含むので Fable）
- G1 workspace_scan/mod.rs ~10-13・637-653 の「ファイルシステムに触るのは…だけ」が、5 回目 G6 の後は偽。`scan_sources` が `read_source` の写しを持つ（連鎖: 5 回目 G6）→ `Err` の枝を `read_source` に通して写しを消し、doc の一覧を合わせる
- G2 temp_path・sample_path の見張りの「逐語で書かない」の検査が自分の 1 ファイルしか見ない。with_default の見張り（5 回目 G5）とずれている（連鎖: 5 回目 G5）→ `files_that_must_not_spell_the_tokens` を除外の表を引数に取る形で mod.rs へ移し、3 つの見張りでそろえる（今の当たり 0）
- G3 sample_path_guard_test.rs ~57-58・907 のコメントを禁じる理由が偽（主検査はコメントを除く）→ 本当の理由（検索で数える母数が動く）に
- G4 with_default_guard_test.rs ~255-270 `kit_default_enables_env_filter` が別のフィーチャ経由の `env-filter` を見ない → 判定は変えず `shortcut:` を 1 行
- G5 with_default_guard_test.rs ~115-116「1 件」→ 数を外す
- G6 sample_path_guard_test.rs ~47-49・225-227「今日実体があるのは ⑶ だけ」→ 「どの形に実体があるかは [`EXCLUSION_FORMS`] が宣言する」の類
