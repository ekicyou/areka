# Requirements Document

> 本文の実測は **2026-10-18・本ブランチ**（main `2d86dbff`）。brief は main の `e2370df8`（棚卸㉗の節を足したもの・本ブランチへはまだ取り込んでいない）も読んだ。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、brief が議題とした点と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。
> SSP の答えは、2026-10-18 に開発者の机の SSP（起動中のゴースト「悪役令嬢クローディア」「窓際のぱすたさんDEBUG」）へ SSP の MCP で当てた実測（下の「SSP で測ったこと」）と、[doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)・ツールの説明文の逐語（[doc/ssp-mcp/tools-list-ssp-2.9.05.json](../../../doc/ssp-mcp/tools-list-ssp-2.9.05.json)）・ukadoc による。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code など）でゴーストを作る人や、エージェントにゴーストを操らせる人は、「台本を流して見る」（`sakurascript`）と「イベントを起こして反応を見る」（`raise_event`）を最もよく使う。SSP ではどちらも動く。areka では 2 本とも `NG:not implemented yet` を返すので、エージェントは台本を試せず、表情の確かめや別 spec の実機の確かめ（`seriko-trigger-intervals`・`mcp-dump-images`）も代わりの手順に頼っている。

**今の状態**: ツールの入口・名前の解決・UI スレッドへの橋は `mcp-tool-entrances` が作り、アプリ本体側 `crates/areka/src/mcp/sakurascript.rs`・`raise_event.rs` の `handle` は `NG:not implemented yet` を返すダミーである。kanade（台本の実行と SHIORI の往復を受け持つ部分）には外から台本を流す経路が無い。イベントを起こす知らせ `KanadeMsg::RaiseEvent`（`crates/areka-kanade/src/msg.rs`）はあるが、許可の表（`crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS`）に無い名前を断り、結果（`crates/areka-kanade/src/change.rs` の `RaiseOutcome`）は返った台本の文字列を持たない。

**何を変えるか**: `sakurascript` の台本がゴースト自身の台本と同じ扱いで再生され、`OK` が返る。`raise_event` は任意のイベント ID と Reference の列で SHIORI を呼び、返った台本を再生し、台本の文字列を結果に含める。再生中・選択肢の最中・切替の最中に届いたときの扱いを SSP に合わせる。

> 起票: 2026-09-29 `/kiro-discovery`（SSP MCP の移植の 3 段目）。2026-10-05 棚卸㉒で `get_status` を `mcp-get-status` へ切り出した（着地済み）。版の段では v0.1.0 の MCP の列の先頭。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人・操らせる人**: 台本を送って表情や台詞の出方を見る、イベントを起こしてゴーストの反応と返った台本を見る、という SSP で普通の使い方が areka では一つもできない。
- **後続の spec の実装者**: `mcp-strict-errors`（strict の記録）・`mcp-user-response`・`script-impact-tiers`・`mcp-shiori-query`・`property-query-channels` の 5 本が本 spec を待つ。とくに「許可の表を通らない任意の名前のイベントを送る口」は `property-query-channels`・`mcp-shiori-query` が使う。
- **実機の確かめ**: `mcp-dump-images` の要件 7.7 ⑵（`sakurascript` で表情を変えてから `dump_surface` で撮る）と、`seriko-trigger-intervals` の「走行の途中で 2 本目の台本を送る」場面は、本 spec を待っている。

### 正典と SSP（ukadoc と SSP で確かめたもの）

