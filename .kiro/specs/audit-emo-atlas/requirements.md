# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **絵の焼き込みとアトラス**（`crates/areka-emo-atlas/` 全部）を受け持つ。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。

## Introduction

この文書は、絵の焼き込みとアトラスのクレート（`crates/areka-emo-atlas/`）に `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直すための要件を定める。要件の段では「全部直す」という形だけを書き、指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書に落として直す対象を確定する。

受け持つ範囲の大きさ（着手のときの測り直し・`main` `4863cb66`）:

| 測った時点 | ファイル | うち Rust のファイル | コード | テスト |
|---|---|---|---|---|
| 起票のとき（`e3c2cbe4`・brief の値） | 46 | — | 5,510 行 | 3,125 行 |
| 棚卸㊱の再測定（`069f1a48`・brief の値） | — | 41 | 11,036 行（コードとテストの合計） | — |
| 着手のとき（`4863cb66`） | 63 | 41 | 7,439 行 | 3,597 行 |

テストに数えたのは、名前が `_tests.rs` で終わるファイル・`tests/` の下のファイル・名前が `_test_support.rs` で終わるファイル。コードには `examples/` の下の道具を含む。ファイルの数には `Cargo.toml` と `src/testdata/` の下の 21 ファイル（画像の検体・説明・正解の写し）を含む。範囲の中は棚卸㊱の再測定（`069f1a48`）から変わっていない。1,000 行に近いのは `src/lib.rs`（873 行）と `src/animated_tests.rs`（838 行）。

## Boundary Context

- **In scope**:
  - `crates/areka-emo-atlas/` の全ファイル（ソース・テスト・`examples/` の道具・`src/testdata/` の検体・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 5 と要件 6 の約束に従う。C19 のあいだは要件 6 で触れない所がある）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。
  - 新しい機能。
  - `vendors/` の下（`examples/` の道具が書き出す `vendors/sample_ghost/` の検体を含む）。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 3）。
  - `audit-wintf-runtime` から申し送られた 3 件（15・125・130）。直し済みなので、この spec では数え直さない（要件 1.6）。
- **Adjacent expectations**:
  - このクレートを使う範囲の外のクレートは、着手のときに `crates/areka`・`crates/areka-emo-compose`・`crates/areka-emo-present`・`crates/areka-emo-text`・`crates/areka-placement`・`crates/pilot`・`crates/surface-compare` の 7 つ。
  - 本体 `crates/areka/` の使い手（本番は `emo2_boot/` の `assets.rs`・`balloon_background.rs`・`shell_box_assets.rs`・`switch_assets.rs`・`frame/switch.rs` と `offline_dump/` の `mod.rs`・`render.rs`、ほかにテスト約 20 本）は、同じ C19 の `runtime-module-untangle` が移す所。`crates/areka-placement/` は C20 の `audit-placement-core` の範囲。この spec はアトラスの公開の形を足すことのほかは変えず、これらを書き換えない（要件 6）。
  - C19 で並ぶほかの 7 本とは、アトラスの公開の形を変えない限り、触るファイルが重ならない（棚卸㊱の測り）。
  - 範囲の中に、1 ファイル 1,000 行の番人の例外表（`crates/log-capture-kit/tests/file_length_guard_test.rs`・範囲の外）に載るファイルは、着手のときに 0 本。
  - 範囲の中の `src/webp_pin_tests.rs` は、範囲の外の根の `Cargo.toml`・`Cargo.lock`・`deny.toml` を読んで `image-webp` の取り込みを確かめる。この spec はその 3 つを変えない（要件 6.4）。

## Requirements

### Requirement 1: 指摘の洗い出し

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行う, the 本見直し shall `crates/areka-emo-atlas/` の全ファイルだけを対象にして `/ponytail:ponytail-audit` をかける
2. The 本見直し shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The 本見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイルを全部読んで、読まなかった所を残さない
4. The 本見直し shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. When 要件の討議で直す対象が確定した, the 本見直し shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか）をこの文書に書き加える
6. The 本見直し shall `audit-wintf-runtime` から申し送られた直し済みの 3 件（15＝WIC の α の判定・125＝名の渡し方・130＝`with_arm` の初期化の失敗）を、指摘の一覧に数え直さない

### Requirement 2: 指摘を全部直す

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that 絵の焼き込みとアトラスのコードからバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本見直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本見直し shall 要件 3 の 3 種類のどれかに当たって開発者が残すと裁定した指摘と、要件 6.3 で開発者が C20 へ回すと裁定した指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出た, the 本見直し shall その箇所を使うテスト・検体・呼び出し元まで直し切り、直しの途中の状態を残さない

### Requirement 3: 直さずに残す指摘は開発者の裁定で決める

**Objective:** As a 開発者, I want 直さずに残す指摘を自分で裁定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. If 指摘の直しが次の 3 種類のどれかに当たる, the 本見直し shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. When 3 種類目に当たるかを判断する, the 本見直し shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする
3. If 指摘の直しが 3 種類のどれにも当たらない, the 本見直し shall その指摘を残すことを議題にせず直す
4. When 開発者が指摘を残すと裁定した, the 本見直し shall 裁定とその理由をこの文書に残し、要件 4 の「新しい指摘 0 件」の数から外す

### Requirement 4: 直した後の確かめ

**Objective:** As a 開発者, I want 直しが新しい問題を生んでいないことを確かめられること, so that 見直しが終わったと安心して言える

#### Acceptance Criteria

1. When 確定した指摘を全部直し終えた, the 本見直し shall 同じ範囲に、要件 1 と同じ作法（件数の上限なし・全部読む）で `/ponytail:ponytail-audit` をもう一度かける
2. If もう一度かけた audit が新しい指摘（直しが生んだ指摘を含む）を出した, the 本見直し shall その指摘も直し、新しい指摘が 0 件になるまで audit をかけ直す
3. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無い, the 本見直し shall 開発者に一言知らせたうえで、5 回目の指摘を直したところでかけ直しを打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書く（2026-11-06 `audit-seriko` で開発者が承認した形）
4. The 本見直し shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を検証の記録に残す
5. When 見直しを終える, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる
6. The 本見直し shall 直したバグと危ない所の 1 件ずつに、その直しが壊れたら落ちるテストを 1 本以上置く
7. The 本見直し shall 足したテストと直したテストを、毎回同じ結果になる（決定論の）テストにする

### Requirement 5: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが安全に、並んで走るほかの spec とぶつからずに入ること, so that ほかのクレートが壊れず、並行作業が止まらない

#### Acceptance Criteria

1. When 直しで関数や型の形を変えた, the 本見直し shall ツリー全体で呼び出し元を探して、全部直す
2. If 直しが及ぶ呼び出し元の範囲を受け持つ spec のセッションが走っている, the 本見直し shall そのセッションと申し合わせてから、その呼び出し元に触る
3. When コード・テスト・検体を消す, the 本見直し shall 消す前に、ツリー全体（テスト・検体・設定・文字列での参照を含む）で使われていないことを確かめる
4. The 本見直し shall 範囲の外のファイルの、直しに要らない箇所を直さない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）

### Requirement 6: C19 の並走の約束

**Objective:** As a 開発者, I want 見直しの直しが同じウェーブ C19 のほかの spec の邪魔をしないこと, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While C19 のあいだ, the 本見直し shall 本体 `crates/areka/` のファイルに手を入れない（C19 の約束 ⑴）
2. While C19 のあいだ, the 本見直し shall アトラスの公開の形（範囲の外から呼べる型・関数・欄の名前と形）を、足すことのほかは変えない（C19 の約束 ⑴）
3. If 確定した指摘がアトラスの公開の形を変えずには直せない, the 本見直し shall その指摘だけを直さずに議題にし、「`runtime-module-untangle` のセッションと申し合わせて C19 のうちに直す」か「C20 へ回す」かを開発者の裁定で決める（brief の 2026-11-13 棚卸㊱の再測定）
4. The 本見直し shall 依存を足さず、`Cargo.lock` を変えない（C19 の約束 ⑷＝`Cargo.lock` の席は `dola-yaml-backend`）
5. If 直しに新しい依存が要ると分かった, the 本見直し shall 依存を足さずに開発者へ議題として出す（足すなら `tech.md` への登記と開発者の承認が要る）
6. The 本見直し shall `crates/areka-emo-text/` に新しいファイルを足さない（C19 の約束 ⑷＝その席は `emo-text-reveal-incremental`）
7. The 本見直し shall `vendors/` の下に手を入れない

### Requirement 7: 守る決まり

**Objective:** As a 開発者, I want 見直しの直しがプロジェクトの決まりを守ること, so that 直しが別の決まりを破らない

#### Acceptance Criteria

1. The 本見直し shall 1 ファイル 1,000 行の番人を赤にせず、直しで行が上限を越えるときは新しいファイルへ分け、番人の例外表へ新しいファイルを足さない
2. The 本見直し shall 記録の無い失敗の道を作らず、失敗は記録（`error!`）を残したうえで呼び出し元へ失敗（`Err`）として返す
3. The 本見直し shall 終了の道を正規の実装にし、終了を早めるための小細工を入れない
4. The 本見直し shall 実機の根と一時フォルダを、ワークツリーの `target` の下に置く
5. The 本見直し shall GPU のテストを実 GPU で回す
6. If 文字化けが起きた, the 本見直し shall コンソールの文字コードを変えて逃げず、読む側を直す

### Requirement 8: spec の大きさ

**Objective:** As a 開発者, I want 1 本の spec が大きくなりすぎないこと, so that 1 本ずつ確かめて着地できる

#### Acceptance Criteria

1. If 確定した指摘の直しが 20 タスクを越えそうになった, the 本見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切ることを議題にする
