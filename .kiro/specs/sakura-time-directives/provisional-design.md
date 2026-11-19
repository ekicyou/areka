# 暫定の設計: sakura-time-directives

## 1. 前置き

- これは**暫定の設計**である（2026-11-19）。正式な `design.md` ではない。要件定義（`requirements.md`）はまだ無く、下の「暫定の要件」は brief と pasta の骨子・取り決めから起こした仮の物である。
- 目的は、正式な spec の流れを始める前に、上位の決定（pasta の骨子・取り決め）へ差し戻しが要る矛盾を見つけること。細部は磨いていない。
- 基にした main: `49b6f3ab5`（ワークツリーの HEAD は `018d102c0`。中身は同じ）。
- コードは読んだだけで、実行して確かめた物は 0 である（11 章）。
- 読んだ物:
  - areka の brief: `.kiro/specs/sakura-time-directives/brief.md`・隣の `talk-fast-forward`・`parallel-visual-script`・`window-timed-directives`・`sound-canon-tags` の `brief.md`。
  - pasta: `novel-areka-contract/final-model-2026-11-19.md`（以下「骨子」）・同 `brief.md` の「決定（2026-11-19…最終形）」の節（以下「取り決め」）・`play-script-guide.md` と `novel-talk-flow/brief.md` の高速再生の行（grep）。
  - areka の側の見立て: `areka-novel-skip-contents`・`areka-novel-x-timeout`・`areka-novel-z1-z10`（Z10）の 3 本。
  - コード: `crates/areka-sakura/src/{compile,duration,drive,strict}.rs`・`crates/areka-parsers/src/sakura/{decode,model}.rs` と裸のタグのテスト・`crates/dola/src/cue/{schedule,runtime,command}.rs`・`crates/areka-emo-text/src/state.rs`（文字の時刻の所）・`crates/areka-seriko/src/talk.rs`・`crates/areka-runtime/src/{consumer_ledger,talk_clock}.rs`。
  - 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の該当の行・`doc/COMPAT_ARCHITECTURE.md` 131 行。
  - ukadoc（MCP）: `\_q`・`\![quicksection,true]`・`\![quicksection,false]`・`\![set,balloonwait,倍率]`・`\__v[オプション]`・`\_w[時間]`・`\__w[時間]`・`\x`・`\![timerraise,…]`。
- 兄弟の `provisional-design.md` は、書き始めた時点で 1 本も無かった（`.kiro/specs/*/provisional-design*.md` と `doc/ideas/*/` を ls して 0 件）。書き終えた時点で在ったのは `sound-canon-tags` と `ghost-material-metadata` の 2 本で、`sound-canon-tags` の物だけ、この単位に関わる行（`\![sound,wait]`・`\_V`・`compile.rs`・出し直し）を grep して照らした（4.7・6 章）。受け口の単位（`doc/ideas/novel-play-flow-provisional/`）・`talk-fast-forward`・`parallel-visual-script`・`window-timed-directives` の物はまだ無く、brief どうしで突き合わせた。

## 2. 暫定の要件

この単位の今の範囲は、brief の「2026-10-10 棚卸㉓」のとおり `\_q`・`\![quicksection,…]`・`\![set,balloonwait,…]` の 3 語である。そこへ、取り決めがこの単位に求める口（要件 12・16）を足した。

