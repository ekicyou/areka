# ギャップ分析: areka-P0-mcp-get-status

> 2026-10-05・本ブランチ（main `44fc0a61` から分かれたもの）で、コードを読むだけで確かめた（ビルド・テストは回していない）。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指す。
> 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C4 の約束）。

## 1. 分析の要約

- **材料はほぼ全部そろっている。** 状態の素（`State::snapshot`）・書式（`ExecutionStatus::derive(..).render()`）・殻がその場で答える手本（`KanadeMsg::ResourceQuery` と `actor_resources::answer`）・UI を塞がずに返事を待つ口（`mcp::later` と `ReplyReceiver::try_recv`）・テストで kanade の送出端だけ差す組み立て（`GhostSession::for_test`）が、どれも既にある。新しく作るのは「問い合わせの変種 1 つ」と「`get_status` の `handle` の中身」だけ。
- **ぶつかる点が 3 つある。** ⑴ brief の「新規 `actor_status.rs`」は `lib.rs` に `mod` を足さないと置けないが、`lib.rs` は C4 の約束で触れない。⑵ 要件 2.1（台本を再生している間は `talking`）と今の導出（終了・切替の別れの台詞の再生中は `talking` を立てない）が食い違う場面がある。⑶ kanade は SHIORI との往復の間ほかの知らせを処理しないので、SHIORI が長く考えていると 10 秒の上限に当たりうる。
- **候補は 3 つ**: A＝殻の腕を `actor.rs` に直に書く（いちばん小さい）・B＝brief どおり別ファイルへ出し、`actor.rs` の中で `#[path]` で読み込む（`lib.rs` に触れない）・C＝kanade が状態を共有の置き場へ書き出し、UI がその場で読む（問い合わせの往復をなくす）。C は要件の「問い合わせの口」と範囲（`schedule/` に触らない）から外れる。
- **規模は S・危険は低。** 既存の作りをなぞるだけで、新しい依存も新しい仕組みも要らない。

## 2. 今あるもの（要件ごとの対応表）

