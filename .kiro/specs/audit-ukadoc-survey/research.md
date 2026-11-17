# ギャップ分析: audit-ukadoc-survey

> 2026-11-17 `kiro-validate-gap`（`/kiro-start` の中のサブエージェント）。範囲 `crates/ukadoc-survey/` 全部（96 ファイル）だけに `/ponytail:ponytail-audit` の作法をかけた。作法のうち 2 つは brief のとおり外した＝「最大 20 件」を外して全件を出す・「読まなかった所（Not checked）」を残さない（96 ファイルを 7 つの群に分け、読み係 7 本が群ごとに頭から終わりまで読んだ＝12 節）。読み係の出した指摘は、まとめ役が重なりを 1 件に畳み（例: 実データの読み直しは 3 つの群が同じものを出した）、主な指摘をコードで確かめ直してから番号を振った。番号は audit の通し番号で、以後の段はこの番号で指す。形は完了した `audit-tools-scripts` の research.md に合わせた。
> 回さなかったもの: `cargo test`・`cargo build`・副手続きの実行。測りは全部コードとデータを読んで行った（回したのは `git`・`wc`・`grep`・`find` の読むだけのコマンドだけ）。速さの数（5）は読み込みの量と呼ぶ回数からの見積もりで、時間は測っていない。

## 1. 要約

- 範囲は ukadoc の網羅の調べの道具（本番のコード 43 ファイル 9,099 行・テスト 52 ファイル 24,988 行・`Cargo.toml` 25 行）。指摘は **79 件**＝必ず直す 4・直したほうがよい 64・できれば 11。テストのコードへの指摘が多い（何も縛らない檻・写しの helper・古い注記）。
- いちばん重いのは **1**＝整合テストが「`.kiro/specs/` の直下に brief.md の無いフォルダが 0 件」を合否に入れている。CLAUDE.md の書く `/kiro-spec-init`・`/kiro-spec-quick`（discovery を通らない道）で spec を作ると、そのワークツリーの `cargo test -p ukadoc-survey`（全体テストの一部）が赤になる。`kiro-next` の「新しい spec のフォルダを作っても赤にならない」前提とぶつかる。次に **2**＝台帳の書き出しがその場の上書きで、途中で止まると手で埋めた台帳が切れたまま残る。**3**＝`candidates` の出力に今もテストの本文（`*_tests/` の下・`tests.rs`）の見本が並ぶ。**4**＝`check` 副手続きは全体報告 `summary.md` の古さを見ないのに、`cargo test` はそれで赤になる（手で回す検査と常時テストの合否が割れる）。
- 速さ・負荷では **5**＝整合テストが 1 本ごとに実データを丸ごと読み直す（`RepoData::load()` が 65 か所・1 回あたり `crates/**/*.rs` 2,557 本・約 33 MB を読む＝1 回の `cargo test` で約 2 GB。並ぶ 8 本のセッションが同じ机で回す）。
- 外への約束（要件 1.6）を変える直しは 28 件。そのうち 22 件は「今のデータでは結果が同じで、形の崩れた入力でだけ落ち方・出方が変わる」もの、3 件は偽の赤を消すもの（1・10・11）、今のデータで出方が変わるのは 3 件（**3** の `candidates` の標準出力・**4** の `check` の終了コードと標準出力・**22** のパイプが閉じたときの終了コード）。今の `doc/ukadoc-coverage/` の入力から副手続きが**書き出すファイルの中身が変わる直しは 0 件**（要件 6.4）。
- 直さずに残す候補（要件 2.2）は 8 件＝⑴ 開発者の裁定・完了 spec の要件／設計に触れうる 6 件（1・4・20・50・53・59）、⑵ わざと置いた印 3 件（1・54・79 の一部。1 は ⑴ と両方）、⑶ ukadoc の正典の読み方に直接触れるものは 0 件の見立て（15・20 は ukadoc で確かめる＝11 節）。C22 の約束（`assets.toml`・`file_length_guard_test.rs`・`Cargo.lock`）とぶつかる直しは 0 件。規模は L・リスクは中（9 節）。

## 2. 測り直し（要件 1.2）

- 取り込んだ `main`: `5749a4c7`（このブランチの `HEAD~1`・`origin/main` と同じ）。範囲の差分は無い（`git diff --stat origin/main HEAD -- crates/ukadoc-survey` が 0 行）。
- 範囲（brief と同じ数え方＝`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）: **96 ファイル＝コード 43 ファイル 9,099 行・テスト 52 ファイル 24,988 行・`Cargo.toml` 1 ファイル 25 行**。起票のとき（`e3c2cbe4`）・要件の段の測り直しと同じ数。

| 群（読み係） | ファイル | 本番の行 | テストの行 |
|---|---|---:|---:|
| A 入口・入出力・共通 | `Cargo.toml`・`lib.rs`・`main.rs`・`error.rs`・`hash.rs`・`tomlout.rs`・`lib_test_support.rs`・`cli/`（6）・`io/`（9） の 25 本 | 1,811 | 4,124（＋Cargo 25） |
| B 型・割り当て・カタログ・3 文書 | `model`・`assignment`・`catalog/`（7）・`documents/`（6） の 17 本 | 3,108 | 3,636 |
| C 整合の判定・差分 | `check/`（11）・`diff` の 13 本 | 1,196 | 4,230 |
| D 台帳・証拠 | `ledger/`（9）・`evidence/`（7） の 16 本 | 2,282 | 3,380 |
| R 報告・テストの入口 | `report/`（9）・`tests/cli_streams.rs`・`tests/consistency.rs`・`consistency/{mod,perturb,examples}.rs` の 14 本 | 881 | 4,143 |
| E 整合テスト ⑴ | `consistency/{checks,linkage_checks,non_vacuity,spec_checks,values_md}.rs` の 5 本 | — | 3,563 |
| F 整合テスト ⑵ | `consistency/{briefing_arms,briefing_checks,documents,documents_checks,documents_non_vacuity,documents_tools}.rs` の 6 本 | — | 3,612 |

- **1,000 行の番人に近いファイル**（900 行以上）: `src/cli/generate_tests.rs` **948 行**・`tests/consistency/linkage_checks.rs` **944 行**。次が `tests/consistency/checks.rs` 893・`src/check/finding_tests.rs` 850・`src/check/content_link_tests.rs` 808 行。直しでテストを足すときは、先に **34**（`generate_tests.rs` を仕事で分ける）を済ませるか、足す先を別のファイルにする。本番の最大は `src/documents/parse.rs` 755 行。
- 物差しの版: brief は ponytail 5.3.2 と書くが、今入っているのは **5.3.3**（`~/.claude/plugins/installed_plugins.json`）。作法（見る順・3 つの組・1 件 4 部・`shortcut:` の扱い）は 5.3.3 の `skills/ponytail-audit/SKILL.md` に従った（先例の `audit-tools-scripts` と同じ）。
- 範囲の中の `shortcut:`・`ponytail:`・`TODO`・`FIXME`・`#[allow(...)]`: **0 件**。`#[expect(dead_code)]` が 1 件（`tests/consistency/checks.rs:231`＝**54**）。

## 3. この道具が何をするか・想定した負荷

**何をするか**: 開発の道具（leaf の bin＋lib・`publish = false`・areka の実行時のコードからは 1 行も参照されない）。ukadoc のスナップショット JSON（`AREKA_UKADOC_SNAPSHOT`・既定は `%APPDATA%` の下）から正典の写し `doc/ukadoc-coverage/catalog.toml` を作り、areka の判定の台帳 4 本（`ledger/{assets,property,sakura-script,shiori}.toml`）と報告（`report/*.md`）を書き出す。整合は常時テスト（`cargo test -p ukadoc-survey`）が守り、そのテストは `.kiro/specs/` の直下・completed の spec 文書と、`doc/ukadoc-coverage/` の 3 文書（`linkage.md`・`briefing.md`・`roadmap-draft.md`）と `values.md` も読む。中身は純粋層（文字列と値だけ）と入出力層 `io` の 2 層。

**想定した負荷**: 開発者 1 人の Windows の机。並んで走るセッションが最大 8 本、それぞれ自分のワークツリーで全体テスト（その中の `cargo test -p ukadoc-survey`）を回す。副手続きは手で時々回す（`ledger-init`・`priority-apply` は手で埋めた台帳を書き替える）。入力はカタログ 1,749 項目・台帳 4 本（226 KB〜1.18 MB・shiori は約 16,500 行）・走査の相手 `crates/**/*.rs` 2,557 本・約 33 MB。

## 4. 外への約束と範囲の外の読み手（いま確かめた事実）

| 外への約束（要件 1.6） | 読み手 | 確かめた所 |
|---|---|---|
| 副手続き 9 つ（引数なし）・終了コード 0／1／2・結果は標準出力・断りと失敗は標準エラー | 手引き `doc/ukadoc-coverage/README.md`・steering `structure.md:477-484`・`workflow.md` | `src/main.rs`・`src/cli/mod.rs`・README |
| 環境変数 `AREKA_UKADOC_SNAPSHOT` | README・`cli/mod.rs` の使い方 | `src/io/snapshot.rs:90-100` |
| `doc/ukadoc-coverage/` の下の書き出す中身と形式（台帳の形式は完了 `areka-P0-ukadoc-survey-toolkit` の要件 2.1・2.2・2.6・付録 A で凍結） | 4 本の調査 spec の後の更新・`ukadoc-catalog-refresh`・`doc-compat-architecture-rename` | `src/tomlout.rs`・`src/ledger/write.rs`・`src/catalog/write.rs`・`src/report/` |
| 整合テストが落ちる条件 | スキル `kiro-next`（Step 8 でこのテストを回す・`SKILL.md:114`「New spec folders are safe」）・`kiro-complete`（`SKILL.md:243`・`:457`・`:651`＝テストの最中に `.kiro/specs/` の直下を増減しない） | 読んだ |
| `candidates` の説明の中のパス（`crates/areka/src/input_events/balloon.rs` ほか） | `input-crate-split`（C23 の見込み）が動かすときに直す | `src/evidence/candidates.rs:26-51` |
| 所見の種類の一覧（15 種） | README `:574` の表 | `src/check/finding.rs` |

## 5. 指摘の一覧（79 件）

旗の読み方:
- **外への約束**: 直すと 4 節の約束が変わるか。「変える」＝何が変わるかを書く。「今のデータは同じ」＝形が崩れた入力でだけ変わり、今の `doc/ukadoc-coverage/` の入力では出方が 1 バイトも変わらない。「守る」＝形は変えないが直し方に気を付ける点がある。「なし」。
- **範囲の外へ**: 直しが `crates/ukadoc-survey/` の外のファイルに及ぶか。
- **残す候補**: 要件 2.2 の ⑴〜⑶ に当たりうるか（候補を挙げるだけで、決めない）。
- **C22**: C22 の約束（`doc/ukadoc-coverage/ledger/assets.toml`・`crates/log-capture-kit/tests/file_length_guard_test.rs`・`Cargo.lock`・1,000 行）とぶつかるか。書いていなければ「ぶつからない」。
- 見出しの末尾の（仮 X）は読み係の仮番号（12 節）。

### 必ず直す（Must fix）

**1. spec のフォルダの一覧のテストが、brief の無いフォルダとテスト中の増減で赤になる**（`tests/consistency/documents_non_vacuity.rs:330-368`）（仮 F1）
- **何か**: `Documents::load` が `.kiro/specs` を読んだ後、テストがもう 1 度 `read_dir` して組み直し、2 つが一致すること、そして直下に brief.md の無いフォルダが 0 件であることを確かめる。
- **困る場面**: ⑴ `/kiro-spec-init "説明"`・`/kiro-spec-quick`（CLAUDE.md の書く discovery を通らない道）で作った `.kiro/specs/<名前>/` には brief.md が無い。そのワークツリーの全体テストが「brief.md を持たない spec ディレクトリが直下に現れた」（:360-364）で赤になる。完了 `areka-P0-ukadoc-coverage-roadmap` の要件 11.1 ⑸「他 spec の起票で赤にならず」と、`kiro-next` の `SKILL.md:114` の前提に反する。⑵ 2 回の読み（`documents.rs:68-73` と :335）の間にフォルダが増えると「一覧が…食い違う」（:350-353）。`kiro-complete` はこれを起きた症状として書き、避ける決まりを置いている（`SKILL.md:243`）。
- **直し**: 組み直しと brief 無し 0 件と completed の brief の確かめ（:330-368・約 39 行）を消す。下限 2 つと `live ^ completed` は残す。
- **飛ばすと**: discovery を通らない起票をしたワークツリーで、関係の無い全体テストが赤になり続ける。
- **外への約束**: 変える（整合テストが落ちる条件が緩む＝偽の赤が 2 つ消える）。
- **範囲の外へ**: 直しは範囲の中だけ。直した後は `kiro-complete` の `SKILL.md:243`・`:457`・`:651` の注意が要らなくなる（残しても害は無い・直すかは議題）。
- **残す候補**: ⑵ に当たりうる（「反例が現れたら赤になるので数え直すこと」とわざと置いた写真の主張）。ただし要件 11.1 ⑸ とぶつかる。⑴ の観点では、「0 も書く」（要件 11.5）の書き方を外すことになる。
- **確かめた**: 今の直下で brief.md が無いのは `completed/` だけ。根は `CARGO_MANIFEST_DIR` から決まる（`io/paths.rs:39`）ので、揺れるのは同じワークツリーの中の操作だけ。

