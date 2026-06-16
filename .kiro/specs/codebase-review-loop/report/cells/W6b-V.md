# W6b-V: wintf ドラッグ × 脆弱性レビュー

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- 観点: V（脆弱性レビュー）。基準: design.md「Security Considerations」（unsafe 境界・整数変換の切り捨て/オーバーフロー・Win32/COM ハンドルのリーク/二重解放・外部入力の検証欠如・panic 経路 DoS を点検し、**挙動を変えない範囲（内部チェック・debug_assert・不変条件コメント・安全な型置換）の対策のみ投入**。API シグネチャ/エラー応答を変える対策・キャプチャ解放のロジック変更・入力検証厳格化は proposals.md へ）。CellExecutor V 規則（R2.3/R2.4）、観点順序 T→S→V（R2.7、W6b-T/W6b-S 完了済みの回帰検知器上で実行）。
- requirements（source 番号）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations、CellExecutor 観点別規則（V、design.md:338）、提案記録様式、セル断片様式。領域定義「W6b: wintf ドラッグ / `crates/wintf/src/ecs/drag/`」。
- 参考: `report/cells/W6b-T.md`（モジュール×テスト対応表・デバイス依存所見1〜4・P61）、`W6b-S.md`（prev_frame_pos 除去・clippy 簡素化・is_released 見送り）、`W6a-V.md`・`W4b-V.md`（V 観点・整数境界・panic 点検・debug_assert/不変条件コメント様式の先例）。
- 境界: `crates/wintf/src/ecs/drag/`（tests/ の該当ドメインは境界内）。`ecs/pointer/`（W6a）・`ecs/window_proc/`（W7a、状態機械・閾値の本番呼び出し元）・`ecs/window/`（WindowHandle 等）には変更を加えていない（読み取りのみ＝本番経路の裏取りと座標 provenance の確認のため）。
- 起点: W6b-S 適用後のクリーンなワークツリー（drag 境界に未コミット差分なし、HEAD = `0c14a9a`）、ベースライン S2 = 1566 passed / 0 failed。反復検証用 drag:: ベースライン = 44 passed / 0 failed（開始時実測）。

## 点検手法

境界内 7 ファイル（mod.rs / state.rs / accumulator.rs / context.rs / dispatch.rs / capture_guard.rs / systems.rs + in-source tests）を grep（複数パターン）＋全文精読で走査した。

- panic 経路: `unwrap()` / `expect(` / `panic!` / `unreachable!` / `unimplemented!` / `todo!` / 配列・スライス添字 `[i]`
- 整数境界: `as i32` / `as f32` / `as usize`（切り捨て・符号反転・飽和）・整数乗算（`*`）/加減算（`+=`/`-`）のオーバーフロー/アンダーフロー・`saturating_`/`checked_`/`wrapping_`
- キャプチャ解放: `SetCapture` / `ReleaseCapture` / `Drop` / `CaptureGuard` / `mem::replace` のライフサイクル追跡（取得→終了/キャンセル/状態遷移での確実な解放、early return・panic・状態遷移漏れでのリーク、二重キャプチャ）
- 状態不変条件: `DragState` 5 状態の遷移不変条件、accumulator/context の `Arc<Mutex>` 共有・ロック整合性

本番経路の裏取り（過去セルの未確認主張による REJECTED を回避。**キャプチャ解放保証・状態遷移・座標 provenance は実コードで確認**）:
- **CaptureGuard の本番ライフサイクル**: `SetCapture` 呼び出しは `CaptureGuard::acquire`（capture_guard.rs:24-36）の1箇所、本番の `acquire` 呼び出し元は `start_preparing`（state.rs:237、`update_drag_state` 内で1回）のみ。`ReleaseCapture` は `Drop`（capture_guard.rs:53-63、`released==false` のときのみ）の1箇所。WM_CAPTURECHANGED 受信時の `mark_released` 経路は `window_proc/`（W7a、境界外）にあることを grep で確認（本セルでは読み取りのみ）。
- **座標 provenance**: `PhysicalPoint = Point { x: i32, y: i32 }`（types.rs:25-28、`pointer/types.rs:18` で別名）。本番のドラッグ座標は `window_proc/mouse_move.rs:110-111` の `(lparam & 0xFFFF) as i16 as i32`（**i16 クライアント座標範囲 [-32768,32767]**）＋ `WindowPos.position`（i32、実モニタ幾何で有界）のオフセット（mouse_move.rs:172-174 `screen = client + pos`）。
- **check_threshold の本番未使用**: `check_threshold`（state.rs）の呼び出し元はワークスペース全 grep で in-source テストのみ。本番の閾値判定は `mouse_move.rs:201-204` に**同一算術がインライン複製**（`dx*dx + dy*dy >= threshold*threshold`）されており、`check_threshold` を経由しない（→ 所見2/P62）。

