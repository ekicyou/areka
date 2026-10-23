# Implementation Plan

- [x] 1. areka-parsers: 着せ替えの読みをスコープごとにし、ゴーストの名前だけを読む口を足す
- [x] 1.1 着せ替えの読みを兄弟の新しいファイルへ移す（振る舞いは変えない）
  - 着せ替えの読み（定数・既定の読み・bindoption の分解・番号と名前の読み）を `resolve.rs` から `resolve_bind.rs` へそのまま移し、`resolve.rs` の中の試験 2 つ（名前の転記・bindoption の分解）を `resolve_bind_tests.rs` へ中身を変えずに移す
  - シェルの読み（`resolve_shell.rs`）は移した先から着せ替えの読みを引く
  - 移しただけで、`resolve.rs` が約 100 行になり、`cargo test -p areka-parsers` が移す前と同じ件数で緑になる
  - _Boundary: ShellBindSetup（areka-parsers）_
  - _Requirements: 14.1_

- [x] 1.2 `char*` の既定と名前、名前を決めた行の番号を読む
  - 行の番号つきの読みを着せ替えの読みの中だけに足す（切り方は今の key-value の読みと同じ・同じキーは後の行の値と行の番号・今の読みは変えない）
  - `char{N}.bindgroup{M}.default`・`.name`・`char{N}.bindoption{M}.group`（N は 2 以上）を読み、スコープ 0・1・2 以上の 3 種類で今の照会（名前・カテゴリ・mustselect・multiple）と新しい「そのスコープの既定 ON の番号」が引けるようにする。名前ごとに決めた行の番号を持たせる
  - 試験: 値の前後の空白を取ること・同じキー 2 行で後の行の値と行の番号になること・CRLF で行の番号がずれないこと・`char2.*` の既定と名前が引けること・`char3` の定義が無ければ空
  - 触るのは `model.rs`・`resolve_bind.rs`・`resolve_bind_tests.rs` だけで、`package/mod.rs` の公開の行は足さない（`BindScope` は `model.rs` に既に公開で在り、足すのは変種と欄だけ）。1.3 と同じファイルを触らない
  - 上の分岐の試験が `resolve_bind_tests.rs` で緑になり、`sakura.*`・`kero.*` の今の試験は期待を変えずに緑のまま
  - _Requirements: 5.2, 5.9, 14.1_

- [x] 1.3 (P) ゴーストの descript.txt だけを読む口を切り出す
  - シェルの解決の中のゴーストの名前の読みを、単独で呼べる公開の関数に切り出し、シェルの解決はそれを使う（失敗の種類は今と同じ「無い」「読めない」）
  - 公開は `package/mod.rs` の今の行 `pub use resolve_shell::resolve_with_shell;` に `read_ghost_names` を足す 1 か所だけ（触るのは `resolve_shell.rs` とこの行）
  - 試験: 名前の在るゴースト・`name` の無いゴースト（空）・descript.txt が無い・フォルダが無い
  - 切り出した関数が単独で名前を返し、今のシェルの解決の試験が期待を変えずに緑
  - _Boundary: GhostNamesReader_
  - _Requirements: 2.6, 7.2_

- [x] 2. 着せ替えの型の付け替え（areka-seriko と画面）・スコープごとの名前の表・名前で当てる 1 つの決め方・時点 T まで進める口
- [x] 2.1 既定の着せ替えの型を「スコープごとの組」に付け替える（振る舞いは変えない）
  - スコープの番号ごとに組を持つ型（DefaultBinds）を足す: どのスコープにも同じ組を返す作り方・1 つのスコープだけに組を持つ作り方・スコープで引く口（書かれていない・数でないスコープは空の組）
  - スコープの状態の作り方・SERIKO の起こし方（時計つきを含む）・シェルの差し替えの合図の既定の組の型を新しい型にし、動的な組が無いときはスコープで引いた組へ戻る
  - areka-seriko の試験 27 本（`src/` の 22 本・`tests/` の 5 本、design の Modified Files の一覧と同じ。2026-10-22 に数え直して 41 本中の 27 本・約 100 か所）を「どのスコープにも同じ組」で包むだけにする（期待は変えない）
  - 新しい型の 3 つの作り方と、スコープで引く口の空の分岐の試験を足す
  - この段の後は 2.2 まで areka のクレートが建たない（確かめは `-p areka-seriko`）。2.1 と 2.2 は 1 人の実装係が 1 つの流れで続けて行い、そのあいだに `crates/areka` を触る (P) のタスクを走らせない
  - `cargo test -p areka-seriko` が期待を 1 つも変えずに緑（`cue_sequence.rs` の scope "1" → `binds:<static>` を含む）
  - _Requirements: 14.3_

