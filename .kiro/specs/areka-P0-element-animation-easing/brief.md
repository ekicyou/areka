# Brief: areka-P0-element-animation-easing

> **覚書の段**（2026-10-12 `/kiro-discovery`）。開発者の指示は「まずは覚書レベルで、スペックレベルは夢よりは優先度高い程度で起票」。形は決めていない。下の「推す形」と「議題」は、着手のときの要件の段で覆してよい。
>
> 出どころ（2026-10-12 開発者）: 「イージング良いよね https://motion-reference.takayustudio.jp/ css イージングの範疇で、エレメントアニメーションの記述や、さくらスクリプトからエレメントにイージング指示が出せたらよいのにと思う。プロパティシステムとも連携する必要があるか？現在、ukadoc でエレメントレベルの情報を取る手段あるのかしら。」

## Problem

- シェルの作者が滑らかな動きを作るには、今はコマを 1 枚ずつ並べるしかない。ukadoc の SERIKO は「コマを待ち時間つきで切り替える」仕組みで、コマとコマの間をつなぐ書き方が無い。動かすだけの `move` も「瞬間的なもので、連続的な移動ではない」と定義されている。
- ゴーストの作者が台本から動きを付けるときも、時間をかける指令（`\![move,…,--time]`・`\![set,alpha,…,--time]`・`\![set,scaling,…,--time]`）は変化の速さの形を選べない（正典に形の指定が無い）。
- 開発者が挙げたサイト（個人学習用のモーションの見本集・デモ 782 件・実装例は HTML／CSS／WAAPI／GSAP ほか・`easeOutCubic`・`easeInOutCubic`・`easeOutBack` などの名前で引ける）のような「速さの形で感触を作る」表現を、areka のシェルと台本で使えるようにしたい。

## Current State

### ukadoc（正典）に在るもの・無いもの

- **イージングはどこにも無い**（`ease`・補間に当たる語は surfaces.txt にもさくらスクリプトにも無い）。
- pattern定義: `animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X座標,Y座標(,オプション...)`。1 コマが持つのは待ち時間と位置だけ。**末尾のオプションは正典に在る**（今は `--alpha,値(0-255)` の 1 つ・SSP 2.8.37〜・`--名前,値` をカンマで続ける書き方）。
- 時間をかける台本の指令: `\![move]`／`\![moveasync]`（`--time=ミリ秒`）・`\![set,alpha,数値,--time=,--wait]`・`\![set,scaling,横,縦,--time=,--wait]`（`--wait` は SSP 2.7.52〜）。どれも変化の形は書いていない。
- アニメーションへの台本の指示: `\i[ID]`・`\![anim,clear|pause|resume|stop,ID]`・`\![anim,offset,ID,x,y]`（即時にずらす）・`\![anim,add,overlay,ID,x,y,表示時間,…]`（コマの列を台本から足す）・`\__w[animation,ID]`（終わるまで待つ）。
- **element 単位の情報を取る手段は無い**。プロパティシステムが届くのはスコープまで: `currentghost.scope(ID).surface.num`（読み書き）・`.animation.num`（動いているアニメーション ID の一覧・書くと追加で実行・2.8.28）・`.x`／`.y`／`.rect`・`.scaling`（実効の拡大率・2.7.89）・`.surface(ID).rect`（2.8.57）・`.seriko.defaultsurface`。element定義は「複数の画像を合成し、その合成結果をベースサーフェスとして一枚の画像のように扱う仕組み」で、実行時に element を名指しする語が正典に無い。**台本とプロパティから指せる一番細かい単位はアニメーション ID**（と `animation*.name` の別名）。

### areka の今（main `d0c697d6`）

