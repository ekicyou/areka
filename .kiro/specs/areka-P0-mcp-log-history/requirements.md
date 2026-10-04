# Requirements Document

> 本文の実測は **2026-10-04・本ブランチ**（main `e2a373b5`＝棚卸㉑の PR#229 のコミット）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、brief が「要件で決める」とした議題と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。答えで作業が変わる議題は見込み 4 件（裁定 3〔network と update の分け方〕・裁定 5〔上限の数〕・裁定 7〔`<名>` の欄〕・裁定 8〔`[<種別>]` の欄〕）。
> **SSP の実測は [research.md](research.md) §2 にある**（2026-10-04・SSP 2.9.07）。要件を書いた時点では SSP が起動しておらず、下の表に「未測」と書いた点の多くは、その後のギャップ分析と追加の実測で測れた。実測と食い違う暫定の裁定は要件ディスカッションで改める。まだ未測なのは、update 種別の行の形・複数行の本文の継続行・もう起動していないゴーストの名前での絞り込み・履歴の上限である。
> 今のログの出口の書き手は既定のまま＝**標準出力**である（brief と初版の「標準エラー」は誤り）。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code など）でゴーストを作る人は、台本を流した後に「エラーが出たか」「何が再生されたか」をログで確かめる。SSP では `get_log` を呼び、`since_id` で前回より後の行だけを読む。areka のログは標準出力へ流れて消えるだけで、動いているアプリへ問い合わせて読めない。

**今の状態**: ログの出口は `crates/areka/src/main.rs` の `fn main()` の先頭の `tracing_subscriber::fmt().with_env_filter(…).init()` の 1 か所だけで、メモリ上の履歴も通し番号も無い。`get_log` は `mcp-tool-entrances` が入口（定義・引数の型の検査・アプリ本体への橋）まで作り、アプリ本体側 `crates/areka/src/mcp/get_log.rs` の `handle` は `NG:not implemented yet` を返すダミーである。

**何を変えるか**: アプリのログの出来事のうち 5 種別（error・script・network・update・status）に当たるものが、通し番号付きでメモリに残る（上限があり、古いものから捨てる）。`get_log` がそれを SSP と同じ書式で返し、`log_type`・`ghost_name`・`since_id`・`max_count` で絞れる。どの出来事がどの種別になるかの規則を決め、他の spec（`mcp-kanade-tools`・`mcp-strict-errors`）が従う取り決めとして文書にする。

> 起票: 2026-09-29 `/kiro-discovery`（SSP MCP 移植の 3 段目の 1 本）。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 の実測）、ツール定義の逐語は [doc/ssp-mcp/tools-list-ssp-2.9.05.json](../../../doc/ssp-mcp/tools-list-ssp-2.9.05.json)。全体の並びは `.kiro/steering/roadmap.md`「SSP MCP の移植」節（本 spec は **C3-⑨**）。入口の約束は完了 spec `mcp-tool-entrances` の要件 3.7・5。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人**: 台本やイベントを送った後、何が起きたかを確かめる手段が無い。標準出力の出力は、窓なしで起動した areka では見えない。
- **後続の spec の実装者**: `mcp-kanade-tools` は「再生した台本」を script 種別へ、`mcp-strict-errors` は「strict の台本の誤り」を error 種別へ残したい。残す先と、残すための約束（どう出せばどの種別になるか）がまだ無い。

### いま何が起きているか（2026-10-04 実測）

