# ギャップ分析: areka-P0-script-security-level

- 実施日: 2026-10-22（ブランチ `claude/areka-p0-script-security-78150a`・`d77b5ea6` の上）
- 入力: `requirements.md`（要件 1〜5）・`brief.md`（棚卸㉒〜㉘の再測定）・steering（`roadmap.md` の C10 の約束と 1 ファイル 1,000 行の決まり・`logging.md` のログの段の基準。`product.md`・`tech.md`・`structure.md` は brief と要件に写された前提で足りるとして通読していない）
- 方法: コードを Grep／Read で読み、数は 2026-10-22 の作業ツリーで数え直した。ukadoc は MCP で 1 回引き直した（要件 4.1 の 3 つの `\![execute,…]` の注記を確かめた）。

---

## 1. 分析の要約

- **SHIORI への要求の組み立ては 1 か所で、出どころを受け取る口が無い。** 要求の文を組むのは `crates/shiori-host32-host/src/shiori3.rs` の `build_request` だけ（32bit の仲介の経路と同じプロセスの経路の両方がここを通る）。`SecurityLevel: local` を **`Reference*` の後＝`ID` より後ろ**に固定で書いており、要件 2.1 の並び（`SecurityLevel` は `ID` より前）とも違う。`Sender` は `"areka"` 固定（`client.rs` の `DEFAULT_SENDER`・`shiori_inproc.rs` の `SENDER`）で、`MCP` を名乗る道が無い。
- **出どころの手がかりは既に kanade の中に散らばっている。** 送るイベントの ID の型 `EventId` が `Static`（運行表）・`Choice`（選択肢とアンカー）・`Sstp`（MCP の `raise_event`）の 3 つを型で区別し、翻訳の出どころ `ScriptSource` が `Event`／`SstpSend`（MCP の `sakurascript`）を区別し、`OnTranslate` の `Reference1`（`sstp-send,owned`・空）は **既に要件 2.4〜2.6 のとおり**に出ている。足りないのは「この 3 つを 1 つの値（出どころ）にまとめ、線のヘッダと再生まで運ぶ」部分。
- **応答の `SecurityLevel` を上へ返す道が無い。** `parse_response` は `Value`・`ErrorLevel`・`ErrorDescription` だけを読み、`ShioriBackend::get` の戻りは `Option<String>`、kanade の `ShioriOutcome::Value(String)` も台本の文字列だけを運ぶ。
- **タグを止める受け手には「どのトークか」は届くが「どこから来たか」は届かない。** `\![enter,nouserbreakmode]` の受け手 `NoUserBreakCueSink`（`crates/areka/src/emo2_boot/user_break_cue.rs`）は合図だけを見る。配送（`crates/areka-ghost/src/dispatcher.rs` の `on_start`）がトークごとに受け手を複製して `BootCueSink::begin_talk(talk_id)` を呼ぶ先例があり、ここで出どころも渡せる。
- **規模は 16〜20 タスク（L）。** 20 を超えるかどうかは、型に欄を足したときの機械的な追随（テストの組み立て 60〜150 か所）を何タスクに数えるかで決まる。超えるなら brief の切り方（⒜ ヘッダと応答の読み取り → ⒝ 台本の印とタグの止め）がそのまま使える。

---

## 2. 今のコードの姿（要件に関わる所）

### 2.1 SHIORI への要求の組み立て（要件 2）

| 場所 | 今の振る舞い | 要件との差 |
|---|---|---|
| `crates/shiori-host32-host/src/shiori3.rs` `build_request`（94〜148 行） | 並びは `Charset`・`Sender`・`Status`・`ID`・`Reference*`・`SecurityLevel: local`。`ShioriRequest` の欄は `method`・`id`・`references`・`sender`・`status`・`charset` | `SecurityLevel` の位置（要件 2.1）・`SecurityOrigin`・`SenderType` が無い（2.1〜2.7）・値が固定（2.3〜2.6） |
| 同 `ShioriRequest` | `status` は「値を解釈しない wire 値」（kanade が語彙を持つ・DD-IT-6） | 同じ型の扱い（文字列のまま受けて書くだけ）で `SecurityLevel`・`SenderType`・`Sender` を足せる＝host32 に出どころの語彙を持ち込まずに済む |
| `crates/shiori-host32-host/src/client.rs` `Shiori3Client::get`／`notify`（142・188 行） | `sender: self.sender`（既定 `"areka"`）で `ShioriRequest` を組む | 1 回の往復ごとに `Sender` を変える口が無い |
| `crates/areka-ghost/src/shiori_inproc.rs` `build_input`（405 行）・`SENDER`（328 行） | 同じ `build_request` を `Charset::UTF_8` 固定で呼ぶ | 同上。要件 2.8（2 つの経路で同じヘッダ・同じ順）は、両方が `build_request` を通る形を保てば構造で満たせる |
| `crates/areka-kanade/src/shiori/real.rs` `ShioriBackend`（62 行） | `get(id, references, status)`・`notify(…)`。既定の実装を持つメソッドの先例が 2 つ（`on_idle`・`unblock_handle`） | 出どころのヘッダを渡す引数が無い。**実装は 21 か所**（本物 2＝`ShioriConnection`・`InProcBackend`／残りはテストの偽物と結線の包み） |
| `crates/areka-kanade/src/shiori/real.rs` `handle_call` | `ShioriCall` の `id`・`references`・`status` だけを backend へ渡す | 出どころを渡す 1 か所 |
| `crates/areka-kanade/src/actor.rs` `round_trip_raw`（631 行） | 送る直前の唯一の関所（許可表の検査・欠番の印・`shiori_request` の記録）。`EventId` の出どころ別に受理規則を分けている | ここで `EventId` から出どころを導ける。呼び手は 2 つ（`round_trip_request` と翻訳の `actor_translate::run_translate`） |
| `crates/areka-kanade/src/msg.rs` `ShioriCall`（351 行） | `Get`／`Notify` に `id: EventId`・`references`・`status` | 欄を足すと**製品だけで 50 か所**の組み立てに波及（`schedule/events.rs` 25・`resources.rs` 9・`actor.rs` 6 ほか） |

