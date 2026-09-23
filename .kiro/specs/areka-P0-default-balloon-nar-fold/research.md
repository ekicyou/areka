# ギャップ分析（research.md）

> 2026-09-23・本ブランチ（main `92f5f448` 相当）で実測。コードは「何の定義か」（関数名・定数名＋ファイルパス）で指し、行番号では指さない。分析であって決定ではない——分かれ目は末尾「設計判断の項目」に番号で並べ、要件ディスカッションへ渡す。

## 1. 要約

- **新しい仕組みは 1 つも要らない。** 畳む道具（`fold-samples`）・登記表（`SAMPLES`）・窓口（`SampleRoot`）・プロセス寿命で根を保持する雛形（`areka-emo-compose/src/sample_test_support.rs` の `static EMO2: LazyLock<SampleRoot>`）が全部揃っている。やることは「既存の手順に従う」だけで、規模 **S・リスク Low**。
- **要件書の事実に 1 点の誤りがある。** 「登記した瞬間から見張りが `STAYSEE_BALLOON_DIR` の綴りを赤にする」は成り立たない。見張りの走査語は末尾に `/` を持つ形（`vendors/sample_ghost/<名>/`）で、定数の値 `"../../vendors/sample_ghost/StayseeBalloon"` は `/` で終わらないため当たらない（§3.1）。付け替え漏れを実際に検出するのは、フォルダが消えた後の結合テストの赤（要件 4.4）である。
- **要件書が挙げていない追随箇所が 5 か所ある**（すべて文言の数や綴りの陳腐化）——`lib_tests.rs` の doc コメントの「5 つ」3 か所・`README.md` の「ここで畳んだ 4 本」・`structure.md` の「バルーン 2 本」・`fold-samples.rs` の doc に書かれた呼び方の例・`staysee_balloon_fixture_test.rs` の module doc（§3.2〜3.6）。
- **展開フォルダを読むコードは `areka-emo-text` の結合テスト一式だけ。** ワークスペース全域（`.rs`・`.toml`・`.json`・`.ps1`・`.sh`・`.yml`・`.txt`・`.bat`）を検索し、`crates/` の外に `vendors/sample_ghost/StayseeBalloon` を読む実行コードは 0 件。`tools/` は `perf` のみ、`.claude/launch.json` は無い（§2.4）。
- **設計フェーズへの持ち越し調査は無い。** 残るのは「どこまで直すか」の線引きだけ（§6）。

## 2. 現状の資産（要件 → 既存資産）

