# ギャップ分析: script-small-tags

- 日付: 2026-10-31
- 基準: ワークツリー `claude/kiro-start-script-small-tags-97dc1c`（main `da020c10` の上・要件は生成済み・未承認）
- 読んだもの: `requirements.md`・`brief.md`（「2026-10-31 棚卸㉛の再測定」）・steering（`roadmap.md` のウェーブ C14 の行と「1 ウェーブに 1 本だけの席」）・下に挙げたソース
- この文書は情報と選択肢を並べる。決めるのは要件の討議と設計。

## 1. まとめ

- `\_u`・`\_m` と `\![bind-noevent,…]` は、今ある型（読み手の腕・着せ替えの受け手の名前の選別・受け手の表の 1 行）に足すだけで済む。規模は小さく、ウェーブ C14 の約束（`emo2_boot/assets.rs` と emo-text の `actor_present.rs` に触らない）の内に収まる。
- `%*` は今、字句の段で「名前が空の環境変数」と「本文の `*`」に割れている。`\![*]` と同じに読むには、字句の段（`lexer.rs`）か読み手の段（`decode.rs`）のどちらかに手を入れる。roadmap の触るファイルの一覧には `lexer.rs` が無い（他のウェーブの兄弟とは重ならない）。
- `\![*]` の印を**描く**のは、文字の層（emo-text）に「絵を 1 文字分として行の中に置く」仕組みを新しく作ることになる。これは `balloon-inline-image`（`\_b[…,inline]`）が作ろうとしている仕組みと同じもの。規模は中くらい（タスク 8〜12）で、この spec の他の部分よりずっと大きい。
- 印の絵の読み込みは、`emo2_boot/assets.rs` に触らずにできる道がある（emo-present の `balloon_definitions.rs` の中で読む）。ただし「面ごとの定義」`FaceDefinition` に欄を足すと、`assets.rs` 523 行目の構造体の書き下し（テスト用の `empty_definitions`）が壊れて、`assets.rs` を直すことになる。欄は `ScopeBalloonDefinitions`（中身が非公開・作り手関数がある）の側に足せば避けられる。
- 要件の Adjacent の「この spec は受け手の表に 1 件だけ足す」と、印を受け手の表の運搬名で文字の層へ運ぶ形（`\f` の前例）は両立しない（41 件でなく 42 件になる）。描くなら、運び方を決める議題が 1 つ増える。

## 2. 今のコードの姿

### 2.1 台本の読み手（`crates/areka-parsers/src/sakura/`）

| ファイル | 行数 | 今の働き（この spec に関わる所） |
|---|---|---|
| `lexer.rs` | 426 | `\_u[0x3000]` は `Tag { word: "_u", args: ["0x3000"] }` として読める（`lexer_word_boundary_tests.rs` の 19 綴りに `_u`・`_m` が入っている）。`\_u[]` の引数は空の列（`\s[]` と同じ）。`%` は `scan_sysvar`（412 行目）が英数字と下線だけを名前に取るので、`%*` は `SysVar("")` と本文 `*` に割れる。`substitute_system_vars` も同じ走査を使う。 |
| `decode.rs` | 510 | `decode_tag`（274 行目）に `_u`・`_m` の腕が無く、`decode_passthrough_tag` が `Raw` と印 `UnknownTag` を付ける。`\![*]` は `decode_noted` の先読み（93 行目）で `fold_choice_marker`（122 行目）へ行き、直後の `\q` と畳んで `Choice` 1 つにし、印 `MarkerIgnored` を付ける。単独なら `GenericCommand { name: "*" }`。 |
| `model.rs` | 178 | `Instruction`（`#[non_exhaustive]`）・`ReadNote`（`#[non_exhaustive]`・`UnknownTag`／`Unclosed`／`ArgumentDefaulted`／`MarkerIgnored`）。 |
| 兄弟のテスト | — | `decode_tests.rs`（699）・`parse_noted_tests.rs`（290・242〜251 行目が `\![*]` の畳みと印を固定）。 |

