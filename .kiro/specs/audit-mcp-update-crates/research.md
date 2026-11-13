# ギャップ分析: audit-mcp-update-crates

> 2026-11-13・枝 `claude/kiro-start-audit-mcp-update-crates-dd5eaa`（HEAD `dab3f2a2`＝`main` の `4863cb66` に spec の初期化のコミットを 1 つ足しただけ。`git merge-base HEAD main` が `4863cb66` と一致＝今の `main` を取り込み済み）。
> 物差しは ponytail のプラグインの `ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.3/skills/ponytail-audit/SKILL.md`・起票の 5.3.2 から上がっているが、見る項目と 3 つの組は同じ）。
> 作法のうち 2 つを外した（要件 1.3・1.4）: 「最大 20 件」は外して全件を載せる。「読まなかった所」は残さない（範囲の 100 ファイルを全部、1 行目から最後の行まで読んだ）。
> 読み方: 本番のコード（`Cargo.toml` を含む 49 ファイル）は 1 人で全部読んだ。テストのファイル（51 ファイル）はクレートごとに 3 人の読み手に分けて全部読ませ、返ってきた指摘は、載せる前にファイルと行を開いて確かめた（確かめて外したものは §4）。外から使われているかはツリー全体を grep して確かめた（文字列での参照・ほかのクレートがソースの本文や記録の文を読む検査も含む）。テストは回していない（読みと grep だけ）。

## 1. 範囲の測り直し（要件 1.2）

| | 起票のとき（main `e3c2cbe4`） | 棚卸㊱（main `069f1a48`） | 今（`4863cb66`） |
|---|---|---|---|
| ファイルの数 | 95 | 100 | **100**（`areka-mcp` 44・`areka-mcp-bridge` 26・`areka-update` 30。fixture のファイルは無い） |
| コードの行（空行を除く・`testkit.rs` をコードに数える） | 6,054 | 5,808 | **5,808** |
| テストの行（同じ） | 12,599 | 12,390 | **12,390** |

- 数え方の注: 2 つの `testkit.rs`（`areka-mcp` 185 行・`areka-update` 178 行・どちらも `#[cfg(test)]`）をテストに回すと、本番のコードは 5,445 行（`Cargo.toml` 3 つの 124 行を含めず）・テストは 12,753 行。
- `069f1a48..4863cb66` の間に範囲のファイルは変わっていない（`git diff --stat` が空＝差分は roadmap だけ）。
- 1,000 行の近く: `areka-mcp-bridge/src/service_tests.rs` 869・`run_tests.rs`（`areka-update`）781・`http_tests.rs` 737・`fake_server.rs` 518。テストを足すなら新しい兄弟のファイルへ（要件 5.1）。

## 2. この範囲がしていること・想定した負荷

- **何をするか**: `areka-mcp` は areka 本体の中で `127.0.0.1` の 1 ポート（既定 9801→9821→隣 +1〜+9）で MCP の Streamable HTTP を待ち受け、`Origin`／`Host` を検査し、JSON-RPC の解釈は公式 SDK（rmcp 3.9.0）に任せ、SSP と同じ 10 本＋独自の `check_script` のツールの要求を UI スレッドへ橋渡しする（返事の上限 10 秒）。`areka-mcp-bridge` は Claude Desktop 向けの別の exe で、標準入出力の 1 行 1 要求を、候補のポートを並べて探して見つけた areka へ自前の HTTP/1.1 で届け、areka が居ないときは控え（`mcp-bridge\server-info.json`・`tools.json`）か組み込みの答えで答える。`areka-update` はネットワーク更新のエンジンで、`updates2.dau`／`updates.txt` を WinHTTP で取り、手元との MD5 の差分だけを取り、作業場所（`.update-work`）で揃えてから 1 件ずつ置き、失敗したら逆順に戻し、最後に `delete.txt` を当てる。
- **想定した負荷**: 利用者は 1 人。MCP のクライアントは数個（Claude Code・Cursor・Claude Desktop の中継）が同じ機械から同時に話す。中継は Desktop ごとに 1 本ずつ同時に動きうる（同じ控えのフォルダを 2 本が書く）。更新は 1 体のゴーストで 1 周ずつ、ファイルは数百まで、1 件は数 MB まで。ポートの番号は SSP と早い者勝ちで、ほかのプロセスが同じ番号を握っていることがある。テストは全体テストで多数が並んで走る（空いたポートの使い回し・机の負荷で時間が延びる）。

## 3. 指摘の一覧（全 51 件）

組の名は audit の 3 つ: **Must fix（必ず直す）**・**Should fix（直したほうがよい）**・**Nice to have（できれば）**。番号は 3 つの組を通して 1 から。

- 数: **Must fix 1 件・Should fix 38 件・Nice to have 12 件**。
- 「公開の形・範囲外」の欄: 直すと 3 つのクレートの公開の形（範囲の外から使われている名前・型・関数の形）が変わるか、範囲の外のファイルに手が及ぶか（要件 1.6）。「C19 で止まる」は要件 6.1〜6.3 の約束を破らないと直せないもの（今回は 0 件）。
- 「残す候補」の欄: 直さずに残してよい 3 種類（⑴ 開発者の裁定・完了した spec の要件に反する直し・⑵ わざと置いた先送りの印・⑶ SSP／ukadoc の正典に関わる振る舞いの変更）に当たるかもしれないもの。決めるのは要件の討議（要件 2.2）。「—」は当たらない。
- ⑶ に当たりそうな指摘は無かった（どの直しも、置き場の決め・テスト・記録の話で、MCP の道具の形・更新の手順とイベントを変えない）。そのため ukadoc は引いていない。
- 範囲の外が範囲の文を当てにしている所（直すときに壊さない・§5 の表の下にまとめた）に触れる指摘は無い。

### 3.1 Must fix（1 件）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 1 | `areka-mcp-bridge/src/config.rs:105`（`profile_env.map(PathBuf::from)`）・`:127-128`（`(Some(dir), _) => Some(dir)`）・`:21`（説明の「空の値も『設定あり』として扱うのも同じ」） | **空か相対の `AREKA_PROFILE_DIR` で、控えの置き場が Claude Desktop の作業フォルダからの相対になる**（brief の件 1）。`AREKA_PROFILE_DIR=""` なら置き場は `mcp-bridge`、`=rel` なら `rel\mcp-bridge`＝Desktop が中継を起こしたときの作業フォルダの下に控えを書く。areka 本体はこの値を `usable_profile_env`（`areka-user-data/src/lib.rs:205-215`）で「空・相対は `error!` を出して捨て、次の決めへ進む」にしたので、本体と中継で置き場が食い違う。隣に areka が居るときは、`locate_app_profile`（`establish.rs:180-185`）が同じ値を既に捨てて `error!` を 1 件残しているのに、`from_values` は捨てた値を先に採る。`LOCALAPPDATA` の段は `:132` で `is_absolute()` を見ているのに、この段だけ見ていない。テストも無い（`config_tests.rs` の 5 行の表は空・相対の `LOCALAPPDATA` だけ） | `from_inputs` で、使えない値を `from_values` に渡さない。形は 2 つ（§8 の 1・§8.1 D1）: ⒜ `areka-user-data` の `usable_profile_env` を `pub` にして呼ぶ（範囲の外の 1 行・持ち主の `audit-ghost-catalog` は完了）⒝ 同じ判定（空でなく絶対パス）を写し、捨てるときに `error!` を出す。どちらでも、隣に areka が居るときは `locate_app_profile` が既に記録しているので 2 件にしない（隣に居るときは `beside` の答えが使える値の置き場を既に含む＝`profile_dir` を渡さなくてよい）。`:21` の説明も直す。縛るテスト: 表に「空」「相対」の 2 行（隣に areka 無し → `LOCALAPPDATA` の段・隣に areka 有り → `beside` の段）と、`error!` がちょうど 1 件 | `Config::from_values` の引数の形は変えない（範囲の外で 0 か所だが `pub`）。⒜ なら `crates/areka-user-data/src/lib.rs` の 1 行（`pub(crate)` → `pub`・`pub use` に 1 語）＝要件 6.7 の許す形 | — |

