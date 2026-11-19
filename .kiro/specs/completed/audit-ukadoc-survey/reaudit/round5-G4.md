# かけ直し 5 回目・G4

範囲は tests/ の下の 16 ファイル・7,417 行。全部の行を読んだ。指摘 2 件（必ず直す 0・直したほうがよい 0・できれば 2）。どちらも [test/doc] で、道具の振る舞い（副手続きの出力・整合テストの判定）は変えない。どちらも残す 3 種類に当たらない（開発者の裁定でも、正典を追う口でも、ukadoc の読み方でもない）。ruling-conflicts 無し。

4 回目の直し（2d1e2a247）の tests/ の差分（briefing_checks.rs:275-277・documents_checks.rs:348-351・406-407・examples.rs:47-51）も読んだ。直しが生んだ新しい食い違いは無い。

| id | 重さ | 種別 | file:line | 何が悪いか | 直し | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G4-1 | できれば | [test/doc] | tests/consistency/checks.rs:203・228 | 数え上げの行「7.4/7.5 ドメイン別報告」は `data.domain_reports.len()` を数えるが、`RepoData::read`（mod.rs:151-154）は `Domain::ALL` の 4 本を全部入れるか読めずに止まるかのどちらかなので、この数は常に 4 で 0 になりえない＝落ちようのない行である。non_vacuity.rs:383-384 は同じ数について「構造的に 4 であり、それ自体は何も言わない」と書いており、2 つのファイルが食い違う | その行（と 203 行の `reports`）を消す。ドメイン別報告の空振りは non_vacuity.rs の `the_four_domain_reports_exist_on_disk_and_are_not_empty` が 1 本ずつ見ている | いいえ | none |
| G4-2 | できれば | [test/doc] | tests/consistency/documents.rs:267-271 | 「実データではどちらも起きる——…`linkage.md` の『数の突き合わせ』は恒等式 `target = from_machine + by_hand + singles` を地の文に書く」は誤り。実際の linkage.md の地の文は ``1. `target` = `from_machine` + `by_hand` + `singles` `` と鍵を 1 つずつ逆引用符で囲んで書いており、錨の綴り `target = `・`singles = ` は linkage.md の本文全体に 1 度ずつ（`[tally]` の中だけ）しか現れない。実データで起きるのは briefing.md の側（`singles = ` が段階の表に 5 回・`[[rank]]` に 3 回）だけである | 「`briefing.md` は…を 1 つずつ持つので、本文全体を錨にすると実データで止まる。`linkage.md` の恒等式は今は鍵を逆引用符で囲んで書いているので当たらないが、囲まずに書けば同じことが起きる」の形に直す（documents_tools.rs:196-198 の「書くだけで同じことが起きる」は仮定の形なので正しい） | いいえ | none |

確かめて指摘にしなかったもの（数えない）: checks.rs:226 の「6.10 状態の語彙を通る行」は 6.3 の行と同じ `ledger_rows` を数える重なりだが、要件ごとに名指す表の作り（要件番号で辿る）として意味が通るので指摘にしない／checks.rs:442-443 の `areka-kanade/src/schedule/events.rs` は `crates/` を省いた綴りだが、そのファイルの 129 行に `OnFirstBoot:1` の正典 URL があり実在する／perturb.rs:42-45 の錨の条件（implemented・`links = []`・`introduced = ""`）は shiori.toml:2862-2868 と一致／documents_checks.rs:80-83 の「状態の分布の表の 3 行目」と「4 桁目」は summary.md:21-30・123-125 と一致（ドメイン別の表の見出しの 4 桁目にも現れるが、行頭で絞るので注記の結論は変わらない）／documents_non_vacuity.rs:297-298 の「束 0 の linkage.md では a・c・e が緑」は腕 b（対象 4 状態の全項目が不在で赤）・f（`target` の数え直しが非 0）・g（恒等式 3）が赤になるので正しい／documents_non_vacuity.rs:76-81 の `MIN_LINKAGE_CITED_IDS = 1_550` は linkage.md の `target = 1550` と一致／spec_checks.rs:598-607 の数の目印は今の先頭行（items 2→3）では「台帳 4 本」の「4」と衝突しないので指摘にしない／mod.rs:61-66 の「[`checks`] の摂動が使う道具」は documents_checks.rs も使うが「checks だけ」とは言っていないので誤りではない／briefing_checks.rs:537-541 の選び方は書いてある `themes` の数を比べるが、腕 e（linkage_checks.rs:231-238）が書いてある値と導出の一致を守るので緑のデータの上では同じ値になる。

ruling-conflicts: 無し。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| tests/consistency.rs | 13 | はい（全行） |
| tests/cli_streams.rs | 226 | はい（全行） |
| tests/consistency/mod.rs | 205 | はい（全行） |
| tests/consistency/perturb.rs | 243 | はい（全行） |
| tests/consistency/checks.rs | 802 | はい（全行） |
| tests/consistency/documents.rs | 440 | はい（全行） |
| tests/consistency/documents_tools.rs | 313 | はい（全行） |
| tests/consistency/documents_non_vacuity.rs | 403 | はい（全行） |
| tests/consistency/documents_checks.rs | 708 | はい（全行） |
| tests/consistency/spec_checks.rs | 608 | はい（全行） |
| tests/consistency/briefing_arms.rs | 748 | はい（全行） |
| tests/consistency/briefing_checks.rs | 545 | はい（全行） |
| tests/consistency/linkage_checks.rs | 822 | はい（全行） |
| tests/consistency/examples.rs | 328 | はい（全行） |
| tests/consistency/non_vacuity.rs | 417 | はい（全行） |
| tests/consistency/values_md.rs | 596 | はい（全行） |
