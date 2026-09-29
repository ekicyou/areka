# Implementation Plan

> 設計（`design.md`）の部品名で境界を示す。コードは「何の定義か」で指し、行番号では指さない。テストは本番ファイルの兄弟ファイルへ置く（`structure.md` の規約・どのファイルも 1,000 行以下）。全体のテストは `tools/test-all.ps1` 1 本で回す（i686 の補助プロセスの成果物が要る）。整形（`cargo fmt --check`）もタスクごとに通す。後回しにする要件は 0。
> 呼び手がまだ無いモジュールは、先頭に `#![allow(dead_code)]` を置いてよい。呼び手を結ぶタスクで外す（どのタスクで外すかを Implementation Notes に残す）。

- [x] 1. 他のクレートへ足す口
- [x] 1.1 (P) 更新のエンジンの後片付けに、成功した確定の残りを示す印を足す
  - 作業場所の走行フォルダを後片付けで消せなかったとき、そのフォルダの直下に空の印のファイルを置く（印が書けなくても残りの挙げ方は今日どおり）
  - 棚の掃除は、`old/` に中身が残っていても印の在るフォルダを「戻せなかった走行」と見なさずに消しにかかり、消せなくても残骸に挙げない。印の無いフォルダと、戻せなかった走行（印を置かない）は今日どおり警告に出る
  - `crates/areka-update/` の中で閉じ、`run` の署名・進捗・結果・失敗の型は変えない
  - 兄弟テストで、消せないファイルを `old/` に残した印つきの走行の後の作業場所づくりが残骸 0 で通り、開いていたファイルを閉じた後の作業場所づくりで消えることと、印の無い `old/` 付きのフォルダが今日どおり残骸に挙がることが緑（既存の `folder_with_content_in_old_survives_repeated_creates_and_is_listed` も緑）
  - _Requirements: 5.9, 9.10, 10.1_
  - _Boundary: areka-update work_

- [x] 1.2 (P) kanade の許可表に更新のイベント 19 語とリソース 2 語を足す
  - 送る 19 語（`OnUpdateProcessExec`〜`OnUpdateResultEx`）を、各 1 行の正典の URL の行つきでイベントの許可表に足し、冒頭の表にも 19 行足す（23 → 42）。`OnUpdateCheck*` 4 語と `OnUpdateResultExplorer` は足さない
  - リソースの許可表に `homeurl`・`useorigin1` を正典の URL の行つきで足す（10 → 12）。`other_homeurl_override` は足さない
  - 数の判定を 42 と 12 へ書き換え、19 語が引けて `OnUpdateCheckComplete` が引けないことを同じテストで判定する
  - kanade のテストが緑で、運行表・殻・`KanadeMsg` の差分が 0
  - _Requirements: 2.15, 9.13, 10.11_
  - _Boundary: areka-kanade schedule events, schedule resources_

- [x] 1.3 (P) `descript.txt` の `homeurl` と `name` を読む口を目録に足す
  - 渡されたフォルダの `descript.txt` から `homeurl` と `name` を読む公開の口を 2 つ足す（前後の空白を落とし、空は無し。無い・読めないは無しで、読めないときは `warn!`）。読み方は既存の単独の鍵の読み手と同じ（鍵は小文字化・値は無変形）
  - `homeurl` の口の上に、ゴースト・シェル・バルーンの正典の URL の行を 3 本置く。目録の素性の型には欄を足さない
  - 在る・無い・空白入り・空の値・読めないファイル・Shift_JIS の兄弟テストが緑
  - _Requirements: 1.3, 1.9, 2.4_
  - _Boundary: areka-ghost catalog_

