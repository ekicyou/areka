# Brief: areka-P0-seriko-trigger-intervals

> 2026-10-04 棚卸㉑で起票（roadmap「覚え書き」の「`shell-implicit-surface` の着地で残した 7 件」の ⑺ を格上げ）。**段は優先（動く画像）**。roadmap-draft の「サーフェスアニメーション（段階 A・順位 6）」の束（47 件）から、最初の 1 本を 20 タスク以内で切り出す。

## Problem

- **利用者**: **台詞を話している間、キャラクターの口が動かない（口パクの `talk` が動かない）。** 多くのゴーストは `animationN.interval,talk,数値` で口を動かしているが、areka はこの語を記録するだけで再生しない。
  - 正典 `talk,数値`（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#talk_2c_6570_5024:1）は「数値分の文字がくるごとにアニメーションする。」。
- **ゴースト作者**: 切り替えた瞬間に 1 回だけ再生する `runonce`、一定の間隔で再生する `periodic`、`\e` で再生する `yen-e` が動かない。台本から呼ぶ `\i[ID]` も効かない。
  - `runonce`（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#runonce:1）は「サーフェスに切り替わった瞬間に1回のみ再生。」。
  - `periodic,数値`（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#periodic_2c_6570_5024:1）は「そのサーフェスである間数値秒間隔で定期的に再生。」。
  - `yen-e`（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#yen-e:1）は「SakuraScriptで\eが来た時に実行。」。
  - `never`（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#never:1）は「自動では実行されない。」で、`\i[*]` などで呼ばれる。
  - `\i[ID番号]`（https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5ci_5bID_756a_53f7_5d:1）は「現スコープ側にID番号のサーフェスアニメーションを表示する。」。
- **開発者**: 持ち主の spec が居ない。`animated-image-playback` は `always` だけを持ち、`runonce`・`bind` などを「サーフェスアニメーションの束＝別途」として範囲外にしている。

## Current State

- 読み込み: `crates/areka-parsers/src/shell/decode.rs` の `normalize_interval` は `bind`・`random`・`bind+random` を型に写し、それ以外の語（`talk`・`runonce`・`periodic` など）は原文のまま `Interval::Other` に転記する（語は落ちていない）。
- 再生: `crates/areka-seriko/src/table.rs` の `AnimationTable::from_world` が採るのは `Random`・`BindRandom` と、読み替える `sometimes`・`rarely` だけ。ほかの `Other` は `debug!` に語を残して採らない。モジュールの冒頭は「口パク（`interval,talk`）・`\i[N]`・動的 bind・talk cue は…この採録フィルタが自然に除外する」と書いている。
- 時計: 抽選の境目（1000 ms）と、コマの累積の待ち時間（`crates/areka-seriko/src/timeline.rs`）の 2 層。繰り返しの仕組みは `animated-image-playback` が入れる。
- 台本: `crates/areka-parsers/src/sakura/decode.rs` は `\i` を「subset 外のタグ」として素通しの `Raw` にする。seriko へ届く指令は `crates/areka-seriko/src/actor.rs` の `SerikoMsg::Cue(TalkCue)` の汎用の運び手で、今は名前 `bind` だけを開封する。
- 網羅台帳: `talk`・`runonce`・`never`・`yen-e`・`periodic`・`always` は `vocabulary-only`・担当なし（`doc/ukadoc-coverage/ledger/assets.toml`）。`\i[ID番号]` と `\i[ID,wait]` は `absent`・担当なし（`sakura-script.toml`）。

## Desired Outcome

- 台詞の文字が出るのに合わせて、`talk` のアニメーションが「数値分の文字ごと」に再生される（口パク）。
- `runonce` は面に切り替わった瞬間に 1 回、`periodic` はその面の間だけ指定の秒ごとに、`yen-e` は `\e` で再生される。
- `never` のアニメーションは自動では動かず、`\i[ID]`（番号、または `animation*.name` の名前）で 1 回再生される。

## Approach

- seriko の表に、引き金の種類（抽選・切替の瞬間・定期・文字の到着・`\e`・台本からの呼び出し）を持たせる。抽選以外は、seriko の外から届く知らせで起こす。
- 文字の到着と `\e` は、台本の再生（dola の cue）から seriko へ汎用の運び手（`\!` コマンドと同じ 1 本の口）で知らせる。新しい個別の型は作らない（決まり「`\!` コマンドは汎用の運び手 1 本」）。
- `\i[ID]` は sakura の解読でタグとして拾い、同じ運び手で seriko へ届ける。
- 繰り返し・時計は `animated-image-playback` の仕組みの上に乗せ、3 つ目の時計を作らない。

