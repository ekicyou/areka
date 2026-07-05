# ギャップ分析（validate-gap）: areka-P0-kanade

- **分析日**: 2026-07-05
- **入力**: `requirements.md`（確定済・不変）／`brief.md`／`.kiro/steering/`（product・tech・structure・roadmap・logging）
- **調査手段**: 既存コードベースの実シンボル調査（`crates/areka-actor`・`crates/shiori-host32-host`・`crates/areka-parsers`・E2E テスト群・workspace 構成）＋隣接 spec 状態確認

---

## 1. 分析サマリ

- **上流依存は全て実在・実 API 確認済み**: `areka-actor`（spawn/reply/停止規約）と `shiori-host32-host`（`Shiori3Client`・`RequestError` 区別語彙）は brief 記載どおりのシンボルで存在し、kanade の要件（特に Req 4.5/4.6/5.3/6.1/6.2）の相当部分が**既存規約・既存語彙の消費だけで構造的に満たせる**。
- **kanade 本体は完全新規**: 運行状態機械・talk 契約型（`StartTalk`/`TalkDone`）・mock shiori／mock sakura sink・観測ハーネスはコードベースに一切存在しない（grep 確認済み・言及は areka-actor rustdoc の例示のみ）。新設クレート `crates/areka-kanade` が自然（workspace は `crates/*` glob で自動収載・命名慣行 `areka-actor`/`areka-emo-atlas` に整合）。
- **推奨方向**: 単一新設クレート＋内部三層（純粋運行状態機械／shiori アクター包装／結線・ハーネス）＝ brief「Boundary Candidates」どおり。純粋状態機械 `step(State, Input) → (State, Vec<Action>)` が決定的テスト（Req 7）の最短経路。
- **主要な未確定（design 送り）**: ukadoc Reference 表の全確認・shiori 往復の待ち方（handler 内同期 vs 応答メッセージ回送）・Tick/時刻注入の表現・talk 重複調停・契約型の配置（sakura-engine への推移的依存）。
- **並走 spec との契約は先決済みで衝突なし**: host32-lifecycle（死活報告型の正本・brief のみ＝kanade は mock 死活語彙で開発）／sakura-engine（talk 契約の消費者・brief のみ＝kanade が正本を新規定義して衝突なし）。

---

## 2. 既存資産の調査（Current State Investigation)

### 2.1 `crates/areka-actor` ✅（アクター規約の正本・2026-07-04 完了）

実シンボル（`src/lib.rs`・`spawn.rs`・`reply.rs`・`ui.rs` で確認）:

| シンボル | kanade 要件との対応 |
|---|---|
| `spawn_actor(name, body) -> (Sender<M>, ActorHandle)` | kanade アクター・mock shiori アクターの起動原語。スレッド名＝アクター名・`info_span!("actor")` 付与 |
| `run_inbox(rx, handler)`＝`Ok(Continue)/Ok(Break)/Err` | **Req 6.2 を基盤が保証**（handler `Err` → `tracing::error!` 記録して継続・ループは殺さない） |
| 停止規約: Close 即時停止＋**全 Sender drop → 正常終了** | **Req 4.6 が規約そのもの**（全指示送信元切断→宙吊りなし）。Req 4.5 の停止観測は `ActorHandle::join`/`is_finished` で成立 |
| `reply_channel() -> (ReplySender<T>, ReplyReceiver<T>)`・`recv_timeout` が `Timeout`/`Dropped` を区別 | Req 5.1 のメッセージ往復（request/reply）境界の器。shiori アクターへの GET/NOTIFY 往復に直用 |
| `spawn_ui`/`UiSender` | 本 spec では不要（kanade は UI スレッド非依存）。emo-present 領分 |

注意点（設計制約として転記すべきもの）:
- `run_inbox` は blocking `recv` のみ。周期 tick 等で `recv_timeout` 自前ループを書く場合も同規約に従うことが rustdoc（spawn.rs）に明記済み——ただし kanade は **Tick 注入式**（Req 3.2）ゆえ、テスト経路に実時間 `recv_timeout` は不要。
- `spawn_actor` の join はデッドロック注意（Sender を握ったまま join しない）→ close 握手完了→Sender drop→join の順序を結線側（ハーネス／将来 ghost-setup）が守る必要。
- テスト慣行: `run_bounded`（期限付き実行でハング検出）ヘルパが areka-actor tests に前例あり。kanade ハーネスでも同型が使える。

### 2.2 `crates/shiori-host32-host` ✅（SHIORI 出口 API・2026-07-05 完了）

