# Implementation Plan

> 設計は `design.md`、調べの記録は `research.md`（SSP 2.9.18 の実測は §8・設計の討議の決定は §13）。テストはすべて x64 の決定論テストで、実装の隣の兄弟ファイルに置く。実機の根・検体・ログはワークツリーの `target\` の下だけ。`window_move.rs`（例外表）は doc の 1 行の書き替えだけで行数を増やさず、`main.rs`・`drag_follow.rs`・`anchor.rs`・`resolver.rs`・`persist.rs`・`chain_finalize.rs`・`zorder_cue.rs`・`install_cue.rs` には、どのタスクも触らない。

- [ ] 1. `\![move]` の読みと位置の計算を、純粋な核として作る
- [x] 1.1 `\![move]` の引数を、名前付きの形と旧い位置の形の両方から同じ指令へ読む
  - 引数のどれかが `--` で始まれば名前付きの形、それ以外は旧い位置の形と判定する（`-353` は旧い形）。名前付きの形はキーを小文字にそろえ、順を問わず、空の値は省略と同じに扱う
  - 両方の形に同じ既定（X・Y は今の座標・time 0・基準 `screen`・両方の参照点 `left.top`）と同じ欄の読み（`fix` と負の数・点の無い 1 語は両軸・`--option` の繰り返し・`ignore-sticky-window` の印）を当てる。旧い形の欄の並びと 7 欄目以降を読まないことは今のまま
  - 知らないキー・`=` の無い引数・`--` で始まらない引数・知らない `--option` の値・`option` 以外の重複（後勝ち）・読めない X・Y（`fix` へ）・読めない time（0 へ）・知らない参照点の語（横 `left`・縦 `top` へ）・他のゴーストの名前の基準（スコープ 0 へ）・time>0 を、指令と一緒に返す置き換えの記録として 1 件ずつ残す。panic しない
  - 指令に `ignore_sticky_window` の欄を足し、読みの本体を `move_cue.rs` から新しい兄弟のファイルへ移す。`move_cue.rs` は子のモジュールを宣言して公開面を再輸出し、旧い縮退の型と分類（`MoveDegradation` の系）を外す。「名前付きの形は Err」を前提にした今の檻は置き換えの檻として書き改め、読みの檻は新しい兄弟のテストへ移す
  - 読みの戻りが変わって壊れる今の呼び手をこのタスクで合わせ、ビルドとテストを緑に保つ: 受け口（`MoveCueSink`）は読みの結果の指令を送り、置き換えの記録を `move_arg_substituted` の `warn!` で 1 件ずつ出す。`move_cue_move_sink_tests.rs` の「名前付きの形は送らない」の檻は「名前付きの形も送る」へ書き改め、`frame_drain_text_tests.rs`・`move_cue_apply_move_tests.rs` の見本の組み立ては読みの結果の指令を使う形へ替える
  - 新しい兄弟のテストで、設計の Testing Strategy に挙げた読みの全分岐と、emo2 の 2 つの台本（新しい位置調整と旧い `-353,,,0,base,base`）の読みの記録が 0 件であることが緑になる
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11, 1.12, 1.13, 3.8, 4.4, 4.5, 4.6, 4.7, 5.1_

- [x] 1.2 基準の枠と参照点から、動かす窓の移動先を求める計算を作る
  - 基準の枠を「窓（矩形と基準位置）」「作業領域」「原点」の 3 形で受け、枠の上の点（`left`・`right`・`center`・`base`）と動かす窓の中の点を軸ごとに求め、`Px` の軸は「枠の上の点＋k 倍した X・Y−動かす窓の中の点」、`Fix` の軸は今の左端（上端）にする。幅・高さの半分は切り捨て、計算は飽和演算で panic しない
  - 作業領域の基準の参照点 `base` は軸ごとに `center` へ、原点の基準の参照点が `left.top` 以外なら原点そのものへ置き換え、置き換えを記録として返す。動かす窓の参照点には置き換えを当てない。置き換えた後に実際に使った基準の参照点も返す
  - 基準位置の解き手（`BaseposResolver` と既定の解き手）を中身を変えずに同じ兄弟のファイルへ移す。今の計算（`resolve_move_target_position`）とその檻はこの段では残し、3.1 で外す
  - 新しい兄弟のテストで、参照点 4×4 の全組（窓の枠・作業領域の枠）・k=1 と k=2・置き換えの記録・SSP の実測表（research.md §8.3）の各番号の位置の一致・emo2 の位置調整が k=1・2 で「エモの右下＝むらさきの左下」・旧い emo2 の着地が今の式と同じ値・奇数の幅と負の座標の丸め、が緑になる
  - _Requirements: 1.3, 1.5, 2.1, 2.2, 2.3, 2.4, 2.6, 3.4, 4.1, 4.2, 4.3, 6.1, 6.2_

- [ ] 2. すべてのキャラクターの窓を既定の位置へ並べ直せるようにする
- [x] 2.1 (P) 起動と同じ配置の規則を今の窓の大きさで解き、並べ直して記憶へ書く手続きを作る
  - 窓の組の既定の位置の材料（descript から組んだ配置の設定）を持つ Resource と、並べ直しの手続きを placement に新しいファイルとして作る。主モニタを選ぶ口を crate の中から使えるように開ける
  - 手続きは、主モニタの作業領域と DPI、各スコープの今のキャラクターの窓とバルーンの窓の大きさで配置の規則を 1 度解き、スコープごとに窓へ書いて（バルーンは今のずらしのまま付いて動く）既定の位置を記し、`reset_window_pos` の記録（scope・前後の位置）を残し、最後に位置の記憶へ 1 度書く。`Anchored`・バルーンのずらしは変えない
  - 材料・窓の組・主モニタが無ければ `reset_window_pos_skipped` の `warn!` で何もせず、大きさの揃わないスコープは `warn!` で外して残りで解く
  - `move_window_to` の doc の「唯一の消費者は `\![move]`」を「呼び手（`\![move]`・連鎖の確定・並べ直し）はどれも `MoveCue` を名乗る」へ 1 行で書き替える（行数を増やさない）
  - 新しい兄弟のテストで、2 スコープの World のずらした窓が配置の規則の位置へ戻り、既定の位置が記され（記憶から戻した窓も記される）、記憶の発行口へ両スコープの位置が届き、バルーンが付いて動き、`Anchored` とバルーンのずらしが変わらず、材料が無いときに `warn!` だけになることが緑になる
  - _Boundary: ResetPos_
  - _Requirements: 8.6, 8.8, 8.9, 9.2_

- [x] 2.2 並べ直しの材料を、起動の準備とシェルの切り替えで置く
  - 起動の準備の結果に配置の設定を載せ、ゴーストの窓の組を作った後に材料の Resource として置く（Z の順の生の値を取り出す所の隣）。シェルの切り替えの後始末では、新しいシェルの descript から組んだ設定で置き換える（すでに組んでいる設定の組み立てを 1 度にまとめる）
  - 起動の準備の注記「配置の設定は捨てる」に、並べ直しだけが読む材料の例外を 1 行足す（コードは変えない）
  - `placement/reset_pos_tests.rs` に起動の準備の結果が設定を運ぶ檻を、シェルの切り替えの後始末の兄弟のテスト（`frame/switch_finish_tests.rs`）に材料が新しいシェルの設定へ替わる檻を足し、どちらも緑になる
  - _Requirements: 8.6_

- [ ] 3. `\![move]` の適用と、窓の位置の指令を運ぶ線をつなぐ
- [x] 3.1 `\![move]` の適用を、基準の解決・吸い付きの当て直し・記録つきで作り直す
  - 適用の本体を `move_cue.rs` から新しい兄弟のファイルへ移して書き直す。基準をスコープ（その窓・無ければ `screen` へ倒して記録）・`me`（動かす前の自分）・`screen`（自分の中心の作業領域・外れたら最近傍）・`primaryscreen`（主モニタの作業領域）・`global`（原点）に解き、1.2 の計算に渡し、置き換えの記録を `move_frame_substituted` の `warn!` で 1 件ずつ出す
  - 求めた位置を対象の窓の `Anchored` で吸い付きの射影に通してから `move_window_to` で書き、`move_applied` の `info!`（scope・基準・両方の参照点・吸い付き・基準の枠の矩形・前・台本・後の位置）を残す。`Anchored` が無ければ `move_anchor_missing` の `warn!` で当て直さずに書く。窓・位置・大きさ・作業領域が無ければ `move_skipped` の `warn!` で動かさない。`ignore_sticky_window` は読まない
  - 今の計算（`resolve_move_target_position`）を外し、`move_cue_tests.rs` のその檻は新しい計算の檻（`move_cue_geometry_tests.rs`）へ移すか重複として外す（旧い emo2 の着地の k=1・2 と `Fix` の軸の場合は残す）。`frame_drain_text_tests.rs` の doc の参照も直す。`move_cue_tests.rs` が空になるなら接続宣言ごと外す
  - 今の檻の「非スコープの基準は動かない」を「非スコープの基準でも動く」へ書き改め、旧い emo2 の着地の檻はそのまま残す
  - 新しい兄弟のテスト（偽の窓・主と副の偽のモニタ・モニタの写しを置いた World）で、基準 5 種と無いスコープの位置・下端に吸い付いた窓の Y が作業領域の下端−高さになり `free` は台本どおり・適用の後に大きさを変えても下端と横の中央を保つ・バルーンのずらしが保たれ記憶に何も送られず画面の外でも寄せず `Anchored` がビット同一・`move_applied` に基準と参照点の欄が出る・材料が無いときの `warn!`、が緑になる
  - 2.1 に依るのは、主モニタを選ぶ口を crate の中から使えるようにするのが 2.1 だから
  - _Depends: 1.2, 2.1_
  - _Requirements: 1.9, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 2.11, 2.12, 3.1, 3.2, 3.3, 3.5, 3.6, 3.7, 4.7, 9.1_

- [x] 3.2 窓の位置の指令の型を足し、受け口・消費者の台帳・move drain を一緒に替える（結合のタスク）
  - 線の型と台帳の一致の檻は一緒に替えないとビルドとテストが通らないので、1 つの結合のタスクにする。替えるファイル: `move_cue.rs`・`frame/drain_resnap.rs`・`frame/wiring.rs`・`frame.rs`（import だけ）・`emo2_boot/mod.rs`・`spine.rs`・`consumer_ledger.rs`・`consumer_ledger_agreement_tests.rs`・`move_cue_move_sink_tests.rs`・`move_cue_move_severity_log_tests.rs`・`frame_drain_text_tests.rs`・`frame_attach_tests.rs`・`frame_test_support.rs`・`frame_visibility_integration_tests.rs`・`balloon_visibility_lifecycle_e2e_tests.rs`・`spine_move_cue_tests.rs`・`input_events/balloon_test_support.rs`
  - 線を流れる型を「移動」と「並べ直し」の 2 種にし、線の受け手・取り出しの戻り・emo2 の起動と spine の型名を新しい型へ替える。線の型を綴るテストと支えのファイル（設計の Modified Files に挙げたもの）は機械的に置き換える
  - 受け口は `move` を「移動」として送り（読みの記録は 1.1 のまま）、`execute` で第 1 引数が `resetwindowpos` なら「並べ直し」を送り、残りの引数は読まない。開けない荷物の `warn!` は宛名が `move` のときだけにし、`execute` 宛は `debug!` にする。台本の進み（envelope の時間）には触れない
  - 受け口の担当と台帳の一致の檻が同時に変わるので、台帳への登記も同じタスクで行う: `(execute, resetwindowpos)` → 移動の受け口を 1 行登記し、件数（24 → 25）・doc の列挙・移動の受け口の doc・「install の受け口は install だけ」の檻の名と doc を合わせ、一致の檻の見本に `execute,resetwindowpos` を、`\![set,position,100,100,1]` と `\![reset,position]` がどの受け口にも届かない檻を足す（受け口の数 9 は変わらない）
  - move drain が「移動」はそのスコープのシェルに今かかっている拡大率（presenter の適用済みの値・無ければ 1・この 1 か所からだけ取りモニタの DPI から計算し直さない）で 3.1 の適用へ、「並べ直し」は 2.1 の手続きへ渡す（`OnResetWindowPos` は送らない）。1 件の失敗は次の件を止めない
  - 受け口のテストで、`move` → 移動・`execute,resetwindowpos` → 並べ直し・`execute,install` と `set,position` は送らない・読みの記録が `warn!` で出る・開けない `execute` の荷物が `warn!` にならない、が緑になり、台帳と一致の檻、型を替えた今のテストも緑のまま
  - _Requirements: 1.10, 2.4, 4.7, 5.2, 7.1, 8.7_

- [x] 3.3 move drain を、面の大きさが確定した後に走らせる
  - フレームの中で move drain の呼び出しを再スナップの後・連鎖の確定の前へ移し、理由を注記する。Z の順の drain の位置は変えない。相の順を前提にした今のテストがあれば書き改める
  - frame の兄弟に新しい相の順の檻を置き（今のハーネスの檻と同じ x64 の接続宣言）、同じフレームで表示の面の大きさの変化と `\![move]` が届くと新しい大きさで右下どうしがそろうこと、「並べ直し」が並べ直しの手続きに届くことが緑になる。面の指令と `\![move]` が別のフレームに割れて届く隙間は扱わないことを檻の注記に書く
  - _Requirements: 2.5, 2.12, 8.7_

- [ ] 4. 右クリックのメニューに「シェル位置初期化」を足す
- [x] 4.1 (P) kanade の許可表に `OnResetWindowPos` と `switchmovetodefaultpositionbutton.caption` を足す
  - イベントの許可表に `OnResetWindowPos`（ukadoc の URL のコメント付き）、リソースの許可表に `switchmovetodefaultpositionbutton.caption` を足し、件数の檻（53 → 54・12 → 13）と履歴の注記・doc の「メニュー 9 名」→ 10 名を合わせる
  - areka-kanade の許可表の檻が新しい件数で緑になる
  - _Boundary: areka-kanade schedule_
  - _Requirements: 8.2, 8.3_

- [x] 4.2 (P) `OnResetWindowPos` を送り、答えに応じて並べ直すかを決める葉の中身を作る
  - menu に新しい葉のファイルを作り、`menu/mod.rs` にはその `mod` の宣言だけを足す（枠の種類・登記・見張りの登記は 4.3）。`OnResetWindowPos` を答えの受け手つきで kanade へ送って受け手を保留の Resource（高々 1 つ）に置く動作と、毎 tick の答えの見張りを作る。送り口が無い・送れないときは `warn!` のうえ並べ直す
  - 答えの分類を純粋な判定にする: まだ → 待つ、台本 → 並べ直さない（`info!`）、返事なし → 並べ直す（`info!`）、`NotSteady`・`NotAllowed`・`Failed` → 並べ直す（`warn!`）、受け手が切れた → `warn!` だけで並べ直さない。どの答えでも保留を外し、`reset_pos_menu_outcome` の記録を残す。待つ時間に上限を設けない
  - 新しい兄弟のテストで、判定の 7 分類・送り口が無いときに並べ直すこと・答えの後に保留が外れること、が緑になる
  - _Boundary: ResetPosFrame_
  - _Depends: 2.1_
  - _Requirements: 8.3, 8.4, 8.5_

- [x] 4.3 「シェル位置初期化」の枠をメニューへ登記して、項目として出す
  - 枠の種類に「シェル位置初期化」を `Balloon` の直後に足し（並び 11）、名前の表（リソース名・既定の名前「シェル位置初期化」）と組 0 の割り当てに載せる。ゴーストの窓の組の枠の登記に葉の登記を、メニューの見張りに 4.2 の答えの見張りを 1 行ずつ足す
  - メニューの枠の並び・名前の表・計画（項目の数 10 → 11）のテストを書き改める
  - 葉の登記（項目を選べるのは窓の組が在り答えを待っていないとき）をこのタスクで作る
  - メニューのテストで、項目が既定の名前とゴーストの返す名前の両方で出て、選ぶと保留が置かれ、保留がある間は項目を選べないことが緑になる
  - _Depends: 2.2, 4.1, 4.2_
  - _Requirements: 8.1, 8.2, 8.3_

- [x] 5. 互換の対応表と網羅台帳を新しい振る舞いに合わせる
  - `doc/COMPAT_ARCHITECTURE.md` §8 の `\![move]` の 5 行を書き直し、要件 10.2 の決めた扱いを行として足す（X・Y の k 倍の行は保ち、位置の記憶の行に並べ直しの後の扱いを足す）。「emo2 は位置の形だけを使う」を根拠にした記述を消す
  - 台帳 `sakura-script.toml` の `move`（持ち主を本 spec へ・時間の部分が残り）・`execute,resetwindowpos`（実装済み）・`set,position`／`reset,position`（SSP 2.9.18 も実装していないので受けない）、`shiori.toml` の `OnResetWindowPos`・`switchmovetodefaultpositionbutton.caption`（実装済み）を改める
  - 同梱の検体（`vendors/sample_ghost/`）の辞書の `\![move]` の用法を点検し、あれば COMPAT と台帳の note に書く
  - `ukadoc-survey` の `report` と `report-summary` で生成物を作り直し、`ukadoc-survey` のテストが緑のまま、台帳と生成物が同じコミットに入る
  - _Requirements: 7.2, 10.1, 10.2, 10.3, 10.4_

- [ ] 6. 全体と実機で確かめる
- [ ] 6.1 触ったクレートのテストを通し、決定論の檻が要件の分岐をすべて覆うことを確かめる
  - `areka` と `areka-kanade` のテストを回し、赤が出たら赤のテストだけに絞って直す
  - 要件 9.3 の分岐（2 つの書き方・参照点・基準・吸い付き・置き換え・固定の指令を受けないこと・並べ直しの判断）それぞれに緑の檻が 1 つ以上あることを、テストの名前の一覧で示す
  - 1,000 行の決まりを超えたファイルが無いことを確かめる
  - _Requirements: 9.3_

- [ ] 6.2 emo2 の 2026-10-17 版を拡大率 100% と 200% で動かし、位置調整と並べ直しを実機で確かめる
  - `vendors/sample_ghost/emo2.nar` を `target\` の下の根へ絶対パスで展開し、`RUST_LOG` を `move_applied`・`move_arg_substituted`・`move_frame_substituted`・`reset_window_pos`・`reset_pos_menu_outcome` が出る水準まで開けて、ログファイルを `target\` の下へ書かせて起こす
  - その場で開発者の許可を得てから、SendInput で右クリックのメニューの「エモの位置調整」と「シェル位置初期化」を選ぶ（拡大率の切り替えは開発者の OS の設定）
  - `move_applied`（`scope=1`・`base=Scope(0)`）の `to_*` と基準の矩形の欄から「`\1` の右端＝`\0` の左端・`\1` の下端＝`\0` の下端」を 100% と 200% の両方で判定し、並べ直しの後は終了して起こし直して同じ位置に出ることを記録で確かめる
  - 手間の大きい壁に当たったら止めずに `/kiro-discovery` で申し送りの spec を起票し、結果・記録の抜粋・申し送り先を `.kiro/specs/areka-P0-move-canon/signoff.md` に残す
  - signoff.md に 100% と 200% の判定の行と並べ直しの確かめが載る
  - _Requirements: 6.1, 6.2, 9.4, 9.5_

## Implementation Notes
- 2.2: 並べ直しの材料 `DefaultPlacementBasis` は、起動では吹き出しのずらしの拡大と windowposition を当てた後の設定、シェルの切り替えでは descript から組んだままの設定を持つ。並べ直しが読むのは `char_pos` だけなので結果は同じだが、後で吹き出しの位置もこの材料から求めるなら、両者をそろえること。
- 3.1: Bottom の吸い付きで `MonitorSnapshot` が無いとき、`project_anchor` は台本の Y をそのまま返し debug だけを出す（ドラッグと同じ縮退・本番は起動で必ず置く）。軸が `Fix` でも作業領域・原点の置き換えの記録は `move_frame_substituted` に出る（要件 4.1/4.2 の「base と書かれた場合」に沿う）。
- 4.1: `crates/areka/src/menu/captions_tests.rs` の注記「引く 9 名」は 4.3 で枠を足すときに 10 名へ合わせる。
- 4.3: `ghost_session.rs` の注記「`Input` の 10 系」は本 spec より前から実際の系の数（12 以上）と合っていない。数を固定する檻は無い。
- 5: `set,position`／`reset,position` は `not-applicable`（持ち主は本 spec）にしたので、linkage の target は 1,550、`MIN_LINKAGE_CITED_IDS` も 1,550。`.kiro/specs/areka-P0-sakura-time-directives/brief.md:72` の持ち主の数「10（…move…）」は古いまま（同 brief の :116 が着手時の付け替えを予告）。
