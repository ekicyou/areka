# Requirements Document

> 本文の実測は **2026-10-05・本ブランチ**（main `44fc0a61` から分かれたもの）。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、brief が「要件の議題」とした点と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。
> 要件を書いた時点では SSP が起動しておらず（SSP の MCP は `Server not available`）、SSP の答えは [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md) の 2.9.05 の実測と、ツールの説明文の逐語（[doc/ssp-mcp/tools-list-ssp-2.9.05.json](../../../doc/ssp-mcp/tools-list-ssp-2.9.05.json)）による。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code など）でゴーストを作る人や、エージェントにゴーストを操らせる人は、台本を送る前に「ゴーストが今しゃべっている最中か・選択肢を出して待っているか」を知りたい。SSP では `get_status` を呼べば分かる。areka の `get_status` は今も `NG:not implemented yet` を返すので、エージェントは台本を送る時機を選べない。

**今の状態**: ツールの入口・名前の解決・UI スレッドへの橋は `mcp-tool-entrances` が作り、アプリ本体側 `crates/areka/src/mcp/get_status.rs` の `handle` は `NG:not implemented yet` を返すダミーである。実行の状態（ukadoc `Status [SSP拡張]` の 10 の語）は kanade が持ち、SHIORI へ送る要求の `Status` の値を作る仕組み（`crates/areka-kanade/src/status.rs` の `ExecutionStatus::derive(..).render()`）も公開されているが、外から kanade に今の状態を問う口が無い。

**何を変えるか**: `get_status` が、宛先のゴーストの今の実行の状態を SSP と同じ書式（カンマ区切りの語・何も無ければ空）で返す。ゴーストが居なければ SSP と同じ文言の `NG:` で答える。

> 起票: 2026-10-05 棚卸㉒で `mcp-kanade-tools` から切り出した（大きさでなく並走のため。`get_status` だけは kanade の運行に触らずに答えられる）。SSP MCP の移植（`.kiro/steering/roadmap.md`「SSP MCP の移植」）の 1 本・ウェーブ C4。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人・操らせる人**: エージェントが台本を送るとき、ゴーストが話している途中なら割り込みになり、選択肢を待っているなら選択が流れる。SSP のツールの説明文も「talking・choosing・timecritical・opening の間に台本を送ると、割り込むか拒まれることがある」と書く。今の areka では送る前に確かめる手段が無い。
- **後続の spec の実装者**: `mcp-user-response`・`mcp-author-tools` は状態を読んで振る舞いを決めたい。`currentghost-property-others` の `currentghost.status` も同じ値を読む。読む口がまだ無い。

### 正典（ukadoc と SSP で確かめたもの）

- **語の一覧**: ukadoc `Status [SSP拡張]`（SHIORI/3.0 の要求のヘッダ）。`talking`（喋っている途中）・`choosing`（選択肢表示中）・`minimizing`・`induction`・`passive`・`timecritical`・`nouserbreak`・`online`（ネットワーク通信中）・`opening(種類)`・`balloon(...)` の 10 語を、複数あればカンマでつなぐ。`opening` と `balloon` の中は `/` 区切り（例 `balloon(0=2/1=0)`）。**`changing` は ukadoc の一覧に無い。**
- **SSP の `get_status`**: 説明文は「SSTP `EXECUTE GetStatus` と同じ・カンマ区切りの旗・空は idle」。旗の一覧に `changing` を含む 11 語。実測の答えは `balloon(0=0/1=0/2=0)`・`talking,balloon(0=0/1=0)`（`/` 区切りで ukadoc と同じ）。宛先が見つからないときは `NG:Cannot find active ghost from specified name`（survey §3・§7.4）。各旗の出る条件の実例は未実測（survey §5）。

### いま何が起きているか（2026-10-05 実測）