**2. 台帳の書き出しがその場の上書きで、途中で止まると手で埋めた台帳が切れたまま残る**（`src/io/files.rs:51-53`）（仮 A1）
- **何か**: `write_lf` は `std::fs::write`＝先にファイルを空にしてから書く。`catalog`・`ledger-init`・`priority-apply`・`report`・`report-summary` の書き出しは全部ここを通る（`cli/generate.rs:64・138・180・201・215`）。
- **困る場面**: `priority-apply` が `ledger/shiori.toml`（1,180,258 バイト・手で埋めた 677 項目）を書いている最中に、ディスクがいっぱいになるか Ctrl+C で止まると、台帳が途中で切れて残る。失敗は返るが、コミットしていない手書きの分はもう戻せない。
- **直し**: 隣に一時ファイル（例 `<名前>.tmp`）を書いて `std::fs::rename` で置き換える（3〜4 行・Windows の `rename` は既存を置き換える）。一時ファイルの名前を返す小さな関数ならテストを 1 本。
- **飛ばすと**: 起きる見込みは低いが、起きたら手書きの台帳がコミットしていない分だけ消える。
- **外への約束**: 守る（書き出すバイト列は同じ。止まったときだけ `*.tmp` が残りうる）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。
- **確かめた**: `files.rs:52`。呼び手 5 か所はどれも `write_lf` を直に呼ぶ。

**3. `candidates` の出力に、テストの本文（`*_tests/` の下・`tests.rs`）の見本が並ぶ**（`src/evidence/candidates.rs:112-117`）（仮 D1）
- **何か**: `is_test_path` はパスだけでテストを除く。見るのは `_tests.rs`・`_test.rs`・`/tests/`・`tests/` の 4 形だけ。
- **困る場面**: `crates/areka-runtime/src/placement_tests/transition_judge_offset.rs:82` の `STABLE_VERDICTS` や `crates/areka/src/placement_tests/zorder_branch_coverage_test_support.rs:94-105` の見本が、今の `candidates` の標準出力に候補として並ぶ。冒頭の説明（:34-45）の「テストの本文は拾わない」が守れていない。除かれていないテスト名のファイルは `runtime_tests/`・`placement_tests/`・`.../tests.rs` など 122 本。
- **直し**: 条件を 2 つ足す（`|| path.contains("_tests/") || path.ends_with("/tests.rs")`）。候補のテストにパスを 1 本足す。
- **飛ばすと**: 調べる人がテストの見本を、正典の URL を置く場所と取り違える。
- **外への約束**: 変える（**今のデータで** `candidates` の標準出力からテストのファイルの行が減る。整合テストは候補を見ないので落ちる条件は同じ・書き出すファイルは無い）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。
- **確かめた**: `candidates.rs:112-117` を読んだ。`git ls-files` で 122 本。

**4. `check` 副手続きは全体報告 `summary.md` の古さを見ないのに、`cargo test` はそれで赤になる**（`src/check/freshness.rs:19-26`・`:63-81`、相手は `tests/consistency/documents_checks.rs:335-360`・`:420-426`）（仮 C1）
- **何か**: freshness は報告と台帳の一致をドメイン別の報告 4 本だけで見る（「全体報告は見ない（要件 7.6）」・開発者裁定 2026-09-02 議題 2）。ところが統合調査の後、整合テストの `summary_findings` が同じ「作り直した本文と一致するか」を `summary.md` にも当てている。
- **困る場面**: 台帳を直して `report` だけ走らせ、`report-summary` を忘れる。`cargo run -p ukadoc-survey -- check` は所見 0 件・終了コード 0（`cli/inspect.rs:64-87` は `check::run` だけを呼ぶ）、`cargo test -p ukadoc-survey` は `the_summary_report_is_as_fresh_as_the_ledgers` で赤。README `:52` の「同じ検査は標準のテスト実行にも入っている」と食い違う。書き方も割れている（テスト側は行番号を出し、freshness は行番号を出さない＝要件 6.11 の考え）。
- **直し**: `CheckInput` に全体報告の本文の欄を 1 つ足し、freshness で `strip_cr(保存側) == render_summary(...)` を比べて `report/summary.md` を場所にした `DomainReportStale` を出す。cli の `check` と整合テストはその本文を読んで渡し、`summary_findings` は消して `check::run` に任せる。freshness の注記（:19-26）と freshness_tests（:347-348・:373-376）を書き直す。差し引きで減る。
- **飛ばすと**: 手で回す検査の緑と、常時テストの赤が食い違ったまま。
- **外への約束**: 変える（`check` の終了コードと標準出力＝全体報告が古いと 1 になり所見が出る。整合テストの落ちる条件は同じで、失敗の本文の形だけが変わる＝行番号が消える）。
- **範囲の外へ**: なし（README は直しで記述どおりになる）。
- **残す候補**: ⑴ に当たりうる（完了 toolkit の要件 7.6・開発者裁定 2026-09-02 議題 2・`tests/consistency/mod.rs:73-75`「設計が固定した入力に足さない」。ただし 7.6 の除外は統合調査の要件 11.2 で覆っている＝README `:449-454`・`tests/consistency/mod.rs:20-28`）。
- **確かめた**: `freshness.rs:19-26` と `documents_checks.rs:335-360` を読んだ。

### 直したほうがよい（Should fix）

#### 負荷・データの書き込み

**5. 整合テストが 1 本ごとに実データを丸ごと読み直す**（`tests/consistency/mod.rs:136-175`・`tests/consistency/documents.rs:55`、呼び手は `tests/consistency/` の 9 ファイル）（仮 E1・R1・F3）
- **何か**: `RepoData::load()` は呼ばれるたびにカタログと台帳 4 本を解析し、`crates/**/*.rs` を全部読み、証拠を取り出して解決し直す。`Documents::load()` も毎回 3 文書と `.kiro/specs` を読み直す。
- **困る場面**: `RepoData::load()` の呼び出しは 65 か所（checks 18・documents_checks 9・linkage_checks 9・briefing_checks 8・values_md 8・examples 5・non_vacuity 5・documents_non_vacuity 2・spec_checks 1）、`Documents::load()` は約 45 か所。走査の相手は 2,557 本・約 33 MB なので、1 回の `cargo test -p ukadoc-survey` で約 2 GB を読んで正規表現にかける。テストは並行に走るので、33 MB の写しがスレッドの数だけ同時にメモリに載る。並ぶ 8 本のセッションが同じ机で回す。どの呼び手も `RepoData` を書き換えない（写しを壊すテストは `.clone()` してから壊す）。
- **直し**: `mod.rs` に `static REPO: LazyLock<RepoData>` を置き、`load()` は `&'static RepoData` を返す（今の本体は `read` へ改名）。`Documents::load` も同じ形。`spec_checks.rs:340` の `repo: RepoData` と `examples.rs` の `RepoData::load().catalog` 3 か所を参照に直す。足すのは 10 行ほど・標準ライブラリだけ。
- **飛ばすと**: 並ぶセッションが増えるほど、全体テストの中でこのクレートだけが机の読み込みと CPU を食い続ける。
- **外への約束**: 守る（合否の条件は同じ）。気を付ける点: 読み込みが panic すると後の呼び出しは「poisoned」で落ち、本当の理由は最初の 1 本にだけ出る（失敗の本文を `LazyLock` の中で絶対パスつきで出す形を保つ）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。
- **確かめた**: `grep -c "RepoData::load()"` の数え上げ（65）。`io/sources.rs:49-61` は呼ばれるたびに全ファイルを `read_normalized`。`RepoData` の欄に `Rc`・`RefCell`・`Cell` は 0 件（`Sync` で持てる見込み＝コンパイルでは未確認）。

**6. `report` が 1 本ずつ「読む→書く」で、3 本目の台帳が壊れていると前の 2 本だけ書き換わって止まる**（`src/cli/generate.rs:174-184`）（仮 A2）
- **何か**: ドメイン別の報告 4 本を、ドメインごとに台帳を読み、描いて、すぐ書き出す。`ledger_init_with`・`priority_apply_with`・`report_summary` は全部決めてから書くのに、ここだけ形が違う。
- **困る場面**: `ledger/sakura-script.toml` の並びが崩れた状態で `report` を打つと、`shiori.md`・`assets.md` は作り直され、残りは古いまま、終了コード 1。新旧の混ざった報告が残る。
- **直し**: 先に 4 本の台帳を全部読み（**36** の `load_ledgers()` を使う）、4 本の本文を描いてから書き出しのループを回す（増える行はほぼ 0）。テストを足すなら **34** を先に。
- **飛ばすと**: 失敗した走行が木を半分だけ書き換える。
- **外への約束**: 守る（成功したときの出力と書き出す中身は同じ）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし（設計 D-11「入力はその台帳 1 本」は描く入力の話で、書く順は縛らない）。
- **C22**: テストを `generate_tests.rs` に足すと 1,000 行を越える＝**34** を先に。
- **確かめた**: `generate.rs:175-182` のループ。対照は `:163-166`・`:192-201`。

**7. `priority-apply` が `priority` の行を取り違えて備考を書き換えることがあり、行末のコメントも消す**（`src/ledger/patch.rs:63-107`）（仮 D2）
- **何か**: 行頭が `priority = ` の行がちょうど 1 本なら、その行をまるごと `priority = "<値>"` に置き換える。その行が本当にその項目の `priority` の欄かは確かめない。
- **困る場面**: ⑴ 本物の欄が `priority="C1"`（TOML として正しく `read` も通す）で、備考の複数行文字列の中に行頭の `priority = "…"` が 1 本あると、当たるのは備考の行だけ。備考が黙って書き換わり、本物の値は変わらず、走らせるたびに「変更 1 項目」が出続ける。⑵ `priority = "C1"  # 仮置き` はコメントが黙って消える。今の台帳 4 本には、どちらの形も 0 件。
- **直し**: 当たった行が `priority = ` ＋ `basic_string(今の値)` と 1 バイトも違わないことを確かめ、違えば `undecidable` で落とす。組み上げた本文を `toml` で読み直し、各 id の `priority` が求めた値かを見る。合わせて 10 行ほど・テスト 2 本。
- **飛ばすと**: 手で書いた備考が書き換わっても誰も気づかない。
- **外への約束**: 変える（今のデータは同じ。付録 A の書き方から外れた `priority` の行で、今は通る `priority-apply` が終了コード 1 で止まる）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし（形式は変えない。付録 A に合わない行を断るだけ）。
- **確かめた**: `grep '^priority'` で形の外れた行は 4 本とも 0 件。呼び手は `cli/generate.rs:263` だけ。

#### 証拠の読み取り

**8. 語彙表の目印の後ろに表が無くても、ファイルのずっと後ろのスライスを読みにいく**（`src/evidence/resolve.rs:290-301`）（仮 D3）
- **何か**: `find_slice_start` は目印の行から本文の終わりまで探し、最初の `= &[` を表の始まりとみなす。
- **困る場面**: ファイルの頭に `//! ukadoc: https://…/list_shiori_resource.html` を書き、関数が続き、ずっと下に `const X: &[&str] = &["version"]` がある本文。`X` の要素がそのページの証拠になり、`TableMissing` は出ない。設計 D-5 の「直後に始まる最初のスライス定数」から外れる。今の目印 3 か所（`crates/areka/src/menu/captions.rs:32`・`:87`・`crates/areka-sylphya/src/vocab/shiori_resource.rs:45`）は表がすぐ後に続くので 0 件。
- **直し**: 文字列とコメントの外で `;` か `{` に出会ったら `None` を返す arm を 1 本足す。「目印→関数→スライス」の本文が `TableMissing` になるテストを 1 本。
- **飛ばすと**: 置き場を間違えた目印 1 行が、偽の「実装済み」の証拠になる。
- **外への約束**: 変える（今のデータは同じ。その形の本文で証拠と `TableMissing` の出方が変わる）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし（設計 D-5 に寄せる直し）。

**9. Rust の本文を読む小さな関数が 2 か所に別々に写してあり、決まりがもうずれている**（`src/evidence/resolve.rs:206-217,243-284,309-353` と `src/evidence/candidates.rs:388-445,513-591`）（仮 D4）
- **何か**: `line_offsets`・`read_string`・`skip_line_comment`・`slice_element_names` と `SLICE_START` が両方にある（`SLICE_START` は型も違う）。
- **困る場面**: `candidates` 側は文字リテラルとブロックコメントを飛ばすが、`resolve` 側は飛ばさない。どちらも「設計 D-5 と同じ規則」と書いてある。語彙表に `/* , */` や `']'` が入ると、候補では 1 要素なのに証拠では要素が割れる、か消える。
- **直し**: 逃がしを多く扱う `candidates.rs` 側を 1 か所に残し、`slice_element_names` が `Option` を返す形にそろえて `resolve.rs` から使う（`resolve.rs` から 80 行ほど消える）。**23** の生文字列の arm もここに置く。
- **飛ばすと**: 証拠と候補が、同じ表を違う形に読む。
- **外への約束**: 守る（今の目印付きの表 3 本は文字リテラルもブロックコメントも持たない）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。

#### 整合テストの偽の赤・見落とし

**10. 宛先の書き換えの摂動が「最初の宛先は roadmap-draft の [[spec]] の行にある」ことに黙って寄りかかっている**（`tests/consistency/spec_checks.rs:457-488`）（仮 E2）
- **何か**: shiori の台帳で最初に見つかる非空の宛先（今は `areka-P0-anchor-tag-canon`）をよその名前に書き換え、腕 c（`owner_count_findings`）が「ちょうど 1 件」赤くなることを主張する。
- **困る場面**: 腕 c は `roadmap.specs` の行しか見ない（:178-192）。`areka-P0-anchor-tag-canon` はもう completed にあるのに `roadmap-draft.md:687` の [[spec]] に残っている。棚卸でこの行を `briefing.md` の [[owner_completed]] へ正しく移すと、実データの判定は緑なのに、この摂動は所見 0 件で「赤が 1 件でない」で落ちる。
- **直し**: 書き換える項目を選ぶ条件に「宛先が `fixture.documents.roadmap.specs` の名前にある」を足す（1 行）。
- **飛ばすと**: 完了した spec の行を正しく整理した人が、壊れていないのに赤を見る。
- **外への約束**: 変える（整合テストの偽の赤が 1 つ消える）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。
- **確かめた**: `spec_checks.rs:455-488` を読んだ。`ledger/shiori.toml:1148` の最初の owner、`roadmap-draft.md:687`。

