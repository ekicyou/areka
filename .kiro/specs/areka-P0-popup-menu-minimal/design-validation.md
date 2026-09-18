# 設計検証レポート: areka-P0-popup-menu-minimal

> 2026-09-18・`/kiro-validate-design`（非対話・サブエージェント）。対象は `design.md`（944 行）・`requirements.md`・`research.md` §1〜§7・`brief.md`・steering 一式。設計が既存コードについて述べている主張は、下の「裏取り」のとおりソースの定義名で確かめた（行番号では指さない）。

## 検証の要約

設計は、開発者裁定（2026-09-18・要件 7.2「メニュー表示中もゴーストは動く」）の前提を実行器のソースで裏取りしたうえで、「純粋な構造（`plan`／`decide`）」と「OS を触る薄い層（`win32.rs`）」を分け、既存の結線様式（NonSend 資源＋`Input` スケジュール・`\!` 汎用キャリアの名前選別・kanade アクター殻の即時応答）に沿って組まれている。要件 76 項目（`7.2a` 含む）はすべて追跡表に載り、1,000 行の番人・依存追加 0 も満たす。契約の書き間違い 1 件と、要件 3.10 の写し方に残る待ち時間の扱い 1 件を直せば、タスク生成へ進める。

## 裏取り（設計の主張と実ソースの突合）

| 設計の主張 | 確かめた場所（ファイル・定義） | 結果 |
|---|---|---|
| tick は `try_borrow_mut` を保持したままハンドラを呼ぶ | `crates/wintf/src/runtime/tick_bridge.rs` の `tick_one_frame_with`（`world.try_borrow_mut()` の中で `try_tick_world`） | 一致 |
| 実行器はモーダルループの中でも他のタスクを poll する | `c:\rust\cargo\registry\src\…\wintf-winmsg-executor-0.0.5\src\lib.rs` の `EXECUTOR_WINDOW`＝`Window::new(WindowType::MessageOnly, …)`（再入可の `Fn`・`new_checked` の `RefCell` 版ではない）・`WM_USER` 受信で `runnable.run()`・`spawn_unchecked_lifetime` が `PostMessageW` で投函し初回は `runnable.schedule()` | 一致。`TrackPopupMenuEx` の内部ループが投函済み `WM_USER` を配送すれば tick タスクは回る。初回 poll は tick が返った後 |
| `TrackPopupMenuEx` は借用の外で呼ぶ | `design.md`「`menu::trigger`」B 区間（借用を解いてから `win32::show`）・Invariants「B の間 World を借りていない」 | 前提は成立（要件 7.2 と整合） |
| `OnPointerReleased` は型だけあって配られない・同 tick の押下＋解放は `else if` で落ちる | `crates/wintf/src/ecs/pointer/dispatch/mod.rs` の `dispatch_pointer_events`（`OnPointerMoved`／`OnPointerPressed` のみ）・`pointer/buffers.rs` の `transfer_buffers_to_world`（`if buf.down_received … else if buf.up_received`） | 一致 |
| ドラッグは左ボタンだけが始める（要件 1.9 の門が右クリックを誤って弾かない） | `crates/wintf/src/ecs/drag/state/mod.rs` の `start_preparing`（doc「WM_LBUTTONDOWN時」）・`DragStateSnapshot::Idle` | 一致 |
| `KanadeMsg` は derive を持たず `ReplySender` を載せられる | `crates/areka-kanade/src/msg.rs` の `pub enum KanadeMsg`（derive なし）・`ShioriMsg::Request { reply: ReplySender }` の前例 | 一致 |
| `KanadeMsg::Close` は step を経ず殻で処理する前例 | `crates/areka-kanade/src/actor.rs` の `spawn_kanade` 内クロージャ（`KanadeMsg::Close => return Ok(ControlFlow::Break(()))`） | 一致 |
| `round_trip_request` は許可表で id を検査する | 同ファイル `round_trip_request`（`is_allowed_event_id ∨ is_allowed_resource_id`） | 一致 |
| `ALLOWED_RESOURCE_IDS` は `username` 1 件・凍結テストあり | `crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`・`allowed_resource_ids_are_exactly_username` | 一致 |
| `on_close` は Ref0 のみ | `crates/areka-kanade/src/schedule/events.rs` の `on_close`（`references: vec![reason.as_ref_str()]`） | 一致 |
| `MountModel` は `#[non_exhaustive]`・`resolve` は 1 欄 1 行 | `crates/areka-parsers/src/package/model.rs` の `MountModel`・`resolve.rs` の `resolve`（`map.get("name").cloned()` 形） | 一致 |
| `GhostRuntime` は `mount` を私有し公開アクセサ無し | `crates/areka-ghost/src/runtime.rs` の `GhostRuntime`（`mount: MountModel`・`kanade()`／`dispatcher()`／`sylphya_publisher()` のみ） | 一致 |
| `EcsWorldSelfRef` は wintf が注入済み・公開 | `crates/wintf/src/ecs/world/mod.rs` の `pub struct EcsWorldSelfRef(pub Weak<RefCell<EcsWorld>>)`・`runtime/mod.rs` の `wire_new_path`（`insert_non_send`）・`pub mod ecs` | 一致 |
| `MouseWiring::new`／`RegionSource` は `pub(crate)`・既存の終了テスト名 | `crates/areka/src/input_events/mod.rs`・`input_events_tests.rs` の `handler_ctrl_left_double_click_sends_one_close_request_and_keeps_the_windows` | 一致 |
| `wire_choice_drain` と同型で system を登録できる | `crates/areka/src/input_events/choice_drain.rs` の `wire_choice_drain`（`insert_non_send`＋`add_systems(… .after(dispatch_pointer_events))`） | 一致 |
| 消費者台帳に `open` は未登記 | `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical`（`move`・`bind`・`set/zorder`・`reset/zorder`） | 一致 |
| 依存追加 0 | ルート `Cargo.toml` の `windows` 機能（`Win32_Graphics_Gdi`・`Win32_UI_WindowsAndMessaging`・`Win32_UI_Shell`）・`wintf-winmsg-executor = "=0.0.5"` | 一致 |
| 1,000 行 | 実測: `resolve.rs` 963（+2）・`main.rs` 948（+5）・`steady.rs` 935（本文不変・`CloseReason::User` リテラル 0 件）・`msg.rs` 744・`input_events/mod.rs` 475（+40） | 全て 1,000 未満 |
| 要件 1.10・要件 10・追跡表 | 追跡表 76 行＝要件 76 項目（欠落 0・余分 0・機械で突合）。1.10 は `MouseWiring::defer_right_double_click`＋`trigger::decide`、10.1〜10.5 は「台帳の担当登記」節 | 被覆 |

