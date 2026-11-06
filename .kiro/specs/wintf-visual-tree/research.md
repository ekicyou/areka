# ギャップ分析（`/kiro-validate-gap wintf-visual-tree`・2026-11-06・枝 `8a572caf`＝main `73c5b25d` の上）

要件（`requirements.md`・確定）と今のコードの差を読み、設計の段へ渡す。読んだもの: `brief.md`・`requirements.md`・steering（`product.md`・`tech.md`・`structure.md`）・申し送りの正本（`completed/audit-wintf-graphics-world/research.md` の 22〜30 番と 8・9 番・`completed/audit-wintf-runtime/research.md` の 14 節と 21・22・23・88 番）・`crates/wintf/src/ecs/visual/`（11 ファイル）・`numerics/`（6 ファイル）・`ecs/graphics/systems/{render,surface,visual_sync,init}.rs`・`ecs/graphics/{visual,core,command_list,mod}.rs`・`ecs/common/ancestors.rs`・`ecs/world/mod.rs`・`ecs/mod.rs`・`com/d2d/mod.rs`・`com/d3d11.rs`・`tools/test-all.ps1`・`tests/graphics/common/mod.rs`・`areka-emo-present/src/display_gpu_tests.rs`（読み戻しの先例）・`wintf-d2d-composite-rules/brief.md`・`wintf-visual-damage-redraw/brief.md`。cargo は回していない。

## 1. 要約

- 描画の木の部品（`VisualDrawContent`・`VisualTransform`・`VisualClip`・`VisualFlags`・`WorldVisual`・`VisualVersion`）と平らにする係 `build` は揃っていて、外からの参照は今も 0（`crates/` 全体を `grep`）。無いのは **描く係（`DrawCommand` → D2D）・スケジュールへの配線・`WorldVisual`／`VisualVersion` を埋める system・木だけの根の描画面の作成と描き直しの印・WARP の足場・テスト** の 5 つ。
- 今の描画の道は 1 本で短い（`render_surface` 146 行＝`BeginDraw`→`SetTransform(拡大＋offset)`→`Clear`→`DrawImage(SOURCE_OVER)`→`EndDraw`、装置の失いの立ち直り込み）。木の分岐は `DrawImage` の直後に 1 か所足せば乗る（要件 1.1・3.4）。
- 要件 1.8・1.9 は `ecs/graphics/systems/surface.rs` に届く: 描画面を作る係は `With<GraphicsCommandList>` で絞り、片づける係は `GraphicsCommandList` の外れで描画面を空にし、印の係は `Changed<GraphicsCommandList|SurfaceGraphics|GlobalArrangement>` しか見ない。木だけの根に「描くものがある」と知らせる印が要る（議題 4）。
- 傷 1〜8 と申し送り（30・21・22・23・88）はどれも今も残る（コードで確かめた・3 節）。見直し 2 本の直しで変わったのは `allow(dead_code)` の理由付けと `aabb.rs` の import の 1 行だけ。傷 3・4 は両方の見直しが同じ所を指すので 1 件ずつ数える。
- 設計で決めるべき分かれ道が要件の 3 議題のほかに 9 つ見えた（7 節）。大きいのは「木だけの根の印の置き方」「`WorldVisual` の座標系（根からの DIP か物理ピクセルか）」「図形の切り抜きの層と `SOURCE_OVER` でない重ね方の両立」の 3 つ。
- 規模の見立ては brief の 15〜19 のまま（L・2 週間弱）。危うさは Medium（D2D の層の意味論と WARP の答えのずれが未確認・変化の検出の「外れた節」の扱い）。

## 2. 今の姿（要件と資産の対応表）

