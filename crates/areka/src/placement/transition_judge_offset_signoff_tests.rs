//! バルーン追従オフセットの**実機サインオフ手順書**と、その判定ランナー
//! （task 8.2・要件 8.1／8.2／8.3／8.4／8.5。手順は task 8.5 で改訂——検体の名指し・
//! 起点の前提・人の手でのドラッグ。task 8.7 で再改訂——実機で判った 3 件＝必須手の撤去・
//! 保存済みオフセットの掃除・32bit helper のアーキテクチャ確認）。
//!
//! design.md「実機サインオフ（要件 8）」の 5 は「手順書はモジュール doc として置く」と定める。
//! 本ファイルの module doc が**その手順書そのもの**であり、直下の `#[ignore]` ランナーが
//! 決定論テストと**同一の判定器**（[`super::judge_offset_log`]）を実機ログへ当てる。判定の
//! 実装をここに 1 行も持たないのが要点で、持った瞬間に「決定論テストが緑でもサインオフだけ
//! 別の判定を通る」形が生まれる。
//!
//! 先行仕様の手順書ファイル（完了アーカイブ側）は**書き換えない**——自らの行に限る規律を
//! 検証側にも適用する（要件 6.6）。本手順書が引き受けるのは `kind=offset` の観測点だけである。
//!
//! # 0. 合否を決めるのは誰か（要件 8.3）
//!
//! **合否は記録の機械判定で決める。** 採取者の仕事は「手順どおりに動かしてログを作ること」
//! だけであり、合格・不合格を目で決めてはならない。判定はランナー（§4）が刷る
//! 「違反 0 件」かどうかで読む。目視は §6.3「判定器が見ないもの」を補うためだけに行い、
//! その結果は判定器の合否を上書きしない。
//!
//! # 1. 環境と検体（要件 8.1）
//!
//! 拡大率の異なるモニタを **2 台以上**（125% 相当と 200% 相当）備えた実機で行う。実効 DPI は
//! DPI 対応プロセス自身のログで確かめる——非対応プロセスから読むと全モニタが 96 に丸められ、
//! 「拡大率の差を 1 度も跨いでいないログ」を作ってしまう。
//!
//! 検体はリポジトリ内の次の 2 つを使う（**この組み合わせ以外では §3 の手順を満たせない**）。
//!
//! | 役 | ディレクトリ（リポジトリ相対） |
//! |---|---|
//! | ゴースト | `crates/pilot/examples/shiori-host-32/fixtures/emo2` |
//! | バルーン | `crates/pilot/examples/shiori-host-32/fixtures/emo2-kakukaku-offsetdpi` |
//!
//! バルーンは **scope 0 がキーワード指定（`windowposition.x,center`）・scope 1 が数値指定
//! （`windowposition.x,-190`）** の混成である。§3 の必須の手は、素の追従スコープ（手 2）と
//! キーワード指定スコープ（手 3）の**両方**を要求する。片側しか持たない検体では手順を
//! 最後まで回せない——
//!
//! - `fixtures/emo2/emo2-kakukaku`（原本）: 両スコープとも数値指定でキーワードが無い。
//!   判定 ⑷ の母数が空になり「揃えを 1 度も測れていない」の**偽の赤**が出る。
//! - `fixtures/emo2-kakukaku-wplimit`: 両スコープともキーワードで素の追従が無い。
//!   判定 ⑶ の母数が空になり得て「低い拡大率側で追随が出ていない」の**偽の赤**が出る。
//!
//! 混成検体の由来（どちらの複製で、どの 2 ファイルだけが違うか）は
//! `fixtures/emo2-kakukaku-offsetdpi/readme.txt` に書いてある。
//!
//! # 2. 採取の準備と観測の点灯
//!
//! ## 2.1 実行体を作り直す
//!
//! 起動する実行体は本ブランチのビルドであること（古い実行体を黙って測ると、直っていない
//! ものを直ったと読む）。採取の前に必ずビルドし直す。
//!
//! ```text
//! cargo build -p areka
//! ```
//!
//! ## 2.2 実行体の隣の 32bit helper のアーキテクチャを確かめる
//!
//! areka はゴーストの SHIORI を、**自分の実行体と同じフォルダに置かれた**
//! `shiori-host32-helper.exe` 経由で読み込む（`crates/areka/src/boot_config.rs` の
//! `default_helper_exe_path`——`current_exe()` の親フォルダへこのファイル名を結合するだけで
//! あり、別の場所を指させる環境変数は無い）。この helper は **32bit（i686）でなければ
//! ならない**。
//!
//! ワークスペース全体をビルドすると、**同じ名前の 64bit の helper** が実行体の隣
//! （`target/debug/`）へ置かれることがある。その状態で起動すると SHIORI の読み込みが落ち、
//! **ゴーストが 1 枚も表示されない**。実測のログはこの形である（2026-08-28）。
//!
//! ```text
//! [helper] LOAD 失敗（観測・ack[0]）: LoadLibraryFailed(... HRESULT(0x800700C1), "%1 は有効な Win32 アプリケーションではありません。")
//! ```
//!
//! 正しい成果物は `target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe` である
//! （作り方は下記 1 行目）。**採取のたびに、これを実行体の隣へ上書きしてから起動する。**
//!
//! ```text
//! cargo build -p shiori-host32-helper --target i686-pc-windows-msvc
//! Copy-Item "<リポジトリの絶対パス>\target\i686-pc-windows-msvc\debug\shiori-host32-helper.exe" "<リポジトリの絶対パス>\target\debug\shiori-host32-helper.exe" -Force
//! ```
//!
//! 上書きできたかは、**置いた実行ファイルの中身**（PE ヘッダの machine 欄）で確かめる。
//! 更新時刻やファイル名では確かめない——名前が同じまま中身だけ 64bit に入れ替わるのが、
//! この落とし穴の姿だからである。`014C` なら 32bit（i686）＝正しい。`8664` なら 64bit＝
//! **そのまま採取してはならない**。
//!
//! ```text
//! $HELPER = "<リポジトリの絶対パス>\target\debug\shiori-host32-helper.exe"
//! $fs = [IO.File]::OpenRead($HELPER); $br = New-Object IO.BinaryReader($fs)
//! $fs.Position = 0x3C; $pe = $br.ReadInt32(); $fs.Position = $pe + 4
//! '{0:X4}' -f $br.ReadUInt16()
//! $br.Close(); $fs.Close()
//! ```
//!
//! ## 2.3 保存済みのバルーンオフセットを消す
//!
//! **採取のたびに、永続プロファイルの保存済みのバルーンオフセットを消してから起動する。**
//! 消す対象は、次のファイルの `[balloon-offset.*]` の節（在るものすべて）である。
//!
//! ```text
//! crates/pilot/examples/shiori-host-32/fixtures/emo2/ghost/master/profile/areka/sylphya.toml
//! ```
//!
//! *理由*: 保存値が効いたスコープでは、キーワード再導出の素材が落ちる
//! （`crates/areka/src/placement/persist.rs:388` 付近の規約。保存値優先の順位が静かに
//! 反転しないための**正しい規約**であって欠陥ではない）。その結果、**保存値が残っている
//! 環境では門の行（`verdict=keyword-pending`）が 1 行も出ず**、判定 ⑷ の母数が空になる
//! （**偽の赤**＝「揃えを 1 度も測れていない」）。実測では、2 本目の採取でキーワード指定
//! スコープが `verdict=anchored`（＝復元した未係留の保存値を最初の観測で係留した形）に
//! なり、⑷ が空になった。
//!
//! *保存値はどこから来るのか——本手順の外の操作である*: `[balloon-offset.*]` を書く経路は
//! リポジトリ全体で **1 本だけ**である。**バルーンを単独で掴んで動かし、離したとき**
//! （`placement/follow/drag_follow.rs` の `on_balloon_drag_end`——ここからの
//! `balloon_offset_entries` 呼出が、この関数の**唯一の**呼出点である。結線は
//! `placement/spawn.rs` のバルーン窓の `OnDragEnd`）。**キャラ窓のドラッグ＝§3 手 4 の
//! モニタ間の往復では 1 バイトも書かれない。** しかも §3 手 4 自身が「往復のあいだ、
//! バルーンをドラッグしない」と命じているので、**手順どおりに回した採取そのものは保存値を
//! 作らない**。作るのは手順の外の操作である——試行錯誤の途中でバルーンを摘まんだ、別の
//! 作業で動かした、等。なお**同じ運転の内側でも**、バルーン単独ドラッグはその場でキーワード
//! 素材を退役させる（同ファイルの `retire_keyword_base_on_save`）ので、採取中もバルーンは
//! 掴まない（§3 手 4 の禁止はこの 2 つを同時に塞いでいる）。
//!
//! ゆえにこの掃除は「前の採取が必ず汚す」からではない。**汚れているかどうかを採取者が
//! 見分けられない**からである——保存値の有無は**本手順の点灯（§2.4）では 1 行も出ない**し、
//! 前の採取が手順どおりだったかは後から確かめようがない。毎回消して、既知の状態から始める。
//!
//! 掃除が効いたことを自分の目で確かめたいときは、点灯へ `areka::persist::restore=info` を
//! 足す（例: `$env:RUST_LOG = "wintf::transition=debug,areka::persist::restore=info"`）。
//! `merge_scope restore` の行がスコープごとに出て、`saved_off_x`／`saved_off_y` が
//! `None` なら保存値は残っていない（`crates/areka/src/placement/persist.rs` の
//! `merge_scope` が刷る）。判定そのものはこの行を読まないので、足しても合否は変わらない。
//!
//! 同じファイルの `[window.*]` の節は**消さなくてよい**（起動位置は保ってよい。判定に効く
//! のはバルーンオフセットの側だけである）。このファイルは実行時に作られる追跡外の
//! プロファイルなので（`crates/pilot/examples/shiori-host-32/.gitignore:3` が
//! `fixtures/emo2/ghost/master/profile/` を除外している）、消してもリポジトリの差分には
//! ならない。
//!
//! ## 2.4 観測を点灯して採取する
//!
//! 観測 target は `wintf::transition`、水準は `debug`。行頭タグは `[transition]` である。
//!
//! ```text
//! $AREKA   = "<リポジトリの絶対パス>\target\debug\areka.exe"
//! $GHOST   = "<リポジトリの絶対パス>\crates\pilot\examples\shiori-host-32\fixtures\emo2"
//! $BALLOON = "<リポジトリの絶対パス>\crates\pilot\examples\shiori-host-32\fixtures\emo2-kakukaku-offsetdpi"
//! $LOG     = "<絶対パス>\signoff.log"
//! $env:RUST_LOG = "wintf::transition=debug"
//! & $AREKA $GHOST $BALLOON 2>&1 | Tee-Object -FilePath $LOG
//! ```
//!
//! **コマンドは PowerShell（本リポジトリの主たるシェル）の形で書いてある。** `VAR=値 コマンド`
//! という先頭への環境変数付与は POSIX シェルの記法であり、PowerShell には無い——貼ると
//! `VAR=値` という名前の**コマンドを探して**「is not recognized」で落ち、1 行も採れない
//! （実測で確認した）。変数は 1 行ずつ `$env:` へ入れ、次の行で起動する。同じ理由で
//! `grep` も使わない（PowerShell には無い・数えるのは `Select-String`）。
//! ゴースト・バルーンは**絶対パス**で与える（相対だと `pasta.dll` の読み込みが落ちる）。
//!
//! 点灯しているかは、採取後に行頭タグを数えて確かめる（0 件なら採り直し）。
//!
//! ```text
//! (Select-String -Path $LOG -SimpleMatch '[transition]' | Measure-Object).Count
//! ```
//!
//! 消灯した観測点の採取を「発生 0 回」の根拠にしてはならない（要件 8.5）。ランナーは追随
//! レコードが 1 行も無いログを**失敗**として落とすので、この取り違えは静かには通らない。
//!
//! # 3. 手順（必須の 5 手・順序も必須・1 手でも欠くと判定が立たない）
//!
//! **前提（なぜ往復操作だけで遷移が切り出せるか）**: 判定器は「拡大率が実際に変わった」記録を
//! 起点にしてログを遷移ごとに切り出す。起点になるのは `kind=monitor`（モニタ表の値が変わった
//! ＝**表示設定の変更**）と `kind=windpi`（**窓**の表示 DPI が書き換わった＝モニタ間の移動）の
//! 2 種別で、いずれも `old_dpi` と `new_dpi` が異なる行だけが起点になる
//! （`transition_judge.rs` の `TransitionOrigin`）。**モニタ間へゴーストを移す操作で出るのは
//! `kind=windpi` の側だけ**である（`kind=monitor` はモニタ表が変わらないので 1 行も出ない）。
//! ゴースト 1 体では窓 1 枚ごとに `kind=windpi` が出るが、同じ変化を指す起点は 1 本の遷移へ
//! 畳まれるので、遷移の本数が窓の枚数だけ水増しされることはない。ゆえに**下記の往復だけで
//! 判定は成立し、表示設定を触る必要は無い**。
//!
//! **拡大率をまたがせる操作は、手 3・手 4 のどちらも手 4 の作法で行う**——人の手で
//! 立ち絵を掴んでモニタ間へドラッグする。外から窓を動かした結果はどの手でも証跡にならない
//! （理由は手 4 に書いた）。
//!
//! 1. **ゴーストを起動し、バルーンを出す。** 起動直後の配置が済むまで待つ。§2 の準備
//!    （実行体の作り直し・隣の 32bit helper の確認・保存済みオフセットの掃除）を
//!    **採取のたびに**済ませてから起動する。
//! 2. **素の追従スコープ（キーワード指定でないバルーン）を最低 1 つ含める**（**必須**）。
//!    素の追従とは、位置がキーワード由来の基本位置で決まっていないバルーンである。
//!    *理由*: 判定 ⑶（低い拡大率側で追随が出ていること）の母数は、スコープごとに
//!    「そのスコープが最後に `verdict=keyword-pending` を出した遷移より後」に限られる。
//!    ログに現れるスコープが**すべて**キーワード指定で、しかも素材の消費が最後の低い側の
//!    遷移より後に来ると、数えられる行が 1 件も無くなり「低い拡大率側で追随が出ていない」の
//!    **偽の赤**が出る。素の追従スコープは門を 1 度も出さないので常に母数に入り、この形を塞ぐ。
//! 3. **キーワード指定のバルーンについて、素材を消費させてから低い拡大率側へ遷移させる**
//!    （**必須**）。素材の消費とは、キーワード由来の基本位置を確定させる面切替・発話を
//!    1 度通すことである。消費させたうえで、**その後に**ゴーストを低い拡大率のモニタへ移す。
//!    *理由*: 判定 ⑷（揃えの残差）は、キーワード指定スコープが**素材消費後に**出した
//!    `verdict=rescaled` の行でしか測れない。消費前の遷移しか無いログは残差を 1 度も測れず、
//!    「揃えを 1 度も測れていない」の**偽の赤**になる（受容された残余＝素材未消費のまま
//!    寸据え置きの遷移を迎えた記録は正しい記録であって欠陥ではない）。
//! 4. **ゴーストを、人の手でドラッグしてモニタ間を往復させる**（125% 側 → 200% 側 → 125% 側 …）。
//!    立ち絵をマウスで掴んで引きずる——**この手は人が実際に手を動かして行う**（下記の限界）。
//!    同じ拡大率へ**戻る**遷移が最低 1 回要る（要件 8.2）。戻らないログは判定 ⑴ を立てられず
//!    「往復が 1 度も観測されていない」になる。往復のあいだ、バルーンをドラッグしない・面を
//!    切り替えない——どちらも基準を引き直すので、往復の区間が切れて突合が成立しなくなる。
//!    *限界（2026-08-28 の実測・裁定済み）*: **合成マウス入力ではゴーストを掴めない。**
//!    areka の窓は 4 つとも `WS_EX_TRANSPARENT` が立ったままで、カーソルを立ち絵の上へ運んでも
//!    外れない（5 水準 × 3 列の 15 点で `WindowFromPoint` は 1 度も areka の窓を返さなかった）。
//!    ゆえにこの手はスクリプトで代行できない。これが合成入力の限界なのか製品の欠陥なのかは
//!    本仕様では切り分けない（開発者裁定）。
//!    *外から窓を動かした結果は証跡にならない*: `SetWindowPos` などで窓を**外から**別モニタへ
//!    移すと表示 DPI は変わり、`kind=windpi` も追随の行も出る。しかしそれは利用者のドラッグ
//!    経路を 1 度も通っていないので、要件 8.2 が言う「ゴーストをモニタ間で往復させる」の充足に
//!    はならない。**そのログでサインオフを出してはならない**（判定器は経路の違いを見ないので、
//!    緑が出てしまう——ここは人が守る規律である）。
//! 5. **先行仕様の残所見を目で確かめる**（要件 8.4 の目視側）: 低い拡大率の側で、バルーンが
//!    キャラに対して定常的にずれていないこと。ずれが見えたら、判定器が緑でも所見として残す。
//!
//! # 4. ランナーの走らせ方
//!
//! ```text
//! $env:AREKA_TRANSITION_LOG = $LOG
//! cargo test -p areka transition_judge_offset_signoff -- --ignored --nocapture
//! ```
//!
//! ここも PowerShell の形である（`$LOG` は §2 で置いた絶対パス）。値は**絶対パス**で与える。
//!
//! 既定の `cargo test` では `#[ignore]` により 1 度も走らない——実機ログが無い環境で
//! 「違反 0 件」を出すと、それが充足の根拠に化けるためである（要件 8.5）。ランナーの本体は
//! [`judges_a_real_machine_offset_log`]、判定の入口は [`signoff_offset_log`] である。
//!
//! **静かに成功しない**: 環境変数が未設定・パスが読めない・追随レコードが 1 行も無い——
//! いずれも**失敗**として落ちる。無視指定のテストが不備なパスで緑になるのは、テストが
//! 無いより悪い。
//!
//! # 5. 判定語（`verdict=` の値・全 6 語）
//!
//! | 判定語 | 意味 | 値 |
//! |---|---|---|
//! | `rescaled` | 基準から表示 DPI 比で引き直した | 動き得る |
//! | `anchored` | 未係留の基準を最初の観測で係留した | 動かない |
//! | `unchanged` | 比が恒等（遷移していない） | 動かない |
//! | `keyword-pending` | キーワード由来の素材が未消費＝見送り（門） | 動かない |
//! | `unresolved` | 基準 DPI か現在 DPI の片側だけが 0＝縮退 | 動かない |
//! | `saturated` | 引き直しが `i32` の域で飽和した | 動き得る |
//!
//! 観測行の見た目（採取者が読む形）:
//!
//! ```text
//! DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=windpi entity=12v1 old_dpi=96 new_dpi=192
//! DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=monitor entity=12v1 old_dpi=96 new_dpi=192 old_wa=0,0,2880,1752 new_wa=0,0,2880,1704
//! DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=offset scope=0 base_dpi=96 new_dpi=192 base_offset=10,20 old_offset=10,20 new_offset=20,40 verdict=rescaled
//! DEBUG wintf::transition: [transition] frame=10 t_us=0 kind=offset scope=1 base_dpi=- new_dpi=0 base_offset=0,0 old_offset=0,0 new_offset=0,0 verdict=keyword-pending
//! ```
//!
//! 先頭 2 行が起点である（§3 の前提）。ドラッグでモニタ間を移した採取では `kind=windpi` だけが
//! 並び、`kind=monitor` は出ない。起点が 1 行も無いログでは、追随の行が何行あっても遷移は
//! 1 本も切り出せない——採取のあと `kind=windpi` を数えて確かめておくとよい。
//!
//! ```text
//! (Select-String -Path $LOG -SimpleMatch 'kind=windpi' | Measure-Object).Count
//! ```
//!
//! 欄が読めなかったところは落とさず番兵 `-` で埋まる（落とすと「記録が出ていない」と
//! 見分けが付かない）。
//!
//! # 6. 合否条件
//!
//! ## 6.1 合格
//!
//! ランナーが刷る 1 行目が次の形（`OffsetReport` の `Display` そのもの）であり、かつ
//! `N`・`M` の**どちらも 0 でない**こと。0 本・0 件で緑は出ない（ランナーが先に落とす）。
//!
//! ```text
//! 追随レコードの判定: 遷移 N 本・offset 行 M 件・違反 0 件
//! ```
//!
//! この 1 行の字面は番人が `Display` から起こして突合する——書式を変えた日にこの節だけが
//! 古くなり、採取者が存在しない語を読む形になるのを防ぐためである。
//!
//! ## 6.2 不合格（判定器が立てる違反・全件が刷られる）
//!
//! | 判定 | 要件 | 立つ違反 |
//! |---|---|---|
//! | ⑴ 往復の前後で反映後の値が bit 同一 | 8.2 | 値が bit 同一でない／往復が 1 度も観測されていない |
//! | ⑵ 判定語が期待の腕 | 8.3 | 期待の腕でない／動かさない腕で動いた／語彙表に無い／門が表示 DPI を運んでいる |
//! | ⑶ 低い拡大率側で追随が出ている | 8.4 | 低い拡大率側で `rescaled` が 1 度も出ていない |
//! | ⑷ キーワード指定スコープの揃えの残差 | 8.5 | 残差が許容量を超えた／揃えを 1 度も測れていない |
//! | 入力そのもの | 8.5 | 追随レコードが 1 行も無い／追随レコードを読めない |
//!
//! 違反が 1 件でもあれば不合格である。「材料が無い」種の違反（往復が観測されていない・
//! 低い拡大率側で出ていない・揃えを測れていない・レコードが 1 行も無い）は、**採り方の不備**
//! であることが多い——§3 の必須の手を 1 つ飛ばすと出る。まず手順を見直し、手順どおりでも
//! 出るなら製品側の欠陥として扱う。
//!
//! ## 6.3 判定器が見ないもの（緑を「全部正しい」と読まないための欄）
//!
//! 判定器が緑でも、次の 5 つは**判定されていない**。これらは task 10.1 の目視サインオフ項目が
//! 引き受ける。
//!
//! 1. **素の追従スコープの値が一貫して間違っていても 4 判定すべてを通る。** 揃えの残差の検査
//!    （⑷）はキーワード指定スコープだけを見るので、素の追従の値がずれたまま往復で再現性を
//!    保っていれば、⑴〜⑶ は成立してしまう。
//! 2. **揃えのずれそのものは見えない。** 残差の検査は D8 が数える 3 つの丸めの出所のうち
//!    1 つだけを測り、そこへ 3 つぶんの上限（3px）を当てている。ゆえに真の残差に対して
//!    およそ 6 倍ゆるい。数 px の見た目のずれは緑のまま通る。
//! 3. **オフセット以外の壊れは視野の外。** キャラ窓の位置・重なり順・可視性・バルーンの寸法は
//!    1 つも見ていない。
//! 4. **一部のスコープだけが毎遷移で基準を建て直す欠陥は隠れ得る。** 他のスコープの往復が
//!    成立していれば ⑴ は緑になる。スコープごとの往復の本数は目で確かめる。
//! 5. **起点より前の行の扱いは判定ごとに違う。** 判定 ⑴⑵⑶ は遷移の内側の行しか読まない
//!    ので、**最初の遷移起点より前に出た記録はこの 3 つでは 1 件も判定されない**——最初の
//!    起点より前の行は**どの遷移にも属さないので捨てる**（`transition_judge.rs` の
//!    `split_transitions`）。**一方、判定 ⑷ の母数だけは起点より前の門の行も読む**
//!    （`transition_judge_offset.rs` の `keyword_pending_scopes`——キーワード指定スコープの
//!    集合を**観測行の全体**から作る・task 8.6）。ゆえに「起点より前の行は判定に無関係」と
//!    読んではならない。なお ⑷ が**残差そのものを測る行は遷移の内側限定のまま**であり
//!    （母数だけが広がった）、合否の厳しさは落ちていない。
//!    起点と認めるのは `kind=monitor` または `kind=windpi` で `old_dpi` と `new_dpi` が異なる
//!    行だけである（同 `TransitionOrigin`・§3 の前提）。
//!    **改訂（task 8.4 で穴が塞がった）**: かつてここには起点が `kind=monitor` だけだと
//!    書いてあり、そのため**モニタ間の往復では起点が 1 行も出ず、手順どおりに回しても遷移が
//!    1 本も切り出せなかった**（2026-08-28 の実機試行で判明・`kind=offset` は 3 行採れたのに
//!    `kind=monitor` は 0 行）。起点集合が `kind=windpi` へ広がったことでこの穴は塞がっており、
//!    §3 の往復で出る追随の行は判定の内側に入る。
//!    **残るのは起動直後の 1 巡目である（判定 ⑴⑵⑶ について）**——1 巡目は全窓へマッチして
//!    追随の行を出す（`emo2_boot/frame/dpi.rs` の `dpi_phase_with`）が、その時点でまだ起点が
//!    1 行も出ていなければ、それらの行はどの遷移にも属さず捨てられる。起動直後の**値**の
//!    正しさは判定器の視野の外であり、目で確かめる（起動直後に出る門の行が ⑷ の母数へ
//!    入ることとは別の話である——母数に入るのはスコープの名であって値ではない）。
//!
//! # 7. 記録に残す限界（この手順が実際に踏んだ落とし穴）
//!
//! ## 7.1 撤去した必須の手——「素材が未消費のうちに 1 度跨がせる」は人の手では実行できない
//!
//! 本手順書には 2026-08-28 まで、必須の手として「キーワード指定のバルーンを、素材が
//! 未消費のうちに遷移へ 1 度通す」（当時の手 3）があった。**この手は撤去した。**
//!
//! *撤去の理由*: 判定 ⑷ の母数の作り方が変わった（task 8.6）。門の行を**観測行の全体**から
//! 拾うようになったので、門の行が遷移の内側に在る必要は無くなった。当時の手 3 が守らせて
//! いたものは、いまは実装の側で満たされている（§6.3 の 5 件目）。
//!
//! *なぜ人の手では実行できなかったか（実測）*: 門の行（`verdict=keyword-pending`）は
//! **起動 +0.00 秒に 1 行出るだけ**であり、素材は **+0.73 秒**（1 本目の採取では +5.0 秒）に
//! areka 自身の起動系列（`ReportedSizeReconcile`）が**自動的に**消費してしまう。一方で
//! 最初の遷移起点は利用者のドラッグ由来ゆえ、必ずそれより後になる（実測 +5.0 秒）。
//! ゆえに門の行は**構造的に必ず最初の起点より前**にあり、遷移の内側には決して現れない
//! ——手順どおりに動かしても、当時の判定 ⑷ の母数は永久に空のままだった。つまり
//! **正しい実装のまま「揃えを 1 度も測れていない」の偽の赤が出続ける形**であり、しかも
//! 採取者にはそれと採り方の不備との区別が付かない。撤去はこの構造を直した結果である。
//!
//! ## 7.2 窓が「見えている」ことを `IsWindowVisible` で判断してはならない
//!
//! §2.2 の 64bit helper を掴んだままの採取では、areka の窓は **4 枚とも `IsWindowVisible` が
//! 真で、座標も画面の内側**にあった。それでも画面には**何も映っていなかった**——SHIORI が
//! 読み込めず、中身が 1 枚も合成されていなかったからである（窓は在るが空である）。この
//! 状態を API の戻り値だけで「表示されている」と読み、別の欠陥を疑って時間を溶かした実例が
//! ある（2026-08-28）。**見えているかどうかは目で見て確かめる。** API に尋ねてよいのは
//! 「窓が在るか」までであって、「中身が映っているか」ではない。