- **MCP の側。** `crates/areka/src/mcp/get_status.rs` の `handle(_world, _ghost, _args, reply)` は `outcome::ng("not implemented yet")` を返す。`get_status_tests.rs` がその文言を期待している（本 spec が書き換える）。宛先の解決は `crates/areka/src/mcp/mod.rs` の振り分けで `Omitted::UseActive`（省略＝起動中の 1 体）として済んでから `handle` に届く（`crates/areka/src/mcp/resolve.rs` の `resolve`）。後から答える口 `mcp::later`（`mod.rs`）がある。結果を作る関数は `crates/areka-mcp/src/tools/outcome.rs` の `value`（素の値・`isError: false`）・`ok`・`ng`。
- **kanade の側。** 実行の状態の素は `State::snapshot()`（`crates/areka-kanade/src/schedule/mod.rs`）が返す `ExecutionSnapshot` で、今は 5 語（`talking`・`choosing`・`nouserbreak`・`online`・`balloon(…)`）が実物から導かれ、残る 5 語（`minimizing`・`induction`・`passive`・`timecritical`・`opening(…)`）は出どころが無く常に出ない（`status.rs` の `ExecutionStatus::derive` の導出表）。`render()` は何も無ければ `None`。
- **外から問う口が無い。** `KanadeMsg`（`crates/areka-kanade/src/msg.rs`）に今の状態を問う変種は無い。殻がその場で答える作りの手本は `KanadeMsg::ResourceQuery`（`actor.rs` の振り分けと `actor_resources.rs`）。
- **kanade は SHIORI の往復の間は手が空かない。** kanade の殻は SHIORI への要求の答えを待つ間ほかの知らせを処理しない（`actor.rs` の `round_trip`）。そのため、SHIORI が長く考えている間に届いた問い合わせは、その往復が終わってから答えられる。
- **待ちの上限。** 入口の橋（`crates/areka-mcp/src/tools/bridge.rs`）が 10 秒の上限（`NG:areka did not respond within 10 seconds`）と終了の途中の答え（`NG:areka is shutting down`）を持つ。
- **ファイルの大きさ。** `msg.rs` 909 行・`actor.rs` 876 行（上限 1,000）。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | SSP の旗 `changing`（ゴーストの切替中と見られる）を足すか（brief の議題） | **足さない**。areka の `get_status` は ukadoc の 10 語だけを出す。`status.rs` と切替の写しの通り道は範囲に入れない | ukadoc の `Status` に `changing` は無い。areka は意味を ukadoc から取り、SSP の実測を正本にしない（開発者方針）。areka では切替の途中は宛先の解決が「起動中のゴースト無し」になり（`resolve.rs` の `active`）、エージェントは `NG:` で「今は送れない」と分かる | 2.7・3.2・5.5 |
| 2 | 何の状態も当たらないときの答え | **空の本文**・`isError: false`（`OK` も付けない） | SSP の説明文「Empty means idle」。素の値を返す他のツール（`get_active_ghost_list`）と同じ作り | 1.3 |
| 3 | 何を「今の状態」とするか | **その時点でそのゴーストが SHIORI へ要求を送るなら付ける `Status` の値と同じ**（同じ状態から同じ書式で作る） | SSP の説明文「SSTP `EXECUTE GetStatus` と同じ」・survey「SHIORI/3.0 の `Status` ヘッダ・`currentghost.status` と同じ内容」。値が 2 つの口で食い違うと、エージェントとゴーストの見ている状態がずれる | 1.2・5.3 |
| 4 | 話し中・選択待ち中の呼び出し | **終わりを待たずに、その時点の状態を返す** | 状態を見て送る時機を選ぶためのツールであり、終わるまで答えないと用をなさない | 4.2 |
| 5 | 宛先を解決した後、答えを作る前にゴーストが降りた（切替・終了・倒れた）とき | **宛先の解決に失敗したときと同じ文言**で答える（`ghost_name` を渡していれば `NG:Cannot find active ghost from specified name`、省略なら `NG:Specified ghost is not active`） | 答えた時点では宛先が居ない。新しい文言を作らず、入口の解決と同じ答えにそろえる | 3.2 |
| 6 | SSP との差をどこに書くか | **`doc/ssp-mcp/get-status-diff-areka.md` を新しく足す**（`dump-images-diff-areka.md` と同じ形） | brief の「触るファイル」に文書は無いが、MCP の他のツールは差を `doc/ssp-mcp/` に残している。ウェーブ C4 の約束（`schedule/`・`lib.rs`・`mod.rs`・`handler.rs`）には当たらない | 5.5 |

## Boundary Context

- **In scope**:
  - `get_status` の本物の答え（状態の語の本文・空の本文）と、宛先が降りた後の答え。
  - kanade に今の実行の状態を問い、その時点の値を返してもらう口。
  - 決定論テスト・SSP との差の一覧・実機での確認（Claude Code から 1 回）。
- **Out of scope**:
  - 実行の状態の決め方そのもの（どの語がいつ立つか・`ExecutionSnapshot` の欄・導出表）。まだ出どころの無い 5 語（`minimizing`・`induction`・`passive`・`timecritical`・`opening(…)`）を出すことも含む。
  - SSP の旗 `changing`（裁定 1）。
  - `sakurascript`・`raise_event`（`mcp-kanade-tools`）・`reload`（`mcp-reload`）・`strict`（`mcp-strict-errors`）。
  - ツールの定義・引数の型の検査・`ghost_name` の解決の規則・待ちの上限（`mcp-tool-entrances` のまま変えない）。
  - `currentghost.status` のプロパティ（`currentghost-property-others`）。
