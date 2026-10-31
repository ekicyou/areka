# Requirements Document

## Project Description (Input)
- **困っている人**: 右クリックメニュー（`crates/areka/src/menu/`）と説明書を開く道（`readme.rs`・`emo2_boot/readme_cue.rs`）へ次に手を入れる開発者と、障害のときに記録を読む人。
- **今の状況**: 右クリックメニューの第 1 スライス（`areka-P0-popup-menu-minimal`）は閉じたが、その検証とレビューが挙げた「直すほどではない／範囲外」の指摘が、閉じた spec の記録に書かれただけで引受先を持っていない。後から 3 件（件 11〜13）も足された。全 13 件のうち 4 件（件 1・2・7・10）は他の spec の作業で既に直っている。残りは、記録の文言の誤り（メニューの組み立ての失敗が「表示の失敗」と書かれる・照会の往復の失敗が「終了へ進む」と書かれる）、記録の型の不揃い、端から端までのテストの穴 2 つ、網羅台帳の引受先が閉じた spec のままの 4 行、実機での撮影や SSP との見比べが要る 3 件である。
- **変えたいこと**: 13 件それぞれが「直した」「直さないと決めた（理由付き）」「別の spec へ渡した」のどれかになる。記録の文が事実に合い、同じ道の記録が同じ型で読める。台本から説明書を開く道と、相方の窓からの終了の指示に、端から端までの決定論テストが付く。網羅台帳の引受先が、まだ閉じていない spec か本 spec の裁定を指す。メニューと説明書の動き（利用者に見えるもの）は変えない。

## Introduction

この spec は、右クリックメニューの第 1 スライスの拾い残しを 1 本にまとめて片づける台帳型の spec である（先例: `areka-P0-mcp-stdio-bridge-residue`・`balloon-canon-residue`）。件の番号は brief の件の番号である。

2026-10-31 の棚卸㉛のウェーブ C14 の約束により、件 11・12（実機でメニューを開いて撮る仕事）と件 13（直した後に SSP と見比べる仕事）は本 spec では直さず、引受先へ渡す。本 spec が直すのは件 3〜6・8・9 である。

### 13 件の仕分け

| 件 | 中身（短く） | 仕分け | 受け持つ要件 |
|---|---|---|---|
| 1 | 返事待ちの間に別の窓で預けた右ダブルクリックを、先の窓の判定で送りうる | 済み（他の spec が直した・確かめて記録する） | 2 |
| 2 | 台本の `\![open,readme]` が、説明書が無くても重い記録を出す | 済み（同上） | 2 |
| 3 | 台本の入口から説明書を開く所までを通す決定論テストが無い | 直す | 3 |
| 4 | メニューの組み立ての失敗が、表示の失敗の文で記録される | 直す | 4 |
| 5 | 記録の型が不揃い | 直す（説明書の受け口）／直さない（kanade の側・理由付き） | 5 |
| 6 | 照会の往復の失敗の文が「終了系列（Fault）へ」のまま | 直す | 6 |
| 7 | `spine.rs` の受け口の本数のコメントが古い | 済み（同上） | 2 |
| 8 | 相方の窓からの終了の指示を kanade の端から端まで通すテストが無い | 直す | 7 |
| 9 | 語彙のみで登記した 4 つの名前に引受先が無い | 渡す（3 人目以降の窓の部分）／直さないと決める（省略した形の部分） | 8 |
| 10 | 説明書を開く OS の呼び出しが描画を止めうる | 済み（同上） | 2 |
| 11 | メニューを開いている間、相方のバルーンが黒い四角に見える | 渡す | 9 |
| 12 | メニューが何も選ばれずに閉じる・出たはずなのに見えない | 渡す | 9 |
| 13 | 「．さくら」の古い資源の名前を問わない・無い項目がある | 渡す | 9 |

### この文書で使う言葉