### 2.2 応答の読み取り（要件 3）

| 場所 | 今の振る舞い | 要件との差 |
|---|---|---|
| `shiori3.rs` `parse_response`（249 行）・`ParsedResponse` | ヘッダ名は大文字小文字を無視して比べる。`Value`・`ErrorLevel`・`ErrorDescription` のほかは読み飛ばす | `SecurityLevel` の腕が無い（3.1）。値の大文字小文字の無視（3.3）と、読めない値の `warn!`（3.4）も無い |
| `client.rs` `map_get_result`・`shiori_inproc.rs` `map_get_outcome` | どちらも `Result<Option<String>, RequestError>`＝台本だけを返す | 応答の `SecurityLevel` を返す口が無い |
| kanade `ShioriOutcome::Value(String)`（`msg.rs` 365 行） | 台本の文字列だけ | 応答の `SecurityLevel` を運ぶ場所が無い。`ShioriOutcome::Value(` の出現は全体 133（製品 23） |
| kanade `Input::ShioriReply { outcome, origin }` | `origin` は応答の出所のイベント ID（`&'static str`・記録用） | 全体 151 の出現（製品 30）。ここに欄を足すとテストの追随が広い |

### 2.3 台本の出どころの運搬（要件 1）

| 場所 | 今の振る舞い | 要件との差 |
|---|---|---|
| `crates/areka-talk/src/lib.rs` `StartTalk`（33 行） | `talk_id`・`script`・`epilogue`。`StartTalk::new` で `epilogue` を空にする先例 | 出どころ・実効のセキュリティレベルの欄が無い（1.4）。**欄を足すと構造体の字面の組み立て 65 か所（製品 3・テスト 62）**が追随 |
| `crates/areka-talk/Cargo.toml` | 依存 0 のクレート。kanade・sakura・ghost・areka が全部これに依存する | 出どころの型の置き場の第一候補（全員から見えて、下に何も引かない） |
| kanade `schedule/mod.rs` `start_talk`（106 行） | MCP の `sakurascript`（`sstp.rs` の `on_send`）・`raise_event` の応答（`on_notify_reply`）・選択肢の `script:`（`steady_choice_script.rs`）が通る共通の手 | ここに出どころを渡せば 3 本の入口がそろう |
| kanade `schedule/types.rs` `ActiveTalk`（158 行） | `talk_id`・`origin: &'static str`（記録用の名札）・`script` | 選択肢の `script:` が元のトークの出どころを引き継ぐ（1.6）には、再生中のトークの出どころの控えが要る。字面の組み立ては 28 か所 |
| kanade `schedule/translate.rs` の出口の規則（`before`／`after`／`capture`／`on_done`） | SHIORI の応答で始まるトークの一括を **1 か所で**捕まえて翻訳へ預け、翻訳の結果で台本を差し替える | 応答の `SecurityLevel`（3.1）と、翻訳の応答の `SecurityLevel`（3.1 の「OnTranslate の応答を含む」）を、トークの開始へ写す一番狭い場所。腕ごと（`boot.rs` 2・`change.rs` 2・`close.rs` 2・`steady.rs` 4）を触らずに済む |
| kanade `schedule/script_log.rs` | 出口で出どころを見て記録の表示（`Ghost:<名前>`・`SSTP(Local,Auth)`・`Ghost`）を決める | 要件 5.1 は表示を変えない。出どころの型を入れても、この表示は今の `ScriptSource` から導くまま残せる |
| `crates/areka-kanade/src/{change.rs,…}` の `ChangeOrigin`・`BootOrigin` | 切替と起動の理由 | 台本の出どころの先例にはならない（brief のとおり） |

### 2.4 タグの止め（要件 4）

