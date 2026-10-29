//! # draw_catalog — フォント候補列の解決（COM 層・draw ファサードの子）
//!
//! [`FontCatalog`] は「記述順に並んだフォント名の候補列」から、この機械に実際に
//! 入っている顔（[`FontFace`]＝家族名・太さ・幅・斜体）を 1 つ選ぶ（要件 9.1／9.2）。
//! 選んだ結果は候補列ごとに覚えるので、同じ列を文字ごとに引き直さない（要件 9.7／3.11）。
//!
//! - 各候補は家族名として引き、無ければ GDI の名前（例「游ゴシック Medium」）として
//!   引き直す（emo-text-dwrite-fidelity 要件 3.1〜3.5）。家族名で当たれば Regular の顔、
//!   GDI の名前で当たればその名前が指す家族・太さ・幅・斜体。切り替えの設定も新しい
//!   名前の書き方も持たない（要件 3.8／3.9）。number ほかの表示もこの台帳を通せば同じ
//!   引き直しが効く（要件 4.7・共有の唯一の入口）。
//! - 候補がフォントファイル名（`.ttf`／`.otf`／`.ttc`・大小無視）のときは読み込まず、
//!   記録を 1 度だけ残して次の候補へ進む（要件 9.3）。
//! - 候補が全滅したときは、試した候補を添えた記録を 1 度だけ残して既定のフォントへ
//!   戻す（要件 9.4／13.2）。
//! - バルーン定義の候補列（`font.name` のカンマ区切り）にも同じ規則を当てる
//!   （[`FontCatalog::pick`]・要件 9.8）。
//!
//! **層規律**: COM 層——UI スレッド専有。失敗は log-first（`error!`／`warn!`）で扱い
//! panic しない。
//!
//! ## 記録の重複判定の鍵
//!
//! 記録済みかどうかは [`WarnKey`]——**候補列そのものを保った**鍵——で判定する。
//! 台本作者の書いた文字列を区切り文字で 1 本の綴りへ潰すと、別々の失敗が同じ鍵に
//! なって片方が無記録で消える（要件 13.5 違反）。引用符はカンマを守るので
//! `\f[name,a,b]` と `\f[name,"a,b"]` は別の候補列として到達する。

use std::cell::{Cell, RefCell};
#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::{BTreeSet, HashMap};

use tracing::{debug, warn};
use windows::Win32::Foundation::DWRITE_E_NOFONT;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FONT_STRETCH, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE, DWRITE_FONT_STYLE_ITALIC,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_FONT_WEIGHT_NORMAL, IDWriteFactory, IDWriteFactory2, IDWriteFont, IDWriteFontCollection,
    IDWriteGdiInterop,
};
use windows::Win32::Graphics::Gdi::{DEFAULT_CHARSET, LOGFONTW};
use windows::core::{BOOL, HSTRING, Interface};

use super::{DEFAULT_FONT_NAME, ResolvedFont, device_err};
use crate::TextLayerError;

/// フォントファイル名と見なす拡張子（大小無視・要件 9.3）。読み込みは実装しない。
const FONT_FILE_EXTENSIONS: [&str; 3] = [".ttf", ".otf", ".ttc"];

/// GDI の名前になりうる最大の長さ（`LOGFONTW::lfFaceName` の 32 単位から終端を除いた数）。
const GDI_FACE_NAME_MAX_UNITS: usize = 31;

/// 名前が指す顔（家族名・太さ・幅・斜体を DirectWrite の型のまま持つ値）。
#[derive(Clone, Debug, PartialEq)]
pub struct FontFace {
    /// DirectWrite の家族名（`CreateTextFormat`／`SetFontFamilyName` へそのまま渡せる名前）。
    pub family: String,
    pub weight: DWRITE_FONT_WEIGHT,
    pub stretch: DWRITE_FONT_STRETCH,
    pub style: DWRITE_FONT_STYLE,
}

impl FontFace {
    /// 家族名で当たった候補の顔（太さ・幅・斜体とも NORMAL・要件 3.3）。
    pub fn regular(family: &str) -> FontFace {
        FontFace {
            family: family.to_owned(),
            weight: DWRITE_FONT_WEIGHT_NORMAL,
            stretch: DWRITE_FONT_STRETCH_NORMAL,
            style: DWRITE_FONT_STYLE_NORMAL,
        }
    }