use std::fs;
use std::path::Path;

use temp_path_kit::TempPath;

use super::transition_judge_offset_tests::{log_of, pass_lines};
use super::{OffsetReport, judge_offset_log};
use crate::placement::transition_diag::KIND_OFFSET;
use crate::placement::transition_judge::TRANSITION_LOG_ENV;
use wintf::ecs::window::transition_diag::FIELD_KIND;

/// 実機ログを読み、追随レコードの判定まで通す。
///
/// `raw_path` は環境変数の値（未設定なら `None`）。I/O の失敗も「追随レコード 0 行」も `Err`
/// にする——どれも「違反 0 件」を作れてしまう入力だからである（要件 8.5）。
fn signoff_offset_log(raw_path: Option<&str>) -> Result<(OffsetReport, String), String> {
    let Some(raw_path) = raw_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Err(format!(
            "{TRANSITION_LOG_ENV} が未設定である。実機ログの絶対パスを PowerShell で与えて \
             `$env:{TRANSITION_LOG_ENV} = \"<絶対パス>\"; cargo test -p areka transition_judge_offset_signoff -- --ignored --nocapture` \
             で実行する（手順は本モジュールの doc §4）"
        ));
    };
    let path = Path::new(raw_path);
    let log = fs::read_to_string(path).map_err(|error| {
        format!("{TRANSITION_LOG_ENV}={raw_path} を読めない: {error}（絶対パスで与えること）")
    })?;
    let report = judge_offset_log(&log);
    if report.rows == 0 {
        return Err(format!(
            "{TRANSITION_LOG_ENV}={raw_path} に追随レコードが 1 行も無い（{} 文字）。\
             観測 target と水準が有効になっていない採取を「発生 0 回」の根拠にしない（要件 8.5・doc §2）",
            log.len()
        ));
    }
    Ok((report, raw_path.to_owned()))
}