| 要件 | 今ある資産 | 差（Missing／Unknown／Constraint） |
|---|---|---|
| 1.1〜1.7 画家の順・既定値・可視 | `draw/builder.rs` の `build`／`visit`（前順 DFS・`Children` の順・無い部品は恒等／無し）。`VisualFlags` は読まない（傷 2） | Missing: 描く係（`DrawCommand` の列を DC へ再生）。Missing: `VisualFlags` の読み（1.6・1.7） |
| 1.2 描画面の持ち主＝根・下の `Visual` は節でない | `Visual` の on_add が窓の下なら `VisualGraphics`／`SurfaceGraphics`／`SurfaceGraphicsDirty` を入れる（`graphics/visual.rs:606-640`）。後から窓へ付けた木にも入れる（`insert_graphics_parts_on_window_join`）＝窓の下の `Visual` は全部が描画面の持ち主 | Constraint: 節（`Visual` 無し）の**下**に `Visual` を置くと、今の 2 つの係が壊れる。① `propagate_global_arrangements` の `NodeQuery` は鎖の全部に `Arrangement` を要るので、節で伝播が止まる（`layout/systems/arrangement_systems.rs:52`・`common/tree_system.rs:312`）。② `visual_hierarchy_sync_system` は「親が `VisualGraphics` を持たない＝根」と見る（`visual_sync.rs:90`）ので、その `Visual` は WUC の木の根になる。要件 1.11（討議で追加）＝**記録して切る**（黙って切らない）。壊れの直しは範囲外・文書に書く |
| 1.8 木だけの根でも描画面を作る | `deferred_surface_creation_system` の `SurfaceCreationFilter` は `With<GraphicsCommandList>`（`surface.rs:192-199`）。`cleanup_surface_on_commandlist_removed` は命令の列が外れると描画面を空にする（`surface.rs:442-489`） | Missing: 木だけの根を作る係と片づける係に見せる印（議題 4） |
| 1.9 同じフレームで描き直す | `mark_dirty_surfaces` は `Changed<GraphicsCommandList>`・`Changed<SurfaceGraphics>`・`GlobalArrangement` の拡大の変化だけ（`surface.rs:174-229`）。描く係は `Changed<SurfaceGraphicsDirty>` | Missing: 節の変化（部品の変化・足す・外す・親の変更・despawn）を根の印へ写す係。外れた節は `RemovedComponents` で分かるが、どの根の下だったかは分からない（議題 6）。Constraint: bevy の変化の印は「係が前に走ってから」なので、節の変化が `Update`／`Draw` の中なら同じ回、`RenderSurface` より後なら次の回になる |
| 1.10・3.1 木を持たない描画面は今と同じ | `render.rs:117-136` | 木の分岐を「木があるときだけ」にすれば答えの画素は変わらない。固定は要件 6.3 の「木を持たない描画面との一致」と既存の `tests/graphics/`・emo の画素のテスト |
| 2.1 重ね方・補間 | `VisualDrawContent` に `D2D1_COMPOSITE_MODE`・`D2D1_INTERPOLATION_MODE` の欄・既定 `SOURCE_OVER`／`LINEAR`（`draw_content.rs:108-124`） | 揃っている。描く係が `DrawImage(list, None, None, interpolation, composite)` に渡すだけ |
| 2.2 根の変形×局所の変形 | 根の変形は `render.rs:105-112`（`GlobalArrangement` の拡大＋`BeginDraw` の offset）。`build` は `root_mat` を受け `world = local * parent`（行ベクトル・`Transform2D` の注記と同じ規約） | 揃っている。ただし `WorldVisual` をどの座標系で持つか（根からの DIP か物理か）で `build` の引数と間引きの viewport が変わる（議題 5） |
| 2.3 回転・拡大・反転 | `Transform2D::pivot_srt`／`to_affine`（`numerics/transform2d.rs`）。負の拡大も行列に乗る | テストが 1 本も無い（23 番）。符号の正しさは純粋なテストで固定（要件 6.6） |
| 2.4・2.5 矩形の切り抜き・回転した矩形 | `plan_clip`: 軸に沿えば `PushClipRect`、回転なら単位矩形の図形＋変換で `PushClipGeometryRect` へ上げる（`builder.rs:143-212`） | Missing: 描く側。`PushAxisAlignedClip` は DC の**今の変換**で矩形を変換するので、押すときの DC の変換を恒等に戻すか、world の矩形を変換してから押すかを決める（議題 7）。`unit_rect` の図形は `ID2D1Factory::CreateRectangleGeometry` で 1 度作って根ごとか資源に持つ（今は呼び手が渡す） |
| 2.6・2.7 任意の図形・入れ子の交わり | `VisualClip::Geometry` → `PushClipGeometryEntity`（world の行列を持つ・図形は実体から引く） | Missing: `PushLayer(geometricMask, maskTransform)` の描き側。Unknown: 層の中で `SOURCE_OVER` でない重ね方がどう効くか（議題 7・`D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND`）。交わりは層の入れ子で自然に成る |
| 2.8 間引きは答えを変えない | `visit` の `eff`（viewport ∩ 祖先の切り抜き）で間引く | 固定の方法: 純粋なテスト（viewport を無限にした列と比べる）＋WARP で画素の一致 |
| 2.9 局所は 96 DPI の DIP | `VisualDrawContent::new_calc_aabb_ex` の注記（DC を 96 DPI に固定せよ）・`local_aabb` は DIP | 揃っている。`new_calc_aabb` に渡す「96 DPI の計算専用 DC」は今の `GraphicsCore` に無い（共有 DC は画面の DPI になりうる）。使い手が `SetDpi(96)` するか、足場だけで持つかを設計で（Research） |
| 3.2・3.3 WUC の木との棲み分けと文書 | `Visual.clip`（`ClipShape`）は `clip_sync.rs` で WUC の `Visual.Clip` へ、`is_visible`／`opacity` は `visual_sync.rs` へ | Missing: 公開の型の注記（`VisualClip`・`VisualFlags` の doc に「描画面の中で効く」、`Visual` の doc に「WUC の木で効く」） |
| 3.4 装置の失いの立ち直り | `render.rs:71-101`（`note_draw_failure`・残りを描かずに抜ける） | 木の分岐を同じループの中に置けば乗る。木の描画の途中の D2D の失敗（`PushLayer`・`DrawImage` は HRESULT を返さない・`EndDraw` で出る）も同じ `EndDraw` の記録に乗る |
| 3.5 既存の画素のテストが同じ答え | `tests/graphics/*`・`areka-emo-present/src/display_gpu_tests.rs` が `render_surface` と同じ引数で再生して固定 | 木の分岐は `DrawImage` の後ろに足すだけなので変わらない。全体テストで確かめる |
| 3.6・3.7 名前・依存 | `windows` の機能 `Win32_Graphics_Direct3D`／`Direct3D11`／`Direct2D` は入っている（ワークスペースの `Cargo.toml:57-63`）。`bitflags` も既に使う | WARP（`D3D_DRIVER_TYPE_WARP`）は同じ enum なので依存は増えない |
| 4.1〜4.2 `WorldVisual` を毎フレーム埋める | `WorldVisual`（`world_visual.rs`・`PENDING`・`from_world_mat`）は型だけ。埋める係 0 | Missing: 根ごとの DFS で `world_mat`・`axis_aligned`・`content_world_aabb`・`subtree_world_aabb` を埋める係。`subtree` は後順で和を取るので再帰か明示のスタック |
| 4.3〜4.4 版を上げる・変形では上げない | `VisualVersion::bump`（`version.rs`）。上げる係 0 | Missing: `Changed<VisualDrawContent>`／`Changed<VisualClip>`／`Changed<VisualFlags>` で 1 つ上げる係（無ければ入れる）。`VisualTransform` を見なければ 4.4 は自然に守れる |
| 4.5 hash に番地を使わない | `clip.rs:27`・`draw_content.rs:23`・`builder.rs:246-252`（傷 1） | 直す: `DrawItem.hash` を（実体・`VisualVersion`・補間・重ね方・world の行列）から取る。`VisualDrawContent`／`VisualClip` の `impl Hash` は番地を混ぜる元なので外す（使い手は `hash_draw` だけ）（議題 11） |
| 4.6 平らにする係が `WorldVisual` を使う | `visit` が `local * parent_world` を自前で積む（傷 2） | 直す: `NodeData` に `WorldVisual` を足し `world_mat`・`content_world_aabb` をそこから読む。`build` の `root_mat` の引数は、議題 5 の答えで残すか消す |
| 5.1 失敗は `error!` | `render.rs`・`surface.rs` は `error!(entity, error, hresult)` の形 | 新しい係も同じ形（`format_entity_name`） |
| 5.2 `GetBounds` の失敗 | `geometry_bounds` が `unwrap_or_default()`（`builder.rs:239-242`・傷 5） | 直す: `Result` を返し、`Err` は `error!`＋その節と子孫を飛ばす（`ClipKind::Culled` と同じ出口） |
| 5.3・5.4 深さの上限と輪 | `visit` は番兵なし（傷 6）。手本: `common/ancestors.rs` の `MAX_ANCESTOR_DEPTH = 1024`＋`error!` 1 行・`insert_graphics_parts_on_window_join` の明示のスタック（`graphics/visual.rs:583-598`） | 直す: 深さを引数で持ち上限で `error!`＋止める。再帰のままでも 1,024 段 × 小さいフレームなら足りるが、明示のスタックにすれば「あふれない」が構造で言える（議題 13） |
| 5.5 軸に沿うかを 1 か所に | `Matrix3x2Ext::axis_aligned`（ちょうど 0・`matrix3x2.rs:11-13`）・`Aabb::transform` の近道（`aabb.rs:133`）・`is_rect_preserving`（1e-4・90 度も可・`builder.rs:268-273`）・`WorldVisual::from_world_mat` は `axis_aligned` を使う（傷 3＝21 番） | 直す: `axis_aligned` を「許容つき・90 度の倍数も沿う」の 1 つにし、`Aabb::transform` の近道と `WorldVisual`・`builder` がそれを使う。許容の値（1e-4 を f32 の桁で説明する）は設計で |
| 5.6・5.7 矩形の働きを `numerics` へ・NaN は空 | `Aabb::{is_empty, intersect, transform}`（`aabb.rs`）と `builder` の `aabb_is_empty`／`aabb_intersect`／`aabb_intersects`／`transform_aabb` が二重（傷 4＝22 番）。`Aabb::is_empty` は `!(r>l && b>t)` で NaN を空と見る・`builder` は `r<=l || b<=t` で NaN を空と見ない。`Aabb::transform` は空なら `EMPTY` を返す・`builder::transform_aabb` は `EMPTY`（±inf）の角を変換して NaN／平面を返す | 直す: `builder` の 4 つを消し `Aabb` の 3 つ（＋`intersects` を 1 つ足すか `intersect().is_empty()` で代える）へ寄せる。NaN と空の答えは純粋なテストで固定 |
| 5.8 `Send/Sync` の注記 | `clip.rs:15-16`・`draw_content.rs:17-18` に注記なし（傷 7）。手本 `command_list.rs:29-33` は工場が 1 つになった今（`core.rs:155-156`）事実に合う | 直す: 同じ形の注記。`VisualClip::Geometry` の `ID2D1Geometry` は**工場**から作る（装置でない）ので「同じ MULTI_THREADED の工場の系列」が根拠。使い手が別の工場で作った図形を入れる道は閉じられない＝注記に「使い手の約束」として書く |
| 5.9 モジュール名 | `ecs/mod.rs:14` `pub mod visual;`（公開）・`ecs/graphics/mod.rs:6` `pub mod visual;`（`Visual` 部品）・`ecs::graphics::*` を `ecs` の根で再公開。統合テストの `tests/visual/` は `Visual` 部品のテスト | 直す: 描画の木の側を改名（議題 2）。`structure.md:55` の `visual/` の行も直す |
| 5.10 README の写し | `draw/README.md`（48 行・`DrawCommand` 等の写し＝30 番） | 直す: 型の写しを消し、ソースへの参照だけ残すか README を消す |
| 5.11 `moved` の単位 | `matrix3x2.rs:16-26`（単位の正方形の 4 角を比べる＝88 番） | 直す: 比べる矩形（`Aabb`）を引数で受けてその角を比べる形か、平行移動は px で・線形の部分は別に比べる形（議題 10）。使い手は今 0・`wintf-visual-damage-redraw` が使う |
| 5.12 `allow(dead_code)` と使い手 0 の働き | `builder.rs:57`・`command.rs:1`（理由つき）。`Aabb::{contains, intersect, union, translate}`＋演算子・`Point2`・`moved`・`pivot_srt`／`to_affine` は使い手 0（88 番） | 配線すれば印は外せる。使い手 0 の働きは消さず純粋なテストで固定（要件 6.6） |
| 6.1 WARP の足場 | `GraphicsCore::create` は `D3D_DRIVER_TYPE_HARDWARE` 固定・私的（`core.rs:147-172`・`:271-274`）。`com/d3d11.rs::d3d11_create_device` は公開で driver type を受ける。`crates/` に WARP の装置は 0 | Missing: WARP の D3D→DXGI→`ID2D1Factory1::CreateDevice`→DC の足場（議題 9） |
| 6.1 読み戻し | wintf には無い。先例は `areka-emo-present/src/display_gpu_tests.rs:138-185`（`CPU_READ|CANNOT_DRAW` の staging へ `CopyFromBitmap` → `Map(READ)` → pitch を踏まえて行複写・約 40 行）。emo-present は wintf の下流なので借りられない | Missing: wintf のテストの中に同じ形の写し（`#[cfg(test)]` の兄弟ファイル） |
| 6.2 全体テストで常に回る | `tools/test-all.ps1` の `cargo test --workspace --no-fail-fast -j 4` | `src/` の `#[cfg(test)]` に置けば自動で入る。環境変数の門を付けなければよい。`src/` のテストには落ちの報告と MTA の常駐（`test_crash_report.rs:142-146`）が付く |
| 6.3〜6.4 画素の見本 | 見本の画像ファイルは無い（`tests/assets/` は別物） | コードの中の値で持つ（議題 3） |
| 6.5 実 GPU の例外を `tech.md` へ | `tech.md` Testing「GPU のテストは実のハードウェア GPU で行う（WARP などへ替えない）」 | 書き足す: 「描画の木の画素の答えに限って WARP」。`GpuPermit`（実 GPU の装置の数を絞る足場）は areka の中にあり wintf には無い・WARP には要らない |
| 6.6〜6.8 純粋なテスト・兄弟ファイル | `numerics/` にテスト 0（23 番）。`ecs/common/ancestors_tests.rs` が兄弟ファイルの形の手本 | 足す: `numerics/*_tests.rs`・`draw/builder_tests.rs`（`Query` を要るので `World` を組んで `world.run_system_once` か `SystemState`） |
| 7.1 example | `examples/clip_demo.rs` が手本（`WinApp`・`Window`・`Visual`・`Rectangle`） | 足す: 節に `VisualDrawContent` を記録する係（装置は起動後に出来るので `world.add_systems(Draw, …)`・`world/mod.rs:420` で足せる）。`Cargo.toml` に `[[example]]` の節は無い（ファイル名で見つかる） |
| 7.2 steering | `structure.md:48-55`・`tech.md` Testing | 書き直す |
| 7.3 1,000 行 | `render.rs` 146・`surface.rs` 341・`world/mod.rs` 717・`builder.rs` 301・`aabb.rs` 237 | 余裕あり。`world/mod.rs` には係の登録が数行増えるだけ |

