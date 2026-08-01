# Brief: areka-P0-windowposition-limit

> **種別**: 挙動バグ（画面外はみ出しは実機観測済み）＋正典語彙の追跡。⓪ghost（placement）帰属。
> **源**: `kero-balloon` R7.4 の COMPAT §8 記録（:145）が「`limit` 未実装のため……バルーンが画面外へはみ出し得る（**実機で観測・追跡対象**）」と書きながら**追跡者が存在しなかった**——2026-08-01 未登記先送り棚卸で孤児と判定・本起票で登記。
> **着手ゲート**: `kero-balloon` の main マージ後（`placement/windowposition.rs` は同 spec の新設ファイル）。M1 内で着手可能な小型 spec（`limit` は正典既定 1＝**全ゴーストに適用される既定挙動**であり emo2 でも実害が出ている）。

## Problem

2 つの未実装が同じファイル（バルーン `descript.txt` の `windowposition` 族）に残っている:

1. **`windowposition.limit`（正典既定 1）**: バルーンを画面内へ制限する既定挙動。areka は未実装のため、ゴーストが画面端に寄っている状態でバルーンが work area 外へ素直にはみ出す（**実機で観測済み**・COMPAT §8 :145）。resolver P5 の「クランプなし（バルーンは work area 外へ素直にはみ出す）」（`resolver.rs:110-112` 付近の doc）は limit=0 相当の挙動＝**正典既定と逆**。
2. **`windowposition.x`/`y` のキーワード値（`center`/`top`/`bottom` 等）**: 数値のみ実装済み（kero-balloon R3.2）。キーワードは未実装で COMPAT §8 :145 に登記済み。emo2 は数値のみ使用のため潜伏。

## Current State

- `crates/areka/src/placement/windowposition.rs`（kero-balloon 新設）: 数値 `windowposition.x/y` → 画面座標調整量の変換＋`ScopeConfig.balloon_offset` への合流のみ。キーワードはパース対象外。
- `limit` に相当するクランプは全経路に存在しない（初期配置 P5・ドラッグ追従・windowposition 適用のいずれも）。
- キャラ窓には P4 クランプがある（`resolver.rs`・キャラ窓のみ・work area 内）——バルーンだけが無制限。

## Desired Outcome

- `windowposition.limit` の正典意味論（ukadoc 全文を MCP で取得して確定）＋ SSP 実挙動（**適用時点**: 初期配置のみか・追従/ドラッグ中もか・ユーザーが手で画面外へ置いた場合に強制的に戻すか）を確認のうえ実装。既定 1。
- キーワード値の語彙を windowposition.rs のパースへ追加（`center`/`top`/`bottom`——ukadoc の値域全量を要件フェーズで確定）。
- 檻: limit の全分岐（0/1 × はみ出し方向 4 辺 × k≠1）＋キーワード語彙。COMPAT §8 の当該行を「実装済み」へ更新。

## Approach

1. 要件フェーズ: ukadoc MCP で `windowposition` 項の全文取得（値域・既定・limit の定義）。SSP 実機で limit 挙動の適用時点を観測（emo2 をわざと画面端へドラッグ→バルーン位置を実測）。
2. 実装: windowposition.rs にキーワード→数値の解決を追加（変換は既存の純関数流儀）。limit クランプは適用時点の観測結果に従い、初期配置なら resolver P5 直後のシーム or `prepare_stages` の合流点、継続適用なら follow 側——**観測してから場所を決める**（kero-balloon の教訓: 場所の直観は SSP 突合なしで信じない）。
3. 檻＋COMPAT 更新。

## Scope

- **In**: `windowposition.limit`・キーワード値の語彙・その檻・COMPAT 更新。
- **Out**: `windowposition.x` の符号規約（kero-balloon R7.6 で SSP 実測確定済み・不変）／P2 キャラ間隔（別 spec `scope-chain-gap`）／バルーンドラッグの自由度そのもの（ユーザー操作を制限するかは SSP 観測の結果に従う——観測前に決めない）。

## Boundary Candidates

- キーワード解決の純関数（決定論檻で全網羅）。
- limit クランプの適用点（観測結果次第で placement か follow のどちらか一方に閉じる）。

## Out of Boundary

- `resolver.rs` P1〜P4（キャラ窓の配置・クランプは既存のまま）。
- バルーン offset の基準・保存（kero-balloon R3.8 確定・不変）。

## Upstream / Downstream

- **Upstream**: `kero-balloon`（windowposition.rs の実形・COMPAT §8 の登記行）。
- **Downstream**: `emo2-conformance-e2e`（画面端シナリオが適合走行で目視される可能性）／`balloon-visibility`（表示ライフサイクルと limit の相互作用は要確認＝hide 中の位置補正は無意味）。

## Existing Spec Touchpoints

- **Extends**: なし（新規境界。COMPAT の追跡行を消化する）。
- **Adjacent**: `scope-chain-gap`（同じ placement 層・**着手順の裁定が要る**＝両方が `placement/mod.rs` の fixture 檻期待値を触り得る。合流セッションで直列順を決める）。

## Constraints

- 丸めは `ScaleRatio` 権威のみ（新丸め規約導入禁止）。
- ukadoc と SSP が食い違う場合は SSP 実挙動を正とし COMPAT へ乖離を記録（互換ベースウェア戦略・kero-balloon R7.6 の先例）。
- 配置（ウェーブ）は合流セッションで裁定。候補: W6.5 以降・`scope-chain-gap` と直列。
