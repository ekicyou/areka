# ギャップ分析: wintf-d2d-composite-rules

> 2026-11-13 `/kiro-validate-gap`（作業木 `claude/kiro-start-wintf-d2d-composite-rules-c739fd`・基点 main `4863cb66` の上の `e5a88cb6`）。要件は `requirements.md`（生成済み・未承認）。ここに書くのは情報と選択肢で、決めではない。

## 1. 分析の要約

- **土台はほぼ揃っている**: 描画の木（`ecs/graphics/visual/`）が節ごとの重ね方・補間・変形・矩形と図形の切り抜きをもう描き、WARP の足場（`visual/draw/emit_test_support.rs`）は「的のビットマップ → CPU_READ のビットマップへ `CopyFromBitmap` → `Map`」の同期の読み戻しをすでに持つ。1 ピクセルの読み戻しは、この D2D だけの手順を 1×1 にすれば足りる（D3D の staging は要らない）。
- **足りないもの**: ブレンドの欄と描き方（`ID2D1DeviceContext6::BlendImage`）・節の不透明度の層（今は `world_visual.rs` 181 行目が `warn!` して 1.0 で描く）・図形の切り抜きの層の旗（今は `D2D1_LAYER_OPTIONS1_NONE` 固定）・描画面ごとの入れ物（今は WUC の `BeginDraw` の DC へ直接描く）・読み戻しの口。どれも wintf の中で閉じ、新しい依存は要らない（`windows` 0.62.2 に `ID2D1DeviceContext6`・`D2D1_BLEND_MODE`〔0〜25 の 26 種〕・`D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND`・`D2D1_PRIMITIVE_BLEND_COPY`・`CLSID_D2D1Blend` が在ることを確かめた）。
- **最大の不確かさは要件 3.1**: 図形の切り抜きの中で `SOURCE_OVER` でない重ね方とブレンドを「下の画素」に効かせる道は、D2D の文書では「層を背景から始める（`INITIALIZE_FROM_BACKGROUND`）＋ 層を積むときの基本の混ぜ方を `COPY` にする（層を降ろすとき、図形の覆いの割合で層と的を 4 チャネルとも線形に補間する）」の組み合わせになる。文書の書き方は短く、降ろすときの式・旗を見る時点（積むときか降ろすときか）・WARP と実 GPU で同じかは書かれていない。**設計の前に WARP で小さく試す（スパイク）のが要る**。
- **ブレンドの口は事実上 `BlendImage` 一択**: ブレンドの効果（`CLSID_D2D1Blend`）は下の画素を入力の絵として渡す必要があり、そのためには描画の途中で的を写す（`Flush` が要る）。D2D は層を積んだままの `Flush` を誤りの状態にする（Layers Overview）ので、図形の切り抜き・不透明度の層の中では効果の道が使えない。`BlendImage` は Windows 10 1903（19H1）以降。
- **規模と危険**: 規模 L（1〜2 週・brief の見立て 11〜15 タスクと合う）。危険は中〜高（要件 3.1 の D2D の振る舞いが文書で確定しない・入れ物の写しで要件 4.2 のビット一致を保つ必要）。

## 2. 今のコードの調べ

### 2.1 描画の道（main の今）

