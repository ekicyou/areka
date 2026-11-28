# ギャップ分析: emo-balloon-merge-log-overlap

> 2026-11-28・本ブランチ（main `b05e652f` の上）でコードを読み直して書いた。行番号はこの時点のもの。設計の着手時に引き直すこと。
> 判断は下さず、材料と選べる形を並べる。決めは要件討議へ回す（末尾の「討議へ渡す決めごと」）。

## 1. 分析の要約

- **重なりの形は brief のとおり残っている**。2 層マージ `load_scope_balloon_model`（`crates/areka-emo-present/src/balloon.rs:521`）は基層を `LoadRecords::Emit` に決め打ちで読み（527）、核 `scope_balloon_model_with`（532）は上書き層の `debug!`／`warn!`（`read_face_override_layer` 474）と確定の `info!`（546）を無条件に出す。起動の準備では、窓の位置の読み（`crates/areka-placement/src/lib.rs:748`）と資産の組み立て（`balloon_definitions.rs:94`・`102`）がそれぞれ scope ごとに 1 回ずつ通る。
- **起動・ゴーストの切り替え・ゴーストの読み込み直しは同じ道を通る**（要件 2 の前提は成り立つ）。シェルの着替えだけが、窓の位置の読み `load_scope_windowpositions` を資産の組み立てなしで単独に呼ぶ（`crates/areka-runtime/src/switch_assets.rs:235`）。
- **今の 2 行を固定している既存のテストがある**。`crates/areka-session/src/boot_shell_tests.rs:157` の `merge: scopes(2 * n)` が、本番の順の起動と切り替えで 2 層マージの確定の `info!` が scope ごとに 2 行出ることを檻に入れている。案 1 でも案 2 でもこのテストは赤になるので、直す必要がある。このファイルは、ウェーブ C29 の約束 ⑹ の「触ってよいファイル」に入っていない。
- **案 1 は小さく収まる**。`balloon.rs` で十数〜20 行、`crates/areka-placement/src/lib.rs` で 8〜12 行ほど増える（896 → 910 前後）。ただし、シェルの着替えでは今どおり記録を出すと決めた場合（要件 4.1）、brief の見立て「`lib.rs` は use の名前と呼び出しを替えるだけで、行は増えない」は成り立たない。
- **案 2 は約束 ⑹ に当たる**。起動の順は配置の準備（窓を作る）が先で、資産の組み立て（`wire_runtime`）が後である。そのため、模型を運ぶ道は `ghost_session.rs`（配置の準備から起動の結線への受け渡し）と `assets.rs` を通らざるを得ない。

## 2. 今のコードの調べ

### 2.1 2 層マージの権威（`areka-emo-present`）

| 項目 | 場所 | 記録 | 記録を出すかどうかの口 |
|---|---|---|---|
| `load_scope_balloon_model`（pub） | `balloon.rs:521` | 基層を `read_descript_layer(.., LoadRecords::Emit)` で読む（527） | 無い |
| `scope_balloon_model_with`（pub(crate)・核） | `balloon.rs:532` | 上書き層の読み（540）と確定の `info!`（546〜557） | 無い（引数に `records` が無い） |
| `read_descript_layer`（pub(crate)） | `balloon.rs:453` | 基層が読めない `warn!` | **在る**（`records: LoadRecords`） |
| `read_face_override_layer`（private） | `balloon.rs:474` | 不在＝`debug!`・不在以外＝`warn!` | 無い |
| `load_scope_balloon_definitions`（pub・資産の組み立て） | `balloon_definitions.rs:81` | 基層を scope ごとに 1 回 `Emit` で読み（94）、面 0 と自分の設定ファイルを持つ面ごとに核を呼ぶ（102） | — |
| `LoadRecords { Emit, Quiet }` | `load_records.rs` | 「`Err` で終える失敗の記録は値に関わらず出す」が取り決め | — |

- 前例として、記録を出さない入口が 2 つ揃っている。`resolve_balloon_faces_quiet`（`balloon.rs:378`）と `load_balloon_use_self_alpha_quiet`（620）で、どちらも「中身は 1 本の `_with(records)`、入口は薄いラッパ」の形をとる。その檻は `balloon_quiet_tests.rs`（229 行）にあり、出す入口と出さない入口の戻り値が等しいことと、出さない入口の記録が 0 行であることを `assert_silenced` で確かめている。
- 2 層マージには `Err` の道が無く、いつも `BalloonModel` を返す。黙らせても「記録の無い失敗の道」は構造上生まれない（要件 3.5 は 2 層マージの側ではほぼ空になる。§4 の R3.5 を参照）。
- 説明の「観測」の節（`balloon.rs:516〜520`）は、placement から scope あたり 1 行、boot から面ごとに 1 行出し、面 0 の行が placement の行と値が一致する、と書いている。brief の言う「`# 観測（R6.3）`」の見出しと「生き証人」の言葉は、今のコードには無い（要件の討議 5 で読み替え済み）。

### 2.2 窓の位置の読み（`areka-placement`）

