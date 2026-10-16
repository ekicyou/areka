# Brief: areka-P0-real-run-input-guard-observations

> 起票: 2026-10-16（`areka-P0-balloon-text-bugs` の実機の確かめ〔タスク 5.3〕で、判定に使わなかった回に見た 2 件。範囲の外の問題として、完了時の棚卸で `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-balloon-text-bugs/real-machine-signoff.md` の 1 章の走行の一覧と 4 章、`tasks.md` の Implementation Notes の「範囲外の観察」。生の記録（`target\btb-signoff\run-Wait-1.log`・`run-Timeout-1.log`）は `target\` の下で git に残らず、消えうる＝**正本は signoff の 4 章の記述**（要る行は下に写した）。**どちらも 1 回だけ見て、再現していない。原因は確かめていない。** 区分 C（原因が分からない観察・再現待ち）。

## Problem

- **件 1**: ポインタの移動と押下を 1 回の `SendInput` で同時に送ると、areka の記録に移動も押下も 1 行も残らなかった。先に移動してから押すと通る。
- **件 2**: 操作していない時間に、バルーンの窓を作業領域へ引き戻す警告（`[visibility-guard]` と `[balloon-limit] Clamp`）が 565 行続いた。

- **利用者**: 件 1 は人の手の操作では起きにくい（移動と押下が同じ瞬間に届くことはまず無い）。ただ、操作を自動で送る道具（支援技術・自動化の道具）を使う利用者には、押したのに何も起きない形で出うる。件 2 は、もし利用者の機械でも起きるなら、バルーンが画面の端で引き戻され続ける（見た目の揺れ・記録の洪水）。
- **開発者**: 実機の確かめを `SendInput` で回すと、理由の分からない空振りが混ざる（件 1）。記録が警告で埋まる（件 2）。

## Current State

どちらも 2026-10-16 の `balloon-text-bugs` の実機の確かめ（HEAD `c74da005`・debug 版の `areka.exe`・同梱の emo2 と emo2-kakukaku・プライマリ DPI 192〔k=2.0〕・開発機 1 台・走らせる道具は同 spec の `real-machine-run.ps1`）。

### 件 1: 移動と押下を同時に送ると、どちらも記録に残らない（Wait の 1 回目）

- 台本は選択肢の後ろに字と `\w9`×20（約 9 秒の待ち）。待ちの最中の押下は、今の決まりどおり `choice_rejected_no_wait` で捨てられた（2 回目と同じ）。
- 待ちが明けた後、ポインタを画面の別の場所から選択肢の行の上へ動かすのと同時に押した（**移動と押下を 1 回の `SendInput` で送った**）。areka の記録に、移動（強調の `choice_hover_inject`）も押下（`choice_selected` や `choice_click_rejected`）も 1 行も残らなかった。
- 選択待ちはそのまま 30 秒の既定で時間切れ、その 5 秒後にバルーンが隠れて `OnBalloonTimeout`。
- 2 回目は、先にポインタを行へ運び、0.4 秒置いてから押して通った（`choice_accepted` → `choice_resolved`）。
- **見立て（確かめていない）**: バルーンの窓はふだんクリックを下へ通す（クリック透過）。ポインタが窓の上に来たことを知って透過を外す判定が、移動の知らせを受けてから働くなら、同じ瞬間に届いた押下は透過のまま下の窓へ抜ける。判定の所は wintf の窓の手続き（`crates/wintf/src/ecs/window_proc/`）と見られる。

### 件 2: 操作していない時間に引き戻しの警告が 565 行続いた（Timeout の 1 回目）

- 1 本目の選択待ちの最中（10:11:49 UTC・操作の道具は何も送っていない時間）に、`[balloon-limit] Clamp runtime 関門` と `[visibility-guard] OffscreenPull／NearestFallback／ClampX` が 565 行続いた。バルーンの窓の書込位置 `x=-46` → `-323` を作業領域へ引き戻す、が繰り返された。
- 直前に `loop ticker catch-up: skipped multiple boundaries`（刻みが詰まった印）。
- 2 本目の測りの前には、むらさきの窓の 1 枚が最小化の位置（`-32768`）に在った（走らせる道具が測る窓を取り違えて止まった）。
- 道具を「最小化の位置の窓を除く」に直した 2 回目では再現しなかった。
- **見立て（確かめていない）**: 外から窓が動かされた（最小化・別のアプリの操作など）ことへの追従と、引き戻しの関門が互いに押し合った疑い。触る所は配置の追従（`crates/areka/src/placement/follow/visibility.rs`・`work_area.rs`）とバルーンの関門（`crates/areka/src/placement/balloon_limit.rs`）と見られる。

## Desired Outcome

次のどれかで、件ごとに閉じる。

- 再現の条件が分かり、areka の側で防げるなら防ぐ（件 1 なら、移動と押下が同時に届いても押下が拾われる。件 2 なら、引き戻しが繰り返されず 1 回で落ち着く）。直し方は要件で選ぶ。
- areka の側では防げない（OS の入力の決まり・外のアプリの操作）と分かったら、その根拠を記録して閉じる。
- 再現しないなら、試した回数と条件を記録して閉じる。
- どの場合も、次に同じことが起きたら原因が読めるように、判定の分岐の記録が足りているかを確かめる（件 1 なら透過を外す判定の記録・件 2 なら引き戻しの前後の窓の位置と出どころ）。

## Approach

**最初のタスクは再現**。再現しなければ、記録を残して閉じてよい。

1. 件 1: `SendInput` で移動と押下を 1 回で送る形と、間を空ける形を、同じ台本で数十回ずつくり返して割合を数える。バルーンの窓と、普通のシェルの窓・箱の選択肢でも比べる。再現したら、透過を外す判定がいつ働くかを記録で追う。
2. 件 2: 窓を外から最小化する・別の位置へ動かす・刻みを詰まらせる（重い処理を並べる）の組み合わせで、引き戻しの繰り返しが起きるかを試す。
3. 結果で、上の Desired Outcome のどれで閉じるかを決める。直すと決めたら、その時点で触るファイルを測り直す。

## Scope

- **In**: 2 件の再現の試み・割合の数え・原因の切り分け・（areka の側で防げると分かったとき）その直し・（防げないとき・再現しないとき）記録・判定の分岐の記録の足し。
- **Out**: 選択肢の待ちと押下の決まりそのもの（完了 `balloon-text-bugs`・`choice-interact`）・拡大率の切り替えでの位置の詰め直し（`dpi-realign-remembered-chain`）・窓のスナップ（`window-snap`）・3 人目以降の窓（`extra-character-windows`）・確かめの道具そのものの直し（同 spec の `real-machine-run.ps1`）。

## Boundary Candidates

- クリック透過を外す判定と、ポインタの移動・押下の届く順（wintf の窓の手続き）。
- 窓の位置の追従と、バルーンを作業領域へ引き戻す関門（`placement/follow/`・`placement/balloon_limit.rs`）。
- 実機の走行の手順（`SendInput` の送り方・窓の数え方）と、失敗した回の証拠の残し方。

## Out of Boundary

- 人の手の操作の振る舞い（今は問題が出ていない＝変えない）。
- 引き戻しの決まり（作業領域の外へ出たら戻す）そのもの。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: `SendInput` で操作する実機の確かめすべて（件 1 が分かるまでは「先に移動してから 0.4 秒置いて押す」の手順で回す）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**（読んで、境界を確かめた）:
  - `dpi-realign-remembered-chain`・`dpi-transition-two-tick-bounce`・`window-snap`・`extra-character-windows`: 配置の追従（`placement/`）を触る。件 2 で直すと決めたら、これらと同じウェーブに置かない。
  - `elevated-drop-filter`・`tasktray-icon`・`zorder-chain-residue`: wintf の窓の手続きを触る・読む。件 1 で直すと決めたら照合する。
  - `emo2-real-run-stability`: emo2 の実機の走行の別の赤（待ちの時間切れ・SHIORI の LOAD の失敗）。原因が違う。実機の窓が出るので、同じ時間に回さない。
  - `areka-test-binary-stability`・`present-emit-tail-latency`: 刻みの詰まりと負荷の下の揺れを測る spec。件 2 の「刻みが詰まった印」が効いていたら、そちらの測りと照らす。

## Constraints

- 段は**その他**・区分 C。見たのはどちらも 1 回＝まず再現を確かめる。
- 実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。emo2 の実走は絶対パスかつ短いパス。
- 止めてよいプロセスは、自分が起こしたと確かめられるものだけ。外から窓を動かす試しは、自分が起こした areka の窓だけに行う。
- 実機の窓が出る・操作を送るので、負荷を掛ける検査と同じ時間には回さない。スクリーンセーバー中・入力デスクトップが `Default` でないときは操作しない。くり返しの走行は、回す前に所要時間を伝える。
- 判定の分岐の記録まで `RUST_LOG` を開けて回す。
- 規模の見込み: XS〜S（確かめと記録だけ 2〜4・areka の側の直しが要ると分かれば 1 件につき 3〜5 タスク）。
- 要件定義のモデル: Opus。

## 優先度の区分

C（原因が分からない観察・再現待ち。人の手の操作では今のところ問題が出ていない）。