### 3.2 Should fix（38 件）

#### `areka-mcp-bridge`（本番のコードの穴と、そのテスト）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 2 | `config.rs:179-181`（W3 の `warn!`）・`config_prepare_tests.rs:234`（「W1〜W3 は 0 件」） | **控えを持たずに走る理由の 1 件（W3「no place for the cache」）に、落ちるテストが無い。** `config_prepare_tests.rs` は W1・W2 の文しか持たず（`:21-24`）、`run` の道具はいつも `Some(LOCALAPPDATA)` を渡すので、隣に areka が無く `LOCALAPPDATA` も無い・相対の道を捕まえたことが無い。この `warn!` を消しても緑＝記録の無い失敗の道（控え無しで黙って走る）ができる。`:234` の注は在りもしない W3 を数える | `Config::from_inputs(None, None, None, None)` と `(…, Some("rel".into()))` を捕まえて、W3 の文の WARN がちょうど 1 件。W3 の文を定数に足し、`:234` の注を本物の 3 つにする | 公開の形は変えない・範囲の外に手は及ばない | — |
| 3 | `http.rs:360-418`（`parse_head` の頭が 64 を越える・`Content-Length` が数でない・`read_chunked` の塊の長さが 16 進でない・溢れる） | **HTTP として読めない応答を `InvalidData` にする 4 つの枝に、落ちるテストが無い。** `read_reply(&mut &raw[..], …)` で生のバイトを食わせるテストは在る（`http_tests.rs:431-450`）が、この 4 つの形は無い。悪い長さを `unwrap_or(0)` や「閉じるまで読む」に変えると、壊れた答えを黙って本文として解くようになるが、テストは緑 | `http_tests.rs` の型の表のテストを 1 本: 65 個の頭・`Content-Length: x`・塊の長さ `zz`・`ffffffffffffffffff` の 4 行で `ErrorKind::InvalidData` | 公開の形は変えない（`read_reply` は crate 内） | — |
| 4 | `tests/exe_test.rs:515-532`（`a_reply_coming_within_5s_after_stdin_closed_is_written_before_exit_code_0`） | **時間の見込みに頼るテスト。** 「標準入力を閉じてから 5 秒以内の答えは書いてから終わる」を、標準入力を書き終えてすぐ閉じ、偽の受け口が 1 秒遅れて答える形で確かめる。閉じた時点は exe が立ち上がる前で、5 秒の中に、exe の起動・`from_env`・探索・名乗りの確かめ・1 秒が全部入る。全体テストの負荷や、建てたばかりの exe をウイルス対策が調べる間に 4 秒を越えると、答えが捨てられて `answers.len()==1` が赤になる | 標準入力を閉じるのを「偽の受け口に `tools/call` が届いた後」にする（`run_bridge` に、届くのを待ってから閉じる形を足す） | 公開の形は変えない（テストの道具） | — |
| 5 | `service_tests.rs:357-375`・`service_retry_tests.rs:163-188` | **本物の 2 秒の遅れで順番を決めるテスト。** どちらも「`ping` の道が、2 秒遅れの答えより先に終わる」を当てにする。`service_retry_tests` の側はその間に、確かめの失敗・覚えを捨てる・探し直し・名乗りの確かめを全部通る。机の負荷で 2 秒を越えると、遅い答えを先に読んで赤になる | 偽の受け口に「テストが放すまで答えない」答え方（例 `Reply::Gated`・`oneshot` で放す）を足し、2 秒の遅れを置き換える | 公開の形は変えない（`tests/common/fake_server.rs` はテストの道具） | — |
| 6 | `config_tests.rs:132-145`（`from_parts_keeps_the_given_values`）・`http_tests.rs:410-427`（`delete_session`・`get_stream` の答え） | **落ちようのないテスト。** `from_parts` は名前つきの欄をそのまま詰めるだけで、入れた値が出てくるかを見ている。もう 1 本は定数を返すだけの 2 つの関数の戻りを見ている | 消す | 公開の形は変えない | — |
| 7 | `http_tests.rs:683-706`（`written_is_true_only_for_the_failures_after_the_real_body`） | **同じことを確かめるテストが 3 か所。** `HttpError::written()` の真偽は、各失敗の場面のテスト（Connect `:608`・`:613`、NotAreka `:503`・`:520`・`:549`、VerifyTimeout `:572`、NoResponse `:629`、NotJson `:658`・`:676`）がそれぞれ確かめ、`service_failure_tests.rs:190-257` も `classify` を通して同じ分け方を確かめる | 表のテストを消す | 公開の形は変えない | — |
| 8 | `config_tests.rs:113-118`・`tests/exe_test.rs:231`・`service_failure_tests.rs:252-256` | **待つ時間の定数を 3 か所で固定している。** 定数を変えると 3 か所を同じに直す手間だけが残る | `exe_test.rs` の記録の行（`connect=300ms verify=3s reply=30s`）と `Timeouts::default()` の行（`config_tests.rs:119-122`）を残し、`:115-117` を消す | 公開の形は変えない | — |
| 9 | `tests/common/fake_server.rs:142-154`（`start_on` の 0 でない番号・立て直しの輪）・`:225-227`（`connections()`）・`tests/common/fake_server_tests.rs:158-183`・`:426` | **偽の受け口の、自己確認だけが使う機能。** 番号を指定して立て直す口と接続の数の口を使うのは、偽の受け口自身のテストだけ（`src/`・`tests/` を grep）。`src/lib.rs:18`・`tests/exe_test.rs:23` の `#[allow(dead_code)]` が使われないことを隠している。その自己確認は「止めた番号で立て直す」ので、空いたポートを並んで走るほかのテストが `bind(0)` で取ると 10 秒回って panic する（#50 と同じ型） | `start_on` を `start(0)` に畳み、立て直しの節と `connections()` を消す（接続の記録は `Log.connections` を `wait` で見る形が既に覆う） | 公開の形は変えない（テストの道具）。`#[allow(dead_code)]` の 2 か所は使い手ごとに部品が違う理由の説明つき＝残る | — |
| 10 | `scan_tests.rs:27-33`・`service_tests.rs:404-410`（`nobody()`）・`http_tests.rs:600-603`・`exe_test.rs:540-543`・`real_receiver_test.rs:287-290`（同じ中身の書き下し）・`http_tests.rs:28-35`／`scan_tests.rs:254-258`（`real_receiver`）・`http_tests.rs:475-491`／`scan_tests.rs:63-73`（`only_the_check_arrived`）・`scan_tests.rs:19-24`／`service_failure_tests.rs:28-33`（`patient()`） | **テストの道具が 2〜5 か所に写されている。** 「誰も待ち受けない番号」の取り方を直すとき、写しの全部を同じに直す | `nobody()` を部品のテストと通しのテストが共有する `tests/common/fake_server.rs` へ移し、`real_receiver`・`only_the_check_arrived`・`patient` は部品のテストの束の中で 1 つにする | 公開の形は変えない | — |