    /// 太字の指定を重ねた太さ。真なら名前の太さと BOLD の大きい方、偽なら名前の太さ（要件 4.2・4.3）。
    pub fn weight_with(&self, bold: bool) -> DWRITE_FONT_WEIGHT {
        if bold {
            // 太字が名前の太さより軽くならないよう大きい方（Black＋太字は 900 のまま）。
            DWRITE_FONT_WEIGHT(self.weight.0.max(DWRITE_FONT_WEIGHT_BOLD.0))
        } else {
            self.weight
        }
    }

    /// 斜体の指定を重ねた斜体。真なら ITALIC、偽なら名前の斜体（要件 4.4・4.5）。
    pub fn style_with(&self, italic: bool) -> DWRITE_FONT_STYLE {
        if italic {
            DWRITE_FONT_STYLE_ITALIC
        } else {
            self.style
        }
    }
}

/// 記録済み判定の鍵——**連結した綴りにしない**（モジュール doc「記録の重複判定の鍵」）。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum WarnKey {
    /// 読み飛ばしたフォントファイル名の候補 1 つ。
    FontFile(String),
    /// 全滅した候補列（列のまま保つ）。
    AllMissing(Vec<String>),
}

/// 実在するフォントの問い合わせ先。
enum FamilySource {
    /// システムのフォント集合と GDI 互換の口（本番経路・生成時に 1 度だけ取得する）。
    System {
        collection: IDWriteFontCollection,
        gdi: IDWriteGdiInterop,
    },
    /// テスト用の偽の集合（家族名と GDI の名前の鍵はどちらも小文字・本番には存在しない）。
    #[cfg(test)]
    Fake {
        families: BTreeSet<String>,
        gdi: BTreeMap<String, FontFace>,
    },
}

/// 候補列→実在する顔の解決・記憶・記録の 1 度化（要件 9.1〜9.4／9.7／9.8／13.2・3.1〜3.11）。
pub struct FontCatalog {
    families: FamilySource,
    /// 候補列→解決した顔（`None`＝全滅）。鍵は候補列そのもの。
    memo: RefCell<HashMap<Vec<String>, Option<FontFace>>>,
    /// 記録済みの鍵（[`WarnKey`]）。
    ///
    /// **記録の周期は「台帳の寿命のあいだ 1 度」**（タスク 9.4 の裁定・6.2 の申し送り）。
    /// 要件 9.3／9.4 は「1 台詞につき 1 度」と書くが、台帳は台詞の切れ目を知らず
    /// （承認済み design の型定義にも空にする口が無い）、実際には台帳が生きているあいだ
    /// 1 度になる。**記録は要件より少ない側へ寄る**ので「記録があふれない」（要件 13.4）の
    /// 目的は満たし、記録なしの失敗経路（要件 13.5）も作らない。要件文言と設計の間の
    /// 既存のズレであり、本仕様の実装が持ち込んだものではない。周期を台詞に合わせるなら
    /// 台詞の切れ目を台帳へ届ける配線が要る（純粋層 `state_decoration.rs` の `Decoration` 側の
    /// `warned` は `ClearAll` で空になる別の集合）。
    warned: RefCell<BTreeSet<WarnKey>>,
    /// 記憶に無くて実際に問い合わせた回数（要件 9.7 の観測点）。
    lookups: Cell<usize>,
}

impl FontCatalog {
    /// システムのフォント集合と GDI 互換の口を 1 度だけ取得して台帳を作る（失敗は `error!`＋`Err`）。
    pub fn new(factory: &IDWriteFactory2) -> Result<FontCatalog, TextLayerError> {
        let base: IDWriteFactory = factory
            .cast()
            .map_err(device_err("QueryInterface(IDWriteFactory)"))?;
        let mut system: Option<IDWriteFontCollection> = None;
        unsafe { base.GetSystemFontCollection(&mut system, false) }
            .map_err(device_err("GetSystemFontCollection"))?;
        let collection = system.ok_or_else(|| {
            tracing::error!("GetSystemFontCollection が成功しながら集合を返さなかった");
            TextLayerError::Device {
                hresult: 0,
                context: "GetSystemFontCollection",
            }
        })?;
        let gdi = unsafe { base.GetGdiInterop() }.map_err(device_err("GetGdiInterop"))?;
        Ok(FontCatalog::with_source(FamilySource::System {
            collection,
            gdi,
        }))
    }

