//! GPU 纹理读回与 PNG 编码。
//!
//! 提供 `copy_texture_to_buffer → map_async → 解码 → PNG` 的完整流程。
//! 支持 Rgba8Unorm（最终帧、模糊纹理）和 Rgba16Float（折射位移图）两种格式。

/// 纹理到 buffer 拷贝时的行对齐字节数。
const COPY_ALIGN: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// 计算行对齐后的字节数。
fn padded_bytes_per_row(width: u32, bytes_per_pixel: u32) -> u32 {
    let unpadded = width * bytes_per_pixel;
    unpadded.div_ceil(COPY_ALIGN) * COPY_ALIGN
}

/// 读回 Rgba8Unorm 纹理并编码为 PNG 字节。
///
/// # 参数
///
/// * `device` - wgpu 设备
/// * `queue` - wgpu 命令队列
/// * `texture` - 源纹理（usage 须含 COPY_SRC）
/// * `size` - 纹理尺寸（width, height）
///
/// # 返回值
///
/// PNG 字节。
///
/// # Panics
///
/// GPU map 失败时 panic。
pub fn dump_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: (u32, u32),
) -> Vec<u8> {
    let (w, h) = size;
    let bpp = 4;
    let padded_row = padded_bytes_per_row(w, bpp);
    let buf_size = (padded_row * h) as u64;

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_rgba8"),
        size: buf_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    let index = queue.submit([encoder.finish()]);

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).ok();
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index),
            timeout: None,
        })
        .unwrap();

    rx.recv().unwrap().unwrap();

    let mapped = slice.get_mapped_range();
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        let off = y as usize * padded_row as usize + x as usize * bpp as usize;
        image::Rgba([
            mapped[off],
            mapped[off + 1],
            mapped[off + 2],
            mapped[off + 3],
        ])
    });

    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    drop(mapped);
    staging.unmap();
    png
}

/// 读回 Rgba16Float 折射位移纹理并编码为方向可视化 PNG。
///
/// R 通道 = UV 偏移 .x（像素），G 通道 = UV 偏移 .y（像素）。
/// 正值编码为暖色（红/绿），负值为冷色，零偏移为黑色。
///
/// # 参数
///
/// * `device` - wgpu 设备
/// * `queue` - wgpu 命令队列
/// * `texture` - 源纹理（usage 须含 COPY_SRC，格式 Rgba16Float）
/// * `size` - 纹理尺寸（width, height）
///
/// # 返回值
///
/// PNG 字节。
pub fn dump_displacement(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    size: (u32, u32),
) -> Vec<u8> {
    let (w, h) = size;
    let bpp = 8;
    let padded_row = padded_bytes_per_row(w, bpp);
    let buf_size = (padded_row * h) as u64;

    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("staging_rgba16f"),
        size: buf_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    let index2 = queue.submit([encoder.finish()]);

    let slice = staging.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).ok();
    });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(index2),
            timeout: None,
        })
        .unwrap();

    rx.recv().unwrap().unwrap();

    let mapped = slice.get_mapped_range();

    // 位移上限（像素），用于颜色映射
    let max_disp: f32 = 20.0;
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        let off = y as usize * padded_row as usize + x as usize * bpp as usize;
        let dx = half::f16::from_le_bytes([mapped[off], mapped[off + 1]]).to_f32();
        let dy = half::f16::from_le_bytes([mapped[off + 2], mapped[off + 3]]).to_f32();
        let r = ((dx / max_disp * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
        let g = ((dy / max_disp * 0.5 + 0.5) * 255.0).clamp(0.0, 255.0) as u8;
        image::Rgba([r, g, 0, 255])
    });

    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();

    drop(mapped);
    staging.unmap();
    png
}
