# 検証の記録（areka-P0-emily-ghost-verification）

えみり（`emily4.nar`）を areka の実機で起こした検証の記録。各節は担当のタスクが埋める（同定・機械の構成・環境変数は 5.1、走行と崩れた所の表は 5.2〜6.2、数の洗い出しは 4.2）。

## 同定

## 機械の構成

## 環境変数

## 走行

## 崩れた所の表

## 数の洗い出し

タスク 4.2 で、ソースと文書の全体を `14 種`・`14 変種`・`fourteen`・`7 つ`・`7 本`・`seven` で検索した（2026-10-17・`git grep -n -I -E "14 種|14 変種|fourteen|7 つ|7 本|seven"`）。本 spec で変わった数は 2 つ——拒否の閉じた語彙（14 種 → 13 種）と、登記した検体の数（7 → 8）。外した場所を除いて 183 行が当たり、全部を下のどれかに片付けた（直した 2 行は直した後は当たらないので、183 行はどれも、この数とは別の数か、変わったことそのものを書いた行）。**片付いていない行は 0 件。**

### 検索から外した場所と理由

- `.kiro/specs/completed/` の下: 終わった spec の当時の記録で、書き直すと当時の事実が消える。上書きした完了 spec の要件は、書き直す代わりに `doc/COMPAT_ARCHITECTURE.md` §8 に番号つきで書いた（要件 6.5・8.2）。
- 本 spec のフォルダ（`.kiro/specs/areka-P0-emily-ghost-verification/`）: 「14 種 → 13 種」「7 → 8」という変化そのものを書いている文書なので、古い数が出るのが正しい。
- `target/`: ビルドの出力で、追跡していない（`git grep` は見ない）。
- `vendors/` の下: 当たりは 0 件だった（検体の README も含めて）。

### 直した行

- `doc/COMPAT_ARCHITECTURE.md` §8「インストールの失敗理由の語の写し」の行: 「拒否の 14 種」と「14 種を 1 つずつ組んで」を 13 種へ。`invalid type` の並びから `CompanionSourceMissing` を外した。
- `doc/ukadoc-coverage/ledger/shiori.toml` の `OnInstallFailure` の備考: 「areka-nar の拒否の 14 種」を 13 種へ。
- タスク 1.2・2・4.1 で直した分（今は当たらない）: 語彙のテスト・`error_tests.rs`・`judge_tests.rs` の 14 → 13、`procedure.rs` の口の数 7 → 8、`sample-ghost-kit` の登記のテストの 7 → 8。検索で、これらの古い数が残っていないことを確かめた。

### この数とは別の数（残した行と理由）

どれも、拒否の語彙の数でも、登記した検体の数でもない。

**ソース（`crates/`・`tools/`・`.claude/`）**

| 場所 | 数えているもの |
|---|---|
| `crates/areka-nar/src/error_tests.rs:372`（`manifest_warnings_are_seven_and_carry_their_key`） | 記録だけを残す警告（`ManifestWarning`）の種類 7。拒否の語彙とは別の型で、本 spec は警告の種類を足していない（飛ばした同梱は結果の別の欄に載せた）ので 7 のまま |
| `crates/areka/src/install/procedure_branch_tests.rs:336`・`crates/areka/src/install/fetch_url_tests.rs:144`（`..._seven_days`） | 日数 7 日（残った中身を消すまでの日数・古い落とし物を消す日数） |
| `crates/areka/src/update/refs_tests.rs:421`（`unsent_seven_words_never_appear`） | 更新の知らせで送らない語 7 つ |
| `crates/areka/src/log_history_tests.rs:139`（`..._fourteen_rows_in_order`） | 記録の履歴の規則の表の 14 行 |
| `crates/areka-parsers/src/balloon/parse_tests.rs:861`（`font_base_key_table_has_fourteen_entries...`） | バルーンの descript の書体の鍵 14 |
| `crates/sample-ghost-kit/src/devroot_sweep_tests.rs:501` | 同じ検体を取る 8 本のスレッドのうち、残りの 7 本（検体の数ではない） |
| `crates/areka/src/mcp/mcp_tests.rs:27・143・146` | MCP の道具のうち、相手を省略できる 7 本 |
| `crates/areka/src/mcp/get_property_tests.rs:6` | 聞くプロパティの名前 7 つ |
| `crates/areka/src/mcp/get_expression_table_tests.rs:343`（「検体 7」・上の 6 語では当たらず、検体の数の言い回しを広げて探したときに出た行） | 表情の表のテストの見本の 7 番目（登記した検体ではない） |
| `crates/areka/src/emo2_boot/spine_conformance_script.rs:618・694・714`・`crates/areka-kanade/src/schedule/events_tests.rs:350・443` | マウスのイベントの Reference 7 つ |
| `crates/areka/src/ghost_session_switch_fallback_tests.rs:420`（`Ref6/7`） | Reference の番号 6 と 7 |
| `crates/areka/src/emo2_boot/user_break_cue.rs:70` | 起動の受け口の 7 本目 |
| `crates/areka/src/emo2_boot/balloon_visibility_phase_reappear_tests.rs:10・267`・`crates/areka/src/emo2_boot/frame_balloon_offset_follow_tests.rs:42・47`・`crates/areka/src/placement/spawn_zorder_pair_deferred_tests.rs:9`・`crates/wintf/src/ecs/window/zorder_chain.rs:355` | そのファイルの中のテストの本数 |
| `crates/areka/src/placement/placement_prepare_tests.rs:343・344・487・491`（`seven_sixths`） | 拡大率 7/6 を入れた変数の名前 |
| `crates/areka-emo-atlas/src/animated_tests.rs:432` | 動く絵を縮める理由 7 つ |
| `crates/areka-emo-compose/src/nesting_report_tests.rs:80` | アニメーションの ID の語 7 つ |
| `crates/areka-emo-text/src/state_cluster_tests.rs:34` | 絵文字の部品の数 7 |
| `crates/areka-emo-text/tests/decoration_readback_test.rs:102`・`crates/areka-emo-text/tests/decoration_readback/mod.rs:78` | DirectWrite が範囲で持てる飾りの鍵 7 つ |
| `crates/areka-ghost/src/catalog_tests.rs:69` | ゴーストの素性の項目 7 つ |
| `crates/areka-mcp-bridge/src/service_tests.rs:188・189` | 要求の id に使った文字列 `"seven"` |
| `crates/areka-seriko/src/looper_film_count_tests.rs:66` | 16 ミリ秒の刻み 7 回 |
| `crates/wintf/src/ecs/window/command_batch_tests.rs:206`・`command_transition_tests.rs:140`・`zorder_chain_diag_tests.rs:101` | 窓の位置の引数 7 つ・窓を作る引数 7 つ・重なりの順の札 7 つ |
| `crates/ukadoc-survey/` の 17 行（`catalog/mod.rs:5`・`documents/derive_tests.rs:445`・`evidence/candidates.rs:512`・`evidence/resolve.rs:307`・`ledger/read_tests.rs:439`・`model.rs:238・278`・`model_tests.rs:127`・`report/tally_tests.rs:112・197`・`tests/consistency/briefing_arms.rs:717`・`checks.rs:765・771`・`linkage_checks.rs:6・23・508`・`spec_checks.rs:39`） | 網羅台帳の道具の数（状態の綴り 7 つ・検査の腕 7 つ・必須の欄 7 つ・正典の URL 7 つ・逃がし文字 7 つ・完了 spec の持ち主の宛先 14 種） |
| `tools/perf/judge-perf.py:260・1287・1431` | 「267 本」の一部（間隔の本数） |
| `.claude/agents/perf-analyze.md:39` | スケジュールの 7 本 |