**11. 「該当なし」の腕の較正が「実データに『該当なし』の束が 0 本」を合否に入れている**（`tests/consistency/linkage_checks.rs:766-773`・注記 `:31-39`）（仮 E3）
- **何か**: 写しの束を 1 つ「該当なし」にして腕 e が赤くなることを見る較正。その前に、実データの束が全部「該当なし」でないことを assert している。
- **困る場面**: 全構成 id が implemented の束に `breakage = "該当なし"` と書くのは要件 4.1 ⑺ が認める正しい書き方で、実データの判定も緑のまま。それでもこの assert が「前提が変わった」で赤になる。較正は `a_bundle_with_an_unfinished_member` が実装済みでない id を持つ束を選ぶので、この前提が無くても成り立つ。
- **直し**: その assert（8 行）を消し、注記 :31-39 を短くする。
- **飛ばすと**: 要件どおりに「該当なし」を初めて書いた人が赤で止まる（`kiro-next` の Step 8 でも止まる）。
- **外への約束**: 変える（偽の赤が 1 つ消える）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。
- **確かめた**: `linkage_checks.rs:760-776` を読んだ。`linkage.md` の `該当なし` は 0 件・束は 67。

**12. 判定 ⑷ の腕 f は `[[barrier]]`・`[[after]]` の行が欠けても緑**（`tests/consistency/briefing_arms.rs:593-641`・下限は `documents_non_vacuity.rs:583-603`）（仮 F2）
- **何か**: 腕 f は、書いてある行それぞれを台帳から数え直した数と比べる。
- **困る場面**: `briefing.md` から `[[after]]` の 4 行（またはどれか 1 ドメイン）や `[[barrier]]` の 6 行を消しても、読み手が 0 行として受け付け（`parse.rs:547`・`:567`）、比べる相手が無いので ⑷ は緑。⑷ の下限は `[[rank]]` と段階 A の items しか見ない。
- **直し**: `distribution_findings` で `after.domain` の集合が `Domain::ALL` と違えば所見を 1 件出す（5 行）。⑷ の下限に `briefing.barriers.len() >= 1` を足す（5 行）。
- **飛ばすと**: 書き戻した後の分布の表が消えても、誰も気づかない。
- **外への約束**: 変える（今のデータは同じ＝barrier 6 行・after 4 行。落ちる条件が増える）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。

#### 3 文書・カタログの読み手の検査の抜け

**13. `toml_blocks` と `prose_lines` で、コードの囲みの見分け方が違う**（`src/documents/parse.rs:82-106`・`:167-181`）（仮 B1）
- **何か**: `toml_blocks` は行の末尾の空白だけを削り、行がちょうど「```toml」のときだけ囲みを開く。`prose_lines` は行の頭の空白を削り、「```」で始まる行のたびに中と外を切り替える。同じ「囲みはどこからどこまでか」の決まりの写しが 2 通り。
- **困る場面**: ⑴ 箇条書きの中に字下げした「  ```toml」や「```TOML」の囲みは `toml_blocks` から見えず（表が読まれない）、`prose_lines` は囲みとみなして中の id を拾わない＝書いた束が「どの束にも属さない」という取り違えた赤になる。⑵ 「```text」の例の中に「```toml」の行を置くと、`toml_blocks` は例を本物の TOML として読み、`prose_lines` はそこで外に出たことになって、その後ろが文書の終わりまで中と外が逆さになる（地の文の裸の id を黙って取りこぼす）。
- **直し**: 「囲みの中か・それは toml か」を持つ走査を 1 本書き、両方が使う（開くのは行頭の空白を削った行が ``` で始まるとき・閉じるのは前後の空白を削った行がちょうど ``` のとき）。差し引き約 5 行減る。字下げと入れ子の例の 2 件をテストに。
- **飛ばすと**: 文書の書き方が 1 つずれるだけで、裸の id の網が黙って外れる。
- **外への約束**: 変える（今のデータは同じ＝3 文書の柵は全部 0 桁目の「```toml」「```sh」で、字下げ・入れ子は 0 件）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。

**14. 見出しで始まらない囲みが、前の囲みの最後の表へ黙って混ざる**（`src/documents/fields.rs:57-58`）（仮 B2）
- **何か**: `read_blocks` は囲みを改行でつないで 1 度だけ読む（設計 D-1）。囲みの頭に見出しが無い鍵は、TOML の決まりで前の囲みの最後の表に属する。
- **困る場面**: `briefing.md` で、ある囲みが `[[rank]]` で終わり、次の囲みが見出しを書き忘れて `insufficient = true` や `override = {…}` で始まると、その欄は 1 つ前の順位の行に黙って付く。省いてよい欄ほど重複の誤りにもならない。
- **直し**: つなぐ前に、囲みごとに「空行と `#` の行を除いた最初の行が `[` で始まらない」ものを `malformed` で落とす（6 行）。
- **飛ばすと**: 書き忘れた見出し 1 つが、別の行の値として通る。
- **外への約束**: 変える（今のデータは同じ＝82 個の toml の囲みは全部見出しで始まる。整合テストと `priority-apply` の落ちる条件が増える）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。

**15. 同じ URL の項目が 2 つあっても、カタログは黙って組み上がる**（`src/catalog/build.rs:45-57`・`src/catalog/read.rs:88-94`）（仮 B3）
- **何か**: `build` は同じ id の 2 度目は止めるが、同じ URL は確かめない。`by_url` は「壊れたカタログでは id の大きい方だけが残る」と書いて黙って片方を捨てる。
- **困る場面**: 新しいスナップショットで 2 つの entry が同じ `url` を持つと、`evidence::resolve`（:79・:93）がその URL の証拠を全部 id の大きい方へ付け、小さい方は黙って証拠 0 件になる。どこにも赤が出ない。
- **直し**: `build` の繰り返しで URL を `BTreeMap<&str, &EntryId>` に入れ、2 度目が来たら id の重複と同じ `SnapshotShape` で 2 つの id を挙げて落とす（6 行）。`by_url` の注記を消す。テスト 1 本。
- **飛ばすと**: 設計 D-4 の実測（URL は全部相異なる）が崩れた日に、証拠の帰属が黙って狂う。
- **外への約束**: 変える（今のスナップショットでは同じ。`catalog` が終了コード 1 で失敗する条件が 1 つ増える）。
- **範囲の外へ**: なし。
- **残す候補**: ⑶ には当たらない見込み（ukadoc の中身でなく、写しの検査）。ukadoc 側で同じ URL を持つ項目が正当にありうるかは 11 節の調べ。

**16. 単独項目の名前と、その中の id が食い違っても読み手が通す**（`src/documents/parse.rs:285-300`）（仮 B4）
- **何か**: 単独項目は「名前が id そのもの」が前提（`priority_by_id` は `singles` の id を束名として引く・`derive.rs:205-210`）。読み手は「`members` がちょうど 1 つ」までしか見ない。
- **困る場面**: 単独項目の囲みを写して `members` だけを書き換え、見出しを直し忘れる（`[bundle."ukadoc:X"]`・`members = ["ukadoc:Y"]`）。`briefing.md` の `singles = ["ukadoc:X"]` の順位が Y に付き、`priority-apply` で Y の台帳へ書き戻される。
- **直し**: `single` の枝で `members[0].as_str() != name` なら両方の綴りを挙げて `malformed`（4 行）。
- **飛ばすと**: 書き写しの誤り 1 つで、違う項目に優先度が付く。
- **外への約束**: 変える（今のデータは同じ＝単独項目 4 件は全部一致・`linkage.md:4758`・`:4773`・`:4789`・`:4803`）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。

**17. 1 つの束の `members` に同じ id を 2 度書けて、導出と順位で扱いが割れる**（`src/documents/parse.rs:280`・`src/documents/derive.rs:61`・`:226-235`）（仮 B12）
- **何か**: 読み手は `members`・`hand`・`machine` の重複を見ない。`derive_bundle` は 1 件に畳み、`priority_by_id` は 2 度目を「2 つの束に属している（後の束は X）」として落とす。
- **困る場面**: `members = ["ukadoc:a:b:1", "ukadoc:a:b:1"]` と書くと、優先度の導出は別の束と取り違えた文面で落ち、腕 a は「束 X と束 X の両方」と出し、数え直し（`linkage_checks.rs:402`）は重複を 2 と数える。
- **直し**: `read_bundle` で重複があればその id を挙げて `malformed`（8 行）。`derive.rs:60-61` の畳みと注記を消す。
- **飛ばすと**: 重複 1 つで、3 か所が 3 通りの文面を出す。
- **外への約束**: 変える（今のデータは同じ見込み＝腕 a が緑なので重複は 0 件。読み手が落とす条件が増える）。
- **範囲の外へ**: なし。
- **残す候補**: 該当なし。

**18. `[[rank]]` の `singles = []` が通り、腕 e も黙って飛ばす**（`src/documents/parse.rs:486`）（仮 B13）
- **何か**: 読み手は空の `singles` を受け付ける。`priority_by_id` は何も配らず、腕 e の `key_of`（`briefing_arms.rs:359` の `keys.first()?`）は `None` を返す。
- **困る場面**: 段階 B の末尾に `rank = 4`・`singles = []` の行を残すと、中身の無い順位の行が載ったまま、どこも赤にならない。
- **直し**: `read_rank` の `Singles` の枝で空なら `malformed`（3 行）。
- **外への約束**: 変える（今のデータは同じ＝`singles = []` は 0 件）。
- **飛ばすと**: 指す相手の無い順位の行が残せる。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**19. カタログの読み手だけが、根の知らない表や鍵を見逃す**（`src/catalog/read.rs:70-78`）（仮 B14）
- **何か**: `read` は `[snapshot]` と `[entry]` の中の知らない欄は落とすが、根のそれ以外は見ない。3 文書の読み手は根も確かめる（`parse.rs:239`・`:431`・`:660`）。同じ決まりの当て方が割れている。
- **困る場面**: `catalog.toml` に `[alias]` の表や `note = "…"` が混じっても読めてしまい、`write(read(t)) == t`（設計の事後条件）が黙って破れる。
- **直し**: `read` の頭で `reject_unknown_keys(&root, &[SNAPSHOT_TABLE, ENTRY_TABLE], "根")?` を 1 行。テスト 1 件。
- **外への約束**: 変える（今のデータは同じ＝根は 2 つの表だけ。全副手続きと整合テストがカタログを読めない条件が 1 つ増える）。
- **飛ばすと**: 形の違うカタログが黙って通る。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**20. 項目 id の 4 欄目（連番）が数字でなくても通る**（`src/model.rs:160-162`）（仮 B17）
- **何か**: アンカー付きの形で見るのは、どの欄も空でないことだけ。
- **困る場面**: `linkage.md` に `"ukadoc:list_shiori_event:OnBoot:l"`（1 の代わりに英小文字の l）と書くと id として通り、その先で「台帳に無い」という別の理由で赤になるか、確かめない場所なら素通りする。
- **直し**: 4 欄目を `seq.bytes().all(|b| b.is_ascii_digit())` にする（1 行）。誤りの例を 1 件足す。
- **外への約束**: 変える（今のデータは同じ見込み＝実データの連番は数字。数え直しは 11 節）。
- **飛ばすと**: 打ち間違えた id が、id の形の誤りとして名指されない。 **範囲の外へ**: なし。
- **残す候補**: ⑴ に当たりうる（完了 toolkit の要件 1.9 は「`ukadoc:<ページ>:<アンカー>:<連番>`」としか書かない＝数字に絞るのは要件の読み方を一歩狭める）。

**21. 除外するディレクトリを深さを問わず名前だけで当てるので、`target` という名の Rust モジュールのディレクトリが黙って走査から落ちる**（`src/io/sources.rs:33`・`:76`）（仮 A4）
- **何か**: `crates/` の下のどの深さでも、名前が `target`・`vendors`・`.git` のディレクトリを飛ばす。
- **困る場面**: 実在の `crates/areka-emo-present/src/presenter/target.rs` がふつうの分け方で `presenter/target/mod.rs` と子へ分かれると、その下の `// ukadoc:` の URL が証拠から消え、`check` が `ImplementedWithoutEvidence` を誤って出す。守るべき相手は今 0 件（ワークスペースは `members = ["crates/*"]`・作ったものは根の `target/`・`vendors/` も根）なので、`walk_excludes_build_output_and_vendored_and_git_dirs`（`sources_tests.rs:121-141`）も除外を外す壊れ方を捕まえられない。
- **直し**: 除外をクレートの直下（`rel` が `crates/<名前>` のとき）だけに絞る（条件 1 つ）か、定数ごと消す（-3 行）。テストを今の形に合わせる。
- **外への約束**: 変える（今のデータは同じ＝該当するディレクトリが現れたときだけ `evidence`・`candidates`・`check` と整合テストの結果が変わる）。
- **飛ばすと**: モジュールの分け方ひとつで、本物の証拠が黙って消える。
- **範囲の外へ**: なし（`crates/log-capture-kit/tests/workspace_scan/mod.rs:48,106` も同じ決まりだが範囲の外なので触らない）。 **残す候補**: 該当なし。