- **Adjacent expectations**:
  - `mcp-tool-entrances` から受け取るもの: 検査を通った引数・解決済みのゴースト・結果を作る関数・後から答える口・10 秒の待ちの上限と終了の途中の答え。
  - `status-execution-states`（完了）から受け取るもの: 実行の状態の素と、`Status` の値を作る書式（正典順・カンマ連結・`/` の下位書式・何も無ければ値なし）。後から他の spec が状態の出どころを足せば、本 spec の手を入れずに `get_status` にもその語が出る。
  - `mcp-kanade-tools`: kanade の知らせの型（`msg.rs`）と殻の振り分け（`actor.rs`）を分け合うので、同じウェーブに置かない。
  - `mcp-user-response`・`mcp-author-tools`・`currentghost-property-others`: 状態を読むとき、本 spec が足す問い合わせの口を使える。
  - 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）: kanade の `schedule/`・`lib.rs` に触らない（`balloon-lifecycle-events` の持ち物）。MCP の `mod.rs`・`handler.rs` に触らない。`msg.rs`・`actor.rs` は上限 1,000 行に近いので、足すものは新しいファイルへ置く。

## Requirements

### Requirement 1: 今の実行の状態を SSP と同じ書式で返す

**Objective:** As a AI エージェントの利用者, I want `get_status` がゴーストの今の実行の状態を SSP と同じ書式で返すこと, so that SSP 向けの呼び方のまま、台本を送ってよい時機かを判断できる

#### Acceptance Criteria

1. When 宛先の解決を通った `get_status` の呼び出しが届く, the areka shall そのゴーストの今の実行の状態を、ukadoc `Status [SSP拡張]` の語をカンマでつないだ本文・`isError: false` の結果で返す（`OK:` などの前置きを付けない）。
2. The areka shall 本文を、同じ時点でそのゴーストが SHIORI へ要求を送るなら付ける `Status` の値と一字違わず同じにする。
3. While そのゴーストに当たる状態が 1 つも無い, the areka shall 本文を空の文字列・`isError: false` の結果で返す。
4. The areka shall 複数の語を ukadoc の語の定義順（`talking`・`choosing`・`minimizing`・`induction`・`passive`・`timecritical`・`nouserbreak`・`online`・`opening(…)`・`balloon(…)`）に並べ、同じ語を 2 度出さない。
5. The areka shall `balloon(…)` の中を `キャラクター番号=バルーン番号` を `/` でつないだ形（キャラクター番号の小さい順・例 `balloon(0=0/1=0)`）にする。
6. The areka shall 結果を本文 1 つだけにする（画像などの content を付けない）。

### Requirement 2: どの語がいつ出るか

**Objective:** As a AI エージェントの利用者, I want 語が実際のゴーストの様子と合っていること, so that `talking` が出ている間は送るのを待ち、消えたら送る、という使い方が当てになる

#### Acceptance Criteria

1. While そのゴーストが台本を再生している, the areka shall 本文に `talking` を含める。
2. While そのゴーストが選択肢を出して選ばれるのを待っている（選ばれた後の SHIORI の応答を待つ間と、時間切れの応答を待つ間を含む）, the areka shall 本文に `choosing` を含める（選択肢を出している間は再生も続いているので `talking,choosing` となる）。
3. While 再生中の台本が中断の無効化モード（`\![enter,nouserbreakmode]`〜`\![leave,nouserbreakmode]`）にある, the areka shall 本文に `nouserbreak` を含め、台本を再生していないときは含めない。
4. While areka がネットワーク通信（更新の手続き・URL からの取得）をしている, the areka shall 本文に `online` を含める。
5. While そのゴーストのバルーンが見えている, the areka shall 本文に、見えているバルーンの組を `balloon(…)` で含め、見えているバルーンが 1 つも無いときは `balloon(…)` を出さない。
6. The areka shall 出どころがまだ無い 5 語（`minimizing`・`induction`・`passive`・`timecritical`・`opening(…)`）を本文に出さない（状態を作り出さない）。
7. The areka shall ukadoc に無い語（SSP の `changing` を含む）を本文に出さない。

### Requirement 3: 宛先と、答えられないときの答え

**Objective:** As a AI エージェントの利用者, I want ゴーストが居ないときに決まった文言の `NG:` が返ること, so that 「今は状態が無い（空）」と「宛先が居ない」を取り違えない

#### Acceptance Criteria

1. The areka shall `ghost_name` の解決（省略＝起動中の 1 体・名前かルートフォルダのフルパス・当たらなければ `NG:Cannot find active ghost from specified name`・起動中のゴーストが無く省略なら `NG:Specified ghost is not active`）を `mcp-tool-entrances` のまま使い、本 spec で変えない。
2. If 宛先を解決した後、状態を答える前にそのゴーストが降りた（切替・終了・倒れた）, then the areka shall 宛先の解決に失敗したときと同じ文言（`ghost_name` を渡していれば `NG:Cannot find active ghost from specified name`、省略なら `NG:Specified ghost is not active`）・`isError: true` で答える。
3. The areka shall 待ちの上限と終了の途中の答えを `mcp-tool-entrances` のまま（10 秒・`NG:areka did not respond within 10 seconds`・`NG:areka is shutting down`）とし、本 spec で変えない。
4. The areka shall `get_status` に `NG:not implemented yet` を返す経路を残さない。
5. The areka shall 要件 3.2 の答えのときに warn 以上の記録を出さない（切替や終了の途中に呼ばれるのは普通の出来事であり、`debug!` までにとどめる）。

