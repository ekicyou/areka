# Gap Analysis: areka-P0-default-balloon-bundle

> 2026-09-18 に本ブランチ（`claude/areka-p0-balloon-bundle-6fe55e`・`f3ca5379`）で実測。file:line と件数は着手時に引き直すこと。
> 本文は「情報と選択肢」であって決定ではない。決定は要件ディスカッションと設計に委ねる。

## 0. 要約

- **本番コードの変更 0 行で全要件が到達可能。** バルーンの起動経路（`resolve_balloon_faces` → `build_balloon_target_from_faces` → `load_scope_balloon_model`）は `areka-emo-present` の**公開 API** で、`areka-emo-text` は `areka-emo-present` を通常依存に持つ。よって新規テストは `crates/areka-emo-text/tests/` の**新規ファイルだけ**で、`Cargo.toml` を触らずに「起動時の焼き込みと同じ経路」を踏める。
- **候補の実物は本文の記述とほぼ一致するが食い違いが 1 件ある**（要件 2.6 の発動条件）: 実物の `descript.txt` には brief／要件書に無い **`paint_transparent_region_black,0`** が在る。ファイル一覧 29 本・LICENSE（CC0 1.0 Universal 法典本文）・`install.txt` の `directory,StayseeBalloon` は一致。最新コミットは `fe1b02f3`（2021-11-05「use_input_alpha」）で、readme の更新履歴（v1.00A 2020/6/27）より後にコミットが 2 つある。
- **候補の descript は `wordwrappoint` と `font.name` を宣言しない。** 折返し基準は `validrect.right` へ縮退（`region.rs` の `resolve_or`・`debug!`）、フォントは `ＭＳ ゴシック` 既定（`draw.rs` の `DEFAULT_FONT_NAME`）へ縮退する。既存の実フォント檻はすべて `Yu Gothic UI` 前提で書かれており、そのまま写せない。要件 3.5 の「`wordwrappoint` を超えたら折り返し」は実物では「`validrect.right` を超えたら折り返し」と同義になる。
- **面の系列が既存検体より 1 段複雑**: `balloons0〜3`（4 面）＋`balloonk0〜1`（2 面）。scope 1 は面 2・3 を本体側へ縮退させ **`warn!` が 2 件出る**（`resolve_balloon_faces` R6.2 の正規動作）。要件 3.8 の「`error!` 0 件」は成り立つが、warn の件数を檻で固定するか否かは設計判断。
- **文書側の変更は台帳 3 文書＋§8 1 行＋brief 3 本**で、番人（`spec_checks.rs` 腕 a〜f）の条件はすべて満たせる形が既に在る（`status = "degraded"` は 10 件の前例・`wave` は自由文字列・束「絵の重ね方」実在・`[[spec]]` 名は本 spec ディレクトリが実在）。

規模 **S**（3 日以内・既存流儀の踏襲のみ）／リスク **Low〜Medium**（COM＋WIC を踏む決定論テストと、実物の PNG／フォントに依存する期待値の較正が唯一の不確実点）。

## 1. 現状の資産（Requirement-to-Asset Map）

### 1.1 バルーン起動経路（本番・変更しない）

| 段 | 定義点 | 本 spec との関係 |
|---|---|---|
| 起動引数 | `crates/areka/src/boot_config.rs` `resolve_config_inputs`（`args[2]` がバルーンの根） | 実機目視はフォルダを置いて argv で起動＝コード変更 0 |
| 面の列挙・解決 | `crates/areka-emo-present/src/balloon.rs` `enumerate_file_names`（全名を 1 回走査・面判定なし）→ `select_faces`（純核）→ `resolve_balloon_faces`（面 0 必在＝`error!`＋`Err`・scope≧1 の縮退面は面ごとに `warn!`・完了時 `info!`） | 要件 3.4／3.8 の観測点。`balloonc*`・`arrow*`・`online*`・`marker.png`・`sstp.png`・`thumbnail.pnr`・`LICENSE` は連鎖の接頭辞に当たらず**無音で**列挙から外れる（記録も無い＝「載らないこと」は列挙結果の不在で固定する） |
| 焼き込み | 同 `build_balloon_target_from_faces`（synthetic surfaces.txt → `shell::parse` → `bake`・`UseSelfAlpha::On` 固定） | 要件 3.4 の α 保持は `AtlasTable::page(...).bytes`（premultiplied BGRA）で α<255 の画素が実在することで固定できる |
| 定義の 2 層マージ | 同 `load_scope_balloon_model`（`descript.txt` 基層＋`{接頭辞}{ID}s.txt`・`decode(&bytes, DefaultEncoding::Ansi)` で charset 宣言を尊重・確定値を `info!`） | 候補に `s.txt` は無い＝上書き層不在は `debug!`（正常縮退） |
| descript の読み手 | `crates/areka-parsers/src/balloon/parse.rs`（`windowposition.*`・`origin.*`・`wordwrappoint.*`・`validrect.*`・`font.*`・`disable.font.*`・`cursor.*`・`vertical`・`writing_mode`・`budoux_newline`） | **`type`・`id`・`name`・`craftman`・`homeurl` は `BalloonModel` に無い**（→ §2.1 Gap-1） |
| 汎用 kv | `crates/areka-parsers/src/kv/parse.rs` `parse_kv(&str) -> BTreeMap<String,String>`・`charset::decode` | `id`／`name`／`type`／`install.txt` の `directory` を本番コードに触らず読む口 |
| 起動側の束ね | `crates/areka/src/emo2_boot/assets.rs`（上の 3 関数を scope ごとに呼ぶ＋`balloon_background::face_origin_color`） | `face_origin_color` だけは bin クレート内部で外から呼べない（→ §2.1 Gap-4） |
| 文字領域の解決 | `crates/areka-emo-text/src/region.rs` `TextRegion::resolve(model, image_size, mode)`（負値＝反対辺基準・`wordwrappoint` 未指定は遠辺へ縮退＝`resolve_or`） | 要件 3.5／3.7 の観測点 |
| フォント | `crates/areka-emo-text/src/draw.rs` `ResolvedFont::resolve`（`font.name` 欠落→`DEFAULT_FONT_NAME`＝`ＭＳ ゴシック`・ログなし） | 候補は `font.name` 無し＝**既定フォント経路**を初めて実物で踏む |
| DPI 追従 | `region.rs` `ScaleContract::new(k, author_dpi)`・レイアウトは image px（k 非依存）・物理化は一点 | 要件 3.7 は headless 構造テスト（`scale_invariance_test.rs` の形）で足りる |

