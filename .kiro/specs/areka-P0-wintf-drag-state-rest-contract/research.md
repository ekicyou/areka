# ギャップ分析: areka-P0-wintf-drag-state-rest-contract

> 実測日: 2026-09-23・本ブランチ（main `92f5f448` 相当）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 対象: `requirements.md`（確定版・要件 1〜6）と既存コードの差。判断はしない。案と、要件ディスカッションへ渡す確認事項を並べる。

## 1. 要約（3〜5 行）

- **規模は S・危険は低い。** 6 要件の全部が既存ファイルの中で閉じ、新しいファイルは 0（テストも既存の兄弟ファイルへ足す）。外部依存の追加は 0・調べ物（Research Needed）は 0。
- **要件の実測はほぼ正しい。** `reset_to_idle` の製品の呼び手 0・`JustEnded` の値を読む製品コード 0・「押している間か」だけを聞く読み手 3（`start_preparing`・`nchittest_cache.rs`・`trigger.rs`）は、`crates/` 全域（`src/`・`tests/`・`examples/`）の grep で裏が取れた。brief が挙げた `resolve_transition`・`keyboard.rs` は寄せられない（問いが違う・可変で触る）ので、要件の 3 か所が正しい。
- **要件がコードと食い違う箇所は 2 件**（§5）。⑴ 要件 1.1 の「`JustStarted` は次のポインタ移動で `Dragging` へ」は誤り——実装は ECS の tick で `dispatch_drag_events`（`crates/wintf/src/ecs/drag/dispatch.rs`）が `Started` の遷移を処理するときに `update_dragging` を呼ぶ。`WM_MOUSEMOVE` に `JustStarted` の腕は無い。⑵ 要件が書く行数（445・776・182）は空行を除いた数で、実際の総行数は 499・863・214。どれも 1,000 未満だが単位を揃える。
- **設計で決める分かれ目は 6 件**（§6）。最大のものは相乗り 1 の「窓の比較をどこに置くか」——`decide` の引数を変えると既存テスト `trigger_tests.rs` の 2 本が壊れる（要件 6.5 と衝突）ので、`decide` へ渡す前に絞る形が要る。
- 記録の水準（相乗り 2）は要件の推奨どおり `warn!` で `logging.md` の表と合う。対案 `debug!`・`info!` の損得は §6-5。

## 2. いまのコード（実測）

### 2.1 ドラッグの状態（wintf）

| 項目 | 実測 | 場所 |
| --- | --- | --- |
| 状態の型 | `DragState`（5 variant・`CaptureGuard` を持つので Clone 不可）と `DragStateSnapshot`（写し・Clone 可）。述語は **0 件** | `crates/wintf/src/ecs/drag/state/mod.rs` |
| 説明の食い違い | `JustStarted`／`JustEnded` の doc が「（1フレームのみ）」・`reset_to_idle` の doc が「（dispatch_drag_events後）」 | 同上 |
| `reset_to_idle` の呼び手 | **製品 0**。定義（`state/mod.rs`）・再輸出（`crates/wintf/src/ecs/drag/mod.rs` の `pub use state::{…}`）・テスト 3 か所（`state/tests.rs` の `test_reset_to_idle_only_from_just_ended`／`test_reset_to_idle_noop_when_preparing`・`crates/wintf/src/ecs/clickthrough/controller_tests.rs` の `eval_honors_drag_snapshot_just_ended_reconverges` の後片付け）。`crates/wintf/tests/`・`crates/*/examples/`・`docs/` は 0 | grep `reset_to_idle` 全域 |
| `JustEnded` の値（entity・position・cancelled）を読む製品コード | **0**。読むのは `DragState::snapshot` の写しだけ。テストは読む（`state/tests.rs` の `test_end_dragging_from_preparing` 等・`controller_tests.rs` の `just_ended()` は手で組む） | — |
| 「1 フレームだけ観測できる」に依存する読み手 | **0**。`resolve_transition` は `Idle`／`Preparing`／`JustEnded` を同じ腕で扱う | `crates/wintf/src/ecs/clickthrough/controller.rs` |
| 製品の遷移 | 押下 `start_preparing`（`Idle`／`JustEnded` → `Preparing`）／閾値到達 `start_dragging`（`Preparing` → `JustStarted`・`crates/wintf/src/ecs/window_proc/mouse_move.rs`）／tick の `dispatch_drag_events` が `DragTransition::Started` を処理して `update_dragging`（`JustStarted` → `Dragging`・`crates/wintf/src/ecs/drag/dispatch.rs`）／`WM_MOUSEMOVE` の `Dragging` 腕が `update_dragging`（`Dragging` → `Dragging`）／解放 `end_dragging`・中断 `cancel_dragging`（→ `JustEnded`）。**`JustEnded` → `Idle` の製品の遷移は 0** | 上記各ファイル |

