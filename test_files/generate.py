"""生成 PLY、FBX、3MF 测试文件（简单三角形模型）。"""

import struct
import zipfile
import os

OUT_DIR = os.path.dirname(os.path.abspath(__file__))


def gen_ply():
    """生成 ASCII PLY 文件：一个三角形（3 顶点 + 1 面）。"""
    content = """\
ply
format ascii 1.0
element vertex 3
property float x
property float y
property float z
element face 1
property list uchar int vertex_indices
end_header
0.0 0.0 0.0
1.0 0.0 0.0
0.5 1.0 0.0
3 0 1 2
"""
    path = os.path.join(OUT_DIR, "test_triangle.ply")
    with open(path, "w", newline="\n") as f:
        f.write(content)
    print(f"  created {path}")


def gen_3mf():
    """生成 3MF 文件（ZIP 包含 XML，一个三角形 mesh）。"""
    model_xml = """\
<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02"
       xmlns:m="http://schemas.microsoft.com/3dmanufacturing/material/2015/02">
  <metadata name="Application">QuickLook Test</metadata>
  <resources>
    <object id="1" type="model">
      <mesh>
        <vertices>
          <vertex x="0" y="0" z="0" />
          <vertex x="10" y="0" z="0" />
          <vertex x="5" y="10" z="0" />
        </vertices>
        <triangles>
          <triangle v1="0" v2="1" v3="2" />
        </triangles>
      </mesh>
    </object>
  </resources>
  <build>
    <item objectid="1" />
  </build>
</model>
"""
    path = os.path.join(OUT_DIR, "test_triangle.3mf")
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("[Content_Types].xml", """\
<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml" />
  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml" />
</Types>
""")
        zf.writestr("_rels/.rels", """\
<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" />
</Relationships>
""")
        zf.writestr("3D/3dmodel.model", model_xml)
    print(f"  created {path}")


def gen_fbx():
    """生成最小化二进制 FBX 7.4 文件（一个三角形 mesh）。

    FBX 二进制格式参考:
    - https://code.blender.org/2013/08/fbx-binary-file-format-specification/
    - https://gist.github.com/Iscle/0dbcee58be8582978d15ea3629ce3e8b

    文件头 (27 字节):
      Bytes 0-20:  "Kaydara FBX Binary  \x00" (21 字节)
      Byte 21:     0x1A (unknown)
      Byte 22:     0x00 (little-endian)
      Bytes 23-26: version (uint32 LE)

    节点格式 (FBX < 7500):
      uint32 EndOffset       -- 节点结束位置（相对于文件头）
      uint32 NumProperties   -- 属性数量
      uint32 PropertyListLen -- 属性数据总长度（字节）
      uint8  NameLen         -- 节点名长度
      char[] Name            -- 节点名（不含终止符）
      Property[]             -- 属性数据
      Node[]                 -- 子节点（递归）
      13 bytes NullRecord    -- 全零

    属性类型标识:
      'Y'=i16, 'C'=bool(i8), 'I'=i32, 'F'=f32, 'D'=f64, 'L'=i64
      'S'=string, 'R'=raw
      'i'=i32[], 'f'=f32[], 'd'=f64[], 'l'=i64[]
    """

    # ── 属性序列化 ──
    def prop_i32(v: int) -> bytes:
        return b"I" + struct.pack("<i", v)

    def prop_i64(v: int) -> bytes:
        return b"L" + struct.pack("<q", v)

    def prop_f64(v: float) -> bytes:
        return b"D" + struct.pack("<d", v)

    def prop_string(s: str) -> bytes:
        b = s.encode("utf-8")
        return b"S" + struct.pack("<I", len(b)) + b

    def prop_arr_f64(arr: list[float]) -> bytes:
        data = struct.pack(f"<{len(arr)}d", *arr)
        # encoding=0 (raw), compressed_length=0
        return b"d" + struct.pack("<III", len(arr), 0, 0) + data

    def prop_arr_i32(arr: list[int]) -> bytes:
        data = struct.pack(f"<{len(arr)}i", *arr)
        return b"i" + struct.pack("<III", len(arr), 0, 0) + data

    # ── 节点构建（两遍写入：先计算大小，再填充 EndOffset）──
    def build_node(name: str, props: list[bytes], children: list[bytes] | None = None) -> bytes:
        name_b = name.encode("ascii")
        prop_data = b"".join(props)
        child_data = b"".join(children) if children else b""

        # 固定头: 4+4+4+1 = 13 字节 + 名称 + 属性 + 子节点 + 13 字节 null
        fixed = 13 + len(name_b) + len(prop_data) + len(child_data) + 13
        end_offset = fixed  # 相对于文件起始（这里假设从 0 开始拼接）

        out  = struct.pack("<I", end_offset)        # EndOffset
        out += struct.pack("<I", len(props))         # NumProperties
        out += struct.pack("<I", len(prop_data))     # PropertyListLen
        out += struct.pack("<B", len(name_b))        # NameLen
        out += name_b                                # Name
        out += prop_data                             # Properties
        out += child_data                            # Children
        out += b"\x00" * 13                          # NullRecord
        return out

    # ── 几何数据 ──
    vertices = [0.0, 0.0, 0.0,  1.0, 0.0, 0.0,  0.5, 1.0, 0.0]
    # FBX PolygonIndex: 负值的绝对值=多边形顶点数，正值=顶点索引
    polygon_index = [-3, 0, 1, 2]

    geometry = build_node("Geometry\x00\x01Geometry", [
        prop_i64(1),
        prop_string("Geometry"),
        prop_string("Mesh"),
    ], children=[
        build_node("Vertices", [prop_arr_f64(vertices)]),
        build_node("PolygonIndex", [prop_arr_i32(polygon_index)]),
    ])

    model = build_node("Model\x00\x01Model", [
        prop_i64(2),
        prop_string("Model"),
        prop_string("Mesh"),
    ], children=[geometry])

    objects = build_node("Objects", [], children=[model])

    # 根节点（空名称）
    body = build_node("", [prop_string("7.4.0")], children=[objects])

    # 文件头: 27 字节
    magic = b"Kaydara FBX Binary  \x00"  # 21 bytes
    header = magic + b"\x1a\x00" + struct.pack("<I", 7400)  # +2+4 = 27 bytes

    content = header + body

    path = os.path.join(OUT_DIR, "test_triangle.fbx")
    with open(path, "wb") as f:
        f.write(content)
    print(f"  created {path}")


if __name__ == "__main__":
    print("Generating test files...")
    gen_ply()
    gen_3mf()
    gen_fbx()
    print("Done!")