#### `areka-mcp`（本番のコードの穴と、そのテスト）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 11 | `dispatch.rs:84-91`（ヘッダの値を `from_utf8_lossy` で読む） | **UTF-8 でない `Origin`／`Host` を拒む検査に、落ちるテストが無い。** 説明のとおり、読めない値を「無し」へ倒さず置換文字入りの値として `gate` に拒ませる、わざとの作り。ところがテストの道具（`testkit::request`）は `&str` のヘッダしか送れず、そういう値を送るテストが無い（`areka-mcp`・`areka-mcp-bridge` を grep）。誰かが `v.to_str().ok()` に「簡単に」すると、UTF-8 でない `Origin` が「無し」になって通る（ブラウザからの DNS の付け替えを止める検査の穴）が、テストは緑 | `testkit` にバイト列のヘッダを送る形を足し、`Origin: http://\xff` と `Host: \xff` で 403 | 公開の形は変えない（`testkit` は `#[cfg(test)]`） | — |
| 12 | `dispatch.rs:134-138`（自前の 413）・`server_protocol_tests.rs:434-446`（`body_over_limit_is_413`） | **自前の上限の枝を消しても、413 のテストは緑のまま。** rmcp にも同じ上限（`with_max_request_body_bytes(MAX_BODY_BYTES)`）を渡しているので、`Limited` の包みや `LengthLimitError` の枝を消しても rmcp が 413 を返す。テストは状態の番号しか見ていない | 本文が `Payload Too Large`・`Content-Type` が `text/plain; charset=utf-8`（自前の `plain` の形）であることも見る。ちょうど上限の本文が 413 にならないこと（1 つずれの誤り）も 1 行 | 公開の形は変えない | — |
| 13 | `server.rs:123-134`（`Drop` の `RecvTimeoutError::Timeout` の枝） | **終了の道の「待ちきれなければ切り離して先へ進む」に、落ちるテストが無い**（grep で `待ちきれなかった`・`Timeout` の試験は 0）。`drop_with_open_connection_returns_within_bound` は速く畳める道しか通らない。`recv_timeout` を `recv()` に直し間違えると、終了が止まったままになるが緑 | `server_tests.rs`（子のモジュールなので非公開の欄に届く）で、送り手を生かしたままの `Listening` を組んで落とし、`SHUTDOWN_WAIT` の前後で戻ることと WARN 1 件を見る（2 秒かかる） | 公開の形は変えない | — |
| 14 | `tools/mod.rs:119-124`（`entrances`）・`:157-167`（`register_rows`）・`tools/tools_socket_tests.rs` 全体 | **本番が組まない登録表を試すテストの束。** 本番の入口は `all_entrances`（`crates/areka/src/main.rs:236` の 1 か所）で、`entrances` を呼ぶのはこのクレートのテストだけ（ツリー全体を grep・範囲の外で 0）。`all_entrances` は `register_rows` と同じ輪を書き直していて、2 つの登録の道が並ぶ。`tools_socket_tests.rs` の 308 行は、本番が一度も作らない 10 本だけの登録表に対して、`tools_own_socket_tests.rs` とほぼ同じ往復を確かめる | `all_entrances` を `register_rows(&TABLE)` の後に `OWN_TABLE` を積む形にまとめ、`entrances` を消す。`tools_socket_tests.rs` のうち写しでない往復（9 本の型つきの呼び出し・待つ間の `ping`）は `tools_own_socket_tests.rs` へ移し、残りは消す | `entrances` は `pub` だが範囲の外で 0 か所＝要件 6.1 の「外から使われている名前」に当たらない（消してよい読み）。範囲の外に手は及ばない | ⑴（完了した `areka-P0-mcp-author-tools` の設計が「`TABLE`・`entrances`・`tools_tests.rs`・`tools_socket_tests.rs` は 1 行も変えない」と書いた＝その spec の中の約束で、後の直しを禁じる裁定ではない読み） |
| 15 | `registry_tests.rs:119-149`（`outcome_carries_text_image_and_is_error`） | **落ちようのないテスト。** テストの中で作った処理の出力を、その処理が返した値と比べるだけで、本番のコードは派生の `PartialEq` しか通らない | 消す | 公開の形は変えない | — |
| 16 | `tools/bridge_tests.rs:84-92`（`send_after_pending_dropped_does_nothing`）・`:252-262`（`send_after_timeout_does_nothing`） | **落ちようのない確かめと、2 本の重なり。** `ReplyTo::send` はどの道でも記録を出さないので `events.is_empty()` は必ず真、`answered.is_cancelled()` は `Drop` が必ず立てる。どちらも「panic しない」しか見ていない | `:259` の `is_abandoned()` の確かめだけを残し、捕捉の行と `:84-92` の 1 本を消す | 公開の形は変えない | — |
| 17 | `dispatch_shape_tests.rs:129-153`（`forward_answers_after_rmcp_415_only_for_json_bodies`） | **同じ要求を確かめるテストが別のファイルに在る。** 前半（無状態・`[1]`・400・-32602）は `server_params_shape_tests.rs:41-86` の表の 1 行、後半（`text/plain` は 415）は `server_protocol_tests.rs:188-199` と同じ要求 | 消す | 公開の形は変えない | — |
| 18 | `server_protocol_tests.rs:140-153`・`server_gate_help_tests.rs:233-246`・`tools/tools_socket_tests.rs:130-133`・`tools/tools_own_socket_tests.rs:211-214`（ほかに基準の比べ `server_protocol_tests.rs:485-497`・`server_params_shape_tests.rs:179-191`） | **「知らない道具の名前は -32602」を 4 か所以上で確かめている** | `server_gate_help_tests` と本番の登録表の `tools_own_socket_tests` の 2 本を残し、`server_protocol_tests.rs:140-153` と `tools_socket_tests.rs:130-133` を消す | 公開の形は変えない | — |
| 19 | `server_protocol_tests.rs:157-168`・`server_params_shape_tests.rs:146-161`・`server_protocol_tests.rs:531-541` | **「知らないメソッドは -32601」を 3 か所で確かめている**（真ん中の表はどの版でもエラーの本文まで見る） | `server_protocol_tests.rs:157-168` を消す | 公開の形は変えない | — |
| 20 | `testkit.rs:190-199`（`self_check_ping_on_ephemeral_port`）／`server_protocol_tests.rs:131-136`（`ping_returns_empty_object`）・`server_protocol_tests.rs:123-128`（`tools_list_is_empty`・`:219-226` と `:341-356` が覆う）・`server_protocol_tests.rs:203-208`／`server_params_shape_tests.rs:112-116`（壊れた `{` の 415） | **`ping`・空の一覧・壊れた本文の 415 が 2 か所ずつ** | `ping_returns_empty_object`・`tools_list_is_empty` と、`server_protocol_tests.rs:203-208` を消す | 公開の形は変えない | — |
| 21 | `tools/tools_socket_tests.rs:113-127`／`tools/tools_own_socket_tests.rs:119-147`（先頭の 10 本が保存した SSP の JSON と同じ）・`server_protocol_tests.rs:314-337`（`ttlMs`・`cacheScope`） | **先頭 10 本の一致を 2 か所で確かめている**（`ttlMs`／`cacheScope` も重なる） | `tools_socket_tests` の側を消す（#14 で束ごと畳むなら一緒に消える） | 公開の形は変えない | — |
| 22 | `tools/tools_tests.rs:38-43`・`:311-324`・`tools/tools_own_tests.rs:196-201`（`entrances_stays_ten`）・`check_tests.rs:262` | **「10 行」を 4 か所で固定している**（`:311-324` は `:38-43` を含み、`entrances_stays_ten` は `:311-324` に含まれる） | `tools_tests.rs:38-43` と `tools_own_tests.rs:196-201` を消す（#14 で `entrances` を消すなら後者は自然に消える） | 公開の形は変えない | — |
| 23 | `help_tests.rs:75-92`／`tools/tools_own_socket_tests.rs:258-272` | **help の「独自のツール」の節を 2 か所で確かめている。** help は登録表を読まず `OWN_TABLE` を読む（`help.rs:29-35`）ので、本番の登録表を渡しても何も足さない。HTTP で届くことは `server_gate_help_tests.rs:77-99` が見る | 通しの側（`tools_own_socket_tests.rs:258-272`）を消す | 公開の形は変えない | — |
| 24 | `tools/tools_tests.rs:278-282`（`reply_wait_is_ten_seconds`）／`tools/bridge_tests.rs:143-149` | **返事の上限 10 秒を 2 か所で固定している**（後者は文言まで見るので、値を変えればそちらも落ちる） | `tools_tests.rs:278-282` を消す | 公開の形は変えない | — |
| 25 | `server_protocol_tests.rs:37-59`・`server_gate_help_tests.rs:17-30`・`tools/tools_socket_tests.rs:20-109`・`tools/tools_own_socket_tests.rs:18-115`・`tools/tools_tests.rs:8-27`・`check_tests.rs:7-12` | **テストの道具が 2〜4 か所に写されている。** `result_of`（4 か所）・`error_of`（2）・`stateless_meta`（3）・`post_legacy`／`post_stateless`／`call_params`／`assert_no_request`（2 ファイルでバイトまで同じ）・`assert_invalid_params`（`entrances` と `all_entrances` の違いだけ）・保存した一覧の定数 `SAVED_LIST`（3）・`STATELESS`／`HELP`／待ちの定数・`obj()`（2）。新しい `_meta` の鍵を 1 か所だけに足す、といった直し漏れが起きる | `testkit.rs` に寄せる（`Response` に `result()`／`error()`・`stateless_meta`・`post_legacy`／`post_stateless`・`SAVED_LIST`・`HELP`） | 公開の形は変えない（`testkit` は `#[cfg(test)]`＝本体からは使えない） | — |

