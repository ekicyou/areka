# Design Document: dola-interrupt-ticks

## Overview

**Purpose**: dola の時刻の土台を作り直す。時刻・長さ・符号つきの差を 100ns の整数の型にし、「今」を読む時計を `QueryUnbiasedInterruptTimePrecise` の 1 本にし、台本（`CueSheet`）と storyboard の層と使い手の全部を同じ物差しへ移す。そのうえで、起点の置き直し・置き直しの知らせ・合図の絶対の発火時刻・絶対の時刻の刻みを入れる。

**Users**: dola の使い手（areka の各エンジンと wintf）と、この土台の上に高速再生基盤・並走・値を時間で変える口を建てる開発者。

**Impact**: dola の公開の口から `f64` の秒が消える。本番の時計は 1 本になり、`GetTickCount64` と QPC の秒は読まれなくなる。受け手（文字の層・seriko・バルーンの寿命の時計）は起点を見積もらず、起点を持たない。受け取った物を、合図が運ぶ絶対の発火時刻で持つ。台本の中身は、受け手に届いた時でなく、合図の発火時刻に効く。

大前提（要件 18）: 同じ起点を持つ台本は、どの受け手でも同じタイミングで再生される。dola は「受け取った現在の時刻」から再生の位置を求めるだけで、その時刻がどう刻まれたかを問わず、受け取った時刻を覚えもしない。刻みが運ぶ時刻は、刻み役が起きた時に読んだ時計の値のままである（格子の値へ寄せない・先読みして配らない）。

設計の案は、設計として必然かどうかで選ぶ。この文書に出るタスクの数は、段取りを組むための見込みであって、案を選ぶ根拠ではない。

### Goals

- 時間の次元を持つ値を、絶対も相対も、cue の層・storyboard の層・storyboard の文書の定義の型の全部で、100ns の整数の型で持つ。
- 本番の「今」を 1 本の時計から読む。
- 倍率が 1 でない時の出来事の判定と、逆向きに求めた時刻が食い違わない事と、どの出来事も本当の時刻より前に「届いた」と判定しない事を、構造で保証する。
- 起点の置き直し（前へも後ろへも）・知らせ・絶対の発火時刻の口を、後で作り直さずに済む形で入れる。
- 台本の中身が効く時刻を、合図の発火時刻にする（届いた時刻にしない）。フレームが見せる物を、届いた合図の発火時刻と、置き直しの知らせと、フレームの時刻だけで決める。受け手は、起点も台本の時刻も持たない。
- 遅れて届いた合図の追い付き方を、受け手ごとに決める。
- 並行のサブエージェントで進められる段取りを、設計が持つ。

### Non-Goals

- `std::time::Instant` で測る締め切りと入力の時刻、別の時計を止める番人（`instant-to-interrupt-ticks`）。
- 高速再生基盤そのもの・`\x` の待ち・並走の流れ・値を時間で変える口。本番で起点を置き直す使い手は、この spec には居ない。
- 表示の周期と刻みの周期（50・1000・16 ms）。区切りの種類を増やす事。
- 付け替えの前の形を読む互換の口・移行の案内・版の行の変更。

## Boundary Commitments

### This Spec Owns

- 時刻の型 `TimePoint`・長さの型 `TimeSpan`・符号つきの差 `TimeDelta`・倍率 `TimeScale`・「今」を読む関数・作者が書く単位からの変え方と丸めの決まり。
- dola の公開の口の全部（cue の層・storyboard の層・文書の定義の型）の時刻と長さ、直列化の形。
- 時刻の表の起点の置き直し・受け口への起点の知らせ（`CueSink::origin_set`。再生の始まりの最初の起点と、置き直し）・合図の絶対の発火時刻（`TalkCue::fire_time`）。
- 使い手の追従: `wintf`・`areka-talk`・`areka-sakura`・`areka-kanade`・`areka-seriko`・`areka-emo-text`・`areka-ghost`・`areka-runtime`・`areka-input`・`areka-session`・本体 `areka`・先進坑。
- 時計の結線: ghost の刻み役（seriko を進める刻み役を含む）・`FrameTime`・`TalkClock`。
- 1 字ごとの間隔の式（文字の層と seriko が呼ぶ 1 本の関数）。
- 台本の中身が効く時刻の決まり（発火時刻に効く）と、受け手が受け取った物を絶対の時刻で持つ決まり（起点を持たない）と、フレームで評価する受け手が、届いた物を時刻が来るまで持つ列（`areka_sakura::held`。文字の層とバルーンの寿命の時計が使う）。
- 受け手ごとの追い付き方の決まり（文字の層・seriko・バルーンの寿命の時計・姿を変える指令の受け口・storyboard の層の値と引き金）。
- storyboard の層の、止めた再生を戻すときの時間の数え方。
- 全体テストの、Windows でない target の検査の段。dola の README・`tech.md`・`structure.md`・替えた所のコードの説明。

### Out of Boundary

- `Instant` の締め切り（`tick_wake`・SHIORI の応答の予算ほか）と、入力の時刻。
- 暦の時刻（`GetLocalTime`・ログの日時）。
- 速さの切り替え・待ちの飛ばし・クリック待ちの通り方・並走の流れの進め方・値を時間で変える口の中身。
- kanade の時間切れの権限の持ち主と、指令の読み分け（無い・0 と負・正）。
- roadmap とほかの spec の brief の書き直し（棚卸の持ち物）。
- crates.io へ出す crate の一覧と順（増えない。`tools/crates-io.ps1` と `doc/crates-io-publish.md` は触らない）。

### Allowed Dependencies

- dola は、ワークスペースのほかの crate に依存しない（今のまま）。時刻の土台（`dola::time`）は dola の中のモジュールで、std と、Windows の時だけ `windows`（機能 `Win32_System_WindowsProgramming`）を使う。
- 依存の向きは「dola → wintf」「dola → `areka-talk` → `areka-sakura`・`areka-kanade` → …」。上の crate が下を引く。逆は作らない。dola が何も引かないので、だれが dola を引いても輪はできない。
- `areka-kanade`・`areka-talk`・`areka-ghost` の本番は、dola を引く（`dola::time` の型と時計のため。新しく足す依存の辺は、この 3 本）。`areka-talk` の説明の「依存 0」は改める。
- 外の crate は足さない。dola の `cfg(windows)` の `windows` の依存に、機能 `Win32_System_WindowsProgramming` を足し、`Win32_System_Performance` を外す（dola が QPC を呼ぶ所は `runtime/clock.rs` だけで、それを消すと呼ぶ所が無くなる）。根の `Cargo.toml` は触らない。
- 再生と評価の口は、時刻を外から受ける。dola の中から「今」を読まず、受けた時刻を覚えない。

### Revalidation Triggers

- `TimePoint`・`TimeSpan`・`TimeDelta` の表せる範囲・丸めの決まり・範囲の端の扱いを変える時。
- `TalkCue` の欄・`CueSink` の口・`OriginNotice` の中身を変える時。
- `CueSheet` の直列化の形を変える時。
- 時刻の表の「取り出す時に発火時刻を決める」「知らせと合図を起きた順に配る」「最初の起点を、どの合図よりも前に知らせる」の決まりを変える時。
- 受け手が、受け取った物を絶対の時刻で持つ（起点を持たない）決まりを変える時。
- 知らせの受け方の決まり（効く時刻より後の時刻だけを、動いた長さだけ動かす・効く時刻以前の物は書き換えない・最初の起点の知らせでは何もしない）を変える時（文字の層・バルーンの寿命の時計・seriko が確かめ直す）。
- 届いた物を持つ列の決まり（届いた順・頭の物が待つ間は後ろも待つ・時刻を持たない物は前の物に続く）を変える時（`areka_sakura::held` を使う 2 つの受け手が確かめ直す）。
- 1 字ごとの間隔の式（`areka_sakura::reveal`）を変える時（文字の層と seriko の両方が確かめ直す）。
- 公開の道 `dola::time` を変える時（中身を dola の下の crate へ移すだけなら、道は変わらないので当たらない）。

## Architecture

### Existing Architecture Analysis

- dola は時刻を外から受ける受け身の作りで、自分の中から「今」を読まない。この作りは保つ。
- cue の層と storyboard の層は、コードで互いを参照しない。共通の物は時刻の型だけになる。
- 時刻の表（`TimedSchedule`）は「進め終えてから配る」2 段の口（`tick`＋`ready`）で、取り出しごとに何かを挟む場所が無い。起点は作る時に 1 回入るだけ。
- 受け口（`CueSink`）は起点を受け取らない。だから文字の層の時計（`TalkClock`）と seriko（`TalkEpoch`）は「今 − 合図の時刻」の最大で起点を見積もる。届く時刻の揺れが台本の時刻に出て、起点は前へ動かせない。
- 台本の中身のうち、字のほか（消去・改行・書体ほか・面の切り替え・着せ替え）は、受け手に届いた時に効く。文字の層は UI のスレッドで届いた合図をその場で当て、seriko は合図を処理する時に時計を読む。フレームごとに「時刻の来た合図を当てる」所は無い。
- 本番の起点はいつも 0。dispatcher が台詞の最初の刻みからの相対の秒を送るからである。
- 時計が 3 つある（QPC の秒・`GetTickCount64` のミリ秒・`Instant`）。前の 2 つをこの spec で 1 本にする。
- `areka-sakura` の `contract.rs` が dola の cue の型を再公開している。`areka-sakura` より上は、`areka-sakura` が替わる時に一緒に替わる。
- dola は、ワークスペースのほかの crate を 1 つも引かない。`areka-kanade`・`areka-talk`・`areka-ghost` の本番は、今は dola を引いていない。

### Architecture Pattern & Boundary Map

```mermaid
graph TB
    Dola[dola]
    Wintf[wintf]
    Talk[areka-talk]
    Sakura[areka-sakura]
    Kanade[areka-kanade]
    Seriko[areka-seriko]
    EmoText[areka-emo-text]
    Ghost[areka-ghost]
    Runtime[areka-runtime]
    Input[areka-input]
    Session[areka-session]
    App[areka]
    Dola --> Wintf
    Dola --> Talk
    Dola --> Sakura
    Dola --> Kanade
    Dola --> Ghost
    Talk --> Sakura
    Talk --> Kanade
    Sakura --> Seriko
    Sakura --> EmoText
    Wintf --> EmoText
    Sakura --> Ghost
    Kanade --> Ghost
    Seriko --> Runtime
    EmoText --> Runtime
    Ghost --> Runtime
    Runtime --> Input
    Input --> Session
    Session --> App
```

**Architecture Integration**:

- 選んだ形: 時刻の土台を dola の中のモジュール `dola::time` に置く。cue の層と storyboard の層は、その場で型を替える。起点の置き直しと知らせは、時刻の表と再生機の口として足す。受け手の側の決まり（届いた物を時刻が来るまで持つ列・1 字ごとの間隔）は、受け手の全部が引いている `areka-sakura` に 1 本ずつ置く。
- 責任の分け方: 丸めと範囲の決まりと直列化の形は `dola::time` の 1 か所。再生機は、時刻と起点を配るまでを持つ。受け取った中身を、どれだけ厳しく時刻に合わせるかは、受け手が中身の種類ごとに決める（「受け手ごとの追い付き方」の節）。作者が書く単位を読む縁は、それぞれの使い手（`areka-sakura` の `duration.rs`・seriko の表を組む所・`areka-runtime` のバルーンの待ち）。
- 保つ作り: 時刻を外から受ける受け身の再生機・全部の受け口へ選ばずに配る形・テストが時刻を外から渡す決定論。
- steering: 時刻は正確に扱う（待ちを丸めない・遅れたら過ぎた分だけ進める）・1 フレーム遅らせない・ベースウェアに判断やモードを足さない・外の crate を足さない・ログ無しの失敗の道を作らない。

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| 時計 | `windows` 0.62・機能 `Win32_System_WindowsProgramming` | `QueryUnbiasedInterruptTimePrecise() -> u64` を呼ぶ | dola の `cfg(windows)` の依存の機能を、`Win32_System_Performance` からこれへ替える |
| 直列化 | `serde` 1（dola の今の依存） | cue は 100ns の整数・storyboard の文書は秒 | アダプタは `dola::time` の中に置く |
| 検査 | `rustup` の target `x86_64-unknown-linux-gnu` | Windows でない target の `cargo check -p dola` | 全体テストの 1 段。外の crate は足さない |

### 設計の決め

| # | 決め | 中身 | 根拠 |
|---|---|---|---|
| D1 | 置き場 | 時刻の土台（型・丸め・倍率の写し・文字の形・ただ 1 つの時計・直列化のアダプタ・テストの組み立て）を、dola の中のモジュール `dola::time`（`crates/dola/src/time/` の下の何本かのファイル）に置く。新しい crate は作らない。「今」を読む関数の定義は 1 つ | dola は時刻の土台そのもので、ワークスペースの一番下の crate である。公開の道は、中身を dola の中に置いても、下の crate に置いて再公開しても、同じ `dola::time` になる。後で中身を下の crate へ移しても、使い手のコードは 1 字も変わらない。だから、置き場は、この spec が固める仕様に入らない。dola は何も引かないので、`areka-kanade`・`areka-talk`・`areka-ghost` の本番が dola を引いても、輪はできない |
| D2 | 型の中身 | `TimePoint`・`TimeDelta` は `i64`、`TimeSpan` は 0 以上の `i64`。どれも 100ns が 1 | 起点は、台本の再生が始まった時刻より前へ動く（要件 11 の 3）。0 から数える時計では、起点が 0 より前になる。受け手も起点を持つので、「0 より前の時刻」を表せない型では、起点を持つ所の全部（時刻の表・受け手の列・テストの偽の時計）に別の型が要る。符号つきなら、起点を持つ所の全部が 1 つの型で済む。時計の値は 2^63 に届かない（約 2 万 9 千年）。TOML の整数（符号つき 64 ビット）にも全部の値が書ける |
| D3 | 範囲の端 | 足し引きの演算子は、端で張り付く（回り込まない）。端を越えたかを知りたい所は `checked_*` で `None` を受ける | 作者の値は端まで来る（選択待ちの時間切れは `i64` の端へ張り付く＝要件 3 の 8）。パニックは致命に限る |
| D4 | 「終わりが無い」 | `Option<TimePoint>` の `None`。終わりの予定の時刻では、`None` は「その時に決まっていない」で、終わらないループはその 1 つである（D19）。周回の後の待ちで最後の値を保つ間は、中の列挙で表す | 型の普通の値（`TimePoint::MAX`）と取り違えない（要件 1 の 7） |
| D5 | 丸め | 最も近い 100ns。ちょうど中間は 0 から遠い側。作者が書く単位からの変え方（`std::time::Duration` からを含む）・`mul_div`・`mul_unit` に使う。倍率の写し（D6）には使わない | 長さを作る計算は、元の値に最も近い長さを作る。0 を挟んで対称になる（負の台本の時刻でも同じ決まり） |
| D6 | 倍率 | `TimeScale`（有限で正の実数）が、前向きの写し F と、その最小の逆 G を持つ。F は切り捨て（負の無限大の側）で求める。出来事の判定も予定の時刻も、この 2 つだけで作る | 写しの意味は「台本の時刻に届いたか」である。切り捨てなら、掛け算の結果が届く前に「届いた」にならない。G を「F が届く最初の値」と定義すると、判定と予定は定義から食い違わない（「倍率の写し」の節） |
| D7 | 直列化 | `CueSheet` は 100ns の整数。storyboard の文書は秒の実数のまま読み書きし、読む縁で 1 回だけ型へ変える | 台本は機械が渡す物・文書は作者が書く物 |
| D8 | 台本の起点 | `CueSheet` の起点は `Option<TimePoint>`。押されていない台本から再生機は作れない（`Err`） | 0 は普通の時刻である。押し忘れた台本が「起点 0＝全部が過ぎた」として一度に配られる道を、型で閉じる |
| D9 | 合図の形 | `TalkCue` に `fire_time: TimePoint` を足す。`at` は `TimeSpan`（台本の時刻）。先渡しの合図の `fire_time` は、その時の起点での予定の値。時刻の表の中では、発火時刻を持たない型（`PendingCue`）で持ち、`TalkCue` は出口（取り出しと先渡し）でだけ組む | 文字の層の空回しは、先渡しの合図を配られる合図と同じ道で当てる。受け口へ渡す型を分けると、同じ道を通せない。表の中の項目は、発火時刻がまだ決まっていないので、その欄を持たない型にして、古い値を持つ合図を作れなくする |
| D10 | 時刻の表 | 取り出し口は `pop_due`（1 件ずつ）だけ。`tick`＋`ready` は無くす。起点は中で 128 ビットで持つ | 取り出しごとに起点を置き直せる（要件 11 の 6）。溜めが無いので、同じ時刻の刻み直しは、守り無しで「何も届かない」「置き直した分だけ届く」になる（要件 11 の 9・10）。トークのスレッドの同じ時刻の守りも要らなくなり、取り除く。型の下の端でも比べが変わらない（要件 11 の 12） |
| D11 | 置き直しの口 | `reanchor(now, at, script_time)`（時刻 `at` が台本の時刻 `script_time` である）と `shift_origin(now, delta)`（負は前へ）。どちらも、呼び手の現在の時刻 `now` を受ける。`now` は、置き直しが効く時刻になる。起点が変わったら `OriginNotice`（効く時刻・新しい起点・前の起点）を返し、変わらなければ何も返さない | 置き直しは、絶対の時刻の上の出来事である。「いつから効くか」が無いと、遅れてまとめて受けた受け手は、その前に起きた事と後に起きる事を分けられない。dola は「今」を覚えないので、効く時刻は呼び手が渡す。動いた長さは「新しい起点 − 前の起点」で求まるので、運ばない。ノベルの作り直しの使い方は、この 2 つの言い方で全部書ける（「使い手の居ない口の確かめ」の節） |
| D12 | 再生機 | `step`（1 件）と `tick`（届く分の全部）。受け口を登録する時に、その受け口へ最初の起点を知らせる（どの合図よりも前）。置き直しで起点が変わるたびに、全部の受け口へ `origin_set` を登録順に配る | 最初の起点 → 合図 → 置き直しの知らせ → 合図の順が、呼んだ順のまま受け口に着く（要件 12 の 2）。最初の起点の知らせは、起点を知らせるだけの物で、置き直しではない。起点を使う受け口は、合図を待たずに起点を知れる。areka の受け手は、絶対の時刻で持つので、これを使わない（D13） |
| D13 | 受け手の時刻の持ち方 | どの受け手も、受け取った物を絶対の時刻で持ち、起点も台本の時刻も持たない。合図の時刻は `cue.fire_time`、字の現れる時刻は `cue.fire_time`＋塊の頭からの隔たり、寿命の事実（表示の終わり・期限）も絶対の時刻。フレームで評価する受け手（文字の層・バルーンの寿命の時計）は、それをフレームの時刻と比べる。最初の起点の知らせでは、何もしない。`TalkClock` は「今」を読む口だけを持つ | 起点を持つ所が受け手ごとに在ると、受け手どうしで起点が食い違う（別の線で物を受ける 2 つの受け手は、同じフレームで別の起点を持ちうる）。持たなければ、食い違う物が無い。最初の起点の知らせを待つ物も無くなり、前の再生の残りの字も、どの受け手でも同じ絶対の時刻に現れる（要件 12 の 8・12 の 9・18 の 2） |
| D14 | 口パク | 1 回の刻みで越えた区切りを、古い順に全部、それぞれの字が現れた時刻で判定する。口パクは、自分の刻みの細かさまで合えば足りる物として扱い、決まりを増やさない | 結果が、刻みの細かさと、合図がまとまって届いた事に依らない（要件 12 の 7・18 の 6）。「いちばん新しい 1 つだけ」を判定する決まりは、刻みの取り方で口の動きが変わる。土台は正確な時刻を渡すが、どれだけ厳しく合わせるかは、中身の種類ごとに決める（「受け手ごとの追い付き方」の節） |
| D15 | 時計の結線 | 刻み役（seriko を進める刻み役を含む）・`FrameTime`・`TalkClock` の既定の時計は、同じ 1 つの関数。kanade と seriko は時計を持たず、渡された時刻（刻み・合図の発火時刻・窓の知らせの時刻）だけで動く | 要件 17。seriko が合図を処理する時に時計を読むと、届いた時刻が姿に出る（D17） |
| D16 | 段取り | 継ぎを置かない。前半は型の付け替え、後半は振る舞い。dola の口と、crate をまたいで運ぶ合図・刻み・選択待ちの知らせの形は、前半で最後の形にする。受け手の側の口（文字の層の提示と読み口・寿命の知らせ・`TalkClock`・seriko の時計と窓の知らせ）は、前半で型を替え、後半でもう一度替わる | 要件 9 の 1（付け替えで変わらない）と 9 の 6（変えた所）を、別々の確かめで示す。受け手の側の口は、起点を見積もる作りの口なので、見積もりをやめる後半でなければ、最後の形にならない（「Migration Strategy」） |
| D17 | 中身が効く時刻 | 姿を変える中身は、届いた時でなく、合図の発火時刻に効く。フレームで評価する受け手は、届いた合図を列に持ち、フレームの時刻がその合図の発火時刻に届いた時に、届いた順に当てる（字と、字のほかの指令を、同じ決まりで）。seriko は、届いた時に当てるが、出来事の時刻に合図の `fire_time` を使う。その場で効く現実の働き（7 つの変更の受け口）は、届いた順にその場で当てる | 届いた時に当てると、フレームの時刻が合図の発火時刻より前のフレームで、まだ来ていない中身が見える。受け手ごとに届く時刻が違えば、姿も違う（要件 18 の 1）。現実の働きは、遅れる評価の時計を持たず、届くのは必ず発火時刻より後なので、届いた時に当てても「発火時刻より前に効く」事が無い |
| D18 | 区切りを解く口 | `resolve_click`・`resolve_choice` は時刻を受けず、時刻に依る事を何も決めない。区切りを外す（選択肢の溜めを空にする）だけ。届いた物を配る事と完了は、呼び手が次に呼ぶ `step`／`tick(now)` が決める | 時刻の表は時刻を覚えない。解く口が完了を決めるには、覚えた時刻が要る。解く口を時刻に依らなくすると、時刻に依る判定の入口は `step` の 1 つになる。本番の呼び手は、解いた直後に、最後に受けた時刻のまま `tick` を呼ぶので、遅れない |
| D19 | 終わりの予定の時刻 | `StartResult.end_time` と `resume` の戻り値の意味は 1 つ: その時に決まっているなら、再生の全体の終わりの時刻。決まっていなければ `None`（終わらないループ・周回の間の待ちを、これから周回ごとに引くループ）。終わりの期限（`finish`）が置いてあれば、それと早い方。今の周回の終わりは、中の値 | 同じ欄が、ループの種類で「全体の終わり」と「今の周回の終わり」に分かれると、受け取った側は欄だけでは意味を読めない |
| D20 | コンパイルの結果 | `compile_storyboard` は開始の時刻を受けない。コンパイルの結果は、storyboard の頭からの隔たり（台本の軸）だけを持つ。絶対の時刻が storyboard の層へ入る所は `DolaRuntime::start(name, start_time)` の 1 つ | 絶対の時刻は、いつも「起点＋隔たり」で作る（cue の層の `CueSheet` と同じ形）。コンパイルの結果に絶対の時刻を焼くと、止めて戻すたび・周回のたびに全部を書き直す事になり、倍率が 1 でない時は、ずらすだけでは足りない。「起点＋隔たり」なら、動くのは起点だけで、1 つのコンパイルの結果が、どの開始の時刻にも使える |
| D21 | 置き直しの知らせを当てる順 | 受け手は、置き直しの知らせを、合図と同じ「届いた順」の中で当てる。seriko は、届いた時に当てる。フレームで評価する受け手は、合図と同じ列に並べ、列に入れる時刻を、効く時刻にする | 知らせが動かすのは、それより前に届いた物である。文字の層は、字の現れる時刻を、合図を当てる時に積むので、前に届いた合図を全部当ててからでなければ、動かす相手がそろわない。届いた順に当てれば、合図をいつ当てたか（フレームの巡り）に依らず、どの受け手も同じ時刻を持つ。知らせの後に届いた合図（新しい起点での発火時刻を持つ）も、効く時刻より前のフレームでは当たらない |
| D22 | どこまで見えるか | 字がどこまで見えるかは、字の現れる時刻とフレームの時刻だけで求める。「見せた字の数」を覚えない | 時刻だけで決まる値である（要件 18 の 6）。覚えると、姿が「どのフレームを回したか」に依り、後ろへ動かす知らせが遅れて着いた受け手は、動いた長さの間ずっと、時刻どおりに受けた受け手と食い違う。覚えなければ、食い違いは、知らせが着くまでの間だけで終わる（「知らせの受け方」の節の限界） |

## File Structure Plan

### Directory Structure

```
crates/dola/
├── Cargo.toml                       # windows の機能を Win32_System_WindowsProgramming へ替える・docs.rs の既定の target を Windows にする
└── src/
    ├── time/                        # 時刻の土台（新しいモジュール。公開の道は dola::time）
    │   ├── mod.rs                   # 子の宣言と再公開
    │   ├── point.rs                 # TimePoint
    │   ├── span.rs                  # TimeSpan
    │   ├── delta.rs                 # TimeDelta
    │   ├── convert.rs               # 作者が書く単位からの変え方・丸めの決まり・TimeRangeError
    │   ├── scale.rs                 # TimeScale（F と G）・比の掛け算・進み具合
    │   ├── text.rs                  # Display と FromStr（値を落とさない秒の小数）
    │   ├── clock.rs                 # now()（cfg(windows)）
    │   ├── testing.rs               # at_ms・ms・delta_ms（機能 test-support）
    │   ├── serde_ticks.rs           # 100ns の整数のアダプタ（cue の層）
    │   ├── serde_secs.rs            # 秒のアダプタ（storyboard の文書）・秒への変え方・文書の値の上限の確かめ
    │   └── *_tests.rs               # 上のファイルごとの隣のテスト
    ├── cue/
    │   ├── command.rs               # Cue・TalkCue・PendingCue・BarrierKind の時刻の欄
    │   ├── schedule.rs              # TimedSchedule・Due・OriginNotice
    │   ├── schedule/reorigin_tests.rs   # 置き直しの単体テスト（新しいファイル）
    │   ├── sheet.rs                 # CueSheet の起点・占有の終わり
    │   ├── runtime.rs               # CuePlayer・CueError
    │   ├── sink.rs                  # CueSink::origin_set
    │   └── test_support.rs          # carrier_cue
    ├── storyboard.rs / transition.rs    # 文書の定義の型
    ├── builder.rs / validate/
    ├── error.rs                     # DolaError（消す variant と足す variant の全部を、家族を並べる前に 1 度で替える）
    ├── compile/{types.rs, mod.rs, resolve.rs}
    │                                # types.rs に、1 周の長さを求める 1 本の関数を置く。開始の時刻の引数と欄は消す
    └── runtime/{facade.rs, types.rs, instance_state.rs, instance_manager/, loop_controller/, timeline_manager/, conflict_resolver.rs}
                                     # clock.rs は消す

crates/dola/tests/cue/
├── runtime_test_reanchor_tests.rs       # 新: 要件 11
├── runtime_test_fire_time_tests.rs      # 新: 要件 13
├── runtime_test_origin_notice_tests.rs  # 新: 要件 12 の 1〜3（最初の起点・効く時刻・変わらない置き直し）
└── runtime_test_late_delivery_tests.rs  # 新: 要件 18 の 3・4

crates/areka-sakura/src/reveal.rs        # 新: 1 字ごとの間隔の式（文字の層と seriko が呼ぶ）
crates/areka-sakura/src/reveal_tests.rs  # 新: 上の単体テスト
crates/areka-sakura/src/held.rs          # 新: 届いた物を、届いた順のまま、絶対の時刻が来るまで持つ列（文字の層とバルーンの寿命の時計が使う）
crates/areka-sakura/src/held_tests.rs    # 新: 上の単体テスト
crates/areka-emo-text/src/actor_held.rs  # 新: 文字の層が列を持つ所と、フレームごとに当てる口（advance）
crates/areka-runtime/src/balloon_visibility_held.rs   # 新: 寿命の知らせを列に持つ所と、フレームごとに時刻の来た物を取り出す口
.kiro/specs/completed/dola-interrupt-ticks/migration-ledger.md   # 新: 記録（外したテスト・取り除いた分かれ道・変える前後の期待・締めの検索）
```

### Modified Files

並行の作業者が同じファイルに触らないための家族の分け方。家族の中は 1 人が持つ。「先」と書いた物は、同じ crate のほかの家族より先に替える。

**dola**