| 場所 | 今の振る舞い | 本 spec との関係 |
|---|---|---|
| `crates/wintf/src/ecs/graphics/systems/render.rs`（153 行） | `render_surface` が `Changed<SurfaceGraphicsDirty>` の描画面ごとに WUC の `BeginDraw(None)` → 拡大＋offset の変換 → 透明で塗る → 根の命令の列を `DrawImage(LINEAR, SOURCE_OVER)`（130〜136 行目）→ 木を `emit` → `EndDraw` | 入れ物へ合成して写す形へ変える中心（要件 4.1・4.8）。`BeginDraw` の失敗の道（失いなら `note_draw_failure` で抜ける・それ以外は印を立て直して次の回）は要件 4.6・4.7 がそのまま流用できる。`EndDraw` の失敗は今は記録だけ（失いの判定をしない）＝入れ物でも同じ形にするかは設計で |
| `crates/wintf/src/ecs/graphics/visual/draw/emit.rs`（65 行） | 列を DC へ再生。`Draw`＝`DrawImage(item.interpolation_mode, item.composite_mode)`。`PushClipRect`＝`PushAxisAlignedClip`。`PushClipGeometry`＝`PushLayer`（`contentBounds` は ±f32::MAX・`opacity 1.0`・`D2D1_LAYER_OPTIONS1_NONE`）。14〜15 行目に「層は透明から始まる・変えるのは `wintf-d2d-composite-rules`」 | ブレンドの描き方（`BlendImage`）・不透明度の層・図形の切り抜きの層の旗を足す場所。引数は `&ID2D1DeviceContext` なので、`BlendImage` のために `ID2D1DeviceContext6` を渡す形へ広げる |
| `crates/wintf/src/ecs/graphics/visual/draw/command.rs`（52 行） | 公開の `DrawCommand`（`Draw`／`PushClipRect`／`PopClipRect`／`PushClipGeometry`／`PopClipGeometry`）と `DrawItem`（`composite_mode` の欄）。欄は `wintf-visual-damage-redraw` が読むので公開 | 不透明度の層の push／pop の命令と、`DrawItem` のブレンドの欄が要る |
| `crates/wintf/src/ecs/graphics/visual/draw/builder.rs`（265 行） | 前順の訪れた列を画家の順の列にする。切り抜きの計画（矩形は軸に沿えば `PushClipRect`・回転なら単位矩形の図形へ上げる・角丸と任意の図形は `PushClipGeometry`）。`hash_draw` は実体・版・補間・重ね方・世界の行列から | 不透明度の層の push／pop を切り抜きと同じ「深さで閉じる」仕組みに乗せられる。`hash_draw` にブレンドを足す |
| `crates/wintf/src/ecs/graphics/visual/draw_content.rs`（67 行） | 公開の `VisualDrawContent { local_aabb, command_list, interpolation_mode, composite_mode }`・`new`／`new_ex` | ブレンドの欄を置く場所。要件 1.5（ブレンドと重ね方を同時に効かせない）を型で表すなら、`composite_mode` を「重ね方かブレンドか」の列挙へ置き換える案がある。使い手は wintf の中（`visual/`・`systems/window_pos.rs`）と `examples/visual_tree_demo.rs`・`tests/graphics/{draw_tree_systems_test,window_pos_systems_test}.rs` だけ（emo は使っていない）＝型を変えても直す所は少ない。ただし `wintf` は crates.io に出す公開のライブラリなので公開の型が変わる（0.x の版） |
| `crates/wintf/src/ecs/graphics/visual/world_visual.rs`（231 行） | `fill_world_visuals`。177〜183 行目が節の `opacity != 1.0` で `warn!`（要件 2.6 で消す）。`is_visible` が偽なら版だけ追って描かない | 不透明度 0 を「見えない」と同じ扱い（版は追う・描かない）にできる（要件 2.2）。`visual.is_changed()` で版が上がり、列の組み直し → `Changed<DrawTree>` → 描き直し、の道がすでに在る（要件 1.8・2.7 は新しい欄をこの道に乗せれば足りる） |
| `crates/wintf/src/ecs/graphics/visual/mod.rs`（214 行） | `Visual.opacity`・`set_opacity`（範囲外は `warn!` してクランプ）・`clamped_opacity`（`opacity.clamp(0.0, 1.0)`）。型の文書に「節の不透明度は保留」 | 要件 2.4 の「`clamped_opacity` の答え」は、Rust の `f32::clamp` が NaN をそのまま返すので **NaN は NaN のまま**。描画面の持ち主は WUC の `SetOpacity(NaN)` に渡している（`systems/visual_sync.rs` 310〜316 行目）。節で D2D の層の `opacity` に NaN を渡したときの答えは未定義＝要件 2.4 の「NaN の扱いも同じ」をどう読むかは確かめが要る（下の Research Needed） |
| `crates/wintf/src/ecs/graphics/systems/surface.rs`（360 行） | `deferred_surface_creation_system` が描画面を作る・大きさが変われば作り直す（`set_surface`）。`cleanup_surface_on_commandlist_removed` が `SurfaceGraphics::clear()` | 入れ物の作成・作り直し・放す時点（要件 4.3・4.5）。入れ物を `SurfaceGraphics` の欄に持てば、`clear()`・`set_surface` と寿命が自然に揃う |
| `crates/wintf/src/ecs/graphics/components.rs` | `SurfaceGraphics`（描画面・筆・大きさ）・`SurfaceGraphicsDirty` | 入れ物の置き場の第一候補 |
| `crates/wintf/src/ecs/graphics/core.rs` | `GraphicsCore`（D3D11・D2D の工場と装置・**共有の DC** `device_context()`・`note_draw_failure`・`is_valid`）。共有の DC は UI スレッドで命令の列の記録（`SetTarget(command_list)`）に使われている | 入れ物へ合成する DC に共有の DC を使うか、専用の DC を持つか（設計）。装置の失いの合図は `Res<GraphicsCore>` の変化の印（`structure.md` の「下流の控え」） |
| `crates/wintf/src/com/wuc.rs`（253 行） | `begin_draw` が `ID2D1DeviceContext` を受けて `ID2D1DeviceContext3` へ cast して返す | 入れ物を使えば WUC の DC は「入れ物を写すだけ」になる。D3D の写し（`BeginDraw` で `IDXGISurface` を受けて `CopySubresourceRegion`）を選ぶなら、ここに口が増える |
| `crates/wintf/src/com/d2d/mod.rs`（158 行） | DC の拡張 trait（`clear` など）。brief の言う補助 `draw_image` は**もう無い**（#429 で消えた） | ブレンドの口（`ID2D1DeviceContext6` への cast）を包む候補の場所 |

### 2.2 先例

