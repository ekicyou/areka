# かけ直し 2 回目・G4

範囲は tests/ の下の 16 ファイル・7,427 行。全部の行を読んだ。指摘 13 件（必ず直す 0・直したほうがよい 0・できれば 13）。どれも注記・重複・余分な assert で、道具の振る舞い（副手続きの出力・整合テストの判定）は変えない。どれも残す 3 種類に当たらない（開発者の裁定でも、正典を追う口でも、ukadoc の読み方でもない）。ruling-conflicts 無し。

1 回目の直しが生んだもの: G4-2（G3 の直しで `slice_element_names` が `evidence/lex.rs` へ移ったのに注記が古い場所を指す）・G4-3（1 回目 G4-5 の直しが 5 行目だけで 75 行目に同じ食い違いが残る）・G4-12（1 回目 G4-15 の直しで括弧書きが本文の言い直しになった）。

| id | 重さ | file:line | 何が悪いか | 直し | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|
| G4-1 | できれば | tests/consistency/values_md.rs:16-20 | 鎖の 1 段目を「台帳のテーマ名 ∈ `CheckInput::themes`（`check::content` のテーマの判定）」と書くが、`check` にテーマ名の判定は無く、テーマ名を拒むのは `ledger::read` の `parse_theme`（`model::THEMES` と完全一致・checks.rs:649-650 の注記どおり） | 1 段目を「台帳のテーマ名 ∈ `THEMES`（`ledger::read` の `parse_theme`）」に直し、2 段目（`CheckInput::themes` == `THEMES`）は報告の突き合わせ相手の話なので鎖から外す | いいえ | none |
| G4-2 | できれば | tests/consistency/values_md.rs:48 | `evidence::candidates::slice_element_names` と書くが、関数は `evidence/lex.rs:28` にある（candidates は呼ぶ側） | `evidence::lex::slice_element_names` に直す | いいえ | none |
| G4-3 | できれば | tests/consistency/perturb.rs:75 | 「直に書き換える欄だけを兄弟（`checks.rs`）へ開ける」—`documents_checks.rs:434` も `summary_report` を直に書き換える | 「兄弟（`checks.rs`・`documents_checks.rs`）」に直す | いいえ | none |
| G4-4 | できれば | tests/consistency/perturb.rs:71-73 | 「台帳のファイルそのものを書き換える確認は人手で 1 度だけ行った」は今は確かめられない過去の言い切り（1 回目 G4-15 と同じ形） | その 1 文を消し、理由の後半（並行に走るテストが途中の状態を読む）だけを残す | いいえ | none |
| G4-5 | できれば | tests/consistency/documents_non_vacuity.rs:6-7 | 「読み込み（`Documents::load`）が 3 文書・全体報告・spec の一覧に届いている」—全体報告は `RepoData` が読む（`Documents` から `summary_text` は消えた・160-186 行も `repo.summary_report` を見る） | 「`Documents::load` が 3 文書と spec の一覧に、`RepoData::load` が全体報告に届いている」に直す | いいえ | none |
| G4-6 | できれば | tests/consistency/documents_non_vacuity.rs:83-86 | `MIN_BRIEFING_CITED_IDS` 自身の注記が「下の [`MIN_BRIEFING_CITED_IDS`] を使う腕が走るたびに」と自分を「下の」と呼び、使うのは腕ではなくテスト（258 行） | 「この下限を使うテスト（`the_citation_and_bundle_checks_have_something_to_judge`）が走るたびに `cited_ids` が数え直す」に直す | いいえ | none |
| G4-7 | できれば | tests/consistency/documents_non_vacuity.rs:145-147 | 「束の表は本文の 5 割強を占める」—今の `summary.md` では束の節が 34,311 バイト中 32,234 バイト（約 9 割）で、数が外れている | 数を書かず「束の表は本文の大半を占めるので」にする | いいえ | none |
| G4-8 | できれば | tests/consistency/briefing_arms.rs:34-36・tests/consistency/briefing_checks.rs:330 | `[[barrier]]` を「6 行」と行数で書く。行数はページの分け直しで増減する（documents_non_vacuity.rs:138 の注記と食い違う形の数の注記） | 「6 行」を「`[[barrier]]` の各行」に言い換え、数を書かない | いいえ | none |
| G4-9 | できれば | tests/consistency/documents_non_vacuity.rs:246-247 | `".kiro/specs"` と `"brief.md"` を手で書き直している。同じ綴りが documents.rs:26・32 の `SPECS_DIR`・`BRIEF_FILE` にあり、片方だけ直ると除外の確かめが別の場所を見る | 2 つの定数を `pub(super)` にしてここで使う | いいえ | none |
| G4-10 | できれば | tests/consistency/documents_checks.rs:177-190・tests/consistency/linkage_checks.rs:117-132 | 「報告 5 本（summary＋ドメイン別 4 本）を歩く」が 2 か所に同じ panic 文ごと写してある（行の拾い方は共有しているのに、どの報告を歩くかが割れている） | `report_bundle_ids` を `machine_bundle_members(repo).into_keys().collect()` にし（`machine_bundle_members` を `pub(super)` で documents_checks.rs 側へ移すか linkage_checks.rs から引く）、歩き方を 1 か所にする | いいえ | none |
| G4-11 | できれば | tests/consistency/non_vacuity.rs:290-294・407-418 | 290-294 は直前の `insert` の assert と 286-289 の和の assert から必ず成り立つ。415-418 の「本文どうしが相異なる」は 410-414 で 1 行目（ドメイン名入りの見出し）が違うと確かめた後なので必ず成り立つ。どちらも赤になれない | 2 つの assert と `bodies` を消し、407-408 の注記を「見出しが自分のドメインを名乗る」に縮める | いいえ | none |
| G4-12 | できれば | tests/consistency/checks.rs:356 | 1 回目の直しで「読んで書き直すと 1 バイトも変わらないこと（カタログの読み書きが往復で変わらない）」と、括弧が本文の言い直しになった | 括弧書きを消す | いいえ | none |
| G4-13 | できれば | tests/consistency/examples.rs:264 | 失敗の本文「空だと この主張が空振りする」に余分な半角空白 | 「空だとこの主張が空振りする」に直す | いいえ | none |

