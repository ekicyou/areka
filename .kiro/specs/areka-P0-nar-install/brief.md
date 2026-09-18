# Brief: areka-P0-nar-install

> 2026-09-12 `/kiro-discovery` 再入で起票。開発者の指示「NAR を解釈したい・NAR インストールしたい・NAR インストールによる試験環境構築スキームが欲しい——これは一度に対応した方がよいので単一 spec」。
> 本文の file:line は**起票時の実測値**。着手時に必ず引き直すこと（roadmap「着手手順」）。

## Problem

**誰の何が困っているか**: areka の開発者（および実機サインオフを回す全員）。

検体ゴースト emo2 が **展開済みのまま** `crates/pilot/examples/shiori-host-32/fixtures/emo2/` に置かれている。ここから 3 つの痛みが出ている。

1. **検体が走行で汚れ、次の走行の挙動が変わる。** `areka-ghost` の `boot_ghost` が `profile_areka_root(&mount.shiori.dir)` を作り、`areka-sylphya` の `FsPersistIo::commit` が `sylphya.toml`（窓位置＋`[boot] count`）を書く。**起動記録があると SHIORI のイベント列が変わる**（初回は `OnFirstBoot`、2 回目以降は飛ばして `OnBoot`）。つまり同じ検体で 2 回走らせると別の結果になる。
2. **その場しのぎの初期化が 1 か所だけ存在する。** `areka/src/emo2_boot/spine.rs` の `emo2_root()` は、呼ばれるたびに `<root>/ghost/master/profile/areka` を `remove_dir_all` している。汚染を 1 か所だけ手で拭っている状態で、他の 37 か所は拭っていない。
3. **無視されていない書き込み先が残っている。** 同じ `ScopeRoots` が `shell: Some(profile_areka_root(&mount.shell.dir))` も設定するが、`crates/pilot/examples/shiori-host-32/.gitignore` が無視するのは `fixtures/emo2/ghost/master/profile/` **だけ**。シェルスコープの commit が初めて走った日に、追跡外ファイルが湧く。

加えて、置き場そのものが分かりにくい。**本番アプリが起動する唯一のゴーストが、別クレートの example のテスト検体の中に 5 階層下がって間借りしている。**

## Current State

**保管形**: `crates/pilot/examples/shiori-host-32/fixtures/` 配下。

| 中身 | 種別 | 大きさ |
|---|---|---|
| `emo2/` | `type,ghost`（バルーン同梱・追跡 150 ファイル） | 6.6 MB（ghost 3.9M／shell 2.7M／同梱バルーン 71K） |
| `emo2-kakukaku-offsetdpi/` | `type,balloon`（areka 製の検証用派生） | 71 KB |
| `emo2-kakukaku-wplimit/` | `type,balloon`（同上） | 67 KB |

`emo2/install.txt` は `type,ghost` / `directory,emo2` / `balloon.directory,emo2-kakukaku` / `balloon.source.directory,emo2-kakukaku` を宣言しており、**ukadoc「インストール」が定める「ゴーストにバルーンを同梱する場合」の配布アーカイブ（nar のペイロード）そのものの形**。内部構造に誤りは無い。

**参照のされ方**（実測）:

- 参照は **38 か所**。うち **26 が「同じ本体をコピペした `emo2_root()`」**——全部 `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pilot/examples/shiori-host-32/fixtures/emo2")` の写し。綴りの揺れ 2 種（一括 `join` と分割 `join`）。`const FIXTURE_DIR` が 1 件、ヘルパ無しの直書きが 11 件。**共有ヘルパは 1 つも存在しない。**
- 消費者の内訳: `src/` 内の単体テスト 16 ファイル／`tests/` 10／`examples/` 9／**本番コード 0**。
- 本番の既定は無関係で、しかも実在しない: `areka/src/boot_config.rs` の `default_ghost_root()` は `CARGO_MANIFEST_DIR` 相対のプレースホルダ `ghost/master`（実在しない）。実ゴーストは `argv[1]`（`resolve_config_inputs`）から来る。

**NAR 側の現状**: `install.txt` の各キー（`type` / `directory` / `balloon.directory` / `balloon.source.directory` / `accept`）を読む実装は**皆無**。言及は `areka-parsers/src/package/resolve.rs` のモジュール doc（自身を対象外と明記）と `package/validation_tests.rs`（「nar 配置の指示書・起動時には使わない」と注記）の 3 か所のみ。zip 展開器も未導入（`flate2` は `png` 経由の間接のみ）。

