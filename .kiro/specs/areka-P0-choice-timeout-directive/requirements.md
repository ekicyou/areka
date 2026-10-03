# Requirements Document

## Project Description (Input)

ゴーストが台本に `\![set,choicetimeout,時間]` を書いても、areka はそれを読まない。選択肢は必ず既定の 30 秒で時間切れになり、`OnChoiceTimeout` が飛ぶ。

利用者から見える害:

- 「時間切れなし」（`0` か `-1`）を指定したメニューが、30 秒で勝手に閉じる。里々・YAYA のゴーストのメニューや長考の選択肢でよく使う書き方である。
- 早押しのような短い指定（例 `500`）も効かず、30 秒待つ。

正典（ukadoc `\![set,choicetimeout,時間]`）: 「選択肢のタイムアウトの時間を指定する。単位はミリ秒。時間のカウントはトークの表示が全て終わってから開始される。そのスクリプト中のみ有効。選択肢より後ろに書いても有効。選択肢がタイムアウトした場合、OnChoiceTimeoutイベントが発生する。時間指定を省略：デフォルト値に戻す／時間指定が0か-1：タイムアウトしない」。

受ける側（選択待ちの期限の判定・時間切れの発火・`OnChoiceTimeout`）は完了 spec `areka-P0-choice-select-events` で動いている。欠けているのは「台本の指定を選択待ちの区切りの時間の値へ焼く」ことである（詳細は `brief.md`）。

## Introduction

本 spec は、台本に書かれた `\![set,choicetimeout,時間]` を areka が読み、その台本の選択肢が待つ時間に反映させる。利用者から見た結果は次の 3 つである。

- 時間を指定したメニューは、台詞の表示が全部終わってから指定のミリ秒で時間切れになり、`OnChoiceTimeout` が飛ぶ。
- `0` と `-1` を指定したメニューは、時間切れにならず、選ぶまで開いたままになる。
- 指定の無い台本、時間を省いた指定の台本は、今日と同じ既定（30 秒）で時間切れになる。

要件の段で確かめた事実（`main` `76e17654` 時点。「何の定義か」で指す）:

- **単位**: 台本はミリ秒。台本から選択待ちまでの途中の値はすべて**秒**で持っている——再生層の選択待ちの区切りの値（dola の `BarrierKind::WaitForChoice` の `timeout`）、それを kanade へ運ぶ通知（areka-kanade の `msg.rs` にある `ChoiceWaiting` の `timeout_directive_secs`）、kanade の期限の写し方（`schedule/choice.rs` の `choice_deadline`＝秒を 1000 倍して四捨五入）。したがって変換は「台本を読む側でミリ秒を秒へ」の 1 回だけでよく、下流の単位は変えない。
- **再生層の扱い（`brief.md` の想定に無かった点）**: 再生層の時刻表（dola の `schedule.rs` にある `TimedSchedule::tick`）は、区切りに時間の値が入っていると、それを**自分で解く期限**として扱う。値が `0` や負のとき、および区切りに着いた時刻がすでに「区切りの時刻＋値」を過ぎているときは、区切りを**飛ばす**。このため、台本の値をそのまま区切りへ入れるだけでは、`0`・`-1` を指定したメニューは選択待ちに入らず、短い正の値も時刻の進み方しだいで選択待ちに入らないことがある。本 spec の要件は「値に依らず選択待ちが必ず成り立つ」ことを求める（要件 5）。どこで満たすかは設計で決める。

## Boundary Context

- **In scope**:
  - `\![set,choicetimeout,時間]` を台本から読み、その台本の選択待ちの時間切れに反映させること（正の値・`0`・`-1` とその他の負の値・時間の省略・読めない値・選択肢より後ろ・複数回）。
  - ミリ秒から秒への変換（1 か所）。
  - 値に依らず選択待ちが成り立つこと。
  - 決定論テスト、網羅台帳の 1 行、`doc/COMPAT_ARCHITECTURE.md` §8 の記録。
