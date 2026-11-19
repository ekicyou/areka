# 設計の検証: audit-test-kits

- 日付: 2026-11-17
- 対象: `.kiro/specs/audit-test-kits/design.md`（コミット `a6d896fd`）
- 入力: `spec.json`（language=ja・phase=design-generated）・`requirements.md`・`design.md`・`research.md`・`.kiro/steering/`（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C22 の行）
- 進め方: `kiro-validate-design` の手順（分析 → 重大な問題 → 強み → GO/NO-GO）。対話なしで判定し、問いは「討議への申し送り」に書いた。

## 設計の検証の要約

76 件の指摘の全部に、直し方・主なファイル・縛るテスト・束が 1 行ずつあり、直さずに残すのは裁定どおり指摘 75 の 1 件だけである。外への約束の変更は読み手つきの表に分かれ、C22 の境と持ち主の行も字面まで避けている。抜き取りで照らした主張はコードと合った。残る問題は 3 件で、どれも表の行の追記か小さな直しで閉じられ、設計の討議で決められる。

## 照らした事実（抜き取り）

| # | 設計の主張 | 照らした結果 |
|---|---|---|
| 1 | D1: rmcp `=3.9.0` の `ServiceError::McpError(ErrorData { code: ErrorCode(i32), message, .. })` を 1 つの腕だけ照らす | 合う。`ServiceError` は `#[non_exhaustive]`（`rmcp-3.9.0/src/service.rs:78-97`）。`McpError` は `ErrorData` の別名（同 54 行）。`ErrorCode(pub i32)`・`message: Cow<'static, str>`（`model.rs:632`・`655-665`）。`Link = RunningService<RoleClient, BridgeClient>`（`areka-mcp-bridge/src/scan.rs:20`）なので、`call_tool` の誤りは `ServiceError` で届く。今は `mcp.rs:184` で `Outcome::Transport` に落ち、198-215 行で猶予を待つ。指摘 3 の事実どおり |
| 2 | D1 の試験: 偽の子の `echo` に `{"text":5}` を送ると `-32602` が返る | 合う見込み。areka-mcp は inputSchema に合わない引数を `ErrorData::invalid_params` で返す（`handler.rs:71`）。rmcp の `jsonrpc_http_status` は `INVALID_PARAMS` を、`_meta` が壊れていなければ 200 で返す（`streamable_http_server/tower.rs:801`）。そのため HTTP の誤りではなく `ServiceError::McpError` として届く |
| 3 | D2: `RELIED_TARGETS` の各項から `crates/<crate>/src/<段>.rs` を引き、頼る語があることと `target:` の上書きが無いことを確かめる | 組める。`RELIED_TARGETS = ["areka_mcp::server", "areka::app_exit"]`（`launch.rs:362`）。引く先の `crates/areka-mcp/src/server.rs` と `crates/areka/src/app_exit.rs` は在る。頼る語 `event = "app_exit"`（130 行）・`event = "os_close_request"`（293 行）・`"MCP: 待受を始めた"`（`server.rs:78`・80）も在る。両ファイルに `target:` は無い |
| 4 | D6: `std::io::stderr()` へ直に書けば通ったテストでも残る。書き先を引数で受ければ決定論で読める | 合う。libtest が捕まえるのは `print!`・`eprint!` の経路だけ。削除を拒む共有モード（`OpenOptionsExt::share_mode`）で開いたファイルがあると、`remove_dir_all` は毎回同じに失敗する |
| 5 | 指摘 9 と `dropped_listener_is_unreachable` は同じ束で直す（29） | 合う。今の試験は `fake("listen-drop")`（窓が無く `hang()`）を閉じずに捨てる（`fake_mcp_test.rs:93-102`）ので、9 の後は必ず赤になる。設計は 29 を同じ束 R2 に入れ、`listen-drop;window` と閉じる形へ改める。ほかの試験は 9 の後も赤にならない。`exit:3`・`child_dying_in_call` は子が既に終わっている。`hang` の締切・`kept_on_panic_child` は panic の途中。`drop_closes_running_window` は窓が閉じる。範囲の外で取っ手を使う 6 ファイルも、全部 `close()` か `wait_exit()` を呼ぶ。設計の Risks は 3 本だけを挙げるが、`stay_on_top_real_run.rs`・`window_link_real_run.rs` も呼ぶ。早く抜ける道（`emo2_mcp_real_run.rs:133`）は子が終わった後に当たる |
| 6 | D8: `[[example]] test = true`・`autoexamples = false` で見本の単体テストを常時の段で回す | 組める。全体テストの段は `cargo test --workspace`（`tools/test-all.ps1:75`）で、`test = true` の見本はこの段で回る。`fold_one(root, scratch, source, check_only)` は根と作業の置き場を引数で受けるので、`WorkDir` の下で回せる。`autoexamples = false` は、兄弟の `fold-samples_tests.rs` が別の見本と見なされないために要る。`examples/<名>/main.rs` の形にすると `vendors/sample_ghost/README.md:186` の名指しが古びるので、設計の選びは妥当。`test = true` の前例は `crates/areka-placement/Cargo.toml:51` |
| 7 | C22 の境（要件 7.3）: 範囲の外で直す 8 ファイルは、ほかの 5 本の触るファイルに当たらない | 合う。`roadmap.md:231` の①〜⑥と突き合わせた。`scale_tests.rs` は①の `presenter_test_support.rs`・`presenter/hit.rs` と別。`attach_wiring_test.rs` は③の `actor*.rs` と別。`wintf/src/ecs/test_support.rs` は②の `ecs/drag/`・`mouse_move*` と別。`window_link_real_run.rs`・`areka-nar/src/error_tests.rs`・`doc/testing.md` は 5 本の誰も触らない |
| 8 | 1,000 行の余裕（要件 6.1） | 余裕あり。いちばん近い `sample_path_guard_test.rs` は 926 行で、1 で約 +8、33・34 で減る。受け皿の `workspace_scan/mod.rs` は 517 行で、33 の移しで増える。ただし 4 つの写しを 1 つに寄せるので 800 行前後に収まる見込み。sample-ghost-kit で長いのは `lib_tests.rs` 787 行・`devroot.rs` 723 行で、どちらも足す分と消す分（59・61〜63・70）が近い |
| 9 | 76 件の網羅（要件 2.1・4.1） | 「直し方の一覧」に 1〜76 が 1 行ずつあり、75 だけが「直さずに残す」。必ず直す・直したほうがよいのバグと危ない所には、新しいテストか改めるテストの名前が付いている（例外は下の重大な問題 2） |
| 10 | D9: `.ps1` が字面で読むのは `sample-ghost-kit:` の頭と `os error 5` だけ | 合う（`tools/load-flake.ps1:194-196`） |