| 家族 | ファイル |
|---|---|
| 時刻のモジュール・型（段 0・先） | `src/time/{mod.rs, point.rs, span.rs, delta.rs, convert.rs, scale.rs, text.rs}` と隣のテスト・`src/lib.rs`（`pub mod time;` の 1 行）。`mod.rs` は、子の宣言の全部（下の家族の分も）を先に書く |
| 時刻のモジュール・時計と縁（段 0） | `src/time/{clock.rs, testing.rs, serde_ticks.rs, serde_secs.rs}` と隣のテスト・`crates/dola/Cargo.toml`（機能 `Win32_System_WindowsProgramming` を足す・`docs.rs` の既定の target）・`tools/test-all.ps1`（検査の段） |
| 土台（段 1・先） | `crates/dola/Cargo.toml`（`Win32_System_Performance` を外す）・`src/lib.rs`（古い再公開を外す）・`src/error.rs`・`src/compile/types.rs`・`src/runtime/mod.rs`（`clock` を外す）・`src/runtime/clock.rs`（消す）・`tests/trigger/common/mod.rs`（文書を組む共有の手助け） |
| cue の本体（先） | `src/cue/{command.rs, schedule.rs, sheet.rs, runtime.rs, sink.rs, mod.rs, test_support.rs, test_support_tests.rs}`・`tests/cue/test_support.rs` |
| cue の単体テスト | `src/cue/command_tests.rs`／`src/cue/schedule/tests.rs`（2 人に割れる） |
| cue の結合テスト | `tests/cue/sheet_test.rs`／`tests/cue/{sheet_sort_test.rs, schedule_test.rs}`／`tests/cue/{runtime_test_*_tests.rs, sink_test.rs}`（3 人に割れる） |
| 文書の型と検査 | `src/{storyboard.rs, transition.rs, builder.rs, document.rs}`・`src/validate/**`・`tests/validation/**`・`tests/general/**`・`tests/trigger/{serde_test.rs, validation_test.rs}`・`tests/runtime/loop_offset_test/loop_offset_serde.rs` |
| コンパイル | `src/compile/{mod.rs, resolve.rs}`・`tests/compile/**`・`tests/trigger/compile_test.rs` |
| runtime の芯 | `src/runtime/{facade.rs, facade_tests.rs, types.rs, instance_state.rs, conflict_resolver.rs}`・`src/runtime/{instance_manager, loop_controller, timeline_manager}/*`・`src/runtime/interpolator/tests.rs`（置物の時刻だけ）・`tests/runtime/**`（上の 1 本を除く）・`tests/trigger/{runtime_test.rs, event_order_test.rs}` |
| 文書 | `crates/dola/README.md` |

2 つ以上の家族が要るファイルは、土台の家族が持つ。

- `src/error.rs`: 「文書の型と検査」（消える variant・`LoopOffsetRangeInverted` の型・文書の値の上限の variant）と「コンパイル」（`LoopDurationOutOfRange`）の両方が要る。variant の足し引きの全部を、土台の家族が 1 度で替える。後の家族は、このファイルを編集しない。`src/error.rs` の中のテスト（表示の字面）も、土台の家族が直す。
- `src/compile/types.rs`: コンパイルの結果の型と、1 周の長さを求める 1 本の関数（`CompiledStoryboard::lap_length`）を置く。「コンパイル」と「runtime の芯」の両方が呼ぶので、土台の家族が先に置く。`CompiledStoryboard.start_time` の欄を消すのも、ここである（`compile_storyboard` の引数を消すのは「コンパイル」の家族。呼ぶ所は「コンパイル」と「runtime の芯」と「文書の型と検査」のテストに在り、それぞれの家族が自分のファイルの呼び出しを直す）。
- `src/time/mod.rs`: 段 0 の 2 つの家族が要る。型の家族が、子の宣言と再公開の全部を先に書き、時計と縁の家族は編集しない。
- `tests/trigger/common/mod.rs`: 秒の実数で文書を組む手助けで、「文書の型と検査」（`validation_test.rs`）・「コンパイル」（`compile_test.rs`）・「runtime の芯」（`runtime_test.rs`）のテストが使う。土台の家族が、型で組む形へ先に替え、後の家族は編集しない。

触らない: `src/easing.rs`・`src/runtime/interpolator/mod.rs`・`src/runtime/subscription_manager/*`・`src/value.rs`・`src/variable.rs`。

**使い手**

| crate | 家族 | ファイル |
|---|---|---|
| `wintf` | 時計と世界（先） | `Cargo.toml`（変えない）・`src/ecs/graphics/core.rs`（`FrameTime`）・`src/ecs/world/mod.rs`・`src/ecs/dola/mod.rs`・`tests/ecs/dola_animator_test.rs`・`FrameTime(` を書くテスト（`world_tick_gate_tests.rs`・`tests/graphics/surface_embedding_test.rs`・`systems_tests.rs`） |
| | タイプライター | `src/ecs/widget/text/typewriter/{mod.rs, tests.rs}`・`typewriter_ir.rs`・`typewriter_layout.rs`・`typewriter_draw.rs`・`examples/typewriter_demo.rs` |
| | 自動の上げ下げ | `src/ecs/graphics/visual/{promotion.rs, systems.rs, promotion_tests.rs, promotion_round_log_tests.rs, systems_promotion_tests.rs}` |
| `areka-talk` | 全部（先） | `Cargo.toml`（dola を足す・説明の「依存 0」を改める）・`src/lib.rs`（`ChoiceWaiting`） |
| `areka-sakura` | 本体（先） | `src/{contract.rs, duration.rs, reveal.rs, held.rs, compile.rs, drive.rs, strict_tap.rs, lib.rs}`・`src/drive_test_support.rs` |
| | コンパイルのテスト | `src/compile_*_tests.rs` |
| | 再生のテスト | `src/drive_*_tests.rs` |
| | 受け手の決まりのテスト | `src/{reveal_tests.rs, held_tests.rs}` |
| `areka-kanade` | 本体と組み立て（先） | `Cargo.toml`（dola を足す）・`src/{msg.rs, lib.rs, actor.rs}`・`src/schedule/{types.rs, choice.rs, close.rs, change.rs, events.rs, steady.rs, mod.rs}`・`tests/kanade/*_test_support.rs`・`tests/kanade/common/**` |
| | `src` のテスト | `src/**/*_tests.rs` |
| | 結合テスト | `tests/kanade/**` |
| `areka-seriko` | 時計と表の縁（先） | `src/{actor.rs, actor_bind.rs, table.rs, table_calls.rs, settle.rs}`・`src/{lib_test_support.rs, actor_test_support.rs}`・`tests/common/mod.rs` |
| | ループと部品の芯 | `src/{looper.rs, looper_trigger.rs, looper_calls.rs, looper_refresh.rs, parts.rs, parts_trigger.rs, parts_calls.rs, parts_refresh.rs, timeline.rs, trigger.rs, calls.rs, talk.rs}` |
| | テスト（2 人に割れる） | `src/looper*_tests.rs`・`src/talk_tests.rs`・`src/trigger_tests.rs`／`src/parts*_tests.rs`・`src/actor*_tests.rs`・`tests/**` |
| `areka-emo-text` | 本体と組み立て（先） | `src/{state.rs, actor.rs, actor_held.rs, actor_present.rs, actor_box.rs, sink.rs}`・`src/{actor_test_support.rs, state_test_support.rs, viewbox_draw_test_support.rs}`・`tests/support/{mod.rs, fixture.rs}` |
| | `src` のテスト（2 人に割れる） | `src/state_*_tests.rs`・`src/lookahead_tests.rs`・`src/choice_anchor_tests.rs`／`src/actor_*_tests.rs`・`src/viewbox_*_tests.rs` |
| | 結合テストと例 | `tests/*.rs`・`tests/decoration_readback/`・`examples/**` |
| `areka-ghost` | 刻み役と時計（先） | `Cargo.toml`（dola を本番の依存へ足す）・`src/{ticker.rs, ticker_tests.rs, runtime.rs, config.rs, runtime_choice_wake_tests.rs}` |
| | dispatcher | `src/{dispatcher.rs, dispatcher_*_tests.rs}` |
| | 受け口と結合テスト | `src/{sink.rs, prop_sink.rs, prop_get_sink.rs, test_support.rs, translate_wiring_e2e_tests.rs}`・`tests/**` |
| `areka-runtime` | 土台と結線（先） | `src/{lib.rs, talk_clock.rs, talk_lifecycle.rs, shell_carry.rs, balloon_visibility_wait.rs, hover_inject.rs}`・`src/frame.rs`（後半: 進める相を足す）・`src/frame/wiring.rs`（後半: 寿命の列の欄）・`src/{frame_test_support.rs, consumer_ledger_test_support.rs, balloon_visibility_test_support.rs, balloon_visibility_phase_test_support.rs, frame_balloon_runtime_test_support.rs}`・`src/{talk_lifecycle_tests.rs, talk_lifecycle_signals_tests.rs, seriko_clock_tests.rs}` |
| | バルーンの判断 | `src/{balloon_visibility.rs, balloon_visibility_decision.rs, balloon_visibility_tests.rs, balloon_visibility_box_tests.rs, balloon_visibility_timeout_*}` |
| | バルーンの相 | `src/balloon_visibility_phase*`（組み立てを除く）・`src/balloon_visibility_held.rs`（後半） |
| | バルーンの通し | `src/{balloon_visibility_choice_wait_tests.rs, balloon_visibility_lifecycle_e2e_tests.rs, balloon_visibility_talk_end*}` |
| | 変更の受け口 | `src/{change_cue*, move_cue*, reload_cue*, switch_cue*, zorder_cue*, zorder_wiring*, window_state_cue*, user_break_cue*}`・`src/frame/window_state_tests.rs` |
| | フレーム | `src/frame/{scale_text.rs, status_report.rs}`・`src/frame_*_tests.rs`・`src/{strict_judges_tests.rs, film_playback_e2e_tests.rs}` |
| `areka-input` | 全部 | `Cargo.toml`（`[dev-dependencies]` に dola を足す）・`src/user_break.rs`（後半: 利用者の中断の知らせに時刻を付ける）・`src/balloon_test_support.rs`・`TalkCue {` と `present_frame(` を書くテスト |
| `areka-session` | 全部 | `src/{runtime_boot.rs, runtime_boot_tests.rs, boot_config.rs, ghost_switch_test_support.rs, shell_balloon_switch_session_lap_test_support.rs}`・時計を差し込むテスト |
| 本体 `areka` | 5 つに割れる | `src/runtime_tests/**`／`src/session_tests/**`／`src/mcp/**`／`src/{install, update}/*_cue*`・`src/thread_roles_tests.rs`・`src/tick_gate_config_producers_tests.rs`（後半: 旗を立てる所の一覧）／`examples/emo-present/**` |
| 先進坑 | 1 か所 | `crates/pilot/examples/pilot-windows-rs-next/q7-vertical/src/old_side.rs` |

**依存の行を足す `Cargo.toml`**: どれも、その crate の「先」の家族が持つ。

- 本番の依存に dola を足す: `areka-talk`・`areka-kanade`・`areka-ghost`（`areka-ghost` は今 `[dev-dependencies]` にだけ在る）。
- テストで `at_ms`・`ms`・`delta_ms` を使う crate は、`[dev-dependencies]` に、機能 `test-support` を付けた dola の行を持つ（dola の今の機能で、正準のキャリアの合図の組み立てと同じ機能）。今も在るのは `areka-seriko`・`areka-ghost`・`areka-runtime`。足すのは `wintf`・`areka-sakura`・`areka-kanade`・`areka-emo-text`・`areka-input` である（`areka-input` は、本番では dola を引かないので、`[dev-dependencies]` にだけ足す）。`areka-talk` は、テストが時刻を書かないので、足さない。`areka-session` と本体は、dola を直に引いていて、機能は `areka-runtime` の機能 `test-support` を通して付く（`areka-runtime` の `Cargo.toml` の `dola/test-support`）。
- dola を直に引かない crate（`areka-input`）の本番のコードが、時刻の型を名指す時は、`areka_sakura::contract` から名指す。`contract.rs` は、dola の cue の型と一緒に、その欄の型である `TimePoint`・`TimeSpan`・`TimeDelta` を再公開する（`TalkCue` を名指せる道から、その欄の型も名指せる）。テストの組み立て（`dola::time::testing`）は、本番の道からは再公開しない。

**道具と文書**

- 根の `Cargo.toml` は触らない（dola の行は今も在る。`windows` の機能は dola の `Cargo.toml` で足す。版の行も動かさない）。
- `tools/test-all.ps1`: 段 `dola non-windows check` を足す（段 `dola toml feature tests` の直後。中身は `rustup target add x86_64-unknown-linux-gnu` と `cargo check -p dola --features toml --target x86_64-unknown-linux-gnu -j 4`）。dola が時刻の土台を含むので、この 1 段が、型と丸めと直列化のアダプタが Windows でない所でコンパイルできる事も示す。
- `.kiro/steering/tech.md`（dola の依存の行・全体テストの段・`areka-kanade`／`areka-talk`／`areka-ghost` が dola を引く事）・`.kiro/steering/structure.md`（cue の層の説明・dola のモジュールの一覧）。
- `crates/dola/README.md`: 時刻の型と長さの型と符号つきの差・「今」を読む関数・作者が書く単位からの変え方と丸めの決まり・眠りの間を数えない決まりを、今の姿だけで書く。付け替えの前の形の説明と、書き換え方の案内は書かない。
- コードの説明: `crates/wintf/src/ecs/graphics/core.rs`（`FrameTime`）・`crates/wintf/src/ecs/world/tick_wake.rs`（後半: 旗を立てる所の一覧の `PRESENT` の行に、文字の層の受け取りを足す）・`crates/areka-runtime/src/{talk_clock.rs, lib.rs}`・`crates/areka-sakura/src/{contract.rs, duration.rs}`・`crates/areka-kanade/src/msg.rs`・`crates/areka-ghost/src/ticker.rs`・`crates/areka-seriko/src/{actor.rs, talk.rs, looper_refresh.rs}`・`crates/areka-emo-text/src/{state.rs, actor.rs, sink.rs}`（「届いた順にその場で当てる」の説明）・dola の `cue/*`・`runtime/facade.rs`・`compile/types.rs`（区間と引き金の時刻を「絶対時刻」と書いている説明を、storyboard の頭からの隔たりへ直す）。

## System Flows

### 1 回の刻みの中の、取り出し・置き直し・知らせの順

```mermaid
sequenceDiagram
    participant Caller as 呼び手
    participant Player as CuePlayer
    participant Sched as TimedSchedule
    participant Sinks as 受け口の全部
    Caller->>Player: register_sink
    Player->>Sinks: origin_set 最初の起点
    Player->>Sinks: preview
    Caller->>Player: step now
    Player->>Sched: pop_due now
    Sched-->>Player: 合図 k と発火時刻
    Player->>Sinks: emit 合図 k
    Player-->>Caller: Delivered
    Caller->>Player: shift_origin now delta
    Player->>Sched: shift_origin now delta
    Sched-->>Player: OriginNotice 効く時刻は now
    Player->>Sinks: origin_set
    Caller->>Player: step now
    Player->>Sched: pop_due now
    Sched-->>Player: 合図 k+1 と新しい起点での発火時刻
    Player->>Sinks: emit 合図 k+1
    Player-->>Caller: Delivered
    Caller->>Player: step now
    Player-->>Caller: Idle
```

- 受け口は、登録の時に最初の起点を受ける。どの合図よりも前である。
- 合図 k の発火時刻は、取り出した時の起点で決まり、後の置き直しで変わらない。合図 k+1 は、置き直した後の起点で決まる。
- 置き直しの知らせは、呼び手が渡した `now` を「効く時刻」として運ぶ。再生機も時刻の表も、`now` を覚えない。
- `tick(now)` は、`step(now)` を `Delivered` が返る間くり返すだけである。置き直しを挟まない呼び手は `tick` を呼ぶ。
- 区切りで止まっている間・最初の刻みの前でも、`reanchor`／`shift_origin` は受け付ける。完了の後と、起点が変わらない置き直しは、何も知らせない（`None` を返す）。

### 時刻が受け手へ届く道（後半の後）

```mermaid
sequenceDiagram
    participant Ticker as 刻み役
    participant Disp as dispatcher
    participant TalkT as トークのスレッド
    participant Player as CuePlayer
    participant Text as 文字の層の列
    participant Seriko as seriko
    participant Frame as フレーム
    Ticker->>Disp: Tick now
    Disp->>TalkT: SakuraMsg Tick now
    Note over TalkT: 最初の刻みの時刻を台本の起点に押す
    TalkT->>Player: register_sink
    Note over Player: 最初の起点の知らせは受け口が受けて何も送らない
    TalkT->>Player: tick now
    Player->>Text: emit 合図 fire_time つき
    Note over Text: 届いた順のまま列に持つ
    Player->>Seriko: emit 合図 fire_time つき
    Note over Seriko: 出来事の時刻は合図の fire_time
    Frame->>Text: advance frame_now
    Note over Text: fire_time の来た合図を届いた順に当てる
```

- 刻みが運ぶ値は、刻み役が起きた時に読んだ時計の値のままで、dispatcher も トークのスレッドも変えない。
- 受け手は、起点を持たない。合図の `fire_time`（再生機が、取り出した時の起点＋相対で決めた絶対の時刻）を、そのまま使う。届いた時刻は、時刻の計算に使わない。
- 文字の層は、届いた時には当てない。フレームの時刻が `fire_time` に届いた物だけを、フレームごとに、届いた順に当てる。seriko は、届いた時に当てるが、時計を読まずに、合図の `fire_time` を出来事の時刻にする。
- 最初の起点の知らせ（前の起点が無い）は、受け口が受けて、何も送らない。置き直しの知らせ（前の起点が在る）だけを、合図と同じ線で、届いた順のまま受け手へ送る。

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1, 1.2, 1.3, 1.4 | 型と整数の足し引き | 時刻の土台 | `TimePoint`・`TimeSpan`・`TimeDelta` と演算子 | — |
| 1.5, 1.9 | 負の差を区別して運ぶ | 時刻の土台 | `TimePoint - TimePoint -> TimeDelta`・`span_since -> Option<TimeSpan>` | — |
| 1.6 | 回り込ませない | 時刻の土台 | 張り付く演算子と `checked_*` | — |
| 1.7 | 「終わりが無い」 | 時刻の土台・`DolaRuntime` | `Option<TimePoint>`・`ValueTime` | — |
| 1.8, 2.7, 15.8 | Windows でない所でのコンパイル | 時刻の土台・全体テストの段 | `cfg(windows)` は `clock.rs` だけ・段 `dola non-windows check` | — |
| 2.1, 2.2, 2.3, 2.4, 2.5 | 時計 | 時刻の土台 | `now() -> TimePoint` | — |
| 2.6, 8.4 | 古い時計を無くす | dola・使い手 | `runtime/clock.rs` を消す | 前半 |
| 2.8 | 時刻を外から受ける（受けた時刻を覚えない） | cue の層・storyboard の層 | `step(now)`・`tick(now)`・`reanchor(now, ..)`・`start(name, start_time)`。区切りを解く口は時刻を受けない | — |
| 3.1, 3.2, 3.3, 3.4, 3.7, 3.8 | 作者が書く単位からの変え方 | 時刻の土台・使い手の縁 | `convert.rs` の口・`TimeRangeError` | — |
| 3.5, 3.6 | 実数の掛け算の 1 か所（倍率の写しは切り捨て・ほかは最も近い 100ns） | 時刻の土台 | `TimeScale`・`mul_div`・`mul_unit` | — |
| 4.1, 4.2, 4.3, 4.4, 4.5 | cue の層の口 | cue の型・`CueSheet`・`TimedSchedule`・`CuePlayer` | 「dola の cue の層」の節 | — |
| 4.6 | 時間切れの指令 | cue の型・`areka-talk`・`areka-kanade` | `BarrierKind::WaitForChoice { timeout: Option<TimeDelta> }` | — |
| 5.1, 5.2, 5.3, 5.4, 5.10 | storyboard の層の口 | 文書の型・コンパイル・`DolaRuntime` | 「dola の storyboard の層」の節。絶対の時刻が入る所は `start(name, start_time)` だけで、`compile_storyboard(doc, name)` は開始の時刻を受けない | — |
| 5.5, 5.6, 5.8 | 終わらないループ | `DolaRuntime`・コンパイルの結果 | `StartResult.end_time: Option<TimePoint>`（決まっていなければ `None`）・`CompiledStoryboard::lap_length`・`MIN_LOOP_DURATION`・`checked_add` | — |
| 5.7, 5.11 | 進み具合と倍率の写し | 時刻の土台・`DolaRuntime` | `TimeScale::forward`・`first_reaching`・`ratio_of` | — |
| 5.9 | 文書の定義の型 | 文書の型 | `TransitionDef`・`LoopOffset`・`KeyframeRef`・`StoryboardEntry` | — |
| 6.1, 6.2, 6.3, 6.4, 6.5, 6.9 | `CueSheet` の直列化 | `dola::time` のアダプタ・`CueSheet` | `serde_ticks` | — |
| 6.6, 6.7 | 文書の秒の読み書き | `dola::time` のアダプタ・文書の型・検査 | `serde_secs`（読む・書く・検査の 3 か所で、文書の値の上限を確かめる） | — |
| 6.8 | 書式ごとの往復のテスト | dola のテスト | JSON と TOML | — |
| 7.1, 7.2, 7.3, 7.5 | 使い手の追従 | 使い手の全部 | 「使い手」の節 | 前半 |
| 7.4 | 作者のミリ秒を縁で 1 回 | `areka-sakura`・`areka-seriko`・`areka-runtime` | `duration.rs`・`table.rs`・`balloon_visibility_wait.rs` | — |
| 7.6 | 正準のキャリアの合図 | cue の層 | `carrier_cue` | — |
| 7.7 | 決定論のテスト | 使い手の全部 | 偽の時計・`dola::time::testing` | — |
| 7.8 | ログ | 時刻の土台・使い手 | `Display`・「ログの形」の節 | — |
| 7.9 | 起きなくなった分かれ道 | 使い手・記録 | `migration-ledger.md` | — |
| 7.10, 9.8 | 1 字ごとの間隔 | `areka-sakura` | `reveal::chunk_reveal_offsets` | — |
| 7.11 | 先進坑 | 先進坑 | `old_side.rs` | 前半 |
| 7.12 | wintf のタイプライターと自動の上げ下げ | wintf | 「wintf」の節 | — |
| 8.1, 8.2, 8.3, 8.5 | 「今」を読む本番の所 | wintf・`areka-runtime` | `FrameTime(TimePoint)`・`TalkClock` の既定の時計 | — |
| 9.1, 9.2, 9.3 | 付け替えで変わらない事 | テスト・全体テストの 1 回目 | 「Testing Strategy」「Migration Strategy」 | 前半の締め |
| 9.4, 9.5, 9.7 | 外す・置き換えるテスト | 記録 | `migration-ledger.md` | — |
| 9.6 | 変える前後の期待 | 記録 | `migration-ledger.md` | 後半 |
| 10.1, 10.2, 10.3, 10.4, 10.5 | 止めて戻す | `DolaRuntime` | `InstanceState::Paused { since }`・`pause`・`resume`（終わりの期限は 10.2 が除く） | — |
| 11.1, 11.2, 11.3, 11.4, 11.5 | 起点の置き直し | `TimedSchedule`・`CuePlayer` | `reanchor(now, at, script_time)`・`shift_origin(now, delta)`（現在の時刻を受ける） | 刻みの中の順 |
| 11.6, 11.7, 11.8 | 取り出しごと・区切りの間・後ろの起点 | `TimedSchedule`・`CuePlayer` | `pop_due`・`step` | 刻みの中の順 |
| 11.9, 11.10, 11.11 | 同じ時刻の刻み直し・置き直さない時 | `TimedSchedule`・`areka-sakura` | 溜めを持たない取り出し・`drive.rs` は同じ時刻の刻みを読み飛ばさない | — |
| 11.12 | 型の下の端（届き方が変わらない） | `TimedSchedule` | 中の 128 ビットの起点 | — |
| 12.1, 12.2, 12.3 | 知らせ（最初の起点と置き直し） | `CuePlayer`・`CueSink` | `origin_set(OriginNotice)`・`register_sink` の最初の起点 | 刻みの中の順 |
| 12.4, 12.5, 12.8, 12.9, 12.10, 12.11, 12.12, 12.13, 12.14 | 文字の層とバルーンの寿命（絶対の時刻で持ち、起点を持たない） | `areka_sakura::held`・`areka-emo-text`・`areka-runtime` | `HeldQueue`・`advance(frame_now)`・`TalkLifecycleSignal`（絶対の時刻）・「知らせの受け方」の節 | 時刻が届く道 |
| 12.6, 12.7 | seriko | `areka-seriko` | `TalkFeed`（絶対の時刻）・`SerikoMsg::OriginSet`・口パクの決まり | 時刻が届く道 |
| 13.1, 13.2, 13.3, 13.4, 13.5, 13.6, 13.7 | 絶対の発火時刻 | `TimedSchedule`・`CuePlayer`・`TalkCue`・`PendingCue` | `Due::Payload { fire_time, at, .. }`・`PendingCue::fire`・`TalkCue::fire_time` | 刻みの中の順 |
| 14.1, 14.2, 14.4, 14.5 | 絶対の刻み | `areka-ghost`・`areka-sakura` | `SakuraMsg::Tick(TimePoint)` | 時刻が届く道 |
| 14.3, 17.8 | 最初の刻みを起点に押す | `areka-sakura` | `drive.rs` の最初の刻み | 時刻が届く道 |
| 14.6 | 表示の終わりを換算せずに渡す | `areka-talk`・`areka-ghost`・`areka-kanade` | `ChoiceWaiting { display_end: TimePoint, .. }` | — |
| 14.7, 17.1, 17.2, 17.4 | 時計の統一 | `areka-ghost`・`areka-seriko`・`areka-runtime` | 刻み役の時計（seriko を進める刻み役を含む）。seriko は渡された時刻だけで動く | — |
| 15.1, 15.2, 15.3, 15.5 | 1 本で終える | 段取り | 「Migration Strategy」 | 全体 |
| 15.4 | 公開の口に `f64` の秒が無い | 締めの検査 | 「締めの検査」の節 | 締め |
| 15.6 | 外の crate を足さない | dola | `windows` の機能を 1 つ替えるだけ。新しい crate も作らない | — |
| 15.7 | 1,000 行 | 段取り | 「1,000 行に近いファイル」の節 | — |
| 16.1, 16.2, 16.3, 16.4 | 文書 | 文書 | README・`tech.md`・`structure.md`・コードの説明 | 締め |
| 17.3, 17.5, 17.7 | kanade の型・締め切り・`OnSecondChange` | `areka-kanade` | 「areka-kanade」の節 | — |
| 17.6, 17.10 | 眠り | 時刻の土台・テスト | 時計の説明・偽の時計を止めるテスト | — |
| 17.9 | 台本の道とフレームの時刻が同じ物差し | 全部 | `TimePoint` | — |
| 18.1, 18.2 | 再生の位置は台本と起点と今だけで決まる | `CuePlayer`・受け手 | D13・D14・D17・D18・D21・D22 | — |
| 18.3, 18.4 | 過ぎた起点・まだ来ていない起点 | `CuePlayer`・`areka_sakura::held` | `pop_due`・`HeldQueue::pop_due` | — |
| 18.5, 18.6, 18.7 | 追い付き方・中身が効く時刻 | 受け手の全部 | 「受け手ごとの追い付き方」の節・「知らせの受け方」の節 | — |

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies | Contracts |
|-----------|--------------|--------|--------------|------------------|-----------|
| 時刻の土台（`dola::time`） | dola | 型・丸め・倍率・時計・直列化のアダプタ | 1, 2, 3, 5.7, 5.11, 6 | `windows`（P0・Windows だけ）・serde（P0） | Service |
| cue の型・`CueSheet` | dola cue | 台本と合図の時刻の欄 | 4, 6, 13 | `dola::time`（P0） | State |
| `TimedSchedule` | dola cue | 取り出し・起点・占有の終わり | 4, 11, 13 | cue の型（P0） | Service, State |
| `CuePlayer`・`CueSink` | dola cue | 配る・起点を知らせる・区切り | 11, 12, 13, 18 | `TimedSchedule`（P0） | Service, Event |
| 文書の型・コンパイル | dola storyboard | 定義の型とコンパイルの結果 | 5, 6 | `dola::time`（P0） | State |
| `DolaRuntime` | dola storyboard | 開始・刻み・止める・戻す | 5, 10 | コンパイル（P0）・`TimeScale`（P0） | Service, State |
| wintf | 使い手 | `FrameTime`・タイプライター・自動の上げ下げ | 7.12, 8 | dola（P0） | State |
| `areka-talk` | 使い手 | `ChoiceWaiting` | 4.6, 14.6 | dola（P0・`dola::time` だけを使う） | Event |
| `areka-sakura` | 使い手 | 作者の単位の縁・刻み・1 字ごとの間隔・届いた物を持つ列 | 7, 12, 14, 18 | dola（P0） | Service, Event, State |
| `areka-kanade` | 使い手 | 締め切り・`OnSecondChange` | 17 | dola（P0・`dola::time` だけを使う） | State |
| `areka-ghost` | 使い手 | 刻み役・dispatcher | 14, 17 | dola（P0） | Event |
| `areka-seriko` | 使い手 | 表の縁・口パク・姿を変える指令の出来事の時刻 | 12, 17, 18 | dola（P0）・`areka-sakura`（P0） | State |
| `areka-emo-text` | 使い手 | 字の現れる時刻・時刻の来た合図を当てる | 7, 12, 18 | `areka-sakura`（P0） | State |
| `areka-runtime` | 使い手 | `TalkClock`・バルーンの寿命・結線・手で組む合図の時刻 | 8, 12, 17, 18 | 上の全部（P0） | State |

### 時刻の土台（`dola::time`）

この節のファイルは、`crates/dola/src/time/` の下に在る。公開の道は `dola::time::*` で、よく使う物（`TimePoint`・`TimeSpan`・`TimeDelta`・`TimeScale`・`now`）は dola の頭でも再公開する。

#### 型（`point.rs`・`span.rs`・`delta.rs`）

