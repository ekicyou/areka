# ギャップ分析: wintf-surface-auto-promotion

> 2026-11-16 `/kiro-validate-gap`（要件の段・requirements.md は未承認）。基準は main `5b3a361c` ＋本ブランチの初期化コミット `ba4c708c`。答えでなく材料と案を並べる。決めは要件の討議と設計の段で行う。

## 1. 分析の要約

- **土台はそろっている**: 規則 `wants_own_surface`（`visual/mod.rs` 95 行目）・3 部品の出し入れ `sync_surface_owner_parts`（`visual/systems.rs` 33 行目）・昇格／降格で全部の根を組み直す `TreeEdits`・前順の訪れと世界の行列 `fill_world_visuals`（`visual/world_visual.rs`）・差分の描き直し（部分木が親の描く列から外れる・戻る変化はそのまま「消えた・足された項」になる）・WARP の足場 `draw/emit_test_support.rs` は在る。要件 3.3 は既存の差分の描き直しでほぼ満たされる。
- **欠けているもの**は 4 つ: (a) 節の更新の時刻の履歴と、上げ下げの純粋な規則（新しく作る）。(b) 自動で上げてよい形の見分け（後に描く中身との重なり・描画面に収まるか・物理ピクセルの整数の位置か ほか・新しく作る）。(c) 節の下の持ち主を WUC の木に正しく繋ぐこと（今は `visual_hierarchy_sync_system` が「親が `VisualGraphics` を持たない＝根」と扱い、`fill_world_visuals` が切って `error!`）と、その位置・不透明度・可視・切り抜きを祖先の節から写すこと。(d) 持ち主の `VisualTransform` を WUC の `TransformMatrix` へ写すこと（今は `update_draw_trees` が `warn!` を残して無視）。
- **いちばん難しい所**は「同じフレームのうちに切り替える」（要件 3.1・2.4・2.5）と PreRenderSurface の中の係の順の噛み合わせ。WUC の Visual を作る係（`visual_resource_management_system`）は `update_draw_trees` より前に回るので、`update_draw_trees` の中で上げると決めても、その回には WUC の Visual も描画面も無く、部分木が 1 フレーム欠ける。案（§4・議題 D1）は「用意してから切り替える 2 段」か「係の順を組み替える」。
- **画素の一致の厳しさが効きを決める**: WUC の子の Visual は親の描画面の中身より必ず上に重なり、`SpriteVisual` の描画面は自分の `Arrangement` の大きさで切れ、WUC の位置は浮動小数で再標本化される。厳しく一致を求めると、動く部品（dola で小数の位置へ動く・拡大 1.25 で整数の DIP が小数の物理ピクセルになる）はほとんどのフレームで上げられない形になり、自動の昇格の主な使い手（動く部品）に効かない恐れがある（議題 D3）。
- **規模**: 要件をそのまま全部作ると 14〜18 タスクで M の上端から L の下端。手動の持ち主を節の下に置いたときの祖先の矩形の切り抜きと不透明度（要件 4.8）と、回転・拡大を含む手動の持ち主の `VisualTransform`（要件 5.4）を「`warn!` で保留」に寄せれば 11〜14 タスクの M に収まる見込み（§6）。

## 2. 今の作り（調べた所）

### 2.1 描画面の持ち主か節か

| 所 | 今の振る舞い | 本 spec との関わり |
|---|---|---|
| `visual/mod.rs` `wants_own_surface(has_command_list, has_own_surface, is_node)` | `has_command_list \|\| has_own_surface \|\| !is_node` の 1 行。フック `on_visual_add` と `sync_surface_owner_parts` が共用 | 自動の上げ下げの状態を 4 つ目の入力にするか、別の規則を前に置くか（議題 D2） |
| `visual/systems.rs` `sync_surface_owner_parts` | 見る変化は `Added<WorldVisual/OwnSurface/GraphicsCommandList>`・`OwnSurface`／`GraphicsCommandList` の外れ・親の変化。答えどおり `VisualGraphics`・`SurfaceGraphics`・`SurfaceGraphicsDirty` を入れ、外すときは `DrawTree` も外す | 自動の状態の出入りも同じ係で拾わせれば、出し入れの作りを足さずに済む |
| `TreeEdits` | 昇格（`Added<VisualGraphics>` かつ `WorldVisual`）・降格（`VisualGraphics` の外れ）があれば全部の根を組み直す | 自動の上げ下げのたびに全部の根の組み直しと項の比べが走る（描き直しは差分で済む）。上げ下げの数が増えると手間が増える（§7 R5） |
| `update_draw_trees` | 毎フレーム全部の根で `fill_world_visuals` を回し、組み直しが要る根だけ `build`。根の `VisualTransform` は入った・変わった回に `warn!` | 判断の材料（版の上がり・世界の行列の変化）が手に入るのはこの係の中だけ |

### 2.2 描画の木と世界の行列

- `fill_world_visuals` は根の子から前順に歩き、`visited`（実体・深さ）と `WorldVisual`（`world_mat`・`content_world_aabb`・`subtree_world_aabb`）を埋める。**子が描画面の持ち主（`has_surface || wv.is_none()`）なら訪れず、親が節なら `error!` を 1 度だけ残して切る**（160 行目）。根の直下の持ち主は記録しない。
- 版（`VisualVersion`）は `Visual` か `VisualDrawContent` が変わった節だけ上がる。`VisualTransform` では上げない（`version.rs` の約束）。行列は値が違うときだけ書く（`*wv != *w`）。
  - よって「版の上がり」は `bumped`、「世界の変形の変化」は `*wv != *w` で、どちらも fill の中で分かる。外の係から見るなら `Changed<VisualVersion>`・`Changed<WorldVisual>` が次の係に見える。
  - **祖先が動くと子孫の行列も全部変わる**。要件の「節の更新」の定義（持ち主から見た世界の変形の変化）のままだと、動くまとめ役の下の子孫が全部同時に頻繁と判断される（議題 D5）。
