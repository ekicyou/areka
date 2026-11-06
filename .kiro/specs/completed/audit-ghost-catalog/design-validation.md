# 設計の検証: audit-ghost-catalog

- 日付: 2026-11-05
- 読んだもの: `spec.json`・`requirements.md`・`design.md`・`research.md`（§3 の番号）・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C17 の行）
- 実物で確かめた所: `crates/areka-nar/src/install.rs`（`WorkArea::create_at`・`prepare_shelf`）・`crates/areka-nar/src/container.rs`（`find_eocd`・`read_eocd`・`read_central_header`）・`crates/areka-nar/Cargo.toml`・`crates/areka-ghost/tests/ghost/inproc_fixture.rs`（`shared_test_ghost`）・`crates/areka-ghost/src/shiori_inproc.rs`（`LoadLibraryW`）・`crates/areka-ghost/src/shiori_wiring.rs`（`helper_load_dir`）・`crates/areka-user-data/src/lib.rs`・`establish.rs`・`crates/log-capture-kit/tests/temp_path_guard_test.rs`・`file_length_guard_test.rs`・`crates/areka-ghost/src/catalog.rs`・`config.rs`・`sylphya_wiring.rs`

## まとめ

設計は 88 件の直しを 1 件ずつ「直し方・範囲の外・縛るテスト・束」で引けるようにしてあり、実物と照らした主な主張（棚の作り・書庫の読みの形・共有ゴーストの消し方・DLL の読み込み・番人の表の件数と行数）は合っている。ただし #18 の直しが、`AREKA_PROFILE_DIR` を見る 4 か所のうち 2 か所しか通さない形になっていて、空や相対の値のときに記憶の移しが黙って飛ぶ。ほかに、1,000 行の番人を理由に直しを控える書き方と、同じ表を持つ別の spec との重なりの書き漏れがある。どれも設計の手直しで済むので、判定は GO。

## 大事な指摘（3 件）

### 指摘 1: #18 の検査が、環境変数を見る 4 か所のうち 2 か所にしか届かない

- **何が問題か**: 設計は `usable_profile_env` を `app_profile_dir` と `readonly_app_profile_dir` にだけ通す（「指摘ごとの直し方」#18 の行）。ところが `crates/areka-user-data/src/establish.rs` には、生の `profile_env.is_none()` で決める所があと 2 つある。`establish_from`（100 行目・在れば記憶を移さない）と `locate_app_profile`（175 行目・MCP の橋が使う。在れば「根が無い」「移す前」の答えを飛ばす）。
- **何が起きるか**: 空か相対の値のとき、置き場は書ける根の `profile\areka` に倒れるのに、`establish` は「値が在る」とみなして記憶を移さない。利用者には、同梱の記憶が消えたように見える。橋の側は、まだ作っていない・移していない置き場を `Ready` で返す。直したはずの所で別のずれが残り、要件 9.4（`AREKA_USER_ROOT` と同じ決まり）を半分しか満たさない。あわせて、`default_app_profile_dir()` は起動の中で何度も呼ばれる（`main.rs` 223・267 行目・`install/startup.rs`）ので、`error!` を「1 回」出すという書き方とも合わない。
- **直し方の案**: 値の検査を 1 か所の関数に置き、4 か所すべての判定（移すか・橋の答え・置き場・読み取りだけの置き場）をその結果で行う。記録は入口（`establish`・`establish_readonly`・`locate_app_profile`）で 1 回にし、`app_profile_dir` は黙って捨てるだけにする（または回数を doc に書く）。縛るテストに「空の値でも `establish` が記憶を移す」と「空の値で `locate_app_profile` が `MigrationPending` を返す」を 1 本ずつ足す。
- **要件**: 9.4・3.2・4.1・5.2
- **設計の場所**: 「指摘ごとの直し方」Should fix の #18 の行・「失敗の扱い」の `areka-user-data` の段

### 指摘 2: 「1,000 以下になる直しはしない」は、番人を理由に直しを控える形になっている

- **何が問題か**: 「1,000 行の番人」の表は、`inproc_e2e_test.rs` について「1,000 以下になる直しはしない」と書き、`file_length_guard_test.rs` の例外表は触らないと決めている（「範囲の外」）。見立ては約 1,045 行（下は 1,030）で、余りは 3〜4% しかない。#42 の寄せや #85 の輪の 1 本化が見立てより多く減れば、1,000 を下回る。
- **何が起きるか**: 下回ったときの道が「直しを控える」しか無い。これは要件 3.1（全部直す）と 8.2（かけ直しで 0 件）に反し、記憶の決めごと「設計は本質的な案で・並走の『触らない約束』を理由に回避の形へ逃げない・膨らむなら他セッションと調整」にも反する。
- **直し方の案**: 下回ったら、例外表から `inproc_e2e_test.rs` の行を消し、件数（`OVER_LIMIT_ALLOWED_COUNT` 9→8）を合わせる、と書く（直しのために要る変更＝要件 6.2）。「範囲の外で触るファイル」の表に条件つきで足す。同じ表には C17 で並走する `audit-emo-compose`・`audit-emo-present`・`audit-seriko` の行もあるので、件数の行はぶつかり得る。触る前にその 3 本へ知らせ、着地の後に表の行数から数え直す（手で写さない）。
- **要件**: 3.1・5.1・6.1・6.2・8.2
- **設計の場所**: 「1,000 行の番人」の表・「境界の約束」の「範囲の外」の `file_length_guard_test.rs` の項