## Scope

- **In**:
  - interval の `talk`・`runonce`・`periodic`・`yen-e`・`never`。
  - `\i[ID番号]`（番号と `animation*.name` の名前）。
  - 決定論のテスト（偽の時計と偽の文字の到着）・網羅台帳の行・実機の確かめ（口パクを持つ検体）。
- **Out**（別途）:
  - `\i[ID,wait]`（アニメーションの終わりまで台詞を待たせる＝台本の進行との同期が要る）。
  - `start`・`alternativestart`・`stop` などの、アニメーションから別のアニメーションを呼ぶメソッド。
  - `\![anim,…]` 系のタグと、束の残り（roadmap-draft「サーフェスアニメーション」の 47 件のうち本 spec が持たないもの）。
  - `always`（`animated-image-playback`）・`bind`（着せ替え・実装済み）・`sometimes`／`rarely`（実装済み）。

## Boundary Candidates

- seriko の表と引き金（`areka-seriko` の `table.rs`・`looper.rs`・`state.rs`）。
- 台本から seriko への知らせ（文字の到着・`\e`・`\i`）。
- sakura の `\i` の解読。

## Out of Boundary

- 台本の進行そのもの（kanade・dola の cue）の作り替え。知らせを 1 本足す以上は変えない。
- 合成と描画（emo-compose・emo-present）。

## Upstream / Downstream

- **Upstream**: `animated-image-playback`（繰り返しの仕組みと同じ表）・`surface-element-nesting`（子の時計）・完了 `seriko-loop`・完了 `seriko-engine`。
- **Downstream**: `\i[ID,wait]` と、束の残りを持つ次の spec（別途）。`makoto-dll-host`（台詞の変換の後の文字で口が動くか）。

## Existing Spec Touchpoints

- **Extends**: 完了 `shell-implicit-surface`（`sometimes`・`rarely` の読み替えと、残る語の記録）・完了 `seriko-loop`（抽選の表）。
- **Adjacent**: `animated-image-playback`（同じ `table.rs`・`looper.rs`）・`surface-element-nesting`（`areka-seriko/src/{table,looper,actor,state}.rs`）・`talk-fast-forward`／`text-reveal-fade`（文字の出方。口パクの「文字が来る」の数え方に関わる）。

## Constraints

- 1 フレーム遅らせる解は取らない。文字の到着と口の動きを同じフレームで解く。
- アニメのエンジンは sakura と seriko の 2 つのまま。3 つ目の時計を作らない。
- テストは兄弟ファイルへ（`actor.rs` 645 行・`bind.rs` 1043 行は伸ばさない）。1 ファイル 1,000 行以下。ログの無い失敗の経路を作らない。

## 2026-10-04 棚卸㉑で起票

- **出どころ**: roadmap「覚え書き」の「`shell-implicit-surface` の着地で残した 7 件」の ⑺（`sometimes`・`rarely` 以外の間隔の語は動かない）。覚え書きの書き方より実害が大きい＝口パクが動かない。
- **規模**: M〜L（16〜20 タスク）。束の全体（47 件）は 20 を超えるので、最初の 1 本を上の In に絞った。それでも 20 を超えそうなら `periodic`・`yen-e` を別途へ回す。
- **前提**: `animated-image-playback`（繰り返しの仕組み）。その前提として `animated-image-decode` と `surface-element-nesting`。
- **触るファイル**: `crates/areka-seriko/src/{table,looper,state,actor,timeline}.rs` と兄弟のテスト・`crates/areka-parsers/src/shell/{model,decode}.rs`（`Interval` の型）・`crates/areka-parsers/src/sakura/decode.rs`（`\i`）・`crates/areka-sakura/src/compile.rs`（汎用の運び手へ載せる）・台本から seriko へ知らせる口（設計で決める）・`doc/ukadoc-coverage/ledger/{assets,sakura-script}.toml`。
- **共有しうる相手**: `animated-image-playback`・`surface-element-nesting`（seriko・シェルのパーサ）・`balloon-font-file`／`text-typesetting` などの文字とバルーンの列（`areka-sakura`）・`talk-fast-forward`（文字の出方）。
- **議題**:
  1. `talk` の「数値分の文字」の数え方。書記素クラスタで数えるか。`\_q`（一括表示）や早送りのときに口をどう動かすか。
  2. 文字の到着を seriko へ知らせる経路（台本の cue に載せるか、文字の層から送るか）。0 フレームで揃える形を設計で決める。
  3. 最初の 1 本の範囲。`\i[ID,wait]` を別途に回すことでよいか。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M〜L（16〜20 タスク）。変わらず。切らない（議題 4 は大きさでなく列の都合の問い）。
