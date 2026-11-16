# 設計の検証: audit-tools-scripts

- 日付: 2026-11-16
- 対象: `design.md`（750 行）・`requirements.md`（要件 1〜9・討議の確定 7 件）・`research.md`（5 節の指摘 1〜19・14 節の設計の段の調べ）
- 読んだ steering: `roadmap.md` の C21 の行（触るファイル ①〜⑤・約束 ⑴〜⑸）・`tech.md`・`structure.md`
- 突き合わせた実物（読むだけ）: `tools/impl-watch-install.ps1`・`tools/mcp-bridge-install.ps1`・`tools/package.ps1`・`tools/encoding-check.ps1`・`tools/release-notes.ps1`・`tools/test-all.ps1`・`tools/app-icon.ps1`・`tools/cursor-art.ps1`・`tools/perf/perf-loop.ps1`・`.github/workflows/release.yml`・`.claude/skills/kiro-complete/SKILL.md`・`.claude/skills/release/procedure.md`・`.claude/skills/update/SKILL.md`・`.kiro/specs/runtime-crate-split/brief.md`

## 設計の検証の要約

19 件の指摘すべてに、直し方・テストの名前・次の本当の操作で確かめるかが 1 行ずつ対応し、要件 1〜9 の受け入れ条件はすべて部品か流れに割り付けてある。設計が拠り所にした事実（`impl-watch-install.ps1` の手 3〜7 の順、`[System.IO.File]::Copy` での写し、`package.ps1` の `-Arch` を文字列で受けて終了コード 3 で断る形、`Invoke-Cleanup` が完成品を消す条件、`release.yml` の段「版の検査」の 54〜73 行、規則 E の表、`test-all.ps1` の段の並び、読み手の行番号〔`kiro-complete` 48・82・436・437 行、`procedure.md` 254 行、`update` 135 行〕）は、実物と一致した。実装へ進める水準にあるが、C21 の並走の約束に関わる 1 点と、全体テストを最初から赤にしうる 1 点と、見どころの検査の名前の一覧の 1 点を、設計の討議で決めておく必要がある。

## 重大な指摘（3 件）

### 指摘 1: `tech.md`・`structure.md` は `runtime-crate-split` の触るファイルにほぼ確実に入る。「実装の頭で確かめる」で先送りせず、今決める

- **懸念**: 設計は `tech.md`（Testing・Required Tools）と `structure.md`（19 行）を書き替える予定で、重なるかどうかを Open Questions の 1 に残している。しかし `runtime-crate-split` の brief（59 行）は「`structure.md` の新しい crate の節」と「文書のパス」を範囲に挙げている。`tech.md` には 53・55・90・155 行に `crates/areka/src/runtime/` のパスが、`structure.md` には 110〜118 行に同じパスがある。2 本とも相手が書き替えるファイルと見てよい。
- **影響**: C21 の約束 ⑸「⑤ は ④ と、`tools/load-flake.ps1` の選択肢のほかに書き換えるファイルが重ならない」と要件 7.3 は、ファイル単位で重ならないことを求めている。行が重ならなくても、両方が同じファイルを書けば約束に反する。さらに要件 9.5 の「既定のブランチとの差分で確かめる」で、自分で反したことを記録することになる。要件 5.4 の「前提は `tech.md` の Testing に書く」も、どちらが書くかが決まらない。
- **提案**: `load-flake.ps1` の選び肢（要件 7.5・7.6）と同じ「着地の順で決める」形にそろえる。実装の終わりより先に `runtime-crate-split` が着地していれば、この spec が 2 本を書く。着地していなければ 2 本には書かず、書く文と場所を相手のセッションへ申し送り、記録に残す（要件 7.4）。要件 5.4 の前提の 1 文も同じ扱いにする。Open Questions の 1 はこの形で閉じ、「行が重ならなければ書いてよいか」は開発者に聞かない。
- **要件との対応**: 要件 7.3・7.4・7.8・5.4・9.5、C21 の約束 ⑸
- **根拠**: design.md「File Structure Plan > 範囲の外で直すファイル」の末尾、「C21 の約束の確かめ」の 2 点目、Open Questions の 1。`research.md` 14.4 の 1 点目

### 指摘 2: `cursor-art` の「コミット済みと同じバイト」は、GDI+ のなめらかな塗りが、コミットした時と同じであることに頼る。今そろっているかを確かめておらず、ずれていたときの道が無い

- **懸念**: 指摘 12 の判定 `committed outputs are current` は、組んだ結果をコミット済みの出力とバイトで比べる。`app-icon.ps1` は、コミット済みの PNG を詰めるだけなので、どの机でも同じバイトになる。一方 `cursor-art.ps1` は、SVG の経路を GDI+ でなめらかに塗る（説明の 3 番）。今ある `same bytes twice` が縛るのは、同じ机で 2 回組んで同じになることだけである。`research.md` 14.1 で `cursor-art.ps1 -SelfTest` を回したが、組んだ結果がコミット済みの `hand.cur`・`grip.cur` と同じかは測っていない（今は比べる処理が無い）。
- **影響**: 2026-10-30 のコミットの後に OS の更新などで GDI+ の塗りが 1 画素でも変わっていれば、新しい段 `cursor art check` は直した日から赤になる。それを直すにはコミット済みの `.cur` を作り直すしかなく、これは要件 8.1 が禁じる「コミット済みの資源を書き替える本番の走り」に当たる。後で OS が更新されたときも、コードを変えていないのに全体テストが赤になる（要件 4.3 の決定論）。
- **提案**: ⑴ 束 8 の頭で、写しで組んだ結果とコミット済みの `.cur` をバイトで比べ、同じかどうかを記録に残す（`target` の下だけで済み、8.1 に当たらない）。⑵ 違っていたときは段を足さずに議題にし、扱いを開発者に決めてもらう、と設計に書く（作り直してコミットするか、この判定を `app-icon` だけに絞るか）。⑶ 落ちたときの 1 行に「絵を直したのに回し忘れた」のほかに「塗りの結果が変わった」もありうることを書き、`tech.md` の段の説明にも添える。
- **要件との対応**: 要件 4.3・4.4・8.1・8.3、指摘 12
- **根拠**: design.md「直し方の一覧」の 12 の行、「sample-fetch・crates-io・app-icon・cursor-art」の節、Testing Strategy の T1 の `committed outputs are current`。`tools/cursor-art.ps1` の 13〜20 行（GDI+ の GraphicsPath でなめらかに塗る）

