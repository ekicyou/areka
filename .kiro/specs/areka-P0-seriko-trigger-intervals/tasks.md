# Implementation Plan

> 順番が依存を表す（後のタスクは前のタスクの上に積む）。`(P)` は直前の兄弟と並走できる印。新しいテストは本番ファイルの中でなく同じディレクトリの兄弟ファイルへ置き、本番・テストとも 1 ファイル 1,000 行以下を守る（要件 9.7）。「同じウェーブで触らない約束」のファイル（台本の読み手・台本のコンパイル・運び手の名前の表・合成）には触らない。

- [x] 1. 読み手: 3 語を数値ごと読む
- [x] 1.1 `talk,数値`・`runonce`・`periodic,数値` を語と数値で運ぶ読みを足す
  - interval の型に 3 つの腕を足す（既存の腕と、animation・pattern・element の欄は変えない）
  - 小文字の完全一致だけを 3 語として読み、大文字混じり・`+` 入りは今までどおり元の綴りのまま「その他の語」へ運ぶ
  - `talk`／`periodic` の数値が正の整数として読めないとき（欠落・0・非数値）は、第 2 欄以降を `,` で繋いだ原文を「その他の語」へ運ぶ（読み手は失敗せず、記録も出さない）
  - 兄弟のテストで 3 語の読み（数値あり・数値なし・0・非数値・大文字混じり・`+` 入り・`runonce`）が緑になり、既存の語（`bind`・`random,数値`・`bind+random,数値`・`always`・`sometimes`・`rarely`）の読みのテストが緑のまま
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 9.1, 9.7_

- [x] 2. seriko の表: 3 語を採る
- [x] 2.1 3 語を引き金として表に採り、採った・採れなかったを記録する
  - 表の引き金の型に 3 つの腕を足す（`periodic` は周期の ms・`talk` は区切りの文字数。どちらも 0 を持てない型）
  - 採ったとき、面の番号・アニメーションの番号・語・数値を添えた `debug!` を表を組むときに 1 回残す
  - 「その他の語」の先頭の語が `talk`／`periodic` のとき（数値が無効）は、面の番号・アニメーションの番号・元の綴りを添えた `warn!` を 1 回残して採らない。コマ列が空の 3 語は既存の `warn!` で採らない。`+` の組み合わせと範囲外の語は今までどおり `debug!` だけ
  - 表に「3 語が在る」「`talk` が在る」の印を持たせ、繰り返しの経路を通す門の判定に 3 語を加える
  - 引き金の型の腕が増えて seriko の中の網羅の `match` が通らなくなる箇所は、このタスクでは「何もしない」腕で埋めてビルドを通す（振る舞いは 4・5 で入れる）
  - 既存の檻のうち `runonce` を「採らない語」の例に使っているものは、範囲外の語（`yen-e`）へ替える
  - 兄弟のテストで、採録の印・無効な数値の非採録・3 語の無い表で印が偽であることが緑になり、seriko の既存のテストが緑のまま
  - _Requirements: 1.5, 1.6, 7.1, 7.2, 7.4, 8.1, 9.7_

- [x] 3. 引き金の判定と文字の写し（純粋な芯）
- [x] 3.1 「見え始めの時刻」を起点にした `runonce`・`periodic` の判定を作る
  - 引き金の状態（見え始めの時刻・`runonce` の 1 回だけの印・`periodic` の周の数え）と、「今この刻みで始めるか・開始の時刻はいつか」を返す純関数を、seriko の新しい兄弟モジュールとして置く（公開面は変えない）
  - `runonce`: 構えた刻みに 1 回だけ鳴り、開始の時刻は見え始めの時刻。印が付いた後は鳴らない
  - `periodic`: 見え始めの瞬間には鳴らず、数値秒ごとの周の境目で鳴る。開始の時刻は最新の周の境目そのもの。再生中の周は飛ばして数えだけ進める。1 回の刻みで 2 周以上またいでも 1 回だけ
  - 隠す・現すの操作（バルーンの窓の閉じ・開き）: 隠れている間は鳴らず、現れた時刻を `periodic` の新しい起点にし、`runonce` の印は残す
  - 再生中の同じアニメーションには何も返さない。捨てる経路では記録を出さない
  - 兄弟のテストで上の分岐（1 回だけ・起点・周の境目・再生中の飛ばし・2 周またぎ・隠す／現す）が偽の時刻で緑になる
  - _Requirements: 2.1, 2.2, 3.1, 3.2, 3.3, 3.4, 3.5, 5.3, 5.9, 6.1, 7.5, 9.3, 9.7_

- [x] 3.2 文字の区切りの判定（`talk`）を引き金の判定に足す
  - 文字の数えの状態（数え始めの序数・数え済みの序数）と、刻み 1 回分の文字の窓（前に見た数・今見えている数・序数から壁時刻への写し）を受けて判定する
  - 数え始めから数値分ごとの区切りを、窓の中で越えたときに鳴らす。開始の時刻は区切りの文字が現れた時刻 t_k そのもの
  - 同じ窓で区切りを 2 つ以上越えたら最新の 1 つにまとめる。再生中は始め直さず数えだけ進める。数えは単調（今見えている数が減った刻みは進めず、鳴らさない）
  - 兄弟のテストで、3 文字ごとの区切り・t_k の開始の時刻・まとめ・再生中・単調が緑になる
  - _Requirements: 4.1, 4.5, 4.6, 4.7, 9.4_

- [x] 3.3 文字が現れる時刻を、文字の層と同じ式で seriko の時計の上に写す
  - 起点の見積もり: 届いた cue ごとに「今 − cue の時刻」の最大を取る（ms で持ち、壁時刻へは最も近い 1 ms へ丸めて写す）
  - スコープごとの文字の時刻の列: 塊ごとに「前の文字＋1 字の間隔」と「塊の頭」の大きい方で 1 文字ずつ積む（文字の層のリビールの式の写し）。文字は書記素クラスタで数え、序数は塊をまたいで連続
  - 消去の知らせ: その時刻までに現れていない文字を列の末尾から捨て、継ぎ目を初期化する（列は単調のまま・現れなかった文字で口は動かない）
  - 数え終えた古い文字の刈り込み（序数は変えない）と、「ある台本の秒までに現れた文字の数」の問い合わせ
  - 兄弟のテストで、文字の層の式（i 文字目＝「前の文字＋1 字の間隔」と「塊の頭」の大きい方）から求めた固定の期待列と一致すること（文字の層の crate を依存に足さない）・結合文字と絵文字の列が 1 文字・「中断→全消去（時刻 0）→新しい塊」と「消去の後の塊」で列が単調のまま可視の数が文字の層と一致すること・刈り込みで序数が変わらないことが緑になる
  - _Requirements: 4.2, 4.3, 4.10, 6.3, 6.4, 9.4, 9.7_

