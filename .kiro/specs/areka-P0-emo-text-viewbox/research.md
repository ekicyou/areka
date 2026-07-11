# Gap Analysis — areka-P0-emo-text-viewbox

> 対象: `.kiro/specs/areka-P0-emo-text-viewbox/requirements.md`（確定済み）
> 種別: 本坑（main）・⑥ emo トラックの増分ユニット（依存＝emo-text-layer ✅ のみ）
> 目的: viewbox 方式（クリップ viewport ＋ content オフセット）への描画実行差し替えの実装戦略を、既存コードとの差分から導く
> 調査日: 2026-07-11

---

## 1. Analysis Summary（3–5 bullets）

- **既存資産は「描画実行の分離シーム」が意図的に用意済み**で、可視窓決定（`LayoutEngine::visible_window`・純粋層）／内容キャンバス（`ContentCanvas`・全行保持・可視窓非適用）／描画実行（`DrawExecutor::render`・全域再描画）が明確に分かれている。本ユニットは `DrawExecutor::render` と `TextSurface`（供給面）だけを差し替えれば足り、純粋層・状態機械・レイアウト・結線層（`actor.rs`・`sink.rs`）は不変で消費できる（R2/R9 の主張は既存構造と整合）。
- **wintf 側の clip primitive は流用可能な形で完備**（`ClipShape::Rectangle` ＋ `clip_sync_system`・DPI/物理化込み）。加えて `visual_hierarchy_sync_system` が ECS `ChildOf` → WUC visual 親子を同期するため、「viewport（clip）＋ content（子・オフセット）」の 2 層 visual 合成は wintf 改変ゼロで組める見込み（R1.2/R9.4）。
- **最大の新規能力は 3 点**: ①content 描画面の「一度描き＋増分追記＋成長（再確保）＋Clear リセット＋上限」（現状は validrect 寸固定・毎フレーム透明 clear＋全域再描画）、②content visual の translate offset による再描画レス・スクロール（現状は `block_offset` を描画原点へ加算＝毎回再描画）、③再描画方式 golden との pixel 等価担保。①が構造的に最も重い（growable swapchain/texture・increment-only 描画）。
- **決定論の再描画レス観測**は既存の `#[cfg(test)] line_layout_creations` カウンタが土台になり得るが、「スクロール時に描画呼び出しが増えない」の檻を新設する必要がある（R3.4/R10.3）。pixel 等価 golden は既存 `draw_readback_test.rs`／`scale_invariance_test.rs` を比較基準として流用できる（別方式の同一入力→同一 readback）。
- **主要リスク**は、DPI/スケール契約を崩さず 2 空間（image px / 物理 px）を保ったまま content 面を成長させること（記憶 areka-window-placement-dpi-coordinate-defect）と、縦書き（vertical_rl）で content 面が「右→左」に成長する軸方向の扱い（成長の起点が右端側）。両者とも既存の軸読み替え正準表と `ScaleContract` に写像できるが、design で式を明示確定すべき。

## 2. 現状調査（Current State）

### 2.1 emo-text-layer（`crates/areka-emo-text/`）の層構造

| 層 | モジュール | 役割 | 本ユニットでの扱い |
|---|---|---|---|
| 純粋層 | `state.rs` / `writing.rs` / `region.rs` / `layout.rs` / `canvas.rs` | 状態機械・writing_mode 解決・領域解決・折返し/行送り/可視窓決定・内容キャンバス | **不変で消費**（R2.1/R2.2/R2.4/R2.5） |
| COM 層 | `draw.rs` / `surface.rs` | DirectWrite/D2D 描画実行・自前 swapchain 供給面 | **差し替え対象**（描画実行・供給面構造） |
| 結線層 | `sink.rs` / `actor.rs` | cue 受信・UI ドレイン・フレーム提示（`present_frame`） | 呼び順は不変・`present_frame` 内の描画実行呼び出しのみ差し替え |

- 純粋層に `windows` 依存が無いことは `lib.rs` の構造テスト（`pure_layer_modules_have_no_windows_imports`）が檻化済み。本ユニットの改変が純粋層へ漏れれば即座に赤くなる（R2.5 の自動担保）。

### 2.2 分離シームの実シンボル（差し替えの結線点）

