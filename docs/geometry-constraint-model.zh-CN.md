# Geometry / Constraint 原生对象设计

## 1. 定位

ChemSema 将 CDX/CDXML 的 `geometry` 与 `constraint` 实现为原生、可编辑、可渲染的 `SceneObject`，而不是导入时拍扁成普通线条或文本。

- Geometry 是由有序 basis 派生出的点、线、平面、质心或法线。
- Constraint 是由有序 basis 定义的距离、角度、二面角或排斥球。
- 标注不反向约束二维结构，也不引入几何求解器；移动 basis 后实时重算。
- 依赖图允许 Geometry 继续作为其他 Geometry/Constraint 的 basis，但禁止循环引用。
- 无法解析的源 `BasisObjects` 显式保存在 `unresolvedBasisIds`；渲染诊断错误，不用 BoundingBox、零点或其他 fallback 伪造结果。

## 2. 创建入口与有序选择

不增加顶层菜单或工具栏按钮。选择工具下，只有当前选择满足某个合法签名时，右键菜单才显示“标注”：

- 1 个点：排斥球。
- 2 个点：距离、指定距离点、百分比点、最佳拟合线、质心、排斥球。
- 3 个点：角度、最佳拟合线、最佳拟合面、质心、排斥球。
- 4 个及以上点：二面角、最佳拟合线、最佳拟合面、质心、排斥球。
- 点 + 线：点线成面；当该线是法线 Geometry 时还可建立法向距离点。
- 点 + 面：法线。
- 线/面 + 线/面：角度。

`Shift+单击` 形成稳定的 `orderedEntities`。框选只表示无序集合，不能创建依赖顺序的对象。创建前由内核返回属性对话框 schema；前端只负责按 schema 绘制，不能自行推断默认值或合法字段。

## 3. CCJS 模型

Geometry 示例：

```json
{
  "id": "geometry_1",
  "type": "geometry",
  "linkPolicy": "linked",
  "payload": {
    "geometry": {
      "feature": "line-from-points",
      "basisEntityIds": ["n1", "n2"],
      "pointIsDirected": false
    }
  }
}
```

Constraint 示例：

```json
{
  "id": "constraint_1",
  "type": "constraint",
  "linkPolicy": "linked",
  "payload": {
    "constraint": {
      "constraintType": "distance",
      "basisEntityIds": ["n1", "n2"],
      "minimum": 4,
      "maximum": 5,
      "ignoreUnconnectedAtoms": false,
      "dihedralIsChiral": false,
      "pointIsDirected": false,
      "display": {
        "autoValue": true,
        "positioningType": "auto",
        "fontFamily": "Arial",
        "fontSize": 7.5,
        "fill": "#000000",
        "fontWeight": 400,
        "italic": false,
        "underline": false,
        "indicatorVisible": true
      }
    }
  }
}
```

`basisEntityIds` 是有序强引用。对应的 `annotation-basis` LinkRelation 只服务于聚焦显示、Alt 双击和通用 Link 行为；CDX/CDXML 仍以官方 `BasisObjects` 为语义权威。

## 4. 支持类型与求值规则

Geometry 完整支持以下官方枚举：

- `point-from-point-point-distance`
- `point-from-point-point-percentage`
- `point-from-point-normal-distance`
- `line-from-points`
- `plane-from-points`
- `plane-from-point-line`
- `centroid-from-points`
- `normal-from-point-plane`

规则是明确分支：

- 指定点按有向基准向量、距离或百分比计算。
- 最佳拟合线使用全部点的二维主方向，不只取首尾点。
- 平面在二维编辑视图中以 basis 的凸包投影显示；少于三个非共线点明确报错。
- 法线、线线角、线面角、面面角递归解析派生对象。
- Distance 要求两个点；Angle 接受三点、四点或两个线/面对象；ExclusionSphere 接受一个或多个点并以其质心为中心。
- 四点 Angle 表示官方二面角签名；`dihedralIsChiral` 决定有符号语义，不另造类型。

## 5. 显示文字与位置

Constraint 的数值文字是对象自身的显示层，不是独立文本框：

