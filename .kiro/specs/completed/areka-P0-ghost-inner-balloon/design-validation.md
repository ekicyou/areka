# 設計の検証: areka-P0-ghost-inner-balloon

- 日付: 2026-10-22
- 対象: `design.md`（要件は確定済み・要件の討議の決め 3 点を含む）・`research.md`
- 進め方: `kiro-validate-design`（分析 → 大事な問題 → 良い点 → GO／NO-GO）。対話なしで判定した。

## まとめ

設計は、一覧を作る口を 1 つ（`balloons_for`）足して 6 か所の呼び手をそこへ替える形と、インストールで同梱の宛先をゴーストの中へ移し、本体と入れ子になる確定を「無い所へ置く」1 手で済ませる形の 2 本で、要件 1〜10 をすべて部品と口に結び付けている。設計の主な主張をコードで確かめたところ、どれも今のコードと合っていた。残る点は、警告の出る条件の書き方と、確かめ残しのテスト 1 本だけで、実装を止めるものは無い。

## コードで確かめた主張

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| 名前で引く 4 か所は「一覧の並びで最初」 | `boot_resolve.rs` `find_balloon_by_name`（152 行）・`shell_balloon_resolve.rs` `resolve_skin_target`（158 行）・`readme/opener.rs` `skin_by_name`（318 行）・`update/desk.rs` の `updateother`（635 行） | 合う。どれも `iter().find` で、並びを「ゴーストの中が先」にすれば要件 3.3 を満たす |
| 一覧を引く所は 6 か所 | `list_balloons`／`balloon_candidates`／`find_balloon` の呼び手を検索 | 合う。`boot_config.rs` 465 行・メニュー・切替・開く・更新・インストールの表示名。他に一覧を読む所（`installedballoonname` など）は無い |
| `resolve_balloon_for_ghost` は起動と切替だけで呼ばれる | `boot_config.rs` 375 行・`ghost_switch.rs` 811 行 | 合う。隠れの記録はゴーストを起こすたびに 1 回になる |
| 組み上げは全配置を先に済ませ、確定は配置の順・失敗は逆順に戻す | `areka-nar/src/lib.rs` 212〜227 行・`install.rs` `commit_all`／`commit_one`／`unwind` | 合う。同梱の組み上げは本体の確定より前に古い `<ゴースト>\balloon\<dir>` を読めるので、同梱自身の `balloon.refresh` で下敷きを作れる。戻しは「同梱を消す → 本体を戻す」 |
| 同梱の `root` を本体の根にすれば作業の場所が 1 つで済む | `install.rs` `area_for`・`lib.rs` 212 行 | 合う。`stage(k)`・`retired(k)` は配置ごとに別 |
| 同梱で `balloon_root` を呼ばなければ根の同名を写す道が消える | `install/destination.rs` `copied_root`・`answer` | 合う。写しは `balloon_root` の答えを作るときだけ起きる |
| `BalloonEntry.root` を読む所は根の口の中だけ | `catalog_roots.rs` 155〜156 行（`install_root`） | 合う。欄の意味を広げても今の読み手は壊れない |
| 一時の置き場の印は根の走査だけが見る | `catalog.rs` `scan_balloons` 211 行 | 合う。ゴーストの中の走査はゴーストのフォルダを直接受けるので `--ghost` でも数える |

## 大事な問題（2 件・どちらも実装を止めない）

### 問題 1: 同梱の取り出し元が書庫の `balloon/<directory>/` そのものの場合の、除外と警告の条件があいまい

- **中身**: CompanionPlan は「本体から書庫の `balloon/<companion.directory>/` の下の項目も除き、除いた項目が 1 つでも在れば warn `companion_overrides_archive_folder`」と書く。作者が SSP と areka の両方で使えるように `balloon.source.directory,balloon/kaku` と `balloon.directory,kaku` を書いた書庫では、その項目は今の「取り出し元の中」の決まりですでに本体から除かれている。設計の文のままだと、正しい書き方の書庫でも警告が出るように読める。
- **影響**: 正しい配布物に毎回警告が残り、記録が信用されなくなる。テストの期待値（警告 1 件）も実装の人によって変わる。
- **直し方**: 除外と警告の対象を「`balloon/<directory>/` の下の項目のうち、どの同梱の取り出し元の中にも無いもの」と書く。取り出し元が `balloon/<directory>` と同じ書庫で警告が 0 件になるテストを `plan_tests.rs` に 1 本足す。
- **要件**: 7.1・7.5・10.3
- **設計の場所**: 「Components and Interfaces」→ CompanionPlan の「責務と制約」・「Testing Strategy」→ インストール

### 問題 2: 書ける根へ写されていないゴーストの中のバルーンの更新先を、テストで固めていない

- **中身**: 要件 8.3 は「変更 0（`Here.balloon_dir`）」としている。`update/worker.rs` の `prepare_targets` は、バルーンの更新先を `copy_for_write` へ通すが、`plan_copy` は親が `<実行ファイルの隣>\balloon` のときだけ写す。ゴーストの中のバルーン（`<根>\ghost\<g>\balloon\<b>`）は写さずにその場所へ書く。`research.md` の Risks は「起動のときのゴーストが `switch_copy` を通るかは読み切っていない」と書いたまま、設計のテストの計画は `desk_resolve_tests.rs` の宛先の確かめだけになっている。
- **影響**: 実行ファイルの隣の根（書けない場所）で動くゴーストでは、ゴーストの中のバルーンの更新が書けずに失敗するおそれがある。ゴーストそのものと同じ扱いなので要件違反ではないが、確かめずに残すと後で見つけにくい。
- **直し方**: タスクに「`plan_copy` がゴーストの中のバルーンの場所で `AsIs` を返すこと」と「起動のときのゴーストが書ける根へ写されるかどうか」を確かめる 1 本を入れる（`update/worker_tests.rs` か `roots/user_root_tests.rs`）。写されない場合の扱いはゴーストそのものと同じでよいことを、そのテストの説明に書く。
- **要件**: 8.1・8.3・10.1
- **設計の場所**: 「Requirements Traceability」8.3 の行・「Testing Strategy」→ 実行中の呼び手・`research.md` の Risks 3 つ目

## 良い点

1. **一覧の並びだけで名前の同着を解いた**。名前で引く 4 か所と段の並び（`resolve_balloon`）を 1 行も変えず、`balloons_for` の並びを「ゴーストの中が先」にするだけで要件 2.5 と 3.3 を同時に満たしている。根だけの口（`list_balloons`・`find_balloon`・`survey`・`install_root`）も変えないので、要件 6 の「変更 0」がコードの形で守られる。
2. **入れ子の確定を最小の手で安全にした**。本体の下敷きと書庫の項目から `balloon\<dir>` を外すので、本体の確定の後は宛先が必ず無く、同梱は「無い所へ置く」1 手で入る。在れば `rename` が失敗して今の逆順の戻しで全体が元へ戻り、記録の無い失敗にならない。今の `commit_all` の約束（全部入るか何も変わらないか）をそのまま使えている。

## 判定

**GO**

- **理由**: 設計の主な主張はコードと合っており、要件 1〜10 のすべてに部品・口・テストが付いている。上の 2 件は書き方の詰めとテストの追加で、どちらもタスクの段で吸収できる。
- **次の段**:
  1. 設計の討議で問題 1・2 を確かめる（直すなら `design.md` の CompanionPlan とテストの計画に 1〜2 行ずつ）。
  2. `/kiro-spec-tasks areka-P0-ghost-inner-balloon` でタスクを作る。`install-live-target-hazards` とは `plan.rs`・`install.rs` が重なるので、同じウェーブに置かない。
