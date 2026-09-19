# Implementation Plan

> 本計画は `.kiro/specs/areka-P0-popup-menu-minimal/design.md` の File Structure Plan と Requirements Traceability に対応する。行数・増分は着手時に `wc -l` で引き直す。新しい外部クレートは足さない。
> 要件 `7.2a`（閉じた後に飛びを起こさない）は数字だけの ID ではないため、design.md と同じく要件 `7.2` の補足として同じタスク（7.3）に対応付ける。

- [x] 1. 基盤: 後続すべてが乗る 4 つの口を開ける
- [x] 1.1 (P) ポインタの「離した」を 1 フレームの旗として配る
  - ポインタの状態に「離した」ボタンの旗（左・右・中・X1・X2）と「どれか立っているか」を足し、ダブルクリックやホイールと同じ 1 フレーム限りの寿命にする
  - 入力段の写しで、押下と解放が同じ tick に入っても解放を落とさない独立の文として旗を立てる（既存の押下優先の分岐は変えない）
  - 配送段で、押下の配送の後に「離した」ハンドラを同じ経路（Tunnel→Bubble）で配り、配送の末尾で旗を消す
  - 完了状態: 解放だけで解放ハンドラが 1 回・同 tick の押下＋解放で押下と解放の両方が届き・配送後に旗が消えていることが wintf の配送テストで緑になり、既存の「配送後に状態が消える」テストが解放の旗も見張っている
  - _Requirements: 1.1_
  - _Boundary: wintf pointer_

- [x] 1.2 (P) 返事を待たずに覗ける受け口を足す
  - アクター往復の受信端に「今届いているなら受け取る・届いていなければ空を返す・送信端が落ちていれば切断を返す」非待機の取り出しを足す
  - 既存の待つ受け取り・期限付きの受け取りの振る舞いと署名は変えない
  - 完了状態: 空・送信後・送信端の消滅の 3 通りが同ファイルの兄弟テストで緑になり、既存テストが無改変で通る
  - _Requirements: 3.2, 3.4_
  - _Boundary: areka-actor reply_

- [x] 1.3 (P) ゴースト定義の説明書キーを転記して UI から読めるようにする
  - ゴースト定義の書き写し模型に説明書ファイル名の欄を足し（正典 URL のコメント 1 行付き）、解決層はキーの値をそのまま写すだけにする（存在確認も既定値の補いもしない）
  - 実行時のゴーストから書き写し模型を読む公開アクセサを足す
  - 完了状態: 解決層の兄弟テストで「キーあり→値・キーなし→無し」が緑になり、UI 側から説明書キーを読める
  - _Requirements: 4.1, 9.4, 10.4_
  - _Boundary: areka-parsers package, areka-ghost runtime_

- [x] 1.4 横断改変: 終了理由にスコープ番号を載せる
  - 終了理由の「利用者」にスコープ番号を持たせ、終了の握手の問い合わせが Ref0 に `user`・Ref1 と Ref2 にスコープ番号を載せるようにする（`system` は従来どおり Ref0 のみ・強制退避の通知は変えない）
  - 運行の状態機械（保留・締切・遷移）の本文は変えず、終了理由を組み立てている全箇所（areka 本体・背骨の台本と期待列・kanade と ghost のテスト）を追随させる
  - 責務をまたぐ機械的な一斉改変であることを明示し、以降のタスクはこの形を前提にする
  - 完了状態: ワークスペースのテストが緑に戻り、終了の握手のテストがスコープ 1 の窓で `["user","1","1"]`、`system` で `["system"]` を期待している
  - _Requirements: 5.1, 5.2, 5.3_

- [x] 2. 運行側: 任意の時点で SHIORI リソースを引く入口
- [x] 2.1 許可名を 10 に広げ、汎用のリソース取得を作る
  - 許可名を「利用者名＋枠 7 種の項目名リソース＋本体側／相方側の表示可否」の 10 名にし、各要素の直上に正典 URL のコメントを 1 行ずつ置く
  - 任意の id からリソース問い合わせを組み立てる汎用関数を足し、既存の利用者名の取得をその上に畳む
  - 許可名の集合を逐語で凍結しているテストを新しい集合に書き換え、名前も実態に合わせて改める
  - 完了状態: 凍結テストが新しい 10 名で緑になり、許可されない id は従来どおり拒否される
  - _Requirements: 3.2, 10.4_
  - _Boundary: areka-kanade schedule/resources_

- [x] 2.2 殻で答える複数件の照会を足す
  - 運行への指示に「id の列と返信端を渡す照会」を足す（指示の種類数を数えている既存テストを追随）
  - 殻の受け口で、終了指示と同じ並びに照会の腕を足し、状態機械を経ずにその場で答える
  - 会話できる状態のときだけ SHIORI へ往復し、それ以外（起動中・終了中・停止後）は SHIORI へ送らず全件「値なし」を返す。状態の判定は純粋関数として切り出す
  - 応答は入力の id と同じ順・同じ長さで 1 回だけ返し、受信側が待ちを諦めていても落ちない
  - 完了状態: 状態判定の純粋関数の兄弟テストが「会話できる状態だけ真」で緑になり、照会が状態機械の再投入に乗らない経路を通ることがコードで示される
  - _Requirements: 3.2, 3.3, 3.4, 3.10_
  - _Depends: 2.1_
  - _Boundary: areka-kanade actor shell_

- [x] 2.3 偽 SHIORI で照会の 4 通りを確かめる
  - 既存の運行テストの道具で起動を済ませたあと照会を送り、項目名リソースが 値を返す／空を返す／値なしを返す／失敗する の 4 通りで結果語彙が対応することを確かめる
  - 起動の前（会話できる状態になる前）に照会を送ると、SHIORI へ送らず全件「値なし」が返ることを確かめる（起動の途中は受信箱から観測できない＝Implementation Notes 2.3）
  - 任意の id の応答を注入する口が道具側に無ければ、既存の応答表と同型の注入口をテスト支援に足す（本番コードは触らない）
  - 完了状態: 5 本のテストが緑で、応答の種類を取り違えるとそのうち少なくとも 1 本が赤になる
  - _Requirements: 9.2_
  - _Depends: 2.2_

- [x] 3. メニューの純粋な構造
- [x] 3.1 module の骨組みを置き、登記の口を作る
  - 以降のタスクが宣言行で衝突しないよう、crate 根へ本仕様の 2 module の宣言を、メニュー module へ配下 4 ファイル（計画・項目名・引き金・OS 表示）の宣言と空の雛形を、このタスクで置く（以降のタスクは中身を埋めるだけ）
  - 枠 7 種の順序付き列挙（並びの権威は 1 か所）、項目の単位（既定名・文言リソース名・有効／無効・チェック・動作または子項目の列）、枠ごとに 1 つだけ持つ供給関数、登記・取り消し・写しの取得を作る
  - 同じ枠への 2 度目の登記は後勝ちで置き換え、そのことを記録する。取り消した枠は写しに現れない
  - 写しは登記順ではなく枠の並び順で、供給関数をその場で呼んで作る（登記時の値を使い回さない）
  - 完了状態: 兄弟テストで「置き換えの記録が出る・取り消すと消える・写しが枠の並び順になる」が緑
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 3.9_
  - _Boundary: areka menu（module の骨組み＋登記）, crate 根の module 宣言_

- [x] 3.2 写しと名前から表示直前の計画を作る（純粋）
  - 枠の並び・群ごとの区切り線・識別子の払い出し（1 起点・深さ優先の出現順・サブメニューの見出しは持たない）・識別子から動作への逆引きを、OS の型に触れずに作る
  - 文言は「リソースの値が非空ならそのまま（アクセラレータ記法を素通し）・それ以外は登記された既定名をアクセラレータ用に写す」で決め、子項目にも同じ規則を適用する
  - 無効とチェックを計画にそのまま写す
  - 完了状態: 兄弟テストで 7 枠の並び・未登記の枠が出ない・子項目の順とチェック位置・無効の写し・識別子の一意性と逆引き・区切りの位置・アクセラレータ記法の 2 系統が緑（要件 9.1 の全項目）
  - _Requirements: 2.1, 2.3, 2.4, 2.5, 2.6, 2.7, 3.5, 6.7, 9.1_
  - _Depends: 3.1_
  - _Boundary: areka menu plan_

