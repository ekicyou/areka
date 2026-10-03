# 設計レビュー: areka-P0-drag-click-without-move

> 2026-10-03 `/kiro-validate-design`（非対話・サブエージェント）。対象は `design.md`（`0d4e2d2b`）・`requirements.md`・`research.md` §7。設計が述べるコードの事実は、この日のワークツリーの実物を Grep／Read で読み直して確かめた。ソースの指し先は行番号でなく「何を定める所か」で書く。

## 要約

根本を `DragAccumulator::set_transition`（`crates/wintf/src/ecs/drag/accumulator.rs`）の 1 か所で塞ぎ、置き場を待ち行列にして速いドラッグの上書きも同じ所で直す設計で、変える製品のファイルは 2 つ・新しい型や公開関数は無い。設計が拠り所にする事実（終了の種を積む 6 か所がすべて `set_transition` を通る・`current_dragging_entity` は `flush` で消えない・開始の腕が `DraggingState` を直接入れる・`update_dragging` は `JustEnded` で何もしない・`FlushResult.transition` の読み手は 3 か所・`Started` 無しの `Ended` を積む既存テストは 5 本・既存の `EcsWorld::new` が 3 種の `Messages` を登録している）は全部実物と一致した。修正の前に赤になるテストを「`FlushResult` の形に触らずに」組む配慮も正しく、修正前の HEAD でそのままコンパイルできる。要件との食い違いは 2 点だけで、1 点は開発者の判断、1 点は置き場の直しで済む。

## 確かめたこと（設計の主張 ⇄ 実物）

| 設計の主張 | 実物 | 判定 |
|---|---|---|
| 終了の種を積むのは 6 か所（`mouse_click.rs` の左解放 2 枝・`keyboard.rs` の ESC・`WM_CANCELMODE`・`WM_ACTIVATE`・`WM_CAPTURECHANGED`）で、すべて `DragAccumulatorResource::set_transition` を通る | `DragTransition::Ended` を積む製品コードはその 6 か所だけ。いずれも `set_transition` 経由 | 一致 |
| `current_dragging_entity` は `Started` で `Some`・`Ended` で `None`・`flush` では消えない＝「開始を積んで終了を積んでいない」の判断材料として既に在る | `DragAccumulator::set_transition` と `flush` の実装どおり（`flush` は `pending_transition.take()` と差分の 0 戻しだけ） | 一致 |
| `FlushResult.transition` の読み手は 3 か所（`accumulator.rs` の中のテスト・`dispatch.rs`・`keyboard_tests.rs`） | ワークスペース検索で同じ 3 か所。`crates/wintf/tests/` の 2 ファイルは `FlushResult` を読まない | 一致 |
| 配る所の開始の腕は `DraggingState` を Command でなく直接入れる → 同じ配りの中の終了の腕から読める | `dispatch_drag_events` の `Started` の腕は `entity_mut.insert(DraggingState{..})` | 一致 |
| 速いドラッグで開始の腕が最後に呼ぶ `update_dragging` は状態が `JustEnded` なので何もしない | `state/mod.rs` の `update_dragging` は `JustStarted`・`Dragging` だけを扱い、他は `_ => {}` | 一致 |
| 配る所と wndproc は同じスレッド（`Rc<RefCell<EcsWorld>>`）なので、配るときの状態は離した直後の `JustEnded` | `dispatch_window_message` と `dispatch_drag_events` は同じ `EcsWorld` を `Rc<RefCell<_>>` で持つ | 一致 |
| `Started` を積まずに `Ended` を積む既存テストは 5 本 | `dispatch_test.rs` の 2 本・`drag_lifecycle.rs` の 3 本。`test_window_dragging_full_lifecycle` は先に `Started` を積む | 一致 |
| 左解放の `should_end`（`Preparing`／`JustStarted`）は `find_owner_window(world, entity) == Some(window_entity)` を見るので、テストの対象 entity に `Window` を持たせれば予備の枝を通る | `find_owner_window`（`window/command.rs`）は自分が `Window` を持てば自分を返す。素の `EcsWorld` では `hit_test_in_window` が `None` を返し予備の枝へ落ちる | 一致 |
| 偽の HWND では開始の腕の枠の座標変換が失敗し `initial_inset = (0,0)` | `WindowHandle::client_to_window_rect` は `get_style()?` と `GetDpiForWindow == 0` で `Err` を返す。偽の HWND では両方とも失敗する | 一致 |
| areka の受け手 `on_char_drag_end` は `DraggingState` が無いと `WindowPos.position` へ縮退して保存する／`on_balloon_drag_end` はバルーンの `WindowPos.position` だけを読む | `drag_follow.rs` の両関数のとおり。`enqueue_window_set_pos` は `WindowHandle` が無いと `false`（偽の `fake_handle` があれば `WindowPos` を書く＝既存テスト `enqueue_window_set_pos_none_updates_position_leaves_size`） | 一致 |
| 赤テストが頼る道具（`FakePersistIo`・`PersistWiring`・`spawn_sylphya`・`load_scope`・`fake_handle`・`window_pos_sized`・`position_of`・`BalloonKeywordBase`・`MonitorSnapshot`）は既にある | `follow_drag_end_persist_tests.rs` の `on_balloon_drag_end_persists_balloon_offset_for_scope` が同じ組み立てを使う。`follow_test_support.rs` に部品がそろう | 一致 |
| 開始を積んだ後に終了を積まずに状態が抜ける経路は無い（＝門の判断材料が古くなって動かさないクリックを通してしまう穴は無い） | 製品の `end_dragging`／`cancel_dragging` の呼び手は 6 か所の終了の所だけで、`JustStarted`・`Dragging` からはいずれも先に `Ended` を積む | 一致 |