**22. 出力の先のパイプが閉じると `print!` が panic し、終了コードが 0／1／2 でなく 101 になる**（`src/cli/inspect.rs:85,114,124,139`・`src/cli/generate.rs:377`）（仮 A5）
- **何か**: 結果を `print!`／`println!` で標準出力へ出す。
- **困る場面**: `ukadoc-survey check | head -n 5`（Git Bash）や PowerShell の `| Select-Object -First 5` でパイプが閉じると、書けなかった `print!` が panic して終了コードは 101、標準エラーには panic の文。`main.rs:14-25` は `Err` を 1 に写すだけで panic は受けない。
- **直し**: `cli/mod.rs` の `write_notice` の隣に `write_stdout(body)`（`stdout().lock().write_all` の失敗を `SurveyError::Io{path:"<標準出力>"}` に・6 行）を置き、5 か所を替える。
- **外への約束**: 変える（パイプが閉じたときの終了コード 101 が 1 に、panic の文が読み書きの失敗の 1 行に）。
- **飛ばすと**: パイプで頭だけ読んだときだけ、終了コードの約束が破れる。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**23. `#[cfg(test)] mod` の範囲を数えるとき、生文字列で括弧の数えがずれる**（`src/evidence/candidates.rs:299-328`・`:483-507`）（仮 D11）
- **何か**: `matching_brace` は `"` を普通の文字列として読むので、`r#"…"#` の中の `"` で文字列を閉じたことになる。
- **困る場面**: テストの中に `r#"a"b"#` のような引用符を奇数個持つ生文字列があると、そこから先で読み違え、テストの中身が範囲の外へ漏れて候補に出る（`None` なら丸ごと漏れる）。今の作業ツリーで実際に何件起きているかは数えていない（11 節）。
- **直し**: `r#*"` で始まる生文字列を対応する `"#*` まで飛ばす arm を 1 本（10 行）。**9** でまとめた 1 か所に置けば証拠の側も同時に直る。テスト 1 本。
- **外への約束**: 変える（`candidates` の出力が減ることがある＝今のデータで変わるかは未確認）。
- **飛ばすと**: テストの見本が候補に混ざることがある。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

#### 台帳の手続きの端

**24. 備考の中に隠れた重複の見出しがあると、`priority-apply` は的外れの文言で止まる**（`src/ledger/patch.rs:67-75`）（仮 D8）
- **何か**: `replace_priority` は並びを確かめずに、すべての塊で `priority` の行を数える。
- **困る場面**: 項目 A の備考に行頭の `[entry."B"]`（B は実在）が書かれていると、較正は集合で比べるので通り、「項目 B の priority の行が無い」で止まって本当の原因（A の備考）が書かれない。`read` は並びを見ないので、ここも素通りする。
- **直し**: `write::check_strictly_ascending` を `pub(super)` にして `replace_priority` の頭で呼ぶ（2 行）。テスト 1 本。
- **外への約束**: 変える（その形の台帳での終了時の文言が `LedgerOutOfOrder` になる）。
- **飛ばすと**: 直す場所を探すのに時間がかかる（止まりはするのでデータは失わない）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**25. 項目の間のコメントは前の塊の尻尾になるので、差し込んだ項目がコメントとその項目の間に入る**（`src/ledger/write.rs:184-192`・`src/ledger/blocks.rs:115-127`）（仮 D5）
- **何か**: 塊の終わりは次の見出しの直前に決まる。項目 B の真上に書いたコメントは、1 つ前の項目 A の塊に入る。
- **困る場面**: `# --- 群 3 ---` の直後に `[entry."B"]` がある台帳で、`ledger-init` が A と B の間に欠けた id X を差し込むと「コメント → X → B」になり、B のコメントが X に付いたように読める。今の台帳 4 本では項目の間のコメントは 0 件。
- **直し**: 差し込む位置を、前の塊の末尾に続く行頭 `#` の連続した行の前へずらす（`splice` の中で 5 行）。テスト 1 本。
- **外への約束**: 変える（その形の台帳でだけ、`ledger-init` が書き出す本文の差し込み位置が変わる。今の台帳では 1 バイトも変わらない）。
- **飛ばすと**: 手で付けた見出しのコメントが、ずれた項目に付いて残る。 **範囲の外へ**: なし。 **残す候補**: 該当なし（形式は凍結のまま）。

**26. 台帳の読みの失敗の文言に、どの台帳かが載らない**（`src/ledger/read.rs:101`・`src/ledger/blocks.rs:58,187-192`）（仮 D6）
- **何か**: `read` はドメインを知っているのに、`blocks::split` の失敗をそのまま上げる。
- **困る場面**: `shiori.toml` の備考に行頭の `[entry.x の説明]` 行があると、`check` は「`doc/ukadoc-coverage/ledger/<ドメイン>.toml`: 塊の見出しが読めない」と出し、4 本のどれかが書かれない。`LedgerSplitMismatch` には置き場そのものが無い。要件 6.12「置き場を必ず添える」から外れる。
- **直し**: `read` の中で `split` の `TomlParse` の `path` を `file` に差し替え、`LedgerSplitMismatch` の `detail` の頭に `file` を付ける（5 行）。
- **外への約束**: 変える（失敗のときに標準エラーへ出す文言だけ）。 **飛ばすと**: 台帳 4 本を探し直す手間が残る。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**27. 提供パッケージの警告の行は、相対パスを渡すと相対の綴りのまま出る**（`src/io/snapshot.rs:118-120`・`:144-168`）（仮 A7）
- **何か**: `load` は失敗の本文に絶対パス（`shown`）を出すが、`package_info` には元の `path` を渡す。
- **困る場面**: `AREKA_UKADOC_SNAPSHOT=index.json` で打つと「警告: …: index.json（2 つ上のディレクトリが無い）」と出て、どこを見たのか分からない。
- **直し**: `snapshot.rs:120` を `package_info(Path::new(&shown))` にする（1 行）。
- **外への約束**: 変える（警告の行のパスが絶対になる。名前だけの相対パスでは作業ディレクトリの 1 つ上の `package.json` を見に行くようになる）。
- **飛ばすと**: 警告の行だけ「探した絶対パス」の決まりから外れる。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**28. `bundle` の型の誤りが「欄が無い」と報告される**（`src/documents/parse.rs:738-743`）（仮 B11）
- **何か**: `string_field(...).map_err(|_| …「欄 bundle が無い」…)` が、欄が無いときと文字列でないときを同じ文面に塗り替える。
- **困る場面**: `roadmap-draft.md` の `[[spec]]` に `bundle = ["時刻の刻み"]` と書くと「欄 bundle が無い（属さないなら none = true と reason を書く）」と出て、書き手は迷う。
- **直し**: 先に `item.contains_key("bundle")` を見て、無いときだけ今の文面、あるときは `string_field` の誤りをそのまま返す（2 行）。
- **外への約束**: 変える（型の誤りのときだけ文面が変わる）。 **飛ばすと**: 取り違えた案内が出る。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**29. 1 項目に同じテーマを 2 度書くと、テーマ別の表で 2 件に数わる**（`src/report/tally.rs:155-157`）（仮 R4）
- **何か**: テーマ別の分布で、項目の `values` を 1 つずつ数える。
- **困る場面**: 台帳に `values = ["気配", "気配"]` と書くと、項目は 1 件なのに「気配」が 2 件になる。`ledger/read.rs:273-286` は重複を落とさず、検査（`check/content.rs:256-`）も語の実在だけを見る。テーマ別の合計はもともと項目数を超えてよいので、食い違いがどこにも現れない。`documents/derive.rs:63-69` は `BTreeSet` で畳んでいて決まりがそろっていない。今の実データは 1,749 項目中 0 件。
- **直し**: 155 行の繰り返しを `entry.values.iter().collect::<BTreeSet<_>>()` で回す（2 行）＋テスト 1 本。赤にしたいなら `ledger::read` で拒む（どちらにするかは設計）。
- **外への約束**: 守る（今の実データは重複 0 件なので `report/*.md` の中身は変わらない）。
- **飛ばすと**: 手で書いた台帳に重複が混じると、テーマ別の数が黙って膨らむ。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**30. 前置きの `pages` の重複が素通りする**（`src/check/structure.rs:80-86`）（仮 C9）
- **何か**: 前置きの担当ページを集合にしてから割り当て表と比べる。
- **困る場面**: 手直しやマージで `pages = ["list_propertysystem", "list_propertysystem"]` になっても、集合では一致して所見 0 件。`ledger::read`（:150-153）も重複を落とさない。前置きは以後バイト列のまま写されるので、重複は残り続ける。
- **直し**: `if declared.len() != ledger.pages.len()` を「余分」と同じ形の所見にする 2〜3 行と、重複で赤になるテスト 1 本。
- **外への約束**: 変える（今のデータは同じ。整合テストと `check` の落ちる条件が 1 つ増える）。
- **飛ばすと**: 誤った前置きが緑のまま残る。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

#### 壊れても落ちないテスト（Missing test）

**31. 形の版の注意のテストが、本文に元から入っている「2」で緑のままになる**（`src/cli/inspect_tests.rs:624-634`）（仮 A3）
- **何か**: `comparability_notice` が新旧両方の形の版を名指しすることを、`"1"` と `"2"` が本文に含まれるかで確かめる。
- **困る場面**: 実装が `next.catalog_format` を書き忘れて「現行 1 / 新しい方 1」と出しても、決まった文「注意: 2 つのカタログは…」（`inspect.rs:329`）の「2」で当たって緑。
- **直し**: 当てる文字列を `"現行 1"` と `"新しい方 2"` にする（1 行）。
- **外への約束**: なし。 **飛ばすと**: 新しい方の版を落とす取り違えを見逃す。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**32. スナップショットの場所の決め方（空の環境変数は既定へ戻る・APPDATA が無ければ失敗）に、壊れたら落ちるテストが無い**（`src/io/snapshot.rs:90-100`）（仮 A6）
- **何か**: `AREKA_UKADOC_SNAPSHOT` が空でなければそれを使い、空か無ければ `APPDATA` から組み立てる。
- **困る場面**: `!given.is_empty()` を消す壊し方をしてもどのテストも赤にならず、`AREKA_UKADOC_SNAPSHOT=`（空）で打つと `PathBuf::from("")` を読みに行く。在中テストは環境変数を読まず（`snapshot_tests.rs:9-12`）、`tests/cli_streams.rs:147,251` は空でない値しか渡さない。
- **直し**: 値を引数で受ける `fn resolve(given: Option<OsString>, appdata: Option<OsString>)` に切り出し（`default_path_from_appdata` と同じ形）、4 通りを 1 本のテストで（+15 行）。
- **外への約束**: なし（振る舞いは同じ）。 **飛ばすと**: 空の環境変数の扱いが黙って壊れうる。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**33. `parse.rs` の形の検査の分岐の多くを、在中テストが縛っていない**（`src/documents/parse.rs:102-104・281-317・502-522・727-751`）（仮 B5）
- **何か**: 閉じ忘れた囲み・空の `members`・単独項目が `machine`／`hand`／`foundation` を持つ・単独項目の `members` が 2 つ以上・空白だけの `reason`・空の `foundation`・`read_override` の全部・`read_bundle_ref` の 3 つの枝。
- **困る場面**: 例えば 287 行の条件を書き換えて単独項目が id を 2 つ持てるようにしても、在中テストは緑。実データの単独項目は 4 件とも 1 id なので整合テストも気付かない。`override` の `second-stage` は実データに 0 件なので、綴りの対応表が壊れても誰も赤にしない。
- **直し**: 見本を 1 か所ずつ書き換えたものを並べて回すテストを 1 本（落ちることと、本文に欄の名前が載ること・約 60 行）。
- **外への約束**: なし。 **飛ばすと**: 形の検査が黙って緩んでも分からない。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**34. `generate_tests.rs` が 948 行で、テストを 1〜2 本足すと 1,000 行の番人を越える**（`src/cli/generate_tests.rs:681-948`）（仮 A15）
- **何か**: 1 つのファイルに、台帳の初期化のテスト（1-679 行）と `priority-apply` のテスト（681-948 行）の 2 つの仕事が入っている。
- **困る場面**: **6**・**7** などで `report`／`priority-apply` のテストを足すと 1,000 行を越え、番人が赤になる。
- **直し**: `priority-apply` の節（約 268 行）を `src/cli/generate_priority_tests.rs` へ移し、`generate_tests.rs` の中に `#[path = …] mod priority;` を置いて `LEDGER_FILES`・`under_coverage`・`spelled_path` を共有する（仕事で分ける）。
- **外への約束**: なし。 **飛ばすと**: 次に足すテストが番人に引っかかる。 **範囲の外へ**: なし。 **残す候補**: 該当なし。
- **C22**: 番人のファイル（`file_length_guard_test.rs`）には触らない。越えを避けるための直し。