| # | 要件 | 出どころ |
|---|------|----------|
| 1 | 台本に裸の `\_q` が在るとき、読む段は、知らないタグとして捨てず、運搬名つきの汎用コマンドに写す。 | brief 棚卸㉓ |
| 2 | コンパイルは、`\_q` を通るたびに「瞬間表示の区間」の入切を反転する。区間の中の字（台詞・システム変数の展開）の cue は、再生時間 0 とし、台本の時刻を進めない。 | ukadoc `\_q`・取り決め「命令の表」の「字を一度に出す」 |
| 3 | `\![quicksection,true]`・`\![quicksection,1]` は区間を入に、`false`・`0` は切にする。`\_q` と混ぜて書いても、同じ 1 つの入切を動かす。ほかの値は、入切を変えずに記録する。 | ukadoc `\![quicksection,…]` |
| 4 | 区間の中でも、`\w`・`\_w` の待ちは、書いた長さのまま台本の時刻を進める。 | ukadoc `\_q`「瞬間表示中もウェイトがかかる」 |
| 5 | 区間は台本 1 本の中だけで効く。`\e` か台本の終わりで切れ、次の台本は切から始まる。`\x`・`\c` では切れない。 | ukadoc「次の\_qか\eまで」・取り決め「効く範囲: その 1 本」 |
| 6 | `\![set,balloonwait,値]` が在るとき、コンパイルは、それより後ろの字の 1 字あたりの待ちを替える。値は、数だけ（倍率）・`%` つき・`ms` つきの 3 つの形。省略と 1 は既定へ戻し、0 は瞬間表示にする。 | ukadoc `\![set,balloonwait,倍率]` |
| 7 | 1 字あたりの待ちの指定は、台本の終わりで既定に戻る。areka は値を覚えない。 | ukadoc「スクリプト実行終了後に100%指定相当にリセット」 |
| 8 | 1 字あたりの待ちの指定は、`\w`・`\_w` の長さを変えない。 | ukadoc「文字ごとのウエイト」 |
| 9 | 値が読めない・範囲の外のとき、トークは止めず、既定として扱い、警告を 1 行残す。strict の台本と `check_script` でも分かる。 | areka の決まり（失敗を黙って捨てない）・`choicetimeout` の前例 |
| 10 | 字と待ちの長さは、コンパイルの時に決めて cue の開始時刻と再生時間に入れる。配る時に導かない。同じ台本は、同じ時刻の表になる。 | brief Problem・Constraints |
| 11 | 3 語は、汎用の運び手の cue としても今までどおり出す（再生時間 0・時刻を進めない）。ここに挙げた名前のほかの `\!` の意味は、コンパイルは読まない。 | brief Desired Outcome |
| 12 | コンパイルは、「その 1 本の字を全部一度に出し、`\w`・`\_w` の長さを 0 にする」走り方を、内側の口として持つ。口を開くのは、出し直しの印 `\![play,again]` を受ける単位である。`\x` の区切りには触らない。 | 取り決め「命令の表」の「出し直しの印」 |
| 13 | 文字の層と口パク（seriko の写し）には、手を入れない。再生時間 0 の字は、cue の時刻に全部見える。 | brief 棚卸㉓・10-10 の申し送り |
| 14 | 3 つの名前を、受け取り手の表（`consumer_ledger.rs`）に登記する。`check_script` は「知らない命令」と答えなくなる。 | brief 棚卸㉓ |
| 15 | クリックの早送りが掛かっても、焼き込んだ時刻は変えない。「区間の中の早送り」「倍率を替えた後の早送り」を、後から着地する側が決定論テストで固定する。 | brief 2026-10-01 の注記 |
| 16 | 並走の区間の中に書かれた 3 語は、主の流れの速さを変えない。要件 12 の口は、並走の区間の中の待ちにも効く。 | 取り決め「並走」・「出し直しの印」 |
| 17 | 網羅台帳の 4 行（`\_q`・`quicksection` の 2 行・`balloonwait`）を実装済みにし、範囲から外した 5 行の持ち主を付け替えるか空にする。`doc/COMPAT_ARCHITECTURE.md` の行を直す。 | brief 棚卸㉓ |
| 18 | 実機で、`talk,数値` の面に `\_q` を流し、口が最新の区切りで 1 回だけ動くことを確かめる。 | brief 10-10 の申し送り |

## 3. 今の作り（確かめた事実）

### 3.1 台本の時刻の作り

- `crates/areka-sakura/src/compile.rs:135-479` の `scan` が、命令の列を 1 回走査して cue の列を作る。時刻は `offset_ms: u64`（141 行）に整数のミリ秒で積み、命令ごとの頭で 1 回だけ秒に直す（165 行）。
- 時刻を進める腕は 3 つだけである。
  - `Instruction::Wait`（173-180 行）: 再生時間だけを持つ `CueCommand::Wait` の cue を出し、`offset_ms` を待ちの長さだけ進める。
  - `Instruction::Text`（187-191 行）: 再生時間 `text_playback_duration(text)` の cue を出し、`offset_ms` を `text_playback_ms(text)` だけ進める。
  - `Instruction::SystemVar`（313-321 行）: 展開した字を `Text` と同じに扱う。
- 字の長さは `crates/areka-sakura/src/duration.rs:44-46` の `text_playback_ms` ＝ 書記素クラスタの数 × `CHAR_NOMINAL_MS`（21 行・50）。この定数は「唯一の定義箇所」と書いてある（16 行）。
- `\![名前,…]` は `Instruction::GenericCommand` の腕（282-307 行）が、再生時間 0 の運び手の cue に写す。その後で `set,choicetimeout` だけ値を読む（289-306 行）。読む関数は公開の `choice_timeout_of`（497-500 行）と `parse_choice_timeout`（522-538 行）で、strict と `check_script` が使い回す。読んだ値は、走査の中のローカル変数 `last_choice_timeout`（148 行）に持ち、関数の外に状態を持たない。
- 走査の前に命令の列を 1 回先読みする前例が在る（`pair_anchors`・152 行。走査の入力は `Clone` できるイテレータ・136 行）。
- 選択肢の無い台本に、区切り（barrier）は 1 つも出ない（447-456 行は `has_choice` のときだけ）。`\x` の腕は無い。

### 3.2 読む段

- `\_q` は字句では裸のタグ `_q` として切れる（`crates/areka-parsers/src/sakura/lexer_bare_tag_tests.rs:50`）。`decode_bare`（`decode.rs:259-301`）に腕が無く、既定の腕（299 行）から `Raw` になる（`parse_bare_tag_tests.rs:53-54` が固定）。`compile.rs:411-419` の `Raw` の腕が、警告を残して捨てる。
- `\__v` と `\__v[disable]` も、字句では切れる（`lexer_bare_tag_tests.rs:66`・`lexer_word_boundary_tests.rs:178`）が、どちらも `Raw` になる（`parse_bare_tag_tests.rs:97-98`）。
- 裸のタグを運搬名つきの汎用コマンドに写す前例が在る: `\v` → `RAISE_TAG_CARRIER`（`decode.rs:294-297`・`model.rs:109`）・`\j[ID]` → `JUMP_TAG_CARRIER`（`model.rs:103`）。
- `\_w[ms]` は `Instruction::Wait` になる（`decode.rs:310`）。