- `scope_windowposition(balloon_root, scope)`（private・`lib.rs:710`）は、系列を `resolve_balloon_faces_quiet` で解き（726・記録を出さない）、`load_scope_balloon_model`（748・記録を出す）を呼んで、`(WindowPosition, WindowPositionRaw)` だけを返す。`validrect` は使わない。
- `load_scope_windowpositions(balloon_root, scope_ids)`（**pub**・559）は `scope_windowposition` を scope ごとに回すだけである。

**`load_scope_windowpositions` の呼び出し元（全部）**

| 呼び出し元 | 道 | 資産の組み立てを伴うか |
|---|---|---|
| `crates/areka-placement/src/lib.rs:411` `prepare_stages_for_shell` | 起動・ゴーストの切り替え・ゴーストの読み込み直し（下の 2.3） | 伴う（後で `build_balloon_assets` が面 0 をもう 1 回マージする） |
| `crates/areka-runtime/src/switch_assets.rs:235` `build_shell` | シェルの着替え（シェルの読み込み直し・`descript` の読み込み直しの `ghost`／`shell` を含む） | 伴わない（バルーンの資産は組み直さない） |
| `crates/areka-placement/src/placement_windowposition_vocab_tests.rs:42`・`244` | テスト（語の解決・読めないバルーンの `warn!`） | — |
| `crates/areka-placement/src/reseed_tests.rs:334` | テスト（材料を作る） | — |

- `prepare_stages_for_shell` は private で、外への入口は `prepare_ghost_windows`（764）・`prepare_ghost_windows_for_shell`（776）・`prepare_ghost_windows_with_work_area`（810・`test-support` のときだけ）の 3 つである。
- 前例の `placement-measure-bake-once` は `scope_windowposition` の系列の解決を `_quiet` に替えた。これは共有の関数の中の直しだったので、**シェルの着替えの道の系列の解決の記録も、そのとき 0 行になっている**（今もシェルの着替えでは「系列の解決が完了」の `info!` は出ない）。2 層マージの記録だけが、両方の道で今も出ている。
- シェルの着替えの後始末 `finish_shell`（`crates/areka-runtime/src/frame/switch.rs:554〜569`）は、読み直した面 0 の `windowposition` を、定義の束がある scope については今出している面の定義（`wiring.balloon_definitions` の `for_face(face).model`）で置き換える。シェルの着替えで読み直した値がそのまま使われるのは、定義の束が無い scope（ふだんは起きない・`debug!`）だけである。
- `crates/areka-runtime/src/switch_assets_tests.rs` の `default_basis_from_boot_equals_the_one_from_shell_switch` は、起動の準備とシェルの着替えで `windowposition` の材料が同じ値になることを既に確かめている。

### 2.3 読み込み直しとゴーストの切り替えの道（要件 2 の確かめ）

コードを辿った結果、次の 3 つは同じ道を通る。

- **起動**: `main.rs:204` `ghost_session::open_ghost_windows` → `prepare_ghost_windows`（`ghost_session.rs:180`）→ `areka_placement::prepare_ghost_windows_for_shell`（195）→ `prepare_stages_for_shell` → `load_scope_windowpositions`（`lib.rs:411`）。その後 `main.rs:476` `boot_ghost` → `boot_wired` → `areka_runtime::wire_runtime`（`ghost_session.rs:580`）→ `build_boot_assets_for` → `build_boot_assets_with_shell` → `build_balloon_assets`（`assets.rs:253`）→ `load_scope_balloon_definitions`（512）。
- **ゴーストの切り替え**（`\![change,ghost]` など）: `ghost_switch::switch_to`（638）→ `between::boot_or_hold` → `boot_into`（816）→ `reopen_ghost_windows`（871）→ 上と同じ `prepare_ghost_windows` → `boot_ghost_strict`（876）→ `boot_wired` → `wire_runtime`。既定へ戻す `switch_to_default` も `boot_into` を通る。
- **ゴーストの読み込み直し**（`ReloadTarget::Ghost`）: `reload::start_switch`（352）→ `accept_switch` → `switch_to`。切り替えと同じ道である。
- 道の外にあるもの:
  - **SHIORI の読み込み直し**（`ReloadTarget::Shiori`）: `rebuild_runtime_in_place`（`ghost_session_runtime.rs:69`）は窓・seriko・束を残し、`runtime_boot::boot_runtime` → `areka_ghost::boot_with_balloon` だけを通る。配置の準備も資産の組み立ても通らないので、2 層マージの記録は今も 0 行で、変わらない。
  - **シェル・バルーン・`descript` の読み込み直し**: `skin_steps`（`reload.rs:513`）が着替えの段（`switch_assets` の `build_shell`／`build_balloon`）へ流す。要件 4.1・4.2 の道である。
- `placement-measure-bake-once` の設計（完了・design.md の 70 行目）にも、同じ「3 つとも同じ道」の確かめがある。今のファイルの配置でも成り立っている。

### 2.4 バルーンの着替え（要件 4.2）

- `build_balloon`（`switch_assets.rs:247`）は `build_balloon_assets` だけを呼び、窓の位置は読み直さない。`finish_balloon`（`frame/switch.rs:623`）は材料の `windowpositions` を空けて、面の書き手に定義から解き直させる。2 層マージの記録は、資産の組み立てから面ごとに 1 行だけ出る。案 1 で変わる所は無い。

### 2.5 起動の準備の外の入口（要件 4.3）

