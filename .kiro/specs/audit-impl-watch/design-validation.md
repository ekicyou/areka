# 設計の検証: audit-impl-watch

- 検証日: 2026-11-13
- 対象: `.kiro/specs/audit-impl-watch/design.md`（要件 `requirements.md`・調べ `research.md`・steering `product.md`／`tech.md`／`structure.md`）
- 突き合わせたコード: `crates/areka-impl-watch/src/`（`store.rs`・`main.rs`・`plan.rs`・`plan_turns.rs`・`main_layering_tests.rs`・各テストの支え）・`tests/real.rs`・`tests/real/mod.rs`・`doc/impl-watch.md`・完了 `areka-P0-impl-watch` の design.md
- 進め方: スキル `kiro-validate-design` の手順（分析 → 重い指摘 → よい所 → GO／NO-GO）。対話なしで判定した。

## まとめ

設計は 17 件の指摘を 1 件ずつ「直し・消す物・足すテスト・要件」に落としていて、書かれた行番号・関数名・道具はコードと合っている。外への約束（コマンド・引数・終了コード・端末の文・`--help`・状態ファイルの形と版 2）を変える直しは無い。重い食い違いは無く、残る指摘は #8 に付いて回る文の直しの漏れと、#3 の見張りの較正の弱さの 2 件だけで、どちらもタスクを作る段で直せる。

## コードとの突き合わせ（確かめた事実）

| 設計の主張 | 確かめた結果 |
|---|---|
| `store.rs` の排他は `File::try_lock`（10 ms ごと・合計 10 秒まで） | 合う。`store.rs:245-267`（`lock`）・`LOCK_PAUSE`／`LOCK_LIMIT`（`:35-37`）。`summary` は `:203-210` で排他を取らずに `write_status` を呼ぶ |
| `File::try_lock` は Rust 1.89 で安定・手元は 1.100.0 | 合う。`rustc 1.100.0`・edition 2024。`store.rs:19` がもう `TryLockError` を読んでいる |
| `Participant::id` を判断が読むのは `plan.rs:355`・`:359`・`:574` の 3 か所 | 合う（`Unstop` の比べ・`Unstopped` の識別・`resume` の `Resumed` の識別） |
| `resume` の呼び手は 3 か所で、どれも手元に識別がある | 合う。`plan_turns.rs:142`（`load_running`・引数 `id`）・`:193`（`replan`・`values_mut()` を `iter_mut()` に替えれば鍵が取れる）・`:293`（`grant_one`・`request.id`） |
| T2 の較正 `plan_turns.rs:304` を `if true` に | 合う。`:304` は `if request.desks.has_test() {` |
| `main.rs` は `#[path]` のリテラルを持つ | 合う。`main.rs` 末尾の `#[path = "main_layering_tests.rs"]`。今の見張りの doc（`main_layering_tests.rs:7`「リテラルの無い起動ファイル」）は事実と違う |
| ASCII の見張りは手で持つ 11 本の一覧 | 合う。`main_layering_tests.rs:150-162` |
| 写しの数（`T0` 4・`entries` 3・`names` 2・`home_in`→`Home` 2・`read_state` 2） | 合う。`plan_test_support.rs:10`・`status_tests.rs:12`・`store_test_support.rs:19`・`wait_test_support.rs:7`／`cli_test_support.rs:100`・`home_tests.rs:16`・`presence_tests.rs:25`／`cli_test_support.rs:110`・`store_test_support.rs:114`／`presence_tests.rs:18`・`store_test_support.rs:42`／`cli_test_support.rs:124`・`store_test_support.rs:129`。中身も同じ（`cli_test_support.rs:60` の `home_in` は `PathBuf` を返す別物で、設計どおり残す） |
| crate の根に `mod test_support` を足しても衝突しない | 合う。`store.rs:413`・`plan.rs:631`・`cli.rs:389`・`wait.rs:306` の `test_support` は各モジュールの子で、`crate::test_support` とは道筋が違う。`Home::dir` は `pub`（`home.rs:15`）なので根の支えから組める |
| `hold_state_lock`（同じプロセスの別のハンドルで握る）が T8 に使える | 合う。`store_ports_tests.rs:76-85`。`:295` の既存のテストが同じやり方で排他の衝突を作っている |
| `super::holds_merge`・`super::lossy` で写しを消せる | 合う。`plan_liveness_tests.rs` は `plan_turns.rs:393` の子で `plan_turns.rs:219` に `holds_merge`。`store_ports_tests.rs` は `store.rs:421` の子で `store.rs:401` に `lossy` |
| 状態ファイルが無い・壊れた・版が合わないときはロックに触らない（D5） | 既存のテストと合う。`cli_commands_tests.rs:334-376` が `status` の後の名前の一覧に `state.lock` が無いことを見ている。D5 は最初の読みで返すので、これらは今のまま通る |
| 1,000 行の番人 | 越えない見込み。足すのは `plan_stop_tests.rs`（698）・`plan_turns_tests.rs`（486）・`store_ports_tests.rs`（742）・`main.rs`（35）。いちばん長い `plan_desk_tests.rs`（924）には足さない |
| 完了 spec の要件は「排他を取らない」と書いていない（D7） | 合う。完了 `areka-P0-impl-watch` の「ロック無し」は design.md の表（398 行目）と限界の注（567 行目）だけ |
| 実機テストは #8 の後も主張が変わらない見込み | 筋は通る。待ち・見張りが `state.lock` を握るのは 30 秒ごとの一回り（`wait.rs:33` `TICK_EVERY`）と終わりの抹消だけで、毎秒の読み直し（`POLL`）は排他を取らない。`tests/real.rs:320` の `status_until` は端末の要約を見るので、書かなかった回でも当たる |