| Field | Detail |
|-------|--------|
| Intent | 時刻・長さ・符号つきの差を、100ns の整数の別々の型で表す |
| Requirements | 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9 |

**Contracts**: Service [x]

```rust
/// 時刻（ある瞬間）。1 は 100ns。QueryUnbiasedInterruptTimePrecise の値と同じ物差し。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct TimePoint(i64);

/// 長さ（0 以上）。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct TimeSpan(i64);

/// 符号つきの差。負は「前」。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct TimeDelta(i64);

impl TimePoint {
    pub const ZERO: Self; pub const MIN: Self; pub const MAX: Self;
    pub const fn from_ticks(ticks: i64) -> Self;
    pub const fn as_ticks(self) -> i64;
    pub fn checked_add(self, span: TimeSpan) -> Option<Self>;
    pub fn checked_add_delta(self, delta: TimeDelta) -> Option<Self>;
    /// self が earlier 以後なら、その長さ。前なら None。
    pub fn span_since(self, earlier: TimePoint) -> Option<TimeSpan>;
}
impl TimeSpan {
    pub const ZERO: Self; pub const MAX: Self;
    /// 100ns の数から作る。負なら Err（TimeRangeError::Negative）。
    pub const fn try_from_ticks(ticks: i64) -> Result<Self, TimeRangeError>;
    pub const fn as_ticks(self) -> i64;
    pub fn checked_add(self, other: TimeSpan) -> Option<Self>;
    pub fn checked_sub(self, other: TimeSpan) -> Option<Self>;
    pub fn to_std(self) -> std::time::Duration; // ちょうどの値
}
impl TimeDelta {
    pub const ZERO: Self; pub const MIN: Self; pub const MAX: Self;
    pub const fn from_ticks(ticks: i64) -> Self;
    pub const fn as_ticks(self) -> i64;
    pub fn is_negative(self) -> bool;
    /// 0 以上なら長さ。負なら None。
    pub fn to_span(self) -> Option<TimeSpan>;
    pub fn checked_add(self, other: TimeDelta) -> Option<Self>;
}
impl From<TimeSpan> for TimeDelta { /* 値は変わらない */ }
```

演算子（どれも整数のまま・端で張り付く）:

| 式 | 結果 |
|---|---|
| `TimePoint ± TimeSpan`・`TimePoint ± TimeDelta` | `TimePoint` |
| `TimePoint - TimePoint` | `TimeDelta` |
| `TimeSpan + TimeSpan` | `TimeSpan` |
| `TimeSpan - TimeSpan` | `TimeDelta` |
| `TimeDelta ± TimeDelta`・`-TimeDelta` | `TimeDelta` |

- `TimePoint + TimePoint`・`TimeSpan` と `TimeDelta` の取り違えは、演算子が無いのでコンパイルで止まる。
- 負になりうる引き算は `TimeDelta` を返す。0 へ張り付ける引き算は置かない（負を黙って 0 にしない）。
- 張り付いた値は、型の普通の端の値である。「終わりが無い」は `Option` で表し、端の値と取り違えない。
- `Default` は 0（`FrameTime` の `Default` と、直列化の既定の長さ 0 に使う）。
- `TimeSpan::try_from_ticks` は、公開の口である。100ns の数は型の物差しそのもので、型から読む口（`as_ticks`）と対になる。直列化の読み込み（`serde_ticks`）も、同じ変え方を通る。ミリ秒に載らない値（1 字ごとの間隔の 333,333 など）を書くテストも、この口で書く。`TimePoint`・`TimeDelta` の `from_ticks` は、負も普通の値なので、失敗が無い。

#### 作者が書く単位からの変え方（`convert.rs`）

| Field | Detail |
|-------|--------|
| Intent | ミリ秒・秒・`std::time::Duration` から長さと符号つきの差を作る口と、丸めの決まりを 1 か所に置く |
| Requirements | 3.1, 3.2, 3.3, 3.4, 3.7, 3.8 |

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeRangeError { NotFinite, Negative, OutOfRange }

impl TimeSpan {
    /// 1 ミリ秒＝10,000。u32 の全部の値がちょうど収まる。
    pub const fn from_millis(ms: u32) -> TimeSpan;
    pub fn try_from_millis(ms: u64) -> Result<TimeSpan, TimeRangeError>;
    /// 最も近い 100ns。ちょうど中間は 0 から遠い側。
    pub fn try_from_secs_f64(secs: f64) -> Result<TimeSpan, TimeRangeError>;
}
/// 100ns より細かい端数は、最も近い 100ns へ丸める。失敗は、範囲の外だけ。
impl TryFrom<std::time::Duration> for TimeSpan { type Error = TimeRangeError; }
impl TimeDelta {
    /// 端を越える値は、端へ張り付ける。
    pub const fn from_millis_saturating(ms: i64) -> TimeDelta;
    pub fn try_from_secs_f64(secs: f64) -> Result<TimeDelta, TimeRangeError>;
}
```

- 丸めの決まりは、このファイルの先頭の説明に 1 度だけ書く: 「最も近い 100ns・ちょうど中間は 0 から遠い側・有限でない値と表せない値は `Err`」。
- `TryFrom<std::time::Duration>`: `Duration` は 1ns まで持つ。100ns より細かい端数は、作者が書く単位からの変え方の全部と同じ決まり（最も近い 100ns・ちょうど中間は 0 から遠い側）で丸める。端数が在る事は失敗にしない。失敗になるのは、丸めた値が型の範囲を越える時だけ（`TimeRangeError::OutOfRange`）。100ns の倍数の `Duration`（ミリ秒から作った物を含む）は、ちょうどの値になる。
- 秒の実数を返す口は、`TimeSpan` にも `TimeDelta` にも置かない。`dola::time` の公開の口に出る秒の実数は、作る口の引数（`try_from_secs_f64` の `secs`）だけである。storyboard の文書へ秒で書く計算は、アダプタ（`serde_secs`）が `as_ticks()` の整数から自分の中で行う。進み具合は `ratio_of` で求める。ログは `Display` で出す。
- 使い手は、読み込む縁で 1 回だけこの口を呼び、後は型のまま運ぶ。

#### 倍率の写し（`scale.rs`）

| Field | Detail |
|-------|--------|
| Intent | 実数を時間に掛けて整数へ戻す計算を 1 か所に置き、前向きの写しとその逆が食い違わない事を保証する |
| Requirements | 3.5, 3.6, 5.7, 5.11 |

```rust
/// 再生の速さの倍率（有限で正）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimeScale(f64);

impl TimeScale {
    pub const ONE: Self;
    /// 有限で正でなければ None。
    pub fn new(value: f64) -> Option<Self>;
    pub fn get(self) -> f64;
    /// F: 経った長さ → 台本の時刻。まだ届いていない側（負の無限大の側）へ切り捨てる。
    pub fn forward(self, elapsed: TimeDelta) -> TimeDelta;
    /// G: 台本の時刻 script に届く、最初の経った長さ。どの長さでも届かなければ None。
    /// script が 0 以上なら、返る値も 0 以上。
    pub fn first_reaching(self, script: TimeDelta) -> Option<TimeDelta>;
}
impl TimeSpan {
    /// self × num ÷ den（最も近い 100ns）。途中は 128 ビットで、端数を積もらせない。den は 0 でない。
    pub fn mul_div(self, num: u64, den: u64) -> TimeSpan;
    /// self × ratio（ratio は 0 以上 1 以下へ収めて掛ける）。
    pub fn mul_unit(self, ratio: f64) -> TimeSpan;
}
impl TimeDelta {
    /// self ÷ whole を実数の比で返す（進み具合）。whole が 0 なら None。
    pub fn ratio_of(self, whole: TimeDelta) -> Option<f64>;
}
```

**丸めの向きは 2 つ在り、意味で分ける**:

- 長さを作る計算（`mul_div`・`mul_unit`）は、D5 の決まり（最も近い 100ns）で丸める。作る値が、元の値に最も近い長さだからである。
- 倍率の写し F は、切り捨て（負の無限大の側）で求める。F の意味は「台本の時刻が、どこまで届いたか」である。最も近い値へ丸めると、掛け算の結果が `s − 0.5` を越えた所で「s に届いた」になり、届く前に出来事が起きる。切り捨てなら、届く前に「届いた」にならない。

**F の定義**: 倍率を k とする。k が 1 なら `F(e) = e`（恒等・どの値でもちょうど）。k が 1 でないなら `F(e) = 張り付け(切り捨て((e を f64 にした値) × k))`。切り捨ては、負の無限大の側の整数へ（負の値でも 0 の側へ寄せない）。張り付けは、型の範囲の端へ。

**届く前に「届いた」にならない**: 整数 s について、`F(e) ≥ s` は「`(e を f64 にした値) × k` の掛け算の結果が s 以上である」事と同じである（切り捨てた整数が s 以上である事と、切り捨てる前の値が s 以上である事は、s が整数なら同じ）。

**F は減らない**: `e1 ≤ e2` なら、整数から f64 への変換は順を保ち、正の k を掛ける IEEE の掛け算は順を保ち、切り捨ては順を保ち、端への張り付けも順を保つ。だから `F(e1) ≤ F(e2)`。

**G の定義**: `G(s) = F(e) ≥ s となる最小の e`。`F(TimeDelta::MAX) < s` なら `None`。求め方は、`s ÷ k` の見積もりから両側へ挟み、F だけを使って二分する（F の呼び出しは多くて 64 回ほど）。

**食い違わない理由**: F が減らず、G が「F が届く最小の値」なので、どの e についても「`e ≥ G(s)`」と「`F(e) ≥ s`」は同じ事である（左から右は F が減らない事から、右から左は G が最小である事から）。出来事の判定（経った長さ e で、台本の時刻が s に届いたか＝`F(e) ≥ s`）と、予定の時刻（起点＋`G(s)` に今が届いたか＝`e ≥ G(s)`）は、どの e でも同じ答えを出す。この理由は、F が減らない事だけに依る。

**そこから出る事**:

- `G(0) = 0`（どの倍率でも）。`F(0) = 0` で、e が負なら `e × k` は負なので、切り捨てた値は −1 以下になる。開始より前の刻み（経った長さが負）で、台本の時刻 0 に届いた事にならない。
- G も減らない（`s1 ≤ s2` なら `G(s1) ≤ G(s2)`）。だから `s ≥ 0` なら `G(s) ≥ G(0) = 0` で、`s > 0` なら `G(s) ≥ 1` である（`F(0) = 0 < s`）。1 周の長さ `L = G(全体の長さ)` は、全体の長さが `TimeSpan`（0 以上）なので負にならず、`TimeSpan` に収まる。
- 周回の中の引き金（発火の台本の時刻 ≤ 全体の長さ）の予定の時刻は、周回の終わりの時刻を越えない。今の「周回の終わりの時刻で抑える」継ぎは要らなくなる。
- 割り切れる時は、ちょうどの値になる。`s ÷ k` が整数で、その掛け算が f64 でちょうど表せる時（倍率 0.5・0.25・2 など）、`G(s) = s ÷ k`。倍率 0.5 で全体の長さ 500,000（0.05 秒）の 1 周は 1,000,000（0.1 秒ちょうど）、倍率 2 で全体の長さ 1,000,000 の 1 周は 500,000 である。周回ごとに端数が削れる事は無い。
- 割り切れない時は、届く最初の刻みになる。倍率 3 で `s = 1,000,000` なら `G(s) = 333,334`（333,333 では 999,999 までしか届かない）。
- 10 進の小数の倍率（0.1・0.3 など）は、f64 がその値をちょうどは持たない。写しは、f64 の倍率と IEEE の掛け算の結果で決まる（例: 倍率 0.1 で `G(1,000,000) = 10,000,000`、倍率 0.3 で `G(300,000) = 1,000,000`・`G(1,000,000) = 3,333,334`）。f64 の掛け算の最後の 1 ビットの丸めは、写しの決まりではなく、実数の倍率が持つ精度である。
- 範囲: `e × k` が型の範囲を越えたら端へ張り付く（F は減らないまま）。G が `None` の storyboard は、1 周の長さが表せないので、コンパイルで拒む。
- 精度: 2^53（約 28 年）を越える経った長さでは f64 が整数をちょうど表せないが、F は減らないままなので、判定と予定は食い違わない。

使う所は 6 つで、全部がこのファイルの口を呼ぶ: 実効の台本の時刻（`forward`・切り捨て）・引き金の予定の時刻（`first_reaching`）・1 周の長さ（`first_reaching`。runtime とコンパイルが同じ 1 本の関数 `CompiledStoryboard::lap_length` を呼ぶ）・進み具合（`ratio_of`）・周回の間の待ち（`mul_unit`・D5 の丸め）・1 字ごとの間隔（`mul_div`・D5 の丸め）。

#### 時計（`clock.rs`）

| Field | Detail |
|-------|--------|
| Intent | 「今」を返す、ただ 1 つの関数 |
| Requirements | 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 17.6 |

```rust
/// 今の時刻。眠り（スリープ・休止）の間は進まない。プロセスごとの起点も倍率も持たない。
#[cfg(windows)]
pub fn now() -> TimePoint;
```

- 中身は `QueryUnbiasedInterruptTimePrecise()` の値をそのまま `TimePoint` にするだけ（割り算も掛け算も無い）。`u64` の値が `i64` に収まらない時（約 2 万 9 千年）だけ、端へ張り付ける。
- `unsafe` のブロックに `// SAFETY:` を書く（引数もポインタも渡さない読み取りで、前提が無い）。
- 失敗の道は無い（OS の関数が失敗を返さない）。
- `cfg(windows)` は `clock.rs`（と、その再公開の行）だけ。`dola::time` のほかの全部と、dola のほかの全部は、どの target でもコンパイルできる。
- 公開の道は `dola::time::now`（dola の頭でも `cfg(windows)` で再公開する）。`dola::runtime::clock` は消す。
- `windows` の機能: この関数は `Win32_System_WindowsProgramming` に在る。dola が QPC（`Win32_System_Performance`）を呼ぶ所は `runtime/clock.rs` だけなので、それを消した後は、dola の `Cargo.toml` から `Win32_System_Performance` を外す。段 0（足すだけ）の間は、古い時計がまだ在るので、2 つの機能が並ぶ。
- `docs.rs`: dola の `[package.metadata.docs.rs]` に、既定の target を Windows にする行を足す（`now` が文書に出るように）。

#### 文字の形（`text.rs`）と、テストの組み立て（`testing.rs`）

- `Display`: 秒の小数で、小数点の下をいつも 7 桁出す（例 `12.3456789`・`-0.0500000`）。値を落とさない。`FromStr` はその逆で、実数を介さずに整数で読む。
- `testing`（dola の今の機能 `test-support` の下）: `at_ms(i64) -> TimePoint`・`ms(u32) -> TimeSpan`・`delta_ms(i64) -> TimeDelta`。テストは、決まった時刻と長さを、作者が書く単位（ミリ秒）の値で書く。その値から型を作る 1 語の口である。本番の口ではないので、正準のキャリアの合図の組み立て（`carrier_cue`）と同じ機能の下に置き、使う crate は `[dev-dependencies]` からだけ機能を付ける（本番のコードは、ミリ秒の値から時刻を作らない。時刻は時計か刻みから来る）。

#### 直列化のアダプタ（`serde_ticks.rs`・`serde_secs.rs`）

- `serde_ticks`: cue の層の欄に付ける。100ns の整数で読み書きする。
- `serde_secs`: storyboard の文書の欄に付ける。秒の実数で読み書きし、文書の値の上限を確かめる（「文書の定義の型と直列化」の節）。

### dola の cue の層

#### cue の型と `CueSheet`

| Field | Detail |
|-------|--------|
| Intent | 台本と合図の時刻の欄を型で持ち、絶対の時刻を「起点＋相対」の整数の足し算で作る |
| Requirements | 4.1, 4.2, 4.3, 4.5, 4.6, 6.1, 6.2, 6.3, 6.4, 6.5, 6.9, 13.2, 13.7 |

```rust
pub enum BarrierKind {
    WaitForInput,
    /// 上の層へ運ぶ指令。0 と負も、そのまま運ぶ。
    WaitForChoice { timeout: Option<TimeDelta> },
}
pub struct Cue {
    pub actor: ActorKey,
    pub start_time: TimeSpan,   // 起点からの隔たり
    pub payload: CuePayload,
    pub duration: TimeSpan,     // 直列化の既定は 0
}
pub struct TalkCue {
    /// 絶対の発火時刻。取り出した時の起点＋at。
    pub fire_time: TimePoint,
    /// 起点からの隔たり（台本の時刻）。
    pub at: TimeSpan,
    pub actor: ActorKey,
    pub command: CueCommand,
    pub duration: TimeSpan,
}
impl TalkCue {
    /// 起点 origin の台本の、台本の時刻 at の合図（fire_time は origin + at）。
    pub fn scheduled(origin: TimePoint, at: TimeSpan, actor: ActorKey, command: CueCommand, duration: TimeSpan) -> Self;
}
/// 時刻の表の中の、まだ取り出していない合図の中身。発火時刻の欄を持たない。
/// 台本の時刻（at）は、時刻の表の項目の相対の時刻が持つ。
pub struct PendingCue {
    pub actor: ActorKey,
    pub command: CueCommand,
    pub duration: TimeSpan,
}
impl PendingCue {
    /// 発火時刻と台本の時刻を付けて、受け口へ渡す合図にする。
    pub fn fire(self, fire_time: TimePoint, at: TimeSpan) -> TalkCue;
}
impl CueSheet {
    pub fn new(cues: Vec<Cue>) -> Self;                         // 起点は押されていない
    pub fn with_absolute_start_time(self, origin: TimePoint) -> Self;
    pub fn absolute_start_time(&self) -> Option<TimePoint>;
    pub fn absolute_fire_time(&self, cue: &Cue) -> Option<TimePoint>;
    pub fn absolute_end_time(&self) -> Option<TimePoint>;
    /// 占有の終わりの、起点からの隔たり（全部の合図の「開始＋長さ」の最大）。
    pub fn relative_end(&self) -> TimeSpan;
}
```

- `sum_on_ms_grid` は消す。絶対の時刻は整数の足し算で、ちょうどの刻みで届く（要件 4 の 4）。
- 長さの clamp（有限でない・負を 0 にする）と、0 以上の `debug_assert` は消える（型が保証する）。並べ替えは `start_time` の安定な並べ替え。
- 本番で合図を手で組む 2 か所（`areka-runtime` の `shell_carry.rs`・`areka-seriko` の `settle.rs`）は、`TalkCue::scheduled` で組む。起点に入れる時刻は、「受け手ごとの追い付き方」の節の「手で組む合図の時刻」に書く。テストの手組みも同じ関数を通す（`fire_time` が「起点＋`at`」である事が、組み立てで保たれる）。
- 時刻の表が持つ項目は `PendingCue` で、`TalkCue` ではない。発火時刻は取り出す時の起点で決まるので、表の中に居る間は、その値がまだ無い。欄を持たない型にする事で、「表の中の合図が、置き直しの前の起点の発火時刻を持っている」状態を作れなくする。`PendingCue` から `TalkCue` を組むのは `fire` の 1 本で、再生機がそれを呼ぶ所は、取り出しと先渡しの 2 つだけである（「`CuePlayer` と `CueSink`」の節）。
- `carrier_cue`（機能 `test-support`）は、呼び方を変えずに新しい型で組む（起点 0・`at` 0・長さ 0）。

**直列化の形**（JSON の例。TOML も同じ欄の名前と整数）:

```json
{"absolute_start_time": 420000000, "cues": [
  {"actor": "sakura", "start_time": 0, "payload": {"Command": {"Text": "hi"}}, "duration": 5000000},
  {"actor": "sakura", "start_time": 5000000, "payload": {"Barrier": {"WaitForChoice": {"timeout": -10000}}}, "duration": 0}
]}
```

- 値は 100ns の整数。`absolute_start_time` と `timeout` は、無い時は欄ごと書かない（読む時は、欄が無い・`null` のどちらも「無い」）。
- 読み込みは、整数でない値・範囲の外の値・負の `start_time`／`duration` を失敗にする。`timeout` は負も 0 も読む。
- 付け替えの前の形（実数の秒）を読む口は無い。

#### `TimedSchedule`

| Field | Detail |
|-------|--------|
| Intent | 起点と相対の時刻から、届いた項目を 1 件ずつ取り出す。起点を置き直す |
| Requirements | 4.1, 4.4, 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7, 11.8, 11.9, 11.10, 11.11, 11.12, 13.1, 13.3, 13.4, 13.5, 13.6 |

**Contracts**: Service [x] / State [x]

```rust
pub enum Entry<T> { Payload(TimeSpan, T), Barrier(TimeSpan, BarrierKind) }

/// 取り出した 1 件。
pub enum Due<T> {
    /// 中身と、取り出した時の起点で決めた絶対の発火時刻と、項目の相対の時刻（台本の時刻）。
    Payload { fire_time: TimePoint, at: TimeSpan, payload: T },
    /// 区切りに着いた（時刻の表は、外から解かれるまで止まる）。
    Barrier(BarrierKind),
}

/// 起点の知らせ。「effective_at から先、台本の時刻 0 は origin である」。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginNotice {
    /// この起点が効き始める時刻。置き直しでは、呼び手が渡した現在の時刻。最初の起点では、その起点。
    pub effective_at: TimePoint,
    /// 新しい起点。
    pub origin: TimePoint,
    /// 前の起点。最初の起点の知らせでは None。
    pub previous: Option<TimePoint>,
}
impl OriginNotice {
    /// 最初の起点の知らせ（effective_at は origin・previous は None）。
    pub fn first(origin: TimePoint) -> Self;
    /// 動いた長さ（origin − previous）。負は前へ（台本の時刻は進む）。最初の起点では None。
    pub fn delta(&self) -> Option<TimeDelta>;
}

impl<T> TimedSchedule<T> {
    pub fn with_horizon(origin: TimePoint, horizon: TimeSpan) -> Self;
    pub fn insert_each(&mut self, entries: impl IntoIterator<Item = Entry<T>>);
    /// now までに届いた次の 1 件。届いた物が無い・区切りで止まっている時は None。
    pub fn pop_due(&mut self, now: TimePoint) -> Option<Due<T>>;
    /// 「時刻 at が、台本の時刻 script_time である」ように起点を置く。now は呼び手の現在の時刻で、
    /// 知らせの効く時刻になる。起点が変わらなければ None。
    pub fn reanchor(&mut self, now: TimePoint, at: TimePoint, script_time: TimeSpan) -> Option<OriginNotice>;
    /// 起点を delta だけ動かす。now と戻り値は reanchor と同じ。
    pub fn shift_origin(&mut self, now: TimePoint, delta: TimeDelta) -> Option<OriginNotice>;
    pub fn origin(&self) -> TimePoint;
    pub fn current_barrier(&self) -> Option<&BarrierKind>;
    pub fn notify_barrier_resolved(&mut self);
    pub fn remaining(&self) -> usize;
    /// 全部を取り出し、区切りが無く、now が占有の終わりに届いている。
    pub fn is_completed_at(&self, now: TimePoint) -> bool;
    /// 起点＋占有の終わり。起点を置き直すと動く。
    pub fn occupancy_horizon(&self) -> TimePoint;
}
```

- **届いたかの判定**: 「起点＋相対 ≤ now」。引き算で相対へ戻さない。起点は中で 128 ビットの整数で持ち、比べも 128 ビットで行う。だから、起点が `TimePoint` の下の端より前になっても、比べの結果は端が無い時と同じである。
- **型の下の端（要件 11 の 12）**: 要件 11 の 12 は、届き方（どの項目が・どの順で・どの刻みで届くか）について満たす。`origin()` と `fire_time` は `TimePoint` で返すので、値が `i64` の範囲の外に在る時だけ、端へ張り付く。その時は `fire_time − at` が中の起点と合わない。張り付くのは、起点か発火時刻が `i64` の端（時計の 0 から約 2 万 9 千年）を越えた時だけで、時計の値（0 以上で、2^63 に届かない）と、そこからの台本の長さの範囲の置き直しでは起きない。
- **発火時刻**: `pop_due` が、取り出した時の起点＋相対で決める。刻みの時刻ではない。決めた値は、後の置き直しで変わらない（取り出した物は、もう表の中に無い）。
- **溜めを持たない**: 取り出した物は返して終わり。同じ時刻でもう一度呼ぶと、新しく届いた物だけが返る（置き直しが無ければ `None`、起点が前へ動いていれば、新しく届いた項目）。同じ時刻の刻み直しの守りと、区切りを解く時に溜めを空にする処理は、消える。
- **置き直し**: まだ取り出していない項目は、次の `pop_due` から新しい起点で判定される。項目は動かさないので、落ちない・2 度届かない・順は変わらない。区切りで止まっている間も、起点は動く。
- **置き直しは、絶対の時刻の上の出来事である**: 置き直しの口は、呼び手の現在の時刻 `now` を受け、それを知らせの効く時刻（`effective_at`）にして返す。時刻の表は `now` を使って何も決めず、覚えもしない（知らせに写すだけ）。`reanchor` の `at` は、起点を決めるための組（「時刻 `at` が台本の時刻 `script_time` である」）で、`now` と同じでなくてよい（音の終わった時刻で置き直す時など）。
- **起点が変わらない置き直し**: 新しい起点が今の起点と等しければ、何も変えず `None` を返す。知らせは出ない。
- **知らせの中身**: 効く時刻・新しい起点・前の起点。動いた長さは、2 つの起点の差で求まるので、欄にしない（`delta()` が返す）。起点は中で 128 ビットで持つので、`origin` と `previous` は、`i64` の範囲の外では端へ張り付いた値になる（上の「型の下の端」）。
- **呼び手の時刻は戻らない前提**: 置き直しに渡す `now` は、その前の `pop_due` に渡した時刻以上とする（呼び手の現在の時刻だから）。そうなら、置き直しの前に取り出した合図の発火時刻は、どれも効く時刻以前である。時刻の表は時刻を覚えないので、これを確かめない。受け手の決まり（「知らせの受け方」の節）は、この前提が破れても、届いた順と時刻だけで結果が決まる。
- **完了**: 状態として持たず、`is_completed_at(now)` が、その時の起点で答える。
- **起点より前の今**: 最初の項目の「起点＋相対」に今が届くまで、何も返らない。0 に寄せない。
- **まだ取り出していない項目の照会**: crate の中の口が、配る順に「その時の起点での予定の発火時刻・相対の時刻・中身」を返す（先渡しが使う）。
- **台本からの変換**: `to_talk_schedule(&CueSheet) -> Result<TimedSchedule<PendingCue>, CueError>`。項目の相対の時刻が台本の時刻（`at`）で、中身は発火時刻を持たない `PendingCue` である。起点の押されていない台本は `Err`。
- **前半と後半**: 前半は、今の `tick`＋`ready` の口のまま型だけを替える（表の項目は `TalkCue` で、`fire_time` は台本の起点＋`at`。前半には起点を動かす口が無いので、この値は古くならない）。この節の口（`pop_due`・`reanchor`・`shift_origin`・`is_completed_at`）へ替えるのと、表の項目を `PendingCue` へ替えるのは、後半の同じ時である（起点を動かす口と、発火時刻を持たない項目の型は、一緒に入る）。

#### `CuePlayer` と `CueSink`

| Field | Detail |
|-------|--------|
| Intent | 取り出した合図に発火時刻を付けて全部の受け口へ配り、起点（最初の起点と置き直し）を知らせる |
| Requirements | 2.8, 11.1, 11.6, 11.7, 12.1, 12.2, 12.3, 13.2, 13.3, 13.7, 18.1, 18.3, 18.4 |

**Contracts**: Service [x] / Event [x]

```rust
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CueError {
    /// 台本の起点が押されていない。
    #[error("cue sheet has no absolute start time")]
    OriginNotStamped,
}

/// step の結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepOutcome { Delivered, Barrier, Idle, Completed }

impl CuePlayer {
    pub fn from_sheet(sheet: &CueSheet) -> Result<Self, CueError>;
    /// 受け口を登録する。登録の時に、その受け口へ最初の起点と先渡しを渡す。
    pub fn register_sink(&mut self, sink: Box<dyn CueSink>);
    /// 届いた項目を 1 件だけ進める。
    pub fn step(&mut self, now: TimePoint) -> StepOutcome;
    /// 届いた項目を全部進める（step を Delivered の間くり返す）。
    pub fn tick(&mut self, now: TimePoint);
    /// 起点を置き直し、起点が変わったら全部の受け口へ知らせる。now は呼び手の現在の時刻（知らせの効く時刻）。
    /// 完了の後と、起点が変わらない時は、何もせず None。
    pub fn reanchor(&mut self, now: TimePoint, at: TimePoint, script_time: TimeSpan) -> Option<OriginNotice>;
    pub fn shift_origin(&mut self, now: TimePoint, delta: TimeDelta) -> Option<OriginNotice>;
    pub fn origin(&self) -> TimePoint;
    pub fn occupancy_horizon(&self) -> TimePoint;
    /// 入力待ちの区切りを外す。時刻を受けず、完了も決めない。
    pub fn resolve_click(&mut self);
    /// 選択待ちの区切りを外す（選択肢の溜めを空にする）。時刻を受けず、完了も決めない。
    pub fn resolve_choice(&mut self, choice_id: &str) -> Option<String>;
    // announce_choice_wait・state・is_completed・pending_choices・current_barrier は、口の形を変えない
}

pub trait CueSink {
    fn emit(&mut self, cue: TalkCue);
    fn emit_with(&mut self, cue: TalkCue, probe: &CueProbe) { self.emit(cue) }
    fn preview(&mut self, upcoming: &[TalkCue]) {}
    fn choice_wait_began(&mut self) {}
    /// 起点の知らせ（最初の起点と、置き直し）。既定は何もしない。
    fn origin_set(&mut self, notice: OriginNotice) {}
}
```