- `load_scope_balloon_model` を直に呼ぶ所: `crates/areka/src/mcp/dump_surface_gpu_test_support.rs:296`、`crates/areka-emo-text/tests/support/fixture.rs:347`（`tests/staysee_balloon_fixture/faces.rs` が記録の宛先を数える）、`balloon_model_tests.rs`。どれも出す入口のままでよい（案 1 では名前も形も変えない）。

### 2.6 記録を数える既存のテスト

| テスト | 何を固定しているか | 案 1 での扱い |
|---|---|---|
| `crates/areka-session/src/boot_shell_tests.rs`（435 行）の `LoadLines::per_boot`（150〜159）・`production_boot_emits_each_load_record_once`（322）・`animated_hidden_shell_boot_emits_each_load_record_once`（338）・`each_switch_adds_one_more_set_of_load_records`（381） | 本番の順（配置の準備 → 起こす処理）の起動と切り替えで、2 層マージの確定の `info!` が scope ごとに **2 行**（`merge: scopes(2 * n)`）。説明（316〜320）にも「scope ごとに 2 行出る」とある | **赤になる**。`scopes(n)` に直し、説明と「非空虚性」の節も直す必要がある（要件 1.1・2.1 の本番の道の檻にそのまま使える） |
| `crates/areka-runtime/src/placement_tests/measure_bake_log_tests.rs`（280 行） | 焼く段と面の表の記録の数（採寸 → シェルの資産 → バルーンの資産を `MemoryDecoder` で通す） | 変わらない。新しい檻の型の手本になる |
| `balloon_model_tests.rs`（317 行） | 出す入口の記録の中身（不在＝`debug!`・不在以外＝`warn!`・基層＝`warn!`・確定の `info!` の欄） | 変わらない（出す入口のまま） |
| `balloon_quiet_tests.rs`（229 行） | 出さない入口 2 つの無音・戻り値の一致・`Err` の記録 | 2 層マージの出さない入口を足すなら、ここが同じ型の置き場になる |
| `switch_assets_tests.rs`（354 行） | シェルの着替えで `windowposition` を全 scope ぶん読み、起動と同じ値になる | 変わらない（記録の数は数えていない） |

### 2.7 テストの文言の中の要件の番号

- `balloon_model_tests.rs` の「R6.3:」の前置きは 3 か所（288・297・307）で、要件の書き手の見立てどおりである。
- 同じファイルには、ほかの要件の番号も残っている: 「（R2.2）」が 3 か所（44・67・73）、「R2.4」が 151、「R2.5」が 96・101、「D8」が 5 か所（173・179・185・217・259）、「要件 3.10/10.9」が 113。
- project の決まり（CLAUDE.md の `## Comments`）では、コードにもテストの名前にも要件の番号を書かない。手入れは「今の差分で足した・変えたものだけ」が原則で、広げるのは開発者が範囲を名指したときだけである。

### 2.8 ファイルの行の数と約束 ⑹

| ファイル | 今の行 | 案 1 で増える見込み | 約束 ⑹ |
|---|---|---|---|
| `crates/areka-emo-present/src/balloon.rs` | 793 | +15〜25（出さない入口・核の `records`・上書き層の読みの `records`・確定の `info!` の条件・説明の書き直し・新しいテストのファイルの宣言 +3） | 触ってよい |
| `crates/areka-placement/src/lib.rs` | 896 | +8〜12（下の 3.1 の S1）、新しいテストのファイルの宣言 +3、`test-support` の口を置くならさらに +5 前後 | 触ってよい（「呼び出しの 1 か所」と書いてある） |
| `crates/areka-session/src/boot_shell_tests.rs` | 435 | ±数行（`2 * n` → `n`・説明） | **名前が無い**（禁止の `ghost_session.rs` とは別のファイル。⑤ `ghost-switch-balloon-fallback` が触るのは `ghost_switch`・`boot_config`・`boot_resolve` とその兄弟のテストで、これとは別） |
| `crates/areka-emo-present/src/balloon_definitions.rs` | 190 | 0（核の形を保てば）／1（核に `records` を足して呼び手を直すなら） | 名前が無い |
| `crates/areka-runtime/src/placement_tests/mod.rs` | 11 | +1（新しい兄弟のテストを置くなら） | 名前が無い |
| `crates/areka-runtime/src/assets.rs`・`crates/areka-session/src/ghost_session.rs`・emo-text の `tests/support/mod.rs` | — | 0 | 触らない（案 1 では触る必要が無い） |

## 3. 実装の選択肢

### 3.1 案 1（記録を出さない入口を足し、起動の準備の窓の位置の読みだけ黙らせる）

**`areka-emo-present` の側**（どの下位案でも同じ）

- 前例どおり、中身 1 本の `_with(records)` と薄いラッパの形にする。
  - `pub fn load_scope_balloon_model_quiet(balloon_dir, scope, face0) -> BalloonModel` を足す。
  - 核に `records: LoadRecords` を通し、`read_face_override_layer` の `debug!`／`warn!` と確定の `info!` を `if records.emits()` で囲む。
