# Brief: areka-P0-scale-exact-rational

> **Discovery 日**: 2026-07-30 ／ **ウェーブ**: W6 の後・W7（emo2-conformance-e2e）の前 ／ **規模**: medium
> **出自**: `completed/areka-P0-emo-dpi-scaling` の残件是正（PR #92・roadmap 追記㊿）の最中に**新規発見**した欠陥。
> 担当 spec が実在しないため [[deferral-requires-verified-owner]] の規律に従い起票する。

> **📌 2026-08-06 追記(60)陳腐化補正（棚卸⑥・col=collision-dpi-hittest PR#100 マージ後の実測・本ブロックが本文より優先）**:
> - **col が本 spec の前提を前進させた（重複設計に注意）**: `ScaleRatio::unscale_coord`（scale.rs **:253**・逆写像・i128 中間・Euclid 除算・f32 非経由・決定論檻 6 本付き）＋ presenter に `applied_ratio() -> Option<ScaleRatio>`（**:744**・有理厳密）＋`ClientHit`/`hit_region_client`（**:248**/**:953**）が新設。「f32 を消す」ではなく **「f32 出口ビュー ∥ 有理権威」の二層併存**（doc :723-731 が判定経路まで明文化）へ前提が移動——本文 Approach の `TextSlotView.scale_ratio` 案は**整合したまま**だが、設計時は `applied_ratio` を配管の起点に使うこと（`ratio()` アクセサ新設と重ねない）。
> - アンカードリフト（+3〜+17・形は全て不変）: 汚染点 `scale: applied.as_f32()` :665 → **:682**（供給域 :659-666 → :676-683）・`as_f32` :144/doc :140 → **:147**/**:142**・`scale_len` :166 → **:169**・emo-text 本番呼出 actor.rs :649-650 → **:665-666**。`TextSlotView::scale` :233 は一致。emo-text への `ScaleRatio` 配管は**依然ゼロ**（region.rs :119 `ceil(v × k_f32)` 健在）＝gap 有効。
> - 干渉更新: cage との scale.rs `mod tests` 共有は不変。budget との presenter.rs 異ハンク判定は col 後も成立（budget=:386-417 ∥ 本 spec=:676-683）。

## Problem

DPI 追従（k = 窓の実モニタ DPI ÷ `author_dpi`）の**丸め権威は整数有理数 `ScaleRatio`** であり、画素・寸法演算で f32 を使うことは D4 で禁じられている。ところが**バルーン文字層だけは k が f32 へ落ちてから寸法演算に使われており**、非二進比で 1px 誤る。

`ScaleRatio::as_f32`（`crates/areka-emo-compose/src/scale.rs:144`）には「**寸法・画素演算にこの値を使ってはならない**」と明記されている（同 :140）。にもかかわらず `presenter.rs:665` が `scale: applied.as_f32()` として `TextSlotView` へ f32 を載せ、それが `TextSlotBinding.scale` → `ScaleContract.scale` と伝わり、`ScaleContract::physical_extent`（`crates/areka-emo-text/src/region.rs`）が `ceil(v × k_f32)` を計算している。**契約違反が配管で成立してしまっている。**

### 実測（2026-07-30・1..1200 の全 v を有理数の厳密 `div_ceil` と突合）

| k | f32 実値 | 誤り件数 / 1200 |
|---|---|---|
| **6/5**（作者 120・窓 144＝150%） | 1.2000000477 | **81**（例: v=25 → 31・正 30） |
| 4/3（作者 144・窓 192） | 1.3333333731 | 0 |
| 8/5・4/5・2/3 | — | 0 |

f32 の積が真値をわずかに超えると `ceil` が +1 に振れる。**k=6/5 は本番到達可能**——ukadoc は `dpi` の推奨値として 120 を挙げており、120 DPI 基準で描かれたゴーストを 150% 表示のモニタで出せばそのまま k=6/5 になる。

## Current State

- **丸め権威は整数のまま健在**: `ScaleRatio::scale_len`（`scale.rs:166`）は `(2·len·num + den) / (2·den)` の整数演算（round half away from zero・u128 中間・最小 1px）。窓寸・サーフェス寸はこちらを通っており汚染されていない。
- **汚染しているのは 1 経路だけ**: `physical_extent` の `ceil(v × k_f32)`。本番の呼び手は `crates/areka-emo-text/src/actor.rs:649-650`（`present_actor` が validrect 寸から文字供給面寸を出す）の 2 箇所のみ。
- **有理数が emo-text へ届かない**: `ScaleRatio` は私有フィールドで **`ratio()` アクセサ自体が存在しない**。`TextSlotView` も `scale() -> f32` しか公開していない（`presenter.rs:233`）。
- 2026-07-30 時点で `region.rs` の `physical_extent` doc に**実測表ごと登記済み**（是正されるまで消さないこと）。

## Desired Outcome