- [x] 2. 本体から更新のエンジンを辿れるようにし、依頼の型の骨組みと台帳の 4 行を同じコミットで入れる
  - 本体 `areka` の依存に `areka-update` を足す（外部クレートと根の `Cargo.toml` は不変）
  - 本体に `update` のモジュールの骨組みを置いて宣言し、依頼の型（対象の種別と Reference の綴り・理由・総括の形・解いた対象・依頼・対象を解く前の要求）を置く。登録の口は 6.2、受付の口は 6.3 で足す。子のモジュールの宣言は、その子を作るタスクがそれぞれ 1 行ずつ足す
  - 同じコミットで、網羅台帳 `assets.toml` の `descript_install` の「相対パス」（証拠はエンジンの `delete.rs` の既存の行）と、`descript_ghost`／`descript_shell`／`descript_balloon` の `homeurl`（証拠は 1.3 の 3 行）を実装済みへ動かし、証拠の実在を検査にかけ、生成物と `roadmap-draft.md` の `owner_count` を生成器で作り直す（手で数を直さない）
  - 本体がビルドでき、種別と理由の綴り（`ghost`／`shell`／`balloon`・`manual`／`script`）の兄弟テストと `ukadoc-survey` のテストが緑
  - _Requirements: 8.2, 8.6_
  - _Depends: 1.3_

- [x] 3. 純粋な部品
- [x] 3.1 (P) イベント名・Reference・番号・失敗理由・総括の写しを作る
  - 19 語のイベント名を、各 1 行の正典の URL の行つきの定数で持つ。ゴーストは `OnUpdate*`、シェル・バルーンは `OnUpdateOther*` の組を種別から選ぶ
  - `useorigin1` の読み（`1` なら 1 始まり・それ以外と返事なしは 0 始まり）、全イベントに同じ値で載せる種別と理由、`OnUpdateProcessExec`・`OnUpdateBegin`・差分の一覧・各ファイルの取得と照合（照合は 1 回の通知から始まりと結果の 2 件）・締め（`none`／`changed` と入れ替えた一覧）・失敗・二重起動（`executing`）の Reference を組む
  - 失敗理由の表は、エンジンの失敗 11 種と取得の失敗 8 種を包む網羅の分岐で、ワイルドカードの腕を置かない。正典に語の無い輸送の失敗は `dns`・`connect`・`tls`・`toolarge`・`http`
  - 総括は `OnUpdateResult`（`種別\x01OK\x01件数` か `種別\x01NG\x01理由(\x01ファイル名)`）と `OnUpdateResultEx`（先頭に名前）。区切りはインストールの判断の区切りの定数を借りる。どの関数も fs・記録・スレッドに触れない
  - 兄弟テストで、進捗 6 種 × ゴースト／シェルの名の件数・名前・Reference、`useorigin1` の 3 通り、失敗の 11 種と取得の 8 種を 1 つずつ組んだ語と判定した種類の数が全種類の数と等しいこと、送らない 7 語が出ないこと、成功と失敗の混ざった 3 対象の総括の 2 形が緑
  - _Requirements: 2.2, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 2.11, 2.13, 2.15, 3.2, 3.3, 4.2, 4.3, 9.1, 9.2, 9.3, 9.8, 10.10, 10.12, 10.14_
  - _Boundary: update refs_
  - _Depends: 2_

- [x] 3.2 (P) 台本 `\![updatebymyself]`・`\![update,…]`・`\![updateother,…]` の受け口と引数の解析を作る
  - 3 つのコマンド名（選別子なし）だけを自分宛てとし、他は `debug!` で見送る。開けない荷物は自分宛てなら `warn!`
  - 解析は純粋な関数に置く: `updatebymyself` と `update,all` は今の 3 つ、`update,ghost+shell…` は並んだ順（`platform` を含む知らない語は断る）、`updateother` は `--shell=名前`／`--balloon=名前` を並んだ順に。更新オプション（`checkonly`・`testonly`・`recovery`・`--option=…`）が 1 つでもあれば要求ごと断り、`--plugin=` ほか知らない `--名前=` は 1 件ずつ `warn!` で読み飛ばし、`--shell=`／`--balloon=` が 1 つも無ければ断る
  - 断りは `warn!` 1 件で要求 0。通れば生の要求を送出端へ 1 件送る（送れなければ `warn!`・台本は殺さない）
  - 素の受信端を渡した兄弟テストで、4 つの入口の形が期待の要求になり、オプション付き・`platform`・`--plugin=` だけ・`--option=` 混じりが要求 0 と `warn!` 1 件、`--shell=S,--plugin=P` が `S` だけの要求になることが緑
  - `emo2_boot` のモジュールの宣言に 1 行足す（受け口の列への登記と消費者台帳は 7.1）
  - _Requirements: 1.5, 1.6, 1.7, 1.8, 9.4, 10.8_
  - _Boundary: emo2_boot update_cue（`emo2_boot/mod.rs` は宣言の 1 行だけ）_
  - _Depends: 2_