/// 実機ログの判定（既定では走らない）。
#[test]
#[ignore = "実機ログの判定（AREKA_TRANSITION_LOG に絶対パスを与えて明示実行する）"]
fn judges_a_real_machine_offset_log() {
    let raw_path = std::env::var(TRANSITION_LOG_ENV).ok();
    let (report, path) = match signoff_offset_log(raw_path.as_deref()) {
        Ok(judged) => judged,
        Err(message) => panic!("{message}"),
    };

    // 合否によらず判定結果の全文を出す（`--nocapture` で記録へ貼る）。
    println!("== balloon offset signoff: {path} ==");
    print!("{report}");

    assert!(
        !report.failed(),
        "実機ログが判定を満たさない（上の列挙が違反の全件・採り方の不備で出る種は doc §6.2）"
    );
}

// ---------------------------------------------------------------------------
// 入口そのものの檻（既定で走る）
// ---------------------------------------------------------------------------

#[test]
fn a_missing_environment_variable_is_an_error_not_an_empty_pass() {
    for raw_path in [None, Some(""), Some("   ")] {
        let error = signoff_offset_log(raw_path).expect_err("未設定は失敗でなければならない");
        assert!(
            error.contains(TRANSITION_LOG_ENV),
            "何を与えればよいか読めない: {error}"
        );
    }
}