- **SSP のツールの説明**: `sakurascript` は「スクリプトは対象ゴースト自身の処理（Owned SSTP）と同様に扱われ、`\![reload,...]` なども実行できる」（SSP ヘルプ「その他の機能」）。`raise_event` は「SSTP NOTIFY の Event 相当・ゴーストが返したスクリプトを再生する・返されたスクリプトは結果にも含まれる」。`get_status` の説明文は「talking・choosing・timecritical・opening の間に台本を送ると、割り込むか拒まれることがある」と書く。
- **SSTP（ukadoc の SSTP 仕様）**: SEND と NOTIFY は同一の処理。`Option: nobreak` が「現在再生中のスクリプトを中断せずにキューイングする」＝指定が無ければ**再生中の台本を中断する**。Owned SSTP は「`\t` などの割り込み禁止状態でも強制的に割り込み再生を可能とする」。応答は再生の終わりを待たず即時に返る（210 は限られた場面だけ）。
- **`OnTranslate` の Reference1**（ukadoc）: 出どころの語（`sstp-send`＝SSTP SEND の場合・`owned`＝Owned SSTP の場合 ほか）をカンマ区切りで並べ、該当が無ければ欠番。areka は今「常に欠番」（`doc/COMPAT_ARCHITECTURE.md` の `OnTranslate` の Reference1 の行・出どころを 1 つも持たないため）。SEND の台本は `Option: notranslate` が無ければ翻訳を通る。
- **SHIORI/3.0**: ゴーストが知らないイベントには 204（返り値なし）を返すのが普通。

### SSP で測ったこと（2026-10-18・SSP の MCP）

| # | 試したこと | SSP の答え |
|---|---|---|
| 1 | `sakurascript`（台本 1 本） | 本文 `OK`。script ログに `[SSTP(Local,Auth)] <ゴースト名> : <台本>` の 1 行 |
| 2 | 15〜20 秒待つ台本の再生中に、別の `sakurascript` を送る | 2 本目も `OK`。2 本目の script ログは 1 本目の直後に出て、短い 2 本目が終わるとすぐゴーストの自発のトークが始まった＝**1 本目は中断され、2 本目が再生された**（積まれない） |
| 3 | 選択肢を出す台本を `sakurascript` で送り（`get_status` は `choosing,balloon(0=0)`）、その最中に別の `sakurascript` を送る | 2 本目も `OK`。`get_status` は `talking,balloon(0=0)` になった＝**選択肢は取り下げられ、2 本目が再生された** |
| 4 | `sakurascript` の `script` が空文字 | `NG:script is empty`（`isError: true`） |
| 5 | `raise_event`（ゴーストが知らない名前・Reference 2 つ・`strict` の有無とも） | `OK:the ghost returned no script.` |
| 6 | `raise_event`（`OnAiTalk`・台本が返る） | 1 行目 `OK:the ghost returned a script and it is being played.`、2 行目 `Script: <返った台本>`。script ログは `[Ghost:OnAiTalk] <ゴースト名> : <台本>`（SSTP の語でなくイベント名） |
| 7 | 台本の再生中に `raise_event`（`OnAiTalk`）を送る | 6 と同じ形の答え。返った台本が再生された（中断して差し替え） |
| 8 | `raise_event` の `event` が空文字 | JSON-RPC の `Invalid params`（ツールの結果でなくエラー） |

測っていないこと: 2 行目との区切りの字（`\n` か `\r\n` か。MCP クライアントの表示からは区別できない）・切替の最中・起動の挨拶や終了の挨拶の最中・`\t` の最中・中断の無効化モードの最中・`OnTranslate` の Reference1 の実際の値（記録する検体ゴーストが要る）。

### いま何が起きているか（2026-10-18 実測）