### 指摘 3: 見どころの検査で `-SelfTest` が使う spec 名の一覧が、設計の中で食い違っている。本物の見どころは本物の一覧で検査する

- **懸念**: release-notes の節は「`-Check`・`-Notes`・`-Body` は `Get-SpecNames $root`（本物の一覧）を使い、`-SelfTest` は見本の `spec-names.txt` を使う」と書く。一方、同じ節の Validation は「`doc/release-notes/v*.md` の実物が全部通ることは、今の自分の検査が確かめる（実物の一覧で `spec-name` が誤って当たらないこと）」と書く。今の `-SelfTest` は、本物の見どころ `real highlights <名> passes`（`release-notes.ps1` 252〜256 行）も同じ `Test-Highlights` で検査している。この検査がどちらの一覧で回るのかが決まっていない。
- **影響**: 見本の一覧で回すと、本物の見どころに今の spec 名（たとえば `app-icon`）が入っても、全体テストは緑のまま、手元の `-Check` と CI の段「見どころの検査」だけが赤になる。`release/procedure.md` の 134 行は「3.1 の全体テストも同じファイルの形を見る」を前提に手順を組んでいるので、その前提が黙って崩れる。今のところ、本物の 495 の spec 名と今ある見どころ（`doc/release-notes/v*.md`）の間に当たりは 0 件である（この検証で測った）。
- **提案**: 本物の見どころの判定は `Get-SpecNames $root`（本物の一覧）で回し、見本の `spec-names.txt` は整形の見本（`subjects.txt`・`expected-*.md`）と、壊した見本の判定にだけ使う、と release-notes の節と Testing Strategy に書き分ける。本物の一覧が 0 件なら `FAIL specs` で赤にする形は `-SelfTest` にも効かせる。
- **要件との対応**: 要件 4.7・3.2・6.2、指摘 1
- **根拠**: design.md「release-notes」の節（`-SelfTest は見本の spec-names.txt`／Validation の 1 文）、Testing Strategy の「T2 mouse-input・T1 に足す検査」の release-notes の 3 行

## 設計の良い所

1. **外への約束の扱いがはっきりしている**: 約束を変える 10 件と件 1・6 を表にし、それぞれの読み手（`doc/impl-watch.md` の表と例・`kiro-complete` の段の一覧と所要時間・`release.yml` の段の本文・規則 E の表）を、行の単位まで挙げて同じ変更の中で直す計画になっている。CI でしか走らない約束（1・9・14・15・16・17）には、「次のリリースで確かめる」の記録が番号ごとに付いている。調べた範囲の行番号と文は、どれも実物と一致した。
2. **8.1 の操作を回さずに、分かれ道の筋を決定論で縛る作りになっている**: 机へ置く手順は偽の置き場と、中身を決めた偽の exe と、偽の確かめで回す（2 つ目の改名の失敗も、削除の共有なしで握った取っ手で起こす）。`Ctrl+C` で後始末が飛ぶ件と、押したボタンが離れない件（5・6）は構文木で縛る。負荷の子が親と一緒に止まる件（7）は、偽の親と負荷の子 1 本で確かめる。写しを寄せる先も、仕事ごとに 2 本（`tool-common.ps1`・`exe-install.ps1`）に分けた。読み込む側の引数を消さないよう `param` を置かない、という決まりも理由つきで書かれている。

## 最終の判断

- **判断**: GO
- **理由**: 設計の作りを大きく組み直す問題は無く、要件はすべて割り付いていて、拠り所にした実物の事実も合っている。上の 3 件は、設計の討議で文を足すか決めを 1 つ入れれば片づき、作り（2 本の共通の部品・7 段・T1〜T3）は変わらない。
- **次の手**:
  1. 設計の討議（`/kiro-design-discussion audit-tools-scripts`）で、上の指摘 1〜3 と、設計の Open Questions の 2（7.7 の 10 件の外で変わる 6 と 18 の約束＝要件 9.5 の記録の範囲）を議題にする。Open Questions の 1 は指摘 1 の形で閉じる。
  2. 決めた分を `design.md` に入れたら、`/kiro-spec-tasks audit-tools-scripts` でタスクを作る（見立ては 15〜18）。

## 気付いたが重大でないもの（討議で扱うかは任意）

- `mcp-bridge-install.ps1` の確かめは、15 秒で止まらないとき `Kill()` を呼ぶが、その後に終わるのを待たない。置き換える前に `.new.exe` を確かめる形になると、確かめに落ちた後に `.new.exe` を消す処理が、まだ動いている exe に当たって失敗しうる。`Install-ToolExe` の「落ちたら `.new.exe` を消す」で、消せなかった場合の扱い（終わるのを待つか、残ったことを 1 行出すか）を決めておく。
- 規則 E の表から `release.yml` の行を消すと、`release.yml` が版を `Get-ArekaVersion` で読んでいることは、規則 D（素の `cargo metadata` を落とす）でしか縛られなくなる。今の振る舞いは保たれるが、守りは 1 段弱くなる。
