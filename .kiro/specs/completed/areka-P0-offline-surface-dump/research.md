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

---

## 10. SSP の測り（2026-10-22・SSP 2.9.23 の --offline-dump）

### 10.0 測り方

- SSP: `C:\wintools\ssp\ssp.exe`（2.9.23.3000）。起動中の SSP には何も送っていない（SSP の MCP も使っていない）。
- 回し方（毎回同じ形・絶対パス）: `ssp.exe --option bootunlock,standalone,readonly --offline-dump "<検体>" --dump-output-dir "<出力>" --dump-error-log "<出力>\log.csv" <下の各項の指定>`。PowerShell の `Start-Process -PassThru` で起こし、30 秒の見張りで終わりを待って終了コードを取った。全 40 回ほど、どれも 1〜2 秒で自分で終わり、止まったものは無い。
- 検体と出力: ワークツリーの `target\offline-dump-survey\` の下（検体 `g0`・`g_noname`・`g_anim`・`g_time`・`g_size`・`g_scope`・`g_bind`・`g_bind10`・`g_warn`、出力は `out\<回の名前>\`）。絵はすべて 1 色塗りの RGBA の PNG（α 255）で、シェルの descript に `seriko.use_self_alpha,1` を書いた。結果は出た PNG の画素の値を読んで決めた（目では見ていない）。
- 出る PNG は RGBA 8 ビット（色の型 6）。何も描かれない所は `(0,0,0,0)`。
- 色の符号: 灰 `(128,128,128)`＝土台。赤 `(255,0,0)`・緑 `(0,255,0)`・青 `(0,0,255)`＝面 100・101・102。`g_time` の always は 40 コマ×10ms でコマ i の色を `(6i,0,200)`、APNG は 40 コマ×10ms でコマ i の色を `(0,6i,100)` にし、画素から何コマ目かを読めるようにした。
- **どの検体にも出る行**: 面 10 が無いシェルでは毎回 `"…","<名前>","Warning","[SERIKO] Surface=10 存在していません。"` が出て、終了コードが 1 になる（スコープ 0 だけを書き出しても出る）。どこからも指されていない面には `Notice` の `[SERIKO] Surface=110 どこからも使われていません。` の形の行が出る（Notice なので終了コードは 0 のまま）。終了コードを比べる回は、面 10 を足した `g_bind10` で確かめた。

### 10.1 runonce とほかの interval の時点（7 節の 1）

検体 `g_anim`（土台 16×8 の灰・runonce は左の 8×8、always は右の 8×8）:

```
surface0 { animation0.interval,runonce
           animation0.pattern0,overlay,100,100,0,0   ← 赤
           animation0.pattern1,overlay,101,100,0,0   ← 緑
           animation0.pattern2,overlay,102,100,0,0 } ← 青