- **MCP の側。** `crates/areka/src/mcp/sakurascript.rs`・`raise_event.rs` の `handle` は `outcome::ng("not implemented yet")` を返し、各 `_tests.rs` がその文言を期待している（本 spec が書き換える）。引数の型はプロトコル側 `crates/areka-mcp/src/tools/sakurascript.rs`（`script`・`ghost_name`・`strict: Option<bool>`）と `raise_event.rs`（`event`・`references: Vec<String>`・`ghost_name`・`strict`）にあり、完成している。後から答える口 `mcp::later`（`crates/areka/src/mcp/mod.rs`）と、それを使う手本 `get_status.rs` の `handle`（kanade へ知らせを送り、答える前にゴーストが降りたら宛先の解決の失敗と同じ文言で答える）がある。
- **kanade の側。** 台本は `Action::StartTalk` からしか始まらない。「台本 1 つを新しいトークとして始める」関数 `start_talk`（`crates/areka-kanade/src/schedule/steady_choice_script.rs`）があり、トークの番号の採番と枠の差し替えを使い回せる（再生中の置き換えのときの選択肢の帳簿の掃除は呼び手の仕事）。`KanadeMsg::RaiseEvent` は普段の状態（Steady）でしか動かず、許可の表（54 語）に無い名前を断る（送り出しの直前にもう一度照合があり、表に無い固定の名前がそこへ届くとゴーストを止める道へ進む）。kanade は普段の状態で SHIORI の台本が返ると、再生中のトークを差し替える（積まない。`OnSecondChange` だけは再生中は返り値を求めない形で送る）。SHIORI の往復から生まれていない台本（選択肢の `script:`）は今は翻訳を通らない。`RaiseOutcome` は `NotAllowed`・`NotSteady`・`Script`・`NoReply`・`Failed` で台本の文字列を持たず、読む所が複数ある（変えると波及する）。
- **翻訳。** SHIORI の台詞は再生の前に `OnTranslate` を通る（`translate-pipeline`）。Reference1 は常に欠番。
- **script 種別のログ。** 取り決めは `doc/ssp-mcp/log-convention.md`（target `areka::log::script`・欄 `ghost`・`label`）。今この target へ台本を出す本番の行は 0 で、`get_log` の `script` はいつも空。トークの起動 `StartTalk` はイベント名を持たず、kanade の起動の設定 `KanadeConfig` はゴーストの名前を持たない（棚卸㉗の brief の節）。
- **待ちの上限。** 入口の橋が 10 秒の上限（`NG:areka did not respond within 10 seconds`）と終了の途中の答え（`NG:areka is shutting down`）を持つ。kanade は SHIORI との往復の間はほかの知らせを処理しない。
- **ファイルの大きさ。** `msg.rs` 930 行・`actor.rs` 901 行・`schedule/mod.rs` 969 行・`schedule/events.rs` 850 行・`schedule/change.rs` 425 行・`schedule/steady.rs` 957 行（上限 1,000）。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 再生中に届いた台本（`sakurascript`）と、再生中に `raise_event` で返った台本 | **今の台本を中断し、新しい台本を再生する**（積まない・断らない） | SSP の実測 2・7。ukadoc の SSTP の `nobreak` が「指定すれば中断せず積む」＝既定は中断 | 3.1・3.2 |
| 2 | 選択肢を待っている最中に届いたとき | **選択肢を取り下げ、新しい台本を再生する**。取り下げた選択肢について選択や時間切れのイベントは送らない | SSP の実測 3。選択肢を出した台本が中断されたのと同じ扱い | 3.3 |
| 3 | 中断の無効化モード（`\![enter,nouserbreakmode]`）の最中 | **割り込む**（裁定 1 と同じ） | 中断の無効化は利用者のクリックによる中断を止めるもので、SSP の MCP の台本は Owned SSTP（`\t` すら越えて割り込む）として扱われる。SSP は未実測 | 3.4 |
| 4 | 切替の最中（ゴーストの置き場が空） | **宛先が居ないときと同じ文言**（`ghost_name` を渡していれば `NG:Cannot find active ghost from specified name`、省略なら `NG:Specified ghost is not active`） | `mcp-get-status` の裁定 5 と同じ答えにそろえる。SSP は未実測 | 3.5 |
| 5 | 起動の挨拶・終了の挨拶・切り替えのお別れの台詞の最中（ゴーストは置き場に居るが普段の状態でない） | **台本を流さず・SHIORI を呼ばず、`NG:the ghost cannot accept a script now`・`isError: true` で答える**（文言は areka 独自） | 起動・終了・切替の手順の途中に外の台本が割り込むと、その手順の台詞と後始末の順序が崩れる。待てば普段の状態に戻るので、エージェントは後で送り直せる。SSP は未実測＝差の一覧に書く | 3.6 |
| 6 | `sakurascript` の台本を `OnTranslate` に通すか・Reference1 に何を入れるか（brief の議題） | **通す**。Reference1 は `sstp-send,owned`（SSTP SEND かつ Owned SSTP の扱い）、Reference2・3 は欠番（イベントが原因でない）。`doc/COMPAT_ARCHITECTURE.md` の「常に欠番」の行を改める | ukadoc: SEND の台本は `notranslate` が無ければ翻訳を通り、Reference1 は該当する出どころの語をカンマ区切りで並べる。SSP は MCP の台本を Owned SSTP と同様に扱う。実際の値は SSP で未実測＝要件ディスカッションで測るか決める | 4.4 |
| 7 | `raise_event` で返った台本の翻訳 | **ゴースト自身のイベントの台詞と同じに通す**（Reference1 は今のまま欠番・Reference2 はイベント名・Reference3 は渡された Reference） | SSP の script ログの語が `Ghost:<イベント名>`（実測 6）＝ゴースト自身のイベントへの反応として扱われている | 4.5 |
| 8 | `raise_event` の結果に含める台本 | **SHIORI が返した台本そのまま**（翻訳の前） | SSP の説明文「ゴーストが返したスクリプト」。翻訳の後の台詞を待つと答えが遅れる | 2.3 |
| 9 | 許可の表に在る名前（`OnBalloonTimeout`・`OnAnchorSelect` など、kanade が普段は Reference を補って送るもの）を `raise_event` で頼まれたとき（brief の議題） | **渡された Reference をそのまま送る**（kanade は補わない・欠けた位置は送らない） | SSP の説明文「Reference0, Reference1, ... of the event」＝呼び手が組む。補うと試したい形を試せない | 2.2 |
| 10 | `raise_event` の `event` が空文字・空白だけ | **SHIORI を呼ばず `NG:event is empty`・`isError: true`** | SSP は JSON-RPC の `Invalid params`（実測 8）。引数の検査は `mcp-tool-entrances` の持ち物でプロトコル側（`crates/areka-mcp`）に触らない約束があるため、ツールの結果で答え、差の一覧に書く | 2.6 |
| 11 | イベント名・Reference に改行などの制御文字が入っているとき | **SHIORI を呼ばず `NG:event or reference contains a control character`・`isError: true`** | SHIORI/3.0 の要求は 1 行 1 見出しで、改行が入ると見出しが割れて別の見出しを送れてしまう（信頼の境界の検査）。SSP は未実測 | 2.7 |
| 12 | SHIORI の往復が失敗した（輸送路が死んだ）とき | **ゴーストが降りたのと同じ文言**（裁定 4 の文言）で答える | 往復の失敗では kanade は今日どおり終了系列へ進み、答える時点でゴーストは居ない | 2.8 |
| 13 | script 種別のログに何を出すか | **再生を始めるトークを全部**（MCP から来たものに限らない）、再生を始めるとき 1 回ずつ出す。表示の語は、`sakurascript` の台本が `SSTP(Local,Auth)`、SHIORI のイベントへの答えの台本（`raise_event` で返ったものを含む）が `Ghost:<そのイベント名>`、SHIORI の往復から生まれていないトーク（選択肢の `script:` の台本など）は `Ghost`。本文は翻訳の後の、実際に再生する台本 | SSP の実測 1・6（SSP の `script` は再生したさくらスクリプトの全部）。全部へ広げたのは棚卸㉗の brief の範囲。本文を翻訳の後にするのは同じ節の「実際に再生する台本が通る所」。往復から生まれていないトークの語は SSP で未実測 | 4.1〜4.3 |
| 14 | `strict` | **受け取って再生の頼みと一緒に下流へ渡す**。答えの文言は変えない（strict の答えの文言と記録は `mcp-strict-errors`） | brief の Scope。SSP の strict の答えの文言は survey §3 | 5.1〜5.3 |