## 3. 傷と申し送りの数え直し（1 件ずつ）

見直し 2 本の記述をコードで確かめ直した。どれも**今も残る**。

| 傷 | 出どころ | 場所（今） | 確かめた事実 |
|---|---|---|---|
| 1 | graphics-world 22 | `clip.rs:27`・`draw_content.rs:23`・`builder.rs:246-252` | `as_raw() as usize` を hash。`PartialEq` も番地比べ（それは正しい） |
| 2 | graphics-world 23 | `builder.rs:17-22`・`:94-97` | `NodeData` に `VisualFlags`・`WorldVisual` が無い。world を自前で積む |
| 3 | graphics-world 24＝runtime 21（1 件） | `matrix3x2.rs:11-13`・`aabb.rs:133`・`builder.rs:268-273`・`world_visual.rs:52` | 3 通り。`WorldVisual::from_world_mat` は厳密な方を使う |
| 4 | graphics-world 25＝runtime 22（1 件） | `builder.rs:219-236`・`:276-298`・`aabb.rs:81-83`・`:91-98`・`:127-160` | 二重。NaN と空の変形で答えが逆（2 節の 5.6・5.7 の行） |
| 5 | graphics-world 26 | `builder.rs:239-242` | `unwrap_or_default()`＝0 の矩形。記録なし |
| 6 | graphics-world 27 | `builder.rs:75-141` | 番兵なしの再帰 |
| 7 | graphics-world 28（手本は 8 番の直しの後） | `clip.rs:15-16`・`draw_content.rs:17-18` | 注記なし。手本 `command_list.rs:29-33` は今の事実（工場 1 つ・`core.rs:155`）に合う |
| 8 | graphics-world 29 | `ecs/mod.rs:14`・`ecs/graphics/mod.rs:6` | 2 つの `visual` |
| 30 | graphics-world 30 | `draw/README.md` | `DrawCommand`・`DrawItem` 等の写し（`command.rs` と同じ） |
| 23 | runtime 23 | `numerics/` | テスト 0 |
| 88 | runtime 88 | `matrix3x2.rs:16-26` ほか | `moved` は単位の正方形の角だけ。使い手 0 の働きの表はあちらの 4 節 |

