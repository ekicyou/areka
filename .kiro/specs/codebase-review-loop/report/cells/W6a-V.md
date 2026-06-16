# W6a-V: wintf ポインター入力 × 脆弱性レビュー

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- 観点: V（脆弱性レビュー）。基準: design.md「Security Considerations」（unsafe 境界・整数変換の切り捨て/オーバーフロー・外部入力の検証欠如・panic 経路による DoS を点検し、**挙動を変えない範囲（内部チェック・debug_assert・不変条件コメント・安全な型置換）の対策のみ投入**。API シグネチャ/エラー応答を変える対策・入力検証厳格化・バッファ上限導入は proposals.md へ）。CellExecutor V 規則（R2.3/R2.4）、観点順序 T→S→V（R2.7、W6a-T/W6a-S 完了済みの回帰検知器上で実行）。
- requirements（source 番号）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations、CellExecutor 観点別規則（V）、提案記録様式、セル断片様式。領域定義「W6a: wintf ポインター入力 / `crates/wintf/src/ecs/pointer/`」。
- 参考: `report/cells/W6a-T.md`（モジュール×テスト対応表・P57 根拠）、`W6a-S.md`（デッドコード除去内容・net -302 LOC・P57/P58）、`W4b-V.md`・`W5b-V.md`（V 観点・整数境界・panic 点検・debug_assert 様式の先例）。
- 境界: `crates/wintf/src/ecs/pointer/`（tests/ の該当ドメインは境界内）。`ecs/drag/`（W6b）・`window_proc/`（バッファ投入元、境界外）には変更を加えていない（読み取りのみ＝入力境界条件の把握と本番経路の裏取りのため）。
- 起点: W6a-S 適用後のクリーンなワークツリー、ベースライン S2 = 1514 passed / 0 failed。反復検証用 pointer:: ベースライン = 54 passed / 0 failed（開始時実測）。

## 点検手法

境界内 6 ファイル（mod.rs / types.rs / buffers.rs / systems.rs / nchittest_cache.rs / dispatch/mod.rs + in-source tests）を grep（複数パターン）＋全文精読で走査した。

- panic 経路: `unwrap()` / `expect(` / `panic!` / `unreachable!` / `unimplemented!` / `todo!` / 配列・スライス添字 `[i]`/`[idx]`
- 整数境界: `as i32` / `as f32` / `as isize` / `as usize`（切り捨て・符号反転・飽和）・整数乗算/減算（`len() - 2` 等）のオーバーフロー/アンダーフロー・ゼロ除算・`saturating_`/`checked_`
- バッファ枯渇: thread_local バッファ（`POINTER_BUFFERS`/`BUTTON_BUFFERS`/`WHEEL_BUFFERS`/`DOUBLE_CLICK_BUFFERS`/`MODIFIER_STATE`）の容量・上限・エントリ寿命（`.remove`/`.retain`/`.clear` の有無）・無制限蓄積（メモリ枯渇 DoS）
- 入力境界条件: 本番経路 `transfer_buffers_to_world` を重点追跡。座標範囲・ボタン状態の不整合・修飾キー・ホイールデルタの極値が World へ反映されるまでの値域を、投入元（`window_proc/mouse_move.rs`・`mouse_dblclick_wheel.rs`・`mouse_click.rs`、いずれも境界外）まで遡って確認

