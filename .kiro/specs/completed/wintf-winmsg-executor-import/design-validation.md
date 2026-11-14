# 設計の検証: wintf-winmsg-executor-import

- 日付: 2026-11-14
- 対象: `design.md`（`084ea2fb6` の時点）・`requirements.md`（承認済み）・`research.md`
- 進め方: `kiro-validate-design`（`rules/design-review.md` の 分析 → 重大な指摘 → 良い点 → GO／NO-GO）。対話なしで判定まで出す。

## 設計の検証の要約

設計は、既に在る `dola` の「版＋path」の形と、`tools/crates-io.ps1` の「較正値は冒頭の 1 か所」の決まりをそのままなぞり、決め打ちの数を 2 から 3 へ直すだけの小さな変更に収めている。要件 1〜7 の全項目が追跡の表で部品と確かめ方に結び付き、主な主張は実物と食い違わない。重大な指摘は 2 件で、どちらも人の手に残る操作の時期の決めであり、作りの直しではない。

## 実物との突き合わせ（読んだ物）

| 設計の主張 | 確かめた物 | 結果 |
|---|---|---|
| 根の `Cargo.toml` 27 行目が `wintf-winmsg-executor = "=0.0.5"`・`dola` が「版＋path」・ワークスペースの版 0.1.0・`repository` は areka | 根の `Cargo.toml` | 合う |
| `crates/pilot/Cargo.toml` が `"0.0.5"` を直書きし、その上に 0.0.5 の注記 | `crates/pilot/Cargo.toml` 15〜19 行目 | 合う |
| 元の `4f20275` の manifest（edition 2021・`MIT OR Apache-2.0`・依存 2 つ・機能 4 つ・`include` 5 つ・docs.rs）と、写す 10 個のファイル | `git -C C:\home\maz\git\wintf-winmsg-executor show 4f20275:Cargo.toml`・`ls-tree` | 合う。`main` は `4f202758…`、作業木は汚れていない（読んだ時点） |
| 単体テスト 15 本（`lib.rs` 13・`window.rs` 2）と doctest 1 本 | `git grep` | 合う（doctest は `compile_fail`） |
| 道具の較正値 `$PUBLISH` と正しい見本 `$good`・`wintf does not publish` の正規表現が写しの項に当たらない・`Test-Fields` は `description`・`license`・`repository` を見る・`-Pending` は 4 文字以上の名前の索引の置き場 | `tools/crates-io.ps1` 68・96〜114・183〜222・262〜323 行目 | 合う。大きさ・索引の較正は `dola`・`wintf` の名前の固定値で、一覧の数に依らない |
| 公開の段は 1 行目の注記だけを直せばよい・認証は Trusted Publishing だけ | `.github/workflows/crates-io.yml` 1 行目・135・141・160 行目 | 合う |
| `release.yml` は直さなくてよい | 版の検査は `name -ceq 'areka'` の 1 つだけを読む | 合う |
| `/release` の「2 行」の置き場（56・58・82・84・231・289・372・389 行目・`SKILL.md` 20 行目）と、`Cargo.lock`・謝辞の判定が `N` を使う | `.claude/skills/release/procedure.md`・`SKILL.md` | 合う |
| 謝辞に著作権の 2 行が在り、ワークスペースのクレートも謝辞に載る・`about.toml` に名指しの設定は無い | `THIRD-PARTY-NOTICES.md` 3362〜3368 行目・`about.toml` | 合う（写しの項は 1 つだけ） |
| 全体テストに clippy の段は無く、`-D warnings` も無い・git のフックも無い | `tools/test-all.ps1` の説明・`core.hooksPath` | 合う（写しを 1 字違わず置いても、警告で赤にならない。コミット 1 の解決できない状態もフックに止められない） |
| `.gitattributes` が無く `core.autocrlf=true` | 作業木 | 合う（blob の番号の突き合わせが成り立つ） |
| `deny.toml` は path の依存のワイルドカードを許す | `allow-wildcard-paths = true` | 合う |

## 重大な指摘（2 件）

🔴 **重大な指摘 1**: crates.io の `wintf-winmsg-executor` の Trusted Publishing の設定を、この spec の間に頼む形にしていない