- 前提の状態: **`animated-image-playback` を待つ必要は無い**。`talk`・`runonce`・`periodic`・`yen-e`・`never`（と `\i`）はどれも「引き金が来たら 1 回流す」で、一番上の面の抽選（`looper.rs` の `on_tick`）が今している「当たったら再生を登録して 1 回流す」形にそのまま乗る。繰り返し（`always`）を要るのは playback の側だけ。待つのは触るファイルの重なりだけ＝playback とは seriko の表・時計（`table.rs`・`looper.rs`・`parts.rs`）、`element-base-method`・`collisionex-regions` とは読み手（`shell/model.rs`・`shell/decode.rs`）、`anchor-tag-canon` とは台本のコンパイル（`compile.rs`・`sakura/decode.rs`）。**列の中で playback より前へ出せる**（口が動かないのは利用者の目に見える）。
- 段: 優先のまま。壊れたのではなく正典の語が未実装。同梱の検体でシェルを持つ 4 本（`vendors/sample_ghost/` の emo2・claudia・konnoyayame・R_POST_and_KOMAINU の `surfaces.txt`）はどれも `talk`・`runonce`・`periodic`・`yen-e`・`never`・`always` を書いていない＝同梱では症状が出ない（`element-base-method` がバグなのは同梱のクローディアでキャラクターが消えるから）。
- 崩れた前提／古くなった位置:
  - 読み手 `normalize_interval` は `talk,数値`・`periodic,数値` の**数値を落とす**（`Interval::Other` は語だけを持つ）。型 `Interval` に欄か腕を足す必要がある。
  - **文字の到着は、もう seriko に届いている**。seriko は全部の cue を受け取り、cue の受け口（`actor.rs` の `handle_message` の分類）が `Text` を「担当外」として `debug!` で読み飛ばしている。`Text` は 1 続きの文字列と、その再生時間（`areka-sakura` の `text_playback_duration` が文字数から求める）を持つ＝seriko の中で「N 文字ごとの時刻」を求められ、新しい知らせの口は要らない見込み（議題 2 の答えが変わる）。
  - `\e` は `compile.rs` の終端の腕で cue を出さずに台本を切り詰めるだけ、`\i` は `sakura/decode.rs` で `Raw` になり `compile.rs` が捨てる。`yen-e`・`\i` だけが台本のコンパイルの列に触る。
  - `surface-element-nesting` が部品の時計（`parts.rs` の `PartClocks`）を足した。子や pattern の先の `talk` も動かすなら、部品の門 `gate` と進め方 `advance` にも引き金を足す。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/shell/{model.rs, decode.rs}`（`Interval`）
  - `crates/areka-seriko/src/{table.rs, looper.rs, parts.rs, actor.rs, timeline.rs}`（`runonce` の切替の瞬間に `state.rs` も）と兄弟のテスト
  - `crates/areka-parsers/src/sakura/decode.rs`・`crates/areka-sakura/src/compile.rs`（`\e`・`\i`）
  - `doc/ukadoc-coverage/ledger/{assets,sakura-script}.toml`・口パクの検体（新しく作る）
- 議題（答えで作業が変わるものだけ）:
  1. 起票時のまま（数え方・`\_q`・早送り）。
  2. 推しを足す: seriko が受け取っている `Text` の cue から自分で数える（emo-text・dola に触らない）。
  3. 起票時のまま。
  4. `yen-e`・`never`＋`\i` を後回しにして、台本のコンパイルの列から切り離すか。切れば seriko と読み手だけで口パク（`talk`・`runonce`・`periodic`）を先に出せる。
- 見つけた穴: 実機の確かめに使える口パクの検体が同梱に無い（作る必要がある）。


## 2026-10-05 `animated-image-playback` からの申し送り（同 spec の要件討議・議題 4／要件 10.5）

> この節は `areka-P0-animated-image-playback` が生きている間（要件 → 設計 → 実装 → 完了）、同 spec の側で正しく保つ（開発者指示 2026-10-05）。設計・完了の段で形が決まるたびに書き足す。着手時に引き直すこと。

- **優先度**: 開発者指示（2026-10-05・棚卸㉒の後）で roadmap の段を「優先（高）」へ上げた（棚卸㉒の再測定の「段: 優先のまま」を上書き）。C5 を組むとき、優先の段の先頭に置く。
- **引き受けるもの（本 spec の In へ足す）**: interval の **`always` を含む組み合わせ**（`bind+always` ほか）。正典は「SSPのみ+区切りで列挙する事で組み合わせ指定が可能」（[descript_shell_surfaces `animation*.interval`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation*.interval%2C%E3%82%A4%E3%83%B3%E3%82%BF%E3%83%BC%E3%83%90%E3%83%AB)）と書くだけで、相手や語順を限らない。決めることは ⑴ `bind+always`＝着せ替えが有効な間だけ繰り返す ⑵ `+` の語順の違い（`always+bind`）⑶ 3 語以上の組み合わせをどこまで読むか。組み合わせだけは `animated-image-playback` の繰り返しの仕組みに働きの上で依存する（棚卸㉒の「働きの依存は無い」は、組み合わせを引き受ける前の `talk` ほかについての話）。
- **`animated-image-playback` が入れるもの**: `always` の**単独**（完全一致）だけ。表示と同時に始まり、最後のコマの後に頭へ戻って繰り返す。一番上のサーフェスでは切り替えで頭から、子のサーフェスでは子の時計の決まり（巻き戻らない）。途中の終わりのコマ（`-1`）は消してから頭へ戻る。待ち時間の合計 0 は繰り返さずに記録を残す。
- **`animated-image-playback` が残すもの（変更 0）**: `always` を含む組み合わせは、今までどおり元の綴りを添えた記録を残して駆動しない（同 spec の要件 4.9）。`runonce`・`never`・`yen-e`・`talk`・`periodic`・`bind` 単独 ほかも同じ（同 4.8）。
- **着せ替えの種類かを見ている場所**（起票時の実測・`animated-image-playback` の research.md 5 章 議題 4）: 合成の `is_bind_interval`（`crates/areka-emo-compose/src/plan.rs`）・`NestTable` の `bind_ids`（`crates/areka-emo-compose/src/nesting.rs`）・seriko の着せ替えの番人（`crates/areka-seriko/src/parts.rs` の `gate`・`looper.rs`）。読み手 `normalize_interval`（`crates/areka-parsers/src/shell/decode.rs`）は完全一致で見分ける。
- **時刻の決まり**: 時刻は正確に扱う（開発者 2026-10-05）。待ち時間は丸めない・画面の更新が遅れたら過ぎた時間の分だけ進める。繰り返しの仕組みの上に載る語も同じ。
- **繰り返しの仕組みの形**（型・関数の名前）: 設計の段で決まりしだい、ここへ書き足す→ 下の「繰り返しの仕組みの形」に記した（2026-10-05）。

### 繰り返しの仕組みの形（2026-10-05 設計の確定時点・`animated-image-playback` の design.md から写し。実装で変わったら同 spec が書き直す）

- 引き金: `LoopTrigger::Always { period_ms, laps }`（`crates/areka-seriko/src/table.rs`）。見分けは `is_always_interval`（`crates/areka-emo-compose/src/nesting.rs`）の 1 関数。
- 計算: `lap_of`・`always_at`（`crates/areka-seriko/src/timeline.rs`）。開始の時刻からの経過だけで今のコマを決める。
- 時計: 一番上は `LoopRuntime` の再生の表、部品は `PartClocks`。「見えたと分かった出来事の時刻で、乱数を引かずに生まれる」。時刻は `SerikoClock`（刻みと同じ時計）。
- 経過 0 の絵は合成が定義から描く（`rest_index`・`plan.rs` の `flatten_surface`）。seriko は経過 0 と同じコマを欄に載せない。欄の意味は 3 つ（`Cell`: 載っていない・コマ・消えている）。
- `always` は外形に全部の pattern が入る（`plan_extent.rs` の `flatten_extent`）。見える部品にも経過 0 の先が入る（`NestTable`）。
- 抽選の対象から外す場所は 2 つ: `LoopRuntime::on_tick` の抽選の輪と、`parts.rs` の `gate`。
- `bind+always` を入れるときは、`is_always_interval`（経過 0 を描くか・外形に数えるか）と「着せ替えの種類か」（`plan.rs` の `is_bind_interval`・`nesting.rs` の `bind_ids`・`parts.rs` の `gate`）の両方に載せ、着せ替えが無効の間は経過 0 の絵も描かない形にする。
- バルーンの面でも部品の経路が回る。時計の鍵に面の種類が入った。