- 読み手は今、記録（`tracing`）を出していない。クレートは `tracing` に依存している（`Cargo.toml`）。
- 読み手の呼び手は 3 つ: 再生（`areka-sakura/src/drive.rs`。strict のときは `parse_noted`、そうでなければ `parse`）・MCP の `check_script`（`parse_noted`）・説明書の行き先（`areka/src/readme/destination.rs` の `parse`）。読み手で記録を出すと、再生しない `check_script` でも記録が出る。

### 2.2 台本の組み立て（`crates/areka-sakura/src/compile.rs`・625 行）

- `Instruction::Text` は 1 つにつき 1 つの文字の合図（`CueCommand::Text`）を作り、再生時間は書記素クラスタの数 × 1 文字の時間（`duration.rs`）。環境変数（`SystemVar`）の腕も、展開した文字を**隣の文字と合わせずに**別の合図にしている（「観測同一」と書いてある前例）。
- 知らないタグを捨てる残りの腕は 422 行目。`Raw` は `debug!` だけで捨てる（警告の水準ではない）。
- strict の連れ `Tracer` は、読みの印を `DetectionPoint` へ写し（105〜118 行目）、`\!` は `command`（121 行目）で受け手の表を引く。`*` と `MarkerIgnored` の組だけは表を引かない特別扱いがある（123〜129 行目）。
- `compile()`（strict でない道）は `Instruction` しか受け取らない。読みの印（`ReadNote`）は strict の道でしか見えない。

### 2.3 台本の確かめ（`check_script` と strict）

- `crates/areka/src/mcp/check_script_judge.rs`（164 行）: 印を診断の種類へ写す（`ArgumentDefaulted` → `unreadable_argument`・`MarkerIgnored` → `ignored`）。`\!` は受け手の表 `ConsumerLedger::consumer_of` を引き、無ければ `unknown_command`。単独の `*` は特別扱いで表を引かない（69〜71 行目）。
- `crates/areka/src/emo2_boot/strict_judges.rs`（48 行）: strict の「知らない命令」は同じ表の答えを写すだけ。`bind-noevent` を表に 1 行足せば、`check_script` と strict の両方で「知らない命令」が消える（別の直しは要らない）。
- 文言の正本 `crates/areka-sakura/src/strict.rs`（220 行）: `MSG_DEFAULTED` は「引数が無いか読めないので既定の値を使う」。`\_u` の捨てる扱いには文言が合わない（議題 3）。

### 2.4 着せ替えの受け手（`crates/areka-seriko/src/actor.rs`・908 行・上限まで 92 行）

- `BIND_COMMAND = "bind"`（59 行目）。名前の選別は 656 行目の `if name != BIND_COMMAND`（ここで `bind-noevent` は「担当外」として `debug!` で読み流される）。壊れた運び物の宛名の判定が 633 行目。
- 名前が `bind` に決まった後の道（引数の読み `parse_bind_directive`・名前の解決 `apply_named_bind`・トグル形とカテゴリ単位形の読み飛ばし・記録）は名前に依らない。`bind-noevent` を同じ道へ入れれば、要件 3.1〜3.3 は今の決まりのまま満たせる。
- 記録の行（672〜760 行目）には今、名前の欄が無い。要件 3.5（どちらの名前で来たかを記録に残す）には、各記録に `name` の欄を足す。
- シェルの読み直しの持ち越し `emo2_boot/shell_carry.rs` は、seriko の今の着せ替えの状態を読んで `\![bind,…]` で流し直す。`bind-noevent` も同じ状態を変えるので、要件 3.4（持ち越し）は手を入れずに満たせる見込み。
- `BIND_COMMAND` を使う所は 3 つ（`actor.rs`・`consumer_ledger.rs` 400 行目・`shell_carry.rs` 369 行目）。前置きで `bind` を拾う所（`starts_with("bind")`）は無い。

### 2.5 受け手の表（`crates/areka/src/emo2_boot/consumer_ledger.rs`・529 行・40 件）

