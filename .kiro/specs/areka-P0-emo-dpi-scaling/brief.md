# Brief: areka-P0-emo-dpi-scaling

> **種別**: 本坑（main）新規。⑥ emo 帰属（**DPI追従レンダリング基盤＝render-scaling foundation**）。2026-07-18 `collision-geometry` Task 4.2 実 DPI 受け入れ却下から派生（開発者指示: 必要なだけ新 spec を立て roadmap 化・依存調整・引き継ぎ網羅）。
> **調査日**: 2026-07-18（DPI追従スコープ調査ワークフロー・7エージェント配管精読）。一次分析は `specs/completed/areka-P0-collision-geometry/research.md §13`。
> **上流**: emo 合成基盤（`completed/areka-P0-emo-atlas`/`-emo-compose`/`-emo-present`）／wintf `DPI` component（既存・`GetDpiForWindow`／`WM_DPICHANGED` ライブ更新）。
> **下流（必須依存）**: **`areka-P0-collision-dpi-hittest`**（当たり判定 ÷k がこの `scale()`→k と実 k× 表示に依存）／DPI追従が波及する全 emo 消費者（`window-placement` 窓寸・`emo-text-layer` 行寸・balloon 寸・`choice-render`）＝各 spec の Revalidation Trigger。
> **M1/M2 配置・collision-geometry 合流**: **未決＝別セッションの計画判断**（[[portfolio-convergence-decided-in-separate-session]]）。DPI追従は開発者言明の**基本設計**だが emo2 は k=1.0 でも E2E 実走する（M1 blocker か否かは要判断）。

## Problem

areka の**基本設計は DPI追従**（画面 DPI に追従してマスコット/サーフェスが拡大縮小する・SSP の固定px等倍とは**異なる思想**・[[areka-dpi-following-core-design]]）。ところが現状、emo 層は **k=1.0 がコンパイル時定数でハードワイヤ**され、マスコットは高 DPI モニタでも拡大しない（固定物理 px）。この「途中状態」が `collision-geometry` Task 4.2 の実 DPI 受け入れを**不成立**にした——モニタ DPI を 2 水準（125%/200%）変えてもマスコットが同一物理寸ゆえ、DPI追従下（scale≠1.0）の当たり判定が全く検証できなかった。

**実測した配管（2026-07-18 調査・file:line）**:
- 合成スケール `CURRENT_COMPOSE_SCALE: f32 = 1.0`（`presenter.rs:126`）→ `TextSlotView.scale` へ唯一代入（`:427`）。`scale()` は素の値返し（`:116-122`）。DPI 由来経路なし。
- 合成 extent `compute_extent`（`plan.rs:366-383`）は native px の (0,0) union・k 乗算なし。`blit.rs:69-163` は 1:1 整数 SourceOver コピー＝**リサンプラ不在**。
- 下流寸法は全て composed extent 従属（k=1.0）: swapchain（`presenter.rs:268-311`/`chain.rs:133-170`）・visual/`Arrangement`（`mount.rs:60-72` `LayoutScale::default()`）・窓 HWND（`follow.rs:551-628` `resize_window_to`）。
- **per-window DPI は入手可能・未消費**: wintf `DPI` component（`dpi.rs:21-28` `scale_x/y()`・`GetDpiForWindow` 実値補正 `window_handle.rs:223-238`・`WM_DPICHANGED` ライブ更新 `window_pos.rs:285-343`・public re-export `ecs/mod.rs:46`）。presenter は `window: Entity`（`:157-161`）も `&mut World`（`apply` `:186`）も持つが `world.get::<DPI>(window)` を読んでいない。design の単一変更点は `TextSlotView::scale()`。
- **wintf は既に k≠1.0 レンダリング実績あり（greenfield でない）**: `DPI`→Window `Arrangement.scale`（`taffy_systems.rs:214-225`）→`GlobalArrangement.transform`（`arrangement.rs:196-234`）→`render.rs:95-111` `dc.SetTransform` の D2D 適用。emo-present は意図的にバイパス中（`mount.rs:60-72`「論理/物理混在事故の構造的排除」）。

## Desired Outcome

emo が surface を **k = monitorDPI ÷ author_dpi** で実際に拡大レンダリングし、`scale()` がその k を返す。マスコットが高 DPI モニタで DPI 相当に拡大表示され、窓/swapchain もそれに追従する。

