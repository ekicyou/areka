# Brief: mcp-strict-errors

> 2026-09-29 `/kiro-discovery` で起票。SSP MCP 移植の **4 段目（最後）**。並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)。file:line は起票時（main `c3876110`）＝着手時に引き直す。

## Problem

AI が書いた台本は、存在しない surface 番号や綴りを誤ったタグを平気で含む。SSP の `strict` を付けると、そうした箇所がエラーログに残り、AI は `get_log(log_type=error, since_id=…)` で自分の誤りを知って直せる。これが無いと、台本は黙って一部だけ動き、AI は何が悪いか分からない。

## Current State

- `sakurascript`／`raise_event` の `strict` の引数は `mcp-kanade-tools` が受けて下流へ渡す口まで作る。エラーログは `mcp-log-history` が作る。
- 正典（SSTP の `Option: strict`・ukadoc spec_sstp）: 「SakuraScript の解釈に失敗した箇所をエラーログに記録」。対象は存在しないサーフェス（`\s`）・アニメーション（`\i`）・バルーン（`\b`）の指定、未知のタグ・`\![コマンド]`・`\&[実体参照]`。
- areka の各消費者（sakura の解釈・seriko・emo・`\!` の汎用キャリアの台帳 `emo2_boot/consumer_ledger.rs`）は、未知や不在を今は `warn!` などで個別に扱っている（着手時に全数を引き直す）。

## Desired Outcome

- strict の台本の再生中に、上の 6 類の失敗が起きた箇所ごとにエラーログへ 1 件ずつ記録される（種別・ゴースト名・何が無かったか）。strict でない台本では記録しない（今のログの振る舞いは変えない）。
- `sakurascript` の返事の `since_id` が、その台本の記録より前の id を指している（AI が差分だけ読める）。

## Approach

台本に「strict」の印を持たせて再生の経路へ通し、各消費者の失敗の分岐で印を見て記録する。記録の口は `mcp-log-history` の error 種別。

## Scope

- **In**: 印の通り道（台本 → talk → 各消費者）・6 類の検出点・記録・`since_id` の約束・決定論テスト（6 類それぞれの赤と緑）。
- **Out**: SSTP の `Option: strict`（SSTP は予約。同じ印を将来 SSTP が立てればよい）。

## Boundary Candidates

- 印を運ぶ経路（kanade → sakura → 消費者）と、記録する口（ログ）の境。

## Out of Boundary

- 失敗の振る舞いそのものを変えること（無い surface を出さない、などの今の挙動は据え置き）。

## Upstream / Downstream

- **Upstream**: `mcp-log-history`・`mcp-kanade-tools`。
- **Downstream**: SSTP（予約）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 未知のタグ・`\!` の消費者を持つ α 後の正典の spec 全般（着手時に並走が無いか確かめる）。

## Constraints

- 「全項目に○○」型の要件になりやすい＝検出点の全数をタスク単位でなく spec 単位の表で持ち、表と実装の一致を検査で判定する（記憶 blanket-requirements-invisible-to-per-task-review・checks-must-judge-not-just-print）。
- 規模 M。


---

## 2026-10-08 `mcp-author-tools` からの申し送り

`mcp-author-tools` が、台本を再生せずに確かめる独自のツール `check_script` を作った。strict の記録はこれに揃える（要件の議論で「先に着地する側が種類の名前と文面の正本を作り、後の側が揃える」と決めた）。