- [x] 4. `\![execute,install,url,URL,nar]`
- [x] 4.1 (P) URL から一時フォルダへ落とす部品と、短命の取得スレッドを作る
  - areka 専用の一時フォルダ（OS の一時フォルダの下の `areka\download\`）を作り、取得の前にそこの 7 日より古いファイルを消す（消せなければ `debug!`・消した数を記録）
  - 取得の境界（エンジンの `Fetch`）を受けて本文を `<pid>-<連番>-<URL の末尾の安全な名前>` に書く。名前が空なら `download.nar`。取得の失敗はファイル 0 で失敗を返す
  - 本体のテストで使い回す偽の取得口（URL ごとに本文か失敗を返す）を本体の兄弟の支えファイルに置く（エンジンの支えはテスト専用で本体から使えない）。6.1・6.6 もこれを使う
  - 短命のスレッド `install-fetch` で取得口を作って落とし（取得口の作り方を差し替えられる口を置き、本番は `WinHttpFetch::new`）、落とし終えたら出どころ「台本」の生のインストールの要求を送る。取得口を作れない・取得の失敗は `error!`（URL と理由）1 件で依頼 0。門は持たない（終了で待たない）
  - 偽の取得口を差した兄弟テストで、ファイル名の形と中身、`500` と時間切れで失敗かつファイル 0、`now` を 8 日進めると古いファイルが消え 7 日未満は残ること、スレッドの一周で依頼が 1 件届くこと・失敗で依頼 0 と `error!` 1 件が緑
  - _Requirements: 6.1, 6.4, 6.5, 6.6, 7.5, 9.11, 10.15_
  - _Boundary: install fetch_url_
  - _Depends: 2_

- [x] 4.2 台本の引数の検査を `path` と `url` の 2 つの腕へ広げ、受け口の `url` の腕から取得を起こす
  - 検査の結果を「パス」か「URL」の 2 つの腕にし、`url` は種別が `nar` か省略のときだけ通す。他の種別（`feed`・`homeurl`・`ical`・`ssf`・知らない語）と、空・`http://`／`https://` で始まらない URL を別々の断りで返す。`path` の腕の検査は今日どおり
  - 受け口は `path` の腕を今日どおりに、`url` の腕で 4.1 の取得を起こし（テストは 4.1 の差し替えの口で偽の取得口を差す）、2 つの断りをそれぞれ `warn!` 1 件で何もしない
  - 戻りの型の変更で赤になる既存のテスト 3 本（判断の 6 通り・選ぶ画面・受け口）を新しい型へ書き換える（消さない。`url` を `NotPath` と判定していた行は URL の腕の判定へ）
  - 判断の兄弟テスト（通る 2 通り・種別の断り 5 通り・URL の断り 2 通り・`path` は今日どおり）と、受け口の `url` の腕で依頼が 1 件届く・断りで依頼 0 と `warn!` 1 件のテストが緑
  - _Requirements: 6.1, 6.2, 6.3, 6.7, 9.11, 9.14, 10.3_
  - _Depends: 4.1_

- [x] 5. 更新の手続き（要求 1 件の一周と対象 1 つの一周）
- [x] 5.1 口を 5 つに限った手続きと偽の口を作り、対象 1 つの 4 経路と入口の分かれ道を固定する
  - 手続きの口（イベントを送って応えを待つ・リソースを 1 回照会する・エンジンを 1 周回す・読み直しを頼む・標準の手続きが始まったと知らせる）をどれも `&self` で置く。応えは「台本あり・返事なし・閉じた」、エンジンの結果は「済んだ・取得口を作れない・閉じた」
  - 要求 1 件は「メニューのときだけ `OnUpdateProcessExec` → 標準が始まった知らせ → 照会 1 回 → 対象ごとに一周 → 総括 → 読み直しの頼み」の順。`OnUpdateProcessExec` に台本が返れば以後 0 件、照会が無ければやめる
  - 対象 1 つは「更新先を解く（ゴーストは照会の値 → `descript.txt`、他は `descript.txt`・無ければ `warn!` で飛ばす）→ `OnUpdateBegin` → エンジン（観測の中で進捗をイベントへ写して送る）→ 締め 1 件」。取得口を作れなければ `error!` と `OnUpdateFailure(connect)`、失敗は `error!`（更新先・対象・段・種類の語・原因のファイル・戻ったか・作業場所）と表の語、成功で消せなかった物は `warn!`。告知を呼ばず、締めへの応えを見ない
  - 偽の口と `Progress` の台本を兄弟の支えファイルに置く（エンジンは呼ばない）
  - 兄弟テストで、4 経路（`none`・`changed`・取得の失敗・MD5 不一致）× ゴースト／シェルの名の列と Reference、締めが常に 1 件、`OnUpdateProcessExec` の 2 通りと台本の入口で送らないこと（標準が始まった知らせは照会より前に 1 回）、更新先の 3 通りと全部飛ばして総括 0、取得口を作れない場合、対象が複数なら 1 つずつ、が緑
  - _Requirements: 1.9, 1.10, 1.12, 1.13, 2.1, 2.12, 2.14, 3.1, 3.4, 3.5, 4.1, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 8.1, 9.1, 9.5, 9.6, 10.2, 10.5, 10.7_
  - _Depends: 3.1_