- 核の形の選び方は 2 つある。
  - (e1) `scope_balloon_model_with` の形を保ち（`Emit` のラッパ）、`records` を受ける private な核を別に置く。`balloon_definitions.rs` は変えない。
  - (e2) `scope_balloon_model_with` に `records` を足し、`balloon_definitions.rs:102` の 1 行を直す。
- 戻り値は出す入口と等しい（`BalloonModel: PartialEq`）。

**`areka-placement` の側**（要件 4.1 の扱いで分かれる）

| 下位案 | 形 | シェルの着替え | `lib.rs` の増え | 呼び手・テストの変更 |
|---|---|---|---|---|
| **S1**（シェルの着替えは今どおり出す・要件 4.1 の前提） | `scope_windowposition` に 2 層マージの関数（`fn(&Path, u32, &ResolvedFace) -> BalloonModel`）か、private な `enum` を渡す。`load_scope_windowpositions`（pub・出す）はそのまま残し、起動の準備の `lib.rs:411` だけが private な出さない兄弟を呼ぶ | 1 行／scope（今どおり） | +8〜12 | `switch_assets.rs` とテスト 2 本は変わらない |
| **S2**（pub の形を変える） | `load_scope_windowpositions` に記録の選び（`bool` か pub の型）を足す | 呼び手が選ぶ | +3〜6 | `switch_assets.rs:235`・`placement_windowposition_vocab_tests.rs`・`reseed_tests.rs` を直す。pub の型が要り、「各クレートが private な enum を持つ」前例から外れる |
| **S0**（丸ごと黙らせる・討議の 2 の別案） | `lib.rs:748` を `load_scope_balloon_model_quiet` に替えるだけ（brief の当初の見立て） | **0 行**（要件 4.1 と食い違う） | 0（use の名前の付け替え） | なし。ただし要件 4.1・6.3 を「0 行」へ改める必要がある |

- S0 の材料: (1) シェルの着替えでは、系列の解決の記録が `placement-measure-bake-once` 以来すでに 0 行である。(2) シェルの着替えで基層の `descript.txt` が読めないとき、同じ着替えの中の `load_balloon_author_dpi`（`source.rs:211`・`read_kv_lenient`）が、パスつきの `warn!` を 1 行出す（scope の欄は無い）。(3) 着替えで読んだ値は `finish_shell` が面の定義で置き換えるのがふつう。
  - S0 で消える記録: シェルの着替えのときの scope ごとの確定の `info!`、上書き層が読めない `warn!`、上書き層の不在の `debug!`。上書き層が着替えの間に読めなくなった場合、その事象は着替えの記録から消える（次のバルーンの読み込みまで見えない）。
- S1 の材料: 「起動の準備の外の道は今どおり出す」を、名前のついた入口の違いでそのまま表せる。`switch_assets.rs` は変更 0 で済む。
- S1 の形の細部（設計で決める）: 関数を渡すか `enum` を渡すかで、`lib.rs` の `use` に `ResolvedFace`／`BalloonModel` が増えるかどうかが変わる。新しい private の型を `lib.rs` に足すと行が増える。

**トレードオフ**

- ✅ 前例（`resolve_balloon_faces_quiet`・`load_balloon_use_self_alpha_quiet`）と同じ型で、読み込みの中身は 1 本のまま。
- ✅ 約束 ⑹ の禁止ファイルに触れない。`PreparedPlacement` は素の値だけを運ぶ、という取り決めも保てる。
- ❌ ファイルを読む回数は減らない（面 0 の上書き層を scope ごとに 2 回読むのは今のまま）。
- ❌ 「2 つの道が同じ定義を見る」の保証が、記録の値の一致からテストへ移る（要件 5）。テストの置き方しだいで「起動の準備の結線が本当に出さない入口を通っているか」の檻が別に要る（下の 3.4）。

### 3.2 案 2（読むのを 1 回にして `BalloonModel` を運ぶ）

- 起動の順は、配置の準備（`prepare_ghost_windows`・窓を作る閉包を組む）が先で、資産の組み立て（`boot_ghost` → `wire_runtime` → `build_balloon_assets`）が後である。運ぶ向きは「配置 → 資産」になる。
- 通る所（少なくとも）:
  - `PreparedPlacement`／`StartupDescriptValues`（`ghost_session.rs:308`）に面 0 の模型を載せる。今は「素の値だけを運ぶ」取り決め。
  - `boot_ghost`／`boot_ghost_strict` → `wire_runtime`（`crates/areka-runtime/src/lib.rs:428`）→ `build_boot_assets_for` → `build_boot_assets_with_shell` → `build_balloon_assets` → `load_scope_balloon_definitions` に「面 0 は渡された模型を使う」口を足す。
  - 着替えの道（`switch_assets.rs` の `build_balloon`）は運ぶ元が無いので、今の読みを残す（道が 2 つに分かれる）。
- 逆向きの前例はすでにある。シェルの着替えの `finish_shell` は、定義の束（資産）から `windowposition` を取って配置へ渡している。ただし起動では資産が後なので、そのまま流用はできない。
- **トレードオフ**:
  - ✅ 重なりそのものが消え、一致はテストではなく構造で保証される。
  - ❌ `ghost_session.rs`・`assets.rs` に入るので、約束 ⑹ により `ghost-alias-file` の着地の後へ回すことになる。規模は M（5〜8 タスクの見込み）に上がる。
  - ❌ 起動の結線の公開の形（`wire_runtime` の入力・`StartupDescriptValues`）が変わる。