**`JustStarted` の出口（要件 1.1 の訂正材料）**: `mouse_move.rs` の `match state_snapshot` は `Preparing` と `Dragging` の腕しか持たず、`JustStarted` は `_ => {}` に落ちる。`JustStarted` → `Dragging` を起こすのは `dispatch.rs` の `dispatch_drag_events` の `DragTransition::Started` の腕（コメント「JustStarted→Dragging遷移（次のWM_MOUSEMOVEでDragEventが発火できるように）」）で、これは `Input` スケジュール（`crates/wintf/src/ecs/world/mod.rs` の登録）＝ **ECS の tick** で走る。したがって `JustStarted` の滞在は「閾値到達の `WM_MOUSEMOVE` から、次の tick の `dispatch_drag_events` まで」であり、「次のポインタ移動」ではない。なお `DragAccumulatorResource` が無い World では `Started` が流れず `JustStarted` に留まる（製品では常に在るので実害 0・説明に書く必要も無い）。

### 2.2 状態を読む製品コードの全件（`snapshot_drag_state`／`read_drag_state`／`update_drag_state` の呼び手・`crates/` 全域）

| 読み手 | 聞いている問い | 述語へ寄せられるか |
| --- | --- | --- |
| `start_preparing`（`state/mod.rs`） | 押している間か（`Preparing \| JustStarted \| Dragging` なら無視） | **寄せる**（`DragState` を持つ） |
| `crates/wintf/src/ecs/pointer/nchittest_cache.rs` の当たり判定（`read_drag_state` で `is_dragging` を作る所） | 押している間か（同じ 3 つなら `HTCLIENT`） | **寄せる**（`DragState` を持つ・`read_drag_state` 経由） |
| `crates/areka/src/menu/trigger.rs` の `handle_release` | 押している間か（`Idle \| JustEnded` でなければ無視） | **寄せる**（`DragStateSnapshot` を持つ） |
| `crates/wintf/src/ecs/clickthrough/controller.rs` の `resolve_transition` | 移動中か（`Dragging \| JustStarted`・`Preparing` を含めない） | 寄せない（別の問い） |
| `crates/wintf/src/ecs/window_proc/keyboard.rs` の `WM_KEYDOWN`／`WM_CANCELMODE`／`WM_ACTIVATE` | 値（entity・start_pos）を取り出す | 寄せない（値が要る） |
| 同 `WM_CAPTURECHANGED` | `update_drag_state` で `capture_guard.mark_released()` を呼ぶ（可変） | 寄せない（写しでは書けない） |
| `crates/wintf/src/ecs/window_proc/mouse_click.rs` の `handle_button_message`（当たり経路と fallback 経路の 2 か所） | 値（hwnd・entity）を取り出して `should_end` を作る | 寄せない |
| `crates/wintf/src/ecs/window_proc/mouse_move.rs` の `WM_MOUSEMOVE` | 値（start_pos・prev_pos・hwnd…）を取り出す | 寄せない |

