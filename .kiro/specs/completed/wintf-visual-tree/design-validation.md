# 設計レビュー: wintf-visual-tree

- 対象: `design.md`（2026-11-06 生成）・`requirements.md`（承認済み）・`research.md` 9〜12 節
- 照合したコード: `ecs/graphics/{visual,clip}.rs`・`ecs/graphics/systems/{render,surface,window_pos,clip_sync,visual_sync}.rs`・`ecs/world/mod.rs`・`ecs/common/ancestors.rs`・`ecs/visual/`（既存の作りかけ）・`numerics/`・`ecs/layout/systems/monitor_systems_transition_tests.rs`
- 実行形: 非対話（cargo は回していない・設計書は直していない）

## Review Summary

案 C（木の層に係を置き、今の道には小さな口だけ足す）は既存の schedule の列・変化の印の作法・装置の失いの道・`format_entity_name` の記録の作法に素直に乗っており、要件 1〜7 はすべて Traceability の表で部品に落ちている。棚卸㉟の約束 2 つ（`Schedule::new(Update)` の行を動かさない・`ecs/mod.rs` は `pub mod visual;` を外すだけ・zorder のファイルは触らない）も守られている。ただし `update_draw_trees` の `Query` の署名が bevy の借用規則で衝突する点、装置の失いで節が持ち主へ化ける点、組み直しの条件が `Arrangement` の変化を見ていない点の 3 つは、設計討議で署名と条件を直してから実装へ進むのがよい。

## Critical Issues

### 🔴 Critical Issue 1: `update_draw_trees` の 2 つの `Query` が同じ部品を可変と共有で持ち、bevy が起動時に落とす

- **Concern**: `fill_world_visuals` の `FillData` は `&mut WorldVisual`・`&mut VisualVersion` を持ち、同じ係が `build` に渡す `NodeData` は `&WorldVisual`・`Option<&VisualVersion>` を持つ。bevy は 1 つの system の中で同じ部品を可変と共有で重ねて持つ `Query` を許さず（B0001）、`Without` で互いに素にできる形でもない（同じ節を両方が読む）。
- **Impact**: 設計どおりに書くと `World` の初期化で panic し、`update_draw_trees` が 1 度も回らない。署名を変えると `fill_world_visuals`／`build` の口（Contracts）も変わるので、タスク分けの前に決めるべき。
- **Suggestion**: 同じ wintf の `visual_hierarchy_sync_system`（`visual_sync.rs:70`）が採っている `ParamSet<(Query<FillData>, Query<NodeData>)>` の形にし、`set.p0()` で埋めて `set.p1()` で組む。または `fill_world_visuals` が返す `visited` を `(Entity, depth, world_mat, axis_aligned, content_world_aabb)` の写しにして `build` から `&WorldVisual`・`&VisualVersion` を外す（純粋なテストも列だけで組めて軽い）。どちらも `pub(crate)` の中の変更で公開面は動かない。
- **Traceability**: 4.1・4.6（埋めた `WorldVisual` を `build` が使う）・1.9
- **Evidence**: design.md「`WorldVisual`＋`fill_world_visuals`」の Contracts の署名／「`build`＋`clip_geometry`」の署名／「`update_draw_trees`」の「読むもの」

### 🔴 Critical Issue 2: 装置の失いで `VisualDrawContent` を外すと、節が持ち主の規則で持ち主へ化ける

- **Concern**: `invalidate_dependent_components` が `VisualDrawContent` を全部外す（3.4）一方、持ち主の規則 `wants_own_surface` は `has_tree_parts`＝「`VisualDrawContent` か `VisualTransform` を持つ」で判じる。`VisualTransform` を持たない節（変形なしの節は多い）は失いの回に `has_tree_parts` が偽になり、`sync_surface_owner_parts` が 3 部品を入れて WUC の見た目を作る。その節の親が節なら `fill_world_visuals` が 1.11 の `error!`（節の下の持ち主）を出し、`visual_hierarchy_sync_system` は「親が見た目を持たない＝根」として WUC の根に付け替える（`visual_sync.rs:94`）。使い手が `Res<GraphicsCore>` の変化で中身を入れ直すと今度は 3 部品が外れる。設計はこの往復を「規則が変わる（揃える）」と書くだけで、使い手の約束違反でない失いの回に `error!` と WUC の作り直しが出ることを扱っていない。
- **Impact**: 装置の失い（`device_lost_recovery_test.rs` の形で全体テストにも在る道）のたびに、本番でない状態の記録（1.11 の `error!`）と WUC の見た目の作って捨てが起きる。記録の正確さ（`logging.md`・「黙って切らない」の裏返し）と 3.4「今の立ち直りの道に乗せる」の両方を損なう。
- **Suggestion**: 節であることを失いで消えない部品に結び付ける。`#[require(WorldVisual, VisualVersion)]` で節は必ず `WorldVisual` を持ち、`invalidate_dependent_components` はこれを外さないので、`has_tree_parts` を `Has<WorldVisual>`（または require で入る印の部品）で判じれば往復は起きず、規則は 1 か所のまま。代案は `VisualDrawContent` を外さず命令の列だけを捨てる形だが、欄の形が変わり `wintf-d2d-composite-rules` の Revalidation Trigger に当たるので推さない。
- **Traceability**: 1.2a・1.11・3.4
- **Evidence**: design.md「`Visual`・`OwnSurface`・`wants_own_surface`」の規則／「`VisualDrawContent`・`VisualTransform`」の「装置の失い」／「graphics/systems（今の道の口）」の `invalidate_dependent_components` の行