- `physical_extent` が**厳密な有理数演算**になり、`k=6/5` を含むあらゆる比で `ceil(v × num / den)` の真値と一致する。
- D4（単一丸め権威・画素演算で f32 禁止）が**文字層まで貫通**し、`as_f32` の「寸法演算に使うな」という宣言が配管上も守られる。
- k が f32 厳密表現可能な比（1・5/4・3/2・2 等＝既存テストの全て）では**バイト同一**。

## Approach

**有理数を配管する**（`ScaleRatio` の num/den を emo-compose → emo-present → emo-text へ通す）。

1. `areka-emo-compose`: `ScaleRatio` に `pub fn ratio(self) -> (u32, u32)`（読み取り専用の厳密エクスポート。**丸め権威は `scale_len` のまま**で、本アクセサは権威を増やさない）。
2. `areka-emo-present`: `TextSlotView` に `scale_ratio` を追加し `presenter.rs:659-666` で `applied.ratio()` から供給。`scale: f32` は D2D `SetTransform` 等の連続量消費者向けに残置。
3. `areka-emo-text`: `ScaleContract` が num/den を保持し、`physical_extent` を `((v as f64) × num / den).ceil()` で計算（整数 v・整商に対し f64 誤差 ~1e-13 ≪ 最小非 tie 距離 1/den ゆえ厳密）。`to_physical`/`to_image` は連続量（サブピクセルオフセット）ゆえ f32 のまま。

**代替案と却下理由**:
- *f64 で計算するだけ*: **却下**。入力の k が既に不正確な f32 なので精度を上げても真値へ戻らない（`0.8f32 = 0.800000011920929`）。
- *f32 から連分数で有理数を復元*: 却下。分母が有界（DPI 比）なので理論上は可能だが、**正典よりも巧妙**（[[canonical-not-minimal-lifecycle]] の趣旨に反する）で、失敗時の挙動が読みにくい。

## Scope

- **In**: 上記 3 段の配管と `physical_extent` の厳密化。`TextSlotBinding::new` / `ScaleContract::new` の署名追随（呼び手 41 + 67 箇所——大半は機械的なテスト側）。厳密性の檻（tie ケースの排他キル）。`region.rs` の登記コメントの解消。
- **Out**: `to_physical`/`to_image` の f32 維持（連続量ゆえ丸め崖が無い）。`placement` 側のログ用 `as_f32`（クエリ契約の出口＝正当）。窓寸・サーフェス寸の経路（既に `scaled_extent` で厳密）。合成・リサンプルの算術（`resample` は `ScaleRatio` を直接受け取り済み）。

## Boundary Candidates

- **有理数の公開面**（`ScaleRatio::ratio` — emo-compose 単独）
- **搬送面**（`TextSlotView.scale_ratio` — emo-present 単独・additive）
- **消費面**（`ScaleContract` の num/den 化と `physical_extent` — emo-text 単独）
- **署名追随**（`TextSlotBinding::new` / `ScaleContract::new` の呼び手更新 — emo-text 内に閉じる機械作業）

## Out of Boundary

- `ScaleContract::new(scale: f32, …)` を**二重コンストラクタとして残すこと**は禁じ手。真値の出所が 2 つに割れると「どちらが権威か」が曖昧になり、本 spec が直そうとしている病そのものを再生産する。移行するなら全呼び手を有理数へ寄せきる。
- 丸め権威を `physical_extent` 側へ増やすこと（権威は `ScaleRatio` 単独＝D4）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-emo-dpi-scaling`（`ScaleRatio`・`TextSlotView`・`ScaleContract` の実体と D2/D4 の裁定）。`completed/areka-P0-emo-text-layer`（2 空間モデル）。
- **Downstream**: `areka-P0-emo2-conformance-e2e`（W7・適合 #1 の DPI 検証）。厳密化後は非二進 k でも供給面寸が理論値と一致するため、e2e の実機判定が絶対値で書ける。

## Existing Spec Touchpoints

- **Extends**: なし（completed 済 spec の残件ゆえ新規境界）。
- **Adjacent**:
  - `areka-P0-kero-balloon`（W5）— `placement/measure.rs` を所有するが、本 spec は measure.rs に触れない（**互いに素**）。
  - `areka-P0-balloon-visibility`（W6）— emo-present を消費するが `presenter.rs` の `TextSlotView` 定義域は触らない見込み。着手時に実測で再突合すること（[[parallel-worktree-brief-staleness-rebase-before-design]]）。
  - `areka-P0-test-cage-determinism`（同時起票）— `areka-emo-present/src/scale.rs` の `mod tests` を触るため**同一ファイル異ハンク**。着手順の裁定が要る。

## Constraints

- **新規外部依存なし**（有理数演算は整数 + f64 で足りる）。
- k が f32 厳密表現可能な比では**バイト同一**であること（既存テストが 1 本も色を変えないのが受け入れの下限）。
- Windows 専用・Rust 2024・[[deterministic-test-coverage-mandate]]（tie ケースは全て実行テストで檻化し、変異の排他キルを計測日付きで記録）。
- [[test-only-decision-branches-not-proven-wiring]]: 41 + 67 の署名追随は配線であって判断分岐ではない。檻に入れるのは `physical_extent` の算術のみでよい。
