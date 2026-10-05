# 設計の検証レポート: areka-P0-mcp-get-status

> 2026-10-05・本ブランチで実施。設計（design.md）と調べ（research.md）の主張を、実物のコードを読んで確かめた（ビルド・テストは回していない）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。対話なしの検証なので、開発者への問いは「設計ディスカッションへ送る点」にまとめた。

## 総評

**判定: GO（実装へ進める）。** 設計が頼っている既存のコードの性質は、確かめた限りすべて実物と合っていた。足すものは「問い合わせの変種 1 つ・殻の関数 1 つ・`handle` の中身」だけで、既存の形（`KanadeMsg::ResourceQuery`・`mcp::later`）をそのままなぞっており、要件 1〜5 のすべてに行き先がある。設計を止める欠陥は 0 件。下の 2 点は設計ディスカッションで決めれば済む大きさである。

## 実物で確かめたこと

| # | 設計の主張 | 確かめた結果 | 根拠 |
|---|---|---|---|
| 1 | 知らせと知らせの間の `State::snapshot()` は、握手の相では `snapshot_without_talk()` と同じ値になる | **正しい** | 定常の相から出る書き込みは `begin_close`（`schedule/steady.rs`）・`begin_change`（`schedule/change.rs`）・`force_quit`・`to_unloading_quit`・`to_unloading_fault`（`schedule/mod.rs`）で、どれも同じ処理の中で `clear_choice_ledger` を呼ぶ。選択待ちの帳簿を作れるのは定常の相だけ（`schedule/mod.rs` の `route` の `Input::ChoiceWaiting` の腕が定常以外を捨て、`steady::on_choice_waiting` も `Phase::Steady { talk: Some }` だけを受ける）。`talk_active_of` は `Steady{Some}` と `BootVersion{Some}` だけを真にし、`nouserbreak` は「再生中 かつ 旗」（`snapshot_with_choice`）。帳簿の途中の段（`Cascading`・`TimeoutInFlight`）は 1 件の処理の中で終わるので、知らせの間には残らない |
| 2 | 起動の挨拶の間も食い違わない | **正しい** | `schedule/boot.rs` の `to_baseware_version` は相を `BootVersion{talk}` にしてから `state.snapshot()` で `Status` を作る（握手の作り方を使わない）。起動の各段は `Boot` 1 件の処理の中で終わる（`actor.rs` の `drive_translating` が応答を入れ直して回す） |
| 3 | 新しい腕は `ResourceQuery` の腕の隣・`sync_online` の後・`step` の前に置ける | **正しい** | `actor.rs` の `spawn_kanade_translating` の振り分けは、`sync_online` → `match msg` の順で、`ResourceQuery` の腕は `actor_resources::answer(&state, …)` を呼んで `return Ok(ControlFlow::Continue(()))` で抜ける。同じ形で置けば `drive_translating` も、返事の後始末（汎用の通知・台詞の切れ目）も通らない |
| 4 | そこで答えても運行の状態は変わらない（要件 4.1） | **正しい（ただし書きつき）** | 関数が `&State` を受ける限り書き換えられない。唯一の書き込みは腕の前の `sync_online`（通信中の写し）で、これは全部の知らせに共通の既存の振る舞い。要件 5.1⑹ が比べる対象から外している |
| 5 | SHIORI との往復の間は答えが遅れる・止まれば返信端が落ちる | **正しい** | `actor.rs` の `round_trip` 系は上限なしで待つ。`areka-actor` の `run_inbox` は処理が `Break` を返すと戻り、受信箱ごと積み残しが落ちる。`ReplyReceiver::try_recv` は送り手の消滅を `Err(ReplyError::Dropped)` で返す（`crates/areka-actor/src/reply.rs`）。止まった後の `Sender::send` は `Err` |
| 6 | `mcp::later` は上限で待つ側が居なくなった組を覗かずに捨て、終了の途中は置き場ごと落ちる | **正しい** | `crates/areka/src/mcp/mod.rs` の `poll_later`（`reply.is_abandoned()` なら捨てる）と `close`。置き場が無いときの `later` は組をその場で落とす＝橋が「終了の途中」と答える |
| 7 | 切替の途中に届いた呼び出し | **正しい** | `drain` は 1 件ごとに `resolve::active` を読み直し、同じ呼び出しの中で `handle` まで進むので、解決と `handle` の間に置き場は変わらない。預けた後に降りた分は返信端の消滅（`Dropped`）で受ける。`resolve::resolve(None, …, Omitted::UseActive)` は必ず `Err` を返し、省略・空文字は `NOT_ACTIVE`、名前ありは `CANNOT_FIND`（`mcp/resolve.rs`）＝要件 3.2 の 2 通りと合う |
| 8 | 解決を通ったのに kanade の送り口が無いことは本番では起きない | **正しい** | `ghost_session.rs` は `kanade` の欄を実行系から写して入れる（実行系があれば必ず `Some`）。`resolve::active` は実行系のある置き場だけを起動中と読む。この枝に届くのはテスト（`GhostSession::for_test(None, …)`・空の World）だけ |
| 9 | `mcp_tests.rs` に触らずに済む | **正しい** | `get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure` は空の World へ振り分け、答えが解決の失敗の 2 つの文言でないことだけを見る。新しい `handle` は置き場なしで `NG:Status is not available` をその場で答えるので緑のまま |
| 10 | kanade の統合テストの土台で K1〜K3 の場面を作れる | **正しい** | `tests/kanade/external_status_test.rs` が同じ土台（`spawn_harness_gated`・`RecordedCall` の `status`）で、再生中の周期の要求の `Status` を逐語で突き合わせている。選択待ちは `KanadeMsg::ChoiceWaiting` を直に送れば `on_choice_waiting` が現行のトークと突き合わせて帳簿を作る |
| 11 | M6 の前提（起動の挨拶は台詞の時計を進めるまで終わらない） | **正しい（条件つき）** | `ghost_switch_test_support.rs` の `SwitchRig` は `TickerMode::Disabled` で起こし、台詞の Tick を入れるのは `pump_talking_until` だけ。`pump_input_until`（時計を進めない）で回す限り再生の完了は kanade へ届かない。問い合わせは `Boot` の後ろに並ぶので、答える時点の相は `Steady{talk: Some}`（挨拶を引き継ぐ） |
| 12 | 行数（上限 1,000）とテストの置き場 | **収まる** | `msg.rs` 909 行・`actor.rs` 876 行・`get_status.rs` 16 行（実測）。足す量の見立て（各 10〜25 行・80 行前後）で上限の内。`tests/kanade/status_query_test.rs` は `resource_query_test.rs`・`external_status_test.rs` と同じ並びで、束ねの `tests/kanade.rs` に宣言を 1 つ足す形も過去の追加と同じ |
| 13 | `KanadeMsg` を網羅して `match` するのは 2 か所 | **正しい** | `actor.rs` の振り分けと、`msg.rs` のテスト `existing_eight_kanade_msg_variants_are_unchanged_by_additive_growth` の `label`。ほかのクレートは個別の変種を取り出すだけ |