- [x] 2.2 画面の着せ替えの型を付け替える（振る舞いは変えない・2.1 と続けて行う）
  - 既定の組を持ち回る areka の本番 7 本（`assets.rs`・`emo2_boot/mod.rs`・`spine.rs`・`shell_carry.rs`・`frame/attach.rs`・`frame/switch.rs`・`frame/wiring.rs`。design の本番 9 本のうち残りの `actor.rs`・`state.rs` は 2.1）を新しい型にし、組が要る所はスコープで引く。`assets.rs` はこの段では今の `sakura.*` の組を「どのスコープにも同じ組」で包む
  - areka の試験 14 本（`emo2_boot/` の 13 本・`input_events/balloon_test_support.rs`、design の Modified Files の一覧どおり・約 28 か所）を「どのスコープにも同じ組」で包むだけにする（期待は変えない）
  - 2.1 の後は areka のクレートが建たないので、2.1 と同じ実装係の同じ流れでこの段まで続けて行い、ワークスペースを建つ状態へ戻す（あいだに `crates/areka` を触る (P) のタスクを置かない）
  - `cargo build --workspace --all-targets` が通り、`cargo test -p areka` が期待を 1 つも変えずに緑
  - _Boundary: ShellBindSetup（areka emo2_boot）_
  - _Requirements: 14.3_

- [x] 2.3 スコープ 2 以上の名前の表と、シェルの読みからの既定と名前の表の作り方
  - 着せ替えの名前の空間に「3 人目以降（番号つき）」を足し、スコープ "0"→本体・"1"→相方・2 以上の数→3 人目以降・ほかは無しにする
  - 名前の表をシェルの着せ替えの読み（1.2）から 3 種類のスコープ分作る口と、カテゴリのパーツ名を descript.txt に書かれた順（同じ名前は 1 度・無いカテゴリは空）で返す口を足す。今のテスト用の作り方は残す
  - 既定の組をシェルの着せ替えの読みから作る口（スコープ 0 に `sakura.*`・1 に `kero.*`・2 以上に `char*.*`・書かれていないスコープは空）を足す
  - `scope_namespace_maps_zero_one_only` を 2 以上が 3 人目以降になる期待へ書き直し、3 種類のスコープ・書かれた順・`char3` 無しの空の試験を足して緑
  - スコープ 2 以上に名前の空間が無いことを前提にした areka の試験の期待も、この段で意図して直す（少なくとも `emo2_boot/shell_carry_tests.rs` の `decide_binds_drops_everything_for_a_scope_without_a_namespace`＝スコープ 2 の着せ替えの持ち越しを全部落とす期待。スコープ 2 が名前の表を引く形に書き直し、試験の名前と `shell_carry_tests.rs` の先頭の説明も改める）。同じ前提を書いた本番の説明（`emo2_boot/shell_carry.rs` の 126 行目あたりの doc）と、`areka-seriko/src/actor.rs` の 563 行目あたりの「scope 2 以降」の読み飛ばしの記録の文言もこの段で今の形に合わせる
  - `cargo test -p areka-seriko` と `cargo test -p areka` が緑
  - _Depends: 1.2_
  - _Requirements: 5.2, 5.9, 14.1, 14.2, 14.3_

- [x] 2.4 名前で 1 パーツを当てる決め方を 1 か所にまとめる
  - 名前の解決 → 方針 → 当てる（排他・multiple・mustselect の OFF は外さない）を 1 つの公開の関数にし、結果を「当てた」「名前の空間が無い」「見つからない」「mustselect で外さなかった」の 4 つで返す（記録は出さない）
  - 画面の `\![bind]` の処理をこの関数経由に組み替える（記録の文は今のまま）
  - 4 つの結果の試験を足し、今の `\![bind]` の試験が期待を変えずに緑
  - `cargo test -p areka-seriko` と `cargo test -p areka` が緑
  - _Requirements: 5.1, 5.3, 5.4, 14.2_

- [x] 2.5 面 1 つを時点 T まで画面と同じ道で進める口
  - その面の surface*ブレスの runonce の待ちの合計の最大を T とする関数（無ければ 0・部品の runonce は T を決めない）
  - 同じスレッドで、1 つのスコープだけに組を持つ状態・空の別名と空の名前の表・抽選が当たらない乱数・注入の時計を作り、「時刻 0 の刻み」「面の合図」「16ms ごとの刻み（T 未満・合図と同じ時刻 0 からの位相）」「時刻 T の刻み」を順に渡して、そのスコープの面の状態を返す。刻みの間隔は本番と同じ 16ms を公開の定数にする
  - 試験（research.md §10.1・§10.2 の表を写す）: runonce の最後のコマが残る・`-1` で土台に戻る・always だけは T=0 で出ない・`random,1` は出ない・periodic は出る（書き出しの位相 0 を固定する行として書く）・runonce＋always は T の時点の always のコマ・2 本の runonce は長い方・動く絵の子は T の時点のコマで繰り返す・2 度呼んで同じ状態
  - 上の表の試験が `settle_tests.rs` で緑で、関数がスレッドを起こさない（同じスレッドで返る）
  - _Requirements: 4.1, 4.7, 4.9, 12.2_