- **可視窓決定（唯一のスクロール決定点・不変）**: `LayoutEngine::visible_window(lines, region, mode) -> VisibleWindow { first_visible_line: usize, block_offset: f32 }`（`layout.rs:255`）。doc コメント（`layout.rs:123–126`）が本ユニットを名指しで「この出力を『クリップ視窓＋内容オフセット』へ写像して描画実行だけを差し替える」と明記。
- **内容キャンバス（描画元・不変）**: `ContentCanvas`（`canvas.rs:200`）は全行を `residents: Vec<Resident>` として保持し可視窓を適用しない（`canvas.rs:194–198`）。住人 index = layout 行 index の 1:1。`ContentCanvas::from_layout`（`canvas.rs:217`）が validrect-local（image px）へ転写。
- **描画実行（差し替える側）**: `DrawExecutor::render(canvas, window, font, mode, contract, surface)`（`draw.rs:523`）。現在は次を毎フレーム行う:
  1. `window.first_visible_line` 以降の住人だけを描画対象に取る（`skip`）。
  2. 各住人の origin に `window.block_offset` を軸方向（横=y／縦=x）へ**加算**（`draw.rs:564–574`）。
  3. `source_tex`（validrect 寸）を透明 clear → `SetTransform(scale k)` → 各行 `DrawTextLayout`（`draw.rs:595–611`）。
  - 行 TextLayout キャッシュ（`line_cache`・行 index キー）で確定行の TextLayout 生成は避けているが、**毎フレーム全域を clear→再描画**する（差分描画なし・SSP 忠実）。
- **呼び順の結線点**: `actor.rs:440–457`（`visible_window` → `ContentCanvas::from_layout` → `DrawExecutor::render` → `TextSurface::present`）。本ユニットは `render`＋`present`（および供給面構造）を差し替え、`visible_window`／`from_layout` の呼び出しは保つ。
- **Clear の適用点**: `TextLayerRuntime::apply_cue`（`actor.rs:210`）が `CueCommand::Clear` で `executor.clear_cache()` を呼ぶ。viewbox では「content 面のリセット」もこの口に写像する必要（R4.3）。

### 2.3 供給面 `TextSurface`（`surface.rs`）の現状構造

- `attach`（`surface.rs:157`）が actor ごと初回のみ、予約スロット entity（`emo-text-layer-slot`）へ **単一の `SpriteVisual`（swapchain brush）＋ `Arrangement`（物理 px 直接）** を donor 装着。`GraphicsCommandList` は挿入しない（wintf の widget 描画経路と競合させない・R9.3）。
- `source_tex`（`surface.rs:132`・D3D11 DEFAULT・B8G8R8A8・RENDER_TARGET）は **validrect 物理寸に固定**され、全 0（premultiplied 透明）初期化。`present` は `CopyResource(backbuffer, source_tex)`→`Present(0)`。`read_back` は `source_tex`→staging→BGRA 密配列（決定論検証口）。
- **含意**: viewbox 化では供給面が「validrect 寸で固定」から「content 全長（成長）」へ変わる。viewport（validrect 寸・clip）と content（成長する swapchain sprite）の 2 visual への分割が必要。

### 2.4 wintf 側の流用可能 primitive（改変しない）

- **`ClipShape`**（`crates/wintf/src/ecs/graphics/clip.rs`）: `Rectangle` / `RoundedRectangle` / `RoundedRectangleIndividual`。`Visual.clip: Option<ClipShape>`（`visual.rs:30`）に載せる。
- **`clip_sync_system`**（`crates/wintf/src/ecs/graphics/systems/clip_sync.rs`）: `Arrangement.size × GlobalArrangement.scale` で物理化して WUC `CompositionClip`（`Rectangle` は inset 0 の RoundedRectangleGeometry・明示サイズ）へ写像し `visual_com.SetClip` する。**Arrangement.size を基準**にするため、clip 対象 entity に有効な `Arrangement`（validrect 物理寸）と `Visual`（clip=Some(Rectangle)）が要る。予約スロットは既に `Visual::default()`（clip=None）を持つため、`visual.clip` を設定すればこの system が拾う。
- **`visual_hierarchy_sync_system`**（`crates/wintf/src/ecs/graphics/systems/visual_sync.rs`）: ECS `ChildOf` → WUC visual 親子（親 `AddVisual` 子）を同期。**viewport（親スロット）の下に content 子 visual を ECS で足せば WUC 親子が張られる**。ただし再ペアレントの既知ギャップ（`hierarchy_reparent_gap_test.rs`）あり——本ユニットは初回一度の構築で足りるため影響は小さいが design で確認。

### 2.5 観測資産（pixel 等価 golden の比較基準）

- `tests/draw_readback_test.rs`: 実 pump 通し（sink→ドレイン→`present_frame`→`read_back`）で「単調増加／Clear 全透明／validrect 封じ込め／スクロールで先頭行消失／同一入力同一 pixel」を横書き・縦書き（vertical_rl）で檻化。**差し替え後もこれらがそのまま green であること**が R6/R2.5 の主要担保。
- `tests/scale_invariance_test.rs`: 複数スケールの不変検証（R6.4 の流用基準）。
- 観測 example `examples/emo-text-layer.rs`: readback 述語で 7 チェックポイント（typewriter／改行／あふれ→スクロール／Clear／複数 actor 独立）を単一 PASS/FAIL 出力。R10 はこのスクロール経路を viewbox 方式へ差し替える（新 example か本 example 差し替えかは設計判断＝§5-DD7）。