- [x] 5.2 読み直しの頼みの時機と、途中で閉じたときの捨て方を固定する
  - `changed` が 1 つでもあれば総括の応えを受けた後に読み直しを 1 回頼み、全部 `none` か失敗なら頼まない
  - 途中でイベントが「閉じた」を返したら、エンジンの一周は最後まで回し（観測では以後送らず `warn!` を 1 件ずつ）、その対象の締め・以後の対象・総括・読み直しを送らない。総括が「閉じた」でも読み直さない
  - 兄弟の `procedure_reload_tests` で、読み直しが総括の後に 1 回・`none`／失敗で 0 回・途中で閉じたら残りのイベント 0 件・総括 0・読み直し 0・観測が全部流れていること、が緑
  - _Requirements: 5.2, 5.4, 5.7, 7.4, 9.9, 10.13, 10.16_

- [ ] 6. 背景スレッドと UI 側の窓口
- [x] 6.1 背景スレッドと本物の口を作り、終了の門に出入りさせる
  - 最初の仕事で 1 度だけスレッド `update` を起こし、仕事（依頼と、依頼を受けた時点の kanade の送出端の写し）を 1 件ずつ受けて手続きを走らせ、終わったら窓口へ「終わった」を頼む。背景スレッドから窓口への頼み（始まった・読み直し・終わった）の型はこのタスクが定義する
  - 本物の口は kanade へ直接 GET で送って返事を待つ。返事の 5 値と切断を「台本あり・返事なし・閉じた」へ写す（定常でない・失敗・切断は閉じた、許可表に無いは返事なしで `error!`）。終了が始まっていれば送らない。照会は `homeurl` と `useorigin1` を 1 回で
  - エンジンの一周は門の始め → 取得口を作る → 書く段へ入る → `run` → 書く段を出る → 終わり。門が閉じていれば取得口も `run` も呼ばずに「閉じた」。取得口の作り方は差し替えられる口にし（本番は `WinHttpFetch::new`・テストは 4.1 の偽の取得口）、背景スレッドを起こす口から渡せるようにする。門の `label` は「更新先 → 対象」
  - 終了で待つ口の上限に達したときの本文を門に依らない語へ改め、既存の終了のテストの `.nar-work` の判定を新しい本文と `label` の判定へ書き換える（消さない）
  - 本番の取得口の型を綴るのはこのファイルと 4.1 の 2 つだけにし、本体の本番ファイルで `WinHttp` で始まる識別子がそれだけであることを字面の検査で判定する
  - 素の受信端で答える兄弟の `worker_tests` で、5 値と切断の写し、門が閉じていれば取得口にも `run` にも入らないこと、終了の後の送出が 0 件、字面の検査が緑。既存の終了のテストも緑
  - _Requirements: 1.11, 1.18, 2.3, 4.4, 5.1, 7.1, 7.3, 7.4, 8.1, 9.12, 9.14, 10.1_
  - _Depends: 4.1, 5.2_

