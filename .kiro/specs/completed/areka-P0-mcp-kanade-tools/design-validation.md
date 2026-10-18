# 設計の検証: areka-P0-mcp-kanade-tools

> 2026-10-18・本ブランチ（`ff0bdf1b`）。`/kiro-validate-design` の手順（分析 → 重大な問題 → 良い点 → GO／NO-GO）で、design.md・requirements.md・research.md・brief.md と steering を読み、設計が頼っている主張を実際のコードで確かめた。SSP には何も送っていない・実機も動かしていない。design.md・requirements.md・research.md・spec.json は変えていない。

## 1. まとめ

設計は要件 1〜7 を全部の部品へ割り当てており、同じウェーブの「触らない約束」（`src/change.rs`・`schedule/boot.rs`・`schedule/steady.rs`・`runtime.rs`・`crates/areka-mcp/**`・`main.rs`・`mcp/mod.rs`）を守ったまま組めることをコードで確かめた。設計の土台になる事実（起動の挨拶が `Steady{talk: Some(origin="boot")}` で流れること、翻訳の帳簿から出どころが取れること、エラー応答が送出点で「返事なし」に写ること、1,000 行の上限に収まること）はどれもコードと合っている。下の 3 点は実装の段（タスク）で手当てできる大きさで、設計を作り直す必要はない。**判定は GO**。

## 2. コードで確かめたこと

| 設計の主張 | 確かめた所 | 結果 |
|---|---|---|
| 上限の近いファイルの行数（`msg.rs` 930・`actor.rs` 901・`schedule/mod.rs` 969・`schedule/steady.rs` 957・`schedule/events.rs` 850） | `wc -l` | 一致。`schedule/translate.rs` 251・`schedule/boot.rs` 345・`areka-ghost/src/config.rs` 206 |
| `msg.rs` のファイルの中のテストは約 418 行 | `msg.rs` の `mod tests`（510〜927 行） | 一致。外へ出せば約 510 行になり、足しても約 560 の見込みは妥当 |
| `actor.rs` の停止と通知の手 4 つ（約 95 行）を兄弟へ出す | `stop_cause_of`・`handoff_of`・`notify_stop`・`send_notice`（782〜866 行・約 85 行） | 実際は約 85 行。足す量（振り分けの 2 腕・返事・`origin` の写し・檻の 1 行）を多めの約 35 行と見ても約 850 行で上限に余裕がある |
| `schedule/mod.rs` の型の塊を `types.rs` へ出しても `boot.rs` の `use super::{Action, ActiveTalk, Input, Phase, State, events, resources}` が変わらない | `boot.rs` 8 行目・`mod.rs` の `Input`〜`Action`（56〜444 行） | 成り立つ。`pub(crate) use types::{…}` で同じ名前が見え、子のテストの `use super::*` もそのまま解ける |
| 起動の挨拶は `basewareversion` の往復と同じ一括で `Steady` へ移り、知らせの切れ目では `Steady{talk: Some(origin="boot")}` で流れている | `boot.rs` の `to_baseware_version`（`origin: "boot"` が 2 か所）・`on_reply` の `BootVersion` + `Notified` の腕・`actor.rs` の `drive_translating`（入れ直しを一括が尽きるまで繰り返す） | 成り立つ。翻訳に預けた挨拶も、`TranslateDone` → `basewareversion` → `Steady` が同じ駆動の中で閉じる |
| `KanadeConfig` を構造体の字面で組む所は 0 か所 | `KanadeConfig {` の検索 | `msg.rs` の `KanadeConfig::new` の中だけ。本番で構成を作るのは `areka-ghost/src/config.rs` の `resolve_kanade_config` の 1 か所（`runtime.rs` 591 行目から呼ぶ） |
| `runtime.rs` は `ghost_name` に触れない | `runtime.rs` の `apply_boot_record_gate` ほか | 書き換えるのは `first_boot`・`vanish_count`・`first_boot_epilogue` など。`ghost_name` には触れない |
| 名前の無いゴーストの代わりの値が MCP の宛先の値と同じ字になる | `mcp/resolve.rs` の `active`・`listed_value`、`ghost_session.rs` の `ghost_dir`、`areka-parsers` の `ShioriMount::dir`（`ghost_root/ghost/master`） | 成り立つ（どちらも同じ根を `std::path::absolute` にし、末尾の区切りを除く） |
| SHIORI のエラー応答は「台本なし」になる（要件 2.4） | `actor.rs` の `round_trip_request`（エラー応答を GET なら `NoContent` に写し `warn!` を 1 件） | 成り立つ。`first_reply_of` の「`Failed` → `Failed`」に来るのは輸送路の失敗だけ |
| 翻訳を通るトークの出どころが出口で分かる | `schedule/translate.rs` の `before`・`after`・`capture`・`on_done`（`TranslateWait::source`） | 成り立つ。翻訳に預けた一括の出口には `StartTalk` が無く、`TranslateDone` の出口に 1 回だけ出る。輸送路の失敗では預けた一括が捨てられ記録も出ない |
| 外から台本を置き換えても、旧トークの後始末は今の置き換えと同じ | `schedule/steady.rs` の `on_reply` の置き換えの腕（採番・枠の差し替え・`clear_choice_ledger`・`choice_prev_talk = None`） | 設計の `on_send` の手順と同じ |
| `KanadeMsg`・`EventId` に変種を足しても他のクレートが壊れない | `KanadeMsg`・`EventId` の `match` の検索 | 他のクレートには網羅の `match` が無い（`_` の腕だけ）。kanade の中で網羅しているのは `actor.rs` の 2 か所と `as_str`・`msg.rs` のテストで、どれも設計の変更の一覧に在る |
| `on_translate` は変えない（他のクレートの呼び手 2 か所） | `emo2_boot/spine_conformance_script.rs`・`areka-ghost/tests/ghost/spine_e2e_test.rs` | 一致 |
| script 種別の記録の形は `get_log` の読み方と合う | `crates/areka/src/log_history.rs` の `TARGET_SCRIPT`・`draft`（欄 `ghost`・`label` を名と表示の語として読む） | 一致 |
| 同じウェーブの `mcp-reload` と触るファイルが重ならない | `.kiro/specs/areka-P0-mcp-reload/brief.md`（触るのは `src/change.rs`・`schedule/boot.rs`。`msg.rs`・`actor.rs`・`schedule/mod.rs` には触らないと明記） | 重ならない |