**35. コミットされた `catalog.toml` が「読んで書き直すと 1 バイトも変わらない」ことを、常時テストが確かめていない**（範囲の `tests/consistency/` 全体・相手は `src/catalog/write.rs:53`）（読み係 B の手掛かりをまとめ役が確かめた）
- **何か**: 台帳には「作り直すと 1 バイトも同じ」のテストがある（`generate_tests.rs` の `re_running_the_init_writes_the_committed_ledgers_with_the_very_same_bytes`）が、カタログには無い。`catalog::write` を呼ぶのは本番の `cli/generate.rs:60` と在中の `read_tests.rs` だけ。
- **困る場面**: `catalog.toml` を手で並べ替えたり空白を変えたりしても常時テストは緑のまま（完了 toolkit の要件 1.5 の「1 バイトの安定」を、コミットされた実物では守っていない）。次に `catalog` を回した人の差分に、関係の無い書き換えが混じる。
- **直し**: 整合テストに `assert_eq!(catalog::write::write(&data.catalog), 読んだ本文)` を 1 本足す（数行）。
- **外への約束**: 変える（整合テストが落ちる条件が 1 つ増える）。今の `catalog.toml` がこの形を満たしているかは未確認（11 節）。満たしていなければ `catalog.toml` の作り直しが要り、要件 6.4（書き出す中身を変えない）と `ukadoc-catalog-refresh` の持ち物に触れる＝議題。
- **飛ばすと**: 手の入ったカタログが黙って通る。 **範囲の外へ**: 満たしていなければ `doc/ukadoc-coverage/catalog.toml`。 **残す候補**: 該当なし。 **C22**: `assets.toml` ではないのでぶつからない。

**36. 読み込みの較正に、ほかのクレートのファイルを名指ししている**（`src/io/files_tests.rs:131-135`・`src/io/sources_tests.rs:24`）（仮 A19）
- **何か**: 復帰文字を落とす読み込みの較正に `crates/areka-sylphya/src/lib.rs` と `crates/log-capture-kit/tests/workspace_scan/mod.rs` を使い、走査の肯定側にも後者を使う。
- **困る場面**: 関係の無い log-capture-kit のテストの道具を移すか名前を変えるだけで、このクレートのテストが赤になる（`runtime-crate-split` のような移し替えで起きる）。
- **直し**: `files_tests` はこのクレート自身の追跡ファイル（`crates/ukadoc-survey/Cargo.toml`＝復帰文字 25 個・`src/lib.rs`）を使う。`sources_tests` の `KNOWN_FILES` から log-capture-kit の 1 本を外す（設計 D-5 が名指しする sylphya の 2 本は残す）。
- **外への約束**: なし。 **飛ばすと**: よそのクレートの整理でこのクレートが赤になりうる。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

#### 同じ決まりの写し（merge／reuse）

**37. 「どの台帳にも割り当ての無いページ」の決まりが 2 か所に違う書き方で置いてあり、区切りの定数も 2 つ**（`src/cli/generate.rs:47-48,300-329`・`src/catalog/build.rs:33,59-68`）（仮 A9）
- **何か／困る場面**: `select_ids_by_domain` は `domain_of` と自前の `BTreeSet` で集め `"・"` でつなぐ。`build.rs` は `assignment.unassigned()` で同じことをし、`PAGE_SEPARATOR` を別に持つ。片方の区切りや並びだけ変えると、同じ `PageNotAssigned` が副手続きによって違う綴りで出る。
- **直し**: generate 側も `assignment.unassigned(...)` を使い、失敗の組み立ては `build.rs` に `pub(crate) fn page_not_assigned` で 1 つだけ置く（-10 行）。
- **外への約束**: 守る（綴りは今と同じ「・」区切り・名前順）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**38. カタログと台帳 4 本の読み方が 5 か所と 3 か所に写してある**（`src/cli/inspect.rs:66,112,133,365-372`・`src/cli/generate.rs:160,192-196,347-350`）（仮 A10）
- **何か／困る場面**: `read_catalog(&files::read_normalized(&paths::catalog_path())?)?` が 5 回、台帳 4 本を読むループが `inspect::load_ledgers` と `generate::report_summary` の 2 か所、さらに `generate::load_ledger`。読み方を 1 つ変えると副手続きによって振る舞いが割れる。
- **直し**: `load_catalog()` と `load_ledgers()` を `cli/mod.rs` に 1 つずつ置き（`pub(super)`）、両方の部から呼ぶ。`generate::load_ledger` を消す（-12 行）。**6** の直しもこれに乗る。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**39. TOML の欄を取り出す道具が 3 か所に写してある**（`src/catalog/read.rs:127-267`・`src/documents/fields.rs:26-242`・`src/ledger/read.rs:130-378`）（仮 B6）
- **何か／困る場面**: `malformed`・`reject_unknown_keys`・`field`・`string_field`・`string_array_field`・`sub_table`（`required_table`）が、場所の引数の有無だけ違って 3 か所にある。失敗の文面を 1 か所で直しても、ほかは古い文面のまま。
- **直し**: `fields.rs` の道具を `pub(crate)` にし、`catalog/read.rs`・`ledger/read.rs` から場所を渡して呼ぶ（約 70 行以上減る）。
- **外への約束**: 守る（失敗の本文が今と同じ綴りになるよう、引数の順だけ合わせる）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**40. カタログの列の並びが、読み手と書き手に 1 つずつある**（`src/catalog/read.rs:46-58`・`src/catalog/write.rs:47・84-93`）（仮 B7）
- **何か／困る場面**: `ENTRY_COLUMNS` の 6 語と冒頭の情報の 8 欄が、read と write に分かれて書いてある。欄を足すとき片方だけ直すと自分で読めなくなる（往復のテストは赤になるが、直す場所が 2 つ）。
- **直し**: `catalog/mod.rs` に `pub(super) const ENTRY_COLUMNS` と `SNAPSHOT_FIELDS` を 1 つずつ置く。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**41. 台帳の置き場・表の名前の綴りが何か所にも写してある**（`src/check/structure.rs:47-50`・`src/check/content.rs:51-54`・`src/documents/derive.rs:261-263`・`src/ledger/{read.rs:50,56,125-127, write.rs:49,55,233-235, blocks.rs:58,64, patch.rs:46}`・`src/documents/parse.rs:53`・`src/catalog/read.rs:37`・`tests/consistency/briefing_arms.rs:177`）（仮 C8・B8・D10）
- **何か／困る場面**: `format!("doc/ukadoc-coverage/ledger/{}.toml", …)` の `ledger_file` が 5 か所＋テスト 1 か所、`LEDGER_DIR`・`LEDGER_FILE`・`ENTRY_TABLE` が ledger の 4 ファイルに、`DOCUMENT_DIR`・`CATALOG_FILE` は `io/paths.rs:32` の `COVERAGE_DIR` の写し。置き場の綴りを変えるとき片方を忘れると、同じ台帳の所見が種類によって違う場所を名指し、`render` は場所ごとに見出しを分けるので 1 つの台帳の所見が 2 つの見出しに割れる。
- **直し**: `ledger/mod.rs` に台帳の綴り（`ledger_file`・`ENTRY_TABLE`）を 1 つずつ、`io/paths.rs` の `COVERAGE_DIR` を `pub` にして相対の綴りを返す関数を置き、全部をそれに寄せる（-20 行ほど）。
- **外への約束**: 守る（失敗の本文と所見の場所の綴りは変えない）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**42. 壊れ方の序列を 2 か所で別々に決めている**（`src/documents/derive.rs:120-129`・`src/documents/mod.rs:76-82`）（仮 B10）
- **何か／困る場面**: `Breakage::ALL` は「悪いほうから並べる・順位の第 1 根拠の序列」と書き、`breakage_weight` は同じ序列を 3・2・1・0 の手書きで持ち直す。`ALL` の並びを入れ替えても重みは変わらず、食い違っても誰も赤にしない。
- **直し**: 重みを `ALL` の中の位置から作り、手書きの match を消す（-6 行）。`axis_key` の値と型は変えない。
- **外への約束**: 守る。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**43. 判定 ⑵ が id の綴りの決まりを手で写している**（`tests/consistency/documents_checks.rs:252-268`）（仮 F7）
- **何か／困る場面**: `ids_in_cell` と `is_id_char` が `parse.rs:184-186` の非公開の `is_id_char` の写し。parse 側で id に使える文字が増えると、⑴ は id を最後まで読むのに ⑵ は途中で切り、切れた id が「報告に無い束 id」として誤って赤になる。
- **直し**: 238 行目を `ids.extend(cited_ids(cell))` にし、2 つの関数を消す（-17 行）。升目に裸で書いた id は ⑴ の `bare_id_tokens` がもう赤にする。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**44. 段の頭文字の比べを 2 か所に書いている**（`tests/consistency/briefing_arms.rs:143`・`:612-613`）（仮 F12）
- **何か／困る場面**: `items_of` と `[[after]]` の数え直しがどちらも `priority.starts_with(stage.as_key())` を書く。段の綴りの決まりが変わったとき片方だけ直ると、腕 b と腕 f の数え方がずれる（注記は「数え方は 1 つに集める」と書いている）。
- **直し**: `items_of(stage, keep_domain)` の形に 1 つにまとめ、613 行目から呼ぶ。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**45. 0 件のテーマの行を並べる決まりが 2 か所にある**（`src/report/tally.rs:133-137`・`src/report/domain.rs:95,141-150`・`src/report/summary.rs:141,233-242`）（仮 R5）
- **何か／困る場面**: `tally` が定数 `THEMES` で 8 テーマを 0 件で用意し、`render_*` が引数の `themes` で同じことをする。引数でテーマを減らせず、`content_link_tests.rs:709-721` が 7 つに絞った `themes` を渡しても報告には 8 つが並ぶ（「テーマ名は引数から来る」の説明 `domain.rs:53-55` と食い違う）。本番の呼び手は全部 `&THEMES` なので今の出力は正しい。
- **直し**: `tally.rs:133-137` の用意をやめて `BTreeMap::new()` にし、並べるのは `theme_names` だけに任せる。`tally_tests.rs` の 3 本（318-338・384-405・410-420）を直す。
- **外への約束**: 守る（今の呼び手では報告の版面は変わらない）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**46. `theme_names`・`push_status_header`・`push_status_row` が `domain.rs` と `summary.rs` にそっくり 2 つある**（`src/report/domain.rs:108-150`・`src/report/summary.rs:175-242`）（仮 R6）
- **何か／困る場面**: 同じ 3 つの関数が 1 文字違わず 2 か所にある。版面は全文一致のテストが固定するので黙って動くことは無いが、`theme_names` は決まりそのもので、片方だけ直す誤りが通る。
- **直し**: 3 つを `tally.rs`（または `domain.rs` の `pub(super)`）に 1 つだけ置き、`summary.rs` から引く（-40 行）。
- **外への約束**: なし（版面は同じ）。 **範囲の外へ**: なし。 **残す候補**: 該当なし（`summary.rs:180-182` の説明の判断で、開発者の裁定ではない）。

**47. `Perturbed::input` が検査の入力を自前で並べ直している**（`tests/consistency/perturb.rs:98-107`）（仮 R11）
- **何か／困る場面**: 割り当て表と `THEMES` を `RepoData::input`（`mod.rs:181-190`）とは別に手で並べる。`mod.rs:111-113` が「組み合わせをテストごとに書くと、渡し忘れた欄が黙って別の値になる」と 1 か所に決めたのに、2 か所目になっている。`CheckInput` に欄が増えたとき（**4** の直しがそれ）、写しの側だけ古い値を渡しかねない。
- **直し**: `CheckInput { catalog: &self.catalog, …, ..self.data.input() }` にする（-2 行・`THEMES` の import も消える）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**48. 整合テストの helper と綴りの定数がファイルの間で写しになっている**（`tests/consistency/{linkage_checks.rs:68,527-551, spec_checks.rs:63-66,312-336, briefing_checks.rs:40-64, documents_checks.rs:93-95,382-389, checks.rs:80, values_md.rs:191,299, non_vacuity.rs:52,80, briefing_arms.rs:73,76, documents.rs:93-95}`）（仮 E4・F9）
- **何か／困る場面**: `assert_no_findings` が 4 本、`assert_single_finding` が 3 本、一字一句同じ。`LINKAGE_MD` が 4 か所、`ROADMAP_MD` が 2 か所、`crates/areka-sylphya/src/vocab/shiori_resource.rs` の綴りが 3 か所。`documents.rs` の `read_text` は `mod.rs:199-201` と同じ中身。失敗の本文の形を 1 か所だけ直すと、判定ごとに出方がずれる。
- **直し**: 2 つの道具と綴りの定数を `documents.rs`（または `mod.rs`）に `pub(super)` で 1 つずつ置き、各ファイルからは `use` する。`documents.rs` の `read_text` は消して `super::read_text` を呼ぶ（-90 行）。
- **外への約束**: 守る（本文は同じ）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。 **C22**: `linkage_checks.rs`（944 行）が減る方向。

**49. 判定のテストの道具の写しが 4 ファイルにある**（`src/check/structure_tests.rs:41-86,666-671`・`src/check/freshness_tests.rs:60-89`・`src/check/finding_tests.rs:30-37`・`src/check/content_test_support.rs:24-94`）（仮 C6）
- **何か／困る場面**: `kinds`・`of_kind`・`only_one`・`details`・`places`・`id`・`ledger_place` の同じ中身が 4 か所。`content_test_support.rs:3-6` 自身が「両方に写すと、片方だけが本文の変更に追随しなくなる」と書いているのに、structure と freshness が写している。
- **直し**: `content_test_support.rs` を `check/test_support.rs` として `check/mod.rs` の `#[cfg(test)] mod test_support;` に置き、4 ファイルから引く（-70 行）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

#### 使われないコード（delete／yagni）

**50. 所見の種類 `LedgerDomainMismatch` はどこからも作られない**（`src/check/finding.rs:61-65,77,107,128`）（仮 C10）
- **何か／困る場面**: 「他の手段で組み立てた `Ledger` への二重の備え」と書いてあるが、作るコードがツリーのどこにも無い。README `:574` は出うる所見としてこれを載せ、利用者は出うるものと読み続ける。`every_kind_of_finding_is_absent_from_real_data` のこの種類の 0 件は必ず真。
- **直し**: 種類・綴り・`ALL` から消し、`finding_tests` の 2 つの逐語の並び（247・277 行）と 15 という数、README の表の 1 行を直す。
- **外への約束**: 変える（所見の種類の一覧が 14 種になる＝README の表。出る所見と版面の並びは変わらない）。
- **範囲の外へ**: `doc/ukadoc-coverage/README.md:574`。
- **残す候補**: ⑴ に当たりうる（完了 toolkit の design.md:802・:822 の判定表が 15 種で、design-validation の提案で足した経緯がある）。