## Boundary Context

- **In scope**:
  - `sakurascript`・`raise_event` の本物の答えと、ゴーストへの働き（台本の再生・任意の名前のイベントの送信・返った台本の再生）。
  - 再生中・選択肢の最中・中断の無効化モードの最中・切替の最中・起動や終了の手順の最中に届いたときの扱い。
  - 許可の表を通さずに任意の名前のイベントを送る口（本 spec の `raise_event` と、後で `property-query-channels`・`mcp-shiori-query` が使う）。
  - MCP から来た台本である印（翻訳の Reference1・script ログの表示の語）。
  - `strict` を受けて下流へ渡す口。
  - 再生を始めたトークを全部 script 種別のログへ出すこと（MCP から来たものに限らない・棚卸㉗で足した範囲）。
  - 決定論テスト（偽 SHIORI）・SSP との差の一覧・実機での確認。
- **Out of scope**:
  - `get_status`（`mcp-get-status` で着地済み）・`reload`（`mcp-reload`）。
  - strict の検出と記録・strict のときの答えの文言（`mcp-strict-errors`）。
  - 実行の状態の決め方（`status-execution-states` の持ち物）。
  - `\t`（タイムクリティカル）そのものと、その最中の外からの台本の扱い（`sakura-time-critical`。areka は今 `\t` を持たない。持つときは ukadoc の Owned SSTP の「割り込み禁止状態でも割り込み再生」に従うことをその spec に申し送る）。
  - SSTP の SEND・NOTIFY の受け口そのもの（予約）。`\![raise]`・`\![embed]` などのタグ。
  - ツールの定義・引数の型の検査・`ghost_name` の解決の規則・待ちの上限（`mcp-tool-entrances` のまま変えない）。