## 重大な問題（3 件）

🔴 **重大な問題 1**: D1 で `call` の返りが `Err` から `Ok(is_error)` へ変わる。範囲の外で `is_error` を見ない読み手を照らしていない
- **懸念**: `crates/areka/tests/emo2_mcp_real_run.rs:184-191` は、`call("get_active_ghost_list", "{}")` の `.text` を、`is_error` を見ずにゴースト名として使う。今は JSON-RPC の誤りが `Err` になり、「呼べませんでした」の panic で落ちる。D1 の後は `"MCP error -32602: …"` が空でないゴースト名として通り、後の段で的外れの文で落ちる。`stay_on_top_real_run.rs:306` と `window_link_real_run.rs:385`・603・630 は、答えを記録に書くだけである。さらに `call_until` は失敗の答えを呼び直す。そのため直らない `-32602`（引数の誤り）が、今の約 2 秒ではなく締切いっぱいまで呼び直される。
- **影響**: 要件 6.2「失敗を黙って成功にしない」に外れうる。設計の「確かめるだけで触らない（`McpError` の形が変わらない）」は、型の形しか見ていない。
- **提案**: 「外への約束の変更と読み手」の 3 の行に、`is_error` を見ない `.call(` の読み手を全部挙げる。そのうえで、`emo2_mcp_real_run.rs:185` に `is_error` の確かめを 1 行足すか（C22 の 5 本のファイルには当たらない）、足さない理由を書く。あわせて `call_until` の扱いを決める。JSON-RPC の誤りの答えを呼び直さずに、すぐ panic するかどうかである。区別の印は `ToolReply` の公開の欄に足さず、私用の印で持てる。
- **追跡**: 要件 3.2・6.2・7.6・7.7
- **証拠**: design.md「設計の決め」D1・「確かめるだけで触らないファイル」・「外への約束の変更と読み手」の 3 の行・「前提として置いた決め」1