## 設計ディスカッションへ送る点（2 件・どちらも実装を止めない）

### 論点 1: 要件に無い枝 `NG:Status is not available` の位置づけと記録の強さ

- **気がかり**: 「置き場に kanade の送り口が無い」の枝は要件 3 の答えの一覧に無い。設計は areka 独自の文言と `error!` 1 件で受ける。枝そのものは妥当（本番では届かない食い違いを「降りた」の `debug!` に混ぜると記録に残らない。`get_property` の `Property system is not available` と同じ並び）。ただし 2 点が決まっていない。⑴ 記録の強さが兄弟と食い違う: `get_property.rs` は同じ場面を `warn!` で残し、`.kiro/steering/logging.md` は `error!` を「致命的・回復不能」としている。⑵ この文言は外から見える答えなのに、要件のどこにも載っていない（設計は差の一覧の補足 1 行に書くとしている）。
- **影響**: 小さい。本番では届かない枝で、エージェントの使い方は変わらない。食い違うのは記録の強さの一貫性と、要件と設計の対応表（この枝に対応する要件の番号が無い）。なお既存の `mcp_tests.rs` のテストは、今後この枝を通って記録を 1 件出すようになる（判定には響かない）。
- **提案**: 記録は `get_property` に合わせて `warn!` にするか、`error!` のままにするなら理由を 1 行書く。要件は直さず「設計の段で足した守りの枝（要件 3.2 の外・本番では届かない）」として設計と差の一覧に残す形で足りる、という扱いをディスカッションで確かめる。
- **要件**: 3.2・3.5・5.5 ／ **設計の箇所**: Boundary Commitments「areka 独自の文言 1 つ」・Error Handling の表・Testing Strategy の M5

