# Brief: sound-canon-tags

## Problem

- **ゴーストの作者・利用者**: さくらスクリプトの正典には音を鳴らすタグ（`\_v`・`\_V`・`\8`・`\![sound,…]`）があるのに、areka は何も鳴らさない。効果音や BGM を持つゴーストは、黙ったまま動く。
- **開発者**（2026-10-28 `/kiro-discovery`・N3「音の層」）: 「areka に音の層を追加したい。現状、BGM、効果音、声は何も無い」。議題 8 で「正典の音のタグから作り、ノベルの拡張は pasta の依頼書の後に積む」「再生の土台は Media Foundation」「ノベルゲーム基盤の段の先頭」に決めた（原文と決めたことは `doc/ideas/novel-platform.md`）。

## Current State

起票のときの実測（main `d6054e4d`）:

- **鳴らす所が 0**: 音のクレート（rodio・cpal・PlaySound など）は無い。
- **字句だけ**: `\_v`・`\_V` は字句解析で見分けるだけ（`crates/areka-parsers/src/sakura/lexer.rs:291`・`lexer_bare_tag_tests.rs`）。`talk-script-metadata` の brief によれば、解析では `UnknownTag` になる。`\![sound,…]` は汎用の運び手で届くが、受け手が居ない。
- **プロパティは名前だけ**: `currentghost.sound(bgm).pause` など 18 葉を登記してある（`crates/areka-sylphya/src/vocab/dotted.rs`・`dotted_set_ledger_tests.rs`）。値は「音再生基盤」待ちで、登記の持ち主は `property-catalog-lists`（同 brief の 22 行目）。
- **音を待っている spec**: `parallel-visual-script`（v0.4.0・並走の台本で音を許すのに鳴らす所が無い・22 行目）と、`sakura-time-directives`（いつか・`\![sound,wait]` を「消費する者が現れるまで」範囲から外している・19 行目と 84 行目）。

正典（ukadoc・2026-10-28 に引き直した）:

- `\_v[ファイル名]`: wave・mp3・wma を再生する。相対パスは「実行したゴーストのフォルダ」/ghost/master から。ファイルが無ければ無視。
- `\_V`: `\_v` の再生が終わるまで待つ。既に終わっているか無ければ無視。
- `\8[ファイル名]`: 自ゴーストのフォルダの wave を再生する。
- `\![sound,play,ファイル名,オプション…]`（`\_v` と同じ・オプションは 2.4〜）・`\![sound,loop,ファイル名]`（`\_V` の対象にならない）・`\![sound,stop,ファイル名]`（省略で全部）・`\![sound,pause,ファイル名]`・`\![sound,resume,ファイル名]`・`\![sound,wait]`（`\_V` と同じ）・`\![sound,load,ファイル名,オプション…]`（先読み・2.4）・`\![sound,option,ファイル名,オプション…]`（再生中のオプションの変更・2.4）・`\![sound,cdplay,トラックNo.]`（音楽 CD）。
- プロパティ（2.8.72）: `currentghost.sound.count`・`currentghost.sound(要素名).{name,loop,playing,pause,position,preload,…}`。`playing`・`pause`・`position` は SET でも効く。プロパティの節は `\![sound,preload]` にも触れる。
- 写しの ukadoc はプロパティの節が古いことがある（2.8.80 で陳腐化）。オプション（`--volume`・`--seektime` など）とプロパティの全部の一覧は、要件の段で ukadoc の本物と照らして決める。

## Desired Outcome

- 正典の音のタグが、SSP と同じ意味で鳴り、止まり、待つ。
- `\_V`・`\![sound,wait]` の待ちが台本の時計（dola）の上で正確に効き、早送りや中断でも食い違わない。
- 18 葉のプロパティが本物の値を返し、SET の効くものは SET で動く。
- `parallel-visual-script` と `sakura-time-directives` の「鳴らす所が無い」が解ける。
- 音が鳴らない・ファイルが無い・形式が読めないときも、areka は落ちず、記録を 1 行残して先へ進む。
- 後でノベルの音（BGM のクロスフェード・区分ごとの音量・声）を、この上に積める形になっている。

## Approach

- **再生の土台は OS の Media Foundation**（2026-10-28 議題 8 の決定）。正典の wav・mp3・wma をそのまま読め、arm64 でも動き、今使っている `windows` クレートで呼べる＝ライセンスと cargo-deny の心配が無い。効果音を重ねて鳴らす混ぜ方（XAudio2 を併せるか）は設計の段で決める。
- **相対パスは `talk-script-metadata` の関数で解く**（台本を実行したゴーストの `ghost/master` が基準）。`talk-script-metadata`（v0.2.0）はこの spec より先に着地する。
- **音の再生は UI スレッドの外**（読み込みとデコードで描画を止めない）。

## Scope

- **In**: 正典の音のタグ（`\_v`・`\_V`・`\8`・`\![sound,play／loop／stop／pause／resume／wait／load／option…]`）・ukadoc のオプションのうち音にかかわるもの・18 葉のプロパティの値と SET・失敗の記録・決定論のテスト（待ち・止め・一時停止・ファイルが無いとき）。
- **Out**: 音楽 CD（`\![sound,cdplay]`）・動画の再生（`\![sound,play,test.mp4,…]`・pasta も「要るかどうかから決める」）・ノベルの音の拡張（BGM のクロスフェード・同じ曲を鳴らし直さない・区分ごとの音量・台詞と結び付く声・バックログからの聞き直し＝pasta の依頼書の後・roadmap の受け口の節）・音量の設定画面（windows-reactor 待ち）。

## Boundary Candidates

- タグを受けて再生へ渡す所（cue の受け手）。
- 再生の土台（Media Foundation の包み・UI スレッドの外）。
- 待ち（`\_V`・`\![sound,wait]`）と台本の時計（dola）の継ぎ目。
- プロパティの値（sylphya の登記に値を出す）。

## Out of Boundary

- ノベル用の音の命令とその名前（pasta の `novel-areka-contract` が決める）。
- ファイルを指すタグの相対パスの解き方そのもの（`talk-script-metadata`）。
- 並走の台本の届け方（`parallel-visual-script`）。

## Upstream / Downstream

- **Upstream**: `talk-script-metadata`（v0.2.0・相対パスの基準）・v0.2.0 の dola の時刻の土台（`dola-interrupt-ticks`・`talk-anchor-interrupt-ticks`＝待ちを正確に刻む）。
- **Downstream**: `parallel-visual-script`（音を鳴らす所）・`sakura-time-directives`（`\![sound,wait]`）・ノベルの音の拡張（受け口の節）・`areka-usage-stats` には関係しない。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `property-catalog-lists`（18 葉の登記の持ち主・値はこの spec が出す）・`talk-fast-forward`（早送りのときの音の扱い）・`text-reveal-fade`（同じ台本の時計に乗る）。

## Constraints

- 区分 C（ukadoc の拾い残し）・版は新しい段「ノベルゲーム基盤」の先頭（2026-10-28 議題 8）。
- 規模の見立て: M〜L（14〜20 タスク）。20 を超えるなら要件の段で、タグの受け口とプロパティで切る。Opus。
- ターゲットは x64 と arm64（Media Foundation の両方での動き）。i686 は helper の側の話で、音は本体が鳴らす。
- 時刻は正確に扱う（待ちを丸めない・更新が遅れたら過ぎた分だけ進める）。
- 失敗の経路は記録を残す（`warn!` を 1 行・黙って飛ばさない）。メッセージボックスは出さない。