- [x] 4. 説明書
- [x] 4.1 説明書のファイルを決めて既定のアプリで開く
  - ゴーストのフォルダの根と定義キーから説明書のパスを決める（キーが無ければ正典の既定名）。中身も文字コードも読まない
  - UI 側の持ち物として、パス・台本からの要求の受信端・「無いことを 1 度だけ記録した」印を持つ資源と、その結線（毎 tick の取り出しを入力の段へ登録）を作る
  - 在否の判定は共有借用だけで済む形にし、初めて「無い」を見たときだけ記録する
  - 既定のアプリで開く関数は戻り値が失敗を示したらパスとコード付きで記録し、ゴーストの動作を続ける
  - 完了状態: 兄弟テストで「キーあり／なしのパス決定」と「ファイルを置く／置かないで在否が変わり、記録は初回だけ」が緑。開く関数は OS に触れるので檻に入れず実機確認へ回す
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.7, 9.4, 11.4_
  - _Depends: 1.3, 3.1_

- [x] 4.2 台本の説明書コマンドの消費者を足す
  - 汎用コマンド運搬から名前で自己選別し、引数なしのときだけ UI へ要求を 1 件送る。引数付きは記録して何もしない。他の名前・非キャリアは良性スキップ
  - 消費者台帳に名前と選別子の行を 1 行足し、正典 URL のコメントはその分岐の定義側に置く
  - 完了状態: 兄弟テストで「引数なし→1 件・引数付き→0 件＋記録・他の名前→0 件」が緑になり、台帳の一意性テストが新しい行を含んで緑
  - _Requirements: 4.5, 4.6, 9.5, 10.4_
  - _Depends: 4.1_

- [x] 4.3 説明書の 2 つの入口を起動時に結ぶ
  - 起動成功後の結線で、消費者を運搬の受け手の列に足し、ゴーストの根と定義キーから決めたパスで UI 側の資源を結線する（送出端と受信端を対で作る）
  - 完了状態: 実機起動で台本の説明書コマンドが UI 側へ届く経路が繋がり、既存の結線テスト・背骨テストが緑のまま
  - _Requirements: 4.5_
  - _Depends: 4.1, 4.2_

- [x] 5. 項目名と表示可否
- [x] 5.1 リソース名の表と、返り値の写し・表示可否の判定
  - 枠 7 種とリソース名と既定名の対応表、表示可否のリソース名をスコープで選ぶ関数、問い合わせない 4 名の表を置く（それぞれ正典 URL のコメント付き）
  - 写しに現れる文言リソースと表示可否の 1 名を集める列挙、運行へ「送るだけで待たない」照会の送出、待ちの上限の定数を作る
  - 返り値の写しは 値が非空→文言／空・値なし→既定名＋記録／失敗→既定名 とし、失敗した id は 1 回の表示につき 1 行にまとめて記録する。表示可否は値が `0` のときだけ抑止＋記録、それ以外（値なし・失敗・上限超過を含む）は表示
  - 完了状態: 兄弟テストで 4 通りの写し・失敗 2 件でも記録が 1 行・上限超過は全件既定名・表示可否の 4 通り・第 1 スライスの照会は 3 件・問い合わせない 4 名が許可表に 1 つも無いこと が緑（要件 9.2 の写し側・9.3 の表示可否・3.8 の判定）
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.6, 3.7, 3.8, 3.10, 10.4_
  - _Depends: 2.2, 1.2, 3.1_
  - _Boundary: areka menu captions_

- [x] 6. 右クリックと右ダブルクリックの規則
- [x] 6.1 右ダブルクリックを預かる箱と終了指示のスコープ
  - 窓のスコープを引く関数と終了指示の送出をモジュール外から呼べる可視性にする
  - 押下ハンドラの右ダブルクリックは即送出をやめ、材料（スコープ・面座標・領域）を高々 1 件預かる形にする（前の預かりは捨てて上書き）。左ダブルクリックは従来どおり
  - Ctrl＋左ダブルクリックの終了指示に窓のスコープを載せる（Ctrl と Ctrl＋Shift の入口は残す）
  - 送るかどうかを決めるのはメニュー側 1 か所であることを不変条件として明記する
  - 完了状態: 新しい兄弟テストで「右ダブルクリックで SHIORI へ何も届かず預かりが残る」「相方側の窓の Ctrl＋左ダブルクリックの終了指示がスコープ 1 になる」が緑、既存の入力テストが追随して緑
  - _Requirements: 1.10, 5.1, 5.3, 5.5, 11.3, 9.6_
  - _Depends: 1.4_
  - _Boundary: areka input_events_

- [x] 7. 表示と引き金
- [x] 7.1 (P) 計画を OS のメニューに写して表示する
  - 計画の各要素をポップアップメニューへ順に足す（項目は識別子付き・無効は灰色・チェックは印・サブメニューは子のメニュー・区切りは区切り）。文字列は UTF-16 へ写す
  - 表示は「前面化 → 戻り値で識別子を受け取る形での追跡表示 → 空メッセージの投函」という既知の作法に従い、失敗と未選択を戻り値で区別できるようにする（直前に最終エラーを 0 にしてから判定）
  - メニューの後始末は RAII で行い、親に渡した子メニューを二重に壊さない
  - 所有者はキャラクター窓の実ハンドル。客席座標から画面座標への変換もここに置く（失敗は呼び手が要求を捨てる）
  - 完了状態: OS を触る `unsafe` がこのファイルと説明書を開く関数の 2 か所に閉じ、表示失敗・未選択・選択の 3 つを呼び手が戻り値で区別できる（表示そのものの確認は実機へ）
  - _Requirements: 1.2, 1.3, 1.7, 2.5, 7.5_
  - _Depends: 3.2_
  - _Boundary: areka menu win32_

- [x] 7.2 (P) 表示可否と待ちの判定を純粋関数にする
  - 「抑止なら預かりがあるときだけ右ダブルクリックを送る・表示なら預かりを捨てる」判定を純粋関数にする
  - 「返事が届いた／期限内で未着／期限超過／切断」の 4 通りを、時刻を引数で受ける純粋関数にする
  - 完了状態: 兄弟テストで判定の 4 組と待ちの 4 通りが緑（要件 9.3 の右ダブルクリック側）
  - _Requirements: 1.10, 3.7, 11.3, 9.3_
  - _Depends: 5.1_
  - _Boundary: areka menu trigger（純粋部分）_

- [x] 7.3 解放を受けて照会を預け、毎 tick 覗いて計画まで作る
  - 解放ハンドラは右ボタンだけを見て、結線前・ドラッグ中なら預かりを捨てて記録し何もしない。返事待ち・表示中なら記録するだけで預かりには触れない（送るか捨てるかは決着した tick の判定に委ねる＝Implementation Notes 7.3）。それ以外は右クリック時点の写しを取り、照会を送って（送れなければ次の tick で全件既定名として扱う）要求を預け、表示中の印を立てる
  - 毎 tick の取り出しは、預かりが無ければ判定 1 つで戻る。決着したら写しと返り値から計画を作る。抑止なら預かりの規則どおりに処理して終える。待っている間に窓が消えていたら記録して捨てる
  - 表示中の印は解放で立て、計画ができた時点（または抑止・破棄で終わった時点）で戻す（表示タスクへ持ち越す形は 7.4 で差し込む）
  - 完了状態: 兄弟テストで「右解放で預かりが 1 件できる・結線前／ドラッグ中は預かりが残らず記録が出る・返事待ち中は預かりを残したまま記録が出る・返事が届いた tick で計画ができ、抑止の tick では計画ができない」が緑
  - _Requirements: 1.8, 1.9, 7.2, 7.4_
  - _Depends: 7.1, 7.2, 5.1, 6.1_

