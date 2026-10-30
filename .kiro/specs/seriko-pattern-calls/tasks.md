# Implementation Plan

- [x] 1. 土台: 欄 2 の原文と 8 語の見分け
- [x] 1.1 pattern定義の欄 2 の原文を読み手が運ぶ
  - pattern定義に欄 2 の原文を持たせ、欄 2 が `(` で始まり `)` を含まないときだけ、`)` を含む欄（無ければ行の終わり）までを継ぎ直して 1 つの原文にする。待ち・X・Y は継ぎ直した欄の次から読む
  - 見出しの行・element定義・collision定義・ほかの描画メソッドの行の読みは変えない。版の宣言には依らない
  - pattern定義を構造体の書き方で作るすべての所（約 24 ファイル・テストが大半・parsers・emo-atlas・emo-compose・emo-present・seriko）に欄を足し、ワークスペース全体がビルドできる
  - `(0,1)`・`( 0 , 1 )`・`(0.1)`・`[0,1]`・`[0.1]`・閉じない `(0`・欄が欠けた `alternativestart,(0,1)`・並びでない `overlay,10,50,0,0` の結果を、欄の読みの兄弟テストで固定し、緑になる
  - _Requirements: 2.2, 2.3, 2.6, 2.7, 9.2_

- [x] 1.2 8 語の見分けと相手の欄の読みを 1 か所に置く
  - 合成の層に、描画メソッドの語が 8 語のどれかを答える照会と、`start`・`stop`・`insert` の番号 1 つ・並びの 4 語の番号の列を読む照会を置く（綴りの比べ方は今の合成メソッドと同じ・記録は出さない）
  - 空・数でない・名前を含む・負の数・空の並びは「読めない相手」として返す。`(…)`／`[…]` の中を `,` か `.` で割り、括弧の無い数だけの欄も並びとして受け、書いた順と重複を保つ
  - 今の「animation を指す語か」「move の語か」の判定を、この照会への薄い呼び出しに寄せる（呼び手は変えない）
  - emo-atlas の着せ替えだけの経路の語の写しに `move` を足し、8 語の欄 2 を面の番号として数えない
  - 語の見分けと欄の読みの兄弟テストが、大文字・`-`・`_`・空白の綴りと、番号・並び・読めない欄の全部の形で緑になる
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 9.2_

- [x] 2. 合成: 着せ替えの手順と差し込み
- [x] 2.1 (P) 着せ替えの描く手順を読み込みのときに 1 度だけ作る
  - 合成の世界を作る終わりに、面 × animation ごとの手順の資源を作る。純 `bind` は全部の pattern を番号の昇順に、`bind+random` は今のまま pattern0 だけをたどる
  - 描く語（今は overlay）で番号が 0 以上のコマは「描く」、純 `bind` の `insert` で相手が同じ面の純 `bind` なら「差し込む」、それ以外は手順に入れない。手順を作るときに記録は出さない
  - 手順の資源の兄弟テストで、3 コマの A（後ろ・`insert,B`・前）、`bind+random` の pattern0 だけ、相手が無い／`bind` でない `insert` が手順に入らないこと、が緑になる
  - _Boundary: BindSteps_
  - _Depends: 1.2_
  - _Requirements: 5.5, 6.5, 6.7, 6.8_

- [x] 2.2 合成が着せ替えの手順を積み、insert を展開する
  - コマの無い有効な着せ替えの静的な寄与を、合成のたびの語の引き直しから、手順を積む形に置き換える
  - `insert,B` の位置に、B が有効なら B の寄与（今のコマがあればそのコマ、無ければ B の手順）を B 自身の座標で積む。B が無効なら何も積まない。差し込まれた B は本来の位置で積まない。差し込む A が無効で展開されなければ B は本来の位置に積む
  - 入れ子は同じ規則で再帰し、1 つの段の中で各着せ替えは 1 度だけ積む（巡回は打ち切る・2 つの差し込み手は描く順で先の方だけ）
  - 着せ替えのコマを合流させない門を `bind+random` だけに狭め、呼ばれて始まった純 `bind` のコマは着せ替えが無効でも描く。有効な純 `bind` のコマが消えれば静的な寄与へ戻る
  - 純 `bind` の全部のコマを描く変更と門を狭める変更で期待が変わる既存の合成のテストは、M6・M5 の測りを根拠に期待を直し、直したテストの名と根拠を作業の記録に残す
  - 合成の兄弟テストで、B が無効・入れ子・巡回・2 つの差し込み手・無効の純 `bind` の今のコマが描かれる・有効の純 `bind` が止められても残る、が緑になり、合成を何度組んでも「未知の合成メソッド名」の記録が 0 件である（ログの捕まえで確かめる）。emo-compose のテストが全部緑である
  - _Requirements: 5.5, 5.6, 5.7, 6.1, 6.2, 6.3, 6.4, 6.7, 6.8, 8.1, 9.5_