surface1 { 同じ＋ animation0.pattern3,overlay,-1,100,0,0 }
surface2 { animation0.interval,always ＋ pattern0〜7（各 50ms・8,0 に 8 色） }
surface3 { animation0.interval,random,1 ＋ animation0.pattern0,overlay,100,100,0,0 }
surface4 { animation0.interval,periodic,1 ＋ 同じ }
surface5 { surface0 の runonce ＋ surface2 の always（animation1） }
```

指定 `--dump-surface-list 0-5`。結果（3 回回して 6 枚ともファイルのハッシュが同じ＝決定論）:

| 面 | 出た絵 |
|---|---|
| 0 runonce | 左が青（最後のコマが残る） |
| 1 runonce の最後が -1 | 灰だけ（土台に戻る） |
| 2 always だけ | 灰だけ（always は何も出ていない） |
| 3 random,1 | 灰だけ |
| 4 periodic,1 | 灰だけ |
| 5 runonce＋always | 左が青・右が always のコマ 5（`(160,200,50)`） |

細かく測るために `g_time`（always・APNG とも 10ms 刻み）で回した（`--dump-surface-list 0-6`・`7,8`・`9,11`）:

| 面 | 中身 | always | APNG | 読み |
|---|---|---|---|---|
| 0 | runonce 1 コマ（待ち 100） | コマ 9 | コマ 10 | 時点 100ms |
| 1 | runonce 1 コマ（待ち 250） | コマ 24 | コマ 25 | 時点 250ms |
| 2 | runonce なし | 出ていない | コマ 0 | 時点 0 |
| 5 | runonce 2 本（100 と 100＋200） | コマ 29 | — | 時点 300ms＝長いほう |
| 9 | runonce（待ち 400） | コマ 39 | コマ 39 | 時点 400ms（APNG はちょうど 1 周の時点で最後のコマのまま） |
| 11 | runonce（待ち 405） | コマ 39 | コマ 0 | 時点 405ms（APNG は 1 周を越えて頭へ戻る） |
| 7・8 | runonce（待ち 1000・450）＋APNG | — | コマ 20・コマ 5 | APNG は繰り返す（1000−800・450−400） |
| 3 | runonce 3 コマ×1000＋random,1 | — | — | random は出ていない |
| 4 | runonce 3 コマ×1000＋periodic,1 | — | — | periodic は出ている（緑） |
| 6 | runonce 3×100＋always（緑 100・-1 100） | 緑 | — | 時点 300ms で always の 2 周目の緑 |

- 読み取れる決まり: 書き出しの時点 T は「その面の runonce のうち、待ちの合計がいちばん長いものの合計」（runonce が無ければ 0）。pattern の待ちは **出す前に待つ時間**（コマ k は待ち 0〜k の合計の時刻に出る）。always と periodic は T まで進み、random は進まない（出ない）。
- **議題 1 への含み**: 5 節の R2（同じスレッドで `handle_message` を回す）で、時計を T まで刻み、always・periodic はそのまま進め、random は抽選を起こさない形にすれば SSP と同じ絵になる。areka の `frame_at` の待ちの数え方（出す前に待つか）が同じかは設計で確かめる。R0（経過 0）は面 0・1・5 で SSP と違う絵になる。

### 10.2 動く絵の element のコマ（7 節の 2）

検体 `g_anim`:

```
surface10 { element0,overlay,base16.png,0,0  element1,overlay,anim.png,0,0 }   ← APNG 赤100・緑100・青100ms
surface11 { 同じで anim0.png }                                                 ← APNG 赤0・緑100・青100ms
surface12 { element1,overlay,anim.png,8,0 ＋ surface0 の runonce }
surface13 { 同じで anim.webp }                                                 ← 動く WebP 赤100・緑100・青100ms
```

- 面 10: 赤（1 枚目）。面 11（1 枚目の待ち 0）: 赤（1 枚目のまま・飛ばさない）。面 13（動く WebP）: 赤（1 枚目）。面 12: APNG は青（T=300 がちょうど 1 周の長さなので最後のコマのまま・10.1 の面 9 と同じ）。
- `g_time` の面 0・1・7・8・9・11（10.1 の表）: APNG のコマは T の時点のもので、繰り返す。ちょうど 1 周の時刻では最後のコマ、それを越えると頭へ戻る。
- **議題 2 への含み**: 動く絵の子も議題 1 と同じ時点 T のコマ（runonce が無ければ 1 枚目）。areka の `push_film_op` は「1 枚目の待ちが 0 なら次のコマ」を描くが、SSP は待ち 0 の 1 枚目を描く＝ここは SSP と違う。

### 10.3 スコープごとの既定の着せ替え（7 節の 3）

検体 `g_scope`（土台 24×8 の灰・面 0 に bind のアニメーション 3 本: 10＝左の赤・11＝中の緑・12＝右の青）。シェルの descript:

```
sakura.bindgroup10.default,1  kero.bindgroup10.default,0  char2.bindgroup10.default,0
sakura.bindgroup11.default,0  kero.bindgroup11.default,1  char2.bindgroup11.default,0
sakura.bindgroup12.default,0  kero.bindgroup12.default,0  char2.bindgroup12.default,1
（名前は sakura・kero・char2 の 3 つとも bindgroup10〜12.name を書いた）
```

| 指定 | 出た絵 |
|---|---|
| `--dump-scope 0` | 赤だけ（`sakura.*` の既定） |
| `--dump-scope 1` | 緑だけ（`kero.*` の既定） |
| `--dump-scope 2` | 青だけ（`char2.*` の既定） |
| `--dump-scope 3`（char3 の定義なし） | 灰だけ（どれも着ない） |

- **議題 3 への含み**: SSP の書き出しはスコープごとの既定を使う（正典 `--dump-bind` の「sakura.bindgroup*.default」はスコープ 0 を例に書いたものと読める）。要件 3.9 の用語どおりで、画面（全スコープ `sakura.*`）とは違う。書き出しを SSP に合わせつつ要件 4.5（画面と同じ）を守るには ⑶（画面も直す）になる。

### 10.4 `--dump-bind` の細部（7 節の 4）

検体 `g_bind`（土台 56×8 の灰・8 マスに 1 パーツずつ）。シェルの descript:

```
sakura.bindoption0.group,head,mustselect
sakura.bindoption1.group,acc,multiple
cloth: uniform（既定 1・赤）casual（0・緑）swim（0・青）   ← bindoption なし＝排他
head : hat（1・水色）ribbon（0・赤紫）                    ← mustselect
acc  : a（1・黄）b（1・白）                               ← multiple
```

既定（指定なし）: uniform・hat・a・b を着ている。各回 `--dump-surface-list 0 --dump-bind "<指定>"`:

| 指定 | 着ているもの | 記録 | 終了コード（`g_bind10`） |
|---|---|---|---|
| `cloth,__ALL__,1` | swim だけ（cloth の最後のパーツ） | なし | 0 |
| `cloth,,1` | swim だけ（`__ALL__` と同じ） | なし | — |
| `cloth,casual`（ON/OFF 省略） | casual（uniform は外れる） | なし | — |
| `cloth,uniform,0` | cloth は何も着ない | なし | — |
| `head,hat,0`（mustselect の OFF） | hat のまま（変わらない） | **なし** | 0 |
| `head,__ALL__,0` | hat のまま | なし | — |
| `head,ribbon,1` | ribbon（hat は外れる） | なし | — |
| `acc,__ALL__,0` | a・b とも外れる | なし | — |
| `acc,a,false;cloth,swim,true` | b と swim（`false`・`true` と `;` が効く） | なし | — |
| `cloth, casual,1`（前に空白） | 変わらない | `"…","g_bind","Warning","[DUMP] bind cloth, casual 存在していません。"` | — |
| `cloth,casual ,1`（後ろに空白） | 変わらない | `"…","g_bind","Warning","[DUMP] bind cloth,casual  存在していません。"`（空白 2 つ） | — |
| `nosuch,x,1`（無いカテゴリ） | 変わらない | `"…","g_bind","Warning","[DUMP] bind nosuch,x 存在していません。"` | 1 |
| `cloth,nosuchpart,1`（無いパーツ） | 変わらない | `"…","g_bind","Warning","[DUMP] bind cloth,nosuchpart 存在していません。"` | — |

- 「—」の回は `g_bind`（面 10 なし）で回したので、終了コードは Surface=10 の Warning で 1 になり、指定の分は読めない。
- 排他のカテゴリで `__ALL__`（空欄）を ON にすると、パーツを書いた順に 1 つずつ ON にした結果になり、最後のパーツだけが残る。
- **議題 5 への含み**: SSP は mustselect の OFF を **黙って無視** する（Warning も出さず終了コード 0）。画面の `\![bind]`（無視）と同じ。要件 5.4（OFF にする）は SSP とも画面とも違う。
- **議題 6 への含み**: 排他で `__ALL__` を ON は「最後のパーツだけ」。空白は削らず、名前の照合に失敗して「無い」の Warning になる（文には空白がそのまま入る）。

### 10.5 CSV の細部（7 節の 5）

ファイルは先頭が BOM（`EF BB BF`）、改行は CRLF。1 回の走りの行は時刻がすべて同じ秒だった。

| 回 | 指定 | 終了コード | PNG | CSV の行（逐語） |
|---|---|---|---|---|
| シェルが無い | `g0` に `--dump-shell nonexist` | 2 | 出ない | `"22:38:18","g0","Warning","[SERIKO] Surface=10 存在していません。"`・`"22:38:18","g0","Error","[DUMP] shell nonexist 存在していません。"` |
| 単独の番号の面が無い | `g0` に `--dump-surface-list "0,99,5-7"` | 1 | surface0 だけ | `"22:38:22","g0","Warning","[SERIKO] Surface=10 存在していません。"`・`"22:38:22","g0","Warning","[DUMP] surface99 存在していません。"`（範囲 5-7 の欠番は行なし） |
| element の絵が無い（面 0 に 2 か所・面 1 に 1 か所） | `g_warn` に `--dump-surface-list "0,1"` | 2 | 2 枚とも出る | `"22:38:24","g_warn","Error","[SERIKO] shell\master\surfaces.txt:Line=10:Surface=0 Element0 存在していません。:nofile.png"`・同じ形で `Line=11:Surface=0 Element1`・`Line=16:Surface=1 Element0`・Surface=10 の Warning |
| 別のシェル（フォルダ名） | `g_warn` に `--dump-shell second` | 2 | 出る | 上と同じ 3 行で `shell\second\surfaces.txt` |
| 面の指定なし | `g_warn`（`--dump-surface-list` なし） | 2 | 出ない | 上と同じ 4 行（記録は出る） |
| ゴーストのフォルダが無い | 無いフォルダ | 0 | 出ない | CSV のファイルも作られない |

- 同じ中身の行が 2 度出ることは無かった（どの行も 1 度ずつ。シェルを 2 度読む・スコープの数だけ読むといった重なりは SSP の CSV には現れない）。element の絵が無いのは Error だが、その面の書き出しは続く。
- **議題 8 への含み**: SSP の CSV に同じ行の重なりは無い。areka で面の表を 2 度組むと同じ warn が 2 行になる（2.3）ので、組む回数を 1 度にするか、同じ行を 1 つにする必要がある。

### 10.6 絵の大きさ（7 節の 6）

検体 `g_anim` の面 20〜23（土台 8×8 の灰）:

```
surface20 { element0,overlay,base8.png,0,0  element1,overlay,red4.png,-2,-2  element2,overlay,green4.png,6,6 }
surface21 { base8＋green4 を 6,6 }   surface22 { base8＋red4 を -2,-2 }
surface23 { base8＋runonce の 8×8 を 4,4 }
```

- 結果: **4 枚とも 18×14**。しかも同じシェルの面 0〜5・10〜13（土台 16×8 ではみ出しなし）も **すべて 18×14** で、絵は (2,2) からずらして置かれ、周りは透明。
- 18×14 は、シェルの全部の面の element と animation の pattern の外形を合わせた範囲（x −2〜16・y −2〜12）に一致する（y の 12 は面 23 の runonce の pattern 4+8 から来る）。
- 確かめ（検体 `g_size`）: 面 0 は 8×8 の土台だけ、面 1 は −2,−3 に 4×4、面 2 は `interval,never` の pattern を 6,7 に 4×4。**面 0 だけ** を書き出しても 12×14（x −2〜10・y −3〜11）で、土台は (2,3) に置かれた。never のアニメーションの pattern も範囲に入る。
- はみ出しの無いシェル（`g0`・`g_scope`・`g_bind`）は土台の大きさのまま（4×4・24×8・56×8）。
- **含み（6 節に無い新しい議題）**: SSP の書き出しの大きさと置き場所は **シェル全体で 1 つ**（全部の面の element と pattern の外形の和）。areka の合成は面ごとに広がる（`extent-element-offset`）ので、はみ出しのある面を持つシェルでは、はみ出しの無い面も含めて大きさと位置がずれる。要件 13（比べる道具）と下流の `pasta-in-windows-verification` は、この差を知って比べる必要がある。

### 10.7 そのほか

- **頭の文字列に `\`**（`--dump-output-prefix "sub\x_"`）: 出力の下に `sub` が **無い** と、PNG は出ず、CSV にも何の行も出ない（黙って書けない・終了コードは他の行で決まる＝この回は Surface=10 の Warning で 1）。`sub` を先に作っておくと `sub\x_0.png` に書かれる。→ **議題 12**: SSP は下のフォルダへの書き出しを許し、フォルダを作らず、失敗を記録しない。areka が要件 6.6 どおり Error にすると、SSP より厳しい側に倒すことになる。
- **`name` の無いゴースト**（`g_noname`）: CSV のゴースト名の列は空（`"22:38:30","","Warning","[DUMP] surface99 存在していません。"`）。→ **議題 13**: SSP は空。フォルダ名で埋めない。
- **`--dump-shell` をフォルダ名で**（`second`）: そのシェルで書き出した。`random`・`lastinstalled` は測っていない（議題 9 は残る）。

---

## 11. 設計の段の調べと決めたこと（2026-10-22・`/kiro-spec-design -y`）

- 調べの種類: 軽い調べ（既存の部品へ足す拡張）。外の調べ（ウェブ）は要らない（新しい外の依存なし・`image` はワークスペースに既在の 0.25.10）。使った手引き: `kiro-spec-design` の規則（design-principles・design-discovery-light・design-synthesis・design-review-gate）と steering の `structure.md`・`logging.md`・`tech.md`。
- コードの調べ（読むだけ・2 つの調べ係に分けた）:
  - areka-seriko: `handle_message`（`actor.rs:432`・crate の中・同期）・`LoopRuntime`（`pub(crate)`）・`SerikoClock`（注入の時計）・`LoopRng`（注入の乱数・`should_fire = rng(k)==0`・1000ms の境で random と BindRandom だけが使う）・periodic は乱数を使わず `lap_of` で決まる・1 回の刻みは最後の周だけを鳴らすので、本番の刻み 16ms（`areka-ghost/src/ticker.rs:262`）で進めないと periodic の絵が変わりうる・`frame_at` はコマ k を待ち 0〜k の合計の時刻に出す（SSP の測りと同じ数え方）・`states.current_pattern(scope, Slot::Shell)` が公開・`AnimationTable` は `Clone`（作り直すと表の warn が重なる）・テストの `ClockRig`（`actor_test_support.rs`）が同じ順で送っている。
  - 着せ替え: `ScopeStates::current_binds` は動的な組が無ければ 1 つの `static_binds` へ後退（全スコープ共通）・`default_bind_ids` は `sakura.*` だけ・`scope_namespace` は 2 以上で `None`・areka-parsers の `read_bindgroup_defaults` は `kero.*` の既定も読んでいるが使われていない・`parse_kv` は値の前後の空白を取る（`default_bind_ids` の `trim` と同じ）・`parse_kv` は `BTreeMap` で書かれた順を失う（`--dump-bind` の `__ALL__` の順のために `BindGroupName::line` を足す）。
  - シェル: `load_shell_target` は中で面の表（probe）を組んで捨て、`build_world` でもう 1 度組む（畳み込みの warn が 2 度）・probe は `build_world` と同じ入力から組む・`Composer` の絵は原点 (0,0) 固定で負の座標を切り捨て、大きさは面ごとで `never`・`runonce` を数えない（`compute_extent` は `pub(crate)`）・動く絵の子は `PatternState` の欄でコマが決まり、既定なら経過 0 のコマ（1 枚目の待ちが 0 なら 2 枚目）。
  - 入口と記録: `--help` の直後なら記憶・MCP・窓に触れない・`log_history` の履歴は分まで・1,000 件・4,096 字・本番で受け口を差し替える `with_default` などは見張り（`with_default_guard_test.rs`）が禁じる・履歴の層は warn 以上の出来事に `always` を返しているので、3 つ目の層を足しても受け口の関心の判定は変わらない。

### 決めたこと（6 節の議題のうち要件で決まらなかったもの）

| 議題 | 決めたこと | 理由 |
|---|---|---|
| 7 記録の取り込み | `log_history::init` に 3 つ目の層（写しの窓）を足し、`capture_begin`／`capture_finish` で開け閉めする | 本番で受け口を差し替えない（見張りの外に出ない）・同じスレッドで走るので届く・本文は履歴と同じ組み方を切らずに使う |
| 8 同じ記録の重なり | 面の表を 1 度だけ組む（`ShellTarget::take_world`）・`AnimationTable` を 1 度だけ作る・下の層が `error!` を出した失敗に上の層は info だけ重ねる | 「1 つの誤りは 1 行」（要件 7.5）を、行を消す後始末でなく組む回数で守る |
| 10 値の解釈の置き場 | `cli` は生の文字列を持つだけ・解釈と Warning は窓の中の `offline_dump` | 要件 7.3（値の扱いは CSV に入れ、名前の読みは入れない）を順番で満たす |
| 11 終了コードの返し方 | `main` は型を変えず `std::process::exit(offline_dump::run(..))` | 足すのは 3 行・ほかの早い戻りに響かない・`exit` は標準出力を流してから終わる |
| 14 比べる道具の置き場 | 新クレート `crates/surface-compare/`（lib＋bin・`publish = false`） | 要件 13.5 を Rust の決定論のテストで書ける・`image` はワークスペースに既在・SSP の PNG を乗算していない RGBA で読める |
| 15 説明書の例 | `start "" /wait areka.exe --offline-dump …` と `echo %ERRORLEVEL%` | リリースの exe は窓の exe で、コマンドプロンプトは終わりを待たない |
| 16 シェルの組み立ての共有 | 既定の組と名前の表だけを共有（`DefaultBinds::from_groups`・`BindResolver::from_groups`）。シェルの資産の組み立て一式（`build_shell_assets`）は共有しない | 要件 14.3 は着せ替えの決め方の一致を求める・`build_shell_assets` は箱やフォントなど書き出しに要らないものを作る |
| runonce の時点（R2） | areka-seriko に `settle_surface`（同じスレッドで `handle_message` を回す・16ms ごとの刻み・乱数は当たらない列）と `settle_time` を足す | 画面と同じ道（要件 4.5）・スレッドを起こさないので写しの窓に届く |
| T の数え方 | その面の surface*ブレスに書かれた runonce の待ちの合計の最大。部品の runonce は T を決めない | 要件 4.1「その面の runonce」・SSP の測りは一番上の runonce だけ |
| 下地 | areka-emo-compose に `shell_bounds`（符号つき・全 interval・負の番号と描かないメソッドは数えない）を足し、組んだ絵を `(-left,-top)` に置く | 要件 4.3・4.5（合成そのものは変えない） |
| 要件 4.8 の読み | 「面の組み立てに失敗」＝`Composer::compose` が `Err`。画像が読めないのは読み込みの Error で、面は書き出す | 画面と同じ絵（4.5）・SSP 2.9.23 も Error を残して書き出す（§10.5） |
| `--dump-shell` の照合 | `resolve_skin_target` の「名前 → フォルダ名」の段を `find_by_name_or_folder` に切り出し、特別な名前を解かずに呼ぶ | 要件 2.4（同じ決まり・`random`／`lastinstalled` はただの名前） |
| ゴーストの名前 | areka-parsers に `read_ghost_names` を切り出す | `--dump-shell` が見つからなくても CSV のゴースト名が要る・`ShellDirMissing` は名前を持たない |
| `--dump-*` の値なし | `--offline-dump` があるときは `MissingValue`（warn・`[cli]`）を出し、値なしとして扱う | 読み方の決まり（空の値は値なし）のまま・黙って捨てない |
| `assets.rs` の shell の descript.txt | 読めないときの `BootWiringError::ShellRead` は今の起動の約束として残す（既定の組は共有の決め方から作る） | 画面の起動の失敗の扱いを、この spec の範囲で変えない |
| 書き出しの順 | 面の番号の昇順 | 重ねの除去と同じ集まりから自然に決まる・正典は順を言わない |

### 合成の見直し（design-synthesis）

- 一般化: 「面 1 つを、見せ始めてから時刻 T まで画面と同じ道で進めた状態」は `settle_surface` 1 つで、runonce・always・periodic・部品・動く絵の子をまとめて扱う（それぞれの写しを持たない＝5 節の R3 を採らない）。「着せ替えの名前で 1 パーツを当てる」は `apply_named_bind` 1 つで、画面の `\![bind]` と `--dump-bind` が共有する。
- 作るか採るか: PNG は今の手書きの関数を共有の場所へ動かすだけ（新しい依存なし）。比べる道具の PNG の読みは `image`（既在）を採る（PNG のフィルタと乗算していない色を自前で書かない）。CSV は標準の文字列で書く。
- 削ったもの: 書き出しのための別のシェルの組み立て一式・CSV の重ねの除去の後始末・記録の受け口の差し替え・`--dump-shell` の特別な名前の扱い・CSV の件数の上限。

### 危うさと手当て

- スコープ 1 以上の初めの着せ替えが画面で変わる（`kero.*` を書いていないシェルの相方は何も着ない）→ 正典どおり。Revalidation Triggers に書き、14.4 のテストで画面と書き出しの両方を固める。
- `static_binds` の型の付け替えが `emo2_boot` の 6 ファイルに及ぶ → parsers → seriko → 画面 → 書き出しの順に段を分ける。
- 合成が負の座標を切り捨てるので、そういう面は SSP の絵と左上の切れ端が違う → `extent-element-offset` の範囲。比べる道具は差の数を出すだけで、許す大きさは使う試験が決める。
- 写しの窓はプロセス全体 → 書き出しのあいだにスレッドを起こさない（今の部品は呼んだスレッドの上だけで走る）。

## 12. 設計の討議で決めたこと（2026-10-22）

- 同じ誤りが CSV に何行も出る件（検証の重い指摘 1）: 合成の側で直す（ShapeIssuesOnce）。定義だけで決まる形の誤り（入れ子の循環・知らない animation-sort・描かない method）は読み込みのときにシェルごとに 1 度だけ warn で出し、面を組むたびの同じ知らせは debug に下げる。写しの層でまとめる案（同じ本文の行を 1 つに）は、読み込みと合成で文が違う重なりが残り、画面の積み重なりも直らないので採らない。要件 7.5 を狭める案は SSP（重なりなし）と食い違うので採らない。開発者が推奨案で承認。

