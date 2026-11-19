# かけ直し 6 回目・G3

範囲は `crates/ukadoc-survey/` の `src/check/`・`src/evidence/`・`src/report/` の全ファイル（28 本・8,587 行）。ここは純粋な層で、カタログ・台帳・証拠・報告本文を値として受け取り、食い違いの一覧・証拠の索引・手掛かり候補・報告本文を返します（ファイルには触れません）。負荷は、開発者 1 人が副手続きや整合テストを回すだけと見ました。

読み手としてはファイルを 1 つも変えていません（この記録だけを書いた）。状態を変える git の操作もしていません。1〜5 回目（`round1-G3.md`〜`round5-G3.md`）の指摘は蒸し返していません。5 回目 G3-1 の ⒝（見出しと全体報告の 1 文を直して報告を作り直す）は tasks.md の Implementation Notes のとおり別 spec へ送られたので、挙げていません。

5 回目の直し（`fea70ff55`。G3 の範囲では `candidates_tests.rs` のテスト 1 本の追加・`resolve_tests.rs` のテスト名・`bundle.rs`・`bundle_tests.rs`・`domain.rs`・`summary.rs` の注記）を差分で読み、新しい問題を残していないことを確かめました。

- 足したテスト `空行と属性とコメントを挟んでも目印とテストの範囲は効く` の「飛ばす分岐のどれを消しても赤になる」は、分岐 5 つ（`has_marker_above` の `#[`・空行、`cfg_test_regions` の空行・`#[`・`//`）を 1 つずつ外した形を手で追い、どれも `version` か `OnTestOnly` が候補に出て赤になることを確かめた。肯定の側の `log_path` は、直上が空行の後のコード行（`MARKED`）なので正しく候補に残る。
- `domain.rs:145-149` の「入ってくる側の id が小さければ別の束 id に、そうでなければ同じ束 id のまま中身が増えて載る」は、束 id が構成 id の最小値であること（`bundle.rs:80`）から正しい。
- `bundle_tests.rs:57-58` と `140-141` は、向きを見ないことを背負うテストとして同じ 2 本を名指すようになった。

下の 2 件はどちらも完了 spec toolkit（`2350e68a9`）の頃からあるもので、1〜5 回目の直しで入ったものではありません。

## 指摘

| id | 重さ | 種別 | 場所 | 何が悪いか | 直し方 | 道具の振る舞いに触るか | 残す種類の候補 |
|---|---|---|---|---|---|---|---|
| G3-1 | できれば | [test/doc] | `src/evidence/candidates.rs:105-107`（`is_test_path` の `ends_with("_test.rs")`・`contains("/tests/")`）・`src/evidence/candidates_tests.rs:441-457` | テストの置き場を見分ける 6 つの条件のうち、`/tests/` と `_test.rs` の 2 つには、壊すと赤になるテストが無い。`candidates_tests.rs` が通すパスは `events_tests.rs`・`placement_tests/a.rs`・`tests.rs` の 3 つだけで、`cli/inspect_tests.rs` のパスも `crates/*/src/*.rs` だけ。例えば 107 行の `\|\| path.contains("/tests/")` を消すと、走査が拾う `crates/*/tests/*.rs`（今 320 本）の表やログ行がすべて候補に出るようになるが、全テストは緑のまま（候補の件数は常時のテストで守らないと決めてある・`candidates.rs:28`） | `テストの置き場の下と_tests_rs_も拾わない` の否定側のパスの列に `crates/x/tests/a.rs` と `crates/x/src/a_test.rs` を足す（肯定側の `crates/x/src/a.rs` はそのまま） | いいえ | なし |
| G3-2 | できれば | [prod] | `src/evidence/candidates.rs:108`（`\|\| path.starts_with("tests/")`） | 届かない枝。`candidates` の呼び手は `cli/inspect.rs:120` だけで、渡すのは `io::sources::walk` の出力、つまり根が `crates` の相対パス（`sources.rs:40,50`「`crates/**/*.rs`」）なので、`tests/` で始まるパスは来ない。リポジトリの根に `tests/` ディレクトリも無い。根の下の `tests/` は 107 行の `contains("/tests/")` が既に受け持つ | 108 行を消す（103 行の「`tests/` 配下」の言い方はそのままでよい。107 行が受け持つ）。副手続きの出力は変わらない。`candidates` を `tests/…` のパスで直に呼んだときの振る舞いだけが変わる | いいえ（副手続きの出力も整合テストの判定も変わらない。関数を直に呼んだときだけ変わる） | なし |

## 確かめて指摘にしなかったもの