- [x] 2.3 外形と入れ子の辺を同じ規則に揃える
  - 外形の数えで、純 `bind` を `always` と同じく全部の pattern で数える（8 語と負の番号は数えない）
  - 入れ子の辺の相手を手順の資源の「描く」コマから作り、入れ子の側の合流の門も `bind+random` だけに狭める
  - 外形と入れ子の変更で期待が変わる既存のテストだけを、M6 の測りを根拠に直し、直したテストの名を作業の記録に残す
  - emo-compose のテストが全部緑のままである
  - _Requirements: 5.6, 6.3, 6.8, 9.5_

- [x] 3. 再生表: 働きのコマと呼ばれるだけの animation
- [x] 3.1 (P) 再生表のコマを描くコマと働きのコマに分ける
  - 再生表のコマに働き（描く・開始・停止・ずらす・何もしない）を持たせる。呼び出しの 6 語は相手の列と「1 つを選ぶか」を持ち、待ちを 0 にする。`move` は待ちと X・Y をそのまま持ち、欄 2 を読まない。`insert` と読めない相手は何もしないコマで、待ちは 0
  - 8 語以外の語は今と同じに解き、今と同じ記録を出す。働きのコマには記録なしの未知の合成メソッドを入れ、描く面を持たせない（絵の番号は無し・面の番号として数えない）
  - interval の純 `bind` と `never` を「呼ばれるだけ」として表に載せ、未知の interval は今のまま載せない。動く部品の数えから働きのコマを外す
  - 「呼ばれるだけ」を、抽選では `always` と同じく乱数を引く前に飛ばし、引き金の判定（一番上・部品の両方）にも腕を足す。再生表のコマを書く所（約 8 か所）を直してビルドが通る
  - 純 `bind` を表に載せないことを前提にした既存のテスト（表のテスト・着せ替えの状態と actor のテスト・着せ替えの通しのテストなど）の期待を直し、名と根拠を作業の記録に残す
  - 再生表の兄弟テストで、語ごとの働き・待ちの 0・働きのコマに描く面が無い・`never` と純 `bind` が呼ばれるだけ・未知の interval が載らない、が緑になり、seriko のテストが全部緑である
  - _Boundary: TableCalls_
  - _Depends: 1.2_
  - _Requirements: 1.1, 1.3, 1.5, 2.4, 7.4, 11.1, 11.3, 11.4_

- [x] 3.2 8 語の誤りを読み込みのときに 1 度だけ記録する
  - 面ごとの表を作った後に、読めない相手・表に無い相手・純 `bind` の呼び出しと `move`・純 `bind` でない animation の `insert`・相手が無いか純 `bind` でない `insert`・巡回する `insert` の 6 種を検査し、1 件ごとに 1 度だけ警告を出す（面・animation・pattern・語・原文の欄）。そのコマは何もしない
  - 表を作るのはシェルで 1 度、バルーンはスコープの読み込みごとに 1 度なので、刻みや合成のたびには出ない
  - 再生表の兄弟テストで、6 種がそれぞれ 1 度だけ出ること、表を使って刻みを回しても増えないこと、が緑になる
  - _Requirements: 2.5, 5.1, 5.3, 5.4, 6.4, 6.5, 6.6, 8.3_

- [x] 3.3 働きのコマで絵を重ねず、前の絵を残す
  - コマの計算の「`-1` で止まる」判定を描くコマの負の番号だけにする（`move,-1,…` で止めない）
  - ある位置以下で最後の描くコマを答える照会を足し、一番上と部品が共有できるようにする
  - 時の流れの兄弟テストで、`move` や呼び出しのコマが今のコマでも前の描くコマ（`base,2` を含む）が選ばれ、描くコマが無ければ欄が外れる、が緑になる
  - _Requirements: 1.3, 7.7_

