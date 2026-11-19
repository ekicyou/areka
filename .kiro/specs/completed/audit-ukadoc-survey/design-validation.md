# 設計の検証: audit-ukadoc-survey

- 日付: 2026-11-17
- 対象: `design.md`（421 行）・`requirements.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`）
- やり方: `kiro-validate-design` の手順（分析 → 重大な問題 → よい所 → GO／NO-GO）。対話はせず、この報告を設計の討議の材料にする。
- 確かめ方: 設計が書くコードの事実（ファイルと行・呼び手・残るテストの名前）を、今の作業ツリーのコードで突き合わせた。cargo は回していない（読んだ結果と `git grep` の結果だけ）。

## 要約

79 件すべてに直し方の行があり（番号 1〜79 が 1 行ずつ・抜けも重なりも 0）、要件の討議の 7 つの裁定はどれも設計に反映されている。範囲の外で触るのは裁定が名指した `doc/ukadoc-coverage/README.md` と `.claude/skills/kiro-complete/SKILL.md` だけで、C22 の約束のファイルにも触らない。ただし指摘 4（`check` が `summary.md` の古さも見る）の直しが、整合テストの摂動のテスト（`tests/consistency/checks.rs`・`perturb.rs`）に及ぶことが設計に書かれていない。このまま実装すると W15 で数本の整合テストが赤になる。

## 分析

### 79 件の行と 7 つの裁定

- 「指摘ごとの直し方」の表は 79 行で、番号 1〜79 がちょうど 1 つずつある。research.md 5 節の見出しも 79 件で、番号はそろっている。
- 裁定 1（書き出す中身を 1 バイトも変えない・28 件はこの spec で直す）: 「外への約束の変わり方」の表と W18 の手順 ⑶ の突き合わせに入っている。
- 裁定 2（指摘 4 を直す）: 行 4 と W15。
- 裁定 3（指摘 1 を直す・`kiro-complete/SKILL.md` の 243・457・651 行を直す）: 行 1 と W16。3 か所の文は実在し、どれも消すテストだけを理由にしている。ツリー全体でこのテストを名指すのは、この 2 行（243・457）と 651〜653 行の症状の塊だけだった。`kiro-next/SKILL.md:114` は直しの後に記述どおりになる。
- 裁定 4（50・53・59 を消す・README の表を直す）: 行 50・53・59。README の 565 行「15 種」と 574 行の `LedgerDomainMismatch` の行は実在する。
- 裁定 5（54 と `inspect.rs:36-37`）: 行 54 と行 79。
- 裁定 6（20・15 を直す）: 行 20・15。
- 裁定 7（大きなテストの削除は、同じ壊れ方を捕まえる残るテストを名指してから）: 「消すテストと残るテスト」の表。確かめで代わりが無かった `the_usage_text_lists_every_name_in_the_dispatch_table` を残すのは、裁定の条件どおり。

### コードとの突き合わせ（12 か所）