**✔ 観測（単一 pass/fail の候補・design で確定）**: 実 DPI（≠96）2 水準で本番 emo2 を表示し、マスコットが各 DPI 相当寸で描画される（125%→約1.25×・200%→約2×）ことを実機で確認（`GetClientRect` ＝ round(k × surface_px)・`scale()==k`）。純粋層のリサンプル正しさ（Strategy A）は決定論 unit（オフスクリーン readback・[[areka-no-ci-gpu-tests-in-cargo-test]]）で網羅。**本番ゴースト先行**（[[areka-placement-real-ghost-first]]）。

## Approach（design で A/B 確定）

- **Strategy A（emo-compose で k× 鮮明ラスタ・emo 思想と整合）**: `compute_extent`（`plan.rs:366-383`）を k 倍 ＋ `blit.rs:69-163` に**リサンプラ新設**（唯一の実作業）。swapchain/visual/窓/AlphaMask は composed extent 従属ゆえ自動追従。emo 自前合成・鮮明性担保（[[areka-emo-own-compositor-atlas]]）と一致。
- **Strategy B（WUC transform で完成1枚を bitmap-stretch・軟い/低コスト）**: `mount.rs` の SpriteVisual サイズ＋`Arrangement.scale`、`follow.rs` 窓寸、`presenter.rs` の scale publish に k を分散。swapchain は surface px のまま。wintf 既存 `SetTransform` 経路の再利用。
- **共通(b)**: presenter が `world.get::<DPI>(window)` を読み、`PresentTarget` に per-target `scale` を持たせ（現状 scale フィールド無し `presenter.rs:52-77`）、`CURRENT_COMPOSE_SCALE` 定数を per-target k へ昇格。`scale()` が k を返す（design 宣言の単一変更点）。

## 開発者が決める設計論点（design ディスカッションへ）

1. **author_dpi の定義**: k の分母。ukadoc/emo2 標準（SSP は 96 前提？shell descript で宣言？）か固定 96 か。正典は ukadoc（[[ukadoc-mcp-preferred-source]]）。k の意味を確定。
2. **整数倍か連続か**: 100/150/200% の段階か連続 k か。リサンプラ選択（nearest/bilinear）と下流の丸めを左右。
3. **Strategy A vs B**: 鮮明ラスタ（`blit.rs` リサンプラ）か WUC transform（軟い/低コスト）か。emo 思想は A。
4. **WM_DPICHANGED ライブ再スケール**: モニタ跨ぎ移動時の k 再導出＋再合成（`window_pos.rs:285-343`）。collision-geometry Revalidation Trigger 2 の 7.3 probe 再実行と同席。
5. **入れ子/mayuna との相互作用**: element 入れ子アニメ・mayuna 着せ替えも同 k で合成されるか。

## Scope

- **In**: emo surface の k× レンダリング（A or B）／`scale()` が window DPI 由来 k を返す／窓・swapchain の k× 追従／実 DPI 実機観測。
- **Out**: 当たり判定の点÷k（**`areka-P0-collision-dpi-hittest`**）／SHIORI・撫で意味論／DPI追従の他消費者波及の**実装**（`window-placement`/`emo-text-layer`/balloon/`choice` の revalidation は各 spec の Revalidation Trigger）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo-compose`・`-emo-present`・`-emo-atlas`（合成基盤）／wintf `DPI` component（既存・consume するだけ・新規依存なし）。
- **Downstream**: `areka-P0-collision-dpi-hittest`（`scale()`→k＋実 k× 表示に依存）／revalidation: `completed/areka-P0-window-placement`（窓寸）・`areka-P0-emo-text-layer`（行寸）・balloon・`choice-render`。

## Constraints

- Rust 2024・tokio 不使用。wintf `DPI` は既存 consume（新規依存なし）。決定論 unit（Strategy A のリサンプルはオフスクリーン readback で網羅・[[deterministic-test-coverage-mandate]]）＋実 DPI 実機観測（[[areka-placement-real-ghost-first]]）。
- 正典は ukadoc（author_dpi・scale 挙動・[[ukadoc-mcp-preferred-source]]）。emo2 は最小適合 fixture。
