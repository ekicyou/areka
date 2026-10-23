# 調べもの: mcp-strict-errors（ギャップ分析）

> 2026-10-23・`/kiro-validate-gap`（`/kiro-start` の中の段）。測った main は `15d8b0f3`＋初期化のコミット `6234da4a`。行の番号はこの時点のもの。
> SSP では測っていない（開発者の許しが要るため）。SSP の答えは `doc/ssp-mcp/survey.md` と各差の一覧に書かれた実測だけを使った。

---

## 1. 分析の要約

- **入口で確かめる形（案 1）の材料は全部そろっている**。判定は `crates/areka/src/mcp/check_script_judge.rs` の `diagnose`（純粋な関数）、事実の写しは `crates/areka/src/mcp/check_script.rs` の `ScriptFactsSnapshot`、別のスレッドで仕上げて答える口は `crates/areka/src/mcp/dump_surface.rs` の `start`／`reply_elsewhere`／`finish`、エラーログへ残す口は target `areka::log::error`（`crates/areka/src/log_history.rs` の `TARGET_ERROR`）、「最後に振った番号」は `log_history::last_id()`、`since_id` を答えに載せる前例は `crates/areka/src/mcp/reload.rs`。足すのは「つなぐ処理」と記録の本文の形だけ。
- **案 1 では kanade の `schedule/sstp.rs` に触らなくてよい**。strict の値は MCP の入口（`Args.strict`）で分かり、受け付けたかどうかは今の `SstpSendOutcome::Accepted`／`SstpNotifyOutcome::Script(台本)` で分かる。kanade の控え `StrictNote` は今のまま記録とテストだけが読む。
- **触るファイルのうち、ウェーブの約束の一覧の外に出るものが 2〜3 つある**: `crates/areka/src/mcp/check_script.rs`（事実を写す関数 `ScriptFactsSnapshot::copy` が非公開）・`doc/ssp-mcp/bridge-diff-areka.md`（要件 6.7 が名指し）・中継の `crates/areka-mcp-bridge/src/{stdio,errors}.rs` と `tests/real_receiver_test.rs`（「中継の答え」に含めて読むなら一覧の内）。どれも C11 のほかの 7 本の持ち物と重ならない。
- **`params` がオブジェクトでない要求は、rmcp 3.5.0 の中では直せない**。rmcp は本文を `ClientJsonRpcMessage` に読めないと、handler に届く前に 415 の平文を返し、差し込む口が無い。直す場所は受け口の `crates/areka-mcp/src/dispatch.rs`（rmcp へ渡す前）と、中継の `stdio.rs`（rmcp の型に読めなかった行への答え）の 2 か所。どちらも「rmcp が読めなかった本文」だけを見る形にでき、答えは rmcp の型（`ErrorData::invalid_params`・`ServerJsonRpcMessage`）で書ける。
- **調べて分かった注意**: ⑴ JSON-RPC 2.0 の字面では、数・文字列・真偽値の `params` は `-32600`（要求の形が誤り）、配列の `params` は `-32602` に当たる（要件 6.1 は全部 `-32602`）。⑵ 再生の側は既に、無い surface・無いバルーン・アンカーの崩れ・読めない `choicetimeout` を `error!`／`warn!` でエラーログへ残している＝strict の記録と同じ誤りが 2 件ずつ並ぶ（ゴースト名で絞れば strict の分だけになる）。⑶ エラーログは種別ごとに 1,000 件まで＝1 つの台本で 1,000 件を超える誤りは古い方が消え、要件 1.3 の「すべて返す」が守れない。⑷ 文面の定数 `MSG_*` 10 個は今日の `areka-tools.md` と 1 バイトも違わない（一致を見るテストは無い）。

---

## 2. 今のコードの姿

### 2.1 MCP の入口（`crates/areka/src/mcp/`）

| ファイル | 行 | 今していること | 本 spec との関係 |
|---|---|---|---|
| `sakurascript.rs` | 112 | 空の台本を断る → kanade へ `KanadeMsg::SstpSend { script, strict, reply }` → `later` で返事を毎フレーム覗く。`Accepted` なら `OK`（本文は空）、`NotAccepted`・`Failed`・返信端の脱落は `NG:` | strict 真の `Accepted` の腕を「事実を写す → 別のスレッドで診断して記録 → `OK:strict errors will be recorded …(since_id=<n>)`」に替える |
| `raise_event.rs` | 95 | 引数の検査 → `KanadeMsg::SstpNotify` → `Script(台本)` なら 2 行の答え、`NoScript` なら `OK:the ghost returned no script.` | strict 真の `Script(台本)` の腕で、返った台本を同じく診断して記録してから、今と同じ文で答える |
| `check_script.rs` | 138 | `dump_surface::start` を使い、UI スレッドで事実（シェルとバルーンの面の ID・scope 0 の別名の表）を写し、別のスレッドで `parse_noted` → `ConsumerLedger::canonical()` → `diagnose` → `render` | 事実を写す `ScriptFactsSnapshot::copy` は**非公開**（`fn copy`）。窓の無いゴーストの見分け（`Emo2Wiring` が無い）と装着待ち（`attached()` が偽なら次のフレーム）もここ |
| `check_script_judge.rs` | 218 | `diagnose(script, reads, ledger, facts) -> Vec<Diagnostic>` と文面の定数 `MSG_*` 10 個。判定は再生が使う関数の答えを写すだけ | strict の記録は同じ `diagnose` を引く（要件 2.6 の「同じ集まり」がこれで成り立つ） |
| `dump_surface.rs` | 437 | `Job`・`Step`・`start`・`reply_elsewhere`・`finish`（panic を受ける）・スレッド名 `mcp-encode` | 別のスレッドで仕上げる口として流用できる（どれも `pub(in crate::mcp)`） |
| `reload.rs` | 74 | 受け付けた直後に `last_id()` を 1 度読み、`since_id=<n>` を答えに載せる。makoto の縮退を `warn!(target: TARGET_ERROR, ghost = %…)` で残す | 答えの文の組み方と、エラーログへ残す書き方の前例 |
| `mod.rs` | 152 | `dispatch` が `resolved!` でゴーストを解決し、`reply.for_ghost(listed_value(ghost))` を付けて各 `handle` へ | 新しい処理のファイルを足すと `mod` の 1 行が要る（ウェーブの一覧の外）。足さずに既存のファイルに置けば触らない |