- **同期の読み戻し（D2D）**: `crates/wintf/src/ecs/graphics/visual/draw/emit_test_support.rs` の `draw_to_pixels` が、`D2D1_BITMAP_OPTIONS_TARGET` の的 → `D2D1_BITMAP_OPTIONS_CPU_READ | CANNOT_DRAW` のビットマップへ `CopyFromBitmap` → `Map(READ)` → 行を詰めて写す → `Unmap`。1 ピクセルの読み戻しは、写し元の矩形を 1×1 にした同じ手順で書ける。`Map` は GPU の仕事が終わるまで待つ＝要件 5.2 の「終えるまで待ってから読む」はこれで満たせる。
- **同期の読み戻し（D3D）**: `crates/areka-emo-text/src/surface.rs` が `CopyResource(staging, front)` → `Map(READ)`（待つ）と `Map(READ, DO_NOT_WAIT)`（待たない）を持つ。D3D の道を選ぶならこれが型になる。
- **入れ物の先例**: 同じ `areka-emo-text/src/surface.rs` が「自前のテクスチャ → `CopyResource` → `Present`」の順を持つ（brief の再測定の記述どおり）。
- **一度だけの記録**: `world_visual.rs` の `reported.first_seen(e, TreeIssue::…)` が「1 度だけ `error!`」の仕組み。要件 1.7 の「1 度記録」に流用できる。
- **WARP の足場と許容差**: 工場と装置をプロセスで 1 度だけ作る・96 DPI・見本はコードの中の値・1 チャネル ±1（`pixel_near`）。本 spec のテストはこれをそのまま使える（要件 7.1・7.2）。
- **実 GPU のテスト**: `crates/wintf/tests/graphics/draw_tree_systems_test.rs` が `GraphicsCore::new()` と描画面の作成・装置の失いからの立ち直りを実 GPU で確かめている。wintf には画面を撮るテストは無い＝要件 7.10 の「画面に出るか」は、入れ物を通した描画面の `BeginDraw`／`EndDraw` が通ることと、読み戻しの口の値で確かめる形が現実的（設計で決める）。

### 2.3 規約と制約

- 1 ファイル 1,000 行（本番・テストとも）。`emit_tests.rs` 498 行・`builder_tests.rs` 577 行・`world_visual_tests.rs` 572 行＝足すテストは新しい兄弟ファイル（`<stem>_<theme>_tests.rs`・共有の補助は `<stem>_test_support.rs`）へ。
- 本番ファイルにテストの本体を書かない（`structure.md` のテスト分離）。
- 失敗は `error!`（実体・HRESULT）＋戻り値。panic は致命のときだけ（`logging.md`・`areka-log-first-no-silent-failure`）。
- 「1 フレーム遅らせる解は取らない」「時刻は正確に扱う」＝読み戻しは同期。
- `tech.md` の Testing の WARP の例外は今「描画の木の画素の答え（`emit_tests.rs`）だけ」（72 行目・90 行目）＝要件 8.1 で広げる。
- 支える Windows の下限は `tech.md` 106 行目に「Windows 10/11（Windows.UI.Composition 対応）」、`dist/README.txt` に「Windows 10／11 の 64 ビット版専用」とだけ在り、版の番号は書かれていない。今のコードが使う API から読める実効の下限は Windows 10 1803（`DesktopWindowTarget` の `ICompositorDesktopInterop`）。

## 3. 要件と資産の対応

凡例: **在る**＝そのまま使える／**足す**＝今の形に足す（Missing）／**未知**＝確かめが要る（Unknown）／**制約**＝守るべき枠（Constraint）