| 要件 | 使える既存のもの | 足りないもの | 印 |
|---|---|---|---|
| 1.1・1.3・1.6（本文 1 つ・空も成功・`isError: false`） | `crates/areka-mcp/src/tools/outcome.rs` の `value`（素の値・`isError: false`）。`get_active_ghost_list.rs` の `handle` が空の本文を `value` で返す手本 | `render()` の `None` を空文字へ写す 1 行 | Missing（小） |
| 1.2（SHIORI へ送る `Status` と一字違わず同じ） | 送る側は `crates/areka-kanade/src/actor.rs` の `round_trip_raw` が `status.render()` を線に載せる。状態は各イベントの組み立てで `State::snapshot()`（定常）か `State::snapshot_without_talk()`（起動・切替・終了の握手）から作る（`crates/areka-kanade/src/schedule/mod.rs`） | 問い合わせの腕が同じ `State::snapshot()` → `ExecutionStatus::derive` → `render()` を通ること | Missing |
| 1.4・1.5（正典順・重複なし・`balloon(0=0/1=0)`） | `crates/areka-kanade/src/status.rs` の `ExecutionStatus::from_states`（正典順に並べて重複を除く）・`render`（カンマ連結）・`BalloonBindings::new` | 無し（書式は全部済み） | — |
| 2.1〜2.5（各語の出る条件） | `status.rs` の `ExecutionStatus::derive` の導出表（5 語が実物から）・`schedule/mod.rs` の `State::snapshot_with_choice`（`nouserbreak` は再生中だけ）・`actor.rs` の `sync_online`（毎メッセージの最初に通信中の数を写す） | 無し。ただし 2.1 は終了・切替の別れの台詞の再生中に食い違う（3 節の事実 3） | Constraint |
| 2.6・2.7（5 語と `changing` を出さない） | `derive` が 5 語を作らない・`ExecutionState` に `changing` が無い | 無し（触らなければ守られる） | — |
| 3.1・3.3（解決と待ちの上限はそのまま） | `crates/areka/src/mcp/mod.rs` の `dispatch`（`GetStatus` は `Omitted::UseActive`）・`crates/areka-mcp/src/tools/bridge.rs` の `call`（上限・終了の途中の答え） | 無し | — |
| 3.2（答える前に降りた → 解決の失敗と同じ文言・`isError: true`） | `crates/areka/src/mcp/resolve.rs` の定数 `NOT_ACTIVE`・`CANNOT_FIND` と純粋な判断 `resolve`。`areka-actor` の `ReplyReceiver::try_recv` が送り手の消滅を `ReplyError::Dropped` で返す | 「降りた」を見分けて `NG:` を返す分岐 | Missing |
| 3.4（`not implemented yet` を残さない） | — | `get_status.rs` の `handle` の書き換え | Missing |
| 3.5・4.4（warn 以上を出さない） | `ReplyTo::send` は記録を出さない（`bridge.rs`）。ただし `ReplyTo` を送らずに落とすと入口が `warn!`（「終了の途中で答えなかった」）を出す | 降りた場合も必ず `send` すること | Constraint |
| 4.1（運行も状態も変えない） | `actor_resources::answer` が「読むだけで書き換えない」作りの手本。`schedule::step` を通さず `return Ok(ControlFlow::Continue(()))` で抜ける | 同じ形の腕 | Missing |
| 4.2・4.3（終わりを待たない・UI を塞がない） | 再生は sakura が受け持ち、kanade は再生中も知らせを受ける。`mcp::later` は毎フレーム覗くだけで待たない | 返事の受け手を `later` に預ける（または別のスレッドで待つ） | Missing |
| 5.1・5.3（kanade の決定論テスト） | `crates/areka-kanade/tests/kanade/external_status_test.rs`（偽の shiori・偽の sakura・保留のハーネス `spawn_harness_gated` で再生中を作り、線の `status` を逐語で突き合わせる）・`src/actor_online_tests.rs`（殻を本物で起こし `(名前, Status)` を記録） | 問い合わせを足したテスト | Missing |
| 5.2・5.4（MCP 側の決定論テスト） | `crates/areka/src/mcp/get_status_tests.rs`（ダミーを固定中）・`GhostSession::for_test(kanade, dir)`（実行系なしで送出端だけ差せる）・`dump_surface.rs` の `WaitAnswer`（後から届く答えを待つテストの補助） | テストの書き換え | Missing |
| 5.5（SSP との差の一覧） | `doc/ssp-mcp/dump-images-diff-areka.md`（表の形と「SSP の印」の書き方の手本）・`doc/ssp-mcp/survey.md` の `get_status` の行 | 新しい文書 | Missing |
| 5.6（実機確認） | 過去の MCP の spec の `verification/signoff.md` | 実機での確認 | Missing |

## 3. 実物で確かめた事実（設計の前提になるもの）