### 2.2 エラーログ（`crates/areka/src/log_history.rs`・`doc/ssp-mcp/log-convention.md`）

- target `areka::log::error` へ info 以上で出した行と、**どの target でも warn 以上の行**が `error` 種別になる（`classify`）。取り決めの target の行だけ、欄 `ghost`・`label` を記録の名と表示の語として読む。
- 行は「出したスレッドの上でその場で」積まれる（log-convention 8 節）＝別のスレッドで出し終えてから答えれば、答えの直後の `get_log` は取りこぼさない（要件 3.1 は満たせる）。
- 上限: 種別ごとに 1,000 件（古い方を捨てる）・本文 4,096 文字。
- 本文はメッセージの後に ` 名前=値`（値は `{:?}`）を続ける。台本の綴りはメッセージに入れれば逆斜線が 1 つのまま、欄に入れると 2 つになる。
- 履歴を写して確かめる口 `capture_begin`／`capture_finish` は**呼んだスレッドの行だけ**を拾う（`mcp-receiver-bugs` の議題 6 と同じ注意）。

### 2.3 再生の側が今エラーログへ出しているもの（問い d）

strict の値に関わらず、今もエラーログに入る行（どれも取り決めの target でない＝名は `[SYSTEM]`・本文は日本語）:

| 誤り | 再生の側の行 | 場所 | strict の記録と並ぶか |
|---|---|---|---|
| 今のシェルに無い数の surface（`\s[99999]`） | `error!`「apply(ShowSurface): 合成失敗 → 表示は適用前のまま」 | `crates/areka-emo-present/src/presenter/show.rs` 226 行 | 並ぶ |
| 解けない surface の名前・範囲外の数（`\s[無い名前]`・`\s[-2]`） | seriko の解決器の `error!`（Unresolved） | `crates/areka-seriko/src/actor.rs`（18 行の注記・457 行付近） | 並ぶ |
| 範囲外のバルーンの数（`\b[-2]`） | seriko の `error!`（Invalid） | 同上 | 並ぶ |
| 今のバルーンに無い ID（`\b[99]`） | 表示の層の答え（未確認） | — | **要確認** |
| アンカーの崩れ 3 つ | `warn!`（`anchor_reopened`・`anchor_unclosed`・`anchor_stray_close`） | `crates/areka-sakura/src/compile.rs` 250〜290 行 | 並ぶ |
| 読めない `\![set,choicetimeout,…]` | `warn!`（`choice_timeout_unreadable`） | `compile.rs` 206 行 | 並ぶ |
| 知らないタグ（`Instruction::Raw`） | `debug!`「M-boot 外タグを無視」＝**残らない** | `compile.rs` の最後の腕 | 並ばない |
| 誰も拾わない `\!` | 台帳の注記は「未登記＝良性未処理」。警告の行は見当たらない | `emo2_boot/consumer_ledger.rs` 268 行 | 並ばない（見た範囲） |
| 読む段の既定への落とし・閉じていない括弧・`ignored` | 読む段は印を付けるだけ（行は見当たらない） | `areka-parsers` の `parse_noted` | 並ばない（見た範囲） |

- つまり strict 真の台本では、`get_log(log_type=error, since_id=<n>)` に同じ誤りが 2 件（再生の側の `[SYSTEM]` の行と、ゴースト名の付いた strict の記録）出ることがある。**`ghost_name` で絞れば strict の記録だけになる**（再生の側の行は `[SYSTEM]`）。
- 要件 3.5 は再生の側の行を変えないと決めているので、重なりは仕様として残る。文書（`areka-tools.md`）に一言書けば AI は取り違えない。
- 再生の側の行は、strict の記録より前に出ることも後に出ることもある（再生は `Accepted` の前に始まっている）。`<n>` より前に出た再生の側の行は `since_id` で切れる。

### 2.4 kanade（`crates/areka-kanade/`）