新しく見えたもの（傷の番号は付けない・設計で扱う）:

- **N1** `DrawItem` は命令の列を持たず `entity` だけ持つ（`command.rs:23-28`）。描く係は `Query<&VisualDrawContent>` で引き直すか、`DrawItem` に `ID2D1CommandList` の複製（AddRef だけ）を持たせる。`ClipGeometryEntity` も同じ。
- **N2** `build` の `unit_rect`（単位矩形の図形）は呼び手が作って渡す。作る所（工場・根ごとか資源か）が無い。
- **N3** `VisualDrawContent::new_calc_aabb` が要る「96 DPI の計算専用 DC」は `GraphicsCore` に無い。
- **N4** 節の下の `Visual` は今の 2 つの係を壊す（2 節の 1.2 の行）。

## 4. 作りの案

### 案 A: 今の係を広げる

`render_surface` のループの `DrawImage` の後ろに木の分岐、`surface.rs` の 3 つの係の filter を広げ、`build` の傷をその場で直し、描く係を `com/d2d/mod.rs` の拡張 trait に足す。

- ✅ ファイルが増えない・装置の失いの道にそのまま乗る
- ❌ `com/d2d/` は COM の薄い包みの層（`structure.md`）。`DrawCommand`（ECS の型）を知る描く係を置くと層の向きが逆になる。`surface.rs` は 341 行で、印の係・作る係・片づける係に木の条件が散らばる

