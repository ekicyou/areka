//! 意味層（Decode）— 構文トークン → 値正規化済み `Shell`（emo2 subset 限定）。
//!
//! Lexer（`lexer::lex`）が産んだ構文トークン列 `Vec<Token>` を消費し、シェルサーフェス
//! モデル `Shell` を組み立てる（依存方向 `model ← lexer ← decode`）。下流が再パース
//! 不要なのは、この層が値を decode しきるため（要件 1.2）。sakura 同様に `Result` を
//! 返さず・パニックせず・空入力で空を返す寛容決定的パスを採る（要件 2.x）。
//!
//! 本ファイル（タスク 4.1）が担うのは **decode スケルトンの枠組みのみ**:
//! - トップレベルのトークン走査と、ブロックヘッダによる**ブロックディスパッチ**
//!   （設計 System Flows「iterate top level tokens → 各ブロック種別」）。
//! - charset 行の寛容スキップ（要件 3.1）・descript ブロックの寛容スキップ（要件 3.2）。
//!   いずれもモデルに保持しない（`Shell` に charset/descript フィールドは存在しない）。
//! - `surfaceNNN` ブロックから surface ID を取り出し、（当面空の）構成要素枠を持つ
//!   `Surface` を積む（要件 4.1）。ID 抽出失敗は `unwrap_or(0)` で既定 0 に倒す（要件 3.3）。
//! - 空入力・ヘッダのみ・ヘッダ欠落・未知ブロック・未知行・`Raw` を寛容吸収し
//!   走査を中断しない（要件 3.3/9.2）。
//!
//! **ディスパッチ優先順位（重要）**: `kero.surface.alias` と `surface.append10` は
//! いずれも部分文字列 "surface" を含むため、素朴な `surfaceNNN` 判定より**先**に
//! これらを分類する。優先順位は
//! `descript` → `kero.surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
//!
//! **スコープ境界（タスク 4.2〜4.6 のシーム）**: 本ファイルは surface の**枠**まで。
//! - element overlay ＋ collision（レイヤ昇順・矩形）→ タスク 4.2 が
//!   `decode_surface_body` 内へ実装する。
//! - animationN 集約（interval＋pattern 群）→ タスク 4.3 が同 body 内へ実装する。
//! - `surface.append*` ターゲット捕捉＋追記 collision/animation → タスク 4.4 が
//!   `decode_append_block` を実装し `Shell.appends` を充填する。
//! - `kero.surface.alias` 写像 → タスク 4.5 が `decode_alias_block` を実装し
//!   `Shell.aliases` を充填する。
//! - subset 外・不正の寛容吸収の最終化 → タスク 4.6。
//! 現段階ではこれらのシーム関数は枠のみ（空を返す/何もしない）に留める。

use super::lexer::Token;
use super::model::{Animation, Collision, Element, Shell, Surface};

/// 構文トークン列を値正規化済みの `Shell` へ写像する（mod 内・`parse` が結線する）。
///
/// - 入力 `tokens` は lexer 出力（構文区切り済み）。
/// - トップレベルを走査し、ブロックヘッダでディスパッチする。charset/descript は
///   読み飛ばし保持しない（要件 3.1/3.2）。`surfaceNNN` は枠を積む（要件 4.1）。
/// - 失敗しない（`Vec`/`Shell` を返す・`Result` でない・要件 2.x）。空入力で空 `Shell`。
/// - 同一入力で同一出力（決定的）。
pub(crate) fn decode(tokens: Vec<Token>) -> Shell {
    let mut shell = Shell {
        surfaces: Vec::new(),
        appends: Vec::new(),
        aliases: Vec::new(),
    };

    for token in tokens {
        match token {
            Token::BlockStart(header) => {
                // ブロック本体を読み飛ばした（lexer が BlockStart..BlockEnd の間へ
                // Line トークンを挟むが、本タスクの枠組みでは本体を関数へ渡さず
                // ディスパッチのみ行う。本体行の消費は各シーム関数の領分）。
                // ※ 本体トークンは後続の for ループで BlockEnd まで素通り吸収される。
                dispatch_block(&mut shell, &header);
            }
            // ブロック本体行・ブロック終端・トップレベル行・不正断片は本タスクでは
            // 個別処理しない。charset 行の非保持（要件 3.1）を含め、いずれも寛容に
            // 読み飛ばす（要件 3.3/9.2）。値化はタスク 4.2〜4.6 の領分。
            Token::Line(_) | Token::BlockEnd | Token::TopLevel(_) | Token::Raw(_) => {}
        }
    }

    shell
}