- **相方の窓**: 2 人目のキャラクター（スコープ 1）の窓。本体の窓はスコープ 0。
- **説明書を開く道**: 右クリックメニューの「説明書」の項目、または台本の `\![open,readme]` から、ゴーストの説明書のファイルを OS の既定のアプリで開くまでの道。OS へ渡す所は開く専用のスレッドが受け持ち、テストでは偽物の OS で回す。
- **記録**: areka が残す構造化ログの 1 行。記録の水準（`error`・`warn`・`info`・`debug`・`trace`）、記録の種類の名前（`event=` の値）、文からなる。
- **記録の型**: 1 行の記録が、文の頭の接頭辞（`[menu]`・`[readme]` など）と `event=` の名前を持つかどうかの形。
- **網羅台帳**: `doc/ukadoc-coverage/ledger/shiori.toml`。正典の名前ごとに、対応の状態・持ち主の spec・備考（引受先を含む）を 1 行ずつ持つ。
- **省略した形のメニュー**: 正典が `*.popupmenu.type` の値 1 として名前だけを挙げているメニュー。中身（どの項目を残すか）は正典が定めていない（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#sakura.popupmenu.type>）。
- **生きている spec**: `.kiro/specs/` 直下に brief を持ち、`completed/` へ入っていない spec。
- **決定論テスト**: 全体テストが毎回走らせ、時間や机の状態に左右されずに同じ結果を出すテスト。

## Boundary Context

- **In scope**:
  - 13 件の仕分けと、その記録。
  - 件 3・8 のテストの追加（件 8 は `crates/areka-kanade/tests/kanade/` の終了の通しのテストの兄弟・件 6 のテストは `actor_resources_tests.rs`）。
  - 件 4・5・6 の記録の文と型の修正（`crates/areka/src/menu/{trigger,win32}.rs`・`menu/trigger_show_tests.rs`・`emo2_boot/readme_cue.rs`・`crates/areka-kanade/src/actor.rs` の文だけ）。
  - 件 9 の網羅台帳の 4 行の持ち主と備考、`roadmap-draft.md` の `areka-P0-popup-menu-minimal` の `owner_count` の 1 行、`menu/captions.rs` の問い合わせない名前の表の説明の追随。
  - 件 11・12 を `popup-menu-shell-look` の brief へ移すこと、件 13 を完了時の棚卸で新しい spec として起票すること。
- **Out of scope**:
  - 件 11・12・13 の再現・原因の特定・修正（引受先が行う）。
  - 省略した形のメニューを作り分けること、3 人目以降のキャラクターの窓を出すこと。
  - メニューの項目の追加・オーナードロー・タスクトレイのメニュー。
  - 閉じた spec（`areka-P0-popup-menu-minimal` ほか）の要件の改訂。
  - kanade の終了の進め方そのもの（記録の文だけを直す）。
  - `crates/areka/src/emo2_boot/spine.rs` の変更（件 7 は済み）。
- **Adjacent expectations**:
  - `popup-menu-shell-look`（生きている spec・オーナードローのメニュー）は、件 11・12 を自分の要件の段で引き受けるかを決める（同 spec の brief がそう書いている）。本 spec はその brief へ件 11・12 を写すだけで、向こうの要件を決めない。
  - `extra-character-windows`（生きている spec・3 人目以降の窓）は、brief の範囲に `char*.popupmenu.visible`（n≧2）を既に入れている。本 spec は網羅台帳と問い合わせない名前の表の引受先をそこへ向けるだけで、問い合わせる側へ移すのは向こうが行う。
  - 同じウェーブで並走する `script-small-tags`・`install-before-boot`・`placement-measure-bake-once`・`emo-atlas-palette-alpha`・`choice-ranges-one-function`・`actor-thread-log-capture` と、触るソースファイルは重ならない。網羅台帳はそれぞれ別の行だけを触る。

## Requirements

### Requirement 1: 13 件の仕分けを記録する

**Objective:** 次にメニューへ手を入れる開発者として、13 件それぞれがどう片づいたかを 1 か所で読みたい。閉じた spec の記録を掘り返さずに済むように。

#### Acceptance Criteria

1. The popup-menu-residue spec shall 件 1〜13 のそれぞれに「直した」「直さないと決めた」「別の spec へ渡した」のどれか 1 つを付け、この spec の文書に件の番号ごとに記録する。
2. Where 件を「直さないと決めた」とする, the popup-menu-residue spec shall その理由と、見直す条件（何が起きたら直すか）を同じ記録に書く。
3. Where 件を「別の spec へ渡した」とする, the popup-menu-residue spec shall 渡し先の spec の名前を記録に書き、渡し先が生きている spec であることを、渡した日の main で確かめる。
4. Where 件を「直した」とする, the popup-menu-residue spec shall 直したことを示すテストの名前、または直した記録の文を同じ記録に書く。
5. The popup-menu-residue spec shall 本 spec を閉じる時点で、仕分けの付いていない件を 1 件も残さない。