| 場所 | 今の振る舞い | 要件との差 |
|---|---|---|
| `crates/areka/src/emo2_boot/user_break_cue.rs` `NoUserBreakCueSink` | `("enter","nouserbreakmode")` → `Enter` を UI へ送る。`Clone` の一括の実装で `BootCueSink` を満たしている | 出どころを知らない（4.2・4.4） |
| `crates/areka-ghost/src/sink.rs` `BootCueSink::begin_talk(talk_id)` | 配送がトークごとの複製の直後に 1 回呼ぶ。既定は何もしない。上書きの先例は `BalloonLifecycleSink`（`talk_lifecycle.rs` 234 行） | 出どころを一緒に渡せば、受け手が自分で止められる（「`\!` は汎用キャリア 1 本・消費側が名前で選ぶ」の決まりに沿う） |
| `crates/areka-ghost/src/dispatcher.rs` `on_start`（325 行） | `StartTalk` を受けて受け手を複製し、`begin_talk` を呼んで `spawn_talk` へ | `StartTalk` に出どころが載れば、ここから受け手へ渡せる |
| `crates/areka/src/emo2_boot/consumer_ledger.rs` | 実装済みの `\!` の受け手の正準台帳（`consumer_of(name, selector)`） | 要件 4.7（表のタグが実装済みなのに止めが無い状態を赤にする）の照合に**読むだけ**で使える。**C10 の約束で本 spec はこのファイルに触らない**（`window-link-directives` の持ち物） |
| 止めるタグの表 | 無い | 新しく作る（4.1・4.8） |
| `\m[umsg,…]` | `decode_tag` の既定の腕から Raw へ落ちて捨てられる（未実装・網羅台帳 `absent`） | 表に載せるだけ（4.6） |
| `\![execute,filewatch]`・`schedule-add`・`schedule-delete` | 未実装 | 表に載せるだけ（4.6）。**網羅台帳 `sakura-script.toml` にこの 3 つの行が無い**（下の「調べが要る点」） |

### 2.5 記録と台帳（要件 5）

- `doc/ukadoc-coverage/ledger/shiori.toml`: 要求の `SecurityLevel` は群 10（`degraded`）、`SenderType`・`SecurityOrigin`・応答の `SecurityLevel` は群 11（`absent`・持ち主は空）。書き換える行は特定できる。
- `doc/ukadoc-coverage/ledger/sakura-script.toml`: `\![enter,nouserbreakmode]`（1027 行・`implemented`・持ち主 `areka-P0-balloon-break`）と `\m[umsg,wparam,lparam]`（5261 行・`absent`）の行がある。
- `doc/COMPAT_ARCHITECTURE.md` §8: areka の裁量（例: `Sender: areka`・選択肢の ID とアンカーのイベントを「ゴースト自身」とする）を登記する先。`OnTranslate` の `Status` の裁量（336 行）が `SecurityLevel` の引き継ぎに触れている。

---

## 3. 要件と資産の対応表

| 要件 | 使える資産 | 区分 |
|---|---|---|
| 1.1〜1.3 出どころの付与 | `EventId` の 3 分類・`ScriptSource`・`sstp.rs` の 2 つの入口 | **Missing**（値の型）・入口の手がかりはある |
| 1.4 再生の終わりまで保つ | `StartTalk`（台本 1 つにつき 1 つ）・トークごとの受け手の複製 | **Missing**（`StartTalk` の欄）・**Constraint**（字面の組み立て 65 か所） |
| 1.5 翻訳の元を引き継ぐ | 翻訳の帳簿 `TranslateWait`・`TranslateRequest` に `source` がある | 一部あり（帳簿に出どころとレベルを足す） |
| 1.6 選択肢の `script:` が引き継ぐ | `steady_choice_script.rs` の `begin`→`start_talk`・`ActiveTalk` | **Missing**（`ActiveTalk` の控え） |
| 1.7 選択肢の ID・アンカーは「ゴースト自身」 | `EventId::Choice`（`anchor.rs`・カスケード） | 既に分類済み。「ゴースト自身」に写す規則を書くだけ |
| 1.8 下流の 9 語・Owned・local／external で表せる | — | **Missing**（型の設計） |
| 2.1 並び | `build_request` | **Constraint**（並びを変える＝字面を固定したテストの書き換え: `shiori3.rs` の中のテスト 3 本・`shiori3_charset_tests.rs`・kanade の `tests/kanade/translate_test.rs` の `header_shape`） |
| 2.2〜2.6 値 | `EventId`・`ScriptSource`・`REF1_SSTP_SEND_OWNED` | **Missing**（出どころ → 4 つのヘッダの写し 1 か所） |
| 2.7 語のつなぎ方 | — | **Missing**（小さな純関数） |
| 2.8 2 つの経路で同じ | 両方が `build_request` を通る | 構造で満たせる（照合のテストは要る） |
| 2.9 他のヘッダの値は不変 | `charset`・`status`・`Reference` の欠番の扱い | 守るだけ（既存テストが番人） |
| 3.1〜3.4 応答の `SecurityLevel` | `parse_response` のヘッダ名の比べ方 | **Missing**（腕・backend の戻り・kanade の運搬） |
| 3.5 他の応答の読み方は不変 | 既存テスト | 守るだけ |
| 4.1・4.8 表 | — | **Missing** |
| 4.2・4.4 止めて記録 | `NoUserBreakCueSink`・`begin_talk` の先例 | **Missing**（受け手が出どころを知る口） |
| 4.3・4.5・4.6 今のまま | — | 守るだけ（テストで固定） |
| 4.7 実装済みなのに止めが無ければ赤 | `consumer_ledger.rs` の `canonical()`／`consumer_of` を読む | **Missing**（照合のテスト）・**Constraint**（台帳のファイルには触らない） |
| 5.1 記録の表示 | `script_log.rs` | 守るだけ |
| 5.2・5.3 台帳 | 行は特定済み | 書き換えだけ。`\![execute,…]` の 3 行が台帳に無い件は **Unknown** |