- **`step`**: 待ちの状態と完了の後は進まない。`pop_due` が中身（`PendingCue`）と発火時刻と相対の時刻を返したら、`PendingCue::fire` で `TalkCue` を組み、選択肢なら `pending_choices` へ積み、全部の受け口へ登録順に `emit` して `Delivered`。区切りなら待ちの状態へ移って `Barrier`。何も届いていなければ、完了を確かめて `Completed` か `Idle`。時刻に依る事（どれが届いたか・完了したか）を決めるのは、この口だけである。
- **最初の起点**: `register_sink` は、登録する受け口へ、まず `origin_set(OriginNotice::first(その時の起点))` を、次に先渡し（`preview`）を渡してから、登録の列へ足す。`emit` は登録済みの受け口にしか行かないので、最初の起点は、その受け口へのどの合図よりも前に着く。本番の呼び手（`drive.rs`）は、最初の刻みで台本の起点を押し、再生機を作り、受け口を登録してから `tick` を呼ぶ。だから、登録が再生の始まりである。後から登録された受け口は、登録の時の起点（置き直しの後なら、置き直した後の起点）を、最初の起点として受ける。
- **最初の起点の知らせは、起点を知らせるだけの物である**: 置き直しではない（前の起点が無い）。欄 `effective_at` には、起点そのものを入れる（「台本の時刻 0 から先、この起点である」）。起点を使う受け口（後の spec の、流れを進める所など）は、合図を待たずに起点を知れる。areka の受け手（文字の層・seriko・バルーンの寿命の時計）は、絶対の時刻で持つので、この知らせでは何もしない。どの受け手も、この知らせの効く時刻を待って、何かを止める事をしない（「知らせの受け方」の節）。
- **順**: `emit` と `origin_set` は、呼ばれたその場で、登録順に全部の受け口へ配る。溜めない。だから「最初の起点 → 置き直しの前に取り出した合図 → 置き直しの知らせ → 置き直しの後に取り出した合図」の順が、受け口に着く順になる。
- **置き直し**: `reanchor`／`shift_origin` は、呼び手の `now` をそのまま時刻の表へ渡し、返った `OriginNotice` を全部の受け口へ配って、同じ物を返す。起点が変わらなければ、何も配らない。再生機は `now` を覚えない。
- **区切りを解く口は、時刻に依らない**: `resolve_click`・`resolve_choice` は、時刻を受けず、時刻に依る事を何も決めない。する事は、区切りを外して再生中の状態へ戻す事（`resolve_choice` は、選択肢の溜めを空にする事も）だけである。区切りの後ろの項目を配る事と、完了は、呼び手が次に呼ぶ `step`／`tick(now)` が決める。解いた直後に「もう占有の終わりを過ぎていたら完了にする」処理（今の `settle_completion_after_resolve`。時刻の表が覚えた時刻で完了を確かめる）は、無くなる。時刻の表は時刻を覚えないので、解く口には、完了を決める材料が無い。
  - 本番の呼び手（`areka-sakura` の `drive.rs` の選択の解決）は、解いた直後に、最後に受けた刻みの時刻のまま `player.tick` を呼び、その後で完了を見る。だから、解いてから完了までに、新しい刻みを待たない（1 刻みの遅れは生まれない）。
  - 解いた直後の `is_completed()` は偽で、同じ時刻の `tick` の後に真になる。解く口だけを呼んで完了を期待していたテストは、解く → `tick` の形へ直す（変える前後の期待として記録に書く）。
- **`TalkCue` を組む所**: 時刻の表の中の項目は `PendingCue` で、発火時刻を持たない。再生機が `TalkCue` を組むのは、取り出し（`step`）と先渡し（`register_sink`）の 2 つの出口だけで、どちらも `PendingCue::fire` に、その時の起点＋相対の時刻を渡す。表の中に「発火時刻の古い合図」は在りえない。
- **先渡し**: `register_sink` は、まだ取り出していない合図を、その時の起点での予定の `fire_time` を付けて渡す。決まった値は `emit` で届く。起点が置き直されなければ、2 つは等しい。先渡しの後に起点が置き直されると、同じ合図が `emit` で届く時の `fire_time` は、先渡しで見せた値と違う。先渡しの値は、後の置き直しで改まりうる予定の値であり、決まった値は `emit` の値だけである。受け口は、先渡しの `fire_time` を時刻の計算に使わない（文字の層の空回しは、時刻を使わない）。この事を確かめるテストを置く（先渡し → 置き直し → 取り出しで、`emit` の `fire_time` が新しい起点＋`at` であり、先渡しの値と置き直しの分だけ違う事）。
- **包む受け口**: ほかの受け口を包む受け口（`areka-sakura` の `StrictTap`）は、`origin_set` を中へ通す。通さないと、既定の「何もしない」に落ちて、中の受け口へ知らせが着かない。通る事のテストを置く。
- **完了の後の置き直し**: 何もしない。全部の合図を配り、占有の終わりも過ぎた台本に、動かす物は無い。
- **台本の起点が押されていない**: `from_sheet` は `Err(CueError::OriginNotStamped)`。本番の呼び手（`drive.rs`）は、直前に起点を押すので起きない。起きたら `error!` を出して、その台詞を中断として終える。
- **前半と後半**: 前半は、`origin_set` の口と最初の起点を配る事を入れない（受け手は見積もりのまま）。入れるのは後半で、置き直しの口・解く口の直しと同じ時である。

### dola の storyboard の層

#### 文書の定義の型と直列化

| Field | Detail |
|-------|--------|
| Intent | 作者が秒で書く値を、読む縁で 1 回だけ型へ変えて持つ |
| Requirements | 5.3, 5.9, 5.10, 6.6, 6.7, 6.8 |

| 欄 | 型 | 文書の字面 |
|---|---|---|
| `TransitionDef.delay` | `TimeSpan` | 秒（既定 0） |
| `TransitionDef.duration` | `Option<TimeSpan>` | 秒 |
| `LoopOffset::Scalar` | `TimeSpan` | 秒 |
| `LoopOffsetRange.min`・`max` | `TimeSpan` | 秒 |
| `KeyframeRef::WithOffset.offset` | `TimeDelta` | 秒（負も書ける） |
| `StoryboardEntry.trigger_start_offset` | `Option<TimeDelta>` | 秒（負も書ける） |
| `Storyboard.time_scale` | `f64` | 実数のまま（次元が無い） |

- 読み書きは `dola::time` の `serde_secs` のアダプタを欄に付ける。読む時は `try_from_secs_f64`。書く時は、アダプタが `as_ticks()` の整数を 10,000,000 で割って秒の実数にする（アダプタの中だけの計算で、公開の口にしない。時刻の土台の型は、秒の実数を返す口を持たない）。
- 秒で読み、書き出し、もう一度読むと、1 度目と 2 度目で同じ値になる。100ns の整数を秒の実数で書いた値は、最も近い 100ns へ丸めると元の整数に戻る。これは絶対値が 2^51（100ns の数。約 7 年）より小さい値でいつも成り立つ。だから、storyboard の文書の時間の値は、この範囲の中だけとする（文書の値の上限）。
- **文書の値の上限は、文書の作り方に依らずに守る**。上限を確かめる関数は `serde_secs` の 1 本で、次の 3 か所が呼ぶ。
  - 読む時: 越える値は、読み込みの失敗。
  - 検査（`validate`）: 文書の定義の型の時間の欄の全部を見て、越える値を `DolaError::TimeOutOfDocumentRange { location, field }` で拒む。`DolaDocumentBuilder::build` と `compile_storyboard` は検査を通すので、`StoryboardBuilder` で型から作った文書も、書き出す前・コンパイルの前に拒まれる。
  - 書く時: 越える値は、書き出しの失敗（`serde` の誤り）。欄を直に組んで検査を通さずに書く道でも、読み戻せない文書は出来ない。
- 往復のテストで、上限の端（2^51 − 1 は往復する・2^51 とその先は、読む・検査・書くの 3 つとも失敗。符号つきの欄は、負の側も同じ）を確かめる。
- 有限でない値・負の長さ・範囲の外は、読み込みの失敗になる。検査の決まり（`validate/rules.rs`）のうち「長さが有限で 0 以上」は、型が保証するので消える。`DolaError::LoopOffsetNegativeMin`／`Max` も消える。`LoopOffsetRangeInverted { min, max }` は `TimeSpan` で残る。消した決まりと、それを確かめていたテストの行き先（読み込みの失敗のテストへ）は、記録に書く。
- `StoryboardBuilder` と文書を字面で組むテストは、型で書く。

#### コンパイルの結果

| Field | Detail |
|-------|--------|
| Intent | 区間と引き金の時刻を、storyboard の頭からの隔たり（台本の軸の符号つきの差）で持つ。開始の時刻に依らない |
| Requirements | 5.2, 5.3, 5.4, 5.10 |

| 欄 | 型 |
|---|---|
| `CompiledStoryboard.time_scale` | `TimeScale` |
| `CompiledStoryboard.total_base_duration` | `TimeSpan` |
| `CompiledVariableTimeline.base_duration` | `TimeDelta`（最後の区間の終わり。頭からの隔たり） |
| `CompiledSegment.start_time`・`end_time` | `TimeDelta`（頭からの隔たり。負のオフセットで、頭より前に置ける） |
| `CompiledTrigger.fire_time` | `TimeDelta`（頭からの隔たり） |
| `CompiledTrigger.start_offset` | `Option<TimeDelta>` |
| `compile_storyboard(doc, name)` | 開始の時刻の引数は無い |

- **コンパイルの結果は、開始の時刻に依らない**: `compile_storyboard` は開始の時刻を受けず、`CompiledStoryboard.start_time` の欄は無い。区間と引き金の時刻は、どれも storyboard の頭（台本の時刻 0）からの隔たりである。絶対の時刻が storyboard の層へ入る所は、`DolaRuntime::start(name, start_time: TimePoint)` の 1 つだけで、絶対の時刻は、いつも「起点（周回の始まり＋止めていた長さ）＋隔たり」で作る。cue の層の `CueSheet`（起点＋相対）と同じ形である。
  - 理由: コンパイルの結果に絶対の時刻を焼くと、止めて戻すたび・周回のたびに、区間と引き金の全部を書き直す事になる。倍率が 1 でない時は、台本の軸と絶対の軸の比が 1 でないので、ずらすだけでは足りない。「起点＋隔たり」なら、動くのは起点だけで、1 つのコンパイルの結果が、どの開始の時刻にも・どの周回にも使える。
  - 中の作り: 予約の名前 `start` のキーフレームは、隔たり 0 である。runtime は、今も開始の時刻 0 でコンパイルしているので、再生の結果は変わらない。
  - 説明の直し: `src/compile/types.rs` が、区間と引き金の時刻を「絶対時刻」と書いている所を、「storyboard の頭からの隔たり」へ直す。
  - テスト: `compile_storyboard` を呼ぶテストは、引数を 1 つ外す。0 でない開始の時刻を渡していた 5 か所は、次のとおりにする（外す物は、理由と一緒に記録に書く）。

    | テスト | 今の確かめ | 行き先 |
    |---|---|---|
    | `tests/compile/boundary_test.rs` の、開始の時刻に NaN を渡す所 | NaN の開始の時刻で、全部の項目が誤りになる | 外す。入力（開始の時刻）そのものが無くなる。表せない入力だけを確かめていた物として記録する |
    | `tests/compile/time_resolution_test/at_placement.rs` の、開始の時刻 5 秒 | キーフレームを指す `at` の置き場所 | 同じ確かめを、頭からの隔たりで書き直す（期待の時刻から 5 秒を引いた値） |
    | `tests/compile/time_resolution_test/sequential.rs` の、開始の時刻 10 秒 | 遅れの在る区間の始まりと終わり | 同じ確かめを、頭からの隔たりで書き直す（0.5 秒から 1.5 秒）。同じファイルに同じ確かめが在れば、外して記録する |
    | 同じファイルの、開始の時刻 3 秒 | 始めの値を初期値から推す区間の時刻と、`result.start_time` | 区間の確かめは、頭からの隔たりで書き直す。`result.start_time` の確かめは、欄と一緒に外して記録する |
    | `tests/trigger/compile_test.rs` の `trigger_fire_time_with_start_time_offset` | 引き金の発火の時刻に、開始の時刻が足される事 | 外す。確かめていた事（開始の時刻を足す事）が無くなる。頭からの発火の時刻は、同じファイルのほかのテストが確かめている事を、外す前に確かめる |

- 足し算は `checked_add` で、越えたら `DolaError`（今の「和が無限大になったら拒む」と同じ所）。
- 1 周の長さが表せない（`first_reaching` が `None`）storyboard は、`DolaError::LoopDurationOutOfRange { location }` で拒む。1 周の長さを求める関数は 1 本（`CompiledStoryboard::lap_length(&self) -> Option<TimeSpan>`＝`time_scale.first_reaching(total_base_duration)`。`src/compile/types.rs` に置く）で、コンパイル（表せるかの確かめ）と runtime（開始の時の 1 周の長さ）の両方が呼ぶ。全体の長さが 0 以上なので、返る値は 0 以上である（「倍率の写し」の節）。
- 並べ替えは整数の比べ。

#### `DolaRuntime`

| Field | Detail |
|-------|--------|
| Intent | 開始・刻み・止める・戻す・終わりの期限を、時刻の型で受け、倍率の写しを通して判定する |
| Requirements | 2.8, 5.2, 5.4, 5.5, 5.6, 5.7, 5.8, 5.11, 10.1, 10.2, 10.3, 10.4, 10.5 |

**Contracts**: Service [x] / State [x]

```rust
impl DolaRuntime {
    pub fn start(&mut self, name: &str, start_time: TimePoint) -> Result<StartResult, RuntimeError>;
    pub fn pause(&mut self, group_id: u64, now: TimePoint) -> Result<(), RuntimeError>;
    /// 戻した後の、再生の全体の終わりの時刻。決まっていなければ None（StartResult.end_time と同じ意味）。
    pub fn resume(&mut self, group_id: u64, now: TimePoint) -> Result<Option<TimePoint>, RuntimeError>;
    pub fn finish(&mut self, group_id: u64, deadline: TimePoint) -> Result<(), RuntimeError>;
    pub fn tick(&mut self, now: TimePoint);
}
pub struct StartResult {
    pub group_id: u64,
    /// 再生の全体の終わりの時刻。その時に決まっていなければ None
    /// （終わらないループ・周回の間の待ちを、これから周回ごとに引くループ）。
    pub end_time: Option<TimePoint>,
    pub affected_group_ids: Vec<u64>,
}
// RuntimeError::TooShortDurationWithInfiniteLoop { storyboard: String, duration: TimeSpan }
// MIN_LOOP_DURATION: TimeSpan = 1,000,000（0.1 秒ちょうど）
```

**中の状態**（`StoryboardInstance`）:

| 欄 | 型 | 意味 |
|---|---|---|
| `loop_start` | `TimePoint` | 今の周回の始まり |
| `pause_accumulated` | `TimeDelta` | 止めていた長さの和（符号つき） |
| `lap_length` | `TimeSpan` | 1 周の経った長さ `G(全体の長さ)`（`CompiledStoryboard::lap_length` の値。0 以上） |
| `time_scale` | `TimeScale` | |
| `finish_deadline` | `Option<TimePoint>` | |
| `state` | `InstanceState` | `Paused { since: TimePoint }` が、止めた時刻を持つ |

- 周回の終わりの時刻は、欄として積まずに `loop_start + pause_accumulated + lap_length` で求める。周回を数えるための中の値で、公開の口には出さない。
- **終わりの予定の時刻（`StartResult.end_time` と `resume` の戻り値）の意味は 1 つ**: その時に決まっているなら、再生の全体の終わりの時刻（最後の周回の終わり）。決まっていなければ `None`。
  - 決まっている時: 1 回だけの再生（今の周回の終わり）。周回の間の待ちが無い有限のループ（残りの周回の数 × 1 周の長さを足した、最後の周回の終わり）。周回の間の待ちが決まった値の有限のループ（下限と上限が等しい。残りの待ちも足す）。待ちを引くループでも、最後の周回に入った後（もう引く待ちが無い）。
  - 決まっていない時: 終わらないループ。周回の間の待ちを周回ごとに乱数で引くループで、まだ引いていない待ちが残っている時（待ちは、周回が終わるたびに引く。`LoopOffset::Scalar(v)` は 0 から v までの幅なので、v が 0 でなければ、ここに入る）。求める和が型の範囲を越える時（回り込ませない。その再生は、延ばせなくなった周回で終わる）。
  - 止めている間は、戻す時刻がまだ無いので、終わりの時刻を言えるのは `resume` の戻り値からである。
  - 終わりの期限（`finish`）が置いてある時の `resume` の戻り値は、期限と、上の「決まっている時」の終わりの時刻の、早い方である（終わりの時刻が決まっていなければ、期限）。再生は、どちらか早い方で終わるからである。終わらないループも、期限を置いた後は、終わりの時刻が決まっている。`StartResult.end_time` は、開始の時には期限がまだ無いので、期限を含まない。
  - 欄の意味を、ループの種類で「全体の終わり」と「今の周回の終わり」に分けない。同じ欄を受け取った側が、ループの種類を知らずに読める。
- **終わらないループの長さの確かめ**: 長さ 0 と下限は、どちらも 1 周の長さ `lap_length`（`G(全体の長さ)`）で比べる。`lap_length` が 0 なら `ZeroDurationWithLoop`（全体の長さが 0 の時だけ）、`lap_length < MIN_LOOP_DURATION`（1,000,000）なら `TooShortDurationWithInfiniteLoop`。`lap_length` がちょうど 1,000,000 なら始まる（例: 倍率 0.5 で全体の長さ 500,000。全体の長さ 499,999 なら 1 周は 999,998 で、拒む）。比べは整数で、境は 1 刻みの単位でちょうどである。
- **経った長さ**: `e = (止まっていれば since、そうでなければ now) − loop_start − pause_accumulated`。**台本の時刻**: `F(e)`。区間の境・引き金・周回の終わりの判定は、全部この 2 つを通す。
- **引き金の予定の時刻**: `loop_start + pause_accumulated + G(fire_time)`。周回の中か外かは、台本の軸で比べる（`fire_time ≤ total_base_duration`）。G の値は、開始の時に引き金ごとに求めて持つ。
- **周回を進める**: 次の周回の始まりは `周回の終わり + 待ち − pause_accumulated`。`checked_add` が `None`（型の範囲を越える）なら、回り込ませずに、その周回で終える（今の「延ばせなくなったら周回を止める」と同じ決まり）。長さ 0・待ち無しの有限のループも、今と同じにその場で終える。
- **1 つ前の時刻**: 出来事の時刻 t の「前に終わる周回まで進める」は、`t` から 1（100ns）を引いた時刻で行う。
- **周回の後の待ち**: 台本の時刻が負で、周回を 1 回以上終えている間は、最後の値を保つ。中の列挙 `ValueTime { At(TimeDelta), AfterLastSegment }` で表し、無限大の番兵は使わない。
- **進み具合**: `(台本の時刻 − 区間の始まり).ratio_of(区間の終わり − 区間の始まり)`。

**止めて戻す**:

- `InstanceState::Paused { since }` が止めた時刻を持つ。「止まっているのに記録が無い」「再生中なのに記録が残っている」は、型で作れない。
- `transition` は、`Paused` への遷移と、`Paused` から `Playing` への遷移を受け付けない（`Err`）。止めるのは `pause`、戻すのは `resume` だけ。`Paused` から終わりの状態へは、今までどおり移れる。
- `resume(now)`: `pause_accumulated += now − since`（符号つき）。戻した直後の経った長さは、止めた時点の値と等しい。`now` が `since` より前でも同じ式で、終わりの時刻は差のとおりに前へ動く。
- 止めている間は、どの時刻で評価しても、止めた時点の値を返す（経った長さを `since` で求めるので、今に依らない）。
- **終わりの期限（`finish`）は、止めている間も、その時刻に効く**。終わりの期限は「絶対の時刻 `deadline` に、この再生を終わりの値で終える」という指令で、再生の台本の時刻（経った長さ）から求める物ではない。止める事が止めるのは台本の時刻の進みで、絶対の時刻の指令は止まらない。だから、止めている間に `deadline` が来たら、その刻みで再生を終え、値は終わりの値になる。これは要件 10 の 2 が除く場合である（要件の字面に、この除く場合を書く）。期限が来るまでは、止めた時点の値のままである。

### 使い手

#### wintf

| Field | Detail |
|-------|--------|
| Intent | フレームの時刻・タイプライター・自動の上げ下げの判断を、時刻の型で持つ |
| Requirements | 7.5, 7.12, 8.1, 8.3, 8.4, 8.5 |

- `FrameTime(pub TimePoint)`。置く所は今と同じ 2 か所（世界を作る時・進める回の頭）で、`dola::time::now()` を読む。隣の `TickStart(Instant)` は範囲の外で、触らない。
- `DolaAnimator::tick(TimePoint)`。
- タイプライター: `TypewriterTalk::new(tokens, start_time: TimePoint)`・`pause`／`resume`／`update` は `TimePoint`・`start_time() -> TimePoint`。行程の項目の時刻（`show_at`・`start_at`・`fire_at`）は開始からの `TimeSpan`、`duration` は `TimeSpan`。開始より前の刻みは、経った長さが負（`TimeDelta`）で、何も現さない。全部を現す `skip` は、無限大を渡さずに、行程の終わりまで進める口で書く。
- 作者が書く値（`TypewriterToken::Wait(f64)` の秒・`Typewriter.default_char_wait: f64` の秒）は秒のまま受け、行程を組む縁で 1 回だけ `TimeSpan` へ変える。変えられない値（有限でない・負）は、長さ 0 として扱い、`warn!` を出す（今は検査が無く、負は行程の時刻を戻し、NaN は後ろの全部を壊す。この入力の扱いの変化は、表せない入力として記録に書く）。
- 自動の上げ下げ: `RuleState` の `since`・`last_now` は `Option<TimePoint>`、保つ長さの定数は `TimeSpan`。

#### areka-talk・areka-sakura

| Field | Detail |
|-------|--------|
| Intent | 選択待ちの知らせ・刻み・作者の単位の縁・1 字ごとの間隔・届いた物を時刻が来るまで持つ列 |
| Requirements | 4.6, 7.4, 7.9, 7.10, 11.9, 11.10, 12.4, 12.9, 12.10, 14.1, 14.3, 14.4, 14.5, 14.6, 17.8, 18.1, 18.2, 18.4 |

```rust
// areka-talk
pub struct ChoiceWaiting {
    pub talk_id: TalkId,
    pub choice_ids: Vec<String>,
    /// 表示の終わりの絶対の時刻（占有の終わり）。
    pub display_end: TimePoint,
    /// 時間切れの指令。無い・0 と負・正を、kanade が読み分ける。
    pub timeout_directive: Option<TimeDelta>,
}

// areka-sakura
pub enum SakuraMsg { /* … */ Tick(TimePoint), /* … */ }
pub fn text_playback_duration(text: &str) -> TimeSpan;          // duration.rs
pub enum ChoiceTimeoutDirective { /* … */ Delta(TimeDelta) }    // 今の Secs(f64) の置き換え

// areka-sakura/src/reveal.rs
/// 1 塊の字の現れる時刻（塊の頭からの隔たり）を、字の順に返す。
/// prev_since_chunk_start は、前の字の現れる時刻 − この塊の頭（前の字が無ければ None）。
pub fn chunk_reveal_offsets(
    prev_since_chunk_start: Option<TimeDelta>,
    duration: TimeSpan,
    count: usize,
) -> impl Iterator<Item = TimeSpan>;

// areka-sakura/src/held.rs
/// フレームで評価する受け手が、届いた物を、届いた順のまま、時刻が来るまで持つ列。
/// 起点も台本の時刻も持たない。
pub struct HeldQueue<T> { /* … */ }
impl<T> HeldQueue<T> {
    pub fn new() -> Self;
    /// 列の後ろへ足す。at は、その物が効く絶対の時刻。None は「自分の時刻を持たない」（前の物に続く）。
    pub fn push(&mut self, at: Option<TimePoint>, item: T);
    /// 列の頭の物の時刻が来ていれば（at が frame_now 以前、または at が None）、取り出す。
    /// 来ていなければ None（後ろの物を先に出さない）。
    pub fn pop_due(&mut self, frame_now: TimePoint) -> Option<T>;
    /// 列の頭の物が待っている時刻（列が空か、頭の物が時刻を持たなければ None）。
    pub fn waiting_until(&self) -> Option<TimePoint>;
    pub fn is_empty(&self) -> bool;
}
```

- **作者の単位の縁**: `duration.rs` の `ms_to_secs` は、`TimeSpan` を返す 1 本に替わる。`Instruction::Wait(Duration)` は `TimeSpan::try_from(Duration)` でちょうど変える（1 ms 未満を切り捨てる継ぎは消す）。選択待ちの時間切れは `TimeDelta::from_millis_saturating`。
- **1 字ごとの間隔**: 今の式は、字ごとに `r = max(前の字の時刻 + 間隔, 塊の頭)`（前の字が無ければ塊の頭）で、間隔は `duration ÷ count` の実数である（文字の層の `extend_chunk` と seriko の `push_chunk`）。塊の中の 2 字目から後ろは `max` が効かないので、塊ごとに、次の 2 つのどちらかになる。新しい式は、その 2 つを、字ごとに 1 回だけ丸めて求める（`count` 字・0 から数えて i 番目・`p` は `prev_since_chunk_start`）。
  - **頭から始まる塊**（前の字が無い、または `p × count + duration < 0`＝前の字に間隔を足しても塊の頭に届かない）: 隔たりは `duration.mul_div(i, count)`。最初の字が、塊の頭ちょうどである。
  - **前の字から続く塊**（`p × count + duration ≥ 0`）: 隔たりは `p + duration − duration.mul_div(count − 1 − i, count)`。最後の字が、`p + duration` ちょうどである。
  - どちらに当たるかは、整数の比べ（`p × count + duration` の符号。途中は 128 ビット）で決める。実数の間隔を作らない。
  - どちらも、字の時刻は「ちょうどの整数（塊の頭、または `p + duration`）± 有理数 `duration × n ÷ count` を 1 回だけ丸めた値」である。丸めは `mul_div` の 1 回で、足し込まないので、字数が増えても端数は積もらない。
  - **今の式（誤差の無い実数で計算した値）からの差は、どの字も 100ns 未満である**。頭から始まる塊は、丸めの 1 回分で、差は `(−50ns, +50ns]`。続く塊は、それに `p`（前の字の時刻）が持つ差が足される。続く塊の最後の字は `p + duration` ちょうどなので、塊を何個続けても、`p` が持つ差は、元の「頭から始まる塊」の最後の字の差 `(−50ns, +50ns]` のまま増えない。続く塊の字は終わりの側から測るので、その丸めの差は `[−50ns, +50ns)` になる。2 つを足した差は `(−100ns, +100ns)` に収まる（ちょうど中間の丸めが 2 つ、同じ向きに重ならない）。前の字の差のために、整数と実数で塊の分かれ（頭から・続く）が違う時（実数の式で、前の字＋間隔が塊の頭から 50ns 未満の所に在る時だけ）も、字の差は同じ幅に収まり、その塊の最後の字の差は `(−50ns, +50ns]` の中に残る。
  - 例（割り切れない長さ）: 長さ 1,000,000（100 ms）・3 字。頭から始まる塊の隔たりは 0・333,333・666,667（実数は 0・333,333.3…・666,666.6…）。次の塊の頭が 1,000,000 後ろで、同じ長さと字数なら、`p = −333,333` で続く塊になり、隔たりは 0・333,334・666,667（差は 0・+67ns・+33ns）。
  - 例（ちょうど中間）: 長さ 10,000・32 字（間隔 312.5）。頭から始まる塊の最後の字は 9,688（実数は 9,687.5）。次の塊の頭が 10,000 後ろで、同じ長さと字数なら、`p = −312` で続く塊になり、隔たりは 0・313・625（実数は 0・312.5・625）。前の字が塊の頭と同じ時刻（`p = 0`）なら、隔たりは 312・625（実数は 312.5・625）。
  - 本番の文字の長さは「字数 × 50 ms」（`duration.rs`）で割り切れるので、本番の値は今と等しい。
  - 文字の層も seriko も、字の現れる時刻を `cue.fire_time + 隔たり`（絶対の時刻）として持つ。`prev_since_chunk_start` は、前の字の現れる時刻（絶対の時刻）から `cue.fire_time` を引いた差である（前半の間は、どちらも台本の時刻で、`cue.at` を引く）。式は 1 本で、写しは無い。「前の字」は、今と同じに、文字の層では同じ場所（普通のバルーンか箱）の前の字、seriko では同じスコープの前の字である。
