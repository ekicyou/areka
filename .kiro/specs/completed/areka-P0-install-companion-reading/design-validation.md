# 設計の検証レポート: areka-P0-install-companion-reading

- 作成日: 2026-10-04（ブランチ `claude/areka-p0-install-companion-c1b07d`・設計のコミット `a022ee0f`）
- 対象: `design.md`（`requirements.md` の要件 1〜8 に対して）
- 方法: 設計が既存のコードについて述べていることを、ソースを読んで 1 つずつ確かめた。テストの実行はしていない（全体テストも回していない）。対話なしで判定まで出した。
- 判定: **GO**（下の 3 点は設計の討議で決めれば足り、設計の骨組みを変える必要は無い）

## 1. 総評

設計は、読み替えを `install.txt` の読み手の 1 関数に閉じ、配置の計画は「段ごとの前方一致」と「途中のフォルダのエントリを本体から除く」の 2 点だけを広げる形で、要件 1〜8 の全ての項目に部品とテストを割り当てている。既存のコードについての主張は、確かめた範囲で全て成り立った（成り立たない主張は 0 件）。残るのは、要件の文言と設計の読み方の間に隙間がある 3 点で、どれも実装の前に一言決めておけば済む。

## 2. 既存コードについての主張の確かめ（結果）

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| 鍵は順を保たないので、探索の順は鍵を引いて作る | `crates/areka-nar/src/manifest.rs` の `lowercased_keys`（`BTreeMap` へ写す関数）と `collect_companions` | 成り立つ |
| 断る理由は 14 種のまま・足すと `judge.rs` が止まる | `crates/areka-nar/src/error.rs` の `refuse_reasons!` の宣言（14 種）と、`crates/areka/src/install/judge.rs` の `failure_word`（ワイルドカードの腕の無い `match`） | 成り立つ |
| 記録の種類は 5 種で、`crates/areka/` に種類の名前を書く場所は 0 か所 | `error.rs` の `ManifestWarning`。`crates/areka/` を `ManifestWarning` で検索して 0 件 | 成り立つ |
| 種類を数で固定しているのは `error_tests.rs` だけ | `crates/areka-nar/src/error_tests.rs` の `warning_name` と `manifest_warnings_are_five_and_carry_their_key` | 成り立つ |
| `collect_tree` は剥がす段数を引数で受ける | `crates/areka-nar/src/plan.rs` の `collect_tree`（第 3 引数 `strip`） | 成り立つ |
| 取り出し元を `/` 区切りで持てば `terms.rs` の本体は変更 0 | `crates/areka/src/install/terms.rs` の `nested_terms`（`format!("{}/{file}", …)`）と、`crates/areka-nar/src/lib.rs` の `NarArchive::entry_bytes`（パスの全体を ASCII の大小を無視して比べる） | 成り立つ |
| 取り出し元の値は宛先のパスに継ぎ足されない | `plan.rs` の `companion_placement`（宛先は `root.join("balloon").join(&companion.directory)`。取り出し元は `in_folder` の引数にしか現れない） | 成り立つ |
| 記録を出す出口は `NarArchive::install` の `warn!` で、変えずに済む | `lib.rs` の `NarArchive::install`（`outcome.warnings` を 1 件ずつ出す） | 成り立つ（ただし下の指摘 1） |
| 結果が変わる既存のテストは 3 本 | `balloon<数字>.` と `source.directory` を書いている全てのテストを検索。欠番のある番号付き・区切りを含む取り出し元を書いているのは `manifest_companion_tests.rs` の 3 本だけ。`reads_the_refresh_and_mask_of_each_companion`（`balloon0`・`balloon1`）は欠番が無く変わらない | 成り立つ |
| 新しいテストのファイルは、既存のテストのファイルの子として繋げば助手の可視性を変えずに済む | `manifest_tests.rs` の末尾（`mod companion` の接続の宣言が既に在る）。`plan_tests.rs` の助手 `planned`・`plan_of`・`refusal_of`・`escapes`・`root_with_ghost` は私有だが、子のモジュールからは見える | 成り立つ |
| `TempPath` の置き場は OS の一時フォルダ・`WorkDir` は `target\` の下 | `crates/temp-path-kit/src/lib.rs` の `TempPath::new`（`std::env::temp_dir()` の下）と、`crates/sample-ghost-kit/src/devroot.rs` の `find_target_dir`・`WorkDir::new`。`.cargo/` は無い | 成り立つ |
| `terms_tests.rs` の助手 `open` は `TempPath` を使っている・`sample-ghost-kit` は `areka` の dev 依存に在る | `crates/areka/src/install/terms_tests.rs` の `open`、`crates/areka/Cargo.toml` | 成り立つ |
| 台帳の検査は `note` の文面を見ない | `crates/ukadoc-survey/src/check/content.rs` の `check_evidence`（実装済みの項目の URL がソースに在るかだけ）。派生の文書（`briefing-assets.md`・`linkage.md`）に載るのは項目の識別子だけで、`note` の文は載らない | 成り立つ |
| `doc/COMPAT_ARCHITECTURE.md` §8 の表は 4 列 | §8 の表の見出し行（項目・裁量・根拠・出典 spec） | 成り立つ |
| 上書きする完了 spec の要件は 3.9 と 3.12 | `.kiro/specs/completed/areka-P0-nar-install/requirements.md` の Requirement 3 の 9 項・12 項 | 成り立つ |

### 2.1 要件 2.4・2.9 が設計どおりに出るか（机上で追った）

取り出し元 `extra/bal1` を例に、`in_folder`（段ごとの前方一致）と `leads_to_folder`（途中のフォルダのエントリ）を本体から除き、残りを `collect_tree` に通した結果を追った。

| 書庫の中身 | フォルダのエントリ | 本体の側 | 同梱の側 |
|---|---|---|---|
| `extra/bal1/descript.txt` だけ | 無し | `extra` は作られない | `descript.txt` |
| 同上 | `extra/`・`extra/bal1/` が在る | `extra/` は途中のフォルダとして除かれ、`extra/bal1/` は配下として除かれる。`extra` は作られない | `descript.txt` |
| `extra/readme.txt` も在る | 在っても無くても | `extra/readme.txt`（親の `extra` は `collect_tree` がファイルの親として作る） | `descript.txt` |
| 空のフォルダ `extra/other/` も在る | `extra/other/` が在る | `extra/other`（親の `extra` も作る） | `descript.txt` |
| `extra/bal10/x.png` も在る | — | `extra/bal10/x.png`（段ごとに比べるので配下にならない） | 入らない |
| 同梱が 2 つ（`extra` と `extra/bal1`） | 在っても無くても | `extra` の配下は 1 件も無い | 前者に `bal1/descript.txt`、後者に `descript.txt` |

- フォルダのエントリの有無で本体の結果は変わらない。要件 2.4 は成り立つ。
- 同梱ごとに自分の取り出し元だけで判定するので、重なった部分は両方へ入る。要件 2.9 は新しい枝なしで成り立つ。
- 1 段の取り出し元には途中のフォルダが無いので、`leads_to_folder` は 1 件も当たらない。要件 5.8（今と同じ）は構造で成り立つ。
- `in_folder` が真のエントリは取り出し元の段数以上の段を持つので、`collect_tree` の剥がしは範囲を外れない。

### 2.2 変更 0 とされたファイルが本当に触らずに済むか

- `crates/areka/src/install/procedure.rs`・`judge.rs`: 断る理由の種類を足さず、`InstallManifest` の欄も `Companion` の欄と型も変えないので、触らずに済む。`crates/areka/` で `Companion.source_directory` と `companions` を読むのは `terms.rs` の `nested_terms` だけである（検索して他は 0 件）。
- `crates/areka-nar/src/install.rs`: 記録は `manifest.warnings` を写すだけで、種類を見ない。触らずに済む。
- `crates/areka-parsers/`: 読み替えは `areka-nar` の中の私有の関数で閉じる。触らずに済む。
- `crates/areka-ghost/`: 起動の側は `balloon.directory` の 1 鍵だけを読み、本 spec はその値を読み替えない。触らずに済む。
- 見張りのテスト（`crates/log-capture-kit/tests/temp_path_guard_test.rs`）は `std::env::temp_dir` の呼出だけを見るので、`terms_tests.rs` を `WorkDir` へ替えても当たらない。

### 2.3 要件の全項目に部品とテストが在るか

- 要件 1.1〜8.5 の全ての項目が、設計の Requirements Traceability の表に 1 行ずつ在る（抜けは 0 件）。
- 要件 8.1 の 13 の場面は、全て Testing Strategy の表のどれかに在る（抜けは 0 件）。うち「本体の `directory` に区切り」は既存の `manifest_tests.rs` の `refuses_a_directory_that_is_not_a_one_level_name` に任せている。
- 新しいテストを持たない項目は、1.8・1.10・2.6・5.5・5.7・6.7（どれも変更 0 の枝で、既存のテストが通り続けることで見る）と、2.8（下の指摘 2）である。

## 3. 設計の討議へ渡す 3 点

### 指摘 1: 断られた書庫では、読み替えの記録がログに出ない

- **気になる点**: 記録（`SourceDirectoryCleaned`・`CompanionNotSearched`）がログに出るのは、`lib.rs` の `NarArchive::install` の成功の枝だけである。`../extra/bal1` と書いた作者の書庫で、取り除いた後の `extra/bal1` が書庫に無いと、`CompanionSourceMissing` で断られ、ログには「取り出し元フォルダが無い: `extra/bal1`」だけが残る。取り除いたことの記録は出ない。`install.txt` の解釈で断られた場合（`NarArchive::open` の失敗）は、記録の列ごと捨てられる。
- **なぜ効くか**: 要件 6 の目的は「思ったとおりに入らなかった理由を自分で辿れる」ことで、取り除きの記録が一番要るのは、取り除いた結果の場所に何も無くて断られたときである。要件 6.7 の文（「インストールが終わったとき」「今の読み飛ばしの記録と同じく」）は満たしているが、目的には届かない場面が残る。
- **案**: ⑴ 今のままとし、設計の Error Handling に「断られたときは記録を出さない。理由には鍵と取り除いた後の値が載るので、書いた値との違いは作者が見比べて分かる」と 1 行書く（変更 0 の約束を保つ）。⑵ `NarArchive::install` の失敗の枝でも記録を出す（`lib.rs` の本体を変えるので、境界の節を書き直す）。手間が少ないのは ⑴。
- **対応する要件**: 2.5・6.1・6.7
- **設計の場所**: 「Error Handling」の表と「Monitoring」

### 指摘 2: 要件 2.8（開発用の道具の追随）を確かめる手が 1 つも無い

- **気になる点**: `crates/sample-ghost-kit/examples/fold-samples.rs` の `strip_folder` を複数段へ広げるが、テストは 0 本、階層付きの検体も 0 体で、広げた枝は一度も実行されない。設計はこれを明記しているが、受け入れ基準の 1 項目が「コンパイルできること」と「読み合わせ」だけで閉じることになる。
- **なぜ効くか**: 「`extra/bal10` を `extra/bal1` の配下と読まない」のような段ごとの突き合わせは、`plan.rs` の側ではテストで固定するのに、同じ判断を写したこの道具の側は固定されない。開発者の方針（正しく動くことと固定されていることは別）とぶつかる。
- **案**: ⑴ `strip_folder` に小さなテストを 1 本だけ足す（1 段は今と同じ・2 段を剥がす・`extra/bal10` は当たらない、の 3 つ）。例のテストは既定では走らないので、`crates/sample-ghost-kit/Cargo.toml` に例の宣言（`test = true`）を 1 つ足す。公開面は増えない。⑵ 今の設計のまま進め、確かめないことを開発者が承知したと討議に残す。
- **対応する要件**: 2.8
- **設計の場所**: 「開発用の道具 / fold-samples」の「確かめ方」

### 指摘 3: 要件 8.2 の「各場面で」を、設計は分担に読み替えている

- **気になる点**: 要件 8.2 は 13 の場面の**それぞれ**で「置かれたファイルとフォルダの全てが宛先の配下に在ること・列の並び・記録」を確かめると書く。設計は、探索の場面（打ち切り・無印・先頭の 0・断片）を `parse_manifest` だけで確かめ、置き場所は配置の計画のテストと公開の入口の 2 本でまとめて確かめる。探索の場面の 1 つずつについて置き場所を見るテストは無い。
- **なぜ効くか**: 分担そのものは筋が通っている（探索の場面は取り出し元が 1 段で、その置き場所は既存の `plan_tests.rs` の `every_path_of_every_plan_stays_under_the_root` が固定している）。ただ「全ての場面で」の形の要件は、タスクごとのレビューでは満たしたかどうかが見えにくい。読み替えたことを開発者が承知していないと、実装の検証の段で差し戻しになる。
- **案**: 討議で「要件 8.2 は設計の分担の読み方でよい」と一言決める。足すなら、公開の入口の 2 本目のテスト（打ち切りの後ろの宛先が作られないことを既に見ている）に、先頭に 0 を付けた綴りの鍵を 1 つ加えるだけで、探索で読まなかった 2 種類の両方が置き場所まで通る。
- **対応する要件**: 8.2（と 8.1）
- **設計の場所**: 「Testing Strategy」の `lib_companion_tests.rs` の節の「要件 8.2 の分担」

## 4. 作りすぎの点検

要件が求めていない部品は見つからなかった（新しいクレート・依存・公開の関数・本番のファイルは 0 個）。削れる余地は次の 2 つで、どちらも小さい。

- `lib_companion_tests.rs`（新しいファイル）は 2 本だけである。既存の `crates/areka-nar/src/lib_tests.rs`（599 行）の末尾に足せば、新しいファイルが 1 本減り、`lib.rs` に接続の宣言を足す必要も無くなる（`lib.rs` が 1 行も変わらなくなる）。
- 「同じ入力を 2 度読んで結果が等しい」テスト（要件 6.6）は、鍵が `BTreeMap` で探索が数の順なので、壊れる経路が無い。記録の列の完全一致を見る他のテストが並びを固定しているので、無くても要件 6.6 は守られる。残すかどうかは好みの範囲。

次の 3 つは要件が求めているので、作りすぎではない。

- 記録の種類を 2 つ足すこと（要件 6.3 が「知らない鍵の読み飛ばしと区別できる形」を、要件 6.1 が書かれていた値と取り除いた後の値の両方を求める）。
- `leads_to_folder`（要件 2.4 の「フォルダのエントリの有無で結果を変えない」に要る。数える処理を書かずに済ませている）。
- `terms_tests.rs` の助手を `WorkDir` へ替えること（要件 8.5。足す 1 本が同じ助手を使う）。

## 5. 良い点

- **読み替えた値を宛先に継ぎ足さない形**。取り出し元の値は書庫の中のエントリを選ぶためにしか使わず、宛先は検査を通った 1 階層の名前と検証済みのエントリ名の尾だけで組む。`..` の取り除きに誤りが在っても根の外へ書き出す道にならないことが、テストではなく構造で言えている。
- **後ろの層を動かさずに並びを変える見立てが正確**。`companions` の順を変えれば、確定・インストールの手続き・知らせまで並べ替え無しで運ばれることをソースで確かめてあり、断る理由の種類を足さないことで `judge.rs` に触らずに済ませている。並走する `areka-P0-install-live-target-hazards` と重なるファイルは 0 本である。

## 6. 判定

- **GO**
- **理由**: 既存のコードについての主張は全て成り立ち、要件 2.4・2.9 は机上で追って設計どおりの結果になり、変更 0 とされたファイルは実際に触らずに済む。指摘の 3 点は「設計に 1 行書く」か「テストを 1 本足す」で閉じ、骨組みは変わらない。
- **次の手**: 設計の討議で指摘 1〜3 を決める → `/kiro-spec-tasks areka-P0-install-companion-reading`。台帳の `note` を書き換えるときは、今の 3 行（「壊れ方:」「束:」「根拠の場所:」）の形を残すこと（検査は見ないが、他の行と揃えるため）。