- [x] 7.4 表示のタスクで出して、戻ってから動作を 1 回だけ行う
  - 表示のタスクは World を借りずに始め、表示中も借りない（借りるのは tick の中だけ）。表示は 1 要求につき高々 1 枚。表示中の印は解放から動作の終わりまでを覆う形へ広げ、早期復帰を含む全経路で必ず戻る守りで管理する
  - 戻ったら外側 World と窓の生存を確かめてから、選ばれた識別子の動作を 1 回だけ呼ぶ（消えていたら記録して呼ばない）。表示中に残った預かりは、World を借りられた全終了経路（選択・未選択・表示失敗・窓が消えていた）で取り出して捨てる
  - 出した・選ばれた・未選択で閉じた・抑止・表示失敗・窓が消えた・無視した解放 の記録を steering の書式（接頭辞と構造化フィールド）で出す
  - 完了状態: 記録の捕捉テストで 7 種のログ行が該当の経路でそれぞれ 1 行出ること、および実機起動で右クリックからメニューが出て閉じ、選んだ項目の動作が 1 回だけ起きること（実機の網羅確認は 9.3）
  - _Requirements: 1.1, 1.4, 1.7, 6.5, 7.2, 7.3, 8.1, 8.2, 8.3, 8.5_
  - _Depends: 7.3_

- [x] 8. 結線と組込の 2 項目
- [x] 8.1 メニューを起動に結び、説明書と終了を登記する
  - 結線関数が ⑴ メニューの持ち物を World へ挿入し ⑵ 組込の 2 項目を登記し ⑶ 毎 tick の取り出しを配送の後に登録する。解放ハンドラの装着は結線関数では行わず、窓を生やした直後の同じクロージャ内（既存の押下ハンドラの装着の隣）で行う（起動時の窓は結線より後に生える＝Implementation Notes 8.1）
  - 説明書は既定名「説明書」・文言リソース・在否で有効／無効が決まり、選ばれたら説明書を開く。終了は既定名「終了」・文言リソース・常に有効で、選ばれたら既存の終了指示をメニューを出した窓のスコープで送る（メニュー固有の抜け道を作らない）
  - 本体の起動は入力の結線の直後に結線の 1 呼出、窓を生やすクロージャに装着の 1 呼出を足す。装着の口は窓を作り直す spec も呼べるよう公開し、装着した件数を記録する（0 件は警告）
  - 完了状態: 兄弟テストで組込 2 項目が写しに現れ（説明書のファイルが無ければ無効）、終了の動作が終了指示をちょうど 1 件送ることを受信側で数えて確かめられる
  - _Requirements: 2.2, 4.3, 5.1, 6.6, 9.6, 11.4_
  - _Depends: 7.4, 4.1, 4.3, 6.1, 3.1_

- [x] 8.2 結線済みの Ctrl＋左ダブルクリックの終了指示を取り除く（開発者裁定 2026-09-19）
  - 実機確認でメニューの「終了」が働くことを確かめたので、同じ役目の隠し操作を取り除く。押下ハンドラの「結線済み・Shift なしの Ctrl＋左ダブルクリック → 終了指示」の腕を消し、その操作は Ctrl の無い左ダブルクリックと同じに扱う
  - 強制退避（結線前の Ctrl＋左ダブルクリック、および Ctrl＋Shift＋左ダブルクリック）は一字も変えずに残す。利用者起因の終了指示を送る本番の呼び手がメニューの「終了」だけになることを確かめる
  - 終了指示の入口を留めていた既存テストと、6.1 で足したスコープのテストを、新しい規則（終了指示は 0 件・左ダブルクリックが 1 件届く・窓は閉じない）へ書き換える。強制退避のテストは無改変で緑のまま
  - 押下ハンドラ・`MouseWiring`・`send_close_request` の説明文のうち偽になるものを直し、9.2 の完了記録の非回帰 4 番に追記する
  - 完了状態: 兄弟テストで「結線済みの Ctrl＋左ダブルクリックは終了指示を送らず左ダブルクリックとして届く」「結線前と Ctrl＋Shift は強制退避のまま」が緑、areka の警告 0、ワークスペースのテストが緑
  - _Requirements: 5.1, 5.3, 5.5, 9.6_
  - _Depends: 8.1_
  - _Boundary: areka input_events_

- [x] 9. 検証と登記
- [x] 9.1 檻が判断を測っていることを摂動で示す
  - 枠の並びの 2 要素を入れ替えて構造テストが赤になること、抑止の腕を表示に変えて判定テストが赤になることを実際に走らせて確かめ、元に戻す
  - 完了状態: 赤の出力（テスト名と失敗内容）と復元の確認をタスクの完了記録に残す
  - _Requirements: 9.7_
  - _Depends: 3.2, 7.2_

- [x] 9.2 非回帰と番人を通す
  - 変えないと決めたもの（右クリック単発の非送出・バルーン窓の右クリック・拒まれた終了の経路・Ctrl と Ctrl＋Shift の入口・待機中の死活監視・トレイアイコン不在）が現状のまま動くことを、既存テストの緑と該当箇所の無改変で確かめる
  - 記録の無い失敗経路が無いことを、失敗経路を 1 件ずつ数え上げて確かめる: 表示失敗（1.7）・リソース問い合わせ失敗と上限超過（3.4）・既定アプリで開けない（4.4）・登記の置き換え（6.3）・表示後に窓が消えた（7.4）・結線の資源が不在。各経路に対応する記録が実在する定義箇所を完了記録に書く
  - 新設・改変したすべてのファイルが 1 ファイル 1,000 行以内であることを行数の番人で確かめる
  - 完了状態: ワークスペースのテストが緑（32bit 補助の成果物を用意した状態）、行数の番人が緑、非回帰 6 項目と失敗経路 6 件の根拠を完了記録に残す
  - _Requirements: 1.5, 1.6, 5.4, 7.1, 8.4, 9.8, 11.1, 11.2_
  - _Depends: 8.1_

- [x] 9.3 実機で OS に触れる部分を確かめる
  - 現行の argv 起動で実ゴースト 2 種を立ち上げ、⑴ 右クリックで出て外クリック／Esc で閉じる ⑵ 説明書が既定アプリで開く ⑶ 終了で終了挨拶が再生されて閉じる ⑷ 里々の項目名（アクセラレータの下線・開き直すと候補が変わりうる）が写る ⑸ 表示中にまばたき・文字送りが続き、表示中に SHIORI を落としても落ちない ⑹ 閉じた後に見えない窓がキャラクター窓の上に残らない を確認する
  - あわせて、Esc／メニュー外クリックで閉じたときに `[menu] TrackPopupMenuEx failed` の `error!` が出ないことを見る（表示中も tick が同じスレッドで回るので、その間に失敗した Win32 呼び出しが最終エラーを汚すと素の「未選択」が失敗と記録されうる＝Implementation Notes 7.1。出るなら設計の判定規則を見直す）
  - 完了状態: 6 項目それぞれの確認手順と結果（観察したログ行・見えた文言）をタスクの完了記録に残す
  - _Requirements: 9.9, 7.2, 7.5_
  - _Depends: 9.2_

- [x] 9.4 網羅台帳へ担当を登記する
  - 台帳 3 種の 15 項目に本仕様を担当として登記する（id は符号化済みなので見た目の名前で探さずカタログから写す）
  - 状態を実測に合わせる: 実装済み 6・語彙のみ 9（それぞれ理由と引受先を備考に書く・引数付きの説明書コマンドは縮退として備考へ）
  - 正典 URL のコメントが 15 項目の定義箇所に 1 行ずつ置かれていることを証拠収集の道具で確かめる
  - 同じコミットでロードマップの下書きに本仕様の行（担当数）を足し、手書きの数を道具で数え直して書く（引き算をしない）
  - 報告を作り直し、網羅調査のテストが緑になることを確かめる
  - 完了状態: `cargo test -p ukadoc-survey` が緑で、台帳の担当欄・ロードマップの行・報告の三者が一致している
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 11.5_
  - _Depends: 9.2_