- **届いた物を持つ列（`held.rs`・後半）**: フレームで評価する受け手が使う、1 本の決まりである。文字の層（合図・置き直しの知らせ・先渡し・トークの印）とバルーンの寿命の時計（寿命の知らせ）が、同じ型を使う。
  - 列は、届いた順を変えない。`pop_due(frame_now)` は、頭の物の時刻が来ていれば出し、来ていなければ `None` を返す。頭の物を飛ばして、後ろの物を先に出す事は無い（台本の順に効く）。
  - 時刻が来たかの判定は 1 つ: 「その物の絶対の時刻 ≤ `frame_now`」。時刻を持たない物（`None`）は、頭に来た時に出る（前の物に続く）。
  - 列は、起点も台本の時刻も持たない。入れる時刻は、入れる側が決めた絶対の時刻である（合図は `cue.fire_time`・置き直しの知らせは効く時刻・現実の出来事は、それが起きた時の時計の読み）。「起点の知らせをまだ受けていない列」という状態は無く、合図を受け取れない時も無い。
  - `waiting_until()` は、頭の物が待っている時刻を返す。呼び手が、その時刻に次のフレームを起こす予約に使う（「areka-runtime」の節）。
  - 2 つの受け手の小さい列に分けずに、1 つの型にする。列の決まり（届いた順・頭の物が待つ間は後ろも待つ・時刻を持たない物は前の物に続く）は、2 つの受け手が同じに守らなければならない決まりで、変える時は両方を確かめ直す（Revalidation Triggers）。決まりを書く所と、そのテストを、1 つにする。1 字ごとの間隔の式（`reveal.rs`）と同じ扱いである。置き場も、同じ理由で `areka-sakura` にする（受け手の全部が引いていて、受け手の側の決まりを置く所）。dola には置かない。再生機は時刻を配るまでを持ち、受け取った物をいつ当てるかは、受け手の側の決まりだからである。
  - 時計も窓も `World` も見ない、純粋な型である。テストは、列に物を入れ、時刻を渡して、出て来る順を見る。
- **刻み**: トークのスレッドは、受けた `TimePoint` をそのまま再生機へ渡す。最初の刻みの時刻を台本の起点に押す（今の `drive.rs` の作りのまま）。有限でない刻みを弾く分かれ道と、その `error!` は消える。
- **同じ時刻の刻みの守りは、取り除く（後半）**: 今の `drive.rs` は、再生中に受けた刻みの時刻が前の刻み以下なら、再生機へ渡さずに読み飛ばす（`debug!` を 1 行）。この守りは、時刻の表の溜めが同じ項目を 2 度配らないための物である。`pop_due` は溜めを持たず、同じ時刻でも前の時刻でも、まだ取り出していない「届いた項目」だけを返す（無ければ何も返さない）。守りに働きが無くなるので、分かれ道と `debug!` を取り除き、受けた刻みは全部 `player.tick` へ渡す。
  - 守りが在ると、同じ時刻の 2 度目の刻みはトークのスレッドで止まり、要件 11 の 10（同じ時刻で 2 度刻まれ、その間に起点が前へ置き直されたら、2 度目に新しい起点で届く項目を届ける）は、時刻の表の中でしか成り立たない。取り除くと、トークのスレッドを通しても成り立つ。
  - 今も、選択を解いた直後の刻み（`on_resolve_choice` が、最後に受けた時刻のまま `player.tick` を呼ぶ）は、守りを通らない。取り除いた後は、刻みの道が 1 つになる。
  - `last_tick`（最後に受けた刻みの時刻）は、選択を解いた直後の刻みに渡す値として残し、受けた刻みの時刻の最大で持つ（前の時刻の刻みで戻さない）。
  - 置き直しが無い再生では、同じ時刻・前の時刻の刻みで何も届かない事は変わらない（今のテスト `duplicate_and_backward_tick_do_not_double_fire` の期待は、そのまま通る）。
  - 前半の間は、時刻の表が `tick`＋`ready` のままなので、守りは残す。取り除くのは、時刻の表を `pop_due` に替えた後である。
- **`ChoiceWaiting`**: `display_end` は `player.occupancy_horizon()` をそのまま入れる。
- **前半と後半**: 前半は、dispatcher が送る値の意味を今のまま（台詞の最初の刻みからの隔たり）にして、型だけ替える。後半で、刻みが運ぶ絶対の時刻をそのまま送る。
  - 前半の間、`SakuraMsg::Tick(TimePoint)` と `ChoiceWaiting.display_end: TimePoint` は、時刻の型に「台詞の最初の刻みからの隔たり」を入れて運ぶ。台本の起点は 0 で、合図の `fire_time` は `at` と同じ値になる。型の意味（時計と同じ物差しの時刻）と値が合わないのは、前半の間だけである。dispatcher が、表示の終わりに `base_now` を足し戻して kanade へ渡す。
  - 後半の 2 で、値が時計の読みになり、`base_now` は消える。消えた事は、締めの検索（`base_now`）で確かめる。

#### areka-kanade

| Field | Detail |
|-------|--------|
| Intent | 刻みが運ぶ時刻で動く。締め切りと `OnSecondChange` を同じ物差しで持つ |
| Requirements | 4.6, 14.6, 17.1, 17.3, 17.5, 17.7 |

- `MonotonicMs` は消し、`TimePoint` を使う（`KanadeMsg::Tick { now }`・`ChoiceTimeoutDue { now }`・`ChoiceWaiting { display_end, timeout_directive }`・相の締め切り・`choice_wake` の送り口）。
- 設定の長さは `KanadeConfig.close_talk_deadline: TimeSpan`・`choice_timeout_default: TimeSpan`（既定はどちらも 30 秒）。
- 選択待ちの期限（`choice_deadline`）: 指令が無ければ `display_end + choice_timeout_default`、0 以下なら無期限、正なら `display_end + 指令`。読み分けと権限の持ち主は変えない。NaN と無限大の指令は、型で表せないので、その分かれ道とテストは消す（記録に書く）。
- `OnSecondChange` の Reference0: `now の値 ÷ 1 時間の 100ns の数（36,000,000,000）` の整数。眠りの間を数えない値になる。割る数は `events.rs` の定数。
- kanade は時計を読まない。dola を引くが、使うのは `dola::time` の型だけで、cue の層と storyboard の層の語彙は使わない。

#### areka-ghost

| Field | Detail |
|-------|--------|
| Intent | 刻み役が 1 本の時計を読み、dispatcher が刻みの時刻を変えずに送る |
| Requirements | 14.1, 14.2, 14.5, 14.6, 14.7, 17.1, 17.2, 17.4 |

- `TickerConfig.clock`・`LoopTickerConfig.clock` は `Box<dyn Fn() -> TimePoint + Send>`。既定は `dola::time::now`（そのために、本番の依存に dola を足す）。周期は `TimeSpan`（50・1000・16 ms は 100ns でちょうど）。`GetTickCount64` を読む関数は消す。
- 境の格子は、今と同じ「時計の 0 からの周期の倍数」。待ちの残りを OS の待ちへ渡す所だけ、`TimeSpan::to_std()` でちょうどの `Duration` にする。
- `Tick { now: TimePoint }`・`TickerMsg::WakeKanadeAt(TimePoint)`。刻みが運ぶ値は、刻み役が起きた時に読んだ時計の値である。
- dispatcher（後半）: `SakuraMsg::Tick(now)` をそのまま送る。`base_now`・逆の写しの丸め・`base_now` が無い時の `warn!` の分かれ道は消える（記録に書く）。`ChoiceWaiting` の `display_end` と `timeout_directive` は、そのまま `KanadeMsg::ChoiceWaiting` へ写す。
- `sink.rs` のログの欄 `at` は `Display` で出し、欄を読むテストは `FromStr` で `TimeSpan` に読んで比べる。

#### areka-seriko

| Field | Detail |
|-------|--------|
| Intent | アニメの時刻を型で持つ。時計を持たず、渡された時刻（刻み・合図の発火時刻・窓の知らせの時刻）だけで動く。字の現れる時刻を、発火時刻から絶対の時刻で持つ |
| Requirements | 7.4, 7.10, 12.6, 12.7, 12.9, 12.10, 17.1, 17.3, 17.4, 18.1, 18.6, 18.7 |

- `SerikoMsg::Tick { now: TimePoint }`・`send_tick(now: TimePoint)`。`*_ms: u64` の引数と欄は、時刻なら `TimePoint`、長さなら `TimeSpan` にする。
- 作者が書く値の縁は、表を組む所（`table.rs`）の 3 つ: pattern定義のコマの待ち（ミリ秒）・`periodic` の秒・`always` の 1 周（コマの待ちの和）。ここで 1 回だけ `TimeSpan` へ変える。毎秒の抽選の 1000 ms と刻みの 16 ms は、`TimeSpan` の定数。
- **時計を持たない（後半）**: seriko が時計を読む所は、合図を処理する時（面の切り替え・着せ替え・バルーンの面・起点の見積もり）と、窓の知らせを送る時だけである。合図は `fire_time` を運び、起点は見積もらず、窓の知らせは送り手が時刻を付ける（下）。だから、seriko が自分で時計を読む所は残らない。`SerikoClock`・`spawn_seriko_clocked`・`LoopRuntime` の時計の欄と、合図を処理する時刻を読む口は消す。seriko を進める刻み役（`areka-ghost` の `LoopTickerConfig.clock`）の既定の時計が `dola::time::now` で、テストの偽の時計を差し込む口は、その刻み役に残る（要件 17 の 1 と 17 の 4 が言う「seriko の時計」は、この刻み役の時計である）。seriko のテストは、刻み・合図・窓の知らせに、時刻を直に書いて渡す。
- **字の現れる時刻（後半）**: `TalkEpoch`（起点の見積もり）は消す。`TalkFeed` は、字ごとの現れる時刻を `TimePoint` で持つ（`cue.fire_time + chunk_reveal_offsets(..)`）。現れた数は「現れる時刻 ≤ 今」の数。台本の時刻へ戻す計算も、ミリ秒への丸めも無い。「今」が発火時刻より前の字は、現れていない。`TalkFeed` は、字の列（数えた分を刈り込む）と別に、「前の字の時刻」の欄を持つ。置き直しの知らせの決まりは、列の字と同じに、この欄にも当てる（当てないと、次の塊の「前の字」が、列の最後の字を読む文字の層と食い違う）。
- **姿を変える指令（`\s`・着せ替え・バルーンの面。後半）**: 届いた順に全部当てる。出来事の時刻は、合図の `fire_time` である（合図を処理する時の時計の読みではない）。当て方は、次の順である。
  1. その面の、`fire_time` までに現れた字の区切りのうち、まだ判定していない物を、口パクの決まりで判定する（替える前の面で起きた事を、先に済ませる）。
  2. 面・着せ替えを替え、`fire_time` を出来事の時刻として構え直す: 一番上の `always` と `runonce` の始まりは `fire_time`。面が見え始めた時刻（`periodic` の周の起点）は `fire_time`。口パクの数え始めは、`fire_time` までに現れた字の数（現れる時刻 ≤ `fire_time` の字の数）。
  3. `fire_time` より後に現れる字は、次の刻みで、替えた後の面の区切りとして判定される。
  - 合図が遅れて届いた時（`fire_time` が、もう済ませた刻みの時刻より前）は、始めたアニメが、過ぎた分だけコマの進んだ姿で出る。刻みの時刻が `fire_time` より前の間（合図が、その時刻の刻みより先に処理された時）は、始めたアニメは、最初のコマの前として扱う。
  - 出来事の時刻が合図だけで決まるので、`always` と `runonce` のコマと、口パクの数え始めは、合図がいつ処理されたかに依らない。
  - 面を画面に出す指令（`DisplayCommand`）そのものは、届いた時にその場で出す。seriko は、届いた合図を、遅れる評価の時計で待たせない。合図が届くのは、必ず発火時刻より後である（再生機は、今が発火時刻に届くまで取り出さない）。
- **知らせ（後半）**: `SerikoSink::origin_set` は、置き直しの知らせ（前の起点が在る物）だけを、`SerikoMsg::OriginSet(OriginNotice)` として合図と同じ口へ積む（順が保たれる）。最初の起点の知らせ（前の起点が無い）は、受け口が受けて、何も送らない。seriko は起点を持たない。置き直しの知らせは、届いた時に当て、全部の `TalkFeed` の字の現れる時刻を、「知らせの受け方」の節の決まりで動かす（効く時刻より後の字だけを、動いた長さだけ動かす）。始まったアニメの始まりの時刻・面が見え始めた時刻・数え済みの字の数は、動かさない（もう起きた事である）。
- **定義の差し替えの依頼（`SerikoMsg::Replace`）**: シェル・バルーンの定義の差し替えは、台本の合図ではなく、フレームの差し替えの相が起こす、その場で効く現実の働きである。時刻を運ばない。届いた時に当て、面を出し直す指令も、その場で出す。差し替えた後の引き金は、次の刻みの時刻で構える（今の作りのまま）。どの刻みになるかは、依頼が届いた時で決まる。台本の時刻に合わせる物ではないので、7 つの変更の受け口と同じ種類として扱う（「受け手ごとの追い付き方」の表）。
- **消去**: `Clear`／`ClearAll` は、現れる時刻が `cue.fire_time` より後の字を捨てる（絶対の時刻どうしの比べ）。
- **窓の知らせ（`SerikoSink::send_stage`。後半）**: 台本の合図ではなく、表示の層の窓の事実（開いた・閉じた・面・世代）を、フレームが見て送る物である。時刻は、それを見たフレームの時刻（`FrameTime`）にする。`send_stage(note, at: TimePoint)` とし、送り手（`areka-runtime` の `frame/status_report.rs`）が、手に持っている `World` から `FrameTime` を読んで渡す。事実の時刻は、それを見たフレームの時刻であり、送る時に時計を読み直すと、同じフレームの中の処理の進み具合が時刻に出る。`SerikoMsg::Stage { note, at: TimePoint }` の時刻は、いつも在る（時計が無い時の `None` は無くなる）。
- **書き出しの進め方（`settle.rs`）**: 画面の外で、決まった時刻まで seriko を進めて絵を求める所である。自分の時刻の軸（0 から）を持ち、刻みも合図もその軸で渡す。面を出す合図は `TalkCue::scheduled(TimePoint::ZERO, TimeSpan::ZERO, ..)` で組む（発火時刻は、進め方の軸の 0）。偽の時計（`AtomicU64`）は、読む所が無くなるので消す。
- **口パクの決まり**: 「受け手ごとの追い付き方」の節。
- 前半は、型だけ替える。`TalkEpoch` と `TalkFeed` の見積もりの式・合図を処理する時に時計を読む所・`SerikoClock`（`Arc<dyn Fn() -> TimePoint + Send + Sync>` にする）は、そのまま残す。

#### areka-emo-text

| Field | Detail |
|-------|--------|
| Intent | 届いた物を、絶対の時刻と一緒に列に持ち、フレームごとに、時刻の来た物を届いた順に当てる。字の現れる時刻を絶対の時刻で持ち、フレームの時刻で現れた数を答える |
| Requirements | 7.5, 7.10, 12.4, 12.5, 12.9, 12.10, 12.13, 18.1, 18.2, 18.4, 18.6, 18.7 |

```rust
impl TextLayerRuntime {
    /// フレームの頭で 1 回呼ぶ。frame_now までに時刻の来た物（合図・置き直しの知らせ・先渡し・トークの印）を、
    /// 届いた順に当てる。返すのは、列の頭の物が待っている時刻（待っている物が無ければ None）。
    pub fn advance(&mut self, frame_now: TimePoint) -> Option<TimePoint>;
}
pub fn present_frame(runtime: &mut TextLayerRuntime, world: &mut World, frame_now: TimePoint) -> Result<(), TextLayerError>;
```

- `visible_glyphs(&ActorKey, frame_now: TimePoint)`・`balloon_shown_glyphs(&ActorKey, surface, frame_now: TimePoint)`・`RevealSchedule::times() -> &[TimePoint]`・`visible(frame_now: TimePoint)`。台本の時刻を受ける引数は、どの口にも残らない。
- 字の現れる時刻は `cue.fire_time + chunk_reveal_offsets(..)`（絶対の時刻）。合図を当てる時に積む。現れた数は「現れる時刻 ≤ フレームの時刻」の数で、それだけである（「見せた字の数」を別に覚えない＝D22）。
- **届いた物を列に持つ（後半）**: 文字の層は、受け口から届いた物を、`TextLayerRuntime` の中の 1 本の列（`HeldQueue`）に、届いた順のまま持つ。UI のスレッドで受け取った時には、当てない（下の「時刻を持たない物」の、列が空の時を除く）。列を持つ所と `advance` は、新しいファイル `actor_held.rs` に置く。

  | 届く物 | 列に入れる時刻 | いつ当たるか |
  |---|---|---|
  | 合図（字も、字のほかの指令も。確かめの口 `CueProbe` が付いていれば、一緒に持つ） | `cue.fire_time` | フレームの時刻が、発火時刻に届いた時 |
  | 置き直しの知らせ（`EmoTextSink::origin_set` → `TextMsg::Origin`。前の起点が在る物だけ） | 効く時刻（`effective_at`） | フレームの時刻が、効く時刻に届いた時 |
  | 先渡し（`TextMsg::Upcoming`） | 無い | 前の物が全部当たった時 |
  | トークの印（選択待ちの始まり・トークの終わり） | 無い | 前の物が全部当たった時 |

- **最初の起点の知らせ**: `EmoTextSink::origin_set` は、前の起点が無い知らせを受けても、何も送らない。文字の層は起点を持たず、この知らせを待つ物も無い。
- **当て方**: `advance(frame_now)` は、`pop_due(frame_now)` が物を返す間、返った順に当てる。合図は、今の「届いた合図を当てる」関数（`TextLayerRuntime::apply_cue`。消去なら描く側の全部を消す求め・選択肢の写しの後始末・箱の写しの刈り込みまで）を通す。先渡しは `preview_talk`、トークの印は、今それぞれを受けた時にしている処理を通す。当てる関数の中身で変わるのは、字の現れる時刻を絶対の時刻で積む事だけである。
- **字と、字のほかの指令は、同じ決まりで効く**: 消去・全部の消去・改行・カーソル・印・書体・アンカーは、字と同じに、フレームの時刻が合図の発火時刻に届いた時に効く。発火時刻が、届いたフレームの時刻より後の消去は、そのフレームでは画面を消さず、フレームの時刻が発火時刻に届いた最初のフレームで消す。
- **順は変えない**: 列は届いた順（再生機が配った順＝台本の順）を保ち、頭の物の時刻が来るまで、後ろの物を当てない。同じ時刻の物は、台本の順に当たる。
- **時刻を持たない物**: 先渡しとトークの印は、自分の時刻を持たず、前の物に続く。届いた時に列が空なら（前に届いた物は、全部当たっている）、待つ相手が無いので、その場で当てる。列に物が残っていれば、後ろに並べ、前の物が当たった同じ `advance` の中で当たる。
  - 先渡し（これから届く合図の全部）は、時刻を使わない空回しで、区間ごとの全文を先に求める物である。空回しは「その時の状態の写し」から始め、区間の番号は「先渡しを当ててからの消去の数」で数える。だから、先渡しは、前の台詞の合図が全部当たった後・その台詞の最初の合図の前に、当てなければならない。列の順が、それを保つ（再生機は、受け口の登録の時に先渡しを渡し、その後に合図を配る）。先渡しを、列に残っている前の台詞の合図より先に当てると、残っている消去が、先渡しの後の消去として数えられ、区間の番号がずれる。空回しと配置の計算は、時刻を見ないので、置き直しで変わらない。
  - 起点を先の時刻に置いた台本の先渡しは、受け口の登録の時に届くので、起点の時刻より前に当たる事が在る（本番の起点は最初の刻みなので、間は無い）。要件 18 の 4 の「何も現さない」が指すのは、その台本の合図の中身（字・字のほかの指令・口パク・寿命の知らせ）である。先渡しは、中身を現す物ではなく、まだ当たっていない合図の配置の下ごしらえで、当てても、画面に出ている物は変わらない。空回しの入れ替えは、場所ごとに、今の区間の全文を持ち越す。空回しの側の全文に替わるのは、最初の消去より前に字を足す場所だけで、内容の在る台本は、頭が全部の消去なので、当たらない。選択待ちの口は、前のトークの終わりの印が、もう閉じている。
  - トークの終わりは「全部の場所の選択肢を済みにする」、選択待ちの始まりは「押下で選択肢を済ませてよい」を、文字の層に伝える。どちらも、その前に届いた合図（選択肢の合図）が当たった後でなければ、意味が合わない。
  - 押下の口（`settle_choices_at`）が読む「選択待ちに入ったか」: 選択待ちの始まりの印が列で待つのは、前に届いた合図がまだ列に在る間（本番では、1 フレームの間）だけである。選択肢の合図がまだ当たっていなければ、選択肢の行は画面に出ていないので、押せない（当たりの行は、最後の提示の写しから読む）。選択肢が画面に出た後に届いた印は、列が空なら、届いた時に当たる。残るのは、選択肢がもう画面に出ていて、その後ろの合図がまだ列で待っている 1 フレームの間の押下である。その押下は、kanade が選択として受け、文字の層は、その場では選択肢を済みにしない。その選択肢は、トークの終わりの印（全部の場所の選択肢を済みにする）で済みになる。
- **strict の台本の知らせ（`CueProbe`）**: 名前の形の `\b[名前]` の箱が無い事の知らせは、合図を当てる時（`advance`）に出す。箱を探すのは、当てる時の状態だからである。知らせの口は、台本の位置とゴーストの名を自分で持ち、どのスレッドから呼ばれても、記録を 1 件書くだけである。トークのスレッドが、終わりの前に残りの控えを記録する事（`drive.rs` の `finish_strict`）との前後は、今も決まっていない（文字の層は、UI のスレッドで後から知らせる）。記録の順は約束しない。記録は台本の位置を持つので、読む側は位置で並べる。
- **置き直しの知らせ**: 「知らせの受け方」の節の決まりである。知らせが当たる時に、全部の場所の `RevealSchedule` の、効く時刻より後の時刻を、動いた長さだけ動かす（効く時刻より前になる時刻は、効く時刻にする）。効く時刻以前の時刻と、もう当てた指令は、書き換えない。動かす決まりは順を保つので、時刻の列は、減らない並びのままである。
- **フレームが見せる物**: あるフレームが見せる物（どの指令まで当たっているか・どの字まで現れているか）は、届いた物（合図と置き直しの知らせ）と、そのフレームの時刻だけで決まる。それが、いつ・どうまとまって届いたかに依らない（そのフレームの時刻までに当たる物が、全部届いていれば）。遅れて届いた物は、届いた次のフレームで、過ぎた分が順に全部当たる。
- **フレームの時刻は戻らない**: `advance` に渡す時刻は、`FrameTime`（1 本の時計の読み）で、前のフレームより前にならない。前の時刻を渡されても、当てた物は戻さない。
- **読み口**: 文字の層の状態を読む口（現れた字の数・消した回数・選択肢の有無・当たりの行）は、その時までに当たった物の状態を返す。合図と置き直しの知らせは `advance` でだけ当たるので、画面に出ている物と、読み口の答えは、同じフレームの物である。
- **列を空にする口は、置かない**: 台詞の中断・置き換えと、実行系だけを起こし直す読み直し（文字の層と結線は残る）では、列に残っている物を捨てない。配られた合図の発火時刻は、配った刻みの時刻以前なので、残っている物は、次のフレームで時刻が来て当たる。中断された台詞の、もう配られた合図が当たる事と、その字が現れ続ける事は、今と同じである（次の、内容の在る台本の頭の全部の消去が消す）。ゴーストの切り替えと、窓を作り直す読み直しは、今と同じに、文字の層と結線ごと作り直すので、列も一緒に無くなる。
- **受け取った時に、旗を立てる**: UI のスレッドで受け取った物を、列に足した後（列が空で、その場で当てた時は、当てた後）に、次の画面の更新でフレームを回す旗（`tick_wake::PRESENT`）を立てる。立てる所は、受け取りの口（`actor_held.rs`）の 1 か所である。文字の線の物は、フレームの外で届き、文字の受け口も、受け取る所も、今は旗を立てない（今は、同じ合図から出た寿命の知らせの旗で、フレームが回る）。旗が無いと、列に入った物は、ほかの線の旗か心拍まで、当たらない。旗は、足した後に立てる（先に立てると、旗を読んでから足すまでの間にフレームが回り、旗だけが倒れる。表示の指令の橋渡しと同じ順である）。
- **次のフレームを起こす**: `advance` が、待っている時刻を返した時は、呼び手（`areka-runtime`）が、その時刻に次のフレームを起こす予約を置く（「areka-runtime」の節）。
- **前半**: 列は入れない。届いた合図をその場で当てる今の作りのまま、型だけ替える。字の現れる時刻は台本の時刻（`TimeSpan`）で持ち、提示と読み口は台本の時刻（今の、0 への寄せのまま 0 以上なので `TimeSpan`）を受ける。この口は、後半でもう一度替わる（D16）。
- テストの「全部が現れている時刻」は、大きい実数でなく `TimePoint::MAX` で書く（前半の間は `TimeSpan::MAX`）。状態の決まりだけを確かめるテストは、今と同じに、当てる関数を直に呼ぶ。列を通すテストは、合図を入れて、`advance` に時刻を渡す。

#### areka-runtime

| Field | Detail |
|-------|--------|
| Intent | フレームの頭で文字の層を進める。文字の相と可視性の相が、同じ状態を、同じフレームの時刻で読む。バルーンの寿命の時計が、寿命の事実を絶対の時刻で持つ。手で組む合図と窓の知らせに、フレームの時刻を付ける |
| Requirements | 7.4, 7.9, 8.2, 8.3, 12.4, 12.8, 12.9, 12.11, 12.12, 12.14, 17.1, 17.9, 18.1, 18.2, 18.4, 18.7 |

```rust
impl TalkClock {
    pub fn new(clock: Arc<dyn Fn() -> TimePoint + Send + Sync>) -> Self;
    /// 今の時刻（差し込まれた時計の読み）。
    pub fn now(&self) -> TimePoint;
}

pub enum TalkLifecycleSignal {
    /// そのトークの最初の合図が配られた。at は、その合図の発火時刻。
    TalkStarted { at: TimePoint, talk_id: Option<TalkId> },
    /// 表示の終わり（発火時刻＋長さ）の、それまでの最大が上がった。at は、上げた合図の発火時刻。
    DisplayEndAt { at: TimePoint, end: TimePoint },
    /// 待ち時間の指定。at は、その合図の発火時刻。
    BalloonTimeout { at: TimePoint, timeout: TalkTimeout },
    /// 選択肢が出た。at は、その合図の発火時刻。
    ChoiceShown { at: TimePoint },
    /// 置き直しの知らせ（前の起点が在る物だけ）。
    OriginMoved(OriginNotice),
    /// トークの受け口が落ちた。at は、落ちた時の時計の読み。
    TalkEnded { at: TimePoint },
    /// 利用者の中断。at は、押下を受けた時の時計の読み。
    UserBreak { at: TimePoint },
}
impl TalkLifecycleSignal {
    /// 列に入れる時刻（この知らせが効く絶対の時刻。OriginMoved は効く時刻）。
    pub fn at(&self) -> TimePoint;
}

/// フレームの頭で、文字の層を進める相。
pub fn run_text_advance_phase(wiring: &mut RuntimeWiring, world: &mut World, frame_now_override: Option<TimePoint>);
pub fn run_text_phase(wiring: &mut RuntimeWiring, world: &mut World, frame_now_override: Option<TimePoint>);
```

- 既定の時計は `dola::time::now`（`FrameTime` と同じ関数）。seriko を進める刻み役と、ghost の刻み役の既定の時計も同じ関数で、`GetTickCount64` を読む `tick_count_ms` は消す。この spec が替える所に、ほかの時計は足さない。
- **`TalkClock` に残る物（後半）**: `TalkClock` は「今」を読む口だけを持つ。起点も、台本の時刻を返す口も持たない。残す理由は 2 つである。現実の出来事に時刻を付ける 2 つの所（トークの受け口が落ちた時＝トークのスレッド、利用者の押下を受けた時＝UI のスレッド）が、同じ 1 つの差し込める時計を読む。テストが、その時計を偽の時計に替える。合図を見て起点を見積もる `observe_cue`・台本の時刻を返す口（`talk_time`・`now_talk_time`）・合図を見るために文字の層の受け口を包んでいた `ClockedTextSink`・台本の時刻を求める `resolve_talk_time` は、消す。
- **判定の時刻は、フレームの時刻である（後半）**: 文字の相と可視性の相は、`FrameTime` をそのまま受け、文字の層と寿命の時計が持つ絶対の時刻と比べる。台本の時刻を求める所は無い。フレームの時刻を求める関数は 1 本（`resolve_frame_now(frame_now_override, world) -> Option<TimePoint>`。テストの値が在ればそれ、無ければ `FrameTime`、どちらも無ければ `None`）で、3 つの相（進める相・可視性の相・文字の相）が同じ関数を呼ぶ。
- **`advance` を呼ぶ所（後半）**: `frame_system` の中で、`RuntimeWiring` を取り出した直後・作業領域の同期と装着の相より前に、進める相（`run_text_advance_phase`。`frame/scale_text.rs` に置く）を置く。1 フレームに 1 回だけである。今は、合図がフレームの外（UI のスレッドのメッセージの取り出し）で当たるので、フレームのどの相も、合図が当たった後の文字の層の状態を見ている。文字の層に触る相（装着・取り出し・差し替え・可視性・文字の拡大率・文字）の全部が、進めた後の状態を見るように、どの相よりも前に置く。可視性の相（文字の相より前に在る）が読む字の数と、文字の相が描く字は、同じ状態・同じフレームの時刻から出る。
  - **文字の層が借りられないフレーム**: 進める相は、文字の層を借りられなければ（ほかの処理が持っている）、進めずに、`error!` を出し（同じ縮みが続く間は 1 回）、次の画面の更新を予約する（`tick_wake::REARM`）。列の物は残り、次のフレームで当たる。毎フレーム文字の層に触る、あとの 2 つの相（文字の拡大率の相・文字の相）も、同じ扱いにそろえる。借りられなければ、そのフレームの仕事を見送り、`error!` を出し、次の画面の更新を予約する。
    - 今は、この 2 つの相は `borrow_mut()` で、借りられなければ落ちる。借りられなければ見送るのは、可視性の相だけである。進める相だけが見送っても、同じフレームの後ろの相が落ちるので、3 つの相を 1 つの扱いにする。
    - 見送った時の姿は、どの相も決まっている（進める相＝列の物が 1 フレーム遅れて当たる。拡大率の相＝文字の層の付け直しが 1 フレーム遅れる。文字の相＝そのフレームは描かない）。続けられるので、落とさない。
    - 装着・差し替え・面の切り替えの相の `borrow_mut()` は、装着や切り替えの出来事の時にだけ文字の層に触る所で、この spec は触らない（今のまま、借りられなければ落ちる）。
  - フレームの時刻が無い世界（`FrameTime` もテストの値も無い）では、進めず、次の画面の更新を予約する（`REARM`）。本番では起きない（wintf は、世界を作る時に `FrameTime` を必ず置く）。
  - 文字の相は、フレームの時刻が在れば `present_frame` を呼ぶ。「起点がまだ無いので見送る」分かれ道は無くなる（合図が 1 つも当たっていない間は、状態が空で、描く物が無い）。
