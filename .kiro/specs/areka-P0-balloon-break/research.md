# ギャップ分析: areka-P0-balloon-break

> 実測日 2026-09-20・本ブランチ（`claude/kiro-balloon-double-click-stop-454210`）。
> コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 本書は**判断材料と選択肢**であって決定ではない。番号付きの分かれ目は最終節にまとめた。

## 1. 要約（5 点）

- **要件の 4 片のうち 3 片は既存の型・経路にそのまま乗る。** 合図の検出（バルーン窓のポインタ押下ハンドラ）・再生の停止（`Action::CancelChoice` → `TalkCommand::CancelChoice` → dispatcher の `fn on_cancel_choice` → `SakuraMsg::Close` → `fn on_close`）・バルーンの非表示（`enum VisibilityAction` の `HideScopes`）は、いずれも既に動いている経路の脇に 1 本足すだけで届く。
- **残る 1 片＝「中断を禁じる旗」だけが新しい線を要求する。** 旗の源は台本の中（`\!` 運搬 cue・talk スレッド）、旗を読む場所は「今トークが再生中か」を知っている kanade（別スレッド）である。この 2 点をつなぐ線は**現在 1 本も無い**。既存の運搬 cue の受け口（`MoveCueSink`・`ZOrderCueSink`・`ReadmeCueSink`）はすべて **talk スレッド → UI スレッド**の向きに送っており、**talk スレッド → kanade** の向きの受け口は 1 つも無い。
- **最大の落とし穴は「隠した直後にバルーンが独りでに出てくる」こと。** バルーンを出す規則は「可視グリフ数が増えた、かつ今は不可視」（`fn decide_content` in `crates/areka/src/emo2_boot/balloon_visibility.rs`）であり、可視グリフ数は**時刻の関数**（`RevealSchedule::visible` in `crates/areka-emo-text/src/state.rs` が「リビール時刻が t 以下の個数」を返す）である。再生を止めても、**既に届いている文字の塊はその塊の持ち時間ぶん文字が増え続ける**。したがって中断で隠した次のフレームに「増えた・今は不可視」が成立し、同じバルーンが再表示されうる。文の途中で止めれば再現し、待ち（`\w`）の最中に止めれば再現しない——**走行ごとに出たり出なかったりする**厄介な形で現れる。設計はこれを正面から塞ぐ必要がある（§4.3）。
- **`Action::CancelChoice` の再利用は下流から見て安全である。** dispatcher の `fn on_cancel_choice` は選択の状態を**一切見ない**——現行 slot の `talk_id` と突き合わせ、一致すれば `SakuraMsg::Close` を転送するだけで、不一致・空なら `cancel_choice_stale` で info 棄却する。選択待ちが無いときに使っても、余計な副作用は起きない。加えて要件 2.6（選択を捨てて `OnChoiceTimeout` を出さない）は、返ってくる `TalkDone{Interrupted}` を `fn on_talk_done`（`crates/areka-kanade/src/schedule/steady.rs`）が受けた時点の既存の掃除（`clear_choice_ledger(state, "steady_talk_done")`）で**追加実装なしに満たされる**。
- **1 ファイル 1,000 行の番人に 2 ファイルが引っかかる見込み。** `crates/areka/src/input_events/balloon.rs` は 917 行・`crates/areka-kanade/src/schedule/steady.rs` は 935 行で、このリポジトリの注釈密度では新しい判断分岐 1 本で容易に超える。兄弟ファイルへの分割は既に前例がある（`balloon_visibility.rs` が `#[path = "balloon_visibility_phase.rs"] mod phase;` で配線層を子モジュールへ出している）。

## 2. 現状の実測（要件が触る 4 片）

### 2.1 合図の検出（UI・入力）

| 事実 | 定義箇所 |
|---|---|
| バルーン窓に付くポインタハンドラは移動・押下の 2 本だけ | `fn on_balloon_pointer_moved`／`fn on_balloon_pointer_pressed`（`crates/areka/src/input_events/balloon.rs`） |
| 装着は `BalloonWindowMarker` を持つ窓だけに行う | `fn attach_balloon_pointer_handlers`（同上） |
| 押下ハンドラは `state.left_down` だけを見て、`double_click` 欄を読まない | `fn on_balloon_pointer_pressed`（同上・doc に「`double_click` フィールドは一切参照しない」） |
| ダブルクリックの欄は 6 値（無し・左・右・中・拡張 1・拡張 2） | `enum DoubleClick`（`crates/wintf/src/ecs/pointer/types/mod.rs`） |
| 欄は配信の末尾で毎回消される＝1 フレーム限り | `fn dispatch_pointer_events` の末尾（`crates/wintf/src/ecs/pointer/dispatch/mod.rs`）と `fn clear_transient_pointer_state`（同 `systems.rs`） |
| キャラ窓側はこの欄を読んで `OnMouseDoubleClick` を送る | `fn on_char_pointer_pressed`（`crates/areka/src/input_events/mod.rs`） |
| スコープ番号は窓から取れる（`usize`） | `struct BalloonWindowMarker`（`crates/areka/src/placement/spawn.rs`） |
| 選択肢の行に当たったかを決める純関数がある | `fn click_selection`／`fn hit_choice_row`（`crates/areka/src/input_events/balloon.rs`） |

**2 打目の扱い（要件 1.5 の前提）**: ダブルクリックの 2 打目は、`left_down` が立った独立の押下として配信され、同時に `double_click = Left` も載る。つまり**現状すでに 2 打目は選択肢の確定として消費されうる**。`fn click_selection` が `Some` を返したときは中断の合図を作らない、という形にすれば要件 1.5 はそのまま満たせる（同じハンドラの中で判定順を決めるだけで、新しい状態を持たなくてよい）。

**要件 1.9（出ていないバルーンでは合図を作らない）**: バルーン窓は非表示のとき破棄されず、α マスクによるクリック透過の登録対象（`fn register_ghost_windows_click_through`・`crates/areka/src/placement/spawn.rs`）である。透過が効いていれば押下ハンドラ自体が着火しないはずだが、**「非表示＝必ず素通し」が成り立つかは未確認**（→ 研究項目 R-1）。ハンドラ側で可視性を照会して自衛するかどうかは設計判断（判断項目 6）。

### 2.2 再生の停止（kanade → dispatcher → sakura）