## 発見した脆弱性候補と判定

### 1. キャプチャ解放漏れ（CaptureGuard ライフサイクル）— 現状の解放保証は健全。終了/キャンセル経路に解放保証の debug_assert＋不変条件コメントを適用

`CaptureGuard`（SetCapture/ReleaseCapture の RAII）のライフサイクルを取得→各遷移→終了/キャンセルまで追跡した。

- **取得（二重キャプチャ防止）— 現状安全**: `start_preparing`（state.rs:223-254）は冒頭で `matches!(state, Preparing|JustStarted|Dragging)` のとき early-return し（state.rs:227-235）、アクティブなドラッグ中は新規 `SetCapture` を呼ばない。したがって1ドラッグにつき `acquire`（SetCapture）は最大1回で**二重キャプチャは構造的に発生しない**（W6b-T `test_start_preparing_ignored_when_already_active` で外側挙動を特性化済み）。**対策不要。**
- **遷移時のガード移動（リーク/誤解放なし）— 現状安全**: `start_dragging`（Preparing→JustStarted、state.rs:266-290）・`update_dragging`（JustStarted→Dragging / Dragging→Dragging、state.rs:307-383）は `std::mem::replace(state, Idle)` で旧状態を取り出し、`if let`/match で**同一の `capture_guard` を新状態へ move** する。`if let` の destructure は直前の `matches!`/match アームでバリアントが確定済みのため必ず成功し、ガードが意図せずドロップ（＝途中解放）される経路はない。`replace` で一時的に `Idle` を経るが、これは同一 `update_drag_state` クロージャの borrow 内で完結し（割り込みなし）、新状態の再代入までの間に観測されない。**対策不要**（W6b-T の状態遷移21件が回帰検知器）。
- **終了/キャンセル時の解放保証 — debug_assert＋不変条件コメントを適用**: `end_dragging`（state.rs:396-）・`cancel_dragging`（state.rs:435-）は、`RefCell` borrow 中に `Drop`（ReleaseCapture が同期的に WM_CAPTURECHANGED を配信）すると `RefCell already borrowed` パニックになるため、`CaptureGuard` をクロージャ外（`_guard`）に取り出し borrow 解放後にドロップする（既存コメント state.rs:392-394/428 で明記）。この設計は `tests/drag/capture_guard_panic_safety_test.rs` で実証済み。ここで内側の抽出 `match old { Preparing|JustStarted|Dragging => Some(capture_guard), _ => None }` の `_ => None` は、外側 match で state が既にアクティブなドラッグ状態に確定しているため**構造的に到達不能**であり、`capture_guard` は必ず `Some`（＝終了/キャンセルのたびに確実に1個のガードが取り出され→ドロップ→ReleaseCapture）。この**キャプチャ解放保証の不変条件**を `debug_assert!(capture_guard.is_some(), ...)` ＋根拠コメントで明文化した（両関数）。リリースで compile-out（挙動不変）、全 well-formed 状態で発火せず。R2.3「挙動を変えない内部チェック」に該当。
- **Dragging 放置 / reset の不変条件 — 現状安全（境界外駆動）**: `reset_to_idle`（state.rs:493-499）は JustEnded のときのみ Idle へ戻す（W6b-T `test_reset_to_idle_*` で特性化）。Dragging→終了の駆動は WM_LBUTTONUP/WM_CANCELMODE（`window_proc/`、W7a）であり、状態機械そのものの遷移契約は健全。「Dragging のまま放置でキャプチャがリークする」のは UI スレッドがメッセージを供給し続けない異常時のみで、これは W7a のメッセージ結線の責務（境界外）。thread_local の `DRAG_STATE` はスレッド終了時に Drop され（残存ガードがあれば ReleaseCapture）、プロセス生存中の論理的解放は終了/キャンセル経路が担保する。**本境界では対策不要**（解放保証の核は上記 debug_assert で文書化）。