## Implementation Notes

- 1.1: 新しい worktree では `vendors/pasta` が未取得で cargo が `pasta_core` を読めない。`git submodule update --init --recursive` を先に走らせる。`cargo` が「invalid metadata」（os error 1455＝ページングファイル不足）で落ちたら `-j 4` で再実行する。
- 1.1: `released` を消すのは `dispatch_pointer_events` の末尾だけ（`clear_transient_pointer_state` は触らない）。dispatch は Input スケジュールへ無条件登録なので翌フレームへ残る経路は無い。下流からは `wintf::ecs::pointer::ButtonReleased` で届く（`ecs/mod.rs` の明示再輸出には無い）。
- 1.3: 正典 URL のコメント（`/// ukadoc:`）は定義箇所 1 か所だけに置く（`doc/ukadoc-coverage/README.md` §3）。転記・呼び出し側（`resolve.rs` など）に同じ URL を書くと `cargo run -p ukadoc-survey -- evidence` が同じ id に 2 ファイルを挙げる＝レビューで差し戻し。`MountModel` は全欄リテラル／全欄分解のテストが 2 か所ある（`model_tests.rs`・`validation_tests.rs`）。
- 1.4: `OnClose` の参照列を `events::on_close` から導かずリテラルで突き合わせているのは spine 一周（`spine_conformance_script.rs` の `expected_calls()`）だけ。kanade の握手テストと areka-ghost e2e は期待値を `events::on_close` から作るので、この列の変更には恒真。`input_events/mod.rs` と `main.rs` は機械的な `User { scope: 0 }` のまま（実スコープは 6.1）。`spine_conformance_lap_tests.rs` は 988 行＝余裕 12 行。
- 2.1: 配列要素の直上の正典 URL は `// ukadoc:`（`///` は rustc が `unused_doc_comment` を出す・`events.rs` の `ALLOWED_EVENT_IDS` と同じ形）。`resource_get` は `schedule::resources` で `pub` だが `lib.rs` の公開ファサードには未追加（クレート外のテストで要るときに足す）。
- 2.2: 照会の Status は設計の逐語 `snapshot_of(&state.phase)` でなく `state.snapshot()`（選択待ちの `choosing` を落とさない・design.md を追随済み）。照会の失敗は通常経路と違い `Unloading{Fault}` へ倒さず `Failed` を UI へ返すだけ（`round_trip` の `error!` 文言「終了系列（Fault）へ」は照会経路では事実と違う＝文言だけの既知の不正確さ）。
- 2.2: 偽 SHIORI の受信端を握ったまま返信しないテストは、判定が後退すると赤にならず無限に待つ（`non_queryable_phase_answers_no_content_without_touching_shiori`）。2.3 で偽 SHIORI を書くときは「受信を記録して返信端を捨てる」形にして assert の赤で終わらせる。
- 2.3: kanade は 1 メッセージを同期で完走させ、`Boot` 1 通で `Steady` まで進む。起動途中の段は受信箱から見えないので、「会話できない状態の照会」は `Idle`（`Boot` より前）で確かめた（design.md を訂正済み）。任意 GET id の応答は `Fixture::with_resource_response`（値の語彙は `MouseResponse` を流用・`Script(String::new())` は 200 の空文字で 204 ではない）。
- 3.1: `menu/mod.rs` 先頭の `#![allow(dead_code)]` は配下 4 module にも及ぶ。8.1 で結線したら必ず外し、直後に `cargo build -p areka` の警告 0 を確かめる。`main.rs` は 950 行（`mod menu; mod readme;` は宣言済み＝以降のタスクは宣言行に触らない）。ログの捕捉は `log-capture-kit` の `capture_lines`（`tracing` マクロを捕まえる）。`Frame` を増やす日の長さ一致は `ORDER`・`slots` を縛り済み、`captions::FRAME_CAPTIONS` は 5.1 で縛る。
- 3.2: `CaptionMap`（`Default`＋`insert`＝空文字を捨てる＋`get`）は `captions.rs` に先置き済み。5.1 の `interpret` はこの `insert` で組む。`plan::build` は `Frame::ORDER` を参照せず入力の順を前提にする（`ORDER` の入れ替えで赤くなるのは `mod_registry_tests`、群分け・識別子・`&` の摂動で赤くなるのが `plan_tests`＝9.1 の摂動はこの対応で選ぶ）。群番号の付け替えは恒等（`build` は群の不等号しか見ない）なので摂動にならない。
- 4.1: `readme.rs` 先頭の `#![allow(dead_code)]` は 4.3 の結線で外す。テストは `ReadmeWiring` を直に `insert_non_send` する（`wire_readme` は分岐の無い配線）。`open`（`ShellExecuteW`）へ届くテストは 1 本も無い＝以降のテストでも実ファイルのある状態で `open_from_world`／`drain_readme_requests` を呼ばない（開発者の机でアプリが開く）。`&World` からの `get_non_send` の共有借用どうしは衝突しない。
- 4.2: `/// ukadoc:` の行は URL を `<…>` で囲まない（解決は完全一致＝`SourceUrlNotInCatalog` で `cargo test -p ukadoc-survey` が 9 本赤になる）。ソースに正典 URL を書いたタスクは必ず `cargo test -p ukadoc-survey -j 4` まで走らせる。`readme_cue.rs` 先頭の `#![allow(dead_code)]` は 4.3 で外す。送出後に `tick_wake::mark` は立てない（門は既定で無効・有効時も 30 コマの心拍で Input 段が回る・立てると `tick_gate_config_producers_tests` の名簿検査が赤）。
- 4.3: `wire_emo2_boot` には boot 成功まで届く決定論の道具が無い（既存テストは fallback 経路だけ・`spine.rs` は `wire_emo2_boot` を呼ばず自前の sinks 4 本で組む）。成功経路の結線は実機確認 9.3 ⑵ で見る。sinks の並びは `zorder_wiring_tests.rs` の `t_zwi05` が `mod.rs` の字面で固定している。`readme::is_available` だけ狭い `#[allow(dead_code)]` が残る（8.1 で外す）。`spine.rs` のコメント「production は 4 本」は陳腐化（実際は 6 本・本仕様の範囲外）。
- 5.1: `send_query` は送出失敗を自分では記録せず `Err(SendFailed)` を返す。呼び手（7.3）がそれを `interpret` へ渡して初めて `warn!` 1 行になる＝7.3 は `Err` を捨てずに必ず `interpret` まで運ぶ。空・値なしの `debug!` も失敗の `warn!` も 1 回の表示につき 1 行（id は構造化欄に列挙）。表示可否の空・値なしは正常系なので記録しない。`captions.rs` の 3 表は `resources.rs` と二重に証拠解決する（設計どおり）。
- 6.1: 7.3／8.1 が入るまで預かりを取り出す本番の呼び手が無く、右ダブルクリックは SHIORI へ届かない（途中状態として受容）。`take_pending_right_double_click`／`send_pending_right_double_click` の狭い `#[allow(dead_code)]` は 7.3 で外す。`MouseWiring` の doc「送出ヘルパ群はポインタハンドラ経由でのみ参照される」はメニューの「終了」が `send_close_request` を呼ぶ 8.1 で偽になるので、そのとき 1 行追随する。`char_scope` が `None`（`CharWindowMarker` 無し）の窓は左右とも旧来どおり何もしない。
- 7.1: RAII は `windows::core::Owned<HMENU>`（`Free` が `DestroyMenu` を呼ぶ）。子は `AppendMenuW(MF_POPUP)` が成功した後にだけ `mem::forget` する。`build(&[PlanEntry])` は私有で、読み戻し（`GetMenuItemCount` ほか）のテストは窓もモーダルループも要らない。**未決の危険**: `TPM_RETURNCMD` の 0 を「未選択／失敗」に分けるのは `GetLastError` だが、表示中は tick が同じ UI スレッドで回るので、その間の Win32 の失敗が最終エラーを汚すと素の未選択が `Err` になり `error!` が出うる（利用者から見える結果は未選択と同じ・記録の重さだけが違う）。owner 消失も同じ形。9.3 で観察し、出るなら規則を見直す。`show` の doc「`Err` は何も表示されていない」もそのとき合わせる。
- 7.2: `poll_step` は OS にも World にも触れないが、`try_recv` は返事を channel から**取り出す**（`Decided(Ok(..))` は 1 度きり・捨てると返事を失い、同じ tick に 2 回呼ぶと `Timeout` へ落ちる）。`InFlightGuard` は `Clone` しない。同じ旗への二重 `engage` は呼び手の誤り（解放ハンドラが旗で門番する）。`QueryFailure` は `PartialEq` を持たないのでテストは `match`。
- 7.3: **設計を改めた**: 返事待ち中（表示 1 枚の旗が立っている間）の解放は、右ダブルクリックの預かりを**捨てずに残す**。右ダブルクリックは「押下 1→解放 1→押下 2（預かる）→解放 2」の順に届き、SHIORI の返事がダブルクリックの間隔より遅いと解放 2 が返事待ちに当たるので、そこで捨てると `visible=0` のゴーストへ届かない（要件 1.10）。返事が速い通常経路は「要求 1 が預かり無しで決着→押下 2 で預かる→解放 2 が要求 2 として受理→要求 2 の判定が送る」。受理した解放も預かりに触れない（どちらもテストで固定・design.md 追随済み）。
- 7.3: 7.4 への必須事項: 表示中の預かりはその後に判定の tick が来ないので、`show_task` の手順 C が **World を借りられた全終了経路**（選択・未選択・表示失敗・窓が消えていた）で取り出して捨てる。落とすと古い預かりが後の無関係な要求の抑止で送られる。`ReadyMenu.guard` は保持するだけのフィールドで、7.4 が表示タスクへ移す（`poll_menu_query` は今は `drop(poll_once(..))`）。
- 7.3: 8.1 への必須事項: `poll_menu_query` は `dispatch_pointer_events` の後に並べる。`menu/mod.rs` の module 全体の許可を外した時点で、`input_events` の `take_pending_right_double_click`／`send_pending_right_double_click` が本番から到達していることを `cargo build -p areka` の警告 0 で確かめる。差し替え口は `MenuWiring.to_screen`（本番は `win32::client_to_screen`）の 1 つだけ。ドラッグ状態のテストは wintf の `start_preparing`（窓ハンドルは null）で `Preparing` を作り、`Drop` で Idle へ戻す（当初は `DragState::JustEnded` を置いていたが、それは製品の休みの状態で「ドラッグ中」ではなかった＝完了記録 9.3 の欠陥）。
- 7.4: 表示の関数は `MenuWiring` の欄でなく内側の `run_show`／`display` の引数（`show_task` が `win32::show` を渡すだけ）。`[menu] shown` は OS の表示を呼ぶ**直前**に出す（`TrackPopupMenuEx` はメニューが閉じるまで戻らない）＝失敗時は shown の後に error が続く。World を借りられない 2 経路は `[menu] world unavailable after menu`（`reason="world dropped"／"world busy"`）の `debug!` で、預かりは捨てられない。`spawn_local` は実行器の窓へ起床を投函するだけで同期 poll しない・tick 中に UI スレッドでメッセージを汲む者もいないので、通常の流れで「world busy」は起きない（レビューで wintf と実行器の実ソースを確認）。完了状態の実機の半分（右クリックで出る・動作 1 回）は 8.1 の結線後に 9.3 で見る。
- 8.1: **設計を改めた**: `open_startup_window` は窓を同期では作らない。World を書き換えるクロージャを積むだけで、適用は `app.run()` の tick の中（`drain_task_pool_commands`）＝`menu::wire_menu` より後。当初の設計どおり `wire_menu` の中で装着すると本番では 0 枚に付き（レビューの探針で `count=0` を実測）、メニューが出ず、6.1 以降は右ダブルクリックも届かなくなる。装着は `main.rs` の窓を生やすクロージャ内、`attach_char_pointer_handlers` の隣の `menu::attach_release_handlers(world)` で行う（是正後の有界実走で「実 sink 結線で起動しました」→ `menu_release_handlers_attached count=2` →「本物のゴースト窓を開きました scopes=[0, 1]」を実装者・レビュアーの双方が確認）。窓を作り直す spec は両方の装着を掛け直す。
- 8.1: `main.rs` の装着の 1 行を消しても決定論テストは気付かない（隣の `attach_char_pointer_handlers` と同じ性質）。気付くのは ⑴ 警告 0 のビルド（消すと dead_code が 14 件出る）⑵ 有界実走の `count=2` の行 ⑶ 実機確認 9.3。0 枚装着の `warn!` が捕まえるのは呼び出しの**順序違い**であって行の削除ではない。有界実走は引数 2 つ（ゴーストの根とバルーンの絶対パス）が要り、`target\debug\shiori-host32-helper.exe` は cargo が x64 で上書きするので i686 版を毎回コピーする。`main.rs` は 958 行。残した狭い `#[allow(dead_code)]` は後続 spec の口 3 つ（`register`・`MenuRegistry::unregister`・`ItemBody::Submenu`）と `captions::UNQUERIED_POPUPMENU_RESOURCES`（証拠行を本番ソースに保つ表・読むのは兄弟テスト）。