**51. `ScanStats` の件数の欄 12 個は、本番でも整合テストでも誰も読まない**（`src/check/mod.rs:67-106,161-214`）（仮 C2）
- **何か／困る場面**: `run` が毎回ページ数・カテゴリ数・項目数・割り当て数・証拠のパス数・報告数などを数えて返すが、cli の `examine` が読むのは `findings` だけ（`inspect.rs:101-105`）、整合テストが読むのは `judgements_run` だけ（`checks.rs:161`）。見本を 1 件変えるたびに `finding_tests.rs:443-477` の数字を書き直すことになる。
- **直し**: `ScanStats` を消し（**52** と一緒に `judgements_run` も）、`finding_tests.rs:416-477` の 2 本と `freshness_tests.rs:141-142,554-558` の件数の主張を消す（-110 行）。
- **外への約束**: なし（端末の文にも書き出す中身にも出ない）。 **範囲の外へ**: なし。 **残す候補**: 該当なし（design.md:798 の `CheckOutcome { findings, stats }` の形は残る）。

**52. 入口の配線を確かめる足場（`judgements_run`・`Judgement.name`・`run_with`・関数ポインタの比べ）は役目を終えている**（`src/check/mod.rs:15-18,108-159`・`src/check/finding_tests.rs:329-410`・`tests/consistency/checks.rs:160-164`）（仮 C4）
- **何か／困る場面**: 「3 判定が所見を 1 件も返さない間は、これでしか呼び忘れを見分けられない」前提の表・名前・代役の仕組みと 3 本のテスト。今は各判定が所見を返し、全テストと摂動が `run` を通すので、呼び忘れれば必ず赤になる。`the_judgement_table_holds_the_three_real_functions` は関数ポインタの番地を比べている（Rust の文書が当てにならないとする比べ方）。
- **直し**: `run` を「3 つを順に呼んで足す」3 行にし、`JUDGEMENTS`・`Judgement`・`run_with` と 3 本のテストと `checks.rs:160-164` の主張を消す（-120 行）。
- **外への約束**: 守る（整合テストの合否は変わらない）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**53. `EntryId::has_anchor` をテスト以外のどこも使っていない**（`src/model.rs:184-187`）（仮 B15）
- **直し**: 関数と、`model_tests.rs:19・28・37・47・51`・`build_tests.rs:261・268` の assert を消す（-11 行）。
- **困る場面**: 呼ばれない公開の関数とそのテストを守り続けることになる。
- **外への約束**: なし。 **範囲の外へ**: 完了 toolkit の design.md:613 に名前の行がある（直すかは任意）。 **残す候補**: ⑴ に近い（完了 spec の設計に載った名前。要件ではない）。

**54. 使われない `Subjects::Zero` と、その腕（`#[expect(dead_code)]`）**（`tests/consistency/checks.rs:220-236`・`:333-346`）（仮 E8）
- **何か／困る場面**: 「新しい判定を足す人のために」残した列挙。今は 1 度も作られず、match の `Zero` の腕も届かない。数え上げの行は全部 `NonEmpty`。
- **直し**: 列挙と `expected` の欄と `Zero` の腕を消し、`count > 0` だけを主張する（-25 行）。
- **外への約束**: なし。 **範囲の外へ**: なし。
- **残す候補**: ⑵ に当たりうる（注記に残す理由が書いてある・`#[expect(dead_code)]`）。

**55. 読んで確かめるだけで、どこも使わない欄を型に持っている**（`src/documents/mod.rs:315-331,364,134`）（仮 B16）
- **何か／困る場面**: `Template` の `shiori`・`url`・`fetched_on`・`files`・`mapping`・`fallback`、`SpecRow` の `wave`、`NamedBundle` の `name`（使うのは `derive_tests.rs:69` だけ）を、在中テストのほかに読む所が無い。値を持ち回る意味が無い。
- **直し**: 読み手では `string_field(...)?;` で確かめて値は捨て、型から欄を消す（-10 行）。
- **外への約束**: 守る（読み手が落とす条件は変えない）。 **範囲の外へ**: なし。 **残す候補**: 該当なし（設計 D-2 の欄の一覧は読み手の検査として残る）。

**56. 使われていない壊す道具 `drop_bundle_name`**（`tests/consistency/documents.rs:283-300`）（仮 F6）
- **何か／困る場面**: 束名を書いた行を 1 行落とす道具。呼ぶのは `documents_tools.rs` の確かめ（228-237・294-298・357）だけで、`briefing_checks.rs:95` は「そのために使えない」と書いて別の写しを使う。
- **直し**: 道具と確かめ 2 本と 357 行目、95 行目の注釈を消し、冒頭の「道具 5 つ」を「4 つ」に（-40 行）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**57. `lf_body` は `strip_cr` の別名で、それを確かめるテストは同じ式を 2 度書いているだけ**（`src/io/files.rs:27-34`・`src/io/files_tests.rs:81-92`）（仮 A12）
- **困る場面**: 名前が 2 つあるので、読む人は「何か違う整形があるのでは」と探す。テスト `lf_body_and_read_normalized_agree_on_shape` は定義どおりなので必ず通る。
- **直し**: `lf_body` を消し、`files.rs:52` で `strip_cr` を呼ぶ。テスト 2 本を消し `files.rs:8` の説明を直す（コード -8・テスト -12 行）。
- **外への約束**: なし。 **範囲の外へ**: なし（completed の検証記録にテスト名が残るが、ただの記録）。 **残す候補**: 該当なし。

**58. 読み手がすでに拒む形を、腕の中でもう一度確かめている（届かない枝）**（`tests/consistency/linkage_checks.rs:270-277`・`:452-456`）（仮 E5）
- **何か／困る場面**: 腕 c の「hand の id が members に無い」と腕 f の「`[tally.singles_by_domain]` に欄が無い」。`read_linkage` が先に `parse.rs:305-312`・`:349-351` で止めるので、読めた `Linkage` ではこの 2 つの枝に 1 度も届かない。摂動にも、この 2 つを壊すものは無い。
- **直し**: 2 つの枝を消す（-12 行）。`None` の腕は `unwrap_or(0)` か `expect` に。
- **外への約束**: なし（読めたデータの上では結果が同じ）。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**59. 腕 d（除外の状態）は腕 b の写しで、同じ壊れ方を 2 件で名指す**（`tests/consistency/linkage_checks.rs:229-253` 対 `:294-312`）（仮 E10）
- **何か／困る場面**: 腕 b は構成 id が対象 4 状態でなければ「状態が alias」などと所見を出し、腕 d はそのうち alias・not-applicable だけをもう一度出す。束に alias の id を 1 つ混ぜると、同じ 1 か所について 2 行出る。腕 d だけが赤になる壊れ方は無い。
- **直し**: 腕 d と、その較正（`smuggling_one_excluded_id_...`・`an_excluded_id`・`EXCLUDED_STATUSES`）を消し（-50 行）、要件 4.3 を指す文言は腕 b の本文に移す。
- **外への約束**: 変える（失敗の本文の件数。落ちる条件そのものは同じ）。 **範囲の外へ**: なし。
- **残す候補**: ⑴ に当たりうる（完了 `areka-P0-ukadoc-coverage-roadmap` の設計の「判定の一覧」⑶ が a〜g の 7 腕を挙げている）。

#### 何も縛らないテスト・言い直しのテスト

**60. `finding_tests.rs` の見本の節が、判定と同じ決まりをテストの中にもう一度書いている。1 本は自分自身と比べている**（`src/check/finding_tests.rs:479-850`）（仮 C3）
- **何か／困る場面**: 判定がまだ空だった頃（タスク 4.1）の足場で、「台帳の id はカタログにある」「厳密な昇順」「実装済みに証拠」ほか 10 の決まりをテストのコードで手書きし直している。`the_four_domain_reports_match_what_the_ledgers_produce`（747-766）は World の報告が `render_domain(ledger, &THEMES)` から作られる（`lib_test_support.rs:449-452`）ので、同じ関数の出力どうしを比べているだけ。件数も content_tests と二重に固定している。判定の決まりを変えるとテスト側の写しも直さないと赤になる。780-850 行の World の道具の 3 本も、各判定のテストが道具を通して等式で見ているので重なる。
- **直し**: 479-778 行と 780-850 行を消す。ほかで守っていない 2 本（605-612 行＝`system.year` の証拠が語彙表のファイルから来る・624-649 行＝URL の無い ukadoc 行を数える）は残すか evidence の在中テストへ移す（-300 行）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**61. 「空にすると赤」の 6 本は、下限の器を空で試し直しているだけ**（`tests/consistency/documents_non_vacuity.rs:371-431,468-517,544-576,605-641,672-714,740-770`・`EMPTY_*` は 170-233）（仮 F5）
- **何か／困る場面**: 各判定の下限を、`Floors` に 0 か空の文書から読んだ値を渡して赤になることを確かめる。本物のテスト（`_has_something_to_judge`）の配線は一度も通らず、たとえば 459-463 で別の数を渡すように壊しても 6 本は緑のまま。下限 0 を拒むことは `Floors::short`（`documents.rs:370-375`）が本物のテストの中で見ており、器そのものの確かめは `documents_tools.rs:29-78` にある。6 本だけが持つのは「実数と下限が等しければ緑」（423-430）の 1 点だけ。
- **直し**: 6 本と `EMPTY_*` の 3 つを消し、`documents_tools.rs` の `the_floor_container_stays_quiet_when_every_row_is_met` に `at_least("ちょうど", 1, 1)` を 1 行（-310 行）。
- **外への約束**: なし（要件 11.4 は `_has_something_to_judge` が満たす）。 **範囲の外へ**: なし。 **残す候補**: 該当なし（完了 spec の tasks.md がこの 6 本を名指しているかは未確認＝11 節）。

**62. 報告のテストの多くが、同じ見本の全文一致のテストの中に丸ごと含まれている**（`src/report/domain_tests.rs:217-264,268-285,315-338,351-436,467-546`・`src/report/summary_tests.rs:274-298,302-311,319-366,381-551,614-651,668-730`）（仮 R9）
- **何か／困る場面**: `sample()` と `THEMES` だけを渡して本文の一部を主張するテストが domain に約 14 本、summary に約 15 本。同じ入力で `the_body_is_pinned_verbatim` が全文をバイトで固定するので、一部だけ壊れて全文一致が緑のまま、は起きない。summary の時刻の検出器とその較正（274-298・341-349）、`contains_line` とその較正（668-684）はこれらを支えるためだけにある。版面を変えるたびに同じ数字を 2〜3 か所で直すことになる。
- **直し**: 別の見本を組むテスト（`a`／`a0` の並び・参照先の無い別名・空の一覧・引数だけのテーマ・`hub` の 2 本の辺・shiori の見出し・カタログの時刻の差し替え・ドメインを跨がない束）は残し、`sample()` だけのものを消す（domain -230・summary -330 行）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし（失敗したときの文が詳しいという得は失われる＝設計で扱う）。