- **ログの出口。** `crates/areka/src/main.rs` の `fn main()` の先頭で `tracing_subscriber::fmt()` に `EnvFilter`（`RUST_LOG`・未設定なら `info`）を付けて `init()` する。出口はこれだけである。`main.rs` は 957 行（上限 1,000）。
- **`get_log` の入口。** プロトコル側 `crates/areka-mcp/src/tools/get_log.rs` の `Args` は `log_type: Option<String>`・`ghost_name: Option<String>`・`since_id: Option<i64>`・`max_count: Option<i64>`。省略と `null` は `None`、`ghost_name` の空の文字列は空のまま届く（`mcp-tool-entrances` の design）。`ghost_name` は解決されずに届く（同 要件 3.7）。アプリ本体側 `crates/areka/src/mcp/get_log.rs` の `handle(_world, _args, reply)` は `NG:not implemented yet` を返し、`get_log_tests.rs` がその文言を期待している（本 spec が書き換える）。
- **出す側の行の今の形。** 更新は `crates/areka-update/src/lib.rs`（メッセージの頭が `[areka_update]`）と `crates/areka/src/update/`（`event = "update_…"`・頭が `[update]`）、インストールは `crates/areka/src/install/`（`event = "install_…"`・頭が `[install]`。URL からの取得は `install/fetch_url.rs` の `install_fetch_begin`・`install_fetch_done` など）、起動・切替の節目は `crates/areka/src/main.rs`・`boot_resolve.rs`・`ghost_session.rs`・`emo2_boot/ghost_switch.rs`（`event = "ghost_switch_…"`・`session_mark_…` など）にある。これらの行は tracing の target を明示しておらず、target はモジュールのパス（`areka_update`・`areka::update::…`・`areka::install::…`・`areka::ghost_session` など）になる。**ゴーストの名前を載せる共通の欄は無い**（`folder`・`homeurl` などの欄が行ごとにある）。
- **target を明示している行もある。** kanade は `target: "kanade"`・`"areka_kanade::resource"`、アトラスは `"areka_emo_atlas"`。本 spec が決める取り決めの名前はこれらと重ならないようにする。
- **依存。** `crates/areka/Cargo.toml` に `tracing`・`tracing-subscriber`（ワークスペースの `version = "0.3", features = ["env-filter"]`）がある。出口を重ねる機能は既定機能に含まれるので、依存は足さずに済む見込み（設計で確かめる）。
- **テストの見張り。** `crates/log-capture-kit/tests/with_default_guard_test.rs` が、ログの捕捉先を直接差す呼び出しの新設をワークスペース全体で見張っている（`.kiro/steering/logging.md`）。出口の組み替えとテストはこの見張りに当たらない書き方にする。

### brief の記述を実物で引き直して改めた点

