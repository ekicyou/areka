# Brief: areka-P0-windowposition-limit

> **種別**: 挙動バグ（画面外はみ出しは実機観測済み）＋正典語彙の追跡。⓪ghost（placement）帰属。
> **源**: `kero-balloon` R7.4 の COMPAT §8 記録（:145）が「`limit` 未実装のため……バルーンが画面外へはみ出し得る（**実機で観測・追跡対象**）」と書きながら**追跡者が存在しなかった**——2026-08-01 未登記先送り棚卸で孤児と判定・本起票で登記。
> **着手ゲート**: `kero-balloon` の main マージ後（`placement/windowposition.rs` は同 spec の新設ファイル）。M1 内で着手可能な小型 spec（`limit` は正典既定 1＝**全ゴーストに適用される既定挙動**であり emo2 でも実害が出ている）。

> **📌 2026-08-13 追記(63)（`areka-P0-scope-chain-gap` からの申し送り・rebase 前提の更新）**: **scg 着地により「キャラ窓の位置を決めるのは `resolve_placement` 唯一」という前提が崩れた。** rebase の対象が resolver の檻だけではなくなっている。
> - **第 2 の位置ライター**: scg の要件 7 が `finalize_chain_once`（`crates/areka/src/emo2_boot/frame/drain_resnap.rs`・判定は `crates/areka/src/placement/chain_finalize.rs`）を新設した。実表示サーフェス寸が確定したフレームで**一度だけ**連鎖を解き直し、後続スコープのキャラ窓 X を `move_window_to` で直接書く。
> - **P4 クランプを経由しない**: この経路は `resolve_placement` を通らないため、**work area 内クランプ（P4）が掛からない**。現行の 2 体構成・実機実測では画面外へ出る事象は観測されていないが、「キャラ窓は必ず work area 内」という構造的保証は失われている。本 brief の Current State が言う「キャラ窓には P4 クランプがある——バルーンだけが無制限」は、**確定経路については成り立たなくなった**。
> - **roadmap 干渉台帳の記述が不完全**: `steering/roadmap.md` の `scg⇄wpl` 行は「`resolver.rs` `resolve_placement` 内 P2 ／ P5 が 30 行差」としか登記していない。実際の干渉面は resolver の外へ広がっている（`chain_finalize.rs`・`drain_resnap.rs`・`placement/spawn.rs` の `ScopeWindows.default_char_pos`）。**rebase 時はこの 3 ファイルも突合すること。**
> - **limit 設計で決めること**: `windowposition.limit` のクランプを「どの位置ライターに掛けるか」。resolver P5 だけに掛けると確定経路が素通しになり、limit=1 の保証がキャラ窓の確定後配置で破れる。**クランプを単一の関門へ集約するか、書き込み口ごとに掛けるかは本 spec の設計判断**とする。
> - scg 側の実装・証跡: `.kiro/specs/completed/areka-P0-scope-chain-gap/`（要件 7・design C6・`real-run-signoff-2026-08-13.log` §5.5）。resolver の P5 ハンクは scg で**差分 0 行**（非接触を維持済み）。

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