- [x] 4. 呼び出しの係と提示の覚え
- [x] 4.1 働きのコマを予定の時刻の順に当てる純粋な係を作る
  - 再生中の各 animation の、まだ当てていない働きのコマのうち予定の時刻（開始 ＋ 累積の待ち・`always` は周ごと）が今以下のものを、時刻・animation・pattern の昇順の待ち行列で 1 つずつ当てる。止められた／やり直された再生の出来事は捨てる
  - 開始は相手の再生を予定の時刻で置き換え（頭からやり直す）、相手の頭の働きのコマも同じ待ち行列に入れる。停止は再生と欄を外し、再生していなければ何もしない。選びは選びの乱数だけで並びの全部から等しく 1 つを選ぶ。`move` は後の時刻のずれが勝つ候補を返す
  - 1 つの時刻の中で同じ animation を 2 度始めない（`debug` だけ）。表に無い相手は何もしない。呼んだ側の再生は止めない
  - 再生中の表を見る口を抽象にし、兄弟テストは偽の表で、遅れた刻みでも予定の時刻で始まる・再生中の相手のやり直し・parallel の同じ時刻・決め打った選び・`alternativestop` が再生していない相手を選ぶと何もしない・自己呼び出しと呼び合いで止まる・止めた後の出来事を捨てる・`move` の最後の値、が緑になる
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.9, 4.1, 4.2, 4.3, 4.4, 5.2, 7.2, 7.3, 8.1, 9.1_

- [x] 4.2 (P) 提示がずれを対象ごとに覚えて読ませる
  - 提示の命令にずれ（対象・x・y・応答なし）を足し、提示の拠点が対象の欄に書く。合成・可視性・合成の覚えには触らない。未装着の対象は今の段の応答と同じく捨てる（`debug`）
  - 提示の読みに対象のずれを返す口を足す（未登録は 0, 0）。命令の畳みでずれが区切りとして残り、順が崩れない
  - 提示の兄弟テストで、ずれが対象ごとに覚えられ、合成の覚えが変わらない、が緑になる
  - _Boundary: MoveShift の伝達（emo-present）_
  - _Requirements: 7.1, 7.6_

- [x] 5. 再生への組み込み
- [x] 5.1 選びの乱数と当て終えた時刻の欄を足す
  - 再生の設定に選びの乱数を足し、設定を書くすべての所を直す（止めた設定と書き出しは種 0）。seriko の中だけでなく、areka の起動（`emo2_boot` の本体・背骨・フィルムの通しのテスト）にも書く所があるので、この task は areka の起動の層へまたがる機械的な追随を含む
  - 本番は抽選とは別の種で作り、今の起動の記録に 2 つの種を並べて出す。背骨の「生きた駆動」は抽選の乱数と選びの乱数の 2 つを運ぶ形にし、呼び手（起動の本体とテスト）が 2 つを渡す
  - 一番上の再生と部品の再生に「働きのコマを当て終えた時刻」を足し、作る所を直す
  - ワークスペース全体がビルドでき、既存の seriko と areka のテストが緑のままである
  - _Depends: 3.1_
  - _Requirements: 3.6, 3.7_

- [x] 5.2 一番上とバルーンの面の刻みと合図で呼び出しを当てる
  - 面の段ごとに、引き金の判定と `always` の始まりの後、コマを置く前に係を呼ぶ。台本の合図の時刻で動く更新でも頭のコマを置く前に呼ぶ
  - 一番上の再生の表と作りかけの欄を一緒に借りる口を実装する（停止で欄も外す）。終わって最後のコマを残したものは再生中に数えない
  - `always` を止めたら、止めた印がある間は始め直さない。印は面を出したとき（同じ面の出し直しを含む）に消す。そのために、再生の実行時の「面を出した」口をこの task で作り、actor から面を出す合図を解いた後（変わった・同じ面の出し直し・シェルの差し替えのどれでも）に呼ぶ（5.4 はこの口にずれの戻しを足すだけ）
  - コマを置く段は、今のコマが働きのコマなら最後の描くコマを置く
  - 結合の兄弟テストで、「．さくら」の形の `alternativestart` で animation0 か 1 が呼び出しの時刻に頭から始まり同じ刻みの表示に載る・`stop` の見え方が `-1` と同じ・止めた `always` が面の出し直しまで止まる・同じ種で `never` を足しても抽選の結果が同じ・バルーンの面でも同じに動く、が緑になる
  - _Depends: 3.3, 4.1_
  - _Requirements: 3.1, 3.4, 3.5, 3.6, 3.8, 3.9, 4.1, 4.5, 11.2, 11.3, 9.1_