### 3.3 案 1'（資産の組み立ての面 0 を黙らせる）＝比べのための参考

- 窓の位置の読みは出す入口のまま残し、資産の組み立ての面 0 のマージを黙らせる。シェルの着替えの記録は自然に今どおりになる。
- ただし `balloon_definitions.rs`（核の呼び出し）を変えることになり、着替えのバルーンの道（`build_balloon_assets` を共有）まで黙ってしまう。それを避けるには `assets.rs`（禁止）か `load_scope_balloon_definitions` の形を変える必要がある。
- `placement-measure-bake-once` の設計 D2（「表示に使う読み込みの側で記録を出し、採寸と窓の位置の読みを黙らせる」）とも向きが逆になる。挙げるだけにとどめる。

### 3.4 テストの置き場の選択肢（要件 5・6）

| 檻 | 置き場の案 | 材料 |
|---|---|---|
| 要件 1.1・2.1（本番の道・emo2） | **既存の `boot_shell_tests.rs` の `2 * n` → `n`**（どの案でも直しが要る） | 起動・動く絵の隠しシェル・ゴーストの切り替えの 3 本が、本番の順でそのまま要件 1.1・2.1 を檻に入れる。emo2 は両 scope とも上書き層を持つので、要件 1.2〜1.4 の検体にはならない |
| 要件 6.1（壊れた検体 4 種 × scope 2 つ） | (t1) `crates/areka-runtime/src/placement_tests/` の新しい兄弟（`measure_bake_log_tests.rs` の型・`MemoryDecoder`・`TempPath::under_target`）。起動の準備の窓の位置の読み（出さない側）と `build_balloon_assets_with_limits` を捕捉して、（文言, scope, `file`）ごとに数える。必要なもの: `areka-placement` の `test-support` に起動の準備と同じ読みを開ける口（`lib.rs` か `test_support.rs` に +5 前後）と、`placement_tests/mod.rs` の 1 行 | 1 か所で「合わせて 1 行」を固定できる。ただし `test-support` の口が本当に `prepare_stages_for_shell` と同じ関数を呼んでいるかは、結線の檻（上の `boot_shell_tests.rs`）が別に支える |
| | (t2) クレートごとに分ける。`areka-placement` の新しいテスト（`lib.rs` から宣言 +3）は起動の準備の読みが 2 層マージの記録 0 行、`areka-emo-present` のテスト（`balloon_quiet_tests.rs` か新しい兄弟）は出さない入口が 0 行で模型が等しく、定義の束が scope／面ごとに 1 行 | 新しい公開の口が要らない。「合わせて 1 行」は組み合わせで示すことになる |
| | (t3) `boot_shell_tests.rs` に壊れた検体を足し、本番の道で数える | 実の WIC で読める絵が要る（emo2 の写しを壊す形）。いちばん本番に近いが、重くなる |
| 要件 6.2（出さない道でも見送りの `warn!` と系列の `error!`） | `areka-placement` のテスト（既存の `load_scope_windowpositions_warns_and_skips_an_unreadable_balloon`（`placement_windowposition_vocab_tests.rs:240`）の出さない版） | private な兄弟は crate の中のテストからしか呼べない |
| 要件 6.3（シェルの着替えは 1 行／scope） | (u1) `areka-placement` のテストで pub の `load_scope_windowpositions` を数える (u2) `switch_assets_tests.rs` の型で `build_shell` を捕捉して数える（`switch_assets.rs` から新しい兄弟を宣言 +3） | (u1) は軽いが、`switch_assets.rs` が出す入口を呼び続けることの檻にはならない。(u2) は結線まで押さえるが、WIC と検体の根が要る |
| 要件 5.1〜5.3（2 つの道の面 0 の定義が等しい） | (v1) `areka-emo-present` で `load_scope_balloon_model_quiet` の模型と `load_scope_balloon_definitions(..).face0().model` を丸ごと比べる（上書き層あり・なしの検体） (v2) `areka-placement` で `windowposition` の対を比べる（`validrect` は窓の位置の読みが持たない） | 要件 5.1 は `validrect` も名指しているので、(v1) の丸ごとの比べ（`PartialEq`）がそのまま合う。面 0 の選び方が同じことは、`resolve_balloon_faces_quiet` と `resolve_balloon_faces` の戻り値の一致の檻（`balloon_quiet_tests.rs`）が既に持つ |

- テストのファイル名は structure.md の兄弟の規則に従う。`balloon.rs` の兄弟なら `balloon_*_tests.rs`。`balloon_definitions.rs` があるので、最長の stem で読み違えない名前にする必要がある。`areka-placement` の `lib.rs` の兄弟は、その crate の慣例で `placement_*_tests.rs`。

## 4. 要件と資産の対応（ギャップの印: Missing／Unknown／Constraint）

