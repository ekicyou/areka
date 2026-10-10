# Requirements Document

> 本文の実測は **2026-10-10・本ブランチ**（main `414d43eb` から分かれたもの）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、開発者の指摘で覆せる（覆したら該当要件も改める）。

## Project Description (Input)

終了の挨拶と、ゴーストの切り替えのお別れの台詞を再生している間、kanade は「台本を再生中」と見なさない。そのため実行の状態（ukadoc `Status [SSP拡張]`）に `talking` が載らない。

- MCP の `get_status` は、ゴーストが話しているのに空か `balloon(…)` だけを返す。
- 切り替えのお別れの台詞の中から `\![raise,…]` で SHIORI へイベントを送ると、その要求の `Status` にも `talking` が載らない。
- お別れの台詞の間にバルーンのイベントが送られるなら、その要求の `Status` にも `talking` が載らない。

直したい姿: ゴーストが終了の挨拶・切り替えのお別れの台詞を再生している間は、実行の状態に `talking` が載る。`get_status` の答えにも、その間に SHIORI へ送る要求の `Status` にも載る。

進め方: 「再生中か」の判定（`crates/areka-kanade/src/schedule/mod.rs` の `talk_active_of`）に 3 つの相（`CloseTalkWait`・`ChangeTalkWait`・`ChangeCloseTalkWait`）を足す。この判定を使うすべての所への波及（`State::snapshot` の利用者・選択待ちや `nouserbreak` の導出・バルーンの 3 つのイベントの要求を組む所・状態の問い合わせに答える所）を要件の段で洗い出す。握手の要求（`OnInitialize`・`OnBoot`・`OnGhostChanging`・`OnClose` など）が `snapshot_without_talk()` のまま変わらないことをテストで固定する。

範囲:

- 含む: お別れの台詞の再生中の `talking`（`get_status` と SHIORI への要求の `Status` の両方）。決定論テスト。SSP との差の一覧（`doc/ssp-mcp/get-status-diff-areka.md`）の該当行を消す。
- 含まない: SSP の旗 `changing`（足さない＝`mcp-get-status` の裁定）。出どころの無い 5 語（`minimizing`・`induction`・`passive`・`timecritical`・`opening(…)`）。握手の要求の `Status` の作り方。MCP のツールの側（`crates/areka/src/mcp/get_status.rs`）。

詳しい経緯と棚卸㉓の再測定は同じフォルダの `brief.md` を見よ。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを操らせる人**: `get_status` の `talking` を見て台本を送る時機を選ぶ。終了の挨拶や切り替えのお別れの台詞の間は、ゴーストが話しているのに `talking` が出ないので、「今は話していない」と読み違える。
- **ゴーストの作者**: お別れの台詞を `OnTranslate` で受けたとき、普段の会話なら付いている `Status: talking` が付いていない。同じ「台詞の翻訳」なのに場面で値が違う。
- **後続の spec の実装者**: `currentghost-property-others` の `currentghost.status` は同じ値を読む。お別れの間だけ値が欠ける穴を引き継いでしまう。

### 正典

ukadoc `Status [SSP拡張]`（SHIORI/3.0 の要求のヘッダ）は `talking` を「喋っている途中」と定める。場面（普段の会話か、お別れか）で分けていない。終了の挨拶もお別れの台詞も「喋っている途中」である。

### お別れの台詞の 3 つの場面

| 場面 | いつ始まるか | kanade の相 |
|---|---|---|
| 終了の挨拶 | 終了の要求で送った `OnClose` に SHIORI が台詞を返したとき | `Phase::CloseTalkWait` |
| 切り替えの送り出しの台詞 | 切り替えで送った `OnGhostChanging` に SHIORI が台詞を返したとき | `Phase::ChangeTalkWait` |
| 切り替えの別れの台詞 | `OnGhostChanging` が応答なし（204）で、続けて送った `OnClose` に SHIORI が台詞を返したとき | `Phase::ChangeCloseTalkWait` |

以下、この 3 つをまとめて「お別れの台詞」と呼ぶ。

