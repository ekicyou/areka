# Brief: talk-script-metadata

## Problem

- **ゴーストの作者**: さくらスクリプトの中でファイルを指すタグ（`\_b`・`\_v`・`\![open,file]`・`\j[file:///…]`・`\![set,tasktrayicon]` など）は、相対パスを「実行したゴーストのフォルダ」/ghost/master（SHIORI の DLL がある所）から読むのが正典。areka では、台本がどのゴーストのものかを台本自身が持っていない。そのため、受け手がそれぞれ「今居るゴースト」を自分で引いている。ゴーストが 1 体だけで、トークの途中に切り替わらないあいだは合うが、複数のゴーストが同時に居るとき（`multi-ghost-runtime`）や、トークの途中で切り替えたときは、別のゴーストのフォルダを引く。
- **開発者**（2026-10-25 `/kiro-discovery`）: 「さくらスクリプトにファイル指定が含まれる場合、そのファイルのフォルダはカレントディレクトリが『ghost/master(SHIORI.DLLのある場所)』フォルダになる。スクリプト情報のメタデータとして、カレントディレクトリを持たせてた方がよいかもしれない」「多分、台本ごとに台本が基準とするカレントディレクトリとか送信元が何かとかを特定できる情報があればよいと思うのです。実装時期はdolaとかの基礎整備をするタイミングかなと思っています」。

## Current State

起票のときの実測（main `7b61252e`）:

- **台本に付いて回る情報**: `StartTalk { talk_id, script, epilogue, origin }`（`crates/areka-talk/src/lib.rs:43-48`）。`origin` は `ScriptOrigin { sender_type, sstp, level }`（`crates/areka-talk/src/origin.rs:121`・`Copy`）で、送り手の種類（ゴースト自身・SSTP・MCP など）と信用度だけを持つ。**どのゴーストが実行するか・どのゴーストや道具が送ったか・基準のフォルダはどこにも無い**。
- **通り道**: kanade の各段が `StartTalk` を作る（`schedule/mod.rs:111` の `start_talk`・`talk_origin::stamp` が origin を押す）→ `TalkCommand::Start`（`crates/areka-kanade/src/actor.rs:491`）→ dispatcher が受け手ごとに `begin_talk(talk_id, origin)` を呼ぶ（`crates/areka-ghost/src/dispatcher.rs:341`・`sink.rs:44`）→ `areka-sakura` の `drive.rs:186-226` が解析して `CueSheet` へ（`origin` は捨てる・`CueSheet` は時刻と cue だけ）→ cue は全部の受け手へ流れる（`\![…]` は汎用の運び手）。
- **相対パスを今解いているのは `\![open,file]` だけ**: `crates/areka/src/readme/opener.rs:352-364`。基準は実行の時点で 1 つだけの `GhostSlot` から取る（`opener.rs:174-194`）。トークからは取らない。
- **ファイルを指すほかのタグは未実装**: `\_b`・`\_v`・`\_V` は解析で `UnknownTag`。`\![sound]`・`\![set,tasktrayicon]` は受け手が居ない。`\f[name,x.ttf]` はフォントファイルを記録だけして飛ばす。`\![execute,install,path]` は相対パスを断る（正典どおり）。
- **フォントの探し場所**: `box_font_search_dirs(shell_dir, ghost/master)`（`crates/areka/src/emo2_boot/shell_box_assets.rs:59`）が起動の時点で 1 回決まる（台本ごとではない・今はテストしか読まない）。
- **プロセスのカレントディレクトリ**: areka は変えない。32bit の SHIORI の helper の子プロセスだけが ghost/master で起きる（`crates/shiori-host32-host/src/process_host.rs:274`）。
- **MCP の `sakurascript`・`raise_event`**: 1 つだけの `GhostSlot` のゴーストへ送る（`crates/areka/src/mcp/sakurascript.rs`）。ネットワークの SSTP の受け口はまだ無い。

ukadoc の基準（逐語・`list_sakura_script.html`）:

- `\_b[ファイルパス,x,y]`「[実行したゴーストのフォルダ]/ghost/masterが基準のフォルダとなる。」（例 `\_b[..\..\shell\master\surface0.png,0,15,opaque]`＝ghost/master の外も指せる）
- `\_v[ファイル名]`「[実行したゴーストのフォルダ]/ghost/masterから相対パス、または絶対パスで指定可。」
- `\![open,file,ファイル名]`「[実行したゴーストのフォルダ]/ghost/master/からの相対パスか、絶対パスで指定。」
- `\![set,tasktrayicon,…]`「[実行したゴーストのフォルダ]から、または[実行したゴーストのフォルダ]/ghost/masterからの相対パスが使用可能。」
- `\![set,wallpaper,…]`「相対パスは[実行したゴーストのフォルダ]/ghost/master、[実行したゴーストのフォルダ]、SSPのフォルダの順に探す。」
- `\![execute,extractarchive,…]`・`compressarchive`・`filewatch`「相対パス指定の場合はghost/master/以下の相対パスと解釈される。」