**brief の「wintf 内 3 か所（`resolve_transition`・`start_preparing`・`keyboard.rs`）」との差**: 上表のとおり `resolve_transition` は別の問い、`keyboard.rs` は値取り出しか可変アクセス。代わりに brief が見落としていた `nchittest_cache.rs` が「押している間か」だけを聞く。**見落とした読み手は 0**（`crates/areka/src/placement/spawn.rs`・`follow/drag_follow.rs` は `drag::` を参照するが、読むのは `DragConfig`／`DraggingState` 等の ECS 部品で、`DragState` は読まない）。

**既存の非対称（範囲外・報告のみ）**: `WM_ACTIVATE` は `Dragging` と `Preparing` で `cancel_dragging` を呼ぶが `JustStarted` は `_ => {}` で呼ばない（`WM_KEYDOWN`／`WM_CANCELMODE`／`WM_CAPTURECHANGED` は 3 つとも扱う）。本仕様は `keyboard.rs` を書き換えない（要件 2.6）ので触らないが、`JustStarted` 中の Alt+Tab で捕捉が残る潜在欠陥として `areka-P0-popup-menu-residue` 型の台帳へ渡す候補になる。

### 2.3 相乗り 1（別の窓の預かり）

- `PendingDoubleClick`（`crates/areka/src/input_events/mod.rs`）は `scope: u32` を持つ。預けるのは `on_char_pointer_pressed` の右ダブルクリックの腕（`defer_right_double_click`・高々 1 件・上書き）。
- `poll_once`（`trigger.rs`）は返事が決着した tick に `take_deferred_double_click` で取り出し、`decide(interpreted.visibility, deferred.as_ref())` の結果 `Suppress { send_double_click: true }` なら `send_pending_right_double_click(deferred)` を呼ぶ。**`deferred.scope` と `request.scope` の比較は 0 件**（要件どおり）。
- `decide` は `(Visibility, Option<&PendingDoubleClick>) -> Decision` の純粋関数。既存テスト `crates/areka/src/menu/trigger_tests.rs` の `decide_sends_the_double_click_only_when_suppressed_and_deferred` は、見本の預かり `pending_double_click()` が **scope 1**、見本の要求 `request()` が **scope 0** で組んである（純粋関数は scope を見ないので今は通る）。→ `decide` に scope の比較を入れると **この既存テストが赤になる**（要件 6.5 と衝突・§6-4）。
- 既存の「同じ窓の預かり＋抑止」の確認は `crates/areka/src/menu/trigger_flow_tests.rs` の `a_late_suppressing_reply_still_delivers_the_right_double_click`（scope 0 の窓＋`defer_double_click` は scope 0 固定）。新テストは scope 1 の預かりを置く手段が要る（既存ヘルパ `defer_double_click` は scope 0 を書き下している→引数付きの兄弟ヘルパを足すか、テスト内で直に `defer_right_double_click` を呼ぶ）。
- 記録: `trace!` は `log_capture_kit::capture_lines(LineFormat::LevelFields, …)` で拾える（同ファイルの `menu_lines(&lines, "TRACE")` が既に使っている）。既存 event 名 `menu_deferred_double_click_dropped` は「表示するので捨てた」の意味で使用中→新しい event 名が要る（要件 6.4）。

### 2.4 相乗り 2（説明書の無いゴースト）