## 重い指摘（2 件）

### 🔴 指摘 1: #8 に付いて回る文の直しが 2 か所漏れていて、タスク 11 の「`cli.rs` の差分 0」と食い違う

- **何が問題か**: #8 の後、`status` は他の呼び出しが `state.lock` を握っている回には `status.md` を書かない。ところが設計が直す文は `doc/impl-watch.md` の 576 行目と 384 行目だけで、次の 2 か所が今の書き方のまま残る。
  - `doc/impl-watch.md:588`（7 節の置き場所のファイルの表）「状態が変わるたびと `status` のたびに書き直される」
  - `cli.rs:353` の doc「状態を変えずに読み、読み物を書き、…」
  さらにタスク 11 は `cli.rs` の差分 0 を確かめると書いているので、かけ直しの audit（タスク 10）がこの doc を古いと指したとき、直すとタスク 11 に反する。`--help` の `status` の行（`cli_args.rs:419` の `write status.md and print a summary`）も同じく言い過ぎになるが、こちらは外への約束なので変えないのが正しい。
- **響き**: 要件 3.2（壊れる文書を全部直す）と 9.3（同じ変更の中で手順書の文を直す）を満たし切らない。かけ直しの audit が文と動きの食い違いとして指し、0 件まで繰り返す回数（要件 8.2）を無駄に増やしうる。
- **直し方**: タスク 2 の触るファイルに `doc/impl-watch.md:588` を足す（例「状態が変わるたびと、`status` が排他を取れたたびに書き直される」）。`cli.rs:353` の doc も 1 行直し、タスク 11 の確かめを「`cli.rs` はコメントだけ」（`state.rs` と同じ扱い）に改める。`--help` の行は外への約束なので変えない旨を、設計の Out of Boundary か Revalidation Triggers に 1 文書いておく（かけ直しの audit への答えになる）。
- **要件**: 3.2・6.4・8.2・9.3
- **設計の場所**: §File Structure Plan の Modified Files の表（`doc/impl-watch.md` の行）・§タスクの見取り図のタスク 2 とタスク 11

### 🔴 指摘 2: #3 の見張り（`src/` を読む形）の常設の較正が「0 本なら赤」だけ

- **何が問題か**: D2 は手で持つ一覧を消し、`src/` の `.rs` のうち名前が `_tests.rs`・`_test_support.rs` で終わらないものを全部見る。常設の較正は「見たファイルが 0 本なら赤」だけで、絞り込みの誤り（例えば末尾の判定を `_test` で切ってしまい、本番のファイルの一部が抜ける・`main.rs` だけが残る）は 1 本でも見えていれば緑のまま通る。T3 の「日本語のリテラルを持つ一時のファイルを置くと赤」は手で 1 度確かめるだけで、後の変更で絞り込みが壊れても落ちるテストが残らない。
- **響き**: 要件 4.1（直した所が壊れたら落ちるテストを 1 本ずつ）が #3 については弱い。見張りが黙って見る範囲を狭めても気付けない（記憶の決めごと「0 件を書く前に較正」と同じ穴）。
- **直し方**: 常設の較正を 1 つ足す。一覧を持ち直さずに済む形として、例えば「見たファイルに `main.rs`（クレートの根）と `cli.rs`（端末へ出す入口）が入っている」を確かめる。名前の絞り込みは 1 つの関数に置き、`plan_test_support.rs`・`cli_args.rs`・`main_test_support.rs`・`plan_turns.rs` の 4 つの名前で、外す・外さないを確かめる短いテストを付ける（`cli_args.rs` と `plan_turns.rs` は外さない、`_test_support.rs` の 2 つは外す）。
- **要件**: 4.1・4.2
- **設計の場所**: 設計の決め D2・§構造の見張り・§Testing Strategy の T3

## よい所

1. **#8 の直しが、今ある作りの順と道具をそのまま使っている。** `store.rs` の `lock` が使う `File::try_lock` を 1 回だけ試す形にし、無い・壊れた・版の合わないファイルは最初の読みで今どおり返す（D5）。このため `cli_commands_tests.rs:334-376` の「ロックを作らない」の主張も、端末の文・終了コード・`--help` も変わらない。読み直しの失敗は `read_only` の `error!` と `Err`、試しの OS の失敗は読み物と同じ `warn!` と、失敗の記録の決まり（要件 5.2）にも沿っている。
2. **指摘ごとの表と、消す物の確かめが具体的。** 17 件それぞれに行番号・消す行の数・足すテスト・較正（直しを戻すと赤になる 1 か所）が書かれ、突き合わせた行番号はどれも今のコードと合っていた。#1 は欄 `Participant::id` を消さずに鍵で見る形にして、状態ファイルの形と版 2 を守り（要件 6.4・7.3・9.2）、古い実行ファイルが同じファイルを読んでも動きが変わらないことまで書いている。

## 判定

- **判定: GO**
- **理由**: 外への約束を変える直しは 0 件で、要件 9.1〜9.4 の 17 件は全部、直し方とテストが決まっている。2 件の指摘はどちらも文の直しとテストを 1 本足す程度で、設計の形を変えずにタスクの段で取り込める。
- **次の手順**:
  1. 設計の討議（`/kiro-design-discussion audit-impl-watch`）で指摘 1・2 の取り込みを決める（取り込むなら design.md の Modified Files の表・タスク 2／11・D2・T3 を直す）。
  2. 決まったら `/kiro-spec-tasks audit-impl-watch` でタスクを作る。