**63. `diff_tests` の 11 本は、同じ入力・同じ答えを一部ずつ言い直しているだけ**（`src/diff_tests.rs:387-415,431-543,595-663,710-729`）（仮 C7）
- **何か／困る場面**: `two_sample_catalogs_split_into_four_separate_lists`（382-385）が 4 つの一覧を丸ごと等式で比べ、ほかの 11 本も同じ `diff(&current(), &next(), &ledgers())` でその一部を見直す。実装をどう壊してもこのテストと同時にしか赤にならない（436-469 の注記自身が認めている）。
- **直し**: 11 本を消し、見本の形の前提（16 桁のうち先頭 15 桁が同じ・ハッシュだけ違う行・ハッシュだけ同じ行）は 1 本の「見本の形」テストに集める（-200 行）。入力の違うテストは残す。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**64. 必ず通るテスト `the_real_perturbation_target_appears_only_as_itself`**（`tests/consistency/documents_checks.rs:756-773`）（仮 F4）
- **何か／困る場面**: `a_twistable_id`（729-739）は「ほかのどの id にも接頭辞として現れない id」しか返さないので、絞り込みの結果は必ず的 1 つで、このテストは落ちようがない。名前は「3 文書」と言うが見ているのは `linkage.md` だけ。巻き添えは `twist_id` の確かめ（`documents.rs:148-155`）と「赤がちょうど 1 件」（449-454）が捕まえている。
- **直し**: テストを消す（-18 行・`BTreeMap` の import も）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**65. 「壊す道具は repo のファイルに触れない」テストは落ちようがなく、更新時刻の比べで揺れうる**（`tests/consistency/documents_tools.rs:312-400`）（仮 F8）
- **何か／困る場面**: 道具を動かす前と後で 4 つのファイルの中身と更新時刻を比べる。道具はどれも `&str` を受けて `String` を返すだけ（`documents.rs:136-312`）。一方で、テスト中に同じワークツリーで `report-summary` を回したり `git checkout` したりすると、更新時刻の比べで赤になる（要件 4.2 の決定論）。
- **直し**: テストと `fingerprint` と `PathBuf`・`SystemTime`・`paths` の import を消す（-90 行）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**66. ほかのテストの言い直しになっている小さなテストの群**（各テストファイル）（仮 A17・B18・C5・E6・E7・E9）
- **何か／困る場面**: 純粋な関数を 2 度呼んで比べるだけのもの・逐語のテストから必ず従う性質を言い直すだけのもの。消しても、どの壊れ方も見逃さない。
  - `src/hash_tests.rs:110-115` `same_input_gives_same_hash`
  - `src/tomlout_tests.rs:366-386` `every_function_is_byte_identical_on_a_second_call`
  - `src/io/files_tests.rs:44-52`・`:72-77`（28-36 の逐語で足りる）
  - `src/io/paths_tests.rs:151-200` の 3 本（67-147 の逐語から従う）
  - `src/io/snapshot_tests.rs:294-300`（`error_tests.rs:6-24` と同じ主張）
  - `src/cli/cli_tests.rs:31-47`・`:202-213`（49-67 と 180-200 で足りる）
  - `src/cli/inspect_tests.rs:141-166`（107-129 の逐語で足りる）
  - `src/assignment_tests.rs:184-200`（`BTreeMap` の鍵で形の上で交わらない＝決して赤にならない）・`:173-181`（158-170 と同じ）・`:118-148`（88-114 の逐語に含まれる）
  - `src/catalog/write_tests.rs:364-375`（write.rs を 1 行も通らない）・`:352-361`（201-203 に含まれる）
  - `src/documents/derive_tests.rs:587-593`・`src/documents/parse_tests.rs:571-578`（同じ引数で 2 回呼ぶだけ）
  - 「手を付けない見本は所見 0 件」の 4 本のうち 3 本（`finding_tests.rs:772-778`・`content_tests.rs:49-54`・`freshness_tests.rs:125-130`。`structure_tests.rs:146-152` を残す）
  - `tests/consistency/checks.rs:123-147` `every_kind_of_finding_is_absent_from_real_data`（上の一括の主張と合否が必ず同じ・読み込みが 1 回増える）
  - `tests/consistency/checks.rs:887-892`（`a_ledger_id_that_left_the_catalog_turns_red` の写し）
  - `tests/consistency/checks.rs:176-197`（`non_vacuity.rs:186-242`・`:393-442` の弱い写し）
  - あわせて直す 1 行（仮 A18）: `src/cli/generate_tests.rs:461` の `&plans[0].body[..len.min(80)]` は、主張が赤になった瞬間に文字の途中のバイト（生成された前置きの 2 行目は 43 バイト目から 3 バイトの文字が並ぶ）で切って panic し、本来の説明を隠す＝`.chars().take(80).collect::<String>()` にする。
- **直し**: 上を消す（合わせて -300 行ほど）・A18 の 1 行を直す。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

**67. `cli_streams` で、同じ失敗の走行を 2 本のテストが別々に起こしている**（`tests/cli_streams.rs:133-165`・`:245-265`）（仮 R10）
- **何か／困る場面**: `catalog`＋存在しないスナップショットの走行を 2 本がそれぞれ同じ 5 行の `Command` で起こし、実行ファイルを 2 回起動している。
- **直し**: 1 本にまとめ、走行の前後で木を見比べたうえで終了コード・標準出力・標準エラーの 3 つを確かめる（-15 行）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

#### 読み手を誤らせる注記

**68. 注記が今のコードと食い違い、守りを実際と違って読ませる**（下の各所）（仮 R2・R3・D7・A16 の一部・C11 の一部）
- **何か／困る場面**:
  - `tests/cli_streams.rs:22-31,101-103,142-144`: 「環境変数の指定は飾り（まだ中身が繋がっていない）」「8 つの副手続き」。今の `catalog` は環境変数を読んでそこで失敗する（`cli/generate.rs:55-58`・`io/snapshot.rs:90-100`）。飾りだと信じて指定を外すと、本物のスナップショットを読みに行く。
  - `src/cli/generate.rs:131-134`: 「手で記入した日から、再実行の主張がこの差し替えを赤にする」。そのテスト（`generate_tests.rs:566`）は `ledger_init_with` を直に呼ぶので、`ledger_init()` の 1 行の差し替えは今も捕まらない。
  - `src/ledger/blocks.rs:34-37`: 並びの検査は `ledger::read` でもすると書くが、`read` は並びを見ない（見るのは `write::check_strictly_ascending` と `check::structure::check_file_order`）。`priority-apply` の経路も守られていると読み違える。
  - `src/check/content.rs:170-172`: 「相手がどの台帳にも無ければ LinkEndpointMissing」。実際に出るのは `CatalogIdMissingFromLedgers`（`structure.rs:170-176`）。
  - `src/check/finding_tests.rs:5-8,16-21,335-337,367-369`・`src/check/mod.rs:145-147`: 「いま 3 つの判定はどれも所見を返さない」（今は返す）。
  - `src/report/mod.rs:2-3`: 「中身は後続タスクが埋める」（4 つとも出来上がっている）。
- **直し**: 該当の文を今の事実に 1 行ずつ直すか消す（コードは変わらない）。
- **外への約束**: なし。 **範囲の外へ**: なし。 **残す候補**: 該当なし。

### できれば（Nice to have）

**69. 走査のテスト 6 本が、それぞれ `crates/` を読み直す**（`src/io/sources_tests.rs:16-18`）（仮 A8）
- 1 回の `cargo test` で約 16,000 回のファイル読み。`static WALKED: OnceLock<Vec<(String, String)>>` で 1 度だけ走査する（3 行）。外への約束: なし。

**70. 台帳の本文を TOML として 2 度読んでいる**（`src/ledger/read.rs:95-101`・`src/ledger/blocks.rs:153-156`）（仮 D9）
- `read` が本文全体を読み、`split` の中の `calibrate` がもう 1 度読む（16,500 行の shiori.toml を `check` と整合テストのたびに 2 度）。`split` に読んだ表を渡す形に分ける（5 行）。外への約束: なし。

**71. `prologue_pages` は `pages_of` をそのまま返すだけの包み**（`src/cli/generate.rs:331-344`）（仮 A11）
- 名前順を守るのは包みでなく `generate_tests.rs:552-604` のバイト比較。`generate.rs:105` を `assignment.pages_of(domain)` にし、理由は 1 行の注釈、テスト 3 本（191・219・237）を直す（-14 行）。外への約束: なし。

**72. 制御文字の逃がしを手書きの 16 進の変換でやっている**（`src/tomlout.rs:168-185`）（仮 A13）
- `write!(out, "\\u{:04X}", u32::from(ch))` の 1 行にして 2 関数を消す（-15 行）。対象は U+007F 以下なので 4 桁で足りる。外への約束: 守る（綴りは 1 バイトも変わらない・付録 A.3・逐語のテスト `tomlout_tests.rs:135-144` が守る）。

**73. 走査の `dedup` は何もしない**（`src/io/sources.rs:53`）（仮 A14）
- `read_dir` は同じ項目を 2 度返さず、相対パスは親ごとに一意。1 行消す。外への約束: なし。

**74. `document_file` を `parse.rs` から素通しで再輸出している**（`src/documents/parse.rs:43-47`・`src/documents/fields.rs:7-8,19`）（仮 B9）
- fields.rs が parse.rs から `DOCUMENT_DIR` を借り、parse.rs が fields.rs から道具を借りる行き来の依存にもなっている。`DOCUMENT_DIR`・`LINKAGE_FILE`・`BRIEFING_FILE` を fields.rs へ移し、derive.rs は `super::fields::document_file` を引く（-8 行）。外への約束: なし。

**75. id の頭の綴り `ukadoc:` を 3 通りに書いている**（`src/documents/parse.rs:63・115`・`src/model.rs:138`）（仮 B19）
- `ID_PREFIX`・`ID_MARK`・生の `"\"ukadoc:"`。`ID_MARK` を model.rs に置いて共有し、`quoted_ids` は `format!("\"{ID_MARK}")` を使う。外への約束: なし。

**76. テーマ別の分布の足し合わせを、件数の数だけ 1 件ずつ回している**（`src/report/summary.rs:213-226`）（仮 R7）
- `for _ in 0..*count { target.add(*status) }` の三重の繰り返し。`StatusCounts::merge`（3 行）を足して 1 行に。外への約束: なし。

**77. 全体報告の冒頭の表の列名「未分類」を字面で書いている**（`src/report/summary.rs:104`・説明 47-49）（仮 R8）
- `format!` で `Status::Unclassified.as_japanese()` を埋め込み、注意書きを消す（-3 行）。外への約束: なし（今の綴りは同じ）。

**78. values_md がカタログの件数 1,749 を 2 本目の場所で釘付けしている**（`tests/consistency/values_md.rs:190-206`・`:217-221`）（仮 E11）
- 読み戻しの較正が言いたいのは「書いた数と読めた数が同じ」だけ。比べる相手を `data.catalog.entries.len()` にし、定数と最初の assert を消す（-8 行・件数の釘は `non_vacuity.rs` の 1 本が残す）。外への約束: 守る。

**79. 注記に古い実測・経緯が残っていて、今のデータと食い違う**（下の各所）（仮 E12・F11・A16 の残り・C11 の残り・R12・D12・B18 の注記・F10）
- `tests/consistency/checks.rs:500-503`（census の 6.5 は今は NonEmpty）・`non_vacuity.rs:5-9,42,101`（実測 1,089 本→今 2,557 本・「初期台帳は全行が未分類」・覆った要件 7.6 の理由）・`linkage_checks.rs:41-47`（target 1552 ほか→実 1550・1351・172）・`spec_checks.rs:32-43`・`documents_checks.rs:62-74`（「briefing の引用 id は 0 件」と `documents_non_vacuity.rs:28-30` の下限 100 が食い違う・1,553 は引き下げ前の数）・`documents_tools.rs:326-338`・`src/cli/generate.rs:89-91,129-130`・`generate_tests.rs:7-18,252-255,689-691`（「台帳は今すべて未分類」→実際は 1,749 項目すべてに状態・「束も 0 行」→67 本）・`src/io/paths_tests.rs:165`（覆った要件 7.6）・`src/tomlout.rs:58`・`src/check/freshness_tests.rs:486,513-514,539`（当時のテスト本数）・`tests/consistency.rs:8-13`（兄弟のモジュールを 5 つしか挙げない＝13 ある）・`src/evidence/candidates.rs:26-30,43-45,50-51,201-204`・`candidates_tests.rs:322`・`extract_tests.rs:40-42`（行番号つきのパスはファイルを触るたびにずれる）・`src/catalog/write_tests.rs:236`（「冒頭 13 行」→コードは 14）・`src/cli/inspect.rs:36-37`（完了 spec のタスク 8.1 を指す先送りの文・追う spec が無い）。
- 読み係 F の挙げた `tests/consistency/documents.rs:68-76` の読む順（completed を後に読むので、移す途中の spec がどちらにも無く見える隙間）もここで直す＝`completed_specs` を先に読む（2 行の入れ替え・spec_checks の腕 b は和集合で見るので害が消える）。
- 2026-11-17 `audit-emo2-boot-spine` からの申し送り（向こうの research.md 6 節「ケ」）: `candidates.rs:26-30`・`:42-46` が書く `crates/areka-runtime` の `consumer_ledger.rs` の `register(` の呼び出し数は今すでに古く、向こうの指摘 57（台帳の登録の重複の直し）が着地するとさらに変わる＝数え直さず、数を書かない形に直す（着地の順に縛られない）。
- **直し**: 日付つきの実測の段落を消し、今の事実の 1 行に直す（ほかの持ち場の変更で動く数は書かない）。行番号つきのパスから行番号を消す（パスそのものは `input-crate-split` が動かすときにその場で直す）。
- **外への約束**: なし（`documents.rs` の読む順の入れ替えも合否の条件は同じ）。
- **範囲の外へ**: なし（`candidates.rs` の説明の中のパスは、`input-crate-split` が `crates/areka/src/input_events/balloon.rs` を動かすときに向こうで直す）。
- **残す候補**: `src/cli/inspect.rs:36-37` は ⑵ に当たりうる（わざと置いた先送りの文）。ほかは該当なし。

## 6. 確かめて指摘にしなかったもの

- **書き出しの順**: `catalog`・`ledger_init_with`・`priority_apply_with`・`report_summary` は、読みと組み立てを全部済ませてから書く（`report` だけが違う＝**6**）。
- **終了コードと出力の分け方**: 結果は標準出力、断り・失敗・使い方は標準エラー、0／1／2 の写しは `main.rs:14-25` で合っている（パイプが閉じたときだけ例外＝**22**）。
- **根の解決**: コンパイル時の `CARGO_MANIFEST_DIR` の 2 段上で、実行時の環境変数は読まない（`io/paths.rs:29-47`）。`tests/cli_streams.rs` の子の走行も根はこれで決まり、環境変数を渡すのは存在しない固定の綴りを指すときだけ＝開発者の機械のスナップショットや `doc/ukadoc-coverage/` を書き替える経路は無い。範囲のテストは一時フォルダを 1 つも使わない（要件 4.3）。GPU を使うテストも無い（要件 4.4）。
- **決定論**: `HashMap` は 0 件。表は全部 `BTreeMap`／`BTreeSet`、所見は種類→場所→id→詳細の順に並べ直す。`freshness` は保存側だけ復帰文字を落としてバイトで比べる。`diff` は鍵の順。
- **`.kiro/specs/` の直下の新しいフォルダ**: 腕 b（`spec_checks.rs:154-171`）は表に書いた名前の実在だけを見て、総数とは比べない（`adding_another_spec_directory_stays_green_...` が確かめている）。ワークツリーは別々なので、ほかのセッションが作ったフォルダは見えない。揺れうるのは同じワークツリーの中の **1** だけ。
- **判定の二重書き**: 判定の中身は `check` と `tests/consistency/` に二重には無い（整合テストは `check::run` を呼ぶ）。導出は `derive_bundle` をそのまま呼ぶ。`Recount`・`OwnerFacts`・`elements_by_line`・`basic_string` の期待式は、わざと別の道筋で数え直す較正で、src に同じ関数は無い。
- **報告の数え方**: 割合と丸めは出していない（全部件数）。全体の分布とドメイン別は同じ `status_counts` を通るので和が合う。数え方の食い違いは **29** だけ。
- **摂動**: `expect_exactly` は（種類, id, 場所）の全体を等しいかで比べるので、所見を 1 件も出さない摂動は赤になる。緑のまま素通りする摂動は見つからなかった（偽の赤に寄りかかるものが **10**・**11**）。
- **スナップショットの欠けた欄**: `io/snapshot.rs` が entry の番号と欄の名前を挙げて落とす。BOM の扱いは無いが、今のファイルに BOM は無い。シンボリックリンクは 0 件で、たどらない。UTF-8 でない `.rs` はパスを添えて失敗する。
- **範囲の外の観察（指摘にしない）**: `crates/areka/src/menu/captions.rs:87-94` の `UNQUERIED_POPUPMENU_RESOURCES`（「問い合わせない」名前の表）にページ URL の目印が付いていて、要素名が「実装の証拠」として集まる＝範囲の外の持ち物（見直しの担当 spec の材料）。

