# ギャップ分析: areka-P0-present-gpu-transform-scale

> 2026-09-11・HEAD `67e0a4d3`・`kiro-validate-gap`。要件（`requirements.md`・付録 A/B）と現行コードの差を、file と「何の定義か」で裏取りして並べる。**本書は材料であって裁定ではない**——選択肢は §5 の議題として要件ディスカッションへ送る。

## 0. 要約（5 点）

1. **撤去対象は 1 本道に閉じている。** k 倍の CPU 経路は `apply_show` の非恒等 k 分岐（`FrameBudget::resample_native_into` → `areka_emo_compose::scale::resample_with`）→ その k 寸バイトからの `FrameBudget::regenerate_mask` → `ComposeCache::insert(.., scale, ..)` の 3 手であり、他に本番の消費者は無い（`resample` の非テスト消費者は `budget.rs` の `resample_with` と examples 2 本のみ・`presenter.rs` は既に `#[cfg(test)]` 束縛）。恒等 k の交代経路（`swap_native_scratch`）を**全 k の唯一の経路**にすれば、Requirement 1.3「k=1 と k≠1 で同じ手順」が構造で成立する。
2. **提示側の変換は wintf を触らずに置ける。** 供給面は `SwapChainPresenter`（自前 swap chain・premultiplied・`DXGI_SCALING_STRETCH`）→ `Compositor::CreateSurfaceBrushWithSurface` → `SpriteVisual::SetSize`（`VisualMount::attach`／`set_bounds`）である。pinned `windows 0.62.2`（feature `UI_Composition` 有効）に `CompositionSurfaceBrush::SetStretch(CompositionStretch::Fill)`・`SetBitmapInterpolationMode`・`Visual::SetScale(Vector3)`・`SetTransformMatrix` がすべて在り（`C:\rust\cargo\registry\src\index.crates.io-*\windows-0.62.2\src\Windows\UI\Composition\mod.rs`）、`crates/wintf/src` に `SetScale`／`SetTransformMatrix`／`SetStretch`／`SetBitmapInterpolationMode` の使用は **0 件**（要件の主張を再確認）。候補は 3 つ（§3.1）——**brush の stretch**（visual 寸＝物理寸・面＝原寸）／**visual の scale 行列**（visual 寸＝原寸・k を f32 で掛ける）／**D2D 描画面へ作り替え**。丸め権威（`scaled_extent`）と物理寸が 1 px も食い違わないのは brush stretch だけである。
3. **クリック透過の ÷k は既存の比例写像が「ほぼ」与える。** `alpha_mask_hit`（`crates/wintf/src/ecs/layout/hit_test/mod.rs`）は `mask_x = ((px − left) / bounds_w × mask.width) as u32` で、境界が物理寸・マスクが原寸なら整数 k で `ScaleRatio::unscale_coord` と**完全一致**、分数 k（5/4 等）で**境界 1 px 以内の差**（Requirement 4.3 の許容内）。wintf 側の判定手順の変更 0（Requirement 9.5）で足りるが、「厳密に `unscale_coord` と同じ」を求めるなら wintf へ k を運ぶ新経路が要る（§3.2）。
4. **決定論の檻は「再導出 34 本・撤去 25 本」の規模。** 付録 A の 11 行を file 単位で数え直した（§4）。撤去はリサンプラそのもの（`scale_resample_tests.rs` 16 本・`scale_prior_path_tests.rs` 6 本・`budget_tests.rs` の x 軸写像席 3 本）で、再導出は「供給面＝原寸・物理寸＝`scaled_extent`・マスク＝原寸バイト由来・k 変化＝ヒット・`resized`＝原寸外形の変化」の 5 つの新不変条件へ期待値を導き直すもの。判定側 `transition_judge`（`crates/areka/src/placement/transition_judge*.rs` の本番 3 本）は `resized` を読んでいない（grep 0 件）＝Requirement 7.6 は「確認して該当 0」で閉じる見込み。
5. **予算の見込みは静かな機械で 8〜12 ms・負荷 5 倍で 30 ms 超。** 撤去後の外れ 1 回＝compose 5〜7 ＋ mask（原寸＝画素 1/4）約 3 ＋ upload（原寸）約 0.1 ms。brief の 16.7 ms は静かな機械で成立の見込みだが、e2e 記録 §13.2 行 9 の「許容 30 ms」との関係、負荷下の扱いは裁定事項（§5 議題 7）。

**規模 M（3〜7 日）・リスク Medium**——製品コードの差分は小さく（撤去が主）、工数の大半は檻 59 本の再導出／撤去と実機 2 水準サインオフに掛かる。未知の技術要素は WUC brush の stretch と補間の実挙動（目視で確かめる 1 点）だけ。

---

## 1. 現状調査（Requirement ↔ 資産の対応）

### 1.1 提示段の流れ（`crates/areka-emo-present/src/presenter/show.rs` `EmoPresenter::apply_show`）

| 手順 | 定義 | いま | 本仕様後（候補） |
|---|---|---|---|
| (0) k 導出 | `derive_scale(target.policy, window_dpi)`（`crates/areka-emo-present/src/scale.rs`） | show 適用ごと | **不変**（Requirement 9.3） |
| (1) 引き当て | `ComposeCache::touch(surface_id, &binds, &pattern, scale)` | k がキー要素 | k をキーから外す（Requirement 5.2） |
| (1a) 合成 | `FrameBudget::native_scratch(|scratch| composer.compose_into(..))` | native 原寸 | **不変** |
| (1b) 表示バッファ | `FrameBudget::display_buffer(recycled)` → 恒等 k は `swap_native_scratch`／非恒等 k は `resample_native_into` | 分岐 2 本 | **交代だけ**（分岐消滅） |
| (1c) マスク | `FrameBudget::regenerate_mask(retired, display.bytes(), ..)` | k 寸バイト由来 | 原寸バイト由来（呼び手の引数が変わるだけ・生成コード不変） |
| (1d) 挿入 | `ComposeCache::insert(.., scale, display, mask, native_extent)` | エントリ＝k 寸面＋原寸 | エントリ＝原寸面（`native` フィールドは `composed` の外形と重複＝§5 議題 4） |
| (2) 遅延生成 | `SwapChainPresenter::new(gfx, &compositor, w, h)`・`VisualMount::attach(.., (w, h), ..)` | `(w, h)`＝エントリの composed 外形＝物理寸 | 供給面は原寸で生成・visual 寸は物理寸（§3.1 の候補で分かれる） |
| (3) 供給 | `chain.upload(&entry.composed)` → `size = chain.size()` → `size_changed = prev_physical != Some(size)` → `mount.set_bounds(world, size)` → `pending_resize` | `chain.size()` が物理寸の**単一真実源** | `chain.size()` は原寸になる＝物理寸は `scale.scaled_extent(native)` から**別に**導く（付録 B 項目 4） |
| 成立点 | `target.applied`・`target.native_size`・`info!` の `scaled_w/h = size` | `size`＝物理寸 | `scaled_w/h` は物理寸を出し続ける（Requirement 2.8）＝導いた物理寸を渡す |

**着眼**: `show.rs` の「表示物理寸は供給面の実寸を単一真実源とする」というコメント上の設計（(3) の `let size = chain.size()`）は本仕様で崩れる。代わりの単一真実源は `applied.scaled_extent(native)` であり、これは既に `prev_physical` の導出・`TextSlotView::physical_size`・`EmoPresenter::target_physical_size`（`presenter/read.rs`）が使っている式である。ゆえに「物理寸の式は 1 つ（`scaled_extent`）・供給面寸は原寸」に揃えると、`show.rs` 内の物理寸の導出点が 2 か所（`prev_physical` と新 `physical`）で同じ式になる——**式を 1 か所へ畳む**か（例: `PresentTarget` の小さなヘルパ）を設計で決める。

### 1.2 供給面と visual（`chain.rs`・`mount.rs`）