- **Adjacent expectations**:
  - `mcp-tool-entrances` から受け取るもの: 検査を通った引数・解決済みのゴースト・結果を作る関数・後から答える口・10 秒の待ちの上限と終了の途中の答え。
  - `translate-pipeline`（完了）: 再生の前の翻訳の経路。本 spec は MCP の台本の出どころの語を渡す。
  - `mcp-strict-errors`: 本 spec が下流へ渡す `strict` の印を使い、記録と答えの文言を足す。
  - `property-query-channels`・`mcp-shiori-query`: 本 spec が作る「許可の表を通さずに任意の名前のイベントを送る口」を使う。
  - `mcp-reload`: 重なり 0 の約束（本 spec は kanade の `src/change.rs`・`schedule/boot.rs` に触らない。上限の近いファイルを分けるときは型を同じ名前で見せ、`schedule/boot.rs` を変えない）。
  - 同じウェーブに置けない相手（同じファイルを触る）: `script-security-level`・`multi-ghost-runtime`・`mcp-strict-errors` ほか（roadmap の干渉台帳）。

## Requirements

### Requirement 1: `sakurascript` で台本をゴースト自身の台本と同じ扱いで再生する

**Objective:** As a AI エージェントの利用者, I want `sakurascript` で送った台本がゴースト自身の台本と同じように再生されること, so that SSP 向けの呼び方のまま、台本の見え方を試せる

#### Acceptance Criteria

1. When 宛先の解決を通った `sakurascript` の呼び出しが空でない `script` で届き、そのゴーストが普段の状態にある, the areka shall その台本をそのゴーストの新しいトークとして再生し、本文 `OK`・`isError: false` の結果で答える。
2. The areka shall 送られた台本を、ゴースト自身が SHIORI から返した台本と同じ扱いで再生する（areka が対応しているさくらスクリプトのタグと `\!` の命令は、ゴースト自身の台本で働くのと同じに働き、MCP から来たことを理由に断らない・弱めない）。
3. The areka shall 答えを、台本の再生を受け付けた時点で返し、再生の終わりを待たない。
4. If `script` が空文字である, then the areka shall 何も再生せず、本文 `NG:script is empty`・`isError: true` の結果で答える。
5. The areka shall 台本の長さを理由に断らない（長い台本でも描画・アニメーション・クリックの受け取りを止めない）。
6. The areka shall `sakurascript` に `NG:not implemented yet` を返す経路を残さない。

### Requirement 2: `raise_event` で任意のイベントを起こし、返った台本を再生して結果に含める

**Objective:** As a AI エージェントの利用者, I want `raise_event` で任意のイベントを起こし、ゴーストが返した台本を再生させてその文字列を受け取ること, so that ランダムトークや反応を試し、返った台本を読める

#### Acceptance Criteria