| 要件 | 使える資産 | ギャップ |
|---|---|---|
| 1.1・1.2 ブレンドの 26 種 | `D2D1_BLEND_MODE`（0〜25）・`ID2D1DeviceContext6::BlendImage`（`windows` 0.62.2 に在る） | **足す**: `VisualDrawContent`・`DrawItem` の欄、`emit` の分岐、DC6 への cast |
| 1.3 外形の外を変えない | ブレンドの式は絵の α が 0 の所で下の画素をそのまま返す（W3C 型の式・D2D のブレンドの効果も同じ） | **未知**: `BlendImage` が絵の外を実際に触らないか（`DrawImage` の非 `SOURCE_OVER` は絵を切り抜きまで透明で広げる＝`draw_content.rs` の文書）。触っても値は変わらないはずだが WARP で固定する |
| 1.4 指定しなければ今のまま | 今の `DrawImage` の道 | **在る**（分岐を足すだけ） |
| 1.5 どちらか 1 つ | — | **足す**: 型で表すか（列挙）、欄を 2 つにして優先を決めるか（設計） |
| 1.6 変形・切り抜きの下 | `emit` の `SetTransform(world_mat * root)`・切り抜きの push | **未知**: `BlendImage` が DC の世界の変換（回転・拡大・反転）を `DrawImage` と同じに効かせるか（文書は `targetOffset` だけを語る）。切り抜きの中の効き先は要件 3 と同じ問題 |
| 1.7 下限より古い Windows | `first_seen` の 1 度だけの記録 | **足す**: DC6 への cast の失敗 → `error!` 1 度 → `SOURCE_OVER`。装置なしの純粋なテストのため、「DC6 が在るか」を差し替えられる形（判定の関数を分ける）が要る |
| 1.8 指定の変化で描き直す | 中身の `is_changed()` → 版 → 列の組み直し → `Changed<DrawTree>` | **在る**（`hash_draw` にブレンドを足すのは `wintf-visual-damage-redraw` のための同一性） |
| 2.1 まとめた 1 枚に不透明度 | D2D の層（`PushLayer` の `opacity`・透明から始まる） | **足す**: 不透明度の層の命令・`build` での push／pop（深さで閉じる仕組みは切り抜きと共通にできる） |
| 2.2 不透明度 0 で描かない | `fill_world_visuals` の「見えない」の道 | **足す**（見えないと同じに扱えば版は追える） |
| 2.3 1.0 は今と同じ | 層を作らない | **在る**（層を出さない分岐） |
| 2.4 `clamped_opacity` と同じ扱い | `Visual::clamped_opacity` | **未知**: NaN は `clamp` を通っても NaN（2.1 の表）。D2D の層に NaN を渡したときの答え・WUC の `SetOpacity(NaN)` の答えを揃えるには測りが要る |
| 2.5 入れ子 | 層の入れ子 | **在る**（D2D の層は入れ子にできる） |
| 2.6 `warn!` を消す | `world_visual.rs` 177〜183 行目 | **足す**（消す） |
| 2.7 変化で描き直す | 1.8 と同じ道 | **在る** |
| 3.1〜3.3 切り抜きの中で下の画素に効く | 矩形の切り抜き（`PushAxisAlignedClip`）の中では D2D の重ね方はもう的に効く | **未知（最大の危険）**: 図形の切り抜き（層）の中では、今は透明から始まる層の中身にしか効かない。§4 の検討を参照 |
| 3.4 不透明度の層の中 | D2D の層の既定（透明から始まる・降ろすとき `SOURCE_OVER` で不透明度を掛ける） | **在る**（不透明度の層を `NONE` で積めば D2D の既定がそのまま 3.4 の答え） |
| 3.5 型の文書 | `emit.rs`・`draw_content.rs` の注記 | **足す** |
| 4.1 入れ物へ合成して写す | `SurfaceGraphics`・WARP の足場の「的のビットマップ」 | **足す**: 入れ物（`D2D1_BITMAP_OPTIONS_TARGET`・B8G8R8A8・α 掛け済み・96 DPI）、合成の DC、写し（D2D の `DrawImage` か D3D の `CopySubresourceRegion`） |
| 4.2 画素が今と同じ | — | **制約**: 写しがビット一致であること（D2D の写しなら整数の offset・恒等・`NEAREST_NEIGHBOR`・同じ DPI。D3D の写しなら構造上一致）。入れ物の原点 (0,0) で描いた答えと、WUC の atlas の offset で直接描いた答えが一致するか（整数の平行移動で反りが変わらないか）は WARP で固定する |
| 4.3 1 枚だけ・放す | `SurfaceGraphics::clear()`・`set_surface` | **足す**（欄を足せば寿命が揃う） |
| 4.4 中身を保つ | — | **足す**（写した後に捨てない＝欄に持つだけ） |
| 4.5 大きさ・装置の失い | 描画面の作り直しの道・`Res<GraphicsCore>` の変化の印 | **足す**（入れ物も作り直す・作り直した回は全面） |
| 4.6・4.7 失敗と失い | `render_surface` の `BeginDraw` の失敗の道・`note_draw_failure` | **在る**（同じ道に乗せる） |
| 4.8 根の列は今どおり | `render_surface` 130〜136 行目 | **在る**（描き先が入れ物に変わるだけ） |
| 5.1・5.2 同期の 1 画素 | `emit_test_support.rs` の `CopyFromBitmap` → `Map` | **足す**: 公開の口（描画面の持ち主の実体と物理ピクセルの座標を受ける）・1×1 の CPU_READ のビットマップ |
| 5.3・5.5 「画素なし」「読めない」 | — | **足す**: 透明と区別できる答えの型（列挙など） |
| 5.4 同じ座標なら読まない | — | **足す**: 入れ物ごとの合成の回数（世代）と、前の座標・答えの控え |
| 5.6 失敗の記録 | `error!` の形 | **足す**（装置の失いを二重に記録しない＝`note_draw_failure` の道に寄せる） |
| 5.7 求めたときだけ | — | **在る**（口を呼んだときだけ読む作りにすれば満たす） |
| 5.8 閾値を持たない | — | **在る**（値だけ返す） |
| 5.9 全体の読み戻し | 足場の `draw_to_pixels` と同じ手順 | **足す**（議題 2 の答え次第） |
| 6.1〜6.3 今の道との両立 | 既存の WARP テスト・実 GPU テスト・WUC の木 | **制約** |
| 6.4 語 | — | **制約**（ukadoc・areka の語を型の名に入れない） |
| 6.5 依存を足さない・本体に触らない | `windows` の既存の機能で足りる（`Win32_Graphics_Direct2D` 系は根の `Cargo.toml` に在る） | **在る** |
| 6.6 失敗の記録 | — | **制約**。D2D の描画の呼び出しは HRESULT を返さず `EndDraw` に出る（`emit.rs` の文書）＝層や `BlendImage` の失敗は `EndDraw` の結果で記録する形になる |
| 6.7 1,000 行・兄弟のテスト | — | **制約**（§2.3） |
| 7.1〜7.9 WARP の決定論のテスト | WARP の足場・`pixel_near` | **足す**: 入れ物と読み戻しの足場（WUC を使わず、入れ物の合成と読み戻しを WARP の装置で通せる形に部品を分ける） |
| 7.10 実 GPU | `tests/graphics/draw_tree_systems_test.rs` の形 | **足す**（画面を撮るテストは無い＝確かめ方は設計） |
| 8.1〜8.3 文書 | `tech.md` 72・90 行目、`structure.md` 55 行目 | **足す** |