- [x] 3. areka-emo-compose・areka-emo-present: シェル全体の外形・形の誤りを 1 度だけ・面の表を 1 度だけ渡す
- [x] 3.1 (P) シェル全体の外形を返す読むだけの口
  - 今の面ごとの外形の辿り方（入れ子・循環の避け方）を写し、座標を符号つきで持って、すべての面の土台・element・element定義が置くサーフェス・すべての animation（`never`・`runonce` を含む）の描く pattern を土台の原点に重ねた外形を返す（負の番号・描かないメソッドは数えない・記録を出さない・(0,0) を必ず含む）
  - 今の面ごとの外形の関数は変えない
  - 試験: 負の座標の element・`never` の pattern・runonce の pattern・入れ子・はみ出しの無いシェルは土台のまま、research.md §10.6 と同じ形の検体で 18×14・12×14 になる
  - 上の試験が `plan_bounds_tests.rs` で緑で、どの面の合成の絵の大きさも外形の右下を超えない
  - _Boundary: ShellCanvasBounds_
  - _Requirements: 4.3_

- [x] 3.2 定義だけで決まる形の誤りを集める関数と、組むたびの知らせを debug に下げる
  - 入れ子の循環・知らない `animation-sort`・描かない method の pattern を面の表から集める純粋な関数を、合成の辿り方と同じ判定から引く形で足す（判定を写さない）
  - 面を組むたびの warn 6 か所（`plan.rs` 360・433・465・534 行目、`plan_extent.rs` 78 行目、`blit.rs` 90 行目＝2026-10-22 に行を確かめた）を debug に下げる。打ち切り・飛ばしの振る舞いは変えない。`blit.rs` 106 行目（アトラス頁が無い）は warn のまま
  - 試験: 循環・知らない sort・描かない method を持つ検体で、集める関数がそれぞれ 1 件ずつ返し、同じ面を何度組んでも合成の warn が増えない（`log-capture-kit` で数える）
  - 上の試験が `plan_issues_tests.rs` ほかで緑で、今の合成の試験が絵の期待を変えずに緑（記録の重さの期待だけを直す）。`cargo test -p areka-emo-compose` と `cargo test -p areka-emo-present` が緑
  - _Depends: 3.1_（`lib.rs` の公開の行を共有するので順に置く）
  - _Requirements: 7.5_

- [x] 3.3 シェルを読むときに形の誤りを 1 度だけ出し、組んだ面の表を 1 度だけ渡す
  - 読み込みで組んだ面の表を捨てずに持たせ、1 度目はそれに atlas を付けて渡し、2 度目からは今の組み方と同じにする口（take_world）を足す。面の表は Debug を持たないので、Debug を手で書き、面の表は有無だけを出す
  - 3.2 で集めた形の誤りを、今の読み込みの知らせと突き合わせて同じ誤りを 2 か所で warn にしないようにし、まだ出ていないもの（少なくとも知らない sort・描かない method の pattern）を面と animation の番号の欄つきで 1 件ずつ warn で出す
  - 画面の資産の組み立ては take_world を呼ばない（画面の絵と記録の数は変えない）
  - 試験: take_world の 1 度目と 2 度目が同じ面の集まりを返す・検体の読み込みで形の誤りの warn がそれぞれ 1 件
  - 上の試験が緑で、`cargo test -p areka-emo-present` と `cargo test -p areka` が緑（画面の記録の数の期待が変わる試験があればこの段で直す）、`shell_target.rs` が 1,000 行の内に収まる
  - _Requirements: 7.5_