- `build` は根の自分の `VisualDrawContent` を恒等の行列で先頭の 1 件にし、根の `clip`・`opacity` は描かない（WUC が受け持つ）。viewport は根の `Arrangement.size` の矩形で、外は間引かれる。

### 2.3 WUC の木の同期（`systems/visual_sync.rs`）

- `visual_hierarchy_sync_system`: 居るべき親を **直の親（`ChildOf`）** の `VisualGraphics` で決め、親が `VisualGraphics` を持たなければ「根」（自分の Visual を控えに置き、どこにも繋がない）とする。LayoutRoot と節を区別しない。
  - よって節の下の持ち主は、今は WUC の木のどこにも繋がらない（画面に出ない）。fill の `error!` と合わせて二重に切られている。
  - 並べ直しは「控えの親が居るべき親と違う」子が出たときだけ起き、そのとき親の `Children` の順で `RemoveAll`→`InsertAtBottom` を繰り返す。**`Children` の並びだけが変わった回（親は同じ）は並べ直さない**（`Changed<Children>` を見ていない）。要件 4.5・4.6 で節の並びの変化に追うなら、ここも足りない（既存の欠け・§3 の Constraint）。
- `visual_property_sync_system`: 位置は `Arrangement.offset × GlobalArrangement` の拡大（`SetOffset`）だけ。`VisualTransform` は見ない。不透明度は自分の `is_visible`／`clamped_opacity` だけ（祖先の節の値を掛けない）。
- `clip_sync_system`: 自分の `Visual.clip` を WUC の角丸矩形の図形の切り抜きへ写す（大きさは自分の `Arrangement.size × 拡大`）。図形の切り抜きは「切り抜きなし」へ読み替えて `warn!`。祖先の節の切り抜きは見ない。WUC の切り抜きは Visual の自分の座標で効く（`TransformMatrix` の前）ので、持ち主の `VisualTransform` を写しても切り抜きの意味は節のとき（局所の座標で切る）と揃う。
- WUC の Visual は全部 `SpriteVisual`（`visual_manager.rs`）。描画面は `SurfaceBrush` で自分の中身になり、**子の Visual は必ずその上に重なる**。描画面の大きさは自分の `GlobalArrangement.bounds` の切り上げ（子孫を集めない・`layout/arrangement.rs` 57 行目）。

### 2.4 描画面・描き方・記録

- `deferred_surface_creation_system`: `VisualGraphics` ＋（`GraphicsCommandList` か `DrawTree`）を持つ実体に、`GlobalArrangement.bounds` の大きさの描画面を作る。作れなければ `error!`（要件 2.5 の「節のまま描き続ける」には、作れなかったことを上げ下げの係へ返す道が要る）。
- `render_surface`: 入れ物（`SurfaceComposite`）へ拡大だけの変換で合成し、描画面へ写す。記録の行 `[render_surface] damage`（target `wintf::damage`・debug）は欄を変えない（要件 7.4）。上げ下げの判断はこの行に頼らない（要件 1.4・1.8）ので、ここは触らない見込み。
- `SurfaceReadback`（`graphics/readback.rs`）は描画面 1 枚ごとの入れ物を読む。自動で上げた部分木は親の読み戻しに載らなくなる（議題 9 の仮定どおり）。使い手（areka の実機・emo-text のテスト）は新しい描き方の部品を使わない（`crates/areka*` に `VisualDrawContent`／`VisualTransform`／`OwnSurface` の使い手は 0 件・grep で確認）ので答えは変わらない。

### 2.5 時刻と tick

- `FrameTime(f64 秒)` は `try_tick_world` の頭で `dola::runtime::clock::now()`（QPC）から入る。テストは資源を差し替えて注入できる（dola・typewriter の前例）。
- tick は旗で間引かれる（`tick_wake`・`tick_gate`）。変化の無い間は回らず、心拍は `TICK_HEARTBEAT_FRAMES = 30` 回の画面更新ごと。**「静かになってから戻す」の判断は、静かな間は tick が回らないので遅れる**（最大で心拍 1 周期・約 0.5 秒）。期限で起こすなら `tick_wake::arm_deadline(Instant)` が在り、tooltip が `FrameTime` から `Instant` へ直して預ける前例を持つ（`ecs/tooltip/system.rs` 219 行目）（議題 D8）。

### 2.6 テストの足場

- WARP の足場は **根 1 つの木を 1 枚の的へ描く** 形（`render_tree_to_pixels`・`tree_commands`・`TreeFrames`）。WUC の重ね（親の描画面の上に子の描画面を位置と不透明度で重ねる）を写す口は無い。要件 8.3 の「上げた形と全部を節にした形の画素の一致」には、描画面ごとに描いた絵を的へ `SOURCE_OVER` で重ねる WARP の口を足す要がある（WUC の合成を D2D で写した近似・§3 Research）。
- 実 GPU のテスト（`tests/graphics/composite_systems_test.rs`・`draw_tree_systems_test.rs`）は HWND の無い `Window` で回し、WUC の描画面の画素を `ICompositionDrawingSurfaceInterop2::CopySurface` で読む。**WUC が重ねた後の画面の画素を読む口は無い**。`Windows.Graphics.Capture`（`GraphicsCaptureItem::CreateFromVisual`）は windows クレートの機能 `Graphics_Capture` が workspace の `Cargo.toml` に無く、足すと `Cargo.toml` に触る（要件 9.4 に反する）。
- 更新が要る既存のテスト: `world_visual_tests.rs` の `owner_under_node_is_cut_with_error_but_not_under_root`・`owner_under_node_errors_once_while_it_persists`（`OWNER_TEXT`）、`systems_tests.rs` の `owner_transform_warns_once_per_change`、`tests/graphics/draw_tree_systems_test.rs` の昇格まわり（`promoted_node_drops_out_of_root_tree` ほか・答えは変わらない見込み）、`draw/README.md` の「写し」の後半の前提 2 つ。