## 完了記録

### 9.1 摂動（要件 9.7・2026-09-18・HEAD `19cce0ec`）

走らせたコマンドはいずれも `cargo test -p areka --bin areka -j 4 menu::`。摂動前と全復元後はどちらも exit 0・`ok. 97 passed; 0 failed`。摂動は 1 つずつ明示の編集で入れ、逆向きの編集で戻し、対象ファイルの sha256 が摂動前と一致することと `git status --porcelain` が空であることを毎回確かめた。実装者とレビュアーがそれぞれ独立に走らせ、P1 と P3 は失敗したテストの集合まで一致した。

| 摂動 | 編集 | 結果 | 赤になったテスト |
|---|---|---|---|
| P1 枠の並び | `menu/mod.rs` の `Frame::ORDER` 末尾 2 要素を入れ替え（`Readme`⇄`Close`） | exit 101・`86 passed; 11 failed` | `menu::registry_tests::frame_order_is_the_seven_frames_and_matches_slot_indices`（left `[.., Close, Readme]`／right `[.., Readme, Close]`）・`menu::registry_tests::snapshot_follows_frame_order_not_registration_order`・`menu::captions::captions_tests::frame_captions_table_is_the_seven_rows_of_requirement_3_1`・`menu::wiring_tests` 2 本・`menu::trigger::trigger_flow_tests` 5 本（例: 照会の並びが left `[visible, closebutton, readmebutton]`／right `[visible, readmebutton, closebutton]`）・`menu::trigger::trigger_wired_tests` 1 本 |
| P2 区切りの群分け | `menu/plan.rs` の `group_of` で `Frame::Close => 3` を `2` へ（説明書と同じ群へ入れる） | exit 101・`90 passed; 7 failed` | `menu::plan::plan_tests::readme_and_close_only_are_separated_by_one_line`（left に `Separator` が無い）・`menu::plan::plan_tests::all_seven_frames_follow_the_declared_order_with_three_separators`・`trigger_flow_tests` 4 本・`trigger_wired_tests` 1 本 |
| P3 抑止の判定 | `menu/trigger.rs` の `decide` で `Visibility::Suppress` の腕を `Decision::Show` へ | exit 101・`92 passed; 5 failed` | `menu::trigger::trigger_tests::decide_sends_the_double_click_only_when_suppressed_and_deferred`（left `Show`／right `Suppress { send_double_click: true }`）・`trigger_flow_tests` の抑止 4 本（`a_suppressed_tick_*` 2 本・`a_late_suppressing_reply_*`・`a_fast_suppressing_reply_*`） |
| （レビュアー追加）ドラッグの関門 | `menu/trigger.rs` の `handle_release` のドラッグ判定を無効化 | exit 101・`96 passed; 1 failed` | `a_release_while_dragging_is_ignored_and_drops_the_deferred_double_click` |

