# Implementation Plan

- [x] 1. 撤去前ゲートとベースライン確認
  - design.md の File Structure Plan（削除3ファイル+7テスト+3example、編集22箇所、Preserve集合）を撤去対象の確定版として確認し、記載外のファイルを削除しないことを明示する
  - ワークツリーで `vendors/pasta` サブモジュールが未展開の場合は `git submodule update --init` を先に実行する（ハーネス worktree の既知の落とし穴）
  - 現行ブランチで release ビルド（opt-level='z'・lto=true）と既存テストスイート（unit+integration）を実行し、全て緑であることを確認・記録する（撤去後比較のベースライン）
  - areka を起動し、shell/balloon の描画とクリックスルー挙動を目視確認し、撤去後比較の基準とする
  - ベースラインのビルド・テスト結果が記録され、撤去対象一覧の確認が完了していること
  - _Requirements: 1.4, 6.4, 6.5_

- [x] 2. ULW専用描画経路の削除とスケジュール再配線
- [x] 2.1 ULW compositor / com::ulw ユーティリティ削除
  - `ecs/graphics/compositor.rs`（WindowD3D11Compositor）を削除する
  - `ecs/graphics/compositor_systems/` ディレクトリ全体（mod.rs・init.rs・render/mod.rs・render/traverse.rs・render/guards.rs）を削除する
  - `com/ulw.rs`（transfer_to_hbitmap・present_layered_window）を削除する
  - `ecs/graphics/mod.rs` の `pub mod compositor;`・`pub mod compositor_systems;` 宣言、`com/mod.rs` の `pub mod ulw;` 宣言を除去する
  - 上記ファイル・ディレクトリがワークツリーに存在しないこと（この時点では world/mod.rs が未修正のためビルドは一時的に失敗する想定＝2.2で解消）
  - _Requirements: 1.1, 1.2, 1.3_
  - _Boundary: ULW composition path (compositor, compositor_systems, com/ulw)_

- [x] 2.2 ECS スケジュール登録の再配線
  - `ecs/world/mod.rs` の `GraphicsSetup` から `compositor_init_system.after(...)` チェーンを除去し `init_window_graphics` 単独登録に整理する
  - `Composition` チェーン末尾の `composite_render_system.after(clip_sync_system)` を除去する（先行3system=visual_hierarchy_sync→visual_property_sync→clip_syncは不変のまま維持）
  - `CommitComposition` の `ulw_present_system` 登録を除去し空スケジュールにする（schedule label・`Schedule::new(CommitComposition)`・`try_run_schedule(CommitComposition)` 呼び出しは残す）
  - 該当箇所の ULW 前提コメント（256-258・334-336行）をWUC現況へ整合する
  - この時点で schedule 登録側のコンパイルエラーが解消していること（CompositionMode 系の残エラーは major 3 で追随）
  - _Requirements: 2.1, 2.2, 2.3_
  - _Depends: 2.1_
  - _Boundary: ECS world schedule (ecs/world/mod.rs)_

- [x] 2.3 systems/window_pos.rs の WindowD3D11Compositor 参照追随（design blast-radius gap 解決）
  - `use crate::ecs::graphics::compositor::WindowD3D11Compositor;`（5行）を除去する
  - `invalidate_dependent_components` の `compositor_query: Query<&mut WindowD3D11Compositor>` 引数（127行）と `for mut comp in compositor_query.iter_mut() { comp.invalidate(); }` ループ（141-143行）を除去する（WucGraphicsResource・BitmapSourceGraphics の無効化は維持）
  - docstring（113-114行）の WindowD3D11Compositor 言及を WUC 現況へ整合する
  - **design の Preserve 集合は `systems/window_pos.rs` を「絶対不変」に列挙するが、同ファイルは削除済み `WindowD3D11Compositor` を実参照しており、Req1.5「ULW 専用シンボル残存参照ゼロ」と矛盾。要件優先で本追随を実施（File Structure Plan の盲点＝2件目の blast-radius gap・composition_mode gap に続く）。編集は ULW compositor 参照除去のみに限定、WUC/BitmapSource ロジックは不変**
  - `window_pos.rs` に `WindowD3D11Compositor` 参照が無く、WUC/BitmapSource 無効化挙動が保たれていること
  - _Requirements: 1.5, 6.2_
  - _Depends: 2.1_
  - _Boundary: ecs/graphics/systems/window_pos.rs (ULW compositor 参照除去のみ)_

