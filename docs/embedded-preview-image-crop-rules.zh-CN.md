# 旧式复合载荷预览与图片裁剪规则

## 1. 数据分层

复合载荷资源必须同时区分两层数据：

1. `format + dataBase64` 是原始 CDX/CDXML 载荷，始终是往返导出的权威数据；
2. `preview` 是从容器内容提取或由文件显式提供的 PNG/JPEG/GIF/BMP 预览，只负责显示和裁剪。

预览失败不得改变、替换或删除原始载荷。资源使用
`chemsema.resource.embedded-object.v1`，并保存明确的 `previewStatus`：

- `decoded`：已有经过限制校验的位图预览；
- `no-preview`：容器签名有效，但没有受支持的位图预览；
- `invalid-signature`：声明格式与容器签名不一致；
- `oversize`：源载荷、解压结果、像素边长或总像素数越界；
- `decode-error`：压缩大小、图像编码或容器内容损坏。

渲染器只有在 `decoded` 且存在 `preview` 时绘制图片；其余状态绘制带格式名和状态的确定性占位，不允许尝试另一条隐式 fallback。

## 2. 容器分支

每个容器使用独立签名入口：

| 格式 | 必需签名 | 预览提取 |
| --- | --- | --- |
| TIFF | `II 2A 00` 或 `MM 00 2A` | 受限 TIFF 解码 |
| EMF | record type `1` 且偏移 40 为 ` EMF` | PNG/JPEG/TIFF/GIF/BMP 或合法 DIB |
| WMF | placeable `D7 CD C6 9A`，或标准 metafile header | PNG/JPEG/TIFF/GIF/BMP 或合法 DIB |
| OLE | CFB `D0 CF 11 E0 A1 B1 1A E1` | presentation stream 中的位图内容 |
| PDF | `%PDF-` | 文件中未再次压缩的图片 preview stream |
| PICT | version-2 opcode，允许 512-byte 文件头 | 内嵌 JPEG/PNG preview |

`CompressedEnhancedMetafile`、`CompressedWindowsMetafile` 和
`CompressedOLEObject` 先走有 64 MiB 硬上限的 zlib 解压，再进入对应未压缩分支；存在
`Uncompressed...Size` 时必须与实际解压长度完全一致。
三种 `Compressed...` CDXML 属性使用 ChemDraw 的可换行 base64 wire encoding；
未压缩复合载荷和常规位图属性使用十六进制。导入和导出按属性名选择唯一编码，不做猜测。
CDX 的二进制属性本身不使用 CDXML 压缩包装；导出 CDX 前必须校验并解压为对应的
`EnhancedMetafile`、`WindowsMetafile` 或 `OLEObject` 二进制属性。

共同上限为：源/解压字节 64 MiB、单边 32768 px、总像素一亿。DIB 只接受明确的
BITMAPINFOHEADER 大小、合法 planes/bit depth/compression 和可证明的像素区长度。

## 3. 裁剪坐标

CCJS 在图片对象 `payload.imageCrop` 中保存整数资源像素矩形：

```json
"imageCrop": { "x": 120, "y": 40, "width": 640, "height": 360 }
```

坐标原点为源预览左上角。四个值必须为整数，宽高大于零，且矩形完全位于
`pixelWidth × pixelHeight` 内。没有可解码预览的复合载荷不能设置裁剪。

固定变换顺序为：

1. 在源像素空间应用 `imageCrop`；
2. 按 `fit` 映射到对象本地 `bbox`；
3. 应用对象 `scale`、`translate` 和 `rotate`。

因此移动、拉伸、旋转、组合、撤销和跨标签页复制都不得改写 `imageCrop`。GUI、SVG、
PNG 与 EMF/Office 预览都读取同一个 `RenderPrimitive::Image.sourceCrop`。

## 4. CDX/CDXML 投影

CDX/CDXML 的 `embeddedobject` 没有标准裁剪字段，禁止伪造官方属性。导出规则为：

- 未裁剪的常规位图继续保留原格式字节；
- 已裁剪的常规位图明确烘焙为 PNG，重新导入后的图片像素尺寸就是裁剪尺寸；
- 复合载荷始终保留原始 EMF/WMF/OLE/TIFF/PDF/PICT 属性；若存在预览，则另外写入 PNG
  preview。已裁剪时只烘焙这份 PNG preview，不修改原始复合载荷。

重新导入后裁剪字段可以归一为“整张预览”，但可见像素、对象框、旋转和层级必须保持不变。

## 5. 关闭门禁

- 九个已提交夹具覆盖 TIFF、EMF、WMF、OLE、PDF、PICT 和三种压缩容器；
- 损坏签名、预览缺失、压缩长度不一致、字节/像素越界分别断言明确状态；
- 内核测试覆盖裁剪校验、SVG viewBox、旋转、复制粘贴、撤销和 CDX/CDXML 投影；
- GUI 门禁覆盖右键入口、四字段对话框、Full Image/Reset 和关闭重开后的持久化；
- Office/EMF 与静态 SVG 使用同一裁剪 primitive，不允许独立重算矩形。
