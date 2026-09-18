# 設計検証レポート: areka-P0-nar-install

> 2026-09-18 実施（非対話・Fable）。対象は確定済みの `design.md`（846 行）・`requirements.md`（10 要件・95 受入基準）・`research.md` 節 9〜14・`brief.md`・steering。設計が依拠する外部クレートと OS の事実は、手元の cargo レジストリ（`zip-8.6.0`・`flate2-1.1.10`・`miniz_oxide-0.9.1`）・rustc 1.98.1 の std ソース・docs.rs・作業ツリーの実測で裏取りした。到達可能性を確かめられない指摘は載せていない。

## 1. レビュー概要

境界の切り方（本番の展開器 `areka-nar` と開発専用の窓口 `sample-ghost-kit` を別クレートに分け、依存の向きで本番から見えなくする）は `temp-path-kit`／`log-capture-kit` の既存の型そのもので、見張りの置き場・走査部品の再利用・`charset`／`kv` の再利用も既存構造に沿っている。95 受入基準は traceability 表で全件に部品と契約が対応し、拒否語彙 13 変種・段階 ①②③ の合否条件・巻き戻しまで実装に十分な粒度で書かれている。

設計が事実として置いている主張（`zip` の印の非公開・`deflate-flate2` 単体のコンパイル不能・`miniz_oxide` の feature 構成・Windows の `rename`・dev 依存の循環）は**全て裏取りで真**だった（節 4）。残る問題は 2 つで、どちらも構造の作り直しではなく文言の確定で解ける: ⑴ 開発用の根の「初回展開中の作業フォルダ」が掃除の対象から守られていない（多重プロセスのテストで実際に踏む）、⑵ 要件の「shall」から外れる 2 件（6.2・10.2）を要件側へ戻す判断が要る。

## 2. 重要な指摘（最大 3・議題送り）

### 🔴 指摘 1: 初回展開中の作業フォルダに札が無く、並走する別プロセスの掃除に消される

- **問題**: `devroot` の初回展開（cache miss）は「`work/<pid>-<連番>/` を空の根として `NarArchive::install` を呼び、`rename` で `cache/` へ入れる」とだけ書かれ、札ファイルを開く手順が無い（design.md「`devroot`」節の箇条書き 2 つ目）。一方、掃除の規則は「**札の無い木は消す**（札を作る前に落ちた残骸）」（同節の箇条書き 5 つ目）で、取得のたびに `work/` 全体を走査する（フローチャート `Sweep → Stamp → Stage`）。札を開く手順が明記されているのは複製（箇条書き 4 つ目「札ファイルを…開いてから…複写」）と `WorkDir`（「札付きで 1 つ配る型」）だけで、初回展開の作業フォルダがどちらの型で作られるかは書かれていない。
- **到達可能性: 確認済み（設計自身のテストで踏む）**。Testing Strategy「多重プロセス（9.5・7.6）: 空の `nar-samples/` から…自分自身のテストバイナリを 4 つ同時に起こし」がまさにこの条件で、プロセス A が emo2（110 ファイル・目安 1 秒未満）を書いている間にプロセス B の取得が `work/` を掃除する。A の作業フォルダに札が無ければ B は `remove_dir_all` する。A は書込途中で `NotFound` を受けて `Io` で失敗するか、B が削除しきる前に `rename` が通れば**欠けた木が `cache/<名>-<刻印>` の名前で入る**。後者は「名前が合う原本は完全」という不変条件（7.6・7.7 の根拠）を破り、以後の全取得が欠けた複製を配る。
- **影響**: 要件 7.6（全員が完全な木を得る）・7.7（半端な木を使える根として返さない）の検証テストが間欠的に赤になるか、最悪は黙って欠けた原本を固定する。
- **提案**（文言の追記で足りる）: ⑴ 初回展開の作業フォルダは**必ず `WorkDir`**（札を `create_new` で開いてからフォルダを作る）で作り、`rename` で `cache/` へ出した後の `Drop` は木が無いこと（`NotFound`）を許容する、と `devroot` 節に明記する。⑵ 掃除が孤児の木を消すときも `cache/` と同じ「先に `work/gc-<pid>-<連番>` へ `rename` してから `remove_dir_all`」にする（pid が再利用された新プロセスが同名 `work/<pid>-1/` を作る瞬間との競合も同時に消える）。⑶ 札の `remove_file` が `NotFound` で失敗した場合は「別プロセスが回収中」として触らず、「共有違反＝生きている」と区別する。
- **Traceability**: 7.5, 7.6, 7.7, 9.5
- **Evidence**: design.md「Components and Interfaces › sample-ghost-kit › `devroot`」State model と箇条書き、「System Flows › 開発用の根の取得」フローチャート、「Testing Strategy › Integration Tests › 多重プロセス」

