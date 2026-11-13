# 設計の検証: audit-mcp-update-crates

> 2026-11-13・枝 `claude/kiro-start-audit-mcp-update-crates-dd5eaa`（HEAD `ccfb7e97`）。対話なしで行った（開発者への問いかけはしていない）。
> 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`roadmap.md` の C19 の行ほか）。設計の主張は、`crates/areka-mcp/`・`crates/areka-mcp-bridge/`・`crates/areka-update/`・`crates/areka-user-data/src/` のコードとツリー全体の grep で突き合わせた。テストは回していない。1 つだけ、この机で .NET の `Socket` を使って `SO_REUSEADDR` の番号の割り当てを測った（下の「確かめたこと」）。

## まとめ

設計は 51 件の指摘を 1 件ずつ「直し方・縛るテスト・束」に落としていて、そのまま実装に入れる細かさがある。#1（中継の `AREKA_PROFILE_DIR`）の流れ・範囲の外の 1 語・C19 の並走の約束・1,000 行の番人は、コードと突き合わせて設計のとおりに成り立つ。ただ、消すテストの名前を範囲の外の文書 `doc/ssp-mcp/transport-diff-areka.md` が名指ししていて、設計の「参照は 0 か所」と「範囲の外で変えるのは 1 つだけ」が事実と合わない。これは設計の直しで済む小さな穴なので、判定は GO とする。

## 確かめたこと（設計の主張とコード）

| 設計の主張 | 結果 | 根拠 |
|---|---|---|
| `usable_profile_env` は `areka-user-data` の `lib.rs` の根に `pub(crate)` で在り、`pub` にする 1 語で外から呼べる（`pub use` は要らない） | 成り立つ | `crates/areka-user-data/src/lib.rs:205`。引数 `Option<&OsStr>`・戻り `Option<PathBuf>` で、`from_inputs` の `profile_env` と型が合う |
| 使える値を受けた `locate_app_profile` は必ず `Ready(<その値>)` を返す（R3）＝隣に areka が居るときは `profile_dir` を渡さなくても置き場は今と同じ | 成り立つ | `establish.rs:185-218`: 使える値があると `if profile_env.is_none()` の `match` を飛ばし、`app_profile_dir` が `profile_env_dir` の値を先に返す |
| #1 の縛るテストは直しを外すと赤 | 成り立つ | 今の `from_values` は `(Some(dir), _) => Some(dir)` で捨てるべき値を先に採る（`config.rs:127-128`）。隣に areka が無い道では今は `usable_profile_env` を呼ばない＝ERROR 0 件 |
| `entrances` を範囲の外から使う所は 0 か所 | 成り立つ（コードでは） | `crates/` の `.rs` で `entrances` を呼ぶのは `areka-mcp` の中だけ。本番の `main.rs` は `all_entrances` を呼ぶ。3 つのクレートと `areka-user-data` はどれも `publish = false` |
| `register_rows` の引数を `impl IntoIterator<Item = (&'static str, Parse)>` にしても今のテストは乗り換えられる | 成り立つ | 呼ぶのは `tools/mod.rs:123` と `tools_tests.rs:382` だけ。後者の行は文字列の定数だけで組んでいる |
| #13 のテストで `McpServer { listening: Some(Listening { .. }) }` を組める | 成り立つ | `server_tests.rs` は `server.rs` の子のモジュール。`Listening` の欄（`addr`・`token`・`done`・`_inbox`・`_thread`）は設計の書いたとおり。`ActorHandle` は落としても join しない（`areka-actor/src/spawn.rs:181-186`） |
| #11: 今は `from_utf8_lossy` で読めない値を置換文字入りにして拒んでいる | 成り立つ | `dispatch.rs:86-90` |
| #26: 判定を関数に出しても `lib_tests.rs` の文字の検査と `log_history_convention_tests.rs` に触れない | 成り立つ | `winhttp.rs:191` の 1 行の置き換えで、`unsafe` も WinHTTP の綴りも足さない。`log_history_convention_tests.rs:388` が見るのは `areka_update::fetch`（ファイル `fetch.rs`）で、設計は名と在りかを変えない |
| `nobody()` の `SO_REUSEADDR` で、並んで走るテストに同じ番号が渡らない | この机では成り立つ | `SO_REUSEADDR` を付けて `127.0.0.1:0` に束ねたまま待ち受けないソケットを 3,000 個作ると番号は 3,000 個とも別。その間に付けずに `bind(0)` を 2,000 回しても、握っている番号は 1 回も返らなかった |
| 中継の dev 依存に `TcpSocket` の要る `tokio` の `net` が在る | 成り立つ | `areka-mcp-bridge/Cargo.toml` の `[dependencies]` の `tokio` に `net`・`sync` が在る（`Cargo.toml` は変えなくてよい） |
| C19 の約束: `areka-user-data` を触る C19 の spec は無い | 成り立つ | `roadmap.md` の C19 の行に `areka-user-data` は無い。`runtime-module-untangle` が触る文書は `doc/ssp-mcp/log-convention.md` ほかで、下の問題 1 の文書とは重ならない |
| 1,000 行の番人 | 成り立つ | いま最大は `service_tests.rs` 869・`run_tests.rs` 781。足しの先の `service_failure_tests.rs` 257・`run_fail_tests.rs` 238・`tools_own_socket_tests.rs` 272・`testkit.rs` 199 で、設計の見立ての後も 1,000 行に届かない |

