# SSP と areka の `sakurascript`・`raise_event` の差（2026-10-19 時点）

台本を流して見る `sakurascript` と、イベントを起こして反応を見る `raise_event` について、SSP と areka で答えが違う場合、または SSP の答えをまだ測っていない場合を行ごとに並べる。成功の本文（`OK`・`OK:the ghost returned no script.`・`OK:the ghost returned a script and it is being played.` と `Script: <返った台本>` の 2 行）、再生中や選択肢の最中に届いた台本が今の台本を止めて新しい台本を再生すること、空の `script` の `NG:script is empty` のように SSP と同じものは載せない。ただし、SSP と同じでも違うと思われやすいものは「SSP と同じ（実測）」として載せる。輸送の差は [transport-diff-areka.md](transport-diff-areka.md)、script 種別のログの取り決めは [log-convention.md](log-convention.md) にある。

- **SSP の印**: 「実測」は SSP で測ったもの（2026-10-18 に開発者の机の SSP へ SSP の MCP で当てた。測った結果の正本は `.kiro/specs/areka-P0-mcp-kanade-tools/requirements.md` の「SSP で測ったこと」の節）。「未実測」は SSP で測っていないもの。「対応物なし」は SSP にその仕組みが無いもの
- **areka の根拠**: 答えを返す処理は `crates/areka/src/mcp/sakurascript.rs` と `crates/areka/src/mcp/raise_event.rs` の `handle`。外からの台本とイベントを受け付けるかの判断は `crates/areka-kanade/src/schedule/sstp.rs` の `accepts`、`raise_event` の引数の検査は `crates/areka-kanade/src/sstp.rs` の `validate_notify`。`OnTranslate` の Reference1 の値は `crates/areka-kanade/src/schedule/events.rs` の `REF1_SSTP_SEND_OWNED`、script 種別のログの表示の語は `crates/areka-kanade/src/schedule/script_log.rs`

| 項目 | areka | SSP | SSP の印 |
|---|---|---|---|
| 起動の挨拶・終了の挨拶・切り替えのお別れの台詞の最中に届いた呼び出し（ゴーストは居るが普段の状態でない） | 台本を流さず SHIORI も呼ばず、`NG:the ghost cannot accept a script now`・`isError: true` で答える（areka の文言）。手順の途中に外の台本が割り込むと、手順の台詞と後始末の順序が崩れるため。待てば普段の状態に戻るので、後で送り直せる | 測っていない | 未実測 |
| ゴーストの切替の途中に届いた呼び出し（ゴーストの置き場が空） | 宛先が居ないときと同じ `NG:`（`ghost_name` を渡していれば `NG:Cannot find active ghost from specified name`、省略なら `NG:Specified ghost is not active`）・`isError: true`。`get_status` と同じ答え | 測っていない | 未実測 |
| `raise_event` の `event` が空文字・空白だけ | SHIORI を呼ばず、ツールの結果の `NG:event is empty`・`isError: true` で答える。引数の検査をプロトコルの層へ足さず、ツールの中で答えるため | ツールの結果でなく、JSON-RPC のエラー `Invalid params` で答える | 実測 |
| `raise_event` のイベント名・Reference に CR・LF・NUL が入っているとき | SHIORI を呼ばず、`NG:event or reference contains a control character`・`isError: true` で答える。断るのは CR・LF・NUL の 3 つだけで、ほかの制御文字（バイト値1 など）はそのまま送る。SHIORI への要求は 1 行に 1 つの見出しで、改行が入ると見出しが割れるため。NUL は areka が欠番の印に使う字のため | 測っていない | 未実測 |
| 中断の無効化モード（`\![enter,nouserbreakmode]`）の最中に届いた台本 | SSP と同じ（実測）。今の台本を止めて、新しい台本を再生する（中断の無効化は利用者のクリックによる中断を止めるもので、外からの台本は止めない） | 2 本目も `OK` で、1 本目が止まり 2 本目が再生された | 実測 |
| `sakurascript` の台本を `OnTranslate` に通すときの Reference1 | `sstp-send,owned`（SSTP SEND かつ Owned SSTP の扱い）。Reference2・3 は欠番（イベントが原因でないため）。`raise_event` で返った台本は、ゴースト自身のイベントの台詞と同じに通す（Reference1 は欠番・Reference2 はイベント名・Reference3 は渡された Reference） | ukadoc では SEND の台本は翻訳を通り、Reference1 は出どころの語をカンマで並べる。SSP は MCP の台本を Owned SSTP と同じに扱う。実際に入る値は、値を記録するゴーストが要るため測っていない | 未実測 |
| `\t`（割り込み禁止）の最中に届いた台本 | areka はまだ `\t` を持たないので、この場面が起きない。`\t` を作るときは ukadoc の Owned SSTP の「割り込み禁止の状態でも割り込んで再生する」に従う | ukadoc では Owned SSTP は `\t` の最中も割り込む。MCP の台本で確かめてはいない | 未実測 |
| SHIORI との往復から生まれていないトーク（選択肢の `script:` の台本など）の script 種別のログの表示の語 | `Ghost`（`[Ghost] <ゴースト名> : <台本>`）。`sakurascript` の台本は `SSTP(Local,Auth)`、イベントへの答えの台本（`raise_event` で返ったものを含む）は `Ghost:<イベント名>` で、この 2 つは SSP と同じ（実測） | `sakurascript` の台本は `[SSTP(Local,Auth)]`、`raise_event` で返った台本は `[Ghost:OnAiTalk]` と出た。往復から生まれていないトークの語は測っていない | 未実測 |
| `raise_event` の答えの 1 行目と 2 行目（`Script: …`）の区切りの字 | SSP と同じ（実測）。`\r\n` | `\r\n`（SSP 2.9.05 へ `raise_event`（`OnAiTalk`）を直に当て、生の答えの本文で確かめた） | 実測 |