## 3. 要件と今の作りの対応（Requirement-to-Asset Map）

| 要件 | 使える今の作り | ギャップ | 種類 |
|---|---|---|---|
| 1.1〜1.3 上げる・戻す・履歴 | `FrameTime`・`VisualVersion` | 節ごとの更新の時刻の並び（決まった回数ぶんの輪）と状態（節・自動で上げた持ち主・入った時刻）の置き場。純粋な規則 | Missing |
| 1.4・1.8 材料はログに依らない | fill の `bumped` と `*wv != *w` | fill から判断の係へ「この回に更新された節」を渡す道 | Missing |
| 1.5 同じフレームは 1 回 | `FrameTime` は 1 フレームで 1 値 | 前に記した時刻と同じなら足さない（規則の中） | Missing（小） |
| 1.6 時刻の戻り | — | 規則の中で経過 0 扱い | Missing（小） |
| 1.9 毎フレーム全部を調べない | fill は今も毎フレーム全部の見える節を歩く | 判断は「この回に更新された節＋自動で上げた持ち主」だけ。fill の歩き自体は今のまま | Constraint |
| 2.1・2.2・2.6 対象と手動の優先 | `wants_own_surface`・`sync_surface_owner_parts` | 自動の状態を規則に入れる形。使い手の部品に触らない（内部の目印か状態の部品を wintf が持つ） | Missing |
| 2.3・2.4 上げない形と即時の戻し | `WorldVisual` の AABB・`visited` の前順・`MixMode`・`ClipShape` | 形の見分けの純粋な関数（§5）。戻しを同じフレームで効かせる係の順 | Missing／Unknown |
| 2.5 描画面を作れない | `deferred_surface_creation_system` の `error!` | 失敗を上げ下げへ返し、同じフレームで節のまま描く道 | Missing |
| 3.1 同じフレームで切り替え | PreRenderSurface の係の順 | WUC の Visual の作成が `update_draw_trees` より前（議題 D1） | Constraint／Unknown |
| 3.2 画素の一致 | WARP の足場（根 1 つ） | WUC の重ねを写す WARP の口。WUC の合成の丸めとの差（±1 か） | Missing／Research |
| 3.3 差分として扱う | 差分の描き直し（消えた・足された項） | ほぼ無し（既存で満たす見込み） | — |
| 4.1・4.2 節の下の持ち主を切らない | fill の訪れ | 切る代わりに、持ち主の位置（祖先の持ち主からの行列）・可視・不透明度の積・矩形の切り抜きを記す | Missing |
| 4.3・4.6・4.10 WUC の親と並び | `visual_hierarchy_sync_system`・`on_visual_graphics_remove` | 居るべき親を「いちばん近い祖先の持ち主」へ、並びを「節を通り抜けた前順」へ。`Children` の並びだけの変化を拾う | Missing |
| 4.4・4.5 位置と DPI | `visual_property_sync_system`（`offset × 拡大`） | 祖先の節の行列を重ねた位置を物理ピクセルへ。変化を拾うフィルタ（祖先の節の変化は本人の `Changed<Arrangement>` に出ない） | Missing |
| 4.7・4.8 可視・不透明度・矩形の切り抜き | `visual_property_sync`・`clip_sync` | 祖先の節の値を掛ける。WUC の切り抜きは 1 つの Visual に 1 つなので、自分の切り抜きとの交わりか、間の WUC の入れ物が要る。WUC の不透明度が子へ掛け算か層か | Missing／Research |
| 4.9 効かせられない形の `warn!` | `ReportedIssues`（続く間 1 度） | 種類を足す | Missing（小） |
| 5.1〜5.3 持ち主の `VisualTransform` | `Transform2D::to_affine`・`warn!` の場所 | `SpriteVisual.TransformMatrix`（Matrix4x4）へ `S⁻¹·(VT × T(offset))·S` を写す。`warn!` を消す | Missing |
| 5.4 回転・拡大の再標本化 | — | 許す範囲の決め（議題 5） | Constraint |
| 6 既定の寄せ方 | `draw/README.md` | 文書だけ | Missing（小） |
| 7 記録 | `ReportedIssues` の型 | 上げた・戻した・見送りの `debug!`。見送りの続く間 1 度 | Missing |
| 8 テスト | WARP の足場・実 GPU の足場・`log_capture_kit` | WUC の重ねの WARP の口。重ねた画面の実 GPU の確かめの手（下の Research） | Missing／Research |
| 9 文書・制約 | `draw/README.md`・`structure.md` 55 行目・`tech.md` 72 行目 | 書き足し。どのファイルも 1,000 行から遠い（最大 `builder_tests.rs` 689 行） | — |

**Research Needed**（設計の段へ）:
- R-a: WUC の `Visual.Opacity` が子へ掛け算で降りるか、層として平らにしてから掛かるか（DirectComposition には `SetOpacityMode` の Layer／Multiply が在る。WUC の既定は文書で確かめきれていない）。要件 4.8 の「不透明度の積」が D2D の層（節の不透明度）と同じ答えになる形を決める材料。
- R-b: WUC の合成（GPU の浮動小数の重ね）と D2D の `SOURCE_OVER`（入れ物の 8 ビットの α 掛け済み）の丸めの差。整数の位置・不透明度 1 なら ±1 に収まるか。WARP の写しの答えを実 GPU で 1 度確かめる手（下の R-d）。
- R-c: WUC の暗黙の確定で、同じ UI スレッドの 1 回の中の「描画面の `BeginDraw`/`EndDraw`」と「Visual の木の付け外し」が同じ画面の更新にそろって出るか（要件 3.1 の 0 フレームの切り替えの前提）。
- R-d: 重ねた後の画面の画素を `Cargo.toml` に触らずに読む手。候補は (1) WUC の Visual の性質（`Parent`・`Offset`・`TransformMatrix`・`Opacity`・`Children` の順）を読み戻して確かめる、(2) 表示した実の窓を GDI（`Win32_Graphics_Gdi` は在る）で画面から写す（画面の操作は排他テストの机の順を取る運用）。
- R-e: 小数の物理ピクセルの位置を「自分の描画面の中で端数だけずらして描く」形（議題 D3 の案 b）で、D2D の答えが親の描画面の中で描いたときとバイトか ±1 で揃うか（ラスタは画素の格子に対する端数にしか依らない見込み）。