1. When 宛先の解決を通った `raise_event` の呼び出しが届き、そのゴーストが普段の状態にある, the areka shall `event` の名前のイベントを、`references` の各要素を順に Reference0・Reference1…として、そのゴーストの SHIORI へ 1 回送る（areka がふだん送るイベントの許可の表に無い名前でも送り、表に無い名前を送ったことを理由にゴーストを止めない・警告も残さない）。
2. The areka shall 許可の表に在る名前を頼まれたときも、渡された Reference だけをそのまま送り、ふだんそのイベントに補っている Reference を足さない。
3. When SHIORI が空でない台本を返す, the areka shall その台本を新しいトークとして再生し、1 行目 `OK:the ghost returned a script and it is being played.`・2 行目 `Script: ` に続けて SHIORI が返した台本そのまま（翻訳の前）の本文・`isError: false` の結果で答える（2 つの行の区切りの字は SSP と同じにする。SSP の生の応答で設計の段に確かめる）。
4. When SHIORI が台本を返さない（返り値なし・空か空白だけの台本・エラーの応答）, the areka shall 何も再生せず（今の再生にも触れず）、本文 `OK:the ghost returned no script.`・`isError: false` の結果で答える。
5. The areka shall 答えを、SHIORI の答えが返った時点で返し、返った台本の再生の終わりを待たない。
6. If `event` が空文字か空白だけである, then the areka shall SHIORI を呼ばず、本文 `NG:event is empty`・`isError: true` の結果で答える。
7. If `event` または `references` のどれかの要素に改行などの制御文字が含まれる, then the areka shall SHIORI を呼ばず、本文 `NG:event or reference contains a control character`・`isError: true` の結果で答える。
8. If SHIORI との往復が失敗した（輸送路が死んだ）, then the areka shall 要件 3.5 の文言で答える（kanade がその後に進む道は今日のまま変えない）。
9. The areka shall `raise_event` に `NG:not implemented yet` を返す経路を残さない。

### Requirement 3: 再生中・選択肢の最中・切替の最中に届いたときの扱い

**Objective:** As a AI エージェントの利用者, I want ゴーストの状態によらず SSP と同じ結果になること, so that 送る前に状態を見なくても結果を予想でき、`get_status` を見て送る時機を選ぶこともできる

#### Acceptance Criteria

1. While そのゴーストが台本を再生している, when `sakurascript` が届く, the areka shall 今の台本を中断し、送られた台本を再生する（積まない・断らない）。
2. While そのゴーストが台本を再生している, when `raise_event` が届く, the areka shall SHIORI を呼び、台本が返れば今の台本を中断してその台本を再生し、返らなければ今の再生を続けさせる。
3. While そのゴーストが選択肢を出して選ばれるのを待っている, when `sakurascript` が届くか `raise_event` で台本が返る, the areka shall 選択肢を取り下げて新しい台本を再生し、取り下げた選択肢について選択のイベントも時間切れのイベントも送らない。
4. While 再生中の台本が中断の無効化モードにある, when `sakurascript` が届くか `raise_event` で台本が返る, the areka shall 要件 3.1〜3.3 と同じに割り込む。
5. If 宛先を解決した後、答える前にそのゴーストが降りた（切替・終了・倒れた）、または切替の途中でゴーストの置き場が空である, then the areka shall 宛先の解決に失敗したときと同じ文言（`ghost_name` を渡していれば `NG:Cannot find active ghost from specified name`、省略なら `NG:Specified ghost is not active`）・`isError: true` で答え、台本を再生しない。
6. While そのゴーストが起動の手順（起動の挨拶を含む）・終了の手順・切替の手順（お別れの台詞を含み、終了や切替が決まって今のトークの終わりを待っている間も含む）の途中にある, when `sakurascript` か `raise_event` が届く, the areka shall 台本を再生せず SHIORI も呼ばず、本文 `NG:the ghost cannot accept a script now`・`isError: true` の結果で答え、その手順の進みを変えない。
7. While kanade が SHIORI との往復の最中にある, when `sakurascript` か `raise_event` が届く, the areka shall その往復が終わってから処理する（10 秒を超えたときの答えは `mcp-tool-entrances` の待ちの上限のまま）。
8. If `raise_event` の答えが待ちの上限で先に返った後に SHIORI の台本が返る, then the areka shall その台本を普段どおり再生する（イベントは送られており、結果だけが届かなかったものとして扱う）。
9. The areka shall 要件 3.5・3.6 の答えのときに warn 以上の記録を出さない（切替や終了の途中に呼ばれるのは普通の出来事であり、`debug!` までにとどめる）。

### Requirement 4: 再生したトークの記録と、MCP から来た台本である印

