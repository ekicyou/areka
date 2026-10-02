# Brief: areka-P0-balloon-font-file

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。普通のバルーンにも効く**正典の実装**。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- **ゴーストとバルーンの作者**: フォルダに同梱したフォントファイル（例: Zen Kurenaido）で台詞を書けない。正典はこれを認めている——バルーンの descript.txt `font.name` は「バルーンのフォルダに置いたフォントファイルも指定可能（SSPのみ）」、`\f[name,…]` は「ghost/master以下や現在のバルーンのフォルダに置いたフォントファイルも指定可能」（例 `\f[name,メイリオ,meiryo.ttf]`）。`number.font.name`・`sstpmessage.font.name`・`communicatebox.font.name` も同じ文言。
- シェル内バルーン（`shell-balloon`）の参考ゴースト「窓際のぱすたさん」は同梱フォントの縦書きが前提。

## Current State

- `crates/areka-emo-text/src/draw_catalog.rs` は `font.name` のカンマ区切りをシステムのフォント集から引く。**`.ttf/.otf/.ttc` は警告を出して意図的に読み飛ばす**（「読み込みは実装しない」）。`draw.rs` の `CreateTextFormat` はフォント集に `None` を渡す＝独自のフォント集・読み込み器はリポジトリに無い。`\f[name]` も同じ規則。
- 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `font.name` と `sakura-script.toml` の `\f[name]` の状態は着手時に確かめる。

## Desired Outcome

- `font.name` と `\f[name,…]` にフォントファイルを書くと、そのフォントで描かれる（カンマ区切りの優先順は正典どおり。ファイルとファミリ名の混在も可）。
- 探す場所: 普通のバルーンは**バルーンのフォルダ → ゴーストのフォルダ（ghost/master）**、シェル内バルーンは**シェルのフォルダ → ゴーストのフォルダ**（2026-10-01 開発者確定。探し場所の一覧は `shell-balloon` が渡す）。
- **縦書きでも同梱フォントのまま描く**。SSP は縦書きで同梱フォントを環境の標準ゴシックの縦書き用異体へ自動で差し替える（ukadoc `vertical` の項）が、これは GDI の `@` 付きフォントの制約によるもので、DirectWrite には無い＝areka は差し替えない（意図的な差。`doc/COMPAT_ARCHITECTURE.md` §8 に登記する）。
- 読めないファイル・壊れたファイルはログに理由を残し、優先順の次へ進む。

## Approach

- DirectWrite のフォントセット（`IDWriteFontSetBuilder` に `AddFontFile`→`CreateFontCollectionFromFontSet`）でフォルダのファイルをフォント集にし、システムのフォント集と合わせて `CreateTextFormat` へ渡す。新しい外部依存は要らない（`windows` crate の機能フラグの範囲）。
- フォント集はバルーン（とシェル内バルーンのシェル）ごとに作り、資産の差し替え（`shell-balloon-switch` の流れ）で作り直す。

## Scope

- **In**: `font.name`・`\f[name]` のファイル指定（バルーン・ゴースト・シェルの各フォルダ）、`number.font.name`・`sstpmessage.font.name`・`communicatebox.font.name` も同じ読み手を通す（表示そのものは各担当）、縦書きでの扱い、ログ、決定論テスト（試験用の小さなフリーのフォントファイルを検体にする＝ライセンスを `THIRD-PARTY-NOTICES.md` に書く）、網羅台帳の更新。
- **Out**: フォントのインストール（OS への登録）・Web フォントの取得・字形の合成。

## Boundary Candidates

- フォントの読み込みとフォント集の寿命（`draw_catalog.rs` の周り）と、`CreateTextFormat` への受け渡し（`draw.rs`）。

## Out of Boundary

- シェル内バルーンの探し場所の決定（`shell-balloon`）。
- 縦書きの字形の確かめ（`text-typesetting`）。

## Upstream / Downstream

- **Upstream**: α の完成宣言。`shell-balloon` とは並走できる（触るのは `draw_catalog.rs`・`draw.rs` の周り。`shell-balloon` が触る `actor.rs`・`region.rs` と別）。
- **Downstream**: `text-ruby`（ルビのフォント）・`balloon-markers`（`number.font.name`・`sstpmessage.font.name`）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `text-align-shadow-canon`・`emo-text-canon-residue`（同じ `areka-emo-text`）。

## Constraints

- `draw.rs` 761 行＝足す量によっては新しいファイルで足す。1 ファイル 1,000 行。
- 試験用フォントの同梱はライセンスの確認を要件で決める（OFL など再配布可のものに限る）。