### 1.2 既存の検体とテストの置き方

- 検体は 3 つ（`fixtures/emo2/emo2-kakukaku`・`fixtures/emo2-kakukaku-offsetdpi`・`fixtures/emo2-kakukaku-wplimit`）。いずれも `charset,UTF-8`・`font.name,Yu Gothic UI`・`font.height,28`・面は `balloons0`／`balloonk0` の各 1 枚＋`s.txt` 上書き層あり・`validrect` は基層で全 0（上書き層で成立）。**候補（Shift_JIS・既定フォント・12px・面 4＋2・上書き層なし・基層で validrect 成立）は既存検体と直交する性質が多い**＝既存テストの期待値を写す形にはならない。
- 検体パスを参照する既存ファイルは実測 **56 本**（`.rs`・`.md`・`.ps1`・`.py`・`.toml`。うち `.rs` は 46 本。nar-install brief の「38」はテスト・example のみの数え方で、名前の揺れは着手時に nar-install 側の一覧で引き直すこと）。**新規テストは `crates/areka-emo-text/tests/` に新しい名前で足す限りこれらと共有 0**。
- 既存の実物 fixture テストの型（すべて `crates/areka-emo-text/tests/`）:
  - `shipped_fixture_region_test.rs`（397 行）: 2 層マージ→`TextRegion::resolve` の全成分を逐語固定＋PNG ヘッダから原寸を自前で読む（`png_native_size`・依存追加なし）。
  - `kero_menu_capacity_test.rs`（820 行）: 実 descript＋実 pasta 台本→`areka_parsers::sakura::parse`→`areka_sakura::compile`→`apply_cue`→`LayoutEngine::layout`（実 `DWriteMetrics`・GPU 不要）で行数・折返し・`\_l` の列を固定。先頭で「実フォントが在ること」を門にする。
  - `choice_fixture_test.rs`／`emo2_fixture_e2e_test.rs`／`line_pitch_readback_test.rs`: headless GPU で `present_frame`→`read_back`→画素檻＋PNG 保存（目視証跡）。
  - `scale_invariance_test.rs`: `FixedMetrics` 注入・k=1.25／2.0／k<1 の写像と k 非依存の構造檻。
  - `vertical_fixture_test.rs`／`choice_fixture_test.rs`: **test-local fixture** を `tests/fixtures/<名>/` に置く前例（本 spec は `vendors/sample_ghost/` を指すのでこの形は使わない）。
- WIC で実 PNG を復号する決定論テストの前例: `crates/areka-emo-atlas/src/emo2_e2e.rs`（`CoInitializeEx(None, COINIT_MULTITHREADED)` を自前で張る `with_com_initialized`・`WicDecoderArm::new()`）。`WicDecoderArm` は `areka_emo_atlas` の root から公開。
- `areka-emo-present` に `tests/` ディレクトリは**無い**（in-crate 兄弟テストのみ・親 `balloon.rs` に接続宣言が要る＝本 spec は触らない）。`areka` は bin クレートで `crates/areka/tests/` から内部へ届かない（`emo2_real_run.rs` は exe を起動する形）。

### 1.3 依存関係（新規テストの置き場を決める材料）

| クレート | `[dependencies]` に在る | `[dev-dependencies]` に在る |
|---|---|---|
| `areka-emo-text` | `areka-parsers`・`areka-sakura`・**`areka-emo-present`**・`windows`・`wintf`・`dola` | `areka-emo-atlas`・`areka-emo-compose`・`log-capture-kit`・`tracing-subscriber` |
| `areka-emo-present` | `areka-parsers`・`areka-emo-atlas`・`areka-emo-compose`・`windows`・`wintf` | `windows`・`log-capture-kit` |
| `areka-emo-atlas` | — | `log-capture-kit` |

→ `crates/areka-emo-text/tests/<新規>.rs` からは、系列解決・焼き込み（WIC・COM）・2 層マージ・文字領域・レイアウト・sakura 台本・ログ捕捉のすべてに **`Cargo.toml` 変更 0** で届く。

### 1.4 保管慣行

- `vendors/sample_ghost/.gitattributes`（`* -text`）は下位フォルダへ効く。実測: `git check-attr text vendors/sample_ghost/StayseeBalloon/LICENSE` → **`unset`**（フォルダ未作成でも規則が当たる）。対照: `fixtures/emo2/emo2-kakukaku/descript.txt` は `unspecified`（無防備）。
- リポジトリ直下 `.gitignore` は `*_test.txt`／`*_dump.txt`。候補の 29 本の名前に大小無視で当たるものは **0 件**（`install.txt`・`readme.txt`・`descript.txt` はいずれも `_test.txt` で終わらない）。`vendors/sample_ghost/.gitignore` の打ち消しは既に在るので追加不要。
- 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の走査は `crates/**/*.rs`（`vendors/`・`target/` 除外）＝新規テストは対象・保管フォルダは対象外。例外表の追加は「表と件数の 2 箇所」の明示編集が要るが、本 spec は足さない。

### 1.5 文書側の受け口

- 台帳 `doc/ukadoc-coverage/ledger/assets.toml`: 対象項目 `ukadoc:descript_balloon:use_self_alpha_2c_5024:1`（`status = "absent"`・`owner = ""`・`priority = "A15"`・note は現状の「黙って壊れる」記述）。`status = "degraded"` の前例は **10 件**（`absent` 410・`implemented` 49・`vocabulary-only` 69・`alias` 4）。隣の `use_input_alpha`・`paint_transparent_region_black` は触らない（要件 6.3）。⚠ note には「areka-emo-present の balloon::read_descript_layer と areka の placement::load_balloon_author_dpi の 2 つだけが定義を読む」など実装名の記述が既に在り、改訂後も実在名で書くこと。
- `doc/ukadoc-coverage/roadmap-draft.md`: `[[spec]]` 27 行（`name`／`stage`／`bundle`／`owner_count`／`wave` の 5 欄）・`[briefs].count = 27`・`snapshot_on = "2026-09-13"`。`wave` は `String`（`documents/parse.rs` `string_field`）で語彙検査なし＝`"A0"` を書いても腕は赤にならない。
- `doc/ukadoc-coverage/linkage.md`: `[bundle."絵の重ね方"]` は実在し、対象 4 項目（`overlay_outside_balloon`・`paint_transparent_region_black`・`use_input_alpha`・`use_self_alpha`）を members に持つ。
- 番人 `crates/ukadoc-survey/tests/consistency/spec_checks.rs`: 腕 a（count＝行数）・b（名前が `.kiro/specs/` 直下か `completed/` に実在）・c（`owner_count`＝台帳 4 本の数え直し）・d（束名が linkage に実在）・e（完了 spec の宛先）・f（宛先の行き先が `[[spec]]` か `[[owner_completed]]`）。本 spec 1 件を owner にすれば `owner_count = 1`。
- 報告の作り直し: `cargo run -p ukadoc-survey -- report`／`report-summary`（`doc/ukadoc-coverage/README.md`）。
- `doc/COMPAT_ARCHITECTURE.md` §8: 4 列表・バルーン系の裁量行が 8 行既在（`areka-P0-kero-balloon` 出典）。透過の扱いの行は 0。
- `verification/` の慣行: 完了 spec 9 本に実在（`signoff-record.md`・`boundary-record.md`・`acceptance-record.md` 等。冒頭に対象仕様・対象要件・実施日・「0. 結論（先に）」）。
- `THIRD-PARTY-NOTICES.md` は `cargo about generate about.hbs` の生成物（先頭に明記）＝触らない。