1. brief の Scope の「network／update／status の出す側（既存の行に target を付ける）」は行わない。同じウェーブ（C3）の約束（brief の 10-04 の追記）のとおり、**出す側の行は 1 行も触らず、既存のモジュールのパスで振り分ける**。`crates/areka/src/install/`・`update/`・`crates/areka-update/` は他の spec（`install-companion-reading`・`network-update-canon-order` など）が触るためである。
2. brief は「error＝warn 以上」と「network／update＝更新・インストールの出来事」を並べて書くが、更新の失敗（`error!`）はどちらにも当たる。1 件の記録は 1 つの種別だけに入れることにし、**warn 以上は出どころを問わず error 種別**、network・update・status は info の行だけとした（裁定 2）。
3. brief は「ゴースト名で絞る」と書くが、今の行にはゴーストの名前の欄が無い。取り決めの欄（`ghost`）を持つ記録だけがゴーストに属し、今ある行は欄が付くまでゴーストに属さない記録として残る（裁定 7・要件 2.6）。既存の行へ欄を足すのは、その行の持ち主の spec が触るときでよい。
4. brief の Current State の「SSP の script の行の種別は `[SSTP(Local,Auth)]`」（survey §3）は、1 行の `[<種別>]` の欄に `script` 以外の語が入ることを示す。出す側が欄で表示の語を渡せる形にした（裁定 8・要件 2.7）。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 種別への振り分けを「出す側に target を足す」か「既存のモジュールのパスで振り分ける」か（brief の議題） | **既存のモジュールのパスで振り分ける**（出す側は 0 行） | C3 の約束。出す側を触ると並走する 4 spec と重なる | 2・8.2 |
| 2 | warn 以上の更新・インストールの行はどの種別か | **error 種別だけ**（1 件は 1 種別）。network・update・status は info の行だけ。debug・trace は残さない | SSP の説明（description）は error を「errors/warnings」と書く。AI は誤りを error だけ見れば拾える | 2.1・2.3〜2.5 |
| 3 | network と update の分け方 | **network**＝URL からの取得の行（`areka::install::fetch_url`・`areka_update::winhttp`・`areka_update::fetch`）。**update**＝それ以外の更新とインストールの行（`areka_update`・`areka::update`・`areka::install`）。細かいパスが先に当たる | brief「network／update＝`areka-update`・インストールの出来事」。SSP の `update` 種別の実例は**未測**（survey §5） | 2.3・2.4 |
| 4 | status に入れる行 | 起動・切替・読み込みの節目＝`areka`（`main.rs`）・`areka::boot_resolve`・`areka::ghost_session`・`areka::emo2_boot::ghost_switch` の info の行 | brief「status＝起動・読み込みの節目」 | 2.5 |
| 5 | 履歴の上限（brief「上限の数は要件で決める」） | **種別ごとに 1,000 件**（合わせて最大 5,000 件）。1 件の本文は **4,096 文字**まで（超えた分は捨て、末尾に ` ...(truncated)` を付ける） | 種別ごとに分けると、status の行が多くても error が押し出されない。最大でも数十 MB に届かない | 1.4〜1.6 |
| 6 | 履歴に残すかどうかを `RUST_LOG` に従わせるか（brief の議題） | **従わせない**（`RUST_LOG=warn` でも status の info は残る。`RUST_LOG=areka_mcp=debug` でも error は残る） | `RUST_LOG` は標準出力の見え方の設定であり、AI が読む履歴が黙って欠けると「エラーなし」と誤読する | 1.7・1.8 |
| 7 | 1 行の `<名>` の欄 | 取り決めの欄 `ghost` があればその値。無ければ **`areka`** | SSP がゴーストに属さない行に何を出すかは**未測**。空にすると 1 行の形（`<名> : <本文>`）が崩れる | 2.6・4.2 |
| 8 | 1 行の `[<種別>]` の欄 | 取り決めの欄 `label` があればその値、無ければ種別の語（`error`・`script`・`network`・`update`・`status`） | 改めた点 4。SSP の script 以外の種別の表示は survey の記述（`[<種別>]`）のまま | 2.7・4.2 |
| 9 | 取り決めの target の名前 | script＝**`areka::log::script`**、error へ直接残す口＝**`areka::log::error`**（どのレベルで出しても残る） | 既存の target・モジュールのパスと重ならない。名前は設計で変えてよいが、変えたら取り決めの文書と要件 2 を合わせる | 2.2・2.1・7 |
| 10 | `log_type` の省略・空の文字列 | 省略は **`error`**。空の文字列は未知の値と同じ `NG:`。5 語との照合は大文字小文字を区別しない（前後の空白は削らない）＝**SSP と同じ**（要件ディスカッション議題 1 で確定） | SSP 2.9.07 の実測（research.md §2.1・§2.3）。SSP 向けの呼び方が同じ答えで通じる | 3.1・3.2 |
| 11 | `ghost_name` の意味（`mcp-tool-entrances` 要件 3.7 が本 spec へ委ねた点） | 省略＝絞らない。値があれば（空も）**起動中のゴーストから探し**（名前かフルパス・他のツールと同じ解決）、当たればそのゴーストの名前の記録だけ、当たらなければ `NG:Cannot find active ghost from specified name`＝**SSP と同じ**（要件ディスカッション議題 2 で確定） | SSP 2.9.07 の実測（research.md §2.1）。名前の打ち間違いが「ログなし」に化けない。もう起動していないゴーストの記録は省略で読める | 5.1〜5.4 |
| 12 | `since_id`・`max_count` の端の値 | `since_id` が負＝0 と同じ（全部）。`max_count` が負＝省略と同じ（数で絞らない）。**`max_count` が 0＝0 件**＝SSP と同じ（要件ディスカッション議題 1 で確定） | SSP 2.9.07 の実測（research.md §2.1） | 5.5〜5.8 |
| 13 | 本文の作り方 | 出来事のメッセージに、取り決めの欄（`ghost`・`label`）を除く残りの欄を ` 名前=値` の形で続ける | 今の行は `event = "…"` などの欄に意味を載せている。欄を捨てると何が起きたか分からない | 4.3 |
| 14 | 通し番号の始まりと寿命 | アプリの起動ごとに **1 から**。履歴から捨てた記録の番号は使い回さない。再起動で 1 へ戻る | SSP の id も起動ごと（survey の例は 2 桁）。AI は `since_id` に「見た中の最大の id」を渡す | 1.2・1.3 |
| 15 | 時刻 | 記録した時点の**現地時刻**・`yyyy/mm/dd hh:mm`（24 時間・0 埋め） | survey §3 の書式。SSP の利用者が見る時刻と同じ読み方 | 4.2 |