- `autoValue=true` 时由 `minimum`/`maximum` 生成，距离使用 Å，角度使用 °；范围用 en dash。
- 用户编辑文字后保存为 `autoValue=false + textOverride`，之后数值不再自动覆盖文字。
- `indicatorVisible` 独立控制辅助线/弧；不隐藏数值文字。
- 字体、字号、颜色、粗体、斜体和下划线均为明确 CCJS 字段。

位置遵循官方 `PositioningType`：

- `auto`：每帧从 basis 计算；CDXML 缺省即 auto，缓存的 `t@p` 不作为固定位置。
- `absolute`：使用明确的 `position`。
- `offset`：使用自动位置加 `positioningOffset`。
- `angle`：保存 `positioningAngle`，并使用文件记录的位置。

右键“标注属性”可以编辑数值范围、自动文字、显示文字、定位模式及参数、指示线、字体和颜色。无效组合（如 absolute 没有位置、offset 没有偏移、angle 没有角度）由内核拒绝，不自动改用其他模式。

## 6. 交互规则

- 普通单击选择标注；普通双击遵循 Group，Alt 双击遵循 Link。
- Alt 命中优先穿透标注，直接选择底下的原子或键。
- 单独拖拽 Geometry/Constraint 时，鼠标移动只创建渲染预览。文档 JSON、自动保存状态和撤销栈都不改变；松手立即丢弃预览并回到 basis 派生位置。
- 移动 basis 原子、键或派生 Geometry 时，每帧重算标注；松手只提交 basis 的实际变化。
- 同时选择 basis 与其标注移动时，只移动 basis，标注跟随重算，禁止叠加第二次平移。
- Geometry/Constraint 不提供旋转柄或缩放柄。需要固定文字位置时使用属性对话框，而不是把临时拖拽解释为定位操作。

ChemDraw 21 实测结论：距离标注拖动时跟手，释放后回到 basis 计算位置，且文档不变；ChemDraw 重新保存可能规范化内部文字基线，这与拖拽持久化无关。

## 7. 生命周期、复制与删除

- 删除标注只删除标注和对应 LinkRelation。
- 删除任一 basis 后，递归级联删除所有直接或间接依赖的 Geometry/Constraint，并清理 LinkRelation。
- 复制标注只有在全部 basis 同时被复制时才成立；粘贴前统一预分配场景、节点、键和派生对象 ID，再一次性重映射强引用。
- Group 子对象执行同样的递归筛选；不能因标注藏在 Group 内而留下悬空 basis。
- CCJS 解析拒绝缺失引用和循环；从外部文件导入但无法映射的引用保留为显式 invalid 状态，允许用户查看诊断和无损导回。

## 8. CDX/CDXML 往返

原生解析、编辑和导出字段：

- Geometry：`GeometricFeature`、`RelationValue`、`BasisObjects`、`PointIsDirected`。
- Constraint：`ConstraintType`、`ConstraintMin`、`ConstraintMax`、`BasisObjects`、`IgnoreUnconnectedAtoms`、`DihedralIsChiral`、`PointIsDirected`。
- 显示：`Visible`、`Z`、`Name`、`BoundingBox`、`color`、`LineWidth`、`HashSpacing`。
- ObjectTag：文字 runs、`PositioningType`、`PositioningAngle`、`PositioningOffset` 及可见性。

CDX 的 `GeometricFeature` 与 `ConstraintType` 使用官方 INT8 枚举编码；编码器同时接受名字和合法数值，解码统一恢复名字。导出 ID 从现有 interchange 最大数字之后分配，原生对象 ID 和 `BasisObjects` 使用同一张映射表。

## 9. 验收

- 所有官方 Geometry/Constraint 类型均可创建、读取、编辑、渲染、保存和重开。
- CCJS、CDXML、CDX 均保持 basis 顺序、显示字段和定位字段。
- SVG、EMF、桌面端与 Web 端复用同一内核 RenderPrimitive。
- 覆盖递归求值、非法签名、共线平面、循环、未解析引用、临时拖拽、basis 跟随、级联删除、复制重映射、属性对话框和 CDX 往返测试。