| 要件 | 既存資産（何の定義か） | ギャップ |
|---|---|---|
| 1.1〜1.3 `.nar` 化と往復一致 | `crates/sample-ghost-kit/examples/fold-samples.rs` の `main`（`--from` 必須）・`fold_one`（`git ls-files` の追跡ファイルだけを写して畳み、本番の展開器で空の根へ戻して 1 バイト単位で突き合わせる）。書庫の書き手 `sample_ghost_kit::fold_tree`（名前順・日時 1980-01-01 固定＝同じ入力から同じバイト列） | 無し。要件段階の実走で `追跡 29 / 展開 29 / balloon/StayseeBalloon=29 / .nar 73021 バイト / 不一致 0 件` を確認済み |
| 1.3 `install.txt` も置かれる | `crates/areka-nar/src/plan.rs` の `body_placement`（最上位 `install.txt` を除くのは `InstallKind::Supplement` のときだけ）。`vendors/sample_ghost/StayseeBalloon/install.txt` は `type,balloon`／`directory,StayseeBalloon` | 無し。29 ファイルの期待値 `EXPECTED_FILE_NAMES` はそのまま |
| 1.4 sha256 が provenance と一致 | `.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/provenance.md` §3.1 の 29 本の表 | **一度きりの照合**（§3.7）。常設の判定は「登記の往復」テストと `assets.rs` の名前集合であり、ハッシュは常設されていない |
| 1.2・1.5 追跡から外す・単一コミット | `vendors/sample_ghost/README.md`「検体を 1 本足す手順」手順 4（`git rm -r --cached` の後に実体を消す）。`.gitattributes` の `* -text` は追加前から効く（`git check-attr` が `text: unset`） | 無し |
| 2.1〜2.5 登記と窓口 | `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES`（5 行）・`SampleRoot::acquire`・`check_registry`（展開結果の要素 `balloon/StayseeBalloon` と登記の行を照合）・`known_sample_names`（一覧は `SAMPLES` から毎回導く＝要件 2.5 は 1 行足せば自動で 6 つになる）・`manual_paths`（`nar-sample-path` が印字） | 無し。`Sample { name: "StayseeBalloon", kind: SampleKind::Balloon, balloons: &[] }` を 1 行 |
| 3.1〜3.4 直書きの数 | `crates/sample-ghost-kit/src/lib_tests.rs` の `every_registered_sample_lands_where_its_registry_row_says`（`SAMPLES.len()` を `5` と突き合わせ）・`every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`（同）・`unknown_sample_fails_with_all_five_known_names`（名前の一覧 2 か所＋関数名） | 無し。ただし要件が挙げていない doc コメントの「5 つ」が 3 か所（§3.2） |
| 4.1〜4.3 結合テストの付け替え | 付け替え先の雛形 `crates/areka-emo-compose/src/sample_test_support.rs`（`static EMO2: LazyLock<SampleRoot>` ＋ `emo2_root()` が `folder().to_path_buf()` を返す）。付け替え元 `crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs` の `staysee_root()`（`PathBuf` を返す・呼び手 5 ファイルは全て `staysee_root()` 経由で、定数 `STAYSEE_BALLOON_DIR` を直接読むのは `test_support.rs` だけ）。dev 依存 `sample-ghost-kit` は `crates/areka-emo-text/Cargo.toml` に既にある | 無し。`staysee_root()` の戻り型を変えなければ呼び手 5 ファイル（`assets`・`definition`・`faces`・`bake`・`region` 等）は **1 行も触らない** |
| 4.4 結合テスト緑 | `assets.rs` の `stored_folder_holds_exactly_the_upstream_29_files` は `read_dir` で直下を列挙しサブフォルダ 0・29 本ちょうどを判定 | 無し。展開器は `<根>/balloon/StayseeBalloon/` の直下に 29 本だけを置く（要件段階の実走で確認） |
| 4.5 見張り緑 | `crates/log-capture-kit/tests/sample_path_guard_test.rs` の `Form::tokens`（`CheckedInTree` は `format!("{VENDOR_PARENT}/{}/", s.name)`）・`workspace_scan/mod.rs` の `scan_tokens`（コメント除去後に `starts_with` の素の部分一致） | **判定力が要件書の記述より弱い**（§3.1） |
| 5.1〜5.5 文書 | `vendors/sample_ghost/README.md` の先頭の表・`.kiro/steering/product.md` 既定バルーンの節・`structure.md`「Sample Ghost Kit Crate」の「検体の顔ぶれ」・`alpha-release-signoff/brief.md`「zip に入れるもの」・`baseware-root-layout/brief.md` の保管先の行・`roadmap.md` #42 | 無し。ただし要件が挙げていない数の陳腐化が 2 か所（§3.3・3.4） |

### 2.4 展開フォルダの他の読み手（全域検索・2026-09-23）

- `crates/**/*.rs` で `StayseeBalloon` を含むのは `areka-emo-text/tests/staysee_balloon_fixture*`（7 ファイル）と `sample-ghost-kit/examples/fold-samples.rs`（doc コメントの呼び方の例 2 行）だけ。フォルダの綴りを**実行行**に持つのは `staysee_balloon_fixture_test.rs` の `STAYSEE_BALLOON_DIR` の 1 か所。
- `.toml`・`.json`・`.ps1`・`.sh`・`.yml`・`.yaml`・`.txt`・`.bat`・`.cmd`（`target/`・`vendors/`・`.git/` を除く）に `StayseeBalloon` は 0 件。`tools/` の中身は `perf` だけ。`.claude/launch.json` は存在しない。
- `.md` は要件書の列挙どおり（steering 3 本＋brief 2 本＋経緯の記録）。`tech.md` は `.nar` 前提で書かれており直す所は無い。