### 案 B: 別の描く道を新しく作る

`RenderSurface` に木専用の係 `render_tree_surface` を足し、木を持つ根だけそちらで描く。

- ✅ `render.rs` に触らない
- ❌ `BeginDraw`・装置の失い・`EndDraw` の 146 行をもう 1 度書く＝要件 3.4 の「同じ立ち直りの道に乗せる」に反する。描画面 1 枚を 2 つの係が描く順の問題も出る

### 案 C（推奨）: 木の層に係を置き、今の道には小さな口だけ足す

- **木の層（改名後のモジュール）に足すもの**: ① `WorldVisual` を埋め `VisualVersion` を上げる係（根ごとの DFS・根の印を立てる）② 描く係 `emit(dc, root_mat, &[DrawCommand], …)`（`draw/emit.rs`・COM を呼ぶのはここだけ）③ 根の印の部品（平らにした `Vec<DrawCommand>` の控え・`unit_rect` を持つ・議題 4）④ 純粋なテストと WARP の足場（兄弟ファイル）。
- **今の道に足す口**: `render.rs` のループに `Option<&根の印>` を読み、`DrawImage` の後ろで `emit` を呼ぶ数行。`surface.rs` の 3 つの filter に根の印を足す（`Or<(With<GraphicsCommandList>, With<根の印>)>`・片づけは `Without<根の印>` も条件・印は `Changed<根の印>`）。`world/mod.rs` に係の登録。
- **数の部品**: `numerics` に寄せ、`builder` の重複を消す。
- ✅ 層の向きを保つ（`com/d2d/` は触らない）・`render.rs`／`surface.rs` の差分が小さい・落ちの報告と MTA 付きの `src/` のテストに置ける
- ❌ ファイルが 5〜7 増える（どれも小さい）