## Boundary Context

- **In scope**:
  - ログの出来事を種別へ振り分けてメモリに残す履歴（通し番号・時刻・上限・古いものから捨てる）。
  - 種別への振り分けの規則（既存のモジュールのパスとレベル・取り決めの target と欄）と、その取り決めの文書（`doc/` の下・新規）。
  - `get_log` の中身（`log_type` の検査・書式・`ghost_name`／`since_id`／`max_count` の絞り込み・0 件の文言）。
  - 後続の spec のための口（「いま最後に振った通し番号」を読む・取り決めの target で error／script へ残す）。
  - ログの出口の組み替え（`crates/areka/src/main.rs` の tracing の初期化だけ）。
  - 決定論テスト。
- **Out of scope**:
  - script 種別の行を出すこと（再生した台本の 1 行＝`mcp-kanade-tools`）。本 spec は受け皿と取り決めだけを持つ。
  - strict の台本の誤りを見つけて記録すること・`sakurascript` の返事の `since_id`（`mcp-strict-errors`）。
  - 既存の出す側の行を変えること（target・欄・文言・レベルのどれも 0 行）。`ghost` の欄を既存の行へ足すこともしない。
  - 標準出力の出力の書式・`RUST_LOG` の意味を変えること。
  - ログのファイルへの保存・再起動をまたぐ履歴・ログ窓の表示。
  - SSP 2.9.05 の欠陥（台本の逆斜線で JSON が壊れる＝survey §4-2）を写すこと（2.9.07 では直っている＝research.md §2.3）。
  - `Cargo.toml` の変更（足さずに済む見込み。要るなら設計で理由を書く）。
- **Adjacent expectations**:
  - `mcp-tool-entrances` の入口（定義の逐語・引数の型の検査・`-32602`・橋・待ちの上限）は変えない。本 spec が触るのは、同 spec の design が固定した「自分のツールのファイル」（`crates/areka/src/mcp/get_log.rs`・`get_log_tests.rs`）と履歴の新規ファイル、`main.rs` の初期化である。`crates/areka/src/mcp/mod.rs`・`crates/areka-mcp/src/**` は触らない。
  - `mcp-kanade-tools` は、再生した台本を取り決めの target と欄（`ghost`・必要なら `label`）で出す。本 spec とコードの依存を作らない。
  - `mcp-strict-errors` は、取り決めの target で error 種別へ残し、「いま最後に振った通し番号」を読んで `sakurascript` の返事に載せる。
  - `ghost` の欄を持たない今の行は、`ghost_name` で絞ると出てこない。各行へ欄を足すのは、その行の持ち主の spec の仕事である。
  - `mcp-tool-entrances` の振り分けのテスト（0 体の `get_log` が名前の解決の `NG:` を出さない）は、本 spec の着地の後も緑のままである。

## Requirements

### Requirement 1: ログの出来事が通し番号付きで履歴に残る