### 指摘 3: 一時フォルダの番人の表を持つ別の spec との重なりが書かれていない

- **何が問題か**: 設計は「C17 で並走しているのは `audit-parsers-sakura` だけ」と書く（「範囲の外で触るファイル」）。しかし `temp_path_guard_test.rs` の例外表には C17 の `audit-emo-present` の行（`crates/areka-emo-present/src/balloon_test_support.rs`）もあり、同じ件数の行（`ALLOWED_COUNT`・`PROCESS_UNIQUE_COUNT`）を相手が直す見込みがある。さらに、この表の 19 行を `target\test-roots` の下へ移すこと自体が、まだ始めていない別の spec `temp-path-exceptions-to-target`（roadmap の「版に縛らない」の列）の受け持ちで、この spec がそのうち 5 行を先に取る。そのことが設計にも roadmap にも書かれていない。
- **何が起きるか**: 件数の行がぶつかったとき、どちらの数が正しいかを手で決めることになる（番人のテストが赤で知らせるので黙っては通らないが、直しの手戻りになる）。`temp-path-exceptions-to-target` の brief と roadmap は 19 行のままで、後から始めたときに範囲を測り違える。
- **直し方の案**: 「範囲の外で触るファイル」に 2 点を足す。⑴ T10・T15・T17 の前に `main` を取り込み、`audit-emo-present` がこの表を触ったかを見て、件数は表の行から数え直す。⑵ 完了のときに roadmap の `temp-path-exceptions-to-target` の行を「残り 14 行（この spec が 5 行を済ませた）」へ直す（`/kiro-complete` の roadmap の手入れで行う、と書く）。
- **要件**: 9.3・6.1・6.2
- **設計の場所**: 「範囲の外で触るファイル」の表の `temp_path_guard_test.rs` の行と、その上の「C17 で並走しているのは…」の一文

## よい所

1. **直しと縛るテストが 1 件ずつ引ける**: 88 件すべてに直し方・束・縛るテスト（または「今のテストが同じ結果で通る」）が付いている。#1 と #45 は「錠を外すと赤」を 1 つのスレッドの順で再現し、待ち時間を使わない。#16 は時刻を渡し、#74 は小さなスタックで再帰に戻すと落ちる形にしてある。要件 4.1・4.2 にまっすぐ届いている。
2. **実物に根を下ろしている**: 確かめた範囲で主張は合っていた。`container.rs` は終端記録の探しが「末尾＝ファイルの終わり」だけに頼るので、末尾の窓（22＋65535＋20）でそのまま動き、zip64 の目印も窓に入る。`WorkArea::create_at` と `prepare_shelf` の作りは錠を差し込める形。`shared_test_ghost` は呼ぶたびに `profile\areka` を消している（307 行目）ので #45 の競りは実在する。`LoadLibraryW` から `LoadLibraryExW` へは、ワークスペースの `Win32_System_LibraryLoader` の機能の中で替えられる。番人の表の件数（19・16・2・1、較正は `inproc_fixture.rs`）と `inproc_e2e_test.rs` の 1,160 行も書いてあるとおり。

## 小さな注（判定には効かない）

- `areka-nar` は `windows` に依らないので、`FILE_FLAG_DELETE_ON_CLOSE` は数の直書き（`0x0400_0000`）に名前を付けて使うことになる。設計の「依存は増えない」と合うが、一言書いておくと迷わない。
- 目次を切り出して読む形にすると、`read_central_header` の拒みの文に出る「位置」が目次の頭からの値に変わる。今までどおりファイルの頭からの位置を出すなら、目次の位置を足して表示する。
- `peek_install_txt` は目次の大きさに上限を持たない（ファイルが実際に大きければ最大で約 4 GiB を UI スレッドで読む）。`open` と同じ `MAX_ARCHIVE_LEN` の先回りを判定にも置くと、答えもそろい、読む量も抑えられる。

## 判定

**GO**

- **理由**: 層の向き・失敗の記録の道・依存の扱いは今の作りと合っており、大きな作り直しは要らない。3 件の指摘はどれも設計の文の足し直しと縛るテストの追加で片が付く。
- **次の段**: 設計の討議（`/kiro-design-discussion audit-ghost-catalog`）で上の 3 件を議題にし、設計を直してから `/kiro-spec-tasks audit-ghost-catalog` へ進む。