本番経路の裏取り（過去セルの未確認主張による REJECTED を回避）:
- `transfer_buffers_to_world` の本番駆動を実コードで確認: `world/mod.rs:458`（`try_tick_world` 冒頭、`try_run_schedule(Input)` の**前**＝WndProc スレッド上で同期呼び出し）。`Input` スケジュールに登録されるのは `dispatch_pointer_events`（world/mod.rs:108-112）等で、削除済み `process_pointer_buffers` は不在（world/mod.rs:114-116 のコメントと整合）。
- ホイール/ダブルクリックの消費不在を実コードで確認（後述所見2）。`process_pointer_buffers` が W6a-S 以前からデッドだった点を、W6a-T 起点 commit 6e7e1ea の `world/mod.rs` を git show + grep して「`add_systems(... process_pointer_buffers ...)` ゼロ」を確認（W6a-S が回帰を導入していないことの裏取り）。
- 座標の入力値域を確認: `push_pointer_sample(x as f32, y as f32, ...)`（mouse_move.rs:346/399）の x/y は `(lparam & 0xFFFF) as i16 as i32`（mouse_move.rs:110-111）由来で **i16 範囲 [-32768, 32767]**。ホイールデルタは `((wparam >> 16) & 0xFFFF) as i16`（mouse_dblclick_wheel.rs:199/217）で **i16**。

## 発見した脆弱性候補と判定

### 1. panic 経路 — 本番経路に到達可能な panic はゼロ。1 箇所に挙動非破壊の不変条件 debug_assert＋コメントを適用

境界内の `unwrap()`/`expect()`/`panic!`/`unreachable!` の本番（非 `#[cfg(test)]`）出現を個別判定した。`systems.rs:160`・`buffers.rs:259+`・`types.rs:405/414/644/652/699`・`nchittest_cache.rs:285` はすべて `#[cfg(test)]` 内のため対象外。

- **`types.rs:257` `self.samples.back().unwrap()` ＋ `:258` `&self.samples[self.samples.len() - 2]`（`PointerBuffer::calculate_velocity`、本番ホットパス）** — 直前の早期 return（types.rs:254、`if self.samples.len() < 2 { return (0.0, 0.0); }`）により到達時点で `len() >= 2` が保証される。よって (a) `back()` は必ず `Some`（unwrap 安全）、(b) usize 添字 `len() - 2` はアンダーフローせず（len>=2）かつ範囲内（< len）。**本番経路で唯一の「スライス添字＋usize 減算」**であり、ガードが `< 1` に弱められると `len()-2` が usize アンダーフロー panic に至る危険境界のため、→ **挙動非破壊の `debug_assert!(len() >= 2)` ＋不変条件コメントを適用**（下記）。リリースでは compile-out（挙動不変）、デバッグでも全 well-formed 入力で発火せず。本関数は `transfer_buffers_to_world`（buffers.rs:136）から本番駆動される。
- **`nchittest_cache.rs` のキャスト群** — `hwnd.0 as isize`（ポインタ→isize、HashMap キー化のみ）、`HTCLIENT/HTTRANSPARENT as isize`（小定数）、`pt.x/pt.y as f32`（i32→f32、ScreenToClient 後のクライアント座標は実用範囲で無損失）。添字・unwrap・除算なし。**現状安全（対策不要）。** `cached_nchittest` は実 HWND/COM/World 依存でユニット到達不能（W6a-T 所見2 と同じ環境制約）。
- **`dispatch/mod.rs` のエンティティ存在チェック** — `dispatch_event_for_handler`（mod.rs:165/181）は path 内の各エンティティで `world.get_entity(entity).is_err()` を確認し削除済みなら静かに return。**現状安全**（W6a-T `test_dispatch_event_for_handler_guards_deleted_entity` で特性化済み）。`build_bubble_path`（mod.rs:114-137）の ChildOf 走査は Window 停止のみで巡回ガードは持たないが、ChildOf の巡回は bevy hierarchy 側の不変条件（祖先巡回の不在）に依存する設計で、本ファイル内に添字/unwrap なし。巡回ガード欠如は別領域の既知 P48（visual_hierarchy の ChildOf 祖先走査）と同型クラスだが、当該 P48 は visual ドメインの所見であり、pointer の build_bubble_path への適用拡張は挙動・構造変更を要するため本 V セルでは現状安全（ChildOf 不変条件に依拠）と判定し churn 回避（二重記録せず P48 参照に留める）。

### 2. バッファ枯渇 / 入力反映経路 — ホイール・ダブルクリック thread_local が消費されない（P59）／HashMap キー単調増加（P60、現状安全）

