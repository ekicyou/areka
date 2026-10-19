# 設計の検証: areka-P0-mcp-receiver-bugs

- 調べた日: 2026-10-18
- 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`・steering（`tech.md`・`structure.md`・`roadmap.md` の該当の行）
- 突き合わせたコード: `crates/areka-mcp/src/handler.rs`・`server_protocol_tests.rs`・`crates/areka-mcp-bridge/tests/real_receiver_test.rs`・`doc/ssp-mcp/bridge-diff-areka.md`・`crates/areka/src/mcp/dump_balloon.rs`・`dump_surface.rs`・`dump_surface_judge.rs`・`crates/log-capture-kit/src/capture.rs`・`crates/sample-ghost-kit/src/bin/nar-sample-path.rs`・rmcp 3.5.0 の元のソース（`handler/server.rs`・`model.rs`）
- 実機の areka は起こしていない。

## まとめ

件 1 の設計は、rmcp 3.5.0 の作りと今のコードに照らして正しい。rmcp の既定の `on_custom_request` は `ErrorData::new(METHOD_NOT_FOUND, method, None)` を返す（`handler/server.rs` 553〜565 行）ので、これを 1 つ上書きして 3 つのメソッドだけ `-32602` にし、ほかは既定と同じ値を返せば、要件 1.3・1.6 の「今のまま」が崩れない。件 2 の確かめの台本も、要件の条件（`target\` の下・i686 の helper・debug の記録）を手順に落としてある。直した方がよい所は 2 つで、どちらも件 2 の確かめの記録と判定の書き方の話。設計の骨組みは変わらない。

## 確かめた設計の主張（コードと合っていたもの）

- `PARAMS_REQUIRED` の 3 つの名前の定数は rmcp に在る: `InitializeResultMethod`（`model.rs` 1055 行）・`CompleteRequestMethod`（3631 行）・`CallToolRequestMethod`（4238 行）。`ConstString::VALUE`（79 行）で綴れる。
- 3 つに絞るのは正しい。`params` が欠かせない型のうち、ほかのもの（`logging/setLevel`・`prompts/get`・`resources/read`・`resources/subscribe`・`resources/unsubscribe`・`tasks/*`）は、形の正しい要求でも rmcp の既定が `-32601` を返す（`handler/server.rs` 409〜636 行・`validate_tasks_capability` も `-32601`）。`completion/complete` の既定は成功の空の答え（402〜408 行）。要件 1.1 の決め方どおり。
- `CustomRequest` は `method`・`params`・`extensions` を持つ（`model.rs` 1026 行）。判断をメソッドの名前だけで決める設計で足りる。
- `capture` は呼んだスレッドだけを、TRACE を含む全部の段で拾う（`capture.rs` 92〜103 行）。判断と記録を純粋な関数に置いてテストのスレッドで呼ぶ設計（議題 6）は成り立つ。
- 中継の比較テストの `id` が 8 の行は 208 行目、`bridge-diff-areka.md` の行は 93 行目で、設計の書きと合う。説明の行は設計の「196〜197 行目」でなく **195〜196 行目**（1 行ずれ。実装のときに読み替えれば足りる）。
- `handler.rs` は今 144 行・`server_protocol_tests.rs` は 446 行で、設計の見込み（190 行ほど・560 行ほど）でも 1,000 行の目安に収まる。テストを兄弟のファイルに置く形（`#[path = "…_tests.rs"]`）は同じクレートの `check.rs`・`gate.rs` ほかと同じ。
- `handler.rs` を触る後続の `mcp-strict-errors` は roadmap で本 spec の後（`mcp-receiver-bugs` → `mcp-strict-errors`）に並んでいて、同時には走らない。
- 件 2 の断りの文 `the balloon of this scope is not ready`（`dump_balloon.rs` 55 行）・上限の文 `areka did not respond within 10 seconds`（`tools/bridge.rs` 118 行）・`nar-sample-path` の出す形（`folder=`・`balloon.<名>=`）・claudia のバルーン名 `claudia` は、コードと合う。

## 直した方がよい所（2 件）

🔴 **件 1: 時間を測る 5 回が「断り」だけでも合格になり、件 2 の止まった道（絵を読む道）を通らないまま閉じうる**
- **何が問題か**: 台本は測る 5 回が ⑴ 絵 か ⑵ 断り なら終了コード 0 とし、記録は「再現しない」で閉じる。ところが `dump_balloon` には絵を読む前に断る道がある（`dump_balloon.rs` 54 行の `the last shown picture is no longer kept` など）。待ちの段で 1 度絵が返った後に、こうした断りが 5 回続いても合格になる。件 2 で答えが返らなかったのは絵を読む道（GPU の写しの終わりを待つ所）なので、断りだけの 5 回では、その道が今は止まらないことを示せない。
- **なぜ効くか**: 要件 3.3 の「再現しない」の結論が、確かめるべき道を通らずに出てしまう。下流の spec はこの記録を「debug 版でも `dump_balloon` の絵が返る」根拠として使う（requirements の Adjacent expectations）。
- **どう直すか**: 測る 5 回のうち絵が返った回の数を記録に書く。測る 5 回に 1 回も絵が無ければ、その走行は「条件を満たせなかった」（終了コード 2）として 10 回に数えない、と台本の判定と RunRecord の結論の書き方に足す。要件 3.1 が断りの答えを認めていることは変えない（断りが混じっても、返っていれば合格）。
- **要件**: 3.1・3.2・3.3
- **設計の場所**: 「ManualRun」の Batch / Job Contract（出力・終了コード）、「RunRecord」の結論の書き方、System Flows「件 2 の確かめ」の 3 つ目の注

🔴 **件 2: 確かめに使う版が「今の main」そのものでなく、記録に main のどのコミットかが残らない**
- **何が問題か**: 要件 3.1 は「今の main から作った debug 版」、3.2 は「areka の版と元のコミット」を記録に残すとする。設計は、要件 2.7 のために件 1 の直しが入った後のコード（spec の枝の先）で回し、記録には `git rev-parse HEAD` だけを書く。枝の先のコミットは main に無いので、後から「main のどこで確かめたか」を辿れない。
- **なぜ効くか**: 記録の元のコミットが main の木の上で特定できないと、件 2 の「再現しない」がどの main に対して言えるのかが曖昧になる。並走の spec が main に入って `dump_balloon` の周りが変わったときに、確かめ直しが要るかを判断できない。
- **どう直すか**: RunRecord に、`HEAD` に加えて main との分かれ目（`git merge-base HEAD origin/main`）を書き、その間の差が `crates/areka-mcp` の件 1 の直し（と spec の文書）だけで `crates/areka/src/mcp/` に触れていないことを `git diff --stat` の結果で添える、と書き足す。
- **要件**: 3.1・3.2
- **設計の場所**: 「ManualRun」の入力と前提（「件 1 の直しが入った後のコードで回す」）、「RunRecord」の書くこと

## よくできている所

- **rmcp の用意した口を 1 つだけ使い、答えの判断を純粋な関数に出した形**。JSON-RPC を自分で読み直さず（「プロトコルは公式 SDK に任せる」の決まりどおり）、表の外は rmcp の既定と同じ値を返すので、変えないと約束した答えが構造の上で守られる。判断を `custom_request_error` に出したので、`mcp` スレッドで出る記録の問題（`capture` は呼んだスレッドだけを拾う）を、全体の捕捉を使わずに解いている。
- **調べで答えを実際に測ってから設計した所**。5 つの版×33 通りの今の答えと、`2026-07-28` の札と `_meta` を揃えた `initialize` が handler まで届くことを、ワークツリーの `target\` の下の道具で測ってある。テストの期待（旧式 200・`2026-07-28` 400、`params` なしは旧式だけ）がその測った値に基づいていて、推測で書いた行が無い。

## 判定

**GO**

- 理由: 件 1 は rmcp 3.5.0 の作りと今のコードに照らして正しく、変えない答えも構造で守られる。上の 2 件はどちらも件 2 の確かめの台本の判定と記録の書き足しで、設計の骨組み・触るファイル・依存は変わらない。
- 次の手順: 設計の討議で上の 2 件を扱い（設計に書き足すか、タスクに落とすかを決める）、その後に `/kiro-spec-tasks areka-P0-mcp-receiver-bugs` でタスクを作る。