- `SwapChainPresenter`（`crates/areka-emo-present/src/chain.rs`）: `create_composition_swap_chain`（`crates/wintf/src/com/dxgi.rs`・B8G8R8A8・**premultiplied**・`DXGI_SCALING_STRETCH`・flip）＋ `source_tex`（真実源）＋ `staging`（readback）。`upload` は外形が変わったときだけ `ResizeBuffers`＋テクスチャ再作成。**供給面を原寸にすると k 変化で `ResizeBuffers` が走らなくなる**＝Requirement 7.6 の `resized` の意味変化はここに由来する。
- `VisualMount::attach`（`crates/areka-emo-present/src/mount.rs`）: `CreateSpriteVisual` → `CreateSurfaceBrushWithSurface(surface)` → `SpriteVisual::SetSize(Vector2{w,h})`（物理 px）→ `SetBrush` → `VisualGraphics::new(sprite)` を `physical_arrangement(size)`（`Arrangement{offset 0, scale 既定, size 物理}`）と同じ bundle で spawn。`set_bounds` は `Arrangement.size` を書き換え（bounds の権威）＋ `SpriteVisual::SetSize` を最善努力（失敗は `warn!`）。
- wintf の `visual_property_sync_system`（`crates/wintf/src/ecs/graphics/systems/visual_sync.rs`）は `Arrangement.offset` だけを WUC へ同期し、**size・scale は同期しない**。ゆえに「`Arrangement.size`＝物理寸（bounds・当たり判定の権威）」と「`SpriteVisual` の寸・brush の伸縮」は独立に決められる——§3.1 の候補 A はこの独立性に乗る。
- brush の伸縮は既定（`CompositionStretch::None` 相当）のまま・補間モードも既定のまま。**visual 寸＝供給面寸**で成立している現状は、寸が違えば「描画は左上原寸ぶんだけ」になる（`SetStretch` を明示しない限り拡大されない）——候補 A で `SetStretch(Fill)` が必須になる理由。

### 1.3 合成メモ（`crates/areka-emo-present/src/cache.rs`）

- `ComposeKey{surface_id, binds, pattern, scale}`・`CacheEntry{composed, mask, native}`・`CAPACITY = 3`・LRU（`touch` が唯一の順序更新・`get` は動かさない・`take_recycled`・`invalidate_all`）。
- k をキーから外すと `position`／`insert`／`touch`／`get` の 4 メソッドの引数から `scale` が消える。呼び手は `show.rs` の 4 か所（`touch`・`get`×2・`insert`）＋ `cache_tests.rs`（1,618 行・例外表かつ `CALIBRATION_DROPPED_ENTRY`＝**行数を増やせない**・引数削減で減る方向）。
- `CacheEntry.native` は「k 適用済み面と原寸の対」を保つために生まれた（cache.rs の `native` フィールド doc）。エントリが原寸になれば `composed.width()/height()` と同値になる——**残すか消すか**は §5 議題 4。`show.rs` の `target.native_size = cache.get(..).map(|e| e.native)` と `display_buffer` の `native: _` の 2 消費者。
- モジュール doc の「k 適用済み」「再サンプル」の記述（冒頭 §表示スケール k のキー参加・§責務分界・`CacheEntry`／`ComposeKey`／`insert`／`get` の各 doc）は Requirement 8.4 の書き換え対象。

### 1.4 予算席（`crates/areka-emo-present/src/presenter/budget.rs`）

- `AllocSite::{ComposeDst, ResampleDst, Xmap, Mask}`・`BudgetDelta`／`BudgetCounters` の 4 フィールド・席 3 つ（`SurfaceSeat`・`XmapSeat`・`MaskRotation`）。
- 撤去で消えるもの: `XmapSeat` 丸ごと・`resample_native_into`・`use areka_emo_compose::scale::{ResampleScratch, resample_with}`。`ResampleDst` は「表示バッファ（リサンプル先）の容量成長」だが、交代経路では表示バッファの成長は**次の適用の `ComposeDst`** に載る（budget.rs 冒頭「design.md との突合」節が既にそう説明している）＝撤去後は `ResampleDst`／`Xmap` が**構造的に常時 0**。
- Requirement 3.4 の二択（0 固定／消費者と同時撤去）の消費者側の実在: `tools/perf/judge-perf.py` の `J_PERF_STAGE_FIELDS`（`t_resample_us` を含む 6 個）・`J_PERF_ALLOC_FIELDS`（`alloc_resample_dst`・`alloc_xmap` を含む 4 個）＝`J_PERF_REQUIRED_FIELDS` で「1 つでも欠けた行があれば exit 2」。`tools/perf` の他スクリプト（`perf-compare.py`・`perf-rank.py`・`perf-ledger.py`・`judge-followup.py`）に当該フィールドの参照は **0 件**。`tools/perf/fixtures/*/run.log` の固定行は旧スキーマ（フィールドを**含む**）だが、判定は必須集合の**存在**しか見ないので、必須集合を縮めても旧 fixture は緑のまま。
- `timing.rs`（`Stage::Resample`・`Stage::COUNT = 5`・`emit_at` の `t_resample_us`）と `timing_tests.rs`（414 行）・`presenter_perf_log_tests.rs`（**956 行**＝1,000 行の見張りに最も近い触る予定のファイル）が対で動く。

### 1.5 k の数学（`crates/areka-emo-compose/src/scale.rs`・603 行）

- 残す: `ScaleRatio`（`new`／`mul`／`is_identity`／`as_f32`／`scale_len`／`scaled_extent`／`unscale_coord`）＝冒頭〜約 230 行。
- 撤去候補: `WEIGHT_*`／`AxisSample`／`AxisWalk`／`blend_axis`／`ResampleScratch`／`resample`／`resample_with`＝約 370 行、および `lib.rs` の `pub use scale::{ScaleRatio, resample}` の `resample`、`composed.rs` の doc 3 か所（`ResampleScratch::capacity` の参照・`resample_with` の呼び手説明）。
- **注意（`unscale_coord` の doc）**: 除算方向の丸め権威は自らを「[`resample`] が実際に用いた画素中心写像 `src = (v + 1/2)·den/num − 1/2` の最近傍整数」と定義している。リサンプラを消すと参照先が消えるが、**式そのものは残せる**——GPU の線形サンプリングもテクセル中心（+0.5）規約で `dst 中心 (d+½) → src (d+½)·den/num − ½` と同じ写像を使うため、「見えているとおりの部位が当たる」の根拠は brush stretch の下でも保たれる（**Research Needed**: WUC `CompositionSurfaceBrush` の Fill＋Linear がこの規約であることの確認は目視サインオフで足りるか、設計で明記する）。

### 1.6 当たり判定とクリック透過

- 領域判定: `EmoPresenter::hit_region_client`（`presenter/hit.rs`）→ `areka_emo_compose::hit_region_scaled`（`hit.rs`）→ `ScaleRatio::unscale_coord`。k は `PresentTarget::applied` の直読。**本仕様で不変**（Requirement 4.5）——マスクの寸に依存していない。
- クリック透過: `evaluate_targets`（`crates/wintf/src/ecs/clickthrough/controller.rs`）→ `hit_test_in_window`（client 物理 px ＋ `WindowPos.position` → screen）→ `hit_test`（`DepthFirstReversePostOrder` で `hit_test_entity`）→ `alpha_mask_hit(world, entity, &global.bounds, point)`。`AlphaMaskResource` を最優先で読み、`bounds` 内判定の後に **比例写像**（`rel = (p − left)/bounds_w`・`mask = (rel × mask.width) as u32`・切り捨て）→ `AlphaMask::is_hit`（範囲外は false）。
- 数値の突合（境界＝`scaled_extent(native)`・マスク＝原寸）:
  - k=2/1・native 8: 物理 16。比例写像は `⌊p·8/16⌋ = ⌊p/2⌋`。`unscale_coord(p) = ⌊(2p+1)/4⌋ = ⌊p/2⌋`。**完全一致**。
  - k=5/4・native 8: 物理 10。比例写像は `⌊p·0.8⌋`、`unscale_coord(p) = ⌊(2p+1)·4/10⌋ = ⌊0.8p + 0.4⌋`。p=2 で 1 と 2、p=7 で 5 と 6——**境界 1 px 以内で異なり得る**（Requirement 4.3 が退行としないと宣言した範囲）。
  - 高さの端数（native 6・k=5/4 → 物理 8＝7.5 の切り上げ）: 比例写像は `⌊p·6/8⌋ = ⌊0.75p⌋`＝厳密には ÷k（0.8）ではなく「÷(物理/原寸)」である。差は最終行付近の 1 px。
  - f32 の切り捨て誤差（`rel × width` が整数境界で 0.99999 になる）も同じ 1 px 帯に入る。
- `AlphaMaskResource` の doc（`hit_test/mod.rs`「マスク原寸＝bounds 寸に一致する運用を想定」）と `alpha_mask_hit` の doc（「emo-present では bounds==マスク原寸で恒等写像」）は本仕様で**事実と食い違う**——コード変更 0 でも doc の 2 行は書き換え対象（Requirement 9.5 は「判定手順」の変更 0 を求めており doc は含まない・設計で file 単位に列挙する）。

### 1.7 読み戻しと golden

