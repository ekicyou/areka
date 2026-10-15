# Brief: areka-P0-window-snap

> **夢の段・覚書**（2026-10-15 `/kiro-discovery`）。開発者の指示は「夢仕様だけど起票お願い」。形は決めていない。下の「推す形」と「議題」は、着手のときの要件の段で覆してよい。
>
> 出どころ（2026-10-15 開発者・要約）: マスコットがデスクトップの床やほかのアプリの窓に吸着（スナップ）する「スナップ API」を足したい。挙動はシェルの定義（YAML などの宣言的な定義）だけで書けるようにする。マスコットが環境のどこに吸着しているか（環境との関係）を本体の側で抽象化し、シェルへ「コンテキスト」として渡す。シェルは座標を計算せず、コンテキストを見てアニメーションと挙動を切り替える。
>
> - 基本のコンテキスト: 床（作業領域の下端）に立つ／座る・窓の上に立つ／座る・窓の下にぶら下がる・縦の壁（窓の端・画面の端）にへばりつく／登る・壁や窓の陰から顔だけ出す。
> - 将来の考慮: 利用者がドラッグで掴んでいる間・吸着先の窓が動く間のしがみつき（慣性・踏ん張り）・吸着先の窓が閉じた／最小化したときの解除と自由落下・複数のマスコットの衝突と積み重なり（塔）。
> - ECS の像: 吸着先の窓（HWND）を見張って特定する仕組み／シェルの定義の制約（Anchor・Offset）から位置を毎フレーム決める仕組み／吸着の状態に応じて重力などの物理を入り切り・値を変える仕組み。

## Problem

- 今の areka のマスコットは、作業領域の端（既定は下端＝タスクバーの上）にしか居場所が無い。ほかのアプリの窓の上に乗る・ぶら下がる・陰から覗く、といった「デスクトップに住んでいる」感じを作る手段が無い。
- 正典（ukadoc）の吸着は `alignmenttodesktop`（`top`・`bottom`・`left`・`right`・`free`）だけで、相手はデスクトップの端に限られる。ほかの窓は相手にならない。
- シェルの作者が「どこに居るか」で絵や動きを替えたくても、今は台本（SHIORI）が座標を問い合わせて自分で計算するしかない。シェルの定義だけで書く道が無い。

## Current State（main `574bf0a6`・起票時の実測）

### ukadoc（正典）に在るもの

- **デスクトップの端への吸着**: descript の `seriko.alignmenttodesktop`（ゴースト側・シェル側・`sakura.`／`kero.`／`char*.` のスコープ別・優先は「ゴースト全体＜ゴーストのスコープ別＜シェル全体＜シェルのスコープ別」・既定 `bottom`）と台本の `\![set,alignmenttodesktop,方向]`（`top`・`bottom`・`left`・`right`・`free`・`default`）。上下に吸着すると上下へのドラッグができなくなり、左右も同様。
- **窓どうしをくっつける**: `\![set,sticky-window,スコープID,…]`・`\![reset,sticky-window]`・descript の `seriko.sticky-window`・プロパティ `currentghost.seriko.sticky-window`（2.8.78）。ドラッグ中と `move` の実行中に、組にした窓が一緒に動く。
- **基準つきの移動**: `\![move]`／`\![moveasync]` の `--base`（`screen` ほか）・`--base-offset`・`--move-offset`（`left.bottom` のような 9 点）・`--time`。
- **ドラッグのイベント**: `OnMouseDragStart`／`OnMouseDragEnd`。
- **無いもの**: ほかのアプリの窓を相手にする吸着・状態（どこに居るか）をシェルへ渡す仕組み・重力や慣性。

### areka の今