- `content_tests.rs:337-354` の `an_entry_that_is_not_implemented_needs_no_evidence` は、所見の主張が `structure_tests::the_untouched_sample_world_has_no_findings` と同じ（`World::normal()` で 0 件）だが、「証拠の無い項目が 8 つある」という材料の主張を足しており、それが無いと 0 件の主張が「実装済みでない項目に証拠が無い」場面を試していることにならない。言い直しのテストとは言えないので指摘にしない。
- `content.rs`・`structure.rs` は台帳を渡された順に見るので、`run` が返す所見の並びは台帳の並びで変わる（`freshness.rs` だけは `Domain::ALL` の順に並べ直す）。ただし本文は `finding::render` が種類・場所・id・詳細で並べ直すので、副手続きの出力にも整合テストの判定にも漏れない。どちらの注記も「同じ入力なら同じ並び」と言うだけで嘘ではない。
- `summary_tests.rs:273` の「要件 7.3・11.2」の 11.2 は統合調査の spec（`areka-P0-ukadoc-coverage-roadmap`）の番号。crate の中では `generate.rs`・`documents/` など統合調査の要件番号を spec 名を添えずに引くのが慣わしで、11.2（`summary.md` を全文一致にする）は中身も合っている。
- `tally.rs:163` の「要件 6.10」は、toolkit の 6.10 が「テーマ名の綴りが違う」を赤にする条件に挙げているので合っている。
- `candidates.rs:197-198` が名指す `crates/areka-placement/src/source.rs` の `const SHELL_DPI_KEY: &str = "seriko.dpi";` と `config.rs` の `shell_kv.get("seriko.zorder")`、`extract_tests.rs:41-42` が名指す `source_tests.rs`・`decode_tests.rs` の行は、どれも今の木にある。
- `freshness.rs:25` の「この repo に `.gitattributes` は無く」は 3 回目に確かめて指摘にしなかったもの（根と `doc/` には無い）。蒸し返さない。
- `lex.rs` の `skip_char_literal` は、複数バイトの文字（`'あ'`）を文字リテラルと読まず 1 バイトずつ進めるが、閉じの `'` の後ろ 2 バイトが `'` になる並びは表の中に現れず、括弧の数えは狂わない。具体的な入力で困る場面が無い。
- `structure_tests.rs:118-119`・`content_tests.rs:62-108`・`content_link_tests.rs` の見本の件数と並び（`world.ledgers[1]` が assets、`[3]` が property）は、`Domain::ALL` の順（shiori・assets・sakura-script・property）と合っている。
- `tally_tests.rs:58-63` の見本の説明（ページ別 4 件・2 件、使う状態 5 つ、使うテーマ 3 つ）と `domain_tests.rs:85-93` の説明は、見本の行を数えて合っていた。

## ruling-conflicts

- なし。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/check/mod.rs | 79 | はい（指摘なし） |
| src/check/finding.rs | 201 | はい（指摘なし） |
| src/check/finding_tests.rs | 184 | はい（指摘なし） |
| src/check/content.rs | 252 | はい（指摘なし） |
| src/check/content_tests.rs | 354 | はい（指摘なし） |
| src/check/content_link_tests.rs | 653 | はい（指摘なし） |
| src/check/freshness.rs | 129 | はい（指摘なし） |
| src/check/freshness_tests.rs | 569 | はい（指摘なし） |
| src/check/structure.rs | 233 | はい（指摘なし） |
| src/check/structure_tests.rs | 738 | はい（指摘なし） |
| src/check/test_support.rs | 118 | はい（指摘なし） |
| src/evidence/mod.rs | 128 | はい（指摘なし） |
| src/evidence/extract.rs | 93 | はい（指摘なし） |
| src/evidence/extract_tests.rs | 313 | はい（指摘なし） |
| src/evidence/lex.rs | 266 | はい（指摘なし） |
| src/evidence/resolve.rs | 309 | はい（指摘なし） |
| src/evidence/resolve_tests.rs | 833 | はい（指摘なし） |
| src/evidence/candidates.rs | 385 | はい（G3-1・G3-2） |
| src/evidence/candidates_tests.rs | 475 | はい（G3-1 の足し先） |
| src/report/mod.rs | 8 | はい（指摘なし） |
| src/report/bundle.rs | 151 | はい（指摘なし） |
| src/report/bundle_tests.rs | 271 | はい（指摘なし） |
| src/report/domain.rs | 171 | はい（指摘なし） |
| src/report/domain_tests.rs | 396 | はい（指摘なし） |
| src/report/summary.rs | 233 | はい（指摘なし） |
| src/report/summary_tests.rs | 388 | はい（指摘なし） |
| src/report/tally.rs | 276 | はい（指摘なし） |
| src/report/tally_tests.rs | 381 | はい（指摘なし） |

Verdict: 必ず直すは 0 件、直したほうがよいも 0 件。残る 2 件は「できれば」で、テストの無い分岐 1 件（G3-1・[test/doc]）と届かない枝 1 件（G3-2・[prod] だが副手続きの出力は変わらない）。報告の本文・副手続きの出力・整合テストの判定に触る指摘は無い。
Lean: G3-2 で −1 行、G3-1 でテストのパス 2 つ（約 +2 行）。依存は −0。

確かめていないもの: テストは走らせていない（G3-1 の「消しても緑」は、`candidates_tests.rs`・`cli/inspect_tests.rs`・`resolve_tests.rs` の見本のパスを全部読んで、`/tests/` か `_test.rs` を含むものが 1 つも無いことで判じた）。外のファイルは、指摘を確かめるのに要る所だけ読んだ（`io/sources.rs:40-95`・`io/files.rs:25-56`・`cli/inspect.rs` の `candidates` の呼び出し・`cli/inspect_tests.rs:280-320`・`model.rs` の `LinkKind`・toolkit と統合調査の requirements.md の要件 6・7・11、`git ls-files` で `crates/*/tests/*.rs` の本数と `*_test.rs` の置き場）。
