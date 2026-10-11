# Brief: areka-P0-sakura-time-directives

> **種別**: 追跡 spec（正典先送りの 4 点セット＝完全語彙＋縮退シーム＋追跡 spec＋roadmap 明記）。④sakura（compile）＋dola（Barrier シーム）帰属。
> **源**: `areka-P0-sakura-dialogue-tags` 要件ディスカッション議題5（2026-07-18 `\!` 汎用キャリア裁定）の compile 側 allowlist 但し書き（同 R4.3 が正本）。敵対的検証（refute-opacity）が「解釈 100% 消費側」の有界反例として確定したクラス。
> **着手ゲート**: M1 外（emo2 実使用は move/bind のみ＝本 allowlist コマンドの使用ゼロ）。これらを使うゴーストの適合が必要になった時に解禁。

## Problem

`\!` 汎用キャリア裁定は「転写は不透明・解釈は消費側」を原則とするが、**compile 自身が第一消費者になるべき時間指令系**が有界（10 個未満）存在する。これらは絶対時刻焼込（`text_playback_duration`・配送時導出は禁忌＝desync・[[areka-dola-absolute-time-sync-broadcast]]）・barrier パラメータ・実行時未確定の待機に構造上干渉し、**消費側の発火時解釈では既焼込の後続 cue 絶対時刻を直せず手遅れ**になる。

M1 は全て汎用 cue として転写（語彙第一級保持）しつつ、compile 追加解釈なし＋消費者不在の良性スキップ（時間効果なし）で縮退する。

## 語彙（完全形・ukadoc 一次 HTML 接地・これが allowlist の全量）

| 群 | コマンド | 干渉の構造 |
|---|---|---|
| **A. テキスト時間指令** | `\![quicksection,true\|false\|数値]`（`\_q` ほぼ互換＝瞬間表示） / `\![set,balloonwait,倍率\|ms指定]`（文字ごとウエイト倍率・スクリプト終了でリセット） | per-char D 焼込の計算そのものを変える |
| **B. script 単位属性→barrier パラメータ** | `\![set,choicetimeout,時間]`（**位置非依存**＝「選択肢より後ろに書いても有効」→ `WaitForChoice{timeout}` へ焼込） / `\![set,balloontimeout,時間]` | cue の区間モデルでは「後方 cue が前方 barrier を書換える」を表現できない＝compile（全 sheet 事前走査）必須 |
| **C. Barrier 級（実行時未確定・自己書換）** | `\![embed,イベント名,r*]`（タグ全体が SHIORI Result で置換され続行＝台本分割＋再調停が必要） / `\![sound,wait]`（=`\_V`） / `\![wait,syncobject,名前,--timeout=]` | 待機長がコンパイル時不可知＝静的絶対時刻タイムラインに直接載らない |
| **D. ブロッキング持続時間引数** | 同期 `\![move]` の時間スロット／`--time`（envelope duration へ転写し offset を進める・moveasync は 0） / `\![set,scaling,--time/--wait]` / `\![set,alpha,--time/--wait]` | duration 実値化に引数解釈が要る（転写は不透明のまま・duration 焼込だけ compile が name を覗く） |

## Desired Outcome

allowlist 各コマンドが compile で追加解釈され、A＝D 焼込補正／B＝barrier パラメータ焼込／C＝台本分割＋Barrier シーム＋オーケストレーター（kanade/sakura）再調停／D＝envelope duration 実値化へ正しく lowering される。**allowlist 外の compile 解釈は引き続き禁止**（dialogue-tags R4.3 が恒久の正本・汎用キャリアの不透明原則を侵食させない）。

## Approach

compile の汎用キャリアアームへ allowlist 判定を追加（純関数・全網羅檻）。C 群は dola の Barrier シームへ写像（動的制御は dola 外側＝settled 裁定）。段階導入可: A/B/D は compile 局所・C は台本分割の設計が要る（C だけ後続波でも良い）。

## Scope

