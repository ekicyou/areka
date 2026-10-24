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

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 棚卸⑳では個別の再測定をしていない（`mcp-tool-entrances` が、各 spec の触るファイルを設計で固定する）。着手は `mcp-tool-entrances` の完了の後で、そのとき接触ファイルを照合する。

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M〜L（12〜18 タスク・6 類それぞれの赤と緑＋印の通り道＋表の検査）。20 を超えたら、⒜ 印の通り道と記録の口＋`\s`・`\b`・`\i` の 3 類 ⒝ 未知のタグ・未知の `\!`・`\&` の 3 類、に切る（⒜ → ⒝）。
- 前提の状態: **未**。`mcp-kanade-tools`（`strict` を受けて下流へ渡す口）と `mcp-log-history`（error 種別の履歴）がどちらも未着手。`mcp-tool-entrances`（PR#223）は着地済み。
- 崩れた前提／古くなった位置:
  - `strict` の引数は型まで来ている: `crates/areka-mcp/src/tools/sakurascript.rs`・`raise_event.rs` の `Args.strict: Option<bool>`（tool-entrances が完成）。アプリ本体側 `crates/areka/src/mcp/{sakurascript,raise_event}.rs` は `mcp-kanade-tools` が中身を入れた後に本 spec が引き継ぐ（干渉台帳で直列と決まっている）。
  - 台本の起動の契約 `StartTalk` の正本は `crates/areka-talk`（`areka_kanade::talk` は再エクスポート）＝「台本に strict の印を持たせる」なら `areka-talk` と、それを読む再生側（`areka-ghost` の dispatcher・`areka-sakura` の drive）を通る。
  - 検出点の候補（着手時に全数を引き直す）: 未知のタグ＝`areka-parsers/src/sakura/model.rs` の `Instruction::Raw` を受ける所／`\s`・`\b` の不在＝`areka-seriko`（`resolve.rs` の注記「呼び手（actor）が warn!＋skip」）と `areka-emo-present`（`presenter/show.rs` ほか）／`\i` の不在＝`areka-seriko`／未知の `\!`＝`crates/areka/src/emo2_boot/consumer_ledger.rs` の `consumer_of` が `None` を返す所／`\&[…]`＝消費者の有無から確かめる。
  - `translate-pipeline` が台詞を `OnTranslate` へ通すようになった＝strict の印は翻訳の後の台本にも付いたまま運ぶ必要がある（`schedule/translate.rs`）。
- 触るファイル（並走の照合用・着手時に確定）:
  - `crates/areka/src/mcp/{sakurascript,raise_event}.rs` と各 `_tests.rs`（`mcp-kanade-tools` の後）
  - `crates/areka-talk/src/`（印）・`crates/areka-kanade/src/`（`msg.rs`・外からの台本の処理＝`mcp-kanade-tools` が作る新規ファイル・`schedule/translate.rs`）
  - `crates/areka-sakura/src/{compile,drive}.rs`・`crates/areka-seriko/src/{actor,resolve}.rs`・`crates/areka-emo-present/src/presenter/show.rs`・`crates/areka/src/emo2_boot/consumer_ledger.rs`
  - 記録の口（`mcp-log-history` で着地済み: target `areka::log::error` の info 以上で出す・欄 `ghost`・`label`。正本は `doc/ssp-mcp/log-convention.md`。出した後の記録を見分けるのは `crate::log_history::last_id()`）
  - 検出点の表（spec 単位・新規）と、表と実装の一致を判定する検査（新規）
- 議題（答えで作業が変わるものだけ）:
  - 印を台本に載せて再生側の各消費者まで運ぶか、kanade が「strict の talk の ID」を覚えて消費者の失敗の記録を talk の ID で拾うか（前者は `areka-talk`・dispatcher・sakura・seriko・emo を貫く／後者は消費者の記録に talk の ID が要る）。
