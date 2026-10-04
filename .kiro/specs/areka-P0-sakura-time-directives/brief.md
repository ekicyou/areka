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
