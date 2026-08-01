# CDX/CDXML 字段复核总账

本总账由官方 CDX 属性/对象表和 Revvity 当前 CDXML DTD 自动生成。机器可读全集见 `schemas/cdx-cdxml-verification-v1.json`。

这里把三件事分开记录：`schemaStatus` 是官方 tag、类型、枚举和默认值是否已核对；`storageStatus` 是是否能无损进入 CCJS、修改并回写；`behaviorStatus` 是是否完成 ChemDraw 实物/视觉行为矩阵。不能用无损保存冒充行为已经完全复现。

当前官方全集：35 个 CDX 对象/辅助对象、262 个 CDX 属性、53 个 CDXML 元素、384 个唯一 CDXML 属性名。无损未覆盖数：0。

## 行为证据门禁

CDXML 不只按 384 个唯一属性名统计，还按“元素 × 属性”展开为 762 个具体声明。当前已复核 762/762，门禁不允许遗留 `in-review`。

这里的 `verified` 不等于“所有对象都已原生重绘”：对于质粒、凝胶标记、化学计量表等专用上下文对象，表示已核对 DTD 词法/枚举/默认值、ChemDraw 对无效上下文的清理行为，以及 CCJS 无损可编辑往返。是原生语义、上下文规则，还是官方明确的只写/未启用，由 `schemas/chemdraw-cdxml-field-evidence-v1.json` 的 `verificationKind` 区分。

原生绘制、编辑和往返的逐项实施顺序见 `docs/cdx-cdxml-native-rendering-backlog.zh-CN.md`。

## 实现规则

- `native-semantic`：已映射为来源无关的 CCJS 明确字段；编辑原生字段，导出器负责换算。
- `typed-interchange`：官方公共词法类型已解析为可编辑 `value`，并保留 tag/type。
- `binary-interchange`：官方没有稳定公共词法形式或结构复杂；保留精确 `rawBase64`，由专用编辑器修改。
- `opaque-by-spec`：官方明确规定为未定义或不解释的字节载荷；不得猜测语义，编辑 `rawBase64`。
- `context-dependent-interchange`：类型由同对象的其他字段决定；保留 `value` 与 `rawBase64`，专用编辑器联合修改。
- `interchange` 不是 `meta`，是 CCJS 顶层的持久化、可编辑、参与导出的正式字段。

## CDX 对象全集

| tag | CDXML 对象 | 实现 | schema | storage | behavior |
| --- | --- | --- | --- | --- | --- |
| `0x8000` | `CDXML` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8001` | `page` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8002` | `group` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8003` | `fragment` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8004` | `n` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8005` | `b` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8006` | `t` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8007` | `graphic` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8017` | `bracketedgroup` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x8018` | `bracketattachment` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x8019` | `crossingbond` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x8008` | `curve` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8009` | `embeddedobject` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8016` | `table` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x800A` | `altgroup` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x800B` | `templategrid` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x800C` | `regnum` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x800D` | `scheme` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x800E` | `step` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x8010` | `spectrum` | `interchange-object` | `verified` | `verified` | `verified` |
| `0x8011` | `objecttag` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x8013` | `sequence` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x8014` | `crossreference` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x801A` | `border` | `interchange-object` | `verified-with-erratum` | `verified` | `verified` |
| `0x801B` | `geometry` | `interchange-object` | `verified-with-erratum` | `verified` | `verified` |
| `0x801C` | `constraint` | `interchange-object` | `verified-with-erratum` | `verified` | `verified` |
| `0x801D` | `tlcplate` | `interchange-object` | `verified-with-erratum` | `verified` | `verified` |
| `0x801E` | `tlclane` | `interchange-object` | `verified-with-erratum` | `verified` | `verified` |
| `0x801F` | `tlcspot` | `interchange-object` | `verified-with-erratum` | `verified` | `verified` |
| `0x8015` | `splitter` | `native-object-tag` | `verified` | `verified` | `verified` |
| `0x8020` | `chemicalproperty` | `native-object-tag` | `verified-with-erratum` | `verified` | `verified` |
| `0x0300` | `colortable` | `property-backed-helper` | `verified` | `verified` | `verified` |
| `0x0100` | `fonttable` | `property-backed-helper` | `verified` | `verified` | `verified` |
| `0x000E` | `represent` | `property-backed-helper` | `verified` | `verified` | `verified` |
| `0x8021` | `arrow` | `interchange-object` | `verified-with-erratum` | `verified` | `verified` |

## CDX 属性全集