**文書**

| 場所 | 数えているもの |
|---|---|
| `doc/COMPAT_ARCHITECTURE.md:287・290・300` | テストの名前の中の 7 日（`..._seven_days`）と、送らない語 7 つ（`unsent_seven_words_never_appear`） |
| `doc/COMPAT_ARCHITECTURE.md:333`（本 spec で足した行） | 「14 種 → 13 種」と、変わったことそのものを書いている |
| `README.md:95`・`doc/CONSTITUTION.md:76` | areka のエンジン 7 つ |
| `README.md:107`・`doc/release-notes/v0.0.2.md:21` | MCP の道具 10 本のうち動く 7 本 |
| `doc/impl-watch.md:27・43` | 机の決まり 7 つ |
| `doc/ssp-mcp/areka-tools.md:65` | 台本の診断の種類 7 つ |
| `doc/roadmap-overview-2026-10-16.html:70` | 10-16 時点のロードマップの写し（檻 7 本・Reference 7 つ・申し送り 7 本） |
| `doc/ukadoc-coverage/README.md:144` | 台帳の状態の綴り 7 つ |
| `doc/ukadoc-coverage/briefing-assets.md:381〜485`（「実例に使う 7 本のファイル」） | emo2 の系統の検体の中にある、正典の書式で書かれたファイル 7 本（検体の数ではない。同じ節が「検体 `R_POST_and_KOMAINU.nar` は数えていない」と断っている） |
| `doc/ukadoc-coverage/briefing-assets.md:814・816` | バルーンの書体の正典の見出し 14 種 |
| `doc/ukadoc-coverage/briefing-property.md:955` | 「7,007 本」の一部（テストの本数） |
| `doc/ukadoc-coverage/briefing-sakura-script.md:8` | 説明書の節 7 つ |
| `doc/ukadoc-coverage/briefing-shiori.md:59`・`doc/ukadoc-coverage/ledger/shiori.toml:30` | 優先度が空の群 7 つ |
| `doc/ukadoc-coverage/briefing.md:1725・1746・2435・2592`・`doc/ukadoc-coverage/roadmap-draft.md:1489・1494` | 「27 本」の一部（spec の本数） |
| `doc/ukadoc-coverage/ledger/property.toml` の 13 行（177・192・1496・1526・1559・1729・1764・2277・2292・2658・2673・2688・2703） | プロパティの名前のルートの枝 7 本 |
| `doc/ukadoc-coverage/linkage.md:153・2406` | 束の残り 7 本・束の欄 7 つ |
| `.kiro/steering/product.md:63`・`.kiro/steering/roadmap.md` の 13 行 | ウェーブ C8 の 7 本・spec の本数（107 本 など）・メニューの枠 7 つ・Reference 7 つなど |
| `.kiro/steering/roadmap-history.md` の 31 行 | 過去のウェーブの記録（その時点の spec・テスト・ファイルの本数）。拒否の語彙と検体の数を言う行は無い |
| ほかの spec の `brief.md` の 28 行（`check-script-arg-checks`・`coverage-roadmap-refresh`・`property-name-case-fold` ほか、19 本の spec） | その spec の数（診断の種類 7 つ・メニューの枠 7 つ・「バルーンのリンクと OS の連携」の 7 本・テストの本数など）。拒否の語彙と検体の数を言う行は無い |

### 配布物の許可表

`tools/package.ps1` の許可表は `ghost/` の下が `emo2` だけ、`balloon/` の下が `emo2-kakukaku` と `StayseeBalloon` だけで、`emily` の綴りは 0 件（大小を無視して数えた）。えみりは配布物に入らない（要件 2.7・変更 0）。