### 🔴 指摘 2: 要件の「shall」から外れる 2 件は要件文の改訂が要る（他の 4 件は許容できる狭め）

設計が Open Questions に自ら挙げた逸脱 6 件を、要件の文言と照らして判定した。

| 逸脱 | 判定 | 根拠 |
|---|---|---|
| **6.2**: `type,supplement` では `refresh` を読まず常に重ね置き | **要件へ戻す** | 6.2 は「While 宛先フォルダが既に存在し `refresh,1` である, shall 全て消してから展開する」で種別を限定していない。ukadoc `descript_install#refresh` も「同じディレクトリのファイルを全て消去」とだけ言う。設計の理由（重ね置き先はゴースト本体で全消去は利用者のゴーストを壊す）は妥当だが、利用者から見える結果が変わる決定なので、開発者が採るなら 6.2 に「`type,supplement` を除く（設計 D9）」の 1 句を足す。採らないなら `Supplement` も `ExistingPolicy` を読む |
| **10.2**: 既定機能を切らず（`with-alloc` 既定のまま）、圧縮側のコードがバイナリに残る | **要件へ戻す（文言の修正）** | 手元の `miniz_oxide-0.9.1/Cargo.toml` は `default = ["with-alloc"]` で、`src/lib.rs` は `pub mod deflate` も `decompress_to_vec_with_limit` も同じ `with-alloc` で括る＝伸長だけを残す feature は**存在しない**。`zip`＋`flate2` 経路も同じ性質。したがって 10.2 の後半「書き込み側の圧縮機能を有効にしない」はどの候補でも字義どおりには満たせず、要件を「書き込み側の API を呼ばないことを検査で見張る」へ改める。前半「既定機能を切り」は `default-features = false, features = ["with-alloc"]` と書けば字義どおり満たせる（同じコードになるが、要らない既定が将来増えても入らない）ので、設計の `Cargo.toml` 記述をこの形に直すことを勧める |
| 4.1: 最初の拒否で全体を止める（全件検証しない） | 許容 | 4.1 の要点は「書き込みの前に止まり、部分展開を残さない」で、どちらの形でも成り立つ。9.2 は理由 1 件で足りる |
| 1.2: `root()` の提供は段 ③ から | 許容 | 1.2 は段を指定していない。段 ① の追跡済み展開形には「根」に相当するフォルダが無く、嘘の値を返すより無いほうがよい。1.6（段 ① は挙動不変）と整合。A1 の 3 仕様は段 ③ の後に着手 |
| 5.3: 取り出し元フォルダ内の `install.txt` は普通のファイルとして置く | 許容 | 5.3 の「無視し、拒否の理由にしない」は解釈しないことを言っており、置くことを禁じていない。8.4 の「20 ファイル一致」はこの形でなければ満たせない |
| 9.3: `.nar` 書き手を `sample-ghost-kit` に置く | 許容 | 9.3 は置き場を定めていない。10.3「テスト用の部品は `[dev-dependencies]` に限る」を満たす唯一の形（`#[cfg(test)]` は他クレートから届かない）。手元の実験でも `nar →dev kit → nar` の循環は cargo が受理し、`cargo test -p nar` は `nar` の lib を 1 回、`--test` を 1 回組む |

- **影響**: 要件は確定済みなので、設計だけが先に変わると要件と設計の突合（後続の実装検証・DoD）が食い違う。
- **提案**: 設計ディスカッションで 6.2 と 10.2 の 2 件だけ開発者に決めてもらい、決まった側の要件文を 1 句ずつ改める（`revise-design-not-just-requirements` の規律どおり design・境界節も同時に）。
- **Traceability**: 6.2, 10.2（参考: 4.1, 1.2, 5.3, 9.3）
- **Evidence**: design.md「Open Questions / Risks」1〜3 項、「Technology Stack › 伸長」行、research.md 決定 D9

## 3. 設計の強み

- **「書く前に全部見る」を構造で成立させている**: `open` で全エントリを伸長・CRC 突合し、伸長済みバイト列を保持してから `install` が書く。2.6・4.1・5.9・5.11 が同じ 1 か所で成り立ち、「消してから展開して途中で失敗し空になる」経路が「組んでから入れ替え」で消えている（決定 D8）。設計レビュー前の版にあった「`.nar-work/` へ書いた後で CRC 破損を拒否する」欠陥を自分で見つけて直している点も評価できる。
- **依存の選び方が実測に基づき、最小**: `zip` の brief 指定（`deflate-flate2` 単体）が実はコンパイルできないこと、印（ビット 11）を返す文書化 API が無いことを突き止め、本番グラフの増分を 12 から 2 に絞った。読み手 350 行の自前実装という代償も明示され、`zip` へ戻す場合の差し替え範囲が `container.rs` 1 つに閉じている。