P1 で `menu::plan::plan_tests` が緑のままなのは欠陥ではない。`plan::build` は既に並んだ入力を受け取り `Frame::ORDER` を読まないので、並びを見張るのは登記側とそこから通しで流れるテストで、計画側の構造（区切りの入り方）は P2 が見張っている。`captions::row` の `debug_assert_eq!` は列挙の宣言順と表の行順の食い違いを見張るもので、`ORDER` 定数の入れ替えでは発火しない（入れ替えを捕まえるのは `captions_tests` の並びの検査）。

### 9.2 非回帰と番人（要件 1.5・1.6・5.4・7.1・8.4・9.8・11.1・11.2・2026-09-18・コードは `19cce0ec` と同一）

差分の基点は `695c40f2`（タスク生成の直後）。「差分なし」はすべて、先に `git ls-files` でパスが実在することを確かめてから `git diff 695c40f2..HEAD -- <path>` を取った。実装者とレビュアーが独立に数え直し、下の数はレビューの訂正を反映してある。

**非回帰 6 項目**

| # | 変えないと決めたもの | 無改変の根拠 | 振る舞いを留めているテスト |
|---|---|---|---|
| 1 | 右クリック単発は `OnMouseClick` を送らない（1.5／11.1） | `input_events/mod.rs` の `on_char_pointer_pressed` で、単発の腕（`DoubleClick::Middle \| XButton1 \| XButton2 \| None => return false`）は差分の文脈行のまま。解放の経路も送らない: `menu/*.rs`・`readme.rs` の本番コードに `KanadeMsg::Mouse`／`MouseInput` の組み立ては 0 件（作るのは `captions::send_query` の `ResourceQuery` だけ）。解放・覗きの経路が起こしうるマウス送出は `send_pending_right_double_click` の 1 本だけで、中身は右の**ダブル**クリック（Ref5＝1） | `input_events_tests.rs::handler_middle_xbutton_and_single_click_do_not_send`（無改変）・`trigger_flow_tests.rs::right_release_keeps_one_pending_query_and_sends_one_resource_query`・`a_suppressed_tick_without_a_deferred_double_click_sends_nothing` |
| 2 | バルーン窓の右クリック（1.6） | `input_events/balloon.rs` と `balloon_*` のテスト 7 本とも差分 0 行。解放ハンドラが付くのは `CharWindowMarker` の窓だけ | 既存のバルーンのテスト群（無改変で緑） |
| 3 | 拒まれた終了の経路（5.4） | `areka-kanade/src/schedule/close.rs` の差分は `mod tests` の中の 6 か所だけで、どれも `CloseReason::User` → `User { scope: 0 }` の綴りの追随（その行を除いた差分は 0 行） | `close.rs` の `value_then_ended_refuses_close_and_resumes_pump`・`value_then_interrupted_refuses_close_same_as_ended`、`close_test_handshake_tests.rs::close_refused_resumes_pump_then_terminates_via_resumed_talk` |
| 4 | Ctrl／Ctrl＋Shift の入口（5.5） | `handler_ctrl_shift_left_double_click_despawns_all_ghost_windows_without_sending` は差分 0。`handler_ctrl_left_double_click_sends_one_close_request_and_keeps_the_windows` は `CloseReason::User { scope: 0 }` への 1 行の追随だけで、判定はスコープまで留める形に**強まった**（弱めていない） | 左の 2 本＋新設 `input_events_menu_tests.rs::ctrl_left_double_click_carries_the_scope_of_the_clicked_window` |
| 5 | 待機中の SHIORI の死活監視（7.1） | `areka-kanade/src/shiori/` の 4 ファイルとも差分 0 行。監視は SHIORI アクター自身のスレッドで回り、UI は同期で待たない（`captions::send_query` は送るだけ・返事は毎 tick の `trigger::poll_step` が `try_recv` で覗くだけ・表示中の `trigger::display` は World を借りない） | `shiori/real_idle_tests.rs` の 4 本（無改変で緑） |
| 6 | トレイアイコンを持たない（11.2） | `grep -rn "Shell_NotifyIcon\|NOTIFYICONDATA" crates/` は 0 件（同じ形の対照 `ShellExecuteW` は `readme.rs` に 4 件当たるので、検索そのものは働いている） | — |

**記録の無い失敗経路: 0 件**（新設の本番 8 ファイル `menu/{mod,plan,captions,trigger,win32}.rs`・`readme.rs`・`emo2_boot/readme_cue.rs`・`areka-kanade/src/actor_resources.rs` の早期離脱・`Err`・`None` の腕を全数読み、その腕自身か、戻り値を必ず受ける呼び手のどちらかに記録があることを追った。レビュアーも 8 本を全文読んで 0 件）。

| 失敗経路 | 記録の定義箇所（関数と `event`） | 水準 | 留めているテスト |
|---|---|---|---|
| 表示失敗（1.7） | `menu::trigger::finish` の `Err` の腕・`menu_display_failed`（`[menu] TrackPopupMenuEx failed`・`error`／`hresult`） | `error!` | `trigger_show_tests.rs::a_failed_display_logs_one_error_and_runs_nothing`・`a_failed_display_logs_shown_then_the_error` |
| 照会の失敗・上限超過・切断・送出失敗（3.4） | `menu::captions::interpret`・`menu_resource_query_unanswered`（`reason=timeout／dropped／send_failed`）と `menu_resource_failed`（失敗した id を 1 行に列挙）。空・値なしは `menu_resource_empty` | `warn!` 1 行／`debug!` | `captions_tests.rs::timeout_defaults_every_id_with_one_warn_carrying_the_reason`・`dropped_and_send_failed_carry_their_own_reason`・`two_failed_ids_are_reported_in_a_single_warn_line` |
| 既定のアプリで開けない（4.4） | `readme::open` の戻り値 ≦ 32 の腕・`readme_open_failed`（`path`／`code`） | `error!` | 無し（`ShellExecuteW` の OS の経路＝実機確認 9.3 ⑵） |
| 登記の置き換え（6.3） | `menu::MenuRegistry::register`・`menu_registration_replaced` | `warn!` | `mod_registry_tests.rs::second_registration_replaces_and_warns_once` |
| 表示後に窓が消えた（7.4） | `menu::trigger::finish`・`menu_window_gone`（`[menu] window gone after menu`）／`menu_world_unavailable`（`reason="world dropped"／"world busy"`）、返事待ちの間は `trigger::poll_once`・`menu_window_gone_while_waiting` | `debug!` | `trigger_show_tests.rs::a_selection_on_a_window_without_a_handle_runs_nothing`・`a_selection_on_a_despawned_window_runs_nothing`・`a_dropped_outer_world_is_logged_and_runs_nothing`・`a_busy_outer_world_is_logged_and_runs_nothing`、`trigger_flow_tests.rs::a_window_that_disappeared_while_waiting_drops_the_request` |
| 結線の資源が不在（8 か所） | `trigger::handle_release`・`menu_release_ignored`（`reason="not wired"`）`trace!`／`menu::request_close`・`menu_close_no_mouse_wiring` `warn!`／`readme::open_from_world`・`readme_open_no_wiring` `warn!`／`readme::is_available`・`readme_available_no_wiring` `trace!`／`readme::take_pending`・`readme_drain_no_wiring` `trace!`／`trigger::poll_menu_query`・`menu_outer_world_ref_missing` `warn!`／`menu::register`・`menu_register_no_wiring` `warn!`／`menu::attach_release_handlers` の 0 枚・`menu_release_handlers_attached` `warn!` | 左記 | `mod_wiring_tests.rs`・`trigger_flow_tests.rs`・`readme_tests.rs`・`trigger_show_tests.rs` の該当テスト（`readme_available_no_wiring` だけは文言でなく「結線が無ければ無効」という判定を `is_available_is_false_without_wiring` が留める） |