- **次のフレームを起こす（後半）**: 門が有効な時、フレームは、起こす旗が立った時と、預けた期限が来た時にだけ回る（どちらも無ければ、心拍の、画面の更新 30 回に 1 回）。決まりは 1 つである。**受け手の列に物が残っている間と、まだ現れていない字が在る間は、必ず、次のフレームが予約されている。**
  - 列に物を足した時: 足した側が、足した後に旗を立てる。文字の線は、UI のスレッドの受け取りの口（`areka-emo-text` の `actor_held.rs`。`PRESENT`）。寿命の線は、受け口が、送った後に旗を立てる（今の `talk_lifecycle.rs` のまま。`PRESENT`）。利用者の中断は、ポインタのメッセージの中で送るので、そのメッセージの旗（`POINTER`）が、送った後のフレームを回す。
  - フレームが回って、時刻の来ていない物が列の頭に残った時: その時刻への予約を置く。文字の層の列は、`advance` が返した時刻 `t` について、進める相が `tick_wake::arm_deadline(Instant::now() + 残り)` を置く。残りは `t − フレームの時刻` を `to_std()` で渡す（`Instant` の側は範囲の外）。寿命の列は、頭の物が待っている時刻（`waiting_until()`）を、期限と同じ予約（`visibility_wake`）に入れ、早い方で起こす。
  - フレームが回って、進める相が進められなかった時（借りられない・フレームの時刻が無い）: 次の画面の更新を予約する（`REARM`）。
  - まだ現れていない字が在る時: 文字の相が、次に現れる字の時刻（全部の場所の、フレームの時刻より後の現れる時刻の、最も早い物）への予約を置く（`arm_deadline`）。今は、現れていない字が在る間、毎回の画面の更新を予約する（`reveal_pending` と `REARM`）。字の現れる時刻は分かっているので、その時刻に起こす。字が先の時刻まで現れない間（後ろへ長く動かす置き直しの後など）に、何も変わらないフレームを回し続けない。見える物は変わらない（字は、現れる時刻の後の、最初の画面の更新で現れる）。
  - 預けられる期限は、最も早い 1 つだけで、来ると倒れる。待つ物が在る限り、相が走るたびに預け直す（可視性の相の期限と同じ扱い）。遠すぎる期限は、可視性の相と同じに、1 時間で頭打ちにして預け直す。
  - 本番で、時刻の来ていない物が列の頭に残るのは、寿命の線だけである（フレームの中で取り出すので、フレームの時刻を読んだ後に刻まれた合図の知らせが、同じフレームで届く）。文字の線の物は、フレームの外で届き、次のフレームの時刻は、その後に読むので、時刻はもう来ている。文字の線で要るのは、足した後にフレームが回る事（上の旗）である。
  - 旗を立てる所の一覧も直す。wintf の `tick_wake.rs` の頭の一覧は、`PRESENT` の行に、文字の層の受け取りを足す。本体の `tick_gate_config_producers_tests.rs` は、`AREKA_PRODUCERS` に `areka-emo-text/actor_held.rs`（`PRESENT`）の行を足し、木の走査の根に `areka-emo-text` を足す。`areka-runtime/frame/scale_text.rs` の行は、`REARM` のままである（進める相が、進められなかった時に立てる）。文字の相と進める相が期限を預ける事は、可視性の相の `arm_deadline` を見るテストと同じ形で確かめる。
- **0 に丸めない**: 判定は、絶対の時刻どうしの比べである。フレームの時刻が合図の発火時刻より前の間は、その合図は当たらず、字も現れず、寿命の知らせも効かない。
- **バルーンの寿命（後半）**: 寿命の時計は、寿命の事実を絶対の時刻で持つ。起点を持たない。
  - 受け口（`BalloonLifecycleSink`）は、知らせごとに、効く時刻を付けて送る（上の型）。合図から作る知らせ（トークの始まり・表示の終わり・待ちの指定・選択肢が出た）は、その合図の発火時刻。表示の終わりの値は `cue.fire_time + cue.duration`（整数の和）。トークの終わりは、受け口が落ちた時の時計の読み（`TalkClock::now()`）。台本の中身ではなく、再生が終わった・中断されたという現実の出来事だからである。時計の読みは必ず在るので、「止まった時刻が無い」分かれ道（`TalkEnd::TimeUnknown`・その時の起点の採り方・`warn!`）は消える。
  - 利用者の中断（`UserBreak`）は、UI のスレッドの押下の口（`areka-input` の `user_break.rs`）が、押下を受けた時の時計の読みを付けて、同じ線へ送る。押下の口は、結線の時に渡された `TalkClock` の複製を読む。列には、ほかの知らせと同じに、届いた順に並ぶ。フレームの時刻は、押下より後に同じスレッドで読むので、次のフレームでは必ず時刻が来ている。本物の時計では、どの知らせの時刻も、それを送った時より後にならないので、押下より前に届いた知らせが、列の頭で押下を待たせる事は無い。
  - 可視性の相は、線から取り出した知らせを、届いた順のまま列（`HeldQueue`）に足し、フレームの時刻が来た物を、頭から順に判断へ渡す（新しいファイル `balloon_visibility_held.rs`）。**知らせが効くのは、フレームの時刻が、その知らせの時刻に届いた最初のフレームである**（頭の物が待つ間は、後ろも待つ）。寿命の線（`std::sync::mpsc`）は、フレームの中で取り出すので、フレームの時刻を読んだ後に刻まれた合図の知らせが、同じフレームで届く事が在る。その知らせは、時刻がまだ来ていないので、次のフレームで効く。
  - トークの終わりを数える通し番号（`talk_ends`。位置の指令の線との、届いた順の突き合わせに使う）は、今と同じに、線から取り出した時に数える。届いた順を数える物で、時刻で待たせる物ではない。
  - 判断（`decide`・`decide_timeout`）は、フレームの時刻（`TimePoint`）で行う。表示の終わり・トークの終わりの時刻・期限は、どれも `TimePoint`。期限は `実効の表示の終わり + 待ち`、切れたかは `フレームの時刻 ≥ 期限`。待ちは `TimeSpan`（既定 30 秒・`\![set,balloontimeout,ミリ秒]` と環境変数は、読む縁で 1 回だけ変える）。「まだ見ていない」は `Option` で表す（負の無限大の番兵を使わない）。有限でない終わりを捨てる分かれ道と `warn!`・負の終わりを 0 に寄せる所は、消える。
  - **現れた字の数の出どころ**: 可視性の相が読む字の数は、文字の層の状態（そのフレームの頭の、進める相が進めた物）を、同じフレームの時刻で読んだ値である（`balloon_shown_glyphs(actor, surface, frame_now)`）。文字の相も、同じ状態を、同じ時刻で描く。寿命の時計は、字の数を自分では持たない。
  - **2 本の線**: 寿命の線と文字の線は別の線なので、同じ合図から出た物が、片方にだけ先に届くフレームが在る。その時も、2 つの相が読む字の数は同じで、時刻の基準も同じ（フレームの時刻）である。受け手が起点を持たないので、線ごとに起点が食い違う事が無い。先に届いた寿命の知らせ（トークの始まり・表示の終わり）は、自分の時刻が来ていれば効き、文字の層の合図は、届いた次のフレームで当たる。「片方の線が 1 フレーム先に届く」事は、今の作りにも在る（寿命の線はフレームの中で、文字の線はフレームの外で受ける）。
  - **置き直しの知らせ**: 受け口は、`origin_set` で受けた知らせのうち、前の起点が在り、そのトークの最初の合図より後の物だけを、`OriginMoved` として同じ線へ送る（最初の合図より前には、受け手は、そのトークの物を何も持っていない）。受け口は、覚えている表示の終わりの最大も、同じ決まりで動かす（動かさないと、前へ動いた後の合図の終わりが「最大を越えない」と判定されて、送られない）。可視性の相は、知らせが当たる時に、持っている表示の終わりが効く時刻より後なら、動いた長さだけ動かす（効く時刻より前になるなら、効く時刻にする）。期限は動かさない。期限が在るのは、トークの終わりが効いた後だけで、その時の表示の終わりから作った物であり、置き直しの知らせは、トークが終わる前にしか届かない。もう下した判断（時間切れで閉じた）は、戻さない。
  - 列を空にする口は、置かない（文字の層の列と同じ。寿命の列は結線が持ち、結線ごと作り直す時にだけ無くなる）。
- **起こしの待ち**: `visibility_wake` が `Instant` の締め切りへ渡す残りは、`(期限 − フレームの時刻)` が正の時だけ `to_std()` で渡す（`Instant` の側は範囲の外）。
- `hover_inject`: 周期は `TimeSpan`、時刻はフレームの時刻（`TimePoint`）で、何周目かは、時刻の 100ns の数を周期で割った整数。0 の周期は「無し」（今の「正でない周期は無し」と同じ）。前半の間は、台本の時刻（`TimeSpan`）を受ける。
- **手で組む合図（`shell_carry.rs`。後半）**: ghost の読み直しの後に、前の面と着せ替えを seriko へ出し直す所である。台本の合図ではなく、フレームの中の装着の処理が起こす出来事なので、時刻は、出し直しを行うフレームの時刻（`FrameTime`）にする。`replay` は、手に持っている `World` から `FrameTime` を読み、`TalkCue::scheduled(フレームの時刻, TimeSpan::ZERO, ..)` で組む。seriko は、その `fire_time` を出来事の時刻にする（「areka-seriko」の節）。`FrameTime` の無い世界（wintf は、世界を作る時に必ず置く）では、`error!` を出して、出し直さない（黙って時刻を作らない）。窓の知らせ（`frame/status_report.rs`）も、同じに `FrameTime` を読んで付ける。
- **7 つの変更の受け口**（`change_cue`・`move_cue`・`reload_cue`・`switch_cue`・`user_break_cue`・`window_state_cue`・`zorder_cue`）は、届いた順に、その場で当てる。合図の時刻は読まない。取り込みの行のほか、本番は変わらない。これらは、フレームで描く状態ではなく、その場で効く現実の働き（ゴーストの切り替え・窓を動かす・読み直す・重なりの順ほか）である。合図が届くのは、必ず発火時刻より後である（再生機は、今が発火時刻に届くまで取り出さず、届くまでには、さらに時間が経つ）。遅れる評価の時計を持たないので、届いた時に当てても、発火時刻より前に効く事が無い。大前提（発火時刻より前に効かない・台本の順に効く）と食い違わない。
- 前半は、`TalkClock` の見積もり（最大を取る）・`ClockedTextSink`・寿命の知らせを届いた時に当てる作りを、型だけ替えて残す（寿命の知らせの時刻は、台本の時刻の `TimeSpan`）。`TalkClock` の口と `TalkLifecycleSignal` の形は、後半でもう一度替わる（D16）。

#### areka-input・areka-session・本体・先進坑

- 本番で時刻が流れる所は、`areka-input` の利用者の中断の知らせ（後半。押下を受けた時の時計の読みを付ける＝`src/user_break.rs`）だけである。時刻の型は `areka_sakura::contract` から名指す。ほかに替えるのは、`TalkClock::new` へ渡す時計・`FrameTime(..)` を書く所・手で組む `TalkCue`・`present_frame(..)`・偽の時計（`TickerConfig`・`LoopTickerConfig`・前半の間の `SerikoClock`）・`MonotonicMs` を書くテスト。文字の層へ合図を直に入れて画面を確かめるテストは、後半で、合図を入れて `advance(フレームの時刻)` を呼ぶ形へ直す（共有の組み立てに 1 つ置く）。
- 先進坑（`q7-vertical/src/old_side.rs`）は全体テストが組まない。替えた後に、そのワークスペースで `cargo check` を 1 回通す。

### 受け手ごとの追い付き方

台本の中身が、いつ効くかの決まりと、合図が発火時刻より遅れて届いた時（起点が過ぎた台本を受けた時・刻みが粗い時・起点が前へ置き直された時）の決まり。中身は、届いた時でなく、合図の発火時刻に効く（D17）。

| 受け手 | 中身の種類 | 決まり |
|---|---|---|
| 文字の層の字 | 積もって姿を作る物（字は、当てる時に積む）と、時刻だけで決まる値（どこまで見えるか） | 合図は、フレームの時刻が合図の発火時刻に届いた時に当たる。当たると、その塊の字の全部が、字ごとの現れる時刻（絶対の時刻）と一緒に積まれる。どこまで見えるかは、フレームの時刻で求めるだけ。遅れて届いたタイプライターの合図は、現れる時刻がもう過ぎている字が、当たったフレームで一度に全部現れ、残りの字は、それぞれの現れる時刻に、元の間隔のまま現れる（1 字 50 ms の塊が 250 ms 遅れて届けば、発火時刻から 0・50・100・150・200 ms の 5 字が一度に現れる。フレームの時刻がちょうど 250 ms 後なら、6 字目も現れる）。1 字ずつ追い掛ける途中の姿は作らない |
| 文字の層の、字のほかの指令（消去・改行・カーソル・印・書体・アンカー） | 積もって姿を作る物 | 字と同じ決まりで、フレームの時刻が合図の発火時刻に届いた時に、届いた順に当てる。遅れて届いた物は、次のフレームで、過ぎた分が順に全部当たる。届いた時には当てない |
| seriko の口パク（`talk` の引き金） | その瞬間にだけ意味を持つ物 | 下の決まり |
| seriko の、姿を変える指令（`\s`・着せ替え・バルーンの面） | 積もって姿を作る物 | 届いた順に全部当てる。出来事の時刻は、合図の発火時刻である: その時刻までの字の区切りを先に判定し、その時刻を、始めるアニメ（`always`・`runonce`）の始まり・面が見え始めた時刻・口パクの数え始めにする。遅れて届いたら、始めたアニメは、過ぎた分だけコマの進んだ姿で出る |
| バルーンの寿命の時計 | 時刻だけで決まる値（寿命の知らせは、積もって姿を作る物） | 寿命の知らせは、フレームの時刻が、知らせの時刻（元の合図の発火時刻。トークの終わりと利用者の中断は、それが起きた時の時計の読み）に届いた時に、届いた順に当てる。表示の終わりと期限は、絶対の時刻で持つ。尽きたかは、フレームの時刻と比べるだけ |
| その場で効く現実の働き（7 つの変更の受け口） | 積もって姿を作る物 | 届いた順に、その場で全部当てる。合図の時刻は読まない。合図が届くのは必ず発火時刻より後で、遅れる評価の時計を持たないので、発火時刻より前に効く事が無い |
| seriko の定義の差し替えの依頼（`SerikoMsg::Replace`） | その場で効く現実の働き | 時刻を運ばない。届いた時に当て、差し替えた後の引き金は、次の刻みの時刻で構える。どの刻みになるかは、届いた時で決まる（台本の時刻に合わせる物ではない） |
| 手で組む合図（読み直しの後の、面と着せ替えの出し直し）と、窓の知らせ | 積もって姿を作る物 | 時刻は、それを行う・見るフレームの時刻。seriko は、姿を変える指令と同じ決まりで当てる |
| storyboard の層の値 | 時刻だけで決まる値 | 今の時刻の値を求めるだけ |
| storyboard の層の引き金 | その瞬間にだけ意味を持つ物 | 過ぎた引き金を時刻の順に全部起こす。子は、刻みの時刻でなく、引き金の予定の時刻（＋開始のずらし）で始める（今の作りのまま） |

**口パクの決まり**: 1 回の刻みで、現れた字の数が区切り（`talk,N` の N 字ごと）を 1 つ以上越えたら、越えた区切りを古い順に 1 つずつ、次のとおり判定する。

1. 区切りの字が現れた時刻 T（絶対の時刻）で、そのアニメが再生中かを見る。
2. 再生中なら、その区切りは見送る（積まない）。
3. 再生中でなければ、T を始まりとして再生を始める（始まりが過去なら、過ぎた分だけコマが進んだ姿になる）。前の再生が T までに終わっていたら、終わりまで当ててから替える。

- 同じ時刻に現れた字（`\_q` の区間・長さ 0 の塊）は、最初の区切りで始まり、後ろの区切りは「再生中」で見送られる。結果は 1 回の再生で、今と同じになる。
- 越えた区切りの時刻が違う時は、細かい刻みで 1 つずつ越えた時と同じ結果になる。だから、口の姿は、刻みの細かさと、合図がまとまって届いた事に依らない。
- 起点の置き直しで複数の字がまとめて現れた時も、同じ決まりで扱う（要件 12 の 7）。
- 数えは戻さない。置き直しの知らせで、字の現れる時刻が後ろへ動いても、もう判定した区切りは判定し直さず、数えた数を越える字が現れるまで鳴らない。
- 「1 回の刻みでは、いちばん新しい区切りの 1 回だけを判定し、手前の区切りは捨てる」決まりからの変更である。変える前後の期待を記録に書く。

**どれだけ厳しく時刻に合わせるかは、中身の種類ごとに決める**: 時刻の土台は、どの受け手にも正確な時刻（合図の発火時刻・起点の知らせの効く時刻）を渡す。受け手が、それにどれだけ厳しく従うかは、中身の種類ごとに決める。

- 字と、字のほかの指令（文字の層）・バルーンの寿命は、届いた物と、フレームの時刻だけで決まる（上の表）。届くのが遅れた時の限界は、「知らせの受け方」の節に書く。
- 口パクは、きっちり合わせなくてよい中身である。seriko が、その時刻の刻みを済ませる前に、合図と知らせが届いていれば、同じ結果になれば足りる。だから、決まりは上の 3 つだけにして、増やさない。seriko は、もう済ませた刻みの判定をやり直さないので、次の時は、結果が違ってよい。
  1. 合図が、その発火時刻より後の刻みを済ませてから届いた時。その字の区切りは、届いた後の最初の刻みで、字の現れた時刻を始まりとして判定される（始まりが過去なので、コマが進んだ姿で出る）。済ませた刻みの間に、同じアニメの別の再生が始まっていれば、見送られる区切りが変わる。
  2. 後ろへ d だけ動かす置き直しの知らせが、効く時刻より後の刻みを済ませてから届いた時。済ませた刻みが数えた区切りは、前の時刻のまま判定済みで、数えを戻さないので、動いた後の時刻では鳴らない。その区切りの口パクの始まりは、時刻どおりに受けた seriko より、d だけ早い。違いの大きさは、刻みの幅でなく、動いた長さ d で決まる。違うのは、効く時刻から、知らせが届くまでに済ませた刻みが数えた区切りだけである。
  3. 再生が終わった直後の区切りを、遅れて判定した時。
- 口パクのテストは、それより細かい一致を求めない。確かめるのは、「同じ時刻の列を、届く順とまとまりを変えて与えても、刻みを済ませる前に合図と知らせが届いていれば、結果が同じ」事と、決まりの 3 つである。合図や知らせが、その時刻の刻みより後に届いた時の一致は、テストにしない。
- 後の spec が足す中身の種類（音・舞台・値を時間で変える口）は、それぞれ、自分がどれだけ厳しく合わせるかを書く。この spec は、その種類の決まりを先に決めない。

### 知らせの受け方

起点の知らせ（`OriginNotice`）を受けた受け手の決まり。文字の層・バルーンの寿命の時計・seriko の、どれにも同じ決まりである。受け手は、受け取った物を絶対の時刻で持ち、起点を持たない（D13）。

**最初の起点の知らせ（前の起点が無い）**: 起点を知らせるだけの物で、置き直しではない。受け手は、何も動かさず、何も待たせない。受け口が受けて、受け手へは送らない。受け手の持つ時刻は、合図の `fire_time` から作った絶対の時刻なので、次の再生が始まっても、前の再生の物の時刻は変わらない。

**置き直しの知らせ（前の起点が在る）**: 動いた長さを d（新しい起点 − 前の起点）、効く時刻を E とする。受け手は、持っている時刻 T のそれぞれを、次のとおりにする。

1. T が E 以前なら、書き換えない。
2. T が E より後なら、T + d にする。T + d が E より前になる時（前へ動いた時）は、E にする。

もう起きた事（当てた指令・判定した口パクの区切り・下した寿命の判断）は、戻さない。

| 受け手 | 持っている時刻（決まりを当てる物） | 戻さない物 |
|---|---|---|
| 文字の層 | 字の現れる時刻（当てた合図の字） | 当てた指令（消した物を戻さない） |
| seriko | 字の現れる時刻（`TalkFeed` の字の列と、「前の字の時刻」の欄） | 判定した区切り・数えた字の数・始まったアニメの始まりの時刻・面が見え始めた時刻 |
| バルーンの寿命の時計 | 表示の終わり（受け口が覚えている最大も） | 期限（トークが終わった後にだけ在る）・トークの終わりの時刻・下した判断 |

- **合図の発火時刻は、動かない**: 置き直しの前に取り出した合図の発火時刻は、どれも効く時刻以前である（再生機は、今が発火時刻に届くまで取り出さず、置き直しに渡す現在の時刻は、その前の取り出しに渡した時刻以上である＝「`TimedSchedule`」の節の前提）。だから、列に残っている合図と、合図から作った寿命の知らせの時刻は、決まりの 1 に当たり、動かない。動くのは、合図の発火時刻より先へ延びる時刻（字の現れる時刻・表示の終わり）である。前提が破れた呼び方でも、受け手は、同じ決まりを、届いた順と、運ばれた時刻だけで当てるので、結果は届き方に依らない。
- **どの再生の物かを分けない**: 受け手は、持っている時刻を、再生ごとに分けない（合図にも知らせにも、再生を分ける印は無い）。中断された台詞の、まだ現れていない字も、効く時刻より後なら、同じ長さだけ動く。文字の層も seriko も同じ決まりなので、食い違わない。寿命の時計の表示の終わりは、トークの始まりで捨てるので、そのトークの物だけが動く。

**当てる順（D21）**: 知らせは、合図と同じ線で、届いた順のまま受け手へ届く。受け手は、届いた順の中で当てる。

- seriko: 届いた時に当てる（合図も、届いた時に、字の現れる時刻を積む）。
- 文字の層とバルーンの寿命の時計（フレームで評価する受け手）: 知らせを、合図と同じ列に並べる。列に入れる時刻は、効く時刻 E である。フレームの時刻が E に届いたフレームで、前に届いた物が全部当たった後に、当たる。
  - 届いた時に当てない理由の 1 つ目: 文字の層は、字の現れる時刻を、合図を当てる時に積む。知らせより前に届いた合図が、まだ当たっていなければ、その字の時刻は、まだ無い。知らせを先に当てると、その字は動かず、合図を先に当てていた受け手と食い違う。列の順に当てれば、どの受け手でも、「届いた順に全部当てた」のと同じ時刻になる（seriko が届いた時に当てた結果とも、同じである）。
  - 理由の 2 つ目: 知らせの後に届く合図は、新しい起点での発火時刻を持つ。前へ動いた後は、その発火時刻が E より前の事が在る。その合図は、E より前のフレームで当たってはならない（E より前は、前の起点が効いていて、その合図の時刻は来ていない）。知らせを E で待たせると、列の順が、それを保つ。
  - 知らせそのものは、E より前のフレームで当てても、見える物を変えない。動くのは E より後の時刻だけで、動いた後も E 以後だから、E より前のフレームの時刻から見れば、動く前も後も「まだ来ていない」である。E で待たせる必要が在るのは、知らせの後ろに並ぶ物である。
  - 知らせが列の頭で待つのは、フレームの時刻が E に届くまでである。E は、置き直しを呼んだ時の現在の時刻なので、本番では、届いた時にもう過ぎている。最初の起点の知らせ（効く時刻が、先の時刻になりうる）は、列に入らない。

**どの受け手も、同じ時刻を持つ**: 文字の層と seriko は、同じ式（`cue.fire_time + chunk_reveal_offsets(..)`）で字の現れる時刻を積み、同じ決まりを、届いた順の同じ所で当てる。だから、同じ字は、同じ絶対の時刻を持つ（1 つのスコープの字が、2 つの場所へ重なって出る時だけは、「前の字」の取り方の違いで、今と同じに違いうる）。起点から時刻を求め直す所が、どの受け手にも無い。

**届き方に依らない**: 受け手 A が、合図と知らせを、それぞれの時刻に受け、受け手 B が、同じ物を、遅れて、まとめて受けたとする。どちらも、使うのは「届いた順」と「合図と知らせが運ぶ時刻」だけで、届いた時刻を使わない。同じフレームの時刻で比べると、そのフレームの時刻までに当たる物が両方に届いていれば、A と B は、同じ字が現れ、同じ指令が当たり、同じ時刻を持つ。B が全部を受けた後は、どのフレームの時刻でも同じである。口パクは、「どれだけ厳しく時刻に合わせるか」のとおり、刻みを済ませる前に合図と知らせが届いていれば、同じである。

**知らせが、効く時刻より後に着いた時の限界**: 知らせは、置き直しを呼んだスレッドから受け手へ運ばれるので、効く時刻 E より後のフレームを回してから着く事が在る（本番では、フレーム 1 つ分）。受け手は、着くまで、置き直しを知らない。

- 前へ動かす知らせ: 着いた次のフレームで、時刻の来た字が一度に現れ、後ろの合図が順に当たる。遅れるだけで、戻る物は無い。
- 後ろへ動かす知らせ: E より後で、着く前の最後のフレームの時刻以前の字は、着く前のフレームが、もう見せている。知らせを当てると、その字の時刻は d だけ後ろへ動き、フレームの時刻がそこへ届くまで、見えなくなる。約束するのは、次の事である。
  - 知らせを当てた後のフレームは、どれも、時刻どおりに受けた受け手と同じ字を見せる（どこまで見えるかは、時刻だけで決まる値である＝D22）。
  - 食い違うのは、E から、知らせが着くまでの間のフレームだけである（その間は、着く前の時刻のまま見せる）。見せてから隠れる字は、E より後で、着く前の最後のフレームの時刻以前の字に限る。
  - 「見せた字は隠さない」決まりにはしない。そうすると、見せた字は、動いた長さ d の間ずっと、時刻どおりに受けた受け手より先に出たままになり、食い違いが、届く遅れでなく d で決まる。
- バルーンの寿命の時計: 表示の終わりは、トークが終わるまで判断に使わないので、遅れて着いても、判断は変わらない。
- 口パク: 「どれだけ厳しく時刻に合わせるか」の 2 のとおり（違いは、動いた長さ d で決まる）。

**前の再生と、次の再生の間**: 次の台本の物は、その合図の発火時刻が来るまで、どの受け手にも効かない。それまでのフレームは、前の再生の姿を見せる。前の再生の、まだ現れていなかった字（中断された台詞の残り）は、文字の層でも seriko でも、元の絶対の時刻に現れる。次の台本が、内容の在る台本なら、頭の全部の消去が、その発火時刻に残りを消す。合図の無い台本は、再生機を作らず、知らせも合図も出ない（`drive.rs` の、空の台本の分かれ道）。頭の全部の消去を持たないのは、後始末の合図だけの台本で、その時は、前の再生の残りの字が、元の時刻のまま現れ続ける（今と同じ）。

**列が待つのは、頭の物だけである**: 列は順を変えないので、頭の物の時刻が来るまで、後ろの物は当たらない。列に入る時刻は、合図の発火時刻・置き直しの効く時刻・現実の出来事の時計の読みで、どれも、本物の時計では、送った時より後にならない。だから、頭の物は、届いた時にはもう時刻が来ているか、フレーム 1 つの内に来る。起点を先の時刻に置いた台本は、起点が来るまで合図を配らないので、列に何も入れない。その台本を、起点が来る前に置き直しても・捨てても、列の頭に、待つ物は残らない。

### 手で組む合図の時刻

再生機を通さずに、本番のコードが組む合図と知らせ。どれも、届いた時に時計を読ませず、送り手が、設計の上で決まる時刻を付ける。