### 規模と危うさ

- 規模: **L**（brief の 15〜19 タスク・分けない）。内訳の目安: 傷の直しと数の部品 4〜5・埋める係と版 2・描く係と根の印と配線 3〜4・WARP の足場と画素のテスト 3・純粋なテスト 2・example と文書 2。
- 危うさ: **Medium**。① D2D の層（`PushLayer`）の中で `SOURCE_OVER` でない重ね方がどう効くかが未確認（議題 7）② WARP の答えが実機の更新でずれる件は ±1 で吸えるかを着手で確かめる ③ 外れた節・despawn を根の印へ写す道（議題 6）④ 既存の画素のテストは `render_surface` と同じ引数で固定しているので、木の分岐を `DrawImage` の後ろに限れば緑のまま。

## 5. 設計へ持ち越す調べもの（Research Needed）

1. **`PushLayer` と重ね方**: 図形の切り抜きは `ID2D1DeviceContext::PushLayer(D2D1_LAYER_PARAMETERS1 { geometricMask, maskTransform, … })` で描く。層の中身は透明から始まるので、`SOURCE_IN`・`DESTINATION_*` などは層の外の絵でなく層の透明に対して効く。`layerOptions = D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND` で層を下の絵から始められる（Microsoft Learn の `D2D1_LAYER_OPTIONS1` の項）。要件 6.3 の「`SOURCE_OVER` でない重ね方 1 つ以上」を切り抜きの下でも固定するなら、この旗と戻し方（`PopLayer` の合成）を着手で測る。`PushAxisAlignedClip` は層を作らないので影響しない。
2. **`PushAxisAlignedClip` と DC の変換**: 矩形は押すときの DC の変換で変換される。`emit` は押す前に `SetTransform(identity)`（world の矩形はもう物理）にするか、根の変形だけ残すか（議題 5 の答えと連動）。アンチエイリアス（`D2D1_ANTIALIAS_MODE_PER_PRIMITIVE`／`ALIASED`）で境界の画素が ±1 を越えて変わりうる → 見本は境界から離れた画素で取る（`areka-emo-text/src/viewbox_draw_render.rs:392` は `ALIASED` を使う先例）。
3. **WARP の装置の作り**: `d3d11_create_device(None, D3D_DRIVER_TYPE_WARP, HMODULE::default(), BGRA_SUPPORT, …)` → `cast::<IDXGIDevice>` → `D2D1CreateFactory(MULTI_THREADED)` → `factory.CreateDevice(&dxgi)` → `CreateDeviceContext`。デバッグ層の旗は WARP でも SDK の部品を要るので足場では付けない。`CoInitialize` は要らない（`test_crash_report` が MTA を常駐させる）。
4. **96 DPI の計算専用 DC**（N3）: 足場の DC は `SetDpi(96,96)` で固定（emo-present の先例 `gpu_dc()`）。本番で `new_calc_aabb` を使う道を用意するかは設計で（使い手は `local_aabb` を自分で与えてもよい）。
5. **bevy 0.19 の `Children` の順と変化**: `ChildOf` の付け替えで `Children` が更新される。`Changed<ChildOf>` と `RemovedComponents<ChildOf>`・despawn の検出の組み合わせを着手で確かめる（議題 6）。
6. **`Query` の純粋なテストの書き方**: `build` は `&Query<NodeData>` を受けるので、`World` を組み `SystemState` か `world.run_system_once` で呼ぶ。`ancestors_tests.rs`・`visual_sync_tests.rs` の形を見て揃える。
7. **WARP の ±1 の根拠**: WARP は決定論だが Windows の更新で 1 段ずれうる（`wintf-d2d-composite-rules` の brief）。着手で同じ絵を 2 度描いて一致、境界の画素を外した見本で ±1 を確かめる。