### 1.6 下流 brief の現状

- `areka-P0-nar-install/brief.md`: 2026-09-18 追記で「畳む対象に `vendors/sample_ghost/StayseeBalloon/` を含める・共有ヘルパの検体名は emo2・R_POST_and_KOMAINU・StayseeBalloon・派生 2 つ」と**既に書かれている**。要件 7.3 の申し送りは「保管が終わった事実と最終のファイル一覧」を 1 段足す形。
- `areka-P0-baseware-root-layout/brief.md`・`areka-P0-alpha-release-signoff/brief.md`: 裁定候補 ⑴ を本 spec へ移した旨は記載済み。既定 id と README 文は未記載。
- 3 本とも `.kiro/specs/` 直下に実在（要件 7.4 の `completed/` 分岐は現時点で発動しない）。

## 2. 要件ごとの実現可能性と欠落

### 2.1 欠落・未知・制約（タグ付き）

| # | 種別 | 内容 | 影響する要件 |
|---|---|---|---|
| Gap-1 | **Missing** | `BalloonModel` は `type`／`id`／`name` を持たない。要件 3.3 の「宣言どおりに読み取り」を `BalloonModel` で満たすには本番変更が要る（要件 8.1 と衝突）。代替は `charset::decode`＋`kv::parse_kv` で同じバイト列を読む（本番コードが `id` を消費する場所は現状 0＝読んでも使い道は下流の `baseware-root-layout`） | 3.3・7.1・8.1 |
| Gap-2 | **Constraint** | 候補は `wordwrappoint` を宣言しない。`TextRegion::resolve` は `validrect.right`（scope 0: 335−26＝**309**）へ縮退し `debug!` を 1 件出す。要件 3.5 の観測は「soft＝hard」の 1 本になる | 3.5・3.7 |
| Gap-3 | **Constraint** | 候補は `font.name` を宣言しない→`ＭＳ ゴシック` 12px。既存の実フォント檻の門（「あ」の送りが em 未満＝`Yu Gothic UI` 前提）は使えない。行送りは 12＋2＝**14**（`line-height-canon` の式）。既定フォントの盲点（記憶 emo-text-byte-equiv-default-font-blindspot）は「既定と同じ描画になっても気付かない」型なので、候補では逆に**既定フォントそのものが正**であり盲点は無いが、フォント名を固定する檻が要る | 3.5・3.6・4.1 |
| Gap-4 | **Constraint** | `balloon_background::face_origin_color`（bin クレート内部）は外から踏めない。候補の (0,0) 画素が透明なら白へ縮退（`debug!`）し、`\f[disable]` の文字色が白寄りになる。表示崩れではなく色味の問題で、実機目視（要件 4）でしか見えない | 4.1・3.9 |
| Gap-5 | **Unknown** | 候補の PNG 原寸は `balloons0.png` **335×205**・`balloonk0.png` **335×135**（IHDR 実測・RGBA 8bit）。`balloons1〜3`・`balloonk1` の原寸と、面 0 の (0,0) の α は未確認。取得後に PNG ヘッダで実測し、期待値（validrect＝(22,20)-(309,158)／相方 (22,20)-(309,88)）を較正すること | 3.3・3.5・3.7 |
| Gap-6 | **Unknown** | 実物の `descript.txt` に **`paint_transparent_region_black,0`** が在る（brief／要件書の候補記述に無い）。要件 2.6 の「本文を実測に合わせて是正」が発動する | 1.1・2.6 |
| Gap-7 | **Unknown** | 最新コミット `fe1b02f3`（2021-11-05・「use_input_alpha」）は readme の更新履歴（v1.00A 2020/6/27）に載っていない。記録に書く「版」は readme の版とコミットの両方を並記する必要がある | 2.3・5.1 |
| Gap-8 | **Constraint** | scope 1 の系列は `balloonk0`・`balloonk1`＋縮退 `balloons2`・`balloons3` で `warn!` 2 件（R6.2 の正規動作）。要件 3.8 は `error!` 0 件のみを求める。warn を「期待 2 件」で固定するか「無視」かは設計判断（固定すれば面の増減で赤くなる） | 3.4・3.8 |
| Gap-9 | **Constraint** | 「`nar-install` が書き換える検体参照ファイル」は `.rs` 46 本（`.md`／`.ps1`／`.py`／`.toml` 込みで 56 本）。brief の 38 と数え方が違う。新規ファイルだけ足せば数え方に依らず共有 0 | 8.3 |
| Gap-10 | **Constraint** | 要件 3.2「検体パスは 1 か所の定数」と「複数の新規テストファイル」は両立しにくい（`tests/` の各ファイルは独立クレート）。1 ファイルに収める（≤1,000 行）か、`tests/<domain>/common/mod.rs` の `#[path]` 共有（structure.md の規約）か | 3.1・3.2・3.11 |
| Gap-11 | **Constraint** | COM＋WIC の実復号は `CoInitializeEx` を各テストスレッドで張る（`emo2_e2e.rs` 型）。`cargo test` の並列スレッドでの二重初期化は `let _ =` で許容している前例に倣う | 3.4 |
| Gap-12 | **Missing** | 実機目視の証跡（要件 4.2）: GPU 合成窓はスクショ不可（記憶）。アプリ側に読み戻し画像を保存する既存の口は無い（`AREKA_*` 環境変数に画像保存は無い）。headless の `read_back`→PNG 保存は `choice_fixture_test.rs` に前例があるので、決定論テスト側で PNG を残し、実機は観察記録（日付・ゴースト・k・所見）で足す | 4.2 |