- 入口は 2 つとも `open_from_world`（`crates/areka/src/readme.rs`）へ来る: メニューの「説明書」の動作（`crates/areka/src/menu/mod.rs` の `readme::open_from_world(world)`）と台本の要求の取り出し（`readme.rs` の `drain_readme_requests` が要求 1 件につき 1 回呼ぶ）。**ガードは 1 か所で両方を覆う**。
- `open_from_world` は `wiring.path` の実在を見ずに `open` を呼ぶ。`open` は `ShellExecuteW` の戻り値が 32 以下なら `error!`（`readme_open_failed`・パスと符号つき）。実在しないファイルなら符号 2・ダイアログは出ない（`readme_tests.rs` の `open_records_a_missing_file_as_an_error_and_returns_err` が固定済み）。
- メニュー側 `is_available` は `wiring.path.exists()` で灰色にし、`missing_logged: Cell<bool>` で「無い」を初回だけ `debug!`（`readme_missing`）。**`open_from_world` から `is_available` を呼ぶと `missing_logged` を消費し、メニュー側の初回 `debug!` が出なくなる**（要件 5.4 に抵触）→ `open_from_world` は `wiring.path.exists()` を直に見る方が安全（§6-6）。
- テストの道具は揃っている: `temp_path_kit::TempPath`・`log_capture_kit`・`wired(&mut world, path)` ヘルパ（`readme_tests.rs`）。「OS を 0 回呼んだ」は `readme_open_failed` 0 行＋`readme_opened` 0 行で言える。

### 2.5 テストファイルの行数（1,000 行の上限に対して）

| ファイル | 総行数 | 空行を除く | 要件の記載 | 余裕（総行数） |
| --- | --- | --- | --- | --- |
| `crates/wintf/src/ecs/drag/state/tests.rs` | **499** | 445 | 445 | 501 |
| `crates/wintf/src/ecs/clickthrough/controller_tests.rs` | 642 | 555 | （記載なし） | 358 |
| `crates/areka/src/menu/trigger_flow_tests.rs` | **863** | 776 | 776 | 137 |
| `crates/areka/src/menu/trigger_tests.rs` | 197 | 172 | （記載なし） | 803 |
| `crates/areka/src/readme_tests.rs` | **214** | 182 | 182 | 786 |

要件の数は空行を除いた数（`Measure-Object -Line` 相当）。`structure.md` の「1 ファイル 1,000 行以下」は単位を明記していないが、総行数で見ても 3 本とも収まる。`trigger_flow_tests.rs` は新テスト 1 本（40〜60 行の見込み）を足しても 900〜925 行で収まるが、余裕は最も小さい。

## 3. 要件 → 資産の対応表（欠けは Missing／不明は Unknown／制約は Constraint）