### いま何が起きているか（2026-10-10 実測）

- **判定。** 「再生中か」を決めるのは `crates/areka-kanade/src/schedule/mod.rs` の `talk_active_of`。真にするのは `Phase::Steady { talk: Some(_) }`（普段の会話）と `Phase::BootVersion { talk: Some(_) }`（起動の挨拶）の 2 つだけ。
- **同じファイルの兄弟の関数は 3 つの場面を知っている。** 相から再生中のトークの番号を引く `current_talk_id` は、上の 2 つに加えて `CloseTalkWait`・`ChangeTalkWait`・`ChangeCloseTalkWait` でも番号を返す。利用者の中断（バルーンのダブルクリック）はこの関数で相手を決めるので、お別れの台詞は「中断で止められるのに `talking` は出ない」という食い違った状態にある。
- **隣の場面では今も `talking` が出る。** 切り替えの送り出しの台詞の途中に終了の要求が届くと、その台詞は普段の会話の扱い（`Phase::Steady { talk: Some(_) }`）へ移る（`schedule/change.rs` の `yield_to_close`）。同じ台詞の再生の途中で `talking` が無→有に変わる。
- **判定を読む所は 1 か所。** `talk_active_of` を呼ぶのは `State::snapshot_with_choice`（`State::snapshot` もここを通る）だけ。`talking` と、`nouserbreak`（「再生中 かつ 中断を禁じる旗の写し」）の 2 語がこの判定から決まる。

### 波及の洗い出し（`State::snapshot`／`State::snapshot_with_choice` を読む所のすべて）

お別れの台詞の再生中に**実際に届く**のは上の 2 行だけで、残りは場面が違うか、送る前に断られる。

| 読む所 | 何を作るか | お別れの台詞の再生中に届くか | 本 spec での変化 |
|---|---|---|---|
| `crates/areka-kanade/src/actor.rs` の `answer_status` | 状態の問い合わせ（`get_status`）の答え | **届く** | `talking` が載る |
| `schedule/translate.rs` の `capture` | 台詞を再生の前に翻訳にかける `OnTranslate` の `Status`。腕が相を決めた後に取るので、お別れの台詞の翻訳ではお別れの相で取る | **届く**（お別れの台詞そのものの翻訳） | `talking` が載る（普段の会話の台詞の翻訳は今も `talking` 付き） |
| `schedule/change.rs` の `on_raise_event` | `\![raise,…]` などのイベントの依頼の要求 | 届かない（定常でなければ `raise_event_not_steady` を記録して捨てる） | なし |
| `schedule/talk_gap.rs` の `begin` | 台詞の切れ目の依頼の印のイベント | 届かない（定常でなければ「定常でない」で返す） | なし |
| `schedule/balloon_events.rs` の `send` | バルーンの 3 つのイベント（`OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout`） | 届かない（3 つの入口とも定常でなければ `not_steady` を記録して送らない） | なし |
| `crates/areka-kanade/src/actor_resources.rs` の `answer` | リソースの照会 | 届かない（定常でなければ SHIORI へ送らず全件「応答なし」を返す） | なし |
| `schedule/steady.rs` の 5 か所 | 定常の `OnSecondChange`・マウスのイベント・選択のイベント | 届かない（定常の相だけ） | なし |
| `schedule/boot.rs` の `basewareversion` の通知 | 起動の挨拶の後の通知 | 届かない（起動の相だけ） | なし |

`OnSecondChange` の Reference3（今トークを始められるか）も同じ判定から決まるが、`OnSecondChange` は定常の相でしか送らないので変わらない。

**brief の記述の訂正。** brief と Project Description は「切り替えのお別れの台詞の中から `\![raise,…]` で送った要求の `Status` に `talking` が載らない」「お別れの台詞の間にバルーンのイベントが送られるなら載らない」と書くが、上の表のとおり、どちらも今は**要求そのものが送られない**。本 spec はこの「送らない」規則を変えない（暫定の裁定 3）。

