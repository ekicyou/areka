# Brief: areka-P0-balloon-offset-dpi

> **種別**: 既知ギャップの正規登記（**宙に浮いていた申し送りの救済**）。⓪ghost（placement）帰属。
> **源**: `kero-balloon` tasks.md の申し送り「DPI 変化時に `BalloonFollow.offset` を k 倍する処理はどこにも実装が無い……要否判断は後続（W5 `dpi-window-vanish` **ほか**窓位置に触る spec）へ」——2026-08-01 棚卸で**規律違反**と判定: 「ほか」は複数条件付きのウェーブ型申し送りであり担当 spec ではない。しかも `dpi-window-vanish` の brief は DPI 追従の全面実装を明示的に **Out** に置き、同 spec 自体が「再現せず・掃除のみ」へ縮退し得る＝**申し送りが黙って死ぬ構造**だった。本起票で担当を確定する。
> **着手ゲート**: `dpi-window-vanish` 着地後（同 spec が `follow.rs` を触る）＋ `dpi-transition-atomicity`（起票済・配置未確定）の再観測結果と突合してから。**着手ゲートが同一なので合流セッションで transition-atomicity との統合も検討対象**。**【2026-08-21 失効・下記追記(70)⑹ が正本】**——`dpi-transition-atomicity` は W6.75 の単独 spec として確定・走行中であり、統合は行われない（atom 先着→本 spec が rebase）。
> **📌 2026-08-01 追記(58)補正（棚卸⑤）**: 着手ゲート半充足——van 着地済み（PR#98）・残りは atom の第1段再観測のみ（**再観測は今すぐ実施可能になった**）。**roadmap 追記(58) の既定路線: atom が「+36px＋檻」へ縮退した場合は本 spec と統合して 1 spec 化**（follow.rs 共有・W6.75 配置）。 **【2026-08-21 失効・下記追記(70)⑹ が正本】**——atom は縮退せず単独フル spec として走行中のため、統合分岐は条件不成立で失効。atom が縮退しない場合は atom 先着→本 spec が rebase。アンカードリフト: U4「再スケールなし」follow.rs:262 → **:265**（:686/:762 にも同契約明文・van +4049 行でも U4 契約は不変）・windowposition.rs 単位混在 doc :91-97 → **:93-94**。exact（丸め権威）先行必須は不変。**【2026-08-14 失効・下記追記(68) が正本】**——丸め権威は exact の着地を待たずに既存で充足しており、本 spec が exact を待つ理由は無くなった（ウェーブ編成上の順序自体は維持）。

> **📌 2026-08-22 棚卸⑩（W6.75 完走後の申し送り・本ブロックが以下の全追記より優先）**:
> - **着手ゲート全開・次ウェーブ＝本 spec 単独（W6.8）**: atom は 2026-08-22 に完走・`/kiro-complete` 済み（PR#114・`.kiro/specs/completed/areka-P0-dpi-transition-atomicity/`）。van・atom の両ゲートが充足し、待つ相手はもう無い。cage との同居は棚卸⑩で却下（cage③ の正典ハーネス改組と本 spec の檻消費が import 面で実干渉）＝単独フルライフサイクル。
> - **アンカー全面再実測（atom が `window_move.rs` を 965→1,223 行へ改稿＋全域 cargo fmt PR#115）**: 本 brief の `follow.rs:262/:265` は**二重に失効**——ファイルは slimming で `placement/follow/window_move.rs` へ改組済みで、U4「座標は物理 px 素通し（U4・再スケールなし）」は現在 **`follow/window_move.rs:24`**（モジュール doc・2026-08-22 実測）。`windowposition.rs` の単位混在 doc は「注意（単位空間の混在・意図的）」見出しで **:191-194**（合流実装＝`:215`・descript 加算合流＝`:407`）。旧 :91-97／:93-94 は失効。follow 系は現在 6 ファイル（`anchor.rs`・`drag_follow.rs`・`keyword_base.rs`・`visibility.rs`・`window_move.rs`・`work_area.rs`）＝design 前 rebase はこの実形に対して行うこと。
> - **設計判断の中心は追記(70) が既に確定させている**: キーワード基本位置は遷移で再導出しない・`BalloonFollow.offset` を k 倍で追随（両者排他・atom D10／要件 6.5）。残る本 spec の裁定は⑴ offset の単位空間契約の一本化（有力案＝作者空間生値＋適用点スケール・Approach 2 のまま）⑵ **SSP の k 跨ぎオラクル観測**（存在しなければ areka 設計原則から導出し COMPAT へ「areka 裁量」登記——本 spec の裁定密度の頂）⑶ persist 往復（保存 k ≠ 復元 k）の意味論。
> - **wpl 由来の隣接実装に注意**: `windowposition-limit`（PR#111）が「実表示寸確定時に一度だけ再導出」する keyword 中央揃え補正を `follow/keyword_base.rs` に持ち込み済み。atom D10 裁定は「**DPI 遷移で**再導出しない」であって keyword_base の初回確定そのものは正典——本 spec の k 倍追随は keyword_base が確定させた offset を**入力として**受ける関係。取り違えないこと。
> - **観測資産は atom の着地物を流用**: 既定 OFF の観測チャネル `wintf::transition`（`transition_diag.rs`）と実機ログ機械判定ランナー・`FrameHarness`（`crates/areka/src/emo2_boot/frame_test_support.rs`）が実機観測と決定論檻の donor になる（作り直さない）。

