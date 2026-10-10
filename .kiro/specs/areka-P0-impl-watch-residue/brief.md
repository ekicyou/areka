# Brief: areka-P0-impl-watch-residue

> 起票: 2026-10-10（`areka-P0-impl-watch` の完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-impl-watch/tasks.md` の Implementation Notes（3.3・4.1・4.2・5.2・5.3・5.4・6.1 の「任意の残り」、「最終検証の残り」、7.2 → 7.3 の任意、7.4 の「知っていて開けてある残り」）。どれも読んで見つけた拾い残しで、机の貸し出しの結果を誤らせるものは無い。区分 C（開発の道具・バグでない持ち越し）。

## Problem

- **開発者**: `areka-impl-watch` は動くが、実装とレビューで「知っていて開けてある」と記録した小さな残りが 1 か所にまとまっていない。次にこのクレートへ手を入れる人が、同じ調べをやり直すことになる。
- 特に (a) は、失敗の文が「`impl-watch.log` を見よ」と指すのに、ログに訳が無い（調べる手がかりが切れている）。

## Current State

2026-10-10 時点（`areka-P0-impl-watch` の完了時・常時テスト 252 本・実機テスト 11 本）。

**直す向きが決まっているもの**

- (a) **壊れた状態ファイルの訳がログに残らない**: `store.rs` の読みが JSON の失敗（行と列）を捨てる。退避の `warn!` と `Broken` の `error!` に行と列が無いので、`state file is broken; see impl-watch.log` と出ても、ログから訳が分からない。
- (b) **時計が 2 系統**: 本物の待ちの口（`wait.rs` の `impl WaitPort for Store`）の `now()` が、`Store` の時計の欄でなく `store::unix_now()` を直に呼ぶ（本番では同じ値）。
- (c) **番の来る順の鍵が 2 か所**: `plan.rs`（番を決める）と `status.rs` の `merge_order`／`load_order`（`pos=<n>` と読み物の並び）。両者の一致を直に判定するテストが無く、片方だけ変えると `pos=` と `status.md` の並びが黙ってずれる。
- (d) **同じ綴り・同じ式の重複**: `none` の綴りが 3 か所（`status.rs` の `NONE`・`store.rs` の `NO_BACKUP`・`wait.rs` の裸のリテラル）。`stopped:` の列の式（空なら `none`・あれば `", "` でつなぐ）が `status.rs` と `wait.rs` の 2 か所。
- (e) **殺された前の待ちの記録が残る**: 始め直した待ちが「直ちの終わり」で終わると、殺された前の待ちの記録が残る（居る印を握っている間は「居る」と見えるため）。次の状態を変える呼び出しの回収で消える。実害は `status` の `waits` が 1 件多く見えること。直すなら `cli::waiting` の申し込みの中で、終わりが出たとき同じ排他のまま `UnregisterWait` を通す（8 行ほどの見立て）。

**決めてから直すもの（議題）**

- (f) **裁定 4 の残りの窓**: 見張りが自分の記録を消す書きが済んでから、居る印を解くまでの間に他人の探りが入ると、「印つき・居る」と見て印を消し、その後の呼び出しで回収されうる（読んで導いたもの・実測なし。起きても `watch` を立てて申し込み直せば戻る。設計書の弱点と手順書に記載済み）。閉じるか、今のまま記録に留めるか。
- (g) **要件 10.2 の字**: 「失敗にしない」例外は読み物（`status.md`）だけと書いてあるが、実物は、待ちの周期の確認（`Tick`）の失敗と、答えが出た後の記録の抹消の失敗も `warn!` だけで済ませる（設計書の「失敗にしないもの」には記載済み）。要件へ 2 つを足すか、実物を要件へ寄せるか。
- (i) **実機テストだけが檻のもの**: `[store] status.md not written…` の `WARN` の行・本物の 1 秒の眠り・ログの実ファイルの行（`command=`／`event=`）は、`#[ignore]` の実機テスト（`tests/real.rs`）だけが判定する。常時テストで拾えるか（記録の捕捉の道具を葉のクレートから使えるか）を調べ、拾えないなら理由を記録する。