**Objective:** As a AI エージェントでゴーストを作る人, I want areka のログのうち意味のある出来事がアプリの中に残っていること, so that 台本やイベントを送った後で、動いている areka に問い合わせて確かめられる

#### Acceptance Criteria

1. When 要件 2 の規則でどれかの種別に当たる出来事がログへ出る, the areka shall その出来事を 1 件の記録（通し番号・記録した時刻・種別・`<名>`・`[<種別>]` の表示の語・本文）として履歴に残す。
2. The areka shall 通し番号を、アプリの起動ごとに 1 から始め、種別を問わず記録 1 件ごとに 1 ずつ増やす（全種別で 1 本の番号。後から残した記録の番号は必ず前の記録より大きい）。
3. The areka shall 履歴から捨てた記録の番号を使い回さない。
4. While ある種別の記録が 1,000 件ある, when その種別の新しい記録が来る, the areka shall その種別のいちばん古い記録を 1 件捨ててから新しい記録を残す（他の種別の記録は捨てない）。
5. If 本文が 4,096 文字を超える, then the areka shall 先頭の 4,096 文字だけを残し、末尾に ` ...(truncated)` を付ける。
6. The areka shall どの種別にも当たらない出来事を履歴に残さない（通し番号も進めない）。
7. The areka shall 履歴に残すかどうかを `RUST_LOG` の設定と独立に決める（`RUST_LOG=warn` でも status・network・update の info の出来事が残り、`RUST_LOG` で特定のクレートだけに絞っても、他のクレートの warn 以上の出来事が error 種別に残る）。
8. The areka shall 標準出力へのログの出力を、履歴を足す前と同じに保つ（同じ `RUST_LOG` で同じ行が同じ書式で出る。`RUST_LOG` が未設定・不正なら `info`）。
9. While 複数のスレッドが同時にログを出している, the areka shall 記録を 1 件も取りこぼさず、番号の重複も 0 件にする（1.4 で捨てるものを除く）。
10. The areka shall 履歴への記録の失敗でアプリを止めない（パニック 0 件）。
11. The areka shall 1.2〜1.6・1.9 を、決めた順に出来事を与えて番号・件数・捨てた記録を確かめる決定論テストで固定する（1,001 件目で最古の 1 件だけが消えること・他の種別が減らないこと・4,097 文字の本文が切られることを含む）。

### Requirement 2: 種別への振り分けの規則

**Objective:** As a AI エージェントでゴーストを作る人, I want ログの出来事が SSP と同じ 5 つの種別に分かれて残ること, so that 「誤りだけ」「再生した台本だけ」のように目的の行だけを読める

#### Acceptance Criteria