**準備機構の前例**: `crates/*` に `build.rs` は **0 本**（`structure.md` が明記する性質）。xtask も無し。唯一の隣接前例は `vendors/pasta` の `pasta_sample_ghost` で、**build script を意図的に拒否**して明示コマンド＋`.nar` のコミットを選んでいる。

## Desired Outcome

完了時に次が真になっている。

1. リポジトリが保管するのは **`vendors/sample_ghost/*.nar`** だけ。展開済みツリーは追跡されない。
2. **`areka-nar`（本番クレート）が NAR を読み、`install.txt` の意味論に従って展開する。** 配布形 → インストール済み形の変換をこのクレートが持つ。
3. テストと実機走行が使う実験環境は、**`target/` 配下に展開された使い捨ての木**。`cargo clean` で消え、次回は自己修復する。
4. **走行ごとに新品**なので、起動記録の汚染が構造的に起こらない。`spine.rs` のその場しのぎの `remove_dir_all` は削除できる。シェルスコープの追跡外書き込みも消える。
5. 38 か所の参照が **1 つの共有ヘルパ**に収束し、保管形式の再変更が 1 行で済む形になっている。

## Approach

**選んだ形**: 本番クレート `areka-nar` ＋ `target/` への遅延展開。

### 3 段構え（各段が独立に緑）

| 段 | 中身 | 検証 |
|---|---|---|
| ① 収束 | 38 か所を 1 つの共有ヘルパへ寄せる。**保管形式は変えない**（展開済みツリーを指したまま） | 挙動不変・全テスト緑。`spine.rs` の `remove_dir_all` はこの段では保持 |
| ② エンジン | `areka-nar` を新設。zip コンテナ読取＋`install.txt` 解釈＋安全な展開 | 決定論テスト（固定 `.nar` を展開して木を検証）。実機不要 |
| ③ 切替 | 共有ヘルパを「無ければ `.nar` から展開」へ。`emo2.nar` を作り展開済みツリーを削除 | 全テスト緑・実機一周 |

段 ① を先に置くのが肝。**38 ファイルへの機械的な書き換えという最も事故りやすい作業を、挙動を変えない状態で済ませる。**

### なぜ遅延展開（明示コマンドではなく）

`vendors/pasta` の前例は明示コマンドだが、**採らない**。「人が覚えておかねばならない事前手順」は忘れられる。本仕様 `areka-P0-charset-canon` のタスク 6.1 で、既存の赤 1 件が `cargo test --workspace` を打ち切るために別クレートの番人が **8 本のタスクレビューを素通りした**実例が出たばかりで、同じ種類の穴を自分で掘ることになる。

競合は解ける。`areka-sylphya` の `FsPersistIo::commit` が「一時ファイルへ書く → `rename` で差し替える」を既に実装しており、同じ形で「一時ディレクトリへ展開 → `rename` で確定」にすれば原子的。陳腐化は `.nar` のハッシュを刻印ファイルに置いて検出する。

### 依存とその扱い

`zip = { version = "8.6", default-features = false, features = ["deflate-flate2"] }` を **`crates/areka-nar/Cargo.toml` に直接**書く（`dola`／`ukadoc-survey` の前例＝ルート manifest は触らない）。`features = ["deflate"]` は**使わない**（`deflate-zopfli` は書き込み側）。

- ライセンスは全て許可リスト内。新規に graph へ入るのは **`typed-path` 1 本だけ**（MIT OR Apache-2.0）。既存の重複警告 7 件は増えない。`cargo deny --offline check licenses bans sources` は起票時に実走して緑を確認済み。
- **`ZipArchive::extract()` を呼んではならない。** RUSTSEC-2025-0168（HIGH・任意ファイル書き込み・`>=1.3.0, <2.3.0`）が住んでいた API であり、かつ名前ベース＝CP437 経路。`by_index` で回し、名前を自分で復号・検証・結合し、**シンボリックリンクを決して作らない**ことで脆弱性の種類ごと回避する。
- **ファイル名の文字コードは自前で決める。** `zip` は一般目的ビット 11 が立っていれば UTF-8、立っていなければ **CP437** として読み、**Shift_JIS の経路を持たない**。日本語ゴーストのアーカイブはフラグ未設定＋CP932 で書かれることが多く、`ZipFile::name()` は文字化けする。`ZipFile::name_raw() -> &[u8]` から生バイトを取り、`encoding_rs`（既存の workspace 依存）で復号する。`enclosed_name()` も CP437 由来なので使わず、パス安全性の検査も自前で持つ。