## 4. 要件 3.1 の危険の検討（要件の作り手が挙げた点）

### 4.1 今の D2D の振る舞い（コードと文書から）

- **矩形の切り抜き（`PushAxisAlignedClip`）**: 層を作らず、的へ直接描く。`DrawImage` の重ね方（たとえば `DESTINATION_IN`）は、切り抜きの中の**的の画素**に効く。非 `SOURCE_OVER` の重ね方は、絵を「今の軸に沿う切り抜き」まで透明で広げてから合成する（`draw_content.rs` の文書）＝切り抜きの中の絵の外も変わる。切り抜きが無いときは描画面の全部まで広がる。つまり「切り抜きの範囲の中では、切り抜きが無いときと同じ式」が今すでに成り立っている。
- **図形の切り抜き（`PushLayer`・`D2D1_LAYER_OPTIONS1_NONE`）**: 層は透明から始まる（`D2D1_LAYER_OPTIONS1` の文書「A premultiplied layer target is pushed and its contents are cleared to transparent black」）。層の中の重ね方は**透明な層の中身**に効き、降ろすときに図形の覆いで切って `SOURCE_OVER` で的へ重なる。だから `DESTINATION_IN`・`DESTINATION_OUT`・`SOURCE_IN` など α を減らす重ね方は、下の画素に何も起こさない（矩形の切り抜きの答えと食い違う）。ブレンドも層の中の透明に対して混ぜるので、混ぜずにただ重ねたのと同じになる。
- 今のテスト（`emit_tests.rs`）には、図形の切り抜きの中の非 `SOURCE_OVER` の重ね方を確かめるものが無い（`destination_over_puts_later_sibling_below` は切り抜きなし）＝食い違いは檻の外にある。

### 4.2 D2D が許す道

| 道 | やり方 | 3.1 に効くか | 不確かさ |
|---|---|---|---|
| **甲: 背景から始める層＋ COPY** | 図形の切り抜きの層を `D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND` で積み、積むときの基本の混ぜ方（`SetPrimitiveBlend`）を `D2D1_PRIMITIVE_BLEND_COPY` にする | Layers Overview: 「the device context has a primitive blend mode … This mode also applies to layers when you call the PushLayer method」「if you are using a layer to clip primitives with transparency set the D2D1_PRIMITIVE_BLEND_COPY mode … The copy mode makes the device context linear interpolate all 4 color channels, including the alpha channel, of each pixel with the contents of the target surface according to the geometric mask of the layer」。層の中身は下の画素の写しから始まり、中の重ね方とブレンドは下の画素に効き、降ろすときに図形の覆いの割合で層と的が補間される＝図形の内側は「切り抜きが無いときの答え」、外側は的のまま。α を減らす重ね方も正しく効く（`SOURCE_OVER` で降ろす形では、背景から始めた層を重ねると下の画素が二重になるので使えない＝`COPY` が要る） | ①降ろすときの補間が「積んだときの混ぜ方」で決まるのか「降ろすときの混ぜ方」で決まるのか（積んだ直後に `SOURCE_OVER` へ戻して中の描画に影響させないでよいか）②`INITIALIZE_FROM_BACKGROUND` と `COPY` を組んだときの式が文書の 1 文以外に書かれていない ③WARP と実 GPU で同じか ④層の `contentBounds` が ±f32::MAX のままだと、背景の写しが描画面の全部になりうる（性能）＝図形の世界の外接矩形に絞るのが要る。`EndDraw` まで誤りが出ない作りなので、試しは画素で見る |
| 乙: 層を使わず効果で組む | 図形の外接矩形の下の画素を写し（`CopyFromBitmap`）、効果のグラフ（ブレンド／重ね方 → 覆いで補間）で答えを作って `BOUNDED_SOURCE_COPY` で戻す | 原理上は効く | 描画の途中で的を写すには `Flush` が要り、D2D は層を積んだままの `Flush` を誤りの状態にする（Layers Overview「If Flush is called before all outstanding layers are popped, the render target is placed into an error state」）＝不透明度の層や外側の図形の切り抜きの中で使えない。手数が多い |
| 丙: 図形を `FillGeometry`＋絵の筆で描く | 層を使わず、図形を命令の列の絵の筆で塗る | 効かない（`FillGeometry` は基本の混ぜ方〔`SOURCE_OVER`・`COPY`・`MIN`・`ADD`〕しか持たず、重ね方の 13 種もブレンドも表せない） | — |
| 丁: 要件を D2D の既定に合わせる | 図形の切り抜きの中は層の中身に効く、を答えとして型の文書に書く | 3.1〜3.3 を書き換える | 後の spec（`draw-methods-canon`）が ukadoc の重ね方を図形の切り抜きと組み合わせると、矩形と図形で答えが分かれる |