### 2.2 要件別の到達性

| 要件 | 到達性 | 使う資産 |
|---|---|---|
| 1（選定・採否） | 文書＋実機目視。コード無し | `boot_config.rs` の argv 経路 |
| 2（保管） | フォルダ作成のみ。`.gitattributes` 継承実測済み・`.gitignore` 衝突 0 | `vendors/sample_ghost/` 慣行 |
| 3.3（descript の読み） | `BalloonModel` で `validrect`／`font.height`／`vertical` 未指定は固定可。`type`／`id`／`name` は **Gap-1** | `balloon::parse_str`・`kv::parse_kv` |
| 3.4（面 0・α 保持） | `resolve_balloon_faces`（scope 0／1）＋WIC bake＋`AtlasTable::page` の α 走査 | Gap-8・Gap-11 |
| 3.5（折返し・validrect 内） | `TextRegion::resolve`＋`LayoutEngine::layout`（実 `DWriteMetrics`・`ＭＳ ゴシック`） | Gap-2・Gap-3 |
| 3.6（`\q`・`\_l`） | `kero_menu_capacity_test.rs` の経路（sakura parse→compile→apply_cue→layout）を候補で | Gap-3 |
| 3.7（k≠1） | `ScaleContract`＋`FixedMetrics`（`scale_invariance_test.rs` 型）を候補の model で | — |
| 3.8（使わない資産が載らない・`error!` 0） | `resolve_balloon_faces` の戻りと `log_capture_kit::capture` | Gap-8 |
| 3.9／3.10／3.11 | 手順・番人既存 | `file_length_guard_test.rs` |
| 4（実機目視） | 手順＋記録 | Gap-4・Gap-12 |
| 5（出典・CC0） | 文書。LICENSE は CC0 1.0 Universal 法典本文と一致（先頭 3 行実測）・readme に「License : CC0」と URL | Gap-7 |
| 6（裁量登記） | §8 1 行・台帳 1 項目・`roadmap-draft` 1 行＋count・報告 2 本・`cargo test -p ukadoc-survey` | §1.5 のとおり受け口実在 |
| 7（下流申し送り） | brief 3 本追記。`id`＝`directory`＝`StayseeBalloon` は実物で一致確認済み | Gap-1（読む口は kv） |
| 8（非回帰） | `cargo test --workspace`（i686 helper の既存手順）・`cargo fmt --check`・番人 | — |

## 3. 実装の選択肢

### Option A: 既存テストファイルへの追記（**取らない**）

`shipped_fixture_region_test.rs` 等に候補の檻を足す。**要件 3.1／8.3 に反する**（`nar-install` の書き換え対象と共有が生じる）。記録のためだけに挙げる。

### Option B: `crates/areka-emo-text/tests/` に新規ファイルを 1 本（推奨候補）

- 置き場: `crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs`（名前は仮）。検体パス定数 1 つ・COM 初期化ヘルパ・PNG ヘッダ読み・kv 読み・面解決＋WIC bake＋α 走査・`TextRegion`・レイアウト（実 `DWriteMetrics`・`ＭＳ ゴシック`）・`\q`／`\_l` 台本・k≠1 の `FixedMetrics` 檻を 1 ファイルに収める。
- 到達: すべて公開 API・`Cargo.toml` 変更 0・既存ファイル変更 0・1,000 行の番人に掛かるので詰め込み過ぎに注意（既存の同型は 397〜820 行）。
- ✅ 要件 3.2「定数 1 か所」が自然に満たせる ✅ 共有 0 が構造で保証される ❌ 1,000 行の上限に近づきやすい ❌ headless GPU の読み戻し（目視証跡 PNG）まで入れると超える見込み

### Option C: 新規ファイル 2〜3 本＋共有モジュール（Option B の分割形）

- `tests/staysee_balloon/common/mod.rs`（検体パス定数・COM・PNG ヘッダ・kv 読み）＋テーマ別ファイル（①保管と定義の読み ②面解決・焼き込み・α ③文字領域・レイアウト・台本・k）。接続は structure.md の `#[path]` 規約。
- ✅ 1,000 行に余裕 ✅ 目視証跡 PNG（headless `read_back`）を別ファイルに置ける ❌ 「定数 1 か所」は common に置けば満たすが、`nar-install` が共有ヘルパへ寄せるときの付け替え先が `common/mod.rs` の 1 行になる（要件 3.2 の意図は満たす） ❌ ファイル数が増える

### 補足: `areka-emo-present` に `tests/` を新設する案（**取らない理由**）

系列解決の権威側で檻を持つ方が筋は良いが、文字領域・レイアウト・sakura 台本は `areka-emo-text` 側にしか無く、両クレートに分けると検体パス定数が 2 か所になる（要件 3.2 違反）。

## 4. 規模とリスク

- **規模: S**（既存流儀の踏襲・新規依存 0・本番変更 0）。テスト 1〜3 本＋文書 6 か所＋フォルダ 1 つ。
- **リスク: Low〜Medium**
  - Low: 保管・文書・番人は受け口が実在し、前例どおり。
  - Medium: 期待値の較正（PNG 原寸 5 枚・面 0 の α・`ＭＳ ゴシック` 12px の送り幅）は取得後の実測が要る。COM＋WIC の決定論テストは前例があるが `areka-emo-text/tests/` では初。

## 5. 設計フェーズへの推奨と持ち越し

### 推奨

- Option B を起点にし、1,000 行の見込みが立たなければ Option C へ。判断材料は「目視証跡 PNG（headless `read_back`）を決定論テストに含めるか」（含めるなら C）。
- 要件 3.3 の `type`／`id`／`name` は **`kv::parse_kv` で読む**（本番変更 0 を優先）。設計で明記。
- 要件 3.5 の観測は「候補は `wordwrappoint` 未宣言→`validrect.right`＝309 が折返し基準」と読み替えて固定。`debug!` 1 件の実在も固定候補。
- 要件 3.8 は `error!` 0 件に加え **`warn!` は scope 1 の面 2・3 の縮退 2 件だけ**であることを固定する（面の増減で赤くなるのは意図どおり）。
- 要件 2.6 は取得時に **`paint_transparent_region_black,0` の 1 行**を本文へ足す是正で発動する見込み。要件 1.1 の候補記述も同時に直す。

### Research Needed（設計で埋める）