意図して静かにしてある腕（失敗ではない・理由付き）: 右以外の解放と Tunnel 相（全ポインタ事象が通る）／返事待ちの無い tick（ほぼ全 tick）／未登記の枠（正常な結果）／`send_query` の送出失敗（記録は次の tick の `interpret` の 1 行へ寄せてある＝その場で出すと 2 行になる）／`poll_step` の各 `Err`（純粋関数・受け手が記録）／`finish` の「選択なし」の戻り（直前に 1 行出している）／`win32` の `?` と `client_to_screen` の `None`（呼び手が記録）／`readme::is_available` の 2 回目以降の不在（初回だけ記録）／表示可否の空・値なし（正常系）。

**行数（9.8）**: `cargo test -p log-capture-kit --test file_length_guard_test` は 6 passed。本仕様が変更・追加した `.rs` は 79 本（新規 22・変更 57）で、1,000 行超は 0 本。950 行以上は 4 本: `spine_conformance_lap_tests.rs` 988（本仕様の増分 0）・`spine.rs` 968（0）・`resolve.rs` 964（+1）・`main.rs` 958（+10）。新設の本番ファイルの最大は `menu/trigger.rs` 558 行、新設テストの最大は `trigger_flow_tests.rs` 848 行。

**全体の関門**: i686 の成果物を用意した状態で `cargo test --workspace -j 4` は exit 0。`Running`＋`Doc-tests` 103 行に対し `test result:` も 103 行（最後まで走った）で、7,811 passed／0 failed／40 ignored。`cargo fmt --all -- --check` は exit 0、`cargo build -p areka` の areka 由来の警告は 0、`cargo test -p ukadoc-survey` は緑。clippy は関門ではないが、本仕様が足した行に当たっていた `collapsible_if` 1 件（`wintf/src/ecs/pointer/buffers.rs` の解放の旗）は 9.2 のレビューを受けて `&& let` へ畳んだ（既存の同種 13 件は担当外なので触っていない）。

### 9.4 網羅台帳への登記（要件 10.1〜10.5・11.5・2026-09-18）

- **登記**: 担当欄が `areka-P0-popup-menu-minimal` の項目は `shiori.toml` 13・`sakura-script.toml` 1・`assets.toml` 1・`property.toml` 0 ＝ 15（`grep -c` で数え、同じ数え方が `areka-P0-property-catalog-lists` を 120 と数えて表の `owner_count = 120` と合うことで較正）。変更のあった台帳の項目はちょうど 15 件で、担当にしないもの（`quitbutton.caption`・`OnMouseClick`・`readme.charset`・`menu,hidden`／`char*.menu`・`OnClose`・束「メニュー」の残り）は 1 行も動いていない。上書きした既存の担当は 0 件（15 件とも空欄からの登記）。
- **状態**: 実装済み 6（`readmebutton.caption`・`closebutton.caption`・`sakura.popupmenu.visible`・`kero.popupmenu.visible`・`descript_ghost` の `readme,ファイル名`・`\![open,readme]`＝引数付きは「警告して何もしない」縮退として備考に記載）。語彙のみ 9（枠 ①〜⑤の caption 5 件＝引く仕組みは在るが枠が未登記なので誰も引かない・引受先は `areka-P0-baseware-root-layout`／`areka-P0-ghost-shell-balloon-switch`／`areka-P0-network-update`／`areka-P0-ghost-install`。`char*.popupmenu.visible` と `popupmenu.type` 3 件は引受先の spec が起票 0 本であることを備考に明記）。
- **証拠**: `cargo run -p ukadoc-survey -- evidence` で 15 件すべて解決（caption 7 と visible 2 は `areka-kanade/src/schedule/resources.rs` と `areka/src/menu/captions.rs` の 2 か所＝設計どおり、`popupmenu.type` 3 と `char*.popupmenu.visible` は `captions.rs` の問い合わせない表、`\![open,readme]` は `emo2_boot/consumer_ledger.rs`、`readme,ファイル名` は `areka-parsers/src/package/model.rs`）。`check` は食い違い 0 件。
- **三者の一致**: 台帳の数え直し 15 ＝ `roadmap-draft.md` の `[[spec]]` 行の `owner_count = 15` ＝ 作り直した報告（`report/summary.md` の実装済み 106→112＝＋6、語彙のみ 446→442。内訳は shiori ＋4・assets ＋1・sakura-script ＋1）。検査が実際に数を見ていることは、レビュアーが `owner_count` を 14 に変えて当該行を名指しした赤になることで確かめた（戻した後は一致）。
- **数え直した手書きの数**（引き算はしていない）: `[briefs].count` 27→28／束を持つ `[[spec]]` 行 13→14／宛先が 2 つ以上の束に散る spec 8→9（本仕様は メニュー 13・配布物の素性 1・作り付けの窓 1 に散る）／宛先 0 本の束 36→35／置き場と表の食い違い 3 本・2 本→8 本・8 本／候補 spec 名の案と既存の綴りの一致 2 行→3 行／`briefing.md` の `[[barrier]]` 4 数（`list_shiori_resource` 実装済み 1→5・語彙のみ 158→154、`descript_ghost` 実装済み 9→10・未対応 64→63）。段階表の「依存する既存 spec」の欄は 67 行を台帳と全数照合して食い違い 0（その過程で本仕様と無関係な 2 行＝「窓の配置と重なり」「バルーンの文字」の陳腐化を是正）。
- **検査**: 台帳だけを編集した直後は `cargo test -p ukadoc-survey` が 91 passed／22 failed（担当が `[[spec]]` に無い・`[[barrier]]` の数・報告の鮮度）。追随後は 601／0／6／113／5 で緑。
- **統合担当への申し送り（本仕様の範囲外）**: ⑴ `roadmap-draft.md`「先頭ウェーブ」の節は 2026-09-13 の写真で、今日数え直すと 324／84／31 は 324／62／55、「バルーンの文字」の状態の分布と「63 件のうち 45 件」（今日は 22 件）も動く。各束の進行中の件数が 84 の内訳そのものなので部分的に直すと算術が壊れる＝節全体の撮り直しが要る（該当行に注記 1 行を添えた）。⑵ `briefing.md` 5-3 の 416 件／1,333 件は日付付きの作業記録なので触っていない（今日は 433 件／1,316 件）。⑶ 段階 B「更新」の候補 spec 名の案 `areka-P0-network-update` が 09-18 に起票された実在の spec と同名（意図しない重なり・裁定が要る）。⑷ 波の欄は全行が旧編成（W13〜W17）の写しのまま。⑸ 9.3 の実機確認で OS 側（メニュー表示・既定アプリで開く）に欠陥が出たら、実装済み 6 件の状態を再判定する。

### 8.2 結線済みの Ctrl＋左ダブルクリックの終了指示の除去（開発者裁定 2026-09-19・要件 5.1／5.3／5.5 改訂）