| # | 設計の主張 | 結果 |
|---|---|---|
| 1 | `documents_non_vacuity.rs:330-368` が組み直し・brief 無し 0 件・completed の brief を確かめる | 合う。`live ^ completed` は 321-328 行で、消す範囲の外 |
| 1 | `kiro-complete/SKILL.md` の 243・457・651〜653 行 | 合う。ほかのスキル・手引き・steering には 0 件 |
| 2 | `write_lf` は `std::fs::write`（`files.rs:51-53`）・呼び手は `generate.rs` の 64・138・180・201・215 の 5 か所 | 合う。今の `write_lf_reports_the_path_it_could_not_write` は「パスに `out.md` が入る」を見るので、`out.md.tmp` でも通る |
| 4 | `freshness.rs:19-26` の注記・`summary_findings`（`documents_checks.rs:335`）・`the_summary_report_is_as_fresh_as_the_ledgers`（420）・`shifting_one_number_…`（514）・`real_repo_data_produces_no_findings`（`checks.rs:116`） | 合う |
| 4 | `documents.summary_text` の読み手は 8 か所 | 数が違う。今は 12 か所（`documents_checks` 6・`documents_non_vacuity` 2・`documents_tools` 1・`linkage_checks` 3）。W15 の触るファイルの一覧には 4 本とも入っているので、作業の境は変わらない |
| 5 | `RepoData::load`（`mod.rs:136-175`）・`spec_checks.rs:340` の `repo: RepoData`・`examples.rs` の `RepoData::load().catalog` 3 か所（115・137・284） | 合う。読み込んだ値を書き換える呼び手は 0 件。ただし同じ `Fixture` の 341 行 `documents: Documents` も `&'static` に直す必要があり、設計に書かれていない（コンパイルで分かる） |
| 22 | `print!` が `inspect.rs` の 85・114・124・139 行・`announce` の `println!` が `generate.rs:377`・`write_notice` が `cli/mod.rs:126` | 合う |
| 34 | `generate_tests.rs` は 948 行・`priority-apply` の節は 681 行から・`LEDGER_FILES`（262）・`spelled_path`（320）・`under_coverage`（329）・A18 は 461 行のバイトの切り出し | 合う |
| 35 | `catalog::write::write(&Catalog) -> String` が公開（`write.rs:53`） | 合う |
| 51・52 | `stats`・`JUDGEMENTS`・`run_with` を使うのは `finding_tests`・`freshness_tests`（141-142・554-558）・`checks.rs:161` だけ | 合う |
| 消すテストの表 | 残るテストの名前（`structure_tests` の 2 本・`content_tests` の 2 本・`extract_tests`・`freshness_tests::the_sample_world_holds_four_non_empty_domain_reports`・`domain_tests`／`summary_tests` の `the_body_is_pinned_verbatim`・`diff_tests::two_sample_catalogs_split_into_four_separate_lists`・`strip_cr_*` の 3 本・`parse_reports_broken_json_with_the_absolute_path`・`error_tests` の 1 本・`inspect_tests` の逐語・`assignment_tests` の `*_requirement_3_1` 4 本と `source_table_…`・`write_tests` の 2 本・`derive_tests` の序列・`a_bundle_with_an_unfinished_member`） | 全部実在する。`two_sample_catalogs_…` は 4 つの一覧を丸ごと等式で見ており、消す 11 本の主張を含む。`domain_tests` の逐語は同じ `render_domain(&sample(), &THEMES)` を全文で固定している。`a_one_character_change_in_a_source_url_is_a_finding` は `content_tests` にある（表は所属を書いていないだけ） |
| 55 | `Template` の 6 欄・`SpecRow::wave` を読む所は在中テストのほかに無い | 合う。`tests/consistency/` からの参照は 0 件 |

### 1,000 行の番人

- 伸びる側のファイルはどれも 1,000 行から十分遠い（`resolve_tests.rs` 778・`structure_tests.rs` 781・`parse.rs` 755・`freshness_tests.rs` 569 など）。
- `checks.rs`（893 行）は W1 で 35 の 1 本を足すと 900 行を少し越える見込みで、設計の「900 行を越えない」は言い過ぎ。ただ 1,000 行には届かない。下の問題 1 の直しで W15 にも数行増えるが、W12 と W17 で減るので、番人は守れる。
- `generate_tests.rs` は W1 で分けてから足すので越えない（34）。

### まとまり（W1〜W18）の順

- 矢印から各まとまりの前に済むまとまりを数え、触るファイルが重なる組を全部調べた。表に書かれた触るファイルの範囲では、重なる組はどれも矢印の道でつながっている（例: W3 と W4 の `io/paths.rs`・W5 と W6 の `catalog/build_tests.rs`・W6 と W8 の `cli/generate.rs`・W11 と W15 の `content_link_tests.rs`・W12 と W14 と W15 の `check/mod.rs` と整合テスト）。
- 矢印の無い組（W9・W13 とほかの大半、W4〜W7 どうし、W10 と W4〜W9・W11〜W15 など）は、表の範囲ではファイルが重ならない。
- 問題 1 の直しで W15 に `checks.rs`・`perturb.rs` が加わっても、W1・W12・W14 の後・W17 の前なので、今の矢印のままで順は保たれる。

