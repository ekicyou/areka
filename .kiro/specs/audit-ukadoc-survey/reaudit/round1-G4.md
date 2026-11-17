# かけ直し 1 回目・G4（整合テスト）

範囲は tests/ の下の 16 ファイル・7,465 行。全部の行を読んだ。指摘 19 件（必ず直す 0・直したほうがよい 2・できれば 17）。どれも残す 3 種類に当たらない。ruling-conflicts 無し。

| id | 重さ | file:line | 何が悪いか | 直し | 振る舞い |
|---|---|---|---|---|---|
| G4-1 | 直したほうがよい | tests/consistency/briefing_checks.rs:223-230・284-287 | 較正の前に「実データに second-stage の override が 0 行」「insufficient の行が 0 行」を assert する。正しい行が足されると偽の赤（元の指摘 11 と同じ形） | 2 つの assert を消す。briefing_arms.rs:40-43・502-505 とテスト名の注記（217・277 行の「実データ 0 行」）の件数の文も削る | はい |
| G4-2 | 直したほうがよい | tests/consistency/briefing_checks.rs:406-410 | 「段階 B の rank 1 が 2 行以上」を実データに要求する。同着が 1 行になっただけで偽の赤 | 写しの briefing に rank 1 の行をもう 1 本足して 2 行の形を写しの上で作り、実データの行数は見ない。briefing_arms.rs:678-679 の「実データでは…複数の行」も削る | はい |
| G4-3 | できれば | tests/consistency/non_vacuity.rs:41 | 全体報告を読むのは mod.rs の RepoData で documents.rs ではない | 「読み込みは mod.rs の RepoData」に直す | いいえ |
| G4-4 | できれば | tests/consistency/perturb.rs:28-29 | the_check_survives_lines_moving が「ちょうど 1 件」を主張すると書くが 0 件を主張している | その名前を挙げの列から外す | いいえ |
| G4-5 | できれば | tests/consistency/perturb.rs:5 | 使う側は checks.rs だけと書くが documents_checks.rs も使う | 「checks.rs と documents_checks.rs の摂動」に直す | いいえ |
| G4-6 | できれば | tests/consistency/documents_non_vacuity.rs:9-10・101-105・284-286 | 「3 文書が引用した機械の束 id」と名乗るが数えるのは linkage.md だけ | 「linkage.md が引用した機械の束 id」に直す | いいえ |
| G4-7 | できれば | tests/cli_streams.rs:4-7 | 標準出力の選び分けが main.rs にあると書くが、標準出力へ書くのは cli の側 | 「標準エラーへの写しと終了コードの対応表は main.rs」に絞る | いいえ |
| G4-8 | できれば | tests/consistency/mod.rs:29-31（cli_streams.rs:38-39 も） | 一時ディレクトリを使わない理由（log-capture-kit の例外表・要件 9.1）が古い | 理由の文を消して「読むだけ（要件 4.3）」にする | いいえ |
| G4-9 | できれば | tests/consistency/non_vacuity.rs:87 | SELF_CRATE_DIR を「crate 内公開」と書くが非公開の const | 「非公開」に直す | いいえ |
| G4-10 | できれば | tests/consistency/documents.rs:1-2・187、documents_tools.rs:1・15 | 「1 か所だけ壊す道具」—twist_id は全部の出現を差し替える | 「狙った綴り・1 つの欄だけを壊す道具」に言い直す | いいえ |
| G4-11 | できれば | tests/consistency/documents_checks.rs:49-50 | 文末の句点が無く文が途切れる | 「。」を足す | いいえ |
| G4-12 | できれば | tests/consistency/documents_checks.rs:282 | text.contains("report.md") が部分一致で gap-report.md などで偽の赤 | ファイル名の境界で見る（前の 1 文字が `/`・`` ` ``・空白・行頭のときだけ） | はい |
| G4-13 | できれば | tests/consistency/examples.rs:279・33 | 「上の 4 本」—実は 5 本。33 行の「読むのは…だけ」も外れている | 数を書かない形に。33 行は「RepoData の一式と要件文書・README」に | いいえ |
| G4-14 | できれば | tests/consistency/values_md.rs:28-42・468-478 | 来歴が 2 か所に重ねて書いてある | 来歴は 1 文に縮めて片方を消す | いいえ |
| G4-15 | できれば | tests/consistency/checks.rs:32-34・116 | 32-34 は確かめられない過去の言い切り。116 の「（指摘 35）」は spec 完了後に読めない番号 | 32-34 を消す。116 は要件の言葉（カタログの読み書きが往復で変わらない）に | いいえ |
| G4-16 | できれば | tests/consistency/non_vacuity.rs:319-322・373-376・408・437-441 | どれも後の主張に含まれる重複の assert | 4 つとも消す | いいえ |
| G4-17 | できれば | tests/consistency/documents.rs:393-397、briefing_arms.rs:175-176 | Floors::new() は Default を呼ぶだけの包み。ledger_file の中継の再輸出 | new を消して Floors::default() に。使う 2 ファイルが ukadoc_survey::ledger::ledger_file を直に引く | いいえ |
| G4-18 | できれば | linkage_checks.rs:76-129・briefing_arms.rs:98-144・spec_checks.rs:60-112 | 3 ファイルが台帳 4 本を畳む型をそれぞれ持つ（2 つは同名の LedgerFacts で中身が違う） | briefing_arms の LedgerFacts に id と owner を足して 3 か所で共有し、ほかの畳みを消す | いいえ |
| G4-19 | できれば | linkage_checks.rs:740・briefing_checks.rs:452 | 見出し「摂動の的（選び方も較正する）」—選び方を較正するテストは無い | 見出しを「摂動の的」に縮める | いいえ |

小さな気づき（数えない）: cli_streams.rs:115-117・217-219 の文字列が生の改行を含み 59 行の `\n` と書き方がそろわない／perturb.rs:129 の `id.clone()` は要らない／linkage_checks.rs:847 のテストが手助けの関数の節の後ろにある。

読んだファイル（16 本すべて全行・はい）: tests/cli_streams.rs 231／tests/consistency.rs 13／mod.rs 207／briefing_arms.rs 746／briefing_checks.rs 532／checks.rs 808／documents.rs 445／documents_checks.rs 640／documents_non_vacuity.rs 400／documents_tools.rs 313／examples.rs 327／linkage_checks.rs 861／non_vacuity.rs 442／perturb.rs 247／spec_checks.rs 638／values_md.rs 615。