## 4. 事実主張の裏取り（依頼された検査項目）

| 項目 | 結果 | 証拠 |
|---|---|---|
| `zip` 8.6 に印（ビット 11）の文書化 API が無い | **真** | `zip-8.6.0/src/lib.rs:40 mod types;`（非公開）・`src/types.rs:168 pub struct ZipFileData`／`:178 pub is_utf8`・`src/read.rs:728 pub trait HasZipMetadata { fn get_metadata(&self) -> &ZipFileData }`。docs.rs の `ZipFile` 頁に印を返す method は無い。`get_metadata().is_utf8` はコンパイルは通るが文書外の経路 |
| `deflate-flate2` 単体は `compile_error!` | **真** | `zip-8.6.0/Cargo.toml`: `flate2 = { version = "1.1", default-features = false, optional = true }`・`deflate-flate2 = ["_deflate-any", "dep:flate2"]`（バックエンド feature を足さない）→ `flate2-1.1.10/src/lib.rs:128 compile_error!("You need to choose a zlib backend")` |
| `miniz_oxide` 0.9.1 の feature | **真（10.2 の判定は指摘 2）** | `default = ["with-alloc"]`・`src/lib.rs:31-33 #[cfg(feature = "with-alloc")] pub mod deflate; pub mod inflate;`・`inflate/mod.rs:177 decompress_to_vec_with_limit` は `with-alloc` 括り。伸長専用 feature は無い |
| 本番グラフに増えるのは 2 crate | **真** | `cargo tree -i miniz_oxide@0.9.1 -e normal` → 「nothing to print」（今日の `flate2`／`png`／`image` は dev 側だけ）。`about.toml` は `ignore-dev-dependencies = true` なので謝辞の差分は `miniz_oxide`・`adler2` に閉じる見込み |
| Windows の `rename`（フォルダ入れ替え） | **真** | rustc 1.98.1 `std/src/sys/fs/windows.rs:1321-1390`: `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` が `ACCESS_DENIED` のとき `FileRenameInfoEx`（`REPLACE_IF_EXISTS | POSIX_SEMANTICS`）へ退避し、宛先が空でないフォルダなら `DIR_NOT_EMPTY` を返す。空フォルダは置き換わる。設計は失敗後に宛先の実在で勝敗を見るのでエラーコードに依存しない。使用中（中のファイルを誰かが開いている）のフォルダは `rename` できないが、設計はそれを「古い原本の回収が失敗しうる」として Risks に載せている（回収の失敗は取得を失敗させない、と明記するとよい＝節 5） |
| `FILE_SHARE_READ` の札で孤児を見分ける | **成立** | 札は `write(true).create_new(true).share_mode(FILE_SHARE_READ)`。他者の `remove_file`（`DeleteFileW`）は DELETE アクセスが要るので共有違反で失敗し、持ち主が死ねば成功する。同一ドライブか否かは無関係。ウイルス対策が孤児の札を掴んでいる瞬間は「生きている」と誤判定するが、次の掃除で回収される（漏れる方向にしか誤らない）。木のファイルを掴まれて `remove_dir_all` が途中で失敗する場合に備え、掃除の失敗は取得を失敗させない扱いが要る |
| 見張りの語彙（同梱バルーン名） | **今日の実体 27 ファイル中 22 に当たる・偽陽性なし** | 実行行で `emo2-kakukaku` を綴る `.rs` は 27。`join("emo2-kakukaku")`／`emo2-kakukaku/`／`/emo2-kakukaku"` の 3 語が実行行で当たるのは 22。当たらない 5 のうち 3 は説明文（`balloon_model_tests.rs:10`・`balloon_target_tests.rs:434`・`state_cue_apply_tests.rs:608`＝当たらないのが正しい）、**2 は補助関数に名前を渡してパスを組む形** `emo2("emo2-kakukaku")`（`areka/examples/emo-present/setup.rs:147`・`areka-emo-atlas/src/emo2_e2e.rs:215`。同形は `areka/src/placement/measure_tests.rs` にも 10 行あるが同ファイルは別の語で当たる）。`balloon("emo2-kakukaku")` は 3 語のどれとも一致しない。research.md 節 10 の「パスを組むのは setup.rs:147 だけ」は emo2_e2e.rs:215 を数え落としている（38 の内側なので書換対象ではある） |
| dev 依存の循環 | **cargo が受理・テスト緑** | 使い捨てワークスペース（`parsers →dev kit → nar → parsers`・`nar →dev kit`）で `cargo test --workspace` 緑。`cargo test -p nar -v` の rustc 起動は `parsers lib`・`nar lib`・`kit lib`・`nar --test` の各 1 回＝`nar` の型は「kit が見る写し」と「テスト対象」の 2 系統になる。窓口はパスしか渡さないので跨がないが、`SampleError::Nar(areka_nar::NarError)` の中身は**写し側**の型なので、`areka-nar` 自身のテストで `crate::NarError` と突き合わせないこと（節 5） |
| 7.5／7.6 の競合安全性 | **指摘 1 を直せば成立** | 原本への入れ方（作業フォルダ → `rename`）と `rename` の勝敗判定は正しい。破れるのは「初回展開中の作業フォルダを別プロセスの掃除が消す」経路だけ（指摘 1） |
| 1,000 行 | **新設は全て見積り 350 以下・例外表に触れない** | 例外表は `log-capture-kit/tests/file_length_guard_test.rs` の `OVER_LIMIT_ALLOWED`（10 件）で設計は触れない。ただし段 ① で書き換える `crates/areka/src/emo2_boot/spine.rs` は**今日 968 行**（残り 31 行）。`with_default_guard_test.rs` 889 行は関数を移すので減る・`workspace_scan/mod.rs` 356 行は増えても余裕 |