### Requirement 2: 済みの 4 件を確かめて記録する（件 1・2・7・10）

**Objective:** 開発者として、他の spec の作業で直った件が本当に直っているかを、今の main で確かめた証拠つきで知りたい。思い込みで閉じないように。

#### Acceptance Criteria

1. The popup-menu-residue spec shall 件 1 について、返事待ちの間に別の窓で預けた右ダブルクリックを今の要求の判定で送らず、記録 1 行（`trace`）を残して捨てることと、それを確かめる決定論テストが在ることを、今の main で確かめて記録する。
2. The popup-menu-residue spec shall 件 2 について、説明書のファイルが無いときの台本の `\![open,readme]` が OS へ渡さずに `warn` の記録 1 行で戻ることと、それを確かめる決定論テストが在ることを、今の main で確かめて記録する。
3. The popup-menu-residue spec shall 件 7 について、`spine.rs` の受け口の本数に触れるコメントが、確かめた日の実際の本数と合っていることを記録する。
4. The popup-menu-residue spec shall 件 10 について、説明書を開く OS の呼び出しが開く専用のスレッドで行われ、メニューと台本の両方の入口がそこを通ることと、UI スレッドに残る OS の呼び出しがファイルの有無の確認だけであることを記録する。
5. If 確かめた結果、件 1・2・7・10 のどれかが直っていない, then the popup-menu-residue spec shall その件を「済み」と記録せず、本 spec で直す件に移す。

### Requirement 3: 台本の入口から説明書を開くまでを通すテスト（件 3）

**Objective:** 開発者として、台本の `\![open,readme]` が説明書を開く所まで届くことを、部品ごとでなく一続きで確かめたい。途中の受け渡しを変えたときに壊れたことに気付けるように。

#### Acceptance Criteria

1. When 説明書のファイルが在るゴーストの台本に引数なしの `\![open,readme]` が 1 つ在り、その台本が再生される, the areka shall 説明書のファイルの絶対パスを行き先とする開く要求を、偽物の OS へちょうど 1 件渡す。
2. If 1 と同じ台本で説明書のファイルが無い, then the areka shall 偽物の OS へ何も渡さず、説明書を開く道の記録（`[readme]` の接頭辞を持つ行）として `warn` の記録をちょうど 1 行残す（起動の全体の他の警告は数えない）。
3. The popup-menu-residue spec shall 1・2 を、台本の文字列を入れてから開く要求が偽物の OS に届くまでの全段を本物の部品で通す決定論テストで確かめる（途中の段を差し替えない）。
4. The popup-menu-residue spec shall 件 3 の実機での観察を本 spec では行わず、その理由（OS へ渡す所はメニューの入口と同じで、実機ではメニューの入口が確かめ済み）を記録する。

### Requirement 4: メニューの組み立ての失敗を正しい文で記録する（件 4）

**Objective:** 障害のときに記録を読む人として、メニューが出なかったのが「組み立てられなかった」のか「表示できなかった」のかを、記録の文だけで区別したい。誤読して違う所を調べないように。

#### Acceptance Criteria

1. If メニューの組み立て（空のメニューを作る段、または項目を 1 つ足す段）が失敗する, then the areka shall 組み立てが失敗したと読める文で `error` の記録を 1 行残し、OS が返した失敗の符号をその記録に含める。
2. If 組み立てが済んだメニューの表示が失敗する, then the areka shall 今と同じ記録の種類の名前と文（`[menu] TrackPopupMenuEx failed`）で `error` の記録を 1 行残す。
3. The areka shall 1 と 2 の記録の文を互いに異ならせる。
4. If メニューの組み立てか表示が失敗する, then the areka shall 今と同じく何も選ばれなかったものとして扱い、ゴーストの動作を続ける。
5. The popup-menu-residue spec shall 1〜3 を、組み立ての失敗と表示の失敗をそれぞれ起こす決定論テストで確かめる。

### Requirement 5: 説明書の受け口の記録の型を揃える（件 5）

**Objective:** 記録を検索する人として、説明書を開く道の記録を 1 つの型で引きたい。同じ道に 3 通りの型が混ざって検索が漏れないように。

#### Acceptance Criteria