- **In**: allowlist 8 コマンド族の compile 追加解釈・lowering・決定論檻（script 直入力→期待 cue/barrier/duration 列）。
- **Out**: allowlist 外の compile 解釈（恒久禁止）／各コマンドの**消費側**実装（該当演者の領分）／SSTP 経由の文脈依存挙動。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-sakura-dialogue-tags`（汎用キャリア＋R4.3 allowlist 契約の正本）／completed `cue-playback-duration`（絶対時刻台本・Barrier シーム・envelope duration）。
- **Downstream**: これらのコマンドを使う実ゴーストの適合／`areka-P0-choice-select-events`（`choicetimeout` の**ランタイム消費側**＝タイムアウト起点・OnChoiceTimeout 発火は W5 の領分・本 spec は compile 焼込のみ）。

## Constraints

- 正典は ukadoc。決定論檻必達・二重待ち禁止（タイミングは焼込絶対 start_time が唯一の権威）。
- 汎用キャリアのワイヤ形・消費側名前選別の規律（dialogue-tags R4.5/R8.7）は不変。

---

> **📌 2026-09-02 棚卸⑫**——`file:line` 主張なし（ukadoc 一次接地のみ）＝ドリフト該当なし。編集集合＝`areka-sakura/compile.rs`（allowlist 判定・lowering）・`dola/src/cue/`（C 群 Barrier）・`areka-kanade/`（C 群再調停）。**`compile.rs` を `text-decoration-canon` と共有**＝W13 と同居不可（段階 A/B/D は compile 局所・C は台本分割＝分割するなら A/B/D／C）。`\![set,balloontimeout]` は residue 項目 7 と対（COMPAT §8 で住み分け済み）。前提「これらを使うゴーストの適合」は未充足＝M2 ゲート据え置き。

---

> **📌 2026-10-01 `/kiro-discovery`（シェル内バルーン）からの注記**——新 spec `areka-P0-talk-fast-forward` が「クリックでの早送り」（areka 独自・利用者の 1 クリックで台詞の時計を次の `\x` か台詞の終わりまで早回し）を持つ。**`\_q`・`\![quicksection]`・`\![set,balloonwait]` は本 spec の担当のまま**で、早送りとは別物（早送りは利用者の操作、`\_q` と `balloonwait` は台本の指示）。両者が同じ台詞の時計を触るので、後から着地する側が「早送り中の `balloonwait` の倍率」「`\_q` の区間の中での早送り」を決定論テストで固定する。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **B 群の `\![set,choicetimeout]` を `areka-P0-choice-timeout-directive` へ切り出した**（利用者に見えるバグ＝時間切れなしを指定したメニューが 30 秒で閉じる。ウェーブ C1）。本 spec に残るのは A 群（`quicksection`・`balloonwait`）・B 群の `balloontimeout`（受ける側は `balloon-lifecycle-events` の項目 7）・C 群・D 群。
- 残りも全部入れると 20 タスクを超える見込み＝要件の段で「A と `balloontimeout`（compile の中で閉じる）」と「C・D（消費する者がまだ居ない＝音の再生・時間つきの移動・拡大と透明度が無い）」に分け、C・D は消費する者が現れるまで置く。
- `compile.rs`（346 行）は `text-decoration-canon` の完了で空いた。今は `choice-timeout-directive`・`anchor-tag-canon`・`talk-fast-forward` と共有＝同時に走らせない。A 群は `talk-fast-forward`・`text-reveal-fade` と「台詞の時計」を分け合う。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: 残り全部では 20 を超える（A・B の残り・C・D で 24〜30）＝**切る**。案: ⒜ 本 spec＝A 群（`\![quicksection,…]`・`\![set,balloonwait,…]`）＋`\_q`（網羅台帳で本 spec が持ち主・下記）＋B 群の `\![set,balloontimeout]` のコンパイルの側＝M（10〜14）／⒝ C 群・D 群＝消費する者が現れるまで置く（`\![sound,wait]` は音の再生、`\![wait,syncobject]` は同期の物、D 群は時間つきの移動・拡大・透明度が無い）。C 群の `\![embed]` だけは消費する者（SHIORI）が既に居るので、`property-query-channels` の再測定の案どおり別 spec（仮名 `sakura-embed-directive`）へ出し、本 spec の C 群からは外す。⒝ を今 brief に分けるかは任意（置くだけなら本 brief の「Out」に残せば足りる）。
- 前提の状態: `choice-timeout-directive`（✅ 10-03）は着地。`talk-fast-forward`（台詞の時計を分け合う・後着がテストで固定する約束）はまだ。
- 崩れた前提／古くなった位置:
  - `crates/areka-sakura/src/compile.rs` は 411 行（前回 346）。汎用キャリアの腕は `compile` の `Instruction::GenericCommand { name, raw_args }` の腕。`choice-timeout-directive` が `\![set,choicetimeout]` の先読みをここへ入れた＝本 spec の「位置によらない属性の先読み」の雛形が在る（`compile_choice_timeout_tests.rs`）。
  - `\_q` は字句で正しく切れない既知の不具合（`anchor-tag-canon` の brief の棚卸⑫の追記）が `\_` の 2 文字の裸の形すべてに在る。`\_q` を実装するなら、その直し（`anchor-tag-canon` の働きの側の先頭）の後でないと動かない。
  - 網羅台帳 `sakura-script.toml` で本 spec が持ち主の行は 10（`\_q`・`quicksection` 2・`balloonwait`・`move`・`scaling` 2・`alpha`・`sound,wait`・`wait,syncobject`）。`\![set,balloontimeout]` の行の持ち主は `areka-P0-balloon-canon-residue` のまま（受ける側は `balloon-lifecycle-events` の項目 7）。
- 触るファイル（並走の照合用・⒜）:
  - `crates/areka-sakura/src/compile.rs`・`crates/areka-sakura/src/duration.rs`（文字ごとの時間の焼き込み）と兄弟のテスト
  - `\_q` を含めるなら `crates/areka-parsers/src/sakura/{lexer,decode}.rs`
  - `doc/ukadoc-coverage/ledger/sakura-script.toml`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: `\_q` を本 spec の ⒜ に入れるか（入れるなら字句の直しの後＝`anchor-tag-canon` の後）。
- 見つけた穴: なし。並走の照合: 台本のコンパイルの列（`anchor-tag-canon`・`talk-fast-forward`）と `compile.rs` を分け合う＝直列のまま。`sakura-time-critical` は `\t` を汎用キャリアへ写せば `compile.rs` を触らずに済む（向こうの再測定）。

## 2026-10-04 棚卸㉑で切った後の範囲

- 残した範囲: ⒜ A 群（`\![quicksection,…]`・`\![set,balloonwait,…]`）と B 群の `\![set,balloontimeout]` のコンパイル側。`\_q` を入れるかは議題のまま（入れるなら字句の直し＝`anchor-tag-canon` の後）。
- 規模: M（10〜14 タスク）。
- 移した先: `\![embed]` は新しい spec `areka-P0-sakura-embed-directive` へ。C 群の残り（`\![sound,wait]`・`\![wait,syncobject]`）と D 群（同期の `\![move]` の時間・`\![set,scaling]`・`\![set,alpha]` の `--time`／`--wait`）は、消費する者（音の再生・同期の物・時間つきの移動・拡大・透明度）が現れるまで roadmap の覚え書きへ戻す（本 spec では作らない）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（10〜14）のまま。切る: なし。
- 前提の状態: 台本のコンパイルの列で `anchor-tag-canon`（働き）→ `talk-fast-forward` の後ろ。どちらもまだ＝着手は待ち。
- 崩れた前提／古くなった位置: なし。C3 は `crates/areka-sakura/`・`crates/areka-parsers/src/sakura/` に触れていない（`compile.rs` 411・`duration.rs` 104・`lexer.rs` 415・`decode.rs` 400 は前回と同じ）。網羅台帳 `sakura-script.toml` で本 spec が持ち主の行は 10 のまま。`\![set,balloontimeout]` の受ける側の `balloon-lifecycle-events` は C4 予定。
- 触るファイル: `crates/areka-sakura/src/{compile.rs, duration.rs}` と兄弟のテスト・（`\_q` を入れるなら）`crates/areka-parsers/src/sakura/{lexer,decode}.rs`・`doc/ukadoc-coverage/ledger/sakura-script.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 議題（答えで作業が変わるものだけ）: `\_q` を入れるか（前回どおり・入れるなら `anchor-tag-canon` の後）。
- 見つけた穴: なし。