- **式は dola に在るが、CSS の形ではない**。`crates/dola/src/easing.rs` の `EasingFunction`＝名前つき 31 語（Linear＋Quadratic／Cubic／…／Back／Bounce の In・Out・InOut）とベジェ（`QuadraticBezier`・`CubicBezier`）。適用は `crates/dola/src/runtime/interpolator/mod.rs` の `apply_easing`（`interpolation` クレート）。ここの `CubicBezier{x0..x3}` は 4 つの値を t で評価する 1 次元のベジェで、**CSS の `cubic-bezier(x1,y1,x2,y2)`（x から t を解いて y を返す）とは別物**。`steps`・`linear()` に当たる型は無い。
- **dola の連続値の側は本番に結線されていない**。`crates/wintf/src/ecs/dola/mod.rs` の `DolaAnimator`・`tick_dola_animators` は定義とテストだけ。areka が dola から使っているのは台本の配信（`dola::cue`）と時計だけ。cue は点のイベントで、区間のあいだ値を変え続ける指令を持たない。
- **時間をかける台本の指令はどれも動いていない**。`\![move]` は第 3 引数の時間を読むが最終位置へ即時に置く（`crates/areka/src/emo2_boot/move_cue.rs` の `TimedMoveImmediate`）。`--time=` の名前つきの形は読み飛ばす。`\![set,alpha]`・`\![set,scaling]`・`\![moveasync]`・`\![anim,…]` は受け取り手が無い（網羅台帳 `sakura-script.toml` で absent）。
- **これらの持ち主が居ない**。台帳の上では `\![move]` の時間・`\![set,alpha]`・`\![set,scaling]` の持ち主は `sakura-time-directives` だが、同 spec は棚卸㉑でこれら（D 群）を範囲から外し、roadmap の覚え書きへ戻した（「消費する者が現れるまで置く」）。`\![moveasync]`・`\![anim,…]` は持ち主の欄が空。
- **SERIKO はコマ送りだけ**。`crates/areka-seriko/src/timeline.rs` の `frame_at` が「今のコマ 1 枚」を返し、合成へは整数の位置の `PatternFrame` を渡す。pattern定義の 7 欄目より後（末尾のオプション）は読み手が見ていない（`crates/areka-parsers/src/shell/decode.rs` の `decode_animations`）＝正典の `--alpha` も未対応。
- **element は実行時に指せない**。ECS の実体はサーフェス単位で、element は配列の要素。表示は 1 枚に焼いた後。変換の型（`crates/areka-emo-compose/src/normalized.rs` の `Transform`）は整数の平行移動だけが動き、回転・拡大は予約の口、不透明度の欄は無い。コマが変わるたびに 1 枚を焼き直す作り。
- **窓の単位なら毎フレーム変えられる**。wintf の `Visual`（不透明度）と `Arrangement`（位置・拡大）を `visual_property_sync_system` が毎フレーム書き込む。回転の欄は無い。
- **プロパティ（sylphya）は掲示板の形**。全体を差し替えて公開し、読む側は都度読むかトークの開始で凍結する（購読も通知も無い）。語彙に `surface.num`・`animation.num` は在るが `currentghost.*` の実値はまだ 0 件（`currentghost-property-tree` が最初）。element・alpha・scaling の語は無い。

### 開発者の 2 つの問いへの答え（起票時の見立て）

- **プロパティシステムとの連携は要るか** → 最初の段には要らない。イージングは「値をどう変えるか」の指定で、値そのものではないので、台本の指令と定義ファイルの書き方に載せれば足りる。プロパティは毎フレーム動く値を流す道ではない（掲示板の形）。関わるのは 2 か所だけ: ⑴ 変化の途中で `currentghost.scope(ID).scaling` などを読んだら途中の値を返す（持ち主は `currentghost-property-others`）、⑵ `animation.num` に書いて始めたアニメーションにも同じ形が効く（定義ファイルの側に書いてあれば自動で効く）。
- **ukadoc で element 単位の情報を取る手段は在るか** → 無い（上の節）。element を指して動かすなら、指し方（名前か番号か）から areka が決めることになる。

## Desired Outcome

- シェルの作者が、定義ファイルに速さの形を 1 語足すだけで、少ないコマで滑らかな動きを書ける。
- ゴーストの作者が、台本の時間をかける指令に速さの形を添えられる。
- 速さの形の語彙は CSS のイージング関数と同じ意味で読める（Web の見本集やカーブの編集の道具で作った値がそのまま使える）。
- 書かなければ今までどおり（正典どおりのコマ送り・正典どおりの指令）。SSP で同じシェルを読んでも壊れない書き方にする。