- `EmoPresenter::read_back`（`presenter/read.rs`）→ `SwapChainPresenter::read_back`（`source_tex` → `staging` → `Map`・`stride = width×4` の密配列）。返るのは**供給面**であり、供給面が原寸になれば自動的に「原寸バイト」になる（Requirement 6.2 は実装ではなく定義の言い直し）。
- k=1 の golden（不変・Requirement 6.1）: `presenter_display_tests.rs` `golden_match_read_back_equals_direct_compose`・`chain.rs` の `upload_read_back_roundtrip_and_resize`・`crates/areka-emo-compose/src/golden_tests*.rs`（合成の golden・k を知らない）・`crates/areka/examples/emo-present/reconcile.rs` の恒等分岐。
- k≠1 を読む消費者: `presenter_test_support.rs` の `scaled_golden`／`scaled_golden_with`（`resample` で期待値を作る・present 側テスト 48 参照）・`examples/emo-present/reconcile.rs`（`resample(&golden, scale, ..)` で期待値を k 倍・長さ比較）・`examples/collision-probe/probe.rs` `assert_drawn_anchor`（「`read_back` は k 適用後の供給面」前提で Head／Bust 中心を物理座標へ写像して α を読む）。

### 1.8 上流 spec と裁量記録

- `.kiro/specs/completed/areka-P0-emo-dpi-scaling/design.md` §Key Design Decisions の表: D3（Strategy A2・却下欄「B＝WUC transform（マスク不整合が W5 境界侵食・鮮明性欠如）」）・D5（整数 bilinear・却下欄「GPU stretch（決定論 readback 檻と不整合）」）・D6（cache×再スケール・「k 変化＝ミス→再合成＋再サンプル」・2026-08-15 の容量 3 追記あり）。D1／D2／D4／D7／D8／D9／D10 は本仕様と無関係に成立し続ける。
- `.kiro/specs/completed/areka-P0-collision-dpi-hittest/`: design.md・requirements.md に「マスク」「mask」の記述は **0 件**（acceptance-record と brief に `info!` 文言「表示・マスクを更新」の引用があるのみ）＝Requirement 8.5 は「該当 0」で閉じられる。
- `doc/COMPAT_ARCHITECTURE.md` §8（沈黙ルール対応表）: 【上書き】行の先例は 3 行（`\_l` 座標の正典所有・`\_l` と `\n` の適用順・`font.height` の意味）で、いずれも「アーカイブ本体は改変しない（または注記 1 行のみ）・上書きの事実と出所を本表へ記す」形。本仕様の行は「画像本体は原寸・拡大縮小は提示側の変換行列・CPU 拡大は不可（開発者裁定 2026-09-11）」＋上書きされる側の出所（emo-dpi-scaling design D3／D5／D6）。**先例の作法との整合**: 先例 1（cursor-tag-canon）はアーカイブ非改変、先例 3（line-height-canon）はアーカイブへ注記 1 行——Requirement 8.1 は「D3／D5／D6 行に追記を置く」＝先例 3 の形（§5 議題 8）。
- プロジェクト記憶: `C:\Users\maz-o\.claude\projects\C--home-maz-git-areka\memory\areka-emo-own-compositor-atlas.md`（「DPI は合成に持ち込まない（表示側の責務）」「CPU vs D2D オフスクリーンは design 判断」の 2 文が本裁定と接続する）・同 `areka-dpi-following-core-design.md`（DPI 追従が基本設計）。MEMORY.md の索引行にも短い追記が要る（Requirement 8.3）。
- `.kiro/steering/roadmap.md`: 所有マップ行「画像は原寸で持ち拡大縮小は D2D 変換行列（2026-09-11 裁定・`present-gpu-transform-scale` が実装）」・表 #1（W13・M・Fable ○）・W14 ①（`dpi-transition-two-tick-bounce` は本仕様着地後に走行 D 形式で再計測）。e2e 記録 `acceptance-record.md` §13.2 行 9 に「目標 1 回 16 ms 以下・許容 30 ms 以下」。

### 1.9 その他の消費者（変更 0 の確認）

- `crates/areka/src/emo2_boot/frame/drain_resnap.rs`（`PhysicalSizeSource::physical_size` → `target_physical_size`）・`frame/dpi.rs`（`refresh_scale` の戻り＝`take_pending_resize` の物理寸 → `reconcile_window_size`）: 照会値の式が `scaled_extent` のまま変わらないため、**呼び手側の変更 0**（Requirement 2.2／2.4）。
- `crates/areka-emo-present/src/balloon.rs`・`presenter/hub.rs`・`presenter/visibility.rs`: `chain.size()` や物理寸を直接読む箇所 **0 件**。バルーン窓も同じ `apply_show` 漏斗を通るため、撤去は target 種別で分岐しない（Boundary Context「全 target」）。
- 文字層（`areka-emo-text`）: `TextSlotView::physical_size`／`scale` を読むだけ（`crates/areka/src`・`areka-emo-text/src` の非テスト消費者 25 か所は API 名を変えないので不変）。

---

## 2. 要件ごとの実現性とギャップ

| Req | 技術的な必要 | 現状の資産 | ギャップ／制約 | 種別 |
|---|---|---|---|---|
| 1.1〜1.3 | 原寸の面を上げる・k は変換の係数のみ | 恒等 k の交代経路 `swap_native_scratch` が「原寸をそのまま上げる」形を既に持つ | 非恒等 k の分岐を消し、交代を全 k の経路にする（show.rs の `if scale.is_identity()` が消える） | Constraint（既存パターンの一般化） |
| 1.4 | `resample`／`resample_with`／`ResampleScratch` の撤去・消費者 0 の機械確認 | 消費者は budget.rs・test_support・examples 2 本・compose の lib 再輸出 | 撤去後の grep を DoD に載せる（`grep -rn "resample" crates --include=*.rs` の残りが doc 0 件・コード 0 件） | Missing（検査台本） |
| 1.5 | `ScaleRatio` の権威を残す | scale.rs 冒頭 230 行で自立 | `unscale_coord` doc の `resample` 参照を式のまま言い直す（§1.5） | Constraint |
| 2.1／2.2 | 物理寸＝`scaled_extent`・照会値不変 | `target_physical_size`・`TextSlotView::physical_size` は既にこの式 | show.rs の物理寸の真実源を `chain.size()` から式へ置き直す（付録 B 項目 4） | Missing（1 か所） |
| 2.3／2.5 | 窓寸・配置・相対配置の不変 | 合成済み 1 枚へ単一 k（変換の適用先が 1 visual） | 候補 A／B とも 1 枚へ 1 つの係数＝相対配置は自明 | — |
| 2.4 | DPI 変化の同一フレーム追従 | `refresh_scale` → `apply_show`（ヒットで再合成なし） | 供給面は再 upload されるが `ResizeBuffers` は走らない・`set_bounds(物理)` だけが変わる | — |
| 2.6 | 補間 bilinear 相当以上 | brush 既定は Linear | 明示するか既定に任せるか（§5 議題 2） | Unknown（目視で確定） |
| 2.7／2.8 | 実機 2 水準・`info!` の `scaled_w/h`＝物理寸 | D10 の `info!` 行・`AREKA_APP_SMOKE_EXIT_MS` の有界起動（`emo-dpi-scaling`／`collision-dpi-hittest` の受け入れ記録に手順あり） | `scaled_w/h` の供給元を `chain.size()` から導いた物理寸へ差し替える（フィールド名・意味不変） | Constraint |
| 3.1／3.2／3.5 | 16.7 ms・catch-up ≤10・分布の報告 | `perf(apply_show)` 行・`judge-perf.py`・e2e 記録の前回値 | 判定の物差し（16.7 と 30）は §5 議題 7 | Unknown（裁定） |
| 3.3／3.4 | perf 行の維持・撤去段の扱い | `timing.rs`／`judge-perf.py` の契約面 | 二択（§5 議題 5） | Unknown（裁定） |
| 3.6 | 定常確保 0 | `FrameBudget` の席と `presenter_budget_steady_state_tests.rs` | 席が 1 つ減る（`XmapSeat`）・計数の意味は不変 | — |
| 4.1／4.4 | 原寸バイトから 1 回・原子対 | `regenerate_mask` は引数のバイト列から作るだけ | 引数を `display`（＝原寸）に差し替えるだけ | — |
| 4.2／4.7 | ÷k 照会・二重縮約の禁止 | `alpha_mask_hit` の比例写像（§1.6） | 「比例写像に任せる」か「照会口を明示」か（§5 議題 3）・二重縮約の檻は `hit_test_in_window` を k=2 で通す決定論テストを present 側に新設（GPU 不要: `AlphaMaskResource`＋`Arrangement` だけの World で足りる） | Unknown（裁定）＋Missing（檻） |
| 4.3／4.5／4.6／4.8 | 許容の明記・領域判定不変・バルーン不変・実機 | `hit_region_client` の `debug!` 行 | 明記のみ | — |
| 5.1〜5.6 | エントリ原寸・k をキーから外す・容量 LRU 不変・保持量減 | cache.rs（§1.3） | `native` フィールドの去就（§5 議題 4）・保持量は k=2 で 10.3 MB → 約 2.6 MB（382×547×4×3＋詰めマスク） | Missing（改変 1 file） |
| 6.1／6.2 | k=1 golden 不変・`read_back`＝原寸 | §1.7 | 定義の言い直し＋ `read.rs` の doc | — |
| 6.3／6.4／6.8 | 既存テストの裁定材料 | §4 の台帳 | 裁定は開発者（§5 議題 4′） | Unknown（裁定） |
| 6.5 | 新設分岐の決定論テスト | 既存の `make_world_with_gpu` 型（別プロセス）・純関数の in-crate | 「変換の係数の適用」は WUC を要する＝offscreen readback では**係数は読めない**（`read_back` は原寸面）。係数の適用は `Arrangement.size`（物理）と `SpriteVisual` 寸の**整合**を檻に入れる形になる（WUC の brush stretch そのものは檻に入れない・Requirement 6.6） | Constraint |
| 6.7 | 1,000 行 | show.rs 486・cache.rs 378・budget.rs 657・mount.rs 462・**perf_log_tests.rs 956**・budget_tests.rs 1,081（例外）・cache_tests.rs 1,618（例外＋較正） | 再導出で増やせないのは perf_log_tests.rs（残 44 行）と例外表の 2 本 | Constraint |
| 7.1〜7.4 | 変換設定の失敗＝`error!`＋`Err`・前状態維持・分岐を増やさない | `device_err` の写像（mount.rs／chain.rs）・`set_bounds` の `SetSize` 失敗は現状 **`warn!` の最善努力** | 候補 A では「変換の設定」＝attach 時の brush 2 呼出（失敗は attach 失敗＝既存の `Err` 経路）＋ k 変化時の `SetSize(物理)`（現状 warn）。**7.1 を字義どおり満たすには `set_bounds` の失敗を `Err` へ格上げする必要があるが、`set_bounds` は upload の後に呼ばれる**ため「表示は適用前の状態を保つ」が構造で言えない（§5 議題 6） | Unknown（裁定） |
| 7.5 | `info!` フィールド不変 | show.rs 成立点 | 供給元の差し替えのみ | — |
| 7.6 | `resized` の意味 | `transition_judge.rs`／`_verdict.rs`／`_offset.rs` に `resized` の読み手 0 件 | 「確認して該当 0」＋ present 側の `a_scale_change_records_the_buffer_resize` を再導出 | — |
| 8.1〜8.7 | 登記 | §1.8 | 文書作業のみ・作法は §5 議題 8 | — |
| 9.1〜9.7 | 変更 0 の境界 | compose の `lib.rs` 再輸出と `composed.rs` doc は触る（9.1 の「plan／blit／compose_into 不変」には抵触しない） | 設計で「触る file」を列挙（compose: `scale.rs`・`lib.rs`・`composed.rs` doc・テスト 2 本／wintf: doc 2 行のみ or 0） | Constraint |