## 3. 要件書と食い違う／要件書が触れていない事実

### 3.1 見張りは `STAYSEE_BALLOON_DIR` を赤にしない（要件書 Introduction の記述が誤り）

- `sample_path_guard_test.rs` の `Form::tokens` は、展開形の検体フォルダの走査語を `format!("{VENDOR_PARENT}/{}/", s.name)` で組む——**末尾に `/` が付く**。陽性の較正 `the_checked_in_sample_tree_is_detected_but_the_nar_file_name_is_not` も `{token}dic`（`/` の後ろに続きがある形）で確かめている。
- `workspace_scan/mod.rs` の `scan_tokens` は `line[at..].starts_with(token)` の素の部分一致で、語形の正規化はしない。
- `STAYSEE_BALLOON_DIR` の値は `"../../vendors/sample_ghost/StayseeBalloon"`——`/` で終わらず直後は閉じ引用符。よって登記しても**当たらない**。
- 影響: 要件 4.5 の「付け替え漏れの判定」は、この定数の形（`join` で継ぎ足す前の裸のフォルダ名）には効かない。付け替え漏れを実際に赤にするのは、フォルダが消えた後に `staysee_root()` の先が実在しなくなって落ちる結合テスト（要件 4.4）である。要件 4.5 自体は「0 件で緑」として成立するが、根拠の文を直すべき。
- 選択肢（§6 の項目 1）: ⒜ 要件書の文だけ直し、見張りは触らない（要件書「本仕様はこれを書き換えず」に沿う）／⒝ 見張りの語形に `{VENDOR_PARENT}/{name}"`（閉じ引用符で終わる形）を足す——`.nar` の綴り `vendors/sample_ghost/<名>.nar"` は名前の直後が `.` なので陰性の較正は壊れない。ただし `sample_path_guard_test.rs` は本仕様の境界の外にある。

### 3.2 `lib_tests.rs` の doc コメントの「5」（要件 3.3 の漏れ）

`unknown_sample_fails_with_all_five_known_names` の関数名は要件 3.3 が挙げるが、同ファイルの module doc（「未登録名は既知の 5 つを含む失敗になる」）・`every_registered_sample_lands_where_its_registry_row_says` の doc（「登記した 5 つの検体は」）・`unknown_sample_fails_with_all_five_known_names` の doc（「既知の名前 5 つを含む」）の 3 か所も 5 のまま残る。コメントなので判定には映らない＝黙って古びる。

### 3.3 `vendors/sample_ghost/README.md` の「ここで畳んだ 4 本」（要件 5.1 の漏れ）

「守る 5 点」⑷ の「ここで畳んだ 4 本（`konnoyayame.nar` 以外）は全エントリが無圧縮」は、`StayseeBalloon.nar` を `fold_tree` で畳むと 5 本になる。要件 5.1 は先頭の表の 1 行だけを求めている。

### 3.4 `.kiro/steering/structure.md` の「バルーン 2 本」（要件 5.2 の漏れ）

「検体の顔ぶれ」の項は「ゴースト 3 体＋バルーン 2 本」と数え、その後ろで既定バルーンを「展開フォルダのまま」と語る。要件 5.2 は後半の記述の削除を求めるが、前半の「2 本」も 3 本に変わる。

### 3.5 `fold-samples.rs` の doc に書かれた呼び方の例

`crates/sample-ghost-kit/examples/fold-samples.rs` の module doc は呼び方の例として `--from vendors/sample_ghost/StayseeBalloon` を 2 行載せている。畳んだ後はこのフォルダが無い。見張りはコメントを見ないので赤にはならないが、次に検体を足す人が読む例が「存在しないフォルダ」を指す。要件は `fold-samples` を「使うだけ・変えない」としている——doc コメントの例を `<フォルダ名>` の置き換えにするのは振る舞いの変更ではないが、境界の解釈が要る（§6 の項目 4）。

### 3.6 `staysee_balloon_fixture_test.rs` の module doc と要件 4.1 の「コード上に 0 か所」