本番経路 `transfer_buffers_to_world`（buffers.rs:127-225）と投入ヘルパを精読し、thread_local バッファの容量・寿命・消費を点検した。

- **個別エントリのメモリは定数上限** — `PointerBuffer` のサンプル列は `push` が `MAX_SAMPLES=5` 超過で pop_front（types.rs:225-230）＝上限 5。`ButtonBuffer`/`WheelBuffer` は単一構造体で `saturating_add`（types.rs:326-333、i16 飽和で panic/ラップなし。W6a-T `test_wheel_buffer_saturates_at_i16_bounds` で特性化済み）。**1 エントリあたりは有界（枯渇しない）。**
- **`WHEEL_BUFFERS` / `DOUBLE_CLICK_BUFFERS` は本番で消費されない（→ P59）** — `transfer_buffers_to_world` は POINTER（位置/速度）・BUTTON（エッジ検出＋reset）・MODIFIER のみ消費し、**WHEEL_BUFFERS と DOUBLE_CLICK_BUFFERS を一切読まない**。本番でこれらを読んでいたのは削除済み `process_pointer_buffers`（W6a-S/P57）で、削除前から `add_systems` 登録ゼロでデッドだった（commit 6e7e1ea の world/mod.rs を git show + grep で確認）。すなわち W6a-S は回帰を導入しておらず、**W6a-S 以前からの潜在ギャップ**である。結果: (a) **マウスホイール入力は PointerState/OnPointer ハンドラに届かない**（`pointer_state.wheel` を非既定値へ書く本番経路がワークスペース全 grep でゼロ。`systems.rs:25` はリセット・:90-94 はデバッグ読み取りのみ）＝ write-only デッドストレージ兼機能ギャップ。(b) `DOUBLE_CLICK_BUFFERS` は書き込み箇所もワークスペース全域でゼロ（ダブルクリックは `mouse_dblclick_wheel.rs:83-111` が直接 component へ書く経路で機能）＝純粋なデッドストレージ。**脆弱性ではない**が、(a) のホイール反映を繋ぐのは挙動変更（ホイールが PointerState に到達し始める）、(b) の削除は S 観点領分のため、いずれも本 V セルでは適用せず **P59** へ記録（R2.4/R5.2）。
- **thread_local の HashMap キーがエンティティ単位で単調増加（→ P60、現状安全で対策不要）** — 全 5 マップ（buffers.rs:20-35）は `entry().or_insert*` で生成するが、**本番に `.remove()`/`.retain()` が存在しない**（grep 実証。`transfer_buffers_to_world` は `PointerBuffer::clear()`／`ButtonBuffer::reset()` で内容を空化するのみでキーは残置。全 wipe はテスト helper のみ）。よってポインター入力を受けた**distinct Entity ごとに 1 エントリが永久蓄積**し、despawn/leave でも除去されない。ただし増加し得るのは**キー数のみ**で、上限は「ポインター入力を受けた distinct Entity 数」＝ bevy の世代付きインデックス再利用により**生存スロット数（UI 要素数）で実質有界**、**イベント発生量（マウス移動回数）には比例しない**。したがって現実的なメモリ枯渇 DoS には至らず（増加量は UI 要素数オーダーで微小・自己抑制的）、**現状は安全（対策不要）**と判定。厳密には despawn 済み世代スロットが再利用されるまで stale キーが残る理論的リークのため、除去対策（leave/despawn フック連動・空エントリ retain）の方針を **P60** へ記録（ButtonBuffer のエッジ検出契約と相互作用するため挙動非破壊性の厳密検証が必要＝挙動変更扱い）。

### 3. 入力イベントの境界条件 — 座標極値・ボタン/修飾キー状態は現状安全。座標極値を特性化テストで固定