1. The areka shall 台本の説明書の受け口（`emo2_boot/readme_cue.rs`）が残す記録のすべてに、`[readme]` の接頭辞と、記録の種類の名前（`event=`）を持たせる。
2. The areka shall 1 の記録の水準と、記録を残す場面（何が起きたら何行残すか）を今と変えない。
3. The areka shall `[menu]`・`[readme]` の記録で今在る記録の種類の名前を変えない。
4. The popup-menu-residue spec shall kanade の照会の記録（`crates/areka-kanade/src/actor_resources.rs`）の型を変えず、「直さないと決めた」と記録する。理由: kanade の記録はクレート全体で接頭辞を持たず、記録の出どころの名前（`target`）と記録の種類の名前で揃っているので、角括弧の接頭辞を足すとクレートの中で不揃いになる。
5. The popup-menu-residue spec shall 1・2 を、受け口の記録の型と水準を見る決定論テストで確かめる。

### Requirement 6: SHIORI との往復の失敗の文を事実に合わせる（件 6）

**Objective:** 記録を読む人として、SHIORI との往復の失敗の記録が、その後に起きないこと（終了へ進むこと）を書かないでほしい。照会の失敗を終了の失敗と誤解しないように。

#### Acceptance Criteria

1. If SHIORI との往復（送る段・答えを受ける段）が失敗する, then the areka shall 失敗した段と理由を述べ、その後に終了へ進むとは述べない文で `error` の記録を 1 行残す。
2. The areka shall 1 の記録の水準と記録の種類の名前（`shiori_send_failed`・`shiori_reply_dropped`・`shiori_reply_timeout`）を今と変えない。
3. The areka shall 往復の失敗の後の振る舞いを、往復を呼ぶどの道（会話・終了・照会・翻訳）でも今と変えない（終了へ進むかどうかは、今と同じく呼んだ側が決める）。
4. The popup-menu-residue spec shall 1 を、照会の道で往復が失敗したときの記録の文を見る決定論テストで確かめる。
5. The popup-menu-residue spec shall 往復の関数の説明（コードの説明の文）にある「失敗は終了（Fault）へ倒す」の記述も、1 と同じく事実に合わせて直す（終了へ倒れるのは会話の道だけで、それを決めるのは呼んだ側）。

### Requirement 7: 相方の窓からの終了の指示を端から端まで通すテスト（件 8）

**Objective:** 開発者として、相方の窓の右クリックメニューで「終了」を選んだとき、そのスコープ番号がゴーストへの終了の知らせまで届くことを、一続きで確かめたい。受け渡しの途中を変えたときに壊れたことに気付けるように。

#### Acceptance Criteria

1. When 相方の窓（スコープ 1）のメニューから終了が指示される, the areka shall ゴーストへの終了の知らせ（`OnClose`）を、Reference0 を `user`・Reference1 と Reference2 をどちらも `1` として送る。
2. When 本体の窓（スコープ 0）のメニューから終了が指示される, the areka shall 同じ知らせを、Reference1 と Reference2 をどちらも `0` として送る。
3. The popup-menu-residue spec shall 1・2 を、終了の指示を kanade の入口に入れてから SHIORI への要求に現れるまでを本物の運行で通す決定論テストで確かめる。期待する References は `["user","1","1"]`・`["user","0","0"]` のように書き下し、知らせを組む関数から導かない（組む関数そのものの誤りにも気付けるように）。
4. The areka shall Reference1 を Reference2 と同じ値にすることを今と変えない（理由: 正典では Reference1 だけが `popupmenu.type` の影響を受けるが、areka は省略した形のメニューを作り分けないので、メニューの持ち主は常に右クリックした窓である）。

### Requirement 8: 語彙のみの 4 つの名前の引受先を確定する（件 9）

**Objective:** 網羅台帳を読む人として、`*.popupmenu.visible`（n≧2）と `*.popupmenu.type` の 4 行が、閉じた spec ではなく、まだ仕事をする spec か確定した裁定を指してほしい。誰も引き取らないまま忘れられないように。

#### Acceptance Criteria