## 6. 設計へ渡す推奨（情報であって決定ではない）

- 案 C。描く係と埋める係は木の層へ、`render.rs`／`surface.rs`／`world/mod.rs` は口だけ。
- `WorldVisual` は根からの DIP（DPI を含まない）で持ち、描くときに根の変形（拡大＋offset）を掛ける（要件 2.9・damage-redraw の「変形では版を上げない」と相性がよい）。
- 木だけの根の印は専用の部品（平らにした列の控えの置き場を兼ねる）。
- hash は（実体・版・補間・重ね方・world の行列）。`impl Hash` の番地混ぜは外す。
- 深さは明示のスタック＋上限 1,024（`MAX_ANCESTOR_DEPTH` を共用）。
- 画素のテストは `src/` の兄弟ファイル（落ちの報告・MTA 付き）、見本はコードの中の値、境界から離れた画素、±1。

## 7. 議題の振り分け（2026-11-06 要件の討議で分類）

要件の末尾の 3 件（1〜3）は**開発者へ**（答えで公開の形・テストの作りが変わる）。ギャップ分析で見えた 4〜13 は **設計の段（`/kiro-design`）で決める設計判断**＝要件は変えない（12 だけは要件 1.11 に写して閉じた）。推奨は「推」。設計の段への約束: 4 の根の印は使い手が置くものではなく係が自動で入れる（要件 1.1・1.8「節を置くだけで」の字面を守る）。