- **懸念**: 設計は設定を「完了の報告で申し送る」（要件 7.1）に留め、次のリリースで設定が済んでいるかを確かめる場所を持たない。`/release` は承認の止まりなしに回るので、設定の前にタグが打たれると、公開の段は `dola` だけ上げて写しで断られて止まる（System Flows「`wintf-winmsg-executor` だけ設定が無いとき」）。crates.io の設定は持ち主のログインでしか読めないので、道具で前もって見つける手も無い。
- **影響**: 要件 4.9 の狙い（次のリリースで areka の CI が 3 つを出す）が、申し送りが読まれたかどうかだけに懸かる。止まってもやり直しで戻せるので壊れはしないが、`dola` だけが新しい版で crates.io に在る中途の状態が出る。
- **提案**: 設計の手順書 2 節が述べるとおり、写しは crates.io に既に版を持つので、設定は今すぐできる。元のリポジトリの後始末の許しを求めるチャット（要件 6.1）と同じ時に、開発者へ設定を頼む文（値は `ekicyou`／`areka`／`crates-io.yml`／environment は空）を一緒に出し、完了の報告では「済み」か「残り」かを示す形にする（要件 7.1 はそのまま満たす）。`/release` の手順は直さない。
- **追跡**: 要件 4.9・5.2・5.3・7.1
- **根拠**: design.md「System Flows / 次のリリースでの公開」・「公開の段 / Implementation Notes（Risks）」・「完了の報告」

🔴 **重大な指摘 2**: 元のリポジトリの README の移転の案内と archive が、areka の PR が本線に入る前に済む順になっている

- **懸念**: 設計は後始末を「areka の側の変更が全体テストで緑になった後の最後のタスク」に置く。`/kiro-complete` が PR を出して squash マージするのはその後なので、README の「以後の版は areka から出る」と archive が、移転先の `main` にまだ写しが無いうちに外へ出る。PR が止まるか差し戻されると、元のリポジトリは読み取り専用で、新しい置き場は本線に無い、という間が開く。
- **影響**: 壊れる物は無い（archive は持ち主が戻せ、リンクはリポジトリの根を指すので切れない）。ただ、元のリポジトリを見た人には、まだ本当でない案内が見える。
- **提案**: どちらかを設計の討議で決める。⑴ 今の順のままにし、許しを求める文に「areka の PR はまだ本線に入っていない・archive は持ち主が戻せる」を書く。⑵ README の PR だけを今の位置で行い、archive は squash マージの後に回して、完了の報告に「archive は本線に入った後に行う」と残りとして載せる（要件 6.2 の残りの扱いに当たる）。共通の文書（`/kiro-complete`）は直さない形で収まるのは ⑴ で、推すのは ⑴。
- **追跡**: 要件 6.1・6.3・6.4・7.2
- **根拠**: design.md「System Flows / 取り込みのコミットの切り方」の図（`Full --> Cleanup`）・「元のリポジトリの後始末 / Batch / Job Contract（Trigger）」

## 重大でない覚え書き（討議の議題にしなくてよい）

- コミット 1 の時点では、ワークスペースの一員の写し（版 0.0.5・path）と、根の crates.io の `=0.0.5` が並び、解決できないことがある。設計は「確かめの地点ではない」と明記し、squash マージで 1 つになり、フックも無いので害は無い。
- cargo の複数クレートの公開の順（依存の済んだ集まりを名前の順で 1 つずつ）は、cargo の `publish.rs` を読んだ裏づけが research.md 10.1 に在る。実際の公開では試していない（この spec の間に出さない決まりどおり）。

## 良い点

1. **既に在る型をなぞり、新しい部品を足さない**: 依存は `dola` の「版＋path」、版上げは `cargo set-version --workspace`（path の行も上げることを試し D で確かめ済み）、公開の順は cargo に任せ、道具は較正値と見本の 2 か所だけ。`test-all.ps1`・`deny.toml`・`release.yml` に触らない理由を、名前の一覧を持たないことで示している。変更が小さく、読み手が確かめやすい。
2. **確かめが実物の数で書かれている**: blob の番号の 10 個の突き合わせ・`Cargo.lock` の差分が写しの項の中だけ（`windows`・`windows-core`・`async-task` が 1 項ずつ）・道具の 4 つの呼び方の期待の出力・謝辞の著作権の 2 行、と各要件に通る／落ちるの判定が付いている。依存の置き換えと一覧の 3 つ化を同じコミットに入れる理由も、試し A・B（一覧から外すと包めない）で裏づけている。

## 判定

**GO**

- 理由: 既に在るアーキテクチャとの食い違いは無く、要件の全項目に部品と確かめ方がある。主な主張は実物と合い、危うさ（謝辞の崩れ・テストの揺れ・公開の途中の止まり）にはそれぞれ予備と手当てがある。2 件の指摘は人の手に残る操作の時期の決めで、設計の討議で決めれば足り、作りの直しは要らない。
- 次の段: 設計の討議（`/kiro-design-discussion wintf-winmsg-executor-import`）で指摘 1・2 を決め、`design.md` に写す。その後 `/kiro-spec-tasks wintf-winmsg-executor-import`。
