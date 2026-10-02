# α 完成判定

本書は α（M2）の完成判定を 1 か所に束ねる文書である（design.md「完成判定の文書 `verification/alpha-completion.md`」・要件 7）。章立ては M1 の完成判定（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`）の形を写す。

完成判定は 3 つの根拠で構成する（要件 7.1）。

- ⑴ ワークスペース全体のテスト（`tools/test-all.ps1`）が署名の根拠のコミットで全段成功したこと（§1）
- ⑵ 依存のライセンスの検査と謝辞の生成が通り、生成し直した `THIRD-PARTY-NOTICES.md` がリポジトリのものと差分 0 であること（§2）
- ⑶ 検証項目表の実機一周の結果（§3）

3 つの根拠はいずれも本書の §1〜§3 から辿れる。⑶ の中身は受入記録 `verification/acceptance-record.md` に在り、本書はそこを指すだけで写しを作らない。**α の完成の宣言は開発者の署名（§4）で成り立ち、AI 単独で宣言しない。** 日付と時刻はすべて日本時間で書く。

- 対象仕様: `areka-P0-alpha-release-signoff`
- 本書は節を順に書き足す。まだ中身の無い節は「まだ書いていない」を意味する。

---

## 1. ⑴ ワークスペース全体のテスト（要件 7.1・7.2・6.2）

2026-10-02 に組み直した後の回で書き直した。前の署名の根拠のコミット `7f8f4e87` での回（2026-10-01 22:02:51 〜 22:11:23・全段 緑・`test-all.log`）は、`emo2.nar` の差し替えとテストの欠陥の直しで置き換えた（受入記録 §9.1）。置き換えまでの全体テストの試み 4 回（`test-all-r-try1`〜`try4`）とその理由も受入記録 §9.1 に書いた。

| 欄 | 値 |
|---|---|
| コマンド | `pwsh -NoProfile -File tools/test-all.ps1 -License` |
| 回したコミット | `8460506d95054bfc195e884d13f6b469d93e8491`（署名の根拠のコミット。件名「壊れた DLL を読んでも OS の「正しくないイメージ」の窓を出さない」）。受入記録 §1 の zip の `commit=8460506` と同じコミット |
| 日時 | 開始 2026-10-02 21:42:08 〜 終了 2026-10-02 21:51:05（`test-all-r-meta.txt`） |
| 終了コード | `0`（`test-all-r-meta.txt` の `exit=0`・`test-all-r.log` の末尾「全段 緑」） |
| 開始時の未コミットの変更 | 0 件（`test-all-r-meta.txt` の `dirty=0`・`test-all-r.log` の結果の見出し「検査したコミット 8460506d・開始時の未コミットの変更 0 件」） |
| 回した後の作業木 | `git status --porcelain` が 0 行のまま（`test-all-r-meta.txt` の `dirty_after=0`。続けて 21:51:45 に始めた zip の組み立ての `package-meta.txt` も `dirty=0`） |
| テストの数 | `test-all-r.log` の `test result:` の行 118 本を足して 9,326 passed・0 failed・44 ignored。`... FAILED` の行は 0 件 |
| 差し替えた `emo2.nar` で通ったこと（要件 6.2） | 差し替えた版は ghost_dev `75e560e`（コミット `1717d29f`・受入記録 §9.1）。名前に `emo2` を含むテストの行が `ok` 727 件・`FAILED` 0 件（残る 1 件は明示実行だけの `emo2_golden::record_golden ... ignored`）。実物の `emo2` で起動から終了まで回す `emo2_real_run_boots_talks_and_exits_zero ... ok` を含む。起動と終了の往復の `smoke_boot_loop_exit` の 4 本も `ok` |
| OS の窓を出していたテスト | `test shiori_inproc::tests::invalid_image_returns_err ... ok`・`test inproc_e2e_test::i3_load_failure_invalid_image_returns_err ... ok`（直したコミット `8460506d` の後の回。受入記録 §9.1） |
| 生の記録 | `C:\home\maz\lap-records\alpha-signoff-20261001\test-all-r.log`・`test-all-r-meta.txt` |

成功した段の一覧（`test-all-r.log` の結果の表を逐語）:

```
==== 結果（検査したコミット 8460506d・開始時の未コミットの変更 0 件） ====
OK    i686 ターゲット導入（0 秒・終了コード 0）
OK    i686 成果物ビルド（2 秒・終了コード 0）
OK    fmt --check（7 秒・終了コード 0）
OK    x64 ワークスペース全テスト（377 秒・終了コード 0）
OK    i686 テスト（host-32 系）（43 秒・終了コード 0）
OK    cargo deny check（31 秒・終了コード 0）
OK    cargo about generate（75 秒・終了コード 0）

全段 緑
```

**完了の手順でアーカイブした後の全体テストは本判定の範囲ではない**（§8）。

---

## 2. ⑵ ライセンスの検査と謝辞（要件 7.1）

§1 と同じ 1 回の `test-all.ps1 -License`（`8460506d`）の後ろの 2 段で回した。

| 欄 | 値 |
|---|---|
| 設定ファイル | `deny.toml`・`about.toml`・`about.hbs`（いずれもリポジトリ直下に在る） |
| ライセンスの検査 | `cargo deny check`（段の終了コード 0）→ **`advisories ok, bans ok, licenses ok, sources ok`**。`error` で始まる行は 0 件。警告の行は 11 件（`warning[duplicate]` 8・`warning[wildcard]` 2・`warning[license-not-encountered]` 1）で、どれも検査を落とさない |
| その出力 | `test-all-r.log` の `==> cargo deny check` の段（今回は別のファイルに写していない。前の回の写し `cargo-deny.txt` は `7f8f4e87` の回のもの） |
| 謝辞の生成 | `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（`test-all.ps1` の段「cargo about generate」・終了コード 0） |
| 差分 | 生成し直したものとリポジトリのものの差分 0（`test-all-r-meta.txt` の `notices_diff=0`）。`test-all.ps1` は `-License` のとき `git diff --quiet -- THIRD-PARTY-NOTICES.md` で差分を調べ、差分があれば「THIRD-PARTY-NOTICES.md に差分あり」の行を出す。`test-all-r.log` にその行は 0 件 |
| 生の記録 | `C:\home\maz\lap-records\alpha-signoff-20261001\test-all-r.log`（`==> cargo deny check` と `==> cargo about generate` の段）・`test-all-r-meta.txt` |

---

## 3. ⑶ 実機一周

タスク 5.1 で書く（受入記録の §7・§8 を指す）。

## 4. サインオフ

## 5. 宣言

タスク 5.1 で書く。

## 6. 持ち越した事項

タスク 5.1 で書く。

## 7. 議題 5 の記録

## 8. 次の段階の起点

タスク 5.1 で書く。