- **種類の名前と文面は `doc/ssp-mcp/areka-tools.md` に揃える**（「⑶ 診断の種類と文面」が正本）。上の 6 類との対応:

  | 6 類 | 種類の名前 | 文面 |
  |---|---|---|
  | 無い surface（`\s`） | `missing_surface` | `no such surface in the current shell; playback does not change the surface` |
  | 無いアニメーション（`\i`） | `missing_animation`（名前だけ予約。areka が `\i` に対応するまでは `unknown_tag`） | 未定（対応するときに `areka-tools.md` へ足す） |
  | 無いバルーン（`\b`） | `missing_balloon` | `no such balloon ID in the current balloon; playback does not change the balloon` |
  | 未知のタグ | `unknown_tag` | `areka does not know this tag; playback drops it` |
  | 未知の `\!` | `unknown_command` | `no part of areka handles this \! command; playback ignores it` |
  | 未知の `\&` | `unknown_entity`（名前だけ予約。areka が `\&` に対応するまでは `unknown_tag`） | 未定（同上） |

  `check_script` はほかに `unreadable_argument`（閉じていない括弧・既定の値へ落とした引数）と `ignored`（受け取るが何もしないもの）も返す。strict の 6 類に入れるかは本 spec の議題にしてよいが、入れるなら名前と文面は同じものを使う。文面の定数は `crates/areka/src/mcp/check_script_judge.rs` の `MSG_*`（文書を写したもの・一致はテストで見ていない）。
- **再生中の判定は、検査と同じ関数を引けば同じ答えになる**。`check_script` は判定を自分で持たず、再生が使う関数の答えを種類へ写しているだけ（`check_script_judge.rs` の `diagnose`）。
  - 未知のタグ・閉じていない括弧・既定の値へ落とした引数・選択肢マーカー → `areka_parsers::sakura::parse_noted` が命令ごとに付ける印（`ReadNote::UnknownTag`・`Unclosed`・`ArgumentDefaulted`・`MarkerIgnored`）と、台本の中の位置（`Read::span`・バイト）。再生が使う `parse` は `parse_noted` から命令だけを取り出したものなので、経路は 1 本。
  - 未知の `\!` → `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical()` を引いて `consumer_of(名前, 第 1 引数)` が `None`。表は 23 組で、各受け口の選別との一致は `consumer_ledger_agreement_tests.rs` が固定している。
  - 無い surface・バルーン → `SurfaceResolver::resolve`・`resolve_balloon_key`（`areka-seriko`）と、表示の層の `EmoPresenter::has_surface`・`surface_ids`（`areka-emo-present`）。
  - `\f` の知らないキー・キーなし → `areka_emo_text::look::apply_font_tag` の `Err` の `failure()`（`FontTagFailure::UnknownKey`・`NoKey`）。
- **検査と再生中の記録の違いとして残るもの**: `check_script` は `\e`・`\-` の後ろも診るが、再生は `compile` がそこで止まるので、再生中の記録には出ない。`check_script` は字面で決まらないもの（シェルやバルーンの切替の後ろの `\s`・`\b`、名前の形の `\b[名前]` など）を診ないが、再生中は実際の答えが分かるので記録できる。
- **`INSTRUCTIONS` の重なり**: `mcp-author-tools` は `handler.rs` の `INSTRUCTIONS` に独自のツールの 1 文を「not implemented yet」の文の前に足した。本 spec は最後の「not implemented yet」の 1 文だけを消し、独自のツールの 1 文は残す。2026-10-08 の時点で `NG:not implemented yet` を返すのは `sakurascript`・`raise_event`・`reload` の 3 本（`get_status` は `mcp-get-status` で着地済み）。

## 2026-10-22 棚卸㉘の申し送り（main 52b014de）

- 件を 1 つ足す（持ち主が居なかった残り物・完了 `mcp-receiver-bugs` の research.md 9 節と要件の「範囲外」）: `params` が入れ物（オブジェクト）でない要求（`7`・`[1]`・`"x"`）に、受け口は 415 の素の文で答え、中継は -32600 を返す＝-32602（引数の誤り）にならない。誤りの答えを正典どおりにそろえるこの spec で一緒に直す。research が勧めた `doc/ssp-mcp/transport-diff-areka.md` への行の追記も一緒に。

## 2026-10-23 要件の討議の裁定と、並走の相手との順（main 15d8b0f3）