## 4. 作り方の案

### 案 A: 今の係を広げる（ファイルを増やさない寄り）

- `fill_world_visuals` が節の下の持ち主を切らずに、祖先の持ち主からの行列・不透明度の積・可視・矩形の切り抜きを持ち主の部品へ書く。同じ関数で「この回に更新された節」の並びも返す。
- `update_draw_trees` の中で判断を行い、上げ下げの状態の部品を書く。
- `visual_hierarchy_sync_system`・`visual_property_sync_system`・`clip_sync_system` をそれぞれ節を通り抜ける形へ直す。
- ✅ 触る係が少ない・今の部品と順をそのまま使う。
- ❌ `update_draw_trees`（271 行のファイルの中の 120 行の係）と `fill_world_visuals` に役目が積もり、判断の純粋さとテストの分けが崩れやすい。係の順（議題 D1）の問題がこの係の中に閉じ込められ、2 段の切り替えを書くと読みにくくなる。

### 案 B: 新しい部品と係を足す（分ける寄り）

- 新しいファイル（例: `visual/promotion.rs` に純粋な規則と状態の部品、`visual/promotion_eligibility.rs` に上げてよい形の見分け、`visual/embedded_owner.rs` に節の下の持ち主の置き場の部品）と、兄弟のテスト。
- 新しい係: 判断の係（`update_draw_trees` の後に回り、fill が残した更新と形を読んで状態を進める）・節の下の持ち主の置き場を WUC へ写す係（Composition）。
- 節の下の持ち主の WUC の写し方に 2 通り:
  - B-1: 持ち主の Visual を祖先の持ち主の Visual の子に直に繋ぎ、重ねた行列・不透明度の積・矩形の交わりを持ち主に写す（WUC の Visual を増やさない。自分の切り抜きと祖先の切り抜きの交わりは両方が軸に沿う矩形の時だけ 1 つにできる）。
  - B-2: 持ち主へ至る道の上の節ごとに中身の無い WUC の `ContainerVisual` を作り、節の位置・不透明度・切り抜きをそれぞれに写す（`clip_sync` の作りをそのまま使え、祖先の性質の扱いが素直。WUC の Visual が増え、節が WUC の部品を持つ新しい形になる）。
- ✅ 判断と形の見分けが純粋な関数として独立し、要件 8.1・8.2 の真理値表のテストが書きやすい。係の順を組み直す余地がある。
- ❌ ファイルと係が増える。B-2 は「節は WUC の Visual を持たない」今の前提を変え、規模が L へ寄る。

### 案 C: 混ぜる（推す材料が多い形・決めは討議で）

- 純粋な規則・形の見分けは新しいファイル（案 B）。節の下の持ち主の行列などは `fill_world_visuals` が記す（案 A・歩きを 2 度しない）。WUC は B-1（直に繋ぐ）。
- 切り替えは「用意してから切り替える 2 段」:
  1. 上げると決めた回（フレーム N）: 状態を「用意中」にして 3 部品を入れる。親の描く列には残したまま（画面は節の形）、自分の Visual と描画面を作り、自分の木を描く。WUC の木には繋がない。
  2. 次の回（N+1）: 描画面が作れて描けていれば「上げた」へ進め、親の描く列から外し、WUC の木へ繋ぐ（親の描き直しと付ける操作が同じ回）。作れなかったら節のまま（要件 2.5）。
  - 戻すのは同じ回に効かせられる（fill が「戻す」の状態の実体を節として訪れ、命令で部品を外す。`on_visual_graphics_remove` が WUC の親から外す）。要件 2.4 の即時の戻しもこの道。
- ✅ 係の順を大きく組み替えずに 0 フレームの切り替え（要件 3.1）と作成の失敗（要件 2.5）を同じ作りで満たす。判断の遅れは 1 フレーム増えるが画素には出ない。
- ❌ 用意の回に描画面を余分に 1 度描く。状態が 3 つ（節・用意中・上げた）になり、規則の真理値表が増える。

## 5. 自動で上げてよい形（要件 2.3 の例の確かめ）

WUC の子の Visual は親の描画面の中身より必ず上に重なり、`SpriteVisual` の描画面は自分の大きさで切れ、親の描画面の外にも出られる。調べた結果、画素を変えずに上げられる形の条件の候補は次のとおり（要件の例に足すもの＝★）。

1. 同じ描画面の中で後に描く中身（前順で部分木より後の件）と、部分木の世界の外接矩形が重ならない（要件の例どおり）。
2. ★ **前順で前に在る手動の持ち主と重ならない**。全部を節にした形では、その持ち主は親の描画面の中身（上げる候補も含む）より上に在る。上げた Visual を前順で WUC の子に並べると、その持ち主より上に来て順が逆になる。代わりに「自動で上げた持ち主は手動の持ち主より下に並べる」とすると要件 4.6（前順）と食い違う（議題 D4）。
3. ★ **部分木の中に描画面の持ち主（手動）が無い**。在ると、その持ち主と兄弟の手動の持ち主との順が上の 2 と同じ理由で崩れる。
4. ★ **部分木の描く範囲が、自分の描画面の矩形（`Arrangement` の大きさ）と親の描画面の矩形と、祖先の節の矩形の切り抜きの中に収まる**（要件の「描画面に収められない」の中身）。外に出る分は節のときは親の描画面か切り抜きで切られ、上げると見えてしまうか切れてしまう。
5. ★ **持ち主から見た世界の行列が平行移動だけで、その量が物理ピクセルの整数**（DPI の拡大を掛けた後）。小数の位置は WUC の再標本化で、回転・拡大は D2D のラスタと WUC の標本化の違いで画素が変わる（議題 D3）。
6. 祖先の節に 1 未満の不透明度・図形の切り抜きが無い（要件の例どおり。矩形の切り抜きは 4 で中に収まれば効かないのと同じ）。
7. 部分木に `SOURCE_OVER` でない混ぜ方の件が無い（要件の例どおり）。
8. ★ **自分の `Visual.clip` が無いか、物理ピクセルの整数の矩形**。持ち主の切り抜きは WUC が受け持ち、角丸は WUC と D2D の縁のにじみが違い、図形の切り抜きは WUC では効かない（`clip_key_for` が「切り抜きなし」に読み替える）。
9. 自分の不透明度（1 未満でも）は、自分の描画面に部分木が平らに描かれ WUC の Visual に子が無いので、D2D の層と同じ意味になる（±1 の見込み・R-b）。条件にしなくてよい見込み。