    fn with_source(families: FamilySource) -> FontCatalog {
        FontCatalog {
            families,
            memo: RefCell::new(HashMap::new()),
            warned: RefCell::new(BTreeSet::new()),
            lookups: Cell::new(0),
        }
    }

    /// テスト用——実機のフォント構成に依存しない偽の集合（家族名だけ）で台帳を作る。
    #[cfg(test)]
    pub(super) fn with_families(installed: &[&str]) -> FontCatalog {
        FontCatalog::with_faces(installed, &[])
    }

    /// テスト用——家族名の集合と「GDI の名前 → 顔」の表で偽の台帳を作る（要件 6.2）。
    #[cfg(test)]
    pub(super) fn with_faces(families: &[&str], gdi: &[(&str, FontFace)]) -> FontCatalog {
        FontCatalog::with_source(FamilySource::Fake {
            families: families.iter().map(|n| n.to_lowercase()).collect(),
            gdi: gdi
                .iter()
                .map(|(name, face)| (name.to_lowercase(), face.clone()))
                .collect(),
        })
    }

    /// 記憶に無くて実際に問い合わせた回数（要件 9.7 の観測点）。
    #[cfg(test)]
    pub(super) fn lookup_count(&self) -> usize {
        self.lookups.get()
    }

    /// 記述順に問い合わせて最初に入っている顔。空列は `None`（記録なし）。
    /// 全滅は `warn!` 1 度＋`None`（要件 3.4／3.6）。
    pub fn face_for(&self, candidates: &[String]) -> Option<FontFace> {
        if candidates.is_empty() {
            return None;
        }
        if let Some(hit) = self.memo.borrow().get(candidates).cloned() {
            return hit;
        }
        self.lookups.set(self.lookups.get() + 1);

        let mut resolved = None;
        for candidate in candidates {
            if is_font_file(candidate) {
                // 読み込みは実装しない（要件 9.3）——記録を 1 度残して次の候補へ。
                if self.first_time(WarnKey::FontFile(candidate.clone())) {
                    warn!(
                        candidate = %candidate,
                        "フォントファイル名の候補は読み込まない——次の候補へ進む"
                    );
                }
                continue;
            }
            // 家族名 → GDI の名前の順（GDI の名前は家族名にも当たるので順を守る・要件 3.4）。
            resolved = if self.has_family(candidate) {
                Some(FontFace::regular(candidate))
            } else {
                self.gdi_face(candidate)
            };
            if resolved.is_some() {
                break;
            }
        }

        if resolved.is_none() && self.first_time(WarnKey::AllMissing(candidates.to_vec())) {
            warn!(
                candidates = %candidates.join(", "),
                "フォントの候補が 1 つも見つからない——既定のフォントへ戻す"
            );
        }
        self.memo
            .borrow_mut()
            .insert(candidates.to_vec(), resolved.clone());
        resolved
    }

    /// [`FontCatalog::face_for`] の家族名だけ（「入っているか」の問いと名前だけの呼び手の口）。
    pub fn family_for(&self, candidates: &[String]) -> Option<String> {
        self.face_for(candidates).map(|face| face.family)
    }

    /// [`FontCatalog::face_for`]、全滅なら [`DEFAULT_FONT_NAME`] の Regular の顔。
    pub fn face_or_default(&self, candidates: &[String]) -> FontFace {
        self.face_for(candidates)
            .unwrap_or_else(|| FontFace::regular(DEFAULT_FONT_NAME))
    }

    /// [`FontCatalog::face_for`]、全滅なら `fallback`（束縛の顔・要件 4.10）。
    pub fn face_or(&self, candidates: &[String], fallback: &FontFace) -> FontFace {
        self.face_for(candidates)
            .unwrap_or_else(|| fallback.clone())
    }