基準は「台本を送ったゴースト」ではなく「**実行した**ゴースト」。

## Desired Outcome

- 台本ごとに、その台本を特定できる情報（**台本の付帯情報**）が付く。少なくとも ⑴ 実行するゴーストと、その置き場（ghost/master とゴーストの最上位）⑵ 送り手が何か（今の `ScriptOrigin` の種類と信用度に加え、どのゴースト・どの道具から来たか）。
- 付帯情報は台本が始まる時点で決まり、そのトークの終わりまで変わらない。トークの途中で切り替えても、複数のゴーストが居ても、ファイルを指すタグは実行したゴーストの ghost/master から読む。
- ファイルを指すタグの相対パスを解く関数は 1 つ。受け手が自分で「今居るゴースト」を引く形は無くなる。
- 今ある受け手 `\![open,file]` がこの関数に乗り換わり、`GhostSlot` を引かなくなる。

## Approach

**台本に付帯情報を持たせ、今の origin の通り道で受け手へ届ける**（2026-10-25 開発者と `/kiro-discovery` で決めた）。

- 付帯情報を `StartTalk` に持たせ、`begin_talk` で受け手へ渡す（今の `origin` が既に通っている道）。付帯情報を `CueSheet`（dola）の側にも持たせるかは要件の段で決める（開発者「dola とかの基礎整備をするタイミング」・✅ `dola-interrupt-ticks` が作り直した、台本の起点を配る所と同じ継ぎ目）。
- 相対パスの解き方は共通の関数 1 つ。絶対パスはそのまま。相対パスは「ghost/master だけ」か「ghost/master → ゴーストの最上位」の探す順で引く（どのタグがどちらかは ukadoc の各項目に従う）。`..` で ghost/master の外へ出るパスは断らない（ukadoc の例が正典）。環境変数の展開は `\![open,file]` だけの決まりなので、opener の側に残す。
- MCP の `sakurascript`・先々の SSTP も、喋るゴーストの置き場を基準にする（ukadoc の「実行した」）。送り手のゴーストや道具は「送り手」の欄に持つ。

採らなかった案:

- **`ScriptOrigin` に置き場を入れる**: origin は「誰が送ったか・どこまで信用するか」の語で、置き場とは別物。`PathBuf` を持つと `Copy` を失う。ただし「送り手が何か」を足す先が `ScriptOrigin` か付帯情報の別の欄かは要件の段で決める。
- **プロセスのカレントディレクトリを ghost/master へ替える**: プロセスで 1 つしかなく、複数のゴースト・台本の並走で取り合いになる（SHIORI を別プロセスで動かすと決めた理由と同じ・`shiori4-api` の brief）。
- **受け手ごとに `GhostSlot` を引く今の形のまま**: 複数のゴーストとトークの途中の切り替えで誤る。

## Scope

- **In**: 台本の付帯情報の型（実行するゴーストと置き場・送り手）・kanade で台本を作る所すべてで付帯情報を埋める（SHIORI の応答・選択肢の台本・MCP の送り込み・起動と終了と切り替えの台本）・`begin_talk` で受け手へ渡す・相対パスを解く共通の関数・`\![open,file]` の乗り換え・決定論のテスト（途中の切り替えで基準が変わらないこと、探す順の 2 通り、絶対パスと `..`）。
- **Out**: ファイルを指すタグそのものの実装（`\_b`・`\_v`・`\![sound]`・`\![set,tasktrayicon]`・`\f` のフォントファイル＝各 spec の持ち物）・複数のゴーストを同時に動かすこと（`multi-ghost-runtime`）・ネットワークの SSTP の受け口（`sstp-receive`）・プロセスのカレントディレクトリを替えること・本体のフォルダを探す順に入れること（`\![set,wallpaper]` の 3 番目＝areka 全体で 1 つなので付帯情報に持たせない・そのタグを作る spec が持つ）。

## Boundary Candidates

- 付帯情報の型と、kanade で埋める所（`areka-talk`・`areka-kanade` の `schedule/`）。
- 受け手へ渡す口（`areka-ghost` の `sink.rs`・`dispatcher.rs`）と、`CueSheet` へ持たせるか（`dola`・`areka-sakura` の `drive.rs`）。
- 相対パスを解く関数と、最初の使い手（`crates/areka/src/readme/opener.rs`）。

## Out of Boundary

- 各タグがファイルを読んだ後の処理（画像・音・フォント・アイコンの読み込み）。
- 送り手で台本を断る・制限する決まり（信用度の扱いは今の `SecurityLevel` のまま）。
- `check_script` でファイルの在る無しを診ること（`check-script-arg-checks` の範囲）。

## Upstream / Downstream

- **Upstream**: 働きの上の前提は無い。
- **Downstream**: `balloon-inline-image`（`\_b[…,inline]`）・`balloon-font-file`（`\f` のフォントファイル）・`tasktray-ghost-icon`（`\![set,tasktrayicon]`）・`multi-ghost-runtime`（送り手のゴーストと実行するゴーストの区別）・`sstp-receive`・`ghost-call`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `multi-ghost-runtime`（`GhostSlot` をゴーストごとの表へ・`KanadeNotice` が送り手のゴーストを持たない件）・`mcp-strict-errors`（C12・`areka-talk` の `lib.rs` と kanade の `msg.rs` を触る）・`parallel-visual-script`（台本の並走）。