- `schedule/sstp.rs`（150 行）の `on_send` は、受け付けたら `StrictNote { talk_id, strict }` を `State::sstp_strict` に控え、翻訳（`OnTranslate`）へ預ける。`SstpSendOutcome::Accepted` は「翻訳を済ませ、再生の開始を送った」後に返る（`sstp.rs` 8 行の注記）。
- `SstpNotifyOutcome::Script(台本)` の中身は「SHIORI が返したまま（翻訳の前）」（`sstp.rs` 19 行）＝要件 4.1 の「`raise_event` はゴーストが返した台本」とそのまま合う。
- **案 1 では kanade に触らない**（strict の値も台本も MCP の入口の手元にある）。

### 2.5 受け口と中継（`crates/areka-mcp/`・`crates/areka-mcp-bridge/`）

- 案内文: `crates/areka-mcp/src/handler.rs` 39 行の定数 `INSTRUCTIONS`（4 文）。最後の 1 文が「A tool that is not implemented yet returns a result starting with "NG:not implemented yet".」。中継は `service.rs` 408 行で同じ定数を読む＝定数から 1 文を消せば中継の組み込みの答えも同時に変わる（要件 5.2 は中継のコードに直す所が無い）。37 行の説明「英文・4 文」も直す。
- 今 `NG:not implemented yet` を返す処理は 0 本（`crates/areka/src/mcp/` に該当の文字列なし）。`areka-mcp` のテストにある `ng("not implemented yet")` は偽の受け手の見本の文で、要件 5.3 とは関係ない。
- 受け口: `dispatch.rs` の `forward` が本文を 4 MiB まで集め、記録用に `peek` で JSON を覗いてから、rmcp の `StreamableHttpService` に渡す。
- 中継: `stdio.rs` の `LineTransport::receive` が 1 行を `ClientJsonRpcMessage` に読み、読めなければ serde の誤りの分類で `-32700`（JSON でない）か `-32600`（要求の形でない・`id` は `null`）を自分で 1 行書く。`params` がオブジェクトでない行は今ここで `-32600`（`id: null`）になる。

---

## 3. 要件ごとの対応表

記号: ✅ 今あるものでできる／🔧 つなぐ処理を足す／❗ 足りない（Missing）／❓ 調べが要る（Unknown）／⛓ 制約（Constraint）

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1.1 答えの文 | SSP の文は `survey.md` 47 行に実測で在る。`reload.rs` が同じ形の文を組む | 🔧 `Accepted` の腕で文を組む。今の `sakurascript_tests.rs` の strict 真の期待（`OK`）は変わる（要件 7.7 で許される） |
| 1.2 `<n>` | `log_history::last_id()` | 🔧 記録を出す直前に 1 度読む（どのスレッドで読むかは設計） |
| 1.3 `since_id` で全部読める | 行はその場で積まれる | ⛓ 種別ごとの上限 1,000 件。1 つの台本の誤りが 1,000 件を超えると古い方が消える（❓ 上限を超えたときの扱いを要件か設計で決める） |
| 1.4・1.5 strict 偽・NG の場合 | 今の腕のまま | ✅（記録は `Accepted` の腕でだけ出す） |
| 2.1・2.2 1 件ずつ・名はゴースト名 | `warn!(target: TARGET_ERROR, ghost = %listed_value(ghost), …)` の前例（`reload.rs`） | 🔧 |
| 2.3 本文に 4 つ | — | ❓ 本文の形（下の 6 節の議題 4）。areka 本体は `serde_json` に依存していない（JSON の行にするなら `areka-mcp` 側に 1 行を作る関数を出すか、手で組む） |
| 2.4 文面が文書と 1 字も違わない | 今日は一致（5 節） | ❗ 一致を見るテストが無い。`log_history_convention_tests.rs` の「文書の表を印で挟んで読む」形が前例 |
| 2.5・2.6 順と集まり | `diagnose` が台本の順に返す | ✅（同じ関数を引けば同じ） |
| 2.7 予約の種類を出さない | `Kind` に予約の 2 つは無い | ✅ |
| 2.8 誤り 0 件なら記録 0 件 | — | ✅ |
| 3.1・3.2 記録を済ませてから答える | `finish` が仕事の後に答えを届ける形 | 🔧 `later` の覗きは「答えを返す」形なので、`Accepted` の後に別のスレッドの終わりを待つ 2 段目が要る（`dump_surface` の `Wait`／`Stage::Encoding` と同じ形） |
| 3.3・3.4 記録しない場合 | 今の腕 | ✅ |
| 3.5・3.6 今の振る舞いと文を変えない | — | ✅（足すだけ） |
| 3.7 表示と他のツールを止めない | `check_script` と同じ分け方（UI スレッドは事実の写しまで＝シェルの大きさで決まる） | 🔧 読む段・診断・記録は別のスレッドで。❓ 10 秒の上限（`NG:areka did not respond within 10 seconds`）に長い台本が当たったとき、記録は残るが答えは時間切れになる |
| 4.1 受け付けた時点の台本 | `sakurascript` は渡された台本、`raise_event` は翻訳の前の台本が手元に在る | ✅ |
| 4.2 受け付けた時点のシェルとバルーン | `ScriptFactsSnapshot::copy` | 🔧 ⛓ `copy` は非公開＝`check_script.rs` に触る（ウェーブの一覧の外）。❓ 写すのを `handle` の時（送る前）にするか `Accepted` の時にするか（間に翻訳の往復がある・その間にシェルが替わるのは別の台本か利用者の操作だけ） |
| 4.3 診ないもの | `diagnose` が既に診ない | ✅ |
| 4.4 窓の無いゴースト | `Emo2Wiring` が無い → facts を None | ✅（同じ見分け）。❓ 窓は在るが装着がまだ（`attached()` が偽）のときの扱い。`check_script` は次のフレームを待つ。kanade が受け付けた後なら起動の挨拶の後なので普段は装着済みのはず（確かめは設計で） |
| 4.5 `\e` の後ろも記録 | `diagnose` がそうしている | ✅ |
| 4.6 取り消さない | — | ✅ |
| 4.7 文書 | `areka-tools.md` ⑷・⑸ | 🔧 strict の記録の節を足し、⑸ の「`mcp-strict-errors` が作る予定」を書き直す |
| 5.1〜5.3 案内文 | 定数 1 つ | 🔧 1 文を消す・説明の「4 文」を直す。テストは定数を読むだけなので、消えたことを見る 1 行を足す |
| 6.1・6.2 受け口の `-32602` | rmcp には口が無い（4 節） | ❗ `dispatch.rs` に判定を足す。❓ 版の経路（200／400）の見分け方 |
| 6.3・6.4 中継の `-32602` | `stdio.rs` が `-32600` を自分で書いている所 | ❗ 同じ所で `id` を拾って `-32602` を返す |
| 6.5 ほかの答えは変えない | — | ⛓ 判定を「rmcp が読めなかった本文」だけに限れば、ほかの答えは通り道が変わらない |
| 6.6 `transport-diff-areka.md` | 行が無い | 🔧 1 行足す（SSP の欄は「未実測」） |
| 6.7 `bridge-diff-areka.md` | 91 行目「オブジェクトでない行と、要求の形でないオブジェクト」に今は含まれている | 🔧 ⛓ ウェーブの一覧の外のファイル |
| 7.1 表の 17 行と確かめの数 | `check_script_judge_tests.rs` の `row01`〜`row17` は `diagnose` を見る（文書は読まない） | ❗ 文書の表を読むテスト（印で挟む形）。17 行の strict の確かめは、記録を作る関数をテストのスレッドで呼んで `capture` で拾う形が前例に合う |
| 7.6 5 つの版 | `server_protocol_tests.rs`（561 行）・中継の `tests/real_receiver_test.rs`（5 つの版の比較） | 🔧 ⛓ `server_protocol_tests.rs` は 561 行＝足すなら新しい兄弟のファイルが無難（その場合 `server.rs` に `#[path]` の 1 行） |