- **裁定**（開発者）:
  - 議題 1: 「手抜きはダメよ？並走相手と相談して実行順を決めてブロッカーとすること。実装は行うこと。」＝案 2（再生の途中まで印を運び、実際に起きた誤りを記録する）。入口で診る案 1 は取らない。
  - 議題 3: 「議題３も。スコープ増やす必要があるなら、一度このスペックはPRして次のウェーブに回すべき。」＝`params` がオブジェクトでない要求への答えは rmcp の中で直す（rmcp の手前で本文を読んで自分で答える形は取らない）。新しい版で直っていれば上げ、直っていなければ上流へ直しを出し、取り込みまでは `[patch.crates-io]` で持つ＝依存の席を使う。
  - ほかの議題: 「あとは承認。完成度が最優先です。」
- **ウェーブ**: C11（⑦）から外した。要件まで（requirements.md・research.md）を PR で main に入れ、設計は待ちが解けてから `/kiro-design mcp-strict-errors` で始める。
- **待ち（ブロッカー）と順**（10-23 に並走の相手と合意）:
  - ① `descript-window-keys` が先に着地する。① が `consumer_ledger.rs` で変えるのは、表の末尾の 2 行（`lock`／`unlock` の `balloonmove`）・手書きの数 36→38・`samples()` と見本の 2 件だけ。`consumer_of` と「未登記」を扱う所の形は変えない（設計で変わったら知らせると約束）。
  - ② `mouse-click-wheel-events` が先に着地する。② が kanade で変えるのは `msg.rs` のマウスの知らせ（`MouseEventKind` とボタンの型）と `schedule/{events,steady,types,mod}.rs`・新しい `mouse_click.rs`。`StartTalk`・SSTP 系の知らせ・`sstp.rs`・`translate.rs`・`actor.rs`・areka-talk・sakura・seriko・emo-present・`consumer_ledger.rs` には触らない＝本 spec は ② の後に `msg.rs` の別の型へ足す。
  - 依存の席（`Cargo.lock`）は 1 ウェーブに 1 本＝本 spec を入れるウェーブでは、ほかに依存を足す spec と版上げを置かない。
  - そのほか、次のウェーブを組む棚卸で重なりを測り直す相手: `presenter/show.rs` の `animated-surface-log-flood`・`present-emit-tail-latency`、seriko の `actor.rs` と `consumer_ledger.rs` の `script-small-tags`、`consumer_ledger.rs` の鎖（`stay-on-top-directives`・`shell-scale-linkage`・`balloon-communicatebox`）、kanade の `msg.rs`・`schedule/sstp.rs` の `sstp-receive`・`multi-ghost-runtime`。
- **規模**: M〜L（13〜19＋rmcp の直し 1〜3）。20 を超えたら棚卸㉑の切り方（⒜ 印の通り道と記録の口＋`\s`・`\b`・`\i` ⒝ 未知のタグ・未知の `\!`・`\&`＋読む段の 3 種類）に、rmcp の直しを ⒜ の前へ置く。
- **要件定義のモデル**: Opus のまま。設計は Opus で十分（印の運び方は `StartTalk.origin` が先例・rmcp の直しは前例 `image-webp` の `[patch.crates-io]`）。