- [x] 4. 一番上の面の配線
- [x] 4.1 一番上の面で `runonce`・`periodic` を鳴らす
  - 面の切り替えの知らせで、その (スコープ, 窓の種類) の引き金の状態を捨てる。構え直しは「面が在り、構える条件（一番上に 3 語が在る、または表に `talk` が在る）を満たす」ときだけ（出来事の時刻を起点にする）。条件を満たさない面では状態も「面に入った」の記録も生まれない
  - 切り替えの刻みで `runonce` を 1 回始め、刻みごとに `periodic` を判定し、始まった再生は今の `random` と同じ経路（コマを番号順に 1 回・`-1` で消して終える・遅れた分だけ進める）に乗せる。開始の時刻は判定が返した時刻
  - 同じ面の再指定・着せ替えだけの変化では構え直さない。別の面から戻ったとき・非表示から戻ったときは構え直す
  - バルーンの窓の閉じ・開きは引き金の状態の隠す・現すに写す（閉じている間は非表示・開き直しで `runonce` は鳴らさず `periodic` は新しい起点）
  - 3 語は抽選の輪に入れず乱数を消費しない。表の差し替えで引き金の状態を捨てる。刻みごとの一番上の判定は「一番上に 3 語が在る」面でだけ回す（`always` の有無と 1 回の走査で求める。表の印は 2.1 のもの）
  - 記録: 面に入った `debug!`・`runonce`／`periodic` の開始 `info!`・捨てる経路は記録なし
  - 兄弟のテスト（偽の刻み）で、最初の表示・同じ面の再指定・着せ替えだけ・戻り・非表示からの復帰の `runonce`、`periodic` の起点・N 秒ごと・面を離れた停止・再生中の飛ばし・2 周またぎ、3 語の無い表で状態が空のまま（`random` の再生中でも）・乱数の消費 0、`random` と混在しても互いに干渉しないこと、バルーンの窓の開き直しで `runonce` が鳴らず `periodic` が新しい起点になることが緑になる
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 3.5, 5.1, 5.2, 5.3, 5.5, 5.7, 5.8, 6.1, 6.2, 7.5, 8.1, 8.2, 9.2, 9.3, 9.6_

- [x] 4.2 一番上の面で `talk` を鳴らす（文字の写しを刻みへ繋ぐ）
  - 届いた cue を受ける口を足す: 表に `talk` が無ければ真偽 1 つで戻る。在れば起点の見積もりを更新し、文字と選択肢の文字をそのスコープの列へ積み、消去はそのスコープ（全消去は全スコープ）の継ぎ目の初期化へ。改行・待ち・`\!` のコマンドは数えない
  - 刻みごとに、表に `talk` が在る (スコープ, 窓の種類) について文字の窓を作り（一番上に `talk` が無く部品にだけ在る面でも作る）、一番上の `talk` は判定が返した t_k を開始の時刻として再生を始める（同じ刻みのうちに経過分のコマを出す）。数え済みの序数は、全部の面が窓を読み終えた刻みの最後に進める
  - 文字の数えは面に構えたときの「今見えている数」から始める（面の切り替えで 0 から）。文字が 1 つも届いていないスコープは 0 から数える（遅延して作らない＝最初の cue と 1 文字目は刻みの間に届くので、遅延すると 1 文字目を数え落とす）
  - `talk` の開始・終わり・停止の記録は `debug!` に留め、文字ごと・刻みごとの記録を増やさない。`talk` の無い表・非表示のスコープでは窓を作らず何も記録しない
  - 兄弟のテスト（偽の刻み・偽の文字の到着）で、3 文字ごとの口の動き・待ちと塊をまたぐ積み上げ・面の切り替えでの数え直し・開始の時刻が t_k・他のスコープが動かないこと・`talk` の無い表と非表示で状態も記録も増えないこと・起点が未確立のまま構えた面で最初の文字から正しく数え始めることが緑になる
  - _Requirements: 4.1, 4.3, 4.4, 4.5, 4.8, 4.9, 4.10, 6.4, 7.3, 8.1, 9.4, 9.6_

- [x] 5. 部品（element定義の子・pattern定義の先の面）の配線
- [x] 5.1 見えると確定した部品だけが `runonce`・`periodic` を受ける 2 段の評価にする
  - 部品の評価を 2 段にする: 1 段目は今までどおり（抽選と `always` の並びを変えない）、2 段目は最終的に見えると決まった部品のうち 3 語を持つものにだけ引き金の状態を構える。表に 3 語が無いときは 2 段目を呼ばない（今の形のまま）
  - 部品が見えた刻み（外側の面の切り替えで最初から見えた場合と、外側のコマの変化で後から見えた場合の両方）を起点に `runonce` を 1 回・`periodic` を数値秒ごとに始め、同じ刻みのうちに子のコマまで出す
  - 部品が見えなくなったら、その部品の 3 語の時計と引き金の状態を捨てる（再び見えたときは新しい起点で、途中のコマから始まらない）。窓の閉じ・面の消去でも捨てる／隠す
  - 3 語は部品でも抽選の輪に入れず乱数を消費しない
  - 兄弟のテストで、見えている部品で始まる・見えていない部品で始まらない・後から見えたときの起点・見えなくなったら捨てる・同じ刻みで子まで出る・3 語の無い表で部品の引き金の状態が生まれない（動く部品が見えていても）ことが緑になり、部品の既存のテストが緑のまま
  - _Requirements: 5.1, 5.2, 5.4, 5.6, 5.7, 5.9, 8.1, 8.2, 9.5, 9.6_

- [x] 5.2 部品の `talk` を、その窓の文字の数えで鳴らす
  - 一番上の配線が作る文字の窓を部品の評価へ渡し、見えている部品の `talk` を同じ判定で鳴らす（部品が後から見えても数え直さない＝その窓の数えを借りる）
  - 一番上の面に 3 語が無く部品にだけ `talk` が在る面でも、4.2 が作る窓を受けて部品の口が動く（4.1 の構える条件の後ろ半分がこのため）
  - 兄弟のテストで、見えている部品の `talk` が区切りで始まること・一番上に 3 語が無い面の部品でも始まること・見えていない部品では始まらないことが緑になる
  - _Depends: 4.2_
  - _Requirements: 4.1, 5.4, 5.6, 9.5_