---

## 3. 実装アプローチの選択肢

### 3.1 提示側の変換の置き場（付録 B 項目 1・Requirement 2.1／2.6／7.1）

| 候補 | 何を変えるか | 物理寸の厳密性 | 失敗経路 | 触る file | 所見 |
|---|---|---|---|---|---|
| **A. surface brush の stretch** | `VisualMount::attach` で `brush.SetStretch(CompositionStretch::Fill)`（＋任意で `SetBitmapInterpolationMode`）。`SpriteVisual::SetSize` と `Arrangement.size` は**従来どおり物理寸**、供給面だけ原寸 | **厳密**——visual の寸が `scaled_extent` の整数 px なので、拡大結果の外形は丸め権威と 1 px も違わない | 変換の設定は attach の 1 回（`device_err` で `Err`）。k 変化は `set_bounds` の `SetSize` のみ（現状 warn） | mount.rs（＋2〜3 行）・show.rs・chain.rs は不変 | 最小差分。WUC が「面の寸≠visual の寸」を Fill で伸縮する挙動は D3D の `DXGI_SCALING_STRETCH` の swap chain と組み合わせて成立する見込みだが**実機目視で 1 度確かめる**（Research Needed） |
| **B. visual の scale 行列** | `SpriteVisual::SetSize(原寸)`＋`Visual::SetScale(Vector3{k, k, 1})`（または `SetTransformMatrix`）。`Arrangement.size` は物理寸のまま（bounds の権威） | **非厳密**——k は `as_f32` で渡すため外形は `native × k` の f32（例 27×7/6＝31.5 px）。`scaled_extent` は 32。境界と描画の外形が半画素ずれ、端が AA で滲む | k 変化ごとに `SetScale` を呼ぶ＝Requirement 7.1 の「変換の設定失敗」が k 変化のたびに起こり得る経路になる（設計しやすい） | mount.rs（`set_scale` 新設）・show.rs | 「行列」という開発者の言葉に最も近いが、`as_f32` を寸法・画素演算に使う禁止（`ScaleRatio::as_f32` doc・唯一の例外は emo-text の供給面寸）に**抵触する**。採るなら例外の裁定が要る |
| **C. D2D 描画面へ作り替え** | swap chain＋surface brush を `CompositionDrawingSurface`（物理寸）＋`BeginDraw`→`SetTransform(scale)`→`DrawBitmap(原寸 bitmap)` に置き換える。原寸のバイトは D2D bitmap（`CreateBitmap`／DXGI 面から）へ | 厳密（描画面が整数の物理寸） | D2D 呼出ごとに `Err` 化できる | chain.rs 全面・mount.rs・wintf の `com/wuc.rs` interop（既存 `begin_draw` が使える） | 「D2D の変換行列」の字義には最も忠実だが差分が最大。`read_back` が物理寸（GPU 補間後）へ戻り、Requirement 6.2「供給面＝原寸」と**衝突**する（原寸 bitmap を別途読む口が要る） |

補間モード（Requirement 2.6）: `CompositionBitmapInterpolationMode::{NearestNeighbor, Linear, MagLinearMinLinearMipLinear}` の 3 値（windows 0.62.2）。`Linear` が bilinear 相当＝旧 D5 の見た目に最も近い。`MagLinearMinLinearMipLinear` は縮小時の mip を要求するが swap chain 面に mip は無い＝実質 `Linear` と同じ。`NearestNeighbor` は整数 k で最も鮮明だが分数 k で画素幅ムラ（2.6 が禁止）。「整数 k だけ Nearest」は k による分岐の新設＝Requirement 7.3 の精神（恒等 k の特別扱い禁止）に近い判断が要る。候補 A では **既定が Linear** なので、明示しなくても 2.6 は満たす（明示すれば契約が読める）。

### 3.2 原寸マスクの ÷k 照会（付録 B 項目 2・Requirement 4.2／4.7）

| 候補 | 内容 | 差 | 触る file |
|---|---|---|---|
| **α. 比例写像に任せる** | `alpha_mask_hit` 不変。境界＝物理寸・マスク＝原寸で ÷(物理/原寸) が自動で掛かる | 整数 k で `unscale_coord` と一致・分数 k と端数行で 1 px 以内の差（4.3 の許容） | wintf コード 0・doc 2 行 |
| **β. 照会口を明示** | wintf の surface entity に k（または「マスク寸と bounds 寸の比」）を持つ component を足し、`alpha_mask_hit` が `unscale_coord` 相当の整数式で引く | `hit_region_client` と**同じ丸め**になる（境界画素まで一致） | wintf `hit_test/mod.rs`・新 component・present の `set_bounds`／`attach` で k を書く＝Requirement 9.5「判定手順の変更 0」と衝突 |
| γ. 1 チャンネルの k 倍マスク | brief の代替案。原寸 α を 1 bit で k 倍に伸ばす | 恒等写像に戻る | Requirement 4.1「k 倍バイト列由来のマスクを作らない」と衝突＝要件と両立しない |

