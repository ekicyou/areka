# Brief: wintf-ulw-removal（本坑 / main）

> **種別**: 本坑（main）。通常の kiro ライフサイクル（requirements → design → tasks → impl → complete）。PR ベース squash マージで `main` へ統合。
> **位置づけ**: M1（emo2-boot）とは別軸の **wintf 基盤層**。表示レイヤーの合成方式を GPU 合成（DComp、後に WUC）単独へ一本化するための **ULW 撤去**。
> **前提依存（順序ゲート）**:
> ```
> _Depends: wintf-clickthrough-alpha-toggle（✅ 2026-07-02 完了＝ゲート解除・着手可）
> ```
> **ULW を安全に消せるのは、本坑クリックスルー（`WS_EX_TRANSPARENT` 動的トグル）が完了し「ULW 無しでも別プロセスクリック透過が成立」と確認できてから** → **✅ 充足**（2026-07-02 完了・areka の窓は既に DComp モード＋clickthrough 登録で運転）。`wintf-dcomp-to-wuc-migration` も **✅ 完了**（GPU 合成パスは WUC 化済み・本 spec の「残す側」）。着手判断（並走検証期間の長さ）は開発者。

## Problem

表示レイヤーの透過は現在 **ULW/DComp 切替式**で、ULW アームは CPU ビットマップ方式（`UpdateLayeredWindow`）。ULW は DComp スワップチェーン合成と併用不可で、別プロセス透過のために GPU 描画を諦める踏み絵になっていた。本坑クリックスルー（`WS_EX_TRANSPARENT` 動的トグル）が **DComp/WUC 描画を維持したまま別プロセス透過**を成立させれば、**ULW は不要**になる。ULW 一式（専用 compositor・GDI 経路・`CompositionMode` の ULW 分岐）を撤去し、表示バックエンドを **GPU 合成単独**へ一本化して、切替式に伴う重複経路・分岐を消して表示レイヤーを単純化したい。

## Current State（調査済み・ULW 専用コードの所在）

| 対象 | 役割（ULW 専用） | ファイル |
|---|---|---|
| `CompositionMode` enum | ULW（既定）/ DComp の二択・生成時固定 | `ecs/window/components.rs` |
| `compute_ex_style()` の ULW 分岐 | ULW→`WS_EX_LAYERED` 保持 | `runtime/window_factory.rs` |
| `WindowD3D11Compositor` | ULW 合成器（D2D bitmap＋GDI HBITMAP＋DIBSection・DComp 非使用） | `ecs/graphics/compositor.rs` |
| ULW compositor init | `WindowD3D11Compositor` 生成/リサイズ（ULW モードのみ） | `ecs/graphics/compositor_systems/init.rs` |
| ULW compose→present | サブツリーを D2D bitmap へ→HBITMAP 転送→`UpdateLayeredWindow` | `ecs/graphics/compositor_systems/render/mod.rs` |
| ULW ユーティリティ | `transfer_to_hbitmap`（D2D→DIB）・`present_layered_window`（ULW） | `com/ulw.rs` |
| 「ULW 一択」記述 | 別プロセス透過は実質 ULW 一択、と断定 | `tech.md` line 83 / `roadmap.md` line 30 |

- **GPU 合成パス（本 spec で残す・触らない）**: WUC 移行済みの graphics 系（`wintf-dcomp-to-wuc-migration` ✅ が確立）。※ 本 brief 旧版の `com/dcomp.rs` 等の列挙は WUC 移行前の記述——残す側の正確なファイル群は design 冒頭で再確認。
- **クリックスルーのα源（✅ 2026-07-03 検証済み・確定）**: 完了した clickthrough 実装は **per-widget αマスク（`AlphaMask::is_hit`）のみ**を α源に使い、ULW compositor の D2D1 staging αバッファは**一切参照しない**（`ecs/clickthrough/controller.rs`・`ecs/layout/hit_test/mod.rs` 実コード確認）。→ `compositor.rs` 撤去はクリックスルーを壊さない。旧「design で確認」条件は充足済み。
- **⚠️ `WS_EX_LAYERED` 同伴フラグの帰属（新規・重要）**: pilot REPORT の必須配合どおり、`WS_EX_TRANSPARENT` 動的トグルには **`WS_EX_LAYERED` を同伴フラグとして立てる必要がある**（ULW/SLWA 非呼出・描画には使わない）。現状これは clickthrough の `apply_layered_companion()`（`ecs/clickthrough/controller.rs:171-180`）が実行時に適用しており、`compute_ex_style()` は DComp モードで LAYERED を**付けない**（`window_factory.rs:64-73`）。**ULW 撤去後、DComp/WUC 窓への `WS_EX_LAYERED` の唯一の源は clickthrough 機構になる**——撤去がこの経路を巻き込まないこと・clickthrough 登録窓が LAYERED を受け取ることを受け入れ基準に含める。

## クロスユニット契約（後続を詰ませない事前考慮・2026-07-03）

- **`CompositionMode` collapse は破壊的変更**: areka 側の呼び出し（`crates/areka/src/main.rs` の `CompositionMode::DComp` 指定等）と、着手予定の `areka-P0-emo-present`／`areka-P0-window-placement` が同じ API に触れる。**順序調整が理想**（本ユニット先行→emo/ghost 系が新 API で書く）。並行着手する場合は「collapse 後の追随はどちらが行うか」を着手時に確定（rebase 責務の明確化）。
- 撤去で `WS_EX_LAYERED` の帰属が clickthrough 機構単独になる点は Current State 記載のとおり受け入れ基準に含める（emo-present のクリック透過観測が依存）。