1. 候補 PNG 全 6 枚（`balloons0〜3`・`balloonk0〜1`）の原寸と面 0 の (0,0) の α（`face_origin_color` の縮退経路を踏むか）。
2. `ＭＳ ゴシック` 12px での `DWriteMetrics`（送り幅・行ボックス丈）と、`validrect` 高さ 138（本体）／68（相方）で入る行数（14 px 送りで 9 行／4 行の見込み）。
3. `AtlasTable::page(...).bytes` の α 走査で「半透明画素が在る」を固定する最小の述語（premultiplied BGRA・`0 < α < 255` の画素数 > 0）。
4. `roadmap-draft.md` の `[briefs].snapshot_on` を更新するか（腕 a は count しか見ない）。
5. `verification/` の記録ファイル名と構成（`signoff-record.md`＋`provenance.md`〔取得元・ハッシュ 29 本〕の 2 本が既存慣行に近い）。

## 6. 要件ディスカッションへ回す設計判断（番号付き）

1. **`type`／`id`／`name` の読み口**（Gap-1）: `BalloonModel` を拡張（本番変更＝要件 8.1 と衝突）か、テスト内で `kv::parse_kv` を使うか。後者なら要件 3.3 の「areka shall 読み取り」は「areka の読み手（parsers）で読める」と読み替える。
2. **`paint_transparent_region_black,0` の扱い**（Gap-6）: 要件 1.1／2.6 の候補記述を是正するだけか、§8 の裁量行の項目名にも既に含まれているので追加の登記は不要とするか。
3. **`wordwrappoint` 未宣言の読み替え**（Gap-2）: 要件 3.5 の文言を「`wordwrappoint`（未宣言なら `validrect` の遠辺）」へ改めるか、そのまま「soft＝hard」の観測 1 本で満たすか。
4. **既定フォントの固定**（Gap-3）: `ResolvedFont` が `ＭＳ ゴシック`／12 へ縮退することを檻で固定するか。固定すると `draw.rs` の既定値変更で赤くなる（意図どおりか）。
5. **`warn!` 2 件の固定**（Gap-8）: scope 1 の縮退面を件数で固定するか、`error!` 0 件だけで足りるとするか。
6. **新規テストの構成**（Gap-10）: 1 ファイル（Option B）か、common 共有の 2〜3 ファイル（Option C）か。決め手は目視証跡 PNG を決定論側に含めるか。
7. **実機目視の証跡の形**（Gap-12）: headless `read_back` の PNG を証跡に数えるか、実機は観察記録のみとするか。
8. **版の記録**（Gap-7）: readme の v1.00A（2020/6/27）と取得コミット `fe1b02f3`（2021-11-05）の両方を並記する形で要件 2.3／5.1 を満たすか。
9. **`face_origin_color` の白縮退**（Gap-4）: 候補で `\f[disable]` の色味が白寄りになる可能性を「既知の制限」として README 申し送りに含めるか、実機目視で見てから決めるか。
10. **検体参照ファイルの数え方**（Gap-9）: 要件 8.3 の「実測 38」を `nar-install` の一覧に揃えるか、本 spec は「既存ファイルに触らない」だけを主張して数を書かないか。

## 7. 要件ディスカッションでの処理（2026-09-18）

§6 の 10 件を要件ディスカッションで仕分けた結果。**この節が §6 の現在の状態を示す正本**。

### 要件側で解決済み（要件書を是正・設計へ持ち越さない）

| §6 | 処理 |
|---|---|
| 1 `type`／`id`／`name` の読み口 | 要件 3.3 を「areka の定義の読み手（`crates/areka-parsers/`）が読み取る」へ改め、`type`／`id`／`name` を保持する型が本番に無いこと・読み口の選択は要件 8.1（本番変更 0 行）を崩さない範囲で設計が決めることを明記。**`BalloonModel` の拡張は要件 8.1 が禁じるので選択肢から外れる。** |
| 2 `paint_transparent_region_black,0` の扱い | 要件 2.6 の発動として Introduction の候補記述へ 1 行を足した。§8 の裁量行の項目名にこの鍵が既に含まれるので**追加の登記は不要**。 |
| 3 `wordwrappoint` 未宣言の読み替え | 要件 3.5 を「折返し基準（`wordwrappoint`。StayseeBalloon は未宣言なので `validrect` の遠辺へ縮退する）」へ改めた。Introduction にも未宣言である事実を足した。 |
| 8 版の記録 | 要件 2.3 が既に「取得したコミットのハッシュ・取得日・readme に書かれた版と日付」を両方求めており、**追加の是正は不要**。 |
| 10 検体参照ファイルの数え方 | 要件 3.1・8.3・Boundary Context から「実測 38」を外し、判定の基準を「既存ファイルへの変更 0 行」に置き換えた（数え方に依らない性質で判定する）。 |

### 設計フェーズへ持ち越す（`/kiro-spec-design` で解決）

| §6 | 論点 | 決め手 |
|---|---|---|
| 4 | 既定書体 `ＭＳ ゴシック`／12 への縮退を檻で固定するか | **開発者裁定（2026-09-18）「ukadoc 準拠にすること」により前提が変わった**。`ＭＳ ゴシック` は ukadoc「`font.name,フォント名`」の既定そのものなので、この縮退を檻で固定することは**正典適合の後退検出器**として妥当になる。設計は「固定するか」ではなく「どの粒度で固定するか」を決める |
| 5 | scope 1 の縮退 `warn!` 2 件を件数で固定するか、`error!` 0 件のみか | 件数で固定すると面の増減で赤くなる。記憶「檻は到達する経路を踏ませよ」に照らし、恒真にならない側を選ぶ |
| 6 | 新規テストの構成（1 ファイル＝Option B ／ common 共有の 2〜3 ファイル＝Option C） | 目視証跡 PNG（headless `read_back`）を決定論側に含めるか。含めるなら 1,000 行の番人に掛かるので C |
| 7 | 実機目視の証跡の形 | 要件 4.2 が「読み戻し画像か、無ければ観察記録」と既に二者を許しているので、設計は**どちらを採ったか**を決めて書けばよい |
| 9 | `face_origin_color` の白縮退（`[disable]` の色味） | 候補の面 0 の (0,0) の α を実測してから。既知の制限として README 申し送り（要件 5.2）に含めるか否かは実機目視の後に確定する |
| Research Needed 1〜5（§5） | PNG 原寸 6 枚・面 0 の α・`ＭＳ ゴシック` 12px の送り・α 走査の最小述語・`snapshot_on` の更新可否・`verification/` のファイル構成 | いずれも取得後の実測で埋まる。設計の Discovery で実測する |