| 要件 | 今ある資産 | ギャップ |
|---|---|---|
| 1.1 確定の `info!` を scope ごとに 1 行 | 核の `info!`（`balloon.rs:546`）・本番の道の檻（`boot_shell_tests.rs`） | **Missing**: 出さない入口。**Constraint**: 既存の檻が 2 行を固定している（直しが要る） |
| 1.2 基層が読めない `warn!` を 1 行 | `read_descript_layer` の `records` | **Missing**: `load_scope_balloon_model` が `Emit` 決め打ち。注意: 同じ起動の中で `load_balloon_author_dpi` も別の文言で `warn!` を出すので、檻は文言で数える |
| 1.3 上書き層が不在以外の理由で読めない `warn!` を 1 行 | `read_face_override_layer` | **Missing**: `records` の口 |
| 1.4 上書き層の不在の `debug!` を 1 行 | 同上 | **Missing**: 同上（討議の 3 の決め次第） |
| 1.5 ほかの面は面ごとに 1 行 | `load_scope_balloon_definitions:99〜102` | 変更不要（窓の位置の読みは面 0 だけ） |
| 1.6 scope をまたいで畳まない | 資産の組み立ては scope ごとに基層を読む（`balloon_definitions.rs:94`） | 変更不要。補足: 配置の scope の集まりは `detect_scopes` 由来（`kero.*` の無いゴーストは scope 0 だけ）、資産は `[0, 1]`。今は scope 1 が 1 行・scope 0 が 2 行で、直した後は両方 1 行 |
| 2.1 切り替え・読み込み直し | 2.3 のとおり同じ道 | 変更不要（起動の直しがそのまま効く）。SHIORI の読み込み直しは 0 行のまま |
| 3.1 水準・文言・欄を変えない | — | Constraint: `if records.emits()` で囲むだけにする |
| 3.2 警告を 0 行にしない | 資産の組み立てが出す | 変更不要 |
| 3.3 結果を変えない | `default_basis_from_boot_equals_the_one_from_shell_switch` ほか | 出さない入口の戻り値の一致の檻が要る（Missing・前例の型あり） |
| 3.4 出さない道でも見送りの `warn!` と系列の `error!` | `lib.rs:729`・`741` の `warn!` は無条件。`resolve_balloon_faces_with` の `error!` も無条件 | 変更不要。檻だけ足す（6.2） |
| 3.5 起動を止める失敗の記録 | 2 層マージには `Err` の道が無い。起動を止めるのは採寸（`measure_scope_sizes`）で、変わらない | **Unknown（言葉の確かめ）**: 2 層マージの側では空の要件になる。設計でそう明記するか、要件の言い回しを討議で整えるか |
| 4.1 シェルの着替えは 1 行／scope | `switch_assets.rs:235` が pub の出す入口を呼ぶ | S1 なら変更不要。S0 なら要件を改める（討議の 2） |
| 4.2 バルーンの着替え | 2.4 のとおり | 変更不要 |
| 4.3 起動の準備の外の入口 | 2.5 のとおり | 変更不要（出す入口の名前と形を保つ） |
| 5.1〜5.3 一致のテスト | `BalloonModel: PartialEq`・`FaceDefinition: PartialEq` | **Missing**: テスト（3.4 の v1／v2） |
| 5.4 説明の書き直し | `balloon.rs:516〜520` | **Missing**: 書き直し。ほかに直す文: `boot_shell_tests.rs:316〜320`、`lib.rs:704〜707`（「採寸・起動時資産構築と同じ規則で同じ面を見る」は今も正しいので、残すかどうか）、`load_scope_windowpositions` の説明（547〜558）に起動の準備は出さない兄弟を通ることを書くか |
| 6.1〜6.5 行の数の檻 | `measure_bake_log_tests.rs` の型・`balloon_quiet_tests.rs` の型・`log_capture_kit::capture` | **Missing**: 檻（置き場は 3.4）。上書き層が不在以外の理由で読めない検体は、同名のフォルダを置く手（`balloon_model_tests.rs:201`）が使える |

## 5. 規模と危うさ

- **規模: S**（案 1 で 3〜5 タスク・1〜2 日）。前例の型（出さない入口・`balloon_quiet_tests.rs`・`measure_bake_log_tests.rs`）がそろっていて、依存の追加も実機も静かな机も要らない。案 2 は **M**（配置 → 起動の結線 → 資産の 3 層をまたぐ形の変更）。
- **危うさ: Low**（案 1）。記録の行を条件で囲むだけで、分岐と戻り値は変わらない。檻 `default_basis_from_boot_equals_the_one_from_shell_switch` と新しい一致の檻が、値のずれを捕まえる。残る危うさは 2 点。
  - (1) 既存の `boot_shell_tests.rs` の直しが約束 ⑹ の範囲の外にある。
  - (2) S1 で `lib.rs` の行が brief の見立てより増える（上限 1,000 の内ではある）。

## 6. 設計へ持ち越す調べ（Research Needed）

1. S1 の形（関数を渡すか private な `enum` か・名前）と、`lib.rs` の正確な増え。rustfmt で折り返した後の行で数える。
2. 3.4 のテストの置き場の組み合わせ（t1／t2／t3・u1／u2・v1／v2）と、`test-support` の口を開けるかどうか。
3. 要件 6.1 の「基層が読めない」検体で、採寸と系列の解決が通るか。採寸は面の絵を読むので `descript.txt` が無くても通る見込みだが、t1 なら採寸を通さずに済む。t3 なら確かめが要る。
4. `boot_shell_tests.rs` の直しが、ウェーブ C29 のほかの 7 本の触るファイルと重ならないことの最終の確かめ。今の roadmap の表の上では重なりは見当たらない。