🔴 **重大な問題 2**: 指摘 45 の「型で縛る」は縛れていない（要件 4.1 の 1 件 1 本）
- **懸念**: 45 は危ない所（子を起こした後の panic で、子が kill されずに置き去りになる）である。ところが縛るテストの欄は「型で縛る（`Run::new` が組み終えた `McpChannel` を受ける）」になっている。型が保証するのは、`Run::new` の中で組まないことだけである。呼び手が `McpChannel::new` を `cmd.spawn()` の後へ戻しても、コンパイルもテストも落ちない。
- **影響**: 要件 4.1 の「壊れたら落ちる」を満たさないまま、「縛った」と記録される（記憶の決め「正しく動くことと、固定されていることは別」）。
- **提案**: 次のどちらかを設計の討議で決める。⑴ 配線の順は判断の分岐ではないので檻にしない、と認めて欄を「—」に改め、要件 4.1 に当たらない理由を一言書く。⑵ `spawn` の後に panic しうる段を、`Run` を組み終えるまで子の守り（`KillOnDrop` と同じ形）で包み、その守りを単体テストで縛る。
- **追跡**: 要件 4.1・6.3
- **証拠**: design.md「できれば（38〜76）」の 45 の行・「Requirements Traceability」4.1 の行

🔴 **重大な問題 3**: `roadmap.md:60` の直しは「自分の行」に当たらない
- **懸念**: 73 の直しとして、`.kiro/steering/roadmap.md:60` の「例外表 5 件」の数を外すとある。この行は「生きている決まり」の 1,000 行の行で、ウェーブ全体の決まりであり、この spec の行ではない。要件 7.3 の例外は「重ならない自分の行だけ」である。このファイルは、並走する 5 本の完了と `/kiro-next` の棚卸も書き換える。
- **影響**: 要件 7.3 の境を広げて読むことになり、着地の順によっては競合しうる。2 本が同じ数を手で直すと、git は黙って通す（記憶「両枝が同じ誤った手書きの数を書くと git は黙って通す」）。
- **提案**: 設計の討議で次のどちらかに決め、「範囲の外で直すファイル」の roadmap.md の行にその扱いを書く。⑴ 開発者の承認を得て直す（`/kiro-complete` の ROADMAP の更新の時に、その 1 語だけ）。⑵ 直さずに次の棚卸（`/kiro-next`）へ申し送る。`structure.md` の 2 節は要件の討議の 4 で承認済みなので、今のままでよい。
- **追跡**: 要件 3.2・7.2・7.3
- **証拠**: design.md「範囲の外で直すファイル」の `.kiro/steering/roadmap.md` の行・「できれば（38〜76）」の 73 の行

### 重大でない気づき（実装の時に合わせればよい）

- 23 の試験は `what: "手動用の根の作り直し"`（日本語）を照らす。一方、32 は `SampleError::Io` の `what` を英語の ASCII にする。束の順（S2 → S1 → S4）では 32 が先に入るので、23 の試験は英語の `what` で書く。
- 「範囲の外で直すファイル」に、`sample_path_guard_test.rs` の新しい綴り（`\` を含む語）と、新しい `fold-samples_tests.rs` の番兵の偽のパスが当たらないかの確かめを足すとよい。登記に無い名前を使えば当たらない。

## 設計の強み

1. **網羅と追跡の形**: 76 件を同じ番号で 1 行ずつ持ち、直し方・ファイル・縛るテスト・束がそろっている。外への約束の変更は、読み手つきの表で別に持つ。持ち主の行（D3 の `event.rs:150`・`event_tests.rs:18-19`・121、D2 の `areka::app_exit` の名指し）は 1 字も変えない形で避けている。検証の段で、差分を照らす的も挙げてある。
2. **事実を先に測った決め**: D9 の `u{XXXX}` を解いた grep、D1 の `#[non_exhaustive]`、D8 の README 186 行の名指し、29 と 9 の結びなど、実装の前に要る事実を設計の段で自分で測っている。抜き取りでも、その主張は全部コードと合った。

## 最終判定

- **判定**: **GO**
- **理由**: 今のアーキテクチャとの食い違いは無く、要件の全項が設計の要素へ追跡できる。重大な問題 3 件は、どれも表の行の追記か 1〜数行の直しで閉じ、設計の構造を変えない。
- **次の段**: 設計の討議（`/kiro-design-discussion audit-test-kits`）で 3 件を議題にし、決めを design.md へ写す。その後に `/kiro-spec-tasks audit-test-kits`。

## 討議への申し送り（対話の代わり）

この検証は対話なしで回したので、次の問いを設計の討議へ渡す。

1. D1 の後、`is_error` を見ない範囲の外の読み手（`emo2_mcp_real_run.rs:185` ほか）に確かめを足すか。`call_until` は JSON-RPC の誤りを呼び直すか（重大な問題 1）。
2. 指摘 45 を「—（配線は檻にしない）」にするか、子の守りで包んでテストで縛るか（重大な問題 2）。
3. `roadmap.md:60` の数を、この spec で直すか、次の棚卸へ申し送るか（重大な問題 3）。