入口ファイルの module doc は「`vendors/sample_ghost/StayseeBalloon/` は areka の既定バルーン…として保管した」「付け替えるのは `STAYSEE_BALLOON_DIR` の 1 行だけ」「`nar-install` が共有ヘルパを導入したら」と、定数が消えた後は成り立たない説明を 3 段落持つ。要件 4.1「綴りをコード上に 0 か所」がコメントを含むかは読み方が分かれる（見張りはコメントを除く）。含めないとしても、消えた定数を説明する doc は直すのが自然。

### 3.7 要件 1.4（sha256 の一致）は一度きりの照合で、常設されない

- 畳む時点では `fold-samples` が追跡ファイルと 1 バイト単位で突き合わせ、その追跡ファイルは provenance の 29 本のハッシュと一致済み（完了 spec の検証記録）。よって要件 1.4 は**推移的に**成り立つが、要件の文言は「`.nar` の中の各ファイル」の直接照合を求めているので、展開結果に `sha256sum` を当てて表と突き合わせた記録を 1 度残すのが素直。
- フォルダを消した後は `--check` も使えない（展開形が要る）。以後の常設の守りは ⒜ `every_sample_nar_installs_exactly_the_elements_its_registry_row_declares`（要素の一致）と ⒝ `assets.rs` の名前集合＋`EXPECTED_FRAME_SIZES` の IHDR 突合、⒞ `.nar` の刻印（長さ＋CRC-32＝`devroot.rs` の `stamp`）である。**中身のハッシュを常設で判定するテストは無い**。足すかどうかは §6 の項目 5。

### 3.8 `registry_records_kind_and_bundled_balloons` は `StayseeBalloon` を見ない

`lib_tests.rs` のこのテストは `emo2`・`R_POST_and_KOMAINU`・バルーン 2 本の種別と同梱を直書きで確かめる（`konnoyayame` も見ていない）。要件 2.1「種別はバルーン・同梱 0 本」の直接の判定はここに 1 行足すのが最短。足さなくても「登記の往復」テストが `.nar` の `type` と登記の種別の一致を判定するので、種別違いは赤になる。

### 3.9 完了 spec `areka-P0-default-balloon-bundle` の要件 3.2 と設計 C2 が陳腐化する

同 spec の要件 3.2「検体パスは定数 1 か所」と設計 C2（`STAYSEE_BALLOON_DIR` を `CARGO_MANIFEST_DIR` 基点で組む）は、本仕様の後は「窓口から名前で引く」に置き換わる。完了アーカイブは書き換えない運用なので、本仕様の design の境界節に「上書きする」旨を 1 行書けば足りる（開発規律「裁定で要件を改訂したら design・境界節まで追随」）。

## 4. 実装アプローチ

### 案 A: 既存の雛形をそのまま写す（推奨）

`test_support.rs` の `staysee_root()` の**中身だけ**を差し替え、入口の定数を消す。呼び手 5 ファイルは `staysee_root()` の戻り型（`PathBuf`）が変わらないので無変更。

```rust
// crates/areka-emo-text/tests/staysee_balloon_fixture/test_support.rs
use std::sync::LazyLock;
use sample_ghost_kit::SampleRoot;

/// 既定バルーンの検体。プロセス寿命で保持する（`SampleRoot` は破棄で複製の木を消すため）。
static STAYSEE: LazyLock<SampleRoot> = LazyLock::new(|| {
    SampleRoot::acquire("StayseeBalloon").expect("StayseeBalloon は登記済みの検体")
});

/// 既定バルーンのフォルダ `<根>/balloon/StayseeBalloon/`。
pub(crate) fn staysee_root() -> PathBuf {
    STAYSEE.folder().to_path_buf()
}
```