### 2. 座標計算の整数境界 — check_threshold の i32 桁あふれ境界は実用座標で安全。安全鎖を特性化テストで固定し、複製/桁あふれ堅牢化は P62 へ

- **`check_threshold` の距離二乗 i32 乗算（state.rs:506-509）** — `dx = current_pos.x - start_pos.x`（i32 減算）・`distance_sq = dx*dx + dy*dy`（i32 乗算）・`threshold_sq = threshold*threshold`。`dx*dx` は `|dx| > 46340`（46341² > i32::MAX）で **debug ビルドでは桁あふれ panic・release ビルドでラップ**する理論境界がある。ただし本番座標差は i16 クライアント幅オーダー（前述 provenance）で 46340 に達せず**現実的には安全**。さらに `check_threshold` 自体は**本番未使用**で、本番判定は `mouse_move.rs:201-204` のインライン複製が担う（同一算術・同一桁あふれ境界）。→ **挙動非破壊の整数境界ハザードコメントを `check_threshold` に追記**（i32 乗算の桁あふれ境界・本番未使用・インライン複製・P62 参照を明記、コード挙動不変）＋ **安全鎖を特性化テスト 2 件で固定**（負デルタ対称性・i16 極値非桁あふれ）。飽和/checked 化（極値での判定結果が変わる）・複製統合（pub API 表面 or W6b↔W7a 境界跨ぎ）はいずれも挙動/構造変更のため → **P62** へ記録（R2.4/R2.9/R5.2）。
- **`accumulate_delta` の i32 加算（accumulator.rs:55-56）** — `self.accumulated_delta.x += delta.x`（i32 +=）。累積和が i32::MAX 超過で debug 桁あふれ panic の理論経路はあるが、本番デルタは1フレームのマウス移動量（i16 幅オーダー）で、`flush`（dispatch_drag_events、毎 ECS tick）が都度デルタを 0 リセットする（accumulator.rs:87）ため、flush 間の累積は微小で桁あふれに達しない。供給元 `accumulate_delta(delta)` の `delta = current_pos - prev_pos`（mouse_move.rs:240-243、隣接フレーム差）も1移動分。**現状安全（対策不要）**。W6b-T の `test_accumulate_delta_sums` が加算挙動を特性化済み。
- **`as f32` キャスト（dispatch.rs:159・294-295）** — `initial_window_pos.x/y as f32`（DraggingState.initial_inset、(f32,f32)）・`pos.x/y as f32`（Offset、f32）。いずれも **i32→f32 の拡大キャスト**で、実用ウィンドウ座標範囲（|coord| < 2^24 ≈ 16.7M、実画面を遥かに超える）で無損失。供給元は WindowPos/Point（内部 ECS 値、外部ファイル流入経路なし）。W6a-V/W4b-V が同型で「内部値・拡大キャスト・無損失」と判定済みのため**二重記録せず現状安全**と判定（churn 回避でコメント追記も見送り）。

### 3. 状態不変条件・共有状態（Arc<Mutex>）— 現状安全（対策不要）