### 範囲の外

- 範囲の外で触るのは README と `kiro-complete/SKILL.md` と本 spec の記録だけ。ロードマップの C22 の行の「触るファイル」①〜④⑥ に、この 2 本は入っていない。
- 新しく足す `generate_priority_tests.rs` と改名先 `check/test_support.rs` を名指しで見張る仕組みは、`crates/log-capture-kit/tests/` には無い。

## 重大な問題

### 問題 1: 指摘 4 の直しが、整合テストの摂動のテストに及ぶことが書かれていない

- **何が問題か**: 4 の直しで freshness は `summary_report` を `render_summary(catalog, ledgers, themes)` と比べる。`tests/consistency/checks.rs` の摂動のテストは、台帳の写しを書き換えて `expect_exactly` で「所見の顔ぶれがちょうどこれ」を見る。台帳の中身が変わると全体報告も変わるので、`an_id_in_two_ledgers_turns_red`（421 行）・`an_alias_chain_turns_red`（624 行）・登場版のテスト（698 行）・`a_misspelled_theme_turns_red`（736 行）などに、`doc/ukadoc-coverage/report/summary.md` の `DomainReportStale` が 1 件ずつ増えて赤になる。ところが W15 の触るファイルに `checks.rs` も `perturb.rs` も入っていない。さらに、行 47 は `Perturbed::input` を `..self.data.input()` にして `summary_report` を本物から流すと書き、行 4 は `shifting_one_number_…` を「`Perturbed` の写しの `summary_report` を壊す」形へ改めると書く。本物から流すと写しで壊せないので、2 つの行が食い違っている。あわせて README の 584 行（`DomainReportStale` の直し方「`report` を走らせ直す」）と 437-438 行（ドメイン別 4 本だけの説明）は、全体報告の場所の所見には合わなくなる（要件 6.6）。
- **影響**: W15 の終わりで整合テストが数本赤になり、直し方をその場で決めることになる。直し方を取り違えると、摂動のテストの「ちょうど」の主張を緩めて緑にする（要件 4.5 が禁じる形）おそれがある。
- **直し方の案**: ⑴ `Perturbed` に `summary_report: String` の欄を持たせ、`input` では写しの欄を渡す（行 47 の `..self.data.input()` は割り当て表とテーマ名だけに使う）。⑵ 台帳を書き換える摂動のテストの期待に、全体報告の場所の `DomainReportStale` を 1 行ずつ足す（「ちょうど」は保ったまま。巻き添えの所見を名指しで固定する今の作法と同じ）。⑶ 在中の `lib_test_support.rs` の `refresh_reports` が全体報告も作り直すことと、`CheckInput` を手で組む在中テスト（`content_link_tests.rs`・`structure_tests.rs`・`freshness_tests.rs`・`finding_tests.rs`）が見本に合う `summary_report` を渡すことを、行 4 に書く。⑷ freshness の全体報告の所見の本文は `report-summary` を名指し、README の 437-438 行と 584 行を同じ変更で直す（「外への約束の変わり方」の表の 4 の行の「直しなし」を改める）。⑸ W15 の触るファイルに `tests/consistency/{checks,perturb}.rs` を足す。
- **要件との対応**: 4.1・4.5・6.6・3.2
- **設計の根拠の場所**: 「指摘ごとの直し方」の行 4 と行 47・「Modified Files」の W15 の行・「外への約束の変わり方」の 4 の行

### 問題 2: 指摘 6 の `report_with` と指摘 38 の `load_ledgers()` の形が合わず、台帳 4 本を読む写しが残る