### 3.3 再生と受け手

- `crates/dola/src/cue/schedule.rs:170-207` の `TimedSchedule::tick` は、「アンカー＋相対の時刻」が今の時刻以下の cue を配る。区切りに着くと止まる（201-204 行）。完了は、配り終えて、かつ今の時刻が占有 horizon（全 cue の「開始＋再生時間」の最大）に着いた時（261-265 行）。
- クリック待ちの区切り `BarrierKind::WaitForInput` と、解く口 `CuePlayer::resolve_click` は dola に在る（`command.rs:75-81`・`runtime.rs:253-259`）。コンパイルは、まだ出さない。
- 文字の層は、cue の再生時間から 1 字の間隔を導く（`crates/areka-emo-text/src/state.rs:538-543` の `interval = cue.duration / glyph_count`）。再生時間 0 なら、全部の字が cue の時刻に同時に見える（同 513-517 行の説明）。
- seriko の口パクの写しも、同じ式である（`crates/areka-seriko/src/talk.rs:73-79` の `push_chunk`）。
- 受け取り手の表は `crates/areka-runtime/src/consumer_ledger.rs`（brief が書く `crates/areka/src/emo2_boot/` からは移っている）。`("set", Some("choicetimeout"), ScriptCompile)` の行が 365 行に在る。
- strict の「値が読めない」は `crates/areka-sakura/src/strict.rs:236-246` の `command_point` が、`choicetimeout` だけを見て答える。

### 3.4 網羅台帳の持ち主（`sakura-script.toml`）

| タグ | status／owner（行） | この単位 |
|------|---------------------|----------|
| `\_q` | absent／`sakura-time-directives`（4293-4296） | **持つ** |
| `\![quicksection,true]`・`\![quicksection,false]` | vocabulary-only／`sakura-time-directives`（2616-2619・2601-2604） | **持つ** |
| `\![set,balloonwait,倍率]` | vocabulary-only／`sakura-time-directives`（3121-3124） | **持つ** |
| `\w時間`・`\_w[時間]` | implemented／空（5568・4374） | 持たない（もう動いている） |
| `\__w[時間]` | absent／空（4062） | 持たない。持ち主が居ない |
| `\__v[オプション]` | absent／空（4046-4060。note に「所有先未定」） | 持たない。**持ち主が居ない** |
| `\x`・`\x[noclear]` | alias／vocabulary-only・`balloon-canon-residue`（5582・5600） | 持たない（`talk-fast-forward` の brief が引き取ったと書く。台帳の欄は古い） |
| `\![raise,…]` | absent／空（2631） | 持たない。持ち主が居ない |
| `\![timerraise,…]` | absent／空（3637-3652） | 持たない。**持ち主が居ない** |
| `\![sound,wait]`（`\_V` は別名） | vocabulary-only／`sakura-time-directives`（3567-3570） | 範囲から外した（brief 84 行）。欄だけ残っている |
| `\![wait,syncobject,…]` | vocabulary-only／`sakura-time-directives`（3860-3863） | 範囲から外した。欄だけ残っている |
| `\![set,alpha,…]`・`\![set,scaling,…]` の 2 行 | absent／`sakura-time-directives`（2996・3217・3233） | `window-timed-directives` が引き取った（brief 141 行） |

- crate の中に `quicksection`・`balloonwait`・`timerraise`・`xtimeout` の語は 0 件（`crates/` を Grep。受け取り手の表にも 0 件）。

## 4. 設計

### 4.1 構成

足す所は 3 つの層で、どれも既に在るファイルの腕である。新しい crate・新しい cue の型は作らない。

| 層 | ファイル | 足す物 |
|----|----------|--------|
| 読む段 | `crates/areka-parsers/src/sakura/model.rs`・`decode.rs` | 運搬名の定数 `QUICK_TAG_CARRIER`（`"\\_q"`）と、`decode_bare` の `"_q"` の腕 |
| 台本のコンパイル | `crates/areka-sakura/src/duration.rs`（か、兄弟の新しいファイル `pace.rs`）・`compile.rs`・`lib.rs` | 速さの状態 `Pace`・値を読む公開の関数・`scan` の 3 つの腕の差し替え |
| 検査と登記 | `crates/areka-runtime/src/consumer_ledger.rs`・`crates/areka-sakura/src/strict.rs`・台帳・`doc/COMPAT_ARCHITECTURE.md` | 表の 3 行・「値が読めない」の点・台帳の行 |

dola・emo-text・seriko・kanade には触らない（再生時間の入った cue を配るだけなので、受け手は今のまま正しく動く。3.3）。

### 4.2 読む段