- **Out of scope**:
  - `\![set,balloontimeout,時間]`（受ける側が未実装。`areka-P0-sakura-time-directives` に残す）。
  - `quicksection`・`balloonwait`・`embed`・`sound,wait`・`syncobject`・`move` ほかの時間の引数（`areka-P0-sakura-time-directives` に残す）。
  - 既定の 30 秒（`KanadeConfig::choice_timeout_default_ms`）そのものの見直し。
  - 時間切れの発火と `OnChoiceTimeout` の送り方（完了 `areka-P0-choice-select-events` のまま）。
  - 台本を読む側が中身を読んでよいコマンドの一覧の拡張（`\![set,choicetimeout,…]` は完了 `areka-P0-sakura-dialogue-tags` の要件 4.3 の一覧に最初から載っている。これ以外のコマンドは読まない）。
- **触らない場所（0 と明記）**:
  - `crates/areka-kanade/src/schedule/` の下は 0 ファイル（期限の判定は読むだけで変えない）。
  - `crates/areka-emo-text/` の下は 0 ファイル。
  - `crates/areka/src/emo2_boot/` の下は 0 ファイル。
  - `crates/areka/src/input_events/` の下は 0 ファイル。
  - `crates/areka-parsers/` の下は 0 ファイル（`\![set,choicetimeout,…]` は今日すでに名前 `set`・引数の並びとして転記されている）。
- **Adjacent expectations**:
  - kanade の期限の判定（`choice_deadline`）は、秒の値について「未指定＝既定」「0 以下＝無期限」「正＝その秒数」と写す。本 spec はこの約束に乗り、変えない。
  - 再生層の選択待ちの区切りの扱いを変えるかどうかは設計で決める。変える場合も、クリック待ち（`WaitForInput`）と時間待ち（`Timeout`）の今日の振る舞いは変えない。
  - 早送り（`areka-P0-talk-fast-forward`・未着手）は台詞の時計を先へ飛ばす。本 spec の要件 5 は、時計が飛んでも選択待ちが成り立つことを求めるので、早送りの着地後も時間の値は壊れない。
  - 同じ台本を読む処理を触る後続（`areka-P0-sakura-time-directives`・`areka-P0-anchor-tag-canon`）は本 spec の後に着手する。

## Requirements

### Requirement 1: 指定した時間で選択肢が時間切れになる

**Objective:** As a ゴーストの作者, I want 台本に書いたミリ秒で選択肢を時間切れにしたい, so that 早押しや短い返事待ちのような仕掛けが作れる

#### Acceptance Criteria

1. When 選択肢を含む台本に `\![set,choicetimeout,N]`（N は正の整数）が書かれている, the areka shall その台本の選択待ちを、台詞の表示が全部終わった時刻から N ミリ秒で時間切れにする。
2. When 要件 1.1 の選択待ちが時間切れになる, the areka shall 今日の既定の時間切れと同じ経路で `OnChoiceTimeout` を送る（Reference0 の中身・送り方は完了 `areka-P0-choice-select-events` のまま）。
3. While 要件 1.1 の期限がまだ来ていない, when 利用者が選択肢を選ぶ, the areka shall 今日と同じく選択を受け付け、時間切れは起こさない。
4. The areka shall 正の整数 N ミリ秒の指定を、端数の誤差なく「表示の終わり＋N ミリ秒」の期限にする（ミリ秒→秒→ミリ秒の行き来で値がずれない）。
5. If N が、扱える範囲を超える大きな正の整数である, then the areka shall 落ちずに、非常に長い時間切れとして扱う。

### Requirement 2: 時間切れなしと既定への戻し

**Objective:** As a ゴーストの作者, I want `0`・`-1` で時間切れを止め、時間を省いて既定へ戻したい, so that 長考のメニューが勝手に閉じない

#### Acceptance Criteria