**見立て**: 甲は D2D の文書に根拠があり、コードの変更は `emit.rs` の `PushClipGeometry` の 1 か所（旗・基本の混ぜ方・`contentBounds`）で済む。ただし文書の記述が 1 文で、降ろすときの式を自分の手で確かめた記録は無い。**設計の前に WARP で「回転した矩形・楕円の切り抜きの中の `DESTINATION_IN`・`DESTINATION_OUT`・`multiply` が、矩形の切り抜きの中と同じ画素になるか」を試すのが要る**。試しが通らなければ、丁（要件の書き換え）を要件討議へ戻すことになる。乙は層の中で使えないので、要件 3.4 と組み合わせられない。

### 4.3 不透明度の層の中（要件 3.4）と入れ子

- 不透明度の層は `NONE`（透明から始める）で積み、降ろすときに不透明度を掛けて `SOURCE_OVER` で重ねる＝D2D の既定どおりで 3.4 の答えになる（重なる子孫が互いに透けない＝要件 2.1 も同じ）。
- 不透明度の層の中に図形の切り抜き（甲）が在ると、「背景」は不透明度の層の今の中身になる＝「同じ層の中で先に描かれた画素に効く」（3.4）と矛盾しない。
- 同じ節に不透明度と図形の切り抜きの両方が在るとき、D2D の層 1 枚に `opacity` と `geometricMask` を両方入れると、甲の `COPY` の補間が不透明度まで背景と混ぜてしまうおそれがある＝**層を 2 枚（外が不透明度・中が切り抜き）に分ける**のが安全（設計で確かめる）。

### 4.4 ブレンドの口と切り抜き・層

- `BlendImage` は「Direct2D の Blend 効果と同じ答え」（文書）で、下の画素として**今の的**（層を積んでいれば層）を読む。矩形の切り抜きの中では的を、甲の層の中では背景の写しを、不透明度の層の中では層の中身を読む＝要件 3 の答えと同じ構図に乗る。
- ブレンドの効果（`CLSID_D2D1Blend`）は下の画素を入力として渡す必要があり、§4.2 乙と同じく層の中で使えない。**下限を 1903 より下げるなら、ブレンドを層の中で描けない**ことになる（要件討議の議題 1 の判断材料）。
- `BlendImage` が WUC の `BeginDraw` の的（atlas の描画面）を読めるかは分からないが、入れ物（自前の `TARGET` のビットマップ）へ描くなら問題にならない＝**入れ物（要件 4）がブレンドの前提になる**。

## 5. 実装の選択肢

### 選択肢 A: 今の部品を広げる（入れ物を `SurfaceGraphics` に持つ）

- `VisualDrawContent`・`DrawItem` に「重ね方かブレンドか」を足し、`emit` が `BlendImage` を呼ぶ（DC6 は合成の回ごとに 1 度 cast）。
- `DrawCommand` に不透明度の層の push／pop を足し、`build` が切り抜きと同じ「深さで閉じる」で出す。`emit` の `PushClipGeometry` に甲の旗を入れる。
- 入れ物を `SurfaceGraphics` の欄にし、`deferred_surface_creation_system`（作る・大きさで作り直す）・`clear()`（放す）に寿命を合わせる。`render_surface` は「入れ物へ合成 → WUC の `BeginDraw` → 入れ物を写す → `EndDraw`」の順。
- 読み戻しの口は `render.rs` と同じ階層に新しいファイル（例: `systems/readback.rs`）。
- ✅ 寿命・失いの道・変化の検出がすべて既存の道に乗る。❌ `render.rs`（153 行）に合成と写しが入って太る（それでも数百行で収まる見込み）。`SurfaceGraphics` の欄が増え、`Send`／`Sync` の根拠の文書が要る。

### 選択肢 B: 入れ物と読み戻しを新しい部品にする