- [x] 6. 統合: 台詞の再生から届く cue を口パクへ繋ぐ
- [x] 6.1 seriko の受け口で、届いた cue を文字の写しへ 1 か所で渡す
  - cue を取り出した直後に、種類を問わず 1 回だけ文字の写しの口へ渡す（今の面の切り替えなどの扱いと記録はそのまま）
  - 「今」は seriko の時計から読む（新しい時計も新しい知らせの口も作らない）
  - 兄弟のテスト（本物のアクター＋偽の時計＋偽の文字の到着）で、文字の到着で口のコマが出ること・`\0` の文字で `\1` の口が動かないこと・非表示のスコープで動かないこと・選択肢の文字が数えられること・改行／待ち／`\!` が数えられないこと・`\s[-1]` から戻ると `runonce` が鳴ることが緑になる。注入した時刻は観測の後に進める
  - _Depends: 4.2, 5.2_
  - _Requirements: 2.6, 4.8, 4.9, 4.10, 6.3, 9.2, 9.4, 9.6, 9.7_

- [x] 7. 口パクの検体と決定論の E2E
- [x] 7.1 3 語を書いた検体の面を用意し、本物の読み手と本物のアクターで通す
  - seriko のテストの検体として、3 語を書いた一番上の面・部品を持つ面と部品の面・無効な数値の面・コマ列が空の面を持つ `surfaces.txt` と、面の役・絵の出どころ（同梱の emo2 の既存の絵を番号で指す・画像ファイルは足さない）・実機の手順への参照を書いた README を置く（合成の crate の下には置かない）
  - 検体を本物の読み手で読み、表を組み、偽の時計のアクターへ面の切り替え・文字・刻みを通して、出てくるコマを判定する E2E を兄弟のテストに置く
  - E2E で、一番上の面の 3 語が鳴ること・部品の面の `talk` と `runonce` が鳴ること・無効な数値の面が `warn!` 2 件で非採録・コマ列が空の面が `warn!` 1 件で非採録になることが緑になる
  - _Depends: 6.1_
  - _Requirements: 1.5, 7.2, 9.6, 9.7, 11.1_

- [x] 8. (P) 網羅台帳を実装に合わせる
  - 台帳の `talk,数値`・`runonce`・`periodic,数値` の 3 行を `implemented`・担当を本 spec にし、areka の裁量（`periodic` は切り替わった瞬間には鳴らさない・`talk` の数える単位と数え直しの時点・同じ時刻の区切りのまとめ・再生中の飛ばし・閉じたバルーンの窓の扱い・`+` の組み合わせは採らない）を note に書く
  - `sometimes`・`rarely` の note の古い 1 文を着地後の実態（3 語は駆動・`yen-e`・`never` は非駆動）に直す
  - `always` の行の担当を `areka-P0-seriko-interval-combinations` へ付け替える（判定と note は変えない）。`yen-e`・`never` の行には触れない
  - 台帳の見張りが要求する分だけ、網羅の下書きの spec 表を合わせる（本 spec の担当の数・付け替え先の spec の行・brief の数）
  - 台帳の整合を見張る常時テストが緑になる
  - _Boundary: 網羅台帳_
  - _Requirements: 10.1, 10.2, 10.3, 10.4_

- [x] 9. 検証
- [x] 9.1 変わらないものの回帰を確かめる
  - 読み手・seriko・台帳の見張りの crate のテストを通しで回し、`random`・`bind+random`・`sometimes`・`rarely`・`always`・`bind` の既存の決定論のテストが 1 本も書き替えなしで緑であること（2.1 の語の差し替え 1 か所を除く）を確かめる
  - seriko を使う側（本体の起動の配線・合成の E2E）のテストが緑で、同梱の検体を読む既存の檻の記録の件数が変わらないことを確かめる
  - 本 spec で足した・触った本番ファイルとテストファイルがどれも 1,000 行以下で、約束のファイルに差分が無いことを `git diff --stat` で確かめる
  - 上の 3 点の結果（緑の件数・行数・差分なし）が実行の出力として残る
  - _Requirements: 1.6, 8.1, 8.2, 8.3, 9.7_