確かめて指摘にしなかったもの（数えない）: checks.rs:584-589 の「台帳 2.7.26／カタログ 2.7.25 の形」は今も 3 件ともその形（shiori の `*.popupmenu.applybindtoself`）／documents_checks.rs:80 の「状態の分布の 3 行目」は今の `summary.md` で正しい／spec_checks.rs の後ろ 2 本の摂動に「摂動の前は緑」の assert が無いが、実データに所見があれば「ちょうど 1 件」が崩れて赤になるので見落としは無い／documents.rs の `shift_count` は表の見出しの直後の行が `[` で始まると隣の表まで範囲が伸びるが、今の 3 文書にその形は無く、`only_occurrence` が別の表の鍵を 2 度見れば止まる。

ruling-conflicts: 無し。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| tests/consistency.rs | 13 | はい（全行） |
| tests/cli_streams.rs | 226 | はい（全行） |
| tests/consistency/mod.rs | 205 | はい（全行） |
| tests/consistency/perturb.rs | 244 | はい（全行） |
| tests/consistency/checks.rs | 802 | はい（全行） |
| tests/consistency/documents.rs | 440 | はい（全行） |
| tests/consistency/documents_tools.rs | 313 | はい（全行） |
| tests/consistency/documents_non_vacuity.rs | 400 | はい（全行） |
| tests/consistency/documents_checks.rs | 689 | はい（全行） |
| tests/consistency/spec_checks.rs | 608 | はい（全行） |
| tests/consistency/briefing_arms.rs | 748 | はい（全行） |
| tests/consistency/briefing_checks.rs | 544 | はい（全行） |
| tests/consistency/linkage_checks.rs | 843 | はい（全行） |
| tests/consistency/examples.rs | 327 | はい（全行） |
| tests/consistency/non_vacuity.rs | 428 | はい（全行） |
| tests/consistency/values_md.rs | 597 | はい（全行） |