---

## 4. 実装の進め方の案

### 案 A: 今ある型に欄を足す（素直に広げる）

- `ShioriCall` の `Get`／`Notify` に出どころの欄を足し、全 50 の組み立てで出どころを決める。`ShioriBackend::get`／`notify` の引数に出どころのヘッダを足す。`ShioriOutcome::Value` を「台本＋応答のレベル」に変える。`StartTalk`・`ActiveTalk` に欄を足す。
- ✅ 型を見れば全部分かる。値がどこから来たかを読み違えない。
- ❌ 波及が最大: `ShioriBackend` の実装 21・`ShioriCall` の組み立て 50＋テスト・`ShioriOutcome::Value` 133・`StartTalk` の字面 65・`ActiveTalk` の字面 28。機械的な追随だけで数タスクになり、20 を超えやすい。後続の `property-query-channels`・`mcp-shiori-query` がまた同じ引数を変えることになる。

### 案 B: 関所と出口で導く（新しい部品を足し、既存の型はほぼ触らない）

- **出どころの型**を `areka-talk` に新しく置く（9 語の `SenderType`・Owned か・`local`／`external`・`Sender` の名前）。出どころから 4 つのヘッダの文字列を作る純関数も同じ所に置く。
- **要求側**: 送る直前の関所 `round_trip_raw` が `EventId` から出どころを導く（`Static`／`Choice` → ゴースト自身、`Sstp` → MCP の NOTIFY）。`OnTranslate` だけは翻訳の殻（`actor_translate::run_translate`）が `ScriptSource` から決めて渡す。`ShioriMsg::Request` に「追加のヘッダ」の入れ物を 1 つ添え、`ShioriBackend` には**既定の実装を持つ新しいメソッド**（例: ヘッダの入れ物を受け、応答のレベルも返す `get`／`notify`）を足す。既定は今の `get`／`notify` を呼ぶので、テストの偽物 19 は触らない。本物 2（`ShioriConnection`・`InProcBackend`）だけが上書きする。host32 の `ShioriRequest` には文字列のまま書く欄を足す（`status` と同じ扱い）。
- **応答側**: `ParsedResponse` に `SecurityLevel` の腕を足し、新しいメソッドの戻りで返す。kanade では `ShioriMsg::Request` の返事の型（`ShioriOutcome` を包む封筒）にレベルを添えるか、殻が `State` の控えへ置く（下の議題 3）。
- **台本側**: SHIORI の応答で始まるトークの出どころとレベルは、運行表の**出口の規則**（`schedule/translate.rs` の `after`／`on_done`）で、一括の中の再生の開始にまとめて写す。外からの台本と選択肢の `script:` は共通の `start_talk` で付ける。
- **タグ側**: 配送の `begin_talk` に出どころを渡し、`NoUserBreakCueSink` が自分で `BootCueSink` を実装して、外部扱いのトークでは `Enter` を送らず `warn!` を 1 件残す。止めるタグの表（5 行）は新しいファイルに置き、表の件数と、実装済みの行が台帳（`consumer_ledger.rs` を読むだけ）と食い違わないことをテストで固定する。
- ✅ 波及が小さい。後続の 2 本が同じ入れ物と同じ新しいメソッドを使える（brief の裁定「`ShioriBackend` の引数を変えず既定の実装を持つメソッドを足す」に合う）。止めの判断は受け手の側（決まりどおり）。
- ❌ `EventId` → 出どころの写しが「型ではなく規則」になる（将来 SSTP の受信が `EventId::Sstp` を共用すると区別がつかなくなる＝その spec が分類を足す前提）。`StartTalk` には結局欄が要る（1.4 を満たして配送まで運ぶため）ので、その追随は案 B でも残る。

### 案 C: 混ぜる（要求側は案 B、運搬は型に欄）

- 要求とヘッダの道は案 B（関所で導く・新しいメソッド）。再生側は `StartTalk`・`ActiveTalk` に正面から欄を足し、`StartTalk::new` は「ゴースト自身・`local`」を既定にして製品の呼び手を据え置く（`epilogue` の先例）。応答のレベルは `Input::ShioriReply` に欄を足すか封筒で運ぶ。
- ✅ 再生側の値の流れは型で追える。要求側の波及は小さい。
- ❌ テストの字面の追随（`StartTalk` 62・`ActiveTalk` 28 前後）は避けられない＝機械的な 1〜2 タスク。