## Constraints

- 区分 A（開発者が起票）・版は **v0.2.0**（wintf・dola の基幹の整備・開発者「dola とかの基礎整備をするタイミング」）。
- 規模の見立て: S〜M（6〜10 タスク）。Opus。
- 付帯情報は台本が始まる時点で決め、トークの途中で書き換えない（途中の切り替えに強い形）。
- 失敗の経路は記録を残す（相対パスが解けない・置き場が無いときは `warn!` を 1 行・黙って飛ばさない）。

## 2026-11-27 棚卸㊹の再測定（main ccd790fc）

- **前提の変化**: 見直しの spec と本体の crate 分けは全部着地した。台本に付いて回る情報は今も送り手の種類と信用度だけ（`crates/areka-talk/src/lib.rs` の `StartTalk` に `strict` の欄が増えた・`origin.rs` の `ScriptOrigin`）。相対パスを解くのは今も `\![open,file]` だけで、受け手は `crates/areka-session/src/readme/opener.rs`（173 行目で今居るゴーストの表 `GhostSlot` を引く）へ移った。`dola-interrupt-ticks` は台本の起点を配る所を作り直し、`talk-anchor-interrupt-ticks` の仕事はそこに入った（あちらに残る仕事は無い）＝前に挙げた「同じ継ぎ目」の相手は居なくなった。
- **触るファイル**: 台本の情報の型 `crates/areka-talk/src/{lib.rs 136, origin.rs 189}`。kanade で台本を作る所 `crates/areka-kanade/src/schedule/` の `mod.rs`（684・132 行目の `start_talk`）・`talk_origin.rs`・`sstp.rs`・`steady.rs`（926）・`boot.rs`・`change.rs`・`close.rs`・`translate.rs`・`balloon_events.rs`・`steady_choice_script.rs`・`types.rs` と `actor.rs`（873・488 行目で送る）・`msg.rs`（607・MCP の送り込み `SstpSend` に送り手を持たせるなら）。実行するゴーストの置き場を kanade へ渡す所 `crates/areka-ghost/src/config.rs`（350・`resolve_kanade_config`）。受け手へ渡す口 `crates/areka-ghost/src/{dispatcher.rs 446, sink.rs 274}`。受け手 `crates/areka-runtime/src/{talk_lifecycle.rs 355, user_break_cue.rs 222}`。台本を解く所 `crates/areka-sakura/src/{drive.rs 619, contract.rs}`。最初の使い手 `opener.rs`（388）。MCP の送り込み `crates/areka/src/mcp/sakurascript.rs`（92）。`CueSheet` にも持たせると決めたら dola の `crates/dola/src/cue/`。フォントの探し場所は `crates/areka-runtime/src/shell_box_assets.rs`（読むだけ）。上限の近くのファイルは無い。依存の席も文字の層の新しいファイルの席も使わない。
- **規模**: 7〜11 タスク（台本を作る所が kanade の 10 ファイルほどに分かれている分で少し上げる）。
- **先に要るもの**: 無し。
- **同じウェーブに置けない相手**: `talk-thread-count-growth`（トークの側を直すときだけ `dispatcher.rs`・`drive.rs`＝あちらの約束で外せる）。`sylphya-sqlite-kvs` とは、置き場を kanade へ渡す所を `crates/areka-ghost/src/runtime.rs` にすると重なる＝「この spec は `runtime.rs` に触らず、`config.rs` の `resolve_kanade_config` で置き場を持たせる」を約束にする（置き場は同じ所の `mount` から取れるので設計は曲がらない）。後のウェーブの `shiori4-api`（kanade の `msg.rs`・`schedule/mod.rs`・`sstp.rs`・`steady.rs`・`actor.rs`）・`sstp-receive`・`multi-ghost-runtime`（今居るゴーストの表）とは別のウェーブ。ウェーブ C28 のほかの 4 本（`draw-methods-canon`・`balloon-text-visual-tree`・`ghost-switch-balloon-fallback`・`instant-to-interrupt-ticks`）とは触るファイルが重ならない。
- **優先度の区分**: A（開発者の起票・「台本ごとに…特定できる情報があればよい」）。
- **要件定義のモデル**: Opus（今の送り手の情報の通り道に欄を足すだけ）。
- **分割の案**: 無し。
- **見つけた穴・古くなった記述**: Current State の道が古い＝`crates/areka/src/readme/opener.rs` は `crates/areka-session/src/readme/opener.rs`、`emo2_boot/shell_box_assets.rs` は `crates/areka-runtime/src/shell_box_assets.rs`、本体の受け手 `emo2_boot/{talk_lifecycle.rs, user_break_cue.rs}` は `crates/areka-runtime/src/` へ移った。Adjacent の `mcp-strict-errors` は着地済み。