### 要件の段での暫定の裁定（開発者の指摘で覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | お別れの台詞の翻訳（`OnTranslate`）の `Status` にも `talking` を載せるか | **載せる** | 普段の会話の台詞の翻訳は今も `talking` 付きで送る（`schedule/translate_tests.rs` が「捕まえた時点の相は再生中」と固定している）。お別れの台詞だけ外すと、同じ仕組みに場面の分岐を足すことになる。`get_status` の答えと SHIORI へ送る値を同じにする約束（完了 `mcp-get-status` の要件 1.2）にも合う | 2.2・2.3 |
| 2 | お別れの台詞が中断の無効化モード（`\![enter,nouserbreakmode]`）の中なら `nouserbreak` を載せるか | **載せる**（普段の会話と同じ規則＝「再生中 かつ 旗」） | `nouserbreak` は `talking` と同じ判定から決まる。お別れの台詞だけ別の規則にする理由が無い | 1.6 |
| 3 | お別れの台詞の再生中に届いたイベントの依頼（`\![raise,…]`）・バルーンのイベント・リソースの照会を送るようにするか | **変えない**（今のまま送らない） | brief の Scope は「再生中の `talking`」だけ。送る・送らないは切り替えと終了の進みの規則で、完了 spec（ゴーストの切り替え・`balloon-lifecycle-events`）が決めたもの | 3.3 |
| 4 | SSP との差の一覧で直す所 | 該当の 1 行を**消し**、「各旗の出る条件」の行の `talking` の説明にお別れの台詞を**足す** | 行を消すだけだと、同じ表の別の行に「普段の会話・起動の挨拶の再生中は `talking`」という古い説明が残る | 4.6 |
| 5 | 実機での確認を課すか | **課さない**。kanade に実際の入力を与えて殻ごと通す決定論テストで、問い合わせの答えまで固定する | MCP のツールの側は 1 行も変えず、その実機の確認は完了 `mcp-get-status` が済ませている。変わるのは kanade の判定 1 か所で、殻を通すテストが `get_status` の答えの素をそのまま見る | 4.2・4.7 |

## Boundary Context

- **In scope**:
  - お別れの台詞の再生中に、実行の状態へ `talking`（と、中断の無効化モードなら `nouserbreak`）を載せること。
  - その値が `get_status` の答えと、お別れの台詞の翻訳（`OnTranslate`）の `Status` の両方に出ること。
  - 決定論テスト（判定の全場面・殻を通す問い合わせ・翻訳の `Status`・握手の要求が変わらないこと）。
  - SSP との差の一覧と、古くなった注記の手直し。
- **Out of scope**:
  - SSP の旗 `changing`（足さない＝完了 `mcp-get-status` の裁定）。出どころの無い 5 語（`minimizing`・`induction`・`passive`・`timecritical`・`opening(…)`）。
  - 握手の要求（`OnInitialize`・`OnBoot`・`OnGhostChanging`・`OnClose`・強制終了の `OnClose` の通知）の `Status` の作り方。
  - お別れの台詞の再生中に、イベントの依頼・バルーンのイベント・リソースの照会・マウスのイベント・選択の確定を送るかどうかの規則。
  - お別れの台詞の進み（始まり方・終わり方・利用者の中断の扱い・待ちの上限）。
  - MCP のツールの側（`crates/areka/src/mcp/get_status.rs`）。kanade が正しい値を返せば手を入れずに直る。
  - `currentghost.status` のプロパティ（`currentghost-property-others`）。
- **Adjacent expectations**:
  - 完了 `status-execution-states` から受け取るもの: 実行の状態の素（`ExecutionSnapshot`）と、`Status` の値を作る書式。書式・語の並び・何も無ければ値なし、は変えない。
  - 完了 `mcp-get-status` から受け取るもの: 状態の問い合わせの口と、SSP との差の一覧。一覧のうち本 spec が直すのは、お別れの台詞の行と「各旗の出る条件」の行だけ。
  - `currentghost-property-others`: 同じ値を読むので、本 spec の後はお別れの台詞の間も `talking` を読める。
  - 同じ時期に `crates/areka-kanade/src/schedule/mod.rs` を触る spec（`mcp-kanade-tools`・`script-security-level`・`anchor-tag-canon`・`sakura-time-critical`・`property-query-channels`）とは同じウェーブに置かない。本 spec は小さいので先に着地させ、相手が取り込む。