- [x] 6.2 窓口を据えて系として登録し、対象を解く
  - 窓口を World に 1 つ据え（ゴーストを起こし直しても作り直さない）、毎 tick の取り出し（背景スレッドの頼み → 生の要求 → 照会の返事）を Input の段の投げ込みの捌きの後に登録し（各枝の中身は 6.3・6.4・6.5 が埋める）、門 `update` を片付けの関数と一緒に登記する（`register_systems` に 1 行）。片付けは後のタスクが持つ物（照会の返事待ち）を捨てる場所として置く
  - 今の対象を解く: ゴーストは置き場のゴーストのフォルダ（名前は SHIORI の名前 → `descript.txt` の `name` → フォルダ名）、シェルは起動時に解いたシェルのフォルダ、バルーンは起動時に解いたバルーンのフォルダ（名前は `descript.txt` の `name` → フォルダ名）。更新先の倒れ先は各 `descript.txt` の `homeurl`
  - `updateother` はゴーストのシェルの目録と根のバルーンの目録の `name` に完全一致（大文字小文字を区別）で引き、引けない名前は `warn!` で飛ばす（隠しシェルは目録に無いので引けない）。無いフォルダも `warn!` で飛ばす
  - 一時の根に置いた emo2 風のフォルダの兄弟の `desk_resolve_tests` で、今の 3 つの `dir`・名前・倒れ先、名前引きの完全一致・大文字小文字違いと隠しシェルが引けないことが緑
  - _Requirements: 1.4, 1.7, 7.1, 10.12, 10.18_

- [ ] 6.3 受付と窓口の段（走っていない・答え待ち・走っている）と預かり 1 枠を作る
  - 受付の口を 1 つ置き、窓口が無い・終了が始まった・走っている（答え待ちで預かりが埋まっているときも）・切替の予約が在る・送出端や起動の文脈が無い・解いた対象が 0 の順に断って判定ごとに `warn!` を 1 件残す。走っているときだけ UI スレッドから `OnUpdateFailure(executing)` を 1 件直接送る（返事なし）。受けたら 6.2 で対象を解き、送出端の写しと一緒に背景スレッドへ渡す（`info!`）
  - 答え待ちの間に届いた要求は対象を解かずに 1 件だけ預かる（`info!`）。「始まった」を受けたら段を走っているにして預かりを `executing` で断り、「終わった」を受けたら段を戻して預かりを受付に掛け直す。受けたらメニューは答え待ち、台本は走っているの段へ。段の遷移は `debug!`
  - 台本の受け口へ配る生の要求の送出端を答える口を置く（窓口が無ければ受信端の無い送出端）
  - 偽の kanade の受信端を使う兄弟の `desk_tests` で、走っている間の要求が `executing` 1 件で総括 0、預かりの 3 通り（応えがあれば後で始まる・応えが無ければ `executing` 1 件・2 件目は断る）を頼みと要求が同じ tick でも別の tick でも同じ結果になること、切替の予約が在れば断ってイベント 0 と `warn!` 1 件、が緑
  - _Requirements: 1.1, 1.14, 1.15, 1.17, 8.1, 9.6, 9.7, 10.9_

- [ ] 6.4 `homeurl` の写しと「選べるか」を作り、定常到達で照会する
  - 定常到達のたびに写しを消して `homeurl` を 1 件照会し（`ghost_switch` の定常到達の腕に 1 行）、返事は毎 tick 覗いて空でなければ写しに置く。終了の片付けで照会の返事待ちを捨てる
  - 選べるのは窓口が在り、終了が始まっておらず、段が走っていないで、写しか 3 つの `descript.txt` のどれかに更新先が在るとき。メニューの動作は今の 3 つを理由 `manual` で受付へ
  - 兄弟の `desk_resolve_tests` で、選べる／選べないの 4 通り（3 つとも無い・写しに在る・`descript.txt` の 1 つに在る・走っている間）と、定常到達で照会が 1 件飛び返事が写しに載ること、終了の後に照会の返事待ちが捨てられることが緑
  - _Requirements: 1.3, 1.4, 1.16, 7.4, 9.5, 10.7_