二重縮約の檻（4.7）: present crate に「k=2・物理 bounds・原寸マスク」の World（GPU 不要）を組み、`hit_test_in_window` の結果を原寸 α の ÷2 で予言した表と突き合わせる 1 本。既存の `mount_visibility_tests.rs`／`attach_fixture` の型が使える。

### 3.3 全体の形（gap-analysis 規則の A／B／C）

- **A. 既存コンポーネントの改変（撤去中心）**——本仕様の形はこれ。新設は「変換の設定」（mount.rs 数行）と決定論テストのみ。差分は撤去が主で、改変対象は present crate の 6 file（show.rs・cache.rs・budget.rs・mount.rs・timing.rs・read.rs doc）・compose crate の 3 file（scale.rs・lib.rs・composed.rs doc）・examples 2 file・`judge-perf.py`（議題 5 次第）。
- B. 新コンポーネント——不要（変換は WUC の既存 API・席の新設も無い）。
- C. 段階化——「①撤去＋供給面原寸化 → ②檻の再導出 → ③実機サインオフ → ④登記」の順序は自然に段階化されるが、①と②を分けてコミットすると①だけの状態で檻が赤になる（tasks で束ねる）。

**規模 M・リスク Medium**（根拠: 製品差分は 10 file 弱で撤去が主＝Low 寄り／檻 59 本の裁定と再導出＋実機 2 水準の目視＋WUC brush stretch の実挙動 1 点が未確認＝Medium）。

---

## 4. 付録 A の再点検（file 単位の実数と再導出の方向）

| 付録 A # | file（行数） | 本数 | 実数点検の結果 | 再導出の方向（案） |
|---|---|---|---|---|
| 1 | `crates/areka-emo-compose/src/scale_resample_tests.rs`（835） | 16 | 全てリサンプラの性質（恒等コピー・外形＝`scaled_extent`・golden・premultiplied・エッジクランプ・作業席） | 撤去候補。`resample_extent_matches_scaled_extent` の「外形」だけは `scaled_extent` 単体の檻（`scale_tests.rs` 相当）が担う |
| 2 | `scale_prior_path_tests.rs`（565） | 6 | 是正前の参照実装とのバイト等価 | 撤去候補（参照対象が消える） |
| 3 | `presenter_dpi_scale_tests.rs`（852） | 11 | k≠1 を読むのは `show_surface_scales_display_to_scaled_extent_at_k2`・`target_physical_size_uses_rounding_authority_and_matches_view_and_chain`・`same_scale_hits_cache_and_window_dpi_change_misses_and_resamples`・`native_size_*` 2 本 | `chain.size()`＝原寸／`target_physical_size`＝`scaled_extent`／`Arrangement.size`＝物理寸／**k 変化＝ヒット**（`cache_hit=true`・compose 段 0）へ |
| 4 | `presenter_fractional_scale_tests.rs`（718） | 4 | `alpha_mask_bits_come_from_k_scaled_display_bytes`（期待＝`from_pbgra32(k 倍バイト)`）は新正典と正反対 | マスク期待＝`from_pbgra32(native)`・bounds＝`scaled_extent`・「k=5/4 で 8×6 → 面 8×6・bounds 10×8」へ |
| 5 | `presenter_budget_equivalence_tests.rs`（600） | 3 | 期待値の生成に `scaled_golden`（`resample`）を使う | 期待＝native の合成バイト（`resample` 不要・簡単になる） |
| 6 | `presenter_budget_steady_state_tests.rs`（661）・`presenter/budget_tests.rs`（1,081・例外表） | 3＋21 | 席の檻のうち `the_resample_scratch_is_the_budgets_own_seat_not_a_throwaway`・`the_resample_scratch_seat_reaches_its_width_once_and_never_regrows`・`an_identity_scale_*` 2 本（「恒等 k だけ交代」の前提） | 席 3 本は撤去・「恒等 k だけ」の檻は「全 k で交代」へ言い直し・定常確保 0（k=2）は不変 |
| 7 | `presenter_perf_log_tests.rs`（**956**） | 6 | `identity_scale_miss_reports_exact_zero_resample_stage_and_no_resample_allocs`・`miss_apply_emits_..._every_stage_and_alloc_wired`（k≠1 外れで `t_resample_us` 非零を期待） | 議題 5 の裁定に従う（0 固定なら「全 k で 0」・撤去ならフィールド走査から外す）。**行数を増やせない** |
| 8 | `presenter_refresh_and_log_tests.rs`（760）・`presenter_visibility_tests.rs`（784）・`presenter/transition_record_tests.rs`（714）・`presenter_resize_report_tests.rs`（265） | 各 1〜2 | `refresh_scale_after_dpi_change_reapplies_new_k`・`external_reshow_while_invisible_updates_scale_bounds_and_mask`（bounds／マスク寸）・`a_scale_change_records_the_buffer_resize`（`resized=true`・w/h＝物理）・`dpi_change_reports_new_physical_size_to_caller` | 戻り値・窓寸報告は不変。`a_scale_change_records_the_buffer_resize` は「k 変化で `resized=false`・w/h＝原寸」へ（Requirement 7.6）。マスク寸は原寸へ |
| 9 | `presenter_read_accessor_tests.rs`（451） | 1 | `visible_surface_hit_uses_applied_scale_at_k2` | 不変 |
| 10 | `crates/areka/examples/emo-present/reconcile.rs`・`examples/collision-probe/probe.rs` | — | `resample` で golden を k 倍・`assert_drawn_anchor` が物理座標へ写像 | golden＝native（`resample` 消滅・k=1 の比較は不変）・anchor は原寸座標で直接読む（写像が消えて簡単になる）。`examples/emo-present.rs` のモジュール doc 10 行も書き換え |
| 11 | `crates/areka/src/emo2_boot/frame_dpi_tests.rs` | 1 | 窓寸のみ | 不変 |

数え直し: **撤去候補 25 本**（#1 16・#2 6・#6 3）／**再導出候補 34 本**（#3 5・#4 4・#5 3・#6 2＋（steady 1）・#7 6・#8 5・#10 の example 2 本相当・`timing_tests.rs` の段数 6 本相当は議題 5 次第）／不変 2。

---

## 5. 設計判断へ送る議題（裁定は開発者・本書は材料のみ）

1. ~~**変換の置き場**~~ → **裁定 D（2026-09-11・要件ディスカッション 議題 1）**: A／B／C は emo の自前経路の中で選ぶ案だったが、開発者の指摘で「emo-present が wintf の DPI 機構を三重に迂回している（物理寸 `Arrangement`・`GraphicsCommandList` 不使用・自前 swap chain）」ことが根因と確定。**D＝wintf のコマンドリスト経路へ戻す**（原寸 D2D bitmap を論理 px の宛先矩形で `DrawBitmap` 記録・`Arrangement` は論理寸・拡大は `render_surface` の `SetTransform`）を採る。A は D が実測で躓いたときの退避案。B／C は却下。§3.1 の表は経緯として残す。
   D の Research Needed（§6 へ追加）: ⑴ 作者側補正（96／author_dpi × app_scale）を宛先矩形で吸収したとき `scaled_extent` と物理寸が一致するか ⑵ WUC 描画面の全面再描画（`BeginDraw`→`DrawImage`）が毎コマ走る代金（旧 upload 0.3 ms 相当で収まるか） ⑶ `ID2D1DeviceContext::CreateBitmap`（premultiplied BGRA・メモリ直渡し）の代金と、合成メモに bitmap を持たせる要否 ⑷ `Visual` の `on_add` が連鎖挿入する `SurfaceGraphics`／`SurfaceGraphicsDirty` に emo entity を素直に乗せられるか（`mount.rs` 冒頭 doc の「衝突しない」前提が逆になる）。
2. **補間モード**（Requirement 2.6）: Linear 固定（既定＝呼出 0 でも成立・明示すれば契約が読める）／整数 k だけ Nearest（鮮明だが k 分岐の新設）。
3. ~~**÷k 照会**~~ → **α で確定**（Requirement 4.2・裁定 D では wintf の `BitmapSource` と同形＝原寸マスク＋物理 bounds）。
4. **`CacheEntry.native` の去就**: 消す（`composed` の外形と同値・cache_tests.rs が縮む）／残す（差分最小・重複した真実源が残る）。
   4′. **付録 A の 59 本の裁定**（Requirement 6.3）: 撤去 25／再導出 34 の分類案（§4）をテストごとに確定する。特に `alpha_mask_bits_come_from_k_scaled_display_bytes`（正反対）と `a_scale_change_records_the_buffer_resize`（`resized` の意味変化）。