- `static` は破棄されないので要件 4.3（プロセスの間ずっと同じ位置・途中で消えない）は型で成り立つ。テストバイナリ 1 本につき複製 1 つ（`areka-emo-compose` と同じ）。プロセス終了で札（`.lock`）は OS が閉じ、残った木は次の取得時の `sweep` が回収する——これは `nar-install` の設計どおりで、`areka-emo-compose` の 2 検体が同じ形で既に動いている。
- 入口 `staysee_balloon_fixture_test.rs` からは定数 `STAYSEE_BALLOON_DIR` と、それを説明する module doc の 3 段落を消す（§3.6）。
- 変更ファイル: `vendors/sample_ghost/`（`.nar` 追加・29 本削除・README）、`sample-ghost-kit/src/{lib.rs,lib_tests.rs}`、`areka-emo-text/tests/staysee_balloon_fixture_test.rs`、同 `test_support.rs`、steering 2 本、brief 2 本、roadmap。brief の「同じウェーブの他 spec と共有 0」は今日も成り立つ（roadmap の A1 干渉台帳 ⑥ と一致）。
- トレードオフ: ✅ 最小・既存パターン・呼び手無変更／❌ 無し（この規模で新しい抽象は要らない）。

### 案 B: 検体の受け口を共有 crate へ寄せる

`sample_test_support.rs` 型の `static LazyLock<SampleRoot>` を crate 横断の共有ヘルパにする案。**採らない**——今日の利用者は `areka-emo-compose`（2 検体）と `areka-emo-text`（1 検体）だけで、各 crate 内 5 行の重複を消すために crate を足す利得が無い。`sample-ghost-kit` の doc は意図的に「借用しか返さない」設計で、保持の仕方は消費側が決めるものとしている。

### 案 C: 案 A ＋ 見張りの語形を 1 つ足す

§3.1 の穴（`/` で終わらない裸のフォルダ名）を `sample_path_guard_test.rs` の `Form::tokens` に `format!("{VENDOR_PARENT}/{}\"", s.name)` を足して塞ぐ。陽性・陰性の較正も 1 対ずつ増やす。トレードオフ: ✅ 「登記した検体の展開フォルダを綴った定数」を将来も機械で拾える／❌ 本仕様の境界（「見張りは書き換えない」）の外・`log-capture-kit` に触る＝A1 の干渉台帳で ④ `keycolor-clickthrough-coverage` が「本文走査のテスト」を触る可能性があり共有 0 が崩れうる。判断は §6 の項目 1。

## 5. 手順の順序（1 コミットに畳む前提の作業順）

1. `cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon` → 印字が `不一致 0 件`／`全ての検体で一致`・終了コード 0。
2. `.nar` を展開した 29 本に `sha256sum` を当て、provenance §3.1 の表と突き合わせた記録を残す（要件 1.4。`nar-sample-path -- StayseeBalloon` の `folder=` を使えば窓口経由で取れる）。
3. `SAMPLES` に 1 行・`lib_tests.rs` の数 2 か所・名前の一覧 2 か所・関数名・doc の「5」3 か所（§3.2）。`registry_records_kind_and_bundled_balloons` に足すかは項目 3。
4. `test_support.rs` を案 A の形へ・入口の定数と doc を削る。
5. `git rm -r --cached vendors/sample_ghost/StayseeBalloon` → 実体を消す（`.gitignore` は当たらないので、消さなければ未追跡として `git status` に残る）。
6. `cargo test -p sample-ghost-kit`／`cargo test -p areka-emo-text --test staysee_balloon_fixture_test`／`cargo test -p log-capture-kit --test sample_path_guard_test`。
7. 文書: README の表＋「畳んだ 4 本」→5 本、`product.md`・`structure.md`（「2 本」→3 本を含む）、brief 2 本、`fold-samples.rs` の doc の例（項目 4）、roadmap #42。
8. 要件 5.5 の検索を、完了 spec と経緯の記録を除いて実行し 0 件を確かめる。

## 6. 設計判断の項目（要件ディスカッションへ）