条件の 1・2・4 は世界の外接矩形の交わりで決まり、fill の `visited` と `WorldVisual` から純粋に出せる（要件 8.2 の純粋なテストに載る）。にじみの余白（`diff.rs` の範囲の外向きの丸め）を同じく足すかは設計で確かめる。

## 6. 規模と危うさ

- **規模: M〜L**（要件をそのまま全部作ると L の下端・14〜18 タスク）。
  - 見込みのタスクの数: 純粋な規則と状態 2・上げてよい形の見分け 1〜2・節の下の持ち主の置き場（fill）1〜2・WUC の親と並び 1〜2・位置と `TransformMatrix` と不透明度と可視 1〜2・祖先の矩形の切り抜き（要件 4.8）1〜2・切り替えの係と配線（2 段・即時の戻し・作成の失敗）2〜3・記録 1・WARP の重ねの口と画素の一致のテスト 2・実 GPU のテスト 1・既存のテストと文書 1。
  - M（10〜15）に収める寄せ方の候補: 手動の持ち主の祖先の矩形の切り抜きと不透明度の積（要件 4.8）を要件 4.9 と同じく `warn!` で保留にする（−2）。回転・拡大を含む手動の持ち主の `VisualTransform`（要件 5.4）を平行移動と拡大だけにする（−1）。自動で上げるのを平行移動・整数の位置だけにする（§5 の 5・案 b の端数の描き込みを後へ）（−1）。
- **危うさ: Medium〜High**。
  - High の訳: 0 フレームの切り替えが係の順と WUC の確定のまとまり（R-c）に依り、確かめが実 GPU と画面の画素に及ぶ（R-d）。WUC の不透明度の意味（R-a）が文書で確かめきれていない。
  - Medium へ下げる材料: 案 C の 2 段の切り替えなら係の順を組み替えない。§5 の条件を厳しく取れば画素の一致は WARP で決定論に固められる。新しい描き方の使い手は wintf の中だけなので、外の描き手の見た目に響かない。

### 主な危うさ

- R1（効きの薄さ）: §5 の 5 を厳しく取ると、dola で小数の位置へ動く部品と拡大 1.25 の窓では上げられない回が多く、自動の昇格が主に「位置の変わらない中身の差し替え（まばたきのコマ）」にしか効かない。
- R2（行ったり来たり）: 形の見分けで戻す（要件 2.4 は履歴を待たない）回と、頻度で上げる回が交互に来ると、履歴の効かない往復が起きうる。即時の戻しの後にも上げの側の履歴を掛けるか（議題 D6）。
- R3（子孫の一斉の昇格）: 祖先の動きで子孫の世界の行列が全部変わるので、材料の定義のままだと子孫が全部同時に上がる（議題 D5）。
- R4（静かな間の戻しの遅れ）: tick が間引かれるので、戻しは最大で心拍 1 周期ぶん遅れる（画素は変わらず GPU のメモリだけ遅れて返る）。
- R5（組み直しの手間）: 上げ下げのたびに `TreeEdits` が全部の根を組み直す。窓の数・根の数が増えると目立ちうる（描き直しは差分で済む）。
- R6（当たり判定とのずれ）: 持ち主の `VisualTransform` が見た目に効くようになると、当たり判定（範囲外・今の作りのまま）は `GlobalArrangement` のままなので、回転・拡大した持ち主は見た目と当たりがずれる。節の `VisualTransform` が当たり判定に効いているかも含め、範囲外として文書に書くか（議題 D9）。
- R7（並びの変化を拾わない既存の欠け）: `visual_hierarchy_sync_system` は `Children` の並びだけの変化で並べ直さない。要件 4.5・4.6 を満たすには、節の並びの変化も拾う必要があり、手動の持ち主の今の振る舞いにも触れる。

## 7. 設計の段への申し送り

- 推す材料が多いのは案 C（純粋な規則と形の見分けは新しいファイル・節の下の持ち主の置き場は fill が記す・WUC は直に繋ぐ・2 段の切り替え）。決めは討議で。
- 触るファイルの見込みは brief どおり（`visual/{mod,systems,world_visual}.rs`・`draw/emit_test_support.rs`（か兄弟の新しい足場）・`systems/{visual_sync,clip_sync}.rs`・`tests/graphics/` の下）。`systems/clip_sync.rs` と `systems/surface.rs`（作成の失敗を返す道）が brief の一覧に無いので、ウェーブの「触らない約束」の確かめに足す。`render.rs` は触らない見込み。
- `Cargo.toml` には触らずに済む見込み（`SpriteVisual.TransformMatrix`・`ContainerVisual` は今の機能 `UI_Composition` の中）。重ねた画面の画素を `Windows.Graphics.Capture` で読む手は機能を足すので取れない。

## 8. 討議に送る設計の判断の項目

> 2026-11-16 要件の討議で振り分け済み: D3〜D7・D9〜D12 は requirements.md の「討議の確定」で決めた（D9 は文書の要件 9.1 へ）。D1・D2・D8 と B-1／B-2・R-a〜R-e は設計の段で決める。

