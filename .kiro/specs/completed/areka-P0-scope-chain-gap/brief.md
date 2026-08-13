# Brief: areka-P0-scope-chain-gap

> **種別**: 挙動バグ（実機観測済み・**本起票まで完全未登記だった**）。⓪ghost（placement）帰属。
> **源**: `kero-balloon` 実機サインオフ（2026-07-31）での開発者目視指摘「むらさきとエモの距離が多分DPI分余計に離さないとダメ」＋ 2026-08-01 未登記先送り棚卸（`/kiro-discovery` 再入・孤児 17 件中の最優先）。
> **着手ゲート**: `kero-balloon` の main マージ後（`placement/mod.rs` の fixture 実走檻が絶対位置を主張するため）。**要件フェーズの前に SSP 実測オラクル採取が必須**（是正式が測定に依存する）。

## Problem

areka の二体既定配置（scope0=むらさき・scope1=エモ）は、**キャラ幅が異なると幅差ぶんの隙間**が空く。実機（DPI 120・k=1.25）で scope0 543×859／scope1 420×500（`kero-balloon/real-run-signoff-2026-07-31.log:24`）⇒ **543 − 420 = 123px の隙間**。開発者が目視で「離れすぎ」と指摘した現象の正体。

**根因は式の座標基準の取り違え**。`crates/areka/src/placement/resolver.rs:155-159` の P2 連鎖:

```
base_x(n≥1) = char_x(n−1) − w(n−1)    // 前スコープの左端 − 前スコープの幅
```

`char_x` は**左端**なので、scope n の左端をここへ置くと scope n は `[char_x(n−1)−w(n−1), char_x(n−1)−w(n−1)+w(n)]` を占め、前スコープとの間に `w(n−1) − w(n)` の隙間が残る。**隣接（密着）にしたいなら引くべきは自分の幅 `w(n)`**。同幅なら偶然一致するため、等幅テストでは見えない。

## Current State

3 つの記録欠陥が重なって「正しいもの」として固着している:

1. **要件が literal にこの式を規定**——`completed/areka-P0-window-placement` R2.9（requirements.md:41）「scope1（相方）を **scope0 のサーフェス画像幅ぶん**左へずらした位置へ置く（SSP de-facto）」。**実装は要件に忠実**であり、疑わしいのは要件そのもの。completed spec は消化不能＝上書き記録の先例は `position-persist` R2.2/R8.5（`kero-balloon` R3.8 裁定・COMPAT §8）。
2. **「SSP de-facto」の札が無検証**——同 spec research.md:78 は scope 相対配置を「**Unknown**: SSP de-facto」と記載したまま、:122 で「**要件討議#2 で確定**」。つまり**開発者討議で決めた値に SSP de-facto の札を貼っただけ**で、SSP 実挙動との突合は一度も行われていない。`kero-balloon` で SSP が areka を覆した 2 件（`windowposition.x` 符号・バルーン追従基準）と**同型の 3 例目**。
3. **檻が名前で嘘をつく**——`resolver.rs:453 t_r2_scope_chain_defaultx_zero_stays_adjacent` は不等幅（400/320/200）を入力しながら `assert_eq!(out[1].char_pos.x, x0 − w0, "…密着（2.9）")` を主張。実際の幾何は **80px の隙間**であり、名前（stays_adjacent）とメッセージ（密着）が檻自身の固定内容と矛盾。読んだ者は「密着が檻に入っている」と誤信する。

さらに: `kero-balloon` の最終検証ゲート（2026-08-01）は `resolver.rs` を「main と blob 同一＝無改変」と**合格判定**した。無改変の証明は正しさの証明ではないのに、清潔証明書として機能してしまった実例。

## Desired Outcome

- SSP 実測で二体の既定相対配置規則を確定し、P2 を是正（仮説は下記）。
- 檻を真実の名前へ（例: 隣接なら `…_stays_adjacent` が本当に gap=0 を主張する形へ）。不等幅入力は維持（等幅では欠陥が見えない）。
- `doc/COMPAT_ARCHITECTURE.md` §8 へ R2.9 上書きの記録（position-persist R2.2/R8.5 上書きと同じ体裁: 否定した先行 AC を名指し・アーカイブ spec は非改変・オラクルと測定値を記載）。