### Requirement 4: ゴーストと利用者を邪魔しない

**Objective:** As a areka を常駐させている利用者, I want エージェントが何度 `get_status` を呼んでもゴーストがいつもどおり動くこと, so that 状態を見張られている間も会話や描画が止まったり変わったりしない

#### Acceptance Criteria

1. The areka shall `get_status` の処理で、SHIORI へイベントを送らず、台本を再生せず、ゴーストの運行（トーク・選択待ち・切替・終了の進み）と実行の状態を変えない（どれも 0 回）。
2. While そのゴーストが台本を再生している、または選択肢を待っている, when `get_status` が届く, the areka shall 再生や選択の終わりを待たずに、その時点の状態を返す（ただし kanade が SHIORI との往復の最中に届いた呼び出しは、その往復が終わってから答える。往復が 10 秒を超えたときは要件 3.3 の待ちの上限の答えになる）。
3. While `get_status` の答えを待っている, the areka shall 描画・アニメーション・クリックの受け取りを止めない（返事を待って UI を塞がない）。
4. The areka shall 成功の答え（要件 1）で warn 以上の記録を出さない。

### Requirement 5: 決定論テスト・差の一覧・実機確認

**Objective:** As a 後でこのツールや実行の状態を触る開発者, I want 答えの中身と文言がテストで固定され、SSP との差が一覧に残ること, so that 状態の出どころが増えたときや書式がずれたときに赤で分かる

#### Acceptance Criteria

1. The areka shall kanade が問い合わせに今の状態を返すことを、kanade に実際の入力（起動・台本の再生・選択肢の提示・状態の知らせ）を与えてその状態へ進めたうえで決定論テストに固定する: ⑴ 何も無い＝値なし、⑵ 再生中＝`talking`、⑶ 選択待ち＝`talking,choosing`、⑷ 中断の無効化モードの知らせ＋再生中＝`nouserbreak` を含む・再生していなければ含まない、⑸ バルーンの知らせ＝`balloon(…)` を含む、⑹ 問い合わせの前後で運行の状態（運行の相・選択待ちの帳簿・再生中のトーク）が変わらない（要件 4.1。殻がどの知らせの前にも行う通信中の数の写しの更新は既存の振る舞いであり、比べる対象に入れない）。
2. The areka shall MCP の側の答えを決定論テストに固定する: ⑴ 値なし＝空の本文・`isError: false`、⑵ 値あり＝その値そのままの本文・`isError: false`、⑶ 答える前に宛先が降りた＝要件 3.2 の文言（`ghost_name` の有無の 2 通り）・`isError: true`。
3. The areka shall 要件 1.2 を決定論テストに固定する: 少なくとも再生中の場面で、`get_status` の本文と、同じ状態で SHIORI へ送る要求の `Status` の値が一致すること。
4. The areka shall `mcp-tool-entrances` が置いたダミーのテスト（`NG:not implemented yet` を固定するもの）を、本 spec の振る舞いを固定するテストへ書き換える。
5. The areka shall SSP との差の一覧（`doc/ssp-mcp/get-status-diff-areka.md`）に、⑴ `changing` を出さないこと（裁定 1）、⑵ 出どころの無い 5 語を出さないこと、⑶ 切替の途中は状態の語でなく `NG:` で答えること、⑷ SSP の各旗の出る条件が未実測であること、⑸ ゴーストの SHIORI が考えている間に届いた呼び出しは、その答えが返ってから答えること（要件 4.2）、を「SSP の印」（実測・未実測・対応物なし）とともに書く。
6. The areka shall 実機で確かめ、結果を本 spec の `verification/signoff.md` に残す: Claude Code から既定のゴースト（emo2）へ ⑴ 何も話していない間に `get_status` を呼ぶと `talking` を含まない答え（空、またはバルーンが見えていれば `balloon(…)` だけ）が返る、⑵ ゴーストが話している間（クリックや自発の会話で話させ、話し始めて 1 フレーム以上たってから。見えているバルーンの組は 1 フレーム遅れて kanade に届くため）に呼ぶと `talking` と `balloon(…)` を含む答えが返る、⑶ 起動していない名前を渡すと `NG:Cannot find active ghost from specified name` が返る、⑷ 呼んでいる間も会話と描画が止まらず、warn 以上の記録が増えない。