## Requirements

### Requirement 1: お別れの台詞の再生中は実行の状態に `talking` が載る

**Objective:** As a AI エージェントでゴーストを操らせる人, I want ゴーストが終了の挨拶や切り替えのお別れの台詞を話している間も実行の状態に `talking` が出ること, so that 「話している間は `talking`」という読み方が場面を問わず当てになる

#### Acceptance Criteria

1. While ゴーストが終了の挨拶（終了の要求で送った `OnClose` に返った台詞）を再生している, the areka shall そのゴーストの実行の状態に `talking` を含める。
2. While ゴーストが切り替えの送り出しの台詞（`OnGhostChanging` に返った台詞）を再生している, the areka shall そのゴーストの実行の状態に `talking` を含める。
3. While ゴーストが切り替えの別れの台詞（`OnGhostChanging` が応答なしで、続けて送った `OnClose` に返った台詞）を再生している, the areka shall そのゴーストの実行の状態に `talking` を含める。
4. When お別れの台詞の再生が終わる（最後まで流れた・`\-` に達した・利用者が中断した・再生の完了待ちの上限を超えた）, the areka shall それ以後の実行の状態に `talking` を含めない（ゴーストを降ろす間も、切り替えが中止されて何も話していない定常へ戻った後も）。
5. While お別れの台詞を求める `OnClose`・`OnGhostChanging` の応答を待っている、または SHIORI が台詞を返さなかった（応答なし）, the areka shall 実行の状態に `talking` を含めない（話していない間は今のまま出さない）。
6. While お別れの台詞を再生していて、その台本が中断の無効化モード（`\![enter,nouserbreakmode]`〜`\![leave,nouserbreakmode]`）にある, the areka shall 実行の状態に `nouserbreak` を含め、モードの外では含めない（普段の会話と同じ規則）。
7. The areka shall 利用者の中断で止められるトークが在る場面と、実行の状態に `talking` が載る場面を一致させる（片方だけに当たる場面は 0）。

### Requirement 2: 同じ値が `get_status` の答えと SHIORI への要求の両方に出る

**Objective:** As a AI エージェントの利用者とゴーストの作者, I want お別れの台詞の間の `talking` が `get_status` にも SHIORI への要求の `Status` にも同じに出ること, so that エージェントとゴーストの見ている状態がずれない

#### Acceptance Criteria

1. When お別れの台詞の再生中に状態の問い合わせ（`get_status`）が届く, the areka shall 答えに `talking` を含める（バルーンが見えていれば `talking,balloon(…)`）。
2. When areka がお別れの台詞を再生の前に `OnTranslate` で翻訳にかける, the areka shall その要求の `Status` に `talking` を含める（普段の会話の台詞の翻訳と同じ）。
3. The areka shall お別れの台詞の再生中に、実行の状態を写して SHIORI へ送る要求のすべてに、同じ時点の `get_status` の答えと同じ値を載せる（今ある該当は要件 2.2 の 1 つだけ。後から同じ作りで増えた要求にも同じ規則が当たる）。

### Requirement 3: 変えないもの

**Objective:** As a ゴーストの作者と後続の spec の実装者, I want お別れの台詞の `talking` のほかは今のままであること, so that 握手の要求や終了・切り替えの進みに頼っている台本とテストが動き続ける

#### Acceptance Criteria

