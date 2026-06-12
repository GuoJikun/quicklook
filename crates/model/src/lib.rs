use std::fs::File;
use std::io::BufReader;

use serde::{Deserialize, Serialize};

use quicklook_error::QuickLookError;

/// 3D 模型基础信息（顶点数、面数、格式）。
#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub vertex_count: usize,
    pub face_count: usize,
    pub format: String,
}

/// 读取 3D 模型文件并返回基础统计信息。
pub fn load_model(path: &str, extension: &str) -> Result<ModelInfo, QuickLookError> {
    log::info!("加载 3D 模型: {}, 扩展名: {}", path, extension);
    let result = match extension {
        "gltf" | "glb" => load_gltf(path),
        "stl" => load_stl(path),
        "obj" => load_obj(path),
        _ => Err(QuickLookError::UnsupportedModelFormat(
            extension.to_string(),
        )),
    };
    match &result {
        Ok(info) => log::info!(
            "3D 模型加载成功: 顶点数={}, 面数={}",
            info.vertex_count,
            info.face_count
        ),
        Err(e) => log::error!("3D 模型加载失败: {}", e),
    }
    result
}

fn load_gltf(path: &str) -> Result<ModelInfo, QuickLookError> {
    let (document, _buffers, _) =
        gltf::import(path).map_err(|e| QuickLookError::ModelParse(e.to_string()))?;

    let mut vertex_count = 0usize;
    let mut face_count = 0usize;

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            if let Some(accessor) = primitive.get(&gltf::Semantic::Positions) {
                vertex_count += accessor.count();
            }
            if let Some(accessor) = primitive.indices() {
                face_count += accessor.count() / 3;
            } else if let Some(accessor) = primitive.get(&gltf::Semantic::Positions) {
                face_count += accessor.count() / 3;
            }
        }
    }

    Ok(ModelInfo {
        vertex_count,
        face_count,
        format: "gltf".to_string(),
    })
}

fn load_stl(path: &str) -> Result<ModelInfo, QuickLookError> {
    let file = File::open(path).map_err(|e| QuickLookError::ModelParse(e.to_string()))?;
    let mut reader = BufReader::new(file);

    let stl =
        stl_io::read_stl(&mut reader).map_err(|e| QuickLookError::ModelParse(e.to_string()))?;

    Ok(ModelInfo {
        // STL 每个面有 3 个顶点，通常顶点不去重
        vertex_count: stl.faces.len() * 3,
        face_count: stl.faces.len(),
        format: "stl".to_string(),
    })
}

fn load_obj(path: &str) -> Result<ModelInfo, QuickLookError> {
    let model = obj::Obj::load(path).map_err(|e| QuickLookError::ModelParse(e.to_string()))?;

    let vertex_count = model.data.position.len();
    let face_count: usize = model
        .data
        .objects
        .iter()
        .flat_map(|o| o.groups.iter())
        .flat_map(|g| g.polys.iter())
        .map(|p| {
            let n = p.0.len();
            if n >= 3 { n - 2 } else { 0 } // 三角剖分：n 边形 = n-2 个三角面
        })
        .sum();

    Ok(ModelInfo {
        vertex_count,
        face_count,
        format: "obj".to_string(),
    })
}