---

## 4. `params` がオブジェクトでない要求（問い b）

### 4.1 rmcp 3.5.0 の中で何が起きるか

- `crates/areka-mcp/Cargo.toml` は `rmcp = "=3.5.0"`。ソースは `C:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\rmcp-3.5.0`。
- POST の処理 `transport/streamable_http_server/tower.rs` の `handle_post` は、Accept と Content-Type を見た後、`transport/common/server_side_http.rs` の `expect_json` で本文を `ClientJsonRpcMessage` に読む。読めなければ **415・平文 `fail to deserialize request body …`** を返す（同ファイル 254〜264 行）。handler（`on_custom_request` を含む）には届かない。
- `StreamableHttpServerConfig` に、読めなかった本文を差し替える口や、答えを差し替える口は無い（欄は keep-alive・セッション・JSON の答え・取り消し・Host／Origin の許しなど）。
- `params` がオブジェクトでないと `CustomRequest` にも読めない（`mcp-receiver-bugs` の research 3.1 と同じ結論）。
- 版の経路: 無状態版（`2026-07-28`）かどうかは、要求の `params._meta` の版（`message_has_per_request_protocol_version`）とヘッダ `MCP-Protocol-Version` で決まる（`tower.rs` 390 行 `is_legacy_request`）。`-32602` を 400 にするのは無状態版の経路だけ（`jsonrpc_http_status`・646 行）。**`params` がオブジェクトでなければ `_meta` は在り得ない**ので、「無状態版の経路」はヘッダだけで見分けることになる。

### 4.2 JSON-RPC 2.0 と SSP

- JSON-RPC 2.0（https://www.jsonrpc.org/specification）: `params` は「在るなら Structured value（配列かオブジェクト）でなければならない」。`-32600` は「送られた JSON が正しい Request object でない」、`-32602` は「メソッドの引数が誤り」。`id` を読み取れなかったとき（Parse error／Invalid Request）は `id` を `null` にする。
- 字面どおりに読むと: **数・文字列・真偽値の `params` は Request object の誤り＝`-32600`**。**配列の `params` は JSON-RPC としては正しく、MCP がオブジェクトを求めるので `-32602`**。要件 6.1 は 4 つとも `-32602`（brief の 2026-10-22 の申し送りどおり）。どちらにしても `id` が読めるなら `id` を付けて返してよい。
- MCP の決まり: 要求の `params` はオブジェクト（`{ [key: string]: unknown }`）。
- SSP: `params` がオブジェクトでない要求の答えは測っていない（`transport-diff-areka.md` に行が無い）。SSP は JSON でない本文に 400・`-32700`、配列（バッチ）に 400・`-32600` を返す（同 19・20 行の実測）＝SSP は本文の形の誤りを JSON-RPC のエラーで返す側。