## 重要な指摘（3 件以内）

### 🔴 指摘 1: 供給関数の契約が自分の中で食い違っている（`&World` と `&mut World`）

- **問題**: `Supplier = Rc<dyn Fn(&World, &MenuContext) -> MenuItem>`・`MenuRegistry::snapshot(&self, world: &World, …)` と定めながら、組込「説明書」の供給関数は `enabled: readme::is_available(world)` を呼び、その `is_available` は `pub(crate) fn is_available(world: &mut World) -> bool`（初回 `debug!` のために `missing_logged` を書き換える）。共有借用の中から可変借用は取れず、この形ではコンパイルできない。
- **影響**: `Supplier` の形は Revalidation Triggers に挙げた**後続 4 本が写す契約**である。設計文書のまま実装に入ると、実装者がどちらかへ黙って寄せ、後続 spec が古い形を読む。
- **提案**: `ReadmeWiring.missing_logged` を `Cell<bool>` にして `is_available(world: &World) -> bool` へ揃える（供給関数の `&World` を保つ。列挙系の供給関数は読むだけなので `&World` で足りる）。`snapshot` を `&mut World` に広げる案は、供給関数の中で World を書き換える道を開くので採らない。
- **要件**: 4.3・6.1・6.2・11.4。**根拠**: `design.md`「`menu::MenuRegistry`／`MenuWiring`／`wire_menu`」Service Interface と「areka readme」Service Interface。

### 🔴 指摘 2: 要件 3.10「起動完了前は問い合わせない」が、UI 側の 1 秒の待ちとして残る

- **問題**: 設計は「問い合わせない」を kanade の殻で写す（`actor_resources::answer` が `Steady` 以外は全件 `NoContent`）。ただし kanade の inbox は直列で、`drive` → `round_trip_request` は `reply_rx.recv()`（無期限）で SHIORI の応答を待つ（`crates/areka-kanade/src/actor.rs` の `round_trip_request`）。起動中の `OnFirstBoot` などの往復の最中に `ResourceQuery` が届くと応答は往復が終わるまで出ず、UI は `recv_timeout(1000ms)` を使い切って `QueryFailure::Timeout` → **`warn!`＋UI スレッド停止 1 秒**（tick も止まる）。要件 3.10 の「起動待ちでメニューを止めない」と、3.4 の「失敗」が正常経路で記録される点の 2 つが残る。到達経路: `spawn_kanade` のクロージャ（`run_inbox`）が 1 件ずつ処理 → `drive` の往復で塞がる → `captions::query` の上限超過。定常時も SHIORI の 1 往復が 1 秒を超えれば同じ。
- **影響**: 発生は「起動直後の右クリック」か「重い SHIORI」に限られ、上限で必ず戻るので落ちはしない。しかし設計自身が Risks に「待ちの間 UI は止まる」と書いており、要件との対応表（3.10 → `answer`）はこの残りを覆っていない。
- **提案**: ⑴ 上限を 1,000 ms から 300 ms 程度へ下げ（第 1 スライス 3 件の往復は通常数十 ms・`research.md` §7.3-3）、上限超過は `warn!` の理由に `timeout` を載せる、⑵ 追跡表の 3.10 の行に「SHIORI には問い合わせない（殻で `NoContent`）・UI の待ちは上限まで」と明記し、要件 3.10 の文言を「SHIORI へ問い合わせない」と読める形へ改めるかを設計ディスカッションで決める（要件 11.5 の同時改訂）。kanade の相を UI へ知らせる新しい口は、答えで作業が変わらない限り足さない。
- **要件**: 3.4・3.10・7.2。**根拠**: `design.md`「`menu::captions`」Implementation Notes の Risks・「kanade `ResourceQuery`」Risks・Performance & Scalability。