### 3 案の比べ

| | 要求の道 | 応答のレベル | 台本の運搬 | 追随の量 | 後続 2 本との相性 |
|---|---|---|---|---|---|
| A | 型の欄（50＋21） | `ShioriOutcome` を変える（133） | 型の欄 | 最大 | 同じ引数を何度も変える |
| B | 関所で導く＋新メソッド（本物 2 だけ） | 封筒か控え | 出口の規則＋`StartTalk` の欄 | 小〜中 | 良い |
| C | B と同じ | 封筒か `Input` の欄 | 型の欄＋既定の構築 | 中 | 良い |

---

## 5. 規模と危うさ

- **規模: L（16〜20 タスク）**。内訳の見立て:
  1. 出どころの型と、4 つのヘッダの文字列を作る純関数（9 語・つなぎ方）＋単体テスト — 1
  2. host32: `ShioriRequest` の欄と並びの変更・字面を固定したテストの書き換え（`shiori3.rs`・`shiori3_charset_tests.rs`・kanade の `translate_test.rs`） — 1〜2
  3. host32: `parse_response` の `SecurityLevel`（大小無視・読めない値の warn）と、応答のレベルを返す口（`client.rs`） — 1
  4. `ShioriBackend` の新しいメソッドと本物 2 つ（`ShioriConnection`・`InProcBackend`）の実装、2 つの経路で同じヘッダになる照合（2.8） — 2
  5. kanade: 関所で `EventId` から出どころを導き、入れ物に載せる（`round_trip_raw`）・応答のレベルを運行表へ運ぶ — 2
  6. kanade: 翻訳の道（`OnTranslate` のヘッダ 2.4〜2.6・翻訳の応答のレベルの重ね合わせ・帳簿への控え） — 1〜2
  7. kanade: トークの開始へ出どころとレベルを付ける（出口の規則・`start_talk`・`ActiveTalk`・選択肢の `script:` の引き継ぎ・選択肢の ID／アンカーはゴースト自身） — 2
  8. `StartTalk` の欄と字面の追随（areka-sakura・areka-ghost・areka-seriko・areka-talk のテスト） — 1
  9. 配送の `begin_talk` で出どころを渡し、`NoUserBreakCueSink` が外部扱いで止めて warn — 1
  10. 止めるタグの表（5 行）と、表の件数・台帳との照合のテスト（4.7・4.8） — 1
  11. 通しのテスト（SHIORI が `SecurityLevel: external` を返すと区間に入らない・`local` では今のまま） — 1
  12. 網羅台帳 2 つと `doc/COMPAT_ARCHITECTURE.md` §8 — 1
  - 合計 15〜18＋予備（ファイルの 1,000 行の上限で兄弟のファイルへ出す作業など）＝**16〜20**。
- **危うさ: 中**。新しい技術は無く、型の先例（`on_idle`・`unblock_handle`・`begin_talk`・`epilogue`）がそろっている。危ないのは ⑴ 並びの変更で字面を固定したテストが一斉に赤になること（意図した変更なので書き換える）、⑵ 運ぶ道が kanade → talk → sakura → ghost → areka と長く、どこかで既定の値（ゴースト自身・`local`）に落ちても黙って通ること（外部扱いの通しのテストを 1 本必ず置く）、⑶ 行数の上限（`shiori3.rs` 821・`actor.rs` 837・`schedule/steady.rs` 957＝足せない・`consumer_ledger.rs` は触れない）。
- **20 を超えたときの切り方**: brief のとおり ⒜ ヘッダと応答の読み取り（上の 1〜6。追加のヘッダの入れ物を含む＝`property-query-channels`・`mcp-shiori-query` が待つ側）→ ⒝ 台本の印とタグの止め（7〜12）。

---

## 6. 設計の段への申し送り

### 推す方向（決定ではない）

- 案 B か案 C。要求側は「関所で導く＋既定の実装を持つ新しいメソッド」で揃え、後続の 2 本が同じ入れ物を使えるようにする。再生側は `StartTalk` に欄を足すこと自体は避けられないので、`StartTalk::new` の既定で製品の呼び手を据え置き、テストの字面は 1 タスクでまとめて追随する。

### 設計で決めること（要件の討議へ渡す議題の候補）