1. The popup-menu-residue spec shall 網羅台帳の 4 行の持ち主の欄（`owner`）を空にし、引受先は備考に書く（先例: `doc/ukadoc-coverage/ledger/assets.toml` の「宛先の欄は空にした」の行。台帳の検査は `owner` に spec の一覧〔`doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]`〕に在る名前だけを許し、一覧へ spec を足すのは棚卸の仕事なので）。
2. The popup-menu-residue spec shall `char*.popupmenu.visible` の行の備考に、引受先を `extra-character-windows` と書き、同 spec が引き取らないと決めた場合はそのときに新しく起票することを書く。
3. The popup-menu-residue spec shall 網羅台帳の `sakura.popupmenu.type`・`kero.popupmenu.type`・`char*.popupmenu.type` の 3 行について、省略した形のメニューを作り分けないことを本 spec の裁定として備考に確定させ、理由（正典が省略した形の中身を定めていない）と見直す条件（`popup-menu-shell-look` がオーナードローのメニューを入れたとき〔正典はこの値を「オーナードローメニューの変更」と書く〕、または中身を決める根拠が得られたとき。どちらも新しく起票する）を書く。`char*.popupmenu.type` の行には、3 人目以降の窓の部分の引受先が `extra-character-windows` であることも書く。
4. The popup-menu-residue spec shall 4 行の対応の状態（語彙のみ）を変えず、spec の一覧（`roadmap-draft.md`）では `areka-P0-popup-menu-minimal` の持ち主の行数（`owner_count`）だけを台帳に合わせて直す（`[[spec]]` の行も、共有の数 `[briefs].count` も変えない）。
7. The popup-menu-residue spec shall 3 の見直す条件を、`popup-menu-shell-look` の brief にも 1 行写す。
5. The popup-menu-residue spec shall 問い合わせない名前の表（`menu/captions.rs` の `UNQUERIED_POPUPMENU_RESOURCES`）の説明を 1・2 と同じ引受先と理由に合わせ、表の中身（4 つの名前）は変えない。
6. The areka shall 右クリックメニューを、どの `*.popupmenu.type` の値でも今と同じ項目で出し、4 つの名前を SHIORI に問い合わせないことを今と変えない。

### Requirement 9: 件 11・12・13 を引受先へ渡す

**Objective:** 開発者として、本 spec では直さない 3 件が、閉じた記録の中に埋もれず、生きている spec の brief に見つけてほしい。本 spec を閉じた後も誰かが拾うように。

#### Acceptance Criteria

1. The popup-menu-residue spec shall 件 11（メニューを開いている間の相方のバルーンの黒い四角）と件 12（メニューが選ばれずに閉じる・見えない・選ばれずに閉じたときの記録が `debug` で見えない）を、`popup-menu-shell-look` の brief に、見つけた所・まだ分かっていないこと・見立て・再現の測り方とともに写す。
2. The popup-menu-residue spec shall 件 13（「．さくら」の古い資源の名前を問わない・項目が無い）を、本 spec の完了時の棚卸で `/kiro-discovery` により新しい spec 1 本として起票して渡す（⑴ 古い名前も問う直しと ⑵ 無い項目を足す仕事をまとめる。分けるのは起票の段で大きいと分かったとき）。その brief に、`popup-menu-shell-look` の件 15（おすすめの資源）との分け方が未決であることも一緒に書く。理由: 生きている spec に受け手が居ない（`popup-menu-shell-look` は項目の増減を範囲の外に置く）。
3. When 件 11・12・13 を写す, the popup-menu-residue spec shall 本 spec の brief のその件の節に、写した先の spec の名前と日付を書く。
4. The popup-menu-residue spec shall 件 11・12・13 のためにメニューを開く所・メニューの表示・SHIORI に問う名前の表（`crates/areka-kanade/src/schedule/resources.rs`）のコードを変えない。

### Requirement 10: 利用者に見える動きを変えない

**Objective:** ゴーストを使う人として、この片づけでメニューと説明書の動きが変わらないでほしい。記録とテストと文書だけが変わるように。

#### Acceptance Criteria

1. The areka shall 右クリックメニューの項目・項目名の問い合わせ・抑止の判定・選んだ後の動作を今と変えない。
2. The areka shall メニューの「説明書」と台本の `\![open,readme]` が開くもの・開かないときの振る舞いを今と変えない。
3. The popup-menu-residue spec shall 新しいテストを、確かめる実装と同じディレクトリの兄弟のファイルか、そのクレートの今在るテストの束に置く。
4. When 本 spec の変更が済む, the popup-menu-residue spec shall 触ったクレート（`areka`・`areka-kanade`）と、網羅台帳を検査するクレート（`ukadoc-survey`）のテストがすべて通ることを確かめる。