- `bind` → `CommandConsumer::Seriko` の 1 行（400 行目）。`bind-noevent` も同じ担当で 1 行足せばよい（新しい受け口は要らない＝`emo2_boot/mod.rs`・`runtime_boot.rs` は触らない）。
- 手書きの数: `consumer_ledger_tests.rs` 298 行目の `40` と、その下の説明の一覧（名前を全部並べた文）。見本: `consumer_ledger_agreement_tests.rs` の `samples()`（57 行目あたりに `("bind", None, …)`・見本と表の行が同じ組かも判定する）。
- 前例: 文字の層が受ける `\f` は運搬名 `FONT_TAG_CARRIER` → `TextLayer` の行。印を同じ形で運ぶなら、`*` → `TextLayer` の行がもう 1 つ要る（議題 2）。

### 2.6 印の族と、バルーンの絵の読み込み

- 印の族の表 `crates/areka-emo-present/src/balloon_marker.rs`（287 行）: スクロールの矢印 `ARROW_FAMILY` とクリック待ち `CLICKWAIT_FAMILY`。名前の連鎖（書いた名前 → スコープ別の名前 → 旧名）は `marker_chain` と `prefix_chain`。ただし絵の選びは**番号つき**（`arrow0.png`・`arrow1.png`・`face_digits_of`）で、番号の無い `marker.png` を選ぶ関数は無い。
- 鍵の読み `crates/areka-parsers/src/balloon/marker.rs`（117 行）: `arrow.filename` と矢印の座標の鍵だけ。`marker.filename` を読む所はどこにも無い。
- 矢印の絵を読むのは emo-present の `balloon_definitions.rs`（133 行）の `load_scope_balloon_definitions`。面ごとの定義 `FaceDefinition { model, arrows }` に束ね、`emo2_boot/frame/balloon_face.rs`（107 行）が `runtime.set_balloon_markers(…, definition.arrows)` で文字の層へ渡す。
- `emo2_boot/assets.rs`（540 行）は `load_scope_balloon_definitions` を呼ぶだけ（488 行目）。関数の形を変えなければ触らない。**罠**: 523 行目（テスト用の `empty_definitions`）と `frame/switch_finish_face_tests.rs` 13 行目が `FaceDefinition { model, arrows }` を欄を並べて書いている。`FaceDefinition` に欄を足すと `assets.rs` がコンパイルできなくなる。
- 網羅台帳 `assets.toml` 1781 行目: `marker.filename` は absent・持ち主 `balloon-canon-residue`（いつか）。

### 2.7 文字の層（`crates/areka-emo-text/src/`）

- 文字は書記素クラスタ 1 つを `TextItem::Glyph` 1 つにして並べる（`state.rs` 510 行目）。分かち書き（`segment.rs`）は改行で区切ったひと続き全体でかけるので、文字の合図が分かれていても折り返しの決め方は変わらない。ただしクラスタの切り分けは合図 1 つの中で行うので、`\_u` で結合文字（例 U+0301・異体字セレクタ U+FE0F）だけを埋めると、前の字と別のクラスタになる（議題 4）。
- 空白だけの本文を特別に扱う所（`trim`・`is_whitespace`）は、文字の層にもバルーンの出し入れ（`emo2_boot/balloon_visibility*`）にも無い。全角の空白の `Text` はふつうの字と同じにバルーンを開く見込み（要件 1.6）。
- 「絵を 1 文字分として行に置く」仕組みは無い。描画は行ごとに DirectWrite の `IDWriteTextLayout` を作って描く（`draw.rs` 499 行目）。DirectWrite には「行の中の 1 文字分の物」（`IDWriteInlineObject`・`SetInlineObject`）があり、emo-text は自前の `IDWriteTextRenderer` を既に実装している（`viewbox_draw_underline.rs` の `UnderlineRecorder`。`DrawInlineObject` の受け口は空のまま）。送り幅は `layout.rs` 120 行目の `advance(text, font_height)` が字の形から測る。
- スクロールの矢印は文字の面とは別の面（`marker_surface.rs`）に描いている。選択肢の前の印は行の中で文字と一緒に動く（要件 5.5・5.6）ので、この形は使えない。
- ファイルの大きさ（上限 1,000 行）: `surface.rs` 995（余地なし）・`draw.rs` 841・`actor.rs` 825・`choice.rs` 790・`state.rs` 755・`layout.rs` 551。足すなら新しいファイルが要る＝「emo-text の 1 本だけの席」を使う。roadmap（204 行目）では C14 でこの席を使う spec は無い。