（3 件目に相当する指摘は無い。下の「軽微な留意点」は設計ディスカッションで一括して扱えばよい。）

## 軽微な留意点（差し戻しには当たらない）

- **台帳の id は符号化済み**: 要件 10.1 と設計「台帳の担当登記」手順 1 は `char*.popupmenu.visible`／`char*.popupmenu.type` と書くが、台帳の実キーは `ukadoc:list_shiori_resource:char_2a.popupmenu.visible:1`／`…char_2a.popupmenu.type:1`（`doc/ukadoc-coverage/ledger/shiori.toml`・README §1「id は符号化済み——見た目で直さず、カタログから写す」）。残り 11 項目・`sakura-script.toml` の `_5c_21_5bopen_2creadme_5d`・`assets.toml` の `descript_ghost:readme_2c_…` は実在を確認した。実装タスクの文面に符号化済みの id を書くこと。`UNQUERIED_POPUPMENU_RESOURCES` の `"char*.popupmenu.visible"` は語彙のみの項目なので証拠検査（`ImplementedWithoutEvidence`）には掛からない（`research.md` §7.2 の確認どおり）。
- **`borrow_mut` は `try_borrow_mut` に**: タスクの A／C 区間は `borrow_mut` と書いてある。到達する再入経路は見つからなかった（投函された `WM_USER` は `GetMessage`／`PeekMessage` のループでしか配送されず、tick の借用中にそのループは無い）が、Error Strategy「panic は使わない」に揃えて `try_borrow_mut` 失敗を `warn!`＋`in_flight=false` で畳むのが安い。
- **預かりの取り出し漏れ**: 解放ハンドラの手順 4（`in_flight` 中）は `trace!` だけで預かりを `take` しない。メニュー表示中はマウスをメニューが捕まえるので実際には届かないが、手順 3 と同じく `take` して捨てる形に揃えると「預かりが残って次の `visible=0` で送られる」道が閉じる。
- **ログの書式**: `[menu] shown` のような接頭辞＋構造化フィールドは `logging.md`「スコーププレフィックス」「構造化フィールド優先」に合う。同じ areka 層の `input_events` は `event = "…"` フィールド様式なので、`menu`／`readme` の 2 モジュールはファイル単位で一貫させればよい。

## 設計の強み

1. **前提を実ソースで裏取りしてから形を決めている**: 「表示中も動く」を成り立たせる条件（実行器がモーダルループの中でも `WM_USER` 経由で他タスクを poll する・初回 poll は tick が返った後）を `wintf-winmsg-executor` の `EXECUTOR_WINDOW`／`spawn_unchecked_lifetime` で確かめ、借用 3 区間（写し／表示／動作）の形へ落としている。本検証でも同じ結論になった。
2. **決定論で検証できる境界の切り方**: 枠と並びは `plan`（OS 非依存・`windows` を import しない）、判定は `decide`（純粋 4 組）、wintf 側は `released` を `else if` から独立した旗として足す。同 tick の押下＋解放を落とさない点まで既存 `transfer_buffers_to_world` の形から導いており、要件 9.1／9.3 のテストが判断分岐を直接踏める。

## 最終判定

**GO**（条件付き）。

- **理由**: 既存の境界・結線様式との衝突は無く、開発者裁定の前提は実行器のソースで成立する。指摘 1 は契約の書き間違いで設計文書の修正だけで解け、指摘 2 は上限の数値と要件文言の写し方の問題で構造を変えない。どちらも設計ディスカッションで決めてから `/kiro-spec-tasks` へ進める。
- **次の手順**: ⑴ 指摘 1 を `design.md` の 2 つの Service Interface に反映（`Cell<bool>`・`&World`）、⑵ 指摘 2 の上限値と要件 3.10 の文言を決めて追跡表の行を直す、⑶ 台帳タスクの文面に符号化済み id を書く、⑷ `/kiro-spec-tasks areka-P0-popup-menu-minimal`。
