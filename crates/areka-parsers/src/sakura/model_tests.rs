//! 命令モデル型（`model`）の単体テスト。
//!
//! 本モジュールは `model` とは別モジュールであり、公開パス
//! `crate::sakura::{Instruction, SurfaceArg, NewLineRatio, Choice, MoveArgs}`
//! 経由で型へアクセスする。これにより「別クレートの下流 engine が
//! 公開面のみで各 variant を構築・比較し、不透明値をアクセサで読める」
//! という I/O 契約（done 基準）を、別モジュール視点で固定する。

#![cfg(test)]

use crate::sakura::{Choice, Instruction, MoveArgs, NewLineRatio, SurfaceArg};
use std::time::Duration;

#[test]
fn construct_text_variant() {
    let i = Instruction::Text("こんばんわー！".to_string());
    assert_eq!(i, Instruction::Text("こんばんわー！".to_string()));
}

#[test]
fn construct_speaker_scope_variant() {
    let i = Instruction::SpeakerScope { n: 1 };
    assert_eq!(i, Instruction::SpeakerScope { n: 1 });
    assert_ne!(i, Instruction::SpeakerScope { n: 0 });
}

#[test]
fn construct_surface_variant_and_read_opaque_value() {
    // \s[...] の中身は不透明文字列のまま保持（要件 2.2/2.3）。
    let arg = SurfaceArg::new("通常".to_string());
    let i = Instruction::Surface(arg.clone());
    assert_eq!(i, Instruction::Surface(SurfaceArg::new("通常".to_string())));
    // 不透明 NewType は read-only アクセサで中身を読める（別クレート視点）。
    assert_eq!(arg.as_str(), "通常");
}

#[test]
fn construct_wait_variant() {
    let i = Instruction::Wait(Duration::from_millis(450));
    assert_eq!(i, Instruction::Wait(Duration::from_millis(450)));
    assert_ne!(i, Instruction::Wait(Duration::from_millis(500)));
}

#[test]
fn construct_newline_variant_and_read_ratio() {
    let r = NewLineRatio::new(1.5);
    let i = Instruction::NewLine(r.clone());
    assert_eq!(i, Instruction::NewLine(NewLineRatio::new(1.5)));
    // 比率は read-only アクセサで読める。
    assert_eq!(r.ratio(), 1.5_f32);
}

#[test]
fn construct_choice_variant_with_references() {
    let c = Choice {
        disp: "はい".to_string(),
        target: "OnYes".to_string(),
        references: vec!["r0".to_string(), "r1".to_string()],
    };
    let i = Instruction::Choice(c.clone());
    assert_eq!(i, Instruction::Choice(c));
    // references 順序保持（要件 5.2）。
    let c2 = Choice {
        disp: "はい".to_string(),
        target: "OnYes".to_string(),
        references: vec!["r1".to_string(), "r0".to_string()],
    };
    assert_ne!(Instruction::Choice(c2.clone()), {
        Instruction::Choice(Choice {
            disp: "はい".to_string(),
            target: "OnYes".to_string(),
            references: vec!["r0".to_string(), "r1".to_string()],
        })
    });
    let _ = c2;
}

#[test]
fn construct_cursor_variant() {
    let i = Instruction::Cursor {
        x: "10".to_string(),
        y: "20".to_string(),
    };
    assert_eq!(
        i,
        Instruction::Cursor {
            x: "10".to_string(),
            y: "20".to_string()
        }
    );
}

#[test]
fn construct_control_variants() {
    assert_eq!(Instruction::End, Instruction::End);
    assert_eq!(Instruction::Clear, Instruction::Clear);
    assert_eq!(Instruction::Quit, Instruction::Quit);
    assert_ne!(Instruction::End, Instruction::Clear);
}

#[test]
fn construct_move_variant() {
    let m = MoveArgs {
        args: vec!["10".to_string(), "20".to_string()],
    };
    let i = Instruction::Move(m.clone());
    assert_eq!(i, Instruction::Move(m));
}

#[test]
fn construct_system_var_variant() {
    // % を除いたキーワードを保持（展開なし・要件 8）。
    let i = Instruction::SystemVar("username".to_string());
    assert_eq!(i, Instruction::SystemVar("username".to_string()));
}

#[test]
fn construct_generic_command_variant() {
    let i = Instruction::GenericCommand {
        name: "open".to_string(),
        raw_args: vec!["sliderinput".to_string()],
    };
    assert_eq!(
        i,
        Instruction::GenericCommand {
            name: "open".to_string(),
            raw_args: vec!["sliderinput".to_string()]
        }
    );
}

#[test]
fn construct_raw_variant() {
    let i = Instruction::Raw("\\foo[a,b]".to_string());
    assert_eq!(i, Instruction::Raw("\\foo[a,b]".to_string()));
}

#[test]
fn derives_clone_and_debug() {
    // Clone / Debug が全 variant で機能する（派生確認）。
    let i = Instruction::Surface(SurfaceArg::new("1000".to_string()));
    let cloned = i.clone();
    assert_eq!(i, cloned);
    let dbg = format!("{i:?}");
    assert!(dbg.contains("Surface"));
}

#[test]
fn newline_ratio_preserves_sign() {
    // 負値も符号付きで保持（要件 4・design 値定数）。
    let r = NewLineRatio::new(-0.5);
    assert_eq!(r.ratio(), -0.5_f32);
}