## Approach

**推す形**（覚書・要件の段で覆してよい）: 小さい順に段を踏み、着手のとき最初の 1〜2 段だけを本 spec の範囲に切る（後ろの段は先に起票しない）。

1. **語彙と式を 1 か所に置く**: CSS の範囲＝`linear`・`ease`・`ease-in`・`ease-out`・`ease-in-out`・`cubic-bezier(x1,y1,x2,y2)`・`steps(n,位置)`（`step-start`・`step-end`）・`linear(点の列)`。置き場は dola の `easing`（今の式の隣に CSS の意味の 3 つを足す）が素直。文字列 → 型の読み手も 1 つだけ。
2. **台本の時間つきの指令に添える**（窓の単位）: `\![set,alpha]`・`\![set,scaling]`・`\![move]`／`\![moveasync]` の `--time` に並べて `--easing=…`。窓の単位は毎フレーム書き込む口がもう在るので、element に手を入れずに済む。ただし土台の時間つきの指令そのものが未実装で持ち主が居ない（議題 1）。
3. **pattern定義のコマ間をつなぐ**: 末尾のオプション（正典の `--alpha,値` と同じ書き方）で、前のコマの位置・不透明度から今のコマの値へ、待ち時間をかけて変える。合成が「コマごとに 1 枚を焼き直す」作りなので、動かす部品を焼かずに別の層で持つ変更が要る。
4. **element を名指しして動かす**: element の指し方（名前）・element 単位の位置・不透明度・拡大・回転・台本からの指示。正典に足場が無く、一番大きい。

**採らない方向（起票時）**:

- 画像や部品ごとに独立の時計を持つ（アニメーションのエンジンが 3 つ目になる・`animated-image-*` の起票のときに採らなかった）。
- Windows のアニメーションの仕組み（Composition の KeyFrameAnimation・UIAnimation）に補間を任せる（時刻を areka が握れず、決定論のテストと「時刻は正確に扱う」原則に合わない）。
- プロパティへ毎フレームの値を流す。
- スプリング（物理）の専用の語。CSS の範囲の外。跳ね返りは `linear()` で近似できる。

## Scope

- **In**（覚書・着手のときに切る）: CSS の範囲のイージングの語彙と式／文字列の読み手／最初の使い手（上の段 2 か段 3 のどちらか 1 つ）／網羅台帳と対応表（`doc/COMPAT_ARCHITECTURE.md`）への areka 独自の書き方の登記。
- **Out**: 文字の現れ方（`text-reveal-fade`・`text-reveal-dance`）／残りの描画メソッド（`draw-methods-canon`・`draw-methods-blend`）／SERIKO の起動のきっかけ（`seriko-script-triggers`・`seriko-interval-combinations`）／`currentghost.*` の値（`currentghost-property-*`）／スプリング・パスに沿う動き・キーフレームを 3 つ以上つなぐ書き方（CSS の `@keyframes` に当たるもの）。

## Boundary Candidates

- イージングの語彙と式（純粋な関数・dola）。
- 文字列の読み手（定義ファイルと台本で同じ 1 つを使う）。
- 台本の時間つきの指令の実行（窓の単位・`emo2_boot` と wintf の書き込み口）。
- pattern定義の末尾オプションの読み手（`shell/decode.rs`・正典の `--alpha` もここ）。
- コマ間の補間（seriko の時刻 → 値）と、焼かずに動かす層（emo-compose・emo-present）。
- element の指し方と element 単位の変換。

## Out of Boundary

- ukadoc の語の意味を変えること（書かないときの動きは正典どおり）。
- SSP の動きの実測を写すこと（意味は ukadoc から取る・areka 独自の所は対応表に記録する）。
- 設定画面。

## Upstream / Downstream