## Scope

- **In**:
  - `areka-nar` クレート新設（本番・`[dependencies]`）。NAR コンテナ読取・`install.txt` 解釈・安全な展開
  - `install.txt` パーサ（`type` / `directory` / `balloon.directory` / `balloon.source.directory`／必要なら `accept`）
  - ファイル名の文字コード決定（ビット 11・Shift_JIS 既定・自前復号）
  - パス安全性（絶対パス・ドライブレター／UNC・`..`・NUL・`\` 区切りの拒否／シンボリックリンク非生成）
  - 展開先の管理（`target/` 配下・原子的確定・ハッシュによる陳腐化検出・自己修復）
  - 38 か所の参照を 1 つの共有ヘルパへ収束
  - `emo2.nar`・`R_POST_and_KOMAINU.nar`・2 つのバルーン単体の `.nar` 化、展開済みツリーの削除、`vendors/sample_ghost/` への集約
  - `spine.rs` のその場しのぎの `remove_dir_all` の削除、`.gitignore` の整理
  - `THIRD-PARTY-NOTICES.md` の再生成、`tech.md` への依存追加の登記

- **Out**:
  - **`.nar` の作成（書き込み側）**。本仕様は読み取りと展開のみ。配布物の生成は M2 予約群の NAR 機能が持つ
  - **利用者が投げた `.nar` を受け取る UI／D&D／インストーラ体験**。M2 予約群の範囲
  - `updates2.dau`／ネットワーク更新／`delete.txt` の解釈
  - **里々標準テンプレート `R_POST_and_KOMAINU` の入手**。2026-09-12 に `vendors/sample_ghost/R_POST_and_KOMAINU/` へ展開形で追跡済み（`satori_license.txt` が「ライセンス文書を同梱すれば再配布可・`.nar` に含めて構わない」と明示し、辞書は readme が著作権放棄を宣言）。本仕様の仕事は**その木を `.nar` へ畳むこと**であって、入手そのものではない
  - ベースウェア直下の `ghost/` `balloon/` 規約と、`areka.exe` の既定パスの再設計（別途）
  - 検体そのものの内容変更（emo2 の中身は 1 バイトも変えない）

## Boundary Candidates

- **コンテナ読取**（zip の中央ディレクトリ・エントリ列挙・名前の生バイト取り出し）
- **`install.txt` の意味論**（配布形 → インストール済み形の写像。`areka-parsers/src/package/` が自然な住処だが、**同クレートは「外部パーサ依存を入れない」方針**なので展開器は同居できない。パーサだけ parsers 側、コンテナ読取は `areka-nar` 側に割れる）
- **展開の実行**（パス安全性・原子的確定・陳腐化検出）
- **検体の解決**（テストと実機に「使える環境の根」を返す唯一の窓口。38 か所の収束先）

## Out of Boundary

- `.nar` の書き出し・配布物生成
- インストーラの利用者体験（D&D・確認ダイアログ・`terms.txt` 表示）
- `areka.exe` の既定ゴーストパスの再設計
- 検体の内容変更・派生バルーンの作り直し

## Upstream / Downstream

- **Upstream**: `areka-parsers`（`kv` と `charset` の既存層＝`install.txt` は行指向の `key,value`）／`encoding_rs`（workspace 既存）／`areka-sylphya` の `FsPersistIo::commit`（原子的確定の手本）。完了仕様 `areka-P0-test-cage-determinism`（`temp-path-kit`／`log-capture-kit` の dev-only 規律）。
- **Downstream**: **M2 予約群の NAR 機能**が本仕様のエンジンをそのまま昇格して使う（二度実装しない）。実機サインオフを持つ全 spec が、新品の実験環境を前提にできるようになる。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**:
  - `areka-P0-charset-canon`（W13・実装中）——ファイル名の文字コードという同種の問題を扱う。本仕様が `encoding_rs` で自前復号する方針は同仕様の「意味論は ukadoc から輸入」と同じ筋。共有ファイルは無い。
  - **roadmap の M2 予約「NAR」**——予約は**製品機能**、本仕様は**開発側の駆動で建てるエンジン**。仮裁定 2「M2 予約群は brief を起票しない」に反しない（予約そのものの起票ではなく、その下に敷く資産を先に建てる）。
  - 実機サインオフを持つ全 spec（`present-gpu-transform-scale` 等）——手順の「検体の絶対パス」が変わる。

## Constraints

- **編成上、単独枠。** コード 38 ファイル・9 クレートに触るため、W13〜W17 のほぼ全 spec と共有ファイルが発生する。**ウェーブの切れ目に単独で置く**しかない（`tick-gate-adoption` と同じ扱い）。並走不可。
- **外部依存の追加には開発者の承認が要る。** `tech.md` は `encoding_rs` を「意図的依存追加＝2026-07-02 承認済」と記録する慣行。`zip` も同じ登記が要る。
- `deny.toml` は許可制のホワイトリスト・git ソース全面禁止・`all-features = true` で評価。`about.toml` の `ignore-dev-dependencies = true` は**本仕様には効かない**（本番クレートのため）。
- `target/` に置くものは**使い捨てのキャッシュ**として扱う。何かがそこに在り続けることに依存しない。`target/debug/` は既知の地雷（x64 helper が落ちて実機走行を壊す前例）なので、明確に名前空間を切った別サブディレクトリを使う。ワークスペース走査 3 種（1,000 行番人・ログ番人・survey）はいずれも `target` を除外するので、展開物が拾われる心配は無い。
- 1 ファイル 1,000 行の番人。例外表には**触らない**。
- `log-capture-kit` を `[dependencies]` に置かない（機械の番人が赤にする）。
- 決定論テスト網羅は必達。展開の検証は固定 `.nar` の入出力で成立させ、実機を要さない形にする。
- **検体は 1 体ではなく 2 体。** `emo2`（pasta.dll）に加え `R_POST_and_KOMAINU`（satori.dll・全ファイル Shift_JIS）が `vendors/sample_ghost/` に入った。共有ヘルパは「検体名を受け取って根を返す」形にすること（emo2 専用の関数にしない）。
- **バイト保存の罠 2 件は `vendors/sample_ghost/` では対処済み・`emo2` 側は未対処。** ⑴ `core.autocrlf=true` ＋ `.gitattributes` 不在だと CRLF が LF で登録され、Windows 以外の取り出しでバイトが変わる（ハッシュによる陳腐化検出が壊れる）。⑵ リポジトリ直下の `.gitignore` の `*_test.txt` が、Windows の大小無視により `dic09_Test.txt` のような辞書を黙って落とす。現行の `fixtures/emo2/` は `git check-attr text` が `unspecified`＝**無防備**なので、移設時に同じ 2 つの手当てを持ち込むこと。

---

## 2026-09-18 追記（棚卸⑭＝α ゴールへの組み直し）

- **本仕様は α（M2）の A0（単独枠・先頭）に格上げ**（2026-09-18 開発者「nar 関係は早く進めないとダメ」で `shell-implicit-surface` との順序を反転）。後続の A1（`shell-implicit-surface`・`baseware-root-layout`・`default-balloon-bundle`）は全て本仕様の共有ヘルパ（検体名 → 根）を前提にする。**サンプルゴーストを増やして試験する仕組み**はこの共有ヘルパと `vendors/sample_ghost/*.nar` の保管慣行がそのまま器になる（検体を足す＝`.nar` 1 つと名前 1 行）。
- **展開先の形＝ベースウェアの根の形に揃える**: `<根>/ghost/<directory>/`・`<根>/balloon/<balloon.directory>/`（ukadoc「全体の構成」の格納フォルダ）。`target/` 配下に作る開発用の根も同じ形にし、`baseware-root-layout` がその根をそのまま `BasewareRoot` として受ける。共有ヘルパは「検体名を受け取って根を返す」に加え「根そのもの」を返せること。
- **Out に書いた「利用者が投げた `.nar` を受け取る UI／D&D／インストーラ体験」の引受先は `areka-P0-ghost-install`（09-18 起票・A5-①）**。同じく Out の `updates2.dau`／ネットワーク更新／`delete.txt` は `areka-P0-network-update`（09-18 起票・A5-②）。「ベースウェア直下の `ghost/` `balloon/` 規約と `areka.exe` の既定パス」は `areka-P0-baseware-root-layout`（09-18 起票・A2）。
- `areka-nar` が受理する `install.txt` の `type` は `ghost`・`shell`・`supplement`・`balloon` の 4 つ。他（`plugin`・`headline`・`language`・`calendar*`・`package`）は理由付きで拒否を返す（製品側が `OnInstallFailure` に写す）。