    /// バルーン定義の候補列（`[name] ++ fallback_chain`）を解決した名前を `name` に
    /// 据えた複製を返す（要件 9.8）。全滅は [`DEFAULT_FONT_NAME`]。
    ///
    /// 既存の書式生成関数（`create_text_format`／`try_create_format`）は**この複製で**
    /// 呼ぶ——解決は書式生成の手前で終わっているので、生成関数の本文は変わらない。
    pub fn pick(&self, font: &ResolvedFont) -> ResolvedFont {
        let mut picked = font.clone();
        picked.name = self
            .family_for(&font.looks.default.name)
            .unwrap_or_else(|| DEFAULT_FONT_NAME.to_owned());
        picked
    }

    /// この鍵での記録がまだ無ければ記録済みにして `true`（記録の 1 度化・要件 9.3／9.4）。
    fn first_time(&self, key: WarnKey) -> bool {
        self.warned.borrow_mut().insert(key)
    }

    /// この機械にその名前のフォントが入っているか（家族名として）。
    fn has_family(&self, name: &str) -> bool {
        match &self.families {
            FamilySource::System { collection, .. } => {
                let mut index = 0u32;
                let mut exists = BOOL(0);
                match unsafe {
                    collection.FindFamilyName(&HSTRING::from(name), &mut index, &mut exists)
                } {
                    Ok(()) => exists.as_bool(),
                    Err(e) => {
                        // log-first: 失敗は記録して「無い」扱いにする（次の候補へ進む）。
                        let _ = device_err("FindFamilyName")(e);
                        false
                    }
                }
            }
            #[cfg(test)]
            FamilySource::Fake { families, .. } => families.contains(&name.to_lowercase()),
        }
    }

    /// GDI の名前としてその名前が指す顔（要件 3.1／3.2／3.5）。見つからない名前は `None`
    /// （記録しない）・想定外の失敗は `error!` を残して `None`（次の候補へ進む）。
    fn gdi_face(&self, name: &str) -> Option<FontFace> {
        let units: Vec<u16> = name.encode_utf16().collect();
        if units.len() > GDI_FACE_NAME_MAX_UNITS {
            debug!(candidate = %name, "31 単位を超える候補は GDI の名前になりえない——引かない");
            return None;
        }
        match &self.families {
            FamilySource::System { gdi, .. } => {
                let mut logfont = LOGFONTW {
                    lfWeight: 400,
                    lfCharSet: DEFAULT_CHARSET,
                    ..Default::default()
                };
                logfont.lfFaceName[..units.len()].copy_from_slice(&units);
                match unsafe { gdi.CreateFontFromLOGFONT(&logfont) } {
                    Ok(font) => face_of(&font),
                    // 見つからない名前は失敗で返る（置き換えは起きない）——失敗ではなく「無い」。
                    Err(e) if e.code() == DWRITE_E_NOFONT => None,
                    Err(e) => {
                        // log-first: 想定外の失敗は記録して次の候補へ進む。
                        let _ = device_err("CreateFontFromLOGFONT")(e);
                        None
                    }
                }
            }
            #[cfg(test)]
            FamilySource::Fake { gdi, .. } => gdi.get(&name.to_lowercase()).cloned(),
        }
    }
}

/// GDI で引いた書体の顔（家族名は `GetFamilyNames` の先頭）。失敗は `error!` を残して `None`。
fn face_of(font: &IDWriteFont) -> Option<FontFace> {
    let family = unsafe {
        font.GetFontFamily()
            .and_then(|family| family.GetFamilyNames())
            .and_then(|names| {
                let mut buffer = vec![0u16; names.GetStringLength(0)? as usize + 1];
                names.GetString(0, &mut buffer)?;
                let len = buffer.iter().position(|&u| u == 0).unwrap_or(buffer.len());
                Ok(String::from_utf16_lossy(&buffer[..len]))
            })
    }
    .map_err(device_err("GetFamilyNames"))
    .ok()?;
    Some(FontFace {
        family,
        weight: unsafe { font.GetWeight() },
        stretch: unsafe { font.GetStretch() },
        style: unsafe { font.GetStyle() },
    })
}

/// 候補がフォントファイル名か（大小無視・要件 9.3）。
fn is_font_file(candidate: &str) -> bool {
    let lower = candidate.to_ascii_lowercase();
    FONT_FILE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
#[path = "draw_catalog_tests.rs"]
mod tests;