#### `areka-update`（本番のコードの穴と、そのテスト）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 26 | `winhttp.rs:191-195`（`available as usize > MAX_BODY_BYTES - body.len()`） | **本文の上限（256 MiB）を越えたら `TooLarge` で止める検査に、落ちるテストがどこにも無い**（`#[ignore]` の実機の一周にも無い）。検査を消すと、大きな本文を全部メモリへ読むようになるが緑 | 判定を純粋な関数（`fn over_limit(have: usize, available: u32) -> bool`）に出し、`winhttp_tests.rs` で上限ちょうどと 1 つ上を見る | 公開の形は変えない（crate 内の関数）。`lib_tests.rs` の文字の検査（`unsafe` は `md5.rs`・`winhttp.rs` だけ・WinHTTP の綴りは `winhttp.rs` だけ）に触れない | — |
| 27 | `lib.rs:303-314`（`fetch_manifest`） | **`updates2.dau` が 404 で、`updates.txt` が 404 でない理由で取れないときの道に、落ちるテストが無い。** ここを `ManifestMissing` に直し間違えると、ゴーストへ渡す理由（`OnUpdateFailure`）が「定義ファイルが無い」に変わるが緑 | `FakeFetch::new().fail(&url("updates.txt"), FetchError::Timeout)`（dau は 404）で `ManifestFetch { name: UpdatesTxt }`・`file()==Some("updates.txt")` | 公開の形は変えない | — |
| 28 | `lib.rs:95-102`（失敗の道で、ほかの走行の残り `residue` を `leftovers` へ） | **失敗の道の残骸の知らせに、落ちるテストが無い。** 失敗のテストはどれも `leftovers.is_empty()` を見るだけで、`:100` が `Vec::new()` を返すようにしても、`keep()` の残りを落としても緑（成功の道は `run_tests.rs:730-737` が見る） | 確定の途中で戻すテストの前に `.update-work/9-9/old/b.txt` を置き、`err.leftovers==[そのフォルダ]` と `leftover` の WARN 1 件 | 公開の形は変えない | — |
| 29 | `lib.rs:238-239`（定義ファイルを作業場所へ書けない＝`Stage::Commit` の `WorkArea`） | **この道を通るテストが無い**（`Stage::Commit` を見るのは `CommitWrite`・`RollbackFailed`・`EscapesTarget` だけ）。段を `Download` に直し間違えても緑 | 最後のファイルを取る `on_get` の差し込みで `new/updates2.dau` という名のフォルダを先に作り、`WorkArea` が `Stage::Commit` で返ること | 公開の形は変えない | — |
| 30 | `commit.rs:69-74`（外へ出るパスで止まり、しかも戻せなかった → 作った `io::Error` つきの `RollbackFailed`） | **この組み合わせの道に、落ちるテストが無い。** 「戻せなかった」を「全部戻せた」の `EscapesTarget` に取り違える直し間違いが緑で通る（戻せなかったときだけ作業場所を残す＝元の内容の在りかを失う） | `commit_tests.rs` で、1 件目の `a.bin` を `pin`（`:191-193` の形）し、2 件目をジャンクションで外へ出す（`:262-273` の形）。`RollbackFailed` で `stuck==["a.bin"]` | 公開の形は変えない | — |
| 31 | `delete.rs:25-35`（`delete_number` の大小を問わない名前）・`:47-53`（先頭の 0 を除いた数での並べ方） | **大文字の `Delete1.TXT` と、先頭が 0 の `delete02.txt` の扱いに、落ちるテストが無い。** `to_ascii_lowercase` を消すと大文字の名前の削除の指示が黙って飛ばされ、0 の扱いを変えると順が変わるが、テストは小文字の名前しか使わない | `delete_files_orders_numerically…` の名前に `Delete1.TXT`・`delete02.txt` を混ぜる（NTFS は大小を区別しないので、今の `delete1.txt`・`delete2.txt` と重ならない名前に替える）。期待の順は `delete.txt, Delete1.TXT, delete02.txt, delete10.txt` | 公開の形は変えない | — |
| 32 | `error_tests.rs:77-81`（`all_kinds_are_unique`）・`:148-153`（`every_display_is_non_empty`） | **落ちようのないテスト。** `ALL_KINDS` は変種の名前から `stringify!` で組むので、重なればコンパイルが通らない。表示はどれも空でない文字の綴りで、中身は `:109-146` が確かめる | 消す | 公開の形は変えない | — |
| 33 | `paths_tests.rs:37-49`（`local_path_is_the_target_joined_with_the_relative_name_as_is`） | **関数の本体（`target.join(rel)`）をテストの中に写して比べている**（2 つ目の確かめは std の `Path` の比べ） | 消す | 公開の形は変えない | — |
| 34 | `testkit_tests.rs:44-54`（`fake_fetch_hook_runs_before_the_answer_is_returned`） | **名前の言う「戻りより先」を見分けられない。** 印は `get` が戻った後に見るので、差し込みが戻りの後に走っても緑。`:30-42` に足すものが無い | 消す（「最中に走る」は `:30-42` と、差し込みで木を書き換える本番の道のテストが見る） | 公開の形は変えない | — |
| 35 | `lib_tests.rs:245` | `"tracing::error!(".matches("error!").count()` が 1 であることを確かめる行。std の `str::matches` を文字の綴りに当てているだけ | 行を消す | 公開の形は変えない | — |
| 36 | `error_tests.rs:72-75`・`:94-105`・`lib_tests.rs:144` | **失敗の語彙の数 11 を 3 か所で固定している** | 名前の並びの表（`:94-105`）を残し、ほかの 2 か所を消す | 公開の形は変えない | ⑴ の疑いは薄い（完了した `areka-P0-update-engine` の要件 7.4 は「11 変種で閉じる」で、数を何か所で縛るかは決めていない） |
| 37 | `error_tests.rs:240-244`（`manifest_name_file_names`） | 定数を定数と比べるだけ。2 つの名前は、どの走行のテストも `url("updates2.dau")`・`url("updates.txt")` で使う | 消す | 公開の形は変えない | — |
| 38 | `lib_tests.rs:106-127`／`run_tests.rs:618-637` | **同じ差し込み（`b` を取るときに `a.dll` を掴む）の組み立てが 2 か所に書き下されている**（`run_tests.rs:3` の「道具は 1 か所」の説明とも食い違う） | `run_tests` に組み立ての関数を 1 つ置き、両方が呼ぶ | 公開の形は変えない | — |
| 39 | `commit_tests.rs:13-35`・`delete_tests.rs:10-29`・`diff_tests.rs:12-32`・`paths_tests.rs:71-93`・`work_tests.rs:7-12`・`run_tests.rs:23-42`（`Fixture`／`fixture()`）・`commit_tests.rs:11`／`delete_tests.rs:8`（`SHARING_VIOLATION`）・`diff_tests.rs:9`／`run_tests.rs:21`（`MD5_ABC`） | **同じ形の道具（作業の根＋`target`＋`canonicalize`）が 6 か所、定数が 2 か所ずつ** | `testkit.rs` に `target_fixture() -> (WorkDir, PathBuf, PathBuf)` と 2 つの定数を置く | 公開の形は変えない | — |