- `decode_bare` に腕を 1 本足す: `"_q"` → `Instruction::GenericCommand { name: QUICK_TAG_CARRIER, raw_args: [] }`。`\v`・`\j` と同じ流儀で、運搬名は逆斜線で始まるので、台本の `\![…]` では書けない。
- `\![quicksection,…]` に写さない理由: `\_q` は反転で、`quicksection` は入か切を指す。正典に、同じ意味の `\!` の綴りが無い（ukadoc: 「\_qとほぼ互換で、スイッチ式でなく開始を指定出来るのが違い」）。
- 角括弧つきの `\_q[…]` は正典に無いので、腕を作らない（`Raw` のまま）。
- `Raw` を期待している見本が変わる: `parse_bare_tag_tests.rs` の 53-54・142・203-204 行・`parse_word_boundary_tests.rs:94-95`。

### 4.3 速さの状態 `Pace`

```rust
/// 台本 1 本の中の、字と待ちの長さの決め方（走査の中だけで生きる）。
pub(crate) struct Pace {
    char_ms: u64,     // 1 字あたりの待ち。既定は CHAR_NOMINAL_MS
    quick: bool,      // 瞬間表示の区間の中
    skip_waits: bool, // 明示の待ちを 0 にする（出し直し）
}
impl Pace {
    pub(crate) fn text_ms(&self, text: &str) -> u64; // quick なら 0。ほかは クラスタの数 × char_ms
    pub(crate) fn wait_ms(&self, ms: u64) -> u64;    // skip_waits なら 0。ほかは ms
    pub(crate) fn apply(&mut self, d: PaceDirective);
    pub(crate) fn instant() -> Self;                 // quick と skip_waits が入（出し直しの初期値）
}

/// 3 語の読み取りの結果（compile・strict・check_script が使い回す）。
pub enum PaceDirective {
    QuickToggle,      // \_q
    Quick(bool),      // \![quicksection,true|1|false|0]
    CharWait(u64),    // \![set,balloonwait,…]（1 字あたりのミリ秒に直した値。0 は瞬間表示）
    CharWaitDefault,  // 省略・1
    Unreadable,       // 読めない・範囲の外
}
pub fn pace_directive_of(name: &str, raw_args: &[String]) -> Option<PaceDirective>;
```

- `pace_directive_of` は、名前が 3 語のどれでもなければ `None`（`choice_timeout_of` と同じ形の純関数。記録を出さない）。
- 値の直し方（`\![set,balloonwait,値]`）:
  - 数だけ（0〜100・小数可）: `char_ms = round(50 × 値)`。
  - `%` つき（0〜10000 の整数）: `char_ms = round(50 × 値 ÷ 100)`。
  - `ms` つき（0〜10000）: `char_ms = 値`。
  - 1 字あたりの待ちは、整数のミリ秒で持つ（`offset_ms` と同じ格子）。端数は、指定を読んだ時に 1 回だけ四捨五入する。字ごとには丸めない（同じ台本は同じ時刻になる）。
- `CHAR_NOMINAL_MS` は今のまま唯一の定義で、`Pace` の既定がそれを読む。

### 4.4 `scan` の変更

```text
let mut pace = Pace::default();            // 受け口の単位が先読みで again を見つけたら Pace::instant()
for instruction in instructions {
    Wait(d)        → ms = pace.wait_ms(d);  Wait の cue（再生時間 ms）を出す; offset_ms += ms
    Text(t)        → ms = pace.text_ms(t);  Text の cue（再生時間 ms）を出す; offset_ms += ms
    SystemVar(n)   → Text と同じ
    GenericCommand → 今までどおり運び手の cue を出す（再生時間 0・offset を進めない）
                     その後で pace_directive_of(name, raw_args) を 1 回読む
                       Some(Unreadable) → warn 1 行（状態は変えない＝既定として扱う）
                       Some(d)          → pace.apply(d)
    ほかの腕       → 変えない
}
```

- 状態は、`last_choice_timeout` と同じに走査のローカル変数だけに持つ。関数の外・ゴースト・プロセスに何も残さない。要件 5・7（台本の終わりで戻る）は、これだけで満たす。`\e`・`\-` は走査を打ち切るので、区間もそこで切れる。
- `balloonwait` も `quicksection` も、書いた位置から後ろに効く（先読みしない）。ukadoc に「後ろに書いても有効」の文は無い（`choicetimeout` だけに在る）。
- 瞬間表示（`quick`）と 1 字あたりの待ち（`char_ms`）は別に持つ。区間を閉じたら、その時の `char_ms` に戻る。
- 選択肢（`\q`）の字は、今も再生時間 0 で出ている（`compile.rs:236-248`）。変えない。
- `skip_waits` のときも、`Wait` の cue は再生時間 0 で出す（台本から待ちの記述を消さない。今の「末尾・単独の待ちも台本に残る」と同じ考え）。

### 4.5 ノベルに関わる所