| 所 | 何か | 付ける時刻 | 理由 |
|---|---|---|---|
| `areka-runtime` の `shell_carry.rs`（面と着せ替えの出し直し） | ghost の読み直しの後、フレームの中の装着の処理が、seriko の受け口へ直に送る合図 | 出し直しを行うフレームの時刻（`FrameTime`）。`TalkCue::scheduled(フレームの時刻, 0, ..)` | 台本の合図ではなく、そのフレームが起こす出来事である。そのフレームが見せる物は、フレームの時刻で決まる |
| `areka-seriko` の `settle.rs`（書き出しの進め方） | 画面の外で、決まった時刻まで seriko を進めるための合図 | 進め方の自分の時刻の軸の 0（`TimePoint::ZERO`） | 進め方は、0 から始まる自分の軸を持ち、刻みもその軸で渡す。本物の時計とは関わらない |
| `areka-runtime` の `frame/status_report.rs` → `SerikoSink::send_stage`（窓の知らせ） | 表示の層の窓の事実（開いた・閉じた・面・世代） | それを見たフレームの時刻（`FrameTime`） | 合図ではないが、seriko が出来事の時刻として使う。事実の時刻は、それを見たフレームの時刻である |

- seriko は起点を持たないので、手で組んだ合図に、起点の知らせは要らない（`fire_time` だけを使う）。

### 使い手の居ない口の確かめ

起点の置き直し・知らせ・絶対の発火時刻は、この spec の中に本番の使い手が居ない。口の形を、`doc/ideas/novel-redesign-2026-11-20/` が書く使い方に 1 つずつ当てた。

| 使い方（出どころ） | 文書が求める事 | この spec の口での書き方 | 後の spec が足す物 |
|---|---|---|---|
| 字を一度に出す・待ちを飛ばす（`00-architecture.md` 4.1・`01-playback-control.md` 5.2） | 字か待ちの項目を取り出すたびに、その長さだけ起点を前へ動かし、同じ刻みで後ろの項目を続けて取り出す。項目は落とさず、順も `at` も長さも変えない | `step(now)` → `shift_origin(now, −長さ)` → `step(now)` …。合図ごとの発火時刻は、取り出した時の起点で決まる。知らせの効く時刻は、どれも同じ `now` で、受け手は、同じ時刻の知らせを届いた順に当てる | 速さの状態と、それを読んで `shift_origin` を呼ぶ所 |
| 早送り（`01` 5.2 の `advance`） | まだ配っていない最初の項目の台本の時刻が「今」になるように起点を置く | `reanchor(now, now, その項目の台本の時刻)` → `tick(now)` | 次の項目の台本の時刻を返す照会（読むだけの口） |
| `\x` の続き（`01` 5.6） | 解く時に、区切りの台本の時刻が「今」になるように起点を置く。待った長さだけ、後ろの合図が後ろへずれる | `reanchor(now, now, 区切りの台本の時刻)` → `resolve_click()` → `tick(now)`。区切りで止まっている間の置き直しを受け付ける。解く口は時刻を受けず、配る事と完了は、続く `tick(now)` が決める | 区切りの台本の時刻を返す照会（読むだけの口）・待ちの始まりと終わりの知らせ |
| 返事を待つ区切り（`01` 5.7） | 区切りを解かずに、起点だけを置き直す | 区切りで止まったまま `reanchor` | 区切りの種類 |
| 音の終わりを待つ（`04-sound.md` 5.4） | 起点を「終わった時刻 − 区切りの台本の時刻」へ付け替える | `reanchor(now, 終わった時刻, 区切りの台本の時刻)`。起点を決める時刻（`at`）は「今」でなくてよい。知らせの効く時刻は、置き直しを呼んだ「今」である（音が終わったと分かった時から効く） | — |
| 並走の流れの起点（`02-parallel.md` 6.6） | 目印の合図が配られた時刻を、流れの起点にする。主の流れの起点が同じ刻みで動いても狂わない | 目印の合図の `fire_time` を、流れの `TimedSchedule::with_horizon(fire_time, ..)` の起点にする。流れの受け口への最初の起点は、流れを進める所が `OriginNotice::first(fire_time)` で知らせる | 流れの集まりと、流れを進める所 |
| 並走の流れの待ち（`02` 6.10） | 流れの残りの時刻を、止まっていた長さだけ後ろへ付け替える | 流れの `TimedSchedule::shift_origin(now, +長さ)`。置き直しは時刻の表の口なので、再生機を通さずに流れを進める所も呼べる。返った `OriginNotice` を、流れを進める所が受け口へ配る | 流れの複製の受け口は、その流れの置き直しの知らせを、主の流れの時刻を持つ受け手へ送らない。受け手は、持っている時刻を再生ごとに分けないので（「知らせの受け方」の節）、送ると、主の流れの字の時刻が動く。知らせの形は変えない |
| 主の流れと並走を同じ時刻で進める（`00` 4.3 の 4・`02` 6.5） | 刻みが絶対の時刻で、dispatcher が読む時計と同じ物差しである | `SakuraMsg::Tick(TimePoint)`・刻み役と同じ時計 | 次に届く時刻を返す照会（読むだけの口） |
| 値を時間で変える（`07-timed-values.md` 5.9） | 合図が絶対の発火時刻を運び、受け口は見積もらない。終わりは「発火時刻＋長さ」。フレームの時刻と同じ物差し | `cue.fire_time`・`TimePoint + TimeSpan`・`DolaRuntime::start(name, start_time: TimePoint)`（コンパイルの結果は開始の時刻に依らないので、合図の発火時刻を、そのまま開始の時刻に渡せる）・`FrameTime(TimePoint)` | 変化を始める・終わりの値にする・止める・値を読む口。その中身を、どれだけ厳しく時刻に合わせるかの決まり |
| もう配った字が付いて来る（`00` 4.3 の 1） | 文字の層の時計・口パク・バルーンの寿命の時計が、置き直した起点に付いて来る | `CueSink::origin_set(OriginNotice)`。効く時刻より前に現れた字は現れたままで、その後の字が、新しい起点に付いて来る | 早送りが済んだ事の知らせ（既定が何もしない口を足す） |
| 音・舞台など、新しい種類の中身 | 中身が、届いた時でなく、決まった時刻に効く | 合図の `fire_time` と、起点の知らせ。フレームで評価する受け手は `areka_sakura::held` の列を使える | その種類の、追い付き方と、時刻に合わせる厳しさの決まり |
| 選択待ちの間に起点を置き直す | 起点が動くと占有の終わり（表示の終わり）も動く。kanade は、選択待ちに入る時に表示の終わり（絶対の時刻）を 1 度受けるだけで、受け口ではないので、置き直しの知らせ（`origin_set`）は kanade に着かない。選択待ちの期限は、前の表示の終わりのまま残る | 置き直しの後の表示の終わりは `player.occupancy_horizon()` で読める。dola の口は足さない | 選択待ちの間に起点を置き直す spec が、新しい表示の終わりを kanade へ送る（areka の中の知らせ＝トークのスレッド → dispatcher → kanade。dola の口は変えない） |

- 取り出しごとの置き直しを、再生機の外の呼び手が `step` と `shift_origin` で書ける。速さの決まりを時刻の表の中へ入れる形にも、同じ 2 つの口の上で書ける。
- 後の spec が足す物は、読むだけの照会・既定が何もしない知らせの口・区切りと項目の種類で、この spec の口の形を変えない。
- 文書どうしで食い違う所（値を時間で変える変化の始まりを、呼んだフレームの時刻にするか合図の発火時刻にするか）は、どちらの案も `fire_time` と `FrameTime` が同じ物差しである事だけを求める。

## Data Models

### ログの形

- 時刻と長さを出す欄は、`Display`（秒の小数・小数点の下 7 桁）で出す。丸めた値だけを出す欄は残さない。
- 欄の名前から、単位の語を外す。

| 所 | 今の欄 | 後の欄 |
|---|---|---|
| `areka-ghost` の `sink.rs`・`areka-emo-text` の `state.rs`／`sink.rs`・`areka-seriko` の `actor.rs`・`areka-sakura` の `drive.rs` | `at`（秒の実数） | `at`（`Display`）と、合図を出す所は `fire_time` |
| `areka-ghost` の `dispatcher.rs` | 表示の終わりの秒 | `display_end` |
| `areka-kanade` の `schedule/*` | `now_ms`・`deadline_ms`・`display_end_ms`・`overshoot_ms` | `now`・`deadline`・`display_end`・`overshoot` |
| `areka-seriko`・`areka-ghost` の刻みのログ | `now_ms` ほか | `now` ほか |

- 欄の字面を読むテストは、`FromStr` で型へ読んで比べる。`tools/` と `doc/` に、これらの欄を字面で読む道具は無い（替える前に、もう 1 度検索して確かめる）。
- `areka-input` の入力の時刻の欄（`Instant` 由来）は範囲の外で、触らない。

## Error Handling

### Error Strategy

| 起きる事 | 扱い |
|---|---|
| 作者の値が長さとして表せない（有限でない・負・範囲の外） | 変える口が `Err(TimeRangeError)` を返す。storyboard の文書は読み込みの失敗。台本の `\w` は 0 以上の整数のミリ秒なので起きない。wintf の作者の秒は 0 として扱い `warn!` |
| `std::time::Duration` に 100ns より細かい端数が在る | 失敗にしない。最も近い 100ns へ丸める。失敗は、範囲の外だけ |
| storyboard の文書の時間の値が、文書の値の上限（絶対値が 2^51 未満）を越える | 読む時は読み込みの失敗。検査は `DolaError::TimeOutOfDocumentRange`（`DolaDocumentBuilder::build` と `compile_storyboard` が拒む）。書く時は書き出しの失敗 |
| 選択待ちの時間切れのミリ秒が端を越える | 端へ張り付ける（今と同じ） |
| 直列化した `CueSheet` の値が表せない | 読み込みの失敗（`serde` の誤り） |
| 台本の起点が押されていない | `CuePlayer::from_sheet` が `Err`。呼び手は `error!` を出して、その台詞を中断で終える |
| 足し引きが型の範囲を越える | 張り付く。周回を延ばせない時は、周回を終える。1 周の長さが表せない時は、コンパイルで拒む |
| 止めた再生を `transition` で動かそうとした | `Err`（`pause`／`resume` を通す） |
| 止めている間に、終わりの期限（`finish`）が来た | 失敗ではない。その刻みで再生を終え、値は終わりの値になる（「止めて戻す」の節） |
| 同じ時刻・前の時刻の刻みが、トークのスレッドに届いた | 失敗ではない。再生機へ渡し、新しく届いた項目が無ければ何も起きない（後半。ログは出さない） |
| 完了の後の置き直し | 何もせず `None`。呼び手が、完了しているか（`is_completed()`）を見て、ログを出す |
| 起点が変わらない置き直し | 失敗ではない。何も変えず、知らせも出さず、`None` |
| 区切りで止まっていないのに、解く口を呼んだ | 今と同じ（`resolve_click` は何もしない・`resolve_choice` は `None`）。解く口は、完了を決めない |
| 時刻の来ていない物が、受け手の列に残った・まだ現れていない字が在る | 失敗ではない。その時刻に、次のフレームを起こす予約を置く |
| 置き直しの知らせが、効く時刻より後のフレームを回してから着いた | 失敗ではない。着いた後のフレームから、動いた後の時刻で判定する（「知らせの受け方」の節の限界） |
| UI のスレッドで文字の層が借りられず、届いた物を列に足せなかった | 今と同じ。`Err` を返し、基盤が `error!` を出して続ける（その物は失われる） |
| 手で組む合図・窓の知らせを送る時に、`FrameTime` が世界に無い | `error!` を出して、送らない（wintf は、世界を作る時に必ず置く） |
| 文字の層が借りられないフレーム（ほかの処理が持っている） | 進める相・文字の拡大率の相・文字の相は、そのフレームの仕事を見送り、`error!` を出し（同じ縮みが続く間は 1 回）、次の画面の更新を予約する。可視性の相は、今の決まりのとおり、そのフレームの観測を見送る。列の物は残り、次のフレームで当たる |
| フレームの時刻が無い世界で、進める相が回った | 進めず、次の画面の更新を予約する（本番では起きない） |

- パニックは足さない。ログ無しで失敗を捨てる道は作らない。
- 型で表せなくなった入力（NaN・無限大）のための本番の分かれ道と、そこでだけ出していたログは、分かれ道ごと取り除き、記録に書く。

## Testing Strategy

テストは、時刻を外から渡して決定論で回す。本物の時計を読むのは、時計そのもののテストだけ。偽の時計は、観測を追い越さない（テストが時計を進めてから刻む）。

### Unit Tests

- **型と丸め**（1.3〜1.6・3.2〜3.4・3.8）: 足し引きの結果の型・端での張り付きと `checked_*` の `None`・`from_millis` のちょうどの値・秒からの丸め（1 つ手前・ちょうど中間・1 つ先）・有限でない値と負の `Err`・符号つきのミリ秒の端の張り付き・`TryFrom<Duration>`（100ns の倍数はちょうど・49ns と 50ns と 51ns の端数の丸め・範囲の外だけが `Err`）・`TimeSpan::try_from_ticks`（0 と正は作れる・負は `Err`）。
- **倍率の写し**（5.6・5.11・3.6）:
  - 食い違わない事: いくつもの倍率（1・0.5・2・1/3・1e-6・1e6）と台本の時刻について、`G(s) − 1`・`G(s)`・`G(s) + 1` の 3 点で「`e ≥ G(s)`」と「`F(e) ≥ s`」が同じである事。F が減らない事。倍率 1 が恒等である事。届かない時の `None`。
  - 届く前に「届いた」にならない事（決まった期待の値）: 倍率 0.1 で `G(1) = 10`・`G(1,000,000) = 10,000,000`。倍率 0.25 で `G(1,000,000) = 4,000,000`・`G(1,000,001) = 4,000,004`。倍率 0.3 で `G(3) = 10`・`G(300,000) = 1,000,000`・`G(1,000,000) = 3,333,334`。倍率 0.5 で `G(500,000) = 1,000,000`・`G(1,000,001) = 2,000,002`。倍率 2 で `G(1,000,000) = 500,000`・`G(1,000,001) = 500,001`。倍率 3 で `G(3) = 1`・`G(1,000,000) = 333,334`。
  - `G(0) = 0`: 1 より小さい倍率（0.5・0.25・0.1・0.3・1e-6）と、1 より大きい倍率（2・3）で。
  - 開始より前の刻み（経った長さが負）: 1 より小さい倍率で `F(−1) = −1`、倍率 2 で `F(−1) = −2`。どの倍率でも、`F(e) ≥ 0` は `e ≥ 0` の時だけ。
  - 下限の境: 倍率 0.5 で全体の長さ 500,000 の終わらないループは、1 周が 1,000,000 で始まる。全体の長さ 499,999 は、1 周が 999,998 で拒まれる。全体の長さ 0 は `ZeroDurationWithLoop`。
  - 1 周の長さが負にならない事: 0 以上の全体の長さと、上の倍率の全部で、`lap_length` が `Some(0 以上)`。
- **時計**（2.2・2.4）: 続けて読んだ値が戻らない事・OS の関数の値と同じ物差しである事（前後で挟む）。
- **文字の形**（7.8）: `Display` と `FromStr` の往復が、端の値を含めて等しい事。
- **時刻の表**（4.4・11.2〜11.12・13.1・13.4・13.6）: ちょうどの刻みで届く・起点より前の刻みで届かない・置き直しの後に新しい起点で届く・1 件ずつ取り出して間で置き直す・区切りの間の置き直し・同じ時刻の 2 度目（置き直し無しは 0 件・前への置き直しの後は新しい分だけ）・何度置き直しても落とさず 2 度届けず順を保つ・下の端を越える置き直しで結果が変わらない・遅れた刻みでも発火時刻が「起点＋相対」・置き直しが返す知らせの中身（効く時刻は渡した現在の時刻・新しい起点・前の起点）・起点が変わらない置き直しは `None`。
- **届いた物を持つ列**（12.4・12.10・18.1・18.4）: 頭の物の時刻が来るまで、後ろの物を出さない（後ろの物の時刻が、先に来ていても）・時刻がフレームの時刻以前の物が、届いた順に出る・ちょうどの時刻で出る・時刻を持たない物は、前の物が出た同じ呼び出しで出る・列の頭に在る、時刻を持たない物は、すぐ出る・`waiting_until` が、待っている頭の物の時刻を返し、空なら `None` を返す・同じ物の列を、1 つずつ入れても、まとめて入れても、同じフレームの時刻で、同じ物が同じ順に出る。
- **コンパイルの結果**（5.2・5.4）: 区間と引き金の時刻が、storyboard の頭からの隔たりである事。同じコンパイルの結果を、違う開始の時刻で始めても、値と引き金が、開始の時刻の差だけずれて同じである事。
- **終わりの予定の時刻**（5.5）: 1 回だけの再生・待ちの無い有限のループ（最後の周回の終わり）・待ちが決まった値の有限のループ・待ちを引く有限のループ（`None`。最後の周回に入った後の `resume` は、終わりの時刻を返す）・終わらないループ（`None`）。止めて戻した後の値が、止めていた長さだけ後ろへずれる事。終わりの期限を置いた後の `resume` が、期限と終わりの時刻の早い方を返す事（終わらないループは、期限）。
- **止めて戻す**（10.1〜10.5）: 戻した直後の値が止めた時点の値と等しい・止めている間はどの時刻でも同じ値・止めた時刻より前で戻しても続く・`transition` で止める／戻す道が `Err`・止めている間に終わりの期限が来たら、その刻みで終わりの値になり、期限の前は止めた時点の値のまま。
- **1 字ごとの間隔**（7.10・9.8）: 今の式を有理数で誤差なく計算した値と比べる。割り切れない長さ（100 ms を 3 字。頭から始まる塊は 0・333,333・666,667、続く塊は 0・333,334・666,667）・ちょうど中間（長さ 10,000 を 32 字。頭から始まる塊の最後の字は 9,688、続く塊は 0・313・625、`p = 0` なら 312・625）・続く塊の最後の字が `p + duration` ちょうど・前の字が遠くて頭から始まる塊（`p × count + duration < 0`）・長さ 0 の塊・字数 0 の塊（何も返さない）。塊を 3 つ以上続けた列（長さと字数を変えて）で、どの字の差も 100ns 未満で、隔たりが減らない事。割り切れる長さ（字数 × 50 ms）で、今の値と等しい事。
- **口パク**（12.7・18.6）: 1 回の刻みで区切りを 3 つ越えた時の再生の始まりが、1 つずつ越えた時と同じ・同じ時刻の字では 1 回・再生中の区切りは見送る。確かめるのは、刻みを済ませる前に合図と知らせが届いている時の一致までで、合図や知らせが、その時刻の刻みより後に届いた時の一致は、テストにしない。
- **seriko の、姿を変える指令**（18.1・18.6・18.7）: 面の切り替えの合図を、発火時刻のすぐ後に処理しても、何刻みか後に処理しても、`always` と `runonce` のコマ（発火時刻からの経った長さで決まる）と、口パクの数え始め（発火時刻までに現れた字の数）が同じ。発火時刻までの字の区切りが、替える前の面で先に判定される。着せ替え・バルーンの面・手で組んだ合図・窓の知らせも、渡した時刻が出来事の時刻になる。
- **seriko の知らせ**（12.6）: 効く時刻以前の字は動かない・後の字は動いた長さだけ動く・前へ動いて効く時刻より前になる字は、効く時刻になる・続けて 2 回（後ろへ動かして、前へ戻す）・字の列を刈り込んだ後の「前の字の時刻」の欄も、同じ決まりで動く（知らせの後に届いた塊の字が、文字の層と同じ時刻になる）・受け口は、最初の起点の知らせを送らない。
- **文字の層の、時刻の来た合図を当てる口**（12.10・12.13・18.1・18.4・18.6）: 発火時刻が 3 秒後の消去が、それより前のフレームでは効かず、フレームの時刻が発火時刻に届いたフレームで効く（改行・カーソル・書体・アンカー・印も同じ）・届いたフレームの時刻より後の発火時刻を持つ合図が、そのフレームでは当たらず、`advance` が、その時刻を返す・同じ時刻の合図が台本の順に当たる・遅れて届いた合図が、次のフレームで順に全部当たる・1 字 50 ms の塊が 250 ms 遅れて届くと、過ぎた字が一度に現れ、残りが元の間隔で現れる・先渡しが、前の台詞の合図の後・その台詞の最初の合図の前に当たり、区間の番号がずれない・トークの終わりの印が、選択肢の合図の後に当たる・列が空の時に届いた先渡しと印が、その場で当たる・画面に出た選択肢への押下が、選択待ちの始まりの印の後なら、選択肢を済みにする。
- **文字の層の、置き直しの知らせ**（12.4・12.5）: 効く時刻以前の字は動かない・後の字は動いた長さだけ動く・前へ動いて効く時刻より前になる字は、効く時刻に現れる・知らせより前に届いて、まだ当たっていない合図の字も、当てた後に動く（合図を先に当てた時と、同じ時刻になる）・知らせの後に届いた合図が、効く時刻より前のフレームで当たらない・受け口は、最初の起点の知らせを送らない。
- **バルーンの寿命**（12.8・12.11・12.14・18.1）: 知らせが、フレームの時刻が知らせの時刻に届いたフレームで効く（その前のフレームでは効かない）・表示の終わりと期限が絶対の時刻で、フレームの時刻と比べて切れる・置き直しの知らせで、効く時刻より後の表示の終わりだけが動き、期限は動かない・受け口が、覚えている最大を同じ決まりで動かす（前へ動いた後の合図の終わりが送られる）・そのトークの最初の合図より前の置き直しの知らせは、送られない・利用者の中断が、取り出したフレームで効く・トークの終わりが、時計の読みを運ぶ。

### Integration Tests

- **付け替えで変わらない事**（9.1〜9.3）: 既存のテストの期待値を、ちょうどの 100ns の値へ写して通す。比べを緩めない。連続の値の許す幅は広げない。
- **直列化の往復**（6.1〜6.8）: `CueSheet` の JSON と TOML（機能 `toml`）・起点の押されていない台本・負の時間切れ・整数でない値と負の長さの失敗・storyboard の文書の秒の往復（読む → 書く → 読むで等しい）・文書の値の上限（2^51 − 1 は往復する・2^51 は、読む・検査・書くの 3 つとも失敗・`StoryboardBuilder` で作った上限を越える文書が `DolaDocumentBuilder::build` と `compile_storyboard` で拒まれる）。
- **再生機**（12.1〜12.3・13.2〜13.7・18.3・18.4）: 受け口が、登録の時に、どの合図よりも前に、最初の起点（効く時刻は起点・前の起点は無い）を受ける・後から登録された受け口は、登録の時の起点を受ける・置き直しの前の合図 → 知らせ → 後の合図の順・知らせの効く時刻が、置き直しに渡した現在の時刻である・起点が変わらない置き直しでは、知らせが出ない・知らせの口を上書きしない受け口の動きが変わらない・起点が過ぎた台本で、過ぎた合図が全部、順に、それぞれの発火時刻で届く・起点が来ていない台本で何も届かない・包む受け口（`StrictTap`）が知らせを中へ通す・先渡しの後に置き直すと、`emit` の `fire_time` が新しい起点＋`at` で、先渡しで見せた値と置き直しの分だけ違う（置き直さなければ等しい）。
- **区切りを解く口**（2.8・18.1）: 解いただけでは、何も配られず、完了にもならない・解いた直後に、同じ時刻で `tick` を呼ぶと、区切りの後ろの届いた項目が配られ、占有の終わりを過ぎていれば、新しい刻みを待たずに完了する・トークのスレッド（`drive.rs`）を通して、選択を解いたその場で完了の知らせが出る（今のテストの、呼び手から見える期待のまま）。
- **絶対の刻み**（14.1〜14.7・11.9・11.10）: dispatcher が、刻みの時刻を変えずに送る（台詞ごとの値に依らない）・最初の刻みが起点になる・起点の置き直しが無い再生で、合図の届く刻みと順が変わらない・`ChoiceWaiting` の表示の終わりが換算無しで kanade へ着く・トークのスレッドへ同じ時刻と前の時刻の刻みを送っても、何も 2 度届かない（守りを取り除いた後も、今のテストの期待のまま）・トークのスレッドが、同じ時刻の刻みを再生機まで渡す（再生機の `tick` が、同じ時刻で 2 度呼ばれる）。
- **受け手の追従**（12.4〜12.14）:
  - 置き直しの知らせの後、まだ現れていない字が、動いた長さと向きのとおりに動く（効く時刻より前のフレームは、変わらない）・効く時刻以前に現れた字は、後ろへ動いても現れたまま・前へ動いて時刻の来ている字は、効く時刻のフレームで一度に現れる・文字の層と seriko が、同じ字に、同じ絶対の時刻を持つ。
  - バルーンの寿命が、文字の層と同じフレームの時刻で判断し、同じフレームの文字の層の状態から字の数を読む・寿命の線と文字の線の、片方にだけ先に届いたフレームで、可視性の相が読む字の数と、文字の相が描く字が同じである。
  - フレームの時刻が発火時刻より前の時に、その合図の字が現れず、指令も当たらない・合図の届く時刻を揺らしても、フレームが見せる物が変わらない・次の台本の合図の発火時刻に届く前のフレームは、前の再生の姿のままである。
  - 起点が先の時刻の台本を、最初の刻みの前に置き直して刻むと、文字の層・寿命の時計・seriko のどれも、起点の時刻を待たずに、置き直した後の発火時刻で当たる。
  - 中断された台詞の残りの字が、後始末の合図だけの台本が続いても、文字の層と seriko で、同じ絶対の時刻に現れる。
  - 後ろへ動かす知らせが、効く時刻より後のフレームを回してから着いた時、着いた後のフレームが、時刻どおりに受けた受け手と同じ字を見せる。
  - 起点が先の時刻の台本の先渡しが、起点より前に当たっても、画面に出ている前の再生の字の配置が変わらない。
  - 次のフレームの予約: 時刻の来ていない物が列に残ったフレームが、その時刻への予約を置く・寿命の受け口の旗でフレームが回り（文字の層の列は空で、何も予約されない）、その後で文字のメッセージが取り出されて列に入った時に、旗が立ち、次の画面の更新でフレームが回って、その合図が当たる・進める相が進められなかったフレーム（借りられない）が、落ちずに、`error!` を出して、次の画面の更新を予約し、次のフレームで列の物が当たる・まだ現れていない字が在る間、文字の相が、次に現れる字の時刻への予約を置き、字が現れ切ったら置かない。
  - 旗を立てる所の一覧: 本体の検査（`AREKA_PRODUCERS` と、木の走査）が、`areka-emo-text/actor_held.rs` を含めて通る。
- **どこで再生しても同じ**（18.5・18.7）: 同じ台本・同じ起点・同じ時刻の列を、2 組の受け手へ、届く順と届く時刻を変えて与える（片方は、それぞれの時刻に・片方は、遅れてまとめて）。台本には、字・消去と改行・面の切り替え・待ちの指定を入れる。同じ観測の時刻で、字の現れた数・当たった指令（文字の層の状態）・口パクの再生の始まり・面の切り替えで始まったアニメのコマ・バルーンの寿命の判断が等しい（口パクは、刻みを済ませる前に合図と知らせが届いている列で比べる）。
  - 置き直しをはさむ列を足す: 途中で、前へ動かす置き直し（まだ現れていない字の一部が、効く時刻に一度に現れる）と、後ろへ動かす置き直し（もう現れた字は現れたまま・残りが遅れる）を 1 回ずつ入れる。片方の受け手は、合図と知らせをそれぞれの時刻に受け、片方は、全部を遅れてまとめて受ける。全部を受けた後の観測の時刻のどれでも、上の物が等しい。文字の層とバルーンの寿命は、途中の観測の時刻（効く時刻の前と後）でも等しい。
  - 文字の層は、フレームの時刻が合図の発火時刻より前の観測（合図が先に届いている）でも、その合図の中身を見せない。
- **眠り**（17.6・17.10）: 偽の時計を止めている間に、kanade の締め切り・台本の再生・seriko のコマが進まない。時計を動かすと、止めた所から続く。
- **`OnSecondChange`**（17.7）: 刻みの時刻から求めた時（hour）の値。
- **Windows でない target**（1.8・2.7・15.8）: 全体テストの段 `dola non-windows check`。

### 締めの検査

- **公開の口に `f64` の秒が無い**（15.4）: dola（`dola::time` を含む）の `pub` の欄・引数・戻り値に出る `f64` を全部挙げ、次の物だけである事を記録する: 倍率（`time_scale`・`TimeScale`）・進み具合と比（`ratio_of`・`mul_unit`）・時間で変わる値そのもの・イージングの引数・作者が書く単位から変える口の引数（`try_from_secs_f64` の `secs`）。秒の実数を返す `pub` の口（戻り値と欄）は 0 件である（`as_secs_f64` の名前の口が、dola に無い事も検索で確かめる）。
- **残っていない物の検索**（2.6・8.4・15.5・17.2）: `sum_on_ms_grid`・`runtime::clock`・`GetTickCount64`・`MonotonicMs`・`QueryPerformance`（dola の中）・`SerikoClock`・`ClockedTextSink`・`TalkEpoch`・`observe_cue`・`resolve_talk_time`・`now_talk_time`・`base_now`・`TimeUnknown`・`settle_completion_after_resolve`・`compile_storyboard` の 3 つ目の引数・実数の秒と型を行き来するだけの口。どれも 0 件。0 件と書く前に、検索の式が、在ると分かっている字に当たる事を確かめる。
- **届いた時に当てる所が残っていない事**（18.1）: 文字の層の、UI のスレッドで受け取る所が、合図と置き直しの知らせを、状態を当てる関数へ直に渡していない事（時刻を持たない物を、列が空の時に当てる所を除く）・seriko が時計を読む所が無い事を、検索で確かめる。
- **受け手が起点を持たない事**（12.9・18.2）: 文字の層・バルーンの寿命の時計・seriko・`TalkClock` に、起点を持つ欄と、台本の時刻を受ける・返す口（`talk_time` の名前の引数を含む）が無い事を、検索で確かめる。
- 結果は `migration-ledger.md` に書く。