- **吸着の芯は在る**。`crates/areka/src/placement/resolver.rs` の `enum Anchor { Top, Bottom, Left, Right, Free }` が `seriko.alignmenttodesktop` の 4 段の優先（`placement/config.rs`）から決まり、部品 `Anchored(Anchor)`（`placement/follow/anchor.rs`）として窓に付く。`project_anchor` が窓を吸着の辺へ映し直し、ドラッグ（`follow/drag_follow.rs`）・大きさの変化・吸着の変更（`follow/window_move.rs` の `anchor_changed_system`）のたびに当て直す。**相手はどれも作業領域の辺**。
- **作業領域はモニタごと・毎フレーム**。`wintf/src/ecs/window/monitor.rs` の `Monitor.work_area`（`rcWork`）を、`follow/work_area.rs` の `MonitorSnapshot` が毎フレーム作り直し、窓の居るモニタの作業領域を引く。DPI の切替は `placement/dpi_sync.rs`・`placement/chain_realign.rs`。
- **ドラッグは在る**。wintf の `ecs/drag/`（5px のしきい値・追跡・終わりの検出）と areka の `follow/drag_follow.rs`。`OnMouseDragStart`／`OnMouseDragEnd` は送っている（`areka-kanade/src/schedule/events.rs`）。
- **ほかのアプリの窓を見張るコードは無い**。`EnumWindows`・`SetWinEventHook` はどこにも無い。wintf の `api.rs` に上下の窓を引く `get_window_above/below`・`get_foreground_window`（読むだけ）が在るだけ。
- **物理は無い**。重力・速さ・慣性の部品は無い（`CursorVelocity` はポインタの速さ）。
- **時間をかける移動も動いていない**。`\![move]` は時間を読むが最終位置へ即時に置く（持ち主は `sakura-time-directives` だが範囲から外した D 群）。`\![moveasync]`・`\![set,alignmenttodesktop]`・`\![set,sticky-window]`・`\4`・`\5` は網羅台帳で absent。
- **3 人目からの窓は無い**（`emo2_boot/mod.rs` の `derive_scopes()` が 0 と 1 だけ・持ち主は `extra-character-windows`）。ほかのゴーストを同時に出す仕組み（多重ゴースト）は roadmap の「予約」。
- **シェルの定義は ukadoc のテキスト形式だけ**（`areka-parsers/src/shell/` が surfaces.txt、`kv/` が descript）。YAML のシェルの定義は無い（`dola-yaml-backend` は dola の任意機能の YAML の依存を外す話で、シェルの定義とは無関係）。
- **起票時に見つけた台帳の古い行**: 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` は descript の `seriko.alignmenttodesktop` の行（8 行）を `status = "absent"`・持ち主なしと書くが、コードは読んで使っている（上の `config.rs`・`Anchor`）。直すのは網羅台帳を整える spec（`coverage-roadmap-refresh`）か、本 spec の着手のとき。

## Desired Outcome

- シェルの作者が、シェルの定義に「吸着の先ごとの絵や動き」を書くだけで、マスコットが床・窓の上・窓の下・壁・窓の陰に居るときの姿を出し分けられる（座標の計算を書かない）。
- 本体が「今どこに、何に、どう吸着しているか」を 1 つの語彙（コンテキスト）で持ち、シェルと台本の両方が同じ語彙で読める。
- 吸着の先の窓が動けば付いて行き、閉じる・最小化する・別の仮想デスクトップへ消えるときは、決まった落ち方（自由落下で床へ）に移る。
- 書かなければ今までどおり（`alignmenttodesktop` の正典どおり）。SSP で同じシェルを読んでも壊れない。

## Approach

**推す形**（覚書・要件の段で覆してよい）: 今ある `Anchor`（`alignmenttodesktop`）を「吸着の相手」と「吸着の仕方」の 2 つに分けて一般化し、相手に「ほかの窓」を足す。小さい順に段を踏み、着手のとき最初の 1〜2 段だけを本 spec の範囲に切る（後ろの段は先に起票しない）。

1. **コンテキストの語彙と渡し方（相手はデスクトップだけ）**: 今の `Anchor` の上に「床に居る（下端）・天井（上端）・左右の壁」をコンテキストとして名付け、シェルと台本へ渡す。渡し方は議題 2。ここだけで「画面の端を登る」「天井にぶら下がる」が書ける。
2. **ほかの窓を見張る**: OS の受け口（`SetWinEventHook` の `WINEVENT_OUTOFCONTEXT` で、窓の移動・大きさ・最小化・破棄・前後の入れ替え・隠れ〔cloak〕を受ける）で吸着の先の窓を追う。窓の見た目の枠は `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` で取る（Windows 10/11 の窓は `GetWindowRect` の外側に見えないリサイズの縁がある＝そのまま乗ると数ピクセル浮く）。吸着の先を決めるのはドラッグを離したとき（下に在る窓の上の辺へ乗る）を推す。
3. **窓に吸着する**: 窓の上に立つ・下にぶら下がる・左右の辺にへばりつく。吸着の先が動けば同じフレームで付いて行く（1 フレーム遅らせない）。閉じる・最小化・隠れたら解除して床へ。
4. **落ちる・投げる**（物理）: 自由落下・投げたときの慣性・着地。値は「始まりからの経過時間」の純粋な関数で決める（時刻を注入して決定論で検査できる形）。時間をかける移動の土台（`\![move]` の時間・`\![moveasync]`）と式（`element-animation-easing` の語彙）を分け合う。
5. **窓の陰から覗く**: 自分の窓を吸着の先の窓の 1 つ後ろへ置き（重なりの順）、体の一部だけが見える位置へ寄せる。今の重なりの順の仕組み（`zorder_chain*`）と、ほかのアプリの窓との前後の決まりが議題。
6. **積み重なり**: 複数のマスコット（同じゴーストのスコープどうし・ほかのゴースト）の上に乗る。正典の `sticky-window` と同じ「組で動く」に乗せられるか。3 人目からの窓（`extra-character-windows`）と多重ゴースト（予約）が先。

**採らない方向（起票時）**:

- 物理エンジンのクレートを入れる（剛体の衝突の計算までは要らない・落下と着地は数式で足りる・決定論の検査と「時刻は正確に扱う」原則に合わせる）。
- ほかのアプリの窓を動かす・閉じる（マスコットが相手の窓を押す・投げる遊びは範囲外＝利用者の窓に手を出さない）。
- 吸着の先の窓をポーリングで見張る（OS の通知で受ける・待機中の負荷を増やさない）。
- 窓の中身（ほかのアプリの描いた絵）を読む・写す。

## Scope

- **In**（夢・着手のときに切る）: コンテキストの語彙と状態の持ち方／シェルと台本への渡し方／ほかの窓の見張り／窓への吸着と解除／落下と着地／覗き／網羅台帳と対応表（`doc/COMPAT_ARCHITECTURE.md`）への areka 独自の書き方の登記。
- **Out**: `\![set,alignmenttodesktop]`・`\![set,sticky-window]` の正典どおりの実装そのもの（本 spec が土台として引き取るかは議題 1）／3 人目からの窓（`extra-character-windows`）／多重ゴースト（予約）／最小化（`tasktray-minimize`）／ほかのアプリの窓を動かすこと／設定画面。

## Boundary Candidates

- コンテキストの語彙と状態（純粋なデータ・どの部品が持つか）。
- シェルの定義の読み手（吸着の先ごとの絵と動きの書き方・`areka-parsers` の `shell/` か `kv/`）。
- コンテキストをシェル（SERIKO・サーフェスの切替）と台本（イベント・プロパティ）へ渡す口。
- ほかの窓の見張り（wintf の OS の受け口・見張る窓を絞る）。
- 吸着の位置の決め方（今の `project_anchor` の一般化・`placement/follow/`）。
- 落下と着地（時刻 → 位置の純粋な関数）。
- 重なりの順（覗き・積み重なり・`zorder_chain*`）。

## Out of Boundary

- ukadoc の語の意味を変えること（書かなければ `alignmenttodesktop` の正典どおり）。
- SSP の動きの実測を写すこと（意味は ukadoc から取る・areka 独自の所は対応表に記録する）。
- ほかのアプリの窓の位置・中身への書き込み。

## Upstream / Downstream

- **Upstream**: `placement/`（`Anchor`・`Anchored`・`project_anchor`・`MonitorSnapshot`・DPI の同期）／wintf の `ecs/drag/`・`ecs/window/monitor.rs`・`zorder_chain*`／`OnMouseDragStart`／`OnMouseDragEnd`（kanade）／時間をかける移動の土台（`sakura-time-directives` から戻した D 群）／`element-animation-easing`（落ち方・着地の形）。
- **Downstream**: 多重ゴースト（予約）の積み重なり・ゴーストどうしの掛け合い／`extra-character-windows` の 3 人目からの吸着。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。
- **Adjacent**:
  - `extra-character-windows` — スコープ 2 からの窓・`placement/` と `input_events/` を分け合う。
  - `dpi-realign-remembered-chain` — DPI の切替の後に並べ直す所（`chain_realign.rs`）を分け合う。
  - `zorder-property`・`zorder-chain-residue` — 重なりの順と `sticky-window` のプロパティ。
  - `tasktray-minimize` — `\![set,windowstate,minimize]` と最小化のイベント（自分の窓の最小化・吸着の解除と関わる）。
  - `surfaces-basepos` — `\![move]` の座標の式（`move_cue.rs`）を分け合う。
  - `element-animation-easing` — 時間をかける指令の土台の持ち主の議題が同じ（議題 1 は両 spec で一緒に決める）。
  - `collisionex-regions` — ドラッグのイベントの当たり判定の識別子。

## Constraints

- **時刻は正確に扱う**（設計の大原則）: 落下・付いて行く動きは経過時間の純粋な関数。1 フレーム遅らせる作りは採らない（吸着の先が動いたフレームに付いて行く）。
- **決定論のテスト**: 時刻とほかの窓の位置を注入して検査できる形（偽の境界で x64 のまま）。
- **OS の受け口を最大限使う**（開発者の方針）: 窓の見張りは WinEvent、見た目の枠は DWM、隠れは `DWMWA_CLOAKED`。
- **負荷**: `EVENT_OBJECT_LOCATIONCHANGE` は文字のカーソルやポインタでも大量に届く＝窓（`OBJID_WINDOW`）と見張る窓だけに絞る。待機中の CPU（`tick-gate-adoption` の見込み）と相談する。
- **権限の境界**: 管理者で動くアプリの窓の通知を受けられるか、乗れるか（UIPI）は着手のときに確かめる（`elevated-drop-filter` と同じ種類の線）。
- **正典を壊さない**: 書かなければ今までどおり。SSP が読める書き方（知らないキーを SSP がどう扱うかは ukadoc で確かめる・実測の写しはしない）。
- **名付けの流儀**: 定義ファイルは ukadoc の用語で名付ける（「surface*ブレス」「element定義」の型）。

## 議題（着手のときに開発者へ・答えで作業が変わるもの）

1. **土台の持ち主**: 正典の `\![set,alignmenttodesktop]`・`\![set,sticky-window]`・時間をかける `\![move]`／`\![moveasync]` はどれも未実装。本 spec が正典どおりの実装ごと引き取るか、先に別の spec（網羅台帳が「窓の配置と重なり」としてまとめる候補に挙げている）を立てて本 spec はその上に乗るか。
2. **コンテキストの渡し方**: ⑴ シェルの定義に「コンテキストごとのサーフェス／アニメーション」を書く（台本は要らない・シェルの作者だけで済む）⑵ SHIORI へイベントで知らせて台本が切り替える ⑶ プロパティ（`currentghost.scope(ID).…`）で読めるようにする。⑴ を主に、⑶ を添えるのを推す（⑵ は新しいイベントを作ることになる）。
3. **シェルの定義の形**: 開発者の言う「YAML 等による宣言的定義」を、新しい YAML のファイルにするか、ukadoc の descript／surfaces.txt に areka 独自のキー（ブレス）を足すか。YAML にするなら読み手の依存を選ぶ（`serde_yaml` は勧告 `RUSTSEC-2024-0449` で `dola-yaml-backend` が外そうとしている）。
4. **吸着の先の選び方**: ドラッグを離した所の下の窓だけか、マスコットが自分で歩いて乗り移るか（自分で動くなら「どの窓へ行くか」を誰が決めるか＝本体か台本か）。
5. **どの窓を相手にするか**: 最大化した窓・全画面のアプリ・ツールの窓・タスクバー・ほかの areka のマスコットの扱い。
6. **覗きの重なり**: 自分の窓をほかのアプリの窓の後ろへ置くと、そのアプリが前へ出たとき・利用者が別の窓を触ったときにどうなるか（隠れたままにするか、床へ戻すか）。
7. **複数のモニタ**: 窓がモニタをまたぐとき・吸着の先が別のモニタへ移ったとき。