## Desired Outcome

ULW 一式が撤去され、表示バックエンドが **GPU 合成単独**（DComp、`wintf-dcomp-to-wuc-migration` 後は WUC）へ collapse。`CompositionMode` は単一化（単一 variant なら enum 撤去、または最小化）。**残す GPU 合成パスの描画結果・挙動は不変**。ビルド通過・起動して同一描画結果。当たり判定・ウィンドウ管理・スレッド構成は不変。

## Approach

1. **ULW 参照の全数洗い出し**: `CompositionMode::ULW`・`WindowD3D11Compositor`・`compositor_systems`・`com/ulw.rs`・`compute_ex_style` の ULW 分岐・ULW を前提にした初期化/スケジュール登録を grep で漏れなく特定。
2. **専用コード削除**: `ecs/graphics/compositor.rs`・`compositor_systems/`・`com/ulw.rs` を撤去。ECS スケジュールから ULW system 群を除去。
3. **`CompositionMode` collapse**: ULW variant 除去。GPU 合成単独になった時点で enum を撤去 or 単一値へ最小化（生成時のデフォルトを GPU 合成へ）。`compute_ex_style` の ULW 分岐（`WS_EX_LAYERED`）を除去し、GPU 合成の ex_style（`WS_EX_NOREDIRECTIONBITMAP`）へ一本化。
4. **ドキュメント正本更新**: steering（tech/product/roadmap）の「ULW 一択」記述は **2026-07-01〜03 に撤回・更新済み**——本 spec では残余（`doc/COMPAT_ARCHITECTURE.md` ほか doc 配下・コード内コメント）の最終整合のみ。
5. **非破壊検証**: 残す GPU 合成パス（WUC）の描画・再描画が撤去前と等価であること、ビルド/起動、**clickthrough 登録窓が `apply_layered_companion()` 経由で `WS_EX_LAYERED` を受け取り透過が機能し続けること**を確認。

**既存コードに触れる前に、削除対象ファイルと変更内容を依頼者へ提示して確認を取る**（推測で消さない）。

## Scope

- **In**: ULW 専用コード（`compositor.rs`/`compositor_systems`/`com/ulw.rs`）の撤去。`CompositionMode` の ULW 分岐除去・collapse。`compute_ex_style` の ULW 分岐除去・デフォルトモードの GPU 合成化。ULW system 群の ECS スケジュール登録解除。「ULW 一択」記述（`tech.md`/`roadmap.md`/`COMPAT_ARCHITECTURE.md`）の更新。残存パスの描画非破壊・ビルド互換検証。
- **Out**: DComp→WUC 差し替え（別 spec `wintf-dcomp-to-wuc-migration`）。クリックスルー機構の実装（`wintf-clickthrough-alpha-toggle`）。当たり判定・ウィンドウ管理・スレッド構成の変更。新機能追加。

## Boundary Candidates

- ULW 専用描画経路の撤去（`WindowD3D11Compositor`＋`compositor_systems`＋GDI present）
- `CompositionMode` の単一化（enum 撤去 or 最小化）＋生成時デフォルト
- `compute_ex_style` の分岐一本化（`WS_EX_LAYERED` 撤去）
- ドキュメント正本（「ULW 一択」記述）の整合更新

## Out of Boundary

- DComp→WUC 差し替え（`wintf-dcomp-to-wuc-migration`）。
- クリックスルーのα源実装（`wintf-clickthrough-alpha-toggle`）。
- M1 emo2-boot の各エンジントラック。

## Upstream / Downstream

- **Upstream**: `wintf-clickthrough-alpha-toggle`（**完了必須**・並走検証を経て「ULW 破棄可」判断）。既存 ULW/DComp 切替基盤。
- **Downstream**: 表示レイヤーの単純化（切替分岐の消滅）。`wintf-dcomp-to-wuc-migration`（独立だが、両完了後に `CompositionMode` は WUC 単独へ最終 collapse）。

## Existing Spec Touchpoints

- **Extends/Replaces**: 既存 ULW/DComp 切替基盤（`wintf-dcomp-to-layered-migration` 系 completed が築いた ULW アーム）を撤去。
- **Adjacent**: `wintf-clickthrough-alpha-toggle`（前提依存・ULW 破棄の実質条件）／`wintf-dcomp-to-wuc-migration`（別ファイル群・独立）。

## Constraints

- Rust 2024・`windows` 0.62.2 系・**tokio 禁止**。（32bit 可搬性制約は host-32 系専用＝wintf 本体は x64＋arm64・i686 検証を課さない）
- **残す GPU 合成パスの描画等価**が受け入れ基準（撤去前後で見た目・再描画が変わらない）。
- **前提依存を破らない**: 本坑クリックスルー完了前に ULW を撤去しない（撤去すると並走安全網が消える）。
- 既存リリース最適化（`opt-level='z'`, `lto=true`）と互換。
- 既存本体コードは推測で消さない（削除前に対象と内容を依頼者へ提示）。
- 設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本として更新。