- [x] 4. 比べる道具の新しいクレート `surface-compare`
- [x] 4.1 (P) 2 つのフォルダの同じ名前の PNG を画素で比べるライブラリ
  - クレートを立てる。`Cargo.toml` は `temp-path-kit` と同じ形（`workspace = "../.."`・`version`／`edition`／`authors`／`license`／`repository` は `.workspace = true`・行頭から `publish = false # <理由>`）。ワークスペースは `members = ["crates/*"]` なので登録の作業は要らない
  - 依存はワークスペースに既に在る版の `image`（png だけ）と dev の `temp-path-kit` だけにする
  - 両方のフォルダの直下の `*.png` を名前（大文字小文字を区別しない）で組にし、乗算していない RGBA 8 ビットで読み、組ごとに「同じ」「大きさが違う」「画素が違う（数・色の差の最大・α の差の最大）」「片方にしか無い」「読めない」を名前の昇順で返す。フォルダが読めなければ Err。合否は決めない
  - 試験（`target\` の下に `image` で PNG の組を書く）: 同じ・大きさ違い・画素違い（数と最大の差）・片方だけ・読めない
  - 上の 5 通りの試験が `lib_tests.rs` で緑で、ワークスペースの 1,000 行と一時パスの見張りに引っかからず、`tools/crates-io.ps1`（引数なし）の判定「一覧」「理由」が通る
  - _Boundary: SurfaceCompareTool_
  - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5_

- [x] 4.2 比べる道具の実行ファイル
  - 引数 2 つ（SSP の書き出したフォルダ・areka の書き出したフォルダ）を受け、1 組 1 行の ASCII（例 `same surface0.png`）と最後に件数のまとめ 1 行を出す。比べ終えたら 0、フォルダが読めない・引数が違うときは 2 で終える。どちらのフォルダにも書かない
  - 「引数の列 → 比べる 2 つのフォルダか、終了コード 2」と「比べた結果 → 出す行と終了コード」を `main.rs` の外のライブラリの純粋な関数に置き、`main.rs` はそれを呼んで出すだけにする（クレートの中に閉じるので design の境は越えない）
  - 試験（`lib_tests.rs` か兄弟のテーマのファイル）: 引数が 0 個・1 個・3 個で 2、読めないフォルダで 2、比べ終えたら 0 と ASCII の行とまとめの行
  - 上の試験が緑で、建った実行ファイルを `target\` の下の PNG の組で回すと、上の行と終了コードが出る
  - _Requirements: 13.1, 13.2, 13.3, 13.4_

- [x] 5. areka（画面）: 既定の着せ替えと名前の表をシェルの読みからスコープごとに作る
- [x] 5.1 画面の既定の組と名前の表をシェルの読みから作る
  - 画面の資産の組み立てで、既定の組と名前の表をシェルの着せ替えの読みからスコープごとに作る（2.3 の口）。今の `sakura.*` だけを拾う関数とその試験を消す。シェルの descript.txt が読めないときの今の起動の失敗は残す
  - 期待が意図して変わる試験（画面の資産を通ってスコープ 1 の組を見ているもの）を `kero.*` へ直し、資産の試験に「スコープ 1 の初めの組が `kero.*`・名前の表がシェルの読みから作られる」を足す
  - `cargo test -p areka` が緑で、スコープ 1 の初めの組が `kero.*` になったことを資産の試験が示す
  - _Depends: 2.3, 2.4_
  - _Requirements: 14.1, 14.2, 14.3_

- [x] 5.2 スコープ 0・1・2 で既定と名前が分かれる検体で画面の組を確かめる
  - research.md §10.3 の `g_scope` と同じ形の検体（`sakura.*`・`kero.*`・`char2.*` で既定と名前が分かれる）を `target\` の下に組む部品を、areka のクレートの試験専用の共有の部品（クレートの根に `src/scope_bind_fixture_test_support.rs`。design の依存の決まりで `offline_dump` は `emo2_boot` の他の部品を使わないため、`emo2_boot/` の下には置かない・`#[cfg(test)]`・`pub(crate)`）として 1 つだけ置く。areka は bin だけのクレートで、画面の試験と 7.8 の書き出しの試験は同じクレートの中の `#[cfg(test)]` なので、どちらからもこの 1 つを使う（検体の組み立てを写さない）
  - 画面の状態の初めの組がスコープ 0・1・2・3 でそれぞれ赤・緑・青・空になり、スコープ 1・2 への `\![bind]` がそのスコープの名前の表で当たることを確かめる
  - 上の試験が緑（14.4 の画面の側）で、検体の部品が 7.8 から呼べる可視性で置かれている
  - _Requirements: 14.1, 14.2, 14.4_

- [ ] 6. areka の共有の部品と起動オプションの欄
- [x] 6.1 (P) PNG にする関数を共有の置き場へ移す
  - 色の戻しと PNG の組み立てを MCP の書き出しの中から `png_image.rs` へ中身を変えずに移し、MCP の base64 の関数は移した関数の結果を base64 にするだけにする（`mod png_image;` を `main.rs` に足す）
  - 色の戻しと PNG の形の試験を `png_image_tests.rs` へ移し、base64 の試験は残す
  - 移した試験と MCP の `dump_surface`／`dump_balloon` の今の試験が期待を変えずに緑
  - _Boundary: PngEncoder_
  - _Requirements: 4.4, 4.6_

- [ ] 6.2 (P) 記録の写しの窓
  - 履歴の本文の組み方（`message` の後に ` 名前=値`）を切らない関数に分け、履歴だけが 4,096 字で切るようにする
  - プロセスの受け口の 3 つ目の層を足す: 出来事で warn 以上だけを関心とし、窓が開いているあいだ、窓を開けたスレッドの記録だけを、現地時刻の時・分・秒・レベル・切らない本文で積む。窓を開ける口（開いていれば空にして開け直す・呼んだスレッドを覚える）と閉じて残した順に返す口を足す。排他の中で記録のマクロを呼ばない。受け口は差し替えない
  - 試験（`log_history_capture_tests.rs`・窓を触る試験はファイルの中の排他 1 つで順に回す）: 本文を切らない・warn 以上だけ写す・違うスレッドから積んでも入らず同じスレッドなら入る・閉じた窓には積まない
  - 上の試験が緑で、`log_history.rs` が約 515 行・受け口の差し替えの見張り（`with_default_guard_test.rs`）が緑のまま
  - _Boundary: RecordCapture_
  - _Requirements: 7.3, 7.4, 7.5, 8.3_

- [ ] 6.3 (P) 名前とフォルダ名でシェルを探す段を切り出す
  - 名前でシェルを切り替えるときの「名前の一致 → フォルダ名の一致」の段を、特別な名前（random・lastinstalled）を解かない関数に切り出し、今の切り替えは特別な名前の判定の後にそれを呼ぶ
  - 試験: 名前で見つかる・フォルダ名で見つかる・名前を先に見る・`random`／`lastinstalled` をただの名前として探す・見つからない
  - 上の試験と今の切り替えの試験が期待を変えずに緑
  - _Boundary: SkinNameMatch_
  - _Requirements: 2.4_