## 3. Requirement-to-Asset Map（gap タグ: 流用 / 拡張 / 新規 / 未確定）

| Req | 必要能力 | 既存資産 | Gap |
|---|---|---|---|
| R1 viewport+content 2 層合成 | `ClipShape::Rectangle`＋`clip_sync_system`／`visual_hierarchy_sync_system`／`Visual.clip` | wintf 完備・スロットは `Visual` 保持 | **拡張**: emo-text 側で content 子 visual を新設し viewport に clip を載せる |
| R2 描画実行差し替え（上流不変） | `visible_window`／`ContentCanvas`／呼び順（actor.rs:448–457） | 分離シーム納品済み | **拡張**: `render` の中身のみ置換・呼び順保持 |
| R3 一度描き・スクロール再描画レス | `line_cache`（TextLayout 再利用）／`line_layout_creations` カウンタ | 部分的（TextLayout は再利用するが面は毎回 clear） | **新規**: 増分追記描画（clear 廃止）＋描画呼び出しカウント檻 |
| R4 content 面の成長・上限・リセット | `source_tex`（固定寸）／`clear_cache`（Clear 適用点） | 固定寸のみ | **新規**: growable content 面・成長規則・上限・Clear リセット |
| R5 writing_mode 追随の軸切替 | `WritingMode`／`block_offset` 軸規約（layout.rs:128–137）／軸読み替え正準表 | 正準表・符号規約あり | **流用**: offset を content visual の軸へ写す（新規実装だが規則は既存） |
| R6 再描画方式との pixel 等価 | `draw_readback_test`／`scale_invariance_test`／readback 述語 | 比較基準あり | **拡張**: 両方式の同一入力→同一 readback 比較を新設 |
| R7 固定層差し込み点予約 | `ResidentContent::Image(ImageSeam)` 型シーム／viewport 直下 | 型シームのみ | **新規（構造予約のみ）**: viewport 直下に offset を受けない層の差し込み点 |
| R8 補間・慣性のシーム | `RegionTransform`（M1 恒等/平行移動のみ）／M2 予約規律 | 予約規律あり | **新規（型/構造シームのみ）**: offset を可視窓決定と補間過程で分離 |
| R9 クロスユニット契約シーム | emo-present `TextSlotView`／`register_actor_view`／choice-render 再利用シーム（layout.rs:88–119） | 消費経路・座標契約あり | **流用**: 改変を描画実行側に閉じる・座標契約点を残す |
| R10 観測 example | `examples/emo-text-layer.rs`（7 チェックポイント） | 既存 example | **拡張**: スクロール経路を viewbox 方式へ差し替え |

## 4. Implementation Approach Options

差し替えの核は「描画実行（`draw.rs`）＋供給面（`surface.rs`）」であり、純粋層・結線層は不変。以下は **供給面/描画面の構成戦略**の 3 案。

### Option A — 単一 content swapchain ＋ viewport clip（visual 2 層・content 面成長）

- content visual = テキスト全長の自前 swapchain（成長時 `ResizeBuffers` or 新 texture 再確保＋旧内容コピー）。viewport（スロット）に `Visual.clip = Rectangle`（validrect 寸）を載せ、content を子 visual として `SetOffset`（or Arrangement offset）でスクロール。
- 描画は増分のみ（新規行/グリフを content 面の確定位置へ追記描画・既存部は触らない）。Clear で content 面を初期寸へ戻す。
- **Trade-offs**: ✅ SSP 的「内容層＋固定層」二層に自然対応（R7 予約が容易）。✅ 再描画レスが構造的に成立（offset 更新のみ）。✅ wintf 改変ゼロ。❌ growable swapchain の `ResizeBuffers`／再確保＋コピーの COM 実装が新規で重い。❌ content 面の物理寸上限管理が要る。
- **推奨度: 高**（brief の「viewport visual＋content visual の 2 層」記述そのもの）。

### Option B — content = オフスクリーン D2D ターゲット texture（swapchain は viewport のみ）

- content は表示に直結しない大きな D2D ターゲット texture（成長可）。毎フレーム viewport swapchain へ content の可視窓部分を `CopyResource`（矩形コピー・offset 指定）で転送し Present。
- **Trade-offs**: ✅ 「一度描き」の content と「提示」を明確に分離（content 面は描画専用・swapchain は viewport 寸固定で成長不要）。✅ 矩形コピーで軸オフセットを表現＝clip primitive を使わずとも viewport 封じ込めが成立。❌ 毎フレーム CopyResource が走る（offset 変化なくてもコピー・ただし再描画=DirectWrite 生成は発生しないため R3 は満たす）。❌ WUC clip visual の「滑らか補間シーム」への発展性が Option A より弱い（M2 は WUC アニメーション活用＝Option A の visual offset の方が親和）。❌ brief の「2 層 visual 合成」記述と字面がずれる（visual は 1 層・content は texture）。
- **推奨度: 中**（実装は堅いが M2 演出シーム・R7 固定層 visual との親和性で A に劣る）。

