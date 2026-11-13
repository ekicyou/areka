# Brief: emo-text-reveal-incremental

> 起票: 2026-11-12（`audit-emo-text-layout-draw` の完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/audit-emo-text-layout-draw/tasks.md` の Implementation Notes「14.1 の伝えるだけ（2 回目）」。どちらも見直しの読み手が範囲の外（`actor*`・`state*`）で見つけ、持ち主だった `audit-emo-text-state-actor` はもう完了している。行数は起票のとき（main `9ca4a2fc` を取り込んだ枝）のもの＝着手時に引き直す。

## Problem

- **利用者**: 長い台詞を 1 字ずつ見せると、字が届くたびの手間が、それまでに届いた字の数に比例して増える（台詞全体で字の数の 2 乗）。ゴーストの台本は大きくなりうる（決め「ゴーストの台本の大きさを甘く見ない」）ので、長い台詞の終わりほど 1 フレームが重くなる。
- **開発者**: 2 か所とも、覚えの鍵や当て直しの形が「全部やり直す」になっている。

- **件 1**: `crates/areka-emo-text/src/actor_present.rs` の `BreakMemo`（770 行あたり）の鍵は（消した回数, 項目の数）。字が届くたびに項目の数が変わるので覚えが当たらず、先渡しの全文が無い（`Basis::Arrived`）文節の折返し（`WrapMode::BudouxWordWrap`）では、字が届くたびに届いた全文を BudouX で区切り直す（`segment_plan(items)`・N 字で N 回 × 全文）。
- **件 2**: `crates/areka-emo-text/src/state_decoration.rs` の `Decoration::rebase`（195 行あたり）は、呼ばれるたびに既定の見た目から始めて、それまでに当てた `\f` の列（`applied`）を全部当て直す。呼び手は 170・497・593 行あたりの 3 か所。`\f` の切り替えが S 回ある台詞では、当て直しが S 回 × 当てた列の長さ（候補の数 N）になる。

## Current State

- 件 1 の覚えは `audit-emo-text-state-actor` の指摘 8 で入った（同じ鍵なら区切りを使い回す）。先渡しの全文がある道（`Basis::Full`）は全文の側の区切りを使うので、この件は当たらない。1 字ずつ（`CharByChar`）も区切りを計算しない。
- 件 2 の当て直しは、2 層（既定・無効表示・選択肢文字色）の差し替えのあとで今の見た目を作り直すためのもの。
- どちらも観測された遅さではなく、コードを読んで見つけた。檻も無い。

## Desired Outcome

- 件 1: 字が 1 つ届くときの区切りの手間が、届いた全文の長さに比例しない（例: 変わった所の近くだけを区切り直す・前の区切りを伸ばす）。区切りの結果は今と同じ（決定論の檻で前と後を比べる）。
- 件 2: `\f` の切り替え 1 回の手間が、それまでに当てた列の長さに比例しない。または、比例しても上の数が要件で示されている。見た目の結果は今と同じ。
- どちらも、見た数を数える試験用の数え（`layout_scan.rs` の `FOLLOWING_SCAN_VISITS` と同じ形）で「項目数の定数倍以下」を固定し、2 乗に戻すと赤になる。

## Approach

件 1 → 件 2 の順。直し方は要件の討議で選ぶ（ここでは決めつけない）。

1. 件 1: BudouX の区切りが「後ろに字が足されたとき、どこより前は変わらないか」を確かめる（文節の境目は後ろの字で動きうる）。変わらない所が言えるなら、前の区切りを残して後ろだけ区切り直す。言えないなら、区切り直す間隔（何字ごと）を決めるか、先渡しの全文の道に寄せる案を議題にする。
2. 件 2: 2 層が変わらない間は、当てるたびに今の見た目を積み増し、2 層が変わったときだけ全部当て直す、などの形を読む。
3. 両方とも、前と後で結果が同じことを決定論のテストで比べる。

## Scope

- **In**: 件 1 の区切りの差分化（または代わりの形）と檻／件 2 の当て直しの形と檻／見た数の数え。
- **Out**: BudouX の区切りの規則そのもの・先渡しの全文の道（`lookahead.rs`）の作り・配置の層（`layout*.rs`）の手間（`audit-emo-text-layout-draw` で済み）。

## Boundary Candidates

- 届いた字の区切りの覚え（`actor_present.rs` の `BreakMemo` と `segment.rs` の `segment_plan`）。
- 見た目の当て直し（`state_decoration.rs` の `Decoration`）。

## Out of Boundary

- 配置・描画の手間（`audit-emo-text-layout-draw` が持っていた範囲）。
- `\f` の解き方（`look.rs` の `apply_font_tag`）。`look.rs` は 998 行（2026-11-13 棚卸㊱の実測・あと 2 行）なので、触るなら先に分ける。

## Upstream / Downstream

- **Upstream**: ✅ `audit-emo-text-state-actor`（`BreakMemo` を入れた）・✅ `audit-emo-text-layout-draw`（配置の層の 2 乗を消した）・✅ `areka-P0-budoux-reveal-reflow`（文節の折返しと先渡し）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: 完了 `audit-emo-text-state-actor`（指摘 8 の覚え）。
- **Adjacent**（同じファイルを触る）: `actor_present.rs` は文字とバルーンの列のほぼ全部（`bottom-row-text-clip`・`range-choice-tag`・`text-typesetting`・`talk-fast-forward` ほか）。`state_decoration.rs` は `text-align-shadow-canon`・`font-size-keywords`・`choice-marker-styling`。

## Constraints

- 結果（区切り・見た目）は今と同じ。変えるなら要件で開発者と決める。
- 1 ファイル 1,000 行まで。log-first。時刻は正確に扱う（1 フレーム遅らせる解は取らない）。
- 規模の見込み: S〜M（6〜11・件 3 を含む）。
- 要件定義のモデル: Opus。

## 優先度の区分

A（見直しの指摘の残り。開発者「とにかく指摘事項は全部直す方針です」の範囲）。起票の訳（`/kiro-complete` の棚卸の条件）: 直し方が複数あり（区切りの差分化・区切り直す間隔・先渡しへ寄せる）、BudouX の区切りがどこまで動くかの調べが要る。

## 件 3（同じ actor 層の拾い残し・記録の出どころ）: `WritingDirectionDecision::resolve` が自分で `warn!` を出す

- `crates/areka-emo-text/src/writing.rs` の `WritingDirectionDecision::resolve`（289〜313 行あたり）は、純粋な解決の中で警告（と矛盾併記の `debug!`）を出す。`region.rs` は警告を登録口へ移してあるのに、こちらは解決のたびに出るので、DPI の再追従や箱の解決で同じ警告が繰り返される。
- `audit-emo-text-layout-draw` の完了の棚卸で直そうとしたが、登録口が 1 つに決まらない（バルーンの装着 `register_actor_binding`・再追従 `refresh_actor_binding`・箱の登録 `actor_box.rs` の `register_box` の 3 つが `ResolvedBalloonText::resolve_with_background` を通る）。`writing_decision_tests.rs` の 10 本ほどと `writing.rs` の tests が解決を直に呼んで件数と文言を固定しているので、登録口の檻へ移し替えが要る。`Decision` は `Copy` を守るため生の値を持たず、登録口でモデルから読み直す形になる。
- 決めること（要件の段）: 登録口をどれにするか（装着と箱の 2 か所か、値の変わる再追従も入れるか）・矛盾併記の `debug!` も一緒に移すか。
- 起票の訳: 直し方が複数あり、開発者の裁定が要る。件 1・2 と同じ `actor*` の層で、規模は小さい（2〜3）ので同じ spec へまとめた。