### 4.3 直し方の案（受け口）

| 案 | 中身 | 良い点 | 弱い点 |
|---|---|---|---|
| **P1: rmcp の手前で、rmcp が読めなかった本文だけを見る**（`dispatch.rs` の `forward`） | `serde_json::from_slice::<ClientJsonRpcMessage>` が失敗し、かつ本文がオブジェクトで `id` を持ち、`params` が在って `null` でもオブジェクトでもないときだけ、`ServerJsonRpcMessage`（`ErrorData::invalid_params`・その `id`）を組んで返す。HTTP の状態はヘッダ `MCP-Protocol-Version` が無状態版なら 400、ほかは 200 | 判定が「rmcp が読めなかった」後なので、ほかの答えの通り道は 1 つも変わらない（要件 6.5）。答えは rmcp の型で書く。足すのは 15〜25 行と兄弟のテスト | 本文を 2 度読む（`peek` で既に 1 度読んでいる＝合わせれば 1 度で済む）。版の経路の見分けを rmcp と別に持つ＝rmcp を上げたときにずれうる（テストで 5 つの版を固定すれば赤で気付ける）。`mcp-receiver-bugs` が「案 D（手前で読む）」を退けた理由は「中継ごしで同じにならない」だったが、今回は中継も rmcp の型に読めずに自分で答える所なので、その理由は当たらない |
| P2: 本文を書き換えて rmcp に通す | `params` を `{}` に、`method` を目印の名前に替えて渡し、`on_custom_request` で `-32602` にする | HTTP の状態と版の経路を rmcp が決める | 要求の意味を書き換える小細工。無状態版では `_meta` の欠落の `-32602` が先に出て文が変わる。採りにくい |
| P3: rmcp を直す（上流への PR・`[patch.crates-io]`） | `expect_json` が読めた `id` を付けて JSON-RPC のエラーを返すようにする | プロトコルを SDK に任せる決まりに一番合う | 上流の取り込み待ち。`[patch]` は依存を足す 1 本だけの席を使い、`Cargo.lock` が動く（版が合わないと黙って crates.io 版を引く罠もある）。このウェーブでは採れない |
| P4: 受け口は 415 のまま、中継だけ直す | — | 触る所が少ない | 要件 6.1 を満たさない |

- 「`id` の無い要求（通知）」「`params` が `null`」は今のまま（要件 6.5）＝P1 の判定から外す。
- `message` の文: 要件 6.2 は「未登録のツール名に答える `-32602` と同じ形」＝番号と欄の組（`code`・`message`・`data` なし）。未登録のツール名の文は rmcp の `tool not found`、型の合わない要求は `invalid params for <method>`（`handler.rs` の `custom_request_error`）。どの文にするかは設計で決める（`invalid params for <method>` に揃えるのが自然。メソッドが読めないときの文も要る）。

### 4.4 直し方の案（中継）

- `stdio.rs` の `receive` で、serde の分類が `Data` のとき（今 `-32600` を返す所）に、行をもう 1 度 `serde_json::Value` として読み、オブジェクトで `id` が在り `params` が `null` でもオブジェクトでもなければ、その `id` で `-32602` を返す。答えは rmcp の型（`ServerJsonRpcMessage`）で書けば `id` の形（数・文字列）が要求のまま返る。ほかは今の `-32600`（`id: null`）のまま。
- 中継は areka の有無に関係なく自分で答える所なので、「受け口へ直接送ったときと同じ」（要件 6.3）は、受け口と中継が同じ判定と同じ文を持つことで成り立つ。判定を 2 か所に持たないなら、判定の関数を `areka-mcp` に置いて中継が読む（中継は既に `areka-mcp` の `INSTRUCTIONS` などを読んでいる）。
- `tests/real_receiver_test.rs` の 5 つの版の比較に 1 行足せば、要件 6.3・7.6 を本物の受け口と比べて確かめられる。

---

## 5. 文面の定数と文書の一致（問い e）

`crates/areka/src/mcp/check_script_judge.rs` の `MSG_*` 10 個を、Rust の逃がし（`\\!` → `\!`）を戻して、`doc/ssp-mcp/areka-tools.md` ⑶ の最初の表の `message` の欄とバイトで比べた（スクリプトで比較）。

| 定数 | 文書の行 | 一致 |
|---|---|---|
| `MSG_UNKNOWN_TAG` | `unknown_tag` | 一致 |
| `MSG_UNKNOWN_COMMAND` | `unknown_command`（`\!` の逆斜線 1 つ） | 一致 |
| `MSG_MISSING_SURFACE` | `missing_surface` | 一致 |
| `MSG_MISSING_BALLOON` | `missing_balloon` | 一致 |
| `MSG_UNCLOSED` | `unreadable_argument`（閉じていない） | 一致 |
| `MSG_DEFAULTED` | `unreadable_argument`（既定の値へ落とす） | 一致 |
| `MSG_IGNORED` | `ignored` | 一致 |
| `MSG_ANCHOR_UNCLOSED` | `unpaired_tag`（閉じが無い） | 一致 |
| `MSG_ANCHOR_REOPENED` | `unpaired_tag`（新しい開き） | 一致 |
| `MSG_ANCHOR_STRAY_CLOSE` | `unpaired_tag`（開いていない所の閉じ） | 一致 |