## 大事な問題（3 件まで）

### 問題 1: 消すテストの名前を、範囲の外の文書が名指ししている

- **何が問題か**: 設計の「消すファイル」は「`tools_socket_tests` の名は `tools/mod.rs` のほかに 0 か所」と書き、「範囲の外で変えるのは `areka-user-data/src/lib.rs` の 1 語だけ」とし、T14 では差分が範囲の 3 つのクレートと spec のフォルダとその 1 ファイルだけであることを確かめる、としている。ところがツリー全体を grep すると、`doc/ssp-mcp/transport-diff-areka.md` の「測ったテスト名」の列が、この spec で消すテストを名指ししている。`ping_returns_empty_object`（#20・17 行目の行はこの 1 本だけ）・`unknown_method_is_jsonrpc_error_from_rmcp`（#19・18 行目と 23 行目の行はこの 1 本だけ）・`tools_call_any_name_is_invalid_params`（#18）・`forward_answers_after_rmcp_415_only_for_json_bodies`（#17）・`unknown_name_is_invalid_params`（`tools_socket_tests.rs` の中＝#14）が載っていて、35 行目には `tools/tools_socket_tests.rs` のファイル名が在る。範囲の中でも、`tools_own_socket_tests.rs` の 4 行目と 96 行目の注が `tools_socket_tests.rs` を名指ししている。`doc/ssp-mcp/areka-tools.md:173` にも `entrances`・`tools_socket_tests.rs` が出る（こちらは完了した `areka-P0-mcp-author-tools` の設計の表の写し）。
- **何に響くか**: この文書は「areka の値はすべて決定論テストの assert から写した。rmcp の版を上げたら表のテストを回して測り直す」と書いた、rmcp の版上げのときの手順書になっている。消したテストの名前が残ると、版上げのときに回すテストが見つからない。逆に文書を直すと、T14 の「差分はこれだけ」の確かめが赤になる。要件 3.2・3.3（文字列での参照を含めてツリー全体で確かめ、壊れる所を全部直す）に対して、設計の測りが事実と合っていない。
- **直す案**: 設計に次を足す。⑴ 範囲の外で変えるファイルに `doc/ssp-mcp/transport-diff-areka.md` を足し、消すテストの名前を、同じ値を確かめて残るテストの名前へ差し替える（`ping` の行・知らない名前の行・知らないメソッドの行・`params` の形の行・35 行目のファイルの一覧）。どの行がどの残るテストへ移るかは、消す行ごとの「残るテストが同じ中身を縛っている」の確かめ（「テストの方針」の「消すテスト」）と同じ表で決める。⑵ `tools_own_socket_tests.rs` の 2 つの注を、移した後の形に直す。⑶ `doc/ssp-mcp/areka-tools.md` は完了した spec の設計の写し（その時点の約束の記録）なので直さない、と決めて書く。⑷ T14 の「差分はこれだけ」の確かめに `doc/ssp-mcp/transport-diff-areka.md` を足す。C19 でこの文書を触る spec は無い（`runtime-module-untangle` が触るのは `log-convention.md`）ので、要件 6.2・6.6 には触れない。要件 6.7 の「直しのために要る変更」に当たる。
- **対応する要件**: 3.2・3.3・6.7・7.5
- **設計の箇所**: 「ファイルの構成」の「消すファイル」・「範囲の外（1 つだけ）」・「仕上げの確かめ」の 4

### 問題 2: 注で閉じる 2 つの枝を、かけ直しの audit で数えない根拠が記録の節と食い違う