- 道具の写しは、クレートごとに #10・#25・#39 に分けた（クレートをまたいで共有する道具は無い）。

### 3.3 Nice to have（12 件）

| # | 場所 | 何が壊れるか（起きる場面） | いちばん小さい直し | 公開の形・範囲外 | 残す候補 |
|---|---|---|---|---|---|
| 40 | `areka-mcp/src/dispatch.rs:140`（本文の読みの失敗 → 400） | この枝を通るテストが無い。413 に変える・空の本文で rmcp へ渡す、の直し間違いが緑 | `Content-Length: 100` で 10 バイトだけ送って書きの側を閉じ、400 | 公開の形は変えない | — |
| 41 | `areka-mcp/src/dispatch.rs:146-147`（`starts_with("application/json")`） | `application/json; charset=utf-8` を送るテストが無い。`==` に直し間違えると、文字コードを添えるクライアントは -32600／-32602 の代わりに rmcp の 415 を受けるが緑 | `server_params_shape_tests` に `charset` つきの 1 行 | 公開の形は変えない | — |
| 42 | `areka-mcp/src/server.rs:188-192`（`accept` の失敗で間を置いて続ける） | この枝のテストが無い（起こしにくい）。間を置く行を消すと、失敗が続くときに WARN と空回りの嵐になる | 受付を差し替える口が無いので、起こせる安い形が見つかったら足す（設計で確かめる） | 公開の形は変えない | — |
| 43 | `areka-update/src/lib.rs:132`（`https://` で始まる更新先） | `https://` の更新先を使うテストが 1 本も無い。`|| starts_with("https://")` を消しても緑 | 差分 0 の走行のテストを `"https://example.test/ghost/"` で 1 本 | 公開の形は変えない | — |
| 44 | `areka-mcp-bridge/src/cache.rs:52-62`（NotFound でない読みの失敗の WARN）・`service.rs:321-323`（通知を届けられなかった WARN） | 記録だけをする枝で、記録を確かめるテストが無い（前者は `cache_tests.rs:116-120` が通るが `None` しか見ない・後者は通るテストが無い） | `cache_tests.rs:116-120` を捕捉で包み、WARN「cannot read the cache」1 件を見る。後者は起こす安い形が見つかったら | 公開の形は変えない | — |
| 45 | `areka-update/src/commit.rs:143-145`（戻しで消せない親フォルダ）・`delete.rs:139`・`:144`（`resolves_under`・`symlink_metadata` の失敗 → 取り除けない）・`paths.rs:37`・`:41`（`resolves_under` の `Err`）・`work.rs:163-166`（棚の中を読めない → `held=[棚]`） | 起こしにくい失敗の枝で、テストが無い | 安い差し込みの形が見つかったら足す（設計で確かめる・見つからなければ記録に残して閉じる） | 公開の形は変えない | — |
| 46 | `areka-mcp/src/help.rs:36`・`server.rs:76`（`format!("…/api/mcp/v1")`） | 受け口のパスを、公開の定数 `MCP_PATH`（`dispatch.rs:32`）を使わずに 2 か所で手で書いている。パスを変えると help の URL と起動の記録だけが古いまま残る | 2 か所とも `MCP_PATH` から組む | 公開の形は変えない。起動の記録の文「MCP: 待受を始めた」は範囲の外（`crates/real-run-kit/src/mcp.rs:19`）が読むが、変わるのは `url` の欄の組み方だけで中身は同じ | — |
| 47 | `areka-update/src/diff_tests.rs:110-130`（作業場所へのジャンクション） | `paths_tests.rs:141-158`（`resolves_under` が偽）と `diff_tests.rs:132-154`（偽なら `Escapes`）の 2 本で覆われている | 消す | 公開の形は変えない | — |
| 48 | `areka-mcp/src/tools/tools_own_socket_tests.rs:242-254` | `assert_eq!(instructions, THREE_SENTENCES)` の後の、句の輪と `not implemented yet` の確かめは、前の `assert_eq!` が結果を決めている | `assert_eq!` だけを残す | 公開の形は変えない | — |
| 49 | `areka-mcp/src/server_tests.rs:181-188`（`drop_closes_port`） | 落とした後に、空いたばかりの番号へつなげないことを見る。並んで走るテストが `127.0.0.1:0` で同じ番号を割り当てられると赤になる（Windows は空いた番号を使い回す・まれ） | 揺れたら、新しくつなぐ代わりに `Drop` の info の記録を見る形へ | 公開の形は変えない | — |
| 50 | `areka-mcp-bridge/src/service_retry_tests.rs:35-45`（`keep_closed`）・`:80-81` | 止めた受け口の番号を取り直す間に、並んで走るテストの `bind(0)` がその番号を取ると 10 秒回って panic する。待ちは `std::thread::sleep` で、テストの tokio のスレッドを止める | 安い直しは無い。受け入れるか、覚えた先を「誰も待ち受けない番号」（`nobody()`）にして接続の失敗を起こす形へ（設計で決める） | 公開の形は変えない | — |
| 51 | 古くなった注 4 つ: `areka-mcp/src/dispatch_shape_tests.rs:127-128`（「8.2 の受け口のテスト」＝完了した `mcp-strict-errors` のタスク番号で、探せない）・`areka-update/src/winhttp.rs:3-4`（「常時テストはこのファイルを呼ばず」＝`winhttp_tests.rs` が `from_win32`・`status_to_result`・`check_url` を呼ぶ）・`areka-update/src/testkit.rs:75`（`tree` の写し元は `areka-nar/src/lib_tests.rs:63`・`install_tests.rs` は読み込むだけ）・`areka-update/src/run_tests.rs:3`（「道具は 1 か所」＝#38 の写しがある） | 読み手が在りもしない物を探す | 注を今の名に直す（`server_params_shape_tests.rs` の `non_object_params_answer_with_the_id_in_every_version`・「常時テストは純粋な判定だけを呼ぶ」・`lib_tests.rs`・#38 と一緒に） | 公開の形は変えない | — |

- 消せる見込み（Lean）: 約 −700 行（#14 の `tools_socket_tests.rs` の束 約 250・`entrances`／`register_rows` 約 20・#6〜#9・#15〜#24・#32〜#37・#47・#48 の消すテスト 約 300・#10・#25・#38・#39 の道具の写し 約 150。足すテスト（#1〜#5・#11〜#13・#26〜#31・#40〜#44）で +300〜400 行）、依存は −0。

## 4. 指摘にしなかったもの（確かめた結果）