### 🔴 Critical Issue 3: 組み直しの条件が根の `Arrangement`（viewport）と節の `Arrangement`（切り抜きの大きさ）の変化を見ていない

- **Concern**: `build` は viewport＝根の `Arrangement.size` で間引き（2.8）、矩形・角丸の切り抜きの大きさを節の `Arrangement.size` から取る（2.4）。ところが `update_draw_trees` の組み直しの条件は「`fill_world_visuals` の変化あり・`RemovedComponents`・`Res<GraphicsCore>` の変化・`DrawTree` が無い」の 4 つで、`fill_world_visuals` は `Arrangement` を読まない。根が広がっても外で間引かれた節は次に何かが変わるまで列に戻らず、節の大きさが変わっても切り抜きは古いまま。版も上がらない（4.3 の「切り抜きが変わる」は `Visual.clip` の形しか指していない）。
- **Impact**: 2.8「間引きは答えを変えない」と 2.4 が配置の変化の回に破れる。描画面の作り直し（`Changed<GlobalArrangement>` → `deferred_surface_creation_system` → `Changed<SurfaceGraphics>` → 描き直し）は起きるので、古い列を新しい大きさの面に描き直す＝見えない不整合として残る。
- **Suggestion**: `FillData` に `Ref<Arrangement>` を足し、根の `Ref<Arrangement>.is_changed()` と、`Visual.clip` が `Some` の節の `Ref<Arrangement>.is_changed()` を「変化あり」に数える（版は上げない＝damage-redraw の約束は変えない）。viewport の変化は全部の根でなくその根だけでよい。純粋なテスト（`world_visual_tests.rs`）に「根の大きさを変えると組み直す」の 1 本を足す。
- **Traceability**: 2.4・2.8・1.9
- **Evidence**: design.md「`update_draw_trees`」の手順③／「`build`＋`clip_geometry`」の viewport と `Arrangement` の扱い／「`WorldVisual`＋`fill_world_visuals`」の戻り値の定義

## Design Strengths

1. **既存の道への口が本当に小さい**: `render.rs` は `Option<&DrawTree>` と `emit` の 1 呼び出し、`surface.rs` は filter の 3 か所、`world/mod.rs` は登録の 2 行。描画面 1 枚を描く係は `render_surface` の 1 つのまま（3.4・3.5 を構造で守る）。`insert_graphics_parts_on_window_join` の名前は `world/mod.rs:329` でしか使われておらず（`init.rs`・`tests/visual/` に参照なし）、置き換えの影響も設計の見込みより小さい。
2. **D2D の意味論を逐語で引いて変換の規則に落としている**: `PushAxisAlignedClip`＝`root` の変換で DIP の矩形を渡す・`PushLayer`＝DC を恒等にして `maskTransform` に全部を渡す・層は `NONE` 固定で変える権限を `wintf-d2d-composite-rules` に渡す、と曖昧さを残していない。画素の見本 14 件（較正の 1 本を含む）が要件 6.3 の 12 件を越えて固定する。

## Final Assessment

- **Decision**: **GO**（条件つき: 上の 3 件を設計討議で設計書に反映してからタスク生成へ）
- **Rationale**: 構造（案 C・層の向き・既存の作法）に食い違いは無く、要件はすべて部品に落ちている。3 件はいずれも `pub(crate)` の署名・規則の入力・組み直しの条件の直しで、設計の骨格を変えない。
- **Next Steps**: `/kiro-design-discussion wintf-visual-tree` で 3 件を議題にし（推奨案で承認できる形）、反映後に `/kiro-spec-tasks wintf-visual-tree`。

## 小さな指摘（討議の材料・GO/NO-GO には影響しない）

1. 「Modified Files」の `init.rs`／`tests/visual/window_join_test.rs` は名前の置き換えに追随する必要が無い（どちらも `insert_graphics_parts_on_window_join` を綴っていない）。
2. `update_draw_trees` の「組み直すか」の 4 条件は純粋な関数（`fn needs_rebuild(changed, removed_any, core_changed, has_tree) -> bool`）に切り出して真理値表で固定できる。実 GPU の統合テストに残すのは `DrawTree` の出入りだけでよい。
3. WARP の足場は画素のテスト 14 件が各々装置を作ると全体テストの時間を食う。`OnceLock` で工場と装置を 1 度だけ作り、テストごとに DC と的のビットマップだけ作る形を検討。
4. `clip_sync_system` の鍵（`ClipKey` の `Option<ClipShape>`）に `Geometry` が入る道は設計どおり `None` に読み替えれば閉じるが、`VisualGraphics` が `ID2D1Geometry` を控えに持たないことを doc に 1 行書いておくと後の読み手が迷わない。
5. 棚卸㉟の約束の確認: `monitor_systems_transition_tests.rs:32` の `include_str!` が見張る `schedules.insert(Schedule::new(Update));`（`world/mod.rs:149`）は設計のどこからも触られない。`ecs/mod.rs` は 14 行目の `pub mod visual;` を外すだけで、`window::{DPI, ZOrderPairStrategy, …}` の再公開と `ecs/window/zorder_*` は動かない。