- [x] 5.3 入れ子の部品の刻みと合図で呼び出しを当てる
  - 部品の時計の進みと更新の中で、その部品の再生の表と欄を借りる口として係を呼ぶ（相手は同じ部品の面の animation）。更新でも頭のコマを置く前に呼ぶ
  - 部品の進みは、今のコマが働きのコマなら最後の描くコマを置く
  - 部品の結合の兄弟テストで、部品の中の `start` と `stop` が一番上と同じ規則で働く、が緑になる
  - _Depends: 3.3, 4.1, 5.1_
  - _Requirements: 3.8, 11.2, 9.1_

- [x] 5.4 ずれの持ち主を作り、表示の命令で外へ出す
  - 再生の実行時にスコープ × 面の段ごとのずれを持つ。一番上と部品の結果のうち時刻が後のずれをその段のずれにし、値が変わったら絵の確定の後にシェル用／バルーン用のずれの命令を出す（同じ値は繰り返さない）
  - 5.2 で作った「面を出した」口に、ずれが 0 でなければ 0 にして (0, 0) の命令を返す処理を足し、actor がそれを出す。終わり・`stop`・`-1` では戻さない
  - areka の写しの層に 2 つの腕を足し、シェルとバルーンの対象への提示のずれの命令に写す（数でないスコープは今のまま捨てて警告）。この task はこの 1 か所だけ areka の層へまたがる結合である
  - 結合の兄弟テストで、ずれの命令の出る時・同じ面の出し直しで (0, 0) が出る・部品の `move` もキャラクター全体のずれになる・オフラインの書き出しの結果にずれが無い、が緑になり、写しのテストで提示の命令に写る
  - _Depends: 4.2_
  - _Requirements: 3.8, 7.1, 7.2, 7.6, 9.6, 8.1_

- [ ] 6. 置き方: ずれを窓の位置へ当てる
- [ ] 6.1 (P) ずれを窓の位置へ当てる口と経路を作る
  - キャラ窓とバルーン窓に「欲しいずれ・倍率・当てた分」の記録を持たせる。キャラ窓は、元の位置（今の位置 − 当てた分）に倍率をかけたずれ（`i32` に飽和）を足し、下揃えの揃え直し・作業領域の中への寄せ・見える範囲の守りの順に通して、新しい経路で位置だけを書き、実際に書いた位置 − 元の位置を当てた分にする
  - バルーン窓は当てた分を記録し、キャラ窓の今の位置で追い直す。バルーンの追いの提案に、そのずれを足す 1 行を入れる
  - 新しい経路を表示の側の置き直しとして、利用者が動かしたと数えない判定・見える範囲の守りを当てる経路・見送りが覆う経路に入れる
  - 窓・位置・作業領域が無ければ警告を出して当てない（次のフレームで当て直す）
  - 置き方の兄弟テストで、倍率をかける・下揃えで y が揃いに負ける・揃えない置き方でずれる・`move,0,10,0,9999` の形が作業領域の中へ寄る・0 で元へ戻る・バルーンが追う、が緑になる
  - _Boundary: MoveShiftPlacement（areka placement）_
  - _Requirements: 7.1, 7.3, 7.5, 7.8, 8.1_

- [ ] 6.2 利用者のドラッグと位置の保存でずれを扱う
  - キャラ窓とバルーン窓のドラッグの終わりで、その窓の当てた分を 0 にする（バルーンは 0 にしてから相対の位置を作る）。上限に近いファイルには口を呼ぶ 1〜2 行だけを足す
  - 位置を保存する所（覚えていないキャラ位置の保存・組の相手の保存）は、当てた分を引いた元の位置を保存する
  - 置き方の兄弟テストで、ドラッグの後のずれ 0（キャラ窓とバルーン窓の両方・バルーンで二重に足されない）と、保存の値がずれを引くこと、が緑になり、触ったファイルが 1,000 行を超えない
  - _Requirements: 7.5, 7.8_