- **何が問題か**: 行 6 は `report_with(read, write)` で読み手を引数で受ける形にする。行 38 は `cli/mod.rs` に引数の無い `load_ledgers()`（中で `files::read_normalized` を呼ぶ）を置く。読み手を受ける `report_with` は引数の無い `load_ledgers()` を呼べないので、`report_with` の中に台帳 4 本を読む繰り返しをもう 1 本書くことになる。今ある `priority_apply_with`（250-255 行）の同じ繰り返しと合わせて、38 が消したい写しが 2 本残る。research.md の指摘 6 は「`load_ledgers()` を使う」と書いており、設計とずれている。
- **影響**: W18 の audit のかけ直しで同じ指摘が出て、直しがもう 1 回まわる見込みが高い（要件 7.2）。
- **直し方の案**: `cli/mod.rs` に読み手を受ける `load_ledgers_with(read: &dyn Fn(&Path) -> Result<String, SurveyError>)` を置き、`load_ledgers()` はそれに `files::read_normalized` を渡す 1 行にする。`report_with` はこれを呼ぶ。`priority_apply_with` は台帳の本文も手元に残す必要があるので今の繰り返しのままでよいが、残す理由を 1 行の注記にする。
- **要件との対応**: 3.1・7.2
- **設計の根拠の場所**: 「共有の形」の `cli/mod.rs` と `cli/generate.rs` の節・行 6 と行 38

## 軽い食い違い（討議を待たずに設計の文を直せばよいもの）

- 行 4 の「`documents.summary_text` の読み手 8 か所」は今 12 か所（ファイルは 4 本で、W15 の一覧と合う）。
- 行 5 は `spec_checks.rs:341` の `documents: Documents` も `&'static Documents` に直すことを書いていない。
- Testing Strategy の「W1 のときの `checks.rs` は 900 行を越えない」は、893 行に 1 本足すので少し越える見込み（1,000 行には届かない）。
- 行 6 の `report()` は、今の書き出しの前の `ensure_parent` を書き手の側に残す（`ledger_init` と同じ形）ことを書いておくとよい。
- 行 50 で `LedgerDomainMismatch` を消すとき、名前を書いた注記が `structure.rs:32`・`structure_tests.rs:24`・`content_link_tests.rs:218`・`finding.rs:61` にもある。消す前の `git grep` の手順で拾えるが、行 50 に挙げておくと漏れない。
- 行番号の小さなずれ: `cli_tests.rs:202` は関数の行が 203（202 は属性の行）・README の「52 行」は 53 行。

## よい所

1. **消すテストの 1 本ごとに残るテストを名指している**。表の残るテストは全部実在し、読んだ範囲では消す側の主張を含んでいた。代わりが見つからなかった 1 本（使い方の文と振り分けの表をつなぐテスト）を、裁定 7 の条件に従って残すと決めているのも筋が通っている。
2. **順番と失敗の扱いが具体的**。W1 で `generate_tests.rs` を先に分ける理由（34）と 35 の確かめを先に回す理由、35 が崩れていたときに `catalog.toml` を作り直さず議題にする道、書き出しの置き換え（2）の事後条件、`LazyLock` の中で panic させない読み込み（5）が、どれも書き切ってある。79 の読む順を research.md の案どおりに入れ替えない理由（移す途中の spec がどちらにも見えない隙間ができる）も、今のコードの読む順（直下を先・`completed/` を後）と合っていた。

## 判定

- **GO**（設計の討議で問題 1・2 を直してからタスクの生成へ進む）
- **理由**: 79 件すべてに直し方があり、裁定 7 つ・範囲の境・1,000 行・まとまりの順はどれも守られている。問題 1 は指摘 4 の影響の書き漏れで、直す場所（`checks.rs`・`perturb.rs`・README）も直し方もはっきりしており、設計の骨組みは変わらない。問題 2 は関数の形を 1 つ合わせれば済む。
- **次の段**: 設計の討議（`kiro-design-discussion`）で問題 1・2 を議題にし、決めた直し方を design.md の行 4・6・38・47・W15 の行・「外への約束の変わり方」の 4 の行へ反映する。軽い食い違いは同じ機会に文を直す。その後 `/kiro-spec-tasks audit-ukadoc-survey`。