1. **出どころの型の置き場と形**。`areka-talk` に置くか（全員から見える・依存 0）。9 語は列挙＋並び（`internal`／`external` を先頭）で持つか、組の型にするか。`Sender` の名前（`areka`／`MCP`）を型に持たせるか、写しの関数で決めるか。
2. **`SenderType`・`Sender` をどこで決めるか**。⒜ 関所が `EventId` から導く（`ShioriCall` に触らない・将来の SSTP の受信が分類を足す前提）か、⒝ `ShioriCall` に欄を足す（50 か所）か。
3. **応答の `SecurityLevel` を運行表へどう運ぶか**。⒜ `ShioriMsg` の返事を封筒（台本の結果＋応答のレベル）にして `Input::ShioriReply` に欄を足す（テストの出現 151）、⒝ `ShioriOutcome::Value` を「台本＋レベル」にする（出現 133）、⒞ 殻が `State` の控えへ置き、出口の規則が読む（追随は最小だが運行表の純粋さが少し崩れる）。
4. **`OnTranslate` の要求の `SecurityLevel` に、元のイベントの応答の `external` を映すか**。ukadoc は「トランスレート元のイベントの属性を引き継ぐ」とだけ書く。要件 2.5 は「元のイベントの `SenderType` と `SecurityLevel` を引き継ぐ」。元の**要求**の値（ゴースト自身なら常に `local`）か、元の応答で下がった後の**実効の値**か。答えで `OnTranslate` の組み立ての材料が変わる。
5. **台本の開始へ出どころとレベルを付ける場所**。出口の規則（`schedule/translate.rs` の `after`／`on_done`）で一括にまとめて付けるか、腕ごと（11 か所）に付けるか。出口なら `schedule/steady.rs`（残り 43 行）を触らずに済む。
6. **止める判断の場所**。⒜ 受け手（`NoUserBreakCueSink`）が `begin_talk` で出どころを受けて自分で止める（決まり「消費側が名前で選ぶ」に沿う・`BootCueSink::begin_talk` の形を変えるか兄弟のメソッドを足すか）、⒝ 再生の層（`areka-sakura` の `drive.rs`）が表を見て合図を配る前に落とす（1 か所で全部止まるが、sakura が `\!` の名前を知ることになる）。
7. **止めるタグの表の置き場と、照合のテストの形**。表は出どころの型の隣（`areka-talk`）か `areka` の受け手の隣か。4.7 の照合は `consumer_ledger.rs` を**読むだけ**の新しいテストファイルで書く（C10 の約束で台帳のファイルには触らない）。
8. **`\![leave,nouserbreakmode]` と区間の閉じ忘れの扱いの確認**。外部扱いのトークで `enter` を止めたとき、同じトークの `leave` は今どおり UI へ流す（4.5）。UI の旗は「区間外の leave」を既に無害に扱うか、設計で確かめる（`crates/areka/src/input_events/user_break.rs` の `fold_no_user_break`）。

9. **`mcp-kanade-tools` の記録用の控えとの照合**（brief 棚卸㉗）。`StartTalk` に出どころを載せるなら、`ActiveTalk::origin`・`StrictNote` の控えから記録の `label` を引く所を、本 spec の出どころから引く形へ置き換える 1 か所が増えるか。
10. **規模の数え直しと分割の判断**。ギャップ分析の見立ては 16〜20。設計で数え直して 20 を超えたら、brief の切り方（⒜ ヘッダと応答の読み取り → ⒝ 台本の印とタグの止め）で切る。

### 調べが要る点（Research Needed）

- **網羅台帳に `\![execute,filewatch]`・`schedule-add`・`schedule-delete` の行が無い**。ukadoc では `schedule-delete` に「2.8.91」の版の印があり、台帳の元になった ukadoc の写しがこれより古いとみられる。要件 5.3 は `nouserbreakmode` と `\m` の行だけを直すので本 spec の範囲で行を足す必要はないが、表（4.1）の注記の出典は ukadoc の URL で書く。台帳の写しの更新は別件として起票するかを判断する。
- **ukadoc の写しを持つ `crates/ukadoc-survey` にも `filewatch` が無い**。4.8（表の件数が変わったら赤）を ukadoc の写しから自動で数える形は今は取れない（そもそも「正典の増減に自動で気付く仕組みは買わない」の方針にも合わない）。表は手で持ち、件数を固定するテストにする。
- **`SecurityLevel` の応答を返す実物の SHIORI**。手元の検体（pasta）は応答に常に `SecurityLevel: local` を付ける（`doc/emo2-conformance-scope.md`）。`external` を返す場面は偽の backend（x64 の偽装の口）と、32bit／同じプロセスの試験用 DLL（`shiori-host32-testdll`・`shiori4-testdll`）で作れるかを設計で確かめる。実機の確かめを要るとするかも設計で決める。
- **MCP の台本の出どころの SSP との突き合わせ**。値は要件が SSP の実測（10-21）で決めている。areka 側で同じヘッダが出たことを確かめるには、要求のヘッダ全体を残す記録が今は無い（`shiori_request` の記録は ID・Reference・Status だけ）。テストで固定すれば足りるか、記録に足すかを設計で決める。

---

## 7. 次の段

- 要件の討議（`/kiro-requirements-discussion areka-P0-script-security-level`）で上の議題 1〜8 のうち答えで作業が変わるものを扱い、その後 `/kiro-design areka-P0-script-security-level` へ進む。

---

## 8. 設計の段の調べと決め事（2026-10-22）

### 8.1 調べ方