- [ ] 6.4 (P) オフラインの書き出しの欄を起動オプションの読みに足す
  - 起動オプションの結果にオフラインの欄（`--offline-dump` があれば値の有無に関わらず在る・8 つの生の文字列・空の値は値なし）を足し、表の `offline-dump` を効く行に、`dump-*` の 7 つを値の欄に、`dump-surface-option`・`dump-animation` を「まだ扱わない」に替える。ほかの `dump-` の名前は今までどおり扱わない名前
  - `--offline-dump` があるときに値の無い `dump-*` の 7 つは値なしの記録（`[cli]`・warn）を出す。`--offline-dump` の値なしは記録しない
  - 今の `parse_tests.rs` の `offline-dump` を扱わない名前とする期待を直し、新しい `parse_offline_tests.rs` に値の有無・7 つの値と `=` の形・空の値・重なり・`--dump-scope -1`・相対パスを生のまま運ぶこと・`--help` と一緒のとき両方が立つことの分岐を書く
  - 上の分岐の試験が緑で、`parse.rs` が 1,000 行の内
  - _Boundary: CliOfflineFields_
  - _Requirements: 1.8, 2.2, 10.2, 12.1_

- [ ] 6.5 オフラインと一緒に来た起動オプションの記録
  - 列を読み終えた後に、`--offline-dump` があるときの `--ghost`・`--random`・`--option`・`--sstp-listen`・位置の引数を「効かせなかった」、`dump-surface-option`・`dump-animation` を「まだ扱わない」、`--offline-dump` が無いときの `dump-*`（まだ扱わない 2 つを含む）を「`--offline-dump` なしでは効かない」として、1 件ずつ warn（`[cli]`）で出す。`--option` の語ごとの今の記録はそのまま
  - `parse_offline_tests.rs` に 3 種類の記録の分岐を足して緑
  - _Requirements: 10.1, 10.3, 10.4, 12.1_

- [ ] 7. オフラインの書き出しのモジュール
- [ ] 7.1 モジュールの骨と検体の部品
  - `offline_dump/` の骨（入口と 5 つの子モジュールの宣言・兄弟の試験の接続）を置き、`main.rs` に `mod offline_dump;` を足す。`crates/areka/Cargo.toml` の dev の依存にワークスペースに既に在る版の `image`（png だけ）を足す
  - 検体の部品（試験専用）: `temp-path-kit` で `target\` の下に、ゴーストの descript.txt・シェルの descript.txt・surfaces.txt・1 色塗りの α 付き PNG を持つゴーストのフォルダを組む。research.md §10 の検体の形（runonce・always・random・periodic・負の座標・mustselect／multiple・element の絵の欠け・中身の無い面）を組める（スコープ別の着せ替えの検体は 5.2 の共有の部品を使う）
  - element の絵の欠けの面は、ほかに描ける層（土台か別の element）を少なくとも 1 つ持たせる（記録が Error 1 行だけになり、合成は失敗しない）
  - 合成の失敗（中身が空）の面は、中身の無い `surfaceN { }` のブレスだけで `surfaceN.png` を置かない形にし、同じシェルにほかの描ける面を持たせる（読み込みはその面を残し＝`decode.rs` 146 行目、合成は error を 1 行出して失敗する＝`plan.rs` 631〜637 行目）
  - 部品で組んだ検体を、今のシェルの読み込みで窓なしに読める試験が緑
  - _Requirements: 12.6_

- [ ] 7.2 (P) 面の指定の読みと展開
  - 7.1 の骨の後に、7.3・7.4 と並べて行える（それぞれ自分のファイルだけを触る）
  - カンマ区切りの各要素の前後の空白を除き、頭の `surface`（大文字小文字を区別しない）を 1 つ外して、単独・範囲・除外・除外の範囲として読む。読めない要素（空・数でない・`a > b`）は warn を出してその要素だけ飛ばす
  - シェルに在る面の番号へ、包含を集めて除外を引き、昇順・重ねなしで展開する。単独の番号で指した無い面は warn、範囲の中の欠番は記録しない
  - 試験: 頭の有無・範囲・除外・除外の範囲・読めない要素・単独の無い面と範囲の欠番・重ね・空白（記録は `log-capture-kit` で数える）
  - 上の試験が `surface_list_tests.rs` で緑
  - _Boundary: SurfaceListReader_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.7, 12.1_

- [ ] 7.3 (P) `--dump-bind` の読みと当て方
  - `;` で区切り（中身の空の区切りは飛ばす）、各指定を 2 つか 3 つの欄として読む。名前の前後の空白は取らない。パーツが空か `__ALL__` はカテゴリ全部。ON/OFF は `1`・`true`・省略で ON、`0`・`false` で OFF、ほかと欄の数の違いは読めない warn
  - 書かれた順に、2.4 の名前で当てる関数を 1 パーツずつ呼ぶ。カテゴリ全部は 2.3 の書かれた順のパーツ名で 1 つずつ呼び、無いカテゴリは warn を 1 件。結果ごとに、無い名前は warn、mustselect の OFF は info（書き出しの記録に入らない）。当てるのは指定のスコープだけで、名前の表はそのスコープのもの
  - 試験: research.md §10.4 の 13 通りの表と、スコープ 0・1・2 の名前の表
  - 上の試験が `bind_spec_tests.rs` で緑
  - _Boundary: BindSpecApplier_
  - _Depends: 2.3, 2.4_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 5.8, 5.9, 12.1_

- [ ] 7.4 (P) 書き出しの記録の CSV と終了コード
  - 写した記録から、空なら 0・warn だけなら 1・error があれば 2 の終了コードと、最も重いレベルの語（`Warning`／`Error`）を返す。CSV の有無に関わらない
  - CSV の中身: 先頭 `EF BB BF`・各行 `"HH:MM:SS","<ゴースト名>","<レベル>","<内容>"`＋CRLF・列の中の `"` は `""`・件数の上限なし。書く口は、記録が空なら何もせず在るファイルに触れず、書けなければ error を出して Err
  - 試験: 終了コードの段（空・Warning・Error・混ざり）・BOM・CRLF・`""`・4 列・レベルの語・ゴースト名が空・空なら作らず在るファイルを残す・10,001 件を間引かない・書けないときの error
  - 上の試験が `report_tests.rs` で緑
  - _Boundary: DumpReport_
  - _Depends: 6.2_
  - _Requirements: 7.1, 7.2, 7.4, 7.5, 7.6, 7.7, 7.8, 8.1, 8.2, 8.3, 12.3_