---

> **📌 2026-10-05 相互登記（`areka-P0-balloon-lifecycle-events` の要件の討議）**——**`\![set,balloontimeout,時間]` は本 spec の担当から外し、`balloon-lifecycle-events` が丸ごと持つ**（開発者裁定「並走の spec が触らないなら本 spec が担当すべき」）。正典は「時間切れはスクリプトの表示が終わってからカウント」「そのスクリプト中のみ有効」で、タグは汎用の `\!` の運び手で表示が終わる時刻までに必ず表示の側へ届くため、コンパイルの側の先読みは要らない（`choicetimeout` とは違い、区切りの値へ焼き込む必要が無い）。本 spec に残るのは A 群（`\![quicksection,…]`・`\![set,balloonwait,…]`）と `\_q`（議題）。網羅台帳の `balloontimeout` の行の持ち主は `balloon-lifecycle-events` が直す。


---

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - `\![set,balloontimeout]` は `balloon-lifecycle-events`（10-08）が持って着地した（台帳の行は `implemented`）。本 spec に残るのは `\![quicksection,…]`・`\![set,balloonwait,…]`・`\_q` の 3 語。
  - **`\_q` を待たせていた理由は無かった**: 棚卸㉑・㉒の「`\_q` は字句で正しく切れない・直しは `anchor-tag-canon` の後」は誤り。字句の直しは完了 `sakura-bare-tag-lexer`（PR#134）で済んでいて、`\_q` は裸の `_q` として切れる（`crates/areka-parsers/src/sakura/lexer_bare_tag_tests.rs` が固定）。今は `decode.rs` の `decode_bare` に腕が無く、素通しになるだけ＝腕を 1 本足せば読める（裸の `\+` を `\![change,ghost,random]` へ写す腕が先例）。議題「`\_q` を入れるか」は「入れる」で閉じてよい。
  - `mcp-author-tools`（10-08 着地）: `decode.rs` は腕ごとに印を返す形になり、`crates/areka-sakura/src/compile.rs`（411 行）は `parse_choice_timeout` を公開しただけ。台本の検査（MCP の `check_script`）は `\!` の受け取り手の表に無い名前を「知らない命令」と答える＝`quicksection` と `set` の `balloonwait` を表（`crates/areka/src/emo2_boot/consumer_ledger.rs`・943 行・担当は台本の組み立て）に足し、一致の検査（`consumer_ledger_agreement_tests.rs`）に見本を足す。値の読み取りを `parse_choice_timeout` と同じ形の公開の関数にすれば、検査が使い回せる。
  - `budoux-reveal-reflow`（10-06 着地）: 文字の層へ先渡しされる合図は再生時間つき＝倍率や瞬間表示を焼き込んだ値がそのまま入る（文字の層に手は要らない）。