実シンボル（`client.rs`・`error.rs`・`process_host.rs`・`parent_window.rs` で確認）:

- **`Shiori3Client::get(id, &[String]) -> Result<Option<String>, RequestError>`**: 200→`Some(Value)`／204→`None`／400・500・ErrorLevel→`Err(Shiori)`。**Req 5.3 の解釈契約はこの戻り値型が既に体現**しており、kanade 側で status 判定を再実装する余地がない（＝再定義禁止の徹底が容易）。
- **`Shiori3Client::notify(id, refs) -> Result<(), RequestError>`**: 同期往復・応答破棄。**Req 5.2「NOTIFY の応答から talk を生成しない」は戻り値 `()` により構造的に保証**される。
- **`RequestError { Handshake, Timeout, Ipc(IpcError), Shiori(ShioriError) }`**: **Req 6.1 の区別語彙（タイムアウト／SHIORI エラー／helper 死活／接続確立失敗）とほぼ 1:1 対応**（Timeout↔タイムアウト・Shiori↔SHIORI エラー・Ipc↔helper 死活の一態様・Handshake↔接続確立失敗）。kanade の失敗写像はこの enum の match 網羅で書ける。
- **スレッド前提**: `Shiori3Client` は `ParentMessageWindow`（`!Send`）を借用し**専有スレッド駆動が前提**。ただし**引数・戻り値はすべて `Send` な所有データ**——rustdoc に「下流 kanade が channel で結果を別スレッドへ渡せる」と明記済み（本 spec を名指しで想定した設計）。brief「Shiori3Client を専有スレッドで包む shiori アクター」はこの前提の実行に他ならない。
- **往復の駆動**: `send_request` は `SendMessageTimeout(SMTO_ABORTIFHUNG)`＋再入 RESPONSE の同期 1 往復。helper からの unsolicited push は M1 に存在しない（死活は `poll_exit` の poll 型）→ **shiori アクタースレッドは要求間に inbox recv でブロックしてよい**（メッセージポンプの常時併走は不要・HELLO pump は接続確立時 `pump_until_hello_or` のみ）。
- **死活 seam**: `spawn() -> HelperHandle`・`poll_exit_kind() -> Option<ExitKind{Clean/Abnormal(i32)/Terminated}>` は存在するが**監視の常設化・統一報告型は未実装**（host32-lifecycle 領分・brief で正本宣言済み）。
- **タイムアウト env**: `AREKA_SHIORI_REQUEST_TIMEOUT_MS`（既定 60s・`0`=無限 debug opt-in）が client 内部で解決される。kanade 側は per-call timeout を持ち込む必要なし。

### 2.3 隣接 spec の状態（並走整合）

| spec | 状態 | kanade への含意 |
|---|---|---|
| `areka-P0-host32-lifecycle` | **brief のみ**（要件未生成・並走可） | 死活報告型は**まだ存在しない**。Req 5.4 の「語彙は lifecycle 正本」は、kanade 側では**mock 死活語彙の暫定 seam**（メッセージ variant 等）として薄く受け、lifecycle 完了時に実型へ差し替える前提が brief 間で合意済み |
| `areka-P0-sakura-engine` | **brief のみ** | `StartTalk`/`TalkDone` は codebase 未存在（grep 済）→ kanade が正本を新規定義しても衝突なし。sakura brief 側も「消費・再定義しない」と明記 |
| `areka-P0-ghost-setup` | 未着手（roadmap のみ） | 結線・boot 指示の本番呼び手は不在。M1 では観測ハーネスが呼び手を代行（Req 7.1）。kanade の公開面（inbox 型・停止観測）が将来 ghost-setup の消費面になることを design で意識 |
| `areka-P0-app-shell` | brief 済 | boot/close 正典順序の転記元（roadmap kanade 行）と同期済み。運行表正本は kanade |

### 2.4 テスト・検証の既存慣行

- **env-gate 2 型が確立**（`shiori-host32-host/tests`）: ①必須資材（helper exe/testdll）＝不在は**明示 panic**（silent skip で緑偽装しない）②任意追験（`HOST32_PASTA_DLL`）＝未設定は **silent skip**。**Req 7.4（実 helper 追験・既定 skip）は②型の直接踏襲**で実装できる。
- **親 message-only 窓の同時生存は高々 1 つ**（同一プロセスで 2 組目が失敗する既知制約・E2E rustdoc 明記）→ kanade の実 helper 追験も**単一 `#[test]` に集約**する必要（既存 E2E と同じ運用）。
- **LOAD は `Shiori3Client` の外**（host32-shiori-load 領分・E2E では明示発行）→ 実 helper 追験ハーネスは spawn→HELLO pump→LOAD→kanade 運行、の順を自前で結線する（mock 経路には不要）。
- ロギングは `tracing` 全体規約（steering logging.md・レベル基準表あり）。「ログ無し失敗経路の禁止」は areka-actor / host32 系で実装済みの前例が豊富。