1. **`lib.rs` を触らずに新しいファイルを置く道がある。** kanade の殻の分室 `actor_resources`・`actor_translate` は `crates/areka-kanade/src/lib.rs` で `mod actor_resources;` と宣言されている。brief の「新規 `actor_status.rs`」を同じ形で置くと `lib.rs` に 1 行足すことになり、C4 の約束（`lib.rs` に触らない）に当たる。一方 `actor.rs` の末尾はテストの子モジュールを `#[path = "actor_tests.rs"] mod tests;` の形で 7 本読み込んでいるので、同じ形でテスト以外の子モジュールも `actor.rs` の中から読み込める（`lib.rs` は無傷）。
2. **腕そのものは数行で済む。** `KanadeMsg::ResourceQuery` の腕（`actor.rs` の `spawn_kanade_translating` の振り分けの `match`）は、`sync_online` の後・`step` の前に置かれ、分室の関数を呼んで `return Ok(ControlFlow::Continue(()))` で抜ける。状態の問い合わせは SHIORI へ往復しないので、`state.snapshot()` → `ExecutionStatus::derive` → `render()` → 返信端へ送る、だけで足りる（`State::snapshot` は `pub(crate)` で同じクレートの殻から呼べる＝`schedule/` に触れない）。
3. **終了・切替の別れの台詞の再生中は `talking` が立たない。** `schedule/mod.rs` の `talk_active_of` は `Phase::Steady { talk: Some(_) }` と `Phase::BootVersion { talk: Some(_) }` だけを真にする。`Phase::CloseTalkWait`・`ChangeTalkWait`・`ChangeCloseTalkWait` は再生中の `talk_id` を持つが、偽になる。この相では SHIORI へ要求を送らないので、今まで「Status に `talking` が載らない」ことが表に出なかった。`get_status` はこの相でも呼べる（終了の挨拶の最中に呼べば空か `balloon(…)` だけが返る）。導出の決め方は本 spec の範囲外（Boundary の Out of scope）。
4. **`online` は問い合わせの直前に最新になる。** `actor.rs` の `sync_online` はどの知らせの処理よりも先に通信中の数を `state.external.online` へ写す（違うときだけ `debug!`）。問い合わせの腕をこの後に置けば `online` は最新。ただしこれは状態の「写し」を書き換える 1 か所なので、要件 5.1⑹「問い合わせの前後で運行の状態が変わらない」をどこまで比べるか（運行の相・選択の帳簿・トークだけか、写しも含むか）で、テストの書き方が変わる。
5. **kanade が待つのは SHIORI との往復の間だけ。** `actor.rs` の `round_trip` は SHIORI の答えを `reply_rx.recv()`（上限なし）で待つ。その間に届いた問い合わせは受信箱に並び、往復が終わってから答えられる。台本の再生や選択の待ちは sakura の側で進むので、kanade は手が空いている（要件 4.2 は満たせる）。SHIORI が 10 秒を超えて考え込むと、入口の上限（`bridge.rs` の `call`・`NG:areka did not respond within 10 seconds`・`warn!` 1 件）に当たる。
6. **「降りた」は返信端の消滅で分かる。** kanade が止まると受信箱ごと返信端が落ち、`ReplyReceiver::try_recv` が `Err(ReplyError::Dropped)` を返す。送る時点で kanade の受信箱がもう無ければ `Sender::send` が `Err` を返す。置き場（`GhostSlot`）が空・`GhostSession::kanade()` が `None` のときも同じく「降りた」と扱える。
7. **3.2 の文言は解決の判断から取り直せる。** `resolve.rs` の `resolve(None, ghost_name, Omitted::UseActive)` は、省略・空文字なら `NOT_ACTIVE`、名前を渡していれば `CANNOT_FIND` を返す。これを呼べば、要件 3.2 の「渡していれば／省略なら」の分け方を入口と同じ 1 か所から引ける。並走の `mcp-ghost-name-match` は `resolve` を直す予定で（SSP は空文字も `Cannot find…`・survey の表）、その後も自動でそろう。
8. **MCP のテストは実行系なしで組める。** `crates/areka/src/ghost_session.rs` の `GhostSession::for_test(kanade, ghost_dir)` は実行系を持たず kanade の送出端だけを差す。`handle` は `ActiveGhost` を引数で受け取るので、テストは偽の受信端を握って「問い合わせが 1 通届く→返信端へ値を送る／落とす」を決定論で作れる。
9. **`KanadeMsg` を網羅して `match` しているのは 2 か所だけ。** `actor.rs` の振り分けと、`msg.rs` のテスト `existing_eight_kanade_msg_variants_are_unchanged_by_additive_growth` の `label`。変種を足すと両方に 1 腕ずつ要る。ほかのクレートは個別の変種を作って送るだけ。
10. **行数の余裕。** `msg.rs` 909 行・`actor.rs` 876 行（上限 1,000）。変種（説明込み 8 行前後）と `label` の 2 行で `msg.rs` は 920 行前後、腕（5〜10 行）で `actor.rs` は 890 行前後。どちらも上限の内に収まる。

## 4. 実装の選択肢

### 選択肢 A: 殻の腕を `actor.rs` に直に書く