- **何が問題か**: 「直さない・回した指摘」の節は「なし」と書いたうえで、続けて「#45 のうち注で閉じた 2 つの枝（`commit.rs` の戻しで作ったフォルダを消せない枝・`delete.rs` の `symlink_metadata` の `Err`）が同じ形で再び出ても新しい指摘に数えない」と例外を置いている。要件 7.2 が数えないとするのは、要件 2.2 で開発者が残すと裁定した指摘と、6.4・6.5 で後へ回すと裁定した指摘だけで、この 2 つの枝はどちらとしても記録されていない。ponytail の audit は「テストの無い危ない所」を見るので、かけ直しで同じ 2 つの枝がまた出る見込みが高い。
- **何に響くか**: T14 で「新しい指摘 0 件」（要件 7.2・7.3）を示すときに、この 2 件を数えない理由が要件の記録から引けない。実装の段で「テストで起こせない枝」を、ひねった差し込みで無理に起こそうとする道にも、黙って外す道にも転びうる。
- **直す案**: 「注で閉じる」は要件の討議で承認された #45 の直し（`research.md` の #45「見つからなければ記録に残して閉じる」・§10.3）の一部だと、設計の「直さない・回した指摘」の節に 2 行で書き、2 つの枝の場所・起こせない理由・承認の出どころ（2026-11-13 の一括承認）を並べる。「なし」は「直さずに残す指摘（要件 2.2）は 0 件。#45 の直しの中で、テストを足さず注で閉じる枝が 2 つ」に書き換える。T14 でかけ直しの audit がこの 2 つの枝を挙げたら、`research.md` のかけ直しの結果にこの記録を指して書く。
- **対応する要件**: 2.5・4.1・7.2・7.3
- **設計の箇所**: 「直さない・回した指摘（要件 2.5・6.5 の記録）」・「#45 起こしにくい失敗の枝（D7）」

## 設計のよい所

1. **#1 の直しが、判定を 1 か所に残したまま、記録を 2 件にしない形になっている。** 隣に areka が居るときは `locate_app_profile` が既に値を判定して `error!` を 1 件残し、使える値の置き場も答えに含めている（`establish.rs` で確かめた）。設計はそこで `profile_dir` を渡さず、隣に居ないときだけ `usable_profile_env` を呼ぶ。範囲の外の変更は 1 語、`from_inputs` の引数と戻りも、`from_values` の公開の形も変えない。縛るテストは 2 つの値 × 隣の有無の 4 通りで、直しを外すとどれが赤になるかまで書いてある。
2. **時間で順番を決めていたテストを、テストが開けるまで答えない門・届いた後に閉じる・握ったままの番号、の 3 つで決定論にしている。** どれもテストの道具の中だけで済み、本番の束ね（`areka_mcp::start`）は変えない。`SO_REUSEADDR` の機序は実測を添えていて、この検証でも番号が重ならないことを確かめた。開け忘れても今の上限で赤になって止まる、前提が崩れたらテストを曲げずに止めて報告する、という逃げ道も書いてある。

## 小さな気づき（大事な問題ではない・実装のときに見ればよい）

- `tests/real_receiver_test.rs` は今 `tests/common/fake_server.rs` を読み込んでいない。#10 で `nobody()` を共有するなら、`exe_test.rs` と同じ `#[path = "common/fake_server.rs"] mod fake_server;`（理由の説明つきの `#[allow(dead_code)]`）を足すことになる。設計の「直すファイル」の行にそれを書いておくと迷わない。
- `usable_profile_env` を `pub` にすると、その説明の文が `pub(crate)` の `profile_env_dir` へのリンクを持つので、`cargo doc` を回すと `rustdoc::private_intra_doc_links` の警告が出る。全体テストは `cargo doc` を回さないので赤にはならない。範囲の外の変更を 1 語に保つなら、このままでよい。
- #44 ⑵ の順の根拠は tokio の `current_thread` の回し方に頼っている。設計は「実装の最初に WARN が出ることを確かめる」「前提が崩れたら止めて報告する」と書いているので、それで足りる。確かめが 1 回でも揺れたら、ひねらずに止めて報告する。

## 判定

**GO**

- **理由**: コードとの突き合わせで、#1 の流れ・範囲の外の 1 語・公開の形・C19 の約束・`log_history_convention_tests.rs`・1,000 行の番人はどれも設計のとおりに成り立った。残る 2 件は、範囲の外の文書 1 つを直す対象に足すことと、記録の節の書き方を直すことで、設計の作りは変わらない。
- **次にすること**: 設計の討議（`/kiro-design-discussion audit-mcp-update-crates`）で問題 1・2 を議題にして設計を直す。その後 `/kiro-spec-tasks audit-mcp-update-crates` でタスクを作る（設計の見立ては 14 本）。