| 要件 | 既存資産 | 差 | 印 |
| --- | --- | --- | --- |
| 1.1〜1.2 状態の説明 | `DragState` の doc（`state/mod.rs`） | 5 variant の doc を出来事で書き直す。`JustStarted` の出口は「次の tick の `dispatch_drag_events`」（§2.1） | 要件の文言修正（§5-1） |
| 1.3 `reset_to_idle` 撤去 | 定義＋再輸出＋テスト 3 か所 | 定義と `pub use` の 1 語を消し、テスト 2 本を除外・1 か所を置換 | Missing（削除のみ） |
| 1.3 後段「製品の呼び手 0 で時機を語る関数 0」 | `check_threshold`（`state/mod.rs`）も製品の呼び手 0（自身の doc がそう書く） | doc は閾値判定の説明で製品の時機は語らない → 撤去対象に **含めない**（0 件の根拠を設計に書く） | Constraint |
| 1.4 遷移を足さず減らさず | `start_preparing`／`end_dragging`／`cancel_dragging` | 変更 0 | — |
| 1.5 R5.2 の説明 | `controller.rs` の `resolve_transition` doc・`runtime/mod.rs` の `wire_click_through` doc | 「終了直後の周で固定が外れ現在の当たりへ戻る」の意味は今の文言で既に成り立つ。ただし `controller.rs` doc 1 項と `docs/click_through.md` は `JustStarted` を「直前 1 フレーム」と書く（§6-2） | Unknown（触るか） |
| 2.1〜2.2 述語 | 0 件 | `DragState` と `DragStateSnapshot` の両方から呼べる 1 述語（§4 案 A） | Missing |
| 2.3〜2.4 寄せ替え 3 か所 | §2.2 | `matches!` 3 か所を述語呼び出しへ | Missing |
| 2.5 並べる箇所 0 | §2.2 | 寄せ替え後の残りは `resolve_transition`（別の問い）と値取り出しのみ | — |
| 3.1〜3.4 契約テスト | `test_start_preparing_allowed_from_just_ended`（既に押下→解放→次の押下を踏む）・`force_idle` ヘルパ | 述語の真偽と「他の関数を呼ばない」を確かめる新テスト 1〜2 本を足す（既存テストは変えない） | Missing |
| 3.5 既存テストの追随 | §2.1 | 2 本除外・`controller_tests.rs` の後片付けを `update_drag_state(\|s\| *s = DragState::Idle)` へ（`JustEnded` は `CaptureGuard` を持たないので借用中の代入で安全） | Missing |
| 3.7 1,000 行 | §2.5 | 収まる | Constraint |
| 4.1〜4.4 窓の比較 | `PendingDoubleClick.scope`・`MenuRequest.scope`・`decide` | 比較 1 条件＋`trace!` 1 行。置き場は §6-4 | Missing |
| 4.5〜4.6 テスト | `trigger_flow_tests.rs` のヘルパ群 | scope 1 の預かりを置くヘルパが無い（§2.3） | Missing |
| 5.1〜5.3 実在チェック | `open_from_world`・`is_available` | `wiring.path.exists()` を先に見て `warn!`・新 event 名 | Missing |
| 5.4 メニュー側不変 | `is_available`・`missing_logged` | `is_available` を再利用しない（§2.4） | Constraint |
| 5.5 テスト | `readme_tests.rs` の `wired`・`TempPath`・`capture` | 1 本足す | Missing |
| 6.4 語彙 | `[drag]`・`[menu]`・`[readme]` | 新 event は新名（例: `menu_deferred_double_click_scope_mismatch`・`readme_open_skipped_missing`） | Constraint |
| 6.5 既存テスト不変 | `trigger_tests.rs` の `decide_*` 2 本 | `decide` の引数を変えると赤 → §6-4 | Constraint |

## 4. 実装の案

### 案 A: 既存ファイルの中で閉じる（推奨・要件 Boundary と一致）

- `state/mod.rs`: doc 書き直し・`reset_to_idle` 削除・述語追加（下の A-1〜A-3）。
- `drag/mod.rs`: `pub use` から `reset_to_idle` を外す。
- `nchittest_cache.rs`・`start_preparing`・`trigger.rs`: `matches!` を述語へ。
- `trigger.rs` の `poll_once`: 預かりを `decide` へ渡す前に窓で絞る（§6-4 の C1）。
- `readme.rs` の `open_from_world`: `exists()` ガード＋`warn!`。
- テスト: `state/tests.rs`（2 本除外・1〜2 本追加）・`controller_tests.rs`（後片付け置換）・`trigger_flow_tests.rs`（1 本追加＋scope 付きヘルパ）・`readme_tests.rs`（1 本追加）。
- 新規ファイル 0・新規依存 0。

述語の形（どれも 1〜2 行）:
- **A-1**: `impl DragState { pub fn is_button_held(&self) -> bool { matches!(self, Preparing{..}|JustStarted{..}|Dragging{..}) } }` と `impl DragStateSnapshot` に同じ 1 行。variant の並びが 2 か所に残るが、どちらも述語の定義そのものなので要件 2.5 の「答えるためだけに並べている箇所」には当たらない。
- **A-2**: `DragStateSnapshot` にだけ書き、`DragState::is_button_held` は `self.snapshot().is_button_held()` に委ねる。並びは 1 か所。写しを作る費用（数十バイトのコピー）が `WM_NCHITTEST` の cache miss 経路に乗るが実測不能の差。
- **A-3**: A-1 に加えて thread-local を読む自由関数 `is_button_held()` も置く。読み手 3 か所のうち `nchittest_cache.rs` と `trigger.rs` が `read_drag_state(|s| s.is_button_held())`／`snapshot_drag_state().is_button_held()` の 1 行で済むので、自由関数は無くても書ける＝足す理由が無い（YAGNI）。

