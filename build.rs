//! Embeds `assets/icon_256.png` as the Windows executable icon (`RT_GROUP_ICON` / `RT_ICON`)
//! so Explorer, shortcuts, and the taskbar display the custom icon.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn push_res_header(out: &mut Vec<u8>, data_size: u32, type_id: u16, name_id: u16, flags: u16, lang: u16) {
    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes()); // header size
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&type_id.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&name_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // data version
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&lang.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // version
    out.extend_from_slice(&0u32.to_le_bytes()); // characteristics
}

fn main() {
    println!("cargo:rerun-if-changed=assets/icon_256.png");
    println!("cargo:rerun-if-changed=assets/icon.jpg");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let png = match fs::read("assets/icon_256.png") {
        Ok(b) => b,
        Err(_) => return,
    };
    let mut res = Vec::with_capacity(128 + png.len());
    // 1. 32-byte null resource header.
    push_res_header(&mut res, 0, 0, 0, 0, 0);
    // 2. RT_ICON (type 3, id 1) containing the 256x256 PNG bytes.
    push_res_header(&mut res, png.len() as u32, 3, 1, 0x1010, 0x0409);
    res.extend_from_slice(&png);
    while res.len() % 4 != 0 {
        res.push(0);
    }
    // 3. RT_GROUP_ICON (type 14, id 1) referencing RT_ICON #1.
    let mut grp = Vec::with_capacity(20);
    grp.extend_from_slice(&0u16.to_le_bytes()); // idReserved
    grp.extend_from_slice(&1u16.to_le_bytes()); // idType = 1 (icon)
    grp.extend_from_slice(&1u16.to_le_bytes()); // idCount = 1
    grp.push(0); // width (256)
    grp.push(0); // height (256)
    grp.push(0); // color count
    grp.push(0); // reserved
    grp.extend_from_slice(&1u16.to_le_bytes()); // planes
    grp.extend_from_slice(&32u16.to_le_bytes()); // bit count
    grp.extend_from_slice(&(png.len() as u32).to_le_bytes()); // bytes in resource
    grp.extend_from_slice(&1u16.to_le_bytes()); // icon resource ID = 1
    push_res_header(&mut res, grp.len() as u32, 14, 1, 0x1030, 0x0409);
    res.extend_from_slice(&grp);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("app_icon.res");
    if fs::write(&res_path, &res).is_err() {
        return;
    }
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let obj_path = out_dir.join("app_icon.o");
        for tool in ["windres", "x86_64-w64-mingw32-windres"] {
            if Command::new(tool)
                .args([res_path.to_str().unwrap(), "-O", "coff", "-o", obj_path.to_str().unwrap()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                println!("cargo:rustc-link-arg-bins={}", obj_path.display());
                break;
            }
        }
    } else {
        println!("cargo:rustc-link-arg-bins={}", res_path.display());
    }
}
