# Brief: areka-P0-collision-dpi-hittest

> **種別**: 本坑（main）新規。⑥ emo 帰属（**`collision-geometry` の DPI追従後続＝当たり判定の point÷k**）。2026-07-18 `collision-geometry` Task 4.2 受け入れ却下から派生。
> **調査日**: 2026-07-18（同 DPI追従スコープ調査ワークフロー）。一次分析は `specs/completed/areka-P0-collision-geometry/research.md §13`。
> **上流（必須・ブロッキング）**: **`areka-P0-emo-dpi-scaling`**（`scale()` が k を返し、マスコットが実際に k× 拡大表示される——**これ無しでは実機に scale≠1.0 の状態が存在せず ÷k を no-op としてしか観測できない**）／`completed/areka-P0-collision-geometry`（純関数 `hit_region`・resolver・presenter 読み口・probe——本 spec が拡張する土台・Task 1-4.1 は不変）。
> **下流**: `input-events`（撫で配信・k 補正後の region を消費）／`emo2-conformance-e2e`。
> **collision-geometry との関係**: 本 spec は collision-geometry の **k=1.0 限定契約（`design.md:50`・Revalidation Trigger 2 `:86`）を解除**し、point÷k を第一級実装する。
> **M1/M2・合流**: **未決＝別セッションの計画判断**（[[portfolio-convergence-decided-in-separate-session]]・下記）。

> **📌 2026-07-31 追記(52)陳腐化補正（W4 完走・本ブロックが㊵以下より優先）**:
> - **上流 `emo-dpi-scaling` ✅完了（2026-07-29・W4）＝ブロッキング解消**。しかも W4 が本 spec の席を**名指しで予約済み**: `applied_scale` アクセサ（presenter.rs:706）新設・**presenter.rs:232／:704 に「`collision-dpi-hittest` の点÷k はこの値を参照してよい」の明文**・:4361 に「マスクの座標契約（点÷k）は W5 の領分」登記。`EmoPresenter::hit_region` は **:867** へドリフト（÷k 未実装のまま・doc :859-861 が「k≠1.0 では呼び手が ÷k してから渡す——変換は本メソッド責務外」と明文化）。
> - ㊵の挿入点2箇所は現存: `emo2_boot/hit_region.rs:69`（`resolve_hit_region`・「shell 窓専用」doc :32）・input_events/mod.rs の DD-IE-10 素通し規約 :96 → **:97/:104-105**。「input_events/mod.rs は W4 choice-interact が増設する共有面」は**充足済みの過去形**（chI 完了・バルーン系は別ファイル `input_events/balloon.rs` に着地＝本 spec と非衝突）。
> - **W5 内最小 spec**: ÷k 挿入点・k 供給源・丸め論点まで上流が掘り済み＝決定論 unit＋probe 拡張＋実 DPI 2 水準の実機受け入れが本体。Strategy A/B・author_dpi 等の open questions は W4 design で決着済み（completed/areka-P0-emo-dpi-scaling/design.md 参照）——本 spec 残は ÷k の丸め方針のみ。判定は絶対 px でなく比（追記㊾）。

> **📌 2026-07-23 追記㊵陳腐化補正（本ブロックが以下の本文より優先）**:
> - **M1/M2 は決着済み＝M1 編入**（2026-07-19 追記㉟開発者裁定）・**ウェーブ配置=W5**（追記㊵攻め再編＝`dpi-window-vanish` ∥ `choice-select-events` と同居・emo-dpi-scaling は W4）。
> - **÷k の着地点の実体訂正（2026-07-23 実測）**: 本文の `presenter.rs:449-453` `EmoPresenter::hit_region` は読み口であり、hit 解決の実体は **`crates/areka/src/emo2_boot/hit_region.rs`**（`resolve_hit_region`・冒頭 doc L4-9 で「shell 窓専用・balloon は扱わない」を明文化）＋ **`crates/areka/src/input_events/mod.rs:96`**（「座標は素通し＝DPI 変換なし（DD-IE-10）」規約）。÷k の挿入点は design でこの2箇所を第一候補として確定すること。**input_events/mod.rs は W4 `choice-interact` がバルーンハンドラを増設する共有面**＝W5 配置（chI 完了後）の根拠。

## Problem

