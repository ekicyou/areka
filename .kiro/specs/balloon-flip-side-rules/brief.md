# Brief: balloon-flip-side-rules

> 2026-10-30 `window-work-area-clamp` の完了時の棚卸で `/kiro-discovery` で起票（対話なし・完了した spec からの引き渡し）。出どころは同 spec の最終検証の「棚卸へ渡す範囲外の項目」の ② と ③（`completed/window-work-area-clamp/tasks.md` の Implementation Notes）・設計の Risks「バルーンの横のずらしの符号」・組 5 の決めごと ②。起票の条件: どちらも直し方が開発者の裁定に要り、件 2 は SSP で測ってから決める。区分 **B**（件 1 は見た目の崩れ。件 2 は確かめていない疑い）。

## Problem

- **件 1（バルーンを反対の側へ並べ替えると、相方の窓に重なりうる）**: キャラ窓が作業領域に収まっていてバルーンだけが横にはみ出すとき、areka はバルーンをキャラ窓の反対の側へ並べ替える（`window-work-area-clamp` の要件 5.3）。並べ替えはそのバルーンのキャラ窓と作業領域だけを見て、相方のキャラ窓を見ない。たとえば本体が作業領域の左端にいて、起動の並べ直しが相方を本体の右へ返した後（同 spec の 4.3 ⑴ の形）、本体の左隣のバルーンがはみ出して右へ並べ替わると、右に置かれた相方の窓の上に重なる。
- **件 2（右に出すバルーンの横のずらしの向き）**: 起動で右に出すバルーンの位置（`placement/resolver.rs` の P5）は、`balloon.offsetx` を右に出すときも画面の右向きに足す。ukadoc の読み（横の＋はどちらの側でもシェルに近づく向き）と、並べ替え（キャラ窓の縦の中心線で映す＝シェルからの間合いが左右で同じ）と食い違うおそれがある。作者が `balloon.alignment,right` と横のずらしを書いたバルーンでは、初めから右に出たバルーンと、左から並べ替えて右に来たバルーンの位置がずれる。SSP は横のずらしが 0 のバルーン（ぱすたの相方）しか測っていない。

## Current State

- 並べ替えは `crates/areka/src/placement/balloon_limit.rs` 157 行目の `place_balloon_in_work_area`（301 行）。受け取るのはバルーンの位置と大きさ、そのバルーンのキャラ窓の矩形、作業領域だけ（174〜187 行目）。呼ぶのは起動の関門 `apply_balloon_limit_one`（同 245 行目・呼び出しは 273 行目）と書き込みの関門 `apply_balloon_limit_gate`（`placement/follow/window_move.rs` 989 行目・呼び出しは 1083 行目）の 2 か所。どちらの関門も、ほかのスコープの窓の矩形を渡していない。
- 映しの式は `x' = char.left + char.right − (pos.x + size.w)`。左右の側を表す値は持たない。並べ替えた先が横に入らなければ今の押し込みに倒れる（要件 5.5）。記録は `[balloon-limit] Clamp … how="flipped"`／`"pushed"`。
- 起動の並べ直しで相方を本体の反対の側へ返すのは `placement/follow/work_area_clamp.rs` の鎖の寄せ（`ChainClampOutcome::Flipped`・162〜169 行目）。そちらは前の窓の寄せた後の矩形を見るが、バルーンは見ない。
- 4.3 ⑴（emo2・記憶に本体だけ・本体を主の画面の左端に）では相方が本体の右へ返された（相方 `x=764`・本体 `0,610 764×1094`）。この回は並べ替えを入れる前（5.1 の前）で、本体のバルーン（800×448）は `from=(-268,352) to=(0,352)` と押し込まれた（`target\wwac43\c1\areka.out` 178 行目）。今の式で映すと x は 232〜1032 になり、相方の窓（764〜）に重なる見込み（計算だけ・実機では見ていない）。4.4（配布版のぱすた）は本体と相方の窓が同じ位置に重なる形で、件 1 は起きない。
- 右に出すときの P5 の式は `resolver.rs` 335〜343 行目（`x.saturating_add(w)` に `ox` を足す）。左に出すときは `x − balloon.w + ox`。キーワードの横の位置の式（`keyword_balloon_pos`）は別の道で、この件の外。
- 配布版のぱすた（`pasta-in-windows`）は `sakura.balloon.alignment,left` と `sakura.balloon.offsetx,476`（バルーンをシェルの上に重ねる形）。左に出す形なので件 2 には当たらない。
- 組 5 の決めごと ②（2026-10-30・開発者が一括承認）で、並べ替えの横のずらしは ukadoc どおり（シェルへ向かう向き・両側で同じ距離）とし、初期配置との食い違いはここで起票すると決まっている。

## Desired Outcome

- 件 1: 並べ替えたバルーンが相方のキャラ窓に重なるときの扱いが決まり、その通りに動く。案は ⑴ 相方の窓も避ける（避けられなければ押し込み） ⑵ 重なるときは並べ替えずに押し込みへ倒す ⑶ 重なってよい（今のまま・記録に残す）のどれか。
- 件 2: 右に出すバルーンの横のずらしの向きが SSP で測った結果に合い、初めから右に出たバルーンと、並べ替えて右に来たバルーンが同じ位置になる。
- どちらも決定論のテストで固定し、SSP との違いが残るなら `doc/COMPAT_ARCHITECTURE.md` §8 に残す。