## 3. 重大な問題（3 点・どれも実装の段で手当てできる）

### 問題 1: `SstpNotify` の名前と Reference の検査を呼び手に任せており、破られるとゴーストが止まる

**懸念**: `KanadeMsg::SstpNotify` は「呼び手が `sstp::validate_notify` で検査済みであること」を前提にし、kanade の運行表（`schedule::sstp::on_notify`）は検査しない。検査を破った依頼が来ると、送出の檻（`round_trip_raw`）が `Failed(Internal)` を返し、`on_notify_reply` はそれを `super::on_shiori_reply` へ渡す。`Steady` は応答待ちの相に数えられるので（`schedule/mod.rs` の `awaits_reply`）、そのまま `Unloading{Fault}`＝ゴーストが倒れる。

**影響**: 設計はこの口を `property-query-channels`・`mcp-shiori-query` が使う「許可の表を通さない口」として渡している（設計「申し送り」）。後の呼び手が検査を 1 か所忘れるだけで、改行の入った名前 1 つでゴーストが落ちる。要件 2.1 の「表に無い名前を送ったことを理由にゴーストを止めない」の趣旨にも、入口の外側の守りが倒れる形は合わない。今の MCP の 2 本だけなら起きないので、今すぐ壊れるものではない。

**提案**: `on_notify`（運行表）でも `validate_notify` を 1 回呼び、破れていれば SHIORI へ送らず状態も変えずに `NotAccepted` で答える（記録は `warn!` 1 件＝呼び手の誤りの知らせ）。送出の檻の規則は今のまま 2 段目の守りとして残す。決定論テストに「CR を含む名前の `SstpNotify` を `step` へ直に渡すと GET が出ず相が変わらない」を 1 本足す。設計の契約は `KanadeMsg::SstpNotify` の説明の 1 文と `on_notify` の説明が変わるだけ。

**根拠**: 設計「SstpEgress」（「入口で検査済みなので構造上は起きない」）、`schedule/mod.rs` の `on_shiori_reply`・`awaits_reply`、`actor.rs` の `round_trip_raw`。

### 問題 2: 空の台本のトークも 1 件ずつ記録され、`get_log` の `script` が空の行で埋まるおそれがある

**懸念**: `script_log::record` は出口の `Action::StartTalk` を全部記録する。今の areka は、SHIORI が「200・`Value` は空」を返すと空の台本でトークを始め（`schedule/steady.rs` の `on_reply`。翻訳は `translate_skipped_empty` で飛ばす）、初回の起動で `OnBoot` が 204 のときも起動の記録のための空の台本のトーク（`boot.rs` の `to_baseware_version` の epilogue だけの腕）を始める。設計の表はこれを `Ghost` の語で記録する形にしている。

**影響**: 何も話さない間も毎秒の `OnSecondChange` に「200・空の `Value`」で答える SHIORI では、毎秒 1 行の空の記録が script 種別に積まれ、種別ごとの上限（`log_history.rs` の `PER_KIND_CAP` 1000）を十数分で使い切って本物の台本の記録を押し出す。標準出力にも毎秒 1 行出る。SSP の `script` は再生したさくらスクリプトの記録で、空の行は出ない（要件の実測 1・6）。要件 7.6 ⑹ の実機の確かめ（MCP で送った台本・`raise_event` の台本・自発の台本が読める）が、こうしたゴーストでは崩れうる。