- [ ] 3. CompositionMode collapse と全 production 参照の追随
- [x] 3.1 CompositionMode enum・Window フィールド・再エクスポート撤去
  - `ecs/window/components.rs` から `CompositionMode` enum 定義・`Window.composition_mode` フィールド・`Window::composition_mode()` メソッド・`Window::default` の `composition_mode` 初期化を削除する
  - `ecs/mod.rs` の `pub use window::{...}` から `CompositionMode` 再エクスポート（47行）を除去する
  - `WindowStyle::default().ex_style` は `WS_EX_LAYERED` のまま据え置く（D4・変更しない）
  - unsafe impl Send/Sync 安全性コメント、および enum 定義部・フィールド部・WindowStyle既定値部（101-107・121-122・148-149・168-176行）の ULW 前提コメントを整合し、in-source `#[cfg(test)]` の ULW 既定 assert を新既定（WUC固定）へ追随する
  - `components.rs`・`ecs/mod.rs` に `CompositionMode` シンボルが存在しないこと
  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends: 2.2_
  - _Boundary: ECS window components + ecs/mod.rs re-export_

- [x] 3.2 (P) compute_ex_style の branchless 一本化
  - `runtime/window_factory.rs::compute_ex_style()` を合成モード引数なしの `fn compute_ex_style(style: &WindowStyle) -> WINDOW_EX_STYLE` に改め、`(style.ex_style & !WS_EX_LAYERED) | WS_EX_NOREDIRECTIONBITMAP` を無条件に返すようにする
  - 呼び出し側 `EcsWindowFactory::create_window` から `composition_mode` の読み取り・引数渡しを除去する
  - in-source test の ULW ケース（`ex_style_ulw_keeps_layered`）を削除し、DComp ケース（`ex_style_dcomp_*`）を唯一経路の回帰検知として残置する
  - docstring（17-19・57-63行）を整合する
  - `compute_ex_style` が引数1つ（style）のみを取り、生成時に `WS_EX_LAYERED` を付与しないことが in-source テストで確認できること
  - _Requirements: 4.1, 4.2, 4.3_
  - _Depends: 3.1_
  - _Boundary: Runtime window factory (runtime/window_factory.rs)_

- [x] 3.3 (P) WM_PAINT ハンドラの DComp 一本化
  - `ecs/window_proc/lifecycle.rs::WM_PAINT` の `composition_mode()` 照会と ULW フォールバック分岐（BeginPaint/EndPaint）を除去し、常に `DefWindowProcW` へ委譲（None返却）する無条件一本化に改める
  - `WM_ERASEBKGND` コメント（22-24行）・`WM_PAINT` docstring（36-39行）を整合する
  - `WM_PAINT` ハンドラが `composition_mode()` を参照せず、常に `None` を返す単一経路になっていることがコードで確認できること
  - _Requirements: 3.4_
  - _Depends: 3.1_
  - _Boundary: window_proc lifecycle (ecs/window_proc/lifecycle.rs)_

- [ ] 3.4 (P) visual.rs の mode ゲート無条件化
  - `find_owner_window_composition_mode`（39-66行）を owner Window 存在判定ヘルパー（DeferredWorld 版・ChildOf 走査・W3b-V 間接巡回 NOTE 維持）へ縮退する
  - `on_visual_add` の `is_dcomp_mode` ゲート（83-90行）を「owner Window が存在する場合」判定へ置換する（orphan Visual への graphics コンポーネント非挿入挙動は不変）
  - `use crate::ecs::window::CompositionMode`（13行）・フック docstring（71行）を整合する
  - visual.rs に `CompositionMode` 参照が無く、orphan Visual 除外挙動が保たれていること
  - _Requirements: 3.4, 6.2_
  - _Depends: 3.1_
  - _Boundary: ecs/graphics/visual.rs_

- [ ] 3.5 (P) init_window_graphics の mode フィルタ除去（Preserve 例外・参照追随のみ）
  - `use ... CompositionMode`（206行）を除去し、`has_dcomp_windows` 判定（212-219行）を query 空チェックへ縮退する
  - per-window `continue` フィルタ（255-258行）を除去し、docstring（185・187行）を整合する
  - レンダリングロジック・`WucGraphicsResource` 遅延初期化・early-return 構造は変更しない（diff を mode フィルタと docstring に限定する）
  - systems/init.rs に `CompositionMode` 参照が無く、mode フィルタ以外の差分が無いこと
  - _Requirements: 3.4, 6.2_
  - _Depends: 3.1_
  - _Boundary: ecs/graphics/systems/init.rs (mode フィルタのみ)_