- `msg.rs` に変種（例 `StatusQuery { reply: ReplySender<Option<String>> }`）と `label` の腕、`actor.rs` の振り分けに数行の腕（`ExecutionStatus::derive(&state.snapshot()).render()` を送る・受け手が居なければ `debug!`）。
- MCP の側は `get_status.rs` の `handle` が置き場の kanade へ問い合わせを送り、返事の受け手を `mcp::later` に預ける。覗く関数は `try_recv` で値なら `outcome::value(値.unwrap_or_default())`、`Dropped` なら 3.2 の `NG:`。
- ✅ 足す行がいちばん少ない・新しいファイルなし・`lib.rs` 無傷
- ❌ brief の「足すものは新しいファイルへ」と字面で違う（行数の上限には収まる）。後続（`mcp-kanade-tools`）も `actor.rs` に腕を足すので、余白を先に使う

### 選択肢 B: brief どおり別ファイル（`actor_status.rs`）へ出し、`actor.rs` の中で `#[path]` で読み込む

- 分室の関数（`answer(&State, reply)`）と兄弟のテストを新しいファイルに置き、`actor.rs` には腕 1 つと `#[path = "actor_status.rs"] mod status;` の 2 行だけ。MCP の側は A と同じ。
- ✅ `actor.rs` の増えが最小・`actor_resources` と同じ「殻で答える問い合わせの分室」の並び・brief と約束の両方を守る
- ❌ 中身が数行の関数のために 1 ファイル増える。分室の宣言場所が `actor_resources`（`lib.rs`）と違う所になる（読み手が迷わないよう説明が 1 行要る）

### 選択肢 C: kanade が状態を書き出し、UI がその場で読む

- kanade が知らせを 1 件処理するたびに `render()` の値を共有の置き場（`Arc<Mutex<..>>` など）へ書き、UI は `get_status` が届いたらその場で読む。
- ✅ SHIORI の往復中でも待たずに答えられる（事実 5 の上限の心配が消える）。後続の `currentghost.status`（`currentghost-property-others`）も同じ置き場を読める
- ❌ 毎メッセージの後に書く所が要り、運行の相が変わるたびに確実に書くには `drive_translating` の中か `schedule/` に手が入る（C4 の約束・Out of scope に当たる）。要件の「kanade に問い、その時点の値を返してもらう口」と形が違う＝要件の改めが要る。往復中の値は「往復の前の値」になり、要件 1.2 の「同じ時点」の意味が揺れる

### 比べ

| 観点 | A | B | C |
|---|---|---|---|
| 触るファイル（kanade） | `msg.rs`・`actor.rs` | `msg.rs`・`actor.rs`・新 1（＋兄弟テスト） | `msg.rs` 以外に `actor.rs` の駆動・`spawn_*` の引数・場合により `schedule/` |
| C4 の約束 | 守る（`lib.rs`・`schedule/` 無傷） | 守る | 破る恐れ |
| 要件との合い方 | 合う | 合う | 要件の改めが要る |
| SHIORI の往復中 | 往復が終わるまで待つ（10 秒の上限あり） | 同左 | 待たない |

MCP の側の返事の待ち方にも 2 案ある（A・B のどちらにも付く）:

- **a. `mcp::later` に預ける**（`dump_surface` の装着待ちと同じ）。毎フレーム `try_recv` で覗く。終了の途中は置き場ごと落ちて入口が `NG:areka is shutting down` を返す（要件 3.3 のまま）。
- **b. 別のスレッドで `recv` して、そのスレッドから `ReplyTo::send`**（`dump_surface.rs` の `reply_elsewhere` と同じ・`ReplyTo` はスレッドをまたげる）。毎フレームの覗きは無いが、問い合わせごとにスレッドを 1 本起こす。

a は既存の口をそのまま使え、スレッドを増やさない。

## 5. 規模と危険

- **規模: S（1〜3 日）**。kanade に変種 1 つと数行の腕、MCP の `handle` の書き換え、テスト 2 群、差の文書 1 本、実機確認 1 回。すべて既存の作りのなぞり。
- **危険: 低**。新しい依存・新しい仕組みなし。状態の決め方には触らない。気を付けるのは事実 3（別れの台詞の再生中に `talking` が出ない）を要件 2.1 とどう整えるかと、事実 5（SHIORI の往復中は答えが遅れる）を差の文書にどう書くかの 2 点だけ。