### 論点 2: M6（本物の単位で端から端まで）の「再生が終わった後」の待ち方が書かれていない

- **気がかり**: M6 の ⑴（時計を進める前は `talking`）は、時計を進めない回し方（`pump_input_until`）を使う限り決定的で、前提は実物で確かめられた。一方 ⑵（再生が終わった後は `talking` を含まない）は、再生の完了が kanade へ届くのが別スレッド越しで、`SwitchRig` には「完了が kanade に届いた」ことを外から知る口が無い。台詞を進めた直後に 1 回だけ問うと、まだ `talking` が返ることがありうる。あわせて、`SwitchRig` は MCP の受け口を据えない（`mcp::install` を呼ばない）ので、テストが自分で据える必要がある（M1〜M5 には書いてあるが M6 には無い）。
- **影響**: 書き方しだいで、負荷のかかった並走のテストでだけ赤くなる不安定なテストになる。
- **提案**: ⑵ は「台詞の時計を進めながら `get_status` を問い直し、`talking` を含まない答えが上限の内に返ること」と書く（`pump_talking_until` の終わりの条件を答えの中身にする）。M6 の準備に「`rig.world` へ `mcp::install` を据える」を足す。本文の期待は、`Input` の段だけを回す間はバルーンの届けの相が走らないので、⑴ はちょうど `talking` になる見込み。
- **要件**: 1.1・2.1・4.2・5.2 ／ **設計の箇所**: Testing Strategy の M6・research.md 8.4 の 3 つ目

## 設計の良い点

- **状態の持ち主に聞く 1 本の道にしたこと。** SHIORI へ送る `Status` と同じ素（`State::snapshot`）・同じ型（`ExecutionStatus`）を返すので、要件 1.2 が「同じ作り方だから同じ」で成り立ち、写しを別に持つ案（往復の前の値になる）を避けている。握手の相で作り方を分けなくてよいことも、実物で裏が取れた。
- **要件 5.3 の 2 段の固定が到達する経路を踏んでいること。** kanade の本物の殻で「問い合わせの答え＝次の周期の要求の線の値」（K1・K2）を、アプリ本体で「答えの `render` の値＝本文」（M2）を見て、M6 が振り分けから本物の kanade までをつなぐ。アプリ本体の土台が線の `Status` を外へ出していない（`spine.rs` は 1,000 行ちょうど）という制約の中で、他の spec のファイルに触らずに済む分け方になっている。

## 小さな気づき（議題にしない・実装で拾えるもの）

- **降りたときの kanade 側の裏付け。** 「止まった kanade の受信箱に残った問い合わせは返信端ごと落ちる」は MCP の側では偽の送り口でだけ固定される（M3）。標準の受け渡しの性質なので必須ではないが、`status_query_test.rs` に「`Close` の後ろに並べた問い合わせの返事は `Dropped` になる」を 1 本足せば、要件 3.2 の kanade 側の半分も本物の殻で固定できる。
- **残りの余白。** 本 spec の後、`actor.rs` は 900 行前後・`msg.rs` は 920 行前後になる。後続の `mcp-kanade-tools` が同じ 2 ファイルに足せる量は 80〜100 行ほど。
- **上限を過ぎた問い合わせの後始末。** SHIORI が 10 秒を超えて考えている間に何度も呼ばれると、問い合わせが kanade の受信箱に呼び出しの数だけ溜まり、往復の後にまとめて「受け手が居ない」の `debug!` になる。溜まる量は呼び出しの数どまりで害は無い。
- **`tests/kanade.rs` の頭の説明**（「本ファイルは以後編集不要」）は今の実態と合っていない（既に何度も宣言が足されている）。本 spec で直す必要は無い。
- **K1 の最初の周期の要求**は、中断の旗が立っていてもトークが無いので `balloon(0=0/1=0)` だけになる（`nouserbreak` は載らない）。期待の列を書くときの注意。

## 最終判定

- **判定**: **GO**
- **理由**: 設計が頼る既存のコードの性質 13 点がすべて実物と合い、要件 1〜5 に抜けが無く、並走の約束（kanade の `schedule/`・`lib.rs`、MCP の `mod.rs`・`handler.rs` に触らない）を本質の形のまま守れている。残る 2 点は記録の強さとテストの書き方の詰めで、設計の形を変えない。
- **次の一歩**: 設計ディスカッションで論点 1・2 を決め、`/kiro-spec-tasks areka-P0-mcp-get-status` へ進む。