> **📌 2026-08-14 追記(68)（`areka-P0-scale-exact-rational` からの申し送り・有理数配管の不採用と丸め権威の存続）**: **有理数の文字層配管は行われない。丸め権威は既存のまま使える。** 両者を取り違えないこと。
> - **⑴ 配管は失効**: `ScaleRatio` の分子・分母を文字層まで配管する厳密化は **2026-08-14 に却下**された。当該 spec は裁定の登記・前提の決定論テスト・申し送りへ縮小され、実行時の挙動は 1 つも変わらない。よって本 brief の「配管を前提にする」記述（Approach 3・Upstream 欄・Constraints）は**失効**する——新しい配管や新 API の着地を待ってはならない。
> - **⑵ 丸め権威は存続**: `ScaleRatio::scale_len` / `ScaleRatio::scaled_extent`（`crates/areka-emo-compose/src/scale.rs`）は**既存のまま利用可能**（`scale-exact-rational` は式も署名も変えず、doc に例外注記を足しただけ）。bod が寸法演算に f32 を持ち込まない規律は**不変**で、必要な API は着手時点で既に揃っている。
> - **⑶ 供給面寸の例外は bod へ適用されない**: 裁定が許した f32 の例外は emo-text `ScaleContract::physical_extent`（文字供給面の確保寸）**ただ 1 点**に限られる。offset の単位空間・DPI 遷移時の変換・保存往復には**一切適用されない**——「f32 を使ってよい一般則」として読まないこと。
> - **⑷ 根拠**: 再説明しない。spec **`areka-P0-scale-exact-rational`** の裁定登記（emo-text `crates/areka-emo-text/src/region.rs` の `ScaleContract::physical_extent` doc）を参照。裁定の前提は決定論テスト `crates/areka-emo-text/tests/physical_extent_arbitration_test.rs` が固定している。
> - offset の単位空間契約と DPI 遷移規則をどう定めるかは**本 spec が決める**——上記は前提の申し送りであって、バルーン位置 DPI 追従の要件裁定ではない。