**テストの穴（任意）**

- (h) `merging` が 2 以上になる状態を踏むテスト（先頭の机だけを見る変異が緑）／登録の直後の最初の `fingerprint` の失敗（`.ok().flatten()` に替える変異が緑）／1 つの識別が 2 つの机を持ったままの `merged`（2 行の出力）／同じ呼び出しの回収と番の決め直しで `stop requested again by <識別>` になる流れ／`Desk::settle` の `try_lock().ok()` が「握られている」以外の失敗も同じ赤に畳む／ASCII の走査を `store.rs`・`home.rs`・`presence.rs` へ広げる／構造テストの赤の文に「`std` からは 1 行に 1 つ」の訳を添える／`status_tests.rs` の見本の識別を小文字へ（挙動は同じ）。

## Desired Outcome

- (a) 壊れた状態ファイルに遭ったとき、ログに JSON の行と列が 1 行残る（常時テストか実機テストで判定する）。
- (b)〜(e) が片づき、綴りと式と鍵がそれぞれ 1 か所になる。
- (f)(g)(i) は、直すか記録に留めるかが決まり、要件・設計書・手順書がそれに合っている。
- (h) のうち採ったものが、変異を当てて赤になることを確かめた上で入っている。

## Approach

要件で (f)(g)(i) の向きを決め、残りは 1 件ずつ「赤になるテスト → 直し」で進める。まとめて 1 本にしたのは、どれも同じクレートの中の小さな直しで、1 件ずつ spec にすると手続きのほうが重いから。直さないと決めたものは、理由を設計書の末尾に 1 行ずつ残す。

## Scope

- **In**: `crates/areka-impl-watch/src/` と `tests/` の上の (a)〜(i)。直した振る舞いに合わせた `doc/impl-watch.md` と `completed/areka-P0-impl-watch/` の文書への追い書きは行わず、この spec の文書に記録する（完了した spec の文書は動かさない）。手順書は、利用者に見える文が変わったときだけ直す。
- **Out**: スキルの書き替え（`areka-P0-impl-watch-skills`）。新しいコマンド・新しい規則。状態ファイルの版を上げる変更（要るなら別に起票する）。

## Boundary Candidates

- ログに残す訳（(a)）。
- 重複の一本化（(b)(c)(d)）。
- 待ちの記録の後始末と残りの窓（(e)(f)）。
- 要件の字と実物の突き合わせ（(g)）。
- テストの穴（(h)(i)）。

## Out of Boundary

- 判断の核（`plan::apply`）の規則。出力の文の形（`granted …`・`queued … pos=<n>`・`gone: …`・`not applied: …`）は変えない。変わるのはログの行と、内側のまとめ方だけ。
- `--help` の文（スキルが読む。変えるなら `areka-P0-impl-watch-skills` と時機を合わせる）。

## Upstream / Downstream

- **Upstream**: `areka-P0-impl-watch`（完了）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-impl-watch`（完了済み。文書は動かさず、この spec が引き継ぐ）。
- **Adjacent**: `areka-P0-impl-watch-skills`（出力の文と `--help` を読む側）。`areka-P0-actor-thread-log-capture`（記録の捕捉の道具。(i) で使えるかを見るだけで、触らない）。

## Constraints

- 端末へ出す文は ASCII だけ（構造テスト `main_layering_tests.rs` が見張る）。`plan.rs` は `std::fs`・時計・`tracing` を綴らない。
- 1 ファイル 1,000 行以内（今の最大は `cli.rs` の 825 行）。テストは兄弟のファイルへ置く。
- exe を子プロセスに立てるテスト・手走りは、置き場所をワークツリーの `target\` の下の Windows 形の絶対パスで必ず上書きする（開発者の機械には本物の `AREKA_IMPL_WATCH_HOME` が設定されている）。
- 変異の戻しに `git checkout` を使わない（控えを `target\` に取り、素のコピーで戻して sha256 を比べる）。
- 規模の見立て: S〜M（8〜12）。