- [ ] 6.5 読み直しの頼みを捌く
  - 「読み直し」の頼みは、終了が始まっておらず、切替の予約が無く、置き場のゴーストのフォルダが頼みのフォルダと同じで、フォルダ名が在る（コマンドライン引数の起動でない）ときだけ、既存の切替の入口へ「同じフォルダ・知らせなし・出どころ＝自動」で 1 回頼み、判定を記録する（受けたら `info!`、他は `warn!`）。満たさなければ理由つきの `warn!` で頼まない。頼み直しはしない
  - 終了が始まった後に届いた読み直しの頼みは落とす
  - 切替の土台（偽の SHIORI）の兄弟の `desk_reload_tests` で、条件を満たせば切替が受け付けられて `OnGhostChanging` と `OnClose` が 0 件、引数の起動・別のゴースト・終了の後では切替の要求 0 件と `warn!` 1 件、読み直しの途中の切替の頼みが今日どおり無視されること、が緑
  - _Requirements: 5.2, 5.3, 5.5, 5.6, 5.8, 7.4, 9.9, 9.12, 10.6, 10.13, 10.17_

- [ ] 6.6 本番の道筋（窓口 → 背景スレッド → kanade → 偽の SHIORI）で、一周のイベントと読み直しを固定する
  - 切替の土台の上で本物の窓口・kanade・本物の口を通し、4.1 の偽の取得口を差した一周で `OnUpdateBegin` → `OnUpdateReady` → 各ファイル → `OnUpdateComplete` → `OnUpdateResult` が偽の SHIORI に届く順を判定する
  - `changed` なら総括の返事の台詞が終わってから同じゴーストが `OnGhostChanging` 無しに起き直り `OnGhostChanged`（自分→自分）が届く、`none` なら切替の要求が 0 件であることを判定する。待ちは返信端と受信端の受け取りで揃え、実時間に依らない
  - 兄弟の新しいファイル `worker_path_tests` のこの 2 本が緑
  - _Requirements: 2.3, 5.1, 5.2, 5.3, 5.5, 9.9, 10.1_
  - _Depends: 6.1, 6.3, 6.5_

- [ ] 7. 2 つの入口を結ぶ
- [ ] 7.1 (P) 台本の受け口を消費者台帳と受け口の列に登記する
  - 消費者台帳に更新の担当（正典の URL の行 3 本）と `updatebymyself`・`update`・`updateother` の 3 行（選別子なし）を足し、行数の判定を 10 から 13 へ書き換える
  - 受け口の列に 10 本目として足し、窓口の生の要求の送出端を渡す（窓の無い起動では組まれない）。列の原文を判定しているテストも直す
  - 台本の文字列から受け口 → 取り出し → 受付まで通したテストと、断りの形で依頼 0 件・`warn!` 1 件のテストが緑
  - 窓口（`desk.rs`）は送出端を借りるだけで編集しない（7.2 と並走できる）
  - _Requirements: 1.1, 1.5, 1.6, 1.7, 1.8, 1.19, 9.4_
  - _Boundary: emo2_boot update_cue, emo2_boot mod, consumer_ledger_
  - _Depends: 3.2, 6.3_

- [ ] 7.2 (P) メニュー「ネットワーク更新」を登記する
  - 「ネットワーク更新」枠へ既定名と `updatebutton.caption` の項目を登記する関数を置き、`boot_wired` から呼ぶ（起こすたびにやり直す・窓の無い起動では登記されない）。選べるかは 6.4 の判定、動作は 6.4 のメニューの動作
  - 同じコミットで網羅台帳の `updatebutton.caption` を実装済みへ動かし（`owner` を本仕様へ）、生成物と `roadmap-draft.md` の `owner_count` を生成器で作り直す
  - メニューの兄弟テストで、起こし直した後も枠が登記されている・3 つとも更新先が無ければ灰色・走っている間は灰色・選ぶと理由 `manual` の要求が受付に届く、が緑。`ukadoc-survey` のテストも緑
  - _Requirements: 1.2, 1.3, 1.4, 1.16, 1.19, 8.3, 9.5_
  - _Boundary: menu update_frame, menu mod, ghost_session boot_wired, doc ukadoc-coverage shiori updatebutton.caption_
  - _Depends: 6.4_