トレードオフ: ✅ 差分最小・既存の作法（`matches!` の 1 行・兄弟テスト）そのまま。❌ 無し（案 B・C を選ぶ理由が見当たらない）。

### 案 B: 述語を別 module（例 `drag/predicates.rs`）へ切り出す

新ファイル 1・利点は無い（述語 1 つに module は過剰）。**却下候補**。

### 案 C: 相乗り 2 件を別 spec に戻す

`trigger.rs` を 2 spec が触ることになり直列化するだけ（brief の相乗りの根拠そのもの）。**却下候補**。

### 規模と危険

- **規模: S**（1 日以内）。差分は doc・1 行の述語×2・`matches!` 3 か所・条件 2 つ・テスト 4〜5 本。
- **危険: 低**。挙動の変更は相乗り 2 件の判断分岐だけで、どちらも決定論テストで到達できる。実機確認は要らない（要件 Adjacent expectations と一致）。
- 唯一の罠: `decide` の引数変更が既存テストを壊す（§6-4）。設計で C1 を選べば消える。

## 5. 要件がコードと食い違う箇所（要件ディスカッションで文言を直す）

1. **要件 1.1 の括弧「`JustStarted` は次のポインタ移動で `Dragging` へ」**は実装と違う。`JustStarted` → `Dragging` は `crates/wintf/src/ecs/drag/dispatch.rs` の `dispatch_drag_events`（`DragTransition::Started` の腕で `update_dragging` を呼ぶ・`Input` スケジュールの tick）で起きる。`WM_MOUSEMOVE`（`mouse_move.rs`）には `JustStarted` の腕が無い。直し: 「`JustStarted` は次の tick の `dispatch_drag_events` で `Dragging` へ（解放・中断なら `JustEnded` へ）」。
2. **行数の単位**: 要件 3.7・4.5・5.5 の 445／776／182 は空行を除いた数。総行数は 499／863／214（§2.5）。上限判定には影響しない（どれも 1,000 未満）が、単位を「総行数」に揃えるか「空行を除く」と明記する。
3. **要件 4.4「判断の場所を 2 つにしない」と要件 6.5「既存テストを 1 本も変えない」の両立**: `decide` の引数に scope を足すと `trigger_tests.rs` の `decide_sends_the_double_click_only_when_suppressed_and_deferred`（預かり scope 1・要求 scope 0）が赤になる。要件そのものの矛盾ではなく、設計の置き場を縛る（§6-4）。要件の文言を「窓の比較は `decide` へ渡す預かりを絞る形でもよい」と緩めるか、`decide` の引数を変えて既存テスト 2 本の見本を直すか（6.5 の例外に加える）のどちらか。
4. **要件 1.5 の対象ファイル**: `docs/click_through.md`（「ドラッグ中の透過抑止」の段落）と `controller.rs` の `resolve_transition` doc 1 項が `JustStarted` を「直前 1 フレーム」と書く。要件 1.1 が禁じるのは `DragState` の説明でのフレーム数表現だけなので違反ではないが、同じ語彙が残る。触るかどうかは §6-2。
5. 要件 Introduction「`start_preparing`（`Idle`／`JustEnded` → `Preparing`）」「解放は `end_dragging`（`Preparing`／`JustStarted`／`Dragging` → `JustEnded`）」「中断は `cancel_dragging`」「`resolve_transition` は `Idle`・`Preparing`・`JustEnded` を同じ枝」「`reset_to_idle` の呼び手 0」「`JustEnded` の値の読み手 0」「読み手 3 か所」「`WM_CAPTURECHANGED` は可変」「`PendingDoubleClick.scope` あり・比較 0」「`open_from_world` は実在を見ない」——**すべて実測と一致**（食い違い 0）。