```
UI → KanadeMsg → schedule::step → Action::CancelChoice{talk_id}
   → TalkCommand::CancelChoice        （crates/areka-kanade/src/actor.rs の fn send_talk_command）
   → DispatcherMsg::CancelChoice      （crates/areka-ghost/src/dispatcher.rs の impl From<TalkCommand>）
   → fn on_cancel_choice → SakuraMsg::Close（slot は保持する）
   → fn on_close（crates/areka-sakura/src/drive.rs）→ player.stop() → TalkDone{Interrupted}
   → fn on_done → KanadeMsg::TalkDone → schedule の fn on_talk_done → steady::on_talk_done → Steady{None}
```

- `fn on_cancel_choice` は **`close_active_if_any` を使わない**（slot を保持したまま `Close` を転送する）。これは意図的で、即 join して slot を空けると中断 ACK が `fn on_done` の一致判定で stale 扱いになり kanade が復帰できなくなるため、と doc に明記がある。**中断にはこの性質がそのまま要る。**
- 停止の意味論（`player.stop()` で未発火の cue を捨てる＝以降のタグ・文字・待ちを 1 つも実行しない）は `fn on_close` の doc と決定論テスト（`crates/areka-sakura/src/drive_lifecycle_tests.rs`）で既に固定済み。要件 7.7 の「再テストしない」に一致する。
- **選択待ちの掃除は自動で付いてくる**: `TalkDone{Interrupted}` は `fn on_talk_done`（`crates/areka-kanade/src/schedule/mod.rs`）が `talk_done_interrupted_as_non_quit` を記録して steady へ委譲し、`fn on_talk_done`（steady）が `clear_choice_ledger` を呼んでから `Steady{None}` へ戻す。`OnChoiceTimeout` は `fn fire_choice_timeout_if_due` が Tick で判定するもので、帳簿が消えていれば二度と発火しない。
- **終了挨拶の最中（要件 3.6）**: `Phase::CloseTalkWait` で `TalkDone{Interrupted}` を受けると `fn on_close_talk_wait`（`crates/areka-kanade/src/schedule/close.rs`）が「終了拒否・`Steady{None}` へ復帰」へ落とす。既存テスト `value_then_interrupted_refuses_close_same_as_ended` が固定済み。ただし**`CloseTalkWait` で中断の合図を受け付けるかどうかは未定**（現状 `Input::Mouse` は Steady 以外では trace して捨てる）。→ 判断項目 4。

### 2.3 バルーンの即時非表示（UI・表示）

| 事実 | 定義箇所 |
|---|---|
| 隠す理由の語彙は 4 つ（内容・全消去・時間切れ・明示） | `enum VisibilityTrigger`（`crates/areka/src/emo2_boot/balloon_visibility.rs`） |
| `Explicit` を**発行する者は居ない**——配線層が「前フレームの記憶と今フレームの観測が食い違った」ことを検出して**記録するだけのラベル**である | `fn log_external_transitions`（`crates/areka/src/emo2_boot/balloon_visibility_phase.rs`）と `enum VisibilityTrigger` の doc「判断中核はこの契機を作らない」 |
| 判断は純関数 1 本に集約されている（World も時計も触らない） | `fn decide`（`balloon_visibility.rs`）＝ `apply_lifecycle_signals` → `decide_content` → `decide_timeout` の 3 段 |
| 表示側がトークについて知る合図は 2 値だけ | `enum TalkLifecycleSignal`（`crates/areka/src/emo2_boot/talk_lifecycle.rs`）＝`TalkStarted`／`DisplayEndAt(f64)` |
| その合図の受信端は 1 本の mpsc で、毎フレーム冒頭に全件取り出される | `Emo2Wiring.lifecycle_rx`（`crates/areka/src/emo2_boot/frame/wiring.rs`）を `fn drain_lifecycle`（`balloon_visibility_phase.rs`）が取り出す |
| 送出端は `wire_emo2_boot`（`crates/areka/src/emo2_boot/mod.rs`）が作り、`BalloonLifecycleSink` へ渡している。**送出端は複製できる** | 同上 |
| 30 秒の既定は 1 か所定義 | `DEFAULT_BALLOON_TIMEOUT_SECS`（`balloon_visibility.rs`） |
| 相の走る位置は `Update`（表示指令の適用の直後・窓寸調整の直前）。入力は同じフレームの `Input` で先に走る | `fn run_balloon_visibility_phase` の doc（`balloon_visibility_phase.rs`） |

**再表示の仕組み（要件 4.7 が壊れてはならない部分）**: `fn decide_content` は「可視グリフ数が前フレームより増えた、かつ今は不可視」で `Show` を積む。中断で隠した後、次のトークが始まると（全消去 → 新しい文字）この条件が成立して普通に出る。**ただし §4.3 の落とし穴がこの同じ条件から生じる。**

**明示的な非表示の入口**: `PresentCommand::Hide` を `presenter.apply` へ渡す（`fn issue_actions`・`balloon_visibility_phase.rs`）。判断層を通さずここを直接叩くと、次フレームに `fn log_external_transitions` が**偽の `trigger=explicit`** を記録する（要件 4.4 が禁じる「既存の理由の意味を変えない」に抵触する）。→ 中断の非表示は**判断中核を通す**のが素直（§4.1 の選択肢 D1）。

### 2.4 中断を禁じる旗（`nouserbreakmode`）

| 事実 | 定義箇所 |
|---|---|
| `\![enter,nouserbreakmode]` は解析で汎用コマンドになる | `Instruction::GenericCommand{name, raw_args}`（`crates/areka-parsers/src/sakura/model.rs`） |
| コンパイルで運搬 cue になる（名前＝`enter`・引数＝`["nouserbreakmode"]`） | `crates/areka-sakura/src/compile.rs` の `Instruction::GenericCommand` アーム（`CueCommand::command_carrier`） |
| 運搬 cue は全受け口へ一斉配信され、各受け口が**自分の名前で選り分ける** | `crates/areka/src/emo2_boot/consumer_ledger.rs` のモジュール doc |
| 登記されている担当は 6 行（`move`・`bind`・`(set,zorder)`・`(reset,zorder)`・`(open,readme)`・フォントタグ）。`enter`／`leave` の行は **0 行** | `ConsumerLedger::canonical`（同上。件数は同ファイルのテスト `canonical_builds_without_duplicate` が 6 で固定） |
| 受け口の書き方の手本（名前＋第 1 引数で選り分け・担当外は debug で読み飛ばし・送出失敗は warn） | `ReadmeCueSink`（`crates/areka/src/emo2_boot/readme_cue.rs`）が最小で最も近い |
| 既存の受け口 6 本はすべて **talk スレッド → UI スレッド**の mpsc | `wire_emo2_boot`（`crates/areka/src/emo2_boot/mod.rs`）の sink 構築部 |
| `ExecutionState::NoUserBreak` は語彙としてだけ存在し、`fn derive` は一度も立てない | `crates/areka-kanade/src/status.rs`（`// 6. nouserbreak ← SEAM` の行） |
| 網羅台帳の 2 行はいずれも `status = "absent"`・`owner = ""` | `doc/ukadoc-coverage/ledger/sakura-script.toml` の `ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1` と `…_5bleave_…` |