- **`areka_update::winhttp`・`areka_update::fetch` の記録が 1 行も出ない**（`crates/areka/src/log_history.rs:116-118` の network の規則が当たる行が無い）: `doc/ssp-mcp/log-convention.md:62` に「取得の行を足したときの受け皿である」と書いた、わざとの受け皿。`areka-update` は記録を `lib.rs` の 1 か所に置く決まり（`lib_tests.rs` の文字の検査）。直すなら新しい機能＝範囲の外。**直しの約束**: `src/fetch.rs`・`src/winhttp.rs` のファイルを消す・名を変えると、`crates/areka/src/log_history_convention_tests.rs:388`（`locate("areka_update::fetch")`）が赤になる（要件 6.3）。11 行の `fetch.rs` を `lib.rs` へ畳む直しは、この理由で指摘にしない。
- **`log_history_convention_tests.rs` への響き**: 51 件のどの直しも、`areka-update` の記録の出どころ（`lib.rs` の `tracing::` の行・`[areka_update]` の文）とモジュールのファイルの名を変えない。#26 は `winhttp.rs` の中に純粋な関数を足すだけ。
- **範囲の外が読む記録の文と公開の口**: `crates/real-run-kit/src/mcp.rs:19-23` と `crates/areka/tests/emo2_real_run/common_tests.rs:142` が `server.rs` の 3 つの文（「MCP: 待受を始めた」・「MCP: どの候補のポートも束ねられなかった（待ち受けない）」・「MCP: 待ち受けない（AREKA_MCP_PORT が 0）」）を、`crates/real-run-kit/src/launch.rs:362` が記録の出どころ `areka_mcp::server` を読む。`crates/real-run-kit` は `areka_mcp_bridge::scan::{Link, Scan, scan}` と `config::Timeouts` を使う。どの直しもこれらを変えない。
- **自前の HTTP の送り手が、名乗りを確かめる前の相手の本文を上限なく読む**（`http.rs:182-200`・`fill`）: 相手は `127.0.0.1` の候補の番号に居るプロセスだけで、確かめの待ちは 3 秒で切れる。悪意のある同じ機械のプロセスは想定の負荷の外。上限なしは完了した spec の要件 1.8 の決め。
- **中継の探索が鍵を持ったまま走る**（`service.rs:90-107`）: 同時に届いた要求が探索を 2 度走らせないための設計 D-13。areka が居ない間の 1 回は、候補 20 個への接続の上限 300 ms 1 回分。
- **WinHTTP の取得に全体の期限が無い**（受信は 1 回の読みごとに 60 秒）: 1 バイトずつ 59 秒おきに送る相手で止まり続けうるが、更新は本体の働き手のスレッドで回り（`crates/areka/src/update/worker.rs`）、UI は止まらない。困る場面が起きていない。
- **更新先の `HTTP://`（大文字）を `InvalidHomeurl` にする**（`lib.rs:132`）: 綴りの大小を問わない URL の決まりからは外れるが、`descript.txt` の `homeurl` を大文字で書くゴーストを見ていない＝場面が無い。直すなら ukadoc の `homeurl` の項で確かめてからの議題（⑶ の候補）。
- **`ToolRegistry::register` の同じ名前の後勝ち**・`gate` のポートの検査（`localhost:` の空のポートを拒む）・`check::as_integer` の 2^53 の線・`request_shape_error` の `id` の型: 確かめて、場面どおりに動く。
- **`stdio.rs` の行の読みが途中で取り消されたとき**: `read_until` は読んだ分を `self.line` に残す（tokio の説明）ので行は欠けない。書きかけの行の形のエラーは `settle_reply` で待つ。
- **`commit.rs` の戻し**: 退避 → 置く → 取り消しの積みの順、`rename` が既存を置き換える前の退避、作った親フォルダの消し方を確かめた。データを失う道は見つからなかった（戻せなかったときは作業場所を残す）。
- **`service_tests.rs:452-485` の組み込みの答えの期待値**: 読み手の 1 人は「本番と同じ写し方をテストで組み直している」と挙げたが、期待値は JSON の字面で独立に書いてあり（名乗り・版・説明は公開の定数）、写し方の誤りを捕まえる。指摘にしない。
- **`commit_tests.rs:50` の「Rust 1.98」**: 較正を取ったときの版の記録で、誤りとは言えない（今の toolchain 1.100 でも較正のテストが同じ振る舞いを確かめ続ける）。
- `#[allow(dead_code)]`・`shortcut:`・`ponytail:` の印は範囲に 4 つ: `areka-mcp-bridge/src/lib.rs:18`・`tests/exe_test.rs:23`（偽の受け口を使い手ごとに部品の違う束で共有する理由つき＝#9 で中身を減らしても残る）・`redirect.rs:83` の `shortcut:`（箱の外で作った控えのフォルダの見立ての限界・今の負荷で困る場面なし）・`winhttp_real_tests.rs:32` の `ponytail:`（`#[ignore]` の実機の一周のスレッドが終わりまで残る・1 本しか立てない）。どれも書いた限界を今の負荷が越えていない＝⑵ の決め。指摘にしない。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今の資産 | ギャップ |
|---|---|---|
| 1（洗い出し） | 本書 §1・§3 | 済み（全 100 ファイルを読んだ・51 件・件 1 は #1） |
| 2（討議で確定） | 本書 §3 の「残す候補」と §8 の議題の種 | 討議で決める: ⑴ の候補 2 件（#14・#36・どちらも疑いは薄い）・⑵ 0 件・⑶ 0 件・C19 で止まる 0 件 |
| 3（直し切る） | 範囲の 3 クレート | #1〜#51 の直し（**Missing**）。件 1（3.5）は #1。範囲の外は #1 の ⒜ を選んだときの `areka-user-data` の 1 行だけ（**Constraint**） |
| 4（テストで縛る） | `log_capture_kit`・`temp-path-kit`（`target\test-roots`）・偽の受け口 `tests/common/fake_server.rs`・偽の取得口 `testkit::FakeFetch`・`pin`／`junction` の道具 | #1 の縛り（**Missing**）・#4・#5 の決定論の形（**Unknown**＝偽の受け口に「放すまで答えない」を足す形を設計で確かめる）・#11 のバイト列のヘッダを送る道具（**Missing**）。外のネットワークに出るテストは無い（`winhttp_real_tests.rs` は `127.0.0.1:0` の自前の受け口・`#[ignore]`） |
| 5（守る決まり） | 1,000 行の番人・`error!`＋`Err`・依存 0 | `service_tests.rs` 869 行＝足すなら新しい兄弟のファイルへ（**Constraint**）。#1 は `error!` で捨てる（本体と同じ）。どの直しも依存を足さない |
| 6（C19 の約束） | 棚卸㊱の約束 ⑴ | 公開の形を変える指摘は 0（#14 の `entrances` は範囲の外で 0 か所）。`crates/areka/` に手は及ばない。`Cargo.lock` に触らない |
| 7（仕上げ） | `tools/test-all.ps1` | かけ直しの audit と既定の枝との差分の確かめ（実装の後） |

- 範囲の外のファイルが範囲の文を当てにしている所（直すときに壊さない）: ⑴ `crates/areka/src/log_history_convention_tests.rs` が `areka-update/src/fetch.rs`・`winhttp.rs`・`lib.rs` の在りかを読む ⑵ `crates/areka/src/log_history.rs:116-119` が記録の出どころ `areka_update::…` を名指す ⑶ `crates/real-run-kit` が `areka_mcp::server` の出どころと 3 つの文、`areka_mcp_bridge::scan`・`config::Timeouts` を使う ⑷ `crates/areka/tests/emo2_real_run/common_tests.rs:142` が束ねの失敗の文を持つ ⑸ `crates/areka/src/main.rs:236` が `all_entrances` を呼ぶ（#14 で形を変えない）。

## 6. 1,000 行の番人

- 範囲で 1,000 行を越えるファイルは無い。近いのは `service_tests.rs` 869・`run_tests.rs` 781・`http_tests.rs` 737。#3 の表のテストは `http_tests.rs` に足しても 760 前後。#5 を `service_tests.rs` で直すなら、行は増えない見込み（遅れの答えを差し替えるだけ）。新しいテストの束を足すなら兄弟のファイル（例 `config_unusable_env_tests.rs`）へ。#25 で `testkit.rs`（185 行）に寄せると 300 行前後。

## 7. 実装の進め方の案

| 案 | 中身 | 良い | 悪い |
|---|---|---|---|
| A 今のファイルで直す | 指摘ごとに今のファイルで直す。新しいファイルは #1 のテストの兄弟 1 つ（要れば） | 層と決まりのまま・差分が小さい・消す指摘が多いので行が減る | 足すテストが多い（#2〜#5・#11〜#13・#26〜#31・#40〜#44） |
| B クレートごとに 3 つのまとまりで | A を `areka-mcp-bridge`（#1〜#10・#44・#50）→ `areka-mcp`（#11〜#25・#40〜#42・#46・#48・#49）→ `areka-update`（#26〜#39・#43・#45・#47）の順に切る（#51 は各々へ） | タスクの境目がファイルの境目と合い、並べて実装できる | — |
| C 組ごとに 2 本へ分ける | Must・Should をこの spec、Nice を同じ範囲の 2 本目へ | — | タスクは 20 に届かない見込み（下）＝分ける理由が弱い |