1. **「描画面の中の節」の見分け方**（要件 1.2・brief 議題 1）: 推＝専用の印を置かず「持ち主の子孫で `Visual` を持たず、描画の木の部品のいずれかを持つ実体」。コードの裏づけ: 窓の下の `Visual` は全部が描画面を持つ（`graphics/visual.rs`）。ただし N4（節の下の `Visual` が今の 2 つの係を壊す）は議題 12 へ。
2. **名前の衝突の解き方**（要件 5.9・brief 議題 2）: 推＝描画の木の側を改名。候補 `ecs::draw_tree`（brief）／`ecs::surface_tree`（「描画面の下の木」が名前で分かる）。型の名前は変えない。`structure.md:55` と `tests/visual/`（`Visual` 部品のテスト・そのまま）の説明も直す。
3. **WARP の答えのずれ**（要件 6.4・brief 議題 3）: 推＝コードの中の値・±1・境界から離れた画素。`wintf-d2d-composite-rules` の議題 4 もこれに揃える。
4. **木だけの根の印**（要件 1.8・1.9・`surface.rs` に届く）: ㋐ 推＝専用の部品（例 `SurfaceDrawTree`／`DrawTreeRoot`。平らにした `Vec<DrawCommand>` の控えと `unit_rect` を持ち、埋める係が木の変化で `set_changed`）。`surface.rs` の 3 つの filter と `render.rs` の読む部品に足す。㋑ `GraphicsCommandList::empty()` を根に置く約束にして `surface.rs` を触らない（要件 1.8 の「命令の列を持たず」と字面が合わない・使い手に約束を強いる）。
5. **`WorldVisual` の座標系**（要件 2.2・2.9・4.1）: ㋐ 推＝根からの DIP（DPI が変わっても `WorldVisual` が変わらない・`build` の `root_mat` の引数は消え、`emit` が根の変形を掛ける・間引きの viewport は描画面の矩形を拡大で割った DIP）。㋑ 物理ピクセル（`build` のまま・DPI の変化で全節が埋め直し）。
6. **外れた節・despawn を根の印へ写す道**（要件 1.9）: ㋐ 推＝`RemovedComponents<VisualDrawContent|VisualClip|VisualFlags|VisualTransform|ChildOf>` が 1 件でもあれば、木を持つ根を全部 `set_changed`（稀で安い・根は数枚）。㋑ 根の印に前の回の節の一覧を持ち、消えた実体の根を引く（正確だが作りが増える）。
7. **図形の切り抜きの層と重ね方**（要件 2.6・2.1・6.3）: `PushLayer` の `INITIALIZE_FROM_BACKGROUND` を使うか（Research 1）。推＝着手で測って決める。要件 6.3 の「`SOURCE_OVER` でない重ね方」の見本を切り抜きの外で取るなら答えに依らない。
8. **切り抜きのアンチエイリアス**（要件 2.4・6.4）: `PER_PRIMITIVE`（D2D の既定・境界が滑らか）か `ALIASED`（境界が決定論に近い）。推＝既定のまま、見本は境界を避ける。
9. **WARP の装置の作り方**（要件 6.1）: ㋐ 推＝テストの足場だけで作る（`d3d11_create_device` ＋ `D2D1CreateFactory` の 10 数行・`GraphicsCore` の公開面を増やさない）。㋑ `GraphicsCore::new_with_driver(D3D_DRIVER_TYPE)` を公開（`wintf-d2d-composite-rules` も使うが、本番の口に WARP が混ざる）。
10. **`Matrix3x2Ext::moved` の直し方**（要件 5.11）: ㋐ 推＝比べる矩形（`Aabb`・DIP か px）を引数で受け、その 4 角の変換後の差を `eps_px` で比べる（名前どおり）。㋑ 平行移動（M31・M32）は px でそのまま、線形の部分は別の許容で比べる。使い手 0 なので署名を変えてよい。
11. **`VisualDrawContent`／`VisualClip` の `impl Hash`**（要件 4.5）: ㋐ 推＝外す（使い手は `hash_draw` だけ・番地を混ぜる元を残さない）。㋑ 版を混ぜる形に書き直す（部品が版を持たないので不自然）。
12. **節の下の `Visual`**（要件 1.2・N4）: ㋐ 推＝埋める係が見つけたら `error!` を 1 行（実体名）残し、その実体と子孫を節と見ない（既存の係の壊れは直さず文書に書く）。㋑ 黙って切る（記録の無い失敗の道になる）。→ **閉じた**: 要件 1.11 に ㋐ を書いた（記録の無い失敗の道は作らない・`logging.md`）。
13. **再帰か明示のスタックか**（要件 5.3・5.4）: ㋐ 推＝明示のスタック（`insert_graphics_parts_on_window_join` と同じ形・あふれないことが構造で言える）。㋑ 再帰に深さの引数（差分が小さい・1,024 段の再帰はスレッドの既定の 2 MB で足りる見込み）。

## 8. 次の段

要件の討議（`/kiro-requirements-discussion`）で 7 節の 13 件を扱い、答えを要件へ写してから `/kiro-design wintf-visual-tree`。設計の段では 5 節の 1・2・7 を先に測る。