- 見つけた穴: なし。並走の注意＝`consumer_ledger.rs` を `mcp-reload`・`makoto-dll-host`・`property-query-channels` も触る。kanade は kanade の進行の列の最後尾（`mcp-kanade-tools` の後）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模（タスク数）と切るかどうか: M〜L（12〜18）のまま。切らない（20 を超えたときの切り方は前回のまま）。
- 前提の状態: **半分**。`mcp-log-history`（PR#235）は着地＝error 種別の履歴と記録の約束（`doc/ssp-mcp/log-convention.md`・target `areka::log::error`）がある。`mcp-kanade-tools`（`strict` を受けて下流へ渡す口）は未着手＝待ち。`mcp-kanade-tools` から `get_status` を切り出しても本 spec の待ちは変わらない（`sakurascript`／`raise_event` の側を待つ）。
- 崩れた前提／古くなった位置:
  - 履歴の層は `crates/areka/src/log_history.rs`（`last_id()` もここ）。`since_id` の約束はこの層の通し番号で書ける。
  - `handler.rs` の `INSTRUCTIONS` の「not implemented yet」の 1 文を消すのは本 spec（roadmap の MCP の 3 段目の約束）。ただし `mcp-author-tools` が同じ `INSTRUCTIONS` に独自ツールの案内を足すので、2 本は `crates/areka-mcp/src/handler.rs` で重なる。今 `NG:not implemented yet` で答えるのは `get_status`・`sakurascript`・`raise_event`・`reload` の 4 本＝`mcp-reload` が本 spec より後に着地すると 1 文を消せない（順は `mcp-reload` → 本 spec を守る）。
  - 未知のタグの受け手は前回の位置のまま（`634032f6..f26aa1c1` で `areka-parsers`・`areka-sakura`・`areka-seriko` の解釈の腕に差分なし。`areka-seriko` は `surface-element-nesting` が `parts`・`table`・`looper`・`actor` を変えた＝`\i` の不在の検出点は着手時に引き直す）。
- 触るファイル（並走の照合用）: 前回の一覧に加えて `crates/areka-mcp/src/handler.rs`（`INSTRUCTIONS` の 1 文）と `server_protocol_tests.rs`（その文言を読む）。`crates/areka/src/log_history.rs` は読むだけ。
- 議題: 前回の 1 つ（印を台本に載せて運ぶか、talk の ID で拾うか）。加えて、`script-security-level` が台本に出どころの印を載せるなら、strict の印も同じ入れ物に載せるか（載せるなら `script-security-level` の後に回す）。
- 見つけた穴: なし。

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

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - 10-08 の `mcp-author-tools` からの申し送り（すぐ上の節）は今の main と合っている。案内文（`crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS`）は 4 文で、最後の 1 文が「not implemented yet」。`NG:not implemented yet` を返すのは `sakurascript`・`raise_event`・`reload` の 3 本。
  - `check_script` の判定（`crates/areka/src/mcp/check_script_judge.rs` の `diagnose`）が、6 類のうち今の areka で起こりうる 4 類（無い surface・無いバルーン・未知のタグ・未知の `\!`）を、再生と同じ関数を引いて答える形で既にある。
  - `\!` の受け取り手の台帳 `crates/areka/src/emo2_boot/consumer_ledger.rs` は 943 行（上限の近く）。
- 新しい議題（答えで作業が大きく変わる）: strict の記録を、⑴ 入口で台本の文字列を `diagnose` に通して出す（`sakurascript` は渡された台本・`raise_event` は SHIORI が返した台本）か、⑵ 起票時の案どおり印を台本に載せて再生の各所まで運ぶか。
  - ⑴ は `areka-talk`・`areka-sakura`・`areka-seriko`・`areka-emo-present`・`consumer_ledger.rs` に触らず、6〜10 タスク。字面で決まらないもの（シェルやバルーンの切替の後ろの `\s`・`\b`）は記録できず、`\e` の後ろも診てしまう。
  - ⑵ は 12〜18 タスク。再生中の実際の答えで記録できる。
- 触るファイル:
  - どちらでも: `crates/areka/src/mcp/{sakurascript,raise_event}.rs` と各 `_tests.rs`・`crates/areka-mcp/src/handler.rs`（案内文の最後の 1 文）・`server_protocol_tests.rs`・`doc/ssp-mcp/areka-tools.md`（種類と文面の正本）・`check_script_judge.rs`（文面の定数を分け合うなら）
  - ⑵ のときだけ: `crates/areka-talk/src/lib.rs`・kanade の `msg.rs`・`mcp-kanade-tools` が作る外からの台本のファイル・`schedule/translate.rs`・`crates/areka-sakura/src/{compile,drive}.rs`・`crates/areka-seriko/src/{actor,resolve}.rs`・`crates/areka-emo-present/src/presenter/show.rs`・`consumer_ledger.rs`・検出点の表と、表と実装の一致を判定する検査（新規）