- 10 個と 10 行が 1 対 1 で、今日は 1 バイトも違わない。種類の名前（`Kind::as_str`・`crates/areka-mcp/src/tools/check_script.rs`）も表の 7 つと同じ。
- ただし一致を見るテストは無い（`check_script_judge_tests.rs` は定数を読むだけで文書を読まない）。要件 2.4・7.1 のために、文書の表を印（`<!-- … -->`）で挟んでテストが読む形（`log_history_convention_tests.rs` の前例）を足すのが近道。印を足すのは `areka-tools.md`（一覧の内）。
- brief の古い記述「10-08 の申し送りの表に `unpaired_tag` の行が無い」は、requirements.md が 7 種類を挙げているので、もう要件には響かない。

---

## 6. 記録すると雑音になる種類か（問い c）

SSP で問題なく動く台本に、strict 真で出うる記録:

| 種類 | 出る例 | SSP では | 雑音か |
|---|---|---|---|
| `ignored` | `\f[sub,1]`・`\f[align,center]`・`\f[height,large]`・`\![*]` | 効く（SSP の strict は「解釈に失敗」だけを記録するので、これは出ないはず） | **一番雑音に近い**。台本は正しく、areka が表示を変えないだけ。ただし「台本が黙って一部だけ動く」は本 spec の問題そのものなので、知らせる意味はある。AI が台本を「直して」しまわないよう、文書に「areka の未対応で、台本の誤りではない」と書く必要がある |
| `unknown_tag`（`\i[…]`・`\&[…]`） | `\i[5]`・`\&[amp]` | 効く | areka が本当に捨てている（再生が SSP と違う）＝自動の試験では知りたい差。雑音ではない |
| `unknown_command`（`\![raise,…]` など） | `\![raise,OnX]` | 効く | 同上（areka が誰も拾っていない） |
| `unreadable_argument` | `\_w[abc]`・`\s[0`・`\![set,choicetimeout,abc]` | SSP もたぶん既定に落とすか捨てる（測っていない） | 台本の誤りに近い。雑音ではない |
| `unpaired_tag` | `\_a[x]`（閉じ無し） | SSP は表示の終わりまで伸ばすと思われる（測っていない） | 書き手の誤りに近い。再生の側も既に `warn!` を出している（2.3 節）＝重なる |

- v0.1.0 のゴール（AI が「エラーログが増えていない＝誤りが無い」と判断する）から見ると、`ignored` を記録に入れると、SSP で正しい台本でも 0 件にならない場合がある。
- 実在の検体（窓際のぱすたさん）の台本がこれらのタグを使うかは確かめていない（❓ `check_script` に検体の台本を通せば数えられる。SSP は要らない）。

---

## 7. 案 1 と案 2（問い a）

### 案 1（入口で確かめる・要件の今の形）

- **記録できるもの**: `check_script` が診るもの全部（⑶ の表の 17 行・7 種類）。`\e`・`\-` の後ろも（アンカーの崩れを除く）。
- **記録できないもの**: シェル・バルーン・ゴーストを切り替えた後ろの `\s`・`\b`、名前の形の `\b[名前]`、窓を持たない話し手での `\s`・`\b`、`\![raise,…]` で呼んだ先の台本、`OnTranslate` で書き換わった後の台本、字面で決まらない引数の誤り（⑷）。
- **逆に、再生されない所も記録する**: `\e` の後ろの誤り（SSP の strict は再生中に起きた誤りだけのはず）。
- **触るファイル**: `crates/areka/src/mcp/{sakurascript,raise_event}.rs` と兄弟のテスト・`check_script_judge.rs`（文書の表を読むテスト・記録を作る関数を置くならここ）・`check_script.rs`（`copy` を開く 1 語か、写しを `check_script_judge.rs` へ移す）・`crates/areka-mcp/src/handler.rs`（案内文）・`crates/areka-mcp/src/dispatch.rs` と兄弟のテスト（`params`）・中継の `stdio.rs`・`errors.rs`・兄弟のテスト・`tests/real_receiver_test.rs`・`doc/ssp-mcp/{areka-tools,transport-diff-areka,bridge-diff-areka}.md`。kanade には触らない。
- **タスクの数**: 8〜11（記録を作る関数と文書の表の検査 2・`sakurascript` 1〜2・`raise_event` 1・案内文 1・受け口の `params` 1〜2・中継の `params` 1・文書 1）。
- **brief の Desired Outcome との関係**: brief の「再生中に 6 類の失敗が起きた箇所ごとに記録」を**字面どおりには満たさない**（入口で字面から確かめる・再生中ではない）。requirements.md は Introduction と要件 4 で、この差を「記録できること・できないこと」として明記し、範囲の外へ出している＝要件としては筋が通っている。v0.1.0 のゴール（AI が自分の台本の誤りを知る）には、字面で決まる誤り（AI が書く誤りの大部分＝無い番号・綴りの誤り）で足りる見込み。