- 種類: **軽い調べ（既存のしくみへの追加）**。新しい外部のライブラリは無いので、外の調べ（Web）は要らなかった。
- 使った手引き: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、設計書の型（`.kiro/settings/templates/specs/design.md`）、steering の `logging.md`（記録の段の基準）。
- コードは作業ツリーで読み直した。数え直した数（2026-10-22）: `ShioriOutcome::Value(` 133（製品 23）・`Input::ShioriReply` 151・`StartTalk {` 85・`StartTalk::new(` 26・`ActiveTalk {` 61・`impl ShioriBackend for` 21・`ShioriMsg::Request {` 31・`TranslateDone(` 30・`ShownTalk {` 7・`ShioriRequest {` 約 20・`ParsedResponse {` 11・`begin_talk(` 6・`EventId::Sstp(` の網羅の `match` 3。

### 8.2 申し送り（§6）の議題への答え

| # | 議題 | 決め | 理由 |
|---|---|---|---|
| 1 | 出どころの型の置き場と形 | `areka-talk` の新しいファイル `origin.rs`。`SenderType` は「先頭の 2 語（`SenderHead`）＋残りの 7 語（`&'static [SenderWord]`）」、Owned は `SstpKind` の 3 値、`SecurityLevel` の 2 値。`Sender` の名前は型に入れず、kanade の `RequestOrigin` が持つ | 依存 0 で全員から見える。先頭の語が必ず在ることを型で保つ（2.7）。`raise_event` の要求とその応答の `OnTranslate` は出どころが同じで `Sender` だけが違う（`MCP`／`areka`）ので、`Sender` は出どころの外に置く |
| 2 | `SenderType`・`Sender` を決める場所 | ⒜ 送るイベントの出所カテゴリ `EventId` から導く。翻訳の要求だけは新しいカテゴリ `EventId::Translate(RequestOrigin)` が元の要求の出どころを抱える | `EventId` は既に「出所カテゴリを型で保持」する型（DD-1）。`ShioriCall`（製品 50 か所）も `ShioriMsg`（31 か所）も形を変えずに済み、`EventId::Static("OnTranslate")` を確かめるテストは 4 か所だけ |
| 3 | 応答の `SecurityLevel` を運行表へ運ぶ道 | ⒝ `ShioriOutcome::Value(String)` を `Value(String, SecurityLevel)` にする | 応答のレベルは台詞（Value）の属性そのもの。⒜（`Input::ShioriReply` の欄・151）より出現が少なく、⒞（殻が `State` に置く）は応答ごとの値を裏口で運ぶ形になる。返信の線（`ReplySender<ShioriOutcome>`）は応答の型が運ぶしかなく、型を変えればコンパイラが全部の場所を挙げる（黙って既定に落ちない） |
| 4 | `OnTranslate` の要求のレベル | 要件の討議で確定済み（元の**要求**の値）。設計では `RequestOrigin::for_translated` が元のイベントのカテゴリから作る | — |
| 5 | トークに付ける場所 | 運行表の出口（`step` の `route` の後・`translate::after` の前）で、台詞の応答の一括の `StartTalk` 全部へ付ける（新しいファイル `schedule/talk_origin.rs`） | 腕（`boot.rs`・`change.rs`・`close.rs`・`steady.rs`）を触らない。`steady.rs`（957 行）に行を足さない。翻訳の出口の規則と同じ前提に立つ |
| 6 | 止める判断の場所 | ⒜ 受け口。`BootCueSink::begin_talk` に出どころの引数を足し、`NoUserBreakCueSink` が自分で止める | 「`\!` は汎用キャリア 1 本・消費側が名前で選ぶ」に沿う。sakura が `\!` の名前を知らずに済む。`begin_talk` を上書きするのは 1 つ（`BalloonLifecycleSink`）だけで、引数を足す追随は小さい |
| 7 | 表の置き場と照合のテスト | 表は受け口の隣（`crates/areka/src/emo2_boot/script_restriction.rs`）。照合は新しいテストファイルが `ConsumerLedger::canonical().consumer_of` を読むだけ。`\m[…]` は `\!` ではないので、`areka-parsers` の読みで `Instruction::Raw` になること（未実装）と照らす | 表を使うのは `areka` の受け口だけ（今も将来の `\![execute,…]` の受け手も同じクレート）。C10 の約束で台帳のファイルに触らない。本 spec は受け口を新しく足さないので台帳の行も足さない（roadmap の「台帳に行を足す spec は 1 本ずつ」の列に載っているが、実際には行を足さない） |
| 8 | 外部扱いで enter を止めたときの leave | 今の UI の旗の畳み込み（`crates/areka/src/input_events/user_break.rs` の `fold_no_user_break`）が区間外の leave を「旗を変えず `debug!` 1 件」で無害に扱うことを確かめた | 設計の変更は要らない |
| 9 | `mcp-kanade-tools` の記録用の控えとの照合 | 置き換えない。記録の種別の表示は今の `ScriptSource` から導くまま（要件 5.1）。`ActiveTalk::origin` と `StrictNote` も触らない | 要件 5.1 が表示の文字を変えないと決めている。出どころを使う下流（`script-impact-tiers`）は `StartTalk.origin` を読めばよい |
| 10 | 規模 | 17（予備を見て 17〜19）。20 を超えないので切らない | 設計書「規模の見立て」の表 |

### 8.3 調べが要る点（§6）への答え