- 規模の見立て: 大きいタスクで 12〜14 本（#1＋#2・#3・#4＋#5・#6〜#10・#11〜#13・#14＋#21＋#22・#15〜#20＋#23〜#25・#26・#27〜#31＋#43・#32〜#39＋#47・#40〜#42＋#44〜#46＋#48〜#50・#51・かけ直し）。20 を越えない見込み＝分けなくてよい（要件 2.6）。
- Effort: **M**（本番のコードを変えるのは #1・#26（純粋な関数を出す）・#14（登録の道を 1 本に）・#46 だけ。ほかはテストの足し引き）。
- Risk: **Low**（振る舞いが変わるのは #1 の置き場の決めだけ。それも areka 本体と同じ決まりへそろえる向き）。

## 8. 討議に渡す議題の種（要件 2）

1. **#1 件 1 の直し方**: ⒜ `areka-user-data` の `usable_profile_env` を `pub` にして呼ぶ（範囲の外の 1 行）⒝ 同じ判定と `error!` を中継に写す。どちらも、隣に areka が居るときは `locate_app_profile` が既に `error!` を 1 件出すので、2 件目を出さない形にする。推奨は ⒜（判定が 1 か所に残る・前の spec の申し送りの文「`areka-user-data` の関数を呼ぶのが小さい」のとおり）。
2. **⑴ の候補 #14**（`entrances` と `tools_socket_tests.rs` を畳む）: 完了した `areka-P0-mcp-author-tools` の設計の「1 行も変えない」は、その spec の中で 10 本の形を守るための約束で、後の見直しを禁じる裁定ではない読み。推奨は畳む（本番が組まない登録表へのテスト 300 行が残り続けるため）。確認を仰ぐ。
3. **⑴ の候補 #36**（語彙の数 11 を 1 か所へ）: 完了した `areka-P0-update-engine` の要件 7.4 は数を決めたが、縛る場所の数は決めていない。推奨は名前の並びの表 1 か所。
4. **#50 の空いた番号の取り直し**: ⒜ 揺れを受け入れて残す（まれ）⒝ 覚えた先を「誰も待ち受けない番号」にして接続の失敗を起こす形へ書き直す。推奨は ⒝（テストの的「探し直す」は変わらない）。

**討議の結果（2026-11-13）**: 4 件とも推奨案で確定した（1＝⒜・2＝畳む・3＝表 1 か所・4＝⒝）。正本は `requirements.md` の「要件の討議で確定したこと」。

## 8.1 設計に回す事項

- D1（#1）: 隣に areka が居るとき・居ないときで `profile_dir` を渡すかの形と、記録の件数（ちょうど 1 件）を縛るテストの形。
- D2（#4・#5）: 偽の受け口に「テストが放すまで答えない」答え方を足す形（`Reply` の変種と放す口）と、`exe_test` の標準入力を「届いた後に閉じる」形。
- D3（#11）: `testkit` にバイト列のヘッダを送る形（今の `request` の引数の形を変えずに足す）。
- D4（#13）: `Drop` の期限切れを起こす `Listening` の組み方（送り手を生かしたまま・何もしない `spawn_actor`）。2 秒の待ちを全体テストの時間に足すことの見積もり。
- D5（#26）: `over_limit` の形と、`winhttp.rs` の中の置き場（`lib_tests.rs` の文字の検査を壊さない）。
- D6（#25・#10・#39）: 道具を寄せる先（`areka-mcp/src/testkit.rs`・`areka-mcp-bridge/tests/common/fake_server.rs`・`areka-update/src/testkit.rs`）と、寄せた後の行数。
- D7（#42・#44・#45）: 起こしにくい枝を起こす安い差し込みがあるか。無ければ「試さない」を記録に残して閉じる形（かけ直しの audit で再出したときの扱い）。

## 9. 調べが要るもの（Research Needed）

- **R1（#4・#5）**: 偽の受け口の今の答え方（`Reply::Delayed` の展開・`fake_server.rs:407`）に「放すまで待つ」を足したとき、`fake_server_tests.rs` の自己確認と `exe_test.rs` の別のプロセスから使う形（`tests/common` を通しのテストと部品のテストの両方が読む）で同じに使えるか。
- **R2（#12）**: rmcp 3.9.0 の 413 の本文と `Content-Type`（読み手の 1 人は `server_side_http.rs:225-235` で「`Payload Too Large: request body exceeds N bytes`・`Content-Type` なし」と読んだ）。設計の段で rmcp の源を開いて確かめる。
- **R3（#1）**: `locate_app_profile` が使える `AREKA_PROFILE_DIR` を受けたとき、いつも `AppProfile::Ready(<その値>)` を返すか（`establish.rs:180-215`・`lib.rs:220-` の `app_profile_dir` は値を先に採る）。返すなら、隣に areka が居るときは `profile_dir` を渡さなくても置き場は変わらない。

## 10. 設計の段の調べ（2026-11-13・`/kiro-spec-design -y`）

- 調べの型: **light**（今あるクレートの中の直し・依存の追加なし）。読んだ物: `kiro-spec-design` の規則（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）・設計の雛形・完了した `audit-sylphya-actor` の設計（形の手本）・steering（`structure.md` のテストの置き方と 1,000 行・`tech.md` のテストの決まり・`roadmap.md` の C19 の行）。ponytail の「いちばん小さい完全な変更」を各直しの形に当てた（足す口は 1 つずつ・写しは 1 か所・消せる物は消す）。
- 設計の正本は `design.md`。ここには調べの答えと、設計で決めた理由だけを残す。

### 10.1 調べの答え（§9 の R1〜R3）

- **R1（偽の受け口に「放すまで待つ」）**: 偽の受け口は自分のスレッドと自分の `current_thread` の tokio で回り、接続ごとにタスクを立てる。待つのを 1 つの要求のタスクの中の `Semaphore::acquire().await` にすれば、ほかの接続は止まらない。開ける口（`Semaphore::close`）は同期なので、部品のテスト（`#[tokio::test]`）からも通しのテスト（同期の `#[test]`・別のプロセスの exe を待つ）からも同じに呼べる。`Script`・`Reply` の `Clone`・`Debug` は `Arc` で保てる。→ `Reply::Gated`（`design.md` #4・#5）。
- **R2（rmcp 3.9.0 の 413）**: `rmcp-3.9.0/src/transport/common/server_side_http.rs` の `expect_json` は、上限を越える塊を見ると状態 413・本文 `Payload Too Large: request body exceeds {max_bytes} bytes`（4 MiB なら `4194304`）を返し、`Content-Type` を付けない。比べは `data.remaining() > max_bytes.saturating_sub(collected.len())`＝上限ちょうどは通る。自前の `plain(413)` は本文 `Payload Too Large`・`text/plain; charset=utf-8` なので、本文と札で見分けられる（`design.md` #12）。
- **R3（`locate_app_profile` と使える `AREKA_PROFILE_DIR`）**: `crates/areka-user-data/src/establish.rs` の `locate_app_profile` は、`usable_profile_env` が値を返すと `if profile_env.is_none()` の枝を飛ばし、`app_profile_dir(Some(値), ..)` が `profile_env_dir` の値を先に返す＝必ず `AppProfile::Ready(<その値>)`。だから隣に areka が居るときは、中継が `profile_dir` を渡さなくても置き場は同じで、捨てた値の `error!` は `locate_app_profile` の 1 件だけになる（`design.md` #1）。`usable_profile_env` は `lib.rs` の根に在る（`pub use` は要らない・`pub(crate)` → `pub` の 1 語）。範囲の外で今これを呼ぶのは `establish.rs` の 3 か所と `lib_tests.rs` だけ。

### 10.2 #50 の機序の測り（この机・Windows 11・2026-11-13）

- 「誰も待ち受けない番号」を覚えた先にするには、その番号で一度 areka（の偽物）が待ち受け、止めた後も番号がほかへ渡らないことが要る。PowerShell の `System.Net.Sockets.Socket`（Winsock を直に呼ぶ）で測った:
  - 待ち受けている（`SO_REUSEADDR` 無しの）口と同じ番号へ、`SO_REUSEADDR` を付けて束ねると `WSAEACCES` で断られる（後から握る形は取れない）。止めた直後に `SO_REUSEADDR` 無しの束ねをすると通る（今の `keep_closed` の取り合いが本当に起きうる）。
  - 先に `SO_REUSEADDR` を付けて束ねたまま待ち受けないソケット A を置き、同じ番号へ `SO_REUSEADDR` を付けたソケット B が束ねて待ち受けられる。B が居る間の接続は通り、B を閉じた後の接続は `ConnectionRefused`。A が居る間、`SO_REUSEADDR` 無しの束ねは断られ、`bind(0)` は別の番号を受け取る。