- **座標範囲（極値・負値・画面外）** — `transfer_buffers_to_world` の `sample.x as i32`（buffers.rs:142）は f32→i32 の**飽和キャスト**（NaN→0・範囲外→i32::MIN/MAX・UB/panic なし）。本番では座標は i16 範囲（前述）のため i32 への切り戻しは無損失（切り捨て/オーバーフローなし）。極値・非有限値が混入しても飽和で吸収されパニック経路にならない。→ **不変条件コメントを追記**（buffers.rs:142、挙動不変）＋ **i16 極値座標と非有限座標の特性化テスト 2 件を追加**（下記、安全鎖の固定）。`mouse_move.rs` 側のクライアント領域ガード（rect 範囲外は DefWindowProcW 委譲）は境界外だが、画面外座標が pointer 経路へ流入しない一次防御として確認した。
- **ボタン状態の不整合（down without up・同時押し・状態機械）** — `ButtonBuffer` は `down_received`/`up_received` の独立 bool で、`transfer_buffers_to_world` は down→true / up→false / どちらもなし→既存維持（エッジ検出）。**down と up が同一 tick に両方立つ場合は down 優先**（buffers.rs:168 の `if buf.down_received` が先、`else if buf.up_received`）で、これは現行の確定的挙動（不整合でパニック・未定義動作なし）。全 5 ボタン独立写像も含め W6a-T の buffers 特性化 9 件（エッジ検出/全ボタン/reset）で固定済み。**現状安全（対策不要）。**
- **修飾キー状態・ホイールデルタ極値** — 修飾キーは `set_modifier_state` が最新値で上書き（累積でない、W6a-T `test_set_modifier_state_overwrites_latest`）。ホイールデルタ i16 極値は saturating_add で飽和（前述、ただし本番未消費＝P59）。**現状安全。**

### 4. mod.rs / types.rs（hit_test スタブ）/ dispatch のその他 — 現状安全（対策不要）

- **mod.rs**: re-export のみ。対象なし。
- **types.rs の hit_test プレースホルダ**（:355-376）: 常に window_entity / スクリーン座標素通しを返す Phase 1 スタブ。添字・キャスト・unwrap なし。W6a-T で 2 件特性化済み。**現状安全。**
- **dispatch_pointer_events**（mod.rs:209-253）: PointerState 収集（Clone デタッチ）→ Pressed ゲート（left||right||middle）→ post-dispatch クリア。排他システムで添字・unwrap なし。W6a-T/既存 dispatch テストで網羅。**現状安全。**

## 適用した挙動非破壊対策（2 ファイル・2 箇所 + 特性化 2 件）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `types.rs` | `PointerBuffer::calculate_velocity`（:254 ガード直後） | `debug_assert!(self.samples.len() >= 2, ...)` ＋不変条件コメント | debug_assert（内部不変条件） | リリースで compile-out（挙動不変）。`back().unwrap()` と添字 `len()-2`（usize 減算）の安全根拠（直上の早期 return が `len()>=2` を保証）を明文化。全 well-formed 入力で発火せず。R2.3「挙動を変えない内部チェック」に該当。本番ホットパス（transfer 経由）唯一の添字＋usize 減算の文書化。 |
| `buffers.rs` | `transfer_buffers_to_world`（:142 座標反映直前） | `f32 as i32` 飽和キャストの安全鎖と i16 範囲 provenance を明記する不変条件コメント | 不変条件コメント | コメントのみ・コード挙動不変。本番座標が WM lparam の i16 範囲由来で i32 切り戻しが無損失なこと・非有限/範囲外でも飽和でパニック回避することを W4b-V の座標キャスト文書化方針と整合的に明文化。 |
| `buffers.rs`（tests） | in-source `mod tests` 末尾 | 入力境界（座標極値・非有限）の特性化テスト 2 件 | 特性化/回帰テスト（S9 命名準拠） | i16 極値座標が無損失反映されること・非有限座標が飽和（NaN→0・+inf→i32::MAX）しパニックしない安全鎖を固定。W6a-T 未カバーの危険境界。 |

### 追加した特性化テスト一覧（`buffers.rs` in-source `mod tests`、2 件）

