# かけ直し 1 回目・G3（判定・証拠・報告）

この 3 つのフォルダは、カタログ・台帳・証拠・報告本文を受け取って食い違いの一覧や報告本文を返す純粋な層です（ファイルには触れません）。負荷は、開発者 1 人が副手続きや整合テストを回すだけと見ました。G3 の 27 ファイル（8,942 行）は最初の行から最後の行まで全部読みました。

読み手としてはファイルを 1 つも変えていません（この記録だけを書いた）。状態を変える git の操作もしていません。

## 指摘

| id | 重さ | 場所 | 何が悪いか | 直し方 | 道具の振る舞いに触るか | 残す種類の候補 |
|---|---|---|---|---|---|---|
| G3-1 | 直したほうがよい | `src/check/content.rs:242-269`・`src/check/finding.rs:89-90,110,131` | `UnknownTheme` は実データの経路では出せない。`ledger/read.rs:254-276` の `parse_theme` が同じ規則（8 テーマとの完全一致）で読み込みの段で先に落とすので、`values = ["気配り "]` と書いた台帳は所見ではなく読み込みの失敗になる。指摘 50 の `LedgerDomainMismatch` と同じ形の二重持ち。README の 286 行「他の綴りは `UnknownTheme` で失敗する」も事実と違う | どちらか 1 か所に寄せる。指摘 50 にそろえるなら `check_themes`・`UnknownTheme`（種類・綴り・`ALL`・14 種の数）・在中テスト 4 本・`tests/consistency/checks.rs:654-` の `a_misspelled_theme_turns_red`・README の 286 行と 584 行を消し、読み込みの段に一本化する | はい（実データでの出力は変わらないが、種類の一覧・README・整合テストが変わる） | なし。要件 6.8 は「整合検査で確かめる」と言うが、読み込みの失敗も整合テストを赤にするので満たす。開発者の裁定ではない |
| G3-2 | 直したほうがよい | `src/report/summary.rs:23-28,91` | 既知の項目で、まだ残っている。「判定 ⑹（本文の全文一致）」は今は `check::run` の `freshness` の仕事。「本 spec が覆している」「要件 11.2」はどの spec のことか読めない | 「`check::freshness` が本文を全文一致で見張る」と書き、spec 名（統合調査）を名指す | いいえ | なし |
| G3-3 | 直したほうがよい | `src/report/domain.rs:13` | 「証拠の有無は `report/summary.md` の担当である」は古い。全体報告は証拠を載せない（`summary.rs:21-28`） | 「証拠は `evidence` 副手続きの担当」に直す | いいえ | なし |
| G3-4 | 直したほうがよい | `src/report/bundle.rs:9-11` | 「ドメイン別報告は両端が自ドメインの links だけを渡す」は嘘。`domain.rs:157-184` は台帳の links を全部渡し、外へ伸びた束を丸ごと落とす（中身が違う。辺を絞るなら A—B—Z から {A,B} が残るが、実際は束ごと消える） | 「台帳の links を全部渡し、構成 id が全部その台帳にある束だけを残す」に直す | いいえ | なし |
| G3-5 | 直したほうがよい | `src/report/summary.rs:32-36` | 「付録 A.2 は alias_of／supersedes を専用の欄にだけ許すので、supersedes だけの対はどの報告にも出ない」は事実と違う。`ledger/read.rs:299` は links の `kind = "alias_of"`／`"supersedes"` を受け付け、実データにも `sakura-script.toml:521` ほかにあって、束の辺になる | 「`alias_of`・`supersedes` の欄は辺にしない。束に出したい対は `links` に書く（kind は 6 種のどれでもよい）」に直す | いいえ | なし |
| G3-6 | 直したほうがよい | `src/check/finding.rs:15` | 「この本文は実行ファイルの標準エラー出力でもある」は古い。`check` は標準出力に書く（`cli/inspect.rs:84,174-181`） | 「標準出力」に直す | いいえ | なし |
| G3-7 | 直したほうがよい | `src/check/finding.rs:91` | `DomainReportStale` の説明がドメイン別報告だけを言うが、`summary.md` の古さも同じ種類で出る（`freshness.rs:79-87`） | 「ドメイン別報告か全体報告が作り直した本文と一致しない」に直す | いいえ | なし |
| G3-8 | 直したほうがよい | `src/check/structure_tests.rs:16-21` | 冒頭の注記の「`PageNotAssigned` は `CatalogIdMissingFromLedgers` を必ず連れてくる」「この 2 つだけは 2 種類を明示」は、同じファイルの 255-291 行（担当違い＋前置き）と 537-565 行（`PageNotAssigned`＋担当違い）に反する | 「連れてくる相方は見本の組み方で変わる。道連れのあるテストは種類を全部明示する」に直す | いいえ | なし |
| G3-9 | 直したほうがよい | `src/check/freshness_tests.rs:448-449` | 文が壊れている：「要件 7.7 が禁じ要件 7.4・7.5 が捕まえる」 | 「要件 7.7 が禁じ、要件 7.4・7.5 が捕まえる」 | いいえ | なし |
| G3-10 | できれば | `evidence/extract_tests.rs:10,229`・`report/bundle_tests.rs:7,96,103,298`・`report/tally_tests.rs:8,393`・`report/domain_tests.rs:7-8,12-13,88,93,319`・`report/summary_tests.rs:7-9,104-105,109,288,309` | 古い完了 spec のタスク番号を引いている（「タスク 1.6 の教訓」など）。特に `domain_tests.rs:12` の「新しさの検査（タスク 4.4）」は部品の名前の代わりにタスク番号を使っている | 括弧のタスク番号を消し、教訓の文だけを残す。部品は `check::freshness` と名指す | いいえ | なし |
| G3-11 | できれば | `src/evidence/resolve.rs:13-15` | 理由が筋違い。完全一致で 1 件に定まる理由は「接頭辞の例が無い」ことではなく、同じ URL を 2 項目が持たないこと（`catalog/build.rs:58-78` が拒む＝指摘 15） | 「同じ URL の 2 項目はカタログの組み立てが拒むので、完全一致で 1 件に定まる」に直す | いいえ | なし |
| G3-12 | できれば | `src/report/tally_tests.rs:177` | 「実データの 38 ページ」と実データの数を書いている（今は正しいが、カタログが変わると古くなる） | 数を消して「実データに今この形の対は無い」にする | いいえ | なし |
| G3-13 | できれば | `src/evidence/candidates.rs:28` | 「拾う数は常時走るテストでは守らない（要件 6.2）」の要件番号が中身と合わない。6.2 は「スナップショットが無くても赤にしない」 | 要件番号を消すか、5.9 にする | いいえ | なし |
| G3-14 | できれば | `src/check/freshness_tests.rs:20` | 「報告は 4 本ある」。このモジュールは報告 5 本を見る | 「ドメイン別報告は 4 本ある」にする | いいえ | なし |
| G3-15 | できれば | `src/evidence/candidates.rs:76-80,385-639`（`resolve.rs:50-53` が借りる） | ソースの字句の読み手（`line_offsets`・`slice_element_names`・`read_string`・`read_raw_string`・`skip_*`）が候補のファイルに住み、証拠の側が `super::candidates::` から借りている。1 ファイルが 2 つの仕事をしている | `evidence/lex.rs` へそのまま移す（振る舞いは変わらない） | いいえ | なし |
| G3-16 | できれば | `src/check/structure_tests.rs:105`・`src/evidence/extract_tests.rs:60` | 直前の等式と同じことを言う重複の assert（`kinds == []` の後の `len() == 0`、1 要素の vec の等式の後の `len() == 1`） | 2 行を消す | いいえ | なし |