## 5. 議題にはしないが実装前に拾っておく観察

1. `spine.rs`（968 行）の段 ① 差分は正味 +31 行以内に収めるか、`emo2_root()` の置換で行数を減らす（段 ③ で `remove_dir_all` を消すと 10 行ほど戻る）。
2. `roadmap.md` は 36 行目・132 行目・148 行目で「`zip`（`nar-install`）の依存承認」と綴る。設計の Modified Files は `roadmap.md` を「実機運転の定石に 1 行」しか挙げていないので、`miniz_oxide` が承認されたらこの 3 か所も同じコミットで直す。
3. 見張りの語彙に `emo2("emo2-kakukaku")` の形（補助関数へ名前を渡す）を 1 語足すと、今日の実体 27 ファイル全部を機械で確かめられる（段 ③ 以降はこの形が実在しないパスを指すのでテスト自身が赤になるが、段 ① の「全て窓口へ寄った」の証明は語彙だけが頼り）。
4. 古い原本の回収・掃除の失敗（別プロセスが複写中・ウイルス対策が掴んでいる）は `debug!`／`warn!` に出して取得は続行する、と明記する。今の文はどちらとも読める。
5. `install` が作る `root/.nar-work/<pid>-<連番>/` は確定後に消す（空の `.nar-work/` を原本に残すと全複製に写る。8.4 のファイル集合比較には影響しないが無駄）。
6. `areka-nar` 自身のテストで `sample_ghost_kit` から返る `NarError`／`crc32` は依存の写し側の型。`WorkDir` と `nar_writer` だけを借り、`SampleError::Nar` の中身を `crate::` の型と比べない。
7. i686 の `pilot` helper は `GHOSTDIR` も引数も無いときだけ `default_fixture_ghostdir()` に落ちる（`helper.rs:115-134`）。窓口へ寄せると、この退避経路は helper 自身が複製を取得するので `SampleRoot` をプロセス寿命で保持する必要がある（`Drop` で木が消える）。親から渡される通常経路は影響なし。
8. 下流（`network-update`）への注記: 「組んでから入れ替え」は宛先フォルダの中のファイルが開かれていると `rename(dest → old)` が失敗する。起動中のゴーストの更新は先に SHIORI を解放してから呼ぶ形になる。本仕様の範囲外だが Revalidation Triggers に 1 行あるとよい。
9. `Overlay` は既存の木を丸ごと作業フォルダへ複写してから上書きする。ゴースト 1 体ぶんの複写が毎回入るが、開発用の根では通らず（6.5）、製品側は 1 回のインストールなので許容。

## 6. 最終判定

**GO**（条件: 指摘 1 の文言追記と、指摘 2 の 6.2・10.2 を設計ディスカッションで確定し要件へ反映）。

**理由**: 既存構造との整合・依存の向き・見張りの置き場・要件被覆に食い違いは無く、設計が置いた外部の事実は全て真だった。残る 2 件は構造の作り直しを要さず、どちらも段落単位の追記と要件の 1 句の改訂で閉じる。実装に進んで支障が出る種類の欠陥ではない。

**次の手順**: `/kiro-design-discussion areka-P0-nar-install` で指摘 1・2 を確定 → 決まった内容を design.md（`devroot` 節・Open Questions・Technology Stack の `Cargo.toml` 記述）と requirements.md（6.2・10.2）へ反映 → `/kiro-spec-tasks areka-P0-nar-install`。