1. The areka shall 起動・切り替え・終了の握手の要求（`OnInitialize`・`OnBoot`・`OnGhostChanging`・`OnClose` と、強制終了の `OnClose` の通知）の `Status` に `talking`・`choosing`・`nouserbreak` を載せない（今のまま。お別れの台詞の再生中に強制終了が届いたときの `OnClose` の通知を含む）。
2. The areka shall 普段の会話と起動の挨拶の再生中の `talking`、および何も再生していない間に `talking` を出さないことを、今のまま保つ。
3. The areka shall お別れの台詞の再生中に届いたイベントの依頼（`\![raise,…]` など）・台詞の切れ目の依頼・バルーンの 3 つのイベント・リソースの照会・マウスのイベント・選択の確定を、今のまま SHIORI へ送らない（本 spec で増える要求は 0）。
4. The areka shall お別れの台詞の再生中に実行の状態へ `choosing` を含めない（選択待ちは定常の会話でだけ成り立つ＝今のまま）。
5. The areka shall お別れの台詞の進み（再生の始まり・完了でゴーストを降ろすこと・利用者の中断の扱い・再生の完了待ちの上限）を変えない。
6. The areka shall `Status` の書式（語の並び・カンマ連結・何も無ければ行を出さない）と、出さない語（SSP の `changing`・出どころの無い 5 語）を変えない。

### Requirement 4: 決定論テストと文書

**Objective:** As a 後で kanade の運行や実行の状態を触る開発者, I want お別れの台詞の `talking` と、変えないものがテストで固定され、文書に古い説明が残らないこと, so that 場面を足したときや判定を触ったときに赤で分かる

#### Acceptance Criteria

1. The areka shall 「再生中か」の判定を、運行の相のすべての種類について決定論テストに固定する: 普段の会話・起動の挨拶・お別れの台詞の 3 つの場面は真、それ以外（起動の各段・挨拶の無い起動・何も再生していない定常・`OnClose` や `OnGhostChanging` の応答待ち・降ろしている間・停止後）は偽。
2. The areka shall kanade に実際の入力を与えて殻ごと通す決定論テストで、お別れの台詞の 3 つの場面それぞれについて、⑴ 再生中の状態の問い合わせの答えが `talking` を含むこと、⑵ 台詞を返す応答を待つ前（何も再生していない間）の答えが `talking` を含まないこと、を固定する。
3. The areka shall お別れの台詞の 3 つの場面それぞれについて、その台詞の翻訳（`OnTranslate`）の要求の `Status` が `talking` を含むことを決定論テストに固定する。
4. The areka shall 要件 1.6 を決定論テストに固定する: お別れの台詞の再生中に中断を禁じる旗の知らせが届いていれば `nouserbreak` を含み、届いていなければ含まない。
5. The areka shall 要件 3.1 を決定論テストに固定する: 少なくとも ⑴ 普段の会話の完了の後に送る `OnClose`、⑵ 切り替えの `OnGhostChanging`、⑶ `OnGhostChanging` が応答なしの後に送る `OnClose`、⑷ お別れの台詞の再生中に届いた強制終了の `OnClose` の通知、の 4 つの `Status` が `talking` を含まないこと（既にあるテストが固定している場面は、そのテストを数える）。
6. The areka shall 要件 1.7 を決定論テストに固定する: 運行の相のすべての種類について、「再生中か」の判定が真であることと、再生中のトークの番号が引けることが一致する。
7. The areka shall SSP との差の一覧（`doc/ssp-mcp/get-status-diff-areka.md`）から「終了の挨拶・切り替えのお別れの台詞の再生中」の行を消し、「各旗の出る条件」の行の `talking` の説明を「普段の会話・起動の挨拶・終了の挨拶・切り替えのお別れの台詞の再生中」に改め、見出しの日付を改める。
8. The areka shall 「再生中」を普段の会話と起動の挨拶の 2 つだけと説明するコードの注記（判定の関数の注記・実行の状態の素の `talk_active` の欄の注記）を、お別れの台詞を含む説明に改める。
9. The areka shall 本 spec の変更の後、`crates/areka-kanade` と `crates/areka` の既存のテストをすべて緑に保つ（お別れの台詞の間に `talking` が無いことを前提にしたテストが在れば、新しい振る舞いへ書き換える）。