- **Upstream**: dola（`easing`・`interpolator`・`interpolation` クレート 0.3.0）／wintf の `Visual`・`Arrangement`／seriko の `timeline`／台本のコンパイル（`CueCommand::Custom` の運び手・`--key=value` は素通し）／受け取り手の名簿（`consumer_ledger.rs`）。
- **Downstream**: `text-reveal-dance`（字の跳ね・揺れが同じ語彙を使える）／`text-reveal-fade`（フェードの形を選べる）／`currentghost-property-others`（途中の値を返す）／将来の「element を動かす」仕事。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。
- **Adjacent**:
  - `sakura-time-directives` — 台帳の上で `\![move]` の時間・`\![set,alpha]`・`\![set,scaling]` の持ち主のまま範囲から外している（D 群）。本 spec がこれらを引き取るなら、台帳の持ち主を付け替える。
  - `seriko-script-triggers` — `\![anim,…]` を「次の spec」として範囲の外に置いている。`animation*.name` の読み取りはここ。
  - `text-reveal-fade` — 「透明度を時間で変える」最初の実装。式の置き場を先に決めた側に、後の側が合わせる。
  - `draw-methods-canon`・`element-clipping-option`・`animated-image-import` — pattern定義・element定義の読み手（`shell/{model,decode}.rs`）と合成の段を分け合う＝シェルの element の列で直列。
  - `check-script-arg-checks` — 新しい `\!` の引数（`--easing`）が検査の対象になる。
  - `surfaces-basepos` — `move_cue.rs` を分け合う。

## Constraints

- **時刻は正確に扱う**（設計の大原則）: 値は「始まりからの経過時間」の純粋な関数で決める。更新が遅れたら過ぎた分だけ進める。1 フレーム遅らせる作りは採らない。
- **決定論のテスト**: 時刻を注入して値を検査できる形にする。
- **正典を壊さない**: 書かなければコマ送り。SSP が読める書き方（知らないオプションを SSP がどう扱うかは ukadoc で確かめる。実測の写しはしない）。
- **独自の書き方の流儀**: 台本は `\!` に入れ、キーは CSS のプロパティ名の `-` を `_` にした名前（roadmap「テーマ別の決めごと」の文字組みの決め）。定義ファイルは「animation*.pattern*のオプション」のように ukadoc の用語で名付ける。
- **負荷**: ずっと動き続ける演出は起床を止めない＝待機中の CPU（`tick-gate-adoption` の見込み）と相談する。
- **はみ出す値**: `easeOutBack` や `cubic-bezier` の y が 0〜1 の外へ出る形は、位置と拡大では行き過ぎて戻る動きになる。不透明度は 0〜100 に丸める。

## 議題（着手のときに開発者へ・答えで作業が変わるもの）

1. **土台の持ち主**: 時間つきの台本の指令（`\![set,alpha]`・`\![set,scaling]`・`\![move]` の時間・`\![moveasync]`）は未実装で持ち主が居ない。本 spec が線形の実装ごと引き取るか、先に別の spec として立てて本 spec は「形を添える」だけにするか。
2. **最初の使い手**: 台本（窓の単位・小さい）から始めるか、pattern定義（シェルの作者に効く・合成の作りを変える）から始めるか。
3. **カンマの衝突**: `cubic-bezier(0.2,0,0,1)` はカンマを含む。台本は `\!` の引数の二重引用符で包めるが、pattern定義の末尾のオプションは正典が `--名前,値` のカンマ区切りなので、同じ書き方では値が割れる。値を空白区切りにする・別の区切りを決める・名前つきの形だけにする、のどれにするか。
4. **名前の語彙**: CSS の 5 語だけにするか、見本集で通じる名前（`easeOutCubic`・`easeOutBack` など＝dola が持つ 31 語）も受けるか。
5. **element の指し方**: 段 4 へ進むなら、名前を定義ファイルのどこに書くか（正典に無い）。進まないなら、指せる最小の単位はアニメーション ID のまま。
6. **回転**: 窓にも element にも回転の口が無い。イージングの対象に回転を入れるか。