---

## 3. 要件→資産マップ（ギャップタグ: Missing／Unknown／Constraint）

| Req | 既存資産 | ギャップ |
|---|---|---|
| 1. boot 運行表 | 正典順序は brief/roadmap kanade 行に転記済み（OnInitialize NOTIFY→起動種別 GET 204 フォールスルー→OnBoot GET→basewareversion NOTIFY）。発行手段は `Shiori3Client`✅（mock 時は同型メッセージ） | **Missing**: 運行状態機械そのもの。**Unknown**: 各イベントの Reference 実装値（design で ukadoc 全確認・Reference 表化が brief の具体指示）。**Unknown**: M1 で毎回発行する起動種別の固定選択（1.6 の固定値運行の具体） |
| 2. Value 配送・talk 契約 | 該当資産なし（`sakura::parse` は下流 sakura-engine の入力であり kanade 非依存＝script 不透明のまま渡す境界は自然に成立） | **Missing**: `StartTalk`/`TalkDone` 型・talk_id 採番・突合状態。**Unknown**: 契約型の配置先（後述 DD-1）。2.5（未知 talk_id→ログ＋継続）は `run_inbox` の Err 継続規約✅で書ける |
| 3. OnSecondChange pump | Tick 注入の前例なし（dola は sakura 側の時間軸・kanade 非依存で可） | **Missing**: pump 状態ゲート（boot 完了後のみ・close 開始後停止）。**Unknown**: Tick の供給方式（design 送り済み・本番側）と時刻注入の表現（DD-3） |
| 4. close 握手 | 停止観測=`ActorHandle::join`✅・全 Sender drop 正常終了✅（4.5/4.6 は規約消費） | **Missing**: OnClose→再生完了待ち→OnCloseAll 分岐の状態機械。**Unknown**: 再生完了待ち上限値（de-facto・design 確定）と注入時刻による期限判定の実装形 |
| 5. SHIORI 呼出境界 | `Shiori3Client::get/notify`✅・戻り値契約✅（5.3 は match で消費）・NOTIFY 応答破棄✅（5.2 構造保証）・`reply_channel`✅（往復の器） | **Missing**: shiori アクター（`ShioriMsg` enum＋専有スレッド包装）と mock shiori アクター。**Constraint**: 死活報告型は lifecycle 未完＝mock 語彙で暫定 seam（5.4） |
| 6. 失敗経路の可観測性 | `RequestError` 区別語彙✅（6.1 と 1:1）・`run_inbox` Err 継続✅（6.2）・logging.md 規約✅・panic 規律の前例✅（areka-actor/host32） | **Missing**: kanade 状態機械側の「エラー→観測可能な状態遷移」写像（全アーム error! の網羅は実装規律） |
| 7. 決定的観測ハーネス | env-gate 2 型✅・`run_bounded` 前例✅・親窓 1 枚制約の運用✅ | **Missing**: mock shiori fixture（OnBoot→固定 Value・OnSecondChange→204 基調＋散発 Value）・mock sakura sink・単一 pass/fail 統合テスト |

---

## 4. 実装アプローチの選択肢

### Option A: 新設単一クレート `crates/areka-kanade`（内部三層モジュール）

brief「Boundary Candidates」の三層をモジュール境界で持つ:
1. **`schedule`（仮）＝純粋運行状態機械**: `step(State, Input) -> (State, Vec<Action>)` 型の純粋関数群。I/O・スレッド・channel 非依存。boot 順序・204 フォールスルー・pump ゲート・close 握手・talk 突合・期限判定（注入時刻）を全てここで決定的に単体テスト。
2. **`shiori_actor`＝`Shiori3Client` の channel 包装**: `ShioriMsg { Get{id, refs, reply}, Notify{id, refs, reply}, Close }` を受ける専有スレッドアクター。実装は host32-request の資産を「包むだけ」。mock shiori は**同じ `ShioriMsg` を受ける別 body**（fixture 応答）＝メッセージ型レベルの差し替え（Req 5.1 の趣旨どおり・trait 不要）。
3. **`kanade_actor`＋契約型＋ハーネス**: `KanadeMsg { Boot, Tick{..}, TalkDone{..}, Close, .. }` の inbox 駆動シェル（状態機械を呼び、Action を shiori channel／sakura sink へ流す）。`StartTalk`/`TalkDone` 型もここが正本。tests/ に mock 結線の単一 pass/fail 統合テスト＋env-gate 実 helper 追験。