**この 1 片が本仕様の本体である。** 旗を誰が持つかで設計全体の形が決まる（§4.2）。

## 3. 「誰が何を知っているか」の地図（判断項目 1 の材料）

| 知識 | 持ち主 | スレッド | 今ある出口 |
|---|---|---|---|
| バルーンが左ダブルクリックされた・どのスコープか | UI（ポインタハンドラ） | UI | `MouseWiring`→`KanadeMsg::Mouse`／`BalloonWiring`→`ChoiceSelectionInbox`→`KanadeMsg::Choice` |
| バルーンが今 画面に出ているか | UI（`EmoPresenter::target_visible`） | UI | なし（UI 内で完結） |
| いま台本を再生中か・その `talk_id` | kanade（`Phase::Steady{talk: Some}`） | kanade | `Action::*`（下流へのみ） |
| 選択待ちかどうか | kanade（`State.choice`） | kanade | 同上 |
| 台本に `\![enter,nouserbreakmode]` が現れた | 再生中の cue 配信（運搬 cue） | talk（per-talk スレッド） | 受け口 6 本・**行き先はすべて UI** |
| 実際に再生が止まった | sakura → dispatcher → kanade | talk→kanade | `TalkDone{Interrupted}` |

**要点**: 「合図（UI）」「再生中か（kanade）」「旗（talk）」の 3 つが**3 つの別々のスレッド**に散っている。どこで突き合わせるかが設計の最初の分岐である。

**遅れの構造**（要件 4.5 の「1 フレーム遅らせる解は採らない」に直結）:

- UI の 1 フレームの中の順序は `Input`（ポインタ配信・既存の取り出し 2 本はここに `dispatch_pointer_events` の**後**で登録されている＝`fn wire_choice_drain`・`crates/areka/src/input_events/choice_drain.rs`／`fn drain_readme_requests`・`crates/areka/src/readme.rs`）→ `Update`（`emo2_frame_system` の中でバルーン可視性の相）である。
- したがって **UI が自分で決めるなら、押下と同じフレームのうちに隠せる**（`Input` で合図を立て、`Update` の相で隠す）。
- **kanade へ往復すると同じフレームには戻らない**。kanade は独立スレッドで、`KanadeMsg` → `step` → `Action` → dispatcher → talk の直列を経る。返事が UI へ届くのは早くても次のフレーム、実際にはそれ以上かかりうる。

## 4. 実装方式の選択肢

分解して、片ごとに選択肢を並べる。組み合わせは最後にまとめる。

### 4.1 合図をどこで受け止めるか（停止と非表示の主体）

**方式 A: kanade が唯一の判断主体（brief の採用案）**

UI は「利用者が中断を求めた・スコープ N」だけを送る。kanade が再生中かどうか・旗が立っているかを見て、止めるなら `Action::CancelChoice`（または新値）を出し、あわせて「バルーンを隠せ」を UI へ返す。

- ✅ 判断が 1 か所。第 2 の停止経路が生まれない（アーキテクチャ決定「talk 中断は単一 Close funnel」に最忠実）。
- ✅ 選択待ちの破棄・定常復帰・終了挨拶の扱いが既存経路に丸ごと乗る。
- ❌ **kanade → UI の「隠せ」を運ぶ線が現在 1 本も無い**（`KanadeStopped` の 1 種だけが逆向きに流れている前例。`wire_emo2_boot` が `kanade_stop_tx`／`kanade_stop_rx` を作って `Emo2Wiring::set_kanade_stop` で据える形）。同型の 2 本目を敷く必要がある。
- ❌ **非表示が押下と同じフレームに間に合わない**（§3 の遅れの構造）。要件 4.5 の読み方次第で不合格になりうる。→ 判断項目 2。
- ❌ 旗を kanade まで運ぶ線が要る（§4.2）。

**方式 B: UI が非表示を決め、kanade が停止を決める（役割分割）**

押下ハンドラが、そのフレームのうちに「隠せ」を表示側へ立てつつ、同じ操作を kanade へも送る。kanade は独立に再生を止める。

- ✅ 非表示が押下と同じフレームで成立する（要件 4.5 に最も素直）。
- ✅ kanade 側は方式 A と同じ。
- ❌ **旗が立っているとき「隠さない」ためには、UI 側も旗を知っていなければならない**（要件 2.3）。旗が 2 か所に写り、食い違う余地が生まれる。
- ❌ 「再生中でない」ときも隠す（要件 2.2）ので UI 側は再生中かを知らなくてよい——この点は逆に都合がよい。**判断の分かれ目は旗ただ 1 つ**である。

**方式 C: UI が全部決める**

UI が旗も持ち、再生中かも知り（新しい合図 `TalkEnded` を足す）、隠して、止める指示を送る。

- ✅ 判断が 1 か所・同フレーム。
- ❌ kanade の帳簿（選択待ち・`pending_close`）と食い違う第 2 の運行判断が生まれる。brief が「却下 B」として既に退けた形に近い。
- ❌ 「再生中か」を UI へ写すのは新しい二重帳簿であり、`enum TalkLifecycleSignal` の設計思想（表示のためだけの観測）に反する。

**方式 D: 旗も判断も再生層（sakura）に置く**

talk アクターが自分の旗を持ち、中断要求を自分で受理／拒否する。kanade は結果（`TalkDone{Interrupted}` が来るか来ないか）でしか知らない。