- 規模: ⑴ なら S〜M（6〜10 タスク）・⑵ なら M〜L（12〜18 タスク）。
- 先に要るもの: `mcp-kanade-tools`（strict を受ける口と、`raise_event` の返り台本）・`mcp-reload`（案内文の 1 文を消す順）。⑵ で印を出どころと同じ入れ物に載せるなら `script-security-level` も先。同じウェーブに置けない相手: `mcp-user-response`・`mcp-shiori-query`（`handler.rs`・`areka-tools.md`）・`script-impact-tiers`（`check_script_judge.rs`・`areka-tools.md`）。
- 優先度の区分: A（開発者の指示「ssp mcp tool の完全移植のための spec 群を立ち上げて」）。
- 要件定義のモデル: Fable（上の議題は開発者の判断・SSP の strict の記録との差をどこまで許すか）。
- 分割の案: なし（20 を超えたときの切り方は棚卸㉑のまま）。
- 見つけた穴・古くなった記述: 棚卸㉑・㉒の「触るファイル」は ⑵ だけを前提にしている。文面の定数（`check_script_judge.rs` の `MSG_*`）と文書の一致はテストで見ていない（申し送りのとおり・本 spec が検査を足す候補）。

## 2026-10-11 棚卸㉔の再測定（main `7c5469f3`）

- 前提の変化:
  - `mcp-stdio-bridge`（10-11）が着地し、案内文（`crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS`）は外から読める定数になった。中継 `areka-mcp-bridge` は areka が居ないときの答えにこの文をそのまま使う＝最後の 1 文を消すと中継の答えも一緒に変わる。中継のテストは定数と比べているので、中継のファイルに直す所は無い。文は 4 文のままで、`NG:not implemented yet` を返すのも 3 本のまま。
  - `anchor-tag-canon`（10-10）が着地し、診断の種類は 6 つから 7 つになった（`unpaired_tag`＝アンカーの開きと閉じの対応の崩れ・文面 3 つ）。正本 `doc/ssp-mcp/areka-tools.md` の見本の表は 17 行。判定 `crates/areka/src/mcp/check_script_judge.rs` は 218 行で、対応の崩れは再生が使うのと同じ判定（`areka_sakura::pair_anchors`）の答え。再生も同じ所で同じ内容の警告を出している＝strict の記録に `unpaired_tag` を入れるかを、6 類の議題に足す。
  - `check_script` は `\e`・`\-` の後ろも診るが、`unpaired_tag` だけは後ろに出さない（正本に明記）。入口で `diagnose` に通す案 ⑴ の弱み「`\e` の後ろも診てしまう」は、この種類には当たらない。
- 触るファイル: 棚卸㉓のとおり。
- 規模: ⑴ なら S〜M（6〜10 タスク）・⑵ なら M〜L（12〜18 タスク）のまま。
- 先に要るもの: `mcp-kanade-tools`・`mcp-reload`（案内文の 1 文を消す順）。`check-script-arg-checks` が先に着地すると種類と文面の正本が増える＝後の側が揃える。同じウェーブに置けない相手: `mcp-user-response`・`mcp-shiori-query`・`mcp-receiver-bugs` の件 1（`handler.rs`）・`check-script-arg-checks`・`script-impact-tiers`（`check_script_judge.rs`・`areka-tools.md`）。
- 優先度の区分: A のまま。要件定義のモデル: Fable のまま。
- 見つけた穴・古くなった記述: 10-08 の申し送りの「6 類」の表に `unpaired_tag` の行が無い。文面の定数（`check_script_judge.rs` の `MSG_*`・10 個になった）と文書の一致は、今もテストで見ていない。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）