- `test_transfer_buffers_to_world_i16_extreme_coords_are_exact` — i16::MIN/MAX 座標（本番入力範囲の極値）を transfer に通し、f32→i32 切り戻しが無損失で正確（client/local とも `(-32768, 32767)`）・パニックなしを固定。
- `test_transfer_buffers_to_world_nonfinite_coords_saturate_without_panic` — 防御的特性化。最新サンプルが NaN/+inf でも transfer がパニックせず、`f32 as i32` 飽和仕様どおり NaN→0・+inf→i32::MAX に縮退することを実測で固定（速度計算分岐も 2 サンプルで通過）。

いずれも既存の `reset_all_buffers()` によるスレッドローカル隔離パターン・既存命名規約（`test_<subject>_<behavior>`）に準拠。

## proposals.md へ回した候補（P59〜）

- **P59**: `WHEEL_BUFFERS`/`DOUBLE_CLICK_BUFFERS` が本番経路で消費されない thread_local（ホイール入力が PointerState に未反映の潜在ギャップ＋デッドストレージ）。kind: その他。ホイール反映を繋ぐのは外部観測可能な挙動変更のため記録のみ（デッドストレージ削除は S 観点領分）。W6a-S 以前からの潜在ギャップであることを commit 6e7e1ea の実コードで裏取り済み（W6a-S は回帰を導入していない）。
- **P60**: ポインター thread_local バッファの HashMap キーがエンティティ単位で単調増加（despawn/leave 時の除去なし）。kind: 挙動変更を伴う脆弱性対策。現状はキー数が UI 要素数で実質有界・イベント量非比例で**現状安全**だが、理論的 stale キー残置の除去対策は ButtonBuffer エッジ検出契約と相互作用する挙動変更のため記録のみ。

既知 proposals の再発見（重複記録なし・参照に留めた）:
- **P57**（W6a-S で解消済みの `process_pointer_buffers` 削除）: P59 の根拠（ホイール/DC 消費者が削除済みデッド関数だった事実）として参照。再記録せず。
- **P58**（`transfer_buffers_to_world` のボタン match 重複 DRY 整理）: 本セルでも当該 match 重複を確認したが挙動非破壊の純可読性整理（S 観点）であり V セルの所掌外。再記録せず参照に留めた。
- **P48**（visual_hierarchy の ChildOf 祖先走査の巡回ガード欠如）: `build_bubble_path` の ChildOf 走査も巡回ガードを持たない同型クラスだが、pointer 側は ChildOf 不変条件に依拠し現状安全と判定、適用拡張は挙動/構造変更のため二重記録せず参照に留めた。

## verification (S2)

- BEFORE: 親検証済みベースライン（W6a-S 直後 = 1514 passed / 0 failed・クリーンワークツリー）を信頼し全量は省略（design フェーズ0 + 親指示「BEFORE S2 は省略可」）。開始時に `cargo test -p wintf --lib pointer::` で **54 passed / 0 failed** を実測（反復検証用ベースライン）。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、5.87s）。削除なし・追加のみのため公開シンボル消失なし。
  - `cargo test --workspace` → **1516 passed / 0 failed / 32 ignored**（全 `test result` 行を awk 合算: passed 合計 1516・failed 合計 0・ignored 32）。ベースライン 1514 から **+2 = 追加した特性化テスト 2 件と一致**（削除ゼロ）。
  - 反復検証: `--lib pointer::` で **56 passed / 0 failed**（54 + 2）。内訳 `pointer::types` 22・`pointer::buffers` **11（9 + 2）**・`pointer::systems` 3・`pointer::nchittest_cache` 6・`pointer::dispatch` 14。wintf lib バイナリ全体: AFTER **311 passed / 0 failed**（W6a-S の 309 + 2）。
  - git diff（境界内のみ）: **67 insertions / 0 deletions**、2 ファイル（`types.rs` +9・`buffers.rs` +58）。新規 `#[test]` 2 件・削除 0。プロダクションロジック変更なし（debug_assert 1＝リリース compile-out・コメント 2・テスト 2 のみ）。