- [ ] 7.5 面 1 つを時点 T まで進めて組み、シェル全体の下地に置く
  - 2.5 の口で時点 T の面の状態を得て、画面と同じ入力の形で合成し、シェル全体の外形（3.1）の大きさの透明な下地へ、組んだ絵の原点を `(-left, -top)` に置いた乗算済み BGRA を返す。合成の失敗は合成自身が error を出すので info だけを足す。下地からのはみ出しは error で失敗にする
  - 試験（7.1 の検体）: 下地の画素が、同じ面・同じ着せ替え・同じ状態で合成した絵を `(-left,-top)` に置いたものと同じ・下地の大きさと位置がどの面も同じ・透明は `(0,0,0,0)`・PNG が MCP と同じ関数から出る・runonce の最後のコマ・always が T のコマ・random が出ない・2 度で同じバイト列
  - 上の試験が `render_tests.rs` で緑
  - _Depends: 2.5, 3.1_
  - _Requirements: 1.5, 4.1, 4.3, 4.4, 4.5, 4.6, 4.7, 4.9, 12.2_

- [ ] 7.6 面の PNG をファイルに書く
  - 名前は `<頭の文字列><番号>.png`（既定 `surface`・`--dump-output-prefix` で置き換え・スコープは入れない）。書く直前に出力先を作る。頭の文字列の `\`・`/` は出力先からの相対の道として扱い、その下のフォルダは作らない。同じ名前は置き換える。書けなければ error を 1 行出して失敗を返す
  - 試験: 既定の名前・頭の文字列の置き換え・出力先を作る・置き換え・その下の道（在る／無い）・名前に使えない文字・スコープが名前に入らない
  - 上の試験が `render_tests.rs` で緑で、`render.rs` と試験のファイルが 1,000 行の内（超えるならテーマで分ける）
  - _Requirements: 4.8, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 12.1_

- [ ] 7.7 書き出しの走りの前半: ゴースト・シェル・写しの窓・始めと終わり
  - 走りの入口: 始めの info（ゴースト・シェル・スコープ・面の指定・出力先・CSV の生の値）→ ゴーストのフォルダを起動したときの今のフォルダから絶対にしてゴーストの名前を読む（値なし・失敗は error を出して CSV を作らず 0）→ 写しの窓を開ける → スコープを読む（無ければ 0・0 以上の整数で読めなければ 0 と warn＝design の順どおり窓を開けた直後・`--dump-shell` の前に置き、CSV の行の順を design と同じにする）→ `--dump-shell` を 6.3 の段で探す（見つからなければ Error を 1 件・面の段を飛ばす）→ シェルを解決する → COM（MTA）を初期化する → シェルを読み、面の表を 1 度だけ受け取る（フォルダが無い・COM の失敗は走りが error、読み込みの失敗は下の層の error だけ）→ 写しの窓を閉じる → 終了コード → CSV → 終わりの info（書いた数・書けなかった数・最も重いレベル・終了コード）
  - 途中の失敗も必ず窓を閉じる段へ流れる。areka の記憶・置き場・MCP・窓の部品を呼ばない（許す依存の一覧の外を使わない）
  - 試験（`offline_dump_tests.rs`・7.1 の検体）: ゴーストのフォルダが無い・descript.txt が無い・値なしで 0 を返し CSV を作らない、ゴーストのフォルダの末尾の `\` の有無で同じシェルになる、`--dump-shell` が名前とフォルダ名で見つかる・見つからない、シェルのフォルダが無い、スコープの無し・読めない値で 0
  - 上の分岐の試験が緑
  - _Depends: 1.3, 3.3, 6.2, 6.3, 6.4, 7.4_
  - _Requirements: 1.1, 1.6, 1.7, 1.8, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.8, 3.10, 8.4, 9.4_

- [ ] 7.8 書き出しの走りの後半: スコープ・着せ替え・面の段
  - 7.7 で読んだスコープを使う。シェルの着せ替えの読みから既定の組と名前の表を画面と同じ口（2.3）で作り、スコープの状態に `--dump-bind` を当て（7.3）、そのスコープの組をその走りの着せ替えにする（利用者の着せ替えの記憶は読まない）
  - 面の表から SERIKO の表とシェル全体の外形を 1 度だけ作り、面の指定があれば昇順に 1 面ずつ描いて書く（7.5・7.6）。失敗した面は飛ばして次へ進み、書いた数と書けなかった数を数える。面の指定が無ければシェルの読み込みまでで閉じる（`--dump-bind` も読まない）
  - 試験（`offline_dump_tests.rs`）: 5.2 で置いた共有の検体の部品（同じクレートの `#[cfg(test)]` の部品なので呼べる）をそのまま使い、スコープ 0・1・2・3 の書き出しの組が `--dump-bind` どおりで、画面の状態の初めの組と同じ・面の指定なしで面を書かない・中身の無い面の合成の失敗で残りの面が書かれる
  - 上の試験が緑
  - _Depends: 2.3, 5.2_（5.2 の検体の部品を使うため）
  - _Requirements: 3.6, 3.8, 3.9, 3.10, 4.2, 4.5, 4.8, 14.3, 14.4_