### 2.8 網羅台帳と文書

- `doc/ukadoc-coverage/ledger/sakura-script.toml`: `%*`（9 行目・vocabulary-only・持ち主が空）・`\![bind-noevent,…]`（710 行目・absent・持ち主 `script-small-tags`）・`\_m[0x00]`（4257 行目・absent・持ち主が空）・`\_u[0x0000]`（4335 行目・absent・持ち主 `script-small-tags`）・`\![*]`（absent・持ち主 `script-small-tags`）。
- `doc/COMPAT_ARCHITECTURE.md` §8（124 行目）が正典の黙る所の areka の決めの表。
- `doc/pasta-real-run.md` 5 節の段 2（184 行目）が着地の後の確かめの行。

### 2.9 SSP のバルーンの手元の事実（読むだけで確かめた）

- この機械の SSP の根 `C:\wintools\ssp\balloon\pasta_tategaki` に `marker*.png` は無い（`arrow0/1.png`・`online*.png`・`sstp_new.png` は在る）。SSP の既定のバルーン `ssp`・`ssp_vertical` にも無い。`emily4`・`emo-kakukaku`・`emo2-kakukaku` には `marker.png` が在る。
- つまり SSP は、印の絵を持たないバルーンでも自前の印を描いている（`ssp-measurement.md` 段 2 の「選択肢 4 つが、それぞれ印付き」）。その絵の形・大きさ・選択肢の字との間は記録が無い（撮った物 `stage2-choice-shot.png` は元のワークツリーごと消えていて見られない）。

## 3. 要件と今のコードの対応

| 要件 | 使える物 | 足りない物 | 種類 |
|---|---|---|---|
| 1.1〜1.4 `\_u`・`\_m` を字にする | 字句は読める・`Instruction::Text` と文字の合図 | `decode_tag` の `_u`・`_m` の腕と番号の読み（`0x` の小文字だけ・16 進の大小・10 進） | 足りない |
| 1.5 直に書いたのと同じ | 環境変数の前例（別の合図でも観測同一）・分かち書きは合図をまたぐ | 結合文字などクラスタが前の字とつながる番号は、合図が分かれると別のクラスタになる | 制約（議題 4） |
| 1.6 全角の空白だけでバルーンが開く | 空白の特別扱いが無い | なし（テストで固定するだけ） | — |
| 1.7 埋めた字を読み直さない | `Text` は組み立て以後に読み直されない。環境変数の置き換えは字句の前の生の台本に対して行う | なし（テストで固定するだけ） | — |
| 1.8 書いた順・位置 | 読み手は入力の順に出す | なし | — |
| 2.1〜2.6 捨てて警告 | `Raw`・印の仕組み | 警告の水準の記録を出す場所（読み手は記録を出さない・strict でない組み立ては印を見ない）・理由を運ぶ形 | 足りない（議題 3） |
| 2.7 何も出さず続ける | `Raw` は合図を作らない | なし | — |
| 3.1〜3.3 `bind-noevent` を `bind` と同じに | 着せ替えの道は名前に依らない | 名前の選別（656・633 行目）に 1 つ足す | 足りない（小） |
| 3.4 持ち越し | `shell_carry.rs` は状態から流し直す | なし（テストで確かめる） | — |
| 3.5 名前の区別を記録に | 運び物は名前を持っている | 記録の行に名前の欄 | 足りない（小） |
| 3.6 イベント 0 | 今どちらも送らない | なし（送らないことをテストで固定） | — |
| 3.7 他の受け手は読み流す | 名前の選別は完全一致 | 受け手の表の 1 行（見本のテストが他の受け口に届かないことを見る） | 足りない（小） |
| 4.1〜4.2 印の位置を後ろへ渡す | 畳みと単独形の汎用の命令 | 畳まずに「印」と「選択肢」を別々に出す形・印を受ける層 | 足りない（議題 1・2・6） |
| 4.3 `%*` | sylphya の語彙は `%*` を「構文の記録・解決の外」としている | `%*` を `\![*]` と同じに読む所（字句か読み手） | 足りない（議題 5） |
| 4.4・6.5 「無視した」をやめる | 印 `MarkerIgnored` と 3 か所の特別扱い | 描く働きと同時に外す | 議題 1 しだい |
| 5.1〜5.7 印の表示 | 印の族の名前の連鎖・矢印の絵の読み込みの前例・自前の `IDWriteTextRenderer` | 番号の無い絵の選び・`marker.filename` の鍵の読み・行の中に 1 文字分の絵を置く並べと描画・areka の既定の印の絵・印の消去とスクロールの連動 | 足りない（大・議題 1） |
| 6.1〜6.4 確かめの道具 | 表を写す作り | `_u`・`_m` の印の写しと文言・表の 1 行 | 足りない（小） |
| 7.1〜7.3 台帳と §8 | 台帳の行は在る | 行の書き換え・§8 の行 | 足りない（文書） |
| 8.1〜8.3 確かめ | 兄弟のテストの置き方・手順書 | 新しい兄弟のテスト（seriko の `bind-noevent`） | 足りない |