- ✅ 旗の置き場所として最も自然（旗の源＝同じ talk の台本であり、talk の寿命と旗の寿命が一致する＝要件 5.4「トークの終わりで解く」が構造で満たされる）。
- ❌ `SakuraMsg::Close` は**拒否できてはならない**（dispatcher の `close_active_if_any` が終了・差し替えで使う）。拒否しうる別メッセージ（例 `SakuraMsg::UserBreak`）を足すと、`fn on_close` を共用しても「入口が 2 つ」になる。要件 3.2 の読み方次第。
- ❌ 拒否されたことを UI へ返す線が要る（バルーンを隠さないため）。方式 A と同じ遅れを持つ。
- ❌ 「再生中でない」ケース（要件 2.2）には talk アクターがそもそも存在しないので、非表示の発生源が 2 系統に割れる。

### 4.2 旗をどこへ運ぶか

前提: 運搬 cue を選り分ける新しい受け口 1 本（`ReadmeCueSink` と同型。担当は `("enter","nouserbreakmode")` と `("leave","nouserbreakmode")` の 2 組）を建てるところまでは、どの方式でも共通である。台帳 `ConsumerLedger::canonical` へ 2 行（登記件数 6 → 8）と、件数を固定しているテストの更新が要る。

**F1: 受け口 → UI（mpsc・既存 6 本と完全に同型）**

- ✅ 前例どおりで最も摩擦が少ない（`wire_emo2_boot` に channel 1 本・sink 1 本を足すだけ・上流の署名を一切変えない）。
- ✅ 取り出しを `Input` の段（`dispatch_pointer_events` の**前**）へ登録すれば、同じフレームの押下判定より先に旗が反映される。既存 2 本は「後」に登録しているので、**前**に置く新例になる。
- ❌ それでも「cue が talk スレッドで発火してから UI が取り出すまで」の実時間の窓は残る（最大 1 フレーム）。台本の意図としては `\![enter,…]` の直後の数十 ms に中断が受理されうるという穴。
- ❌ 方式 A（kanade 判断）だと、旗を UI からさらに kanade へ中継することになり二重の遅れになる。**F1 は方式 B と噛み合う。**

**F2: 受け口 → kanade（新しい向きの線）**

kanade の投函端（`Sender<KanadeMsg>`）は `areka_ghost::boot_with_kanade_stop` が返る**まで存在しない**のに、受け口は boot へ渡すため**その前に**組まねばならない。同じ鶏卵は既にこのリポジトリで 2 度解かれている:

- `reply_channel` による一度きりの受け渡し（`crates/areka-ghost/src/dispatcher.rs` の self-sender ハンドオフ）。
- 受信端だけ手元に置いて後から据える（説明書の受け口＝`readme_rx` を boot 成立後に配る形・`wire_emo2_boot`）。

したがって「受け口は共有の置き場（`Arc<OnceLock<Sender<KanadeMsg>>>` 等）を持ち、boot 成立後に 1 度だけ据える」は既存の作法の延長で書ける。

- ✅ **旗と「再生中か」が同じスレッドの同じ状態機械に載る＝判断に遅れが無い**（方式 A の弱点を消す）。
- ✅ 要件 5.7（`areka-P0-status-execution-states` が旗を読めるようにしておく）が自然に満たされる——旗が kanade の `State` にあれば、`ExecutionSnapshot` に欄を 1 本足すだけで `nouserbreak` トークンが立つ（`status.rs` の SEAM 注記がまさにその形を予告している）。
- ❌ 据える前に cue が届く窓（boot 直後の `OnBoot` 応答トークが即座に始まる）。据わっていなければ記録して捨てる縮退になる。
- ❌ `KanadeMsg` に 1 値、`schedule::Input` に 1 値、横断ルーティングに 1 アームが増える。

**F3: 旗を talk アクター（`TalkDriver`）に持たせる**

受け口を経由せず、`crates/areka-sakura/src/drive.rs` が cue を自分で見る——は成立しない（cue の配信先は `CueSink` であって driver ではない）。実際には「受け口が talk スレッド上で自分の talk の driver へ書き戻す」形が要り、`CueSink` は driver への参照を持たない。**共有状態（`Arc<AtomicBool>`）を sink と driver で共有する**なら書けるが、アクターモデルの外側の抜け道になる。方式 D 専用。

### 4.3 非表示をどう作るか（そして再表示の落とし穴）

**D1: 表示合図に 1 値足し、判断中核を通す（推奨に最も近い形）**

`enum TalkLifecycleSignal` へ 3 値目（例 `Interrupted`）を足し、`fn apply_lifecycle_signals` がそれを畳み込んで「全スコープを隠す」を導く。`enum VisibilityTrigger` に 5 値目（例 `Interrupted`）を足してログの語彙を分ける。

- ✅ 判断が `fn decide` の中に留まる＝`prev_visible` が正しく更新され、偽の `trigger=explicit` が出ない（要件 4.4）。
- ✅ 決定論テストが既存の形（純関数へ観測を与えて行動列を見る）でそのまま書ける＝要件 7.4 の「画面を描かずに確かめる」に直行。
- ✅ 送出端 `lifecycle_tx` は複製できるので、**UI が自分で立てる**（方式 B）も **kanade 側の中継が立てる**（方式 A）も同じ受け口に乗る。
- ⚠ `enum TalkLifecycleSignal` の doc は「talk スレッドから UI スレッドへ流れる」と書いており、UI 自身が投函する使い方は doc の改訂が要る。

**D2: 配線層が `PresentCommand::Hide` を直接発行する**

- ✅ 最短。
- ❌ 判断中核の `prev_visible` と食い違い、次フレームに偽の `trigger=explicit` が記録され、不要なポインタ滞在の掃除まで走る。要件 4.4 に抵触する。**採らない方がよい。**

**■ 落とし穴: 隠した直後の独りでの再表示（設計が必ず塞ぐべき点）**

- 出す規則は `fn decide_content` の「可視グリフ数が増えた、かつ今は不可視」。
- 可視グリフ数は `TextLayerState::visible_glyphs(actor, t)` ＝ `RevealSchedule::visible(t)` ＝「リビール時刻が `t` 以下の個数」（`crates/areka-emo-text/src/state.rs`）。リビール時刻は文字の塊が**届いた時点で先まで予定表に積まれる**（同ファイルの `fn extend_chunk`）。
- 再生を止めても、**既に届いている塊の残りの文字は時刻の進行だけで増え続ける**。よって中断で隠した次のフレームに「増えた・今は不可視」が成立し、同じバルーンが再び出る。
- 文の途中で止めれば再現し、待ち（`\w`）の最中や塊の描き終わりで止めれば再現しない。**走行ごとに出たり出なかったりする**。