- **DragState 遷移不変条件**: 5 状態（Idle/Preparing/JustStarted/Dragging/JustEnded）の遷移は W6b-T で全面特性化済み（21件）。各遷移関数は `update_drag_state` の単一 borrow 内で `mem::replace` により atomic に状態を差し替え、中間状態（一時的 Idle）が外部観測されない。`start_dragging`/`update_dragging`/`start_preparing` の非対象状態 no-op、`end_dragging`/`cancel_dragging` の JustEnded 生成も確定的。**panic・未定義動作・不整合なし。現状安全。**
- **accumulator / context の Arc<Mutex> 共有**: `DragAccumulatorResource`・`WindowDragContextResource` は `if let Ok(mut acc) = self.inner.lock()` でロック失敗（poison）時は黙って no-op（accumulator.rs:136/143/150、context.rs:62/74）、`flush`/`get` は `.lock().ok().map(...)` で poison 時 None を返す（accumulator.rs:157、context.rs:69）。**poison でパニックしない**（unwrap 不使用）。ロック粒度は単一メソッド内に閉じデッドロック経路なし。`unsafe impl Send/Sync for WindowDragContext`（context.rs:29-30）は HWND（実体は整数ハンドル）を Arc<Mutex> 越しに渡す既存の安全性主張で、コメントで根拠明記済み（データ競合は Mutex が排除）。**現状安全（対策不要）。**

### 4. panic 経路 — 本番経路に到達可能な panic はゼロ（対策不要）

- 境界内の `unwrap()`/`expect()`/`panic!`/`unreachable!` の本番（非 `#[cfg(test)]`）出現は**ゼロ**。grep で検出された `expect(`/`panic!` は全て `#[cfg(test)] mod tests` 内（context.rs:132+、accumulator.rs:172+、state.rs:541+ のテスト）。
- `update_dragging` の context 読取は `unwrap_or(HWND::default())` / `unwrap_or(Point{0,0})`（state.rs:321-322）で**全域フォールバック**（`unwrap` ではない）。context=None も明示フォールバック（state.rs:327/330）。**panic 経路なし。**
- 配列・スライス添字 `[i]` の本番出現は境界内にゼロ（状態機械は match ベース、accumulator/context はフィールドアクセスのみ）。
- `dispatch_drag_events`（dispatch.rs）の World アクセスは `get`/`get_mut`/`get_entity_mut`/`get_resource` で全て `Option`/`Result` を返し `if let`/`let else` で安全に分岐（削除済みエンティティ・リソース不在で静かに skip）。`build_bubble_path`/`dispatch_event_for_handler` は pointer 側（W6a、W6a-V で点検済み）。**本番経路に到達可能な panic はゼロ。現状安全。**

## 適用した挙動非破壊対策（1 ファイル・3 箇所 + 特性化 2 件）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `state.rs` | `end_dragging`（CaptureGuard 抽出直後） | `debug_assert!(capture_guard.is_some(), ...)` ＋キャプチャ解放保証の不変条件コメント | debug_assert（内部不変条件） | リリースで compile-out（挙動不変）。外側 match でアクティブ状態確定済み＝抽出は必ず Some（`_ => None` 到達不能）。取り出したガードは _guard で borrow 解放後にドロップ→ReleaseCapture 実行（解放漏れなし）の保証を明文化。全 well-formed 状態で発火せず。R2.3 該当。 |
| `state.rs` | `cancel_dragging`（CaptureGuard 抽出直後） | `debug_assert!(capture_guard.is_some(), ...)` ＋同上の不変条件コメント | debug_assert（内部不変条件） | 同上（キャンセル経路でも解放漏れなし）。リリース compile-out。 |
| `state.rs` | `check_threshold`（doc コメント） | i32 乗算の桁あふれ境界（\|dx\|>46340）・本番未使用・mouse_move.rs インライン複製・P62 参照を明記する整数境界ハザードコメント | 不変条件/ハザードコメント | コメントのみ・コード挙動不変。実用座標（i16 幅由来）では桁あふれしない安全鎖と、複製/堅牢化が挙動変更で P62 行きである根拠を W4b-V/W6a-V の座標キャスト文書化方針と整合的に明文化。 |
| `state.rs`（tests） | in-source `mod tests`（check_threshold 群末尾） | 整数境界の特性化テスト 2 件 | 特性化/回帰テスト（S9 命名準拠） | 負デルタ対称性（符号非依存）と i16 極値デルタ非桁あふれの安全鎖を固定。W6b-T 未カバーの危険境界（負デルタ・極値）。 |

### 追加した特性化テスト一覧（`state.rs` in-source `mod tests`、2 件）