### Option C — ハイブリッド（content D2D ターゲット面 ＋ viewport/content の 2 visual・offset は visual translate）

- 描画面は Option B の growable D2D ターゲット texture（描画専用・DirectWrite 生成は増分のみ）だが、提示は Option A の viewport（clip）＋content（子 visual・swapchain surface for content texture）の 2 visual 構成にして、スクロールを **子 visual の translate offset** で表現（CopyResource を毎フレーム走らせない）。
- content texture を WUC surface へ載せる方法: `ICompositorInterop::CreateCompositionSurfaceForSwapChain`（現行 surface.rs が使う経路）を content swapchain に適用、描画は content の `source_tex`（成長）へ、提示は content swapchain Present。offset は content visual に載せる。
- **Trade-offs**: ✅ 描画面成長（texture 再確保＋コピー）と提示（visual offset）を分離＝両者の良いとこ取り。✅ M2 の WUC アニメーション補間へ直結（offset を visual プロパティで動かす）。✅ R7 固定層＝viewport 直下の別 visual で自然。❌ 実質 Option A に「描画面 = 成長 texture」を足した形で、構成要素が最多（swapchain 成長 or texture 成長 + surface 巻き直しの扱いを design で 1 つに決める必要）。
- **推奨度: 高〜中**（A と実装が収束しがち。design で「content 面の実体（swapchain 直か texture＋surface 巻きか）」を一点確定すれば A/C は同一物になる＝§5-DD2）。

> 共通の非交差保証: いずれの案でも改変は `draw.rs`／`surface.rs`（COM 層）と `actor.rs` の描画呼び出し結線に閉じ、純粋層・`sink.rs`・emo-present 消費経路（`TextSlotView`／`register_actor_view`）は不変（R9.1/R9.2）。emo2-boot（並走）は sink／装着 API／present_frame を消費するのみで本改変面と非交差。

## 5. 設計判断アイテム（Design Decisions — 要件ディスカッションへ送る）

1. **DD1 offset 写像の正準式**: `VisibleWindow { first_visible_line, block_offset }` を content visual の translate へどう写すか。現状 render は `first_visible_line` を skip＋`block_offset` を原点加算に使うが、viewbox では **content を全行一度描き**するため、両者を単一の visual offset（軸方向）へ畳む式が要る（例: offset = −(先頭可視行の near 位置差) を軸へ）。`block_offset` の符号規約（横=負/vertical_rl=正/vertical_lr=負・layout.rs:130–137）との一致を明示。

2. **DD2 content 描画面の実体**: 「成長する swapchain（`ResizeBuffers`）」か「成長する D2D ターゲット texture ＋ 別 swapchain/surface へコピー提示」か（Option A vs B/C の収束点）。再確保時の既存内容保全（コピー）の実装コストとデバイス失敗時の縮退（log-first）を含めて確定。

3. **DD3 content 面の初期サイズ・成長単位・上限**: 初期寸（例: validrect 寸 or validrect × N）、伸長の粒度（1 行 pitch 単位 or 倍々 or 固定増分）、上限値（物理 px・メモリ実量）と上限到達時の扱い（Clear まで飽和 / 最古行破棄 / 強制リセット）。R4.4 は「具体値は設計で確定」と明示。

4. **DD4 増分追記の判定と `line_cache` の再解釈**: どこまでを「確定 content（再描画しない）」とし、どの増分（typewriter 進行中の最終行・NewLine・Text 追記）だけを追記描画するか。現状の行 TextLayout キャッシュ（確定行は再利用・リビール中の最終行のみ更新）を content 面の増分追記へどう写すか。リビール途中行の再描画は「文字再描画」に当たるか（R3.3 の「増分だけ追記」との整合）。

5. **DD5 viewport clip の実現手段**: (a) スロットの `Visual.clip = Rectangle` を設定して `clip_sync_system` に任せる（Arrangement.size = validrect 物理寸が clip 寸になる）か、(b) content 面を validrect 寸へコピーして封じ込める（Option B）か。(a) は wintf system への依存が増える（clip_sync が Changed 検出で走るタイミング）・content 子 visual の親子構築（`visual_hierarchy_sync_system`）の順序保証を design で確認。

6. **DD6 再描画レスの決定論観測手段**: `DrawExecutor` の `line_layout_creations`（TextLayout 生成回数）＋新設「グリフ描画呼び出し回数」カウンタで「スクロール（可視窓のみ移動）フレームでは描画呼び出しが増えない」を檻化する形（R3.4/R10.3）。ログ捕捉（`with_log_cage` パターン）とカウンタのどちらを正典にするか。