塞ぎ方の候補:

1. **中断の掛け金**: 中断で隠したスコープに「次のトークが始まるまで内容では出さない」印を立て、`TalkStarted` で解く。`BalloonVisibilityState` は既にスコープごとの記憶（`struct ScopeVisibility`）と会話単位の記憶（`signal_gap_warned` 等）を持っており、同じ形で 1 本足すだけ。要件 4.7 は `TalkStarted` が解くことで満たされる。
2. **文字層を空にする**: 中断時に文字層の内容を消し、可視グリフ数を 0 へ落とす。増加エッジが二度と立たない。ただし「バルーンの可視性の既存規則は変えない」という境界（要件 4 の範囲外宣言）を越え、文字層への新しい指令が要る。
3. **リビール予定表を止める**: 文字層に「これ以降リビールしない」を教える。上流（`areka-emo-text`）の改造になり、影響範囲が最も広い。

→ 判断項目 3。なお **⑴ が既存の形に最も近く、変更が最も小さい**。

**■ 既に届いていて未描画の cue**: 表示指令（`PresentCommand`）は `Emo2Wiring.rx` に溜まり、フレームの取り出しで適用される（バルーン可視性の相はその**直後**に走る）。中断の時点で溜まっていた指令は当該フレームで適用済みになるので、隠す判断はそれらを見たうえで下される。**面（サーフェス）を戻さない**（要件 3.5）とも整合する——止めた時点で適用済みの面がそのまま残る。

### 4.4 停止指示の運び方

**S1: `Action::CancelChoice` をそのまま使い回す**

- ✅ 触るファイルが kanade の 1 か所だけ（`areka-talk`・`areka-ghost`・`actor.rs` は無改変）。
- ✅ 下流から見て安全なことは §2.2 で確認済み（dispatcher は選択の状態を見ない）。
- ❌ 名前とログ（`send_talk_command` の `kind = "cancel_choice"`・dispatcher の `cancel_choice_stale`）が中断の実体と食い違う。要件 6.1 の「スコープ番号を info で 1 行」は kanade が自分で別に書けばよいので、ここは読みやすさだけの問題。
- ❌ doc（`Action::CancelChoice` の「タイムアウト後に SHIORI が応答を返さなかった場合の解除」）の改訂が要る。

**S2: `Action`／`TalkCommand`／`DispatcherMsg` に中断専用の 4 値目を足す**

- ✅ ログと名前が実体に合う。
- ❌ `crates/areka-talk/src/lib.rs`・`crates/areka-kanade/src/{schedule/mod.rs,actor.rs}`・`crates/areka-ghost/src/dispatcher.rs` の 4 ファイルに追随が要る。`TalkCommand` は 3 値の順序保存契約を持つ型で、`impl From<TalkCommand> for DispatcherMsg` の網羅 match が増える（コンパイラが漏れを止めるので機械的ではある）。
- ❌ dispatcher 側の受け口は `fn on_cancel_choice` と**中身が同一**になる（同じ slot 突合 → 同じ `SakuraMsg::Close`）。実質の重複。

**S3: `Action::CancelChoice` を中立な名前へ改名して両用にする**（例 `Action::InterruptTalk`）

- ✅ 重複なく名前も合う。
- ❌ 既存の呼び出し・テスト・doc（選択タイムアウトの経路）を巻き込む改名になる。並走禁止の指定（#35・`translate-pipeline`・`sakura-time-directives` と `schedule/` を取り合う）を踏まえると、改名の波及は避けたいところ。

### 4.5 kanade の各状態で中断要求をどう扱うか（要件 2 の受理規則）

現状の `Input::Mouse` の扱い（`crates/areka-kanade/src/schedule/mod.rs` の横断アーム）は「Steady のみ委譲・それ以外は trace して状態不変」。中断要求も同じ枠に乗せられるが、状態ごとに意味が違う。

| 状態 | 再生中か | 中断要求への素直な答え | 備考 |
|---|---|---|---|
| `Idle`／`BootInit`／`BootPrefetch`／`BootType`／`BootMain` | いいえ | 止める対象なし（要件 2.2） | バルーンはそもそも出ていない見込み（要件 1.9 と重なる） |
| `BootVersion{talk: Some}` | はい（起動挨拶） | ここで止められるべきか要検討 | 現状 `Input::Mouse` すら届かない |
| `Steady{talk: None}` | いいえ | 止める対象なし（要件 2.2） | 居残りバルーンのダブルクリック＝裁定 5 の本体 |
| `Steady{talk: Some}` | はい | 止める（要件 2.1） | 本命 |
| `ClosePending` | いいえ（`OnClose` の応答待ち） | 止める対象なし | 握手にマウスを割り込ませない既存規律（`fn on_mouse` の close 保留ガード）と揃えるか |
| `CloseTalkWait` | はい（別れの台詞） | 止めると終了拒否へ落ちる（要件 3.6） | 既存の `value_then_interrupted_refuses_close_same_as_ended` が結果を固定済み |
| `Unloading`／`Stopped` | いいえ | 何もしない | |

→ 判断項目 4。要件 3.6 は `CloseTalkWait` で**受理する**ことを前提に書かれているが、横断アームの既定（Steady のみ）と矛盾するので明示が要る。

### 4.6 旗の寿命と入れ子（要件 5.4／5.5）

- 要件 5.5（入れ子を数えない）は真偽 1 本で足りる。
- 要件 5.4（閉じ忘れたままトークが終わったら解く）は、旗の置き場所で難易度が変わる:
  - **kanade に置く（F2）**: `TalkDone` を受けた時点（`fn on_talk_done`）で落とす。`clear_choice_ledger` と同じ掃除点に 1 行足すだけ。
  - **UI に置く（F1）**: 「トークが終わった」を UI は知らない。`TalkStarted` が来たら落とす（＝次のトークの始まりで解く）という近似になる。**間の空白期間は旗が立ったまま**で、その間に居残りバルーンをダブルクリックしても隠れない（要件 2.2 と食い違う）。→ F1 を採るなら合図の追加が要る。
  - **talk に置く（F3/方式 D）**: talk の寿命＝旗の寿命なので構造で満たされる。

## 5. 要件 ↔ 既存資産の対応表