## 7. 要件との対応表（Requirement-to-Asset Map）

| 要件 | 今ある物 | ギャップ |
|---|---|---|
| 1 洗い出し（範囲だけ・測り直し・上限なし・全部読む・番号・外への約束） | この文書（2 節・5 節・12 節） | なし。**Constraint**: 外への約束を変える 28 件の旗（5 節） |
| 2 直す対象の確定・残す 3 種類 | 5 節の「残す候補」 | **Unknown**: 9 件（10 節の議題）。⑶ に当たるものは 0 件の見立て（15・20 は 11 節で ukadoc を引いて確かめる） |
| 3 全部直す・呼び元まで | 範囲の外の呼び手は README・スキル・steering・データだけ（4 節） | **Constraint**: 範囲の外に及ぶのは **50**（README）・**1**（`kiro-complete` の注意・任意）・**35**（`catalog.toml` の作り直しが要る場合）の 3 件 |
| 4 テストで縛る・決定論・`target` の下・実 GPU | 在中テスト＋常時テスト | **Missing**: 直すバグ・危ない所（1〜30）の 1 件ごとのテスト。揺れの種 **65** を消す。一時フォルダと GPU は使わない |
| 5 1,000 行・記録の無い失敗の道を作らない・依存なし | 900 行以上が 2 本 | **Constraint**: テストを足す先（**34** を先に・`linkage_checks.rs` は減らす方向）。**5** の `LazyLock` は標準ライブラリ（依存なし） |
| 6 範囲の外・C22 | C22 の約束（roadmap C22 の行） | **Constraint**: 6.4「外への約束を変えない」と、外への約束を変える 28 件の扱い（10 節の議題 1） |
| 7 かけ直し・全体テスト・6.3／6.4 の確かめ | — | **Missing**: 副手続きの出力の突き合わせの手順（`main` の道具と同じ入力で `report`・`report-summary`・`ledger-init`・`check`・`evidence`・`candidates` を回して比べる）は設計で決める |

## 8. 直し方の選択肢

- **案 A（今のファイルの中で直す）**: 79 件をそれぞれ今のファイルの中で直す。写しの統合先は既にある親（`cli/mod.rs`・`ledger/mod.rs`・`check/mod.rs`・`documents/fields.rs`・`tests/consistency/{mod,documents}.rs`）に置く。
  - ✅ 新しいファイルが要らない・構造（純粋層／入出力層）がそのまま。❌ `generate_tests.rs`（948 行）にはテストを足せない。
- **案 B（新しいファイルを足して寄せる）**: Rust の本文の読み手（**9**）・囲みの走査（**13**）・判定のテストの道具（**49**）・整合テストの helper（**48**）をそれぞれ新しいファイルに置く。
  - ✅ 写しの置き場がはっきりする。❌ ファイルが 4〜5 本増え、モジュールの一覧と注記の直しが増える（ponytail の「いちばん小さな変更」から遠い）。
- **案 C（混ぜる）**: 案 A を基本にし、新しいファイルは仕事で分ける必要があるものだけ（**34** の `generate_priority_tests.rs`、**49** の `check/test_support.rs`＝今の `content_test_support.rs` の改名）。Rust の本文の読み手は `candidates.rs` 側に 1 つ残して `resolve.rs` から使う（新しいファイルなし）。
  - ✅ 1,000 行の番人を避けつつファイルの増加は最小。❌ `candidates.rs`（595 行）が読み手の持ち主になり、`resolve.rs` との依存の向きが 1 本増える。
- タスクの切り方の見立て（設計で決める）: ファイルのまとまりごと＝⑴ 書き出しと入口（2・6・22・27・31・32・34・37・38・57・71・72・73・69）⑵ 台帳（7・24・25・26・70）⑶ 証拠（3・8・9・23）⑷ カタログと型（15・19・20・40・53・75）⑸ 3 文書（13・14・16・17・18・28・33・39・41・42・55・74）⑹ 判定（4・30・49・50・51・52・60・63）⑺ 報告（29・45・46・62・76・77）⑻ 常時テストの読み込みと偽の赤（1・5・10・11・12・35・36・47・48）⑼ 常時テストの痩せ（43・44・54・56・58・59・61・64・65・66・67・78）⑽ 注記（68・79）＋検証の段（かけ直し・6.3／6.4 の突き合わせ・全体テスト）。おおよそ 16〜19 タスク。

## 9. 規模とリスク

- **規模: L（1〜2 週）**。指摘は 79 件と多いが、ほとんどが範囲の中の小さな直しと削除（消せる行はおよそ 2,400 行＝テストが大半・本番は 300〜400 行）。足す本番のコードは数十行、足すテストは 25〜30 本。
- **リスク: 中**。依存は足さず、構造も変えない。ただし ⑴ 外への約束を変える直しが 28 件あり、C22 の要件 6.4 との兼ね合いを先に決める必要がある。⑵ 常時テストの痩せ（60〜67）は、消す前に「同じ壊れ方を捕まえる別のテストがある」ことを 1 件ずつ確かめ直す必要がある。⑶ **5** の `LazyLock` 化は、失敗の本文の出方（最初の 1 本だけに理由が出る）を保つ書き方が要る。

Lean: -2,400 lines（おおよそ）, -0 dependencies possible.

## 10. 設計への申し送り・議題の候補（要件の討議の材料）

1. **C22 の「外への約束を変えない」（要件 6.4）と、変える直し 28 件の扱い**。内訳: ⑴ 今のデータでは出方が 1 バイトも変わらず、形の崩れた入力でだけ落ち方・文言が変わるもの 22 件（7・8・12〜21・23〜28・30・35・50・59。23 と 35 は 11 節の確かめしだいで ⑶ へ移る）、⑵ 偽の赤を消すもの 3 件（1・10・11）、⑶ 今のデータで出方が変わるもの 3 件（3＝`candidates` の標準出力・4＝`check` の終了コードと標準出力・22＝パイプが閉じたときの終了コード）。書き出すファイルの中身が変わるものは 0 件。案: ⑴⑵ は 6.4 の「変えない」に当たらないと読む（今の入力からの出力は同じ）か、要件 6.5 の議題として 1 件ずつ裁定するか。⑶ の 3 件は 6.5 の議題（C22 の中で直す／ほかの 5 本の着地を待つ／後へ回す）。
2. **4（`check` と常時テストの `summary.md`）**: 残す候補 ⑴（開発者裁定 2026-09-02 議題 2・完了 toolkit の要件 7.6）。ただし 7.6 の除外は統合調査の要件 11.2 で既に覆っている。直すなら `check` の終了コードが変わる＝議題 1 の ⑶。
3. **1（brief の無いフォルダで赤）**: 残す候補 ⑵（わざと置いた「0 件の写真」）と、`areka-P0-ukadoc-coverage-roadmap` 要件 11.1 ⑸・`kiro-next` の前提とのぶつかり。直した後に `kiro-complete` の `SKILL.md:243`・`:457`・`:651` の注意を緩めるか（範囲の外・任意）。
4. **50（`LedgerDomainMismatch` を消す）**: 残す候補 ⑴（完了 toolkit の設計の 15 種の判定表）。消すなら README `:574` を同じ変更で直す（要件 6.6）。
5. **59（腕 d を消す）**: 残す候補 ⑴（完了 coverage-roadmap の設計の「判定の一覧」⑶ の 7 腕）。
6. **20（連番を数字に絞る）**: 残す候補 ⑴（完了 toolkit の要件 1.9 の読み方を狭める）。ukadoc の id の形に数字以外の連番がありえないかを確かめてから（11 節）。
7. **54（`Subjects::Zero`・`#[expect(dead_code)]`）・79 の `inspect.rs:36-37`**: 残す候補 ⑵（わざと置いた印）。
8. **53（`has_anchor`）**: 完了 spec の設計に名前があるだけで要件ではない＝⑴ に当たらないと読むか。
9. **35（`catalog.toml` の 1 バイトの安定を常時テストで縛る）**: 今の `catalog.toml` が `write(read(t)) == t` を満たさなければ、作り直しが要件 6.4 と `ukadoc-catalog-refresh` の持ち物に触れる＝先に確かめる。
10. **62・60・61・63・66（大きなテストの削除・合わせて約 1,400 行）**: 失敗したときの文が詳しいという得を手放すことになる。全部消すか、版面を逐語で固定するテスト 1 本＋別の見本のテストだけを残す形でよいか（推す＝消す。逐語のテストが同じ壊れ方を全部捕まえる）。
11. **29（テーマの重複）**: 数え方で畳むか、`ledger::read` で書き間違いとして拒むか（拒むと整合テストの落ちる条件が増える＝議題 1 の ⑴）。
12. **5（実データの読み込みを 1 回に）**: 読み込みが失敗したとき、2 本目以降のテストは「poisoned」で落ちる（本当の理由は最初の 1 本にだけ出る）＝この出方でよいか。

## 11. 調べが要る点（Research Needed）

- **35**: 今の `doc/ukadoc-coverage/catalog.toml` が `catalog::write::write(&read(t)) == t` を満たすか（実装の段の最初に `cargo test` 1 本で確かめる）。
- **15・20**: ukadoc（SSP 公式仕様書）で、同じ URL を持つ 2 つの項目・数字でない連番が正当にありうるか（ukadoc の MCP で引き、出どころと引用を添える＝要件 2.3）。今のスナップショットでは URL は全部相異なり（設計 D-4 の実測）、連番は 1,730 件とも数字の見込み（数え直していない）。
- **23**: 作業ツリーのテストのファイルに、引用符を奇数個持つ生文字列が今あるか（あれば今の `candidates` の出力も変わる＝議題 1 の ⑶ に移る）。
- **61**: 完了 spec（`areka-P0-ukadoc-coverage-roadmap`）の tasks.md・design.md が「空にすると赤」の 6 本を名指しているか（名指していれば議題 5 と同じ ⑴ の候補）。
- **5**: `RepoData`・`Documents` の全部の欄が `Sync` か（`Rc`・`RefCell`・`Cell` の grep は 0 件・コンパイルでは未確認）。所要時間の実測（今の `cargo test -p ukadoc-survey` の時間）は、直しの前後で 1 回ずつ測る（排他テストの机で）。

## 12. 読み方の記録（全部読んだことの証跡・要件 1.4）

96 ファイルを 7 つの群に分け、読み係（サブエージェント）7 本が群ごとに全部を頭から終わりまで読んだ（大きいファイルは分けて最後まで）。どの群も「読んだファイル」の表で 96 本すべてに「はい」を返した。まとめ役（このギャップ分析）は重なりを畳み、主な指摘（1・2・3・4・5・6・10・11・22・35）の行をコードで読み直して確かめた。

| 群 | 読んだファイル（本数） | 出した仮の指摘 | この文書での番号 |
|---|---|---|---|
| A 入口・入出力・共通 | 25 | A1〜A19 | 2・6・21・22・27・31・32・34・36・37・38・57・66・68・69・71・72・73・79 ほか（A18＝下の注） |
| B 型・割り当て・カタログ・3 文書 | 17 | B1〜B19 | 13〜20・28・33・39〜42・53・55・66・74・75・79 |
| C 整合の判定・差分 | 13 | C1〜C11 | 4・30・41・49〜52・60・63・66・68・79 |
| D 台帳・証拠 | 16 | D1〜D12 | 3・7・8・9・23〜26・41・68・70・79 |
| R 報告・テストの入口 | 14 | R1〜R12 | 5・29・45〜47・62・67・68・76・77・79 |
| E 整合テスト ⑴ | 5（＋参照） | E1〜E12 | 5・10・11・48・54・58・59・66・78・79 |
| F 整合テスト ⑵ | 6（＋参照） | F1〜F12 | 1・5・12・43・44・48・56・61・64・65・79 |

- 注: 仮 A18（テストの失敗の文の小さな直し）は **66** の中の 1 項目として扱う（番号は振らない）。
- 畳んだ重なり: 実データの読み直し（E1・R1・F3→**5**）、台帳の場所の綴り（C8・B8・D10→**41**）、整合テストの helper（E4・F9→**48**）、言い直しのテスト（A17・B18・C5・E6・E7・E9→**66**）、注記（R2・R3・D7・A16・C11→**68**、E12・F11・A16・C11・R12・D12・B18・F10→**79**）。
