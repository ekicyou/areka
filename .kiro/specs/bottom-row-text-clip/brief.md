# Brief: bottom-row-text-clip

> 起票: 2026-10-16（`areka-P0-balloon-text-bugs` の完了時の棚卸で、範囲の外の問題として `/kiro-discovery` の決まりで起票）。出どころは `completed/areka-P0-balloon-text-bugs/` の `real-machine-signoff.md` の 7.4（実機で撮った画像 `images/underline-bottom-row-zoom4x.png`）と、`tasks.md` の Implementation Notes の 2.2・2.6・4.6・最終検証の申し送り ⑷。区分 B（バグ）。触る場所が同じ文字の層の小さい残り 3 件を「件 2〜4」としてまとめた（小さいバグの起票は触る場所が同じものを 1 本にまとめる決まり）。行数は起票のとき（`balloon-text-bugs` のブランチ・2026-10-16）のもの＝着手時に引き直す。

## Problem

- **利用者**: バルーンの最下行の字の下の部分（`g`・`y` などの、ベースラインより下へ伸びる所）が、文字の領域の下端で欠ける。同梱のバルーン emo2-kakukaku・拡大率 2.0 で、最下行の `g` が `q`、`y` が `v` に見える（件 1）。
- **開発者**: 文字の層に、記録の無い抜け道と、檻で確かめていない写しが残っている（件 2〜4）。どれも観測された不具合ではないが、`balloon-text-bugs` で触った所の拾い残し。

- **件 1（本体）**: 最下行の字の下の部分が、文字の面の下端（validrect の下端）で欠ける。
- **件 2**: 潰れた描画範囲の再追従で差し込み口の entity が同時に替わると、古い差し込み口に前の字が残りうる（理論上の穴・観測は無い）。
- **件 3**: 横書きの右から左の下線の写しを、どの檻でも実物（D2D 自身の下線）と突き合わせていない。
- **件 4**: 箱の押下の後の「済み」の処理で、文字の層が見つからないとき記録なしで抜ける道がある。

## Current State

- 件 1 は実機の画像で見た（2026-10-16・debug 版の `areka.exe`・開発機 1 台・下の「件 1」の節）。下線の無い字でも欠けるかは確かめていない。
- 件 2〜4 はコードを読んで見つけた。実機でも檻でも赤は出ていない。
- 触るファイル（見込み）: 件 1＝`crates/areka-emo-text/src/layout.rs`（`visible_window`）・`region.rs`（495 行）・`actor_present.rs`（435 行）のどれか（直し方しだい）。件 2＝`actor_present.rs`・`surface_window_child.rs`（`TextSurface::release`）。件 3＝`underline.rs`（156 行）の檻（実 GPU の読み戻し）。件 4＝`crates/areka/src/input_events/shell_box_handler.rs`（386 行）。
- 細部は各件の節。

## Desired Outcome

- 件 1: 最下行の字の下の部分が欠けない。または、欠けを避けられない形（文字の領域の高さが行の高さの倍数でない、など）での決まりが要件で決まり、その決まりどおりに見える。
- 件 2: 再追従で差し込み口が替わっても、古い差し込み口に前の字が残らない。替わらないと確かめられたら、その根拠を記録して閉じる。
- 件 3: 右から左の横書きの下線の位置が、実物と檻で突き合わされている。
- 件 4: 文字の層が見つからない道でも記録が残る（ログの無い失敗の道を作らない決まり）。

## Approach

**件 1 → 件 2〜4 の順**。件 1 は要件の段で直し方を選ぶ。件 2〜4 は小さく、件 1 と同じファイルを触るついでに片づける。

1. 件 1: まず、下線の無い字だけの最下行でも欠けるかを実機で撮って確かめる（今の画像は最下行の字が全部下線の区間の中）。次に、欠けの量が行の矩形（字の大きさの箱）と字の実際の下端のどちらで決まっているかを測る。
2. 件 1 の直し方は要件の討議で決める（下の「件 1」の「守る決まり」の範囲で）。
3. 件 2: 再追従で差し込み口の entity が同時に替わる場面が実際にありうるかを読み、ありうるなら古い差し込み口から外す形にする。
4. 件 3: 実 GPU の檻で、右から左の横書きの下線を D2D 自身の下線と突き合わせる（縦書きの檻〔`balloon-text-bugs` タスク 2.6〕と同じ形）。
5. 件 4: 抜け道に `error!`（または `warn!`）を足す。

