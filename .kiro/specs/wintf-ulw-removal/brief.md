# Brief: wintf-ulw-removal（本坑 / main）

> **種別**: 本坑（main）。通常の kiro ライフサイクル（requirements → design → tasks → impl → complete）。PR ベース squash マージで `main` へ統合。
> **位置づけ**: M1（emo2-boot）とは別軸の **wintf 基盤層**。表示レイヤーの合成方式を GPU 合成（DComp、後に WUC）単独へ一本化するための **ULW 撤去**。
> **前提依存（順序ゲート）**:
> ```
> _Depends: wintf-clickthrough-alpha-toggle（完了・並走検証期間を経てULW破棄可の判断）
> ```
> **ULW を安全に消せるのは、本坑クリックスルー（`WS_EX_TRANSPARENT` 動的トグル）が完了し「ULW 無しでも別プロセスクリック透過が成立」と確認できてから**。クリックスルー brief の並走方針（「完全有効と判断されれば ULW 破棄／ただし当面は並走・即時撤去しない」）に一致。`wintf-dcomp-to-wuc-migration` とは**触るファイルが別ゆえ独立**（順序任意）。

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

- **DComp パス（本 spec で残す・触らない）**: `com/dcomp.rs`, `ecs/graphics/dcomp_resource.rs`, graphics `components.rs`, `systems/{init,surface,visual_sync,render}.rs`。
- **クリックスルーのα源に関する注意**: クリックスルー brief は接続先候補に ULW compositor の D2D1 staging αバッファ（`compositor.rs`・`CPU_READ`）を挙げるが、GPU 合成パスのクリックスルーは **per-widget αマスク**（`ecs/widget/bitmap_source/alpha_mask.rs` の `AlphaMask::is_hit`/`from_pbgra32`）を α源に使う想定。`compositor.rs` 撤去がクリックスルーのα源を奪わないことを **design で確認**する（前提依存の実質条件）。

## Desired Outcome

ULW 一式が撤去され、表示バックエンドが **GPU 合成単独**（DComp、`wintf-dcomp-to-wuc-migration` 後は WUC）へ collapse。`CompositionMode` は単一化（単一 variant なら enum 撤去、または最小化）。**残す GPU 合成パスの描画結果・挙動は不変**。ビルド通過・起動して同一描画結果。当たり判定・ウィンドウ管理・スレッド構成は不変。

## Approach

1. **ULW 参照の全数洗い出し**: `CompositionMode::ULW`・`WindowD3D11Compositor`・`compositor_systems`・`com/ulw.rs`・`compute_ex_style` の ULW 分岐・ULW を前提にした初期化/スケジュール登録を grep で漏れなく特定。
2. **専用コード削除**: `ecs/graphics/compositor.rs`・`compositor_systems/`・`com/ulw.rs` を撤去。ECS スケジュールから ULW system 群を除去。
3. **`CompositionMode` collapse**: ULW variant 除去。GPU 合成単独になった時点で enum を撤去 or 単一値へ最小化（生成時のデフォルトを GPU 合成へ）。`compute_ex_style` の ULW 分岐（`WS_EX_LAYERED`）を除去し、GPU 合成の ex_style（`WS_EX_NOREDIRECTIONBITMAP`）へ一本化。
4. **ドキュメント正本更新**: `tech.md` line 83 ／ `roadmap.md` line 30 の「別プロセス透過は実質 ULW 一択」記述を、クリックスルー方式確定に合わせて更新。設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本に反映。
5. **非破壊検証**: 残す GPU 合成パスの描画・再描画が撤去前と等価であること、ビルド/起動を確認。

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

- Rust 2024・`windows` 0.62.2 系・**tokio 禁止**。32bit 可搬性を崩さない。
- **残す GPU 合成パスの描画等価**が受け入れ基準（撤去前後で見た目・再描画が変わらない）。
- **前提依存を破らない**: 本坑クリックスルー完了前に ULW を撤去しない（撤去すると並走安全網が消える）。
- 既存リリース最適化（`opt-level='z'`, `lto=true`）と互換。
- 既存本体コードは推測で消さない（削除前に対象と内容を依頼者へ提示）。
- 設計判断の変更は `doc/COMPAT_ARCHITECTURE.md` を正本として更新。
