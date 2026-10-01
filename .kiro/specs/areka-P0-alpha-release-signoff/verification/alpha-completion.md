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

| 欄 | 値 |
|---|---|
| コマンド | `pwsh -NoProfile -File tools/test-all.ps1 -License` |
| 回したコミット | `7f8f4e8701ab0e0a890f446665ed1fda715881a0`（署名の根拠のコミット。件名「受入記録 §10 に説明書の突き合わせと機械の確かめを書き §8 に候補 2 つを登記する (2.5)」）。受入記録 §1 の zip の `commit=7f8f4e8` と同じコミット |
| 日時 | 開始 2026-10-01 22:02:51 〜 終了 2026-10-01 22:11:23（`test-all-meta.txt`） |
| 終了コード | `0`（`test-all-meta.txt` の `exit=0`・`test-all.log` の末尾「全段 緑」） |
| 開始時の未コミットの変更 | 0 件（`test-all-meta.txt` の `dirty=0`・`test-all.log` の結果の見出し「検査したコミット 7f8f4e87・開始時の未コミットの変更 0 件」） |
| 回した後の作業木 | `git status --porcelain` が 0 行のまま（全体テストの終了 22:11:23 の後、22:11:47 に始めた zip の組み立ての `package-meta.txt` が `dirty=0`） |
| テストの数 | `test-all.log` の `test result:` の行 118 本を足して 9,326 passed・0 failed・44 ignored。`... FAILED` の行は 0 件 |
| 差し替えた `emo2.nar` で通ったこと（要件 6.2） | 名前に `emo2` を含むテストの行が `ok` 727 件・`FAILED` 0 件（残る 1 件は明示実行だけの `emo2_golden::record_golden ... ignored`）。実物の `emo2` で起動から終了まで回す `emo2_real_run_boots_talks_and_exits_zero ... ok` を含む。起動と終了の往復の `smoke_boot_loop_exit` の 4 本も `ok` |
| 生の記録 | `C:\home\maz\lap-records\alpha-signoff-20261001\test-all.log`・`test-all-meta.txt` |

成功した段の一覧（`test-all.log` の結果の表を逐語）:

```
==== 結果（検査したコミット 7f8f4e87・開始時の未コミットの変更 0 件） ====
OK    i686 ターゲット導入（0 秒・終了コード 0）
OK    i686 成果物ビルド（1 秒・終了コード 0）
OK    fmt --check（4 秒・終了コード 0）
OK    x64 ワークスペース全テスト（427 秒・終了コード 0）
OK    i686 テスト（host-32 系）（41 秒・終了コード 0）
OK    cargo deny check（15 秒・終了コード 0）
OK    cargo about generate（21 秒・終了コード 0）

全段 緑
```

**完了の手順でアーカイブした後の全体テストは本判定の範囲ではない**（§8）。

---

## 2. ⑵ ライセンスの検査と謝辞（要件 7.1）

§1 と同じ 1 回の `test-all.ps1 -License` の後ろの 2 段で回した。

| 欄 | 値 |
|---|---|
| 設定ファイル | `deny.toml`・`about.toml`・`about.hbs`（いずれもリポジトリ直下に在る） |
| ライセンスの検査 | `cargo deny check`（段の終了コード 0）→ **`advisories ok, bans ok, licenses ok, sources ok`**。`error` で始まる行は 0 件。警告の行は 11 件（`warning[duplicate]` 8・`warning[wildcard]` 2・`warning[license-not-encountered]` 1）で、どれも検査を落とさない |
| その出力 | `C:\home\maz\lap-records\alpha-signoff-20261001\cargo-deny.txt` |
| 謝辞の生成 | `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（`test-all.ps1` の段「cargo about generate」・終了コード 0） |
| 差分 | `git diff --quiet -- THIRD-PARTY-NOTICES.md` の終了コード 0＝生成し直したものとリポジトリのものの差分 0。`test-all.log` に「THIRD-PARTY-NOTICES.md に差分あり」の行は 0 件 |
| 生の記録 | `C:\home\maz\lap-records\alpha-signoff-20261001\test-all.log`（`==> cargo deny check` と `==> cargo about generate` の段） |

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