- ✅ 命名・配置が既存慣行（areka-actor 等）に整合・workspace 自動収載
- ✅ 純粋状態機械により Req 7.3（反復同一結果）が最短で成立
- ✅ mock 差し替えが「同一メッセージ型・別 body」で済み、trait／framework 化を規約どおり回避
- ❌ `shiori-host32-host` への通常依存が入るため、将来 sakura-engine が契約型目的で areka-kanade に依存すると host32 系が推移的に付いてくる（DD-1）

### Option B: 契約型を分離（`areka-kanade` 本体＋契約専用の置き場）

`StartTalk`/`TalkDone`（＋必要なら `KanadeMsg` の公開部分）を極小の契約クレート（例 `areka-kanade-api`）または feature 分割で切り出し、sakura-engine は契約だけに依存する。

- ✅ 依存方向が最も清潔（sakura-engine が host32 系を推移的に引かない）
- ✅ 「kanade が正本・sakura が消費」の宣言がクレート境界として物理化
- ❌ M1 時点でクレートが 1 つ増える（2 型のための箱＝過剰化リスク・areka-actor の「抽象は 2 例目まで作らない」原則と緊張）
- ❌ 契約の変更が 2 クレート跨ぎになる

### Option C: ハイブリッド（M1 は Option A・分離は sakura-engine 着手時に判断）

M1 では単一クレートで進め、契約型はモジュール（例 `areka_kanade::talk`）として公開面を明確化しておく。sakura-engine の design 時に、推移的依存が実害（ビルド時間・アーキ制約）を生むと判明した場合のみ切り出す（切り出しはモジュール→クレートの機械的移動で済むよう、talk モジュールを host32 型に非依存で書いておく）。

- ✅ 2 例目（sakura-engine 実着手）駆動の判断＝プロジェクト原則に整合
- ✅ talk モジュールを最初から host32 非依存に保てば、後日の切り出しコストは小さい
- ❌ sakura-engine 着手時に判断を一度持ち越す（先送りが明示されていれば許容範囲）

**所見**: A と C は実質同一の初手であり、**「talk 契約モジュールを host32 型非依存で書く」規律を design に明記した上で A/C 系**が既存慣行と最も整合的。B は sakura-engine 側の設計材料が出てから正当化可能。（最終決定は design 領分・ここでは選ばない）

### 内部設計の主要分岐（design で確定すべき代替案）

**(a) shiori 往復の待ち方**
- **(a-1) handler 内同期待ち**: kanade の inbox handler 内で `reply.recv_timeout(..)` を呼び切る。順序が自然（boot 系列の逐次性が制御フローそのまま）・相関管理不要。ブロック中は inbox が進まないが、上限は `Shiori3Client` 側の実効 timeout（既定 60s）＋`recv_timeout` で有界。mock 経路では即応答ゆえテストは決定的。
- **(a-2) 応答を inbox メッセージへ回送**: shiori アクターが応答を `KanadeMsg::ShioriReply{corr_id, result}` として kanade inbox へ送り返す。kanade は完全ノンブロッキング（Close 即応・Tick 取りこぼしなし）だが、相関 id・「応答待ち中」状態の明示管理が状態機械に増える。
- トレードオフの核心: **Req 3.4（close 開始後は Tick で OnSecondChange を発行しない）や Close 即時性**と、状態機械の複雑度のバランス。(a-1) でも「ブロックが有界・mock では即時」なら要件は満たせる。純粋状態機械の形は (a-2) の方が素直（全入力がメッセージ）である点も考慮。

**(b) 時刻注入の表現**
- **(b-1) Tick メッセージに時刻同梱**（`Tick { now: SomeInstant }`）: 状態機械は受領時刻だけで期限判定（close 上限・Req 4.4）でき、Clock 抽象が不要。最小。
- **(b-2) Clock seam（関数ポインタ／クロージャ注入）**: メッセージ外でも現在時刻を取れるが、抽象が 1 枚増える。areka-actor の「トレイト過剰抽象回避」方針とは (b-1) が整合的。

---

## 5. 工数・リスク評価