## 6. 設計で決める分かれ目（要件ディスカッションへ）

1. **述語の名前と置き場**（§4 A-1／A-2／A-3）。名前の候補: `is_button_held`（brief の例・「押している間」の直訳）／`is_left_button_held`（右・中ボタンのドラッグは範囲外だが名前で限定する）。推奨は A-1＋`is_button_held`（差分最小・並びは定義 2 か所のみ）。
2. **「1 フレーム」の語彙を `state/mod.rs` の外でも消すか**: `controller.rs` の `resolve_transition` doc 1 項・`docs/click_through.md` の 1 段落。消すなら「閾値到達から次の tick の `dispatch_drag_events` まで」へ。消さないなら要件 1.5 に「`JustStarted` の『1 フレーム』は残す」と書いて 0 件を明示する。
3. **契約テストの形**: ⒜ 新テスト 1 本で解放と中断の両方を踏む／⒝ 解放 1 本＋中断 1 本の兄弟 2 本（要件 3.4 はどちらも許す）。既存の `test_start_preparing_allowed_from_just_ended` は「`JustEnded` から次の押下を受け付ける」を既に固定しているので、新テストが足すのは「述語が偽」と「他の関数を 1 つも呼ばない」の 2 点。既存テストに assert を足す手もあるが要件 6.5（既存テスト不変）に反するので新設。
4. **相乗り 1 の比較の置き場**: **C1** `decide(visibility, deferred.as_ref().filter(|d| d.scope == scope))` と、絞られて落ちたときの `trace!` を `poll_once` に置く（`decide` 不変・既存テスト緑）／**C2** `decide` の引数に `scope` を足す（判断が本当に 1 か所になるが `trigger_tests.rs` の見本 2 本を直す＝要件 6.5 の例外）／**C3** `Suppress` の腕の中で比べる（判断が 2 か所＝要件 4.4 違反・却下）。推奨は C1。C1 でも「送る／送らない」を決めるのは `decide` のままで、窓の比較は `decide` への入力（要件 4.4 の文言どおり）。
5. **相乗り 2 の記録の水準**（要件の確認事項 2）: `warn!`（推奨・`logging.md` の表「回復可能なエラー・フォールバック」）／`debug!`（メニュー側 `readme_missing` と揃う・利用者の障害調査では見えない）／`info!`（ライフサイクルの表に合わない）。`error!` は要件が既に除外。
6. **相乗り 2 の実在チェックの書き方**: `wiring.path.exists()` を `open_from_world` に直に書く（推奨・`is_available` は `missing_logged` を消費するので呼ばない＝要件 5.4 を守る）。`is_available` と `open_from_world` で `exists()` が 2 か所になるが、片方は「灰色にする」もう片方は「開かない」で問いが違う。

## 7. Research Needed

- **0 件**。外部依存の追加は無く、使う道具（`log_capture_kit`・`temp_path_kit`・`update_drag_state`）はどれも同じ crate のテストで使用実績がある。

## 8. 設計フェーズへの推奨

- 案 A・述語は A-1（`is_button_held` を `DragState` と `DragStateSnapshot` に各 1 行）・相乗り 1 は C1・相乗り 2 は `exists()` 直書き＋`warn!`。
- 要件 1.1 の `JustStarted` の出口の文言を先に直す（§5-1）。設計の状態遷移図はこの訂正を前提に描く。
- タスクは 5〜7 本の見立てどおり: ①doc＋述語＋撤去（wintf）②読み手 3 か所③契約テスト＋既存テスト追随④相乗り 1＋テスト⑤相乗り 2＋テスト（⑥ `docs/click_through.md` の語彙は §6-2 の裁定次第）。