## 4. 実装の道の選択肢

### 4.1 `\_u`・`\_m`（要件 1・2・6.1〜6.3）

どの道でも、出せる番号は `Instruction::Text(1 文字)` にする（腕は `decode_tag` に 1 つ・番号の読みは小さな関数）。違いは「捨てたときの警告をどこで出すか」と「理由をどう運ぶか」。

- **道 A1（新しい命令の種類）**: 捨てる番号は `Instruction` に新しい種類（例: 元の綴りと理由を持つ「捨てた文字の番号」）を足す。組み立てに腕を 1 つ足して `warn!` を出し、合図は作らない。`check_script` と strict はこの種類を「読めない引数」に写す。
  - 良い点: 理由が命令と一緒に運ばれ、strict でない再生の道でも警告が出る。読み手は記録を出さないままでいられる。`check_script` では記録が出ない。
  - 悪い点: `Instruction` を見る所（`anchor_pair.rs`・`duration.rs`・`sysvar.rs`・`spine_conformance_script.rs`・`readme/destination.rs`）の残りの腕を確かめる手間。`model.rs` が少し広がる。
- **道 A2（読み手で警告）**: 捨てる番号は今の `Raw` にして、読み手が `warn!` を直に出す。印は新しい種類（例: 捨てた番号）か、今の `ArgumentDefaulted`。
  - 良い点: いちばん小さい。
  - 悪い点: 読み手が初めて記録を出す（「読み手は記録を出さない寛容な写像」という今の流儀から外れる）。`check_script` で確かめただけでも警告が出る。
- **道 A3（今の印を流用）**: `Raw`＋`ArgumentDefaulted`。警告は道 A2 と同じく読み手。
  - 悪い点: `check_script` の文言が「既定の値を使う」になり、実物（捨てる）と食い違う。

番号の読みの細かい所（どの道でも同じ）: `0x` の小文字だけ・16 進の数字は大小を問わない・先頭の 0 は問わない・空の列・引数 2 つ以上・空白・符号は読めない番号。`char::from_u32` はサロゲートの片割れを `None` で返すので、その判定に使える（他の範囲は自前の判定）。

### 4.2 `\![bind-noevent,…]`（要件 3・6.4）

道は実質 1 つ。

