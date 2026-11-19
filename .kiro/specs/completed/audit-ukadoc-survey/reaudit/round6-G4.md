# かけ直し 6 回目・G4

範囲は tests/ の下の 16 ファイル・7,417 行。全部の行を読んだ。指摘 2 件（必ず直す 0・直したほうがよい 0・できれば 2）。どちらも [test/doc] で、道具の振る舞い（副手続きの出力・整合テストの判定）は変えない。どちらも残す 3 種類に当たらない（開発者の裁定でも、正典を追う口でも、ukadoc の読み方でもない）。ruling-conflicts 無し。

5 回目の直し（fea70ff55）の tests/ の差分（checks.rs:227-228 の数え上げの行の削除と注記・documents.rs:267-272 の錨の注記）も読んだ。直しが生んだ新しい食い違いは無い。G4-1 は 5 回目 G4-2 と同じ doc コメントの続きの段落だが、別の文（表の範囲の終わり）についての指摘で、5 回目の直しを戻すものではない。

| id | 重さ | 種別 | file:line | 何が悪いか | 直し | 振る舞い | 残す種類 |
|---|---|---|---|---|---|---|---|
| G4-1 | できれば | [test/doc] | tests/consistency/documents.rs:274-276・280-285 | 「表の範囲は…次に行頭が `[` で始まる行の手前まで…囲みを閉じる ` ``` ` の行はこの範囲に入るが、そこに `鍵 = ` は現れない」は誤り。表が囲みの最後の表だと、範囲は閉じの ` ``` ` を越えて後ろの地の文と次の囲みの開きまで伸びる。実データで `briefing.md` の `[stage.E]`（943 行）の範囲は 983 行の `[[barrier]]` の手前まで＝948〜981 行の 4 節の地の文を丸ごと含み、`roadmap-draft.md` の `[briefs]`（620 行）の範囲も 664 行の `[[spec]]` の手前まで地の文を含む。今その地の文に `singles = `・`count = ` が逆引用符なしで現れないので通っているだけで、書けば摂動が「ちょうど 1 度現れない」で偽の赤になる（5 回目 G4-2 で直した「地の文に書けば同じことが起きる」が、表の中に絞ったはずのこの道具でも起きる） | 範囲の終わりを「次に行頭が `[` で始まる行」か「閉じの ` ``` ` の行」の早いほうにし、注記を「囲みの外へははみ出さない」に直す | いいえ | none |
| G4-2 | できれば | [test/doc] | tests/consistency/documents_tools.rs:194-229 | 「同じ鍵が別の表と地の文にも現れるとき…」と言うが、作った本文の地の文「恒等式は target = from_machine + by_hand + singles である。」は狙う鍵の綴り `singles = ` を含まず、しかも表の見出しより前にあるので範囲に入りようがない。3 つ目の assert（「地の文まで書き換えている」）はどんな実装でも落ちず、本文全体を錨にする壊し方は 2 つの表だけで捕まる＝地の文の腕は何も見ていない | 地の文を `[stage.B]` の囲みの**後ろ**に置き、`singles = 0` を逆引用符なしで書く形にする（G4-1 の直しの前なら赤・後なら緑になる較正になる）。少なくとも注記の「地の文にも現れる」を事実に合わせる | いいえ | none |

確かめて指摘にしなかったもの（数えない）: documents_non_vacuity.rs:246-247 の `SELF_CRATE_PREFIX` は non_vacuity.rs:96 に実在／checks.rs:649 の「8 つの綴り」と :651 の「気配」「気配り」は model.rs:112-121 の `THEMES` と一致／non_vacuity.rs:58-65 のカテゴリ 6 種の和は 1,749 で一致／linkage_checks.rs:22-24 の「`from_machine` が 1 多い」は、1 つ抜くと書いてある数が数え直しより 1 多くなるので正しい／linkage_checks.rs:381-383 の「腕 f が緑でも恒等式は自動では成り立たない」は、状態が 7 語あって恒等式 3 の左辺が 6 語しか数えないことと分割の主張から正しい／briefing_checks.rs:223-228 の override の較正は実データの override の行を 1 行書き換える形で、行が無ければ `expect` で大きな声で止まるので黙って通らない（briefing_arms.rs:40-42 の「写しに 1 行作って」は insufficient・singles・override の 3 つを合わせた言い方として通る）／briefing_checks.rs:199-201 の rank 2 を 3 にする摂動は、同着の rank 2 が他にもあれば所見が増えるが `any` で見るので正しい／values_md.rs:327-335 の `all` は目印が剥がれると空で成り立つが、目印の有無は :472 の `the_vocabulary_route_binds_items_on_todays_real_data` が見ているので指摘にしない／spec_checks.rs:99 の腕 b は brief の無いディレクトリを実在と見ないが、要件 10.2 の「brief 済み」どおり。

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