### 案 2（再生まで印を運ぶ）

- **記録できるもの**: 再生中に実際に起きた誤り（切替の後ろの `\s`・`\b`、名前の形の `\b`、翻訳の後の台本、`\![raise]` の先は除く見込み）。SSP の strict の意味に一番近い。
- **記録できないもの**: `\e` の後ろ（再生しないので）。読む段で決まる種類（`ignored`・`unreadable_argument`）は、再生の側に印を運んでも検出点を新しく作ることになる。
- **触るファイル**: 案 1 の大半に加えて `crates/areka-talk/src/lib.rs`（`StartTalk` に印・`script-security-level` の `origin` の隣が前例）・kanade の `msg.rs`・`schedule/sstp.rs`・`schedule/translate.rs`（348 行）・`crates/areka-sakura/src/{compile,drive}.rs`・`crates/areka-seriko/src/{actor,resolve}.rs`・`crates/areka-emo-present/src/presenter/show.rs`・`crates/areka/src/emo2_boot/consumer_ledger.rs`（496 行）と、検出点の表と一致の検査（新）。
- **タスクの数**: 13〜19。
- **C11 での可否**: `consumer_ledger.rs`（① `descript-window-keys`）・kanade の `msg.rs`（② `mouse-click-wheel-events`）・`presenter/show.rs` を約束で触らないので、このウェーブでは採れない（採るなら足し分を次のウェーブ以降へ）。

### 案 3（組み合わせ・段を分ける）

- このウェーブは案 1（要件の今の形）。再生中にしか分からない種類（切替の後ろの `\s`・`\b`、名前の形の `\b`）だけを、後のウェーブで案 2 の形で足す（要件の Out of scope に書いてある方向）。後で足すときは、strict の記録の種類と文面を同じにし、入口の記録と再生中の記録が同じ誤りを 2 度出さない約束（例: 入口は字面で決まるものだけ、再生中は字面で決まらないものだけ）を決める。
- 良い点: 今のウェーブで v0.1.0 のゴールに要る分が着地し、残りの差は文書に書かれて見える。弱い点: 記録の出どころが 2 つになり、後の spec で重なりの約束が要る。

---

## 8. 規模と危うさ

- **規模: S〜M**（8〜11 タスク）。今ある判定・写し・別のスレッドの口・エラーログの口をつなぐだけ。新しい型や依存は無い（`areka` に `serde_json` を足さない形で書ける）。
- **危うさ: 低〜中**。低い理由＝判定は `check_script` と同じ関数で、記録の口は取り決めどおり。中にする理由＝⑴ `params` の直しは rmcp の外で版の経路（200／400）を見分けるので、rmcp を上げたときのずれをテストで押さえる必要がある ⑵ 答えを返す前に別のスレッドの終わりを待つ 2 段の覗きを `sakurascript`・`raise_event` に足す（`dump_surface` の `Wait` が前例）。

---

## 9. 設計と要件の討議への申し送り

### 決める候補（要件の討議に回す）

