//! 着色器加载抽象。
//!
//! 通过 [`ShaderLoader`] trait 解耦着色器获取方式，`GlassRenderer` 只需调用
//! `loader.load(name)` 即可拿到 `ShaderSource`，不关心来源是编译期嵌入、
//! 文件系统还是 naga_oil 模块合成。
//!
//! # 内置实现
//!
//! - [`EmbeddedLoader`] — 编译期 `include_str!` 嵌入，返回 `ShaderSource::Wgsl`。
//! - [`NagaOilLoader`] — naga_oil `Composer` 合成，支持 `#import` 语法，
//!   返回 `ShaderSource::Naga`，避免 WGSL→IR→WGSL 往返。

use std::borrow::Cow;
use std::cell::RefCell;

use naga_oil::compose::{ComposableModuleDescriptor, Composer, NagaModuleDescriptor};

/// 着色器加载 trait。
///
/// 根据着色器名称返回 `wgpu::ShaderSource`，支持 WGSL 源码或 naga Module。
pub trait ShaderLoader {
    /// 加载指定名称的着色器。
    fn load(&self, name: &str) -> wgpu::ShaderSource<'static>;
}

/// 编译期嵌入的着色器加载器。
///
/// 使用 `include_str!` 在编译时把 `.wgsl` 文件内容嵌入到二进制中，
/// 运行时按名称查找返回 `ShaderSource::Wgsl`。
///
/// # Panics
///
/// 传入未识别的着色器名称会 panic。
pub struct EmbeddedLoader;

impl ShaderLoader for EmbeddedLoader {
    fn load(&self, name: &str) -> wgpu::ShaderSource<'static> {
        let src = match name {
            "blur_horizontal" => include_str!("../../shaders/compute/blur_horizontal.wgsl"),
            "blur_vertical" => include_str!("../../shaders/compute/blur_vertical.wgsl"),
            "composite" => include_str!("../../shaders/fragment/composite.wgsl"),
            "fullscreen_triangle" => {
                include_str!("../../shaders/common/fullscreen_triangle.wgsl")
            }
            _ => panic!("未知着色器: {name}"),
        };
        wgpu::ShaderSource::Wgsl(Cow::Owned(src.to_owned()))
    }
}

/// 基于 naga_oil 的着色器加载器。
///
/// 内部持有 [`Composer`]，构造时注册所有公共模块（`#define_import_path`），
/// 加载时通过 `#import` 解析依赖，返回 `ShaderSource::Naga`。
///
/// 相比 [`EmbeddedLoader`]，避免了 WGSL→naga IR→WGSL 的往返，
/// 且公共模块只解析一次，多个着色器共享 IR。
///
/// # Panics
///
/// 模块注册失败或着色器编译失败时 panic。
pub struct NagaOilLoader {
    composer: RefCell<Composer>,
}

impl NagaOilLoader {
    /// 创建加载器，注册所有公共模块。
    pub fn new() -> Self {
        let mut composer = Composer::non_validating();

        // 注册公共模块：这些文件包含 `#define_import_path`，可被其他着色器 #import
        let common_modules = [
            ("sdf", include_str!("../../shaders/common/sdf.wgsl")),
            (
                "glass_material",
                include_str!("../../shaders/common/glass_material.wgsl"),
            ),
            (
                "fullscreen_triangle",
                include_str!("../../shaders/common/fullscreen_triangle.wgsl"),
            ),
        ];

        for (name, source) in &common_modules {
            composer
                .add_composable_module(ComposableModuleDescriptor {
                    source,
                    file_path: &format!("{name}.wgsl"),
                    ..Default::default()
                })
                .unwrap_or_else(|e| panic!("注册公共模块 {name} 失败: {e}"));
        }

        Self {
            composer: RefCell::new(composer),
        }
    }
}

impl Default for NagaOilLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ShaderLoader for NagaOilLoader {
    fn load(&self, name: &str) -> wgpu::ShaderSource<'static> {
        let source = match name {
            "blur_horizontal" => include_str!("../../shaders/compute/blur_horizontal.wgsl"),
            "blur_vertical" => include_str!("../../shaders/compute/blur_vertical.wgsl"),
            "composite" => include_str!("../../shaders/fragment/composite.wgsl"),
            _ => panic!("未知着色器: {name}"),
        };

        let module = self
            .composer
            .borrow_mut()
            .make_naga_module(NagaModuleDescriptor {
                source,
                file_path: &format!("{name}.wgsl"),
                ..Default::default()
            })
            .unwrap_or_else(|e| panic!("编译着色器 {name} 失败: {e}"));

        wgpu::ShaderSource::Naga(Cow::Owned(module))
    }
}