## 6. 設計の段で決めること（要件ディスカッションへの申し送り）

1. **新しいファイルの置き方。** brief の「新規 `actor_status.rs`」は `lib.rs` に `mod` を足さないと置けない（C4 の約束に当たる）。A（`actor.rs` に直に数行）か B（`actor.rs` の中から `#[path]` で読み込む）か。どちらも `lib.rs` に触れない。
2. **要件 2.1 と、終了・切替の別れの台詞の再生中。** 今の導出はこの相で `talking` を立てない（事実 3）。要件 1.2（SHIORI へ送る値と同じ）と導出は範囲外のままにするなら、2.1 の「台本を再生している間」を「定常と起動の挨拶の再生中」に絞るか、差の文書に「別れの台詞の間は `talking` が出ない」と書くか。導出を直す（`talk_active_of` に 3 相を足す）なら `schedule/` に触れ、C4 の約束と範囲を越える（SHIORI へ送る `Status` も変わる＝別の spec の仕事）。
3. **SHIORI の往復中に届いた問い合わせ。** 往復が終わるまで答えが遅れ、10 秒を超えれば入口の上限の `NG:` と `warn!` 1 件になる（事実 5）。要件 4.2 は「再生・選択の終わり」を待たないことで、往復の終わりを待つことは許すと読めるか。読めるなら、差の文書に 1 行書くだけで済む。
4. **返信端の型。** `Option<String>`（`render()` の値そのもの・MCP の側は空文字へ写すだけ）か `ExecutionStatus`（呼び手が `render()` する・後続の `currentghost.status` が語の集合として読める）か。
5. **要件 3.2 の文言の取り方。** `resolve(None, args.ghost_name, Omitted::UseActive)` を呼んで入口と同じ判断から取る（事実 7・`mcp-ghost-name-match` の後もそろう）か、`ghost_name` の有無を `handle` の中で直に分けるか。前者なら、空文字の扱いが入口と食い違わない。
6. **要件 5.1⑹ の「運行の状態が変わらない」の比べ方。** 殻は問い合わせの前にも `sync_online` で通信中の写しを更新する（事実 4・既存の振る舞い）。比べるのは運行の相・選択の帳簿・トークに限るか。
7. **MCP の側の待ち方。** `mcp::later`（案 a）か、別のスレッドで待つ（案 b）か。

> 要件ディスカッション（2026-10-05）での扱い: 3 と 6 は要件の書き足しで決着した（要件 4.2 に往復中は往復の後に答える旨・要件 5.5 ⑸ に差の一行・要件 5.1 ⑹ に比べる対象）。2 は開発者との議題にした。1・4・5・7 と 7 節の「`currentghost.status` との相乗り」は設計の段（`/kiro-spec-design`）で決める。

## 7. 調べが要る点（Research Needed）

- **後続の `currentghost.status` との相乗り。** `currentghost-property-others` の brief は「`get_status` と先に着地した方の読み口を使う」と書く。プロパティの読みが同期で答えを要るなら、問い合わせの往復（A・B）は合わず、C の置き場が要るかもしれない。本 spec では決めず、後続が決める点として記録だけ残す。
- **切替の途中に `get_status` が届く時機。** `resolve.rs` の `active` は「切替の途中は置き場が空」と書く。置き場に古いゴーストが残っている間に問い合わせを送り、その kanade が別れの台詞の相で答える場面（事実 3 と重なる）がどれくらいの長さあるかは、実機確認のときに見る。
- **バルーンの写しの 1 フレームの遅れ。** 見えているバルーンの組は、フレームの終わりの届けの相（`crates/areka/src/emo2_boot/frame/status_report.rs` の `run_status_report_phase`）が kanade へ送る。MCP の要求を汲む系は Input の段なので、同じフレームで変わったバルーンは次のフレームの問い合わせから載る。kanade が SHIORI へ送る `Status` も同じ写しを使うので要件 1.2 は崩れないが、実機確認で「話し始めの直後に呼ぶと `balloon(…)` がまだ無い」ことがありうる。