**(1) 字を一度に出す（`\_q`）。** 1 本の頭に `\_q` が在れば、その 1 本の字の cue は全部、再生時間 0 になる。字と字の間に待ちが無ければ、字の cue は同じ時刻に並び、1 回の刻みで全部配られる。待ち（`\w`・`\_w`）は残る（要件 4。正典どおり）。これは、取り決めの「高速再生で行が出る前に置き換わるなら、pasta が行の末尾に短い待ちを入れる」（`play-script-guide.md` 221 行）が、`\_q` の中でも効くことを意味する。

**(2) 待ちを飛ばす口。** この単位は出せる。`Pace::instant()`（`quick` と `skip_waits` が入）を走査の初期値にすれば、その 1 本は、字の再生時間も待ちの長さも 0 になり、`\x` の区切りだけが残る。取り決めの「出し直しの印 … 字を一度に出す。`\w`・`\_w` を飛ばす。`\x` はふつうに待つ」の、時刻に関わる所は、これで足りる。

- 口を開く腕（`\![play,again]` を読む所）は、受け口の単位が `compile.rs` に足す。印は「その 1 本」に効く物なので、走査の前の先読み（`pair_anchors` の前例）で見つけて初期値を替える形を推す（印が字より後ろに書かれても、頭から効く）。
- 実行時に時計を早回しする形（`talk-fast-forward` の仕組みを頭で掛ける）は推さない。コンパイルで 0 にすれば、状態の命令が全部同じ時刻に並び、取り決めの「その 1 本の同じ時刻の命令を全部当ててから、最初の絵を出す」が、時刻の表の上でそのまま成り立つ。早回しは `talk-fast-forward` の着地を待つことにもなる。
- 「頭で単発の音と声を止める・並走を全部止める・残る命令の値を既定へ戻す・貯めない」は、時刻の話でないので、この単位は持たない（受け口・音・並走・舞台の単位）。

**(3) 貯めない区間（`\__v[disable]`）。** この単位は持たない。台帳でも持ち主が居ない（3.4）。

- 正典の意味（ukadoc `https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#__v`）: 「次の\__vまでの文字列に対して、音声合成機能の発声内容を調整する。」「\__v[disable]：音声合成無効」。続けて「[SSPのみ]この効果はバックログ(発言履歴)への記録にも及ぶ。」「\__v[disable]の区間は音声合成だけでなくバックログへの記録も行われなくなる。」と在る。
- 取り決めの使い方（バックログに記録しない）は、この「SSPのみ」の文と**合う**。正典の主の意味（読み上げの入切）とも、ぶつからない（areka に音声合成は無い。入ったら、その区間は読み上げもしない）。
- 渡し方の勧め（受け口の単位へ）: 読む段で、`\__v[…]` と裸の `\__v` を、運搬名つきの汎用コマンド（`"\\__v"`。引数は角括弧の中をそのまま。裸は引数 0 個）に写す。コンパイルは意味を読まず、再生時間 0 の運び手の cue として、書いた位置の時刻に出す。バックログの受け手が、名前で拾って「記録しない」の入切を自分で持つ（`\f` の運び手を文字の層が拾うのと同じ形）。同じ時刻の cue は書いた順に配られるので、字の cue との前後は保たれる。時刻に触らないので、この単位の allowlist には入らない。
- 読む段の腕は `decode.rs`・`model.rs` で、この単位と同じファイルである。

**(4) 字の前の間（`\w`・`\_w`）。** もう動いている（3.2）。この単位が足すのは、要件 8（倍率が効かない）と要件 12（飛ばす口）だけ。

**(5) `\![timerraise,…]`。** この単位の物ではない。brief の語彙の表（8 つの族）に無く、台帳でも持ち主が居ない。台本の時刻でなく、実行時のタイマーとイベントの話なので、コンパイルは読まない。areka が受けたら、今は運び手の cue になり、誰も拾わず、`debug` の記録だけが残る。pasta は areka へ `\![play,xtimeout,…]` を出すので、ノベルには要らない。

### 4.6 失敗の扱い

- 読めない値・範囲の外の値: `warn!` を 1 行（`event = "pace_directive_unreadable"`・生の欄）。状態は変えず、トークは続く。
- strict の台本では、`command_point` に「値が読めない」を 1 つ足す（`ChoiceTimeoutUnreadable` と並ぶ点。名前は要件で決める）。
- 閉じていない区間（`\_q` が奇数個）は誤りでない（正典: `\e` まで）。記録しない。

### 4.7 ほかの単位との境