| tag | CDXML 名 | CDX 类型 | 实现/编辑 | schema | storage | behavior | 规则摘要 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `0x0001` | `CreationUserName` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | The name of the creator (program user's name) of the document. Official lexical/binary codec is available; value is editable. |
| `0x0002` | `CreationDate` | `CDXDate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The time of object creation. Official lexical/binary codec is available; value is editable. |
| `0x0003` | `CreationProgram` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | The name of the program, including version and platform, that created the associated CDX object. ChemDraw 4.0 uses "ChemDraw 4.0" as the value of CreationProgram. Official lexical/binary codec is available; value is editable. |
| `0x0004` | `ModificationUserName` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | The name of the last modifier (program user's name) of the document. Official lexical/binary codec is available; value is editable. |
| `0x0005` | `ModificationDate` | `CDXDate` | `typed-interchange/value` | `verified` | `verified` | `verified` | Time of the last modification. Official lexical/binary codec is available; value is editable. |
| `0x0006` | `ModificationProgram` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | The name of the program, including version and platform, of the last program to perform a modification. ChemDraw 4.0 uses "ChemDraw 4.0" as the value of CreationProgram. Official lexical/binary codec is available; value is editable. |
| `0x0008` | `Name` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for objecttags. Name of an object. Official lexical/binary codec is available; value is editable. |
| `0x0009` | `Comment` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | An arbitrary string intended to be meaningful to a user. Official lexical/binary codec is available; value is editable. |
| `0x000A` | `Z` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | Back-to-front ordering index in 2D drawing. Official lexical/binary codec is available; value is editable. |
| `0x000B` | `RegistryNumber` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | A registry or catalog number of a molecule object. Official lexical/binary codec is available; value is editable. |
| `0x000C` | `RegistryAuthority` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | A string that specifies the authority which issued a registry or catalog number. Some examples of registry authorities are CAS, Beilstein, Aldrich, and Merck. Official lexical/binary codec is available; value is editable. |
| `0x000E` | `RepresentsProperty` | `CDXRepresentsProperty` | `native-semantic/value` | `verified` | `verified` | `verified` | Indicates that this object represents some property in some other object. Official lexical/binary codec is available; value is editable. |
| `0x000F` | `IgnoreWarnings` | `CDXBooleanImplied` | `native-semantic/value` | `verified` | `verified` | `verified` | Signifies whether chemical warnings should be suppressed on this object. Official lexical/binary codec is available; value is editable. |
| `0x0010` | `Warning` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | A warning concerning possible chemical problems with this object. Official lexical/binary codec is available; value is editable. |
| `0x0011` | `Visible` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | The object is visible if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0012` | `SupersededBy` | `CDXObjectID` | `typed-interchange/value` | `verified` | `verified` | `verified` | The ID of the object that should be read instead of this one. Official lexical/binary codec is available; value is editable. |
| `0x0100` | `fonttable` | `CDXFontTable` | `native-semantic/children` | `verified` | `verified` | `verified` | Required if fonts are used. A list of fonts used in the document. Edit structured font children and explicit native text styles. |
| `0x0200` | `p` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The 2D location (in the order of vertical and horizontal locations) of an object. Official lexical/binary codec is available; value is editable. |
| `0x0201` | `xyz` | `CDXPoint3D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The 3D location (in the order of X-, Y-, and Z-locations in right-handed coordinate system) of an object in CDX coordinate units. The precise meaning of this attribute varies depending on the type of object. Official lexical/binary codec is available; value is editable. |
| `0x0202` | `extent` | `CDXPoint2D` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for templategrids. The width and height of an object in CDX coordinate units. The precise meaning of this attribute varies depending on the type of object. Official point codec is available; TemplateGrid consumes the pair as native cell width and height. |
| `0x0203` | `extent3D` | `CDXPoint3D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The width, height, and depth of an object in CDX coordinate units (right-handed coordinate system). The precise meaning of this attribute varies depending on the type of object. Official lexical/binary codec is available; value is editable. |
| `0x0204` | `BoundingBox` | `CDXRectangle` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for pictures and spectra. Required for graphics and text until 6.0. The smallest rectangle that encloses the graphical representation of the object. Official lexical/binary codec is available; value is editable. |
| `0x0205` | `RotationAngle` | `INT32` | `typed-interchange/value` | `verified` | `verified` | `verified` | The angular orientation of an object in degrees * 65536. Official lexical/binary codec is available; value is editable. |
| `0x0207` | `Head3D` | `CDXPoint3D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The 3D location (in the order of X-, Y-, and Z-locations in right-handed coordinate system) of the head of an object in CDX coordinate units. Official lexical/binary codec is available; value is editable. |
| `0x0208` | `Tail3D` | `CDXPoint3D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The 3D location (in the order of X-, Y-, and Z-locations in right-handed coordinate system) of the tail of an object in CDX coordinate units. Official lexical/binary codec is available; value is editable. |
| `0x0209` | `TopLeft` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The location of the top-left corner of a quadrilateral object, possibly in a rotated or skewed frame. Official lexical/binary codec is available; value is editable. |
| `0x020A` | `TopRight` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The location of the top-right corner of a quadrilateral object, possibly in a rotated or skewed frame. Official lexical/binary codec is available; value is editable. |
| `0x020B` | `BottomRight` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The location of the bottom-right corner of a quadrilateral object, possibly in a rotated or skewed frame. Official lexical/binary codec is available; value is editable. |
| `0x020C` | `BottomLeft` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The location of the bottom-left corner of a quadrilateral object, possibly in a rotated or skewed frame. Official lexical/binary codec is available; value is editable. |
| `0x020D` | `Center3D` | `CDXPoint3D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The 3D location of the logical center of an object. Official lexical/binary codec is available; value is editable. |
| `0x020E` | `MajorAxisEnd3D` | `CDXPoint3D` | `typed-interchange/value` | `verified-with-erratum` | `verified` | `verified` | The 3D location of the end of the major axis of an object in CDX coordinate units. Official lexical/binary codec is available; value is editable. |
| `0x020F` | `MinorAxisEnd3D` | `CDXPoint3D` | `typed-interchange/value` | `verified-with-erratum` | `verified` | `verified` | The 3D location of the end of the minor axis of an object in CDX coordinate units. Official lexical/binary codec is available; value is editable. |
| `0x0300` | `colortable` | `CDXColorTable` | `native-semantic/children` | `verified` | `verified` | `verified` | The color palette used throughout the document. Edit structured color children and explicit native colors. |
| `0x0301` | `color` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The foreground color of an object represented as the two-based index into the object's color table. Official lexical/binary codec is available; value is editable. |
| `0x0302` | `bgcolor` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The background color of an object represented as the two-based index into the object's color table. Official lexical/binary codec is available; value is editable. |
| `0x0400` | `NodeType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of a node object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0401` | `LabelDisplay` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The characteristics of node label display. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0402` | `Element` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The atomic number of the atom representing this node. Official lexical/binary codec is available; value is editable. |
| `0x0403` | `ElementList` | `CDXElementList` | `native-semantic/value` | `verified` | `verified` | `verified` | A list of atomic numbers. Official lexical/binary codec is available; value is editable. |
| `0x0404` | `Formula` | `CDXFormula` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The composition of a node representing a fragment whose composition is known, but whose connectivity is not. For example, C 4 H 9 represents a mixture of the 4 butyl isomers. Official data type is reserved/undefined; ChemDraw does not read or write it. |
| `0x0420` | `Isotope` | `INT16` | `native-semantic/value` | `verified` | `verified` | `verified` | The absolute isotopic mass of an atom (2 for deuterium, 14 for carbon-14). Official lexical/binary codec is available; value is editable. |
| `0x0421` | `Charge` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The atomic charge of an atom. Official lexical/binary codec is available; value is editable. |
| `0x0422` | `Radical` | `UINT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The atomic radical attribute of an atom. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0423` | `FreeSites` | `UINT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Indicates that up to the specified number of additional substituents are permitted on this atom. Official lexical/binary codec is available; value is editable. |
| `0x0424` | `ImplicitHydrogens` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies that implicit hydrogens are not allowed on this atom. Official lexical/binary codec is available; value is editable. |
| `0x0425` | `RingBondCount` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The number of ring bonds attached to an atom. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0426` | `UnsaturatedBonds` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Indicates whether unsaturation should be present or absent. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0427` | `RxnChange` | `CDXBooleanImplied` | `native-semantic/value` | `verified` | `verified` | `verified` | If present, signifies that the reaction change of an atom must be as specified. Official lexical/binary codec is available; value is editable. |
| `0x0428` | `RxnStereo` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The change of stereochemistry of an atom during a reaction. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0429` | `AbnormalValence` | `CDXBooleanImplied` | `native-semantic/value` | `verified` | `verified` | `verified` | Signifies that an abnormal valence for an atom is permitted. Official lexical/binary codec is available; value is editable. |
| `0x042B` | `NumHydrogens` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The number of (explicit) hydrogens in a labeled atom consisting of one heavy atom and (optionally) the symbol H (e.g., CH 3 ). Official lexical/binary codec is available; value is editable. |
| `0x042E` | `HDot` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies the presence of an implicit hydrogen with stereochemistry specified equivalent to an explicit H atom with a wedged bond. Official lexical/binary codec is available; value is editable. |
| `0x042F` | `HDash` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies the presence of an implicit hydrogen with stereochemistry specified equivalent to an explicit H atom with a hashed bond. Official lexical/binary codec is available; value is editable. |
| `0x0430` | `Geometry` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The geometry of the bonds about this atom. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0431` | `BondOrdering` | `CDXObjectIDArray` | `typed-interchange/value` | `verified` | `verified` | `verified` | An ordering of the bonds to this node, used for stereocenters, fragments, and named alternative groups with more than one attachment. Official lexical/binary codec is available; value is editable. |
| `0x0432` | `Attachments` | `CDXObjectIDArrayWithCounts` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for multi- and variable attached nodes. For multicenter attachment nodes or variable attachment nodes, a list of IDs of the nodes which are multiply or variably attached to this node. Official lexical/binary codec is available; value is editable. |
| `0x0433` | `GenericNickname` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | The name of the generic nickname. Official lexical/binary codec is available; value is editable. |
| `0x0434` | `AltGroupID` | `CDXObjectID` | `native-semantic/value` | `verified` | `verified` | `verified` | The ID of the alternative group object that describes this node. Official lexical/binary codec is available; value is editable. |
| `0x0435` | `SubstituentsUpTo` | `UINT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Indicates that substitution is restricted to no more than the specified value. Official lexical/binary codec is available; value is editable. |
| `0x0436` | `SubstituentsExactly` | `UINT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Indicates that exactly the specified number of substituents must be present. Official lexical/binary codec is available; value is editable. |
| `0x0437` | `AS` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The node's absolute stereochemistry according to the Cahn-Ingold-Prelog system. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0438` | `Translation` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Provides for restrictions on whether a given node may match other more- or less-general nodes. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0439` | `AtomNumber` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | Atom number, as text. Official lexical/binary codec is available; value is editable. |
| `0x043A` | `ShowAtomQuery` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Show the query indicator if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x043B` | `ShowAtomStereo` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | Show the stereochemistry indicator if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x043C` | `ShowAtomNumber` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | Show the atom number if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x043D` | `LinkCountLow` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | Low end of repeat count for link nodes. Official lexical/binary codec is available; value is editable. |
| `0x043E` | `LinkCountHigh` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | High end of repeat count for link nodes. Official lexical/binary codec is available; value is editable. |
| `0x043F` | `IsotopicAbundance` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Isotopic abundance of this atom's isotope. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0440` | `ExternalConnectionType` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | Type of external connection, for atoms of type kCDXNodeType_ExternalConnectionPoint. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0441` | `GenericList` | `CDXGenericList` | `native-semantic/value` | `verified` | `verified` | `verified` | A list of generic nicknames. Official lexical/binary codec is available; value is editable. |
| `0x0442` | `ShowTerminalCarbonLabels` | `CDXBooleanImplied` | `native-semantic/value` | `verified` | `verified` | `verified` | Signifies whether terminal carbons (carbons with zero or one bond) should display a text label with the element symbol and appropriate hydrogens. Official lexical/binary codec is available; value is editable. |
| `0x0443` | `ShowNonTerminalCarbonLabels` | `CDXBooleanImplied` | `native-semantic/value` | `verified` | `verified` | `verified` | Signifies whether non-terminal carbons (carbons with more than one bond) should display a text label with the element symbol and appropriate hydrogens. Official lexical/binary codec is available; value is editable. |
| `0x0444` | `HideImplicitHydrogens` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether implicit hydrogens should be displayed on otherwise-atomic atom labels (NH2 versus N). Official lexical/binary codec is available; value is editable. |
| `0x0445` | `ShowAtomEnhancedStereo` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Show the enhanced stereochemistry indicator if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0446` | `EnhancedStereoType` | `UINT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of enhanced stereochemistry present on this atom. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0447` | `EnhancedStereoGroupNum` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The group number associated with Or and And enhanced stereochemistry types. Official lexical/binary codec is available; value is editable. |
| `0x0500` | `Racemic` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Indicates that the molecule is a racemic mixture. Official lexical/binary codec is available; value is editable. |
| `0x0501` | `Absolute` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Indicates that the molecule has known absolute configuration. Official lexical/binary codec is available; value is editable. |
| `0x0502` | `Relative` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Indicates that the molecule has known relative stereochemistry, but unknown absolute configuration. Official lexical/binary codec is available; value is editable. |
| `0x0503` | `Formula` | `CDXFormula` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The molecular formula representation of a molecule object. Official data type is reserved/undefined; ChemDraw does not read or write it. |
| `0x0504` | `Weight` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | The average molecular weight of a molecule object. Official lexical/binary codec is available; value is editable. |
| `0x0505` | `ConnectionOrder` | `CDXObjectIDArray` | `typed-interchange/value` | `verified` | `verified` | `verified` | An ordered list of attachment points within a fragment. Official lexical/binary codec is available; value is editable. |
| `0x0600` | `Order` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The order of a bond object. This is a bit-encoded property. Official lexical/binary codec is available; value is editable. |
| `0x0601` | `Display` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The display type of a bond object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0602` | `Display2` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The display type for the second line of a double bond. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0603` | `DoublePosition` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The position of the second line of a double bond. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0604` | `B` | `CDXObjectID` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for bonds. The ID of the CDX node object at the first end of a bond. Official lexical/binary codec is available; value is editable. |
| `0x0605` | `E` | `CDXObjectID` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for bonds. The ID of the CDX node object at the second end of a bond. Official lexical/binary codec is available; value is editable. |
| `0x0606` | `Topology` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Indicates the desired topology of a bond in a query. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0607` | `RxnParticipation` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | Specifies that a bond is affected by a reaction. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0608` | `BeginAttach` | `UINT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | Indicates where within the Bond_Begin node a bond is attached. Official lexical/binary codec is available; value is editable. |
| `0x0609` | `EndAttach` | `UINT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | Indicates where within the Bond_End node a bond is attached. Official lexical/binary codec is available; value is editable. |
| `0x060A` | `BS` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The bond's absolute stereochemistry according to the Cahn-Ingold-Prelog system. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x060B` | `BondCircularOrdering` | `CDXObjectIDArray` | `typed-interchange/value` | `verified` | `verified` | `verified` | Ordered list of attached bond IDs. Official lexical/binary codec is available; value is editable. |
| `0x060C` | `ShowBondQuery` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | Show the query indicator if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x060D` | `ShowBondStereo` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | Show the stereochemistry indicator if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x060E` | `CrossingBonds` | `CDXObjectIDArray` | `typed-interchange/value` | `verified` | `verified` | `verified` | The set of bonds that cross a given bond. Official lexical/binary codec is available; value is editable. |
| `0x060F` | `ShowBondRxn` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | Show the reaction-change indicator if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0700` | `(not used)` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for text objects. The text of a text object. Official lexical/binary codec is available; value is editable. |
| `0x0701` | `Justification` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The horizontal justification of a text object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0702` | `LineHeight` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The line height of a text object. Official lexical/binary codec is available; value is editable. |
| `0x0703` | `WordWrapWidth` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The word-wrap width of a text object. Official lexical/binary codec is available; value is editable. |
| `0x0704` | `LineStarts` | `INT16ListWithCounts` | `typed-interchange/value` | `verified` | `verified` | `verified` | Zero-based styled-text positions. The count prefix exists only in binary CDX; CDXML contains positions only. Positions materialize soft wraps only with `WordWrapWidth`; atom-label values are derived chemical-layout boundaries and do not rewrite the source formula. Official codec, ChemDraw 22.2 silent probes, and public-corpus role separation are verified. |
| `0x0705` | `LabelAlignment` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The alignment of the text with respect to the node position. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0706` | `LabelLineHeight` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | Text line height for atom labels Official lexical/binary codec is available; value is editable. |
| `0x0707` | `CaptionLineHeight` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | Text line height for non-atomlabel text objects Official lexical/binary codec is available; value is editable. |
| `0x0708` | `InterpretChemically` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether to the text label should be interpreted chemically (if possible). Official lexical/binary codec is available; value is editable. |
| `0x0800` | `MacPrintInfo` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The 120 byte Macintosh TPrint data associated with the CDX document object. Refer to Macintosh Toolbox manual for detailed description. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0801` | `WinPrintInfo` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The Windows DEVMODE structure associated with the CDX document object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0802` | `PrintMargins` | `CDXRectangle` | `typed-interchange/value` | `verified` | `verified` | `verified` | The outer margins of the Document. Official lexical/binary codec is available; value is editable. |
| `0x0803` | `ChainAngle` | `INT32` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default chain angle setting in degrees * 65536. Official lexical/binary codec is available; value is editable. |
| `0x0804` | `BondSpacing` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The spacing between segments of a multiple bond, measured relative to bond length. Official lexical/binary codec is available; value is editable. |
| `0x0805` | `BondLength` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default bond length. Official lexical/binary codec is available; value is editable. |
| `0x0806` | `BoldWidth` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default bold bond width. Official lexical/binary codec is available; value is editable. |
| `0x0807` | `LineWidth` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default line width. Official lexical/binary codec is available; value is editable. |
| `0x0808` | `MarginWidth` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default amount of space surrounding atom labels. Official lexical/binary codec is available; value is editable. |
| `0x0809` | `HashSpacing` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default spacing between hashed lines used in wedged hashed bonds. Official lexical/binary codec is available; value is editable. |
| `0x080A` | `(not used)` | `CDXFontStyle` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The default style for atom labels.. Official lexical/binary codec is available; value is editable. |
| `0x080B` | `(not used)` | `CDXFontStyle` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The default style for non-atomlabel text objects.. Official lexical/binary codec is available; value is editable. |
| `0x080C` | `CaptionJustification` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The horizontal justification of a caption (non-atomlabel text object) This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x080D` | `FractionalWidths` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether to use fractional width information when drawing text. Official lexical/binary codec is available; value is editable. |
| `0x080E` | `Magnification` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The view magnification factor Official lexical/binary codec is available; value is editable. |
| `0x080F` | `WidthPages` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The width of the document in pages. Official lexical/binary codec is available; value is editable. |
| `0x0810` | `HeightPages` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The height of the document in pages. Official lexical/binary codec is available; value is editable. |
| `0x0811` | `DrawingSpace` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of drawing space used for this document. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0812` | `Width` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The width of an object in CDX coordinate units, possibly in a rotated or skewed frame. Official lexical/binary codec is available; value is editable. |
| `0x0813` | `Height` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The height of an object in CDX coordinate units, possibly in a rotated or skewed frame. Official lexical/binary codec is available; value is editable. |
| `0x0814` | `PageOverlap` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The amount of overlap of pages when a poster is tiled. Official lexical/binary codec is available; value is editable. |
| `0x0815` | `Header` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | The text of the header. Official lexical/binary codec is available; value is editable. |
| `0x0816` | `HeaderPosition` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The vertical offset of the header baseline from the top of the page. Official lexical/binary codec is available; value is editable. |
| `0x0817` | `Footer` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | The text of the footer. Official lexical/binary codec is available; value is editable. |
| `0x0818` | `FooterPosition` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The vertical offset of the footer baseline from the bottom of the page. Official lexical/binary codec is available; value is editable. |
| `0x0819` | `PrintTrimMarks` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | If present, trim marks are to printed in the margins. Official lexical/binary codec is available; value is editable. |
| `0x081A` | `LabelFont` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default font family for atom labels. Official lexical/binary codec is available; value is editable. |
| `0x081B` | `CaptionFont` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default font style for captions (non-atom-label text objects). Official lexical/binary codec is available; value is editable. |
| `0x081C` | `LabelSize` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default font size for atom labels. Official lexical/binary codec is available; value is editable. |
| `0x081D` | `CaptionSize` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default font size for captions (non-atom-label text objects). Official lexical/binary codec is available; value is editable. |
| `0x081E` | `LabelFace` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default font style for atom labels. Official lexical/binary codec is available; value is editable. |
| `0x081F` | `CaptionFace` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default font face for captions (non-atom-label text objects). Official lexical/binary codec is available; value is editable. |
| `0x0820` | `LabelColor` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default color for atom labels Official lexical/binary codec is available; value is editable. |
| `0x0821` | `CaptionColor` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default color for captions (non-atom-label text objects). Official lexical/binary codec is available; value is editable. |
| `0x0822` | `BondSpacingAbs` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The absolute distance between segments of a multiple bond. Official lexical/binary codec is available; value is editable. |
| `0x0823` | `LabelJustification` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The default justification for atom labels. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0824` | `FixInPlaceExtent` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | Defines a size for OLE In-Place editing. Official lexical/binary codec is available; value is editable. |
| `0x0825` | `Side` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required. A specific side of an object (rectangle). This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0826` | `FixInPlaceGap` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | Defines a padding for OLE In-Place editing. Official lexical/binary codec is available; value is editable. |
| `0x0827` | `CartridgeData` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | Transient data used by the CambridgeSoft Oracle Cartridge. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0900` | `WindowIsZoomed` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether the main viewing window is zoomed (maximized). Official lexical/binary codec is available; value is editable. |
| `0x0901` | `WindowPosition` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The top-left position of the main viewing window. Official lexical/binary codec is available; value is editable. |
| `0x0902` | `WindowSize` | `CDXPoint2D` | `typed-interchange/value` | `verified` | `verified` | `verified` | Height and width of the document window. Official lexical/binary codec is available; value is editable. |
| `0x0A00` | `GraphicType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of graphical object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A01` | `LineType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of a line object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A02` | `ArrowType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of arrow object, which represents line, arrow, arc, rectangle, or orbital. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A03` | `RectangleType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of a rectangle object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A04` | `OvalType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of an arrow object that represents a circle or ellipse. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A05` | `OrbitalType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of orbital object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A06` | `BracketType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of symbol object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A07` | `SymbolType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of symbol object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A08` | `CurveType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of curve object. This is a bit-encoded property. Official lexical/binary codec is available; value is editable. |
| `0x0A20` | `HeadSize` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The size of the arrow's head. Official lexical/binary codec is available; value is editable. |
| `0x0A21` | `AngularSize` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The size of an arc (in degrees * 10, so 90 degrees = 900). Official lexical/binary codec is available; value is editable. |
| `0x0A22` | `LipSize` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The size of a bracket. Official lexical/binary codec is available; value is editable. |
| `0x0A23` | `CurvePoints` | `CDXCurvePoints` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for curves. The B&eacute;zier curve's control point locations. Official lexical/binary codec is available; value is editable. |
| `0x0A24` | `BracketUsage` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The syntactical chemical meaning of the bracket (SRU, mer, mon, xlink, etc). This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A25` | `PolymerRepeatPattern` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The head-to-tail connectivity of objects contained within the bracket. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A26` | `PolymerFlipType` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | The flip state of objects contained within the bracket. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A27` | `BracketedObjectIDs` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | The set of objects contained in a BracketedGroup. Official lexical/binary codec is available; value is editable. |
| `0x0A28` | `RepeatCount` | `FLOAT64` | `native-semantic/value` | `verified` | `verified` | `verified` | The number of times a multiple-group BracketedGroup is repeated. Official lexical/binary codec is available; value is editable. |
| `0x0A29` | `ComponentOrder` | `INT16` | `native-semantic/value` | `verified` | `verified` | `verified` | The component order associated with a BracketedGroup. Official lexical/binary codec is available; value is editable. |
| `0x0A2A` | `SRULabel` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | The label associated with a BracketedGroup that represents an SRU. Official lexical/binary codec is available; value is editable. |
| `0x0A2B` | `GraphicID` | `CDXObjectID` | `native-semantic/value` | `verified` | `verified` | `verified` | The ID of a graphical object (bracket, brace, or parenthesis) associated with a Bracket Attachment. Official lexical/binary codec is available; value is editable. |
| `0x0A2C` | `BondID` | `CDXObjectID` | `native-semantic/value` | `verified` | `verified` | `verified` | Required. The ID of a bond that crosses a Bracket Attachment. Official lexical/binary codec is available; value is editable. |
| `0x0A2D` | `InnerAtomID` | `CDXObjectID` | `native-semantic/value` | `verified` | `verified` | `verified` | Required. The ID of the node located within the Bracketed Group and attached to a bond that crosses a Bracket Attachment. Official lexical/binary codec is available; value is editable. |
| `0x0A2E` | `CurvePoints3D` | `CDXCurvePoints3D` | `typed-interchange/value` | `verified` | `verified` | `verified` | The B&eacute;zier curve's control point locations in 3D space. Official lexical/binary codec is available; value is editable. |
| `0x0A2F` | `ArrowHeadType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of arrowhead. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A30` | `HeadCenterSize` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The size of the arrow's head from the tip to the back of the head. Official lexical/binary codec is available; value is editable. |
| `0x0A31` | `HeadWidth` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The half-width of the arrow's head. Official lexical/binary codec is available; value is editable. |
| `0x0A32` | `ShadowSize` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The size of the object's shadow. Official lexical/binary codec is available; value is editable. |
| `0x0A33` | `ArrowShaftSpacing` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The width of the space between a multiple-component arrow shaft, as in an equilibrium arrow. Official lexical/binary codec is available; value is editable. |
| `0x0A34` | `ArrowEquilibriumRatio` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The ratio of the length of the left component of an equilibrium arrow (viewed from the end to the start) to the right component. Official lexical/binary codec is available; value is editable. |
| `0x0A35` | `ArrowHeadHead` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of arrowhead at the head of the arrow. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A36` | `ArrowHeadTail` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of arrowhead at the tail of the arrow. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A37` | `FillType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of the fill, for objects that can be filled. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A38` | `CurveSpacing` | `UINT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The width of the space between a a Doubled curve. Official lexical/binary codec is available; value is editable. |
| `0x0A39` | `Closed` | `CDXBooleanImplied` | `typed-interchange/value` | `verified-with-erratum` | `verified` | `verified` | Signifies whether object is closed. Official lexical/binary codec is available; value is editable. |
| `0x0A3A` | `Dipole` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether the arrow is a dipole arrow. Official lexical/binary codec is available; value is editable. |
| `0x0A3B` | `NoGo` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether arrow is a no-go arrow, and the type of no-go (crossed-through or hashed-out) if so. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A3C` | `CornerRadius` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | Official description: radius of a rounded rectangle corner. ChemDraw SVG probes verify the behavioral unit: positive values are hundredths-encoded multipliers of the graphic's normal `LineWidth`; missing/zero selects `600`; `BoldWidth` affects the outline but not this basis. ChemSema stores the resolved absolute radius and reverses the conversion on export. |
| `0x0A3D` | `FrameType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of frame on an object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A60` | `Edition` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The section information (SectionHandle) of the Macintosh Publish & Subscribe edition embedded in the CDX picture object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A61` | `EditionAlias` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | The alias information of the Macintosh Publish & Subscribe edition embedded in the CDX picture object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A62` | `MacPICT` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A Macintosh PICT data object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A63` | `WindowsMetafile` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A Microsoft Windows Metafile object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A64` | `OLEObject` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | An OLE object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A65` | `EnhancedMetafile` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A Microsoft Windows Enhanced Metafile object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A6E` | `GIF` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A binary GIF data object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A6F` | `TIFF` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A binary TIFF data object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A70` | `PNG` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A binary PNG data object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A71` | `JPEG` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A binary JPEG data object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A72` | `BMP` | `Unformatted` | `opaque-by-spec/rawBase64` | `verified` | `verified` | `verified` | A binary BMP data object. Official type is uninterpreted bytes; rawBase64 is authoritative. |
| `0x0A80` | `XSpacing` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for spectra. The spacing in logical units (ppm, Hz, wavenumbers) between points along the X-axis of an evenly-spaced grid. Official lexical/binary codec is available; value is editable. |
| `0x0A81` | `XLow` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for spectra. The first data point for the X-axis of an evenly-spaced grid. Official lexical/binary codec is available; value is editable. |
| `0x0A82` | `XType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of units the X-axis represents. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A83` | `YType` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of units the Y-axis represents. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A84` | `XAxisLabel` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | A label for the X-axis. Official lexical/binary codec is available; value is editable. |
| `0x0A85` | `YAxisLabel` | `CDXString` | `typed-interchange/value` | `verified` | `verified` | `verified` | A label for the Y-axis. Official lexical/binary codec is available; value is editable. |
| `0x0A86` | `(not used)` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | Required for spectra. The Y-axis values for the spectrum. It is an array of double values corresponding to X-axis values. Official lexical/binary codec is available; value is editable. |
| `0x0A87` | `Class` | `INT16` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of spectrum represented. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0A88` | `YLow` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | Y value to be used to offset data when storing XML. Official lexical/binary codec is available; value is editable. |
| `0x0A89` | `YScale` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | Y scaling used to scale data when storing XML. Official lexical/binary codec is available; value is editable. |
| `0x0AA0` | `OriginFraction` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | The distance of the origin line from the bottom of a TLC Plate, as a fraction of the total height of the plate. Official lexical/binary codec is available; value is editable. |
| `0x0AA1` | `SolventFrontFraction` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | The distance of the solvent front from the top of a TLC Plate, as a fraction of the total height of the plate. Official lexical/binary codec is available; value is editable. |
| `0x0AA2` | `ShowOrigin` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Show the origin line near the base of the TLC Plate if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0AA3` | `ShowSolventFront` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Show the solvent front line near the top of the TLC Plate if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0AA4` | `ShowBorders` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Show borders around the edges of the TLC Plate if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0AA5` | `ShowSideTicks` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Show tickmarks up the side of the TLC Plate if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0AB0` | `Rf` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | The Retention Factor (R f ) of an individual spot. Official lexical/binary codec is available; value is editable. |
| `0x0AB1` | `Tail` | `CDXCoordinate` | `typed-interchange/value` | `verified` | `verified` | `verified` | The length of the "tail" of an individual spot. Official lexical/binary codec is available; value is editable. |
| `0x0AB2` | `ShowRf` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | Show the spot's Retention Fraction (R f ) value if non-zero.. Official lexical/binary codec is available; value is editable. |
| `0x0B00` | `TextFrame` | `CDXRectangle` | `native-semantic/value` | `verified` | `verified` | `verified` | The bounding box of upper portion of the Named Alternative Group, containing the name of the group. Official lexical/binary codec is available; value is editable. |
| `0x0B01` | `GroupFrame` | `CDXRectangle` | `native-semantic/value` | `verified` | `verified` | `verified` | The bounding box of the lower portion of the Named Alternative Group, containing the definition of the group. Official lexical/binary codec is available; value is editable. |
| `0x0B02` | `Valence` | `INT16` | `native-semantic/value` | `verified` | `verified` | `verified` | The number of attachment points in each alternative in a named alternative group. Official lexical/binary codec is available; value is editable. |
| `0x0B80` | `GeometricFeature` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The type of the geometrical feature (point, line, plane, etc.). This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0B81` | `RelationValue` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | The numeric relationship (if any) among the basis objects used to define this object. Official lexical/binary codec is available; value is editable. |
| `0x0B82` | `BasisObjects` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for geometries and constraints. An ordered list of objects used to define this object. Official lexical/binary codec is available; value is editable. |
| `0x0B83` | `ConstraintType` | `INT8` | `typed-interchange/value` | `verified` | `verified` | `verified` | The constraint type (distance, angle, or exclusion sphere). This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0B84` | `ConstraintMin` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | The minimum value of the constraint. Official lexical/binary codec is available; value is editable. |
| `0x0B85` | `ConstraintMax` | `FLOAT64` | `typed-interchange/value` | `verified` | `verified` | `verified` | The maximum value of the constraint. Official lexical/binary codec is available; value is editable. |
| `0x0B86` | `IgnoreUnconnectedAtoms` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether unconnected atoms should be ignored within the exclusion sphere. Official lexical/binary codec is available; value is editable. |
| `0x0B87` | `DihedralIsChiral` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | Signifies whether a dihedral is signed or unsigned. Official lexical/binary codec is available; value is editable. |
| `0x0B88` | `PointIsDirected` | `CDXBooleanImplied` | `typed-interchange/value` | `verified` | `verified` | `verified` | For a point based on a normal, signifies whether it is in a specific direction relative to the reference point. Official lexical/binary codec is available; value is editable. |
| `0x0BB0` | `ChemicalPropertyType` | `UINT32` | `native-semantic/value` | `verified` | `verified` | `verified` | The type of property (name, formula, molecular weight, etc.). This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0BB1` | `ChemicalPropertyDisplayID` | `CDXObjectID` | `native-semantic/value` | `verified` | `verified` | `verified` | The ID of a graphical object used to display the property value. Official lexical/binary codec is available; value is editable. |
| `0x0BB2` | `ChemicalPropertyIsActive` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | Whether the property should be recalculated in response to changes in the basis objects. Official lexical/binary codec is available; value is editable. |
| `0x0C00` | `ReactionStepAtomMap` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | Represents pairs of mapped atom IDs; each pair is a reactant atom mapped to to a product atom. Official lexical/binary codec is available; value is editable. |
| `0x0C01` | `ReactionStepReactants` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | An order list of reactants present in the Reaction Step. Official lexical/binary codec is available; value is editable. |
| `0x0C02` | `ReactionStepProducts` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | An order list of products present in the Reaction Step. Official lexical/binary codec is available; value is editable. |
| `0x0C03` | `ReactionStepPlusses` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | An ordered list of pluses used to separate components of the Reaction Step. Official lexical/binary codec is available; value is editable. |
| `0x0C04` | `ReactionStepArrows` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | An ordered list of arrows used to separate components of the Reaction Step. Official lexical/binary codec is available; value is editable. |
| `0x0C05` | `ReactionStepObjectsAboveArrow` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | An order list of objects above the arrow in the Reaction Step. Official lexical/binary codec is available; value is editable. |
| `0x0C06` | `ReactionStepObjectsBelowArrow` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | An order list of objects below the arrow in the Reaction Step. Official lexical/binary codec is available; value is editable. |
| `0x0C07` | `ReactionStepAtomMapManual` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | Represents pairs of mapped atom IDs; each pair is a reactant atom mapped to to a product atom. Official lexical/binary codec is available; value is editable. |
| `0x0C08` | `ReactionStepAtomMapAuto` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | Represents pairs of mapped atom IDs; each pair is a reactant atom mapped to to a product atom. Official lexical/binary codec is available; value is editable. |
| `0x0D00` | `TagType` | `INT16` | `native-semantic/value` | `verified` | `verified` | `verified` | The tag's data type. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0D03` | `Tracking` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | The tag will participate in tracking if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0D04` | `Persistent` | `CDXBoolean` | `native-semantic/value` | `verified` | `verified` | `verified` | The tag will be resaved to a CDX file if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x0D05` | `Value` | `varies` | `native-semantic/value` | `verified` | `verified` | `verified` | The value is a INT32, FLOAT64 or unformatted string depending on the value of ObjectTag_Type. Decode and edit the explicit native value according to the containing ObjectTag TagType. |
| `0x0D06` | `PositioningType` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | How the object should be positioned with respect to its containing object. This is an enumerated property. Official lexical/binary codec is available; value is editable. |
| `0x0D07` | `PositioningAngle` | `INT32` | `native-semantic/value` | `verified` | `verified` | `verified` | Angular positioning, in degrees * 65536. Official lexical/binary codec is available; value is editable. |
| `0x0D08` | `PositioningOffset` | `CDXPoint2D` | `native-semantic/value` | `verified` | `verified` | `verified` | Offset positioning. Official lexical/binary codec is available; value is editable. |
| `0x0E00` | `SequenceIdentifier` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for sequences. A unique (but otherwise random) identifier for a given Sequence object. Official lexical/binary codec is available; value is editable. |
| `0x0F00` | `CrossReferenceContainer` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | An external object containing (as an embedded object) the document containing the Sequence object being referenced. Official lexical/binary codec is available; value is editable. |
| `0x0F01` | `CrossReferenceDocument` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | An external document containing the Sequence object being referenced. Official lexical/binary codec is available; value is editable. |
| `0x0F02` | `CrossReferenceIdentifier` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for cross-references.. A unique (but otherwise random) identifier for a given Cross-Reference object. Official lexical/binary codec is available; value is editable. |
| `0x0F03` | `CrossReferenceSequence` | `CDXString` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for cross-references.. A value matching the SequenceIdentifier of the Sequence object to be referenced. Official lexical/binary codec is available; value is editable. |
| `0x1000` | `PaneHeight` | `CDXCoordinate` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for templategrids. The height of the viewing window of a template grid. TemplateGrid pane height is decoded as official 16.16 CDXCoordinate and edited through the native layout model. |
| `0x1001` | `NumRows` | `INT16` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for templategrids. The number of rows of the CDX TemplateGrid object. TemplateGrid dimension is validated and edited through the native layout model. |
| `0x1002` | `NumColumns` | `INT16` | `native-semantic/value` | `verified` | `verified` | `verified` | Required for templategrids. The number of columns of the CDX TemplateGrid object. TemplateGrid dimension is validated and edited through the native layout model. |
| `0x1100` | `Integral` | `CDXBoolean` | `typed-interchange/value` | `verified` | `verified` | `verified` | The group is considered to be integral (non-subdivisible) if non-zero. Official lexical/binary codec is available; value is editable. |
| `0x1FF0` | `SplitterPositions` | `CDXObjectIDArray` | `native-semantic/value` | `verified` | `verified` | `verified` | An array of vertical positions that subdivide a page into regions. Decode as the official CDXObjectIDArray. Preserve this ChemDraw 6 compatibility field as explicit legacy object IDs; it is not a coordinate array. |
| `0x1FF1` | `PageDefinition` | `INT8` | `native-semantic/value` | `verified` | `verified` | `verified` | A description of the type of formatting used by the page, or by the splitter. This is an enumerated property. Decode as the official INT8 page/splitter formatting enum with values Undefined through UserDefined. |

## CDXML 元素全集

| 元素 | 内容模型 | 属性数 |
| --- | --- | ---: |
| `altgroup` | `(objecttag \| annotation \| t \| fragment \| group \| graphic \| bracket)+` | 13 |
| `annotation` | `EMPTY` | 3 |
| `arrow` | `(objecttag \| annotation)*` | 36 |
| `b` | `(objecttag \| annotation)*` | 38 |
| `bioshape` | `(objecttag \| annotation \| curve)*` | 43 |
| `border` | `EMPTY` | 7 |
| `bracketattachment` | `(crossingbond)*` | 2 |
| `bracketedgroup` | `(bracketattachment+,bracketedgroup*)` | 8 |
| `CDXML` | `(colortable?,fonttable?,page+,templategrid?)` | 89 |
| `chemicalproperty` | `EMPTY` | 11 |
| `color` | `EMPTY` | 3 |
| `coloredmoleculararea` | `EMPTY` | 3 |
| `colortable` | `(color+)` | 1 |
| `constraint` | `(objecttag \| annotation)*` | 16 |
| `crossingbond` | `EMPTY` | 3 |
| `crossreference` | `(t*)` | 4 |
| `curve` | `(objecttag \| annotation)*` | 26 |
| `embeddedobject` | `(objecttag \| annotation)*` | 25 |
| `font` | `EMPTY` | 3 |
| `fonttable` | `(font+)` | 1 |
| `fragment` | `((n \| b \| t \| graphic \| curve \| objecttag \| annotation \| regnum \| coloredmoleculararea)*)` | 10 |
| `geometry` | `(objecttag \| annotation)*` | 11 |
| `gepband` | `(annotation \| embeddedobject \| marker)*` | 10 |
| `geplane` | `((annotation \| gepband \| t)*)` | 3 |
| `gepplate` | `((annotation \| geplane)*)` | 27 |
| `graphic` | `(objecttag \| annotation \| represent \| t)*` | 38 |
| `group` | `((t \| fragment \| group \| graphic \| altgroup \| curve \| step \| scheme \| spectrum \| objecttag \| annotation \| plasmidmap \| rlogic \| arrow \| bioshape)*)` | 4 |
| `marker` | `(annotation \| t \| curve)*` | 10 |
| `n` | `(objecttag \| annotation \| t \| fragment)*` | 61 |
| `objecttag` | `(t*)` | 11 |
| `page` | `((t \| fragment \| group \| graphic \| altgroup \| curve \| step \| scheme \| spectrum \| embeddedobject \| sequence \| crossreference \| splitter \| table \| bracketedgroup \| border \| geometry \| constraint \| tlcplate \| gepplate \| chemicalproperty \| arrow \| bioshape \| stoichiometrygrid \| plasmidmap \| objecttag \| annotation \| rlogic)*)` | 21 |
| `plasmidmap` | `((objecttag \| annotation \| plasmidregion \| plasmidmarker \| t \| graphic)*)` | 16 |
| `plasmidmarker` | `(objecttag \| annotation \| t \| curve)*` | 10 |
| `plasmidregion` | `(objecttag \| annotation \| plasmidmarker)*` | 24 |
| `regnum` | `EMPTY` | 3 |
| `represent` | `EMPTY` | 2 |
| `rlogic` | `(s \| rlogicitem)*` | 7 |
| `rlogicitem` | `EMPTY` | 5 |
| `s` | `(#PCDATA)` | 5 |
| `scheme` | `(step+)` | 1 |
| `sequence` | `(t*)` | 1 |
| `sgcomponent` | `((objecttag \| sgdatum)*)` | 6 |
| `sgdatum` | `(objecttag \| embeddedobject)*` | 8 |
| `spectrum` | `(#PCDATA \| objecttag \| annotation)*` | 23 |
| `splitter` | `EMPTY` | 2 |
| `step` | `EMPTY` | 10 |
| `stoichiometrygrid` | `((objecttag \| annotation \| sgcomponent)*)` | 14 |
| `t` | `(s \| objecttag \| annotation)+` | 29 |
| `table` | `(page \| objecttag \| annotation)*` | 13 |
| `templategrid` | `EMPTY` | 4 |
| `tlclane` | `((objecttag \| annotation \| tlcspot)*)` | 2 |
| `tlcplate` | `((objecttag \| annotation \| tlclane)*)` | 25 |
| `tlcspot` | `(objecttag \| annotation \| embeddedobject)*` | 11 |

## CDXML 唯一属性名全集

| 属性 | 出现元素数 | 实现 | schema | storage | behavior |
| --- | ---: | --- | --- | --- | --- |
| `alpha` | 25 | `native-semantic` | `verified` | `verified` | `verified` |
| `BoundingBox` | 21 | `native-semantic` | `verified` | `verified` | `verified` |
| `color` | 27 | `native-semantic` | `verified` | `verified` | `verified` |
| `GroupFrame` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `id` | 45 | `native-semantic` | `verified` | `verified` | `verified` |
| `IgnoreWarnings` | 8 | `native-semantic` | `verified` | `verified` | `verified` |
| `p` | 7 | `native-semantic` | `verified` | `verified` | `verified` |
| `SupersededBy` | 17 | `native-semantic` | `verified` | `verified` | `verified` |
| `TextFrame` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Valence` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Visible` | 21 | `native-semantic` | `verified` | `verified` | `verified` |
| `Warning` | 8 | `native-semantic` | `verified` | `verified` | `verified` |
| `Z` | 25 | `native-semantic` | `verified` | `verified` | `verified` |
| `Content` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Keyword` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `AngularSize` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowEquilibriumRatio` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowheadCenterSize` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowheadHead` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowheadTail` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowheadType` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowheadWidth` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowShaftSpacing` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowSource` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowTarget` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BoldWidth` | 12 | `native-semantic` | `verified` | `verified` | `verified` |
| `CaptionFace` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CaptionFont` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CaptionSize` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Center3D` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Dipole` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `FadePercent` | 5 | `typed-interchange` | `verified` | `verified` | `verified` |
| `FillType` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `HashSpacing` | 9 | `native-semantic` | `verified` | `verified` | `verified` |
| `Head3D` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `HeadSize` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `LineType` | 6 | `typed-interchange` | `verified` | `verified` | `verified` |
| `LineWidth` | 16 | `native-semantic` | `verified` | `verified` | `verified` |
| `MajorAxisEnd3D` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MinorAxisEnd3D` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `NoGo` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Tail3D` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `B` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `BeginAttach` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `BeginExternalNum` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BondCircularOrdering` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BondLength` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BondSpacing` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `BondSpacingAbs` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `BS` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Connectivity` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CrossingBonds` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `CrossingBondss` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Display` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Display2` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `DoublePosition` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `E` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `EndAttach` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `EndExternalNum` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `LabelFace` | 10 | `native-semantic` | `verified` | `verified` | `verified` |
| `LabelFont` | 10 | `native-semantic` | `verified` | `verified` | `verified` |
| `LabelSize` | 10 | `native-semantic` | `verified` | `verified` | `verified` |
| `MarginWidth` | 8 | `native-semantic` | `verified` | `verified` | `verified` |
| `Order` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `RxnParticipation` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowBondQuery` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowBondRxn` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowBondStereo` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `Topology` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `BioShapeType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CylinderDistance` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CylinderHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CylinderWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `DNAWaveHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `DNAWaveLength` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `DNAWaveOffset` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `DNAWaveWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `EnzymeHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `EnzymeReceptorSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `EnzymeWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GolgiHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GolgiLength` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GolgiWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GproteinLowerHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GproteinUpperHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `HelixProteinExtra` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ImmunoglobinHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ImmunoglobinWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MembraneElementSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MembraneEndAngle` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MembraneMajorAxisSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MembraneMinorAxisSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MembraneStartAngle` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `NeckHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `NeckWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `PipeWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `xyz` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Side` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GraphicID` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `BracketedObjectIDs` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `BracketUsage` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `ComponentOrder` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `PolymerFlipType` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `PolymerRepeatPattern` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `RepeatCount` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `SRULabel` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `AminoAcidTermini` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `bgalpha` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `bgcolor` | 3 | `native-semantic` | `verified` | `verified` | `verified` |
| `CaptionColor` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CaptionJustification` | 4 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CaptionLineHeight` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `CartridgeData` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChainAngle` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropAnalysis` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropBoilingPt` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropCLogP` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropCMR` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropCritPres` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropCritTemp` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropCritVol` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropEForm` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropExactMass` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropFormula` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropFragmentLabel` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropGibbs` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropHenry` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropID` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropLogP` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropLogS` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropMeltingPt` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropMolWt` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropMOverZ` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropMR` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropName` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemPropPKa` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemProptPSA` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Comment` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CreationDate` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CreationProgram` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `CreationUserName` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `FixInPlaceExtent` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `FixInPlaceGap` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `FractionalWidths` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `HideImplicitHydrogens` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `InterpretChemically` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `LabelColor` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `LabelJustification` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `LabelLineHeight` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `MacPrintInfo` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Magnification` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ModificationDate` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ModificationProgram` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ModificationUserName` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Name` | 7 | `native-semantic` | `verified` | `verified` | `verified` |
| `PrintMargins` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ResidueBlockCount` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ResidueWrapCount` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RxnAutonumberConditions` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RxnAutonumberFormat` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RxnAutonumberStart` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RxnAutonumberStyle` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowAtomEnhancedStereo` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowAtomNumber` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowAtomQuery` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowAtomStereo` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowNonTerminalCarbonLabels` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowResidueID` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowSequenceBonds` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowSequenceTermini` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowSequenceUnlinkedBranches` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowTerminalCarbonLabels` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `WindowIsZoomed` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `WindowPosition` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `WindowSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `WinPrintInfo` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BasisObjects` | 4 | `native-semantic` | `verified` | `verified` | `verified` |
| `ChemicallySignificant` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ChemicalPropertyDisplayID` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ChemicalPropertyIsActive` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ChemicalPropertyType` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ExternalBonds` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `PositioningAngle` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `PositioningOffset` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `PositioningType` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `b` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `g` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `r` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ConstraintMax` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ConstraintMin` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ConstraintType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `DihedralIsChiral` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `IgnoreUnconnectedAtoms` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `PointIsDirected` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BondID` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `InnerAtomID` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `CrossReferenceContainer` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `CrossReferenceDocument` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `CrossReferenceIdentifier` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `CrossReferenceSequence` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Closed` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CurvePoints` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CurvePoints3D` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CurveSpacing` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CurveType` | 3 | `typed-interchange` | `verified` | `verified` | `verified` |
| `HeadCenterSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `HeadWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BMP` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CompressedEnhancedMetafile` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CompressedOLEObject` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CompressedWindowsMetafile` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Edition` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `EditionAlias` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `EnhancedMetafile` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GIF` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `JPEG` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MacPICT` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `OLEObject` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `PDF` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `PNG` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RotationAngle` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `TIFF` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `UncompressedEnhancedMetafileSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `UncompressedOLEObjectSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `UncompressedWindowsMetafileSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `WindowsMetafile` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `charset` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `name` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Absolute` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ConnectionOrder` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Formula` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Racemic` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Relative` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `SequenceType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Weight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GeometricFeature` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RelationValue` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BandValue` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Height` | 3 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowValue` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Width` | 4 | `native-semantic` | `verified` | `verified` | `verified` |
| `LabelText` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `AxisWidth` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BottomLeft` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BottomRight` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `EndRange` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `LabelsAngle` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowBorders` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowScale` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `StartRange` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `TopLeft` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `TopRight` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Transparent` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `UnitID` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ArrowType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `BracketType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `CornerRadius` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `FrameType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `GraphicType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `LipSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `OrbitalType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `OvalType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RectangleType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShadowSize` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `SymbolType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Integral` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `DisplayName` | 3 | `native-semantic` | `verified` | `verified` | `verified` |
| `MarkerAngle` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `MarkerOffset` | 2 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Persistent` | 3 | `native-semantic` | `verified` | `verified` | `verified` |
| `TagType` | 3 | `native-semantic` | `verified` | `verified` | `verified` |
| `Value` | 3 | `native-semantic` | `verified` | `verified` | `verified` |
| `AbnormalValence` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `AltGroupID` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `AS` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `AtomID` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `AtomNumber` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Attachments` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `BondOrdering` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Charge` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Element` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ElementList` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `EnhancedStereoGroupNum` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `EnhancedStereoType` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ExternalConnectionNum` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ExternalConnectionType` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `FreeSites` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `GenericList` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `GenericNickname` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Geometry` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `HDash` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `HDot` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ImplicitHydrogens` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Isotope` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `IsotopicAbundance` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `LabelDisplay` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `LinkCountHigh` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `LinkCountLow` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `NeedsClean` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `NodeType` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `NumHydrogens` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Radical` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `RingBondCount` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `RxnChange` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `RxnStereo` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ShowAtomID` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `SubstituentsExactly` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `SubstituentsUpTo` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Translation` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `UnsaturatedBonds` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Tracking` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `BoundsInParent` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `DrawingSpace` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Footer` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `FooterPosition` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Header` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `HeaderPosition` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `HeightPages` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `PageDefinition` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `PageOverlap` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `PrintTrimMarks` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `SplitterPositions` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `WidthPages` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `NumberBasePairs` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RingRadius` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RegionEnd` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RegionOffset` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RegionStart` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RegistryAuthority` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `RegistryNumber` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `attribute` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `object` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `LineHeight` | 2 | `native-semantic` | `verified` | `verified` | `verified` |
| `RLogicGroup` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RLogicIfThenGroup` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RLogicOccurrence` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `RLogicRestH` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `face` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `font` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `size` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `SequenceIdentifier` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ComponentIsHeader` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ComponentIsReactant` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ComponentReferenceID` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `IsEdited` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `IsHidden` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `IsReadOnly` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `SGDataType` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `SGDataValue` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `SGPropertyType` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Class` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `XAxisLabel` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `XLow` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `XSpacing` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `XType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `YAxisLabel` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `YLow` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `YScale` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `YType` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ReactionStepArrows` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepAtomMap` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepAtomMapAuto` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepAtomMapManual` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepObjectsAboveArrow` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepObjectsBelowArrow` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepPlusses` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepProducts` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `ReactionStepReactants` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `Justification` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `LabelAlignment` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `LineStarts` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `WordWrapWidth` | 1 | `native-semantic` | `verified` | `verified` | `verified` |
| `extent` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `NumColumns` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `NumRows` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `PaneHeight` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `OriginFraction` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowOrigin` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowSideTicks` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowSolventFront` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `SolventFrontFraction` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Rf` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `ShowRf` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |
| `Tail` | 1 | `typed-interchange` | `verified` | `verified` | `verified` |

## 已知官方表勘误

- `kCDXProp_3DMajorAxisEnd`：发布表 {"cdxmlName":"Center3D"}；采用 {"cdxmlName":"MajorAxisEnd3D"}。依据：SDK constant name, description, and current Revvity DTD。
- `kCDXProp_3DMinorAxisEnd`：发布表 {"cdxmlName":"Center3D"}；采用 {"cdxmlName":"MinorAxisEnd3D"}。依据：SDK constant name, description, and current Revvity DTD。
- `kCDXProp_Closed`：发布表 {"tag":"0x0A38","cdxType":"CDXBoolean"}；采用 {"tag":"0x0A39","cdxType":"CDXBooleanImplied"}。依据：0x0A38 is CurveSpacing; CDX sequence and empty-property encoding。

## ChemDraw 二进制对象 tag 勘误

- ChemDraw 12.0.2 与 21.0.0.28 的实际 CDX 输出一致使用：`Border=0x801a`、`Geometry=0x801b`、`Constraint=0x801c`、`TLCPlate=0x801d`、`TLCLane=0x801e`、`TLCSpot=0x801f`、`ChemicalProperty=0x8020`、`Arrow=0x8021`。发布版静态对象表从 Border 起发生了偏移；账本保留其原值为 `publishedTag`，运行时值记为 `tag` 并标为 `verified-with-erratum`。
- 导入器仍识别 ChemSema beta 阶段写出的偏移 tag，但新文件统一写 ChemDraw 实际 tag；这属于格式级兼容，不按单个样例分支。