## 7. 討議へ渡す決めごと（候補）

1. **R6.3 の扱い（案 1 か案 2 か）**: 本分析は案 1 が約束 ⑹ の内に収まり、案 2 は `ghost_session.rs`・`assets.rs` に入ることを確かめた（3.2）。
2. **シェルの着替えの記録（要件 4.1）**: S1（今どおり 1 行／scope・`lib.rs` は +8〜12 行）か S0（0 行・`lib.rs` は use の付け替えだけ）か。S0 の材料: 系列の解決の記録は前例で既に 0 行になっていること、基層が読めないときは `load_balloon_author_dpi` の `warn!` が別に 1 行残ること、着替えで読んだ値はふつう面の定義で置き換わること。S0 の欠点: 上書き層の事象と scope ごとの確定の記録は、着替えでは見えなくなる。
3. **上書き層の不在の `debug!`（要件 1.4）を 1 行に含めるか**: 核の `records` を通す形なら、含めても含めなくても同じ手間で書ける。
4. **既存の檻 `boot_shell_tests.rs` の直しを本 spec の範囲に入れ、約束 ⑹ の「触ってよいファイル」に足すか**: 直さないと、どの案でも赤になる。roadmap の C29 の行の ⑧ の触るファイルの欄を直すかどうかも合わせて決める。
5. **テストの置き場（3.4）**: 本番の道の檻は既存の `boot_shell_tests.rs`、壊れた検体の数は t1（runtime の新しい兄弟＋`test-support` の口）か t2（crate ごとに分ける）か。シェルの着替えは u1 か u2 か。一致は v1 か v2 か。
6. **テストの文言の中の要件の番号の範囲**: 「R6.3:」の 3 か所だけを外すか、同じファイルのほかの番号（R2.2・R2.4・R2.5・D8・要件 3.10/10.9）まで外すか。後者は決まりの「開発者が範囲を名指したときだけ広げる」に当たるので、名指しが要る。
7. **要件 3.5 の言い回し**: 2 層マージには起動を止める失敗の道が無い。設計で「該当する道は採寸で、変わらない」と書くだけにするか、要件の文を整えるか。
8. **brief の見立ての読み替え**: 「`lib.rs` は行が増えない」は S0 のときだけ成り立つ。S1 なら +8〜12 行。決めは要らず、読み替えとして記録に残す。

## 8. 設計の判断（2026-11-28・design 段）

> 発見は light（既存の仕組みの拡張）。外の依存・新しい道具は無いので、外の調べはしていない。使ったのは `kiro-spec-design` の規則（design-principles・discovery-light・synthesis・review-gate）と、steering の `structure.md` のテストの兄弟の命名の規則だけ。要件討議の決め（案 1・シェルの着替えは今どおり・不在の `debug!` も 1 行・`boot_shell_tests.rs` を範囲に入れる・「R6.3:」の 3 か所だけ）を前提にした。

### 8.1 選んだ形

| 決め | 選んだもの | 理由 | 選ばなかったもの |
|---|---|---|---|
| 核の形（§3.1） | **e1**: `scope_balloon_model_with` の名前と形を保つ Emit のラッパにし、`records` を受ける private の核 `merge_scope_layers` を別に置く。出す入口と出さない入口は private の `load_scope_balloon_model_with(.., records)` を通す | `balloon_definitions.rs` は約束 ⑹ の触ってよいファイルに無い | e2（`balloon_definitions.rs:102` の 1 行を直す・関数が 1 つ減る）。設計の討議で確かめる |
| 上書き層の読みの切り替え | `read_face_override_layer` に `records` を足し、`match` に守りの腕 `Err(_) if !records.emits() => None` を 1 本足す | 不在の `debug!` と不在以外の `warn!` の 2 腕を両方 `if` で囲むより行が少なく、2 腕の水準の分岐はそのまま残る | 2 腕をそれぞれ `if records.emits()` で囲む（+4 行） |
| placement の形（§3.1 S1） | private `load_scope_windowpositions_with(balloon_root, scope_ids, merge: fn(&Path, u32, &ResolvedFace) -> BalloonModel)`。pub の `load_scope_windowpositions` はその `load_scope_balloon_model` 版のラッパ。411 だけが `load_scope_balloon_model_quiet` を渡す | 足す型が無い・選んだ入口の名前が呼び出しの場に出る・`switch_assets.rs` とテスト 2 本は変わらない | private の `enum`（型と `match` で行が増える）・`bool`（呼び出しの場で読めない）・S2（pub の形が変わり呼び手 3 か所を直す） |
| 起動の準備 1 回の行の数（§3.4 の 6.1） | **t2 の 1 か所版**: `areka-placement` の新しい兄弟 `placement_balloon_merge_log_tests.rs` で、起動の準備の読み（private の中身に出さない入口を渡す）と、資産の組み立てが scope ごとに呼ぶ `load_scope_balloon_definitions` を 1 回の捕捉の中で通して数える | 新しい公開の口が要らない・約束 ⑹ の内（宣言は `lib.rs`）・「合わせて 1 行」を 1 か所で固定できる。411 の結線は `boot_shell_tests.rs`（emo2・本番の順）が支える | t1（`placement_tests/mod.rs` と `test-support` の口が要る・約束 ⑹ の外）・t3（実の WIC で読める絵が要る） |
| 出さない道の失敗（6.2） | 同じ新しい兄弟で、出さない入口を渡した中身を、フォルダが無いバルーンと面 0 が無いバルーンで通す | private の中身は crate の中のテストからしか呼べない | — |
| シェルの着替え（6.3） | **u1**: 同じ新しい兄弟で pub の `load_scope_windowpositions` を数える | `switch_assets.rs` を変えない | u2（`switch_assets.rs` に宣言が要る・約束 ⑹ の外） |
| 一致（5.1〜5.3） | **v1**: `areka-emo-present` の新しい兄弟 `balloon_merge_quiet_tests.rs` で、出さない入口の模型と `load_scope_balloon_definitions(..).face0().model` を丸ごと比べる（上書き層あり・なしの scope）。同じファイルに出さない入口の無音と戻り値の一致の檻も置く | 要件 5.1 は `validrect` も名指す。`BalloonModel: PartialEq` で丸ごと比べられる | v2（窓の位置の読みは `validrect` を持たない） |
| テストのファイル名 | `balloon_merge_quiet_tests.rs`（`mod merge_quiet_tests`）・`placement_balloon_merge_log_tests.rs`（`mod balloon_merge_log_tests`） | `balloon_merge.rs` は無いので逆向きに読み違えない。placement は crate の慣例の `placement_*` | `balloon_quiet_tests.rs` へ足す（約束 ⑹ の外） |