- 入れ物を独立の部品（例: `SurfaceComposite`）として描画面の持ち主に付け、作成・作り直し・放すを専用の係で行う。合成と写しを `render_surface` から呼ぶ純粋な関数（DC と入れ物と列を受ける）に出し、WARP の足場から同じ関数を呼べるようにする。
- ✅ WUC を使わずに「入れ物へ合成 → 写し → 読み戻し」を WARP で通せる（要件 7.7・7.8 が書きやすい）。`SurfaceGraphics` を変えない。❌ 部品と係が増え、描画面の寿命と入れ物の寿命を合わせる仕組みを別に持つ（失いの合図・大きさの変化・持ち主から外れたとき）。

### 選択肢 C: 組み合わせ（推奨の見立てではなく、比べるための形）

- 入れ物の**置き場**は A（`SurfaceGraphics` の欄＝寿命を共有）、**合成と写しの手順**は B（DC・入れ物・列を受ける関数を別ファイルに置き、`render_surface` と WARP の足場の両方から呼ぶ）。
- ブレンド・不透明度の層・甲の旗は、どの選択肢でも `visual/draw/`（`command.rs`・`builder.rs`・`emit.rs`）の拡張になる。
- ✅ テストの書きやすさと寿命の単純さを両取りできる。❌ 関数の境（どこまでを WARP で通すか）を設計で決める必要がある。

### 写しの方式（どの選択肢でも決める）

| 方式 | 中身 | 長所 | 短所 |
|---|---|---|---|
| D2D の写し | WUC の DC で入れ物のビットマップを `DrawImage`（offset の平行移動・恒等・`NEAREST_NEIGHBOR`・`SOURCE_COPY` か透明で塗った後の `SOURCE_OVER`） | 今の `BeginDraw` の口のまま・装置は同じ（WUC の描画装置は `GraphicsCore` の D2D の装置から作られている＝`wuc_resource.rs`） | 4.2 のビット一致を DPI・補間の設定で保つ必要（WUC の DC の DPI を確かめる） |
| D3D の写し | `BeginDraw` で `IDXGISurface`（または `ID3D11Texture2D`）を受け、即時の文脈で `CopySubresourceRegion` | 構造上ビット一致 | `com/wuc.rs` に別の `BeginDraw` の口が要る・D2D の `Flush`（入れ物の `EndDraw`）との順序を守る必要・D3D の即時の文脈を UI スレッドで触る |

## 6. 規模と危険

- **規模: L（1〜2 週）** — ブレンド・不透明度の層・層の旗・入れ物と写し・読み戻しの 5 つの口と、それぞれの WARP のテスト（26 種のブレンドだけでも 26 点以上）。どれも既存の型の延長だが、触るファイルが 10 前後に広がる。brief の 11〜15 タスクと合う。
- **危険: 中〜高** — 要件 3.1 の甲の振る舞いが文書の 1 文でしか裏付けられない（試しが通らなければ要件の書き換え）。`BlendImage` の変形の扱い（1.6）と絵の外（1.3）も文書に無い。入れ物の写しで既存の画面がビット一致で変わらないこと（4.2・6.1）を全体テストで守る必要がある。ほかは既存の型の延長で低い。

## 7. 設計の段への申し送り

### 7.1 決めること（設計の段）

1. ブレンドを型でどう持つか（`composite_mode` を「重ね方かブレンドか」の列挙へ置き換えるか、欄を足して優先を決めるか）。公開の型が変わる範囲（`VisualDrawContent`・`DrawItem`・`DrawCommand`）。
2. 不透明度の層の命令の形（新しい push／pop か、切り抜きの命令に不透明度を持たせるか）と、同じ節の不透明度と図形の切り抜きの層の順（§4.3 の 2 枚案）。
3. 入れ物の置き場（選択肢 A／B／C）と写しの方式（D2D か D3D か）。
4. 合成に使う DC（`GraphicsCore` の共有の DC か専用の DC か）。
5. 読み戻しの口の形（実体と座標を受ける関数か SystemParam か・答えの型〔画素／画素なし／読めない〕・世代の数え方）。
6. 実 GPU のテストで「画面に出るか」をどう確かめるか（wintf には画面を撮るテストが無い）。

### 7.2 Research Needed（設計の前・中に確かめる）