- `test_check_threshold_symmetric_for_negative_delta` — start_pos(100,100) から current_pos を左上へ動かし dx/dy を負にしても、`dx*dx` が正評価され距離判定が対称に働くことを固定（(97,96)→dx=-3,dy=-4,距離5=閾値ちょうど→true / (98,98)→距離≈2.83<5→false）。i32 乗算の符号非依存性を特性化。
- `test_check_threshold_i16_extent_delta_no_overflow` — i16::MAX 相当の水平デルタ（dx=32767, dy=0）で `distance_sq=1_073_676_289`（< i32::MAX）が桁あふれせず正確に評価され閾値 5 を上回って true になることを固定。本番入力範囲の上限相当（桁あふれ境界 46340 未満）の安全鎖を特性化。

いずれも既存の `force_idle()` によるスレッドローカル隔離パターン・既存命名規約（`test_<subject>_<behavior>`）に準拠。

## proposals.md へ回した候補（P62）

- **P62**: ドラッグ閾値判定 `check_threshold` の本番未使用＋インライン複製（mouse_move.rs:201-204）と距離二乗算術の i32 桁あふれ境界。kind: その他（本番未使用 pub 関数の整理＋複製統合。整数境界堅牢化を伴う場合は挙動変更を伴う脆弱性対策）。(1) 複製統合（mouse_move インライン→check_threshold 呼び出し、または check_threshold 削除）は pub API 表面の変更 or W6b↔W7a 境界跨ぎ、(2) i32→i64 昇格/saturating 化は極値判定結果が変わる挙動変更のため、いずれも本ループでは実装せず記録のみ。現行算術の安全鎖を特性化2件＋ハザードコメントで固定。

既知 proposals の再発見（重複記録なし・参照に留めた）:
- **P57**（W6a-S で解消済みの process_pointer_buffers 死関数）: `check_threshold` の「本番未使用 pub 関数＋二重実装」は P57 と同型クラス。P62 の rationale で同型である旨を参照（再記録せず）。
- **P61**（W6b-T 申告・W6b-S で prev_frame_pos 除去で解消、`CaptureGuard::is_released` は低優先で維持）: 本 V セルでも `is_released` がテスト専用アクセサであることを確認したが、解放保証の点検対象は `Drop`/`released` フラグの整合性であり `is_released`（テスト観測 API）はデッドコード整理（S 観点）の話題のため、V セルでは二重記録せず P61 参照に留めた。

## verification (S2)

- BEFORE: 親検証済みベースライン（W6b-S 直後 = 1566 passed / 0 failed・クリーンワークツリー）を信頼し全量は省略（design フェーズ0 + 親指示「BEFORE S2 は省略可」）。開始時に `cargo test -p wintf --lib drag::` で **44 passed / 0 failed** を実測（反復検証用ベースライン）。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、4.91s）。debug_assert・コメント・テスト追加のみで公開シンボル/型シグネチャ不変。
  - `cargo test --workspace` → **1568 passed / 0 failed / 32 ignored**（全 `test result` 行を awk 合算: passed 合計 1568・failed 合計 0・ignored 32、`test result: FAILED`/`error[`/`panicked` 行ゼロ）。ベースライン 1566 から **+2 = 追加した特性化テスト 2 件と一致**（削除ゼロ）。
  - 反復検証: `cargo test -p wintf --lib drag::` で **46 passed / 0 failed**（44 + 2）。`cargo test -p wintf --test drag` で **19 passed / 0 failed**（不変）。
  - git diff（境界内のみ）: `state.rs` **+57 / −0**（debug_assert 2＝リリース compile-out・check_threshold ハザードコメント 1・特性化テスト 2 件）。新規 `#[test]` 2 件・削除 0。プロダクションロジック変更なし。境界外 `state.rs` 以外の drag ソース（mod/accumulator/context/dispatch/capture_guard/systems）は **±0**（未編集）。