- 前提の変化: 版の段で **v0.1.0**（MCP の列の最後）。10-11 から触るファイルは動いていない（案内文は `crates/areka-mcp/src/handler.rs` の 36 行目の 1 定数・最後の 1 文が「not implemented yet」／判定 `crates/areka/src/mcp/check_script_judge.rs` 218 行）。
- 触るファイル: 棚卸㉓のとおり（案 ⑴＝入口で診る、案 ⑵＝再生まで印を運ぶ、で違う）。
- 規模: ⑴ なら 6〜10・⑵ なら 12〜18 のまま。
- 先に要るもの: `mcp-kanade-tools`（strict を受ける口と `raise_event` の返り台本）・`mcp-reload`（案内文の 1 文を消す順）。⑵ で印を出どころと同じ入れ物に載せるなら `script-security-level` も先。
- 同じウェーブに置けない相手（24 本の中）: `mcp-receiver-bugs`（`handler.rs`＝向こうは `on_custom_request` を足し、こちらは案内文の 1 文。行は違うが許す重なりに入らない）・`mcp-kanade-tools`（`crates/areka/src/mcp/{sakurascript,raise_event}.rs`）・`mcp-reload`（`doc/ssp-mcp/areka-tools.md`）。⑵ のときは `script-security-level`（`crates/areka-talk/src/lib.rs`・kanade の `msg.rs`・`schedule/translate.rs`・`crates/areka-sakura/src/drive.rs`）と、`consumer_ledger.rs` を触る `mcp-reload`・`ghost-call`・`multi-ghost-runtime` も。列の外: `check-script-arg-checks`・`script-impact-tiers`・`mcp-user-response`・`mcp-shiori-query`。
- 重なり 0: `property-name-case-fold`・`emo2-real-run-stability`・`dpi-realign-remembered-chain`・`baseware-root-list`。
- 優先度の区分: A のまま。
- 要件定義のモデル: **Opus**（10-15 の付け直し。⑴ と ⑵ の分かれ目は開発者に問う議題だが、⑴ の形は `check_script` の `diagnose` が先例として決めている＝先例で決まらない骨格の分かれ目には当たらない）。
- 分割の案: なし。⑵ で 20 を超えたら棚卸㉑の切り方。
- 見つけた穴・古くなった記述: 棚卸㉓・㉔の「Fable のまま」は古い。10-08 の申し送りの「6 類」の表に `unpaired_tag` の行が無い・文面の定数と文書の一致をテストで見ていない（どちらも㉔のまま）。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 版: **v0.1.0（棚卸㉗でゴールを組み直した後も残る）**。自動の試験で AI が自分の台本の誤りを `get_log(error)` から知るための仕組みで、ぱすた開発が SSP で使っている「strict の誤りの記録」に当たる（棚卸㉗の問い合わせの答え＝`sakurascript` の strict で着せ替えを試す）。
- 前提の変化: 10-16 から触るファイルは動いていない（`crates/areka-mcp/src/handler.rs` 144 行・案内文は 36 行目の定数で最後の 1 文が「not implemented yet」・`crates/areka/src/mcp/check_script_judge.rs` 218 行・文面の定数 `MSG_*` 10 個）。中継の側は `mcp-stdio-bridge-residue` が変えたが、案内文の定数を読む形は同じ＝最後の 1 文を消すと中継の答えも一緒に変わるのは前のまま。
- 触るファイル: 棚卸㉓のとおり（案 ⑴＝入口で診る・案 ⑵＝再生まで印を運ぶ）。
- 規模: ⑴ なら 6〜10・⑵ なら 12〜18。
- 先に要るもの: `mcp-kanade-tools`（strict を受ける口と `raise_event` の返り台本）・`mcp-reload`（案内文の 1 文を消す順）。
- 同じウェーブに置けない相手（v0.1.0）: `mcp-receiver-bugs`（`handler.rs`）・`mcp-kanade-tools`・`mcp-reload`（`areka-tools.md`）。⑵ のときは `script-security-level`・`window-link-directives`（`consumer_ledger.rs`）も。
- 優先度の区分: A のまま。要件定義のモデル: **Opus** のまま。
- 分割の案: なし。
- 自動の試験のための注記: `real-run-mcp-kit` の窓口が着地していれば、strict の記録の実機の確かめ（台本を流す → `get_log(error, since_id)`）をその窓口の 1 本の試験で書ける。
- 見つけた穴・古くなった記述: 10-08 の申し送りの「6 類」の表に `unpaired_tag` の行が無い・文面の定数と文書の一致をテストで見ていない（㉔・㉕のまま）。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 先に要る 3 本（`mcp-kanade-tools`・`mcp-reload`・`mcp-receiver-bugs`）は全部着地した＝今すぐ着手できる。MCP の 10 本のうち `NG:not implemented yet` を返すものは 0 本（`reload` は 10-22、`sakurascript`・`raise_event` は 10-19 に中身が入った）。strict の値は、kanade が「始めたトークの番号と結んで控える」形で既に運ばれている（kanade の `schedule/sstp.rs` の `StrictNote`・運行状態 `State::sstp_strict`・今は記録の欄とテストだけが読む）＝案 ⑴（入口で診る）も案 ⑵（再生まで運ぶ）もこの控えから始められる。`\!` の受け取り手の台帳 `crates/areka/src/emo2_boot/consumer_ledger.rs` は `mcp-reload` が中のテストを兄弟へ出して 960→472 行になった。
- 触るファイル: 案 ⑴＝`crates/areka/src/mcp/{sakurascript,raise_event}.rs`（112・95 行）と兄弟のテスト・`crates/areka-mcp/src/handler.rs`（178・案内文は 39 行目の定数）・`doc/ssp-mcp/areka-tools.md`・文面を分け合うなら `crates/areka/src/mcp/check_script_judge.rs`（218）・控えを読む口を足すなら kanade の `schedule/sstp.rs`。案 ⑵＝さらに `crates/areka-talk/src/lib.rs`・`crates/areka-sakura/src/drive.rs`（574）・`crates/areka-seriko/src/{actor,resolve}.rs`・`crates/areka-emo-present/src/presenter/show.rs`・`consumer_ledger.rs`（472）・kanade の `schedule/translate.rs`（300）と、検出点の表と一致の検査（新）。
- 規模: 案 ⑴ 6〜10・案 ⑵ 12〜18（変わらず）。
- 先に要るもの: 働きの上では無し。同じウェーブに置けない相手（v0.1.0）: 案 ⑴ は 0。案 ⑵ は `script-security-level`（`areka-talk`・`drive.rs`・`translate.rs`）と、`consumer_ledger.rs` に行を足す `stay-on-top-directives`・`script-small-tags`・`descript-window-keys`・`window-link-directives`。1 本だけの席（依存を足す・emo-text の新しいファイル）は取らない。静かな机・前後の全体テストの比べ・人の待ちは要らない（実機の確かめは `real-run-kit` の試験 1 本で書ける）。
- 優先度の区分: A のまま（開発者「ssp mcp tool の完全移植のための spec 群を立ち上げて」）。
- 要件定義のモデル: Opus のまま（案 ⑴ は `check_script` の判定が先例・案 ⑵ の「トークの番号で拾う」形は `StrictNote` が先例＝先例の無い骨格の分かれ目に当たらない）。
- 分割の案: なし（案 ⑵ で 20 を超えたら棚卸㉑の切り方）。
- 見つけた穴・古くなった記述: 案内文の最後の 1 文（「まだ作っていないツールは `NG:not implemented yet` で始まる答えを返す」）は、当てはまるツールがもう無い＝`crates/areka-mcp/src/handler.rs` 39 行目から今すぐ消せる（文を逐語で見るテストは無く、`server_protocol_tests.rs` と中継の `service_tests.rs` は定数を読むだけ）。roadmap はこの 1 文を本 spec が消す約束にしている＝先に消すかは棚卸の決め。roadmap の「`consumer_ledger.rs`（960 行）」は 472 行に読み替える。10-08 の申し送りの表に `unpaired_tag` の行が無い・文面の定数と文書の一致をテストで見ていないのは前のまま。