- 経緯: 9.3 の実機確認で、相方側のメニューの「終了」から終了の挨拶が再生されて正規に閉じることを開発者が確かめ、「この実装により Ctrl＋ダブルクリックの終了経路は不要になる。除去せよ」と裁定した。取り除いたのは**結線済み・Shift なしの Ctrl＋左ダブルクリック → 終了指示**だけで、強制退避（結線前の Ctrl＋左ダブルクリック・Ctrl＋Shift＋左ダブルクリック）は一字も変えていない。メニューは結線後のキャラクター窓にしか出ず、応答しない SHIORI は終了の握手を拒むので、起動に失敗した・固まったゴーストから抜ける口はこれだけである（同じ実機確認で、絵を出せない里々の検体は検証用ダミー窓へ落ち、メニューを出せなかった）。
- 実装: `input_events::on_char_pointer_pressed` の Ctrl の腕は強制退避 1 本（条件 `ctrl_down && double_click == Left && (!wired || shift_down)`）だけになり、成り立たなければ Ctrl を無視して左ダブルクリックの経路へ落ちる。利用者起因の終了指示を送る本番の呼び手は `menu::request_close` の 1 つだけ（`send_close_request` をワークスペース全域で検索）。記録 `close_requested` はコード・steering・`doc/` のどこにも残っていない（0 件）。
- テスト: 書き換えた 2 本（`input_events_tests.rs::handler_wired_ctrl_left_double_click_sends_no_close_request_and_delivers_the_double_click`・`input_events_menu_tests.rs::wired_ctrl_left_double_click_sends_no_close_request_on_the_partner_window`）は、改変前のハンドラに対して「終了指示ではなく左ダブルクリックを期待」で赤（105 passed／2 failed）。較正は両方向: 内側の `if` だけを消す危険な形（Ctrl＋左で常に強制退避）は 3 本が赤（窓が全部消える）、強制退避を丸ごと消す形は強制退避のテストが赤。どちらも戻した後に sha256 が一致。強制退避のテスト 2 本（`handler_ctrl_shift_left_double_click_despawns_all_ghost_windows_without_sending`・`escape_works_without_mouse_wiring`）は差分 0。`only_escape_terminates_ghost_windows` は偽になった説明文と局所名だけを直し、表明は同じ。「終了指示に窓のスコープが載る」は `mod_wiring_tests.rs::the_close_action_sends_exactly_one_close_request_with_the_window_scope`（スコープ 1 と 0）と `trigger_wired_tests.rs` が留める。
- 関門: `cargo test --workspace -j 4` は exit 0・7,811 passed／0 failed／40 ignored（103＝103）、areka の警告 0、`cargo fmt`・行数の番人とも緑。
- **9.2 の非回帰 4 番への追記**: 9.2 の時点では「Ctrl／Ctrl＋Shift の入口（5.5）」を無改変で残したが、この裁定で要件 5.5 は「強制退避の入口を残す」へ改まった。強制退避のテスト 2 本が無改変で緑であることが、改訂後の 5.5 の非回帰の根拠である。
- 追随した文書: 要件 5.1／5.3／5.5、設計の該当 3 か所、steering `roadmap.md` の「終了は Ctrl＋左ダブルクリックで求める」2 か所（実機運転の定石は「メニューの『終了』で求める」へ）、`spine_close_wiring_tests.rs` と `input_events_tests.rs` の偽になった説明 2 行。

### 9.3 実機確認（2026-09-19・emo2＝実 pasta・4 回の走行・⑷ だけ上流待ちで引き渡し）

起動は引数 2 つ（ゴーストの根とバルーンの絶対パス）、`RUST_LOG=info,areka::menu=trace,areka::readme=debug`、i686 の helper を `target\debug\` へコピーしてから。起動直後に `[menu] release handlers attached … count=2` →「本物のゴースト窓を開きました scopes=[0, 1]」。2 回の走行とも `ERROR` は 0 行。

| # | 確認 | 結果 | 観察したログ |
|---|---|---|---|
| ⑴ | 右クリックで出る・外クリック／Esc で閉じる・2 度目も | 済 | `[menu] shown scope=0 items=2` → `[menu] dismissed scope=0` が本体側 2 回、`scope=1` で相方側 3 回 |
| ⑴′ | 閉じただけで失敗と記録されないか（7.1 のレビューが挙げた危険） | 済・起きなかった | 未選択 5 回とも `dismissed`。`[menu] TrackPopupMenuEx failed` は 0 件 |
| ⑵ | 「説明書」で readme.txt が既定のアプリで開く | 済（ログ上） | `[menu] selected scope=0 frame=Readme id=1` → `[readme] opened the readme with the default application … emo2\readme.txt`（2 回） |
| ⑶ | 「終了」で終了の挨拶が再生されて閉じる | 済（開発者が目視で確認） | `[menu] selected scope=1 frame=Close id=2` → `kanade: OnClose GET を発行し握手を開始 reason="user"` → `close talk を再生起動` →（4.5 秒後）`talk_done_quit` → `unload_clean` → `ghost_quit` → `ghost shutdown sequence completed` |
| ⑷ | 里々の `readmebutton.caption`（`(&R)` の下線・開き直すと変わる） | **未実施・上流待ち** | `R_POST_and_KOMAINU` は `surface 0 has no layers at all (extent 0x0)` で窓の配置に失敗し、検証用ダミー窓へ落ちた（キャラクター窓が無いのでメニューを出せない）。里々の標準テンプレートの `surfaceNNNN.png` の慣習が未実装（`areka-P0-shell-implicit-surface`・α A1-①）。emo2 の pasta は caption を定義しないので `resource answered empty or no content: using the default labels`（既定名「説明書」「終了」）になる＝これは想定どおり。項目名の差し替えそのものは決定論テスト（`captions_tests.rs`・`resource_query_test.rs` の 4 通り・`trigger_wired_tests.rs` の「取扱説明書(&R)」）が留めている |
| ⑸ | 表示中も動く・表示中に SHIORI を落としても落ちない | 済 | 前半: メニューが開いていた区間（合計 14.2 秒）の中に SERIKO・描画のログが 22 行ある＝表示中も tick が回っている。後半（4 回目の走行）: メニューを開いたまま 8 秒後に helper を外から止めた（タスクマネージャへフォーカスを移すとメニューが閉じるので、ログの `menu_shown` を見張る番人が自分の起こした helper を止めた）→ `helper_exited exit=Abnormal(-1)` → `shiori_down`（終了系列 Fault）→ `[menu] dismissed scope=0` → `ghost shutdown sequence completed`・プロセスは exit 0。窓が先に消えるので `menu_null_post_failed`（`debug!`・無効な窓ハンドル）が 1 行出る＝記録付きで無害 |
| ⑹ | 閉じた後に見えない窓が残らない | 済（開発者が操作して「特に問題なし」） | 4 回目の走行で、メニューを閉じた後の左ダブルクリック 2 回がどちらも `OnMouseDoubleClick` の台詞へ届いている |
| 1.10 | 右ダブルクリックはメニューが出る側では届かない | 済 | 3・4 回目の走行とも `origin="OnMouseDoubleClick"` の件数は左ダブルクリックの件数と同じ（2＝2）＝右からは 1 件も届いていない。素早い右 2 連打は「表示 → 2 打目の押下がメニューを閉じる → 2 打目の解放で再表示」（`shown` → `dismissed` → 0.1 秒後に `shown`）になり、2 打目の押下はメニューのモーダルループが食うので `double_click=Right` 自体がほぼ起きない |
| 8.2 | 結線済みの Ctrl＋左ダブルクリックでは閉じない | 済（開発者が操作して「特に問題なし」） | 4 回目の走行で `mouse_escape_close` は 0 件・左ダブルクリック 2 回はどちらも普通のダブルクリックの台詞になった |

**実機確認が見つけた欠陥 1 件（同日是正）**: 3 回目の走行で、左クリックを 1 度した後は右クリックでメニューが出なくなった（以後の解放がすべて `[menu] ignored release reason="dragging"`）。wintf のドラッグの状態は左ボタンを離すと `JustEnded` で休み、製品には待機へ戻す `reset_to_idle` の呼び手が無い。`trigger` は「`Idle` でなければドラッグ中」と読んでいた。1・2 回目の走行は右クリックしかしなかったので踏まなかった。決定論テストは「ドラッグ中」の代役に `JustEnded` を置いていた（捕捉の持ち主が要らない唯一の状態だった）ので、欠陥そのものを合格条件に固定していた。是正: `JustEnded` を待機と同じに扱う（wintf の透過制御と同じ読み方）。テストは製品と同じ関数（`start_preparing` → `end_dragging`）で「左クリックの後」を作る 1 本を足し、ドラッグ中のテストは `Preparing`（左ボタンを押したまま）へ置き直した。是正を外すと足した 1 本が赤になることを確かめた。4 回目の走行で、左ダブルクリックの 4 秒後に `[menu] shown` が出ることを確認。

**⑷ の引き渡し**: 引受先 `areka-P0-shell-implicit-surface`（brief のみ・α A1-①）の `brief.md` に 2026-09-19 の追記として「実機サインオフで里々の項目名を 1 度見ること」を書いた。