1. **見張りの穴をどう扱うか**（§3.1）: 要件書の「登記した瞬間から赤にする」の文を「見張りは末尾 `/` 付きの形しか拾わない・付け替え漏れは結合テストが赤にする」へ直すだけか（案 A）、`Form::tokens` に閉じ引用符で終わる形を足して塞ぐか（案 C）。後者は境界の外のファイルに触る。
2. **要件 4.1「コード上に 0 か所」の範囲**（§3.6）: 実行行だけか、doc コメントも含めるか。含めないとしても入口ファイルの module doc 3 段落は「消えた定数の説明」になるので直す前提で書く。
3. **`registry_records_kind_and_bundled_balloons` に `StayseeBalloon` を足すか**（§3.8）: 要件 2.1 の直接の判定を 1 行で得られる。足さない場合は「登記の往復」テストを 2.1 の根拠とする。
4. **`fold-samples.rs` の doc の例**（§3.5）: `<フォルダ名>` の置き換えに直す（コメントのみ・振る舞い 0 変更）か、「使うだけ・変えない」を字義どおり守って残すか。
5. **29 本の sha256 を常設の判定にするか**（§3.7）: 今は provenance の表と一度きりの照合。`assets.rs` に sha256 の表を足せば「中身の差し替え」を常設で拾えるが、`areka-emo-text` に sha256 の実装（依存か自前）が要る。要件 1.4 は一度きりの照合で満たせるので、常設化は本仕様の外へ置く選択もある。
6. **README に出どころの節を足すか**: `konnoyayame.nar` には「出どころとライセンス」の節がある。`StayseeBalloon.nar` の出どころ・ハッシュ・CC0 は完了 spec の provenance に在るので、表の 1 行にそこへのポインタを書けば足りるか、同じ形の節を設けるか。
7. **要件書の漏れ 3 点の扱い**（§3.2〜3.4）: `lib_tests.rs` の doc の「5」・README の「4 本」・`structure.md` の「2 本」を要件 3.3／5.1／5.2 に書き足すか、タスクの注記で済ませるか。

## 7. 規模とリスク

- **規模: S**（要件書どおり XS〜S。触るファイルは 10 前後で、うち Rust は 4 ファイル・新規 0）。
- **リスク: Low**——全て既存の手順と雛形の写し。唯一の注意点は「畳む・登記・付け替え・追跡から外す」を同じコミットに畳むこと（途中状態はどれも赤）。
- **Research Needed: 無し。** 外部依存の追加も無い。

## 8. 要件ディスカッションでの扱い（2026-09-23）

§6 の各項目の行き先。

- **要件へ反映済み（自明な修正）**: 項目 1（見張りの穴＝要件書の記述を訂正し、見張りは触らない。本仕様の後は読み先のフォルダが消えるので、裸のフォルダ名が残っても結合テストが赤にする＝実害が無い）・項目 2（要件 4.1 はコメントを含めて 0 か所）・項目 4（`fold-samples.rs` の doc の例は `<展開フォルダ>` へ＝要件 5.5）・項目 7（`lib_tests.rs` の「5」＝要件 3.4、README の「4 本」＝要件 5.1、`structure.md` の「2 本」＝要件 5.2）・§3.9（完了 spec `default-balloon-bundle` 要件 3.2／設計 C2 の上書きを Adjacent expectations に明記）。要件 1.4 は「畳んだ直後の照合＋記録」と読める形に直した。
- **設計フェーズへ持ち越し**: 項目 3（`registry_records_kind_and_bundled_balloons` に 1 行足すか）・項目 6（README の出どころを節にするかポインタにするか）。どちらも how で、要件の判定は他の受入基準で既に立つ。
- **開発者と議論**: 項目 5（中身が無改変であることを常設の検査で固定するか）。→ **固定しない**（2026-09-23 裁定）。目的は保管形の切り替えで、中身の保証は今日と同じ強さのまま据え置く。

## 9. 設計フェーズの記録（2026-09-23・`/kiro-spec-design -y`）