要件との照合: 2.4（縮退を残す）＝受け手は触らない・既存テストで確かめる。3.5（状態の移り変わりと捕捉の解放）＝wndproc と状態機械を触らない。5.6（兄弟ファイル・1,000 行）＝新しいファイル 2 本と既存の直し 3 本の行数見積りに余裕がある（下の問題 2 を除く）。5.7（根は `target\` の下）＝`target\drag-click-signoff\`。1 フレーム遅らせる解は無い（同じ配りの中で開始 → 終了）。

## 重要な問題（最大 3）

### 🔴 問題 1: 速いドラッグの「行き先へ置く」がバルーンの窓では成り立たない（開発者の判断が要る）

- **指摘**: 要件 2.3 は「その窓をドラッグの行き先へ置き、前 2 項（2.1 キャラ・2.2 バルーン）と同じく位置を記憶へ 1 件書く」とあり、キャラクターとバルーンの両方を対象にしている。設計は速いドラッグの行き先の置き方をキャラクターの受け手にだけ頼っている（`on_char_drag_end` が `DraggingState`＋離した位置から最終位置を求めて `enqueue_window_set_pos` で置く）。バルーンの受け手 `on_balloon_drag_end` は `DraggingState` を読まず、バルーンの `WindowPos.position` をそのまま保存する。バルーンの窓は `DragConfig::default()`（`move_window: true`）で、ドラッグ中の移動は wndproc が `Dragging` 状態の `WM_MOUSEMOVE` で行うが、速いドラッグでは `Preparing → JustStarted → JustEnded` が 1 回の画面更新に収まり `Dragging` 状態のマウス移動が 1 回も来ない（`mouse_move.rs` の `JustStarted` は `_ => {}`）。よって修正後も、バルーンを弾くように動かすと、バルーンは押す前の位置に残り、変わっていない相対位置が 1 件保存され、キーワードで決めた位置の素材（`BalloonKeywordBase`）が退役する。
- **影響**: 要件 2.3 のバルーン側が設計で満たされず、テスト計画（5.4 は「キャラクターの窓」だけを名指し・T7-5 もキャラだけ）もこの穴を見ていない。修正の前と同じ振る舞いなので後退ではないが、「速いドラッグはふつうのドラッグと同じ道筋を通る」という要件の言い方とは食い違う。
- **提案**: 開発者に 2 択で決めてもらう。(a) 要件 2.3 の「行き先へ置く」をキャラクターの窓に限ると明記し、バルーンの速いドラッグ（押す前の位置に残り・相対位置 1 件・素材の退役）は今どおりとして design の Non-Goals に 1 行書く（`on_balloon_drag_end` は境界の外・`drag_follow.rs` は 936 行で余裕 64 行）。(b) バルーンの受け手にも `DraggingState` から行き先を求めて置く腕を足す（境界を `drag_follow.rs` へ広げる＝1,000 行の上限と並走の約束を再確認し、T7-5 のバルーン版を足す）。どちらでも、動かさないクリックの修正（本 spec の本題）には影響しない。
- **要件**: 2.3・4.4・5.4
- **設計の該当箇所**: 「System Flows › 速いドラッグ（修正後）」「Components › C4」「Testing Strategy › T7-5」

### 🔴 問題 2: 累積器の新しいテスト T1 を本番ファイルの中に置くのは要件 5.6 と流儀に反する（置き場を直せば済む）

- **指摘**: 設計は T1-1〜T1-3（`ended_without_started_is_dropped` ほか）を `accumulator.rs` の中の既存 `mod tests` へ足すとしている。要件 5.6 は「新しいテストは本番ファイルの兄弟ファイルへ置き」と定め、`structure.md` の Unit Tests も「新規のテストモジュールは本番ファイルの中に本体を書かない」としている。
- **影響**: 要件の字面に反する。`accumulator.rs` は 320 行で行数の心配は無いが、同じ spec の中で `mouse_click_tests.rs` には兄弟ファイルの形を使い、累積器だけ中に書くと流儀が割れる。
- **提案**: `crates/wintf/src/ecs/drag/accumulator_tests.rs` を新しく作り、`accumulator.rs` の末尾に `#[cfg(test)] #[path = "accumulator_tests.rs"] mod accumulator_tests;` を置く。既存の 7 本もそこへ移してよい（`transition → transitions` の直しと同じ変更で入るので手間は増えない）。File Structure Plan と Testing Strategy の該当行を書き換えるだけ。
- **要件**: 5.6
- **設計の該当箇所**: 「File Structure Plan › Modified Files（製品）› accumulator.rs」「Testing Strategy › wintf・累積器の中のテスト」