| 相手 | 境 |
|------|----|
| 受け口の単位（`doc/ideas/novel-play-flow-provisional/`） | `\![play,again]`・`\![play,xtimeout,…]` の腕、`\x` の区切り、`\![raise]`、バックログと `\__v` は向こう。この単位は `Pace::instant()` を出すだけ |
| `talk-fast-forward` | 早送りは実行時に台詞の時計を進める。この単位はコンパイルの時に長さを決める。焼き込んだ時刻の上を時計が飛ぶだけなので、どちらが先でも成り立つ。`\x` の区切りを足すのは向こう。区間と倍率は `\x` をまたいで続く |
| `parallel-visual-script` | 区間の時刻の積み（主の流れを進めない）は向こう。向こうは区間の中の待ちの長さを `Pace::wait_ms` で決める（出し直しで 0）。区間の中の 3 語は、向こうが読み捨てて記録する（字に関わるタグ） |
| `window-timed-directives` | 待つ形（`\![move]` の時間・`--wait`）で時刻を進める腕は向こう。長さは `Pace::wait_ms` を通す（出し直しで瞬間）。台帳の `alpha`・`scaling` の 3 行は向こうへ |
| `sound-canon-tags` | `\_V`・`\![sound,wait]` は、コンパイルの区切りも実行時の解き方も、向こうが丸ごと持つ。向こうの暫定の設計も同じ分け方である（同 94 行: `compile.rs` で `sound,wait` の運び手の直後に区切り `WaitForExternal{"sound"}` を置く。同 149・182 行: 台帳の行の持ち主を向こうへ付け替える）。向こうの仮定 A12「`\_q` の中でも `\_V` は待つ」は、要件 4 と合う。**合っていない所が 1 つ**: 出し直しの 1 本に `\_V` が在った時に区切りを置くかどうかを、向こうは書いていない（長さの分からない待ちなので `Pace::wait_ms` には通せない。`Pace` の `skip_waits` を見て区切りを置かない、を推す） |
| `seriko-talk-clock-fidelity` | 式は変えないので、写しの見張りは赤にならない見込み |

決まり（この単位が置く）: **`scan` の中で時刻を進める腕は、長さを必ず `Pace` に通す。** 字は `text_ms`、時間の分かる待ちは `wait_ms`。長さの分からない待ち（外の出来事を待つ区切り）は、`skip_waits` が入なら置かない。

## 5. pasta に期待する事

| # | 期待 | 骨子・取り決めとの照らし |
|---|------|--------------------------|
| 1 | 高速再生の 1 本は、字より前に `\_q` を 1 つ打つ。閉じは要らない（1 本の終わりで切れる）。 | **合っている**。取り決め「命令の表」: 「字を一度に出す（高速再生） \| 正典の `\_q`。pasta が `\x` と瞬間の命令を出さない \| その 1 本」 |
| 2 | 高速再生の 1 本に、`\w`・`\_w` を出さない。出せば、areka は書いた長さだけ待つ（正典どおり）。 | **合っている**（待ちのト書きが「瞬間の命令」に入るなら）。ただし、辞書の単語に作者が直書きした `\_w`（骨子 2.2 の例 `＠稲光：\![parallel]\s[12]\_w[200]\s[10]\_w[200]\s[12]`）を pasta が外せるかは、取り決めに書いていない（9 章） |
| 3 | 高速再生で、行が出る前に次の行で置き換わるのを防ぐ待ちは、`\_w[ミリ秒]` で入れる。 | **合っている**。`play-script-guide.md` 221 行「起きるなら、pasta が行の末尾に短い待ちを入れる」。`\_q` の中でも待ちは効く |
| 4 | 出し直しの 1 本は `\![play,again]` だけでよい。`\_q` を併せて打たなくてよい（打っても同じ）。印は、その 1 本のどこに在っても頭から効く。 | **合っている**。取り決め「出し直しの印 … 字を一度に出す。`\w`・`\_w` を飛ばす。`\x` はふつうに待つ」。「どこに在っても」は、この設計が置いた仮定（取り決めは位置を書いていない） |
| 5 | 貯めない区間は、開きが `\__v[disable]`、閉じが裸の `\__v`。1 本の終わりまで貯めないなら、閉じは要らない。 | **合っている**。取り決め「貯めない \| 正典の `\__v[disable]` の区間」。正典の意味とも合う（4.5 の (3)）。areka の側に持ち主が居ないことだけが残る |
| 6 | areka へは `\![timerraise,…]` を出さない。 | **合っている**。取り決め「SSP へは、`\x` の時間を `\![timerraise,…]` … の形で出す（pasta が直す）」。出ても壊れない（誰も拾わず、記録だけ） |
| 7 | 遊ぶ人の「文字の速さ」を伝えたいときは、正典の `\![set,balloonwait,値]` を 1 本ごとに打つ。areka は値を覚えない。 | 食い違いは無い。取り決め「要件定義へ送ること」に「遊ぶ人の設定（文字の速さ・音量・窓の形）を伝える命令」が残っている。「正典を先に使う」に沿えば、新しい `play` の命令は要らない（勧め） |
| 8 | `\![quicksection,…]` は使わなくてよい（`\_q` で足りる）。 | 取り決めに記述は無い |

- イベント（GET か NOTIFY か・Reference）: この単位が受ける・送るイベントは無い。
- 順番とタイミング: `\_q` と `\![set,balloonwait,…]` は、書いた位置から後ろに効く。字より前に打つ。目印（`\![play,mark,…]`）との前後は問わない。

## 6. ほかの単位に期待する事

