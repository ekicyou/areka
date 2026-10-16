# ギャップ分析: areka-P0-dev-helper-x64-clobber

> 2026-10-16・main `1e6590d8` の上のワークツリー（`51c06e52`）で、要件（requirements.md）と今のコードを見比べた。重いテスト・全体テスト・実機は回していない。使ったのは、ファイルを読むことと、ビルドをしない `cargo +nightly ... -Z unstable-options --unit-graph`（cargo が何を作るつもりかの一覧を出すだけ）の 2 回だけ。

## 1. 要約

- **全体テストそのものは、`target\debug\shiori-host32-helper.exe` を x64 版にしない**（静的に確かめた・下の 3 節）。x64 の段（`cargo test --workspace`）では helper は「テスト用の実行ファイル」としてだけ作られ、`target\debug\` へは出ない。むしろ同じ段の中で煙テストと MCP の `get_log` の実走が i686 版を `areka.exe` の隣へ写すので、**全体テストが最後まで回れば、今でも隣は i686 版で終わるはず**である。
- **x64 版を隣へ置くのは素の `cargo build`**（ワークスペースの根で `-p` なし）。ワークスペースに `default-members` が無いので全クレートが対象になり、helper も x64 で作られて `target\debug\` へ出る。しかも cargo は「作り直しが要らない」ときでも出力を `target\debug\` へ結び直すので、**全体テストの後に一度でも `cargo build` を打てば、写した i686 版が x64 版へ戻る**。README（78 行）と steering の `tech.md`（118 行）は、ビルドの手順として素の `cargo build` を書いている。
- したがって要件 1（全体テストの終わりの状態）は「今の成り行きで多くの場合は満たされているものを、成り行きに頼らず確かめて、だめなら赤の行にする」仕事になる。開発者が実際に踏む罠（全体テストの後の `cargo build`）を知らせるのは要件 3（areka の記録）の役目で、こちらが本筋になる。
- 足りない物は 2 つ。① 全体テストの最後に、隣の helper の向きを確かめて揃える段。② areka の本体（テストではない側）で helper の向きを見る手立て（今は 0 か所。テストの側に同じ判定が 2 つ写しで在るだけ）。どちらも新しい依存は要らない。
- 規模は S（3〜5 タスク）・リスクは低。決めることは「記録を出す場所（`areka-ghost` の `shiori_wiring.rs` か `shiori-host32-host` の `process_host.rs` か）」「向きの見分け方（実行ファイルの見出しを自分で読むか、OS の `GetBinaryTypeW` に問うか）」「全体テストの段の置き場と、x64 の段から helper を外すか」。

## 2. 調べた物

| 場所 | 中身 |
|---|---|
| `tools/test-all.ps1`（74 行） | 段の並び（45〜58 行）。x64 の段は 49 行の `cargo test --workspace --no-fail-fast -j 4`。i686 の成果物は 46 行で `--target i686-pc-windows-msvc` へ作る。結果の一覧は 60〜67 行で ASCII |
| `Cargo.toml`（根） | `members = ["crates/*"]` だけで `default-members` が無い。`.cargo/config.toml` も無い（`CARGO_TARGET_DIR` も未設定） |
| `crates/shiori-host32-helper/` | bin 1 つ（`src/main.rs`）だけ。`tests/`・`examples/`・`benches/` は無い。テストの `cfg` は `main.rs` 587 行の `cfg(all(test, target_arch = "x86"))`（約 42 秒の檻）だけで、x64 でしか走らないテストは無い |
| `crates/areka/build.rs` | `areka.manifest` を埋めるだけ。helper を写す処理は無い |
| `crates/areka/tests/smoke_boot_loop_exit.rs` | `is_i686_pe`（143 行）・`ensure_i686_artifact`（158 行・一時名へ写して改名で置く）・`ensure_helper_beside_areka`（200 行）。①②④ の 3 本のテスト（286・321・391 行）が最初に呼ぶ。環境変数の門は無く、常に走る |
| `crates/areka/tests/mcp_get_log_real_run.rs` | 同じ判定 `is_i686_pe`（57 行）と `ensure_helper_beside_areka`（71 行）の写し。222 行のテストが呼ぶ |
| `crates/areka/src/boot_config.rs` 414 行 | `default_helper_exe_path`（`exe_location` の親＋ファイル名）。本仕様は触らない（`baseware-root-list` の席） |
| `crates/areka/src/main.rs` 194 行 | `let helper_exe = default_helper_exe_path();` → `ShioriWiring::Helper { helper_exe }` |
| `crates/areka-ghost/src/runtime.rs` 657 行 | `ShioriWiring::Helper` のとき `shiori_wiring::real_connect` を組む |
| `crates/areka-ghost/src/shiori_wiring.rs`（299 行） | `real_connect`（130 行）: 親窓 → `spawn`（154 行）→ HELLO 待ち → LOAD の ack が `[1]` か（170〜173 行の文「SHIORI の LOAD が成功 ack [1] を返さなかった（proxy 未確立）: [0]」）。記録の宛先 `ghost-boot`（34 行）。兄弟のテストに「helper が無いパス」の検査（273 行）がある |
| `crates/shiori-host32-host/src/process_host.rs`（632 行） | `spawn`（248 行・`Command` を組むだけ）→ `spawn_command`（284 行・起こして job に入れる）。依存は `tracing` を持つ。`windows` の機能に `Win32_Storage_FileSystem` は無い |
| `crates/areka-kanade/src/shiori/real.rs` 418〜426 行 | `real_connect` の `Err` を `error!`（`event = "connect_failed"`）で記録する所。今の「SHIORI を読めなかった」ときの扱い（`ShioriDown`）もここ |
| `crates/areka/src/placement/transition_judge_offset_signoff_tests.rs` 60〜95 行 | 「ワークスペース全体をビルドすると 64bit の helper が隣に置かれる」実測（2026-08-28）と、PE の machine 欄で確かめる PowerShell の手順 |
| `tools/package.ps1` 351 行 `Read-PeInfo` | 配布物の機械の種類（i686＝`0x014c`）を確かめる PowerShell の読み手。test-all からは呼べない（スクリプトの中の関数） |
| `.kiro/steering/roadmap.md` 147 行 | ウェーブ C7 の ⑥ の約束＝「`tools/test-all.ps1` だけ（案 1・2）。`boot_config.rs` で直す案 3 は取らない（①）＝その形が要るなら `process_host.rs` の側」 |

## 3. どの操作が x64 版を隣へ置くか（要件生成時の未確定点）

### 3.1 確かめ方

cargo の作るつもりの一覧（ビルドはしない）を 2 通り出し、helper の部品だけを見た。

| 操作 | helper の部品 | `target\debug\shiori-host32-helper.exe` へ出るか |
|---|---|---|
| `cargo test --workspace --no-run`（全体テストの x64 の段と同じ選び方） | `mode=test` の 1 つだけ | 出ない |
| `cargo build`（根・`-p` なし） | `mode=build` の 1 つ | 出る（x64） |

裏付けの cargo の決まり（cargo の今のソース・手元は cargo 1.99.1）:

- テスト用の実行ファイルは `target\debug\` へ出さない（`src/compiler/build_runner/compilation_files.rs` の `uplift_to`「Tests, check, doc, etc. should not be uplifted」＝`mode` が `Build` のときだけ出す）。
- 結合テスト（`tests/`）があるクレートは、**同じクレートの** bin を `Build` で作る（`src/compiler/unit_dependencies.rs` の「If any integration tests/benches are being run, make sure that binaries are built as well」＝`unit.pkg.targets()` だけ）。helper のクレートには結合テストが無いので、x64 の段で helper の bin は `Build` にならない。`areka.exe` は `areka` に結合テストがあるので `Build` で作られて `target\debug\` に出る。
- 作り直しが要らない（fresh）ときも出力を結び直す（`src/compiler/mod.rs` の「Need to link targets on both the dirty and fresh」）。

### 3.2 全体テストの 1 回の中の流れ

1. 46 行: i686 の helper を `target\i686-pc-windows-msvc\debug\` に作る（`target\debug\` は触らない）。
2. 49 行の組み立て: `areka.exe` を `target\debug\` へ出す。helper はテスト用の実行ファイルだけ（`target\debug\deps\`）。**隣の helper は前の状態のまま**。
3. 49 行のテストの実行: `areka` の煙テスト ①②④ と `get_log` の実走が、隣が i686 版でなければ i686 版を写して置く（一時名へ写して改名＝置き換え）。
4. 50 行以降: i686 の段・公開前の確認（`--target-dir` は別の場所）・文字コードの判定などは `target\debug\` を触らない。`-License` の 2 段もビルドしない。

→ 全体テストが 3 まで進めば、終わったときの隣は i686 版である。3 まで進まない（x64 の組み立てが赤で `areka` のテストが動かない・煙テストの手前で止まる など）と、隣は前の状態のまま（前に `cargo build` を打っていれば x64 版）で終わる。

### 3.3 開発者が踏む道筋の読み

- 全体テストの後に素の `cargo build`（README と `tech.md` の手順どおり）を打つと、helper が x64 で作られる（または fresh のまま結び直される）ので、写した i686 版が x64 版へ戻る。2026-10-06 の `ghost-standard-balloon` 6.2 の「全体テストの後は x64 版」は、この順（全体テスト → `cargo build` → 実機）だった読みが最も自然である。**確かめてはいない**（その日の操作の記録が無い）。
- `cargo build -p areka`・`cargo run -p areka`・`cargo test -p areka` は helper を作らない（`areka` は helper を依存に持たない）。`cargo clippy`・`cargo check` は実行ファイルを出さない。`cargo build --workspace`・`cargo build -p shiori-host32-helper`（x64）は出す。
- 結論: 要件が「どの操作で x64 版になるかに依らない形」で書かれているのは正しい。全体テストの側の直し（要件 1）だけでは開発者の罠は消えず、要件 3 の記録が罠を知らせる主役になる。

## 4. 要件と今の物の対応

| 要件 | 今ある物 | 足りない物 | 区分 |
|---|---|---|---|
| 1.1 終わったとき隣が i686 版 | 煙テストと `get_log` の実走が成り行きで写す（3.2） | 成り行きに頼らない最後の確かめと揃え | Missing |
| 1.2 途中が赤でも 1.1 | `Step` は止めない（38〜43 行） | 揃えの段を i686 のビルドの赤以外に依らない形で置く | Missing |
| 1.3 揃えられないときの赤の行と終了コード 1 | 一覧と `exit 1` の仕組み（60〜72 行） | 揃えの段が終了コードを返すこと（`$LASTEXITCODE` を立てる書き方） | Missing（仕組みは流用） |
| 1.4 直後に claudia を起こせる | 3.2 のとおり多くは今も成り立つ | 手で 1 回起こす確かめ（実機・重い回 0） | Constraint（確かめは手） |
| 1.5 新しい文は ASCII | 今の一覧の行は ASCII | 新しい段の名前と文を ASCII で書く | 規律 |
| 2.1 段を外さない | — | 足すだけにする | 規律 |
| 2.2 helper の単体テストを少なくとも 1 回 | 50 行の i686 の段で走る・x64 でしか走らないテストは無い | x64 の段から helper を外すなら、外しても減らないことの記録 | Constraint |
| 2.3 隣に揃えて使うテストを今と同じ条件で | 煙テスト・`get_log` の実走は自分で揃える | 最後の段で揃えるだけなら影響しない（49 行の中の順は変えない） | 影響なし |
| 2.4 約 42 秒の檻は i686 の段だけ | `main.rs` 587 行の `cfg` | 触らない | 影響なし |
| 3.1 読む前に error の 1 行 | 本体側の判定は 0 か所 | 判定と記録（`spawn` の前） | Missing |
| 3.2 1 行にパス・理由・直し方 | — | 文の組み立て | Missing |
| 3.3 今と同じ扱いで続ける | `real.rs` の `connect_failed` → `ShioriDown` | 記録の後もそのまま起こして今の失敗の道を通す（早く `Err` にするかは決めごと） | 決めごと |
| 3.4 i686 のときは記録しない・起こし方を変えない | — | 判定が i686 なら何もしない | Missing |
| 3.5 確かめられないときの 1 行 | — | 読めない・PE でないときの 1 行。**ファイルが無いとき**の扱い（今は `spawn` の失敗の文だけで、パスが文に入らない） | 決めごと |
| 3.6 探す場所を広げない・外から指す口を足さない | `default_helper_exe_path` は触らない | 守るだけ | 規律 |

## 5. 取り得る形

### 5.1 全体テストの側（要件 1・2）

**形 T1: 最後に「隣の helper を i686 に揃える」段を 1 つ足す（`tools/test-all.ps1` だけ）**

- 段の中身: `target\debug\shiori-host32-helper.exe` の機械の種類を見る → i686（`0x014c`）ならそのまま → 違う・無いなら `target\i686-pc-windows-msvc\debug\` の i686 版を一時名へ写して改名で置く（テストと同じ置き方）→ 置いた物をもう一度見て i686 でなければ赤。
- 置き場: すべての cargo の段の後（`-License` の 2 段もビルドしないので、その前でも後でも結果は同じ。今の一覧の並びの最後に置くのが読みやすい）。
- 機械の種類の読み手は PowerShell で数行（`package.ps1` の `Read-PeInfo` は別のスクリプトの中なので呼べない。`transition_judge_offset_signoff_tests.rs` の 5 行の手順と同じ形）。
- 良い点: 1 ファイル・十数行。煙テストの成り行きに頼らずに 1.1〜1.3 が成り立つ。
- 気を付ける点: 隣の helper を開発者の areka が動かしている最中だと、改名が断られる（赤の行になる＝1.3 のとおり）。`Step` の決まり（`$LASTEXITCODE` を結果にする）に合わせ、段の中で終了コードを立てる書き方が要る（PowerShell の文だけの段は `$LASTEXITCODE` が前の段のまま残る）。

**形 T2: T1 に加えて、x64 の段から helper を外す（`--exclude shiori-host32-helper`）**

- 3.1 のとおり、外しても `target\debug\` の helper の状態は変わらない（もともと x64 の段は helper を `target\debug\` へ出さない）。brief の「案 ⑴ が 1 行で取れる」の前提（x64 の段が helper を作るので外せば上書きが止まる）は当たらない。
- 得るのは x64 の段の時間が helper の単体テストの分だけ短くなることだけ。x64 でしか走らないテストは無いので 2.2 は保てる。
- 本仕様の目的には効かないので、取るなら「時間の都合」として別に決める。

**形 T3（要件の外・参考）: 素の `cargo build` が helper を作らないようにする（根の `Cargo.toml` に `default-members`）**

- 開発者の本当の罠（全体テストの後の `cargo build`）を元から断てる唯一の形。ただし境界（`tools/test-all.ps1` だけの約束・「全体テスト以外の操作で作った x64 版は記録で知らせるところまで」）の外で、要件の改訂が要る。
- 欠点: `members = ["crates/*"]` の 32 クレートを列挙し直す保守（新しいクレートの書き忘れが黙って `cargo build`・素の `cargo test` から漏れる）。根の `Cargo.toml` は版上げの PR が触るファイル。`default-members` で glob が使えるかは要確認。
- 似た軽い形として、README と `tech.md` の「ビルド: `cargo build`」を `cargo build -p areka` に替える案もある（文書だけ・境界の外）。

### 5.2 areka の側（要件 3）

**置き場の候補**

| 形 | 場所 | 良い点 | 気を付ける点 |
|---|---|---|---|
| R-A | `crates/areka-ghost/src/shiori_wiring.rs` の `real_connect` の中、`spawn`（154 行）の前 | helper を使う本番の道だけを通る（`Custom`・`InProc` は通らない）。記録の宛先 `ghost-boot` と兄弟のテスト（helper が無いパスの検査）が既にある。呼び手が 1 つ | 実行ファイルの見出しを読むのは「ゴーストの組み立て」の層には少し低い話 |
| R-B | `crates/shiori-host32-host/src/process_host.rs`（`spawn` の前に呼ぶ公開の判定関数を足し、`spawn` か `real_connect` から呼ぶ） | 「helper は i686 であるべき」は host-32 の知識で、ここが持ち主。roadmap の約束が名指しする側。重なりは `makoto-dll-host`（いつかの列）だけ | `spawn` の中で記録すると、helper の場所を直接渡す結合テスト（`tests/*_e2e.rs`）もこの道を通る（i686 を渡すので記録は出ない・害は無い）。`spawn` は `Command` を組むだけの下位の口なので、記録をそこへ混ぜるかは決めごと |
| R-C | `crates/areka/src/main.rs` 194 行の直後（起動に 1 回） | 1 か所で済む | `main.rs` は同じウェーブの `baseware-root-list` の席。要件 3.1 は「SHIORI を読もうとするとき」＝ゴーストごとの読み込みの前で、起動に 1 回とは合わない。取らない見込み |

**向きの見分け方の候補**

| 形 | 中身 | 良い点 | 気を付ける点 |
|---|---|---|---|
| D-1 | 標準のファイル読みで、先頭の 0x3C の 4 バイト（PE の見出しの位置）→ その位置 +4 の 2 バイト（機械の種類）を見る。`0x014c` なら i686 | テストの `is_i686_pe` と同じ判定・依存も機能の追加も 0。判定を「バイト列 → 結果」の純粋な関数にできるので、決定論のテストが合成したバイト列だけで書ける | テストの写しはファイル全体を読む。本体では先頭だけ（例: 4 KiB）を読むのが筋。`PE\0\0` の署名も確かめるかは決めごと（無ければ「実行ファイルの形をしていない」＝3.5） |
| D-2 | OS の `GetBinaryTypeW` に問う（32bit なら `SCS_32BIT_BINARY`、64bit なら `SCS_64BIT_BINARY`） | OS の受け口を使う方針に合う。読めない・実行ファイルでないは OS が `FALSE` で返す | `windows` の機能 `Win32_Storage_FileSystem` を足す（`Cargo.lock` は変わらない）。テストに実物のファイルが要る（合成の見出しを `target\` の下へ書けば足りるが、OS の判定の細部に頼る）。arm64 の areka でも helper は i686 なので「32bit か」で足りる |

**記録の後の扱い（3.3）の候補**

- そのまま `spawn` へ進む: 今の失敗の道（LOAD の ack `[0]` → `connect_failed` → `ShioriDown`）をそのまま通るので、3.3 の「今と同じ扱い」を文字どおり満たす。記録は 2 行（新しい 1 行＋今の `connect_failed`）になる。
- 判定が x64 ならその場で `Err` を返す: helper を起こさずに済むが、`connect_failed` の `reason` の文が変わる（今の文を当てにする物が無いかの確認が要る）。3.4 の「i686 のときは今と変えない」は保てる。

### 5.3 組み合わせ

- 最小: T1 ＋ R-A（または R-B）＋ D-1。触るのは `tools/test-all.ps1` と Rust の 1 ファイル（＋兄弟のテストのファイル）。新しい依存 0・`Cargo.lock` 0。
- roadmap の約束「`tools/test-all.ps1` だけ」は、要件 3 が確定した今は守れない（要件 3 は areka の本体の記録を求める）。約束が名指しする逃げ先は `process_host.rs` の側（R-B）。R-A の `shiori_wiring.rs` も同じウェーブの 8 本のどれとも重ならない（`baseware-root-list` が触るのは `areka-ghost` では `catalog.rs`）。

## 6. 規模とリスク

- 規模: **S**（3〜5 タスク）。全体テストの段 1 つ（十数行）・判定の関数と記録（数十行）・兄弟のテスト・手で 1 回の確かめ。
- リスク: **低**。既にある判定（テストの `is_i686_pe`）と既にある失敗の道をなぞるだけ。新しい依存・新しい仕組みは無い。残るのは「全体テストの最後の段が、開発者の動かしている areka に当たったとき」の扱いだけ（赤の行で知らせる＝要件 1.3 で吸収できる）。

## 7. 設計への申し送り

### 7.1 決めること（要件の討議に回す）

1. **記録を出す場所**: `areka-ghost` の `shiori_wiring.rs`（`real_connect` の `spawn` の前）か、`shiori-host32-host` の `process_host.rs`（判定の関数を持たせ、どこから呼ぶか）か。roadmap の約束は後者を名指ししている。
2. **向きの見分け方**: 見出しを自分で読む（D-1・テストと同じ判定・依存 0）か、OS の `GetBinaryTypeW`（D-2・`windows` の機能を 1 つ足す）か。D-1 なら `PE\0\0` の署名まで確かめるか。
3. **記録の後に起こすか**: そのまま `spawn` して今の失敗の道を通す（記録 2 行）か、x64 と分かった時点で `Err` を返す（`connect_failed` の文が変わる）か。要件 3.3 の文言には前者が素直に合う。
4. **helper のファイルが無いとき**: 要件 3.5 は「在るが読めない」を挙げる。無いときは今の「spawn に失敗」の文（パスが入らない）に任せるか、3.5 と同じ 1 行（パス入り）も出すか。
5. **全体テストの段の置き場と名前**: すべての cargo の段の後（一覧の最後）に置くか、`i686 tests` の直後か。段の名前（ASCII）。i686 の成果物のビルドが赤のときに、古い i686 版が残っていればそれを置いてよいか（1.3 は「i686 のビルドが赤」を揃えられない例に挙げる＝ビルドが赤なら置いても赤の行にするか）。
6. **x64 の段から helper を外すか（T2）**: 本仕様の目的には効かない（3.1）。時間の短縮だけのために入れるか、入れないか。
7. **（要件の外・参考）素の `cargo build` の扱い**: 開発者の罠の本当の入口は素の `cargo build`（3.3）。`default-members`（T3）や README・`tech.md` の手順の書き替えは境界の外。取るなら要件の改訂か、別の起票。

> 要件の討議（2026-10-16）での振り分け: 1・2・3 と 5 のうち「段の置き場・名前・終了コードの立て方」は設計フェーズで決める。4・5 のうち「i686 のビルドが赤のときの扱い」・6・7 は要件の討議の議題にした。結論（全議題を推奨案で確定）: 4＝ファイルが無いときもパス入りの 1 行（要件 3.5）。5＝揃えの段は「隣が i686 版か」だけで合否・古い i686 版でも置く（要件 1.3）。6＝x64 の段から helper を外さない（範囲外）。7＝`default-members` は取らず、README と `tech.md` の手順を `cargo build -p areka` に替える（要件 1.6）。

### 7.2 調べること（Research Needed）

- **fresh のときの結び直しの実測**: 「i686 版を写した後、作り直しの要らない `cargo build` でも x64 版へ戻る」を、最初のタスクで軽い確かめ（helper のクレートだけの `cargo build -p shiori-host32-helper` を 2 回・その間に i686 版を置いて機械の種類を見る）で確かめる。3.1 は cargo のソースからの読みで、手元の 1.99.1 での実測はしていない。
- **2026-10-06 の罠の道筋**: 全体テストの後に `cargo build` があったかは記録が無く確かめられない。実害は無い（要件は道筋に依らない）ので、確かめないままでよいかを討議で決める。
- **`default-members` で glob が使えるか**（T3 を取るときだけ）。
- **全体テストの段で終了コードを立てる書き方**: 今の `Step` は `$LASTEXITCODE` を読むので、PowerShell の文だけの段では明示に `$global:LASTEXITCODE` を立てるか、別のプロセスで回す形にするかを設計で決める（ほかの道具の段は `pwsh -NoProfile -File` で別のプロセスにしている）。

## 8. 設計フェーズの記録（2026-10-16・`/kiro-spec-design -y`）

### 8.1 要約

- **調べ方**: 軽い調べ（既存の仕組みへの追加）。サブエージェントは使わず、主の文脈でコードを読んだ（`tools/test-all.ps1`・`shiori_wiring.rs`・`process_host.rs`・`lib.rs`・`Cargo.toml`・`smoke_boot_loop_exit.rs` の `is_i686_pe`・`real.rs` の `connect_failed`・`logging.md`・README と `tech.md` のビルドの行）。外の資料は要らなかった（新しい依存・新しい API が無い）。
- **使ったスキル・決まり**: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`。steering の `logging.md`（レベルの選び方・構造化フィールド）。
- **見直しの門**: 1 回目で通った（局所の直し 1 か所＝実機の確かめで使う全体テストの 1 回を「実装の最後の検証の 1 回」と明記）。要件の穴・矛盾は無い。

### 8.2 決めたこと（7.1 の 1・2・3・5 の残り）

#### 決定: 記録を出す場所＝`shiori-host32-host` の `spawn` の入口（R-B）
- **背景**: 要件 3.1（読む前に 1 行）。候補は R-A（`real_connect` の `spawn` の前）と R-B（`process_host.rs`）。
- **選んだ形**: `process_host.rs` に判定と記録の関数を置き、`spawn` の最初の文で呼ぶ。`spawn_command`（`cmd.exe` などを起こすテスト用の下位の口）には置かない。
- **理由**: 「helper は i686 であるべき」は host-32 の知識で、持ち主は host-32 の層。`spawn` は helper を起こすすべての道が通る 1 か所なので、呼び手ごとに足さずに済む。roadmap のウェーブ C7 の約束が名指しする逃げ先でもある。`real_connect`（`areka-ghost`）・kanade・`boot_config.rs`・`main.rs` を触らない＝同じウェーブの `baseware-root-list` と重ならない。
- **引き換え**: 結合テストも `spawn` を通るが、渡すのは i686 の helper か無いパスなので、記録は 0 件か警告 1 件で害は無い。
- **記録の文の「直し方」**: 要件 3.2 の「i686 版の helper を `areka.exe` の隣へ置く」は、記録の `path` の欄（＝本番では常に `areka.exe` の隣）と「このパスへ i686 版を置く」の本文で満たす。host-32 の部品が `areka.exe` という呼び手の名前を知らずに済む。

#### 決定: 向きの見分け方＝見出しを自分で読む（D-1）・`MZ` と `PE\0\0` も確かめる
- **選んだ形**: 先頭の最大 4 KiB を std だけで読み、純粋な関数 `classify_helper_image(&[u8]) -> HelperImage { I686, OtherMachine(u16), NotPe }` で判定する。
- **理由**: 依存も `windows` の機能も足さない（`process_host` の「std だけ」の決まりを保つ）。合成したバイト列で決定論のテストが書ける。`GetBinaryTypeW`（D-2）は機能 `Win32_Storage_FileSystem` の追加と、テストに実ファイルが要るので取らない。署名まで見るのは、要件 3.5 の「実行ファイルの形をしていない」を「別の機械の種類」と分けて正しく言い分けるため（数行で済む）。
- **既存の写しの扱い**: テストの側の `is_i686_pe`（2 つの写し）は本体から呼べない形のまま触らない（テストの領分・本仕様の境界の外）。
- **上限**: `e_lfanew` が 4 KiB の外の実行ファイルは `NotPe`（警告）になる。MSVC の linker の helper は 0x100 前後。広げるなら `e_lfanew` へ seek して読む（コードに `ponytail:` の注記を残す）。

#### 決定: 記録の後も起こす（今の失敗の道をそのまま通す）
- **選んだ形**: 判定は戻り値を持たず、`spawn` の以後を変えない。x64 の helper なら今日と同じく LOAD の ack `[0]` → `connect_failed` → `ShioriDown`。
- **理由**: 要件 3.3（今と同じ扱い）・3.4（i686 のときは起こし方を変えない）・3.5（確かめられないときも起こし方を変えない）を文字どおり満たす。早く `Err` にすると `connect_failed` の `reason` の文が変わり、失敗の道が 2 本になる。「ログ無しの失敗にしない（error! + Err）」は、本当の失敗が今日の `Err` の道で記録されることで保たれる。新しい 1 行はその原因の名指し。
- **レベル**: 向きが違う＝`error!`（`helper_not_i686`・要件 3.1）。確かめられない＝`warn!`（`helper_image_unverified`・回復の見込みがある前置き。本当の失敗は続く `SpawnError` などが `error!` で記録する）。宛先は `target: "host32-helper"`（`shiori-charset`・`shiori-actor` と同じ「部品名の宛先」の形）。

#### 決定: 全体テストの段＝最後の段 `i686 helper beside areka.exe`・スクリプトの中の関数・`$global:LASTEXITCODE`
- **選んだ形**: `-License` の段の後（一覧を出す前）に 1 段。`Test-I686Pe` 関数（`MZ`・`PE\0\0`・0x014c）で置き先を見て、違えば `target\i686-pc-windows-msvc\debug\` の i686 版を一時名へ写して `Move-Item -Force` で置く。段の中のすべての出口で `$global:LASTEXITCODE` を 0 か 1 に立てる。
- **理由**: 最後の段なら、どの段が赤でも（要件 1.2）、どの段も後から `target\debug\` を触らない（要件 1.1）。別のスクリプトに分けると、ファイルが増え、roadmap の約束「`tools/test-all.ps1` だけ」から外れる。`$LASTEXITCODE` を自分で立てないと前の段の値が残る（`Step` の決まり）。
- **ASCII**: 端末の文は固定の英文。.NET の例外の本文は日本語に訳されることがあるので出さず、型の名前と `HResult` の 16 進だけを出す（要件 1.5・「道具が端末へ出す文は ASCII だけ」）。
- **写し元**: debug だけ（全体テストは debug だけを作る・`target\release\` は範囲外）。

#### 決定: 文書の書き替え（要件 1.6）
- `README.md` の `cargo build` → `cargo build -p areka`。`tech.md` の `cargo build` → `cargo build -p areka`、`cargo build --release` → `cargo build -p areka --release`（素の `cargo build` の一種なので、要件 1.6 の「素の `cargo build` を手順として示さない」に合わせる）。

### 8.3 まとめの 3 つの見方（design-synthesis）

- **一般化**: 判定の規則（`MZ`・`PE\0\0`・0x014c）は本体の Rust とスクリプトの PowerShell の 2 か所に要るが、言語が違うので 1 つにまとめない。規則を両方の設計に同じ言葉で書き、ずれを防ぐ。
- **作るか借りるか**: OS の `GetBinaryTypeW` は借りられるが、機能の追加とテストの実ファイルの手間が判定の数行より重い。標準ライブラリで作る。
- **削ったもの**: 判定を公開の API にする案（呼び手が `spawn` だけ）・`real_connect` の側の重ねての判定・別のスクリプトの道具・揃えの段の `-SelfTest`・x64 の段から helper を外す変更（範囲外）・判定の結果で起こし方を変える分岐。記録の関数を判定と読みから分けたのは、i686 のときに記録 0 件であることをファイルなしで確かめるため（テストの継ぎ目として要る分だけ）。

### 8.4 リスクと手当て

- 開発者が `target\debug\areka.exe` を動かしている最中に全体テストを回すと、揃えの段の置き換えが断られる → 赤の行（`is areka.exe running?` の案内つき）で知らせる。要件 1.3 のとおり。
- 判定の宛先や `event` の名前を後で変えると、ログの検索で確かめる下流の手順が読み替えになる → design.md の Revalidation Triggers に書いた。
- `clippy-199-lints` が同じ `tools/test-all.ps1` に段を足すとき、揃えの段より後ろに置くと要件 1.1 の保証が崩れる → 同じく Revalidation Triggers に書いた。

## 9. 設計の議論の記録（2026-10-16・`/kiro-design-discussion`）

- 検証の判定は GO（`design-validation.md`）。大事な問題 3 点のうち 2 点は自明な直しとして設計へ反映した。
  - 記録の宛先を `shiori-host32` にした。出す側の名前で、定数 `LOG_TARGET` に置く。
  - 実機の確かめは、x64 版を置いての確かめ、最後の全体テスト 1 回、写さずに claudia を起こす確かめの順で行う。
- 残る 1 点（全体テストの揃えの段の失敗の道を、毎回確かめるか）は議題 1 として出し、推奨案 ⑴ で確定した。開発者は「承認」と答えた。
  - 決めたこと: 段の中身は新しい道具 `tools/sync-i686-helper.ps1` に移す。その道具の `-SelfTest` を、全体テストの段 `i686 helper sync check` で毎回回す。偽の小さな PE で 6 つの分かれ道（揃っていた・置いた・置き先が無い・写し元が無い・写し元が x64・置き換えが断られた）を確かめる。
  - 理由
    - 決定論で確かめられる判断の分かれ道は、テストで守る決まりがある。
    - `release-notes.ps1`・`roadmap-overview.ps1` の `-SelfTest` と同じ形になる。
    - 費用は 1 秒に満たない。
    - 道具を別のプロセスで呼ぶので、終了コードは最後にもう一度 `Test-I686Pe` を見た 1 か所で決まる。
  - 取らなかった案 ⑵（一回きりの確かめ）: ファイルは増えないが、終了コードの立て忘れや一時ファイルの残りを見張れない。
  - 影響: ファイル 1 本と段 1 つが増える。触るファイルの約束（`tools/test-all.ps1` だけ）からは、さらに外れる。