- [ ] 8. 入口でオフラインへ渡して終える
  - 記録の初期化と起動オプションの読みと `--help` の判定の直後に、オフラインの欄があれば走らせてその終了コードでプロセスを終える分岐を数行で足す（スレッドの役目の登録・性能の報告・起動の解決・書けるかの確かめ・二重起動の確かめ・MCP・窓より前）。判断と記録は `offline_dump` に置く
  - `--help` が真ならオフラインは走らない
  - `main.rs` が 1,000 行の内で、建った `areka.exe --offline-dump <検体>` を `target\` の下の検体で 1 度回すと、窓を出さずに PNG を書いて終わり終了コードを返す（分岐ごとの確かめは 10 の試験）
  - _Depends: 7.7, 7.8_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.9, 9.1, 9.2, 9.3, 10.2_

- [ ] 9. 配布物の説明書と steering の構造
  - `dist/README.txt` の起動オプションの節: 効く起動オプションに `--offline-dump` を加え、`--dump-*` の 7 つの働き・書き出す名前・CSV の形・終了コード（0・1・2、3 は出ない）・`start "" /wait` と `echo %ERRORLEVEL%` の例・まだ扱わない 2 つ（記録に残して無視）を書き、「まだ扱わない」の文から `--offline-dump` を外す
  - `.kiro/steering/structure.md` に新しいクレート `surface-compare` の項と、`offline_dump` の許す依存を書く
  - 説明書に 11.1〜11.3 の各項が在り、`help_tests.rs` が緑
  - _Requirements: 11.1, 11.2, 11.3_

- [ ] 10. 建った areka.exe での決定論の試験
- [ ] 10.1 書き出しの成功と CSV の中身
  - `crates/areka/tests/offline_dump_exe_test.rs` を足し、建った `areka.exe` を `std::process::Command` で回して終了コードとファイルを読む（根・検体・出力はすべて `temp-path-kit` で `target\` の下）
  - areka は bin だけのクレートなので、統合の試験から `#[cfg(test)]` の `offline_dump_test_support` は使えない。検体は統合の試験の側で `temp-path-kit`・`image`（dev）・`std` だけで組む（`structure.md` の `tests/` の決まりどおり、入口の `tests/offline_dump_exe_test.rs` は `#[path]` の宣言だけにし、試験の本体はテーマごとに `tests/offline_dump_exe_test/` の下へ、共有の部品は `tests/offline_dump_exe_test/common/mod.rs` に置く）。試験のファイルは 1,000 行の内で、超えるならテーマで分ける
  - 面の画像の欠けの面にはほかに描ける層を、合成の失敗の面は中身の無いブレスだけで画像を置かない形にする（7.1 と同じ決まり）
  - 誤りの無い検体: 終了コード 0・CSV を作らない・在った CSV を残す・PNG の名前と大きさ
  - 面の画像の欠け（Error）と無い単独の番号（Warning）の検体: 終了コード 2・BOM・CRLF・行の数・レベル・同じ本文の行が無い（入れ子の循環と知らない sort と描かない method を持つ検体でも形の誤りは 1 行）・起動オプションの名前の記録（`[cli]`）が入らない
  - 同じ指定で 2 度回すと PNG のバイト列が同じ
  - 上の試験が緑
  - _Depends: 8_
  - _Requirements: 4.9, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 8.1, 12.3, 12.6_