#[test]
fn an_unreadable_path_is_an_error_not_an_empty_pass() {
    let temp = TempPath::new("offset-signoff-missing");
    let missing = temp.child("does-not-exist.log");
    assert!(!missing.exists(), "この檻は存在しないパスを前提にする");
    let error = signoff_offset_log(missing.to_str()).expect_err("読めないパスは失敗");
    assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
}

#[test]
fn a_directory_given_instead_of_a_log_is_an_error_not_an_empty_pass() {
    // 採取先のフォルダを渡す取り違えは実際に起こる。読めない理由が何であれ失敗にする。
    let temp = TempPath::new("offset-signoff-directory");
    let directory = temp.path();
    assert!(directory.is_dir());
    let error = signoff_offset_log(directory.to_str()).expect_err("フォルダは読めない");
    assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
}

#[test]
fn a_log_without_any_offset_record_is_an_error_not_an_empty_pass() {
    // 空のファイル・観測行を 1 行も含まないファイル・観測行は在るが追随レコードだけが
    // 消えているファイルの 3 つ。どれも「違反 0 件」を作れてしまう入力なので、合格ではなく
    // 失敗として落ちなければならない（要件 8.5）。
    let temp = TempPath::new("offset-signoff-empty");
    let path = temp.child("empty.log");
    let kind_offset = format!("{FIELD_KIND}={KIND_OFFSET}");
    let without_offset_rows = log_of(
        &pass_lines()
            .into_iter()
            .filter(|line| !line.contains(&kind_offset))
            .collect::<Vec<_>>(),
    );
    assert!(
        !without_offset_rows.is_empty(),
        "起点の行は残っているはず（そうでないと 3 つ目の入力が 2 つ目と同じになる）"
    );
    for body in ["", "何も観測していないログ\n", &without_offset_rows] {
        fs::write(&path, body).expect("一時ファイルを書けるはず");
        let error = signoff_offset_log(path.to_str()).expect_err("追随レコード 0 行は失敗");
        assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
    }
    fs::remove_file(&path).expect("一時ファイルを消せるはず");
}

#[test]
fn the_runner_reads_the_same_pure_function_as_the_deterministic_tests() {
    // 既知の合格ログを一時ファイルへ落として入口を一巡させ、判定が決定論テストと同じ結論を
    // 返すことを固定する（判定の実装が 2 つに分かれていないことの裏取り）。
    let temp = TempPath::new("offset-signoff-fixture");
    let path = temp.child("fixture.log");
    let log = log_of(&pass_lines());
    fs::write(&path, &log).expect("一時ファイルを書けるはず");

    let (report, echoed) = signoff_offset_log(path.to_str()).expect("既知の合格ログは判定できる");
    assert_eq!(echoed, path.to_str().expect("UTF-8 のはず"));
    assert_eq!(report, judge_offset_log(&log), "入口が別の判定を通っている");
    assert!(!report.failed(), "既知の合格ログが赤になった:\n{report}");
    assert_eq!(report.transitions.len(), 4, "{report}");

    fs::remove_file(&path).expect("一時ファイルを消せるはず");
}
