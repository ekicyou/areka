//! `shell_target`: シェルのフォルダから面の絵を組み立てる**読み込みの権威**。
//!
//! 本モジュールはまず「シェルのフォルダ直下のファイル名から、どの番号の面の画像がどれか」を
//! 決める純粋な判定 [`select_surface_images`] を持つ（R1.1〜R1.6・R1.8）。正典は
//! `surface<数字>.png` というファイル名の慣習だけで置かれた絵を、その番号の面の画像として
//! 認める——`surfaces.txt` に `element` 行が 1 本も無いシェル（里々・YAYA の標準テンプレート）は
//! この慣習にすべてを委ねている。
//!
//! 名前の判定そのものはバルーン側と**同じ 1 つの実装**（[`crate::balloon::face_digits_of`]・
//! 接頭辞を大小無視で外す → `.png` を外す → 残りが空でなく全部 ASCII 数字）を接頭辞
//! `surface` で呼ぶ。ゆえに「バルーンの `face_id_of` と同じ扱いにそろえる」（R1.4）は申し合わせ
//! ではなく構造で成り立つ。
//!
//! # 記録を出さないこと
//!
//! [`select_surface_images`] は fs にも記録にも触れない純粋な関数であり、重複（R1.5 の `warn!`）
//! と桁溢れ（R1.6 の `debug!`）は**事実として戻り値に載せるだけ**である。記録を出すのは
//! fs を触る入口（読み込み 1 回につき 1 度だけ出す）の責務で、本モジュールへ後から足される。

use std::collections::{BTreeMap, BTreeSet};

use crate::balloon::face_digits_of;

/// シェルの面画像の接頭辞（大小無視で比較される）。
const SURFACE_PREFIX: &str = "surface";

/// 名前の一覧から決まった「番号 → 採ったファイル名」と、その過程で捨てたもの。
///
/// 3 つの欄はどれも**入力の順に依存しない**（R1.8）。番号は数値の昇順、名前は辞書順である。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SurfaceImageSelection {
    /// 番号 → 採ったファイル名（**元の綴りのまま**——焼く側が実パスを開くため）。
    pub images: BTreeMap<u32, String>,
    /// 同じ番号に複数あったもの: `(番号, 採った名前, 捨てた名前の一覧)`。番号の昇順（R1.5）。
    pub duplicates: Vec<(u32, String, Vec<String>)>,
    /// 形は面の画像だが、数字が `u32` に収まらなかった名前。辞書順（R1.6）。
    pub overflow: Vec<String>,
}

/// 名前の一覧から「番号 → 採ったファイル名」を決める（fs に触らない純粋な関数）。
///
/// 判定と選択は次のとおり——
///
/// 1. **名前の判定**: [`face_digits_of`] に接頭辞 `surface` を与え、`surface{数字列}.png`
///    （接頭辞・拡張子は大小無視）の数字列を得る。`surfaces.txt`・`surface.png`・
///    `surface+0.png`・`surface0.pna` のように 3 段のどれかを満たさない名前は 0 件である
///    （R1.3）。同名の `.pna` を読まない（R2.6）のは、この拡張子の判定の帰結である。
/// 2. **番号の読み**: 数字列を 10 進数として読む。先頭の 0 は無視され、`surface0.png`・
///    `surface00.png`・`surface000.png`・`surface0000.png` はすべて面 0、`surface0010.png` は
///    面 10 になる（R1.2）。`u32` に収まらない数字列は画像と認めず
///    [`SurfaceImageSelection::overflow`] へ送る（R1.6）。
/// 3. **重複の裁き**: 同じ番号に複数の名前が解決したら**ファイル名の辞書順で最小**を採り、
///    残りを [`SurfaceImageSelection::duplicates`] に積む（R1.5・バルーンの `select_faces` と
///    同じ規則）。フォルダの走査順に結果が左右されないためである。
///
/// 入力はファイル名だけ——フォルダそのものやサブフォルダの中身を除くのは一覧を採る側の
/// 責務であり（R1.3）、本関数は渡された名前の列だけを見る。
pub fn select_surface_images<S: AsRef<str>>(names: &[S]) -> SurfaceImageSelection {
    // 番号 → 候補名の集合。集合を辞書順に保つことで、採用（最小）と不採用の並びが
    // 入力の順に依存しなくなる（R1.8）。
    let mut candidates: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let mut overflow: BTreeSet<String> = BTreeSet::new();

    for name in names {
        let name = name.as_ref();
        let Some(digits) = face_digits_of(SURFACE_PREFIX, name) else {
            continue;
        };
        // 数字列であることは判定済みゆえ、失敗は桁溢れだけである（R1.6）。
        match digits.parse::<u32>() {
            Ok(id) => {
                candidates.entry(id).or_default().insert(name.to_string());
            }
            Err(_) => {
                overflow.insert(name.to_string());
            }
        }
    }

    let mut images: BTreeMap<u32, String> = BTreeMap::new();
    let mut duplicates: Vec<(u32, String, Vec<String>)> = Vec::new();
    for (id, names) in candidates {
        let mut names = names.into_iter();
        // 候補の無い番号は作られないため、最初の 1 件が辞書順最小＝採用名である。
        let Some(adopted) = names.next() else {
            continue;
        };
        let dropped: Vec<String> = names.collect();
        if !dropped.is_empty() {
            duplicates.push((id, adopted.clone(), dropped));
        }
        images.insert(id, adopted);
    }

    SurfaceImageSelection {
        images,
        duplicates,
        overflow: overflow.into_iter().collect(),
    }
}

#[cfg(test)]
#[path = "shell_target_names_tests.rs"]
mod names_tests;