- [ ] 6.3 ずれの相をフレームに組み込む
  - 毎フレーム、移動の相の直後に、各ゴーストの窓について提示のずれと当てた倍率を読み、倍率がまだ無ければ何もせず、記録と同じなら何もせず、窓の書き込みの見送りの門の間は次のフレームへ回し、そうでなければ 6.1 の口で当てる
  - ずれの相の兄弟テストで、提示のずれが窓へ当たる・値と倍率が変わったフレームだけ書く・見送りの間は書かずに後で当てる、が緑になる
  - _Depends: 4.2, 5.4, 6.1_
  - _Requirements: 3.8, 7.1, 7.3, 7.6_

- [ ] 7. 網羅台帳と互換の文書を更新する
  - 網羅台帳の `start,ID`・`stop,ID`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert,ID`・`move` の 8 行と interval の `never` の行を、実装した状態と担当の spec にする
  - 互換の文書の §8 に、等しい確からしさ・巡回の止め・`insert` の行の座標を使わない・使えない所は何もせず記録・M1〜M8 の結果・`move` に倍率をかける（SSP はかけない）・着せ替えを純 `bind` と読む・止めた `always` は面の出し直しまで止まる・差し込む側が無効なら B は本来の位置、を書く
  - 台帳の検査（ukadoc-survey の検査）が通る
  - _Requirements: 10.1, 10.2, 8.2_

- [ ] 8. 確かめ
- [ ] 8.1 「．さくら」のシェルで 8 語の警告が 0 件であることを固定する
  - 検体の「．さくら」のシェル `master` から再生表と合成の形の記録を作り、8 語に由来する警告が 0 件で、`surface0` の animation2 が「並び [0, 1] から 1 つを選んで開始」であることを兄弟テストで固定し、緑になる
  - _Depends: 2.2, 3.2_
  - _Requirements: 1.4, 9.3_

- [ ] 8.2 既存のテストを全部通す
  - parsers・emo-atlas・emo-compose・seriko・emo-present・areka のテストと clippy を通す。期待を直したテストがあれば名と根拠（M6 など）を作業の記録に残す
  - 対象のクレートのテストが全部緑である
  - _Requirements: 9.5_

- [ ] 8.3 「．さくら」を実機で 1 回動かして確かめる
  - 根・検体・一時フォルダはワークツリーの `target\` の下だけ。絶対パスで起こし、seriko の `debug` まで記録を開ける
  - 記録に 8 語に由来する WARN が 0 件、`surface0` などで `alternativestart` が animation0 か 1 を始めた記録、areka の MCP で `\s[12]`・`\s[22]` を出して揃えない置き方では窓の矩形が `move` のとおり下へずれ（`9999` は作業領域の中へ戻る）、下揃えではずれが見えないこと、書き出しの写しにずれが写らないこと、を記録と窓の矩形で確かめて作業の記録に残す
  - _Requirements: 9.4, 9.6_

## Implementation Notes
- 1.1: 欄 2 の原文（`Pattern.target`）は欄ごとに前後の空白を落としてから `,` で継ぐので、`( 0 , 1 )` は `( 0,1 )` になる。並びを読む側（1.2）は項目ごとに空白を落とすこと。
- 1.1: `cargo clippy --no-deps` で `areka-seriko` の `actor.rs:89`・`looper.rs:426`、依存込みで `dola` にもとからの指摘がある（この spec の差分の外）。
- 1.2: 8 語の見分けは `areka_emo_compose::pattern_word`（`pattern_word`・`call_targets`）。`start`/`stop`/`insert` の括弧つきの番号 `(1)` は読めない相手。`clippy --all-targets --no-deps` は emo-compose のテストのファイル（触っていない）に `chunks_exact` の指摘が 5 件もとからある（`tools/test-all.ps1` は clippy を回さない）。
- 2.1: 描く語の判定は design の `quiet_method` でなく既存の `method::is_implemented_name`（記録なし・Overlay だけ真）。`BindStep`・`EmoWorld::bind_steps` の `#[allow(dead_code)]` は 2.2 で使い始めたら外す。自分自身への `insert` は手順に残り、巡回の打ち切りは 2.2 の展開側。
- 2.2: 入れ子の側の合流の門（nesting.rs の ③・`SurfaceParts.bind_random_ids`）は 2.2 で前倒し済み。2.3 に残るのは入れ子の辺の相手を BindSteps の「描く」コマから作ることと外形（plan_extent.rs）の数え。
- 2.2: 期待を直したテスト（根拠）: plan_ops_tests::coexisting_pattern0_and_pattern1_composes_only_index0（M6）・golden_tests_blink_static_tests::emo2_blink_binds_are_statically_inactive（M6・1403 の pattern2 が増える）・plan_nesting_tests::part_frames_go_through_the_same_bind_and_method_rules（M5）・plan_ops_tests::non_overlay_pattern0_logs_debug_and_is_not_drawn（5.5）・plan_issues_tests::composing_repeatedly_adds_no_warn（5.5）・nesting_visible_tests::inactive_bind_frames_are_not_counted → inactive_bind_random_frames_are_not_counted（M5）・nesting_visible_tests::fixture_table_lists_children_and_bind_targets／bind_targets_keep_only_drawable_pattern0（M5）・nesting_visible_tests::inactive_bind_frames_change_neither_ops_nor_parts（M5）。
- 2.2: 「有効な A に今のコマがある間は A の手順を展開せず B は本来の位置」は SSP で測っていない読み（設計 453 行と 424 行の組み合わせ）。7 の互換の文書 §8 に「展開されない（無効・コマあり）とき」と書く。合成のたびの debug「有効な着せ替えに描く手順が無い」は旧来の「pattern0 が無い」と同じ頻度。
- 2.3: 期待を直したテスト: nesting_visible_tests::bind_targets_keep_only_drawable_pattern0 → bind_targets_keep_only_drawable_steps（[(5,3)] → [(4,3),(5,3)]・M6）。`bind+random` の外形は今のまま最小の番号の pattern（`min_by_key`）で数える（pattern0 の無い瞬きを大きさに数える既存の意図）。
- 3.1: 働きは `FrameAct`（`table_calls.rs`）・呼ばれるだけは `LoopTrigger::CallOnly`。部品の番人は `CallOnly` を `Gate::Trigger` に写す（在る時計を進めるだけ・自分では始めない・乱数なし）。`Gate::Off` は `bind+random` で外れたときだけ。見えなくなった部品の `CallOnly` の時計は `drop_transient_where`（`is_trigger`）で捨てられず残る — 5.3 で決めてテストで固定する。
- 3.1: 期待を直したテスト: table::tests::only_random_and_bindrandom_are_recorded_others_debug_logged・table::always_tests::combinations_and_other_words_keep_the_debug_arm（11.1）。純 `bind` を使う actor_clock_tests（BIND_ALWAYS）・table_parts_tests の bind の件は「表に載らない」を前提にしておらず（着せ替えのコマは compose の BindSet・一番上の looper は CallOnly を抽選しない・has_animated_parts は描くコマで決まる）期待を変えずに緑。働きのコマは `p.surface_id` を保つので `move,-1,…` の止まりは 3.3 で直す。looper.rs 927・table.rs 926・parts.rs 896・parts_tests.rs 905 行で上限が近い。
- 申し送り（10-30・animated-surface-log-flood から）: ぱすたの 1400（`bind+random`・pattern0 が閉じた目）が再生していないあいだも pattern0 が静的に描かれ、目が閉じたまま戻らない。あちらが先に main で「`bind+random` は静的に積まない（コマがあるときだけ描く）」を直す。main を取り込むときは `bind_steps.rs` の `BindSteps::build` の `bind+random` の枝を空の手順にして合わせる。
- 3.2: 8 語の検査は `table_calls.rs` の `check_calls`（表を作る `from_world_and_films` の中で 1 度）。巡回する `insert` は巡回の上のコマごとに 1 件（A↔B で 2 件）。純 `bind` の中の働きのコマは `Inert`（待ち・x・y 0）に書き換える。8.1 の「．さくら」の確かめはこの 6 種の WARN が 0 件であることを見る。
- 3.3: `timeline::visible_frame`（`#[allow(dead_code)]`・5.2/5.3 で使い始めたら外す）。`always_at` にも同じ「描くコマの負の番号だけで止まる」を当てた。5.2/5.3 まで、働きのコマが今のコマになると looper の `pattern_frame` がその `surface_id` を欄に置く途中の状態（`move,-1` なら u32::MAX）— 5.2/5.3 で `visible_frame` に置き換える。`always` の 2 周目以降で周の頭が働きのコマだと `visible_frame` は None になるので、前の周の描くコマを残すかは 5.2 で扱う。`looper.rs` の `pattern_frame` の doc「負は Stopped」も 5.2 で直す。
- 4.1: 係は `calls.rs` の `run_calls`（口は `CallPlays`・`mod calls` に `#[allow(dead_code)]`、5.2/5.3 で外す）。`CallPlays::start` の実装は当て終えた印を None に戻すこと。止めた `always` の印は 5.2 の `CallPlays::stop` の実装側で付ける。5.2 は抽選・`always`・引き金の始まりを係の呼び出しより前に置く（同じ時刻の 2 度目の開始の止めはこの順が前提）。
- 4.2: `PresentCommand::MoveShift { target, x, y }`・読み口 `EmoPresenter::move_shift(target)`。`ReplaceTarget` ではずれを引き継がず 0 から（design 596 行・5.4 で差し替え時に (0,0) を送る）。areka の背骨の写し `spine_conformance_support.rs` の `project_display` では MoveShift は `Unknown` に写る — 5.4 で要るなら足す。
- 5.1: `SerikoLoopConfig.pick_rng`（止めた設定・`settle.rs` の書き出し・テストは種 0）・本番は `entropy_seed()` を 2 回引いて `seed` と `pick_seed` を同じ info に出す。背骨は `LoopDriver::Live(rng, pick_rng)`。`Playback.acts_through`・`PartAnim::Playing.acts_through`（`Option<u64>`・`#[allow(dead_code)]`）と `on_tick` の `pick_rng: _` は 5.2/5.3 で使い始めて外す。actor_bind_loop_tests.rs はもとから 1,346 行（上限超え）。
- 5.2: 一番上の口は `looper_calls.rs` の `TopPlays`・`run_top_calls`・`put_visible`。「面を出した」口は `looper.rs` の `on_surface_shown(scope, slot) -> bool`（5.4 で設計どおり `Option<DisplayCommand>` にしてずれの戻しを足す）。`CallOutcome.shift` はまだ誰も読まない（5.4）。actor の差し替え（Replace／Rebased）の枝で全スコープへ口を呼ぶ処理にテストが無い — 5.4 で 1 本足す。部品の `acts_through` の allow は 5.3 で外す。looper.rs は 979 行。
- 5.3: 部品の口は `parts_calls.rs` の `PartPlays`・`run_part_calls`（働きのコマの無い部品は飛ばす）。`always` の周の見え方は `timeline::always_visible` を一番上と共有。見えなくなった部品の `CallOnly` の時計は抽選の時計と同じく残す（`called_never_clock_survives_while_the_part_is_hidden`）。部品の `CallOutcome`（`move`）はまだ捨てている — 5.4 で読む。引き金の相（runonce/periodic/talk の後に呼び出しを当て直す道）は一番上・部品ともテストが無い — 5.4 か最終検証で 1 本。parts.rs 962・looper.rs 981 行。
- 5.4: `on_surface_shown` は `bool` のまま（5.2 の注の「Option にする」は取り下げ）。ずれの持ち主は `looper_calls.rs` の `Shifts`、(0, 0) は送り箱に積んで `take_shifts` で合図の `Show`／`Rebased` の後に出す（design.md の契約の行を追随済み）。命令は `DisplayCommand::MoveShift`／`MoveShiftBalloon`、areka の `adapter.rs` で `PresentCommand::MoveShift` に写す。`\s[-1]`（Hide）でもずれは 0 へ戻る（SSP で未測定・7 の互換の文書に 1 行）。背骨の写し `project_display` は MoveShift を `Unknown` のまま。引き金の相（periodic/talk の後の当て直し）の刻みのテストはまだ無い — 最終検証に持ち越し。looper.rs 993・parts.rs 973・actor.rs 908 行。
