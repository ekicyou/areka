# Requirements Document

## Project Description (Input)
**誰が困っているか**: 里々・YAYA などの標準の辞書を持つ既存ゴーストを areka で動かす利用者とゴースト作者。これらの辞書は SHIORI イベント `OnTranslate` で台詞を表示の直前に直している（敬称の重なり「さんさん」の除去・語尾の変化・自動の間の挿入など）。

**今どうなっているか**: ukadoc の正典では、SHIORI が返した台詞はバルーンに表示される直前に「翻訳」を通る（順序は `OnTranslate` → ゴースト側 MAKOTO → シェル側 MAKOTO）。`OnTranslate` の Reference0 には、ベースウェアが環境変数（`%username` 等）を展開した後の台詞が入る。areka は今日 `OnTranslate` を一度も送らず、翻訳を挟む継ぎ目も無い。そのため作者が直しているはずの台詞（「ユーザーさんさん」・語尾違い）が、正常な顔のまま間違って表示される。また areka の `%username` の展開は再生の直前に行われており、翻訳を置ける位置より後にある。

**何を変えるか**: SHIORI の返した台詞が再生に渡る前に、正典の順序（環境変数の展開 → `OnTranslate` → MAKOTO の鎖を差し込む口〔本 spec では素通し〕→ 再生）で翻訳を 1 か所に通す。`OnTranslate` を正典の Reference で送り、応答の台詞を最終の台詞にする。204 やエラー応答のときは元の台詞で進み、黙らない。「SHIORI との往復は一度に 1 つまで」の決まりを守る。MAKOTO/2.0 DLL のホスティングは後半の spec `areka-P0-makoto-dll-host` が受け持つ（詳細は brief.md）。

## Introduction
SHIORI が返した台詞を、バルーンに表示する前に正典（ukadoc「トランスレータ」「OnTranslate」）の順序で翻訳する。翻訳は 1 か所で行い、SHIORI の返した台詞が再生されるすべての経路がそこを通る。本 spec が受け持つのは、環境変数の展開を翻訳の前へ移すこと・SHIORI イベント `OnTranslate` を送って応答を最終の台詞にすること・後半の spec `areka-P0-makoto-dll-host` が MAKOTO の鎖を差し込む口を 1 つ用意すること（本 spec ではその口は台詞をそのまま返す）。

