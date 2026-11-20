# 暫定の設計: element-animation-easing

> **作り直し済み（2026-11-20）**: この暫定の設計は、開発者の決め（`doc/ideas/novel-decisions-2026-11-20.md`）の前の物である。決めで作り直した物は `doc/ideas/novel-redesign-2026-11-20/`（骨組みは `00-architecture.md`）。この文書は、確かめた事実（`path:行`）の控えとして残す。

## 1. 前置き

- **暫定の設計である。** 正式な `design.md` ではない。要件定義（`requirements.md`）はまだ無く、下の「暫定の要件」は brief と pasta の骨子・取り決めから起こした仮の物である。
- 日付: 2026-11-19。基にした main: `49b6f3ab5`。
- 目的: ノベル（演劇）の「転換」（値を時間をかけて変える・絵を溶かす）の土台を、どの単位が出すかを大枠で決め、上位の決定とぶつかる所を先に見つける。細部は磨いていない。
- コードは読んだだけである。ビルド・テスト・起動はしていない。
- 読んだ物:
  - この単位の brief（`.kiro/specs/element-animation-easing/brief.md`）。
  - 隣の brief: `window-timed-directives`・`shell-multi-compose`・`parallel-visual-script`・`shell-hover-animation`・`seriko-script-triggers`（`\i` の所）・`dola-interrupt-ticks`・`talk-anchor-interrupt-ticks`・`talk-fast-forward`・`text-reveal-fade`（Desired Outcome と Approach）。
  - pasta: 骨子 `final-model-2026-11-19.md`（全部）・取り決めの brief の「決定（2026-11-19。最終形）」の節（199〜349 行）・`play-script-guide.md` の 3 章（100〜143 行）・`discovery-2026-11-19.md` と `research-ssp-capabilities.md` は `--time` の行だけ。
  - areka の側の見立て: `areka-novel-fade-start-point-2026-11-19.md`・`areka-novel-fade-and-parallel-2026-11-19.md`。
  - roadmap（`.kiro/steering/roadmap.md`）の 41・425・426・540・1165・1734・1980 行。網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\i`・`\![anim,…]`・`\![move]`・`\![set,alpha]` の行。
  - コード: 3 章に `path:行` で挙げた所。
- 言葉: この文書の「転換」は pasta の取り決めの `\![play,trans,ミリ秒]` を指す。dola の型 `TransitionDef` は「dola のトランジション」と書いて区別する。

## 2. 暫定の要件

出どころの略: 〔B〕＝この単位の brief、〔取〕＝pasta の取り決めの brief の「決定（最終形）」、〔骨〕＝骨子、〔W〕＝`window-timed-directives` の brief、〔共〕＝areka の決まり。

| 番号 | 要件 | 出どころ |
| ---- | ---- | -------- |
| R1 | 速さの形を表す文字列を読むとき、読み手は `linear`・`ease`・`ease-in`・`ease-out`・`ease-in-out`・`step-start`・`step-end`・`cubic-bezier(x1,y1,x2,y2)`・`steps(n,位置)`・`linear(点の列)` を、CSS のイージング関数と同じ意味で受ける | 〔B〕Approach 1・Desired Outcome 3 |
| R2 | 読み手は、見本集で通じる 30 語（`easeInSine`〜`easeInOutBounce`）も受け、dola が持つ名前つきの式へ写す（仮定 A4） | 〔B〕議題 4 |
| R3 | `cubic-bezier` は、進み具合を x として t を解き、y を返す。進み具合 0 は 0、1 は 1 を返す。同じ入力には、いつも同じ値を返す | 〔B〕Current State・Constraints「決定論のテスト」 |
| R4 | 形の語彙・式・文字列の読み手は、1 か所（dola の `easing`）にだけ置く。定義ファイルと台本は、同じ読み手を使う | 〔B〕Approach 1・Boundary Candidates |
| R5 | 形の文字列が読めないとき、areka はトークを止めず、等速に置き換えて、記録を 1 行出す | 〔共〕失敗を黙って捨てない・〔W〕Desired Outcome |
| R6 | 台本の `\![set,alpha]`・`\![set,scaling]`・`\![move]`・`\![moveasync]` に `--time` が在り `--easing=形` が添えてあるとき、areka はその形で値を変える | 〔B〕Approach 2・議題 1 の決着 |
| R7 | `--easing` が書いていないとき、値の変わり方は `window-timed-directives` の等速と、どの時刻でも同じ値である | 〔B〕Desired Outcome 4・Constraints「正典を壊さない」 |
| R8 | 時間をかけて変わる値は、「始まりの時刻からの経過」と長さと形だけで決まる。更新が遅れたら、過ぎたぶんだけ進んだ値を出す。1 フレーム遅らせる作りにしない | 〔B〕Constraints・〔共〕 |
| R9 | ある値が変わっている途中に、同じ値への次の時間つきの変更が来たら、areka は、その時刻の途中の値を出発点にして、新しい長さと形で続ける | 〔取〕転換「今、画面に出ている絵（転換の途中なら、途中の絵）が出発点になる」・〔W〕Approach |
| R10 | 字の早送りと出し直しのとき、areka は、走っている時間つきの変化を、同じフレームで終わりの値にする | 〔取〕転換「クリックで字を早送りしたら、走っている転換も、新しい絵へ飛ぶ」 |
| R11 | 走っている変化と同じ終わりの値をもう一度受けたら、areka は何もしない（やり直さない） | 〔取〕命令の表「同じ値をもう一度受けたら何もしない」 |
| R12 | 時間つきの変化は、`\x` や選択肢で台本が待っている間も進み、さくらスクリプト 1 本の終わりをまたいで続く | 〔取〕転換「転換は始めるだけで、後ろの字を待たせない」・〔骨〕2.2 |
| R13 | 形が 0〜1 の外へ出るとき、位置と大きさは行き過ぎて戻る。透明度は 0〜100 に収める | 〔B〕Constraints「はみ出す値」 |
| R14 | 時間つきの変化が 1 つも走っていないとき、areka はそのためにフレームを起こさない | 〔B〕Constraints「負荷」 |
| R15 | 値の検査は、時刻を外から渡す決定論のテストで行う | 〔B〕Constraints |
| R16 | `--easing` は、`check_script` の表・網羅台帳・`doc/COMPAT_ARCHITECTURE.md` の「areka が決めた点」に登記する | 〔B〕Scope In・Existing Spec Touchpoints |
| R17 | `--easing` を知らない SSP で同じ台本が壊れないことを、要件の段で ukadoc と SSP の MCP で確かめる（この仕事では確かめていない） | 〔B〕Constraints |
| R18 | （この spec の範囲の外・大枠だけ）pattern定義のコマ間をつなぐ書き方と、element を名指しして動かす書き方は、同じ読み手と同じ時間つきの変化の仕組みを使う | 〔B〕Approach 3・4 |

## 3. 今の作り（確かめた事実）

### 3.1 dola の「連続値の側」は、もう在る（本番に結線されていない）

- 形の型: `crates/dola/src/easing.rs:7`（`EasingFunction`＝`Named(EasingName)` か `Parametric(ParametricEasing)`）。名前は 31 語（`easing.rs:18-50`）。`ParametricEasing::CubicBezier{x0,x1,x2,x3}`（`easing.rs:59`）は 4 つの値を t で評価する 1 次元のベジェで、CSS の `cubic-bezier` ではない（`crates/dola/src/runtime/interpolator/mod.rs:197-204`）。`steps`・`linear()` に当たる型は無い。
- 式の適用: `apply_easing`（`interpolator/mod.rs:131`・見える範囲は `pub(super)`）。進み具合は呼び手が 0〜1 に収める（`mod.rs:84`）。出力は収めない（`mod.rs:195`「オーバーシュート…許容された挙動」）。
- dola のトランジション: `crates/dola/src/transition.rs:27`（`TransitionDef{from, to, relative_to, easing, delay, duration}`）。
- 動かす口: `crates/dola/src/runtime/facade.rs`。`start(name, start_time)`（91 行・開始時刻を渡す）・`conclude(group_id)`（229 行・「最終値ジャンプ終了」）・`cancel(group_id)`（240 行・「現在値凍結破棄」）・`finish(group_id, deadline)`（262 行）・`tick(current_time)`（294 行）・`last_result()`（302 行）・`has_instances()`（309 行）。
- 同じ変数への割り込み: `crates/dola/src/storyboard.rs:48`（`InterruptionPolicy`＝`Cancel`・`Conclude`・`Trim`・`Compress`・`Never`）。`Cancel`／`Trim` は、古い側を**新しい側の開始時刻で評価**した値で止める（`crates/dola/src/runtime/conflict_resolver.rs:73-74`）。
- `from` を省いたときの値: コンパイルの時に、同じストーリーボードの中の直前の区間の `to`、無ければ変数の初期値（`crates/dola/src/compile/resolve.rs:357-377`・`compile/mod.rs:286-292`）。**走っている値（実行時の今の値）を取る所は、読んだ範囲には無い**（`transition.rs:29` の注釈は「省略時は配置時点の変数の現在値」）。
- 始められるのは、読み込んだ文書（`load_document`・`facade.rs:71`）に名前で載っているストーリーボードだけである。
- ECS の入れ物と毎フレームの係: `crates/wintf/src/ecs/dola/mod.rs:75`（`DolaAnimator`）・`mod.rs:187`（`tick_dola_animators`。`FrameTime` で全部を刻み、走っていれば起床の旗 `ANIM` を立てる）。
- **本番の結線は 0**: `tick_dola_animators` と `DolaRuntime` を `crates/` 全体で grep した。当たるのは定義（`wintf/src/ecs/dola/mod.rs`）・再公開（`wintf/src/ecs/mod.rs:19`）・テスト（`wintf/tests/ecs/dola_animator_test.rs`）だけで、areka の crate には 1 件も無い。

### 3.2 時計は、今は 3 本ある

- 台本の時計: `crates/areka-ghost/src/dispatcher.rs:392-399`。台詞の最初の刻みを起点にして、`(now − base)/1000` 秒を送る。`now` は `MonotonicMs`（`crates/areka-kanade/src/msg.rs:30-33`・`GetTickCount64` 相当のミリ秒）。cue は、刻みが発火時刻に着いた時に配られる（`crates/dola/src/cue/runtime.rs:179`）。
- SERIKO の時計: `crates/areka-runtime/src/lib.rs:654-659`（16 ms の刻み）・`lib.rs:676-679`（`GetTickCount64`）。コマは「再生の始まりからの経過」の純関数（`crates/areka-seriko/src/timeline.rs:106` の `frame_at`）。
- フレームの時計: `crates/wintf/src/ecs/graphics/core.rs:330`（`FrameTime(f64)`・`QueryPerformanceCounter` の秒）。
- v0.2.0 の `dola-interrupt-ticks`・`talk-anchor-interrupt-ticks`（どちらも brief だけ）は、3 本を `QueryUnbiasedInterruptTimePrecise` の 1 本の物差しにそろえ、台本の起点を本物の絶対時刻にする、と書いている。

### 3.3 台本と窓の側

- 台本の時刻の表: `crates/areka-sakura/src/compile.rs:141-190`。時刻が進むのは字と待ちだけ。`\![名前,…]` は `CueCommand::Custom` で運び、`--key=value` は素通しである（`crates/dola/src/cue/command.rs:183-188`）。
- 引数の区切り: `\!` の引数は、頭の二重引用符で包める（`crates/areka-parsers/src/sakura/lexer.rs:418-422`）。
- `\![move]` の時間: 読むが、最終位置へすぐ置く（`crates/areka-runtime/src/move_cue_args.rs:74-75` の `TimedMoveImmediate`・`:188-196`）。名前つきの形は `--` と `=` で分ける（`move_cue_args.rs:134`）。
- 受け手の表: `crates/areka-runtime/src/consumer_ledger.rs:235`・`:343`（`("move", None, MoveSink)`）。
- 毎フレーム書ける窓の値: `Visual.opacity`（`crates/wintf/src/ecs/graphics/visual/mod.rs:108-113`）・`Arrangement.offset`／`scale`（`crates/wintf/src/ecs/layout/arrangement.rs:10-14`）。写す係は `visual_property_sync_system`（`crates/wintf/src/ecs/graphics/systems/visual_sync.rs:334`）。回転の欄は無い。
- 倍率の席: `crates/areka-emo-present/src/scale.rs:44-56`（`ScalePolicy.app_scale` は 1 倍のまま）。

### 3.4 SERIKO と合成の側

- pattern定義の読み手は、6 欄（メソッド・サーフェス番号・待ち・X・Y）までを取る（`crates/areka-parsers/src/shell/decode.rs:538-550`・型は `crates/areka-parsers/src/shell/model.rs:155-171`）。末尾のオプションの欄は無い。
- 合成へ渡すコマは整数の位置だけである（`crates/areka-emo-compose/src/pattern.rs:85-94`）。変換の型は平行移動（`crates/areka-emo-compose/src/normalized.rs:18-51`）。
- サーフェスが替わると、SERIKO はその面の再生状態を捨てる（`crates/areka-seriko/src/looper.rs:794-798`）。

### 3.5 `\i[ID]` の持ち主（頼まれた確かめ）

- 網羅台帳では、`\i[ID番号]`（`sakura-script.toml:5256-5260`）も `\i[ID,wait]`（`:5240-5244`）も `status = "absent"`・`owner = ""`。`\__w[animation,ID]`（`:4078-4081`）と `\![anim,…]`（`:579-582` ほか）も同じ。
- brief の上では、`\i[ID番号]` は `seriko-script-triggers` の範囲に入っている（同 brief 52 行・台帳の行は同 spec が完了の時に付ける＝98 行）。`\i[ID,wait]`・`\__w[animation,ID]`・`\![anim,…]` は同 brief が範囲の外に置き（56・57 行）、「次の spec（別途）」としている（75 行）。その spec の brief は無い（`.kiro/specs/*/brief.md` を `i[ID,wait]` で grep して、当たったのは `seriko-script-triggers` だけ）。
- **この単位の持ち物ではない。** pasta の頼みごとは `\i[ID]`（待たない形）だけなので、`seriko-script-triggers`（v0.1.0）が満たす。

## 4. 設計

### 4.1 結論（先に）

1. **「時間をかけて値を変える」仕組みは、新しく作らない。** dola の連続値の側（`DolaRuntime`）と wintf の `DolaAnimator` が、開始時刻・長さ・形・割り込み・終わりへ飛ぶ・起床の旗を、もう持っている。足りないのは、本番の結線と、小さい口 2 つである（4.3）。
2. **3 つの単位は、その 1 つを分け合う。** 持ち分は 4.2 の表のとおり。この単位が出すのは「形」（語彙・式・読み手・`--easing`）で、転換の土台（結線と口）は、最初の使い手である `window-timed-directives` が作る（brief の議題 1 の決着どおり）。
3. **この単位の最初の範囲は、段 1（語彙と式）と段 2（台本の `--easing`）にする**（仮定 A1）。ノベルの転換は、形の引数を持たないので、段 3・段 4 を要らない。

### 4.2 3 つの単位の持ち分（同じ仕組みを別々に作らない形）

| 物 | 置き場 | 作る単位 | 使う単位 |
| -- | ------ | -------- | -------- |
| 値を時間で動かす本体（開始時刻・長さ・形・割り込み・終わりへ飛ぶ） | `crates/dola/src/runtime/`（在る） | 作らない | 3 つとも |
| 毎フレームの係と起床の旗 | `crates/wintf/src/ecs/dola/mod.rs`（在る） | 作らない | 3 つとも |
| 本番の結線（係をスケジュールに載せる・cue の受け手から ECS へ渡す道） | `crates/areka-runtime/` | `window-timed-directives` | 3 つとも |
| 口 1: 「この変数を、その時刻の値から、終わりの値へ、長さと形で」を 1 回で始める | `crates/dola/src/runtime/facade.rs`（足す） | `window-timed-directives` | 3 つとも |
| 口 2: 走っている変化を全部、終わりの値にする | 同上（`conclude` を束ねる） | `window-timed-directives` | `talk-fast-forward`・`shell-multi-compose` |
| 変数の値を、窓の透明度・位置・倍率へ書く係 | `crates/areka-runtime/`・`crates/areka-placement/`・`crates/areka-emo-present/src/scale.rs` | `window-timed-directives` | — |
| 変数の値を、舞台のアクターの `Visual`・`Arrangement` へ書く係 | 舞台の窓の側 | `shell-multi-compose` | — |
| 転換の組（`\![play,trans]` から「次の字・次の待ち・次の転換・1 本の終わり」まで）・前の絵を残して溶かす | 舞台の窓の側・`areka-emo-present` | `shell-multi-compose` | — |
| **形の語彙・CSS の式・文字列の読み手** | `crates/dola/src/easing.rs`・`interpolator/` | **この単位** | 3 つとも・`text-reveal-fade`・`text-reveal-dance` |
| **`--easing` を読んで口 1 へ渡す** | `crates/areka-runtime/` の引数の読み | **この単位** | — |

- 口 1 は、最初から形の欄（`Option<EasingFunction>`）を持つ。`window-timed-directives` は、そこへ `None`（等速）を渡す。この単位は、そこへ読んだ形を渡すだけになる。
- `shell-multi-compose` が roadmap で挙げている前提（`element-animation-easing`）の中身は、取り決めの最終形（種類の引数なし）では、口 1・口 2 と結線である。つまり、**働きの前提は `window-timed-directives` の側に在る**。この単位は、転換の既定の形を等速以外にする場合だけ、前提になる（8 章の問 1）。

### 4.3 dola に足す物（この単位の持ち分は a と b）

- **a. CSS の形の型**: `ParametricEasing`（`easing.rs:55`）に 3 つ足す。
  - `CssCubicBezier{x1, y1, x2, y2}`（x1・x2 は 0〜1。外なら読み手が断る）。
  - `Steps{count, position}`（位置は `jump-start`・`jump-end`・`jump-none`・`jump-both`）。
  - `CssLinear{points}`（出力と、あれば入力の割合の組の列）。
  - 今の `CubicBezier{x0..x3}` は、意味が違うので、そのまま残す。CSS の 5 語は、`CssCubicBezier` の定数と `Named(Linear)` へ写す。
- **b. 式と読み手**: `apply_easing` に 3 つの腕を足す。`cubic-bezier` は、回数を固定したニュートン法と二分法で t を解く（外部クレートは足さない。`interpolation` はそのまま使う）。読み手は `dola::easing` の関数 1 つ（文字列 → `Result<EasingFunction, 読めない理由>`）。`apply_easing` を crate の外から呼べるようにする（段 3 と `text-reveal-*` が使う）。
- **c. 口 1（`window-timed-directives` の持ち分。ここでは形だけ決める）**: 引数は、変数・終わりの値・長さ・形・開始時刻。出発点は、同じ変数の走っている変化を、**新しい開始時刻で評価した値**（`conflict_resolver.rs:73-74` の `Cancel` と同じ計算）。走っていなければ、変数の今の値。走っている変化と終わりの値が同じなら、何もしない（R11）。
  - 今の dola は、名前つきのストーリーボードを文書から始める形しか持たず、`from` を省くと変数の初期値になる（3.1）。台本の値は実行時に決まるので、口 1 が要る。文書を指令のたびに組み直す形は採らない（検証とコンパイルを毎回通すことになる）。
- **d. 口 2（同上）**: 入れ物（`DolaAnimator`）の中の、走っている全部を `conclude` する。

### 4.4 流れ（窓の単位・段 2）

```
台本 \![set,alpha,0,--time=500,"--easing=ease-out"]
  → compile.rs: Custom{command:"set", params:[alpha, 0, --time=500, --easing=ease-out]}
      （待つ形だけ、時刻の表を 500 ms 進める＝window-timed-directives）
  → CuePlayer が、発火時刻に着いた刻みで配る
  → 受け手（名前で自分の物だけ拾う）: 引数を読む。--easing は dola の読み手 1 つで読む
      読めなければ等速にして warn! を 1 行（R5）
  → 既に在る受け渡しの道（move・zorder と同じ形）で ECS へ:
      {スコープ, 変数, 終わりの値, 長さ, 形, 開始時刻＝cue の絶対の発火時刻}
  → フレームの相: そのスコープの窓の DolaAnimator に、口 1 で始める
  → tick_dola_animators(FrameTime) → 値を書く係 → Visual.opacity ほか
```

- 始まりは「cue の絶対の発火時刻」で、受け手に届いた時刻ではない。届くのが刻み 1 回ぶん遅れても、最初のフレームは「発火時刻から過ぎたぶんだけ進んだ値」を出す（R8）。終わりの時刻は、台本の時刻の表と一致する。
- 値は、変わっている間ずっと動くので、フレームごとに読む（プル）。始まり・終わり・割り込みは、時点が分かるので、知らせで渡す（プッシュ）。

### 4.5 時刻の扱い（頼まれた確かめ 4）

- **始まりを決めるのは台本の時計。値を読むのはフレームの時計。SERIKO の時計は使わない。**
- 台本の時計のうち、使うのは「発火の絶対時刻」だけである。台詞からの相対の時計の上では動かさない。理由は 2 つ。
  - 相対の時計は、`\x` と選択肢で止まる（Barrier）。その上で動かすと、待っている間に転換が止まる（R12 に反する）。
  - さくらスクリプト 1 本ごとに起点が替わる。1 本をまたいで続く転換を、相対の時計では表せない。
- 今は、台本の時計（`GetTickCount64` のミリ秒）とフレームの時計（`QueryPerformanceCounter` の秒）の物差しが違い、発火時刻を `FrameTime` へ正確に直せない。**v0.2.0 の時刻の土台（`dola-interrupt-ticks` → `talk-anchor-interrupt-ticks`）で 1 本になることを前提にする**（仮定 A2。`window-timed-directives` と同じ前提）。
- 早送りは、時計を進めて表すのでなく、口 2 で表す。`talk-fast-forward` は「台詞の時計を `\x` の地点まで早回しする」形だが、待たない転換は `\x` の地点より後ろで終わることがあり、早回しだけでは終わりの値に届かない。

### 4.6 「途中の絵から続ける」「終わりの絵へ飛ばす」（頼まれた確かめ 2）

- **支えられる。** 数の値（透明度・位置・大きさ）は、口 1 の出発点の決め方（4.3 c）で「途中から続ける」になり、口 2 で「終わりへ飛ぶ」になる。どちらも、本体の計算は dola にもう在る。
- 形が等速でないとき、途中から続けると、新しい形は進み具合 0 から始まる（速さはつながらない）。CSS のトランジションと同じ動きで、値は飛ばない。
- 行き過ぎる形（`easeOutBack` など）の途中の値は、0〜1 の外に在ることがある。出発点には、その値をそのまま使う。透明度は、書く係が 0〜100 に収める（R13）。
- **サーフェスの転換（前の絵から次の絵へ溶ける）は、数の値 1 つでは表せない。** 「前の絵」を消えるまで残し、その透明度を 1 → 0 に動かす。途中に次の転換が来たら、消えかけの絵はそのまま消え続け、今の絵が新しい「前の絵」になる（`areka-novel-fade-start-point-2026-11-19.md` の見立て）。残す絵の列と重ね方は `shell-multi-compose` の持ち物で、1 枚ごとの透明度に、同じ口 1 を使う。この単位の型に足す物は無い。
- 転換なしの変更（すぐ替える）が、走っている変化の値に来たら、その変化を捨てて、新しい値を置く（`cancel` の後に値を書く）。

### 4.7 SERIKO のアニメーションとの関係（頼まれた確かめ 3）

- SERIKO は「どのコマの絵か」を決める仕組みで、時間つきの変化は「組み上がった 1 枚（スコープやアクターの `Visual`）を、どの透明度・位置・大きさで出すか」を決める仕組みである。層が違い、互いの状態を読まない。
- 転換の間も、新しい絵の SERIKO のアニメーションは進む。前の絵は、サーフェスが替わった時に再生状態が捨てられる（`looper.rs:794-798`）ので、止まった 1 枚として消える。
- 早送りと口 2 は、SERIKO のアニメーションを飛ばさない（取り決め: 並走とシェルの時計の動きは早送りしない）。
- **段 3（pattern定義のコマ間をつなぐ）を作るときの形（大枠だけ・この spec の最初の範囲の外）**:
  - 読み手: pattern定義の末尾のオプションを読む（正典の `--alpha,値` もここ。`decode.rs:538-550` と `model.rs:155` に欄を足す）。
  - 値: SERIKO は、コマが替わる時点で「前のコマの値 → 今のコマの値・待ち時間・形・コマの始まりの時刻」を知らせ、フレームの相が口 1 で動かす。SERIKO の 16 ms の刻みで値を取って焼き直す形は採らない（刻みとフレームがずれ、時刻が正確でなくなる・毎刻み 1 枚を焼き直す）。
  - 絵: 動かす部品を、焼いた 1 枚と別の `Visual` で持つ変更が、合成（`areka-emo-compose`）と表示（`areka-emo-present`）に要る。ここが一番重い。
- **段 4（element を名指しして動かす）**: element の指し方が正典に無い。段 3 の「別の `Visual` で持つ」作りの上に、名前を付ける仕事になる。設計していない。

### 4.8 失敗の扱い

- 読めない形・0〜1 の外の x1／x2・`steps(0)`・点が 1 つの `linear()`: 等速に置き換え、`warn!` を 1 行（原文・命令・スコープ）。トークは止めない。メッセージボックスは出さない。
- `--easing` が在って `--time` が無い（か 0）: 値はすぐ替わる。形は使わない。`debug!` を 1 行。
- 口 1 が dola の誤り（`RuntimeError`）を返した: 終わりの値をすぐ置いて、`error!` を 1 行（値を失わない）。
- NaN・無限大は、読み手が断る（今の `validate` の数値の規則と同じ線）。

### 4.9 ほかの単位との境

- `window-timed-directives`: 時間つきの指令の実行・時刻の表・結線・口 1・口 2・値を書く係を持つ。この単位は、その引数の読みに `--easing` の 1 欄を足し、口 1 の形の欄へ渡す。同じファイル（`crates/areka-runtime/src/move_cue_args.rs`・新しい `set` の引数の読み）を触るので、直列（この単位が後）。
- `shell-multi-compose`: 転換の組・前の絵・アクターの値を書く係を持つ。形の引数は取り決めに無いので、この単位からは「既定の形」を受け取るだけ（等速なら、何も受け取らない）。
- `parallel-visual-script`: 並走の区間の中に時間つきの指令を書けるかは、同 brief の議題 2。書けるなら、口 2（早送り）がその変化を飛ばすかを決める必要がある（8 章の問 5）。
- `shell-hover-animation`: SERIKO の起動のきっかけ（interval と bindgroup）だけを足す。時間つきの変化は使わない。境は重ならない。段 3 を作る時だけ、seriko の同じファイル（`looper.rs`・`table.rs`）で直列になる。
- `seriko-script-triggers`: `\i[ID]` を持つ（3.5）。境は重ならない。
- `text-reveal-fade`・`balloon-scroll-fade`（v0.3.0）: 字の透明度を時間で変える。この単位より先に着地する。式を別に書かず、dola の `apply_easing` を使う形にそろえてもらう（6 章）。
- `check-script-arg-checks`: `--easing` の値の検査は、dola の読み手を呼ぶだけにする（検査の式を 2 つ持たない）。

## 5. pasta に期待する事

| 番号 | 期待 | 骨子・取り決めとの関係 |
| ---- | ---- | ---------------------- |
| P1 | 転換の命令に、形の引数を付けない。areka は既定の形（仮定 A3: 等速）で動かす | **合っている**。取り決め「転換」: 「命令の形（仮）: `\![play,trans,ミリ秒]`」。`play-script-guide.md` 3 章: 「種類は書かず、時間だけ書く」 |
| P2 | 時間は 0 以上のミリ秒の整数。0 は「ここからはすぐ替える」 | **合っている**。取り決め「転換」: 「時間 0 は、前の転換の組を終わらせる」 |
| P3 | 字を待たせたい転換は、組の変更を全部書いた後ろに、正典の `\_w[同じ時間]` を打つ（areka は「転換を待つ」命令を持たない） | **合っている**。取り決め「転換」: 「待つのは正典の待ちのタグで、pasta が打つ」「まとめの終わりは『次の字・次の待ち・次の転換・1 本の終わり』」 |
| P4 | 同じ値を毎回書いてよい（areka は、走っている変化をやり直さない） | **合っている**。取り決め「命令の表」: 「表の命令は、同じ値をもう一度受けたら何もしない」 |
| P5 | 出し直しの 1 本（`\![play,again]`）の頭で、areka は走っている転換も全部、終わりの値にする | **文面に無い**（食い違いではない）。取り決めの出し直しの行は「鳴っている途中の単発の音と声を止め、並走を全部止める。残る命令の値を全部既定へ戻してから、続く命令を当てる」で、転換に触れていない。値を既定へ戻す時に変化は捨てられるので、結果は同じになる。文面に 1 句足してもらうと、読み違いが出ない |
| P6 | 高速再生の 1 本（`\_q` 付き・転換の命令なし）が来たとき、前の 1 本で始めた転換がまだ走っていたら、areka はどうするか | **決まっていない**。取り決め「転換」は「クリックで字を早送りしたら、走っている転換も、新しい絵へ飛ぶ」（クリックの早送り）だけを書く。高速再生は pasta が `\_q` を付ける形（骨子 2.6）で、areka から見ると、ふつうの 1 本である。8 章の問 5 |
| P7 | 舞台を開かない演劇（デスクトップの窓のまま）で、areka へ届く位置・大きさ・透明度の命令は、正典（`\![set,alpha,…,--time]`）か、`play` の命令と転換か | **文面どうしが読み分けられない**。取り決め「原則」: 「正典に在る物は正典のタグで出し、`play` の命令は、正典に無い物だけにする」。同「命令の表」: 「舞台・並び・位置・重ね順・大きさ・透明度・転換・揺れ・フラッシュ・色調・カメラ ｜ `play` の命令」。経緯（`discovery-2026-11-19.md` 1401〜1409 行）は「SSP では正典のタグ、areka では `play` の命令」。どちらで届いても、areka の中では同じ窓の同じ値なので、同じ仕組み（4.2）で動かす。ただし、デスクトップの窓への `play` の位置・大きさ・透明度・転換の受け手を持つ単位が、areka に無い（6 章） |
| P8 | `\i[ID]`（アニメーションを名指しで始める） | **合っている**。取り決め「並走」: 「頼みごとに足す物: 正典の `\i[ID]`」。areka の持ち主は `seriko-script-triggers`（3.5）。この単位ではない |
| P9 | pasta は `--easing` を打たない（演劇で使わない）。作者が、ふつうのトークで正典の時間つきの指令に手で添える物である | 取り決めに記載は無い（食い違いではない）。SSP が `--easing` をどう扱うかは、確かめていない（R17） |

## 6. ほかの単位に期待する事

1. **`window-timed-directives`（一番大事）**: 等速の実装を、自前の線形の計算で書かず、dola の連続値の側（`DolaRuntime`・`DolaAnimator`）の結線として作る。口 1 は最初から形の欄を持つ。口 2 を持つ。値を書く係は、窓の単位に閉じてよいが、口と結線は、舞台のアクターからも使える置き場（dola・wintf・`areka-runtime`）に置く。
2. **同 brief の文面**: Approach の「値は台本の時計で動かす」は、「始まりは cue の絶対の発火時刻・値はフレームの時刻で読む・`\x` で待つ間も進む」と読める形に直してもらう（4.5。相対の時計の上で動かすと、待たない形〔`\![moveasync]`・`--wait` 無しの `\![set,alpha]`〕が `\x` で止まる）。
3. **着手の順**: v0.2.0 の時刻の土台 → `window-timed-directives` → この単位（段 1・段 2）。段 1（dola の式と読み手）だけは、どの単位とも同じファイルを触らないので、先に出せる。`shell-multi-compose` の Dependencies（`roadmap.md:1734`）は、`window-timed-directives` を足し、この単位は「転換の既定の形を等速以外にする場合だけ」に弱められる。
4. **`shell-multi-compose`**: 転換の組・前の絵の列・アクターの値を書く係・「同じ時刻の命令を全部当ててから最初の絵を出す」（取り決め「命令の表」）を持つ。時間で値を動かす所は、口 1・口 2 を使い、別に作らない。
5. **デスクトップの窓への `play` の命令**（P7）: 舞台を開かない演劇で `\![play,trans]`・`\![play,透明度]` などが届くなら、受け手は `window-timed-directives` の値を書く係へつなぐ。持ち主を、`shell-multi-compose` か `window-timed-directives` のどちらかの要件に入れる。
6. **`talk-fast-forward`**: 早送りの時に、口 2 を呼ぶ（舞台のアクターにも、窓の単位にも）。窓の単位の指令を飛ばすかは `window-timed-directives` の議題 3。
7. **`text-reveal-fade`・`balloon-scroll-fade`**: フェードの式を、dola の `apply_easing` の上に書く（この単位が後から形を足せる）。
8. **v0.2.0 の時刻の土台**: `FrameTime` と台本の起点が、同じ物差しになる（4.5 の前提）。
9. **`currentghost-property-others`**: 変化の途中に `currentghost.scope(ID).scaling` を読んだ時の値は、`window-timed-directives` の議題 5 で決まる。この単位は関わらない。

## 7. 置いた仮定

- A1: この spec の最初の範囲は、段 1（語彙・式・読み手）と段 2（台本の `--easing`）。段 3・段 4 は起票しない（brief の Approach「最初の 1〜2 段だけを本 spec の範囲に切る」・議題 2 を「台本から」と仮に答えた）。
- A2: 着手は v0.2.0 の時刻の土台と `window-timed-directives` の後で、台本の起点とフレームの時計は同じ物差しになっている。
- A3: 転換（`\![play,trans]`）の既定の形は等速。正典の `--time` の既定も等速（ukadoc は形を書いていない。SSP の実際の形は測っていない）。
- A4: 名前の語彙は、CSS の 7 語と関数 3 つに、見本集の 30 語を足す（dola に式がもう在り、写すだけで済む）。
- A5: 台本でカンマを含む形（`cubic-bezier(…)`・`steps(…)`・`linear(…)`）は、引数の全体を二重引用符で包む（`"--easing=cubic-bezier(0.2,0,0,1)"`）。読み手は、括弧の中の区切りにカンマと空白の両方を受ける。
- A6: 回転は入れない（窓にも element にも口が無い。brief の議題 6）。
- A7: 口 1 の出発点は、走っている変化を新しい開始時刻で評価した値（最後に描いたフレームの値ではない）。差は 1 フレームぶんより小さく、時刻に対して正確な側を取った。
- A8: 同じ終わりの値の再指定は、長さや形が違っても、何もしない。
- A9: `--easing` は、`\![move]`（待つ形）にも `\![moveasync]` にも同じに効く。待つ長さは `--time` のままで、形では変わらない。
- A10: dola の `load_document` で文書を差し替えても、走っているインスタンスは影響を受けない（コンパイル済みの区間を持つ作りから読んだ見立て。口 1 が文書を使わないなら、関わらない）。

## 8. 決まっていない所（開発者に諮る問い）

1. **転換の既定の形は、等速でよいか。それとも `ease` 系にするか。** → 等速なら、`shell-multi-compose` はこの単位を待たずに作れる。等速以外なら、この単位の段 1 が `shell-multi-compose` の前提になる。
2. **土台（結線・口 1・口 2）は、決着どおり `window-timed-directives` が作る、でよいか。条件として「dola の連続値の側を使う」を同 spec の要件に入れてよいか。** → 入れないと、等速の計算が `areka-runtime` の中に手書きされ、舞台のアクターとこの単位が、同じ物をもう 1 つ作ることになる。
3. **デスクトップの窓へ届く `play` の位置・大きさ・透明度・転換（P7）の受け手を、どの単位が持つか。** → `shell-multi-compose` が持つなら、同 spec は舞台の窓より先に、窓の単位の受け手を作る。`window-timed-directives` が持つなら、同 spec の範囲に `play` の 4 つの命令と転換の組が入り、規模が増える。
4. **最初の使い手は、台本（段 2）でよいか**（brief の議題 2）。→ pattern定義（段 3）から始めるなら、合成と表示の作りを変える仕事が先に来て、規模が 2 倍を超える（10 章）。
5. **早送りで終わりへ飛ぶ範囲**: 並走の区間で始めた時間つきの変化・窓の単位の待たない指令（`\![moveasync]`・`--wait` 無しの `\![set,alpha]`）・高速再生の 1 本（`\_q`）が来た時に走っている転換。→ 口 2 を「入れ物の全部」にするか、「どの流れが始めたか」の印を変化に持たせるかが変わる。
6. **名前の語彙**（brief の議題 4）: 見本集の 30 語を受けるか。→ 受けないなら、読み手の表が 30 行減るだけで、式は変わらない。
7. **カンマの衝突**（brief の議題 3）: 台本は二重引用符で包む形でよいか。pattern定義（段 3）は、正典の `--名前,値` のカンマ区切りと割れるので、段 3 を作る時に決める。→ 読み手の括弧の中の区切りの決まりが変わる。
8. **`steps()` の形を透明度と位置に効かせてよいか**（コマ送りの見た目を、少ない記述で書ける。害は見当たらない）。→ 断るなら、読み手で落とす。

## 9. 差し戻しが要りそうな所

- **骨子・取り決め（pasta）への差し戻しは、無い。** 「途中の絵が出発点」「始めるだけで待たせない」「早送りで新しい絵へ飛ぶ」「形の引数なし」は、どれも 4 章の形で支えられる。
- 取り決めの要件定義へ送る物が 2 つ在る（差し戻しではなく、文面の足し）: P5（出し直しの頭で転換も終わらせる、の 1 句）・P7（デスクトップの窓での位置・大きさ・透明度の届け方。原則「正典を先に使う」と命令の表の読み分け）。
- areka の中の決めごとで、直しが要りそうな所:
  - `roadmap.md:1734` の `shell-multi-compose` の Dependencies（`element-animation-easing` → 働きの前提は `window-timed-directives`）。
  - `window-timed-directives` の brief の Approach「値は台本の時計で動かす」の文面（6 章の 2）と、Out「演劇では正典の `--time` を使わない、と pasta の側で決まっている」の出どころ（取り決めの brief の決定の節・骨子・`play-script-guide.md` を `--time` で grep して、この文は見つからなかった。`discovery-2026-11-19.md` 1401〜1409 行の経緯に「areka では `play` の命令」が在る）。
  - この単位の brief の Current State「areka が dola から使っているのは台本の配信と時計だけ」は合っているが、「cue は点のイベントで、区間のあいだ値を変え続ける指令を持たない」から「土台が無い」と読むのは違う。連続値の側は dola に在り、結線が無いだけである（3.1）。

## 10. 規模の見立て

| 範囲 | タスクの数の幅 | 中身 |
| ---- | -------------- | ---- |
| 段 1（語彙・式・読み手） | 5〜7 | 型 3 つ・式 3 つ・読み手・`apply_easing` の公開・決定論のテスト（CSS の定義の値との突き合わせ） |
| 段 2（台本の `--easing`） | 4〜6 | 4 つの指令の引数の読み・口 1 へ渡す・読めない値の記録・`check_script` の表・網羅台帳と `COMPAT_ARCHITECTURE.md`・SSP での確かめ |
| **この spec の最初の範囲（段 1＋段 2）** | **9〜13** | `window-timed-directives` が土台を作った後 |
| 土台もこの単位が作る場合（順が逆になったとき） | ＋5〜7（14〜20） | 結線・口 1・口 2・値を書く係の最小 |
| 段 3（pattern定義のコマ間） | ＋12〜18 | 読み手・SERIKO からの知らせ・焼かずに動かす層（合成と表示） |
| 段 4（element の名指し） | 見立てていない | 指し方から決める |

- brief の見立ては「未測定（覚書・最初の 2 段で M の見込み）」（`roadmap.md:425`）。この見立て（9〜13）は、M（12〜16）の下の端か、それより小さい。差の理由は 2 つ: 土台が `window-timed-directives` へ出た（議題 1 の決着）・本体が dola にもう在る。

## 11. 確かめた物と机の上だけの物

- **実行して確かめた物: 0。**
- **コードを読んで確かめた物**（3 章の `path:行`）:
  - dola の形の型と式・dola のトランジションの型・`DolaRuntime` の口の一覧と注釈・`Cancel`／`Trim` が新しい開始時刻で評価すること・`from` を省いた時の決まり方（`resolve.rs:357-377` と呼び手 `compile/mod.rs:286-292`）。
  - `DolaAnimator` と `tick_dola_animators` が本番で使われていないこと（`crates/` を 2 つの名前で grep）。
  - 3 本の時計の出どころ。`\![move]` の時間の今の扱い。`\!` の引数の二重引用符。`Visual`・`Arrangement` の欄。pattern定義の読み手が取る欄。サーフェスが替わった時に SERIKO が再生状態を捨てること。
  - 網羅台帳の `\i`・`\![anim,…]`・`\__w[animation,ID]` の持ち主の欄が空であること。
- **読んでいない・机の上だけの物**:
  - `DolaRuntime` の中（`timeline_manager`・`instance_manager`・`subscription_manager`）は読んでいない。`start` の後で `from` を実行時の値へ差し替える所が在るかは、確かめていない（在れば、口 1 はもっと小さくなる）。`conclude`・`cancel` の後の値の残り方は、注釈（`facade.rs:226-239`）を読んだだけ。
  - `DolaAnimator` を窓の実体に付けて、毎フレーム `Visual.opacity`・窓の位置・倍率の席へ書く係が、今の `areka-runtime`・`areka-placement`・`areka-emo-present` の作りに乗るかは、読んでいない。窓の位置は ECS の `Arrangement` でなく placement の持ち物なので、書く係は 3 つに分かれる見込み。
  - cue の受け手から ECS へ渡す道（`move_rx`・`zorder_rx` と同じ形）に、開始時刻（絶対）を載せられるかは、v0.2.0 の時刻の土台の設計しだいで、読んでいない。
  - 台詞の刻みの周期（`talk-anchor-interrupt-ticks` の brief は「50・1000・16 ms」と書く）は、コードで確かめていない。
  - `cubic-bezier` を解く式の誤差と回数は、決めていない。
  - SSP が `--easing` と、pattern定義の知らないオプションをどう扱うかは、ukadoc でも SSP でも確かめていない。
  - 段 3・段 4 は、brief の Current State の記述（合成が「コマごとに 1 枚を焼き直す」）を信じて書いた。`areka-emo-compose`・`areka-emo-present` の合成の道は読んでいない。
  - 前の絵を残して溶かす作り（`shell-multi-compose` の持ち物）が、今の表示の作りに乗るかは、読んでいない。