## 2026-10-24 棚卸㉚の再測定（main 9318cdb0）
- **前提の変化**: 待ちの 2 本が着地した＝設計（`/kiro-design mcp-strict-errors`）を今すぐ始められる。① `descript-window-keys`（10-24）は `emo2_boot/consumer_ledger.rs` に約束どおり表の末尾 2 行（`lock`／`unlock` の `balloonmove`）を足しただけ（台帳は 38 行・ファイルは 496→504 行・`consumer_of` の形は同じ）。同 spec は seriko の `actor.rs`（768→799 行）に、シェルの切替で今の面が新しいシェルに無いとき戻り先の面へ替える分岐と警告（`log_shell_fallback`）も足した＝台本の `\s` の誤りではないが、要件 4.1（切替の後ろの `\s`）の検出点の表で「記録するか・しないか」を決める行が 1 つ増える。② `mouse-click-wheel-events`（10-24）は kanade の `msg.rs` にマウスの型だけを足した（544→575 行・`StartTalk` と SSTP の知らせには触れていない）。
- **rmcp の直しの席**: rmcp は `crates/areka-mcp`・`crates/areka-mcp-bridge`・`crates/real-run-kit` の 3 つの `Cargo.toml` が `=3.5.0` で固定している。版を上げる（研究の時点で最新 3.7.0）なら 3 つとも書き換え、3 クレートの追随が要る。直しを持つなら、根の `Cargo.toml` の `[patch.crates-io]`（91 行目・今は `image-webp` の 1 行）に並べ、`deny.toml` の `allow-git`（61 行目）に名指しで足し、`THIRD-PARTY-NOTICES.md`（443 行目 `rmcp 3.5.0`）を合わせる。直しの枝の版が固定の `3.5.0` と合わないと cargo は黙って公開版を使う＝枝を 3.5.x に置くか、3 つの固定も一緒に変える。どちらも `Cargo.lock` の席を使う（1 ウェーブに 1 本）。
- **触るファイル**: `crates/areka/src/mcp/{sakurascript,raise_event,check_script}.rs`（112・95 ほか）と兄弟のテスト・`check_script_judge.rs`（218）・`crates/areka-mcp/src/handler.rs`（178・案内文は 39 行目のまま）と受け口のテスト・中継の `stdio.rs`（147）・`errors.rs`（54）と兄弟のテスト・`crates/areka-talk/src/lib.rs`（133）・kanade の `msg.rs`（575）・`schedule/{sstp,translate}.rs`（150・348）・`crates/areka-sakura/src/{compile,drive}.rs`（481・576）・seriko の `actor.rs`（799）と `resolve.rs`（314）・`crates/areka-emo-present/src/presenter/show.rs`（426）・`consumer_ledger.rs`（504）・`doc/ssp-mcp/{areka-tools,transport-diff-areka,bridge-diff-areka}.md`・検出点の表と一致の検査（新）・上の rmcp の席。1,000 行に近いファイルは無い。
- **規模**: 14〜22（13〜19＋rmcp の直し 1〜3）のまま。20 を超えたら 10-23 の節の切り方。
- **先に要るもの**: 働きの上では無い（待ちは解けた）。同じウェーブに置けない相手（v0.1.0）: `consumer_ledger.rs` の `stay-on-top-directives`・`script-small-tags`・`shell-scale-linkage`・`balloon-communicatebox`・`balloon-markers`、seriko の `actor.rs` の `script-small-tags`、`presenter/show.rs` の `animated-surface-log-flood`、`areka-talk` の `balloon-communicatebox`、`Cargo.lock` を使うほかの spec。`bridge-store-user-root`（中継は向こうが `config.rs`・こちらが `stdio.rs`・`errors.rs`）・`startup-install-option`・`default-balloon-selfmade` とは 0。emo-text に新しいファイルは足さない。
- **中の待ち**: 設計の段の SSP の測り（打つ前に毎回開発者の許し）・rmcp の上流へ直しを出す許し（取り込みは待たない＝`[patch]` で持つ）。実機の確かめは `real-run-kit` の試験 1 本。静かな机は要らない。
- **優先度の区分**: A のまま（開発者「ssp mcp tool の完全移植のための spec 群を立ち上げて」）。
- **要件定義のモデル**: Opus のまま。設計も Opus（印の運び方は `StartTalk.origin`、rmcp の直しは `image-webp` の `[patch.crates-io]` が先例）。
- **分割の案**: 10-23 の節のまま。
- **見つけた穴・古くなった記述**: research.md 10 節の rmcp の席の一覧は `crates/areka-mcp/Cargo.toml` だけを挙げる＝版を上げるならほかの 2 つの `Cargo.toml` も足して読む。10-08 の申し送りの「6 類」の表と「表は 23 組」は古い（要件 2.6 が 7 種類を決め、台帳は 38 行）＝要件が正本。旧節の「文面の定数と文書の一致をテストで見ていない」は要件 7.1（文書の表を読んで比べる）が引き取った。