1. When 出来事のレベルが warn 以上である（2.2 の target を除く）、または target が `areka::log::error` である（レベルを問わない）, the areka shall その出来事を error 種別として残す（出どころのクレート・モジュールを問わない。他の種別には入れない）。
2. When 出来事の target が `areka::log::script` である, the areka shall その出来事をレベルを問わず script 種別として残す（warn 以上で出されても 2.1 より先に当たり、error 種別には入れない）。
3. When レベルが info で、target が `areka::install::fetch_url`・`areka_update::winhttp`・`areka_update::fetch` のどれか（またはその下のモジュール）である出来事が出る, the areka shall その出来事を network 種別として残す（今ログを出しているのは `areka::install::fetch_url` の info 3 行だけで、`areka_update::winhttp`・`areka_update::fetch` は 1 行も出していない。この 2 つは、行の持ち主の spec が取得の行を足したときの受け皿である）。
4. When レベルが info で、target が `areka_update`・`areka::update`・`areka::install` のどれか（またはその下のモジュール）であり、2.3 に当たらない出来事が出る, the areka shall その出来事を update 種別として残す。
5. When レベルが info で、target が `areka`（下のモジュールを含まない）・`areka::boot_resolve`・`areka::ghost_session`・`areka::emo2_boot::ghost_switch` のどれかである出来事が出る, the areka shall その出来事を status 種別として残す。
6. When 残す出来事が `ghost` の欄を持つ, the areka shall その値を記録の `<名>` とする。持たなければ `<名>` を `areka` とする。
7. When 残す出来事が `label` の欄を持つ, the areka shall その値を記録の `[<種別>]` の表示の語とする。持たなければ種別の語（`error`・`script`・`network`・`update`・`status`）とする（表示の語が変わっても、記録の種別は変わらない）。
8. The areka shall 2.1〜2.5 のどれにも当たらない出来事（レベルが debug・trace で取り決めの target でないもの、2.3〜2.5 のどの target でもない info の出来事）を残さない。
9. The areka shall target の照合を「そのモジュール自身か、`::` で区切った下のモジュール」で行う（`areka::update` は `areka::update::desk` に当たり、`areka::updater` には当たらない）。2.5 の `areka` だけは例外で、完全に一致する target だけに当たる（下のモジュールまで含めると、アプリ本体の info の行が全部 status になるため）。
10. The areka shall 標準出力の出力に warn 以上で現れる出来事を、areka のクレートの外のライブラリが出したものも含めて error 種別に残す（外のライブラリが別のログの仕組みで出す行が履歴へ届くことを、設計で実物を使って確かめる。届かないものがあれば取り決めの文書に名前を書く）。
11. The areka shall 2.1〜2.9 を、種別ごとに「当たる出来事」と「当たらない出来事」を 1 つ以上与える決定論テストで固定する（warn の更新の行が error だけに入ること・`areka::updater` が update に入らないこと・`ghost` と `label` の有無の 4 通りを含む）。
12. The areka shall 2.3〜2.5 の target が今のコードに実在することを、テストで判定する（名指ししたモジュールが無くなる・名前が変わると赤になる）。

### Requirement 3: `get_log` の `log_type`

**Objective:** As a AI エージェントの利用者, I want `log_type` で読む種別を選べ、誤った値は SSP と同じ文言で分かること, so that SSP 向けの呼び方がそのまま通じる

#### Acceptance Criteria

1. When `get_log` の `log_type` が無い, the areka shall error 種別を読む。
2. When `log_type` が `error`・`script`・`network`・`update`・`status` のどれかと、大文字小文字の違いを除いて一致する（`STATUS`・`Status` は status。前後の空白は削らない）, the areka shall その種別を読む。
3. If `log_type` が 3.1・3.2 のどれでもない（空の文字列・前後に空白のある語を含む）, then the areka shall 本文 `NG:Unknown log_type (error / script / network / update / status)`・`isError: true` の結果で答える。
4. While `ghost_name` が無い, the areka shall `get_log` を、起動中のゴーストが 0 体でも答える（名前の解決の `NG:` を出さない。`ghost_name` があるときは要件 5.3）。
5. The areka shall `get_log` で、ゴーストに何もさせない（SHIORI を呼ばない・台本を流さない）。
6. The areka shall 3.1〜3.3 を決定論テストで固定する（5 語それぞれ・省略・空・未知の語・大文字の `ERROR`・前後に空白のある ` error `）。

### Requirement 4: `get_log` の結果の書式

**Objective:** As a AI エージェントの利用者, I want 結果が SSP と同じ 1 行 1 件の書式で返ること, so that SSP 向けに書いた読み方（`#id` を拾って次の `since_id` に渡す）がそのまま使える

#### Acceptance Criteria

