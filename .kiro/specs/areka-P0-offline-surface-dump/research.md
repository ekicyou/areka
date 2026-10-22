# ギャップ分析: areka-P0-offline-surface-dump

- 調べた日: 2026-10-22（ワークツリー `claude/areka-p0-offline-surface-dump-0006ba`・main `6e0ea8d8` の上）
- 入力: `requirements.md`（要件 1〜13・討議で確かめる点 5 つ）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`）
- 正典: ukadoc MCP の `ssphelp:option`（「起動オプション一覧」の「オフラインサーフェスダンプ」・https://ssp.shillest.net/ukadoc/ssphelp/option.html ）・`ukadoc:descript_shell_surfaces:runonce`・`ukadoc:descript_shell:kero.bindgroup*.default,数値` ほか
- SSP は起こしていない（`ssp.exe` を立てず、SSP の MCP も呼んでいない）。測りが要る点は末尾の「調べが要る点」に、何を測るかを書いた。

---

## 1. まとめ（5 点）

- **部品はほぼ揃っている**: シェルを窓なしで読む `load_shell_target`（WIC＋CPU だけ）・面を組む `Composer::compose`（純粋）・既定の着せ替えの組 `default_bind_ids`→`build_static_bindset`・着せ替えの名前の表 `BindResolver`（`resolve`／`policy`／`category_ids`）・積み上げ `accumulate`・名前でシェルを探す `shell_candidates`＋`resolve_skin_target`・PNG にする `unpremultiply`＋`png`（MCP の中だけで見える）・面の番号の拡張の書き方の展開（`parse_target_element`／`expand_targets`・どちらも私的）が在る。
- **足りないもの**: 起動オプションの欄（`--offline-dump` は今「扱わない」の行）・`main.rs` の入口・書き出しの走りそのもの（新しいモジュール）・書き出しの記録を CSV にする層（今の記録の履歴は 1 種類 1,000 件まで・時刻は分まで・本文 4,096 字で切る＝要件 7.5・7.2 に足りない）・runonce を 1 周させた後の `PatternState` を作る口（今は areka-seriko のアクターの中でしか進まない）・SSP の絵と比べる道具。
- **runonce の時点（議題 1）**: 窓なしで進める今の口は無い。いちばん小さく正しいのは、areka-seriko に「注入の時計で `handle_message` を同じスレッドで回し、runonce が終わる時刻まで刻んで最後の `Show` の `PatternState` を返す」公開の関数を 1 つ足す案（2〜3 タスク・画面と同じ道を通るので要件 4.5 を崩さない）。always・random・periodic をその時刻でどう扱うかは正典が言い切らず、SSP で測る必要がある。
- **画面の着せ替えとの食い違い（新しい議題）**: 画面は **どのスコープにも `sakura.bindgroup*.default` だけ**を既定として当てている（`default_bind_ids` は `kero.*` を読まない・`ScopeStates` の既定は全スコープ共通）。要件の用語「既定の着せ替え」（`kero.bindgroup*.default`・`char*.bindgroup*.default`）と要件 4.5（画面と同じ絵）が、スコープ 1 以上で両立しない。また着せ替えの名前の表はスコープ 2 以上を持たない（`scope_namespace` が `None`）。
- **規模と危うさ**: M〜L（14〜18 タスク）。危うさは中（runonce の時点と記録の取り込みの 2 か所に新しい形が要る・ほかは今ある部品を窓なしで呼ぶだけ）。

---

## 2. 今の作りの調べ

### 2.1 起動オプション（`crates/areka/src/cli/`）

- `cli/parse.rs`（459 行）: 表 `TABLE` に `spec("offline-dump", None, Arity::One, Kind::Unsupported)`（65 行目）。`dump-` で始まる名前は `lookup_long` がまとめて `(Arity::Optional, Kind::Unsupported)` で読み、`ParseNote::Unsupported`（warn・`startup_option_unsupported`）を 1 件出して値ごと捨てる。
- `cli/mod.rs`（63 行）: `StartupOptions { boot, listen, modules, help }`。オフラインの欄は無い。
- 値の読み方は「次の引数が名前でなければ値」。`--dump-scope -1` のように値が `-` で始まると名前として読まれる（今の決まりのまま・要件 12.1 の分岐に入る）。`--dump-bind` の `;` と `,` はそのまま 1 つの値として届く。
- `parse_tests.rs` は 679 行＝足すテストは新しい兄弟のファイルへ（steering の 1,000 行の目安）。

### 2.2 入口（`crates/areka/src/main.rs`・965 行）

- 順: `human_panic` → `log_history::init()` → `cli::read()` → `if opts.help { return cli::help::run(); }`（164〜167 行目）→ `thread_roles::install` → `perf_thread_report::start` → `resolve_boot`（起動中の印を **読む**）→ `write_failure::check_boot_dirs` → MCP の待受 → `WinApp`。
- `--help` と同じ位置に「オフラインなら走らせて終わる」を置けば、起動の解決・記憶・MCP・窓・GPU のどれにも触れない（要件 1.2〜1.4・9.1〜9.3 は入口の位置だけで満たせる）。足すのは数行＝上限 1,000 行の内。
- `fn main() -> Result<()>`（`windows::core::Result`）なので、終了コード 1・2 を返すには `std::process::exit(code)` か `main` の戻りの型を `ExitCode` 系へ変える必要がある（下の議題）。
- **リリースの組み立ては GUI のサブシステム**（`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`・1 行目）。コマンドプロンプトの対話や PowerShell の `&` は GUI の exe の終わりを待たず、`$LASTEXITCODE` も取れない（`Start-Process -Wait -PassThru` か `cmd /c start /wait` が要る）。SSP も同じ形なので正典との差ではないが、説明書（要件 11）の例と CI の使い方に関わる。テストの `std::process::Command` は待つので試験には響かない。標準出力の記録も、リリースではリダイレクトしない限り見えない。

### 2.3 シェルの読み込みと合成

| 部品 | 場所 | 窓なしで使えるか |
|---|---|---|
| シェルを読む入口 `load_shell_target(shell_dir, &decoder)` | `crates/areka-emo-present/src/shell_target.rs` 267 行目 | 使える。要るのは COM の初期化（`WicDecoderArm::new()`）だけ。読み込みの警告・誤りはすべてこの関数が tracing で出す |
| 面の表 `ShellTarget::build_world()` | 同 236 行目 | 使える。スコープごとに組む（中身は同じ） |
| 合成 `Composer::compose(world, atlas, id, binds, pattern)` | `crates/areka-emo-compose/src/lib.rs` 165 行目 | 使える（純粋・CPU）。失敗は `build_plan` が `error!` を出してから `Err` |
| ゴーストの descript と既定のシェル `resolve_with_shell(ghost_root, Ansi, None)` | `crates/areka-parsers/src/package/resolve_shell.rs` | 使える。`seriko.defaultsurfacedirectoryname`、無ければ `master`。利用者の最後のシェルの記憶は読まない（要件 2.3 に合う）。`GhostNames.name` が CSV のゴースト名になる |
| シェルの資産の組み立て一式 `build_shell_assets` | `crates/areka/src/emo2_boot/assets.rs` 330 行目付近 | 使える（`BindResolver`・既定の組・`AnimationTable` を一度に作る）。ただし箱のフォントを探す場所など、書き出しに要らないものも作る |
| 名前でシェルを探す `shell_candidates`＋`resolve_skin_target` | `crates/areka/src/emo2_boot/shell_balloon_resolve.rs` | 使える（純粋）。照合は「`name` → フォルダ名」・特別な名前 `random`・`lastinstalled` を先に解く |
| 単体の合成 `EmoPresenter::compose_alone` | `crates/areka-emo-present/src/presenter/snapshot.rs` 74 行目 | 窓の結線が要るので使わない。参考: 着せ替えは「最後に出した組」、動きは `PatternState::default()`（＝MCP `dump_surface` の面の指定の答え） |

- 画面の絵も同じ `Composer` で組み、拡大は出す段（D2D の変換）でかける＝合成の絵は拡大していない（要件 4.3・4.5 の前提は成り立つ）。
- **記録の重なり**: `load_shell_target` は中で面の表を 1 度組み（`build_shell_target_with_boxes` の `probe`）、書き出しがスコープの面の表をもう 1 度組むと、`fold.rs`（警告 9 か所）・`world.rs`（4 か所）の warn が **同じ内容でもう 1 度** 出る（`build_world` の注記「既存の `warn!` は今日と同じく組むたびに出る」）。CSV に同じ行が 2 度並ぶ。画面の起動でも同じことが起きている（スコープの数だけ）。

### 2.4 着せ替え

- 既定: `default_bind_ids(&shell_kv)`（`assets.rs`）は **`sakura.bindgroup{N}.default` だけ**を読む（注記「`kero.*` scope の bindgroup default は本タスクの対象外（M-dual 増分）」）。`ScopeStates::new(static_binds)` は全スコープに同じ既定の組を使う。→ 画面ではスコープ 1 も `sakura.*` の既定で着ている。
- 正典は 3 つを分けている:
  - `ukadoc:descript_shell:sakura.bindgroup*.default,数値`「アニメーションID*番の着せ替えを最初から表示するか否か。1で表示、0で非表示。(本体側)」
  - `ukadoc:descript_shell:kero.bindgroup*.default,数値`「…(相方側)」
  - `ukadoc:descript_shell:char*.bindgroup*.default,数値`「…(3人目以降)」
  - 一方 `ssphelp:option` の `--dump-bind` は「基準となる状態は、シェルのdescript.txtで指定された各パーツのデフォルト（sakura.bindgroup*.default）」と sakura だけを名指しし、「着せ替えは --dump-scope で指定したスコープに対して適用される。」とも書く。
- 名前の表 `BindResolver` は sakura と kero の 2 つだけ。`scope_namespace` はスコープ "0"→Sakura・"1"→Kero・2 以上→`None`（`char*.bindgroup*.name` を読まない）。
- 画面の `\![bind]`（`actor.rs` 560〜650 行）: 排他（`Default`／`MustSelect` の着衣は `apply_bind_exclusive`）・`Multiple` は加算・**`mustselect` の脱衣は無視**（「解除不可」）・パーツ名が空（`CategoryWide`）と値の省略（`Toggle`）は warn で読み飛ばす・`parse_bind_directive` は前後の空白を **削る**。
- `addid`（`*.bindgroup*.addid`）は areka のどこにも無い（画面でも効かない）。
- `--dump-bind` に要る形（空白を削らない・`true`／`false`・省略は ON・`__ALL__`）は今の `parse_bind_directive` と違うので、読み方は新しく書く。名前の解決・排他・積み上げは `BindResolver`＋`accumulate`（純粋）で組める。`ScopeStates::apply_bind_exclusive` も公開されている。

### 2.5 面の番号の拡張の書き方

- `areka-parsers/src/shell/decode.rs` の `parse_target_element`（私的）: `!N`・`!a-b`・`a-b`・`N` を `AppendTarget` へ。数字として読めないと **0 に倒す**（要件 3.5「読めない要素は Warning で無視」と合わない）。
- `areka-emo-compose/src/fold.rs` の `expand_targets`（`pub(crate)`）: 包含を記述順に並べ、除外を集合で引く。展開すると「単独で指したか範囲で指したか」が消える（要件 3.3・3.4 の区別ができない）。
- → `--dump-surface-list` の読みは書き出しのモジュールに小さく書くのが素直（頭の `surface` を各要素で外す・読めない要素を知らせる・単独と範囲を区別する・重ねは 1 度）。`AppendTarget` の型（公開）を使い回すかは設計で決める。

### 2.6 PNG

- `crates/areka/src/mcp/dump_surface_image.rs`（80 行）: `png_base64`（`pub(in crate::mcp)`）・`unpremultiply`（私的・α 0 は `(0,0,0,0)`・`min(255, round(色×255÷α))`）・`png`（私的・IHDR＋IDAT 1 つ＋IEND・`miniz_oxide`・CRC は `areka_nar::crc32`）・`base64`（私的）。テストは `dump_surface_image_tests.rs`（246 行）。
- 要件 4.6（色の戻し方を MCP と同じに）は「`unpremultiply`＋`png` を共有の場所へ出し、MCP は `base64` を重ねるだけにする」で満たせる。依存は足さない。

### 2.7 記録（CSV の材料）

- `crates/areka/src/log_history.rs`（424 行）: `init()` が `fmt` の層（標準出力・`RUST_LOG`）と履歴の層を重ねて **プロセスの既定の受け口** にする。履歴は 5 種別に振り分け、種別ごとに `PER_KIND_CAP = 1000` 件・時刻 `Stamp` は **分まで**・本文は `BODY_CAP_CHARS = 4096` 字で切る。→ そのまま CSV の元にすると要件 7.2（`HH:MM:SS`）・7.5（10,000 件まで間引かない）を満たせない。
- 記録をファイルへ書く層は無い（`tracing-appender` は使っていない）。要件 9.4 の「普段から書いている areka の記録のファイル」は今は無い＝書き出しで書くファイルは PNG と CSV だけになる。
- シェルの読み込みと合成は **呼んだスレッドの上だけ** で走る（`areka-emo-atlas`・`areka-emo-compose`・`shell_target.rs`・`areka-parsers` にスレッドを起こす所は無い）。→ `tracing::subscriber::with_default` で書き出しの間だけ受け口を差し替える形も取れる（下の案）。
- 起動オプションの読みの記録（`[cli]`）は `cli::read()` の中で出るので、取り込みを始める前に終わっている＝要件 7.3 の「入れない記録」は順番だけで外せる。ただし `--dump-shell`・`--dump-scope`・`--dump-surface-list`・`--dump-bind` の **値の扱い** の warn は CSV に入れる（要件 7.3）ので、値の読み（解釈）は `cli` でなく取り込みの後に行う必要がある。

### 2.8 SERIKO の動き（議題 1・2 の土台）

- `areka-seriko`: 一番上の面の再生は `LoopRuntime`（`looper.rs`・crate の中だけ）。`on_tick(now_ms, &mut ScopeStates)` が抽選 → 進行（`frame_at`）→ `PatternState` の差分、`refresh(scope, slot, at_ms, states)` が「見え始め」に runonce を鳴らす。部品（element定義の子）の runonce は `parts.rs`／`parts_trigger.rs`。
- 公開されているもの: 純関数 `frame_at`（`FinishedResidual`＝最後のコマが残る・`Stopped`＝`-1` で土台に戻る）・`always_at`・`AnimationTable::from_world`・`spawn_seriko_clocked`（スレッドを起こすアクター・時計 `SerikoClock` を注入できる）・`MockSurfaceOutput`・`ScopeStates`。
- `handle_message`（`actor.rs` 432 行目）は crate の中の **同期の関数**で、`SerikoMsg::Cue`／`Tick`／`Stage` を 1 件ずつ処理する。テスト（`actor_clock_tests.rs` など）は注入の時計で絶対時刻を刻んでいる。→ crate の中に「同じスレッドで `handle_message` を回す」公開の関数を 1 つ足せば、画面と同じ道で runonce を最後まで進められる。
- pattern の `start`・`alternativestart`・`insert` などは合成のメソッドに無い（`ComposeMethod::Unknown` に入り描かない）＝runonce が別のアニメーションを起こす連鎖は今の areka では起きない（進める足場が単純で済む）。
- 正典:
  - `ukadoc:descript_shell_surfaces:runonce`「サーフェスに切り替わった瞬間に1回のみ再生。」
  - `ssphelp:option` の `--dump-animation`「まず通常のダンプと同じ画像（runonceのアニメーションを1周させた後の状態）を surface0.png として出力し」「runonce・always などのアニメーションは通常のダンプと同様に動いており、その上に指定アニメーションが重なる。」
  - → 通常のダンプでも always は「動いている」と読めるが、**何ミリ秒の時点のコマか**は書いていない。random・periodic・talk・bind（着せ替え）・yen-e・never の扱いも書いていない。

### 2.9 動く絵（APNG・動く WebP）の element（議題 2）

- 動く絵は `areka-emo-compose/src/film.rs` で「子」（element を持たず `always` を 1 本持つ部品）に分かれる。合成は `push_film_op`（`plan_always.rs`）で、`PatternState` の欄が空なら **経過 0 のコマ**（`sheet.rest`＝1 枚目、1 枚目の待ちが 0 なら次のコマ）を描く。
- 画面では子の `always` が時計で進む＝「画面と同じコマ」（要件 4.7）は **いつの画面か** で変わる。決まった 1 枚を出すには、経過 0（1 枚目）にするか、runonce が終わった時刻の経過に合わせるかを決める必要がある（議題 1 と同じ問い）。
- `import` で読む画像（`animated-image-import`）は範囲外のまま（要件の Out of scope どおり）。

### 2.10 SSP の絵と比べる道具（要件 13）

- PNG を読む部品: `image` クレート（`png`・`webp` の機能つき）がワークスペースに在る（`areka-emo-atlas`・`wintf` の依存）。WIC の読み手は乗算済みの BGRA を返すので、α の差を正しく比べるには `image` で乗算していない RGBA を読むほうが素直。
- 置き場の候補: ⑴ 新しい小さなクレート（`publish = false`・bin 1 本＋純粋な比べの関数）⑵ 既存のテスト用クレート（`sample-ghost-kit` など）の `src/bin/` ⑶ `tools/` の PowerShell（`System.Drawing`）。決定論のテスト（要件 13.5）を Rust で書くなら ⑴ か ⑵。新しい外の依存は足さない（`image` はワークスペースに在る）。

---

## 3. 要件と今の部品の対応

| 要件 | 使える部品 | 足りないもの（印: 無＝Missing／未＝Unknown／縛＝Constraint） |
|---|---|---|
| 1.1〜1.5 窓・SHIORI・MCP・GPU を使わない | `--help` と同じ早い戻りの位置・`load_shell_target`・`Composer` | 無: `main.rs` の入口・走りのモジュール。縛: COM の初期化（MTA）を走りの頭で 1 度 |
| 1.6・1.7 始めと終わりの info | tracing | 無: 2 行 |
| 1.8 値が無い | `parse` の `Arity::One` | 無: 「値なしのオフライン」を表す欄（今は `Unsupported` に落ちる） |
| 1.9 起動中の areka・SSP に任せない | 早い戻り（二重起動の確かめより前） | なし |
| 2.1・2.6 ゴーストのフォルダ | `resolve_with_shell`（`StartPointMissing`／`StartPointUnreadable`） | 無: 失敗を「CSV なし・終了コード 0・error の記録」へ写す |
| 2.2 相対パス | `std::env::current_dir` | 縛: 今のフォルダは起動時に読む（途中で変えない） |
| 2.3〜2.5 シェルの決め方 | `resolve_with_shell(…, None)`・`shell_candidates`＋`resolve_skin_target` | 未: `random`・`lastinstalled` を `--dump-shell` で受けるか |
| 2.7 シェルの読み込みの失敗 | `ShellLoadError`（`List`／`Read`／`Empty`） | なし（写すだけ） |
| 3.1〜3.7 面の指定 | `AppendTarget`（公開の型） | 無: 読み手（単独と範囲の区別・読めない要素の warn・重ねの除去） |
| 3.8〜3.10 スコープ | `build_world` | 縛: スコープ 2 以上の着せ替えの名前の表が無い・既定の組は全スコープ sakura（下の議題） |
| 4.1 runonce を 1 周 | `frame_at`・`LoopRuntime`・`handle_message`（crate の中） | 無: 窓なしで進める公開の口。未: always 等の扱い（SSP で測る） |
| 4.2・4.3・4.4 | `Composer`（拡大しない・乗算済み BGRA） | なし |
| 4.5 画面と同じ | 同じ `Composer` | 縛: 画面の既定の着せ替えが全スコープ sakura（3.9 と食い違う） |
| 4.6 色の戻し方 | `unpremultiply`＋`png` | 無: 共有の場所へ出す（MCP の可視性 `pub(in crate::mcp)`） |
| 4.7 動く絵のコマ | `push_film_op`（経過 0 のコマ） | 未: どの時点か（議題 1 と一緒に決める） |
| 4.8 合成の失敗 | `build_plan` の `error!`＋`Err` | 縛: 合成が出す error と書き出しが出す error で 2 行になりうる |
| 5.1〜5.8 `--dump-bind` | `BindResolver`・`accumulate`・`apply_bind_exclusive` | 無: 読み手（空白を削らない・`true`／`false`・省略 ON・`__ALL__`）。未: `mustselect` の OFF・スコープ 2 以上・`addid` |
| 6.1〜6.7 ファイル名と置き場 | `std::fs` | 未: 頭の文字列に `\` や `:` が入ったとき（下の議題） |
| 7.1〜7.8 CSV | `GetLocalTime`（秒まで取れる） | 無: 取り込みの層（秒・件数・本文を切らない）。今の履歴は使えない（1,000 件・分まで・4,096 字） |
| 8.1〜8.4 終了コード | — | 無: `main` から 0・1・2 を返す道 |
| 9.1〜9.4 書かない | 早い戻り（`resolve_boot`・`write_failure`・`WinApp` より前） | なし（テストで固める） |
| 10.1〜10.4 ほかの起動オプション | `ParseNote` | 無: 新しい記録の種類（効かせない・`--offline-dump` なしの `--dump-*`） |
| 11 説明書 | `dist/README.txt` の「■ 起動オプション」（33 行目「効く起動オプション（6 つ）」・59 行目「まだ扱わない」の文） | 無: 節の追記 |
| 12 決定論のテスト | `temp-path-kit`（`target\` の下）・`log-capture-kit`・PNG の手書きの関数（検体の絵を作れる） | 無: テスト一式 |
| 13 比べる道具 | `image`（`png`） | 無: 道具とテスト。未: 置き場 |

---

## 4. 作り方の案

### 案 A: 今のファイルを広げる（`emo2_boot/assets.rs` と `mcp/` に足す）

- `build_shell_assets` をそのまま呼び、書き出しの関数を `emo2_boot/` に置く。PNG は `mcp/dump_surface_image.rs` の可視性を `pub(crate)` に広げるだけ。
- ✅ 新しいファイルが少ない。シェルの資産の組み方が画面と 1 つになる。
- ❌ `emo2_boot/` は「統合の背骨」で、窓の結線の文脈を持つ。窓を作らない走りを混ぜると境が濁る。`build_shell_assets` は箱やフォントの場所など要らないものも作る。PNG を MCP の下に置いたまま外から使うのは置き場の意味とずれる。

### 案 B: 新しいモジュールを立てる（brief の推し）

- 新 `crates/areka/src/offline_dump/`（`mod.rs`＝入口と順番・`run.rs`＝走り・`surface_list.rs`＝面の指定の読み・`bind_spec.rs`＝`--dump-bind` の読みと当て方・`csv.rs`＝CSV と記録の取り込み・兄弟の `*_tests.rs`）。
- `cli/` は値を **生の文字列のまま** 欄に入れるだけにし、解釈と warn は `offline_dump` が取り込みの後で行う（要件 7.3 の「値の扱いの記録は入れ、名前の読み取りの記録は入れない」を順番で満たす）。
- PNG は共有の場所（例 `crates/areka/src/png_image.rs`）へ出し、MCP は `base64` だけを重ねる。
- runonce は areka-seriko に公開の関数を 1 つ足す（下の 5 節の R2）。
- 比べる道具は別の置き場（6 節の議題）。
- ✅ 窓の文脈と混ざらない。テストが走りの単位で書ける。`main.rs` は数行。
- ❌ シェル・着せ替えの組み立ての数行（`resolve_with_shell`→`load_shell_target`→`default_bind_ids`→`BindResolver::new`）が `assets.rs` と書き出しの 2 か所になる（共有の小さな関数に出すかは設計）。

### 案 C: 混ぜる

- 走りは案 B の新しいモジュール。シェルの資産のうち「既定の着せ替えの組と `BindResolver` を作る」部分だけを `assets.rs` から純粋な関数として切り出し、画面と書き出しで共有する（スコープごとの既定の組の直しを入れるなら、その 1 か所を直せば両方に効く）。
- ✅ 画面と書き出しの着せ替えの基準が 1 つになる（要件 4.5 を構造で守れる）。
- ❌ `assets.rs` に手が入る（起動の結線の回帰を見る範囲が広がる）。

---

## 5. runonce を 1 周させた後の状態（議題 1）の調べと費用

| 案 | 中身 | 費用 | 要件 4.5（画面と同じ）との合い方 | 危うさ |
|---|---|---|---|---|
| R0: 進めない | `PatternState::default()`（MCP の面の指定と同じ・経過 0） | 0 タスク | 経過 0 の画面とは同じ。runonce を持つ面は正典と違う絵になる | 低。ただし SSP との比べで runonce の面が必ず違う |
| R1: アクターを使う | `spawn_seriko_clocked`＋注入の時計＋`MockSurfaceOutput` で `\s[ID]` の合図と刻みを送り、最後の `Show` を読む | 3 タスク前後 | 画面と同じ道 | 中。別スレッドになるので、そのスレッドの記録は `with_default` の取り込みに入らない（取り込みをプロセス全体の層にする必要）。待ち合わせが要る |
| R2: 同じスレッドで回す公開の関数を足す | areka-seriko の中に、`ScopeStates`・`LoopRuntime` を作り `handle_message` を同じスレッドで呼ぶ関数（例 `settled_pattern(table, resolver, bind_resolver, binds, scope, surface_id) -> PatternState`）。終わりの時刻は表から「runonce のコマの待ちの合計の最大（一番上と部品）」で求め、その時刻まで刻む。乱数は注入（抽選を起こさない列） | 2〜3 タスク（関数 1・終わりの時刻の計算 1・runonce／-1 で止まる／部品の runonce／always 併存の決定論テスト 1） | 画面と同じ道（`handle_message`→`on_tick`→`frame_at`） | 中。always・random・periodic をその時刻でどう扱うか（下の調べ）で刻み方と乱数の与え方が変わる |
| R3: 純関数で直に組む | `AnimationTable` の runonce だけを `frame_at(…, u64::MAX)` で読み、`PatternState` を手で組む | 2 タスク | 画面の進行（部品の時計・着せ替えの門・`-1` の扱い）を写しで持つ＝写しがずれると 4.5 が崩れる | 中〜高（写しの保守） |

- 補足: 今の areka では pattern の `start`・`alternativestart`・`insert` が描かれない（2.8）ので、runonce が他のアニメーションを起こす連鎖を追う足場は要らない。
- 補足: 着せ替え（`interval,bind`）の要素は `BindSet` で決まり、時間では進まない。`bind+runonce` のような複合の語は今の表で `Other`（採らない）。
- R2 の終わりの時刻を「runonce の最後のコマの時刻」にすると、always と動く絵の子はその時刻の経過のコマになる（runonce が無い面では経過 0）。「always は経過 0 のまま」にするなら、always を刻まない仕掛け（表から always を外す・時計を 2 本にする）が別に要る。どちらに合わせるかは SSP の測りで決める。

---

## 6. 設計の段で決めること（議題の候補）

1. **runonce の時点と、ほかの interval の扱い**（議題 1）: R0〜R3 のどれか。R2 を取るなら、runonce が終わった時刻での always・動く絵の子・random・periodic・talk の扱い（経過 0 に止めるか、その時刻まで進めるか、抽選を起こさないか）。SSP の測り（7 節の 1）で決める。
2. **動く絵の element のコマ**（議題 2）: 経過 0（1 枚目）か、runonce が終わった時刻のコマか。1 と同じ答えにそろえるのが素直。SSP の測り（7 節の 2）。
3. **スコープ 1 以上の既定の着せ替え（新しい議題）**: 画面は全スコープ `sakura.bindgroup*.default`。要件の用語・3.9 は `kero.*`・`char*.*` を指し、`--dump-bind` の正典は `sakura.*` を名指し。どれに合わせるか: ⑴ 書き出しも画面と同じ（sakura だけ・用語を直す）⑵ 書き出しだけスコープごと（4.5 が崩れる）⑶ 画面と書き出しの両方をスコープごとに直す（案 C・`default_bind_ids` と `ScopeStates` の既定を広げる・範囲が画面へ広がる）。SSP の測り（7 節の 3）で SSP の書き出しがどれを使うかも確かめられる。
4. **スコープ 2 以上の `--dump-bind`**: 名前の表が無いので、今のままでは指定がすべて「無いカテゴリ」の Warning になる。`char*.bindgroup*.name` を読むところまで広げるか、2 以上は「扱わない」を記録して無視するか。
5. **`mustselect` のカテゴリの OFF**: 画面の `\![bind]` は無視（解除不可）。正典の `--dump-bind` は「意味は\![bind]と同じ」。要件 5.4（OFF にする）と食い違う。画面に合わせて無視＋Warning にするか。
6. **`--dump-bind` の `__ALL__`／空のパーツ名と排他**: 排他のカテゴリで全パーツを ON にしたときの答え（最後のパーツだけ残るか・全部 ON か）。SSP の測り（7 節の 4）。画面の `\![bind]` はこの形を読まない（warn）＝画面で同じ組を作る道が無い。
7. **記録の取り込みの形**: ⑴ `tracing::subscriber::with_default` で書き出しの間だけ「標準出力の層＋取り込みの層」の受け口に差し替える（同じスレッドで走るので届く・`log_history` に触らない）⑵ `log_history::init` に眠っている第 3 の層を足し、書き出しが起こす。runonce を R1（別スレッド）にするなら ⑵ が要る。
8. **同じ内容の記録の重なり**: 面の表を組むたびに同じ warn が出る（2.3）。CSV で同じ行を 1 つにするか（正典は「似た内容のログを要約行…にまとめる処理も行わない」）、面の表を組む回数を減らすか、そのまま出すか。合成の失敗で合成と書き出しの両方が error を出す件も同じ問い。
9. **`--dump-shell` の特別な名前**: `random`・`lastinstalled` を切り替えと同じく解くか、ただの名前として探すか（オフラインでは `lastinstalled` の控えが無い）。
10. **値の解釈の置き場**: `cli` は生の文字列を持つだけにし、解釈と warn を `offline_dump` で行う（要件 7.3 のため）か、`cli` で解釈して記録の材料を後で出すか。
11. **終了コードの返し方**: `std::process::exit` か、`main` を `ExitCode` を返す形へ変えるか（`--help` の道と、ほかの `?` の早い戻りへの響き）。
12. **頭の文字列（`--dump-output-prefix`）に `\`・`/`・`:` などが入ったとき**: 下のフォルダへ書くのを許すか、要件 6.6 の Error にするか（SSP の振る舞いは正典に無い）。
13. **CSV のゴースト名**: `name` が無いゴーストのときの値（フォルダ名か空か）。
14. **比べる道具の置き場**: 新しい小さなクレートの bin か、既存のテスト用クレートの bin か、`tools/` の PowerShell か（要件 13.5 の決定論のテストを Rust で書くかで決まる）。
15. **説明書の例と GUI のサブシステム**: リリースの `areka.exe` は終わりを待たれず `$LASTEXITCODE` も取れないので、説明書の例を `start /wait`・`Start-Process -Wait -PassThru` の形にするか（要件 11.2）。
16. **シェルの組み立ての共有**: 着せ替えの基準と名前の表を作る数行を `assets.rs` と共有の関数にするか（案 C）、書き出しに写すか（案 B）。議題 3 で ⑶ を取るなら共有が要る。

---

## 7. 調べが要る点（SSP で測る・この分析では測っていない）

測りは SSP の `--offline-dump` を開発者か AI のエージェントが SSP で回して得る（`ssp.exe` を立てる許可が要る）。検体は `target\` の下に置く。

1. **runonce とほかの interval の時点**: 面 0 に ⑴ `interval,runonce`（3 コマ・待ち 100ms・最後のコマを残す）⑵ 同じで最後が `-1` ⑶ `interval,always`（周期 300ms・3 コマで色の違う絵）⑷ `interval,random,1` ⑸ `interval,periodic,1` を別々の面と、⑴＋⑶ を同じ面に持つシェルを作り、`--offline-dump <検体> --dump-surface-list "0-5"` で書き出す。見るもの: ⑴ は最後のコマが残るか・⑵ は土台に戻るか・⑶ は 1 枚目か別のコマか（何度書き出しても同じか）・⑷⑸ のコマが出るか・⑴＋⑶ の always は経過 0 か runonce の終わり（200ms）の時点か。
2. **動く絵の element のコマ**: 1 枚目と 2 枚目以降の色が違う APNG（と動く WebP）を element に置いた面を書き出し、何枚目が出るか（1 枚目の待ちが 0 の絵も 1 つ）。runonce を同じ面に持たせたときに変わるか。
3. **スコープ 1 の既定の着せ替え**: `sakura.bindgroup10.default,1`・`kero.bindgroup10.default,0`（とその逆）を持つシェルで `--dump-scope 0` と `--dump-scope 1` を書き出し、スコープ 1 の基準が `kero.*` か `sakura.*` か。スコープ 2 と `char2.bindgroup*.default` も同じく。
4. **`--dump-bind` の細部**: 排他のカテゴリ（`bindoption` 無し）で `カテゴリ,__ALL__,1` の答え・`mustselect` のカテゴリで `カテゴリ,パーツ,0` の答え（OFF になるか・Warning が出るか）・パーツ名の前後に空白がある指定の Warning の文。
5. **CSV の細部**: `--dump-shell` が見つからないときの行の文と終了コード・単独の番号の面が無いときの行の文・同じ警告が繰り返されたときに行が 1 つか複数か。areka の文を SSP に合わせる要件は無いが、比べの道具や下流の試験が文で拾うかの判断材料になる。
6. **絵の大きさ**（下流の `pasta-in-windows-verification` 向けの注意）: element が土台の外へはみ出す面で、SSP の書き出しの大きさ（土台の大きさで切るか、はみ出しを含めるか）。areka の合成は置いた element の外形に広がる（`extent-element-offset` の起票がある）＝比べで「大きさが違う」が出うる。

---

## 8. 規模と危うさ

- **規模: M〜L（14〜18 タスク）**。内訳の目安: 起動オプションの欄と表の行とテスト 2・入口 1・面の指定の読み 1・`--dump-bind` の読みと当て方 2・シェルとゴーストの決め方 1・runonce の口（R2） 2〜3・PNG の共有と MCP の付け替え 1・記録の取り込みと CSV 2・終了コードと終わりの記録 1・書かないことのテスト 1・説明書 1・比べる道具とテスト 2。議題 3 で ⑶（画面も直す）を取ると +1〜2。上限 20 の内。
- **危うさ: 中**。今ある部品を窓なしで呼ぶ所は低い。中にしているのは ⑴ runonce の時点（正典が always 等を言い切らない・SSP の測りが要る）⑵ 記録の取り込み（今の履歴が使えず新しい層が要る）⑶ スコープ 1 以上の既定の着せ替えが画面と正典で食い違う、の 3 つ。

## 9. 設計の段への申し送り

- 推しの形は **案 B（新しいモジュール）を骨にし、議題 3 の答え次第で案 C の共有を足す**。runonce は **R2**（同じスレッドで `handle_message` を回す公開の関数）が、画面と同じ道を通り、記録の取り込みも同じスレッドで済むので素直。ただし決めるのは要件の討議（上の議題 1〜16）。
- 設計の前に 7 節の 1〜4 を SSP で測ると、議題 1・2・3・5・6 の答えが測りで決まる。
- 触るファイルの見立て: 新 `crates/areka/src/offline_dump/`（`mod.rs`・`run.rs`・`surface_list.rs`・`bind_spec.rs`・`csv.rs`・兄弟のテスト）・新 共有の PNG の置き場（例 `crates/areka/src/png_image.rs`）・`crates/areka/src/mcp/dump_surface_image.rs` とそのテスト・`cli/mod.rs`・`cli/parse.rs`・新しい兄弟のテスト・`main.rs`（数行）・`crates/areka-seriko/src/`（新しいファイル 1 つ＋`lib.rs` の `pub use`）・`dist/README.txt`・比べる道具の置き場。読むだけ: `shell_target.rs`・`areka-emo-compose`・`emo2_boot/assets.rs`・`shell_balloon_resolve.rs`・`resolve_shell.rs`。