### 8.2 行の数（rustfmt の折り返しの後の見込み）

- `crates/areka-placement/src/lib.rs`: 896 → **915〜919**（+19〜23）。内訳: `use areka_emo_present::balloon::{…}` が 4 つの名前で 100 桁を越えて 4 行に折れる +3、411 の呼び出しの折り返しと説明 +2、pub の本体 −3、新しい中身 +13〜15（`filter_map` の行がちょうど 100 桁で、折れれば +2）、`scope_windowposition` の署名 +1、説明 0〜+1、テストの宣言 +3。`use areka_parsers::balloon::{BalloonModel, …}` は 78 桁で 1 行のまま。
- `crates/areka-emo-present/src/balloon.rs`: 793 → **830〜845**（+37〜52）。内訳: 出さない入口 +11、`load_scope_balloon_model_with` +12、核の分け直し +14〜18、上書き層の読みの署名の折り返しと守りの腕 +6、出す入口の本体 −2、説明 +1、テストの宣言 +3。
- brief の見立て「`lib.rs` は行が増えない」は S0 のときだけ成り立つ（§7 の 8）。要件討議の 2 の「数行」は実際には約 20 行になる。上限 1,000 の内。

### 8.3 統合（synthesis）

- **一般化**: 要件 1.1〜1.4 は「2 層マージの記録 4 種を同じ 1 つの口で切り替える」1 つの問題。核まで `LoadRecords` を通す 1 本の形で 4 種をまとめて扱い、種ごとの口は作らない。
- **作るか採るか**: 前例の `LoadRecords` と「中身 1 本＋薄い入口」の形をそのまま採る。新しい型・依存は無い。
- **簡素化**: `test-support` の口・runtime の新しいテスト・private の `enum` を足さない。pub の形（`load_scope_balloon_model`・`load_scope_windowpositions`）は変えない。

### 8.4 調べで分かったこと（§6 の持ち越し）

1. S1 の形と行の数は 8.1・8.2 のとおり。
2. テストの置き場は 8.1 のとおり。`test-support` の口は開けない。
3. 基層が読めない検体は採寸を通さない（t2 の 1 か所版なので採寸の通りを確かめる必要が無い）。
4. 同じ文言「balloon: バルーン既定設定（descript.txt）の読取に失敗（空層で継続）」は、透過の宣言の読み（`load_balloon_use_self_alpha`・資産の組み立てでバルーン 1 つにつき 1 回）も出す。基層が読めないバルーンで scope を 2 つ読む起動では、文言で数えると今の 5 行が 3 行（2 層マージ 2・透過の宣言の読み 1）になる。透過の宣言の読みは範囲の外（要件の境界）なので変えない。行の数の檻は透過の宣言の読みを通さないので、2 層マージの 2 行だけを数える。
5. `areka-placement` の通常の依存に `areka-emo-atlas` があるので、テストは `MemoryDecoder`・`AlphaParams`・`UseSelfAlpha` をそのまま使える（dev-dependency を足さない）。`load_scope_balloon_definitions` は `areka_emo_present::balloon_definitions` から pub で引ける。
6. `log_capture_kit::capture` は `debug!` も捕まえる（`balloon_model_tests.rs` の不在の `debug!` の檻が前例）。

### 8.5 設計の討議へ渡す前提

design.md の「前提（設計の討議で確かめる）」の 5 件（e1・行の数の檻の置き場・u1・透過の宣言の読みの同じ文言・`lib.rs` の約 20 行）。