- `actor.rs` に定数 `BIND_NOEVENT_COMMAND`（または `bind.rs` に「着せ替えの名前か」を答える小さな関数）を足し、656 行目と 633 行目の判定に入れる。記録の行に `name` の欄を足す。足す行は 10 行前後（余地 92 行の内）。テストは新しい兄弟のファイル（`actor_bind_noevent_tests.rs` など）に置き、`actor.rs` に `#[path]` の 3 行を足す。
- `consumer_ledger.rs` に `bind-noevent` → `Seriko` の 1 行・説明の一覧の文。`consumer_ledger_tests.rs` の手書きの数 40 → 41 と説明の文。`consumer_ledger_agreement_tests.rs` の見本に 1 行。
- 「イベントを起こさない」の目印は、運び物が名前を持っていることそのもの（受け手は名前を区別できる）。送る spec ができたときに使う分かれ目は、656 行目の後に名前で分ければ済む。今は分ける所を作らない（作っても使う所が無い）。

### 4.3 `%*`（要件 4.3）

- **道 B1（字句の段）**: `scan_sysvar` で `%` の直後が `*` なら名前 `*` の `SysVar` にする。読み手は `SysVar("*")` を `\![*]` と同じ命令へ写す。`substitute_system_vars` は名前 `*` で引くが、解決の答えは「無い」なので台本は変わらない（sylphya の語彙も `%*` を解決の外としている）。
  - 良い点: 1 か所で済み、範囲（span）も `%*` の 2 文字を正しく覆う。
  - 悪い点: roadmap の触るファイルの一覧に無い `lexer.rs` を触る（C14 の兄弟は誰も触らない）。
- **道 B2（読み手の段）**: `SysVar("")` の直後の本文が `*` で始まるとき、読み手が組にして畳む。
  - 悪い点: 本文を切り分ける特別扱いになり、範囲の計算も分かりにくい。

### 4.4 `\![*]` の印（要件 4・5・6.5・議題 1）

- **道 C1（この spec で描く・全部）**
  - 読み手: `fold_choice_marker` をやめ、`\![*]` を「印」1 つ、続く `\q` を今どおりの選択肢 1 つとして別々に出す（畳む関数を消すのがいちばん小さい）。`MarkerIgnored` と 3 か所の特別扱い（`compile.rs` 123 行目・`check_script_judge.rs` 69 行目・印の写し）を外す。
  - 運び方: (a) 運搬名 `*` の汎用の合図にして受け手の表に `*` → `TextLayer` を足す（`\f` の前例・表は 42 件になり、要件の「1 件だけ足す」と食い違う）か、(b) dola の合図に印の種類を足す（アンカーの前例・網羅の match を持つ emo-text・`areka-ghost/src/sink.rs`・dola の `sink.rs`・テストの支えへ波及・表は増えない）。
  - 絵の読み込み: 鍵 `marker.filename` の読み（`areka-parsers/src/balloon/marker.rs`）・番号の無い名前の選び（`balloon_marker.rs`）・読み込み（`balloon_definitions.rs`）。`assets.rs` に触らないため、絵は `FaceDefinition` でなく `ScopeBalloonDefinitions` の非公開の欄に持つ（または関数の形を変えない別の受け渡し）。`frame/balloon_face.rs` から文字の層へ渡す。
  - 文字の層: 印を 1 文字分の項目として並べ（送り幅＝絵の幅、縦書きは高さ）、描くときに絵を置く（DirectWrite の行の中の物を使うか、送り幅だけ取って自分で描くか）。消去・押し出し・スクロールは文字と同じ項目の列にいるので自然についてくる見込み。選択肢の当たりは `\q` の字の範囲だけを数えるので、印は外に出る（要件 5.7）。新しいファイル＝emo-text の席を使う。
  - areka の既定の印の絵（要件 5.4）: 絵を同梱するか、図形を描くか。形は SSP を測ってから（調べる事項 1）。
  - 規模: タスク 8〜12。危うさ: 中〜高（新しい並べの項目・縦書き・拡大率・DirectWrite の行の中の物）。
