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