- **Discovery Scope**: Simple Addition（既存の手順と雛形の写し）。正式なディスカバリと外部調査は行わず、主要ファイルを読み直す軽い確認だけ。サブエージェントの派遣は 0。
- **読み直しで確かめた事実（設計に効いたもの）**: `SAMPLES` は 5 行（`lib.rs` の定義）／`known_sample_names()`・`manual_paths` はどちらも `SAMPLES` から導く＝登記 1 行で 2.4・2.5 が成り立つ／`staysee_root()` を直接呼ぶのは `assets.rs`・`bake.rs`・`faces.rs` と `test_support.rs` 内の 2 関数で、他のテーマは `staysee_model()`・`resolve_staysee()` 経由＝戻り型 `PathBuf` を保てばテーマ別 8 ファイルは 0 行変更／`static EMO2: LazyLock<SampleRoot>`（`areka-emo-compose/src/sample_test_support.rs`）が同じ形で既にコンパイルされている／`install.txt` は `type,balloon`・`directory,StayseeBalloon`／`LICENSE` は CC0 1.0 の法典本文／`sha256sum` は Git Bash に在る。
- **Synthesis**: 一般化＝無し（要件は全て「1 検体を既存の形へ揃える」の 1 問題）。Build vs Adopt＝全て既存の採用（`fold-samples`・`SampleRoot`・`LazyLock` 雛形・完了 spec の provenance 表）で、新規の実装 0。単純化＝案 B（共有ヘルパ）と案 C（見張りの語形追加）を却下、新しいテスト関数 0・新規ファイルは照合記録の 1 本だけ。
- **設計判断（持ち越し 2 件の決着）**:
  - 決定 A（項目 3）: `registry_records_kind_and_bundled_balloons` のバルーンのループに `"StayseeBalloon"` を 1 語足す。2.1「種別はバルーン・同梱 0 本」の直接の判定をこの 1 語で得られ、既存の assert 文言 `"{name} は同梱バルーン 0"` が 0 を明示的に書く。新しい assert は 0。`konnoyayame` が同テストに無い点は本仕様の外。
  - 決定 B（項目 6）: README は節を設けず、表の 1 行＋出どころの箇条書き 1 行（CC0・上流 URL・畳み直し可・配布物へ同梱可・ハッシュと突合は完了 spec の `verification/provenance.md` §1・§3 へのポインタ）。`konnoyayame` の節は禁止事項（畳み直さない・配布物へ入れない）を伝えるためのもので、`StayseeBalloon` には禁止が 0。sha256 の表を README にも写すと 2 か所目が黙って古びる。`tech.md`「出どころとライセンスを README に登記」は 1 行で満たす。
- **レビューゲート**: 要件 ID 27 個すべてが design.md に現れる／境界 4 節・File Structure Plan は実パスで埋まっている／コンポーネント 6 つ全てにファイルの対応がある／符牒・placeholder 0。修正パス 0 回で通過。要件の隙間や矛盾は見つからなかった。
- **リスク（設計後も残るもの）**: ⑴ 作業順を守らないと途中状態が赤（畳むのはフォルダが在るうち・追跡から外すのは判定の前）→ design.md「作業順」に固定。⑵ `.nar` を作り直したときに中身が変わっても常設の判定は名前と縦横だけ（裁定どおり・PR の差分が唯一の検出）。⑶ `shell-balloon-switch` が検体を足すと逐語の `6` の追随が再発（Revalidation Triggers に明記）。

## 10. 設計ディスカッションでの扱い（2026-09-23）

設計レビュー（`design-validation.md`・判定 GO）の重大な問題 1 件と軽微な指摘 3 件を精査した。**開発者に聞かねば作業が変わる議題は 0 件**（いずれも既存の慣例と要件の文言で決まる）。自明な修正 5 件を design.md に反映した。

- 完了判定の検索を 2 本に分け、期待ヒットを明記（⒜ `crates/` の `/` 無しの綴り 0 件＝要件 4.1・5.5／⒝ 文書の `/` 付きの綴りは `roadmap.md` #37・#42 と `roadmap-history.md` の 4 件以外 0 件）。着手前の実測: ⒜ 4 件・⒝ 10 件。
- roadmap #42 の行は、完了した行の慣例（本文を残して状態列・段列を ✅ に・`/kiro-complete` が行う。#37 の行が実例）に従い、本文は書き換えない。
- `baseware-root-layout/brief.md` 冒頭の追記（「畳む仕事は #42 が持つ…偽になった」）を要件 5.3 の追随に含める（`/` 付きの綴りを持たないので検索には掛からない＝手で直す）。
- 照合記録の Output に `nar-sample-path` の印字 2 行と終了コードを逐語で残す（要件 2.4 の直接の証跡）。
- 入口ファイルの module doc で消える定数を語る 2 文を Modified Files に名指し。