## 既知の項目のうち G3 の外のもの（確かめた結果）

- `tests/consistency/briefing_checks.rs:223-230` と `284-287` はまだ残っている。実データに second-stage の override も insufficient の行も無いことを assert しているので、道具が正しくてもデータが 1 行増えれば赤になる（元の指摘 11 と同じ偽の赤の種）。G3 の外なので、その範囲の読み手に回す。

## ruling-conflicts

- なし。G3-1 は要件 6.8（完了 toolkit）の文言とぶつかって見えるが、開発者の裁定ではない。指摘 50 と同じ理屈（読み込みの段が先に落とす）で消せる。

## 読んだファイル

| ファイル | 行数 | 読んだ |
|---|---|---|
| src/check/mod.rs | 79 | はい |
| src/check/finding.rs | 202 | はい |
| src/check/finding_tests.rs | 297 | はい |
| src/check/content.rs | 280 | はい |
| src/check/content_tests.rs | 364 | はい |
| src/check/content_link_tests.rs | 813 | はい |
| src/check/freshness.rs | 129 | はい |
| src/check/freshness_tests.rs | 569 | はい |
| src/check/structure.rs | 233 | はい |
| src/check/structure_tests.rs | 755 | はい |
| src/check/test_support.rs | 118 | はい |
| src/evidence/mod.rs | 127 | はい |
| src/evidence/extract.rs | 93 | はい |
| src/evidence/extract_tests.rs | 315 | はい |
| src/evidence/resolve.rs | 297 | はい |
| src/evidence/resolve_tests.rs | 832 | はい |
| src/evidence/candidates.rs | 643 | はい |
| src/evidence/candidates_tests.rs | 454 | はい |
| src/report/mod.rs | 8 | はい（指摘なし） |
| src/report/bundle.rs | 129 | はい |
| src/report/bundle_tests.rs | 311 | はい |
| src/report/domain.rs | 188 | はい |
| src/report/domain_tests.rs | 397 | はい |
| src/report/summary.rs | 258 | はい |
| src/report/summary_tests.rs | 388 | はい |
| src/report/tally.rs | 253 | はい |
| src/report/tally_tests.rs | 410 | はい |

Verdict: 振る舞いの不具合は見つからなかった。先に直すのは G3-1（`UnknownTheme` は実データの経路から届かない）。注記が事実と違う G3-2〜G3-9 は同じ変更で直せる。
Lean: G3-1 で約 −120 行（判定 1 つと在中テスト 4 本）、G3-16 で −2 行。依存は −0。

確かめていないもの: テストは走らせていない。外のファイルは、指摘を確かめるのに要る所だけ読んだ（`ledger/read.rs`・`cli/inspect.rs`・`catalog/build.rs`・`tests/consistency/checks.rs`・`briefing_checks.rs`・README）。字句の読み手の端の場合（`'\u{…}'` のような長い文字リテラルや、`'あ'` のような多バイトの文字リテラル）は、実データに当たる例を探していない。