- 全 2 件が初回実行で合格（特性化テスト＝GREEN by construction。下記 RED 代替を参照）。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` の総数 174 警告/error 行。**本セル編集ファイル（`ecs/pointer/buffers.rs`・`types.rs`）を参照する診断はゼロ**（当該パスでの grep ヒットなし）。本セルの編集（debug_assert・不変条件コメント・特性化テスト追加）は**新規 clippy 警告/error を一切導入していない**。
- ポインターモジュール配下の既存 lint は `nchittest_cache.rs:60` の collapsible_if 1 件のみ（W6a-S 記録済み・本セル未編集・対象集合外）。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点の担当）。

## RED フェーズ代替の検証

追加 2 件はいずれも既存の安全な飽和挙動／無損失反映の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に Rust の `f32 as i32` 飽和キャスト仕様（NaN→0・+inf→i32::MAX・範囲超過→飽和）と本番座標の i16 範囲 provenance から導出した。初回実行で 2 件とも導出どおり一致し（特に非有限テストで NaN→0・+inf→i32::MAX を実測確認）、飽和キャストの安全鎖が現行実装を正確に固定していることを相互確認した。debug_assert も全 well-formed 構築で発火せず（pointer:: 56 件が緑のまま）、リリース挙動不変を S2 全量（1516=1514+2）で実証した。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W6a 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` バイナリは **79 passed / 0 failed** と合格（隔離再実行不要）。本セルの追加テストとは無関係。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダ追加なし）。本セルの変更は debug_assert 1・不変条件コメント 2 箇所・特性化テスト 2 件のみで、新たな unsafe・スタブ・TODO/FIXME を導入していない。
- 点検は境界内 6 ファイル（+ in-source tests）を grep＋精読で網羅。panic 経路（本番の添字＋usize 減算 1 箇所を含む全 unwrap/添字）を個別に到達可能性判定し、整数変換（f32→i32 飽和）・バッファ枯渇（個別有界＋キー単調増加）・入力境界（座標極値・ボタン/修飾キー不整合）をすべて判定。挙動非破壊対策が妥当な 2 箇所（velocity 不変条件 debug_assert＋座標キャスト根拠コメント）と特性化価値のある座標極値 2 件を適用、挙動変更を要する 2 件（ホイール未反映＝P59、HashMap キー増加＝P60）を記録。
- **本番挙動主張の裏取り**: `transfer_buffers_to_world` のスケジュール登録・スレッドモデル（world/mod.rs:458・try_run_schedule(Input) の前）、ホイール/DC 消費不在、`process_pointer_buffers` が W6a-S 以前からデッドだった事実（commit 6e7e1ea の git show + grep）をすべて実コードで確認。数字（1514→1516・pointer 54→56・buffers 9→11・lib 309→311・diff 67/0）はすべて実測（cargo test / git diff）で裏取り、推測なし。
- 件数整合: 1516 = 1514 + 2、pointer:: 56 = 54 + 2、buffers 11 = 9 + 2、git diff +67/-0、新規 #[test] 2 件。すべて相互一致。
- 境界遵守: 変更は `types.rs`・`buffers.rs`（いずれも W6a 境界内）＋ `proposals.md`（提案台帳）＋本断片のみ。tasks.md 未更新・コミット未作成・`window_proc/`（投入元・読み取りのみ）/`drag/`/他領域/`vendors/`/機能spec文書への変更なし。
- 結論: 本境界は脆弱性耐性が高く、warranted な挙動非破壊対策は velocity 不変条件 debug_assert＋座標キャスト根拠コメントの 2 箇所と座標極値特性化 2 件に限られた。本番経路に到達可能な panic・整数オーバーフロー・現実的なバッファ枯渇 DoS は不在。挙動変更を要する 2 件（ホイール入力の PointerState 未反映ギャップ、thread_local キーの単調増加）は R2.4/R5.2 に従い P59/P60 へ記録し、その他は現状安全と判定して churn を回避した。