7. **DD7 観測 example の差し替え方針**: 既存 `examples/emo-text-layer.rs` を viewbox 方式へ**改変**するか、viewbox 専用 example を**新設**するか（R10.1）。前者は再描画方式の観測資産を失う（pixel 等価比較の基準がテスト側 `draw_readback_test` に一本化される）。後者は 2 example 併存。pixel 等価 golden の比較を「テスト内で両方式を同時に走らせる」か「再描画方式の固定 golden bytes を保存して照合」かも含む（DD8 と連動）。

8. **DD8 pixel 等価の比較実装**: (a) 同一プロセス内で再描画方式（現 `DrawExecutor`）と viewbox 方式を両方走らせて readback をバイト比較、(b) 再描画方式の golden bytes をファイル固定して照合、(c) 既存 `draw_readback_test` の述語（単調増加・封じ込め・先頭行消失）を viewbox 実装でも green にする（厳密 pixel 一致でなく述語一致）。要件は「pixel 等価」を明記（R6.1）ゆえ (a)/(b) 相当が要る——どちらを主檻にするか。

9. **DD9 縦書き content 面の成長方向**: vertical_rl は列が右→左へ成長し、書字開始角が validrect 右上（region.start）。content 面の物理原点と成長方向（右基準で左へ伸ばす／面内は左右反転しない座標にして offset で吸収）を design で確定（記憶 areka-window-placement-dpi-coordinate-defect の二空間混在を持ち込まない）。

10. **DD10 R7 固定層差し込み点の構造形**: 「viewport 直下・offset を受けない層」を、(a) ECS 上の別 visual entity（未装着スロット）として予約するか、(b) 型/enum（`ResidentContent::Image` は content 側の型シーム——固定層は content でなく viewport 直下ゆえ別物）として予約するか。M1 の pixel 等価 golden（固定層なし）に影響しない形（R7.4）。

## 6. Effort / Risk

- **Effort: L（1–2 週）**。純粋層・結線層・parse は無改変で、改変は `draw.rs`／`surface.rs`（COM 層）＋ `actor.rs` の結線点＋新規テストに限局。ただし growable content 面（DD2/DD3）と増分追記描画（DD4）と pixel 等価比較（DD8）が新規 COM/検証実装で、それぞれ M 相当の重さ。合計で L。
- **Risk: Medium**。
  - clip primitive・visual 階層同期は wintf 実装済み＝既知（Low 要因）。
  - growable swapchain/texture の再確保＋既存内容コピー、縦書き成長方向、2 空間（image px/物理 px）維持は新規で綻びやすい（Medium 要因・記憶 dpi-coordinate-defect）。
  - pixel 等価（別方式の完全一致）は DirectWrite の非決定な下位差（サブピクセル）を持ち込まない限り成立見込みだが、両方式で TextLayout 生成経路・スケール適用点を同一に保つ規律が要る（Medium 要因）。
  - 実 DPI（非 96）の正しさは自動 readback（k=1.0 恒常の現行契約）では証明されず手動確認が残る（R10.6・記憶 areka-placement-real-ghost-first）——DoD 申し送りで緩和。

## 7. Research Needed（design フェーズへ持ち越す調査項目）

- **RN1**: WUC 自前 swapchain の `ResizeBuffers`（or 再確保）で「既存描画内容を保全」する具体手段（`ICompositionSurface` 巻き直しの要否・`CreateCompositionSurfaceForSwapChain` を content 成長のたびに再取得するコスト）。surface.rs の現行 swapchain 生成経路（`create_composition_swap_chain`・BufferCount=2・flip model）を成長へ拡張できるか。
- **RN2**: `clip_sync_system` が content 子 visual の親（viewport スロット）に対して、emo-text の donor 装着（`VisualGraphics` 直挿入・`GraphicsCommandList` 不挿入）と両立して発火するか（Changed 検出タイミング・system 実行順）。viewport に clip を載せる場合の `Arrangement.size`（validrect 物理寸）と content の Arrangement（成長寸）の分離。
- **RN3**: content 子 visual の translate offset を「WUC visual の `SetOffset`（emo 直操作・現 surface.rs が `SetSize` を直操作するのと同型）」で持つか「ECS `Arrangement.offset`（wintf の offset 同期 system 経由）」で持つか。前者は再描画レス・低レイテンシだが wintf の GlobalArrangement とのズレに注意。
- **RN4**: 上限到達時のメモリ実量見積り（validrect 数百 px × 長大 talk の行数）と、SSP の `\_b --option=fixed` 固定層が想定する content 面規模（ukadoc/SSP 仕様の裏取り・mcp__ukadoc 参照余地）。
- **RN5**: pixel 等価比較を同一プロセス内 2 方式並走で行う場合の、両 `DrawExecutor`（再描画方式）と viewbox executor の TextLayout/format 経路完全共有（`create_text_format`・probe 経路）の担保方法。

---