- **網羅台帳に `\![execute,…]` の 3 行が無い件**: 要件の討議で `ukadoc-catalog-refresh` の持ち物と決まった。判定の表の出典は ukadoc の URL で書く。
- **`external` を返す SHIORI**: 実物は要らない。x64 の偽の backend（新しいメソッドを上書きして `security_level: Some("external")` を返す）で運行表から受け口まで通せる。32bit の仲介と同じプロセスの経路は、線の書き読みの純粋な関数（`build_request`・`parse_response`・`map_get_outcome`・共通の組み立て `request_of`）の単体テストで足りる。試験用 DLL（`shiori-host32-testdll`・`shiori4-testdll`）は直さない。手元の検体（pasta）は応答に常に `SecurityLevel: local` を付けるので、実機の振る舞いは変わらない。
- **MCP の台本の出どころの SSP との突き合わせ**: 値は要件が SSP の実測（10-21）で決めている。テストで値を固定し、加えて送る直前の記録 `shiori_request`（`trace!`）に `sender`・`sender_type`・`security_level` を載せて、実機でも同じヘッダが出たことを記録で確かめられるようにする。

### 8.4 合成（一般化・作るか採るか・削り）

- **一般化**: 要求のヘッダ（2.x）と台本の実効のレベル（1.x・3.x）と止めの判定（4.x）は、どれも「出どころ」という 1 つの値の見え方の違いだった。`ScriptOrigin` 1 つから 3 つとも導く。要求の側は `Sender` の名前を足した `RequestOrigin`。
- **一般化（入れ物）**: 追加のヘッダの入れ物 `RequestHeaders` は、本 spec の 4 つの欄だけを持つ構造体にした。後続の `mcp-shiori-query`（`X-MCP-PassThru-*`）・`property-query-channels`（`SenderType: internal,property`）は欄か語の組を足すだけで、`ShioriBackend` の新しいメソッドの形は変えずに済む。
- **作るか採るか**: 外のもので代わりになるものは無い（伺かの正典のヘッダの語彙そのもの）。SHIORI の線のコードは既存の `shiori-host32-host` の codec に足す。
- **削り**:
  - `ShioriCall` に欄を足す案（製品 50 か所）と `ShioriMsg` に欄を足す案（31 か所）は採らない。`EventId` の出所カテゴリで足りる。
  - `Input::ShioriReply` に欄を足す案は採らない（応答の型 `ShioriOutcome` で運ぶ）。
  - `ActiveTalk` に欄を足す案（61 か所）は採らない。選択肢の `script:` の引き継ぎは「最後に再生を始めたトークの控え」`ShownTalk`（7 か所）に出どころを足して読む。
  - 再生の層（sakura）での止めは採らない（sakura が `\!` の名前を知ることになる）。
  - `check_script` の診断・`\![execute,…]` の台帳の行・SSTP の `SecurityOrigin` の値は範囲外（要件で確定）。

### 8.5 危うさと手当て

- ヘッダの並びを変えると、要求の文の字面を固定したテスト（`shiori3.rs` の中・`shiori3_charset_tests.rs`・kanade の `tests/kanade/translate_test.rs`）が一斉に赤になる。意図した変更なので書き換える。
- 運ぶ道（kanade → talk → 配送 → 受け口）のどこかで既定の `GHOST` に落ちても黙って通る。通しのテスト（`script_security_e2e_tests.rs`）を 1 本必ず置く。
- 機械的な追随が広い（`ShioriOutcome::Value` 133・`StartTalk` 85・`TranslateDone` 30）。どれも型の変更でコンパイラが全部を挙げるので、見落としは起きない。`schedule/steady.rs`（957 行）は型の追随だけで行を増やさない。
- 1,000 行の上限: 新しい処理は新しいファイル（`origin.rs`・`request_headers.rs`・`request_origin.rs`・`talk_origin.rs`・`script_restriction.rs`）へ出す。`shiori3.rs`（821）・`client.rs`（775）・`actor.rs`（837）・`spine.rs`（940）は足す量が小さいか触らない（通しのテストは `spine.rs` を使わず、`runtime_boot_tests.rs` と同じ組み方で書く）。

## 9. 設計の討議の結果（2026-10-22）

- 自明な直し 3 件: 判定の表の照合を旗だけでなく受け口の振る舞いで確かめる（要件 4.7）・中身へ渡す包み 4 つ（`Recorder<B>`・テストの `BoxedBackend` 3 つ）も新しい 2 メソッドを渡す（`ShioriBackend` の実装は 22）・roadmap の「`consumer_ledger.rs` に行を足す鎖」から本 spec を外す。
- 議題 1（開発者「承認」）: 実機の確かめを 1 本足す（`crates/areka/tests/script_security_real_run.rs`・`real-run-kit`）。理由: 偽の SHIORI の檻は本物の pasta.dll の結線で出どころが既定の値に落ちる欠陥を隠しうる。この spec の肝は SSP の実測と同じヘッダが出ることなので、本物の起動の道で 1 度は見る。決定論のテストは置いたまま。見立ては 18 タスク（予備で 18〜20）。