- **R1（最優先・設計の前に）**: §4.2 甲の WARP での試し。回転した矩形と楕円の切り抜きの中で、`DESTINATION_IN`・`DESTINATION_OUT`・`DESTINATION_OVER`・ブレンド 1 種が、矩形の切り抜きの中と同じ画素になるか。基本の混ぜ方を `COPY` にするのは `PushLayer` の直前だけでよいか（直後に `SOURCE_OVER` へ戻して中の描画が変わらないか）。図形の縁の半端な覆いの答え。同じ試しを実 GPU でも 1 度。
- **R2**: `BlendImage` が DC の世界の変換（回転・拡大・負の拡大）を `DrawImage` と同じに効かせるか・絵の外の画素を触るか（1.3・1.6）。
- **R3**: 入れ物の原点で描いた答えと、WUC の atlas の offset で直接描いた答えがビット一致か（4.2）。WUC の `BeginDraw` の DC の DPI。
- **R4**: 1×1 の同期の読み戻し（`CopyFromBitmap` → `Map`）の待ちの長さを実 GPU で測る（マウスが動くたびに呼ばれる前提の見積もり・brief の議題 3）。
- **R5**: `ID2D1DeviceContext6` への cast が、`GraphicsCore` の装置から作った DC（`ID2D1Device::CreateDeviceContext`）でも通るか（工場・装置の版に依らず、実行時の D2D の版だけで決まるか）。
- **R6**: 節の不透明度が NaN のときの D2D の層の答えと、WUC の `SetOpacity(NaN)` の答え（要件 2.4 の「NaN の扱いも同じ」をどう満たすか）。
- **R7**: 甲の層で `contentBounds` を図形の世界の外接矩形（物理ピクセル）に絞ったときの性能と答えの不変。

## 8. 要件討議へ出る論点（答えで要件か作業が変わるもの）

1. **要件 3.1〜3.3 の実現性**（新しい論点・最大の危険）: 図形の切り抜きの中で下の画素に効かせる道は、D2D では「背景から始める層＋ COPY」（§4.2 甲）しかなく、その振る舞いは文書の 1 文でしか裏付けられない。試し（R1）が通らなかったときに、要件 3.1〜3.3 を「D2D の既定（図形の切り抜きの中は層の中身に効く）を型の文書に書く」へ書き換えるか、乙（効果で組む・層の中で使えない）まで広げるか、の分かれ目を先に決めておくと設計が止まらない。
2. **ブレンドの口と Windows の下限**（要件の議題 1・要件 1.7）: 効果の道（1903 より前でも動く）は、下の画素を写すのに `Flush` が要るので**層の中（不透明度の層・図形の切り抜き）では使えない**。下限を 1903 より下げると、要件 3・2 と組み合わせたブレンドが描けなくなる。今のコードの実効の下限は Windows 10 1803 で、1903 へ上げる差は小さい（Windows 10 の今の版はすべて 1903 より新しい）。下限を `tech.md` にどう書くか。
3. **入れ物を全部の描画面に持つか**（要件の議題 4）: 遅延して作る形（読み戻しが初めて求められたときに作る）は、最初の読み戻しが「読めない」になり「1 フレーム遅らせる解は取らない」と衝突する。ブレンドも入れ物の上でしか確実に描けない（§4.4）。全部に持つ形は、描画面ごとにメモリが 1 枚分増え、描き直しのたびに写しが 1 回増える。
4. **クリックの透過の付け替え**（要件の議題 2）: 本 spec の後も emo は CPU で合成するので、今の α マスク（`ecs/widget/bitmap_source/alpha_mask.rs`・emo-present の `presenter/budget.rs`・`cache.rs`）の出どころは消えない。置き換えないなら、読み戻しの口は使い手が 0 のまま着地する（使い手は `hit-id-pass`・`draw-methods-canon`）。待ちの長さ（R4）を測って記録に残すだけで足りるか。全体の読み戻し（5.9）を本 spec が持つか。
5. **要件 2.4 の NaN**: 「`clamped_opacity` の答えと同じ扱い」は NaN を NaN のまま D2D へ渡すことになる。NaN を 0 か 1 に寄せる決めを足すか、WUC の答えを測って揃えるか（R6）。
6. **要件 7.10 の実 GPU の確かめ方**: wintf に画面を撮るテストは無い。「入れ物を通した描画面の `BeginDraw`／`EndDraw` が実 GPU で通り、読み戻しの口が答えの画素を返す」で「画面に出る」とみなしてよいか。

## 9. 参照

- [Layers Overview（Direct2D）](https://learn.microsoft.com/en-us/windows/win32/direct2d/direct2d-layers-overview) — 層の基本の混ぜ方・`COPY` の補間・`INITIALIZE_FROM_BACKGROUND`・層を積んだままの `Flush` の誤り
- [D2D1_LAYER_OPTIONS1](https://learn.microsoft.com/en-us/windows/win32/api/d2d1_1/ne-d2d1_1-d2d1_layer_options1) — `NONE`＝透明で塗る・`INITIALIZE_FROM_BACKGROUND`＝塗らない
- [ID2D1DeviceContext6::BlendImage](https://learn.microsoft.com/en-us/windows/win32/api/d2d1_3/nf-d2d1_3-id2d1devicecontext6-blendimage) — Blend 効果と同じ答え・`d2d1_3.h`・19H1（Windows 10 1903）
- `windows` 0.62.2 の `Win32/Graphics/Direct2D/mod.rs`・`Common/mod.rs` — `ID2D1DeviceContext6`・`BlendImage`・`D2D1_BLEND_MODE_DIVISION`（25）・`CLSID_D2D1Blend`・`D2D1_LAYER_OPTIONS1_INITIALIZE_FROM_BACKGROUND`・`D2D1_PRIMITIVE_BLEND_COPY` が在ることを確かめた