- **D1 切り替えの係の順**: 同じフレームで切り替えるために、(a) 用意してから切り替える 2 段（案 C）、(b) PreRenderSurface の中の係の順を組み替え、`update_draw_trees` の中で上げた実体の WUC の Visual と描画面をその回に作る、のどちらか。
- **D2 自動の状態の置き場**: 規則 `wants_own_surface` の 4 つ目の入力にするか、wintf の内部の状態の部品（使い手は付け外ししない）で持つか。要件 2.6（使い手の部品に触らない）と両立させる形。
- **D3 画素の一致の厳しさと小数の位置**: (a) 平行移動・物理ピクセルの整数だけを上げる（厳しい・動く部品に効きにくい）、(b) 端数を自分の描画面の中でずらして描き、WUC の位置は整数にする（一致を保ったまま動く部品にも効く。端数が変わる回は自分の描画面を描き直す。余白 1 ピクセル）、(c) 小数の位置の再標本化の違いを許す（要件 3.2 を緩める）。
- **D4 自動で上げた持ち主の WUC の並び**: 前に在る手動の持ち主と重なる節を上げない（§5 の 2・要件 4.6 の前順のまま）か、自動で上げた持ち主を手動の持ち主より下に並べる（4.6 を自動の分だけ変える）か。
- **D5 「節の更新」を誰に数えるか**: 祖先の変形の変化で子孫の行列が変わった回を、子孫に数えず変形を持つ節（局所の変形か配置が変わった節）にだけ数えるか。同じ回に祖先と子孫が両方上がる答えになったら祖先だけを上げるか。上げた持ち主の部分木の中の節の更新を、その持ち主の「更新」に数えるか（戻しの条件）。
- **D6 即時の戻しの後の履歴**: 形で戻した（要件 2.4）後も、上げの側の履歴（要件 1.3）を掛けて往復を防ぐか。
- **D7 §5 の条件の採否**: ★の 5 つ（前の手動の持ち主との重なり・部分木の中の持ち主・描画面と親と祖先の切り抜きに収まること・整数の平行移動・自分の切り抜きの形）を要件 2.3 の例に足すか。にじみの余白を交わりの判定に足すか。
- **D8 静かな間の戻し**: 心拍で拾う（遅れは最大約 0.5 秒・何も足さない）か、`tick_wake::arm_deadline` で戻す時刻に起こすか。
- **D9 当たり判定**: 持ち主の `VisualTransform` が見た目に効いた後も当たり判定を今のままにすることを、範囲外として `draw/README.md` に書くか。
- **D10 規模の寄せ方**: 要件 4.8（手動の持ち主の祖先の矩形の切り抜きと不透明度の積）と要件 5.4（回転・拡大の手動の持ち主）を本 spec で作るか、`warn!` で保留にして M に収めるか。作るなら WUC の写し方を B-1（直に繋ぐ・切り抜きは交わり）か B-2（道の上の節ごとに WUC の入れ物）か。
- **D11 実 GPU の確かめの形**: 要件 8.5 を WUC の Visual の性質の読み戻し（親・位置・行列・不透明度・並び）と描画面ごとの画素（`CopySurface`）で満たすとするか、表示した窓の画面の画素まで読むか（画面の操作の机の運用が要る）。
- **D12 並びの変化**: `Children` の並びだけが変わった回に WUC の並べ直しをする作りを本 spec で足すか（手動の持ち主の今の振る舞いにも効く既存の欠け）。