- [ ] 8. 台帳・生成物・配布物の説明・互換の記述・申し送りを実物に揃える
  - `shiori.toml` の 19 イベント（証拠は写しの定数の上の行）と `homeurl`・`useorigin1`、`sakura-script.toml` の `\![updatebymyself…]`・`\![update,更新対象…]`・`\![updateother,…]`・`\![execute,install,url,…]` を実装済みにする。送らない行（`OnUpdateCheck*` 4 語・`OnUpdateResultExplorer`・`OnUpdatedataCreating`／`Created`・`other_homeurl_override`・`\![update,platform]`・`\![execute,createupdatedata]`）は状態を動かさず備考に理由
  - 生成物と `roadmap-draft.md` の数を生成器で作り直す（手で数を直さない）
  - `dist/README.txt` の 2 行から「ネットワーク更新」の語を外す（シェル・バルーンの切り替えの分は残す）
  - `doc/COMPAT_ARCHITECTURE.md` §8 に、正典が沈黙している点の扱いと、エンジンの前提「SHIORI を先に解放」を外して呼ぶこと（写像中の DLL の退避は印で区別）を記す。Monitoring の表に無い記録の語が実装で増えていれば design の表へ拾う
  - 後続 `shell-balloon-switch` と `alpha-release-signoff` の brief に、要件 8.8 の申し送り（今のシェル・バルーンの読み替えと許可表 42・リソース 12／第三者の手順「更新する」と既知の制限）を足す
  - `ukadoc-survey` のテストが緑で、台帳の検査が赤を出さない
  - _Requirements: 5.10, 8.3, 8.4, 8.5, 8.8_
  - _Depends: 4.2, 7.1, 7.2_

- [ ] 9. 全体の確認と実機サインオフ
- [ ] 9.1 全体のテストと規律の検査を通す
  - `tools/test-all.ps1` が緑。既存のテストは置き換え無しに消していない（数の判定は 23 → 42・10 → 12・10 → 13 へ書き換えただけ）。ネットへ出るテストは常時テストに無い
  - 本番とテストのどのファイルも 1,000 行以下。本番コードが読む環境変数・外部クレート・同期送信（`SendMessageW(`／`SendMessageTimeoutW(`）の例外表の追加が 0。メッセージボックスを出す呼び出しが本仕様のコードに 0
  - 印の判定（`session_mark_verdict`）の引数と理由の語、切替・メニューの終了・OS の終了で SHIORI を待つ期限、終了で待つ口の呼び手（同じ出発点の予算）が変わっていない
  - _Requirements: 1.18, 4.1, 5.11, 7.2, 7.6, 7.7, 8.6, 8.7, 9.14, 9.15_

- [ ] 9.2 実機で確かめて `signoff.md` に記録する
  - `RUST_LOG` をイベントの送出・更新先の解決・判断の分かれ目が見える所まで開ける（`info,areka=debug,areka::update=debug,areka_update=debug,kanade=trace`）。検体は根へ入れた emo2（引数なし・短い絶対パス）。配布サイトへ差分 1 件を置くのは開発者の手（置いてもらってから走らせる）
  - ⑴ メニュー「ネットワーク更新」→ 差分 1 件が入る → `OnUpdateBegin`・`OnUpdateReady`・`OnUpdateComplete`（`changed`）の台詞 → 台詞の後に引っ込んで戻る（`OnGhostChanged`）→ シェル・バルーンの中身も読み直されている ⑵ もう 1 度 → `none` → 読み直さない ⑶ `homeurl` の無いシェルが飛ばされ総括にゴーストとバルーンだけ ⑷ ⑴ の走行で `.update-work/` に印が置かれ、⑵ の走行で黙って消えるか（本番の 32bit SHIORI）。置けなければローカルの http で ⑴⑵ を行い、https 未確認を既知の制限へ
  - 4 項目が `signoff.md` に記録されている。予期しない結果は記録して開発者の判断を仰ぐ
  - _Requirements: 9.16, 9.17, 10.4_

## Implementation Notes