- → `nobody()` に `SO_REUSEADDR` を付け、`FakeServer::start_on` は `nobody()` が握る番号の上で `SO_REUSEADDR` を付けて待ち受ける。止めた後の取り直しの輪（`keep_closed`・`start_on` の 10 秒の輪）が要らなくなる。較正のテスト `start_on_a_held_port_refuses_after_stop` が、この机の振る舞いが変わったら赤にする。本番の束ね（`areka_mcp::start`）は変えない。
- 退けた案: ⑴ 覚えた先を差し込む口を中継に足す（`Linked` は rmcp の手順を通った `RunningService` で、テストから組めない・本番に口を足すことになる） ⑵ 止めた受け口を「接続をすぐ切る」形にする（確かめの途中で切れるのは `NotAreka`＝名乗りの違いと同じ道で、テストの的「接続できない」から外れる） ⑶ 揺れを受け入れる（議題 4 で取らないと確定）。

### 10.3 #45・#42・#44 の起こしにくい枝（§8.1 D7）

- 自分自身を指すジャンクション（`cmd /c mklink /J loop loop`）を開くと `ERROR_CANT_RESOLVE_FILENAME`（1921・PowerShell で HRESULT `0x80070781` を測った）。Rust はこれを `NotFound` にも `NotADirectory` にもしないので、`paths.rs` の `resolves_under` の 2 つの `Err`（`symlink_metadata`・`canonicalize`）・`delete.rs` の `resolves_under` の `Err` → 取り除けない・`work.rs` の `purge_committed` の棚を読めない枝、の 4 つを決定論で通せる。既にある道具 `testkit::junction` を使う（依存も新しい道具も要らない）。
- 差し込みが見つからなかった 2 つ（#45 の直しの「見つからなければ記録に残して閉じる」）:
  - `commit.rs` の戻しで、確定がその場で作ったフォルダを消せない枝（`NotFound`・`DirectoryNotEmpty` のほか）: 消す相手は確定の途中で作られ、確定の途中に手を入れる口が無い（取得口の差し込みは確定の前に終わる・`pin` は在るファイルにしか効かない・前もって作るとそのフォルダは「作ったフォルダ」にならない）。
  - `delete.rs` の `symlink_metadata` の `Err`（`NotFound` のほか）: 直前の `resolves_under` が同じパスの `symlink_metadata` を先に見て、失敗ならそこで取り除けない一覧へ入る。ここに来るのは、その間にほかのプロセスが木を変えたときだけ。
  - どちらもエラーの扱いは残し（要件 5.2）、その場に「試さない」の 1 行の注を置く。かけ直しの audit で同じ 2 つが出たら、#45 の直しの中として数えない（`design.md`「直さない・回した指摘」）。
- #42（`accept` の失敗）: tokio の `TcpListener::accept` の失敗は決定論で起こせない。受付を差し替える口は本番に足さず、腕の中身（`warn!` と 100 ms の待ち）を関数に出して、その関数を直に試す（指摘の危なさ＝「待ちの行を消すと嵐」をそのまま縛る）。
- #44 ⑵（通知を届けられなかった WARN）: 偽の受け口が通知に 500 の平文で答えると、自前の送り手は `NotJson` を返し、rmcp 3.9.0 の `Peer::send_notification` はその失敗を返す（`service.rs` の 834 行目・送り手の答えを `oneshot` で待つ）。通知は rmcp が別のタスクで回すが、`current_thread` の tokio では I/O を待たない道は次の I/O の待ちの前に回し切られるので、後に `ping` を 1 往復すれば記録は揃っている。

### 10.4 設計で決めたこと（§8.1 D1〜D6）

- **D1**: `from_inputs` だけを直す（`from_values` は値だけの判定のまま・`pub` の形も同じ）。隣に areka が居れば `profile_dir` は渡さず、居なければ `usable_profile_env` の答えを渡す。記録はどちらも `areka_user_data` の `error!` ちょうど 1 件。縛るテストは `config_prepare_tests.rs`（2 値 × 隣の有無）。
- **D2**: `Reply::Gated(Gate, Box<Reply>)`（`Gate` は `Arc<Semaphore>`・開けたら閉じない）。#4 は門でなく「届いた後に閉じる」（`run` に `before_close` を渡す）で直す＝5 秒の見込みから exe の起動の時間を外すのが的で、答えの 1 秒の遅れは今のまま残せる。
- **D3**: ヘッダの値をバイト列で渡す口でなく、組み上がった要求のバイト列をそのまま送る `send_raw` を 1 つ足す（#11 の UTF-8 でない値と #40 の短い本文の両方に使える・`request` は引数を変えずにその上に載せる）。
- **D4**: `McpServer` を子のモジュールのテストで直に組む（`done` の送り手をテストが持つ・`spawn_actor` の何もしない取っ手）。落とすのは別のスレッドで行い、期限つきで待つ（`recv()` に直し間違えても止まらずに赤）。全体テストへの響きは約 2 秒（並んで走る）。
- **D5**: `over_limit(have: usize, available: u32) -> bool` を `winhttp.rs` の中、`MAX_BODY_BYTES` の隣に置く。`unsafe` と WinHTTP の API の綴りを持たないので、`lib_tests.rs` の文字の検査に触れない。
- **D6**: 寄せる先は各クレートの今の道具（`areka-mcp/src/testkit.rs`・`areka-mcp-bridge/tests/common/fake_server.rs`・`areka-update/src/testkit.rs`）。中継の部品のテストどうしで借りる道具（`real_receiver`・`only_the_check_arrived`・`patient`）は、`crate::` を使えない共有のファイルに入れず、`http.rs` の `mod tests` と `service.rs` の `mod failure_tests` を `pub(crate)` にして共有元にする（`cfg(test)` の中だけ）。寄せた後の行数は `design.md`「1,000 行の番人」（どれも 1,000 行の内側・新しいファイルは要らない）。
- **#14**: `register_rows` を残して行の列（`IntoIterator`）を受ける形にし、`all_entrances` がそれを呼ぶ（登録の道が 1 本になる）。`entrances` だけを消す。`tools_tests.rs` の登録のテスト（読めない行を飛ばすテストを含む）は `register_rows` を使い続ける。

### 10.5 合成（generalization・build vs adopt・simplification）

- まとめた物: 「誰も待ち受けない番号」の 5 つの写しと、#50 の「止めた後も番号を握る」を、`nobody()` 1 つ（`SO_REUSEADDR` 付き）にまとめた。#11 と #40 の「普通でない要求を送る」を `send_raw` 1 つにまとめた。
- 使った既存の物: 門は `tokio::sync::Semaphore`（既に依存にある）・握る番号は `tokio::net::TcpSocket::set_reuseaddr`・ジャンクションは既にある `testkit::junction`。作った物は無い（依存も足さない）。
- 削った物: 受付を差し替える口（#42）・覚えた先を差し込む口（#50）・ヘッダの値のバイト列の口（#11）は作らない。`from_values` の形は変えない（#1）。新しいファイルは作らない。

### 10.6 設計の検め（review gate）

- 1 回目: 要件の 42 個の番号が全部 `design.md` に在ること、51 件が「指摘ごとの直し方」に 1 行ずつ在ることを、python で機械的に確かめた。中身の検めで 2 つ直した（`Gate::new` は clippy の `new_without_default` に当たるので `Gate::shut` へ・#44 ⑵ の記録の捕まえ方を `#[test]`＋`capture`＋`current_thread` の形と、順の根拠まで書いた）。
- 2 回目: 直しの後に同じ検めで通った。要件の段へ戻す穴は無かった。