### 補足（重要な問題には数えない・実装時に気を付ける点）

- `on_char_drag_end` の経路で `BalloonFollow` が無ければ `follow_balloon` は素通し（設計 C4 の Risks どおり）。T7-5 の期待値は `initial_inset = (0,0)` に寄りかかる（偽の HWND で座標変換が失敗する実装の事実）。期待値の式はテストのコメントに「組み立ての都合」と書いておく。
- 設計の C3 Risks は「`Window` の `on_add` フックは Command を積むだけ」の根拠に `dispatch_test.rs` を挙げるが、そちらは `World::new()` であって `EcsWorld::new()` ではない。フック本体（`components.rs` の `on_window_add`）は Command の積み込みだけなので結論は変わらないが、根拠の書き方は正確にしておく。
- Boundary Commitments の「`OnDragEnd` の受け手」に `cleanup_drag_state` が並んでいるが、これは `MessageReader<DragEndEvent>` の system で `OnDragEnd` の受け手ではない。意味は通るので直すなら語だけ。

## 設計の良い点

1. **根本を本当に 1 か所で直している**。6 か所の積み手が通る唯一の口に、既に在る値（`current_dragging_entity`）で判断を置くので、新しい型も状態も公開関数も要らず、今後終了を積む所が増えても自動で同じ決まりに従う。待ち行列（`Vec`）1 本で速いドラッグの上書きも同時に消え、「2 枠」や「印」より部品が少ない。
2. **修正の前に赤になるテストが、修正前の HEAD でコンパイルできる形で組まれている**。`FlushResult` の形に触らず `Messages` の `drain` の件数と受け手の保存で観測する判断、wintf 側は wndproc から `dispatch_window_message` で配り、areka 側は公開 API だけで本物の判断（`set_transition`）を通す組み立てが、要件 5.1・5.2・5.4 の「修正の前に失敗し後に通る」を字面どおり満たす。

## 判定

**GO**（問題 1 を設計ディスカッションで決め、問題 2 を置き場の直しとして反映した上で、タスク生成へ進んでよい）。

理由: 設計が頼る事実は全部実物と一致し、修正の形は要件 1・3（動かさないクリックで終了を配らない・開始 → 終了の順）を最小の差分で満たす。食い違いは速いドラッグのバルーン側（要件の言い方と設計の範囲のずれ）と、テストの置き場の 2 点だけで、どちらも本題（動かさないクリックで保存しない）の正しさを揺るがさない。

次の一歩: `/kiro-design-discussion areka-P0-drag-click-without-move` で問題 1 の 2 択を決め、問題 2 と補足の 3 点を design.md に反映 → `/kiro-spec-tasks areka-P0-drag-click-without-move`。