- **Effort: M（3〜7 日）** — 新設 1 クレート・純粋状態機械＋アクターシェル＋mock 2 種＋統合ハーネス。上流 API は全て完成・確認済みで統合面の不確実性が低い一方、boot/close/talk の状態遷移網羅と Reference 表の design 確定作業がある。
- **Risk: Medium** — 技術リスクは低（tokio 不要・窓/COM/GPU 非関与・x64 純ロジック・全依存✅）。中リスクは**正典意味論の解釈**（Reference 実装値・OnCloseAll の発火条件・talk 重複時の de-facto 挙動）で、ukadoc MCP で design 冒頭に潰す計画が brief に織り込み済み。

---

## 6. Research Needed（design フェーズへ持ち越す調査項目）

1. **ukadoc Reference 表の全確認**（brief 必読リストの具体指示）: `OnInitialize`（Ref0="reload" 判定）・`OnFirstBoot`（Ref0=vanish count）・`OnBoot`（Ref0=shell 名・Ref6/7 任意）・`OnGhostChanged`/`OnGhostCalled`/`OnVanished`・`OnClose`/`OnCloseAll`（Ref0=理由）・`OnSecondChange`（Ref0〜Ref4・SSP 拡張の扱い）・`basewareversion`（Ref0/1/2）→ M-boot 送出最小集合の Reference 表を design.md に載せ、mock fixture をそこから生成。
2. **OnCloseAll の発火条件**: 単一ゴースト構成の終了で OnCloseAll を出すのが正典/デファクトか（Req 4.3 の分岐意味論の裏取り）。
3. **talk 重複時の de-facto 調停**: 再生中の OnSecondChange 発火可否（Ref3 talk 可否フラグの意味）・新規 Value の破棄/キュー/中断の SSP 慣行。
4. **close 再生完了待ち上限の de-facto 値**（SSP 実装の待ち時間相場）。
5. **Tick の本番供給方式**: ghost-setup 所有のティッカースレッド／kanade 内 `recv_timeout` 自前ループ／その他——本 spec の観測は注入式で閉じるが、本番結線の形は design で方向付けが要る。

---

## 7. 設計判断事項（requirements discussion へ供する論点・番号付き）

- **DD-1: talk 契約型の配置と依存方向** — `StartTalk`/`TalkDone` を `areka-kanade` 内モジュールとするか契約クレート分離か（§4 Option A/B/C）。分離しない場合も「talk モジュールは host32 型非依存で書く」規律を設けるか。
- **DD-2: shiori 往復の待ち方** — handler 内同期待ち (a-1) か応答メッセージ回送 (a-2) か（§4 内部分岐 (a)）。Close 即時性・Req 3.4 のゲート・状態機械の純度への影響を含めて確定。
- **DD-3: 時刻注入の表現** — `Tick{now}` 同梱 (b-1) か Clock seam (b-2) か。close 上限判定（Req 4.4）との統合方法。
- **DD-4: 死活報告の暫定 seam の形** — lifecycle 正本確定前の mock 死活語彙をどこに置くか（`KanadeMsg` の variant／shiori アクターからの通知メッセージ／`RequestError::Ipc` 経由の縮退判断のみで M1 を済ますか）。差し替え時の変更面を最小にする形。
- **DD-5: M1 boot 系列の固定値** — ✅**解決済み（要件ディスカッション #1・2026-07-05）**: 毎回 `OnFirstBoot`（Ref0=固定 0）を GET 発行し、204 フォールスルー経由で `OnBoot` へ進む（Req 1.6 に反映済み）。`OnGhostChanged` 系は M1 で常に非該当。position-persist 完了後は固定値が永続値読み出しに差し替わるのみで運行の形は不変。
- **DD-6: talk 重複時の調停規則**（requirements の design 送り事項の確定）: 破棄／キュー／中断のいずれか＋Research 3 の裏取り結果を反映。
- **DD-7: `TalkDone{quit:true}` の運行上の扱い** — ✅**解決済み（要件ディスカッション #2・2026-07-05）**: `\-`（quit=true）が**唯一のスクリプト起因終了トリガ**——由来イベント・close 握手中か否かを問わず、受領時点で終了系列（unload を含む正規終了経路→アクター停止）へ直行。OnClose は「終了要求」にすぎず、応答スクリプトに `\-` が無ければ（quit=false）ゴーストは終了せず定常運転へ復帰する（終了拒否）。Req 4 を全面改稿・Req 3.4 に復帰規則を追記済み。ukadoc 裏取り済み（`\-`=本体終了・`\e`=スクリプト終了）。**追補**: 終了にはもう一つ**強制経路**がある——OS シャットダウン（とにかく落とす）・SSP デバッグ用強制終了（`\-` 同等効果）。強制終了指示は quit の有無・握手状態を問わず終了系列へ直行（Req 4.4 新設）。OS シャットダウンの検出・強制判定は器（app-shell/ghost-setup）の責務で kanade は指示の受け手。強制終了時の OnClose 発行有無（best-effort）と Ref0 理由値（shutdown 等）は design で ukadoc 確認（design 送りに追加済み）。
- **DD-8: 実 helper 追験ハーネスの結線範囲** — env-gate テストが spawn→HELLO→**LOAD**→boot 運行を自前結線する（LOAD は kanade の責務外・ghost-setup 先取りの最小限）ことの確認と、親窓 1 枚制約下での単一 `#[test]` 集約。
- **DD-9: kanade 公開面の最小化** — 将来の呼び手 ghost-setup が消費する面（inbox Sender・ActorHandle・契約型）だけを公開し、状態機械・shiori アクター内部を非公開に保つ線引き。