- [ ] 3.6 wintf 内 test-only 追随と lib ビルド確認
  - `ecs/clickthrough/controller.rs` の in-source test ヘルパ `spawn_live_window`（922-940行付近）の `CompositionMode::DComp` 指定を新API（フィールド指定なし）へ追随する（production コードは変更しない）
  - `runtime/mod.rs` の in-source test `close_to_reconcile_to_shutdown_chain_wakes_listener` の import（425行）・フィールド指定（456行）を追随する
  - 3.1〜3.6 完了時点で `cargo build -p wintf` および lib の in-source テストがコンパイル通過すること
  - _Requirements: 3.4_
  - _Depends: 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 4. areka・examples・外部テストの追随
- [ ] 4.1 (P) areka crate の追随
  - `crates/areka/src/main.rs` の `composition_mode: CompositionMode::DComp`（225・292行）・`use ... CompositionMode`（29行）を除去し、220-231行の `composition_mode` 前提コメントを WUC 固定の現況へ整合する
  - `crates/areka/src/tests.rs` の `assert_eq!(window.composition_mode(), CompositionMode::DComp)`（108・118行）の2テストを削除または WUC 固定の別観測へ書き換える
  - `crates/areka/examples/clickthrough_two_rects.rs` の `composition_mode: CompositionMode::DComp`（131行）・import（48行）を除去する
  - areka crate（lib・bin・examples・tests）がビルドを通過し、tests.rs のテストが緑であること
  - _Requirements: 3.5, 7.2_
  - _Depends: 3.6_
  - _Boundary: areka crate_

- [ ] 4.2 (P) wintf examples の追随
  - `ulw_twin_demo.rs`・`ulw_debug_demo.rs`・`multi_backend_demo.rs` を削除する（ULW 主題の消滅・D5）
  - `clip_demo.rs` の `create_ulw_clip_window`（87・262・282行）を除去し、clip 検証を DComp 単独へ書き換える
  - `dcomp_demo.rs`・`dcomp_taffy_demo.rs` の `composition_mode:` フィールド指定と `use ... CompositionMode` を除去する
  - `postmessage_click_test.rs` の ULW present 言及コメントを整合する（既定生成のリテラルは変更不要）
  - 残置 examples が `cargo build --examples -p wintf` でビルドを通過すること
  - _Requirements: 3.4_
  - _Depends: 3.6_
  - _Boundary: wintf examples_

- [ ] 4.3 (P) wintf 外部テストの削除・書き換え・追随
  - `tests/graphics/compositor_init_system_test.rs`・`compositor_integration_test.rs`・`compositor_lifecycle_test.rs`・`compositor_opacity_test.rs`・`compositor_render_system_test.rs`・`compositor_transfer_test.rs` の6本を削除し、`tests/graphics.rs` の該当 `#[path]` 宣言（12-23行）を除去する（D6）
  - `tests/window/composition_mode_test.rs` を削除し、`tests/window.rs` の該当宣言（2-3行）を除去する（D8）
  - `tests/window/find_owner_composition_mode_test.rs` を owner Window 存在判定（自身が Window／ChildOf 祖先に Window／orphan の3ケース）の検証へ書き換え、ファイル名・`tests/window.rs` の mod 宣言（4-5行）を新ヘルパー名へ追随する（D8改訂・W3b-V 経路カバレッジ維持）
  - `tests/graphics/dcomp_integration_test.rs`・`init_window_graphics_test.rs` の `CompositionMode` 参照を追随する（WUC 側本体ロジックは不変）
  - `cargo test -p wintf --no-run` で全テストターゲットがコンパイル通過すること
  - _Requirements: 3.4_
  - _Depends: 3.6_
  - _Boundary: wintf tests_