- **道 C2（この spec は既定の印だけ・絵の読み込みは後）**
  - 印を「1 文字分の既定の印」として並べて描くところまでをこの spec で作り、`marker.filename` と絵の読み込み（要件 5.2・5.3）は受け取る spec（`balloon-canon-residue` か `balloon-inline-image`）へ渡す。ぱすたは `marker.png` を持たないので、v0.1.0 の確かめ（件 3）はこれで閉じる。
  - 既定の印を字の形（書体の記号の字）で出すなら、並べの新しい項目すら要らず、今の文字の道に乗る。ただし印が台詞の 1 文字と同じに数えられ（一字ずつ出る時間が 1 字分増える・`dump_balloon` に字として出る）、SSP の自前の印（絵）と見た目が合うかは測るまで分からない。
  - 規模: 字の形なら 2〜3 タスク・1 文字分の絵の項目なら 5〜8 タスク。危うさ: 低〜中。
- **道 C3（描かずに渡す）**
  - この spec は `%*` を `\![*]` と同じに読む所までにして、`\![*]` は今の「無視した」のまま（要件 4.4・6.5 の条件の側）。描く仕事は v0.1.0 の中の持ち主へ渡す（`balloon-canon-residue` を v0.1.0 へ上げる・`balloon-inline-image` を v0.1.0 へ上げて印も持たせる・`/kiro-discovery` で新しく起票する、のどれか）。
  - 気をつける点: 要件 4.1・4.2 は条件なしで「印を出す層へ渡す」と書いてあるが、受ける層が無いあいだは渡す先が無い。渡す先が無いまま運搬名の合図にすると、`check_script` が「知らない命令」と言い出す（今の特別扱いが要る）。道 C3 にするなら 4.1・4.2 を「印を出す働きが在るあいだ」の条件つきにするか、読み手の形だけ先に変えるかを決める。
  - v0.1.0 の着地までのウェーブの数が増えるかもしれない（受け取る spec を C15・C16 のどこに入れるか）。
  - 規模: この spec はタスク 4〜6。危うさ: 低。

### 4.5 道の組み合わせの見立て

| 組み合わせ | タスク | 触るファイル（主なもの） | C14 の約束 |
|---|---|---|---|
| A1＋bind＋B1＋C3 | 4〜6 | `decode.rs`・`model.rs`・`lexer.rs`・`compile.rs`・`check_script_judge.rs`・`strict.rs`・`actor.rs`・`consumer_ledger*`・台帳・§8 | 守れる |
| A1＋bind＋B1＋C2（字の形） | 6〜8 | 上に加えて既定の印の字の扱い | 守れる |
| A1＋bind＋B1＋C1 | 10〜15 | 上に加えて emo-present の `balloon_marker.rs`・`balloon_definitions.rs`・パーサの `balloon/marker.rs`・`frame/balloon_face.rs`・emo-text の新しいファイルと `state.rs`・`layout*`・`draw*`・（運び方 b なら dola と受け手 3 か所） | 欄の置き場を選べば `assets.rs` に触らずに済む。`actor_present.rs` は触らずに済む見込み（選択肢の範囲は `choice.rs` と `state.rs` の側）。emo-text の席を使う |

## 5. 規模と危うさ

- `\_u`・`\_m`・`bind-noevent`・`%*`・台帳と文書: **S**（既にある型に足す・依存を足さない）。危うさ **低**（名前の選別と読み手の腕の前例が多い）。
- `\![*]` を描く（道 C1）: **M**（新しい並べの項目と描画・絵の読み込みの道・縦書きと拡大率）。危うさ **中〜高**（DirectWrite の行の中の物を emo-text で使った前例が無い・`surface.rs` に余地が無い・`balloon-inline-image` と同じ仕組みを先に作ることになる）。

## 6. 設計で決めること（議題の候補）

