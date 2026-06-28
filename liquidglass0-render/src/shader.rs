//! 着色器加载抽象。
//!
//! 通过 [`ShaderLoader`] trait 解耦着色器获取方式，`GlassRenderer` 只需调用
//! `loader.load_wgsl(name)` 即可拿到 WGSL 源码，不关心字符串来自文件系统、
//! 编译期嵌入还是 naga_oil 模块合成。
//!
//! # 内置实现
//!
//! - [`EmbeddedLoader`] — 编译期 `include_str!` 嵌入。
//!
//! Phase 2 将引入 [`NagaOilLoader`] 支持 `#import` 语法。

/// 着色器加载 trait。
///
/// 根据着色器名称返回 WGSL 源码字符串。
pub trait ShaderLoader {
    /// 加载指定名称的 WGSL 着色器源码。
    fn load_wgsl(&self, name: &str) -> String;
}

/// 编译期嵌入的着色器加载器。
///
/// 使用 `include_str!` 在编译时把 `.wgsl` 文件内容嵌入到二进制中，
/// 运行时按名称查找返回。
///
/// # Panics
///
/// 传入未识别的着色器名称会 panic。
pub struct EmbeddedLoader;

impl ShaderLoader for EmbeddedLoader {
    fn load_wgsl(&self, name: &str) -> String {
        let src = match name {
            "blur_horizontal" => include_str!("../../shaders/compute/blur_horizontal.wgsl"),
            "blur_vertical" => include_str!("../../shaders/compute/blur_vertical.wgsl"),
            "composite" => include_str!("../../shaders/fragment/composite.wgsl"),
            "fullscreen_triangle" => {
                include_str!("../../shaders/common/fullscreen_triangle.wgsl")
            }
            _ => panic!("未知着色器: {name}"),
        };
        src.to_owned()
    }
}