5. **撤去段の perf フィールド**（Requirement 3.4）: 0 固定（`Stage::Resample`・`alloc_resample_dst`・`alloc_xmap` を残し常時 0・`judge-perf.py` 不変・語彙が死ぬ）／消費者と同時撤去（`timing.rs`・`budget.rs`・`timing_tests.rs`・`perf_log_tests.rs`・`judge-perf.py` の必須集合 2 タプルを更新・旧 fixture は余分なフィールドとして無害・Revalidation Trigger に該当）。
6. **Requirement 7.1 の実現形**: 候補 A では変換の設定は attach の 1 回に畳まれ、k 変化時に失敗し得るのは `set_bounds` の `SetSize`（現状 `warn!`）だけ。`warn!` のまま「変換の設定失敗＝attach 失敗」と読み替えるか、`set_bounds` を `Result` にして upload の**前**へ移す（前状態維持を構造で言うため・`test-cage-determinism` ④の観測点＝upload のエラー分岐の字面を動かさない制約に注意）か。
7. ~~**受け入れの物差し**~~ → **要件ディスカッションで確定（2026-09-11・自明修正）**: brief の 16.7 ms（静かな機械の 1 コマ）が合格線・e2e 記録の「許容 30 ms」は置き換わる（記録は非改変）・負荷下の値は Requirement 3.5 で報告のみ（Requirement 3.1 に明記）。撤去後の見込みは静かな機械で 8〜12 ms（compose 5〜7 ＋ mask 約 3 ＋ upload 約 0.1）。
8. ~~**登記の作法**~~ → **要件が既に採択済み**: Requirement 8.1 が「D3／D5／D6 行に追記を置く」＝先例 3（line-height-canon）の形を指定している。議題として閉じる。
9. **物理寸の単一真実源の置き直し**（付録 B 項目 4・§1.1 着眼）: `show.rs` の `size_changed`／`pending_resize`／`info!` の `scaled_w/h` が読む物理寸を `chain.size()`（原寸になる）から `applied.scaled_extent(native)` へ移す。式は `target_physical_size`／`TextSlotView::physical_size` と同じなので **1 か所のヘルパへ畳む**（例: `PresentTarget` の小さなメソッド）か、`prev_physical` と新 `physical` の 2 か所で同じ式を書くか。

## 6. Research Needed（設計フェーズへ持ち越す未確認）

- WUC `CompositionSurfaceBrush::SetStretch(Fill)` が swap chain 由来の `ICompositionSurface` に対して期待どおり伸縮すること、および `Linear` の標本点がテクセル中心規約（`unscale_coord` の式と同じ写像）であること——実機 1 度の目視＋スクリーンショットで確かめる（決定論の檻には入れない・Requirement 6.6）。
- 候補 A で k 変化時に `SpriteVisual::SetSize` だけを変えたとき、WUC が同一フレームで再合成すること（`Commit` 廃止＝vsync tick の暗黙反映・steering tech.md）。
- マスク生成（`AlphaMask::from_pbgra32`・画素ごとに `get` を呼ぶ詰め込み）の原寸での実測（k=2 で 11 ms → 1/4 画素で約 3 ms の見込み）。高速化は本仕様の範囲外（brief ⓐ の精神）だが、16.7 ms の内訳として数字を取る。
- `judge-perf.py --selftest` が必須集合の縮小後も緑であること（議題 5 で撤去を採る場合）。

---

## 7. 設計フェーズの調査ログ（2026-09-11・`kiro-spec-design`・裁定 D の実装可能性を file と「何の定義か」で裏取り）

> §1〜§6 は要件フェーズの材料。本節以降は裁定 D（wintf のコマンドリスト経路へ戻す）を前提に、設計 (a)〜(l) を確定するために調べ直した事実と決定である。**§3.1 の候補 A〜C・§5 議題 1 の A 系の記述は経緯として残し、以下が上書きする。**

### 7.1 wintf の DPI 伝播はゴースト窓には届いていない（設計 (a) の前提を覆す事実）

- **Context**: 要件「根因」節は「スケールは Window エンティティの `Arrangement.scale` に DPI から入って子へ累積伝播する」と書く。emo entity が `Arrangement.scale = 1.0` のままで wintf の伝播に乗れるかを確かめた。
- **Sources**: `crates/wintf/src/ecs/layout/systems/taffy_systems.rs` `update_arrangements_system`（`With<TaffyStyle>` フィルタ・Window かつ `DPI` 有りのときだけ `LayoutScale{dpi.scale_x, dpi.scale_y}` を書く）／`crates/wintf/src/ecs/layout/systems/window_pos_systems.rs` `sync_window_arrangement_from_window_pos`（offset のみ）／`crates/areka/src/placement/spawn.rs` モジュール doc「`BoxStyle`（論理 DIP）と `DragConstraint` は一切付けない（U2・DD8）」／`crates/wintf/src/ecs/layout/arrangement.rs` `impl Mul<Arrangement> for GlobalArrangement`。
- **Findings**: `Arrangement.scale` を書く本番コードは `update_arrangements_system` の 1 か所だけで、対象は `TaffyStyle` を持つ entity に限られる。ゴースト窓（キャラ窓・バルーン窓）は placement が `BoxStyle` を付けずに spawn するため `TaffyStyle` を持たず、窓の `Arrangement.scale` は既定 `(1.0, 1.0)` のまま **DPI が入らない**。ゆえに現行 `mount.rs` の「`Arrangement` を物理寸で直接設定すれば `GlobalArrangement.bounds` が物理寸になる」は、窓のスケールが 1.0 だからこそ成立していた。
- **Implications**: 裁定 D で「拡大は wintf の `render_surface` の `SetTransform(GlobalArrangement のスケール)`」に委ねるには、**emo の surface entity 自身の `Arrangement.scale` に k を書く**しかない（`GlobalArrangement = 窓 GA(スケール 1.0) × emo Arrangement`）。作者側補正（96／author_dpi × app_scale）と窓 DPI（窓 dpi／96）は既に `derive_scale` が 1 つの k に畳んでいるので、「補正の吸収先」問題は消え、**k の政策・導出点は 1 文字も変わらない**（Requirement 9.3 がそのまま満たされる）。宛先矩形に k を掛ける案（`Arrangement.size` が物理寸になり `SetTransform` は恒等）は、裁定 D の「`Arrangement` は論理寸」に反するので採らない。ゴースト窓へ `BoxStyle`／`TaffyStyle` を付けて wintf に DPI を書かせる案は placement の U2 を覆す wintf／placement 側の変更＝Requirement 9.5 に反するので採らない。

### 7.2 スケジュール順と表示の着地時刻（+1 tick）

- **Sources**: `crates/wintf/src/ecs/world/mod.rs` の tick 本体（`Input → Update → PostLayout → GraphicsSetup → Draw → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize` の順に `try_run_schedule`）と schedule 登録（`PreRenderSurface`: `visual_resource_management_system` → `deferred_surface_creation_system` → `mark_dirty_surfaces` → `cleanup_surface_on_commandlist_removed`／`RenderSurface`: `render_surface`／`Composition`: `visual_hierarchy_sync_system` → `visual_property_sync_system`）／`crates/areka/src/emo2_boot/mod.rs` の `add_systems(FrameFinalize, emo2_frame_system.before(apply_zorder_chain))`。
- **Findings**: 提示段の `apply_show` は tick の**最後**の schedule（`FrameFinalize`）で走る。挿した `GraphicsCommandList`・書き換えた `Arrangement` は次の tick の `PostLayout`（伝播）→ `PreRenderSurface`（面の生成／寸合わせ）→ `RenderSurface`（描画）で消費される。現行の `SwapChainPresenter::upload` は `Present(0)` を同 tick 内で出す。
- **Implications**: 画素が画面に載る時刻は「apply の tick の次の vsync」から「次の tick の描画の次の vsync」へ **1 tick（60Hz で 16.7 ms）遅れる**。表示成立点（`info!` 行・状態・照会値・窓寸 reconcile 要求）は従来どおり同一 tick 内で成立するため Requirement 2.4 の「同一フレーム内で表示を成立」はこの意味で満たすが、**見た目の着地は +1 tick** である。文字層（emo-text）は自前 swap chain で同 tick に present するため、面切替のコマだけ絵が文字より 1 tick 遅れて着く。drain 相を `Draw` より前へ動かせば消えるが、それは areka の frame 配線（`FrameFinalize` の排他 system・z 順の鎖との順序）の変更であり本仕様の範囲外。設計ディスカッションへ「既知の帰結」として提示し、下流 `dpi-transition-two-tick-bounce` の再計測に含める。
- **tick 門との関係**: `crates/wintf/src/ecs/world/tick_gate.rs` `should_run` は `gate_enabled == false`（既定 OFF・`AREKA_TICK_GATE`）なら常に `Run`。門が採用された場合、apply の次の tick を予約する旗が無いと描画が来ない。旗 `REARM`（「まだ仕事がある」）の先例は `crates/areka/src/emo2_boot/frame/scale_text.rs`（`tick_wake::mark(tick_wake::REARM)`）。生産者は wintf `tick_wake.rs` の doc 行にファイル名で列挙し、areka 側は `crates/areka/src/tick_gate_config_producers_tests.rs` の `AREKA_PRODUCERS` が字面で検査する。