## Approach

**Step 0（要件フェーズ前・必須）: SSP オラクル採取。** 同一ゴースト emo2・profile 削除・初回起動の SSP で両キャラ窓矩形を実測（`kero-balloon` 6.1 と同手順: DPI aware プロセスから read-only ポーリング。**6.1 当時の SSP 実測はバルーン offset のみ記録済み・キャラ窓の生矩形は未保存**＝再採取が要る）。判別すべき仮説:

- **H1 隣接**: `base_x(n) = char_x(n−1) − w(n)`（gap=0・最有力＝R2.9 の意図「二体は左右に並び重ならない」の自然な読み）
- **H2 固定マージン**: 隣接＋定数 px（SSP に既定間隔がある可能性）
- **H3 defaultx 由来**: SSP は連鎖せず各 scope の `sakura.defaultx`/`kero.defaultx` を独立に画面基準で解釈（この場合 P2 連鎖という構造自体が SSP と別物）

いずれの仮説でも「幅差ぶんの隙間」は説明されない（emo2 は defaultx 未宣言 or 0）ため、**現行式が SSP と一致する仮説は無い**見込み——ただし kero-balloon の教訓どおり、**測ってから式を書く**こと（単発観測で言える不変量の範囲も記録する）。

**Step 1**: P2 是正＋T-R2 系檻の意味・名称是正（`:453`/`:495` T-R2 補・`:847` T-R4 補の連鎖依存も追随）。
**Step 2**: 絶対位置を主張する fixture 檻の期待値更新——`placement/mod.rs` の `prepare_emo2_returns_two_scope_placements` ほか（kero-balloon が per-scope 期待値化済みの箇所）。**`prepare_emo2_matches_ssp_balloon_offsets_at_dpi_120` は無傷のはず**（バルーン offset は自 char 窓相対＝char 位置が動いても不変）——崩れたらそれ自体が欠陥のシグナル。
**Step 3**: COMPAT §8 記録＋`window-placement` R2.9 上書き登記。

## Scope

- **In**: `resolver.rs` P2 式・その檻群・`placement/mod.rs`/`measure.rs` の位置期待値追随・COMPAT §8・R2.9 上書き記録。
- **Out**: `default_x` の意味論（R2.10・右端からの左方向オフセット＝別途 SSP 確認済みの討議確定でありここでは触らない）／`windowposition.limit`（別 spec `windowposition-limit`）／重なり回避等の複雑ロジック（canon 沈黙・R2.9 の「単純な基準配置のみ」は維持）／バルーン offset 基準（kero-balloon R3.8 で確定済み・不変）。

## Boundary Candidates

- P2 純関数（resolver）＝決定論檻で全網羅可能（不等幅×DPI 行列）。
- emo2 実寸系 fixture 檻（543/420 の実測値で SSP オラクル値を固定）。

## Out of Boundary

- `spawn.rs`・`follow.rs`・`persist.rs`（位置の**初期解決**のみが対象・追従/保存は触らない）。
- キャラ窓原点（下端中央）の符号化。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-window-placement`（R2.9 正本・上書き記録の対象）／`kero-balloon`（pmod 檻の per-scope 期待値・SSP 突合檻＝**マージ後に rebase 必須**）。
- **Downstream**: `emo2-conformance-e2e` 適合 #10（kero 一式の検証はこの間隔で目視される）／`balloon-visibility`（バルーン位置は char 位置に従属）。

## Existing Spec Touchpoints

- **Extends**: なし（新規境界）。
- **Adjacent**: `dpi-window-vanish`（placement/mod.rs・follow.rs を触るが **resolver.rs は非所有**〔brief は DPI 不変性テストの置き場としてのみ言及〕——van 完走後の着手なら干渉なし。設計前に origin/main へ再実測突合のこと）。

## Constraints

- Rust 2024。丸めは `ScaleRatio` 権威のみ（新丸め規約の導入禁止＝kero-balloon R3.6 と同じ制約を引き継ぐ）。
- 配置（ウェーブ）は**合流セッションで裁定**（単一 spec の椅子から決めない）。候補: W6.5 以降（W5 完走後・kero-balloon の檻着地が前提）。
- 実機検証は本番ゴースト＋実 DPI≠96（areka-placement-real-ghost-first）。