## Approach

- 要件の段の最初に、SSP で測る（打つ前に許し・ウェーブの兄弟に知らせる）: ⑴ 本体を作業領域の端に置いて相方を反対の側へ返した形で、本体のバルーンがどこへ行くか（相方の窓に重なるか） ⑵ `balloon.alignment,right` と 0 でない `balloon.offsetx` を書いた検体で、初めの位置と、窓を端へ寄せて反対の側へ並べ替わった位置。取り方は MCP の `get_property` の `currentghost.balloon.scope(n).rect`（`research.md` 13 節と同じ）。
- 件 1 の第一の案は、関門に「ほかのスコープのキャラ窓の矩形」を渡し、並べ替えた先がそれに重なるなら押し込みへ倒す（式は `place_balloon_in_work_area` の中の 1 段）。相方の窓を避けてさらに動かす形は、3 体以上の窓（`extra-character-windows`）でも同じ規則で解けるかを要件の段で確かめる。
- 件 2 は測った結果で P5 の右の式の符号を決める。変えるなら、記憶や `balloon_offset`（相対の位置）を持つ今の利用者の見た目が変わるので、どこまで変わるかを要件の段で数える。
- 直す前に赤になる決定論のテストを置く（相方の窓に重なる並べ替え・右に出すバルーンの横のずらし）。

## Scope

- **In**: 並べ替えの判定に相方の窓を入れること（件 1）・右に出すバルーンの初期配置の横のずらしの向き（件 2）・記録の欄・決定論のテスト・SSP の測り・§8 の行。
- **Out**: キャラ窓の鎖の寄せそのもの（`work_area_clamp.rs`）・キーワードの横の位置の式（`keyword_balloon_pos`）・バルーンの縦の置き方・3 体以上の窓（`extra-character-windows`）。

## Boundary Candidates

- 並べ替えの判定（`placement/balloon_limit.rs` の `place_balloon_in_work_area`）と 2 つの関門の呼び出し（`apply_balloon_limit_one`・`window_move.rs` の `apply_balloon_limit_gate`）
- 初期配置の右の式（`placement/resolver.rs` P5）

## Out of Boundary

- バルーンの押し込みの式そのもの（`limit_correction`）
- 利用者がバルーンを離したときの補正

## Upstream / Downstream

- **Upstream**: 完了 spec `window-work-area-clamp`（`.kiro/specs/completed/window-work-area-clamp/`・要件 5.3〜5.8・`research.md` 13 節・設計の BalloonFlip と Risks）。完了の `areka-P0-windowposition-limit`（押し込みの決まり）・`areka-P0-balloon-offset-dpi`。
- **Downstream**: `extra-character-windows`（3 体目の窓の並べ替え）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec の範囲外の項目の引き渡し）。
- **Adjacent**: `balloon-canon-residue`（`placement/resolver.rs` のバルーンの左右の判定を触る）・`window-snap`（`resolver.rs` の吸着）・`audit-placement-core`・`audit-placement-follow`（見直し）・`placement-crate-split`（`placement/` を `areka-placement` へ移す＝同じウェーブに置かない・パスは roadmap の「本体の crate 分け」の対応表で読み替える）。

## Constraints

- 失敗の道は必ず記録（`error!`＋`Err`）。並べ替えと押し込みの記録は今の `[balloon-limit] Clamp` の行の欄で足す（新しい語を作らない）。
- テストは兄弟のファイル `*_tests.rs`。`window_move.rs` は前から 1,000 行を超えている（1,227 行）＝足す分は `balloon_limit.rs` の側に置く。
- ゴーストの名前・初回起動を見る分岐を入れない（`window-work-area-clamp` 要件 7.8 と同じ）。
- SSP を測るときは毎回開発者の許しを取る。試験の根・検体はワークツリーの `target\` の下だけ。

- **優先度の区分**: B（件 1 はバルーンが窓に重なる見た目の崩れ。件 2 は確かめていない疑い）。
- **版**: v0.2.0（致命的でない新しいバグ＝次の版。v0.1.0 の検体のぱすたは本体と相方の窓が重なる形で、バルーンは左に出すので、どちらの件にも当たらない）。
- **要件定義のモデル**: Opus。今ある判定に段を 1 つ足すかと、式の符号の決めで、新しい境界・並行の設計には当たらない。
- **規模**: S（4〜7 タスクの見込み・見積もりは要件の段）。
- **分割の案**: なし（触る場所がどちらもバルーンの置き方の式で、SSP の測りも 1 回の机でまとめて取れる）。
- **要件の段の議題**（答えで作業が変わるもの）:
  1. 件 1 の扱い（相方の窓も避ける／押し込みへ倒す／重なってよい）。SSP の測り ⑴ を見て決める。
  2. 件 2 の右の式の符号を変えるか。SSP の測り ⑵ を見て決める。変えるなら今の利用者の見た目が変わる範囲。