**Objective:** As a AI エージェントの利用者と areka の開発者, I want 再生したトークが SSP と同じ印で記録され、MCP の台本の出どころが翻訳にも伝わること, so that `get_log` で何がどこから再生されたかを SSP と同じ読み方で追え、ゴーストの翻訳も SSP と同じに働く

#### Acceptance Criteria

1. When ゴーストのトークの再生を始める（MCP から来たものに限らず、起動の挨拶・普段の会話・イベントへの反応・終了や切替の台詞を含む）, the areka shall script 種別のログ（`get_log` の `log_type=script` で読めるもの）に、そのゴーストの名前（`get_active_ghost_list` と同じ名前）・表示の語・翻訳の後の実際に再生する台本の 1 件を、トーク 1 つにつき 1 回だけ残す。
2. The areka shall 要件 4.1 の表示の語を、`sakurascript` の台本では `SSTP(Local,Auth)`、SHIORI のイベントへの答えの台本（`raise_event` で返ったものを含む）では `Ghost:<そのイベント名>`（例 `Ghost:OnAiTalk`）、SHIORI の往復から生まれていないトークでは `Ghost` とする。
3. If トークの再生が始まらなかった（翻訳の輸送路の失敗などで表示しなかった）, then the areka shall そのトークの script 種別のログを残さない。
4. When `sakurascript` の台本を再生する, the areka shall その台本を再生の前に `OnTranslate` へ通し、Reference1 を `sstp-send,owned` とし、Reference2・Reference3 は欠番とする。
5. When `raise_event` で返った台本を再生する, the areka shall その台本を、ゴースト自身のイベントの台詞と同じ形で `OnTranslate` へ通す（Reference1 は欠番・Reference2 はイベント名・Reference3 は渡された Reference）。
6. The areka shall MCP から来ていない台本の翻訳の Reference を、本 spec の前と変えない。

### Requirement 5: `strict` を受けて下流へ渡す

**Objective:** As a `mcp-strict-errors` の実装者, I want `strict` の値が再生の頼みと一緒に運ばれること, so that strict の検出と記録を、本 spec の答えの経路を作り直さずに足せる

#### Acceptance Criteria

1. When `sakurascript` か `raise_event` が `strict` 付きで届く（真・偽・省略のどれでも）, the areka shall 断らずに受け付け、その値を、再生する台本（`raise_event` では返った台本）と結びついた形で台本を再生する側へ渡す。
2. The areka shall 本 spec の範囲では、`strict` の値によって答えの文言・再生のしかた・記録を変えない（strict のときの答えの文言と記録は `mcp-strict-errors` が足す）。
3. The areka shall 決定論テストで、再生する側が受け取った `strict` の値が、呼び出しに渡された値（真・偽・省略）と一致することを固定する。

### Requirement 6: ゴーストと既存の振る舞いを壊さない

**Objective:** As a areka を常駐させている利用者と後でこの部分を触る開発者, I want MCP の 2 本が増えても、MCP を使わないときのゴーストの動きが変わらないこと, so that 既存のゴーストと既存の決定論テストが 1 本も壊れない

#### Acceptance Criteria

1. The areka shall MCP の `raise_event` 以外の経路で送るイベントには、今までどおり許可の表を効かせる（許可の表を通らない口は本 spec の `raise_event` と、それを使うと明示した後続の spec だけが使う）。
2. The areka shall 既存の `RaiseEvent` の呼び手（インストール・更新・ファイルの投げ込み・シェルやバルーンの切替など）の振る舞いと結果を変えない。
3. The areka shall トークの番号の採番と枠を、ゴースト自身の台本と MCP の台本で同じ規則にし、既存の決定論テストを 1 本も落とさない。
4. While `sakurascript`・`raise_event` の答えを待っている, the areka shall 描画・アニメーション・クリックの受け取りを止めない（返事を待って UI を塞がない）。
5. The areka shall 成功の答え（要件 1.1・2.3・2.4）で warn 以上の記録を出さない。

### Requirement 7: 決定論テスト・差の一覧・実機確認