## 2026-10-22 棚卸㉘の申し送り（main 52b014de）

- 件を 1 つ足す（持ち主が居なかった残り物・完了 `mcp-receiver-bugs` の research.md 9 節と要件の「範囲外」）: `params` が入れ物（オブジェクト）でない要求（`7`・`[1]`・`"x"`）に、受け口は 415 の素の文で答え、中継は -32600 を返す＝-32602（引数の誤り）にならない。誤りの答えを正典どおりにそろえるこの spec で一緒に直す。research が勧めた `doc/ssp-mcp/transport-diff-areka.md` への行の追記も一緒に。

## 2026-10-23 棚卸㉙の再測定（main aa8da8a7）
- **前提の変化**: `script-security-level`（10-23 着地）が、台本の始めの知らせ（`crates/areka-talk/src/lib.rs` の `StartTalk`）に出どころの欄 `origin` を足し、再生の側は運ぶだけで、受け口が読んで判断する形（判定の表は `crates/areka/src/emo2_boot/script_restriction.rs`）を作った＝案 ⑵（再生まで印を運ぶ）の先例ができた。strict の印をこの欄の隣に置くか、出どころの型（`crates/areka-talk/src/origin.rs` の `ScriptOrigin`＝SHIORI へ送る見出しにも使う）の中に入れるかは設計で決める（隣を推す）。kanade の控え（`schedule/sstp.rs` 150 行の `StrictNote`）は今も記録とテストだけが読む。
- **触るファイル**: 案 ⑴＝`crates/areka/src/mcp/{sakurascript,raise_event}.rs`（112・95 行・変わらず）と兄弟のテスト・`crates/areka-mcp/src/handler.rs`（178・案内文は 39 行目）・`doc/ssp-mcp/areka-tools.md`・`check_script_judge.rs`（218）・要れば kanade の `schedule/sstp.rs`。案 ⑵＝さらに `areka-talk` の `lib.rs`（133）・`crates/areka-sakura/src/drive.rs`（576）・seriko の `actor.rs`（768）と `resolve.rs`（314）・`crates/areka-emo-present/src/presenter/show.rs`（426）・`emo2_boot/consumer_ledger.rs`（472→496・`window-link-directives` が行を足した）・kanade の `schedule/translate.rs`（300→348）と、検出点の表と一致の検査（新）。棚卸㉘の申し送りの件（`params` が入れ物でない要求の答え）は `crates/areka-mcp/` の受け口と中継の答え・`doc/ssp-mcp/transport-diff-areka.md`。
- **規模**: 案 ⑴ 7〜11・案 ⑵ 13〜19（申し送りの件の 1 つ分を足した）。上限の内。
- **先に要るもの**: 働きの上では無い＝先頭。同じウェーブに置けない相手（v0.1.0）: 案 ⑴ は 0（`schedule/sstp.rs` を挙げるのは後の版の `resident-ghost-rotation`・`shiori4-api` だけ）。案 ⑵ は `consumer_ledger.rs` の `stay-on-top-directives`・`script-small-tags`・`descript-window-keys`・`shell-scale-linkage`・`balloon-communicatebox`、`presenter/show.rs` の `animated-surface-log-flood`、余力の枠の `multi-ghost-runtime`（`mcp/sakurascript.rs` がゴーストの箱を読む・kanade の `msg.rs`）。`mouse-click-wheel-events` とは案 ⑴ で 0、案 ⑵ で kanade の `msg.rs` を触るときだけ重なる。`shiori2-protocol` とは 0。1 本だけの席・静かな机・人の待ちは要らない。
- **優先度の区分**: A のまま（開発者「ssp mcp tool の完全移植のための spec 群を立ち上げて」）。
- **要件定義のモデル**: Opus のまま（案 ⑴ は `check_script` の判定、案 ⑵ は `StartTalk` の出どころの欄と受け口の判定の表が先例）。
- **分割の案**: なし（案 ⑵ で 20 を超えたら棚卸㉑の切り方）。
- **見つけた穴・古くなった記述**: 案内文の最後の 1 文（`crates/areka-mcp/src/handler.rs` 39 行目「まだのツールは `NG:not implemented yet` で始まる答えを返す」）は、当てはまるツールが 0 本のまま残っている＝今すぐ消せる。棚卸㉘の「`translate.rs`（300）」「`consumer_ledger.rs`（472）」は 348・496 に読み替える。10-08 の申し送りの表に `unpaired_tag` の行が無いこと・文面の定数と文書の一致をテストで見ていないことは前のまま。

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