## Migration Strategy

設計が持つ段取り。影響はワークスペースの全体に及ぶので、並行のサブエージェントで進める。

### コンパイルの順

各 `Cargo.toml` で確かめた辺（下が先）:

```mermaid
graph TB
    S0[段0 dola の時刻のモジュール]
    S1[段1 dola の付け替え]
    S2a[段2 wintf]
    S2b[段2 areka-talk と areka-sakura]
    S2c[段2 areka-kanade]
    S3a[段3 areka-seriko]
    S3b[段3 areka-emo-text]
    S3c[段3 areka-ghost]
    S4[段4 areka-runtime]
    S5[段5 areka-input と areka-session と areka と先進坑]
    S6[段6 前半の締め 全体テスト1回目]
    S7[段7 後半 振る舞い]
    S8[段8 文書と締めの検査]
    S0 --> S1
    S1 --> S2a
    S1 --> S2b
    S1 --> S2c
    S2b --> S2c
    S2b --> S3a
    S2a --> S3b
    S2b --> S3b
    S2b --> S3c
    S2c --> S3c
    S3a --> S4
    S3b --> S4
    S3c --> S4
    S4 --> S5
    S5 --> S6
    S6 --> S7
    S7 --> S8
```

- 確かめ直した辺（各 `Cargo.toml` と、この設計で足す 3 本）: `wintf` → dola。`areka-talk` → dola（足す）。`areka-sakura` → dola・`areka-talk`。`areka-kanade` → `areka-talk`・dola（足す）。`areka-seriko` → dola・`areka-sakura`。`areka-emo-text` → `areka-sakura`・dola・`wintf`。`areka-ghost` → `areka-talk`・`areka-kanade`・`areka-sakura`・dola（本番の依存へ足す）。dola は何も引かないので、足した 3 本で輪はできず、順は変わらない。
- 段 0 は、dola の中に時刻のモジュールを足すだけである（古い時計と `f64` の口は残る）。ワークスペースは緑のままで、ほかの crate は 1 行も変わらない。
- `areka-kanade` と `areka-talk` が使うのは `dola::time` だけなので、編集は段 0 の後に始められる。組んで確かめるのは、dola の全体が組める段 1 の後である（dola は 1 つの crate なので、付け替えの途中は組めない）。
- `wintf`・`areka-talk`＋`areka-sakura` は互いに独立。`areka-kanade` は `areka-talk` の後。`areka-seriko`・`areka-emo-text`・`areka-ghost` は互いに独立。`areka-runtime` より上は 1 本の列。
- `areka-sakura` の `contract.rs` が dola の cue の型を再公開しているので、`areka-sakura` より上の crate は、形の替わる時期を crate ごとにずらせない。

### 継ぎを置かない・前半と後半

- **継ぎを置かない**: 実数の秒から型へ黙って変わる仮の変換は、置かない。そういう変換が在ると、まだ替えていない呼び出しが丸めつきで通り、替え残しがコンパイルの誤りに出ない。置かなければ、要件 15 の 5 は「置かなかった」で満たせる。
- **前半（型の付け替え）**: 下の crate から上へ、型を替える。dola の口と、crate をまたいで運ぶ合図・刻み・選択待ちの知らせの形は、前半で最後の形まで替える（`TalkCue` の `fire_time`・`SakuraMsg::Tick(TimePoint)`・`ChoiceWaiting` の欄・`CueSheet` の起点の `Option`）。値の意味と振る舞いは変えない: dispatcher は今の「台詞の最初の刻みからの隔たり」を送り、`TalkClock` と seriko は今の見積もりのまま、文字の層は届いた合図をその場で当てるまま、seriko は合図を処理する時に時計を読むまま、時刻の表は今の `tick`＋`ready` のまま、`fire_time` は台本の起点＋`at` を入れる。storyboard の層の口の形（`compile_storyboard` から開始の時刻を外す事・終わりの予定の時刻の意味）は、前半で最後の形にする（全部の呼び出しを、前半で 1 度だけ書き直す）。
  - **前半の間、時刻の型に、時計の物差しでない値が入る所**: dispatcher が送る刻みは「台詞の最初の刻みからの隔たり」のままなので、`SakuraMsg::Tick(TimePoint)`・台本の起点（0）・合図の `fire_time`（`at` と同じ値）・`ChoiceWaiting.display_end: TimePoint` は、時刻の型に、その隔たりを入れて運ぶ。型の意味（時計と同じ物差しの時刻）と値が合わないのは、前半の間だけである。後半の 2 で、値が時計の読みになり、dispatcher の `base_now` は消える（締めの検索で確かめる）。
  - **後半でもう一度替わる口**: 受け手の側の口は、前半では、今の意味（台本の時刻・見積もりの起点・seriko の時計）のまま型だけを替え、後半で、もう一度替わる。当たるのは、文字の層の `present_frame`・`visible_glyphs`・`balloon_shown_glyphs`・`RevealSchedule`・`run_text_phase` の時刻の引数、`TalkLifecycleSignal` の形、`TalkClock` の口、`SerikoClock` と `spawn_seriko_clocked`、`SerikoMsg::Stage` と `send_stage`、`hover_inject` の時刻である。これらを名指すテストのファイルは、前半と後半の 2 度、触る（後半の見込みに入れてある）。
- **後半（振る舞い）**: 止めて戻す・時刻の表の取り出しと置き直し（表の項目を `PendingCue` へ替える事を含む）・起点の知らせ（最初の起点と置き直し）・取り出す時の発火時刻・区切りを解く口を時刻に依らなくする事・トークのスレッドの同じ時刻の守りを取り除く事・絶対の刻み・受け手が、受け取った物を絶対の時刻で持つ事（起点の見積もりをやめ、起点を持たない）・届いた物を時刻が来るまで持つ列（文字の層とバルーンの寿命）・置き直しの知らせの受け方・フレームの頭で文字の層を進める事・受け手の列と、まだ現れていない字のための、次のフレームの予約・seriko の出来事の時刻を発火時刻にする事と、時計を外す事・手で組む合図と窓の知らせの時刻・口パクの決まり。
- **分ける理由**: 要件 9 の 1（付け替えで結果が変わらない）と 9 の 6（変えた所の前後の期待）を、別々の確かめで示すためである。前半の終わりの全体テストは「ミリ秒の格子に載る入力と、表せる入力について、同じ入力から同じ結果」を示し、後半のテストは、変えた所を名指しで示す。
- **前半の全体テストが示す事と、示さない事**: 前半の全体テストは、既存のテストの期待を、ちょうどの 100ns の値へ写して通す。示すのは、既存のテストが使う入力（ミリ秒の格子に載る時刻・表せる値）について、結果が変わらない事である。下の「前半に入る、振る舞いの変化」は、型を替えると一緒に入る変化で、前半の全体テストが「変わらない」と示す範囲の外に在る。どれも、変える前の期待と変えた後の期待を、テストの名前で記録に名指しする（記録の 3）。
- **前半に入る、振る舞いの変化**（型と一緒に入り、後半へ分けられない物）:
  1. **時計の出どころ**: 刻み役と seriko の時計を、`GetTickCount64` から 1 本の時計へ替える。ミリ秒の値を 10,000 倍して型に載せる仮の形は、継ぎなので置かない。決定論のテストは偽の時計で回るので、結果は変わらない。本番で変わる事は、眠りの間を数えない事・`OnSecondChange` の値・分解能・刻みの境の格子の 0 の位置・seriko のミリ秒への丸めが無くなる事である。
  2. **1 字ごとの間隔の式**: 実数の間隔を足し込む式から、字ごとに 1 回だけ丸める整数の式へ替わる。割り切れない長さでは、字の現れる時刻と口パクの時刻が 100ns 未満だけ動く。割り切れる長さ（本番の「字数 × 50 ms」）では動かない。
  3. **ミリ秒の格子に載らない時刻の足し算**: `sum_on_ms_grid` が消え、絶対の時刻は整数の足し算になる。格子に載る値どうしは今と同じ値で、載らない値は、f64 の和でなく、最も近い 100ns へ丸めた値どうしの和になる。
  4. **台本の起点の `Option` と、`from_sheet` の `Err`**: 起点の押されていない台本から再生機を作ると、今は起点 0 の台本として動く。前半から後は `Err(CueError::OriginNotStamped)` になる。
  5. **表せない値を直していた所が、失敗になる**: cue の長さの clamp（有限でない・負を 0 にする）が消え、そういう値は型で作れず、直列化した台本の読み込みでは失敗になる。選択待ちの時間切れの NaN と無限大の指令も、型で作れない。
  6. **storyboard の文書の、失敗の出る所**: 有限でない長さ・負の長さ・負の周回のずらしは、検査（`validate`）の誤り（`InvalidNumber`・`LoopOffsetNegativeMin`／`Max`）から、読み込みの失敗へ替わる。検査のその決まりは消える。
  7. **storyboard の文書の値の上限**: 絶対値が 2^51（約 7 年）以上の時間の値は、読む・検査・書くのどれでも拒む。今は読める。
  8. **storyboard の文書の秒の丸め**: 文書の秒は、読む縁で最も近い 100ns へ丸める。区間の境と引き金の時刻は 100ns の格子に載る。
  9. **倍率が 1 でない再生の、出来事の時刻**: 区間の境・引き金・周回の終わりは、切り捨ての写し F とその逆 G で決まる。今の f64 の割り算の値から、整数の刻みに載る分だけ動く（割り切れる時は動かない）。引き金の予定の時刻を周回の終わりで抑える継ぎは消える。
  10. **終わらないループの下限の境**: 1 周の長さと下限の比べが、整数でちょうどになる。今は f64 の割り算の結果で比べるので、境のごく近くの値は、f64 の丸めの向きで結果が決まっている。
  11. **終わりの予定の時刻の意味**: 開始と `resume` の戻り値は、決まっていれば、再生の全体の終わりの時刻。決まっていなければ `None`。今は、どのループでも、今の周回の終わりの時刻を返す。変わるのは、終わらないループ（`None` になる）・回数つきのループ（最後の周回の終わりになる）・待ちを引く回数つきのループ（`None` になる）である。1 回だけの再生は変わらない。
  12. **出来事の直前の時刻**: 「出来事の時刻より前に終わる周回まで進める」は、今は f64 の 1 つ手前の値（`next_down`）、後は 100ns 手前の時刻で行う。
  13. **wintf の作者の秒**: タイプライターの待ちと 1 字の待ちの既定が有限でない・負の時、長さ 0 として扱い `warn!` を出す（今は検査が無く、負は行程の時刻を戻し、NaN は後ろの全部を壊す）。
  14. **`Instruction::Wait` の 1 ms 未満**: 切り捨てる継ぎが消え、`Duration` の値を最も近い 100ns で受ける（台本の `\w` はミリ秒なので、本番の値は変わらない）。
  15. **ログの欄**: 時刻と長さの欄が、秒の小数 7 桁の字面と、単位の語の無い名前になる（「ログの形」の節）。
  16. **コンパイルの開始の時刻**: `compile_storyboard` の開始の時刻の引数と、`CompiledStoryboard.start_time` の欄が消える。runtime は今も 0 で呼んでいるので、再生の結果は変わらない。変わるのは、0 でない開始の時刻でコンパイルしていたテストの 5 か所だけである（「コンパイルの結果」の節）。
- 着手の時に、この一覧を数え直す。型を替える作業で、ここに無い振る舞いの変化が見つかったら、記録の 3 に足し、黙って通さない。

### 段ごとの中身と確かめ

タスクの数は、段取りを組むための見込みである。

| 段 | 中身 | 並べられる家族 | ワークスペース | その段で使える確かめ | 見込みのタスク |
|---|---|---|---|---|---|
| 0 | dola の時刻のモジュール（`src/time/*`・`src/lib.rs` の 1 行・dola の `Cargo.toml` の `windows` の機能）・全体テストの段 `dola non-windows check`。足すだけで、古い時計と `f64` の口は残す | 2（型と丸めと倍率と文字の形／時計とテストの組み立てと直列化のアダプタと検査の段） | 緑（足すだけ） | `cargo test -p dola`・`cargo test -p dola --features toml`（どちらも、古い口のテストごと緑）・`cargo check -p dola --features toml --target x86_64-unknown-linux-gnu` | 4 |
| 1 | dola の付け替え。土台（`src/error.rs` と `compile/types.rs` を含む。`Cargo.toml` から `Win32_System_Performance` を外す）と cue の本体を先に、その後に家族。`compile_storyboard` から開始の時刻を外す事と、終わりの予定の時刻の意味も、ここで入れる | 2 → 6 | ここから赤 | 本体は `cargo check -p dola --all-targets` の誤りの一覧で自分のファイルの分が 0 になる事。結合テストは入口ごとに `cargo test -p dola --test cue`（ほか `compile`・`general`・`runtime`・`trigger`・`validation`）。締めに `cargo test -p dola` と `cargo test -p dola --features toml` | 13〜15 |
| 2 | `wintf`（3 家族）・`areka-talk`＋`areka-sakura`（3 家族）・`areka-kanade`（3 家族。`areka-talk` の後） | 3 → 9 | 赤 | crate ごとに `cargo test -p <crate>`（下の段が緑になってから）。同じ crate の `src` のテストを別々の作業者が直す間は `cargo check -p <crate> --all-targets` | 9 |
| 3 | `areka-seriko`（4 家族）・`areka-emo-text`（4 家族）・`areka-ghost`（3 家族） | 3 → 11 | 赤 | 同上 | 11 |
| 4 | `areka-runtime`。土台と結線を先に、その後に 5 家族 | 1 → 5 | 赤 | 同上 | 7 |
| 5 | `areka-input` → `areka-session` → 本体（5 家族）・先進坑 | 1 → 5 | 本体が済めば緑 | `cargo test -p <crate>`。先進坑はそのワークスペースで `cargo check` | 5 |
| 6 | 前半の締め: 残っていない物の検索・記録・**全体テスト 1 回目** | 1 | 緑 | `pwsh -NoProfile -File tools/test-all.ps1 -Format` | 1 |
| 7 | 後半。順は下 | 2〜3 | 順の 1〜3 は緑のまま。4 は赤（`areka-emo-text`・`areka-seriko` の本体が口を替えてから、`areka-runtime`・`areka-input`・`areka-session`・本体の直しが済むまで）。5 は緑 | 緑の間は、要件ごとのテストと `cargo test -p <crate>`。4 の赤の間は、`cargo test -p areka-sakura`・`-p areka-emo-text`・`-p areka-seriko`（下の crate は、自分だけで緑になる）と、上の crate の `cargo check -p <crate> --all-targets` の誤りの一覧 | 20〜24 |
| 8 | 文書と締めの検査 | 2 | 緑 | 検索の結果を記録へ | 3 |
| 完了 | **全体テスト 2 回目** | — | 緑 | `/kiro-complete` の全体テスト（`tools/test-all.ps1 -Format -License`・アーカイブの後の 1 回） | — |

- **赤の間**（段 1 の始まりから段 5 の終わりまでと、段 7 の順の 4 の間）: `cargo build --workspace`・全体の clippy・`tools/test-all.ps1` は通らない。使える確かめは、crate ごとの `cargo test -p`（下の crate が緑なら組める）・`cargo check -p <crate> --all-targets`・`cargo fmt`。コミットの時に組む仕掛けは無いので、赤の間もブランチへコミットできる。
- **段の入口**: 上の段の作業者は、下の段が緑になる前から編集を始めてよい。確かめは、下の段の crate が `cargo test -p` で緑になってから。
- **組む確かめは 1 本ずつ**: 並行にできるのは編集で、`cargo` を回す確かめは同じ時に 1 本だけにする（同じ `target` の錠と、重いビルドを同時に走らせない決まり）。
- **全体テストは 2 回**: 段 6 と完了の時だけ。途中の確かめは crate ごとに絞る。

### 後半の順

1. **止めて戻す**（dola の中だけ・要件 10）と、**時刻の表と再生機**（`pop_due`・現在の時刻を受ける `reanchor` と `shift_origin`・`OriginNotice`・`step`・`origin_set` と、登録の時の最初の起点・取り出す時の発火時刻・時刻に依らない解く口・要件 11・12 の 1〜3・13）。2 つは別のファイルで、並べられる。時刻の表の `tick`＋`ready` を使う既存のテストは、同じ確かめのまま、取り出しを集める手助けを通す形へ直す（期待は変えない）。解く口だけで完了を期待していたテストは、解く → `tick` へ直す（記録の 3）。dola の外に、時刻の表の口を直に呼ぶ所は無い。受け手は、まだ知らせを読まないので（既定は何もしない）、ここまででワークスペースは緑である。
2. **絶対の刻み**（dispatcher・`drive.rs`・`ChoiceWaiting` の値の意味・要件 14・17 の 8）と、**`drive.rs` の同じ時刻の守りを取り除く事**（要件 11 の 9・10。1 の `pop_due` の後でなければならない）。受け手が見積もりのままでも、テストは緑である。
3. **受け手の列の型**（`areka-sakura` の `held.rs` と、その単体テスト・要件 12 の 4・18 の 1）。足すだけで、4 の 2 つの家族が使う。時刻の型だけを使うので、1・2 と並べられる。
4. **絶対の時刻で持つ受け手**。3 つは別の crate で、編集は並べられる。1・2・3 の後でなければならない（刻みが相対のままだと、合図の `fire_time` は時計の物差しに載らず、フレームの時刻と比べられない）。ここは赤になる。文字の層と seriko の本体が口を替えてから、結線と、その上の crate の直しが済むまで、上の crate は組めない。
   - 文字の層（`areka-emo-text`）: 届いた物を列に持ち、`advance` で当てる。字の現れる時刻を絶対の時刻で積む。提示と読み口が、フレームの時刻を受ける。受け口が、置き直しの知らせを送る。
   - seriko（`areka-seriko`）: 字の現れる時刻を絶対の時刻で持つ・置き直しの知らせ・姿を変える指令の出来事の時刻を発火時刻にする・窓の知らせが時刻を受ける・書き出しの進め方・時計を外す・口パクの決まり。
   - 結線とバルーンの寿命（`areka-runtime`）: フレームの頭に、進める相を置く・`TalkClock` から起点を外し、`ClockedTextSink` を消す・寿命の知らせを絶対の時刻で運び、列で持つ・判断をフレームの時刻で行う・次のフレームを起こす予約・手で組む合図と窓の知らせに `FrameTime` を付ける。この家族は、上の 2 つの口（`advance`・提示と読み口・`send_stage` の時刻の引数）を使うので、2 つの本体の後に確かめる。`areka-input`（利用者の中断の知らせの時刻を含む）・`areka-session`・本体の、文字の層へ合図を直に入れるテストと、`spawn_seriko_clocked` を呼ぶ所の直しも、ここに続ける。旗を立てる所の一覧（本体の `tick_gate_config_producers_tests.rs` と、wintf の `tick_wake.rs` の頭の一覧）の直しも、ここに続ける。
5. **全体の成り立ちのテスト**（要件 18 の 5・7＝置き直しをはさむ列を含む、眠り＝17 の 6・10、`OnSecondChange`＝17 の 7）。

### 先に替える共有のファイル

家族を並べる前に、1 人が替える。

| crate | ファイル | 理由 |
|---|---|---|
| `dola` | `src/time/*`・`src/lib.rs`・`Cargo.toml`（段 0） | 全部の前提。`src/time/mod.rs` は、段 0 の型の家族が、子の宣言の全部を先に書く |
| `dola` | `src/error.rs` | 「文書の型と検査」と「コンパイル」の両方が variant を要る。足し引きの全部を 1 度で替え、後の家族は編集しない |
| `dola` | `src/cue/*`（本体の全部）・`src/cue/test_support.rs` | 互いに呼び合う。`carrier_cue` は外の 35 ファイルが名前で使う |
| `dola` | `tests/cue/test_support.rs` | 引数の形で、結合テストの書き方が決まる |
| `dola` | `src/compile/types.rs` | コンパイルの側と runtime の側の両方が読む。1 周の長さを求める 1 本の関数（`CompiledStoryboard::lap_length`）も、ここに置く |
| `dola` | `tests/trigger/common/mod.rs` | 文書を組む手助け。3 つの家族のテスト（`compile_test.rs`・`runtime_test.rs`・`validation_test.rs`）が使う。持ち主は、土台の家族 |
| `areka-talk` | `Cargo.toml`・`src/lib.rs` | `areka-sakura`・`areka-kanade`・`areka-ghost` が使う |
| `areka-kanade`・`areka-ghost` | `Cargo.toml`（dola の行） | その crate のほかの家族が、`dola::time` を取り込む |
| `areka-sakura` | `contract.rs`（時刻の型の再公開を含む）・`duration.rs`・`reveal.rs` | 上の crate の全部が受ける |
| `wintf`・`areka-sakura`・`areka-kanade`・`areka-emo-text`・`areka-input` | `Cargo.toml` の `[dev-dependencies]` の、機能 `test-support` を付けた dola の行 | その crate のテストの家族が、テストの組み立て（`at_ms`・`ms`・`delta_ms`）を使う。持ち主は、その crate の「先」の家族（`areka-input` は 1 家族） |
| `areka-sakura` | `held.rs`（後半） | `areka-emo-text` と `areka-runtime` が使う |
| `areka-emo-text` | `actor_held.rs` の `advance`・提示と読み口の時刻の引数・`sink.rs` の知らせの口（後半） | `areka-runtime` のフレームの頭と、上の crate のテストが呼ぶ |
| `areka-runtime` | `talk_lifecycle.rs` の知らせの形・`talk_clock.rs`（後半） | 判断・相・通しの家族と、`areka-input` の押下の口が使う |
| `areka-seriko` | `actor.rs` の `send_stage` の時刻の引数と、時計を外した起動の口（後半） | `areka-runtime`・`areka-session`・本体が呼ぶ |
| `wintf` | `ecs/graphics/core.rs` | wintf の中の 3 家族と、上の crate が読む |
| `areka-seriko`・`areka-emo-text`・`areka-kanade` | 共有の組み立て（`*_test_support.rs`・`tests/common/mod.rs`・`tests/support/*`・`areka-kanade` の `tests/kanade/common/**`） | テストの家族が使う |
| `areka-runtime` | 「土台と結線」の家族 | ほかの家族と上の crate が使う |
| `areka-input`・`areka-session` | `*_test_support.rs` | 本体が使う |

### 1,000 行に近いファイル

番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`・上限 1,000 行）に触れないための決まり。

- 触る前に行数を測る。整形の後に 1,000 行を越える見込みのファイルは、先に主題ごとに分けてから直す（接続の決まりは `structure.md` の Unit Tests の節）。
- 全面を触る物（分ける前提で段取りを組む）: `wintf` の `typewriter/tests.rs`（944）・`areka-seriko` の `looper_tests.rs`（975）・`parts_tests.rs`（975）・`parts_trigger_tests.rs`（906）・`looper_talk_tests.rs`（861）・`areka-ghost` の `dispatcher_choice_tests.rs`（908）・dola の `tests/trigger/runtime_test.rs`（854）。
- 触る量が小さい物（行を増やさない。増えるなら先に分ける）: `areka-ghost` の `tests/ghost/inproc_e2e_test.rs`（964）・`areka-kanade` の `schedule/events.rs`（941）・`schedule/steady.rs`（926）・`actor.rs`（873）・`areka-runtime` の `frame_test_support.rs`（939）・`balloon_visibility_phase.rs`（848）・`balloon_visibility_tests.rs`（847）・`areka-emo-text` の `lookahead_tests.rs`（901）・`actor.rs`（873）・`actor_present.rs`（866）・`areka-seriko` の `looper.rs`（887）・本体の `runtime_tests/spine_conformance_lap_tests.rs`（862）。
- 行を増やさない手立ての 1 つ: テストの決まった時刻は、`dola::time::testing` の 1 語の組み立て（`at_ms`・`ms`・`delta_ms`）で書ける。この組み立てを置く理由は行数ではない（「文字の形と、テストの組み立て」の節）。
- 新しく足す物は、新しいファイルへ置く（置き直しのテスト・要件 18 のテスト・文字の層の列と `advance`＝`actor_held.rs`・寿命の列＝`balloon_visibility_held.rs`・受け手の列の型＝`held.rs` ほか）。`areka-emo-text` の `actor.rs`（873）と `actor_present.rs`（866）と、`areka-runtime` の `balloon_visibility_phase.rs`（848）には、列の中身を足さない。

### 記録

`.kiro/specs/completed/dola-interrupt-ticks/migration-ledger.md` に、次の 4 つを書く。

1. **外したテストと理由**（9.4・9.5・9.7）: 表せない入力だけを確かめていた物（`compile_storyboard` の開始の時刻に NaN を渡す物を含む）・ミリ秒の格子の継ぎを確かめていた物・丸めそのものを確かめていた物・確かめていた事そのものが無くなった物（コンパイルの開始の時刻を足す事・`result.start_time`）。置き換えた先のテストの名前も書く。
2. **取り除いた本番の分かれ道とログ**（7.9）: 有限でない刻み・有限でない終わり・NaN の周期・`base_now`・時計が無い時の合図と、時計が無い時の窓の知らせ（seriko）・トークのスレッドの同じ時刻の守り（`drive.rs` の `debug!`）・解いた直後に完了を確かめる処理（`settle_completion_after_resolve`）・cue の長さの clamp・storyboard の文書の検査の決まり（有限・0 以上・`LoopOffsetNegativeMin`／`Max`）・起点の見積もり（`TalkClock::observe_cue`・`ClockedTextSink`・seriko の `TalkEpoch`）・台本の時刻を求める所（`resolve_talk_time`・`TalkClock` の台本の時刻の口・起点がまだ無いフレームを見送る分かれ道）・止まった時刻が無い時の分かれ道（`TalkEnd::TimeUnknown` と、その `warn!`）・負の表示の終わりを 0 に寄せる所・seriko の時計（`SerikoClock`・`spawn_seriko_clocked`・合図を処理する時刻を読む口）ほか。
3. **変える前の期待と、変えた後の期待**（9.6）: テストの名前で名指しする。前半と後半を分けて書く。
   - 前半に入る物（「継ぎを置かない・前半と後半」の節の一覧の全部）: 時計の出どころ（眠り・`OnSecondChange`・分解能・境の格子の 0 の位置・seriko のミリ秒への丸め）・1 字ごとの間隔の式・ミリ秒の格子に載らない時刻の足し算・台本の起点の `Option` と `from_sheet` の `Err`・表せない値を直していた所（cue の長さの clamp・時間切れの NaN と無限大）・storyboard の文書の失敗の出る所（検査から読み込みへ）・文書の値の上限・文書の秒の丸め・倍率が 1 でない再生の出来事の時刻・終わらないループの下限の境・終わりの予定の時刻の意味・出来事の直前の時刻・wintf の作者の秒・`Instruction::Wait` の 1 ms 未満・ログの欄・コンパイルの開始の時刻。
   - 後半に入る物: 止めて戻す・時刻の表の取り出しと置き直し・区切りを解く口（解いただけでは完了にならない）・トークのスレッドの同じ時刻の守り・起点の見積もり（やめる。受け手は起点を持たず、合図の発火時刻で持つ）・刻みの値・口パクの決まり・フレームの時刻が発火時刻より前の時の扱い（0 に丸めない）・文字の層の、字のほかの指令が効く時（届いた時から、フレームの時刻が発火時刻に届いたフレームへ）・先渡しとトークの印が当たる時・バルーンの寿命の知らせが効く時と、その時刻（台本の時刻から、絶対の時刻へ。トークの終わりと利用者の中断は、時計の読み）・文字の相が、合図が 1 つも当たっていない間も提示を呼ぶ事・文字の線の受け取りが旗を立てる事・まだ現れていない字が在る間の予約（毎回の画面の更新から、次に現れる字の時刻へ）・文字の層が借りられないフレームの扱い（文字の拡大率の相と文字の相が、落ちる事から、見送って予約する事へ）・`hover_inject` の時刻（台本の時刻から、フレームの時刻へ）・seriko の、姿を変える指令の出来事の時刻（合図を処理する時の時計の読みから、発火時刻へ）・窓の知らせと、手で組む合図の時刻（フレームの時刻）・書き出しの進め方の合図の時刻。
4. **締めの検査の結果**（15.4・15.5・17.2）。

## Open Questions

この文書に、開発者へ確かめる事は残っていない。

## Supporting References

- 調べの控えと、案の比べ: `research.md`（14 章が設計の段の分・16 章が設計の討議で決まった事・17 章が 2 回目の検証の後に決まった事）。
- ノベルの作り直しの使い方: `doc/ideas/novel-redesign-2026-11-20/00-architecture.md`・`01-playback-control.md`・`02-parallel.md`・`04-sound.md`・`07-timed-values.md`。
- 完了の関門: `.kiro/steering/workflow.md`（全体テストは、アーカイブの後の 1 回）。