> **📌 2026-08-21 追記(70)（`areka-P0-dpi-transition-atomicity` からの申し送り・キーワード基本位置は再導出しない＝offset を k 倍する裁定）**: **DPI 遷移でキーワード由来のバルーン基本位置は再導出しないことが確定した。代わりに `BalloonFollow.offset` を k 倍で追随させる——その実装は本 spec の責任である。**
> - **⑴ 裁定**: キーワード由来のバルーン基本位置（`BalloonKeywordBase` の一度きり再導出）は **DPI／拡大率遷移で再導出しない**。遷移では `BalloonFollow.offset` を**拡大率倍（k 倍）で追随**させる。**両者は排他**——どちらか一方だけを行う（両方やれば中央揃えが二重に動く）。出所は `areka-P0-dpi-transition-atomicity` の設計判断 **D10**（同 spec `design.md` の `Architecture Pattern & Boundary Map` 内の「キー決定」 D10）と要件 **6.5**（同 spec `requirements.md` Requirement 6）。
> - **⑵ 根拠**（再説明しない・要旨のみ）: ① キーワード式 `(char_w − balloon_w) / 2` は両寸が同じ k で伸びるなら k 倍した結果と **≤1px**（`scale-exact-rational` の +1 許容）で一致する＝再導出しても k 倍しても着地点は同じ。② 再導出は一度消費した素材（`BalloonKeywordBase`）の保持と、遷移中の **2 度目の窓書込**を要し、atom が守ろうとしている遷移の原子性（一度書き）を損なう。同じ着地点なら安い方＝offset の k 倍を採る。
> - **⑶ 従属関係**: **本 spec（bod）は atom の裁定に従う**（ロードマップ W6.75「bod は atom に従う」）。atom は**裁定だけ**を行い、`follow/keyword_base.rs` の一度きり再導出を**呼ばない・変えない**（atom design の Out of Boundary）。`BalloonFollow.offset` の単位空間契約と k 倍の実装は、atom の Non-Goals に明記されたとおり**本 spec の In**。atom 側からの参照点＝ `design.md` D10 ／ `requirements.md` 6.5 ／ `tasks.md` 5.7。
> - **⑷ 帰結（着手時の前提）**: atom 着地後も **offset は非スケールのまま**（`follow.rs` U4 は不変）。キーワード基本位置は初回の寸変化で素材を消費して確定するため（`crates/areka/src/placement/follow/keyword_base.rs`）、遷移後は中央揃えがずれたまま残る——これは atom が受容した残余であり、**解消は本 spec の責務**。決定論テストの行列（Desired Outcome の k 遷移 × アンカー × 保存/復元）には「キーワード由来の offset が k 倍で中央揃えを保つ」ケースを必ず入れること。
> - **⑸ 上記⑵①の「+1 許容」を f32 の許可と読まないこと**: これは**一致の許容幅（大きさ）**の話であって、追記(68)⑶ の f32 例外（emo-text `ScaleContract::physical_extent` ただ 1 点）とは別物。k 倍の演算自体は従来どおり `ScaleRatio::scale_len`／`scaled_extent` の丸め権威に従い、offset に f32 を持ち込まない（Constraints 不変）。
> - **⑹ 「統合候補」は失効**: roadmap 追記(58)/(69) の「atom が縮退したら bod と統合して 1 spec 化」は**条件不成立で失効**した——atom は縮退せず W6.75 の**単独フル spec** として走行中（2026-08-21 時点で群 1〜4 と 5.1〜5.6 が着地済み）。残る路線は roadmap が併記するもう一方＝**atom 先着 → 本 spec が rebase** のみ。以後、統合された 1 spec を期待して本 brief を読まないこと。

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
3. `follow.rs`（DPI 遷移フック）＋`persist.rs`（保存空間の明文化）＋`windowposition.rs`（合流欄の単位統一）を是正。**`scale-exact-rational`（W6.5）の `ScaleRatio` 配管と丸め権威を前提にする**（f32 を持ち込まない）。**【2026-08-14 失効・冒頭の追記(68) が正本】**——配管は却下され行われない。丸め権威（`ScaleRatio::scale_len`/`scaled_extent`）は既存のまま利用可能で、f32 を持ち込まない規律は不変。

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

- **Upstream**: `dpi-window-vanish`（follow.rs の編集面・着地待ち）／`dpi-transition-atomicity`（**2026-08-21 確定: W6.75 の単独 spec として走行中・統合しない・atom 先着→本 spec が rebase。キーワード基本位置の裁定〔D10／要件 6.5〕に本 spec は従う＝追記(70)**。旧記「起票済・配置未確定——同一ゲート・統合候補」は**失効**）／`scale-exact-rational`（丸め権威の配管。**【2026-08-14 失効・冒頭の追記(68) が正本】**——配管は却下・丸め権威は既存充足ゆえ、上流依存は申し送りのみに縮小）／`kero-balloon`（R3.8 の窓相対契約と persist 生値保存が前提）。
- **Downstream**: `emo2-conformance-e2e` 適合 #1（DPI 検証は追従込み）。

## Existing Spec Touchpoints

- **Extends**: なし（新規境界）。**【2026-08-21 失効・追記(70)⑹ が正本】**——旧記「合流セッションで `dpi-transition-atomicity` と 1 spec に束ねる選択肢を裁定すること」は裁定済み＝**束ねない**（atom は単独フル spec）。
- **Adjacent**: `dpi-window-vanish`（follow.rs 共有＝直列必須）。

## Constraints

- 画素演算 f32 禁止（`ScaleRatio` 有理数・W6.5 exact の着地形に従う）。**【2026-08-14 失効・冒頭の追記(68) が正本】**——「exact の着地形に従う」は失効（新たな着地形は無い）。禁止の規律そのものは不変で、既存の `ScaleRatio::scale_len`/`scaled_extent` にそのまま従う。
- 実機検証は実 DPI 混在環境（125%/200% デュアル等・emo-dpi-scaling 6.5 の手順が donor）。
- 配置は合流セッション裁定。候補: W6.5 以降・van/transition-atomicity の観測結果待ち。