## Scope

- **In**: 件 1 の再現の撮り直し・欠けの量の測り・直し（要件で選んだ形）・実 GPU の読み戻しの檻／件 2 の場面の確かめと直し／件 3 の檻／件 4 の記録。
- **Out**: 下線の引き方そのもの（`balloon-text-bugs` で確定＝ベースラインの直下・収まらないときだけ内側へ寄せる）・文字の領域（validrect）の決め方（09-05 の裁定）・折り返しの基準（`wordwrappoint`）・縦書きの字の欠け（要るなら議題）・アンカーの見た目（`anchor-style-canon`）。

## Boundary Candidates

- 最下行が「収まる」の数え方（`layout.rs` の `visible_window` が使う行の矩形）と、字が実際に塗る範囲（ink の矩形）の違い。
- 文字の面の大きさと、行を置く位置（面は validrect ちょうど＝変えない）。
- 差し込み口の付け外し（`TextSurface::release` と再追従）。
- 箱の押下の記録（`shell_box_handler.rs`）。

## Out of Boundary

- 文字の面を validrect より広げること（09-05 の裁定・下の「守る決まり」）。
- 下線の有無で行の位置や入る行の数を変えること（2026-10-16 の裁定）。
- 字の形を変えること（`DrawTextLayout` の描き方）。

## Upstream / Downstream

- **Upstream**: ✅ `balloon-text-bugs`（10-16 完了・下線の引き方と、潰れた描画範囲の扱いを決めた）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: 完了 `balloon-text-bugs`（件 3 の下線・件 2 の潰れた描画範囲・件 1 の箱の選択肢の済み）。
- **Adjacent**（同じファイルを触る＝同じウェーブに置けない相手）:
  - `actor_present.rs`: `choice-ranges-one-function`・`range-choice-tag`・`balloon-font-file`・`anchor-style-canon`・`text-typesetting`・`talk-fast-forward` ほか、文字とバルーンの列のほぼ全部。
  - `region.rs`: `real-run-log-noise`・`balloon-font-file`・`text-typesetting`・`balloon-canon-residue`。
  - `layout.rs`: `text-typesetting`・`text-ruby`・`balloon-markers`・`balloon-scroll-fade`・`text-align-shadow-canon`・`balloon-canon-residue`。
  - `surface_window_child.rs`（件 2 で触るなら）: `balloon-element-order`・`clippy-199-lints`。
  - `underline.rs`: `anchor-style-canon`・`choice-marker-styling`（どちらも下線の見た目を変える＝件 3 の檻を先に置けば、あちらの直しを捕まえられる）。
  - `shell_box_handler.rs`: `range-choice-tag`・`link-context-copy`・`balloon-link-hover`・`choice-marker-styling`・`balloon-markers`・`talk-fast-forward`。
  - `text-typesetting`: 行の高さと行送りを変える spec。件 1 の直しが行の置き方に触る形になったら、こちらと数え方を揃える。

## Constraints

- 段は**バグ**・区分 B。
- **守る決まり（件 1 の直し方を選ぶときの枠）**:
  - 面は validrect より広げない（09-05 の裁定「面は validrect ちょうど」・`validrect` は超えてはならない絶対の上限）。
  - 下線の有無でレイアウト（行の位置・入る行の数）を変えない（2026-10-16 `balloon-text-bugs` 議題 3・開発者「それなら字にかぶるほうがまし」）。
  - 直し方の候補（例: 入る行の数を字の実際の下端で数える／最下行を内側へ寄せる／今のままにして決まりとして記録する）は、要件の討議で開発者と決める。ここでは決めつけない。
- 実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ。emo2 の実走は絶対パスかつ短いパス。止めてよいプロセスは、自分が起こしたと確かめられるものだけ。
- GPU のテストは実 GPU で行う。
- 規模の見込み: S〜M（6〜11。件 1＝4〜7・件 2〜4＝合わせて 2〜4）。
- 要件定義のモデル: Opus。