`collision-geometry` は当たり判定を **k=1.0 前提**で実装した（点を無変換で純関数へ照合）。これは DPI追従（基本設計・[[areka-dpi-following-core-design]]）下では不正——マスコットが k× 拡大表示されると、窓 client 物理 px は k 倍空間になるが collision 矩形は author/surface px のまま。**拡大後の点を k で縮約（÷k）してから照合しないと当たらない**。この ÷k は現状:
- 未実装（`EmoPresenter::hit_region` `presenter.rs:449-453` が点を無変換で `areka_emo_compose::hit_region` へ渡す・doc `:444-445` が「k=1.0 契約」を明記）。
- 未テスト（`collision-geometry` の実 DPI probe はモニタ DPI を変えても k=1.0 ゆえヒットテスト経路が同一だった）。

＝Task 4.2 受け入れ却下の核心。

## Desired Outcome

マスコットが scale≠1.0 で拡大表示された状態で、拡大後の窓 client 点を正しく ÷k 縮約して Head/Bust/None を解決する。

**✔ 観測（2段）**:
- **決定論 unit**: `EmoPresenter::hit_region` が点÷k で照合する分岐を、**fake k（例 2.0）を per-target scale へ注入**して網羅（client (100,100)・k=2.0 → surface px (50,50) 照合で Head/Bust/None＋境界 on/off・k=1.0 で no-op 保存）。純 `hit.rs:57-62` は**不変**（k は caller 境界で吸収）。GPU 不要・完全決定論（[[test-only-decision-branches-not-proven-wiring]]・[[deterministic-test-coverage-mandate]]）。
- **実機受け入れ**: 上流 `emo-dpi-scaling` でマスコットが実際に k× 表示される状態で collision-probe を拡張実行し、目視で頭/胸/背景を狙い解決一致（反トートロジー遵守＝狙点は目視のみ・`SetCursorPos`/`SendInput` 不使用）。**dpi≠96 の2水準で「マスコットが実際に異なる scale で表示された」証跡**（k=1.25 と k=2.0 で異なる拡大寸）を記録＝Task 4.2 が本来要求していた検証。

## Approach

1. **caller 境界の ÷k**: `presenter.rs:452` で純関数呼び出し前に `(x/scale, y/scale)`（scale = 上流が供給する per-target k）。純 `hit.rs` は**不変**（DPI を一切参照しない設計 `design.md:318` を維持・k は presenter 境界で吸収）。
2. **決定論 unit 檻**: fake k 注入で ÷k 分岐を網羅。÷k の丸め（floor/round）を確定（i64÷f32・**境界1px がヒットに効く**）。
3. **collision-geometry design 契約改訂**: k=1.0 限定契約（`design.md:50`）と Revalidation Trigger 2（`:86`）を「÷k を本 spec が実装済み」へ更新。
4. **probe 拡張＋実機受け入れ**: collision-probe を上流の k× 表示に対応させ、`acceptance-record.md` を「scale≠1.0 実機」で完成（Task 4.2 の正しい受け入れ）。

## 開発者が決める設計論点（design ディスカッションへ）

1. **÷k の丸め**: floor vs round（境界画素のヒット挙動）。**上流 `emo-dpi-scaling` のリサンプル丸めと整合**させること。
2. **境界の含端×k**: collision-geometry C2 の閉区間境界を k 空間へどう写すか（k 倍後の 1px 境界の内外）。
3. **seriko/mayuna collision との相互作用**: element 入れ子・mayuna 着せ替えの collision も同 k か（collision 集合が base surface id だけで決まる前提＝collision-geometry Revalidation Trigger 5 との合流）。

## Scope

- **In**: `EmoPresenter::hit_region` の point÷k／fake-k 決定論 unit／collision-geometry k=1.0 契約の改訂／collision-probe 拡張＋scale≠1.0 実機受け入れ。
- **Out**: レンダリング k× 拡大（**上流 `areka-P0-emo-dpi-scaling`**）／SHIORI 配信（`input-events`）／balloon choice hit（`choice-render`）／純 `hit.rs` の改変（k は caller 吸収ゆえ不変）。

## Upstream / Downstream

- **Upstream（必須・ブロッキング）**: `areka-P0-emo-dpi-scaling`（`scale()`→k＋実 k× 表示）／`completed/areka-P0-collision-geometry`（土台）。
- **Downstream**: `input-events`／`emo2-conformance-e2e`。

## Constraints

- Rust 2024・新規依存なし・純 `hit.rs` 不変（k は caller 吸収）。
- **実機受け入れは上流の k× 表示着地が前提**（単独では実機 7.3 を閉じられない＝合流ゲート・[[portfolio-convergence-decided-in-separate-session]]）。決定論 unit は上流と独立に先行 landing 可（k=1.0 で no-op 保存ゆえ）。