1. **印を描く仕事の持ち主**（要件の議題 1）: 道 C1（全部ここ）・C2（既定の印だけここ）・C3（描かずに渡す）のどれか。C2・C3 なら受け取る spec と、その spec を v0.1.0 のどのウェーブに入れるか。C1 なら emo-text の席をこの spec が使うことを roadmap に書く。
2. **印の運び方**（C1・C2 で絵の項目を作るとき）: 運搬名の汎用の合図＋受け手の表の行（表は 42 件・要件 Adjacent の「1 件だけ足す」を直す）か、dola の合図の新しい種類（表は増えない・受け手 3 か所へ波及）か。
3. **捨てた番号の警告の出し所と理由の運び方**: 道 A1（新しい命令の種類・組み立てで警告）・A2（読み手で警告）・A3（今の印を流用）。あわせて `check_script` の文言を新しく足すか（今の `MSG_DEFAULTED` は「既定の値を使う」で、捨てる扱いと合わない）。
4. **結合文字などの番号**: `\_u` で埋めた字が前の字と 1 つのクラスタになる番号（結合文字・異体字セレクタ・ZWJ）を、直に書いたのと同じにするか（読み手か組み立てで隣の本文と合わせる）、別のクラスタのままにして §8 に書くか。要件 1.5 は「同じにする」と書いている。
5. **`%*` を読む段**: 字句（道 B1・`lexer.rs` を触るファイルに足す）か読み手（道 B2）か。
6. **`\![*]` と `\q` を別々に出すか**: 畳みをやめると、`check_script` の範囲（span）も `\![*]` と `\q[…]` に分かれる（`parse_noted_tests.rs` 242〜251 行目を直す）。道 C3 のときに読み手の形だけ先に変えるか、今の畳みのまま残すか。
7. **`bind-noevent` の記録の形**: 名前の欄（例 `name = "bind-noevent"`）を全部の記録の行に足すか、適用の記録（`seriko: bind 適用`）だけか。実機の確かめ（件 18）で grep する目印をどれにするか。
8. **`\_m` の 10 進の表記**（要件の議題 2 の残り）: 正典の記述例に無い。受け付けるか。

## 7. 調べる事項（設計の段へ持ち越す）

1. **SSP の自前の印**（道 C1・C2 なら必須）: 印の絵を持たないバルーン（`pasta_tategaki` か SSP の `ssp`）で `\![*]\q[…]` を出し、ssp MCP の `dump_balloon` と撮影で、印の形・大きさ・選択肢の字との間・縦書きでの向きを測る。開発者の機械の SSP。比べに `emily4` の `marker.png` の大きさも見る。
2. **SSP の出せない番号**（要件の議題 2・任意）: `\_u[0]`・サロゲート・`0X` の大文字・`\_m[0x80]` を SSP が出すか捨てるか。測るなら ssp MCP の `sakurascript` と `dump_balloon`。
3. **DirectWrite の行の中の物**（道 C1）: emo-text の行ごとの `IDWriteTextLayout` に `SetInlineObject` を使えるか、縦書き（読みの向きを変えた行）での大きさの答え方、拡大率（`SetTransform`）との合わせ方。送り幅を測る `advance` の道（`layout.rs` 120 行目）に字でない項目を通す形。
4. **面ごとの `marker.filename`**（道 C1）: 面の上書きの設定ファイル（`balloons2s.txt` など）に `marker.filename` を書いたときに面ごとに変えるか。矢印は面ごと（`FaceDefinition`）に持っている。
5. **`Instruction` の残りの腕**（道 A1・C1 の b）: `#[non_exhaustive]` なので外のクレートは必ず残りの腕を持つが、その腕が黙って捨てて困る所が無いかを `anchor_pair.rs`・`duration.rs`・`sysvar.rs`（`join_texts`）・`spine_conformance_script.rs`・`readme/destination.rs` で確かめる。

## 8. 次の段へ

- この分析を要件の討議（`kiro-requirements-discussion`）の材料にする。特に議題 1（描く持ち主）と、上の 2・3・4 は要件の書き直しにつながる。
- 討議の後、`/kiro-design script-small-tags` で設計へ進む。