- **受け口の単位**:
  - `\![play,again]` の腕を `compile.rs` に足し、`Pace::instant()` を初期値にする。先に着地する側が `Pace` を作る（この単位が先、を仮定）。
  - `\![play,again]`・`\![play,xtimeout,…]` は、コンパイルが意味を読む命令になる。brief の「allowlist 外の compile 解釈は引き続き禁止」（Desired Outcome）と `doc/COMPAT_ARCHITECTURE.md` 131 行の一覧に、この 2 つを足す必要がある。一覧の文面を直すのは、足す側（受け口の単位）。
  - `\__v` の読む段の腕・運び手の名前・受け取り手の表の行・バックログの受け手を持つ（4.5 の (3)）。台帳の `\__v` の行の持ち主になる。
- **`talk-fast-forward`**: brief は「この単位は `talk-fast-forward` の後」と書く。暫定の設計では、順はどちらでもよい。向こうが `\x` の後ろの時刻を付け替える形（向こうの議題 ⑴）にしても、`Pace` は時刻の積みの手前の関数なので、ぶつからない。
- **`parallel-visual-script`・`window-timed-directives`・`sound-canon-tags`**: 4.7 の決まり（時刻を進める腕は `Pace` に通す）に乗る。
- **同じファイルを触る単位**（同じウェーブに置けない）:
  - `compile.rs`: この単位・受け口・`talk-fast-forward`・`parallel-visual-script`・`window-timed-directives`・`sound-canon-tags`・`range-choice-tag`。
  - `decode.rs`・`model.rs`: この単位・受け口（`\__v`・`\x`）・`talk-fast-forward`（`\x`）・`sakura-time-critical`。
  - `consumer_ledger.rs`: `\!` の名前を足す単位の全部。
- **段**: この単位は、roadmap で「その他・区分 C」（424 行）に在る。取り決めは `\_q` を正典の物として areka に求めるので、ノベルゲーム基盤の段で、受け口の単位より前（か同じ時）に要る。段の付け替えは棚卸で決める。

## 7. 置いた仮定

1. `\_q` の区間は `\x`・`\c` で切れない（ukadoc は「次の\_qか\eまで」とだけ書く）。
2. `quicksection` の値は `true`・`1`・`false`・`0` だけを受ける。brief の語彙の表の「数値」は、ukadoc に 0 と 1 しか無いので、ほかの数は読めない値として扱う。
3. `balloonwait` の端数は、1 字あたりのミリ秒に直す時に 1 回だけ四捨五入する。SSP の丸め方は測っていない。
4. `balloonwait` の範囲の外（数だけで 100 より上・負・`%` や `ms` で 10000 より上）は、読めない値と同じに既定として扱う。
5. `balloonwait` も `quicksection` も、書いた位置から後ろにだけ効く。
6. 3 語の運び手の cue は、今までどおり出す（`choicetimeout` と同じ。受け手は名前で担当外と判じる）。
7. `\![play,again]` は、その 1 本のどこに書いても頭から効く（先読み）。受け口の単位が腕を持つ。
8. 出し直しの「待ちを飛ばす」は、並走の区間の中の待ち・時間つきの窓の指令の待つ形・音の待ちにも効く。
9. `\![sound,wait]`・`\_V` は `sound-canon-tags` が丸ごと持ち（向こうの暫定の設計と合っている）、`\![wait,syncobject]` は置いたままにする。台帳の欄は、先に着手する側が付け替える・空にする。
10. `\__v` は、読む段の腕も受け手も、受け口の単位が持つ。
11. この単位が、`compile.rs` を触るノベルの単位の中で最初に着地する（`Pace` を作る）。
12. 口パクの写し（`talk.rs`）は、再生時間 0 の字が 1 回の刻みで区切りを 2 つ以上越えても、最新の 1 つだけ鳴る（brief 10-10 の申し送りの文に依る。配線を通したテストは無い）。

## 8. 決まっていない所

開発者に諮る問いの形で書く。

1. **`\__v` の持ち主をどこに置くか。** 受け口の単位（バックログ）が丸ごと持つ（推し）か、読む段の腕と運び手だけをこの単位に入れる（同じ `decode.rs` を触るため）か。→ 後者なら、この単位のタスクが 2 つ増え、台帳の持ち主が替わる。
2. **この単位の段を、ノベルゲーム基盤の段へ上げるか。** → 上げないと、受け口の単位が `\_q` と `Pace` を自分で作ることになり、この単位と同じ物が 2 つできる。
3. **出し直しの時刻の作りは、コンパイルで 0 にする（推し）か、早送りの時計の早回しを頭で掛けるか。** → 後者なら要件 12 と `Pace::skip_waits` は要らず、受け口の単位が `talk-fast-forward` の着地を待つ。
4. **`balloonwait` の端数と範囲の外を、SSP の MCP で測ってから決めるか。** → 測った結果で、4.3 の直し方（四捨五入・既定として扱う）が、切り捨てや clamp に替わる。
5. **`\__w[時間]`（再生開始からの通しの待ち）を、この単位に入れるか。** 持ち主が居ない時間の指令で、コンパイルは通しの時刻を知っているので、待ちの長さを引き算で出せる。ノベルには要らない。→ 入れるならタスクが 1〜2 増える。
6. **出し直しの 1 本に `\_V`（音の待ち）が在った時、区切りを置かない、でよいか**（4.7）。→ 置くなら、出し直しが音の終わりを待って止まる。`sound-canon-tags` の `compile.rs` の腕が `Pace` を見るかどうかが変わる。
7. **`\![timerraise,…]` を areka で受ける日が来たら、誰が持つか。** 今は持ち主が居ない。`\![raise]` と同じ群なので、受け口の単位か、新しい brief。→ この単位の設計は変わらない。