1. When 絞り込み（要件 5）の後に 1 件以上の記録が残る, the areka shall それらを古い順（通し番号の小さい順）に、記録と記録の間を `\r\n` で区切った 1 つの文字列として、`isError: false` の結果で答える（先頭に `OK:` を付けない。末尾に `\r\n` を付けない）。
2. The areka shall 1 件の記録を `#<id> <yyyy/mm/dd hh:mm> [<表示の語>] <名> : <本文>` の形で書く（`<id>` は通し番号の 10 進、時刻は記録した時点の現地時刻で 24 時間・0 埋め、欄の間は半角の空白 1 つ、`<名>` と `<本文>` の間は ` : `）。
3. The areka shall 本文を、出来事のメッセージに、取り決めの欄（`ghost`・`label`）を除く残りの欄を ` 名前=値` の形で出来事に書かれた順に続けたものとする（残りの欄が無ければメッセージだけ。メッセージが無ければ欄だけ）。
4. When 本文が改行（`\r\n`・`\n`・`\r`）を含む, the areka shall 改行ごとに `\r\n` とタブ 1 つへ置き換える（2 行目以降はタブで始まる。行の頭が `#` で始まるのは記録の 1 行目だけ）。
5. When 絞り込みの後に記録が 0 件である, the areka shall 本文 `(no log entries)`・`isError: false` の結果で答える。
6. The areka shall 本文の字を変えずに返す（逆斜線・二重引用符・制御文字・日本語を含む台本でも、MCP の応答は JSON として読め、読んだ文字列は記録した本文と一致する）。
7. The areka shall 4.1〜4.6 を決定論テストで固定する（時刻は与えた値で書式だけを見る。`\0\s[0]こんにちは\n\![raise,OnTest]` のような逆斜線入りの本文・3 行の本文・0 件・1 件・複数件を含む）。逆斜線入りの本文が JSON として読めて一致することは、実ソケットの `tools/call` でも 1 本固定する。

### Requirement 5: `ghost_name`・`since_id`・`max_count` の絞り込み

**Objective:** As a AI エージェントの利用者, I want 前回より後の行だけ・新しい方から数件だけ・あるゴーストの行だけを読めること, so that 同じログを何度も読まずに、自分の操作の結果だけを確かめられる

#### Acceptance Criteria

1. When `ghost_name` が無い, the areka shall ゴーストで絞らない（ゴーストに属さない記録も含める）。
2. When `ghost_name` が起動中のゴーストの名前、またはルートフォルダのフルパスに当たる（照合は `mcp-tool-entrances` 要件 3.2・3.3 と同じ＝他のツールと同じ解決。フルパスは大文字小文字・区切り・末尾の区切りの差を同じとみなす）, the areka shall `<名>` がそのゴーストの名前（`get_active_ghost_list` が返す値）と完全に一致する記録だけを残す。
3. If `ghost_name` があり（空の文字列を含む）、起動中のどのゴーストにも当たらない（起動中のゴーストが 0 体のときを含む）, then the areka shall 本文 `NG:Cannot find active ghost from specified name`・`isError: true` の結果で答える（SSP と同じ。もう起動していないゴーストの記録は、`ghost_name` を省いて読む）。
4. When `ghost_name` が起動中のゴーストに当たり、そのゴーストの記録が 1 件も無い, the areka shall 0 件（要件 4.5）として答える。
5. When `since_id` がある, the areka shall 通し番号がその値より大きい記録だけを残す。`since_id` が無い・負であるときは番号で絞らない。
6. When `max_count` が 1 以上である, the areka shall 他の絞り込み（種別・`ghost_name`・`since_id`）を済ませた後の記録のうち、新しい方からその件数だけを残す（返す順は古い順のまま）。
7. When `max_count` が無い・負である, the areka shall 件数で絞らない（その種別に残っている記録を全部返しうる）。When `max_count` が 0 である, the areka shall 0 件（要件 4.5）として答える。
8. When `since_id` がいま最後に振った通し番号以上である, the areka shall 0 件（要件 4.5）として答える。
9. The areka shall 絞り込みを「種別 → `ghost_name` → `since_id` → `max_count`」の順に行う。
10. The areka shall 5.1〜5.9 を決定論テストで固定する（種別の混ざった 10 件ほどの履歴に対し、`since_id` の境の値〔ちょうど同じ・1 つ小さい・負・最大より大きい〕、`max_count` の 0・負・1・件数より大きい値、`since_id` と `max_count` の併用、名前の一致・フルパスの読み替え・当たらない名前と空の名前の `NG:`・当たるが記録の無いゴーストの 0 件を含む）。