- [x] 9.2 実機で口パク・`runonce`・`periodic` を確かめる
  - ワークツリーの `target\` の下に emo2 の写しを作り、写しの `surfaces.txt` の末尾へ検体の面を書き足す（検体そのものと `target\` の外は書き替えない）
  - 有界の自動終了・走行ごとのプロファイル・判定の分岐が読める記録の水準（seriko と文字の層を `debug`）で起こし、MCP の `sakurascript` で面の切り替えと台詞（3 の倍数の文字・待ち・改ページ・選択肢）・部品の面・戻り・非表示からの復帰を送り、面の写しを繰り返し撮る
  - 記録の検索で、`runonce` が面に入るたびに 1 回・`periodic` が 2,000 ms ごと・`talk` の開始の時刻が文字の層の記録から求めた t_k と 1 刻み以内・無効な面の `warn!` の件数・emo2 の素の面の記録が前の実行体と同じ、を判定する
  - 範囲外でも areka の未対応のために動かなかった件はすべて `/kiro-discovery` で起票する
  - 判定の結果（各項目の件数と時刻の差）と起票の一覧が報告に残る
  - _Depends: 7.1_
  - _Requirements: 8.3, 11.2, 11.3, 11.4_

## Implementation Notes

- 1.1: 読み手が `runonce` を `Interval::Runonce` で返すようになったので、seriko の既存の檻 2 本（`table_interval_words_tests.rs` の `other_interval_words_are_not_recorded_and_keep_their_original_vocab`・`table_always_tests.rs` の `combinations_and_other_words_keep_the_debug_arm`）が 2.1 の着地まで赤になる。どちらも語の列に `runonce` を持つ。2.1 で列から外す（設計の Modified Files には `table.rs` の中の檻 1 本しか載っていない）。
- 1.1: 数値の読みは設計の `fields[2].parse::<u32>()` と同じ答えになる既存の `field_u32` を使った。読み手の中で `periodic,3` を「未認識の語」の検体にしていた檻（`decode_tests_lenient_input_tests.rs`）は `yen-e,3` へ替えた。`decode_tests_animation_tests.rs` の冒頭のコメント「sometimes/periodic 等」は古いまま（未修正・軽微）。
- 2.1: `runonce` を「採らない語」の例にしていた檻の差し替えは 4 ファイル（`table.rs` の中の 1 本・`table_interval_words_tests.rs`・`table_always_tests.rs`・`table_parts_tests.rs` の 2 件）。9.1 の「語の差し替え 1 か所を除く」はこの 4 ファイルと読む。
- 2.1: 途中の状態 — `parts.rs` の `gate` は 3 語を一時的に `Gate::Off` へ落としている（5.1 で `Gate::Trigger` へ）。`looper.rs` の抽選の `match` は 3 語を乱数の前で `continue`。`table.rs` の `has_talk()` は `#[cfg_attr(not(test), expect(dead_code, …))]` 付き（4.x で `looper.rs` が読み始めたら外す。外し忘れは「満たされない expect」の警告で出る）。3 語を持つ部品は既に `has_animated_parts` を真にする。
- 2.1: `parts_always_tests.rs` の `always_beside_random_does_not_change_the_draws` は「`always` の無い側」に `runonce` を使ったまま緑。5.1 で部品の `runonce` が鳴るようになったとき、乱数 2 回のままかを確かめる（赤なら語を `yen-e` へ）。
- 2.1: `table.rs` は 898 行（残り約 100 行）。採録の `debug!` の `value` は `periodic` が秒・`talk` が文字数・`runonce` は欄なし。
- 3.1: `trigger.rs` の `arm(at_ms, open, _revealed)`・`show(at_ms, _revealed)`・`poll(.., _talk)` は設計の署名どおりで、文字まわりの引数は 3.2 が使い始める（3.1 の檻は書き替え不要）。`TalkWindow` の `#[expect(dead_code)]` は 3.2 で外す。`lib.rs` の `mod trigger;` の `#[cfg_attr(not(test), expect(dead_code, …))]` はモジュール全体に掛かるので、配線が済んだ時点（4.1〜5.x）で必ず外し、使われない項目が残っていないことを確かめる。
- 3.1: 設計の `hide` の doc（「`periodic` の周を捨てる」）と不変条件（「`show` で `last_lap` だけ空になる」）が食い違う。実装は不変条件の側（`poll` から見える差は無い）。設計の 1 文は文書だけの直し。
- 3.1: 閉じた窓で構えた面（`arm(open=false)`）の `runonce` は、最初に開いた時刻で 1 回鳴る（まだ鳴らしていないため）。開き直しでは鳴らない。台帳の note（タスク 8）に 1 文足す候補。再生中の `runonce` は印を付けない（設計の事後条件どおり・一番上では到達しない防御）。
- 3.2: `poll` の `Talk` の腕は数えを進めない（窓 1 つを一番上の何本もの `talk` と部品が読むため）。呼び手の順は「`talk_window_bounds` → 窓を組む → 一番上を `poll` → 部品へ窓を渡す → `advance_talk(now_seen)`」。`advance_talk` を部品の評価より前に置くと、部品が同じ区切りを見落とす。
- 3.2 → 4.2: 設計の一番上の配線にある `TalkCursor` の遅延生成（構えた時点で起点が未確立）の口が `Armed` に無い。4.2 で `trigger.rs` に「数えが無いときだけ `base = seen = now_seen` で作る」口を 1 本と檻 1 本（作った刻みは鳴らない・既に在れば置き換えない）を足す。`show(at, None)` は数えを落とすので、起点が未確立のまま開き直した slot もこの口で拾う。
- 3.2 → 4.2: `hide` は `talk` の数えを残し `show` が置き換える（設計の `hide` の doc「数えを捨てる」とは字面が違う・`poll` から見える差は無い）。隠れている間も `talk_window_bounds()` が古い `Some` を返すので、刈り込みの最小を取るときに隠れた slot を外すか、`hide` に `self.talk = None` を足して doc に揃える。どちらもしないと、閉じたバルーンの窓のスコープの文字の列が刈り込まれない。
- 3.2 → 5.2: 部品の `Armed` は `talk: None`（数えを持たず slot の窓を借りる）。部品は窓を `poll` に渡すだけで `advance_talk` は呼ばない。
- 3.3: `talk.rs` に設計の署名に無い `TalkFeed::push_text(at_s, duration_s, text)` を足した（`push_chunk(.., cluster_count(text))` の 1 行・書記素クラスタの決まりを `talk.rs` に置くため）。4.2 の `observe_cue` は `Text`／`Choice{text}` の両方でこの口を呼ぶ（自前で `cluster_count` を綴らない）。配線したら `lib.rs` の `mod talk;` の `expect(dead_code)` を外す。
- 3.3 → 4.2（重要）: `restart_chain` は「数え済み（序数 < 数えの `seen`）なのにまだ刈り込まれず列に残っていて、現れる時刻が消去の時刻より後」の文字を捨て、その序数を次の塊の別の文字に振り直す。`advance_talk` の単調 max と `g >= prev_seen` は二重に鳴るのを防ぐだけで、新しい台詞の頭の文字が区切りを越えても鳴らない（口が止まる）。4.2 が保証すること＝`restart_chain` を呼ぶ時点で、そのスコープの生きている数えの `seen` より小さい序数が列に残っていない（列の `base` ≥ その `seen`）。崩れる場面は (a) 隠れた slot の古い `seen` を刈り込みの最小に含めたとき (b) 刻みの間に構えた数えの直後、次の刻みの前に全消去が届いたとき。4.2 の檻に「区切りの途中で中断 → 新しい台詞の最初の区切りで口が動く」を 1 本入れる。
- 3.3 → 4.2: 刈り込み済みの序数に `time_of` は `None` を返すので、窓の `wall_ms` の閉包は `g >= base`（列の `base`）を保つ。経過は `now.saturating_sub(started)` で引く（丸めで今を 1 ms 超える余地を理屈の上では排除できない）。全消去の前置きが無い新しい台詞の塊は設計の前提の外（列の単調性は守られない・ガードなし）。
- 3.3（設計の水準の覚え・範囲外）: 文字の層の列は「スコープ × 文字の場所（普通のバルーン／名前の箱）」ごと、`TalkFeed` はスコープごと。今の台本の組み立て（1 字 50 ms・次の cue は前の cue の終わり以後）では差は ms の丸めで消える。
- 4.1: `trigger.rs` に設計に無い読み口 `Armed::is_visible()` を足した（窓の開け閉めを `hide`／`show` へ 1 回ずつ写すため）。`looper.rs` は 842 行。`talk` だけが理由で構えた面は 4.1 では門を広げていない（4.2 で門に表の `talk` の印を足す）。`arm_slot` の `Armed::arm(.., None)`／`show(.., None)` と `fire_top_triggers` の `poll(.., None)` が 4.2 の差し込み口。`trigger.rs` の項目ごとの `expect(dead_code)` 3 か所（`TalkCursor`・`advance_talk`・`talk_window_bounds`）は 4.2 で外す。
- 4.1（査読の指摘・設計の水準）: 判定は進行より前で `playing = playback に在る`（設計の字面どおり）。「境目の時刻に再生中か」でなく「まだ片付けていないか」で測るので、再生の終わりと次の境目の間に刻みが入らないとその周を飛ばす。長さが周期ちょうどの animation（`periodic,1` で待ちの合計 1000 ms）は毎回飛んで 2 周期に 1 回しか鳴らない。`talk` も同じ形（区切りの間隔ちょうどの長さの口の animation）。
- 4.1（既存の死角・範囲外）: `\b` を受けていないスコープで窓の知らせだけがバルーンの面の番号を変えると、`on_surface_changed` が呼ばれず引き金の状態が古いまま残る（一番上の `always` と同じ死角・`actor.rs` の `on_stage`）。完了の棚卸で起票する候補。
- 4.1: 窓が閉じても再生中の `runonce`／`periodic` は最後まで進む（止まるのは新しく鳴らす方だけ・`random` と同じ）。時刻が分からない `refresh` では構えず、最初の刻みがその時刻で構えて同じ刻みで頭のコマを出す。
- 4.1-fix（上の査読の指摘を是正・設計も改訂済み）: 「再生中か」は刻みの時刻でなく、判定が返す開始の時刻で測る。`poll(anim, now_ms, playing_at: impl Fn(u64) -> bool, talk)`。呼び手は「`playback` に在り、`frame_at(frames, t.saturating_sub(started_at_ms))` が `Pending`／`Active`」と答える。開始の時刻には終えていてまだ片付けていない同じ animation の再生は、`put_top_play`（進行の塊を文言そのままに切り出した関数）でその時刻まで進めて片付けてから入れ替える（終わりの記録は 1 件）。`looper.rs` は 895 行。
- 4.1-fix → 4.2: `talk` の終わりの記録を `debug!` へ下げる場所は `put_top_play` の 1 か所（進行と入れ替えの両方が通る）。`fire_top_triggers` は文字の窓を足すと引数が 8 つになる。`looper.rs` が 1,000 行に届くなら、自由関数の塊（`put_top_play`・`fire_top_triggers`・`arm_slot`）を兄弟の本番ファイルへ出す。
- 4.1-fix → 5.1／5.2: 部品の 2 段目も同じ述語の形（`Playing` が在り、その時刻の `look` がまだ終えていない）で `poll` する。同じ刻みのうちに同じ animation を 2 度 `poll` しない（開始の時刻にもう終えている `talk` の animation は 2 度目も「鳴る」と答える）。`Runonce` の腕が尋ねる時刻（見え始め）を固定する檻はまだ無い（部品で分岐に届くなら `trigger_tests.rs` に 1 表明足す）。
- 4.1-fix（範囲外の観察）: 抽選（`random`／`bind+random`）にも「`playback` に在れば境界の抽選から外す」同じ形が在る（終えたのに片付けていない再生は、その境界の抽選を 1 回飛ばし乱数も引かない）。既存の振る舞いなので触っていない（要件 8.1）。完了の棚卸で起票する候補。
- 4.2: 上の「3.2 → 4.2」の 2 件は次の形で決着 — 遅延生成の口は不採用（最初の cue と 1 文字目は刻みの間に届くので、遅延すると 1 文字目を数え落とす。構える・現す時点で「文字の列が無いスコープは 0」から数える＝`revealed_at`）。`hide` は `talk` の数えを捨てる（`self.talk = None`）。設計と 4.2 のタスク文も直した。
- 4.2: 刻みの最後の `settle_talk` が、スコープごとに数えを今現れている数まで進めて列をそこまで刈り込む（slot の輪の外＝門で飛ばされた面も進む）。消去の直後は `Armed::realign_talk(total)` で数えを列の総数へ揃える（数えた文字の数は保つ）。不変条件＝「列の頭の序数 ≤ 生きている数えの `seen` ≤ 列の総数」。
- 4.2 → 5.2: 窓を部品へ渡す差し込み口は `on_tick` の `parts.advance(..)` の 1 か所（その位置で `window` が生きている・コメントあり）。`settle_talk` はその後ろ。見えている `talk` の部品は既存の `moving_visible` で門を通る。
- 4.2 → 6.1: `observe_cue` の `#[cfg_attr(not(test), expect(dead_code, …))]` は `actor.rs` が呼び始めたら外す。文字でない cue が起点を動かすこと（`observe` が種類の `match` の前に在る配線）を固定する表明を、アクターの檻に 1 つ足す候補。
- 4.2: `looper.rs` は 928 行（残り約 70 行）。構える・鳴らす・文字まわりの補助は兄弟の本番ファイル `looper_trigger.rs`（子モジュール `top_trigger`）へ出した。3 語の記録の target は `areka_seriko::looper::top_trigger`（9.2 の記録の検索は文言で）。`fire_top_triggers` は `#[allow(clippy::too_many_arguments)]`（`parts.rs` に先例）。
- 4.2（設計の水準・完了の棚卸の候補）: 中断の後の新しい台詞の頭の全消去（台本の 0 秒）は、「最後の刻みより後に現れてまだ刈り込まれていない前の台詞の文字」も列から落とす（`restart_chain(cue.at)` が前の台詞の秒と新しい台詞の 0 秒を比べるため）。その文字は数えに入らず、区切りの文字だったらその 1 回は鳴らない（1 刻み未満の窓・ふつう 0〜1 文字・以後の区切りの位相が 1 文字ずれる）。
- 4.2（範囲外）: `cargo clippy -p areka-seriko --all-targets` が既存の `collapsible_if`（`looper.rs` の単調性の番人）と `large_enum_variant`（`actor.rs`）を出す。新規の `looper_talk_tests.rs` にも同じ形の入れ子の `if` が 1 件（既存の檻と同じ書き方・検証コマンドの外）。
- 5.1: 設計と違う形で受け入れたもの（設計の部品の節は 5.2 の後にまとめて直す）— ⑴ 捨て方を 2 つに分けた: `drop_finite`（窓の新しい出番・`actor.rs` の `on_stage` から）＝部品の 3 語の時計を捨てて引き金の状態は隠す（`runonce` の印は残る）／新しい `drop_hidden`（`\s[-1]`・`\b[-1]`＝`LoopRuntime::refresh` の面なしの枝）＝時計も引き金の状態も捨てる（戻ると `runonce` がもう 1 回）。⑵ 部品は 1 段目が先に進行するので、「再生中か」は 1 段目が進める前の（鍵, 開始の時刻）の控え `was_playing` で答える。⑶ 見えない部品のコマ外しと動く絵の子の評価は外側の輪の後。⑷ `retain_cells`／`unwrite`（鳴り直した 3 語が頭の待ちの前なら、1 段目が書いた前のコマを外す）。⑸ `table.rs` の読み口 `has_triggers()`。⑹ 兄弟の本番ファイル `parts_trigger.rs`（子モジュール `part_trigger`・`fire_part_triggers`・記録の target は `areka_seriko::parts::part_trigger`）。⑺ 走っている 3 語の時計を捨てたとき `debug!`「seriko: part 引き金の時計を捨てた」（回数つきの `always` と同じ扱い・時計 1 本につき 1 回）。
- 5.1（areka の裁量・台帳の note の候補）: 外側の面が A→B に替わっても見え続けた部品は構え直さない（`runonce` は鳴り直さず `periodic` は元の起点で続く。B に居ない部品は捨てる）。窓が閉じると部品の再生中の 3 語の時計は捨てる（一番上は最後まで進む＝ここは揃っていない・閉じている間は絵が出ない）。
- 5.1 → 5.2: 窓の差し込み口は `parts_trigger.rs` の `state.poll(anim, at_ms, playing_at, None)` の `None` 1 か所（`advance` → 閉包 → `fire_part_triggers` へ引数を通す。`refresh` は `runonce` だけなので `None` のまま）。`talk` の終わりの記録を `debug!` へ下げる場所は `parts.rs` の `progress` の 1 か所（今は全部 `info!`）。「同じ刻みに同じ animation を 2 度判定しない」は `rebuild` の `judged` で成立済み＝5.2 で檻を 1 本（待ち 0 のコマ 1 枚の `talk` が同じ刻みに 2 回鳴らない）。部品の `Armed` は数えを持たない。`looper_talk_tests.rs` の `MOUTH_ON_PART` の表は部品 100 が構えられるようになった。
- 5.1: 行数 — `parts.rs` 877・`parts_trigger.rs` 120・`looper.rs` 929・`table.rs` 897。9.2 の記録の検索語「trigger runonce を鳴らした」は部品の文言「part trigger runonce を鳴らした」にも当たる（件数の数え方に注意）。
- 5.1（範囲外・完了の棚卸の候補）: ⑴ 新しい出番で閉じた知らせを見ていない場合、部品は隠す→現すで `periodic` の起点が置き直るが、一番上は窓の開閉が変わらないので起点が動かない。⑵ 2 段目で鳴った部品が同じ評価の後の回で別の 3 語のコマに隠されると、「鳴らした」の `info!` が出た刻みの最後に時計と状態を捨てる（絵には出ない・記録 1 行だけ・稀）。⑶ 既存の `bind.rs`（1,043 行）と `actor_bind_loop_tests.rs`（1,336 行）が 1,000 行を超えている（本 spec では触っていない）。
- 5.2: 窓は `on_tick` の `parts.advance(.., open, window.as_ref(), rng, pattern)` → 2 段目の閉包 → `fire_part_triggers(.., talk, ..)` → `poll` へ通る（`refresh` は窓なし）。部品の `talk` の末尾残留・停止は `parts.rs` の `progress` で `debug!`（`looper.rs` と共有のマクロ `log_play_end!`・`pub(crate) use` で読む）。設計の部品の節・流れ図・File Structure は 5.1・5.2 の実装に合わせて直した。
- 5.2: 9.1 の「書き替えなし」の例外がもう 1 つ — `PartClocks::advance` に窓の引数を足したので、既存の檻 4 ファイル 8 か所（`parts_tests.rs` 5・`parts_always_tests.rs` 1・`parts_film_tests.rs` 1・`looper_always_tests.rs` 1）に `None,` を 1 行ずつ足した（期待値・入力は不変）。5.1 の `parts_tests.rs` の補助 8 本の `pub(super)` 化、4.1-fix の `trigger_tests.rs` の `poll` の引数の機械的な差し替えも同じ類。
- 5.2 → 9.2: 記録の検索語「talk を鳴らした」は部品の文言「part trigger talk を鳴らした」にも当たる。`talk` の「part 停止」「part 末尾残留」は `debug!` なので、実機は `areka_seriko` を `debug` で開ける。
- 5.2: 部品が見えた刻みの窓に区切りが入っていると、その刻みで鳴る（開始の時刻 t_k は部品の起点より前になりうる・1 刻み未満。`runonce`／`periodic` は起点より前に始まらないので、ここだけ非対称・設計に 1 文足した・この場面を固定する檻は無い）。行数 — `parts.rs` 893・`parts_trigger.rs` 127・`looper.rs` 931・`looper_talk_tests.rs` 893・`parts_trigger_tests.rs` 852。
- 6.1: `actor.rs` の `handle_message` が cue を取り出した直後（種類の `match` の前）に `loop_runtime.observe_cue(&cue)` を 1 回。本番の配送は端から端まで繋がっている（`areka-sakura` の `drive.rs` が全 sink を選別なしで登録・`emo2_boot/mod.rs` は素の `SerikoSink` を入れている・時計は `spawn_seriko_clocked(.., Some(tick_count_ms))` で常に在る＝`emo2_boot` の変更は不要）。seriko に本 spec の `expect(dead_code)` は残っていない。行数 — `actor.rs` 773・`looper.rs` 924・`actor_talk_tests.rs` 353。
- 6.1: 「非表示・`talk` の無い表で状態が増えない」の欄の水準の固定は `looper_talk_tests.rs`（`hidden_scope_keeps_no_state_and_counts_from_its_return`・`cues_change_nothing_when_the_table_has_no_talk`）に在り、アクターの檻は指令と記録の件数で見る。`cargo test -p areka --lib emo2_boot` は 6.1 では回していない（9.1 で確かめる）。
- 7.1: 検体 `crates/areka-seriko/tests/fixtures/trigger-intervals/surfaces.txt` は面 9100〜9104（9100＝一番上に 3 語・9101＋9102＝部品にだけ `talk` と `runonce`・9103＝無効な数値 `talk,abc`／`periodic,0` と大文字混じり `Talk,3`・9104＝コマ列が空）。件数は採録の `debug!` 5（9100 の 0・1・2＋9102 の 0・1）・`warn!` 3（9103 の 0・1＋9104 の 0）。設計の表に足したもの: 9103・9104 の `element0,overlay,surface0.png,0,0`（実機で立ち絵が出るように）と 9103 のコマ行（「採らない」を観測できるように）。`descript` は置かない（emo2 の末尾へ書き足す断片）。
- 7.1 → 9.2: emo2 の `surfaces.txt` の末尾へ丸ごと書き足しても件数は同じ（`warn!` 3・採録 5・emo2 単独は 0・0）で、番号は当たらず絵は全部 emo2 に在ることを E2E `fixture_appended_to_emo2_keeps_the_counts_and_finds_every_picture` が固定している。空のコマ列の記録の検索語は既存の文言「seriko table: コマ列が空のアニメは非採録（要件 8.3）」。
- 7.1: アクターの檻の共有の足場（`ClockRig`・`cue`・`text`・`clear_all`・`frames`・`single_show`・`part_frames`・`top_frames`）は `actor_test_support.rs` へ移した（`actor_talk_tests.rs`・`actor_parts_tests.rs` のテスト本体は不変）。E2E の主な表明は同期の `handle_message`＋手で進める時計で、別スレッドのアクター（`spawn_seriko_clocked`）は 9100 の 1 本。
- 8: 台帳の見張り（`cargo test -p ukadoc-survey`）は、設計が挙げた `assets.toml`・`roadmap-draft.md` のほかに ⑴ `implemented` の項目ごとの正典 URL のコメント（`table.rs` の 3 語の腕の上に 3 行）⑵ `briefing.md` の `[[barrier]]` の数 ⑶ 報告の作り直し（`cargo run -p ukadoc-survey -- report`／`-- report-summary`。道具は 5 本とも LF で書き出すので CRLF へ戻す）を求めた。1 回目の実装者は旧い境界で BLOCKED を返し、要件 10.2（見張りを緑に保つ）の内側として境界を広げて再依頼した。設計の台帳の節にも書き足した。
- 8: 範囲を少し越えて直したもの（査読が受け入れ）— `briefing-assets.md` の表の `always`・`base` の 2 行を「語彙のみ」→「縮退」（別 spec の取り残し・数え直した内訳と表を合わせるため）、`sometimes`／`rarely` の note の「`normalize_interval` が名前として認めるのは 3 つの語」→「6 つの語」。
- 8（完了の棚卸の候補）: ⑴ `always:1` の note の末尾「組み合わせは areka-P0-seriko-trigger-intervals が引き受ける（担当欄）」は担当の付け替えで実態と合わない（要件 10.3 が note を変えないと決めているので触っていない＝`areka-P0-seriko-interval-combinations` へ申し送る）。⑵ 触っていない行の古い決まり文句（`random,数値`・`endtalk`・`starttalk`・`never`・`yen-e` の note の「駆動するのは 2 語だけ」「名前として認めるのは駆動する 3 つの語」）、`briefing-assets.md` の注記 ⑵「駆動する 2 語の一方」と日付つきの行番号、`roadmap-draft.md` の本 spec の `wave = "C5 の候補"`、束「サーフェスアニメーション」の「依存する既存 spec」に `areka-P0-shell-implicit-surface`（2 件）が無いこと。
- 9.1（結果・2026-10-10）: ⑴ `cargo test -p areka-parsers -p areka-seriko -p areka-emo-compose -p areka --no-fail-fast -j 4` を 1 回 — areka 本体 2,895 passed（無視 2）・`emo2_real_run` 1・areka-emo-compose lib 370＋`surface_nesting_fixture_test` 2・areka-parsers lib 565・areka-seriko lib 416＋`balloon_face_e2e` 5・`bind_e2e` 9・`cue_sequence` 1・`loop_integration` 9・`regression` 3。赤は 4 本（`mcp_get_log_real_run` 1・`smoke_boot_loop_exit` 3）で、どれも「i686 の shiori-host32-helper.exe が見つかりません」＝新しいワークツリーの前提不足（areka を起こす前のテストの準備で落ちる）。`cargo build -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` の後、その 2 対象だけ回し直して 5 本とも緑。`cargo test -p ukadoc-survey` は lib 601・cli_streams 6・consistency 113・doc 5 で 0 failed（タスク 8 の直後）。出力は `target\task9.1\`。⑵ 触った `.rs` 32 本の行数の最大は 924（`looper.rs`）・以下 `table.rs` 900・`parts.rs` 893・`looper_talk_tests.rs` 893・`parts_trigger_tests.rs` 852・`parts_tests.rs` 841。⑶ 約束のファイル 12 パス（`crates/areka-parsers/src/sakura`・`crates/areka-sakura/src/compile.rs`・`emo2_boot/{consumer_ledger,mod}.rs`・`crates/areka-emo-compose`・seriko の `timeline`／`state`／`output`／`resolve`／`bind`・`crates/dola`・`crates/areka-emo-text`）は実在し、`git diff --stat` は空（較正: 同じコマンドを `table.rs` に向けると 138 insertions）。`crates/areka` 全体・`Cargo.toml`／`Cargo.lock` も差分なし。
- 9.1: 「既存の決定論のテストは書き替えなし」の例外の一覧（査読がブランチ全体の差分で全ハンクを分類・期待値と乱数の回数の変更は 0）— 語の差し替え `runonce`→`yen-e` 4 ファイル（2.1）／`periodic,3`→`yen-e,3` 1 本（1.1）／`PartClocks::advance` の `None,` 8 か所 4 ファイル（5.2）／`parts_tests.rs` の補助 8 本の `pub(super)` 化（5.1）／`actor_parts_tests.rs`・`actor_talk_tests.rs`・`looper_trigger_tests.rs` からの補助の移動（4.2・7.1）／網羅 `match` の腕の追加 2 か所（`model_tests.rs` の `interval_variants_construct_and_match`・`table.rs` の中の `recorded_anims_satisfy_postconditions`）／既存ファイルへの檻の追加（`looper_parts_tests.rs` の末尾に 2 本・既存の檻は不変）。
- 9.1: 同梱の検体 4 本の `surfaces.txt` に `runonce`／`periodic`／`talk` の interval は 0 件（emo2＝`bind` 30・`bind+random` 3・`random` 4、claudia＝`sometimes` 1、konnoyayame＝`sometimes` 1、R_POST_and_KOMAINU＝0）。検体を読む既存の檻は無変更で緑。
- 7.1 の直し（9.2 の実機で発覚・2026-10-10）: 検体の立ち絵を、emo2 の surface1000 が既定の着せ替えで出す 5 枚（体 `purple/0/base1.png`・目 `purple/4/normal.png`・眉 `purple/3/normal.png`・髪飾り `purple/a/ribbon.png`・閉じた口 `purple/2/niko.png`）の element定義に直した。初版（設計の表）の `surface0.png` は 434×687 の別の 1 枚絵で、口・紅・キラリ（382×547 の立ち絵の顔に合わせた絵）が描き込みの吹き出しの脇に浮いた。9101 の部品 9102 は element1→element4（部品のキラリが目の絵に隠れないように）、9102 の地は `mouthbase.png`→`niko.png`。animation の行・番号・件数（採録 5・`warn!` 3）は不変で、E2E は書き替えなしで緑。上の 7.1 の覚えの「9103・9104 の `element0,overlay,surface0.png,0,0`」はこの形に読み替える。設計の検体の表と README も直した。
- 9.2（結果・2026-10-10・証跡は `target\p-trigger\` の `summary.txt`／`summary2.txt`・走行 A2＝一番上 9100・B2＝部品 9101＋9102 と無効 9103・9104・D＝素の emo2・どれも有界の自動終了で exit 0）: ⑴ `runonce` — 9100 へ入った 3 回（最初・`\s[0]` からの戻り・`\s[-1]` からの戻り）に 1 回ずつ、開始の時刻＝面に入った時刻。同じ面の再指定は 0 回。部品は見えた 2 回に 2 回。⑵ `periodic,2` — 20 件とも「面に入った時刻＋2,000 の倍数」ちょうど（滞在 3 回で 6・2・12 件。同じ面の再指定で起点は動かない）。記録の遅れは最大 9.4 ms。⑶ `talk` — 一番上 `talk,3` は 10 件（入ってから 3・9・12・18・21・27・30 文字目と、戻った後の 3・9・12 文字目。6・15・24 文字目は口の animation 160 ms がまだ再生中なので見送り＝要件 4.7）、部品 `talk,2` は 8 件（長さちょうどで終えた区切りは鳴る枝）。隣り合う開始の差と文字の層の式の差の食い違いは最大 9.0 ms（起点の見積もりが 1 回動いた回）・部品は 0.0 ms。絶対の見積もり（開始の時刻 − 文字の層の記録から求めた t_k）は −16.9〜+1.2 ms（見積もりの誤差は数 ms・16 ms の境に触れる回がある）。`\1` の台詞の間は `\0` の口は 0 件。⑷ 記録 — 検体つきの走行は seriko の `WARN` が 3 件だけ（9103 の `talk,abc`・`periodic,0`、9104 の空のコマ列）・採録の `debug!` 5 件。素の emo2 は 3 語の記録 0 件・seriko の `WARN` 0 件・記録の種類 10 個は全部基点のコミットのソースに在る文言。⑸ 絵 — 口は口の位置・紅は両頬・キラリは両目の上、口が 2 つに見えるコマは無い。9100 と 9101 の静止の絵は emo2 の surface1000 の既定の姿と PNG のハッシュまで同じ。
- 9.2（手順の読み替え）: areka の MCP の `sakurascript` は中身が無い（`NG:not implemented yet`）ので、写しの `boot.lua` の `OnBoot`／`OnFirstBoot` が生の台本 1 本を返す形で流した（`animated-image-playback` の前例）。「前の実行体と同じ」は前の実行体を建てず、新しい実行体で素の emo2 を起こして「3 語の記録 0・記録の種類が増えていない」で確かめた。実機で踏めなかった場面（檻だけが固定）: 改ページ `\x`（areka が未対応）・3 語の面にいる間の着せ替えだけの変化・台詞の中断と新しい台詞・同じ時刻の区切りのまとめ（`\_q` が未対応）。
- 9.2（範囲外・完了の棚卸で `/kiro-discovery` に起票する候補・要件 11.4）: ① MCP の `sakurascript`（と `raise_event`）が中身なし（担当の目当ては `areka-P0-mcp-kanade-tools`）。② `\x` が待たず消しもしない（直後の文字がそのまま続く・`areka-P0-balloon-canon-residue`）。③ `\_q` が効かない（1 字 50 ms のまま・`areka-P0-sakura-time-directives`）。④ `dump_surface` は `\s[-1]` で隠れている間も直前の絵を返す（隠れたことが絵から読めない）。⑤ cue の届く時刻の揺れ（最初の cue を基準に −9〜+50 ms。文字の層も seriko も「今 − cue の時刻」の最大を起点にするので、起点が台詞の途中で 1 回前へ動く）。⑥ 同梱の emo2 そのままで出る起動時の `WARN`（`areka_emo_atlas: bake: element が全透明…purple/a/null.png` 2 件・`areka_emo_text::actor::attach: 折返し基準が描画範囲の外…emo2-kakukaku` 1 件）。⑦ 1 回だけ、起動 3.6 秒で exit 1（SHIORI の LOAD が応答を返さない＝`LoadReturnedFalse`。写しを作り直したら通った・原因は未確認・証拠は `target\p-trigger\run-B2-fail1`）。⑧ 面を離れた瞬間に再生中の animation の「停止」の記録が出ない（`on_surface_changed` が再生を黙って落とす・基点のコミットからの振る舞い）。⑨ 文字の層と seriko が層ごとに自分の到着で起点を取るため、`talk` の開始と文字の現れが最大 17 ms ほどずれて見積もられる（設計「時計」の「1 刻みを超えて見えたら `seriko_clock` の結線の議題として起票」に当たる境目）。参考: `areka_emo_text` の `debug!`（「文字層の再追従」「未指定の origin」）が刻みごとに出て記録の大半を占める（素の emo2 でも同じ・既存）。
- 検証（`/kiro-validate-impl`・2026-10-10）: 判定 GO。⑴ 全体テスト `pwsh -NoProfile -File tools/test-all.ps1` を 1 回 — コミット `978a08eb`・始めた時点の未コミット 0 件・全段が緑（i686 の成果物・`fmt --check`・x64 の全テスト 810 秒・i686 のテスト・crates.io の確認・文字コードの判定）・11,130 passed／0 failed／無視 60。出力は `target\validate\test-all.txt`。⑵ 起動の確かめ — 9.2 の実機の走行 5 本（`areka.exe` が emo2 の写しを起こして有界の自動終了で exit 0）と全体テストの `smoke_boot_loop_exit`。⑶ 要件の網羅 — 受け入れ基準 1.1〜11.4 の全部に実装と檻（または証跡）が在り、基準の本文に反する振る舞いは無い。⑷ 設計・境界 — 依存の向きの違反なし・約束の 12 パスは差分 0・`Cargo.toml`／`Cargo.lock` の差分 0・公開面は `Interval` と `LoopTrigger` の 3 腕だけ・TODO 類 0・本 spec の `expect(dead_code)` 0。設計の文の取りこぼしは「実装との対応」の節にまとめて直した。全体テストの後に入れたのはコメント 2 か所（`decode_tests_animation_tests.rs` の冒頭・`trigger.rs` の `realign_talk` の天井の説明）と文書だけ。
- 検証が拾った軽い点（完了の棚卸で「台帳の note に 1 文」か「起票」かを決める）: ⑴ バルーンの面の番号の切り替え（`\b[番号]`）で 3 語が構え直ることを固定する檻が無い（経路はシェルと共有・窓の開け閉めの檻は在る）。⑵ 窓が閉じると部品の 3 語の時計を末尾のコマ（`Residual`）ごと捨てるので、`-1` で終わらない部品の `runonce` は閉じ→開きで末尾のコマが戻らない（一番上はコマが残る）。⑶ 後から見えた部品の起点は「見えた刻みの時刻」（外側のコマの正確な時刻ではない）・表の差し替えの後の構え直しも次の刻みの時刻。⑷ 文字の層の式が変わっても赤になる檻が無い（写しであって共有でない・実機の差の計測だけが見張り）。⑸ 「同じ刻みで区切りを 2 つ以上越える」は純関数の檻だけ（配線の段と実機は `\_q` 未対応で踏めず）。⑹ 同じ決まりの写し — 「3 語か」の述語が 3 か所（`looper.rs` の `is_trigger_word`・`parts.rs` の `is_trigger`・`table.rs` の `has_triggers` の `matches!`）、窓の開け閉めの写しが `arm_slot` と `fire_part_triggers` に同文、`Armed::arm` の `at_ms` は常に `Some`。⑺ 要件 11.4 の起票（9.2 の ①〜⑨）は `/kiro-complete` の棚卸で行う（まだ起票していない）。