---

## 8. design フェーズへの推奨

1. design 冒頭で **Research 1（Reference 表）を ukadoc MCP により確定**し、mock shiori fixture・状態機械の期待列・ハーネスの assert を全てその表から導出する（単一の正本から三点を生成）。
2. 実装アプローチは **Option A/C 系（単一クレート `crates/areka-kanade`・内部三層・talk モジュール host32 非依存）** を軸に、DD-1〜DD-9 を design 議題として明示的に確定する。
3. 統合テストは「mock 結線・単一 pass/fail・時刻注入」（Req 7.2/7.3）を主観測、env-gate 実 helper 追験（Req 7.4・`HOST32_PASTA_DLL` 型 silent skip）を従とする既存 2 型パターンの踏襲で設計する。

---

# design フェーズ調査・決定記録（2026-07-05・kiro-spec-design）

## 9. Research Log（ukadoc 正典確認＝Research 1〜4 の消化）

### 9.1 boot/close/OnSecondChange の Reference 表（Research 1 ✅確定）

ukadoc MCP で全イベントを確認し、design.md「ukadoc Reference 表」に M-boot 送出最小集合として転記した（mock fixture・状態機械期待列・ハーネス assert は `schedule/events.rs` を単一実装点として同表から導出）。出典 doc id:

| イベント | ukadoc doc id | 要点 |
|---|---|---|
| OnInitialize | `ukadoc:list_shiori_event:OnInitialize:1` | NOTIFY。Ref0=リロード時 `reload`・通常起動は無し |
| OnFirstBoot | `ukadoc:list_shiori_event:OnFirstBoot:1` | GET。Ref0=vanish 回数。**204 なら続けて OnBoot**（フォールスルーの正典根拠） |
| OnBoot | `ukadoc:list_shiori_event:OnBoot:1` | GET。Ref0=起動時シェル名・Ref6=`halt`/Ref7=落ちたゴースト名（MATERIA/SSP・M1 省略） |
| OnGhostChanged | `ukadoc:list_shiori_event:OnGhostChanged:1` | フォールスルー元（204→OnBoot）。M1 非該当（常に OnFirstBoot・Req 1.6） |
| basewareversion | `ukadoc:list_shiori_event:basewareversion:1` | NOTIFY。Ref0=version・Ref1=本体識別・Ref2=SSP のみ詳細（M1 省略） |
| OnSecondChange | `ukadoc:list_shiori_event:OnSecondChange:1` | Ref0=OS 連続起動時間(h)・Ref1=見切れ・Ref2=重なり・Ref3=talk 可否・Ref4=SSP のみ放置秒。**「トーク再生不能な時は Reference3=0 の上 NOTIFY で通知・返されたスクリプトは無視」** |
| OnClose | `ukadoc:list_shiori_event:OnClose:1` | GET。Ref0=`user`/`system`（SSP）・Ref1/2=スコープ番号（SSP・M1 省略）。「OnGhostChanging・OnCloseAll に 204 の場合続けて発生」 |
| OnCloseAll | `ukadoc:list_shiori_event:OnCloseAll:1` | GET。SSP 自体の終了・**唯一起動中のゴーストへの終了指示**・OS シャットダウンで発生。Ref 構成は OnClose と同一 |

### 9.2 OnCloseAll の発火条件と順序（Research 2 ✅・正典差分として記録）

- **Findings**: ukadoc 上、SSP の全終了（単一ゴースト終了含む）は **OnCloseAll→(204)→OnClose** の順（OnClose の説明文「OnGhostChanging、OnCloseAll に対してスクリプトが返されなかった（204）場合、続けてこのイベントが発生する」）。
- **Implications**: 確定済み要件（Req 4.1/4.6＝OnClose 先行・204 で OnCloseAll→終了）とは**順序が逆**。要件は要件ディスカッションで確定済み（OnClose 握手が終了拒否権を担う M1 構成）のため設計はこれに従い、差分を **DD-11** として明示・`events.rs`＋`schedule/close.rs` に順序を局所化し、**M-e2e（emo2-conformance-e2e）を Revalidation Trigger** とした。OnCloseAll の GET 応答 Value は M1 非再生（info ログ＋破棄）。