| 要件 | 既存資産 | 差分 |
|---|---|---|
| 1.1 合図の検出と運搬 | `fn on_balloon_pointer_pressed`・`struct BalloonWindowMarker` | **Missing**: `double_click` を読む枝と送出口 1 本 |
| 1.2 左ダブルクリック限定 | `enum DoubleClick` が 6 値で列挙済み | 追加なし（列挙の 1 値だけ拾う） |
| 1.3 キャラ窓は不変 | `fn on_char_pointer_pressed` | 触らない（非退行の確認だけ） |
| 1.4 右ボタン不変 | 押下ハンドラは `left_down` 以外を素通し | 追加なし |
| 1.5 選択肢の行の 2 打目 | `fn click_selection` が `Some`／`None` を返す | **Constraint**: 判定順を「選択の確定が成立したら中断を作らない」に固定 |
| 1.6 余白の 2 打目は中断 | 同上（`None` の側） | 上と対 |
| 1.7 渡せなかったら `error!` | `fn send_selection` の warn 前例 | **Constraint**: 本仕様は `error!` を要求（既存は warn）ので語彙を揃えない |
| 1.8 結線前は `trace!` | `mouse_pressed_no_wiring`／`choice_pressed_no_emo2` の前例 | 追加なし（同型） |
| 1.9 出ていないバルーン | クリック透過の登録あり | **Unknown（R-1）**: 非表示時に押下が届くか未確認 |
| 2.1〜2.5 受理規則 | `Phase::Steady{talk}` が「再生中か」の正本 | **Missing**: 中断要求の入力とアーム |
| 2.6 選択待ちの破棄・`OnChoiceTimeout` を出さない | `clear_choice_ledger`（`steady_talk_done` 掃除点）・`fn fire_choice_timeout_if_due` | **追加不要**（既存経路で自動的に満たされる・§2.2） |
| 3.1〜3.3 その場で止まる | `fn on_close`（`drive.rs`）＋`TalkEndReason::Interrupted` | 追加なし（既に決定論テスト済み・要件 7.7） |
| 3.4 定常へ戻る | `fn on_talk_done`（steady） | 追加なし |
| 3.5 面を戻さない | `player.stop()` は面へ触れない | 追加なし（不作為の確認） |
| 3.6 別れの台詞の中断 | `fn on_close_talk_wait`＋既存テスト | **Constraint**: `CloseTalkWait` で要求を受理する経路を通すか（判断項目 4） |
| 3.7 SHIORI へ 1 件も送らない | — | **Constraint**: `Action::ShioriRequest` を積まないこと |
| 4.1〜4.3 全スコープ即時非表示 | `VisibilityAction::HideScopes`・`fn decide` | **Missing**: 中断という契機（§4.3 D1） |
| 4.4 既存の理由の意味を変えない | `enum VisibilityTrigger`（4 値） | **Missing**: 5 値目。かつ判断中核を通すこと |
| 4.5 次の描画までに消える | 相の順序（`Input`→`Update`） | **Constraint**: kanade 往復だと同フレームに間に合わない（判断項目 2） |
| 4.6 隠した後の時間切れ | `fn decide_timeout` が可視 0 で計測を破棄（`NoVisibleScope`） | **ほぼ満たす**。破棄の info 記録 1 行が出る点だけ確認要（判断項目 7） |
| 4.7 次のトークは普通に出る | `fn decide_content` の増加エッジ | **Missing／危険**: §4.3 の再表示の落とし穴を塞ぐこと |
| 5.1〜5.6 無効化の区間 | 運搬 cue は最後まで届く／受け口は 0 本 | **Missing**: 受け口 1 本＋旗の置き場（§4.2） |
| 5.7 `status` の読み口を塞がない | `ExecutionSnapshot` の SEAM 注記 | **Constraint**: 旗が kanade にあれば欄 1 本で届く（F2 の利点） |
| 5.8 SSTP の判定を置かない | — | 追加なし（零の明示） |
| 6.1〜6.6 記録 | `.kiro/steering/logging.md`・既存の `event = "…"` 規約 | **Missing**: 語彙 4〜5 種 |
| 7.1〜7.7 決定論テスト | 兄弟ファイル配置の前例多数 | **Missing**: 6 分岐＋摂動 |
| 7.8 1,000 行 | `crates/log-capture-kit/tests/file_length_guard_test.rs` | **Constraint**: §6 |
| 8.1〜8.4 台帳 | `doc/ukadoc-coverage/ledger/sakura-script.toml`・`ConsumerLedger::canonical`（6 行・件数テストあり） | **Missing**: 台帳 2 行＋消費者台帳 2 行＋件数の更新＋`roadmap-draft.md` の数え直し |

## 6. 1 ファイル 1,000 行の番人（要件 7.8）

番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 10 件・`OVER_LIMIT_ALLOWED_COUNT = 10`）。例外表は「今そこにある超過」だけを表す決まりなので、**新しいファイルを例外表へ足す道は事実上無い**（表の項目は「今も超過している」ことを別のテストが要求する＝足すには先に超過させる必要があり、規律に反する）。

編集見込みのファイルの実測（2026-09-20・本ブランチ）:

| ファイル | 行数 | 余裕 |
|---|---:|---|
| `crates/areka-kanade/src/schedule/steady.rs` | 935 | **65** |
| `crates/areka/src/input_events/balloon.rs` | 917 | **83** |
| `crates/areka/src/placement/spawn.rs` | 810 | 190（触らない見込み） |
| `crates/areka/src/emo2_boot/balloon_visibility.rs` | 770 | 230 |
| `crates/areka-kanade/src/msg.rs` | 761 | 239 |
| `crates/areka/src/input_events/balloon_pointer_handler_tests.rs` | 758 | 242 |
| `crates/areka-kanade/src/schedule/steady_choice_tests.rs` | 725 | 275 |
| `crates/areka-kanade/src/schedule/mod.rs` | 713 | 287 |
| `crates/areka/src/emo2_boot/mod.rs` | 673 | 327 |
| `crates/areka/src/emo2_boot/consumer_ledger.rs` | 665 | 335 |
| `crates/areka-kanade/src/schedule/close.rs` | 609 | 391 |
| `crates/areka/src/emo2_boot/balloon_visibility_phase.rs` | 588 | 412 |
| `crates/areka/src/input_events/mod.rs` | 542 | 458 |
| `crates/areka-sakura/src/drive.rs` | 542 | 458（触らない見込み） |
| `crates/areka-kanade/src/actor.rs` | 506 | 494 |
| `crates/areka-ghost/src/dispatcher.rs` | 429 | 571 |
| `crates/areka-kanade/src/status.rs` | 391 | 609 |
| `crates/areka/src/emo2_boot/frame/wiring.rs` | 350 | 650 |
| `crates/areka/src/emo2_boot/talk_lifecycle.rs` | 204 | 796 |
| `crates/areka-talk/src/lib.rs` | 118 | 882 |