## 8. 要件ディスカッション決定（議題1・2026-07-11）

要件ディスカッションで機構方式を確定した。**§4 の Option A/B/C（いずれも growable content 面＋visual offset 前提）ではなく、新たに導出した「ダーティ矩形スクロール（案C・`ScrollDC`／`InvalidateRect`／`WM_PAINT` 型）」を採用**する。

### 決定内容
- **固定 validrect 物理寸の単一描画面（ダブルバッファ）**。描画面は成長させない（Option A/B/C の growable 面を棄却）。
- **スクロール＝面内 whole-pixel blit**（確定済みピクセルの平行移動）。**ダーティ矩形（現在行 ∪ 露出帯）だけを D2D `DrawTextLayout` で（再）描画**し、確定 content は再描画しない。
- **D2D 描画コマンドを維持**：確定 content 用の別途グリフビットマップ／`ID2D1CommandList` キャッシュは設けず、**描画面＋blit を保持機構**とする（`ContentCanvas` がバッキングストア）。
- **サブピクセル維持**：ダーティ帯は真位置 D2D 描画・確定ピクセルは whole-pixel blit（再サンプリングなし＝ClearType 位相不変）。スクロール位置は f32 内部・写像は物理 px 整数（M2 補間シーム）。
- **pixel 等価**：k=1.0 で byte 一致（再描画方式 `DrawExecutor` を `#[cfg(test)]` 独立オラクル保持・同一プロセス live-diff が主檻）。k≠1.0 は小数アキュムレータで累積ドリフト回避＋≤0.5px・手動確認（R10.6）。

### 選定理由（負荷・前提の突合）
- 前提1「DPI サブピクセル維持」＋前提2「不可視領域を描かない」を最も literal に満たす。Option A（growable＋visual offset）は端数 translate でビットマップ再サンプル＝ClearType 滲みが生じ前提1に反する。案B（clip＋全再描画）は前提を満たすが確定 content を毎フレーム再コンポジット。
- 案C は per-frame 描画＝1 行・描画面メモリ＝固定・結線波及最小・最悪でも案B と同等（全ダーティ＝全再描画に縮退）。GPU 負荷でなく D2D CPU 発行が律速で、ダーティ限定でそれを抑える。

### DD への影響（design へ持ち越す項目の更新）
- **DD2/DD3（growable 面の実体・初期寸/成長単位/上限）→ 消滅**。描画面は validrect 固定・ダブルバッファ。上限値管理は不要（固定）。
- **DD5（viewport clip の実現手段）→ 変更**。visual レベルの `ClipShape`/`clip_sync_system` は M1 必須依存でない。封じ込め＝描画面寸、ダーティ限定＝D2D `PushAxisAlignedClip`。
- **DD1（offset 写像）→ 継続・具体化**。`{first_visible_line, block_offset}` を「面内 blit 量（whole-pixel）＋露出帯位置」へ写す式を design で確定。
- **DD4（増分追記・`line_cache` 再解釈）→ 継続**。「ダーティ矩形＝現在行 ∪ 露出帯」の判定と、多行同時 reveal／Clear 直後の全ダーティ縮退の扱い。
- **DD6（再描画レスの決定論観測）→ 継続**。`DrawTextLayout` 実行回数がダーティ矩形分に限られる檻。
- **DD7/DD8（example 差し替え・pixel 等価比較）→ 継続**。live-diff 主檻（k=1.0 byte）を確定。
- **DD9（縦書き成長方向）→ 変更**。growable 成長でなく「縦書き＝横 blit の向き・露出帯の辺」を確定。
- **DD10（固定層構造）→ 変更**。viewport 直下 child でなく「scroll 面に重ねる別合成層（blit 対象外）」。
- **新規 DD11**：重なり安全 blit のダブルバッファ運用（front→ずらし blit→back へ露出帯描画→present／swap）と、k≠1.0 の小数アキュムレータ（Bresenham 式で blit 距離を振る）による真位置格子吸着。
- **新規 DD12**：後方スクロール（もし将来）時の露出帯を `ContentCanvas` から再描画する経路（auto-follow talk では前方＝1 行帯ゆえ M1 非論点）。

### 副次的帰結
- brief.md の Approach「viewport visual＋content visual の 2 層」記述は**案C（単一 validrect 面＋面内 blit）に更新**（brief は次回同期時に追随・要件が正本）。
- RN1（growable swapchain の `ResizeBuffers`／内容保全）は**不要化**。RN2〜RN5 は案C向けに読み替え（clip_sync 依存は縮小・blit ダブルバッファの present 経路が新論点）。

---

## 9. 設計フェーズ Discovery & Synthesis（2026-07-11・design 生成時）

> Discovery 種別: **light（Extension）**——既存 emo-text-layer の分離シーム消費ゆえ integration-focused。外部調査（WebSearch）は不要（新規依存ゼロ・使用 API は D3D11 `CopyResource`/`CopySubresourceRegion`・D2D `PushAxisAlignedClip` の既知 primitive のみ）。