1. When 選択肢を含む台本に `\![set,choicetimeout,0]` または `\![set,choicetimeout,-1]` が書かれている, the areka shall その台本の選択待ちを時間切れにせず、利用者が選ぶか台本が中断されるまで続ける。
2. When 選択肢を含む台本に `-1` 以外の負の整数（例 `-5`）が書かれている, the areka shall `0`・`-1` と同じく時間切れにしない（正典が黙っている点の areka の裁量。要件 10.3 で記録する）。
3. When 選択肢を含む台本に時間を省いた `\![set,choicetimeout]` または時間の欄が空の `\![set,choicetimeout,]` が書かれている, the areka shall その台本の選択待ちを既定（`KanadeConfig::choice_timeout_default_ms`＝30,000 ミリ秒）で時間切れにする。
4. When 選択肢を含む台本に `\![set,choicetimeout,…]` が 1 つも無い, the areka shall 今日と同じく既定で時間切れにする（選択待ちの区切りに入る値は今日と同じ「未指定」のまま）。

### Requirement 3: 書いた位置と回数

**Objective:** As a ゴーストの作者, I want 指定を台本のどこに書いても効かせたい, so that 正典どおり選択肢より後ろに書いた台本も期待どおりに動く

#### Acceptance Criteria

1. When `\![set,choicetimeout,…]` が選択肢（`\q`）より後ろに書かれている, the areka shall 選択肢より前に書いたときと同じ時間切れにする。
2. When 同じ台本に `\![set,choicetimeout,…]` が 2 回以上書かれている, the areka shall 台本の中で最後に書かれたものの値を使う（時間の省略も「既定に戻す」という 1 回の指定として数える）。
3. When `\![set,choicetimeout,…]` が終わりのタグ（`\e`・`\-`）より後ろにだけ書かれている, the areka shall それを数えない（正典の `\e`「この後に書かれたスクリプトは実行・表示されない」に従う）。
4. When 選択肢を含まない台本に `\![set,choicetimeout,…]` が書かれている, the areka shall 何も変えず、誤りとしても扱わない。

### Requirement 4: 効くのはその台本の中だけ

**Objective:** As a ゴーストの作者, I want 指定が次の台本へ持ち越されないようにしたい, so that 正典の「そのスクリプト中のみ有効」が守られる

#### Acceptance Criteria

1. When 時間を指定した台本の後に、指定の無い別の台本が選択肢を出す, the areka shall 後の台本の選択待ちを既定で時間切れにする。
2. When 選択肢を選んだ結果や `OnChoiceTimeout` の応答として新しい台本が返る, the areka shall その新しい台本を、前の台本の指定を引き継がずに扱う。

### Requirement 5: 選択待ちは値に依らず必ず成り立つ

**Objective:** As a 利用者, I want どんな値を指定したメニューでも選べる状態になってほしい, so that 指定のせいでメニューが選べないまま消えることがない

#### Acceptance Criteria

1. When 選択肢を含む台本が台詞の表示を終える, the areka shall 台本の時間の指定が正・`0`・負・省略・読めない値・無しのどれであっても、選択待ちに入る（選択肢が選べる状態になり、kanade へ選択待ちの知らせが届く）。
2. If 台詞の時計が、選択待ちの区切りの時刻と指定の時間を足した時刻よりも先へ一度に進んでから区切りに着く, then the areka shall 区切りを飛ばさずに選択待ちに入る。
3. While 選択待ちが続いている, the areka shall 時間切れの判定を kanade の既存の期限の判定だけに任せ、再生の側が台本の値を理由に選択待ちを自分で解くことをしない。
4. The areka shall 選択肢を含まない台本と、クリック待ち・時間待ちの区切りについて、今日と同じ振る舞いを保つ。

### Requirement 6: 読めない値

**Objective:** As a ゴーストの作者, I want 誤った値を書いたときに理由が記録に残ってほしい, so that 指定が効かない原因を突き止められる

#### Acceptance Criteria

1. If `\![set,choicetimeout,…]` の時間の欄が整数として読めない（例 `abc`・`1.5`・`500ms`）, then the areka shall その指定を既定として扱い、台本の値と既定に倒したことが分かる警告を記録に残す。
2. When 読めない値の指定の後に読める値の指定が書かれている, the areka shall 要件 3.2 に従い最後の指定の値を使う。
3. The areka shall 読めない値で止まったり、台本の他の部分の表示を変えたりしない。

### Requirement 7: 今日の転記と時刻の並びを変えない