- [ ] 10.2 失敗の分岐と終了コード
  - ゴーストのフォルダの末尾の `\` の有無で同じ PNG・`Command::current_dir` で起動したときの今のフォルダを `target\` の下に置き、相対の `--offline-dump`・`--dump-output-dir`・`--dump-error-log` がそこから読まれる（相対の出力先に PNG と CSV が書かれる）
  - `--dump-error-log` なしでも同じ終了コード・`--dump-shell` が無い名前で PNG なし・終了コード 2・ゴーストのフォルダが無いと終了コード 0・CSV なし・面の指定なしで PNG なし・CSV は出る・`--offline-dump` に値なしで終了コード 0・中身の無い面で合成の失敗の Error がちょうど 1 行で残りの面は書かれる・シェルのフォルダが無いと終了コード 2
  - 上の各分岐の試験が緑
  - _Requirements: 1.8, 2.1, 2.2, 2.5, 2.6, 2.7, 3.6, 4.8, 8.2, 8.4, 12.4, 12.6_

- [ ] 10.3 書かないことの試験
  - 環境変数 `AREKA_USER_ROOT`（書ける根）と `AREKA_PROFILE_DIR`（記憶の置き場＝`areka-user-data/src/lib.rs` 18〜20 行目）をそれぞれ `target\` の下の空の根に向け、`AREKA_ROOT`（置き場の並び）も読まれるなら同じく空の根に向ける
  - 回す前後で、ゴーストのフォルダの木・書ける根の木・記憶の置き場の木（名前・大きさ・更新時刻）を比べて同じであること、出力先に書かれたのが PNG と CSV だけであることを確かめる
  - 上の試験が緑
  - _Requirements: 1.2, 1.3, 1.4, 9.1, 9.2, 9.3, 9.4, 12.5, 12.6_

## Implementation Notes

- 1.2: `char*` の欄は `BindGroupDefaults` の pub の BTreeMap 4 本（`char_default_on`・`char_names`・`char_mustselect`・`char_multiple`）。スコープごとの名前を引く `names()` は private なので、2.3 は `char_names` を直に読み、名前・既定・option のどれかを持つスコープを拾うには 4 本の鍵を合わせて見る。`Char(0)`／`Char(1)` は空を返す
- 2.1/2.2: `DefaultBinds`（`areka-seriko/src/bind.rs`）は `uniform`・`single`・`Default`・`for_scope`。試験 3 か所（assets_tests:150・switch_tests:795・frame_attach_carry_tests:123）は `.ids()` を `for_scope("0")` で読む＝2.3／5.1 で既定がスコープごとになったら見直す。`cargo test -p areka` の real-run 統合試験（mcp_get_log_real_run・smoke_boot_loop_exit）は i686 の helper を先に建てないと赤（`tools/test-all.ps1` は先に建てる）。単体は `cargo test -p areka --bins`
- 2.3: 画面は 5.1 までスコープ 2 の `\![bind]` を error（名前を解決できない）で読み飛ばす＝5.1 で `assets.rs` を `BindResolver::from_groups` へ替えて閉じる。`BindGroupName` は areka-parsers の外から名指せないので `from_groups` は pub の欄を `decls!` マクロで開く
- 2.4: `apply_named_bind` の結果は design の形と違い `Applied { id, outcome }`・`MustSelectKept { id }` と id を持つ（画面の記録「bind 適用」「mustselect カテゴリの脱衣指示を無視」が欄 `id` を出すため）。7.3 は id を使わなくてよい
- 3.1: `shell_bounds` は右下に面ごとの `compute_extent` も重ねる（非 overlay のメソッドの pattern も合成の絵を広げるため）。`tracing::subscriber::with_default` は本番で使えない（`log-capture-kit` の `with_default_guard_test` が見張る）。bind／always の pattern が循環するシェルでは `compute_extent`（`plan_extent.rs:78`）が warn を出す＝3.2 で debug に下げ、循環する bind の検体で `shell_bounds` が黙ることも 3.2 で確かめる
- 3.2: 転記層は知らない `animation-sort`／`collision-sort` を `SortOrder::Other(Box<str>)` として残す（前は記録なしで捨てていた＝設計の前提と合わず、記録の無い失敗の道を作らない決まりで根本を直した）。`ShapeIssue::UnknownAnimationSort { value }` はシェル全体で 1 件・面と animation の欄なし＝3.3 はその欄なしで出す。`move` は `nesting.rs::pattern_edge` で辺にしない（NestReport と共有）。描かない method で親へ戻る pattern は `Cycle` と `UndrawnPattern` の 2 件になる＝3.3 の突き合わせで 1 誤り 1 行に寄せる
- 4.2: surface-compare の行の形は `same|size-differs|pixels-differ|only-in-ssp|only-in-areka|unreadable <名前> ...` とまとめ `total=.. ...`。フォルダが読めないときの行はどちらのフォルダかを言わない（要件の外・直すなら `compare_dirs` の `list_pngs` の `?` を `map_err` で側と道筋つきに）
- 5.2: 共有の検体 `crates/areka/src/scope_bind_fixture_test_support.rs`（`scope_bind_ghost()`）の `write_solid_png` は MCP の非公開の `fn png` の一時の写し＝6.1 で `png_image.rs` へ移したら移した関数を呼ぶ形に替え、shortcut の注記を消す