**Objective:** As a 後でこのツールや kanade を触る開発者, I want 答えの中身と状態ごとの扱いがテストで固定され、SSP との差が一覧に残ること, so that 振る舞いがずれたときに赤で分かる

#### Acceptance Criteria

1. The areka shall kanade の側の振る舞いを、偽 SHIORI を使い kanade に実際の入力を与えた決定論テストに固定する: ⑴ 普段の状態で外からの台本が新しいトークとして再生される、⑵ 再生中に届いた台本が今の台本を中断する、⑶ 選択肢の最中に届いた台本が選択肢を取り下げ、選択や時間切れのイベントが 0 回、⑷ 起動・終了・切替の手順の途中は再生せず SHIORI へ送らない、⑸ 許可の表に無い名前のイベントが渡された Reference のまま SHIORI へ届き、返った台本の文字列が呼び手へ返る、⑹ 返り値なし・空白だけ・エラーの応答で「台本なし」が返り、今の再生が続く、⑺ 許可の表に在る名前でも Reference を補わない、⑻ MCP 以外の経路では許可の表が今までどおり効く、⑼ `OnTranslate` の Reference が要件 4.4・4.5 のとおり、⑽ 再生を始めたトーク 1 つにつき script 種別のログが 1 件（要件 4.1・4.3）で、表示の語が出どころごとに要件 4.2 のとおり。
2. The areka shall MCP の側の答えを決定論テストに固定する: 要件 1.1・1.4・2.3・2.4・2.6・2.7・3.5（`ghost_name` の有無の 2 通り）・3.6 の本文と `isError`。
3. The areka shall script 種別のログ（要件 4.1・4.2）が `get_log` の `log_type=script` で読めること（表示の語・ゴーストの名前・本文）を、MCP の側の決定論テストに固定する。
4. The areka shall `mcp-tool-entrances` が置いたダミーのテスト（`NG:not implemented yet` を固定するもの）を、本 spec の振る舞いを固定するテストへ書き換える。
5. The areka shall SSP との差の一覧（`doc/ssp-mcp/` の下・既存の `*-diff-areka.md` と同じ形）に、⑴ 起動・終了・切替の手順の途中は `NG:the ghost cannot accept a script now` で断ること（裁定 5）、⑵ 切替の途中は宛先が居ないときの `NG:` で答えること（裁定 4）、⑶ `event` が空のときツールの結果の `NG:` で答えること（SSP は JSON-RPC のエラー・裁定 10）、⑷ 制御文字を含むイベント名・Reference を断ること（裁定 11）、⑸ 中断の無効化モードの最中も割り込むこと（裁定 3）、⑹ `OnTranslate` の Reference1 の値（裁定 6）、⑺ `\t` を持たないこと、⑻ SHIORI の往復から生まれていないトークの表示の語（裁定 13）、を「SSP の印」（実測・未実測・対応物なし）とともに書く。
6. The areka shall 実機で確かめ、結果を本 spec の `verification/signoff.md` に残す: Claude Code から既定のゴースト（emo2）へ ⑴ `sakurascript` で台本を送ると `OK` が返り台詞が表示される、⑵ 長く待つ台本の再生中に 2 本目を送ると 1 本目が中断され 2 本目が表示される、⑶ `sakurascript` で表情を変えた後に `dump_surface` を 1 回呼ぶと変わった姿が返る（`mcp-dump-images` の要件 7.7 ⑵ の本来の手順）、⑷ `raise_event` で台本を返すイベントを起こすと台本の文字列を含む答えが返り台詞が表示される、⑸ ゴーストが知らない名前で `OK:the ghost returned no script.` が返る、⑹ `get_log`（`log_type=script`）に、MCP で送った台本（`[SSTP(Local,Auth)]`）・`raise_event` で返った台本（`[Ghost:<イベント名>]`）・ゴーストの自発の会話の台本が読める、⑺ 呼んでいる間も描画が止まらず warn 以上の記録が増えない。
7. When 要件 7.6 ⑴ が通る, the areka shall `seriko-trigger-intervals` の検体の README（`crates/areka-seriko/tests/fixtures/trigger-intervals/README.md` の「実機の確かめ」）の、`sakurascript` を使えないための読み替えの記述を、`sakurascript` で送る本来の手順へ戻す。
