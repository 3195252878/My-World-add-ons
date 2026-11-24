use std::env;
use std::fs;

fn le16(d: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([*d.get(p).unwrap_or(&0), *d.get(p + 1).unwrap_or(&0)])
}

fn le32(d: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([
        *d.get(p).unwrap_or(&0),
        *d.get(p + 1).unwrap_or(&0),
        *d.get(p + 2).unwrap_or(&0),
        *d.get(p + 3).unwrap_or(&0),
    ])
}

struct Img {
    w: u8,
    h: u8,
    planes: u16,
    bpp: u16,
    blob: Vec<u8>,
}

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=custom_icon.ico");

    // ---- 解析 ICO 文件（各尺寸位图原样作为 RT_ICON 数据）----
    let ico = fs::read("custom_icon.ico").expect("读取 custom_icon.ico 失败");
    assert!(ico.len() >= 6, "custom_icon.ico 太短，连头部都不完整");
    assert_eq!(&ico[0..4], &[0, 0, 1, 0], "custom_icon.ico 不是有效的 ICO");
    let n = le16(&ico, 4) as usize;
    assert!(n > 0, "ICO 中没有图标图像");
    // 目录项共 n*16 字节，头 6 字节，超了说明文件被截断或条目数虚高
    assert!(
        6 + n * 16 <= ico.len(),
        "custom_icon.ico 已损坏：目录项声称 {n} 个图像，但文件只有 {} 字节",
        ico.len()
    );
    let mut imgs: Vec<Img> = Vec::with_capacity(n);
    for i in 0..n {
        let e = 6 + i * 16;
        let off = le32(&ico, e + 12) as usize;
        let size = le32(&ico, e + 8) as usize;
        // 数据和大小都要落在文件内，否则下面切片会 panic 且看不出原因
        assert!(
            off <= ico.len() && size <= ico.len() - off,
            "custom_icon.ico 第 {} 个图像越界：offset={off} size={size} 文件 {} 字节",
            i + 1,
            ico.len()
        );
        imgs.push(Img {
            w: ico[e],
            h: ico[e + 1],
            planes: le16(&ico, e + 4),
            bpp: le16(&ico, e + 6),
            blob: ico[off..off + size].to_vec(),
        });
    }
    assert!(!imgs.is_empty(), "ICO 中没有图标图像");

    // ---- GROUP_ICON 资源数据（GRPICONDIRENTRY 每项 14 字节）----
    let mut group: Vec<u8> = vec![0, 0, 1, 0];
    group.extend_from_slice(&(n as u16).to_le_bytes());
    for (i, im) in imgs.iter().enumerate() {
        group.extend_from_slice(&[im.w, im.h, 0, 0]);
        group.extend_from_slice(&im.planes.to_le_bytes());
        group.extend_from_slice(&im.bpp.to_le_bytes());
        group.extend_from_slice(&(im.blob.len() as u32).to_le_bytes());
        group.extend_from_slice(&((i + 1) as u16).to_le_bytes());
    }

    // ---- .rsrc 段布局（目录树 + 数据项 + 数据，段内相对偏移）----
    let off_l0 = 0usize;
    let off_l1_icon = 32;
    let off_l1_group = off_l1_icon + 16 + n * 8;
    let off_l2_icon = off_l1_group + 24;
    let off_l2_group = off_l2_icon + n * 24;
    let off_de_icon = off_l2_group + 24;
    let off_de_group = off_de_icon + n * 16;
    let off_blob_icon = off_de_group + 16;
    let mut blob_rva = vec![0u32; n];
    let mut p = off_blob_icon;
    for (i, im) in imgs.iter().enumerate() {
        blob_rva[i] = p as u32;
        p += im.blob.len();
    }
    let off_group_blob = p;
    let total = off_group_blob + group.len();

    let mut buf = vec![0u8; total];
    fn wr32(buf: &mut [u8], off: usize, v: u32) {
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn wr16(buf: &mut [u8], off: usize, v: u16) {
        buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }
    // 一级目录：类型 ICON(3) + GROUP_ICON(14)
    wr16(&mut buf, off_l0 + 14, 2);
    wr32(&mut buf, off_l0 + 16, 3);
    wr32(&mut buf, off_l0 + 20, (off_l1_icon as u32) | 0x8000_0000);
    wr32(&mut buf, off_l0 + 24, 14);
    wr32(&mut buf, off_l0 + 28, (off_l1_group as u32) | 0x8000_0000);
    // 二级目录（ICON）：7 个图像 id
    for (i, _) in imgs.iter().enumerate() {
        let d = off_l1_icon + 16 + i * 8;
        wr16(&mut buf, off_l1_icon + 14, n as u16);
        wr32(&mut buf, d, (i + 1) as u32);
        wr32(&mut buf, d + 4, (off_l2_icon + i * 24) as u32 | 0x8000_0000);
    }
    // 二级目录（GROUP_ICON）：id 1
    wr16(&mut buf, off_l1_group + 14, 1);
    wr32(&mut buf, off_l1_group + 16, 1);
    wr32(
        &mut buf,
        off_l1_group + 20,
        (off_l2_group as u32) | 0x8000_0000,
    );
    // 三级语言目录 + 数据项（OffsetToData 存 0，由 blob 符号的 ADDR32NB 重定位填出最终 RVA）
    for i in 0..n {
        let d = off_l2_icon + i * 24;
        wr16(&mut buf, d + 14, 1);
        wr32(&mut buf, d + 16, 0);
        wr32(&mut buf, d + 20, (off_de_icon + i * 16) as u32);
    }
    wr16(&mut buf, off_l2_group + 14, 1);
    wr32(&mut buf, off_l2_group + 16, 0);
    wr32(&mut buf, off_l2_group + 20, off_de_group as u32);
    for i in 0..n {
        let d = off_de_icon + i * 16;
        wr32(&mut buf, d, 0);
        wr32(&mut buf, d + 4, imgs[i].blob.len() as u32);
    }
    wr32(&mut buf, off_de_group, 0);
    wr32(&mut buf, off_de_group + 4, group.len() as u32);
    for (i, im) in imgs.iter().enumerate() {
        buf[blob_rva[i] as usize..blob_rva[i] as usize + im.blob.len()].copy_from_slice(&im.blob);
    }
    buf[off_group_blob..off_group_blob + group.len()].copy_from_slice(&group);

    // ---- 打包为 COFF 对象：.rsrc 段 + 段符号/blob 符号 + ADDR32NB 重定位 ----
    // 约定与 windres 产物一致：OffsetToData 字段存 0，由指向 blob 符号（值=段内偏移）
    // 的 ADDR32NB 重定位在链接时填出最终 RVA
    let nreloc = n + 1;
    let rel_size = nreloc * 10;
    let nsyms = 2 + nreloc; // 段符号+其辅助记录 + 每个 blob 一个符号
    let sym_ptr = 20 + 40 + total + rel_size;
    let mut obj: Vec<u8> = Vec::with_capacity(sym_ptr + 18 * nsyms as usize + 4);
    obj.extend_from_slice(&0x8664u16.to_le_bytes()); // AMD64
    obj.extend_from_slice(&1u16.to_le_bytes()); // 节数量
    obj.extend_from_slice(&0u32.to_le_bytes()); // 时间戳
    obj.extend_from_slice(&(sym_ptr as u32).to_le_bytes());
    obj.extend_from_slice(&(nsyms as u32).to_le_bytes());
    obj.extend_from_slice(&0u16.to_le_bytes()); // 无可选头
    obj.extend_from_slice(&0x0100u16.to_le_bytes()); // 32BIT_MACHINE
    let mut sec = [0u8; 40];
    sec[0..8].copy_from_slice(b".rsrc\0\0\0");
    sec[16..20].copy_from_slice(&(total as u32).to_le_bytes()); // raw size
    sec[20..24].copy_from_slice(&60u32.to_le_bytes()); // raw pointer
    sec[24..28].copy_from_slice(&((60 + total) as u32).to_le_bytes()); // 重定位表位置
    sec[32..34].copy_from_slice(&(nreloc as u16).to_le_bytes());
    sec[36..40].copy_from_slice(&0x4000_0040u32.to_le_bytes()); // 初始化数据|读
    obj.extend_from_slice(&sec);
    obj.extend_from_slice(&buf);
    for i in 0..nreloc {
        let (pos, sym) = if i < n {
            (off_de_icon + i * 16, 2 + i)
        } else {
            (off_de_group, 2 + n)
        };
        obj.extend_from_slice(&(pos as u32).to_le_bytes());
        obj.extend_from_slice(&(sym as u32).to_le_bytes());
        obj.extend_from_slice(&0x0003u16.to_le_bytes()); // AMD64 ADDR32NB（RVA）
    }
    // 符号表：0=段符号（带 1 条辅助记录），1=辅助，2..=各 blob 符号
    obj.extend_from_slice(b".rsrc\0\0\0");
    obj.extend_from_slice(&0u32.to_le_bytes()); // value
    obj.extend_from_slice(&1u16.to_le_bytes()); // 段号
    obj.extend_from_slice(&0u16.to_le_bytes()); // 类型
    obj.push(3); // IMAGE_SYM_CLASS_STATIC
    obj.push(1); // 1 条辅助记录
    obj.extend_from_slice(&(total as u32).to_le_bytes()); // aux: 段长度
    obj.extend_from_slice(&(nreloc as u16).to_le_bytes()); // aux: 重定位数
    obj.extend_from_slice(&0u16.to_le_bytes()); // aux: 行号数
    obj.extend_from_slice(&0u32.to_le_bytes()); // aux: 校验和
    obj.extend_from_slice(&0u16.to_le_bytes()); // aux: 编号
    obj.push(0); // aux: selection
    obj.extend_from_slice(&[0, 0, 0]); // aux: 保留
    for i in 0..nreloc {
        let value = if i < n {
            blob_rva[i]
        } else {
            off_group_blob as u32
        };
        let name = format!("$R{i:06}");
        obj.extend_from_slice(name.as_bytes()); // 恰好 8 字节，无需字符串表
        obj.extend_from_slice(&value.to_le_bytes()); // 段内偏移
        obj.extend_from_slice(&1u16.to_le_bytes()); // 段号
        obj.extend_from_slice(&0u16.to_le_bytes()); // 类型
        obj.push(3); // IMAGE_SYM_CLASS_STATIC
        obj.push(0);
    }
    obj.extend_from_slice(&4u32.to_le_bytes()); // 空字符串表

    let out_dir = env::var("OUT_DIR").unwrap();
    let obj_path = format!("{}/icon_resource.o", out_dir);
    fs::write(&obj_path, &obj).expect("写出图标资源对象失败");
    println!("cargo:rustc-link-arg-bins={}", obj_path);
}