**Objective:** As a areka の開発者, I want 指定を読むことが台本の他の振る舞いに波及しないでほしい, so that 既存の台本とテストがそのまま通る

#### Acceptance Criteria

1. The areka shall `\![set,choicetimeout,…]` を今日と同じく汎用のコマンドとして台本の流れへ転記し続ける（受け手は今日と同じく自分の担当でない名前として扱う）。
2. The areka shall 指定の有無で、台詞の文字・サーフェス・待ちなど他の出来事の内容と時刻の並びを変えない（変わるのは選択待ちの区切りに入る時間の値だけ）。
3. The areka shall `\![set,choicetimeout,…]` 以外のコマンドの中身を、本 spec のために新しく読まない。

### Requirement 8: 単位の変換は 1 か所

**Objective:** As a areka の開発者, I want ミリ秒と秒の取り違えが起きない形にしたい, so that 1000 倍ずれた時間切れが生まれない

#### Acceptance Criteria

1. The areka shall 台本のミリ秒を、下流が使う秒へ変える処理を、台本を読む側のただ 1 か所に置く。
2. The areka shall 下流（再生層の選択待ちの区切りの値・kanade への選択待ちの知らせ・kanade の期限の判定）の単位を秒のまま変えない。

### Requirement 9: 決定論テスト

**Objective:** As a areka の開発者, I want 振る舞いを決定論テストで固定したい, so that 後続の spec が同じ処理を触っても壊れたことに気付ける

#### Acceptance Criteria

1. The areka shall 台本の文字列を直に入れて、選択待ちの区切りに入る時間の値が期待どおりになることを確かめるテストを持つ。少なくとも次の場合を含む: 指定なし・正の値・`0`・`-1`・`-1` 以外の負の値・時間の省略・時間の欄が空・選択肢の後ろ・複数回（後の値が勝つ・省略が最後なら既定）・終わりのタグの後ろ・選択肢の無い台本・読めない値。
2. The areka shall 読めない値の場合に警告が記録されることと、正しい値の場合に警告が記録されないことを、記録を捕まえて確かめるテストを持つ。
3. The areka shall 要件 5.1・5.2 を、正・`0`・`-1`・省略の各値について、台詞の時計を区切りの手前から区切りと指定の時間を越えた先まで一度に進めても選択待ちに入ることで確かめるテストを持つ。
4. The areka shall 正の整数 N ミリ秒の指定が、台本から kanade の期限まで通して「表示の終わり＋N ミリ秒」になることを確かめるテストを持つ。
5. The areka shall これらのテストを、ネットにも実機にも実物の SHIORI にも依らず、時刻を外から与えて毎回同じ結果になる形で書く。
6. The areka shall 指定の無い台本についての既存のテスト（区切りの値が「未指定」であることを確かめているもの）を、書き換えずに通す。

### Requirement 10: 記録の更新

**Objective:** As a areka の開発者, I want 網羅台帳と正典が黙っている点の記録を実装に合わせたい, so that 後から読む人が今の対応状況と裁量を正しく知れる

#### Acceptance Criteria

1. When 本 spec の実装が着地する, the 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` shall `\![set,choicetimeout,時間]` の行を実装済み（`implemented`）にし、引受先を本 spec に替え、注記を今の振る舞い（壊れ方とログ）に書き直す。
2. The 網羅台帳 shall 台帳を検査する既存のテストを通し続ける。
3. When 本 spec の実装が着地する, the `doc/COMPAT_ARCHITECTURE.md` §8 shall 正典が黙っている点の areka の裁量を 1 行で記録する（同じ台本に複数回書いたら最後のものが勝つ・`-1` 以外の負の値も時間切れなし・時間の欄が空は省略と同じ・読めない値は既定として扱い警告を残す・終わりのタグより後ろは数えない）。
4. When 本 spec の実装が着地する, the `doc/COMPAT_ARCHITECTURE.md` §8 shall 「compile 側時間指令 allowlist」の行に、`set,choicetimeout` は本 spec で実際に読むようになったことを書き足す（他の時間の指令は `areka-P0-sakura-time-directives` の担当のまま）。