- 触るファイル: `crates/areka-sakura/src/{compile.rs, duration.rs, lib.rs}` と兄弟のテスト・`crates/areka-parsers/src/sakura/{decode.rs, model.rs}`（`\_q`）・受け取り手の表の 2 本・（検査に値の誤りを答えさせるなら）`crates/areka/src/mcp/check_script_judge.rs`・台帳 `sakura-script.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。emo-text には触らない。
- 規模: 8〜12 タスク（棚卸㉒は 10〜14。`balloontimeout` が抜け、表と検査が増えた）。
- 分割の案: 切らない。
- 先に要るもの: 働きの前提は無い。ファイルの順は台本のコンパイルの列（`anchor-tag-canon` → `range-choice-tag` → `talk-fast-forward` の後）。早送りとの決め（早送り中の倍率・`\_q` の中の早送り）は後から着地する側が検査で固定する。
- 優先度の区分: C（ukadoc の先送りの追跡）。
- 要件定義のモデル: Opus（コンパイルの中で閉じる・`\_q` の議題は解けた）。
- 見つけた穴・古くなった記述: 網羅台帳で本 spec が持ち主の 10 行のうち 6 行（同期の `\![move]`・`\![set,alpha]`・`\![set,scaling]` の 2 行・`\![sound,wait]`・`\![wait,syncobject]`）は、棚卸㉑で範囲から外したのに持ち主が本 spec のまま＝着手のときに付け替える。冒頭の「着手ゲート: M1 外」「allowlist 8 コマンド族」は今の範囲（3 語）と合わない。

## 2026-10-10 `areka-P0-seriko-trigger-intervals` の完了時の申し送り

出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes（9.2 の ③・検証の ⑷⑸）と同 spec の設計書（Revalidation Triggers）。

- **実機で見たこと（2026-10-10）**: 台本の `\_q…\_q` は効かない。挟んだ字も 1 字 50 ms のまま現れた。上の「棚卸㉓の再測定」に書いてあるとおりの今の姿（読み手に腕が無く素通し）で、新しい事実は無い。
- **口パクが、文字の現れる時刻の式を写して持つようになった**: `seriko-trigger-intervals` で、seriko は interval `talk,数値` のために「i 文字目が現れる時刻」を自分でも計算する（`crates/areka-seriko/src/talk.rs` の `TalkFeed`・`TalkEpoch`）。式は文字の層と同じで、入力は cue の時刻・再生時間・文字数だけ。**共有ではなく写し**で、文字の層の式を変えても赤になるテストは無い。
- **本 spec がすること**: 上の節は「倍率や瞬間表示を焼き込んだ再生時間が cue に入るので、文字の層に手は要らない」と書く。seriko の写しも同じ cue の再生時間から計算するので、**cue の再生時間を変えるだけなら `talk.rs` にも手は要らない見込み**。要るのは確かめ: `\_q` で再生時間 0 の字がまとめて届くと、1 回の刻みで区切りを 2 つ以上越える。この場面は判定の純粋な関数のテストだけが固定していて、配線を通したテストも実機の確かめも無い（`\_q` が未対応で踏めなかった）。本 spec の実機の確かめに「`talk,数値` の面で `\_q` を流し、口が最新の区切りで 1 回だけ動く」を 1 項目足す（検体は `crates/areka-seriko/tests/fixtures/trigger-intervals/` を使い回せる）。
- 文字が現れる時刻の式そのもの・起点の取り方に触ることになったら、`talk.rs` の写しも同じに変える。写しの見張りは新しい spec `seriko-talk-clock-fidelity` が持つ。