## 9. 差し戻しが要りそうな所

差し戻しが**必ず要る**矛盾は、見つからなかった。確かめを頼みたい所が 1 つ在る。

- **高速再生の間の、pasta が外せない待ち。**
  - 上位の文: 骨子 2.6「高速再生: pasta が `\_q` を付け、`\x` と瞬間の命令を出さない。」・骨子 3 章の対応表「`\![play,backlog,0／1]`・`\![play,skip,1]`・`\![play,auto,1]`・`\![play,screen,モード]` \| 正典の `\__v[disable]`・`\_q`、`xtimeout`、舞台のト書き」。
  - 正典の文（ukadoc `\_q`）: 「ただし、ウェイトのかかるさくらスクリプト全般は実行され、瞬間表示中もウェイトがかかる。」
  - ぶつかり方: 取り下げた `\![play,skip,1]` は、areka の側で待ちも飛ばす案だった（`areka-novel-skip-contents` の表）。`\_q` に替えたので、待ちを外すのは pasta だけになった。pasta がト書きの待ちを出さないのは決まっているが、辞書の単語に作者が直書きした `\w`・`\_w`（並走の区間の中を含む）は、pasta が展開した後の文字列に残る見込みである。残れば、高速再生の行が、その長さだけ遅くなる。
  - 壊れはしない（終わりの絵は同じ）。areka の設計も変わらない。決めるのは「受け入れる」か「pasta が、展開した後の文字列から待ちのタグを落とす」かで、pasta の側（`novel-talk-flow`）の要件で足りる見込み。骨子の文を変えるほどではない。

## 10. 規模の見立て

- brief の見立て: 8〜12 タスク（棚卸㉓・㉔）。
- この設計での見立て: **9〜13 タスク**。

| まとまり | タスク |
|----------|--------|
| 読む段（`\_q` の腕・運搬名・見本の直し） | 1〜2 |
| `Pace` と値の読み取り（純関数とテスト） | 2〜3 |
| `scan` の腕の差し替えと決定論テスト（区間・倍率・混ぜ書き・`\e`・待ちは残る） | 2〜3 |
| 待ちを飛ばす口（`Pace::instant()` とテスト。腕は受け口の単位） | 1 |
| 受け取り手の表・strict・`check_script` | 1〜2 |
| 台帳・`COMPAT_ARCHITECTURE.md`・実機の確かめ（`talk,数値` と `\_q`） | 2 |

- 差: 上の幅で 1 つ増えた（待ちを飛ばす口）。`\__v` の読む段を入れるなら +2、`\__w` を入れるなら +1〜2。どれを入れても 20 には届かない。切る案は無い。

## 11. 確かめた物と机の上だけの物

- **実行して確かめた物**: 0。
- **コードを読んで確かめた物**:
  - 時刻を進める腕は `Wait`・`Text`・`SystemVar` の 3 つ（`compile.rs` の `scan` を全部読んだ）。
  - `\_q`・`\__v` は字句で切れて `Raw` になる（テストの見本を読んだ）。
  - 再生時間 0 の字は cue の時刻に全部見える（emo-text の `state.rs` の式と説明・seriko の `talk.rs` の式）。
  - `choicetimeout` の前例の形（読む関数・ローカルの状態・表の行・strict の点）。
  - 台帳の持ち主の欄（3.4 の表の行）。
  - `quicksection`・`balloonwait`・`timerraise`・`xtimeout` が crate に 0 件。
- **ukadoc を引いて確かめた物**: `\_q`・`quicksection`・`balloonwait`・`\__v`・`\_w`・`\__w`・`\x`・`timerraise` の本文（引用は本文の各所）。
- **机の上だけの物**（読んでいない・動かしていない）:
  - 口パクが、1 回の刻みで区切りを 2 つ以上越えた時の動き（仮定 12）。
  - `check_script` の判定（`crates/areka/src/mcp/check_script_judge.rs`）と strict の配線の、足す所の細部。
  - `consumer_ledger_agreement_tests.rs` に見本を足す形。
  - 高速再生の 1 本（占有 horizon が 0 に近い）で、絵が 1 フレーム出るかどうか。受け口の単位と実機の話である。
  - SSP の `balloonwait` の丸めと範囲の外・`\_q` と `\x` の関係。
  - pasta が辞書の単語の中の待ちを外せるか（9 章）。
  - 兄弟の暫定の設計との突き合わせは、`sound-canon-tags` の 1 本だけ（grep で当たった行を読んだ。全文は読んでいない）。受け口の単位・`talk-fast-forward`・`parallel-visual-script`・`window-timed-directives` の物は、まだ無かった。