1. **案 1 だけで行くか（要件の今の形）**: 事実＝案 1 は 8〜11 タスクで C11 の約束の内に収まり、案 2 は `consumer_ledger.rs`・kanade の `msg.rs`・`presenter/show.rs` に触るので C11 では採れない。brief の「再生中に記録」は字面どおりには満たさないが、要件 4 が差を明記している。→ 推奨: 案 1 で進め、再生中にしか分からない種類は後のウェーブの別 spec（案 3）として起票するかを、完了の棚卸で決める。
2. **`params` がオブジェクトでない要求の番号**: 事実＝JSON-RPC 2.0 の字面では数・文字列・真偽値は `-32600`、配列は `-32602`。MCP は `params` をオブジェクトと決めている。SSP は測っていない。今の中継は `-32600`（`id: null`）。→ 推奨: 要件どおり 4 つとも `-32602`（その `id` 付き）。理由＝クライアントから見て「引数の形を直せ」が一番伝わり、受け口と中継で 1 つの番号に揃う。字面との差は `transport-diff-areka.md` の行に一言書く。字面に厳密に合わせたいなら「数・文字列・真偽値は `id` 付きの `-32600`、配列は `-32602`」に分ける案もある。
3. **受け口の直し方**: 事実＝rmcp 3.5.0 には差し込む口が無い（415 は handler の前）。→ 推奨: 案 P1（`dispatch.rs` で、rmcp が読めなかった本文だけを見て、rmcp の型で答える）。判定の関数は `areka-mcp` に 1 つ置き、中継も同じ関数を使う。上流 rmcp への PR は別に出してよい（取り込まれたら P1 を消す）。版の経路はヘッダ `MCP-Protocol-Version` で見分け、5 つの版のテストで rmcp の未登録の名前の答えと同じ状態になることを固定する。
4. **strict の記録の本文の形（要件 2.3）**: 事実＝本文はメッセージ＋` 名前=値`（値は `{:?}`＝逆斜線が 2 つになる）。`check_script` の 1 行は JSON。`areka` は `serde_json` に依存していない。→ 推奨: メッセージに `<kind> <start>-<end> <綴り>: <message>` の形で書く（綴りの逆斜線が 1 つのまま読める・依存を足さない）。JSON の 1 行にそろえたいなら、`areka-mcp` の `check_script.rs` に 1 件を 1 行にする関数を出して使う（ウェーブの一覧の外のファイルが 1 つ増える）。
5. **表示の語（`label`）**: 事実＝SSP の strict の記録の表示の語は測っていない。取り決めの既定は `Error`。→ 推奨: 既定の `Error` のまま（`label` を付けない）。SSP を測る許しが出たら合わせる。
6. **`ignored` を strict の記録に入れるか（要件 2.6 は 7 種類全部）**: 事実＝SSP で正しく効く台本でも出る（`\f[sub,1]` など）＝「誤りが無ければ 0 件」にならない台本がある。一方で「黙って一部だけ動く」を知らせる意味はある。→ 推奨: 要件どおり入れる（`check_script` と同じ集まり・判断の物差しを 1 つにする）。文書に「`ignored` は areka の未対応で、台本の誤りではない」と書く。v0.1.0 の試験で邪魔になると分かれば、そのとき外す。
7. **再生の側の行との重なり（要件 3.5）**: 事実＝無い surface・バルーン（一部要確認）・アンカーの崩れ・読めない `choicetimeout` は、今も再生の側が `[SYSTEM]` の名でエラーログへ残す。strict の記録はゴースト名が付くので `ghost_name` で絞れば分かれる。→ 推奨: 重なりは変えず（要件 3.5 のまま）、`areka-tools.md` の strict の節に「同じ誤りが再生の側の記録としても出ることがある。strict の記録だけを読むには `ghost_name` で絞る」と書く。
8. **1 つの台本の誤りが 1,000 件を超えたとき（要件 1.3）**: 事実＝エラーログは種別ごとに 1,000 件で古い方を捨てる。→ 推奨: 要件 1.3 に「エラーログの上限（1,000 件）を超えた分は古い方から消える」と書き足すか、strict の記録に上限（例: 1 つの台本で 100 件・超えた分は件数だけを 1 件で残す）を置くかを決める。後者は記録の数が `check_script` と食い違う（要件 2.6 と衝突）ので、前者（上限を明記）を推す。
9. **触るファイルの一覧（並走の約束）**: 事実＝要件どおりに作ると、ウェーブの一覧の外のファイルとして `crates/areka/src/mcp/check_script.rs`（`copy` が非公開）と `doc/ssp-mcp/bridge-diff-areka.md`（要件 6.7）が要る。中継の `stdio.rs`・`errors.rs`・`tests/real_receiver_test.rs`・受け口の `dispatch.rs` と兄弟のテストは「受け口と中継の答え」に入る。kanade の `schedule/sstp.rs` は要らない。C11 のほかの 7 本はどれもこれらに触らない。→ 推奨: 一覧に足すことを要件の討議で確認する（新しい処理のファイルを足さず、`mcp/mod.rs` には触らない）。

### 設計の段で決めること

- 事実を写す時（送る前の `handle` か、`Accepted` の後か）と、`<n>` を読むスレッド。
- 2 段の覗き（kanade の返事 → 別のスレッドの終わり）の形。`dump_surface` の `Wait` を流用するか、`sakurascript.rs` に小さく書くか。
- 記録を作る関数の置き場（`check_script_judge.rs` か `sakurascript.rs`）と、17 行の確かめをテストのスレッドの `capture` で書く形。
- 文書の表を印で挟んで読むテスト（要件 2.4・7.1）の置き場。
- `params` の判定の関数の置き場（`areka-mcp` に 1 つ置いて受け口と中継で使う）と、`message` の文。
- 受け口の新しいテストの置き場（`server_protocol_tests.rs` は 561 行）。
- strict の記録の本文の形（上の候補 4。要件 2.3 は「4 つを読み取れる」だけを決める。推しは `<kind> <start>-<end> <綴り>: <message>` をメッセージに書く形）。（2026-10-23 要件の討議で設計へ回した）
- 表示の語（`label`）（上の候補 5。推しは既定の `Error`。SSP の語は測っていない）。（同上）

### 調べもの（Research Needed）

- 今のバルーンに無い ID（`\b[99]`）を再生したとき、エラーログに行が出るか（2.3 節の「要確認」）。
- 窓は在るが装着がまだのゴーストに、kanade が台本を受け付けることがあるか（要件 4.4 の境）。
- 長い台本（例: 数 MB）で、診断と記録が 10 秒の上限に収まるか（`check_script` と同じ上限）。
- 窓際のぱすたさんの台本に `ignored`・`unknown_tag`・`unknown_command` の出るタグがどれだけあるか（`check_script` に通せば数えられる・SSP は要らない）。
- SSP の strict の記録の表示の語・本文・`since_id` の意味（測るなら開発者の許しを取る）。

### 次の段

要件の討議（`/kiro-requirements-discussion mcp-strict-errors`）で上の 1〜9 を決め、その後 `/kiro-design mcp-strict-errors`。