### 7.3 コマンドリスト経路のレシピ（`BitmapSource` からの lift）

- **Sources**: `crates/wintf/src/ecs/widget/bitmap_source/systems.rs` `draw_bitmap_sources`（`dc.CreateCommandList` → `SetTarget(&command_list)` → `set_transform(identity)` → `BeginDraw` → `draw_bitmap(bitmap, Some(dest_rect＝Arrangement.size の論理 px), 1.0, HIGH_QUALITY_CUBIC, None, None)` → `EndDraw` → `command_list.close()` → `GraphicsCommandList::new`・既存と `!=` のときだけ `insert`）／同 `create_d2d_bitmap`（`D2D1_BITMAP_PROPERTIES1{B8G8R8A8_UNORM, PREMULTIPLIED, dpi＝dc.GetDpi, BITMAP_OPTIONS_NONE}`）／`crates/wintf/src/com/d2d/mod.rs` `D2D1DeviceContextExt::{set_transform, draw_bitmap}`・`D2D1CommandListExt::close`／`crates/wintf/src/ecs/graphics/core.rs` `GraphicsCore::device_context`（共有 DC）／`windows 0.62.2` `ID2D1DeviceContext::CreateBitmap(size: D2D_SIZE_U, sourcedata: Option<*const c_void>, pitch: u32, bitmapproperties: *const D2D1_BITMAP_PROPERTIES1) -> Result<ID2D1Bitmap1>`（`C:\rust\cargo\registry\src\index.crates.io-*\windows-0.62.2\src\Windows\Win32\Graphics\Direct2D\mod.rs`）／`crates/wintf/src/ecs/graphics/systems/render.rs` `render_surface`（`begin_draw(None)` → `SetTransform{M11=scale_x, M22=scale_y, M31/M32=offset}` → `clear(透明)` → `DrawImage(command_list, LINEAR, SOURCE_OVER)` → `end_draw`）。
- **Findings**: emo に要るのは「WIC 由来の bitmap」を「メモリ直渡し（`CreateBitmap`・premultiplied BGRA・pitch＝`ComposedSurface::stride`）」に替えるだけで、記録手順は逐語で lift できる。宛先矩形を明示する `DrawBitmap` は bitmap の DPI に依らず矩形へ伸縮する（`create_d2d_bitmap` の dc.GetDpi 指定はそのまま踏襲して差分 0）。bitmap は記録したコマンドリストが参照を保持するため、キャッシュに bitmap を別に持つ必要は無い。`GraphicsCommandList`（`crates/wintf/src/ecs/graphics/command_list.rs`）は `#[derive(Component, Debug, Clone, PartialEq)]` で `empty()` 構築子を持つ＝GPU 無しのテストでも値として扱える。
- **Implications**: 新設は `record_display(dc, &ComposedSurface) -> Result<GraphicsCommandList, PresentError>` 1 関数（＋純関数の記録レシピ）で足りる。補間モードは記録時の `DrawBitmap` の引数が支配する（`render_surface` の外側 `DrawImage` は LINEAR 固定）。

### 7.4 `Visual::on_add` に emo entity を乗せる（設計 (h)）

- **Sources**: `crates/wintf/src/ecs/graphics/visual.rs` `on_visual_add`（`Arrangement` 不在なら既定挿入・`owner_window_exists`（`ChildOf` を辿って `Window` を探す）が真なら `VisualGraphics::default()`／`SurfaceGraphics::default()`／`SurfaceGraphicsDirty::default()` を、常に `BrushInherit` を挿す）／`crates/wintf/src/ecs/graphics/visual_manager.rs` `visual_resource_management_system`（`Changed<VisualGraphics>` かつ `!is_valid()` で `CreateSpriteVisual`）／`crates/wintf/src/ecs/graphics/systems/surface.rs` `deferred_surface_creation_system`（`Or<(Changed<GlobalArrangement>, Changed<GraphicsCommandList>)>`・`calculate_surface_size_from_global_arrangement` で寸を決め `CreateDrawingSurface(B8G8R8A8, Premultiplied)`・`SpriteVisual::SetSize(面寸)`・`SetBrush`・`SurfaceGraphicsDirty` を進める）／同 `mark_dirty_surfaces`（`Changed<GraphicsCommandList>`／`Added|Changed<SurfaceGraphics>`／`Changed<GlobalArrangement>`）／`crates/areka/src/placement/spawn.rs`（ゴースト窓は `Window` component を持つ）。
- **Findings**: surface entity を「`Visual` ＋ `Arrangement`（自前）＋ `GraphicsCommandList` ＋ `HitTest` ＋ `AlphaMaskResource` ＋ `ChildOf(窓)`」で spawn し、**`VisualGraphics` を自前で入れなければ** `on_add` の連鎖がそのまま乗る。SpriteVisual の生成・寸・brush・面の生成／リサイズ・再描画のトリガはすべて wintf の既存 system が担う。`mount.rs` 冒頭 doc の「非衝突」節（自前 `VisualGraphics::new(sprite)` で `on_add` の既定挿入を避け、`GraphicsCommandList` を入れないことで `deferred_surface_creation_system` を発火させない）は**正反対**になる。
- **Implications**: `VisualMount` から COM 呼び出し（`CreateSpriteVisual`／`CreateSurfaceBrushWithSurface`／`SetSize`／`SetBrush`）が全て消え、`attach` は純 ECS の spawn になる（テストは GPU 不要になる）。text-layer slot（兄弟・上位 z）は不変。同一エントリの再適用で `GraphicsCommandList` を挿し直すと `Changed` が立って全面再描画が毎コマ走るため、`BitmapSource` と同じく **値が異なるときだけ挿す**（`PartialEq`）。`Arrangement` も同値なら書かない（`Changed<Arrangement>` → `GlobalArrangement` → 再描画の連鎖を止める）。

### 7.5 物理寸の丸めの一致（設計 (a) 後半・Requirement 2.1/2.2）

- **Sources**: `crates/wintf/src/ecs/graphics/systems/init.rs` `calculate_surface_size_from_global_arrangement`（`bounds` の幅高を `ceil()` して `u32`・0 以下は `None`）／`crates/areka-emo-compose/src/scale.rs` `ScaleRatio::scale_len`（`(2·len·num + den) / (2·den)`＝round half away from zero）／`arrangement.rs` `Mul`（`bounds.right = left + size.width × result_scale_x`・f32）。
- **Findings**（`bounds` 幅＝`native × k_f32`・面寸＝`ceil`・照会値＝`scaled_extent`）:

  | native | k | native×k（厳密） | wintf 面寸（ceil） | `scaled_extent` | 差 |
  |---|---|---|---|---|---|
  | 382×547 | 2/1 | 764×1094 | 764×1094 | 764×1094 | 0 |
  | 382×547 | 5/4 | 477.5×683.75 | 478×684 | 478×684 | 0 |
  | 5 | 5/4 | 6.25 | 7 | 6 | **+1** |
  | 27 | 7/6 | 31.5 | 32 | 32 | 0 |
  | 31 | 7/6 | 36.17 | 37 | 36 | **+1** |
  | 6 | 7/6 | 7（f32 積 6.9999998） | 7 | 7 | 0 |

  整数 k では常に一致。分数 k では小数部が (0, 0.5) のとき wintf が **1 px 大きい**（`ceil` と round の差）。f32 の表現誤差（k の f32 が真値より大きい側にあるとき整数積が `N+ε` になる）も同じ +1 の帯に入る。負の側（wintf が小さい）は起きない（`ceil ≥ round`）。
- **Implications**: 差は Requirement 2.1 が退行としない 1 px に収まる。**照会値・窓寸 reconcile は `scaled_extent`（整数の丸め権威）のまま**、wintf の描画面はそれ以上に 1 px はみ出し得るが、窓 client（`scaled_extent` 寸）がその 1 px を切り落とすので見えない。当たり判定の境界 `bounds` は `native × k` の f32 そのもの（ceil 前）なので、比例写像は厳密に ÷k（f32）になる。「wintf を触らず両者を完全一致させる」手段は無い（`ceil` は wintf の規約）ため、一致は「整数 k で厳密・分数 k で ≤1 px」を檻に入れて閉じる。

