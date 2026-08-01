# Brief: areka-P0-balloon-offset-dpi

> **種別**: 既知ギャップの正規登記（**宙に浮いていた申し送りの救済**）。⓪ghost（placement）帰属。
> **源**: `kero-balloon` tasks.md の申し送り「DPI 変化時に `BalloonFollow.offset` を k 倍する処理はどこにも実装が無い……要否判断は後続（W5 `dpi-window-vanish` **ほか**窓位置に触る spec）へ」——2026-08-01 棚卸で**規律違反**と判定: 「ほか」は複数条件付きのウェーブ型申し送りであり担当 spec ではない。しかも `dpi-window-vanish` の brief は DPI 追従の全面実装を明示的に **Out** に置き、同 spec 自体が「再現せず・掃除のみ」へ縮退し得る＝**申し送りが黙って死ぬ構造**だった。本起票で担当を確定する。
> **着手ゲート**: `dpi-window-vanish` 着地後（同 spec が `follow.rs` を触る）＋ `dpi-transition-atomicity`（起票済・配置未確定）の再観測結果と突合してから。**着手ゲートが同一なので合流セッションで transition-atomicity との統合も検討対象**。
> **📌 2026-08-01 追記(58)補正（棚卸⑤）**: 着手ゲート半充足——van 着地済み（PR#98）・残りは atom の第1段再観測のみ（**再観測は今すぐ実施可能になった**）。**roadmap 追記(58) の既定路線: atom が「+36px＋檻」へ縮退した場合は本 spec と統合して 1 spec 化**（follow.rs 共有・W6.75 配置）。atom が縮退しない場合は atom 先着→本 spec が rebase。アンカードリフト: U4「再スケールなし」follow.rs:262 → **:265**（:686/:762 にも同契約明文・van +4049 行でも U4 契約は不変）・windowposition.rs 単位混在 doc :91-97 → **:93-94**。exact（丸め権威）先行必須は不変。

## Problem

2 つの単位・スケール意味論の欠落が balloon offset 周りに残っている:

**A. DPI 変化時の `BalloonFollow.offset` 非スケール**: offset は物理 px で保持され（`follow.rs:262`「再スケールなし・U4」）、モニタ間 DPI 遷移（k 変化）で数値がそのまま持ち越される。窓寸は k 倍で変わるのに offset だけ旧 k の物理距離のまま⇒**高 DPI へ移るとバルーンが相対的に近く（低 DPI へは遠く）見える**。kero-balloon の step 6 撤去で「窓相対不変」は確立したが、その不変量は**同一 k 内**でのみ SSP 実測済み。k 跨ぎの正しい挙動（offset も k 倍か・SSP は何をするか）は**未測定**。

**B. `ScopeConfig.balloon_offset` 合流欄の単位空間混在**: `windowposition.rs:91-97` の doc が明記するとおり**意図的な暫定**——windowposition 由来の調整量は k 適用済み物理 px で合流するが、既存供給元 descript の `balloon.offsetx/offsety` は**非スケール生値**のまま同じ欄へ加算される。emo2 は descript offset 未宣言（None）ゆえ潜伏。宣言するゴーストが来た瞬間に k≠1 で顕在化する。

## Current State

- A: 実装ゼロ（`follow.rs` U4 が「再スケールなし」と明文で確定・kero-balloon tasks.md 申し送りに記録）。方向としては step 6 撤去で SSP に近づいており、**k 倍が本当に正しいかも自明ではない**（SSP は DPI 追従自体が別思想＝k=1 固定に近い。「SSP に k 跨ぎのオラクルが存在しない」可能性があり、その場合は areka 自身の設計原則〔DPI 追従が基本設計・全表示経路がスケール〕から導出して COMPAT に「areka 裁量」として記録する）。
- B: `design.md`（kero-balloon）Service Interface と `windowposition.rs` doc に「意図的・Out of scope（W5 対象外）」と登記済み——ただし担当 spec 無し。

## Desired Outcome

- `balloon_offset`／`BalloonFollow.offset` の**単位空間契約を一本化**（すべて「現在の k における物理 px」か、作者空間生値＋適用点スケールか——どちらかに統一し、設計判断を COMPAT へ）。
- DPI 遷移時の offset 変換規則を確定・実装（SSP オラクルが存在するなら実測、存在しないなら areka 設計原則からの導出を「areka 裁量」として COMPAT 記録）。
- 檻: k 遷移 × アンカー × 保存/復元の行列（persist の生値保存〔kero-balloon R3.8〕との整合を含む——保存値はどの k の物理 px か、復元時に k が違ったらどうするかを必ず檻に入れる）。

## Approach

1. `dpi-window-vanish` 5.1/5.2 着地後・`dpi-transition-atomicity` の再観測と同じ実機セッションで、**SSP のモニタ跨ぎバルーン挙動を観測**（SSP が DPI で何もしないならそれ自体が観測結果）。
2. 単位空間契約を設計で確定（有力: 「保存は作者空間（96dpi 相当）生値・適用点で現在 k を乗算」＝descript offset との合流も同空間になり B が同時に解ける）。
3. `follow.rs`（DPI 遷移フック）＋`persist.rs`（保存空間の明文化）＋`windowposition.rs`（合流欄の単位統一）を是正。**`scale-exact-rational`（W6.5）の `ScaleRatio` 配管と丸め権威を前提にする**（f32 を持ち込まない）。

## Scope

- **In**: offset の単位空間契約・DPI 遷移時変換・descript `balloon.offsetx/offsety` の k 適用統一・檻・COMPAT 記録。
- **Out**: バルーン追従の基準（窓相対＝kero-balloon R3.8 確定・不変）／キャラ窓の DPI 遷移（`dpi-window-vanish`・`dpi-transition-atomicity` の領分）／windowposition 語彙（`windowposition-limit`）。

## Boundary Candidates

- 単位変換の純関数（k₁→k₂ 遷移・決定論檻全網羅）。
- persist 往復 property（保存 k ≠ 復元 k の行列）。

## Out of Boundary

- `resolver.rs`（初期配置式は `scope-chain-gap` の領分）。
- キャラ窓原点の下端中央符号化（不変）。

## Upstream / Downstream

- **Upstream**: `dpi-window-vanish`（follow.rs の編集面・着地待ち）／`dpi-transition-atomicity`（起票済・配置未確定——**同一ゲート・統合候補**）／`scale-exact-rational`（丸め権威の配管）／`kero-balloon`（R3.8 の窓相対契約と persist 生値保存が前提）。
- **Downstream**: `emo2-conformance-e2e` 適合 #1（DPI 検証は追従込み）。

## Existing Spec Touchpoints

- **Extends**: なし（新規境界——ただし合流セッションで `dpi-transition-atomicity` と 1 spec に束ねる選択肢を裁定すること）。
- **Adjacent**: `dpi-window-vanish`（follow.rs 共有＝直列必須）。

## Constraints

- 画素演算 f32 禁止（`ScaleRatio` 有理数・W6.5 exact の着地形に従う）。
- 実機検証は実 DPI 混在環境（125%/200% デュアル等・emo-dpi-scaling 6.5 の手順が donor）。
- 配置は合流セッション裁定。候補: W6.5 以降・van/transition-atomicity の観測結果待ち。