## 8. 設計フェーズの Discovery（2026-09-18・`/kiro-spec-design`）

### 8.1 要約

- **Feature**: `areka-P0-default-balloon-bundle`
- **Discovery Scope**: Extension（light discovery）——本番コード変更 0 行・新規依存 0 の「資産＋新規テスト＋文書」なので、統合点（公開 API・保管慣行・台帳の番人）の実測に絞った。
- **Key Findings**:
  - 上流 `github.com/ponapalt/StayseeBalloon` を取得できた（HEAD `fe1b02f30d5e263cf30df800c32b2b525a52e3ad`・2021-11-06 05:04:31 +0900＝UTC 2021-11-05・件名「use_input_alpha」）。§5 Research Needed 1〜5 は**すべて実測で埋まった**（8.2）。
  - 面 6 枚の (0,0) はすべて α＝0 → 起動側の `face_origin_color` は白へ縮退する（`debug!`）。半透明画素（0<α<255）は 6 枚とも 2,000 個超で実在する＝要件 3.4 の α 保持は「半透明画素が焼き込み後にも残る」で固定できる。
  - `ＭＳ ゴシック` は upem 256・ascent 220・descent 36（`draw_metrics.rs` の実測記録）＝行ボックス比 1.0・半角 0.5em。12px では全角 12・半角 6・行送り 14・行ボックス 12 が期待値。
  - `roadmap-draft.md` は「表は着手時点の写真」と自ら書いており、行の追加は前例 0（過去 3 spec は `owner_count` の書き換えのみ）。だが台帳の宛先に本 spec を書けば腕 f が `[[spec]]` 行を要求する＝行の追加は必須。`snapshot_on` は据え置き、追加の事実は表の直前の散文に 1 文で残す。

### 8.2 Research Log

#### 上流の取得と保管する 29 本の実測（要件 2.2・2.3・2.5・2.6・5.1・7.1）

- **Context**: §5 の 1・5、要件 2.6 の発動判定。
- **Sources**: `git clone https://github.com/ponapalt/StayseeBalloon.git`（作業用一時領域・リポジトリ外）・`sha256sum`・`iconv -f SHIFT_JIS`・PNG IHDR の直読み。
- **Findings**:
  - ファイルは **29 本**（`LICENSE`・`arrow0/1.png`・`balloonc0〜4.png`・`balloonk0/1.png`・`balloons0〜3.png`・`descript.txt`・`install.txt`・`marker.png`・`online0〜8.png`・`readme.txt`・`sstp.png`・`thumbnail.pnr`）で要件 2.2 の一覧と一致。`.pna` 無し。sha256 は設計時に採取済みで、保管後に `verification/provenance.md` へ写す（保管フォルダから採り直した値と一致することを確認して書く）。
  - `descript.txt`（Shift_JIS・CRLF）の要点は要件の記述と一致（`charset,Shift_JIS`・`type,balloon`・`name,Balloon for Staysee Syncfield`・`id,StayseeBalloon`・`craftman,SSP BUGTRAQ`・`craftmanw,ばぐとら研究所/整備班`・`craftmanurl,http://ms.shillest.net/`・`homeurl,http://ms.shillest.net/balloon/StayseeBalloon/`・`use_self_alpha,1`・`use_input_alpha,1`・`paint_transparent_region_black,0`・`validrect.left,22`／`top,20`／`right,-26`／`bottom,-47`・`font.height,12`）。要件に挙げていない鍵として `font.color.r/g/b,0/40/100`・`anchor.font.color.*`・`cursor.blendmethod,none`・`cursor.style,square`・`cursor.brush/pen/font.color.*`・`arrow0/1.x/y`・`onlinemarker.*`・`sstpmarker.*`・`sstpmessage.*`・`number.*`・`communicatebox.*` が在る。**要件 2.6 の「食い違い」には当たらない**（要件は「要点」を列挙しており、一覧・LICENSE 種別・要点はすべて一致）。`font.color`・`cursor.*` は areka の読み手（`balloon/parse.rs`）が読む鍵なので、設計の期待値に入れる（文字色 (0,40,100)・選択肢 hover は `cursor.style,square`＝SquareFill 実導出）。
  - `install.txt`: `charset,Shift_JIS`／`type,balloon`／`name,Balloon for Staysee Syncfield`／`directory,StayseeBalloon`＝descript の `id` と同綴り（要件 7.1）。
  - `readme.txt`: 作者「ぽな（ばぐとら研究所/整備班）」・「License : CC0 https://creativecommons.org/publicdomain/zero/1.0/deed.ja」・更新履歴 2020/6/26 v1.00・2020/6/27 v1.00A。`LICENSE` は「Creative Commons Legal Code / CC0 1.0 Universal」で始まる法典本文（CRLF）。
  - リポジトリ直下 `.gitignore`（`*_test.txt`／`*_dump.txt`）に大小無視で当たる名前は **0 件**（要件 2.5）。`git check-attr text vendors/sample_ghost/StayseeBalloon/descript.txt` と `.../balloons0.png` は**フォルダ未作成の時点で `unset`**（要件 2.4・`* -text` が下位へ効く）。
- **Implications**: 要件 2.6 は発動しない（本文の是正なし）。`verification/provenance.md` に 29 本のハッシュ・コミット・日付（+0900 と UTC の両方）・readme の版を書く。

#### PNG 原寸と α の実測（§5 の 1・3、要件 3.3〜3.5・3.7）

- **Findings**（IHDR 直読み・自前のフィルタ復元で全画素走査）:

  | ファイル | 原寸 | 色種 | (0,0) の RGBA | α＝0 | 0<α<255 | α＝255 |
  |---|---|---|---|---|---|---|
  | `balloons0.png` | 335×205 | RGBA8 | (0,0,0,0) | 9,617 | **2,445** | 56,613 |
  | `balloons1.png` | 335×205 | RGBA8 | (0,0,0,0) | 9,622 | 2,399 | 56,654 |
  | `balloons2.png` | 335×395 | RGBA8 | (0,0,0,0) | 12,467 | 3,026 | 116,832 |
  | `balloons3.png` | 335×395 | RGBA8 | (0,0,0,0) | 12,472 | 2,996 | 116,857 |
  | `balloonk0.png` | 335×135 | RGBA8 | (0,0,0,0) | 8,567 | **2,190** | 34,468 |
  | `balloonk1.png` | 335×135 | RGBA8 | (0,0,0,0) | 8,572 | 2,186 | 34,467 |

  他: `balloonc0〜4` 405×61 RGBA・`arrow0/1`／`marker` 12×10 RGBA・`online0〜8` 90×90 グレー＋α・`sstp` 12×9 グレー＋α・`thumbnail.pnr` 176×43 パレット。