- 1.2: 送るイベントは正典で 19 語（spec の「20 語・43」は数え違いで dce16c5c で 19・42 へ直した）。後続の「19 語」「19 イベント」も同じ数。`events_tests.rs` の凍結の並び（`..._forty_two_...`）にも同じ 19 語が入っている
- 2: `update/mod.rs` 先頭の `#![allow(dead_code)]` は 6.3（受付の口）で外す。`owner_count`・`briefing.md` の数を書き出す生成器は無い（`ukadoc-survey` は report／report-summary だけ）＝`check` と整合テストの求める値を写す。`delete.rs` の `// ukadoc:` 行にアンカーを足した（境界外・コメントだけ）
- 3.1: `TargetEnd` は `refs.rs` に在る（`summary_refs` の入力）＝5.1 の手続きは `super::refs::TargetEnd` を使う。`begin_refs` は受けた `dir` を `display()` で書くだけ＝`OnUpdateBegin` の Ref1 を絶対パスにする（`std::path::absolute`）のは 5.1 の対象 1 つの一周の仕事
- 3.2: `emo2_boot/update_cue.rs` 先頭の `#![allow(dead_code)]` は 7.1（受け口の列への登記）で外す。`updateother` の `--` で始まらない裸の語（正典に無い）は `Refusal::Option` で要求ごと断る。`update,all+ghost` は断らず `[Ghost,Shell,Balloon,Ghost]`（重複はそのまま）
- 4.1: `install/fetch_url.rs` 先頭の `#![allow(dead_code)]` は 4.2 で外す。偽の取得口は `crate::install::fetch_url_test_support::FakeFetch`（`new().serve(url, bytes)`／`fail(url, err)`・表に無い URL は `NotFound`）。取得口の差し替えは `MakeFetch = Box<dyn FnOnce() -> Result<Box<dyn Fetch>, FetchError> + Send>`（スレッドの中で作る）。`std::env::temp_dir` は一時フォルダの見張り（`log-capture-kit/tests/temp_path_guard_test.rs`）の例外表に載せないと赤＝fetch_url.rs を `ProcessUnique` で登記済み。`capture` は呼んだスレッドだけを捕る＝スレッドの中の記録は中身の関数を同期で呼んで判定する
- 4.2: 台帳 `sakura-script.toml` の `\![execute,install,path,…]` の行の注記（「url を含む→ warn!（install_cue_unsupported）」「後続 areka-P0-network-update が足す予定」）が古い＝8 で `url` の腕と `install_cue_bad_url`／`install_cue_unsupported_kind` へ直す。受け口の差し替えは `InstallCueSink::with_fetch(tx, StartFetch)`（本番の `new` は `spawn_download`）
- 5.1: `error!(update_fetch_unavailable)` は手続き（`run_target`）の 1 件だけ＝6.1 の `run_engine` は `WinHttpFetch::new()` の `Err` を記録せず `gate.end()` の上で `EngineRun::Unavailable(e)` を返す（design の worker の節の 1 文は 8 で直す）。記録の語 `update_absolute_failed`・`update_order_begin` を 8 で Monitoring の表へ。偽の口の支え `procedure_test_support.rs` は `procedure.rs` の子（`#[path]`）＝5.2 の `procedure_reload_tests` も `procedure.rs` に宣言する。`OnUpdateProcessExec` が「閉じた」を返す枝（`update_abandoned at=process_exec`）は 5.2 で固定する
- 5.2: 手続きが固定するのは「総括の応えの後に読み直しを頼む」まで。要件 5.3（総括への返事の台詞が終わってから読み直す）は窓口と kanade の側＝6.5・6.6 が判定する。`OnUpdateBegin` で閉じたらエンジンは回さない（5.7 の「走っている更新」は走り出した後）
- 6.1: 取得口の作り方は `NewFetch = Arc<dyn Fn() -> Result<Box<dyn Fetch>, FetchError> + Send + Sync>`（対象ごとに呼ぶ）＝`spawn_worker(desk, gate, new_fetch)` の第 3 引数。本番は `worker::winhttp_fetch()`（6.3 で渡す）・テストは偽物。窓口への頼みは `DeskAsk::{Started, Reload, OrderDone}`。記録の語 `update_gate_closed`・`update_desk_gone`・`update_order_done` を 8 で Monitoring の表へ
- 6.2: 窓口 `UpdateDesk` は `update::register`（`register_systems` から 1 回）で据える。`drain` の枝（`answer`・`take_raw`・`peek_homeurl`）は仮＝6.3・6.4・6.5 が埋める。対象の解決は `here(world)`（World から写すだけ）と `resolve(&Here, raw)`（フォルダを読む）に分けた。記録の語 `update_target_resolved`・`update_query_discarded`・`update_desk_ask`・`update_desk_raw` と `update_target_skipped` の `reason`（`no_homeurl`・`no_folder`・`no_shell`・`name_not_found`）を 8 で Monitoring の表へ