### Requirement 6: 後続の spec のための口

**Objective:** As a 後続の spec（`mcp-kanade-tools`・`mcp-strict-errors`）の実装者, I want 本 spec のコードに依存せずに履歴へ残せ、通し番号の今の値を読めること, so that 並走しても同じファイルを奪い合わない

#### Acceptance Criteria

1. When 他の処理が取り決めの target（`areka::log::script`・`areka::log::error`）でログを出す, the areka shall その出来事を、出した側が本 spec の型や関数を使っていなくても、要件 2 のとおり履歴に残す。
2. The areka shall 「いま最後に振った通し番号」（1 件も無ければ 0）を、アプリ本体の他の処理が読める口として持つ（その値を `since_id` に渡して `get_log` を呼ぶと、読んだ時点より後の記録だけが返る）。
3. When ある処理が出来事を出し終えてから `get_log` が届く, the areka shall その出来事を結果に含める（出した直後の問い合わせで取りこぼさない）。
4. The areka shall 6.1〜6.3 を決定論テストで固定する（取り決めの target で出した行が script・error へ入ること、番号を読んでから出した行だけが `since_id` で返ること）。

### Requirement 7: 取り決めの文書

**Objective:** As a ログを出す側の spec の実装者, I want どう出せばどの種別に残るかが 1 つの文書で分かること, so that 履歴のコードを読まずに正しい行を書ける

#### Acceptance Criteria

1. The areka shall 取り決めの文書を `doc/` の下に 1 つ持ち、次を書く: 5 つの種別と振り分けの規則（要件 2.1〜2.5・2.8・2.9）、取り決めの target の名前、取り決めの欄（`ghost`・`label`）の意味、本文の作り方（要件 4.3）、上限（要件 1.4・1.5）、`RUST_LOG` と独立であること（要件 1.7）。
2. The areka shall 文書に、script の行と error へ直接残す行の書き方の例を 1 つずつ載せる。
3. The areka shall 文書の規則（種別ごとの target の一覧と取り決めの名前）と実装の一致を、テストで判定する（どちらかだけを変えると赤になる）。
4. The areka shall `.kiro/steering/logging.md` の「Subscriber 初期化」の例を、組み替えた後の実物に合わせる。

### Requirement 8: 触る範囲

**Objective:** As a 同じウェーブで並走する spec の実装者, I want 本 spec が共有のファイルをほとんど触らないこと, so that 互いに rebase を強いられない

#### Acceptance Criteria

1. The areka shall `crates/areka/src/main.rs` の変更を、ログの出口の組み替えと履歴のモジュールの宣言だけにとどめる。**触る理由**: ログの出口はアプリの起動の先頭で 1 度だけ決まり、`fn main()` のその 1 か所に履歴を重ねる以外に、すべての出来事を履歴へ届ける場所が無い。`main.rs` は 1,000 行の上限を超えない。
2. The areka shall 出す側の行（`crates/areka-update/`・`crates/areka/src/update/`・`crates/areka/src/install/`・`boot_resolve.rs`・`ghost_session.rs`・`emo2_boot/`）を 0 行変える。
3. The areka shall `crates/areka/src/mcp/mod.rs`・`crates/areka-mcp/src/**` を 0 行変える（`get_log` の定義の逐語・引数の検査・橋はそのまま）。
4. The areka shall 既存の決定論テストを、`crates/areka/src/mcp/get_log_tests.rs`（`NG:not implemented yet` の期待を本物の答えへ書き換える）を除いて変えずに緑のままにする。
5. The areka shall ログの捕捉先を直接差す呼び出しを新設しない（`crates/log-capture-kit/tests/with_default_guard_test.rs` が緑のまま）。履歴のテストでログを捕まえる要があるときは `log-capture-kit` を通す。