### 9.1 実シンボル再確認（コード突合・全て一致）

- `LayoutEngine::visible_window`（layout.rs:255）→ `VisibleWindow { first_visible_line: usize, block_offset: f32 }`（layout.rs:127–138）。**符号規約を実測確認**: 横書き＝負（内容が上へ）・vertical_rl＝正・vertical_lr＝負（doc 明記・layout.rs:130–137）——viewbox の blit 符号はこれを素通しする（独自軸規則を発明しない）。
- `ContentCanvas::from_layout`（canvas.rs:217）: 住人 transform＝validrect-local 平行移動（端数素通し）・住人 index＝行 index 1:1・`GlyphRunContent.size`＝行矩形寸——行指紋（内容＋位置＋寸）の材料が既に揃っている。
- `DrawExecutor::render`（draw.rs:523）: 透明 clear→`SetTransform(scale(k))` 一点→origin＝`transform.offset()`＋`block_offset` 軸加算→`DrawTextLayout`。**行 TextLayout キャッシュ `line_layout`（draw.rs:653）は行 index キー・内容不変再利用・`clear_cache` のみ全破棄**——そのまま `LineLayoutStore` として抽出可能（両方式の TextLayout 経路共有＝RN5 の解）。
- `TextSurface`（surface.rs:129）: 単一 `source_tex`（DEFAULT・RENDER_TARGET・premultiplied 透明初期化）・`present`＝`CopyResource`→`Present(0)`・`read_back`＝staging。**ダブルバッファ化は `sources: [tex; 2]`＋front index の内部拡張で外部契約（attach/present/read_back シグネチャ）不変**。
- Clear 適用点＝`TextLayerRuntime::apply_cue`（actor.rs:210–217）の `executor.clear_cache()`——viewbox では `request_clear()`（planner 初期化＋行キャッシュ破棄＋次フレーム FullClear）へ写像。
- `DrawExecutor` の外部消費者ゼロ（tests/・examples/・他 crate に direct 参照なし・grep 確認）——`#[cfg(test)]` オラクル化は破壊なしで可能。
- wintf `D2D1DeviceContextExt` に `PushAxisAlignedClip` の安全 wrapper は**ない**（command_types.rs は widget 描画経路用）→ viewbox_draw は windows-rs 直呼び（COM 層規律の範囲内・wintf 改変不要）。

### 9.2 RN1–RN5 の処置（§7 の読み替え結果）

- **RN1（growable swapchain の内容保全）**: 案C 採用で**不要化**（描画面は固定寸・成長しない）。
- **RN2（clip_sync 両立）**: **不要化**——visual クリップ非依存（封じ込め＝面寸・ダーティ限定＝D2D `PushAxisAlignedClip`）。donor 装着経路は不変。
- **RN3（visual offset の持ち方）**: **不要化**——スクロールは面内 blit（visual offset を使わない）。
- **RN4（メモリ上限・`\_b` 規模）**: 上限管理は**構造消滅**（固定寸 2 枚＋staging 1 枚）。`\_b --option=fixed` は「スクロールした時に画像を動かさない」＝scroll 面に描かない別合成層として `present` の合成点（front→backbuffer コピー直後）へ予約——ukadoc 裏取りは実装増分側の宿題として残る（M1 は型シームのみ）。
- **RN5（両方式の TextLayout/format 経路共有）**: **解決**——`create_text_format`（既存共有）＋`LineLayoutStore` 抽出（行 TextLayout 経路の一本化）＋専用 D2D DC の同一生成経路＋origin 同式＋`SetTransform(scale(k))` 一点則。live-diff は **in-source `#[cfg(test)]`**（`#[cfg(test)]` の DrawExecutor は統合テスト（tests/）から不可視のため——surface.rs 既存テストの headless World＋Compositor＋GraphicsCore パターンで実行可能なことを確認済み）。

### 9.3 DD の最終確定（design.md へ反映済み）