### 9.3 talk 重複時の de-facto 調停（Research 3 ✅＝DD-6 の根拠）

- **Findings**: OnSecondChange の正典意味論そのものが調停規則——「トーク再生不能な時は Reference3 が 0 になった上で **NOTIFY** でイベント通知される。返されたスクリプトは無視される」。
- **Implications**: talk 再生中の pump は NOTIFY(Ref3=0) 化することで、重複 Value を**発生源から断てる**（破棄/キュー/中断の選択問題が消滅）。防御として想定外の Value 到来は warn!＋破棄。

### 9.4 close 再生完了待ち上限（Research 4・de-facto）

- **Findings**: ukadoc に正典タイムアウト値は存在しない（de-facto 領域）。
- **Implications**: `KanadeConfig.close_talk_deadline_ms`（既定 30_000ms・結線側構成可能）とし、判定は注入 Tick 時刻のみで行う（Req 4.7 のテスト可能性を充足）。

### 9.5 強制終了時の OnClose 発行有無・Ref0 理由値（design 送り ✅＝DD-10）

- **Findings**: ukadoc OnClose/OnCloseAll の Ref0 は ユーザー終了=`user`・シャットダウン=`system`。OS シャットダウン時も SSP はイベントを発火する（OnCloseAll の発生条件に明記）。
- **Implications**: ForceQuit は **best-effort NOTIFY OnClose（Ref0=理由）を 1 発**→終了系列直行（GET 握手は Req 4.4 の「直行」を毀損するため行わない・送出失敗はログのみ）。

### 9.6 実装資産の追確認（unload seam）

- **Findings**: `MsgTag::Unload` はワイヤ定義済みだが、現行 helper は Unload を「記録のみ・無応答」（`shiori-host32-helper/src/main.rs` L314）・host 側にも unload API なし。正規 unload→exit(0) 経路は host32-lifecycle が増設中（記憶 canonical-not-minimal-lifecycle）。
- **Implications**: kanade は境界契約 `ShioriMsg::Unload`／`ShioriOutcome::Unloaded` のみ所有。real 側は M1 暫定（接続資材 Drop の既存 RAII teardown）・lifecycle 完了時に正規経路へ差し替え（Revalidation Trigger）。mock 経路が Req 4.3 の運行形を観測する。

## 10. Architecture Pattern Evaluation（設計レビューゲートでの反転を含む）

| Option | 内容 | 評価 |
|---|---|---|
| (a-2) 応答メッセージ回送 | shiori アクターが `Sender<KanadeMsg>` を保持し `ShioriReply` を inbox へ回送 | **却下（レビューゲートで欠陥発見）**: kanade は `Sender<ShioriMsg>` を、shiori は `Sender<KanadeMsg>` を常時保持するため **Sender 循環**が成立し、結線側が全 Sender を drop しても両者の inbox が切断されない＝**Req 4.9（全指示送信元切断→正常終了・宙吊りなし）が構造的に満たせない**。std mpsc に weak sender は無く、転送スレッド等の迂回も循環を移すだけで解消しない |
| (a-1) handler 内同期往復（**採用**） | `ShioriMsg` に `ReplySender<ShioriOutcome>`（oneshot）同梱・シェルが `recv` で受け切り `Input::ShioriReply` として状態機械へ即時再投入 | 循環なし（oneshot は往復ごとに消費）＝Req 4.9 構造保証。envelope 規約の正本流儀。純粋状態機械は「全入力メッセージ」の形を維持（Phase の待ち点はそのまま・相関 id は不要化）。トレード＝ForceQuit が in-flight 呼出完了（実効 timeout 有界・mock 即応）まで遅延——best-effort として許容 |

（クレート構成は §4 Option C を採用: 単一 `crates/areka-kanade`・`talk.rs`/`msg.rs`/`schedule/` は host32 非依存規律・切り出しは sakura-engine 着手時判断）

## 11. Design Decisions（DD-1〜DD-11 確定・design.md「設計判断」表が正本）