正典の根拠: ukadoc [トランスレータ](https://ssp.shillest.net/ukadoc/manual/manual_translator.html)（「トランスレートのタイミングはバルーンに表示される直前」・SHIORI→MAKOTO の順）・[OnTranslate](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnTranslate:1)（Reference0＝環境変数の展開などの後の元のスクリプト・Reference1＝出所で「該当がない場合欠番」・Reference2＝このスクリプトが発生したイベント ID・Reference3＝そのイベントの Reference をバイト値 1 で区切ったもの・「OnTranslate 自身では再度発生しない」）。正典は `OnTranslate` を「元の SHIORI Event に対して返却されたスクリプトが…再び SHIORI に送られる」と書くので、送り先は台詞を返した SHIORI である。

### 用語
- **台詞**: SHIORI が GET の応答として返したさくらスクリプト。
- **元のイベント**: その台詞を返させた SHIORI イベント（ID と Reference の並び）。
- **翻訳**: 台詞を受け取って、表示に使う台詞を返す一連の処理（展開 → `OnTranslate` → MAKOTO の口）。
- **展開**: `%username` などの環境変数を値の文字列へ置き換えること。
- **areka が自分で作る台詞**: SHIORI が返したものでない台詞（例: 初回起動で挨拶の台詞が無いときに areka が足す、文字を持たない起動の記録だけの台詞）。

### 台詞が再生される経路（2026-10-03 に main `d4f9e93d` で実測・設計の着手時に再確認する）
SHIORI の台詞が再生へ渡る経路は次の 5 種類で、今日はすべて同じ 1 つの出口（再生の開始の要求）へ集まる。SSTP・コミュニケート・プラグイン由来の台詞の経路は 0 本（未実装）。
1. 起動の挨拶: `OnFirstBoot`／`OnBoot`／`OnGhostChanged`（切替で来た新しいゴースト）／ネットワーク更新で読み直した後の起動のイベント
2. 終了の別れ: `OnClose`
3. ゴースト切替の送り出し: `OnGhostChanging`、それが 204 のときに続く切替の `OnClose`、切替を取りやめた後に流す `OnGhostChanging` の台詞
4. 定常: 毎秒のポンプ（`OnSecondChange` 等）・マウスのイベント・外から頼まれたイベント（汎用の通知の入口）の応答
5. 選択肢: 選択肢を選んだ後の連鎖の応答・`OnChoiceTimeout` の応答

## Boundary Context
- **In scope**: 上の経路すべての台詞の翻訳・展開を翻訳の前へ移すこと・`OnTranslate` の送出と応答の扱い・MAKOTO の鎖を差し込む口 1 つ（本 spec では素通し）・翻訳の待ちを「SHIORI との往復は一度に 1 つまで」の決まりの中で表すこと・控える台詞を翻訳後にすること・決定論テスト・`doc/COMPAT_ARCHITECTURE.md` §8（正典が沈黙する箇所の areka の裁量）への登記。
- **Out of scope**: MAKOTO/2.0 DLL の読み込み・通信・文字コード・`\![load|unload|reload,makoto]`（→ `areka-P0-makoto-dll-host`）。SSTP・コミュニケート・プラグインなど新しい台詞の出所を作ること（よって Reference1 の出所の語彙・SSTP の `notranslate`・descript の `sstp.alwaystranslate` も範囲外）。`%(...)` の展開。`username` 以外の環境変数名を新たに展開できるようにすること。`OnTranslate` 以外の新しいイベント。`\![reload,shiori]`。再生側（台本の解釈・文字の表示）の振る舞いの変更。
- **Adjacent expectations**:
  - `areka-P0-makoto-dll-host` は、本 spec が用意する口へ「台詞と元のイベントの出所を受け取り、台詞を返す」形で MAKOTO の鎖を差し込む。本 spec はその口の向こうで何が起きるか（DLL・プロセス・文字コード）を一切持たない。
  - SHIORI の失敗の分類（エラー応答 400・500 は 204 と同じ扱いで会話を続ける・輸送路の失敗はそのゴーストの SHIORI の故障）は、完了 spec `areka-P0-shiori-fault-notice` の決まりをそのまま使い、`OnTranslate` のために例外を作らない。
  - `areka-P0-property-query-channels`・`areka-P0-balloon-lifecycle-events`・`areka-P0-network-update-canon-order` は同じ運行の判断のファイルを触るので、本 spec と同時には進めない（本 spec が先）。`areka-P0-ukadoc-survey-shiori` は完了済みのため、`OnTranslate` の ukadoc の URL の注記と台帳の更新は本 spec が行う（要件 3.7）。
- **開発上の制約**: 1 ファイル 1,000 行未満を保つ（触るファイルが上限に近い場合は、振る舞いを変えずに先に分ける）。ファイルの長さの検査の例外表には触れない。areka 本体のクレートの `ghost_session.rs`・`main.rs`・`emo2_boot/` の本番のコードには触れない（同じウェーブの spec が触る）。例外として、`emo2_boot/` と `crates/areka/src/` のテスト用の偽の SHIORI（`emo2_boot/spine.rs` の `ScriptedShioriBackend`）に「`OnTranslate` は既定で 204」を足すことと、テストの期待（記録した呼び出しの列）に `OnTranslate` を書き足すことは行ってよい（2026-10-03 開発者裁定）。既定の 204 は呼び出しの記録に残し、記録から隠さない。同じテストのファイルを同じウェーブの spec（`shell-balloon` 等）も触った場合は、後から着地する側がテストの行を rebase で直す。`crates/areka-ghost/tests/ghost/spine_e2e_test.rs` の同名の偽の SHIORI も同じ扱いとする。

## Requirements

### Requirement 1: 翻訳を 1 か所で通す
**Objective:** As a 既存ゴーストの利用者, I want SHIORI が返したどの台詞も表示の前に必ず翻訳を通ってほしい, so that ゴースト作者が `OnTranslate` で直した台詞がどの場面でも表示される

#### Acceptance Criteria
1. When SHIORI が GET の応答として台詞を返したとき, the areka shall その台詞を再生へ渡す前に翻訳を 1 回だけ通し、翻訳の結果の台詞を再生へ渡す。
2. The areka shall 「台詞が再生される経路」に挙げた 5 種類の経路のすべてで、台詞を同じ 1 つの翻訳に通す（経路ごとに別の翻訳を持たない）。
3. When areka が自分で作る台詞を再生するとき, the areka shall その台詞を翻訳せず、展開も `OnTranslate` の送出も行わない。
4. Where 起動の挨拶の台詞の後ろに areka が初回起動の記録を足すとき, the areka shall SHIORI の返した部分だけを翻訳し、areka が足す部分は翻訳に渡さず翻訳の結果の後ろにそのまま付ける。
5. If 翻訳を通らずに再生へ渡る経路が 1 本でも生じたとき, the areka shall それを決定論テストで赤として検出する（各経路について、翻訳を外すと赤になるテストを置く）。

### Requirement 2: 正典の順序（展開してから翻訳）
**Objective:** As a ゴースト作者, I want `OnTranslate` に届く台詞が環境変数を展開した後のものであってほしい, so that 「%usernameさん」ではなく「太郎さん」を見て敬称の重なりなどを直せる

#### Acceptance Criteria
1. The areka shall 翻訳の中の順序を「展開 → `OnTranslate` → MAKOTO の鎖の口 → 再生」とする。
2. When 台詞に `%username` が含まれるとき, the areka shall `OnTranslate` の Reference0 に、利用者名を展開した後の台詞を入れる（例: 利用者名が「太郎」のとき `%usernameさん` は `太郎さん` として届く）。ただし、展開すると台本の読みが変わる並び（例: `\w%username` で利用者名が数字で始まる——直前のタグが値の先頭を引数として飲み込む）の台詞は、受け入れ基準 5（表示が今日と 1 文字も違わない）を優先して翻訳の前には展開せず、警告を 1 件記録して `%username` の綴りのまま届ける（再生時の展開は今日どおり働く）。この扱いは互換の逸脱として登記する（設計討議 2026-10-03）。
3. The areka shall 展開に、今日の再生時の展開と同じ規則・同じ値の源（その台詞の再生に使われるはずだった値の写し。`username` のほか `selfname`・`selfname2`・`keroname` など写しに値がある名前はその値）・同じ既定値（`username` に値が無いときの「ユーザーさん」）を使う。
4. The areka shall 値が無く既定値も無い環境変数の名前を `%名前` の綴りのまま残す（今日の再生時の展開と同じ）。
5. When `OnTranslate` の応答が 204 で、MAKOTO の鎖が素通しのとき, the areka shall 利用者に表示される台詞を今日の表示と 1 文字も違わないものにする（`%` を含む台詞・タグの引数の中の `%`・エスケープされた `%` を含む台詞でも同じ）。
6. If 翻訳の結果の台詞に環境変数の綴り（`%username` 等）が残っているとき, the areka shall 再生時に今日と同じ規則でそれを展開する（再生側の展開は残し、翻訳の前に展開済みの部分は再生側で変わらない）。

### Requirement 3: `OnTranslate` の送出
**Objective:** As a 里々・YAYA の辞書を持つゴースト, I want 正典どおりの Reference で `OnTranslate` を受け取りたい, so that 標準の辞書の `OnTranslate` がそのまま働く

#### Acceptance Criteria
1. When 翻訳を通す台詞があるとき, the areka shall その台詞を返した SHIORI へ `OnTranslate` を GET で 1 回送る。ただし SHIORI が 200 で中身が空（0 文字）の台詞を返したときは、正典の「スクリプトが返却された場合」に当たらないものとして `OnTranslate` を送らず、今日と同じに進む。
2. The areka shall `OnTranslate` の Reference0 を展開済みの台詞、Reference2 を元のイベントの ID、Reference3 を元のイベントの Reference の並びをバイト値 1 で区切って連ねたもの（元のイベントの Reference が 0 個のときは空文字列）とする。
3. The areka shall Reference1 を欠番とする（areka にはコミュニケート・SSTP・プラグイン・トランスレートしない指定のどの出所も無いため。正典「該当がない場合欠番となる」）。
4. The areka shall `OnTranslate` を、SHIORI へ送ってよいイベントの表に加える。
5. While ゴーストを切り替えている間, the areka shall 送り出しの台詞（`OnGhostChanging` とそれに続く切替の `OnClose` の台詞）の `OnTranslate` を切替の前のゴーストの SHIORI へ送り、切替の後のゴーストの起動の挨拶（`OnGhostChanged` 等）の `OnTranslate` を切替の後のゴーストの SHIORI へ送る。
6. The areka shall `OnTranslate` に、areka が他の GET に付けるのと同じ要求の見出し（`Status`・`SecurityLevel` 等）を付ける。
7. The areka shall 送ってよいイベントの表の `OnTranslate` の行に、他の行と同じ形で ukadoc の URL の注記を付け、網羅の台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）の `OnTranslate` の項目を実装済み（担当は本 spec）に改める（完了 spec `areka-P0-file-drop`・`areka-P0-network-update` と同じ手順。調査 spec `areka-P0-ukadoc-survey-shiori` は完了済みで、後から注記を足す担い手はいない）。

### Requirement 4: `OnTranslate` の応答の扱い
**Objective:** As a 利用者, I want 翻訳がうまくいかないときでもゴーストが黙らずに元の台詞を話してほしい, so that 翻訳の不具合で会話が消えない

#### Acceptance Criteria
1. When `OnTranslate` の応答が 200 で台詞を持つとき, the areka shall その台詞を MAKOTO の鎖の口へ渡し、その結果を最終の台詞とする。
2. When `OnTranslate` の応答が 200 で空の台詞を持つとき, the areka shall 空の台詞を最終の台詞として採用し、元の台詞へ戻さない。
3. When `OnTranslate` の応答が 204 のとき, the areka shall 展開済みの元の台詞を MAKOTO の鎖の口へ渡す。
4. If `OnTranslate` に SHIORI がエラー応答（400・500 等）を返したとき, the areka shall 204 と同じく展開済みの元の台詞で進み、元のイベントの ID を含む警告を 1 件記録する。
5. If `OnTranslate` の往復で輸送路の失敗（つながらない・期限内に応答が無い・通信が切れた）が起きたとき, the areka shall 他の SHIORI イベントの輸送路の失敗と同じ扱い（そのゴーストの SHIORI の故障）とし、`OnTranslate` のための例外を作らない。
6. The areka shall `OnTranslate` の応答の台詞（翻訳の結果）に対して `OnTranslate` を再び送らない。
7. When `OnTranslate` の応答を受け取ったとき, the areka shall 元のイベントの ID と応答の種類（置換・空・204・エラー応答）を 1 件記録する。

### Requirement 5: SHIORI との往復は一度に 1 つまで
**Objective:** As a areka の保守者, I want 翻訳の往復が加わっても「SHIORI との往復は一度に 1 つまで」の決まりが例外なく守られてほしい, so that SHIORI との対話の順序が崩れない

#### Acceptance Criteria
1. The areka shall `OnTranslate` を、元のイベントの応答を受け取り終えた後にだけ送り、元のイベントの往復と同時に進めない。
2. The areka shall 翻訳の待ちを運行表（SHIORI の往復の順序を決める純粋な状態機械）の状態として表し、「同時に進行中の SHIORI 往復は 1 つまで」の決まりに新しい例外を作らない（運行表の外の殻の中で往復を足す形は取らない。状態を相として持つか、相の外の帳簿として持つかは設計で決める）。
3. While 翻訳の応答を待っている間, the areka shall 他の SHIORI イベント（毎秒のポンプ・マウス・外から頼まれたイベント・終了や切替の握手のイベント）を送らない。
4. When 翻訳の応答を待っている間にマウス・毎秒の時刻・終了の要求・切替の要求・外からのイベントの依頼が届いたとき, the areka shall それらを捨てず、翻訳の結果の台詞の再生が始まった後に、今日の「その台詞を再生している間」の規則で扱う。
5. When 翻訳を終えて台詞の再生を始めるとき, the areka shall 再生の開始の時点の運行の状態（起動の途中・定常・選択肢の連鎖・終了の握手・切替の握手のいずれか）を、翻訳が無かった今日と同じにする。
6. The areka shall 台詞が返ったイベント 1 つにつき SHIORI との往復を 1 回だけ増やし、台詞が返らなかった（204 等）イベントと中身が空の 200 のイベントでは 0 回とする。

### Requirement 6: 控える台詞は翻訳の後
**Objective:** As a ゴースト作者, I want 後のイベントの Reference に入る「前の台詞」が実際に表示された台詞であってほしい, so that 表示と食い違う台詞をゴーストが受け取らない

#### Acceptance Criteria
1. The areka shall 再生中の台詞として控える台詞（`OnChoiceTimeout` の Reference0 の源）を、翻訳の後の台詞とする。
2. The areka shall 切替の前のゴーストの「切り替え時のスクリプト」として控え、切替の後のゴーストの `OnGhostChanged` の Reference1 に入れる台詞を、翻訳の後の台詞とする。
3. The areka shall 次の 6 点を正典が沈黙する箇所の areka の裁量として `doc/COMPAT_ARCHITECTURE.md` §8 に登記する: ⑴ 控える台詞は翻訳の後 ⑵ `OnGhostChanged` の Reference1 は翻訳の後 ⑶ Reference1 は常に欠番 ⑷ エラー応答のときは元の台詞で進む ⑸ 翻訳の結果に残った環境変数は再生時に展開する ⑹ 中身が空の 200 には `OnTranslate` を送らない。
4. The areka shall §8 の既存の行「`OnGhostChanged` の Ref1 に何を載せるか」（今は「`OnGhostChanging` が返した台本をそのまま載せる」）を、⑵ と食い違わない書き方に改める（新しい行を足すだけで既存の行を残さない）。

### Requirement 7: MAKOTO の鎖を差し込む口
**Objective:** As a 後半の spec `areka-P0-makoto-dll-host` の実装者, I want MAKOTO の鎖を差し込む口が 1 つだけ決まっていてほしい, so that 翻訳の経路を作り直さずに MAKOTO の DLL を足せる

#### Acceptance Criteria
1. The areka shall `OnTranslate` の後・再生の前に、台詞と元のイベントの出所（ID）を受け取って台詞を返す口を 1 つだけ持つ。
2. While MAKOTO が差し込まれていない間, the areka shall その口で台詞を 1 文字も変えずに返す。
3. The areka shall その口を `OnTranslate` の応答の種類（200・204・エラー応答）に依らず通す。
4. The areka shall その口の手前に DLL・プロセス・文字コードの知識を持ち込まない（本 spec が足す MAKOTO の実装は素通しの 1 つだけで、DLL を扱う実装は 0 個）。

### Requirement 8: 挙動不変と検証
**Objective:** As a 開発者, I want 翻訳の追加が `OnTranslate` を持たないゴーストの見た目を変えないことと、各規則が決定論テストで固定されていることを確かめたい, so that emo2 などの既存の動きを壊さずに取り込める

#### Acceptance Criteria
1. The areka shall 次を DLL を使わない決定論テスト（偽の SHIORI と純粋な関数）で確かめる: ⑴ 5 種類の各経路が翻訳を通ること ⑵ 展開の後に `OnTranslate` が届くこと（偽の SHIORI が `太郎さん` を受け取る） ⑶ 応答の行列（200 で置換・200 で空・204・エラー応答・輸送路の失敗） ⑷ `OnTranslate` が再び送られないこと ⑸ Reference0〜3 の中身と Reference1 の欠番 ⑹ 記録の語彙 ⑺ 翻訳の待ちの間に届いた入力の扱い ⑻ 控える台詞が翻訳の後であること ⑼ areka が自分で作る台詞が翻訳されないこと。
2. When `OnTranslate` に 204 を返す SHIORI（emo2 の pasta）で動かすとき, the areka shall 表示される台詞を本 spec の前と 1 文字も違わないものにする（emo2 の e2e と実機の台詞の記録で確かめる）。
3. The areka shall ワークスペース全体のテスト（`tools/test-all.ps1` の手順）を緑で通す。
4. The areka shall 本 spec で増える状態と失敗の経路のすべてに記録を残し、記録の無いまま進む経路を 0 本とする。