- **Implications**:
  - 文字描画範囲（`TextRegion::resolve`・負値は反対辺基準）: scope 0（335×205）＝left 22・top 20・right **309**・bottom **158**（幅 287・高さ 138）／scope 1（335×135）＝left 22・top 20・right 309・bottom **88**（高さ 68）。折返し基準（`wordwrappoint` 未宣言）は遠辺 309 へ縮退（`debug!` 1 件）。
  - (0,0) が透明なので `emo2_boot::balloon_background::face_origin_color` は `REASON_NOT_OPAQUE`（トリムで原点が落ちていれば `REASON_ORIGIN_TRIMMED_AWAY`）で白 (255,255,255) へ落ちる（`debug!`・bin クレート内部で外から踏めない）。効くのは `\f[disable]` の混色だけで、表示崩れではない。
  - α 保持の最小述語: `AtlasTable::resolve(SetId(0), "balloons0.png")` → `entry.placement.uv_rect` の内側を `page.bytes`（premultiplied BGRA・`stride` 明示）で走査し、`0 < A < 255` の画素数 > 0。`entry.original == (335,205)` も併せて固定する。

#### `ＭＳ ゴシック` 12px の計測値（§5 の 2、要件 3.5・3.6・1.3）

- **Sources**: `crates/areka-emo-text/src/draw_metrics.rs` の実測記録（upem 256・ascent 220・descent 36）・`layout.rs`／`choice.rs` の「比ちょうど 1.0」・`state.rs` の `line_pitch = font_height + line_gap(2)`・`layout.rs` `visible_window` の「境界ちょうどは超えていない（> 判定）」。
- **Findings**: 全角 12・半角 6（0.5em）・行ボックス 12・行送り 14。scope 0 の 1 行に入る字数は全角 **23**（22＋23×12＝298・次で 310>309）・半角 **47**（22＋47×6＝304・次で 310>309）。行数は n 行目の下端 ＝ 20＋14(n−1)＋12 ≤ 遠辺 → scope 0 は **10 行**（下端 158＝境界ちょうど・あふれ非発火）・scope 1 は **5 行**（下端 88）。
- **Implications**: 既存の実フォント檻の門（「あ」の送り < em）は使えない。門は「半角 `a` の送りがちょうど 6・全角 `あ` が 12・行ボックス 12」を直接固定する（`ＭＳ ゴシック` が無い環境では別書体へ落ちて赤になる＝既存テストの `Yu Gothic UI` 前提と同じ扱い）。**これらは設計時の導出値**であり、実装時に `DWriteMetrics` で実測して較正する（違えば理由を `verification/` に書く）。

#### 台帳と `roadmap-draft.md` の受け口（§5 の 4、要件 6.2〜6.5）

- **Findings**: `spec_checks.rs` 腕 c は `owner_count` を台帳 4 本の数え直しと突き合わせ、腕 f は台帳の全 owner が `[[spec]]` か `[[owner_completed]]` に載ることを要求する。`[[spec]]` 行の欄は `name`／`stage`／`bundle`／`owner_count`／`wave`（`wave` は自由文字列）。`roadmap-draft.md` の散文（「行数が 27 のまま…」の段落）は「表は着手した時点の写真」「`snapshot_on` は行の集合を撮った日」と定めており、**行の追加は前例 0**（`git log` で過去 3 spec は `owner_count` と `reason` の書き換えのみ）。
- **Implications**: 台帳に owner を書く以上、行の追加は腕 f が要求する必須作業。`snapshot_on` は据え置き（腕は count しか読まない・日付の意味は「写真を撮った日」）、追加の事実と理由は表の直前の散文に 1 文（日付付き）で残す。`[briefs].count` は 28。

#### 新規テストの置き場と到達（§3 Option B／C の決着、要件 3.1・3.2・3.11・8.2）

- **Findings**: `crates/areka-emo-text/Cargo.toml` の通常依存に `areka-emo-present`・`areka-parsers`・`areka-sakura`・`windows`・`wintf`、dev 依存に `areka-emo-atlas`・`log-capture-kit` が在る＝系列解決・WIC bake・α 走査・2 層マージ・領域解決・レイアウト・台本・ログ捕捉のすべてに `Cargo.toml` 変更 0 で届く。同型の既存檻は 397〜820 行。headless GPU の読み戻し＋PNG 符号化は `choice_fixture_test.rs` で約 100 行の追加になる。
- **Implications**: 決定論側に GPU 読み戻しを**含めない**（8.4 DD1）。1 ファイル約 600〜800 行で 1,000 行の番人に収まる見込み。

### 8.3 Architecture Pattern Evaluation

| Option | 概要 | 強み | 弱み | 判定 |
|---|---|---|---|---|
| B: 新規テスト 1 ファイル（`crates/areka-emo-text/tests/staysee_balloon_fixture_test.rs`） | 検体パス定数 1 つ・全観測を 1 ファイルに | 要件 3.2 を構造で満たす・共有 0・番人 1 本 | 1,000 行に近づく | **採用**（GPU 読み戻しを含めないので収まる） |
| C: 共有モジュール＋2〜3 ファイル | `#[path]` で common を共有 | 行数に余裕 | ファイル数・付け替え先が common の 1 行になる | 不採用。実装で 1,000 行を超えたときの**縮退先**（親ファイルに定数を残し `#[path]` でテーマ分割＝structure.md の規約）としてのみ残す |
| GPU 読み戻し PNG を決定論側に含める | `choice_fixture_test.rs` 型 | 目視証跡が自動で出る | 実機の色味・DPI 切替は写らない・行数増 | 不採用。要件 4.2 の「読み戻しが無ければ観察記録」を採る |

### 8.4 Design Decisions

#### DD1: 新規テストは 1 ファイル・GPU 読み戻しなし（§7 持ち越し 6・7）
- **Context**: 要件 3.1／3.2／3.11・4.2。
- **Alternatives**: Option B／Option C／GPU 読み戻し込み。
- **Selected**: Option B。実機目視の証跡は観察記録（`verification/signoff-record.md`）。
- **Rationale**: 要件 3.2「定数 1 か所」を構造で満たす最小形。GPU 経路そのものは `emo2_fixture_e2e_test.rs` 等の既存檻が別検体で踏んでおり、本 spec の観測点は「この検体の値」である。
- **Trade-offs**: 目視証跡が画像でなく記録になる（要件 4.2 が明示的に許す）。
- **Follow-up**: 実装で 1,000 行を超えたら Option C の形へ分割（定数は親に残す）。