### 7.6 swap chain ヘルパの消費者（Requirement 1.4 の「消費者 0 なら撤去」）

- **Sources**: `grep -rn "create_composition_swap_chain\|create_composition_surface_for_swap_chain" crates --include=*.rs`。
- **Findings**: `crates/areka-emo-present/src/chain.rs`（撤去対象）・`crates/areka-emo-present/tests/swapchain_spike.rs`（chain の spike＝撤去対象）・**`crates/areka-emo-text/src/surface.rs`（文字層の供給面・不変）**。
- **Implications**: 撤去後も消費者が 1 つ残る（emo-text）ため、wintf の `com/dxgi.rs` `create_composition_swap_chain`・`com/wuc.rs` `create_composition_surface_for_swap_chain` は**撤去しない（wintf コード変更 0）**。

### 7.7 テストの前提（GPU 要否）

- **Sources**: `crates/areka-emo-present/src/presenter_test_support.rs` `make_world_with_gpu`（`GraphicsCore::new` HARDWARE ＋ `WucGraphicsResource` を World へ挿す・窓無し・画素は読まない）／`mount_test_support.rs` `attach_fixture`（現行は `SwapChainPresenter::new` を要する＝GPU）／`chain_fault_tests.rs`（`fault_point` 注入・11 組合せ）／`crates/areka-emo-present/src/presenter_upload_failure_tests.rs`（3 本）。
- **Findings**: 現行の presenter 檻 48 本は全て `make_world_with_gpu` 型（GPU の資源生成は要るが描画結果は読まない）。裁定 D 後にミス経路が触る GPU は `CreateBitmap`＋コマンドリスト記録だけで、`render_surface` は檻の外（wintf の system を回さない）。`VisualMount` は COM を触らなくなるので mount 系は **GPU 不要**になる。
- **Implications**: 新設・改変の判断分岐は「純関数（記録レシピ・論理 `Arrangement`・丸めの一致表・÷k 写像・perf フィールド集合・`resized` 述語）＝GPU 不要」と「提示段の流れ（k 変化ヒット・失敗経路・`read_back` 等価）＝既存の `make_world_with_gpu` 型」に分ける。Requirement 6.5「GPU を要する確認はオフスクリーン readback の既存の型に限る」は後者（資源生成のみ・画素読み戻し無し）をこの型と読む。失敗注入は `chain.rs` の `fault_point`（test ビルドのみ実体）を新モジュールへそのまま移す。

### 7.8 行数の見張りへの波及

- `crates/log-capture-kit/tests/file_length_guard_test.rs` `OVER_LIMIT_ALLOWED` は「例外表に載っているのに超過の実体が無い」エントリを**赤にする**（stale 検査）。`presenter/budget_tests.rs`（1,081 行）はリサンプル席の檻 3 本を撤去すると 1,000 行を下回り得る＝その場合は表から外し `OVER_LIMIT_ALLOWED_COUNT` を 11→10 にする（表と件数の 2 か所）。`cache_tests.rs`（1,618 行）は引数削減で縮むが 1,000 行は下回らない（`CALIBRATION_DROPPED_ENTRY` の較正も不変）。走査対象は `.rs` のみ（`tests/workspace_scan/mod.rs`・`name.ends_with(".rs")`）ゆえ `judge-perf.py`（4,153 行）は対象外。

## 8. 設計判断（(a)〜(l)・design.md の正本を要約）

| 項 | 決定 | 根拠（要点） |
|---|---|---|
| (a) k の置き場 | surface entity の `Arrangement{offset 0, scale (k,k), size native}`・宛先矩形は `(0,0,native_w,native_h)` | §7.1（窓のスケールは 1.0）・§7.5（丸め ≤1 px）。`ScaleRatio::as_f32` の 2 つ目の裁定済み消費者（変換行列の係数＝寸法演算ではない）として doc に登記 |
| (b) 補間 | `D2D1_INTERPOLATION_MODE_LINEAR` を `DrawBitmap` に記録・全 k で同一 | 旧 D5（bilinear）と同じ見え方・`render_surface` の外側と同じ定数・k で分岐しない（7.3）。cubic は premultiplied α 端でのリンギングを避けて採らない（1 定数の差なので実機で望めば変えられる） |
| (c) perf | `Stage::Resample`・`t_resample_us`・`alloc_resample_dst`・`alloc_xmap` を**撤去**し `judge-perf.py` の 2 タプルを同時更新・`t_upload_us` は名を保ち意味を「原寸 bitmap 生成＋命令記録」へ（ヒットは 0） | 0 固定は死語彙（「表示するだけの数は必ず古びる」）・旧 fixture は余分フィールドとして無害・`--selftest` 緑を DoD |
| (d) メモ | `CacheEntry{composed, mask, display: GraphicsCommandList}`・`ComposeKey` から `scale` を外す・`native` フィールドは外形と同値ゆえ消す | k 変化＝ヒットで D2D 作業 0・bitmap はリストが保持・`GraphicsCommandList::empty()` で GPU 無しの cache 檻が組める |
| (e) `read_back` | `last_show` のキーで引いたエントリの `composed.bytes()` を返す（k 非依存）。エントリが消えていれば `error!`＋`Err` | 表示面は書込専用・cage ④ の観測点は `record_display` の失敗注入（4 点）へ移る |
| (f) 遷移観測 | `stage=upload` 行は `w/h`＝物理寸（`scaled_extent`）のまま・`resized`＝原寸の外形が前回表示から変わった回 | 判定器の読み手 0（要件 7.6）・`SurfaceStage::Upload` の語は契約ゆえ保つ |
| (g) 失敗 | `record_display` の失敗（`CreateBitmap`／`CreateCommandList`／`EndDraw`／`Close`）は `device_err` 経由で `error!`＋`Err(Device)`・**`take_recycled` より前**に行い表示・メモ・World は全て適用前のまま | 合成失敗と同じ規律（R3.4）・k 分岐なし・panic なし |
| (h) spawn | `Visual`＋`Arrangement`（論理）＋`GraphicsCommandList`＋`HitTest`＋`AlphaMaskResource`＋`ChildOf(窓)`・`VisualGraphics` は入れない・同値なら挿し直さない・成功時に `tick_wake::REARM` | §7.4・§7.2 |
| (i) 触る file | wintf コード 0・doc 3 行（`hit_test/mod.rs` 2・`tick_wake.rs` 1）・emo-text 0・balloon.rs 0・areka は examples 2＋doc 1 | §7.6・バルーンは同じ漏斗 |
| (j) 檻 | 撤去 44 本（compose 22・chain 14・spike 1・perf 1・budget 2・cache 4）／再導出 34 本／新設 10 本（T-N1〜T-N9・T-G1）／不変 127 本（`#[test]` の実数え・design.md 付録 A′） | 陳腐化テスト方針 |
| (k) 行数 | 新 `display.rs` 約 150 行・触る file は全て 1,000 未満・`budget_tests.rs` の例外表からの除外に注意 | §7.8 |
| (l) 登記 | D3／D5／D6・Option D への追記文と COMPAT §8 の【上書き】行の文言を design.md に固定 | 先例 3（line-height-canon）の作法 |

## 9. リスク（設計フェーズ）

- **+1 tick の着地遅れ**（§7.2）——実機サインオフの目視で「絵と文字の 1 コマずれ」が見えるかを確かめる。見えるなら drain 相の位置は別 spec（`dpi-transition-two-tick-bounce` の再計測と同じ走行で採る）。
- **WUC 描画面のリサイズ頻度**——k 変化のたびに `deferred_surface_creation_system` が面を作り直す（旧 `ResizeBuffers` と同じ頻度・稀）。原寸の変化（面切替で外形が違う面）でも作り直しが走る＝旧 `upload` の外形変化時と同じ回数。
- **`tick_wake` 生産者の登記漏れ**——`show.rs` を wintf `tick_wake.rs` の REARM 行へ載せる。areka 側の字面検査（`AREKA_PRODUCERS`）は areka 内ファイルしか読まないため、emo-present の生産者はそこには載らない（載せない理由を design に書く）。
- **`as_f32` の禁止則との整合**——変換行列の係数への使用を 2 つ目の裁定済み例外として `ScaleRatio::as_f32` の doc へ明記しないと、次の読み手が「禁止の違反」と読む。