/// ブロックヘッダ（`BlockStart` の CSV フィールド列）でブロック種別を判定し振り分ける。
///
/// 優先順位は `descript` → `kero.surface.alias` → `surface.append*` → `surfaceNNN` → 未知。
/// alias/append は "surface" を含むため plain `surfaceNNN` より先に判定する（誤分類回避）。
fn dispatch_block(shell: &mut Shell, header: &[String]) {
    let head = header.first().map(String::as_str).unwrap_or("");

    if head == "descript" {
        // descript ブロックは寛容スキップ・非保持（要件 3.2）。本体は for 側で素通り。
        return;
    }
    if head == "kero.surface.alias" {
        // alias 写像はタスク 4.5 の領分。現段階では吸収のみ（何も積まない）。
        decode_alias_block(shell, header);
        return;
    }
    if head.starts_with("surface.append") {
        // append 追記定義はタスク 4.4 の領分。現段階では吸収のみ（何も積まない）。
        decode_append_block(shell, header);
        return;
    }
    if let Some(rest) = head.strip_prefix("surface") {
        // `surfaceNNN` ブロック（要件 4.1）。NNN を取り出し、空枠の Surface を積む。
        // 非数値・欠落は `unwrap_or(0)` で既定 0 に倒す（要件 3.3・パニックしない）。
        let id = rest.parse::<u32>().unwrap_or(0);
        let (elements, collisions, animations) = decode_surface_body();
        shell.surfaces.push(Surface {
            id,
            elements,
            collisions,
            animations,
        });
        return;
    }

    // 未知ブロック（未対応ヘッダ）は寛容に吸収する（要件 9.2）。何も積まない。
}

/// surface ブロック本体（element/collision/animation 行）を対応型へ decode する枠。
///
/// タスク 4.1 の枠組みでは本体行を消費せず、常に空の 3 種を返す。
/// - タスク 4.2 が element overlay ＋ collision（レイヤ昇順・矩形）を充填する。
/// - タスク 4.3 が animationN 集約（interval＋pattern 群）を充填する。
/// これらはこの関数へ本体トークン（`Vec<Token>` のうち BlockStart..BlockEnd の Line 群）を
/// 引数で受け取る形へ拡張される想定のシームである。
fn decode_surface_body() -> (Vec<Element>, Vec<Collision>, Vec<Animation>) {
    (Vec::new(), Vec::new(), Vec::new())
}

/// `surface.append*` 追記ブロックを `SurfaceAppend` へ decode する枠（タスク 4.4）。
///
/// タスク 4.1 では何も積まない（吸収のみ）。タスク 4.4 がヘッダのターゲット指定捕捉
/// （`parse_targets`・展開しない）と、本体の collision/animation 充填を実装し、
/// `shell.appends` へ push する。
fn decode_append_block(_shell: &mut Shell, _header: &[String]) {
    // タスク 4.4 で実装。現段階は寛容吸収（要件 9.2）。
}

/// `kero.surface.alias` ブロックを `SurfaceAlias` 群へ decode する枠（タスク 4.5）。
///
/// タスク 4.1 では何も積まない（吸収のみ）。タスク 4.5 が各 `KEY,[id,...]` 行を
/// opaque キー＋順序付き数値 ID リストへ写像し（重複キー保持）、`shell.aliases` へ
/// push する。
fn decode_alias_block(_shell: &mut Shell, _header: &[String]) {
    // タスク 4.5 で実装。現段階は寛容吸収（要件 9.2）。
}
