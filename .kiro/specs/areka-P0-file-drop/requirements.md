# Requirements Document

> 本文の実測は **2026-09-29・本ブランチ**（main `c3876110`＝`ghost-install` の完了 PR#198 のコミット）のもの。brief の起票時（09-28・`10a8d724`）から `ghost-install` が着地したので、依頼の型・許可表の数・記録の語を引き直した。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 要件 10 の裁定は、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。答えで作業が変わる議題は見込み 0（brief のとおり）。

## Project Description (Input)

**誰の何が困っているか**: `.nar` を手に入れた第三者は、エクスプローラからゴーストの上へ落として入れようとするが、`ghost-install` が着地してもメニューと台本からしか入れられず、落としても何も起きない。ゴーストの作者が辞書に書いた `OnFileDrop2`・`OnDirectoryDrop` の返事は areka では 1 度も動かない。

**今の状態**: ドラッグ＆ドロップの受け口が 0（`WM_DROPFILES` の腕・受け入れの宣言 `WS_EX_ACCEPTFILES`・落とされた物の振り分けのどれも本番に無い）。`OnFileDrop2`・`OnDirectoryDrop` は kanade の許可表に 0 語。先進坑 `pilot-dropfiles-on-wuc-window` は go（`WM_DROPFILES` で届く・宣言は 1 ビット・絵の外は OS が背後へ渡す）。`ghost-install` は依頼に書庫を 1 本以上並べられる口を 1 つ持って着地した。

**何を変えるか**: キャラクター窓・バルーン窓の絵の上へ落とされた物を、インストール対象（`install.txt` を持つ `.nar`／`.zip`）は `ghost-install` の手続きへ 1 つの依頼で渡し、インストール対象でないファイルは `OnFileDrop2`、フォルダは `OnDirectoryDrop` で今のゴーストへ知らせる。順は「インストール対象でないファイル → フォルダ → インストール対象」。新しいイベント名は作らず、受け取った 1 件ごとに記録を残す。