#### DD2: `type`／`id`／`name`／`directory` は `charset::decode`＋`kv::parse_kv` で読む（§7 で確定済みの再掲）
- **Selected**: テスト内で `areka_parsers::charset::decode(&bytes, DefaultEncoding::Ansi)` → `areka_parsers::kv::parse_kv`。`BalloonModel` は拡張しない（要件 8.1）。
- **Rationale**: 本番コードが `id` を消費する場所は現状 0（下流 `baseware-root-layout` が定数で持つ）。

#### DD3: ログ檻は「解決の窓」に限り、scope 1 の `warn!` を **2 件ちょうど**で固定する（§7 持ち越し 5）
- **Alternatives**: `error!` 0 件のみ／`warn!` 件数固定。
- **Selected**: `log_capture_kit::capture` の窓に `resolve_balloon_faces`＋`load_scope_balloon_model` だけを入れ、scope 0 は `error!` 0・`warn!` 0、scope 1 は `error!` 0・`warn!` **2**（`surface_id` 2 と 3・`prefix` `balloons`）。bake（WIC）は窓の外（WIC 側の警告の有無を本 spec は主張しない）。
- **Rationale**: 件数で固定すると面の増減で赤くなる——それは保管フォルダ（要件 2.2 で無改変）が変わった合図であり意図どおり。「`error!` 0 件のみ」は不在主張で恒真に寄る。
- **Trade-offs**: `resolve_balloon_faces` の R6.2 の文言変更で赤くなる（欄 `surface_id`／`prefix` で判定し、本文の文字列一致にはしない）。

#### DD4: 既定書体の固定粒度（§7 持ち越し 4）
- **Selected**: ⑴ `model.font().name()` が `None`（宣言なし）・⑵ `ResolvedFont::resolve(&model).name == "ＭＳ ゴシック"`（**リテラル**＝ukadoc 既定の後退検出器）かつ `== DEFAULT_FONT_NAME`（定数との同一性）・`height == 12.0`・⑶ 実 `DWriteMetrics` で `advance('a')==6`・`advance('あ')==12`・`line_box_height(12)==12`・`line_pitch(12)==14`。
- **Rationale**: 開発者裁定「ukadoc 準拠」により、既定 `ＭＳ ゴシック` からの後退は正典適合の後退そのもの。⑶ は実フォントの門を兼ねる。

#### DD5: `face_origin_color` の白縮退は「既知の制限」に**含めない**（§7 持ち越し 9）
- **Selected**: README 申し送り文の既知の制限は「半透明前提のバルーンだけが正しく表示される」の 1 つに留める。白縮退は `verification/signoff-record.md` の所見欄に事実として書く。
- **Rationale**: 効くのは `\f[disable]` の混色だけで、emo2 の台本には無効表示が出ない＝第三者が見る画面に差が出ない。実機目視で色味に所見が出たときだけ 4.3 の経路で扱う。

#### DD6: `roadmap-draft.md` の行追加と `snapshot_on` 据え置き（§5 の 4）
- **Selected**: `[[spec]]` 行（`name = "areka-P0-default-balloon-bundle"`・`stage = "A"`・`bundle = "絵の重ね方"`・`owner_count = 1`・`wave = "A0"`）を末尾に足し、`[briefs].count = 28`、`snapshot_on` は `"2026-09-13"` のまま。表の直前の散文に「2026-09-18 に本 spec の行を 1 行足した（台帳の宛先に書いたため腕 f が要求する）」を 1 文加える。

#### DD7: `verification/` は 2 本
- **Selected**: `provenance.md`（取得元・コミット・日付・readme の版・29 本の sha256・`.gitignore` 照合・`check-attr`・`id`＝`directory`・要件 2.6 の判定）と `signoff-record.md`（裁定 1.2 の記録・不採用理由・決定論テストの結果・較正の差・実機目視の観察記録・3.9／4.3 の処理・README 申し送り文・台帳検査の結果・下流申し送りの実施記録）。
- **Rationale**: 完了 spec の慣行（`charset-canon` の `signoff-record.md`＋`boundary-record.md`）に倣い、出典（機械が照合する値）と判断（人が読む記録）を分ける。

### 8.5 Risks & Mitigations

- **`ＭＳ ゴシック` 12px の導出値が実測とずれる**（半角 6／全角 12／行ボックス 12）— 実装の最初に `DWriteMetrics` で計測し、ずれたら期待値を実測に合わせ、理由（フォントの版・ヒンティング）を `signoff-record.md` に書く。導出の根拠は `draw_metrics.rs` の実測記録。
- **WIC bake がテストスレッドで COM 初期化を要する** — `emo2_e2e.rs` の `with_com_initialized`（`CoInitializeEx(None, COINIT_MULTITHREADED)`・`RPC_E_CHANGED_MODE` 許容）を新規ファイル内に写す（他のテストファイルから `use` はできない）。
- **`resolve_balloon_faces` の `warn!` を件数で固定** — 保管フォルダの面が増減すると赤。意図どおり（要件 2.2）。
- **1,000 行超過** — Option C へ縮退（定数は親に残す）。
- **`cargo test --workspace` の i686 helper 前提** — 既存手順どおり（記憶 workspace-test-needs-i686-host32-artifacts）。本 spec の新規テストは x64 のみ。

### 8.6 References

- [ponapalt/StayseeBalloon](https://github.com/ponapalt/StayseeBalloon) — 取得元（HEAD `fe1b02f30d5e263cf30df800c32b2b525a52e3ad`）
- ukadoc `descript_balloon` `font.name,フォント名` — 「デフォルトはＭＳ ゴシック」（`ukadoc:descript_balloon:font.name_2c_30d5_30a9_30f3_30c8_540d:1`）
- `crates/areka-emo-present/src/balloon.rs` — `resolve_balloon_faces`／`load_scope_balloon_model`／`build_balloon_target_from_faces`（公開 API・`UseSelfAlpha::On` 固定）
- `crates/areka-emo-text/src/draw_metrics.rs` — `ＭＳ ゴシック` upem 256・ascent 220・descent 36 の実測記録
- `doc/ukadoc-coverage/roadmap-draft.md`・`crates/ukadoc-survey/tests/consistency/spec_checks.rs` — 腕 a〜f