**結論**: `steady.rs`（65 行）と `balloon.rs`（83 行）は、このリポジトリの注釈密度（判断分岐 1 本に 30〜60 行の doc が付く）を踏まえると**ほぼ確実に超える**。分割の前例は 2 つある。

- 子モジュールを兄弟ファイルへ出す: `balloon_visibility.rs` の `#[path = "balloon_visibility_phase.rs"] mod phase;`（親の非公開項目へ到達できる）。
- サブモジュールのディレクトリ化: `emo2_boot/frame.rs` → `emo2_boot/frame/`（`.kiro/steering/structure.md` が推奨する形）。

なお `break` は Rust の予約語なのでモジュール名には使えない（`user_break` 等）。

## 7. 規模とリスク

- **規模: M**（3〜7 日）。片の数は 4 つで各片は小さいが、**crate を 3 つ（`areka`・`areka-kanade`・場合により `areka-talk`／`areka-ghost`）またぎ、新しい向きの線を 1〜2 本敷き、2 ファイルを分割する**。brief の見積り「S〜M」の上側。
- **リスク: Middle**。
  - 高いところ: §4.3 の再表示の落とし穴（見落とすと「たまに消えない」欠陥になり、実機でしか気づけない）。要件 4.5 と kanade 往復の遅れの衝突（判断項目 2）。
  - 低いところ: 停止経路は完成済みで再テスト不要。受理・掃除・終了拒否は既存の経路が丸ごと使える。運搬 cue の受け口は `ReadmeCueSink` という近い手本がある。

## 8. 設計フェーズへ持ち越す調査項目（Research Needed）

- **R-1**: バルーンが非表示のとき、その窓のポインタ押下ハンドラは着火するか。α マスクのクリック透過（`fn register_ghost_windows_click_through`）が「見えない＝素通し」を保証するのか、それとも窓の矩形内では常に着火するのか。要件 1.9 を配線で満たすか自衛で満たすかが決まる。
- **R-2**: `TalkLifecycleSignal::TalkStarted` が、そのトークの最初の文字が可視グリフ数へ反映される**前**に取り出されることの保証。`wire_emo2_boot` の sink 登録順（文字 → 表示ライフサイクル）と `fn run_balloon_visibility_phase` の手順（合図の取り出し → 観測）から成り立つと読めるが、中断の掛け金（§4.3 の ⑴）を `TalkStarted` で解く設計はこの順序に依存する。
- **R-3**: 旗を kanade へ据える（F2）場合、据わる前に運搬 cue が届く窓がどれだけあるか。`OnBoot` の応答トークは boot 直後に始まるので、`wire_emo2_boot` の「受け口を作る → boot → 投函端を据える」の間に `\![enter,nouserbreakmode]` を含む起動挨拶が走る可能性。
- **R-4**: 選択待ちの段が `Cascading`／`TimeoutInFlight`（SHIORI 応答待ち）のときに中断が受理されたら、後から返ってくる応答が `fn on_reply` の choice 先行アームで新しいトークを起こしうる。中断の後に望まないトークが始まらないか。
- **R-5**: `fn decide_timeout` が中断の非表示の直後に出す `MeasurementDiscarded{reason: no_visible_scope}` の info 記録 1 行が、要件 4.6 の「余計な記録も出さない」に触れるか（既存の記録なので触れない、という読みが自然だが明示が要る）。

## 9. 設計判断項目（要件ディスカッションへ渡す）

いずれも**答えで作業が変わる**もの。既定案を添えるが決定はしない。

1. **中断を受理するかどうかを、どこで決めるか。**
   選択肢: 方式 A（kanade が全部決める・brief の採用案）／方式 B（非表示は UI・停止は kanade の分割）／方式 D（再生層が決める）。
   影響: 旗の運び先（判断項目 5）と非表示の速さ（判断項目 2）が連動して決まる。
   **既定案: 方式 A**（brief の採用案。ただし判断項目 2 の答え次第で B へ倒れる）。

2. **要件 4.5 の「次に画面が描かれるときにはもうバルーンが無い」を、kanade 往復の遅れに対してどう読むか。**
   UI が押下を検出したフレームの `Update` で隠すには、判断が UI 内で完結していなければならない（方式 B）。kanade に往復すると最短でも次のフレーム、実際にはそれ以上になる。
   選択肢: ⑴ 字義どおり「同じフレーム」を要求する（→ 方式 B へ）／⑵「利用者に見える遅れがない範囲（おおむね 1〜2 フレーム）」と読む（→ 方式 A のまま）。
   **既定案: ⑵**（記憶 `no-frame-delay-fixes-change-the-state-shape` が禁じているのは「ずれを 1 フレームの先送りで誤魔化す解」であって、別スレッドへの往復そのものではない、という読み）。**ここは開発者の確認が要る。**

3. **隠した直後の独りでの再表示（§4.3）をどう塞ぐか。**
   選択肢: ⑴ 中断の掛け金を可視性の状態へ 1 本足し `TalkStarted` で解く／⑵ 中断時に文字層の内容を消す／⑶ 文字のリビール予定表そのものを止める。
   **既定案: ⑴**（変更が最も小さく、既存の状態の持ち方と同型。要件 4.7 が `TalkStarted` で自然に満たされる）。

4. **`CloseTalkWait`（別れの台詞の再生中）で中断要求を受理するか。**
   要件 3.6 は受理を前提に書かれているが、横断ルーティングの既定（`Input::Mouse` は Steady のみ）と食い違う。`ClosePending`（`OnClose` の応答待ち）も同様に決める必要がある。
   **既定案: `CloseTalkWait` は受理する（要件 3.6 のとおり終了拒否へ落ちる）／`ClosePending` は受理しない（握手にマウスを割り込ませない既存規律＝`fn on_mouse` の close 保留ガードと揃える）**。`BootVersion{talk: Some}`（起動挨拶の最中）も同じ問いを持つ。
   **既定案（起動挨拶）: 受理しない**（起動挨拶は `\![enter,nouserbreakmode]` を使う典型例であり、そこを止められないことは利用者の不利益にならない）。