> 起票: 2026-09-28 `/kiro-discovery`（`ghost-install` の要件ディスカッション議題 1・開発者確定「分けてよい」）。先進坑 `pilot-dropfiles-on-wuc-window` は 2026-09-26 に開発者判定 go（一次記録は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md` の「検証結果」）。

## Introduction

### 誰が困っているか

- **`.nar` を手に入れた第三者**: 正典は「install.txtが適切に用意されていれば、D&Dなどの手段でインストーラ機能が働き、自動的にインストールできる」と書く。伺かの利用者が最初に試すのはゴーストの上へ落とすことだが、areka では落としても何も起きない。
- **ゴーストの作者**: ファイルを渡されたときの台詞や遊び（`OnFileDrop2`）・フォルダを渡されたときの台詞（`OnDirectoryDrop`）が areka では 1 度も動かない。

### いま何が起きているか（2026-09-29 実測）

- **受け口が 0。** `WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`IDropTarget`／`WS_EX_ACCEPTFILES`／`DragFinish` は `crates/**/*.rs` のうち先進坑 `crates/pilot/examples/pilot-dropfiles-on-wuc-window/` の 2 ファイルにしか無く、本番は 0 件。
- **窓手続きの振り分け表に腕が無い。** `crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message` は `WM_ERASEBKGND`〜`WM_CAPTURECHANGED` の 30 腕を持ち、`WM_DROPFILES` は 0 本（表に無いものは既定処理へ）。窓に関数を差す部品の前例は `crates/wintf/src/ecs/window/components.rs` の `OnCloseRequest`・`OnSessionEnd`（どちらも `fn(&mut World, Entity)`）で、areka 側の装着は `crates/areka/src/app_exit.rs` の `attach_os_close_request`（`GhostWindowMarker` を持つ窓に、窓の handle が付いたとき 2 部品を差す・`ghost_session::register_systems` が登録）。
- **受け入れの宣言が無い。** ゴースト窓の様式 `crates/areka/src/placement/spawn.rs` の `window_style` は `WS_POPUP | WS_VISIBLE`／`WS_EX_LAYERED | WS_EX_TOOLWINDOW`（キャラクター窓とバルーン窓で共通）。窓のスコープ番号は `CharWindowMarker { scope }`・`BalloonWindowMarker { scope }` から読める（`app_exit::on_ghost_os_close` が同じ読み方をしている）。
- **`ghost-install` の口は在る。** `crates/areka/src/install/mod.rs` の `InstallOrder { archives: Vec<PathBuf>, origin: InstallOrigin }`・`InstallOrigin { Menu, Script }`・`submit(world, order) -> SubmitVerdict { Queued, Empty, NoDesk, Closing }`（UI スレッド・依頼を手続きへ渡す唯一の口）。受けたら `info!(install_order_queued)`、断ったら `warn!(install_order_refused)`。手続きは依頼を届いた順に 1 本ずつ扱い、ゴーストが定常でなければ定常まで待たせ、書庫 2 本以上が全部成功したときだけ `OnInstallCompleteAll` を送る（`install/procedure.rs` の `run_order`）。完了 `ghost-install` の設計は「`file-drop` は `InstallOrigin` に自分の出どころを 1 つ足す」を再検証の引き金に挙げている。
- **イベントの送り道は在る。** `KanadeMsg::RaiseEvent { id, references, method, reply }`（`crates/areka-kanade/src/msg.rs`・`ghost-install` で `reply` が増えた）→ `schedule/change.rs` の `on_raise_event`。許可表 `ALLOWED_EVENT_IDS`（`schedule/events.rs`）に無い名前は `warn!(event = "raise_event_not_allowed")`、定常以外は `warn!(event = "raise_event_not_steady")` で捨てる（積まない）。許可表は **21 語**（元の 13＋インストール系 8）で、`events_change_tests.rs` が `assert_eq!(ALLOWED_EVENT_IDS.len(), 21)` と直書きで判定している。`OnFileDrop2`・`OnDirectoryDrop`・`OnFileDrop`・`OnFileDropping`・`OnFileDropEx`・`OnFileDropped` は `crates/` に 0 件。
- **MIME の表は無い。** `mime`／`MIME`／`application/` は `crates/` の `.rs`・`.toml` に 0 件。
- **網羅台帳。** `doc/ukadoc-coverage/ledger/shiori.toml` の `OnFileDrop2`（`introduced = "2.7.98"`）・`OnDirectoryDrop` は `status = "absent"`・`owner = ""`・`priority = "B4"`。`OnFileDrop`・`OnFileDropEx`・`OnFileDropped` は `status = "alias"`（`alias_of` は `OnFileDrop2`）。`OnFileDropping` は別扱い。報告書は `cargo run -p ukadoc-survey -- report`／`-- report-summary` が作り直す。`roadmap-draft.md` は人が書き、`[briefs].count` は表の行数を機械が見張る（`cargo test -p ukadoc-survey`）。
- **同期送信の見張り。** `crates/areka/src/session_end_sync_send_tests.rs` の `ALLOWED_SYNC_SENDS` は本番の `SendMessageW(`／`SendMessageTimeoutW(` を 2 件の例外表で固定している。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [インストール](https://ssp.shillest.net/ukadoc/manual/manual_install.html) | 「narはzipの拡張子を変えただけのもので、実体はzipなので、サーバによってはzipで配布する場合もある。」「install.txtが適切に用意されていれば、D&Dなどの手段でインストーラ機能が働き、自動的にインストールできる。」「narはinstall.txtを含んだゴーストフォルダをベースウェア（SSP）本体にD&Dすることによって作成可能。」 | 書庫と見なす拡張子は `.nar` と `.zip`。フォルダから `.nar` を作る機能は範囲外（フォルダは `OnDirectoryDrop`）。 |
| [`OnFileDrop2`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFileDrop2:1) | 「ファイルがDnDされた際に発生。このイベントが現時点での最新仕様となる。」Reference0「ドロップされたファイルパス。複数ファイル/ディレクトリがあればbyte値1で区切る。」Reference1「ドロップされたキャラクターのスコープ番号。本体側0、相方1、3人目以降は2以降。」Reference2「ドロップされたファイルのMIMEタイプ(SSP 2.7.98以降)。複数ファイル/ディレクトリがあればbyte値1で区切る。」 | インストール対象でないファイルの投げ込みを 1 回で知らせる。Reference は 3 つ。 |
| [`OnDirectoryDrop`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnDirectoryDrop:1) | 「ディレクトリがDnD された際に発生。」Reference0「ドロップされたディレクトリのパス。」Reference1「ドロップされたキャラクターのスコープ番号。本体側0、相方1、3人目以降は2以降。」 | Reference0 はパス 1 つ＝フォルダ 1 つにつき 1 回。 |
| [`OnFileDrop`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFileDrop:1)／[`OnFileDropEx`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFileDropEx:1)／[`OnFileDropped`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFileDropped:1) | いずれも「[旧仕様]ファイルがDnDされた際に発生。」 | 送らない（0 件）。 |
| [`OnFileDropping`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFileDropping:1) | 「ファイルをドラッグしたままのカーソルがゴースト上に乗った際に発生。」 | ドラッグ中の知らせは本仕様の受け方では取れない。送らない（0 件）。 |
| [`OnArchiveViewerOpen`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnArchiveViewerOpen:1) ほか | 「OnFileDrop2にゴーストが応答せず、かつアーカイブビューアーが開かれた際に発生。」 | areka にビューアは無い。応えが無くても何も開かず、送らない（0 件）。 |
| [`OnTextDrop`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnTextDrop:1)／[`OnURLDropping`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnURLDropping:1) ほか／[`OnOtherObjectDropping`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnOtherObjectDropping:1) ほか | テキスト・URL・「ファイル」として扱えない物の投げ込み。 | 本仕様の受け方（ファイルのパスの一覧だけが届く）では取れない。範囲外（α 後・URL は `network-update` の Out）。 |

正典が沈黙している点（インストール対象とインストール対象でない物が混ざったときの順・複数のフォルダの知らせ方・MIME の決め方・ゴーストが定常でないときの扱い・`install.txt` を持つフォルダの扱い）は areka の裁量として要件 10 で決める。SSP の挙動を測って合わせることはしない。

### brief の記述を実物で引き直して改めた点

1. brief の「`ghost-install` が 13 語 → 21 語にした後（予定）」は着地済み。許可表は 21 語で、本仕様が 2 語足して **23 語**にする（`events_change_tests.rs` の判定を 21 から 23 へ）。
2. brief の「`KanadeMsg::RaiseEvent { id, references, method }`」は、`ghost-install` で応えを受ける欄 `reply` が増えた。本仕様は応えを使わない（欄の中身は設計で決める）。
3. brief の「`roadmap-draft.md` の数は生成器で作り直す」は、`roadmap-draft.md` が人の手で書く文書で、機械が見張るのは `[briefs].count` と表の行数の一致である。生成器で作り直すのは報告書（`report/`）。
4. brief の「`ghost-install` の依頼の型」は `InstallOrder`（書庫のパスを 1 本以上）と `submit` の口 1 つとして着地し、出どころの語 `InstallOrigin` は `Menu`・`Script` の 2 つ。本仕様は出どころを 1 つ足す（完了 `ghost-install` の設計の再検証の引き金どおり）。

## Boundary Context

- **In scope**:
  - キャラクター窓・バルーン窓が、絵の上へ落とされたファイル・フォルダの一覧を受け取ること（受け入れの宣言と受け口）。
  - 落とされた物を「インストール対象・インストール対象でないファイル・フォルダ」に分けること。
  - インストール対象を `ghost-install` の手続きへ 1 つの依頼で渡すこと。
  - インストール対象でないファイルの `OnFileDrop2`（MIME を含む）・フォルダの `OnDirectoryDrop` の送出と、許可表への 2 語の追加（21 → 23）。
  - 受け取りと振り分けの記録。
  - 網羅台帳 2 行・報告書・`roadmap-draft.md`。
  - 決定論テストと実機確認（`signoff.md`）。
- **Out of scope**:
  - インストールの手続きそのもの（`accept` の照合・利用条件・展開・`OnInstall*` の送出・`OnInstallCompleteAll` の判断・`lastinstalled`・入れた後の切替）＝完了 `ghost-install`。本仕様が書庫について見るのは目次（`install.txt` が在るか）だけで、展開も中身の解釈もしない。
  - ドラッグ中の知らせ `OnFileDropping`・旧仕様 `OnFileDrop`／`OnFileDropEx`／`OnFileDropped`・応えが無いときのビューア（`OnArchiveViewerOpen` など）。
  - 汎用の書庫の解凍機・ビューアとしての働き（`OnArchiveViewerOpen` などの先で SSP が開く道具。areka はデスクトップマスコットの枠を超えない＝恒久に範囲外・裁定 10.7）。メニューと台本からのインストール（完了 `ghost-install`）で `install.txt` の無い書庫を `OnInstallFailure` とする扱いは変えない。
  - テキスト・URL・「ファイル」として扱えない物の投げ込み（`OnTextDrop`／`OnURLDropping`／`OnURLDropped`／`OnURLDropFailure`／`OnOtherObjectDropping`／`OnOtherObjectDropped`）＝α 後。URL は `network-update` の Out にも明記。
  - 管理者として起動した areka へ、ふつうの権限のエクスプローラから落としたときの手当て（OS の仕組みで窓に届かない＝既知の制限として `alpha-release-signoff` へ申し送る）。
  - フォルダから `.nar` を作ること（`OnNarCreating`／`OnNarCreated`）。
  - `dist/README.txt` の「■ .nar の入れ方」の本文（`alpha-release-signoff`）。
  - ゴースト窓以外の窓（wintf の example の窓など）に受け入れの宣言を付けること（0 枚）。
- **Adjacent expectations**:
  - 完了 `ghost-install` の保証に依る: 依頼を渡す口は 1 つ・依頼は届いた順に 1 本ずつ・ゴーストが定常でなければ定常まで待たせる・終了が始まっていれば断って `warn!`・書庫 2 本以上が全部成功したときだけ `OnInstallCompleteAll`・areka は入れた後の切替を主導しない（裁定 5）。本仕様はこれらを繰り返し実装しない。
  - 完了 `pilot-dropfiles-on-wuc-window` の結果に依る: 受け入れの宣言はビット 1 つで足りる・絵の外（透けている所）に落とした物は OS が背後の窓へ渡すので窓に届かない・透過の付け外しを繰り返しても宣言は残る。
  - 完了 `ghost-shell-balloon-switch` の汎用の通知の入口に依る: 許可表に無い名前と定常以外の依頼は `warn!` の上で捨てる・応答は再生中の台詞を置き換える。
  - 後続 `shell-balloon-switch` は許可表の数を本仕様の 23 から動かす。`alpha-release-signoff` は第三者の手順「`.nar` を窓へ落とす」を本仕様の着地の上で行う。
  - 完了 spec の文書は書き換えない。

## Requirements

### Requirement 1: ゴーストの窓が落とされた物を受け取る

**Objective:** As a `.nar` を手に入れた第三者, I want エクスプローラからゴーストの絵の上へ落とせること, so that 伺かで覚えたとおりの操作で渡せる

#### Acceptance Criteria

1. The areka shall キャラクター窓とバルーン窓の両方を、ファイルとフォルダを落とせる窓として建てる（宣言はどちらの窓も同じ・スコープ番号を問わない）。
2. When ゴーストを起こし直した後（切替・上書きの起こし直し）に窓を建て直す, the areka shall 建て直した窓も同じく落とせる窓にする（起こし直しの回数によらない）。
3. The areka shall ゴースト窓以外の窓（areka が建てる他の窓・wintf の example の窓）を落とせる窓にしない（宣言を足す窓 0 枚）。
4. When 利用者が絵の上（透けていない所）で落とす, the areka shall 落とされた物の絶対パスの一覧と、落とされた窓のスコープ番号（キャラクター窓ならその番号・バルーン窓ならそのバルーンが属するキャラクターの番号）を受け取る。
5. When 利用者が絵の外（透けている所）で落とす, the areka shall 何もしない（受け取りの記録 0 件・イベント 0 件・依頼 0 件。落とした物は OS が背後の窓へ渡す）。
6. When 落とされた物を 1 回受け取る, the areka shall 記録を 1 件残し、件数・窓のスコープ番号・種類の内訳（インストール対象・インストール対象でないファイル・フォルダのそれぞれの数。0 も書く）を載せる。
7. If 落とされた物の一覧を OS から受け取り損ねる, then the areka shall `warn!` を 1 件残し、イベントも依頼も出さない（0 件）。OS から借りた資源は必ず返す。
8. If 受け取った一覧が 0 件である, then the areka shall 記録を 1 件残し、イベントも依頼も出さない（0 件）。
9. The areka shall 落とされた物を受け取る間も、ゴーストの描画・台詞の再生・メニュー・クリック透過の付け外しを止めない（振り分けで書庫について読むのは目次だけで、展開しない）。
10. The areka shall 透過の付け外しが何度起きても、落とせる窓のままにする（先進坑で 36 回の付け外しの後も宣言が残ったことを、決定論テストで固定する）。

### Requirement 2: 落とされた物を 3 つに分ける

**Objective:** As a ゴーストの作者と `.nar` を落とす利用者, I want 落とした物が「入れる物」「渡す物」「フォルダ」に迷いなく分かれること, so that `.nar` は入り、それ以外はゴーストの台詞になる

#### Acceptance Criteria

1. The areka shall 落とされた物 1 つを、次の順で 1 つの種類に決める: ⑴ フォルダなら「フォルダ」⑵ 拡張子が `.nar` または `.zip`（大文字小文字を区別しない・`.NAR`・`.Zip` も同じ）で、書庫の目次が読めて最上位に `install.txt` が在るなら「インストール対象」⑶ それ以外（`install.txt` の無い書庫・目次の読めない書庫を含む）は「インストール対象でないファイル」。`.nar` と `.zip` は同じ規則で扱う（拡張子で分けない）。
2. The areka shall 種類の判定に、フォルダかどうか・拡張子・（`.nar`／`.zip` だけ）目次に `install.txt` が在るか、の 3 つ以外の情報（`install.txt` の中身・他のエントリの中身・ファイルの大きさ）を使わない。`install.txt` を探す規則は `ghost-install` の手続きが書庫を開くときの規則と同じにする（振り分けと手続きで判定が食い違わない）。
3. When `install.txt` を持つフォルダが落とされる, the areka shall フォルダとして扱う（`.nar` を作らない・インストールの手続きへ渡さない）。
4. When `install.txt` を持たない `.nar`／`.zip`、または目次の読めない `.nar`／`.zip` が落とされる, the areka shall インストール対象でないファイルとして `OnFileDrop2` で知らせる（インストールの手続きへ渡さない・`OnInstallFailure` は出ない）。目次が読めなかったときは、その理由を `warn!` に 1 件残す。
5. If フォルダかどうかを OS に問い合わせられない（落とした直後に無くなった等）, then the areka shall フォルダでないものとして要件 2.1 の ⑵ 以降で決め（無くなった書庫は目次が読めないのでインストール対象でないファイルになる）、問い合わせに失敗したことを `warn!` に 1 件残す。
6. The areka shall 一覧の中の並び（落とされた順）を、種類ごとに分けた後も保つ。
7. When インストール対象として渡した書庫が、手続きの中で入らない（`accept` の不一致・利用条件で断られた・展開の失敗など）, the areka shall その知らせを `ghost-install` の手続きに任せる（`OnInstallFailure`／`OnInstallRefuse`。本仕様は `OnFileDrop2` へ回し直さない）。「インストール対象だが入らない」は失敗であり、投げ込みの知らせではない。

### Requirement 3: インストール対象は `ghost-install` の手続きへ 1 つの依頼で渡す

**Objective:** As a `.nar` を手に入れた第三者, I want 落とした `.nar` がメニューから入れたときと同じ手順で入ること, so that 入れ方によって結果が変わらない

#### Acceptance Criteria

1. When 受け取った一覧にインストール対象が 1 本以上ある, the areka shall そのインストール対象の全部を落とされた順に並べた 1 つの依頼を、`ghost-install` の依頼を渡す唯一の口へ渡す（1 回の投げ込みにつき依頼 1 つ・インストール対象 1 本ごとに分けない）。
2. The areka shall 渡した後の手順（`OnInstallBegin` から締めの知らせまで・`accept` の照合・利用条件・待ち行列・定常まで待つこと・`OnInstallCompleteAll` の判断）を `ghost-install` の手続きに任せ、本仕様で重ねて作らない（インストールの手順 0 本）。
3. The areka shall 依頼の出どころを、記録で「窓への投げ込み」と分かる語にする（メニュー・台本と区別できる・手続きは出どころで分岐しない）。
4. When 手続きが別の依頼を扱っている最中にインストール対象が落とされる, the areka shall 依頼を待ち行列へ積み（捨てない）、先の依頼が終わってから扱わせる。
5. When 終了が指示された後にインストール対象が落とされる, the areka shall そのインストール対象を入れず（展開 0 件）、入れなかったことを記録に 1 件残す（本仕様で足す知らせ・画面 0 件）。依頼を渡す前に止めるか、渡して手続きの側で捨てさせるかは設計で決める（今の `submit` は、終了が指示されてから実行の輪が返るまでの間は断らず積み、後で `discard_for_exit` が捨てる）。
6. When 落とされたインストール対象のインストールが終わる, the areka shall 表示中のゴーストをそのまま残す（切替の要求 0 件・完了 `ghost-install` 裁定 5 のまま）。
7. When 複数の `.nar` を一度に落とし、その全部が入る, the areka shall 最後に `OnInstallCompleteAll` が 1 回届く状態にする（送るのは `ghost-install` の手続き・本仕様は依頼を 1 つにまとめるだけ）。
8. The areka shall インストール対象の依頼をゴーストが定常でないときも渡す（手続きの待ち行列が定常まで待たせる・本仕様は捨てない）。

### Requirement 4: インストール対象でないファイルは `OnFileDrop2` で知らせる

**Objective:** As a ゴーストの作者, I want 辞書に書いた `OnFileDrop2` の返事が正典どおりの Reference で届くこと, so that ファイルを渡されたときの台詞や遊びが areka でも動く

#### Acceptance Criteria

1. When 受け取った一覧にインストール対象でないファイルが 1 つ以上ある, the areka shall 今のゴーストへ `OnFileDrop2` を GET で **1 回だけ**送る（ファイルの数によらず 1 回）。
2. The areka shall `OnFileDrop2` の Reference0 を、インストール対象でないファイルの絶対パスを落とされた順に byte 値 1 で区切って並べたものにする（フォルダとインストール対象は含めない・0 件なら送らない）。
3. The areka shall Reference1 を、落とされた窓のスコープ番号（本体 0・相方 1…）にする。
4. The areka shall Reference2 を、Reference0 の各ファイルと同じ並び・同じ数で、拡張子から決めた MIME タイプを byte 値 1 で区切って並べたものにする。決められない拡張子と拡張子の無いファイルは空（区切りは残す＝Reference0 と Reference2 の要素数は常に同じ）。
5. The areka shall 拡張子と MIME タイプの対応表を持ち、拡張子の大文字小文字を区別しない。表に載せる拡張子の範囲は設計で決めるが、`.nar` と `.zip` はどちらも `application/zip` とする（正典「実体はzip」・`install.txt` の無い書庫が `OnFileDrop2` に載るため）（表の全項目と、表に無い拡張子・拡張子なしを決定論テストで判定する）。
6. The areka shall Reference3 以降を送らない（Reference は 3 つ）。
7. When ゴーストが `OnFileDrop2` に台本を返す, the areka shall 汎用の通知の入口の規則どおり、その台本で再生中の台詞を置き換える。
8. When ゴーストが `OnFileDrop2` に応えない, the areka shall 何も開かず何も出さない（ビューア 0・画面 0・代わりのイベント 0）。応えが無かったことを送り手が知る必要は無い。

### Requirement 5: フォルダは `OnDirectoryDrop` で知らせる

**Objective:** As a ゴーストの作者, I want フォルダを渡されたときの返事が正典どおりに届くこと, so that フォルダを使う辞書が areka でも動く

#### Acceptance Criteria

1. When 受け取った一覧にフォルダが 1 つ以上ある, the areka shall フォルダ 1 つにつき `OnDirectoryDrop` を GET で 1 回、落とされた順に送る（Reference0 はパス 1 つ・区切って並べない）。
2. The areka shall `OnDirectoryDrop` の Reference0 をそのフォルダの絶対パス、Reference1 を落とされた窓のスコープ番号にする（Reference は 2 つ・Reference2 以降 0 個）。
3. When フォルダが 2 つ以上ある, the areka shall 後のフォルダへの返事が前の返事の台詞を置き換えることを承知の上で、それでも 1 つずつ送る（正典の Reference0 がパス 1 つであるため・裁定 10.2）。

### Requirement 6: 混ざった投げ込みの順と、ゴーストが定常でないとき

**Objective:** As a 利用者, I want ファイル・フォルダ・インストール対象を一度に落としても、知らせが今のゴーストへ先に届き、そのあとインストール対象が入ること, so that インストール対象の手続きでゴーストが替わっても、落とした物の知らせが行方不明にならない

#### Acceptance Criteria

1. When 受け取った一覧に 2 種類以上が混ざっている, the areka shall 「インストール対象でないファイル（`OnFileDrop2` 1 回）→ フォルダ（`OnDirectoryDrop` を 1 つずつ）→ インストール対象（依頼 1 つ）」の順に扱う。
2. The areka shall インストール対象の依頼を、`OnFileDrop2`・`OnDirectoryDrop` を送った後に渡す（インストール対象の手続きは時間がかかり、終わるとゴーストが替わりうるため、知らせは今のゴーストへ先に届ける）。
3. While ゴーストが定常でない（起動の途中・切替の途中・上書きの起こし直しの途中）, when インストール対象でないファイルまたはフォルダが落とされる, the areka shall そのイベントを送らず（後で送り直さない）、`warn!` を 1 件残す（汎用の通知の入口が定常以外を捨てるのと同じ扱い）。インストール対象は要件 3.8 のとおり渡す。
4. While 終了が指示された後, when 何かが落とされる, the areka shall イベントを送らず（0 件）、インストール対象は要件 3.5 のとおり入れず、記録が残ること以外に何も起こさない（本仕様で足す経路 0）。
5. The areka shall 1 回の投げ込みの扱い（振り分け・送出・依頼）を、受け取ったその巡の中で終える（次の投げ込みを待たない・順を入れ替えない）。

### Requirement 7: 正典の語だけを送る

**Objective:** As a ゴーストの作者, I want areka が正典に無いイベント名を作らないこと, so that 辞書を areka 向けに書き分けずに済む

#### Acceptance Criteria

1. The areka shall 本仕様で新しいイベント名を作らない（0 個）。送るのは正典の `OnFileDrop2`・`OnDirectoryDrop` の 2 語だけ。
2. The areka shall `OnFileDropping`・`OnFileDrop`・`OnFileDropEx`・`OnFileDropped`・`OnArchiveViewerOpen`・`OnMediaPlayerOpen`・`OnPictureViewerOpen`・`OnTextDrop`・`OnURLDropping`・`OnURLDropped`・`OnURLDropFailure`・`OnOtherObjectDropping`・`OnOtherObjectDropped`・`OnNarCreating`・`OnNarCreated` を送らない（0 件）。
3. The areka shall kanade の許可表を 21 語から **23 語**にする（足すのは `OnFileDrop2`・`OnDirectoryDrop` の 2 語・正典の URL を添える）。
4. The areka shall `OnFileDrop2`・`OnDirectoryDrop` を汎用の通知の入口から送る（イベント専用の送り道を新設しない・0 本）。

### Requirement 8: 記録・台帳・文書が実物と揃う

**Objective:** As a 後続の spec の開発者, I want 落とされた物の行方が記録から追え、台帳と文書が実物と食い違わないこと, so that 次の作業と利用者の手順が誤った前提に立たない

#### Acceptance Criteria

1. The areka shall 受け取り（要件 1.6）・振り分けの結果・送ったイベントの名前と Reference の要約・渡した依頼のインストール対象の数を記録に残し、記録の無い失敗の経路を作らない（0 本）。
2. The areka shall 失敗をメッセージボックスで伝えない（0 個）。本仕様が出す画面は 0 個。
3. The 本仕様 shall 網羅台帳 `shiori.toml` の `OnFileDrop2`・`OnDirectoryDrop` の 2 行を実装済み（`owner = "areka-P0-file-drop"`）へ動かし、報告書は生成器で作り直す（手で数を直さない）。`OnFileDropping` と旧仕様 3 語（`alias`）は動かさない。
4. The 本仕様 shall `roadmap-draft.md` に本仕様の行を足して `[briefs].count` を表の行数（37 → 38）に合わせ、束の表の「投げ込み」の行（候補名の案 `areka-P0-file-drop-events`・依存する既存 spec「0 本」）を着地後の台帳の `owner` に合わせて直し、`cargo test -p ukadoc-survey` で判定させる。
5. The 本仕様 shall 本番コードが読む環境変数を足さず（0 個）、外部クレートを足さない（0 個。既に依存している `windows` クレートの機能を足すことは数えない）。
6. The 本仕様 shall 本番コードに同期送信（`SendMessageW(`／`SendMessageTimeoutW(`）を足さない（既存の見張りの例外表を増やさない・0 件）。
7. The 本仕様 shall 本番ファイルとテストファイルのどれも 1,000 行を超えさせない。
8. The 本仕様 shall 送るイベントの定義の場所に正典の URL の行（`// ukadoc:`）を置く。
9. The 本仕様 shall 管理者として起動した areka へふつうの権限から落とせないことを、既知の制限として `alpha-release-signoff` へ申し送る（本仕様の手当て 0）。

### Requirement 9: 決定論テストと実機確認

**Objective:** As a 開発者, I want 判断の分かれ目がテストで固定され、実機で一周が見えること, so that 後続の変更で黙って崩れない

#### Acceptance Criteria

1. The 本仕様 shall 窓手続きの振り分けを、偽の落とし物の知らせで判定する: 受け口を持つ窓ではパスの一覧と窓の識別で受け口が 1 回呼ばれる／受け口を持たない窓では既定処理へ進む／一覧を受け取り損ねたときは `warn!` 1 件で受け口を呼ばない。
2. The 本仕様 shall ゴースト窓の様式に受け入れの宣言が入ること・キャラクター窓とバルーン窓の両方に受け口が差されること・起こし直した窓にも差されること・ゴースト窓以外に差されないことを判定する。
3. The 本仕様 shall 振り分けの規則（要件 2.1〜2.7）を判定する: フォルダ優先・`.nar`／`.zip`／大文字小文字・`install.txt` を持つ `.nar` と `.zip`（どちらもインストール対象）・`install.txt` の無い `.nar` と `.zip`（どちらもインストール対象でないファイル）・目次の読めない書庫・`install.txt` を持つフォルダ・拡張子なし・問い合わせ失敗・順の保持・振り分けと手続きの `install.txt` の探し方が同じであること。
4. The 本仕様 shall `OnFileDrop2` の Reference 3 つ（複数ファイルの区切り・スコープ番号・MIME の並びと要素数の一致・空の MIME）と、`OnDirectoryDrop` の Reference 2 つとフォルダごとの回数を判定する。
5. The 本仕様 shall 混ざった投げ込みの順（ファイル → フォルダ → インストール対象）と、インストール対象の依頼が 1 つにまとまり落とされた順を保つこと・依頼の出どころの語を判定する。
6. The 本仕様 shall 定常でないときに `OnFileDrop2`・`OnDirectoryDrop` が送られず `warn!` が 1 件残ること、インストール対象の依頼は渡されることを判定する。
7. The 本仕様 shall 許可表が 23 語であることを判定する（既存の判定の数 21 を書き換える）。
8. The 本仕様 shall MIME の対応表の全項目と、表に無い拡張子・拡張子なし・大文字の拡張子を判定する（値を印字するだけにしない）。
9. The 本仕様 shall 記録の捕捉を `log-capture-kit` で行い、既存のテストを置き換え無しに消さない。
10. When 実機で確認する, the 開発者 shall 手で落とす前に「何が起きないのが正しいか」（絵の外・透けた余白の位置・背後の窓に落ちること）を決めて `signoff.md` に書き、記録の水準をイベントの送出と振り分けの分かれ目が見える所まで開ける。
11. When 実機で確認する, the 開発者 shall 次を見て `signoff.md` に記録する: ⑴ ゴーストの `.nar` を絵の上へ落とす → 入る → 表示中のゴーストのまま ⑵ バルーン窓へ落としても同じ ⑶ インストール対象でないファイルを落とす → `OnFileDrop2`（記録で Reference 3 つを確かめる）⑷ フォルダを落とす → `OnDirectoryDrop` ⑸ 絵の外へ落とす → areka の記録に受け取りが出ない ⑹ 複数の `.nar` を一度に落とす → 最後に `OnInstallCompleteAll`。

### Requirement 10: 裁定（暫定・要件ディスカッションで確定）

**Objective:** As a 開発者, I want 正典が沈黙している点の読みが理由つきで並んでいること, so that 要件ディスカッションで覆すか決めるだけで済む

#### Acceptance Criteria

1. The 本仕様 shall **裁定 1: 混ざった投げ込みの順**を「インストール対象でないファイル → フォルダ → インストール対象」とする（要件 6.1・完了 `ghost-install` 暫定の確定 12 から引き継ぎ）。理由: インストール対象の手続きは長く、終わるとゴーストが替わりうる。知らせは今のゴーストへ先に届ける。
2. The 本仕様 shall **裁定 2: 複数のフォルダ**を「フォルダ 1 つにつき `OnDirectoryDrop` 1 回」とする（要件 5.1）。理由: 正典の Reference0 は「ディレクトリのパス」1 つで、区切って並べる書き方が無い。後の返事が前の台詞を置き換えるのは汎用の通知の入口の規則のままとし、まとめる書き方を areka が発明しない。
3. The 本仕様 shall **裁定 3: フォルダはインストール対象より先に見る**（要件 2.1）。理由: `foo.nar` という名前のフォルダはインストール対象ではない。`install.txt` を持つフォルダから `.nar` を作る機能（正典の「D&Dによって作成可能」）は範囲外なので、フォルダはすべて `OnDirectoryDrop`。
4. The 本仕様 shall **裁定 4: 定常でないときのインストール対象でない物**を「送らず `warn!` 1 件」とする（要件 6.3）。理由: 汎用の通知の入口が定常以外を捨てる規則と揃える。書庫は `ghost-install` の待ち行列が定常まで待たせるので捨てない（要件 3.8）。後で送り直すと、起動の挨拶の後に前の投げ込みの台詞が突然出る。
5. The 本仕様 shall **裁定 5: MIME**を「拡張子から決め、決められなければ空・表の広さは設計で決める」とする（要件 4.4・4.5）。理由: 正典は決め方を書かない。ファイルの中身を読むと受け取りが遅くなる。空にしても Reference の要素数を保てば辞書側は並びで突き合わせられる。
6. The 本仕様 shall **裁定 6: 問い合わせに失敗した物**を「拡張子だけで決め `warn!`」とする（要件 2.5）。理由: 一覧の 1 件を黙って落とすと記録の無い失敗になる。
7. The 本仕様 shall **裁定 7: 「インストール対象か」を先に、「投げ込みの知らせか」を後に決める**（要件 2.1・2.4・2.7・**2026-09-29 要件ディスカッション議題 1 で開発者確定**）。インストール対象は「`.nar`／`.zip` で目次の最上位に `install.txt` が在る物」だけ。`install.txt` の無い書庫・目次の読めない書庫は `.nar` も `.zip` も `OnFileDrop2` で知らせる。インストール対象だが入らない物は `OnInstallFailure`（手続き）。理由: 正典の `OnFileDrop2` の条件は「ファイルがDnDされた際に発生。」だけで、書庫を除く決まりは無い。正典は「narはzipの拡張子を変えただけ」と書くので、`.nar` と `.zip` は拡張子でなく中身（`install.txt`）で分ける。`.zip` から `OnFileDrop2` の経路を無くすと、辞書に書いた書庫への台詞が areka で動かない。完了 `ghost-install` 暫定の確定 10（`install.txt` の無い `.zip` を `OnInstallFailure`）はメニューと台本の入口の扱いとして残し、投げ込みの入口では本裁定が上書きする。応えが無いときに SSP が開くアーカイブビューア（[`OnArchiveViewerOpen`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnArchiveViewerOpen:1)）と汎用の解凍機の働きは取らない（areka はデスクトップマスコットの枠を超えない）。
8. The 本仕様 shall **裁定 8: 1 回の投げ込みのインストール対象は依頼 1 つ**とする（要件 3.1）。理由: `OnInstallCompleteAll` は「複数のnarをD&Dした時などに」全部入ったときに 1 回（正典）。依頼を分けると手続きは全部入ったことを知れない。