- **DD1（写像正準式）確定**: 真位置 `pos = block_offset × k`（軸: 横=y・縦=x・符号素通し）。量子化 `target = round(pos)`（真位置直接丸め＝増分丸めの累積なし・構造的ドリフトゼロ）・blit＝`target − committed`・不変条件 `|committed − pos| ≤ 0.5`。k=1.0 は行 pitch が `ceil` 由来の整数ゆえ `pos` 整数＝byte 一致の構造前提。
- **DD4（ダーティ判定）確定**: dirty＝露出帯 ∪ 変化行（前回 commit の行指紋 `(text, block_pos_bits, extent_bits)` と新 canvas の差分——リビール中の現在行・catch-up 複数行・新規行を一様検出）∪ 全域（Clear 後/format 組替後/初回）。ガード余白 `DIRTY_GUARD_IMG_PX`＝1 image px（AA こぼれ吸収・live-diff で検証・不足時は定数一点で増加）。描画対象＝dirty 交差の全住人（クリップで dirty 限定＝隣接行 AA こぼれの再現）。
- **DD5 確定**: visual クリップ不使用。ダーティ限定は「恒等変換下で物理整数矩形を `PushAxisAlignedClip`（**ALIASED**・格子厳密）→ `Clear`（クリップ内のみ）→ `SetTransform(scale(k))` → 描画 → Pop」。
- **DD6 確定**: `DrawStats`（常時コンパイル u64: line_layout_creations／draw_text_layout_calls／blits／full_clears）を正典とする（example からも読めるため。ログ捕捉 `with_log_cage` は補助）。
- **DD7 確定**: 既存 example を**本番経路差し替えで viewbox 化**（新 example は作らない）。再描画方式の観測は in-source live-diff 檻へ一本化（オラクル保全）。
- **DD8 確定**: (a) 同一プロセス live-diff が主檻（in-source `#[cfg(test)]`・k=1.0 byte 比較・横/縦 × あふれ前後/連続/Clear 前後）。golden bytes ファイル固定は不採用（DirectWrite の環境差に脆い）。既存 readback 述語（tests/）は無改変 green の従檻。
- **DD9 確定**: 縦書き＝横 blit（vertical_rl＝露出帯 左端・内容が右へ／vertical_lr＝逆）。面の成長概念は消滅ゆえ「成長方向」問題も消滅。
- **DD10 確定**: `FixedOverlaySeam`（zero-sized `#[non_exhaustive]`・surface.rs）＋ `present` の合成点命名（front→backbuffer コピー直後・Present 直前）。scroll 面（sources）には描かない＝blit 非対象・read_back（front）に影響しない＝R7.4 成立。
- **DD11 確定**: ダブルバッファ運用＝「front→back を blit ずらしコピー（blit=0 は全面 CopyResource）→ back へダーティ描画 → flip → present（front）」。back 全被覆不変条件（blit 写域 ∪ dirty＝面全域）で 2 フレーム前残像を構造排除。**plan/commit 二相**（COM 成功時のみ commit）で失敗フレーム再試行を決定論化。
- **DD12 確定**: 後方スクロールは M1 非論点（auto-follow 前方のみ）。露出帯描画は常に `ContentCanvas`（バッキングストア）由来のため、将来の後方スクロールも同経路で成立（構造は既に対応）。

### 9.4 Synthesis（設計統合の 3 レンズ）

- **Generalization**: blit 量・ダーティ導出を writing_mode 非依存の単一式（軸単位ベクトル×block_offset 素通し）へ畳む——layout.rs/visible_window と同じ「正規化ブロック座標」規律の踏襲。f32 真位置／whole-pixel 量子化の分離が M1 ステップと M2 補間の両方を同一 `plan/commit` インターフェイスで受ける（実装は M1 分のみ）。
- **Build vs Adopt**: 新規依存ゼロ。古典 `ScrollDC`/`WM_PAINT` 規律を D3D11 copy＋D2D clip で写す（プラットフォーム native 能力の採用）。wintf `ClipShape` は M1 不採用（要件 9.4 の確定・visual クリップは blit 方式に不要）。`ID2D1CommandList`／グリフ bitmap キャッシュは要件 3.4 で明示的に不採用。
- **Simplification**: growable 面・visual 2 層・上限管理・成長規則（旧 Option A/B/C の複雑さ）はすべて消滅。新規型は 4 つ（ScrollPlanner/FramePlan/ViewboxExecutor/LineLayoutStore 抽出）＋型シーム 2 つ（ScrollState/FixedOverlaySeam）に限局。オラクルは改変ゼロで `#[cfg(test)]` へ移すのみ。

### 9.5 リスク登録（design 反映済み）

- **ClearType 位相不変仮定**（整数平行移動＝ラスタ結果の平行移動）: live-diff byte 檻が第一防衛。破れた場合も全ダーティ縮退（＝レガシー等価の全域再描画）で正しさは保持（性能のみ劣化）。
- **AA こぼれのガード不足**: `DIRTY_GUARD_IMG_PX` 一点で調整可能・live-diff が検出器。
- **blit box の off-by-one**: 既知パターン直書き→blit→read_back の往復檻で殺す。
- **実 DPI（k≠1.0）**: 自動檻は述語（`|committed−pos|≤0.5` 恒真＋scale_invariance 資産）まで。非 96 DPI 実機の手動確認を実装フェーズ DoD 申し送り（R10.6・記憶 areka-placement-real-ghost-first）。

---

_本ドキュメントは情報提供（分析と選択肢）であり実装決定ではない。§8 は要件ディスカッションの決定記録・§9 は design フェーズの調査/統合記録（確定内容の正本は design.md）。_