- [ ] 5. ドキュメント整合とビルド/回帰検証
- [ ] 5.1 ドキュメントと残余コメントの最終整合確認
  - `doc/COMPAT_ARCHITECTURE.md`（44・99・105・108行）の ULW 残存前提記述を GPU 合成単独へ整合する（108行「非スコープ(残置):ULWアーム…除去は別spec」を「除去済み」へ）
  - grep で wintf/areka コード内コメントに ULW を残存機構として前提する記述が残っていないことを横断確認する
  - steering（tech.md・product.md・roadmap.md）を変更していないことを確認する
  - ULW 残存前提記述の grep 結果が0件であり、doc/COMPAT_ARCHITECTURE.md が現況に整合していること
  - _Requirements: 7.1, 7.2, 7.3_
  - _Depends: 4.1, 4.2, 4.3_

- [ ] 5.2 残存シンボル grep とリリースビルド検証
  - `WindowD3D11Compositor`・`UpdateLayeredWindow`・`compositor_init_system`・`composite_render_system`・`ulw_present_system`・`transfer_to_hbitmap`・`present_layered_window`・`CompositionMode`・`composition_mode`・`find_owner_window_composition_mode` が wintf/areka crate から一切消えていることを grep で確認する
  - release ビルド（`opt-level='z'`・`lto=true`）が撤去後に通過することを確認する
  - 残存シンボル grep が0件であり release ビルドが成功終了すること
  - _Requirements: 1.5, 6.4_
  - _Depends: 4.1, 4.2, 4.3_

- [ ] 5.3 既存テスト回帰確認
  - `tick_order_tests`（`EXPECTED_ORDER` 13本固定列・件数assert）が無改変で緑であることを確認する（Req2.3の受入）
  - `dcomp_integration_test`・`init_window_graphics_test`・`surface_pixel_equivalence_test` 等 WUC 側既存テストの緑維持を確認する
  - areka `tests.rs`・書き換え後の owner 存在判定テスト・`compute_ex_style` の DComp in-source テストが緑であることを確認する
  - `cargo test --workspace` が全て成功すること
  - _Requirements: 2.3, 6.1, 6.2, 6.3_
  - _Depends: 5.2_

- [ ] 5.4 起動目視サニティとクリックスルー回帰確認
  - areka を起動し、shell/balloon 窓が撤去前ベースライン（タスク1）と同一の描画で表示されることを目視確認する
  - クリックスルー登録窓で透明ピクセル上のクリックが別プロセスへ透過し続けることを確認する（クリックスルー機構のコードが非改変であることも diff で確認する）
  - 起動サニティの結果（描画同一・クリックスルー機能維持）が確認済みであること
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 6.1, 6.5_
  - _Depends: 5.3_

## Implementation Notes

- **ベースライン（タスク1・2026-07-04）**: `git submodule update --init vendors/pasta` 実行済（worktree 未展開の落とし穴）。debug `cargo build --workspace` 緑。`cargo test -p wintf -p areka` 全緑（wintf lib 542／com 61／drag 19／ecs 102／graphics 146／layout 170／visual 52／widget 16／win_app 2／window 30／clickthrough 9+31ignored、areka bin 63）。**既知の無関係失敗**: `cargo test --workspace` は `shiori-host32-helper` に2件の既存失敗あり（`testdll_drop_invokes_courtesy_unload`・`loopback_hello_request_echo_and_bounded_loop`）＝32bit SHIORI ヘルパーで本 spec スコープ外。最終検証（5.3）では wintf/areka の緑維持で判定し、この2件はベースライン既存として扱う。
- **cargo は PowerShell で実行**（Git Bash の GNU coreutils `link.exe` が MSVC link を遮蔽する既知の罠）。
- **中間タスクはビルド非通過が設計想定**: 2.1（world/mod.rs 未修正でビルド一時失敗）・3.1〜3.5（CompositionMode 撤去の追随途中）。コンパイル通過ゲートは 2.2（schedule 側）・3.6（wintf lib）・4.1〜4.3（areka/examples/tests）。
- **design blast-radius gap（2.3 で解決・2026-07-04 実装時発見）**: design の Preserve 集合が「絶対不変」に列挙した `systems/window_pos.rs` が、削除済み `WindowD3D11Compositor` を実参照（import・`invalidate_dependent_components` の query・invalidate ループ）。Req1.5「ULW 専用シンボル残存参照ゼロ」＋3.6 ビルド通過ゲートが要件優先ゆえ、ULW compositor 参照除去のみの追随を task 2.3 として追加。File Structure Plan の編集リスト外（composition_mode gap に続く2件目の盲点）。WUC/BitmapSource 無効化ロジックは不変。
