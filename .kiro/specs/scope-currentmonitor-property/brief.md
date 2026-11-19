# Brief: scope-currentmonitor-property

> 2026-11-19 `/kiro-discovery` で起票。出どころは、ghost_dev の「ぱすた開発」セッションが開発者の発注で書いた台本「席替え」（今の SSP だけで遊べるノベル）。開発者「同じ台本を SSP でも areka でも動かす」「areka 環境に実装の起票投げておいてね」。開発者は 2026-11-19 に、小さい spec に切り出す形を承認した（「推奨で起票」）。

## Problem

- **ゴーストの作者**: 窓が居るモニターの範囲を知りたいのに、areka では値が出ない。辞書は空文字を受け取って進み、作者にも遊ぶ人にも何も見えない（黙って壊れる）。
- **台本「席替え」**は、自分の窓の矩形とモニターの範囲を比べて、左寄りか右寄りかを決める（「廊下側」の席を、元の席と反対側の端にする）。画面の端へ寄せるときに、タスクバーを避けたい。
- SSP 2.9.34 では読める（「ぱすた開発」が実機で確かめた値: `currentghost.scope(0).currentmonitor.rect`＝`-2560,195,0,1795`・`currentghost.scope(0).rect`＝`-2563,819,-1732,1845`）。

## Current State

起票のときの実測（main `e382b09b0`）:

- `currentghost.scope(ID).currentmonitor.*` の 5 葉のうち、動くのは `dpi` だけ（完了 `shell-scale-linkage`）。残りの 4 葉（`rect`・`work`・`primary`・`bpp`）は名前だけ在って値が出ない（網羅台帳 `doc/ukadoc-coverage/ledger/property.toml` で `vocabulary-only`・持ち主は `currentghost-property-tree`）。
- 隣の `currentghost.scope(ID).rect` は動く（完了 `mcp-scope-rect-property`）。
- **読み手の型は在る**: 聞かれた時に値を取りに行く読み手（`crates/areka/src/ghost_session_scope_rect.rs`・`crates/areka/src/ghost_session_scope_scale.rs`）。同じ型で 4 葉を足せる見込み。
- **モニターの情報の出どころも在る**: wintf がモニターの矩形と作業領域を持つ（`crates/wintf/src/ecs/window/monitor.rs`）。
- 持ち主の `currentghost-property-tree`（v0.3.0）は、動く値を出す口と `currentghost.balloon.scope` の 19 項目だけを持ち、残りは `currentghost-property-others`（いつか）へ切ってある。席替えの台本は、そこまで待てない。
- **正典（ukadoc を 2026-11-19 に引き直した・どれも SSP 2.8.16）**:
  - `.rect`: 「ウインドウが表示されているモニターの矩形座標(左,上,右,下)」。
  - `.work`: 「ウインドウが表示されているモニターの作業領域(左,上,右,下)。タスクバー等のシステム予約領域を除いたウインドウを配置できる領域」。
  - `.primary`: 「プライマリモニターかどうか。プライマリの場合は1、それ以外は0」。
  - `.bpp`: 「モニターの色深度(bits per pixel)」。
  - 出どころ: `https://ukatech.shillest.net/ukadoc-spec/list_propertysystem.html#currentghost.scope-ID-.currentmonitor.rect` ほか。
- `OnDisplayChange`・`OnDisplayChangeEx`（モニターの構成が変わった知らせ）は無い（`shiori.toml` で `absent`・持ち主なし・C2）。台本は必須としていない。

## Desired Outcome

- `currentghost.scope(ID).currentmonitor.rect`・`.work`・`.primary`・`.bpp` が、正典の形で値を返す（`\![get,property,…]`・MCP の `get_property`・SHIORI からの読みのどの道でも）。
- 値は、聞かれた時の、その窓が居るモニターの物。窓が別のモニターへ移れば、次に聞いた時に替わる。
- 座標の単位と原点は、動いている `currentghost.scope(ID).rect` と同じ（2 つを比べられる）。
- 窓の無いスコープは、今の `rect`・`dpi` と同じ扱い。
- 台本「席替え」が、モニターの範囲と作業領域を使って席を決められる。

## Approach

- **小さい spec に切り出す**（2026-11-19 開発者の承認）。前例は `mcp-scope-rect-property` と `shell-scale-linkage`（使う者が現れた葉だけを先に出した）。
- **聞かれた時に取りに行く**（プル）。窓の矩形と同じで、変わった時点が分からない値なので、押し出さない（「変わった時点が分かるならプッシュ・分からないならプル」の決まり）。毎フレーム比べない。
- 一次情報源から取る（wintf のモニターの情報。無ければ OS へ聞く）。
- 4 葉をまとめて出す。読み手の型が同じなので、`rect` と `work` だけに絞っても手間はほとんど変わらない。

## Scope

- **In**: `currentmonitor.rect`・`.work`・`.primary`・`.bpp` の 4 葉の読み手と登記・スコープの形の 4 つ（今の `rect`・`dpi` と同じ）・窓の無いスコープの扱い・網羅台帳の 4 行の更新（状況と持ち主）・`doc/COMPAT_ARCHITECTURE.md` の行・決定論のテスト。
- **Out**: `OnDisplayChange`・`OnDisplayChangeEx`（起票しない。タスクバーを避ける用途は `.work` で足りる見込み。roadmap の覚え書きに残す）・`currentghost.*` のほかの葉（`currentghost-property-tree`・`currentghost-property-others`）・書き込み（正典に SET の定めが無い）・`system.*` のモニターの一覧（`system-property-values`）。

## Boundary Candidates

- 4 葉の読み手（`crates/areka/src/` の `ghost_session_scope_*.rs` の並び）。
- 窓からモニターを引く所（wintf のモニターの情報）。
- 名前の登記（`crates/areka-sylphya/src/vocab/`）。

## Out of Boundary

- プロパティの仕組みそのもの（読む道・名前の解き方）は変えない。
- 窓の配置の決まりは変えない。

## Upstream / Downstream

- **Upstream**: 完了済みの `mcp-scope-rect-property`・`shell-scale-linkage`（読み手の型）。待つ物は無い。
- **Downstream**: 台本「席替え」・`currentghost-property-others`（残りの葉が 4 つ減る）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `currentghost-property-tree`（網羅台帳の上の今の持ち主・動く値を出す口）・`currentghost-property-others`・`system-property-values`・`window-timed-directives`（席替えの台本が一緒に使う）。

## Constraints

- 区分 A（開発者が起票を頼んだ）・v0.4.0（一緒に使う `window-timed-directives` と同じ段。待つ物が無いので、席が空けば前に出してよい）。
- 規模の見立て: S（5〜8 タスク）。Opus。
- 意味は ukadoc から輸入する。座標の単位（拡大の前か後か・仮想画面の原点）は、動いている `currentghost.scope(ID).rect` に合わせ、食い違うなら SSP の MCP で測る。
- プロパティの動く値の列（`crates/areka-sylphya/src/vocab/` ほか）の spec と同時に走らせない。
- 台本「席替え」の本文は ghost_dev のリポジトリの物で、areka のリポジトリへ写さない。