- 全 2 件が初回実行で合格（特性化テスト＝GREEN by construction。下記 RED 代替を参照）。debug_assert は全 well-formed 状態で発火せず（drag:: 46 件が緑のまま）、リリース挙動不変を S2 全量（1568=1566+2）で実証。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` の drag モジュール参照診断は `dispatch.rs:107/119/172/340`（collapsible_if ×4）のみで、**いずれも W6b-S が意図的に見送り記録した既存 lint**（本セル未編集の dispatch.rs）。**本セル編集ファイル（`state.rs`）を参照する診断はゼロ**。本セルの編集（debug_assert・ハザードコメント・特性化テスト追加）は**新規 clippy 警告/error を一切導入していない**。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点の担当）。

## RED フェーズ代替の検証

追加 2 件はいずれも既存の安全な i32 算術挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に i32 乗算の符号非依存性（`(-3)²=9`）と i32 範囲（i16::MAX² = 1_073_676_289 < i32::MAX = 2_147_483_647）から導出した。初回実行で 2 件とも導出どおり一致し、距離二乗算術の安全鎖（実用座標では桁あふれせず符号対称に評価）が現行実装を正確に固定していることを相互確認した。debug_assert も全 well-formed 状態構築（force_idle 起点の全遷移）で発火せず、リリース挙動不変を S2 全量で実証した。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W6b 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で failed 合計ゼロ（当該バイナリ含め全 `test result` 行が passed のみ、隔離再実行不要）。本セルの追加テストとは無関係。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダ追加なし）。本セルの変更は debug_assert 2・ハザード/不変条件コメント 1 箇所・特性化テスト 2 件のみで、新たな unsafe・スタブ・TODO/FIXME を導入していない。
- 点検は境界内 7 ファイル（+ in-source tests）を grep＋精読で網羅。キャプチャ解放漏れ（取得の二重防止・遷移時のガード move・終了/キャンセルの解放保証・Dragging 放置）を実コードで個別判定し、座標整数境界（check_threshold i32 乗算・accumulate_delta i32 加算・as f32 拡大キャスト）・panic 経路（本番 unwrap/expect/添字ゼロ）・状態不変条件（5 状態遷移・Arc<Mutex> poison 耐性）をすべて判定。挙動非破壊対策が妥当な 3 箇所（解放保証 debug_assert ×2・check_threshold ハザードコメント）と特性化価値のある整数境界 2 件を適用、挙動変更を要する 1 件（check_threshold 複製/桁あふれ堅牢化＝P62）を記録。
- **本番挙動主張の裏取り**: キャプチャ解放保証（`acquire` 唯一の呼び出し元 start_preparing・`Drop` の released ガード・終了/キャンセルの borrow 外ドロップ）、座標 provenance（i16 クライアント+ウィンドウオフセット）、check_threshold の本番未使用＋mouse_move.rs インライン複製（grep + 精読）をすべて実コードで確認。数字（1566→1568・drag:: 44→46・git diff +57/-0・新規 #[test] 2 件）はすべて実測（cargo test / git diff）で裏取り、推測なし。
- 件数整合: 1568 = 1566 + 2、drag:: 46 = 44 + 2、git diff +57/-0、新規 #[test] 2 件、debug_assert 2、proposals +6（P62 1件）。すべて相互一致。
- 境界遵守: 変更は `state.rs`（W6b 境界内）＋ `proposals.md`（提案台帳）＋本断片のみ。tasks.md 未更新・コミット未作成・`window_proc/`（呼び出し元・読み取りのみ）/`pointer/`/`window/`/他領域/`vendors/`/機能spec文書への変更なし。初期 git status に見えた W4a layout 差分は本セルの diff（HEAD 比較）に含まれず、本セルは一切触れていない。
- 結論: 本境界は脆弱性耐性が高く、warranted な挙動非破壊対策はキャプチャ解放保証 debug_assert 2 箇所＋check_threshold 整数境界ハザードコメント 1 箇所と整数境界特性化 2 件に限られた。本番経路に到達可能な panic・現実的な整数オーバーフロー・キャプチャ解放漏れ・状態不整合は不在（解放は終了/キャンセル経路と thread_local Drop が担保、二重キャプチャは start_preparing が排除、座標は実用範囲で桁あふれせず）。挙動変更を要する 1 件（check_threshold 複製/桁あふれ堅牢化）は R2.4/R2.9/R5.2 に従い P62 へ記録し、その他は現状安全と判定して churn を回避した。
