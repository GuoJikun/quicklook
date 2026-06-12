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
        "ply" => load_ply(path),
        "fbx" => load_fbx(path),
        "3mf" => load_3mf(path),
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

fn load_ply(path: &str) -> Result<ModelInfo, QuickLookError> {
    let file = File::open(path).map_err(|e| QuickLookError::ModelParse(e.to_string()))?;
    let mut reader = BufReader::new(file);

    let ply = ply_rs::parser::Parser::<ply_rs::ply::DefaultElement>::new()
        .read_ply(&mut reader)
        .map_err(|e| QuickLookError::ModelParse(e.to_string()))?;

    let vertex_count = ply
        .payload
        .get(&"vertex".to_string())
        .map(|e| e.len())
        .unwrap_or(0);
    let face_count = ply
        .payload
        .get(&"face".to_string())
        .map(|e| e.len())
        .unwrap_or(0);

    Ok(ModelInfo {
        vertex_count,
        face_count,
        format: "ply".to_string(),
    })
}

fn load_fbx(path: &str) -> Result<ModelInfo, QuickLookError> {
    use fbxcel::tree::any::AnyTree;

    let file = File::open(path).map_err(|e| QuickLookError::ModelParse(e.to_string()))?;
    let reader = BufReader::new(file);

    let tree = AnyTree::from_seekable_reader(reader)
        .map_err(|e| QuickLookError::ModelParse(e.to_string()))?;

    let mut vertex_count = 0usize;
    let mut face_count = 0usize;

    match tree {
        AnyTree::V7400(_version, tree, _footer) => {
            collect_fbx_stats(tree.root(), &mut vertex_count, &mut face_count);
        }
        _ => {
            return Err(QuickLookError::ModelParse(
                "Unsupported FBX version".to_string(),
            ));
        }
    }

    Ok(ModelInfo {
        vertex_count,
        face_count,
        format: "fbx".to_string(),
    })
}

fn collect_fbx_stats(
    node: fbxcel::tree::v7400::NodeHandle<'_>,
    vertex_count: &mut usize,
    face_count: &mut usize,
) {
    let name = node.name();
    let attrs = node.attributes();

    if name == "Vertices" {
        if let Some(fbxcel::low::v7400::AttributeValue::ArrF64(v)) = attrs.first() {
            *vertex_count += v.len() / 3;
        }
    } else if name == "PolygonIndex" {
        if let Some(fbxcel::low::v7400::AttributeValue::ArrI32(v)) = attrs.first() {
            *face_count += count_faces_from_polygon_index(v);
        }
    }

    for child in node.children() {
        collect_fbx_stats(child, vertex_count, face_count);
    }
}

fn count_faces_from_polygon_index(indices: &[i32]) -> usize {
    let mut count = 0usize;
    let mut i = 0;
    while i < indices.len() {
        if indices[i] < 0 {
            let n = (!indices[i]) as i32;
            count += (n - 2) as usize;
            i += n as usize + 1;
        } else {
            count += 1;
            i += 1;
        }
    }
    count
}

fn load_3mf(path: &str) -> Result<ModelInfo, QuickLookError> {
    use lib3mf::Model;

    let file = File::open(path).map_err(|e| QuickLookError::ModelParse(e.to_string()))?;
    let model = Model::from_reader(file)
        .map_err(|e| QuickLookError::ModelParse(e.to_string()))?;

    let mut vertex_count = 0usize;
    let mut face_count = 0usize;

    for object in &model.resources.objects {
        if let Some(ref mesh) = object.mesh {
            vertex_count += mesh.vertices.len();
            face_count += mesh.triangles.len();
        }
    }

    Ok(ModelInfo {
        vertex_count,
        face_count,
        format: "3mf".to_string(),
    })
}