---

## 件 1: 最下行の字の下の部分が欠ける（本体）

### 見たこと

`balloon-text-bugs` のタスク 5.5（2026-10-16・`real-machine-signoff.md` の 7.4）。同梱のバルーン emo2-kakukaku の面 0（400×224・validrect 36/46/-44/-56＝文字の領域 320×122）・Yu Gothic UI 28 画素・拡大率 2.0（プライマリ DPI 192）で、4 行の台詞を MCP の `dump_balloon` で撮った。

- 3 行目の「gy」は、字の下の部分が下線（y=136）を越えて y=142 まで描かれている（約 6 画素）。
- 4 行目（最下行）は、文字の領域の下端 y=168 から下に字の画素が 0。最下行の `g` は `q`、`y` は `v` のように見える（作者の下線「下線gyぁ」とアンカー「アンカgyぁ」の両方）。
- 画像は `completed/areka-P0-balloon-text-bugs/images/underline-bottom-row-zoom4x.png`（3〜4 行目を最近傍で 4 倍）。

### 見立て（確かめていない）

- 文字の面は validrect ちょうどの大きさ（高さ 122・下端 y=168）で作られ、4 行目のベースラインは y≈166。字の下の部分は約 6 画素下まで要るので、172 付近まで要るところが 168 で切れる。
- 行の矩形（字の大きさの箱）で数えると 4 行目は収まるとされ、見える窓（`visible_window`）のスクロールも起きない。
- 下線の有無とは関係なく起きる見込み。ただし今回の台詞は最下行の字が全部下線の区間の中にあり、**下線の無い字で欠けるかは確かめていない**＝要件の段の最初に確かめる。

## 件 2: 再追従で差し込み口が替わると古い差し込み口に字が残りうる

- `crates/areka-emo-text/src/actor_present.rs` の、描画範囲が潰れたときの片付け（136 行付近）は `render.surface.release(world, binding.slot)` を呼ぶ。`binding` は今の結線の写しなので、再追従で差し込み口の entity が同時に替わっていると、新しい差し込み口から外そうとし、古い差し込み口に前の字が残りうる。
- 観測は無い。差し込み口は窓ごとに固定と見られる（`balloon-text-bugs` の最終検証の申し送り ⑷）＝まず、替わる場面がありうるかを読む。

## 件 3: 右から左の横書きの下線を実物と突き合わせていない

- `crates/areka-emo-text/src/underline.rs` の `stroke_rect` は、右から左の横書きでは原点から左へ幅を伸ばして写す。`balloon-text-bugs` の檻（タスク 2.3＝左から右の書体だけ・2.6＝縦書き）のどれも、この腕を実物（D2D 自身の下線）と突き合わせていない（同 spec の Implementation Notes の 2.2・2.6）。
- 同じ関数の `origin` の引数は本番で常に (0,0)（タスク 2.5 で原点を Draw に渡した名残）。檻を足すときに、要らなければ外す。

## 件 4: 箱の押下の後の済みで記録なしに抜ける道

- `crates/areka/src/input_events/shell_box_handler.rs` の `send_selection`（292 行付近）は、選択を送れた後に `if let Some(runtime) = runtime_of(world) { settle_after_selection(…) }` で箱の選択肢を済みにする。`runtime_of` が `None` のときは何も記録せずに抜ける。
- 今は押下の入口（`locate`）が文字の層を前提にするので、この道は通らない（`balloon-text-bugs` の Implementation Notes の 4.6）。それでも、ログの無い失敗の道を残さない決まりに合わせて記録を足す。

## 優先度の区分

B（バグ。件 1 は利用者に見える字の欠け。件 2〜4 は同じ場所の拾い残し）。

## 2026-10-16 棚卸㉖で足した件（`balloon-text-bugs` の最終確認の拾い残し）

- 件 5（小さい）: `crates/areka-emo-text/src/underline.rs` の 62 行あたりの `stroke_rect` は、引数 `origin` が本番ではいつも (0,0) で渡される（使われていない引数）。件 3 で `stroke_rect` に触るときに一緒に外すか、意味のある使い道を決める。持ち主が居なかったので、同じ関数を触るこの spec へ渡した。