- **DD-1 ✅**: talk 契約は `areka-kanade::talk` モジュール（host32 型・areka-actor 型に非依存＝std のみ）。契約クレート分離は 2 例目（sakura-engine 実着手）駆動。
- **DD-2 ✅**: handler 内同期往復（a-1）。根拠と反転経緯は §10。
- **DD-3 ✅**: `Tick{now: MonotonicMs}` 同梱（b-1）・Clock 抽象なし。`now_ms` の本番意味論は「OS 起動からの経過 ms（GetTickCount64 相当）」＝OnSecondChange Ref0（hour）が正典一致。
- **DD-4 ✅**: `KanadeMsg::ShioriDown{reason: String}` の variant 1 個。lifecycle 正本確定時の差し替え面＝variant＋状態機械 1 アーム＋real.rs 報告箇所。
- **DD-5 ✅**（要件確定済み・再掲）: 毎回 OnFirstBoot(Ref0="0")→204→OnBoot。
- **DD-6 ✅**: talk 再生中の pump は NOTIFY(Ref3=0)（§9.3）。キュー・中断は導入しない。close は現行 talk の完了後に握手開始（同時 active talk ≤ 1 不変条件・`pending_close` 保留）。
- **DD-7 ✅**（要件確定済み・再掲）: quit=true 唯一トリガ・強制経路は迂回。
- **DD-8 ✅**: env-gate 追験は単一 `#[test]`（親窓 1 枚制約）・`HOST32_PASTA_DLL` silent skip・spawn→HELLO→LOAD（`send_request(MsgTag::Load, ..)` 既存慣行）を connect クロージャとしてテストが自前結線。
- **DD-9 ✅**: 公開面＝`spawn_kanade`・`KanadeMsg`・talk 契約型・`ShioriMsg` 系・`KanadeConfig`・`spawn_shiori_actor`。`schedule/` は `pub(crate)`。
- **DD-10 ✅**（新規・design 送り消化）: ForceQuit＝best-effort NOTIFY OnClose(Ref0=理由)→終了系列直行（§9.5）。
- **DD-11 ✅**（新規）: OnClose→OnCloseAll 順序の正典差分を意図的差分として記録・局所化・M-e2e Revalidation Trigger（§9.2）。

## 12. Synthesis 記録

- **一般化**: boot・pump・close・強制終了・Fault の 5 経路を単一の純粋状態機械 `step(State, Input) -> (State, Vec<Action>)` に統合（経路別の機構を作らない）。イベント構成は `events.rs` の関数群に一点化し、実装・fixture・assert の三点を同一正本から導出。
- **Build vs Adopt**: アクター機構＝areka-actor 採用（再発明ゼロ）・SHIORI 契約解釈＝`Shiori3Client` 戻り値型の消費のみ（status 判定を再実装しない）・新規外部依存ゼロ（tokio 禁止・std mpsc 起点の凍結規約に整合）。
- **単純化**: 相関 id（corr）を DD-2 の同期往復化で全廃・Clock trait 不採用・mock は「同一 enum を受ける別 body」で trait/framework 化を回避・Phase は待ち点 9 状態に限定。

## 13. Risks & Mitigations（design 時点）

- **close 順序の正典差分（DD-11）** — `events.rs`＋`close.rs` に局所化・M-e2e で SSP 実挙動と突合し必要なら順序入替（波及は 2 ファイル）。
- **実経路での Tick 滞留（同期往復ブロック中）** — 解除後 catch-up 処理（burst は滞留秒数分で有界・mock 経路は非発生）。SSP も同型の catch-up 挙動。
- **ForceQuit 遅延（in-flight 呼出中）** — `AREKA_SHIORI_REQUEST_TIMEOUT_MS`（既定 60s）で有界・OS シャットダウンの最終強制力は OS 側。
- **join デッドロック（Sender 保持のまま join）** — 結線規律を rustdoc 明記・ハーネスは drop→join 順＋`run_bounded` 期限付き。
- **lifecycle 並走との差し替え面** — ShioriDown seam／Unload 暫定実装ともに変更面を 1 箇所に限定（Revalidation Triggers に登録済み）。

## 14. References（design フェーズ追加分）

- ukadoc doc id 群（§9.1 の表）— Reference 表・DD-6/DD-10/DD-11 の正典根拠
- `crates/areka-actor/src/spawn.rs`／`reply.rs` — 停止規約・oneshot・`FnOnce(Receiver<M>)` 境界（DD-2 の実装前提）
- `crates/shiori-host32-host/src/client.rs`／`error.rs` — `Shiori3Client`・`RequestError` 写像元
- `crates/shiori-host32-helper/src/main.rs`（Unload 無応答の現状）・`crates/shiori-host32-ipc/src/lib.rs`（`MsgTag::Unload` ワイヤ定義）— §9.6 の根拠