参考（R-a）: [IDCompositionVisual2::SetOpacityMode](https://learn.microsoft.com/en-us/windows/win32/api/dcomp/nf-dcomp-idcompositionvisual2-setopacitymode)・[LayerVisual Class](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.layervisual)・[Composition visual](https://technet.microsoft.com/en-us/windows/uwp/graphics/composition-visual-tree)

---

# 設計の段（2026-11-16 `/kiro-spec-design -y`）

## Summary
- **Feature**: `wintf-surface-auto-promotion`
- **Discovery Scope**: Extension（wintf の描画の木の拡張・発見の手順は light。外の依存は足さない）
- **Key Findings**:
  - 自動で上げた持ち主は自分の埋める係を回さず、親の持ち主の走査の列（前順で連続）の部分を、原点を整数の物理ピクセルへずらして描けばよい。世界の行列は手動の持ち主からの DIP の 1 つに保て、判断の材料・形の見分け・部分の列が 1 度の歩きでそろう。
  - 判断は `update_draw_trees` の中（埋めた直後）で行い、切り替えは「用意してから切り替える 2 段」にすれば、係の登録（`ecs/world/mod.rs`）を変えずに、どの回の画面でも部分木が欠けず 2 度描かれない。
  - 今の WUC の兄弟の並びは「`Children` の順に `InsertAtBottom`＝最初の子が最上」（`systems/visual_sync.rs` の注記・`tests/visual/common/mod.rs` の期待）で、節の描く列（画家の順・最初の子が最下）と向きが逆。要件 2.3 の例「前順で前の手動の持ち主と重なる」はこの向きを画家の順と見た書き方で、画素を変えるのは前順で後の持ち主・ほかの自動の持ち主との重なり。

## Research Log

### D1 同じフレームの切り替え
- **Context**: WUC の Visual を作る係が `update_draw_trees` より前に回る（ギャップ分析 §1）。
- **Findings**:
  - `sync_surface_owner_parts`（先頭）→ `visual_resource_management_system` → `update_draw_trees` → `deferred_surface_creation_system` の間は命令が反映される（今も「入れた部品は同じ回に作られる」）。
  - 判断の材料のうち頻度は変化の印から先頭でも取れるが、形の見分け（要件 2.4 の「その形になったフレーム」）はこの回の世界の行列が要り、埋めた後にしか分からない。
  - 描画面の作成の失敗は `update_draw_trees` の後にしか分からない。判断の回に親の列から外すと、失敗の回に部分木が 1 回欠ける。
- **Implications**: 2 段（Node→Preparing→Active）。用意中は親の列に残し、自分の描画面を描くが WUC の木に繋がない。描画面ができた次の回に親の列から外すことと WUC へ繋ぐことを同じ回に行う。戻しは判断の回に効く（親の列に戻し、Composition で外す）。判断から切り替えまでの回の遅れは画素に出ない（「1 フレーム遅らせて合わせる」解ではない）。係の順の組み替えは `ecs/world/mod.rs` に触るので取らない。

### D2 自動の状態の置き場
- **Findings**: 節は必ず `WorldVisual` を持つ（`VisualDrawContent`／`VisualTransform` の `require`）。`WorldVisual` に `#[require(AutoPromotion)]` を付ければ、命令で部品を足す揺れ（アーキタイプの移り）が無く、実体が消えれば状態も消える（`Local<HashMap>` の掃除が要らない）。
- **Implications**: 相は規則 `wants_own_surface` の 4 つ目の入力。履歴の書き込みは変化の印を立てず、相の移りだけ印を立てて `sync_surface_owner_parts` に拾わせる（`clip_sync` の `bypass_change_detection` の前例）。

### D8 静かな間の戻し
- **Findings**: 心拍は `TICK_HEARTBEAT_FRAMES = 30`（`ecs/world/tick_gate.rs`）。心拍の回にも `update_draw_trees` は回り、埋める係は全部の見える節を歩く。
- **Implications**: 心拍で拾う。戻しの遅れ（最大で心拍 1 周期）は画素に出ず、GPU のメモリを返すのが遅れるだけ。`tick_wake::arm_deadline` の預けは足さない。

### WUC の写し方（B-1／B-2）
- **Findings**: B-1 は WUC の Visual を増やさず、装置の失いの立ち直りに新しい物を足さない。祖先の矩形の切り抜きは、自分の切り抜きが無ければ `CompositionGeometricClip` の矩形＋`CompositionClip.TransformMatrix`（逆行列）で 1 つにでき、自分の切り抜きが矩形で行列が軸に沿えば交わりの矩形にできる。それ以外は 1 つの切り抜きに合わせられない。B-2 は道の上の節ごとに `ContainerVisual` を作れば全部の形を素直に効かせられるが、節が WUC の部品を持つ新しい形になり、作成・失いの立ち直り（`visual_manager.rs`・`components.rs` は触らない約束）まで要る。
- **Implications**: B-1。合わせられない形は `warn!` を 1 度（設計の前提 C）。

### R-a WUC の不透明度
- **Sources**: [LayerVisual Class](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.layervisual)・[Composition visuals](https://learn.microsoft.com/en-us/windows/apps/develop/composition/composition-visual-tree)
- **Findings**: 子を 1 枚の層に平らにするのは `LayerVisual`（「子を 1 つの層へ平らにする」）で、ふつうの `ContainerVisual`／`SpriteVisual` の `Opacity` は層にしない（子へ掛け算で降りる）。
- **Implications**: 節の下の手動の持ち主の不透明度は「自分 × 祖先の節の積」を写す（討議の確定 3 の「積」と同じ。重なる子孫どうしでは層と答えが違う）。自動で上げた持ち主は不透明度を D2D の層で自分の描画面に描くので WUC には 1 を写し、層の意味のまま。

### R-b 丸めの差
- **Findings**: 自動で上げた形の違いは、親の描画面の中の D2D の `SOURCE_OVER`（8 ビットの α 掛け済み）と、WUC の重ね（GPU の浮動小数）の差だけ。不透明度の層も部分の列の中の D2D の層なので、層の答えは同じ。
- **Implications**: 許しは ±1（討議の確定 2）。WARP の写しは WUC の重ねを D2D で写した近似で、実 GPU の重ねた画面は読まない（討議の確定 6）。

### R-c 同じ回の確定
- **Findings**: 13 本の予定は `try_tick_world` の 1 回の中で UI スレッドの上で続けて回る。WUC の暗黙の確定はスレッドが呼び出しから戻った後にまとめて出るので、同じ回の描画面の描き込みと Visual の木の付け外しは同じ画面の更新に出る。wintf は今も、持ち主を動かして親を描き直す回をこの前提で回している。
- **Implications**: 2 段の切り替えの「外す・繋ぐ・親を描き直す」を同じ回に行えば足りる。文書で確かめきれない点として危うさに残す（実 GPU の重ねた画面は読まない）。

### R-d 実 GPU の確かめ
- 討議の確定 6 で決まり（WUC の性質の読み戻しと `CopySurface`）。

### R-e 端数を自分の描画面の中でずらす形
- **Findings**:
  - D2D のラスタ（にじみの被覆・ビットマップの補間の標本の位置・文字の位置）は、装置の座標の画素の格子に対する端数で決まる。部分の列が世界の行列から原点を引けば、装置の座標が親の描画面の中のちょうど整数ずれた値になり、同じ画素の答えを整数ずらしたものになる見込みだった。
  - 浮動小数の誤差（原点の DIP は 2 進で割り切れない値になりうる）で装置の座標が 1e-6 ピクセルほどずれうる。被覆の量子化の境にちょうど乗った所だけが変わりうるが、許し ±1 の内、と見込んだ。**この見込みは測って外れた**: 原点 `floor(拡大 × 位置) / 拡大` を DIP で引くと、f32 は `px / 拡大` を持てず（10.3 − 7.2f = 3.1000004 → 装置の座標 3.8750005、全体の木では 12.875）、にじみのある楕円が拡大 1.25 で最大 16（5 画素）ずれた。根の直し: 整数の原点を物理ピクセルのまま `scale(s,s) * translation(-px)` で世界の行列と外接矩形から引き、部分の列を持ち主の描画面の物理ピクセルの空間にする（切り抜き・層の範囲・`hash_draw` の鍵もこの空間・描き直しは恒等の根の変換）。直した後は 10.3・7.6 DIP の端数のある子を含む矩形・楕円・線形・縮小の HIGH_QUALITY_CUBIC が拡大 1.0・1.25・1.5 の全画素で ±1 に揃った。
  - 描画面の大きさは配置の大きさの切り上げなので、端数で右と下へ最大 1 ピクセル出る。自動で上げた持ち主の描画面に 1 ピクセルの余白を足す（`systems/surface.rs`）。
- **Implications**: 案 b を取り、WARP のテストで ±1 に揃うことを確かめた。条件は原点を物理ピクセルの整数で引くこと。落とし（案 a・`FractionalOrigin`）は使わない（`INTEGER_POSITION_ONLY = false`）。

### WUC の変形の順
- **Sources**: [Visual.TransformMatrix](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.visual.transformmatrix)・[Visual Class](https://learn.microsoft.com/en-us/uwp/api/windows.ui.composition.visual)
- **Findings**: `TransformMatrix` と `Offset` を掛ける順は文書に書かれていない。
- **Implications**: 平行移動だけなら `Offset`（今のまま）、それ以外は `TransformMatrix` に全部を入れて `Offset` を 0 にする。どちらの順でも答えが同じになる。

### 重なりの順
- **Findings**: `visual_hierarchy_sync_system` は `Children` の順に `InsertAtBottom` するので最初の子が最上（`tests/visual/common/mod.rs` の「Children の逆順になる」）。節は画家の順。全部を節にした形では、候補の節は親の描画面の中に在り、同じ親の持ち主の下の WUC の子（手動の持ち主）全部より下。
- **Implications**: 要件 4.6 どおり自動で上げた持ち主を前順の位置に並べると、前順で後の手動の持ち主とほかの自動の持ち主との上下が全部を節にした形と逆になる。要件 2.3 の統べる文でこれらと重なる候補も上げない形にした（設計の前提 B）。一番下に並べる形なら条件が要らないが、討議の確定 2（D4）を変える。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|---|---|---|---|---|
| 部分の列（採用） | 自動の持ち主の描画面は親の走査の列の部分を原点をずらして描く | 世界の行列が 1 つ・歩きが 1 度・持ち主自身の切り抜きと層と変形も節と同じ命令 | 根の木を組み直す回に部分も組み直す | 案 C の「埋める係が記す」を広げた形 |
| 自動の持ち主も自分で埋める | 今の手動の持ち主と同じく根として歩く | 今の作りに近い | 世界の行列が根ごとに別になり、形の見分けとの行き来が要る。持ち主の切り抜きと不透明度を WUC が受け持ち、角丸の縁と層の意味が D2D と違う | 取らない |
| 係の順の組み替え（D1 の b） | 判断の回に WUC の Visual と描画面を作る | 判断の回に切り替わる | `ecs/world/mod.rs` に触る。作成の失敗の回に 1 回欠ける | 取らない |
| B-2 | 道の上の節ごとに WUC の入れ物 | 切り抜きと不透明度の全部の形 | 節が WUC の部品を持つ・立ち直り | 討議で必要なら |

## Design Decisions（総合）

### 一般化
- 「節の下の手動の持ち主」と「自動で上げた持ち主」は、どちらも「親の持ち主の WUC の Visual の子として、走査の列の位置と祖先の積み重ねから写す持ち主」で、置き場の部品 `SurfacePlacement` 1 つ（`Embedded`／`Auto`）にまとめた。直の子の持ち主は置き場を持たず今の写し方＋自分の `VisualTransform`。
- 親の決め（`wuc_host_of`）と子の並び（`wuc_children_order`）は、節と用意中の自動の持ち主を通り抜ける 1 つの規則で、手動・自動・直の子の全部に効く（要件 4.11 の並びだけの変化も同じ道）。

### 作るか取り入れるか
- `LayerVisual` は使わない（自動の持ち主の層は D2D が描く・手動の持ち主は積の意味で討議済み）。
- `Windows.Graphics.Capture` は使わない（`Cargo.toml` に触る・討議の確定 6）。
- 判断の規則は自前（履歴つきの小さな状態機械・外の部品で足りるものが無い）。

### 削ったもの
- 新しい係は足さない（今の 6 つの係の中で呼ぶ）。
- WUC の入れ物（B-2）を作らない。
- 閾値の設定の口・止める口・上限は作らない（討議の確定 7）。
- `tick_wake::arm_deadline` の預けは作らない（D8）。

## Risks & Mitigations
- 効きの薄さ（R1）: 手動の持ち主との重なり（前・後）と、平行移動・整数の矩形の切り抜きの条件で、上げられる場合が狭い — 記録の見送りの理由で数え、討議で前提 B・D を緩めるかを決める。
- 2 段の切り替えの重さ: 用意中の回に自分の描画面を余分に描く（2 回ぶん）— 上げるのは頻繁に変わる節だけなので、描き直しはどのみち起きている。
- 同じ回の確定（R-c）を実 GPU で直に確かめられない — 今の手動の持ち主の動きと同じ前提。WARP で全部の回の画素を固める。
- 組み直しの手間（R5）: 上げ下げのたびに全部の根を組み直す — 描き直しは差分で済む。上げ下げの数は履歴（3 秒）で抑える。
- 当たり判定のずれ（R6）: 文書に書く（要件 9.1）。

## 参照した skill
- `kiro-spec-design`（本手順）と、その規則（design-principles・design-discovery-light・design-synthesis・design-review-gate）。描画・UI の別の skill は当てはまらない（wintf の中の描画の作り）。


> 2026-11-16 設計の討議: 開発者が全 6 議題を推奨案で承認した（描画面の矩形は部分木の描く範囲から・2 段の切り替えと要件 3.1 の書き直し・自動で上げた持ち主は WUC の子の一番下・平行移動だけと整数の矩形の切り抜きの条件を外す・定数はそのまま・B-1／心拍／案 b／記録の語）。決めの全文は design.md の「設計の討議の確定」。