**提案**: `record` で台本が空（0 文字）の `StartTalk` は記録しない（`trim` まではしない＝空白だけの台本は再生されるので記録する）。決定論テスト ⑽ に「`Value("")` の応答で始まったトークと epilogue だけのトークは 0 件」を足し、差の一覧の ⑻ に「台本が空のトークは記録しない」と 1 行書く。要件 4.1 の「再生を始める」の読みとして、中身の無いトークは数えない、を設計の注記に残す。

**根拠**: 設計「script 種別のログの表示の語」の表の最後の行、`schedule/steady.rs` の `on_reply`（`Steady{None}` + `Value`）、`schedule/translate.rs` の `capture`（0 文字は翻訳しないまま再生）、`boot.rs` の `to_baseware_version`、`log_history.rs` の `PER_KIND_CAP`。

### 問題 3: 殻のテスト「起動の途中は `NotAccepted`」の作り方が、知らせを 1 通ずつ処理する作りと合わない

**懸念**: テストの計画の「kanade 殻」の 3 は「起動の途中（`Boot` の前・`Boot` 直後に止めた偽の shiori）→ `NotAccepted`」と書く。だが殻は `Boot` の知らせを 1 回の駆動で `Steady` まで進め、その間ほかの知らせを処理しない（要件 3.7・設計「Existing Architecture Analysis」）。偽の shiori を起動の途中で止める（答えを返さない）と、後から送った `SstpSend` は受信箱で待つだけで返事が来ず、テストが止まるか、上限の待ちに頼る形になる。偽の shiori を落とす意味なら往復の失敗で `Fault` になり、`NotAccepted` ではなく `Failed` が返る。

**影響**: 実装係がこの書き方どおりに作ると、決定論でないテスト（実時間の待ちに頼る）か、意図と違う結果を固定するテストになる。メモリの「決定論的テスト網羅は必達」に当たる。

**提案**: 殻のテストの 3 を、知らせの切れ目で実際に起こる 3 つの場面に書き直す: ⑴ `Boot` を送る前（`Idle`）→ `NotAccepted` ⑵ 偽の shiori が `OnBoot` に台本を返し、挨拶の `TalkDone` をまだ送っていない（`Steady{talk: Some(origin="boot")}`）→ `NotAccepted`・SHIORI へ何も送らない ⑶ 挨拶の `TalkDone` を送った後 → `Accepted`。起動の各相（`BootMain` など）の拒否は、運行表のテスト ⑷ が `step` を直に呼んで固定しているので、殻では重ねない。

**根拠**: 設計「Testing Strategy」の「kanade 殻」3、`actor.rs` の `drive_translating`、`boot.rs` の `on_reply`。

## 4. 良い点

- **既存の口を広げず、新しい入口を足した**: `KanadeMsg::RaiseEvent` と `RaiseOutcome`（`src/change.rs`）を 1 字も変えず、`SstpSend`・`SstpNotify` と出所カテゴリ `EventId::Sstp` を足す形にしたので、要件 6.1・6.2 が型の上で守られる。ゴーストの名前を `config.rs` の中で入れる・型の塊を同じ名前で見せて `boot.rs` を変えない・共通の部品を `sakurascript.rs` に置いて `mcp/mod.rs` に触らない、と、同じウェーブの約束を全部回避の形で満たしており、どれもコードで成り立つことを確かめた。
- **記録の置き場の選び方が正確**: script 種別の記録を運行表の出口（`step` の最後）に置いたことで、元のイベント（`reply_source`・`TranslateWait::source`）と翻訳の後の台本の両方が手元にあり、翻訳に預けた一括は出口に `StartTalk` が無いので重複せず、輸送路の失敗では記録が出ない（要件 4.1・4.3）。`raise_event` の名前も殻の固定のラベルでなく元のイベントから取るので本当の名前になる。決定論テストも `step` を呼んだスレッドで記録を捕まえられる。

## 5. 判定

**GO**。

- **理由**: 要件 1〜7 の全項目に部品と契約と流れが割り当てられ、設計が頼る事実（挨拶の相・翻訳の帳簿・エラー応答の写し・型の分け方・行数・他の spec と触るファイル）はコードと合っている。上の 3 点は、運行表の 1 か所の検査の追加・記録の 1 条件・テストの場面の書き直しで、どれも設計の骨格（入口・受け付けの判定・記録の置き場・型の分け方）を変えない。
- **次の段**: `/kiro-spec-tasks areka-P0-mcp-kanade-tools` でタスクを作るときに、問題 1〜3 の手当て（`on_notify` の検査と 1 本のテスト・空の台本を記録しない条件とテストと差の一覧の 1 行・殻のテスト 3 の 3 場面）をタスクの中身に入れる。設計の段で直すなら、design.md の `KanadeMsg::SstpNotify`・`on_notify` の説明、ScriptLog の表、殻のテストの 3 の 3 か所だけを改める。