5. **旗をどこに置き、どう運ぶか。**
   選択肢: F1（受け口 → UI）／F2（受け口 → kanade・新しい向きの線）／F3（talk 内に持つ）。
   連動: 判断項目 1 が方式 A なら F2 が自然（旗と「再生中か」が同じスレッドに載り、遅れが無く、要件 5.7 の読み口も欄 1 本で開く）。方式 B なら F1。
   **既定案: F2**（要件 5.4「トークの終わりで解く」が `TalkDone` の掃除点 1 行で済み、要件 5.7 の `status` の読み口が `ExecutionSnapshot` の SEAM どおりに開く）。

6. **要件 1.9（出ていないバルーンでは合図を作らない）を、配線で満たすか自衛で満たすか。**
   選択肢: ⑴ クリック透過に任せ、押下ハンドラは何も確認しない（R-1 が「素通しする」と出た場合）／⑵ 押下ハンドラが `EmoPresenter::target_visible` を照会して自衛する。
   **既定案: R-1 の結果次第。素通しが保証できないなら ⑵**（照会の口は既に `fn collect_observations` が使っており、新しい依存は増えない）。

7. **停止指示の運び方（§4.4）。**
   選択肢: S1（`Action::CancelChoice` を使い回す）／S2（4 値目を足す）／S3（改名して両用にする）。
   **既定案: S1**（下流から見て安全なことは確認済み。触るファイルが 1 つで済み、`schedule/` を取り合う並走 spec への波及が最小。ログの語彙が実体と食い違う点は、受理側の kanade が `event = "balloon_break_accepted"` 相当の行を別に 1 本書くことで補える）。

8. **中断の非表示は「全スコープ」だが、どのスコープを対象に数えるか。**
   `fn decide` が見る母集合は「装着済みのバルーン scope」（`Emo2Wiring.balloon_models` のキー）。そのうち現に可視な scope だけを隠せば足りる（既に不可視のものへ重ねて指令を出さない＝要件 4.6 の「もう一度隠そうとしない」と同じ規律）。
   **既定案: 現に可視な scope だけを `HideScopes` に載せる**（`fn decide_timeout` の `targets` の作り方と同型）。

9. **要件 1.7 の失敗記録の水準。**
   既存の隣（`fn send_selection` の `choice_selection_send_failed`）は **warn** だが、要件 1.7 は **`error!`** を求めている。同じ「受け口が消えている」という事象に 2 つの水準が並ぶ。
   **既定案: 要件どおり `error!`**（隣を warn のまま据え置く理由——選択の取りこぼしは会話が止まるだけだが、中断の取りこぼしは利用者が黙らせる手段を失う——を設計に 1 行書く）。

## 10. 次の一歩

要件ディスカッションで判断項目 1・2・5 の 3 つ（互いに連動する）を先に決め、続いて 3・4・7 を決める。そのうえで `/kiro-design areka-P0-balloon-break` へ進む。

## 11. 要件ディスカッションの裁定記録（2026-09-20）

### 議題 1: 判断項目 1・2・5 と 4 の一部（開発者裁定）

- **判断項目 2 → ⑴ 字義どおり**。要件 4.5 は緩めない。押下と同じ描画のうちに隠す。
- **判断項目 1 → 方式 B の改良形**。§4.1 の方式 B の欠点「旗が 2 か所に写る」は、**旗を UI だけが持ち、UI が旗を見てから合図を送る**ことで消える。
  - UI（押下ハンドラ）: 旗が立っていれば何もしない（`debug!`）。立っていなければ、その場で「中断された」を可視性の判断中核へ入れ（§4.3 D1）、同時に kanade へ中断の要求を送る。
  - kanade: 再生中なら場面を問わず止める／再生中でなければ `debug!` で何もしない。**旗を持たず、「隠せ」も出さない。**
  - 「再生中でなくても隠す」（裁定 5）のおかげで UI は再生中かを知らなくてよい＝判断材料がスレッドの持ち主ごとにきれいに割れる。
- **判断項目 5 → F1**（受け口 → UI・既存 6 本と同型の 7 本目）。取り出しは `Input` の段で `dispatch_pointer_events` の**前**に置く（同じフレームの押下判定より先に旗を反映する）。F2 を採らないので **R-3（旗を据える前に cue が届く窓）は消滅**する。
- **§4.6 の F1 の空白期間は受け入れる**: 要件 5.4 を「遅くとも次のトークの始まりまでに解く」へ改訂した。`TalkStarted` の畳み込み（`fn apply_lifecycle_signals`）に 1 行足すだけで済み、`TalkLifecycleSignal` に「終わった」を足さない。閉じ忘れた台本の後の居残りバルーンは 30 秒のタイムアウトまで消せない（要件 5.4 に明記）。
- **判断項目 4 → 再生中の場面はすべて受理**（要件 2.5・2.8）: `Steady{talk: Some}`・`BootVersion{talk: Some}`・`CloseTalkWait`。`ClosePending` その他は止める対象が無いので `debug!` のみ。方式 B 改では UI が無条件に隠すので、kanade が場面で断ると「隠れたのに再生が続く」が生じる——ゆえに場面で断らない。
- **不採用になった線**: kanade → UI の「隠せ」（方式 A）・再生 → kanade の旗（F2）。どちらも敷かない。
- **後続 spec への帰結**: `areka-P0-status-execution-states` が `nouserbreak` を立てるときは、UI → kanade に「旗が変わった」を 1 種足す（既存の向き）。本仕様は作らない（要件 5.7）。

### 設計へ持ち越す調査項目の追加

- **R-6**: `BootVersion{talk: Some}` で中断の要求を受理する経路。現状 `Input::Mouse` は Steady 以外で捨てられる（横断アーム）。中断の要求は別の入力として、アクティブな talk を運ぶ phase（`crates/areka-kanade/src/schedule/mod.rs` の `fn on_talk_done` が突合対象にしている `Steady{Some}`／`BootVersion{Some}`／`CloseTalkWait`）で受理する。起動の挨拶が `Interrupted` で終わったときに boot 系列が Steady へ正しく進むか（完了 spec `kanade-boot-talkdone-drop` が直した箇所）を設計で確かめる。
- **R-7**: 旗の取り出しを `dispatch_pointer_events` の前に登録する新例の置き方（既存 2 本は後）。
